use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::OnceCell;
use tokio::time::Instant;
use uuid::Uuid;

use super::api::SlackApi;
use super::mrkdwn::{bound_chunks, split_message, to_mrkdwn, MAX_MESSAGES};
use super::{MESSAGE_LIMIT, PLATFORM};
use crate::discord::adapter::{
    append_runtime_footer, extract_media_directives, media_path_is_denied, validate_media_path_in,
};
use crate::discord::table_render::{
    fallback_text, transform_markdown_tables_to_images, RenderedTable,
};
use crate::models::{filter_reasoning, is_explicit_silence};
use crate::{OmonError, OutboundAction, OutboundDispatcher, Result, SessionKey, StreamChunk};

const STREAM_PLACEHOLDER: &str = "…";
const STREAM_UPDATE_INTERVAL: Duration = Duration::from_millis(1200);
/// How long a finished stream id is remembered so a retried final chunk does not repost.
const COMPLETED_RETENTION: Duration = Duration::from_secs(15 * 60);
const APPROVAL_COMMAND_LIMIT: usize = 2800;
const APPROVAL_REASON_LIMIT: usize = 300;
const APPROVAL_EXPIRED_TEXT: &str = "⌛ Approval expired";
const STATUS_REACTIONS: [&str; 3] = ["eyes", "white_check_mark", "x"];
const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;
const DEAD_TARGET_PROBE_INTERVAL: Duration = Duration::from_secs(60);
const DEAD_TARGET_CODES: [&str; 4] = [
    "channel_not_found",
    "is_archived",
    "not_in_channel",
    "account_inactive",
];
const PRIVACY_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

type StreamKey = (String, Uuid);

#[derive(Default)]
struct StreamState {
    ts: Option<String>,
    last_sequence: Option<u64>,
    last_update: Option<Instant>,
    final_sent: usize,
    prepared: Option<(String, Vec<RenderedTable>)>,
}

#[derive(Clone, Debug)]
struct DeadTarget {
    reason: String,
    marked_at: DateTime<Utc>,
    probed_at: Option<DateTime<Utc>>,
}

pub struct SlackEgress {
    api: SlackApi,
    streams: Mutex<HashMap<StreamKey, Arc<tokio::sync::Mutex<StreamState>>>>,
    completed: Mutex<HashMap<StreamKey, Instant>>,
    approvals: Mutex<HashMap<Uuid, (String, String)>>,
    update_interval: Duration,
    workspace_root: Option<PathBuf>,
    runtime_footer: bool,
    default_model: Option<String>,
    approval_mentions: Vec<String>,
    approval_mentions_enabled: bool,
    pool: Option<SqlitePool>,
    dead_targets: Mutex<HashMap<String, DeadTarget>>,
    dead_targets_loaded: OnceCell<()>,
    probe_interval: Duration,
    upload_limit: u64,
}

impl SlackEgress {
    pub fn new(api: SlackApi) -> Self {
        Self {
            api,
            streams: Mutex::new(HashMap::new()),
            completed: Mutex::new(HashMap::new()),
            approvals: Mutex::new(HashMap::new()),
            update_interval: STREAM_UPDATE_INTERVAL,
            workspace_root: None,
            runtime_footer: false,
            default_model: None,
            approval_mentions: Vec::new(),
            approval_mentions_enabled: false,
            pool: None,
            dead_targets: Mutex::new(HashMap::new()),
            dead_targets_loaded: OnceCell::new(),
            probe_interval: DEAD_TARGET_PROBE_INTERVAL,
            upload_limit: MAX_UPLOAD_BYTES,
        }
    }

    /// Extra root `MEDIA:` directives may upload from, besides the temp directories.
    pub fn with_workspace_root(mut self, root: PathBuf) -> Self {
        self.workspace_root = Some(root);
        self
    }

    pub fn with_runtime_footer(mut self, enabled: bool) -> Self {
        self.runtime_footer = enabled;
        self
    }

    pub fn with_default_model(mut self, model: Option<String>) -> Self {
        self.default_model = model;
        self
    }

    pub fn with_approval_mentions(mut self, users: Vec<String>, enabled: bool) -> Self {
        self.approval_mentions = users;
        self.approval_mentions_enabled = enabled;
        self
    }

    /// Persists dead targets in `slack_dead_targets` so they survive restarts.
    pub fn with_pool(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool);
        self
    }

    #[cfg(test)]
    fn with_update_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    #[cfg(test)]
    fn with_probe_interval(mut self, interval: Duration) -> Self {
        self.probe_interval = interval;
        self
    }

    #[cfg(test)]
    fn with_upload_limit(mut self, limit: u64) -> Self {
        self.upload_limit = limit;
        self
    }

    /// Replaces the approval buttons with the decision once a button was handled.
    pub async fn mark_approval_decided(&self, request_id: Uuid, label: &str) -> Result<()> {
        let Some((channel, ts)) = self.approvals.lock().remove(&request_id) else {
            return Ok(());
        };
        self.api
            .update_message(&channel, &ts, label, Some(text_blocks(label)))
            .await
    }

    async fn send_message(
        &self,
        session: &SessionKey,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<()> {
        let filtered = filter_reasoning(content);
        if filtered.is_empty() || is_explicit_silence(&filtered) {
            return Ok(());
        }
        let channel = session.channel_id.as_str();
        let thread = reply_thread(session, reply_to);
        self.skip_dead(channel).await?;
        let decorated = if self.runtime_footer {
            append_runtime_footer(
                &filtered,
                self.default_model.as_deref(),
                None,
                self.workspace_root.as_deref(),
            )
        } else {
            filtered
        };
        let (without_media, media_error) = self.deliver_media(channel, thread, &decorated).await;
        let (text, tables) = transform_markdown_tables_to_images(&without_media);
        for chunk in render(&text) {
            self.post(channel, &chunk, thread, None).await?;
        }
        let tables_result = self.deliver_tables(channel, thread, &tables).await;
        match media_error {
            Some(error) => Err(error),
            None => tables_result,
        }
    }

    async fn edit_message(&self, session: &SessionKey, ts: &str, content: &str) -> Result<()> {
        let filtered = filter_reasoning(content);
        if filtered.is_empty() || is_explicit_silence(&filtered) {
            return Ok(());
        }
        let Some(text) = render(&filtered).into_iter().next() else {
            return Ok(());
        };
        self.api
            .update_message(&session.channel_id, ts, &text, None)
            .await
    }

    async fn stream(&self, session: &SessionKey, chunk: StreamChunk) -> Result<()> {
        let key = (session.storage_key(), chunk.stream_id);
        if self.completed.lock().contains_key(&key) {
            return Ok(());
        }
        let channel = session.channel_id.as_str();
        if !self.streams.lock().contains_key(&key) {
            self.skip_dead(channel).await?;
        }
        let cell = self.streams.lock().entry(key.clone()).or_default().clone();
        let mut state = cell.lock().await;
        if self.completed.lock().contains_key(&key) {
            return Ok(());
        }
        let thread = reply_thread(session, chunk.reply_to.as_deref());
        let filtered = filter_reasoning(&chunk.content);

        if !chunk.is_final {
            if state
                .last_sequence
                .is_some_and(|sequence| chunk.sequence <= sequence)
            {
                return Ok(());
            }
            state.last_sequence = Some(chunk.sequence);
            let Some(ts) = state.ts.clone() else {
                let ts = self.post(channel, STREAM_PLACEHOLDER, thread, None).await?;
                state.ts = Some(ts);
                state.last_update = Some(Instant::now());
                return Ok(());
            };
            let due = state
                .last_update
                .is_none_or(|at| at.elapsed() >= self.update_interval);
            if !due || filtered.is_empty() || is_explicit_silence(&filtered) {
                return Ok(());
            }
            let Some(text) = render(&filtered).into_iter().next() else {
                return Ok(());
            };
            state.last_update = Some(Instant::now());
            if let Err(error) = self.api.update_message(channel, &ts, &text, None).await {
                tracing::debug!(%error, channel, "failed to update slack stream preview");
            }
            return Ok(());
        }

        let mut media_error = None;
        if state.prepared.is_none() {
            let prepared = if is_explicit_silence(&filtered) {
                (String::new(), Vec::new())
            } else {
                let (without_media, error) = self.deliver_media(channel, thread, &filtered).await;
                media_error = error;
                transform_markdown_tables_to_images(&without_media)
            };
            state.prepared = Some(prepared);
        }
        let (text, tables) = state.prepared.clone().unwrap_or_default();
        let chunks = render(&text);
        if chunks.is_empty() {
            if let Some(ts) = &state.ts {
                self.api.delete_message(channel, ts).await?;
            }
        }
        for (index, text) in chunks.iter().enumerate().skip(state.final_sent) {
            match (index, state.ts.clone()) {
                (0, Some(ts)) => self.api.update_message(channel, &ts, text, None).await?,
                (0, None) => {
                    let ts = self.post(channel, text, thread, None).await?;
                    state.ts = Some(ts);
                }
                _ => {
                    self.post(channel, text, thread, None).await?;
                }
            }
            state.final_sent = index + 1;
        }
        drop(state);
        self.finish_stream(key);
        let tables_result = self.deliver_tables(channel, thread, &tables).await;
        match media_error {
            Some(error) => Err(error),
            None => tables_result,
        }
    }

    fn finish_stream(&self, key: StreamKey) {
        self.streams.lock().remove(&key);
        let mut completed = self.completed.lock();
        completed.retain(|_, at| at.elapsed() < COMPLETED_RETENTION);
        completed.insert(key, Instant::now());
    }

    /// Uploads every `MEDIA:` directive and returns the text without them. Media problems never
    /// stop the text: a missing path is only noted, while a rejected path or failed upload is
    /// noted and returned as an error to report after the text went out.
    async fn deliver_media(
        &self,
        channel: &str,
        thread: Option<&str>,
        text: &str,
    ) -> (String, Option<OmonError>) {
        let (mut without_media, media_paths) = extract_media_directives(text);
        let roots: Vec<PathBuf> = self.workspace_root.iter().cloned().collect();
        let mut failed = Vec::new();
        let mut deferred = None;
        for media_path in &media_paths {
            let label = Path::new(media_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(media_path.as_str())
                .to_owned();
            let outcome = match validate_media_path_in(media_path, &roots) {
                Ok(path) => self.upload_path(channel, thread, &path).await,
                Err(error) if !Path::new(media_path).exists() => {
                    tracing::warn!(%error, media_path, "media directive names a missing file");
                    failed.push(label);
                    continue;
                }
                Err(error) => Err(error),
            };
            if let Err(error) = outcome {
                tracing::warn!(%error, media_path, "slack media upload failed; delivering text");
                failed.push(label);
                deferred.get_or_insert(error);
            }
        }
        append_slack_media_failure_note(&mut without_media, &failed);
        (without_media, deferred)
    }

    /// Uploads rendered table images in the reply's thread. When one fails, the remaining tables
    /// are posted as code blocks so their content is never lost.
    async fn deliver_tables(
        &self,
        channel: &str,
        thread: Option<&str>,
        tables: &[RenderedTable],
    ) -> Result<()> {
        for (index, table) in tables.iter().enumerate() {
            let result = self
                .api
                .upload_bytes(
                    channel,
                    thread,
                    &table.filename,
                    table.png_bytes.clone(),
                    None,
                )
                .await;
            self.record(channel, &result).await;
            if let Err(error) = result {
                for chunk in render(&fallback_text(&tables[index..])) {
                    if let Err(fallback_error) = self.post(channel, &chunk, thread, None).await {
                        tracing::warn!(%fallback_error, channel, "failed to post slack table fallback");
                    }
                }
                return Err(error);
            }
        }
        Ok(())
    }

    async fn upload_path(&self, channel: &str, thread: Option<&str>, path: &Path) -> Result<()> {
        let canonical = tokio::fs::canonicalize(path).await.map_err(|error| {
            OmonError::Config(format!("cannot resolve upload {}: {error}", path.display()))
        })?;
        if media_path_is_denied(&canonical) {
            return Err(OmonError::Config(format!(
                "refusing to upload secret-bearing file: {}",
                path.display()
            )));
        }
        let metadata = tokio::fs::metadata(&canonical).await.map_err(|error| {
            OmonError::Config(format!("cannot stat upload {}: {error}", path.display()))
        })?;
        if !metadata.is_file() {
            return Err(OmonError::Config(format!(
                "upload is not a regular file: {}",
                path.display()
            )));
        }
        if metadata.len() > self.upload_limit {
            return Err(OmonError::Config(format!(
                "upload {} is {} bytes, over the {} byte limit",
                path.display(),
                metadata.len(),
                self.upload_limit
            )));
        }
        let result = self
            .api
            .upload_file(channel, thread, &canonical, None)
            .await
            .map(drop);
        self.record(channel, &result).await;
        result
    }

    async fn post(
        &self,
        channel: &str,
        text: &str,
        thread: Option<&str>,
        blocks: Option<Value>,
    ) -> Result<String> {
        let result = self.api.post_message(channel, text, thread, blocks).await;
        self.record(channel, &result).await;
        result
    }

    async fn load_dead_targets(&self) {
        let Some(pool) = &self.pool else { return };
        self.dead_targets_loaded
            .get_or_init(|| async {
                let rows =
                    sqlx::query_as::<_, (String, String, DateTime<Utc>, Option<DateTime<Utc>>)>(
                        "SELECT channel_id, reason, marked_at, probed_at FROM slack_dead_targets",
                    )
                    .fetch_all(pool)
                    .await;
                match rows {
                    Ok(rows) => {
                        let mut targets = self.dead_targets.lock();
                        for (channel, reason, marked_at, probed_at) in rows {
                            targets.entry(channel).or_insert(DeadTarget {
                                reason,
                                marked_at,
                                probed_at,
                            });
                        }
                    }
                    Err(error) => tracing::warn!(%error, "failed to load slack dead targets"),
                }
            })
            .await;
    }

    /// Refuses delivery to a channel known to be dead, except for one probe per interval.
    async fn skip_dead(&self, channel: &str) -> Result<()> {
        self.load_dead_targets().await;
        let now = Utc::now();
        {
            let mut targets = self.dead_targets.lock();
            let Some(target) = targets.get_mut(channel) else {
                return Ok(());
            };
            let since = target.probed_at.unwrap_or(target.marked_at);
            if (now - since).to_std().unwrap_or_default() < self.probe_interval {
                return Err(OmonError::Multiplexer(format!(
                    "slack dead target short-circuited: channel={channel} ({})",
                    target.reason
                )));
            }
            target.probed_at = Some(now);
        }
        if let Some(pool) = &self.pool {
            let result =
                sqlx::query("UPDATE slack_dead_targets SET probed_at = ? WHERE channel_id = ?")
                    .bind(now)
                    .bind(channel)
                    .execute(pool)
                    .await;
            if let Err(error) = result {
                tracing::warn!(%error, channel, "failed to record slack dead target probe");
            }
        }
        Ok(())
    }

    /// Marks the channel dead on a channel-level Slack error and heals it on success.
    async fn record<T>(&self, channel: &str, result: &Result<T>) {
        let outcome = match result {
            Ok(_) => Ok(()),
            Err(error) => Err(dead_target_reason(error)),
        };
        self.record_outcome(channel, outcome).await;
    }

    async fn record_outcome(
        &self,
        channel: &str,
        outcome: std::result::Result<(), Option<&'static str>>,
    ) {
        match outcome {
            Ok(()) => {
                if self.dead_targets.lock().remove(channel).is_none() {
                    return;
                }
                tracing::info!(channel, "slack dead target delivered again; cleared");
                if let Some(pool) = &self.pool {
                    let result = sqlx::query("DELETE FROM slack_dead_targets WHERE channel_id = ?")
                        .bind(channel)
                        .execute(pool)
                        .await;
                    if let Err(error) = result {
                        tracing::warn!(%error, channel, "failed to clear slack dead target");
                    }
                }
            }
            Err(reason) => {
                let Some(reason) = reason else {
                    return;
                };
                let now = Utc::now();
                self.dead_targets.lock().insert(
                    channel.to_owned(),
                    DeadTarget {
                        reason: reason.to_owned(),
                        marked_at: now,
                        probed_at: None,
                    },
                );
                tracing::warn!(channel, reason, "marked slack channel as dead target");
                if let Some(pool) = &self.pool {
                    let result = sqlx::query(
                        "INSERT INTO slack_dead_targets (channel_id, reason, marked_at, probed_at)
                         VALUES (?, ?, ?, NULL)
                         ON CONFLICT(channel_id) DO UPDATE SET
                           reason = excluded.reason, marked_at = excluded.marked_at, probed_at = NULL",
                    )
                    .bind(channel)
                    .bind(reason)
                    .bind(now)
                    .execute(pool)
                    .await;
                    if let Err(error) = result {
                        tracing::warn!(%error, channel, "failed to persist slack dead target");
                    }
                }
            }
        }
    }

    async fn react(
        &self,
        session: &SessionKey,
        ts: &str,
        emoji: &str,
        remove_others: bool,
    ) -> Result<()> {
        let Some(name) = reaction_name(emoji) else {
            tracing::debug!(emoji, "no slack reaction name for emoji");
            return Ok(());
        };
        let channel = session.channel_id.as_str();
        if remove_others {
            for other in STATUS_REACTIONS.iter().filter(|other| **other != name) {
                if let Err(error) = self.api.remove_reaction(channel, ts, other).await {
                    tracing::debug!(%error, channel, reaction = other, "failed to remove slack reaction");
                }
            }
        }
        if let Err(error) = self.api.add_reaction(channel, ts, &name).await {
            tracing::debug!(%error, channel, reaction = %name, "failed to add slack reaction");
        }
        Ok(())
    }

    async fn request_approval(
        &self,
        session: &SessionKey,
        request_id: Uuid,
        command: &str,
        reason: &str,
    ) -> Result<()> {
        let channel = session.channel_id.as_str();
        self.skip_dead(channel).await?;
        let mentions = approval_mentions(&self.approval_mentions, self.approval_mentions_enabled);
        let fallback = format!(
            "{}Approval needed: {}",
            mentions
                .as_deref()
                .map(|m| format!("{m} "))
                .unwrap_or_default(),
            truncate_chars(reason, APPROVAL_REASON_LIMIT)
        );
        let ts = self
            .post(
                channel,
                &fallback,
                session.thread_id.as_deref(),
                Some(approval_blocks(
                    request_id,
                    command,
                    reason,
                    mentions.as_deref(),
                )),
            )
            .await?;
        self.approvals
            .lock()
            .insert(request_id, (session.channel_id.clone(), ts));
        Ok(())
    }

    async fn expire_approval(&self, request_id: Uuid) -> Result<()> {
        let Some((channel, ts)) = self.approvals.lock().remove(&request_id) else {
            return Ok(());
        };
        self.api
            .update_message(
                &channel,
                &ts,
                APPROVAL_EXPIRED_TEXT,
                Some(text_blocks(APPROVAL_EXPIRED_TEXT)),
            )
            .await
    }

    async fn upload(&self, session: &SessionKey, path: &Path) -> Result<()> {
        let channel = session.channel_id.as_str();
        self.skip_dead(channel).await?;
        self.upload_path(channel, session.thread_id.as_deref(), path)
            .await
    }

    async fn channel_is_owner_only(&self, channel: &str, owner: &str, bot: &str) -> bool {
        let Ok(info) = self.api.conversations_info(channel).await else {
            return false;
        };
        if info["is_private"].as_bool() != Some(true) {
            return false;
        }
        let Ok(members) = self.api.conversations_members(channel).await else {
            return false;
        };
        members.iter().any(|member| member == owner)
            && members
                .iter()
                .all(|member| member == owner || member == bot)
    }
}

#[async_trait]
impl OutboundDispatcher for SlackEgress {
    /// True for a private channel whose only members are the owner and this bot.
    async fn owner_thread_is_private(&self, session: &SessionKey) -> bool {
        if session.platform != PLATFORM || session.guild_id.is_none() {
            return false;
        }
        let Some(bot) = session.bot_id.as_deref().filter(|bot| !bot.is_empty()) else {
            return false;
        };
        let owner = super::owner_id();
        tokio::time::timeout(
            PRIVACY_CHECK_TIMEOUT,
            self.channel_is_owner_only(&session.channel_id, &owner, bot),
        )
        .await
        .unwrap_or(false)
    }

    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action {
            OutboundAction::SendMessage {
                session,
                content,
                reply_to,
            } => {
                self.send_message(&session, &content, reply_to.as_deref())
                    .await
            }
            OutboundAction::EditMessage {
                session,
                platform_message_id,
                content,
            } => {
                self.edit_message(&session, &platform_message_id, &content)
                    .await
            }
            OutboundAction::DeleteMessage {
                session,
                platform_message_id,
            } => {
                self.api
                    .delete_message(&session.channel_id, &platform_message_id)
                    .await
            }
            OutboundAction::UploadFile { session, path } => self.upload(&session, &path).await,
            OutboundAction::Stream { session, chunk } => self.stream(&session, chunk).await,
            OutboundAction::Typing { .. } => Ok(()),
            OutboundAction::React {
                session,
                message_id,
                emoji,
                remove_others,
            } => {
                self.react(&session, &message_id, &emoji, remove_others)
                    .await
            }
            OutboundAction::ApprovalRequest {
                session,
                request_id,
                command,
                reason,
            } => {
                self.request_approval(&session, request_id, &command, &reason)
                    .await
            }
            OutboundAction::ExpireApproval { request_id } => self.expire_approval(request_id).await,
        }
    }
}

/// Session thread first; a top-level reply in a channel threads under the triggering message.
/// DMs stay flat.
fn reply_thread<'a>(session: &'a SessionKey, reply_to: Option<&'a str>) -> Option<&'a str> {
    session.thread_id.as_deref().or_else(|| {
        session
            .guild_id
            .is_some()
            .then_some(reply_to)
            .flatten()
            .filter(|ts| {
                ts.split_once('.').is_some_and(|(seconds, fraction)| {
                    !seconds.is_empty()
                        && !fraction.is_empty()
                        && seconds.bytes().all(|byte| byte.is_ascii_digit())
                        && fraction.bytes().all(|byte| byte.is_ascii_digit())
                })
            })
    })
}

fn dead_target_reason(error: &OmonError) -> Option<&'static str> {
    let OmonError::Multiplexer(message) = error else {
        return None;
    };
    let code = message.strip_prefix("slack ")?.rsplit(": ").next()?.trim();
    DEAD_TARGET_CODES
        .iter()
        .copied()
        .find(|known| *known == code)
}

fn append_slack_media_failure_note(text: &mut String, failed: &[String]) {
    if failed.is_empty() {
        return;
    }
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    text.push_str("_⚠️ 파일 첨부 실패: ");
    text.push_str(&failed.join(", "));
    text.push('_');
}

fn render(text: &str) -> Vec<String> {
    let chunks = split_message(&to_mrkdwn(text), MESSAGE_LIMIT)
        .into_iter()
        .filter(|chunk| !chunk.trim().is_empty())
        .collect();
    bound_chunks(chunks, MAX_MESSAGES, MESSAGE_LIMIT)
}

fn approval_mentions(users: &[String], enabled: bool) -> Option<String> {
    if !enabled {
        return None;
    }
    let mut users: Vec<&str> = users
        .iter()
        .map(|user| user.trim())
        .filter(|user| !user.is_empty() && user.chars().all(|c| c.is_ascii_alphanumeric()))
        .collect();
    users.sort_unstable();
    users.dedup();
    (!users.is_empty()).then(|| {
        users
            .iter()
            .map(|user| format!("<@{user}>"))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn reaction_name(emoji: &str) -> Option<String> {
    let emoji = emoji.trim().trim_end_matches('\u{fe0f}');
    let name = match emoji {
        "👀" => "eyes",
        "✅" => "white_check_mark",
        "❌" => "x",
        "⏳" => "hourglass_flowing_sand",
        other => {
            let name = other.strip_prefix(':')?.strip_suffix(':')?;
            let valid = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '\''));
            return valid.then(|| name.to_owned());
        }
    };
    Some(name.to_owned())
}

fn approval_blocks(request_id: Uuid, command: &str, reason: &str, mentions: Option<&str>) -> Value {
    let reason = escape(&truncate_chars(reason, APPROVAL_REASON_LIMIT));
    let command =
        escape(&truncate_chars(command, APPROVAL_COMMAND_LIMIT)).replace("```", "`\u{200b}``");
    let button = |label: &str, choice: &str, style: Option<&str>| {
        let id = format!("omon:approval:{request_id}:{choice}");
        let mut button = json!({
            "type": "button",
            "text": { "type": "plain_text", "text": label },
            "action_id": id,
            "value": id,
        });
        if let Some(style) = style {
            button["style"] = json!(style);
        }
        button
    };
    json!([
        {
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("{}*Approval needed*\n{reason}", mentions.map(|m| format!("{m}\n")).unwrap_or_default()) },
        },
        {
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("```\n{command}\n```") },
        },
        {
            "type": "actions",
            "block_id": format!("omon:approval:{request_id}"),
            "elements": [
                button("Allow Once", "once", Some("primary")),
                button("Allow Session", "session", Some("primary")),
                button("Always Allow", "always", None),
                button("Deny", "deny", Some("danger")),
            ],
        },
    ])
}

fn text_blocks(text: &str) -> Value {
    json!([{ "type": "section", "text": { "type": "mrkdwn", "text": escape(text) } }])
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(limit.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;

    struct Fake {
        base: String,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
        overrides: Arc<Mutex<HashMap<String, Value>>>,
    }

    impl Fake {
        fn requests(&self) -> Vec<(String, Value)> {
            self.requests.lock().clone()
        }

        fn paths(&self) -> Vec<String> {
            self.requests().into_iter().map(|(path, _)| path).collect()
        }

        fn respond(&self, path: &str, reply: Value) {
            self.overrides.lock().insert(path.to_owned(), reply);
        }

        fn clear(&self) {
            self.requests.lock().clear();
            self.overrides.lock().clear();
        }

        fn egress(&self) -> SlackEgress {
            SlackEgress::new(SlackApi::with_base_url(
                "xoxb-test".into(),
                self.base.clone(),
            ))
        }
    }

    async fn fake() -> Fake {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<(String, Value)>::new()));
        let overrides = Arc::new(Mutex::new(HashMap::<String, Value>::new()));
        let state = (requests.clone(), overrides.clone(), base.clone());
        let app = axum::Router::new().fallback(move |req: axum::extract::Request| {
            let (recorded, overrides, base) = state.clone();
            async move {
                let path = req.uri().path().trim_start_matches('/').to_owned();
                let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body = crate::slack::api::decode_test_body(&bytes);
                let posts = {
                    let mut requests = recorded.lock();
                    requests.push((path.clone(), body));
                    requests
                        .iter()
                        .filter(|(p, _)| p == "chat.postMessage")
                        .count()
                };
                if let Some(reply) = overrides.lock().get(&path).cloned() {
                    return axum::Json(reply);
                }
                let reply = match path.as_str() {
                    "chat.postMessage" => json!({ "ok": true, "ts": format!("100.{posts:04}") }),
                    "files.getUploadURLExternal" => json!({
                        "ok": true,
                        "upload_url": format!("{base}/upload/F1"),
                        "file_id": "F1",
                    }),
                    _ => json!({ "ok": true }),
                };
                axum::Json(reply)
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake {
            base,
            requests,
            overrides,
        }
    }

    fn session() -> SessionKey {
        SessionKey::new("slack", Some("T1"), "C1", Some("1699.0001"), "")
    }

    fn stream(stream_id: Uuid, sequence: u64, content: &str, is_final: bool) -> OutboundAction {
        OutboundAction::Stream {
            session: session(),
            chunk: StreamChunk {
                stream_id,
                sequence,
                content: content.into(),
                is_final,
                reply_to: Some("1699.0001".into()),
            },
        }
    }

    fn long_reply() -> String {
        (0..700)
            .map(|line| format!("line {line:04} of the reply"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[tokio::test]
    async fn send_message_posts_converted_text_in_thread_and_skips_silence() {
        let server = fake().await;
        let egress = server.egress();
        let content = "<think>plan</think>**Done** & see [docs](https://example.com)";

        egress
            .dispatch(OutboundAction::SendMessage {
                session: session(),
                content: content.into(),
                reply_to: Some("1699.0002".into()),
            })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::SendMessage {
                session: session(),
                content: "NO_REPLY".into(),
                reply_to: None,
            })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let (path, body) = &requests[0];
        assert_eq!(path, "chat.postMessage");
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["thread_ts"], "1699.0001");
        let expected = to_mrkdwn(&filter_reasoning(content));
        assert_eq!(body["text"], expected.as_str());
        assert!(!expected.contains("plan"));
    }

    #[tokio::test]
    async fn stream_posts_placeholder_updates_and_finishes_with_overflow() {
        let server = fake().await;
        let egress = server.egress().with_update_interval(Duration::ZERO);
        let id = Uuid::new_v4();
        let reply = long_reply();
        let expected = render(&reply);
        assert!(expected.len() >= 2);

        egress.dispatch(stream(id, 0, "Hel", false)).await.unwrap();
        egress
            .dispatch(stream(id, 1, "Hello **world**", false))
            .await
            .unwrap();
        egress
            .dispatch(stream(id, 1, "stale", false))
            .await
            .unwrap();
        egress.dispatch(stream(id, 2, &reply, true)).await.unwrap();
        egress.dispatch(stream(id, 2, &reply, true)).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2 + expected.len());
        assert_eq!(requests[0].0, "chat.postMessage");
        assert_eq!(requests[0].1["text"], STREAM_PLACEHOLDER);
        assert_eq!(requests[0].1["thread_ts"], "1699.0001");
        assert_eq!(requests[1].0, "chat.update");
        assert_eq!(requests[1].1["ts"], "100.0001");
        assert_eq!(requests[1].1["text"], to_mrkdwn("Hello **world**").as_str());
        assert_eq!(requests[2].0, "chat.update");
        assert_eq!(requests[2].1["ts"], "100.0001");
        assert_eq!(requests[2].1["text"], expected[0].as_str());
        for (request, chunk) in requests[3..].iter().zip(&expected[1..]) {
            assert_eq!(request.0, "chat.postMessage");
            assert_eq!(request.1["text"], chunk.as_str());
            assert_eq!(request.1["thread_ts"], "1699.0001");
        }
        assert!(egress.streams.lock().is_empty());
    }

    #[tokio::test]
    async fn stream_preview_updates_are_throttled() {
        let server = fake().await;
        let egress = server
            .egress()
            .with_update_interval(Duration::from_secs(3600));
        let id = Uuid::new_v4();

        egress.dispatch(stream(id, 0, "a", false)).await.unwrap();
        egress.dispatch(stream(id, 1, "ab", false)).await.unwrap();
        egress.dispatch(stream(id, 2, "abc", false)).await.unwrap();
        egress.dispatch(stream(id, 3, "abcd", true)).await.unwrap();

        assert_eq!(server.paths(), ["chat.postMessage", "chat.update"]);
        assert_eq!(server.requests()[1].1["text"], to_mrkdwn("abcd").as_str());
    }

    #[tokio::test]
    async fn silent_final_deletes_placeholder() {
        let server = fake().await;
        let egress = server.egress();
        let id = Uuid::new_v4();

        egress.dispatch(stream(id, 0, "NO_", false)).await.unwrap();
        egress
            .dispatch(stream(id, 1, "NO_REPLY", true))
            .await
            .unwrap();
        egress
            .dispatch(stream(Uuid::new_v4(), 0, "[SILENT]", true))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(server.paths(), ["chat.postMessage", "chat.delete"]);
        assert_eq!(requests[1].1["channel"], "C1");
        assert_eq!(requests[1].1["ts"], "100.0001");
    }

    #[tokio::test]
    async fn approval_posts_buttons_then_expires_once() {
        let server = fake().await;
        let egress = server.egress();
        let request_id = Uuid::new_v4();

        egress
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id,
                command: "rm -rf <dir> && echo ```".into(),
                reason: "deletes files".into(),
            })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let (path, body) = &requests[0];
        assert_eq!(path, "chat.postMessage");
        assert_eq!(body["thread_ts"], "1699.0001");
        let blocks = body["blocks"].as_array().unwrap();
        assert!(blocks[0]["text"]["text"]
            .as_str()
            .unwrap()
            .starts_with("*Approval needed*\ndeletes files"));
        let code = blocks[1]["text"]["text"].as_str().unwrap();
        assert!(code.starts_with("```\nrm -rf &lt;dir&gt; &amp;&amp; echo "));
        assert_eq!(code.matches("```").count(), 2);
        let buttons = blocks[2]["elements"].as_array().unwrap();
        let ids: Vec<&str> = buttons
            .iter()
            .map(|b| b["value"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            ["once", "session", "always", "deny"]
                .map(|choice| format!("omon:approval:{request_id}:{choice}"))
        );
        for button in buttons {
            assert_eq!(button["action_id"], button["value"]);
        }
        let styles: Vec<Option<&str>> = buttons.iter().map(|b| b["style"].as_str()).collect();
        assert_eq!(
            styles,
            [Some("primary"), Some("primary"), None, Some("danger")]
        );

        let (path, body) = &requests[1];
        assert_eq!(path, "chat.update");
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["ts"], "100.0001");
        assert_eq!(body["text"], APPROVAL_EXPIRED_TEXT);
        assert!(body["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|block| block["type"] != "actions"));
    }

    #[test]
    fn approval_command_is_truncated() {
        let blocks = approval_blocks(Uuid::new_v4(), &"a".repeat(5000), "why", None);
        let code = blocks[1]["text"]["text"].as_str().unwrap();
        assert!(code.chars().count() <= APPROVAL_COMMAND_LIMIT + 8);
        assert!(code.contains('…'));
    }

    #[tokio::test]
    async fn mark_approval_decided_replaces_buttons_with_label() {
        let server = fake().await;
        let egress = server.egress();
        let request_id = Uuid::new_v4();
        egress
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id,
                command: "ls".into(),
                reason: "list".into(),
            })
            .await
            .unwrap();

        egress
            .mark_approval_decided(request_id, "✅ Allowed once by <@U1>")
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].0, "chat.update");
        assert_eq!(requests[1].1["text"], "✅ Allowed once by <@U1>");
        assert_eq!(requests[1].1["blocks"][0]["type"], "section");
    }

    #[tokio::test]
    async fn react_maps_emoji_and_removes_other_status_reactions() {
        let server = fake().await;
        let egress = server.egress();
        let react = |emoji: &str, remove_others| OutboundAction::React {
            session: session(),
            message_id: "1699.0005".into(),
            emoji: emoji.into(),
            remove_others,
        };

        egress.dispatch(react("✅", true)).await.unwrap();
        egress.dispatch(react(":rocket:", false)).await.unwrap();
        egress.dispatch(react("🦀", true)).await.unwrap();

        let requests = server.requests();
        let calls: Vec<(&str, &str)> = requests
            .iter()
            .map(|(path, body)| (path.as_str(), body["name"].as_str().unwrap()))
            .collect();
        assert_eq!(
            calls,
            [
                ("reactions.remove", "eyes"),
                ("reactions.remove", "x"),
                ("reactions.add", "white_check_mark"),
                ("reactions.add", "rocket"),
            ]
        );
        assert!(requests
            .iter()
            .all(|(_, body)| body["channel"] == "C1" && body["timestamp"] == "1699.0005"));
    }

    #[test]
    fn reaction_names() {
        assert_eq!(reaction_name("👀").as_deref(), Some("eyes"));
        assert_eq!(reaction_name("❌").as_deref(), Some("x"));
        assert_eq!(
            reaction_name("⏳").as_deref(),
            Some("hourglass_flowing_sand")
        );
        assert_eq!(
            reaction_name("✅\u{fe0f}").as_deref(),
            Some("white_check_mark")
        );
        assert_eq!(reaction_name(":thumbsup:").as_deref(), Some("thumbsup"));
        assert_eq!(reaction_name("::"), None);
        assert_eq!(reaction_name("🦀"), None);
    }

    fn channel_session() -> SessionKey {
        SessionKey::new("slack", Some("T1"), "C1", None::<String>, "").with_bot_id("UBOT")
    }

    fn send(session: SessionKey, content: &str, reply_to: Option<&str>) -> OutboundAction {
        OutboundAction::SendMessage {
            session,
            content: content.into(),
            reply_to: reply_to.map(str::to_owned),
        }
    }

    fn posted_texts(server: &Fake) -> Vec<String> {
        server
            .requests()
            .into_iter()
            .filter(|(path, _)| path == "chat.postMessage")
            .map(|(_, body)| body["text"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    async fn pool() -> SqlitePool {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations")
            .run(database.pool())
            .await
            .unwrap();
        database.pool().clone()
    }

    #[tokio::test]
    async fn rendered_output_neutralizes_broadcast_mentions() {
        let server = fake().await;
        server
            .egress()
            .dispatch(send(session(), "<!channel> ping <@U1> <!subteam^S1>", None))
            .await
            .unwrap();
        assert_eq!(
            posted_texts(&server),
            ["&lt;!channel&gt; ping <@U1> &lt;!subteam^S1&gt;"]
        );
    }

    #[tokio::test]
    async fn dashboard_message_id_does_not_become_slack_thread_timestamp() {
        let server = fake().await;
        server
            .egress()
            .dispatch(send(
                channel_session(),
                "hi",
                Some("b872b0c2-6be7-4ea8-9e71-cdf38d8846bc"),
            ))
            .await
            .unwrap();
        assert!(server.requests()[0].1.get("thread_ts").is_none());
    }

    #[tokio::test]
    async fn top_level_channel_reply_threads_under_trigger_but_dm_stays_flat() {
        let server = fake().await;
        let egress = server.egress();
        egress
            .dispatch(send(channel_session(), "hi", Some("1700.0009")))
            .await
            .unwrap();
        let dm = SessionKey::new("slack", None::<String>, "D1", None::<String>, "U1");
        egress
            .dispatch(send(dm, "hi", Some("1700.0010")))
            .await
            .unwrap();
        let id = Uuid::new_v4();
        egress
            .dispatch(OutboundAction::Stream {
                session: channel_session(),
                chunk: StreamChunk {
                    stream_id: id,
                    sequence: 0,
                    content: "done".into(),
                    is_final: true,
                    reply_to: Some("1700.0011".into()),
                },
            })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests[0].1["thread_ts"], "1700.0009");
        assert!(requests[1].1.get("thread_ts").is_none());
        assert_eq!(requests[2].1["thread_ts"], "1700.0011");
    }

    #[tokio::test]
    async fn media_directive_uploads_file_and_strips_directive() {
        let server = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("chart.txt");
        std::fs::write(&file, "data").unwrap();
        let content = format!("Here it is\nMEDIA:{}", file.display());

        server
            .egress()
            .dispatch(send(session(), &content, None))
            .await
            .unwrap();

        assert_eq!(
            server.paths(),
            [
                "files.getUploadURLExternal",
                "upload/F1",
                "files.completeUploadExternal",
                "chat.postMessage",
            ]
        );
        let requests = server.requests();
        assert_eq!(requests[2].1["channel_id"], "C1");
        assert_eq!(requests[2].1["thread_ts"], "1699.0001");
        assert_eq!(requests[3].1["text"], "Here it is");
    }

    #[tokio::test]
    async fn media_failures_note_in_text_and_report_after_delivery() {
        let server = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join(".env");
        std::fs::write(&secret, "TOKEN=x").unwrap();
        let missing = dir.path().join("gone.png");

        let missing_only = format!("text\nMEDIA:{}", missing.display());
        server
            .egress()
            .dispatch(send(session(), &missing_only, None))
            .await
            .unwrap();
        assert_eq!(
            posted_texts(&server),
            ["text\n\n_⚠️ 파일 첨부 실패: gone.png_"]
        );

        server.clear();
        let denied = format!("text\nMEDIA:{}", secret.display());
        let error = server
            .egress()
            .dispatch(send(session(), &denied, None))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("outside authorized roots"),
            "{error}"
        );
        assert_eq!(server.paths(), ["chat.postMessage"]);
        assert_eq!(posted_texts(&server), ["text\n\n_⚠️ 파일 첨부 실패: .env_"]);
    }

    #[tokio::test]
    async fn media_directive_in_stream_final_is_uploaded_once() {
        let server = fake().await;
        let egress = server.egress();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("out.txt");
        std::fs::write(&file, "x").unwrap();
        let id = Uuid::new_v4();
        let content = format!("done\nMEDIA:{}", file.display());

        egress.dispatch(stream(id, 0, "do", false)).await.unwrap();
        egress
            .dispatch(stream(id, 1, &content, true))
            .await
            .unwrap();
        egress
            .dispatch(stream(id, 1, &content, true))
            .await
            .unwrap();

        assert_eq!(
            server.paths(),
            [
                "chat.postMessage",
                "files.getUploadURLExternal",
                "upload/F1",
                "files.completeUploadExternal",
                "chat.update",
            ]
        );
        assert_eq!(server.requests()[4].1["text"], "done");
    }

    #[tokio::test]
    async fn upload_guards_reject_secrets_and_oversize_before_reading() {
        let server = fake().await;
        let egress = server.egress().with_upload_limit(4);
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.bin");
        std::fs::write(&big, "0123456789").unwrap();
        let key = dir.path().join("id.pem");
        std::fs::write(&key, "k").unwrap();
        let small = dir.path().join("ok.txt");
        std::fs::write(&small, "ok").unwrap();

        let upload = |path: &Path| OutboundAction::UploadFile {
            session: session(),
            path: path.to_path_buf(),
        };
        let error = egress.dispatch(upload(&big)).await.unwrap_err();
        assert!(
            error.to_string().contains("over the 4 byte limit"),
            "{error}"
        );
        let error = egress.dispatch(upload(&key)).await.unwrap_err();
        assert!(error.to_string().contains("secret-bearing"), "{error}");
        assert!(server.requests().is_empty());

        egress.dispatch(upload(&small)).await.unwrap();
        assert_eq!(server.paths().len(), 3);
    }

    #[tokio::test]
    async fn markdown_table_is_uploaded_as_png_in_the_same_thread() {
        let server = fake().await;
        let content = "Results:\n\n| name | score |\n|---|---|\n| a | 1 |\n| b | 2 |\n\nend";
        let (expected_text, tables) = transform_markdown_tables_to_images(content);

        server
            .egress()
            .dispatch(send(session(), content, None))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests[0].0, "chat.postMessage");
        assert_eq!(requests[0].1["text"], render(&expected_text)[0].as_str());
        if tables.is_empty() {
            assert_eq!(requests.len(), 1);
            return;
        }
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[1].1["filename"], "table_1.png");
        assert_eq!(requests[3].0, "files.completeUploadExternal");
        assert_eq!(requests[3].1["thread_ts"], "1699.0001");
    }

    #[tokio::test]
    async fn failed_table_upload_falls_back_to_code_block() {
        let tables = vec![RenderedTable {
            original_text: "| a | b |\n|---|---|\n| 1 | 2 |".into(),
            png_bytes: vec![1, 2, 3],
            filename: "table_1.png".into(),
        }];
        let server = fake().await;
        server.respond(
            "files.getUploadURLExternal",
            json!({ "ok": false, "error": "internal_error" }),
        );

        let error = server
            .egress()
            .deliver_tables("C1", Some("1699.0001"), &tables)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("internal_error"));
        let requests = server.requests();
        assert_eq!(requests[1].0, "chat.postMessage");
        assert_eq!(requests[1].1["thread_ts"], "1699.0001");
        assert_eq!(
            requests[1].1["text"],
            "```\n| a | b |\n|---|---|\n| 1 | 2 |\n```"
        );
    }

    #[tokio::test]
    async fn runtime_footer_is_appended_when_enabled() {
        let server = fake().await;
        server
            .egress()
            .with_runtime_footer(true)
            .with_default_model(Some("anthropic/claude-x".into()))
            .dispatch(send(session(), "answer", None))
            .await
            .unwrap();
        server
            .egress()
            .with_default_model(Some("anthropic/claude-x".into()))
            .dispatch(send(session(), "plain", None))
            .await
            .unwrap();
        assert_eq!(posted_texts(&server), ["answer\n\n_claude-x_", "plain"]);
    }

    #[tokio::test]
    async fn runaway_reply_is_capped_at_eight_messages_with_notice() {
        let server = fake().await;
        let reply = (0..4000)
            .map(|line| format!("line {line:05} of a very long reply"))
            .collect::<Vec<_>>()
            .join("\n");
        server
            .egress()
            .dispatch(send(session(), &reply, None))
            .await
            .unwrap();
        let texts = posted_texts(&server);
        assert_eq!(texts.len(), MAX_MESSAGES);
        assert!(texts[MAX_MESSAGES - 1].ends_with(super::super::mrkdwn::TRUNCATION_NOTICE));
        assert!(texts.iter().all(|text| text.len() <= MESSAGE_LIMIT));
    }

    #[tokio::test]
    async fn approval_request_mentions_allowed_users_when_enabled() {
        let server = fake().await;
        server
            .egress()
            .with_approval_mentions(vec!["U2".into(), "U1".into(), "U2".into()], true)
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id: Uuid::new_v4(),
                command: "ls".into(),
                reason: "list".into(),
            })
            .await
            .unwrap();
        server
            .egress()
            .with_approval_mentions(vec!["U1".into()], false)
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id: Uuid::new_v4(),
                command: "ls".into(),
                reason: "list".into(),
            })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests[0].1["text"], "<@U1> <@U2> Approval needed: list");
        assert_eq!(
            requests[0].1["blocks"][0]["text"]["text"],
            "<@U1> <@U2>\n*Approval needed*\nlist"
        );
        assert_eq!(requests[1].1["text"], "Approval needed: list");
        assert_eq!(
            requests[1].1["blocks"][0]["text"]["text"],
            "*Approval needed*\nlist"
        );
    }

    #[test]
    fn dead_target_reason_matches_channel_level_codes_only() {
        let error =
            |detail: &str| OmonError::Multiplexer(format!("slack chat.postMessage: {detail}"));
        assert_eq!(
            dead_target_reason(&error("channel_not_found")),
            Some("channel_not_found")
        );
        assert_eq!(
            dead_target_reason(&error("is_archived")),
            Some("is_archived")
        );
        assert_eq!(
            dead_target_reason(&error("not_in_channel")),
            Some("not_in_channel")
        );
        assert_eq!(
            dead_target_reason(&error("account_inactive")),
            Some("account_inactive")
        );
        assert_eq!(dead_target_reason(&error("msg_too_long")), None);
        assert_eq!(
            dead_target_reason(&OmonError::Config("channel_not_found".into())),
            None
        );
    }

    #[tokio::test]
    async fn dead_target_is_persisted_skipped_probed_and_cleared() {
        let pool = pool().await;
        let server = fake().await;
        server.respond(
            "chat.postMessage",
            json!({ "ok": false, "error": "channel_not_found" }),
        );
        let egress = server.egress().with_pool(pool.clone());

        egress
            .dispatch(send(session(), "one", None))
            .await
            .unwrap_err();
        let reason: String =
            sqlx::query_scalar("SELECT reason FROM slack_dead_targets WHERE channel_id = 'C1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(reason, "channel_not_found");
        let error = egress
            .dispatch(send(session(), "two", None))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("dead target short-circuited"),
            "{error}"
        );
        assert_eq!(server.paths(), ["chat.postMessage"]);

        server.clear();
        server.respond(
            "chat.postMessage",
            json!({ "ok": false, "error": "msg_too_long" }),
        );
        let other = SessionKey::new("slack", Some("T1"), "C2", None::<String>, "");
        server
            .egress()
            .with_pool(pool.clone())
            .dispatch(send(other, "x", None))
            .await
            .unwrap_err();

        server.clear();
        let restarted = server
            .egress()
            .with_pool(pool.clone())
            .with_probe_interval(Duration::ZERO);
        restarted
            .dispatch(send(session(), "three", None))
            .await
            .unwrap();
        assert_eq!(posted_texts(&server), ["three"]);
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM slack_dead_targets")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0);
        assert!(restarted.dead_targets.lock().is_empty());
    }

    #[tokio::test]
    async fn owner_thread_is_private_requires_private_owner_and_bot_only() {
        let owner = crate::slack::owner_id();
        let server = fake().await;
        let egress = server.egress();
        let private =
            SessionKey::new("slack", Some("T1"), "G1", Some("1.1"), "").with_bot_id("UBOT");

        server.respond(
            "conversations.info",
            json!({ "ok": true, "channel": { "id": "G1", "is_private": true } }),
        );
        server.respond(
            "conversations.members",
            json!({ "ok": true, "members": [&owner, "UBOT"] }),
        );
        assert!(egress.owner_thread_is_private(&private).await);

        server.respond(
            "conversations.members",
            json!({ "ok": true, "members": [&owner, "UBOT", "UOTHER"] }),
        );
        assert!(!egress.owner_thread_is_private(&private).await);

        server.respond(
            "conversations.members",
            json!({ "ok": true, "members": ["UBOT"] }),
        );
        assert!(!egress.owner_thread_is_private(&private).await);

        server.respond(
            "conversations.members",
            json!({ "ok": true, "members": [&owner, "UBOT"] }),
        );
        server.respond(
            "conversations.info",
            json!({ "ok": true, "channel": { "id": "G1", "is_private": false } }),
        );
        assert!(!egress.owner_thread_is_private(&private).await);

        server.respond(
            "conversations.info",
            json!({ "ok": true, "channel": { "id": "G1", "is_private": true } }),
        );
        let dm = SessionKey::new(
            "slack",
            None::<String>,
            "D1",
            None::<String>,
            owner.as_str(),
        )
        .with_bot_id("UBOT");
        assert!(!egress.owner_thread_is_private(&dm).await);
        let no_bot = SessionKey::new("slack", Some("T1"), "G1", Some("1.1"), "");
        assert!(!egress.owner_thread_is_private(&no_bot).await);
    }
}

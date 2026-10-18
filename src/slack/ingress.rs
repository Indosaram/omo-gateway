//! Events API ingress: maps Slack message events to `InboundEvent`s and routes them.
//!
//! Order per event, mirroring Discord `handle_event`: lifecycle events, mapping and dedupe,
//! owner-DM proposal commands, session-status interception, lane owner replies, text `/stop`,
//! context enrichment (reply/forward/topic/channel), processing reaction, instant steer, and
//! finally the split-message debouncer, which claims every constituent and routes one turn.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use super::debounce::{FlushSink, SlackDebouncer, DEFAULT_DEBOUNCE};
use super::{AllowBots, SlackApi, SlackConfig, MAX_CHANNEL_CONTEXT_LIMIT, MESSAGE_LIMIT, PLATFORM};
use crate::discord::adapter::{
    compose_reply_context, format_channel_context, format_channel_topic_context,
};
use crate::discord::attachments::{is_text_attachment, MAX_INLINED_ATTACHMENT_BYTES};
use crate::discord::pairing::SlackPairingStore;
use crate::{
    DeliveryLedgerService, InboundEvent, MessageAttachment, OmonError, Result, SessionKey,
    SmartApprovalGuard,
};

const PROCESSING_REACTION: &str = "eyes";
const PROPOSAL_FOOTER: &str = "\n\n이 메시지 스레드에 '<번호> 해'를 답장하면 승인됩니다.";

#[derive(Clone)]
struct Shared {
    config: Arc<SlackConfig>,
    api: SlackApi,
    multiplexer: crate::SessionMultiplexer,
    pool: SqlitePool,
    bot_user_id: String,
}

pub struct SlackIngress {
    shared: Shared,
    team_id: String,
    debouncer: SlackDebouncer,
    inbox_actions: Option<Arc<dyn crate::inbox::proposals::Actions>>,
    approvals: SmartApprovalGuard,
    pairing: Option<SlackPairingStore>,
    user_names: Arc<Mutex<HashMap<String, String>>>,
}

impl SlackIngress {
    pub fn new(
        config: Arc<SlackConfig>,
        api: SlackApi,
        multiplexer: crate::SessionMultiplexer,
        pool: SqlitePool,
        bot_user_id: String,
        team_id: String,
    ) -> Self {
        let shared = Shared {
            config,
            api,
            multiplexer,
            pool,
            bot_user_id,
        };
        let debouncer = SlackDebouncer::new(DEFAULT_DEBOUNCE, Arc::new(shared.clone()));
        Self {
            shared,
            team_id,
            debouncer,
            inbox_actions: None,
            approvals: SmartApprovalGuard::new(),
            pairing: None,
            user_names: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_inbox_actions(
        mut self,
        actions: Arc<dyn crate::inbox::proposals::Actions>,
    ) -> Self {
        self.inbox_actions = Some(actions);
        self
    }

    pub fn with_approvals(mut self, approvals: SmartApprovalGuard) -> Self {
        self.approvals = approvals;
        self
    }

    pub fn with_pairing(mut self, pairing: SlackPairingStore) -> Self {
        self.pairing = Some(pairing);
        self
    }

    #[cfg(test)]
    fn with_debounce(mut self, duration: std::time::Duration) -> Self {
        self.debouncer = SlackDebouncer::new(duration, Arc::new(self.shared.clone()));
        self
    }

    /// Routes every debounced message now; called on shutdown and drain.
    pub async fn flush(&self) {
        self.debouncer.flush_all().await;
    }

    pub fn config(&self) -> &SlackConfig {
        &self.shared.config
    }

    pub fn api(&self) -> &SlackApi {
        &self.shared.api
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.shared.pool
    }

    pub fn bot_user_id(&self) -> &str {
        &self.shared.bot_user_id
    }

    pub fn team_id(&self) -> &str {
        &self.team_id
    }

    pub fn multiplexer(&self) -> &crate::SessionMultiplexer {
        &self.shared.multiplexer
    }

    fn paired_users(&self) -> Vec<String> {
        self.pairing
            .as_ref()
            .map(SlackPairingStore::get_paired_user_ids_sync)
            .unwrap_or_default()
    }

    async fn known_thread(&self, event: &Value) -> Result<bool> {
        let Some(key) = thread_candidate(&self.shared.bot_user_id, &self.team_id, event) else {
            return Ok(false);
        };
        // A thread rooted by the bot (a cron or lane report posted from the channel session)
        // has no thread session yet, but a reply under it is addressed to the bot.
        if rooted_by_bot(event, &self.shared.bot_user_id) {
            return Ok(true);
        }
        session_exists(&self.shared.pool, &key).await
    }

    pub async fn handle(&self, event: Value) -> Result<()> {
        let s = &self.shared;
        if super::lifecycle::handle(&s.multiplexer, &s.pool, &s.bot_user_id, &event).await? {
            return Ok(());
        }
        if self.intercept_lane_reply(&event).await? {
            return Ok(());
        }
        let known_thread = self.known_thread(&event).await?;
        let paired = self.paired_users();
        let Some(mut inbound) = map_event(
            &s.config,
            &s.bot_user_id,
            &self.team_id,
            &event,
            known_thread,
            &paired,
        ) else {
            self.maybe_prompt_pairing(&event).await;
            return Ok(());
        };
        let delivery_id = delivery_id_of(&inbound);
        if DeliveryLedgerService::new(s.pool.clone())
            .is_duplicate(&delivery_id)
            .await?
            || self.debouncer.is_pending(&delivery_id).await
        {
            tracing::info!(delivery_id, "Ignoring duplicate Slack delivery");
            return Ok(());
        }
        let user = str_field(&event, "user").unwrap_or_default().to_owned();
        let channel = inbound.session.channel_id.clone();
        let ts = inbound.platform_message_id.clone();
        let dm = is_dm(&event, &channel);

        if dm
            && self
                .handle_owner_proposals(&event, &user, &channel, &ts)
                .await?
        {
            return Ok(());
        }

        let private_owner = event_is_plain(&event)
            && crate::sessions::status_reply::owner_dm(&inbound.session, &user, "");
        if s.multiplexer
            .intercept_session_status(&inbound, private_owner)
            .await?
        {
            return Ok(());
        }

        if is_stop(&inbound.content) {
            self.debouncer.cancel(&inbound.session).await;
            let interrupted = crate::discord::commands::stop_session(
                &s.pool,
                &self.approvals,
                &s.multiplexer,
                &inbound.session,
            )
            .await
            .map_err(|error| OmonError::Multiplexer(error.to_string()))?;
            tracing::info!(session = %inbound.session, interrupted, "processed Slack text stop command");
            return Ok(());
        }

        self.enrich(&mut inbound, &event, dm).await;

        if s.config.processing_reactions {
            if let Err(error) = s.api.add_reaction(&channel, &ts, PROCESSING_REACTION).await {
                tracing::warn!(%error, channel, "failed to add Slack processing reaction");
            }
        }

        if inbound.attachments.is_empty() && !inbound.content.trim().is_empty() {
            match steer_claimed_event(&s.pool, &s.multiplexer, &inbound, &delivery_id).await {
                Ok(true) => {
                    tracing::info!(session = %inbound.session, "steered Slack message into active turn");
                    advance_cursor(&s.pool, &inbound).await;
                    return Ok(());
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(session = %inbound.session, %error, "failed to steer Slack message into active turn");
                }
            }
        }

        inbound.delivery_id = Some(delivery_id);
        self.debouncer.enqueue(inbound).await;
        Ok(())
    }

    /// Owner DM inbox commands: "제안" lists pending proposals (each bound to the message that
    /// shows it) and "<n> 해" replied in that message's thread approves one.
    async fn handle_owner_proposals(
        &self,
        event: &Value,
        user: &str,
        channel: &str,
        ts: &str,
    ) -> Result<bool> {
        use crate::inbox::proposals::{
            approval_number, ApprovalResult, Proposals, Reply, OWNER_ID,
        };
        let s = &self.shared;
        if !crate::inbox::triage::enabled()
            || user != s.config.owner_id()
            || event.get("bot_id").is_some_and(|value| !value.is_null())
        {
            return Ok(false);
        }
        let text = str_field(event, "text").unwrap_or_default().trim();
        if text == "제안" {
            let proposals = Proposals::new(s.pool.clone(), OWNER_ID).await?;
            let pending = proposals.pending(channel).await?;
            let budget = MESSAGE_LIMIT - PROPOSAL_FOOTER.len();
            let mut groups: Vec<(String, Vec<i64>)> = Vec::new();
            for proposal in &pending {
                let mut entry = proposal.display();
                if entry.len() > budget {
                    entry = truncate_bytes(&entry, budget.saturating_sub(4));
                    entry.push('…');
                }
                match groups.last_mut() {
                    Some((text, numbers)) if text.len() + 2 + entry.len() <= budget => {
                        text.push_str("\n\n");
                        text.push_str(&entry);
                        numbers.push(proposal.number);
                    }
                    _ => groups.push((entry, vec![proposal.number])),
                }
            }
            for (text, numbers) in groups {
                let shown = s
                    .api
                    .post_message(channel, &format!("{text}{PROPOSAL_FOOTER}"), None, None)
                    .await?;
                for number in numbers {
                    proposals.presented(number, channel, &shown).await?;
                }
            }
            return Ok(true);
        }
        if approval_number(text).is_none() {
            return Ok(false);
        }
        let proposals = Proposals::new(s.pool.clone(), OWNER_ID).await?;
        let thread_ts = reply_thread_ts(event);
        let result = match self.inbox_actions.as_ref() {
            Some(actions) => {
                proposals
                    .approve(
                        Reply {
                            author_id: OWNER_ID,
                            is_bot: false,
                            channel_id: channel,
                            message_id: ts,
                            referenced_message_id: thread_ts,
                            content: text,
                        },
                        actions.as_ref(),
                    )
                    .await
            }
            None => Err(OmonError::Config("proposal executor unavailable".into())),
        };
        let reply = match result {
            Ok(ApprovalResult::Executed {
                owner_text: Some(text),
                ..
            }) => text,
            Ok(ApprovalResult::Executed { lane_key, .. }) => {
                format!(
                    "승인된 작업을 시작했습니다: {}",
                    lane_key.unwrap_or_default()
                )
            }
            Ok(ApprovalResult::AlreadyClaimed) => {
                "이미 처리됐거나 변경된 제안입니다. 재실행하지 않았습니다.".into()
            }
            Ok(ApprovalResult::NotMatched) => {
                "승인할 제안 메시지의 스레드에 '<번호> 해'를 답장해 주세요.".into()
            }
            Err(error) => format!("제안 실행 실패: {error}"),
        };
        for chunk in super::mrkdwn::split_message(&reply, MESSAGE_LIMIT) {
            s.api
                .post_message(channel, &chunk, Some(thread_ts.unwrap_or(ts)), None)
                .await?;
        }
        Ok(true)
    }

    /// Sends a throttled pairing code to a DM author who is not authorized. Operators approve it
    /// with `/omo-pair <code>`.
    async fn maybe_prompt_pairing(&self, event: &Value) {
        let s = &self.shared;
        let Some(pairing) = &self.pairing else {
            return;
        };
        let (Some(user), Some(channel)) = (str_field(event, "user"), str_field(event, "channel"))
        else {
            return;
        };
        if s.config.allow_all_users
            || str_field(event, "type") != Some("message")
            || !matches!(str_field(event, "subtype"), None | Some("file_share"))
            || event.get("bot_id").is_some_and(|value| !value.is_null())
            || user == s.bot_user_id
            || !is_dm(event, channel)
            || s.config.is_allowed(user)
            || pairing.is_user_paired_sync(user)
        {
            return;
        }
        let code = match pairing.check_and_record_notification(user).await {
            Ok(Some(code)) => code,
            Ok(None) => return,
            Err(error) => {
                tracing::error!(%error, "failed to evaluate Slack pairing notification");
                return;
            }
        };
        let prompt = format!(
            "🔒 *Authorization Required*\nThis bot requires operator pairing. Your pairing code is `{code}`.\nAsk an operator to approve your access with `/omo-pair {code}`."
        );
        if s.api
            .post_message(channel, &prompt, None, None)
            .await
            .is_ok()
        {
            let _ = pairing.record_confirmed_delivery_at(user, Utc::now()).await;
        }
    }

    /// Adds reply context (root of an unknown thread), channel topic and recent channel context.
    async fn enrich(&self, inbound: &mut InboundEvent, event: &Value, dm: bool) {
        let s = &self.shared;
        let channel = inbound.session.channel_id.clone();
        if let Some(thread_ts) = reply_thread_ts(event) {
            let root_known = session_exists(&s.pool, &inbound.session)
                .await
                .unwrap_or(false);
            if !root_known {
                self.add_reply_context(inbound, &channel, thread_ts).await;
            }
        }
        if dm {
            return;
        }
        if s.config.channel_topic_context {
            match s.api.conversations_info(&channel).await {
                Ok(info) => {
                    let topic =
                        text_at(&info, "/topic/value").or_else(|| text_at(&info, "/purpose/value"));
                    prepend(
                        &mut inbound.content,
                        format_channel_topic_context(topic, None),
                    );
                }
                Err(error) => {
                    tracing::warn!(%error, channel, "failed to fetch Slack channel topic")
                }
            }
        }
        let mentioned = str_field(event, "text")
            .is_some_and(|text| text.contains(&format!("<@{}>", s.bot_user_id)));
        let limit = s
            .config
            .channel_context_limit
            .min(MAX_CHANNEL_CONTEXT_LIMIT);
        if s.config.channel_context && mentioned && limit > 0 {
            let ts = inbound.platform_message_id.clone();
            let fetched = match reply_thread_ts(event) {
                Some(thread_ts) => {
                    s.api
                        .conversations_replies(&channel, thread_ts, None, (limit + 1) as u32)
                        .await
                }
                None => {
                    s.api
                        .conversations_history(&channel, None, Some(&ts), limit as u32)
                        .await
                }
            };
            match fetched {
                Ok(mut messages) => {
                    messages
                        .retain(|m| str_field(m, "ts").is_some_and(|t| ts_key(t) < ts_key(&ts)));
                    messages.sort_by_key(|m| ts_key(str_field(m, "ts").unwrap_or_default()));
                    let skip = messages.len().saturating_sub(limit);
                    let mut history = Vec::new();
                    for message in messages.into_iter().skip(skip) {
                        let text = str_field(&message, "text").unwrap_or_default();
                        let author = str_field(&message, "user").unwrap_or_default();
                        if text.trim().is_empty() || author == s.bot_user_id || author.is_empty() {
                            continue;
                        }
                        history.push((self.user_name(author).await, text.to_owned()));
                    }
                    prepend(&mut inbound.content, format_channel_context(&history));
                }
                Err(error) => {
                    tracing::warn!(%error, channel, "failed to fetch recent Slack channel context")
                }
            }
        }
    }

    async fn add_reply_context(&self, inbound: &mut InboundEvent, channel: &str, thread_ts: &str) {
        let root = match self
            .shared
            .api
            .conversations_replies(channel, thread_ts, None, 1)
            .await
        {
            Ok(messages) => messages
                .into_iter()
                .find(|m| str_field(m, "ts") == Some(thread_ts)),
            Err(error) => {
                tracing::warn!(%error, channel, thread_ts, "failed to fetch Slack thread root");
                None
            }
        };
        let Some(root) = root else {
            return;
        };
        let parent_files = attachments(&root);
        let mut quoted = str_field(&root, "text")
            .unwrap_or_default()
            .trim()
            .to_owned();
        if let Some(summary) = attachment_summary(&parent_files) {
            quoted = if quoted.is_empty() {
                summary
            } else {
                format!("{quoted} {summary}")
            };
        }
        let author = match str_field(&root, "user") {
            Some(user) => self.user_name(user).await,
            None => str_field(&root, "username")
                .or_else(|| text_at(&root, "/bot_profile/name"))
                .unwrap_or("unknown")
                .to_owned(),
        };
        inbound.content = compose_reply_context(&author, &quoted, &inbound.content);
        for file in parent_files {
            if !inbound
                .attachments
                .iter()
                .any(|existing| existing.id == file.id)
            {
                inbound.attachments.push(file);
            }
        }
    }

    async fn user_name(&self, user: &str) -> String {
        if let Some(name) = self.user_names.lock().await.get(user) {
            return name.clone();
        }
        let name = match self.shared.api.users_info(user).await {
            Ok(info) => ["/profile/display_name", "/real_name", "/name"]
                .iter()
                .find_map(|pointer| text_at(&info, pointer))
                .unwrap_or(user)
                .to_owned(),
            Err(error) => {
                tracing::debug!(%error, user, "failed to resolve Slack user name");
                return user.to_owned();
            }
        };
        self.user_names
            .lock()
            .await
            .insert(user.to_owned(), name.clone());
        name
    }

    /// Claims and routes one replayed event, resolving once its turn reached a terminal outcome.
    /// `Ok(false)` for duplicates and text `/stop` (already acted on live).
    pub async fn route_awaiting_turn(&self, mut event: InboundEvent) -> Result<bool> {
        if is_stop(&event.content) {
            tracing::info!(session = %event.session, "Skipping replay of Slack text stop command");
            return Ok(false);
        }
        let s = &self.shared;
        let delivery_id = delivery_id_of(&event);
        let ledger = DeliveryLedgerService::new(s.pool.clone());
        if !ledger.record_incoming_as(&event, &delivery_id).await? {
            return Ok(false);
        }
        s.hydrate(&mut event).await;
        event.delivery_id = Some(delivery_id.clone());
        let session_key = event.session.storage_key();
        let platform_message_id = event.platform_message_id.clone();
        let event_id = event.id.to_string();
        if let Err(error) = s.multiplexer.route_awaiting_turn(event).await {
            ledger.mark_failed(&delivery_id, error.to_string()).await?;
            let _ = sqlx::query(
                "DELETE FROM messages WHERE session_key = ? AND (id = ? OR (platform_message_id != '' AND platform_message_id = ?))",
            )
            .bind(&session_key)
            .bind(&event_id)
            .bind(&platform_message_id)
            .execute(&s.pool)
            .await;
            return Err(error);
        }
        Ok(true)
    }

    /// Maps a stored Slack message for backfill with the same rules as live ingress.
    pub(crate) fn map_replayed(&self, event: &Value, known_thread: bool) -> Option<InboundEvent> {
        map_event(
            &self.shared.config,
            &self.shared.bot_user_id,
            &self.team_id,
            event,
            known_thread,
            &self.paired_users(),
        )
    }
}

#[async_trait]
impl FlushSink for Shared {
    async fn flush(&self, mut event: InboundEvent, constituents: Vec<String>) -> Result<()> {
        let delivery_id = delivery_id_of(&event);
        let ledger = DeliveryLedgerService::new(self.pool.clone());
        let recorded = if constituents.len() <= 1 {
            ledger.record_incoming_as(&event, &delivery_id).await?
        } else {
            ledger
                .record_incoming_with_constituents(&event, &delivery_id, &constituents)
                .await?
        };
        if !recorded {
            tracing::info!(delivery_id, "Ignoring duplicate Slack delivery");
            return Ok(());
        }
        self.hydrate(&mut event).await;
        event.delivery_id = Some(delivery_id.clone());
        let cursor_event = event.clone();
        if let Err(error) = self.multiplexer.route(event).await {
            ledger.mark_failed(&delivery_id, error.to_string()).await?;
            return Err(error);
        }
        advance_cursor(&self.pool, &cursor_event).await;
        Ok(())
    }
}

impl Shared {
    /// Downloads attachments, inlines small text files and transcribes audio.
    async fn hydrate(&self, event: &mut InboundEvent) {
        let dir = self
            .config
            .attachment_dir
            .join(safe_component(&event.session.channel_id));
        let mut transcripts = Vec::new();
        for attachment in &mut event.attachments {
            let dest = dir.join(format!(
                "{}-{}",
                safe_component(&event.platform_message_id),
                safe_component(&attachment.filename)
            ));
            if let Err(error) = self.api.download_file(&attachment.url, &dest).await {
                tracing::warn!(
                    attachment_id = %attachment.id,
                    filename = %attachment.filename,
                    %error,
                    "failed to download Slack attachment; routing remote metadata only"
                );
                continue;
            }
            attachment.local_path = Some(dest.clone());
            if crate::voice_transcription::is_audio_attachment(
                attachment.content_type.as_deref(),
                &attachment.filename,
            ) {
                transcripts.push(crate::voice_transcription::transcribe_audio(&dest).await);
            } else if is_text_attachment(&attachment.filename, attachment.content_type.as_deref())
                && attachment.size_bytes.unwrap_or(0) <= MAX_INLINED_ATTACHMENT_BYTES
            {
                if let Ok(meta) = tokio::fs::metadata(&dest).await {
                    if meta.len() <= MAX_INLINED_ATTACHMENT_BYTES {
                        if let Ok(bytes) = tokio::fs::read(&dest).await {
                            attachment.text_content =
                                Some(String::from_utf8_lossy(&bytes).into_owned());
                        }
                    }
                }
            }
        }
        for transcript in transcripts {
            event.content = match transcript {
                Ok(text) => format!("🎤 [음성 전사]: {text}\n\n{}", event.content),
                Err(error) => {
                    tracing::warn!(%error, "failed to transcribe Slack audio attachment");
                    format!(
                        "{}\n\n_⚠️ 음성 전사 실패 (일반 오디오 첨부로 처리)_",
                        event.content
                    )
                }
            };
        }
    }
}

/// Injects the message into a turn already running in the session. Returns `Ok(false)` when no
/// turn is active; the ledger is written only once the steer landed.
async fn steer_claimed_event(
    pool: &SqlitePool,
    multiplexer: &crate::SessionMultiplexer,
    event: &InboundEvent,
    delivery_id: &str,
) -> Result<bool> {
    // A short owner reply answers a pending approval card instead of steering the turn.
    let answered = event.author_id.as_deref() == Some(crate::slack::owner_id().as_str())
        && multiplexer
            .resolve_owner_reply(&event.session, &event.content)
            .await
            .unwrap_or(false);
    if !answered && !multiplexer.steer(&event.session, &event.content).await? {
        return Ok(false);
    }
    let ledger = DeliveryLedgerService::new(pool.clone());
    if let Err(error) = ledger.record_incoming_as(event, delivery_id).await {
        tracing::warn!(delivery_id, %error, "failed to record steered Slack delivery");
    }
    if let Err(error) = crate::multiplexer::record_inbound_transcript(pool, event).await {
        tracing::warn!(%error, "failed to mirror steered Slack message into transcript");
    } else {
        let index = crate::MessageSearchIndex::new(pool.clone());
        match index.index_inbound(&event.session, event).await {
            Ok(true) => {
                if let Some(sender) = multiplexer.inbox_event_sender() {
                    if let Err(error) = crate::inbox::watchers::on_gateway_message(&sender).await {
                        tracing::warn!(%error, "Slack inbox event receiver is closed after message index commit");
                    }
                }
            }
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, "failed to index steered Slack message"),
        }
    }
    let _ = ledger.mark_delivered(delivery_id).await;
    Ok(true)
}

impl SlackIngress {
    /// Owner replies in a Slack lane thread are consumed by the lane (reopen/close) before the
    /// mention and thread gating of `map_event`, which would drop an unmentioned reply.
    async fn intercept_lane_reply(&self, event: &Value) -> Result<bool> {
        let s = &self.shared;
        let (Some(channel), Some(thread_ts), Some(user), Some(ts)) = (
            str_field(event, "channel"),
            reply_thread_ts(event),
            str_field(event, "user"),
            str_field(event, "ts"),
        ) else {
            return Ok(false);
        };
        let text = strip_mention(str_field(event, "text").unwrap_or_default(), &s.bot_user_id);
        if user != s.config.owner_id()
            || !matches!(str_field(event, "type"), Some("message" | "app_mention"))
            || !matches!(str_field(event, "subtype"), None | Some("thread_broadcast"))
            || !event_is_plain(event)
            || text.is_empty()
            || is_stop(&text)
            || !channel_admitted(
                channel,
                is_dm(event, channel),
                &s.config.allowed_channels,
                &s.config.ignored_channels,
            )
        {
            return Ok(false);
        }
        if !crate::lanes::slack_threads::intercept_owner_reply(
            &s.pool,
            channel,
            thread_ts,
            &s.bot_user_id,
            user,
            s.config.owner_id(),
            ts,
            &text,
        )
        .await?
        {
            return Ok(false);
        }
        if let Err(error) = crate::lanes::runner::wake(&s.pool).await {
            tracing::warn!(%error, channel, thread_ts, "lane wake after Slack owner reply failed");
        }
        Ok(true)
    }
}

async fn advance_cursor(pool: &SqlitePool, event: &InboundEvent) {
    let Some(bot_id) = event.session.bot_id.as_deref() else {
        return;
    };
    let thread = event.session.thread_id.as_deref().unwrap_or_default();
    if let Err(error) = write_cursor(
        pool,
        bot_id,
        &event.session.channel_id,
        thread,
        &event.platform_message_id,
    )
    .await
    {
        tracing::debug!(%error, channel = %event.session.channel_id, "failed to advance Slack cursor");
    }
}

pub(crate) async fn read_cursor(
    pool: &SqlitePool,
    bot_id: &str,
    channel: &str,
    thread_ts: &str,
) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT last_ts FROM slack_cursors WHERE bot_id = ? AND channel_id = ? AND thread_ts = ?",
    )
    .bind(bot_id)
    .bind(channel)
    .bind(thread_ts)
    .fetch_optional(pool)
    .await?)
}

/// Moves the cursor forward only; an older `ts` (out-of-order delivery) never rewinds it.
pub(crate) async fn write_cursor(
    pool: &SqlitePool,
    bot_id: &str,
    channel: &str,
    thread_ts: &str,
    ts: &str,
) -> Result<()> {
    if let Some(current) = read_cursor(pool, bot_id, channel, thread_ts).await? {
        if ts_key(&current) >= ts_key(ts) {
            return Ok(());
        }
    }
    sqlx::query(
        "INSERT INTO slack_cursors (bot_id, channel_id, thread_ts, last_ts, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(bot_id, channel_id, thread_ts) DO UPDATE SET last_ts = excluded.last_ts, updated_at = excluded.updated_at",
    )
    .bind(bot_id)
    .bind(channel)
    .bind(thread_ts)
    .bind(ts)
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn session_exists(pool: &SqlitePool, key: &SessionKey) -> Result<bool> {
    let row: Option<i64> = sqlx::query_scalar("SELECT 1 FROM sessions WHERE session_key = ?")
        .bind(key.storage_key())
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

fn rooted_by_bot(event: &Value, bot_user_id: &str) -> bool {
    reply_thread_ts(event).is_some() && str_field(event, "parent_user_id") == Some(bot_user_id)
}

fn thread_candidate(bot_user_id: &str, team_id: &str, event: &Value) -> Option<SessionKey> {
    let channel = str_field(event, "channel")?;
    if is_dm(event, channel) {
        return None;
    }
    let thread_ts = reply_thread_ts(event)?;
    Some(
        SessionKey::new(PLATFORM, Some(team_id), channel, Some(thread_ts), "")
            .with_bot_id(bot_user_id),
    )
}

impl SlackIngress {
    pub(crate) async fn is_known_thread(&self, event: &Value) -> Result<bool> {
        self.known_thread(event).await
    }
}

/// Pure mapping from a Slack `message` / `app_mention` event to an inbound event. `None` drops it.
pub fn map_event(
    config: &SlackConfig,
    bot_user_id: &str,
    team_id: &str,
    event: &Value,
    known_thread: bool,
    paired_users: &[String],
) -> Option<InboundEvent> {
    let kind = str_field(event, "type")?;
    match kind {
        "message" => {
            if !matches!(
                str_field(event, "subtype"),
                None | Some("file_share" | "thread_broadcast" | "bot_message")
            ) {
                return None;
            }
        }
        "app_mention" => {}
        _ => return None,
    }
    let text = str_field(event, "text").unwrap_or_default();
    let mention = format!("<@{bot_user_id}>");
    let mentioned = text.contains(&mention);
    let bot_id = str_field(event, "bot_id");
    if bot_id.is_some() || str_field(event, "subtype") == Some("bot_message") {
        match config.allow_bots {
            AllowBots::None => return None,
            AllowBots::Mentions if !mentioned => return None,
            AllowBots::Mentions | AllowBots::All => {}
        }
    }
    let user = str_field(event, "user").or(bot_id)?;
    if user == bot_user_id {
        return None;
    }
    if !(config.allow_all_users
        || config.is_allowed(user)
        || paired_users.iter().any(|id| id == user))
    {
        return None;
    }
    let channel = str_field(event, "channel")?;
    let ts = str_field(event, "ts")?;
    let dm = is_dm(event, channel);
    if !channel_admitted(
        channel,
        dm,
        &config.allowed_channels,
        &config.ignored_channels,
    ) {
        return None;
    }
    let reply_thread = reply_thread_ts(event);
    let free = config.free_response_channels.iter().any(|id| id == channel);

    let session = (if dm {
        SessionKey::new(PLATFORM, None::<String>, channel, reply_thread, user)
    } else {
        if kind == "message" {
            if mentioned {
                return None;
            }
            let implicit_thread = known_thread && !config.thread_require_mention;
            if !free && !implicit_thread {
                return None;
            }
        }
        // A top-level message (mention or free-response) roots its own thread session, so the
        // conversation the agent remembers is the same thread its replies and lane reports land in.
        let thread = reply_thread.or_else(|| config.auto_thread.then_some(ts));
        SessionKey::new(PLATFORM, Some(team_id), channel, thread, user)
    })
    .with_bot_id(bot_user_id);

    let mut content = strip_mention(text, bot_user_id);
    let (forward_block, forward_files) = forwarded(event);
    if !forward_block.is_empty() {
        content = if content.is_empty() {
            forward_block
        } else {
            format!("{content}\n\n{forward_block}")
        };
    }
    let mut attachments = attachments(event);
    for file in forward_files {
        if !attachments.iter().any(|existing| existing.id == file.id) {
            attachments.push(file);
        }
    }
    if content.is_empty() && attachments.is_empty() {
        return None;
    }

    let mut inbound = InboundEvent::message(session, ts, content)
        .with_delivery_id(format!("slack:{channel}:{ts}"))
        .with_author_id(user)
        .with_attachments(attachments);
    if let Some(thread_ts) = reply_thread {
        inbound = inbound.with_referenced_message_id(thread_ts);
    }
    if let Some(received_at) = ts_to_datetime(ts) {
        inbound = inbound.with_received_at(received_at);
    }
    Some(inbound)
}

/// Ignored channels always reject; a non-empty allowlist restricts channels but not DMs.
fn channel_admitted(channel: &str, dm: bool, allowed: &[String], ignored: &[String]) -> bool {
    if ignored.iter().any(|id| id == channel) {
        return false;
    }
    dm || allowed.is_empty() || allowed.iter().any(|id| id == channel)
}

/// Shared or forwarded messages arrive as `attachments[]` entries flagged `is_share` or
/// `is_msg_unfurl`; they become a `[Forwarded]` block plus their files.
fn forwarded(event: &Value) -> (String, Vec<MessageAttachment>) {
    let Some(items) = event.get("attachments").and_then(Value::as_array) else {
        return (String::new(), Vec::new());
    };
    let mut parts = Vec::new();
    let mut files = Vec::new();
    for item in items {
        let flagged = |name: &str| item.get(name).and_then(Value::as_bool).unwrap_or(false);
        if !flagged("is_share") && !flagged("is_msg_unfurl") {
            continue;
        }
        let item_files = attachments(item);
        let mut part = str_field(item, "text")
            .or_else(|| str_field(item, "fallback"))
            .unwrap_or_default()
            .trim()
            .to_owned();
        if let Some(summary) = attachment_summary(&item_files) {
            part = if part.is_empty() {
                summary
            } else {
                format!("{part} {summary}")
            };
        }
        if let Some(author) =
            str_field(item, "author_name").or_else(|| str_field(item, "author_subname"))
        {
            if !part.is_empty() {
                part = format!("{author}: {part}");
            }
        }
        if !part.is_empty() {
            parts.push(part);
        }
        files.extend(item_files);
    }
    let block = if parts.is_empty() {
        String::new()
    } else {
        format!("[Forwarded]\n{}", parts.join("\n"))
    };
    (block, files)
}

fn attachment_summary(files: &[MessageAttachment]) -> Option<String> {
    match files {
        [] => None,
        [one] => Some(format!("[Attachment: {}]", one.filename)),
        many => Some(format!(
            "[Attachments: {}]",
            many.iter()
                .map(|f| f.filename.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn delivery_id_of(event: &InboundEvent) -> String {
    event.delivery_id.clone().unwrap_or_else(|| {
        format!(
            "slack:{}:{}",
            event.session.channel_id, event.platform_message_id
        )
    })
}

fn is_stop(content: &str) -> bool {
    content.trim().eq_ignore_ascii_case("/stop")
}

fn strip_mention(text: &str, bot_user_id: &str) -> String {
    text.replace(&format!("<@{bot_user_id}>"), "")
        .trim()
        .to_owned()
}

/// A human-typed message with no files, shares or bot author.
fn event_is_plain(event: &Value) -> bool {
    event.get("bot_id").is_none_or(Value::is_null)
        && event
            .get("files")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        && forwarded(event).0.is_empty()
}

fn text_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
}

fn prepend(content: &mut String, block: String) {
    if block.is_empty() {
        return;
    }
    *content = if content.trim().is_empty() {
        block
    } else {
        format!("{block}\n\n{content}")
    };
}

fn truncate_bytes(text: &str, max: usize) -> String {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn str_field<'a>(event: &'a Value, field: &str) -> Option<&'a str> {
    event.get(field).and_then(Value::as_str)
}

fn is_dm(event: &Value, channel: &str) -> bool {
    match str_field(event, "channel_type") {
        Some(channel_type) => channel_type == "im",
        None => channel.starts_with('D'),
    }
}

fn reply_thread_ts(event: &Value) -> Option<&str> {
    let thread_ts = str_field(event, "thread_ts")?;
    (Some(thread_ts) != str_field(event, "ts")).then_some(thread_ts)
}

fn attachments(event: &Value) -> Vec<MessageAttachment> {
    let Some(files) = event.get("files").and_then(Value::as_array) else {
        return Vec::new();
    };
    files
        .iter()
        .filter_map(|file| {
            let id = str_field(file, "id")?;
            let url = str_field(file, "url_private_download")
                .or_else(|| str_field(file, "url_private"))?;
            Some(MessageAttachment {
                id: id.to_owned(),
                filename: str_field(file, "name").unwrap_or(id).to_owned(),
                url: url.to_owned(),
                content_type: str_field(file, "mimetype").map(str::to_owned),
                size_bytes: file.get("size").and_then(Value::as_u64),
                local_path: None,
                text_content: None,
            })
        })
        .collect()
}

/// Orderable key for a Slack `ts` (`secs.micros`); malformed parts sort as zero.
pub(crate) fn ts_key(ts: &str) -> (u64, u32) {
    let (secs, frac) = ts.split_once('.').unwrap_or((ts, ""));
    let micros = format!("{frac:0<6}")
        .get(..6)
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0);
    (secs.parse().unwrap_or(0), micros)
}

fn ts_to_datetime(ts: &str) -> Option<DateTime<Utc>> {
    let (secs, frac) = ts.split_once('.').unwrap_or((ts, ""));
    let secs = secs.parse::<i64>().ok()?;
    let micros = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<6}").get(..6)?.parse::<u32>().ok()?
    };
    DateTime::from_timestamp(secs, micros * 1_000)
}

fn safe_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0') {
                '_'
            } else {
                c
            }
        })
        .collect();
    match cleaned.as_str() {
        "" | "." | ".." => "_".to_owned(),
        _ => cleaned,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use axum::body::Body;
    use axum::extract::Request;
    use axum::response::Response;
    use parking_lot::Mutex as SyncMutex;
    use serde_json::json;

    use super::*;
    use crate::{Database, MultiplexerConfig, SessionMultiplexer};

    const BOT: &str = "UBOT";
    const TEAM: &str = "T1";
    const OWNER: &str = "UOWNER";

    fn config() -> SlackConfig {
        SlackConfig {
            allowed_users: vec![OWNER.into()],
            free_response_channels: vec!["CFREE".into()],
            auto_thread: true,
            attachment_dir: PathBuf::from("/tmp/omon-slack-test"),
            channel_context: false,
            channel_topic_context: false,
            ..SlackConfig::test_default()
        }
    }

    fn map(config: &SlackConfig, event: &Value, known: bool) -> Option<InboundEvent> {
        map_event(config, BOT, TEAM, event, known, &[])
    }

    fn channel_mention(kind: &str) -> Value {
        json!({
            "type": kind,
            "channel": "C1",
            "channel_type": "channel",
            "user": OWNER,
            "text": "<@UBOT> hello there",
            "ts": "1700000000.000100",
        })
    }

    fn dm(text: &str, ts: &str) -> Value {
        json!({ "type": "message", "channel": "D1", "channel_type": "im", "user": OWNER, "text": text, "ts": ts })
    }

    #[test]
    fn top_level_dm_maps_to_dm_session_without_thread() {
        let inbound = map(&config(), &dm("hi bot", "1700000000.000200"), false).unwrap();
        assert_eq!(
            inbound.session,
            SessionKey::new("slack", None::<String>, "D1", None::<String>, OWNER).with_bot_id(BOT)
        );
        assert_eq!(inbound.content, "hi bot");
        assert_eq!(
            inbound.delivery_id.as_deref(),
            Some("slack:D1:1700000000.000200")
        );
        assert_eq!(inbound.author_id.as_deref(), Some(OWNER));
        assert_eq!(inbound.referenced_message_id, None);
        assert_eq!(inbound.received_at.timestamp(), 1_700_000_000);
        assert_eq!(inbound.received_at.timestamp_subsec_micros(), 200);
    }

    #[test]
    fn thread_reply_sets_thread_session_and_referenced_message_id() {
        let mut event = dm("follow up", "1700000001.000000");
        event["thread_ts"] = json!("1700000000.000200");
        let inbound = map(&config(), &event, false).unwrap();
        assert_eq!(
            inbound.session.thread_id.as_deref(),
            Some("1700000000.000200")
        );
        assert_eq!(
            inbound.referenced_message_id.as_deref(),
            Some("1700000000.000200")
        );
    }

    #[test]
    fn top_level_channel_messages_root_their_own_thread_session() {
        let inbound = map(&config(), &channel_mention("app_mention"), false).unwrap();
        assert_eq!(
            inbound.session,
            SessionKey::new("slack", Some(TEAM), "C1", Some("1700000000.000100"), "")
                .with_bot_id(BOT)
        );
        assert_eq!(inbound.content, "hello there");

        let mut free = channel_mention("app_mention");
        free["channel"] = json!("CFREE");
        assert_eq!(
            map(&config(), &free, false)
                .unwrap()
                .session
                .thread_id
                .as_deref(),
            Some("1700000000.000100")
        );
        let mut free_plain = channel_mention("message");
        free_plain["channel"] = json!("CFREE");
        free_plain["text"] = json!("no mention here");
        assert_eq!(
            map(&config(), &free_plain, false)
                .unwrap()
                .session
                .thread_id
                .as_deref(),
            Some("1700000000.000100")
        );
        let mut free_reply = free_plain.clone();
        free_reply["ts"] = json!("1700000009.000000");
        free_reply["thread_ts"] = json!("1700000000.000100");
        assert_eq!(
            map(&config(), &free_reply, false).unwrap().session,
            map(&config(), &free_plain, false).unwrap().session
        );

        let mut no_thread = config();
        no_thread.auto_thread = false;
        assert_eq!(
            map(&no_thread, &channel_mention("app_mention"), false)
                .unwrap()
                .session
                .thread_id,
            None
        );
        assert!(map(&config(), &channel_mention("message"), false).is_none());
    }

    #[test]
    fn thread_reply_gating_honours_known_thread_and_require_mention() {
        let reply = json!({
            "type": "message", "channel": "C1", "channel_type": "channel", "user": OWNER,
            "text": "more context", "ts": "1700000005.000000", "thread_ts": "1700000000.000100",
        });
        assert!(map(&config(), &reply, false).is_none());
        assert!(map(&config(), &reply, true).is_some());
        let mut strict = config();
        strict.thread_require_mention = true;
        assert!(map(&strict, &reply, true).is_none());
    }

    #[test]
    fn reply_under_a_bot_rooted_thread_counts_as_known() {
        let mut reply = json!({
            "type": "message", "channel": "C1", "channel_type": "channel", "user": OWNER,
            "text": "make it a checklist", "ts": "1700000005.000000",
            "thread_ts": "1700000000.000100", "parent_user_id": "UBOT",
        });
        assert!(rooted_by_bot(&reply, "UBOT"));
        reply["parent_user_id"] = json!(OWNER);
        assert!(!rooted_by_bot(&reply, "UBOT"));
        let root = json!({ "type": "message", "channel": "C1", "user": "UBOT", "ts": "1.0", "thread_ts": "1.0", "parent_user_id": "UBOT" });
        assert!(!rooted_by_bot(&root, "UBOT"));
    }

    #[test]
    fn channel_lists_bots_and_pairing_control_admission() {
        let mut lists = config();
        lists.allowed_channels = vec!["CALLOWED".into()];
        lists.ignored_channels = vec!["CFREE".into()];
        assert!(map(&lists, &channel_mention("app_mention"), false).is_none());
        let mut allowed = channel_mention("app_mention");
        allowed["channel"] = json!("CALLOWED");
        assert!(map(&lists, &allowed, false).is_some());
        assert!(map(&lists, &dm("dm is exempt", "1.1"), false).is_some());
        let mut ignored = dm("x", "1.2");
        ignored["channel"] = json!("CFREE");
        ignored["channel_type"] = json!("channel");
        assert!(map(&lists, &ignored, false).is_none());

        let mut bot = channel_mention("app_mention");
        bot["bot_id"] = json!("B1");
        bot["user"] = json!("UOTHERBOT");
        let mut cfg = config();
        cfg.allow_all_users = true;
        assert!(map(&cfg, &bot, false).is_none());
        cfg.allow_bots = AllowBots::Mentions;
        assert!(map(&cfg, &bot, false).is_some());
        let mut unmentioned = bot.clone();
        unmentioned["text"] = json!("no mention");
        unmentioned["channel"] = json!("CFREE");
        unmentioned["type"] = json!("message");
        assert!(map(&cfg, &unmentioned, false).is_none());
        cfg.allow_bots = AllowBots::All;
        assert!(map(&cfg, &unmentioned, false).is_some());

        let mut stranger = dm("hi", "1.3");
        stranger["user"] = json!("USTRANGER");
        assert!(map(&config(), &stranger, false).is_none());
        assert!(map_event(
            &config(),
            BOT,
            TEAM,
            &stranger,
            false,
            &["USTRANGER".into()]
        )
        .is_some());
    }

    #[test]
    fn unsupported_subtypes_self_and_empty_are_dropped() {
        let base = dm("hi", "1.2");
        let mut own = base.clone();
        own["user"] = json!(BOT);
        assert!(map(&config(), &own, false).is_none());
        for subtype in ["message_changed", "message_deleted", "channel_join"] {
            let mut edited = base.clone();
            edited["subtype"] = json!(subtype);
            assert!(map(&config(), &edited, false).is_none(), "{subtype}");
        }
        let mut empty = base;
        empty["text"] = json!("   ");
        assert!(map(&config(), &empty, false).is_none());
    }

    #[test]
    fn files_and_forwarded_shares_become_content_and_attachments() {
        let event = json!({
            "type": "message", "subtype": "file_share", "channel": "D1", "channel_type": "im",
            "user": OWNER, "text": "look", "ts": "1.2",
            "files": [{ "id": "F1", "name": "a.png", "mimetype": "image/png", "size": 42,
                        "url_private_download": "https://files.slack.com/a/download" }, { "id": "F3" }],
            "attachments": [
                { "is_share": true, "author_name": "Kim", "text": "original words",
                  "files": [{ "id": "F2", "name": "b.txt", "url_private": "https://files.slack.com/b" }] },
                { "title": "plain unfurl", "text": "not a share" },
            ],
        });
        let inbound = map(&config(), &event, false).unwrap();
        assert_eq!(
            inbound.content,
            "look\n\n[Forwarded]\nKim: original words [Attachment: b.txt]"
        );
        let ids: Vec<&str> = inbound.attachments.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["F1", "F2"]);
        assert_eq!(
            inbound.attachments[0].url,
            "https://files.slack.com/a/download"
        );
    }

    #[test]
    fn ts_key_orders_slack_timestamps() {
        assert!(ts_key("1700000000.000100") < ts_key("1700000000.000200"));
        assert!(ts_key("999.9") < ts_key("1000.0"));
        assert_eq!(ts_key("12.5"), (12, 500_000));
        assert_eq!(safe_component("../etc/passwd"), ".._etc_passwd");
        assert_eq!(safe_component(".."), "_");
    }

    struct SignalRunner(tokio::sync::mpsc::UnboundedSender<InboundEvent>);

    #[async_trait]
    impl crate::AgentRunner for SignalRunner {
        async fn run(
            &self,
            _session: &mut crate::SessionContext,
            event: InboundEvent,
        ) -> Result<()> {
            let _ = self.0.send(event);
            Ok(())
        }
    }

    type Calls = Arc<SyncMutex<Vec<(String, Value)>>>;
    type Responder = dyn Fn(&str, &Value) -> Value + Send + Sync;

    async fn fake_slack(responder: Arc<Responder>) -> (String, Calls) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let calls: Calls = Arc::new(SyncMutex::new(Vec::new()));
        let recorded = calls.clone();
        let app = axum::Router::new().fallback(move |req: Request| {
            let recorded = recorded.clone();
            let responder = responder.clone();
            async move {
                let path = req.uri().path().to_owned();
                if let Some(name) = path.strip_prefix("/files/") {
                    recorded.lock().push((path.clone(), Value::Null));
                    return Response::builder()
                        .status(200)
                        .header("content-type", "text/plain")
                        .body(Body::from(format!("contents of {name}")))
                        .unwrap();
                }
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body: Value = crate::slack::api::decode_test_body(&body);
                let method = path.trim_start_matches("/api/").to_owned();
                let reply = responder(&method, &body);
                recorded.lock().push((method, body));
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(reply.to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        (base, calls)
    }

    fn ok() -> Arc<Responder> {
        Arc::new(|_: &str, _: &Value| json!({ "ok": true, "ts": "1800000000.000001" }))
    }

    struct Harness {
        ingress: SlackIngress,
        rx: tokio::sync::mpsc::UnboundedReceiver<InboundEvent>,
        calls: Calls,
        pool: SqlitePool,
        base: String,
    }

    async fn harness(config: SlackConfig, responder: Arc<Responder>) -> Harness {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            db.pool().clone(),
            Arc::new(SignalRunner(tx)),
            MultiplexerConfig::default(),
        );
        let (base, calls) = fake_slack(responder).await;
        let ingress = SlackIngress::new(
            Arc::new(config),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            db.pool().clone(),
            BOT.into(),
            TEAM.into(),
        )
        .with_debounce(Duration::from_secs(3600));
        Harness {
            ingress,
            rx,
            calls,
            pool: db.pool().clone(),
            base,
        }
    }

    async fn next(rx: &mut tokio::sync::mpsc::UnboundedReceiver<InboundEvent>) -> InboundEvent {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("turn should reach the runner")
            .expect("runner channel open")
    }

    fn methods(calls: &Calls) -> Vec<String> {
        calls
            .lock()
            .iter()
            .map(|(method, _)| method.clone())
            .collect()
    }

    #[tokio::test]
    async fn handle_routes_once_reacts_and_dedupes_redelivery() {
        let mut h = harness(config(), ok()).await;
        let mention = channel_mention("app_mention");
        h.ingress.handle(mention.clone()).await.unwrap();
        h.ingress.handle(mention.clone()).await.unwrap();
        h.ingress.flush().await;
        assert_eq!(next(&mut h.rx).await.content, "hello there");
        h.ingress.handle(mention).await.unwrap();
        h.ingress.handle(channel_mention("message")).await.unwrap();
        assert_eq!(methods(&h.calls), ["reactions.add"]);

        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM delivery_ledger WHERE message_id = 'slack:C1:1700000000.000100'",
        )
        .fetch_one(&h.pool)
        .await
        .unwrap();
        assert_eq!(rows, 1);
    }

    #[tokio::test]
    async fn flush_claims_constituents_routes_and_advances_cursor_forward_only() {
        let mut h = harness(config(), ok()).await;
        let event = map(&config(), &channel_mention("app_mention"), false).unwrap();
        h.ingress
            .shared
            .flush(event, vec!["slack:C1:1700000000.000100".into()])
            .await
            .unwrap();
        assert_eq!(next(&mut h.rx).await.content, "hello there");
        assert_eq!(
            read_cursor(&h.pool, BOT, "C1", "1700000000.000100")
                .await
                .unwrap()
                .as_deref(),
            Some("1700000000.000100")
        );
        write_cursor(&h.pool, BOT, "C1", "1700000000.000100", "1600000000.0")
            .await
            .unwrap();
        assert_eq!(
            read_cursor(&h.pool, BOT, "C1", "1700000000.000100")
                .await
                .unwrap()
                .as_deref(),
            Some("1700000000.000100"),
            "cursor never moves backwards"
        );
    }

    #[tokio::test]
    async fn split_messages_coalesce_into_one_turn_with_constituents() {
        let mut cfg = config();
        cfg.processing_reactions = false;
        let mut h = harness(cfg, ok()).await;
        h.ingress
            .handle(dm("part two", "1700000000.000200"))
            .await
            .unwrap();
        h.ingress
            .handle(dm("part one", "1700000000.000100"))
            .await
            .unwrap();
        h.ingress.flush().await;
        let routed = next(&mut h.rx).await;
        assert_eq!(routed.content, "part one\npart two");
        assert_eq!(
            routed.delivery_id.as_deref(),
            Some("slack:D1:1700000000.000200")
        );
        assert!(
            h.calls.lock().is_empty(),
            "processing_reactions=false must not react"
        );
        let constituents: Vec<String> = sqlx::query_scalar(
            "SELECT constituent_id FROM delivery_ledger_constituents ORDER BY constituent_id",
        )
        .fetch_all(&h.pool)
        .await
        .unwrap();
        assert_eq!(
            constituents,
            ["slack:D1:1700000000.000100", "slack:D1:1700000000.000200"]
        );
    }

    #[tokio::test]
    async fn text_stop_cancels_pending_batch_without_routing() {
        let mut cfg = config();
        cfg.processing_reactions = false;
        let mut h = harness(cfg, ok()).await;
        h.ingress
            .handle(dm("do the thing", "1.000001"))
            .await
            .unwrap();
        h.ingress.handle(dm("/stop", "1.000002")).await.unwrap();
        h.ingress.flush().await;
        assert!(
            h.rx.try_recv().is_err(),
            "cancelled batch must not reach the runner"
        );
    }

    #[tokio::test]
    async fn reply_context_channel_context_and_topic_are_prepended() {
        let mut cfg = config();
        cfg.channel_context = true;
        cfg.channel_topic_context = true;
        cfg.processing_reactions = false;
        let responder: Arc<Responder> = Arc::new(|method: &str, body: &Value| match method {
            "conversations.replies" => json!({ "ok": true, "messages": [
                { "ts": "1700000000.000001", "user": "UROOT", "text": "root question",
                  "files": [{ "id": "FROOT", "name": "root.png", "url_private": "http://127.0.0.1:1/root" }] },
                { "ts": "1700000000.000050", "user": "UPEER", "text": "peer remark" },
                { "ts": "1700000000.000060", "user": BOT, "text": "bot words" },
            ]}),
            "conversations.info" => {
                json!({ "ok": true, "channel": { "topic": { "value": "Ship v2" } } })
            }
            "users.info" => {
                let name = if body["user"] == "UROOT" {
                    "Root Person"
                } else {
                    "Peer"
                };
                json!({ "ok": true, "user": { "profile": { "display_name": name } } })
            }
            _ => json!({ "ok": true }),
        });
        let mut h = harness(cfg, responder).await;
        h.ingress.handle(json!({
            "type": "app_mention", "channel": "C1", "channel_type": "channel", "user": OWNER,
            "text": "<@UBOT> what now?", "ts": "1700000000.000100", "thread_ts": "1700000000.000001",
        })).await.unwrap();
        h.ingress.flush().await;
        let routed = next(&mut h.rx).await;
        assert_eq!(
            routed.content,
            "[Recent channel context]\nRoot Person: root question\nPeer: peer remark\n\n[Channel Topic]\nShip v2\n\n> [Replying to @Root Person]: root question [Attachment: root.png]\n\nwhat now?"
        );
        assert!(routed.attachments.iter().any(|a| a.id == "FROOT"));
        let users = methods(&h.calls)
            .iter()
            .filter(|m| *m == "users.info")
            .count();
        assert_eq!(users, 2, "user names are cached");
    }

    #[tokio::test]
    async fn text_attachments_are_downloaded_and_inlined() {
        let mut cfg = config();
        cfg.processing_reactions = false;
        let dir = tempfile::tempdir().unwrap();
        cfg.attachment_dir = dir.path().to_path_buf();
        let mut h = harness(cfg, ok()).await;
        let url = format!("{}/files/notes.txt", h.base);
        h.ingress.handle(json!({
            "type": "message", "subtype": "file_share", "channel": "D1", "channel_type": "im",
            "user": OWNER, "text": "", "ts": "1.5",
            "files": [{ "id": "F1", "name": "notes.txt", "mimetype": "text/plain", "size": 20, "url_private_download": url }],
        })).await.unwrap();
        h.ingress.flush().await;
        let routed = next(&mut h.rx).await;
        assert_eq!(
            routed.attachments[0].text_content.as_deref(),
            Some("contents of notes.txt")
        );
        assert!(routed.attachments[0]
            .local_path
            .as_ref()
            .is_some_and(|p| p.exists()));
    }

    #[tokio::test]
    async fn unauthorized_dm_gets_one_pairing_prompt() {
        let h = harness(config(), ok()).await;
        let ingress = h
            .ingress
            .with_pairing(SlackPairingStore::new(h.pool.clone()));
        let mut stranger = dm("let me in", "1.1");
        stranger["user"] = json!("USTRANGER");
        ingress.handle(stranger.clone()).await.unwrap();
        stranger["ts"] = json!("1.2");
        ingress.handle(stranger).await.unwrap();
        let posts: Vec<Value> = h
            .calls
            .lock()
            .iter()
            .filter(|(m, _)| m == "chat.postMessage")
            .map(|(_, b)| b.clone())
            .collect();
        assert_eq!(posts.len(), 1, "prompt is throttled");
        assert_eq!(posts[0]["channel"], "D1");
        assert!(posts[0]["text"].as_str().unwrap().contains("/omo-pair "));
    }

    #[tokio::test]
    async fn owner_reply_in_lane_thread_is_consumed_before_routing() {
        let mut h = harness(config(), ok()).await;
        sqlx::query("INSERT INTO lanes (key, title, lead_session_key, thread_channel_id, cwd, brief_path, session_key, status, done_signals, last_activity_at) VALUES ('lane-a', 't', 'lead', 'slack:C1:1700000000.000001', '/tmp', '/tmp/b', 's', 'closed', '[]', CURRENT_TIMESTAMP)")
            .execute(&h.pool).await.unwrap();
        sqlx::query("INSERT INTO lane_thread_bindings (lane_key, bot_id) VALUES ('lane-a', ?)")
            .bind(BOT)
            .execute(&h.pool)
            .await
            .unwrap();
        h.ingress.handle(json!({
            "type": "message", "channel": "C1", "channel_type": "channel", "user": OWNER,
            "text": "reopen please", "ts": "1700000000.000200", "thread_ts": "1700000000.000001",
        })).await.unwrap();
        let replies: Vec<(String, String)> =
            sqlx::query_as("SELECT message_id, text FROM lane_owner_replies")
                .fetch_all(&h.pool)
                .await
                .unwrap();
        assert_eq!(
            replies,
            vec![(
                "slack:C1:1700000000.000200".to_owned(),
                "reopen please".to_owned()
            )]
        );
        h.ingress.flush().await;
        assert!(h.rx.try_recv().is_err());
    }

    struct LookupActions;

    #[async_trait]
    impl crate::inbox::proposals::Actions for LookupActions {
        async fn start_lane(
            &self,
            _: &str,
            _: &crate::inbox::proposals::Proposal,
            _: &str,
            _: &str,
        ) -> Result<()> {
            Ok(())
        }
        async fn lookup(&self, proposal: &crate::inbox::proposals::Proposal) -> Result<String> {
            Ok(format!("looked up #{}", proposal.number))
        }
        async fn draft(&self, _: &crate::inbox::proposals::Proposal) -> Result<String> {
            Ok(String::new())
        }
    }

    #[tokio::test]
    async fn owner_dm_lists_proposals_and_approves_in_thread() {
        use crate::inbox::proposals::{Action, Proposals, OWNER_ID};
        let mut h = harness(config(), ok()).await;
        sqlx::query("INSERT INTO inbox_items (id, source, source_ref, title, snippet, received_at, link, raw_hash, collected_at) VALUES ('i1', 'mail', 'r1', 'Invoice', 's', CURRENT_TIMESTAMP, 'https://l', 'h', CURRENT_TIMESTAMP)")
            .execute(&h.pool).await.unwrap();
        sqlx::query("INSERT INTO inbox_decisions (item_id, class, reason, proposed_action, status, decided_at) VALUES ('i1', 'bot', 'r', 'check invoice', 'open', CURRENT_TIMESTAMP)")
            .execute(&h.pool).await.unwrap();
        let proposals = Proposals::new(h.pool.clone(), OWNER_ID).await.unwrap();
        let proposal = proposals
            .propose("i1", "D1", &Action::Lookup)
            .await
            .unwrap();

        let ingress = h.ingress.with_inbox_actions(Arc::new(LookupActions));
        ingress.handle(dm("제안", "1.1")).await.unwrap();
        let shown: Option<String> =
            sqlx::query_scalar("SELECT message_id FROM inbox_proposals WHERE number = ?")
                .bind(proposal.number)
                .fetch_one(&h.pool)
                .await
                .unwrap();
        assert_eq!(shown.as_deref(), Some("1800000000.000001"));

        let mut approve = dm(&format!("{} 해", proposal.number), "1.2");
        approve["thread_ts"] = json!("1800000000.000001");
        ingress.handle(approve).await.unwrap();
        let posts: Vec<Value> = h
            .calls
            .lock()
            .iter()
            .filter(|(m, _)| m == "chat.postMessage")
            .map(|(_, b)| b.clone())
            .collect();
        assert_eq!(posts.len(), 2);
        assert!(posts[0]["text"].as_str().unwrap().contains("check invoice"));
        assert_eq!(posts[1]["text"], format!("looked up #{}", proposal.number));
        assert_eq!(posts[1]["thread_ts"], "1800000000.000001");
        ingress.flush().await;
        assert!(
            h.rx.try_recv().is_err(),
            "proposal commands never start a turn"
        );
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

use super::approval::SmartApprovalGuard;
use super::attachments::AttachmentDownloader;
use crate::storage::PendingWriteScope as PendingScope;
use crate::{ChatMessage, LlmClient, OmonError, ProfileRouter, SessionKey, SessionMultiplexer};

pub type CommandError = Box<dyn std::error::Error + Send + Sync>;
pub type PoiseContext<'a> = poise::Context<'a, PoiseData, CommandError>;

#[derive(Clone)]
pub struct PoiseData {
    pub multiplexer: SessionMultiplexer,
    pub pool: SqlitePool,
    pub started_at: Instant,
    pub tools: Vec<String>,
    pub mcp_endpoints: Vec<String>,
    pub approvals: SmartApprovalGuard,
    pub free_response_channels: Vec<u64>,
    pub allowed_users: Vec<u64>,
    pub allowed_roles: Vec<u64>,
    pub allow_all_users: bool,
    pub thread_sessions_per_user: bool,
    pub allowed_channels: Vec<u64>,
    pub ignored_channels: Vec<u64>,
    /// Thread IDs the bot is actively participating in (created or @mentioned).
    /// Kept in-memory: fast, zero-overhead, sufficient for active gateway runtime lifecycle.
    pub active_threads: Arc<RwLock<HashSet<u64>>>,
    /// Durable thread ownership mapping (thread_id -> bot_id) cached in memory and backed by SQLite.
    pub thread_owners: Arc<RwLock<HashMap<u64, u64>>>,
    pub thread_require_mention: bool,
    pub allow_bots: super::adapter::AllowBotsMode,
    pub channel_topic_context: bool,
    pub auto_thread: bool,
    pub channel_context: bool,
    pub channel_context_limit: usize,
    pub processing_reactions: bool,
    pub approval_mentions: bool,
    pub approvals_deny: Vec<String>,
    pub runtime_footer: bool,
    pub destructive_slash_confirm: bool,
    pub primary_bot_id: Option<u64>,
    pub attachment_downloader: Option<AttachmentDownloader>,
    pub tool_registry: crate::ToolRegistry,
    pub llm: Option<LlmClient>,
    pub profile_router: ProfileRouter,
    pub pairing_store: super::PairingStore,
    pub missed_backfill: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatewayStats {
    pub active_sessions: usize,
    pub memory_count: i64,
    pub uptime: Duration,
    pub ledger_count: i64,
}

impl PoiseData {
    pub fn new(multiplexer: SessionMultiplexer, pool: SqlitePool) -> Self {
        let profile_router = multiplexer.profile_router().clone();
        let pairing_store = super::PairingStore::new(pool.clone());
        Self {
            multiplexer,
            pool,
            started_at: Instant::now(),
            tools: Vec::new(),
            mcp_endpoints: Vec::new(),
            approvals: SmartApprovalGuard::new(),
            free_response_channels: Vec::new(),
            allowed_users: Vec::new(),
            allowed_roles: Vec::new(),
            allow_all_users: false,
            thread_sessions_per_user: true,
            allowed_channels: Vec::new(),
            ignored_channels: Vec::new(),
            active_threads: Arc::new(RwLock::new(HashSet::new())),
            thread_owners: Arc::new(RwLock::new(HashMap::new())),
            thread_require_mention: false,
            allow_bots: super::adapter::AllowBotsMode::None,
            channel_topic_context: false,
            auto_thread: false,
            channel_context: false,
            channel_context_limit: 10,
            processing_reactions: true,
            approval_mentions: false,
            approvals_deny: Vec::new(),
            runtime_footer: false,
            destructive_slash_confirm: true,
            primary_bot_id: None,
            attachment_downloader: None,
            tool_registry: crate::ToolRegistry::new(),
            llm: None,
            profile_router,
            pairing_store,
            missed_backfill: false,
        }
    }

    pub fn mark_thread_active(&self, thread_id: u64) {
        if let Ok(mut set) = self.active_threads.write() {
            set.insert(thread_id);
        }
    }

    pub fn mark_thread_owner(&self, thread_id: u64, bot_id: u64) {
        if let Ok(mut map) = self.thread_owners.write() {
            map.insert(thread_id, bot_id);
        }
        if let Ok(mut set) = self.active_threads.write() {
            set.insert(thread_id);
        }
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let _ = crate::Database::record_thread_owner(&pool, thread_id, bot_id).await;
        });
    }

    pub async fn get_thread_owner_durable(&self, thread_id: u64) -> Option<u64> {
        if let Ok(map) = self.thread_owners.read() {
            if let Some(owner) = map.get(&thread_id).copied() {
                return Some(owner);
            }
        }
        if let Ok(Some(owner)) = crate::Database::get_thread_owner(&self.pool, thread_id).await {
            if let Ok(mut map) = self.thread_owners.write() {
                map.insert(thread_id, owner);
            }
            if let Ok(mut set) = self.active_threads.write() {
                set.insert(thread_id);
            }
            return Some(owner);
        }
        None
    }

    pub fn get_thread_owner_cached(&self, thread_id: u64) -> Option<u64> {
        self.thread_owners
            .read()
            .ok()
            .and_then(|map| map.get(&thread_id).copied())
    }

    pub fn is_thread_active(&self, thread_id: u64) -> bool {
        self.active_threads
            .read()
            .map(|set| set.contains(&thread_id))
            .unwrap_or(false)
    }

    pub async fn stats(&self) -> Result<GatewayStats, sqlx::Error> {
        let memory_count = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&self.pool)
            .await?;
        let ledger_count = sqlx::query_scalar("SELECT COUNT(*) FROM delivery_ledger")
            .fetch_one(&self.pool)
            .await?;
        Ok(GatewayStats {
            active_sessions: self.multiplexer.active_sessions(),
            memory_count,
            uptime: self.started_at.elapsed(),
            ledger_count,
        })
    }
}

pub fn all() -> Vec<poise::Command<PoiseData, CommandError>> {
    vec![
        model(),
        reset(),
        stop(),
        status(),
        tools(),
        skill(),
        skills(),
        memory(),
        cron(),
        steer(),
        undo(),
        retry(),
        compress(),
        title(),
        thread(),
        deny(),
        yolo(),
        pair(),
    ]
}

pub fn is_user_allowed(allowed_users: &[u64], user_id: u64) -> bool {
    is_user_authorized(user_id, &[], allowed_users, &[], false)
}

/// Evaluates user authorization based on user ID allowlist, role membership, and allow-all bypass.
pub fn is_user_authorized(
    user_id: u64,
    user_roles: &[u64],
    allowed_users: &[u64],
    allowed_roles: &[u64],
    allow_all_users: bool,
) -> bool {
    if allow_all_users {
        return true;
    }
    if !allowed_users.is_empty() && allowed_users.contains(&user_id) {
        return true;
    }
    if !allowed_roles.is_empty() && user_roles.iter().any(|r| allowed_roles.contains(r)) {
        return true;
    }
    false
}

/// Outcome of evaluating slash command admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandAdmissionResult {
    Allowed,
    UnauthorizedUser,
    UnauthorizedChannel,
    MissingGuildMetadata,
}

impl CommandAdmissionResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Evaluates whether a channel is admitted by channel allow/ignore lists.
pub fn is_channel_authorized(
    channel_id: u64,
    parent_channel_id: Option<u64>,
    allowed_channels: &[u64],
    ignored_channels: &[u64],
    is_dm: bool,
) -> bool {
    // 1. Blacklist: channel or its parent in ignored_channels -> rejected.
    if ignored_channels.contains(&channel_id) {
        return false;
    }
    if let Some(parent_id) = parent_channel_id {
        if ignored_channels.contains(&parent_id) {
            return false;
        }
    }

    // 2. Whitelist: if allowed_channels is non-empty, guild channels must be in allowed_channels
    // (or their parent must be in allowed_channels). DMs are exempt.
    if !is_dm && !allowed_channels.is_empty() {
        let channel_allowed = allowed_channels.contains(&channel_id);
        let parent_allowed = parent_channel_id
            .map(|pid| allowed_channels.contains(&pid))
            .unwrap_or(false);
        if !channel_allowed && !parent_allowed {
            return false;
        }
    }

    true
}

/// Channel identity and guild metadata resolution for slash command admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandChannelScope {
    pub channel_id: u64,
    pub parent_channel_id: Option<u64>,
    pub is_dm: bool,
    pub guild_metadata_available: bool,
}

impl CommandChannelScope {
    pub fn guild(channel_id: u64, parent_channel_id: Option<u64>) -> Self {
        Self {
            channel_id,
            parent_channel_id,
            is_dm: false,
            guild_metadata_available: true,
        }
    }

    pub fn dm(channel_id: u64) -> Self {
        Self {
            channel_id,
            parent_channel_id: None,
            is_dm: true,
            guild_metadata_available: true,
        }
    }

    pub fn missing_guild_metadata(channel_id: u64) -> Self {
        Self {
            channel_id,
            parent_channel_id: None,
            is_dm: false,
            guild_metadata_available: false,
        }
    }
}

/// Evaluates slash command admission at the command admission boundary.
pub fn check_slash_admission(
    user_id: u64,
    is_paired: bool,
    channel: CommandChannelScope,
    config: &super::adapter::InboundFilterConfig<'_>,
) -> CommandAdmissionResult {
    if !channel.guild_metadata_available {
        return CommandAdmissionResult::MissingGuildMetadata;
    }

    let user_authorized = is_paired
        || config.paired_users.contains(&user_id)
        || is_user_authorized(
            user_id,
            config.user_roles,
            config.allowed_users,
            config.allowed_roles,
            config.allow_all_users,
        );
    if !user_authorized {
        return CommandAdmissionResult::UnauthorizedUser;
    }

    let parent_id = channel.parent_channel_id.or(config.parent_channel_id);
    if !is_channel_authorized(
        channel.channel_id,
        parent_id,
        config.allowed_channels,
        config.ignored_channels,
        channel.is_dm,
    ) {
        return CommandAdmissionResult::UnauthorizedChannel;
    }

    CommandAdmissionResult::Allowed
}

pub async fn command_check(ctx: PoiseContext<'_>) -> Result<bool, CommandError> {
    let data = ctx.data();
    let user_id = ctx.author().id.get();
    let is_paired = data.pairing_store.is_user_paired(user_id).await;
    let user_roles: Vec<u64> = match ctx.author_member().await {
        Some(member) => member.roles.iter().map(|r| r.get()).collect(),
        None => Vec::new(),
    };
    let channel_id = ctx.channel_id().get();
    let channel_scope = if ctx.guild_id().is_some() {
        match ctx.channel_id().to_channel(ctx.serenity_context()).await {
            Ok(serenity::Channel::Guild(guild_channel)) => {
                CommandChannelScope::guild(channel_id, guild_channel.parent_id.map(|id| id.get()))
            }
            _ => CommandChannelScope::missing_guild_metadata(channel_id),
        }
    } else {
        CommandChannelScope::dm(channel_id)
    };

    let config = super::adapter::InboundFilterConfig {
        allowed_users: &data.allowed_users,
        allowed_roles: &data.allowed_roles,
        user_roles: &user_roles,
        allow_all_users: data.allow_all_users,
        allowed_channels: &data.allowed_channels,
        ignored_channels: &data.ignored_channels,
        ..Default::default()
    };

    match check_slash_admission(user_id, is_paired, channel_scope, &config) {
        CommandAdmissionResult::Allowed => Ok(true),
        CommandAdmissionResult::UnauthorizedUser => {
            ctx.send(
                poise::CreateReply::default()
                    .content("You are not authorized to use this command.")
                    .ephemeral(true),
            )
            .await?;
            Ok(false)
        }
        CommandAdmissionResult::UnauthorizedChannel => {
            ctx.send(
                poise::CreateReply::default()
                    .content("This command cannot be used in this channel.")
                    .ephemeral(true),
            )
            .await?;
            Ok(false)
        }
        CommandAdmissionResult::MissingGuildMetadata => {
            ctx.send(
                poise::CreateReply::default()
                    .content("Channel authorization check failed.")
                    .ephemeral(true),
            )
            .await?;
            Ok(false)
        }
    }
}

enum PendingAction<'a> {
    List,
    Review(&'a str),
    Approve(&'a str),
    Reject(&'a str),
}

async fn pending_command(
    pool: &SqlitePool,
    scope: PendingScope<'_>,
    action: PendingAction<'_>,
) -> crate::Result<poise::CreateReply> {
    let records = match action {
        PendingAction::List => crate::storage::list_pending_writes_scoped(pool, scope).await?,
        PendingAction::Review(id) => {
            vec![crate::storage::get_pending_write_scoped(pool, id, scope).await?]
        }
        PendingAction::Approve(id) => {
            let result = crate::storage::approve_pending_write_scoped(pool, id, scope).await?;
            let content = if result.is_some() {
                "Pending write approved."
            } else {
                "Pending write was already consumed."
            };
            return Ok(poise::CreateReply::default().content(content));
        }
        PendingAction::Reject(id) => {
            let rejected = crate::storage::reject_pending_write_scoped(pool, id, scope).await?;
            let content = if rejected {
                "Pending write rejected."
            } else {
                "Pending write was already consumed."
            };
            return Ok(poise::CreateReply::default().content(content));
        }
    };
    if records.is_empty() {
        return Ok(poise::CreateReply::default().content("No pending writes in this scope."));
    }
    let content = serde_json::to_vec_pretty(&records)
        .map_err(|error| OmonError::Database(error.to_string()))?;
    Ok(poise::CreateReply::default()
        .content(format!(
            "{} pending write(s). Download the complete payloads before approving an ID.",
            records.len()
        ))
        .attachment(serenity::CreateAttachment::bytes(
            content,
            "pending-writes.json",
        )))
}

#[poise::command(slash_command)]
/// Execute or inspect an OMO skill
pub async fn skill(
    ctx: PoiseContext<'_>,
    #[description = "Skill action: list, search, read, run, pending, review, approve, reject"]
    action: String,
    #[description = "Skill name, query, or pending ID"] name_or_query: Option<String>,
) -> Result<(), CommandError> {
    skill_dispatch(ctx, &action, name_or_query).await
}

#[poise::command(slash_command)]
/// Execute, inspect, or manage capability skills
pub async fn skills(
    ctx: PoiseContext<'_>,
    #[description = "Skill action: list, search, read, run, pending, review, approve, reject"]
    action: Option<String>,
    #[description = "Skill name, query, or pending ID"] name_or_query: Option<String>,
) -> Result<(), CommandError> {
    let act = action.unwrap_or_else(|| "list".to_string());
    skill_dispatch(ctx, &act, name_or_query).await
}

async fn skill_dispatch(
    ctx: PoiseContext<'_>,
    action: &str,
    name_or_query: Option<String>,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    let query_val = name_or_query.clone().unwrap_or_default();
    let pool = &data.pool;

    match action {
        "pending" | "review" | "approve" | "apply" | "reject" | "deny" | "drop" => {
            let id = query_val.trim();
            let action = match action {
                "pending" if id.is_empty() => PendingAction::List,
                "pending" | "review" => PendingAction::Review(id),
                "approve" | "apply" => PendingAction::Approve(id),
                _ => PendingAction::Reject(id),
            };
            ctx.send(pending_command(pool, PendingScope::Skills, action).await?)
                .await?;
        }
        "list" => {
            let res = data
                .tool_registry
                .execute("skills", serde_json::json!({"action": "list"}))
                .await;
            match res {
                Ok(val) => {
                    let total = val
                        .get("total_skills")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let skills = val
                        .get("skills")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let list_str = skills
                        .iter()
                        .take(30)
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let reply_text = format!(
                        "📚 **Available OMO Skills** ({total} total):\n`{list_str}`\n*(Use `/skill action:read name_or_query:<skill_name>` to inspect)*"
                    );
                    for chunk in chunk_slash_reply(&reply_text, 2000) {
                        ctx.say(chunk).await?;
                    }
                }
                Err(e) => {
                    ctx.say(format!("❌ Failed to list skills: {e}")).await?;
                }
            }
        }
        "search" => {
            let res = data
                .tool_registry
                .execute(
                    "skills",
                    serde_json::json!({"action": "search", "query": query_val}),
                )
                .await;
            match res {
                Ok(val) => {
                    let count = val.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let matches = val
                        .get("matches")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let list_str = matches
                        .iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n- ");
                    ctx.say(format!("🔍 **Skill Search Results for `{query_val}`** ({count} matches):\n- {list_str}")).await?;
                }
                Err(e) => {
                    ctx.say(format!("❌ Search failed: {e}")).await?;
                }
            }
        }
        "read" => {
            let res = data
                .tool_registry
                .execute(
                    "skills",
                    serde_json::json!({"action": "read", "name": query_val}),
                )
                .await;
            match res {
                Ok(val) => {
                    let content = val
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No content");
                    let preview = skill_read_preview(content);
                    ctx.say(format!(
                        "📖 **Skill: `{query_val}`**\n```markdown\n{preview}\n```"
                    ))
                    .await?;
                }
                Err(e) => {
                    ctx.say(format!("❌ Could not read skill `{query_val}`: {e}"))
                        .await?;
                }
            }
        }
        "run" => {
            ctx.say(format!(
                "🚀 Injecting skill `{query_val}` into current OMO session..."
            ))
            .await?;
            let session_key = session_key(ctx).await?;
            let prompt = format!("Execute skill: {}", query_val);
            let event = crate::InboundEvent::message(session_key, ctx.id().to_string(), prompt);
            let _ = data.multiplexer.route(event).await;
        }
        _ => {
            ctx.say("Usage: `/skills action:<list|search|read|run|pending|review|approve|reject> name_or_query:<name_or_id>`")
                .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// Inspect or review persistent memories and pending memory writes
pub async fn memory(
    ctx: PoiseContext<'_>,
    #[description = "Memory action: pending, review, approve, reject, list"] action: Option<String>,
    #[description = "Pending write ID or query"] id_or_query: Option<String>,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let action_str = action.unwrap_or_else(|| "list".to_string()).to_lowercase();
    let pool = &ctx.data().pool;
    let key = session_key(ctx).await?;

    match action_str.as_str() {
        "pending" | "review" | "approve" | "apply" | "reject" | "deny" | "drop" => {
            let id = id_or_query.as_deref().unwrap_or("").trim();
            let action = match action_str.as_str() {
                "pending" if id.is_empty() => PendingAction::List,
                "pending" | "review" => PendingAction::Review(id),
                "approve" | "apply" => PendingAction::Approve(id),
                _ => PendingAction::Reject(id),
            };
            ctx.send(
                pending_command(pool, PendingScope::Memory(&key.storage_key()), action).await?,
            )
            .await?;
        }
        _ => {
            let memories: Vec<(String, String)> = sqlx::query_as(
                "SELECT id, content FROM memories WHERE session_key = ? ORDER BY updated_at DESC LIMIT 10",
            )
            .bind(key.storage_key())
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            if memories.is_empty() {
                ctx.say("No memories stored for this session.").await?;
            } else {
                let mut lines = vec![format!("**Stored memories ({})**:", memories.len())];
                for (id, content) in memories {
                    let preview = preview_text(&content, 100);
                    lines.push(format!("• `{id}`: {preview}"));
                }
                ctx.say(lines.join("\n")).await?;
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// Inspect or manage background cron jobs
pub async fn cron(
    ctx: PoiseContext<'_>,
    #[description = "Cron action: list, add, delete"] action: Option<String>,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    let act = action.unwrap_or_else(|| "list".to_string());

    match act.as_str() {
        "list" => {
            let res = data
                .tool_registry
                .execute("cron", serde_json::json!({"action": "list"}))
                .await;
            match res {
                Ok(val) => {
                    let count = val.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let jobs = val
                        .get("cron_jobs")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let mut lines = Vec::new();
                    for j in jobs.iter().take(15) {
                        let id = j.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                        let expr = j
                            .get("expression")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        let next = j
                            .get("next_run_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("paused");
                        lines.push(format!("• `{id}`: `{expr}` (Next: `{next}`)"));
                    }
                    let body = lines.join("\n");
                    ctx.say(format!(
                        "⏰ **Active OMO Cron Jobs** ({count} total):\n{body}"
                    ))
                    .await?;
                }
                Err(e) => {
                    ctx.say(format!("❌ Failed to list cron jobs: {e}")).await?;
                }
            }
        }
        _ => {
            ctx.say("Usage: `/cron [action:list]`").await?;
        }
    }
    Ok(())
}

pub async fn execute_model_command(
    data: &PoiseData,
    key: &SessionKey,
    name: &str,
) -> Result<(), CommandError> {
    ensure_session(&data.pool, key).await?;
    data.multiplexer.set_model(key, name.to_string()).await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Switch the model used by this Discord session.
pub async fn model(
    ctx: PoiseContext<'_>,
    #[description = "Model name"] name: String,
) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    execute_model_command(ctx.data(), &key, &name).await?;
    ctx.say(format!("Model switched to `{name}`.")).await?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResetCommandResult {
    NeedsConfirmation,
    Executed,
}

pub async fn execute_reset_command(
    data: &PoiseData,
    key: &SessionKey,
    confirm: Option<bool>,
) -> Result<ResetCommandResult, CommandError> {
    if data.destructive_slash_confirm && confirm != Some(true) {
        return Ok(ResetCommandResult::NeedsConfirmation);
    }
    reset_session(&data.pool, &data.approvals, key).await?;
    let _ = data.multiplexer.reset(key).await;
    Ok(ResetCommandResult::Executed)
}

pub async fn reset_session(
    pool: &SqlitePool,
    approvals: &SmartApprovalGuard,
    key: &SessionKey,
) -> Result<(), CommandError> {
    let storage_key = key.storage_key();
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM messages WHERE session_key = ?")
        .bind(&storage_key)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM memories WHERE session_key = ?")
        .bind(&storage_key)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE sessions SET state_json = '{}', updated_at = CURRENT_TIMESTAMP WHERE session_key = ?",
    )
    .bind(&storage_key)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    // Commit before guard/cache callbacks to prevent deadlock on max_connections(1)
    approvals.clear_session(key).await;
    Ok(())
}

#[poise::command(slash_command)]
/// Clear conversation context and persistent memory for this session.
pub async fn reset(
    ctx: PoiseContext<'_>,
    #[description = "Confirm destruction (true/false)"] confirm: Option<bool>,
) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let data = ctx.data();
    match execute_reset_command(data, &key, confirm).await? {
        ResetCommandResult::NeedsConfirmation => {
            ctx.say("⚠️ This will permanently clear conversation context and memories for this session. Re-run `/reset confirm:true` to proceed.").await?;
        }
        ResetCommandResult::Executed => {
            ctx.say("Session context and memory cleared.").await?;
        }
    }
    Ok(())
}

pub async fn stop_session(
    pool: &SqlitePool,
    approvals: &SmartApprovalGuard,
    multiplexer: &SessionMultiplexer,
    key: &SessionKey,
) -> Result<bool, CommandError> {
    let interrupted = multiplexer.stop(key).await?;
    let storage_key = key.storage_key();
    let existing: Option<String> =
        sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(&storage_key)
            .fetch_optional(pool)
            .await?;
    if let Some(state_json) = existing {
        let mut state: crate::SessionState = serde_json::from_str(&state_json)?;
        if state.yolo {
            state.yolo = false;
            sqlx::query(
                "UPDATE sessions SET state_json = ?, updated_at = CURRENT_TIMESTAMP WHERE session_key = ?",
            )
            .bind(serde_json::to_string(&state)?)
            .bind(&storage_key)
            .execute(pool)
            .await?;
        }
    }
    // Commit/persist before guard/cache callbacks to prevent deadlock on max_connections(1)
    // and ensure failed persistence never acknowledges unsaved change.
    approvals.set_yolo(key, false).await;
    Ok(interrupted)
}

#[poise::command(slash_command, prefix_command)]
/// Stop the active agent turn for this Discord session and mark it suspended.
pub async fn stop(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let interrupted = stop_session(
        &ctx.data().pool,
        &ctx.data().approvals,
        &ctx.data().multiplexer,
        &key,
    )
    .await?;
    let message = if interrupted {
        "🛑 Stopped active turn and marked session suspended."
    } else {
        "🛑 No active turn running; session marked suspended."
    };
    ctx.send(
        poise::CreateReply::default()
            .content(message)
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Show gateway health and persistence statistics.
pub async fn status(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let stats = ctx.data().stats().await?;
    ctx.say(format!(
        "Active sessions: {}\nMemory entries: {}\nUptime: {}s\nLedger entries: {}",
        stats.active_sessions,
        stats.memory_count,
        stats.uptime.as_secs(),
        stats.ledger_count
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// List configured tools and MCP endpoints.
pub async fn tools(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let tools = if ctx.data().tools.is_empty() {
        "(none)".to_owned()
    } else {
        ctx.data().tools.join(", ")
    };
    let endpoints = if ctx.data().mcp_endpoints.is_empty() {
        "(none)".to_owned()
    } else {
        ctx.data().mcp_endpoints.join("\n")
    };
    ctx.say(format!("Tools: {tools}\nMCP endpoints:\n{endpoints}"))
        .await?;
    Ok(())
}

async fn session_key(ctx: PoiseContext<'_>) -> Result<SessionKey, CommandError> {
    let guild_id = ctx.guild_id().map(|id| id.to_string());
    let channel_id = ctx.channel_id();
    let (session_channel_id, thread_id) = if guild_id.is_some() {
        match channel_id.to_channel(ctx.serenity_context()).await? {
            serenity::Channel::Guild(channel) if is_thread(channel.kind) => {
                // Match message ingress when parent metadata is unavailable.
                (
                    channel.parent_id.unwrap_or(channel_id),
                    Some(channel_id.to_string()),
                )
            }
            _ => (channel_id, None),
        }
    } else {
        (channel_id, None)
    };
    let user_id = if guild_id.is_some() {
        String::new()
    } else {
        ctx.author().id.to_string()
    };
    Ok(SessionKey::new(
        "discord",
        guild_id,
        session_channel_id.to_string(),
        thread_id,
        user_id,
    )
    .with_bot_id(ctx.serenity_context().cache.current_user().id.to_string()))
}

fn is_thread(kind: serenity::ChannelType) -> bool {
    matches!(
        kind,
        serenity::ChannelType::NewsThread
            | serenity::ChannelType::PublicThread
            | serenity::ChannelType::PrivateThread
    )
}

async fn ensure_session(pool: &SqlitePool, key: &SessionKey) -> Result<(), OmonError> {
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json)
         VALUES (?, ?, ?, ?, ?, ?, '{}') ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(key.storage_key())
    .bind(&key.platform)
    .bind(&key.guild_id)
    .bind(&key.channel_id)
    .bind(&key.thread_id)
    .bind(&key.user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Inject steering guidance into the session's ongoing or next turn.
pub async fn steer(
    ctx: PoiseContext<'_>,
    #[description = "Steering guidance for the agent"] text: String,
) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let prompt = format_steer_prompt(&text);

    // Prefer injecting into the turn that is already executing so the guidance lands
    // immediately; only fall back to queueing a fresh turn when nothing is running.
    let steered = ctx.data().multiplexer.steer(&key, &prompt).await?;
    if !steered {
        let event = crate::InboundEvent::message(key, ctx.id().to_string(), prompt);
        ctx.data().multiplexer.route(event).await?;
    }

    ctx.send(
        poise::CreateReply::default()
            .content(if steered {
                format!("🎯 Steering guidance applied to the running turn: `{text}`")
            } else {
                format!("🎯 Steering guidance queued: `{text}`")
            })
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Remove the most recent exchange from the session conversation history.
pub async fn undo(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let storage_key = key.storage_key();

    match undo_last_exchange(&ctx.data().pool, &storage_key).await? {
        Some(result) => {
            let user_preview = preview_text(&result.user_content, 120);
            let response = if let Some(assistant_content) = result.assistant_content {
                let assistant_preview = preview_text(&assistant_content, 120);
                format!(
                    "↩️ **Undid last exchange** ({} messages removed):\n• **User:** `{user_preview}`\n• **Assistant:** `{assistant_preview}`",
                    result.deleted_count
                )
            } else {
                format!(
                    "↩️ **Undid last message** ({} message removed):\n• **User:** `{user_preview}`",
                    result.deleted_count
                )
            };
            ctx.say(response).await?;
        }
        None => {
            ctx.say("No conversation history to undo.").await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// Re-run the last user message in this session.
pub async fn retry(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let storage_key = key.storage_key();

    let user_row: Option<(i64, String)> = sqlx::query_as(
        "SELECT sequence, content FROM messages WHERE session_key = ? AND role = 'user' ORDER BY sequence DESC LIMIT 1",
    )
    .bind(&storage_key)
    .fetch_optional(&ctx.data().pool)
    .await?;

    let Some((user_seq, user_content)) = user_row else {
        ctx.send(
            poise::CreateReply::default()
                .content("No previous user message found to retry.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    sqlx::query("DELETE FROM messages WHERE session_key = ? AND sequence >= ?")
        .bind(&storage_key)
        .bind(user_seq)
        .execute(&ctx.data().pool)
        .await?;

    let preview = preview_text(&user_content, 100);
    let event = crate::InboundEvent::message(key, ctx.id().to_string(), user_content);
    ctx.data().multiplexer.route(event).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("🔄 Retrying last message: `{preview}`"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Compress conversation history into a concise summary.
pub async fn compress(ctx: PoiseContext<'_>) -> Result<(), CommandError> {
    ctx.defer().await?;
    let key = session_key(ctx).await?;
    let storage_key = key.storage_key();

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT sequence, role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
    )
    .bind(&storage_key)
    .fetch_all(&ctx.data().pool)
    .await?;

    if rows.len() < 2 {
        ctx.say("Conversation history is too short to compress.")
            .await?;
        return Ok(());
    }

    let cutoff_sequence = rows.last().map(|(seq, _, _)| *seq).unwrap_or(0);
    let history: Vec<(String, String)> = rows.into_iter().map(|(_, r, c)| (r, c)).collect();
    let chars_before: usize = history.iter().map(|(_, c)| c.len()).sum();
    let prompt = build_compression_prompt(&history);

    let summary = if let Some(llm) = &ctx.data().llm {
        match llm.stream(&[ChatMessage::new("user", prompt)], &[]).await {
            Ok(mut stream) => {
                let mut acc = String::new();
                while let Some(chunk) = stream.next().await {
                    if let Ok(chunk) = chunk {
                        acc.push_str(&chunk.content);
                    }
                }
                if acc.trim().is_empty() {
                    fallback_summary(&history)
                } else {
                    acc.trim().to_string()
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "LLM compression stream failed, using fallback summary");
                fallback_summary(&history)
            }
        }
    } else {
        fallback_summary(&history)
    };

    let summary_content = format!("[Conversation Summary]\n{summary}");
    let chars_after = summary_content.len();

    replace_history_with_summary(
        &ctx.data().pool,
        &storage_key,
        cutoff_sequence,
        &summary_content,
    )
    .await?;

    let (before, after, pct) = calculate_compression_stats(chars_before, chars_after);
    ctx.say(format!(
        "🗜️ **Conversation Compressed**\n• **Before:** {before} chars\n• **After:** {after} chars\n• **Reduction:** {pct:.1}%"
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// Set or rename the current Discord thread title.
pub async fn title(
    ctx: PoiseContext<'_>,
    #[description = "New title for the thread"] text: String,
) -> Result<(), CommandError> {
    let channel_id = ctx.channel_id();
    let channel = channel_id.to_channel(ctx.serenity_context()).await?;
    match channel {
        serenity::Channel::Guild(guild_channel) if is_thread(guild_channel.kind) => {
            let builder = serenity::EditThread::new().name(text.trim());
            guild_channel
                .id
                .edit_thread(ctx.serenity_context(), builder)
                .await?;
            ctx.say(format!("🏷️ Thread renamed to `{}`.", text.trim()))
                .await?;
        }
        _ => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ `/title` can only be used inside a thread.")
                    .ephemeral(true),
            )
            .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// Create a new thread and optionally start a session in it.
pub async fn thread(
    ctx: PoiseContext<'_>,
    #[description = "Thread name"] name: String,
    #[description = "Optional first message to send in the thread"] message: Option<String>,
) -> Result<(), CommandError> {
    let channel_id = ctx.channel_id();
    let channel = channel_id.to_channel(ctx.serenity_context()).await?;
    match channel {
        serenity::Channel::Guild(guild_channel) => {
            if is_thread(guild_channel.kind) {
                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ Cannot create a thread inside an existing thread.")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            let builder = serenity::CreateThread::new(name.trim());
            let created_thread = channel_id
                .create_thread(ctx.serenity_context(), builder)
                .await?;
            let thread_id_u64 = created_thread.id.get();
            let bot_id = ctx.serenity_context().cache.current_user().id;
            ctx.data().mark_thread_owner(thread_id_u64, bot_id.get());

            if let Some(starter_msg) = message.filter(|m| !m.trim().is_empty()) {
                let user_id = if ctx.guild_id().is_some() {
                    String::new()
                } else {
                    ctx.author().id.to_string()
                };
                let thread_key = SessionKey::new(
                    "discord",
                    ctx.guild_id().map(|id| id.to_string()),
                    channel_id.to_string(),
                    Some(created_thread.id.to_string()),
                    user_id,
                )
                .with_bot_id(bot_id.to_string());

                let event =
                    crate::InboundEvent::message(thread_key, ctx.id().to_string(), starter_msg);
                let _ = ctx.data().multiplexer.route(event).await;
            }

            ctx.say(format!("🧵 Created thread <#{}>.", created_thread.id))
                .await?;
        }
        _ => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ Threads can only be created in server text channels.")
                    .ephemeral(true),
            )
            .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// Deny a pending dangerous command approval with an optional reason.
pub async fn deny(
    ctx: PoiseContext<'_>,
    #[description = "Optional reason explaining why the command was denied"] reason: Option<String>,
) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let resolved = ctx
        .data()
        .approvals
        .resolve_session_deny(&key, reason.clone())
        .await;
    if resolved {
        let msg = match reason {
            Some(r) if !r.trim().is_empty() => {
                format!("❌ Denied pending command approval: `{}`", r.trim())
            }
            _ => "❌ Denied pending command approval.".to_string(),
        };
        ctx.say(msg).await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .content("No pending approval found to deny.")
                .ephemeral(true),
        )
        .await?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YoloOutcome {
    pub enabled: bool,
    pub message: String,
}

pub async fn yolo_toggle(
    pool: &SqlitePool,
    approvals: &SmartApprovalGuard,
    key: &SessionKey,
    mode: Option<&str>,
) -> Result<YoloOutcome, CommandError> {
    ensure_session(pool, key).await?;
    let storage_key = key.storage_key();
    let state_json: String =
        sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(&storage_key)
            .fetch_one(pool)
            .await?;
    let mut state: crate::SessionState = serde_json::from_str(&state_json)?;
    let effective = approvals.is_yolo(key).await;
    let new_yolo = match mode.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("on" | "enable" | "true" | "yes" | "1") => true,
        Some("off" | "disable" | "false" | "no" | "0") => false,
        _ => !effective,
    };
    state.yolo = new_yolo;
    sqlx::query(
        "UPDATE sessions SET state_json = ?, updated_at = CURRENT_TIMESTAMP WHERE session_key = ?",
    )
    .bind(serde_json::to_string(&state)?)
    .bind(&storage_key)
    .execute(pool)
    .await?;

    // Commit/persist before guard/cache callbacks to prevent deadlock on max_connections(1)
    // and ensure failed persistence never acknowledges unsaved change.
    approvals.set_yolo(key, new_yolo).await;

    let status_str = if new_yolo { "enabled" } else { "disabled" };
    let note = if new_yolo {
        "\n⚠️ Unconditional hardline and deny rules are still enforced."
    } else {
        ""
    };
    Ok(YoloOutcome {
        enabled: new_yolo,
        message: format!("⚡ YOLO mode **{status_str}** for this session.{note}"),
    })
}

#[poise::command(slash_command)]
/// Toggle YOLO mode (approval bypass) for this session.
pub async fn yolo(
    ctx: PoiseContext<'_>,
    #[description = "Enable or disable YOLO mode (on/off)"] mode: Option<String>,
) -> Result<(), CommandError> {
    let key = session_key(ctx).await?;
    let outcome = yolo_toggle(
        &ctx.data().pool,
        &ctx.data().approvals,
        &key,
        mode.as_deref(),
    )
    .await?;
    ctx.send(
        poise::CreateReply::default()
            .content(outcome.message)
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
/// Authorize a new user via their one-time pairing code.
pub async fn pair(
    ctx: PoiseContext<'_>,
    #[description = "8-character pairing code (e.g. ABCD-EFGH)"] code: String,
) -> Result<(), CommandError> {
    let data = ctx.data();
    let user_roles: Vec<u64> = match ctx.author_member().await {
        Some(member) => member.roles.iter().map(|r| r.get()).collect(),
        None => Vec::new(),
    };
    if !is_user_authorized(
        ctx.author().id.get(),
        &user_roles,
        &data.allowed_users,
        &data.allowed_roles,
        data.allow_all_users,
    ) {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ You are not authorized to approve pairing codes.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    match data
        .pairing_store
        .approve_code(&code, ctx.author().id.get())
        .await?
    {
        super::PairingOutcome::Success { user_id } => {
            ctx.say(format!(
                "✅ Successfully paired and authorized user <@{user_id}>!"
            ))
            .await?;
        }
        super::PairingOutcome::InvalidCode => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ Invalid pairing code.")
                    .ephemeral(true),
            )
            .await?;
        }
        super::PairingOutcome::Expired => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ That pairing code has expired.")
                    .ephemeral(true),
            )
            .await?;
        }
        super::PairingOutcome::LockedOut => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ Pairing locked out due to too many failed attempts (5+).")
                    .ephemeral(true),
            )
            .await?;
        }
    }
    Ok(())
}

pub fn format_steer_prompt(text: &str) -> String {
    format!("[Steering] {}", text.trim())
}

pub fn preview_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() > max_len {
        let truncated: String = trimmed.chars().take(max_len.saturating_sub(3)).collect();
        format!("{truncated}...")
    } else {
        trimmed.to_string()
    }
}

pub fn skill_read_preview(content: &str) -> &str {
    if content.len() <= 1800 {
        return content;
    }
    let mut end = 1800;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    &content[..end]
}

pub fn chunk_slash_reply(content: &str, limit: usize) -> Vec<String> {
    crate::discord::chunk_markdown_paginated(content, limit, false)
}

pub fn build_compression_prompt(history: &[(String, String)]) -> String {
    let transcript = history
        .iter()
        .map(|(role, content)| format!("{role}: {content}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "Summarize the following conversation concisely, preserving key context, user requests, agent findings, facts, and decisions:\n\n{transcript}\n\nSummary:"
    )
}

pub fn fallback_summary(history: &[(String, String)]) -> String {
    let mut lines = Vec::new();
    for (role, content) in history {
        let first_line = content.lines().next().unwrap_or("").trim();
        let preview = if first_line.chars().count() > 80 {
            let truncated: String = first_line.chars().take(77).collect();
            format!("{truncated}...")
        } else {
            first_line.to_string()
        };
        if !preview.is_empty() {
            lines.push(format!("- {role}: {preview}"));
        }
    }
    lines.join("\n")
}

pub fn calculate_compression_stats(before_chars: usize, after_chars: usize) -> (usize, usize, f64) {
    let pct = if before_chars > 0 && before_chars >= after_chars {
        ((before_chars - after_chars) as f64 / before_chars as f64) * 100.0
    } else {
        0.0
    };
    (before_chars, after_chars, pct)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoResult {
    pub user_content: String,
    pub assistant_content: Option<String>,
    pub deleted_count: u64,
}

pub async fn undo_last_exchange(
    pool: &SqlitePool,
    session_key: &str,
) -> Result<Option<UndoResult>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let user_row: Option<(i64, String)> = sqlx::query_as(
        "SELECT sequence, content FROM messages WHERE session_key = ? AND role = 'user' ORDER BY sequence DESC LIMIT 1",
    )
    .bind(session_key)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((user_seq, user_content)) = user_row else {
        return Ok(None);
    };

    let assistant_row: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM messages WHERE session_key = ? AND role = 'assistant' AND sequence >= ? ORDER BY sequence DESC LIMIT 1",
    )
    .bind(session_key)
    .bind(user_seq)
    .fetch_optional(&mut *tx)
    .await?;

    let assistant_content = assistant_row.map(|(c,)| c);

    let delete_res = sqlx::query("DELETE FROM messages WHERE session_key = ? AND sequence >= ?")
        .bind(session_key)
        .bind(user_seq)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Some(UndoResult {
        user_content,
        assistant_content,
        deleted_count: delete_res.rows_affected(),
    }))
}

/// Replace the summarized prefix of a session's history with a single summary message.
///
/// Only rows at or before `cutoff_sequence` (the last sequence present in the snapshot that
/// was actually summarized) are deleted; messages that arrived during summarization survive.
/// The summary reuses `cutoff_sequence` so it stays ordered before those later messages.
pub async fn replace_history_with_summary(
    pool: &SqlitePool,
    session_key: &str,
    cutoff_sequence: i64,
    summary_content: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM messages WHERE session_key = ? AND sequence <= ?")
        .bind(session_key)
        .bind(cutoff_sequence)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO messages (sequence, id, session_key, role, content, metadata_json, created_at)
         VALUES (?, ?, ?, 'system', ?, '{\"compressed\": true}', CURRENT_TIMESTAMP)",
    )
    .bind(cutoff_sequence)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(session_key)
    .bind(summary_content)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn pending_review_preserves_session_kind_and_full_payload() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('A', 'discord', 'channel', 'owner', '{}')")
            .execute(pool).await.unwrap();
        let memory = serde_json::json!({
            "session_key": "A", "content": "private-sentinel", "metadata": {}
        });
        let id = crate::storage::stage_pending_write(pool, "memory", &memory.to_string())
            .await
            .unwrap();
        let mut failures = Vec::new();
        for action in [PendingAction::List, PendingAction::Review(&id)] {
            let reply = pending_command(pool, PendingScope::Memory("B"), action).await;
            if let Ok(reply) = reply {
                let data = format!(
                    "{}{}",
                    reply.content.unwrap_or_default(),
                    reply
                        .attachments
                        .iter()
                        .map(|a| String::from_utf8_lossy(&a.data))
                        .collect::<Vec<_>>()
                        .join("")
                );
                if data.contains("private-sentinel") {
                    failures.push("foreign session can review memory");
                }
            }
        }
        let result = pending_command(pool, PendingScope::Skills, PendingAction::Approve(&id)).await;
        if result.is_ok() {
            failures.push("skills approval accepts memory kind");
        }
        let id = crate::storage::stage_pending_write(pool, "memory", &memory.to_string())
            .await
            .unwrap();
        let _ = pending_command(pool, PendingScope::Memory("B"), PendingAction::Reject(&id)).await;
        if crate::storage::get_pending_write(pool, &id)
            .await
            .unwrap()
            .is_none()
        {
            failures.push("foreign session can reject memory");
        }
        let content = format!("{}tail-sentinel", "한".repeat(2500));
        let skill = serde_json::json!({"name": "review-skill", "content": content});
        let id = crate::storage::stage_pending_write(pool, "skill", &skill.to_string())
            .await
            .unwrap();
        let reply = pending_command(pool, PendingScope::Skills, PendingAction::Review(&id))
            .await
            .unwrap();
        let downloaded: Option<Vec<crate::storage::PendingWrite>> = reply
            .attachments
            .first()
            .and_then(|attachment| serde_json::from_slice(&attachment.data).ok());
        if downloaded
            .as_ref()
            .and_then(|items| items.first())
            .map(|record| record.payload.as_str())
            != Some(skill.to_string().as_str())
        {
            failures.push("full skill payload is not downloadable");
        }
        assert!(reply.content.as_deref().unwrap_or("").chars().count() <= 2000);
        pool.close().await;
        println!("U08 boundary failures: {failures:?}");
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn test_all_commands_count() {
        let commands = all();
        assert_eq!(commands.len(), 18);
        let names: HashSet<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        for expected in &[
            "model", "reset", "stop", "status", "tools", "skill", "skills", "memory", "cron",
            "steer", "undo", "retry", "compress", "title", "thread", "deny", "yolo", "pair",
        ] {
            assert!(names.contains(expected), "missing command {expected}");
        }
    }

    #[tokio::test]
    async fn pending_owner_can_download_apply_and_reject_memory() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('owner-scope', 'discord', 'channel', 'owner', '{}')")
            .execute(pool).await.unwrap();
        let payload = serde_json::json!({
            "session_key": "owner-scope",
            "content": format!("{}tail", "한".repeat(2500)),
            "metadata": {"source": "fixture"}
        });
        let id = crate::storage::stage_pending_write(pool, "memory", &payload.to_string())
            .await
            .unwrap();
        for action in [PendingAction::List, PendingAction::Review(&id)] {
            let reply = pending_command(pool, PendingScope::Memory("owner-scope"), action)
                .await
                .unwrap();
            let records: Vec<crate::storage::PendingWrite> =
                serde_json::from_slice(&reply.attachments[0].data).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].payload, payload.to_string());
        }
        pending_command(
            pool,
            PendingScope::Memory("owner-scope"),
            PendingAction::Approve(&id),
        )
        .await
        .unwrap();
        let saved: String =
            sqlx::query_scalar("SELECT content FROM memories WHERE session_key = 'owner-scope'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(saved, payload["content"].as_str().unwrap());
        let id = crate::storage::stage_pending_write(pool, "memory", &payload.to_string())
            .await
            .unwrap();
        pending_command(
            pool,
            PendingScope::Memory("owner-scope"),
            PendingAction::Reject(&id),
        )
        .await
        .unwrap();
        assert!(crate::storage::get_pending_write(pool, &id)
            .await
            .unwrap()
            .is_none());
        pool.close().await;
    }

    #[test]
    fn test_format_steer_prompt() {
        assert_eq!(
            format_steer_prompt("focus on Rust implementation"),
            "[Steering] focus on Rust implementation"
        );
        assert_eq!(
            format_steer_prompt("  padded whitespace  \n"),
            "[Steering] padded whitespace"
        );
    }

    #[test]
    fn test_preview_text() {
        assert_eq!(preview_text("short text", 50), "short text");
        assert_eq!(
            preview_text("this is a very long text that will be truncated", 20),
            "this is a very lo..."
        );
    }

    #[test]
    fn test_calculate_compression_stats() {
        let (before, after, pct) = calculate_compression_stats(1000, 250);
        assert_eq!(before, 1000);
        assert_eq!(after, 250);
        assert!((pct - 75.0).abs() < f64::EPSILON);

        let (_before, _after, pct) = calculate_compression_stats(0, 0);
        assert_eq!(pct, 0.0);

        let (_before, _after, pct) = calculate_compression_stats(100, 150);
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn test_fallback_summary_and_prompt_builder() {
        let history = vec![
            (
                "user".to_string(),
                "Hello, can you help me write a function?".to_string(),
            ),
            (
                "assistant".to_string(),
                "Sure! Here is the function:\nfn main() {}".to_string(),
            ),
        ];
        let prompt = build_compression_prompt(&history);
        assert!(prompt.contains("user: Hello, can you help me write a function?"));
        assert!(prompt.contains("assistant: Sure! Here is the function:"));

        let fallback = fallback_summary(&history);
        assert!(fallback.contains("- user: Hello, can you help me write a function?"));
        assert!(fallback.contains("- assistant: Sure! Here is the function:"));
    }

    #[tokio::test]
    async fn test_undo_last_exchange_empty_db() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let result = undo_last_exchange(db.pool(), "empty-session")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_undo_last_exchange_full_cycle() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let session_key = "test-undo-session";

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();

        // Turn 1
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m1', ?, 'user', 'turn 1 question')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m2', ?, 'assistant', 'turn 1 answer')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();

        // Turn 2
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m3', ?, 'user', 'turn 2 question')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m4', ?, 'tool', 'tool result')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m5', ?, 'assistant', 'turn 2 answer')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();

        // Undo turn 2
        let undo = undo_last_exchange(db.pool(), session_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(undo.user_content, "turn 2 question");
        assert_eq!(undo.assistant_content.as_deref(), Some("turn 2 answer"));
        assert_eq!(undo.deleted_count, 3);

        // Verify only turn 1 remains
        let remaining: Vec<(String, String)> = sqlx::query_as(
            "SELECT role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(db.pool())
        .await
        .unwrap();

        assert_eq!(remaining.len(), 2);
        assert_eq!(
            remaining[0],
            ("user".to_string(), "turn 1 question".to_string())
        );
        assert_eq!(
            remaining[1],
            ("assistant".to_string(), "turn 1 answer".to_string())
        );

        // Undo turn 1
        let undo1 = undo_last_exchange(db.pool(), session_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(undo1.user_content, "turn 1 question");
        assert_eq!(undo1.deleted_count, 2);

        let remaining_after: Vec<(String,)> =
            sqlx::query_as("SELECT content FROM messages WHERE session_key = ?")
                .bind(session_key)
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert!(remaining_after.is_empty());

        // Further undo returns None
        let undo_none = undo_last_exchange(db.pool(), session_key).await.unwrap();
        assert!(undo_none.is_none());
    }

    #[tokio::test]
    async fn test_retry_query_and_cleanup() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let session_key = "test-retry-session";

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();

        // No messages initially
        let user_row: Option<(i64, String)> = sqlx::query_as(
            "SELECT sequence, content FROM messages WHERE session_key = ? AND role = 'user' ORDER BY sequence DESC LIMIT 1",
        )
        .bind(session_key)
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert!(user_row.is_none());

        // Insert turn 1
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m1', ?, 'user', 'first prompt')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m2', ?, 'assistant', 'first response')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();

        // Insert turn 2 (failed/needs retry)
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m3', ?, 'user', 'retry this prompt')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES ('m4', ?, 'assistant', 'failed partial response')")
            .bind(session_key)
            .execute(db.pool())
            .await
            .unwrap();

        // Fetch last user message
        let (user_seq, user_content): (i64, String) = sqlx::query_as(
            "SELECT sequence, content FROM messages WHERE session_key = ? AND role = 'user' ORDER BY sequence DESC LIMIT 1",
        )
        .bind(session_key)
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(user_content, "retry this prompt");

        // Clean up sequence >= user_seq
        let res = sqlx::query("DELETE FROM messages WHERE session_key = ? AND sequence >= ?")
            .bind(session_key)
            .bind(user_seq)
            .execute(db.pool())
            .await
            .unwrap();
        assert_eq!(res.rows_affected(), 2);

        // Verify only turn 1 remains
        let remaining: Vec<(String, String)> = sqlx::query_as(
            "SELECT role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].1, "first prompt");
    }

    #[tokio::test]
    async fn test_compress_db_replacement() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let session_key = "test-compress-session";

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();

        for i in 0..5 {
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, 'user', ?)",
            )
            .bind(format!("um{i}"))
            .bind(session_key)
            .bind(format!("User message {i} with some content"))
            .execute(db.pool())
            .await
            .unwrap();
            sqlx::query("INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, 'assistant', ?)")
                .bind(format!("am{i}"))
                .bind(session_key)
                .bind(format!("Assistant response {i} with helpful details"))
                .execute(db.pool())
                .await
                .unwrap();
        }

        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT sequence, role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 10);

        let history: Vec<(String, String)> = rows.into_iter().map(|(_, r, c)| (r, c)).collect();
        let summary = fallback_summary(&history);
        let summary_content = format!("[Conversation Summary]\n{summary}");

        let mut tx = db.pool().begin().await.unwrap();
        sqlx::query("DELETE FROM messages WHERE session_key = ?")
            .bind(session_key)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at)
             VALUES (?, ?, 'system', ?, '{\"compressed\": true}', CURRENT_TIMESTAMP)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(session_key)
        .bind(&summary_content)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let after_rows: Vec<(String, String)> =
            sqlx::query_as("SELECT role, content FROM messages WHERE session_key = ?")
                .bind(session_key)
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert_eq!(after_rows.len(), 1);
        assert_eq!(after_rows[0].0, "system");
        assert!(after_rows[0].1.starts_with("[Conversation Summary]"));
    }

    #[tokio::test]
    async fn test_compress_preserves_messages_arriving_after_snapshot() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let session_key = "test-compress-race-session";

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();

        for i in 0..3 {
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, 'user', ?)",
            )
            .bind(format!("um{i}"))
            .bind(session_key)
            .bind(format!("User message {i}"))
            .execute(db.pool())
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, 'assistant', ?)",
            )
            .bind(format!("am{i}"))
            .bind(session_key)
            .bind(format!("Assistant response {i}"))
            .execute(db.pool())
            .await
            .unwrap();
        }

        // Snapshot taken by /compress before the (slow) LLM summarization.
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT sequence, role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 6);
        let cutoff_sequence = rows.last().map(|(seq, _, _)| *seq).unwrap();
        let history: Vec<(String, String)> = rows.into_iter().map(|(_, r, c)| (r, c)).collect();

        // A user message and its reply land while summarization is still in flight.
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content) VALUES ('late-user', ?, 'user', 'late question during compression')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content) VALUES ('late-assistant', ?, 'assistant', 'late answer during compression')",
        )
        .bind(session_key)
        .execute(db.pool())
        .await
        .unwrap();

        let summary_content = format!("[Conversation Summary]\n{}", fallback_summary(&history));
        replace_history_with_summary(db.pool(), session_key, cutoff_sequence, &summary_content)
            .await
            .unwrap();

        let after_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(db.pool())
        .await
        .unwrap();

        assert_eq!(
            after_rows.len(),
            3,
            "summary plus the two messages that arrived after the snapshot must remain, got {after_rows:?}"
        );
        assert_eq!(after_rows[0].0, "system");
        assert!(after_rows[0].1.starts_with("[Conversation Summary]"));
        assert_eq!(
            after_rows[1],
            (
                "user".to_string(),
                "late question during compression".to_string()
            )
        );
        assert_eq!(
            after_rows[2],
            (
                "assistant".to_string(),
                "late answer during compression".to_string()
            )
        );
    }

    #[test]
    fn test_is_thread_detection() {
        assert!(is_thread(serenity::ChannelType::PublicThread));
        assert!(is_thread(serenity::ChannelType::PrivateThread));
        assert!(is_thread(serenity::ChannelType::NewsThread));
        assert!(!is_thread(serenity::ChannelType::Text));
        assert!(!is_thread(serenity::ChannelType::Voice));
        assert!(!is_thread(serenity::ChannelType::Private));
    }

    struct DummyRunner;
    #[async_trait::async_trait]
    impl crate::AgentRunner for DummyRunner {
        async fn run(
            &self,
            _session: &mut crate::SessionContext,
            _event: crate::InboundEvent,
        ) -> crate::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn yolo_toggle_matches_effective_state_across_reset_and_restart() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let guard_1 = SmartApprovalGuard::new().with_pool(pool.clone());
        let runner = Arc::new(DummyRunner);
        let multiplexer =
            SessionMultiplexer::new(pool.clone(), runner, crate::MultiplexerConfig::default());

        let session_a = SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-1",
            None::<String>,
            "user-1",
        );
        let session_b = SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-2",
            None::<String>,
            "user-2",
        );

        let mut failures = Vec::new();

        // 1. Initial toggle on session_a: enable YOLO
        let outcome = yolo_toggle(pool, &guard_1, &session_a, Some("on"))
            .await
            .unwrap();
        if !outcome.enabled || !outcome.message.contains("enabled") {
            failures.push("initial enable did not return enabled outcome".to_string());
        }
        if !guard_1.is_yolo(&session_a).await {
            failures.push("guard_1 effective yolo is false after enable".to_string());
        }
        if guard_1.is_yolo(&session_b).await {
            failures.push("session_b unexpectedly has yolo enabled".to_string());
        }

        // Verify persisted state in DB for session_a
        let state_json_a: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(session_a.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        let state_a: crate::SessionState = serde_json::from_str(&state_json_a).unwrap();
        if !state_a.yolo {
            failures.push("persisted state in DB does not have yolo=true".to_string());
        }

        // 2. Process reload: create fresh guard with same SQLite pool and restore state
        let guard_2 = SmartApprovalGuard::new().with_pool(pool.clone());
        let loaded = guard_2.load_persisted_yolo().await.unwrap();
        if loaded != 1 {
            failures.push(format!("load_persisted_yolo returned {loaded}, expected 1"));
        }
        if !guard_2.is_yolo(&session_a).await {
            failures.push("recreated guard does not restore persisted yolo (effective is false while persisted is true)".to_string());
        }

        // Toggle on session_a with guard_2 (mode None toggles against effective state)
        let outcome_toggle = yolo_toggle(pool, &guard_2, &session_a, None).await.unwrap();
        if outcome_toggle.enabled || !outcome_toggle.message.contains("disabled") {
            failures.push("toggle from enabled state did not return disabled outcome".to_string());
        }
        if guard_2.is_yolo(&session_a).await {
            failures.push("guard_2 effective yolo is true after toggle off".to_string());
        }
        let state_json_after: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(session_a.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        let state_after: crate::SessionState = serde_json::from_str(&state_json_after).unwrap();
        if state_after.yolo {
            failures.push("persisted state in DB has yolo=true after toggle off".to_string());
        }

        // 2b. Twin-bot same-DM coords regression:
        // Enable only bot84 in same DM coordinates.
        // Recreate guard and restore persisted yolo.
        // 84 must true; 42 and botless alias false.
        let dm_bot84 = SessionKey::new(
            "discord",
            None::<String>,
            "dm-chan-99",
            None::<String>,
            "dm-user-1",
        )
        .with_bot_id("84");
        let dm_bot42 = SessionKey::new(
            "discord",
            None::<String>,
            "dm-chan-99",
            None::<String>,
            "dm-user-1",
        )
        .with_bot_id("42");
        let dm_botless = SessionKey::new(
            "discord",
            None::<String>,
            "dm-chan-99",
            None::<String>,
            "dm-user-1",
        );

        let outcome_twin = yolo_toggle(pool, &guard_1, &dm_bot84, Some("on"))
            .await
            .unwrap();
        if !outcome_twin.enabled {
            failures.push("bot84 enable outcome was not enabled".to_string());
        }
        if !guard_1.is_yolo(&dm_bot84).await {
            failures.push("guard_1 effective yolo is false for bot84".to_string());
        }
        if guard_1.is_yolo(&dm_bot42).await {
            failures.push("guard_1 unexpectedly enabled yolo for bot42".to_string());
        }
        if guard_1.is_yolo(&dm_botless).await {
            failures.push("guard_1 unexpectedly enabled yolo for botless alias".to_string());
        }

        let guard_twin = SmartApprovalGuard::new().with_pool(pool.clone());
        let _ = guard_twin.load_persisted_yolo().await;
        if !guard_twin.is_yolo(&dm_bot84).await {
            failures.push("recreated guard does not restore persisted yolo for bot84".to_string());
        }
        if guard_twin.is_yolo(&dm_bot42).await {
            failures.push("recreated guard unexpectedly restored yolo for bot42".to_string());
        }
        if guard_twin.is_yolo(&dm_botless).await {
            failures
                .push("recreated guard unexpectedly restored yolo for botless alias".to_string());
        }

        // 2c. Malformed session state must fail closed visibly, not silently skip/unwrap_or_default and erase other fields
        let malformed_session = SessionKey::new(
            "discord",
            None::<String>,
            "dm-chan-corrupt",
            None::<String>,
            "dm-user-corrupt",
        )
        .with_bot_id("84");
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', 'dm-chan-corrupt', 'dm-user-corrupt', 'INVALID_JSON_CORRUPT')"
        )
        .bind(malformed_session.storage_key())
        .execute(pool)
        .await
        .unwrap();

        let guard_corrupt = SmartApprovalGuard::new().with_pool(pool.clone());
        let malformed_load = guard_corrupt.load_persisted_yolo().await;
        if malformed_load.is_ok() {
            failures.push("load_persisted_yolo unexpectedly succeeded when malformed session state was present".to_string());
        }
        if guard_corrupt.is_yolo(&malformed_session).await {
            failures.push("guard unexpectedly granted yolo to malformed session".to_string());
        }

        let malformed_toggle = yolo_toggle(pool, &guard_1, &malformed_session, Some("on")).await;
        if malformed_toggle.is_ok() {
            failures
                .push("yolo_toggle unexpectedly succeeded on malformed session state".to_string());
        }
        let raw_json_after: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(malformed_session.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        if raw_json_after != "INVALID_JSON_CORRUPT" {
            failures
                .push("yolo_toggle erased or rewrote malformed session state in DB".to_string());
        }

        let malformed_stop = stop_session(pool, &guard_1, &multiplexer, &malformed_session).await;
        if malformed_stop.is_ok() {
            failures
                .push("stop_session unexpectedly succeeded on malformed session state".to_string());
        }
        let raw_json_after_stop: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(malformed_session.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        if raw_json_after_stop != "INVALID_JSON_CORRUPT" {
            failures
                .push("stop_session erased or rewrote malformed session state in DB".to_string());
        }

        sqlx::query("DELETE FROM sessions WHERE session_key = ?")
            .bind(malformed_session.storage_key())
            .execute(pool)
            .await
            .unwrap();

        // 3. Reset lifecycle: re-enable yolo and add cached approval on session_a; session_b unaffected
        yolo_toggle(pool, &guard_1, &session_a, Some("on"))
            .await
            .unwrap();
        guard_1.approve_session(&session_a, "pattern:cmd1").await;

        yolo_toggle(pool, &guard_1, &session_b, Some("on"))
            .await
            .unwrap();
        guard_1.approve_session(&session_b, "pattern:cmd2").await;

        reset_session(pool, &guard_1, &session_a).await.unwrap();

        if guard_1.is_yolo(&session_a).await {
            failures.push("reset did not clear guard yolo for session_a".to_string());
        }
        if guard_1.is_approved(&session_a, "pattern:cmd1").await {
            failures.push("reset did not clear cached approval grant for session_a".to_string());
        }
        if !guard_1.is_yolo(&session_b).await {
            failures.push("reset on session_a inadvertently cleared session_b yolo".to_string());
        }
        if !guard_1.is_approved(&session_b, "pattern:cmd2").await {
            failures.push(
                "reset on session_a inadvertently cleared session_b cached grant".to_string(),
            );
        }

        // 4. Completed stop lifecycle: session_b has yolo enabled and active_model set
        let state_json_b: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(session_b.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        let mut state_b: crate::SessionState = serde_json::from_str(&state_json_b).unwrap();
        state_b.active_model = Some("custom-model".to_string());
        sqlx::query("UPDATE sessions SET state_json = ? WHERE session_key = ?")
            .bind(serde_json::to_string(&state_b).unwrap())
            .bind(session_b.storage_key())
            .execute(pool)
            .await
            .unwrap();

        stop_session(pool, &guard_1, &multiplexer, &session_b)
            .await
            .unwrap();

        if guard_1.is_yolo(&session_b).await {
            failures.push("completed stop did not disable guard yolo for session_b".to_string());
        }
        let state_json_b_stopped: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(session_b.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        let state_b_stopped: crate::SessionState =
            serde_json::from_str(&state_json_b_stopped).unwrap();
        if state_b_stopped.yolo {
            failures.push(
                "completed stop did not disable persisted yolo in DB for session_b".to_string(),
            );
        }
        if state_b_stopped.active_model.as_deref() != Some("custom-model") {
            failures.push(
                "completed stop failed to preserve other session state fields (active_model)"
                    .to_string(),
            );
        }

        // 5. Failed persistence must not acknowledge unsaved change
        let closed_db = Database::connect("sqlite::memory:").await.unwrap();
        let guard_closed = SmartApprovalGuard::new().with_pool(closed_db.pool().clone());
        closed_db.pool().close().await;

        let failed_result =
            yolo_toggle(closed_db.pool(), &guard_closed, &session_a, Some("on")).await;
        if failed_result.is_ok() {
            failures
                .push("failed persistence unexpectedly acknowledged unsaved change".to_string());
        }
        if guard_closed.is_yolo(&session_a).await {
            failures.push("failed persistence modified runtime guard state".to_string());
        }

        database.pool().close().await;

        println!("U11 constituent failures: {failures:#?}");
        assert!(
            failures.is_empty(),
            "U11 constituent failures: {failures:#?}"
        );
    }
}

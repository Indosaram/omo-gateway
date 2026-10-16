use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use super::{SlackApi, SlackConfig, MESSAGE_LIMIT, PLATFORM};
use crate::discord::commands::{
    build_compression_prompt, calculate_compression_stats, collect_compression_summary,
    execute_model_command, execute_reset_command, format_steer_prompt, preview_text,
    replace_history_with_summary, retry_last_user_message, skill_read_preview, stop_session,
    undo_last_exchange, yolo_toggle, CommandError, ResetCommandResult,
};
use crate::sessions::control::{ControlError, SendMode, SessionControl, Target};
use crate::sessions::overview::{self, format_detail, format_overview};
use crate::sessions::watch;
use crate::storage::PendingWriteScope as PendingScope;
use crate::{
    ChatMessage, InboundEvent, LlmClient, OmonError, PoiseData, Result, SessionKey,
    SessionMultiplexer, SmartApprovalGuard, ToolRegistry,
};

/// Slack allows at most five posts per `response_url`.
const MAX_RESPONSE_URL_POSTS: usize = 5;
/// Mirrors the Discord `/compress` prompt budget (private there).
const MAX_COMPRESSION_PROMPT_CHARS: usize = 120_000;
const UNAUTHORIZED: &str = "You are not authorized to use this command.";
const OWNER_ONLY: &str = "이 명령은 소유자만 사용할 수 있어요.";

/// Every `/omo-*` command registered in `docs/slack-app-manifest.json`, with its usage.
pub const COMMANDS: [(&str, &str); 18] = [
    ("model", "/omo-model <name>"),
    ("reset", "/omo-reset [confirm]"),
    ("stop", "/omo-stop"),
    ("status", "/omo-status"),
    ("tools", "/omo-tools"),
    ("skill", "/omo-skill <list|search|read|run|pending|review|approve|reject> [name|query|id]"),
    ("skills", "/omo-skills [list|search|read|run|pending|review|approve|reject] [name|query|id]"),
    ("memory", "/omo-memory [list|pending|review|approve|reject] [id]"),
    ("cron", "/omo-cron [list]"),
    ("steer", "/omo-steer <guidance>"),
    ("undo", "/omo-undo"),
    ("retry", "/omo-retry"),
    ("compress", "/omo-compress"),
    ("title", "/omo-title <text>"),
    ("thread", "/omo-thread <name> [| first message]"),
    ("deny", "/omo-deny [reason]"),
    ("yolo", "/omo-yolo [on|off]"),
    (
        "sessions",
        "/omo-sessions overview | read <target> [limit] | send <target> [--mode auto|steer|follow_up] <text> | stop <target> | answer <target> [--request <id>] <text> | watch <target> | unwatch <target>",
    ),
];

/// What the Discord commands read from `PoiseData`, minus the Discord-only fields.
#[derive(Clone)]
pub struct SlackCommandDeps {
    pub multiplexer: SessionMultiplexer,
    pub pool: SqlitePool,
    pub approvals: SmartApprovalGuard,
    pub tools: Vec<String>,
    pub mcp_endpoints: Vec<String>,
    pub tool_registry: ToolRegistry,
    pub llm: Option<LlmClient>,
    pub started_at: Instant,
}

impl SlackCommandDeps {
    pub fn new(multiplexer: SessionMultiplexer, pool: SqlitePool) -> Self {
        Self {
            multiplexer,
            pool,
            approvals: SmartApprovalGuard::new(),
            tools: Vec::new(),
            mcp_endpoints: Vec::new(),
            tool_registry: ToolRegistry::new(),
            llm: None,
            started_at: Instant::now(),
        }
    }
}

pub struct SlackCommands {
    config: Arc<SlackConfig>,
    api: SlackApi,
    data: SlackCommandDeps,
    bot_user_id: String,
    /// Adapter so the Discord `execute_*` helpers and `stats()` run unchanged.
    poise: PoiseData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashInvocation {
    pub command: String,
    pub text: String,
    pub user_id: String,
    pub channel_id: String,
    pub team_id: String,
    pub response_url: Option<String>,
    pub thread_ts: Option<String>,
}

impl SlashInvocation {
    pub fn from_payload(payload: &Value) -> Self {
        let field = |name: &str| {
            payload
                .get(name)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let optional = |name: &str| {
            payload
                .get(name)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        };
        Self {
            command: field("command"),
            text: field("text"),
            user_id: field("user_id"),
            channel_id: field("channel_id"),
            team_id: field("team_id"),
            response_url: optional("response_url"),
            thread_ts: optional("thread_ts"),
        }
    }

    pub fn is_dm(&self) -> bool {
        self.channel_id.starts_with('D')
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionsCommand {
    Overview,
    Read { target: String, limit: Option<usize> },
    Send { target: String, text: String, mode: String },
    Stop { target: String },
    Answer { target: String, text: String, request_id: Option<String> },
    Watch { target: String },
    Unwatch { target: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlackCommand {
    Model(String),
    Reset { confirm: Option<bool> },
    Stop,
    Status,
    Tools,
    Skill { action: String, arg: Option<String> },
    Memory { action: String, arg: Option<String> },
    Cron { action: String },
    Steer(String),
    Undo,
    Retry,
    Compress,
    Title(String),
    Thread { name: String, message: Option<String> },
    Deny(Option<String>),
    Yolo(Option<String>),
    Sessions(SessionsCommand),
    Help,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub ephemeral: bool,
    pub text: String,
}

impl Reply {
    fn public(text: impl Into<String>) -> Self {
        Self { ephemeral: false, text: text.into() }
    }

    fn private(text: impl Into<String>) -> Self {
        Self { ephemeral: true, text: text.into() }
    }

    fn visible(ephemeral: bool, text: impl Into<String>) -> Self {
        Self { ephemeral, text: text.into() }
    }
}

/// Parses `/omo-<name>` plus its text. `Err` carries the usage line to show.
pub fn parse_command(command: &str, text: &str) -> std::result::Result<SlackCommand, String> {
    let name = command.trim().trim_start_matches('/');
    let name = name.strip_prefix("omo-").unwrap_or(name);
    let text = text.trim();
    let usage = || {
        COMMANDS
            .iter()
            .find(|(command, _)| *command == name)
            .map(|(_, usage)| format!("Usage: `{usage}`"))
            .unwrap_or_else(help_text)
    };
    let required = |value: &str| {
        if value.is_empty() {
            Err(usage())
        } else {
            Ok(value.to_owned())
        }
    };
    let (first, rest) = split_word(text);
    let rest_opt = (!rest.is_empty()).then(|| rest.to_owned());
    match name {
        "model" => required(text).map(SlackCommand::Model),
        "reset" => match text.to_ascii_lowercase().as_str() {
            "" => Ok(SlackCommand::Reset { confirm: None }),
            "confirm" | "confirm:true" | "true" | "yes" | "y" | "1" => {
                Ok(SlackCommand::Reset { confirm: Some(true) })
            }
            "confirm:false" | "false" | "no" | "n" | "0" => {
                Ok(SlackCommand::Reset { confirm: Some(false) })
            }
            _ => Err(usage()),
        },
        "stop" => Ok(SlackCommand::Stop),
        "status" => Ok(SlackCommand::Status),
        "tools" => Ok(SlackCommand::Tools),
        "skill" => {
            let action = required(first)?.to_ascii_lowercase();
            Ok(SlackCommand::Skill { action, arg: rest_opt })
        }
        "skills" => Ok(SlackCommand::Skill {
            action: if first.is_empty() { "list".into() } else { first.to_ascii_lowercase() },
            arg: rest_opt,
        }),
        "memory" => Ok(SlackCommand::Memory {
            action: if first.is_empty() { "list".into() } else { first.to_ascii_lowercase() },
            arg: rest_opt,
        }),
        "cron" => Ok(SlackCommand::Cron {
            action: if first.is_empty() { "list".into() } else { first.to_ascii_lowercase() },
        }),
        "steer" => required(text).map(SlackCommand::Steer),
        "undo" => Ok(SlackCommand::Undo),
        "retry" => Ok(SlackCommand::Retry),
        "compress" => Ok(SlackCommand::Compress),
        "title" => required(text).map(SlackCommand::Title),
        "thread" => {
            let (name, message) = match text.split_once('|') {
                Some((name, message)) => (name.trim(), Some(message.trim())),
                None => (text, None),
            };
            Ok(SlackCommand::Thread {
                name: required(name)?,
                message: message.filter(|m| !m.is_empty()).map(str::to_owned),
            })
        }
        "deny" => Ok(SlackCommand::Deny((!text.is_empty()).then(|| text.to_owned()))),
        "yolo" => Ok(SlackCommand::Yolo((!text.is_empty()).then(|| text.to_owned()))),
        "sessions" => parse_sessions(text).map(SlackCommand::Sessions).ok_or_else(usage),
        _ => Ok(SlackCommand::Help),
    }
}

fn parse_sessions(text: &str) -> Option<SessionsCommand> {
    let (sub, rest) = split_word(text);
    let (target, rest) = split_word(rest);
    let target = target.to_owned();
    let needs_target = |command: SessionsCommand| (!target.is_empty()).then_some(command);
    match sub.to_ascii_lowercase().as_str() {
        "" | "overview" => Some(SessionsCommand::Overview),
        "read" => {
            let limit: Option<usize> = match rest {
                "" => None,
                value => Some(value.parse().ok()?),
            };
            needs_target(SessionsCommand::Read { target: target.clone(), limit })
        }
        "send" => {
            let (mode, text) = take_option(rest, "--mode");
            let mode = mode.unwrap_or_else(|| "auto".to_owned()).to_ascii_lowercase();
            if !matches!(mode.as_str(), "auto" | "steer" | "follow_up") || text.is_empty() {
                return None;
            }
            needs_target(SessionsCommand::Send { target: target.clone(), text, mode })
        }
        "stop" => needs_target(SessionsCommand::Stop { target: target.clone() }),
        "answer" => {
            let (request_id, text) = take_option(rest, "--request");
            if text.is_empty() {
                return None;
            }
            needs_target(SessionsCommand::Answer { target: target.clone(), text, request_id })
        }
        "watch" => needs_target(SessionsCommand::Watch { target: target.clone() }),
        "unwatch" => needs_target(SessionsCommand::Unwatch { target: target.clone() }),
        _ => None,
    }
}

fn take_option(text: &str, flag: &str) -> (Option<String>, String) {
    let (first, rest) = split_word(text);
    if first == flag {
        let (value, rest) = split_word(rest);
        if !value.is_empty() {
            return (Some(value.to_owned()), rest.to_owned());
        }
    } else if let Some(value) = first.strip_prefix(flag).and_then(|v| v.strip_prefix('=')) {
        if !value.is_empty() {
            return (Some(value.to_owned()), rest.to_owned());
        }
    }
    (None, text.trim().to_owned())
}

fn split_word(text: &str) -> (&str, &str) {
    let text = text.trim();
    match text.split_once(char::is_whitespace) {
        Some((first, rest)) => (first, rest.trim()),
        None => (text, ""),
    }
}

pub fn help_text() -> String {
    let mut lines = vec!["*omo-gateway commands*".to_owned()];
    lines.extend(COMMANDS.iter().map(|(_, usage)| format!("• `{usage}`")));
    lines.join("\n")
}

/// DMs (`D…`) are per-user sessions; channels are shared per team, matching ingress keys.
pub fn session_key(invocation: &SlashInvocation, bot_user_id: &str) -> SessionKey {
    let key = if invocation.is_dm() {
        SessionKey::new(
            PLATFORM,
            None::<String>,
            invocation.channel_id.clone(),
            invocation.thread_ts.clone(),
            invocation.user_id.clone(),
        )
    } else {
        SessionKey::new(
            PLATFORM,
            Some(invocation.team_id.clone()),
            invocation.channel_id.clone(),
            invocation.thread_ts.clone(),
            "",
        )
    };
    key.with_bot_id(bot_user_id)
}

/// `**bold**` from shared helpers renders literally in Slack; mrkdwn bold is `*bold*`.
fn mrkdwn_bold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut parts = text.split("**");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    let rest: Vec<&str> = parts.collect();
    if rest.len() % 2 == 1 {
        out.push_str("**");
        out.push_str(&rest.join("**"));
        return out;
    }
    for (index, part) in rest.iter().enumerate() {
        if index % 2 == 0 {
            out.push('*');
        }
        out.push_str(part);
        if index % 2 == 0 {
            out.push('*');
        }
    }
    out
}

fn command_error(error: CommandError) -> OmonError {
    OmonError::Multiplexer(error.to_string())
}

fn synthetic_command_event(session: SessionKey, content: impl Into<String>) -> InboundEvent {
    InboundEvent::message(session, String::new(), content)
}

/// Number of leading rows (at least one) whose rendered transcript fits `max_chars`.
fn select_compression_window(rows: &[(i64, String, String)], max_chars: usize) -> usize {
    let mut total = 0usize;
    let mut count = 0usize;
    for (_, role, content) in rows {
        let len = role.len() + content.len() + 4;
        if count > 0 && total + len > max_chars {
            break;
        }
        total += len;
        count += 1;
    }
    count
}

impl SlackCommands {
    pub fn new(
        config: Arc<SlackConfig>,
        api: SlackApi,
        bot_user_id: impl Into<String>,
        data: SlackCommandDeps,
    ) -> Self {
        let mut poise = PoiseData::new(data.multiplexer.clone(), data.pool.clone());
        poise.started_at = data.started_at;
        poise.tools = data.tools.clone();
        poise.mcp_endpoints = data.mcp_endpoints.clone();
        poise.approvals = data.approvals.clone();
        poise.tool_registry = data.tool_registry.clone();
        poise.llm = data.llm.clone();
        Self {
            config,
            api,
            data,
            bot_user_id: bot_user_id.into(),
            poise,
        }
    }

    pub fn bot_user_id(&self) -> &str {
        &self.bot_user_id
    }

    /// Handles one `slash_commands` payload. The envelope is already acked by the socket.
    pub async fn handle(&self, payload: &Value) -> Result<()> {
        let invocation = SlashInvocation::from_payload(payload);
        if !self.config.is_allowed(&invocation.user_id) {
            return self.send(&invocation, Reply::private(UNAUTHORIZED)).await;
        }
        let command = match parse_command(&invocation.command, &invocation.text) {
            Ok(command) => command,
            Err(usage) => return self.send(&invocation, Reply::private(usage)).await,
        };
        match self.execute(&invocation, command).await {
            Ok(reply) => self.send(&invocation, reply).await,
            Err(error) => {
                let notice = Reply::private(format!("⚠️ `{}` failed: {error}", invocation.command));
                if let Err(send_error) = self.send(&invocation, notice).await {
                    tracing::warn!(%send_error, "slack command error reply failed");
                }
                Err(error)
            }
        }
    }

    async fn execute(&self, invocation: &SlashInvocation, command: SlackCommand) -> Result<Reply> {
        let key = session_key(invocation, &self.bot_user_id);
        match command {
            SlackCommand::Help => Ok(Reply::private(help_text())),
            SlackCommand::Model(name) => {
                execute_model_command(&self.poise, &key, &name)
                    .await
                    .map_err(command_error)?;
                Ok(Reply::public(format!("Model switched to `{name}`.")))
            }
            SlackCommand::Reset { confirm } => {
                match execute_reset_command(&self.poise, &key, confirm)
                    .await
                    .map_err(command_error)?
                {
                    ResetCommandResult::NeedsConfirmation => Ok(Reply::public(
                        "⚠️ This will permanently clear conversation context and memories for this session. Re-run `/omo-reset confirm` to proceed.",
                    )),
                    ResetCommandResult::Executed => {
                        Ok(Reply::public("Session context and memory cleared."))
                    }
                }
            }
            SlackCommand::Stop => {
                let interrupted = stop_session(
                    &self.data.pool,
                    &self.data.approvals,
                    &self.data.multiplexer,
                    &key,
                )
                .await
                .map_err(command_error)?;
                Ok(Reply::private(if interrupted {
                    "🛑 Stopped active turn and marked session suspended."
                } else {
                    "🛑 No active turn running; session marked suspended."
                }))
            }
            SlackCommand::Status => {
                let stats = self.poise.stats().await?;
                Ok(Reply::public(format!(
                    "Active sessions: {}\nMemory entries: {}\nUptime: {}s\nLedger entries: {}",
                    stats.active_sessions,
                    stats.memory_count,
                    stats.uptime.as_secs(),
                    stats.ledger_count
                )))
            }
            SlackCommand::Tools => {
                let tools = if self.data.tools.is_empty() {
                    "(none)".to_owned()
                } else {
                    self.data.tools.join(", ")
                };
                let endpoints = if self.data.mcp_endpoints.is_empty() {
                    "(none)".to_owned()
                } else {
                    self.data.mcp_endpoints.join("\n")
                };
                Ok(Reply::public(format!("Tools: {tools}\nMCP endpoints:\n{endpoints}")))
            }
            SlackCommand::Skill { action, arg } => self.skill(key, &action, arg).await,
            SlackCommand::Memory { action, arg } => self.memory(&key, &action, arg).await,
            SlackCommand::Cron { action } => self.cron(&action).await,
            SlackCommand::Steer(text) => {
                let prompt = format_steer_prompt(&text);
                let steered = self.data.multiplexer.steer(&key, &prompt).await?;
                if !steered {
                    self.data
                        .multiplexer
                        .route(synthetic_command_event(key, prompt))
                        .await?;
                }
                Ok(Reply::private(if steered {
                    format!("🎯 Steering guidance applied to the running turn: `{text}`")
                } else {
                    format!("🎯 Steering guidance queued: `{text}`")
                }))
            }
            SlackCommand::Undo => {
                let reply = match undo_last_exchange(&self.data.pool, &key.storage_key()).await? {
                    Some(result) => {
                        let user_preview = preview_text(&result.user_content, 120);
                        match result.assistant_content {
                            Some(assistant) => format!(
                                "↩️ *Undid last exchange* ({} messages removed):\n• *User:* `{user_preview}`\n• *Assistant:* `{}`",
                                result.deleted_count,
                                preview_text(&assistant, 120)
                            ),
                            None => format!(
                                "↩️ *Undid last message* ({} message removed):\n• *User:* `{user_preview}`",
                                result.deleted_count
                            ),
                        }
                    }
                    None => "No conversation history to undo.".to_owned(),
                };
                Ok(Reply::public(reply))
            }
            SlackCommand::Retry => {
                let retried = retry_last_user_message(
                    &self.data.pool,
                    &self.data.multiplexer,
                    key,
                    String::new(),
                )
                .await
                .map_err(command_error)?;
                Ok(Reply::private(match retried {
                    Some(content) => {
                        format!("🔄 Retrying last message: `{}`", preview_text(&content, 100))
                    }
                    None => "No previous user message found to retry.".to_owned(),
                }))
            }
            SlackCommand::Compress => self.compress(&key).await,
            SlackCommand::Title(_) => Ok(Reply::private(
                "❌ Slack threads have no title to rename; `/omo-title` is Discord-only.",
            )),
            SlackCommand::Thread { name, message } => {
                self.thread(invocation, &key, &name, message).await
            }
            SlackCommand::Deny(reason) => {
                let resolved = self
                    .data
                    .approvals
                    .resolve_session_deny(&key, reason.clone())
                    .await;
                Ok(if resolved {
                    Reply::public(match reason {
                        Some(reason) if !reason.trim().is_empty() => {
                            format!("❌ Denied pending command approval: `{}`", reason.trim())
                        }
                        _ => "❌ Denied pending command approval.".to_owned(),
                    })
                } else {
                    Reply::private("No pending approval found to deny.")
                })
            }
            SlackCommand::Yolo(mode) => {
                let outcome = yolo_toggle(
                    &self.data.pool,
                    &self.data.approvals,
                    &self.data.multiplexer,
                    &key,
                    mode.as_deref(),
                )
                .await
                .map_err(command_error)?;
                Ok(Reply::private(mrkdwn_bold(&outcome.message)))
            }
            SlackCommand::Sessions(command) => {
                if invocation.user_id != self.config.owner_id() {
                    return Ok(Reply::private(OWNER_ONLY));
                }
                Ok(self.sessions(command, !invocation.is_dm()).await)
            }
        }
    }

    async fn skill(&self, key: SessionKey, action: &str, arg: Option<String>) -> Result<Reply> {
        let query = arg.unwrap_or_default();
        if let Some(pending) = pending_action(action, query.trim()) {
            return pending_reply(&self.data.pool, PendingScope::Skills, pending).await;
        }
        let registry = &self.data.tool_registry;
        let reply = match action {
            "list" => match registry.execute("skills", json!({"action": "list"})).await {
                Ok(value) => {
                    let total = value.get("total_skills").and_then(Value::as_u64).unwrap_or(0);
                    let list = value
                        .get("skills")
                        .and_then(Value::as_array)
                        .map(|skills| {
                            skills
                                .iter()
                                .take(30)
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!(
                        "📚 *Available OMO Skills* ({total} total):\n`{list}`\n_(Use `/omo-skill read <skill_name>` to inspect)_"
                    )
                }
                Err(error) => format!("❌ Failed to list skills: {error}"),
            },
            "search" => match registry
                .execute("skills", json!({"action": "search", "query": query}))
                .await
            {
                Ok(value) => {
                    let count = value.get("count").and_then(Value::as_u64).unwrap_or(0);
                    let list = value
                        .get("matches")
                        .and_then(Value::as_array)
                        .map(|matches| {
                            matches
                                .iter()
                                .take(30)
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("\n- ")
                        })
                        .unwrap_or_default();
                    format!("🔍 *Skill Search Results for `{query}`* ({count} matches):\n- {list}")
                }
                Err(error) => format!("❌ Search failed: {error}"),
            },
            "read" => match registry
                .execute("skills", json!({"action": "read", "name": query}))
                .await
            {
                Ok(value) => {
                    let content = value.get("content").and_then(Value::as_str).unwrap_or("No content");
                    format!(
                        "📖 *Skill: `{query}`*\n```\n{}\n```",
                        skill_read_preview(content)
                    )
                }
                Err(error) => format!("❌ Could not read skill `{query}`: {error}"),
            },
            "run" => {
                let event = synthetic_command_event(key, format!("Execute skill: {query}"));
                match self.data.multiplexer.route(event).await {
                    Ok(()) => format!("🚀 Injecting skill `{query}` into current OMO session..."),
                    Err(error) => format!("❌ Could not run skill `{query}`: {error}"),
                }
            }
            _ => "Usage: `/omo-skills <list|search|read|run|pending|review|approve|reject> [name|query|id]`"
                .to_owned(),
        };
        Ok(Reply::public(reply))
    }

    async fn memory(&self, key: &SessionKey, action: &str, arg: Option<String>) -> Result<Reply> {
        let id = arg.unwrap_or_default();
        let storage_key = key.storage_key();
        if let Some(pending) = pending_action(action, id.trim()) {
            return pending_reply(&self.data.pool, PendingScope::Memory(&storage_key), pending)
                .await;
        }
        let memories: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, content FROM memories WHERE session_key = ? ORDER BY updated_at DESC LIMIT 10",
        )
        .bind(&storage_key)
        .fetch_all(&self.data.pool)
        .await?;
        if memories.is_empty() {
            return Ok(Reply::public("No memories stored for this session."));
        }
        let mut lines = vec![format!("*Stored memories ({})*:", memories.len())];
        for (id, content) in memories {
            lines.push(format!("• `{id}`: {}", preview_text(&content, 100)));
        }
        Ok(Reply::public(lines.join("\n")))
    }

    async fn cron(&self, action: &str) -> Result<Reply> {
        if action != "list" {
            return Ok(Reply::public("Usage: `/omo-cron [list]`"));
        }
        let reply = match self
            .data
            .tool_registry
            .execute("cron", json!({"action": "list"}))
            .await
        {
            Ok(value) => {
                let count = value.get("count").and_then(Value::as_u64).unwrap_or(0);
                let lines: Vec<String> = value
                    .get("cron_jobs")
                    .and_then(Value::as_array)
                    .map(|jobs| {
                        jobs.iter()
                            .take(15)
                            .map(|job| {
                                let id = job.get("id").and_then(Value::as_str).unwrap_or_default();
                                let expr = job
                                    .get("expression")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default();
                                let next = job
                                    .get("next_run_at")
                                    .and_then(Value::as_str)
                                    .unwrap_or("paused");
                                format!("• `{id}`: `{expr}` (Next: `{next}`)")
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                format!("⏰ *Active OMO Cron Jobs* ({count} total):\n{}", lines.join("\n"))
            }
            Err(error) => format!("❌ Failed to list cron jobs: {error}"),
        };
        Ok(Reply::public(reply))
    }

    async fn compress(&self, key: &SessionKey) -> Result<Reply> {
        let Some(llm) = &self.data.llm else {
            return Ok(Reply::public(
                "❌ Compression requires a configured LLM client; history left untouched.",
            ));
        };
        let storage_key = key.storage_key();
        let mut rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT sequence, role, content FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(&storage_key)
        .fetch_all(&self.data.pool)
        .await?;
        if rows.len() < 2 {
            return Ok(Reply::public("Conversation history is too short to compress."));
        }
        let window = select_compression_window(&rows, MAX_COMPRESSION_PROMPT_CHARS);
        rows.truncate(window);
        if rows.len() < 2 {
            return Ok(Reply::public("The oldest message is too large to compress."));
        }
        let cutoff_sequence = rows.last().map(|(sequence, _, _)| *sequence).unwrap_or(0);
        let history: Vec<(String, String)> =
            rows.into_iter().map(|(_, role, content)| (role, content)).collect();
        let chars_before: usize = history.iter().map(|(_, content)| content.len()).sum();
        let prompt = build_compression_prompt(&history);
        let stream = llm.stream(&[ChatMessage::new("user", prompt)], &[]).await;
        let summary = match collect_compression_summary(stream).await {
            Ok(summary) => summary,
            Err(error) => {
                tracing::warn!(%error, "LLM compression failed; history left untouched");
                return Ok(Reply::public(format!("❌ Compression failed: {error}")));
            }
        };
        let summary_content = format!("[Conversation Summary]\n{summary}");
        replace_history_with_summary(&self.data.pool, &storage_key, cutoff_sequence, &summary_content)
            .await?;
        let (before, after, pct) = calculate_compression_stats(chars_before, summary_content.len());
        Ok(Reply::public(format!(
            "🗜️ *Conversation Compressed*\n• *Before:* {before} chars\n• *After:* {after} chars\n• *Reduction:* {pct:.1}%"
        )))
    }

    /// Slack has no thread objects: the thread is a root message, and its `ts` keys the session.
    async fn thread(
        &self,
        invocation: &SlashInvocation,
        key: &SessionKey,
        name: &str,
        message: Option<String>,
    ) -> Result<Reply> {
        if invocation.thread_ts.is_some() {
            return Ok(Reply::private("❌ Cannot create a thread inside an existing thread."));
        }
        let root_ts = self
            .api
            .post_message(&invocation.channel_id, &format!("🧵 *{name}*"), None, None)
            .await?;
        if let Some(starter) = message {
            let thread_key = SessionKey {
                thread_id: Some(root_ts.clone()),
                ..key.clone()
            };
            if let Err(error) = self
                .data
                .multiplexer
                .route(synthetic_command_event(thread_key, starter))
                .await
            {
                return Ok(Reply::public(format!(
                    "❌ Created thread *{name}* but could not start a session: {error}"
                )));
            }
        }
        Ok(Reply::public(format!("🧵 Created thread *{name}*.")))
    }

    async fn sessions(&self, command: SessionsCommand, ephemeral: bool) -> Reply {
        match self.sessions_inner(command, ephemeral).await {
            Ok(reply) => reply,
            Err(error) => Reply::visible(ephemeral, format!("⚠️ {error}")),
        }
    }

    async fn sessions_inner(
        &self,
        command: SessionsCommand,
        ephemeral: bool,
    ) -> std::result::Result<Reply, String> {
        let control = SessionControl::from_environment().map_err(|e| e.to_string())?;
        let resolve = |target: String| {
            let control = &control;
            async move {
                let parsed: Target = target.parse().map_err(|e: ControlError| e.to_string())?;
                control.resolve(&parsed).await.map_err(|e| e.to_string())
            }
        };
        let text = match command {
            SessionsCommand::Overview => {
                let data = overview::collect(&control, 100).await;
                let formatted = format_overview(&data, Utc::now(), MESSAGE_LIMIT);
                if formatted.trim().is_empty() {
                    "(내용 없음)".to_owned()
                } else {
                    formatted
                }
            }
            SessionsCommand::Read { target, limit } => {
                let (resolved, session) = resolve(target).await?;
                let transcript = control
                    .read(&resolved, limit.unwrap_or(20), None)
                    .await
                    .map_err(|e| e.to_string())?;
                let tail: Vec<String> = transcript
                    .get("items")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| {
                                let role = item.get("role").and_then(Value::as_str).unwrap_or("unknown");
                                let content = item.get("content").and_then(Value::as_str).unwrap_or("");
                                format!("{role}: {content}")
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let probe = crate::sessions::probe::Probe::new(control.runner());
                let snapshot = probe.snapshot(resolved.machine(), 100).await.ok();
                let live = snapshot
                    .as_ref()
                    .and_then(|snap| {
                        snap.sessions.iter().find(|s| {
                            s.session_id == session.session_id || s.routing_id == session.session_id
                        })
                    })
                    .cloned()
                    .unwrap_or_else(|| crate::sessions::probe::LiveSession {
                        machine: resolved.machine().to_string(),
                        session_id: session.session_id.clone(),
                        routing_id: session.session_id.clone(),
                        endpoint_kind: "unknown".to_string(),
                        name: session.name.clone(),
                        cwd: None,
                        streaming: false,
                        compacting: false,
                        blocking_question: false,
                        pending_messages: 0,
                        message_count: 0,
                        last_user: String::new(),
                        last_assistant: String::new(),
                        questions: Vec::new(),
                        messages_error: None,
                        updated_at: None,
                    });
                format_detail(&resolved.to_string(), &live, &tail, MESSAGE_LIMIT)
            }
            SessionsCommand::Send { target, text, mode } => {
                let (resolved, _) = resolve(target).await?;
                let mode = match mode.as_str() {
                    "steer" => SendMode::Steer,
                    "follow_up" => SendMode::FollowUp,
                    _ => SendMode::Auto,
                };
                control
                    .send(&resolved, &text, mode, uuid::Uuid::new_v4())
                    .await
                    .map_err(|e| e.to_string())?;
                let target = resolved.to_string();
                let watched = match watch::watch(&self.data.pool, &target).await {
                    Ok(()) => "감시 목록에 등록했어요".to_owned(),
                    Err(error) => {
                        tracing::warn!(%error, %target, "auto-watch after send failed");
                        format!("감시 등록은 실패했어요: {error}")
                    }
                };
                format!("✅ `{target}`로 지시를 전송했어요. {watched}.")
            }
            SessionsCommand::Stop { target } => {
                let (resolved, _) = resolve(target).await?;
                control.interrupt(&resolved).await.map_err(|e| e.to_string())?;
                format!("🛑 `{resolved}` 실행을 중단(인터럽트)했어요.")
            }
            SessionsCommand::Answer { target, text, request_id } => {
                let (resolved, _) = resolve(target).await?;
                control
                    .answer(&resolved, &text, request_id.as_deref())
                    .await
                    .map_err(|e| e.to_string())?;
                format!("💬 `{resolved}` 질문에 답변을 전송했어요.")
            }
            SessionsCommand::Watch { target } => {
                let (resolved, _) = resolve(target).await?;
                let target = resolved.to_string();
                watch::watch(&self.data.pool, &target)
                    .await
                    .map_err(|e| e.to_string())?;
                format!("👀 `{target}` 세션을 감시 목록에 등록했어요.")
            }
            SessionsCommand::Unwatch { target } => {
                let parsed: Target = target.parse().map_err(|e: ControlError| e.to_string())?;
                let target = match control.resolve(&parsed).await {
                    Ok((resolved, _)) => resolved.to_string(),
                    Err(_) => parsed.to_string(),
                };
                match watch::unwatch(&self.data.pool, &target).await {
                    Ok(true) => format!("제외: `{target}` 세션 감시를 해제했어요."),
                    Ok(false) => format!("`{target}` 세션은 감시 목록에 없었어요."),
                    Err(error) => return Err(error.to_string()),
                }
            }
        };
        Ok(Reply::visible(ephemeral, text))
    }

    /// Up to five `response_url` posts; the rest go through the Web API in the same channel.
    async fn send(&self, invocation: &SlashInvocation, reply: Reply) -> Result<()> {
        let mut chunks = crate::discord::chunk_slash_reply(&reply.text, MESSAGE_LIMIT);
        if chunks.is_empty() {
            chunks.push("(내용 없음)".to_owned());
        }
        let response_type = if reply.ephemeral { "ephemeral" } else { "in_channel" };
        let via_url = if invocation.response_url.is_some() {
            MAX_RESPONSE_URL_POSTS
        } else {
            0
        };
        for (index, chunk) in chunks.iter().enumerate() {
            match invocation.response_url.as_deref() {
                Some(url) if index < via_url => {
                    self.api
                        .post_response_url(url, json!({ "response_type": response_type, "text": chunk }))
                        .await?;
                }
                _ if reply.ephemeral => {
                    self.api
                        .post_ephemeral(&invocation.channel_id, &invocation.user_id, chunk)
                        .await?;
                }
                _ => {
                    self.api
                        .post_message(&invocation.channel_id, chunk, invocation.thread_ts.as_deref(), None)
                        .await?;
                }
            }
        }
        Ok(())
    }
}

enum PendingAction<'a> {
    List,
    Review(&'a str),
    Approve(&'a str),
    Reject(&'a str),
}

fn pending_action<'a>(action: &str, id: &'a str) -> Option<PendingAction<'a>> {
    Some(match action {
        "pending" if id.is_empty() => PendingAction::List,
        "pending" | "review" => PendingAction::Review(id),
        "approve" | "apply" => PendingAction::Approve(id),
        "reject" | "deny" | "drop" => PendingAction::Reject(id),
        _ => return None,
    })
}

/// Re-implements the private Discord `pending_command`; always ephemeral, payload inline.
async fn pending_reply(
    pool: &SqlitePool,
    scope: PendingScope<'_>,
    action: PendingAction<'_>,
) -> Result<Reply> {
    let records = match action {
        PendingAction::List => crate::storage::list_pending_writes_scoped(pool, scope).await?,
        PendingAction::Review(id) => {
            vec![crate::storage::get_pending_write_scoped(pool, id, scope).await?]
        }
        PendingAction::Approve(id) => {
            let result = crate::storage::approve_pending_write_scoped(pool, id, scope).await?;
            return Ok(Reply::private(if result.is_some() {
                "Pending write approved."
            } else {
                "Pending write was already consumed."
            }));
        }
        PendingAction::Reject(id) => {
            let rejected = crate::storage::reject_pending_write_scoped(pool, id, scope).await?;
            return Ok(Reply::private(if rejected {
                "Pending write rejected."
            } else {
                "Pending write was already consumed."
            }));
        }
    };
    if records.is_empty() {
        return Ok(Reply::private("No pending writes in this scope."));
    }
    let content = serde_json::to_string_pretty(&records)
        .map_err(|error| OmonError::Database(error.to_string()))?;
    Ok(Reply::private(format!(
        "{} pending write(s). Review the complete payloads before approving an ID.\n```\n{content}\n```",
        records.len()
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use async_trait::async_trait;
    use axum::extract::Request;
    use parking_lot::Mutex;

    use super::*;
    use crate::{AgentRunner, Database, MultiplexerConfig, SessionContext};

    fn invocation(command: &str, text: &str, user: &str, channel: &str) -> Value {
        json!({
            "command": command,
            "text": text,
            "user_id": user,
            "channel_id": channel,
            "team_id": "T1",
        })
    }

    fn parse(command: &str, text: &str) -> SlackCommand {
        parse_command(command, text).unwrap()
    }

    #[test]
    fn parses_every_simple_command() {
        assert_eq!(parse("/omo-model", " opus "), SlackCommand::Model("opus".into()));
        assert!(parse_command("/omo-model", "").unwrap_err().contains("/omo-model <name>"));
        assert_eq!(parse("/omo-reset", ""), SlackCommand::Reset { confirm: None });
        assert_eq!(parse("/omo-reset", "confirm"), SlackCommand::Reset { confirm: Some(true) });
        assert_eq!(parse("/omo-reset", "no"), SlackCommand::Reset { confirm: Some(false) });
        assert!(parse_command("/omo-reset", "maybe").is_err());
        assert_eq!(parse("/omo-stop", ""), SlackCommand::Stop);
        assert_eq!(parse("/omo-status", ""), SlackCommand::Status);
        assert_eq!(parse("/omo-tools", ""), SlackCommand::Tools);
        assert_eq!(parse("/omo-steer", "focus on tests"), SlackCommand::Steer("focus on tests".into()));
        assert!(parse_command("/omo-steer", " ").is_err());
        assert_eq!(parse("/omo-undo", ""), SlackCommand::Undo);
        assert_eq!(parse("/omo-retry", ""), SlackCommand::Retry);
        assert_eq!(parse("/omo-compress", ""), SlackCommand::Compress);
        assert_eq!(parse("/omo-title", "New name"), SlackCommand::Title("New name".into()));
        assert_eq!(parse("/omo-deny", ""), SlackCommand::Deny(None));
        assert_eq!(parse("/omo-deny", "too risky"), SlackCommand::Deny(Some("too risky".into())));
        assert_eq!(parse("/omo-yolo", ""), SlackCommand::Yolo(None));
        assert_eq!(parse("/omo-yolo", "on"), SlackCommand::Yolo(Some("on".into())));
        assert_eq!(parse("/omo-nope", ""), SlackCommand::Help);
    }

    #[test]
    fn parses_skill_memory_cron_and_thread_arguments() {
        assert_eq!(
            parse("/omo-skill", "Read git-master"),
            SlackCommand::Skill { action: "read".into(), arg: Some("git-master".into()) }
        );
        assert!(parse_command("/omo-skill", "").is_err());
        assert_eq!(
            parse("/omo-skills", ""),
            SlackCommand::Skill { action: "list".into(), arg: None }
        );
        assert_eq!(
            parse("/omo-skills", "search code review"),
            SlackCommand::Skill { action: "search".into(), arg: Some("code review".into()) }
        );
        assert_eq!(
            parse("/omo-memory", ""),
            SlackCommand::Memory { action: "list".into(), arg: None }
        );
        assert_eq!(
            parse("/omo-memory", "approve abc-1"),
            SlackCommand::Memory { action: "approve".into(), arg: Some("abc-1".into()) }
        );
        assert_eq!(parse("/omo-cron", ""), SlackCommand::Cron { action: "list".into() });
        assert_eq!(
            parse("/omo-thread", "Plan | start planning"),
            SlackCommand::Thread { name: "Plan".into(), message: Some("start planning".into()) }
        );
        assert_eq!(
            parse("/omo-thread", "Just a name"),
            SlackCommand::Thread { name: "Just a name".into(), message: None }
        );
        assert!(parse_command("/omo-thread", "| only message").is_err());
    }

    #[test]
    fn parses_sessions_subcommands() {
        let sessions = |text: &str| match parse("/omo-sessions", text) {
            SlackCommand::Sessions(command) => command,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(sessions(""), SessionsCommand::Overview);
        assert_eq!(sessions("overview"), SessionsCommand::Overview);
        assert_eq!(
            sessions("read local:01a1 20"),
            SessionsCommand::Read { target: "local:01a1".into(), limit: Some(20) }
        );
        assert_eq!(
            sessions("read local:01a1"),
            SessionsCommand::Read { target: "local:01a1".into(), limit: None }
        );
        assert!(parse_command("/omo-sessions", "read local:01a1 many").is_err());
        assert_eq!(
            sessions("send local:01a1 run the tests"),
            SessionsCommand::Send {
                target: "local:01a1".into(),
                text: "run the tests".into(),
                mode: "auto".into()
            }
        );
        assert_eq!(
            sessions("send local:01a1 --mode steer stop now"),
            SessionsCommand::Send {
                target: "local:01a1".into(),
                text: "stop now".into(),
                mode: "steer".into()
            }
        );
        assert!(parse_command("/omo-sessions", "send local:01a1 --mode loud hi").is_err());
        assert_eq!(sessions("stop mac:ab"), SessionsCommand::Stop { target: "mac:ab".into() });
        assert_eq!(
            sessions("answer local:01a1 yes please"),
            SessionsCommand::Answer {
                target: "local:01a1".into(),
                text: "yes please".into(),
                request_id: None
            }
        );
        assert_eq!(
            sessions("answer local:01a1 --request r-9 Option A"),
            SessionsCommand::Answer {
                target: "local:01a1".into(),
                text: "Option A".into(),
                request_id: Some("r-9".into())
            }
        );
        assert!(parse_command("/omo-sessions", "answer local:01a1").is_err());
        assert_eq!(sessions("watch local:x"), SessionsCommand::Watch { target: "local:x".into() });
        assert_eq!(sessions("unwatch local:x"), SessionsCommand::Unwatch { target: "local:x".into() });
        assert!(parse_command("/omo-sessions", "watch").is_err());
        assert!(parse_command("/omo-sessions", "explode local:x").is_err());
    }

    #[test]
    fn session_key_maps_dm_channel_and_thread() {
        let dm = SlashInvocation::from_payload(&invocation("/omo-status", "", "U1", "D123"));
        let key = session_key(&dm, "UBOT");
        assert_eq!(key.platform, "slack");
        assert_eq!(key.guild_id, None);
        assert_eq!(key.channel_id, "D123");
        assert_eq!(key.thread_id, None);
        assert_eq!(key.user_id, "U1");
        assert_eq!(key.bot_id.as_deref(), Some("UBOT"));

        let channel = SlashInvocation::from_payload(&invocation("/omo-status", "", "U1", "C9"));
        let key = session_key(&channel, "UBOT");
        assert_eq!(key.guild_id.as_deref(), Some("T1"));
        assert_eq!(key.channel_id, "C9");
        assert_eq!(key.user_id, "");
        assert_eq!(key.bot_id.as_deref(), Some("UBOT"));

        let mut payload = invocation("/omo-status", "", "U1", "C9");
        payload["thread_ts"] = json!("1700.01");
        let threaded = SlashInvocation::from_payload(&payload);
        assert_eq!(session_key(&threaded, "UBOT").thread_id.as_deref(), Some("1700.01"));
    }

    #[test]
    fn help_lists_every_manifest_command() {
        let manifest: Value =
            serde_json::from_str(include_str!("../../docs/slack-app-manifest.json")).unwrap();
        let commands = manifest["features"]["slash_commands"].as_array().unwrap();
        assert_eq!(commands.len(), 18);
        let help = help_text();
        for command in commands {
            let name = command["command"].as_str().unwrap();
            assert!(help.contains(&format!("`{name}")), "help misses {name}");
            assert_ne!(parse_command(name, ""), Ok(SlackCommand::Help), "{name} not dispatched");
        }
        assert_eq!(COMMANDS.len(), commands.len());
    }

    #[test]
    fn mrkdwn_bold_converts_pairs_only() {
        assert_eq!(mrkdwn_bold("⚡ YOLO mode **enabled** now"), "⚡ YOLO mode *enabled* now");
        assert_eq!(mrkdwn_bold("a ** b"), "a ** b");
        assert_eq!(mrkdwn_bold("plain"), "plain");
    }

    struct NoopRunner;

    #[async_trait]
    impl AgentRunner for NoopRunner {
        async fn run(&self, _session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
            Ok(())
        }
    }

    struct Fake {
        base: String,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
    }

    async fn fake_slack() -> Fake {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let app = axum::Router::new().fallback(move |req: Request| {
            let recorded = recorded.clone();
            async move {
                let path = req.uri().path().to_owned();
                let body = axum::body::to_bytes(req.into_body(), usize::MAX).await.unwrap();
                let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                recorded.lock().push((path, body));
                axum::Json(json!({ "ok": true, "ts": "1700.0001" }))
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake { base, requests }
    }

    async fn commands(fake: &Fake) -> SlackCommands {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let multiplexer =
            SessionMultiplexer::new(pool.clone(), Arc::new(NoopRunner), MultiplexerConfig::default());
        let config = SlackConfig {
            bot_token: "xoxb-test".into(),
            app_token: "xapp-test".into(),
            allowed_users: vec!["UOWNER".into(), "UFRIEND".into()],
            free_response_channels: Vec::new(),
            auto_thread: true,
            attachment_dir: PathBuf::from("/tmp"),
        };
        SlackCommands::new(
            Arc::new(config),
            SlackApi::with_base_url("xoxb-test".into(), format!("{}/api", fake.base)),
            "UBOT",
            SlackCommandDeps::new(multiplexer, pool),
        )
    }

    fn with_response_url(mut payload: Value, fake: &Fake) -> Value {
        payload["response_url"] = json!(format!("{}/hook", fake.base));
        payload
    }

    #[tokio::test]
    async fn unauthorized_user_gets_ephemeral_rejection() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;
        let payload = with_response_url(invocation("/omo-yolo", "on", "USTRANGER", "C1"), &fake);

        commands.handle(&payload).await.unwrap();

        let requests = fake.requests.lock().clone();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, "/hook");
        assert_eq!(requests[0].1["response_type"], "ephemeral");
        assert_eq!(requests[0].1["text"], UNAUTHORIZED);
        let yolo: Option<String> = sqlx::query_scalar("SELECT state_json FROM sessions LIMIT 1")
            .fetch_optional(&commands.data.pool)
            .await
            .unwrap();
        assert_eq!(yolo, None);
    }

    #[tokio::test]
    async fn sessions_is_owner_only_and_unknown_command_shows_help() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;

        commands
            .handle(&with_response_url(invocation("/omo-sessions", "overview", "UFRIEND", "C1"), &fake))
            .await
            .unwrap();
        commands
            .handle(&with_response_url(invocation("/omo-bogus", "", "UOWNER", "C1"), &fake))
            .await
            .unwrap();

        let requests = fake.requests.lock().clone();
        assert_eq!(requests[0].1["text"], OWNER_ONLY);
        assert_eq!(requests[0].1["response_type"], "ephemeral");
        assert_eq!(requests[1].1["response_type"], "ephemeral");
        assert_eq!(requests[1].1["text"], help_text());
    }

    #[tokio::test]
    async fn status_replies_in_channel_through_response_url() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;

        commands
            .handle(&with_response_url(invocation("/omo-status", "", "UOWNER", "C1"), &fake))
            .await
            .unwrap();

        let requests = fake.requests.lock().clone();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1["response_type"], "in_channel");
        let text = requests[0].1["text"].as_str().unwrap();
        assert!(text.starts_with("Active sessions: 0\nMemory entries: 0"), "{text}");
    }

    #[tokio::test]
    async fn yolo_persists_state_and_replies_ephemeral() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;

        commands
            .handle(&with_response_url(invocation("/omo-yolo", "on", "UOWNER", "D7"), &fake))
            .await
            .unwrap();

        let key = SessionKey::new("slack", None::<String>, "D7", None::<String>, "UOWNER")
            .with_bot_id("UBOT");
        assert!(commands.data.approvals.is_yolo(&key).await);
        let state: String = sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(key.storage_key())
            .fetch_one(&commands.data.pool)
            .await
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&state).unwrap()["yolo"], true);
        let requests = fake.requests.lock().clone();
        assert_eq!(requests[0].1["response_type"], "ephemeral");
        assert!(requests[0].1["text"].as_str().unwrap().contains("*enabled*"));
    }

    #[tokio::test]
    async fn long_replies_use_five_response_url_posts_then_post_message() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;
        let mut payload = with_response_url(invocation("/omo-status", "", "UOWNER", "C1"), &fake);
        payload["thread_ts"] = json!("1699.5");
        let invocation = SlashInvocation::from_payload(&payload);
        let line = "x".repeat(100);
        let text = vec![line; 300].join("\n");

        commands.send(&invocation, Reply::public(text)).await.unwrap();

        let requests = fake.requests.lock().clone();
        let hooks = requests.iter().filter(|(path, _)| path == "/hook").count();
        let posts: Vec<&Value> = requests
            .iter()
            .filter(|(path, _)| path == "/api/chat.postMessage")
            .map(|(_, body)| body)
            .collect();
        assert_eq!(hooks, MAX_RESPONSE_URL_POSTS);
        assert!(!posts.is_empty());
        assert!(posts.iter().all(|body| body["channel"] == "C1" && body["thread_ts"] == "1699.5"));
        assert!(requests
            .iter()
            .all(|(_, body)| body["text"].as_str().unwrap().chars().count() <= MESSAGE_LIMIT));
    }

    #[tokio::test]
    async fn ephemeral_without_response_url_uses_post_ephemeral() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;

        commands
            .handle(&invocation("/omo-model", "", "UOWNER", "C1"))
            .await
            .unwrap();

        let requests = fake.requests.lock().clone();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, "/api/chat.postEphemeral");
        assert_eq!(requests[0].1["user"], "UOWNER");
        assert!(requests[0].1["text"].as_str().unwrap().contains("/omo-model <name>"));
    }

    #[tokio::test]
    async fn thread_posts_root_message_and_rejects_nested() {
        let fake = fake_slack().await;
        let commands = commands(&fake).await;

        commands
            .handle(&with_response_url(invocation("/omo-thread", "Plan", "UOWNER", "C1"), &fake))
            .await
            .unwrap();
        let mut nested = with_response_url(invocation("/omo-thread", "Plan", "UOWNER", "C1"), &fake);
        nested["thread_ts"] = json!("1.1");
        commands.handle(&nested).await.unwrap();

        let requests = fake.requests.lock().clone();
        assert_eq!(requests[0].0, "/api/chat.postMessage");
        assert_eq!(requests[0].1["channel"], "C1");
        assert_eq!(requests[0].1["text"], "🧵 *Plan*");
        assert_eq!(requests[1].1["text"], "🧵 Created thread *Plan*.");
        assert_eq!(requests[2].1["response_type"], "ephemeral");
        assert_eq!(requests.len(), 3);
    }
}

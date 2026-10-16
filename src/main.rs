// allow: SIZE_OK — main gateway application orchestration and integration tests
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use clap::{Parser, Subcommand};
use omon_gateway::migrate::MigrateArgs;
use omon_gateway::storage::init_pool;
use omon_gateway::{
    authorized_cron_roots, canonical_authorized_directory,
    cron_runs_retention_days_from_environment, cron_script_timeout_secs_from,
    prune_terminal_cron_runs, validate_agent_backend_env, AgentBackend, AgentCronExecutor,
    ApprovalPolicy, AttachmentDownloader, CronScheduler, CronTool, DeadTargetRegistry,
    DiscordAdapter, DiscordApprovalRequester, DiscordEgress, FileTool, HermesJob,
    HermesStoreSynchronizer, InboundEvent, LlmClient, LlmConfig, LlmProvider, McpTool,
    MultiplexerConfig, OmoBackend, OmoBackendConfig, OmoDaemonSupervisor, OmonError,
    OpenAiSpeechToText, OutboundAction, OutboundDispatcher, PoiseData, ProfileRoute, ProfileRouter,
    RestartLoopGuard, Result, ScaleToZero, SessionContext, SessionKey, SessionMultiplexer,
    SmartApprovalGuard, TerminalTool, ToolRegistry,
};
#[allow(unused_imports)]
pub use omon_gateway::{load_cron_skills, resolve_skill_bundle, resolve_workspace_instructions};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "omo-gateway")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run,
    Migrate(MigrateArgs),
}

impl Cli {
    #[allow(dead_code)]
    fn into_command(self) -> Command {
        self.command.unwrap_or(Command::Run)
    }
}

pub(crate) struct Config {
    discord_bot_tokens: Vec<String>,
    database_url: String,
    default_model: String,
    openai_api_base: Option<String>,
    openai_api_key: Option<String>,
    anthropic_base_url: Option<String>,
    anthropic_api_key: Option<String>,
    workspace_root: PathBuf,
    extra_tool_roots: Vec<PathBuf>,
    free_response_channels: Vec<u64>,
    allowed_users: Vec<u64>,
    allowed_roles: Vec<u64>,
    allow_all_users: bool,
    thread_sessions_per_user: bool,
    thread_require_mention: bool,
    allowed_channels: Vec<u64>,
    ignored_channels: Vec<u64>,
    auto_thread: bool,
    channel_context: bool,
    channel_context_limit: usize,
    processing_reactions: bool,
    approval_policy: ApprovalPolicy,
    approval_timeout_secs: u64,
    cron_script_timeout_secs: u64,
    approval_mentions: bool,
    approvals_deny: Vec<String>,
    profile_routes: Vec<ProfileRoute>,
    runtime_footer: bool,
    allow_bots: omon_gateway::AllowBotsMode,
    channel_topic_context: bool,
    discord_missed_backfill: bool,
    destructive_slash_confirm: bool,
}

impl Config {
    fn from_env() -> Result<Self> {
        let mut free_response_channels: Vec<u64> = env::var("DISCORD_FREE_RESPONSE_CHANNELS")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|p| p.trim().parse::<u64>().ok())
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(home) = env::var("DISCORD_HOME_CHANNEL") {
            for h in home.split(',') {
                if let Ok(id) = h.trim().parse::<u64>() {
                    if !free_response_channels.contains(&id) {
                        free_response_channels.push(id);
                    }
                }
            }
        }
        // A set-but-unparsable allowlist must refuse to start: an empty list
        // silently enables open pairing prompts for every DM sender.
        let allowed_users = parse_allowlist(
            "DISCORD_ALLOWED_USERS",
            optional_env("DISCORD_ALLOWED_USERS").as_deref(),
        )?;
        let allowed_roles = parse_allowlist(
            "DISCORD_ALLOWED_ROLES",
            optional_env("DISCORD_ALLOWED_ROLES").as_deref(),
        )?;
        let allow_all_users =
            parse_bool_from(optional_env("DISCORD_ALLOW_ALL_USERS").as_deref(), false);
        let thread_sessions_per_user = parse_bool_from(
            optional_env("DISCORD_THREAD_SESSIONS_PER_USER").as_deref(),
            true,
        );
        let thread_require_mention = parse_bool_from(
            optional_env("DISCORD_THREAD_REQUIRE_MENTION").as_deref(),
            false,
        );
        let allowed_channels = parse_allowlist(
            "DISCORD_ALLOWED_CHANNELS",
            optional_env("DISCORD_ALLOWED_CHANNELS").as_deref(),
        )?;
        let ignored_channels = parse_u64_list(optional_env("DISCORD_IGNORED_CHANNELS").as_deref());

        let mut tokens = Vec::new();
        if let Ok(tok) = env::var("DISCORD_BOT_TOKEN") {
            for t in tok.split(',') {
                let trimmed = t.trim().trim_matches('"').trim_matches('\'');
                if !trimmed.is_empty() {
                    tokens.push(trimmed.to_string());
                }
            }
        }
        if let Ok(toks) = env::var("DISCORD_BOT_TOKENS") {
            for t in toks.split(',') {
                let trimmed = t.trim().trim_matches('"').trim_matches('\'');
                if !trimmed.is_empty() && !tokens.contains(&trimmed.to_string()) {
                    tokens.push(trimmed.to_string());
                }
            }
        }
        if tokens.is_empty() {
            return Err(OmonError::Config(
                "missing required environment variable DISCORD_BOT_TOKEN".into(),
            ));
        }

        let workspace_root = env::var_os("OMON_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                home.join(".omon").join("workspace")
            });
        let _ = std::fs::create_dir_all(&workspace_root);

        let extra_tool_roots = optional_env("OMON_TOOL_ROOTS")
            .map(|val| {
                val.split(':')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            })
            .filter(|roots| !roots.is_empty())
            .unwrap_or_else(|| {
                let home = env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));
                vec![home]
            });

        let mut profile_routes = omon_gateway::try_parse_profile_routes(
            &optional_env("DISCORD_PROFILE_ROUTES").unwrap_or_default(),
        )
        .map_err(|error| OmonError::Config(format!("invalid DISCORD_PROFILE_ROUTES: {error}")))?;
        let channel_prompt_routes = omon_gateway::parse_channel_prompts(
            &optional_env("DISCORD_CHANNEL_PROMPTS").unwrap_or_default(),
        );
        profile_routes.extend(channel_prompt_routes);

        Ok(Self {
            discord_bot_tokens: tokens,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://omon_gateway.db".to_owned()),
            default_model: Self::resolve_default_model(),
            openai_api_base: optional_env("OPENAI_API_BASE"),
            openai_api_key: optional_env("OPENAI_API_KEY"),
            anthropic_base_url: optional_env("ANTHROPIC_BASE_URL"),
            anthropic_api_key: optional_env("ANTHROPIC_API_KEY"),
            workspace_root,
            extra_tool_roots,
            free_response_channels,
            allowed_users,
            allowed_roles,
            allow_all_users,
            thread_sessions_per_user,
            thread_require_mention,
            allowed_channels,
            ignored_channels,
            auto_thread: parse_bool_from(optional_env("DISCORD_AUTO_THREAD").as_deref(), false),
            channel_context: parse_bool_from(
                optional_env("DISCORD_CHANNEL_CONTEXT").as_deref(),
                false,
            ),
            channel_topic_context: parse_bool_from(
                optional_env("DISCORD_CHANNEL_TOPIC_CONTEXT").as_deref(),
                false,
            ),
            channel_context_limit: optional_env("DISCORD_CHANNEL_CONTEXT_LIMIT")
                .and_then(|val| val.trim().parse::<usize>().ok())
                .unwrap_or(omon_gateway::DEFAULT_CHANNEL_CONTEXT_LIMIT)
                .min(omon_gateway::MAX_CHANNEL_CONTEXT_LIMIT),
            processing_reactions: parse_bool_from(
                optional_env("DISCORD_PROCESSING_REACTIONS").as_deref(),
                true,
            ),
            approval_policy: ApprovalPolicy::parse(optional_env("APPROVAL_MODE").as_deref()),
            approval_timeout_secs: approval_timeout_secs_from(
                optional_env("APPROVAL_TIMEOUT_SECS").as_deref(),
            ),
            cron_script_timeout_secs: cron_script_timeout_secs_from(
                optional_env("OMON_CRON_SCRIPT_TIMEOUT_SECS").as_deref(),
            ),
            approval_mentions: parse_bool_from(
                optional_env("DISCORD_APPROVAL_MENTIONS").as_deref(),
                false,
            ),
            approvals_deny: env::var("APPROVALS_DENY")
                .or_else(|_| env::var("OMON_APPROVALS_DENY"))
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            profile_routes,
            runtime_footer: parse_bool_from(
                optional_env("DISCORD_RUNTIME_FOOTER").as_deref(),
                false,
            ),
            allow_bots: omon_gateway::AllowBotsMode::parse(
                optional_env("DISCORD_ALLOW_BOTS").as_deref(),
            ),
            discord_missed_backfill: parse_bool_from(
                optional_env("DISCORD_MISSED_BACKFILL").as_deref(),
                false,
            ),
            destructive_slash_confirm: parse_bool_from(
                optional_env("APPROVALS_DESTRUCTIVE_SLASH_CONFIRM").as_deref(),
                true,
            ),
        })
    }

    pub(crate) fn resolve_default_model() -> String {
        optional_env("OMON_DEFAULT_MODEL")
            .or_else(|| optional_env("DEFAULT_MODEL"))
            .unwrap_or_else(|| "mahoquot/nekos-claude-sonnet-5-5".to_string())
    }

    fn llm_config(&self, model: impl Into<String>) -> LlmConfig {
        let model = model.into();
        let anthropic = model.starts_with("claude");
        let mut config = LlmConfig::new(
            if anthropic {
                LlmProvider::Anthropic
            } else {
                LlmProvider::OpenAi
            },
            model,
        );
        if anthropic {
            config.base_url = self.anthropic_base_url.clone();
            config.api_key = self.anthropic_api_key.clone();
        } else {
            config.base_url = self.openai_api_base.clone();
            config.api_key = self.openai_api_key.clone();
        }
        config
    }
}

#[derive(Default)]
struct SharedDispatcher {
    inner: RwLock<Option<Arc<dyn OutboundDispatcher>>>,
}

impl SharedDispatcher {
    async fn set(&self, dispatcher: Arc<dyn OutboundDispatcher>) {
        *self.inner.write().await = Some(dispatcher);
    }
}

#[async_trait]
impl OutboundDispatcher for SharedDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        let dispatcher =
            self.inner.read().await.clone().ok_or_else(|| {
                OmonError::Config("outbound dispatcher is not initialized".into())
            })?;
        dispatcher.dispatch(action).await
    }
}

pub use omon_gateway::ledger::recover_pending_delivery_obligations;

/// Combined `run` mode: the dashboard is served from the gateway runtime so it
/// sees and resolves the gateway's own approvals and sessions.
#[allow(clippy::too_many_arguments)]
async fn attach_gateway_dashboard(
    settings: &dashboard::DashboardSettings,
    approval_requester: &DiscordApprovalRequester,
    shared_dispatcher: &SharedDispatcher,
    primary: Arc<dyn OutboundDispatcher>,
    pool: SqlitePool,
    multiplexer: SessionMultiplexer,
    scheduler: CronScheduler,
    tools: ToolRegistry,
    config_view: serde_json::Value,
    workspace_root: PathBuf,
    skill_roots: Vec<PathBuf>,
    bot_connections: usize,
) -> dashboard::DashboardState {
    let events = dashboard::WebDashboardDispatcher::new();
    let composite: Arc<dyn OutboundDispatcher> =
        Arc::new(dashboard::CompositeDispatcher::new(primary, events.clone()));
    // Approvals and session output reach both Discord and the dashboard.
    approval_requester.set_dispatcher(composite.clone()).await;
    shared_dispatcher.set(composite).await;
    dashboard::DashboardState::new(
        pool,
        Some(multiplexer),
        scheduler,
        tools,
        approval_requester.guard().clone(),
        events,
        config_view,
        workspace_root,
        skill_roots,
        bot_connections,
        settings.web_root.clone(),
    )
}

#[derive(Clone, Debug, Default)]
pub struct StartupAuthorization {
    pub allowed_users: Vec<u64>,
    pub allow_all_users: bool,
    pub allowed_channels: Vec<u64>,
    pub ignored_channels: Vec<u64>,
    pub paired_users: Vec<u64>,
}

impl StartupAuthorization {
    pub fn is_authorized(&self, session_key: &SessionKey) -> bool {
        self.is_authorized_actor(session_key, Some(&session_key.user_id))
    }

    fn is_authorized_actor(&self, session_key: &SessionKey, actor: Option<&str>) -> bool {
        if let Ok(channel_id) = session_key.channel_id.parse::<u64>() {
            if !self.ignored_channels.is_empty() && self.ignored_channels.contains(&channel_id) {
                return false;
            }
            // DMs are exempt from the channel allow-list, matching live ingress.
            if session_key.guild_id.is_some()
                && !self.allowed_channels.is_empty()
                && !self.allowed_channels.contains(&channel_id)
            {
                return false;
            }
        }
        self.allow_all_users
            || actor
                .and_then(|id| id.parse::<u64>().ok())
                .is_some_and(|id| {
                    self.allowed_users.contains(&id) || self.paired_users.contains(&id)
                })
    }
}

/// Startup recovery: finds sessions marked resume_pending from a previous run/crash/restart,
/// reconstructs their last unfinished user turn, and re-dispatches them through the multiplexer.
pub async fn recover_resume_pending_sessions(
    pool: &SqlitePool,
    multiplexer: &SessionMultiplexer,
) -> Result<usize> {
    recover_resume_pending_sessions_with_auth(pool, multiplexer, None).await
}

// Only an exact platform message fetched by the Discord REST provider supplies
// trusted shared-lane author provenance. Transcript/inbound-event search entries
// derive author_id from historical session state and must not authorize recovery.
async fn pending_recovery_actor(
    pool: &SqlitePool,
    key: &SessionKey,
    unfinished: &omon_gateway::storage::UnfinishedTurn,
) -> Result<Option<String>> {
    if key.platform == "discord" {
        if let Some(message_id) = unfinished.platform_message_id.as_deref() {
            let author: Option<String> = sqlx::query_scalar(
                "SELECT author_id FROM message_search_documents
                 WHERE platform = 'discord' AND guild_id IS ? AND channel_id = ?
                   AND message_id = ? AND CASE WHEN json_valid(metadata_json)
                       THEN json_extract(metadata_json, '$.source') END = 'discord_rest'",
            )
            .bind(&key.guild_id)
            .bind(key.thread_id.as_deref().unwrap_or(&key.channel_id))
            .bind(message_id)
            .fetch_optional(pool)
            .await?;
            // Even an empty/invalid REST author prevents a stale legacy fallback.
            if author.is_some() {
                return Ok(author);
            }
        }
    }
    if key.guild_id.is_none() {
        return Ok(Some(key.user_id.clone()));
    }
    Ok(sqlx::query_scalar(
        "SELECT p.author_id FROM legacy_guild_pending_auth p
         JOIN messages m ON m.id = p.message_id AND m.sequence = p.message_sequence
         WHERE m.id = ? AND m.session_key = ? AND p.canonical_key = m.session_key
           AND m.role = 'user'",
    )
    .bind(&unfinished.message_id)
    .bind(key.storage_key())
    .fetch_optional(pool)
    .await?)
}

// Deferred/unknown actors must not consume the global restart-loop budget merely
// because their durable pending markers are intentionally retained.
async fn count_authorized_pending_recoveries(
    pool: &SqlitePool,
    auth: &StartupAuthorization,
) -> Result<usize> {
    let mut count = 0;
    for key in omon_gateway::storage::fetch_resume_pending_session_keys(pool).await? {
        if omon_gateway::storage::is_session_suspended(pool, &key.storage_key()).await? {
            continue;
        }
        if let Some(unfinished) =
            omon_gateway::storage::find_last_unfinished_user_turn(pool, &key.storage_key()).await?
        {
            let actor = pending_recovery_actor(pool, &key, &unfinished).await?;
            if auth.is_authorized_actor(&key, actor.as_deref()) {
                count += 1;
            } else {
                warn!(session = %key, actor = ?actor, "deferring pending recovery: current actor unknown or unauthorized; marker retained");
            }
        }
    }
    Ok(count)
}

/// Delivery-ledger claim of the unfinished turn, so the replayed turn completes
/// the original claim instead of leaving it `in_progress` forever.
async fn recovery_delivery_id(
    pool: &SqlitePool,
    storage_key: &str,
    platform_message_id: Option<&str>,
) -> Result<Option<String>> {
    let Some(platform_message_id) = platform_message_id else {
        return Ok(None);
    };
    let delivery_id: Option<String> = sqlx::query_scalar(
        "SELECT message_id FROM delivery_ledger WHERE session_key = ? AND (platform_message_id = ? OR message_id = ?) ORDER BY created_at DESC LIMIT 1",
    )
    .bind(storage_key)
    .bind(platform_message_id)
    .bind(platform_message_id)
    .fetch_optional(pool)
    .await?;
    Ok(delivery_id)
}

/// Rebuilds the event for a stored unfinished turn.
///
/// The stored transcript content is already the rendered prompt (attachment
/// blocks and inlined file text included), so attachments are NOT re-attached:
/// that would append the blocks a second time. The original message id is kept
/// so the actor's `ON CONFLICT(id)` insert does not duplicate the user row, and
/// the `[system] ` transcript prefix is mapped back to system origin.
fn recovery_event(
    session: &SessionKey,
    unfinished: omon_gateway::storage::UnfinishedTurn,
    delivery_id: Option<String>,
) -> InboundEvent {
    let (origin, content) = match unfinished.content.strip_prefix("[system] ") {
        Some(rest) => (omon_gateway::EventOrigin::System, rest.to_owned()),
        None => (omon_gateway::EventOrigin::User, unfinished.content.clone()),
    };
    InboundEvent {
        id: Uuid::parse_str(&unfinished.message_id).unwrap_or_else(|_| Uuid::new_v4()),
        session: session.clone(),
        platform_message_id: String::new(),
        delivery_id,
        referenced_message_id: None,
        author_id: None,
        origin,
        content,
        attachments: Vec::new(),
        received_at: unfinished.created_at,
    }
}

pub async fn recover_resume_pending_sessions_with_auth(
    pool: &SqlitePool,
    multiplexer: &SessionMultiplexer,
    auth: Option<&StartupAuthorization>,
) -> Result<usize> {
    let pending_keys = omon_gateway::storage::fetch_resume_pending_session_keys(pool).await?;
    let mut resumed_count = 0;
    for session_key in pending_keys {
        let storage_key = session_key.storage_key();
        let is_suspended = omon_gateway::storage::is_session_suspended(pool, &storage_key).await?;
        if is_suspended {
            omon_gateway::storage::clear_session_resume_pending(pool, &storage_key).await?;
            info!(
                session = %session_key,
                "skipping restart recovery for suspended session"
            );
            continue;
        }

        if let Some(unfinished) =
            omon_gateway::storage::find_last_unfinished_user_turn(pool, &storage_key).await?
        {
            if let Some(auth) = auth {
                let actor = pending_recovery_actor(pool, &session_key, &unfinished).await?;
                if !auth.is_authorized_actor(&session_key, actor.as_deref()) {
                    warn!(
                        session = %session_key, actor = ?actor,
                        "deferring pending recovery: current actor unknown or unauthorized; marker retained"
                    );
                    continue;
                }
            }
            let delivery_id = recovery_delivery_id(
                pool,
                &storage_key,
                unfinished.platform_message_id.as_deref(),
            )
            .await?;
            let cleared =
                omon_gateway::storage::clear_session_resume_pending(pool, &storage_key).await?;
            if !cleared {
                continue;
            }
            let event = recovery_event(&session_key, unfinished, delivery_id);
            info!(
                session = %session_key,
                "re-dispatching unfinished user turn on restart recovery"
            );
            if let Err(error) = multiplexer.route(event).await {
                tracing::error!(
                    session = %session_key,
                    %error,
                    "failed to route resumed session event; marker restored for next boot"
                );
                omon_gateway::storage::mark_session_resume_pending(pool, &storage_key).await?;
            } else {
                resumed_count += 1;
            }
        } else {
            omon_gateway::storage::clear_session_resume_pending(pool, &storage_key).await?;
        }
    }
    Ok(resumed_count)
}

fn tool_enabled(name: &str, enabled: Option<&[String]>) -> bool {
    let Some(enabled) = enabled else { return true };
    enabled.iter().any(|toolset| {
        toolset == name
            || (toolset == "web" && matches!(name, "web_search" | "web_fetch"))
            || (toolset == "cron" && name == "cron")
    })
}

async fn ensure_agent_session(pool: &SqlitePool, session: &SessionContext) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(session.key.storage_key())
    .bind(&session.key.platform)
    .bind(&session.key.guild_id)
    .bind(&session.key.channel_id)
    .bind(&session.key.thread_id)
    .bind(&session.key.user_id)
    .bind(
        serde_json::to_string(&session.state)
            .map_err(|error| OmonError::Database(error.to_string()))?,
    )
    .bind(session.created_at)
    .bind(session.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

fn build_cron_tools(
    job: &HermesJob,
    defaults: &ToolRegistry,
    workspace_root: &Path,
) -> Result<Option<ToolRegistry>> {
    let Some(workdir) = job.workdir.as_ref() else {
        return Ok(None);
    };
    let roots = authorized_cron_roots(job, workspace_root)?;
    let workdir = canonical_authorized_directory(workdir, &roots, "Hermes workdir")?;
    let mut tools = defaults.clone();
    tools.register(TerminalTool::new(&workdir));
    tools.register(FileTool::new(&workdir));
    Ok(Some(tools))
}

fn parse_llm_provider(name: &str) -> Option<LlmProvider> {
    let lower = name.trim().to_lowercase();
    match lower.as_str() {
        "openai" | "gpt" => Some(LlmProvider::OpenAi),
        "anthropic" | "claude" => Some(LlmProvider::Anthropic),
        "deepseek" => Some(LlmProvider::DeepSeek),
        "ollama" => Some(LlmProvider::Ollama),
        _ => None,
    }
}

pub fn build_cron_llm_config(
    base: &LlmConfig,
    provider: Option<&str>,
    base_url: Option<&str>,
    model: Option<&str>,
) -> LlmConfig {
    let mut config = base.clone();

    if let Some(m) = model.filter(|m| !m.trim().is_empty()) {
        config.model = m.trim().to_string();
    }

    if let Some(p) = provider.filter(|p| !p.trim().is_empty()) {
        if let Some(parsed) = parse_llm_provider(p) {
            if parsed != config.provider {
                // The base key belongs to the base provider.
                config.api_key = None;
            }
            config.provider = parsed;
        } else {
            warn!(provider = %p, "Unknown LLM provider override, keeping base provider");
        }
    }

    if let Some(b) = base_url.filter(|b| !b.trim().is_empty()) {
        // Never send the base credential to a job-supplied endpoint.
        config.api_key = None;
        config.base_url = Some(b.trim().to_string());
    }

    config
}

/// Owner Discord id for the inbox digest. OMON_OWNER_ID overrides the built-in owner.
fn owner_id_from(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(omon_gateway::inbox::proposals::OWNER_ID)
        .to_owned()
}

/// Gmail account polled by the mail watcher. OMON_MAIL_ACCOUNT overrides the built-in account.
fn mail_account_from(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("freedomzero91@gmail.com")
        .to_owned()
}

fn hermes_skill_dirs(hermes_root: &Path, home: &Path) -> Vec<PathBuf> {
    vec![
        hermes_root.join("skills"),
        home.join(".omon").join("skills"),
    ]
}

/// Owner home DM that leads lanes, proposals and inbox triage. Zero-config:
/// OMON_LANE_DEFAULT_LEAD overrides the built-in OmOMeow owner DM.
fn default_lead() -> String {
    std::env::var("OMON_LANE_DEFAULT_LEAD")
        .unwrap_or_else(|_| omon_gateway::lanes::kibitzer::DEFAULT_LEAD.to_owned())
}

#[tokio::main]
#[allow(dead_code)]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    match Cli::parse().into_command() {
        Command::Run => run_gateway().await,
        Command::Migrate(args) => omon_gateway::migrate::run_migrate(args).await,
    }
}

/// Full Disk Access is the one TCC permission macOS never hands to a program
/// itself. Touching protected paths at startup registers this binary in the FDA
/// list; when the reads are still denied we surface the privacy pane so the
/// user makes the single toggle that then sticks (the binary is signed with a
/// stable Developer ID identity, so the grant survives rebuilds).
/// Returns true if all protected probes were readable, false if access was denied.
fn ensure_full_disk_access(workspace_root: &Path) -> bool {
    let home = env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/Library/Application Support/com.apple.TCC/TCC.db"),
        "/Library/Application Support/com.apple.TCC/TCC.db".to_string(),
        format!("{home}/Library/Messages/chat.db"),
        format!("{home}/Library/Safari"),
        format!("{home}/Library/Mail"),
    ];
    let mut denied = Vec::new();
    for path in candidates {
        let probe = Path::new(&path);
        if !probe.exists() {
            continue;
        }
        let outcome = if probe.is_dir() {
            match std::fs::read_dir(probe) {
                Ok(mut entries) => match entries.next() {
                    Some(Ok(_)) | None => Ok(()),
                    Some(Err(error)) => Err(error),
                },
                Err(error) => Err(error),
            }
        } else {
            std::fs::File::open(probe).and_then(|mut file| {
                let mut byte = [0u8; 1];
                std::io::Read::read(&mut file, &mut byte).map(|_| ())
            })
        };
        if let Err(error) = outcome {
            denied.push(format!("{path}: {error}"));
        }
    }
    if denied.is_empty() {
        info!("full disk access: granted (all protected probes readable)");
        return true;
    }
    let marker = workspace_root.join(FDA_PROMPTED_MARKER);
    if !claim_fda_prompt(&marker) {
        warn!(
            denied = %denied.join(" | "),
            "full disk access: denied; the privacy pane was already opened on an earlier boot"
        );
        return false;
    }
    warn!(
        denied = %denied.join(" | "),
        "full disk access: denied; opening the privacy pane once for a one-time toggle"
    );
    if cfg!(target_os = "macos") {
        if let Err(error) = tokio::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
            .spawn()
        {
            warn!(%error, "could not open the Full Disk Access privacy pane");
        }
    }
    false
}

/// Single marker file for the Full Disk Access owner DM. Its content is the binary identity
/// (size + mtime), so a new deploy re-alerts once while crash-loop restarts of the same binary
/// do not, and markers never accumulate (incident 2026-10-16).
const FDA_DM_MARKER: &str = ".fda_dm_prompted";

/// Sends a Discord DM alert to the owner if Full Disk Access is missing.
/// Dispatched at most once per binary identity (sized and timestamped marker)
/// to notify the owner upon new deployment without spamming on crash loops
/// (incident 2026-10-16).
async fn notify_owner_fda_denied_if_needed(
    workspace_root: &Path,
    http: &serenity::http::Http,
    owner_id: &str,
) {
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            warn!(%error, "could not determine current executable path for FDA notice");
            PathBuf::from("omo-gateway")
        }
    };
    let (size, mtime) = match tokio::fs::metadata(&current_exe).await {
        Ok(meta) => {
            let size = meta.len();
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (size, mtime)
        }
        Err(error) => {
            warn!(%error, "could not read metadata of current executable for FDA marker");
            (0, 0)
        }
    };
    let identity = format!("{size}:{mtime}");
    let marker_path = workspace_root.join(FDA_DM_MARKER);
    if tokio::fs::read_to_string(&marker_path)
        .await
        .is_ok_and(|seen| seen.trim() == identity)
    {
        return;
    }

    let message = format!(
        "⚠️ omo-gateway에 전체 디스크 접근 권한이 없습니다. iMessage 수집이 중단됩니다. 시스템 설정 → 개인정보 보호 및 보안 → 전체 디스크 접근 권한에서 {}를 켜 주세요.",
        current_exe.display()
    );

    let recipient = serde_json::json!({
        "recipient_id": owner_id,
    });

    match http.create_private_channel(&recipient).await {
        Ok(channel) => {
            if let Err(error) = channel.say(http, &message).await {
                warn!(%error, "failed to send Full Disk Access denial alert to owner via Discord DM");
            } else {
                info!(marker = %marker_path.display(), "notified owner of missing Full Disk Access via Discord DM");
                if let Err(error) = tokio::fs::write(&marker_path, &identity).await {
                    warn!(%error, path = %marker_path.display(), "could not record FDA DM marker");
                }
            }
        }
        Err(error) => {
            warn!(%error, "failed to create private DM channel with owner for FDA alert");
        }
    }
}

const FDA_PROMPTED_MARKER: &str = ".fda_prompted";

/// True exactly once per workspace: the marker records that the privacy pane
/// was already shown, so crash-looping launchd restarts do not re-open it.
fn claim_fda_prompt(marker: &Path) -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)
        .is_ok()
}

async fn run_gateway() -> Result<()> {
    run_gateway_with_dashboard(None).await
}

/// Fetches the Discord bot user for the provided HTTP client, retrying transient
/// transport-level network errors (DNS, connect, timeout, reqwest errors) with
/// exponential backoff (2s doubling to 60s cap). Permanent HTTP status errors
/// (e.g. 401 unauthorized due to invalid token) fail fast immediately (incident 2026-10-16).
async fn fetch_current_user_with_retry(
    http: &serenity::http::Http,
) -> Result<serenity::model::user::CurrentUser> {
    let mut backoff = std::time::Duration::from_secs(2);
    const MAX_BACKOFF: std::time::Duration = std::time::Duration::from_secs(60);

    loop {
        match http.get_current_user().await {
            Ok(user) => return Ok(user),
            Err(serenity::Error::Http(serenity::http::HttpError::Request(error))) => {
                warn!(
                    %error,
                    retry_in_secs = backoff.as_secs(),
                    "transient network failure during Discord bot identity lookup; retrying after backoff"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn run_gateway_with_dashboard(
    dashboard_settings: Option<dashboard::DashboardSettings>,
) -> Result<()> {
    if !parse_bool_from(optional_env("OMON_DISCORD_ENABLED").as_deref(), true) {
        if !parse_bool_from(optional_env("OMON_LANES_ENABLED").as_deref(), true) {
            return Err(OmonError::Config(
                "Discord-free gateway requires OMON_LANES_ENABLED".into(),
            ));
        }
        let url = required_env("DATABASE_URL")?;
        let pool = init_pool(&url).await?;
        let paths: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list")
            .fetch_all(&pool)
            .await?;
        info!(databases = ?paths, "gateway database opened");
        let shutdown = tokio_util::sync::CancellationToken::new();
        let workspace = env::var_os("OMON_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".omon/workspace")
            });
        validate_agent_backend_env()?;
        let backend_config = OmoBackendConfig::from_env()?.with_workspace_root(workspace.clone());
        // The Discord path and the dashboard spawn the local daemon; a headless gateway that
        // skipped this had no app-server at all, so every lane, lead and cron turn failed.
        let _daemon_supervisor = OmoDaemonSupervisor::ensure(&backend_config).await?;
        let headless_cron_config =
            OmoBackendConfig::cron_from_env()?.with_workspace_root(workspace.clone());
        let _cron_daemon_supervisor =
            if headless_cron_config.appserver_url == backend_config.appserver_url {
                None
            } else {
                OmoDaemonSupervisor::ensure(&headless_cron_config).await?
            };
        let mut routes = omon_gateway::try_parse_profile_routes(
            &optional_env("DISCORD_PROFILE_ROUTES").unwrap_or_default(),
        )
        .map_err(|error| OmonError::Config(format!("invalid DISCORD_PROFILE_ROUTES: {error}")))?;
        routes.extend(omon_gateway::parse_channel_prompts(
            &optional_env("DISCORD_CHANNEL_PROMPTS").unwrap_or_default(),
        ));
        let (multiplexer, watcher) = headless_lead(
            &pool,
            backend_config,
            ProfileRouter::new(routes),
            workspace.clone(),
        );
        let cron_backend = Arc::new(
            OmoBackend::new(headless_cron_config, Arc::new(LocalTranscriptOutput))
                .with_pool(pool.clone()),
        );
        let scheduler = CronScheduler::new(
            pool.clone(),
            Arc::new(AgentCronExecutor {
                backend: cron_backend,
                workspace_root: workspace.clone(),
                pool: pool.clone(),
                cron_script_timeout_secs: cron_script_timeout_secs_from(
                    optional_env("OMON_CRON_SCRIPT_TIMEOUT_SECS").as_deref(),
                ),
            }),
        );
        omon_gateway::cron::remediate::install(Arc::new(
            omon_gateway::cron::remediate::Remediator {
                pool: pool.clone(),
                judge: Arc::new(omon_gateway::judge::JevCli::new(
                    None,
                    Some(pool.clone()),
                    None,
                    None,
                )),
                runtime: Arc::new(omon_gateway::cron::remediate::GatewayRuntime {
                    pool: pool.clone(),
                    scheduler: scheduler.clone(),
                    briefs: workspace.join(".omo/cron-fix/briefs"),
                }),
            },
        ))?;
        let native_cron = omon_gateway::cron::report::bind_native().await?;
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|error| OmonError::Config(error.to_string()))?;
        let kibitzer = initialize_kibitzer(&pool).await?;
        let mut lanes = spawn_lane_service(&pool, &shutdown, kibitzer)?;
        scheduler.start().await;
        lanes.spawn(native_cron.serve(pool.clone(), scheduler.clone(), shutdown.clone()));
        let dispatcher = omon_gateway::lanes::dispatch::EventDispatcher::new(
            pool.clone(),
            multiplexer.clone(),
            Arc::new(omon_gateway::judge::JevCli::new(
                None,
                Some(pool.clone()),
                None,
                None,
            )),
        );
        lanes.spawn(dispatcher.serve(shutdown.clone()));
        let watcher_shutdown = shutdown.clone();
        lanes.spawn(async move {
            watcher.serve(watcher_shutdown).await;
            Ok(())
        });
        let (inbox_shutdown, inbox_shutdown_rx) = tokio::sync::watch::channel(false);
        let mut inbox = spawn_inbox_collector(&pool, inbox_shutdown_rx)?;
        info!(
            "headless gateway lead runtime ready; output delivery is local transcript acceptance"
        );
        let outcome = tokio::select! {
            result = lanes.join_next() => lane_service_exit(result),
            result = tokio::signal::ctrl_c() => result.map_err(|error| OmonError::Config(error.to_string())),
            _ = terminate.recv() => Ok(()),
        };
        shutdown.cancel();
        scheduler.shutdown().await;
        stop_inbox_collector(&mut inbox, inbox_shutdown).await;
        let cleanup = stop_lead_runtime(&multiplexer, &mut lanes).await;
        if let Err(error) = cleanup {
            return outcome.and(Err(error));
        }
        pool.close().await;
        info!("gateway lane service stopped");
        return outcome;
    }
    let config = Config::from_env()?;
    let fda_granted = ensure_full_disk_access(&config.workspace_root);
    let pool = init_pool(&config.database_url).await?;
    let kibitzer = initialize_kibitzer(&pool).await?;

    let approval_guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let loaded_allowlist = approval_guard.load_persisted_allowlist().await?;
    info!(
        loaded_allowlist,
        "loaded persisted approval allowlist entries"
    );
    let loaded_yolo = approval_guard.load_persisted_yolo().await?;
    info!(loaded_yolo, "loaded persisted yolo sessions");
    let approval_requester = Arc::new(DiscordApprovalRequester::new(
        approval_guard.clone(),
        std::time::Duration::from_secs(config.approval_timeout_secs),
    ));
    let (inbox_shutdown, inbox_shutdown_rx) = tokio::sync::watch::channel(false);
    let mut inbox = spawn_inbox_collector(&pool, inbox_shutdown_rx)?;
    let mut tools = ToolRegistry::new().with_approval_requester(
        approval_requester.clone(),
        std::time::Duration::from_secs(config.approval_timeout_secs + 5),
    );
    let mut terminal_tool = TerminalTool::new(&config.workspace_root)
        .with_authorized_roots(config.extra_tool_roots.clone())
        .with_approval(
            config.approval_policy,
            approval_requester.clone(),
            std::time::Duration::from_secs(config.approval_timeout_secs + 5),
        )
        .with_deny_globs(config.approvals_deny.clone());

    if let Some(scanner_url) = optional_env("TIRITH_SCANNER_URL") {
        let fail_open = parse_bool_from(optional_env("TIRITH_FAIL_OPEN").as_deref(), true);
        let timeout_secs = optional_env("TIRITH_TIMEOUT_SECS")
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(omon_gateway::DEFAULT_TIRITH_TIMEOUT_SECS);
        info!(
            url = %scanner_url,
            fail_open,
            timeout_secs,
            "configuring external security scanner (Tirith)"
        );
        let tirith_scanner = omon_gateway::TirithScanner::new(
            scanner_url,
            fail_open,
            std::time::Duration::from_secs(timeout_secs),
        );
        terminal_tool = terminal_tool.with_external_scanner(tirith_scanner);
    }
    tools.register(terminal_tool);
    tools.register(
        FileTool::new(&config.workspace_root)
            .with_authorized_roots(config.extra_tool_roots.clone()),
    );
    tools.register(McpTool::default());
    let cron_tool = CronTool::new(pool.clone());
    tools.register(cron_tool.clone());
    tools.register(omon_gateway::WebSearchTool);
    tools.register(omon_gateway::WebFetchTool);
    tools.register(omon_gateway::BrowserTool::default());
    let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let hermes_root = env::var_os("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".hermes"));
    tools.register(
        omon_gateway::SkillsTool::new(hermes_skill_dirs(&hermes_root, &home))
            .with_pool(pool.clone()),
    );
    let tool_names = tools.names();

    validate_agent_backend_env()?;
    let omo_config = OmoBackendConfig::from_env()?
        .with_workspace_root(config.workspace_root.clone())
        .with_default_model(Some(config.default_model.clone()));
    // Zero-config daemon lifecycle: spawn/keep-alive/kill the local
    // `omo app-server` unless an external one is already serving.
    let _daemon_supervisor = OmoDaemonSupervisor::ensure(&omo_config).await?;
    info!(
        appserver_url = %omo_config.appserver_url,
        "Initializing agent backend: OMO app-server"
    );
    let shared_dispatcher = Arc::new(SharedDispatcher::default());
    let omo_backend = Arc::new(
        OmoBackend::new(omo_config.clone(), shared_dispatcher.clone()).with_pool(pool.clone()),
    );
    let runner: Arc<dyn AgentBackend> = omo_backend.clone();
    let profile_router = ProfileRouter::new(config.profile_routes.clone());
    let drain_watcher = omon_gateway::DrainWatcher::new(
        config.workspace_root.clone(),
        std::time::Duration::from_secs(3),
    );
    drain_watcher.scan_at(
        omon_gateway::current_instantiation_epoch(),
        chrono::Utc::now(),
    );
    let mut drain_rx = drain_watcher.receiver();
    // The boot scan already published any marker present at startup; that initial value
    // must not count as a change, or a leftover marker would end the run immediately.
    drain_rx.mark_unchanged();
    let mut multiplexer = SessionMultiplexer::with_profile_router(
        pool.clone(),
        runner.clone(),
        Some(shared_dispatcher.clone()),
        MultiplexerConfig::default(),
        profile_router.clone(),
    )
    .with_drain_receiver(drain_rx.clone());
    if let Some((_, sender)) = &inbox {
        multiplexer = multiplexer.with_inbox_event_sender(sender.clone());
    }
    let scale_to_zero = ScaleToZero::start(multiplexer.clone());

    let mut bot_http_clients = HashMap::new();
    let mut default_bot_id = None;
    for token in &config.discord_bot_tokens {
        let http = Arc::new(serenity::http::Http::new(token));
        let bot_id = fetch_current_user_with_retry(&http).await?.id.to_string();
        if default_bot_id.is_none() {
            default_bot_id = Some(bot_id.clone());
        }
        if bot_http_clients.insert(bot_id.clone(), http).is_some() {
            return Err(OmonError::Config(format!(
                "multiple Discord tokens resolve to the same bot identity {bot_id}"
            )));
        }
    }
    let default_bot_id = default_bot_id
        .ok_or_else(|| OmonError::Config("no Discord bot identities were configured".into()))?;

    if !fda_granted {
        if let Some(default_http) = bot_http_clients.get(&default_bot_id) {
            let owner_id = owner_id_from(optional_env("OMON_OWNER_ID").as_deref());
            notify_owner_fda_denied_if_needed(&config.workspace_root, default_http, &owner_id)
                .await;
        }
    }
    let dead_targets = Arc::new(
        DeadTargetRegistry::new()
            .with_pool(pool.clone())
            .with_probe_interval(std::time::Duration::from_secs(60)),
    );
    dead_targets.load_from_db(&pool).await?;
    let discord_egress = Arc::new(
        DiscordEgress::with_bot_clients(default_bot_id.clone(), bot_http_clients)?
            .with_dead_targets(dead_targets.clone())
            .with_approval_mentions(config.allowed_users.clone(), config.approval_mentions)
            .with_runtime_footer(config.runtime_footer)
            .with_default_model(config.default_model.clone())
            .with_workspace_root(config.workspace_root.clone()),
    );
    let discord_egress_dispatcher: Arc<dyn OutboundDispatcher> = discord_egress.clone();
    shared_dispatcher.set(discord_egress.clone()).await;
    approval_requester
        .set_dispatcher(discord_egress.clone())
        .await;
    approval_requester
        .set_heartbeat(multiplexer.activity_heartbeat())
        .await;

    let retention_days = cron_runs_retention_days_from_environment()?;
    let pruned = prune_terminal_cron_runs(&pool, retention_days, chrono::Utc::now()).await?;
    info!(pruned, retention_days, "pruned old terminal cron runs");

    let cron_sync = HermesStoreSynchronizer::from_environment(pool.clone())?;
    let imported = cron_sync.sync().await?;
    info!(imported, "synchronized Hermes cron stores");

    let recovered = recover_pending_delivery_obligations(&pool, discord_egress.clone()).await?;
    info!(
        recovered,
        "recovered pending outbound delivery obligations on boot"
    );

    let restart_guard_path = config.workspace_root.join("restart_loop.json");
    let restart_guard = RestartLoopGuard::new(restart_guard_path);
    let auth = StartupAuthorization {
        allowed_users: config.allowed_users.clone(),
        allow_all_users: config.allow_all_users,
        allowed_channels: config.allowed_channels.clone(),
        ignored_channels: config.ignored_channels.clone(),
        paired_users: {
            let pairing = omon_gateway::discord::PairingStore::new(pool.clone());
            pairing.init_cache().await?;
            pairing.get_paired_user_ids().await
        },
    };
    let pending_sessions_count = count_authorized_pending_recoveries(&pool, &auth).await?;
    if pending_sessions_count > 0 {
        if restart_guard.check_and_record() {
            warn!(
                pending_sessions_count,
                "Restart-loop breaker TRIPPED: skipping auto-resume of in-flight sessions to break crash loop"
            );
        } else {
            let recovered_sessions =
                recover_resume_pending_sessions_with_auth(&pool, &multiplexer, Some(&auth)).await?;
            info!(
                recovered_sessions,
                "recovered resume_pending sessions on boot"
            );
        }
    }

    // Cron and interactive share one multiplexed daemon instance with distinct agent threads
    let mut cron_omo_config =
        OmoBackendConfig::cron_from_env()?.with_workspace_root(config.workspace_root.clone());
    cron_omo_config
        .default_model
        .get_or_insert_with(|| config.default_model.clone());
    let _cron_daemon_supervisor = if cron_omo_config.appserver_url == omo_config.appserver_url {
        None
    } else {
        OmoDaemonSupervisor::ensure(&cron_omo_config).await?
    };
    info!(
        appserver_url = %cron_omo_config.appserver_url,
        total_timeout_secs = cron_omo_config.total_timeout.as_secs(),
        "Initializing cron agent backend: OMO app-server"
    );
    let cron_backend: Arc<dyn AgentBackend> = Arc::new(
        OmoBackend::new(cron_omo_config, shared_dispatcher.clone()).with_pool(pool.clone()),
    );

    let scheduler = CronScheduler::with_dispatcher(
        pool.clone(),
        Arc::new(omon_gateway::inbox::digest::DigestExecutor {
            pool: pool.clone(),
            inner: Arc::new(AgentCronExecutor {
                backend: cron_backend,
                workspace_root: config.workspace_root.clone(),
                pool: pool.clone(),
                cron_script_timeout_secs: config.cron_script_timeout_secs,
            }),
        }),
        discord_egress,
    )
    .with_hermes_sync(cron_sync);
    // Persisted owner DM only; never fall back to DISCORD_HOME_CHANNEL.
    let owner_id = owner_id_from(optional_env("OMON_OWNER_ID").as_deref());
    let owner = owner_id.as_str();
    // The lead DM belongs to the OmOMeow bot; other bots' DMs with the owner do not count.
    let home_dm: Option<String> = sqlx::query_scalar(
        "SELECT session_key FROM sessions WHERE session_key=? AND platform='discord' \
         AND guild_id IS NULL AND thread_id IS NULL AND user_id=?",
    )
    .bind(default_lead())
    .bind(owner)
    .fetch_optional(&pool)
    .await?;
    if let Some(home_dm) = home_dm {
        let home_dm = home_dm.parse::<SessionKey>()?;
        omon_gateway::inbox::digest::register(&pool, &scheduler, &home_dm, owner).await?;
    } else {
        warn!("inbox digest not registered: owner home DM is not yet persisted");
    }
    scheduler.start().await;
    cron_tool.bind_scheduler(Arc::new(scheduler.clone()));

    let mut poise_data = PoiseData::new(multiplexer.clone(), pool.clone());
    poise_data.approvals = approval_guard.clone();
    poise_data.pairing_store.init_cache().await?;
    poise_data.profile_router = profile_router;
    poise_data.runtime_footer = config.runtime_footer;
    poise_data.destructive_slash_confirm = config.destructive_slash_confirm;
    poise_data.missed_backfill = config.discord_missed_backfill;
    poise_data.llm = LlmClient::new(config.llm_config(config.default_model.clone())).ok();
    poise_data.tools = tool_names;
    if omon_gateway::inbox::triage::enabled() {
        use omon_gateway::inbox::proposals::{GatewayActions, Proposals, ReadOnlyDirect, OWNER_ID};
        Proposals::new(pool.clone(), OWNER_ID).await?;
        let lead = default_lead();
        SessionKey::from_storage_key(&lead)?;
        let model =
            omon_gateway::agent::model_for_purpose(omon_gateway::agent::ModelPurpose::InboxWrite);
        let direct = ReadOnlyDirect {
            llm: LlmClient::new(config.llm_config(model))?,
        };
        poise_data.inbox_actions = Some(Arc::new(GatewayActions {
            pool: pool.clone(),
            judge: Arc::new(omon_gateway::inbox::triage::production_judge(pool.clone())),
            direct: Arc::new(direct),
            brief_directory: std::env::current_dir()
                .map_err(|e| OmonError::Config(e.to_string()))?
                .join(".omo/inbox-proposals"),
            lead_session_key: lead,
            parent_channel_id: None,
        }));
    }
    poise_data.tool_registry = tools.clone();
    poise_data.free_response_channels = config.free_response_channels.clone();
    poise_data.allowed_users = config.allowed_users.clone();
    poise_data.allowed_roles = config.allowed_roles.clone();
    poise_data.allow_all_users = config.allow_all_users;
    poise_data.thread_sessions_per_user = config.thread_sessions_per_user;
    poise_data.thread_require_mention = config.thread_require_mention;
    poise_data.allow_bots = config.allow_bots;
    poise_data.allowed_channels = config.allowed_channels.clone();
    poise_data.ignored_channels = config.ignored_channels.clone();
    poise_data.auto_thread = config.auto_thread;
    poise_data.channel_topic_context = config.channel_topic_context;
    poise_data.channel_context = config.channel_context;
    poise_data.channel_context_limit = config.channel_context_limit;
    poise_data.processing_reactions = config.processing_reactions;
    poise_data.approval_mentions = config.approval_mentions;
    poise_data.approvals_deny = config.approvals_deny.clone();
    let mut downloader = AttachmentDownloader::new(&config.workspace_root)?;
    if let Some(api_key) = &config.openai_api_key {
        let base_url = config
            .openai_api_base
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        downloader = downloader.with_stt(Arc::new(OpenAiSpeechToText::new(api_key, base_url)));
    }
    poise_data.attachment_downloader = Some(downloader);
    poise_data.primary_bot_id = Some(default_bot_id.parse().map_err(|_| {
        OmonError::Config(format!(
            "invalid primary Discord bot identity {default_bot_id}"
        ))
    })?);
    let adapter = DiscordAdapter::new(poise_data.clone()).with_approval_guard(approval_guard);

    let mut clients = Vec::new();
    let mut shard_managers = Vec::new();
    for token in &config.discord_bot_tokens {
        let client = adapter.client(token).await?;
        shard_managers.push(client.shard_manager.clone());
        clients.push(client);
    }

    let readiness = omon_gateway::collect_runtime_readiness(
        &pool,
        &config.workspace_root,
        &config.default_model,
        clients.len(),
    )
    .await;
    if readiness.is_ok() {
        info!(status = %readiness.status, checks = ?readiness.checks, "runtime readiness probes passed");
    } else {
        warn!(status = %readiness.status, checks = ?readiness.checks, "runtime readiness probes reported degraded status");
    }

    info!(
        model = %config.default_model,
        database = %config.database_url,
        bot_count = clients.len(),
        "omo-gateway listening on Discord"
    );

    let lane_shutdown = tokio_util::sync::CancellationToken::new();
    omon_gateway::cron::remediate::install(Arc::new(omon_gateway::cron::remediate::Remediator {
        pool: pool.clone(),
        judge: Arc::new(omon_gateway::judge::JevCli::new(
            None,
            Some(pool.clone()),
            None,
            None,
        )),
        runtime: Arc::new(omon_gateway::cron::remediate::GatewayRuntime {
            pool: pool.clone(),
            scheduler: scheduler.clone(),
            briefs: config.workspace_root.join(".omo/cron-fix/briefs"),
        }),
    }))?;
    // A restart during a remediation rerun would otherwise leave the problem stuck in
    // `remediating` with nobody told.
    match omon_gateway::cron::remediate::sweep_stale_remediating().await {
        Ok(0) => {}
        Ok(count) => info!(count, "re-escalated stale cron remediations"),
        Err(error) => warn!(%error, "failed to sweep stale cron remediations"),
    }
    let mut lane_tasks = spawn_lane_service(&pool, &lane_shutdown, kibitzer)?;
    lane_tasks.spawn(omon_gateway::cron::report::serve_native(
        pool.clone(),
        scheduler.clone(),
        lane_shutdown.clone(),
    ));
    if parse_bool_from(optional_env("OMON_LANES_ENABLED").as_deref(), true) {
        let dispatcher = omon_gateway::lanes::dispatch::EventDispatcher::new(
            pool.clone(),
            multiplexer.clone(),
            Arc::new(omon_gateway::judge::JevCli::new(
                None,
                Some(pool.clone()),
                None,
                None,
            )),
        );
        lane_tasks.spawn(dispatcher.serve(lane_shutdown.clone()));
    }
    let watcher_shutdown = lane_shutdown.clone();
    lane_tasks.spawn(async move {
        drain_watcher.serve(watcher_shutdown).await;
        Ok(())
    });
    if parse_bool_from(optional_env("OMON_SESSION_WATCH_ENABLED").as_deref(), true) {
        match (
            omon_gateway::sessions::control::SessionControl::from_environment(),
            SessionKey::from_storage_key(&default_lead()),
        ) {
            (Ok(control), Ok(owner_dm)) => {
                let watcher = omon_gateway::sessions::watch::Watcher::with_policy(
                    pool.clone(),
                    control,
                    omon_gateway::sessions::watch::interval_from_env(),
                    omon_gateway::sessions::watch::NotifyPolicy::from_env(),
                );
                lane_tasks.spawn(watcher.serve(
                    discord_egress_dispatcher.clone(),
                    owner_dm,
                    lane_shutdown.clone(),
                ));
            }
            (control, owner_dm) => warn!(
                control_error = ?control.err().map(|error| error.to_string()),
                owner_error = ?owner_dm.err().map(|error| error.to_string()),
                "session watcher not started"
            ),
        }
    }

    let dashboard_shutdown = tokio_util::sync::CancellationToken::new();
    let mut dashboard_server = None;
    if let Some(settings) = dashboard_settings {
        let state = attach_gateway_dashboard(
            &settings,
            &approval_requester,
            &shared_dispatcher,
            discord_egress_dispatcher.clone(),
            pool.clone(),
            multiplexer.clone(),
            scheduler.clone(),
            tools.clone(),
            dashboard::config_view_from_gateway(&config, &settings),
            config.workspace_root.clone(),
            hermes_skill_dirs(&hermes_root, &home),
            config.discord_bot_tokens.len(),
        )
        .await;
        dashboard_server =
            Some(dashboard::spawn_server(settings, state, dashboard_shutdown.clone()).await?);
    }

    let mut join_set = tokio::task::JoinSet::new();
    for mut client in clients {
        join_set.spawn(async move { client.start().await });
    }

    #[cfg(unix)]
    let mut sigterm_stream =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .map_err(|error| OmonError::Config(format!("failed to listen for SIGTERM: {error}")))?;
    let mut discord_task_failed = false;
    let mut lane_failure = None;
    tokio::select! {
        result = lane_tasks.join_next(), if !lane_tasks.is_empty() => {
            lane_failure = lane_service_exit(result).err();
            omon_gateway::global_debouncer().flush_all(&poise_data).await;
            let _ = multiplexer.mark_in_flight_resume_pending().await;
            for sm in shard_managers {
                sm.shutdown_all().await;
            }
        }
        Some(res) = join_set.join_next() => {
            if discord_client_failed(&res) {
                discord_task_failed = true;
            }
            omon_gateway::global_debouncer().flush_all(&poise_data).await;
            let _ = multiplexer.mark_in_flight_resume_pending().await;
        }
        signal = tokio::signal::ctrl_c() => {
            signal.map_err(|error| OmonError::Config(format!("failed to listen for Ctrl+C: {error}")))?;
            info!("shutdown signal received");
            omon_gateway::global_debouncer().flush_all(&poise_data).await;
            let _ = multiplexer.mark_in_flight_resume_pending().await;
            for sm in shard_managers {
                sm.shutdown_all().await;
            }
        }
        _ = wait_for_drain(&mut drain_rx) => {
            warn!("drain request detected via .drain_request.json marker; shutting down gracefully");
            omon_gateway::global_debouncer().flush_all(&poise_data).await;
            let _ = multiplexer.mark_in_flight_resume_pending().await;
            for sm in shard_managers {
                sm.shutdown_all().await;
            }
        }
        // Production stop paths (systemctl stop, launchctl kill, container
        // stop) send SIGTERM, not SIGINT. Without this arm every graceful
        // shutdown marker, in-flight turn, and resume-state write is skipped
        // exactly when the process is stopped normally.
        _ = async {
            #[cfg(unix)]
            {
                sigterm_stream.recv().await;
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        } => {
            info!("SIGTERM received; shutting down gracefully");
            omon_gateway::global_debouncer().flush_all(&poise_data).await;
            let _ = multiplexer.mark_in_flight_resume_pending().await;
            for sm in shard_managers {
                sm.shutdown_all().await;
            }
        }
    }

    lane_shutdown.cancel();
    dashboard_shutdown.cancel();
    if let Some(server) = dashboard_server {
        let _ = server.await;
    }
    stop_inbox_collector(&mut inbox, inbox_shutdown).await;
    scheduler.shutdown().await;
    scale_to_zero.shutdown().await;
    let cleanup = stop_lead_runtime(&multiplexer, &mut lane_tasks).await;
    cleanup?;
    pool.close().await;
    if let Some(error) = lane_failure {
        return Err(error);
    }
    if discord_task_failed {
        // A crashed Discord task must not look like a successful run to the
        // supervisor; exit non-zero so systemd/launchd restarts us.
        std::process::exit(1);
    }
    warn!("omo-gateway stopped");
    Ok(())
}

/// Resolves when the drain flag becomes true. A drain that was already active
/// at boot is the initial value, so clearing or expiring it (true -> false) must
/// not end the wait: only a transition to true does. A dropped sender never
/// resolves, leaving shutdown to the other select arms.
async fn wait_for_drain(drain_rx: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if drain_rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
        if *drain_rx.borrow() {
            return;
        }
    }
}

struct LocalTranscriptOutput;

#[cfg(test)]
mod headless_tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use tokio_tungstenite::tungstenite::Message;

    #[tokio::test]
    async fn qa_gaps_headless_inbox_starts_when_enabled_and_joins() {
        let (shutdown, mut receiver) = tokio::sync::watch::channel(false);
        let (started, observed) = tokio::sync::oneshot::channel();
        let mut inbox = start_inbox_when_enabled(true, || {
            let (sender, _events) = omon_gateway::inbox::watchers::event_channel(1);
            Ok((
                tokio::spawn(async move {
                    started.send(()).unwrap();
                    receiver.wait_for(|stopped| *stopped).await.unwrap();
                }),
                sender,
            ))
        })
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), observed)
            .await
            .unwrap()
            .unwrap();
        stop_inbox_collector(&mut inbox, shutdown).await;
        assert!(inbox.as_ref().unwrap().0.is_finished());
    }

    #[tokio::test]
    async fn qa_gaps_headless_cron_socket_is_bound_and_removed_on_shutdown() {
        struct Executor;
        #[async_trait]
        impl omon_gateway::cron::CronTaskExecutor for Executor {
            async fn execute(&self, _: &omon_gateway::CronJob) -> Result<Option<String>> {
                unreachable!()
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cron.sock");
        let db = omon_gateway::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let scheduler = CronScheduler::new(db.pool().clone(), Arc::new(Executor));
        let server = omon_gateway::cron::report::NativeServer::bind(path.clone())
            .await
            .unwrap();
        assert!(tokio::net::UnixStream::connect(&path).await.is_ok());
        let shutdown = tokio_util::sync::CancellationToken::new();
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(server.serve(db.pool().clone(), scheduler, shutdown.clone()));
        shutdown.cancel();
        tasks.join_next().await.unwrap().unwrap().unwrap();
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn qa_gaps_headless_inbox_does_not_start_when_disabled() {
        let (shutdown, _receiver) = tokio::sync::watch::channel(false);
        let mut inbox: Option<InboxRuntime> = start_inbox_when_enabled(false, || {
            panic!("disabled inbox must not construct collectors or triage");
        })
        .unwrap();
        stop_inbox_collector(&mut inbox, shutdown).await;
        assert!(inbox.is_none());
    }

    struct StartupOnlyBackend;

    #[async_trait]
    impl AgentBackend for StartupOnlyBackend {
        async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
            panic!("startup must not invoke a model");
        }
    }

    #[tokio::test]
    async fn qa_gaps_kibitzer_enabled_initializes_before_spawning() {
        let database = omon_gateway::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let pool = database.pool();
        let worker = initialize_kibitzer_when_enabled(true, || {
            omon_gateway::lanes::kibitzer::Kibitzer::new(
                pool.clone(),
                SessionKey::from_storage_key(omon_gateway::lanes::kibitzer::DEFAULT_LEAD).unwrap(),
                Arc::new(omon_gateway::judge::JevCli::new(None, None, None, None)),
                Arc::new(StartupOnlyBackend),
                Arc::new(omon_gateway::lanes::kibitzer::Capture::default()),
                Arc::new(omon_gateway::work_heartbeat::SystemClock),
                true,
            )
        })
        .await
        .unwrap();
        let objects: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name IN ('kibitzer_state','kibitzer_inbox_insert','kibitzer_lane_insert')")
            .fetch_one(pool).await.unwrap();
        assert_eq!(objects, 3);
        let states: i64 = sqlx::query_scalar("SELECT count(*) FROM kibitzer_state")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(states, 1);
        let shutdown = tokio_util::sync::CancellationToken::new();
        let mut tasks = tokio::task::JoinSet::new();
        let hint = spawn_kibitzer(&mut tasks, worker, &shutdown);
        assert!(hint.is_some());
        assert_eq!(tasks.len(), 1);
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(1), tasks.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn qa_gaps_kibitzer_disabled_has_no_schema_or_task() {
        let database = omon_gateway::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let worker = initialize_kibitzer_when_enabled(false, || {
            panic!("disabled kibitzer must not be constructed");
        })
        .await
        .unwrap();
        let objects: i64 = sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name IN ('kibitzer_state','kibitzer_inbox_insert','kibitzer_lane_insert')")
            .fetch_one(database.pool()).await.unwrap();
        assert_eq!(objects, 0);
        let mut tasks = tokio::task::JoinSet::new();
        assert!(spawn_kibitzer(
            &mut tasks,
            worker,
            &tokio_util::sync::CancellationToken::new()
        )
        .is_none());
        assert!(tasks.is_empty());
    }

    struct DeliveryBarrier {
        delivered: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    }

    struct BackendDeliveryBarrier {
        backend: OmoBackend,
        delivered: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    }

    #[async_trait]
    impl omon_gateway::AgentBackend for BackendDeliveryBarrier {
        async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
            omon_gateway::AgentBackend::run(&self.backend, session, event).await?;
            self.delivered
                .lock()
                .await
                .take()
                .unwrap()
                .send(())
                .unwrap();
            std::future::pending().await
        }
    }

    #[async_trait]
    impl OutboundDispatcher for DeliveryBarrier {
        async fn dispatch(&self, action: OutboundAction) -> Result<()> {
            let final_chunk =
                matches!(&action, OutboundAction::Stream { chunk, .. } if chunk.is_final);
            LocalTranscriptOutput.dispatch(action).await?;
            if final_chunk {
                self.delivered
                    .lock()
                    .await
                    .take()
                    .unwrap()
                    .send(())
                    .unwrap();
                std::future::pending::<()>().await;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn headless_early_shutdown_preserves_accepted_reply() {
        early_shutdown_preserves_reply(false).await;
        early_shutdown_preserves_reply(true).await;
    }

    async fn early_shutdown_preserves_reply(delivery_returned: bool) {
        let root = tempfile::tempdir().unwrap();
        let pool = init_pool(&format!(
            "sqlite://{}",
            root.path().join("early.db").display()
        ))
        .await
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            for method in ["initialize", "thread/start", "turn/start"] {
                let request: Value =
                    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert_eq!(request["method"], method);
                let result = if method == "turn/start" {
                    json!({"turn":{"id":"early-turn","status":"inProgress"}})
                } else {
                    json!({"thread":{"id":"early-thread"}})
                };
                ws.send(Message::text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string(),
                ))
                .await
                .unwrap();
            }
            ws.send(Message::text(json!({"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{"threadId":"early-thread","turnId":"early-turn","itemId":"early-item","delta":"EARLY_DURABLE_REPLY"}}).to_string())).await.unwrap();
            ws.send(Message::text(json!({"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"early-thread","turn":{"id":"early-turn","status":"completed"}}}).to_string())).await.unwrap();
        });
        let (delivered, accepted) = tokio::sync::oneshot::channel();
        let backend: Arc<dyn omon_gateway::AgentBackend> = if delivery_returned {
            Arc::new(BackendDeliveryBarrier {
                backend: OmoBackend::new(
                    OmoBackendConfig::new(format!("ws://{address}")),
                    Arc::new(LocalTranscriptOutput),
                )
                .with_pool(pool.clone()),
                delivered: tokio::sync::Mutex::new(Some(delivered)),
            })
        } else {
            let output = Arc::new(DeliveryBarrier {
                delivered: tokio::sync::Mutex::new(Some(delivered)),
            });
            Arc::new(
                OmoBackend::new(OmoBackendConfig::new(format!("ws://{address}")), output)
                    .with_pool(pool.clone()),
            )
        };
        let mux = SessionMultiplexer::new(pool.clone(), backend, MultiplexerConfig::default());
        let lead = SessionKey::new("local", None::<String>, "early14", None::<String>, "owner");
        mux.route(InboundEvent::message(lead.clone(), "early14", "reply"))
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), accepted)
            .await
            .unwrap()
            .unwrap();
        // Acceptance happened, but the real backend remains inside dispatch until cancellation.
        mux.shutdown().await.unwrap();
        peer.await.unwrap();
        let replies: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_key = ? AND role = 'assistant' AND content = 'EARLY_DURABLE_REPLY'").bind(lead.storage_key()).fetch_one(&pool).await.unwrap();
        assert_eq!(replies, 1);
        let state: String =
            sqlx::query_scalar("SELECT state FROM delivery_obligations WHERE session_key = ?")
                .bind(lead.storage_key())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            state,
            if delivery_returned {
                "delivered"
            } else {
                "attempting"
            }
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn headless_marker_reply_and_joined_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let pool = init_pool(&format!("sqlite://{}", root.path().join("qa.db").display()))
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (finish_tx, finish_rx) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            for method in ["initialize", "thread/start", "turn/start"] {
                let request: Value =
                    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert_eq!(request["method"], method);
                if method == "turn/start" {
                    assert!(request["params"]["input"]
                        .to_string()
                        .contains("[LANE_EVENT]"));
                }
                let result = if method == "turn/start" {
                    json!({"turn":{"id":"t14","status":"inProgress"}})
                } else {
                    json!({"thread":{"id":"r14"}})
                };
                ws.send(Message::text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string(),
                ))
                .await
                .unwrap();
            }
            ws.send(Message::text(json!({"jsonrpc":"2.0","method":"item/agentMessage/delta","params":{"threadId":"r14","turnId":"t14","itemId":"m14","delta":"TASK14_REPLY_SENTINEL"}}).to_string())).await.unwrap();
            ws.send(Message::text(json!({"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"r14","turn":{"id":"t14","status":"completed"}}}).to_string())).await.unwrap();
            finish_rx.await.unwrap();
        });
        omon_gateway::write_drain_request(root.path(), Some("task14-test"), true).unwrap();
        let config = OmoBackendConfig::new(format!("ws://{address}"))
            .with_workspace_root(root.path().to_path_buf());
        let (mux, watcher) = headless_lead(
            &pool,
            config,
            ProfileRouter::default(),
            root.path().to_path_buf(),
        );
        let mut drain = watcher.receiver();
        assert!(mux.is_draining());
        let lead = SessionKey::new(
            "local",
            None::<String>,
            "headless14",
            None::<String>,
            "owner",
        );
        let id = sqlx::query("INSERT INTO lane_events (lead_session_key, kind, body, created_at) VALUES (?, 'done_claimed', 'TASK14_INPUT', ?)")
            .bind(lead.storage_key()).bind(chrono::Utc::now()).execute(&pool).await.unwrap().last_insert_rowid();
        let dispatcher = omon_gateway::lanes::dispatch::EventDispatcher::new(
            pool.clone(),
            mux.clone(),
            Arc::new(omon_gateway::judge::JevCli::new(
                None,
                Some(pool.clone()),
                None,
                None,
            )),
        );
        dispatcher.tick(chrono::Utc::now()).await.unwrap();
        let receipt: Option<String> =
            sqlx::query_scalar("SELECT delivery_reason FROM lane_events WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(receipt.is_none());
        let mut refused = InboundEvent::message(lead.clone(), "probe", "TASK14_INPUT");
        refused.origin = omon_gateway::EventOrigin::System;
        assert!(mux.route(refused).await.is_err());
        omon_gateway::clear_drain_request(root.path()).unwrap();
        watcher.scan_at(
            omon_gateway::current_instantiation_epoch(),
            chrono::Utc::now(),
        );
        drain.changed().await.unwrap();
        assert!(!*drain.borrow());
        dispatcher.tick(chrono::Utc::now()).await.unwrap();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            mux.wait_for_session_idle(&lead),
        )
        .await
        .unwrap()
        .unwrap();
        let context = mux.session_context(&lead).await.unwrap().unwrap();
        assert_eq!(
            context.state.metadata.get("omo_thread_id"),
            Some(&json!("r14"))
        );
        finish_tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), peer)
            .await
            .unwrap()
            .unwrap();
        mux.shutdown().await.unwrap();
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT role, content FROM messages WHERE session_key = ? ORDER BY sequence",
        )
        .bind(lead.storage_key())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(rows
            .iter()
            .any(|(role, content)| role == "user" && content.starts_with("[system] ")));
        assert!(rows
            .iter()
            .any(|(role, content)| role == "assistant" && content == "TASK14_REPLY_SENTINEL"));
        let receipt: String =
            sqlx::query_scalar("SELECT delivery_reason FROM lane_events WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(receipt, "lead_turn");
        let delivered: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM delivery_obligations WHERE session_key = ? AND state = 'delivered'")
            .bind(lead.storage_key()).fetch_one(&pool).await.unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(mux.active_sessions(), 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn headless_output_rejects_upload_and_approval() {
        let session = SessionKey::new("local", None::<String>, "output14", None::<String>, "owner");
        assert!(LocalTranscriptOutput
            .dispatch(OutboundAction::UploadFile {
                session: session.clone(),
                path: PathBuf::from("local.txt"),
            })
            .await
            .is_err());
        assert!(LocalTranscriptOutput
            .dispatch(OutboundAction::ApprovalRequest {
                session,
                request_id: Uuid::new_v4(),
                command: "command".into(),
                reason: "approval".into(),
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn headless_active_shutdown_joins_interrupt_and_preserves_binding() {
        let root = tempfile::tempdir().unwrap();
        let pool = init_pool(&format!(
            "sqlite://{}",
            root.path().join("shutdown.db").display()
        ))
        .await
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            for method in ["initialize", "thread/start", "turn/start"] {
                let request: Value =
                    serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert_eq!(request["method"], method);
                let result = if method == "turn/start" {
                    json!({"turn":{"id":"t14-stop","status":"inProgress"}})
                } else {
                    json!({"thread":{"id":"r14-stop"}})
                };
                ws.send(Message::text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string(),
                ))
                .await
                .unwrap();
            }
            ws.send(Message::Ping(vec![14].into())).await.unwrap();
            assert!(matches!(
                ws.next().await.unwrap().unwrap(),
                Message::Pong(_)
            ));
            started_tx.send(()).unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            let mut control = tokio_tungstenite::accept_async(socket).await.unwrap();
            for method in ["initialize", "turn/interrupt"] {
                let request: Value =
                    serde_json::from_str(control.next().await.unwrap().unwrap().to_text().unwrap())
                        .unwrap();
                assert_eq!(request["method"], method);
                if method == "turn/interrupt" {
                    assert_eq!(request["params"]["threadId"], "r14-stop");
                }
                control
                    .send(Message::text(
                        json!({"jsonrpc":"2.0","id":request["id"],"result":{}}).to_string(),
                    ))
                    .await
                    .unwrap();
            }
        });
        let config = OmoBackendConfig::new(format!("ws://{address}"))
            .with_workspace_root(root.path().to_path_buf());
        let (mux, _) = headless_lead(
            &pool,
            config,
            ProfileRouter::default(),
            root.path().to_path_buf(),
        );
        let lead = SessionKey::new("local", None::<String>, "stop14", None::<String>, "owner");
        mux.route(InboundEvent::message(lead.clone(), "stop14", "TASK14_STOP"))
            .await
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        mux.shutdown().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), peer)
            .await
            .unwrap()
            .unwrap();
        assert!(mux
            .route(InboundEvent::message(lead.clone(), "late14", "late"))
            .await
            .is_err());
        let (pending, state): (i64, String) =
            sqlx::query_as("SELECT resume_pending, state_json FROM sessions WHERE session_key = ?")
                .bind(lead.storage_key())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(pending, 1);
        let state: Value = serde_json::from_str(&state).unwrap();
        assert_eq!(state["metadata"]["omo_thread_id"], "r14-stop");
        assert_eq!(state["suspended"], false);
        assert_eq!(mux.active_sessions(), 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn headless_watcher_cancels_and_joins() {
        let root = tempfile::tempdir().unwrap();
        omon_gateway::write_drain_request(root.path(), Some("task14-test"), true).unwrap();
        let watcher = omon_gateway::DrainWatcher::new(
            root.path().to_path_buf(),
            std::time::Duration::from_secs(3),
        );
        let mut receiver = watcher.receiver();
        let shutdown = tokio_util::sync::CancellationToken::new();
        let task = tokio::spawn(watcher.serve(shutdown.clone()));
        tokio::time::timeout(std::time::Duration::from_secs(5), receiver.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(*receiver.borrow());
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
    }
}

#[async_trait]
impl OutboundDispatcher for LocalTranscriptOutput {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action {
            OutboundAction::Stream { session, chunk } => {
                if chunk.is_final {
                    info!(session = %session, stream_id = %chunk.stream_id, delivery = "local_transcript", "headless local output accepted");
                }
                Ok(())
            }
            OutboundAction::UploadFile { .. } => Err(OmonError::Multiplexer(
                "headless output cannot perform external uploads".into(),
            )),
            OutboundAction::ApprovalRequest { .. } => Err(OmonError::Multiplexer(
                "headless output has no approval recipient".into(),
            )),
            OutboundAction::SendMessage { .. }
            | OutboundAction::EditMessage { .. }
            | OutboundAction::DeleteMessage { .. }
            | OutboundAction::Typing { .. }
            | OutboundAction::React { .. }
            | OutboundAction::ExpireApproval { .. } => Err(OmonError::Multiplexer(
                "headless local output requires Stream".into(),
            )),
        }
    }
}

fn headless_lead(
    pool: &SqlitePool,
    config: OmoBackendConfig,
    router: ProfileRouter,
    workspace: PathBuf,
) -> (SessionMultiplexer, omon_gateway::DrainWatcher) {
    let watcher = omon_gateway::DrainWatcher::new(workspace, std::time::Duration::from_secs(3));
    watcher.scan_at(
        omon_gateway::current_instantiation_epoch(),
        chrono::Utc::now(),
    );
    let backend =
        Arc::new(OmoBackend::new(config, Arc::new(LocalTranscriptOutput)).with_pool(pool.clone()));
    let mux = SessionMultiplexer::with_profile_router(
        pool.clone(),
        backend,
        None,
        MultiplexerConfig::default(),
        router,
    )
    .with_drain_receiver(watcher.receiver());
    (mux, watcher)
}

async fn stop_lead_runtime(
    mux: &SessionMultiplexer,
    tasks: &mut tokio::task::JoinSet<Result<()>>,
) -> Result<()> {
    let (actors, services) = tokio::join!(mux.shutdown(), async {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let mut failure = None;
            while let Some(result) = tasks.join_next().await {
                let result = result
                    .map_err(|error| OmonError::Multiplexer(error.to_string()))
                    .and_then(|result| result);
                if let Err(error) = result {
                    failure.get_or_insert(error);
                }
            }
            failure.map_or(Ok(()), Err)
        })
        .await
        .map_err(|_| OmonError::Multiplexer("runtime shutdown exceeded 30 seconds".into()))?
    });
    actors.and(services)
}

async fn initialize_kibitzer(
    pool: &SqlitePool,
) -> Result<Option<omon_gateway::lanes::kibitzer::Kibitzer>> {
    initialize_kibitzer_when_enabled(
        parse_bool_from(optional_env("OMON_KIBITZER_ENABLED").as_deref(), true),
        || omon_gateway::lanes::kibitzer::Kibitzer::from_env(pool.clone()),
    )
    .await
}

async fn initialize_kibitzer_when_enabled(
    enabled: bool,
    build: impl FnOnce() -> Result<omon_gateway::lanes::kibitzer::Kibitzer>,
) -> Result<Option<omon_gateway::lanes::kibitzer::Kibitzer>> {
    if !enabled {
        info!("kibitzer disabled; no worker started");
        return Ok(None);
    }
    let worker = build()?;
    worker.initialize().await?;
    info!("kibitzer initialized before producers");
    Ok(Some(worker))
}

fn spawn_kibitzer(
    tasks: &mut tokio::task::JoinSet<Result<()>>,
    worker: Option<omon_gateway::lanes::kibitzer::Kibitzer>,
    shutdown: &tokio_util::sync::CancellationToken,
) -> Option<Arc<tokio::sync::Notify>> {
    worker.map(|worker| {
        let hint = Arc::new(tokio::sync::Notify::new());
        let serve_hint = hint.clone();
        let shutdown = shutdown.clone();
        tasks.spawn(async move { worker.serve(serve_hint, shutdown).await });
        hint
    })
}

fn spawn_lane_service(
    pool: &sqlx::SqlitePool,
    shutdown: &tokio_util::sync::CancellationToken,
    kibitzer: Option<omon_gateway::lanes::kibitzer::Kibitzer>,
) -> Result<tokio::task::JoinSet<Result<()>>> {
    let mut tasks = tokio::task::JoinSet::new();
    let kibitzer_hint = spawn_kibitzer(&mut tasks, kibitzer, shutdown);
    if parse_bool_from(optional_env("OMON_LANES_ENABLED").as_deref(), true) {
        let mut runner = omon_gateway::lanes::runner::LaneRunner::from_env(pool.clone())?;
        if let Some(hint) = kibitzer_hint {
            runner = runner.with_kibitzer_hint(hint);
        }
        tasks.spawn(runner.serve(shutdown.clone()));
    }
    Ok(tasks)
}

type InboxRuntime = (
    tokio::task::JoinHandle<()>,
    omon_gateway::inbox::watchers::InboxEventSender,
);

fn start_inbox_when_enabled<T>(
    enabled: bool,
    start: impl FnOnce() -> Result<T>,
) -> Result<Option<T>> {
    if enabled {
        start().map(Some)
    } else {
        Ok(None)
    }
}

fn spawn_inbox_collector(
    pool: &SqlitePool,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<Option<InboxRuntime>> {
    start_inbox_when_enabled(omon_gateway::inbox::triage::enabled(), || {
        spawn_enabled_inbox_collector(pool, shutdown)
    })
}

fn spawn_enabled_inbox_collector(
    pool: &SqlitePool,
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<InboxRuntime> {
    let rules = omon_gateway::inbox::default_filters()?;
    let (event_tx, event_rx) = omon_gateway::inbox::watchers::event_channel(
        omon_gateway::inbox::watchers::DEFAULT_EVENT_CHANNEL_CAPACITY,
    );
    event_tx
        .try_send(omon_gateway::inbox::watchers::InboxWatcherEvent::Startup)
        .map_err(|error| OmonError::Config(error.to_string()))?;
    let pool = pool.clone();
    let watcher_tx = event_tx.clone();
    let triage = omon_gateway::inbox::triage::Triage::new(
        pool.clone(),
        Arc::new(omon_gateway::inbox::triage::production_judge(pool.clone())),
        Arc::new(
            omon_gateway::inbox::triage::SmallModelWriter::new(
                omon_gateway::agent::OmoBackendConfig::from_env()?,
            )
            .with_pool(pool.clone()),
        ),
        default_lead(),
    );
    let triage_shutdown = shutdown.clone();
    let task = tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();

        let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(16);
        tasks.spawn(
            omon_gateway::inbox::watchers::run_event_watch_with_shutdown(
                pool.clone(),
                inbox_sources(&pool),
                rules,
                event_rx,
                Some(progress_tx),
                Some(shutdown.clone()),
            ),
        );
        tasks.spawn(triage.serve(progress_rx, triage_shutdown));
        tasks.spawn(run_imessage_event_watcher(
            pool.clone(),
            watcher_tx.clone(),
            shutdown.clone(),
        ));
        tasks.spawn(run_mail_event_watcher(watcher_tx.clone(), shutdown.clone()));
        tasks.spawn(run_kakao_event_timer(watcher_tx, shutdown.clone()));

        let mut shutdown = shutdown;
        tokio::select! {
            _ = async {
                while let Some(result) = tasks.join_next().await {
                    if let Err(error) = result {
                        warn!(%error, "inbox watcher task stopped");
                    }
                }
            } => {}
            _ = async {
                loop {
                    if *shutdown.borrow() || shutdown.changed().await.is_err() {
                        break;
                    }
                }
            } => {}
        }
        while tasks.join_next().await.is_some() {}
    });
    Ok((task, event_tx))
}

fn inbox_sources(pool: &SqlitePool) -> Vec<Box<dyn omon_gateway::inbox::InboxSource>> {
    vec![
        Box::new(omon_gateway::inbox::sources::mail::MailSource),
        Box::new(omon_gateway::inbox::sources::kakao::KakaoSource::default()),
        Box::new(omon_gateway::inbox::sources::imessage::IMessageSource),
        Box::new(omon_gateway::inbox::sources::discord::DiscordSource::new(
            pool.clone(),
        )),
    ]
}

fn log_inbox_results(results: Vec<omon_gateway::inbox::SourceCollectResult>) {
    for result in results {
        if let Some(error) = result.error {
            let auth_error = error.contains("auth_expired")
                || error.to_ascii_lowercase().contains("unauthorized")
                || error.to_ascii_lowercase().contains("credential");
            warn!(
                source = result.source.as_str(),
                auth_error,
                "inbox source collection failed; check local CLI and credential readiness"
            );
        } else {
            info!(
                source = result.source.as_str(),
                inserted = result.stats.inserted,
                duplicates = result.stats.duplicates,
                dropped = result.stats.dropped,
                "inbox source collection completed"
            );
        }
    }
}

async fn run_kakao_event_timer(
    sender: omon_gateway::inbox::watchers::InboxEventSender,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let first_tick = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut interval = tokio::time::interval_at(first_tick, std::time::Duration::from_secs(120));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if *shutdown.borrow() {
            break;
        }
        tokio::select! {
            _ = interval.tick() => {},
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() { break; }
            }
        }
        tokio::select! {
            result = omon_gateway::inbox::watchers::on_periodic(&sender) => {
                if result.is_err() { break; }
            }
            _ = shutdown.changed() => break,
        }
    }
}

const WATCHER_BACKOFF_INITIAL: std::time::Duration = std::time::Duration::from_secs(5);
const WATCHER_BACKOFF_MAX: std::time::Duration = std::time::Duration::from_secs(300);
const WATCHER_HEALTHY_RUN: std::time::Duration = std::time::Duration::from_secs(60);

/// How one watcher child lifetime ended.
#[derive(Clone, Debug, PartialEq, Eq)]
enum WatcherExit {
    /// Shutdown (or a closed event channel): do not restart.
    Stop,
    /// Failed or exited without doing its job: restart with growing backoff.
    Failed { stderr_tail: String },
    /// Did its job and exited on its own: restart after the initial delay.
    Completed,
}

const WATCHER_STDERR_TAIL_MAX_BYTES: usize = 2048;

/// Keeps the most recent stderr output lines up to a bounded byte size (~2 KiB).
/// Older lines are dropped from the front when total byte length exceeds the limit
/// (incident 2026-10-16).
#[derive(Debug, Clone)]
pub(crate) struct StderrTailBuffer {
    max_bytes: usize,
    total_bytes: usize,
    lines: std::collections::VecDeque<String>,
}

impl StderrTailBuffer {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            total_bytes: 0,
            lines: std::collections::VecDeque::new(),
        }
    }

    pub(crate) fn push_line(&mut self, line: &str) {
        let line_len = line.len();
        let line = if line_len > self.max_bytes {
            let start = line_len - self.max_bytes;
            let mut char_start = start;
            while char_start < line_len && !line.is_char_boundary(char_start) {
                char_start += 1;
            }
            &line[char_start..]
        } else {
            line
        };

        let added_bytes = line.len() + if self.lines.is_empty() { 0 } else { 1 };
        self.lines.push_back(line.to_string());
        self.total_bytes += added_bytes;

        while self.total_bytes > self.max_bytes && self.lines.len() > 1 {
            if let Some(front) = self.lines.pop_front() {
                let removed = front.len() + 1;
                self.total_bytes = self.total_bytes.saturating_sub(removed);
            }
        }
    }

    pub(crate) fn render(&self) -> String {
        let mut out = String::with_capacity(self.total_bytes);
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        out
    }
}

fn spawn_stderr_tail_collector(
    stderr: Option<tokio::process::ChildStderr>,
) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        let Some(stderr) = stderr else {
            return String::new();
        };
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(stderr).lines();
        let mut buffer = StderrTailBuffer::new(WATCHER_STDERR_TAIL_MAX_BYTES);
        while let Ok(Some(line)) = lines.next_line().await {
            buffer.push_line(&line);
        }
        buffer.render()
    })
}

/// Combines a child's stderr tail with the non-event lines it printed on stdout, so a
/// watcher that reports failures on stdout (imsg) still explains why it exited.
fn join_tails(stderr_tail: String, stdout_noise: &StderrTailBuffer) -> String {
    let stdout_tail = stdout_noise.render();
    match (stderr_tail.is_empty(), stdout_tail.is_empty()) {
        (_, true) => stderr_tail,
        (true, false) => stdout_tail,
        (false, false) => format!("{stderr_tail}\n{stdout_tail}"),
    }
}

async fn reap_stderr_tail(handle: tokio::task::JoinHandle<String>) -> String {
    tokio::time::timeout(std::time::Duration::from_millis(500), handle)
        .await
        .ok()
        .and_then(|res| res.ok())
        .unwrap_or_default()
}

/// Returns true if the stdout line from `zele mail watch` represents a real mail event.
/// Comment/header lines starting with '#' (e.g. `# Watching inbox every 60s (Ctrl+C to stop)`)
/// and blank lines are ignored so immediate exits are not classified as completed runs
/// (incident 2026-10-16).
pub(crate) fn is_mail_event_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.starts_with('#')
}

fn next_watcher_backoff(current: std::time::Duration) -> std::time::Duration {
    (current * 2).min(WATCHER_BACKOFF_MAX)
}

/// Sleeps for `delay`; false when shutdown arrived first.
async fn watcher_backoff(
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    delay: std::time::Duration,
) -> bool {
    let sleep = tokio::time::sleep(delay);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => return true,
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return false;
                }
            }
        }
    }
}

/// Restarts an inbox watcher child with bounded exponential backoff instead of
/// letting one failure end the watcher until the gateway restarts.
async fn supervise_watcher<F, Fut>(
    name: &'static str,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    mut run_once: F,
) where
    F: FnMut(tokio::sync::watch::Receiver<bool>) -> Fut,
    Fut: std::future::Future<Output = WatcherExit>,
{
    let mut delay = WATCHER_BACKOFF_INITIAL;
    let mut last_fda_error: Option<String> = None;
    loop {
        if *shutdown.borrow() {
            return;
        }
        let started = tokio::time::Instant::now();
        let outcome = run_once(shutdown.clone()).await;
        let wait = match outcome {
            WatcherExit::Stop => return,
            WatcherExit::Completed => {
                last_fda_error = None;
                delay = WATCHER_BACKOFF_INITIAL;
                delay
            }
            WatcherExit::Failed { stderr_tail } => {
                if name == "imsg"
                    && (stderr_tail.contains("Operation not permitted")
                        || stderr_tail.contains("Cannot access Messages database")
                        || stderr_tail.contains("authorization denied"))
                    && last_fda_error.as_deref() != Some(stderr_tail.as_str())
                {
                    tracing::error!(
                        watcher = name,
                        stderr_tail = %stderr_tail,
                        "imsg watcher failed due to missing Full Disk Access; grant Full Disk Access to omo-gateway in System Settings to resume iMessage collection"
                    );
                    last_fda_error = Some(stderr_tail);
                }
                if started.elapsed() >= WATCHER_HEALTHY_RUN {
                    delay = WATCHER_BACKOFF_INITIAL;
                }
                let wait = delay;
                delay = next_watcher_backoff(delay);
                wait
            }
        };
        warn!(
            watcher = name,
            retry_in_secs = wait.as_secs(),
            "inbox watcher stopped; restarting after backoff"
        );
        if !watcher_backoff(&mut shutdown, wait).await {
            return;
        }
    }
}

async fn run_imessage_event_watcher(
    pool: SqlitePool,
    sender: omon_gateway::inbox::watchers::InboxEventSender,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    supervise_watcher("imsg", shutdown, move |shutdown| {
        let pool = pool.clone();
        let sender = sender.clone();
        async move { run_imessage_watch_once(&pool, &sender, shutdown).await }
    })
    .await;
}

/// One `imsg watch` child lifetime.
async fn run_imessage_watch_once(
    pool: &SqlitePool,
    sender: &omon_gateway::inbox::watchers::InboxEventSender,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> WatcherExit {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;

    let since_rowid = match omon_gateway::inbox::watchers::latest_imessage_rowid(pool).await {
        Ok(rowid) => rowid,
        Err(error) => {
            warn!(%error, "could not read numeric iMessage watcher rowid; starting without --since-rowid");
            None
        }
    };
    let mut child = match Command::new("imsg")
        .args(omon_gateway::inbox::watchers::imsg_watch_args(since_rowid))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            warn!(%error, "imsg watch could not start; check Full Disk Access and local CLI readiness");
            return WatcherExit::Failed {
                stderr_tail: String::new(),
            };
        }
    };
    let stderr_task = spawn_stderr_tail_collector(child.stderr.take());
    let Some(stdout) = child.stdout.take() else {
        warn!("imsg watch has no stdout pipe");
        let stderr_tail = reap_stderr_tail(stderr_task).await;
        return WatcherExit::Failed { stderr_tail };
    };
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    // imsg reports errors such as a Full Disk Access denial on stdout, not stderr.
    let mut stdout_noise = StderrTailBuffer::new(WATCHER_STDERR_TAIL_MAX_BYTES);
    let mut exit = WatcherExit::Failed {
        stderr_tail: String::new(),
    };
    loop {
        if *shutdown.borrow() {
            exit = WatcherExit::Stop;
            break;
        }
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    exit = WatcherExit::Stop;
                    break;
                }
            }
            line = lines.next_line() => match line {
                Ok(Some(line)) => {
                    let Some(rowid) = omon_gateway::inbox::watchers::parse_imsg_watch_line(&line) else {
                        stdout_noise.push_line(&line);
                        continue;
                    };
                    {
                        tokio::select! {
                            result = omon_gateway::inbox::watchers::on_imessage_event(sender, Some(rowid)) => {
                                if result.is_err() {
                                    exit = WatcherExit::Stop;
                                    break;
                                }
                            }
                            _ = shutdown.changed() => {
                                exit = WatcherExit::Stop;
                                break;
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    warn!(%error, "failed reading imsg watch output");
                    break;
                }
            },
            status = child.wait() => {
                let stderr_tail = join_tails(reap_stderr_tail(stderr_task).await, &stdout_noise);
                match status {
                    Ok(status) => warn!(
                        ?status,
                        stderr_tail = %stderr_tail,
                        "imsg watch exited; check local permissions and CLI readiness"
                    ),
                    Err(error) => warn!(
                        %error,
                        stderr_tail = %stderr_tail,
                        "waiting for imsg watch failed"
                    ),
                }
                return WatcherExit::Failed { stderr_tail };
            }
        }
    }
    if let Err(error) = child.kill().await {
        warn!(%error, "failed to stop imsg watch child");
    }
    let status = child.wait().await;
    let stderr_tail = join_tails(reap_stderr_tail(stderr_task).await, &stdout_noise);
    match exit {
        WatcherExit::Failed { .. } => {
            warn!(
                ?status,
                stderr_tail = %stderr_tail,
                "imsg watch exited; check local permissions and CLI readiness"
            );
            WatcherExit::Failed { stderr_tail }
        }
        other => other,
    }
}

async fn run_mail_event_watcher(
    sender: omon_gateway::inbox::watchers::InboxEventSender,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    supervise_watcher("mail", shutdown, move |shutdown| {
        let sender = sender.clone();
        async move { run_mail_watch_once(&sender, shutdown).await }
    })
    .await;
}

/// One `zele mail watch` child lifetime. Every output line raises a mail event
/// as it arrives (coalesced when the channel is full), so a long-running watch
/// is not silent until it exits.
async fn run_mail_watch_once(
    sender: &omon_gateway::inbox::watchers::InboxEventSender,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> WatcherExit {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;
    use tokio::sync::mpsc::error::TrySendError;

    let account = mail_account_from(optional_env("OMON_MAIL_ACCOUNT").as_deref());
    let mut child = match Command::new("zele")
        .args([
            "mail",
            "watch",
            "--interval",
            "60",
            "--folder",
            "inbox",
            "--account",
            account.as_str(),
            "--filter",
            "is:unread -category:promotions -category:social -category:updates",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            warn!(%error, "zele mail watch could not start; check local CLI and credential readiness");
            return WatcherExit::Failed {
                stderr_tail: String::new(),
            };
        }
    };
    let stderr_task = spawn_stderr_tail_collector(child.stderr.take());
    let Some(stdout) = child.stdout.take() else {
        warn!("zele mail watch has no stdout pipe");
        let stderr_tail = reap_stderr_tail(stderr_task).await;
        return WatcherExit::Failed { stderr_tail };
    };
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    let mut produced_event = false;
    let mut stop = false;
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    stop = true;
                    break;
                }
            }
            line = lines.next_line() => match line {
                Ok(Some(line)) if is_mail_event_line(&line) => {
                    produced_event = true;
                    match sender.try_send(omon_gateway::inbox::watchers::InboxWatcherEvent::Mail) {
                        Ok(()) | Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Closed(_)) => {
                            stop = true;
                            break;
                        }
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => {
                    warn!(%error, "failed reading zele mail watch output");
                    break;
                }
            }
        }
    }
    if stop {
        let _ = child.kill().await;
        let _ = child.wait().await;
        let _ = reap_stderr_tail(stderr_task).await;
        return WatcherExit::Stop;
    }
    let status = child.wait().await;
    let stderr_tail = reap_stderr_tail(stderr_task).await;
    match status {
        Ok(status) if status.success() && produced_event => WatcherExit::Completed,
        Ok(status) => {
            warn!(
                ?status,
                produced_event,
                stderr_tail = %stderr_tail,
                "zele mail watch exited without a clean event run; check local account credential readiness"
            );
            WatcherExit::Failed { stderr_tail }
        }
        Err(error) => {
            warn!(
                %error,
                stderr_tail = %stderr_tail,
                "waiting for zele mail watch failed"
            );
            WatcherExit::Failed { stderr_tail }
        }
    }
}

async fn stop_inbox_collector(
    inbox: &mut Option<InboxRuntime>,
    shutdown: tokio::sync::watch::Sender<bool>,
) {
    let Some((task, _)) = inbox.as_mut() else {
        return;
    };
    let _ = shutdown.send(true);
    match tokio::time::timeout(std::time::Duration::from_secs(5), &mut *task).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!(%error, "inbox collector task failed while stopping"),
        Err(_) => {
            task.abort();
            let _ = task.await;
        }
    }
}

fn discord_client_failed<E: std::fmt::Debug>(
    res: &std::result::Result<std::result::Result<(), E>, tokio::task::JoinError>,
) -> bool {
    match res {
        Ok(Ok(())) => false,
        Ok(Err(err)) => {
            tracing::error!("Discord client exited with error: {:?}", err);
            true
        }
        Err(join_error) => {
            tracing::error!("Discord client task failed: {join_error}");
            true
        }
    }
}

fn lane_service_exit(
    result: Option<std::result::Result<Result<()>, tokio::task::JoinError>>,
) -> Result<()> {
    match result {
        Some(Ok(Err(error))) => Err(error),
        Some(Err(error)) => Err(OmonError::Multiplexer(error.to_string())),
        Some(Ok(Ok(()))) | None => Err(OmonError::Multiplexer(
            "lane service stopped unexpectedly".into(),
        )),
    }
}

fn required_env(name: &str) -> Result<String> {
    optional_env(name)
        .ok_or_else(|| OmonError::Config(format!("missing required environment variable {name}")))
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn approval_timeout_secs_from(raw: Option<&str>) -> u64 {
    raw.and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(900)
}

/// Unrecognised tokens fall back to `default` instead of silently reading as
/// false, so a typo cannot disable a default-true safety flag.
pub fn parse_bool_from(raw: Option<&str>, default: bool) -> bool {
    let Some(value) = raw else { return default };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        other => {
            if !other.is_empty() {
                warn!(
                    value = other,
                    default, "unrecognised boolean setting; using the default"
                );
            }
            default
        }
    }
}

// An allowlist that was set but parses to nothing would silently mean
// "unrestricted"; refuse to start instead.
fn parse_allowlist(name: &str, raw: Option<&str>) -> Result<Vec<u64>> {
    let ids = parse_u64_list(raw);
    if ids.is_empty() && raw.is_some_and(|s| !s.trim().is_empty()) {
        return Err(OmonError::Config(format!(
            "{name} is set but contains no valid numeric ids"
        )));
    }
    Ok(ids)
}

pub fn parse_u64_list(raw: Option<&str>) -> Vec<u64> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .filter_map(|p| match p.parse::<u64>() {
                Ok(id) => Some(id),
                Err(_) => {
                    warn!(entry = p, "ignoring unparsable id in list");
                    None
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod runner_tests {
    use std::collections::HashMap;
    use std::fs;

    use clap::Parser;

    use super::{
        approval_timeout_secs_from, canonical_authorized_directory, hermes_skill_dirs,
        load_cron_skills, tool_enabled, Cli, Command,
    };
    use omon_gateway::{
        cron_script_timeout_secs_from, resolve_cron_script_timeout, HermesJob,
        DEFAULT_CRON_SCRIPT_TIMEOUT_SECS,
    };

    #[test]
    fn parses_cron_script_timeout_secs_from_env() {
        assert_eq!(cron_script_timeout_secs_from(Some("300")), 300);
        assert_eq!(cron_script_timeout_secs_from(Some(" 3600 ")), 3600);
        assert_eq!(cron_script_timeout_secs_from(None), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("   ")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("0")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("-10")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("invalid")), 1800);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn cron_script_timeout_reaps_descendants() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("hermes");
        let scripts_dir = home.join("scripts");
        fs::create_dir_all(&scripts_dir).unwrap();

        let pid_file = temp.path().join("child.pid");
        let script = format!(
            "#!/bin/bash\nsleep 30 &\necho $! > {}\nwait\n",
            pid_file.display()
        );
        fs::write(scripts_dir.join("leaker.sh"), script).unwrap();

        let job_json = serde_json::json!({
            "id": "leaker-job",
            "name": "Leaker Job",
            "prompt": "run",
            "script": "leaker.sh",
            "timeout_secs": 1,
            "schedule": {"kind": "cron", "expr": "0 * * * *"},
            "_omon_hermes_home": home.to_str().unwrap()
        });

        let hermes_job: HermesJob = serde_json::from_value(job_json).unwrap();
        let res = omon_gateway::run_cron_script(&hermes_job, "leaker.sh", temp.path(), 1).await;
        assert!(res.is_err(), "Script must timeout");

        let pid_str = fs::read_to_string(&pid_file).unwrap_or_default();
        let pid: i32 = pid_str.trim().parse().unwrap();

        #[cfg(unix)]
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let ret = unsafe { libc::kill(pid, 0) };
            if ret == 0 {
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
            assert_ne!(
                ret, 0,
                "Descendant process {pid} must NOT be running after script timeout"
            );
        }
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn cron_monitor_unchanged_skips_agent() {
        use omon_gateway::CronTaskExecutor;
        use std::sync::Arc;

        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("hermes");
        let scripts_dir = home.join("scripts");
        fs::create_dir_all(&scripts_dir).unwrap();

        let data_file = temp.path().join("data.txt");
        fs::write(&data_file, "initial content").unwrap();

        let monitor_script = format!("#!/bin/sh\ncat {}\n", data_file.display());
        fs::write(scripts_dir.join("monitor.sh"), monitor_script).unwrap();

        let job_json = serde_json::json!({
            "id": "monitored-job",
            "name": "Monitored Job",
            "prompt": "analyze changes",
            "monitor_script": "monitor.sh",
            "schedule": {"kind": "cron", "expr": "0 * * * *"},
            "_omon_hermes_home": home.to_str().unwrap()
        });

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let hermes_job: HermesJob = serde_json::from_value(job_json).unwrap();
        let run_count = Arc::new(std::sync::Mutex::new(0));

        #[derive(Clone)]
        struct MonitorTestBackend {
            runs: Arc<std::sync::Mutex<usize>>,
        }
        #[async_trait::async_trait]
        impl omon_gateway::AgentBackend for MonitorTestBackend {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                _event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                *self.runs.lock().unwrap() += 1;
                Ok(())
            }
        }

        let executor = omon_gateway::AgentCronExecutor {
            backend: Arc::new(MonitorTestBackend {
                runs: run_count.clone(),
            }),
            workspace_root: home.clone(),
            pool: pool.clone(),
            cron_script_timeout_secs: 10,
        };

        let cron_job = omon_gateway::CronJob {
            id: "monitored-job".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: serde_json::to_string(&hermes_job).unwrap(),
            enabled: true,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            authority: "omon_owned".into(),
        };

        let res1 = executor.execute(&cron_job).await;
        assert!(res1.is_ok());
        assert_eq!(
            *run_count.lock().unwrap(),
            1,
            "Agent must run on first changed/initial snapshot"
        );

        let res2 = executor.execute(&cron_job).await;
        assert!(res2.is_ok());
        assert_eq!(
            *run_count.lock().unwrap(),
            1,
            "Agent must be SKIPPED when monitor snapshot is unchanged"
        );

        fs::write(&data_file, "updated content").unwrap();
        let res3 = executor.execute(&cron_job).await;
        assert!(res3.is_ok());
        assert_eq!(
            *run_count.lock().unwrap(),
            2,
            "Agent must run when monitor snapshot changes"
        );
    }

    #[tokio::test]
    async fn cron_notepad_is_durable_and_profile_scoped() {
        use omon_gateway::CronTaskExecutor;
        use serde_json::Value;
        use std::sync::Arc;
        use uuid::Uuid;

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // 1. Set notepad for profile A and profile B for the same job_id
        omon_gateway::set_cron_notepad(&pool, "alpha", "job1", "cursor", "100")
            .await
            .unwrap();
        omon_gateway::set_cron_notepad(&pool, "beta", "job1", "cursor", "200")
            .await
            .unwrap();

        let notes_alpha = omon_gateway::get_cron_notepads(&pool, "alpha", "job1")
            .await
            .unwrap();
        assert_eq!(
            notes_alpha,
            vec![("cursor".to_string(), "100".to_string())],
            "Profile alpha must retain its scoped cursor note"
        );

        let notes_beta = omon_gateway::get_cron_notepads(&pool, "beta", "job1")
            .await
            .unwrap();
        assert_eq!(
            notes_beta,
            vec![("cursor".to_string(), "200".to_string())],
            "Profile beta must retain its scoped cursor note"
        );

        // 2. Reject notes exceeding 16 KiB
        let oversized = "x".repeat(17 * 1024);
        let err = omon_gateway::set_cron_notepad(&pool, "alpha", "job1", "big", &oversized)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("16 KiB"));

        // 3. Verify notepad is injected into the assembled cron prompt
        struct NotepadTestBackend {
            captured: Arc<std::sync::Mutex<Option<String>>>,
        }
        #[async_trait::async_trait]
        impl omon_gateway::AgentBackend for NotepadTestBackend {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                *self.captured.lock().unwrap() = Some(event.content);
                Ok(())
            }
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let home = std::env::temp_dir().join(format!("omon-test-notepad-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();

        let executor = omon_gateway::AgentCronExecutor {
            backend: Arc::new(NotepadTestBackend {
                captured: captured.clone(),
            }),
            workspace_root: home.clone(),
            pool: pool.clone(),
            cron_script_timeout_secs: 10,
        };

        let mut extra = HashMap::new();
        extra.insert("profile".into(), Value::String("alpha".into()));

        let hermes_job = HermesJob {
            id: "job1".into(),
            name: "Notepad Job".into(),
            prompt: "Summarize status".into(),
            skills: vec![],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            script: None,
            no_agent: false,
            ack_command: None,
            context_from: None,
            schedule: omon_gateway::HermesSchedule::default(),
            schedule_display: "".into(),
            repeat: omon_gateway::HermesRepeat::default(),
            enabled: true,
            state: "".into(),
            created_at: None,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            last_delivery_error: None,
            deliver: None,
            failure_deliver: None,
            origin: None,
            enabled_toolsets: None,
            workdir: None,
            attach_to_session: None,
            timeout_secs: None,
            brief_file: None,
            catch_up_hours: None,
            monitor_script: None,
            monitor_url: None,
            monitor_state: None,
            extra,
        };

        let cron_job = omon_gateway::CronJob {
            id: "job1".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: serde_json::to_string(&hermes_job).unwrap(),
            enabled: true,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            authority: "omon_owned".into(),
        };

        executor.execute(&cron_job).await.unwrap();

        let prompt = captured
            .lock()
            .unwrap()
            .clone()
            .expect("Prompt must be captured");
        assert!(
            prompt.contains("[Notepad]\ncursor: 100"),
            "Prompt must contain scoped notepad note: {prompt}"
        );
        assert!(
            !prompt.contains("200"),
            "Prompt must not contain other profile note"
        );

        let _ = std::fs::remove_dir_all(home);
    }

    #[tokio::test]
    async fn cron_preflight_blocks_unconfigured_discord() {
        use omon_gateway::{CronJob, CronScheduler, CronTaskExecutor, OmonError};
        use std::sync::Arc;

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        struct CountingExecutor(Arc<std::sync::Mutex<usize>>);
        #[async_trait::async_trait]
        impl CronTaskExecutor for CountingExecutor {
            async fn execute(
                &self,
                _job: &CronJob,
            ) -> std::result::Result<Option<String>, OmonError> {
                *self.0.lock().unwrap() += 1;
                Ok(Some("completed".into()))
            }
        }

        let runs = Arc::new(std::sync::Mutex::new(0));
        let executor = Arc::new(CountingExecutor(runs.clone()));
        // CronScheduler with NO dispatcher
        let scheduler = CronScheduler::new(pool.clone(), executor);

        // 1. Explicit discord delivery with unconfigured discord -> preflight blocked
        let explicit_discord_job = serde_json::json!({
            "id": "unconfigured_discord_job",
            "name": "Unconfigured Discord",
            "prompt": "run task",
            "deliver": "discord:12345",
            "schedule": {"kind": "cron", "expr": "0 * * * *"}
        });
        let cron_job_discord = CronJob {
            id: "unconfigured_discord_job".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: serde_json::to_string(&explicit_discord_job).unwrap(),
            enabled: true,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            authority: "omon_owned".into(),
        };

        let err = scheduler.execute_job(&cron_job_discord).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("unconfigured delivery transport: discord"),
            "Preflight must fail with unconfigured transport: {err}"
        );
        assert_eq!(
            *runs.lock().unwrap(),
            0,
            "Agent must not be invoked when delivery preflight fails"
        );

        // 2. Local delivery -> bypasses check and executes
        let local_job = serde_json::json!({
            "id": "local_job",
            "name": "Local Job",
            "prompt": "run task",
            "deliver": "local",
            "schedule": {"kind": "cron", "expr": "0 * * * *"}
        });
        let cron_job_local = CronJob {
            id: "local_job".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: serde_json::to_string(&local_job).unwrap(),
            enabled: true,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            authority: "omon_owned".into(),
        };

        let res = scheduler.execute_job(&cron_job_local).await;
        assert!(res.is_ok(), "Local job must execute successfully");
        assert_eq!(
            *runs.lock().unwrap(),
            1,
            "Agent must be invoked for local delivery"
        );
    }

    /// Serializes tests that mutate process-global environment variables.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn default_model_precedence_matches_dashboard() {
        let _env = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Case 1: OMON_DEFAULT_MODEL=A, DEFAULT_MODEL=B -> both resolve A
        unsafe {
            std::env::set_var("OMON_DEFAULT_MODEL", "model_a");
            std::env::set_var("DEFAULT_MODEL", "model_b");
        }
        let gw_model = super::Config::resolve_default_model();
        let db_model = super::dashboard_runtime::effective_default_model();
        assert_eq!(gw_model, "model_a");
        assert_eq!(db_model, "model_a");

        // Case 2: Only DEFAULT_MODEL=B -> both resolve B
        unsafe {
            std::env::remove_var("OMON_DEFAULT_MODEL");
            std::env::set_var("DEFAULT_MODEL", "model_b");
        }
        let gw_model = super::Config::resolve_default_model();
        let db_model = super::dashboard_runtime::effective_default_model();
        assert_eq!(gw_model, "model_b");
        assert_eq!(db_model, "model_b");

        // Case 3: Both absent -> both resolve to documented fallback "mahoquot/nekos-claude-sonnet-5-5"
        unsafe {
            std::env::remove_var("OMON_DEFAULT_MODEL");
            std::env::remove_var("DEFAULT_MODEL");
        }
        let gw_model = super::Config::resolve_default_model();
        let db_model = super::dashboard_runtime::effective_default_model();
        assert_eq!(gw_model, "mahoquot/nekos-claude-sonnet-5-5");
        assert_eq!(db_model, "mahoquot/nekos-claude-sonnet-5-5");
    }

    #[tokio::test]
    async fn readiness_degrades_when_backend_unavailable() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let workspace_root =
            std::env::temp_dir().join(format!("omon-test-ready-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_root).unwrap();

        struct DummyExec;
        #[async_trait::async_trait]
        impl omon_gateway::CronTaskExecutor for DummyExec {
            async fn execute(
                &self,
                _job: &omon_gateway::CronJob,
            ) -> std::result::Result<Option<String>, omon_gateway::OmonError> {
                Ok(None)
            }
        }

        let state = super::dashboard::DashboardState::new(
            pool.clone(),
            None,
            omon_gateway::CronScheduler::new(pool.clone(), std::sync::Arc::new(DummyExec)),
            omon_gateway::ToolRegistry::new(),
            omon_gateway::SmartApprovalGuard::new(),
            super::dashboard::WebDashboardDispatcher::new(),
            serde_json::json!({"appserver_url": "http://127.0.0.1:59999"}),
            workspace_root.clone(),
            vec![],
            1,
            workspace_root.clone(),
        );

        let app = super::dashboard::router(state);

        // 1. Probe health: must be HTTP 200
        let health_req = Request::builder()
            .uri("/api/health")
            .header("Host", "127.0.0.1:19744")
            .body(Body::empty())
            .unwrap();
        let health_resp = app.clone().oneshot(health_req).await.unwrap();
        assert_eq!(health_resp.status(), StatusCode::OK);

        // 2. Probe readiness: must be HTTP 503 Service Unavailable when backend connection is refused
        let ready_req = Request::builder()
            .uri("/api/ready")
            .header("Host", "127.0.0.1:19744")
            .body(Body::empty())
            .unwrap();
        let ready_resp = app.oneshot(ready_req).await.unwrap();
        assert_eq!(
            ready_resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "Readiness must degrade to 503 when backend is unreachable"
        );
        let bytes = axum::body::to_bytes(ready_resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            json["checks"]["backend"], false,
            "Checks must report backend as false"
        );

        let _ = std::fs::remove_dir_all(workspace_root);
    }

    async fn fix_main_pending_session(
        pool: &sqlx::SqlitePool,
        user: &str,
    ) -> omon_gateway::SessionKey {
        let session_key =
            omon_gateway::SessionKey::new("discord", None::<String>, "7", None::<String>, user);
        let storage_key = session_key.storage_key();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '7', ?, '{}')")
            .bind(&storage_key)
            .bind(user)
            .execute(pool)
            .await
            .unwrap();
        omon_gateway::storage::mark_session_resume_pending(pool, &storage_key)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('fix-main-turn', ?, 'user', 'unfinished task', '[]')",
        )
        .bind(&storage_key)
        .execute(pool)
        .await
        .unwrap();
        session_key
    }

    struct FixMainNoopRunner;
    #[async_trait::async_trait]
    impl omon_gateway::AgentRunner for FixMainNoopRunner {
        async fn run(
            &self,
            _session: &mut omon_gateway::SessionContext,
            _event: omon_gateway::InboundEvent,
        ) -> omon_gateway::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn fix_main_recovery_route_error_keeps_resume_pending() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let session_key = fix_main_pending_session(&pool, "42").await;

        // A draining multiplexer refuses every route() call.
        let (_drain_tx, drain_rx) = tokio::sync::watch::channel(true);
        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(FixMainNoopRunner),
            omon_gateway::MultiplexerConfig::default(),
        )
        .with_drain_receiver(drain_rx);

        let recovered = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered, 0);
        let pending = omon_gateway::storage::fetch_resume_pending_session_keys(&pool)
            .await
            .unwrap();
        assert_eq!(
            pending,
            vec![session_key],
            "route failure must keep the session resume_pending for the next boot"
        );
    }

    #[tokio::test]
    async fn fix_main_recovery_authorizes_paired_users() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        fix_main_pending_session(&pool, "42").await;
        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(FixMainNoopRunner),
            omon_gateway::MultiplexerConfig::default(),
        );
        let auth = super::StartupAuthorization {
            paired_users: vec![42],
            ..Default::default()
        };
        let recovered =
            super::recover_resume_pending_sessions_with_auth(&pool, &multiplexer, Some(&auth))
                .await
                .unwrap();
        assert_eq!(recovered, 1, "paired user's pending turn must be recovered");
    }

    #[tokio::test]
    async fn fix_r5_main_drain_wait_ignores_clearing_a_boot_time_drain() {
        // A drain already active at boot is the receiver's initial value.
        let (tx, mut rx) = tokio::sync::watch::channel(true);
        let mut wait = Box::pin(super::wait_for_drain(&mut rx));
        assert!(futures_util::poll!(wait.as_mut()).is_pending());

        // Clearing the marker (true -> false) must not end the wait.
        tx.send_replace(false);
        assert!(
            futures_util::poll!(wait.as_mut()).is_pending(),
            "cancelling a drain must not shut the gateway down"
        );

        // A new drain request (false -> true) does.
        tx.send_replace(true);
        tokio::time::timeout(std::time::Duration::from_secs(5), wait)
            .await
            .expect("a drain request must resolve the wait");
    }

    #[tokio::test]
    async fn fix_r5_main_boot_marker_from_a_real_watcher_does_not_end_the_wait() {
        let dir = tempfile::tempdir().unwrap();
        omon_gateway::drain_control::write_drain_request(dir.path(), Some("test"), false).unwrap();
        let watcher = omon_gateway::DrainWatcher::new(
            dir.path().to_path_buf(),
            std::time::Duration::from_secs(3),
        );
        assert!(watcher.scan_at("", chrono::Utc::now()));
        let mut rx = watcher.receiver();
        rx.mark_unchanged();
        let mut wait = Box::pin(super::wait_for_drain(&mut rx));
        assert!(
            futures_util::poll!(wait.as_mut()).is_pending(),
            "a marker already present at boot must not shut the gateway down"
        );
    }

    #[tokio::test]
    async fn fix_r5_main_drain_wait_stays_pending_when_sender_is_gone() {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        let mut wait = Box::pin(super::wait_for_drain(&mut rx));
        assert!(futures_util::poll!(wait.as_mut()).is_pending());
        drop(tx);
        assert!(
            futures_util::poll!(wait.as_mut()).is_pending(),
            "a dropped drain sender must leave shutdown to the other arms"
        );
    }

    #[test]
    fn fix_r5_main_recovery_event_keeps_id_origin_and_does_not_reattach() {
        let session =
            omon_gateway::SessionKey::new("discord", None::<String>, "7", None::<String>, "42");
        let id = uuid::Uuid::new_v4();
        let attachments = serde_json::to_string(&vec![omon_gateway::MessageAttachment {
            id: "a1".into(),
            filename: "main.rs".into(),
            url: "https://cdn.example/main.rs".into(),
            content_type: None,
            size_bytes: None,
            local_path: None,
            text_content: Some("fn main() {}".into()),
        }])
        .unwrap();
        let turn = |content: &str| omon_gateway::storage::UnfinishedTurn {
            message_id: id.to_string(),
            content: content.to_owned(),
            metadata_json: attachments.clone(),
            created_at: chrono::Utc::now(),
            platform_message_id: Some("p1".into()),
        };

        let user = super::recovery_event(
            &session,
            turn("look\n\n[Attachment: main.rs]"),
            Some("d1".into()),
        );
        assert_eq!(user.id, id, "the stored message id must be reused");
        assert_eq!(user.delivery_id.as_deref(), Some("d1"));
        assert_eq!(user.origin, omon_gateway::EventOrigin::User);
        assert!(
            user.attachments.is_empty(),
            "attachments are already rendered into the stored content"
        );
        assert_eq!(user.content, "look\n\n[Attachment: main.rs]");

        let system = super::recovery_event(&session, turn("[system] lane finished"), None);
        assert_eq!(system.origin, omon_gateway::EventOrigin::System);
        assert_eq!(system.content, "lane finished");
    }

    #[tokio::test]
    async fn fix_r5_main_recovery_reuses_message_id_and_ledger_delivery() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let session_key =
            omon_gateway::SessionKey::new("discord", None::<String>, "7", None::<String>, "42");
        let storage_key = session_key.storage_key();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '7', '42', '{}')")
            .bind(&storage_key)
            .execute(&pool)
            .await
            .unwrap();
        omon_gateway::storage::mark_session_resume_pending(&pool, &storage_key)
            .await
            .unwrap();
        let message_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, platform_message_id)
             VALUES (?, ?, 'user', 'unfinished task', '[]', 'p1')",
        )
        .bind(message_id.to_string())
        .bind(&storage_key)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO delivery_ledger (delivery_id,session_key,event_id,status,message_id,platform_message_id,received_at) VALUES ('d1',?,'e1','in_progress','discord:p1','p1','2020-01-01T00:00:00Z')")
            .bind(&storage_key)
            .execute(&pool)
            .await
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        struct Recorder(tokio::sync::mpsc::UnboundedSender<omon_gateway::InboundEvent>);
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for Recorder {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                let _ = self.0.send(event);
                Ok(())
            }
        }
        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(Recorder(tx)),
            omon_gateway::MultiplexerConfig::default(),
        );

        let recovered = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered, 1);

        let event = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
            .await
            .expect("recovered turn must reach the runner")
            .unwrap();
        assert_eq!(event.id, message_id);
        assert_eq!(event.delivery_id.as_deref(), Some("discord:p1"));
        // The actor persists the inbound row before running the turn; the reused
        // id must hit ON CONFLICT instead of adding a second user row.
        let user_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE session_key = ? AND role = 'user'",
        )
        .bind(&storage_key)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(user_rows, 1, "recovery must not duplicate the user row");
    }

    #[test]
    fn fix_r5_main_unknown_boolean_tokens_keep_the_default() {
        for typo in ["enabled", "y", "t", "ture"] {
            assert!(super::parse_bool_from(Some(typo), true), "{typo}");
            assert!(!super::parse_bool_from(Some(typo), false), "{typo}");
        }
    }

    #[test]
    fn fix_r5_main_user_allowlist_must_parse() {
        assert!(super::parse_allowlist("DISCORD_ALLOWED_USERS", Some("<@123>")).is_err());
        assert!(super::parse_allowlist("DISCORD_ALLOWED_ROLES", Some("admins")).is_err());
        assert_eq!(
            super::parse_allowlist("DISCORD_ALLOWED_USERS", Some("1, 2")).unwrap(),
            vec![1, 2]
        );
    }

    #[test]
    fn fix_r5_main_cron_llm_override_never_carries_the_base_key_elsewhere() {
        let mut base = omon_gateway::LlmConfig::new(omon_gateway::LlmProvider::OpenAi, "gpt-4o");
        base.api_key = Some("base-secret".into());

        let model_only = super::build_cron_llm_config(&base, None, None, Some("gpt-4o-mini"));
        assert_eq!(model_only.api_key.as_deref(), Some("base-secret"));
        let same_provider = super::build_cron_llm_config(&base, Some("openai"), None, None);
        assert_eq!(same_provider.api_key.as_deref(), Some("base-secret"));

        let other_provider = super::build_cron_llm_config(&base, Some("anthropic"), None, None);
        assert_eq!(other_provider.api_key, None);
        let other_host =
            super::build_cron_llm_config(&base, None, Some("https://attacker.example/v1"), None);
        assert_eq!(other_host.api_key, None);
    }

    #[test]
    fn fix_r5_main_owner_and_mail_account_are_configurable() {
        assert_eq!(
            super::owner_id_from(None),
            omon_gateway::inbox::proposals::OWNER_ID
        );
        assert_eq!(
            super::owner_id_from(Some("  ")),
            omon_gateway::inbox::proposals::OWNER_ID
        );
        assert_eq!(super::owner_id_from(Some(" 123 ")), "123");
        assert_eq!(super::mail_account_from(Some("a@b.c")), "a@b.c");
        assert_eq!(super::mail_account_from(None), "freedomzero91@gmail.com");
    }

    #[test]
    fn fix_r5_main_full_disk_access_prompt_is_claimed_once() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join(super::FDA_PROMPTED_MARKER);
        assert!(super::claim_fda_prompt(&marker));
        assert!(
            !super::claim_fda_prompt(&marker),
            "a restart must not reopen the privacy pane"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fix_r5_main_watcher_restarts_with_bounded_backoff() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = runs.clone();
        let started = tokio::time::Instant::now();
        super::supervise_watcher("test", shutdown_rx, move |_| {
            let counter = counter.clone();
            async move {
                if counter.fetch_add(1, Ordering::SeqCst) < 2 {
                    super::WatcherExit::Failed {
                        stderr_tail: String::new(),
                    }
                } else {
                    super::WatcherExit::Stop
                }
            }
        })
        .await;
        assert_eq!(runs.load(Ordering::SeqCst), 3, "failures must be retried");
        assert_eq!(
            started.elapsed(),
            super::WATCHER_BACKOFF_INITIAL * 3,
            "backoff doubles between consecutive failures (5s + 10s)"
        );
        assert_eq!(
            super::next_watcher_backoff(super::WATCHER_BACKOFF_MAX),
            super::WATCHER_BACKOFF_MAX
        );
    }

    #[tokio::test(start_paused = true)]
    async fn fix_r5_main_watcher_backoff_wakes_on_shutdown() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let shutdown_tx = Arc::new(shutdown_tx);
        let runs = Arc::new(AtomicUsize::new(0));
        let counter = runs.clone();
        super::supervise_watcher("test", shutdown_rx, move |_| {
            let counter = counter.clone();
            let shutdown_tx = shutdown_tx.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                shutdown_tx.send_replace(true);
                super::WatcherExit::Failed {
                    stderr_tail: String::new(),
                }
            }
        })
        .await;
        assert_eq!(runs.load(Ordering::SeqCst), 1, "no restart after shutdown");
    }

    #[test]
    fn startup_recovery_exempts_dms_from_channel_allowlist() {
        let auth = super::StartupAuthorization {
            allowed_users: vec![42],
            allowed_channels: vec![111],
            ..Default::default()
        };
        let dm =
            omon_gateway::SessionKey::new("discord", None::<String>, "999", None::<String>, "42");
        assert!(
            auth.is_authorized(&dm),
            "a DM must not be rejected by the guild channel allow-list"
        );
        let other_guild_channel =
            omon_gateway::SessionKey::new("discord", Some("g"), "999", None::<String>, "42");
        assert!(
            !auth.is_authorized(&other_guild_channel),
            "a guild channel outside the allow-list stays rejected"
        );
        let ignored = super::StartupAuthorization {
            allowed_users: vec![42],
            ignored_channels: vec![999],
            ..Default::default()
        };
        assert!(!ignored.is_authorized(&dm), "ignored channels still apply");
    }

    #[tokio::test]
    async fn fix_main_panicked_discord_client_counts_as_failure() {
        async fn boom() -> std::result::Result<(), String> {
            panic!("discord client panicked")
        }
        let mut join_set = tokio::task::JoinSet::new();
        join_set.spawn(boom());
        let res = join_set.join_next().await.unwrap();
        assert!(res.is_err(), "precondition: task must end with JoinError");
        assert!(super::discord_client_failed(&res));
        let ok: std::result::Result<std::result::Result<(), String>, tokio::task::JoinError> =
            Ok(Ok(()));
        assert!(!super::discord_client_failed(&ok));
        let err: std::result::Result<std::result::Result<(), String>, tokio::task::JoinError> =
            Ok(Err("boom".into()));
        assert!(super::discord_client_failed(&err));
    }

    #[tokio::test]
    async fn fix_share_combined_dashboard_resolves_gateway_approvals() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use omon_gateway::{ApprovalRequester, OutboundAction, OutboundDispatcher};
        use std::sync::Arc;
        use tower::ServiceExt;

        struct Primary(tokio::sync::mpsc::UnboundedSender<OutboundAction>);
        #[async_trait::async_trait]
        impl OutboundDispatcher for Primary {
            async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
                let _ = self.0.send(action);
                Ok(())
            }
        }
        struct NoopCron;
        #[async_trait::async_trait]
        impl omon_gateway::CronTaskExecutor for NoopCron {
            async fn execute(
                &self,
                _job: &omon_gateway::CronJob,
            ) -> omon_gateway::Result<Option<String>> {
                Ok(None)
            }
        }

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let workspace_root =
            std::env::temp_dir().join(format!("omon-test-share-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_root).unwrap();

        // Gateway-side runtime, wired the way run_gateway wires it.
        let (primary_tx, mut primary_rx) = tokio::sync::mpsc::unbounded_channel();
        let primary: Arc<dyn OutboundDispatcher> = Arc::new(Primary(primary_tx));
        let guard = omon_gateway::SmartApprovalGuard::new();
        let requester = Arc::new(omon_gateway::DiscordApprovalRequester::new(
            guard,
            std::time::Duration::from_secs(30),
        ));
        requester.set_dispatcher(primary.clone()).await;
        let shared_dispatcher = super::SharedDispatcher::default();
        shared_dispatcher.set(primary.clone()).await;
        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            Arc::new(FixMainNoopRunner),
            omon_gateway::MultiplexerConfig::default(),
        );
        let scheduler = omon_gateway::CronScheduler::new(pool.clone(), Arc::new(NoopCron));
        let settings = super::dashboard::DashboardSettings {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 19744,
            insecure: false,
            web_root: workspace_root.clone(),
        };

        let state = super::attach_gateway_dashboard(
            &settings,
            &requester,
            &shared_dispatcher,
            primary,
            pool.clone(),
            multiplexer,
            scheduler,
            omon_gateway::ToolRegistry::new(),
            serde_json::json!({}),
            workspace_root.clone(),
            vec![],
            1,
        )
        .await;
        let detached = state.detached_from_gateway;
        let app = super::dashboard::router(state);

        // An approval raised through the gateway's own requester/guard.
        let session =
            omon_gateway::SessionKey::new("discord", Some("bot1"), "123", None::<String>, "42");
        let pending_requester = requester.clone();
        let decision = tokio::spawn(async move {
            pending_requester
                .request_approval(&session, "rm -rf build", "cleanup")
                .await
        });
        let delivered = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                match primary_rx.recv().await {
                    Some(OutboundAction::ApprovalRequest { request_id, .. }) => break request_id,
                    Some(_) => continue,
                    None => panic!("primary dispatcher closed"),
                }
            }
        })
        .await
        .expect("approval must reach the Discord egress");

        let pending = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/approvals/pending")
                    .header("Host", "127.0.0.1:19744")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pending.status(), StatusCode::OK);
        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(pending.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        println!("FIX-SHARE pending={body}");
        let listed = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == serde_json::json!(delivered));
        assert!(listed, "gateway approval must be listed by the dashboard");

        let resolved = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/approvals/{delivered}/resolve"))
                    .header("Host", "127.0.0.1:19744")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"decision": "once"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resolved.status(), StatusCode::OK);
        let decision = tokio::time::timeout(std::time::Duration::from_secs(5), decision)
            .await
            .expect("requester must observe the dashboard decision")
            .unwrap();
        assert!(matches!(decision, Ok(omon_gateway::ApprovalDecision::Once)));
        assert!(!detached, "combined dashboard shares the gateway runtime");
        let _ = std::fs::remove_dir_all(workspace_root);
    }

    #[test]
    fn fix_main_allowlist_rejects_all_invalid_entries() {
        assert!(super::parse_allowlist("DISCORD_ALLOWED_CHANNELS", Some("abc, chan-x")).is_err());
        assert_eq!(
            super::parse_allowlist("DISCORD_ALLOWED_CHANNELS", Some("1, bad, 2")).unwrap(),
            vec![1, 2]
        );
        assert!(super::parse_allowlist("DISCORD_ALLOWED_CHANNELS", None)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn recovery_rechecks_current_owner_authorization() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // User 42 in channel 7
        let session_key =
            omon_gateway::SessionKey::new("discord", Some("bot1"), "7", None::<String>, "42");
        let storage_key = session_key.storage_key();

        // Setup session and unfinished turn in DB
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '7', '42', '{}')")
            .bind(&storage_key)
            .execute(&pool)
            .await
            .unwrap();
        omon_gateway::storage::mark_session_resume_pending(&pool, &storage_key)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('turn1', ?, 'user', 'unfinished task', '[]')",
        )
        .bind(&storage_key)
        .execute(&pool)
        .await
        .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        struct TestRunner(tokio::sync::mpsc::UnboundedSender<String>);
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for TestRunner {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                let _ = self.0.send(event.content);
                Ok(())
            }
        }

        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(TestRunner(tx)),
            omon_gateway::MultiplexerConfig::default(),
        );

        // Current authorization: user 42 is REVOKED (only user 99 is allowed)
        let auth = super::StartupAuthorization {
            allowed_users: vec![99],
            allow_all_users: false,
            allowed_channels: vec![],
            ignored_channels: vec![],
            paired_users: vec![],
        };

        let recovered =
            super::recover_resume_pending_sessions_with_auth(&pool, &multiplexer, Some(&auth))
                .await
                .unwrap();
        assert_eq!(recovered, 0, "Revoked user session must NOT be recovered");
        assert!(
            rx.try_recv().is_err(),
            "No event should be dispatched for revoked user"
        );
    }

    #[test]
    fn cron_and_interactive_share_one_daemon() {
        let _env = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::remove_var("OMON_OMO_APPSERVER_URL");
            std::env::remove_var("OMON_OMO_CRON_APPSERVER_URL");
        }

        let interactive = omon_gateway::agent::OmoBackendConfig::from_env().unwrap();
        let cron = omon_gateway::agent::OmoBackendConfig::cron_from_env().unwrap();

        assert_eq!(interactive.appserver_url, "ws://127.0.0.1:19742");
        assert_eq!(
            cron.appserver_url, interactive.appserver_url,
            "Cron and interactive must share one daemon endpoint"
        );
        assert!(
            cron.total_timeout < interactive.total_timeout,
            "Cron retains dedicated per-turn timeout policy"
        );
    }

    #[tokio::test]
    async fn drain_is_reversible_and_preserves_accepted_work() {
        use omon_gateway::{
            write_drain_request, AgentRunner, DrainWatcher, InboundEvent, MultiplexerConfig,
            OmonError, SessionContext, SessionKey, SessionMultiplexer,
        };
        use std::sync::Arc;

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let state_dir =
            std::env::temp_dir().join(format!("omon-test-drain-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&state_dir).unwrap();

        let watcher = DrainWatcher::new(state_dir.clone(), std::time::Duration::from_millis(10));
        let epoch = omon_gateway::current_instantiation_epoch().to_string();

        let (processed_tx, mut processed_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        struct CountingRunner(tokio::sync::mpsc::UnboundedSender<String>);
        #[async_trait::async_trait]
        impl AgentRunner for CountingRunner {
            async fn run(
                &self,
                _session: &mut SessionContext,
                event: InboundEvent,
            ) -> Result<(), OmonError> {
                let _ = self.0.send(event.content);
                Ok(())
            }
        }

        let mux = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(CountingRunner(processed_tx)),
            MultiplexerConfig::default(),
        )
        .with_drain_receiver(watcher.receiver());

        let session = SessionKey::new("discord", Some("bot1"), "123", None::<String>, "42");
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '123', '42', '{}')")
            .bind(session.storage_key())
            .execute(&pool)
            .await
            .unwrap();

        let m1 = InboundEvent::message(session.clone(), "m1", "turn 1");
        let m2 = InboundEvent::message(session.clone(), "m2", "turn 2");
        mux.route(m1).await.unwrap();
        mux.route(m2).await.unwrap();

        // Drain marker placed
        write_drain_request(&state_dir, Some("operator"), false).unwrap();
        watcher.scan_at(&epoch, chrono::Utc::now());

        // Work accepted before the drain still runs to completion.
        for expected in ["turn 1", "turn 2"] {
            let got = tokio::time::timeout(std::time::Duration::from_secs(10), processed_rx.recv())
                .await
                .expect("accepted turn must be processed while draining")
                .unwrap();
            assert_eq!(got, expected);
        }

        // m3 must be rejected because gateway is draining
        let m3 = InboundEvent::message(session.clone(), "m3", "turn 3");
        let err = mux.route(m3).await.unwrap_err();
        assert!(err.to_string().contains("gateway is draining"));

        // Drain marker removed
        std::fs::remove_file(state_dir.join(".drain_request.json")).unwrap();
        watcher.scan_at(&epoch, chrono::Utc::now());

        // m4 must be accepted after drain is reversed
        let m4 = InboundEvent::message(session.clone(), "m4", "turn 4");
        let res4 = mux.route(m4).await;
        assert!(res4.is_ok(), "m4 must be accepted after drain is reversed");
        let got = tokio::time::timeout(std::time::Duration::from_secs(10), processed_rx.recv())
            .await
            .expect("m4 must be processed after drain is reversed")
            .unwrap();
        assert_eq!(got, "turn 4");

        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[test]
    fn resolves_cron_script_timeout_with_overrides_and_fallbacks() {
        use std::time::Duration;

        // Job override takes precedence over global default
        assert_eq!(
            resolve_cron_script_timeout(Some(300), 1800),
            Duration::from_secs(300)
        );
        // None falls back to global default
        assert_eq!(
            resolve_cron_script_timeout(None, 2400),
            Duration::from_secs(2400)
        );
        // Zero override falls back to global default
        assert_eq!(
            resolve_cron_script_timeout(Some(0), 1800),
            Duration::from_secs(1800)
        );
        // None with zero global default falls back to DEFAULT_CRON_SCRIPT_TIMEOUT_SECS
        assert_eq!(
            resolve_cron_script_timeout(None, 0),
            Duration::from_secs(DEFAULT_CRON_SCRIPT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn parses_approval_timeout_secs_from_env() {
        assert_eq!(approval_timeout_secs_from(Some("120")), 120);
        assert_eq!(approval_timeout_secs_from(Some(" 300 ")), 300);
        assert_eq!(approval_timeout_secs_from(None), 900);
        assert_eq!(approval_timeout_secs_from(Some("")), 900);
        assert_eq!(approval_timeout_secs_from(Some("   ")), 900);
        assert_eq!(approval_timeout_secs_from(Some("0")), 900);
        assert_eq!(approval_timeout_secs_from(Some("-10")), 900);
        assert_eq!(approval_timeout_secs_from(Some("invalid")), 900);
    }

    #[test]
    fn parses_bool_from_env_variants() {
        assert!(super::parse_bool_from(Some("true"), false));
        assert!(super::parse_bool_from(Some("True"), false));
        assert!(super::parse_bool_from(Some("1"), false));
        assert!(super::parse_bool_from(Some("yes"), false));
        assert!(super::parse_bool_from(Some("on"), false));
        assert!(!super::parse_bool_from(Some("false"), true));
        assert!(!super::parse_bool_from(Some("0"), true));
        assert!(!super::parse_bool_from(Some("no"), true));
        assert!(!super::parse_bool_from(Some("off"), true));
        assert!(!super::parse_bool_from(Some(""), false));
        assert!(super::parse_bool_from(None, true));
        assert!(!super::parse_bool_from(None, false));
    }

    #[test]
    fn hermes_skill_dirs_use_documented_roots() {
        let dirs = hermes_skill_dirs(std::path::Path::new("/x"), std::path::Path::new("/h"));

        assert_eq!(
            dirs,
            vec![
                std::path::PathBuf::from("/x/skills"),
                std::path::PathBuf::from("/h/.omon/skills"),
            ]
        );
        assert!(dirs
            .iter()
            .all(|path| !path.to_string_lossy().contains("workspace/.hermes")));
    }

    #[test]
    fn parses_extra_tool_roots_colon_separated() {
        let raw = Some("/Users/test/docs:/Users/test/code");
        let parsed = raw
            .map(|val| {
                val.split(':')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(std::path::PathBuf::from)
                    .collect::<Vec<_>>()
            })
            .filter(|roots| !roots.is_empty())
            .unwrap_or_default();

        assert_eq!(
            parsed,
            vec![
                std::path::PathBuf::from("/Users/test/docs"),
                std::path::PathBuf::from("/Users/test/code")
            ]
        );
    }

    #[test]
    fn cli_defaults_to_run_without_a_subcommand() {
        let cli = Cli::try_parse_from(["omo-gateway"]).unwrap();
        assert!(matches!(cli.into_command(), Command::Run));
    }

    #[test]
    fn cli_maps_explicit_run_to_the_run_path() {
        let cli = Cli::try_parse_from(["omo-gateway", "run"]).unwrap();
        assert!(matches!(cli.into_command(), Command::Run));
    }

    #[test]
    fn cli_parses_migrate_flags() {
        let cli =
            Cli::try_parse_from(["omo-gateway", "migrate", "--dry-run", "--no-cutover"]).unwrap();
        match cli.into_command() {
            Command::Migrate(args) => {
                assert!(args.dry_run);
                assert!(args.no_cutover);
            }
            command => panic!("expected migrate command, got {command:?}"),
        }
    }

    #[test]
    fn cli_rejects_unknown_subcommands() {
        assert!(Cli::try_parse_from(["omo-gateway", "bogus"]).is_err());
    }

    #[test]
    fn maps_hermes_web_toolset_to_both_web_tools() {
        let enabled = vec!["web".to_string()];
        assert!(tool_enabled("web_search", Some(&enabled)));
        assert!(tool_enabled("web_fetch", Some(&enabled)));
        assert!(!tool_enabled("terminal", Some(&enabled)));
    }

    #[test]
    fn rejects_cron_workdir_outside_authorized_roots() {
        let base = std::env::temp_dir().join(format!("omon-cron-roots-{}", uuid::Uuid::new_v4()));
        let workspace = base.join("workspace");
        let hermes = base.join("hermes");
        let outside = base.join("outside");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&hermes).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let roots = vec![
            fs::canonicalize(&workspace).unwrap(),
            fs::canonicalize(&hermes).unwrap(),
        ];

        assert!(canonical_authorized_directory(&workspace, &roots, "workdir").is_ok());
        assert!(canonical_authorized_directory(&hermes, &roots, "workdir").is_ok());
        assert!(canonical_authorized_directory(&outside, &roots, "workdir").is_err());

        let _ = fs::remove_dir_all(base);
    }

    #[derive(Default)]
    struct CapturingDispatcher {
        actions: tokio::sync::Mutex<Vec<omon_gateway::OutboundAction>>,
    }

    #[async_trait::async_trait]
    impl omon_gateway::OutboundDispatcher for CapturingDispatcher {
        async fn dispatch(&self, action: omon_gateway::OutboundAction) -> omon_gateway::Result<()> {
            self.actions.lock().await.push(action);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_recover_pending_delivery_obligations_redispatches_dead_process_rows() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        let dispatcher = std::sync::Arc::new(CapturingDispatcher::default());
        let dead_pid = 999_999_i64;

        let session_key = omon_gateway::SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-recover",
            None::<String>,
            "user-recover",
        );
        super::ensure_agent_session(
            &pool,
            &omon_gateway::SessionContext::new(session_key.clone()),
        )
        .await
        .unwrap();

        let ledger = omon_gateway::ledger::DeliveryLedgerService::new(pool.clone());
        // 1. Pending obligation from dead process
        let _ = ledger
            .record_obligation("obl-rec-pending", &session_key, "first dead text")
            .await;
        sqlx::query("UPDATE delivery_obligations SET owner_pid = ? WHERE id = 'obl-rec-pending'")
            .bind(dead_pid)
            .execute(&pool)
            .await
            .unwrap();

        // 2. Attempting obligation from dead process (crashed mid-send)
        let _ = ledger
            .record_obligation("obl-rec-attempting", &session_key, "second dead text")
            .await;
        sqlx::query("UPDATE delivery_obligations SET state = 'attempting', owner_pid = ? WHERE id = 'obl-rec-attempting'")
            .bind(dead_pid)
            .execute(&pool)
            .await
            .unwrap();

        let recovered_count =
            super::recover_pending_delivery_obligations(&pool, dispatcher.clone())
                .await
                .unwrap();
        assert_eq!(recovered_count, 2);

        // Verify actions dispatched
        let actions = dispatcher.actions.lock().await.clone();
        assert_eq!(actions.len(), 2);

        let contents: Vec<String> = actions
            .iter()
            .map(|a| match a {
                omon_gateway::OutboundAction::Stream { chunk, .. } => chunk.content.clone(),
                _ => String::new(),
            })
            .collect();

        // Pending obligation should NOT have duplicate marker
        assert_eq!(contents[0], "first dead text");
        // Attempting obligation SHOULD have the recovered duplicate marker
        assert!(contents[1].contains("♻️ Recovered reply"));
        assert!(contents[1].contains("second dead text"));

        // Both obligations should now be marked 'delivered' in the database
        let obl1: omon_gateway::ledger::DeliveryObligation = ledger
            .get_obligation("obl-rec-pending")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(obl1.state, "delivered");
        let obl2: omon_gateway::ledger::DeliveryObligation = ledger
            .get_obligation("obl-rec-attempting")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(obl2.state, "delivered");
    }

    #[tokio::test]
    async fn test_recover_pending_delivery_obligations_preserves_bot_identity() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        let dispatcher = std::sync::Arc::new(CapturingDispatcher::default());
        let dead_pid = 999_999_i64;

        let session_key = omon_gateway::SessionKey::new(
            "discord",
            None::<String>,
            "chan-recover-bot",
            None::<String>,
            "user-recover-bot",
        )
        .with_bot_id("42");
        super::ensure_agent_session(
            &pool,
            &omon_gateway::SessionContext::new(session_key.clone()),
        )
        .await
        .unwrap();

        let ledger = omon_gateway::ledger::DeliveryLedgerService::new(pool.clone());
        let _ = ledger
            .record_obligation("obl-rec-bot", &session_key, "bot response text")
            .await;
        sqlx::query("UPDATE delivery_obligations SET owner_pid = ? WHERE id = 'obl-rec-bot'")
            .bind(dead_pid)
            .execute(&pool)
            .await
            .unwrap();

        let recovered_count =
            super::recover_pending_delivery_obligations(&pool, dispatcher.clone())
                .await
                .unwrap();
        assert_eq!(recovered_count, 1);

        let actions = dispatcher.actions.lock().await.clone();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            omon_gateway::OutboundAction::Stream { session, chunk } => {
                assert_eq!(session.bot_id.as_deref(), Some("42"));
                assert_eq!(session.channel_id, "chan-recover-bot");
                assert_eq!(chunk.content, "bot response text");
            }
            _ => panic!("expected Stream action"),
        }
    }

    #[cfg(test)]
    mod ag07_upgrade_tests {
        use std::collections::BTreeMap;
        use std::panic::AssertUnwindSafe;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        use futures_util::FutureExt;
        use omon_gateway::{InboundEvent, SessionContext, SessionKey, SessionMultiplexer};
        use serde_json::{json, Value};
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use sqlx::SqlitePool;
        use tokio::sync::mpsc;

        const LIMIT: Duration = Duration::from_secs(20);
        const KEYS: [&str; 7] = [
            "7:discord|3:100|3:200|-|2:42|2:84",
            "7:discord|3:100|3:200|-|2:43|2:84",
            "7:discord|3:100|3:200|-|0:|2:84",
            "7:discord|3:100|3:201|-|2:42|2:84",
            "7:discord|3:100|3:201|-|2:43|2:84",
            "7:discord|3:100|3:202|-|2:42|2:84",
            "7:discord|3:100|3:203|-|0:|2:84",
        ];
        const CANONICAL: &str = "7:discord|3:100|3:202|-|0:|2:84";
        type Snapshot = BTreeMap<String, Vec<Value>>;

        // Capture every column, including IDs, sequence numbers, timestamps and opaque
        // serialized payload bytes. These are real tables, not mocked lookup results.
        async fn snapshot(pool: &SqlitePool) -> Snapshot {
            let mut result = BTreeMap::new();
            for table in [
                "sessions",
                "messages",
                "delivery_ledger",
                "cron_jobs",
                "memories",
                "delivery_obligations",
                "pending_writes",
                "message_search_documents",
                "delivery_ledger_constituents",
            ] {
                let columns: Vec<String> =
                    sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                        .bind(table)
                        .fetch_all(pool)
                        .await
                        .unwrap();
                let fields = columns
                    .iter()
                    .map(|c| format!("'{c}',\"{c}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                let rows: Vec<String> = sqlx::query_scalar(&format!(
                    "SELECT json_object({fields}) FROM {table} ORDER BY rowid",
                ))
                .fetch_all(pool)
                .await
                .unwrap();
                result.insert(
                    table.to_owned(),
                    rows.into_iter()
                        .map(|s| serde_json::from_str(&s).unwrap())
                        .collect(),
                );
            }
            result
        }

        #[allow(clippy::large_enum_variant)]
        enum Observed {
            Run(SessionContext, InboundEvent),
            Finished,
        }
        struct Recorder {
            tx: mpsc::UnboundedSender<Observed>,
            calls: Arc<AtomicUsize>,
        }
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for Recorder {
            async fn run(
                &self,
                session: &mut SessionContext,
                event: InboundEvent,
            ) -> omon_gateway::Result<()> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.tx
                    .send(Observed::Run(session.clone(), event))
                    .map_err(|e| omon_gateway::OmonError::Multiplexer(e.to_string()))?;
                Ok(())
            }
        }
        #[async_trait::async_trait]
        impl omon_gateway::OutboundDispatcher for Recorder {
            async fn dispatch(
                &self,
                action: omon_gateway::OutboundAction,
            ) -> omon_gateway::Result<()> {
                if matches!(
                    action,
                    omon_gateway::OutboundAction::Typing { active: false, .. }
                ) {
                    self.tx
                        .send(Observed::Finished)
                        .map_err(|e| omon_gateway::OmonError::Multiplexer(e.to_string()))?;
                }
                Ok(())
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Authorization {
            AllowAll,
            ExplicitAllowlist,
            Deferral,
        }

        #[test]
        fn ag07_collisions_defer_only_ambiguous_lanes() {
            exercise_upgrade(false, Authorization::AllowAll);
        }

        #[test]
        fn ag07_upgrade_preserves_references_and_rolls_back() {
            exercise_upgrade(true, Authorization::AllowAll);
        }

        #[test]
        fn ag07_explicit_allowlist_uses_exact_pending_actor() {
            exercise_upgrade(false, Authorization::ExplicitAllowlist);
        }

        #[test]
        fn ag07_unknown_or_unauthorized_actor_retains_pending() {
            exercise_upgrade(false, Authorization::Deferral);
        }

        fn exercise_upgrade(inject_failure: bool, authorization: Authorization) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("legacy.sqlite");
            let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let (outcome, actors_closed, pool_closed, calls) = runtime.block_on(async {
                let mut pool: Option<SqlitePool> = None;
                let mut mux: Option<SessionMultiplexer> = None;
                let (tx, mut rx) = mpsc::unbounded_channel();
                let mut tx = Some(tx);
                let calls = Arc::new(AtomicUsize::new(0));
                let outcome = AssertUnwindSafe(async {
                    tokio::time::timeout(LIMIT, async {
                        pool = Some(SqlitePoolOptions::new().max_connections(1).connect_with(
                            SqliteConnectOptions::new().filename(&path).create_if_missing(true).foreign_keys(true),
                        ).await.unwrap());
                        let old = pool.as_ref().unwrap();
                        let shipped = sqlx::migrate!("./migrations");
                        let old_schema = sqlx::migrate::Migrator {
                            migrations: std::borrow::Cow::Owned(shipped.iter().filter(|m| m.version <= 27).cloned().collect()),
                            ..sqlx::migrate::Migrator::DEFAULT
                        };
                        old_schema.run(old).await.unwrap();
                        // The pre-upgrade runtime creates this non-FK constituent table.
                        sqlx::query("CREATE TABLE delivery_ledger_constituents (parent_delivery_id TEXT NOT NULL, constituent_id TEXT NOT NULL PRIMARY KEY)")
                            .execute(old).await.unwrap();
                        for (i, raw) in KEYS.iter().enumerate() {
                            let key = SessionKey::from_storage_key(raw).unwrap();
                            let user = if i == 1 || i == 4 { "43" } else if i == 2 || i == 6 { "" } else { "42" };
                            let state = json!({"active_model":"fixture-model", "system_prompt":"fixture-persona", "enabled_toolsets":["memory"],
                                "metadata":{"omo_thread_id":format!("remote-{i}"),"checkpoint":{"literal_raw":raw}}});
                            sqlx::query("INSERT INTO sessions (session_key,platform,guild_id,channel_id,user_id,state_json,resume_pending,created_at,updated_at) VALUES (?,'discord','100',?,?,?,?, '2020-01-01T00:00:00Z','2020-01-01T00:00:00Z')")
                                .bind(raw).bind(&key.channel_id).bind(user).bind(state.to_string()).bind(if i == 1 { 0 } else { 1 })
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO messages (id,session_key,role,content,metadata_json,created_at,platform_message_id) VALUES (?,?,'user',?,?,'2020-01-01T00:00:00Z',?)")
                                .bind(format!("00000000-0000-4000-8000-{i:012}"))
                                .bind(raw).bind(format!("work {raw}")).bind("[]").bind(format!("p{i}"))
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO delivery_ledger (delivery_id,session_key,event_id,status,message_id,platform_message_id,received_at) VALUES (?,?,?,'pending',?,?,'2020-01-01T00:00:00Z')")
                                .bind(format!("d{i}")).bind(raw).bind(format!("e{i}")).bind(format!("discord:p{i}")).bind(format!("p{i}"))
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO delivery_ledger_constituents VALUES (?,?)")
                                .bind(format!("discord:p{i}")).bind(format!("part{i}")).execute(old).await.unwrap();
                            sqlx::query("INSERT INTO cron_jobs (id,session_key,expression,payload_json,enabled) VALUES (?,?,'0 * * * * *',?,0)")
                                .bind(format!("c{i}")).bind(raw).bind(json!({"prompt":raw,"checkpoint":{"raw":raw}}).to_string())
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO memories (id,session_key,content,metadata_json) VALUES (?,?,?,?)")
                                .bind(format!("m{i}")).bind(raw).bind(raw).bind(json!({"raw":raw}).to_string())
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO delivery_obligations (id,session_key,channel_id,content,state) VALUES (?,?,?,?,'pending')")
                                .bind(format!("o{i}")).bind(raw).bind(&key.channel_id).bind(json!({"text":raw,"checkpoint":{"raw":raw}}).to_string())
                                .execute(old).await.unwrap();
                            sqlx::query("INSERT INTO pending_writes (id,kind,payload) VALUES (?,'memory',?)")
                                .bind(format!("w{i}")).bind(json!({"session_key":raw,"content":raw,"metadata":{"original_key":raw}}).to_string())
                                .execute(old).await.unwrap();
                        }
                        sqlx::query("INSERT INTO pending_writes (id,kind,payload) VALUES ('skill','skill',?),('invalid','memory','not-json')")
                            .bind(json!({"session_key":KEYS[5],"content":KEYS[5]}).to_string()).execute(old).await.unwrap();
                        if authorization != Authorization::AllowAll {
                            // Real search storage, with the provenance emitted by Discord REST.
                            // The transcript trigger's historical user_id is not such evidence.
                            let index = omon_gateway::MessageSearchIndex::new(old.clone());
                            let mut document = index.get("discord", "203", "p6").await.unwrap().unwrap();
                            document.author_id = "42".into();
                            document.metadata = json!({"source":"discord_rest", "author_is_bot":false});
                            index.upsert(&document).await.unwrap();
                        }
                        let before = snapshot(old).await;
                        eprintln!("AG-07 upgrade pre-existing raw_keys={KEYS:?}; inject_failure={inject_failure}");
                        if inject_failure {
                            sqlx::query("CREATE TRIGGER ag07_reject_rekey BEFORE UPDATE OF session_key ON memories BEGIN SELECT RAISE(ABORT,'ag07-rekey-fault'); END")
                                .execute(old).await.unwrap();
                        }
                        pool.take().unwrap().close().await;
                        if inject_failure {
                            match omon_gateway::storage::init_pool(&url).await {
                                Ok(opened) => { opened.close().await; panic!("injected rekey fault must reject the upgrade"); }
                                Err(error) => assert!(error.to_string().contains("ag07-rekey-fault"), "unexpected upgrade error: {error}"),
                            }
                            // Inspect the failed upgrade without running it again. All parent and
                            // dependent rows must be exactly the pre-upgrade snapshot.
                            pool = Some(SqlitePoolOptions::new().max_connections(1).connect_with(
                                SqliteConnectOptions::new().filename(&path).foreign_keys(true),
                            ).await.unwrap());
                            let failed = pool.as_ref().unwrap();
                            assert_eq!(snapshot(failed).await, before, "partial rekey escaped rollback");
                            let violations: i64 = sqlx::query_scalar("SELECT count(*) FROM pragma_foreign_key_check")
                                .fetch_one(failed).await.unwrap();
                            assert_eq!(violations, 0);
                            let deferred: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_guild_session_conflicts")
                                .fetch_one(failed).await.unwrap();
                            assert_eq!(deferred, 0, "quarantine writes escaped the failed transaction");
                            let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_guild_pending_auth")
                                .fetch_one(failed).await.unwrap();
                            assert_eq!(grants, 0, "pending-author grants escaped the failed transaction");
                            sqlx::query("DROP TRIGGER ag07_reject_rekey").execute(failed).await.unwrap();
                            pool.take().unwrap().close().await;
                        }
                        pool = Some(omon_gateway::storage::init_pool(&url).await.unwrap());
                        let mut expected = before.clone();
                        for (table, rows) in &mut expected {
                            for row in rows {
                                if row.get("session_key").and_then(Value::as_str) == Some(KEYS[5]) {
                                    row["session_key"] = CANONICAL.into();
                                }
                                if table == "pending_writes" && row["id"] == "w5" {
                                    let mut payload: Value = serde_json::from_str(row["payload"].as_str().unwrap()).unwrap();
                                    payload["session_key"] = CANONICAL.into();
                                    row["payload"] = payload.to_string().into();
                                }
                            }
                        }
                        let upgraded = snapshot(pool.as_ref().unwrap()).await;
                        pool.take().unwrap().close().await;
                        pool = Some(omon_gateway::storage::init_pool(&url).await.unwrap());
                        assert_eq!(snapshot(pool.as_ref().unwrap()).await, upgraded, "reopening repeated or changed the upgrade");
                        // Replay first: pre-repair RED must be wrong-lane behavior, not a query
                        // against the not-yet-shipped quarantine table.
                        let current = pool.as_ref().unwrap();
                        let probe = Arc::new(Recorder { tx: tx.take().unwrap(), calls: calls.clone() });
                        mux = Some(SessionMultiplexer::with_dispatcher(current.clone(), probe.clone(), Some(probe), Default::default()));
                        let current_mux = mux.as_ref().unwrap();
                        let auth = if authorization == Authorization::AllowAll {
                            super::super::StartupAuthorization { allow_all_users: true, ..Default::default() }
                        } else {
                            super::super::StartupAuthorization { allowed_users: vec![42], allow_all_users: false,
                                allowed_channels: vec![202,203], ignored_channels: vec![], paired_users: vec![] }
                        };
                        if authorization == Authorization::Deferral {
                            for allowed_users in [vec![99], vec![]] {
                                let denied = super::super::StartupAuthorization { allowed_users, ..auth.clone() };
                                let recovered = super::super::recover_resume_pending_sessions_with_auth(current,current_mux,Some(&denied)).await.unwrap();
                                eprintln!("AG-07 revoked actor recovery recovered={recovered}; allowed_users={:?}",denied.allowed_users);
                                assert_eq!(recovered,0);
                                assert_eq!(snapshot(current).await,upgraded,"authorization denial consumed pending intent");
                                assert_eq!(calls.load(Ordering::SeqCst),0);
                            }
                        }
                        let recovered = super::super::recover_resume_pending_sessions_with_auth(current,current_mux,Some(&auth)).await.unwrap();
                        eprintln!("AG-07 collision recovery recovered={recovered}");
                        assert_eq!(recovered, 2);
                        let mut observed = BTreeMap::new();
                        let mut finished = 0;
                        for _ in 0..4 {
                            match rx.recv().await.expect("actor event channel closed") {
                                Observed::Run(context, event) => {
                                    eprintln!("AG-07 collision backend key={}; remote={:?}",context.key.storage_key(),context.state.metadata.get("omo_thread_id"));
                                    assert!(context.key.user_id.is_empty(),"authorization leaked actor identity into the shared routing key");
                                    observed.insert(context.key.storage_key(), (context.state, event.content));
                                }
                                Observed::Finished => finished += 1,
                            }
                        }
                        assert_eq!(observed.keys().map(String::as_str).collect::<Vec<_>>(), vec![CANONICAL, KEYS[6]], "ambiguous history replayed or unrelated lane blocked");
                        assert_eq!(finished, 2);
                        for (key, i) in [(CANONICAL,5), (KEYS[6],6)] {
                            let (state, content) = &observed[key];
                            assert_eq!(state.metadata["omo_thread_id"], format!("remote-{i}"));
                            assert_eq!(content, &format!("work {}",KEYS[i]));
                        }
                        assert_eq!(upgraded, expected, "rekey changed payloads, IDs, order, initiator, checkpoints or dependent rows");
                        let violations: i64 = sqlx::query_scalar("SELECT count(*) FROM pragma_foreign_key_check")
                            .fetch_one(current).await.unwrap();
                        assert_eq!(violations,0);
                        let conflicts: Vec<(String,String)> = sqlx::query_as("SELECT raw_key,canonical_key FROM legacy_guild_session_conflicts ORDER BY raw_key")
                            .fetch_all(current).await.unwrap();
                        let mut expected_conflicts: Vec<_> = KEYS[..5].iter().map(|raw| (raw.to_string(),SessionKey::from_storage_key(raw).unwrap().storage_key())).collect();
                        expected_conflicts.sort();
                        assert_eq!(conflicts, expected_conflicts);
                        eprintln!("AG-07 deferred OD-01 conflicts={conflicts:?}");
                        // Both canonical-present and canonical-absent collisions must refuse
                        // real ordinary ingress rather than choose a history or create a shadow.
                        for channel in ["200","201"] {
                            let key = SessionKey::new("discord",Some("100"),channel,None::<String>,"42").with_bot_id("84");
                            let result = current_mux.route_awaiting_turn(InboundEvent::message(key,format!("blocked-{channel}"),"must not execute")).await;
                            assert!(result.is_err(), "ambiguous lane accepted ingress: {channel}");
                        }
                        let after = snapshot(current).await;
                        for (table, rows) in &before {
                            assert_eq!(after[table][..5], rows[..5], "ambiguous rows modified in {table}");
                        }
                        assert_eq!(super::super::recover_resume_pending_sessions_with_auth(current,current_mux,Some(&auth)).await.unwrap(),0);
                        assert_eq!(omon_gateway::storage::count_resume_pending_sessions(current).await.unwrap(),4);
                        assert!(omon_gateway::storage::fetch_resume_pending_session_keys(current).await.unwrap().is_empty(), "deferred lanes must not feed the startup restart-loop gate");
                        if authorization == Authorization::Deferral {
                            // Current-format pending work, not obsolete rows seeded after upgrade.
                            // A new message in the formerly legacy lane must NOT inherit its grant.
                            for (i,key) in [(5,CANONICAL),(6,KEYS[6])] {
                                sqlx::query("INSERT INTO messages (id,session_key,role,content,metadata_json,created_at,platform_message_id) VALUES (?,?,'user','new shared-lane work','[]','2020-01-02T00:00:00Z',?)")
                                    .bind(format!("00000000-0000-4000-8000-00000000008{i}"))
                                    .bind(key).bind(format!("new-p{i}")).execute(current).await.unwrap();
                                omon_gateway::storage::mark_session_resume_pending(current,key).await.unwrap();
                            }
                            let index = omon_gateway::MessageSearchIndex::new(current.clone());
                            let unknown = index.get("discord","202","new-p5").await.unwrap().unwrap();
                            assert_eq!(unknown.author_id,"42");
                            assert_eq!(unknown.metadata["source"],"transcript");
                            let mut unauthorized = index.get("discord","203","new-p6").await.unwrap().unwrap();
                            unauthorized.author_id = "43".into();
                            unauthorized.metadata = json!({"source":"discord_rest", "author_is_bot":false});
                            index.upsert(&unauthorized).await.unwrap();
                            // Reopening must not assign an old user's grant to the new message.
                            omon_gateway::storage::init_pool(&url).await.unwrap().close().await;
                            let deferred = snapshot(current).await;
                            for _ in 0..2 {
                                let recovered = super::super::recover_resume_pending_sessions_with_auth(current,current_mux,Some(&auth)).await.unwrap();
                                eprintln!("AG-07 new-work authorization recovered={recovered}; unknown=202; unauthorized=203");
                                assert_eq!(recovered,0);
                                assert_eq!(snapshot(current).await,deferred,"unknown/unauthorized pending work was consumed");
                                assert_eq!(omon_gateway::storage::count_resume_pending_sessions(current).await.unwrap(),6);
                            }
                            assert_eq!(calls.load(Ordering::SeqCst),2);
                            // Reauthorizing the proven actor may replay its lane; the unknown
                            // neighbor must remain pending instead of causing global refusal.
                            let reauthorized = super::super::StartupAuthorization { allowed_users: vec![43], ..auth.clone() };
                            assert_eq!(super::super::recover_resume_pending_sessions_with_auth(current,current_mux,Some(&reauthorized)).await.unwrap(),1);
                            let Some(Observed::Run(context,event)) = rx.recv().await else { panic!("reauthorized work missing"); };
                            assert_eq!(context.key.storage_key(),KEYS[6]);
                            assert!(context.key.user_id.is_empty());
                            assert_eq!(context.state.metadata["omo_thread_id"],"remote-6");
                            assert_eq!(event.content,"new shared-lane work");
                            assert!(matches!(rx.recv().await,Some(Observed::Finished)));
                            assert_eq!(omon_gateway::storage::count_resume_pending_sessions(current).await.unwrap(),5);
                        }
                    }).await.expect("AG-07 upgrade fixture timed out");
                }).catch_unwind().await;
                drop(mux.take());
                drop(tx.take());
                let actors_closed = tokio::time::timeout(LIMIT,async { while rx.recv().await.is_some() {} }).await;
                let pool_closed = tokio::time::timeout(LIMIT,async { if let Some(pool) = pool.take() { pool.close().await; } }).await;
                (outcome,actors_closed,pool_closed,calls.load(Ordering::SeqCst))
            });
            drop(runtime);
            let removed = temp.close();
            eprintln!("AG-07 upgrade cleanup actors_closed={actors_closed:?}; pool_closed={pool_closed:?}; temp_removed={removed:?}; runtime_dropped=true");
            assert!(
                actors_closed.is_ok(),
                "AG-07 upgrade actor cleanup timed out"
            );
            assert!(pool_closed.is_ok(), "AG-07 upgrade pool cleanup timed out");
            removed.expect("AG-07 upgrade temporary database cleanup failed");
            if let Err(panic) = outcome {
                std::panic::resume_unwind(panic);
            }
            let expected_calls = if authorization == Authorization::Deferral {
                3
            } else {
                2
            };
            assert_eq!(
                calls, expected_calls,
                "only authorized unambiguous pending work may execute"
            );
        }
    }

    #[test]
    fn ag07_legacy_guild_startup_replays_preserved_history_once() {
        use std::borrow::Cow;
        use std::panic::AssertUnwindSafe;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        use futures_util::FutureExt;
        use omon_gateway::{
            AgentRunner, InboundEvent, MultiplexerConfig, OutboundAction, OutboundDispatcher,
            SessionContext, SessionKey, SessionMultiplexer, SessionState,
        };
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use tokio::sync::mpsc;

        const RAW: &str = "7:discord|3:100|3:200|-|2:42|2:84";
        const LIMIT: Duration = Duration::from_secs(20);
        type History = Vec<(String, String, String, String, Option<String>)>;

        #[allow(clippy::large_enum_variant)]
        enum Observed {
            Run(SessionContext, InboundEvent, History),
            Finished,
        }
        struct Recorder {
            pool: sqlx::SqlitePool,
            tx: mpsc::UnboundedSender<Observed>,
            calls: Arc<AtomicUsize>,
        }
        #[async_trait::async_trait]
        impl AgentRunner for Recorder {
            async fn run(
                &self,
                session: &mut SessionContext,
                event: InboundEvent,
            ) -> omon_gateway::Result<()> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                // Use the actor's actual key, never RAW or a test-side alias lookup.
                let history: History = sqlx::query_as(
                    "SELECT id, role, content, metadata_json, platform_message_id
                         FROM messages WHERE session_key = ? AND id IN ('ag07-user',
                         'ag07-assistant', '00000000-0000-4000-8000-000000000007')
                         ORDER BY sequence",
                )
                .bind(session.key.storage_key())
                .fetch_all(&self.pool)
                .await?;
                self.tx
                    .send(Observed::Run(session.clone(), event, history))
                    .map_err(|e| omon_gateway::OmonError::Multiplexer(e.to_string()))?;
                Ok(())
            }
        }
        #[async_trait::async_trait]
        impl OutboundDispatcher for Recorder {
            async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
                // This is emitted by the real actor after completion/flush, not by run().
                if matches!(action, OutboundAction::Typing { active: false, .. }) {
                    self.tx
                        .send(Observed::Finished)
                        .map_err(|e| omon_gateway::OmonError::Multiplexer(e.to_string()))?;
                }
                Ok(())
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("legacy.sqlite");
        let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
        // Own the runtime as well: a failed cleanup timeout cannot leave detached actors.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (outcome, actors_closed, pool_closed, calls) = runtime.block_on(async {
            let mut pool: Option<sqlx::SqlitePool> = None;
            let mut mux: Option<SessionMultiplexer> = None;
            let (tx, mut rx) = mpsc::unbounded_channel();
            let mut tx = Some(tx);
            let calls = Arc::new(AtomicUsize::new(0));
            let outcome = AssertUnwindSafe(async {
                tokio::time::timeout(LIMIT, async {
                    pool = Some(
                        SqlitePoolOptions::new()
                            .max_connections(1)
                            .connect_with(
                                SqliteConnectOptions::new()
                                    .filename(&path)
                                    .create_if_missing(true)
                                    .foreign_keys(true),
                            )
                            .await
                            .unwrap(),
                    );
                    let old = pool.as_ref().unwrap();
                    // Freeze the old schema, including sqlx checksums/bookkeeping. Never
                    // connect with today's Database migrator before inserting legacy rows.
                    let shipped = sqlx::migrate!("./migrations");
                    let old_schema = sqlx::migrate::Migrator {
                        migrations: Cow::Owned(
                            shipped
                                .iter()
                                .filter(|m| m.version <= 16)
                                .cloned()
                                .collect(),
                        ),
                        ..sqlx::migrate::Migrator::DEFAULT
                    };
                    old_schema.run(old).await.unwrap();
                    let mut state = SessionState {
                        active_model: Some("ag07-model".into()),
                        system_prompt: Some("ag07-persona".into()),
                        enabled_toolsets: Some(vec!["memory".into()]),
                        ..Default::default()
                    };
                    state
                        .metadata
                        .insert("omo_thread_id".into(), "ag07-remote-history".into());
                    state.metadata.insert("ag07_checkpoint".into(), 7.into());
                    sqlx::query(
                        "INSERT INTO sessions (session_key, platform, guild_id, channel_id,
                             thread_id, user_id, state_json, resume_pending, created_at, updated_at)
                             VALUES (?, 'discord', '100', '200', NULL, '42', ?, 1,
                             '2020-01-01T00:00:00Z', '2020-01-01T00:02:00Z')",
                    )
                    .bind(RAW)
                    .bind(serde_json::to_string(&state).unwrap())
                    .execute(old)
                    .await
                    .unwrap();
                    let expected: History = vec![
                        (
                            "ag07-user".into(),
                            "user".into(),
                            "ag07-old-question".into(),
                            "[]".into(),
                            Some("7001".into()),
                        ),
                        (
                            "ag07-assistant".into(),
                            "assistant".into(),
                            "ag07-old-answer".into(),
                            "[]".into(),
                            None,
                        ),
                        (
                            "00000000-0000-4000-8000-000000000007".into(),
                            "user".into(),
                            "ag07-pending-work".into(),
                            "[]".into(),
                            Some("7003".into()),
                        ),
                    ];
                    for (id, role, content, metadata, platform_id) in &expected {
                        sqlx::query(
                            "INSERT INTO messages (id, session_key, role, content,
                                 metadata_json, platform_message_id, created_at)
                                 VALUES (?, ?, ?, ?, ?, ?, '2020-01-01T00:01:00Z')",
                        )
                        .bind(id)
                        .bind(RAW)
                        .bind(role)
                        .bind(content)
                        .bind(metadata)
                        .bind(platform_id)
                        .execute(old)
                        .await
                        .unwrap();
                    }
                    let stored: String = sqlx::query_scalar("SELECT session_key FROM sessions")
                        .fetch_one(old)
                        .await
                        .unwrap();
                    assert_eq!(stored.as_bytes(), RAW.as_bytes());
                    eprintln!(
                        "AG-07 pre-upgrade raw={stored}; raw_bytes={:?}",
                        stored.as_bytes()
                    );
                    pool.take().unwrap().close().await;

                    // The same storage-open and authorized recovery functions used by main.
                    pool = Some(omon_gateway::storage::init_pool(&url).await.unwrap());
                    let current = pool.as_ref().unwrap();
                    let probe = Arc::new(Recorder {
                        pool: current.clone(),
                        tx: tx.take().unwrap(),
                        calls: calls.clone(),
                    });
                    mux = Some(SessionMultiplexer::with_dispatcher(
                        current.clone(),
                        probe.clone(),
                        Some(probe),
                        MultiplexerConfig::default(),
                    ));
                    let current_mux = mux.as_ref().unwrap();
                    let auth = super::StartupAuthorization {
                        allow_all_users: true,
                        ..Default::default()
                    };
                    let recovered = super::recover_resume_pending_sessions_with_auth(
                        current,
                        current_mux,
                        Some(&auth),
                    )
                    .await
                    .unwrap();
                    eprintln!("AG-07 recovery raw={RAW}; recovered={recovered}");
                    assert_eq!(recovered, 1, "legacy pending work stranded: {RAW}");
                    let Some(Observed::Run(context, event, history)) = rx.recv().await else {
                        panic!("no backend replay for {RAW}");
                    };
                    eprintln!(
                        "AG-07 backend key={}; remote_binding={:?}",
                        context.key.storage_key(),
                        context.state.metadata.get("omo_thread_id")
                    );
                    assert_eq!(context.state, state, "remote binding/state lost: {RAW}");
                    assert_eq!(context.key.bot_id.as_deref(), Some("84"));
                    assert_eq!(event.session, context.key);
                    assert_eq!(event.content, "ag07-pending-work");
                    assert_eq!(history, expected, "original history not addressable: {RAW}");
                    assert!(matches!(rx.recv().await, Some(Observed::Finished)));
                    // Normal guild ingress must resolve the same preserved conversation too.
                    let ingress =
                        SessionKey::new("discord", Some("100"), "200", None::<String>, "42")
                            .with_bot_id("84");
                    let loaded = current_mux
                        .session_context(&ingress)
                        .await
                        .unwrap()
                        .unwrap();
                    assert_eq!(loaded.state, state);
                    assert_eq!(
                        omon_gateway::storage::count_resume_pending_sessions(current)
                            .await
                            .unwrap(),
                        0
                    );
                    assert_eq!(
                        super::recover_resume_pending_sessions_with_auth(
                            current,
                            current_mux,
                            Some(&auth),
                        )
                        .await
                        .unwrap(),
                        0
                    );
                    // Retain every original transcript row globally as well as at the backend.
                    let retained: History = sqlx::query_as(
                        "SELECT id, role, content, metadata_json, platform_message_id FROM messages
                             WHERE id IN ('ag07-user', 'ag07-assistant',
                             '00000000-0000-4000-8000-000000000007') ORDER BY sequence",
                    )
                    .fetch_all(current)
                    .await
                    .unwrap();
                    assert_eq!(retained, expected);
                })
                .await
                .expect("AG-07 fixture timed out");
            })
            .catch_unwind()
            .await;

            // Finally runs on assertion failure and scenario timeout. Channel closure proves
            // all actor-held recorder/dispatcher owners were released; no polling or sleeps.
            drop(mux.take());
            drop(tx.take());
            let actors_closed =
                tokio::time::timeout(LIMIT, async { while rx.recv().await.is_some() {} }).await;
            let pool_closed = tokio::time::timeout(LIMIT, async {
                if let Some(pool) = pool.take() {
                    pool.close().await;
                }
            })
            .await;
            (
                outcome,
                actors_closed,
                pool_closed,
                calls.load(Ordering::SeqCst),
            )
        });
        drop(runtime);
        let removed = temp.close();
        eprintln!(
            "AG-07 cleanup actors_closed={actors_closed:?}; pool_closed={pool_closed:?}; temp_removed={removed:?}; runtime_dropped=true"
        );
        assert!(actors_closed.is_ok(), "AG-07 actor cleanup timed out");
        assert!(pool_closed.is_ok(), "AG-07 pool cleanup timed out");
        removed.expect("AG-07 temporary database cleanup failed");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
        assert_eq!(
            calls, 1,
            "legacy work must reach the backend exactly once: {RAW}"
        );
    }

    #[test]
    fn recovery_preserves_bot_identity() {
        test_recover_resume_pending_sessions_preserves_bot_identity();
    }

    #[tokio::test]
    async fn test_recover_resume_pending_sessions_preserves_bot_identity() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();

        let bot42 = omon_gateway::SessionKey::new(
            "discord",
            None::<String>,
            "chan-rec-bot",
            None::<String>,
            "user-rec-bot",
        )
        .with_bot_id("42");
        let bot84 = omon_gateway::SessionKey::new(
            "discord",
            None::<String>,
            "chan-rec-bot",
            None::<String>,
            "user-rec-bot",
        )
        .with_bot_id("84");

        super::ensure_agent_session(&pool, &omon_gateway::SessionContext::new(bot42.clone()))
            .await
            .unwrap();
        super::ensure_agent_session(&pool, &omon_gateway::SessionContext::new(bot84.clone()))
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-42-main', ?, 'user', 'turn 42', '[]')",
        )
        .bind(bot42.storage_key())
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-84-main', ?, 'user', 'turn 84', '[]')",
        )
        .bind(bot84.storage_key())
        .execute(&pool)
        .await
        .unwrap();

        omon_gateway::storage::mark_session_resume_pending(&pool, &bot42.storage_key())
            .await
            .unwrap();
        omon_gateway::storage::mark_session_resume_pending(&pool, &bot84.storage_key())
            .await
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Option<String>, String)>(4);

        struct BotRecorderRunner(tokio::sync::mpsc::Sender<(Option<String>, String)>);
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for BotRecorderRunner {
            async fn run(
                &self,
                session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                let _ = self
                    .0
                    .send((session.key.bot_id.clone(), event.content))
                    .await;
                Ok(())
            }
        }

        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(BotRecorderRunner(tx)),
            omon_gateway::MultiplexerConfig::default(),
        );

        let recovered = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered, 2);

        let ev1 = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("first event timeout")
            .expect("first event channel closed");
        let ev2 = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("second event timeout")
            .expect("second event channel closed");

        let runs = [ev1, ev2];
        assert!(runs
            .iter()
            .any(|(b, c)| b.as_deref() == Some("42") && c == "turn 42"));
        assert!(runs
            .iter()
            .any(|(b, c)| b.as_deref() == Some("84") && c == "turn 84"));
    }

    #[tokio::test]
    async fn restart_chain_preserves_pending_until_explicit_input() {
        use omon_gateway::RestartLoopGuard;

        struct RecordingRunner(tokio::sync::mpsc::UnboundedSender<String>);
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for RecordingRunner {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                self.0.send(event.content).unwrap();
                Ok(())
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", temp.path().join("gateway.db").display());
        let pool = omon_gateway::storage::init_pool(&url).await.unwrap();
        let key = omon_gateway::SessionKey::new(
            "discord",
            None::<String>,
            "restart-channel",
            None::<String>,
            "owner",
        );
        super::ensure_agent_session(&pool, &omon_gateway::SessionContext::new(key.clone()))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('restart-pending', ?, 'user', 'old pending', '[]')",
        )
        .bind(key.storage_key())
        .execute(&pool)
        .await
        .unwrap();
        omon_gateway::storage::mark_session_resume_pending(&pool, &key.storage_key())
            .await
            .unwrap();
        let path = temp.path().join("restart_loop.json");
        assert!(!RestartLoopGuard::new(&path).check_and_record_at(0.0));
        assert!(!RestartLoopGuard::new(&path).check_and_record_at(150.0));
        let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(RecordingRunner(sent)),
            omon_gateway::MultiplexerConfig::default(),
        );
        let recovered = if RestartLoopGuard::new(&path).check_and_record_at(300.0) {
            0
        } else {
            super::recover_resume_pending_sessions(&pool, &multiplexer)
                .await
                .unwrap()
        };
        assert_eq!(
            recovered, 0,
            "slow restart loop must not automatically rerun work"
        );
        assert_eq!(multiplexer.active_sessions(), 0);
        assert_eq!(
            omon_gateway::storage::count_resume_pending_sessions(&pool)
                .await
                .unwrap(),
            1
        );
        multiplexer
            .route(omon_gateway::InboundEvent::message(
                key.clone(),
                "explicit-after-breaker",
                "explicit new input",
            ))
            .await
            .unwrap();
        let observed = tokio::time::timeout(std::time::Duration::from_secs(5), received.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observed, "explicit new input");
        multiplexer.stop(&key).await.unwrap();
        pool.close().await;
    }

    #[tokio::test]
    async fn test_recover_resume_pending_sessions_redispatches_unfinished_turn() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();

        let session_key = omon_gateway::SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-rec-turn",
            None::<String>,
            "user-rec-turn",
        );
        super::ensure_agent_session(
            &pool,
            &omon_gateway::SessionContext::new(session_key.clone()),
        )
        .await
        .unwrap();

        // Persist an unfinished user turn
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-unfin', ?, 'user', 'resume this prompt', '[]')",
        )
        .bind(session_key.storage_key())
        .execute(&pool)
        .await
        .unwrap();

        // Mark resume_pending
        omon_gateway::storage::mark_session_resume_pending(&pool, &session_key.storage_key())
            .await
            .unwrap();

        let pending = omon_gateway::storage::fetch_resume_pending_session_keys(&pool)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);

        struct MockRunner;
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for MockRunner {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                assert_eq!(event.content, "resume this prompt");
                Ok(())
            }
        }

        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(MockRunner),
            omon_gateway::MultiplexerConfig::default(),
        );

        let recovered = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered, 1);

        // Resume pending flag must now be cleared
        let pending_after = omon_gateway::storage::fetch_resume_pending_session_keys(&pool)
            .await
            .unwrap();
        assert!(pending_after.is_empty());

        // A second recovery sweep should find 0 sessions and not resume twice
        let recovered_second = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered_second, 0);
    }

    #[tokio::test]
    async fn test_suspended_session_suppresses_auto_resume() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        let session_key = omon_gateway::SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-suspended",
            None::<String>,
            "user-suspended",
        );
        let mut session = omon_gateway::SessionContext::new(session_key.clone());
        session.state.suspended = true;
        super::ensure_agent_session(&pool, &session).await.unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-suspended', ?, 'user', 'should not be resumed', '[]')",
        )
        .bind(session_key.storage_key())
        .execute(&pool)
        .await
        .unwrap();

        // Mark session as resume_pending
        omon_gateway::storage::mark_session_resume_pending(&pool, &session_key.storage_key())
            .await
            .unwrap();

        struct PanicRunner;
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for PanicRunner {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                _event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                panic!("Suspended session must not be auto-resumed!");
            }
        }

        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(PanicRunner),
            omon_gateway::MultiplexerConfig::default(),
        );

        let recovered = super::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(
            recovered, 0,
            "Suspended session must be skipped by recovery"
        );

        // The resume_pending flag should be cleared so it won't repeatedly re-attempt
        let pending = omon_gateway::storage::fetch_resume_pending_session_keys(&pool)
            .await
            .unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_restart_loop_guard_suppresses_crash_loop_auto_resume() {
        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let guard_file = temp.path().join("restart_loop.json");
        let guard = omon_gateway::RestartLoopGuard::with_config(&guard_file, 3, 60);

        let session_key = omon_gateway::SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-poison",
            None::<String>,
            "user-poison",
        );
        super::ensure_agent_session(
            &pool,
            &omon_gateway::SessionContext::new(session_key.clone()),
        )
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-poison', ?, 'user', 'crash daemon command', '[]')",
        )
        .bind(session_key.storage_key())
        .execute(&pool)
        .await
        .unwrap();

        omon_gateway::storage::mark_session_resume_pending(&pool, &session_key.storage_key())
            .await
            .unwrap();

        // Simulate 2 previous boots within window
        guard.record_boot_at(10.0);
        guard.record_boot_at(20.0);

        // 3rd boot at t=30.0 trips the breaker!
        let tripped = guard.check_and_record_at(30.0);
        assert!(tripped, "Breaker must be tripped on 3rd boot");

        let dispatch_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let dispatch_counter_clone = dispatch_counter.clone();

        struct PoisonMockRunner {
            counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        }
        #[async_trait::async_trait]
        impl omon_gateway::AgentRunner for PoisonMockRunner {
            async fn run(
                &self,
                _session: &mut omon_gateway::SessionContext,
                _event: omon_gateway::InboundEvent,
            ) -> omon_gateway::Result<()> {
                self.counter
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }

        let multiplexer = omon_gateway::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(PoisonMockRunner {
                counter: dispatch_counter_clone,
            }),
            omon_gateway::MultiplexerConfig::default(),
        );

        // Because breaker is tripped, gateway startup skips auto-resume:
        let pending_count = omon_gateway::storage::count_resume_pending_sessions(&pool)
            .await
            .unwrap();
        assert_eq!(pending_count, 1);
        if !tripped {
            let _ = super::recover_resume_pending_sessions(&pool, &multiplexer).await;
        }

        // Verify that no task was dispatched
        assert_eq!(
            dispatch_counter.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        // Session remains marked resume_pending for manual resolution / next real user event
        assert_eq!(
            omon_gateway::storage::count_resume_pending_sessions(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_load_cron_skills_missing_skill_resilience() {
        let temp_dir =
            std::env::temp_dir().join(format!("omon-test-skills-{}", uuid::Uuid::new_v4()));
        let skills_dir = temp_dir.join("skills");
        let skill_a_dir = skills_dir.join("skill_a");
        std::fs::create_dir_all(&skill_a_dir).unwrap();
        std::fs::write(skill_a_dir.join("SKILL.md"), "Instructions for skill A").unwrap();

        let mut extra = HashMap::new();
        extra.insert(
            "_omon_hermes_home".into(),
            serde_json::Value::String(temp_dir.to_string_lossy().into_owned()),
        );

        // 1. Partial missing skills with prompt -> warning prepended, job does not fail
        let partial_job = HermesJob {
            id: "job_partial".into(),
            name: "Partial".into(),
            prompt: "Summarize status".into(),
            skills: vec!["skill_a".into(), "skill_missing".into()],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            script: None,
            no_agent: false,
            ack_command: None,
            context_from: None,
            schedule: omon_gateway::HermesSchedule::default(),
            schedule_display: "".into(),
            repeat: omon_gateway::HermesRepeat::default(),
            enabled: true,
            state: "".into(),
            created_at: None,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            last_delivery_error: None,
            deliver: None,
            failure_deliver: None,
            origin: None,
            enabled_toolsets: None,
            workdir: None,
            attach_to_session: None,
            timeout_secs: None,
            brief_file: None,
            catch_up_hours: None,
            monitor_script: None,
            monitor_url: None,
            monitor_state: None,
            extra: extra.clone(),
        };
        let loaded = load_cron_skills(&partial_job).unwrap();
        assert!(loaded.contains("⚠️ Skill(s) not found and skipped: skill_missing"));
        assert!(loaded.contains("[Skill: skill_a]\nInstructions for skill A"));

        // 2. All skills missing but non-empty prompt -> returns warning only, succeeds
        let missing_with_prompt_job = HermesJob {
            id: "job_missing".into(),
            name: "Missing".into(),
            prompt: "Do something anyway".into(),
            skills: vec!["missing_1".into(), "missing_2".into()],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            script: None,
            no_agent: false,
            ack_command: None,
            context_from: None,
            schedule: omon_gateway::HermesSchedule::default(),
            schedule_display: "".into(),
            repeat: omon_gateway::HermesRepeat::default(),
            enabled: true,
            state: "".into(),
            created_at: None,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            last_delivery_error: None,
            deliver: None,
            failure_deliver: None,
            origin: None,
            enabled_toolsets: None,
            workdir: None,
            attach_to_session: None,
            timeout_secs: None,
            brief_file: None,
            catch_up_hours: None,
            monitor_script: None,
            monitor_url: None,
            monitor_state: None,
            extra: extra.clone(),
        };
        let loaded_warn = load_cron_skills(&missing_with_prompt_job).unwrap();
        assert_eq!(
            loaded_warn,
            "⚠️ Skill(s) not found and skipped: missing_1, missing_2"
        );

        // 3. All skills missing and EMPTY prompt -> fails with Config error
        let empty_prompt_missing_job = HermesJob {
            id: "job_empty".into(),
            name: "Empty".into(),
            prompt: "".into(),
            skills: vec!["missing_skill".into()],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            script: None,
            no_agent: false,
            ack_command: None,
            context_from: None,
            schedule: omon_gateway::HermesSchedule::default(),
            schedule_display: "".into(),
            repeat: omon_gateway::HermesRepeat::default(),
            enabled: true,
            state: "".into(),
            created_at: None,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            last_delivery_error: None,
            deliver: None,
            failure_deliver: None,
            origin: None,
            enabled_toolsets: None,
            workdir: None,
            attach_to_session: None,
            timeout_secs: None,
            brief_file: None,
            catch_up_hours: None,
            monitor_script: None,
            monitor_url: None,
            monitor_state: None,
            extra,
        };
        let err = load_cron_skills(&empty_prompt_missing_job).unwrap_err();
        assert!(err
            .to_string()
            .contains("empty prompt and all skills were missing"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_resolve_skill_bundle_and_expansion() {
        let temp_dir =
            std::env::temp_dir().join(format!("omon-test-bundles-{}", uuid::Uuid::new_v4()));
        let skills_dir = temp_dir.join("skills");
        let bundles_dir = temp_dir.join("skill-bundles");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::create_dir_all(&bundles_dir).unwrap();

        // 1. Create a regular single skill
        let s1_dir = skills_dir.join("skill_1");
        std::fs::create_dir_all(&s1_dir).unwrap();
        std::fs::write(s1_dir.join("SKILL.md"), "Skill 1 instructions").unwrap();

        // 2. Create another single skill
        let s2_dir = skills_dir.join("skill_2");
        std::fs::create_dir_all(&s2_dir).unwrap();
        std::fs::write(s2_dir.join("SKILL.md"), "Skill 2 instructions").unwrap();

        // 3. Create a YAML bundle in skill-bundles/
        std::fs::write(
            bundles_dir.join("backend_bundle.yaml"),
            "name: backend_bundle\nskills:\n  - skill_1\n  - skill_2\n",
        )
        .unwrap();

        // 4. Create a multi-skill directory bundle in skills/group_bundle/
        let group_dir = skills_dir.join("group_bundle");
        let member_a = group_dir.join("member_a");
        let member_b = group_dir.join("member_b");
        std::fs::create_dir_all(&member_a).unwrap();
        std::fs::create_dir_all(&member_b).unwrap();
        std::fs::write(member_a.join("SKILL.md"), "Member A instructions").unwrap();
        std::fs::write(member_b.join("SKILL.md"), "Member B instructions").unwrap();

        // Verify resolve_skill_bundle for YAML manifest bundle
        let resolved_yaml =
            super::resolve_skill_bundle(&skills_dir, Some(&temp_dir), "backend_bundle");
        assert_eq!(
            resolved_yaml,
            Some(vec!["skill_1".to_string(), "skill_2".to_string()])
        );

        // Verify resolve_skill_bundle for directory bundle
        let resolved_dir =
            super::resolve_skill_bundle(&skills_dir, Some(&temp_dir), "group_bundle");
        assert_eq!(
            resolved_dir,
            Some(vec![
                "group_bundle/member_a".to_string(),
                "group_bundle/member_b".to_string()
            ])
        );

        // Verify single skill returns None (not a bundle)
        let resolved_single = super::resolve_skill_bundle(&skills_dir, Some(&temp_dir), "skill_1");
        assert_eq!(resolved_single, None);

        // Verify load_cron_skills expands the bundle and loads skill bodies
        let mut extra = HashMap::new();
        extra.insert(
            "_omon_hermes_home".into(),
            serde_json::Value::String(temp_dir.to_string_lossy().into_owned()),
        );

        let bundle_job = HermesJob {
            id: "job_bundle".into(),
            name: "Bundle Job".into(),
            prompt: "Perform bundle task".into(),
            skills: vec!["backend_bundle".into()],
            skill: None,
            model: None,
            provider: None,
            base_url: None,
            script: None,
            no_agent: false,
            ack_command: None,
            context_from: None,
            schedule: omon_gateway::HermesSchedule::default(),
            schedule_display: "".into(),
            repeat: omon_gateway::HermesRepeat::default(),
            enabled: true,
            state: "".into(),
            created_at: None,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            last_delivery_error: None,
            deliver: None,
            failure_deliver: None,
            origin: None,
            enabled_toolsets: None,
            workdir: None,
            attach_to_session: None,
            timeout_secs: None,
            brief_file: None,
            catch_up_hours: None,
            monitor_script: None,
            monitor_url: None,
            monitor_state: None,
            extra,
        };

        let loaded = load_cron_skills(&bundle_job).unwrap();
        assert!(
            loaded.contains("[Skill: skill_1]\nSkill 1 instructions"),
            "Loaded skills must include skill_1: {loaded}"
        );
        assert!(
            loaded.contains("[Skill: skill_2]\nSkill 2 instructions"),
            "Loaded skills must include skill_2: {loaded}"
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_build_cron_llm_config_overrides() {
        let base = omon_gateway::LlmConfig::new(omon_gateway::LlmProvider::OpenAi, "gpt-4o-mini");

        // 1. Override model only
        let cfg1 = super::build_cron_llm_config(&base, None, None, Some("gpt-4o"));
        assert_eq!(cfg1.model, "gpt-4o");
        assert_eq!(cfg1.provider, omon_gateway::LlmProvider::OpenAi);
        assert_eq!(cfg1.base_url, None);

        // 2. Override provider only
        let cfg2 = super::build_cron_llm_config(&base, Some("anthropic"), None, None);
        assert_eq!(cfg2.provider, omon_gateway::LlmProvider::Anthropic);
        assert_eq!(cfg2.model, "gpt-4o-mini");

        // 3. Override base_url only
        let cfg3 =
            super::build_cron_llm_config(&base, None, Some("http://127.0.0.1:11434/api"), None);
        assert_eq!(cfg3.base_url.as_deref(), Some("http://127.0.0.1:11434/api"));
        assert_eq!(cfg3.model, "gpt-4o-mini");

        // 4. Override all three
        let cfg4 = super::build_cron_llm_config(
            &base,
            Some("deepseek"),
            Some("https://api.deepseek.com/v1"),
            Some("deepseek-chat"),
        );
        assert_eq!(cfg4.provider, omon_gateway::LlmProvider::DeepSeek);
        assert_eq!(
            cfg4.base_url.as_deref(),
            Some("https://api.deepseek.com/v1")
        );
        assert_eq!(cfg4.model, "deepseek-chat");

        // 5. Empty / whitespace overrides preserve base
        let cfg5 = super::build_cron_llm_config(&base, Some("  "), Some(""), Some(" "));
        assert_eq!(cfg5.model, "gpt-4o-mini");
        assert_eq!(cfg5.provider, omon_gateway::LlmProvider::OpenAi);
        assert_eq!(cfg5.base_url, None);
    }

    #[test]
    fn test_resolve_workspace_instructions() {
        let temp_dir =
            std::env::temp_dir().join(format!("omon-test-instructions-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. None when neither AGENTS.md nor CLAUDE.md exists
        assert_eq!(super::resolve_workspace_instructions(&temp_dir), None);

        // 2. Loads AGENTS.md
        std::fs::write(
            temp_dir.join("AGENTS.md"),
            "Rule 1: Always format code with cargo fmt.\n",
        )
        .unwrap();
        let loaded = super::resolve_workspace_instructions(&temp_dir).unwrap();
        assert_eq!(
            loaded,
            "[Workspace instructions]\nRule 1: Always format code with cargo fmt."
        );

        // 3. Precedence: AGENTS.md beats CLAUDE.md
        std::fs::write(
            temp_dir.join("CLAUDE.md"),
            "Claude rules that should be ignored.",
        )
        .unwrap();
        let loaded_prec = super::resolve_workspace_instructions(&temp_dir).unwrap();
        assert!(loaded_prec.contains("Rule 1: Always format code with cargo fmt."));
        assert!(!loaded_prec.contains("Claude rules"));

        // 4. CLAUDE.md when AGENTS.md removed
        std::fs::remove_file(temp_dir.join("AGENTS.md")).unwrap();
        let loaded_claude = super::resolve_workspace_instructions(&temp_dir).unwrap();
        assert_eq!(
            loaded_claude,
            "[Workspace instructions]\nClaude rules that should be ignored."
        );

        // 5. Truncation when exceeding 8000 chars
        let long_content = "X".repeat(8500);
        std::fs::write(temp_dir.join("CLAUDE.md"), &long_content).unwrap();
        let loaded_trunc = super::resolve_workspace_instructions(&temp_dir).unwrap();
        assert_eq!(
            loaded_trunc,
            format!("[Workspace instructions]\n{}", "X".repeat(8000))
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn imsg_stdout_error_reaches_the_exit_tail() {
        let mut noise = super::StderrTailBuffer::new(super::WATCHER_STDERR_TAIL_MAX_BYTES);
        noise.push_line("unable to open database file (code: 14)");
        noise.push_line("Permission Error: Cannot access Messages database");
        let tail = super::join_tails(String::new(), &noise);
        assert!(tail.contains("Cannot access Messages database"));
        assert_eq!(
            super::join_tails("err".into(), &noise).lines().next(),
            Some("err")
        );
        let empty = super::StderrTailBuffer::new(8);
        assert_eq!(super::join_tails("err".into(), &empty), "err");
    }

    #[test]
    fn test_is_mail_event_line_ignores_comments_and_blank_lines() {
        // Incident 2026-10-16: zele prints a header line at startup:
        // "# Watching inbox every 60s (Ctrl+C to stop)"
        // This must not be treated as an event.
        assert!(!super::is_mail_event_line(
            "# Watching inbox every 60s (Ctrl+C to stop)"
        ));
        assert!(!super::is_mail_event_line("   # indented comment"));
        assert!(!super::is_mail_event_line(""));
        assert!(!super::is_mail_event_line("   \t  \n"));

        // Real mail events / output lines
        assert!(super::is_mail_event_line("new message arrived"));
        assert!(super::is_mail_event_line(
            "{\"id\": \"msg-123\", \"subject\": \"hello\"}"
        ));
        assert!(super::is_mail_event_line("  mail item 42  "));
    }

    #[test]
    fn test_stderr_tail_buffer_bounded_capacity() {
        use super::StderrTailBuffer;

        let mut buf = StderrTailBuffer::new(20);
        assert_eq!(buf.render(), "");

        // Appending lines within capacity preserves everything
        buf.push_line("line 1");
        assert_eq!(buf.render(), "line 1");
        buf.push_line("line 2");
        assert_eq!(buf.render(), "line 1\nline 2");

        // Pushing another line that exceeds capacity drops older lines from front
        buf.push_line("line 3 is long");
        let rendered = buf.render();
        assert!(
            rendered.len() <= 20,
            "rendered len {} must be <= 20",
            rendered.len()
        );
        assert!(rendered.ends_with("line 3 is long"));
        assert!(!rendered.contains("line 1"));

        // Pushing a single line larger than capacity truncates it safely
        let mut small_buf = StderrTailBuffer::new(10);
        small_buf.push_line("0123456789extra_content");
        let truncated = small_buf.render();
        assert!(
            truncated.len() <= 10,
            "rendered len {} must be <= 10",
            truncated.len()
        );
        assert!(truncated.ends_with("extra_content") || truncated.ends_with("ontent"));

        // Handles multi-byte UTF-8 safely without panicking on char boundaries
        let mut utf8_buf = StderrTailBuffer::new(10);
        utf8_buf.push_line("한글메시지테스트");
        let rendered_utf8 = utf8_buf.render();
        assert!(rendered_utf8.len() <= 10);
    }
}

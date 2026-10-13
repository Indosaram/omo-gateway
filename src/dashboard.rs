use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use omon_gateway::{
    readiness, CronJob, CronJobSpec, CronScheduler, InboundEvent, OmonError, OutboundAction,
    OutboundDispatcher, SessionKey, SessionMultiplexer, SmartApprovalGuard, ToolRegistry,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::{FromRow, SqlitePool};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::trace::TraceLayer;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{EnvFilter, Layer};
use uuid::Uuid;

const DEFAULT_DASHBOARD_HOST: &str = "127.0.0.1";
const DEFAULT_DASHBOARD_PORT: u16 = 9119;
const MAX_PAGE_SIZE: u32 = 200;
const LOG_CAPACITY: usize = 2_000;
const DEFAULT_BACKEND_APPSERVER_URL: &str = "ws://127.0.0.1:19742";

#[derive(Clone, Debug, clap::Args)]
pub struct DashboardArgs {
    /// Address to bind the dashboard HTTP server to.
    #[arg(long, default_value = DEFAULT_DASHBOARD_HOST)]
    pub host: String,
    /// Port to bind the dashboard HTTP server to.
    #[arg(long, default_value_t = DEFAULT_DASHBOARD_PORT)]
    pub port: u16,
    /// Allow binding to a non-loopback interface without transport authentication.
    #[arg(long)]
    pub insecure: bool,
}

#[derive(Clone, Debug)]
pub struct DashboardSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub insecure: bool,
    pub web_root: PathBuf,
}

impl DashboardSettings {
    pub fn from_args(args: DashboardArgs) -> Self {
        Self {
            enabled: true,
            host: args.host,
            port: args.port,
            insecure: args.insecure,
            web_root: dashboard_web_root(),
        }
    }

    pub fn from_env() -> Self {
        let port_raw = env::var("DASHBOARD_PORT").ok();
        let enabled = env_bool("DASHBOARD_ENABLED", false) || port_raw.is_some();
        let port = match port_raw.as_deref().map(str::trim) {
            Some(raw) => raw.parse::<u16>().unwrap_or_else(|_| {
                tracing::warn!(
                    value = raw,
                    default = DEFAULT_DASHBOARD_PORT,
                    "invalid DASHBOARD_PORT; falling back to the default port"
                );
                DEFAULT_DASHBOARD_PORT
            }),
            None => DEFAULT_DASHBOARD_PORT,
        };
        Self {
            enabled,
            host: env::var("DASHBOARD_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_DASHBOARD_HOST.to_owned()),
            port,
            insecure: env_bool("DASHBOARD_INSECURE", false),
            web_root: dashboard_web_root(),
        }
    }

    pub fn validate(&self) -> Result<(), OmonError> {
        if self.port == 0 {
            return Err(OmonError::Config(
                "dashboard port must be greater than zero".into(),
            ));
        }
        if !is_loopback_host(&self.host) {
            return Err(OmonError::Config(format!(
                "refusing to expose the unauthenticated dashboard on non-loopback host {}; public bind requires transport authentication which is not configured",
                self.host
            )));
        }
        Ok(())
    }

    pub fn display_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn dashboard_web_root() -> PathBuf {
    env::var_os("DASHBOARD_WEB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("web/dist"))
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let host = host.trim_matches(|c| c == '[' || c == ']');
    host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn extract_host(host_str: &str) -> &str {
    let host_str = host_str.trim();
    if let Some(rest) = host_str.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &rest[..end];
        }
    }
    if host_str.matches(':').count() == 1 {
        if let Some(colon) = host_str.rfind(':') {
            return &host_str[..colon];
        }
    }
    host_str
}

fn extract_port(host_str: &str) -> Option<u16> {
    let host_str = host_str.trim();
    if let Some(rest) = host_str.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let after = &rest[end + 1..];
            if let Some(colon) = after.strip_prefix(':') {
                return colon.parse::<u16>().ok();
            }
            return None;
        }
    }
    if host_str.matches(':').count() == 1 {
        if let Some(colon) = host_str.rfind(':') {
            return host_str[colon + 1..].parse::<u16>().ok();
        }
    }
    None
}

fn is_same_origin(origin_str: &str, host_header: &str) -> bool {
    let Ok(uri) = origin_str.parse::<axum::http::Uri>() else {
        return false;
    };
    let Some(authority) = uri.authority() else {
        return false;
    };
    if authority.as_str().eq_ignore_ascii_case(host_header) {
        return true;
    }
    let Some(origin_host) = uri.host() else {
        return false;
    };
    let host_only = extract_host(host_header);
    if !origin_host.eq_ignore_ascii_case(host_only) {
        return false;
    }
    let origin_port = uri.port_u16().or_else(|| match uri.scheme_str() {
        Some("http") | Some("ws") => Some(80),
        Some("https") | Some("wss") => Some(443),
        _ => None,
    });
    let host_port = extract_port(host_header);
    origin_port == host_port
}

async fn validate_host_and_origin(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let host_str = match request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => h,
        None => match request.uri().authority() {
            Some(a) => a.as_str(),
            None => {
                return (StatusCode::BAD_REQUEST, "missing Host header").into_response();
            }
        },
    };

    let host = extract_host(host_str);
    if !is_loopback_host(host) {
        return (StatusCode::FORBIDDEN, "untrusted Host header").into_response();
    }

    let is_ws = request
        .headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));

    if is_ws {
        let Some(origin_str) = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
        else {
            return (StatusCode::FORBIDDEN, "missing Origin on WebSocket upgrade").into_response();
        };

        if !is_same_origin(origin_str, host_str) {
            return (
                StatusCode::FORBIDDEN,
                "cross-origin WebSocket upgrade rejected",
            )
                .into_response();
        }
    } else if !request.method().is_safe() {
        // A state-changing request carrying a foreign Origin is a cross-site request from
        // a page the operator merely visited; the loopback Host check alone does not stop it.
        if let Some(origin_str) = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
        {
            if !is_same_origin(origin_str, host_str) {
                return (
                    StatusCode::FORBIDDEN,
                    "cross-origin state-changing request rejected",
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

#[derive(Clone, Debug, Serialize)]
pub struct DashboardLogEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Value,
}

#[derive(Clone)]
pub struct DashboardLogStore {
    entries: Arc<Mutex<VecDeque<DashboardLogEntry>>>,
    sender: broadcast::Sender<DashboardLogEntry>,
    sequence: Arc<AtomicU64>,
}

impl Default for DashboardLogStore {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(512);
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_CAPACITY))),
            sender,
            sequence: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl DashboardLogStore {
    fn push(&self, level: &str, target: &str, message: String, fields: Value) {
        // The id, the stored entry and the broadcast are ordered under one lock so
        // subscribers and snapshots always observe strictly increasing ids.
        let mut entries = self.entries.lock();
        let entry = DashboardLogEntry {
            id: self.sequence.fetch_add(1, Ordering::Relaxed),
            timestamp: Utc::now(),
            level: level.to_owned(),
            target: target.to_owned(),
            message,
            fields,
        };
        if entries.len() >= LOG_CAPACITY {
            entries.pop_front();
        }
        entries.push_back(entry.clone());
        let _ = self.sender.send(entry);
    }

    pub fn snapshot(&self) -> Vec<DashboardLogEntry> {
        self.entries.lock().iter().cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DashboardLogEntry> {
        self.sender.subscribe()
    }
}

static DASHBOARD_LOGS: OnceLock<DashboardLogStore> = OnceLock::new();

pub fn global_logs() -> DashboardLogStore {
    DASHBOARD_LOGS.get_or_init(Default::default).clone()
}

struct EventFieldVisitor {
    message: Option<String>,
    fields: Map<String, Value>,
}

impl EventFieldVisitor {
    fn new() -> Self {
        Self {
            message: None,
            fields: Map::new(),
        }
    }
}

impl Visit for EventFieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        } else {
            self.fields
                .insert(field.name().to_owned(), Value::String(value.to_owned()));
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_owned(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_owned(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_owned(), Value::Number(value.into()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(rendered);
        } else {
            self.fields
                .insert(field.name().to_owned(), Value::String(rendered));
        }
    }
}

#[derive(Clone)]
struct DashboardLogLayer {
    logs: DashboardLogStore,
}

impl<S> Layer<S> for DashboardLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = EventFieldVisitor::new();
        event.record(&mut visitor);
        let message = visitor.message.unwrap_or_else(|| {
            visitor
                .fields
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        });
        self.logs.push(
            metadata.level().as_str(),
            metadata.target(),
            message,
            Value::Object(visitor.fields),
        );
    }
}

pub fn init_tracing() {
    let logs = global_logs();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(DashboardLogLayer { logs })
        .init();
}

#[derive(Clone, Debug, Serialize)]
pub struct PendingApprovalView {
    pub id: Uuid,
    pub session: SessionKey,
    pub command: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct WebDashboardDispatcher {
    sender: broadcast::Sender<OutboundAction>,
    pending: Arc<RwLock<HashMap<Uuid, PendingApprovalView>>>,
}

impl Default for WebDashboardDispatcher {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self {
            sender,
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl WebDashboardDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OutboundAction> {
        self.sender.subscribe()
    }

    pub async fn pending_approvals(&self) -> Vec<PendingApprovalView> {
        let mut pending = self
            .pending
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        pending.sort_by_key(|entry| entry.created_at);
        pending
    }
}

#[async_trait]
impl OutboundDispatcher for WebDashboardDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        match &action {
            OutboundAction::ApprovalRequest {
                session,
                request_id,
                command,
                reason,
            } => {
                self.pending.write().await.insert(
                    *request_id,
                    PendingApprovalView {
                        id: *request_id,
                        session: session.clone(),
                        command: command.clone(),
                        reason: reason.clone(),
                        created_at: Utc::now(),
                    },
                );
            }
            OutboundAction::ExpireApproval { request_id } => {
                self.pending.write().await.remove(request_id);
            }
            _ => {}
        }
        // The dashboard can only deliver to web sessions; messenger delivery
        // through it must fail closed so callers never record a false success.
        let undeliverable = match &action {
            OutboundAction::SendMessage { session, .. }
            | OutboundAction::EditMessage { session, .. }
            | OutboundAction::DeleteMessage { session, .. }
            | OutboundAction::UploadFile { session, .. }
            | OutboundAction::Stream { session, .. } => {
                (session.platform != "web").then(|| session.platform.clone())
            }
            _ => None,
        };
        let _ = self.sender.send(action);
        match undeliverable {
            Some(platform) => Err(OmonError::Config(format!(
                "web dashboard dispatcher cannot deliver to {platform} sessions"
            ))),
            None => Ok(()),
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct CompositeDispatcher {
    primary: Arc<dyn OutboundDispatcher>,
    dashboard: WebDashboardDispatcher,
}

#[allow(dead_code)]
impl CompositeDispatcher {
    pub fn new(primary: Arc<dyn OutboundDispatcher>, dashboard: WebDashboardDispatcher) -> Self {
        Self { primary, dashboard }
    }
}

#[async_trait]
impl OutboundDispatcher for CompositeDispatcher {
    async fn owner_thread_is_private(&self, session: &SessionKey) -> bool {
        self.primary.owner_thread_is_private(session).await
    }
    async fn dispatch_direct_reply(
        &self,
        session: SessionKey,
        content: String,
    ) -> omon_gateway::Result<()> {
        let observed = self
            .dashboard
            .dispatch_direct_reply(session.clone(), content.clone())
            .await;
        if session.platform == "web" {
            observed
        } else {
            self.primary.dispatch_direct_reply(session, content).await
        }
    }

    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        let web_only = action_session(&action).is_some_and(|session| session.platform == "web");
        let observed = self.dashboard.dispatch(action.clone()).await;
        if web_only {
            observed
        } else {
            self.primary.dispatch(action).await
        }
    }
}

fn action_session(action: &OutboundAction) -> Option<&SessionKey> {
    match action {
        OutboundAction::SendMessage { session, .. }
        | OutboundAction::EditMessage { session, .. }
        | OutboundAction::DeleteMessage { session, .. }
        | OutboundAction::UploadFile { session, .. }
        | OutboundAction::Stream { session, .. }
        | OutboundAction::Typing { session, .. }
        | OutboundAction::React { session, .. }
        | OutboundAction::ApprovalRequest { session, .. } => Some(session),
        OutboundAction::ExpireApproval { .. } => None,
    }
}

pub type DiskSampler = Arc<dyn Fn(&Path) -> (Option<u64>, Option<u64>) + Send + Sync>;

#[derive(Clone)]
pub struct DashboardState {
    pub pool: SqlitePool,
    pub multiplexer: Option<SessionMultiplexer>,
    pub scheduler: CronScheduler,
    pub tools: ToolRegistry,
    pub approvals: SmartApprovalGuard,
    pub events: WebDashboardDispatcher,
    pub config: Value,
    pub workspace_root: PathBuf,
    pub skill_roots: Vec<PathBuf>,
    pub bot_connections: usize,
    pub started_at: Instant,
    pub web_root: PathBuf,
    pub logs: DashboardLogStore,
    pub disk_sampler: Option<DiskSampler>,
    /// True when this dashboard owns its own multiplexer and approval guard
    /// instead of sharing the Discord gateway's runtime (combined `run` mode
    /// and standalone `dashboard`). Messenger sessions and approvals then live
    /// in a different runtime and must not be driven or reported from here.
    pub detached_from_gateway: bool,
}

#[allow(clippy::too_many_arguments)]
impl DashboardState {
    pub fn new(
        pool: SqlitePool,
        multiplexer: Option<SessionMultiplexer>,
        scheduler: CronScheduler,
        tools: ToolRegistry,
        approvals: SmartApprovalGuard,
        events: WebDashboardDispatcher,
        config: Value,
        workspace_root: PathBuf,
        skill_roots: Vec<PathBuf>,
        bot_connections: usize,
        web_root: PathBuf,
    ) -> Self {
        Self {
            pool,
            multiplexer,
            scheduler,
            tools,
            approvals,
            events,
            config,
            workspace_root,
            skill_roots,
            bot_connections,
            started_at: Instant::now(),
            web_root,
            logs: global_logs(),
            disk_sampler: None,
            detached_from_gateway: false,
        }
    }

    pub fn detached_from_gateway(mut self) -> Self {
        self.detached_from_gateway = true;
        self
    }

    pub fn with_disk_sampler<F>(mut self, sampler: F) -> Self
    where
        F: Fn(&Path) -> (Option<u64>, Option<u64>) + Send + Sync + 'static,
    {
        self.disk_sampler = Some(Arc::new(sampler));
        self
    }

    pub fn sample_disk(&self) -> (Option<u64>, Option<u64>) {
        if let Some(ref sampler) = self.disk_sampler {
            sampler(&self.workspace_root)
        } else {
            (
                fs2::total_space(&self.workspace_root).ok(),
                fs2::available_space(&self.workspace_root).ok(),
            )
        }
    }
}

#[allow(dead_code)]
pub(crate) fn config_view_from_gateway(
    config: &super::Config,
    settings: &DashboardSettings,
) -> Value {
    json!({
        "model": config.default_model,
        "providers": {
            "openai_base_url": config.openai_api_base,
            "openai_api_key_configured": !config.openai_api_key.trim().is_empty(),
            "anthropic_base_url": config.anthropic_base_url,
            "anthropic_api_key_configured": config.anthropic_api_key.as_deref().is_some_and(|value| !value.trim().is_empty()),
        },
        "approval": {
            "policy": format!("{:?}", config.approval_policy),
            "timeout_secs": config.approval_timeout_secs,
            "mention_requesters": config.approval_mentions,
            "deny_patterns": config.approvals_deny,
        },
        "workspace_root": config.workspace_root,
        "tool_roots": config.extra_tool_roots,
        "discord": {
            "bot_count": config.discord_bot_tokens.len(),
            "allowed_users": config.allowed_users,
            "allowed_roles": config.allowed_roles,
            "allowed_channels": config.allowed_channels,
            "ignored_channels": config.ignored_channels,
            "allow_all_users": config.allow_all_users,
            "allow_bots": format!("{:?}", config.allow_bots),
            "auto_thread": config.auto_thread,
            "thread_sessions_per_user": config.thread_sessions_per_user,
            "thread_require_mention": config.thread_require_mention,
        },
        "runtime": {
            "processing_reactions": config.processing_reactions,
            "runtime_footer": config.runtime_footer,
            "cron_script_timeout_secs": config.cron_script_timeout_secs,
        },
        "dashboard": {
            "host": settings.host,
            "port": settings.port,
            "insecure": settings.insecure,
        }
    })
}

pub async fn spawn_server(
    settings: DashboardSettings,
    mut state: DashboardState,
    shutdown: CancellationToken,
) -> Result<JoinHandle<()>, OmonError> {
    settings.validate()?;
    state.web_root = settings.web_root.clone();
    let listener = TcpListener::bind((settings.host.as_str(), settings.port))
        .await
        .map_err(|error| {
            OmonError::Config(format!(
                "failed to bind dashboard on {}: {error}",
                settings.display_address()
            ))
        })?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| OmonError::Config(format!("failed to read dashboard address: {error}")))?;
    tracing::info!(address = %local_addr, "omon dashboard listening");
    let app = router(state);
    Ok(tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await;
        if let Err(error) = result {
            tracing::error!(%error, "dashboard HTTP server stopped with an error");
        }
    }))
}

pub fn router(state: DashboardState) -> Router {
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/health", get(api_health))
        .route("/api/readiness", get(api_readiness))
        .route("/api/ready", get(api_readiness))
        .route("/api/sessions", get(list_sessions))
        .route(
            "/api/sessions/{id}",
            get(get_session).delete(delete_session),
        )
        .route("/api/sessions/{id}/messages", get(list_messages))
        .route("/api/sessions/{id}/chat", post(post_chat))
        .route("/api/sessions/{id}/stop", post(stop_session))
        .route("/api/sessions/{id}/ws", get(session_ws))
        .route("/api/cron/jobs", get(list_cron_jobs).post(create_cron_job))
        .route(
            "/api/cron/jobs/{id}",
            get(get_cron_job)
                .put(update_cron_job)
                .delete(delete_cron_job),
        )
        .route("/api/cron/jobs/{id}/trigger", post(trigger_cron_job))
        .route("/api/cron/jobs/{id}/pause", post(pause_cron_job))
        .route("/api/cron/jobs/{id}/resume", post(resume_cron_job))
        .route("/api/cron/runs", get(list_cron_runs))
        .route("/api/config", get(get_config))
        .route("/api/tools", get(list_tools))
        .route("/api/skills", get(list_skills))
        .route("/api/memory", get(list_memory))
        .route("/api/approvals/pending", get(list_pending_approvals))
        .route("/api/approvals/{id}/resolve", post(resolve_approval))
        .route("/api/approvals/allowlist", get(list_approval_allowlist))
        .route("/api/bots", get(list_bots).post(create_bot))
        .route(
            "/api/bots/{id}",
            get(get_bot).put(update_bot).delete(delete_bot),
        )
        .route("/api/logs", get(list_logs))
        .route("/api/logs/ws", get(logs_ws))
        .fallback(get(serve_static))
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(validate_host_and_origin))
        .with_state(state)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn not_found(resource: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, format!("{resource} not found"))
    }
}

impl From<OmonError> for ApiError {
    fn from(error: OmonError) -> Self {
        match error {
            OmonError::Config(msg) => Self::new(StatusCode::BAD_REQUEST, msg),
            other => Self::new(StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.message,
                "status": self.status.as_u16(),
            })),
        )
            .into_response()
    }
}

async fn api_health(State(state): State<DashboardState>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "time": Utc::now(),
    }))
}

/// Resolves the agent backend URL the same way `OmoBackendConfig::from_env` does:
/// an explicit config value, then `OMON_OMO_APPSERVER_URL`, then the local daemon default.
fn readiness_backend_url(config: &Value, env_url: Option<&str>) -> String {
    config
        .get("appserver_url")
        .and_then(Value::as_str)
        .or(env_url)
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_BACKEND_APPSERVER_URL)
        .to_owned()
}

async fn api_readiness(State(state): State<DashboardState>) -> Response {
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .is_ok();
    let workspace_ok = tokio::fs::metadata(&state.workspace_root).await.is_ok();
    let (disk_total, disk_available) = state.sample_disk();
    let disk_pressure = readiness::classify_disk_pressure_opt(disk_total, disk_available);
    let disk_ok = workspace_ok && disk_pressure == "ok";
    let chat_ok = state.multiplexer.is_some();
    let env_url = std::env::var("OMON_OMO_APPSERVER_URL").ok();
    let backend_configured = state.config.get("appserver_url").is_some() || env_url.is_some();
    let appserver_url = readiness_backend_url(&state.config, env_url.as_deref());
    let backend_check = readiness::probe_backend(Some(appserver_url.as_str())).await;
    let backend_ok = backend_check.status == "ok";
    // The backend gates readiness when this dashboard serves chat, or when a backend was
    // configured explicitly; a bare read-only dashboard does not depend on it.
    let ready = db_ok && workspace_ok && disk_ok && (backend_ok || !(chat_ok || backend_configured));
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if ready { "ready" } else { "degraded" },
            "checks": {
                "database": db_ok,
                "workspace": workspace_ok,
                "disk": disk_ok,
                "chat_runtime": chat_ok,
                "backend": backend_ok,
            },
            "disk": {
                "workspace_total_bytes": disk_total,
                "workspace_available_bytes": disk_available,
                "pressure": disk_pressure,
            }
        })),
    )
        .into_response()
}

async fn api_status(State(state): State<DashboardState>) -> Result<Json<Value>, ApiError> {
    let stored_sessions = scalar_count(&state.pool, "SELECT COUNT(*) FROM sessions").await?;
    let messages = scalar_count(&state.pool, "SELECT COUNT(*) FROM messages").await?;
    let cron_jobs = scalar_count(&state.pool, "SELECT COUNT(*) FROM cron_jobs").await?;
    let memories = scalar_count(&state.pool, "SELECT COUNT(*) FROM memories").await?;
    let active_sessions = state
        .multiplexer
        .as_ref()
        .map_or(0, SessionMultiplexer::active_sessions);
    let (disk_total, disk_available) = state.sample_disk();
    let memory_bytes = process_memory_bytes();
    let (disk_headroom_status, disk_used_percent, disk_pressure) =
        match (disk_total, disk_available) {
            (Some(total), Some(avail)) => {
                let (status, pct, pressure) = readiness::calculate_disk_headroom(
                    total,
                    avail,
                    readiness::DISK_DEGRADED_PERCENT,
                );
                (status, Some(pct), pressure)
            }
            _ => ("degraded".to_string(), None, "unknown".to_string()),
        };
    let total_mb = disk_total.map(|t| t / readiness::DISK_BYTES_PER_MB);
    let available_mb = disk_available.map(|a| a / readiness::DISK_BYTES_PER_MB);
    let overall_status = if disk_headroom_status == "ok" {
        "ok"
    } else {
        "degraded"
    };
    Ok(Json(json!({
        "status": overall_status,
        "model": state.config["model"].clone(),
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "bot_connections": state.bot_connections,
        "active_sessions": active_sessions,
        "chat_available": state.multiplexer.is_some(),
        "database": {
            "sessions": stored_sessions,
            "messages": messages,
            "cron_jobs": cron_jobs,
            "memories": memories,
            "pool_size": state.pool.size(),
            "pool_idle": state.pool.num_idle(),
        },
        "memory": {
            "process_bytes": memory_bytes,
        },
        "disk": {
            "status": disk_headroom_status,
            "workspace_total_bytes": disk_total,
            "workspace_available_bytes": disk_available,
            "total_bytes": disk_total,
            "available_bytes": disk_available,
            "total_mb": total_mb,
            "available_mb": available_mb,
            "free_mb": available_mb,
            "used_percent": disk_used_percent,
            "pressure": disk_pressure,
        },
        "pending_approvals": state.approvals.pending_count().await,
        "time": Utc::now(),
    })))
}

async fn scalar_count(pool: &SqlitePool, sql: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await
}

fn process_memory_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        // SAFETY: getrusage writes a fully initialized rusage structure when it returns 0.
        let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if result != 0 {
            return None;
        }
        // SAFETY: a successful getrusage call initialized the value above.
        let usage = unsafe { usage.assume_init() };
        #[cfg(target_os = "macos")]
        {
            Some(usage.ru_maxrss as u64)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Some((usage.ru_maxrss as u64).saturating_mul(1024))
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[derive(Clone, Debug, Serialize, FromRow)]
struct SessionRow {
    session_key: String,
    platform: String,
    guild_id: Option<String>,
    channel_id: String,
    thread_id: Option<String>,
    user_id: String,
    state_json: String,
    created_at: String,
    updated_at: String,
}

impl SessionRow {
    fn view(self) -> Value {
        json!({
            "id": self.session_key,
            "platform": self.platform,
            "guild_id": self.guild_id,
            "channel_id": self.channel_id,
            "thread_id": self.thread_id,
            "user_id": self.user_id,
            "state": serde_json::from_str::<Value>(&self.state_json).unwrap_or_else(|_| json!({})),
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        })
    }
}

/// Builds a `LIKE '%value%'` pattern that matches `value` literally: `%`, `_` and the
/// escape character itself are escaped (queries must use `ESCAPE '\\'`).
fn like_contains_pattern(value: &str) -> String {
    let mut pattern = String::with_capacity(value.len() + 2);
    pattern.push('%');
    for ch in value.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    pattern.push('%');
    pattern
}

#[derive(Debug, Deserialize)]
struct PageQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
}

impl PageQuery {
    fn values(&self) -> (u32, u32, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
        let offset = i64::from((page - 1).saturating_mul(per_page));
        (page, per_page, offset)
    }
}

async fn list_sessions(
    State(state): State<DashboardState>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (page, per_page, offset) = query.values();
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(like_contains_pattern);

    let (rows, total) = if let Some(pattern) = search {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, created_at, updated_at
             FROM sessions
             WHERE session_key LIKE ? ESCAPE '\\' OR platform LIKE ? ESCAPE '\\' OR channel_id LIKE ? ESCAPE '\\' OR user_id LIKE ? ESCAPE '\\'
             ORDER BY updated_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(i64::from(per_page))
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sessions
             WHERE session_key LIKE ? ESCAPE '\\' OR platform LIKE ? ESCAPE '\\' OR channel_id LIKE ? ESCAPE '\\' OR user_id LIKE ? ESCAPE '\\'",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.pool)
        .await?;
        (rows, total)
    } else {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, created_at, updated_at
             FROM sessions ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(i64::from(per_page))
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;
        let total = scalar_count(&state.pool, "SELECT COUNT(*) FROM sessions").await?;
        (rows, total)
    };

    Ok(Json(json!({
        "items": rows.into_iter().map(SessionRow::view).collect::<Vec<_>>(),
        "page": page,
        "per_page": per_page,
        "total": total,
    })))
}

async fn get_session(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let storage_id = resolve_storage_id(&state.pool, &id).await?;
    let row = fetch_session_row(&state.pool, &storage_id)
        .await?
        .ok_or_else(|| ApiError::not_found("session"))?;
    let message_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE session_key = ?")
            .bind(&storage_id)
            .fetch_one(&state.pool)
            .await?;
    let mut view = row.view();
    if let Value::Object(ref mut object) = view {
        object.insert("message_count".into(), Value::Number(message_count.into()));
        object.insert(
            "active".into(),
            Value::Bool(state.multiplexer.as_ref().is_some_and(|mux| {
                parse_storage_key(&storage_id).is_some_and(|key| mux.contains_session(&key))
            })),
        );
    }
    Ok(Json(view))
}

async fn fetch_session_row(
    pool: &SqlitePool,
    storage_id: &str,
) -> Result<Option<SessionRow>, sqlx::Error> {
    sqlx::query_as::<_, SessionRow>(
        "SELECT session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, created_at, updated_at
         FROM sessions WHERE session_key = ?",
    )
    .bind(storage_id)
    .fetch_optional(pool)
    .await
}

#[derive(Debug, Serialize, FromRow)]
struct MessageRow {
    sequence: i64,
    id: String,
    role: String,
    content: String,
    metadata_json: String,
    created_at: String,
}

async fn list_messages(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let storage_id = resolve_storage_id(&state.pool, &id).await?;
    if fetch_session_row(&state.pool, &storage_id).await?.is_none() {
        return Err(ApiError::not_found("session"));
    }
    let (page, per_page, offset) = query.values();
    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT sequence, id, role, content, metadata_json, created_at
         FROM messages WHERE session_key = ? ORDER BY sequence ASC LIMIT ? OFFSET ?",
    )
    .bind(&storage_id)
    .bind(i64::from(per_page))
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE session_key = ?")
        .bind(&storage_id)
        .fetch_one(&state.pool)
        .await?;
    let items = rows
        .into_iter()
        .map(|row| {
            json!({
                "sequence": row.sequence,
                "id": row.id,
                "role": row.role,
                "content": row.content,
                "metadata": serde_json::from_str::<Value>(&row.metadata_json).unwrap_or_else(|_| json!({})),
                "created_at": row.created_at,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "items": items,
        "page": page,
        "per_page": per_page,
        "total": total,
    })))
}

async fn delete_session(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response, ApiError> {
    let storage_id = resolve_storage_id(&state.pool, &id).await?;
    if let (Some(mux), Some(key)) = (&state.multiplexer, parse_storage_key(&storage_id)) {
        let _ = mux.stop(&key).await;
    }
    let result = sqlx::query("DELETE FROM sessions WHERE session_key = ?")
        .bind(&storage_id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("session"));
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatRequest {
    #[serde(alias = "content")]
    pub(crate) message: String,
}

async fn post_chat(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<ChatRequest>,
) -> Result<Response, ApiError> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "message must not be empty",
        ));
    }
    let mux = state.multiplexer.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "chat runtime is not configured; set DEFAULT_MODEL and provider credentials",
        )
    })?;
    let key = resolve_session_key(&state.pool, &id).await?;
    reject_detached_messenger_session(&state, &key)?;
    let event = InboundEvent::message(key.clone(), Uuid::new_v4().to_string(), message);
    let owner = omon_gateway::sessions::status_reply::owner_id();
    let private_owner = omon_gateway::sessions::status_reply::owner_dm(&key, &owner, &owner);
    let private_owner = private_owner
        || (omon_gateway::sessions::status_reply::matches_intent(message)
            && omon_gateway::sessions::status_reply::owner_thread(&key, &owner, &owner)
            && mux.owner_thread_is_private(&key).await);
    if mux
        .intercept_session_status(&event, private_owner)
        .await
        .map_err(ApiError::from)?
    {
        return Ok((
            StatusCode::OK,
            Json(json!({"queued": false, "direct_reply": true, "session_id": key.storage_key()})),
        )
            .into_response());
    }
    mux.route(event).await.map_err(ApiError::from)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "queued": true,
            "session_id": key.storage_key(),
        })),
    )
        .into_response())
}

async fn stop_session(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let key = resolve_session_key(&state.pool, &id).await?;
    reject_detached_messenger_session(&state, &key)?;
    let Some(mux) = state.multiplexer.as_ref() else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "chat runtime is not configured",
        ));
    };
    let stopped = mux.stop(&key).await.map_err(ApiError::from)?;
    Ok(Json(json!({
        "stopped": stopped,
        "session_id": key.storage_key(),
    })))
}

async fn session_ws(
    ws: WebSocketUpgrade,
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response, ApiError> {
    let key = resolve_session_key(&state.pool, &id).await?;
    let storage_id = key.storage_key();
    Ok(ws
        .on_upgrade(move |socket| handle_session_socket(socket, state, key, storage_id))
        .into_response())
}

async fn handle_session_socket(
    mut socket: WebSocket,
    state: DashboardState,
    key: SessionKey,
    storage_id: String,
) {
    let mut events = state.events.subscribe();
    let mut tracked_approvals: HashSet<Uuid> = state
        .events
        .pending_approvals()
        .await
        .into_iter()
        .filter(|approval| approval.session.storage_key() == storage_id)
        .map(|approval| approval.id)
        .collect();
    let ready = json!({
        "type": "ready",
        "session_id": storage_id,
        "chat_available": state.multiplexer.is_some(),
    });
    if socket
        .send(WsMessage::Text(ready.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(incoming) = incoming else { break; };
                let Ok(message) = incoming else { break; };
                match message {
                    WsMessage::Text(text) => {
                        let text = text.as_str();
                        let parsed = serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({"type":"message","content":text}));
                        match parsed.get("type").and_then(Value::as_str).unwrap_or("message") {
                            "message" | "chat" => {
                                let content = parsed.get("content").and_then(Value::as_str).unwrap_or_default().trim();
                                if content.is_empty() {
                                    let _ = send_ws_error(&mut socket, "message must not be empty").await;
                                    continue;
                                }
                                if let Err(error) = reject_detached_messenger_session(&state, &key) {
                                    let _ = send_ws_error(&mut socket, &error.message).await;
                                    continue;
                                }
                                let Some(mux) = state.multiplexer.as_ref() else {
                                    let _ = send_ws_error(&mut socket, "chat runtime is not configured").await;
                                    continue;
                                };
                                let event = InboundEvent::message(key.clone(), Uuid::new_v4().to_string(), content);
                                let owner = omon_gateway::sessions::status_reply::owner_id();
                                let private_owner = omon_gateway::sessions::status_reply::owner_dm(&key, &owner, &owner);
                                let private_owner = private_owner || (omon_gateway::sessions::status_reply::matches_intent(content)
                                    && omon_gateway::sessions::status_reply::owner_thread(&key, &owner, &owner)
                                    && mux.owner_thread_is_private(&key).await);
                                match mux.intercept_session_status(&event, private_owner).await {
                                    Ok(true) => {
                                        let ack = json!({"type":"accepted","direct_reply":true,"session_id":key.storage_key()});
                                        if socket.send(WsMessage::Text(ack.to_string().into())).await.is_err() { break; }
                                        continue;
                                    }
                                    Ok(false) => {}
                                    Err(error) => { let _ = send_ws_error(&mut socket, &error.to_string()).await; continue; }
                                }
                                if let Err(error) = mux.route(event).await {
                                    let _ = send_ws_error(&mut socket, &error.to_string()).await;
                                } else {
                                    let ack = json!({"type":"accepted","session_id":key.storage_key()});
                                    if socket.send(WsMessage::Text(ack.to_string().into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            "stop" => {
                                if let Err(error) = reject_detached_messenger_session(&state, &key) {
                                    let _ = send_ws_error(&mut socket, &error.message).await;
                                } else if let Some(mux) = state.multiplexer.as_ref() {
                                    match mux.stop(&key).await {
                                        Ok(stopped) => {
                                            let ack = json!({"type":"stopped","stopped":stopped});
                                            if socket.send(WsMessage::Text(ack.to_string().into())).await.is_err() { break; }
                                        }
                                        Err(error) => { let _ = send_ws_error(&mut socket, &error.to_string()).await; }
                                    }
                                }
                            }
                            "ping" => {
                                if socket.send(WsMessage::Text(json!({"type":"pong"}).to_string().into())).await.is_err() { break; }
                            }
                            other => {
                                let _ = send_ws_error(&mut socket, &format!("unsupported websocket message type {other}")).await;
                            }
                        }
                    }
                    WsMessage::Ping(payload) => {
                        if socket.send(WsMessage::Pong(payload)).await.is_err() { break; }
                    }
                    WsMessage::Close(_) => break,
                    _ => {}
                }
            }
            outbound = events.recv() => {
                match outbound {
                    Ok(action) => {
                        if session_event_matches(&action, &storage_id, &mut tracked_approvals) {
                            let payload = json!({"type":"event","event":action});
                            if socket.send(WsMessage::Text(payload.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let warning = json!({"type":"warning","message":format!("skipped {skipped} dashboard events")});
                        if socket.send(WsMessage::Text(warning.to_string().into())).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

/// Whether `action` belongs on the websocket of session `storage_id`. An
/// `ExpireApproval` carries no session, so expiries are matched against the approval
/// requests this socket has seen (or found pending when it connected).
fn session_event_matches(
    action: &OutboundAction,
    storage_id: &str,
    tracked_approvals: &mut HashSet<Uuid>,
) -> bool {
    match action {
        OutboundAction::ApprovalRequest {
            session,
            request_id,
            ..
        } if session.storage_key() == storage_id => {
            tracked_approvals.insert(*request_id);
            true
        }
        OutboundAction::ExpireApproval { request_id } => tracked_approvals.remove(request_id),
        other => action_session(other).is_some_and(|session| session.storage_key() == storage_id),
    }
}

async fn send_ws_error(socket: &mut WebSocket, message: &str) -> Result<(), axum::Error> {
    socket
        .send(WsMessage::Text(
            json!({"type":"error","message":message}).to_string().into(),
        ))
        .await
}

/// A dashboard with its own runtime must not drive messenger sessions: their
/// replies would only reach the web broadcast, never the messenger.
fn reject_detached_messenger_session(
    state: &DashboardState,
    key: &SessionKey,
) -> Result<(), ApiError> {
    if state.detached_from_gateway && key.platform != "web" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            format!(
                "{} sessions are owned by the messenger gateway runtime and cannot be chatted with or stopped from this dashboard",
                key.platform
            ),
        ));
    }
    Ok(())
}

fn web_session_key(id: &str) -> SessionKey {
    SessionKey::new("web", None::<String>, id, None::<String>, "dashboard")
}

async fn resolve_storage_id(pool: &SqlitePool, id: &str) -> Result<String, sqlx::Error> {
    let exact = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE session_key = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if exact != 0 {
        return Ok(id.to_owned());
    }
    Ok(web_session_key(id).storage_key())
}

async fn resolve_session_key(pool: &SqlitePool, id: &str) -> Result<SessionKey, ApiError> {
    let storage_id = resolve_storage_id(pool, id).await?;
    if let Some(key) = parse_storage_key(&storage_id) {
        return Ok(key);
    }
    if let Some(row) = fetch_session_row(pool, &storage_id).await? {
        return Ok(SessionKey::new(
            row.platform,
            row.guild_id,
            row.channel_id,
            row.thread_id,
            row.user_id,
        ));
    }
    Ok(web_session_key(id))
}

fn parse_storage_key(input: &str) -> Option<SessionKey> {
    fn read_component(input: &str, index: &mut usize) -> Option<Option<String>> {
        let bytes = input.as_bytes();
        if *index >= bytes.len() {
            return None;
        }
        if bytes[*index] == b'-' {
            *index += 1;
            return Some(None);
        }
        let length_start = *index;
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
        if *index == length_start || bytes.get(*index) != Some(&b':') {
            return None;
        }
        let length = input.get(length_start..*index)?.parse::<usize>().ok()?;
        *index += 1;
        let end = index.checked_add(length)?;
        let value = input.get(*index..end)?.to_owned();
        *index = end;
        Some(Some(value))
    }

    let mut index = 0usize;
    let mut values = Vec::new();
    while index < input.len() {
        values.push(read_component(input, &mut index)?);
        if index == input.len() {
            break;
        }
        if input.as_bytes().get(index) != Some(&b'|') {
            return None;
        }
        index += 1;
    }
    if values.len() != 5 && values.len() != 6 {
        return None;
    }
    Some(SessionKey {
        platform: values.first()?.clone()?,
        guild_id: values.get(1)?.clone(),
        channel_id: values.get(2)?.clone()?,
        thread_id: values.get(3)?.clone(),
        user_id: values.get(4)?.clone()?,
        bot_id: values.get(5).cloned().flatten(),
    })
}

#[derive(Debug, Deserialize)]
struct CronJobInput {
    id: Option<String>,
    expression: String,
    #[serde(default)]
    payload: Value,
    session_key: Option<String>,
    enabled: Option<bool>,
}

/// Partial update: every omitted field keeps the stored value.
#[derive(Debug, Deserialize)]
struct CronJobUpdateInput {
    expression: Option<String>,
    payload: Option<Value>,
    session_key: Option<String>,
    enabled: Option<bool>,
}

/// Payload keys the scheduler maintains while a job runs; an edit must not reset them.
const SCHEDULER_OWNED_PAYLOAD_KEYS: [&str; 5] = [
    "last_status",
    "last_run_at",
    "last_occurrence",
    "last_error",
    "last_delivery_error",
];

fn preserve_scheduler_state(existing: &Value, updated: &mut Value) {
    let (Some(old), Some(new)) = (existing.as_object(), updated.as_object_mut()) else {
        return;
    };
    for key in SCHEDULER_OWNED_PAYLOAD_KEYS {
        if let Some(value) = old.get(key) {
            if !new.contains_key(key) {
                new.insert(key.to_owned(), value.clone());
            }
        }
    }
    if let Some(done) = old.get("repeat").and_then(|repeat| repeat.get("completed")) {
        if let Some(repeat) = new.get_mut("repeat").and_then(Value::as_object_mut) {
            repeat.entry("completed").or_insert_with(|| done.clone());
        }
    }
    if let Some(done) = old.get("completed") {
        if !new.contains_key("completed") && !new.contains_key("repeat") {
            new.insert("completed".to_owned(), done.clone());
        }
    }
}

async fn list_cron_jobs(State(state): State<DashboardState>) -> Result<Json<Value>, ApiError> {
    let jobs = sqlx::query_as::<_, CronJob>(
        "SELECT * FROM cron_jobs ORDER BY enabled DESC, next_run_at IS NULL, next_run_at, id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "items": jobs.into_iter().map(cron_job_view).collect::<Vec<_>>(),
    })))
}

async fn create_cron_job(
    State(state): State<DashboardState>,
    Json(input): Json<CronJobInput>,
) -> Result<Response, ApiError> {
    if input.expression.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "expression must not be empty",
        ));
    }
    if input.id.as_deref().is_some_and(|id| id.trim().is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "id must not be empty",
        ));
    }
    if input.payload.is_null() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "payload is required",
        ));
    }
    let spec = CronJobSpec {
        expression: input.expression,
        payload: input.payload,
        session_key: input.session_key,
    };
    let enabled = input.enabled != Some(false);
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let job = state
        .scheduler
        .register_new(id.clone(), spec, enabled)
        .await?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                format!("cron job {id} already exists"),
            )
        })?;
    let job = state
        .scheduler
        .get(&job.id)
        .await?
        .ok_or_else(|| ApiError::not_found("cron job"))?;
    Ok((StatusCode::CREATED, Json(cron_job_view(job))).into_response())
}

async fn get_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let job = state
        .scheduler
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("cron job"))?;
    Ok(Json(cron_job_view(job)))
}

async fn update_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<CronJobUpdateInput>,
) -> Result<Json<Value>, ApiError> {
    let existing = state
        .scheduler
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("cron job"))?;
    let expression = match input.expression {
        Some(expression) if expression.trim().is_empty() => {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "expression must not be empty",
            ));
        }
        Some(expression) => expression,
        None => existing.expression.clone(),
    };
    let existing_payload = existing.payload();
    let payload = match input.payload {
        Some(mut payload) => {
            if let Ok(old) = &existing_payload {
                preserve_scheduler_state(old, &mut payload);
            }
            payload
        }
        None => existing_payload?,
    };
    // An omitted session_key keeps the stored binding; an empty one clears it.
    let session_key = match input.session_key {
        Some(key) if key.trim().is_empty() => None,
        Some(key) => Some(key),
        None => existing.session_key.clone(),
    };
    let spec = CronJobSpec {
        expression,
        payload,
        session_key,
    };
    state.scheduler.register_with_id(&id, spec).await?;
    match input.enabled {
        Some(true) => {
            state.scheduler.resume(&id).await?;
        }
        // register_with_id schedules a next run; a paused job must not keep one.
        Some(false) => {
            state.scheduler.pause(&id).await?;
        }
        None if !existing.enabled => {
            state.scheduler.pause(&id).await?;
        }
        None => {}
    }
    let job = state
        .scheduler
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("cron job"))?;
    Ok(Json(cron_job_view(job)))
}

async fn delete_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response, ApiError> {
    if !state.scheduler.delete(&id).await? {
        return Err(ApiError::not_found("cron job"));
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn trigger_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let job = state
        .scheduler
        .get(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("cron job"))?;
    // A detached dashboard only has the web dispatcher: the agent would run, then
    // delivery to Discord would fail. Leave those jobs to the gateway's scheduler.
    if state.detached_from_gateway {
        let has_messenger_destination = job
            .payload()
            .ok()
            .and_then(|payload| omon_gateway::delivery_destinations(&payload).ok())
            .is_some_and(|destinations| !destinations.is_empty());
        if has_messenger_destination {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "this cron job delivers to a messenger; it must be triggered from the gateway runtime, not a detached dashboard",
            ));
        }
    }
    if !state.scheduler.trigger(&id).await? {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "cron job is already running or unavailable",
        ));
    }
    Ok(Json(json!({"triggered": true, "id": id})))
}

async fn pause_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    if !state.scheduler.pause(&id).await? {
        return Err(ApiError::not_found("cron job"));
    }
    Ok(Json(json!({"paused": true, "id": id})))
}

async fn resume_cron_job(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    if !state.scheduler.resume(&id).await? {
        return Err(ApiError::not_found("cron job"));
    }
    Ok(Json(json!({"resumed": true, "id": id})))
}

/// A job whose stored payload is not valid JSON is still listed (payload null plus an
/// `error` field) so the operator can find, repair or delete it.
fn cron_job_view(job: CronJob) -> Value {
    let (payload, error) = match job.payload() {
        Ok(payload) => (payload, Value::Null),
        Err(error) => (Value::Null, Value::String(error.to_string())),
    };
    json!({
        "id": job.id,
        "session_key": job.session_key,
        "expression": job.expression,
        "payload": payload,
        "enabled": job.enabled,
        "next_run_at": job.next_run_at,
        "created_at": job.created_at,
        "updated_at": job.updated_at,
        "error": error,
    })
}

#[derive(Debug, Serialize, FromRow)]
struct CronRunRow {
    run_id: String,
    job_id: String,
    lease_expires_at: String,
    started_at: String,
    completed_at: Option<String>,
    status: String,
    attempt: i64,
    error: Option<String>,
    owner_pid: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CronRunsQuery {
    job_id: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
}

async fn list_cron_runs(
    State(state): State<DashboardState>,
    Query(query): Query<CronRunsQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let rows = match (query.job_id.as_deref(), query.status.as_deref()) {
        (Some(job_id), Some(status)) => {
            sqlx::query_as::<_, CronRunRow>(
                "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid
                 FROM cron_runs WHERE job_id = ? AND status = ? ORDER BY started_at DESC LIMIT ?",
            )
            .bind(job_id)
            .bind(status)
            .bind(i64::from(limit))
            .fetch_all(&state.pool)
            .await?
        }
        (Some(job_id), None) => {
            sqlx::query_as::<_, CronRunRow>(
                "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid
                 FROM cron_runs WHERE job_id = ? ORDER BY started_at DESC LIMIT ?",
            )
            .bind(job_id)
            .bind(i64::from(limit))
            .fetch_all(&state.pool)
            .await?
        }
        (None, Some(status)) => {
            sqlx::query_as::<_, CronRunRow>(
                "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid
                 FROM cron_runs WHERE status = ? ORDER BY started_at DESC LIMIT ?",
            )
            .bind(status)
            .bind(i64::from(limit))
            .fetch_all(&state.pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, CronRunRow>(
                "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid
                 FROM cron_runs ORDER BY started_at DESC LIMIT ?",
            )
            .bind(i64::from(limit))
            .fetch_all(&state.pool)
            .await?
        }
    };
    Ok(Json(json!({"items": rows})))
}

async fn get_config(State(state): State<DashboardState>) -> Json<Value> {
    Json(state.config)
}

async fn list_tools(State(state): State<DashboardState>) -> Json<Value> {
    let mut names = state.tools.names();
    names.sort();
    let items = names
        .into_iter()
        .filter_map(|name| {
            state.tools.get(&name).map(|tool| {
                json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.input_schema(),
                })
            })
        })
        .collect::<Vec<_>>();
    Json(json!({"items": items}))
}

#[derive(Clone, Debug, Serialize)]
struct SkillView {
    name: String,
    source: String,
    path: String,
    description: Option<String>,
}

async fn list_skills(State(state): State<DashboardState>) -> Result<Json<Value>, ApiError> {
    let roots = state.skill_roots.clone();
    let items = tokio::task::spawn_blocking(move || discover_skills(&roots))
        .await
        .map_err(|error| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("skill discovery failed: {error}"),
            )
        })?;
    Ok(Json(json!({"items": items})))
}

/// Only the head of a SKILL.md is needed for its description.
const SKILL_HEAD_BYTES: u64 = 8 * 1024;

fn read_skill_head(path: &Path) -> String {
    use std::io::Read;
    let Ok(file) = std::fs::File::open(path) else {
        return String::new();
    };
    let mut buffer = Vec::new();
    if file
        .take(SKILL_HEAD_BYTES)
        .read_to_end(&mut buffer)
        .is_err()
    {
        return String::new();
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

fn discover_skills(roots: &[PathBuf]) -> Vec<SkillView> {
    let mut seen = HashSet::new();
    let mut skills = Vec::new();
    for root in roots {
        collect_skills(root, root, 0, &mut seen, &mut skills);
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.path.cmp(&b.path)));
    skills
}

fn collect_skills(
    source_root: &Path,
    current: &Path,
    depth: usize,
    seen: &mut HashSet<PathBuf>,
    skills: &mut Vec<SkillView>,
) {
    if depth > 4 || !current.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_skills(source_root, &path, depth + 1, seen, skills);
            continue;
        }
        let is_skill = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("skill.md"));
        if !is_skill {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(canonical) {
            continue;
        }
        let content = read_skill_head(&path);
        let description = content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("---"))
            .map(|line| line.chars().take(240).collect::<String>());
        let name = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("skill")
            .to_owned();
        skills.push(SkillView {
            name,
            source: source_root.display().to_string(),
            path: path.display().to_string(),
            description,
        });
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BotProfileRow {
    pub bot_id: String,
    pub name: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub enabled_toolsets: Option<String>,
    pub custom_settings_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct UpdateBotPayload {
    name: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    enabled_toolsets: Option<Vec<String>>,
    custom_settings: Option<Value>,
}

async fn list_bots(State(state): State<DashboardState>) -> Result<Json<Value>, ApiError> {
    let profiles = sqlx::query_as::<_, BotProfileRow>(
        "SELECT bot_id, name, model, system_prompt, enabled_toolsets, custom_settings_json, created_at, updated_at FROM bot_profiles ORDER BY name ASC"
    )
    .fetch_all(&state.pool)
    .await?;

    let items = profiles
        .into_iter()
        .map(|b| {
            json!({
                "bot_id": b.bot_id,
                "name": b.name,
                "model": b.model,
                "system_prompt": b.system_prompt,
                "enabled_toolsets": b.enabled_toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()),
                "custom_settings": serde_json::from_str::<Value>(&b.custom_settings_json).unwrap_or_else(|_| json!({})),
                "created_at": b.created_at,
                "updated_at": b.updated_at,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({
        "items": items,
        "total": items.len(),
        "runtime_connections": state.bot_connections,
    })))
}

async fn get_bot(
    State(state): State<DashboardState>,
    AxumPath(bot_id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let profile = sqlx::query_as::<_, BotProfileRow>(
        "SELECT bot_id, name, model, system_prompt, enabled_toolsets, custom_settings_json, created_at, updated_at FROM bot_profiles WHERE bot_id = ?"
    )
    .bind(&bot_id)
    .fetch_optional(&state.pool)
    .await?;

    let b = profile.ok_or_else(|| ApiError::not_found("bot profile"))?;
    let item = json!({
        "bot_id": b.bot_id,
        "name": b.name,
        "model": b.model,
        "system_prompt": b.system_prompt,
        "enabled_toolsets": b.enabled_toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>()),
        "custom_settings": serde_json::from_str::<Value>(&b.custom_settings_json).unwrap_or_else(|_| json!({})),
        "created_at": b.created_at,
        "updated_at": b.updated_at,
    });

    Ok(Json(item))
}

#[derive(Debug, Deserialize)]
struct CreateBotPayload {
    bot_id: String,
    name: String,
    model: Option<String>,
    system_prompt: Option<String>,
    enabled_toolsets: Option<Vec<String>>,
    custom_settings: Option<Value>,
}

async fn create_bot(
    State(state): State<DashboardState>,
    Json(payload): Json<CreateBotPayload>,
) -> Result<Json<Value>, ApiError> {
    let bot_id = payload.bot_id.trim();
    if bot_id.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "bot_id cannot be empty",
        ));
    }
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "name cannot be empty",
        ));
    }
    // Blank values mean "use the default", exactly like update_bot: store NULL, never "".
    let model = payload.model.filter(|m| !m.trim().is_empty());
    let system_prompt = payload.system_prompt.filter(|p| !p.trim().is_empty());
    let toolsets_str = payload
        .enabled_toolsets
        .filter(|ts| !ts.is_empty())
        .map(|ts| ts.join(","));
    let settings_json = payload
        .custom_settings
        .map(|cs| cs.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let now = Utc::now().to_rfc3339();

    // Create never overwrites: an existing profile is changed through PUT /api/bots/{id}.
    let inserted = sqlx::query(
        "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets, custom_settings_json, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(bot_id) DO NOTHING"
    )
    .bind(bot_id)
    .bind(name)
    .bind(&model)
    .bind(&system_prompt)
    .bind(&toolsets_str)
    .bind(&settings_json)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            format!("bot profile {bot_id} already exists"),
        ));
    }

    Ok(Json(json!({
        "status": "created",
        "bot_id": bot_id,
        "name": name,
    })))
}

async fn delete_bot(
    State(state): State<DashboardState>,
    AxumPath(bot_id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let result = sqlx::query("DELETE FROM bot_profiles WHERE bot_id = ?")
        .bind(&bot_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("bot profile"));
    }

    Ok(Json(json!({
        "status": "deleted",
        "bot_id": bot_id,
    })))
}

async fn update_bot(
    State(state): State<DashboardState>,
    AxumPath(bot_id): AxumPath<String>,
    Json(payload): Json<UpdateBotPayload>,
) -> Result<Json<Value>, ApiError> {
    let name = match payload.name.as_deref().map(str::trim) {
        Some("") => {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "name cannot be empty",
            ));
        }
        other => other.map(str::to_owned),
    };
    // An omitted optional field must retain the stored value; an explicitly empty value
    // clears the field to SQL NULL ("use the default"). Each field is applied by one
    // UPDATE (flag + value, CASE keeps the column when the flag is false) so concurrent
    // updates of different fields cannot overwrite each other.
    let (set_model, model) = match payload.model {
        Some(m) if m.trim().is_empty() => (true, None),
        Some(m) => (true, Some(m)),
        None => (false, None),
    };
    let (set_prompt, system_prompt) = match payload.system_prompt {
        Some(p) if p.trim().is_empty() => (true, None),
        Some(p) => (true, Some(p)),
        None => (false, None),
    };
    let (set_toolsets, toolsets_str) = match payload.enabled_toolsets {
        Some(ts) if ts.is_empty() => (true, None),
        Some(ts) => (true, Some(ts.join(","))),
        None => (false, None),
    };
    let (set_settings, settings_json) = match payload.custom_settings {
        Some(cs) => (true, Some(cs.to_string())),
        None => (false, None),
    };
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE bot_profiles SET
            name = COALESCE(?, name),
            model = CASE WHEN ? THEN ? ELSE model END,
            system_prompt = CASE WHEN ? THEN ? ELSE system_prompt END,
            enabled_toolsets = CASE WHEN ? THEN ? ELSE enabled_toolsets END,
            custom_settings_json = CASE WHEN ? THEN COALESCE(?, custom_settings_json) ELSE custom_settings_json END,
            updated_at = ?
         WHERE bot_id = ?",
    )
    .bind(&name)
    .bind(set_model)
    .bind(&model)
    .bind(set_prompt)
    .bind(&system_prompt)
    .bind(set_toolsets)
    .bind(&toolsets_str)
    .bind(set_settings)
    .bind(&settings_json)
    .bind(&now)
    .bind(&bot_id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("bot profile"));
    }

    get_bot(State(state), AxumPath(bot_id)).await
}

#[derive(Debug, Serialize, FromRow)]
struct MemoryRow {
    id: String,
    session_key: String,
    content: String,
    metadata_json: String,
    created_at: String,
    updated_at: String,
}

async fn list_memory(
    State(state): State<DashboardState>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    let (page, per_page, offset) = query.values();
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(like_contains_pattern);
    let (rows, total) = if let Some(pattern) = search {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, session_key, content, metadata_json, created_at, updated_at FROM memories
             WHERE content LIKE ? ESCAPE '\\' OR session_key LIKE ? ESCAPE '\\' ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(i64::from(per_page))
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM memories WHERE content LIKE ? ESCAPE '\\' OR session_key LIKE ? ESCAPE '\\'",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.pool)
        .await?;
        (rows, total)
    } else {
        let rows = sqlx::query_as::<_, MemoryRow>(
            "SELECT id, session_key, content, metadata_json, created_at, updated_at FROM memories
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(i64::from(per_page))
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;
        let total = scalar_count(&state.pool, "SELECT COUNT(*) FROM memories").await?;
        (rows, total)
    };
    let items = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "session_key": row.session_key,
                "content": row.content,
                "metadata": serde_json::from_str::<Value>(&row.metadata_json).unwrap_or_else(|_| json!({})),
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "items": items,
        "page": page,
        "per_page": per_page,
        "total": total,
    })))
}

const DETACHED_APPROVALS_WARNING: &str = "this dashboard runs its own approval guard; approvals pending in the messenger gateway are not listed here and must be resolved in the messenger";

async fn list_pending_approvals(State(state): State<DashboardState>) -> Json<Value> {
    let mut body = json!({
        "items": state.events.pending_approvals().await,
        "pending_count": state.approvals.pending_count().await,
    });
    if state.detached_from_gateway {
        body["complete"] = json!(false);
        body["gateway_approvals_visible"] = json!(false);
        body["warning"] = json!(DETACHED_APPROVALS_WARNING);
    }
    Json(body)
}

#[derive(Debug, Deserialize)]
struct ResolveApprovalRequest {
    decision: String,
}

async fn resolve_approval(
    State(state): State<DashboardState>,
    AxumPath(id): AxumPath<Uuid>,
    Json(request): Json<ResolveApprovalRequest>,
) -> Result<Json<Value>, ApiError> {
    let suffix = match request.decision.trim().to_ascii_lowercase().as_str() {
        "once" | "allow_once" => "once",
        "session" | "allow_session" => "session",
        "always" | "allow_always" => "always",
        "deny" | "reject" => "deny",
        _ => {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "decision must be one of Once, Session, Always, or Deny",
            ));
        }
    };
    let custom_id = format!("omon:approval:{id}:{suffix}");
    if !state.approvals.resolve_custom_id(&custom_id).await {
        if state.detached_from_gateway {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                format!(
                    "approval request not found in this dashboard; {DETACHED_APPROVALS_WARNING}"
                ),
            ));
        }
        return Err(ApiError::not_found("approval request"));
    }
    // The requester's terminal event updates both dashboard and Discord UI.
    Ok(Json(
        json!({"resolved": true, "id": id, "decision": suffix}),
    ))
}

#[derive(Debug, Serialize, FromRow)]
struct ApprovalAllowlistRow {
    pattern_key: String,
    created_at: String,
}

async fn list_approval_allowlist(
    State(state): State<DashboardState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, ApprovalAllowlistRow>(
        "SELECT pattern_key, created_at FROM approval_allowlist ORDER BY created_at DESC, pattern_key",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({"items": rows})))
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    limit: Option<u32>,
    level: Option<String>,
    search: Option<String>,
}

async fn list_logs(
    State(state): State<DashboardState>,
    Query(query): Query<LogsQuery>,
) -> Json<Value> {
    let limit = query.limit.unwrap_or(250).clamp(1, 1_000) as usize;
    let level = query.level.map(|value| value.to_ascii_uppercase());
    let search = query.search.map(|value| value.to_ascii_lowercase());
    let mut entries = state
        .logs
        .snapshot()
        .into_iter()
        .filter(|entry| {
            level
                .as_deref()
                .is_none_or(|level| entry.level.eq_ignore_ascii_case(level))
                && search.as_deref().is_none_or(|search| {
                    entry.message.to_ascii_lowercase().contains(search)
                        || entry.target.to_ascii_lowercase().contains(search)
                        || entry
                            .fields
                            .to_string()
                            .to_ascii_lowercase()
                            .contains(search)
                })
        })
        .collect::<Vec<_>>();
    if entries.len() > limit {
        entries.drain(0..entries.len() - limit);
    }
    Json(json!({"items": entries}))
}

async fn logs_ws(ws: WebSocketUpgrade, State(state): State<DashboardState>) -> Response {
    ws.on_upgrade(move |socket| handle_logs_socket(socket, state.logs))
        .into_response()
}

async fn handle_logs_socket(mut socket: WebSocket, logs: DashboardLogStore) {
    // Subscribe before taking the snapshot so entries logged while the snapshot is
    // being sent are not lost; ids already sent in the snapshot are skipped below.
    let mut receiver = logs.subscribe();
    let tail = logs.snapshot();
    let mut last_sent_id = tail.last().map_or(0, |entry| entry.id);
    for entry in tail
        .into_iter()
        .rev()
        .take(100)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        if socket
            .send(WsMessage::Text(
                json!({"type":"log","entry":entry}).to_string().into(),
            ))
            .await
            .is_err()
        {
            return;
        }
    }
    loop {
        tokio::select! {
            entry = receiver.recv() => {
                match entry {
                    Ok(entry) => {
                        if entry.id <= last_sent_id {
                            continue;
                        }
                        last_sent_id = entry.id;
                        if socket.send(WsMessage::Text(json!({"type":"log","entry":entry}).to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        if socket.send(WsMessage::Text(json!({"type":"warning","message":format!("skipped {skipped} log entries")}).to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                let Some(incoming) = incoming else { break; };
                match incoming {
                    Ok(WsMessage::Ping(payload)) => {
                        if socket.send(WsMessage::Pong(payload)).await.is_err() { break; }
                    }
                    Ok(WsMessage::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }
}

async fn serve_static(State(state): State<DashboardState>, uri: Uri) -> Response {
    if uri.path().starts_with("/api/") {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error":"API endpoint not found"})),
        )
            .into_response();
    }
    let requested = uri.path().trim_start_matches('/');
    let relative = if requested.is_empty() {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(requested)
    };
    if safe_relative_path(&relative) {
        let path = state.web_root.join(&relative);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return static_response(&path, bytes);
        }
    }
    let index = state.web_root.join("index.html");
    if let Ok(bytes) = tokio::fs::read(&index).await {
        return static_response(&index, bytes);
    }
    fallback_dashboard_html()
}

fn safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn static_response(path: &Path, bytes: Vec<u8>) -> Response {
    let content_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

fn fallback_dashboard_html() -> Response {
    const HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>omon gateway dashboard</title><style>
:root{color-scheme:dark;background:#090b10;color:#edf0f7;font-family:Inter,ui-sans-serif,system-ui,sans-serif}body{max-width:960px;margin:0 auto;padding:48px 24px}h1{font-size:2rem;margin-bottom:.25rem}.muted{color:#9aa5b5}code{background:#161b25;padding:.15rem .35rem;border-radius:.35rem}.card{border:1px solid #283244;border-radius:16px;padding:20px;margin-top:24px;background:#10141d}a{color:#8ab4ff}li{margin:.45rem 0}</style></head>
<body><h1>omon gateway dashboard</h1><p class="muted">The dashboard API is running, but <code>web/dist</code> was not found. Build the React UI with <code>cd web &amp;&amp; npm install &amp;&amp; npm run build</code>.</p>
<div class="card"><h2>API explorer</h2><ul>
<li><a href="/api/status">/api/status</a> — runtime status</li><li><a href="/api/sessions">/api/sessions</a> — sessions</li><li><a href="/api/cron/jobs">/api/cron/jobs</a> — scheduled jobs</li><li><a href="/api/tools">/api/tools</a> — registered tools</li><li><a href="/api/skills">/api/skills</a> — skills</li><li><a href="/api/approvals/pending">/api/approvals/pending</a> — pending approvals</li><li><a href="/api/logs">/api/logs</a> — recent logs</li></ul></div></body></html>"#;
    let mut response = Response::new(Body::from(HTML));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::to_bytes;
    use axum::http::Request;
    use omon_gateway::{PayloadTaskExecutor, ToolRegistry};
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    use super::*;

    async fn test_state() -> DashboardState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");
        let scheduler = CronScheduler::new(pool.clone(), Arc::new(PayloadTaskExecutor));
        let approvals = SmartApprovalGuard::new().with_pool(pool.clone());
        let workspace = std::env::current_dir().expect("current dir");
        DashboardState::new(
            pool,
            None,
            scheduler,
            ToolRegistry::new(),
            approvals,
            WebDashboardDispatcher::new(),
            json!({"model":"test-model","providers":{"openai_api_key_configured":true}}),
            workspace,
            Vec::new(),
            0,
            PathBuf::from("does-not-exist"),
        )
    }

    async fn json_body(response: Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn health_and_status_endpoints_report_runtime_state() {
        let app = router(test_state().await);
        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        assert_eq!(json_body(health).await["status"], "ok");

        let status = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let body = json_body(status).await;
        assert_eq!(body["database"]["sessions"], 0);
        assert_eq!(body["chat_available"], false);
    }

    #[tokio::test]
    async fn approval_lifecycle_local_http_surface() {
        use omon_gateway::discord::adapter::DiscordEgress;
        use omon_gateway::discord::approval::{
            ApprovalDecision, ApprovalError, ApprovalRequester, DiscordApprovalRequester,
        };
        use serenity::all::{HttpBuilder, Message};
        use std::time::Duration;

        // Given: the production router, reducer, composite dispatcher, guard,
        // requester and Discord HTTP client. Only Discord's wire peer is local.
        let root = tempfile::tempdir().unwrap();
        let mut state = test_state().await;
        state.workspace_root = root.path().join("workspace");
        state.web_root = root.path().join("web");
        let (wire_tx, mut wire_rx) = tokio::sync::mpsc::unbounded_channel();
        let rest = Router::new().fallback(
            move |method: axum::http::Method, uri: Uri, Json(body): Json<Value>| {
                let tx = wire_tx.clone();
                async move {
                    tx.send((method, uri.path().to_owned(), body)).unwrap();
                    let mut message = Message::default();
                    message.id = serenity::all::MessageId::new(100);
                    message.channel_id = serenity::all::ChannelId::new(43);
                    Json(message)
                }
            },
        );
        let rest_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let rest_addr = rest_listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let rest_shutdown = shutdown.clone();
        let rest_task = tokio::spawn(async move {
            axum::serve(rest_listener, rest)
                .with_graceful_shutdown(rest_shutdown.cancelled_owned())
                .await
                .unwrap();
        });
        let egress = Arc::new(DiscordEgress::new(Arc::new(
            HttpBuilder::new("U03-local")
                .proxy(format!("http://{rest_addr}"))
                .ratelimiter_disabled(true)
                .build(),
        )));
        let requester = Arc::new(DiscordApprovalRequester::new(
            state.approvals.clone(),
            Duration::from_secs(60),
        ));
        requester
            .set_dispatcher(Arc::new(CompositeDispatcher::new(
                egress.clone(),
                state.events.clone(),
            )))
            .await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router(state.clone());
        let app_shutdown = shutdown.clone();
        let app_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(app_shutdown.cancelled_owned())
                .await
                .unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let session = SessionKey::new("discord", Some("guild"), "42", Some("43"), "7");
        let mut preserves_content = true;
        let mut events = state.events.subscribe();

        for ending in [
            "once", "session", "always", "deny", "drop", "clear", "cancel",
        ] {
            let key = session.clone();
            let task_requester = requester.clone();
            let task_key = key.clone();
            if ending == "dead" {
                egress.dead_targets().mark_dead(43, "fixture");
            }
            let task = tokio::spawn(async move {
                task_requester
                    .request_approval_scoped(
                        &task_key,
                        "rm -rf fixture",
                        "display only",
                        &format!("U03:{ending}"),
                    )
                    .await
            });
            let request_id = match tokio::time::timeout(Duration::from_secs(3), events.recv())
                .await
                .unwrap()
                .unwrap()
            {
                OutboundAction::ApprovalRequest { request_id, .. } => request_id,
                other => panic!("unexpected event {other:?}"),
            };
            if ending != "dead" {
                let (method, path, body) =
                    tokio::time::timeout(Duration::from_secs(3), wire_rx.recv())
                        .await
                        .unwrap()
                        .unwrap();
                assert_eq!(method, axum::http::Method::POST);
                assert!(path.ends_with("/channels/43/messages"));
                assert_eq!(
                    body["components"][0]["components"]
                        .as_array()
                        .unwrap()
                        .len(),
                    4
                );
                let pending: Value = client
                    .get(format!("http://{addr}/api/approvals/pending"))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                assert_eq!(pending["items"][0]["id"], request_id.to_string());
                assert_eq!(pending["pending_count"], 1);
            }

            // When: terminate via real HTTP resolution, caller cancellation,
            // guard cleanup or failed dispatch. Subscriptions precede the action.
            match ending {
                "drop" => task.abort(),
                "clear" => state.approvals.clear_session(&key).await,
                "cancel" => state.approvals.cancel(request_id).await,
                "dead" => {}
                decision => {
                    let response = client
                        .post(format!("http://{addr}/api/approvals/{request_id}/resolve"))
                        .json(&json!({"decision": decision}))
                        .send()
                        .await
                        .unwrap();
                    assert_eq!(response.status(), StatusCode::OK);
                    let result: Value = response.json().await.unwrap();
                    assert_eq!(result["resolved"], true);
                }
            }
            let result = tokio::time::timeout(Duration::from_secs(3), task)
                .await
                .unwrap();
            match ending {
                "drop" => assert!(result.unwrap_err().is_cancelled()),
                "clear" | "cancel" | "dead" => {
                    assert_eq!(result.unwrap(), Err(ApprovalError::Cancelled))
                }
                "deny" => assert_eq!(result.unwrap(), Ok(ApprovalDecision::Deny { reason: None })),
                _ => assert!(result.unwrap().unwrap().is_approved()),
            }
            assert!(
                matches!(tokio::time::timeout(Duration::from_secs(3), events.recv()).await.unwrap().unwrap(),
                OutboundAction::ExpireApproval { request_id: id } if id == request_id)
            );
            if ending != "dead" {
                let (method, path, body) =
                    tokio::time::timeout(Duration::from_secs(3), wire_rx.recv())
                        .await
                        .unwrap()
                        .unwrap();
                println!("AP08 local {ending} {method} {path} edit={body}");
                assert_eq!(method, axum::http::Method::PATCH);
                assert!(path.ends_with("/channels/43/messages/100"));
                assert_eq!(body["components"], json!([]));
                preserves_content &= body.get("content").is_none();
            }
            let pending: Value = client
                .get(format!("http://{addr}/api/approvals/pending"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(pending["pending_count"], 0);
            assert_eq!(pending["items"], json!([]));
            assert_eq!(egress.approval_message_count().await, 0);
            assert!(events.try_recv().is_err());
            assert!(wire_rx.try_recv().is_err());
            let duplicate = client
                .post(format!("http://{addr}/api/approvals/{request_id}/resolve"))
                .json(&json!({"decision":"once"}))
                .send()
                .await
                .unwrap();
            assert_eq!(duplicate.status(), StatusCode::NOT_FOUND);
            println!("AP08 local {ending} guard=0 dashboard=0 egress=0 duplicate=404");
        }

        // Then: one terminal event cleans both real surfaces without overwriting
        // a component interaction's decision. Close resources before assertion.
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(3), rest_task)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), app_task)
            .await
            .unwrap()
            .unwrap();
        state.pool.close().await;
        root.close().unwrap();
        println!("AP08 local cleanup: listeners joined, DB closed, temp root removed; preserves_content={preserves_content}");
        assert!(
            preserves_content,
            "terminal edit must not replace a resolution with expired content"
        );
    }

    #[tokio::test]
    async fn sessions_list_and_transcript_are_paginated() {
        let state = test_state().await;
        let key = SessionKey::new("web", None::<String>, "test", None::<String>, "dashboard");
        let storage_key = key.storage_key();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json) VALUES (?, 'web', NULL, 'test', NULL, 'dashboard', '{}')",
        )
        .bind(&storage_key)
        .execute(&state.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json) VALUES (?, ?, 'user', 'hello', '{}')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&storage_key)
        .execute(&state.pool)
        .await
        .unwrap();
        let app = router(state);
        let sessions = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/sessions?per_page=10")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(sessions).await;
        assert_eq!(body["total"], 1);
        assert_eq!(body["items"][0]["platform"], "web");

        let transcript = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions/test/messages")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(transcript.status(), StatusCode::OK);
        let body = json_body(transcript).await;
        assert_eq!(body["items"][0]["content"], "hello");
    }

    #[tokio::test]
    async fn cron_crud_routes_use_scheduler_semantics() {
        let app = router(test_state().await);
        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/cron/jobs")
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"expression":"@every 5m","payload":{"content":"ping"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let created = json_body(create).await;
        let id = created["id"].as_str().unwrap();

        let pause = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/cron/jobs/{id}/pause"))
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pause.status(), StatusCode::OK);

        let get_job = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/cron/jobs/{id}"))
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(get_job).await;
        assert_eq!(body["enabled"], false);
    }

    #[tokio::test]
    async fn config_endpoint_never_exposes_provider_secret_values() {
        let app = router(test_state().await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json_body(response).await;
        let rendered = body.to_string();
        assert!(!rendered.contains("OPENAI_API_KEY"));
        assert_eq!(body["providers"]["openai_api_key_configured"], true);
    }

    #[tokio::test]
    async fn disk_pressure_real_loopback_http_surface() {
        let root = tempfile::tempdir().unwrap();
        let sample_cell = Arc::new(Mutex::new((
            Some(1000 * readiness::DISK_BYTES_PER_MB),
            Some(200 * readiness::DISK_BYTES_PER_MB),
        )));
        let cell_clone = sample_cell.clone();

        let mut state = test_state().await;
        state.workspace_root = root.path().join("workspace");
        tokio::fs::create_dir_all(&state.workspace_root)
            .await
            .unwrap();
        state = state.with_disk_sampler(move |_| *cell_clone.lock());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let app_shutdown = shutdown.clone();
        let app = router(state.clone());
        let app_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(app_shutdown.cancelled_owned())
                .await
                .unwrap();
        });

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();

        // 1. Critical headroom sample: 1000 MiB total, 200 MiB free (< 256 MiB floor)
        *sample_cell.lock() = (
            Some(1000 * readiness::DISK_BYTES_PER_MB),
            Some(200 * readiness::DISK_BYTES_PER_MB),
        );
        let health = client
            .get(format!("http://{addr}/api/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_body: Value = health.json().await.unwrap();
        assert_eq!(health_body["status"], "ok");

        let status = client
            .get(format!("http://{addr}/api/status"))
            .send()
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let status_body: Value = status.json().await.unwrap();
        assert_eq!(status_body["status"], "degraded");
        assert_eq!(status_body["disk"]["pressure"], "critical");
        assert_eq!(status_body["disk"]["status"], "degraded");
        assert_eq!(status_body["disk"]["used_percent"], 80.0);
        assert_eq!(status_body["disk"]["total_mb"], 1000);
        assert_eq!(status_body["disk"]["available_mb"], 200);
        assert_eq!(
            status_body["disk"]["workspace_total_bytes"],
            1000 * 1024 * 1024
        );
        assert_eq!(
            status_body["disk"]["workspace_available_bytes"],
            200 * 1024 * 1024
        );

        let readiness = client
            .get(format!("http://{addr}/api/readiness"))
            .send()
            .await
            .unwrap();
        assert_eq!(readiness.status(), StatusCode::SERVICE_UNAVAILABLE);
        let ready_body: Value = readiness.json().await.unwrap();
        assert_eq!(ready_body["status"], "degraded");
        assert_eq!(ready_body["checks"]["disk"], false);
        assert_eq!(ready_body["disk"]["pressure"], "critical");

        // 2. Large capacity with headroom: 1,000,000 MiB total, 50,000 MiB free (95% used, 50 GB free)
        *sample_cell.lock() = (
            Some(1000000 * readiness::DISK_BYTES_PER_MB),
            Some(50000 * readiness::DISK_BYTES_PER_MB),
        );
        let health2 = client
            .get(format!("http://{addr}/api/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(health2.status(), StatusCode::OK);

        let status2 = client
            .get(format!("http://{addr}/api/status"))
            .send()
            .await
            .unwrap();
        assert_eq!(status2.status(), StatusCode::OK);
        let status2_body: Value = status2.json().await.unwrap();
        assert_eq!(status2_body["status"], "ok");
        assert_eq!(status2_body["disk"]["pressure"], "ok");
        assert_eq!(status2_body["disk"]["status"], "ok");
        assert_eq!(status2_body["disk"]["used_percent"], 95.0);
        assert_eq!(status2_body["disk"]["total_mb"], 1000000);
        assert_eq!(status2_body["disk"]["available_mb"], 50000);

        let readiness2 = client
            .get(format!("http://{addr}/api/readiness"))
            .send()
            .await
            .unwrap();
        assert_eq!(readiness2.status(), StatusCode::OK);
        let ready2_body: Value = readiness2.json().await.unwrap();
        assert_eq!(ready2_body["status"], "ready");
        assert_eq!(ready2_body["checks"]["disk"], true);
        assert_eq!(ready2_body["disk"]["pressure"], "ok");

        // 3. Zero capacity sample: 0 total, 0 free
        *sample_cell.lock() = (Some(0), Some(0));
        let health3 = client
            .get(format!("http://{addr}/api/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(health3.status(), StatusCode::OK);

        let status3 = client
            .get(format!("http://{addr}/api/status"))
            .send()
            .await
            .unwrap();
        assert_eq!(status3.status(), StatusCode::OK);
        let status3_body: Value = status3.json().await.unwrap();
        assert_eq!(status3_body["status"], "degraded");
        assert_eq!(status3_body["disk"]["pressure"], "unknown");
        assert_eq!(status3_body["disk"]["status"], "degraded");

        let readiness3 = client
            .get(format!("http://{addr}/api/readiness"))
            .send()
            .await
            .unwrap();
        assert_eq!(readiness3.status(), StatusCode::SERVICE_UNAVAILABLE);
        let ready3_body: Value = readiness3.json().await.unwrap();
        assert_eq!(ready3_body["status"], "degraded");
        assert_eq!(ready3_body["checks"]["disk"], false);
        assert_eq!(ready3_body["disk"]["pressure"], "unknown");

        // 4. Unreadable filesystem sample: None, None
        *sample_cell.lock() = (None, None);
        let health4 = client
            .get(format!("http://{addr}/api/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(health4.status(), StatusCode::OK);

        let status4 = client
            .get(format!("http://{addr}/api/status"))
            .send()
            .await
            .unwrap();
        assert_eq!(status4.status(), StatusCode::OK);
        let status4_body: Value = status4.json().await.unwrap();
        assert_eq!(status4_body["status"], "degraded");
        assert_eq!(status4_body["disk"]["pressure"], "unknown");
        assert!(status4_body["disk"]["workspace_total_bytes"].is_null());

        let readiness4 = client
            .get(format!("http://{addr}/api/readiness"))
            .send()
            .await
            .unwrap();
        assert_eq!(readiness4.status(), StatusCode::SERVICE_UNAVAILABLE);
        let ready4_body: Value = readiness4.json().await.unwrap();
        assert_eq!(ready4_body["status"], "degraded");
        assert_eq!(ready4_body["checks"]["disk"], false);
        assert_eq!(ready4_body["disk"]["pressure"], "unknown");

        // 5. Elevated boundary sample: 4000 MiB total, 400 MiB free (< 512 MiB floor, 90% used)
        *sample_cell.lock() = (
            Some(4000 * readiness::DISK_BYTES_PER_MB),
            Some(400 * readiness::DISK_BYTES_PER_MB),
        );
        let health5 = client
            .get(format!("http://{addr}/api/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(health5.status(), StatusCode::OK);

        let status5 = client
            .get(format!("http://{addr}/api/status"))
            .send()
            .await
            .unwrap();
        assert_eq!(status5.status(), StatusCode::OK);
        let status5_body: Value = status5.json().await.unwrap();
        assert_eq!(status5_body["status"], "degraded");
        assert_eq!(status5_body["disk"]["pressure"], "elevated");
        assert_eq!(status5_body["disk"]["status"], "degraded");
        assert_eq!(status5_body["disk"]["used_percent"], 90.0);

        let readiness5 = client
            .get(format!("http://{addr}/api/readiness"))
            .send()
            .await
            .unwrap();
        assert_eq!(readiness5.status(), StatusCode::SERVICE_UNAVAILABLE);
        let ready5_body: Value = readiness5.json().await.unwrap();
        assert_eq!(ready5_body["status"], "degraded");
        assert_eq!(ready5_body["checks"]["disk"], false);
        assert_eq!(ready5_body["disk"]["pressure"], "elevated");

        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(3), app_task)
            .await
            .unwrap()
            .unwrap();
        state.pool.close().await;
        root.close().unwrap();
    }

    #[test]
    fn canonical_session_key_parser_handles_embedded_separators_and_bot_identity() {
        let original = SessionKey::new(
            "discord",
            Some("guild|x"),
            "channel:1",
            Some("thread|2"),
            "user|3",
        )
        .with_bot_id("bot:4");
        let parsed = parse_storage_key(&original.storage_key()).expect("parse storage key");
        assert_eq!(parsed, original);
    }

    struct U65RecordingRunner {
        events: tokio::sync::mpsc::UnboundedSender<InboundEvent>,
    }

    #[async_trait]
    impl omon_gateway::AgentRunner for U65RecordingRunner {
        async fn run(
            &self,
            _session: &mut omon_gateway::SessionContext,
            event: InboundEvent,
        ) -> omon_gateway::Result<()> {
            let _ = self.events.send(event);
            Ok(())
        }
    }

    #[tokio::test]
    async fn dashboard_rejects_untrusted_host_and_ws_origin() {
        use futures_util::{SinkExt, StreamExt};
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let root = tempfile::tempdir().unwrap();
        let mut state = test_state().await;
        state.workspace_root = root.path().join("workspace");
        state.web_root = root.path().join("web");
        tokio::fs::create_dir_all(&state.workspace_root)
            .await
            .unwrap();

        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        let runner = Arc::new(U65RecordingRunner { events: event_tx });
        let mux = SessionMultiplexer::new(
            state.pool.clone(),
            runner,
            omon_gateway::MultiplexerConfig::default(),
        );
        state.multiplexer = Some(mux);

        // 1. Verify public-bind refusal even with insecure=true
        let invalid_settings = DashboardSettings {
            enabled: true,
            host: "0.0.0.0".into(),
            port: 9119,
            insecure: true,
            web_root: root.path().join("web"),
        };
        let bind_rejected = invalid_settings.validate().is_err();
        println!("U65 public bind with insecure=true rejected={bind_rejected}");

        // Spawn real dashboard server on loopback
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);

        let settings = DashboardSettings {
            enabled: true,
            host: "127.0.0.1".into(),
            port: addr.port(),
            insecure: false,
            web_root: root.path().join("web"),
        };
        let shutdown = CancellationToken::new();
        let server_handle = spawn_server(settings, state.clone(), shutdown.clone())
            .await
            .unwrap();

        // 2. Untrusted HTTP Host denial
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(
                b"GET /api/sessions HTTP/1.1\r\nHost: evil.test\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let mut resp_bytes = Vec::new();
        stream.read_to_end(&mut resp_bytes).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp_bytes);
        let http_status_line = resp_str.lines().next().unwrap_or_default().to_string();
        println!("U65 HTTP GET /api/sessions Host=evil.test status_line={http_status_line}");

        // 3. Untrusted WS Origin denial before data or admitted event
        let mut evil_ws_req = format!("ws://{addr}/api/sessions/probe/ws")
            .into_client_request()
            .unwrap();
        evil_ws_req.headers_mut().insert(
            axum::http::header::ORIGIN,
            axum::http::HeaderValue::from_static("https://evil.test"),
        );
        let mut admitted_evil = false;
        let ws_evil_status: String;
        match tokio_tungstenite::connect_async(evil_ws_req).await {
            Ok((mut evil_ws, resp)) => {
                ws_evil_status = resp.status().to_string();
                println!(
                    "U65 WS /api/sessions/probe/ws Origin=https://evil.test connected status={ws_evil_status}"
                );
                let _ = evil_ws.next().await;
                let _ = evil_ws
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"type":"message","content":"probe"})
                            .to_string()
                            .into(),
                    ))
                    .await;
                if let Ok(Some(ev)) =
                    tokio::time::timeout(Duration::from_millis(500), event_rx.recv()).await
                {
                    println!("U65 WS evil origin admitted event={:?}", ev.content);
                    admitted_evil = true;
                }
            }
            Err(err) => {
                ws_evil_status = err.to_string();
                println!(
                    "U65 WS /api/sessions/probe/ws Origin=https://evil.test rejected: {ws_evil_status}"
                );
            }
        }

        // 4. Local same-origin HTTP and WS work
        let mut local_stream = TcpStream::connect(addr).await.unwrap();
        let local_req =
            format!("GET /api/sessions HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        local_stream.write_all(local_req.as_bytes()).await.unwrap();
        let mut local_resp_bytes = Vec::new();
        local_stream
            .read_to_end(&mut local_resp_bytes)
            .await
            .unwrap();
        let local_resp_str = String::from_utf8_lossy(&local_resp_bytes);
        let local_http_status_line = local_resp_str
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        println!(
            "U65 local HTTP GET /api/sessions Host={addr} status_line={local_http_status_line}"
        );

        let mut local_ws_req = format!("ws://{addr}/api/sessions/probe/ws")
            .into_client_request()
            .unwrap();
        local_ws_req.headers_mut().insert(
            axum::http::header::ORIGIN,
            axum::http::HeaderValue::from_str(&format!("http://{addr}")).unwrap(),
        );
        let mut admitted_local = false;
        match tokio_tungstenite::connect_async(local_ws_req).await {
            Ok((mut local_ws, resp)) => {
                println!("U65 local WS connected status={}", resp.status());
                let _ = local_ws.next().await;
                local_ws
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        json!({"type":"message","content":"probe"})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
                if let Ok(Some(ev)) =
                    tokio::time::timeout(Duration::from_secs(2), event_rx.recv()).await
                {
                    println!("U65 local WS admitted event={:?}", ev.content);
                    admitted_local = true;
                }
            }
            Err(err) => {
                println!("U65 local WS failed err={err}");
            }
        }

        // Clean up resources before assertions
        shutdown.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(3), server_handle).await;
        state.pool.close().await;
        root.close().unwrap();

        // Assertions
        assert!(
            bind_rejected,
            "public bind must be refused even when insecure=true"
        );
        assert!(
            http_status_line.contains("403") || http_status_line.contains("400"),
            "GET /api/sessions with Host: evil.test must be rejected with 400/403, got: {http_status_line}"
        );
        assert!(
            !admitted_evil,
            "WS with untrusted Origin must not admit event to runtime"
        );
        assert!(
            ws_evil_status.contains("403") || ws_evil_status.contains("400"),
            "WS with untrusted Origin must be rejected with 400/403, got: {ws_evil_status}"
        );
        assert!(
            admitted_local,
            "local same-origin WS must be accepted and admit message"
        );
    }

    #[tokio::test]
    async fn bot_create_with_existing_id_returns_conflict_and_keeps_profile() {
        let state = test_state().await;
        let pool = state.pool.clone();
        let app = router(state);
        let post = |body: Value| {
            Request::builder()
                .method("POST")
                .uri("/api/bots")
                .header(header::HOST, "127.0.0.1")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let first = app
            .clone()
            .oneshot(post(json!({
                "bot_id": "bot-dup",
                "name": "original",
                "system_prompt": "keep me",
            })))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let second = app
            .clone()
            .oneshot(post(json!({"bot_id": "bot-dup", "name": "clobber"})))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
        let (name, prompt): (String, Option<String>) =
            sqlx::query_as("SELECT name, system_prompt FROM bot_profiles WHERE bot_id = 'bot-dup'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "original");
        assert_eq!(prompt.as_deref(), Some("keep me"));
    }

    #[tokio::test]
    async fn bot_update_with_only_name_preserves_other_fields() {
        let state = test_state().await;
        let app = router(state);

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bots")
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "bot_id": "bot-partial-update",
                            "name": "original",
                            "model": "gpt-4o",
                            "system_prompt": "you are original",
                            "enabled_toolsets": ["terminal", "file"],
                            "custom_settings": {"temperature": 1},
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::OK);

        let update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/bots/bot-partial-update")
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({"name": "renamed"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update.status(), StatusCode::OK);
        let updated = json_body(update).await;

        assert_eq!(updated["name"], "renamed");
        assert_eq!(
            updated["model"], "gpt-4o",
            "omitted model must retain stored value: {updated}"
        );
        assert_eq!(
            updated["system_prompt"], "you are original",
            "omitted system_prompt must retain stored value: {updated}"
        );
        assert_eq!(
            updated["enabled_toolsets"],
            json!(["terminal", "file"]),
            "omitted enabled_toolsets must retain stored value: {updated}"
        );
        assert_eq!(
            updated["custom_settings"],
            json!({"temperature": 1}),
            "omitted custom_settings must retain stored value: {updated}"
        );

        let cleared = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/bots/bot-partial-update")
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({"name": "renamed", "model": "", "system_prompt": "  ", "enabled_toolsets": []})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cleared.status(), StatusCode::OK);
        let cleared = json_body(cleared).await;
        assert!(
            cleared["model"].is_null(),
            "explicitly empty model must clear to default: {cleared}"
        );
        assert!(
            cleared["system_prompt"].is_null(),
            "explicitly empty system_prompt must clear: {cleared}"
        );
        assert!(
            cleared["enabled_toolsets"].is_null(),
            "explicitly empty toolsets must clear to all defaults: {cleared}"
        );
    }

    #[tokio::test]
    async fn cross_origin_state_changing_http_requests_are_rejected() {
        let state = test_state().await;
        let app = router(state);

        let body = || {
            Body::from(json!({"bot_id": "bot-origin-check", "name": "origin-check"}).to_string())
        };

        let cross_origin_post = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bots")
                    .header(header::HOST, "127.0.0.1:9119")
                    .header(header::ORIGIN, "http://evil.test")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            cross_origin_post.status(),
            StatusCode::FORBIDDEN,
            "POST with a foreign Origin must be rejected"
        );

        let same_origin_post = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/bots")
                    .header(header::HOST, "127.0.0.1:9119")
                    .header(header::ORIGIN, "http://127.0.0.1:9119")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(body())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            same_origin_post.status(),
            StatusCode::OK,
            "POST with a matching Origin must be accepted"
        );

        let cross_origin_get = app
            .oneshot(
                Request::builder()
                    .uri("/api/bots")
                    .header(header::HOST, "127.0.0.1:9119")
                    .header(header::ORIGIN, "http://evil.test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            cross_origin_get.status(),
            StatusCode::OK,
            "GET with a foreign Origin must still be served"
        );
    }

    #[tokio::test]
    async fn bot_profile_delete_stays_deleted() {
        use std::time::Duration;

        let root = tempfile::tempdir().unwrap();
        let mut state = test_state().await;
        state.workspace_root = root.path().join("workspace");
        state.web_root = root.path().join("web");
        tokio::fs::create_dir_all(&state.workspace_root)
            .await
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let app_shutdown = shutdown.clone();
        let app = router(state.clone());
        let app_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(app_shutdown.cancelled_owned())
                .await
                .unwrap();
        });

        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();

        let bot_id = "1465631383862120451";

        // 1. POST saved bot 1465631383862120451
        let create_payload = json!({
            "bot_id": bot_id,
            "name": "wawabot-saved",
            "model": "gpt-4o",
            "system_prompt": "You are wawabot custom profile",
            "enabled_toolsets": ["terminal", "file"]
        });
        let post_resp = client
            .post(format!("http://{addr}/api/bots"))
            .json(&create_payload)
            .send()
            .await
            .unwrap();
        let post_status = post_resp.status();
        let post_body: Value = post_resp.json().await.unwrap();
        println!("U69 POST /api/bots status={post_status} body={post_body}");

        // BotsPage reload sequence: GET /api/bots after creation
        let list_after_create_resp = client
            .get(format!("http://{addr}/api/bots"))
            .send()
            .await
            .unwrap();
        let list_after_create: Value = list_after_create_resp.json().await.unwrap();
        let created_in_list = list_after_create["items"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|b| b["bot_id"] == bot_id && b["name"] == "wawabot-saved")
        });
        println!("U69 GET /api/bots after create: created_in_list={created_in_list}");

        // GET detail of created bot
        let detail_resp = client
            .get(format!("http://{addr}/api/bots/{bot_id}"))
            .send()
            .await
            .unwrap();
        let detail_status = detail_resp.status();
        let detail_body: Value = detail_resp.json().await.unwrap();
        println!(
            "U69 GET /api/bots/{bot_id} status={detail_status} name={:?}",
            detail_body["name"]
        );

        // 2. DELETE bot 1465631383862120451
        let del_resp = client
            .delete(format!("http://{addr}/api/bots/{bot_id}"))
            .send()
            .await
            .unwrap();
        let del_status = del_resp.status();
        let del_body: Value = del_resp.json().await.unwrap();
        println!("U69 DELETE /api/bots/{bot_id} status={del_status} body={del_body}");

        // BotsPage reload sequence: GET /api/bots after delete
        let list_after_delete_resp = client
            .get(format!("http://{addr}/api/bots"))
            .send()
            .await
            .unwrap();
        let list_after_delete_status = list_after_delete_resp.status();
        let list_after_delete: Value = list_after_delete_resp.json().await.unwrap();
        let deleted_in_list = list_after_delete["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|b| b["bot_id"] == bot_id));
        println!(
            "U69 GET /api/bots after delete status={list_after_delete_status} deleted_in_list={deleted_in_list} (RED: synthetic row returns; GREEN: row absent)"
        );

        // GET detail of deleted bot
        let detail_after_delete_resp = client
            .get(format!("http://{addr}/api/bots/{bot_id}"))
            .send()
            .await
            .unwrap();
        let detail_after_delete_status = detail_after_delete_resp.status();
        let detail_after_delete_body: Value = detail_after_delete_resp.json().await.unwrap();
        println!(
            "U69 GET /api/bots/{bot_id} after delete status={detail_after_delete_status} body={detail_after_delete_body} (RED: 200 fabricated; GREEN: 404)"
        );

        // GET detail of unknown bot
        let unknown_resp = client
            .get(format!("http://{addr}/api/bots/unknown-bot-999"))
            .send()
            .await
            .unwrap();
        let unknown_status = unknown_resp.status();
        let unknown_body: Value = unknown_resp.json().await.unwrap();
        println!(
            "U69 GET /api/bots/unknown-bot-999 status={unknown_status} body={unknown_body} (RED: 200 fabricated; GREEN: 404)"
        );

        // 3. DB unavailable test
        state.pool.close().await;
        let db_down_resp = client
            .get(format!("http://{addr}/api/bots"))
            .send()
            .await
            .unwrap();
        let db_down_status = db_down_resp.status();
        let db_down_body: Value = db_down_resp.json().await.unwrap_or(json!({}));
        let invented_inventory = db_down_body["items"]
            .as_array()
            .is_some_and(|arr| !arr.is_empty());
        println!(
            "U69 DB unavailable GET /api/bots status={db_down_status} invented_inventory={invented_inventory} (RED: 200 with fake rows; GREEN: error)"
        );

        // Clean up server resources before asserting
        shutdown.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(3), app_task).await;
        root.close().unwrap();

        // Assertions
        assert_eq!(post_status, StatusCode::OK);
        assert!(created_in_list, "created bot must be present in bot list");
        assert_eq!(detail_status, StatusCode::OK);
        assert_eq!(del_status, StatusCode::OK);

        assert!(
            !deleted_in_list,
            "deleted bot 1465631383862120451 must NOT appear in bot list, but synthetic row was returned: {list_after_delete}"
        );
        assert_eq!(
            detail_after_delete_status,
            StatusCode::NOT_FOUND,
            "GET detail of deleted bot must be 404 Not Found, got: {detail_after_delete_status}"
        );
        assert_eq!(
            unknown_status,
            StatusCode::NOT_FOUND,
            "GET detail of unknown bot must be 404 Not Found, got: {unknown_status}"
        );
        assert_ne!(
            db_down_status,
            StatusCode::OK,
            "DB unavailable must return error, not 200 OK"
        );
        assert!(
            !invented_inventory,
            "DB unavailable must not return invented inventory: {db_down_body}"
        );
    }

    struct FixedCronOutput;

    #[async_trait]
    impl omon_gateway::CronTaskExecutor for FixedCronOutput {
        async fn execute(&self, job: &CronJob) -> omon_gateway::Result<Option<String>> {
            Ok(Some(format!("cron-ran:{}", job.id)))
        }
    }

    #[tokio::test]
    async fn fix_dash_web_dispatcher_fails_closed_for_messenger_delivery() {
        let dispatcher = WebDashboardDispatcher::new();
        let mut events = dispatcher.subscribe();
        let discord = SessionKey::new("discord", None::<String>, "999", None::<String>, "7");
        let web = web_session_key("probe");
        let send = |session: SessionKey| OutboundAction::SendMessage {
            session,
            content: "hi".into(),
            reply_to: None,
        };
        assert!(dispatcher.dispatch(send(web)).await.is_ok());
        assert!(
            dispatcher.dispatch(send(discord.clone())).await.is_err(),
            "web dashboard dispatcher must not report Discord delivery as successful"
        );
        // Dashboard observers still see the attempted action.
        assert!(events.try_recv().is_ok());
        assert!(events.try_recv().is_ok());

        // A dashboard-triggered Discord cron run must record a failed obligation.
        let state = test_state().await;
        let scheduler = CronScheduler::with_dispatcher(
            state.pool.clone(),
            Arc::new(FixedCronOutput),
            Arc::new(dispatcher.clone()),
        );
        let payload = json!({
            "id": "fix_dash_cron",
            "name": "Fix Dash Cron",
            "prompt": "run task",
            "deliver": "discord:999",
            "schedule": {"kind": "cron", "expr": "0 * * * *"}
        });
        sqlx::query("INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority, created_at, updated_at) VALUES ('fix_dash_cron', '0 * * * *', ?, 1, 'omon_owned', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(payload.to_string())
            .execute(&state.pool)
            .await
            .unwrap();
        let job = scheduler.get("fix_dash_cron").await.unwrap().unwrap();
        let result = scheduler.execute_job(&job).await;
        let states = sqlx::query_scalar::<_, String>(
            "SELECT state FROM delivery_obligations WHERE channel_id = '999'",
        )
        .fetch_all(&state.pool)
        .await
        .unwrap();
        println!("FIX-DASH-1 execute_job={result:?} obligation_states={states:?}");
        assert!(
            result.is_err(),
            "Discord cron delivery via web dispatcher must fail"
        );
        assert_eq!(states, vec!["failed".to_owned()]);
    }

    #[tokio::test]
    async fn fix_dash_detached_dashboard_reports_gateway_approvals_unavailable() {
        let state = test_state().await.detached_from_gateway();
        let app = router(state);
        let pending = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/approvals/pending")
                    .header(header::HOST, "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pending.status(), StatusCode::OK);
        let body = json_body(pending).await;
        println!("FIX-DASH-2 pending={body}");
        assert_eq!(body["complete"], false);
        assert_eq!(body["gateway_approvals_visible"], false);
        assert!(body["warning"].as_str().is_some_and(|w| !w.is_empty()));

        let resolve = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/approvals/{}/resolve", Uuid::new_v4()))
                    .header(header::HOST, "127.0.0.1")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"decision":"once"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        println!("FIX-DASH-2 resolve unknown status={}", resolve.status());
        assert_eq!(resolve.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn fix_dash_detached_dashboard_rejects_chat_and_stop_for_messenger_sessions() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let mut state = test_state().await.detached_from_gateway();
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        state.multiplexer = Some(SessionMultiplexer::new(
            state.pool.clone(),
            Arc::new(U65RecordingRunner { events: event_tx }),
            omon_gateway::MultiplexerConfig::default(),
        ));
        let discord = SessionKey::new("discord", Some("g"), "123", None::<String>, "42");
        let storage = discord.storage_key();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '123', '42', '{}')")
            .bind(&storage)
            .execute(&state.pool)
            .await
            .unwrap();
        let app = router(state.clone());
        let post = |uri: String, body: &'static str| {
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::HOST, "127.0.0.1")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap()
        };
        let encoded = urlencoding::encode(&storage).into_owned();
        let chat = app
            .clone()
            .oneshot(post(
                format!("/api/sessions/{encoded}/chat"),
                r#"{"message":"hello"}"#,
            ))
            .await
            .unwrap();
        let chat_status = chat.status();
        let stop = app
            .clone()
            .oneshot(post(format!("/api/sessions/{encoded}/stop"), ""))
            .await
            .unwrap();
        let stop_status = stop.status();
        let web_chat = app
            .clone()
            .oneshot(post(
                "/api/sessions/web-probe/chat".into(),
                r#"{"message":"hello"}"#,
            ))
            .await
            .unwrap();
        let web_chat_status = web_chat.status();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let shutdown = CancellationToken::new();
        let app_shutdown = shutdown.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(app_shutdown.cancelled_owned())
                .await
                .unwrap();
        });
        let mut request = format!("ws://{addr}/api/sessions/{encoded}/ws")
            .into_client_request()
            .unwrap();
        request.headers_mut().insert(
            header::ORIGIN,
            HeaderValue::from_str(&format!("http://{addr}")).unwrap(),
        );
        let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let _ready = ws.next().await.unwrap().unwrap();
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"type":"message","content":"hello"})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
        let reply: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        let _ = ws.close(None).await;
        shutdown.cancel();
        let _ = server.await;

        println!(
            "FIX-DASH-3 chat={chat_status} stop={stop_status} web_chat={web_chat_status} ws_reply={reply}"
        );
        assert_eq!(chat_status, StatusCode::CONFLICT);
        assert_eq!(stop_status, StatusCode::CONFLICT);
        assert_eq!(web_chat_status, StatusCode::ACCEPTED);
        assert_eq!(reply["type"], "error");
    }

    #[tokio::test]
    async fn fix_dash_create_cron_job_rejects_duplicate_id() {
        let state = test_state().await;
        let app = router(state.clone());
        let create = |content: &str| {
            Request::builder()
                .method("POST")
                .uri("/api/cron/jobs")
                .header(header::HOST, "127.0.0.1")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"id":"dup","expression":"@every 5m","payload":{"content":content}})
                        .to_string(),
                ))
                .unwrap()
        };
        let first = app.clone().oneshot(create("original")).await.unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
        let second = app.oneshot(create("overwrite")).await.unwrap();
        let second_status = second.status();
        let job = state.scheduler.get("dup").await.unwrap().unwrap();
        println!(
            "FIX-DASH-4 second_status={second_status} payload={}",
            job.payload_json
        );
        assert_eq!(second_status, StatusCode::CONFLICT);
        assert!(job.payload_json.contains("original"));
    }

    #[tokio::test]
    async fn fix_r2_status_reports_configured_model() {
        let response = router(test_state().await)
            .oneshot(fix_r5_request("GET", "/api/status", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["model"], "test-model");
    }

    #[tokio::test]
    async fn fix_r2_cron_create_without_id_honors_disabled_and_requires_payload() {
        let state = test_state().await;
        let app = router(state.clone());
        let created = app
            .clone()
            .oneshot(fix_r5_request(
                "POST",
                "/api/cron/jobs",
                Some(json!({"expression":"@every 5m","enabled":false,"payload":{"content":"x"}})),
            ))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = json_body(created).await;
        assert_eq!(created["enabled"], false);
        assert!(created["next_run_at"].is_null());
        let id = created["id"].as_str().unwrap();
        assert!(Uuid::parse_str(id).is_ok(), "generated id: {id}");

        let missing = app
            .oneshot(fix_r5_request(
                "POST",
                "/api/cron/jobs",
                Some(json!({"expression":"@every 5m"})),
            ))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn fix_r2_create_bot_normalizes_blank_overrides_to_null() {
        let app = router(test_state().await);
        let created = app
            .clone()
            .oneshot(fix_r5_request(
                "POST",
                "/api/bots",
                Some(json!({
                    "bot_id": "bot-blank",
                    "name": "blank",
                    "model": "",
                    "system_prompt": "  ",
                    "enabled_toolsets": []
                })),
            ))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        let bot = app
            .oneshot(fix_r5_request("GET", "/api/bots/bot-blank", None))
            .await
            .unwrap();
        assert_eq!(bot.status(), StatusCode::OK);
        let bot = json_body(bot).await;
        assert!(bot["model"].is_null(), "{bot}");
        assert!(bot["system_prompt"].is_null(), "{bot}");
        assert!(bot["enabled_toolsets"].is_null(), "{bot}");
    }

    fn fix_r5_request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
        let builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "127.0.0.1");
        match body {
            Some(body) => builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        }
    }

    #[test]
    fn fix_r5_readiness_backend_url_prefers_config_then_env_then_default() {
        let with_config = json!({"appserver_url": " ws://cfg:1 "});
        assert_eq!(
            readiness_backend_url(&with_config, Some("ws://env:2")),
            "ws://cfg:1"
        );
        let empty = json!({});
        assert_eq!(
            readiness_backend_url(&empty, Some("ws://env:2")),
            "ws://env:2"
        );
        assert_eq!(
            readiness_backend_url(&empty, Some("  ")),
            DEFAULT_BACKEND_APPSERVER_URL
        );
        assert_eq!(
            readiness_backend_url(&empty, None),
            DEFAULT_BACKEND_APPSERVER_URL
        );
    }

    #[tokio::test]
    async fn fix_r5_readiness_fails_when_chat_backend_is_down() {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead_addr = probe.local_addr().unwrap();
        drop(probe);

        let mut state = test_state().await.with_disk_sampler(|_| {
            (
                Some(1_000_000 * readiness::DISK_BYTES_PER_MB),
                Some(900_000 * readiness::DISK_BYTES_PER_MB),
            )
        });
        state.config["appserver_url"] = json!(format!("ws://{dead_addr}"));

        // An explicitly configured backend gates readiness even without a chat runtime.
        let without_chat = router(state.clone())
            .oneshot(fix_r5_request("GET", "/api/readiness", None))
            .await
            .unwrap();
        assert_eq!(without_chat.status(), StatusCode::SERVICE_UNAVAILABLE);

        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        state.multiplexer = Some(SessionMultiplexer::new(
            state.pool.clone(),
            Arc::new(U65RecordingRunner { events: event_tx }),
            omon_gateway::MultiplexerConfig::default(),
        ));
        let with_chat = router(state)
            .oneshot(fix_r5_request("GET", "/api/readiness", None))
            .await
            .unwrap();
        assert_eq!(with_chat.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = json_body(with_chat).await;
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["checks"]["backend"], false);
        assert_eq!(body["checks"]["database"], true);
    }

    #[tokio::test]
    async fn fix_r5_cron_put_without_payload_keeps_payload_and_session_key() {
        let state = test_state().await;
        let payload = json!({
            "content": "ping",
            "last_status": "succeeded",
            "repeat": {"times": 5, "completed": 2}
        });
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('web:keep', 'web', 'keep', 'dashboard', '{}')")
            .execute(&state.pool)
            .await
            .unwrap();
        let mut spec = CronJobSpec::new("@every 5m", payload.clone());
        spec.session_key = Some("web:keep".into());
        state
            .scheduler
            .register_with_id("keep", spec)
            .await
            .unwrap();
        let app = router(state.clone());

        let schedule_only = app
            .clone()
            .oneshot(fix_r5_request(
                "PUT",
                "/api/cron/jobs/keep",
                Some(json!({"expression":"@every 10m"})),
            ))
            .await
            .unwrap();
        assert_eq!(schedule_only.status(), StatusCode::OK);
        let job = state.scheduler.get("keep").await.unwrap().unwrap();
        assert_eq!(job.expression, "@every 10m");
        assert_eq!(job.payload().unwrap(), payload);
        assert_eq!(job.session_key.as_deref(), Some("web:keep"));

        // A new payload keeps scheduler-owned state and the repeat counter.
        let new_payload = app
            .oneshot(fix_r5_request(
                "PUT",
                "/api/cron/jobs/keep",
                Some(json!({"payload":{"content":"new","repeat":{"times":10}}})),
            ))
            .await
            .unwrap();
        assert_eq!(new_payload.status(), StatusCode::OK);
        let job = state.scheduler.get("keep").await.unwrap().unwrap();
        let stored = job.payload().unwrap();
        assert_eq!(stored["content"], "new");
        assert_eq!(stored["last_status"], "succeeded");
        assert_eq!(stored["repeat"]["times"], 10);
        assert_eq!(stored["repeat"]["completed"], 2);
        assert_eq!(job.session_key.as_deref(), Some("web:keep"));
    }

    #[tokio::test]
    async fn fix_r5_cron_put_on_paused_job_keeps_it_paused_without_next_run() {
        let state = test_state().await;
        state
            .scheduler
            .register_with_id(
                "paused",
                CronJobSpec::new("@every 5m", json!({"content": "x"})),
            )
            .await
            .unwrap();
        state.scheduler.pause("paused").await.unwrap();
        let response = router(state.clone())
            .oneshot(fix_r5_request(
                "PUT",
                "/api/cron/jobs/paused",
                Some(json!({"expression":"@every 10m"})),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let job = state.scheduler.get("paused").await.unwrap().unwrap();
        assert!(!job.enabled);
        assert!(job.next_run_at.is_none());
    }

    #[tokio::test]
    async fn fix_r5_cron_create_rejects_empty_id() {
        let response = router(test_state().await)
            .oneshot(fix_r5_request(
                "POST",
                "/api/cron/jobs",
                Some(json!({"id":"  ","expression":"@every 5m","payload":{"content":"x"}})),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn fix_r5_cron_list_degrades_per_row_on_corrupt_payload() {
        let state = test_state().await;
        state
            .scheduler
            .register_with_id(
                "good",
                CronJobSpec::new("@every 5m", json!({"content": "x"})),
            )
            .await
            .unwrap();
        sqlx::query("INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority, created_at, updated_at) VALUES ('bad', '@every 5m', '{not json', 1, 'omon_owned', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .execute(&state.pool)
            .await
            .unwrap();
        let response = router(state)
            .oneshot(fix_r5_request("GET", "/api/cron/jobs", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        let bad = items.iter().find(|item| item["id"] == "bad").unwrap();
        assert!(bad["payload"].is_null());
        assert!(bad["error"].as_str().is_some_and(|e| !e.is_empty()));
        let good = items.iter().find(|item| item["id"] == "good").unwrap();
        assert!(good["error"].is_null());
    }

    #[tokio::test]
    async fn fix_r5_cron_runs_do_not_expose_claim_token() {
        let state = test_state().await;
        state
            .scheduler
            .register_with_id(
                "leased",
                CronJobSpec::new("@every 5m", json!({"content": "x"})),
            )
            .await
            .unwrap();
        sqlx::query("INSERT INTO cron_runs (run_id, job_id, claim_token, lease_expires_at, started_at, status, attempt) VALUES ('run-1', 'leased', 'secret-fencing-token', '2099-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'running', 1)")
            .execute(&state.pool)
            .await
            .unwrap();
        let response = router(state)
            .oneshot(fix_r5_request("GET", "/api/cron/runs", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["items"][0]["run_id"], "run-1");
        assert!(body["items"][0].get("claim_token").is_none());
        assert!(!body.to_string().contains("secret-fencing-token"));
    }

    #[tokio::test]
    async fn fix_r5_trigger_missing_job_is_404_and_detached_messenger_job_is_refused() {
        let state = test_state().await.detached_from_gateway();
        state
            .scheduler
            .register_with_id(
                "discord-job",
                CronJobSpec::new(
                    "@every 5m",
                    json!({"content": "x", "deliver": "discord:999"}),
                ),
            )
            .await
            .unwrap();
        let app = router(state.clone());
        let missing = app
            .clone()
            .oneshot(fix_r5_request("POST", "/api/cron/jobs/nope/trigger", None))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let refused = app
            .oneshot(fix_r5_request(
                "POST",
                "/api/cron/jobs/discord-job/trigger",
                None,
            ))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::CONFLICT);
        let runs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_runs")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(runs, 0, "a refused trigger must not start a run");
    }

    #[test]
    fn fix_r5_like_pattern_escapes_wildcards() {
        assert_eq!(like_contains_pattern("abc"), "%abc%");
        assert_eq!(like_contains_pattern("a%b_c\\d"), "%a\\%b\\_c\\\\d%");
    }

    #[tokio::test]
    async fn fix_r5_session_search_matches_wildcards_literally() {
        let state = test_state().await;
        for (key, channel) in [("k1", "c_1"), ("k2", "cx1")] {
            sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', ?, '42', '{}')")
                .bind(key)
                .bind(channel)
                .execute(&state.pool)
                .await
                .unwrap();
        }
        let response = router(state)
            .oneshot(fix_r5_request("GET", "/api/sessions?search=c_1", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["total"], 1);
        assert_eq!(body["items"][0]["id"], "k1");
    }

    #[test]
    fn fix_r5_session_socket_forwards_expiry_of_its_own_approvals_only() {
        let mine = SessionKey::new("web", None::<String>, "mine", None::<String>, "dashboard");
        let other = SessionKey::new("web", None::<String>, "other", None::<String>, "dashboard");
        let storage = mine.storage_key();
        let (mine_id, other_id) = (Uuid::new_v4(), Uuid::new_v4());
        let request = |session: &SessionKey, request_id: Uuid| OutboundAction::ApprovalRequest {
            session: session.clone(),
            request_id,
            command: "ls".into(),
            reason: "test".into(),
        };
        let mut tracked = HashSet::new();
        assert!(session_event_matches(
            &request(&mine, mine_id),
            &storage,
            &mut tracked
        ));
        assert!(!session_event_matches(
            &request(&other, other_id),
            &storage,
            &mut tracked
        ));
        assert!(!session_event_matches(
            &OutboundAction::ExpireApproval {
                request_id: other_id
            },
            &storage,
            &mut tracked
        ));
        assert!(session_event_matches(
            &OutboundAction::ExpireApproval {
                request_id: mine_id
            },
            &storage,
            &mut tracked
        ));
        // Already forwarded: a duplicate expiry is not forwarded again.
        assert!(!session_event_matches(
            &OutboundAction::ExpireApproval {
                request_id: mine_id
            },
            &storage,
            &mut tracked
        ));
    }

    #[test]
    fn fix_r5_log_ids_are_strictly_increasing_in_snapshot_and_broadcast() {
        let logs = DashboardLogStore::default();
        let mut receiver = logs.subscribe();
        for n in 0..5 {
            logs.push("INFO", "test", format!("entry {n}"), json!({}));
        }
        let snapshot_ids: Vec<u64> = logs.snapshot().iter().map(|entry| entry.id).collect();
        assert_eq!(snapshot_ids.len(), 5);
        assert!(snapshot_ids.windows(2).all(|pair| pair[0] < pair[1]));
        let mut last = 0;
        for _ in 0..5 {
            let entry = receiver.try_recv().unwrap();
            assert!(entry.id > last);
            last = entry.id;
        }
    }
}

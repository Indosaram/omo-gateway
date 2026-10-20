// allow: SIZE_OK — OMO WebSocket protocol state machine
use std::collections::{hash_map::Entry, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex as ParkingMutex;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::agent_workspace::{agent_workspace_slug, resolve_workspace, AgentWorkspace};
use super::backend::AgentBackend;

use super::omo_config::OmoBackendConfig;
use super::omo_protocol::{
    approval_allow_response, approval_denial_response, approval_denial_with_reason,
    initialize_request, is_approval_request, thread_resume_request, thread_start_request,
    turn_start_request, turn_steer_request, user_input_cancellation,
};
use crate::models::{
    filter_intent_gate, filter_reasoning, is_explicit_silence, render_user_prompt, InboundEvent,
    OutboundAction, SessionContext, StreamChunk,
};
use crate::{OmonError, OutboundDispatcher, Result};

type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

mod follow;

/// Approval denials tolerated per turn before the gateway aborts it: a
/// policy-denial loop otherwise burns the entire turn deadline flailing.
pub const APPROVAL_DENIAL_TURN_LIMIT: u32 = 5;

#[allow(dead_code)]
pub const DEFAULT_FALLBACK_MODEL: &str = crate::agent::omo_config::DEFAULT_FALLBACK_MODEL;

fn is_model_rejected_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("model not found")
        || lower.contains("unknown model")
        || lower.contains("unknown_model")
        || lower.contains("model rejected")
        || lower.contains("model update rejected")
        || lower.contains("rejected the model")
        || lower.contains("invalid model")
        || lower.contains("unsupported model")
        || lower.contains("model_not_found")
        || lower.contains("thread/settings/update")
}

/// The engine refuses a turn/start while another turn is still processing on the thread
/// (observed verbatim: "Agent is already processing. Specify streamingBehavior ...").
/// A retry submitted in that window must wait for the running turn instead of failing.
fn is_thread_busy_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("already processing")
}

/// Owner-facing note for a proposal whose turn ended before the owner answered anything: the
/// card is retired, so saying "your answer arrived" would describe an answer that never happened.
fn turn_ended_ask_notice(proposal_id: i64) -> String {
    format!(
        "proposal #{proposal_id}: that turn ended before you answered, so the action did not run. Ask again to re-issue it."
    )
}

/// Owner-facing note for a proposal the owner did answer, but only after the turn that asked for
/// it had already ended.
fn late_ask_notice(proposal_id: i64) -> String {
    format!(
        "proposal #{proposal_id}: your answer arrived after that turn ended, so the action did not run. Ask again to re-issue it."
    )
}

/// Turn-scoped ownership of the approval cards a turn opened. Dropping it — on every return
/// path of `execute_turn`, including cancellation — cancels the token, so each spawned Ask
/// task settles its card and expires its proposal instead of outliving the turn.
struct TurnAskScope {
    token: CancellationToken,
}

impl TurnAskScope {
    fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl Drop for TurnAskScope {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Expires a proposal the turn abandoned, unless the owner already decided it. Returns whether
/// the proposal was expired here. One conditional statement, never read-then-write: the owner's
/// answer can land between a read and an update, and overwriting an approved proposal with
/// `expired` would silently cancel an action the owner had just allowed.
async fn expire_abandoned_proposal(
    pool: &SqlitePool,
    proposal_id: i64,
) -> std::result::Result<bool, sqlx::Error> {
    let expired = sqlx::query(
        "UPDATE action_proposals
         SET state = ?, decided_at = CURRENT_TIMESTAMP
         WHERE id = ? AND state = ?",
    )
    .bind(crate::autonomy::ProposalState::Expired.as_str())
    .bind(proposal_id)
    .bind(crate::autonomy::ProposalState::Pending.as_str())
    .execute(pool)
    .await?
    .rows_affected();
    Ok(expired == 1)
}

/// The turn id a frame is about, in either of the shapes the app-server uses.
fn frame_turn_id(frame: &Value) -> Option<&str> {
    frame
        .pointer("/params/turnId")
        .or_else(|| frame.pointer("/params/turn/id"))
        .and_then(Value::as_str)
}

/// What the turn loop does next after a frame that arrived before `turn/start` was acknowledged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreAckAction {
    /// Nothing to do; keep waiting for the acknowledgement.
    Continue,
    /// The refusal was for a thread that is now idle: submit the same prompt once more.
    Resubmit,
}

/// Pre-acknowledgement protocol state: the turn boundaries seen on the thread, and whether a
/// `turn/start` refusal is waiting for the thread to go idle. The refusal only clears on a
/// terminal frame for *this* thread, and a completion already seen before the refusal counts,
/// so a prompt refused while the engine was finishing never waits for a frame that will not come.
#[derive(Default)]
struct PreAckState {
    /// turn/started and turn/completed frames, in arrival order, as (is_start, turn_id).
    events: Vec<(bool, String)>,
    /// A turn/start refusal asked this loop to wait for the thread to go idle.
    awaiting_idle: bool,
    /// The refusal was already answered with one resubmission.
    resubmitted: bool,
}

impl PreAckState {
    /// Folds a `turn/start` refusal. True when the loop must wait for the thread to go idle and
    /// then resubmit instead of failing the turn; false for every other error and for a second
    /// refusal of the same submission.
    fn note_refusal(&mut self, error: &str) -> bool {
        if self.resubmitted || !is_thread_busy_error(error) {
            return false;
        }
        self.resubmitted = true;
        self.awaiting_idle = true;
        true
    }

    /// True when the thread was already idle when the refusal arrived: the last boundary seen
    /// is a completion, so there is no running turn left to wait for.
    fn idle_already(&self) -> bool {
        self.awaiting_idle && self.events.last().is_some_and(|(is_start, _)| !is_start)
    }

    /// Folds one pre-acknowledgement frame and says whether the prompt must be submitted again.
    /// Frames for another thread are ignored: only this thread's idle state matters.
    fn observe(&mut self, frame: &Value, thread_id: &str) -> PreAckAction {
        if frame.pointer("/params/threadId").and_then(Value::as_str) != Some(thread_id) {
            return PreAckAction::Continue;
        }
        let Some(turn) = frame_turn_id(frame) else {
            return PreAckAction::Continue;
        };
        match frame.get("method").and_then(Value::as_str).unwrap_or("") {
            "turn/started" => self.events.push((true, turn.to_string())),
            "turn/completed" => {
                self.events.push((false, turn.to_string()));
                if self.awaiting_idle {
                    self.awaiting_idle = false;
                    return PreAckAction::Resubmit;
                }
            }
            _ => {}
        }
        PreAckAction::Continue
    }
}

/// Extra slack beyond a turn's own total deadline before a refusal-guard entry
/// is presumed dead and a new submission may take its place.
#[allow(dead_code)]
pub const ACTIVE_TURN_STALE_MARGIN: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTurn {
    pub thread_id: String,
    /// None means submission may have reached the peer but no start ACK was observed.
    pub turn_id: Option<String>,
    /// When this submission was recorded, consumed by the stale-entry reclaim.
    pub inserted_at: tokio::time::Instant,
    pub open_tools: usize,
}

/// A guard entry older than the turn's total deadline plus the stale margin
/// cannot be a live turn: the deadline has elapsed, so reclaim the entry
/// instead of bricking the session until the next gateway restart.
#[allow(dead_code)]
fn active_turn_entry_is_stale(age: Duration, effective_total_timeout: Duration) -> bool {
    age > effective_total_timeout + ACTIVE_TURN_STALE_MARGIN
}

/// Text of a finished agent message frame (`item/completed` of an `agentMessage`).
fn completed_agent_message_text<'a>(method: &str, frame: &'a Value) -> Option<&'a str> {
    if method != "item/completed"
        || frame.pointer("/params/item/type").and_then(Value::as_str) != Some("agentMessage")
    {
        return None;
    }
    frame.pointer("/params/item/text").and_then(Value::as_str)
}

/// Whether a frame of the followed turn shows the agent working on the prompt.
/// Turn bookkeeping and user-message echoes (including the gateway's own
/// progress steers) do not count.
fn is_agent_progress(method: &str, frame: &Value) -> bool {
    let agent_frame =
        method.starts_with("item/") || matches!(method, "turn/plan/updated" | "turn/diff/updated");
    agent_frame && frame.pointer("/params/item/type").and_then(Value::as_str) != Some("userMessage")
}

/// Whether a frame shows the followed thread/turn doing real work, so it may renew
/// the inactivity watchdog and the interactive total deadline. Responses to
/// gateway-issued requests (progress steers, interrupts), thread-status
/// notifications and frames of other threads or turns are not progress: counting
/// them would let the gateway's own steer acks keep a hung turn alive forever.
fn frame_is_turn_progress(
    method: &str,
    frame: &Value,
    thread_id: &str,
    turn_id: Option<&str>,
    awaiting_successor: bool,
) -> bool {
    if method.is_empty() {
        return false;
    }
    match frame.pointer("/params/threadId").and_then(Value::as_str) {
        Some(frame_thread) if frame_thread != thread_id => return false,
        None if !is_approval_request(method) => return false,
        _ => {}
    }
    if !(method.starts_with("turn/")
        || is_agent_progress(method, frame)
        || is_approval_request(method))
    {
        return false;
    }
    let frame_turn = frame
        .pointer("/params/turnId")
        .or_else(|| frame.pointer("/params/turn/id"))
        .and_then(Value::as_str);
    match (frame_turn, turn_id) {
        (Some(frame_turn), Some(current)) if frame_turn != current => {
            awaiting_successor && method == "turn/started"
        }
        _ => true,
    }
}

/// Makes sure a cron session's `.omo/settings.json` disables the `ttsr` extension
/// without clobbering anything else: an unreadable or non-JSON file is left alone,
/// an already-correct file is not rewritten, and a rewrite goes through a temp file.
async fn ensure_cron_ttsr_disabled(settings_path: &std::path::Path) {
    let mut settings: serde_json::Map<String, Value> = match tokio::fs::read_to_string(
        settings_path,
    )
    .await
    {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(map) => map,
            Err(error) => {
                tracing::warn!(path = %settings_path.display(), %error, "cron settings.json is not valid JSON; leaving it untouched");
                return;
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(error) => {
            tracing::warn!(path = %settings_path.display(), %error, "cannot read cron settings.json; leaving it untouched");
            return;
        }
    };
    let disabled = settings
        .entry("disabledBuiltinExtensions".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(entries) = disabled else {
        tracing::warn!(path = %settings_path.display(), "disabledBuiltinExtensions is not an array; leaving cron settings.json untouched");
        return;
    };
    if entries.iter().any(|entry| entry.as_str() == Some("ttsr")) {
        return;
    }
    entries.push(Value::String("ttsr".to_string()));
    let Ok(content) = serde_json::to_string_pretty(&Value::Object(settings)) else {
        return;
    };
    let tmp_path = settings_path.with_extension("json.tmp");
    if let Err(error) = tokio::fs::write(&tmp_path, content.as_bytes()).await {
        tracing::warn!(path = %tmp_path.display(), %error, "failed to write cron settings.json");
        return;
    }
    if let Err(error) = tokio::fs::rename(&tmp_path, settings_path).await {
        tracing::warn!(path = %settings_path.display(), %error, "failed to replace cron settings.json");
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }
}

/// Interactive (Discord) turns stop at 1800s (30m); cron turns keep their own budget.
const INTERACTIVE_TURN_CAP: Duration = Duration::from_secs(1800);

pub type UtcClock = Arc<dyn Fn() -> chrono::DateTime<chrono::Utc> + Send + Sync>;

#[derive(Clone)]
pub struct OmoBackend {
    pub config: OmoBackendConfig,
    pub pool: Option<SqlitePool>,
    pub dispatcher: Arc<dyn OutboundDispatcher>,
    pub thread_ids: Arc<ParkingMutex<HashMap<String, String>>>,
    pub active_turns: Arc<ParkingMutex<HashMap<String, ActiveTurn>>>,
    pub approval_requester: Option<Arc<crate::DiscordApprovalRequester>>,
    pub ledger_enabled: Option<bool>,
    pub full_access: Option<bool>,
    pub clock: Option<UtcClock>,
    followers: Arc<ParkingMutex<HashMap<String, follow::Follower>>>,
    follow_engine_turns: bool,
}

impl OmoBackend {
    pub fn new(config: OmoBackendConfig, dispatcher: Arc<dyn OutboundDispatcher>) -> Self {
        Self {
            config,
            pool: None,
            dispatcher,
            thread_ids: Arc::new(ParkingMutex::new(HashMap::new())),
            active_turns: Arc::new(ParkingMutex::new(HashMap::new())),
            approval_requester: None,
            ledger_enabled: None,
            full_access: None,
            clock: None,
            followers: Arc::new(ParkingMutex::new(HashMap::new())),
            follow_engine_turns: false,
        }
    }

    pub fn with_pool(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn with_approval_requester(
        mut self,
        requester: Arc<crate::DiscordApprovalRequester>,
    ) -> Self {
        self.approval_requester = Some(requester);
        self
    }

    pub fn with_ledger_enabled(mut self, enabled: bool) -> Self {
        self.ledger_enabled = Some(enabled);
        self
    }

    /// Keeps a subscription open after each interactive turn so turns the engine starts on
    /// its own (a detached cell finishing, a monitor firing) reach the messenger.
    pub fn with_engine_turn_follow(mut self, enabled: bool) -> Self {
        self.follow_engine_turns = enabled;
        self
    }

    pub fn with_full_access(mut self, enabled: bool) -> Self {
        self.full_access = Some(enabled);
        self
    }

    pub fn with_clock<F>(mut self, clock: F) -> Self
    where
        F: Fn() -> chrono::DateTime<chrono::Utc> + Send + Sync + 'static,
    {
        self.clock = Some(Arc::new(clock));
        self
    }

    fn is_ledger_enabled(&self) -> bool {
        self.ledger_enabled
            .unwrap_or_else(crate::agent::agent_workspace::approval_ledger_enabled)
    }

    /// Writes the workspace permission preset when the action ledger is on for *this* backend.
    /// The backend's own flag decides — tests and embedded runtimes construct it with
    /// `with_ledger_enabled`, so a second read of `OMON_APPROVAL_LEDGER_ENABLED` would ignore
    /// their choice and leave the engine sending no approval frames to the ledger.
    async fn ensure_workspace_preset(&self, settings_path: &std::path::Path) -> bool {
        crate::agent::agent_workspace::ensure_workspace_permission_preset(
            settings_path,
            self.is_ledger_enabled(),
            crate::agent::agent_workspace::WORKSPACE_PERMISSION_PRESET,
        )
        .await
    }

    /// The owner granted every permission: engine approvals run as under YOLO (payments still ask).
    fn is_full_access(&self) -> bool {
        self.full_access
            .unwrap_or_else(crate::agent::agent_workspace::full_access_enabled)
    }

    fn now_utc(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock
            .as_ref()
            .map(|c| c())
            .unwrap_or_else(chrono::Utc::now)
    }

    async fn connect_ws(&self, deadline: tokio::time::Instant) -> Result<WsStream> {
        let mut request = self
            .config
            .appserver_url
            .as_str()
            .into_client_request()
            .map_err(|e| OmonError::Config(format!("invalid appserver url: {e}")))?;

        if let Some(token) = &self.config.auth_token {
            let header_val = format!("Bearer {token}")
                .parse()
                .map_err(|e| OmonError::Config(format!("invalid auth token header: {e}")))?;
            request.headers_mut().insert("Authorization", header_val);
        }

        let mut last_err = None;
        let retry_limit = deadline.min(tokio::time::Instant::now() + Duration::from_secs(15));
        let mut attempt = 0;
        while tokio::time::Instant::now() < retry_limit {
            attempt += 1;
            if attempt > 1 {
                let sleep_until =
                    (tokio::time::Instant::now() + Duration::from_millis(500)).min(retry_limit);
                tokio::time::sleep_until(sleep_until).await;
                if tokio::time::Instant::now() >= retry_limit {
                    break;
                }
            }
            let connect_fut = tokio_tungstenite::connect_async(request.clone());
            let timeout_at =
                deadline.min(tokio::time::Instant::now() + self.config.connect_timeout);
            match tokio::time::timeout_at(timeout_at, connect_fut).await {
                Ok(Ok((ws, _))) => return Ok(ws),
                Ok(Err(e)) => {
                    last_err = Some(OmonError::Llm(format!(
                        "failed to connect to omo app-server at {}: {e}",
                        self.config.appserver_url
                    )));
                }
                Err(_) => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err(OmonError::Llm("turn exceeded total deadline".into()));
                    }
                    last_err = Some(OmonError::Llm(format!(
                        "timeout connecting to omo app-server at {}",
                        self.config.appserver_url
                    )));
                }
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(OmonError::Llm("turn exceeded total deadline".into()));
        }

        Err(last_err.unwrap_or_else(|| {
            OmonError::Llm(format!(
                "failed to connect to omo app-server at {}",
                self.config.appserver_url
            ))
        }))
    }

    async fn do_initialize(&self, ws: &mut WsStream, deadline: tokio::time::Instant) -> Result<()> {
        if tokio::time::Instant::now() >= deadline {
            return Err(OmonError::Llm("turn exceeded total deadline".into()));
        }
        ws.send(initialize_request())
            .await
            .map_err(|e| OmonError::Llm(format!("failed to send initialize: {e}")))?;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(OmonError::Llm(
                    "turn exceeded total deadline: timeout waiting for initialize response".into(),
                ));
            }
            let timeout_at =
                deadline.min(tokio::time::Instant::now() + self.config.request_timeout);
            let Some(msg) = tokio::time::timeout_at(timeout_at, ws.next())
                .await
                .map_err(|_| {
                    if tokio::time::Instant::now() >= deadline {
                        OmonError::Llm(
                            "turn exceeded total deadline: timeout waiting for initialize response"
                                .into(),
                        )
                    } else {
                        OmonError::Llm("timeout waiting for initialize response".into())
                    }
                })?
            else {
                break;
            };

            let msg =
                msg.map_err(|e| OmonError::Llm(format!("ws error during initialize: {e}")))?;
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(text.as_str())
                    .map_err(|e| OmonError::Llm(format!("invalid json: {e}")))?;
                if val.get("id").and_then(Value::as_u64) == Some(1) {
                    if let Some(err) = val.get("error") {
                        return Err(OmonError::Llm(format!("initialize error: {err}")));
                    }
                    return Ok(());
                }
            }
        }
        Err(OmonError::Llm("closed before initialize response".into()))
    }

    /// Resolves the remote OMO app-server thread ID for the session.
    ///
    /// # Persona & Model Flow across Session Lifecycle:
    ///
    /// 1. **Initial Thread Creation (`thread/start`)**:
    ///    When a session does not yet have an assigned `omo_thread_id` in session metadata,
    ///    `thread/start` is dispatched with:
    ///    - `developerInstructions`: Initial `session.state.system_prompt` resolved from the bot
    ///      profile route or database profile override.
    ///    - `model`: `session.state.active_model` or fallback `config.default_model`.
    ///
    ///    The returned `thread.id` is saved into `session.state.metadata["omo_thread_id"]`
    ///    and cached in `self.thread_ids`.
    ///
    /// 2. **Mid-Session Profile or Model Changes**:
    ///    - **Model Changes**: The active model (`session.state.active_model`) is forwarded per-turn
    ///      in `turn/start(model: ...)`. Any runtime `/model` switch or updated route model takes
    ///      effect immediately on the next turn on the existing remote thread.
    ///    - **System Prompt / Persona Changes**: Per the OMO app-server protocol, `developerInstructions`
    ///      are bound immutably to the remote thread at `thread/start`. If a profile's system prompt
    ///      changes mid-session, the existing thread retains its initial developer instructions.
    ///      To re-bind updated system instructions, a new session thread must be started (e.g. by
    ///      clearing `omo_thread_id` from session metadata or establishing a fresh conversation).
    async fn resolve_thread_id(
        &self,
        ws: &mut WsStream,
        session: &mut SessionContext,
        deadline: tokio::time::Instant,
    ) -> Result<String> {
        let storage_key = session.key.storage_key();
        let is_cron = session.key.user_id.starts_with("cron:")
            || session
                .state
                .metadata
                .get("cron_scheduler_delivery")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let is_lane = session.key.platform == "lane";
        if !is_cron && session.key.thread_id.is_none() {
            if let (Some(pool), Some(id)) = (
                &self.pool,
                session
                    .state
                    .metadata
                    .get("omo_thread_id")
                    .and_then(Value::as_str)
                    .map(String::from),
            ) {
                if thread_owns_conversation(pool, session, &id).await? {
                    tracing::warn!(session = %session.key, omo_thread_id = %id, "conversation belongs to a thread of this channel; starting a fresh one");
                    session.state.metadata.remove("omo_thread_id");
                    self.thread_ids.lock().remove(&storage_key);
                    clear_session_binding(pool, session).await?;
                }
            }
        }
        if !is_cron && !is_lane && self.config.per_agent_workspace {
            // A resumed thread reloads project settings, so the preset must be in place before
            // thread/resume as well as before thread/start.
            if let Some(root) = &self.config.workspace_root {
                let slug = agent_workspace_slug(
                    &session.key.platform,
                    &session.key.user_id,
                    session.key.bot_id.as_deref(),
                );
                if slug.starts_with("bot-") {
                    let omo_dir = resolve_workspace(root, &slug).cwd.join(".omo");
                    if tokio::fs::create_dir_all(&omo_dir).await.is_ok() {
                        self.ensure_workspace_preset(&omo_dir.join("settings.json"))
                            .await;
                    }
                }
            }
        }
        if is_cron {
            session.state.metadata.remove("omo_thread_id");
            self.thread_ids.lock().remove(&storage_key);
        } else if let Some(id) = session
            .state
            .metadata
            .get("omo_thread_id")
            .and_then(Value::as_str)
            .map(String::from)
        {
            if tokio::time::Instant::now() >= deadline {
                return Err(OmonError::Llm("turn exceeded total deadline".into()));
            }
            ws.send(thread_resume_request(&id)).await.map_err(|error| {
                OmonError::Llm(format!("failed to send thread/resume: {error}"))
            })?;
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(OmonError::Llm(
                        "turn exceeded total deadline: timeout waiting for thread/resume response"
                            .into(),
                    ));
                }
                let timeout_at =
                    deadline.min(tokio::time::Instant::now() + self.config.request_timeout);
                let Some(message) = tokio::time::timeout_at(timeout_at, ws.next())
                    .await
                    .map_err(|_| {
                        if tokio::time::Instant::now() >= deadline {
                            OmonError::Llm(
                                "turn exceeded total deadline: timeout waiting for thread/resume response".into(),
                            )
                        } else {
                            OmonError::Llm("timeout waiting for thread/resume response".into())
                        }
                    })?
                else {
                    break;
                };
                let message = message.map_err(|error| {
                    OmonError::Llm(format!("ws error in thread/resume: {error}"))
                })?;
                if let Message::Text(text) = message {
                    let response: Value = serde_json::from_str(text.as_str())
                        .map_err(|error| OmonError::Llm(format!("invalid json: {error}")))?;
                    if let Some(reply) = user_input_cancellation(&response) {
                        ws.send(reply).await.map_err(|error| {
                            OmonError::Llm(format!("user input cancellation failed: {error}"))
                        })?;
                        continue;
                    }
                    if response.get("id").and_then(Value::as_u64) == Some(2) {
                        if let Some(error) = response.get("error") {
                            let message = error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            if message.contains("no rollout found")
                                || message.contains("thread not found")
                            {
                                session.state.metadata.remove("omo_thread_id");
                                self.thread_ids.lock().remove(&storage_key);
                                return Err(OmonError::Llm(format!(
                                    "continuity_error: remote thread rollout missing for {id}: {message}"
                                )));
                            }
                            return Err(OmonError::Llm(format!("thread/resume error: {error}")));
                        }
                        session
                            .state
                            .metadata
                            .insert("omo_thread_id".into(), json!(id));
                        self.thread_ids.lock().insert(storage_key, id.clone());
                        if !is_cron {
                            if let Some(pool) = &self.pool {
                                persist_session_binding(pool, session, &id).await?;
                            }
                        }
                        return Ok(id);
                    }
                }
            }
            return Err(OmonError::Llm(
                "closed before thread/resume response".into(),
            ));
        }

        let model = session
            .state
            .active_model
            .as_deref()
            .or(self.config.default_model.as_deref());

        let workspace = if is_lane {
            let lane_cwd_str = session
                .state
                .metadata
                .get("lane_cwd")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    OmonError::Config("lane session missing required 'lane_cwd' metadata".into())
                })?;
            let cwd_path = PathBuf::from(lane_cwd_str);
            if !cwd_path.is_absolute() {
                return Err(OmonError::Config(format!(
                    "lane_cwd must be an absolute path: {}",
                    cwd_path.display()
                )));
            }
            if !tokio::fs::try_exists(&cwd_path).await.unwrap_or(false) {
                return Err(OmonError::Config(format!(
                    "lane_cwd does not exist: {}",
                    cwd_path.display()
                )));
            }
            let is_dir = match tokio::fs::metadata(&cwd_path).await {
                Ok(m) => m.is_dir(),
                Err(e) => {
                    return Err(OmonError::Config(format!("lane_cwd cannot be read: {e}")));
                }
            };
            if !is_dir {
                return Err(OmonError::Config(format!(
                    "lane_cwd is not a directory: {}",
                    cwd_path.display()
                )));
            }
            Some(AgentWorkspace {
                cwd: cwd_path.clone(),
                roots: vec![cwd_path],
            })
        } else if self.config.per_agent_workspace {
            if let Some(root) = &self.config.workspace_root {
                let slug = agent_workspace_slug(
                    &session.key.platform,
                    &session.key.user_id,
                    session.key.bot_id.as_deref(),
                );
                let ws = resolve_workspace(root, &slug);
                tokio::fs::create_dir_all(&ws.cwd).await.map_err(|err| {
                    OmonError::Config(format!(
                        "failed to provision agent workspace at {}: {err}",
                        ws.cwd.display()
                    ))
                })?;
                let shared_dir = root.join("shared");
                tokio::fs::create_dir_all(&shared_dir)
                    .await
                    .map_err(|err| {
                        OmonError::Config(format!(
                            "failed to provision agent workspace at {}: {err}",
                            shared_dir.display()
                        ))
                    })?;
                let omo_dir = ws.cwd.join(".omo");
                tokio::fs::create_dir_all(&omo_dir).await.map_err(|err| {
                    OmonError::Config(format!(
                        "failed to provision agent workspace at {}: {err}",
                        omo_dir.display()
                    ))
                })?;
                let omo_json_path = omo_dir.join("omo.json");
                if !tokio::fs::try_exists(&omo_json_path).await.unwrap_or(false) {
                    let content = serde_json::to_string_pretty(&json!({
                        "memory": {
                            "agent": slug
                        }
                    }))
                    .map_err(|err| {
                        OmonError::Config(format!(
                            "failed to provision agent workspace at {}: {err}",
                            omo_json_path.display()
                        ))
                    })?;
                    tokio::fs::write(&omo_json_path, content.as_bytes())
                        .await
                        .map_err(|err| {
                            OmonError::Config(format!(
                                "failed to provision agent workspace at {}: {err}",
                                omo_json_path.display()
                            ))
                        })?;
                }
                if is_cron {
                    ensure_cron_ttsr_disabled(&omo_dir.join("settings.json")).await;
                } else if slug.starts_with("bot-") {
                    self.ensure_workspace_preset(&omo_dir.join("settings.json"))
                        .await;
                }
                Some(ws)
            } else {
                None
            }
        } else {
            None
        };

        if tokio::time::Instant::now() >= deadline {
            return Err(OmonError::Llm("turn exceeded total deadline".into()));
        }

        ws.send(thread_start_request(
            session.state.system_prompt.as_deref(),
            model,
            workspace.as_ref(),
            session.state.enabled_toolsets.as_deref(),
        ))
        .await
        .map_err(|e| OmonError::Llm(format!("failed to send thread/start: {e}")))?;

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(OmonError::Llm(
                    "turn exceeded total deadline: timeout waiting for thread/start response"
                        .into(),
                ));
            }
            let timeout_at =
                deadline.min(tokio::time::Instant::now() + self.config.request_timeout);
            let Some(msg) = tokio::time::timeout_at(timeout_at, ws.next())
                .await
                .map_err(|_| {
                    if tokio::time::Instant::now() >= deadline {
                        OmonError::Llm(
                            "turn exceeded total deadline: timeout waiting for thread/start response".into(),
                        )
                    } else {
                        OmonError::Llm("timeout waiting for thread/start response".into())
                    }
                })?
            else {
                break;
            };
            let msg = msg.map_err(|e| OmonError::Llm(format!("ws error in thread/start: {e}")))?;
            if let Message::Text(text) = msg {
                let val: Value = serde_json::from_str(text.as_str())
                    .map_err(|e| OmonError::Llm(format!("invalid json: {e}")))?;
                if val.get("id").and_then(Value::as_u64) == Some(2) {
                    if let Some(err) = val.get("error") {
                        return Err(OmonError::Llm(format!("thread/start error: {err}")));
                    }
                    if let Some(id) = val.pointer("/result/thread/id").and_then(Value::as_str) {
                        let id_str = id.to_string();
                        if !is_cron {
                            session
                                .state
                                .metadata
                                .insert("omo_thread_id".into(), json!(id_str));
                            self.thread_ids.lock().insert(storage_key, id_str.clone());
                            if let Some(pool) = &self.pool {
                                persist_session_binding(pool, session, &id_str).await?;
                            }
                        }
                        return Ok(id_str);
                    }
                }
            }
        }
        Err(OmonError::Llm(
            "failed to obtain thread id from thread/start".into(),
        ))
    }

    async fn emit_chunk(
        &self,
        session: &SessionContext,
        stream_id: Uuid,
        sequence: u64,
        content: String,
        is_final: bool,
        reply_to: Option<String>,
    ) -> Result<()> {
        let chunk = StreamChunk {
            stream_id,
            sequence,
            content,
            is_final,
            reply_to,
        };
        self.dispatcher
            .dispatch(OutboundAction::Stream {
                session: session.key.clone(),
                chunk,
            })
            .await
    }
}

impl OmoBackend {
    /// Shows mid-turn status text to the user as a normal message. It is neither a
    /// stream chunk nor persisted as the assistant reply; true only when dispatched.
    async fn relay_status(
        &self,
        session: &SessionContext,
        content: String,
        reply_to: Option<String>,
    ) -> bool {
        match self
            .dispatcher
            .dispatch(OutboundAction::SendMessage {
                session: session.key.clone(),
                content,
                reply_to,
            })
            .await
        {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(session = %session.key, %error, "failed to relay mid-turn status");
                false
            }
        }
    }

    async fn capture_lane_item(
        &self,
        session: &SessionContext,
        item: &Value,
        turn: &str,
    ) -> Result<()> {
        if session.key.platform != "lane"
            || item.get("type").and_then(Value::as_str) != Some("agentMessage")
        {
            return Ok(());
        }
        let Some(text) = item.get("text").and_then(Value::as_str) else {
            return Ok(());
        };
        let identity = item
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                use sha2::{Digest, Sha256};
                format!("{:x}", Sha256::digest(text.as_bytes()))
            });
        self.dispatcher
            .dispatch(OutboundAction::EditMessage {
                session: session.key.clone(),
                platform_message_id: format!("lane-item:{turn}:{identity}"),
                content: text.into(),
            })
            .await
    }

    async fn deliver_rendered_result(
        &self,
        session: &mut SessionContext,
        stream_id: Uuid,
        sequence: u64,
        rendered: &str,
        reply_to: Option<String>,
    ) -> Result<()> {
        session
            .state
            .metadata
            .insert("cron_agent_output".into(), json!(rendered));

        if session.key.platform == "lane"
            && session
                .state
                .metadata
                .get("lane_completed_item")
                .and_then(Value::as_bool)
                == Some(true)
        {
            return Ok(());
        }

        let suppress_emission = session
            .state
            .metadata
            .get("cron_suppress_direct_emission")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if suppress_emission {
            return Ok(());
        }

        let obl_id = format!("obl:turn:{}", stream_id);
        if let Some(pool) = &self.pool {
            ensure_session_row(pool, session).await?;
            let ledger = crate::DeliveryLedgerService::new(pool.clone());
            ledger
                .record_obligation(&obl_id, &session.key, rendered)
                .await?;
            ledger.mark_obligation_attempting(&obl_id).await?;
            persist_message(pool, session, rendered, &obl_id).await?;
        }

        let delivered = self
            .emit_chunk(
                session,
                stream_id,
                sequence,
                rendered.to_string(),
                true,
                reply_to.clone(),
            )
            .await;

        if let Some(pool) = &self.pool {
            let ledger = crate::DeliveryLedgerService::new(pool.clone());
            if delivered.is_ok() {
                ledger.mark_obligation_delivered(&obl_id).await?;
            } else if let Err(ref e) = delivered {
                ledger
                    .mark_obligation_failed(&obl_id, &e.to_string())
                    .await?;
            }
        }

        delivered?;
        Ok(())
    }

    async fn clear_stuck_thread(&self, session: &mut SessionContext) {
        let storage_key = session.key.storage_key();
        self.thread_ids.lock().remove(&storage_key);
        session.state.metadata.remove("omo_thread_id");
        if let Some(pool) = &self.pool {
            let _ = clear_session_binding(pool, session).await;
        }
    }

    async fn setup_turn(
        &self,
        session: &mut SessionContext,
        deadline: tokio::time::Instant,
        effective_total_timeout: Duration,
    ) -> Result<(WsStream, String)> {
        let mut ws = self.connect_ws(deadline).await?;
        self.do_initialize(&mut ws, deadline).await?;
        let thread_id = self.resolve_thread_id(&mut ws, session, deadline).await?;

        if tokio::time::Instant::now() >= deadline {
            return Err(OmonError::Llm(format!(
                "turn exceeded total deadline of {effective_total_timeout:?}"
            )));
        }

        Ok((ws, thread_id))
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_turn(
        &self,
        mut ws: WsStream,
        thread_id: String,
        session: &mut SessionContext,
        event: InboundEvent,
        deadline: tokio::time::Instant,
        effective_total_timeout: Duration,
        model_override: Option<&str>,
        fallback_eligible: &mut bool,
        side_effects: &mut bool,
        last_turn: &mut Option<String>,
        briefings: Option<&crate::mirror::CronBriefings>,
    ) -> Result<()> {
        let mut user_prompt = render_user_prompt(&event);
        // Other people's text in this turn (system deliveries such as the action loop, inbox and
        // GitHub, a non-owner author, or thread lines in the context pack) taints its actions.
        let owner_author = crate::context::owner_author_for(&event.session);
        let mut turn_tainted = event.origin != crate::EventOrigin::User
            || owner_author.is_none()
            || event.author_id.as_deref() != owner_author.as_deref();
        if let Some(pool) = &self.pool {
            if let Some(pack) = crate::context::pack_for_turn(pool, &event).await {
                turn_tainted |= crate::context::pack_has_foreign_text(&pack);
                user_prompt = crate::context::wrap_prompt(&pack, &user_prompt);
            }
        }
        // Cron deliveries are posted by the scheduler, not by this thread; the agent
        // gets the ones it has not seen yet so follow-up questions about them work.
        // They are consumed only once turn/start is acknowledged (see below).
        if let Some(briefings) = briefings {
            user_prompt = format!("{}\n{user_prompt}", briefings.text);
        }
        let model = model_override
            .map(str::to_owned)
            .or_else(|| session.state.active_model.clone())
            .or_else(|| self.config.default_model.clone());
        // Invariant: model changes flow only via active_model / turn/start params / thread/settings/update.
        // omo_config reads global omo.json but never writes it.
        // thread/start misses custom providers and turn/start may ignore model.
        // Await the custom-provider-aware settings RPC on every turn.
        if let Some(model) = model.as_deref() {
            let effort = std::env::var("OMON_DEFAULT_EFFORT")
                .or_else(|_| std::env::var("DEFAULT_EFFORT"))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "high".to_string());
            ws.send(Message::text(
                json!({
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "thread/settings/update",
                    "params": { "threadId": thread_id, "model": model, "effort": effort }
                })
                .to_string(),
            ))
            .await
            .map_err(|e| OmonError::Llm(format!("model update send failed: {e}")))?;
            loop {
                let frame = tokio::time::timeout_at(deadline, ws.next())
                    .await
                    .map_err(|_| OmonError::Llm("model update timed out".into()))?
                    .ok_or_else(|| OmonError::Llm("closed before model update".into()))?
                    .map_err(|e| OmonError::Llm(format!("model update transport error: {e}")))?;
                if let Message::Text(text) = frame {
                    let response: Value = serde_json::from_str(&text).map_err(|e| {
                        OmonError::Llm(format!("invalid model update response: {e}"))
                    })?;
                    if let Some(reply) = user_input_cancellation(&response) {
                        ws.send(reply).await.map_err(|error| {
                            OmonError::Llm(format!("user input cancellation failed: {error}"))
                        })?;
                        continue;
                    }
                    if response.get("method").is_none()
                        && response.get("id").and_then(Value::as_u64) == Some(4)
                    {
                        if let Some(error) = response.get("error") {
                            return Err(OmonError::Llm(format!("model update rejected: {error}")));
                        }
                        if response.get("result").is_none() {
                            return Err(OmonError::Llm(
                                "model update acknowledgement missing result".into(),
                            ));
                        }
                        break;
                    }
                }
            }
            if model_override.is_some() {
                session
                    .state
                    .metadata
                    .insert("omo_fallback_model_active".into(), json!(true));
            } else {
                session.state.metadata.remove("omo_fallback_model_active");
            }
        }
        // Own the submission before the first socket-write poll. The actor drops
        // this future before cancel(), so ambiguous ownership must outlive it.
        let submitted_at = tokio::time::Instant::now();
        let prior_turn_to_interrupt = {
            let mut active_turns = self.active_turns.lock();
            match active_turns.entry(session.key.storage_key()) {
                Entry::Vacant(entry) => {
                    entry.insert(ActiveTurn {
                        thread_id: thread_id.clone(),
                        turn_id: None,
                        inserted_at: submitted_at,
                        open_tools: 0,
                    });
                    None
                }
                Entry::Occupied(mut occupied) => {
                    let prior_turn_id = occupied.get().turn_id.clone();
                    let prior_thread_id = occupied.get().thread_id.clone();
                    let age = submitted_at.duration_since(occupied.get().inserted_at);

                    tracing::warn!(
                        age_secs = age.as_secs(),
                        "prior active-turn entry replaced with new turn request"
                    );
                    occupied.insert(ActiveTurn {
                        thread_id: thread_id.clone(),
                        turn_id: None,
                        inserted_at: submitted_at,
                        open_tools: 0,
                    });

                    prior_turn_id.map(|turn_id| (prior_thread_id, turn_id))
                }
            }
        };

        if let Some((prior_thread_id, prior_turn_id)) = prior_turn_to_interrupt {
            let interrupt_msg = json!({
                "jsonrpc": "2.0",
                "id": 9_005,
                "method": "turn/interrupt",
                "params": { "threadId": prior_thread_id, "turnId": prior_turn_id }
            });
            let _ = ws.send(Message::text(interrupt_msg.to_string())).await;
            tracing::info!(
                turn_id = %prior_turn_id,
                "interrupted prior active turn due to new user prompt in session"
            );
        }
        if let Err(e) = ws
            .send(turn_start_request(
                &thread_id,
                &user_prompt,
                model.as_deref(),
            ))
            .await
        {
            // The submission never reached the peer, so ownership ends here and
            // a retry may proceed without waiting out the stale-entry margin.
            self.active_turns.lock().remove(&session.key.storage_key());
            return Err(OmonError::Llm(format!("failed to send turn/start: {e}")));
        }

        let is_cron_session = session.key.user_id.starts_with("cron:")
            || session
                .state
                .metadata
                .get("cron_scheduler_delivery")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let is_lane = session.key.platform == "lane";

        // Immediately notify Discord that the agent is typing while preparing/reasoning
        if !is_cron_session && !is_lane {
            let _ = self
                .dispatcher
                .dispatch(OutboundAction::Typing {
                    session: session.key.clone(),
                    active: true,
                })
                .await;
        }

        let stream_id = Uuid::new_v4();
        let reply_to = if !event.platform_message_id.is_empty() {
            Some(event.platform_message_id.clone())
        } else {
            None
        };
        let mut sequence: u64 = 0;
        if is_lane {
            session.state.metadata.remove("lane_completed_item");
        }
        let mut full_content = String::new();
        let mut assistant_error = None;
        let mut total_tool_calls: usize = 0;
        let mut started_ids: std::collections::HashSet<String> = Default::default();
        // Tool items started but not yet completed. A tool runs silently (a subagent task, a long
        // command), so a frame gap while one is open is work in progress, not a wedged server.
        let mut open_tool_ids: std::collections::HashSet<String> = Default::default();
        // Code of the latest eval tool item: its bare approval frame names no command, so the
        // ledger classifies the shell commands in the cell instead.
        let mut last_eval_code: Option<String> = None;
        let mut approval_denials: u32 = 0;
        let mut messages_delivered: usize = 0;
        let mut last_relay_at: Option<tokio::time::Instant> = None;
        // The agent already produced a message: it is composing, so no more progress steers.

        let started_at = tokio::time::Instant::now();
        let mut turn_id: Option<String> = None;
        let mut turn_started_ack = false;
        let mut last_activity_at = tokio::time::Instant::now();
        // The response watchdogs count from when our prompt starts processing (the
        // turn/start acknowledgement), not from submission: the app-server queues
        // the prompt behind any run already active on the thread.
        let mut prompt_started_at = started_at;
        // First agent output for our prompt; disarms the initial-response interrupt.
        let mut first_progress_at: Option<tokio::time::Instant> = None;
        // turn/started and turn/completed frames seen before the acknowledgement, plus whether a
        // turn/start refusal is waiting for this thread to go idle (see `PreAckState`).
        let mut pre_ack = PreAckState::default();
        // Bounded wait for that idle boundary: a thread that never reports one fails the turn
        // instead of holding it until the acknowledgement deadline.
        let mut idle_wait_deadline: Option<tokio::time::Instant> = None;
        // Our acknowledged turn id was completed by the run our prompt was queued
        // behind; the next turn that starts on the thread is processing our prompt.
        let mut awaiting_successor_turn = false;
        let _no_content_grace = self.config.no_content_grace;
        let (approval_tx, mut approval_rx) = tokio::sync::mpsc::unbounded_channel::<(
            Value,
            String,
            tokio::time::Instant,
            crate::autonomy::AskResolution,
        )>();
        let mut auto_allowed_notices: Vec<String> = Vec::new();

        let ack_window = Duration::from_secs(30);
        let mut ack_deadline =
            started_at + ack_window.min(deadline.saturating_duration_since(started_at));

        let mut deadline = deadline;
        let cleanup_reserve = if effective_total_timeout >= Duration::from_secs(10) {
            Duration::from_secs(5)
        } else if effective_total_timeout >= Duration::from_secs(1) {
            effective_total_timeout / 5
        } else {
            Duration::ZERO
        };
        let mut work_deadline = deadline.checked_sub(cleanup_reserve).unwrap_or(deadline);

        // The turn's approval cards die with it: this scope's Drop cancels every spawned Ask task.
        let ask_scope = TurnAskScope::new();

        loop {
            let now = tokio::time::Instant::now();
            if let Some(wait_until) = idle_wait_deadline {
                if now >= wait_until {
                    // The refusal cannot be retried safely: the peer may still be executing the
                    // running turn, and resubmitting now is exactly the collision this waits for.
                    self.active_turns.lock().remove(&session.key.storage_key());
                    return Err(OmonError::Llm(
                        "turn/start refused: the thread is still processing another turn; not resubmitted"
                            .into(),
                    ));
                }
            }
            if !turn_started_ack && now >= ack_deadline {
                // The peer can still execute this queued submission. Keep ownership
                // so cancellation can reconcile it; never treat it as rejected.
                return Err(OmonError::Llm(
                    "timeout waiting for turn/start acknowledgement from omo app-server; submission outcome unknown, do not resubmit".into(),
                ));
            }

            let effective_work_deadline = if turn_id.is_some() {
                work_deadline
            } else {
                deadline
            };

            if now >= effective_work_deadline {
                if let Some(turn_id) = &turn_id {
                    let interrupt = json!({
                        "jsonrpc": "2.0",
                        "id": 9_001,
                        "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    });
                    let _ = ws.send(Message::text(interrupt.to_string())).await;
                    let rem = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if !rem.is_zero() {
                        let drain_fut = async {
                            while let Some(frame) = ws.next().await {
                                let text = match frame {
                                    Ok(Message::Text(text)) => text,
                                    // Ping/pong/binary frames must not abort the drain.
                                    Ok(_) => continue,
                                    Err(_) => break,
                                };
                                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                                    if val.get("id").and_then(Value::as_u64) == Some(9_001) {
                                        return Some(val);
                                    }
                                    let m = val.get("method").and_then(Value::as_str).unwrap_or("");
                                    if m == "item/agentMessage/delta" {
                                        if let Some(delta) =
                                            val.pointer("/params/delta").and_then(Value::as_str)
                                        {
                                            full_content.push_str(delta);
                                        }
                                    } else if m == "item/completed" {
                                        if let Some("agentMessage") =
                                            val.pointer("/params/item/type").and_then(Value::as_str)
                                        {
                                            if let Some(text) = val
                                                .pointer("/params/item/text")
                                                .and_then(Value::as_str)
                                            {
                                                full_content.clear();
                                                full_content.push_str(text);
                                                if is_lane {
                                                    if let Some(item) = val.pointer("/params/item")
                                                    {
                                                        if let Err(error) = self
                                                            .capture_lane_item(
                                                                session, item, turn_id,
                                                            )
                                                            .await
                                                        {
                                                            return Some(
                                                                json!({"lane_capture_error": error.to_string()}),
                                                            );
                                                        }
                                                        session.state.metadata.insert(
                                                            "lane_completed_item".into(),
                                                            json!(true),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    } else if m == "turn/completed" {
                                        return Some(val);
                                    }
                                }
                            }
                            None
                        };
                        if let Ok(Some(val)) = tokio::time::timeout(rem, drain_fut).await {
                            if let Some(error) =
                                val.get("lane_capture_error").and_then(Value::as_str)
                            {
                                self.active_turns.lock().remove(&session.key.storage_key());
                                return Err(OmonError::Llm(error.into()));
                            }
                            let status = val.pointer("/params/turn/status").and_then(Value::as_str);
                            let interrupt_confirmed = val.get("method").is_none()
                                && val.get("id").and_then(Value::as_u64) == Some(9_001)
                                && val.get("error").is_none()
                                && val.get("result").is_some();
                            let terminal_confirmed = val.get("method").and_then(Value::as_str)
                                == Some("turn/completed")
                                && val.pointer("/params/threadId").and_then(Value::as_str)
                                    == Some(thread_id.as_str())
                                && val
                                    .pointer("/params/turn/id")
                                    .or_else(|| val.pointer("/params/turnId"))
                                    .and_then(Value::as_str)
                                    == Some(turn_id.as_str())
                                && matches!(status, Some("interrupted" | "completed" | "failed"));
                            if interrupt_confirmed || terminal_confirmed {
                                self.active_turns.lock().remove(&session.key.storage_key());
                            }
                            if status == Some("completed")
                                && (!full_content.is_empty() || total_tool_calls > 0)
                            {
                                if full_content.contains("[output interrupted by stream rule]") {
                                    return Err(OmonError::Llm(
                                        "omo turn failed: output interrupted by stream rule".into(),
                                    ));
                                }
                                let mut scrubbed_content = filter_reasoning(&full_content);
                                if is_cron_session {
                                    scrubbed_content = filter_intent_gate(&scrubbed_content);
                                }
                                if is_explicit_silence(&scrubbed_content) {
                                    if let Some(ack_command) = session
                                        .state
                                        .metadata
                                        .get("cron_ack_command")
                                        .and_then(Value::as_str)
                                        .filter(|command| !command.trim().is_empty())
                                    {
                                        crate::cron::ack::run_ack_logged(ack_command).await;
                                    }
                                    return Ok(());
                                }

                                let rendered = if scrubbed_content.trim().is_empty() {
                                    "✅ Done.".to_string()
                                } else {
                                    scrubbed_content.clone()
                                };

                                self.deliver_rendered_result(
                                    session,
                                    stream_id,
                                    sequence,
                                    &rendered,
                                    reply_to.clone(),
                                )
                                .await?;

                                if let Some(ack_command) = session
                                    .state
                                    .metadata
                                    .get("cron_ack_command")
                                    .and_then(Value::as_str)
                                    .filter(|command| !command.trim().is_empty())
                                {
                                    crate::cron::ack::run_ack_logged(ack_command).await;
                                }
                                return Ok(());
                            }
                        }
                    }
                }
                self.active_turns.lock().remove(&session.key.storage_key());
                self.clear_stuck_thread(session).await;
                tokio::time::sleep(Duration::from_millis(50)).await;
                return Err(OmonError::Llm(format!(
                    "turn exceeded total deadline of {:?}; turn/interrupt sent",
                    effective_total_timeout
                )));
            }

            // Fail fast when the agent shows no sign of working on the prompt: nothing
            // delivered and no agent output within initial_response_timeout (45s) of
            // the prompt starting. Once output flows, the progress steer below asks for
            // briefs instead, and the inactivity and total deadlines remain the backstop
            // for a turn that stalls later. The thread binding is kept: a silent model
            // is not a wedged thread, and dropping the binding loses the conversation.
            // Lane turns run unattended under their own budgets and are exempt.
            let initial_response_timeout = self.config.initial_response_timeout;
            let awaiting_first_response = !is_cron_session
                && !is_lane
                && turn_started_ack
                && messages_delivered == 0
                && first_progress_at.is_none();
            if awaiting_first_response
                && now.duration_since(prompt_started_at) >= initial_response_timeout
            {
                if let Some(turn_id) = &turn_id {
                    let interrupt = json!({
                        "jsonrpc": "2.0",
                        "id": 9_002,
                        "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    });
                    let _ = ws.send(Message::text(interrupt.to_string())).await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                self.active_turns.lock().remove(&session.key.storage_key());
                // Nothing reached the user and no tool ran, so the prompt can be re-run on the
                // fallback model. The common cause is the primary model's quota being exhausted:
                // the runtime then retries in silence and never forwards an event.
                *fallback_eligible = total_tool_calls == 0;
                return Err(OmonError::Llm(format!(
                    "turn interrupted: agent produced no output within {}s of starting the prompt",
                    initial_response_timeout.as_secs()
                )));
            }

            // Inactivity timeout: if the server stops sending ANY streaming frames for more than 180s
            // (or request_timeout, whichever is shorter for interactive turns), interrupt the turn immediately.
            let inactivity_timeout = if !is_cron_session && !is_lane {
                self.config.request_timeout.min(Duration::from_secs(180))
            } else {
                self.config.request_timeout
            };
            if open_tool_ids.is_empty() && now >= last_activity_at + inactivity_timeout {
                if let Some(turn_id) = &turn_id {
                    let interrupt = json!({
                        "jsonrpc": "2.0",
                        "id": 9_003,
                        "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    });
                    let _ = ws.send(Message::text(interrupt.to_string())).await;
                }
                self.active_turns.lock().remove(&session.key.storage_key());
                // After a tool ran, re-running the prompt would repeat its side effects, so only a
                // turn that did no tool work is reported as a stall the caller may restart and retry.
                return Err(OmonError::Llm(if total_tool_calls == 0 {
                    "timeout during turn streaming: no activity from server".into()
                } else {
                    "timeout during turn streaming: server went silent after tool calls".into()
                }));
            }

            let mut next_timeout = effective_work_deadline;
            if !turn_started_ack {
                next_timeout = next_timeout.min(ack_deadline);
            }
            if open_tool_ids.is_empty() {
                next_timeout = next_timeout.min(last_activity_at + inactivity_timeout);
            }
            if awaiting_first_response {
                next_timeout = next_timeout.min(prompt_started_at + initial_response_timeout);
            }

            let msg = tokio::select! {
                biased;
                Some((req_id, tool_id, ask_started, resolution)) = approval_rx.recv() => {
                    open_tool_ids.remove(&tool_id);
                    if let Some(active) = self.active_turns.lock().get_mut(&session.key.storage_key()) {
                        active.open_tools = open_tool_ids.len();
                    }
                    let waited = ask_started.elapsed();
                    deadline += waited;
                    work_deadline += waited;
                    prompt_started_at += waited;
                    last_activity_at = tokio::time::Instant::now();
                    match resolution {
                        crate::autonomy::AskResolution::Allow => {
                            let _ = ws.send(approval_allow_response(&req_id)).await;
                        }
                        crate::autonomy::AskResolution::Deny { reason, count_toward_limit } => {
                            if count_toward_limit {
                                approval_denials += 1;
                                if approval_denials >= APPROVAL_DENIAL_TURN_LIMIT {
                                    if let Some(turn_id) = &turn_id {
                                        let interrupt = json!({
                                            "jsonrpc": "2.0",
                                            "id": 9_002,
                                            "method": "turn/interrupt",
                                            "params": { "threadId": thread_id, "turnId": turn_id }
                                        });
                                        let _ = ws.send(Message::text(interrupt.to_string())).await;
                                    }
                                    self.active_turns.lock().remove(&session.key.storage_key());
                                    return Err(OmonError::Llm(format!(
                                        "tool approval denied {approval_denials} times by gateway policy; turn aborted"
                                    )));
                                }
                            }
                            let _ = ws.send(approval_denial_with_reason(&req_id, &reason)).await;
                        }
                    }
                    continue;
                }
                timed = tokio::time::timeout_at(next_timeout, ws.next()) => {
                    match timed {
                        Ok(Some(msg)) => msg,
                        Ok(None) => break,
                        Err(_) => continue,
                    }
                }
            };

            let msg = msg.map_err(|e| OmonError::Llm(format!("ws streaming error: {e}")))?;
            let val = match &msg {
                Message::Text(text) => {
                    let parsed = serde_json::from_str::<Value>(text.as_str()).map_err(|e| {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        OmonError::Llm(format!("invalid json: {e}"))
                    })?;
                    // Invariant: only frames showing real work for this thread/turn (or the
                    // turn/start acknowledgement itself) renew the inactivity watchdog and the
                    // interactive 30m deadline. Steer/interrupt acks and unrelated frames must
                    // not, or the gateway's own progress steers would keep a hung turn alive.
                    let frame_method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
                    let is_start_ack = !turn_started_ack
                        && parsed.get("method").is_none()
                        && parsed.get("id").and_then(Value::as_u64) == Some(3);
                    if is_start_ack
                        || frame_is_turn_progress(
                            frame_method,
                            &parsed,
                            &thread_id,
                            turn_id.as_deref(),
                            awaiting_successor_turn,
                        )
                    {
                        last_activity_at = tokio::time::Instant::now();
                        if !is_cron_session && !is_lane {
                            deadline = last_activity_at + effective_total_timeout;
                            work_deadline =
                                deadline.checked_sub(cleanup_reserve).unwrap_or(deadline);
                        }
                    }
                    Some(parsed)
                }
                _ => None,
            };
            // Deadline checks run on EVERY frame — ping/pong included — so a
            // keepalive stream cannot mask a wedged turn past the cap.
            let method = val
                .as_ref()
                .and_then(|v| v.get("method"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if method != "turn/completed" && tokio::time::Instant::now() >= effective_work_deadline
            {
                if let Some(turn_id) = &turn_id {
                    let interrupt = json!({
                        "jsonrpc": "2.0",
                        "id": 9_001,
                        "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    });
                    let _ = ws.send(Message::text(interrupt.to_string())).await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                self.active_turns.lock().remove(&session.key.storage_key());
                self.clear_stuck_thread(session).await;
                return Err(OmonError::Llm(format!(
                    "turn exceeded total deadline of {:?}; turn/interrupt sent",
                    effective_total_timeout
                )));
            }

            let Some(val) = val else {
                continue;
            };

            if val.get("method").is_none()
                && val.get("id").and_then(Value::as_u64) == Some(3)
                && !turn_started_ack
            {
                if let Some(error) = val.get("error") {
                    let err_str = error.to_string();
                    // A retry (model fallback, restart recovery) submitted while the engine is
                    // still processing another turn is refused outright. Wait for that turn's
                    // terminal frame on this already-subscribed connection, then submit once more,
                    // so the retry never collides with a running engine turn. Ownership is kept
                    // meanwhile, so cancel() can still reconcile the submission.
                    if pre_ack.note_refusal(&err_str) {
                        tracing::warn!(
                            session = %session.key,
                            %thread_id,
                            "turn/start refused: the thread is still processing; waiting for it to go idle, then submitting once more"
                        );
                        if pre_ack.idle_already() {
                            // The running turn finished while the refusal was in flight, so the
                            // idle boundary will not be replayed: submit again right away.
                            if let Err(e) = ws
                                .send(turn_start_request(
                                    &thread_id,
                                    &user_prompt,
                                    model.as_deref(),
                                ))
                                .await
                            {
                                self.active_turns.lock().remove(&session.key.storage_key());
                                return Err(OmonError::Llm(format!(
                                    "failed to send turn/start: {e}"
                                )));
                            }
                            last_activity_at = tokio::time::Instant::now();
                            // The replaced submission's turn boundaries no longer describe our prompt.
                            pre_ack.events.clear();
                            continue;
                        }
                        idle_wait_deadline = Some(
                            last_activity_at
                                + ack_window
                                    .min(deadline.saturating_duration_since(last_activity_at)),
                        );
                        continue;
                    }
                    self.active_turns.lock().remove(&session.key.storage_key());
                    if is_model_rejected_error(&err_str) {
                        *fallback_eligible = true;
                    }
                    return Err(OmonError::Llm(format!("turn/start error: {error}")));
                }
                let id = val
                    .pointer("/result/turn/id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        OmonError::Llm("turn/start acknowledgement missing turn id".into())
                    })?;
                let acked_turn = id.to_string();
                // A turn/completed for our id before the acknowledgement means the run
                // our prompt was queued behind finished under it. A turn that started
                // after that is the one processing our prompt.
                let taken_over_at = pre_ack
                    .events
                    .iter()
                    .position(|(is_start, tid)| !is_start && *tid == acked_turn);
                let successor = taken_over_at.and_then(|at| {
                    pre_ack.events[at + 1..]
                        .iter()
                        .rev()
                        .find(|(is_start, tid)| *is_start && *tid != acked_turn)
                        .map(|(_, tid)| tid.clone())
                });
                pre_ack.events.clear();
                if taken_over_at.is_some() {
                    tracing::info!(
                        %thread_id,
                        acked_turn = %acked_turn,
                        successor = ?successor,
                        "queued prompt's turn id was completed by the run ahead of it; following its successor"
                    );
                }
                awaiting_successor_turn = taken_over_at.is_some() && successor.is_none();
                let turn_id_str = successor.unwrap_or(acked_turn);
                {
                    let mut active_turns = self.active_turns.lock();
                    let inserted_at = active_turns
                        .get(&session.key.storage_key())
                        .map(|existing| existing.inserted_at)
                        .unwrap_or_else(tokio::time::Instant::now);
                    active_turns.insert(
                        session.key.storage_key(),
                        ActiveTurn {
                            thread_id: thread_id.clone(),
                            turn_id: Some(turn_id_str.clone()),
                            inserted_at,
                            open_tools: open_tool_ids.len(),
                        },
                    );
                }
                *last_turn = Some(turn_id_str.clone());
                turn_id = Some(turn_id_str);
                turn_started_ack = true;
                prompt_started_at = tokio::time::Instant::now();
                // The prompt carrying the cron briefings was accepted: only now are they consumed.
                if let (Some(briefings), Some(pool)) = (briefings, &self.pool) {
                    if let Err(error) =
                        crate::mirror::mark_cron_briefings_injected(pool, &briefings.message_ids)
                            .await
                    {
                        tracing::warn!(session = %session.key, %error, "failed to mark cron briefings injected");
                    }
                }
                continue;
            }

            if let (Some(req_id), Some(method)) =
                (val.get("id"), val.get("method").and_then(Value::as_str))
            {
                // Messenger clients cannot answer the engine's interactive form.
                // Explicit cancellation lets the agent ask in ordinary chat rather
                // than blocking the thread queue for the form's 30-minute timeout.
                if let Some(reply) = user_input_cancellation(&val) {
                    ws.send(reply).await.map_err(|error| {
                        OmonError::Llm(format!("user input cancellation failed: {error}"))
                    })?;
                    continue;
                }
                if is_approval_request(method) {
                    // A toolset restriction denies even under YOLO, and counts toward the
                    // denial-loop guard like any other policy denial.
                    let mut toolset_blocked = false;
                    if let Some(ref toolsets) = session.state.enabled_toolsets {
                        let is_command = method.contains("commandExecution")
                            || val.pointer("/params/command").is_some()
                            || val.pointer("/params/item/command").is_some();
                        let allows_terminal = toolsets
                            .iter()
                            .any(|t| t == "terminal" || t == "bash" || t == "shell");
                        if is_command && !allows_terminal {
                            tracing::warn!(
                                session = %session.key,
                                "tool declined: commandExecution is not in enabled_toolsets {:?}",
                                toolsets
                            );
                            toolset_blocked = true;
                        }
                    }

                    let unrestricted = session.state.yolo || self.is_full_access();
                    if self.is_ledger_enabled() && !toolset_blocked {
                        let bare_eval = crate::autonomy::is_bare_eval_frame(&val);
                        let eval_classification = if bare_eval {
                            last_eval_code.as_deref().and_then(|code| {
                                crate::autonomy::classify_eval_code(
                                    code,
                                    session
                                        .state
                                        .metadata
                                        .get("workspace_cwd")
                                        .and_then(Value::as_str),
                                )
                            })
                        } else {
                            None
                        };
                        // Only a cell whose code was seen and has no side effect runs unasked; a
                        // bare eval frame without its code stays unknown and goes to the ledger.
                        if !unrestricted
                            && bare_eval
                            && last_eval_code.is_some()
                            && eval_classification.is_none()
                        {
                            let _ = ws.send(approval_allow_response(req_id)).await;
                            continue;
                        }
                        let always_mode = matches!(
                            std::env::var("APPROVAL_MODE")
                                .or_else(|_| std::env::var("APPROVAL_POLICY"))
                                .as_deref(),
                            Ok("always")
                        );
                        let ws_cwd = session
                            .state
                            .metadata
                            .get("workspace_cwd")
                            .and_then(Value::as_str);
                        let classification = eval_classification
                            .unwrap_or_else(|| crate::autonomy::classify_frame(&val, ws_cwd));
                        if let Some(pool) = &self.pool {
                            let lead_key = if is_lane {
                                sqlx::query_scalar::<_, String>(
                                    "SELECT lead_session_key FROM lanes WHERE key = ?",
                                )
                                .bind(&session.key.channel_id)
                                .fetch_optional(pool)
                                .await
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| crate::lanes::kibitzer::DEFAULT_LEAD.to_string())
                            } else {
                                session.key.storage_key()
                            };
                            let payload_str = val.to_string();
                            let pre = crate::autonomy::evaluate_pre_ask(
                                pool,
                                crate::autonomy::PreAskInput {
                                    origin: crate::autonomy::ProposalOrigin::EngineApproval,
                                    session_key: &session.key.storage_key(),
                                    lead_session_key: &lead_key,
                                    classification: &classification,
                                    payload_json: &payload_str,
                                    tainted: turn_tainted,
                                    yolo: unrestricted,
                                    always_mode,
                                    now: self.now_utc(),
                                },
                            )
                            .await;
                            match pre {
                                Ok(crate::autonomy::PreAskOutcome::Allow { notice, .. }) => {
                                    if let Some(line) = notice {
                                        crate::report::status::record_tool_activity(
                                            &session.key,
                                            &line,
                                        );
                                        auto_allowed_notices.push(line);
                                    }
                                    let _ = ws.send(approval_allow_response(req_id)).await;
                                    continue;
                                }
                                Ok(crate::autonomy::PreAskOutcome::Deny {
                                    reason,
                                    count_toward_limit,
                                    ..
                                }) => {
                                    if count_toward_limit {
                                        approval_denials += 1;
                                        if approval_denials >= APPROVAL_DENIAL_TURN_LIMIT {
                                            if let Some(turn_id) = &turn_id {
                                                let interrupt = json!({
                                                    "jsonrpc": "2.0",
                                                    "id": 9_002,
                                                    "method": "turn/interrupt",
                                                    "params": { "threadId": thread_id, "turnId": turn_id }
                                                });
                                                let _ = ws
                                                    .send(Message::text(interrupt.to_string()))
                                                    .await;
                                            }
                                            self.active_turns
                                                .lock()
                                                .remove(&session.key.storage_key());
                                            return Err(OmonError::Llm(format!(
                                                "tool approval denied {approval_denials} times by gateway policy; turn aborted"
                                            )));
                                        }
                                    }
                                    let _ =
                                        ws.send(approval_denial_with_reason(req_id, &reason)).await;
                                    continue;
                                }
                                Ok(crate::autonomy::PreAskOutcome::Ask { proposal_id }) => {
                                    if let Some(requester) = self.approval_requester.clone() {
                                        let tool_id = format!("approval:{proposal_id}");
                                        open_tool_ids.insert(tool_id.clone());
                                        if let Some(active) = self
                                            .active_turns
                                            .lock()
                                            .get_mut(&session.key.storage_key())
                                        {
                                            active.open_tools = open_tool_ids.len();
                                        }
                                        let ask_started = tokio::time::Instant::now();
                                        let req_id_owned = req_id.clone();
                                        let pool_clone = pool.clone();
                                        let ask_session = if is_lane {
                                            crate::SessionKey::from_storage_key(&lead_key)
                                                .unwrap_or_else(|_| session.key.clone())
                                        } else {
                                            session.key.clone()
                                        };
                                        let engine_reason = val
                                            .pointer("/params/reason")
                                            .and_then(Value::as_str)
                                            .unwrap_or(&classification.summary);
                                        let card_reason = format!(
                                            "{engine_reason}\neffect: {} | target: {}",
                                            classification.effect.as_str(),
                                            classification.target
                                        );
                                        let cmd_display = classification.summary.clone();
                                        let tx = approval_tx.clone();
                                        let ask_dispatcher = self.dispatcher.clone();
                                        let turn_end = ask_scope.token();
                                        tokio::spawn(async move {
                                            // The turn's scope owns this card. When the turn ends on
                                            // any path (owner cancel, a new message preempting it, the
                                            // work deadline) the token fires, the pending prompt is
                                            // dropped — which removes it from the guard and expires the
                                            // owner's card — and the proposal is expired here instead
                                            // of staying approvable for an action nothing will run.
                                            let asked = tokio::select! {
                                                biased;
                                                res = requester.request_for_proposal(
                                                    &ask_session,
                                                    proposal_id,
                                                    &cmd_display,
                                                    &card_reason,
                                                ) => Some(res.map(|(_, decision)| decision)),
                                                _ = turn_end.cancelled() => None,
                                            };
                                            let Some(res) = asked else {
                                                tracing::info!(
                                                    proposal_id,
                                                    "the turn ended before the owner answered; expired the approval card"
                                                );
                                                let _ = expire_abandoned_proposal(
                                                    &pool_clone,
                                                    proposal_id,
                                                )
                                                .await;
                                                let notice = turn_ended_ask_notice(proposal_id);
                                                let _ = ask_dispatcher
                                                    .dispatch(OutboundAction::SendMessage {
                                                        session: ask_session.clone(),
                                                        content: notice,
                                                        reply_to: None,
                                                    })
                                                    .await;
                                                return;
                                            };
                                            // The answer arrived, but the turn that asked may still have
                                            // ended first: the same rule applies.
                                            if tx.is_closed() {
                                                let _ = expire_abandoned_proposal(
                                                    &pool_clone,
                                                    proposal_id,
                                                )
                                                .await;
                                                let notice = late_ask_notice(proposal_id);
                                                let _ = ask_dispatcher
                                                    .dispatch(OutboundAction::SendMessage {
                                                        session: ask_session.clone(),
                                                        content: notice,
                                                        reply_to: None,
                                                    })
                                                    .await;
                                                return;
                                            }
                                            let resolution = crate::autonomy::finalize_ask_result(
                                                &pool_clone,
                                                proposal_id,
                                                &classification,
                                                "engine_approval",
                                                &lead_key,
                                                res,
                                            )
                                            .await
                                            .unwrap_or(crate::autonomy::AskResolution::Deny {
                                                reason: format!(
                                                    "owner has not answered yet; continue with other work and mention pending proposal #{proposal_id} in your report"
                                                ),
                                                count_toward_limit: false,
                                            });
                                            let _ = tx.send((
                                                req_id_owned,
                                                tool_id,
                                                ask_started,
                                                resolution,
                                            ));
                                        });
                                        continue;
                                    } else {
                                        let _ = crate::autonomy::update_proposal_decision(
                                            pool,
                                            proposal_id,
                                            crate::autonomy::ProposalState::Expired,
                                            None,
                                            None,
                                        )
                                        .await;
                                        let reason = format!(
                                            "owner has not answered yet; continue with other work and mention pending proposal #{proposal_id} in your report"
                                        );
                                        let _ = ws
                                            .send(approval_denial_with_reason(req_id, &reason))
                                            .await;
                                        continue;
                                    }
                                }
                                Err(error) => {
                                    tracing::warn!(session = %session.key, %error, "action ledger pre-ask failed; declining");
                                    let _ = ws
                                        .send(approval_denial_with_reason(
                                            req_id,
                                            "the gateway could not record this action; retry it shortly",
                                        ))
                                        .await;
                                    continue;
                                }
                            }
                        } else {
                            // The ledger is on but this backend has no database: never fall back
                            // to the legacy auto-approve.
                            tracing::warn!(session = %session.key, "action ledger enabled without a database; declining");
                            let _ = ws
                                .send(approval_denial_with_reason(
                                    req_id,
                                    "the gateway cannot record actions for this session",
                                ))
                                .await;
                            continue;
                        }
                    }

                    if !toolset_blocked && unrestricted {
                        tracing::info!(
                            session = %session.key,
                            "approving daemon tool request under active YOLO mode"
                        );
                        let _ = ws.send(approval_allow_response(req_id)).await;
                        continue;
                    }

                    // Unless APPROVAL_MODE is explicitly set to "always" (strict interactive gating),
                    // allow daemon tool requests for authenticated single-user sessions
                    // rather than silently discarding them with gateway-policy denial.
                    let auto_approve = !matches!(
                        std::env::var("APPROVAL_MODE")
                            .or_else(|_| std::env::var("APPROVAL_POLICY"))
                            .as_deref(),
                        Ok("always")
                    );

                    if !toolset_blocked && auto_approve {
                        tracing::info!(
                            session = %session.key,
                            "approving daemon tool request under gateway approval policy"
                        );
                        let _ = ws.send(approval_allow_response(req_id)).await;
                        continue;
                    }
                    approval_denials += 1;
                    if approval_denials >= APPROVAL_DENIAL_TURN_LIMIT {
                        tracing::error!(
                                denials = approval_denials,
                                "approval denial loop: aborting turn instead of flailing until the deadline"
                            );
                        if let Some(turn_id) = &turn_id {
                            let interrupt = json!({
                                "jsonrpc": "2.0",
                                "id": 9_002,
                                "method": "turn/interrupt",
                                "params": { "threadId": thread_id, "turnId": turn_id }
                            });
                            let _ = ws.send(Message::text(interrupt.to_string())).await;
                        }
                        self.active_turns.lock().remove(&session.key.storage_key());
                        return Err(OmonError::Llm(format!(
                                "tool approval denied {approval_denials} times by gateway policy; turn aborted"
                            )));
                    }
                    let _ = ws.send(approval_denial_response(req_id)).await;
                    continue;
                }
            }

            // Only the successful start response owns the active turn identity.
            // Subscription replay (including thread idle without a turn ID) is
            // not evidence that this turn has produced output or completed.
            let frame_turn_id = val
                .pointer("/params/turnId")
                .or_else(|| val.pointer("/params/turn/id"))
                .and_then(Value::as_str);
            let turn_bearing = method.starts_with("turn/")
                || method.starts_with("item/")
                || method == "thread/status/changed"
                || (method == "error"
                    && (val.pointer("/params/turnId").is_some()
                        || val.pointer("/params/turn").is_some()));
            if turn_bearing {
                if val.pointer("/params/threadId").and_then(Value::as_str)
                    != Some(thread_id.as_str())
                {
                    continue;
                }
                if !turn_started_ack {
                    // The app-server acknowledges turn/start only once our prompt is
                    // dequeued. Until then the thread is busy with a run that is not
                    // processing our prompt (typically one a background job completion
                    // started), and that run may finish under our turn id. Keep waiting
                    // while it works, record its turn boundaries, and relay its
                    // finished messages instead of dropping them.
                    ack_deadline = ack_deadline.max((last_activity_at + ack_window).min(deadline));
                    if pre_ack.observe(&val, &thread_id) == PreAckAction::Resubmit {
                        idle_wait_deadline = None;
                        if let Err(e) = ws
                            .send(turn_start_request(
                                &thread_id,
                                &user_prompt,
                                model.as_deref(),
                            ))
                            .await
                        {
                            self.active_turns.lock().remove(&session.key.storage_key());
                            return Err(OmonError::Llm(format!("failed to send turn/start: {e}")));
                        }
                        last_activity_at = tokio::time::Instant::now();
                        ack_deadline = last_activity_at
                            + ack_window.min(deadline.saturating_duration_since(last_activity_at));
                        // The replaced submission's turn boundaries no longer describe our prompt.
                        pre_ack.events.clear();
                        tracing::info!(
                            session = %session.key,
                            %thread_id,
                            "thread went idle; resubmitted the prompt that turn/start refused"
                        );
                        continue;
                    }
                    if !is_cron_session && !is_lane {
                        if let Some(text) = completed_agent_message_text(&method, &val) {
                            let scrubbed = filter_reasoning(text);
                            if !scrubbed.trim().is_empty() && !is_explicit_silence(&scrubbed) {
                                if let Err(error) = self
                                    .deliver_rendered_result(
                                        session, stream_id, sequence, &scrubbed, None,
                                    )
                                    .await
                                {
                                    self.active_turns.lock().remove(&session.key.storage_key());
                                    return Err(error);
                                }
                                *side_effects = true;
                                sequence = sequence.saturating_add(1);
                            }
                        }
                    }
                    continue;
                }
                if let Some(frame_turn) = frame_turn_id {
                    if Some(frame_turn) != turn_id.as_deref() {
                        let successor_started = awaiting_successor_turn
                            && first_progress_at.is_none()
                            && method == "turn/started";
                        if !successor_started {
                            continue;
                        }
                        let successor = frame_turn.to_string();
                        tracing::info!(
                            %thread_id,
                            successor = %successor,
                            "following the turn that took over our queued prompt"
                        );
                        if let Some(active) =
                            self.active_turns.lock().get_mut(&session.key.storage_key())
                        {
                            active.turn_id = Some(successor.clone());
                        }
                        *last_turn = Some(successor.clone());
                        turn_id = Some(successor);
                        awaiting_successor_turn = false;
                    }
                }
                if first_progress_at.is_none() && is_agent_progress(&method, &val) {
                    first_progress_at = Some(tokio::time::Instant::now());
                }
            }

            match method.as_str() {
                "turn/started" => {
                    approval_denials = 0;
                }
                "item/started" | "item/completed" => {
                    approval_denials = 0;
                    let is_started =
                        val.get("method").and_then(Value::as_str) == Some("item/started");
                    let Some(item) = val.pointer("/params/item") else {
                        continue;
                    };
                    let item_id = item.get("id").and_then(Value::as_str).unwrap_or("");
                    if !is_started {
                        open_tool_ids.remove(item_id);
                        if let Some(active) =
                            self.active_turns.lock().get_mut(&session.key.storage_key())
                        {
                            active.open_tools = open_tool_ids.len();
                        }
                    }

                    if is_started && !item_id.is_empty() && started_ids.insert(item_id.to_string())
                    {
                        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
                        if item.get("tool").and_then(Value::as_str) == Some("eval") {
                            last_eval_code = item
                                .pointer("/arguments/code")
                                .and_then(Value::as_str)
                                .map(str::to_string);
                        }
                        if !matches!(item_type, "agentMessage" | "userMessage" | "reasoning" | "") {
                            // A started tool may already have changed the world: never resubmit.
                            *side_effects = true;
                            open_tool_ids.insert(item_id.to_string());
                            if let Some(active) =
                                self.active_turns.lock().get_mut(&session.key.storage_key())
                            {
                                active.open_tools = open_tool_ids.len();
                            }
                            crate::report::status::record_tool_activity(&session.key, item_type);
                            // If the agent emitted an initial status/plan message before invoking tools,
                            // immediately deliver it to Discord so the user is never left waiting in silence.
                            // The first status goes out as the acknowledgement. After that a
                            // narration is relayed only once enough time has passed, so a quick
                            // task is "ack, then answer" and a long one gets an occasional update
                            // instead of a message before every tool call.
                            if !is_cron_session
                                && !is_lane
                                && !full_content.trim().is_empty()
                                && !should_relay_interim(
                                    messages_delivered,
                                    last_relay_at,
                                    tokio::time::Instant::now(),
                                )
                            {
                                full_content.clear();
                            }
                            if !is_cron_session && !is_lane && !full_content.trim().is_empty() {
                                let scrubbed = filter_reasoning(&full_content);
                                if !scrubbed.trim().is_empty() && !is_explicit_silence(&scrubbed) {
                                    // Visible as a normal message; counted and cleared only when it
                                    // was actually dispatched, so a failed relay is not lost and
                                    // does not disarm the progress steer.
                                    if self.relay_status(session, scrubbed, reply_to.clone()).await
                                    {
                                        *side_effects = true;
                                        messages_delivered += 1;
                                        last_relay_at = Some(tokio::time::Instant::now());
                                        full_content.clear();
                                    }
                                } else {
                                    full_content.clear();
                                }
                            }

                            total_tool_calls += 1;
                        }
                    }

                    if !is_started {
                        if let Some("agentMessage") = item.get("type").and_then(Value::as_str) {
                            if item.get("stopReason").and_then(Value::as_str) == Some("error") {
                                assistant_error = Some(
                                    item.get("errorMessage")
                                        .and_then(Value::as_str)
                                        .filter(|message| !message.trim().is_empty())
                                        .unwrap_or("assistant message failed")
                                        .to_owned(),
                                );
                                continue;
                            }
                            if let Some(text) = item.get("text").and_then(Value::as_str) {
                                full_content.clear();
                                full_content.push_str(text);

                                if is_lane {
                                    if let Err(error) = self
                                        .capture_lane_item(
                                            session,
                                            item,
                                            turn_id.as_deref().unwrap_or(&thread_id),
                                        )
                                        .await
                                    {
                                        self.active_turns.lock().remove(&session.key.storage_key());
                                        return Err(error);
                                    }
                                    session
                                        .state
                                        .metadata
                                        .insert("lane_completed_item".into(), json!(true));
                                }
                                // A completed message is relayed at the next tool start (a status
                                // that more work follows); relayed here it would duplicate the final
                                // reply whenever it turns out to be the last message of the turn.
                            }
                        }
                    }
                }
                "item/agentMessage/delta" => {
                    if let Some(delta) = val.pointer("/params/delta").and_then(Value::as_str) {
                        if !delta.is_empty() {
                            full_content.push_str(delta);
                            let suppress_emission = session
                                .state
                                .metadata
                                .get("cron_suppress_direct_emission")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            if !suppress_emission {
                                let _ = self
                                    .emit_chunk(
                                        session,
                                        stream_id,
                                        sequence,
                                        full_content.clone(),
                                        false,
                                        reply_to.clone(),
                                    )
                                    .await;
                                sequence = sequence.saturating_add(1);
                            }
                        }
                    }
                }
                "turn/completed" => {
                    if let Some(error) = &assistant_error {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        return Err(OmonError::Llm(format!("omo turn failed: {error}")));
                    }
                    match val.pointer("/params/turn/status").and_then(Value::as_str) {
                        Some("interrupted") => {
                            self.active_turns.lock().remove(&session.key.storage_key());
                            return Err(OmonError::Llm("omo turn interrupted".into()));
                        }
                        Some("failed") => {
                            self.active_turns.lock().remove(&session.key.storage_key());
                            let err_msg = val
                                .pointer("/params/turn/error")
                                .and_then(Value::as_str)
                                .or_else(|| {
                                    val.pointer("/params/turn/error/message")
                                        .and_then(Value::as_str)
                                })
                                .unwrap_or("turn failed");
                            let is_quota_error = [
                                "429",
                                "500",
                                "502",
                                "503",
                                "504",
                                "rate limit",
                                "overloaded",
                            ]
                            .iter()
                            .any(|marker| err_msg.to_lowercase().contains(marker));
                            let is_rejected = is_model_rejected_error(err_msg);
                            *fallback_eligible = total_tool_calls == 0
                                && full_content.is_empty()
                                && (is_quota_error || is_rejected);
                            return Err(OmonError::Llm(format!("omo turn failed: {err_msg}")));
                        }
                        Some("completed") => {}
                        _ => continue,
                    }

                    if full_content.contains("[output interrupted by stream rule]") {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        return Err(OmonError::Llm(
                            "omo turn failed: output interrupted by stream rule".into(),
                        ));
                    }

                    let mut scrubbed_content = filter_reasoning(&full_content);
                    if is_cron_session {
                        scrubbed_content = filter_intent_gate(&scrubbed_content);
                    }
                    if is_explicit_silence(&scrubbed_content) {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        if let Some(ack_command) = session
                            .state
                            .metadata
                            .get("cron_ack_command")
                            .and_then(Value::as_str)
                            .filter(|command| !command.trim().is_empty())
                        {
                            crate::cron::ack::run_ack_logged(ack_command).await;
                        }
                        return Ok(());
                    }

                    // Messages relayed mid-turn count: the relay clears the buffer once
                    // it has delivered the text.
                    let has_content = !scrubbed_content.is_empty()
                        || total_tool_calls > 0
                        || messages_delivered > 0;
                    if !has_content {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        *fallback_eligible = full_content.is_empty();
                        return Err(OmonError::Llm(
                            "omo turn ended with no content (likely upstream LLM failure)".into(),
                        ));
                    }
                    self.active_turns.lock().remove(&session.key.storage_key());

                    // A closing placeholder only helps when the user has seen nothing.
                    if !scrubbed_content.trim().is_empty() || messages_delivered == 0 {
                        let mut rendered = if scrubbed_content.trim().is_empty() {
                            "✅ Done.".to_string()
                        } else {
                            scrubbed_content.clone()
                        };
                        if !is_cron_session && !auto_allowed_notices.is_empty() {
                            rendered.push_str("\n\n");
                            rendered.push_str(&auto_allowed_notices.join("\n"));
                        }

                        self.deliver_rendered_result(
                            session,
                            stream_id,
                            sequence,
                            &rendered,
                            reply_to.clone(),
                        )
                        .await?;
                    }

                    if let Some(ack_command) = session
                        .state
                        .metadata
                        .get("cron_ack_command")
                        .and_then(Value::as_str)
                        .filter(|command| !command.trim().is_empty())
                    {
                        crate::cron::ack::run_ack_logged(ack_command).await;
                    }
                    return Ok(());
                }
                "error" | "turn/error" => {
                    let is_turn_error = val.pointer("/params/turnId").is_some()
                        || val.pointer("/params/turn").is_some()
                        || method == "turn/error";
                    if !is_turn_error {
                        tracing::warn!(
                            %thread_id,
                            ?val,
                            "non-fatal notification error from omo app-server; continuing turn"
                        );
                        continue;
                    }
                    let err_msg = val
                        .pointer("/params/message")
                        .or_else(|| val.pointer("/params/error"))
                        .or_else(|| val.pointer("/params/error/message"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error from omo app-server");
                    self.active_turns.lock().remove(&session.key.storage_key());
                    return Err(OmonError::Llm(format!(
                        "omo app-server turn error: {err_msg}"
                    )));
                }
                _ => {}
            }
        }
        // Reaching the end of the stream without a terminal frame means the
        // daemon connection closed mid-turn. Preserve ownership: disconnect
        // proves neither rejection nor completion, and must not enable resubmission.
        Err(OmonError::Llm(
            "connection closed before turn completion".into(),
        ))
    }
}

/// Minimum spacing between two interim status messages after the acknowledgement.
const INTERIM_REPORT_INTERVAL: Duration = Duration::from_secs(180);

/// Whether an interim narration should reach the user: always the first one (the
/// acknowledgement), afterwards only when the previous one is old enough.
fn should_relay_interim(
    messages_delivered: usize,
    last_relay_at: Option<tokio::time::Instant>,
    now: tokio::time::Instant,
) -> bool {
    messages_delivered == 0
        || last_relay_at.is_none_or(|at| now.duration_since(at) >= INTERIM_REPORT_INTERVAL)
}

/// Errors that mean the app-server stopped sending this turn's events at all, as
/// opposed to the model failing or the turn being refused.
fn is_backend_stall(error: &OmonError) -> bool {
    let message = error.to_string();
    message.contains("agent produced no output within")
        || message.contains("no activity from server")
}

/// Errors that mean no connection to the app-server could be established at all.
fn is_connect_failure(error: &OmonError) -> bool {
    let message = error.to_string();
    message.contains("failed to connect to omo app-server")
        || message.contains("timeout connecting to omo app-server")
}

/// Errors that mean the app-server connection was reset or dropped in flight
/// (for example when the daemon process was force-restarted by another turn; incident 2026-10-16).
fn is_transport_reset(error: &OmonError) -> bool {
    let msg = error.to_string();
    msg.contains("Connection reset")
        || msg.contains("connection reset")
        || msg.contains("os error 54")
        || msg.contains("Broken pipe")
        || msg.contains("connection closed")
        || msg.contains("closed before")
        || msg.contains("ws streaming error")
        || msg.contains("os error 10054")
        || msg.contains("os error 10053")
}

#[async_trait]
impl AgentBackend for OmoBackend {
    async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
        let is_cron_session = session.key.user_id.starts_with("cron:")
            || session
                .state
                .metadata
                .get("cron_scheduler_delivery")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        let is_lane = session.key.platform == "lane";
        if is_lane {
            let lane_cwd = session
                .state
                .metadata
                .get("lane_cwd")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    OmonError::Config("lane session missing required 'lane_cwd' metadata".into())
                })?;
            let cwd_path = PathBuf::from(lane_cwd);
            if !cwd_path.is_absolute() {
                return Err(OmonError::Config(format!(
                    "lane_cwd must be an absolute path: {}",
                    cwd_path.display()
                )));
            }
            match tokio::fs::metadata(&cwd_path).await {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    return Err(OmonError::Config(format!(
                        "lane_cwd is not a directory: {}",
                        cwd_path.display()
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(OmonError::Config(format!(
                        "lane_cwd does not exist: {}",
                        cwd_path.display()
                    )));
                }
                Err(error) => {
                    return Err(OmonError::Config(format!(
                        "lane_cwd cannot be read: {error}"
                    )));
                }
            }
        }
        let effective_total_timeout = if is_cron_session {
            self.config.total_timeout
        } else if is_lane {
            if self.config.total_timeout != Duration::from_secs(1800) {
                self.config.total_timeout
            } else {
                crate::agent::omo_config::lane_turn_timeout_from_env()?
            }
        } else {
            self.config.total_timeout.min(INTERACTIVE_TURN_CAP)
        };
        let deadline = tokio::time::Instant::now() + effective_total_timeout;
        let follows_engine_turns = self.follow_engine_turns && !is_cron_session && !is_lane;
        if follows_engine_turns {
            self.stop_following(&session.key.storage_key());
        }

        // Cron deliveries are posted by the scheduler, not by this thread. Take the
        // pending ones once for every attempt of this run (a model fallback re-runs
        // the turn); they are marked consumed only after turn/start is acknowledged.
        let briefings = match (&self.pool, !is_cron_session && !is_lane) {
            (Some(pool), true) => {
                match crate::mirror::peek_pending_cron_briefings(pool, &session.key.storage_key())
                    .await
                {
                    Ok(briefings) => briefings,
                    Err(error) => {
                        tracing::warn!(session = %session.key, %error, "failed to load cron briefings");
                        None
                    }
                }
            }
            _ => None,
        };

        // Positive pre-submit setup: connect, initialize, resolve thread.
        // If a transport error occurs BEFORE turn/start submission, retry once
        // within the deadline budget.
        let setup_outcome = self
            .setup_turn(session, deadline, effective_total_timeout)
            .await;
        let (ws, thread_id) = match setup_outcome {
            Ok(pair) => pair,
            Err(err) => {
                let retryable = matches!(
                    &err,
                    OmonError::Llm(msg)
                        if msg.contains("Connection reset")
                            || msg.contains("os error 54")
                            || msg.contains("Broken pipe")
                            || msg.contains("connection closed")
                            || msg.contains("closed before")
                            || msg.contains("Handshake not finished")
                            || msg.contains("Connection refused")
                            || msg.contains("os error 61")
                            || msg.contains("failed to connect to omo app-server")
                            || msg.contains("ws error in initialize")
                            || msg.contains("ws error during initialize")
                            || msg.contains("ws error in thread/")
                            || msg.contains("ws error during thread/")
                            || msg.contains("os error 10053")
                            || msg.contains("os error 10054")
                            || msg.contains("os error 10061")
                            || msg.contains("중단되었습니다")
                            || msg.contains("현재 연결은")
                );
                if retryable {
                    tracing::warn!(
                        "omo daemon connection reset or refused before turn submission; retrying once after cooldown"
                    );
                    let cooldown = Duration::from_millis(500);
                    if tokio::time::Instant::now() + cooldown >= deadline {
                        tokio::time::sleep_until(deadline).await;
                        return Err(OmonError::Llm(format!(
                            "turn exceeded total deadline of {effective_total_timeout:?}"
                        )));
                    }
                    tokio::time::sleep(cooldown).await;
                    match self
                        .setup_turn(session, deadline, effective_total_timeout)
                        .await
                    {
                        Ok(pair) => pair,
                        // Still unreachable: the daemon is wedged, not restarting. Replace
                        // the one this gateway owns now instead of failing the message
                        // (incident 2026-10-16: 4h38m of reset/timeout with no restart).
                        Err(err)
                            if is_connect_failure(&err)
                                && crate::agent::force_restart_owned_daemon(
                                    &self.config.appserver_url,
                                )
                                .await =>
                        {
                            tracing::warn!(
                                session = %session.key,
                                error = %err,
                                "omo app-server unreachable; restarted it, retrying setup once"
                            );
                            self.setup_turn(session, deadline, effective_total_timeout)
                                .await?
                        }
                        Err(err) => return Err(err),
                    }
                } else {
                    return Err(err);
                }
            }
        };

        // Execution phase: turn/start socket write begins here. Once submission
        // may have begun or the turn is accepted, we must NEVER resubmit the turn
        // automatically even on disconnect, because the peer does not deduplicate turns.
        let fallback_model = &self.config.fallback_model;
        let primary_model = session
            .state
            .active_model
            .as_deref()
            .or(self.config.default_model.as_deref())
            .map(str::to_owned);
        let mut fallback_eligible = false;
        // Set once a tool started or a message reached the user: such a turn is never resubmitted.
        let mut side_effects = false;
        let turn_started = std::time::Instant::now();
        // The engine turn id this run owned last: the follower must not re-deliver its reply.
        let mut last_turn: Option<String> = None;
        let mut result = self
            .execute_turn(
                ws,
                thread_id,
                session,
                event.clone(),
                deadline,
                effective_total_timeout,
                None,
                &mut fallback_eligible,
                &mut side_effects,
                &mut last_turn,
                briefings.as_ref(),
            )
            .await;
        let mut retried = false;
        // A turn that got no frames back is how a wedged app-server looks from here (it
        // accepts the prompt, runs it, and never forwards the events). Restart the daemon
        // this gateway owns and run the turn once more instead of surfacing the error.
        // A silent model looks the same from the event stream, but then the app-server still
        // answers readyz; killing it would abort every other session's turn and fix nothing.
        if result.as_ref().is_err_and(is_backend_stall)
            && !crate::agent::owned_daemon_responsive(&self.config.appserver_url).await
            && crate::agent::force_restart_owned_daemon(&self.config.appserver_url).await
        {
            retried = true;
            tracing::warn!(
                session = %session.key,
                error = ?result,
                "omo app-server stopped forwarding turn events; restarted it, retrying the turn once"
            );
            let deadline = tokio::time::Instant::now() + effective_total_timeout;
            let (ws, thread_id) = self
                .setup_turn(session, deadline, effective_total_timeout)
                .await?;
            fallback_eligible = false;
            result = self
                .execute_turn(
                    ws,
                    thread_id,
                    session,
                    event.clone(),
                    deadline,
                    effective_total_timeout,
                    None,
                    &mut fallback_eligible,
                    &mut side_effects,
                    &mut last_turn,
                    briefings.as_ref(),
                )
                .await;
        }
        if !retried
            && !side_effects
            && result.as_ref().is_err_and(is_transport_reset)
            && crate::agent::forced_restart_since(&self.config.appserver_url, turn_started)
        {
            // Incident 2026-10-16: when a stalled turn in another session triggers a forced
            // restart of the owned omo app-server, concurrent turns in flight are terminated
            // by SIGKILL and fail with a transport reset (e.g. "ws streaming error: Connection reset").
            // The killed daemon cannot finish the turn, but tools it already ran and messages
            // already relayed stay done, so only a turn with no side effects yet is resubmitted.
            // Wait until the replacement daemon answers readyz, then retry the turn once.
            if crate::agent::wait_owned_daemon_ready(
                &self.config.appserver_url,
                Duration::from_secs(60),
            )
            .await
            {
                tracing::warn!(
                    session = %session.key,
                    error = ?result,
                    "turn lost to a forced app-server restart; retrying once"
                );
                let deadline = tokio::time::Instant::now() + effective_total_timeout;
                let (ws, thread_id) = self
                    .setup_turn(session, deadline, effective_total_timeout)
                    .await?;
                fallback_eligible = false;
                result = self
                    .execute_turn(
                        ws,
                        thread_id,
                        session,
                        event.clone(),
                        deadline,
                        effective_total_timeout,
                        None,
                        &mut fallback_eligible,
                        &mut side_effects,
                        &mut last_turn,
                        briefings.as_ref(),
                    )
                    .await;
            }
        }
        if result.is_err()
            && fallback_eligible
            && primary_model
                .as_deref()
                .is_some_and(|model| model != fallback_model)
        {
            tracing::warn!(
                model = fallback_model,
                error = ?result,
                "retrying turn failure with fallback model"
            );
            let deadline = tokio::time::Instant::now() + effective_total_timeout;
            let (ws, thread_id) = self
                .setup_turn(session, deadline, effective_total_timeout)
                .await?;
            result = self
                .execute_turn(
                    ws,
                    thread_id,
                    session,
                    event,
                    deadline,
                    effective_total_timeout,
                    Some(fallback_model.as_str()),
                    &mut fallback_eligible,
                    &mut side_effects,
                    &mut last_turn,
                    briefings.as_ref(),
                )
                .await;
        }
        if follows_engine_turns {
            if let Some(thread_id) = session
                .state
                .metadata
                .get("omo_thread_id")
                .and_then(Value::as_str)
            {
                self.follow_thread(session, thread_id.to_owned(), last_turn.clone());
            }
        }
        result
    }

    async fn cancel(&self, session: &SessionContext) -> Result<()> {
        let storage_key = session.key.storage_key();
        let active = self.active_turns.lock().get(&storage_key).cloned();
        let Some(active) = active else {
            return Ok(());
        };
        let Some(turn_id) = active.turn_id.as_deref() else {
            return Err(OmonError::Llm(format!(
                "cancellation unresolved: turn/start on thread {} may have been accepted, but no turn id was acknowledged",
                active.thread_id
            )));
        };

        // Bounded cancellation: reserve up to 5s for interrupt/terminal cleanup
        let cancel_timeout = self.config.request_timeout.min(Duration::from_secs(5));
        let cancel_deadline = tokio::time::Instant::now() + cancel_timeout;
        let result = tokio::time::timeout_at(cancel_deadline, async {
            let mut ws = self.connect_ws(cancel_deadline).await?;
            self.do_initialize(&mut ws, cancel_deadline).await?;

            let interrupt_id = 9_001u64;
            let interrupt = json!({
                "jsonrpc": "2.0",
                "id": interrupt_id,
                "method": "turn/interrupt",
                "params": {
                    "threadId": active.thread_id,
                    "turnId": turn_id,
                }
            });
            ws.send(Message::text(interrupt.to_string()))
                .await
                .map_err(|e| OmonError::Llm(format!("failed to send turn/interrupt: {e}")))?;

            while let Some(msg) = ws.next().await {
                let msg = msg
                    .map_err(|e| OmonError::Llm(format!("ws error during turn/interrupt: {e}")))?;
                if let Message::Text(text) = msg {
                    let val: Value = serde_json::from_str(&text).map_err(|e| {
                        OmonError::Llm(format!("invalid json in interrupt ack: {e}"))
                    })?;
                    if val.get("method").is_none()
                        && val.get("id").and_then(Value::as_u64) == Some(interrupt_id)
                    {
                        if let Some(error) = val.get("error") {
                            return Err(OmonError::Llm(format!(
                                "turn/interrupt error from peer: {error}"
                            )));
                        }
                        if val.get("result").is_some() {
                            return Ok(());
                        }
                        return Err(OmonError::Llm(
                            "turn/interrupt acknowledgement missing result".into(),
                        ));
                    }
                    if val.get("method").and_then(Value::as_str) == Some("turn/completed") {
                        let status = val.pointer("/params/turn/status").and_then(Value::as_str);
                        let frame_turn = val
                            .pointer("/params/turn/id")
                            .or_else(|| val.pointer("/params/turnId"))
                            .and_then(Value::as_str);
                        if val.pointer("/params/threadId").and_then(Value::as_str)
                            == Some(active.thread_id.as_str())
                            && frame_turn == Some(turn_id)
                            && (status == Some("interrupted")
                                || status == Some("completed")
                                || status == Some("failed"))
                        {
                            return Ok(());
                        }
                    }
                }
            }
            Err(OmonError::Llm(
                "connection closed before turn/interrupt acknowledged".into(),
            ))
        })
        .await
        .map_err(|_| OmonError::Llm("timeout waiting for turn/interrupt acknowledgement".into()))?;
        if result.is_ok() {
            let mut active_turns = self.active_turns.lock();
            if active_turns.get(&storage_key) == Some(&active) {
                active_turns.remove(&storage_key);
            }
        }
        result
    }

    async fn resolve_owner_reply(&self, session: &crate::SessionKey, text: &str) -> Result<bool> {
        if !self.is_ledger_enabled() {
            return Ok(false);
        }
        let guard = self.approval_requester.as_ref().map(|r| r.guard());
        Ok(
            crate::autonomy::try_resolve_short_reply(self.pool.as_ref(), guard, session, text)
                .await
                .unwrap_or(false),
        )
    }

    async fn steer(&self, session: &SessionContext, guidance: &str) -> Result<bool> {
        let storage_key = session.key.storage_key();
        let active = self.active_turns.lock().get(&storage_key).cloned();
        let Some(active) = active else {
            return Ok(false);
        };
        let Some(turn_id) = active.turn_id.as_deref() else {
            return Ok(false);
        };

        // Bounded steer: one short-lived socket round trip, never blocking the caller for long.
        let steer_timeout = self.config.request_timeout.min(Duration::from_secs(5));
        let steer_deadline = tokio::time::Instant::now() + steer_timeout;
        let mut ws = self.connect_ws(steer_deadline).await?;
        self.do_initialize(&mut ws, steer_deadline).await?;

        let steer_req_id = 9_500u64;
        let steer_msg = turn_steer_request(steer_req_id, &active.thread_id, turn_id, guidance);
        ws.send(steer_msg)
            .await
            .map_err(|e| OmonError::Llm(format!("failed to send turn/steer: {e}")))?;

        // The peer rejects a steer whose turn already ended or whose id moved on. Only a real
        // acknowledgement counts as delivered: reporting success on a rejected steer would
        // mark the message delivered and silently drop it from the conversation.
        loop {
            let frame = match tokio::time::timeout_at(steer_deadline, ws.next()).await {
                Ok(Some(frame)) => frame,
                Ok(None) => return Ok(false),
                Err(_) => return Ok(false),
            };
            let frame = frame.map_err(|e| OmonError::Llm(format!("ws error during steer: {e}")))?;
            let Message::Text(text) = frame else {
                continue;
            };
            let Ok(val) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            if val.get("id").and_then(Value::as_u64) != Some(steer_req_id) {
                continue;
            }
            if let Some(error) = val.get("error") {
                tracing::warn!(
                    session = %session.key,
                    turn_id = %turn_id,
                    %error,
                    "turn/steer rejected by app-server; falling back to a fresh turn"
                );
                return Ok(false);
            }
            tracing::info!(
                session = %session.key,
                thread_id = %active.thread_id,
                turn_id = %turn_id,
                "injected instant steer into active turn"
            );
            return Ok(true);
        }
    }

    async fn tool_in_flight(&self, session: &SessionContext) -> Option<bool> {
        self.active_turns
            .lock()
            .get(&session.key.storage_key())
            .map(|active| active.open_tools > 0)
    }
}

async fn clear_session_binding(pool: &sqlx::SqlitePool, session: &SessionContext) -> Result<()> {
    ensure_session_row(pool, session).await?;
    sqlx::query(
        "UPDATE sessions
         SET state_json = json_remove(COALESCE(NULLIF(state_json, ''), '{}'), '$.metadata.omo_thread_id'),
             updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         WHERE session_key = ?",
    )
    .bind(session.key.storage_key())
    .execute(pool)
    .await
    .map_err(|e| OmonError::Database(format!("clear thread binding: {e}")))?;
    Ok(())
}

/// True when a thread session of the same channel is bound to `omo_thread_id`. A thread is
/// the more specific conversation: work started there (and lane reports routed by the
/// conversation id) must not continue in the channel-wide session.
async fn thread_owns_conversation(
    pool: &sqlx::SqlitePool,
    session: &SessionContext,
    omo_thread_id: &str,
) -> Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sessions
         WHERE platform = ? AND channel_id = ? AND thread_id IS NOT NULL AND session_key <> ?
           AND json_extract(state_json, '$.metadata.omo_thread_id') = ?)",
    )
    .bind(&session.key.platform)
    .bind(&session.key.channel_id)
    .bind(session.key.storage_key())
    .bind(omo_thread_id)
    .fetch_one(pool)
    .await
    .map_err(|e| OmonError::Database(format!("thread binding lookup: {e}")))
}

async fn persist_session_binding(
    pool: &sqlx::SqlitePool,
    session: &SessionContext,
    omo_thread_id: &str,
) -> Result<()> {
    ensure_session_row(pool, session).await?;
    sqlx::query(
        "UPDATE sessions
         SET state_json = json_set(COALESCE(NULLIF(state_json, ''), '{}'), '$.metadata.omo_thread_id', ?),
             updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         WHERE session_key = ?",
    )
    .bind(omo_thread_id)
    .bind(session.key.storage_key())
    .execute(pool)
    .await
    .map_err(|e| OmonError::Database(format!("persist thread binding: {e}")))?;
    Ok(())
}

async fn ensure_session_row(pool: &sqlx::SqlitePool, session: &SessionContext) -> Result<()> {
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT OR IGNORE INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(session.key.storage_key())
    .bind(&session.key.platform)
    .bind(&session.key.guild_id)
    .bind(&session.key.channel_id)
    .bind(&session.key.thread_id)
    .bind(&session.key.user_id)
    .bind(serde_json::to_string(&session.state).unwrap_or_else(|_| "{}".to_string()))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| OmonError::Llm(format!("ensure session row: {e}")))?;
    Ok(())
}

async fn persist_message(
    pool: &SqlitePool,
    session: &SessionContext,
    content: &str,
    obligation_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now();
    // A relayed frame can arrive more than once for one turn (resume replays, pre-ack relays);
    // the transcript keeps one row per delivered text, not one per frame.
    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at)
         SELECT ?, ?, ?, ?, ?, ?
         WHERE NOT EXISTS (
             SELECT 1 FROM messages
             WHERE session_key = ? AND role = 'assistant' AND content = ?
               AND json_extract(metadata_json, '$.delivery_obligation_id') = ?
         )",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(session.key.storage_key())
    .bind("assistant")
    .bind(content)
    .bind(json!({"delivery_obligation_id": obligation_id}).to_string())
    .bind(now)
    .bind(session.key.storage_key())
    .bind(content)
    .bind(obligation_id)
    .execute(pool)
    .await
    .map_err(|e| OmonError::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod active_turn_staleness_tests {
    use std::time::Duration;

    use super::active_turn_entry_is_stale;

    #[test]
    fn fresh_entry_within_turn_budget_is_not_stale() {
        // given
        let budget = Duration::from_secs(1200);

        // when / then
        assert!(!active_turn_entry_is_stale(Duration::from_secs(0), budget));
        assert!(!active_turn_entry_is_stale(
            budget + super::ACTIVE_TURN_STALE_MARGIN - Duration::from_secs(1),
            budget
        ));
    }

    #[test]
    fn entry_older_than_budget_plus_margin_is_stale() {
        // given
        let budget = Duration::from_secs(300);

        // when / then
        assert!(active_turn_entry_is_stale(
            budget + super::ACTIVE_TURN_STALE_MARGIN + Duration::from_secs(1),
            budget
        ));
    }

    #[test]
    fn agent_progress_excludes_steer_echoes_and_bookkeeping() {
        let user = serde_json::json!({"params":{"item":{"type":"userMessage"}}});
        assert!(!super::is_agent_progress("item/started", &user));
        assert!(!super::is_agent_progress("item/completed", &user));
        assert!(!super::is_agent_progress("turn/started", &user));
        let tool = serde_json::json!({"params":{"item":{"type":"commandExecution"}}});
        assert!(super::is_agent_progress("item/started", &tool));
        assert!(super::is_agent_progress(
            "item/reasoning/textDelta",
            &serde_json::json!({})
        ));
    }
}

#[cfg(test)]
mod fixr5_agent_tests {
    use serde_json::json;
    use std::time::Duration;

    use super::{
        ensure_cron_ttsr_disabled, frame_is_turn_progress, is_backend_stall, is_transport_reset,
        should_relay_interim, INTERIM_REPORT_INTERVAL,
    };
    use crate::OmonError;

    #[test]
    fn interim_reports_are_the_ack_then_spaced_updates() {
        let start = tokio::time::Instant::now();
        // The first narration is the acknowledgement.
        assert!(should_relay_interim(0, None, start));
        // Narrations right after it are held back.
        assert!(!should_relay_interim(
            1,
            Some(start),
            start + Duration::from_secs(20)
        ));
        // A long task gets an update once the interval has passed.
        assert!(should_relay_interim(
            1,
            Some(start),
            start + INTERIM_REPORT_INTERVAL
        ));
    }

    #[test]
    fn only_missing_turn_events_count_as_a_backend_stall() {
        use crate::OmonError;
        assert!(is_backend_stall(&OmonError::Llm(
            "turn interrupted: agent produced no output within 45s of starting the prompt".into()
        )));
        assert!(is_backend_stall(&OmonError::Llm(
            "timeout during turn streaming: no activity from server".into()
        )));
        assert!(!is_backend_stall(&OmonError::Llm(
            "turn exceeded total deadline of 1800s; turn/interrupt sent".into()
        )));
        assert!(!is_backend_stall(&OmonError::Llm(
            "turn/start error: model rejected".into()
        )));
        // Silence after tool work must not restart the daemon and re-run the prompt.
        assert!(!is_backend_stall(&OmonError::Llm(
            "timeout during turn streaming: server went silent after tool calls".into()
        )));
    }

    #[test]
    fn steer_acks_and_unrelated_frames_do_not_count_as_turn_progress() {
        let progress = |frame: serde_json::Value, awaiting_successor: bool| {
            let method = frame.get("method").and_then(|m| m.as_str()).unwrap_or("");
            frame_is_turn_progress(method, &frame, "th1", Some("t1"), awaiting_successor)
        };
        // The gateway's own steer/interrupt acks carry no method.
        assert!(!progress(json!({"id": 9100, "result": {}}), false));
        assert!(!progress(json!({"id": 9001, "result": {}}), false));
        // Status noise and frames of other threads or turns.
        assert!(!progress(
            json!({"method":"thread/status/changed","params":{"threadId":"th1","turnId":"t1"}}),
            false
        ));
        assert!(!progress(
            json!({"method":"item/started","params":{"threadId":"other","turnId":"t1","item":{"type":"commandExecution"}}}),
            false
        ));
        assert!(!progress(
            json!({"method":"item/started","params":{"threadId":"th1","turnId":"t0","item":{"type":"commandExecution"}}}),
            false
        ));
        // The gateway's own steer echoed back as a user message is not agent work.
        assert!(!progress(
            json!({"method":"item/completed","params":{"threadId":"th1","turnId":"t1","item":{"type":"userMessage"}}}),
            false
        ));
        // Real work for this thread and turn renews the watchdogs.
        assert!(progress(
            json!({"method":"item/started","params":{"threadId":"th1","turnId":"t1","item":{"type":"commandExecution"}}}),
            false
        ));
        assert!(progress(
            json!({"method":"item/agentMessage/delta","params":{"threadId":"th1","turnId":"t1","delta":"x"}}),
            false
        ));
        assert!(progress(
            json!({"method":"turn/completed","params":{"threadId":"th1","turn":{"id":"t1","status":"completed"}}}),
            false
        ));
        assert!(progress(
            json!({"id": 5, "method":"item/commandExecution/requestApproval","params":{"threadId":"th1"}}),
            false
        ));
        // The successor of a queued prompt's turn is followed only while awaited.
        let successor =
            json!({"method":"turn/started","params":{"threadId":"th1","turn":{"id":"t9"}}});
        assert!(!progress(successor.clone(), false));
        assert!(progress(successor, true));
    }

    #[tokio::test]
    async fn cron_settings_json_is_merged_atomically_and_never_clobbered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        // Missing file: created with the extension disabled.
        ensure_cron_ttsr_disabled(&path).await;
        let created: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(created["disabledBuiltinExtensions"], json!(["ttsr"]));

        // Existing settings are preserved and already-correct files are not rewritten.
        std::fs::write(
            &path,
            r#"{"theme":"dark","disabledBuiltinExtensions":["a","ttsr"]}"#,
        )
        .unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        ensure_cron_ttsr_disabled(&path).await;
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
        ensure_cron_ttsr_disabled(&path).await;
        let merged: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(merged["theme"], "dark");
        assert_eq!(merged["disabledBuiltinExtensions"], json!(["ttsr"]));
        assert!(!dir.path().join("settings.json.tmp").exists());

        // A file that is not strict JSON is left exactly as the user wrote it.
        let hand_edited = "{\n  // my settings\n  \"theme\": \"dark\",\n}\n";
        std::fs::write(&path, hand_edited).unwrap();
        ensure_cron_ttsr_disabled(&path).await;
        assert_eq!(std::fs::read_to_string(&path).unwrap(), hand_edited);
    }

    #[test]
    fn transport_reset_matching_identifies_ws_and_connection_errors() {
        assert!(is_transport_reset(&OmonError::Llm(
            "ws streaming error: WebSocket protocol error: Connection reset without closing handshake".into()
        )));
        assert!(is_transport_reset(&OmonError::Llm(
            "connection closed before message completed".into()
        )));
        assert!(is_transport_reset(&OmonError::Llm(
            "os error 54: Connection reset by peer".into()
        )));
        assert!(is_transport_reset(&OmonError::Llm(
            "Broken pipe (os error 32)".into()
        )));
        assert!(!is_transport_reset(&OmonError::Llm(
            "agent produced no output within 300s".into()
        )));
        assert!(!is_transport_reset(&OmonError::Llm(
            "invalid prompt argument".into()
        )));
    }
}

#[cfg(test)]
mod review_20261020_agent_tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{
        expire_abandoned_proposal, is_thread_busy_error, OmoBackend, PreAckAction, PreAckState,
        TurnAskScope,
    };
    use crate::agent::agent_workspace::WORKSPACE_PERMISSION_PRESET;
    use crate::agent::omo_config::OmoBackendConfig;
    use crate::autonomy::{
        insert_proposal, Classification, Effect, NewProposal, ProposalOrigin, ProposalState,
    };
    use crate::models::OutboundAction;
    use crate::{OutboundDispatcher, Result};

    struct NullDispatcher;

    #[async_trait::async_trait]
    impl OutboundDispatcher for NullDispatcher {
        async fn dispatch(&self, _action: OutboundAction) -> Result<()> {
            Ok(())
        }
    }

    fn backend(ledger_enabled: bool) -> OmoBackend {
        OmoBackend::new(OmoBackendConfig::default(), Arc::new(NullDispatcher))
            .with_ledger_enabled(ledger_enabled)
    }

    #[test]
    fn a_busy_thread_rejection_is_told_apart_from_a_real_turn_start_failure() {
        // The exact rejection the app-server returns while another turn is processing.
        assert!(is_thread_busy_error(
            r#"{"code":-32603,"message":"Agent is already processing. Specify streamingBehavior ('steer' or 'followUp') to queue the message."}"#
        ));
        assert!(!is_thread_busy_error("model rejected"));
        assert!(!is_thread_busy_error(
            "turn exceeded total deadline of 1800s; turn/interrupt sent"
        ));
    }

    #[test]
    fn a_busy_refusal_waits_for_the_same_thread_to_go_idle_and_then_resubmits_once() {
        // given: a refusal for a thread whose running turn is still open
        let mut state = PreAckState::default();
        let busy = json!({"code":-32603,"message":"Agent is already processing."}).to_string();
        assert!(state.note_refusal(&busy));
        assert!(
            !state.idle_already(),
            "a running turn is still open: must wait"
        );

        // when: progress for that turn, then a completion for another thread
        let started =
            json!({"method":"turn/started","params":{"threadId":"th1","turn":{"id":"t-run"}}});
        assert_eq!(state.observe(&started, "th1"), PreAckAction::Continue);
        let foreign = json!({"method":"turn/completed","params":{"threadId":"th-other","turn":{"id":"t-run"}}});

        // then: another thread's idle state never triggers our resubmission
        assert_eq!(state.observe(&foreign, "th1"), PreAckAction::Continue);

        // when: the turn that blocked us completes on our thread
        let done =
            json!({"method":"turn/completed","params":{"threadId":"th1","turn":{"id":"t-run"}}});

        // then: exactly one resubmission is requested
        assert_eq!(state.observe(&done, "th1"), PreAckAction::Resubmit);
        assert!(!state.note_refusal(&busy), "a second refusal must not loop");
        assert_eq!(state.observe(&done, "th1"), PreAckAction::Continue);
    }

    #[test]
    fn a_turn_that_finished_before_the_refusal_is_resubmitted_immediately() {
        // given: the running turn completed while our refusal was in flight
        let mut state = PreAckState::default();
        let done =
            json!({"method":"turn/completed","params":{"threadId":"th1","turn":{"id":"t-run"}}});
        assert_eq!(state.observe(&done, "th1"), PreAckAction::Continue);

        // when: the refusal arrives afterwards
        let busy = json!({"code":-32603,"message":"Agent is already processing."}).to_string();
        assert!(state.note_refusal(&busy));

        // then: the thread is already idle, so no further frame will ever arrive
        assert!(
            state.idle_already(),
            "a completion seen before the refusal means the thread is idle now"
        );
    }

    #[test]
    fn a_turn_ending_cancels_its_approval_scope() {
        // given
        let token = {
            let scope = TurnAskScope::new();
            let token = scope.token();
            assert!(!token.is_cancelled());
            token
        };

        // then: dropping the scope on any return path of the turn cancels every Ask task
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn a_turn_ending_expires_its_pending_approval_instead_of_leaving_it_approvable() {
        // given: a pending proposal a turn opened
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let classification = Classification::new(
            Effect::VcsRemote,
            "git push",
            "origin/main",
            "git push origin main",
        );
        let pending = insert_proposal(
            pool,
            &NewProposal {
                origin: ProposalOrigin::EngineApproval,
                session_key: "slack:T1:C1::U1".into(),
                signature: classification.signature.clone(),
                effect: classification.effect,
                target: classification.target.clone(),
                summary: classification.summary.clone(),
                payload_json: "{}".into(),
                tainted: false,
                state: ProposalState::Pending,
                precedent_id: None,
                owner_words: None,
                platform_message_id: None,
            },
        )
        .await
        .unwrap();

        // when: the turn that asked ends without an answer
        assert!(expire_abandoned_proposal(pool, pending).await.unwrap());

        // then: the card can no longer be approved for an action nothing will run
        let stored = crate::autonomy::get_proposal(pool, pending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.state, ProposalState::Expired.as_str());
        assert!(stored.decided_at.is_some());
        // and the conditional update is idempotent: a second turn end changes nothing
        assert!(!expire_abandoned_proposal(pool, pending).await.unwrap());

        // and an already decided proposal is never rewritten, which is what makes the
        // conditional UPDATE (not a read followed by an unconditional write) the safe shape:
        // an owner's approval landing at the same moment must survive the turn's expiry.
        let approved = insert_proposal(
            pool,
            &NewProposal {
                origin: ProposalOrigin::EngineApproval,
                session_key: "slack:T1:C1::U1".into(),
                signature: classification.signature.clone(),
                effect: classification.effect,
                target: classification.target.clone(),
                summary: classification.summary.clone(),
                payload_json: "{}".into(),
                tainted: false,
                state: ProposalState::Approved,
                precedent_id: None,
                owner_words: None,
                platform_message_id: None,
            },
        )
        .await
        .unwrap();
        assert!(!expire_abandoned_proposal(pool, approved).await.unwrap());
        let stored = crate::autonomy::get_proposal(pool, approved)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.state, ProposalState::Approved.as_str());
        assert!(
            stored.decided_at.is_some(),
            "the approval's decision timestamp must not be replaced"
        );
    }

    #[tokio::test]
    async fn workspace_preset_provisioning_follows_the_backend_ledger_override() {
        let dir = tempfile::tempdir().unwrap();

        let enabled_path = dir.path().join("enabled.json");
        assert!(backend(true).ensure_workspace_preset(&enabled_path).await);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&enabled_path).unwrap()).unwrap();
        assert_eq!(
            written["permissionPreset"],
            json!(WORKSPACE_PERMISSION_PRESET)
        );

        let disabled_path = dir.path().join("disabled.json");
        assert!(!backend(false).ensure_workspace_preset(&disabled_path).await);
        assert!(
            !disabled_path.exists(),
            "a backend with the ledger off must not provision the preset even when the env var says on"
        );
    }
}

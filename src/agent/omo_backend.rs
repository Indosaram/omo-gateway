// allow: SIZE_OK — OMO WebSocket protocol state machine
use std::collections::{hash_map::Entry, HashMap};
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
use uuid::Uuid;

use super::agent_workspace::{agent_workspace_slug, resolve_workspace};
use super::backend::AgentBackend;

use super::omo_config::OmoBackendConfig;
use super::omo_protocol::{
    approval_allow_response, approval_denial_response, initialize_request, is_approval_request,
    thread_resume_request, thread_start_request, turn_start_request,
};
use crate::models::{
    filter_intent_gate, filter_reasoning, is_explicit_silence, render_user_prompt, InboundEvent,
    OutboundAction, SessionContext, StreamChunk,
};
use crate::{OmonError, OutboundDispatcher, Result};

type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

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

/// Extra slack beyond a turn's own total deadline before a refusal-guard entry
/// is presumed dead and a new submission may take its place.
pub const ACTIVE_TURN_STALE_MARGIN: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTurn {
    pub thread_id: String,
    /// None means submission may have reached the peer but no start ACK was observed.
    pub turn_id: Option<String>,
    /// When this submission was recorded, consumed by the stale-entry reclaim.
    pub inserted_at: tokio::time::Instant,
}

/// A guard entry older than the turn's total deadline plus the stale margin
/// cannot be a live turn: the deadline has elapsed, so reclaim the entry
/// instead of bricking the session until the next gateway restart.
fn active_turn_entry_is_stale(age: Duration, effective_total_timeout: Duration) -> bool {
    age > effective_total_timeout + ACTIVE_TURN_STALE_MARGIN
}

/// Interactive (Discord) turns stop at 1800s (30m); cron turns keep their own budget.
const INTERACTIVE_TURN_CAP: Duration = Duration::from_secs(1800);

pub struct OmoBackend {
    pub config: OmoBackendConfig,
    pub pool: Option<SqlitePool>,
    pub dispatcher: Arc<dyn OutboundDispatcher>,
    pub thread_ids: Arc<ParkingMutex<HashMap<String, String>>>,
    pub active_turns: Arc<ParkingMutex<HashMap<String, ActiveTurn>>>,
}

impl OmoBackend {
    pub fn new(config: OmoBackendConfig, dispatcher: Arc<dyn OutboundDispatcher>) -> Self {
        Self {
            config,
            pool: None,
            dispatcher,
            thread_ids: Arc::new(ParkingMutex::new(HashMap::new())),
            active_turns: Arc::new(ParkingMutex::new(HashMap::new())),
        }
    }

    pub fn with_pool(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool);
        self
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
    ///    When a session does not yet have an assigned `omo_thread_id` (in session metadata or
    ///    the in-memory cache), `thread/start` is dispatched with:
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
        if is_cron {
            session.state.metadata.remove("omo_thread_id");
            self.thread_ids.lock().remove(&storage_key);
        } else if let Some(id) = session
            .state
            .metadata
            .get("omo_thread_id")
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| self.thread_ids.lock().get(&storage_key).cloned())
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

        let workspace = if self.config.per_agent_workspace {
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
                    let settings_path = omo_dir.join("settings.json");
                    let mut settings: serde_json::Map<String, Value> =
                        if tokio::fs::try_exists(&settings_path).await.unwrap_or(false) {
                            tokio::fs::read_to_string(&settings_path)
                                .await
                                .ok()
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_default()
                        } else {
                            serde_json::Map::new()
                        };
                    let disabled = settings
                        .entry("disabledBuiltinExtensions".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = disabled {
                        if !arr.iter().any(|v| v.as_str() == Some("ttsr")) {
                            arr.push(Value::String("ttsr".to_string()));
                        }
                    }
                    if let Ok(content) = serde_json::to_string_pretty(&Value::Object(settings)) {
                        let _ = tokio::fs::write(&settings_path, content.as_bytes()).await;
                    }
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
    ) -> Result<()> {
        let user_prompt = render_user_prompt(&event);
        let model = model_override.or(session
            .state
            .active_model
            .as_deref()
            .or(self.config.default_model.as_deref()));
        // Invariant: model changes flow only via active_model / turn/start params / thread/settings/update.
        // omo_config reads global omo.json but never writes it.
        if let Some(model) = model.filter(|_| {
            model_override.is_some()
                || session
                    .state
                    .metadata
                    .get("omo_fallback_model_active")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        }) {
            ws.send(Message::text(
                json!({
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "thread/settings/update",
                    "params": { "threadId": thread_id, "model": model }
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
                    if response.get("id").and_then(Value::as_u64) == Some(4) {
                        if let Some(error) = response.get("error") {
                            let err_str = error.to_string();
                            *fallback_eligible = is_model_rejected_error(&err_str);
                            return Err(OmonError::Llm(format!("model update rejected: {error}")));
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
        {
            let mut active_turns = self.active_turns.lock();
            match active_turns.entry(session.key.storage_key()) {
                Entry::Vacant(entry) => {
                    entry.insert(ActiveTurn {
                        thread_id: thread_id.clone(),
                        turn_id: None,
                        inserted_at: submitted_at,
                    });
                }
                Entry::Occupied(mut occupied) => {
                    let age = submitted_at.duration_since(occupied.get().inserted_at);
                    if !active_turn_entry_is_stale(age, effective_total_timeout) {
                        return Err(OmonError::Llm(
                            "previous omo turn outcome is unresolved; refusing another turn/start"
                                .into(),
                        ));
                    }
                    tracing::warn!(
                        age_secs = age.as_secs(),
                        "stale active-turn entry reclaimed; prior turn outcome presumed lost"
                    );
                    occupied.insert(ActiveTurn {
                        thread_id: thread_id.clone(),
                        turn_id: None,
                        inserted_at: submitted_at,
                    });
                }
            }
        }
        if let Err(e) = ws
            .send(turn_start_request(&thread_id, &user_prompt, model))
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

        // Immediately notify Discord that the agent is typing while preparing/reasoning
        if !is_cron_session {
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
        let mut full_content = String::new();
        let mut tool_call_counts: std::collections::BTreeMap<String, usize> = Default::default();
        let mut total_tool_calls: usize = 0;
        let mut started_ids: std::collections::HashSet<String> = Default::default();
        let mut approval_denials: u32 = 0;

        let started_at = tokio::time::Instant::now();
        let mut turn_id: Option<String> = None;
        let mut turn_started_ack = false;
        let mut last_activity_at = tokio::time::Instant::now();
        let _no_content_grace = self.config.no_content_grace;

        let ack_deadline = started_at
            + Duration::from_secs(30).min(deadline.saturating_duration_since(started_at));

        let cleanup_reserve = if effective_total_timeout >= Duration::from_secs(10) {
            Duration::from_secs(5)
        } else if effective_total_timeout >= Duration::from_secs(1) {
            effective_total_timeout / 5
        } else {
            Duration::ZERO
        };
        let work_deadline = deadline.checked_sub(cleanup_reserve).unwrap_or(deadline);

        loop {
            let now = tokio::time::Instant::now();
            if !turn_started_ack && now >= ack_deadline {
                return Err(OmonError::Llm(
                    "timeout waiting for turn/start acknowledgement from omo app-server".into(),
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
                            while let Some(Ok(Message::Text(text))) = ws.next().await {
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

                                session
                                    .state
                                    .metadata
                                    .insert("cron_agent_output".into(), json!(rendered.clone()));

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
                                        .record_obligation(&obl_id, &session.key, &rendered)
                                        .await?;
                                    ledger.mark_obligation_attempting(&obl_id).await?;
                                }

                                let delivered = self
                                    .emit_chunk(
                                        session,
                                        stream_id,
                                        sequence,
                                        rendered.clone(),
                                        true,
                                        reply_to.clone(),
                                    )
                                    .await;

                                if let Some(pool) = &self.pool {
                                    persist_message(pool, session, &rendered).await?;
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
                tokio::time::sleep(Duration::from_millis(50)).await;
                return Err(OmonError::Llm(format!(
                    "turn exceeded total deadline of {:?}; turn/interrupt sent",
                    effective_total_timeout
                )));
            }

            if now >= last_activity_at + self.config.request_timeout {
                if let Some(turn_id) = &turn_id {
                    let interrupt = json!({
                        "jsonrpc": "2.0",
                        "id": 9_003,
                        "method": "turn/interrupt",
                        "params": { "threadId": thread_id, "turnId": turn_id }
                    });
                    let _ = ws.send(Message::text(interrupt.to_string())).await;
                }
                return Err(OmonError::Llm("timeout during turn streaming".into()));
            }

            let mut next_timeout = effective_work_deadline;
            if !turn_started_ack {
                next_timeout = next_timeout.min(ack_deadline);
            }
            next_timeout = next_timeout.min(last_activity_at + self.config.request_timeout);

            let msg = match tokio::time::timeout_at(next_timeout, ws.next()).await {
                Ok(Some(msg)) => msg,
                Ok(None) => break,
                Err(_) => continue,
            };

            let msg = msg.map_err(|e| OmonError::Llm(format!("ws streaming error: {e}")))?;
            let val = match &msg {
                Message::Text(text) => {
                    last_activity_at = tokio::time::Instant::now();
                    Some(
                        serde_json::from_str::<Value>(text.as_str())
                            .map_err(|e| OmonError::Llm(format!("invalid json: {e}")))?,
                    )
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
                    self.active_turns.lock().remove(&session.key.storage_key());
                    let err_str = error.to_string();
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
                        OmonError::Llm("turn/start acknowledgement missing turn id".into())
                    })?;
                let turn_id_str = id.to_string();
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
                    },
                );
                turn_id = Some(turn_id_str);
                turn_started_ack = true;
                continue;
            }

            if let (Some(req_id), Some(method)) =
                (val.get("id"), val.get("method").and_then(Value::as_str))
            {
                if is_approval_request(method) {
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
                            let _ = ws.send(approval_denial_response(req_id)).await;
                            continue;
                        }
                    }

                    if session.state.yolo {
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

                    if auto_approve {
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
            if turn_bearing
                && (!turn_started_ack
                    || val.pointer("/params/threadId").and_then(Value::as_str)
                        != Some(thread_id.as_str())
                    || frame_turn_id != turn_id.as_deref())
            {
                continue;
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

                    if is_started && !item_id.is_empty() && started_ids.insert(item_id.to_string())
                    {
                        let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
                        if !matches!(item_type, "agentMessage" | "userMessage" | "reasoning" | "") {
                            total_tool_calls += 1;
                            let tool_name = item
                                .get("command")
                                .or_else(|| item.get("tool"))
                                .or_else(|| item.get("name"))
                                .or_else(|| item.get("path"))
                                .and_then(Value::as_str)
                                .unwrap_or(item_type);
                            let short_name =
                                tool_name.split_whitespace().next().unwrap_or(tool_name);
                            *tool_call_counts.entry(short_name.to_string()).or_insert(0) += 1;

                            // Live-stream tool activity to Discord so the user
                            // sees progress during long turns instead of just
                            // a typing indicator.
                            if !is_cron_session {
                                let breakdown: Vec<String> = tool_call_counts
                                    .iter()
                                    .map(|(k, v)| {
                                        if *v > 1 {
                                            format!("`{k}` ×{v}")
                                        } else {
                                            format!("`{k}`")
                                        }
                                    })
                                    .collect();
                                let progress = format!(
                                    "-# 🔧 도구 실행 중… ({total_tool_calls}회: {})",
                                    breakdown.join(", ")
                                );
                                let _ = self
                                    .emit_chunk(
                                        session,
                                        stream_id,
                                        sequence,
                                        progress,
                                        false,
                                        reply_to.clone(),
                                    )
                                    .await;
                                sequence = sequence.saturating_add(1);
                            }
                        }
                    }

                    if !is_started {
                        if let Some("agentMessage") = item.get("type").and_then(Value::as_str) {
                            if let Some(text) = item.get("text").and_then(Value::as_str) {
                                full_content.clear();
                                full_content.push_str(text);
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

                    let has_content = !scrubbed_content.is_empty() || total_tool_calls > 0;
                    if !has_content {
                        self.active_turns.lock().remove(&session.key.storage_key());
                        *fallback_eligible = full_content.is_empty();
                        return Err(OmonError::Llm(
                            "omo turn ended with no content (likely upstream LLM failure)".into(),
                        ));
                    }
                    self.active_turns.lock().remove(&session.key.storage_key());

                    let rendered = if scrubbed_content.trim().is_empty() {
                        "✅ Done.".to_string()
                    } else {
                        scrubbed_content.clone()
                    };
                    session
                        .state
                        .metadata
                        .insert("cron_agent_output".into(), json!(rendered.clone()));

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
                            .record_obligation(&obl_id, &session.key, &rendered)
                            .await?;
                        ledger.mark_obligation_attempting(&obl_id).await?;
                    }

                    let delivered = self
                        .emit_chunk(
                            session,
                            stream_id,
                            sequence,
                            rendered.clone(),
                            true,
                            reply_to.clone(),
                        )
                        .await;
                    if let Some(pool) = &self.pool {
                        persist_message(pool, session, &rendered).await?;
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
        let effective_total_timeout = if is_cron_session {
            self.config.total_timeout
        } else {
            self.config.total_timeout.min(INTERACTIVE_TURN_CAP)
        };
        let deadline = tokio::time::Instant::now() + effective_total_timeout;

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
                    self.setup_turn(session, deadline, effective_total_timeout)
                        .await?
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
        let result = self
            .execute_turn(
                ws,
                thread_id,
                session,
                event.clone(),
                deadline,
                effective_total_timeout,
                None,
                &mut fallback_eligible,
            )
            .await;
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
            return self
                .execute_turn(
                    ws,
                    thread_id,
                    session,
                    event,
                    deadline,
                    effective_total_timeout,
                    Some(fallback_model.as_str()),
                    &mut fallback_eligible,
                )
                .await;
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

async fn persist_message(pool: &SqlitePool, session: &SessionContext, content: &str) -> Result<()> {
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(session.key.storage_key())
    .bind("assistant")
    .bind(content)
    .bind("{}")
    .bind(now)
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
}

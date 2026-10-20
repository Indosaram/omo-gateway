//! Delivers turns the engine starts on its own after a gateway turn ends.
//!
//! A detached eval cell finishing, a monitor firing or a todo backstop wakes the agent
//! without a user message. The app-server routes those frames only to connections
//! subscribed to the thread, and the gateway's per-turn connection is already closed,
//! so the reply reached the session transcript but never the messenger (incident
//! 2026-10-18: "끝나면 '끝'이라고만 답할게요" was answered in the transcript only).
//! After each interactive turn the gateway keeps one subscription per session open and
//! posts the final message of every engine-started turn.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{
    approval_allow_response, approval_denial_with_reason, is_approval_request,
    thread_resume_request, OmoBackend,
};
use crate::models::{filter_reasoning, is_explicit_silence, SessionContext};
use crate::{OmonError, Result};

/// Long enough for background cells and monitors the agent arms for an owner-side condition.
const FOLLOW_WINDOW: Duration = Duration::from_secs(6 * 3600);
/// Terminal frames queued while nobody was subscribed are replayed on subscription; only
/// ones that completed this close to the follower start are new.
const REPLAY_TOLERANCE_SECS: f64 = 30.0;

static FOLLOWER_GENERATION: AtomicU64 = AtomicU64::new(0);

pub(super) struct Follower {
    generation: u64,
    token: CancellationToken,
}

fn final_agent_text(turn: &Value) -> Option<&str> {
    turn.get("items")?
        .as_array()?
        .iter()
        .rev()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
        .and_then(|item| item.get("text").and_then(Value::as_str))
}

fn frame_turn_id(frame: &Value) -> Option<&str> {
    frame
        .pointer("/params/turnId")
        .or_else(|| frame.pointer("/params/turn/id"))
        .and_then(Value::as_str)
}

fn agent_message_text(frame: &Value) -> Option<&str> {
    (frame.pointer("/params/item/type").and_then(Value::as_str) == Some("agentMessage"))
        .then(|| frame.pointer("/params/item/text").and_then(Value::as_str))
        .flatten()
}

impl OmoBackend {
    /// Stops a session's follower. An engine-started turn already in flight is still
    /// delivered; the follower exits once it completes.
    pub(super) fn stop_following(&self, storage_key: &str) {
        if let Some(follower) = self.followers.lock().remove(storage_key) {
            follower.token.cancel();
        }
    }

    pub(super) fn follow_thread(&self, session: &SessionContext, thread_id: String) {
        let storage_key = session.key.storage_key();
        let generation = FOLLOWER_GENERATION.fetch_add(1, Ordering::Relaxed);
        let token = CancellationToken::new();
        let follower = Follower {
            generation,
            token: token.clone(),
        };
        if let Some(previous) = self.followers.lock().insert(storage_key.clone(), follower) {
            previous.token.cancel();
        }
        let backend = self.clone();
        let mut session = session.clone();
        tokio::spawn(async move {
            let started = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
            if let Err(error) = backend
                .run_follower(&mut session, &thread_id, token, started)
                .await
            {
                tracing::warn!(session = %session.key, %error, "engine-turn follower ended");
            }
            let mut followers = backend.followers.lock();
            if followers
                .get(&storage_key)
                .is_some_and(|current| current.generation == generation)
            {
                followers.remove(&storage_key);
            }
        });
    }

    async fn run_follower(
        &self,
        session: &mut SessionContext,
        thread_id: &str,
        token: CancellationToken,
        started: f64,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + FOLLOW_WINDOW;
        let setup_deadline = tokio::time::Instant::now() + self.config.request_timeout * 2;
        let mut ws = self.connect_ws(setup_deadline).await?;
        self.do_initialize(&mut ws, setup_deadline).await?;
        ws.send(thread_resume_request(thread_id))
            .await
            .map_err(|e| OmonError::Llm(format!("follower thread/resume send failed: {e}")))?;

        let storage_key = session.key.storage_key();
        let mut stopping = false;
        let mut current: Option<String> = None;
        let mut latest_text = String::new();
        let mut last_delivered: Option<String> = None;
        loop {
            let frame = tokio::select! {
                _ = token.cancelled(), if !stopping => {
                    if current.is_none() {
                        return Ok(());
                    }
                    stopping = true;
                    continue;
                }
                _ = tokio::time::sleep_until(deadline) => return Ok(()),
                frame = ws.next() => frame,
            };
            let Some(frame) = frame else {
                return Ok(());
            };
            let Message::Text(text) =
                frame.map_err(|e| OmonError::Llm(format!("follower transport error: {e}")))?
            else {
                continue;
            };
            let Ok(val) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            if val.get("id").and_then(Value::as_u64) == Some(2) && val.get("method").is_none() {
                if let Some(error) = val.get("error") {
                    return Err(OmonError::Llm(format!("follower thread/resume: {error}")));
                }
                continue;
            }
            let gateway_turn_running = self.active_turns.lock().contains_key(&storage_key);
            let method = val.get("method").and_then(Value::as_str).unwrap_or("");

            if let (Some(req_id), true) = (val.get("id"), is_approval_request(method)) {
                // A running gateway turn answers its own approvals on its connection.
                if !gateway_turn_running {
                    let response = self.answer_unattended_approval(session, &val, req_id).await;
                    let _ = ws.send(response).await;
                }
                continue;
            }
            if val.pointer("/params/threadId").and_then(Value::as_str) != Some(thread_id) {
                continue;
            }
            let turn = frame_turn_id(&val).map(str::to_owned);
            match method {
                "turn/started" if current.is_none() => {
                    if gateway_turn_running {
                        return Ok(());
                    }
                    current = turn;
                    latest_text.clear();
                }
                "item/completed" => {
                    if current.is_none() && !gateway_turn_running {
                        current = turn.clone();
                    }
                    if current.is_some() && turn == current {
                        if let Some(text) = agent_message_text(&val) {
                            latest_text = text.to_string();
                        }
                    }
                }
                "turn/completed" => {
                    let followed = current.is_some() && turn == current;
                    let fresh_replay = current.is_none()
                        && !gateway_turn_running
                        && val
                            .pointer("/params/turn/completedAt")
                            .and_then(Value::as_f64)
                            .is_some_and(|at| at + REPLAY_TOLERANCE_SECS >= started);
                    if !followed && !fresh_replay {
                        continue;
                    }
                    let text = val
                        .pointer("/params/turn")
                        .and_then(final_agent_text)
                        .map(str::to_owned)
                        .unwrap_or_else(|| std::mem::take(&mut latest_text));
                    current = None;
                    latest_text.clear();
                    let rendered = filter_reasoning(&text);
                    // While stopping, a gateway turn is waiting on this thread and its pre-ack
                    // relay already delivers every completed message of this engine turn.
                    if !stopping
                        && val.pointer("/params/turn/status").and_then(Value::as_str)
                            == Some("completed")
                        && !rendered.trim().is_empty()
                        && !is_explicit_silence(&rendered)
                        && last_delivered.as_deref() != Some(rendered.as_str())
                    {
                        tracing::info!(
                            session = %session.key,
                            turn = ?turn,
                            "delivering a turn the engine started after the gateway turn ended"
                        );
                        self.deliver_rendered_result(session, Uuid::new_v4(), 0, &rendered, None)
                            .await?;
                        last_delivered = Some(rendered);
                    }
                    if stopping {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    /// Nobody is in the conversation to click a card: allow what the owner's grant covers
    /// (payments never), decline the rest with a reason the agent can report.
    async fn answer_unattended_approval(
        &self,
        session: &SessionContext,
        frame: &Value,
        req_id: &Value,
    ) -> Message {
        let unrestricted = session.state.yolo || self.is_full_access();
        let ws_cwd = session
            .state
            .metadata
            .get("workspace_cwd")
            .and_then(Value::as_str);
        let classification = crate::autonomy::classify_frame(frame, ws_cwd);
        if !self.is_ledger_enabled() {
            return if unrestricted && classification.effect != crate::autonomy::Effect::Spend {
                approval_allow_response(req_id)
            } else {
                approval_denial_with_reason(
                    req_id,
                    "this needs the owner's approval and nobody is in the conversation right now",
                )
            };
        }
        let Some(pool) = &self.pool else {
            return approval_denial_with_reason(
                req_id,
                "the gateway cannot record actions for this session",
            );
        };
        let session_key = session.key.storage_key();
        let payload = frame.to_string();
        let outcome = crate::autonomy::evaluate_pre_ask(
            pool,
            crate::autonomy::PreAskInput {
                origin: crate::autonomy::ProposalOrigin::EngineApproval,
                session_key: &session_key,
                lead_session_key: &session_key,
                classification: &classification,
                payload_json: &payload,
                tainted: true,
                yolo: unrestricted,
                always_mode: false,
                now: self.now_utc(),
            },
        )
        .await;
        match outcome {
            Ok(crate::autonomy::PreAskOutcome::Allow { .. }) => approval_allow_response(req_id),
            Ok(crate::autonomy::PreAskOutcome::Deny { reason, .. }) => {
                approval_denial_with_reason(req_id, &reason)
            }
            Ok(crate::autonomy::PreAskOutcome::Ask { proposal_id }) => {
                let _ = crate::autonomy::update_proposal_decision(
                    pool,
                    proposal_id,
                    crate::autonomy::ProposalState::Expired,
                    None,
                    None,
                )
                .await;
                approval_denial_with_reason(
                    req_id,
                    &format!(
                        "this needs the owner's approval and nobody is in the conversation right now; mention pending proposal #{proposal_id} in your report"
                    ),
                )
            }
            Err(error) => {
                tracing::warn!(session = %session.key, %error, "follower pre-ask failed; declining");
                approval_denial_with_reason(
                    req_id,
                    "the gateway could not record this action; retry it shortly",
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::final_agent_text;

    #[test]
    fn final_agent_text_takes_the_last_agent_message() {
        let turn = json!({"items": [
            {"type": "agentMessage", "text": "narration"},
            {"type": "commandExecution", "command": "ls"},
            {"type": "agentMessage", "text": "끝"},
            {"type": "reasoning", "text": "hidden"},
        ]});
        assert_eq!(final_agent_text(&turn), Some("끝"));
        assert_eq!(final_agent_text(&json!({"items": []})), None);
        assert_eq!(final_agent_text(&json!({})), None);
    }
}

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures_util::{SinkExt, StreamExt};
use omon_gateway::agent::{AgentBackend, OmoBackend, OmoBackendConfig};
use omon_gateway::autonomy::{
    get_proposal, insert_proposal, recover_pending_proposals_on_startup, try_resolve_short_reply,
    Effect, NewProposal, ProposalOrigin, ProposalState,
};
use omon_gateway::lanes::runner::CapturingDispatcher;
use omon_gateway::slack::interactive::handle_block_actions;
use omon_gateway::slack::{AllowBots, SlackApi, SlackConfig, SlackEgress};
use omon_gateway::storage::{init_pool, take_undelivered_events_at};
use omon_gateway::{
    ApprovalDecision, DiscordApprovalRequester, InboundEvent, OutboundAction, OutboundDispatcher,
    Result, SessionContext, SessionKey, SmartApprovalGuard,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Default)]
struct RecordingDispatcher {
    actions: Arc<Mutex<Vec<OutboundAction>>>,
    approval_notified: Arc<Notify>,
}

#[async_trait]
impl OutboundDispatcher for RecordingDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        let is_approval = matches!(action, OutboundAction::ApprovalRequest { .. });
        self.actions.lock().push(action);
        if is_approval {
            self.approval_notified.notify_waiters();
        }
        Ok(())
    }
}

impl RecordingDispatcher {
    fn last_approval_request_id(&self) -> Option<Uuid> {
        self.actions
            .lock()
            .iter()
            .rev()
            .find_map(|action| match action {
                OutboundAction::ApprovalRequest { request_id, .. } => Some(*request_id),
                _ => None,
            })
    }
}

fn daytime_clock() -> chrono::DateTime<Utc> {
    chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 14, 0, 0)
        .unwrap()
        .with_timezone(&Utc)
}

fn quiet_hour_clock() -> chrono::DateTime<Utc> {
    chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 3, 0, 0)
        .unwrap()
        .with_timezone(&Utc)
}

#[tokio::test]
async fn a_owner_once_within_thirty_minutes_sends_allow_for_req_id() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let dispatcher = RecordingDispatcher::default();
    let requester = Arc::new(DiscordApprovalRequester::new(
        guard.clone(),
        Duration::from_secs(1800),
    ));
    requester.set_dispatcher(Arc::new(dispatcher.clone())).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<Value>();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let v: Value = serde_json::from_str(&text).unwrap();
            let id = v.get("id").and_then(Value::as_u64);
            let method = v.get("method").and_then(Value::as_str).unwrap_or("");
            match (id, method) {
                (Some(1), "initialize") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":1,"result":{}}).to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(2), "thread/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":2,"result":{"thread":{"id":"th-1"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(3), "turn/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"turn-1"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "id": 42,
                            "method": "item/commandExecution/requestApproval",
                            "params": {
                                "threadId": "th-1",
                                "turnId": "turn-1",
                                "itemId": "approval-bash",
                                "command": "git push origin lane/qa-a",
                                "reason": "Command: $ git push origin lane/qa-a"
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                _ if v.get("id").and_then(Value::as_u64) == Some(42) => {
                    let _ = resp_tx.send(v);
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "item/completed",
                            "params": {
                                "threadId": "th-1",
                                "turnId": "turn-1",
                                "item": { "id": "msg-1", "type": "agentMessage", "text": "pushed" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "turn/completed",
                            "params": {
                                "threadId": "th-1",
                                "turn": { "id": "turn-1", "status": "completed" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    break;
                }
                _ => {}
            }
        }
    });

    let disp_wait = dispatcher.clone();
    let guard_wait = guard.clone();
    tokio::spawn(async move {
        disp_wait.approval_notified.notified().await;
        let req_id = disp_wait.last_approval_request_id().unwrap();
        guard_wait
            .resolve_custom_id(&format!("omon:approval:{req_id}:once"))
            .await;
    });

    let config = OmoBackendConfig::new(format!("ws://{addr}"));
    let backend = OmoBackend::new(config, Arc::new(dispatcher.clone()))
        .with_pool(pool.clone())
        .with_approval_requester(requester)
        .with_ledger_enabled(true)
        .with_clock(daytime_clock);

    let key = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");
    let mut session = SessionContext::new(key.clone());
    let event = InboundEvent::message(key, "m1", "push branch");
    backend
        .run_cancelable(&mut session, event, CancellationToken::new())
        .await
        .unwrap();

    let answer = resp_rx.await.unwrap();
    assert_eq!(answer.get("id").and_then(Value::as_u64), Some(42));
    assert_eq!(
        answer.pointer("/result/decision").and_then(Value::as_str),
        Some("accept")
    );

    let proposal = get_proposal(&pool, 1).await.unwrap().unwrap();
    assert_eq!(proposal.state, "approved");
}

#[tokio::test]
async fn b_timeout_sends_decline_sets_expired_and_leaves_denial_counter_unchanged() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let dispatcher = RecordingDispatcher::default();
    let requester = Arc::new(DiscordApprovalRequester::new(
        guard.clone(),
        Duration::from_millis(20),
    ));
    requester.set_dispatcher(Arc::new(dispatcher.clone())).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (resp_tx, mut resp_rx) = tokio::sync::mpsc::unbounded_channel::<Value>();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let mut req_idx = 0u64;

        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let v: Value = serde_json::from_str(&text).unwrap();
            let id = v.get("id").and_then(Value::as_u64);
            let method = v.get("method").and_then(Value::as_str).unwrap_or("");
            match (id, method) {
                (Some(1), "initialize") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":1,"result":{}}).to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(2), "thread/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":2,"result":{"thread":{"id":"th-b"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(3), "turn/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"turn-b"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                    req_idx = 1;
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "id": 101,
                            "method": "item/commandExecution/requestApproval",
                            "params": {
                                "threadId": "th-b",
                                "turnId": "turn-b",
                                "command": format!("git push origin lane/timeout-{req_idx}"),
                                "reason": "push"
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                _ if v
                    .get("id")
                    .and_then(Value::as_u64)
                    .is_some_and(|i| (101..=105).contains(&i)) =>
                {
                    let _ = resp_tx.send(v);
                    if req_idx < 5 {
                        req_idx += 1;
                        ws.send(Message::text(
                            json!({
                                "jsonrpc": "2.0",
                                "id": 100 + req_idx,
                                "method": "item/commandExecution/requestApproval",
                                "params": {
                                    "threadId": "th-b",
                                    "turnId": "turn-b",
                                    "command": format!("git push origin lane/timeout-{req_idx}"),
                                    "reason": "push"
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .unwrap();
                    } else {
                        ws.send(Message::text(
                            json!({
                                "jsonrpc": "2.0",
                                "method": "item/completed",
                                "params": {
                                    "threadId": "th-b",
                                    "turnId": "turn-b",
                                    "item": { "id": "m-b", "type": "agentMessage", "text": "carried on after timeouts" }
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .unwrap();
                        ws.send(Message::text(
                            json!({
                                "jsonrpc": "2.0",
                                "method": "turn/completed",
                                "params": {
                                    "threadId": "th-b",
                                    "turn": { "id": "turn-b", "status": "completed" }
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .unwrap();
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    let config = OmoBackendConfig::new(format!("ws://{addr}"));
    let backend = OmoBackend::new(config, Arc::new(dispatcher.clone()))
        .with_pool(pool.clone())
        .with_approval_requester(requester)
        .with_ledger_enabled(true)
        .with_clock(daytime_clock);

    let key = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");
    let mut session = SessionContext::new(key.clone());
    let event = InboundEvent::message(key, "mb", "test timeout");
    backend
        .run_cancelable(&mut session, event, CancellationToken::new())
        .await
        .expect("5 expired timeouts must NOT trigger APPROVAL_DENIAL_TURN_LIMIT abort");

    let first = resp_rx.recv().await.unwrap();
    assert_eq!(
        first.pointer("/result/decision").and_then(Value::as_str),
        Some("decline")
    );
    let reason = first
        .pointer("/result/reason")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        reason.contains("owner has not answered yet; continue with other work and mention pending proposal #1 in your report"),
        "unexpected reason: {reason}"
    );

    let p1 = get_proposal(&pool, 1).await.unwrap().unwrap();
    assert_eq!(p1.state, "expired");
}

#[tokio::test]
async fn c_no_inactivity_interrupt_fires_during_twenty_minute_wait() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let dispatcher = RecordingDispatcher::default();
    let requester = Arc::new(DiscordApprovalRequester::new(
        guard.clone(),
        Duration::from_secs(1800),
    ));
    requester.set_dispatcher(Arc::new(dispatcher.clone())).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let interrupts = Arc::new(Mutex::new(Vec::<Value>::new()));
    let interrupts_srv = interrupts.clone();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let v: Value = serde_json::from_str(&text).unwrap();
            let id = v.get("id").and_then(Value::as_u64);
            let method = v.get("method").and_then(Value::as_str).unwrap_or("");
            if method == "turn/interrupt" {
                interrupts_srv.lock().push(v);
                continue;
            }
            match (id, method) {
                (Some(1), "initialize") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":1,"result":{}}).to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(2), "thread/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":2,"result":{"thread":{"id":"th-c"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(3), "turn/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"turn-c"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "id": 77,
                            "method": "item/commandExecution/requestApproval",
                            "params": {
                                "threadId": "th-c",
                                "turnId": "turn-c",
                                "command": "git push origin lane/wait-20m",
                                "reason": "push"
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                _ if v.get("id").and_then(Value::as_u64) == Some(77) => {
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "item/completed",
                            "params": {
                                "threadId": "th-c",
                                "turnId": "turn-c",
                                "item": { "id": "m-c", "type": "agentMessage", "text": "done after 20m wait" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "turn/completed",
                            "params": {
                                "threadId": "th-c",
                                "turn": { "id": "turn-c", "status": "completed" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    break;
                }
                _ => {}
            }
        }
    });

    let disp_wait = dispatcher.clone();
    let guard_wait = guard.clone();
    tokio::spawn(async move {
        disp_wait.approval_notified.notified().await;
        tokio::time::sleep(Duration::from_millis(180)).await;
        let req_id = disp_wait.last_approval_request_id().unwrap();
        guard_wait
            .resolve_custom_id(&format!("omon:approval:{req_id}:once"))
            .await;
    });

    let mut config = OmoBackendConfig::new(format!("ws://{addr}"));
    config.request_timeout = Duration::from_millis(60);
    let backend = OmoBackend::new(config, Arc::new(dispatcher.clone()))
        .with_pool(pool.clone())
        .with_approval_requester(requester)
        .with_ledger_enabled(true)
        .with_clock(daytime_clock);

    let key = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");
    let mut session = SessionContext::new(key.clone());
    let event = InboundEvent::message(key, "mc", "wait 20m");
    backend
        .run_cancelable(&mut session, event, CancellationToken::new())
        .await
        .expect("turn must succeed after 20m approval wait");

    assert!(
        interrupts.lock().is_empty(),
        "no turn/interrupt may fire during a 20m approval wait"
    );
}

#[tokio::test]
async fn d_quiet_hour_path_declines_and_sets_release_at() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let dispatcher = RecordingDispatcher::default();
    let requester = Arc::new(DiscordApprovalRequester::new(
        guard,
        Duration::from_secs(1800),
    ));
    requester.set_dispatcher(Arc::new(dispatcher.clone())).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel::<Value>();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let v: Value = serde_json::from_str(&text).unwrap();
            let id = v.get("id").and_then(Value::as_u64);
            let method = v.get("method").and_then(Value::as_str).unwrap_or("");
            match (id, method) {
                (Some(1), "initialize") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":1,"result":{}}).to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(2), "thread/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":2,"result":{"thread":{"id":"th-d"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                (Some(3), "turn/start") => {
                    ws.send(Message::text(
                        json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"turn-d"}}})
                            .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "id": 88,
                            "method": "item/commandExecution/requestApproval",
                            "params": {
                                "threadId": "th-d",
                                "turnId": "turn-d",
                                "command": "git push origin lane/quiet",
                                "reason": "push"
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                }
                _ if v.get("id").and_then(Value::as_u64) == Some(88) => {
                    let _ = resp_tx.send(v);
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "item/completed",
                            "params": {
                                "threadId": "th-d",
                                "turnId": "turn-d",
                                "item": { "id": "m-d", "type": "agentMessage", "text": "held for 08:00" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    ws.send(Message::text(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "turn/completed",
                            "params": {
                                "threadId": "th-d",
                                "turn": { "id": "turn-d", "status": "completed" }
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .unwrap();
                    break;
                }
                _ => {}
            }
        }
    });

    let config = OmoBackendConfig::new(format!("ws://{addr}"));
    let backend = OmoBackend::new(config, Arc::new(dispatcher.clone()))
        .with_pool(pool.clone())
        .with_approval_requester(requester)
        .with_ledger_enabled(true)
        .with_clock(quiet_hour_clock);

    let key = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");
    let mut session = SessionContext::new(key.clone());
    let event = InboundEvent::message(key, "md", "quiet push");
    backend
        .run_cancelable(&mut session, event, CancellationToken::new())
        .await
        .unwrap();

    let answer = resp_rx.await.unwrap();
    assert_eq!(
        answer.pointer("/result/decision").and_then(Value::as_str),
        Some("decline")
    );
    let reason = answer
        .pointer("/result/reason")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(reason, "quiet hours; proposal #1 will be asked at 08:00");

    let due_at_03 = take_undelivered_events_at(&pool, None, quiet_hour_clock())
        .await
        .unwrap();
    assert!(
        due_at_03.is_empty(),
        "event with future release_at must be skipped before 08:00 KST"
    );

    let after_08 = chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 8, 1, 0)
        .unwrap()
        .with_timezone(&Utc);
    let due_after_08 = take_undelivered_events_at(&pool, None, after_08)
        .await
        .unwrap();
    assert_eq!(due_after_08.len(), 1);
    assert_eq!(due_after_08[0].kind, "action_loop");
}

#[tokio::test]
async fn e_synthetic_block_actions_from_allowed_non_owner_is_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let app = axum::Router::new()
            .fallback(|| async { axum::Json(json!({"ok": true, "ts": "1700000000.0001"})) });
        axum::serve(listener, app).await.unwrap();
    });

    let api = SlackApi::with_base_url("xoxb-test".into(), format!("http://{addr}"));
    let egress = SlackEgress::new(api.clone());
    let owner = omon_gateway::slack::owner_id();
    let non_owner = "U999NONOWNER";
    let config = SlackConfig {
        bot_token: "xoxb-test".into(),
        app_token: "xapp-test".into(),
        allowed_users: vec![owner.clone(), non_owner.to_string()],
        free_response_channels: Vec::new(),
        auto_thread: false,
        attachment_dir: std::env::temp_dir(),
        home_channel: None,
        approval_mentions: false,
        allowed_channels: Vec::new(),
        ignored_channels: Vec::new(),
        allow_bots: AllowBots::None,
        thread_require_mention: false,
        channel_context: false,
        channel_context_limit: 10,
        channel_topic_context: false,
        processing_reactions: false,
        runtime_footer: false,
        missed_backfill: false,
        destructive_slash_confirm: false,
        allow_all_users: true,
    };
    assert!(config.is_allowed(non_owner));

    let guard = SmartApprovalGuard::new();
    let prompt = guard.request().await;
    let req_id = prompt.request_id;

    let payload = json!({
        "type": "block_actions",
        "user": { "id": non_owner },
        "channel": { "id": "C123" },
        "actions": [
            { "value": format!("omon:approval:{req_id}:once") }
        ]
    });

    handle_block_actions(&payload, &config, &guard, &egress, &api)
        .await
        .unwrap();

    assert_eq!(
        guard.pending_count().await,
        1,
        "allowed non-owner click must be rejected and leave the request pending"
    );

    let owner_payload = json!({
        "type": "block_actions",
        "user": { "id": owner },
        "channel": { "id": "C123" },
        "actions": [
            { "value": format!("omon:approval:{req_id}:once") }
        ]
    });
    handle_block_actions(&owner_payload, &config, &guard, &egress, &api)
        .await
        .unwrap();
    assert_eq!(guard.pending_count().await, 0);
}

#[tokio::test]
async fn f_short_reply_with_one_pending_resolves_once_and_with_two_pending_routes_normal_text() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let guard = SmartApprovalGuard::new().with_pool(pool.clone());
    let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");

    let p1_id = insert_proposal(
        &pool,
        &NewProposal {
            origin: ProposalOrigin::EngineApproval,
            session_key: session.storage_key(),
            signature: "vcs_remote:git push:/repo".into(),
            effect: Effect::VcsRemote,
            target: "/repo".into(),
            summary: "git push origin lane/1".into(),
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
    let prompt_1 = guard
        .request_with_session_and_proposal(Some(session.clone()), Some(p1_id))
        .await;

    let resolved_one = try_resolve_short_reply(Some(&pool), Some(&guard), &session, "응")
        .await
        .unwrap();
    assert!(resolved_one, "'응' with 1 pending proposal must resolve");
    let dec = prompt_1.wait(Duration::from_millis(50)).await.unwrap();
    assert_eq!(dec, ApprovalDecision::Once);
    omon_gateway::autonomy::update_proposal_decision(
        &pool,
        p1_id,
        ProposalState::Approved,
        None,
        None,
    )
    .await
    .unwrap();

    let p2_id = insert_proposal(
        &pool,
        &NewProposal {
            origin: ProposalOrigin::EngineApproval,
            session_key: session.storage_key(),
            signature: "vcs_remote:git push:/repo2".into(),
            effect: Effect::VcsRemote,
            target: "/repo2".into(),
            summary: "git push origin lane/2".into(),
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
    let p3_id = insert_proposal(
        &pool,
        &NewProposal {
            origin: ProposalOrigin::EngineApproval,
            session_key: session.storage_key(),
            signature: "vcs_remote:git push:/repo3".into(),
            effect: Effect::VcsRemote,
            target: "/repo3".into(),
            summary: "git push origin lane/3".into(),
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
    let _prompt_2 = guard
        .request_with_session_and_proposal(Some(session.clone()), Some(p2_id))
        .await;
    let _prompt_3 = guard
        .request_with_session_and_proposal(Some(session.clone()), Some(p3_id))
        .await;

    let resolved_two = try_resolve_short_reply(Some(&pool), Some(&guard), &session, "응")
        .await
        .unwrap();
    assert!(
        !resolved_two,
        "'응' with 2 pending proposals must NOT resolve and must route as normal text"
    );
}

#[tokio::test]
async fn g_restart_turns_pending_rows_into_expired_and_emits_one_loop_event() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let lead = omon_gateway::SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1")
        .storage_key();
    let lead = lead.as_str();

    for i in 1..=2 {
        insert_proposal(
            &pool,
            &NewProposal {
                origin: ProposalOrigin::EngineApproval,
                // An inbox proposal stores the bare channel id, which is not a session key.
                session_key: if i == 1 {
                    lead.to_string()
                } else {
                    "D0C9HNVUK5M".into()
                },
                signature: format!("vcs_remote:git push:/repo-{i}"),
                effect: Effect::VcsRemote,
                target: format!("/repo-{i}"),
                summary: format!("git push origin lane/{i}"),
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
    }

    let count = recover_pending_proposals_on_startup(&pool, lead)
        .await
        .unwrap();
    assert_eq!(count, 2);

    let pending_left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM action_proposals WHERE state = 'pending'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pending_left, 0);

    let expired_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM action_proposals WHERE state = 'expired'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(expired_count, 2);

    let events = take_undelivered_events_at(&pool, None, Utc::now())
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "action_loop");
    assert!(events[0].body.contains("[ACTION_LOOP]"));
    assert_eq!(events[0].lead_session_key, lead);
}

#[tokio::test]
async fn h_lane_native_approval_request_no_longer_fails_the_lane() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let mut lane = omon_gateway::storage::Lane::new(
        "lane-h",
        "test",
        "slack:T1:C1::U1",
        "/tmp",
        "/tmp/brief.md",
        "lane:lane-h",
        "[]",
    );
    lane.status = "working".into();
    omon_gateway::storage::insert_lane(&pool, &lane)
        .await
        .unwrap();

    let owner_disp = RecordingDispatcher::default();
    let capture = CapturingDispatcher::new(pool.clone())
        .with_approval_dispatcher(Arc::new(owner_disp.clone()));
    let lane_sess = SessionKey::new(
        "lane",
        None::<String>,
        "lane-h",
        None::<String>,
        "lane:lane-h",
    );

    capture
        .dispatch(OutboundAction::ApprovalRequest {
            session: lane_sess,
            request_id: Uuid::new_v4(),
            command: "git push origin lane/lane-h".into(),
            reason: "lane push".into(),
        })
        .await
        .expect("lane native ApprovalRequest must succeed instead of failing the lane");

    assert!(owner_disp.last_approval_request_id().is_some());
}

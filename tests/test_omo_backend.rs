use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use omon_gateway::{
    AgentBackend, Database, InboundEvent, MultiplexerConfig, OmoBackend, OmoBackendConfig,
    OmonError, OutboundAction, OutboundDispatcher, SessionContext, SessionKey, SessionMultiplexer,
    StreamChunk,
};
use parking_lot::Mutex as ParkingMutex;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

// Public backend entry point over a real loopback WebSocket. Each script is a
// new subscription to r1, so completed t1 frames can replay while t2 is active.
async fn run_correlated_frames(frames: Vec<Value>) -> (omon_gateway::Result<()>, Vec<StreamChunk>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        for method in ["initialize", "thread/resume", "turn/start"] {
            let message = ws.next().await.unwrap().unwrap();
            let request: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(request["method"], method);
            if method != "initialize" {
                assert_eq!(request["params"]["threadId"], "r1");
            }
            if method != "turn/start" {
                ws.send(Message::text(
                    json!({"jsonrpc":"2.0", "id":request["id"],
                    "result":{"thread":{"id":"r1"}}})
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }
        for frame in frames {
            ws.feed(Message::text(frame.to_string())).await.unwrap();
        }
        ws.flush().await.unwrap();
        stop_rx.await.unwrap();
    });
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        dispatcher.clone(),
    );
    let key = SessionKey::new("local", None::<String>, "u12", None::<String>, "user");
    let mut session = SessionContext::new(key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        backend.run(&mut session, InboundEvent::message(key, "u12", "probe")),
    )
    .await;
    stop_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
    (
        result.expect("bounded backend completion"),
        dispatcher.stream_chunks(),
    )
}

fn correlation_ack() -> Value {
    json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"t2","status":"inProgress"}}})
}

#[tokio::test]
async fn model_rejected_failure_triggers_fallback_to_deepseek_and_persists_marker() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        // Attempt 0: Primary model fails turn/start with "model not found"
        let (socket1, _) = listener.accept().await.unwrap();
        let mut ws1 = tokio_tungstenite::accept_async(socket1).await.unwrap();
        for method in ["initialize", "thread/resume", "turn/start"] {
            let msg = ws1.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);
            if method == "turn/start" {
                assert_eq!(req["params"]["model"], "custom/quota-dead-model");
                ws1.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "error": { "code": -32603, "message": "model not found: custom/quota-dead-model" }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            } else {
                ws1.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "r1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }

        // Attempt 1: Fallback re-setup + execute_turn with default fallback model
        let (socket2, _) = listener.accept().await.unwrap();
        let mut ws2 = tokio_tungstenite::accept_async(socket2).await.unwrap();
        for method in [
            "initialize",
            "thread/resume",
            "thread/settings/update",
            "turn/start",
        ] {
            let msg = ws2.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);
            if method == "thread/settings/update" {
                assert_eq!(req["params"]["model"], "inferhub/cb/deepseek-v4.1-flash");
            }
            if method == "turn/start" {
                assert_eq!(req["params"]["model"], "inferhub/cb/deepseek-v4.1-flash");
                ws2.send(Message::text(correlation_ack().to_string()))
                    .await
                    .unwrap();
            } else {
                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "r1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }
        ws2.send(Message::text(
            correlation_delta("r1", "t2", "Fallback OK").to_string(),
        ))
        .await
        .unwrap();
        ws2.send(Message::text(
            correlation_terminal("t2", "completed").to_string(),
        ))
        .await
        .unwrap();
    });

    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        dispatcher.clone(),
    );
    let key = SessionKey::new(
        "local",
        None::<String>,
        "fallback-session",
        None::<String>,
        "user",
    );
    let mut session = SessionContext::new(key.clone());
    session.state.active_model = Some("custom/quota-dead-model".into());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        backend.run(
            &mut session,
            InboundEvent::message(key.clone(), "msg", "test"),
        ),
    )
    .await
    .unwrap();

    assert!(
        result.is_ok(),
        "turn must succeed via fallback: {:?}",
        result.err()
    );
    assert_eq!(
        session
            .state
            .metadata
            .get("omo_fallback_model_active")
            .and_then(Value::as_bool),
        Some(true),
        "persisted fallback marker must be active after fallback execution"
    );
    // Primary active_model remains unchanged in session state
    assert_eq!(
        session.state.active_model.as_deref(),
        Some("custom/quota-dead-model")
    );
    assert!(backend.active_turns.lock().is_empty());
    peer.await.unwrap();

    // Simulated restart with persisted marker: Next turn uses fallback settings directly
    let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address2 = listener2.local_addr().unwrap();
    let peer2 = tokio::spawn(async move {
        let (socket, _) = listener2.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        for method in [
            "initialize",
            "thread/resume",
            "thread/settings/update",
            "turn/start",
        ] {
            let msg = ws.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);
            if method == "thread/settings/update" {
                assert_eq!(req["params"]["model"], "custom/quota-dead-model");
            }
            if method == "turn/start" {
                assert_eq!(req["params"]["model"], "custom/quota-dead-model");
                ws.send(Message::text(correlation_ack().to_string()))
                    .await
                    .unwrap();
            } else {
                ws.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "r1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }
        ws.send(Message::text(
            correlation_delta("r1", "t2", "Restarted OK").to_string(),
        ))
        .await
        .unwrap();
        ws.send(Message::text(
            correlation_terminal("t2", "completed").to_string(),
        ))
        .await
        .unwrap();
    });

    let backend2 = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address2}")),
        dispatcher.clone(),
    );
    let result2 = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        backend2.run(
            &mut session,
            InboundEvent::message(key, "msg2", "next turn"),
        ),
    )
    .await
    .unwrap();

    assert!(
        result2.is_ok(),
        "restarted turn must succeed: {:?}",
        result2.err()
    );
    peer2.await.unwrap();
}

#[tokio::test]
async fn fallback_one_shot_guard_never_refalls_back_from_fallback() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        // Attempt 0: Primary model IS the fallback model, and it fails.
        // It must NOT trigger another fallback!
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        for method in ["initialize", "thread/resume", "turn/start"] {
            let msg = ws.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);
            if method == "turn/start" {
                assert_eq!(req["params"]["model"], "inferhub/cb/deepseek-v4.1-flash");
                ws.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "error": { "code": -32603, "message": "model not found: deepseek" }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            } else {
                ws.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "r1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        dispatcher.clone(),
    );
    let key = SessionKey::new(
        "local",
        None::<String>,
        "fallback-oneshot",
        None::<String>,
        "user",
    );
    let mut session = SessionContext::new(key.clone());
    session.state.active_model = Some("inferhub/cb/deepseek-v4.1-flash".into());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        backend.run(&mut session, InboundEvent::message(key, "msg", "test")),
    )
    .await
    .unwrap();

    assert!(result.is_err(), "must fail without infinite fallback loop");
    assert!(result.unwrap_err().to_string().contains("model not found"));
    peer.await.unwrap();
}

fn correlation_delta(thread: &str, turn: &str, text: &str) -> Value {
    json!({"jsonrpc":"2.0","method":"item/agentMessage/delta",
        "params":{"threadId":thread,"turnId":turn,"itemId":"m1","delta":text}})
}

fn correlation_terminal(turn: &str, status: &str) -> Value {
    json!({"jsonrpc":"2.0","method":"turn/completed",
        "params":{"threadId":"r1","turn":{"id":turn,"status":status,"error":{"message":"failure-sentinel"}}}})
}

#[tokio::test]
async fn rejects_foreign_and_stale_turn_frames() {
    let (result, chunks) = run_correlated_frames(vec![
        correlation_ack(),
        correlation_delta("r9", "t9", "SECRET"),
        correlation_terminal("t1", "completed"),
        correlation_delta("r1", "t2", "OK"),
        correlation_terminal("t2", "completed"),
    ])
    .await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        chunks
            .iter()
            .filter(|c| c.is_final)
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>(),
        vec!["OK"]
    );
    assert!(chunks.iter().all(|c| !c.content.contains("SECRET")));
}

#[tokio::test]
async fn correlation_requires_ack_and_ignores_subscription_replay() {
    let replay = vec![
        correlation_delta("r1", "t2", "SECRET"),
        correlation_terminal("t2", "completed"),
        correlation_ack(),
        correlation_delta("r1", "t2", "O"),
        json!({"method":"thread/status/changed","params":{"threadId":"r1","status":{"type":"idle"}}}),
        json!({"method":"turn/started","params":{"threadId":"r1","turnId":"t1"}}),
        correlation_delta("r1", "t1", "SECRET"),
        json!({"method":"item/started","params":{"threadId":"r9","turnId":"t9","item":{"id":"tool","type":"commandExecution","command":"SECRET"}}}),
        json!({"method":"item/completed","params":{"threadId":"r1","turnId":"t1","item":{"type":"agentMessage","text":"SECRET"}}}),
        json!({"method":"turn/error","params":{"threadId":"r9","turnId":"t9","message":"SECRET"}}),
        json!({"method":"error","params":{"threadId":"r1","turn":{"id":"t1"},"message":"SECRET"}}),
        json!({"method":"item/agentMessage/delta","params":{"delta":"SECRET"}}),
        json!({"method":"turn/completed","params":{"turn":{"id":"t2","status":"completed"}}}),
        correlation_delta("r1", "t2", "K"),
        correlation_terminal("t2", "completed"),
    ];
    let (result, chunks) = run_correlated_frames(replay).await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        chunks
            .iter()
            .filter(|c| c.is_final)
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>(),
        vec!["OK"]
    );
    assert!(chunks.iter().all(|c| !c.content.contains("SECRET")));
}

#[tokio::test]
async fn correlation_distinguishes_interrupted_and_failed() {
    for status in ["interrupted", "failed"] {
        let (result, chunks) = run_correlated_frames(vec![
            correlation_ack(),
            correlation_delta("r1", "t2", "partial"),
            correlation_terminal("t2", status),
        ])
        .await;
        let error = result.expect_err(status);
        assert!(error.to_string().contains(status), "{error}");
        assert!(!chunks.iter().any(|c| c.is_final));
    }
}

struct CapturingDispatcher {
    actions: ParkingMutex<Vec<OutboundAction>>,
    tool_started: ParkingMutex<Option<tokio::sync::oneshot::Sender<()>>>,
    turn_started_tx: ParkingMutex<Option<tokio::sync::mpsc::UnboundedSender<String>>>,
}

impl CapturingDispatcher {
    fn new() -> Self {
        Self {
            actions: ParkingMutex::new(Vec::new()),
            tool_started: ParkingMutex::new(None),
            turn_started_tx: ParkingMutex::new(None),
        }
    }

    fn stream_chunks(&self) -> Vec<StreamChunk> {
        self.actions
            .lock()
            .iter()
            .filter_map(|action| match action {
                OutboundAction::Stream { chunk, .. } => Some(chunk.clone()),
                _ => None,
            })
            .collect()
    }
}

#[async_trait]
impl OutboundDispatcher for CapturingDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        let tool_started = matches!(&action, OutboundAction::Stream { chunk, .. }
            if !chunk.is_final && chunk.content.ends_with("U12_TOOL_STARTED"));
        if let OutboundAction::Stream { chunk, .. } = &action {
            if let Some(tx) = self.turn_started_tx.lock().as_ref() {
                let _ = tx.send(chunk.content.clone());
            }
        }
        self.actions.lock().push(action);
        if tool_started {
            if let Some(sender) = self.tool_started.lock().take() {
                sender
                    .send(())
                    .expect("tool-start subscriber still waiting");
            }
        }
        Ok(())
    }
}

/// Dispatcher whose Stream deliveries always fail — for proving the ack hook
/// stays silent when delivery did not succeed.
struct FailingStreamDispatcher;

#[async_trait]
impl OutboundDispatcher for FailingStreamDispatcher {
    async fn dispatch(&self, _action: OutboundAction) -> omon_gateway::Result<()> {
        Err(OmonError::Llm("discord send failed".into()))
    }
}

struct FakeAppServer {
    pub port: u16,
    pub thread_start_count: Arc<AtomicUsize>,
    pub thread_resume_count: Arc<AtomicUsize>,
    pub received_developer_instructions: Arc<ParkingMutex<Option<String>>>,
    pub received_model: Arc<ParkingMutex<Option<String>>>,
    pub received_cwd: Arc<ParkingMutex<Option<String>>>,
    pub received_roots: Arc<ParkingMutex<Vec<String>>>,
    pub turn_threads: Arc<ParkingMutex<Vec<String>>>,
    pub approval_responses: Arc<ParkingMutex<Vec<Value>>>,
    pub conn_count: Arc<AtomicUsize>,
    pub interrupts: Arc<ParkingMutex<Vec<Value>>>,
}

impl FakeAppServer {
    async fn spawn() -> Self {
        Self::spawn_inner(false, false, false, false, 0).await
    }

    async fn spawn_with_activity() -> Self {
        Self::spawn_inner(true, false, false, false, 0).await
    }

    async fn spawn_without_turn_completed() -> Self {
        Self::spawn_inner(false, false, false, true, 0).await
    }

    async fn spawn_with_delayed_turn_completed(delay_ms: u64) -> Self {
        Self::spawn_inner(false, false, false, false, delay_ms).await
    }

    /// Drop the client's first connection abruptly: exercises the
    /// one-shot transport retry.
    async fn spawn_with_first_connection_drop() -> Self {
        Self::spawn_inner(false, true, false, false, 0).await
    }

    /// Stream deltas forever (well, 5s) without turn/completed: exercises
    /// the whole-turn deadline + turn/interrupt.
    async fn spawn_with_looping_deltas() -> Self {
        Self::spawn_inner(false, false, true, false, 0).await
    }

    async fn spawn_inner(
        emit_activity: bool,
        drop_first_connection: bool,
        loop_deltas: bool,
        omit_turn_completed: bool,
        turn_completed_delay_ms: u64,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind free port");
        let port = listener.local_addr().expect("local addr").port();

        let thread_start_count = Arc::new(AtomicUsize::new(0));
        let thread_resume_count = Arc::new(AtomicUsize::new(0));
        let received_developer_instructions = Arc::new(ParkingMutex::new(None));
        let received_model = Arc::new(ParkingMutex::new(None));
        let received_cwd = Arc::new(ParkingMutex::new(None));
        let received_roots = Arc::new(ParkingMutex::new(Vec::new()));
        let turn_threads = Arc::new(ParkingMutex::new(Vec::new()));
        let approval_responses = Arc::new(ParkingMutex::new(Vec::new()));
        let emit_activity_flag = Arc::new(AtomicBool::new(emit_activity));
        let conn_count = Arc::new(AtomicUsize::new(0));
        let drop_flag = drop_first_connection;
        let interrupts = Arc::new(ParkingMutex::new(Vec::new()));

        let tsc = thread_start_count.clone();
        let trc = thread_resume_count.clone();
        let rdi = received_developer_instructions.clone();
        let rm = received_model.clone();
        let rcwd = received_cwd.clone();
        let rroots = received_roots.clone();
        let tt = turn_threads.clone();
        let ar = approval_responses.clone();
        let ea = emit_activity_flag.clone();
        let cc = conn_count.clone();
        let drop_first = drop_flag;
        let it = interrupts.clone();
        let looping = loop_deltas;
        let omit_terminal = omit_turn_completed;
        let terminal_delay_ms = turn_completed_delay_ms;

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let conn_num = cc.fetch_add(1, Ordering::SeqCst) + 1;
                if drop_first && conn_num == 1 {
                    // Abruptly drop the first connection (no WS close frame):
                    // the client must see a transport error and retry once.
                    drop(stream);
                    continue;
                }
                let tsc = tsc.clone();
                let trc = trc.clone();
                let rdi = rdi.clone();
                let rm = rm.clone();
                let rcwd = rcwd.clone();
                let rroots = rroots.clone();
                let tt = tt.clone();
                let ar = ar.clone();
                let ea = ea.clone();
                let it = it.clone();

                tokio::spawn(async move {
                    let ws = match tokio_tungstenite::accept_async(stream).await {
                        Ok(ws) => ws,
                        Err(_) => return,
                    };
                    let (mut ws_sink, mut ws_stream) = ws.split();

                    let (outgoing_tx, mut outgoing_rx) = tokio::sync::mpsc::channel::<Message>(32);

                    tokio::spawn(async move {
                        while let Some(msg) = outgoing_rx.recv().await {
                            if ws_sink.send(msg).await.is_err() {
                                break;
                            }
                        }
                    });

                    while let Some(Ok(msg)) = ws_stream.next().await {
                        if let Message::Text(text) = msg {
                            let parsed: Value = match serde_json::from_str(text.as_str()) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };

                            // Check if client is responding to a server request (e.g. approval denial)
                            if parsed.get("method").is_none() && parsed.get("id").is_some() {
                                ar.lock().push(parsed.clone());
                                continue;
                            }

                            let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
                            let id = parsed.get("id").and_then(Value::as_u64).unwrap_or(0);

                            match method {
                                "turn/interrupt" => {
                                    it.lock().push(parsed.clone());
                                }
                                "initialize" => {
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "userAgent": "fake-app-server/1.0",
                                            "codexHome": "/tmp/codex"
                                        }
                                    });
                                    let _ = outgoing_tx.send(Message::text(resp.to_string())).await;
                                }
                                "thread/start" => {
                                    tsc.fetch_add(1, Ordering::SeqCst);
                                    let params =
                                        parsed.get("params").cloned().unwrap_or(Value::Null);
                                    if let Some(dev_inst) =
                                        params.get("developerInstructions").and_then(Value::as_str)
                                    {
                                        *rdi.lock() = Some(dev_inst.to_string());
                                    }
                                    if let Some(m) = params.get("model").and_then(Value::as_str) {
                                        *rm.lock() = Some(m.to_string());
                                    }
                                    if let Some(cwd) = params.get("cwd").and_then(Value::as_str) {
                                        *rcwd.lock() = Some(cwd.to_string());
                                    }
                                    if let Some(roots) = params
                                        .get("runtimeWorkspaceRoots")
                                        .and_then(Value::as_array)
                                    {
                                        *rroots.lock() = roots
                                            .iter()
                                            .filter_map(Value::as_str)
                                            .map(String::from)
                                            .collect();
                                    }

                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "thread": {
                                                "id": "server-assigned-thread-uuid-1234",
                                                "sessionId": "fake-session-001"
                                            },
                                            "model": "claude-3-5-sonnet"
                                        }
                                    });
                                    let _ = outgoing_tx.send(Message::text(resp.to_string())).await;
                                }
                                "thread/resume" => {
                                    trc.fetch_add(1, Ordering::SeqCst);
                                    let thread_id = parsed
                                        .pointer("/params/threadId")
                                        .and_then(Value::as_str)
                                        .unwrap_or_default();
                                    if thread_id == "stale-thread" {
                                        let response = json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {
                                                "code": -32603,
                                                "message": "no rollout found for thread id stale-thread"
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(response.to_string()))
                                            .await;
                                        continue;
                                    }
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "thread": {
                                                "id": "server-assigned-thread-uuid-1234",
                                                "sessionId": "fake-session-001"
                                            }
                                        }
                                    });
                                    let _ = outgoing_tx.send(Message::text(resp.to_string())).await;
                                }
                                "turn/start" => {
                                    let params =
                                        parsed.get("params").cloned().unwrap_or(Value::Null);
                                    let thread_id = params
                                        .get("threadId")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                    tt.lock().push(thread_id.clone());

                                    // Send turn/start response
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "turn-001",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = outgoing_tx.send(Message::text(resp.to_string())).await;

                                    // Send server request requiring client denial response: execCommandApproval
                                    let approval_req = json!({
                                        "jsonrpc": "2.0",
                                        "id": 999,
                                        "method": "execCommandApproval",
                                        "params": {
                                            "command": "rm -rf /",
                                            "reason": "destructive"
                                        }
                                    });
                                    let _ = outgoing_tx
                                        .send(Message::text(approval_req.to_string()))
                                        .await;

                                    // Observe the exact denial before publishing completion.
                                    let response = tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        ws_stream.next(),
                                    )
                                    .await
                                    .unwrap()
                                    .unwrap()
                                    .unwrap();
                                    let response: Value =
                                        serde_json::from_str(response.to_text().unwrap()).unwrap();
                                    assert_eq!(response["id"], 999);
                                    ar.lock().push(response);

                                    // Stream item/agentMessage/delta chunks
                                    let emit_activity = ea.load(Ordering::SeqCst);
                                    if emit_activity {
                                        let reasoning_done = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/completed",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "item": {
                                                    "type": "reasoning",
                                                    "text": "hmm let me think about this carefully"
                                                }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(reasoning_done.to_string()))
                                            .await;

                                        let tool_started = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/started",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "item": {
                                                    "type": "commandExecution",
                                                    "id": "call-1",
                                                    "command": "echo OMOITEMPROBE-OK",
                                                    "status": "inProgress"
                                                }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(tool_started.to_string()))
                                            .await;

                                        let output_delta = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/commandExecution/outputDelta",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "itemId": "call-1",
                                                "delta": "OMOITEMPROBE-OK"
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(output_delta.to_string()))
                                            .await;

                                        let tool_completed = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/completed",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "item": {
                                                    "type": "commandExecution",
                                                    "id": "call-1",
                                                    "command": "echo OMOITEMPROBE-OK",
                                                    "status": "completed",
                                                    "aggregatedOutput": "OMOITEMPROBE-OK"
                                                }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(tool_completed.to_string()))
                                            .await;
                                    }

                                    if looping {
                                        // Emulate an agent that never finishes:
                                        // deltas forever, no turn/completed.
                                        let t_id = thread_id.clone();
                                        let sink = outgoing_tx.clone();
                                        tokio::spawn(async move {
                                            let started = json!({
                                                "jsonrpc": "2.0",
                                                "method": "turn/started",
                                                "params": {
                                                    "threadId": t_id,
                                                    "turnId": "turn-loop"
                                                }
                                            });
                                            let _ =
                                                sink.send(Message::text(started.to_string())).await;
                                            let mut i: u32 = 0;
                                            while i < 60 {
                                                i += 1;
                                                let d = json!({
                                                    "jsonrpc": "2.0",
                                                    "method": "item/agentMessage/delta",
                                                    "params": {
                                                        "threadId": t_id,
                                                        "turnId": "turn-loop",
                                                        "itemId": "loop-msg",
                                                        "delta": format!("loop {} ", i)
                                                    }
                                                });
                                                if sink
                                                    .send(Message::text(d.to_string()))
                                                    .await
                                                    .is_err()
                                                {
                                                    break;
                                                }
                                                tokio::time::sleep(
                                                    std::time::Duration::from_millis(150),
                                                )
                                                .await;
                                            }
                                            let done = json!({
                                                "jsonrpc": "2.0",
                                                "method": "turn/completed",
                                                "params": {
                                                    "threadId": t_id,
                                                    "turn": {"id": "turn-loop", "status": "completed"}
                                                }
                                            });
                                            let _ =
                                                sink.send(Message::text(done.to_string())).await;
                                        });
                                        // NOTE: the reader task must keep running
                                        // so it can observe the client's
                                        // turn/interrupt frame.
                                    }

                                    if !looping {
                                        let delta1 = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/agentMessage/delta",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "itemId": "msg-001",
                                                "delta": if emit_activity { "OMO" } else { "Hello " }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(delta1.to_string()))
                                            .await;

                                        let delta2 = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/agentMessage/delta",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "itemId": "msg-001",
                                                "delta": if emit_activity { "ACT-OK" } else { "World!" }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(delta2.to_string()))
                                            .await;

                                        // Send item/completed
                                        let item_completed = json!({
                                            "jsonrpc": "2.0",
                                            "method": "item/completed",
                                            "params": {
                                                "threadId": thread_id,
                                                "turnId": "turn-001",
                                                "item": {
                                                    "type": "agentMessage",
                                                    "text": if emit_activity { "OMOACT-OK" } else { "Hello World!" }
                                                }
                                            }
                                        });
                                        let _ = outgoing_tx
                                            .send(Message::text(item_completed.to_string()))
                                            .await;

                                        // Send turn/completed
                                        let turn_completed = json!({
                                            "jsonrpc": "2.0",
                                            "method": "turn/completed",
                                            "params": {
                                                "threadId": thread_id,
                                                "turn": {
                                                    "id": "turn-001",
                                                    "status": "completed"
                                                }
                                            }
                                        });
                                        if !omit_terminal {
                                            if terminal_delay_ms > 0 {
                                                tokio::time::sleep(
                                                    std::time::Duration::from_millis(
                                                        terminal_delay_ms,
                                                    ),
                                                )
                                                .await;
                                            }
                                            let _ = outgoing_tx
                                                .send(Message::text(turn_completed.to_string()))
                                                .await;
                                        } else {
                                            let idle = json!({
                                                "jsonrpc": "2.0",
                                                "method": "thread/status/changed",
                                                "params": {
                                                    "threadId": thread_id,
                                                    "status": {"type": "idle"}
                                                }
                                            });
                                            let _ = outgoing_tx
                                                .send(Message::text(idle.to_string()))
                                                .await;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                });
            }
        });

        Self {
            port,
            thread_start_count,
            thread_resume_count,
            received_developer_instructions,
            received_model,
            received_cwd,
            received_roots,
            turn_threads,
            approval_responses,
            conn_count,
            interrupts,
        }
    }
}

async fn spawn_turn_start_error_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let response = match request.get("method").and_then(Value::as_str) {
                Some("initialize") => json!({"jsonrpc":"2.0","id":id,"result":{}}),
                Some("thread/start") => json!({
                    "jsonrpc":"2.0","id":id,
                    "result":{"thread":{"id":"thread-1"}}
                }),
                Some("turn/start") => json!({
                    "jsonrpc":"2.0","id":id,
                    "error":{"code":-32603,"message":"turn rejected"}
                }),
                _ => continue,
            };
            let _ = ws.send(Message::text(response.to_string())).await;
        }
    });
    port
}

async fn spawn_cross_thread_completed_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            match request.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                        ))
                        .await;
                }
                Some("thread/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"thread-1"}}})
                                .to_string(),
                        ))
                        .await;
                }
                Some("turn/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-1","status":"inProgress"}}})
                                .to_string(),
                        ))
                        .await;
                    let first = json!({
                        "jsonrpc":"2.0","method":"item/agentMessage/delta",
                        "params":{"threadId":"thread-1","turnId":"turn-1","itemId":"m1","delta":"Hello "}
                    });
                    let _ = ws.send(Message::text(first.to_string())).await;
                    // A turn/completed belonging to a DIFFERENT thread must
                    // not terminate this turn (observed on a live daemon at
                    // +6s while the real turn was still running).
                    let stray = json!({
                        "jsonrpc":"2.0","method":"turn/completed",
                        "params":{"threadId":"thread-other","turn":{"id":"turn-x","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(stray.to_string())).await;
                    let second = json!({
                        "jsonrpc":"2.0","method":"item/agentMessage/delta",
                        "params":{"threadId":"thread-1","turnId":"turn-1","itemId":"m1","delta":"World!"}
                    });
                    let _ = ws.send(Message::text(second.to_string())).await;
                    let done = json!({
                        "jsonrpc":"2.0","method":"turn/completed",
                        "params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(done.to_string())).await;
                }
                _ => continue,
            }
        }
    });
    port
}

async fn spawn_empty_turn_completed_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            match request.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                        ))
                        .await;
                }
                Some("thread/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"thread-1"}}})
                                .to_string(),
                        ))
                        .await;
                }
                Some("turn/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-1","status":"inProgress"}}})
                                .to_string(),
                        ))
                        .await;
                    // Terminal signal with NO streamed content: the turn must
                    // fail instead of recording a successful empty delivery.
                    let done = json!({
                        "jsonrpc":"2.0","method":"turn/completed",
                        "params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(done.to_string())).await;
                }
                _ => continue,
            }
        }
    });
    port
}

/// Like [`spawn_empty_turn_completed_server`], but the content-less terminal
/// arrives only after `delay_ms`, so the configured empty-terminal grace
/// window has already elapsed when it lands.
async fn spawn_delayed_empty_turn_completed_server(delay_ms: u64) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            match request.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                        ))
                        .await;
                }
                Some("thread/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"thread-1"}}})
                                .to_string(),
                        ))
                        .await;
                }
                Some("turn/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-1","status":"inProgress"}}})
                                .to_string(),
                        ))
                        .await;
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    let done = json!({
                        "jsonrpc":"2.0","method":"turn/completed",
                        "params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(done.to_string())).await;
                }
                _ => continue,
            }
        }
    });
    port
}

/// Completes the handshake and then emits only WebSocket ping frames: this
/// is the keepalive-alive-but-wedged daemon shape that must still hit the
/// whole-turn deadline.
async fn spawn_pinging_idle_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            match request.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                        ))
                        .await;
                }
                Some("thread/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"thread-1"}}})
                                .to_string(),
                        ))
                        .await;
                }
                Some("turn/start") => {
                    let _ = ws
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-1","status":"inProgress"}}})
                                .to_string(),
                        ))
                        .await;
                    // Keepalive pings forever; zero turn events ever arrive.
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        if ws
                            .send(Message::Ping(b"keepalive".to_vec().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                _ => continue,
            }
        }
    });
    port
}

async fn spawn_tool_after_agent_message_server(
    release: tokio::sync::oneshot::Receiver<()>,
    stop: tokio::sync::oneshot::Receiver<()>,
    peers: &mut tokio::task::JoinSet<()>,
) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    peers.spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            let Ok(request) = serde_json::from_str::<Value>(text.as_str()) else {
                continue;
            };
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            match request.get("method").and_then(Value::as_str) {
                Some("initialize") => {
                    let response = json!({"jsonrpc":"2.0","id":id,"result":{}});
                    let _ = ws.send(Message::text(response.to_string())).await;
                }
                Some("thread/start") => {
                    let response = json!({
                        "jsonrpc":"2.0","id":id,
                        "result":{"thread":{"id":"thread-tool-sequence"}}
                    });
                    let _ = ws.send(Message::text(response.to_string())).await;
                }
                Some("turn/start") => {
                    let response = json!({
                        "jsonrpc":"2.0","id":id,
                        "result":{"turn":{"id":"turn-tool-sequence","status":"inProgress"}}
                    });
                    let _ = ws.send(Message::text(response.to_string())).await;
                    let intent = json!({
                        "jsonrpc":"2.0","method":"item/completed",
                        "params":{"threadId":"thread-tool-sequence","turnId":"turn-tool-sequence","item":{"type":"agentMessage","id":"intent","text":"I read this as the digest task."}}
                    });
                    let _ = ws.send(Message::text(intent.to_string())).await;
                    let tool_started = json!({
                        "jsonrpc":"2.0","method":"item/started",
                        "params":{"threadId":"thread-tool-sequence","turnId":"turn-tool-sequence","item":{"type":"commandExecution","id":"tool-1","command":"write digest","status":"inProgress"}}
                    });
                    let _ = ws.send(Message::text(tool_started.to_string())).await;

                    // No tool-start dispatch exists; this ordered delta witnesses
                    // processing of the preceding correlated tool-start frame.
                    let processed = json!({
                        "method":"item/agentMessage/delta",
                        "params":{"threadId":"thread-tool-sequence","turnId":"turn-tool-sequence","itemId":"intent","delta":"U12_TOOL_STARTED"}
                    });
                    ws.send(Message::text(processed.to_string())).await.unwrap();
                    release.await.expect("release held tool result");

                    let tool_completed = json!({
                        "jsonrpc":"2.0","method":"item/completed",
                        "params":{"threadId":"thread-tool-sequence","turnId":"turn-tool-sequence","item":{"type":"commandExecution","id":"tool-1","command":"write digest","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(tool_completed.to_string())).await;
                    let digest = json!({
                        "jsonrpc":"2.0","method":"item/completed",
                        "params":{"threadId":"thread-tool-sequence","turnId":"turn-tool-sequence","item":{"type":"agentMessage","id":"digest","text":"## Actual digest body"}}
                    });
                    let _ = ws.send(Message::text(digest.to_string())).await;
                    let idle = json!({
                        "jsonrpc":"2.0","method":"turn/completed",
                        "params":{"threadId":"thread-tool-sequence","turn":{"id":"turn-tool-sequence","status":"completed"}}
                    });
                    let _ = ws.send(Message::text(idle.to_string())).await;
                    stop.await.expect("backend finished before peer shutdown");
                    return;
                }
                _ => {}
            }
        }
    });
    port
}

#[tokio::test]
async fn test_omo_backend_surfaces_turn_start_rpc_error_immediately() {
    // Given: app-server rejects turn/start before any turn notifications.
    let port = spawn_turn_start_error_server().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_total_timeout(std::time::Duration::from_secs(5));
    let backend = OmoBackend::new(config, Arc::new(CapturingDispatcher::new()));
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
    let mut session = SessionContext::new(session_key.clone());

    // When: a turn is rejected at the RPC boundary.
    let started = std::time::Instant::now();
    let error = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "hello"),
        )
        .await
        .expect_err("turn/start RPC error must fail the run");

    // Then: the concrete server error is surfaced without waiting for the deadline.
    assert!(error.to_string().contains("turn rejected"));
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

#[tokio::test]
async fn test_omo_backend_requires_turn_terminal_even_after_final_agent_message() {
    let server = FakeAppServer::spawn_without_turn_completed().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port))
        .with_request_timeout(std::time::Duration::from_millis(800));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    assert!(
        result.is_err(),
        "uncorrelated idle must not finalize: {result:?}"
    );
    let chunks = dispatcher.stream_chunks();
    assert!(!chunks.iter().any(|chunk| chunk.is_final));
}

#[tokio::test]
async fn test_omo_backend_waits_for_final_message_after_tool_activity() {
    // Positive witness: the same interval must age the real backend clock.
    run_quiet_tool_interval(false).await;
    run_quiet_tool_interval(true).await;
}

async fn run_quiet_tool_interval(clock_witness: bool) {
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let mut peers = tokio::task::JoinSet::new();
    let port = spawn_tool_after_agent_message_server(release_rx, stop_rx, &mut peers).await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(3))
        .with_no_content_grace(std::time::Duration::from_secs(1))
        .with_total_timeout(std::time::Duration::from_secs(if clock_witness {
            1
        } else {
            10
        }));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    *dispatcher.tool_started.lock() = Some(started_tx);
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    let run = backend.run(
        &mut session,
        InboundEvent::message(session_key, "msg", "digest"),
    );
    tokio::pin!(run);
    tokio::select! {
        biased;
        result = &mut run => panic!("backend ended before tool-start witness: {result:?}"),
        started = tokio::time::timeout(std::time::Duration::from_secs(5), started_rx) => {
            started.expect("bounded tool-start dispatch").expect("tool-start recorded");
        }
    }

    // Pause after real socket I/O, avoiding auto-advance during the handshake.
    // Poll the actual run, not a JoinHandle: expired fallback timers must run.
    tokio::time::pause();
    let quiet_started = tokio::time::Instant::now();
    tokio::time::advance(std::time::Duration::from_millis(1_200)).await;
    let mut poll_res = futures_util::poll!(&mut run);
    if clock_witness && poll_res.is_pending() {
        tokio::time::advance(std::time::Duration::from_millis(100)).await;
        poll_res = futures_util::poll!(&mut run);
    }
    if clock_witness {
        assert!(
            poll_res.is_ready(),
            "backend must time out when total deadline expires without frames"
        );
        let error = match poll_res {
            std::task::Poll::Ready(Err(e)) => e,
            other => panic!("expected deadline error, got: {other:?}"),
        };
        assert!(
            error.to_string().contains("turn exceeded total deadline"),
            "{error}"
        );
        assert!(!dispatcher
            .stream_chunks()
            .iter()
            .any(|chunk| chunk.is_final));
        tokio::time::resume();
        let _ = release_tx.send(());
        let _ = stop_tx.send(());
        tokio::time::timeout(std::time::Duration::from_secs(5), peers.join_next())
            .await
            .expect("bounded peer cleanup")
            .expect("one peer")
            .expect("peer succeeded");
        assert!(peers.is_empty());
        println!("U12 clock witness: processed tool-start +1200ms exceeds grace=1000ms and backend deadline=1000ms; deadline error; finals=0");
        return;
    }
    assert!(poll_res.is_pending(), "backend ended during held tool");
    assert!(!dispatcher
        .stream_chunks()
        .iter()
        .any(|chunk| chunk.is_final));
    assert!(quiet_started.elapsed() > std::time::Duration::from_millis(1_100));
    assert!(quiet_started.elapsed() > backend.config.no_content_grace);
    println!(
        "U12 quiet: processed tool-start; virtual_ms=1200; backend=pending; finals=0; result=held"
    );
    tokio::time::resume();

    release_tx
        .send(())
        .expect("release correlated completion/digest/terminal");
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), &mut run)
        .await
        .expect("bounded final digest");
    stop_tx
        .send(())
        .expect("stop peer after backend completion");
    tokio::time::timeout(std::time::Duration::from_secs(5), peers.join_next())
        .await
        .expect("bounded peer cleanup")
        .expect("one peer")
        .expect("peer succeeded");
    assert!(peers.is_empty());

    assert!(result.is_ok(), "tool sequence failed: {result:?}");
    let final_chunk = dispatcher
        .stream_chunks()
        .into_iter()
        .find(|chunk| chunk.is_final)
        .expect("final digest chunk");
    assert!(final_chunk.content.contains("## Actual digest body"));
    assert!(!final_chunk
        .content
        .contains("I read this as the digest task"));
    assert_eq!(
        dispatcher
            .stream_chunks()
            .iter()
            .filter(|chunk| chunk.is_final)
            .count(),
        1
    );
    println!(
        "U12 quiet: released correlated completion/digest/terminal; actual digest final=1; peers=0"
    );
}

#[tokio::test]
async fn test_omo_backend_accepts_received_completion_at_deadline_edge() {
    // Reconciled for U14: a completion received within the configured total
    // deadline (at the edge) is accepted, but stale terminals past the
    // deadline cannot bypass.
    let server = FakeAppServer::spawn_with_delayed_turn_completed(40).await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port))
        .with_request_timeout(std::time::Duration::from_secs(2))
        .with_total_timeout(std::time::Duration::from_millis(100));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    assert!(
        result.is_ok(),
        "deadline-edge completion failed: {result:?}"
    );
    assert!(dispatcher
        .stream_chunks()
        .last()
        .is_some_and(|chunk| chunk.is_final));

    // Stale terminal past total deadline cannot bypass:
    let server_late = FakeAppServer::spawn_with_delayed_turn_completed(80).await;
    let config_late = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server_late.port))
        .with_request_timeout(std::time::Duration::from_secs(2))
        .with_total_timeout(std::time::Duration::from_millis(50));
    let dispatcher_late = Arc::new(CapturingDispatcher::new());
    let backend_late = OmoBackend::new(config_late, dispatcher_late);
    let session_key_late =
        SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session_late = SessionContext::new(session_key_late.clone());

    let result_late = backend_late
        .run(
            &mut session_late,
            InboundEvent::message(session_key_late, "msg", "digest"),
        )
        .await;

    assert!(
        result_late.is_err(),
        "stale terminal arriving past deadline must not bypass"
    );
    assert!(result_late
        .unwrap_err()
        .to_string()
        .contains("turn exceeded total deadline"),);
}

#[tokio::test]
async fn test_omo_backend_e2e_thread_lifecycle_and_streaming() {
    // Given: Fake app-server running on 127.0.0.1 with ephemeral port
    let server = FakeAppServer::spawn().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let config = OmoBackendConfig::new(&url).with_default_model(Some("claude-3-5-sonnet"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(session_key.clone());
    session.state.system_prompt = Some("You are a helpful test persona.".to_string());
    session.state.active_model = Some("claude-3-5-sonnet".to_string());

    // When: First turn executes
    let event1 = InboundEvent::message(session_key.clone(), "msg-1", "Say hello");
    let result1 = backend.run(&mut session, event1).await;

    // Then: First turn succeeds
    assert!(result1.is_ok(), "First turn failed: {:?}", result1.err());
    assert_eq!(server.thread_start_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        server.received_developer_instructions.lock().as_deref(),
        Some("You are a helpful test persona.")
    );
    assert_eq!(
        server.received_model.lock().as_deref(),
        Some("claude-3-5-sonnet")
    );

    // Thread ID stored in session state metadata
    let stored_thread_id = session
        .state
        .metadata
        .get("omo_thread_id")
        .and_then(Value::as_str)
        .map(String::from);
    assert_eq!(
        stored_thread_id.as_deref(),
        Some("server-assigned-thread-uuid-1234"),
        "threadId must be stored in session state metadata"
    );

    // StreamChunks yielded in order
    let chunks = dispatcher.stream_chunks();
    assert!(!chunks.is_empty(), "Must yield stream chunks");
    let combined_content = chunks.last().map(|c| c.content.clone()).unwrap_or_default();
    assert_eq!(combined_content, "Hello World!");

    // Server approval request was answered with denial
    let approvals = server.approval_responses.lock().clone();
    assert!(!approvals.is_empty(), "Approval response must be sent");
    let first_approval = &approvals[0];
    assert_eq!(first_approval.get("id").and_then(Value::as_u64), Some(999));
    let allow = first_approval
        .get("result")
        .and_then(|r| r.get("allow"))
        .and_then(Value::as_bool);
    let decision = first_approval
        .get("result")
        .and_then(|r| r.get("decision"))
        .and_then(Value::as_str);
    assert_eq!(allow, Some(false));
    assert!(matches!(decision, Some("decline") | Some("deny")));

    // When: Second turn executes for the same session
    let event2 = InboundEvent::message(session_key.clone(), "msg-2", "Say hello again");
    let result2 = backend.run(&mut session, event2).await;

    // Then: Second turn resumes the existing thread on its new WebSocket and
    // reuses the threadId without starting another thread.
    assert!(result2.is_ok(), "Second turn failed: {:?}", result2.err());
    assert_eq!(
        server.thread_start_count.load(Ordering::SeqCst),
        1,
        "thread/start must NOT be called again on second turn with same session"
    );
    assert_eq!(
        server.thread_resume_count.load(Ordering::SeqCst),
        1,
        "thread/resume must subscribe the new WebSocket before the second turn"
    );
    let turn_threads = server.turn_threads.lock().clone();
    assert_eq!(turn_threads.len(), 2);
    assert_eq!(turn_threads[0], "server-assigned-thread-uuid-1234");
    assert_eq!(turn_threads[1], "server-assigned-thread-uuid-1234");
}

#[tokio::test]
async fn test_omo_backend_replaces_stale_cached_thread() {
    // Given: a session carrying a thread ID that the app-server has unloaded.
    let server = FakeAppServer::spawn().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher);
    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-stale",
    );
    let mut session = SessionContext::new(session_key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("stale-thread"));

    // When: a turn starts after the remote thread has been unloaded.
    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg-1", "Hello"),
        )
        .await;

    // Then: missing rollout on resume must fail explicitly with continuity_error,
    // not silently start a fresh thread with lost history.
    assert!(
        result.is_err(),
        "missing rollout must fail explicitly rather than silently starting a fresh thread"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("continuity_error"),
        "error must report continuity_error, got: {err}"
    );
    assert_eq!(server.thread_resume_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        server.thread_start_count.load(Ordering::SeqCst),
        0,
        "must not issue thread/start replacement"
    );
}

#[tokio::test]
async fn test_omo_backend_cron_sessions_always_start_fresh_thread() {
    // Given: a cron session that still carries a cached thread id.
    // Resumed in-memory threads in the app-server do not deliver turn
    // completion events to the new connection, so cron sessions must
    // never resume — result-only reporting needs no conversation
    // continuity.
    let server = FakeAppServer::spawn().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher);
    let session_key = SessionKey::new(
        "discord",
        None::<String>,
        "chan-1",
        None::<String>,
        "cron:omon-sample-digest-v1",
    );
    let mut session = SessionContext::new(session_key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("cached-thread"));

    // When: a cron turn runs.
    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg-1", "Run the digest"),
        )
        .await;

    // Then: the cached thread is never resumed — a fresh thread/start is
    // issued, and no thread id is cached for cron sessions.
    assert!(result.is_ok(), "cron turn failed: {result:?}");
    assert_eq!(server.thread_resume_count.load(Ordering::SeqCst), 0);
    assert_eq!(server.thread_start_count.load(Ordering::SeqCst), 1);
    assert!(!session.state.metadata.contains_key("omo_thread_id"));
}

#[tokio::test]
async fn test_omo_backend_unreachable_daemon_error() {
    // Port 1 is not listening
    let config = OmoBackendConfig::new("ws://127.0.0.1:1").with_default_model(Some("test-model"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher);

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(session_key.clone());
    let event = InboundEvent::message(session_key, "msg-1", "Hello");

    let result = backend.run(&mut session, event).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_omo_backend_emits_hermes_activity_lines() {
    // Given: fake server emitting reasoning + commandExecution activity items
    let server = FakeAppServer::spawn_with_activity().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let config = OmoBackendConfig::new(&url).with_default_model(Some("claude-3-5-sonnet"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(session_key.clone());
    session.state.system_prompt = Some("You are a helpful test persona.".to_string());

    // When: a turn runs through tool + reasoning activity
    let event = InboundEvent::message(session_key.clone(), "msg-1", "Run the probe command");
    let result = backend.run(&mut session, event).await;
    assert!(result.is_ok(), "turn failed: {:?}", result.err());

    // Then: the final message carries reply cleanly without noisy raw logs or tool badges
    let chunks = dispatcher.stream_chunks();
    assert!(!chunks.is_empty(), "no chunks emitted");

    let last = chunks.last().map(|c| c.content.clone()).unwrap_or_default();
    let expected = "OMOACT-OK";
    assert_eq!(last, expected, "final chunk layout mismatch: {last:?}");
}

#[tokio::test]
async fn test_omo_backend_cron_session_suppresses_activity_lines_and_delivers_only_final_result() {
    // Given: fake server emitting reasoning + commandExecution activity items followed by result
    let server = FakeAppServer::spawn_with_activity().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let config = OmoBackendConfig::new(&url).with_default_model(Some("claude-3-5-sonnet"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "cron:omon-sample-digest-v1",
    );
    let mut session = SessionContext::new(session_key.clone());
    session.state.metadata.insert(
        "cron_scheduler_delivery".to_string(),
        serde_json::json!(true),
    );

    // When: a cron turn runs through tool + reasoning activity
    let event = InboundEvent::message(
        session_key.clone(),
        "cron:omon-sample-digest-v1",
        "Run the probe command",
    );
    let result = backend.run(&mut session, event).await;
    assert!(result.is_ok(), "turn failed: {:?}", result.err());

    // Then: final chunk delivered must NOT contain activity lines (no ⚙️ Running, no > 💭, no ⤷)
    let chunks = dispatcher.stream_chunks();
    assert!(!chunks.is_empty(), "no chunks emitted");

    let final_chunk = chunks
        .iter()
        .find(|c| c.is_final)
        .map(|c| c.content.as_str())
        .expect("final chunk must be emitted");

    assert!(
        !final_chunk.contains("⚙️ Running"),
        "cron final chunk must not contain tool activity lines: {final_chunk}"
    );
    assert!(
        !final_chunk.contains("> 💭"),
        "cron final chunk must not contain reasoning blocks: {final_chunk}"
    );
    assert!(
        !final_chunk.contains("⤷"),
        "cron final chunk must not contain output excerpts: {final_chunk}"
    );
    assert_eq!(
        final_chunk, "OMOACT-OK",
        "cron final chunk should contain ONLY the pure final result"
    );
}

#[tokio::test]
async fn test_omo_backend_persists_without_preexisting_session_row() {
    // Regression: cron sessions are created implicitly by backend.run —
    // persisting the assistant message must not violate the messages->
    // sessions foreign key (code 787).
    let server = FakeAppServer::spawn().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let config = OmoBackendConfig::new(&url).with_default_model(Some("claude-3-5-sonnet"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone()).with_pool(pool.clone());

    let session_key = SessionKey::new(
        "local",
        None::<String>,
        "test-cron-job",
        None::<String>,
        "cron:test-cron-job",
    );
    let mut session = SessionContext::new(session_key.clone());
    session.state.system_prompt = Some("You are a helpful test persona.".to_string());

    let event = InboundEvent::message(session_key.clone(), "msg-1", "Say hello");
    let result = backend.run(&mut session, event).await;
    assert!(
        result.is_ok(),
        "turn must succeed without a pre-existing session row: {:?}",
        result.err()
    );

    let session_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(session_rows >= 1, "session row must be ensured");

    let assistant: Option<String> =
        sqlx::query_scalar("SELECT content FROM messages WHERE role='assistant'")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(assistant.as_deref(), Some("Hello World!"));

    let fk_enabled: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        fk_enabled, 1,
        "foreign keys must be enforced for this test to be meaningful"
    );
}

#[tokio::test]
async fn test_omo_backend_retries_once_after_connection_drop() {
    // Given: the fake server drops the client's first connection abruptly
    let server = FakeAppServer::spawn_with_first_connection_drop().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let config = OmoBackendConfig::new(&url).with_default_model(Some("claude-3-5-sonnet"));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(session_key.clone());

    // When: the turn survives exactly one transport drop
    let event = InboundEvent::message(session_key.clone(), "msg-1", "Say hello");
    let result = backend.run(&mut session, event).await;
    assert!(
        result.is_ok(),
        "turn must succeed via the one-shot transport retry: {:?}",
        result.err()
    );

    // Then: the retry reconnected and served the full flow
    assert_eq!(server.conn_count.load(Ordering::SeqCst), 2);
    let chunks = dispatcher.stream_chunks();
    let last = chunks.last().map(|c| c.content.clone()).unwrap_or_default();
    assert_eq!(last, "Hello World!");
}

#[tokio::test]
async fn test_omo_backend_deadline_interrupts_looping_turn() {
    // Given: a daemon whose agent loops forever streaming deltas
    let server = FakeAppServer::spawn_with_looping_deltas().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let config = OmoBackendConfig::new(&url)
        .with_default_model(Some("claude-3-5-sonnet"))
        .with_total_timeout(std::time::Duration::from_millis(600));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(session_key.clone());

    // When: the turn runs past the whole-turn deadline
    let event = InboundEvent::message(session_key.clone(), "msg-1", "Loop forever");
    let started = std::time::Instant::now();
    let result = backend.run(&mut session, event).await;
    let elapsed = started.elapsed();

    // Then: the turn fails fast instead of hanging, and the daemon is
    // asked to interrupt the turn (freeing the thread).
    assert!(
        result.is_err(),
        "looping turn must hit the total deadline, got {:?}",
        result.ok()
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "deadline must fire promptly, took {:?}",
        elapsed
    );
    assert!(
        !server.interrupts.lock().is_empty(),
        "turn/interrupt must be sent to the daemon"
    );
}

#[tokio::test]
async fn test_omo_backend_provisions_agent_workspace_and_passes_to_thread_start() {
    // Given: a running FakeAppServer and an OmoBackend configured with a workspace root
    let server = FakeAppServer::spawn().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp_dir.path().to_path_buf();

    let config = OmoBackendConfig::new(&url).with_workspace_root(workspace_root.clone());
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    )
    .with_bot_id("1465631383862120451");
    let mut session = SessionContext::new(session_key.clone());

    // When: running one backend turn
    let event = InboundEvent::message(session_key.clone(), "msg-1", "Hello from bot");
    let result = backend.run(&mut session, event).await;
    assert!(
        result.is_ok(),
        "backend.run must succeed: {:?}",
        result.err()
    );

    // Then: thread/start received cwd and roots, agent and shared dirs exist, and .omo/omo.json exists
    let expected_agent_dir = workspace_root
        .join("agents")
        .join("bot-1465631383862120451");
    let expected_shared_dir = workspace_root.join("shared");
    let expected_cwd_str = expected_agent_dir.to_str().unwrap().to_string();
    let expected_shared_str = expected_shared_dir.to_str().unwrap().to_string();

    assert_eq!(
        *server.received_cwd.lock(),
        Some(expected_cwd_str.clone()),
        "FakeAppServer must receive provisioned cwd"
    );
    assert_eq!(
        *server.received_roots.lock(),
        vec![expected_cwd_str, expected_shared_str],
        "FakeAppServer must receive runtimeWorkspaceRoots"
    );
    assert!(
        expected_agent_dir.exists(),
        "agent workspace directory must exist"
    );
    assert!(
        expected_shared_dir.exists(),
        "shared workspace directory must exist"
    );

    let omo_json_path = expected_agent_dir.join(".omo").join("omo.json");
    assert!(omo_json_path.exists(), ".omo/omo.json must exist");
    let omo_json_str = std::fs::read_to_string(&omo_json_path).expect("read omo.json");
    let omo_json_val: Value = serde_json::from_str(&omo_json_str).expect("valid json in omo.json");
    assert_eq!(
        omo_json_val,
        json!({
            "memory": {
                "agent": "bot-1465631383862120451"
            }
        })
    );
}

#[tokio::test]
async fn test_omo_backend_omits_workspace_when_per_agent_disabled() {
    // Given: FakeAppServer and OmoBackend configured with workspace root but per_agent_workspace disabled
    let server = FakeAppServer::spawn().await;
    let url = format!("ws://127.0.0.1:{}", server.port);

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp_dir.path().to_path_buf();

    let config = OmoBackendConfig::new(&url)
        .with_workspace_root(workspace_root)
        .with_per_agent_workspace(false);
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-1",
    )
    .with_bot_id("1465631383862120451");
    let mut session = SessionContext::new(session_key.clone());

    // When: running one backend turn
    let event = InboundEvent::message(session_key.clone(), "msg-1", "Hello");
    let result = backend.run(&mut session, event).await;
    assert!(
        result.is_ok(),
        "backend.run must succeed: {:?}",
        result.err()
    );

    // Then: thread/start received no cwd or runtimeWorkspaceRoots
    assert_eq!(
        *server.received_cwd.lock(),
        None,
        "FakeAppServer must not receive cwd when kill-switch is disabled"
    );
    assert!(
        server.received_roots.lock().is_empty(),
        "FakeAppServer must not receive runtimeWorkspaceRoots when kill-switch is disabled"
    );
}

#[tokio::test]
async fn test_omo_backend_ignores_turn_completed_for_other_thread() {
    let port = spawn_cross_thread_completed_server().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(3));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    assert!(
        result.is_ok(),
        "a cross-thread turn/completed must not end this turn: {result:?}"
    );
    let chunks = dispatcher.stream_chunks();
    let final_chunk = chunks.last().expect("final chunk");
    assert!(final_chunk.is_final);
    assert_eq!(
        final_chunk.content, "Hello World!",
        "the turn must keep streaming until its own thread completes"
    );
}

#[tokio::test]
async fn test_omo_backend_ignores_turn_completed_without_content() {
    let port = spawn_empty_turn_completed_server().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_millis(500))
        .with_total_timeout(std::time::Duration::from_secs(5));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    // The premature empty terminal frame must NOT complete the turn as a
    // success (that recorded a digest delivery that never happened). The
    // stream then stalls and the connection/gap failure surfaces instead.
    assert!(
        result.is_err(),
        "empty-content terminal must not succeed: {:?}",
        result.ok()
    );
    assert!(
        dispatcher.stream_chunks().is_empty(),
        "no final chunk may be emitted for an empty turn"
    );
}

#[tokio::test]
async fn test_omo_backend_deadline_fires_on_ping_only_stream() {
    // Regression for the 1800s no-response hang: keepalive pings kept the
    // read loop awake while the daemon produced zero turn events, so the
    // whole-turn deadline only fired on text frames and never happened. The
    // deadline check must fire on non-text frames too.
    let port = spawn_pinging_idle_server().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(1))
        .with_total_timeout(std::time::Duration::from_millis(150));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "hello"),
        )
        .await;

    let err = result.expect_err("ping-only stream must hit the total deadline");
    assert!(
        err.to_string().contains("turn exceeded total deadline"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_omo_backend_empty_terminal_past_grace_fails_fast() {
    // Regression for the 53-minute silence: a daemon that records an upstream
    // LLM failure as an empty successful completion must surface right after
    // the grace window instead of stalling until the silence deadline.
    let port = spawn_delayed_empty_turn_completed_server(400).await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_no_content_grace(std::time::Duration::from_millis(100));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
    let mut session = SessionContext::new(session_key.clone());

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "hello"),
        )
        .await;

    let err = result.expect_err("empty terminal past grace must fail the turn");
    assert!(
        err.to_string().contains("no content"),
        "unexpected error: {err}"
    );
    assert!(
        dispatcher.stream_chunks().is_empty(),
        "no final chunk may be emitted for an empty turn"
    );
}

#[tokio::test]
async fn test_omo_backend_runs_ack_command_after_cron_delivery() {
    // Given: a cron session carrying an ack command, and a daemon whose turn
    // completes with content (successful final delivery)
    let server = FakeAppServer::spawn().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port))
        .with_request_timeout(std::time::Duration::from_secs(3));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone());

    let marker = tempfile::tempdir().expect("tempdir");
    let marker_path = marker.path().join("ack-ran");

    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());
    session
        .state
        .metadata
        .insert("cron_scheduler_delivery".into(), json!(true));
    session.state.metadata.insert(
        "cron_ack_command".into(),
        json!(format!(
            "touch '{}'",
            marker_path.to_string_lossy().replace('\\', "/")
        )),
    );

    // When: the turn completes
    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    // Then: the ack command executed after the successful delivery
    assert!(result.is_ok(), "turn must succeed: {:?}", result.err());
    assert!(
        marker_path.exists(),
        "ack command must run after successful cron delivery"
    );
}

#[tokio::test]
async fn test_omo_backend_skips_ack_when_delivery_fails() {
    // Given: a cron session with an ack command, but every delivery fails
    let server = FakeAppServer::spawn().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port))
        .with_request_timeout(std::time::Duration::from_secs(3));
    let dispatcher = Arc::new(FailingStreamDispatcher);
    let backend = OmoBackend::new(config, dispatcher);

    let marker = tempfile::tempdir().expect("tempdir");
    let marker_path = marker.path().join("ack-ran");

    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());
    session.state.metadata.insert(
        "cron_ack_command".into(),
        json!(format!(
            "touch '{}'",
            marker_path.to_string_lossy().replace('\\', "/")
        )),
    );

    // When: the turn completes against a failing egress
    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    // Then: delivery failure returns Err and the ack never ran — the
    // checkpoint must not commit for a delivery that did not happen.
    assert!(
        result.is_err(),
        "delivery failure must return Err on delivery failure"
    );
    assert!(
        !marker_path.exists(),
        "ack command must NOT run when delivery failed"
    );
}

/// Approval-request server that fires `count` execCommandApproval requests,
/// one per denial response received. Reproduces a policy-denial loop.
async fn spawn_repeated_approval_server(count: usize) -> u16 {
    use futures_util::{SinkExt, StreamExt};
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    tokio::spawn(async move {
        let Ok((stream, _)) = listener.accept().await else {
            return;
        };
        let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        let (mut sink, mut stream) = ws.split();
        let mut next_req_id = 1000usize;
        let mut pending_approvals: usize = 0;
        loop {
            let Some(Ok(Message::Text(text))) = stream.next().await else {
                break;
            };
            let Ok(request) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if request.get("method").is_none() {
                // A client response (e.g. an approval denial): fire the next
                // approval request while any remain.
                if pending_approvals > 0 {
                    pending_approvals -= 1;
                    next_req_id += 1;
                    let req = json!({
                        "jsonrpc": "2.0",
                        "id": next_req_id,
                        "method": "execCommandApproval",
                        "params": { "command": "rm -rf /", "reason": "policy probe" }
                    });
                    let _ = sink.send(Message::text(req.to_string())).await;
                }
                continue;
            }
            let method = request.get("method").and_then(Value::as_str).unwrap_or("");
            let id = request.get("id").and_then(Value::as_u64).unwrap_or(0);
            match method {
                "initialize" => {
                    let _ = sink
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                        ))
                        .await;
                }
                "thread/start" => {
                    let _ = sink
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"thread-1"}}})
                                .to_string(),
                        ))
                        .await;
                }
                "turn/start" => {
                    let _ = sink
                        .send(Message::text(
                            json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-1","status":"inProgress"}}})
                                .to_string(),
                        ))
                        .await;
                    // Fire the first approval request; the rest follow on each
                    // denial response.
                    let req = json!({
                        "jsonrpc": "2.0",
                        "id": 1000,
                        "method": "execCommandApproval",
                        "params": { "command": "rm -rf /", "reason": "policy probe" }
                    });
                    let _ = sink.send(Message::text(req.to_string())).await;
                    pending_approvals = count - 1;
                }
                _ => {}
            }
        }
    });
    port
}

#[tokio::test]
async fn test_omo_backend_aborts_turn_on_repeated_approval_denials() {
    // Given: a daemon stuck in a policy-denial loop (8 approval requests)
    let port = spawn_repeated_approval_server(8).await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(3));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher);
    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());

    // When: the denials accumulate
    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "do the thing"),
        )
        .await;

    // Then: the turn aborts with a policy error instead of flailing until
    // the whole-turn deadline.
    let error = result.expect_err("denial loop must abort the turn");
    assert!(
        error.to_string().contains("approval denied"),
        "unexpected error: {error}"
    );
}

// These separately selected cases exercise the exported API, not backend
// internals. Each invocation owns its runtime, socket, peer task and database.
#[test]
fn stop_before_start_ack_never_reports_uncancelled_success() {
    exercise_pending_start_stop("AG-01");
}

#[test]
fn pending_start_stop_public_surface() {
    exercise_pending_start_stop("AG-01-SURFACE");
}

fn exercise_pending_start_stop(case: &'static str) {
    use futures_util::FutureExt;
    use std::time::Duration;
    use tokio::time::timeout;

    let bound = Duration::from_secs(10);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("fixture runtime");
    let (outcome, peer_result, database_closed) = runtime.block_on(async {
        // Given: a subscribed peer-received signal before route, an isolated
        // database, and a daemon peer which never acknowledges request 3.
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let key =
            SessionKey::new("discord", None::<String>, "7", None::<String>, "42").with_bot_id("84");
        let event = InboundEvent::message(key.clone(), "ag01-probe", "probe");
        let expected_prompt = omon_gateway::models::render_user_prompt(&event);
        let config = OmoBackendConfig::new(format!("ws://{address}"))
            .with_request_timeout(Duration::from_secs(60));
        let dispatcher = Arc::new(CapturingDispatcher::new());
        let backend =
            Arc::new(OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone()));
        let multiplexer = SessionMultiplexer::with_dispatcher(
            db.pool().clone(),
            backend,
            Some(dispatcher),
            MultiplexerConfig::default(),
        );
        let (received_tx, received_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let remote_active = Arc::new(AtomicBool::new(false));
        let peer_active = remote_active.clone();
        let starts = Arc::new(AtomicUsize::new(0));
        let peer_starts = starts.clone();
        let mut peer = tokio::spawn(async move {
            let server = async move {
                let mut received_tx = Some(received_tx);
                let mut connection = 0;
                loop {
                    let (socket, _) = listener.accept().await.map_err(|e| e.to_string())?;
                    connection += 1;
                    let mut ws = tokio_tungstenite::accept_async(socket)
                        .await
                        .map_err(|e| e.to_string())?;
                    eprintln!("{case} connected={connection}");
                    while let Some(frame) = ws.next().await {
                        let frame = match frame {
                            Ok(frame) => frame,
                            Err(error) => {
                                // Dropping the run can reset TCP without a WS
                                // close handshake. Neither outcome cancels work.
                                eprintln!("{case} transport_closed={error}");
                                break;
                            }
                        };
                        let text = match frame {
                            Message::Text(text) => text,
                            Message::Close(frame) => {
                                eprintln!("{case} close_frame={frame:?}");
                                break;
                            }
                            other => {
                                eprintln!("{case} control_frame={other:?}");
                                continue;
                            }
                        };
                        eprintln!("{case} client->peer {text}");
                        let request: Value =
                            serde_json::from_str(&text).map_err(|e| e.to_string())?;
                        let response = match request["method"].as_str() {
                            Some("initialize") => {
                                json!({"jsonrpc":"2.0","id":request["id"],"result":{}})
                            }
                            Some("thread/start") | Some("thread/resume") => {
                                json!({
                                    "jsonrpc":"2.0","id":request["id"],
                                    "result":{"thread":{"id":"r1"}}
                                })
                            }
                            Some("turn/start") => {
                                peer_starts.fetch_add(1, Ordering::SeqCst);
                                if request["id"] != 3
                                    || request["params"]["threadId"] != "r1"
                                    || request["params"]["input"]
                                        != json!([{"type":"text","text":expected_prompt}])
                                {
                                    return Err(format!("invalid start request: {request}"));
                                }
                                peer_active.store(true, Ordering::SeqCst);
                                eprintln!("{case} ack_id=3 held remote_active=true");
                                received_tx
                                    .take()
                                    .ok_or_else(|| "accepted work was resubmitted".to_string())?
                                    .send(())
                                    .map_err(|_| "start subscriber dropped".to_string())?;
                                // No response or delta: only peer receipt
                                // establishes that stop may now be triggered.
                                continue;
                            }
                            Some("turn/interrupt") => {
                                if request["params"]["threadId"] != "r1"
                                    || request["params"]["turnId"] != "t1"
                                {
                                    return Err(format!("unmatched interrupt: {request}"));
                                }
                                // Only the supported, matching interrupt
                                // terminates this peer's persistent work.
                                peer_active.store(false, Ordering::SeqCst);
                                json!({"jsonrpc":"2.0","id":request["id"],"result":{}})
                            }
                            _ => return Err(format!("unsupported request: {request}")),
                        };
                        eprintln!("{case} peer->client {response}");
                        ws.send(Message::text(response.to_string()))
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    eprintln!(
                        "{case} disconnected={connection} remote_active={}",
                        peer_active.load(Ordering::SeqCst)
                    );
                    // Work belongs to the peer, not this connection. Accept a
                    // reconnect without clearing it or resubmitting turn/start.
                }
            };
            tokio::select! {
                result = server => result,
                shutdown = shutdown_rx => {
                    shutdown.map_err(|_| "fixture shutdown sender dropped".to_string())
                }
            }
        });

        let outcome = std::panic::AssertUnwindSafe(async {
            // When: route through the public API, then stop only after the
            // peer has accepted work with its start ACK still withheld.
            eprintln!("{case} route dm channel=7 user=42 bot=84 content=probe");
            timeout(bound, multiplexer.route(event))
                .await
                .expect("bounded route")
                .expect("route admitted");
            timeout(bound, received_rx)
                .await
                .expect("bounded peer-received signal")
                .expect("peer received turn/start");
            let stopped = timeout(bound, multiplexer.stop(&key))
                .await
                .expect("stop must produce a result, not time out");
            let active_at_stop = remote_active.load(Ordering::SeqCst);
            eprintln!("{case} stop={stopped:?} remote_active={active_at_stop}");

            // Then: a successful cancellation cannot leave peer work alive.
            // Only a backend cancellation error is allowed, not a database or
            // multiplexer/runtime failure; an outer timeout is not an outcome.
            assert!(
                !(matches!(&stopped, Ok(true)) && active_at_stop),
                "stop returned success for the active turn while remote work remains active"
            );
            assert!(
                !matches!(&stopped, Ok(false)),
                "the peer accepted an active turn; stop must not report an idle session"
            );
            assert!(
                matches!(&stopped, Ok(true) | Err(OmonError::Llm(_))),
                "expected cancellation success or a backend cancellation error, got {stopped:?}"
            );
            assert_eq!(
                starts.load(Ordering::SeqCst),
                1,
                "do not resubmit accepted work"
            );
        })
        .catch_unwind()
        .await;

        // Cleanup runs even when the behavioral assertion panics. No fixture
        // task is detached; a wedged peer is aborted and then joined.
        if shutdown_tx.send(()).is_err() {
            eprintln!("{case} peer exited before fixture shutdown; inspecting join result");
        }
        let peer_result = match timeout(bound, &mut peer).await {
            Ok(joined) => joined
                .map_err(|e| format!("peer task failed: {e}"))
                .and_then(|result| result),
            Err(error) => {
                peer.abort();
                let joined = peer.await;
                Err(format!(
                    "peer cleanup timed out: {error}; abort join={joined:?}"
                ))
            }
        };
        drop(multiplexer);
        let database_closed = timeout(bound, db.close()).await;
        eprintln!("{case} cleanup peer={peer_result:?} database_closed={database_closed:?}");
        (outcome, peer_result, database_closed)
    });
    // The exported mux has no actor join API. Its entire runtime is owned by
    // this fixture, so internal actor tasks cannot outlive either invocation.
    drop(runtime);
    eprintln!("{case} cleanup runtime_dropped=true no_fixture_files=true");
    peer_result.expect("peer protocol and cleanup succeeded");
    database_closed.expect("fixture database closed");
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn stop_interrupts_remote_turn_before_ack() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let (turn_ack_tx, mut turn_ack_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (interrupt_seen_tx, _interrupt_seen_rx) = tokio::sync::oneshot::channel::<()>();
    let interrupt_seen_tx = Arc::new(tokio::sync::Mutex::new(Some(interrupt_seen_tx)));

    let interrupt_received = Arc::new(AtomicBool::new(false));
    let tool_completed = Arc::new(AtomicBool::new(false));
    let peer_kept_work_alive = Arc::new(AtomicBool::new(false));

    let ir_clone = interrupt_received.clone();
    let _tc_clone = tool_completed.clone();
    let pk_clone = peer_kept_work_alive.clone();

    let peer_handle = tokio::spawn(async move {
        let mut turn_in_progress = false;

        while let Ok((socket, _)) = listener.accept().await {
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            while let Some(msg) = ws.next().await {
                let Ok(Message::Text(text)) = msg else {
                    continue;
                };
                let Ok(req) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                let method = req.get("method").and_then(Value::as_str).unwrap_or("");

                match method {
                    "initialize" => {
                        let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                        let _ = ws.send(Message::text(resp.to_string())).await;
                    }
                    "thread/resume" | "thread/start" => {
                        let resp = json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"r1"}}});
                        let _ = ws.send(Message::text(resp.to_string())).await;
                    }
                    "turn/start" => {
                        assert_eq!(req["params"]["threadId"], "r1");
                        turn_in_progress = true;
                        let resp = json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"t1","status":"inProgress"}}});
                        let _ = ws.send(Message::text(resp.to_string())).await;
                        let delta = json!({
                            "jsonrpc": "2.0",
                            "method": "item/agentMessage/delta",
                            "params": {
                                "threadId": "r1",
                                "turnId": "t1",
                                "itemId": "m1",
                                "delta": "WORK_STARTED"
                            }
                        });
                        let _ = ws.send(Message::text(delta.to_string())).await;
                    }
                    "turn/interrupt" => {
                        assert_eq!(
                            req["params"]["threadId"], "r1",
                            "interrupt threadId must match r1"
                        );
                        assert_eq!(
                            req["params"]["turnId"], "t1",
                            "interrupt turnId must match t1"
                        );
                        ir_clone.store(true, Ordering::SeqCst);
                        turn_in_progress = false;

                        let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                        let _ = ws.send(Message::text(resp.to_string())).await;

                        let terminal = json!({
                            "jsonrpc":"2.0",
                            "method":"turn/completed",
                            "params":{"threadId":"r1","turn":{"id":"t1","status":"interrupted"}}
                        });
                        let _ = ws.send(Message::text(terminal.to_string())).await;

                        if let Some(tx) = interrupt_seen_tx.lock().await.take() {
                            let _ = tx.send(());
                        }
                    }
                    _ => {}
                }
            }
            if turn_in_progress {
                pk_clone.store(true, Ordering::SeqCst);
            }
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let key = SessionKey::new(
        "discord",
        Some("guild-u13"),
        "channel-u13",
        None::<String>,
        "user-u13",
    );
    let mut initial_session = SessionContext::new(key.clone());
    initial_session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'discord', 'guild-u13', 'channel-u13', 'user-u13', ?)"
    )
    .bind(key.storage_key())
    .bind(serde_json::to_string(&initial_session.state).unwrap())
    .execute(db.pool())
    .await
    .unwrap();

    let config = OmoBackendConfig::new(format!("ws://{address}"))
        .with_request_timeout(std::time::Duration::from_secs(5));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    *dispatcher.turn_started_tx.lock() = Some(turn_ack_tx);
    let backend =
        Arc::new(OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone()));
    let multiplexer = SessionMultiplexer::with_dispatcher(
        db.pool().clone(),
        backend.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
    );

    let event = InboundEvent::message(key.clone(), "msg-u13", "execute long turn");
    multiplexer.route(event).await.unwrap();

    let received_chunk =
        tokio::time::timeout(std::time::Duration::from_secs(5), turn_ack_rx.recv())
            .await
            .expect("bounded wait for turn start ack")
            .expect("turn ack received");
    assert_eq!(received_chunk, "WORK_STARTED");

    let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), multiplexer.stop(&key))
        .await
        .expect("bounded stop")
        .expect("stop succeeds");

    assert!(stopped, "multiplexer.stop must return true for active turn");
    assert!(
        interrupt_received.load(Ordering::SeqCst),
        "peer must receive turn/interrupt before stop reports success"
    );
    assert!(
        peer_kept_work_alive.load(Ordering::SeqCst),
        "peer must keep work alive across disconnect until interrupt"
    );
    assert!(
        !tool_completed.load(Ordering::SeqCst),
        "no post-stop tool completion"
    );

    let row: (String,) = sqlx::query_as("SELECT state_json FROM sessions WHERE session_key = ?")
        .bind(key.storage_key())
        .fetch_one(db.pool())
        .await
        .unwrap();
    let loaded_state: omon_gateway::SessionState = serde_json::from_str(&row.0).unwrap();
    assert_eq!(
        loaded_state
            .metadata
            .get("omo_thread_id")
            .and_then(Value::as_str),
        Some("r1"),
        "sticky binding to r1 must be preserved"
    );

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
}

#[tokio::test]
async fn stop_interrupts_remote_turn() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let interrupt_a_seen = Arc::new(AtomicBool::new(false));
    let ia_clone = interrupt_a_seen.clone();

    let (b_release_tx, b_release_rx) = tokio::sync::oneshot::channel::<()>();
    let b_release_rx = Arc::new(tokio::sync::Mutex::new(Some(b_release_rx)));

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            let ia = ia_clone.clone();
            let b_rx = b_release_rx.clone();

            tokio::spawn(async move {
                while let Some(msg) = ws.next().await {
                    let Ok(Message::Text(text)) = msg else {
                        continue;
                    };
                    let Ok(req) = serde_json::from_str::<Value>(&text) else {
                        continue;
                    };
                    let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

                    match method {
                        "initialize" => {
                            let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                            let _ = ws.send(Message::text(resp.to_string())).await;
                        }
                        "thread/resume" | "thread/start" => {
                            let thread_id = req["params"]["threadId"].as_str().unwrap_or("r1");
                            let resp = json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":thread_id}}});
                            let _ = ws.send(Message::text(resp.to_string())).await;
                        }
                        "turn/start" => {
                            let thread_id = req["params"]["threadId"].as_str().unwrap_or("");
                            let turn_id = if thread_id == "r1" { "t1" } else { "t2" };
                            let resp = json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":turn_id,"status":"inProgress"}}});
                            let _ = ws.send(Message::text(resp.to_string())).await;
                            let delta_text = if thread_id == "r1" {
                                "LANE_A_ACTIVE"
                            } else {
                                "LANE_B_ACTIVE"
                            };
                            let delta = json!({
                                "jsonrpc": "2.0",
                                "method": "item/agentMessage/delta",
                                "params": {
                                    "threadId": thread_id,
                                    "turnId": turn_id,
                                    "itemId": "m1",
                                    "delta": delta_text
                                }
                            });
                            let _ = ws.send(Message::text(delta.to_string())).await;

                            if thread_id != "r1" {
                                let rx = b_rx.lock().await.take();
                                if let Some(rx) = rx {
                                    let _ = rx.await;
                                }
                                let completed = json!({
                                    "jsonrpc":"2.0","method":"turn/completed",
                                    "params":{"threadId":"r2","turn":{"id":"t2","status":"completed"}}
                                });
                                let _ = ws.send(Message::text(completed.to_string())).await;
                            }
                        }
                        "turn/interrupt" => {
                            let thread_id = req["params"]["threadId"].as_str().unwrap_or("");
                            let turn_id = req["params"]["turnId"].as_str().unwrap_or("");
                            if thread_id == "r1" && turn_id == "t1" {
                                ia.store(true, Ordering::SeqCst);
                                let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                let _ = ws.send(Message::text(resp.to_string())).await;
                                let terminal = json!({
                                    "jsonrpc":"2.0","method":"turn/completed",
                                    "params":{"threadId":"r1","turn":{"id":"t1","status":"interrupted"}}
                                });
                                let _ = ws.send(Message::text(terminal.to_string())).await;
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let key_a = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        None::<String>,
        "user-a",
    );
    let key_b = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-2",
        None::<String>,
        "user-b",
    );

    let mut session_a = SessionContext::new(key_a.clone());
    session_a
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));
    let mut session_b = SessionContext::new(key_b.clone());
    session_b
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r2"));

    for (k, s) in [(&key_a, &session_a), (&key_b, &session_b)] {
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'discord', 'guild-1', ?, ?, ?)"
        )
        .bind(k.storage_key())
        .bind(&k.channel_id)
        .bind(&k.user_id)
        .bind(serde_json::to_string(&s.state).unwrap())
        .execute(db.pool())
        .await
        .unwrap();
    }

    let config = OmoBackendConfig::new(format!("ws://{address}"))
        .with_request_timeout(std::time::Duration::from_secs(5));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    *dispatcher.turn_started_tx.lock() = Some(chunk_tx);
    let backend =
        Arc::new(OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone()));
    let multiplexer = SessionMultiplexer::with_dispatcher(
        db.pool().clone(),
        backend.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
    );

    multiplexer
        .route(InboundEvent::message(key_a.clone(), "msg-a", "turn A"))
        .await
        .unwrap();
    multiplexer
        .route(InboundEvent::message(key_b.clone(), "msg-b", "turn B"))
        .await
        .unwrap();

    let mut seen_a = false;
    let mut seen_b = false;
    while !seen_a || !seen_b {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), chunk_rx.recv())
            .await
            .expect("bounded delta recv")
            .expect("delta received");
        if chunk == "LANE_A_ACTIVE" {
            seen_a = true;
        } else if chunk == "LANE_B_ACTIVE" {
            seen_b = true;
        }
    }

    let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), multiplexer.stop(&key_a))
        .await
        .expect("bounded stop")
        .expect("stop succeeds");

    assert!(stopped);
    assert!(
        interrupt_a_seen.load(Ordering::SeqCst),
        "interrupt sent for lane A"
    );

    let actions = dispatcher.actions.lock().clone();
    let typing_a: Vec<bool> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::Typing { session, active } if session == &key_a => Some(*active),
            _ => None,
        })
        .collect();
    let typing_b: Vec<bool> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::Typing { session, active } if session == &key_b => Some(*active),
            _ => None,
        })
        .collect();

    assert!(typing_a.contains(&true), "lane A started typing");
    assert_eq!(
        typing_a.last(),
        Some(&false),
        "lane A stopped and released typing"
    );
    assert!(typing_b.contains(&true), "lane B started typing");
    assert_eq!(
        typing_b.last(),
        Some(&true),
        "lane B typing still active while lane A stopped"
    );

    let _ = b_release_tx.send(());

    let row_a: (String,) = sqlx::query_as("SELECT state_json FROM sessions WHERE session_key = ?")
        .bind(key_a.storage_key())
        .fetch_one(db.pool())
        .await
        .unwrap();
    let state_a: omon_gateway::SessionState = serde_json::from_str(&row_a.0).unwrap();
    assert_eq!(
        state_a
            .metadata
            .get("omo_thread_id")
            .and_then(Value::as_str),
        Some("r1")
    );

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseToBlock {
    Connect,
    Initialize,
    ThreadStart,
    ThreadResume,
    TurnStartAck,
    Streaming,
    RetryInitialize,
    SmallerConfiguredCap,
}

async fn run_deadline_test_for_phase(phase: PhaseToBlock) {
    eprintln!("Phase: {phase:?}");
    match phase {
        PhaseToBlock::Connect => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_connect_timeout(std::time::Duration::from_secs(2400))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            let _ = futures_util::poll!(&mut run);

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1801)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout connecting"),
                        "unexpected error: {msg}"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("blocked connect must not succeed"),
                std::task::Poll::Pending => {
                    panic!("RED run remains pending with a renewed setup budget (connect)");
                }
            }
        }
        PhaseToBlock::Initialize => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (init_received_tx, mut init_received_rx) = tokio::sync::mpsc::channel(1);
            let turn_start_count = Arc::new(AtomicUsize::new(0));
            let tsc = turn_start_count.clone();

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            if method == "initialize" {
                                let _ = ws
                                    .send(Message::text(
                                        json!({"jsonrpc":"2.0","method":"system/ping"}).to_string(),
                                    ))
                                    .await;
                                let _ = init_received_tx.send(()).await;
                                std::future::pending::<()>().await;
                            } else if method == "turn/start" {
                                tsc.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), init_received_rx.recv()) => {
                    res.expect("bounded wait").expect("initialize received by server");
                }
            }

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1801)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout waiting for initialize"),
                        "unexpected error: {msg}"
                    );
                    assert_eq!(
                        turn_start_count.load(Ordering::SeqCst),
                        0,
                        "no turn/start after expiry"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("blocked initialize must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::ThreadStart => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (start_received_tx, mut start_received_rx) = tokio::sync::mpsc::channel(1);
            let turn_start_count = Arc::new(AtomicUsize::new(0));
            let tsc = turn_start_count.clone();

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            match method {
                                "initialize" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "thread/start" => {
                                    let _ = ws
                                        .send(Message::text(
                                            json!({"jsonrpc":"2.0","method":"system/info"})
                                                .to_string(),
                                        ))
                                        .await;
                                    let _ = start_received_tx.send(()).await;
                                    std::future::pending::<()>().await;
                                }
                                "turn/start" => {
                                    tsc.fetch_add(1, Ordering::SeqCst);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), start_received_rx.recv()) => {
                    res.expect("bounded wait").expect("thread/start received by server");
                }
            }

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1801)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout waiting for thread/start"),
                        "unexpected error: {msg}"
                    );
                    assert_eq!(
                        turn_start_count.load(Ordering::SeqCst),
                        0,
                        "no turn/start after expiry"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("blocked thread/start must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget (thread/start)");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::ThreadResume => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (resume_received_tx, mut resume_received_rx) = tokio::sync::mpsc::channel(1);
            let turn_start_count = Arc::new(AtomicUsize::new(0));
            let tsc = turn_start_count.clone();

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            match method {
                                "initialize" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "thread/resume" => {
                                    let _ = ws
                                        .send(Message::text(
                                            json!({"jsonrpc":"2.0","method":"system/info"})
                                                .to_string(),
                                        ))
                                        .await;
                                    let _ = resume_received_tx.send(()).await;
                                    std::future::pending::<()>().await;
                                }
                                "turn/start" => {
                                    tsc.fetch_add(1, Ordering::SeqCst);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            session
                .state
                .metadata
                .insert("omo_thread_id".into(), json!("existing-thread-id"));
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), resume_received_rx.recv()) => {
                    res.expect("bounded wait").expect("thread/resume received by server");
                }
            }

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1801)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout waiting for thread/resume"),
                        "unexpected error: {msg}"
                    );
                    assert_eq!(
                        turn_start_count.load(Ordering::SeqCst),
                        0,
                        "no turn/start after expiry"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("blocked thread/resume must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget (thread/resume)");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::TurnStartAck => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (turn_start_tx, mut turn_start_rx) = tokio::sync::mpsc::channel(1);

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            match method {
                                "initialize" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "thread/start" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"t1"}}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "turn/start" => {
                                    let _ = ws
                                        .send(Message::text(
                                            json!({"jsonrpc":"2.0","method":"system/ping"})
                                                .to_string(),
                                        ))
                                        .await;
                                    let _ = turn_start_tx.send(()).await;
                                    std::future::pending::<()>().await;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(600))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), turn_start_rx.recv()) => {
                    res.expect("bounded wait").expect("turn/start received by server");
                }
            }

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(31)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn/start acknowledgement")
                            || msg.contains("turn exceeded total deadline"),
                        "unexpected error: {msg}"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("un-ACKed turn/start must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget (turn/start ACK)");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::Streaming => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (interrupt_tx, interrupt_rx) = tokio::sync::oneshot::channel::<()>();
            let interrupt_seen = Arc::new(AtomicBool::new(false));
            let is_clone = interrupt_seen.clone();
            let interrupt_tx = Arc::new(tokio::sync::Mutex::new(Some(interrupt_tx)));
            let itx_clone = interrupt_tx.clone();

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            match method {
                                "initialize" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "thread/start" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"t1"}}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "turn/start" => {
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{"turn":{"id":"turn-active","status":"inProgress"}}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let started = json!({
                                        "jsonrpc":"2.0",
                                        "method":"turn/started",
                                        "params":{"threadId":"t1","turnId":"turn-active"}
                                    });
                                    let _ = ws.send(Message::text(started.to_string())).await;
                                    let delta = json!({
                                        "jsonrpc":"2.0",
                                        "method":"item/agentMessage/delta",
                                        "params":{"threadId":"t1","turnId":"turn-active","itemId":"m1","delta":"STREAMING"}
                                    });
                                    let _ = ws.send(Message::text(delta.to_string())).await;
                                }
                                "turn/interrupt" => {
                                    is_clone.store(true, Ordering::SeqCst);
                                    if let Some(tx) = itx_clone.lock().await.take() {
                                        let _ = tx.send(());
                                    }
                                    let resp = json!({"jsonrpc":"2.0","id":id,"result":{}});
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let completed = json!({
                                        "jsonrpc":"2.0",
                                        "method":"turn/completed",
                                        "params":{"threadId":"t1","turn":{"id":"turn-active","status":"interrupted"}}
                                    });
                                    let _ = ws.send(Message::text(completed.to_string())).await;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let (stream_started_tx, mut stream_started_rx) = tokio::sync::mpsc::unbounded_channel();
            let dispatcher = Arc::new(CapturingDispatcher::new());
            *dispatcher.turn_started_tx.lock() = Some(stream_started_tx);
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), stream_started_rx.recv()) => {
                    res.expect("bounded wait").expect("streaming chunk received");
                }
            }
            assert!(
                !backend.active_turns.lock().is_empty(),
                "turn must be active"
            );

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(1801)).await;
            let mut poll_result = futures_util::poll!(&mut run);
            if poll_result.is_pending() {
                tokio::time::advance(std::time::Duration::from_secs(5)).await;
                poll_result = futures_util::poll!(&mut run);
            }
            tokio::time::resume();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), interrupt_rx).await;

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline"),
                        "expected total deadline message: {msg}"
                    );
                    assert!(
                        interrupt_seen.load(Ordering::SeqCst),
                        "bounded interrupt cleanup must occur for active turn"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("streaming hang must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget (streaming)");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::RetryInitialize => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (first_init_tx, mut first_init_rx) = tokio::sync::mpsc::channel(1);
            let (trigger_reset_tx, mut trigger_reset_rx) = tokio::sync::mpsc::channel(1);
            let (second_init_tx, mut second_init_rx) = tokio::sync::mpsc::channel(1);
            let turn_start_count = Arc::new(AtomicUsize::new(0));
            let tsc = turn_start_count.clone();

            let peer_handle = tokio::spawn(async move {
                let mut conn_idx = 0;
                while let Ok((socket, _)) = listener.accept().await {
                    conn_idx += 1;
                    let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                        return;
                    };
                    if conn_idx == 1 {
                        while let Some(Ok(msg)) = ws.next().await {
                            if let Message::Text(text) = msg {
                                if let Ok(req) = serde_json::from_str::<Value>(&text) {
                                    let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                                    let method =
                                        req.get("method").and_then(Value::as_str).unwrap_or("");
                                    if method == "initialize" {
                                        let _ = first_init_tx.send(()).await;
                                        let _ = trigger_reset_rx.recv().await;
                                        let err_resp = json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {
                                                "code": -32603,
                                                "message": "Connection reset"
                                            }
                                        });
                                        let _ = ws.send(Message::text(err_resp.to_string())).await;
                                        let _ = ws.close(None).await;
                                        break;
                                    }
                                }
                            }
                        }
                    } else if conn_idx == 2 {
                        while let Some(Ok(msg)) = ws.next().await {
                            if let Message::Text(text) = msg {
                                if let Ok(req) = serde_json::from_str::<Value>(&text) {
                                    let method =
                                        req.get("method").and_then(Value::as_str).unwrap_or("");
                                    if method == "initialize" {
                                        let _ = second_init_tx.send(()).await;
                                        std::future::pending::<()>().await;
                                    } else if method == "turn/start" {
                                        tsc.fetch_add(1, Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(2400))
                .with_total_timeout(std::time::Duration::from_secs(1800));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), first_init_rx.recv()) => {
                    res.expect("bounded wait").expect("first initialize received");
                }
            }

            tokio::time::pause();
            // Spend 1500s in first initialize
            tokio::time::advance(std::time::Duration::from_secs(1500)).await;
            let _ = futures_util::poll!(&mut run);

            // Signal connection reset to trigger retry
            let _ = trigger_reset_tx.send(()).await;
            let _ = futures_util::poll!(&mut run);

            // 500ms cooldown
            tokio::time::advance(std::time::Duration::from_millis(500)).await;
            let _ = futures_util::poll!(&mut run);

            tokio::time::resume();
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), second_init_rx.recv()) => {
                    res.expect("bounded wait").expect("second initialize received after cooldown");
                }
            }
            tokio::time::pause();

            // Advance to original start + 1801s (1500s + 0.5s + 300.5s = 1801s)
            tokio::time::advance(std::time::Duration::from_millis(300_500)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout waiting for initialize"),
                        "unexpected error: {msg}"
                    );
                    assert_eq!(
                        turn_start_count.load(Ordering::SeqCst),
                        0,
                        "no turn/start after expiry"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("retry hang must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
        PhaseToBlock::SmallerConfiguredCap => {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let (init_tx, mut init_rx) = tokio::sync::mpsc::channel(1);

            let peer_handle = tokio::spawn(async move {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            if method == "initialize" {
                                let _ = init_tx.send(()).await;
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                }
            });

            let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
                .with_request_timeout(std::time::Duration::from_secs(600))
                .with_total_timeout(std::time::Duration::from_secs(45));
            let dispatcher = Arc::new(CapturingDispatcher::new());
            let backend = OmoBackend::new(config, dispatcher);
            let session_key =
                SessionKey::new("discord", None::<String>, "chan", None::<String>, "user");
            let mut session = SessionContext::new(session_key.clone());
            let event = InboundEvent::message(session_key, "msg", "hello");

            let mut run = std::pin::pin!(backend.run(&mut session, event));
            tokio::select! {
                res = &mut run => panic!("unexpected termination: {res:?}"),
                res = tokio::time::timeout(std::time::Duration::from_secs(5), init_rx.recv()) => {
                    res.expect("bounded wait").expect("initialize received");
                }
            }

            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(46)).await;
            let poll_result = futures_util::poll!(&mut run);
            tokio::time::resume();

            match poll_result {
                std::task::Poll::Ready(Err(err)) => {
                    let msg = err.to_string();
                    assert!(
                        msg.contains("turn exceeded total deadline")
                            || msg.contains("timeout waiting for initialize"),
                        "unexpected error: {msg}"
                    );
                }
                std::task::Poll::Ready(Ok(_)) => panic!("smaller cap must not succeed"),
                std::task::Poll::Pending => {
                    peer_handle.abort();
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
                    panic!("RED run remains pending with a renewed setup budget (smaller cap)");
                }
            }
            peer_handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;
        }
    }
}

#[tokio::test]
async fn interactive_deadline_bounds_all_protocol_phases() {
    for phase in [
        PhaseToBlock::Connect,
        PhaseToBlock::Initialize,
        PhaseToBlock::ThreadStart,
        PhaseToBlock::ThreadResume,
        PhaseToBlock::TurnStartAck,
        PhaseToBlock::Streaming,
        PhaseToBlock::RetryInitialize,
        PhaseToBlock::SmallerConfiguredCap,
    ] {
        run_deadline_test_for_phase(phase).await;
    }
}

#[tokio::test]
async fn accepted_disconnect_does_not_resubmit_turn() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let side_effect_counter = Arc::new(AtomicUsize::new(0));
    let connection_count = Arc::new(AtomicUsize::new(0));

    let tsc = turn_start_count.clone();
    let sec = side_effect_counter.clone();
    let cc = connection_count.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let conn_idx = cc.fetch_add(1, Ordering::SeqCst);
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<Value>(&text) {
                        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                        match method {
                            "initialize" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "capabilities": {} }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/start" | "thread/resume" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "thread-u15" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "turn/start" => {
                                tsc.fetch_add(1, Ordering::SeqCst);
                                sec.fetch_add(1, Ordering::SeqCst);
                                if conn_idx == 0 {
                                    // Connection 1: ACK the turn start, then abruptly close stream before terminal
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "turn-1",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let delta = json!({
                                        "jsonrpc": "2.0",
                                        "method": "item/agentMessage/delta",
                                        "params": {
                                            "threadId": "thread-u15",
                                            "turnId": "turn-1",
                                            "itemId": "item-1",
                                            "delta": "in-flight output"
                                        }
                                    });
                                    let _ = ws.send(Message::text(delta.to_string())).await;
                                    // Close connection without terminal turn/completed
                                    let _ = ws.close(None).await;
                                    break;
                                } else {
                                    // Connection 2 (if client resubmits after reconnect):
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "turn-2",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let terminal = json!({
                                        "jsonrpc": "2.0",
                                        "method": "turn/completed",
                                        "params": {
                                            "threadId": "thread-u15",
                                            "turn": {
                                                "id": "turn-2",
                                                "status": "completed"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(terminal.to_string())).await;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u15"),
        "channel-u15",
        None::<String>,
        "user-u15",
    );
    let mut session = SessionContext::new(session_key.clone());
    let event = InboundEvent::message(session_key, "msg-u15", "mutating task with side effects");

    let result = backend.run(&mut session, event).await;

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert_eq!(
        turn_start_count.load(Ordering::SeqCst),
        1,
        "accepted turn must not be resubmitted after disconnect; turn/start must be sent exactly once"
    );
    assert_eq!(
        side_effect_counter.load(Ordering::SeqCst),
        1,
        "daemon side effect must occur exactly once, not duplicated across reconnect"
    );
    assert!(
        result.is_err(),
        "backend.run must fail on accepted disconnect rather than fabricating success or resubmitting"
    );
}

#[tokio::test]
async fn pre_ack_disconnect_does_not_resubmit_turn() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let side_effect_counter = Arc::new(AtomicUsize::new(0));
    let connection_count = Arc::new(AtomicUsize::new(0));

    let tsc = turn_start_count.clone();
    let sec = side_effect_counter.clone();
    let cc = connection_count.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let conn_idx = cc.fetch_add(1, Ordering::SeqCst);
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<Value>(&text) {
                        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                        match method {
                            "initialize" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "capabilities": {} }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/start" | "thread/resume" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "thread-u15-preack" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "turn/start" => {
                                tsc.fetch_add(1, Ordering::SeqCst);
                                sec.fetch_add(1, Ordering::SeqCst);
                                if conn_idx == 0 {
                                    // Connection 1: Close connection immediately without ACK
                                    let _ = ws.close(None).await;
                                    break;
                                } else {
                                    // Connection 2 (if resubmitted):
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "turn-2",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let terminal = json!({
                                        "jsonrpc": "2.0",
                                        "method": "turn/completed",
                                        "params": {
                                            "threadId": "thread-u15-preack",
                                            "turn": {
                                                "id": "turn-2",
                                                "status": "completed"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(terminal.to_string())).await;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u15-preack"),
        "channel-u15-preack",
        None::<String>,
        "user-u15-preack",
    );
    let mut session = SessionContext::new(session_key.clone());
    let event = InboundEvent::message(session_key, "msg-u15-preack", "ambiguous pre-ack submit");

    let result = backend.run(&mut session, event).await;

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert_eq!(
        turn_start_count.load(Ordering::SeqCst),
        1,
        "pre-ack submitted turn must not be resubmitted; turn/start must be sent exactly once"
    );
    assert_eq!(
        side_effect_counter.load(Ordering::SeqCst),
        1,
        "daemon side effect must occur exactly once, even on ambiguous pre-ack disconnect"
    );
    assert!(
        result.is_err(),
        "backend.run must report unknown/failed outcome without replaying"
    );
}

#[tokio::test]
async fn pre_submit_disconnect_retries_and_succeeds() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let connection_count = Arc::new(AtomicUsize::new(0));

    let tsc = turn_start_count.clone();
    let cc = connection_count.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let conn_idx = cc.fetch_add(1, Ordering::SeqCst);
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            if conn_idx == 0 {
                // Connection 1: Abruptly close during initialize (pre-submit)
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            if method == "initialize" {
                                let _ = ws.close(None).await;
                                break;
                            }
                        }
                    }
                }
            } else {
                // Connection 2: Serve the full flow
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(req) = serde_json::from_str::<Value>(&text) {
                            let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                            let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                            match method {
                                "initialize" => {
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": { "capabilities": {} }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "thread/start" | "thread/resume" => {
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": { "thread": { "id": "thread-u15-presubmit" } }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                                "turn/start" => {
                                    tsc.fetch_add(1, Ordering::SeqCst);
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "turn-1",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                    let delta = json!({
                                        "jsonrpc": "2.0",
                                        "method": "item/agentMessage/delta",
                                        "params": {
                                            "threadId": "thread-u15-presubmit",
                                            "turnId": "turn-1",
                                            "itemId": "item-1",
                                            "delta": "success after retry"
                                        }
                                    });
                                    let _ = ws.send(Message::text(delta.to_string())).await;
                                    let terminal = json!({
                                        "jsonrpc": "2.0",
                                        "method": "turn/completed",
                                        "params": {
                                            "threadId": "thread-u15-presubmit",
                                            "turn": {
                                                "id": "turn-1",
                                                "status": "completed"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(terminal.to_string())).await;
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let dispatcher = Arc::new(CapturingDispatcher::new());
    let backend = OmoBackend::new(config, dispatcher.clone()).with_pool(db.pool().clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u15-presubmit"),
        "channel-u15-presubmit",
        None::<String>,
        "user-u15-presubmit",
    );
    let mut session = SessionContext::new(session_key.clone());
    let event = InboundEvent::message(session_key, "msg-u15-presubmit", "pre-submit retry task");

    let result = backend.run(&mut session, event).await;

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert!(
        result.is_ok(),
        "pre-submit connection failure must retry and succeed: {:?}",
        result.err()
    );
    assert_eq!(
        turn_start_count.load(Ordering::SeqCst),
        1,
        "turn/start must be executed exactly once after pre-submit retry"
    );
    assert_eq!(
        connection_count.load(Ordering::SeqCst),
        2,
        "client must have reconnected after pre-submit failure"
    );
}

struct BindingTestDispatcher {
    action_tx: tokio::sync::mpsc::UnboundedSender<OutboundAction>,
    actions: ParkingMutex<Vec<OutboundAction>>,
}

impl BindingTestDispatcher {
    fn new(action_tx: tokio::sync::mpsc::UnboundedSender<OutboundAction>) -> Self {
        Self {
            action_tx,
            actions: ParkingMutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl OutboundDispatcher for BindingTestDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        let _ = self.action_tx.send(action.clone());
        self.actions.lock().push(action);
        Ok(())
    }
}

#[tokio::test]
async fn failed_first_turn_keeps_durable_binding() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let thread_start_count = Arc::new(AtomicUsize::new(0));
    let thread_resume_count = Arc::new(AtomicUsize::new(0));
    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let transcript = Arc::new(ParkingMutex::new(Vec::<String>::new()));

    let tsc = thread_start_count.clone();
    let trc = thread_resume_count.clone();
    let turn_count = turn_start_count.clone();
    let tr = transcript.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<Value>(&text) {
                        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                        match method {
                            "initialize" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "capabilities": {} }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/start" => {
                                tsc.fetch_add(1, Ordering::SeqCst);
                                tr.lock().push("thread/start".to_string());
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "r1" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/resume" => {
                                let tid = req
                                    .pointer("/params/threadId")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                                trc.fetch_add(1, Ordering::SeqCst);
                                tr.lock().push(format!("thread/resume:{tid}"));
                                if tid == "r-missing" {
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                            "code": -32603,
                                            "message": "no rollout found for thread id r-missing"
                                        }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                } else {
                                    let resp = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": { "thread": { "id": tid } }
                                    });
                                    let _ = ws.send(Message::text(resp.to_string())).await;
                                }
                            }
                            "turn/start" => {
                                let count = turn_count.fetch_add(1, Ordering::SeqCst);
                                tr.lock().push("turn/start".to_string());
                                if count == 0 {
                                    // Turn 1 fails
                                    let ack = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": "t1",
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(ack.to_string())).await;
                                    let terminal = json!({
                                        "jsonrpc": "2.0",
                                        "method": "turn/completed",
                                        "params": {
                                            "threadId": "r1",
                                            "turn": {
                                                "id": "t1",
                                                "status": "failed",
                                                "error": { "message": "simulated turn 1 failure" }
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(terminal.to_string())).await;
                                } else {
                                    // Subsequent turns succeed
                                    let turn_id = format!("t{}", count + 1);
                                    let ack = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "turn": {
                                                "id": turn_id,
                                                "status": "inProgress"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(ack.to_string())).await;
                                    let delta = json!({
                                        "jsonrpc": "2.0",
                                        "method": "item/agentMessage/delta",
                                        "params": {
                                            "threadId": "r1",
                                            "turnId": turn_id,
                                            "delta": "turn completed successfully"
                                        }
                                    });
                                    let _ = ws.send(Message::text(delta.to_string())).await;
                                    let terminal = json!({
                                        "jsonrpc": "2.0",
                                        "method": "turn/completed",
                                        "params": {
                                            "threadId": "r1",
                                            "turn": {
                                                "id": turn_id,
                                                "status": "completed"
                                            }
                                        }
                                    });
                                    let _ = ws.send(Message::text(terminal.to_string())).await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    // File-backed real SQLite database
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("u16_binding.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let db1 = Database::connect(&db_url).await.unwrap();

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u16"),
        "channel-u16",
        None::<String>,
        "user-u16",
    );

    // 1. Initial turn on Multiplexer 1: starts thread r1, turn fails
    let (action1_tx, mut action1_rx) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher1 = Arc::new(BindingTestDispatcher::new(action1_tx));
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let backend1 = Arc::new(
        OmoBackend::new(config.clone(), dispatcher1.clone()).with_pool(db1.pool().clone()),
    );
    let mux1 = SessionMultiplexer::with_dispatcher(
        db1.pool().clone(),
        backend1,
        Some(dispatcher1.clone()),
        MultiplexerConfig::default(),
    );

    let event1 = InboundEvent::message(session_key.clone(), "msg-u16-1", "first turn that fails");
    mux1.route(event1).await.unwrap();

    // Await turn 1 completion (failure message delivered via dispatcher, ignoring typing indicator)
    let action = loop {
        let act = tokio::time::timeout(std::time::Duration::from_secs(5), action1_rx.recv())
            .await
            .expect("bounded wait for turn 1 action")
            .expect("turn 1 action received");
        if matches!(act, OutboundAction::SendMessage { .. }) {
            break act;
        }
    };
    assert!(
        matches!(action, OutboundAction::SendMessage { .. }),
        "turn 1 failure must dispatch error SendMessage"
    );

    // Shut down mux 1 and verify initial counts
    drop(mux1);
    db1.close().await;

    assert_eq!(
        thread_start_count.load(Ordering::SeqCst),
        1,
        "turn 1 must start thread exactly once"
    );

    // 2. Reconstruct completely new backend & mux from file DB
    let db2 = Database::connect(&db_url).await.unwrap();
    let (action2_tx, mut action2_rx) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher2 = Arc::new(BindingTestDispatcher::new(action2_tx));
    let backend2 = Arc::new(
        OmoBackend::new(config.clone(), dispatcher2.clone()).with_pool(db2.pool().clone()),
    );
    let mux2 = SessionMultiplexer::with_dispatcher(
        db2.pool().clone(),
        backend2,
        Some(dispatcher2.clone()),
        MultiplexerConfig::default(),
    );

    let event2 = InboundEvent::message(
        session_key.clone(),
        "msg-u16-2",
        "second turn should resume",
    );
    mux2.route(event2).await.unwrap();

    // Await turn 2 completion (streaming response, ignoring typing indicator)
    let action2 = loop {
        let act = tokio::time::timeout(std::time::Duration::from_secs(5), action2_rx.recv())
            .await
            .expect("bounded wait for turn 2 action")
            .expect("turn 2 action received");
        if matches!(act, OutboundAction::Stream { .. }) {
            break act;
        }
    };
    assert!(
        matches!(action2, OutboundAction::Stream { .. }),
        "turn 2 must stream content"
    );

    // Verification of durable binding scenario:
    // RED on unpatched code: thread_start_count is 2 (second thread/start sent)
    // GREEN on patched code: thread_start_count is 1, thread_resume_count is 1 (resumed r1)
    assert_eq!(
        thread_start_count.load(Ordering::SeqCst),
        1,
        "durable binding must not issue a second thread/start after first turn failure"
    );
    assert_eq!(
        thread_resume_count.load(Ordering::SeqCst),
        1,
        "reconstructed backend/mux must resume r1"
    );

    // 3. Missing-rollout subcase:
    // When thread/resume fails with 'no rollout found', must return continuity_error,
    // not silently start a fresh replacement thread.
    let missing_key = SessionKey::new(
        "discord",
        Some("guild-u16"),
        "channel-u16",
        None::<String>,
        "user-missing",
    );
    let mut missing_session = SessionContext::new(missing_key.clone());
    missing_session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r-missing"));

    let (action3_tx, _action3_rx) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher3 = Arc::new(BindingTestDispatcher::new(action3_tx));
    let backend3 = OmoBackend::new(config.clone(), dispatcher3).with_pool(db2.pool().clone());
    let missing_event = InboundEvent::message(missing_key, "msg-missing", "probe missing rollout");
    let missing_result = backend3.run(&mut missing_session, missing_event).await;

    drop(mux2);
    db2.close().await;
    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert!(
        missing_result.is_err(),
        "missing rollout must return an error, not silently succeed with fresh history"
    );
    let err_msg = missing_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("continuity_error"),
        "missing rollout must explicitly report continuity_error, got: {err_msg}"
    );
}

#[tokio::test]
async fn missing_rollout_fails_with_continuity_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let thread_start_count = Arc::new(AtomicUsize::new(0));
    let tsc = thread_start_count.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<Value>(&text) {
                        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                        match method {
                            "initialize" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "capabilities": {} }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/resume" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": {
                                        "code": -32603,
                                        "message": "no rollout found for thread id r-missing"
                                    }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/start" => {
                                tsc.fetch_add(1, Ordering::SeqCst);
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "r-replacement" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "turn/start" => {
                                let ack = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "turn": {
                                            "id": "t1",
                                            "status": "inProgress"
                                        }
                                    }
                                });
                                let _ = ws.send(Message::text(ack.to_string())).await;
                                let terminal = json!({
                                    "jsonrpc": "2.0",
                                    "method": "turn/completed",
                                    "params": {
                                        "threadId": "r-replacement",
                                        "turn": {
                                            "id": "t1",
                                            "status": "completed"
                                        }
                                    }
                                });
                                let _ = ws.send(Message::text(terminal.to_string())).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let (action_tx, _action_rx) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher = Arc::new(BindingTestDispatcher::new(action_tx));
    let backend = OmoBackend::new(config, dispatcher);

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u16-missing"),
        "channel-u16-missing",
        None::<String>,
        "user-u16-missing",
    );
    let mut session = SessionContext::new(session_key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r-missing"));

    let event = InboundEvent::message(session_key, "msg-missing-1", "probe missing rollout");
    let result = backend.run(&mut session, event).await;

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert_eq!(
        thread_start_count.load(Ordering::SeqCst),
        0,
        "missing rollout must not silently issue thread/start replacement"
    );
    assert!(
        result.is_err(),
        "missing rollout must return explicit error instead of silent success with empty history"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("continuity_error"),
        "missing rollout error must contain 'continuity_error', got: {err}"
    );
}

#[tokio::test]
async fn persistence_failure_prevents_turn_submission() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let tsc = turn_start_count.clone();

    let peer_handle = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(mut ws) = tokio_tungstenite::accept_async(socket).await else {
                continue;
            };

            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(req) = serde_json::from_str::<Value>(&text) {
                        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
                        match method {
                            "initialize" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "capabilities": {} }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/start" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "r-persist-fail" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "thread/resume" => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "thread": { "id": "r-reconnect-18" } }
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                            "turn/start" => {
                                tsc.fetch_add(1, Ordering::SeqCst);
                                let ack = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": {
                                        "code": -32000,
                                        "message": "undurable side effect: turn/start should not have been submitted"
                                    }
                                });
                                let _ = ws.send(Message::text(ack.to_string())).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();

    // Trigger that fails any session update/insert for persisting thread binding
    sqlx::query(
        "CREATE TRIGGER block_binding_persistence BEFORE UPDATE ON sessions
         WHEN NEW.state_json LIKE '%omo_thread_id%'
         BEGIN
             SELECT RAISE(ABORT, 'binding write blocked');
         END;",
    )
    .execute(db.pool())
    .await
    .unwrap();

    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_request_timeout(std::time::Duration::from_secs(5))
        .with_total_timeout(std::time::Duration::from_secs(10));
    let (action_tx, _action_rx) = tokio::sync::mpsc::unbounded_channel();
    let dispatcher = Arc::new(BindingTestDispatcher::new(action_tx));
    let backend = OmoBackend::new(config, dispatcher).with_pool(db.pool().clone());

    let session_key = SessionKey::new(
        "discord",
        Some("guild-u16-block"),
        "channel-u16-block",
        None::<String>,
        "user-u16-block",
    );
    let mut session = SessionContext::new(session_key.clone());
    // Pre-insert session row so the UPDATE trigger triggers when persisting the binding
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json)
         VALUES (?, 'discord', 'guild-u16-block', 'channel-u16-block', 'user-u16-block', '{}')",
    )
    .bind(session_key.storage_key())
    .execute(db.pool())
    .await
    .unwrap();

    let event = InboundEvent::message(session_key, "msg-block-1", "turn that must not submit");
    // 1. Initial turn path: failure to persist thread binding on thread/start prevents turn/start submission
    let result = backend.run(&mut session, event).await;
    assert!(
        result.is_err(),
        "backend.run must fail if thread binding persistence fails on initial turn"
    );
    assert_eq!(
        turn_start_count.load(Ordering::SeqCst),
        0,
        "failure to persist thread binding must prevent initial turn submission (turn/start must not be sent)"
    );

    // 2. Reconnect path: session already has an existing thread binding.
    // When executing a subsequent turn, thread/resume succeeds remotely.
    // If updating/persisting the binding in SQLite fails, backend must fail BEFORE turn/start.
    let reconnect_key = SessionKey::new(
        "discord",
        Some("guild-u18-reconnect"),
        "channel-u18-reconnect",
        None::<String>,
        "user-u18-reconnect",
    );
    let mut reconnect_session = SessionContext::new(reconnect_key.clone());
    reconnect_session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r-reconnect-18"));

    // Pre-insert session row with omo_thread_id
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json)
         VALUES (?, 'discord', 'guild-u18-reconnect', 'channel-u18-reconnect', 'user-u18-reconnect', '{\"metadata\":{\"omo_thread_id\":\"r-reconnect-18\"}}')",
    )
    .bind(reconnect_key.storage_key())
    .execute(db.pool())
    .await
    .unwrap();

    let reconnect_event = InboundEvent::message(
        reconnect_key,
        "msg-block-reconnect",
        "reconnect turn that must not submit if persistence fails",
    );
    let reconnect_result = backend.run(&mut reconnect_session, reconnect_event).await;

    peer_handle.abort();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), peer_handle).await;

    assert!(
        reconnect_result.is_err(),
        "backend.run must fail on reconnect if thread binding persistence fails"
    );
    assert_eq!(
        turn_start_count.load(Ordering::SeqCst),
        0,
        "failure to persist thread binding on reconnect must prevent turn submission (turn/start must not be sent)"
    );
}

#[tokio::test]
async fn omo_approval_roundtrip_uses_session_policy() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (decision_tx, mut decision_rx) = tokio::sync::mpsc::unbounded_channel();

    let server = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(msg)) = ws.next().await {
                if let Ok(text) = msg.to_text() {
                    let v: serde_json::Value = serde_json::from_str(text).unwrap();
                    let id = v.get("id");
                    match v.get("method").and_then(Value::as_str) {
                        Some("initialize") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                                ))
                                .await;
                        }
                        Some("thread/start") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({
                                        "jsonrpc":"2.0","id":id,
                                        "result":{"thread":{"id":"t1"}}
                                    })
                                    .to_string(),
                                ))
                                .await;
                        }
                        Some("turn/start") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({
                                        "jsonrpc":"2.0","id":id,
                                        "result":{"turn":{"id":"turn1","status":"inProgress"}}
                                    })
                                    .to_string(),
                                ))
                                .await;
                            // Server sends approval request to gateway
                            let req = json!({
                                "jsonrpc": "2.0",
                                "id": 999,
                                "method": "item/commandExecution/request",
                                "params": {"command": "cargo test"}
                            });
                            let _ = ws.send(Message::text(req.to_string())).await;
                        }
                        _ => {
                            // Check if this is the response to our approval request id: 999
                            if v.get("id").and_then(Value::as_i64) == Some(999) {
                                let decision =
                                    v["result"]["decision"].as_str().unwrap_or("").to_string();
                                let _ = decision_tx.send(decision);
                                let idle = json!({
                                    "jsonrpc":"2.0","method":"turn/completed",
                                    "params":{"threadId":"t1","turn":{"id":"turn1","status":"completed"}}
                                });
                                let _ = ws.send(Message::text(idle.to_string())).await;
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_total_timeout(std::time::Duration::from_secs(5));
    let backend = OmoBackend::new(config, Arc::new(CapturingDispatcher::new()));

    let session_key = SessionKey::new("discord", Some("bot1"), "c1", None::<String>, "u1");
    let mut session = SessionContext::new(session_key.clone());
    session.state.yolo = true;

    let event = InboundEvent::message(session_key, "m1", "run with yolo");
    let _ = backend.run(&mut session, event).await;

    let decision = decision_rx.recv().await.expect("must receive decision");
    assert_eq!(
        decision, "accept",
        "YOLO session must accept approval request"
    );

    let _ = server.await;
}

#[tokio::test]
async fn profile_tool_restrictions_are_enforced() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (decision_tx, mut decision_rx) = tokio::sync::mpsc::unbounded_channel();

    let server = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            while let Some(Ok(msg)) = ws.next().await {
                if let Ok(text) = msg.to_text() {
                    let v: serde_json::Value = serde_json::from_str(text).unwrap();
                    let id = v.get("id");
                    match v.get("method").and_then(Value::as_str) {
                        Some("initialize") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({"jsonrpc":"2.0","id":id,"result":{}}).to_string(),
                                ))
                                .await;
                        }
                        Some("thread/start") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({
                                        "jsonrpc":"2.0","id":id,
                                        "result":{"thread":{"id":"t1"}}
                                    })
                                    .to_string(),
                                ))
                                .await;
                        }
                        Some("turn/start") => {
                            let _ = ws
                                .send(Message::text(
                                    json!({
                                        "jsonrpc":"2.0","id":id,
                                        "result":{"turn":{"id":"turn1","status":"inProgress"}}
                                    })
                                    .to_string(),
                                ))
                                .await;
                            // Server requests commandExecution (terminal), which is not in enabled_toolsets ["web"]
                            let req = json!({
                                "jsonrpc": "2.0",
                                "id": 888,
                                "method": "item/commandExecution/request",
                                "params": {"command": "cargo test"}
                            });
                            let _ = ws.send(Message::text(req.to_string())).await;
                        }
                        _ => {
                            if v.get("id").and_then(Value::as_i64) == Some(888) {
                                let decision =
                                    v["result"]["decision"].as_str().unwrap_or("").to_string();
                                let _ = decision_tx.send(decision);
                                let idle = json!({
                                    "jsonrpc":"2.0","method":"turn/completed",
                                    "params":{"threadId":"t1","turn":{"id":"turn1","status":"completed"}}
                                });
                                let _ = ws.send(Message::text(idle.to_string())).await;
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
        .with_total_timeout(std::time::Duration::from_secs(5));
    let backend = OmoBackend::new(config, Arc::new(CapturingDispatcher::new()));

    let session_key = SessionKey::new("discord", Some("bot1"), "c1", None::<String>, "u1");
    let mut session = SessionContext::new(session_key.clone());
    session.state.yolo = true; // Even in YOLO mode!
    session.state.enabled_toolsets = Some(vec!["web".into()]); // Only web allowed!

    let event = InboundEvent::message(session_key, "m1", "try executing command");
    let _ = backend.run(&mut session, event).await;

    let decision = decision_rx.recv().await.expect("must receive decision");
    assert_eq!(
        decision, "decline",
        "commandExecution must be declined when restricted to web toolset"
    );

    let _ = server.await;
}

#[tokio::test]
async fn failed_final_delivery_is_not_acknowledged() {
    let pool = omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let server = FakeAppServer::spawn().await;
    let config = OmoBackendConfig::new(format!("ws://127.0.0.1:{}", server.port))
        .with_request_timeout(std::time::Duration::from_secs(3));
    let dispatcher = Arc::new(FailingStreamDispatcher);
    let backend = OmoBackend::new(config, dispatcher).with_pool(pool.clone());

    let marker = tempfile::tempdir().expect("tempdir");
    let marker_path = marker.path().join("ack-ran");

    let session_key = SessionKey::new("discord", None::<String>, "chan", None::<String>, "cron");
    let mut session = SessionContext::new(session_key.clone());
    session.state.metadata.insert(
        "cron_ack_command".into(),
        json!(format!(
            "touch '{}'",
            marker_path.to_string_lossy().replace('\\', "/")
        )),
    );

    let result = backend
        .run(
            &mut session,
            InboundEvent::message(session_key, "msg", "digest"),
        )
        .await;

    assert!(
        result.is_err(),
        "delivery failure on final turn must return Err, got: {result:?}"
    );
    assert!(
        !marker_path.exists(),
        "ack command must NOT run when delivery failed"
    );

    let obl_rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id, state FROM delivery_obligations")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        !obl_rows.is_empty(),
        "delivery obligation must be recorded in DB"
    );
    assert_eq!(obl_rows[0].1, "failed", "obligation state must be failed");
}

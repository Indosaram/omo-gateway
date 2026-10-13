use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use omon_gateway::{
    AgentBackend, InboundEvent, OmoBackend, OmoBackendConfig, OutboundAction, OutboundDispatcher,
    SessionContext, SessionKey, StreamChunk,
};
use parking_lot::Mutex as ParkingMutex;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Default)]
struct Capture(ParkingMutex<Vec<StreamChunk>>);

impl Capture {
    fn finals(&self) -> Vec<String> {
        self.0
            .lock()
            .iter()
            .filter(|c| c.is_final)
            .map(|c| c.content.clone())
            .collect()
    }
}

#[async_trait]
impl OutboundDispatcher for Capture {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        if let OutboundAction::Stream { chunk, .. } = action {
            self.0.lock().push(chunk);
        }
        Ok(())
    }
}

async fn spawn_peer(
    conns: usize,
    after_ack: Vec<String>,
    after_interrupt: Vec<String>,
) -> (
    u16,
    Arc<ParkingMutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let methods = Arc::new(ParkingMutex::new(Vec::new()));
    let log = methods.clone();
    let peer = tokio::spawn(async move {
        for _ in 0..conns {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            while let Some(Ok(Message::Text(text))) = ws.next().await {
                let request: Value = serde_json::from_str(text.as_str()).unwrap();
                let method = request["method"].as_str().unwrap_or("").to_string();
                log.lock().push(method.clone());
                let id = request["id"].clone();
                let result = match method.as_str() {
                    "thread/start" | "thread/resume" => json!({"thread":{"id":"r1"}}),
                    "turn/start" => json!({"turn":{"id":"t2","status":"inProgress"}}),
                    _ => json!({}),
                };
                let reply = json!({"jsonrpc":"2.0","id":id,"result":result});
                // Terminal frames precede the interrupt ack so the deadline drain
                // observes turn/completed before the 9_001 response.
                if method == "turn/interrupt" {
                    for frame in &after_interrupt {
                        ws.send(Message::text(frame.clone())).await.unwrap();
                    }
                }
                ws.send(Message::text(reply.to_string())).await.unwrap();
                if method == "turn/start" {
                    for frame in &after_ack {
                        ws.send(Message::text(frame.clone())).await.unwrap();
                    }
                }
            }
        }
    });
    (port, methods, peer)
}

fn frame(value: Value) -> String {
    value.to_string()
}

fn completed(status: &str) -> String {
    frame(json!({"method":"turn/completed",
        "params":{"threadId":"r1","turn":{"id":"t2","status":status}}}))
}

fn interactive_session(user: &str) -> (SessionKey, SessionContext) {
    let key = SessionKey::new("local", None::<String>, user, None::<String>, "user");
    let mut session = SessionContext::new(key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));
    (key, session)
}

#[tokio::test]
async fn turn_error_and_invalid_frame_release_active_turn() {
    let turn_error = frame(json!({"method":"turn/error",
        "params":{"threadId":"r1","turnId":"t2","message":"boom"}}));
    for bad in [turn_error, "not-json".to_string()] {
        let (port, _, peer) = spawn_peer(1, vec![bad], vec![]).await;
        let backend = OmoBackend::new(
            OmoBackendConfig::new(format!("ws://127.0.0.1:{port}")),
            Arc::new(Capture::default()),
        );
        let (key, mut session) = interactive_session("release");
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            backend.run(&mut session, InboundEvent::message(key, "m", "go")),
        )
        .await
        .unwrap();
        assert!(result.is_err(), "{result:?}");
        assert!(
            backend.active_turns.lock().is_empty(),
            "errored turn left active_turns entry: {result:?}"
        );
        drop(backend);
        tokio::time::timeout(Duration::from_secs(5), peer)
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn deadline_drain_strips_intent_gate_for_cron() {
    let message = frame(json!({"method":"item/completed","params":{"threadId":"r1",
        "turnId":"t2","item":{"id":"m1","type":"agentMessage",
        "text":"> I read this as a digest. I'll stop when done.\n\nDIGEST_BODY"}}}));
    let (port, methods, peer) = spawn_peer(1, vec![], vec![message, completed("completed")]).await;
    let capture = Arc::new(Capture::default());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://127.0.0.1:{port}"))
            .with_total_timeout(Duration::from_secs(1)),
        capture.clone(),
    );
    let key = SessionKey::new(
        "discord",
        None::<String>,
        "chan",
        None::<String>,
        "cron:job",
    );
    let mut session = SessionContext::new(key.clone());
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        backend.run(&mut session, InboundEvent::message(key, "m", "run")),
    )
    .await
    .unwrap();
    assert!(result.is_ok(), "{result:?}");
    assert!(methods.lock().iter().any(|m| m == "turn/interrupt"));
    assert_eq!(capture.finals(), vec!["DIGEST_BODY".to_string()]);
    drop(backend);
    tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn reset_thread_binding_starts_fresh_thread_instead_of_cached_one() {
    let reply = frame(json!({"method":"item/agentMessage/delta",
        "params":{"threadId":"r1","turnId":"t2","itemId":"m1","delta":"OK"}}));
    let (port, methods, peer) = spawn_peer(2, vec![reply, completed("completed")], vec![]).await;
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://127.0.0.1:{port}")),
        Arc::new(Capture::default()),
    );
    let key = SessionKey::new("local", None::<String>, "reset", None::<String>, "user");
    let mut session = SessionContext::new(key.clone());
    for turn in 0..2 {
        if turn == 1 {
            session.state.metadata.remove("omo_thread_id");
        }
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            backend.run(
                &mut session,
                InboundEvent::message(key.clone(), "m", "hello"),
            ),
        )
        .await
        .unwrap();
        assert!(result.is_ok(), "{result:?}");
    }
    let thread_methods: Vec<String> = methods
        .lock()
        .iter()
        .filter(|m| m.starts_with("thread/"))
        .cloned()
        .collect();
    assert_eq!(thread_methods, vec!["thread/start", "thread/start"]);
    drop(backend);
    tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}

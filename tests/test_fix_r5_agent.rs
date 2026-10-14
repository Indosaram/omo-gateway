use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use omon_gateway::{
    mirror_to_session, peek_pending_cron_briefings, AgentBackend, InboundEvent, OmoBackend,
    OmoBackendConfig, OutboundAction, OutboundDispatcher, SessionContext, SessionKey,
};
use parking_lot::Mutex as ParkingMutex;
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

#[derive(Default)]
struct Recorder(ParkingMutex<Vec<OutboundAction>>);

#[async_trait]
impl OutboundDispatcher for Recorder {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        self.0.lock().push(action);
        Ok(())
    }
}

type Peer = WebSocketStream<TcpStream>;

async fn send(ws: &mut Peer, value: Value) {
    ws.send(Message::text(value.to_string())).await.unwrap();
}

/// Answers the handshake requests and returns the `turn/start` request.
async fn serve_until_turn_start(ws: &mut Peer) -> Value {
    loop {
        let message = ws.next().await.unwrap().unwrap();
        let request: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        let id = request["id"].clone();
        match request["method"].as_str().unwrap() {
            "initialize" | "thread/settings/update" => {
                send(ws, json!({"jsonrpc":"2.0","id":id,"result":{}})).await
            }
            "thread/resume" => {
                send(
                    ws,
                    json!({"jsonrpc":"2.0","id":id,"result":{"thread":{"id":"r1"}}}),
                )
                .await
            }
            "turn/start" => return request,
            other => panic!("unexpected request {other}"),
        }
    }
}

fn ack() -> Value {
    json!({"jsonrpc":"2.0","id":3,"result":{"turn":{"id":"t2","status":"inProgress"}}})
}

fn terminal() -> Value {
    json!({"method":"turn/completed",
        "params":{"threadId":"r1","turn":{"id":"t2","status":"completed"}}})
}

fn agent_message(id: &str, text: &str) -> Value {
    json!({"method":"item/completed","params":{"threadId":"r1","turnId":"t2",
        "item":{"id":id,"type":"agentMessage","text":text}}})
}

fn resumed_session(user: &str) -> (SessionKey, SessionContext) {
    let key = SessionKey::new("local", None::<String>, user, None::<String>, "user");
    let mut session = SessionContext::new(key.clone());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("r1"));
    (key, session)
}

#[tokio::test]
async fn mid_turn_status_is_a_visible_message_not_a_stream_chunk_or_the_reply() {
    // Given: a turn whose agent reports status, then runs a tool, then answers.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        serve_until_turn_start(&mut ws).await;
        send(&mut ws, ack()).await;
        send(&mut ws, agent_message("m-status", "Checking now.")).await;
        send(
            &mut ws,
            json!({"method":"item/started","params":{"threadId":"r1","turnId":"t2",
                "item":{"id":"tool-1","type":"commandExecution"}}}),
        )
        .await;
        send(&mut ws, agent_message("m-final", "All done")).await;
        send(&mut ws, terminal()).await;
        let _ = stop_rx.await;
    });
    let dispatcher = Arc::new(Recorder::default());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        dispatcher.clone(),
    );
    let (key, mut session) = resumed_session("status");

    // When: the turn runs to completion.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        backend.run(&mut session, InboundEvent::message(key, "m", "go")),
    )
    .await
    .expect("bounded run");
    let _ = stop_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .expect("bounded peer")
        .unwrap();

    // Then: the status reached the user as one ordinary message, never as a
    // stream chunk (the adapter drops non-final chunks), and the reply is the
    // final answer only.
    result.expect("turn succeeds");
    let actions = dispatcher.0.lock().clone();
    let sent: Vec<&str> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::SendMessage { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sent, vec!["Checking now."]);
    let chunks: Vec<_> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::Stream { chunk, .. } => Some(chunk),
            _ => None,
        })
        .collect();
    assert!(
        chunks
            .iter()
            .all(|chunk| !chunk.content.contains("Checking now.")),
        "status must not travel as a stream chunk: {chunks:?}"
    );
    let finals: Vec<&str> = chunks
        .iter()
        .filter(|chunk| chunk.is_final)
        .map(|chunk| chunk.content.as_str())
        .collect();
    assert_eq!(finals, vec!["All done"]);
    assert_eq!(session.state.metadata["cron_agent_output"], "All done");
}

#[tokio::test]
async fn only_the_first_status_is_relayed_then_the_final_answer() {
    // Given: a turn that narrates before every tool call, then answers.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        serve_until_turn_start(&mut ws).await;
        send(&mut ws, ack()).await;
        for (index, text) in ["On it.", "Reading the log.", "Found 3 failures."]
            .into_iter()
            .enumerate()
        {
            send(&mut ws, agent_message(&format!("m-{index}"), text)).await;
            send(
                &mut ws,
                json!({"method":"item/started","params":{"threadId":"r1","turnId":"t2",
                    "item":{"id":format!("tool-{index}"),"type":"commandExecution"}}}),
            )
            .await;
        }
        send(&mut ws, agent_message("m-final", "Report")).await;
        send(&mut ws, terminal()).await;
        let _ = stop_rx.await;
    });
    let dispatcher = Arc::new(Recorder::default());
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        dispatcher.clone(),
    );
    let (key, mut session) = resumed_session("ack-only");

    // When: the turn runs to completion.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        backend.run(&mut session, InboundEvent::message(key, "m", "go")),
    )
    .await
    .expect("bounded run");
    let _ = stop_tx.send(());
    tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .expect("bounded peer")
        .unwrap();

    // Then: the user sees one acknowledgement and the final answer, nothing between.
    result.expect("turn succeeds");
    let actions = dispatcher.0.lock().clone();
    let sent: Vec<&str> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::SendMessage { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sent, vec!["On it."]);
    let finals: Vec<&str> = actions
        .iter()
        .filter_map(|action| match action {
            OutboundAction::Stream { chunk, .. } if chunk.is_final => Some(chunk.content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(finals, vec!["Report"]);
}

#[tokio::test]
async fn cron_briefings_survive_a_model_fallback_and_are_consumed_after_the_ack() {
    // Given: a pending cron briefing and a primary model the daemon rejects.
    let pool = omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .unwrap();
    let (key, mut session) = resumed_session("briefing");
    session.state.active_model = Some("custom/dead-model".into());
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
         VALUES (?, 'local', 'briefing', 'user', '{}')",
    )
    .bind(key.storage_key())
    .execute(&pool)
    .await
    .unwrap();
    assert!(mirror_to_session(
        &pool,
        &key.storage_key(),
        "assistant",
        "BRIEFING_PAYLOAD_R5",
        Some("cron:job-a"),
    )
    .await
    .unwrap());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let prompt = |request: &Value| {
            request
                .pointer("/params/input/0/text")
                .and_then(Value::as_str)
                .unwrap()
                .to_owned()
        };
        // Attempt 0: turn/start is rejected because of the model.
        let (socket, _) = listener.accept().await.unwrap();
        let mut first = tokio_tungstenite::accept_async(socket).await.unwrap();
        let request = serve_until_turn_start(&mut first).await;
        let first_prompt = prompt(&request);
        send(
            &mut first,
            json!({"jsonrpc":"2.0","id":request["id"],
                "error":{"code":-32603,"message":"model not found: custom/dead-model"}}),
        )
        .await;
        // Attempt 1: the fallback re-run must still carry the briefing.
        let (socket, _) = listener.accept().await.unwrap();
        let mut second = tokio_tungstenite::accept_async(socket).await.unwrap();
        let request = serve_until_turn_start(&mut second).await;
        let second_prompt = prompt(&request);
        send(&mut second, ack()).await;
        send(&mut second, agent_message("m1", "Fallback OK")).await;
        send(&mut second, terminal()).await;
        let _ = stop_rx.await;
        (first_prompt, second_prompt)
    });
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        Arc::new(Recorder::default()),
    )
    .with_pool(pool.clone());

    // When: the run fails over to the fallback model.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        backend.run(&mut session, InboundEvent::message(key.clone(), "m", "go")),
    )
    .await
    .expect("bounded run");
    let _ = stop_tx.send(());
    let (first_prompt, second_prompt) = tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .expect("bounded peer")
        .unwrap();

    // Then: both attempts carried the briefing and it is consumed only now.
    result.expect("fallback turn succeeds");
    assert!(
        first_prompt.contains("BRIEFING_PAYLOAD_R5"),
        "{first_prompt}"
    );
    assert!(
        second_prompt.contains("BRIEFING_PAYLOAD_R5"),
        "the fallback attempt lost the briefing: {second_prompt}"
    );
    assert!(peek_pending_cron_briefings(&pool, &key.storage_key())
        .await
        .unwrap()
        .is_none());
}

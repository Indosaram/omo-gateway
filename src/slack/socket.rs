//! Slack Socket Mode client: acks every envelope, forwards typed events, reconnects with backoff.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::slack::SlackApi;
use crate::Result;

const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);
const DEFAULT_RECEIVE_WATCHDOG: Duration = Duration::from_secs(1200);

/// Longest silence tolerated on an open socket before reconnecting
/// (`OMON_SLACK_RECEIVE_WATCHDOG_SECS`, default 1200). Slack pings every few seconds, so a
/// half-open connection that stops delivering frames is replaced instead of hanging forever.
pub fn receive_watchdog() -> Duration {
    parse_watchdog(
        std::env::var("OMON_SLACK_RECEIVE_WATCHDOG_SECS")
            .ok()
            .as_deref(),
    )
}

fn parse_watchdog(raw: Option<&str>) -> Duration {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map_or(DEFAULT_RECEIVE_WATCHDOG, Duration::from_secs)
}

#[derive(Clone, Debug, PartialEq)]
pub enum SocketEvent {
    /// `payload.event` with `team` filled from `payload.team_id` when missing, and the event id.
    EventsApi(Value, String),
    SlashCommand(Value),
    Interactive(Value),
}

pub struct SocketClient {
    api: SlackApi,
    app_token: String,
}

enum SessionEnd {
    Shutdown,
    Disconnect,
    Dropped(String),
    ReceiverClosed,
}

impl SocketClient {
    pub fn new(api: SlackApi, app_token: String) -> Self {
        Self { api, app_token }
    }

    pub async fn run(
        self,
        tx: mpsc::Sender<SocketEvent>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let mut backoff = BACKOFF_MIN;
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
            let url = tokio::select! {
                () = shutdown.cancelled() => return Ok(()),
                url = self.api.connections_open(&self.app_token) => url,
            };
            let end = match url {
                Ok(url) => session(&url, &tx, &shutdown, &mut backoff, receive_watchdog()).await,
                Err(e) => SessionEnd::Dropped(format!("apps.connections.open failed: {e}")),
            };
            if !should_reconnect(end, &mut backoff, &shutdown).await {
                return Ok(());
            }
        }
    }
}

/// Socket Mode loop against a fixed WebSocket URL, skipping `apps.connections.open`.
#[cfg(test)]
pub(crate) async fn run_on_url(
    url: &str,
    tx: mpsc::Sender<SocketEvent>,
    shutdown: CancellationToken,
) -> Result<()> {
    run_on_url_with_watchdog(url, tx, shutdown, DEFAULT_RECEIVE_WATCHDOG).await
}

#[cfg(test)]
pub(crate) async fn run_on_url_with_watchdog(
    url: &str,
    tx: mpsc::Sender<SocketEvent>,
    shutdown: CancellationToken,
    watchdog: Duration,
) -> Result<()> {
    let mut backoff = BACKOFF_MIN;
    loop {
        if shutdown.is_cancelled() {
            return Ok(());
        }
        let end = session(url, &tx, &shutdown, &mut backoff, watchdog).await;
        if !should_reconnect(end, &mut backoff, &shutdown).await {
            return Ok(());
        }
    }
}

async fn should_reconnect(
    end: SessionEnd,
    backoff: &mut Duration,
    shutdown: &CancellationToken,
) -> bool {
    match end {
        SessionEnd::Shutdown => false,
        SessionEnd::ReceiverClosed => {
            tracing::info!("slack socket: event receiver closed, stopping");
            false
        }
        SessionEnd::Disconnect => {
            tracing::info!("slack socket: server requested disconnect, reconnecting");
            true
        }
        SessionEnd::Dropped(reason) => {
            tracing::warn!(
                "slack socket: connection lost ({reason}), reconnecting in {}s",
                backoff.as_secs()
            );
            let delay = *backoff;
            *backoff = (*backoff * 2).min(BACKOFF_MAX);
            tokio::select! {
                () = shutdown.cancelled() => false,
                () = tokio::time::sleep(delay) => true,
            }
        }
    }
}

async fn session(
    url: &str,
    tx: &mpsc::Sender<SocketEvent>,
    shutdown: &CancellationToken,
    backoff: &mut Duration,
    watchdog: Duration,
) -> SessionEnd {
    let ws = tokio::select! {
        () = shutdown.cancelled() => return SessionEnd::Shutdown,
        ws = tokio_tungstenite::connect_async(url) => ws,
    };
    let mut ws = match ws {
        Ok((ws, _)) => ws,
        Err(e) => return SessionEnd::Dropped(format!("connect failed: {e}")),
    };
    *backoff = BACKOFF_MIN;
    tracing::info!("slack socket: connected");

    loop {
        let frame = tokio::select! {
            () = shutdown.cancelled() => {
                let _ = ws.close(None).await;
                return SessionEnd::Shutdown;
            }
            frame = tokio::time::timeout(watchdog, ws.next()) => match frame {
                Ok(frame) => frame,
                Err(_) => {
                    let _ = ws.close(None).await;
                    return SessionEnd::Dropped(format!(
                        "receive watchdog: no frame for {}s",
                        watchdog.as_secs()
                    ));
                }
            },
        };
        let text = match frame {
            None => return SessionEnd::Dropped("stream ended".into()),
            Some(Err(e)) => return SessionEnd::Dropped(format!("read failed: {e}")),
            Some(Ok(Message::Text(text))) => text,
            Some(Ok(Message::Ping(data))) => {
                if let Err(e) = ws.send(Message::Pong(data)).await {
                    return SessionEnd::Dropped(format!("pong failed: {e}"));
                }
                continue;
            }
            Some(Ok(Message::Close(frame))) => {
                return SessionEnd::Dropped(format!("closed by server: {frame:?}"));
            }
            Some(Ok(_)) => continue,
        };
        let frame: Value = match serde_json::from_str(text.as_str()) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::warn!("slack socket: ignoring non-JSON frame: {e}");
                continue;
            }
        };
        let Some((ack, event, disconnect)) = classify(&frame) else {
            continue;
        };
        if let Some(envelope_id) = ack {
            let ack = json!({ "envelope_id": envelope_id }).to_string();
            if let Err(e) = ws.send(Message::text(ack)).await {
                return SessionEnd::Dropped(format!("ack failed: {e}"));
            }
        }
        if disconnect {
            let _ = ws.close(None).await;
            return SessionEnd::Disconnect;
        }
        if let Some(event) = event {
            if tx.send(event).await.is_err() {
                let _ = ws.close(None).await;
                return SessionEnd::ReceiverClosed;
            }
        }
    }
}

/// Splits a Socket Mode frame into (envelope id to ack, event to forward, disconnect requested).
/// `None` for frames that need no action at all.
pub fn classify(frame: &Value) -> Option<(Option<String>, Option<SocketEvent>, bool)> {
    let ack = frame
        .get("envelope_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let kind = frame
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = frame.get("payload");
    let (event, disconnect) = match kind {
        "hello" => {
            let host = frame
                .pointer("/debug_info/host")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            tracing::info!("slack socket: hello from {host}");
            (None, false)
        }
        "disconnect" => {
            let reason = frame
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            tracing::info!("slack socket: disconnect requested ({reason})");
            (None, true)
        }
        "events_api" => (payload.and_then(events_api), false),
        "slash_commands" => (payload.cloned().map(SocketEvent::SlashCommand), false),
        "interactive" => (payload.cloned().map(SocketEvent::Interactive), false),
        other => {
            tracing::debug!("slack socket: ignoring frame type {other:?}");
            (None, false)
        }
    };
    (ack.is_some() || event.is_some() || disconnect).then_some((ack, event, disconnect))
}

fn events_api(payload: &Value) -> Option<SocketEvent> {
    let mut event = payload.get("event")?.clone();
    if let (Some(map), Some(team)) = (event.as_object_mut(), payload.get("team_id")) {
        if !map.get("team").is_some_and(Value::is_string) {
            map.insert("team".into(), team.clone());
        }
    }
    let event_id = payload
        .get("event_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some(SocketEvent::EventsApi(event, event_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    fn events_frame(envelope_id: &str) -> Value {
        json!({
            "envelope_id": envelope_id,
            "type": "events_api",
            "accepts_response_payload": false,
            "payload": {
                "team_id": "T1",
                "event_id": "Ev1",
                "event": { "type": "message", "channel": "C1", "user": "U1", "text": "hi", "ts": "1.2" }
            }
        })
    }

    #[test]
    fn classify_events_api_merges_team_and_acks() {
        let (ack, event, disconnect) = classify(&events_frame("env-1")).unwrap();
        assert_eq!(ack.as_deref(), Some("env-1"));
        assert!(!disconnect);
        let Some(SocketEvent::EventsApi(event, event_id)) = event else {
            panic!("expected EventsApi");
        };
        assert_eq!(event_id, "Ev1");
        assert_eq!(event["team"], "T1");
        assert_eq!(event["text"], "hi");
    }

    #[test]
    fn classify_events_api_keeps_existing_team() {
        let mut frame = events_frame("env-2");
        frame["payload"]["event"]["team"] = json!("T9");
        let (_, event, _) = classify(&frame).unwrap();
        let Some(SocketEvent::EventsApi(event, _)) = event else {
            panic!("expected EventsApi");
        };
        assert_eq!(event["team"], "T9");
    }

    #[test]
    fn classify_slash_and_interactive() {
        let slash = json!({
            "envelope_id": "env-s",
            "type": "slash_commands",
            "payload": { "command": "/omo-status", "user_id": "U1" }
        });
        let (ack, event, disconnect) = classify(&slash).unwrap();
        assert_eq!(ack.as_deref(), Some("env-s"));
        assert!(!disconnect);
        assert_eq!(
            event,
            Some(SocketEvent::SlashCommand(
                json!({ "command": "/omo-status", "user_id": "U1" })
            ))
        );

        let interactive = json!({
            "envelope_id": "env-i",
            "type": "interactive",
            "payload": { "type": "block_actions", "actions": [] }
        });
        let (ack, event, _) = classify(&interactive).unwrap();
        assert_eq!(ack.as_deref(), Some("env-i"));
        assert_eq!(
            event,
            Some(SocketEvent::Interactive(
                json!({ "type": "block_actions", "actions": [] })
            ))
        );
    }

    #[test]
    fn classify_hello_disconnect_and_unknown() {
        assert_eq!(
            classify(&json!({ "type": "hello", "num_connections": 1 })),
            None
        );
        let (ack, event, disconnect) =
            classify(&json!({ "type": "disconnect", "reason": "refresh_requested" })).unwrap();
        assert_eq!((ack, event, disconnect), (None, None, true));
        let (ack, event, disconnect) =
            classify(&json!({ "type": "something_new", "envelope_id": "env-u" })).unwrap();
        assert_eq!(
            (ack.as_deref(), event, disconnect),
            (Some("env-u"), None, false)
        );
    }

    #[tokio::test]
    async fn acks_before_forwarding_and_reconnects_after_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let (acks_tx, mut acks_rx) = mpsc::unbounded_channel::<String>();

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            ws.send(Message::text(json!({ "type": "hello" }).to_string()))
                .await
                .unwrap();
            ws.send(Message::text(
                json!({ "type": "disconnect", "reason": "refresh_requested" }).to_string(),
            ))
            .await
            .unwrap();
            while let Some(Ok(msg)) = ws.next().await {
                if msg.is_close() {
                    break;
                }
            }

            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
            ws.send(Message::Ping(b"p".to_vec().into())).await.unwrap();
            ws.send(Message::text(events_frame("env-42").to_string()))
                .await
                .unwrap();
            let mut got_pong = false;
            while let Some(Ok(msg)) = ws.next().await {
                match msg {
                    Message::Pong(data) => got_pong = &data[..] == b"p",
                    Message::Text(text) => acks_tx.send(text.as_str().to_owned()).unwrap(),
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            got_pong
        });

        let (tx, mut rx) = mpsc::channel(8);
        let shutdown = CancellationToken::new();
        let client = tokio::spawn(run_on_url_owned(url, tx, shutdown.clone()));

        let ack = tokio::time::timeout(Duration::from_secs(5), acks_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let ack: Value = serde_json::from_str(&ack).unwrap();
        assert_eq!(ack, json!({ "envelope_id": "env-42" }));

        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let SocketEvent::EventsApi(event, event_id) = event else {
            panic!("expected EventsApi");
        };
        assert_eq!(event_id, "Ev1");
        assert_eq!(event["team"], "T1");
        assert_eq!(event["channel"], "C1");

        shutdown.cancel();
        let result = tokio::time::timeout(Duration::from_secs(5), client)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok());
        let got_pong = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert!(got_pong);
    }

    #[tokio::test]
    async fn shutdown_during_backoff_returns_ok() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        drop(listener);
        let (tx, _rx) = mpsc::channel(1);
        let shutdown = CancellationToken::new();
        let client = tokio::spawn(run_on_url_owned(url, tx, shutdown.clone()));
        shutdown.cancel();
        let result = tokio::time::timeout(Duration::from_secs(5), client)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn watchdog_env_parsing_defaults_to_1200s() {
        assert_eq!(parse_watchdog(None), Duration::from_secs(1200));
        assert_eq!(parse_watchdog(Some(" 30 ")), Duration::from_secs(30));
        assert_eq!(parse_watchdog(Some("0")), Duration::from_secs(1200));
        assert_eq!(parse_watchdog(Some("soon")), Duration::from_secs(1200));
    }

    #[tokio::test]
    async fn silent_socket_is_dropped_by_watchdog_and_reconnected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let (reconnected_tx, mut reconnected_rx) = mpsc::unbounded_channel::<()>();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut silent = tokio_tungstenite::accept_async(socket).await.unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            let _second = tokio_tungstenite::accept_async(socket).await.unwrap();
            reconnected_tx.send(()).unwrap();
            while let Some(Ok(msg)) = silent.next().await {
                if msg.is_close() {
                    return true;
                }
            }
            true
        });

        let (tx, _rx) = mpsc::channel(8);
        let shutdown = CancellationToken::new();
        let client_shutdown = shutdown.clone();
        let client = tokio::spawn(async move {
            run_on_url_with_watchdog(&url, tx, client_shutdown, Duration::from_millis(200)).await
        });

        tokio::time::timeout(Duration::from_secs(10), reconnected_rx.recv())
            .await
            .expect("watchdog should drop the silent socket and reconnect")
            .unwrap();
        assert!(tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap());
        shutdown.cancel();
        let result = tokio::time::timeout(Duration::from_secs(5), client)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok());
    }

    async fn run_on_url_owned(
        url: String,
        tx: mpsc::Sender<SocketEvent>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        run_on_url(&url, tx, shutdown).await
    }
}

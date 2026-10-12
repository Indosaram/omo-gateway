//! Actual Serenity HTTP serialization against an owned, non-forwarding loopback peer.
use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use futures_util::FutureExt;
use omon_gateway::lanes::discord_threads::{DiscordThreads, ThreadStatus};
use serde_json::{json, Value};
use serenity::all::{ChannelId, GuildChannel, HttpBuilder, Message, MessageId};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinSet;
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(5);
const PARENT: u64 = 200;
const THREAD: u64 = 300;
const MESSAGE: u64 = 400;

struct Peer {
    name: String,
    archived: bool,
    requests: mpsc::UnboundedSender<Value>,
}

fn channel(peer: &Peer) -> Value {
    let mut value = serde_json::to_value(GuildChannel::default()).unwrap();
    value["id"] = json!(THREAD.to_string());
    value["guild_id"] = json!("100");
    value["parent_id"] = json!(PARENT.to_string());
    value["type"] = json!(11);
    value["name"] = json!(peer.name);
    value["thread_metadata"] = json!({
        "archived": peer.archived, "auto_archive_duration": 60,
        "archive_timestamp": "2026-10-11T00:00:00Z", "locked": false
    });
    value
}

fn message(channel_id: u64, body: &Value, id: u64) -> Value {
    let mut value = serde_json::to_value(Message::default()).unwrap();
    value["id"] = json!(id.to_string());
    value["channel_id"] = json!(channel_id.to_string());
    value["content"] = body["content"].clone();
    value
}

async fn record_request(
    State(peer): State<Arc<Mutex<Peer>>>,
    method: Method,
    uri: Uri,
    bytes: Bytes,
) -> Response {
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    let mut peer = peer.lock().await;
    peer.requests
        .send(json!({"method": method.as_str(), "path": uri.path(), "body": body}))
        .unwrap();
    let path = uri.path();
    match (method.as_str(), path) {
        ("GET", "/api/v10/users/@me") => Json(json!({
            "id": "99", "username": "local-qa", "discriminator": "0000",
            "avatar": null, "bot": true
        }))
        .into_response(),
        ("POST", "/api/v10/channels/999/messages") => (
            StatusCode::NOT_FOUND,
            Json(json!({"code": 10003, "message": "Unknown Channel"})),
        )
            .into_response(),
        ("POST", "/api/v10/channels/200/messages") => {
            Json(message(PARENT, &body, THREAD)).into_response()
        }
        ("POST", "/api/v10/channels/200/messages/300/threads") => {
            peer.name = body["name"].as_str().unwrap().into();
            Json(channel(&peer)).into_response()
        }
        ("GET", "/api/v10/channels/300") => Json(channel(&peer)).into_response(),
        ("PATCH", "/api/v10/channels/300") => {
            if let Some(name) = body["name"].as_str() {
                peer.name = name.into();
            }
            if let Some(archived) = body["archived"].as_bool() {
                peer.archived = archived;
            }
            Json(channel(&peer)).into_response()
        }
        ("POST", "/api/v10/channels/300/messages") => {
            Json(message(THREAD, &body, MESSAGE)).into_response()
        }
        ("PATCH", "/api/v10/channels/300/messages/400") => {
            Json(message(THREAD, &body, MESSAGE)).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"code": 0, "message": "unexpected local route"})),
        )
            .into_response(),
    }
}

fn safe_mentions(body: &Value) {
    let parse = body["allowed_mentions"]["parse"].as_array().unwrap();
    assert!(!parse.contains(&json!("everyone")));
    assert!(!parse.contains(&json!("roles")));
    assert!(body["allowed_mentions"]["roles"]
        .as_array()
        .is_none_or(Vec::is_empty));
}

#[tokio::test]
async fn lane_start_creates_and_binds_thread_when_only_loopback_is_configured() {
    // Given: an owned HTTP peer, isolated database and tokenless child environment.
    let temp = tempfile::tempdir().unwrap();
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "create done.txt").await.unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let (requests, mut received) = mpsc::unbounded_channel();
    let peer = Arc::new(Mutex::new(Peer {
        name: String::new(),
        archived: false,
        requests,
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(record_request).with_state(peer);
    let mut tasks = JoinSet::new();
    tasks.spawn(async move { axum::serve(listener, app).await.unwrap() });
    let lead = omon_gateway::lanes::runner::lane_key("lead").storage_key();

    // When: invoke the actual CLI, with neither singular nor plural bot tokens.
    let outcome = timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
            .current_dir(temp.path())
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", temp.path())
            .env("DATABASE_URL", &url)
            .env("OMON_DISCORD_API_BASE", format!("http://{address}"))
            .env("OMON_GUARD_MEM_FREE_MIN_PCT", "0")
            .env("OMON_GUARD_DISK_FREE_MIN_GB", "0")
            .env("OMON_GUARD_LOAD_ALERT", "1000000")
            .env("OMON_GUARD_LOAD_OK", "1000000")
            .args(["lane", "start", "tokenless", "--title", "QA", "--brief"])
            .arg(&brief)
            .args([
                "--done",
                "file done.txt",
                "--lead",
                &lead,
                "--model",
                "small",
                "--thread",
                "200",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await;
    tasks.abort_all();
    while let Some(joined) = timeout(BOUND, tasks.join_next()).await.unwrap() {
        assert!(joined.unwrap_err().is_cancelled());
    }

    // Then: creation is acknowledged and the bot/thread binding is persisted.
    let output = outcome.unwrap().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["lane"]["thread_channel_id"], "300");
    let database = omon_gateway::storage::Database::connect(&url)
        .await
        .unwrap();
    let binding: (String, String, String, String) = sqlx::query_as(
        "SELECT l.status,l.parent_channel_id,l.thread_channel_id,b.bot_id FROM lanes l JOIN lane_thread_bindings b ON b.lane_key=l.key WHERE l.key='tokenless'",
    )
    .fetch_one(database.pool()).await.unwrap();
    assert_eq!(
        binding,
        ("working".into(), "200".into(), "300".into(), "99".into())
    );
    for (method, path) in [
        ("GET", "/api/v10/users/@me"),
        ("POST", "/api/v10/channels/200/messages"),
        ("POST", "/api/v10/channels/200/messages/300/threads"),
    ] {
        let request = received.try_recv().unwrap();
        assert_eq!(request["method"], method);
        assert_eq!(request["path"], path);
    }
    assert!(received.try_recv().is_err());
    database.close().await;
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn lifecycle_and_typed_failure_use_only_owned_loopback_http() {
    // Given: request subscription exists before any actual helper call.
    let (requests, mut received) = mpsc::unbounded_channel();
    let peer = Arc::new(Mutex::new(Peer {
        name: String::new(),
        archived: false,
        requests,
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(record_request).with_state(peer);
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let threads = DiscordThreads::with_http(Arc::new(
        HttpBuilder::new("local-fixture-only")
            .client(
                reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(BOUND)
                    .build()
                    .unwrap(),
            )
            .proxy(format!("http://{address}"))
            .ratelimiter_disabled(true)
            .build(),
    ));
    // When: exercise the production helper, not a fake DiscordThreads implementation.
    let result = std::panic::AssertUnwindSafe(async {
        let title = "한".repeat(150);
        let created = threads.create_thread(ChannelId::new(PARENT), "qa-local", &title).await.unwrap();
        assert_eq!(created.starter_message_id.get(), THREAD);
        assert_eq!(created.thread_channel_id.get(), THREAD);
        assert_eq!(created.thread_name.chars().count(), 100);
        assert!(created.thread_name.starts_with("🔄 "));
        let renamed = threads.set_status(ChannelId::new(THREAD), ThreadStatus::Done).await.unwrap();
        assert!(renamed.starts_with("✅ "));
        assert_eq!(renamed.chars().count(), 100);
        let archived = threads.archive(ChannelId::new(THREAD)).await.unwrap();
        assert!(archived.thread_metadata.unwrap().archived);
        assert!(threads.is_archived(ChannelId::new(THREAD)).await.unwrap());
        let restored = threads.unarchive(ChannelId::new(THREAD)).await.unwrap();
        assert!(!restored.thread_metadata.unwrap().archived);
        assert!(!threads.is_archived(ChannelId::new(THREAD)).await.unwrap());
        let posted = threads.post(ChannelId::new(THREAD), "@everyone @here <@&777>").await.unwrap();
        assert_eq!(posted.id.get(), MESSAGE);
        let edited = threads.edit(ChannelId::new(THREAD), MessageId::new(MESSAGE), "edited @everyone <@&777>").await.unwrap();
        assert_eq!(edited.id.get(), MESSAGE);
        let error = threads.post(ChannelId::new(999), "invalid local channel").await.unwrap_err();
        assert_eq!(error.status_code(), Some(reqwest::StatusCode::NOT_FOUND));
        assert_eq!(error.discord_code(), Some(10003));
        let mut trace = Vec::new();
        for _ in 0..11 {
            trace.push(timeout(BOUND, received.recv()).await.unwrap().unwrap());
        }
        let expected = [
            ("POST", "/api/v10/channels/200/messages"),
            ("POST", "/api/v10/channels/200/messages/300/threads"),
            ("GET", "/api/v10/channels/300"),
            ("PATCH", "/api/v10/channels/300"),
            ("PATCH", "/api/v10/channels/300"),
            ("GET", "/api/v10/channels/300"),
            ("PATCH", "/api/v10/channels/300"),
            ("GET", "/api/v10/channels/300"),
            ("POST", "/api/v10/channels/300/messages"),
            ("PATCH", "/api/v10/channels/300/messages/400"),
            ("POST", "/api/v10/channels/999/messages"),
        ];
        for (request, (method, path)) in trace.iter().zip(expected) {
            assert_eq!(request["method"], method);
            assert_eq!(request["path"], path);
        }
        for index in [0, 8, 9, 10] { safe_mentions(&trace[index]["body"]); }
        assert_eq!(trace[0]["body"]["content"], format!("lane qa-local: {title}"));
        assert_eq!(trace[1]["body"]["name"], created.thread_name);
        assert_eq!(trace[3]["body"]["name"], renamed);
        assert_eq!(trace[4]["body"]["archived"], true);
        assert_eq!(trace[6]["body"]["archived"], false);
        assert_eq!(trace[8]["body"]["content"], posted.content);
        assert_eq!(trace[9]["body"]["content"], edited.content);
        assert!(received.try_recv().is_err());
        json!({"surface": "local Serenity HTTP seam; not live Discord", "endpoint": address.to_string(),
            "trace": trace, "starter_id": THREAD, "thread_id": THREAD, "message_id": MESSAGE,
            "failure": {"http_status": 404, "discord_code": 10003},
            "source_sha256": format!("{:x}", Sha256::digest(include_bytes!("../src/lanes/discord_threads.rs"))),
            "test_sha256": format!("{:x}", Sha256::digest(include_bytes!("test_lane_thread_transport.rs")))})
    }).catch_unwind();
    let outcome = timeout(Duration::from_secs(30), result).await;
    // Then: cleanup completes even on assertion failure; no listener survives the test.
    tasks.abort_all();
    while let Some(joined) = timeout(BOUND, tasks.join_next()).await.unwrap() {
        assert!(joined.unwrap_err().is_cancelled());
    }
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    let mut receipt = match outcome {
        Ok(Ok(receipt)) => receipt,
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(error) => panic!("bounded local lifecycle failed: {error}"),
    };
    receipt["cleanup"] = json!({"server_task_joined": true, "listener_closed": true});
    println!("TASK10_LOCAL_RECEIPT={receipt}");
    println!("TASK10_LOCAL_FAILURE={}", receipt["failure"]);
}

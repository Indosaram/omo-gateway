//! retry_queued against an owned loopback Discord REST peer (never live Discord).
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use axum::body::Bytes;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use chrono::Utc;
use omon_gateway::lanes::guard::{retry_queued, GuardThresholds, HostSample, LAUNCH_RETRY_BACKOFF};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use serde_json::{json, Value};
use serenity::all::{GuildChannel, Message};
use tokio::sync::oneshot;
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(5);

type Gate = (oneshot::Sender<()>, oneshot::Receiver<()>);

/// Requests whose path is registered here pause until the test releases them.
static GATES: Mutex<Option<HashMap<String, Gate>>> = Mutex::new(None);
static REQUESTS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static SERVER: OnceLock<SocketAddr> = OnceLock::new();

fn gate(path: &str) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
    let (arrived_tx, arrived_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    GATES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(path.into(), (arrived_tx, release_rx));
    (arrived_rx, release_tx)
}

fn requests_to(path: &str) -> usize {
    REQUESTS
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, p)| p == path)
        .count()
}

async fn handle(method: Method, uri: Uri, bytes: Bytes) -> Response {
    let path = uri.path().to_string();
    REQUESTS
        .lock()
        .unwrap()
        .push((method.as_str().into(), path.clone()));
    let paused = GATES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .remove(&path);
    if let Some((arrived, release)) = paused {
        arrived.send(()).unwrap();
        let _ = release.await;
    }
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    let parts: Vec<&str> = path.trim_start_matches("/api/v10/").split('/').collect();
    match (method.as_str(), parts.as_slice()) {
        ("GET", ["users", "@me"]) => Json(json!({
            "id": "99", "username": "local-qa", "discriminator": "0000",
            "avatar": null, "bot": true
        }))
        .into_response(),
        ("POST", ["channels", "999" | "998", "messages"]) => (
            StatusCode::NOT_FOUND,
            Json(json!({"code": 10003, "message": "Unknown Channel"})),
        )
            .into_response(),
        ("POST", ["channels", parent, "messages"]) => {
            let parent: u64 = parent.parse().unwrap();
            let mut value = serde_json::to_value(Message::default()).unwrap();
            value["id"] = json!((parent + 1000).to_string());
            value["channel_id"] = json!(parent.to_string());
            value["content"] = body["content"].clone();
            Json(value).into_response()
        }
        ("POST", ["channels", parent, "messages", message, "threads"]) => {
            let mut value = serde_json::to_value(GuildChannel::default()).unwrap();
            value["id"] = json!(message.to_string());
            value["guild_id"] = json!("100");
            value["parent_id"] = json!(parent.to_string());
            value["type"] = json!(11);
            value["name"] = body["name"].clone();
            Json(value).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"code": 0, "message": "unexpected local route"})),
        )
            .into_response(),
    }
}

/// One loopback peer per test binary, on its own runtime so it outlives each test runtime.
fn server() {
    SERVER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                tx.send(listener.local_addr().unwrap()).unwrap();
                axum::serve(listener, Router::new().fallback(handle))
                    .await
                    .unwrap();
            });
        });
        let address = rx.recv().unwrap();
        std::env::set_var("OMON_DISCORD_API_BASE", format!("http://{address}"));
        address
    });
}

fn thresholds(lanes_max: u64) -> GuardThresholds {
    GuardThresholds {
        load_alert: 150.0,
        load_ok: 80.0,
        mem_free_min_pct: 20,
        disk_free_min_gb: 50,
        lanes_max,
    }
}

fn host() -> HostSample {
    HostSample::new(10.0, 50, 100, 0)
}

async fn db(dir: &tempfile::TempDir) -> Database {
    Database::connect(&format!(
        "sqlite://{}",
        dir.path().join("guard.db").display()
    ))
    .await
    .unwrap()
}

async fn queued(database: &Database, key: &str, parent: Option<&str>, age_minutes: i64) {
    let mut lane = Lane::new(
        key,
        "Queued",
        "lead",
        "/tmp",
        "/tmp/brief",
        format!("lane:{key}"),
        "file done.txt",
    );
    lane.parent_channel_id = parent.map(Into::into);
    let at = Utc::now() - chrono::Duration::minutes(age_minutes);
    lane.created_at = Some(at);
    lane.updated_at = Some(at);
    insert_lane(database.pool(), &lane).await.unwrap();
}

async fn event_kinds(database: &Database, key: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT kind FROM lane_events WHERE lane_key = ? ORDER BY id")
        .bind(key)
        .fetch_all(database.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn discord_thread_creation_does_not_hold_the_db_connection() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "slow", Some("210"), 1).await;
    let (arrived, release) = gate("/api/v10/channels/210/messages");
    let pool = database.pool().clone();
    let retry = tokio::spawn(async move { retry_queued(&pool, &host(), &thresholds(5)).await });
    timeout(BOUND, arrived).await.unwrap().unwrap();
    // While Discord is slow, the single DB connection must stay usable.
    let probe = timeout(
        Duration::from_secs(2),
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lanes").fetch_one(database.pool()),
    )
    .await;
    release.send(()).unwrap();
    assert_eq!(
        probe
            .expect("DB connection held during Discord call")
            .unwrap(),
        1
    );
    let launched = timeout(BOUND, retry).await.unwrap().unwrap().unwrap();
    assert_eq!(launched, ["slow"]);
}

#[tokio::test]
async fn promoted_thread_is_bound_to_the_creating_bot() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "bound", Some("220"), 1).await;
    let launched = retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .unwrap();
    assert_eq!(launched, ["bound"]);
    let binding: (String, String, String) = sqlx::query_as(
        "SELECT l.status, l.thread_channel_id, b.bot_id FROM lanes l JOIN lane_thread_bindings b ON b.lane_key = l.key WHERE l.key = 'bound'",
    )
    .fetch_one(database.pool())
    .await
    .expect("lane thread binding row");
    assert_eq!(binding, ("working".into(), "1220".into(), "99".into()));
}

#[tokio::test]
async fn one_lanes_discord_failure_neither_aborts_the_pass_nor_reposts() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "good", Some("230"), 2).await;
    queued(&database, "bad", Some("999"), 1).await;
    let first = retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .expect("a single lane failure must not abort the pass");
    assert_eq!(first, ["good"]);
    assert_eq!(event_kinds(&database, "bad").await, ["launch_failed"]);
    let second = retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .unwrap();
    assert!(second.is_empty());
    // The failed lane is not retried (no new starter post) within LAUNCH_RETRY_BACKOFF.
    assert_eq!(requests_to("/api/v10/channels/999/messages"), 1);
    assert_eq!(
        get_lane(database.pool(), "bad")
            .await
            .unwrap()
            .unwrap()
            .status,
        "queued"
    );
}

#[tokio::test]
async fn failed_launch_is_retried_after_the_backoff() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "flaky", Some("998"), 30).await;
    let first = retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .unwrap();
    assert!(first.is_empty());
    assert_eq!(event_kinds(&database, "flaky").await, ["launch_failed"]);
    // Within the backoff the lane is skipped.
    retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .unwrap();
    assert_eq!(requests_to("/api/v10/channels/998/messages"), 1);
    // Age the failure past the backoff; the lane row itself is untouched.
    let backoff_minutes = LAUNCH_RETRY_BACKOFF.as_secs() as i64 / 60 + 1;
    sqlx::query("UPDATE lane_events SET created_at = ? WHERE lane_key = 'flaky'")
        .bind(Utc::now() - chrono::Duration::minutes(backoff_minutes))
        .execute(database.pool())
        .await
        .unwrap();
    retry_queued(database.pool(), &host(), &thresholds(5))
        .await
        .unwrap();
    assert_eq!(requests_to("/api/v10/channels/998/messages"), 2);
}

#[tokio::test]
async fn queued_lanes_are_promoted_oldest_first() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "older", None, 10).await;
    queued(&database, "newer", None, 1).await;
    let launched = retry_queued(database.pool(), &host(), &thresholds(1))
        .await
        .unwrap();
    assert_eq!(launched, ["older"]);
}

#[tokio::test]
async fn lane_closed_during_thread_creation_is_recorded_not_launched() {
    server();
    let dir = tempfile::tempdir().unwrap();
    let database = db(&dir).await;
    queued(&database, "raced", Some("250"), 1).await;
    let (arrived, release) = gate("/api/v10/channels/250/messages/1250/threads");
    let pool = database.pool().clone();
    let retry = tokio::spawn(async move { retry_queued(&pool, &host(), &thresholds(5)).await });
    timeout(BOUND, arrived).await.unwrap().unwrap();
    // The lane leaves 'queued' while Discord is still creating its thread.
    let closed = timeout(
        Duration::from_secs(2),
        sqlx::query("UPDATE lanes SET status = 'closed' WHERE key = 'raced'")
            .execute(database.pool()),
    )
    .await;
    release.send(()).unwrap();
    closed
        .expect("DB connection held during Discord call")
        .unwrap();
    let launched = timeout(BOUND, retry).await.unwrap().unwrap().unwrap();
    assert!(launched.is_empty());
    let lane = get_lane(database.pool(), "raced").await.unwrap().unwrap();
    assert_eq!(lane.status, "closed");
    assert_eq!(lane.thread_channel_id, None);
    let body: String = sqlx::query_scalar(
        "SELECT body FROM lane_events WHERE lane_key = 'raced' AND kind = 'launch_orphan_thread'",
    )
    .fetch_one(database.pool())
    .await
    .expect("orphan thread must be recorded");
    assert!(body.contains("1250"), "{body}");
}

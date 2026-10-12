use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use chrono::{TimeZone, Utc};
use futures_util::FutureExt;
use omon_gateway::lanes::activity::LaneActivity;
use omon_gateway::lanes::discord_threads::DiscordThreads;
use omon_gateway::lanes::posts::{self, Post, PostState};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::MockClock;
use serde_json::{json, Value};
use serenity::all::{GuildChannel, HttpBuilder, Message};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinSet;
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(5);

struct Peer {
    requests: mpsc::UnboundedSender<Value>,
    next_id: u64,
    deleted: bool,
    reject_creation: bool,
}

async fn record(
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
        .send(json!({"method":method.as_str(),"path":uri.path(),"body":body}))
        .unwrap();
    if uri.path() == "/api/v10/channels/300" && method == Method::PATCH {
        let mut channel = serde_json::to_value(GuildChannel::default()).unwrap();
        channel["id"] = json!("300");
        channel["type"] = json!(11);
        channel["name"] = body["name"].clone();
        return Json(channel).into_response();
    }
    let id = if method == Method::POST {
        if peer.reject_creation {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"code":0,"message":"local ambiguous send"})),
            )
                .into_response();
        }
        peer.next_id += 1;
        peer.next_id
    } else if method == Method::PATCH && uri.path().contains("/messages/") {
        if peer.deleted {
            peer.deleted = false;
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"code":10008,"message":"Unknown Message"})),
            )
                .into_response();
        }
        uri.path().rsplit('/').next().unwrap().parse().unwrap()
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":0,"message":"unexpected local route"})),
        )
            .into_response();
    };
    let mut message = serde_json::to_value(Message::default()).unwrap();
    message["id"] = json!(id.to_string());
    message["channel_id"] = json!("300");
    message["content"] = body["content"].clone();
    Json(message).into_response()
}

#[tokio::test]
async fn milestones_status_replacement_and_restart_use_owned_loopback() {
    // Given: an owned SQLite database and a recording HTTP subscription before delivery.
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("status.db").display());
    let db = Database::connect(&url).await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap();
    let mut lane = Lane::new(
        "status-qa",
        "결과 /Users/test/private ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        "lead",
        "/tmp",
        "/tmp/brief",
        "lane",
        "file done",
    );
    lane.status = "working".into();
    lane.thread_channel_id = Some("300".into());
    lane.status_message_id = Some("400".into());
    lane.created_at = Some(now);
    lane.turns_run = 2;
    insert_lane(db.pool(), &lane).await.unwrap();
    let (requests, mut observed) = mpsc::unbounded_channel();
    let peer = Arc::new(Mutex::new(Peer {
        requests,
        next_id: 500,
        deleted: false,
        reject_creation: false,
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(record).with_state(peer.clone());
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let sink = DiscordThreads::with_http(Arc::new(
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
    // When: real captured milestone reports drain through the production HTTP sink.
    let outcome = timeout(Duration::from_secs(40), std::panic::AssertUnwindSafe(async {
        let activity = LaneActivity::new(db.pool().clone(), Arc::new(MockClock::new(now)));
        activity.captured_item(&lane.key, "[REPORT] status-qa | milestone | first\n[REPORT] status-qa | milestone | second", 0, "item-1").await.unwrap();
        assert_eq!(posts::drain_with_status(db.pool(), &sink, now).await.unwrap(), 2);
        let mut trace = Vec::new();
        for _ in 0..5 { trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap()); }
        assert_eq!(trace.iter().filter(|r| r["method"] == "POST").count(), 2);
        assert_eq!(trace.iter().filter(|r| r["path"] == "/api/v10/channels/300/messages/400").count(), 2);
        assert_eq!(posts::drain_with_status(db.pool(), &sink, now).await.unwrap(), 0);
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(14)).await.unwrap();
        assert!(observed.try_recv().is_err());
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(15)).await.unwrap();
        trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap());
        sqlx::query("UPDATE lanes SET status='blocked' WHERE key=?").bind(&lane.key).execute(db.pool()).await.unwrap();
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(16)).await.unwrap();
        let rename = timeout(BOUND, observed.recv()).await.unwrap().unwrap();
        assert!(rename["body"]["name"].as_str().unwrap().starts_with("⏸ "));
        trace.push(rename);
        trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap());
        peer.lock().await.deleted = true;
        posts::refresh_lane_status(db.pool(), &sink, &lane.key, now + chrono::Duration::minutes(17), true).await.unwrap();
        trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap());
        trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap());
        assert_eq!(get_lane(db.pool(), &lane.key).await.unwrap().unwrap().status_message_id.as_deref(), Some("503"));
        for request in &trace {
            let body = &request["body"];
            let text = body["content"].as_str().or_else(|| body["name"].as_str()).unwrap();
            assert!(!text.contains("/Users/test/"));
            assert!(!text.contains("ghp_"));
            assert!(text.chars().count() <= 2000);
            if body.get("content").is_some() {
                assert_eq!(body["allowed_mentions"]["parse"], json!([]));
                assert_eq!(body["allowed_mentions"]["replied_user"], false);
            }
        }
        let rows: Vec<String> = sqlx::query_scalar("SELECT text FROM lane_transcript WHERE role='thread_post'").fetch_all(db.pool()).await.unwrap();
        assert_eq!(rows.len(), 2);
        for text in rows { let post: Post = serde_json::from_str(&text).unwrap(); assert_eq!(post.state, PostState::Sent); assert!(post.message_id.is_some()); }
        sqlx::query("UPDATE lanes SET status='verified_done' WHERE key=?").bind(&lane.key).execute(db.pool()).await.unwrap();
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(18)).await.unwrap();
        let rename = timeout(BOUND, observed.recv()).await.unwrap().unwrap();
        assert!(rename["body"]["name"].as_str().unwrap().starts_with("✅ "));
        trace.push(rename);
        trace.push(timeout(BOUND, observed.recv()).await.unwrap().unwrap());
        // A closed lane's thread is archived by close; Discord rejects edits there
        // (50083), so the status refresh leaves closed lanes alone.
        sqlx::query("UPDATE lanes SET status='closed' WHERE key=?").bind(&lane.key).execute(db.pool()).await.unwrap();
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(18)).await.unwrap();
        assert!(observed.try_recv().is_err());
        posts::refresh_statuses(db.pool(), &sink, now + chrono::Duration::minutes(40)).await.unwrap();
        assert!(observed.try_recv().is_err());
        peer.lock().await.deleted = true;
        peer.lock().await.reject_creation = true;
        posts::refresh_lane_status(db.pool(), &sink, &lane.key, now + chrono::Duration::minutes(18), true).await.unwrap();
        let receipt: String = sqlx::query_scalar("SELECT text FROM lane_transcript WHERE role='status_receipt' ORDER BY id DESC LIMIT 1").fetch_one(db.pool()).await.unwrap();
        assert_eq!(serde_json::from_str::<Value>(&receipt).unwrap()["post"]["state"], "uncertain");
        while observed.try_recv().is_ok() {}
        db.pool().close().await;
        let reopened = Database::connect(&url).await.unwrap();
        posts::recover(reopened.pool()).await.unwrap();
        posts::refresh_statuses(reopened.pool(), &sink, now + chrono::Duration::minutes(40)).await.unwrap();
        assert!(observed.try_recv().is_err());
        assert_eq!(get_lane(reopened.pool(), &lane.key).await.unwrap().unwrap().status_message_id.as_deref(), Some("503"));
        reopened.close().await;
        println!("TASK18_LOCAL_RECEIPT={}", json!({"surface":"owned loopback, not live Discord","two_milestones":2,"status_edits":2,"replacement_id":"503","uncertain_no_retry":true,"trace":trace}));
    }).catch_unwind()).await;
    // Then: server and owned database are closed on success, panic, or bounded timeout.
    tasks.abort_all();
    while let Some(result) = timeout(BOUND, tasks.join_next()).await.unwrap() {
        assert!(result.unwrap_err().is_cancelled());
    }
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    db.close().await;
    match outcome {
        Ok(Ok(())) => (),
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(error) => panic!("bounded status transport: {error}"),
    }
}

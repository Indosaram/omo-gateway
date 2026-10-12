use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use futures_util::FutureExt;
use omon_gateway::lanes::activity::LaneActivity;
use omon_gateway::lanes::discord_threads::DiscordThreads;
use omon_gateway::lanes::posts::{self, Post, PostState};
use omon_gateway::storage::{insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::{Clock, MockClock};
use serde_json::{json, Value};
use serenity::all::{HttpBuilder, Message};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::timeout;

const BOUND: Duration = Duration::from_secs(5);
const REPORT: &str = "[REPORT] qa-1 | milestone | done.txt created.\n";
const ITEM: &str = "lane-item:e983ce62-dbff-4095-977d-558557c8599c:chatcmpl-resp_01766c0e2f8bb957016acbbd5ac8908191b5484d6894ea977d:0";

#[derive(Clone, Copy)]
enum Reply {
    Acknowledge,
    CloseAfterAcceptance,
}

async fn accept_request(stream: &mut TcpStream) -> Value {
    let mut bytes = Vec::new();
    let header_end = loop {
        let mut part = [0; 1024];
        let count = stream.read(&mut part).await.unwrap();
        assert_ne!(count, 0, "request ended before headers");
        bytes.extend_from_slice(&part[..count]);
        assert!(bytes.len() <= 16384);
        if let Some(offset) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break offset + 4;
        }
    };
    let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
    let first = headers.lines().next().unwrap();
    assert_eq!(first, "POST /api/v10/channels/300/messages HTTP/1.1");
    let length: usize = headers
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap();
    assert!(length <= 8192);
    let mut body = bytes[header_end..].to_vec();
    assert!(body.len() <= length);
    body.resize(length, 0);
    let available = bytes.len() - header_end;
    stream.read_exact(&mut body[available..]).await.unwrap();
    json!({"method":"POST", "path":"/api/v10/channels/300/messages", "body":serde_json::from_slice::<Value>(&body).unwrap()})
}

async fn stored(database: &Database) -> Post {
    let text: String = sqlx::query_scalar(
        "SELECT text FROM lane_transcript WHERE role='thread_post' ORDER BY id LIMIT 1",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

async fn scenario(reply: Reply) {
    // Given: a declared replay of the real captured REPORT, in a new owned DB only.
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("qa.db");
    let database = Database::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    let clock = MockClock::new(Utc.with_ymd_and_hms(2026, 10, 11, 16, 46, 25).unwrap());
    let mut lane = Lane::new(
        "qa-1",
        "local replay",
        "lead",
        "/tmp",
        "/tmp/unused",
        "lane",
        "file done.txt",
    );
    lane.status = "working".into();
    lane.thread_channel_id = Some("300".into());
    lane.last_activity_at = clock.now();
    insert_lane(database.pool(), &lane).await.unwrap();
    let activity = LaneActivity::new(database.pool().clone(), Arc::new(clock));
    activity
        .captured_item("qa-1", REPORT, 0, ITEM)
        .await
        .unwrap();
    let pending = stored(&database).await;
    assert_eq!(pending.state, PostState::Pending);
    assert_eq!(pending.body, "done.txt created.");
    assert!(pending.message_id.is_none());
    let (requests, mut observed) = mpsc::unbounded_channel();
    let (release, allow_response) = oneshot::channel();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = timeout(BOUND, accept_request(&mut stream)).await.unwrap();
        requests.send(request.clone()).unwrap();
        timeout(BOUND, allow_response).await.unwrap().unwrap();
        match reply {
            Reply::Acknowledge => {
                let mut response = serde_json::to_value(Message::default()).unwrap();
                response["id"] = json!("400");
                response["channel_id"] = json!("300");
                response["content"] = request["body"]["content"].clone();
                let text = response.to_string();
                let wire = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
                stream.write_all(wire.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
            Reply::CloseAfterAcceptance => drop(stream),
        }
        loop {
            let (mut extra, _) = listener.accept().await.unwrap();
            let request = timeout(BOUND, accept_request(&mut extra)).await.unwrap();
            requests.send(request).unwrap();
            drop(extra);
        }
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
    // When: drain the production ledger through the real DiscordThreads HTTP sink.
    let outcome = timeout(Duration::from_secs(20), std::panic::AssertUnwindSafe(async {
        let worker = posts::drain(database.pool(), &sink);
        tokio::pin!(worker);
        let request = tokio::select! {
            result = &mut worker => panic!("drain ended before peer acceptance: {result:?}"),
            request = timeout(BOUND, observed.recv()) => request.unwrap().unwrap(),
        };
        let sending = stored(&database).await;
        assert_eq!(sending.state, PostState::Sending);
        assert!(sending.message_id.is_none());
        assert_eq!(request["body"]["content"], pending.body);
        assert_eq!(request["body"]["nonce"], pending.nonce);
        assert_eq!(request["body"]["enforce_nonce"], true);
        let parse = request["body"]["allowed_mentions"]["parse"].as_array().unwrap();
        assert!(!parse.contains(&json!("everyone")));
        assert!(!parse.contains(&json!("roles")));
        release.send(()).unwrap();
        let sent_count = timeout(BOUND, &mut worker).await.unwrap().unwrap();
        let after = stored(&database).await;
        match reply {
            Reply::Acknowledge => {
                assert_eq!(sent_count, 1);
                assert_eq!(after.state, PostState::Sent);
                assert_eq!(after.message_id.as_deref(), Some("400"));
                assert!(after.error.is_none());
            }
            Reply::CloseAfterAcceptance => {
                assert_eq!(sent_count, 0);
                assert_eq!(after.state, PostState::Uncertain);
                assert!(after.message_id.is_none());
                assert!(after.error.is_some());
            }
        }
        database.pool().close().await;
        let reopened = Database::connect(&format!("sqlite://{}", db_path.display())).await.unwrap();
        posts::recover(reopened.pool()).await.unwrap();
        assert_eq!(posts::drain(reopened.pool(), &sink).await.unwrap(), 0);
        let restarted = stored(&reopened).await;
        assert_eq!(restarted.state, after.state);
        assert_eq!(restarted.message_id, after.message_id);
        assert_eq!(restarted.nonce, pending.nonce);
        assert!(observed.try_recv().is_err(), "transport retried an accepted request");
        reopened.close().await;
        json!({"surface":"local real Serenity HTTP + persisted post ledger; not live Discord",
            "input":"declared replay of GhvMrT completed REPORT; no new backend capture",
            "original_thread":"01a12bdb-1fe5-7345-ae81-738e3573330f", "completed_item":ITEM,
            "endpoint":address.to_string(), "request":request, "after_ack_or_close":after,
            "after_restart":restarted, "request_count":1, "restart_send_count":0,
            "posts_sha256":format!("{:x}", Sha256::digest(include_bytes!("../src/lanes/posts.rs"))),
            "helper_sha256":format!("{:x}", Sha256::digest(include_bytes!("../src/lanes/discord_threads.rs"))),
            "test_sha256":format!("{:x}", Sha256::digest(include_bytes!("test_lane_post_transport.rs")))})
    }).catch_unwind()).await;
    // Then: server tasks and listener are retired on success, panic and timeout.
    tasks.abort_all();
    while let Some(joined) = timeout(BOUND, tasks.join_next()).await.unwrap() {
        assert!(joined.unwrap_err().is_cancelled());
    }
    assert!(TcpStream::connect(address).await.is_err());
    database.close().await;
    let mut receipt = match outcome {
        Ok(Ok(receipt)) => receipt,
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(error) => panic!("bounded local ledger scenario failed: {error}"),
    };
    receipt["cleanup"] =
        json!({"server_task_joined":true,"listener_closed":true,"owned_db_closed":true});
    println!("TASK12_LOCAL_POST_RECEIPT={receipt}");
}

#[tokio::test]
async fn actual_http_ack_is_persisted_and_not_reposted_after_restart() {
    scenario(Reply::Acknowledge).await;
}

#[tokio::test]
async fn accepted_request_without_ack_is_uncertain_and_never_blindly_retried() {
    scenario(Reply::CloseAfterAcceptance).await;
}

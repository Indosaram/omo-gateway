use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::sessions::{
    control::{SessionControl, Target},
    external::{remote_checks_with, ExternalDriver},
    registry::{CommandOutput, CommandRunner},
};
use omon_gateway::storage::{get_lane, Database, Lane};
use omon_gateway::work_heartbeat::MockClock;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct RemoteHost {
    fail_listing: AtomicBool,
    shutdown: Mutex<Option<CancellationToken>>,
    sends: Mutex<Vec<String>>,
}

fn json_output(value: Value) -> CommandOutput {
    CommandOutput {
        code: 0,
        stdout: serde_json::to_vec(&value).unwrap(),
        stderr: vec![],
    }
}

fn unquote(word: &str) -> String {
    word.strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
        .unwrap_or(word)
        .replace("'\\''", "'")
}

#[async_trait]
impl CommandRunner for RemoteHost {
    async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
        if argv.first().map(String::as_str) != Some("ssh") {
            if machine == "local" || !self.fail_listing.load(Ordering::SeqCst) {
                let sessions = if machine == "local" {
                    json!([])
                } else {
                    json!([{"session_id":"session-1","cwd":"/tmp","status":"running","controls":["send","interrupt"]}])
                };
                return Ok(json_output(sessions));
            }
            if let Some(shutdown) = self.shutdown.lock().unwrap().as_ref() {
                shutdown.cancel();
            }
            return Err("ssh: connect timed out".into());
        }
        let words: Vec<String> = argv.iter().skip(7).map(|word| unquote(word)).collect();
        match words.get(2).map(String::as_str) {
            Some("read") => Ok(json_output(json!({"messages":[],"nextCursor":"next"}))),
            Some("send") => {
                self.sends.lock().unwrap().push(words[4].clone());
                Ok(json_output(json!({"ok":true})))
            }
            _ => {
                if words.iter().any(|word| word == "cargo") {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
                Ok(json_output(json!({})))
            }
        }
    }
}

async fn database(temp: &tempfile::TempDir) -> Database {
    Database::connect(&format!(
        "sqlite://{}?mode=rwc",
        temp.path().join("test.db").display()
    ))
    .await
    .unwrap()
}

fn driver(database: &Database, host: Arc<RemoteHost>) -> ExternalDriver<RemoteHost> {
    ExternalDriver::new(
        database.pool().clone(),
        SessionControl::new(host, vec![]),
        Arc::new(MockClock::new(
            Utc.with_ymd_and_hms(2026, 10, 13, 0, 0, 0).unwrap(),
        )),
    )
}

#[tokio::test]
async fn remote_multiline_goal_is_flattened_and_sent() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let host = Arc::new(RemoteHost::default());
    let driver = driver(&database, host.clone());
    let target: Target = "omarchy:session-1".parse().unwrap();
    let key = driver
        .adopt(
            &target,
            "line1\nline2\r\nline3",
            "file proof.txt",
            omon_gateway::lanes::kibitzer::DEFAULT_LEAD,
        )
        .await
        .unwrap();
    driver.send_goal(&key).await.unwrap();
    let sends = host.sends.lock().unwrap().clone();
    assert_eq!(sends.len(), 1, "{sends:?}");
    assert!(!sends[0].contains(['\n', '\r']), "{:?}", sends[0]);
    assert!(sends[0].starts_with("line1 | line2"), "{:?}", sends[0]);
    let sent: bool =
        sqlx::query_scalar("SELECT sent_at IS NOT NULL FROM external_sessions WHERE lane_key=?")
            .bind(&key)
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(sent, "goal must be marked sent");
    database.close().await;
}

#[tokio::test]
async fn failed_remote_listing_does_not_fail_an_adopted_lane() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let host = Arc::new(RemoteHost::default());
    let driver = driver(&database, host.clone());
    let target: Target = "omarchy:session-1".parse().unwrap();
    let key = driver
        .adopt(
            &target,
            "finish proof",
            "file proof.txt",
            omon_gateway::lanes::kibitzer::DEFAULT_LEAD,
        )
        .await
        .unwrap();
    driver.send_goal(&key).await.unwrap();
    sqlx::query("INSERT INTO lane_inbox(lane_key, text) VALUES (?, ?)")
        .bind(&key)
        .bind("[LEAD] not done: nudge")
        .execute(database.pool())
        .await
        .unwrap();

    let notify = Arc::new(Notify::new());
    let shutdown = CancellationToken::new();
    *host.shutdown.lock().unwrap() = Some(shutdown.clone());
    host.fail_listing.store(true, Ordering::SeqCst);
    notify.notify_one();
    driver
        .drive(&key, notify, shutdown)
        .await
        .expect("one failed listing is a transient error, not a dead session");

    let lane = get_lane(database.pool(), &key).await.unwrap().unwrap();
    assert_ne!(lane.status, "failed");
    let unconsumed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lane_inbox WHERE lane_key=? AND consumed_at IS NULL",
    )
    .bind(&key)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(unconsumed, 1, "the message must stay queued for a retry");
    database.close().await;
}

#[tokio::test(start_paused = true)]
async fn remote_command_done_signal_gets_more_than_twenty_seconds() {
    let host = Arc::new(RemoteHost::default());
    let control = SessionControl::new(host, vec!["omarchy".into()]);
    let target: Target = "omarchy:session-1".parse().unwrap();
    let lane = Lane::new(
        "k",
        "t",
        "lead",
        "/tmp",
        "",
        "s",
        r#"command ["cargo","test"]"#,
    );
    let reasons = remote_checks_with(&control, &target, &lane).await.unwrap();
    assert!(
        reasons.is_empty(),
        "a 60 s command must verify within its own budget: {reasons:?}"
    );
}

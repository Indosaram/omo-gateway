use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::judge::{JevCli, Judge, Purpose};
use omon_gateway::sessions::{
    control::{SessionControl, Target},
    external::{remote_checks_with, ExternalDriver, MIGRATION},
    registry::{CommandOutput, CommandRunner},
};
use omon_gateway::storage::{get_lane, get_lane_history, Database, Lane};
use omon_gateway::work_heartbeat::MockClock;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

enum Step {
    Value(Value),
    Fail(String),
    Code(i32),
}

#[derive(Default)]
struct Fake {
    cwd: String,
    fail_all: bool,
    reads: Mutex<VecDeque<Step>>,
    sends: Mutex<Vec<bool>>,
    pool: Mutex<Option<SqlitePool>>,
    notify: Mutex<Option<Arc<Notify>>>,
    shutdown: Mutex<Option<CancellationToken>>,
}

fn output(code: i32, value: Value) -> CommandOutput {
    CommandOutput {
        code,
        stdout: serde_json::to_vec(&value).unwrap(),
        stderr: vec![],
    }
}

#[async_trait]
impl CommandRunner for Fake {
    async fn run(&self, _machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
        if self.fail_all {
            return Err("ssh: connect timed out".into());
        }
        if argv.iter().any(|word| word == "list") {
            return Ok(output(
                0,
                json!([{"session_id":"session-1","cwd":self.cwd,"status":"running","controls":["send","interrupt"]}]),
            ));
        }
        if argv.iter().any(|word| word == "read") {
            let step = self
                .reads
                .lock()
                .unwrap()
                .pop_front()
                .expect("scripted read");
            let drained = self.reads.lock().unwrap().is_empty();
            if let Some(notify) = self.notify.lock().unwrap().clone() {
                if drained {
                    self.shutdown.lock().unwrap().as_ref().unwrap().cancel();
                } else {
                    notify.notify_one();
                }
            }
            return match step {
                Step::Value(value) => Ok(output(0, value)),
                Step::Fail(message) => Err(message),
                Step::Code(code) => Ok(output(code, json!({}))),
            };
        }
        if argv.iter().any(|word| word == "send") {
            let pool = self.pool.lock().unwrap().clone();
            let free = match pool {
                Some(pool) => {
                    // A transaction held across send owns the single pooled
                    // connection, so this bounded acquire can only time out.
                    match tokio::time::timeout(
                        Duration::from_secs(5),
                        pool.begin_with("BEGIN IMMEDIATE"),
                    )
                    .await
                    {
                        Ok(Ok(tx)) => {
                            tx.rollback().await.unwrap();
                            true
                        }
                        _ => false,
                    }
                }
                None => true,
            };
            self.sends.lock().unwrap().push(free);
            if !free {
                return Err("pool connection held during send".into());
            }
        }
        Ok(output(0, json!({"ok":true})))
    }
}

async fn database(temp: &tempfile::TempDir) -> Database {
    let database = Database::connect(&format!(
        "sqlite://{}?mode=rwc",
        temp.path().join("test.db").display()
    ))
    .await
    .unwrap();
    let migrated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('lanes') WHERE name='external_target')",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    if !migrated {
        sqlx::raw_sql(MIGRATION)
            .execute(database.pool())
            .await
            .unwrap();
    }
    database
}

async fn adopted(
    temp: &tempfile::TempDir,
    database: &Database,
    fake: Fake,
) -> (Arc<Fake>, ExternalDriver<Fake>, String) {
    let fake = Arc::new(Fake {
        cwd: temp.path().to_str().unwrap().into(),
        ..fake
    });
    fake.reads
        .lock()
        .unwrap()
        .push_front(Step::Value(json!({"messages":[],"nextCursor":"baseline"})));
    let driver = ExternalDriver::new(
        database.pool().clone(),
        SessionControl::new(fake.clone(), vec![]),
        Arc::new(MockClock::new(
            Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap(),
        )),
    );
    let key = driver
        .adopt(
            &"local:session-1".parse().unwrap(),
            "finish proof",
            "file proof.txt",
            omon_gateway::lanes::kibitzer::DEFAULT_LEAD,
        )
        .await
        .unwrap();
    (fake, driver, key)
}

#[tokio::test]
async fn remote_check_transport_error_becomes_reason() {
    let control = SessionControl::new(
        Arc::new(Fake {
            fail_all: true,
            ..Fake::default()
        }),
        vec!["omarchy".into()],
    );
    let target: Target = "omarchy:session-1".parse().unwrap();
    let lane = Lane::new("k", "t", "lead", "/tmp", "", "s", "file proof.txt");
    let reasons = remote_checks_with(&control, &target, &lane)
        .await
        .expect("transport errors must not abort the check");
    assert_eq!(reasons.len(), 2, "{reasons:?}");
}

#[tokio::test]
async fn drive_tolerates_two_transient_errors() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let fake = Fake::default();
    fake.reads.lock().unwrap().extend([
        Step::Fail("ssh: connect timed out".into()),
        Step::Fail("ssh: connect timed out".into()),
        Step::Value(json!({"messages":[],"nextCursor":"next"})),
    ]);
    let (fake, driver, key) = adopted(&temp, &database, fake).await;
    let notify = Arc::new(Notify::new());
    let shutdown = CancellationToken::new();
    *fake.shutdown.lock().unwrap() = Some(shutdown.clone());
    *fake.notify.lock().unwrap() = Some(notify.clone());
    notify.notify_one();
    driver
        .drive(&key, notify, shutdown)
        .await
        .expect("two transient errors must not fail the lane");
    assert!(fake.reads.lock().unwrap().is_empty());
    database.close().await;
}

#[tokio::test]
async fn drive_fails_immediately_when_session_not_running() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let fake = Fake::default();
    fake.reads
        .lock()
        .unwrap()
        .extend([Step::Code(3), Step::Code(3)]);
    let (fake, driver, key) = adopted(&temp, &database, fake).await;
    let notify = Arc::new(Notify::new());
    let shutdown = CancellationToken::new();
    *fake.shutdown.lock().unwrap() = Some(shutdown.clone());
    *fake.notify.lock().unwrap() = Some(notify.clone());
    notify.notify_one();
    assert!(driver.drive(&key, notify, shutdown).await.is_err());
    assert_eq!(fake.reads.lock().unwrap().len(), 1);
    database.close().await;
}

#[tokio::test]
async fn sends_run_outside_transactions() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let fake = Fake::default();
    fake.reads
        .lock()
        .unwrap()
        .push_back(Step::Value(json!({"messages":[],"nextCursor":"next"})));
    let (fake, driver, key) = adopted(&temp, &database, fake).await;
    *fake.pool.lock().unwrap() = Some(database.pool().clone());
    sqlx::query("INSERT INTO lane_inbox(lane_key, text) VALUES (?, ?)")
        .bind(&key)
        .bind("owner follow-up")
        .execute(database.pool())
        .await
        .unwrap();
    let notify = Arc::new(Notify::new());
    let shutdown = CancellationToken::new();
    *fake.shutdown.lock().unwrap() = Some(shutdown.clone());
    *fake.notify.lock().unwrap() = Some(notify.clone());
    notify.notify_one();
    driver.drive(&key, notify, shutdown).await.unwrap();
    assert_eq!(*fake.sends.lock().unwrap(), vec![true, true]);
    let unsent: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM external_sessions WHERE lane_key=? AND sent_at IS NULL) + (SELECT count(*) FROM lane_inbox WHERE lane_key=? AND consumed_at IS NULL)",
    )
    .bind(&key)
    .bind(&key)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(unsent, 0);
    database.close().await;
}

#[tokio::test]
async fn release_closes_lane_and_writes_history() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let (_fake, driver, key) = adopted(&temp, &database, Fake::default()).await;
    driver
        .release(&"local:session-1".parse().unwrap())
        .await
        .unwrap();
    let lane = get_lane(database.pool(), &key).await.unwrap().unwrap();
    assert_eq!(lane.status, "closed");
    let history = get_lane_history(database.pool(), &key).await.unwrap();
    let last = history.last().unwrap();
    assert_eq!(last.from_status.as_deref(), Some("working"));
    assert_eq!(last.to_status, "closed");
    assert_eq!(last.reason.as_deref(), Some("external session released"));
    database.close().await;
}

#[tokio::test]
async fn read_new_records_activity_after_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let fake = Fake::default();
    fake.reads.lock().unwrap().push_back(Step::Value(
        json!({"messages":[{"id":"item-1","role":"assistant","text":"progress"}],"nextCursor":"next"}),
    ));
    let (_fake, driver, key) = adopted(&temp, &database, fake).await;
    driver.send_goal(&key).await.unwrap();
    sqlx::query("INSERT INTO lane_history(lane_key,at,from_status,to_status,reason) VALUES(?,?,'verified_done','working','reopened')")
        .bind(&key)
        .bind(Utc::now())
        .execute(database.pool())
        .await
        .unwrap();
    assert_eq!(driver.read_new(&key).await.unwrap(), 1);
    let activity: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lane_transcript WHERE lane_key=? AND role='activity_item'",
    )
    .bind(&key)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(activity, 1);
    database.close().await;
}

#[tokio::test]
async fn persisted_counts_exclude_calls_before_kst_midnight() {
    let temp = tempfile::tempdir().unwrap();
    let database = database(&temp).await;
    let kst_midnight = (Utc::now() + chrono::Duration::hours(9))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        - chrono::Duration::hours(9);
    for (id, at) in [
        ("yesterday", kst_midnight - chrono::Duration::minutes(30)),
        ("today", kst_midnight + chrono::Duration::minutes(30)),
    ] {
        sqlx::query("INSERT INTO judge_calls(id,purpose,kind,ms,outcome,used_fallback,created_at) VALUES(?,'done_diff','diff',1,'ok',0,?)")
            .bind(id)
            .bind(at)
            .execute(database.pool())
            .await
            .unwrap();
    }
    let judge = JevCli::new(None, Some(database.pool().clone()), None, None);
    assert_eq!(judge.sync_persisted_counts().await.unwrap(), (1, 0));
    database.close().await;
}

#[tokio::test]
async fn done_diff_over_4000_chars_reaches_judge_untruncated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(&repo)
            .args(args)
            .output()
            .unwrap();
        assert!(status.status.success(), "{args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    std::fs::write(repo.join("big.txt"), "start\n").unwrap();
    git(&["add", "big.txt"]);
    git(&["commit", "-q", "-m", "initial"]);
    let body = format!("start\n{}\nEND_MARKER\n", "x".repeat(9000));
    std::fs::write(repo.join("big.txt"), body).unwrap();

    let capture = temp.path().join("stdin.txt");
    let bin = temp.path().join("typesafe-judge");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/bash\ncat > \"{}\"\necho \"fulfills=0.90 clean=0.90 risk=0.10\"\n",
            capture.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let judge = JevCli::new(Some(bin), None, None, None).with_jev_enabled(true);
    let verdict = judge
        .diff(Purpose::DoneDiff, "check big change", &repo, None)
        .await
        .unwrap();
    assert_eq!(verdict.fulfills, 0.90);
    let captured = std::fs::read_to_string(&capture).unwrap();
    assert!(captured.chars().count() > 9000, "{}", captured.len());
    assert!(captured.contains("END_MARKER"));
}

#[cfg(unix)]
#[tokio::test]
async fn done_diff_returns_when_judge_never_reads_a_large_stdin() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(&repo)
            .args(args)
            .output()
            .unwrap();
        assert!(status.status.success(), "{args:?}");
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    std::fs::write(repo.join("big.txt"), "start\n").unwrap();
    git(&["add", "big.txt"]);
    git(&["commit", "-q", "-m", "initial"]);
    // Well above the ~64 KiB pipe buffer, below the 200k scrub cap.
    let body = format!("start\n{}\n", "y\n".repeat(80_000));
    std::fs::write(repo.join("big.txt"), body).unwrap();

    // A judge that never reads stdin and outlives the 15 s judge deadline.
    let bin = temp.path().join("typesafe-judge");
    std::fs::write(&bin, "#!/bin/bash\nexec sleep 120\n").unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

    let judge = JevCli::new(Some(bin), None, None, None).with_jev_enabled(true);
    let outcome = tokio::time::timeout(
        Duration::from_secs(60),
        judge.diff(Purpose::DoneDiff, "check big change", &repo, None),
    )
    .await;
    assert!(
        outcome.is_ok(),
        "diff must honour its own deadline even when the judge never drains stdin"
    );
}

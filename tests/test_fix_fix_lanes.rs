use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::lanes::cli::{start, LaneModel, StartArgs};
use omon_gateway::lanes::guard::{GuardThresholds, LaunchDecision};
use omon_gateway::lanes::posts::{self, Post, PostSink};
use omon_gateway::lanes::runner::lane_key;
use omon_gateway::storage::lanes::request_close;
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::{OmonError, Result};

async fn db(dir: &tempfile::TempDir) -> Database {
    Database::connect(&format!("sqlite://{}", dir.path().join("fix.db").display()))
        .await
        .unwrap()
}

fn lane(key: &str, status: &str, thread: Option<&str>) -> Lane {
    let mut lane = Lane::new(
        key,
        "QA",
        lane_key("lead").storage_key(),
        "/tmp",
        "/tmp/brief",
        lane_key(key).storage_key(),
        "file done.txt",
    );
    lane.status = status.into();
    lane.thread_channel_id = thread.map(Into::into);
    lane
}

async fn post_states(db: &Database) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT json_extract(text, '$.state') FROM lane_transcript WHERE role = 'thread_post' ORDER BY id",
    )
    .fetch_all(db.pool())
    .await
    .unwrap()
}

/// Sends succeed; status renames and edits use the trait defaults (always error).
struct SendOnly {
    sends: AtomicUsize,
}

#[async_trait]
impl PostSink for SendOnly {
    async fn send(&self, _post: &Post) -> Result<String> {
        Ok(format!(
            "{}",
            900 + self.sends.fetch_add(1, Ordering::SeqCst)
        ))
    }
}

/// First status send fails; later sends succeed. Renames succeed.
struct FlakyStatus {
    sends: AtomicUsize,
}

#[async_trait]
impl PostSink for FlakyStatus {
    async fn send(&self, _post: &Post) -> Result<String> {
        if self.sends.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(OmonError::Config("lane post transport: refused".into()));
        }
        Ok("777".into())
    }

    async fn rename_status(
        &self,
        _thread: u64,
        _status: omon_gateway::lanes::discord_threads::ThreadStatus,
        _title: &str,
    ) -> Result<()> {
        Ok(())
    }
}

struct NoJudge;

#[async_trait]
impl Judge for NoJudge {
    async fn choice(
        &self,
        _: Purpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        panic!("explicit model size must not consult the judge")
    }
    async fn noul(&self, _: Purpose, _: &str, _: &str) -> std::result::Result<f64, JudgeError> {
        panic!("unused")
    }
    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        panic!("unused")
    }
}

#[tokio::test]
async fn forced_close_after_failed_unforced_intent_rearms_with_force() {
    let dir = tempfile::tempdir().unwrap();
    let db = db(&dir).await;
    insert_lane(db.pool(), &lane("close-fix", "verified_done", None))
        .await
        .unwrap();
    let first = request_close(db.pool(), "close-fix", false, None)
        .await
        .unwrap();
    sqlx::query("UPDATE lane_close_intents SET state='failed', error='boom' WHERE id=?")
        .bind(first)
        .execute(db.pool())
        .await
        .unwrap();
    let second = request_close(db.pool(), "close-fix", true, Some("forced: retry"))
        .await
        .unwrap();
    let (id, state, force, note): (i64, String, bool, Option<String>) = sqlx::query_as(
        "SELECT id, state, force, note FROM lane_close_intents WHERE lane_key='close-fix'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(id, second);
    assert_eq!(state, "pending");
    assert!(force);
    assert_eq!(note.as_deref(), Some("forced: retry"));
}

#[tokio::test]
async fn milestone_status_failure_does_not_abort_drain() {
    let dir = tempfile::tempdir().unwrap();
    let db = db(&dir).await;
    let lane = lane("drain-fix", "working", Some("300"));
    insert_lane(db.pool(), &lane).await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let mut tx = db.pool().begin().await.unwrap();
    posts::enqueue(&mut tx, &lane, "milestone:1", "first", now)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let sink = SendOnly {
        sends: AtomicUsize::new(0),
    };
    let drained = posts::drain_with_status(db.pool(), &sink, now).await;
    assert_eq!(drained.unwrap(), 1);
    assert_eq!(post_states(&db).await, vec!["sent".to_string()]);
}

#[tokio::test]
async fn uncertain_status_receipt_unblocks_after_bounded_interval() {
    let dir = tempfile::tempdir().unwrap();
    let db = db(&dir).await;
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let mut row = lane("status-fix", "working", Some("300"));
    row.created_at = Some(now);
    insert_lane(db.pool(), &row).await.unwrap();
    let sink = FlakyStatus {
        sends: AtomicUsize::new(0),
    };
    posts::refresh_lane_status(db.pool(), &sink, "status-fix", now, true)
        .await
        .unwrap();
    assert_eq!(sink.sends.load(Ordering::SeqCst), 1);
    // Within the bound the unacknowledged receipt still blocks a blind resend.
    posts::refresh_lane_status(
        db.pool(),
        &sink,
        "status-fix",
        now + Duration::minutes(1),
        true,
    )
    .await
    .unwrap();
    assert_eq!(sink.sends.load(Ordering::SeqCst), 1);
    // After the bound the status flow resumes instead of freezing forever.
    posts::refresh_lane_status(
        db.pool(),
        &sink,
        "status-fix",
        now + Duration::minutes(16),
        true,
    )
    .await
    .unwrap();
    assert_eq!(sink.sends.load(Ordering::SeqCst), 2);
    let stored = get_lane(db.pool(), "status-fix").await.unwrap().unwrap();
    assert_eq!(stored.status_message_id.as_deref(), Some("777"));
}

#[tokio::test]
async fn post_without_thread_reaches_terminal_state() {
    let dir = tempfile::tempdir().unwrap();
    let db = db(&dir).await;
    let lane = lane("nothread-fix", "working", None);
    insert_lane(db.pool(), &lane).await.unwrap();
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let mut tx = db.pool().begin().await.unwrap();
    posts::enqueue(&mut tx, &lane, "milestone:1", "first", now)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let sink = SendOnly {
        sends: AtomicUsize::new(0),
    };
    assert_eq!(posts::drain(db.pool(), &sink).await.unwrap(), 0);
    assert_eq!(sink.sends.load(Ordering::SeqCst), 0);
    assert_eq!(post_states(&db).await, vec!["failed".to_string()]);
}

#[tokio::test]
async fn start_rechecks_lane_cap_atomically_with_insert() {
    let dir = tempfile::tempdir().unwrap();
    let db = db(&dir).await;
    let max = GuardThresholds::from_env().lanes_max;
    for index in 0..max {
        insert_lane(db.pool(), &lane(&format!("busy-{index}"), "working", None))
            .await
            .unwrap();
    }
    let brief = dir.path().join("brief.md");
    tokio::fs::write(&brief, "fixture").await.unwrap();
    let args = StartArgs {
        key: "late-fix".into(),
        title: "late".into(),
        brief,
        done: "file done.txt".into(),
        thread: None,
        cwd: Some(dir.path().to_path_buf()),
        lead: Some(lane_key("lead").storage_key()),
        model: LaneModel::Small,
    };
    // A decision sampled before the other lanes launched is stale by insert time.
    let (code, _) = start(db.pool(), args, &NoJudge, LaunchDecision::Ok)
        .await
        .unwrap();
    assert_eq!(code, 4);
    let stored = get_lane(db.pool(), "late-fix").await.unwrap().unwrap();
    assert_eq!(stored.status, "queued");
}

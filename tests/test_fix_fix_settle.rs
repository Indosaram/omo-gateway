use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::{
    lanes::{
        settle::Settler,
        signals::{DoneSignal, SignalResult, SignalRunner},
    },
    storage::{get_lane, insert_lane, Database, Lane},
    work_heartbeat::MockClock,
};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap()
}

struct Signals;
#[async_trait]
impl SignalRunner for Signals {
    async fn check(&self, signal: &DoneSignal, _: &Path) -> SignalResult {
        SignalResult::pass(signal.to_string(), "verified")
    }
}

fn claimed(key: &str, cwd: &str, brief: &str) -> Lane {
    let mut lane = Lane::new(key, key, "lead", cwd, brief, "session", "file proof");
    lane.status = "done_claimed".into();
    lane.claim_at = Some(now() - chrono::Duration::minutes(5));
    lane
}

fn settler(db: &Database) -> Settler {
    Settler::new(
        db.pool().clone(),
        Arc::new(MockClock::new(now())),
        Arc::new(Signals),
    )
}

async fn git(cwd: &Path, args: &[&str]) {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn per_lane_check_error_rejects_lane_and_settle_continues() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path().to_str().unwrap();
    let db = Database::connect("sqlite::memory:").await.unwrap();
    insert_lane(db.pool(), &claimed("broken", cwd, "/missing-brief"))
        .await
        .unwrap();
    insert_lane(db.pool(), &claimed("healthy", cwd, "/missing-brief"))
        .await
        .unwrap();
    // An unparsable remote target makes remote_checks return Err for this lane only.
    sqlx::query(
        "UPDATE lanes SET kind='external', external_target='not-a-target' WHERE key='broken'",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let verified = settler(&db).settle_due().await.unwrap();
    assert_eq!(verified, vec!["healthy".to_owned()]);
    let broken = get_lane(db.pool(), "broken").await.unwrap().unwrap();
    assert_eq!(broken.status, "not_done");
    assert!(broken.objection.unwrap().contains("remote checks"));
}

#[tokio::test]
async fn other_branches_unpushed_commits_do_not_block_lane() {
    let temp = tempfile::tempdir().unwrap();
    let remote = temp.path().join("remote.git");
    let repo = temp.path().join("repo");
    git(
        temp.path(),
        &["init", "--quiet", "--bare", remote.to_str().unwrap()],
    )
    .await;
    git(temp.path(), &["init", "--quiet", repo.to_str().unwrap()]).await;
    git(&repo, &["commit", "--quiet", "--allow-empty", "-m", "base"]).await;
    git(
        &repo,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    )
    .await;
    git(
        &repo,
        &["push", "--quiet", "origin", "HEAD:refs/heads/lane"],
    )
    .await;
    git(&repo, &["fetch", "--quiet", "origin"]).await;
    // Another lane's branch carries a local-only commit; HEAD stays pushed.
    git(&repo, &["branch", "other"]).await;
    git(&repo, &["checkout", "--quiet", "other"]).await;
    git(
        &repo,
        &["commit", "--quiet", "--allow-empty", "-m", "other lane"],
    )
    .await;
    git(&repo, &["checkout", "--quiet", "-"]).await;
    let db = Database::connect("sqlite::memory:").await.unwrap();
    insert_lane(
        db.pool(),
        &claimed("lane", repo.to_str().unwrap(), "/missing-brief"),
    )
    .await
    .unwrap();
    assert_eq!(
        settler(&db).settle_due().await.unwrap(),
        vec!["lane".to_owned()]
    );

    // An unpushed commit on HEAD itself still rejects the lane.
    git(&repo, &["commit", "--quiet", "--allow-empty", "-m", "mine"]).await;
    let db = Database::connect("sqlite::memory:").await.unwrap();
    insert_lane(
        db.pool(),
        &claimed("lane", repo.to_str().unwrap(), "/missing-brief"),
    )
    .await
    .unwrap();
    assert!(settler(&db).settle_due().await.unwrap().is_empty());
    let lane = get_lane(db.pool(), "lane").await.unwrap().unwrap();
    assert!(lane.objection.unwrap().contains("unpushed"));
}

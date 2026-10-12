use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::{
    judge::{
        Choice, DiffVerdict, FakeJudgeFallback, JevCli, Judge, JudgeError, Purpose as JudgePurpose,
    },
    lanes::{
        settle::Settler,
        signals::{DoneSignal, SignalResult, SignalRunner},
    },
    storage::{get_lane, insert_lane, Database, Lane},
    usage::{self, UsageArgs},
    work_heartbeat::MockClock,
};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap()
}

#[tokio::test]
async fn usage_json_matches_seeded_judge_rows_and_since_window() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    for (id, fallback, outcome) in [
        ("a", 0, "failed"),
        ("b", 1, "result"),
        ("c", 0, "capped:result"),
    ] {
        sqlx::query("INSERT INTO judge_calls(id,purpose,kind,ms,outcome,used_fallback,created_at) VALUES(?,'done_diff','diff',1,?,?,?)").bind(id).bind(outcome).bind(fallback).bind(now()).execute(db.pool()).await.unwrap();
    }
    let text = usage::render(
        db.pool(),
        &UsageArgs {
            since: "1h".into(),
            json: true,
        },
        now() + chrono::Duration::minutes(30),
    )
    .await
    .unwrap();
    let report: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(report.get("tiers").is_none());
    assert_eq!(report["judge_calls"], 3);
    assert_eq!(report["fallback_calls"], 1);
    assert!((report["fallback_rate"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-9);
    assert_eq!(report["cap_reached"], true);
    assert!(usage::render(
        db.pool(),
        &UsageArgs {
            since: "0h".into(),
            json: true
        },
        now()
    )
    .await
    .is_err());
}

#[tokio::test]
async fn yesterday_uses_kst_day_and_inbox_filter_counts() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let at = now() - chrono::Duration::hours(1);
    sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,raw_hash,collected_at) VALUES('x','mail','x','x',?,'x',?)").bind(at).bind(at).execute(db.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO inbox_filter_stats(day,source,rule,count) VALUES('2026-10-11','mail','ads',4)",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let report = usage::yesterday(db.pool(), now() + chrono::Duration::days(1))
        .await
        .unwrap();
    assert_eq!((report.inbox_collected, report.inbox_passed), (5, 1));
    assert!(!report.digest_line().is_empty());
}

struct FakeJudge {
    verdict: Option<DiffVerdict>,
}
#[async_trait]
impl Judge for FakeJudge {
    async fn choice(
        &self,
        _: JudgePurpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        unreachable!()
    }
    async fn noul(
        &self,
        _: JudgePurpose,
        _: &str,
        _: &str,
    ) -> std::result::Result<f64, JudgeError> {
        unreachable!()
    }
    async fn diff(
        &self,
        purpose: JudgePurpose,
        prompt: &str,
        repo: &Path,
        revision: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        assert_eq!(purpose, JudgePurpose::DoneDiff);
        assert!(prompt.chars().count() > 1500);
        let revision = revision.expect("base revision");
        // The judge sees the lane's real paths: done.txt is added at its own path.
        let captured = tokio::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["diff", "--cached", "--name-status", revision, "--"])
            .output()
            .await
            .unwrap();
        assert!(captured.status.success());
        let names = String::from_utf8_lossy(&captured.stdout);
        assert!(names.lines().any(|line| line == "A\tdone.txt"), "{names}");
        self.verdict
            .ok_or_else(|| JudgeError::FallbackFailed("unavailable".into()))
    }
}
struct Signals;
#[async_trait]
impl SignalRunner for Signals {
    async fn check(&self, signal: &DoneSignal, _: &Path) -> SignalResult {
        SignalResult::pass(signal.to_string(), "verified")
    }
}

async fn settle_case(verdict: Option<DiffVerdict>, expected: &str) {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    tokio::fs::create_dir(&repo).await.unwrap();
    let output = tokio::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repo)
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["hash-object", "-t", "tree", "-w", "--stdin"])
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    tokio::fs::write(repo.join("done.txt"), "complete")
        .await
        .unwrap();
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "Implement the feature".repeat(200))
        .await
        .unwrap();
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let mut lane = Lane::new(
        "test",
        "Test",
        "lead",
        repo.to_str().unwrap(),
        brief.to_str().unwrap(),
        "session",
        "file proof",
    );
    lane.status = "done_claimed".into();
    lane.claim_at = Some(now() - chrono::Duration::minutes(5));
    insert_lane(db.pool(), &lane).await.unwrap();
    sqlx::query("INSERT INTO lane_usage_context(lane_key,start_commit,purpose) VALUES('test','4b825dc642cb6eb9a060e54bf8d69288fbee4904','lane_small')").execute(db.pool()).await.unwrap();
    let settler = Settler::new(
        db.pool().clone(),
        Arc::new(MockClock::new(now())),
        Arc::new(Signals),
    )
    .with_judge(Arc::new(FakeJudge { verdict }));
    let verified = settler.settle_due().await.unwrap();
    let result = get_lane(db.pool(), "test").await.unwrap().unwrap();
    assert_eq!(result.status, expected);
    if expected == "not_done" {
        assert!(verified.is_empty());
        let objection = result.objection.unwrap();
        assert!(objection.contains("fulfills=") && objection.contains("risk="));
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM lane_inbox WHERE lane_key='test'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(count, 1);
    } else {
        assert_eq!(verified, vec!["test"]);
    }
    let history: String = sqlx::query_scalar(
        "SELECT reason FROM lane_history WHERE reason LIKE 'done_diff%' ORDER BY id LIMIT 1",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    if verdict.is_none() {
        assert!(history.starts_with("done_diff skipped"));
    } else {
        assert!(
            history.contains("fulfills=")
                && history.contains("clean=")
                && history.contains("risk=")
        );
    }
}

#[tokio::test]
async fn unrelated_diff_reopens_claim_with_numbers() {
    settle_case(Some(DiffVerdict::new(0.01, 0.02, 1.57)), "not_done").await;
}
#[tokio::test]
async fn high_risk_diff_reopens_claim() {
    settle_case(Some(DiffVerdict::new(0.9, 0.9, 1.51)), "not_done").await;
}
#[tokio::test]
async fn threshold_verdict_allows_verified_done() {
    settle_case(Some(DiffVerdict::new(0.65, 0.1, 1.5)), "verified_done").await;
}
#[tokio::test]
async fn unavailable_judge_proceeds_with_signals_and_records_skip() {
    settle_case(None, "verified_done").await;
}

#[tokio::test]
async fn done_diff_disabled_and_capped_never_call_model_fallback() {
    let fallback = Arc::new(FakeJudgeFallback::default());
    let judge = JevCli::new(None, None, Some(fallback.clone()), None).with_jev_enabled(false);
    assert!(judge
        .diff(JudgePurpose::DoneDiff, "goal", Path::new("/"), None)
        .await
        .is_err());
    let judge = JevCli::new(None, None, Some(fallback.clone()), None)
        .with_jev_enabled(true)
        .with_caps(0, 50);
    assert!(matches!(
        judge
            .diff(JudgePurpose::DoneDiff, "goal", Path::new("/"), None)
            .await,
        Err(JudgeError::DailyCapExceeded { .. })
    ));
    assert_eq!(
        fallback
            .call_count
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

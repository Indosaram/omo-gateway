use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::cli::change_claim_at;
use omon_gateway::lanes::runner::{lane_key, LaneRunner};
use omon_gateway::lanes::signals::{DoneSignal, SignalResult, SignalRunner};
use omon_gateway::storage::lanes::{is_lane_paused, pause_lane};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::{Clock, MockClock};
use omon_gateway::{InboundEvent, Result, SessionContext};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Notify, Semaphore};
use tokio_util::sync::CancellationToken;

struct ClaimBackend {
    pool: SqlitePool,
    clock: Arc<MockClock>,
    calls: AtomicUsize,
    claimed: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl AgentBackend for ClaimBackend {
    async fn run(&self, session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        change_claim_at(
            &self.pool,
            &session.key.channel_id,
            true,
            "scripted evidence",
            self.clock.now(),
        )
        .await?;
        self.claimed.send(()).unwrap();
        Ok(())
    }
}

struct Checks {
    calls: AtomicUsize,
    ok: bool,
    failure_detail: &'static str,
}

#[async_trait]
impl SignalRunner for Checks {
    async fn check(&self, signal: &DoneSignal, _cwd: &Path) -> SignalResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.ok {
            SignalResult::pass(signal.to_string(), "checked live")
        } else {
            SignalResult::fail(signal.to_string(), self.failure_detail)
        }
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    db: Database,
    clock: Arc<MockClock>,
    lane: Lane,
    backend: Arc<ClaimBackend>,
    checks: Arc<Checks>,
    claimed: mpsc::UnboundedReceiver<()>,
}

async fn git(path: &Path, args: &[&str]) {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

impl Fixture {
    async fn new(ok: bool) -> Self {
        Self::build(ok, true, true).await
    }

    async fn new_without_remote(ok: bool) -> Self {
        Self::build(ok, false, true).await
    }

    async fn new_unborn(ok: bool) -> Self {
        Self::build(ok, false, false).await
    }

    async fn build(ok: bool, with_remote: bool, with_commit: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        tokio::fs::create_dir(&repo).await.unwrap();
        let repo = tokio::fs::canonicalize(repo).await.unwrap();
        git(&repo, &["init"]).await;
        if with_commit {
            tokio::fs::write(repo.join("tracked.txt"), "original\n")
                .await
                .unwrap();
            git(&repo, &["add", "tracked.txt"]).await;
            git(
                &repo,
                &[
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "-m",
                    "fixture",
                ],
            )
            .await;
            if with_remote {
                let remote = temp.path().join("remote.git");
                tokio::fs::create_dir(&remote).await.unwrap();
                git(&remote, &["init", "--bare"]).await;
                git(
                    &repo,
                    &["remote", "add", "origin", remote.to_str().unwrap()],
                )
                .await;
                git(&repo, &["push", "-q", "origin", "HEAD:refs/heads/fixture"]).await;
            }
        }
        let brief = temp.path().join("brief.md");
        tokio::fs::write(&brief, "Make the proof artifact then claim.")
            .await
            .unwrap();
        let db = Database::connect(&format!(
            "sqlite://{}?mode=rwc",
            temp.path().join("lanes.db").display()
        ))
        .await
        .unwrap();
        let clock = Arc::new(MockClock::new(
            Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap(),
        ));
        let mut lane = Lane::new(
            "done-test",
            "Done test",
            "lead",
            repo.to_str().unwrap(),
            brief.to_str().unwrap(),
            lane_key("done-test").storage_key(),
            "file proof.txt",
        );
        lane.status = "working".into();
        lane.created_at = Some(clock.now());
        insert_lane(db.pool(), &lane).await.unwrap();
        let (claimed, receive) = mpsc::unbounded_channel();
        let backend = Arc::new(ClaimBackend {
            pool: db.pool().clone(),
            clock: clock.clone(),
            calls: AtomicUsize::new(0),
            claimed,
        });
        Self {
            _temp: temp,
            db,
            clock,
            lane,
            backend,
            checks: Arc::new(Checks {
                calls: AtomicUsize::new(0),
                ok,
                failure_detail: "missing proof artifact",
            }),
            claimed: receive,
        }
    }

    fn runner(&self) -> LaneRunner {
        LaneRunner::new(self.db.pool().clone(), self.backend.clone())
            .with_settlement(self.clock.clone(), self.checks.clone())
    }

    async fn claim(&mut self) {
        let runner = self.runner();
        let key = self.lane.key.clone();
        let task = tokio::spawn(async move {
            runner
                .run_lane(&key, Arc::new(Notify::new()), CancellationToken::new())
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), self.claimed.recv())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    async fn status(&self) -> Lane {
        get_lane(self.db.pool(), &self.lane.key)
            .await
            .unwrap()
            .unwrap()
    }
}

#[tokio::test]
async fn claim_then_object_queues_reasons_and_returns_not_done() {
    let mut fixture = Fixture::new(true).await;
    fixture.claim().await;
    change_claim_at(
        fixture.db.pool(),
        &fixture.lane.key,
        false,
        "missing regression coverage",
        fixture.clock.now(),
    )
    .await
    .unwrap();
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    assert_eq!(
        row.objection.as_deref(),
        Some("missing regression coverage")
    );
    let messages: Vec<String> = sqlx::query_scalar(
        "SELECT text FROM lane_inbox WHERE lane_key = ? AND consumed_at IS NULL ORDER BY id",
    )
    .bind(&fixture.lane.key)
    .fetch_all(fixture.db.pool())
    .await
    .unwrap();
    assert!(messages
        .last()
        .unwrap()
        .contains(row.objection.as_deref().unwrap()));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn passing_signals_and_clean_tree_verify_only_after_five_minutes() {
    let mut fixture = Fixture::new(true).await;
    fixture.claim().await;
    let runner = fixture.runner();
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::seconds(299));
    assert!(runner.settle_due().await.unwrap().is_empty());
    assert_eq!(fixture.status().await.status, "done_claimed");
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 0);
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::seconds(1));
    assert_eq!(
        runner.settle_due().await.unwrap(),
        vec![fixture.lane.key.clone()]
    );
    assert_eq!(fixture.status().await.status, "verified_done");
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failing_signal_returns_not_done_with_live_detail() {
    let mut fixture = Fixture::new(false).await;
    fixture.claim().await;
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    assert!(fixture.runner().settle_due().await.unwrap().is_empty());
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    assert!(row.objection.unwrap().contains("missing proof artifact"));
    let body: String = sqlx::query_scalar("SELECT body FROM lane_events WHERE kind = 'not_done'")
        .fetch_one(fixture.db.pool())
        .await
        .unwrap();
    assert!(body.contains("missing proof artifact"));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn dirty_tracked_file_returns_uncommitted_reason() {
    let mut fixture = Fixture::new(true).await;
    fixture.claim().await;
    tokio::fs::write(
        Path::new(&fixture.lane.cwd).join("tracked.txt"),
        "modified\n",
    )
    .await
    .unwrap();
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    fixture.runner().settle_due().await.unwrap();
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    assert!(row.objection.unwrap().contains("uncommitted"));
}

#[tokio::test]
async fn runner_restart_uses_persisted_deadline_and_settles_once() {
    let mut fixture = Fixture::new(true).await;
    fixture.claim().await;
    let old = fixture.runner();
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(4));
    assert!(old.settle_due().await.unwrap().is_empty());
    drop(old);
    let restarted = fixture.runner();
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(1));
    assert_eq!(
        restarted.settle_due().await.unwrap(),
        vec![fixture.lane.key.clone()]
    );
    assert!(restarted.settle_due().await.unwrap().is_empty());
    assert!(fixture.runner().settle_due().await.unwrap().is_empty());
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), 1);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_history WHERE to_status = 'verified_done'")
            .fetch_one(fixture.db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn interrupted_settlement_recovers_without_replaying_signals() {
    let mut fixture = Fixture::new(true).await;
    fixture.claim().await;
    let claim_at = fixture.status().await.claim_at.unwrap();
    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'done_claimed', 'done_claimed', 'settlement_started')")
        .bind(&fixture.lane.key).bind(claim_at).execute(fixture.db.pool()).await.unwrap();
    fixture.clock.set(claim_at + chrono::Duration::minutes(5));
    let restarted = fixture.runner();
    assert!(restarted.settle_due().await.unwrap().is_empty());
    assert!(restarted.settle_due().await.unwrap().is_empty());
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    assert!(row
        .objection
        .as_deref()
        .unwrap()
        .contains("settlement interrupted"));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 0);
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_events WHERE kind = 'not_done'")
            .fetch_one(fixture.db.pool())
            .await
            .unwrap();
    assert_eq!(events, 1);
    change_claim_at(
        fixture.db.pool(),
        &fixture.lane.key,
        true,
        "fresh evidence",
        fixture.clock.now(),
    )
    .await
    .unwrap();
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    assert_eq!(
        restarted.settle_due().await.unwrap(),
        vec![fixture.lane.key.clone()]
    );
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unpushed_commit_with_local_bare_remote_returns_not_done() {
    let mut fixture = Fixture::new(true).await;
    let repo = Path::new(&fixture.lane.cwd);
    tokio::fs::write(repo.join("tracked.txt"), "committed change\n")
        .await
        .unwrap();
    git(repo, &["add", "tracked.txt"]).await;
    git(
        repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "unpushed fixture",
        ],
    )
    .await;
    fixture.claim().await;
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    assert!(fixture.runner().settle_due().await.unwrap().is_empty());
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    let reason = row.objection.unwrap();
    assert!(
        reason.contains(&format!("unpushed {}", fixture.lane.cwd)),
        "actual settlement objection: {reason}"
    );
    assert!(!reason.contains("(no remote)"));
    assert!(!reason.contains("uncommitted"));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unpushed_commit_without_any_remote_returns_not_done() {
    let mut fixture = Fixture::new_without_remote(true).await;
    fixture.claim().await;
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    assert!(fixture.runner().settle_due().await.unwrap().is_empty());
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    let reason = row.objection.unwrap();
    assert!(
        reason.contains(&format!("unpushed {} (no remote)", fixture.lane.cwd)),
        "actual settlement objection: {reason}"
    );
    assert!(!reason.contains("uncommitted"));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn repo_without_commits_and_remote_is_not_refused() {
    let mut fixture = Fixture::new_unborn(true).await;
    fixture.claim().await;
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    let verified = fixture.runner().settle_due().await.unwrap();
    assert_eq!(verified, vec![fixture.lane.key.clone()]);
    let row = fixture.status().await;
    assert_eq!(row.status, "verified_done");
    assert!(row.objection.is_none());
}

#[tokio::test]
async fn settlement_event_scrubs_generated_detail_and_preserves_local_reason() {
    // Given: a live checker failure with a private path and synthetic token.
    let mut fixture = Fixture::new(false).await;
    let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    Arc::get_mut(&mut fixture.checks).unwrap().failure_detail =
        "/Users/alice/proof.txt ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    fixture.claim().await;
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));

    // When: the persisted claim becomes due.
    fixture.runner().settle_due().await.unwrap();

    // Then: the generated event is scrubbed, but the local objection stays useful.
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    let reason = row.objection.unwrap();
    assert!(reason.contains("/Users/alice/proof.txt"));
    assert!(reason.contains(token));
    let body: String = sqlx::query_scalar("SELECT body FROM lane_events WHERE kind = 'not_done'")
        .fetch_one(fixture.db.pool())
        .await
        .unwrap();
    assert!(!body.contains("/Users/alice"));
    assert!(!body.contains(token));
    assert!(body.contains("~/proof.txt"));
    assert!(body.contains("<redacted>"));
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

struct HeldCheck {
    calls: AtomicUsize,
    entered: mpsc::UnboundedSender<()>,
    release: Semaphore,
    old_ok: bool,
}

#[async_trait]
impl SignalRunner for HeldCheck {
    async fn check(&self, signal: &DoneSignal, _cwd: &Path) -> SignalResult {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.entered.send(()).unwrap();
            self.release.acquire().await.unwrap().forget();
            if !self.old_ok {
                return SignalResult::fail(signal.to_string(), "stale old failure");
            }
        }
        SignalResult::pass(signal.to_string(), "current check")
    }
}

#[tokio::test]
async fn same_timestamp_reclaim_rejects_old_success_and_failure_results() {
    for old_ok in [true, false] {
        let mut fixture = Fixture::new(true).await;
        fixture.claim().await;
        let claim_at = fixture.status().await.claim_at.unwrap();
        let (entered, mut entry) = mpsc::unbounded_channel();
        let checks = Arc::new(HeldCheck {
            calls: AtomicUsize::new(0),
            entered,
            release: Semaphore::new(0),
            old_ok,
        });
        fixture.clock.set(claim_at + chrono::Duration::minutes(5));
        let runner = fixture
            .runner()
            .with_settlement(fixture.clock.clone(), checks.clone());
        let old = runner.clone();
        let attempt = tokio::spawn(async move { old.settle_due().await });
        tokio::time::timeout(Duration::from_secs(5), entry.recv())
            .await
            .unwrap()
            .unwrap();
        change_claim_at(
            fixture.db.pool(),
            &fixture.lane.key,
            false,
            "new objection",
            claim_at,
        )
        .await
        .unwrap();
        change_claim_at(
            fixture.db.pool(),
            &fixture.lane.key,
            true,
            "new claim",
            claim_at,
        )
        .await
        .unwrap();
        let before: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM lane_history), (SELECT count(*) FROM lane_events), (SELECT count(*) FROM lane_inbox)"
        ).fetch_one(fixture.db.pool()).await.unwrap();
        checks.release.add_permits(1);
        assert!(tokio::time::timeout(Duration::from_secs(5), attempt)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .is_empty());
        let after: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM lane_history), (SELECT count(*) FROM lane_events), (SELECT count(*) FROM lane_inbox)"
        ).fetch_one(fixture.db.pool()).await.unwrap();
        assert_eq!(before, after, "old attempt must write no result artifacts");
        let row = fixture.status().await;
        assert_eq!(row.status, "done_claimed");
        assert_eq!(row.claim_at, Some(claim_at));
        assert_eq!(row.claim_evidence.as_deref(), Some("new claim"));
        assert!(row.objection.is_none());
        fixture.clock.set(claim_at + chrono::Duration::seconds(299));
        assert!(runner.settle_due().await.unwrap().is_empty());
        assert_eq!(checks.calls.load(Ordering::SeqCst), 1);
        fixture.clock.set(claim_at + chrono::Duration::minutes(5));
        assert_eq!(
            runner.settle_due().await.unwrap(),
            vec![fixture.lane.key.clone()]
        );
        assert!(runner.settle_due().await.unwrap().is_empty());
        assert_eq!(checks.calls.load(Ordering::SeqCst), 2);
        assert_eq!(fixture.status().await.status, "verified_done");
    }
}

struct CheckCompletion(mpsc::UnboundedSender<()>);

impl Drop for CheckCompletion {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct ShutdownCheck {
    entered: mpsc::UnboundedSender<()>,
    completed: mpsc::UnboundedSender<()>,
    release: Semaphore,
}

#[async_trait]
impl SignalRunner for ShutdownCheck {
    async fn check(&self, signal: &DoneSignal, _cwd: &Path) -> SignalResult {
        let _completion = CheckCompletion(self.completed.clone());
        self.entered.send(()).unwrap();
        self.release.acquire().await.unwrap().forget();
        SignalResult::pass(signal.to_string(), "shutdown check")
    }
}

struct ShutdownBackend {
    entered: mpsc::UnboundedSender<()>,
    cancelled: mpsc::UnboundedSender<()>,
    completed: mpsc::UnboundedSender<()>,
    release: Semaphore,
}

struct ServiceErrorBackend {
    held: Arc<ShutdownBackend>,
    release: Semaphore,
    pool: SqlitePool,
    post_error: bool,
    join_error: bool,
}

#[async_trait]
impl AgentBackend for ServiceErrorBackend {
    async fn run(&self, _session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
        unreachable!("cancelable backend required")
    }

    async fn run_cancelable(
        &self,
        session: &mut SessionContext,
        event: InboundEvent,
        cancellation: CancellationToken,
    ) -> Result<()> {
        if session.key.channel_id == "error-lane" {
            self.release.acquire().await.unwrap().forget();
            if self.post_error {
                if !self.join_error {
                    sqlx::query("UPDATE lanes SET status = 'blocked' WHERE key = 'error-lane'")
                        .execute(&self.pool)
                        .await?;
                }
                return Ok(());
            }
            panic!("injected lane turn failure");
        }
        self.held.run_cancelable(session, event, cancellation).await
    }
}

struct LocalPostSink;

struct PanicJudge;

#[async_trait]
impl omon_gateway::judge::Judge for PanicJudge {
    async fn choice(
        &self,
        _purpose: omon_gateway::judge::Purpose,
        _state: &str,
        _instructions: &str,
        _options: &[&str],
    ) -> std::result::Result<omon_gateway::judge::Choice, omon_gateway::judge::JudgeError> {
        panic!("injected lane task join failure");
    }

    async fn noul(
        &self,
        _purpose: omon_gateway::judge::Purpose,
        _state: &str,
        _instructions: &str,
    ) -> std::result::Result<f64, omon_gateway::judge::JudgeError> {
        unreachable!()
    }

    async fn diff(
        &self,
        _purpose: omon_gateway::judge::Purpose,
        _prompt: &str,
        _repo: &Path,
        _revision: Option<&str>,
    ) -> std::result::Result<omon_gateway::judge::DiffVerdict, omon_gateway::judge::JudgeError>
    {
        unreachable!()
    }
}

#[async_trait]
impl omon_gateway::lanes::posts::PostSink for LocalPostSink {
    async fn send(&self, _post: &omon_gateway::lanes::posts::Post) -> Result<String> {
        unreachable!("malformed post must fail before transport")
    }
}

#[tokio::test]
async fn service_storage_errors_cancel_and_join_backend_and_checker() {
    for error_class in ["list", "generation", "posts", "failure", "join"] {
        let mut fixture = Fixture::new(true).await;
        fixture.claim().await;
        fixture
            .clock
            .set(fixture.clock.now() + chrono::Duration::minutes(5));
        let mut active = fixture.lane.clone();
        active.key = "held-lane".into();
        active.session_key = lane_key(&active.key).storage_key();
        active.status = "working".into();
        insert_lane(fixture.db.pool(), &active).await.unwrap();
        let (check_entered, mut check_entry) = mpsc::unbounded_channel();
        let (check_completed, mut check_completion) = mpsc::unbounded_channel();
        let checks = Arc::new(ShutdownCheck {
            entered: check_entered,
            completed: check_completed,
            release: Semaphore::new(0),
        });
        let (entered, mut entry) = mpsc::unbounded_channel();
        let (cancelled, mut cancellation) = mpsc::unbounded_channel();
        let (completed, mut completion) = mpsc::unbounded_channel();
        let held = Arc::new(ShutdownBackend {
            entered,
            cancelled,
            completed,
            release: Semaphore::new(0),
        });
        let backend = Arc::new(ServiceErrorBackend {
            held: held.clone(),
            release: Semaphore::new(0),
            pool: fixture.db.pool().clone(),
            post_error: matches!(error_class, "posts" | "join"),
            join_error: error_class == "join",
        });
        let runner = LaneRunner::new(fixture.db.pool().clone(), backend.clone())
            .with_settlement(fixture.clock.clone(), checks.clone())
            .with_post_sink(Arc::new(LocalPostSink))
            .with_judge(Arc::new(PanicJudge));
        let stop = CancellationToken::new();
        let task = tokio::spawn(runner.serve(stop.clone()));
        tokio::time::timeout(Duration::from_secs(5), entry.recv())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), check_entry.recv())
            .await
            .unwrap()
            .unwrap();
        let expected = match error_class {
            "list" => {
                sqlx::query(
                    "UPDATE lanes SET turns_run = 'invalid-counter' WHERE key = 'done-test'",
                )
                .execute(fixture.db.pool())
                .await
                .unwrap();
                // Only the working-list decode fails; the held lane remains readable at join.
                sqlx::query("UPDATE lanes SET status = 'working' WHERE key = 'done-test'")
                    .execute(fixture.db.pool())
                    .await
                    .unwrap();
                "turns_run"
            }
            "posts" => {
                sqlx::query("INSERT INTO lane_transcript(lane_key, at, role, text) VALUES ('held-lane', CURRENT_TIMESTAMP, 'thread_post', '{\"state\":\"pending\"}')").execute(fixture.db.pool()).await.unwrap();
                "lane post JSON"
            }
            "generation" => {
                let mut tx = fixture.db.pool().begin().await.unwrap();
                sqlx::query("ALTER TABLE lane_history RENAME TO saved_history")
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                sqlx::query("CREATE VIEW lane_history AS SELECT id, lane_key, at, from_status, to_status, reason FROM saved_history UNION ALL SELECT X'FF', 'error-lane', CURRENT_TIMESTAMP, 'closed', 'working', 'reopened'")
                    .execute(&mut *tx).await.unwrap();
                tx.commit().await.unwrap();
                let mut error_lane = active.clone();
                error_lane.key = "error-lane".into();
                error_lane.session_key = lane_key(&error_lane.key).storage_key();
                insert_lane(fixture.db.pool(), &error_lane).await.unwrap();
                "decoding column 0"
            }
            "join" => {
                let mut error_lane = active.clone();
                error_lane.key = "error-lane".into();
                error_lane.session_key = lane_key(&error_lane.key).storage_key();
                insert_lane(fixture.db.pool(), &error_lane).await.unwrap();
                backend.release.add_permits(1);
                "injected lane task join failure"
            }
            "failure" => {
                sqlx::query("CREATE TRIGGER reject_failure BEFORE UPDATE OF status ON lanes WHEN NEW.status = 'failed' BEGIN SELECT RAISE(ABORT, 'injected failure persistence'); END").execute(fixture.db.pool()).await.unwrap();
                let mut error_lane = active.clone();
                error_lane.key = "error-lane".into();
                error_lane.session_key = lane_key(&error_lane.key).storage_key();
                insert_lane(fixture.db.pool(), &error_lane).await.unwrap();
                backend.release.add_permits(1);
                "injected failure persistence"
            }
            _ => unreachable!(),
        };
        omon_gateway::lanes::runner::wake(fixture.db.pool())
            .await
            .unwrap();
        if error_class == "posts" {
            // A completed lane turn produces the real posts_ready notification.
            let mut error_lane = active.clone();
            error_lane.key = "error-lane".into();
            error_lane.session_key = lane_key(&error_lane.key).storage_key();
            insert_lane(fixture.db.pool(), &error_lane).await.unwrap();
            backend.release.add_permits(1);
            omon_gateway::lanes::runner::wake(fixture.db.pool())
                .await
                .unwrap();
        }
        tokio::time::timeout(Duration::from_secs(5), cancellation.recv())
            .await
            .unwrap_or_else(|error| panic!("{error_class} cancellation: {error}"))
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), check_completion.recv())
            .await
            .unwrap()
            .unwrap();
        if error_class == "generation" {
            sqlx::query("DROP VIEW lane_history")
                .execute(fixture.db.pool())
                .await
                .unwrap();
            sqlx::query("ALTER TABLE saved_history RENAME TO lane_history")
                .execute(fixture.db.pool())
                .await
                .unwrap();
        }
        assert!(stop.is_cancelled());
        assert!(!task.is_finished());
        held.release.add_permits(1);
        tokio::time::timeout(Duration::from_secs(5), completion.recv())
            .await
            .unwrap()
            .unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "{error_class}: {error}"
        );
        if error_class == "generation" {
            match &error {
                omon_gateway::OmonError::Database(detail) => {
                    assert!(
                        detail.contains("i64") && detail.contains("BLOB"),
                        "{detail}"
                    );
                }
                other => panic!("generation fault replaced by {other}"),
            }
        }
        assert_eq!(
            get_lane(fixture.db.pool(), &active.key)
                .await
                .unwrap()
                .unwrap()
                .turns_run,
            1
        );
    }
}

#[async_trait]
impl AgentBackend for ShutdownBackend {
    async fn run(&self, _session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
        Err(omon_gateway::OmonError::Config(
            "run_cancelable required".into(),
        ))
    }

    async fn run_cancelable(
        &self,
        _session: &mut SessionContext,
        _event: InboundEvent,
        cancellation: CancellationToken,
    ) -> Result<()> {
        self.entered.send(()).unwrap();
        cancellation.cancelled().await;
        self.cancelled.send(()).unwrap();
        self.release.acquire().await.unwrap().forget();
        self.completed.send(()).unwrap();
        Err(omon_gateway::OmonError::Multiplexer(
            "test cancellation completed".into(),
        ))
    }
}

#[tokio::test]
async fn settlement_error_and_normal_stop_join_active_backend_and_checker() {
    for settlement_error in [true, false] {
        let mut fixture = Fixture::new(true).await;
        fixture.claim().await;
        fixture
            .clock
            .set(fixture.clock.now() + chrono::Duration::minutes(5));
        let mut active = fixture.lane.clone();
        active.key = "active-shutdown".into();
        active.session_key = lane_key(&active.key).storage_key();
        active.status = "working".into();
        insert_lane(fixture.db.pool(), &active).await.unwrap();
        if settlement_error {
            sqlx::query("CREATE TRIGGER reject_settlement BEFORE UPDATE OF status ON lanes WHEN NEW.status = 'verified_done' BEGIN SELECT RAISE(ABORT, 'injected settlement failure'); END")
                .execute(fixture.db.pool()).await.unwrap();
        }
        // Subscribe to all lifecycle boundaries before starting serve.
        let (check_entered, mut check_entry) = mpsc::unbounded_channel();
        let (check_completed, mut check_completion) = mpsc::unbounded_channel();
        let checks = Arc::new(ShutdownCheck {
            entered: check_entered,
            completed: check_completed,
            release: Semaphore::new(0),
        });
        let (backend_entered, mut backend_entry) = mpsc::unbounded_channel();
        let (backend_cancelled, mut backend_cancellation) = mpsc::unbounded_channel();
        let (backend_completed, mut backend_completion) = mpsc::unbounded_channel();
        let backend = Arc::new(ShutdownBackend {
            entered: backend_entered,
            cancelled: backend_cancelled,
            completed: backend_completed,
            release: Semaphore::new(0),
        });
        let runner = LaneRunner::new(fixture.db.pool().clone(), backend.clone())
            .with_settlement(fixture.clock.clone(), checks.clone());
        let stop = CancellationToken::new();
        let task = tokio::spawn(runner.serve(stop.clone()));
        tokio::time::timeout(Duration::from_secs(5), backend_entry.recv())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), check_entry.recv())
            .await
            .unwrap()
            .unwrap();
        if settlement_error {
            checks.release.add_permits(1);
        } else {
            stop.cancel();
        }
        tokio::time::timeout(Duration::from_secs(5), backend_cancellation.recv())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), check_completion.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            !task.is_finished(),
            "serve must wait for backend cancellation completion"
        );
        backend.release.add_permits(1);
        tokio::time::timeout(Duration::from_secs(5), backend_completion.recv())
            .await
            .unwrap()
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert!(stop.is_cancelled());
        if settlement_error {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("injected settlement failure"));
        } else {
            result.unwrap();
        }
        let row = get_lane(fixture.db.pool(), &active.key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.turns_run, 1,
            "lane task persistence completed before serve returned"
        );
    }
}

#[tokio::test]
async fn paused_claim_is_settled_and_pause_survives() {
    let mut fixture = Fixture::new(true).await;
    // The claim lands first (a paused lane starts no turn), then the owner pauses.
    fixture.claim().await;
    pause_lane(fixture.db.pool(), &fixture.lane.key, "lead")
        .await
        .unwrap();
    assert!(is_lane_paused(fixture.db.pool(), &fixture.lane.key)
        .await
        .unwrap());
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    let settled = fixture.runner().settle_due().await.unwrap();
    assert_eq!(settled, vec![fixture.lane.key.clone()]);
    let row = fixture.status().await;
    assert_eq!(row.status, "verified_done");
    assert!(is_lane_paused(fixture.db.pool(), &fixture.lane.key)
        .await
        .unwrap());
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn paused_claim_with_failing_check_is_not_done_and_stays_paused() {
    let mut fixture = Fixture::new(false).await;
    // The claim lands first (a paused lane starts no turn), then the owner pauses.
    fixture.claim().await;
    pause_lane(fixture.db.pool(), &fixture.lane.key, "lead")
        .await
        .unwrap();
    assert!(is_lane_paused(fixture.db.pool(), &fixture.lane.key)
        .await
        .unwrap());
    fixture
        .clock
        .set(fixture.clock.now() + chrono::Duration::minutes(5));
    let settled = fixture.runner().settle_due().await.unwrap();
    assert!(settled.is_empty());
    let row = fixture.status().await;
    assert_eq!(row.status, "not_done");
    assert!(row.objection.unwrap().contains("missing proof artifact"));
    let body: String = sqlx::query_scalar("SELECT body FROM lane_events WHERE kind = 'not_done'")
        .fetch_one(fixture.db.pool())
        .await
        .unwrap();
    assert!(body.contains("missing proof artifact"));
    assert!(is_lane_paused(fixture.db.pool(), &fixture.lane.key)
        .await
        .unwrap());
    assert_eq!(fixture.checks.calls.load(Ordering::SeqCst), 1);
}

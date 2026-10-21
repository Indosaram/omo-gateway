use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::activity::LaneActivity;
use omon_gateway::lanes::cli::change_claim;
use omon_gateway::lanes::runner::{lane_key, LaneRunner, NUDGE_AFTER, NUDGE_TEXT};
use omon_gateway::storage::lanes::{
    ack_pause, is_lane_paused, pause_acked, pause_lane, resume_lane,
};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::MockClock;
use omon_gateway::{InboundEvent, Result, SessionContext};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Notify};
use tokio_util::sync::CancellationToken;

const BOUND: Duration = Duration::from_secs(10);

/// Turn 1 runs until cancelled; every later turn claims done so the lane loop ends.
struct Fake {
    pool: SqlitePool,
    calls: AtomicUsize,
    started: mpsc::UnboundedSender<usize>,
    steered: mpsc::UnboundedSender<String>,
    tool: Option<bool>,
}

#[async_trait]
impl AgentBackend for Fake {
    async fn run(&self, session: &mut SessionContext, _: InboundEvent) -> Result<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.started.send(call).unwrap();
        if call == 1 {
            std::future::pending::<()>().await;
        }
        change_claim(&self.pool, &session.key.channel_id, true, "file evidence").await
    }

    async fn steer(&self, _: &SessionContext, guidance: &str) -> Result<bool> {
        self.steered.send(guidance.into()).unwrap();
        Ok(true)
    }

    async fn tool_in_flight(&self, _: &SessionContext) -> Option<bool> {
        self.tool
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    database: Database,
    backend: Arc<Fake>,
    started: mpsc::UnboundedReceiver<usize>,
    steered: mpsc::UnboundedReceiver<String>,
}

async fn fixture(tool: Option<bool>) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "Make done.txt").await.unwrap();
    let mut lane = Lane::new(
        "ctl",
        "Control",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        brief.to_str().unwrap(),
        lane_key("ctl").storage_key(),
        "file done.txt",
    );
    lane.status = "working".into();
    insert_lane(database.pool(), &lane).await.unwrap();
    let (started_tx, started) = mpsc::unbounded_channel();
    let (steered_tx, steered) = mpsc::unbounded_channel();
    let backend = Arc::new(Fake {
        pool: database.pool().clone(),
        calls: AtomicUsize::new(0),
        started: started_tx,
        steered: steered_tx,
        tool,
    });
    Fixture {
        _temp: temp,
        database,
        backend,
        started,
        steered,
    }
}

fn session() -> SessionContext {
    SessionContext::new(lane_key("ctl"))
}

async fn nudges(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM lane_events WHERE lane_key = 'ctl' AND kind = 'nudged'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn last_activity(pool: &SqlitePool) -> chrono::DateTime<Utc> {
    get_lane(pool, "ctl")
        .await
        .unwrap()
        .unwrap()
        .last_activity_at
}

#[tokio::test]
async fn a_pause_cancels_the_running_turn_and_starts_no_next_turn() {
    let mut fx = fixture(Some(false)).await;
    let pool = fx.database.pool().clone();
    let runner = LaneRunner::new(pool.clone(), fx.backend.clone());
    let notify = Arc::new(Notify::new());
    let task = tokio::spawn({
        let notify = notify.clone();
        async move {
            runner
                .run_lane("ctl", notify, CancellationToken::new())
                .await
        }
    });
    assert_eq!(
        tokio::time::timeout(BOUND, fx.started.recv())
            .await
            .unwrap(),
        Some(1)
    );
    assert!(pause_lane(&pool, "ctl", "test").await.unwrap());
    notify.notify_one();
    tokio::time::timeout(BOUND, task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(fx.backend.calls.load(Ordering::SeqCst), 1);
    assert!(fx.started.try_recv().is_err());
    let lane = get_lane(&pool, "ctl").await.unwrap().unwrap();
    assert_eq!(lane.status, "working");
    assert_eq!(lane.last_turn_outcome.as_deref(), Some("paused"));
    let reasons: Vec<String> = sqlx::query_scalar(
        "SELECT reason FROM lane_history WHERE lane_key = 'ctl' AND reason = 'paused'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(reasons, vec!["paused".to_owned()]);

    LaneRunner::new(pool.clone(), fx.backend.clone())
        .run_lane("ctl", Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(fx.backend.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn b_resume_allows_the_next_turn() {
    let mut fx = fixture(Some(false)).await;
    let pool = fx.database.pool().clone();
    fx.backend.calls.store(1, Ordering::SeqCst);
    assert!(pause_lane(&pool, "ctl", "test").await.unwrap());
    LaneRunner::new(pool.clone(), fx.backend.clone())
        .run_lane("ctl", Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    assert!(fx.started.try_recv().is_err());

    assert!(resume_lane(&pool, "ctl").await.unwrap());
    assert!(!is_lane_paused(&pool, "ctl").await.unwrap());
    tokio::time::timeout(
        BOUND,
        LaneRunner::new(pool.clone(), fx.backend.clone()).run_lane(
            "ctl",
            Arc::new(Notify::new()),
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(fx.started.try_recv().ok(), Some(2));
    assert_eq!(
        get_lane(&pool, "ctl").await.unwrap().unwrap().status,
        "done_claimed"
    );
}

#[tokio::test]
async fn c_paused_lane_gets_no_nudge_and_no_stale_post() {
    let mut fx = fixture(Some(false)).await;
    let pool = fx.database.pool().clone();
    assert!(pause_lane(&pool, "ctl", "test").await.unwrap());
    let quiet_at = last_activity(&pool).await + chrono::Duration::minutes(30);

    let activity = LaneActivity::new(pool.clone(), Arc::new(MockClock::new(quiet_at)));
    let decision = activity
        .quiet_generation("ctl", true, Some(0))
        .await
        .unwrap();
    assert!(decision.thread_posts.is_empty());
    let stale: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lane_events WHERE lane_key = 'ctl' AND kind = 'stale'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stale, 0);
    assert!(get_lane(&pool, "ctl")
        .await
        .unwrap()
        .unwrap()
        .stale_posted_at
        .is_none());

    let runner = LaneRunner::new(pool.clone(), fx.backend.clone());
    assert!(!runner
        .nudge_quiet_at("ctl", &session(), 0, quiet_at)
        .await
        .unwrap());
    assert_eq!(nudges(&pool).await, 0);
    assert!(fx.steered.try_recv().is_err());
}

#[tokio::test]
async fn d_nudge_is_skipped_while_a_tool_is_in_flight() {
    for tool in [Some(true), None] {
        let mut fx = fixture(tool).await;
        let pool = fx.database.pool().clone();
        let quiet_at = last_activity(&pool).await + NUDGE_AFTER + chrono::Duration::minutes(1);
        let runner = LaneRunner::new(pool.clone(), fx.backend.clone());
        assert!(!runner
            .nudge_quiet_at("ctl", &session(), 0, quiet_at)
            .await
            .unwrap());
        assert_eq!(nudges(&pool).await, 0);
        assert!(fx.steered.try_recv().is_err());
    }
}

#[tokio::test]
async fn e_one_nudge_per_silence() {
    let mut fx = fixture(Some(false)).await;
    let pool = fx.database.pool().clone();
    let start = last_activity(&pool).await;
    let runner = LaneRunner::new(pool.clone(), fx.backend.clone());

    assert!(!runner
        .nudge_quiet_at("ctl", &session(), 0, start + chrono::Duration::minutes(5))
        .await
        .unwrap());

    let first = start + NUDGE_AFTER + chrono::Duration::minutes(1);
    assert!(runner
        .nudge_quiet_at("ctl", &session(), 0, first)
        .await
        .unwrap());
    assert_eq!(fx.steered.try_recv().ok().as_deref(), Some(NUDGE_TEXT));
    assert!(!runner
        .nudge_quiet_at("ctl", &session(), 0, first + chrono::Duration::minutes(30))
        .await
        .unwrap());
    assert!(fx.steered.try_recv().is_err());
    assert_eq!(nudges(&pool).await, 1);

    let spoke = first + chrono::Duration::minutes(5);
    LaneActivity::new(pool.clone(), Arc::new(MockClock::new(spoke)))
        .captured_generation("ctl", "working on it", Some(0))
        .await
        .unwrap();
    assert!(runner
        .nudge_quiet_at(
            "ctl",
            &session(),
            0,
            spoke + NUDGE_AFTER + chrono::Duration::minutes(1)
        )
        .await
        .unwrap());
    assert_eq!(nudges(&pool).await, 2);
}

async fn cli(url: &str, dir: &std::path::Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    run_cli(url, dir, args, Some("1")).await
}

/// `confirm_secs: None` keeps the CLI's own 30 s pause confirmation deadline.
async fn run_cli(
    url: &str,
    dir: &std::path::Path,
    args: &[&str],
    confirm_secs: Option<&str>,
) -> (Option<i32>, serde_json::Value) {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"));
    command
        .current_dir(dir)
        .env("DATABASE_URL", url)
        .env_remove("OMON_LANE_PAUSE_CONFIRM_SECS")
        .args(args);
    if let Some(secs) = confirm_secs {
        command.env("OMON_LANE_PAUSE_CONFIRM_SECS", secs);
    }
    let output = command.output().await.unwrap();
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

async fn file_database(temp: &tempfile::TempDir) -> (String, Database) {
    let url = format!("sqlite://{}", temp.path().join("ctl.db").display());
    let database = Database::connect(&url).await.unwrap();
    (url, database)
}

async fn insert_ctl(pool: &SqlitePool, temp: &tempfile::TempDir, status: &str) {
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "Make done.txt").await.unwrap();
    let mut lane = Lane::new(
        "ctl",
        "Control",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        brief.to_str().unwrap(),
        lane_key("ctl").storage_key(),
        "file done.txt",
    );
    lane.status = status.into();
    if status == "done_claimed" {
        // A fresh claim stays inside the 5-minute window, so settlement leaves it alone.
        lane.claim_at = Some(Utc::now());
    }
    insert_lane(pool, &lane).await.unwrap();
}

async fn working_while_paused_events(pool: &SqlitePool, key: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM lane_events WHERE lane_key = ? AND kind = 'working_while_paused'",
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn f_resume_on_an_unpaused_lane_exits_2_with_not_paused() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("ctl.db").display());
    let database = Database::connect(&url).await.unwrap();
    let mut lane = Lane::new(
        "ctl",
        "Control",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        "unused",
        lane_key("ctl").storage_key(),
        "file done.txt",
    );
    lane.status = "working".into();
    insert_lane(database.pool(), &lane).await.unwrap();

    let (code, body) = cli(&url, temp.path(), &["lane", "resume", "ctl"]).await;
    assert_eq!(code, Some(2));
    assert_eq!(body["error"], "NOT_PAUSED");

    let (code, _) = cli(&url, temp.path(), &["lane", "resume", "missing"]).await;
    assert_ne!(code, Some(0));

    // No lane service runs here, so nothing confirms the pause; the row stays open.
    let (code, body) = cli(&url, temp.path(), &["lane", "pause", "ctl"]).await;
    assert_eq!(code, Some(3));
    assert_eq!(body["error"], "PAUSE_UNVERIFIED");
    assert_eq!(body["paused"], true);
    let (code, body) = cli(&url, temp.path(), &["lane", "list", "--json"]).await;
    assert_eq!(code, Some(0));
    assert_eq!(body["lanes"][0]["key"], "ctl");
    assert_eq!(body["lanes"][0]["paused"], true);

    let (code, body) = cli(&url, temp.path(), &["lane", "resume", "ctl"]).await;
    assert_eq!(code, Some(0));
    assert_eq!(body["paused"], false);
    let (_, body) = cli(&url, temp.path(), &["lane", "list"]).await;
    assert_eq!(body["lanes"][0]["paused"], false);
}

#[tokio::test]
async fn h_pause_without_runner_exits_3_unverified() {
    let temp = tempfile::tempdir().unwrap();
    let (url, database) = file_database(&temp).await;
    insert_ctl(database.pool(), &temp, "working").await;

    let (code, body) = cli(&url, temp.path(), &["lane", "pause", "ctl"]).await;
    assert_eq!(code, Some(3));
    assert_eq!(body["ok"], false);
    assert_eq!(body["key"], "ctl");
    assert_eq!(body["error"], "PAUSE_UNVERIFIED");
    assert_eq!(body["paused"], true);
    assert!(is_lane_paused(database.pool(), "ctl").await.unwrap());
    assert!(!pause_acked(database.pool(), "ctl").await.unwrap());
}

#[tokio::test]
async fn i_pause_with_running_service_confirms() {
    let temp = tempfile::tempdir().unwrap();
    let (url, database) = file_database(&temp).await;
    let pool = database.pool().clone();
    insert_ctl(&pool, &temp, "working").await;
    let (started_tx, mut started) = mpsc::unbounded_channel();
    let (steered_tx, _steered) = mpsc::unbounded_channel();
    let backend = Arc::new(Fake {
        pool: pool.clone(),
        calls: AtomicUsize::new(0),
        started: started_tx,
        steered: steered_tx,
        tool: Some(false),
    });
    let stop = CancellationToken::new();
    let service = tokio::spawn(LaneRunner::new(pool.clone(), backend.clone()).serve(stop.clone()));
    // Turn 1 blocks until cancelled, so the pause must stop a running turn.
    assert_eq!(
        tokio::time::timeout(BOUND, started.recv()).await.unwrap(),
        Some(1)
    );

    let (code, body) = run_cli(&url, temp.path(), &["lane", "pause", "ctl"], None).await;
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(body["confirmed"], true);
    assert_eq!(body["paused"], true);
    assert_eq!(body["changed"], true);
    assert!(pause_acked(&pool, "ctl").await.unwrap());
    assert_eq!(
        get_lane(&pool, "ctl")
            .await
            .unwrap()
            .unwrap()
            .last_turn_outcome
            .as_deref(),
        Some("paused")
    );

    let (code, body) = run_cli(&url, temp.path(), &["lane", "pause", "ctl"], None).await;
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(body["confirmed"], true);
    assert_eq!(body["changed"], false);
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);

    stop.cancel();
    tokio::time::timeout(BOUND, service)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn j_output_after_confirmed_pause_emits_one_event() {
    let fx = fixture(Some(false)).await;
    let pool = fx.database.pool().clone();
    let activity = LaneActivity::new(pool.clone(), Arc::new(MockClock::new(Utc::now())));

    assert!(pause_lane(&pool, "ctl", "test").await.unwrap());
    // Output before the runner confirmed the pause is the turn being stopped, not a leak.
    activity.captured("ctl", "still finishing").await.unwrap();
    assert_eq!(working_while_paused_events(&pool, "ctl").await, 0);

    assert!(ack_pause(&pool, "ctl").await.unwrap());
    activity.captured("ctl", "one").await.unwrap();
    activity.captured("ctl", "two").await.unwrap();
    assert_eq!(working_while_paused_events(&pool, "ctl").await, 1);
    let (lead, body): (String, String) = sqlx::query_as(
        "SELECT lead_session_key, body FROM lane_events WHERE lane_key = 'ctl' AND kind = 'working_while_paused'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lead, lane_key("lead").storage_key());
    assert!(body.contains("ctl"), "{body}");
    assert!(is_lane_paused(&pool, "ctl").await.unwrap());
    assert_eq!(
        get_lane(&pool, "ctl").await.unwrap().unwrap().status,
        "working"
    );

    // External path: pause_lane refuses external lanes, so the acked row is written directly.
    sqlx::query("INSERT INTO lanes(key,title,lead_session_key,cwd,brief_path,session_key,omo_thread_id,status,done_signals,last_activity_at,kind,external_target) VALUES('ext','t','lead:c','/tmp','','','s','working','done',?,'external','omarchy:ext')")
        .bind(Utc::now())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO lane_pauses(lane_key, paused_at, paused_by, resumed_at, acked_at) VALUES ('ext', ?, 'test', NULL, ?)")
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await
        .unwrap();
    activity
        .captured_item("ext", "external output", 0, "item-1")
        .await
        .unwrap();
    activity
        .captured_item("ext", "more output", 0, "item-2")
        .await
        .unwrap();
    assert_eq!(working_while_paused_events(&pool, "ext").await, 1);
}

#[tokio::test]
async fn k_pause_of_done_claimed_lane_confirms_via_service_pass() {
    let temp = tempfile::tempdir().unwrap();
    let (url, database) = file_database(&temp).await;
    let pool = database.pool().clone();
    insert_ctl(&pool, &temp, "done_claimed").await;
    let (started_tx, _started) = mpsc::unbounded_channel();
    let (steered_tx, _steered) = mpsc::unbounded_channel();
    let backend = Arc::new(Fake {
        pool: pool.clone(),
        calls: AtomicUsize::new(0),
        started: started_tx,
        steered: steered_tx,
        tool: Some(false),
    });
    let stop = CancellationToken::new();
    // A done_claimed lane never gets a lane task: only the service pass can confirm it.
    let service = tokio::spawn(LaneRunner::new(pool.clone(), backend.clone()).serve(stop.clone()));

    let (code, body) = run_cli(&url, temp.path(), &["lane", "pause", "ctl"], None).await;
    assert_eq!(code, Some(0), "{body}");
    assert_eq!(body["confirmed"], true);
    assert_eq!(body["changed"], true);
    assert!(pause_acked(&pool, "ctl").await.unwrap());
    assert_eq!(
        get_lane(&pool, "ctl").await.unwrap().unwrap().status,
        "done_claimed"
    );
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);

    stop.cancel();
    tokio::time::timeout(BOUND, service)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn l_pause_of_external_lane_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let (url, database) = file_database(&temp).await;
    let pool = database.pool().clone();
    sqlx::query("INSERT INTO lanes(key,title,lead_session_key,cwd,brief_path,session_key,omo_thread_id,status,done_signals,last_activity_at,kind,external_target) VALUES('ext','t','lead:c','/tmp','','','s','working','done',?,'external','omarchy:ext')")
        .bind(Utc::now())
        .execute(&pool)
        .await
        .unwrap();

    let (code, body) = cli(&url, temp.path(), &["lane", "pause", "ext"]).await;
    assert_eq!(code, Some(2));
    assert_eq!(body["ok"], false);
    assert_eq!(body["key"], "ext");
    assert_eq!(body["error"], "NOT_PAUSABLE");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_pauses WHERE lane_key = 'ext'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    assert!(pause_lane(&pool, "ext", "test").await.is_err());
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::activity::LaneActivity;
use omon_gateway::lanes::cli::change_claim;
use omon_gateway::lanes::runner::{lane_key, LaneRunner, NUDGE_AFTER, NUDGE_TEXT};
use omon_gateway::storage::lanes::{is_lane_paused, pause_lane, resume_lane};
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
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(dir)
        .env("DATABASE_URL", url)
        .args(args)
        .output()
        .await
        .unwrap();
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
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

    let (code, body) = cli(&url, temp.path(), &["lane", "pause", "ctl"]).await;
    assert_eq!(code, Some(0));
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

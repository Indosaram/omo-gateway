use async_trait::async_trait;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::{
    cli::change_claim,
    runner::{lane_key, reopen, LaneRunner},
};
use omon_gateway::storage::{get_lane, get_lane_history, insert_lane, Database, Lane};
use omon_gateway::{InboundEvent, OmonError, Result, SessionContext};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify, Semaphore};
use tokio_util::sync::CancellationToken;

struct Backend {
    pool: sqlx::SqlitePool,
    seen: Mutex<Vec<(Option<String>, String)>>,
    missing: bool,
}

#[async_trait]
impl AgentBackend for Backend {
    async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
        let id = session
            .state
            .metadata
            .get("omo_thread_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        self.seen.lock().unwrap().push((id.clone(), event.content));
        if self.missing && id.is_some() {
            return Err(OmonError::Llm(
                "continuity_error: remote thread rollout missing".into(),
            ));
        }
        session.state.metadata.insert(
            "omo_thread_id".into(),
            serde_json::json!(id.unwrap_or("fresh".into())),
        );
        change_claim(&self.pool, "qa-reopen", true, "test evidence").await
    }
}

async fn setup(status: &str) -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let brief = dir.path().join("brief.md");
    tokio::fs::write(&brief, "original brief").await.unwrap();
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let mut lane = Lane::new(
        "qa-reopen",
        "QA",
        lane_key("lead").storage_key(),
        dir.path().to_str().unwrap(),
        brief.to_str().unwrap(),
        lane_key("qa-reopen").storage_key(),
        "file done.txt",
    );
    lane.status = status.into();
    lane.turns_run = 24;
    lane.omo_thread_id = Some("previous".into());
    insert_lane(db.pool(), &lane).await.unwrap();
    (dir, db)
}

#[tokio::test]
async fn closed_reopen_resumes_id_with_exact_owner_message() {
    let (_dir, db) = setup("closed").await;
    reopen(db.pool(), "qa-reopen", Some("add a line to done.txt"))
        .await
        .unwrap();
    let backend = Arc::new(Backend {
        pool: db.pool().clone(),
        seen: Mutex::new(Vec::new()),
        missing: false,
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        LaneRunner::new(db.pool().clone(), backend.clone()).run_lane(
            "qa-reopen",
            Arc::new(Notify::new()),
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        backend.seen.lock().unwrap()[0],
        (Some("previous".into()), "add a line to done.txt".into())
    );
    assert_eq!(
        get_lane(db.pool(), "qa-reopen")
            .await
            .unwrap()
            .unwrap()
            .turns_run,
        1
    );
}

#[tokio::test]
async fn missing_thread_falls_back_once_and_records_history() {
    let (_dir, db) = setup("failed").await;
    reopen(db.pool(), "qa-reopen", Some("continue"))
        .await
        .unwrap();
    let backend = Arc::new(Backend {
        pool: db.pool().clone(),
        seen: Mutex::new(Vec::new()),
        missing: true,
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        LaneRunner::new(db.pool().clone(), backend.clone()).run_lane(
            "qa-reopen",
            Arc::new(Notify::new()),
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    {
        let seen = backend.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].0, None);
        assert!(seen[1].1.contains("original brief"));
        assert!(seen[1]
            .1
            .contains("Resumed lane; previous thread unavailable"));
    }
    assert_eq!(
        get_lane(db.pool(), "qa-reopen")
            .await
            .unwrap()
            .unwrap()
            .omo_thread_id
            .as_deref(),
        Some("fresh")
    );
    let history = get_lane_history(db.pool(), "qa-reopen").await.unwrap();
    assert!(history
        .iter()
        .any(|h| h.reason.as_deref() == Some("Resumed lane; previous thread unavailable")));
}

#[tokio::test]
async fn working_and_verified_done_reopen_are_illegal() {
    for status in ["working", "verified_done", "done_claimed"] {
        let (_dir, db) = setup(status).await;
        assert!(reopen(db.pool(), "qa-reopen", None)
            .await
            .unwrap_err()
            .to_string()
            .contains("IllegalTransition"));
        assert!(get_lane_history(db.pool(), "qa-reopen")
            .await
            .unwrap()
            .is_empty());
    }
}

struct ClosingBackend {
    pool: sqlx::SqlitePool,
    started: mpsc::UnboundedSender<()>,
    cancelled: mpsc::UnboundedSender<()>,
    release: Arc<Semaphore>,
    seen: Mutex<Vec<String>>,
}

#[async_trait]
impl AgentBackend for ClosingBackend {
    async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
        self.run_cancelable(session, event, CancellationToken::new())
            .await
    }

    async fn run_cancelable(
        &self,
        _session: &mut SessionContext,
        event: InboundEvent,
        cancellation: CancellationToken,
    ) -> Result<()> {
        self.seen.lock().unwrap().push(event.content);
        self.started.send(()).unwrap();
        if self.seen.lock().unwrap().len() > 1 {
            return change_claim(&self.pool, "qa-reopen", true, "reopened result").await;
        }
        cancellation.cancelled().await;
        self.cancelled.send(()).unwrap();
        self.release.acquire().await.unwrap().forget();
        Err(OmonError::Llm("pre-close turn cancelled".into()))
    }

    async fn steer(&self, _session: &SessionContext, _guidance: &str) -> Result<bool> {
        panic!("the reopening message must not be steered into the old turn")
    }
}

#[tokio::test]
async fn close_reopen_discards_old_turn_and_delivers_new_owner_text_first() {
    let (_dir, db) = setup("working").await;
    sqlx::query("UPDATE lanes SET turns_run = 2, last_turn_outcome = NULL WHERE key = 'qa-reopen'")
        .execute(db.pool())
        .await
        .unwrap();
    let (started, mut started_rx) = mpsc::unbounded_channel();
    let (cancelled, mut cancelled_rx) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let backend = Arc::new(ClosingBackend {
        pool: db.pool().clone(),
        started,
        cancelled,
        release: release.clone(),
        seen: Mutex::new(Vec::new()),
    });
    let notify = Arc::new(Notify::new());
    let runner = LaneRunner::new(db.pool().clone(), backend.clone());
    let run_notify = notify.clone();
    let task = tokio::spawn(async move {
        runner
            .run_lane("qa-reopen", run_notify, CancellationToken::new())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), started_rx.recv())
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE lanes SET status = 'closed' WHERE key = 'qa-reopen'")
        .execute(db.pool())
        .await
        .unwrap();
    reopen(db.pool(), "qa-reopen", Some("add a line to done.txt"))
        .await
        .unwrap();
    let reopened = get_lane(db.pool(), "qa-reopen").await.unwrap().unwrap();
    assert_eq!(reopened.status, "working");
    assert_eq!(reopened.turns_run, 0);
    assert_eq!(backend.seen.lock().unwrap().len(), 1);
    notify.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(5), cancelled_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(backend.seen.lock().unwrap().len(), 1);
    release.add_permits(1);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let row = get_lane(db.pool(), "qa-reopen").await.unwrap().unwrap();
    assert_eq!(row.status, "done_claimed");
    assert_eq!(row.turns_run, 1);
    {
        let seen = backend.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1], "add a line to done.txt");
    }
    let (consumed,): (i64,) = sqlx::query_as("SELECT count(*) FROM lane_inbox WHERE lane_key = 'qa-reopen' AND text = 'add a line to done.txt' AND consumed_at IS NOT NULL")
        .fetch_one(db.pool()).await.unwrap();
    assert_eq!(consumed, 0);
    let (pending,): (i64,) = sqlx::query_as("SELECT count(*) FROM lane_inbox WHERE lane_key = 'qa-reopen' AND text = 'add a line to done.txt' AND consumed_at IS NULL")
        .fetch_one(db.pool()).await.unwrap();
    assert_eq!(pending, 1);
}

struct CompletedBackend {
    pool: sqlx::SqlitePool,
    notify: Arc<Notify>,
}

#[async_trait]
impl AgentBackend for CompletedBackend {
    async fn run(&self, _session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
        omon_gateway::lanes::cli::enqueue(&self.pool, "qa-reopen", "next turn only").await?;
        change_claim(&self.pool, "qa-reopen", true, "completed result").await?;
        self.notify.notify_one();
        Ok(())
    }

    async fn steer(&self, _session: &SessionContext, _guidance: &str) -> Result<bool> {
        panic!("a completed turn must not receive steering")
    }
}

#[tokio::test(flavor = "current_thread")]
async fn completed_turn_with_ready_notification_keeps_inbox_pending() {
    let (_dir, db) = setup("working").await;
    sqlx::query("UPDATE lanes SET turns_run = 0 WHERE key = 'qa-reopen'")
        .execute(db.pool())
        .await
        .unwrap();
    let notify = Arc::new(Notify::new());
    let backend = Arc::new(CompletedBackend {
        pool: db.pool().clone(),
        notify: notify.clone(),
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        LaneRunner::new(db.pool().clone(), backend).run_lane(
            "qa-reopen",
            notify,
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let row = get_lane(db.pool(), "qa-reopen").await.unwrap().unwrap();
    assert_eq!(row.status, "done_claimed");
    assert_eq!(row.turns_run, 1);
    let (pending, consumed): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(consumed_at) FROM lane_inbox WHERE lane_key = 'qa-reopen' AND text = 'next turn only'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(pending, 1);
    assert_eq!(consumed, 0);
}

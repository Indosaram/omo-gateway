use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::cli::{change_claim, enqueue};
use omon_gateway::lanes::runner::{lane_key, CapturingDispatcher, LaneRunner};
use omon_gateway::multiplexer::OutboundDispatcher;
use omon_gateway::storage::{get_lane, insert_lane, read_transcript, Database, Lane};
use omon_gateway::{InboundEvent, OmonError, OutboundAction, Result, SessionContext};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, Notify, Semaphore};
use tokio_util::sync::CancellationToken;

struct Script {
    pool: SqlitePool,
    calls: AtomicUsize,
    claim_at: usize,
    errors: bool,
    observed: mpsc::UnboundedSender<(SessionContext, String)>,
    release: Option<Arc<Semaphore>>,
    steered: mpsc::UnboundedSender<String>,
}

#[async_trait]
impl AgentBackend for Script {
    async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
        let count = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.observed
            .send((session.clone(), event.content))
            .unwrap();
        session
            .state
            .metadata
            .insert("omo_thread_id".into(), serde_json::json!("stored-thread"));
        if let Some(release) = &self.release {
            release.acquire().await.unwrap().forget();
        }
        if self.errors {
            return Err(OmonError::Llm("scripted turn failure".into()));
        }
        CapturingDispatcher::new(self.pool.clone())
            .dispatch(OutboundAction::SendMessage {
                session: session.key.clone(),
                content: format!("[REPORT] {} | milestone | {count}", session.key.channel_id),
                reply_to: None,
            })
            .await?;
        if count == self.claim_at {
            change_claim(&self.pool, &session.key.channel_id, true, "file evidence").await?;
        }
        Ok(())
    }

    async fn steer(&self, _session: &SessionContext, guidance: &str) -> Result<bool> {
        self.steered.send(guidance.into()).unwrap();
        Ok(true)
    }
}

async fn fixture() -> (tempfile::TempDir, Database, Lane) {
    let temp = tempfile::tempdir().unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let brief = temp.path().join("brief.md");
    tokio::fs::write(
        &brief,
        "Make done.txt; $(touch SHOULD_NOT_EXIST) is literal data.",
    )
    .await
    .unwrap();
    let mut lane = Lane::new(
        "test-lane",
        "Test",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        brief.to_str().unwrap(),
        lane_key("test-lane").storage_key(),
        "file done.txt",
    );
    lane.status = "working".into();
    lane.model = Some("inferhub/cb/deepseek-v4.1-flash".into());
    insert_lane(database.pool(), &lane).await.unwrap();
    (temp, database, lane)
}

fn script(
    pool: &SqlitePool,
    claim_at: usize,
    errors: bool,
    release: Option<Arc<Semaphore>>,
) -> (
    Arc<Script>,
    mpsc::UnboundedReceiver<(SessionContext, String)>,
    mpsc::UnboundedReceiver<String>,
) {
    let (observed, receive) = mpsc::unbounded_channel();
    let (steered, steer_receive) = mpsc::unbounded_channel();
    (
        Arc::new(Script {
            pool: pool.clone(),
            calls: AtomicUsize::new(0),
            claim_at,
            errors,
            observed,
            release,
            steered,
        }),
        receive,
        steer_receive,
    )
}

#[tokio::test]
async fn chains_until_claim_and_captures_without_discord() {
    let (temp, database, lane) = fixture().await;
    let (backend, mut observed, _steered) = script(database.pool(), 3, false, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    let row = get_lane(database.pool(), &lane.key).await.unwrap().unwrap();
    assert_eq!(row.status, "done_claimed");
    assert_eq!(row.turns_run, 3);
    assert_eq!(row.omo_thread_id.as_deref(), Some("stored-thread"));
    let (context, prompt) = observed.recv().await.unwrap();
    assert_eq!(context.state.active_model, lane.model);
    assert_eq!(context.state.metadata["lane_cwd"], lane.cwd);
    assert!(prompt.contains("$(touch SHOULD_NOT_EXIST)"));
    assert!(prompt.contains("omo-gateway lane claim-done test-lane"));
    assert!(!temp.path().join("SHOULD_NOT_EXIST").exists());
    let transcript = read_transcript(database.pool(), &lane.key, None)
        .await
        .unwrap();
    assert_eq!(
        transcript
            .iter()
            .filter(|entry| entry.role == "agent")
            .count(),
        3
    );
}

#[tokio::test]
async fn twenty_four_turn_cap_fails_with_event() {
    let (_temp, database, lane) = fixture().await;
    let (backend, _observed, _steered) = script(database.pool(), usize::MAX, false, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    let row = get_lane(database.pool(), &lane.key).await.unwrap().unwrap();
    assert_eq!(row.turns_run, 24);
    assert_eq!(row.status, "failed");
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM lane_events WHERE kind = 'turn_cap'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn three_errors_fail_with_dead_event() {
    let (_temp, database, lane) = fixture().await;
    let (backend, _observed, _steered) = script(database.pool(), usize::MAX, true, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    let row = get_lane(database.pool(), &lane.key).await.unwrap().unwrap();
    assert_eq!(row.turns_run, 3);
    assert_eq!(row.status, "failed");
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM lane_events WHERE kind = 'dead'")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn active_send_steers_and_consumes_only_after_ack() {
    let (_temp, database, lane) = fixture().await;
    let release = Arc::new(Semaphore::new(0));
    let (backend, mut observed, mut steered) =
        script(database.pool(), 1, false, Some(release.clone()));
    let runner = LaneRunner::new(database.pool().clone(), backend);
    let notify = Arc::new(Notify::new());
    let runner_notify = notify.clone();
    let task = tokio::spawn(async move {
        runner
            .run_lane("test-lane", runner_notify, CancellationToken::new())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.recv())
        .await
        .unwrap()
        .unwrap();
    enqueue(database.pool(), &lane.key, "use B").await.unwrap();
    notify.notify_one();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), steered.recv())
            .await
            .unwrap()
            .unwrap(),
        "use B"
    );
    release.add_permits(1);
    task.await.unwrap().unwrap();
    let (count,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM lane_inbox WHERE text = 'use B' AND consumed_at IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn restart_uses_persistent_thread_and_idle_message() {
    let (_temp, database, lane) = fixture().await;
    sqlx::query("UPDATE lanes SET turns_run = 1, omo_thread_id = 'previous-thread' WHERE key = ?")
        .bind(&lane.key)
        .execute(database.pool())
        .await
        .unwrap();
    enqueue(database.pool(), &lane.key, "next prompt")
        .await
        .unwrap();
    let (backend, mut observed, _steered) = script(database.pool(), 1, false, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    let (session, prompt) = observed.recv().await.unwrap();
    assert_eq!(session.state.metadata["omo_thread_id"], "previous-thread");
    assert_eq!(prompt, "next prompt");
}

#[tokio::test]
async fn repeated_cancellation_keeps_lane_resumable() {
    let (_temp, database, lane) = fixture().await;
    for _ in 0..3 {
        let (backend, mut observed, _steered) = script(
            database.pool(),
            usize::MAX,
            false,
            Some(Arc::new(Semaphore::new(0))),
        );
        let runner = LaneRunner::new(database.pool().clone(), backend);
        let cancellation = CancellationToken::new();
        let token = cancellation.clone();
        let task = tokio::spawn(async move {
            runner
                .run_lane("test-lane", Arc::new(Notify::new()), token)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), observed.recv())
            .await
            .unwrap()
            .unwrap();
        cancellation.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    let row = get_lane(database.pool(), &lane.key).await.unwrap().unwrap();
    assert_eq!(row.status, "working");
    assert_eq!(row.omo_thread_id.as_deref(), Some("stored-thread"));
}

#[tokio::test]
async fn misleading_success_output_never_verifies_or_closes_lane() {
    let (_temp, database, lane) = fixture().await;
    CapturingDispatcher::new(database.pool().clone())
        .dispatch(OutboundAction::SendMessage {
            session: lane_key(&lane.key),
            content: "[REPORT] test-lane | done | all checks passed; verified_done".into(),
            reply_to: None,
        })
        .await
        .unwrap();
    assert_eq!(
        get_lane(database.pool(), &lane.key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
    change_claim(database.pool(), &lane.key, true, "all checks passed")
        .await
        .unwrap();
    assert_eq!(
        get_lane(database.pool(), &lane.key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
    assert!(!std::path::Path::new(&lane.cwd).join("done.txt").exists());
}

#[tokio::test]
async fn stale_lane_binding_prefers_durable_session_binding() {
    let (_temp, database, lane) = fixture().await;
    let context = SessionContext::new(lane_key(&lane.key));
    sqlx::query("INSERT INTO sessions(session_key, platform, channel_id, user_id, state_json, created_at, updated_at) VALUES (?, 'lane', ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(context.key.storage_key()).bind(&context.key.channel_id).bind(&context.key.user_id)
        .bind(serde_json::json!({"metadata": {"omo_thread_id": "durable-thread"}}).to_string()).execute(database.pool()).await.unwrap();
    sqlx::query("UPDATE lanes SET omo_thread_id = 'stale-thread' WHERE key = ?")
        .bind(&lane.key)
        .execute(database.pool())
        .await
        .unwrap();
    let (backend, mut observed, _steered) = script(database.pool(), 1, false, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        observed.recv().await.unwrap().0.state.metadata["omo_thread_id"],
        "durable-thread"
    );
}

#[tokio::test]
async fn dirty_worktree_is_preserved_by_claim() {
    let (temp, database, lane) = fixture().await;
    assert!(tokio::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(temp.path())
        .status()
        .await
        .unwrap()
        .success());
    tokio::fs::write(temp.path().join("tracked.txt"), "before")
        .await
        .unwrap();
    assert!(tokio::process::Command::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(temp.path())
        .status()
        .await
        .unwrap()
        .success());
    tokio::fs::write(temp.path().join("tracked.txt"), "after")
        .await
        .unwrap();
    change_claim(database.pool(), &lane.key, true, "dirty claim")
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(temp.path().join("tracked.txt"))
            .await
            .unwrap(),
        "after"
    );
    assert_eq!(
        get_lane(database.pool(), &lane.key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
    let output = tokio::process::Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(temp.path())
        .output()
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "tracked.txt"
    );
}

#[tokio::test]
async fn two_failed_checks_escalate_small_lane_only_once() {
    let (_temp, database, lane) = fixture().await;
    for _ in 0..2 {
        change_claim(database.pool(), &lane.key, true, "claimed")
            .await
            .unwrap();
        change_claim(database.pool(), &lane.key, false, "signal failed")
            .await
            .unwrap();
    }
    let (backend, mut observed, _steered) = script(database.pool(), 2, false, None);
    LaneRunner::new(database.pool().clone(), backend)
        .run_lane(&lane.key, Arc::new(Notify::new()), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        observed
            .recv()
            .await
            .unwrap()
            .0
            .state
            .active_model
            .as_deref(),
        Some("mahoquot/claude-opus-5-5")
    );
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM lane_history WHERE reason = 'escalated'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn service_recovers_file_database_and_shutdown_releases_socket() {
    let (temp, _old, mut lane) = fixture().await;
    let database = Database::connect(&format!(
        "sqlite://{}",
        temp.path().join("service.db").display()
    ))
    .await
    .unwrap();
    lane.turns_run = 1;
    lane.omo_thread_id = Some("recovered-thread".into());
    insert_lane(database.pool(), &lane).await.unwrap();
    let (backend, mut observed, _steered) = script(
        database.pool(),
        usize::MAX,
        false,
        Some(Arc::new(Semaphore::new(0))),
    );
    let runner = LaneRunner::new(database.pool().clone(), backend.clone());
    let stop = CancellationToken::new();
    let task = tokio::spawn(runner.serve(stop.clone()));
    let (session, _) = tokio::time::timeout(std::time::Duration::from_secs(5), observed.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.state.metadata["omo_thread_id"], "recovered-thread");
    let second = LaneRunner::new(database.pool().clone(), backend)
        .serve(CancellationToken::new())
        .await;
    assert!(second.unwrap_err().to_string().contains("already owns"));
    stop.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let (backend, mut observed, _steered) = script(
        database.pool(),
        usize::MAX,
        false,
        Some(Arc::new(Semaphore::new(0))),
    );
    let stop = CancellationToken::new();
    let task = tokio::spawn(LaneRunner::new(database.pool().clone(), backend).serve(stop.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.recv())
        .await
        .unwrap()
        .unwrap();
    stop.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn startup_discord_flag_controls_token_requirement() {
    const CHILD: &str = "LANE_STARTUP_TOKEN_CASE";
    const COMPLETE: &str = "startup-token-case-complete";
    if let Ok(case) = std::env::var(CHILD) {
        assert!(std::env::var_os("DISCORD_BOT_TOKEN").is_none());
        assert!(std::env::var_os("DISCORD_BOT_TOKENS").is_none());
        let db_path = std::env::current_dir().unwrap().join("startup.db");
        let database = Database::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .unwrap();
        let result = LaneRunner::from_env(database.pool().clone());
        if case == "disabled" {
            use sha2::{Digest, Sha256};
            use tracing_subscriber::prelude::*;

            struct Ready(std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>);
            impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Ready {
                fn on_event(
                    &self,
                    event: &tracing::Event<'_>,
                    _ctx: tracing_subscriber::layer::Context<'_, S>,
                ) {
                    if event.metadata().target() == "omon_gateway::lanes::runner" {
                        let mut socket = false;
                        event.record(
                            &mut |field: &tracing::field::Field, _value: &dyn std::fmt::Debug| {
                                socket |= field.name() == "socket";
                            },
                        );
                        if socket {
                            if let Some(sender) = self.0.lock().unwrap().take() {
                                sender.send(()).unwrap();
                            }
                        }
                    }
                }
            }
            let (ready, received) = tokio::sync::oneshot::channel();
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry().with(Ready(std::sync::Mutex::new(Some(ready)))),
            )
            .unwrap();
            let canonical = tokio::fs::canonicalize(&db_path).await.unwrap();
            let digest = format!(
                "{:x}",
                Sha256::digest(canonical.as_os_str().as_encoded_bytes())
            );
            let socket =
                std::path::PathBuf::from("/tmp").join(format!("omon-lanes-{}.sock", &digest[..24]));
            let runner = result.expect("Discord-disabled lane startup needs no token");
            let stop = CancellationToken::new();
            let mut task = tokio::spawn(runner.serve(stop.clone()));
            let readiness = tokio::time::timeout(std::time::Duration::from_secs(5), received).await;
            let socket_ready = socket.exists();
            stop.cancel();
            let joined = tokio::time::timeout(std::time::Duration::from_secs(5), &mut task).await;
            let outcome = match joined {
                Ok(result) => result.unwrap(),
                Err(error) => {
                    task.abort();
                    let _ = task.await;
                    panic!("startup service shutdown exceeded deadline: {error}");
                }
            };
            outcome.unwrap();
            assert!(readiness.unwrap().is_ok());
            assert!(socket_ready);
            assert!(!socket.exists());
            tokio::fs::remove_file(socket.with_extension("lock"))
                .await
                .unwrap();
            let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM lanes")
                .fetch_one(database.pool())
                .await
                .unwrap();
            assert_eq!(count, 0);
        } else {
            assert!(matches!(
                result,
                Err(OmonError::Config(message)) if message.contains("DISCORD_BOT_TOKEN")
            ));
        }
        database.close().await;
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let path = std::path::PathBuf::from(format!("{}{suffix}", db_path.display()));
            if path.exists() {
                tokio::fs::remove_file(&path).await.unwrap();
            }
            assert!(!path.exists());
        }
        tokio::fs::write(COMPLETE, case).await.unwrap();
        return;
    }
    for (case, flag) in [
        ("disabled", Some("false")),
        ("enabled", Some("true")),
        ("default", None),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "startup_discord_flag_controls_token_requirement",
                "--exact",
                "--nocapture",
            ])
            .env_clear()
            .env(CHILD, case)
            .env("HOME", temp.path())
            .env("TMPDIR", temp.path())
            .env("OMON_OMO_APPSERVER_URL", "ws://127.0.0.1:1")
            .current_dir(temp.path())
            .kill_on_drop(true);
        if let Some(flag) = flag {
            command.env("OMON_DISCORD_ENABLED", flag);
        }
        let mut child = command.spawn().unwrap();
        let waited = tokio::time::timeout(std::time::Duration::from_secs(10), child.wait()).await;
        let status = match waited {
            Ok(result) => result.unwrap(),
            Err(error) => {
                child.kill().await.unwrap();
                child.wait().await.unwrap();
                panic!("startup child {case} exceeded deadline: {error}");
            }
        };
        assert!(status.success(), "startup child {case}: {status}");
        assert_eq!(
            tokio::fs::read_to_string(temp.path().join(COMPLETE))
                .await
                .unwrap(),
            case
        );
    }
}

#[tokio::test]
#[ignore = "requires a disposable app-server and explicit QA database"]
async fn live_qa_runner_process() {
    let url = std::env::var("TODO8_QA_DATABASE_URL").expect("explicit scratch database");
    let database = Database::connect(&url).await.unwrap();
    let runner = LaneRunner::from_env(database.pool().clone()).unwrap();
    let stop = CancellationToken::new();
    let mut signal =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    let token = stop.clone();
    let task = tokio::spawn(runner.serve(token));
    println!("TODO8_QA_RUNNER_PID={}", std::process::id());
    signal.recv().await;
    stop.cancel();
    task.await.unwrap().unwrap();
    database.close().await;
    println!("TODO8_QA_RUNNER_CLEAN_EXIT");
}

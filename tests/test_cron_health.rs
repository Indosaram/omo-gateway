use std::sync::Arc;

use async_trait::async_trait;
use omon_gateway::{CronJob, CronJobSpec, CronScheduler, CronTaskExecutor, OmonError, Result};
use omon_gateway::{OutboundAction, OutboundDispatcher};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::Notify;

struct FakeExecutor {
    result: Result<Option<String>>,
    finished: Arc<Notify>,
    release: Option<Arc<Notify>>,
}

#[async_trait]
impl CronTaskExecutor for FakeExecutor {
    async fn execute(&self, _job: &CronJob) -> Result<Option<String>> {
        self.finished.notify_one();
        if let Some(release) = &self.release {
            release.notified().await;
        }
        match &self.result {
            Ok(output) => Ok(output.clone()),
            Err(error) => Err(OmonError::ToolExecution(error.to_string())),
        }
    }
}

struct FailingDispatcher;

#[async_trait]
impl OutboundDispatcher for FailingDispatcher {
    async fn dispatch(&self, _action: OutboundAction) -> Result<()> {
        Err(OmonError::Multiplexer("dispatch fixture failed".into()))
    }
}

async fn setup(
    result: Result<Option<String>>,
    payload: serde_json::Value,
) -> (SqlitePool, CronScheduler, String, Arc<Notify>) {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let finished = Arc::new(Notify::new());
    let executor = Arc::new(FakeExecutor {
        result,
        finished: finished.clone(),
        release: None,
    });
    let scheduler = CronScheduler::new(pool.clone(), executor);
    let job = scheduler
        .register(CronJobSpec::new("interval:1h", payload))
        .await
        .unwrap();
    (pool, scheduler, job.id, finished)
}

async fn setup_with_dispatcher(
    result: Result<Option<String>>,
    payload: serde_json::Value,
) -> (SqlitePool, CronScheduler, String, Arc<Notify>) {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let finished = Arc::new(Notify::new());
    let scheduler = CronScheduler::with_dispatcher(
        pool.clone(),
        Arc::new(FakeExecutor {
            result,
            finished: finished.clone(),
            release: None,
        }),
        Arc::new(FailingDispatcher),
    );
    let job = scheduler
        .register(CronJobSpec::new("interval:1h", payload))
        .await
        .unwrap();
    (pool, scheduler, job.id, finished)
}

async fn setup_direct_delivery_failure(
    payload: serde_json::Value,
) -> (SqlitePool, CronScheduler, String, Arc<Notify>) {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let finished = Arc::new(Notify::new());
    let scheduler = CronScheduler::with_dispatcher(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("report".into())),
            finished: finished.clone(),
            release: None,
        }),
        Arc::new(FailingDispatcher),
    );
    let job = scheduler
        .register(CronJobSpec::new("interval:1h", payload))
        .await
        .unwrap();
    (pool, scheduler, job.id, finished)
}

async fn execute_and_wait(scheduler: &CronScheduler, job_id: &str, finished: &Notify) {
    let completion = finished.notified();
    tokio::pin!(completion);
    scheduler.trigger(job_id).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), completion)
        .await
        .expect("fake executor completion signal");
    scheduler.shutdown().await;
}

#[tokio::test]
async fn failed_run_opens_problem_in_same_scheduler_execution() {
    let (pool, scheduler, job, finished) = setup(
        Err(OmonError::ToolExecution("fixture failure".into())),
        json!({}),
    )
    .await;
    execute_and_wait(&scheduler, &job, &finished).await;
    let (kind, status): (String, String) = sqlx::query_as("SELECT kind, status FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((kind.as_str(), status.as_str()), ("failed", "open"));
}

#[tokio::test]
async fn expected_output_problem_is_opened_and_fixed_by_clean_success() {
    let (pool, scheduler, job, finished) = setup(
        Ok(Some("wrong output".into())),
        json!({"expect":{"output_regex":"wanted"}}),
    )
    .await;
    execute_and_wait(&scheduler, &job, &finished).await;
    let kind: String = sqlx::query_scalar("SELECT kind FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kind, "empty_success");

    let next_finished = Arc::new(Notify::new());
    let executor = Arc::new(FakeExecutor {
        result: Ok(Some("wanted output".into())),
        finished: next_finished.clone(),
        release: None,
    });
    let scheduler = CronScheduler::new(pool.clone(), executor);
    let completion = next_finished.notified();
    tokio::pin!(completion);
    scheduler.trigger(&job).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), completion)
        .await
        .unwrap();
    scheduler.shutdown().await;
    let status: String = sqlx::query_scalar("SELECT status FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "fixed");
}

#[test]
fn hook_has_no_timer_loop() {
    let source = include_str!("../src/cron/health.rs");
    assert!(source.contains("OMON_CRON_WATCH_ENABLED"));
    assert!(!source.contains("tokio::time::interval"));
    assert!(!source.contains("tokio::time::sleep"));
}

#[tokio::test]
async fn run_without_expect_creates_no_empty_success_problem() {
    let (pool, scheduler, job, finished) = setup(Ok(Some("anything".into())), json!({})).await;
    execute_and_wait(&scheduler, &job, &finished).await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = ? AND kind = 'empty_success'",
    )
    .bind(&job)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn delivery_failure_uses_committed_terminal_outcome() {
    let payload = json!({"content":"send this", "channel_id":"12345", "attach_to_session":false});
    let (pool, scheduler, job, finished) = setup_direct_delivery_failure(payload).await;
    execute_and_wait(&scheduler, &job, &finished).await;
    let (status, kind): (String, String) = sqlx::query_as(
        "SELECT r.status, p.kind FROM cron_runs r JOIN cron_problems p ON p.run_id = r.run_id WHERE r.job_id = ?",
    )
    .bind(&job)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(kind, "delivery_failed");
}

#[tokio::test]
async fn health_preparation_error_does_not_change_committed_run_status() {
    let (pool, scheduler, job, finished) = setup(
        Err(OmonError::ToolExecution("run failed".into())),
        json!({}),
    )
    .await;
    sqlx::query("DROP TABLE cron_problems")
        .execute(&pool)
        .await
        .unwrap();
    execute_and_wait(&scheduler, &job, &finished).await;
    let status: String = sqlx::query_scalar("SELECT status FROM cron_runs WHERE job_id = ?")
        .bind(&job)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn health_problem_insert_error_does_not_change_committed_run_status() {
    let (pool, scheduler, job, finished) = setup(
        Err(OmonError::ToolExecution("run failed".into())),
        json!({}),
    )
    .await;
    sqlx::query("DROP TABLE lane_events")
        .execute(&pool)
        .await
        .unwrap();
    execute_and_wait(&scheduler, &job, &finished).await;
    let status: String = sqlx::query_scalar("SELECT status FROM cron_runs WHERE job_id = ?")
        .bind(&job)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn disabled_health_switch_writes_no_problems_or_events() {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "disabled_health_switch_subprocess_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("OMON_CRON_WATCH_ENABLED", "false")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("kill-switch subprocess must finish promptly")
    .expect("kill-switch subprocess must launch");
    assert!(
        output.status.success(),
        "kill-switch subprocess failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "invoked by disabled_health_switch_writes_no_problems_or_events in an isolated environment"]
async fn disabled_health_switch_subprocess_helper() {
    let (pool, scheduler, job, finished) = setup(
        Err(OmonError::ToolExecution("disabled-switch failure".into())),
        json!({}),
    )
    .await;
    execute_and_wait(&scheduler, &job, &finished).await;

    let run_status: String = sqlx::query_scalar("SELECT status FROM cron_runs WHERE job_id = ?")
        .bind(&job)
        .fetch_one(&pool)
        .await
        .unwrap();
    let problems: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'cron_problem'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(run_status, "failed");
    assert_eq!(problems, 0);
    assert_eq!(events, 0);
}

#[tokio::test]
async fn stale_lease_reclaim_creates_stuck_problem() {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let now = chrono::Utc::now();
    let job = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("unused".into())),
            finished: Arc::new(Notify::new()),
            release: None,
        }),
    )
    .register(CronJobSpec::new("interval:1h", json!({})))
    .await
    .unwrap();
    sqlx::query("INSERT INTO cron_runs(run_id, job_id, claim_token, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid) VALUES ('stale-run', ?, 'stale-token', ?, ?, NULL, 'running', 1, NULL, NULL)")
        .bind(&job.id).bind(now - chrono::TimeDelta::minutes(5)).bind(now - chrono::TimeDelta::minutes(10))
        .execute(&pool).await.unwrap();
    let scheduler = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("unused".into())),
            finished: Arc::new(Notify::new()),
            release: None,
        }),
    )
    .with_clock(move || now);
    let _ = scheduler.trigger(&job.id).await.unwrap();
    let kind: String =
        sqlx::query_scalar("SELECT kind FROM cron_problems WHERE run_id = 'stale-run'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(kind, "stuck");
    scheduler.shutdown().await;
}

#[tokio::test]
async fn completed_delivery_error_is_emitted_once_after_failure_commit() {
    let lead =
        omon_gateway::SessionKey::new("discord", None::<String>, "12345", None::<String>, "owner")
            .storage_key();
    for configured_lead in [Some(lead.as_str()), None, Some("cron:invalid")] {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "delivery_lead_subprocess_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("OMON_CRON_WATCH_ENABLED", "true")
            .kill_on_drop(true);
        match configured_lead {
            Some(value) => {
                command.env("OMON_LANE_DEFAULT_LEAD", value);
            }
            None => {
                command.env_remove("OMON_LANE_DEFAULT_LEAD");
            }
        }
        let output = tokio::time::timeout(std::time::Duration::from_secs(30), command.output())
            .await
            .expect("lead fixture must finish")
            .expect("lead fixture must launch");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
#[ignore = "invoked with isolated lead configuration by the parent test"]
async fn delivery_lead_subprocess_helper() {
    // Given: an actual delivery failure and process-local lead configuration.
    let payload = json!({"content":"send", "channel_id":"12345"});
    let (pool, scheduler, job, finished) =
        setup_with_dispatcher(Ok(Some("output".into())), payload).await;
    // When: the scheduler commits its terminal outcome and health event.
    execute_and_wait(&scheduler, &job, &finished).await;
    let problems: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = ? AND kind = 'delivery_failed'",
    )
    .bind(&job)
    .fetch_one(&pool)
    .await
    .unwrap();
    let events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'cron_problem'")
            .fetch_one(&pool)
            .await
            .unwrap();
    // Then: problems persist regardless of route validity; an unset lead routes to the
    // built-in owner DM, and only an invalid one produces no event.
    assert_eq!(problems, 1);
    let configured = std::env::var("OMON_LANE_DEFAULT_LEAD")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| omon_gateway::lanes::kibitzer::DEFAULT_LEAD.to_owned());
    let expected = omon_gateway::SessionKey::from_storage_key(&configured).ok();
    assert_eq!(events, i64::from(expected.is_some()));
    if let Some(expected) = expected {
        let lead: String = sqlx::query_scalar(
            "SELECT lead_session_key FROM lane_events WHERE kind = 'cron_problem'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            omon_gateway::SessionKey::from_storage_key(&lead).unwrap(),
            expected
        );
        sqlx::query("DROP TABLE lane_events")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM cron_problems")
            .execute(&pool)
            .await
            .unwrap();
        let run: String = sqlx::query_scalar("SELECT run_id FROM cron_runs WHERE job_id = ?")
            .bind(&job)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            omon_gateway::cron::health::on_run_finished(&pool, &job, &run, true)
                .await
                .is_err()
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_problems")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn startup_scan_records_unrun_recurring_occurrence_once() {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let now = chrono::Utc::now();
    let scheduled = now - chrono::TimeDelta::hours(2);
    let payload = json!({"expect":{"output_regex":"ok"}});
    sqlx::query("INSERT INTO cron_jobs(id, expression, payload_json, enabled, next_run_at, created_at, updated_at) VALUES ('missed-job', 'interval:1h', ?, 1, ?, ?, ?)")
        .bind(payload.to_string()).bind(scheduled).bind(scheduled).bind(scheduled)
        .execute(&pool).await.unwrap();
    let execution_started = Arc::new(Notify::new());
    let release_execution = Arc::new(Notify::new());
    let scheduler = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("ok".into())),
            finished: execution_started.clone(),
            release: Some(release_execution.clone()),
        }),
    )
    .with_clock(move || now);
    scheduler.start().await;
    let scheduler_ready = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = 'missed-job' AND kind = 'missed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scheduler_ready, 1);
    let execution_started = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        execution_started.notified(),
    )
    .await;
    release_execution.notify_one();
    scheduler.shutdown().await;
    assert!(
        execution_started.is_ok(),
        "due execution should start after startup scan"
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = 'missed-job' AND kind = 'missed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    scheduler.start().await;
    scheduler.shutdown().await;
    let count_again: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = 'missed-job' AND kind = 'missed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    assert_eq!(count_again, 1);
}

#[tokio::test]
async fn identical_failure_recurs_with_one_new_event() {
    let lead =
        omon_gateway::SessionKey::new("discord", None::<String>, "12345", None::<String>, "owner")
            .storage_key();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "recurrence_subprocess_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("OMON_CRON_WATCH_ENABLED", "true")
            .env("OMON_LANE_DEFAULT_LEAD", lead)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "invoked with isolated lead configuration by the parent test"]
async fn recurrence_subprocess_helper() {
    // Given: a real failed run, recovered by a real clean scheduler run.
    let (pool, scheduler, job, finished) = setup(
        Err(OmonError::ToolExecution("recurring fixture".into())),
        json!({}),
    )
    .await;
    execute_and_wait(&scheduler, &job, &finished).await;
    let first_run: String = sqlx::query_scalar("SELECT run_id FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE cron_problems SET detail=?,lane_key='old-lane'")
        .bind(json!({"cause":"old", "reruns":3, "owner_approved":true}).to_string())
        .execute(&pool)
        .await
        .unwrap();
    let clean_finished = Arc::new(Notify::new());
    let clean = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("clean".into())),
            finished: clean_finished.clone(),
            release: None,
        }),
    );
    execute_and_wait(&clean, &job, &clean_finished).await;
    let status: String = sqlx::query_scalar("SELECT status FROM cron_problems")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "fixed");
    omon_gateway::cron::health::on_run_finished(&pool, &job, &first_run, false)
        .await
        .unwrap();
    let before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind='cron_problem'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, 1);

    // When: the same failure recurs and its terminal callback is replayed.
    let next_finished = Arc::new(Notify::new());
    let failed = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Err(OmonError::ToolExecution("recurring fixture".into())),
            finished: next_finished.clone(),
            release: None,
        }),
    );
    execute_and_wait(&failed, &job, &next_finished).await;
    let (status, run, detail, lane): (String, String, String, Option<String>) =
        sqlx::query_as("SELECT status,run_id,detail,lane_key FROM cron_problems")
            .fetch_one(&pool)
            .await
            .unwrap();
    omon_gateway::cron::health::on_run_finished(&pool, &job, &run, false)
        .await
        .unwrap();

    // Then: one fresh event and a reopened incident with reset remediation state.
    assert_eq!(status, "open");
    assert_ne!(run, first_run);
    assert!(lane.is_none());
    let reset = omon_gateway::cron::remediate::Detail::parse(&detail);
    assert_eq!(reset.reruns, 0);
    assert!(!reset.owner_approved);
    assert!(reset.cause.contains("recurring fixture"));
    let after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind='cron_problem'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn existing_late_run_in_occurrence_window_is_not_missed() {
    let database = omon_gateway::Database::connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = database.pool().clone();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let now = chrono::Utc::now();
    let scheduled = now - chrono::TimeDelta::hours(2);
    sqlx::query("INSERT INTO cron_jobs(id, expression, payload_json, enabled, next_run_at, created_at, updated_at) VALUES ('catchup-job', 'interval:1h', '{}', 1, ?, ?, ?)")
        .bind(scheduled).bind(scheduled).bind(scheduled).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO cron_runs(run_id, job_id, claim_token, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid) VALUES ('late-run','catchup-job','late-token',?,?,?,'succeeded',1,NULL,NULL)")
        .bind(scheduled + chrono::TimeDelta::minutes(30))
        .bind(scheduled + chrono::TimeDelta::minutes(30))
        .bind(scheduled + chrono::TimeDelta::minutes(30))
        .execute(&pool).await.unwrap();
    let scheduler = CronScheduler::new(
        pool.clone(),
        Arc::new(FakeExecutor {
            result: Ok(Some("done".into())),
            finished: Arc::new(Notify::new()),
            release: None,
        }),
    )
    .with_clock(move || now);
    scheduler.start().await;
    scheduler.shutdown().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cron_problems WHERE job_id = 'catchup-job' AND kind = 'missed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use omon_gateway::{
    resolve_predecessor_output, CronJob, CronJobSpec, CronScheduler, CronTaskExecutor, Database,
    OmonError, Result,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn fixed_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-08-16T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

struct EditingExecutor {
    pool: SqlitePool,
    edited_payload: Option<String>,
    fail: bool,
}

#[async_trait]
impl CronTaskExecutor for EditingExecutor {
    async fn execute(&self, job: &CronJob) -> Result<Option<String>> {
        if let Some(edited) = &self.edited_payload {
            sqlx::query("UPDATE cron_jobs SET payload_json = ? WHERE id = ?")
                .bind(edited)
                .bind(&job.id)
                .execute(&self.pool)
                .await
                .unwrap();
        }
        if self.fail {
            return Err(OmonError::ToolExecution("boom".into()));
        }
        Ok(None)
    }
}

fn scheduler(pool: &SqlitePool, edited_payload: Option<&str>, fail: bool) -> CronScheduler {
    let now = fixed_now();
    CronScheduler::new(
        pool.clone(),
        Arc::new(EditingExecutor {
            pool: pool.clone(),
            edited_payload: edited_payload.map(str::to_owned),
            fail,
        }),
    )
    .with_clock(move || now)
}

async fn set_next_run(pool: &SqlitePool, id: &str, at: DateTime<Utc>) {
    sqlx::query("UPDATE cron_jobs SET next_run_at = ? WHERE id = ?")
        .bind(at)
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn huge_catch_up_hours_does_not_panic_scheduler() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let scheduler = scheduler(pool, None, false);
    let job = scheduler
        .register(CronJobSpec::new(
            "0 * * * *",
            json!({"catch_up_hours": i64::MAX as u64}),
        ))
        .await
        .unwrap();
    set_next_run(pool, &job.id, fixed_now() - TimeDelta::hours(2)).await;

    let claimed = scheduler.run_due_jobs().await.unwrap();
    scheduler.wait_idle().await;
    assert_eq!(claimed, 1, "overdue job must still be claimed");
}

async fn assert_advanced_after_mid_run_edit(fail: bool, expected_status: &str) {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let edited = r#"{"prompt_note":"edited"}"#;
    let scheduler = scheduler(pool, Some(edited), fail);
    let now = fixed_now();
    let job = scheduler
        .register(CronJobSpec::new("interval:1h", json!({})))
        .await
        .unwrap();
    let occurrence = now - TimeDelta::minutes(5);
    set_next_run(pool, &job.id, occurrence).await;

    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 1);
    scheduler.wait_idle().await;

    let after = scheduler.get(&job.id).await.unwrap().unwrap();
    assert!(
        after.next_run_at.unwrap() > now,
        "next_run_at must advance past the executed occurrence, got {:?}",
        after.next_run_at
    );
    let payload: Value = after.payload().unwrap();
    assert_eq!(
        payload["prompt_note"], "edited",
        "mid-run edit must be kept"
    );
    assert_eq!(payload["last_status"], expected_status);
    assert_eq!(payload["last_occurrence"], occurrence.to_rfc3339());

    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 0);
}

#[tokio::test]
async fn success_advances_schedule_after_mid_run_payload_edit() {
    assert_advanced_after_mid_run_edit(false, "succeeded").await;
}

#[tokio::test]
async fn failure_advances_schedule_after_mid_run_payload_edit() {
    assert_advanced_after_mid_run_edit(true, "failed").await;
}

#[tokio::test]
async fn failure_backoff_and_resume_respect_schedule_timezone() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let scheduler = scheduler(pool, None, true);
    // now = 2026-08-16T12:00Z = 21:00 KST; next 09:00 KST = 2026-08-17T00:00Z
    // (the UTC-evaluated answer would be 2026-08-17T09:00Z).
    let expected: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-08-17T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let job = scheduler
        .register_with_id(
            "tz_job",
            CronJobSpec::new(
                "0 9 * * *",
                json!({
                    "id": "tz_job",
                    "name": "tz job",
                    "prompt": "daily task",
                    "catch_up_hours": 24,
                    "schedule": {"kind": "cron", "expr": "0 9 * * *", "timezone": "Asia/Seoul"}
                }),
            ),
        )
        .await
        .unwrap();
    assert_eq!(job.next_run_at, Some(expected));
    set_next_run(pool, &job.id, expected - TimeDelta::days(1)).await;

    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 1);
    scheduler.wait_idle().await;
    let after_failure = scheduler.get(&job.id).await.unwrap().unwrap();
    assert_eq!(after_failure.next_run_at, Some(expected), "failure path");

    assert!(scheduler.pause(&job.id).await.unwrap());
    assert!(scheduler.resume(&job.id).await.unwrap());
    let after_resume = scheduler.get(&job.id).await.unwrap().unwrap();
    assert_eq!(after_resume.next_run_at, Some(expected), "resume path");
}

#[tokio::test]
async fn repeat_limit_reached_by_manual_run_disables_job() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let scheduler = scheduler(pool, None, false);
    let job = scheduler
        .register(CronJobSpec::new(
            "interval:1h",
            json!({"repeat": {"times": 1, "completed": 0}}),
        ))
        .await
        .unwrap();
    let receipt = scheduler.trigger_wait(&job.id).await.unwrap();
    assert_eq!(receipt.status, "succeeded");
    set_next_run(pool, &job.id, fixed_now() - TimeDelta::minutes(1)).await;

    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 0);
    let after = scheduler.get(&job.id).await.unwrap().unwrap();
    assert!(!after.enabled, "exhausted job must be disabled");
    assert!(
        after.next_run_at.is_none(),
        "exhausted job must clear next_run_at"
    );
}

#[tokio::test]
async fn predecessor_lookup_escapes_like_wildcards() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json, created_at, updated_at) \
         VALUES ('discord:1:cron:aXb', 'discord', '1', 'cron:aXb', '{}', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) \
         VALUES ('m1', 'discord:1:cron:aXb', 'assistant', 'other job output', '{}', ?)",
    )
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    assert_eq!(resolve_predecessor_output(pool, "", "a_b").await, None);
    assert_eq!(resolve_predecessor_output(pool, "", "%").await, None);
    assert_eq!(
        resolve_predecessor_output(pool, "", "aXb").await.as_deref(),
        Some("other job output")
    );
}

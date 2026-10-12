use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use clap::Parser;
use omon_gateway::{
    inbox::{
        cli::{execute, InboxArgs},
        digest::{self, DigestExecutor},
        list::{list, ClassFilter},
    },
    CronJob, CronScheduler, CronTaskExecutor, Database, Result, SessionKey,
};

#[derive(Debug, Parser)]
struct Cli {
    #[command(flatten)]
    inbox: InboxArgs,
}

fn args(values: &[&str]) -> InboxArgs {
    Cli::try_parse_from(std::iter::once("inbox").chain(values.iter().copied()))
        .unwrap()
        .inbox
}

async fn db() -> Database {
    Database::connect("sqlite::memory:").await.unwrap()
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap()
}

async fn seed(
    pool: &sqlx::SqlitePool,
    id: &str,
    class: &str,
    status: &str,
    due: Option<&str>,
    received: &str,
) {
    sqlx::query("INSERT INTO inbox_items(id,source,source_ref,author,title,snippet,received_at,link,raw_hash,collected_at) VALUES(?,'mail',?,'sender','summary','PRIVATE_SNIPPET',?,'https://example.test/item','hash',?)")
        .bind(id).bind(id).bind(received).bind(now()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO inbox_decisions(item_id,class,reason,proposed_action,due_at,status,decided_at) VALUES(?,?,'reason','do task',?,?,?)")
        .bind(id).bind(class).bind(due).bind(status).bind(now()).execute(pool).await.unwrap();
}

#[tokio::test]
async fn cli_filters_and_orders_due_then_received_on_seeded_database() {
    // Given: mixed classes, statuses, and deliberately unsorted timestamps.
    let db = db().await;
    for (id, class, status, due, received) in [
        (
            "late",
            "owner",
            "open",
            Some("2026-10-13T00:00:00Z"),
            "2026-10-10T00:00:00Z",
        ),
        (
            "early-new",
            "owner",
            "open",
            Some("2026-10-12T00:00:00Z"),
            "2026-10-10T00:00:00Z",
        ),
        (
            "early-old",
            "owner",
            "open",
            Some("2026-10-12T00:00:00Z"),
            "2026-10-09T00:00:00Z",
        ),
        ("undated", "owner", "open", None, "2026-10-08T00:00:00Z"),
        ("done", "owner", "done", None, "2026-10-08T00:00:00Z"),
        ("bot", "bot", "proposed", None, "2026-10-08T00:00:00Z"),
        ("fyi", "fyi", "open", None, "2026-10-08T00:00:00Z"),
    ] {
        seed(db.pool(), id, class, status, due, received).await;
    }
    // When: the CLI parses the documented owner/open/JSON flags.
    let output = execute(
        db.pool(),
        args(&["list", "--class", "owner", "--status", "open", "--json"]),
    )
    .await;
    // Then: only pending owner work is returned, deadlines precede undated work.
    assert_eq!(output.code, 0);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["early-old", "early-new", "late", "undated"]
    );
    assert!(rows.iter().all(|r| r.get("snippet").is_none()));
    eprintln!("task25 owner/open count={}", rows.len());
}

#[tokio::test]
async fn cli_class_filters_include_bot_proposals_and_fyi() {
    // Given: one task in each public class.
    let db = db().await;
    for (id, class, status) in [
        ("owner", "owner", "done"),
        ("bot", "bot", "proposed"),
        ("fyi", "fyi", "open"),
    ] {
        seed(db.pool(), id, class, status, None, "2026-10-10T00:00:00Z").await;
    }
    // When: public filters are exercised with and without the open flag.
    for class in ["owner", "bot", "fyi"] {
        let output = execute(db.pool(), args(&["list", "--class", class, "--json"])).await;
        let rows: Vec<serde_json::Value> = serde_json::from_str(&output.stdout).unwrap();
        // Then: each filter includes only its class.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["class"], class);
    }
    assert_eq!(
        list(db.pool(), Some(ClassFilter::Bot), true)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn set_records_each_terminal_status_without_executing_actions() {
    // Given: independent open items for each documented status.
    let db = db().await;
    for status in ["done", "dismissed"] {
        seed(
            db.pool(),
            status,
            "bot",
            "open",
            None,
            "2026-10-10T00:00:00Z",
        )
        .await;
        // When: the CLI sets the requested local status.
        let output = execute(db.pool(), args(&["set", status, "--status", status])).await;
        // Then: SQLite reflects exactly that status, not an executed lane.
        assert_eq!(output.code, 0);
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status,lane_key FROM inbox_decisions WHERE item_id=?")
                .bind(status)
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(row, (status.into(), None));
    }
}

#[tokio::test]
async fn unknown_set_returns_exit_two() {
    // Given: a scratch DB with no such item.
    let db = db().await;
    // When: the exact failure QA command reaches the CLI handler.
    let output = execute(db.pool(), args(&["set", "nope", "--status", "done"])).await;
    // Then: wiring propagates this code via process::exit, with no stdout.
    assert_eq!(output.code, 2);
    assert!(output.stdout.is_empty());
}

#[test]
fn invalid_cli_values_are_rejected_at_boundary() {
    // Given/When: unsupported classes and status changes are parsed.
    for values in [
        vec!["inbox", "list", "--class", "ignore"],
        vec!["inbox", "set", "x", "--status", "open"],
        vec!["inbox", "list", "--status", "done"],
    ] {
        // Then: clap returns the standard usage exit code.
        assert_eq!(Cli::try_parse_from(values).unwrap_err().exit_code(), 2);
    }
}

struct FakeExecutor(AtomicUsize);
#[async_trait]
impl CronTaskExecutor for FakeExecutor {
    async fn execute(&self, _: &CronJob) -> Result<Option<String>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Some("other job".into()))
    }
}

fn home() -> SessionKey {
    SessionKey::new("discord", None::<String>, "dm", None::<String>, "owner").with_bot_id("bot")
}

#[tokio::test]
async fn digest_registers_one_gateway_job_at_nine_kst_and_bypasses_models() {
    // Given: a real scratch scheduler and a fake normal executor.
    let db = db().await;
    let fake = Arc::new(FakeExecutor(AtomicUsize::new(0)));
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(sessions, 0);
    let executor = Arc::new(DigestExecutor {
        pool: db.pool().clone(),
        inner: fake.clone(),
    });
    let scheduler = CronScheduler::new(db.pool().clone(), executor.clone());
    seed(
        db.pool(),
        "owner",
        "owner",
        "open",
        None,
        "2026-10-10T00:00:00Z",
    )
    .await;
    seed(
        db.pool(),
        "bot",
        "bot",
        "open",
        None,
        "2026-10-10T00:00:00Z",
    )
    .await;
    // When: startup registration is repeated, and the stored job executes.
    digest::register(db.pool(), &scheduler, &home(), "owner")
        .await
        .unwrap();
    digest::register(db.pool(), &scheduler, &home(), "owner")
        .await
        .unwrap();
    let job = scheduler.get(digest::JOB_ID).await.unwrap().unwrap();
    let message = executor.execute(&job).await.unwrap().unwrap();
    // Then: one job and one combined output, routed through normal owner origin.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM cron_jobs")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    let session: (String, String) =
        sqlx::query_as("SELECT channel_id,user_id FROM sessions WHERE session_key=?")
            .bind(home().storage_key())
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(session, ("dm".into(), "owner".into()));
    assert_eq!(job.expression, "0 9 * * *");
    assert_eq!(job.payload().unwrap()["schedule"]["timezone"], "Asia/Seoul");
    let destinations = omon_gateway::delivery_destination(&job.payload().unwrap()).unwrap();
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].chat_id, "dm");
    assert_eq!(
        job.next_run_at
            .unwrap()
            .with_timezone(&chrono_tz::Asia::Seoul)
            .format("%H:%M")
            .to_string(),
        "09:00"
    );
    assert!(!message.contains("PRIVATE_SNIPPET"));
    assert_eq!(fake.0.load(Ordering::SeqCst), 0);
    let mut other = job.clone();
    other.id = "other".into();
    assert_eq!(
        executor.execute(&other).await.unwrap().as_deref(),
        Some("other job")
    );
    assert_eq!(fake.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn digest_empty_lists_produce_no_output() {
    // Given: no work and no auth problems.
    let db = db().await;
    // When/Then: no output means the scheduler sends no message.
    assert!(digest::build(db.pool(), now()).await.unwrap().is_none());
}

#[tokio::test]
async fn digest_statistics_without_owner_work_produce_no_output() {
    let db = db().await;
    let yesterday = now() - chrono::Duration::hours(24);
    seed(
        db.pool(),
        "information",
        "fyi",
        "open",
        None,
        "2026-10-10T00:00:00Z",
    )
    .await;
    sqlx::query("UPDATE inbox_items SET collected_at=?")
        .bind(yesterday)
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO judge_calls(id,purpose,kind,ms,outcome,created_at) VALUES('judge','inbox_class','choice',1,'fyi',?)")
        .bind(yesterday).execute(db.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO inbox_filter_stats(day,source,rule,count) VALUES(?,'mail','promotion',5)",
    )
    .bind(yesterday.date_naive().to_string())
    .execute(db.pool())
    .await
    .unwrap();
    assert!(digest::build(db.pool(), now()).await.unwrap().is_none());
}

#[tokio::test]
async fn approval_without_prepared_proposal_is_rejected_without_status_change() {
    let db = db().await;
    seed(
        db.pool(),
        "bot",
        "bot",
        "open",
        None,
        "2026-10-10T00:00:00Z",
    )
    .await;
    let output = execute(db.pool(), args(&["set", "bot", "--status", "approved"])).await;
    assert_eq!(output.code, 2);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM inbox_decisions")
            .fetch_one(db.pool())
            .await
            .unwrap(),
        "open"
    );
}

#[tokio::test]
async fn auth_only_digest_is_suppressed_after_success_and_returns_next_kst_day() {
    // Given: an auth failure, a non-auth error, and a succeeded delivered digest.
    let db = db().await;
    sqlx::query("INSERT INTO inbox_cursors(source,cursor,updated_at,error) VALUES('mail','',?,'auth_expired SECRET'),('kakao','',?,'network error')").bind(now()).bind(now()).execute(db.pool()).await.unwrap();
    let scheduler = CronScheduler::new(
        db.pool().clone(),
        Arc::new(FakeExecutor(AtomicUsize::new(0))),
    );
    digest::register(db.pool(), &scheduler, &home(), "owner")
        .await
        .unwrap();
    let first = digest::build(db.pool(), now()).await.unwrap().unwrap();
    assert!(!first.contains("SECRET"));
    sqlx::query("INSERT INTO cron_runs(run_id,job_id,claim_token,lease_expires_at,started_at,completed_at,status,attempt) VALUES('run',?,'claim',?,?,?,'succeeded',1)").bind(digest::JOB_ID).bind(now()).bind(now()).bind(now()).execute(db.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO cron_outputs(profile,job_id,run_id,output,created_at) VALUES('',?,'run',?,?)",
    )
    .bind(digest::JOB_ID)
    .bind(first)
    .bind(now())
    .execute(db.pool())
    .await
    .unwrap();
    // When/Then: read-back of durable success suppresses duplicates only this day.
    assert!(
        digest::build(db.pool(), now() + chrono::Duration::hours(14))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        digest::build(db.pool(), now() + chrono::Duration::hours(15))
            .await
            .unwrap()
            .is_some()
    );
}

#[test]
fn digest_rejects_shared_channels_threads_and_other_users() {
    // Given/When: invalid destinations are supplied to registration.
    for key in [
        SessionKey::new("discord", Some("guild"), "shared", None::<String>, "owner"),
        SessionKey::new("discord", None::<String>, "dm", Some("thread"), "owner"),
        SessionKey::new("discord", None::<String>, "dm", None::<String>, "other"),
    ] {
        // Then: no cron spec is admitted for a non-owner-home-DM target.
        assert!(digest::spec(&key, "owner").is_err());
    }
}

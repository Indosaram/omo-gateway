use std::time::Duration;

use chrono::{TimeZone, Utc};
use omon_gateway::inbox::health::{self, Backoff, HealthAction, WatcherRun};
use omon_gateway::storage::Database;

const LEAD: &str = "slack:D0LEAD";
const LONG_RUN: Duration = Duration::from_secs(90);

async fn source_health_events(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM lane_events WHERE kind = 'source_health'")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn drive(pool: &sqlx::SqlitePool, backoff: &mut Backoff, at: chrono::DateTime<Utc>) -> bool {
    let step = backoff.step(WatcherRun::AuthFailed, LONG_RUN);
    match step.health {
        HealthAction::Dead => health::report_dead(pool, "mail", "auth_expired", LEAD, at)
            .await
            .unwrap(),
        _ => false,
    }
}

#[tokio::test]
async fn three_auth_failures_give_one_event_and_a_fourth_same_day_gives_none() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let morning = Utc.with_ymd_and_hms(2026, 10, 18, 1, 0, 0).unwrap();
    let mut backoff = Backoff::default();

    for _ in 0..3 {
        drive(pool, &mut backoff, morning).await;
    }
    assert_eq!(source_health_events(pool).await, 1);
    let (body, lead): (String, String) = sqlx::query_as(
        "SELECT body, lead_session_key FROM lane_events WHERE kind = 'source_health'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(body.contains("zele login"));
    assert_eq!(lead, LEAD);

    let later_same_kst_day = Utc.with_ymd_and_hms(2026, 10, 18, 14, 0, 0).unwrap();
    assert!(!drive(pool, &mut backoff, later_same_kst_day).await);
    assert_eq!(source_health_events(pool).await, 1);

    let next_kst_day = Utc.with_ymd_and_hms(2026, 10, 18, 15, 30, 0).unwrap();
    assert!(drive(pool, &mut backoff, next_kst_day).await);
    assert_eq!(source_health_events(pool).await, 2);

    health::clear(pool, "mail").await.unwrap();
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM source_health")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    assert_eq!(
        source_health_events(pool).await,
        2,
        "recovery posts nothing"
    );

    let mut relapse = Backoff::default();
    for _ in 0..3 {
        drive(pool, &mut relapse, next_kst_day).await;
    }
    assert_eq!(
        source_health_events(pool).await,
        2,
        "a relapse after recovery on the same KST day stays silent"
    );
}

#[test]
fn auth_failing_long_runs_keep_doubling_to_the_cap() {
    let mut backoff = Backoff::default();
    let waits: Vec<u64> = (0..8)
        .map(|_| {
            backoff
                .step(WatcherRun::AuthFailed, LONG_RUN)
                .wait
                .as_secs()
        })
        .collect();
    assert_eq!(waits, [5, 10, 20, 40, 80, 160, 300, 300]);
}

#[test]
fn healthy_long_run_resets_to_initial() {
    let mut backoff = Backoff::default();
    for _ in 0..4 {
        backoff.step(WatcherRun::AuthFailed, LONG_RUN);
    }
    assert_eq!(backoff.step(WatcherRun::Failed, LONG_RUN).wait.as_secs(), 5);

    let mut backoff = Backoff::default();
    for _ in 0..4 {
        backoff.step(WatcherRun::Failed, Duration::from_secs(1));
    }
    let step = backoff.step(WatcherRun::Completed, LONG_RUN);
    assert_eq!(step.wait.as_secs(), 5);
    assert_eq!(step.health, HealthAction::Healthy);
}

#[test]
fn zele_auth_errors_are_detected() {
    assert!(health::is_auth_failure(
        "Error: invalid_grant: Token has been expired"
    ));
    assert!(health::is_auth_failure(
        "auth_expired: mail credentials expired"
    ));
    assert!(!health::is_auth_failure("ECONNRESET while fetching"));
}

#[test]
fn token_refresh_that_dies_on_dns_is_not_an_auth_failure() {
    // zele stderr from the 2026-10-20 02:10Z DNS outage that raised a false "auth is dead".
    let tail = "Token expired, refreshing...\nWatch failed: API call failed: Error: request to \
                https://oauth2.googleapis.com/token failed, reason: getaddrinfo ENOTFOUND \
                oauth2.googleapis.com";
    assert!(health::is_network_failure(tail));
    assert!(!health::is_auth_failure(tail));
    assert!(health::is_auth_failure(
        "Token expired, refreshing...\ninvalid_grant"
    ));
}

#[test]
fn a_credential_verdict_outranks_a_network_error_in_the_same_tail() {
    // The reverse mistake matters more: a revoked refresh token that also logs a timeout must
    // still reach the owner, or mail silently stops for good.
    assert!(health::is_auth_failure(
        "Token expired, refreshing...\ninvalid_grant: Token has been expired or revoked\n\
         request timed out"
    ));
    assert!(health::is_auth_failure(
        "Error: 401 Unauthorized (ECONNRESET while retrying)"
    ));
    assert!(health::is_auth_failure("Not logged in. Try: zele login"));
    // A plain network failure stays a network failure.
    assert!(!health::is_auth_failure(
        "fetch failed: getaddrinfo EAI_AGAIN oauth2.googleapis.com"
    ));
    assert!(!health::is_network_failure("invalid_grant"));
}

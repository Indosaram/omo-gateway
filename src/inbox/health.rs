//! Watcher restart backoff and dead-source reporting. The backoff decision is pure so
//! the supervisor in main stays a thin loop; the DB side reports an auth-dead source to
//! the owner lead at most once per source per KST day and clears silently on recovery.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::Result;

pub const BACKOFF_INITIAL: Duration = Duration::from_secs(5);
pub const BACKOFF_MAX: Duration = Duration::from_secs(300);
pub const HEALTHY_RUN: Duration = Duration::from_secs(60);
pub const AUTH_FAILURES_BEFORE_NOTICE: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherRun {
    Completed,
    Failed,
    AuthFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthAction {
    Healthy,
    Degraded,
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub wait: Duration,
    pub health: HealthAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    pub delay: Duration,
    pub auth_failures: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            delay: BACKOFF_INITIAL,
            auth_failures: 0,
        }
    }
}

pub fn next_backoff(current: Duration) -> Duration {
    (current * 2).min(BACKOFF_MAX)
}

impl Backoff {
    /// A long run resets the delay only when it did not end in an auth failure; an
    /// expired login keeps doubling up to the cap instead of looping every 5 s.
    pub fn step(&mut self, run: WatcherRun, ran_for: Duration) -> Step {
        match run {
            WatcherRun::Completed => {
                self.auth_failures = 0;
                self.delay = BACKOFF_INITIAL;
                Step {
                    wait: self.delay,
                    health: HealthAction::Healthy,
                }
            }
            WatcherRun::Failed => {
                self.auth_failures = 0;
                if ran_for >= HEALTHY_RUN {
                    self.delay = BACKOFF_INITIAL;
                }
                let wait = self.delay;
                self.delay = next_backoff(self.delay);
                Step {
                    wait,
                    health: HealthAction::Degraded,
                }
            }
            WatcherRun::AuthFailed => {
                self.auth_failures = self.auth_failures.saturating_add(1);
                let wait = self.delay;
                self.delay = next_backoff(self.delay);
                Step {
                    wait,
                    health: if self.auth_failures >= AUTH_FAILURES_BEFORE_NOTICE {
                        HealthAction::Dead
                    } else {
                        HealthAction::Degraded
                    },
                }
            }
        }
    }
}

pub fn is_auth_failure(output_tail: &str) -> bool {
    let lower = output_tail.to_ascii_lowercase();
    [
        "auth_expired",
        "token expired",
        "invalid_grant",
        "unauthorized",
        "unauthenticated",
        "not authenticated",
        "not logged in",
        "login required",
        "zele login",
        "invalid credentials",
        "credentials expired",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub fn notice_text(source: &str) -> String {
    match source {
        "mail" => "mail: zele 로그인 만료 - 터미널에서 `zele login` 실행 필요".to_owned(),
        other => format!("{other}: 인증 실패가 계속됨 - 해당 CLI 로그인 확인 필요"),
    }
}

pub fn kst_day(at: DateTime<Utc>) -> String {
    at.with_timezone(&chrono_tz::Asia::Seoul)
        .date_naive()
        .to_string()
}

/// Records a dead source and queues one `source_health` lane event for `lead`. Returns
/// true only when an event was inserted; repeats on the same KST day insert nothing.
pub async fn report_dead(
    pool: &SqlitePool,
    source: &str,
    detail: &str,
    lead: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    let today = kst_day(now);
    let day_start = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
        .ok()
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .and_then(|midnight| midnight.and_local_timezone(chrono_tz::Asia::Seoul).single())
        .map(|at| at.with_timezone(&Utc))
        .unwrap_or(now);
    let body = notice_text(source);
    let detail: String = detail.chars().take(500).collect();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO source_health(source, state, detail, since, notified_on) \
         VALUES(?, 'auth_failed', ?, ?, NULL) \
         ON CONFLICT(source) DO UPDATE SET state = excluded.state, detail = excluded.detail",
    )
    .bind(source)
    .bind(&detail)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let mut claimed = sqlx::query(
        "UPDATE source_health SET notified_on = ? \
         WHERE source = ? AND (notified_on IS NULL OR notified_on <> ?)",
    )
    .bind(&today)
    .bind(source)
    .bind(&today)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        > 0;
    if claimed {
        let already_today: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM lane_events WHERE kind = 'source_health' AND body = ? \
             AND julianday(created_at) >= julianday(?) AND julianday(created_at) < julianday(?)",
        )
        .bind(&body)
        .bind(day_start)
        .bind(day_start + chrono::TimeDelta::days(1))
        .fetch_one(&mut *tx)
        .await?;
        claimed = already_today == 0;
    }
    if claimed {
        sqlx::query(
            "INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) \
             VALUES(NULL, ?, 'source_health', ?, ?)",
        )
        .bind(lead)
        .bind(&body)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(claimed)
}

pub async fn clear(pool: &SqlitePool, source: &str) -> Result<()> {
    sqlx::query("DELETE FROM source_health WHERE source = ?")
        .bind(source)
        .execute(pool)
        .await?;
    Ok(())
}

//! Retention policies and automatic pruning for ephemeral per-turn and per-item tables.
//!
//! Only rows nothing reads back are removed. Two kinds of state are deliberately kept:
//! live state (a source that is still dead) and dedupe state a still-present row depends on
//! (`action_loop_seen`). Pruning either would re-notify the owner or re-enqueue old items.

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::SqlitePool;

use crate::{OmonError, Result};

/// Default retention period in days for completed ephemeral rows.
pub const DEFAULT_RETENTION_DAYS: i64 = 30;

/// Environment override for [`DEFAULT_RETENTION_DAYS`], read once so the boot prune and the
/// daily sweep use the same window.
pub const RETENTION_DAYS_ENV: &str = "OMON_STORAGE_RETENTION_DAYS";

/// Counts of rows pruned across tables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub turn_status_messages: u64,
    pub action_loop_seen: u64,
    pub action_loop_outcomes: u64,
    pub session_watch_notices: u64,
    pub report_claims: u64,
}

impl PruneReport {
    /// Returns the grand total of all rows pruned.
    pub fn total(&self) -> u64 {
        self.turn_status_messages
            + self.action_loop_seen
            + self.action_loop_outcomes
            + self.session_watch_notices
            + self.report_claims
    }
}

/// Reads [`RETENTION_DAYS_ENV`], defaulting to [`DEFAULT_RETENTION_DAYS`].
pub fn retention_days_from_environment() -> Result<i64> {
    parse_retention_days(std::env::var_os(RETENTION_DAYS_ENV).as_deref())
}

/// The parsing half of [`retention_days_from_environment`], split out so it can be tested
/// without mutating process-wide environment state.
fn parse_retention_days(value: Option<&std::ffi::OsStr>) -> Result<i64> {
    let Some(value) = value else {
        return Ok(DEFAULT_RETENTION_DAYS);
    };
    let value = value.to_string_lossy();
    let days = value.parse::<i64>().map_err(|_| {
        OmonError::Config(format!(
            "{RETENTION_DAYS_ENV} must be a whole number of days, got `{value}`"
        ))
    })?;
    if days < 1 {
        return Err(OmonError::Config(format!(
            "{RETENTION_DAYS_ENV} must be at least 1, got `{value}`: a shorter window would \
             delete every completed row on each sweep"
        )));
    }
    Ok(days)
}

/// Runs one `DELETE`, propagating every failure: migrations create all of these tables, so a
/// missing table means a broken schema and must not be mistaken for "nothing to prune".
async fn delete_before(pool: &SqlitePool, sql: &str, cutoff: DateTime<Utc>) -> Result<u64> {
    let result = sqlx::query(sql).bind(cutoff).execute(pool).await?;
    Ok(result.rows_affected())
}

/// Prunes completed and expired records using the configured window
/// ([`RETENTION_DAYS_ENV`], default [`DEFAULT_RETENTION_DAYS`]).
pub async fn prune(pool: &SqlitePool, now: DateTime<Utc>) -> Result<PruneReport> {
    prune_older_than(pool, retention_days_from_environment()?, now).await
}

/// Prunes completed and expired records older than `retention_days`.
///
/// Removed, once past the cutoff:
/// - `turn_status_messages`: finished states (`done`, `failed`); only a `running` row is read
///   back, by a restarted turn adopting its status message.
/// - `action_loop_seen`: marks whose item row is gone, so no scan can produce that item again.
///   A mark is the only thing that stops `scan_unseen_candidates` from re-enqueueing its item,
///   so marks for items that still exist are kept no matter how old they are.
/// - `action_loop_outcomes`: `verified` (the provider readback returned this item's result) and
///   `failed` (the loop gave up); both are terminal. `enqueued` is in flight, and `executed` only
///   says the lead acted and recorded a reference, so both stay until the item is verified.
/// - `session_watch_notices`: delivered notices; a duplicate is only possible while the same
///   notice key is live, and keys carry a per-session message count or question hash.
/// - `report_claims`: resolved claims; `pending` ones are swept to `unmeasurable` at boot.
///
/// `source_health` is never pruned: the row *is* the live "this source is dead" marker plus its
/// once-per-KST-day notification claim, so deleting it would drop an active dead source and let
/// the next failure notify the owner again on the same day.
pub async fn prune_older_than(
    pool: &SqlitePool,
    retention_days: i64,
    now: DateTime<Utc>,
) -> Result<PruneReport> {
    if retention_days < 1 {
        return Err(OmonError::Config(format!(
            "storage retention days must be at least 1, got {retention_days}"
        )));
    }
    let cutoff = TimeDelta::try_days(retention_days)
        .and_then(|span| now.checked_sub_signed(span))
        .ok_or_else(|| {
            OmonError::Config(format!(
                "storage retention of {retention_days} days is out of range"
            ))
        })?;

    let turn_status_messages = delete_before(
        pool,
        "DELETE FROM turn_status_messages \
         WHERE state <> 'running' \
           AND julianday(updated_at) < julianday(?)",
        cutoff,
    )
    .await?;

    // Mirrors the item keys built by `scan_unseen_candidates`; any prefix this loop does not
    // know is left alone rather than guessed at.
    let action_loop_seen = delete_before(
        pool,
        "DELETE FROM action_loop_seen \
         WHERE julianday(seen_at) < julianday(?) \
           AND ( \
             (item_key LIKE 'inbox:%' AND NOT EXISTS ( \
                 SELECT 1 FROM inbox_items i WHERE action_loop_seen.item_key = 'inbox:' || i.id)) \
             OR (item_key LIKE 'github:%' AND NOT EXISTS ( \
                 SELECT 1 FROM inbox_items i WHERE action_loop_seen.item_key = 'github:' || i.id)) \
             OR (item_key LIKE 'lane_event:%' AND NOT EXISTS ( \
                 SELECT 1 FROM lane_events e WHERE action_loop_seen.item_key = 'lane_event:' || e.id)) \
             OR (item_key LIKE 'source_health:%' AND NOT EXISTS ( \
                 SELECT 1 FROM source_health h \
                 WHERE action_loop_seen.item_key = 'source_health:' || h.source || ':' || h.since)) \
           )",
        cutoff,
    )
    .await?;

    let action_loop_outcomes = delete_before(
        pool,
        "DELETE FROM action_loop_outcomes \
         WHERE status IN ('verified', 'failed') \
           AND julianday(updated_at) < julianday(?)",
        cutoff,
    )
    .await?;

    let session_watch_notices = delete_before(
        pool,
        "DELETE FROM session_watch_notices \
         WHERE julianday(sent_at) < julianday(?)",
        cutoff,
    )
    .await?;

    let report_claims = delete_before(
        pool,
        "DELETE FROM report_claims \
         WHERE state <> 'pending' \
           AND julianday(COALESCE(checked_at, created_at)) < julianday(?)",
        cutoff,
    )
    .await?;

    Ok(PruneReport {
        turn_status_messages,
        action_loop_seen,
        action_loop_outcomes,
        session_watch_notices,
        report_claims,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    async fn count(pool: &SqlitePool, sql: &str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn test_prune_retention_only_removes_old_completed_rows() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let now = at(2026, 10, 20);
        let old = at(2026, 9, 1);
        let recent = at(2026, 10, 15);

        // 1. turn_status_messages: a running row is never pruned, finished ones are.
        for (session, event, state, stamp) in [
            ("s1", "e1", "running", old),
            ("s2", "e2", "done", old),
            ("s3", "e3", "done", recent),
            ("s4", "e4", "failed", old),
        ] {
            sqlx::query(
                "INSERT INTO turn_status_messages (session_key, event_id, platform_message_id, started_at, state, updated_at) \
                 VALUES (?, ?, 'm', ?, ?, ?)",
            )
            .bind(session)
            .bind(event)
            .bind(stamp)
            .bind(state)
            .bind(stamp)
            .execute(pool)
            .await
            .unwrap();
        }

        // 2. action_loop_seen: a mark whose item still exists must survive the cutoff.
        sqlx::query(
            "INSERT INTO inbox_items (id, source, source_ref, snippet, received_at, raw_hash, collected_at) \
             VALUES ('kept', 'mail', 'kept', 's', ?, 'h', ?)",
        )
        .bind(old)
        .bind(old)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO lane_events (lead_session_key, kind, body, created_at) \
             VALUES ('lead', 'blocked', 'b', ?)",
        )
        .bind(old)
        .execute(pool)
        .await
        .unwrap();
        let event_id: i64 = count(pool, "SELECT max(id) FROM lane_events").await;
        sqlx::query(
            "INSERT INTO source_health (source, state, detail, since, notified_on) \
             VALUES ('mail', 'auth_failed', 'auth_expired', ?, '2026-09-01')",
        )
        .bind(old)
        .execute(pool)
        .await
        .unwrap();
        let since: String =
            sqlx::query_scalar("SELECT since FROM source_health WHERE source = 'mail'")
                .fetch_one(pool)
                .await
                .unwrap();
        for key in [
            "inbox:kept".to_owned(),
            "inbox:gone".to_owned(),
            format!("lane_event:{event_id}"),
            "lane_event:9999".to_owned(),
            format!("source_health:mail:{since}"),
            "legacy:unknown".to_owned(),
        ] {
            sqlx::query("INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)")
                .bind(key)
                .bind(old)
                .execute(pool)
                .await
                .unwrap();
        }

        // 3. action_loop_outcomes: only verified and failed are terminal.
        for (key, status, stamp) in [
            ("o1", "verified", old),
            ("o2", "executed", old),
            ("o3", "enqueued", old),
            ("o4", "failed", old),
            ("o5", "verified", recent),
        ] {
            sqlx::query(
                "INSERT INTO action_loop_outcomes \
                    (item_key, lane_event_id, kind, source, line, status, external_id, attempts, enqueued_at, updated_at) \
                 VALUES (?, ?, 'inbox_bot', 'mail', 'l', ?, ?, 1, ?, ?)",
            )
            .bind(key)
            .bind(event_id)
            .bind(status)
            .bind(if status == "verified" { Some("evt-1") } else { None })
            .bind(stamp)
            .bind(stamp)
            .execute(pool)
            .await
            .unwrap();
        }

        // 4. session_watch_notices: delivered notices expire.
        for (key, stamp) in [("k_old", old), ("k_recent", recent)] {
            sqlx::query(
                "INSERT INTO session_watch_notices (dedupe_key, target, kind, sent_at) VALUES (?, 't1', 'alert', ?)",
            )
            .bind(key)
            .bind(stamp)
            .execute(pool)
            .await
            .unwrap();
        }

        // 5. report_claims: pending claims are never pruned, resolved ones expire.
        for (message, state, stamp) in [
            ("m1", "pending", old),
            ("m2", "confirmed", old),
            ("m3", "confirmed", recent),
        ] {
            sqlx::query(
                "INSERT INTO report_claims (session_key, platform_message_id, sentence, argv_json, expect_regex, state, created_at, checked_at) \
                 VALUES ('s1', ?, 'sent', '[]', '.*', ?, ?, ?)",
            )
            .bind(message)
            .bind(state)
            .bind(stamp)
            .bind(if state == "pending" { None } else { Some(stamp) })
            .execute(pool)
            .await
            .unwrap();
        }

        // Explicit window, not `prune`: the test must not depend on the ambient environment.
        let report = prune_older_than(pool, DEFAULT_RETENTION_DAYS, now)
            .await
            .unwrap();
        assert_eq!(report.turn_status_messages, 2, "old done and failed rows");
        assert_eq!(report.action_loop_seen, 2, "only marks whose item is gone");
        assert_eq!(report.action_loop_outcomes, 2, "old verified and failed");
        assert_eq!(report.session_watch_notices, 1);
        assert_eq!(report.report_claims, 1);
        assert_eq!(report.total(), 8);

        // Live state and dedupe state survive.
        assert_eq!(
            count(pool, "SELECT count(*) FROM source_health").await,
            1,
            "an active dead source is never pruned"
        );
        assert_eq!(
            count(pool, "SELECT count(*) FROM action_loop_seen").await,
            4,
            "marks for existing items, source_health and unknown prefixes stay"
        );
        assert_eq!(
            count(
                pool,
                "SELECT count(*) FROM action_loop_outcomes WHERE status IN ('executed', 'enqueued')"
            )
            .await,
            2
        );
        assert_eq!(
            count(pool, "SELECT count(*) FROM turn_status_messages").await,
            2
        );
        assert_eq!(
            count(pool, "SELECT count(*) FROM session_watch_notices").await,
            1
        );
        assert_eq!(count(pool, "SELECT count(*) FROM report_claims").await, 2);
    }

    #[test]
    fn retention_window_defaults_and_refuses_a_degenerate_value() {
        use std::ffi::OsStr;

        assert_eq!(parse_retention_days(None).unwrap(), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention_days(Some(OsStr::new("90"))).unwrap(), 90);
        for bad in ["0", "-1", "", "30d", " 30"] {
            let error = parse_retention_days(Some(OsStr::new(bad))).unwrap_err();
            assert!(
                error.to_string().contains(RETENTION_DAYS_ENV),
                "{bad}: {error}"
            );
        }
    }

    #[tokio::test]
    async fn prune_rejects_a_zero_day_window() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let error = prune_older_than(database.pool(), 0, at(2026, 10, 20))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("at least 1"), "{error}");
    }
}

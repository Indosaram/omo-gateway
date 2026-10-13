use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;
use sqlx::SqlitePool;

use crate::{Result, SessionKey};

/// Upper bound for each (first / last) line kept in a problem signature.
const SIGNATURE_LINE_CHARS: usize = 160;

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}\b").expect("valid regex")
});
static HEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[0-9a-fA-F]{8,}\b").expect("valid regex"));
static DIGITS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]+").expect("valid regex"));

fn normalize_signature_line(line: &str) -> String {
    let line = UUID_RE.replace_all(line.trim(), "<uuid>");
    let line = HEX_RE.replace_all(&line, "<hex>");
    let line = DIGITS_RE.replace_all(&line, "#");
    line.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(SIGNATURE_LINE_CHARS)
        .collect()
}

/// Dedup key for a problem: the kind plus a normalized, bounded form of the error text.
/// Timestamps, pids, request ids and other digits or hex tokens are stripped so one
/// underlying fault that prints varying noise maps to a single problem row.
pub(super) fn problem_signature(kind: &str, detail: &str) -> String {
    let mut lines = detail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let first = lines
        .next()
        .map(normalize_signature_line)
        .unwrap_or_default();
    match lines.next_back().map(normalize_signature_line) {
        Some(last) if last != first => format!("{kind}:{first} | {last}"),
        _ => format!("{kind}:{first}"),
    }
}

pub async fn on_run_finished(
    pool: &SqlitePool,
    job_id: &str,
    run_id: &str,
    delivery_failed: bool,
) -> Result<()> {
    if !watch_enabled() {
        return Ok(());
    }

    let now = chrono::Utc::now();
    let payload_json: Option<String> =
        sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(pool)
            .await?;
    let Some(payload_json) = payload_json else {
        return Ok(());
    };
    let payload: Value = serde_json::from_str::<Value>(&payload_json).unwrap_or_default();
    let terminal: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT status, error FROM cron_runs WHERE run_id = ? AND job_id = ? AND status != 'running'",
    )
    .bind(run_id)
    .bind(job_id)
    .fetch_optional(pool)
    .await?;
    let Some((status, run_error)) = terminal else {
        return Ok(());
    };
    let output = if status == "succeeded" {
        match output_for_run(pool, job_id, run_id).await {
            Ok(output) => output,
            Err(error) => {
                tracing::error!(%error, job_id, run_id, "cron health output query failed");
                return Ok(());
            }
        }
    } else {
        None
    };
    let problem = match status.as_str() {
        "failed" if run_error.as_deref() == Some("lease expired before completion") => {
            Some(("stuck", "lease expired before completion".to_string()))
        }
        "failed" if delivery_failed => Some((
            "delivery_failed",
            run_error
                .clone()
                .unwrap_or_else(|| "delivery failed".to_string()),
        )),
        "failed" => Some((
            "failed",
            run_error.unwrap_or_else(|| "cron run failed".to_string()),
        )),
        "succeeded" => match classify_expected_output(&payload, &output) {
            Ok(Some(detail)) => Some(("empty_success", detail)),
            Ok(None) => None,
            Err(error) => {
                tracing::error!(%error, job_id, run_id, "cron health output check failed");
                Some(("empty_success", error))
            }
        },
        _ => None,
    };

    let signature = match &problem {
        Some((kind, detail)) => problem_signature(kind, detail),
        None => String::new(),
    };
    let stale_cutoff = now - chrono::TimeDelta::seconds(super::remediate::STALE_REMEDIATING_SECS);
    let mut tx = pool.begin().await?;
    if let Some((kind, detail)) = problem {
        let inserted = sqlx::query(
            "INSERT INTO cron_problems(job_id, kind, run_id, signature, detail, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'open', ?, ?)
             ON CONFLICT(job_id, kind, signature) DO UPDATE SET
             status = 'open', run_id = excluded.run_id, detail = excluded.detail,
             lane_key = NULL, created_at = excluded.created_at, updated_at = excluded.updated_at
             WHERE (cron_problems.status IN ('fixed', 'ignored')
                    OR (cron_problems.status = 'remediating'
                        AND julianday(cron_problems.updated_at) < julianday(?)
                        AND (cron_problems.lane_key IS NULL
                             OR NOT EXISTS (SELECT 1 FROM lanes
                                            WHERE lanes.key = cron_problems.lane_key
                                            AND lanes.status NOT IN ('verified_done', 'closed')))))
             AND cron_problems.run_id IS NOT excluded.run_id",
        )
        .bind(job_id)
        .bind(kind)
        .bind(run_id)
        .bind(&signature)
        .bind(&detail)
        .bind(now)
        .bind(now)
        .bind(stale_cutoff)
        .execute(&mut *tx)
        .await?;
        if inserted.rows_affected() > 0 {
            let problem_id: i64 = sqlx::query_scalar(
                "SELECT id FROM cron_problems WHERE job_id = ? AND kind = ? AND signature = ?",
            )
            .bind(job_id)
            .bind(kind)
            .bind(&signature)
            .fetch_one(&mut *tx)
            .await?;
            if let Some(lead) = default_lead() {
                crate::storage::lanes::enqueue_lane_event_with_executor(
                    &mut *tx,
                    None,
                    &lead,
                    "cron_problem",
                    &serde_json::json!({"problem_id": problem_id, "job_id": job_id, "kind": kind, "run_id": run_id, "detail": detail}).to_string(),
                )
                .await?;
            }
        }
    } else {
        sqlx::query(
            "UPDATE cron_problems SET status = 'fixed', updated_at = ? WHERE job_id = ? AND status IN ('open','remediating','needs_owner')",
        )
        .bind(now)
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn output_for_run(pool: &SqlitePool, job_id: &str, run_id: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT output FROM cron_outputs WHERE job_id = ? AND run_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(job_id)
    .bind(run_id)
    .fetch_optional(pool)
    .await?)
}

fn classify_expected_output(
    payload: &Value,
    output: &Option<String>,
) -> std::result::Result<Option<String>, String> {
    let Some(pattern) = payload
        .get("expect")
        .and_then(|value| value.get("output_regex"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let regex =
        Regex::new(pattern).map_err(|error| format!("invalid expected output regex: {error}"))?;
    Ok(
        (!output.as_deref().is_some_and(|text| regex.is_match(text)))
            .then(|| format!("output did not match expected regex: {pattern}")),
    )
}

pub async fn record_missed_occurrence(
    pool: &SqlitePool,
    job_id: &str,
    occurrence: chrono::DateTime<chrono::Utc>,
    next_occurrence: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    if !watch_enabled() {
        return Ok(());
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM cron_runs WHERE job_id = ? AND started_at >= ? AND started_at < ?)",
    )
    .bind(job_id)
    .bind(occurrence)
    .bind(next_occurrence)
    .fetch_one(pool)
    .await?;
    if exists {
        return Ok(());
    }
    record_problem(
        pool,
        job_id,
        "missed",
        None,
        &occurrence.to_rfc3339(),
        "scheduled occurrence had no run row",
    )
    .await
}

async fn record_problem(
    pool: &SqlitePool,
    job_id: &str,
    kind: &str,
    run_id: Option<&str>,
    signature: &str,
    detail: &str,
) -> Result<()> {
    let now = chrono::Utc::now();
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO cron_problems(job_id, kind, run_id, signature, detail, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'open', ?, ?) ON CONFLICT(job_id, kind, signature) DO NOTHING",
    )
    .bind(job_id)
    .bind(kind)
    .bind(run_id)
    .bind(signature)
    .bind(detail)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    if inserted.rows_affected() > 0 {
        let problem_id: i64 = sqlx::query_scalar(
            "SELECT id FROM cron_problems WHERE job_id = ? AND kind = ? AND signature = ?",
        )
        .bind(job_id)
        .bind(kind)
        .bind(signature)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(lead) = default_lead() {
            crate::storage::lanes::enqueue_lane_event_with_executor(
                &mut *tx,
                None,
                &lead,
                "cron_problem",
                &serde_json::json!({"problem_id":problem_id,"job_id":job_id,"kind":kind,"run_id":run_id,"detail":detail}).to_string(),
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

fn default_lead() -> Option<String> {
    match std::env::var("OMON_LANE_DEFAULT_LEAD") {
        Ok(lead) => match SessionKey::from_storage_key(&lead) {
            Ok(session) => Some(session.storage_key()),
            Err(error) => {
                tracing::warn!(%error, "invalid OMON_LANE_DEFAULT_LEAD; cron problem recorded without event");
                None
            }
        },
        Err(error) => {
            tracing::warn!(%error, "OMON_LANE_DEFAULT_LEAD unavailable; cron problem recorded without event");
            None
        }
    }
}

pub(super) fn watch_enabled() -> bool {
    watch_enabled_from(std::env::var("OMON_CRON_WATCH_ENABLED").ok().as_deref())
}

fn watch_enabled_from(value: Option<&str>) -> bool {
    value
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::{problem_signature, watch_enabled_from};

    #[test]
    fn signature_ignores_timestamps_pids_and_ids_but_keeps_the_fault() {
        let first = problem_signature(
            "failed",
            "cron script failed with Some(1): 2026-10-13T01:02:03Z pid 4242 req 9f8e7d6c5b4a connect refused",
        );
        let second = problem_signature(
            "failed",
            "cron script failed with Some(1): 2026-10-14T11:12:13Z pid 17 req 00112233aabb connect refused",
        );
        assert_eq!(first, second);
        let other = problem_signature("failed", "cron script failed with Some(1): disk full");
        assert_ne!(first, other);
        assert_ne!(first, problem_signature("stuck", "cron script failed with Some(1): 2026-10-13T01:02:03Z pid 4242 req 9f8e7d6c5b4a connect refused"));
    }

    #[test]
    fn signature_is_bounded_and_uses_first_and_last_line() {
        let trace = format!("Traceback\n{}\nValueError: boom 77", "frame\n".repeat(5000));
        let long_line = "x".repeat(100_000);
        assert!(problem_signature("failed", &long_line).chars().count() < 200);
        let sig = problem_signature("failed", &trace);
        assert!(sig.chars().count() < 400);
        assert_eq!(sig, "failed:traceback | valueerror: boom #");
    }

    #[test]
    fn switch_defaults_on_and_disables_false_like_values() {
        assert!(watch_enabled_from(None));
        assert!(watch_enabled_from(Some("true")));
        assert!(!watch_enabled_from(Some("false")));
        assert!(!watch_enabled_from(Some("0")));
        assert!(!watch_enabled_from(Some("off")));
    }
}

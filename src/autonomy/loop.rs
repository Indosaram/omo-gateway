use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use super::ledger::{in_quiet_hours_kst, next_0800_kst};

pub const ENABLED_ENV: &str = "OMON_ACTION_LOOP_ENABLED";
pub const DAILY_CAP_ENV: &str = "OMON_ACTION_LOOP_DAILY_CAP";
pub const DEFAULT_DAILY_CAP: i64 = 24;
pub const COALESCE_WINDOW: Duration = Duration::from_secs(120);
const MAX_LISTED_ITEMS: usize = 20;
/// An enqueued item whose event reached the lead this long ago without an outcome is treated as
/// a failed turn and re-queued.
pub const REQUEUE_AFTER: Duration = Duration::from_secs(30 * 60);
/// Emissions per item (first enqueue plus re-queues) before it is marked failed.
pub const MAX_ATTEMPTS: i64 = 3;
/// The statuses an item can carry. `verified` is the only "done" and it needs a provider readback
/// (see `record_outcome`); once set it is terminal against later reports.
pub const OUTCOME_STATUSES: [&str; 3] = ["executed", "verified", "failed"];
/// An item whose source is older than this is emitted with a `[backlog]` marker, never dropped:
/// a long outage must not lose work, but the lead has to know the item is not fresh.
pub const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
/// `detail` prefix for an item whose last delivery produced no outcome: its side effect is
/// unknown, so the next emission asks for reconciliation instead of a repeat of the action.
pub const RECONCILE_PREFIX: &str = "reconcile:";
/// `detail` prefix written once such an item has been reconciled.
pub const RECONCILED_PREFIX: &str = "reconciled:";
/// CLI that can read a calendar event back (`<cmd> cal get <event id>`).
pub const CALENDAR_CMD_ENV: &str = "OMON_ACTION_LOOP_CALENDAR_CMD";
pub const DEFAULT_CALENDAR_CMD: &str = "zele";

pub fn action_loop_enabled() -> bool {
    matches!(
        std::env::var(ENABLED_ENV)
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Ok("1" | "true" | "yes" | "on")
    )
}

pub fn daily_cap_from_env() -> i64 {
    std::env::var(DAILY_CAP_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_DAILY_CAP)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopItem {
    pub item_key: String,
    pub kind: String,
    pub source: String,
    pub line: String,
    pub link: Option<String>,
    /// Emission number this item is going out as: 1 for a first sighting, n for a re-queue.
    pub attempts: i64,
    /// The previous delivery of this item produced no outcome, so whether it already happened is
    /// unknown: it is emitted for reconciliation, not for execution.
    pub reconcile: bool,
    /// When the underlying source row appeared, so stale backlog can be marked as such.
    pub source_at: Option<DateTime<Utc>>,
}

impl LoopItem {
    /// Old enough that acting on it without checking first may be wrong: an item from a long
    /// outage, not a fresh arrival.
    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        self.source_at.is_some_and(|at| {
            now.signed_duration_since(at)
                .to_std()
                .map(|age| age >= STALE_AFTER)
                .unwrap_or(false)
        })
    }

    pub fn format_entry(&self) -> String {
        match &self.link {
            Some(link) if !link.is_empty() => {
                format!("[{}:{}] {} ({})", self.kind, self.source, self.line, link)
            }
            _ => format!("[{}:{}] {}", self.kind, self.source, self.line),
        }
    }
}

pub struct ActionLoop {
    pub pool: SqlitePool,
    pub lead_session_key: String,
    pub daily_cap: i64,
    pub coalesce_window: Duration,
    pending_batch: tokio::sync::Mutex<Vec<LoopItem>>,
}

impl ActionLoop {
    pub fn new(pool: SqlitePool, lead_session_key: impl Into<String>) -> Self {
        Self {
            pool,
            lead_session_key: lead_session_key.into(),
            daily_cap: daily_cap_from_env(),
            coalesce_window: COALESCE_WINDOW,
            pending_batch: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_daily_cap(mut self, cap: i64) -> Self {
        self.daily_cap = cap.max(1);
        self
    }

    pub fn with_coalesce_window(mut self, window: Duration) -> Self {
        self.coalesce_window = window;
        self
    }

    /// Buffers unseen items. An item is marked seen only when `flush_coalesced` emits it, so a
    /// restart inside the coalesce window, a failed flush or the daily cap never loses it: the
    /// next scan finds it again.
    /// Also re-queues enqueued items whose lead turn ended without an outcome (bounded by
    /// `MAX_ATTEMPTS`).
    pub async fn collect_new_items(&self, now: DateTime<Utc>) -> sqlx::Result<usize> {
        let mut candidates = scan_unseen_candidates(&self.pool, &self.lead_session_key).await?;
        candidates.extend(requeue_stale_outcomes(&self.pool, &self.lead_session_key, now).await?);
        let mut added = 0usize;
        let mut batch = self.pending_batch.lock().await;
        for item in candidates {
            if batch.iter().any(|queued| queued.item_key == item.item_key) {
                continue;
            }
            batch.push(item);
            added += 1;
        }
        Ok(added)
    }

    pub async fn flush_coalesced(&self, now: DateTime<Utc>) -> sqlx::Result<Option<i64>> {
        let _ = self.collect_new_items(now).await?;
        let batch: Vec<LoopItem> = {
            let mut batch = self.pending_batch.lock().await;
            if batch.is_empty() {
                return Ok(None);
            }
            std::mem::take(&mut *batch)
        };
        // `item_key` is unique across leads (migration 0054) and the owning lead is only recorded
        // on the row's lane_event, so a second lead must not take an item another lead already
        // emitted: the event and the retry accounting belong to that lead.
        let foreign: Vec<String> = sqlx::query_scalar(
            "SELECT o.item_key
             FROM action_loop_outcomes o
             JOIN lane_events e ON e.id = o.lane_event_id
             WHERE e.lead_session_key <> ?",
        )
        .bind(&self.lead_session_key)
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::with_capacity(batch.len());
        for item in batch {
            if foreign.iter().any(|key| key == &item.item_key) {
                tracing::warn!(
                    lead = %self.lead_session_key,
                    item_key = %item.item_key,
                    "action_loop item already belongs to another lead; not taking it over"
                );
                // Mark it seen so this lead stops re-scanning it on every sweep.
                sqlx::query(
                    "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
                     ON CONFLICT(item_key) DO NOTHING",
                )
                .bind(&item.item_key)
                .bind(now)
                .execute(&self.pool)
                .await?;
                continue;
            }
            items.push(item);
        }
        if items.is_empty() {
            return Ok(None);
        }

        let day_start = kst_day_start_utc(now);
        let day_end = day_start + chrono::Duration::days(1);
        let today_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM lane_events
             WHERE kind = 'action_loop'
               AND lead_session_key = ?
               AND julianday(created_at) >= julianday(?)
               AND julianday(created_at) < julianday(?)",
        )
        .bind(&self.lead_session_key)
        .bind(day_start)
        .bind(day_end)
        .fetch_one(&self.pool)
        .await?;

        if today_count >= self.daily_cap {
            tracing::warn!(
                lead = %self.lead_session_key,
                cap = self.daily_cap,
                deferred_items = items.len(),
                "action_loop daily cap reached; deferring the batch to the next flush"
            );
            // Leave unseen items out of the buffer so the next scan re-arms the flush timer.
            return Ok(None);
        }

        let mut fresh = Vec::new();
        let mut reconcile = Vec::new();
        for item in items {
            if item.reconcile {
                reconcile.push(item);
            } else {
                fresh.push(item);
            }
        }
        // Two events may be written in one flush; the cap counts events, not items.
        let mut budget = self.daily_cap - today_count;
        let mut last = None;
        for (group, is_reconcile) in [(fresh, false), (reconcile, true)] {
            if group.is_empty() {
                continue;
            }
            if budget <= 0 {
                tracing::warn!(
                    lead = %self.lead_session_key,
                    cap = self.daily_cap,
                    deferred_items = group.len(),
                    "action_loop daily cap reached mid-flush; the rest waits for the next day"
                );
                continue;
            }
            last = Some(self.emit(&group, now, is_reconcile).await?);
            budget -= 1;
        }
        Ok(last)
    }

    /// Writes one event with its seen marks and `enqueued` outcome rows in a single transaction.
    /// `reconcile` switches the instruction from "act" to "do not repeat the action": those items
    /// were delivered once already and produced no outcome, so their side effect is unknown and
    /// re-executing them would duplicate it.
    async fn emit(
        &self,
        items: &[LoopItem],
        now: DateTime<Utc>,
        reconcile: bool,
    ) -> sqlx::Result<i64> {
        let mut formatted: Vec<String> = items
            .iter()
            .take(MAX_LISTED_ITEMS)
            .map(|item| {
                let mut marks = String::new();
                if reconcile {
                    marks.push_str(&format!(
                        "[reconcile attempt {}/{}] ",
                        item.attempts, MAX_ATTEMPTS
                    ));
                } else if item.attempts > 1 {
                    marks.push_str(&format!("[retry {}/{}] ", item.attempts, MAX_ATTEMPTS));
                }
                if item.is_stale(now) {
                    marks.push_str("[backlog] ");
                }
                format!("{marks}{} {}", item.item_key, item.format_entry())
            })
            .collect();
        if items.len() > MAX_LISTED_ITEMS {
            formatted.push(format!("+{} more", items.len() - MAX_LISTED_ITEMS));
        }
        let body = if reconcile {
            format!(
                "[ACTION_LOOP_RECONCILE] {} items whose earlier delivery produced no outcome; the side effect is unknown, so do NOT repeat the action. Search the calendar for the item key first (the description of any event you created carries it). If it already happened, record `omo-gateway autonomy outcome <key> --status verified --external-id <that event id>`; if it did not, record `--status failed --detail <what you checked>`. | {}",
                items.len(),
                formatted.join(" | ")
            )
        } else {
            format!(
                "[ACTION_LOOP] {} items: enqueued is not done. After acting on an item run `omo-gateway autonomy outcome <key> --status executed --external-id <result reference> --detail <what you did>` (or --status failed --detail <why>). An item counts as done only with `--status verified --external-id <calendar event id>`, which the gateway confirms by reading that event back from the calendar and checking it carries the item key: put `<key>` in the event description when you create it. Other kinds of result cannot be verified yet, so record them as `executed` and never report them as done. | {}",
                items.len(),
                formatted.join(" | ")
            )
        };
        let release_at: Option<DateTime<Utc>> = if in_quiet_hours_kst(now) {
            Some(next_0800_kst(now))
        } else {
            None
        };

        let mut tx = self.pool.begin().await?;
        let res = sqlx::query(
            "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at, release_at)
             VALUES (NULL, ?, 'action_loop', ?, ?, ?)",
        )
        .bind(&self.lead_session_key)
        .bind(&body)
        .bind(now)
        .bind(release_at)
        .execute(&mut *tx)
        .await?;
        let event_id = res.last_insert_rowid();
        for item in items {
            sqlx::query(
                "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
                 ON CONFLICT(item_key) DO NOTHING",
            )
            .bind(&item.item_key)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO action_loop_outcomes
                    (item_key, lane_event_id, kind, source, line, link, status, attempts, enqueued_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, 'enqueued', 1, ?, ?)
                 ON CONFLICT(item_key) DO UPDATE SET
                    lane_event_id = excluded.lane_event_id,
                    attempts = action_loop_outcomes.attempts + 1,
                    updated_at = excluded.updated_at
                 WHERE action_loop_outcomes.status = 'enqueued'",
            )
            .bind(&item.item_key)
            .bind(event_id)
            .bind(&item.kind)
            .bind(&item.source)
            .bind(&item.line)
            .bind(&item.link)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let totals = outcome_counts_for_lead(&self.pool, &self.lead_session_key).await?;
        let count = |status: &str| {
            totals
                .iter()
                .find(|(s, _)| s == status)
                .map(|(_, n)| *n)
                .unwrap_or(0)
        };
        tracing::info!(
            items = items.len(),
            event_id,
            reconcile,
            total_enqueued = count("enqueued"),
            total_executed = count("executed"),
            total_verified = count("verified"),
            total_failed = count("failed"),
            "action_loop enqueued"
        );
        Ok(event_id)
    }

    /// On first enable, marks every existing candidate as seen without emitting, so only
    /// rows that arrive after the loop is switched on become events.
    pub async fn baseline_if_first_run(&self, now: DateTime<Utc>) -> sqlx::Result<usize> {
        let seen: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM action_loop_seen")
            .fetch_one(&self.pool)
            .await?;
        if seen > 0 {
            return Ok(0);
        }
        let candidates = scan_unseen_candidates(&self.pool, &self.lead_session_key).await?;
        for item in &candidates {
            sqlx::query(
                "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
                 ON CONFLICT(item_key) DO NOTHING",
            )
            .bind(&item.item_key)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }
        tracing::info!(
            "action_loop baseline marked existing items seen items={}",
            candidates.len()
        );
        Ok(candidates.len())
    }

    pub async fn serve(self, wake: Arc<Notify>, shutdown: CancellationToken) -> crate::Result<()> {
        if let Err(error) = self.baseline_if_first_run(Utc::now()).await {
            tracing::warn!(%error, "action_loop baseline failed");
        }
        let mut coalesce_deadline: Option<tokio::time::Instant> = None;
        let mut sweep_interval = tokio::time::interval(Duration::from_secs(30));
        sweep_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            if shutdown.is_cancelled() {
                break;
            }
            let sleep_fut = async {
                match coalesce_deadline {
                    Some(dl) => tokio::time::sleep_until(dl).await,
                    None => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                _ = sleep_fut => {
                    coalesce_deadline = None;
                    if let Err(error) = self.flush_coalesced(Utc::now()).await {
                        tracing::warn!(%error, "action_loop flush failed");
                    }
                }
                _ = wake.notified() => {
                    match self.collect_new_items(Utc::now()).await {
                        Ok(added) if added > 0 && coalesce_deadline.is_none() => {
                            coalesce_deadline = Some(tokio::time::Instant::now() + self.coalesce_window);
                        }
                        Ok(_) => {}
                        Err(error) => tracing::warn!(%error, "action_loop collect failed"),
                    }
                }
                _ = sweep_interval.tick() => {
                    match self.collect_new_items(Utc::now()).await {
                        Ok(added) if added > 0 && coalesce_deadline.is_none() => {
                            coalesce_deadline = Some(tokio::time::Instant::now() + self.coalesce_window);
                        }
                        Ok(_) => {}
                        Err(error) => tracing::warn!(%error, "action_loop periodic scan failed"),
                    }
                }
            }
        }
        Ok(())
    }
}

pub async fn outcome_counts(pool: &SqlitePool) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as(
        "SELECT status, COUNT(*) FROM action_loop_outcomes GROUP BY status ORDER BY status",
    )
    .fetch_all(pool)
    .await
}

/// Counts only the items whose event belongs to `lead_session_key`, so one lead's log line never
/// reports another lead's backlog.
pub async fn outcome_counts_for_lead(
    pool: &SqlitePool,
    lead_session_key: &str,
) -> sqlx::Result<Vec<(String, i64)>> {
    sqlx::query_as(
        "SELECT o.status, COUNT(*)
         FROM action_loop_outcomes o
         JOIN lane_events e ON e.id = o.lane_event_id
         WHERE e.lead_session_key = ?
         GROUP BY o.status ORDER BY o.status",
    )
    .bind(lead_session_key)
    .fetch_all(pool)
    .await
}

/// What a caller reports happened to an item, grouped so the record path takes one payload
/// instead of four loose arguments.
#[derive(Debug, Clone, Copy)]
pub struct OutcomeReport<'a> {
    pub status: &'a str,
    pub external_id: Option<&'a str>,
    pub detail: Option<&'a str>,
}

impl<'a> OutcomeReport<'a> {
    pub fn new(status: &'a str) -> Self {
        Self {
            status,
            external_id: None,
            detail: None,
        }
    }

    pub fn external_id(mut self, external_id: Option<&'a str>) -> Self {
        self.external_id = external_id;
        self
    }

    pub fn detail(mut self, detail: Option<&'a str>) -> Self {
        self.detail = detail;
        self
    }
}

/// The provider readback seam: which calendar CLI to ask and how long to wait for it. Grouped so a
/// test can hand the record path a stub without a per-knob argument list.
#[derive(Debug, Clone)]
pub struct Readback {
    pub program: String,
    pub timeout: Duration,
}

impl Readback {
    /// The configured provider (`OMON_ACTION_LOOP_CALENDAR_CMD`, default `zele`).
    pub fn from_env() -> Self {
        Self {
            program: std::env::var(CALENDAR_CMD_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| DEFAULT_CALENDAR_CMD.to_string()),
            timeout: READBACK_TIMEOUT,
        }
    }

    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            timeout: READBACK_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Records what the agent did with an action-loop item.
///
/// `executed` means the lead acted and recorded the reference of the result it produced. It is
/// still a claim by the agent, so the item is not done: it stays `executed` until something
/// outside this process confirms the result.
///
/// `verified` is the only status that means done, and it is not a string the caller supplies: the
/// gateway reads the event back from the calendar provider (`<cmd> cal get <event id>`, default
/// `zele`) and requires the returned event to carry the item key, which is how the artifact is
/// tied to this item rather than to any other event. A provider this process cannot read (a pull
/// request, a message) cannot be verified at all, and saying so is the honest answer.
///
/// An item whose earlier delivery produced no outcome is *unreconciled*: its side effect is
/// unknown. Such an item may only be closed by naming the artifact that already exists
/// (`executed --external-id`) or by saying what was checked (`failed --detail`), so a retry can
/// never silently repeat the side effect.
pub async fn record_outcome(
    pool: &SqlitePool,
    item_key: &str,
    report: OutcomeReport<'_>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    record_outcome_with(pool, item_key, report, now, &Readback::from_env()).await
}

async fn record_outcome_with(
    pool: &SqlitePool,
    item_key: &str,
    report: OutcomeReport<'_>,
    now: DateTime<Utc>,
    readback: &Readback,
) -> Result<(), String> {
    let key = item_key.trim();
    let status = report.status.trim().to_ascii_lowercase();
    if !OUTCOME_STATUSES.contains(&status.as_str()) {
        return Err(format!(
            "invalid status `{status}` (expected executed, verified or failed)"
        ));
    }
    let external_id = report
        .external_id
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let detail = report.detail.map(str::trim).filter(|why| !why.is_empty());

    let row: Option<(String, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "SELECT status, external_id, detail, attempts FROM action_loop_outcomes WHERE item_key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let Some((current_status, current_external_id, current_detail, attempts)) = row else {
        return Err(format!("unknown action-loop item `{key}`"));
    };
    // `verified` is terminal: a delayed report from an earlier attempt must not erase a result the
    // provider already confirmed. Only another readback may touch a verified row.
    if current_status == "verified" && status != "verified" {
        return Err(format!(
            "`{key}` is already verified ({}) and cannot be overwritten with `{status}`: a later report from an earlier attempt must not erase a confirmed result",
            current_external_id.as_deref().unwrap_or("no external id")
        ));
    }
    let unreconciled = current_detail
        .as_deref()
        .is_some_and(|d| d.starts_with(RECONCILE_PREFIX));

    let proof = if status == "verified" {
        let id = external_id.ok_or("--status verified needs --external-id <calendar event id>")?;
        Some(read_calendar_event_within(&readback.program, id, key, readback.timeout).await?)
    } else {
        None
    };

    if status == "executed" && unreconciled && external_id.is_none() {
        return Err(format!(
            "`{key}` is unreconciled: an earlier turn received it and recorded no outcome, so whether it already happened is unknown. Do not repeat the action; pass --external-id <the artifact that already exists> if it happened, or --status failed --detail <what you checked> if it did not."
        ));
    }
    if status == "failed" && unreconciled && detail.is_none() {
        return Err(format!(
            "`{key}` is unreconciled; record --detail <what you checked> so the unknown side effect is not silently dropped"
        ));
    }

    let detail = match (proof, unreconciled) {
        (Some(proof), true) => Some(format!("{RECONCILED_PREFIX}{proof}")),
        (Some(proof), false) => Some(proof),
        (None, true) => {
            let note = detail.map(|d| format!("; note={d}")).unwrap_or_default();
            Some(format!(
                "{RECONCILED_PREFIX}attempt {attempts} recorded as {status}{note}"
            ))
        }
        (None, false) => detail.map(str::to_string),
    };
    // Re-recording must not lose the reference the item already carries.
    let external_id = external_id.or(current_external_id.as_deref());

    // The guard repeats the read above inside the write, so a verification that lands between the
    // check and the update still cannot be clobbered.
    let res = sqlx::query(
        "UPDATE action_loop_outcomes
         SET status = ?, external_id = ?, detail = COALESCE(?, detail), updated_at = ?
         WHERE item_key = ? AND (status <> 'verified' OR ? = 'verified')",
    )
    .bind(&status)
    .bind(external_id)
    .bind(detail.as_deref())
    .bind(now)
    .bind(key)
    .bind(&status)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    if res.rows_affected() == 0 {
        return Err(format!(
            "`{key}` was verified or removed after this `{status}` report was read; nothing was written"
        ));
    }
    tracing::info!(
        item_key = key,
        %status,
        unreconciled,
        external_id = external_id.unwrap_or(""),
        readback = detail.as_deref().unwrap_or(""),
        "action_loop outcome recorded"
    );
    Ok(())
}

/// `detail` prefix on a row adopted for an item that was delivered before `action_loop_outcomes`
/// existed. It records that the outcome chain is a reconstruction, not an observed enqueue.
pub const LEGACY_PREFIX: &str = "legacy-reconcile:";

/// A seen inbox item that has no outcome row: `class`, `reason`, `source`, `author`, `title`,
/// `link`, `seen_at`.
type LegacySeenRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
);

/// Registers an item that was already delivered **before** `action_loop_outcomes` existed, so it
/// has a seen mark but can never own an outcome row through the normal path (and would therefore
/// be invisible to reconciliation forever).
///
/// This is deliberately narrow and explicit — nothing calls it automatically, and it cannot be
/// used as a backfill:
/// - the item must be a real `inbox_decisions` row of class `bot`/`owner` **and** already have an
///   `action_loop_seen` mark (so it was genuinely taken by the loop);
/// - it must have no outcome row yet (otherwise `record_outcome` is the right command);
/// - `--external-id` must pass the same exact provider readback as verification: the provider must
///   return that id and the event's description must carry this item key as a whole token.
///
/// Only then is the row written, as `executed` (never `verified`: the gateway did not observe the
/// action, it found an artifact that already exists and is tied to this item). `lane_event_id` stays
/// NULL, so `requeue_stale_outcomes` — which inner-joins `lane_events` — can never re-emit it as a
/// fresh side effect. The seen mark is left untouched, so the item also stays out of the scan.
pub async fn adopt_legacy_item(
    pool: &SqlitePool,
    item_key: &str,
    external_id: &str,
    detail: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    adopt_legacy_item_with(
        pool,
        item_key,
        external_id,
        detail,
        now,
        &Readback::from_env(),
    )
    .await
}

async fn adopt_legacy_item_with(
    pool: &SqlitePool,
    item_key: &str,
    external_id: &str,
    detail: Option<&str>,
    now: DateTime<Utc>,
    readback: &Readback,
) -> Result<(), String> {
    let key = item_key.trim();
    let external_id = external_id.trim();
    if external_id.is_empty() {
        return Err("--external-id <calendar event id> is required".into());
    }
    let item_id = key.strip_prefix("inbox:").ok_or_else(|| {
        format!("`{key}` is not an inbox item key; only `inbox:<id>` items can be adopted")
    })?;

    // Both the source row and the seen mark, in one read: the mark is the evidence that the loop
    // really did take this item, and it is what keeps it out of later scans.
    let row: Option<LegacySeenRow> = sqlx::query_as(
        "SELECT d.class, d.reason, i.source, COALESCE(i.author, ''), COALESCE(i.title, ''), i.link, s.seen_at
         FROM inbox_decisions d
         JOIN inbox_items i ON i.id = d.item_id
         JOIN action_loop_seen s ON s.item_key = 'inbox:' || d.item_id
         WHERE d.item_id = ? AND d.class IN ('bot', 'owner')",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let Some((class, reason, source, author, title, link, seen_at)) = row else {
        return Err(format!(
            "`{key}` is not a bot/owner inbox item the loop already took: adoption needs both an inbox decision and an `action_loop_seen` mark. Nothing was written."
        ));
    };

    let existing: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT status, external_id FROM action_loop_outcomes WHERE item_key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    if let Some((status, ext)) = existing {
        return Err(format!(
            "`{key}` already has an outcome row ({status}{}); use `omo-gateway autonomy outcome` instead",
            ext.map(|id| format!(", external_id {id}")).unwrap_or_default()
        ));
    }

    // The same exact check verification uses: the artifact must exist and carry this item's key as
    // its own token. Without it there is nothing tying the artifact to the item.
    let proof = read_calendar_event_within(&readback.program, external_id, key, readback.timeout)
        .await
        .map_err(|error| {
            format!("{error} Nothing was written; adoption needs an artifact that already exists and carries `{key}`.")
        })?;

    let note = detail.map(str::trim).filter(|d| !d.is_empty());
    let detail = match note {
        Some(note) => format!("{LEGACY_PREFIX}{proof}; note={note}"),
        None => format!("{LEGACY_PREFIX}{proof}"),
    };
    let author_label = if author.trim().is_empty() {
        "unknown"
    } else {
        author.trim()
    };
    let line = format!(
        "content from {author_label}: {} ({})",
        one_line(&title, 120),
        one_line(&reason, 80)
    );

    // `lane_event_id` is NULL on purpose: this row must never be picked up by the requeue query,
    // which joins `lane_events`, so the item can never be re-emitted as a fresh side effect.
    sqlx::query(
        "INSERT INTO action_loop_outcomes
            (item_key, lane_event_id, kind, source, line, link, status, external_id, detail, attempts, enqueued_at, updated_at)
         VALUES (?, NULL, ?, ?, ?, ?, 'executed', ?, ?, 1, ?, ?)",
    )
    .bind(key)
    .bind(format!("inbox_{class}"))
    .bind(&source)
    .bind(&line)
    .bind(&link)
    .bind(external_id)
    .bind(&detail)
    .bind(seen_at)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!(
        item_key = key,
        external_id,
        readback = %proof,
        seen_at = %seen_at,
        "action_loop legacy item adopted"
    );
    Ok(())
}

/// Upper bound on one provider readback. A hung calendar CLI must not hold the turn or the CLI
/// caller open; the child is killed when the future is dropped.
pub const READBACK_TIMEOUT: Duration = Duration::from_secs(30);

/// The fields the calendar CLI prints for one event (`zele cal get <id>` prints YAML).
#[derive(Debug, serde::Deserialize)]
struct CalendarEventDoc {
    #[serde(default)]
    id: String,
    #[serde(default)]
    description: String,
}

/// True when `text` contains `token` as a whole word: `inbox:1` must not match `inbox:10`.
/// Surrounding punctuation is dropped, so `(inbox:1)` and `inbox:1.` still match, while a longer
/// key that merely starts with the same characters does not.
fn contains_exact_token(text: &str, token: &str) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return false;
    }
    let edge = |ch: char| !(ch.is_alphanumeric() || ch == ':' || ch == '_' || ch == '-');
    text.split_whitespace()
        .map(|word| word.trim_matches(edge))
        .any(|word| word == token)
}

/// Reads the event back from the calendar provider and proves it is *this* item's result: the
/// provider must return the requested event id, and the event's description must contain the item
/// key as a whole token. Both checks run on the parsed YAML fields, never on raw stdout, so a
/// description that merely contains `inbox:10` cannot verify `inbox:1`.
async fn read_calendar_event_within(
    program: &str,
    external_id: &str,
    item_key: &str,
    timeout: Duration,
) -> Result<String, String> {
    let mut command = tokio::process::Command::new(program);
    command
        .arg("cal")
        .arg("get")
        .arg(external_id)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = match tokio::time::timeout(timeout, command.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return Err(format!(
                "`verified` needs a calendar readback, but `{program} cal get` cannot run: {error}"
            ))
        }
        Err(_) => {
            return Err(format!(
                "`{program} cal get {external_id}` did not answer within {:?}; the child was killed and nothing was verified",
                timeout
            ))
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`{program} cal get {external_id}` failed ({}): {}",
            output.status,
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let event: CalendarEventDoc = serde_yaml::from_str(&stdout).map_err(|error| {
        format!("`{program} cal get {external_id}` did not print one event as YAML: {error}")
    })?;
    if event.id.trim() != external_id {
        return Err(format!(
            "`{program} cal get {external_id}` returned `{}`; the calendar does not hold `{external_id}`",
            event.id.trim()
        ));
    }
    if !contains_exact_token(&event.description, item_key) {
        return Err(format!(
            "event `{external_id}` exists but its description does not carry `{item_key}` as its own token: the result cannot be tied to this item. Create the event with `{item_key}` in its description, then verify again."
        ));
    }
    Ok(format!("readback=calendar:{external_id}"))
}

type OutcomeItemRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
    DateTime<Utc>,
);

/// Items whose event reached the lead `REQUEUE_AFTER` ago and still have no outcome: the turn
/// failed or produced nothing, so whether the side effect happened is unknown. Re-queue them below
/// `MAX_ATTEMPTS` as *reconciliation* items and mark the rest failed.
async fn requeue_stale_outcomes(
    pool: &SqlitePool,
    lead_session_key: &str,
    now: DateTime<Utc>,
) -> sqlx::Result<Vec<LoopItem>> {
    let cutoff = now
        - chrono::Duration::from_std(REQUEUE_AFTER).unwrap_or_else(|_| chrono::Duration::zero());
    // The event-wake judge can decline an event (`delivery_reason = 'jev_skip'`): the lead never
    // saw those items, so a skip is not a failed attempt and re-emitting would only be skipped
    // again. Record it as terminal instead of burning the retry budget.
    let skipped = sqlx::query(
        "UPDATE action_loop_outcomes
         SET status = 'failed', detail = ?, updated_at = ?
         WHERE status = 'enqueued'
           AND lane_event_id IN (
               SELECT id FROM lane_events
               WHERE lead_session_key = ? AND delivery_reason = 'jev_skip'
           )",
    )
    .bind("event wake judge skipped the item (jev_skip): the lead turn never saw it")
    .bind(now)
    .bind(lead_session_key)
    .execute(pool)
    .await?;
    if skipped.rows_affected() > 0 {
        tracing::warn!(
            items = skipped.rows_affected(),
            "action_loop items failed: event wake judge skipped them"
        );
    }
    // The first delivery produced no outcome, so the side effect is unknown from here on. Mark the
    // row before re-emitting so the state survives a restart, and so `record_outcome` can refuse a
    // bare "executed" that would hide a duplicate.
    let marked = sqlx::query(
        "UPDATE action_loop_outcomes
         SET detail = ?, updated_at = ?
         WHERE status = 'enqueued'
           AND (detail IS NULL OR detail NOT LIKE 'reconcile:%')
           AND lane_event_id IN (
               SELECT id FROM lane_events
               WHERE lead_session_key = ? AND delivered_at IS NOT NULL
                 AND (delivery_reason IS NULL OR delivery_reason <> 'jev_skip')
                 AND julianday(delivered_at) < julianday(?)
           )",
    )
    .bind(format!(
        "{RECONCILE_PREFIX} delivered with no outcome; side effect unknown, do not repeat"
    ))
    .bind(now)
    .bind(lead_session_key)
    .bind(cutoff)
    .execute(pool)
    .await?;
    if marked.rows_affected() > 0 {
        tracing::warn!(
            items = marked.rows_affected(),
            "action_loop items are unreconciled: their turns ended without an outcome"
        );
    }
    let exhausted = sqlx::query(
        "UPDATE action_loop_outcomes
         SET status = 'failed', detail = ?, updated_at = ?
         WHERE status = 'enqueued' AND attempts >= ?
           AND lane_event_id IN (
               SELECT id FROM lane_events
               WHERE lead_session_key = ? AND delivered_at IS NOT NULL
                 AND (delivery_reason IS NULL OR delivery_reason <> 'jev_skip')
                 AND julianday(delivered_at) < julianday(?)
           )",
    )
    .bind(format!("no outcome after {MAX_ATTEMPTS} attempts"))
    .bind(now)
    .bind(MAX_ATTEMPTS)
    .bind(lead_session_key)
    .bind(cutoff)
    .execute(pool)
    .await?;
    if exhausted.rows_affected() > 0 {
        tracing::warn!(
            items = exhausted.rows_affected(),
            max_attempts = MAX_ATTEMPTS,
            "action_loop items failed: lead turns produced no outcome"
        );
    }
    let rows: Vec<OutcomeItemRow> = sqlx::query_as(
        "SELECT o.item_key, o.kind, o.source, o.line, o.link, o.attempts, o.enqueued_at
         FROM action_loop_outcomes o
         JOIN lane_events e ON e.id = o.lane_event_id
         WHERE o.status = 'enqueued' AND o.attempts < ?
           AND e.lead_session_key = ? AND e.delivered_at IS NOT NULL
           AND (e.delivery_reason IS NULL OR e.delivery_reason <> 'jev_skip')
           AND julianday(e.delivered_at) < julianday(?)
         ORDER BY o.enqueued_at ASC",
    )
    .bind(MAX_ATTEMPTS)
    .bind(lead_session_key)
    .bind(cutoff)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(item_key, kind, source, line, link, attempts, enqueued_at)| LoopItem {
                item_key,
                kind,
                source,
                line,
                link,
                attempts: attempts + 1,
                reconcile: true,
                source_at: Some(enqueued_at),
            },
        )
        .collect())
}

/// Reads a DATETIME column that may hold either an RFC3339 value or SQLite's own text form.
fn parse_stored_datetime(value: &str) -> Option<DateTime<Utc>> {
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ] {
        if let Ok(parsed) = DateTime::parse_from_str(value, fmt) {
            return Some(parsed.with_timezone(&Utc));
        }
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(value, fmt) {
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    None
}

fn kst_day_start_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    let kst = now.with_timezone(&chrono_tz::Asia::Seoul);
    let zero = NaiveTime::from_hms_opt(0, 0, 0).expect("00:00:00");
    chrono_tz::Asia::Seoul
        .from_local_datetime(&kst.date_naive().and_time(zero))
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now)
}

fn one_line(s: &str, max_chars: usize) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        collapsed
    } else {
        let mut out: String = collapsed
            .chars()
            .take(max_chars.saturating_sub(1))
            .collect();
        out.push('…');
        out
    }
}

type InboxCandidateRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
);

/// A GitHub inbox row that has no triage decision, as `scan_unseen_candidates` selects it.
type GithubCandidateRow = (String, String, String, Option<String>, DateTime<Utc>);

/// An undelivered lane event the loop may surface, as `scan_unseen_candidates` selects it.
type LaneEventCandidateRow = (i64, Option<String>, String, String, DateTime<Utc>);

async fn scan_unseen_candidates(
    pool: &SqlitePool,
    lead_session_key: &str,
) -> sqlx::Result<Vec<LoopItem>> {
    let _ =
        crate::inbox::proposals::Proposals::new(pool.clone(), crate::inbox::proposals::OWNER_ID)
            .await;
    let mut out = Vec::new();

    // No age window: an item that was never emitted must survive an arbitrarily long outage, so
    // staleness is marked (`[backlog]`) rather than filtered out. Dedupe is the pair retention and
    // this loop agree on: `storage::retention` keeps a seen mark while its item row exists and
    // never prunes an `enqueued`/`executed` outcome, so a taken item keeps at least one of the two
    // records for as long as it could possibly be re-scanned.
    let inbox_rows: Vec<InboxCandidateRow> = sqlx::query_as(
        "SELECT d.item_id, d.class, d.reason, i.source, COALESCE(i.author, ''), COALESCE(i.title, ''), i.link, d.decided_at
             FROM inbox_decisions d
             JOIN inbox_items i ON i.id = d.item_id
             WHERE d.class IN ('bot', 'owner')
               AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'inbox:' || d.item_id)
               AND NOT EXISTS (SELECT 1 FROM action_loop_outcomes o WHERE o.item_key = 'inbox:' || d.item_id)
               -- A GitHub item taken before triage keeps its `github:` key; triage must not
               -- surface it again under `inbox:` (2026-10-20 13:43Z, ferryx#12 twice).
               AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'github:' || d.item_id)
               AND NOT EXISTS (SELECT 1 FROM action_loop_outcomes o WHERE o.item_key = 'github:' || d.item_id)
               AND NOT EXISTS (SELECT 1 FROM inbox_proposals p WHERE p.item_id = d.item_id)
               AND NOT EXISTS (
                   SELECT 1 FROM lane_events e
                   WHERE e.kind = 'inbox_urgent' AND e.body LIKE 'inbox:' || d.item_id || '%'
               )
             ORDER BY d.decided_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for (item_id, class, reason, source, author, title, link, decided_at) in inbox_rows {
        let author_label = if author.trim().is_empty() {
            "unknown"
        } else {
            author.trim()
        };
        let line = format!(
            "content from {author_label}: {} ({})",
            one_line(&title, 120),
            one_line(&reason, 80)
        );
        out.push(LoopItem {
            item_key: format!("inbox:{item_id}"),
            kind: format!("inbox_{class}"),
            source,
            line,
            link,
            attempts: 1,
            reconcile: false,
            source_at: Some(decided_at),
        });
    }

    let gh_rows: Vec<GithubCandidateRow> = sqlx::query_as(
        "SELECT i.id, i.author, i.title, i.link, i.received_at
         FROM inbox_items i
         WHERE i.source = 'github'
           AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'github:' || i.id)
           AND NOT EXISTS (SELECT 1 FROM action_loop_outcomes o WHERE o.item_key = 'github:' || i.id)
           AND NOT EXISTS (
               SELECT 1 FROM inbox_decisions d
               WHERE d.item_id = i.id AND d.class IN ('bot', 'owner')
           )
         ORDER BY i.received_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for (id, author, title, link, received_at) in gh_rows {
        let line = if author.trim().is_empty() {
            one_line(&title, 160)
        } else {
            format!("content from {}: {}", author.trim(), one_line(&title, 140))
        };
        out.push(LoopItem {
            item_key: format!("github:{id}"),
            kind: "github".into(),
            source: "github".into(),
            line,
            link,
            attempts: 1,
            reconcile: false,
            source_at: Some(received_at),
        });
    }

    // `delivered_at IS NULL` is itself a durable dedupe: the event row outlives the seen mark, so
    // a delivered event is never a candidate again even after retention prunes `action_loop_seen`.
    let event_rows: Vec<LaneEventCandidateRow> = sqlx::query_as(
        "SELECT e.id, e.lane_key, e.kind, e.body, e.created_at
         FROM lane_events e
         WHERE e.delivered_at IS NULL
           AND (e.lead_session_key = ? OR e.lead_session_key IS NULL OR e.lead_session_key = '')
           AND e.kind IN ('blocked', 'not_done', 'dead', 'turn_cap', 'nudged', 'precedent_candidate', 'source_health')
           AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'lane_event:' || e.id)
         ORDER BY e.id ASC",
    )
    .bind(lead_session_key)
    .fetch_all(pool)
    .await?;

    for (id, lane_key, kind, body, created_at) in event_rows {
        let src = lane_key.unwrap_or_else(|| kind.clone());
        out.push(LoopItem {
            item_key: format!("lane_event:{id}"),
            kind,
            source: src,
            line: one_line(&body, 180),
            link: None,
            attempts: 1,
            reconcile: false,
            source_at: Some(created_at),
        });
    }

    let health_rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT source, state, detail, since
         FROM source_health
         WHERE NOT EXISTS (
             SELECT 1 FROM action_loop_seen s
             WHERE s.item_key = 'source_health:' || source_health.source || ':' || source_health.since
         )",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (source, state, detail, since) in health_rows {
        let since_at = parse_stored_datetime(&since);
        out.push(LoopItem {
            item_key: format!("source_health:{source}:{since}"),
            kind: "source_health".into(),
            source,
            line: format!("{state}: {}", one_line(&detail, 140)),
            link: None,
            attempts: 1,
            reconcile: false,
            source_at: since_at,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAD: &str = "slack:T1:C1::U1";

    fn daytime_utc() -> DateTime<Utc> {
        chrono_tz::Asia::Seoul
            .with_ymd_and_hms(2026, 10, 18, 14, 0, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    async fn insert_item(
        pool: &SqlitePool,
        id: &str,
        source: &str,
        class: Option<&str>,
        at: DateTime<Utc>,
    ) {
        sqlx::query(
            "INSERT INTO inbox_items (id, source, source_ref, author, title, snippet, received_at, raw_hash, collected_at)
             VALUES (?, ?, ?, 'Alice', '다음주 화요일 3시 미팅 잡아줘', 'body', ?, 'hash', ?)",
        )
        .bind(id)
        .bind(source)
        .bind(id)
        .bind(at)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
        if let Some(class) = class {
            sqlx::query(
                "INSERT INTO inbox_decisions (item_id, class, reason, status, decided_at, model)
                 VALUES (?, ?, 'schedule request', 'open', ?, 'jev')",
            )
            .bind(id)
            .bind(class)
            .bind(at)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    async fn outcome(
        pool: &SqlitePool,
        key: &str,
    ) -> (String, i64, Option<String>, Option<String>) {
        sqlx::query_as(
            "SELECT status, attempts, external_id, detail FROM action_loop_outcomes WHERE item_key = ?",
        )
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn action_loop_events(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'action_loop'")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn body_of(pool: &SqlitePool, event_id: i64) -> String {
        sqlx::query_scalar("SELECT body FROM lane_events WHERE id = ?")
            .bind(event_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// Writes one stub script under a name derived from its content, so two stubs in the same
    /// directory can never overwrite each other.
    fn write_stub(dir: &std::path::Path, tag: &str, script: &str) -> String {
        use std::os::unix::fs::PermissionsExt;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(script, &mut hasher);
        let safe: String = tag
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        let path = dir.join(format!(
            "stub-cal-{safe}-{:x}",
            std::hash::Hasher::finish(&hasher)
        ));
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// A stand-in for the calendar CLI. `event_body` is what the provider returns for the event:
    /// `None` means the event does not exist there (the CLI exits non-zero, as `zele` does).
    fn stub_calendar(dir: &std::path::Path, event_id: &str, event_body: Option<&str>) -> String {
        let script = match event_body {
            Some(body) => format!(
                "#!/bin/sh\nif [ \"$1 $2\" = \"cal get\" ] && [ \"$3\" = \"{event_id}\" ]; then\n  printf '%s\\n' '{body}'\n  exit 0\nfi\necho \"event \\\"$3\\\" not found\" >&2\nexit 1\n"
            ),
            None => "#!/bin/sh\necho \"event \\\"$3\\\" not found\" >&2\nexit 1\n".to_string(),
        };
        write_stub(dir, event_id, &script)
    }

    /// A provider that answers *any* requested id with `event_body` and exits 0. Needed to reach
    /// the branch where the provider succeeds but returns a different event than the one asked
    /// for; `stub_calendar` exits non-zero instead, which is a different (also valid) refusal.
    fn stub_calendar_any(dir: &std::path::Path, event_body: &str) -> String {
        let script = format!(
            "#!/bin/sh\nif [ \"$1 $2\" = \"cal get\" ]; then\n  printf '%s\\n' '{event_body}'\n  exit 0\nfi\nexit 1\n"
        );
        write_stub(dir, "any", &script)
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("omo-action-loop-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn deliver(pool: &SqlitePool, event_id: i64, at: DateTime<Utc>, reason: &str) {
        sqlx::query("UPDATE lane_events SET delivered_at = ?, delivery_reason = ? WHERE id = ?")
            .bind(at)
            .bind(reason)
            .bind(event_id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn item_is_done_only_when_the_calendar_reads_the_event_back_for_it() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "kakao-1", "kakao", Some("bot"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        let event_id = worker
            .flush_coalesced(now)
            .await
            .unwrap()
            .expect("kakao item must reach the action loop");

        assert_eq!(outcome(&pool, "inbox:kakao-1").await.0, "enqueued");
        let body = body_of(&pool, event_id).await;
        assert!(body.contains("inbox:kakao-1 [inbox_bot:kakao]"), "{body}");
        assert!(
            body.contains("put `<key>` in the event description"),
            "the instruction must say how the result is tied to the item: {body}"
        );

        record_outcome(
            &pool,
            "inbox:kakao-1",
            OutcomeReport::new("executed")
                .external_id(Some("gcal-evt-42"))
                .detail(Some("created the event")),
            now,
        )
        .await
        .unwrap();
        let (status, _, ext, detail) = outcome(&pool, "inbox:kakao-1").await;
        assert_eq!(status, "executed", "a claim by the agent is not done");
        assert_eq!(ext.as_deref(), Some("gcal-evt-42"));
        assert_eq!(detail.as_deref(), Some("created the event"));

        let dir = temp_dir("readback");
        // The calendar does not hold the event: the agent's id was a claim.
        let missing = stub_calendar(&dir, "gcal-evt-42", None);
        let error = record_outcome_with(
            &pool,
            "inbox:kakao-1",
            OutcomeReport::new("verified").external_id(Some("gcal-evt-42")),
            now,
            &Readback::new(missing.as_str()),
        )
        .await
        .expect_err("the calendar must hold the event");
        assert!(
            error.contains("not found"),
            "a provider that does not have the event must refuse it: {error}"
        );
        assert_eq!(outcome(&pool, "inbox:kakao-1").await.0, "executed");

        // The event exists but belongs to something else.
        let unrelated = stub_calendar(
            &dir,
            "gcal-evt-42",
            Some("id: gcal-evt-42\nsummary: 다른 미팅\nstart: Oct 21, 3:00 PM"),
        );
        let error = record_outcome_with(
            &pool,
            "inbox:kakao-1",
            OutcomeReport::new("verified").external_id(Some("gcal-evt-42")),
            now,
            &Readback::new(unrelated.as_str()),
        )
        .await
        .expect_err("an event without the item key is not this item's result");
        assert!(error.contains("cannot be tied to this item"), "{error}");
        assert_eq!(outcome(&pool, "inbox:kakao-1").await.0, "executed");

        // The event exists and carries the item key: the provider confirms it.
        let tied = stub_calendar(
            &dir,
            "gcal-evt-42",
            Some("id: gcal-evt-42\nsummary: C 미팅\nstart: Oct 21, 3:00 PM\ndescription: inbox:kakao-1"),
        );
        record_outcome_with(
            &pool,
            "inbox:kakao-1",
            OutcomeReport::new("verified").external_id(Some("gcal-evt-42")),
            now,
            &Readback::new(tied.as_str()),
        )
        .await
        .unwrap();
        let (status, _, ext, detail) = outcome(&pool, "inbox:kakao-1").await;
        assert_eq!(status, "verified");
        assert_eq!(ext.as_deref(), Some("gcal-evt-42"));
        assert!(
            detail
                .unwrap_or_default()
                .contains("readback=calendar:gcal-evt-42"),
            "the row must record which readback confirmed it"
        );
        assert_eq!(
            outcome_counts(&pool).await.unwrap(),
            vec![("verified".to_string(), 1)]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn item_key_matches_only_as_a_whole_token() {
        assert!(contains_exact_token("inbox:1", "inbox:1"));
        assert!(contains_exact_token("미팅 예약 inbox:1", "inbox:1"));
        assert!(contains_exact_token("(inbox:1)", "inbox:1"));
        assert!(contains_exact_token("inbox:1.", "inbox:1"));
        assert!(!contains_exact_token("inbox:10", "inbox:1"));
        assert!(!contains_exact_token("inbox:10 예약", "inbox:1"));
        assert!(!contains_exact_token("x-inbox:1", "inbox:1"));
        assert!(!contains_exact_token("", "inbox:1"));
        assert!(!contains_exact_token("inbox:1", ""));
    }

    #[tokio::test]
    async fn readback_matches_the_parsed_id_and_the_whole_item_key() {
        let dir = temp_dir("token");
        // The description carries a longer key that starts with the same characters.
        let decoy = stub_calendar(
            &dir,
            "evt-1",
            Some("id: evt-1\ndescription: inbox:10 and inbox:100"),
        );
        let error = read_calendar_event_within(&decoy, "evt-1", "inbox:1", READBACK_TIMEOUT)
            .await
            .expect_err("inbox:10 must not verify inbox:1");
        assert!(error.contains("does not carry"), "{error}");

        // The provider succeeds but answers with a different event than the one asked for.
        let wrong_id = stub_calendar_any(&dir, "id: evt-2\ndescription: inbox:1");
        let error = read_calendar_event_within(&wrong_id, "evt-1", "inbox:1", READBACK_TIMEOUT)
            .await
            .expect_err("the provider must return the requested id");
        assert!(error.contains("does not hold"), "{error}");

        // The provider reports the event does not exist: refused, and the message says so.
        let absent = stub_calendar(&dir, "evt-9", None);
        let error = read_calendar_event_within(&absent, "evt-9", "inbox:1", READBACK_TIMEOUT)
            .await
            .expect_err("an event the provider does not have is not verified");
        assert!(error.contains("not found"), "{error}");

        // Not YAML at all: refused rather than substring-matched.
        let garbage = stub_calendar(&dir, "evt-3", Some("event evt-3 has inbox:1"));
        let error = read_calendar_event_within(&garbage, "evt-3", "inbox:1", READBACK_TIMEOUT)
            .await
            .expect_err("unparseable provider output proves nothing");
        assert!(error.contains("as YAML"), "{error}");

        let good = stub_calendar(&dir, "evt-4", Some("id: evt-4\ndescription: 미팅 inbox:1"));
        assert_eq!(
            read_calendar_event_within(&good, "evt-4", "inbox:1", READBACK_TIMEOUT)
                .await
                .unwrap(),
            "readback=calendar:evt-4"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn readback_is_bounded_and_kills_a_hung_provider() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("hang");
        let path = dir.join("hang-cal");
        std::fs::write(&path, "#!/bin/sh\nexec sleep 300\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Goes through the real record path with a short bound, so the seam is what is exercised:
        // a hung provider must be given up on and the child killed with it.
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "hang-1", "kakao", Some("owner"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        worker.flush_coalesced(now).await.unwrap().unwrap();

        let bound = std::time::Duration::from_millis(300);
        let started = std::time::Instant::now();
        let error = record_outcome_with(
            &pool,
            "inbox:hang-1",
            OutcomeReport::new("verified").external_id(Some("evt-1")),
            now,
            &Readback::new(path.to_string_lossy()).with_timeout(bound),
        )
        .await
        .expect_err("a hung provider verifies nothing");
        assert!(error.contains("did not answer within"), "{error}");
        assert!(
            started.elapsed() < bound + std::time::Duration::from_secs(5),
            "the readback must be bounded by its timeout, not by the child"
        );
        assert_eq!(
            outcome(&pool, "inbox:hang-1").await.0,
            "enqueued",
            "a timed-out readback must not change the row"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn verified_without_a_calendar_readback_is_refused() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "gh-1", "github", Some("bot"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        worker.flush_coalesced(now).await.unwrap().unwrap();

        let dir = temp_dir("noreadback");
        let absent = dir.join("no-such-cal").to_string_lossy().into_owned();
        let error = record_outcome_with(
            &pool,
            "inbox:gh-1",
            OutcomeReport::new("verified").external_id(Some("1234")),
            now,
            &Readback::new(absent.as_str()),
        )
        .await
        .expect_err("without a working provider there is no verification");
        assert!(error.contains("cannot run"), "{error}");
        assert_eq!(outcome(&pool, "inbox:gh-1").await.0, "enqueued");

        let error = record_outcome_with(
            &pool,
            "inbox:gh-1",
            OutcomeReport::new("verified"),
            now,
            &Readback::new(absent.as_str()),
        )
        .await
        .expect_err("verified needs the event id to read back");
        assert!(error.contains("--external-id"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A pre-0054 delivery: the loop took the item (seen mark) but no outcome row could exist.
    async fn insert_legacy_seen_item(
        pool: &SqlitePool,
        id: &str,
        seen_at: DateTime<Utc>,
    ) -> String {
        insert_item(pool, id, "kakao", Some("owner"), seen_at).await;
        sqlx::query(
            "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
             ON CONFLICT(item_key) DO NOTHING",
        )
        .bind(format!("inbox:{id}"))
        .bind(seen_at)
        .execute(pool)
        .await
        .unwrap();
        format!("inbox:{id}")
    }

    #[tokio::test]
    async fn legacy_seen_item_is_adopted_as_executed_after_readback() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        let key =
            insert_legacy_seen_item(&pool, "legacy-1", now - chrono::Duration::hours(2)).await;

        let dir = temp_dir("legacy");
        let tied = stub_calendar(
            &dir,
            "evt-legacy",
            Some("id: evt-legacy\ndescription: 미팅 inbox:legacy-1"),
        );
        adopt_legacy_item_with(
            &pool,
            &key,
            "evt-legacy",
            Some("found the event the lead created before the upgrade"),
            now,
            &Readback::new(tied.as_str()),
        )
        .await
        .unwrap();

        let (status, attempts, ext, detail) = outcome(&pool, &key).await;
        assert_eq!(status, "executed", "adoption never claims verified");
        assert_eq!(ext.as_deref(), Some("evt-legacy"));
        assert_eq!(attempts, 1);
        let detail = detail.unwrap_or_default();
        assert!(detail.starts_with(LEGACY_PREFIX), "{detail}");
        assert!(detail.contains("readback=calendar:evt-legacy"), "{detail}");
        assert!(
            detail.contains("note=found the event the lead created before the upgrade"),
            "{detail}"
        );

        // Provenance is honest: no lane event is invented and the seen mark is kept.
        let lane_event: Option<i64> =
            sqlx::query_scalar("SELECT lane_event_id FROM action_loop_outcomes WHERE item_key = ?")
                .bind(&key)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            lane_event.is_none(),
            "adoption must not attach a lane event"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM lane_events")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "adoption writes no lane event"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM action_loop_seen WHERE item_key = ?"
            )
            .bind(&key)
            .fetch_one(&pool)
            .await
            .unwrap(),
            1,
            "the seen mark must survive, so the item is never re-emitted"
        );

        // And it is not re-emitted or re-queued afterwards.
        let worker = ActionLoop::new(pool.clone(), LEAD);
        assert_eq!(worker.collect_new_items(now).await.unwrap(), 0);
        assert_eq!(worker.flush_coalesced(now).await.unwrap(), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn adoption_refuses_unseen_unknown_and_wrong_key_items() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        let dir = temp_dir("legacy-refuse");
        let tied = stub_calendar(
            &dir,
            "evt-x",
            Some("id: evt-x\ndescription: 미팅 inbox:legacy-ok"),
        );
        let readback = Readback::new(tied.as_str());

        // Never taken by the loop: no seen mark.
        insert_item(&pool, "never-seen", "kakao", Some("owner"), now).await;
        let error =
            adopt_legacy_item_with(&pool, "inbox:never-seen", "evt-x", None, now, &readback)
                .await
                .expect_err("an item the loop never took cannot be adopted");
        assert!(error.contains("already took"), "{error}");

        // Seen but not an inbox item the loop can vouch for.
        sqlx::query(
            "INSERT INTO action_loop_seen (item_key, seen_at) VALUES ('github:gh-unknown', ?)",
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
        let error =
            adopt_legacy_item_with(&pool, "github:gh-unknown", "evt-x", None, now, &readback)
                .await
                .expect_err("only inbox items have the row evidence adoption needs");
        assert!(error.contains("not an inbox item key"), "{error}");

        let error =
            adopt_legacy_item_with(&pool, "inbox:no-such-row", "evt-x", None, now, &readback)
                .await
                .expect_err("an unknown item is not adoptable");
        assert!(error.contains("already took"), "{error}");

        // Seen, but the artifact does not carry this item's key.
        let other = insert_legacy_seen_item(&pool, "legacy-other", now).await;
        let decoy = stub_calendar(
            &dir,
            "evt-y",
            Some("id: evt-y\ndescription: 미팅 inbox:legacy-other-suffix"),
        );
        let error = adopt_legacy_item_with(
            &pool,
            &other,
            "evt-y",
            None,
            now,
            &Readback::new(decoy.as_str()),
        )
        .await
        .expect_err("a description with a longer key must not adopt");
        assert!(error.contains("does not carry"), "{error}");

        // Seen and correct key, but the provider does not have the event.
        let missing = stub_calendar(&dir, "evt-z", None);
        let error = adopt_legacy_item_with(
            &pool,
            &other,
            "evt-z",
            None,
            now,
            &Readback::new(missing.as_str()),
        )
        .await
        .expect_err("a nonexistent artifact cannot be adopted");
        assert!(error.contains("not found"), "{error}");

        // Nothing was written by any refusal.
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM action_loop_outcomes")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "every refusal must leave the table empty"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn adoption_is_explicit_and_never_backfills_the_seen_backlog() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        for i in 1..=3 {
            insert_legacy_seen_item(&pool, &format!("legacy-{i}"), now).await;
        }

        // Scanning and flushing must not adopt anything on their own.
        let worker = ActionLoop::new(pool.clone(), LEAD);
        worker.collect_new_items(now).await.unwrap();
        worker.flush_coalesced(now).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM action_loop_outcomes")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "there is no automatic backfill of pre-existing seen items"
        );

        // Adopting one leaves the other two untouched: it is one explicit item per call.
        let dir = temp_dir("legacy-one");
        let tied = stub_calendar(
            &dir,
            "evt-one",
            Some("id: evt-one\ndescription: 미팅 inbox:legacy-1"),
        );
        adopt_legacy_item_with(
            &pool,
            "inbox:legacy-1",
            "evt-one",
            None,
            now,
            &Readback::new(tied.as_str()),
        )
        .await
        .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM action_loop_outcomes")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1,
            "only the named item is adopted"
        );

        // A second call for the same item is refused: `outcome` owns it from now on.
        let error = adopt_legacy_item_with(
            &pool,
            "inbox:legacy-1",
            "evt-one",
            None,
            now,
            &Readback::new(tied.as_str()),
        )
        .await
        .expect_err("re-adopting an item that now has a row is refused");
        assert!(error.contains("already has an outcome row"), "{error}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn verified_row_is_terminal_against_a_delayed_report() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "cal-2", "kakao", Some("owner"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        worker.flush_coalesced(now).await.unwrap().unwrap();

        let dir = temp_dir("terminal");
        let tied = stub_calendar(
            &dir,
            "evt-9",
            Some("id: evt-9\ndescription: 미팅 inbox:cal-2"),
        );
        record_outcome_with(
            &pool,
            "inbox:cal-2",
            OutcomeReport::new("verified").external_id(Some("evt-9")),
            now,
            &Readback::new(tied.as_str()),
        )
        .await
        .unwrap();
        assert_eq!(outcome(&pool, "inbox:cal-2").await.0, "verified");

        // An earlier attempt's turn finishes late and reports its own outcome. It must not be able
        // to erase the result the provider already confirmed.
        let later = now + chrono::Duration::minutes(5);
        for status in ["executed", "failed"] {
            let error = record_outcome(
                &pool,
                "inbox:cal-2",
                OutcomeReport::new(status)
                    .external_id(Some("evt-other"))
                    .detail(Some("late report from an earlier attempt")),
                later,
            )
            .await
            .expect_err("a verified row must not be overwritten");
            assert!(error.contains("already verified"), "{status}: {error}");
        }
        let (status, _, ext, detail) = outcome(&pool, "inbox:cal-2").await;
        assert_eq!(status, "verified", "the confirmed status survives");
        assert_eq!(
            ext.as_deref(),
            Some("evt-9"),
            "the confirmed external id survives"
        );
        assert!(
            detail
                .unwrap_or_default()
                .contains("readback=calendar:evt-9"),
            "the readback provenance survives"
        );

        // Re-verifying the same item is allowed: it is still a readback, not a blind write.
        record_outcome_with(
            &pool,
            "inbox:cal-2",
            OutcomeReport::new("verified").external_id(Some("evt-9")),
            later,
            &Readback::new(tied.as_str()),
        )
        .await
        .unwrap();
        assert_eq!(outcome(&pool, "inbox:cal-2").await.0, "verified");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn unreconciled_item_cannot_be_closed_by_a_bare_executed() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "cal-1", "kakao", Some("owner"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        let first = worker.flush_coalesced(now).await.unwrap().unwrap();
        // The lead turn took the event and ended without recording anything.
        deliver(&pool, first, now, "lead_turn").await;

        let later = now + chrono::Duration::minutes(31);
        assert_eq!(worker.collect_new_items(later).await.unwrap(), 1);
        let (status, attempts, _, detail) = outcome(&pool, "inbox:cal-1").await;
        assert_eq!(status, "enqueued");
        assert_eq!(attempts, 1);
        assert!(
            detail.unwrap_or_default().starts_with(RECONCILE_PREFIX),
            "the unknown side effect must be recorded as reconciliation state"
        );

        let reconcile_event = worker.flush_coalesced(later).await.unwrap().unwrap();
        let body = body_of(&pool, reconcile_event).await;
        assert!(body.starts_with("[ACTION_LOOP_RECONCILE]"), "{body}");
        assert!(body.contains("do NOT repeat the action"), "{body}");
        assert!(
            body.contains("[reconcile attempt 2/3] inbox:cal-1"),
            "a re-emission is a reconciliation, not a retry of the action: {body}"
        );

        let error = record_outcome(&pool, "inbox:cal-1", OutcomeReport::new("executed"), later)
            .await
            .expect_err("a bare executed would hide a possible duplicate");
        assert!(error.contains("unreconciled"), "{error}");
        let error = record_outcome(&pool, "inbox:cal-1", OutcomeReport::new("failed"), later)
            .await
            .expect_err("closing an unreconciled item needs what was checked");
        assert!(error.contains("--detail"), "{error}");

        record_outcome(
            &pool,
            "inbox:cal-1",
            OutcomeReport::new("executed").external_id(Some("gcal-evt-7")),
            later,
        )
        .await
        .unwrap();
        let (status, _, ext, detail) = outcome(&pool, "inbox:cal-1").await;
        assert_eq!(status, "executed");
        assert_eq!(ext.as_deref(), Some("gcal-evt-7"));
        assert!(
            detail.unwrap_or_default().starts_with(RECONCILED_PREFIX),
            "the row must record that reconciliation happened"
        );
        // Reconciled items are not re-emitted again.
        assert_eq!(
            worker
                .collect_new_items(later + chrono::Duration::hours(2))
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn item_without_outcome_is_requeued_as_reconcile_then_failed() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let start = daytime_utc();
        insert_item(&pool, "lead-1", "kakao", Some("owner"), start).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        let mut now = start;
        for attempt in 1..=MAX_ATTEMPTS {
            let event_id = worker
                .flush_coalesced(now)
                .await
                .unwrap()
                .unwrap_or_else(|| panic!("attempt {attempt} must emit"));
            assert_eq!(outcome(&pool, "inbox:lead-1").await.1, attempt);
            let body = body_of(&pool, event_id).await;
            if attempt == 1 {
                assert!(body.starts_with("[ACTION_LOOP] "), "{body}");
            } else {
                assert!(body.starts_with("[ACTION_LOOP_RECONCILE] "), "{body}");
                assert!(
                    body.contains(&format!(
                        "[reconcile attempt {}/{}] inbox:lead-1",
                        attempt, MAX_ATTEMPTS
                    )),
                    "{body}"
                );
            }
            deliver(&pool, event_id, now, "lead_turn").await;
            now += chrono::Duration::hours(1);
        }
        assert_eq!(
            worker.collect_new_items(now).await.unwrap(),
            0,
            "the retry count is bounded"
        );
        assert_eq!(outcome(&pool, "inbox:lead-1").await.0, "failed");
    }

    #[tokio::test]
    async fn judge_skipped_event_does_not_burn_attempts() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "skip-1", "kakao", Some("bot"), now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        let event_id = worker.flush_coalesced(now).await.unwrap().unwrap();
        deliver(&pool, event_id, now, "jev_skip").await;

        assert_eq!(
            worker
                .collect_new_items(now + chrono::Duration::hours(1))
                .await
                .unwrap(),
            0,
            "a judge skip must not be re-emitted"
        );
        let (status, attempts, _, detail) = outcome(&pool, "inbox:skip-1").await;
        assert_eq!(status, "failed");
        assert_eq!(attempts, 1, "a judge skip is not an attempt by the lead");
        assert!(
            detail.unwrap_or_default().contains("never saw it"),
            "the reason must be recorded"
        );
        assert_eq!(action_loop_events(&pool).await, 1);
    }

    #[tokio::test]
    async fn item_from_a_long_outage_is_emitted_as_backlog_not_dropped() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        let ten_days_ago = now - chrono::Duration::days(10);
        insert_item(&pool, "outage-1", "kakao", Some("owner"), ten_days_ago).await;

        let worker = ActionLoop::new(pool.clone(), LEAD);
        let event_id = worker
            .flush_coalesced(now)
            .await
            .unwrap()
            .expect("work that arrived during an outage must not be dropped");
        let body = body_of(&pool, event_id).await;
        assert!(body.contains("[backlog] inbox:outage-1"), "{body}");
        assert_eq!(outcome(&pool, "inbox:outage-1").await.0, "enqueued");
    }

    #[tokio::test]
    async fn a_lost_seen_mark_does_not_resurrect_a_taken_item() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "kept-1", "kakao", Some("owner"), now).await;
        insert_item(&pool, "kept-gh", "github", None, now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        worker.flush_coalesced(now).await.unwrap().unwrap();
        assert_eq!(action_loop_events(&pool).await, 1);

        // `storage::retention` keeps a mark while its item row exists, so this is the worst case,
        // not the normal one: both marks gone. The outcome rows must still hold the dedupe, since
        // retention never prunes an `enqueued` or `executed` outcome.
        sqlx::query("DELETE FROM action_loop_seen")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(worker.collect_new_items(now).await.unwrap(), 0);
        assert_eq!(worker.flush_coalesced(now).await.unwrap(), None);
        assert_eq!(action_loop_events(&pool).await, 1);
    }

    #[tokio::test]
    async fn another_lead_does_not_take_over_an_emitted_item() {
        const OTHER: &str = "slack:T2:C2::U2";
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        sqlx::query(
            "INSERT INTO source_health (source, state, detail, since) VALUES ('mail', 'auth_failed', 'token expired', ?)",
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        let owner = ActionLoop::new(pool.clone(), LEAD);
        let event_id = owner.flush_coalesced(now).await.unwrap().unwrap();
        // The seen mark can be pruned; the outcome row still owns the item.
        sqlx::query("DELETE FROM action_loop_seen")
            .execute(&pool)
            .await
            .unwrap();

        let other = ActionLoop::new(pool.clone(), OTHER);
        assert_eq!(
            other.flush_coalesced(now).await.unwrap(),
            None,
            "another lead must not re-emit an item that already has an event"
        );
        let owner_event: i64 = sqlx::query_scalar(
            "SELECT lane_event_id FROM action_loop_outcomes WHERE item_key LIKE 'source_health:mail:%'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(owner_event, event_id);
        assert_eq!(action_loop_events(&pool).await, 1);
        assert!(
            outcome_counts_for_lead(&pool, OTHER)
                .await
                .unwrap()
                .is_empty(),
            "the other lead's counts must not include this item"
        );
        assert_eq!(
            outcome_counts_for_lead(&pool, LEAD).await.unwrap(),
            vec![("enqueued".to_string(), 1)]
        );
    }

    #[tokio::test]
    async fn triaged_github_item_appears_once_per_batch() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "gh-1", "github", Some("bot"), now).await;
        insert_item(&pool, "gh-2", "github", None, now).await;
        let items = scan_unseen_candidates(&pool, LEAD).await.unwrap();
        let keys: Vec<&str> = items.iter().map(|i| i.item_key.as_str()).collect();
        assert_eq!(keys, vec!["inbox:gh-1", "github:gh-2"]);
    }

    #[tokio::test]
    async fn github_item_taken_before_triage_is_not_taken_again_after_triage() {
        // given: an untriaged GitHub item emitted under its github: key
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let now = daytime_utc();
        insert_item(&pool, "gh-3", "github", None, now).await;
        let worker = ActionLoop::new(pool.clone(), LEAD);
        assert!(worker.flush_coalesced(now).await.unwrap().is_some());

        // when: triage later classifies the same item
        sqlx::query(
            "INSERT INTO inbox_decisions (item_id, class, reason, status, decided_at, model)
             VALUES ('gh-3', 'owner', 'review', 'open', ?, 'jev')",
        )
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // then: it is not surfaced a second time under inbox:
        let keys: Vec<String> = scan_unseen_candidates(&pool, LEAD)
            .await
            .unwrap()
            .into_iter()
            .map(|item| item.item_key)
            .collect();
        assert!(
            keys.is_empty(),
            "already-taken GitHub item resurfaced: {keys:?}"
        );
    }
}

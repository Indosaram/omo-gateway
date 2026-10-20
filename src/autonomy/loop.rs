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
}

impl LoopItem {
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
    pub async fn collect_new_items(&self, _now: DateTime<Utc>) -> sqlx::Result<usize> {
        let candidates = scan_unseen_candidates(&self.pool, &self.lead_session_key).await?;
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
        let items: Vec<LoopItem> = {
            let mut batch = self.pending_batch.lock().await;
            if batch.is_empty() {
                return Ok(None);
            }
            std::mem::take(&mut *batch)
        };

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

        let mut formatted: Vec<String> = items
            .iter()
            .take(MAX_LISTED_ITEMS)
            .map(LoopItem::format_entry)
            .collect();
        if items.len() > MAX_LISTED_ITEMS {
            formatted.push(format!("+{} more", items.len() - MAX_LISTED_ITEMS));
        }
        let body = format!(
            "[ACTION_LOOP] {} items: {}",
            items.len(),
            formatted.join(" | ")
        );
        let release_at: Option<DateTime<Utc>> = if in_quiet_hours_kst(now) {
            Some(next_0800_kst(now))
        } else {
            None
        };

        // The event and the seen marks commit together: an item is either emitted and seen, or
        // neither (and the next scan picks it up again).
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
        for item in &items {
            sqlx::query(
                "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
                 ON CONFLICT(item_key) DO NOTHING",
            )
            .bind(&item.item_key)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        tracing::info!("action_loop delivered items={}", items.len());
        Ok(Some(res.last_insert_rowid()))
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
);

async fn scan_unseen_candidates(
    pool: &SqlitePool,
    lead_session_key: &str,
) -> sqlx::Result<Vec<LoopItem>> {
    let _ =
        crate::inbox::proposals::Proposals::new(pool.clone(), crate::inbox::proposals::OWNER_ID)
            .await;
    let mut out = Vec::new();

    let inbox_rows: Vec<InboxCandidateRow> = sqlx::query_as(
        "SELECT d.item_id, d.class, d.reason, i.source, COALESCE(i.author, ''), COALESCE(i.title, ''), i.link
             FROM inbox_decisions d
             JOIN inbox_items i ON i.id = d.item_id
             WHERE d.class IN ('bot', 'owner')
               AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'inbox:' || d.item_id)
               AND NOT EXISTS (SELECT 1 FROM inbox_proposals p WHERE p.item_id = d.item_id)
               AND NOT EXISTS (
                   SELECT 1 FROM lane_events e
                   WHERE e.kind = 'inbox_urgent' AND e.body LIKE 'inbox:' || d.item_id || '%'
               )
             ORDER BY d.decided_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for (item_id, class, reason, source, author, title, link) in inbox_rows {
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
        });
    }

    let gh_rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT i.id, i.author, i.title, i.link
         FROM inbox_items i
         WHERE i.source = 'github'
           AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'github:' || i.id)
         ORDER BY i.received_at ASC",
    )
    .fetch_all(pool)
    .await?;

    for (id, author, title, link) in gh_rows {
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
        });
    }

    let event_rows: Vec<(i64, Option<String>, String, String)> = sqlx::query_as(
        "SELECT e.id, e.lane_key, e.kind, e.body
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

    for (id, lane_key, kind, body) in event_rows {
        let src = lane_key.unwrap_or_else(|| kind.clone());
        out.push(LoopItem {
            item_key: format!("lane_event:{id}"),
            kind,
            source: src,
            line: one_line(&body, 180),
            link: None,
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
        out.push(LoopItem {
            item_key: format!("source_health:{source}:{since}"),
            kind: "source_health".into(),
            source,
            line: format!("{state}: {}", one_line(&detail, 140)),
            link: None,
        });
    }

    Ok(out)
}

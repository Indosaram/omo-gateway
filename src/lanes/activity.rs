//! Persistent activity decisions. The runner owns turn scheduling and post delivery.
use std::sync::Arc;

use chrono::Duration;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::scrub::scrub;
use crate::judge::{Judge, Purpose};
use crate::storage::{get_lane, Lane};
use crate::work_heartbeat::Clock;
use crate::{OmonError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Blocker,
    Question,
    Done,
    Milestone,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Report<'a> {
    pub kind: ReportKind,
    pub text: &'a str,
    pub line: &'a str,
}

#[derive(Default)]
pub struct StreamCapture {
    sequence: Option<u64>,
    pub content: String,
    finalized: bool,
}

impl StreamCapture {
    /// One instance per stream: content is a snapshot, never an append-only delta.
    pub fn accept(&mut self, chunk: &crate::StreamChunk) -> Option<&str> {
        if self.finalized
            || self
                .sequence
                .is_some_and(|sequence| chunk.sequence <= sequence)
        {
            return None;
        }
        self.sequence = Some(chunk.sequence);
        self.content.clone_from(&chunk.content);
        self.finalized = chunk.is_final;
        chunk.is_final.then_some(self.content.as_str())
    }
}

/// Only complete, matching-key report lines are actionable; pipes in text survive.
pub fn reports<'a>(key: &'a str, message: &'a str) -> impl Iterator<Item = Report<'a>> {
    message.lines().filter_map(move |line| {
        let line = line.trim();
        let mut parts = line.strip_prefix("[REPORT] ")?.splitn(3, '|');
        if parts.next()?.trim() != key {
            return None;
        }
        let kind = match parts.next()?.trim() {
            "blocker" => ReportKind::Blocker,
            "question" => ReportKind::Question,
            "done" => ReportKind::Done,
            "milestone" => ReportKind::Milestone,
            _ => return None,
        };
        let text = parts.next()?.trim();
        (!text.is_empty()).then_some(Report { kind, text, line })
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ActivityDecision {
    pub pause: bool,
    pub report_count: usize,
    /// Raw curated report text; todo18/19 must scrub before transport.
    pub thread_posts: Vec<String>,
    /// The item id was already captured: nothing was applied a second time.
    pub replayed: bool,
}

#[derive(Clone)]
pub struct LaneActivity {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl LaneActivity {
    pub fn new(pool: SqlitePool, clock: Arc<dyn Clock>) -> Self {
        Self { pool, clock }
    }

    /// Call once per captured complete agent message, not again at turn end.
    pub async fn captured(&self, key: &str, message: &str) -> Result<ActivityDecision> {
        self.captured_generation(key, message, None).await
    }

    pub async fn captured_generation(
        &self,
        key: &str,
        message: &str,
        generation: Option<i64>,
    ) -> Result<ActivityDecision> {
        self.capture(key, message, generation, None).await
    }

    pub async fn captured_item(
        &self,
        key: &str,
        message: &str,
        generation: i64,
        item_id: &str,
    ) -> Result<ActivityDecision> {
        self.capture(key, message, Some(generation), Some(item_id))
            .await
    }

    async fn capture(
        &self,
        key: &str,
        message: &str,
        generation: Option<i64>,
        item_id: Option<&str>,
    ) -> Result<ActivityDecision> {
        let mut tx = self.pool.begin().await?;
        let now = self.clock.now();
        let lane: Option<Lane> = if let Some(generation) = generation {
            sqlx::query_as("UPDATE lanes SET last_activity_at = ?, stale_posted_at = NULL, updated_at = ? WHERE key = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? RETURNING *")
                .bind(now).bind(now).bind(key).bind(key).bind(generation).fetch_optional(&mut *tx).await?
        } else {
            sqlx::query_as("UPDATE lanes SET last_activity_at = ?, stale_posted_at = NULL, updated_at = ? WHERE key = ? RETURNING *")
                .bind(now).bind(now).bind(key).fetch_optional(&mut *tx).await?
        };
        let Some(lane) = lane else {
            tx.rollback().await?;
            return Ok(ActivityDecision {
                pause: true,
                ..ActivityDecision::default()
            });
        };
        let source = format!(
            "{}:{}",
            generation.unwrap_or(0),
            item_id
                .map(str::to_owned)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        );
        if item_id.is_some() {
            let seen: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_transcript WHERE lane_key = ? AND role = 'activity_item' AND text = ?)")
                .bind(key).bind(&source).fetch_one(&mut *tx).await?;
            if seen {
                tx.rollback().await?;
                return Ok(ActivityDecision {
                    pause: lane.status != "working",
                    replayed: true,
                    ..ActivityDecision::default()
                });
            }
            sqlx::query("INSERT INTO lane_transcript(lane_key, at, role, text) VALUES (?, ?, 'activity_item', ?)")
                .bind(key).bind(now).bind(&source).execute(&mut *tx).await?;
        }
        let mut decision = ActivityDecision {
            pause: lane.status != "working",
            ..ActivityDecision::default()
        };
        let working_while_paused = sqlx::query("UPDATE lane_pauses SET seen_working_at = ? WHERE lane_key = ? AND resumed_at IS NULL AND acked_at IS NOT NULL AND seen_working_at IS NULL")
            .bind(now).bind(key).execute(&mut *tx).await?;
        if working_while_paused.rows_affected() == 1 {
            self.event(
                &mut tx,
                &lane,
                "working_while_paused",
                &format!("lane {key} produced output after its pause was confirmed"),
            )
            .await?;
        }
        for report in reports(key, message) {
            decision.report_count += 1;
            sqlx::query("UPDATE lanes SET last_report = ? WHERE key = ?")
                .bind(report.line)
                .bind(key)
                .execute(&mut *tx)
                .await?;
            if lane.status == "working" && !decision.pause {
                match report.kind {
                    ReportKind::Milestone => {
                        super::posts::enqueue(
                            &mut tx,
                            &lane,
                            &format!("{source}:{}", decision.report_count),
                            report.text,
                            now,
                        )
                        .await?;
                        decision.thread_posts.push(report.text.into());
                    }
                    ReportKind::Blocker | ReportKind::Question => {
                        let kind = if report.kind == ReportKind::Question {
                            "question"
                        } else {
                            "blocked"
                        };
                        self.block(&mut tx, &lane, kind, report.text).await?;
                        decision.pause = true;
                    }
                    ReportKind::Done => {
                        if lane.last_turn_outcome.as_deref() != Some("idle_without_claim") {
                            self.event(&mut tx, &lane, "idle_without_claim", report.text)
                                .await?;
                            sqlx::query("UPDATE lanes SET last_turn_outcome = 'idle_without_claim' WHERE key = ?")
                                .bind(key).execute(&mut *tx).await?;
                        }
                        decision.pause = true;
                    }
                }
            }
        }
        tx.commit().await?;
        Ok(decision)
    }

    /// Successful turn with zero accepted reports. Errors stay in the runner's
    /// consecutive-error counter; do not classify them or call this on a timer.
    pub async fn unreported_turn(
        &self,
        key: &str,
        last_message: &str,
        judge: &dyn Judge,
    ) -> Result<ActivityDecision> {
        let generation: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'")
            .bind(key).fetch_one(&self.pool).await?;
        self.unreported_turn_generation(key, last_message, judge, generation)
            .await
    }

    pub async fn unreported_turn_generation(
        &self,
        key: &str,
        last_message: &str,
        judge: &dyn Judge,
        generation: i64,
    ) -> Result<ActivityDecision> {
        let lane = get_lane(&self.pool, key)
            .await?
            .ok_or_else(|| OmonError::Config(format!("lane not found: {key}")))?;
        if lane.status != "working" {
            return Ok(ActivityDecision {
                pause: true,
                ..ActivityDecision::default()
            });
        }
        let excerpt: String = scrub(last_message).chars().take(2000).collect();
        let choice = judge
            .choice(
                Purpose::ReportIntent,
                &excerpt,
                "Classify the last lane message: finished, progress, or blocked_or_question.",
                &["finished", "progress", "blocked_or_question"],
            )
            .await
            .map_err(|error| OmonError::Config(format!("lane report intent: {error}")))?;
        let kind = match choice.selected.as_str() {
            "progress" => None,
            "finished" => Some("idle_without_claim"),
            "blocked_or_question" => Some("blocked"),
            other => return Err(OmonError::Config(format!("invalid report intent: {other}"))),
        };
        let mut tx = self.pool.begin().await?;
        // Judge ran outside the transaction: a concurrent claim/close wins.
        let current: Option<Lane> = sqlx::query_as("UPDATE lanes SET updated_at = updated_at WHERE key = ? AND status = 'working' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? RETURNING *")
            .bind(key).bind(key).bind(generation).fetch_optional(&mut *tx).await?;
        let mut decision = ActivityDecision {
            pause: current.is_none(),
            ..ActivityDecision::default()
        };
        if let (Some(lane), Some(kind)) = (current, kind) {
            if kind == "blocked" {
                self.block(&mut tx, &lane, kind, &excerpt).await?;
            } else {
                self.event(&mut tx, &lane, kind, &excerpt).await?;
                sqlx::query(
                    "UPDATE lanes SET last_turn_outcome = 'idle_without_claim' WHERE key = ?",
                )
                .bind(key)
                .execute(&mut *tx)
                .await?;
            }
            decision.pause = true;
        }
        tx.commit().await?;
        Ok(decision)
    }

    /// A host-continued turn ran no tool and sent no report: park the lane the way a finished
    /// turn without a claim is parked, and tell the lead once. Returns whether this call parked it;
    /// a concurrent claim, close or reopen wins.
    pub async fn no_progress(&self, key: &str, generation: i64) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let lane: Option<Lane> = sqlx::query_as("UPDATE lanes SET last_turn_outcome = 'idle_without_claim', updated_at = ? WHERE key = ? AND status = 'working' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? RETURNING *")
            .bind(self.clock.now()).bind(key).bind(key).bind(generation).fetch_optional(&mut *tx).await?;
        let Some(lane) = lane else {
            tx.rollback().await?;
            return Ok(false);
        };
        self.event(
            &mut tx,
            &lane,
            "no_progress",
            &format!(
                "lane {key} was continued by the host and ended its turn without a tool call or report; parked until a message arrives"
            ),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Cheap local check, armed only while a turn is running. Persisted latch
    /// makes repeated checks and reconstruction after restart produce no effects.
    pub async fn quiet(&self, key: &str, turn_running: bool) -> Result<ActivityDecision> {
        self.quiet_generation(key, turn_running, None).await
    }

    pub async fn quiet_generation(
        &self,
        key: &str,
        turn_running: bool,
        generation: Option<i64>,
    ) -> Result<ActivityDecision> {
        if !turn_running {
            return Ok(ActivityDecision::default());
        }
        let now = self.clock.now();
        let mut tx = self.pool.begin().await?;
        let lane: Option<Lane> = if let Some(generation) = generation {
            sqlx::query_as("UPDATE lanes SET stale_posted_at = ?, updated_at = ? WHERE key = ? AND status = 'working' AND stale_posted_at IS NULL AND last_activity_at <= ? AND NOT EXISTS(SELECT 1 FROM lane_pauses WHERE lane_key = lanes.key AND resumed_at IS NULL) AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? RETURNING *")
                .bind(now).bind(now).bind(key).bind(now - Duration::minutes(20)).bind(key).bind(generation).fetch_optional(&mut *tx).await?
        } else {
            sqlx::query_as("UPDATE lanes SET stale_posted_at = ?, updated_at = ? WHERE key = ? AND status = 'working' AND stale_posted_at IS NULL AND last_activity_at <= ? AND NOT EXISTS(SELECT 1 FROM lane_pauses WHERE lane_key = lanes.key AND resumed_at IS NULL) RETURNING *")
                .bind(now).bind(now).bind(key).bind(now - Duration::minutes(20)).fetch_optional(&mut *tx).await?
        };
        let mut decision = ActivityDecision::default();
        if let Some(lane) = lane {
            let text = lane
                .last_report
                .as_deref()
                .unwrap_or("No lane report received.");
            self.event(&mut tx, &lane, "stale", text).await?;
            super::posts::enqueue(
                &mut tx,
                &lane,
                &format!("stale:{}:{}", generation.unwrap_or(0), now.to_rfc3339()),
                text,
                now,
            )
            .await?;
            decision.thread_posts.push(text.into());
        }
        tx.commit().await?;
        Ok(decision)
    }

    async fn block(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        lane: &Lane,
        kind: &str,
        text: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE lanes SET status = 'blocked', updated_at = ? WHERE key = ?")
            .bind(self.clock.now())
            .bind(&lane.key)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'working', 'blocked', ?)")
            .bind(&lane.key).bind(self.clock.now()).bind(super::state::safe_reason(text)).execute(&mut **tx).await?;
        self.event(tx, lane, kind, text).await
    }

    async fn event(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        lane: &Lane,
        kind: &str,
        text: &str,
    ) -> Result<()> {
        let body: String = scrub(text).chars().take(2000).collect();
        sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&lane.key).bind(&lane.lead_session_key).bind(kind).bind(body)
            .bind(self.clock.now()).execute(&mut **tx).await?;
        Ok(())
    }
}

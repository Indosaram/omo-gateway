//! One evolving status message per long lead turn: posted once the turn has run for
//! [`STATUS_INTERVAL`], edited in place every interval, and finalized when the turn ends.
//! The posted message id is persisted in `turn_status_messages`, so a restarted turn edits
//! the stored message instead of posting a duplicate.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use parking_lot::Mutex;
use sqlx::SqlitePool;

use crate::{strip_leading_message_timestamps, OutboundAction, OutboundDispatcher, SessionKey};

pub const STATUS_INTERVAL: Duration = Duration::from_secs(180);
const OWNER_RECENT_SECS: i64 = 10 * 60;
const HEADLINE_CHARS: usize = 80;
const REASON_CHARS: usize = 200;

/// `OMON_STATUS_MESSAGE_ENABLED` (default true), read once per process.
pub fn status_message_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        !matches!(
            std::env::var("OMON_STATUS_MESSAGE_ENABLED")
                .map(|value| value.trim().to_ascii_lowercase())
                .as_deref(),
            Ok("0" | "false" | "no" | "off")
        )
    })
}

/// KST 00:00-07:59 is quiet: no new status message and no interim edits.
pub fn in_quiet_hours(now: DateTime<Utc>) -> bool {
    now.with_timezone(&chrono_tz::Asia::Seoul).hour() < 8
}

fn tool_registry() -> &'static Mutex<HashMap<String, String>> {
    static TOOLS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    TOOLS.get_or_init(Default::default)
}

/// Records the most recent tool a session's running turn started, shown in interim edits.
pub fn record_tool_activity(session: &SessionKey, tool: &str) {
    let tool = tool.trim();
    if tool.is_empty() {
        return;
    }
    tool_registry()
        .lock()
        .insert(session.storage_key(), tool.chars().take(60).collect());
}

fn last_tool(storage_key: &str) -> Option<String> {
    tool_registry().lock().get(storage_key).cloned()
}

fn clear_tool(storage_key: &str) {
    tool_registry().lock().remove(storage_key);
}

pub struct TurnStart {
    pub session: SessionKey,
    pub event_id: String,
    pub owner_text: String,
    pub reply_to: Option<String>,
    /// When the owner's triggering message arrived; lifts the quiet-hour hold for 10 minutes.
    pub owner_message_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
}

pub struct StatusDriver {
    pool: SqlitePool,
    dispatcher: Arc<dyn OutboundDispatcher>,
    session: SessionKey,
    storage_key: String,
    event_id: String,
    reply_to: Option<String>,
    headline: String,
    started_at: DateTime<Utc>,
    owner_message_at: Option<DateTime<Utc>>,
    message_id: Option<String>,
    next_tick: i64,
    unsupported: bool,
}

impl StatusDriver {
    /// Prepares the driver for a turn, adopting a still-running stored message for the
    /// same event (restart recovery) instead of planning a new one.
    pub async fn start(
        pool: SqlitePool,
        dispatcher: Arc<dyn OutboundDispatcher>,
        turn: TurnStart,
    ) -> Self {
        let storage_key = turn.session.storage_key();
        let stored: Option<(String, DateTime<Utc>)> = match sqlx::query_as(
            "SELECT platform_message_id, started_at FROM turn_status_messages
             WHERE session_key = ? AND event_id = ? AND state = 'running'",
        )
        .bind(&storage_key)
        .bind(&turn.event_id)
        .fetch_optional(&pool)
        .await
        {
            Ok(row) => row,
            Err(error) => {
                tracing::warn!(session = %turn.session, %error, "failed to load stored turn status message");
                None
            }
        };
        let (message_id, started_at) = match stored {
            Some((id, at)) => (Some(id), at),
            None => (None, turn.started_at),
        };
        let headline: String = strip_leading_message_timestamps(&turn.owner_text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(HEADLINE_CHARS)
            .collect();
        let mut driver = Self {
            pool,
            dispatcher,
            session: turn.session,
            storage_key,
            event_id: turn.event_id,
            reply_to: turn.reply_to.filter(|id| !id.is_empty()),
            headline,
            started_at,
            owner_message_at: turn.owner_message_at,
            message_id,
            next_tick: 1,
            unsupported: false,
        };
        driver.next_tick = driver.ticks_elapsed(turn.started_at) + 1;
        driver
    }

    fn ticks_elapsed(&self, now: DateTime<Utc>) -> i64 {
        ((now - self.started_at).num_seconds() / STATUS_INTERVAL.as_secs() as i64).max(0)
    }

    /// Wall-clock time of the next status post or edit.
    pub fn next_due(&self) -> DateTime<Utc> {
        self.started_at
            + chrono::Duration::seconds(self.next_tick * STATUS_INTERVAL.as_secs() as i64)
    }

    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    fn held_for_quiet_hours(&self, now: DateTime<Utc>) -> bool {
        in_quiet_hours(now)
            && self
                .owner_message_at
                .is_none_or(|at| (now - at).num_seconds() > OWNER_RECENT_SECS)
    }

    /// Posts the status message on the first due tick, then edits it on later ones.
    pub async fn tick(&mut self, now: DateTime<Utc>) {
        if now < self.next_due() {
            return;
        }
        self.next_tick = self.ticks_elapsed(now) + 1;
        if self.unsupported || self.held_for_quiet_hours(now) {
            return;
        }
        let minutes = (now - self.started_at).num_minutes().max(0);
        match self.message_id.clone() {
            None => self.post(now, minutes).await,
            Some(id) => {
                let mut body = format!("작업 중: {} · {minutes}분", self.headline);
                if let Some(tool) = last_tool(&self.storage_key) {
                    body.push_str(&format!(" · {tool}"));
                }
                self.edit(&id, body).await;
            }
        }
    }

    async fn post(&mut self, now: DateTime<Utc>, minutes: i64) {
        let body = format!("작업 중: {} · {minutes}분", self.headline);
        match self
            .dispatcher
            .post_status(&self.session, body, self.reply_to.clone())
            .await
        {
            Ok(Some(id)) => {
                if let Err(error) = sqlx::query(
                    "INSERT INTO turn_status_messages
                         (session_key, event_id, platform_message_id, started_at, state, updated_at)
                     VALUES (?, ?, ?, ?, 'running', ?)
                     ON CONFLICT(session_key, event_id) DO UPDATE SET
                         platform_message_id = excluded.platform_message_id,
                         state = 'running',
                         updated_at = excluded.updated_at",
                )
                .bind(&self.storage_key)
                .bind(&self.event_id)
                .bind(&id)
                .bind(self.started_at)
                .bind(now)
                .execute(&self.pool)
                .await
                {
                    tracing::warn!(session = %self.session, %error, "failed to persist turn status message");
                }
                self.message_id = Some(id);
            }
            Ok(None) => {
                tracing::debug!(session = %self.session, "dispatcher cannot return status message ids; status message off for this turn");
                self.unsupported = true;
            }
            Err(error) => {
                tracing::warn!(session = %self.session, %error, "failed to post turn status message");
            }
        }
    }

    async fn edit(&self, id: &str, content: String) {
        if let Err(error) = self
            .dispatcher
            .dispatch(OutboundAction::EditMessage {
                session: self.session.clone(),
                platform_message_id: id.to_owned(),
                content,
            })
            .await
        {
            tracing::warn!(session = %self.session, %error, "failed to edit turn status message");
        }
    }

    /// Final edit to "완료 · N분" or "실패 · <reason>". A turn that never posted sends nothing.
    /// Finishing an existing message is allowed in quiet hours: it is an edit, not a new post.
    pub async fn finish(self, now: DateTime<Utc>, outcome: std::result::Result<(), String>) {
        clear_tool(&self.storage_key);
        let Some(id) = self.message_id.clone() else {
            return;
        };
        let minutes = (now - self.started_at).num_minutes().max(0);
        let (body, state) = match outcome {
            Ok(()) => (format!("완료 · {minutes}분"), "done"),
            Err(reason) => {
                let reason: String = reason.chars().take(REASON_CHARS).collect();
                (format!("실패 · {reason}"), "failed")
            }
        };
        self.edit(&id, body).await;
        if let Err(error) = sqlx::query(
            "UPDATE turn_status_messages SET state = ?, updated_at = ?
             WHERE session_key = ? AND event_id = ?",
        )
        .bind(state)
        .bind(now)
        .bind(&self.storage_key)
        .bind(&self.event_id)
        .execute(&self.pool)
        .await
        {
            tracing::warn!(session = %self.session, %error, "failed to finalize turn status message");
        }
    }

    /// Leaves the stored message running for the restarted turn to adopt.
    pub fn abandon(self) {
        clear_tool(&self.storage_key);
    }
}

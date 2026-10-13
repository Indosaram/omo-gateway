use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{DeliveryStatus, InboundEvent, OmonError, Result, SessionKey};

pub const RECOVERED_REPLY_MARKER: &str =
    "♻️ Recovered reply — the gateway restarted during delivery, so this may be a duplicate:\n\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryObligationState {
    Pending,
    Attempting,
    Delivered,
    Failed,
    Abandoned,
}

impl DeliveryObligationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Attempting => "attempting",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
        }
    }
}

use std::sync::OnceLock;

pub fn process_started_at() -> &'static str {
    static STARTED_AT: OnceLock<String> = OnceLock::new();
    STARTED_AT.get_or_init(|| Utc::now().to_rfc3339())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct DeliveryObligation {
    pub id: String,
    pub session_key: String,
    pub channel_id: String,
    pub thread_id: Option<String>,
    pub content: String,
    pub state: String,
    pub attempts: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_pid: Option<i64>,
    pub last_error: Option<String>,
    #[serde(default)]
    pub owner_started_at: Option<String>,
}

/// Returns true if the process with the given PID is currently alive on this host.
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid as i32, 0) };
        if res == 0 {
            true
        } else {
            let err = std::io::Error::last_os_error();
            err.raw_os_error() == Some(libc::EPERM)
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle == 0 || handle == -1 {
            false
        } else {
            let mut exit_code: u32 = 0;
            let res = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
            unsafe { CloseHandle(handle) };
            if res != 0 {
                exit_code == 259 // STILL_ACTIVE
            } else {
                false
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct DeliveryLedgerEntry {
    pub message_id: String,
    pub session_key: String,
    pub status: String,
    pub received_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub processing_latency_ms: Option<i64>,
    pub platform_message_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DeliveryLedgerService {
    pool: SqlitePool,
}

impl DeliveryLedgerService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record_incoming(&self, event: &InboundEvent) -> Result<bool> {
        self.record_incoming_as(event, &event.platform_message_id)
            .await
    }

    pub async fn record_incoming_as(
        &self,
        event: &InboundEvent,
        delivery_id: &str,
    ) -> Result<bool> {
        self.ensure_session(&event.session).await?;
        claim_delivery_row(&self.pool, event, delivery_id).await
    }

    /// True when a live claim exists: any row for this id that is not `failed`.
    /// Failed rows stay reclaimable by `record_incoming_as`.
    async fn is_claimed(&self, message_id: &str) -> Result<bool> {
        let alt = message_id.strip_prefix("discord:").unwrap_or_default();
        let prefixed = format!("discord:{message_id}");
        let claimed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM delivery_ledger
             WHERE (message_id = ? OR message_id = ? OR message_id = ? OR platform_message_id = ?)
             AND status != 'failed')",
        )
        .bind(message_id)
        .bind(alt)
        .bind(&prefixed)
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(claimed)
    }

    pub async fn is_duplicate(&self, message_id: &str) -> Result<bool> {
        let alt = message_id.strip_prefix("discord:").unwrap_or_default();
        let prefixed = format!("discord:{message_id}");
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM delivery_ledger WHERE message_id = ? OR message_id = ? OR message_id = ? OR platform_message_id = ?)",
        )
        .bind(message_id)
        .bind(alt)
        .bind(&prefixed)
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    pub async fn is_completed(&self, message_id: &str) -> Result<bool> {
        let alt = message_id.strip_prefix("discord:").unwrap_or_default();
        let prefixed = format!("discord:{message_id}");
        let completed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM delivery_ledger WHERE (message_id = ? OR message_id = ? OR message_id = ? OR platform_message_id = ?) AND status = 'delivered')",
        )
        .bind(message_id)
        .bind(alt)
        .bind(&prefixed)
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(completed)
    }

    pub async fn record_incoming_with_constituents(
        &self,
        event: &InboundEvent,
        delivery_id: &str,
        constituent_ids: &[String],
    ) -> Result<bool> {
        self.ensure_session(&event.session).await?;
        self.ensure_constituents_table().await?;

        // Only a live claim on the batch's own delivery id makes it a duplicate: a failed
        // batch is reclaimable, and one already-seen constituent must not drop the others.
        if self.is_claimed(delivery_id).await? {
            return Ok(false);
        }
        let mut fresh: Vec<&String> = Vec::new();
        for c_id in constituent_ids {
            if c_id != delivery_id && !self.is_claimed(c_id).await? {
                fresh.push(c_id);
            }
        }

        let mut tx = self.pool.begin().await?;
        if !claim_delivery_row(&mut *tx, event, delivery_id).await? {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO delivery_ledger_constituents (parent_delivery_id, constituent_id)
             VALUES (?, ?)
             ON CONFLICT(constituent_id) DO UPDATE SET parent_delivery_id = excluded.parent_delivery_id",
        )
        .bind(delivery_id)
        .bind(delivery_id)
        .execute(&mut *tx)
        .await?;

        for c_id in fresh {
            let platform_msg_id = c_id.strip_prefix("discord:").unwrap_or(c_id);
            sqlx::query(
                "INSERT INTO delivery_ledger (
                    delivery_id, session_key, event_id, message_id, status,
                    platform_message_id, created_at, updated_at, received_at
                 ) VALUES (?, ?, ?, ?, 'in_progress', ?, ?, ?, ?)
                 ON CONFLICT(message_id) DO UPDATE SET
                    status = 'in_progress',
                    updated_at = excluded.updated_at,
                    error = NULL
                 WHERE delivery_ledger.status = 'failed'",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(event.session.storage_key())
            .bind(c_id)
            .bind(c_id)
            .bind(platform_msg_id)
            .bind(event.received_at)
            .bind(event.received_at)
            .bind(event.received_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO delivery_ledger_constituents (parent_delivery_id, constituent_id)
                 VALUES (?, ?)
                 ON CONFLICT(constituent_id) DO UPDATE SET parent_delivery_id = excluded.parent_delivery_id",
            )
            .bind(delivery_id)
            .bind(c_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    pub async fn mark_delivered(&self, message_id: &str) -> Result<()> {
        self.complete(message_id, DeliveryStatus::Delivered, None)
            .await
    }

    pub async fn mark_failed(&self, message_id: &str, error: impl Into<String>) -> Result<()> {
        self.complete(message_id, DeliveryStatus::Failed, Some(error.into()))
            .await
    }

    pub async fn get(&self, message_id: &str) -> Result<Option<DeliveryLedgerEntry>> {
        let alt = message_id.strip_prefix("discord:").unwrap_or_default();
        let prefixed = format!("discord:{message_id}");
        Ok(sqlx::query_as::<_, DeliveryLedgerEntry>(
            "SELECT message_id, session_key, status, received_at, completed_at,
                    processing_latency_ms, platform_message_id, error
             FROM delivery_ledger
             WHERE message_id = ? OR message_id = ? OR message_id = ? OR platform_message_id = ?
             ORDER BY completed_at DESC, created_at DESC
             LIMIT 1",
        )
        .bind(message_id)
        .bind(alt)
        .bind(&prefixed)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn complete(
        &self,
        message_id: &str,
        status: DeliveryStatus,
        error: Option<String>,
    ) -> Result<()> {
        let completed_at = Utc::now();
        let status = status_name(&status);
        let err_str = error.as_deref();
        self.ensure_constituents_table().await?;
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE delivery_ledger
             SET status = ?, error = ?, completed_at = ?, updated_at = ?,
                 processing_latency_ms = MAX(0, CAST((julianday(?) - julianday(received_at)) * 86400000 AS INTEGER))
             WHERE message_id = ?",
        )
        .bind(status)
        .bind(err_str)
        .bind(completed_at)
        .bind(completed_at)
        .bind(completed_at)
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(OmonError::Database(format!(
                "delivery message {message_id} does not exist"
            )));
        }

        sqlx::query(
            "UPDATE delivery_ledger
             SET status = ?, error = ?, completed_at = ?, updated_at = ?,
                 processing_latency_ms = MAX(0, CAST((julianday(?) - julianday(received_at)) * 86400000 AS INTEGER))
             WHERE message_id IN (
                 SELECT constituent_id FROM delivery_ledger_constituents WHERE parent_delivery_id = ?
             )",
        )
        .bind(status)
        .bind(err_str)
        .bind(completed_at)
        .bind(completed_at)
        .bind(completed_at)
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(())
    }

    async fn ensure_constituents_table(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS delivery_ledger_constituents (
                parent_delivery_id TEXT NOT NULL,
                constituent_id TEXT NOT NULL PRIMARY KEY
            );",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_delivery_ledger_constituents_parent
             ON delivery_ledger_constituents(parent_delivery_id);",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_session(&self, session: &SessionKey) -> Result<()> {
        let state_json = serde_json::to_string(&crate::SessionState::default())
            .map_err(|error| OmonError::Database(error.to_string()))?;
        sqlx::query(
            "INSERT INTO sessions (
                session_key, platform, guild_id, channel_id, thread_id, user_id, state_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_key) DO NOTHING",
        )
        .bind(session.storage_key())
        .bind(&session.platform)
        .bind(&session.guild_id)
        .bind(&session.channel_id)
        .bind(&session.thread_id)
        .bind(&session.user_id)
        .bind(state_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Records an outbound delivery obligation as 'pending'.
    pub async fn record_obligation(
        &self,
        id: &str,
        session: &SessionKey,
        content: &str,
    ) -> Result<()> {
        self.ensure_session(session).await?;
        let now = Utc::now();
        let pid = std::process::id() as i64;
        let started_at = process_started_at();
        sqlx::query(
            "INSERT INTO delivery_obligations (
                id, session_key, channel_id, thread_id, content, state,
                attempts, created_at, updated_at, owner_pid, last_error, owner_started_at
             ) VALUES (?, ?, ?, ?, ?, 'pending', 0, ?, ?, ?, NULL, ?)
             ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                -- New content has not been sent yet, whatever the old row's state was.
                state = 'pending',
                updated_at = excluded.updated_at,
                owner_pid = excluded.owner_pid,
                owner_started_at = excluded.owner_started_at",
        )
        .bind(id)
        .bind(session.storage_key())
        .bind(&session.channel_id)
        .bind(&session.thread_id)
        .bind(content)
        .bind(now)
        .bind(now)
        .bind(pid)
        .bind(started_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks an obligation as 'attempting' immediately before dispatch.
    pub async fn mark_obligation_attempting(&self, id: &str) -> Result<()> {
        self.update_obligation_state(id, "attempting", None).await
    }

    /// Marks an obligation as 'delivered' once dispatch is confirmed.
    pub async fn mark_obligation_delivered(&self, id: &str) -> Result<()> {
        self.update_obligation_state(id, "delivered", None).await
    }

    /// Marks an obligation as 'failed' on definitive rejection or error.
    pub async fn mark_obligation_failed(&self, id: &str, error: &str) -> Result<()> {
        self.update_obligation_state(id, "failed", Some(error))
            .await
    }

    async fn update_obligation_state(
        &self,
        id: &str,
        state: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE delivery_obligations
             SET state = ?, updated_at = ?, last_error = ?
             WHERE id = ?",
        )
        .bind(state)
        .bind(now)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(OmonError::Database(format!(
                "delivery obligation {id} not found"
            )));
        }
        Ok(())
    }

    /// Fetches a delivery obligation by ID.
    pub async fn get_obligation(&self, id: &str) -> Result<Option<DeliveryObligation>> {
        let row = sqlx::query_as::<_, DeliveryObligation>(
            "SELECT id, session_key, channel_id, thread_id, content, state,
                    attempts, created_at, updated_at, owner_pid, last_error, owner_started_at
             FROM delivery_obligations
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Finds undelivered obligations owned by dead processes, increments attempts,
    /// re-stamps ownership to this process, and returns them for redelivery.
    ///
    /// Stale rows (older than `stale_after_secs`) and exhausted rows (`attempts >= max_attempts`)
    /// are transitioned to 'abandoned' and omitted from the return list.
    pub async fn sweep_recoverable(
        &self,
        max_attempts: i64,
        stale_after_secs: i64,
    ) -> Result<Vec<DeliveryObligation>> {
        let now = Utc::now();
        let current_pid = std::process::id() as i64;
        let current_started_at = process_started_at();
        let rows = sqlx::query_as::<_, DeliveryObligation>(
            "SELECT id, session_key, channel_id, thread_id, content, state,
                    attempts, created_at, updated_at, owner_pid, last_error, owner_started_at
             FROM delivery_obligations
             WHERE state IN ('pending', 'attempting', 'failed')
             ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut claimed = Vec::new();
        for row in rows {
            if let Some(pid) = row.owner_pid {
                if pid > 0 && is_process_alive(pid as u32) {
                    // Only a row stamped by THIS process start is owned by a live instance. A
                    // legacy row with no stamp cannot prove ownership (PID reuse, e.g. PID 1
                    // in containers), so it falls through to the stale/abandon checks.
                    if row.owner_started_at.as_deref() == Some(current_started_at) {
                        continue;
                    }
                }
            }

            let age_secs = now.signed_duration_since(row.created_at).num_seconds();
            if row.attempts >= max_attempts || age_secs > stale_after_secs {
                sqlx::query(
                    "UPDATE delivery_obligations SET state = 'abandoned', updated_at = ? WHERE id = ?",
                )
                .bind(now)
                .bind(&row.id)
                .execute(&self.pool)
                .await?;
                continue;
            }

            let result = sqlx::query(
                "UPDATE delivery_obligations
                 SET owner_pid = ?, owner_started_at = ?, attempts = attempts + 1, updated_at = ?
                 WHERE id = ? AND (owner_pid IS NULL OR owner_pid = ? OR owner_pid = ?)",
            )
            .bind(current_pid)
            .bind(current_started_at)
            .bind(now)
            .bind(&row.id)
            .bind(row.owner_pid)
            .bind(current_pid)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() > 0 {
                let mut updated_row = row;
                updated_row.attempts += 1;
                updated_row.owner_pid = Some(current_pid);
                updated_row.owner_started_at = Some(current_started_at.to_string());
                claimed.push(updated_row);
            }
        }

        Ok(claimed)
    }

    /// Prunes terminal delivery obligations ('delivered' or 'abandoned') by age and count bounds.
    pub async fn prune_terminal_obligations(
        &self,
        retention_secs: i64,
        max_retained_rows: i64,
    ) -> Result<u64> {
        let now = Utc::now();
        let cutoff = now - chrono::TimeDelta::seconds(retention_secs);
        let mut total_pruned = 0;

        let res = sqlx::query(
            "DELETE FROM delivery_obligations
             WHERE state IN ('delivered', 'abandoned') AND updated_at < ?",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;
        total_pruned += res.rows_affected();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM delivery_obligations WHERE state IN ('delivered', 'abandoned')",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if count > max_retained_rows {
            let excess = count - max_retained_rows;
            let res = sqlx::query(
                "DELETE FROM delivery_obligations
                 WHERE id IN (
                     SELECT id FROM delivery_obligations
                     WHERE state IN ('delivered', 'abandoned')
                     ORDER BY updated_at ASC
                     LIMIT ?
                 )",
            )
            .bind(excess)
            .execute(&self.pool)
            .await?;
            total_pruned += res.rows_affected();
        }

        Ok(total_pruned)
    }

    /// Scans for failed delivery obligations owned by the current process instance
    /// and belonging to `bot_id`, where the error was a transient transport error (e.g. connection refused).
    ///
    /// Excludes non-retryable errors (e.g. HTTP 403 Forbidden, request timeout ambiguity).
    /// Atomically transitions claimed obligations from 'failed' to 'attempting'.
    pub async fn sweep_failed_for_runtime(
        &self,
        bot_id: &str,
        max_attempts: i64,
        stale_after_secs: i64,
    ) -> Result<Vec<DeliveryObligation>> {
        let now = Utc::now();
        let current_pid = std::process::id() as i64;
        let current_started_at = process_started_at();

        let rows = sqlx::query_as::<_, DeliveryObligation>(
            "SELECT id, session_key, channel_id, thread_id, content, state,
                    attempts, created_at, updated_at, owner_pid, last_error, owner_started_at
             FROM delivery_obligations
             WHERE state = 'failed' AND owner_pid = ? AND owner_started_at = ?
             ORDER BY created_at ASC",
        )
        .bind(current_pid)
        .bind(current_started_at)
        .fetch_all(&self.pool)
        .await?;

        let mut claimed = Vec::new();
        for row in rows {
            let matches_bot = if let Ok(session) = SessionKey::from_storage_key(&row.session_key) {
                session.bot_id.as_deref() == Some(bot_id)
            } else {
                false
            };
            if !matches_bot {
                continue;
            }

            let is_retryable_transport = match row.last_error.as_deref() {
                Some(err) => {
                    let lower = err.to_ascii_lowercase();
                    (lower.contains("connection refused")
                        || lower.contains("connect error")
                        || lower.contains("transport error")
                        || lower.contains("connection reset")
                        || lower.contains("broken pipe"))
                        && !lower.contains("403")
                        && !lower.contains("forbidden")
                        && !lower.contains("timeout")
                }
                None => false,
            };

            if !is_retryable_transport {
                continue;
            }

            let age_secs = now.signed_duration_since(row.created_at).num_seconds();
            if row.attempts >= max_attempts || age_secs > stale_after_secs {
                continue;
            }

            let result = sqlx::query(
                "UPDATE delivery_obligations
                 SET state = 'attempting', attempts = attempts + 1, updated_at = ?
                 WHERE id = ? AND state = 'failed'",
            )
            .bind(now)
            .bind(&row.id)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() > 0 {
                let mut updated = row;
                updated.state = "attempting".to_string();
                updated.attempts += 1;
                claimed.push(updated);
            }
        }

        Ok(claimed)
    }
}

/// Startup recovery sweep: finds undelivered obligations from previous dead processes,
/// claims them, and re-dispatches them to the platform (tagging recovered deliveries).
/// Returns only deliveries whose dispatch and delivered checkpoint both succeeded.
pub async fn recover_pending_delivery_obligations(
    pool: &SqlitePool,
    dispatcher: std::sync::Arc<dyn crate::OutboundDispatcher>,
) -> Result<usize> {
    let ledger = DeliveryLedgerService::new(pool.clone());
    let recoverable = ledger.sweep_recoverable(3, 86400).await?;
    let mut count = 0;
    // One obligation's bookkeeping failure must not strand the remaining recoverable
    // obligations; the first error is still returned so the caller sees it.
    let mut first_error: Option<OmonError> = None;
    for obligation in recoverable {
        let session_key: SessionKey = if let Ok(key) =
            SessionKey::from_storage_key(&obligation.session_key)
        {
            key
        } else if let Ok(Some((stored_key, platform, guild_id, channel_id, thread_id, user_id))) =
            sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, String)>(
                "SELECT session_key, platform, guild_id, channel_id, thread_id, user_id FROM sessions WHERE session_key = ?",
            )
            .bind(&obligation.session_key)
            .fetch_optional(pool)
            .await
        {
            SessionKey::from_storage_key(&stored_key).unwrap_or_else(|_| {
                SessionKey::new(platform, guild_id, channel_id, thread_id, user_id)
            })
        } else {
            SessionKey::new(
                "discord",
                None::<String>,
                &obligation.channel_id,
                obligation.thread_id.as_deref(),
                "recovered-delivery",
            )
        };

        let content = if obligation.state == "pending" {
            obligation.content.clone()
        } else {
            format!("{}{}", RECOVERED_REPLY_MARKER, obligation.content)
        };

        let stream_id = uuid::Uuid::new_v4();
        let chunk = crate::StreamChunk {
            stream_id,
            sequence: 0,
            content,
            is_final: true,
            reply_to: None,
        };

        if let Err(error) = ledger.mark_obligation_attempting(&obligation.id).await {
            tracing::error!(%error, obligation = %obligation.id, "failed to mark obligation attempting; skipping dispatch");
            first_error.get_or_insert(error);
            continue;
        }
        let result = dispatcher
            .dispatch(crate::OutboundAction::Stream {
                session: session_key,
                chunk,
            })
            .await;

        match result {
            Ok(_) => match ledger.mark_obligation_delivered(&obligation.id).await {
                Ok(()) => count += 1,
                Err(error) => {
                    tracing::error!(%error, obligation = %obligation.id, "reply dispatched but delivered checkpoint failed");
                    first_error.get_or_insert(error);
                }
            },
            Err(ref err) => {
                if let Err(error) = ledger
                    .mark_obligation_failed(&obligation.id, &err.to_string())
                    .await
                {
                    tracing::error!(%error, obligation = %obligation.id, "failed to record obligation dispatch failure");
                    first_error.get_or_insert(error);
                }
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(count),
    }
}

/// Inserts the delivery row as `in_progress`, or reclaims a previously `failed` one.
/// Returns true when a row was inserted or reclaimed; an already live claim yields false.
async fn claim_delivery_row<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Sqlite>,
    event: &InboundEvent,
    delivery_id: &str,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO delivery_ledger (
            delivery_id, session_key, event_id, message_id, status,
            platform_message_id, created_at, updated_at, received_at
         ) VALUES (?, ?, ?, ?, 'in_progress', ?, ?, ?, ?)
         ON CONFLICT(message_id) DO UPDATE SET
            status = 'in_progress',
            updated_at = excluded.updated_at,
            error = NULL
         WHERE delivery_ledger.status = 'failed'",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(event.session.storage_key())
    .bind(delivery_id)
    .bind(delivery_id)
    .bind(&event.platform_message_id)
    .bind(event.received_at)
    .bind(event.received_at)
    .bind(event.received_at)
    .execute(executor)
    .await?;
    // A fresh insert or the reclaim of a previously failed delivery both yield one
    // affected row; an already pending/delivered claim yields zero and stays deduped.
    Ok(result.rows_affected() == 1)
}

fn status_name(status: &DeliveryStatus) -> &'static str {
    match status {
        DeliveryStatus::Pending => "pending",
        DeliveryStatus::InProgress => "in_progress",
        DeliveryStatus::Delivered => "delivered",
        DeliveryStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use chrono::Duration;

    fn test_session() -> SessionKey {
        SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-1",
            None::<String>,
            "user-1",
        )
    }

    #[tokio::test]
    async fn test_obligation_lifecycle_transitions() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = DeliveryLedgerService::new(db.pool().clone());
        let session = test_session();

        let obl_id = "test-obl-1";
        service
            .record_obligation(obl_id, &session, "Hello world response")
            .await
            .unwrap();

        let obl: DeliveryObligation = service.get_obligation(obl_id).await.unwrap().unwrap();
        assert_eq!(obl.id, obl_id);
        assert_eq!(obl.state, "pending");
        assert_eq!(obl.attempts, 0);
        assert_eq!(obl.content, "Hello world response");
        assert_eq!(obl.channel_id, "chan-1");
        assert_eq!(obl.last_error, None);

        // Transition to attempting
        service.mark_obligation_attempting(obl_id).await.unwrap();
        let obl: DeliveryObligation = service.get_obligation(obl_id).await.unwrap().unwrap();
        assert_eq!(obl.state, "attempting");

        // Transition to delivered
        service.mark_obligation_delivered(obl_id).await.unwrap();
        let obl: DeliveryObligation = service.get_obligation(obl_id).await.unwrap().unwrap();
        assert_eq!(obl.state, "delivered");

        // Transition to failed
        service
            .mark_obligation_failed(obl_id, "Network timeout 504")
            .await
            .unwrap();
        let obl: DeliveryObligation = service.get_obligation(obl_id).await.unwrap().unwrap();
        assert_eq!(obl.state, "failed");
        assert_eq!(obl.last_error.as_deref(), Some("Network timeout 504"));
    }

    #[tokio::test]
    async fn test_sweep_recoverable_selection_and_pruning() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = DeliveryLedgerService::new(db.pool().clone());
        let session = test_session();
        let current_pid = std::process::id() as i64;
        let dead_pid = current_pid; // Previous incarnation; historical start stamp set below.
        let now = Utc::now();

        // 1. Pending obligation from dead process -> recoverable
        service
            .record_obligation("obl-pending-dead", &session, "content 1")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET owner_pid = ? WHERE id = 'obl-pending-dead'")
            .bind(dead_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 2. Attempting obligation from dead process -> recoverable
        service
            .record_obligation("obl-attempting-dead", &session, "content 2")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET state = 'attempting', owner_pid = ? WHERE id = 'obl-attempting-dead'")
            .bind(dead_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 3. Failed obligation from dead process -> recoverable
        service
            .record_obligation("obl-failed-dead", &session, "content 3")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET state = 'failed', owner_pid = ? WHERE id = 'obl-failed-dead'")
            .bind(dead_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 4. Delivered obligation from dead process -> NOT recoverable
        service
            .record_obligation("obl-delivered-dead", &session, "content 4")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET state = 'delivered', owner_pid = ? WHERE id = 'obl-delivered-dead'")
            .bind(dead_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 5. Pending obligation from CURRENT LIVE process -> NOT recoverable (live process owns it)
        service
            .record_obligation("obl-live-proc", &session, "content 5")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET owner_pid = ? WHERE id = 'obl-live-proc'")
            .bind(current_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 6. Obligation with attempts >= 3 -> abandoned
        service
            .record_obligation("obl-max-attempts", &session, "content 6")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET attempts = 3, owner_pid = ? WHERE id = 'obl-max-attempts'")
            .bind(dead_pid)
            .execute(db.pool())
            .await
            .unwrap();

        // 7. Obligation older than stale cutoff (2 days old) -> abandoned
        let stale_time = now - Duration::days(2);
        service
            .record_obligation("obl-stale", &session, "content 7")
            .await
            .unwrap();
        sqlx::query(
            "UPDATE delivery_obligations SET created_at = ?, owner_pid = ? WHERE id = 'obl-stale'",
        )
        .bind(stale_time)
        .bind(dead_pid)
        .execute(db.pool())
        .await
        .unwrap();

        // Model a previous incarnation of our owned PID, without assuming that
        // any arbitrary PID is absent. Keep the live row's current start stamp.
        let previous_started_at =
            DateTime::parse_from_rfc3339(process_started_at()).unwrap() - Duration::days(1);
        sqlx::query(
            "UPDATE delivery_obligations SET owner_started_at = ?
             WHERE owner_pid = ? AND id != 'obl-live-proc'",
        )
        .bind(previous_started_at.to_rfc3339())
        .bind(current_pid)
        .execute(db.pool())
        .await
        .unwrap();

        let claimed: Vec<DeliveryObligation> = service.sweep_recoverable(3, 86400).await.unwrap();
        let claimed_ids: Vec<String> = claimed.into_iter().map(|o| o.id).collect();

        assert_eq!(claimed_ids.len(), 3);
        assert!(claimed_ids.contains(&"obl-pending-dead".to_string()));
        assert!(claimed_ids.contains(&"obl-attempting-dead".to_string()));
        assert!(claimed_ids.contains(&"obl-failed-dead".to_string()));

        // Verify that max-attempts and stale rows are now 'abandoned'
        let obl_max: DeliveryObligation = service
            .get_obligation("obl-max-attempts")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(obl_max.state, "abandoned");

        let obl_stale: DeliveryObligation =
            service.get_obligation("obl-stale").await.unwrap().unwrap();
        assert_eq!(obl_stale.state, "abandoned");

        // Verify claimed rows now have owner_pid set to current_pid and attempts incremented
        let obl_p: DeliveryObligation = service
            .get_obligation("obl-pending-dead")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(obl_p.owner_pid, Some(current_pid));
        assert_eq!(obl_p.attempts, 1);
    }

    fn batch_event(session: &SessionKey, id: &str) -> InboundEvent {
        InboundEvent::message(session.clone(), id, "chunk")
    }

    #[tokio::test]
    async fn failed_batch_is_reclaimed_and_seen_constituent_does_not_drop_the_rest() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = DeliveryLedgerService::new(db.pool().clone());
        let session = test_session();
        let event = batch_event(&session, "30");
        let ids = vec![
            "discord:10".to_string(),
            "discord:20".to_string(),
            "discord:30".to_string(),
        ];

        assert!(service
            .record_incoming_with_constituents(&event, "discord:30", &ids)
            .await
            .unwrap());
        // Live claim -> duplicate.
        assert!(!service
            .record_incoming_with_constituents(&event, "discord:30", &ids)
            .await
            .unwrap());
        // A failed batch (parent and constituents) must be retryable.
        service.mark_failed("discord:30", "boom").await.unwrap();
        assert!(service
            .record_incoming_with_constituents(&event, "discord:30", &ids)
            .await
            .unwrap());
        service.mark_delivered("discord:30").await.unwrap();

        // A later batch containing one already-delivered constituent still processes.
        let next = batch_event(&session, "50");
        let next_ids = vec![
            "discord:20".to_string(),
            "discord:40".to_string(),
            "discord:50".to_string(),
        ];
        assert!(service
            .record_incoming_with_constituents(&next, "discord:50", &next_ids)
            .await
            .unwrap());
        assert!(service.get("discord:40").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn legacy_unstamped_obligation_with_current_pid_is_not_skipped_forever() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = DeliveryLedgerService::new(db.pool().clone());
        let session = test_session();
        service
            .record_obligation("obl-legacy", &session, "legacy")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET owner_started_at = NULL, owner_pid = ?")
            .bind(std::process::id() as i64)
            .execute(db.pool())
            .await
            .unwrap();
        let claimed = service.sweep_recoverable(3, 86400).await.unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, "obl-legacy");
    }

    struct FlakyDispatcher;

    #[async_trait::async_trait]
    impl crate::OutboundDispatcher for FlakyDispatcher {
        async fn dispatch(&self, _action: crate::OutboundAction) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn recovery_continues_past_one_failing_obligation_and_reports_the_error() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let service = DeliveryLedgerService::new(db.pool().clone());
        let session = test_session();
        for id in ["obl-a", "obl-b"] {
            service.record_obligation(id, &session, id).await.unwrap();
        }
        let previous =
            DateTime::parse_from_rfc3339(process_started_at()).unwrap() - Duration::days(1);
        sqlx::query("UPDATE delivery_obligations SET owner_started_at = ?")
            .bind(previous.to_rfc3339())
            .execute(db.pool())
            .await
            .unwrap();
        // The first obligation's attempting update is rejected; the second must still recover.
        sqlx::query(
            "CREATE TRIGGER fail_a BEFORE UPDATE ON delivery_obligations
             WHEN NEW.id = 'obl-a' AND NEW.state = 'attempting'
             BEGIN SELECT RAISE(ABORT, 'FAULT_A'); END;",
        )
        .execute(db.pool())
        .await
        .unwrap();
        let result =
            recover_pending_delivery_obligations(db.pool(), std::sync::Arc::new(FlakyDispatcher))
                .await;
        assert!(result.unwrap_err().to_string().contains("FAULT_A"));
        let b = service.get_obligation("obl-b").await.unwrap().unwrap();
        assert_eq!(b.state, "delivered");
    }
}

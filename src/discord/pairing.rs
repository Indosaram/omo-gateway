use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::{OmonError, Result};

pub const PAIRING_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
pub const CODE_LENGTH: usize = 8;
pub const CODE_TTL_SECONDS: i64 = 3600; // 1 hour
pub const RATE_LIMIT_SECONDS: i64 = 600; // 10 minutes
pub const LOCKOUT_DURATION_SECONDS: i64 = 900; // 15 minutes
pub const MAX_FAILED_ATTEMPTS: i64 = 5;
pub const MAX_PENDING_CODES: usize = 100;
pub const DEFAULT_PLATFORM_KEY: &str = "discord";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingOutcome {
    Success { user_id: u64 },
    InvalidCode,
    Expired,
    LockedOut,
}

#[derive(sqlx::FromRow)]
struct PairingCodeRow {
    code: String,
    user_id: String,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    attempts: i64,
}

#[derive(Clone)]
pub struct PairingStore {
    pool: SqlitePool,
    paired_cache: Arc<RwLock<HashSet<u64>>>,
}

impl PairingStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            paired_cache: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Initializes the in-memory cache of paired users from SQLite.
    pub async fn init_cache(&self) -> Result<()> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT user_id FROM paired_users")
            .fetch_all(&self.pool)
            .await?;
        let mut set = self
            .paired_cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        set.clear();
        for (uid_str,) in rows {
            if let Ok(uid) = uid_str.parse::<u64>() {
                set.insert(uid);
            }
        }
        Ok(())
    }

    /// Fast in-memory check if a user is paired.
    pub async fn is_user_paired(&self, user_id: u64) -> bool {
        self.is_user_paired_sync(user_id)
    }

    /// Synchronous read check against the paired cache. The lock is a std
    /// `RwLock` that is never held across an await, so this cannot spuriously
    /// fail while a pairing approval updates the cache.
    pub fn is_user_paired_sync(&self, user_id: u64) -> bool {
        self.paired_cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&user_id)
    }

    /// Returns a list of all currently paired user IDs (async).
    pub async fn get_paired_user_ids(&self) -> Vec<u64> {
        self.get_paired_user_ids_sync()
    }

    /// Returns a list of all currently paired user IDs (sync).
    pub fn get_paired_user_ids_sync(&self) -> Vec<u64> {
        self.paired_cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .copied()
            .collect()
    }

    /// Normalizes code formatting (removes dashes/spaces, uppercase).
    pub fn normalize_code(raw: &str) -> String {
        raw.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect()
    }

    /// Formats an 8-char code as `XXXX-XXXX`.
    pub fn format_code(raw: &str) -> String {
        let normalized = Self::normalize_code(raw);
        if normalized.len() == 8 {
            format!("{}-{}", &normalized[..4], &normalized[4..])
        } else {
            normalized
        }
    }

    /// Generates a secure random 8-character code from the unambiguous alphabet.
    pub fn generate_raw_code() -> String {
        use uuid::Uuid;
        let bytes = Uuid::new_v4().into_bytes();
        let mut code = String::with_capacity(CODE_LENGTH);
        for byte in bytes.iter().take(CODE_LENGTH) {
            let idx = (*byte as usize) % PAIRING_ALPHABET.len();
            code.push(PAIRING_ALPHABET[idx] as char);
        }
        code
    }

    /// Checks whether the platform is currently locked out at the given timestamp within a transaction.
    async fn is_platform_locked_tx(
        tx: &mut Transaction<'_, Sqlite>,
        platform_key: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let row: Option<(DateTime<Utc>, i64)> = sqlx::query_as(
            "SELECT locked_until, failed_attempts FROM pairing_platform_lockout WHERE platform_key = ?",
        )
        .bind(platform_key)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((locked_until, failed_attempts)) = row {
            if failed_attempts >= MAX_FAILED_ATTEMPTS && now < locked_until {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Checks whether the platform is currently locked out at the given timestamp.
    pub async fn is_platform_locked_at(&self, now: DateTime<Utc>) -> Result<bool> {
        let row: Option<(DateTime<Utc>, i64)> = sqlx::query_as(
            "SELECT locked_until, failed_attempts FROM pairing_platform_lockout WHERE platform_key = ?",
        )
        .bind(DEFAULT_PLATFORM_KEY)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((locked_until, failed_attempts)) = row {
            if failed_attempts >= MAX_FAILED_ATTEMPTS && now < locked_until {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Checks whether the platform is currently locked out.
    pub async fn is_platform_locked(&self) -> Result<bool> {
        self.is_platform_locked_at(Utc::now()).await
    }

    /// Internal helper to record a failed attempt atomically within a transaction.
    /// Uses an atomic conditional UPSERT to eliminate lost increments and unique constraint races.
    async fn record_failed_attempt_tx(
        tx: &mut Transaction<'_, Sqlite>,
        platform_key: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let deadline = now + chrono::Duration::seconds(LOCKOUT_DURATION_SECONDS);
        sqlx::query(
            "INSERT INTO pairing_platform_lockout (platform_key, locked_until, failed_attempts)
             VALUES (?, ?, 1)
             ON CONFLICT(platform_key) DO UPDATE SET
                 failed_attempts = CASE
                     WHEN strftime('%s', ?) >= strftime('%s', pairing_platform_lockout.locked_until) AND pairing_platform_lockout.failed_attempts >= ? THEN 1
                     ELSE pairing_platform_lockout.failed_attempts + 1
                 END,
                 locked_until = CASE
                     WHEN strftime('%s', ?) >= strftime('%s', pairing_platform_lockout.locked_until) AND pairing_platform_lockout.failed_attempts >= ? THEN ?
                     WHEN pairing_platform_lockout.failed_attempts + 1 >= ? THEN ?
                     ELSE pairing_platform_lockout.locked_until
                 END
             WHERE platform_key = ?",
        )
        .bind(platform_key)
        .bind(now)
        .bind(now)
        .bind(MAX_FAILED_ATTEMPTS)
        .bind(now)
        .bind(MAX_FAILED_ATTEMPTS)
        .bind(now)
        .bind(MAX_FAILED_ATTEMPTS)
        .bind(deadline)
        .bind(platform_key)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn reset_failed_attempts_tx(
        tx: &mut Transaction<'_, Sqlite>,
        platform_key: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO pairing_platform_lockout (platform_key, locked_until, failed_attempts) VALUES (?, ?, 0)
             ON CONFLICT(platform_key) DO UPDATE SET failed_attempts = 0, locked_until = excluded.locked_until",
        )
        .bind(platform_key)
        .bind(now)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Claims the notification slot unless `user_key` was notified within the rate-limit window.
    async fn claim_notification_tx(
        tx: &mut Transaction<'_, Sqlite>,
        user_key: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let claim_res = sqlx::query(
            "INSERT INTO pairing_notifications (user_id, last_notified_at)
             VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_notified_at = excluded.last_notified_at
             WHERE (strftime('%s', ?) - strftime('%s', pairing_notifications.last_notified_at)) >= ?",
        )
        .bind(user_key)
        .bind(now)
        .bind(now)
        .bind(RATE_LIMIT_SECONDS)
        .execute(&mut **tx)
        .await?;
        Ok(claim_res.rows_affected() > 0)
    }

    /// The notification table exists only to rate-limit; a row older than
    /// the window can never throttle again, so drop it instead of letting
    /// an unauthenticated population grow the table without bound.
    async fn prune_notifications_tx(
        tx: &mut Transaction<'_, Sqlite>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM pairing_notifications
             WHERE (strftime('%s', ?) - strftime('%s', last_notified_at)) > ?",
        )
        .bind(now)
        .bind(RATE_LIMIT_SECONDS)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Records a failed approval attempt and triggers platform lockout if MAX_FAILED_ATTEMPTS reached.
    pub async fn record_failed_attempt_at(&self, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        Self::record_failed_attempt_tx(&mut tx, DEFAULT_PLATFORM_KEY, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Cleans up expired pairing codes within a transaction.
    async fn cleanup_expired_codes_tx(
        tx: &mut Transaction<'_, Sqlite>,
        now: DateTime<Utc>,
    ) -> Result<u64> {
        let result = sqlx::query("DELETE FROM pairing_codes WHERE expires_at <= ?")
            .bind(now)
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected())
    }

    /// Cleans up expired pairing codes up to the given timestamp.
    pub async fn cleanup_expired_codes_at(&self, now: DateTime<Utc>) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let affected = Self::cleanup_expired_codes_tx(&mut tx, now).await?;
        tx.commit().await?;
        Ok(affected)
    }

    /// Cleans up expired pairing codes globally.
    pub async fn cleanup_expired_codes(&self) -> Result<u64> {
        self.cleanup_expired_codes_at(Utc::now()).await
    }

    /// Internal transaction-scoped helper for requesting/issuing a pairing code.
    /// Runs all operations within the provided transaction to prevent pool re-entrancy and races.
    async fn request_pairing_code_tx(
        tx: &mut Transaction<'_, Sqlite>,
        platform_key: &str,
        user_id_str: &str,
        now: DateTime<Utc>,
    ) -> Result<String> {
        if Self::is_platform_locked_tx(tx, platform_key, now).await? {
            return Err(OmonError::Approval(
                "Pairing is locked out due to too many failed attempts".into(),
            ));
        }

        // 1. Cleanup expired codes globally, propagating DB errors
        Self::cleanup_expired_codes_tx(tx, now).await?;

        // 2. Check if there is already an active, unexpired code for this user
        let existing: Option<(String, DateTime<Utc>, DateTime<Utc>, i64)> = sqlx::query_as(
            "SELECT code, created_at, expires_at, attempts FROM pairing_codes WHERE user_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id_str)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((code, _created_at, expires_at, attempts)) = existing {
            if attempts >= MAX_FAILED_ATTEMPTS {
                // If platform lockout has passed, this code row is expired/locked; delete it and allow new code issuance.
                sqlx::query("DELETE FROM pairing_codes WHERE user_id = ?")
                    .bind(user_id_str)
                    .execute(&mut **tx)
                    .await?;
            } else if expires_at > now {
                return Ok(Self::format_code(&code));
            }
        }

        // Delete any previous code for this user before enforcing capacity and inserting a new code
        sqlx::query("DELETE FROM pairing_codes WHERE user_id = ?")
            .bind(user_id_str)
            .execute(&mut **tx)
            .await?;

        // 3. Enforce bounded pending count, propagating count error
        let active_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")
                .bind(now)
                .fetch_one(&mut **tx)
                .await?;

        if active_count as usize >= MAX_PENDING_CODES {
            let to_evict = (active_count as usize - MAX_PENDING_CODES + 1) as i64;
            sqlx::query(
                "DELETE FROM pairing_codes WHERE code IN (
                    SELECT code FROM pairing_codes WHERE expires_at > ? ORDER BY created_at ASC LIMIT ?
                )",
            )
            .bind(now)
            .bind(to_evict)
            .execute(&mut **tx)
            .await?;
        }

        let raw_code = Self::generate_raw_code();
        let expires_at = now + chrono::Duration::seconds(CODE_TTL_SECONDS);

        sqlx::query(
            "INSERT INTO pairing_codes (code, user_id, created_at, expires_at, attempts) VALUES (?, ?, ?, ?, 0)",
        )
        .bind(&raw_code)
        .bind(user_id_str)
        .bind(now)
        .bind(expires_at)
        .execute(&mut **tx)
        .await
        .map_err(|e| OmonError::Database(format!("failed to store pairing code: {e}")))?;

        Ok(Self::format_code(&raw_code))
    }

    /// Requests a one-time pairing code for an unauthorized user at the specified timestamp.
    /// Cleans up expired codes, enforces bounded pending capacity, and respects platform lockout.
    pub async fn request_pairing_code_at(
        &self,
        user_id: u64,
        now: DateTime<Utc>,
    ) -> Result<String> {
        let mut tx = self.pool.begin().await?;
        let code =
            Self::request_pairing_code_tx(&mut tx, DEFAULT_PLATFORM_KEY, &user_id.to_string(), now)
                .await?;
        tx.commit().await?;
        Ok(code)
    }

    /// Requests a one-time pairing code for an unauthorized user.
    /// Reuses an unexpired active code if within the rate-limit window.
    pub async fn request_pairing_code(&self, user_id: u64) -> Result<String> {
        self.request_pairing_code_at(user_id, Utc::now()).await
    }

    /// Checks and records DM notification delivery with rate limiting.
    /// Returns Ok(Some(code)) if notification should be sent, or Ok(None) if throttled/locked out.
    ///
    /// Clarification of attempt vs confirmed delivery timestamp:
    /// `pairing_notifications.last_notified_at` records the notification decision attempt timestamp
    /// (reservation) to linearize concurrent incoming messages prior to outbound Discord transport.
    /// Confirmed delivery timestamps can be updated via `record_confirmed_delivery_at`.
    pub async fn check_and_record_notification_at(
        &self,
        user_id: u64,
        now: DateTime<Utc>,
    ) -> Result<Option<String>> {
        let mut tx = self.pool.begin().await?;

        if Self::is_platform_locked_tx(&mut tx, DEFAULT_PLATFORM_KEY, now).await? {
            tx.commit().await?;
            return Ok(None);
        }

        let user_id_str = user_id.to_string();

        let paired: Option<(String,)> =
            sqlx::query_as("SELECT user_id FROM paired_users WHERE user_id = ?")
                .bind(&user_id_str)
                .fetch_optional(&mut *tx)
                .await?;

        if paired.is_some() {
            tx.commit().await?;
            return Ok(None);
        }

        // Atomically check rate limit and claim notification attempt slot.
        if !Self::claim_notification_tx(&mut tx, &user_id_str, now).await? {
            tx.commit().await?;
            return Ok(None);
        }

        // Notification slot claimed; obtain active/new pairing code within the same transaction.
        // Propagate any database error instead of swallowing it as throttled success.
        let code =
            Self::request_pairing_code_tx(&mut tx, DEFAULT_PLATFORM_KEY, &user_id_str, now).await?;

        Self::prune_notifications_tx(&mut tx, now).await?;

        tx.commit().await?;

        Ok(Some(code))
    }

    /// Confirms outbound delivery of a pairing notification at the specified timestamp.
    /// Distinguishes the pre-transport attempt/throttle reservation timestamp from confirmed transport delivery.
    pub async fn record_confirmed_delivery_at(
        &self,
        user_id: u64,
        delivered_at: DateTime<Utc>,
    ) -> Result<()> {
        let user_id_str = user_id.to_string();
        sqlx::query(
            "INSERT INTO pairing_notifications (user_id, last_notified_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_notified_at = excluded.last_notified_at",
        )
        .bind(&user_id_str)
        .bind(delivered_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Checks and records DM notification delivery with rate limiting.
    pub async fn check_and_record_notification(&self, user_id: u64) -> Result<Option<String>> {
        self.check_and_record_notification_at(user_id, Utc::now())
            .await
    }

    /// Approves a pairing code entered by an operator at the specified timestamp.
    pub async fn approve_code_at(
        &self,
        input_code: &str,
        _operator_id: u64,
        now: DateTime<Utc>,
    ) -> Result<PairingOutcome> {
        let normalized = Self::normalize_code(input_code);
        if normalized.is_empty() {
            return Ok(PairingOutcome::InvalidCode);
        }

        let mut tx = self.pool.begin().await?;

        if Self::is_platform_locked_tx(&mut tx, DEFAULT_PLATFORM_KEY, now).await? {
            tx.commit().await?;
            return Ok(PairingOutcome::LockedOut);
        }

        let record: Option<PairingCodeRow> = sqlx::query_as(
            "SELECT code, user_id, created_at, expires_at, attempts FROM pairing_codes WHERE code = ?",
        )
        .bind(&normalized)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = record else {
            // An unknown code only feeds the platform-wide lockout; it must not
            // burn attempts on unrelated pending codes.
            Self::record_failed_attempt_tx(&mut tx, DEFAULT_PLATFORM_KEY, now).await?;
            tx.commit().await?;
            return Ok(PairingOutcome::InvalidCode);
        };

        if row.attempts >= MAX_FAILED_ATTEMPTS {
            tx.commit().await?;
            return Ok(PairingOutcome::LockedOut);
        }

        if now > row.expires_at {
            // Delete expired code
            sqlx::query("DELETE FROM pairing_codes WHERE code = ?")
                .bind(&row.code)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(PairingOutcome::Expired);
        }

        let Ok(user_id) = row.user_id.parse::<u64>() else {
            tx.commit().await?;
            return Ok(PairingOutcome::InvalidCode);
        };

        // Atomically claim the code by deleting it.
        // If concurrent approvals race for the same code, exactly one claims it.
        let delete_res = sqlx::query("DELETE FROM pairing_codes WHERE code = ?")
            .bind(&row.code)
            .execute(&mut *tx)
            .await?;

        if delete_res.rows_affected() == 0 {
            tx.commit().await?;
            return Ok(PairingOutcome::InvalidCode);
        }

        // Insert into paired_users
        sqlx::query(
            "INSERT INTO paired_users (user_id, paired_at) VALUES (?, ?) ON CONFLICT(user_id) DO NOTHING",
        )
        .bind(&row.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| OmonError::Database(format!("failed to record paired user: {e}")))?;

        // Reset platform failed attempts on successful pairing
        Self::reset_failed_attempts_tx(&mut tx, DEFAULT_PLATFORM_KEY, now).await?;

        // Commit transaction before modifying in-memory cache
        tx.commit().await?;

        // Cache callbacks only after commit
        self.paired_cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(user_id);

        // The rate-limit row for this user served its purpose; drop it so the
        // notification table does not accumulate one row per paired user.
        if let Err(error) = sqlx::query("DELETE FROM pairing_notifications WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
        {
            tracing::warn!(%error, user_id, "failed to prune pairing notification row after pairing");
        }

        Ok(PairingOutcome::Success { user_id })
    }

    /// Approves a pairing code entered by an operator.
    pub async fn approve_code(&self, input_code: &str, operator_id: u64) -> Result<PairingOutcome> {
        self.approve_code_at(input_code, operator_id, Utc::now())
            .await
    }
}

pub const SLACK_PLATFORM_KEY: &str = "slack";
/// Prefix for Slack user ids in the shared `pairing_codes`/`pairing_notifications` tables, so
/// they never collide with Discord snowflakes and Discord `/pair` never accepts them.
const SLACK_USER_PREFIX: &str = "slack:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlackPairingOutcome {
    Success { user_id: String },
    InvalidCode,
    Expired,
    LockedOut,
}

/// `PairingStore` for Slack string user ids: shares code generation, rate limiting and
/// lockout rules (under platform key `slack`) and records approvals in `slack_paired_users`.
#[derive(Clone)]
pub struct SlackPairingStore {
    pool: SqlitePool,
    paired_cache: Arc<RwLock<HashSet<String>>>,
}

impl SlackPairingStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            paired_cache: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    fn code_user_key(user_id: &str) -> String {
        format!("{SLACK_USER_PREFIX}{user_id}")
    }

    pub async fn init_cache(&self) -> Result<()> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT user_id FROM slack_paired_users")
            .fetch_all(&self.pool)
            .await?;
        let mut set = self
            .paired_cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        set.clear();
        set.extend(rows.into_iter().map(|(user_id,)| user_id));
        Ok(())
    }

    pub async fn is_user_paired(&self, user_id: &str) -> bool {
        self.is_user_paired_sync(user_id)
    }

    pub fn is_user_paired_sync(&self, user_id: &str) -> bool {
        self.paired_cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(user_id)
    }

    pub async fn get_paired_user_ids(&self) -> Vec<String> {
        self.get_paired_user_ids_sync()
    }

    pub fn get_paired_user_ids_sync(&self) -> Vec<String> {
        self.paired_cache
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    pub async fn is_platform_locked_at(&self, now: DateTime<Utc>) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let locked = PairingStore::is_platform_locked_tx(&mut tx, SLACK_PLATFORM_KEY, now).await?;
        tx.commit().await?;
        Ok(locked)
    }

    pub async fn is_platform_locked(&self) -> Result<bool> {
        self.is_platform_locked_at(Utc::now()).await
    }

    pub async fn record_failed_attempt_at(&self, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        PairingStore::record_failed_attempt_tx(&mut tx, SLACK_PLATFORM_KEY, now).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn request_pairing_code_at(
        &self,
        user_id: &str,
        now: DateTime<Utc>,
    ) -> Result<String> {
        let mut tx = self.pool.begin().await?;
        let code = PairingStore::request_pairing_code_tx(
            &mut tx,
            SLACK_PLATFORM_KEY,
            &Self::code_user_key(user_id),
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(code)
    }

    pub async fn request_pairing_code(&self, user_id: &str) -> Result<String> {
        self.request_pairing_code_at(user_id, Utc::now()).await
    }

    /// `Ok(Some(code))` when a pairing prompt should be sent; `None` when paired, throttled or locked out.
    pub async fn check_and_record_notification_at(
        &self,
        user_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<String>> {
        let mut tx = self.pool.begin().await?;
        if PairingStore::is_platform_locked_tx(&mut tx, SLACK_PLATFORM_KEY, now).await? {
            tx.commit().await?;
            return Ok(None);
        }
        let paired: Option<(String,)> =
            sqlx::query_as("SELECT user_id FROM slack_paired_users WHERE user_id = ?")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let user_key = Self::code_user_key(user_id);
        if paired.is_some() || !PairingStore::claim_notification_tx(&mut tx, &user_key, now).await?
        {
            tx.commit().await?;
            return Ok(None);
        }
        let code =
            PairingStore::request_pairing_code_tx(&mut tx, SLACK_PLATFORM_KEY, &user_key, now)
                .await?;
        PairingStore::prune_notifications_tx(&mut tx, now).await?;
        tx.commit().await?;
        Ok(Some(code))
    }

    pub async fn check_and_record_notification(&self, user_id: &str) -> Result<Option<String>> {
        self.check_and_record_notification_at(user_id, Utc::now())
            .await
    }

    pub async fn record_confirmed_delivery_at(
        &self,
        user_id: &str,
        delivered_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO pairing_notifications (user_id, last_notified_at) VALUES (?, ?)
             ON CONFLICT(user_id) DO UPDATE SET last_notified_at = excluded.last_notified_at",
        )
        .bind(Self::code_user_key(user_id))
        .bind(delivered_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn approve_code_at(
        &self,
        input_code: &str,
        operator_id: &str,
        now: DateTime<Utc>,
    ) -> Result<SlackPairingOutcome> {
        let normalized = PairingStore::normalize_code(input_code);
        if normalized.is_empty() {
            return Ok(SlackPairingOutcome::InvalidCode);
        }
        let mut tx = self.pool.begin().await?;
        if PairingStore::is_platform_locked_tx(&mut tx, SLACK_PLATFORM_KEY, now).await? {
            tx.commit().await?;
            return Ok(SlackPairingOutcome::LockedOut);
        }
        let record: Option<PairingCodeRow> = sqlx::query_as(
            "SELECT code, user_id, created_at, expires_at, attempts FROM pairing_codes WHERE code = ?",
        )
        .bind(&normalized)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((row, user_id)) = record.and_then(|row| {
            let user_id = row.user_id.strip_prefix(SLACK_USER_PREFIX)?.to_owned();
            Some((row, user_id))
        }) else {
            PairingStore::record_failed_attempt_tx(&mut tx, SLACK_PLATFORM_KEY, now).await?;
            tx.commit().await?;
            return Ok(SlackPairingOutcome::InvalidCode);
        };
        if row.attempts >= MAX_FAILED_ATTEMPTS {
            tx.commit().await?;
            return Ok(SlackPairingOutcome::LockedOut);
        }
        if now > row.expires_at {
            sqlx::query("DELETE FROM pairing_codes WHERE code = ?")
                .bind(&row.code)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(SlackPairingOutcome::Expired);
        }
        let claimed = sqlx::query("DELETE FROM pairing_codes WHERE code = ?")
            .bind(&row.code)
            .execute(&mut *tx)
            .await?;
        if claimed.rows_affected() == 0 {
            tx.commit().await?;
            return Ok(SlackPairingOutcome::InvalidCode);
        }
        sqlx::query(
            "INSERT INTO slack_paired_users (user_id, paired_at, operator_id) VALUES (?, ?, ?)
             ON CONFLICT(user_id) DO NOTHING",
        )
        .bind(&user_id)
        .bind(now)
        .bind(operator_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| OmonError::Database(format!("failed to record paired slack user: {e}")))?;
        PairingStore::reset_failed_attempts_tx(&mut tx, SLACK_PLATFORM_KEY, now).await?;
        tx.commit().await?;

        self.paired_cache
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(user_id.clone());
        if let Err(error) = sqlx::query("DELETE FROM pairing_notifications WHERE user_id = ?")
            .bind(&row.user_id)
            .execute(&self.pool)
            .await
        {
            tracing::warn!(%error, %user_id, "failed to prune slack pairing notification row after pairing");
        }
        Ok(SlackPairingOutcome::Success { user_id })
    }

    pub async fn approve_code(
        &self,
        input_code: &str,
        operator_id: &str,
    ) -> Result<SlackPairingOutcome> {
        self.approve_code_at(input_code, operator_id, Utc::now())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_format_and_normalization() {
        let code = "ABCD2345";
        let formatted = PairingStore::format_code(code);
        assert_eq!(formatted, "ABCD-2345");

        let normalized = PairingStore::normalize_code("abcd-2345");
        assert_eq!(normalized, "ABCD2345");
    }

    #[test]
    fn test_generate_raw_code_alphabet_and_length() {
        for _ in 0..100 {
            let code = PairingStore::generate_raw_code();
            assert_eq!(code.len(), CODE_LENGTH);
            for byte in code.bytes() {
                assert!(PAIRING_ALPHABET.contains(&byte), "byte must be in alphabet");
                // Confirm no 0, O, 1, I
                assert_ne!(byte, b'0');
                assert_ne!(byte, b'O');
                assert_ne!(byte, b'1');
                assert_ne!(byte, b'I');
            }
        }
    }

    #[tokio::test]
    async fn test_pairing_lifecycle_and_approval() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool);
        store.init_cache().await.unwrap();

        let user_id = 987654321_u64;
        assert!(!store.is_user_paired(user_id).await);

        // Request code
        let code = store.request_pairing_code(user_id).await.unwrap();
        assert_eq!(code.len(), 9); // XXXX-XXXX

        // Approve with valid code
        let outcome = store.approve_code(&code, 11111).await.unwrap();
        assert_eq!(outcome, PairingOutcome::Success { user_id });

        // Now user should be paired
        assert!(store.is_user_paired(user_id).await);
        assert!(store.is_user_paired_sync(user_id));

        // Approving again with same code should return InvalidCode (consumed)
        let second_try = store.approve_code(&code, 11111).await.unwrap();
        assert_eq!(second_try, PairingOutcome::InvalidCode);
    }

    #[tokio::test]
    async fn test_pairing_expiry_and_lockout() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        // Test expired code
        let now = Utc::now();
        let expired_time = now - chrono::Duration::hours(2);
        sqlx::query(
            "INSERT INTO pairing_codes (code, user_id, created_at, expires_at, attempts) VALUES ('EXPIRED1', '123456789', ?, ?, 0)"
        )
        .bind(expired_time - chrono::Duration::hours(1))
        .bind(expired_time)
        .execute(&pool)
        .await
        .unwrap();

        let outcome = store.approve_code("EXPIRED1", 11111).await.unwrap();
        assert_eq!(outcome, PairingOutcome::Expired);

        // Test lockout after 5 attempts
        sqlx::query(
            "INSERT INTO pairing_codes (code, user_id, created_at, expires_at, attempts) VALUES ('LOCKOUT1', '123456789', ?, ?, 5)"
        )
        .bind(now)
        .bind(now + chrono::Duration::hours(1))
        .execute(&pool)
        .await
        .unwrap();

        let outcome = store.approve_code("LOCKOUT1", 11111).await.unwrap();
        assert_eq!(outcome, PairingOutcome::LockedOut);
    }

    #[tokio::test]
    async fn pairing_throttles_notifications_on_repeated_rejected_dm() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let user_id = 999888777_u64;
        let t0 = Utc::now();

        let mut sends = 0;
        for _ in 0..2 {
            if let Ok(Some(_code)) = store.check_and_record_notification_at(user_id, t0).await {
                sends += 1;
            }
        }
        assert_eq!(
            sends, 1,
            "repeated rejected DM at same timestamp must send only 1 notification, got {sends}"
        );
    }

    #[tokio::test]
    async fn pairing_preserves_lockout_across_fresh_requests() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let user_id = 424242_u64;
        let t0 = Utc::now();
        let _code = store.request_pairing_code_at(user_id, t0).await.unwrap();

        // 5 invalid operator approvals
        for _ in 0..5 {
            let outcome = store
                .approve_code_at("WRONG_CODE", 11111, t0)
                .await
                .unwrap();
            assert_eq!(outcome, PairingOutcome::InvalidCode);
        }

        // Fresh request by user while locked out
        let t1 = t0 + chrono::Duration::seconds(10);
        let fresh_dm = store
            .check_and_record_notification_at(user_id, t1)
            .await
            .unwrap();
        assert_eq!(
            fresh_dm, None,
            "fresh DM must not issue code while platform is locked out"
        );

        let fresh_result = store.request_pairing_code_at(user_id, t1).await;
        assert!(
            fresh_result.is_err(),
            "fresh request while locked out must fail"
        );

        let attempts: i64 =
            sqlx::query_scalar("SELECT attempts FROM pairing_codes WHERE user_id = ?")
                .bind(user_id.to_string())
                .fetch_optional(&pool)
                .await
                .unwrap()
                .unwrap_or(0);

        assert_eq!(
            attempts, 0,
            "unknown-code failures must not burn attempts on unrelated pending codes"
        );
        assert!(
            store.is_platform_locked_at(t1).await.unwrap(),
            "platform must remain locked until deadline"
        );
    }

    #[tokio::test]
    async fn operator_typos_do_not_invalidate_pending_codes_after_lockout() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool);
        store.init_cache().await.unwrap();

        let user_id = 31337_u64;
        let t0 = Utc::now();
        let code = store.request_pairing_code_at(user_id, t0).await.unwrap();
        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert_eq!(
                store.approve_code_at("TYPO2222", 1, t0).await.unwrap(),
                PairingOutcome::InvalidCode
            );
        }
        let after = t0 + chrono::Duration::seconds(LOCKOUT_DURATION_SECONDS + 1);
        assert_eq!(
            store.approve_code_at(&code, 1, after).await.unwrap(),
            PairingOutcome::Success { user_id }
        );
    }

    #[tokio::test]
    async fn pairing_throttles_notifications_and_preserves_lockout() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let user_id = 123456789_u64;
        let t0 = Utc::now();

        // Part 1: Repeated rejected DM at same injected timestamp => 1 send
        let mut sends = 0;
        let mut decisions = Vec::new();
        for _ in 0..2 {
            let decision = store
                .check_and_record_notification_at(user_id, t0)
                .await
                .unwrap();
            if decision.is_some() {
                sends += 1;
            }
            decisions.push(decision);
        }
        assert_eq!(
            sends, 1,
            "repeated rejected DM at same timestamp must send only 1 notification, got {sends}"
        );
        assert!(decisions[0].is_some(), "first DM must yield a pairing code");
        assert_eq!(
            decisions[1], None,
            "second DM at same timestamp must be throttled"
        );

        // Advance clock past rate limit window (600s) => second notification permitted
        let t1 = t0 + chrono::Duration::seconds(RATE_LIMIT_SECONDS + 1);
        let second_window = store
            .check_and_record_notification_at(user_id, t1)
            .await
            .unwrap();
        assert!(
            second_window.is_some(),
            "DM after rate limit window must be notified"
        );

        // Part 2: Five invalid operator approvals then fresh DM => locked until deadline
        let active_code = store.request_pairing_code_at(user_id, t1).await.unwrap();
        for _ in 0..5 {
            let outcome = store
                .approve_code_at("WRONG_CODE", 11111, t1)
                .await
                .unwrap();
            assert_eq!(outcome, PairingOutcome::InvalidCode);
        }

        // 6th approval attempt must be locked out
        let locked_check = store
            .approve_code_at("WRONG_CODE", 11111, t1)
            .await
            .unwrap();
        assert_eq!(locked_check, PairingOutcome::LockedOut);

        // Approval of the actual code must also be locked out
        let active_check = store
            .approve_code_at(&active_code, 11111, t1)
            .await
            .unwrap();
        assert_eq!(active_check, PairingOutcome::LockedOut);

        // Fresh DM at t1 + 10s must remain locked out; cannot issue code with attempts=0
        let t_dm = t1 + chrono::Duration::seconds(10);
        let dm_decision = store
            .check_and_record_notification_at(user_id, t_dm)
            .await
            .unwrap();
        assert_eq!(
            dm_decision, None,
            "fresh DM must not issue prompt while locked out"
        );

        let fresh_request = store.request_pairing_code_at(user_id, t_dm).await;
        assert!(
            fresh_request.is_err(),
            "requesting code while locked out must fail"
        );

        let attempts: i64 =
            sqlx::query_scalar("SELECT attempts FROM pairing_codes WHERE user_id = ?")
                .bind(user_id.to_string())
                .fetch_optional(&pool)
                .await
                .unwrap()
                .unwrap_or(0);

        assert_eq!(
            attempts, 0,
            "unknown-code failures must not burn attempts on the pending code"
        );
        assert!(
            store.is_platform_locked_at(t_dm).await.unwrap(),
            "platform must remain locked until deadline"
        );

        // Advance clock past platform lockout deadline (900s)
        let t_unlocked = t1 + chrono::Duration::seconds(LOCKOUT_DURATION_SECONDS + 1);
        assert!(
            !store.is_platform_locked_at(t_unlocked).await.unwrap(),
            "platform lockout must expire after deadline"
        );

        // Fresh DM after deadline succeeds and issues code
        let unlocked_dm = store
            .check_and_record_notification_at(user_id, t_unlocked)
            .await
            .unwrap();
        assert!(
            unlocked_dm.is_some(),
            "fresh DM after lockout deadline must succeed"
        );
    }

    #[tokio::test]
    async fn pairing_bounded_pending_count_and_expiry_cleanup() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let t0 = Utc::now();

        // Insert an expired code manually
        let expired_time = t0 - chrono::Duration::hours(2);
        sqlx::query(
            "INSERT INTO pairing_codes (code, user_id, created_at, expires_at, attempts) VALUES ('OLD_EXP1', '99999', ?, ?, 0)",
        )
        .bind(expired_time - chrono::Duration::hours(1))
        .bind(expired_time)
        .execute(&pool)
        .await
        .unwrap();

        // Cleanup expired codes
        let cleaned = store.cleanup_expired_codes_at(t0).await.unwrap();
        assert_eq!(cleaned, 1);

        // Verify bounded capacity: insert MAX_PENDING_CODES
        for i in 0..MAX_PENDING_CODES {
            let uid = 10000 + i as u64;
            let _ = store.request_pairing_code_at(uid, t0).await.unwrap();
        }

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")
                .bind(t0)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, MAX_PENDING_CODES as i64);

        // Request one more code -> oldest should be evicted, keeping count at MAX_PENDING_CODES
        let _ = store.request_pairing_code_at(999999, t0).await.unwrap();
        let count_after: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")
                .bind(t0)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count_after, MAX_PENDING_CODES as i64);
    }

    #[tokio::test]
    async fn test_atomic_concurrent_notifications_single_send_decision() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store1 = PairingStore::new(pool.clone());
        let store2 = PairingStore::new(pool.clone());
        store1.init_cache().await.unwrap();

        let user_id = 777111222_u64;
        let t0 = Utc::now();

        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let b1 = barrier.clone();
        let b2 = barrier.clone();
        let s1 = store1.clone();
        let s2 = store2.clone();

        let h1 = tokio::spawn(async move {
            b1.wait().await;
            s1.check_and_record_notification_at(user_id, t0).await
        });
        let h2 = tokio::spawn(async move {
            b2.wait().await;
            s2.check_and_record_notification_at(user_id, t0).await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        let res1 = r1.unwrap().unwrap();
        let res2 = r2.unwrap().unwrap();

        let some_count = [&res1, &res2].iter().filter(|r| r.is_some()).count();
        assert_eq!(
            some_count, 1,
            "concurrent same-time notifications must yield exactly 1 send decision, got {some_count}"
        );
    }

    #[tokio::test]
    async fn test_atomic_concurrent_failed_attempts_lockout() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let t0 = Utc::now();
        let barrier = Arc::new(tokio::sync::Barrier::new(5));
        let mut handles = Vec::new();

        for _ in 0..5 {
            let b = barrier.clone();
            let s = store.clone();
            handles.push(tokio::spawn(async move {
                b.wait().await;
                s.record_failed_attempt_at(t0).await
            }));
        }

        for h in handles {
            h.await.unwrap().unwrap();
        }

        let (_locked_until, failed_attempts): (DateTime<Utc>, i64) = sqlx::query_as(
            "SELECT locked_until, failed_attempts FROM pairing_platform_lockout WHERE platform_key = ?",
        )
        .bind(DEFAULT_PLATFORM_KEY)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            failed_attempts, 5,
            "all 5 concurrent failed attempts must be recorded without lost increment, got {failed_attempts}"
        );
        assert!(
            store.is_platform_locked_at(t0).await.unwrap(),
            "platform must be locked out at t0 after 5 failed attempts"
        );
    }

    #[tokio::test]
    async fn test_atomic_concurrent_issuance_and_capacity() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let user_id = 999111_u64;
        let t0 = Utc::now();

        // 1. Concurrent requests for the same user must return the identical active code
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let b1 = barrier.clone();
        let b2 = barrier.clone();
        let s1 = store.clone();
        let s2 = store.clone();

        let h1 = tokio::spawn(async move {
            b1.wait().await;
            s1.request_pairing_code_at(user_id, t0).await
        });
        let h2 = tokio::spawn(async move {
            b2.wait().await;
            s2.request_pairing_code_at(user_id, t0).await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        let code1 = r1.unwrap().unwrap();
        let code2 = r2.unwrap().unwrap();

        assert_eq!(
            code1, code2,
            "concurrent requests for same user must yield identical active code"
        );

        let user_code_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE user_id = ?")
                .bind(user_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            user_code_count, 1,
            "exactly 1 code row must exist for user, got {user_code_count}"
        );

        // 2. Capacity bound must not be breached under concurrent insertions at capacity limit
        // Pre-fill to MAX_PENDING_CODES - 1
        for i in 0..(MAX_PENDING_CODES - 1) {
            let uid = 20000 + i as u64;
            let _ = store.request_pairing_code_at(uid, t0).await.unwrap();
        }

        let barrier_cap = Arc::new(tokio::sync::Barrier::new(2));
        let bc1 = barrier_cap.clone();
        let bc2 = barrier_cap.clone();
        let sc1 = store.clone();
        let sc2 = store.clone();

        let hc1 = tokio::spawn(async move {
            bc1.wait().await;
            sc1.request_pairing_code_at(888001, t0).await
        });
        let hc2 = tokio::spawn(async move {
            bc2.wait().await;
            sc2.request_pairing_code_at(888002, t0).await
        });

        let (rc1, rc2) = tokio::join!(hc1, hc2);
        rc1.unwrap().unwrap();
        rc2.unwrap().unwrap();

        let total_pending: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes WHERE expires_at > ?")
                .bind(t0)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            total_pending as usize <= MAX_PENDING_CODES,
            "active pending count must not exceed capacity {MAX_PENDING_CODES}, got {total_pending}"
        );
    }

    #[tokio::test]
    async fn test_atomic_concurrent_approval_single_claim() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let user_id = 555666777_u64;
        let t0 = Utc::now();
        let code = store.request_pairing_code_at(user_id, t0).await.unwrap();

        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let b1 = barrier.clone();
        let b2 = barrier.clone();
        let s1 = store.clone();
        let s2 = store.clone();
        let c1 = code.clone();
        let c2 = code.clone();

        let h1 = tokio::spawn(async move {
            b1.wait().await;
            s1.approve_code_at(&c1, 1001, t0).await
        });
        let h2 = tokio::spawn(async move {
            b2.wait().await;
            s2.approve_code_at(&c2, 1002, t0).await
        });

        let (r1, r2) = tokio::join!(h1, h2);
        let out1 = r1.unwrap().unwrap();
        let out2 = r2.unwrap().unwrap();

        let success_count = [&out1, &out2]
            .iter()
            .filter(|o| matches!(o, PairingOutcome::Success { .. }))
            .count();
        let invalid_count = [&out1, &out2]
            .iter()
            .filter(|o| matches!(o, PairingOutcome::InvalidCode))
            .count();

        assert_eq!(
            success_count, 1,
            "exactly one concurrent approval must succeed"
        );
        assert_eq!(
            invalid_count, 1,
            "competing approval must receive InvalidCode (consumed)"
        );
        assert!(store.is_user_paired(user_id).await);
    }

    #[tokio::test]
    async fn notification_rows_are_pruned_by_window_and_by_pairing() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = PairingStore::new(pool.clone());
        store.init_cache().await.unwrap();

        let now = Utc::now();
        let fresh_user = 111111111_u64;
        let stale_user = 222222222_u64;
        let driver_user = 333333333_u64;

        // Fresh claim for fresh_user (keeps its row: it just throttled a DM).
        assert!(store
            .check_and_record_notification_at(fresh_user, now)
            .await
            .unwrap()
            .is_some());

        // A stale row: older than the rate-limit window, so it can never
        // throttle again — dead state.
        let stale_time = now - chrono::Duration::seconds(RATE_LIMIT_SECONDS + 60);
        sqlx::query("INSERT INTO pairing_notifications (user_id, last_notified_at) VALUES (?, ?)")
            .bind(stale_user.to_string())
            .bind(stale_time)
            .execute(&pool)
            .await
            .unwrap();

        // Another notification claim triggers the prune in the same tx.
        assert!(store
            .check_and_record_notification_at(driver_user, now)
            .await
            .unwrap()
            .is_some());

        let stale_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_notifications WHERE user_id = ?")
                .bind(stale_user.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stale_rows, 0, "stale notification rows must be pruned");

        let fresh_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_notifications WHERE user_id = ?")
                .bind(fresh_user.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(fresh_rows, 1, "fresh rows must survive the prune");

        // After successful pairing the user's row is no longer needed.
        let code = store.request_pairing_code(fresh_user).await.unwrap();
        let outcome = store.approve_code(&code, 424242).await.unwrap();
        assert_eq!(
            outcome,
            PairingOutcome::Success {
                user_id: fresh_user
            }
        );
        let rows_after_pairing: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pairing_notifications WHERE user_id = ?")
                .bind(fresh_user.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            rows_after_pairing, 0,
            "paired user's notification row must be deleted"
        );
    }

    #[tokio::test]
    async fn slack_store_pairs_string_ids_into_its_own_table() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let slack = SlackPairingStore::new(pool.clone());
        let discord = PairingStore::new(pool.clone());
        slack.init_cache().await.unwrap();

        let code = slack
            .check_and_record_notification("U0STRANGER")
            .await
            .unwrap()
            .expect("first prompt is sent");
        assert_eq!(
            slack
                .check_and_record_notification("U0STRANGER")
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            slack.request_pairing_code("U0STRANGER").await.unwrap(),
            code
        );

        assert_eq!(
            discord.approve_code(&code, 1).await.unwrap(),
            PairingOutcome::InvalidCode,
            "Discord /pair must not consume Slack codes"
        );
        assert_eq!(
            slack.approve_code(&code, "UOWNER").await.unwrap(),
            SlackPairingOutcome::Success {
                user_id: "U0STRANGER".into()
            }
        );
        assert!(slack.is_user_paired("U0STRANGER").await);
        let (operator,): (String,) =
            sqlx::query_as("SELECT operator_id FROM slack_paired_users WHERE user_id = ?")
                .bind("U0STRANGER")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(operator, "UOWNER");
        let discord_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM paired_users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(discord_rows, 0);
        assert_eq!(
            slack
                .check_and_record_notification("U0STRANGER")
                .await
                .unwrap(),
            None
        );

        let reloaded = SlackPairingStore::new(pool);
        reloaded.init_cache().await.unwrap();
        assert_eq!(
            reloaded.get_paired_user_ids_sync(),
            vec!["U0STRANGER".to_owned()]
        );
    }

    #[tokio::test]
    async fn slack_lockout_is_separate_from_discord() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let slack = SlackPairingStore::new(pool.clone());
        let discord = PairingStore::new(pool);
        let now = Utc::now();
        let code = slack.request_pairing_code_at("U1", now).await.unwrap();

        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert_eq!(
                slack
                    .approve_code_at("ZZZZ-ZZZZ", "UOWNER", now)
                    .await
                    .unwrap(),
                SlackPairingOutcome::InvalidCode
            );
        }
        assert!(slack.is_platform_locked_at(now).await.unwrap());
        assert!(!discord.is_platform_locked_at(now).await.unwrap());
        assert_eq!(
            slack.approve_code_at(&code, "UOWNER", now).await.unwrap(),
            SlackPairingOutcome::LockedOut
        );
        assert!(slack.request_pairing_code_at("U2", now).await.is_err());

        let later = now + chrono::Duration::seconds(LOCKOUT_DURATION_SECONDS + 1);
        assert!(!slack.is_platform_locked_at(later).await.unwrap());
        assert_eq!(
            slack.approve_code_at(&code, "UOWNER", later).await.unwrap(),
            SlackPairingOutcome::Success {
                user_id: "U1".into()
            }
        );
        let expired = slack.request_pairing_code_at("U3", later).await.unwrap();
        let after_ttl = later + chrono::Duration::seconds(CODE_TTL_SECONDS + 1);
        assert_eq!(
            slack
                .approve_code_at(&expired, "UOWNER", after_ttl)
                .await
                .unwrap(),
            SlackPairingOutcome::Expired
        );
    }
}

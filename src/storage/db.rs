use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;
use crate::OmonError;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

// Run after schema migrations and before any consumer can load session state.
// A quarantine record is evidence of an unresolved collision, not a routing alias.
async fn upgrade_legacy_guild_session_keys(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await?;
    let stored: Vec<String> =
        sqlx::query_scalar("SELECT session_key FROM sessions ORDER BY session_key")
            .fetch_all(&mut *tx)
            .await?;
    let blocked: std::collections::HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT canonical_key FROM legacy_guild_session_conflicts",
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();
    let mut lanes = std::collections::BTreeMap::<String, Vec<String>>::new();
    for raw in stored {
        let key = match crate::SessionKey::from_storage_key(&raw) {
            Ok(key) => key,
            Err(error) => {
                tracing::debug!(raw_key = %raw, %error, "leaving noncanonical historical session key unchanged");
                continue;
            }
        };
        if key.guild_id.is_some() {
            lanes.entry(key.storage_key()).or_default().push(raw);
        }
    }
    for (canonical, raw_keys) in lanes {
        // Include an already-canonical row in the collision group. Even a non-pending
        // sibling owns history; resume_pending is not a history-selection policy.
        if raw_keys.len() > 1 || blocked.contains(&canonical) {
            for raw in &raw_keys {
                sqlx::query(
                    "INSERT INTO legacy_guild_session_conflicts (raw_key, canonical_key)
                     VALUES (?, ?) ON CONFLICT(raw_key) DO NOTHING",
                )
                .bind(raw)
                .bind(&canonical)
                .execute(&mut *tx)
                .await?;
            }
            tracing::warn!(
                canonical_key = %canonical, raw_keys = ?raw_keys,
                "legacy guild session collision deferred (OD-01); histories and pending markers retained"
            );
            continue;
        }
        let raw = &raw_keys[0];
        if raw == &canonical {
            continue;
        }
        // Validate the stored initiator against the complete legacy key bytes.
        // The canonical lane's historical user_id alone is not author evidence.
        let key = crate::SessionKey::from_storage_key(raw)?;
        let initiator: String =
            sqlx::query_scalar("SELECT user_id FROM sessions WHERE session_key = ?")
                .bind(raw)
                .fetch_one(&mut *tx)
                .await?;
        let component = |value: Option<&str>| match value {
            Some(value) => format!("{}:{value}", value.len()),
            None => "-".to_owned(),
        };
        let mut legacy_parts = vec![
            component(Some(&key.platform)),
            component(key.guild_id.as_deref()),
            component(Some(&key.channel_id)),
            component(key.thread_id.as_deref()),
            component(Some(&initiator)),
        ];
        if let Some(bot) = key.bot_id.as_deref() {
            legacy_parts.push(component(Some(bot)));
        }
        if initiator.parse::<u64>().is_ok() && legacy_parts.join("|") == *raw {
            sqlx::query(
                "INSERT INTO legacy_guild_pending_auth
                    (message_id, message_sequence, canonical_key, raw_key, author_id)
                 SELECT m.id, m.sequence, ?, ?, ? FROM messages m
                 JOIN sessions s ON s.session_key = m.session_key
                 WHERE s.session_key = ? AND s.resume_pending = 1 AND m.role = 'user'
                   AND m.sequence = (SELECT MAX(sequence) FROM messages WHERE session_key = ?)",
            )
            .bind(&canonical)
            .bind(raw)
            .bind(&initiator)
            .bind(raw)
            .bind(raw)
            .execute(&mut *tx)
            .await?;
        }
        // Do not delete/reinsert: every declared child FK is ON DELETE CASCADE,
        // not ON UPDATE CASCADE. Deferred checks permit an atomic in-place rekey.
        // Keep user_id as the original initiator and preserve state_json byte-for-byte.
        sqlx::query("UPDATE sessions SET session_key = ? WHERE session_key = ?")
            .bind(&canonical)
            .bind(raw)
            .execute(&mut *tx)
            .await?;
        for table in [
            "messages",
            "delivery_ledger",
            "cron_jobs",
            "memories",
            "delivery_obligations",
        ] {
            sqlx::query(&format!(
                "UPDATE {table} SET session_key = ? WHERE session_key = ?"
            ))
            .bind(&canonical)
            .bind(raw)
            .execute(&mut *tx)
            .await?;
        }
        // The memory-approval payload is the one persisted serialized session-key
        // reference. Never replace occurrences in content, metadata, or skill payloads.
        sqlx::query(
            "UPDATE pending_writes SET payload = json_set(payload, '$.session_key', ?)
             WHERE kind = 'memory' AND CASE WHEN json_valid(payload)
             THEN json_extract(payload, '$.session_key') END = ?",
        )
        .bind(&canonical)
        .bind(raw)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens a SQLite pool, configures durable file databases for WAL mode,
    /// and applies every embedded migration before returning.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let in_memory = database_url == "sqlite::memory:"
            || database_url.starts_with("sqlite::memory:?")
            || database_url.contains("mode=memory");

        let mut options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));

        if !in_memory {
            options = options
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
                .pragma("wal_autocheckpoint", "1000");
        }

        // SQLite permits only one writer at a time. A single warm connection
        // queues all access in sqlx instead of letting pooled write transactions
        // collide at SQLite and fail with SQLITE_BUSY. It also keeps a plain
        // `sqlite::memory:` database coherent because each connection would
        // otherwise receive a private database.
        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .connect_with(options)
            .await?;

        MIGRATOR.run(&pool).await?;
        if let Err(error) = upgrade_legacy_guild_session_keys(&pool).await {
            pool.close().await;
            return Err(error);
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS discord_thread_owners (
                thread_id TEXT PRIMARY KEY,
                bot_id TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS delivery_ledger_constituents (
                parent_delivery_id TEXT NOT NULL,
                constituent_id TEXT NOT NULL PRIMARY KEY
            );",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_delivery_ledger_constituents_parent
             ON delivery_ledger_constituents(parent_delivery_id);",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Durably records bot-specific thread ownership in SQLite.
    pub async fn record_thread_owner(pool: &SqlitePool, thread_id: u64, bot_id: u64) -> Result<()> {
        record_thread_owner(pool, thread_id, bot_id).await
    }

    /// Fetches the durable bot-specific thread owner, if any.
    pub async fn get_thread_owner(pool: &SqlitePool, thread_id: u64) -> Result<Option<u64>> {
        get_thread_owner(pool, thread_id).await
    }

    /// Loads all durably persisted thread owners.
    pub async fn load_all_thread_owners(pool: &SqlitePool) -> Result<HashMap<u64, u64>> {
        load_all_thread_owners(pool).await
    }
}

/// Durably records bot-specific thread ownership in SQLite.
pub async fn record_thread_owner(pool: &SqlitePool, thread_id: u64, bot_id: u64) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS discord_thread_owners (
            thread_id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO discord_thread_owners (thread_id, bot_id, updated_at)
         VALUES (?, ?, (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')))
         ON CONFLICT(thread_id) DO UPDATE SET bot_id = excluded.bot_id, updated_at = excluded.updated_at",
    )
    .bind(thread_id.to_string())
    .bind(bot_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetches the durable bot-specific thread owner, if any.
pub async fn get_thread_owner(pool: &SqlitePool, thread_id: u64) -> Result<Option<u64>> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS discord_thread_owners (
            thread_id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );",
    )
    .execute(pool)
    .await?;

    let row: Option<(String,)> =
        sqlx::query_as("SELECT bot_id FROM discord_thread_owners WHERE thread_id = ?")
            .bind(thread_id.to_string())
            .fetch_optional(pool)
            .await?;

    Ok(row.and_then(|(s,)| s.parse().ok()))
}

/// Loads all durably persisted thread owners.
pub async fn load_all_thread_owners(pool: &SqlitePool) -> Result<HashMap<u64, u64>> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS discord_thread_owners (
            thread_id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );",
    )
    .execute(pool)
    .await?;

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT thread_id, bot_id FROM discord_thread_owners")
            .fetch_all(pool)
            .await?;

    let mut map = HashMap::new();
    for (t, b) in rows {
        if let (Ok(t_num), Ok(b_num)) = (t.parse::<u64>(), b.parse::<u64>()) {
            map.insert(t_num, b_num);
        }
    }
    Ok(map)
}

/// Marks a session as having an in-flight or queued turn pending restart recovery.
pub async fn mark_session_resume_pending(pool: &SqlitePool, session_key: &str) -> Result<()> {
    sqlx::query(
        "UPDATE sessions SET resume_pending = 1, updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE session_key = ?",
    )
    .bind(session_key)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clears the resume_pending marker for a session. Returns `true` if the flag was set.
pub async fn clear_session_resume_pending(pool: &SqlitePool, session_key: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE sessions SET resume_pending = 0, updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE session_key = ? AND resume_pending = 1",
    )
    .bind(session_key)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Persists the remote conversation binding (`omo_thread_id`) to the session's state_json in SQLite.
/// Guarantees that the session row exists and that `omo_thread_id` is durably checkpointed before turn execution.
#[allow(dead_code)]
pub async fn persist_session_binding(
    pool: &SqlitePool,
    session: &crate::SessionContext,
    omo_thread_id: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (
            session_key, platform, guild_id, channel_id, thread_id, user_id,
            state_json, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, '{}', (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')))
         ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(session.key.storage_key())
    .bind(&session.key.platform)
    .bind(&session.key.guild_id)
    .bind(&session.key.channel_id)
    .bind(&session.key.thread_id)
    .bind(&session.key.user_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE sessions
         SET state_json = json_set(COALESCE(NULLIF(state_json, ''), '{}'), '$.metadata.omo_thread_id', ?),
             updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         WHERE session_key = ?",
    )
    .bind(omo_thread_id)
    .bind(session.key.storage_key())
    .execute(pool)
    .await?;

    Ok(())
}

/// Marks a session as suspended (or unsuspended) in SQLite state_json.
pub async fn mark_session_suspended(
    pool: &SqlitePool,
    session_key: &str,
    suspended: bool,
) -> Result<()> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(session_key)
            .fetch_optional(pool)
            .await?;
    if let Some((state_json,)) = row {
        let mut state: crate::SessionState = serde_json::from_str(&state_json)
            .map_err(|error| crate::OmonError::Database(error.to_string()))?;
        state.suspended = suspended;
        let new_json = serde_json::to_string(&state)
            .map_err(|error| crate::OmonError::Database(error.to_string()))?;
        sqlx::query(
            "UPDATE sessions SET state_json = ?, updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE session_key = ?",
        )
        .bind(new_json)
        .bind(session_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Checks if a session is currently marked suspended.
pub async fn is_session_suspended(pool: &SqlitePool, session_key: &str) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(session_key)
            .fetch_optional(pool)
            .await?;
    if let Some((state_json,)) = row {
        let state: crate::SessionState = serde_json::from_str(&state_json).unwrap_or_default();
        return Ok(state.suspended);
    }
    Ok(false)
}

/// Counts the number of sessions currently marked resume_pending.
pub async fn count_resume_pending_sessions(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE resume_pending = 1")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

#[derive(sqlx::FromRow)]
struct ResumePendingSessionRow {
    #[allow(dead_code)]
    session_key: String,
    platform: String,
    guild_id: Option<String>,
    channel_id: String,
    thread_id: Option<String>,
    user_id: String,
}

/// Queries all session keys that currently have resume_pending = 1.
pub async fn fetch_resume_pending_session_keys(
    pool: &SqlitePool,
) -> Result<Vec<crate::SessionKey>> {
    let rows: Vec<ResumePendingSessionRow> = sqlx::query_as(
        "SELECT session_key, platform, guild_id, channel_id, thread_id, user_id
         FROM sessions WHERE resume_pending = 1 AND NOT EXISTS (
             SELECT 1 FROM legacy_guild_session_conflicts c
             WHERE c.raw_key = sessions.session_key OR c.canonical_key = sessions.session_key
         ) ORDER BY updated_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            crate::SessionKey::from_storage_key(&row.session_key).unwrap_or_else(|_| {
                crate::SessionKey::new(
                    row.platform,
                    row.guild_id,
                    row.channel_id,
                    row.thread_id,
                    row.user_id,
                )
            })
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct UnfinishedTurn {
    pub message_id: String,
    pub content: String,
    pub metadata_json: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub platform_message_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LastMessageRow {
    id: String,
    role: String,
    content: String,
    metadata_json: String,
    created_at: chrono::DateTime<chrono::Utc>,
    platform_message_id: Option<String>,
}

/// Finds the last unfinished user turn for a session if the most recent transcript row is a user message.
pub async fn find_last_unfinished_user_turn(
    pool: &SqlitePool,
    session_key: &str,
) -> Result<Option<UnfinishedTurn>> {
    let row: Option<LastMessageRow> = sqlx::query_as(
        "SELECT id, role, content, metadata_json, created_at, platform_message_id
         FROM messages
         WHERE session_key = ?
         ORDER BY sequence DESC
         LIMIT 1",
    )
    .bind(session_key)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        if row.role == "user" {
            return Ok(Some(UnfinishedTurn {
                message_id: row.id,
                content: row.content,
                metadata_json: row.metadata_json,
                created_at: row.created_at,
                platform_message_id: row.platform_message_id,
            }));
        }
    }
    Ok(None)
}

/// Startup recovery: finds sessions marked resume_pending from a previous run/crash/restart,
/// reconstructs their last unfinished user turn, and re-dispatches them through the multiplexer.
#[allow(dead_code)]
pub async fn recover_resume_pending_sessions(
    pool: &SqlitePool,
    multiplexer: &crate::SessionMultiplexer,
) -> Result<usize> {
    let pending_keys = fetch_resume_pending_session_keys(pool).await?;
    let mut resumed_count = 0;
    for session_key in pending_keys {
        let storage_key = session_key.storage_key();
        let is_suspended = is_session_suspended(pool, &storage_key).await?;
        let cleared = clear_session_resume_pending(pool, &storage_key).await?;
        if !cleared {
            continue;
        }
        if is_suspended {
            tracing::info!(
                session = %session_key,
                "skipping restart recovery for suspended session"
            );
            continue;
        }

        if let Some(unfinished) = find_last_unfinished_user_turn(pool, &storage_key).await? {
            let delivery_id: Option<String> = if let Some(ref pid) = unfinished.platform_message_id
            {
                sqlx::query_scalar(
                    "SELECT message_id FROM delivery_ledger WHERE session_key = ? AND (platform_message_id = ? OR message_id = ?) ORDER BY created_at DESC LIMIT 1",
                )
                .bind(&storage_key)
                .bind(pid)
                .bind(pid)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
            } else {
                sqlx::query_scalar(
                    "SELECT message_id FROM delivery_ledger WHERE session_key = ? ORDER BY created_at DESC LIMIT 1",
                )
                .bind(&storage_key)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
            };

            let attachments: Vec<crate::MessageAttachment> =
                serde_json::from_str(&unfinished.metadata_json).unwrap_or_default();
            let event = crate::InboundEvent {
                id: uuid::Uuid::parse_str(&unfinished.message_id)
                    .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                session: session_key.clone(),
                platform_message_id: String::new(),
                delivery_id,
                content: unfinished.content,
                attachments,
                received_at: unfinished.created_at,
            };
            tracing::info!(
                session = %session_key,
                "re-dispatching unfinished user turn on restart recovery"
            );
            if let Err(error) = multiplexer.route(event).await {
                tracing::error!(
                    session = %session_key,
                    %error,
                    "failed to route resumed session event"
                );
            } else {
                resumed_count += 1;
            }
        } else {
            let delivery_id: Option<String> = sqlx::query_scalar(
                "SELECT message_id FROM delivery_ledger WHERE session_key = ? ORDER BY created_at DESC LIMIT 1",
            )
            .bind(&storage_key)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some(del_id) = delivery_id {
                let ledger = crate::DeliveryLedgerService::new(pool.clone());
                let _ = ledger.mark_delivered(&del_id).await;
            }
            resumed_count += 1;
        }
    }
    Ok(resumed_count)
}

/// Checks if a message with the given `platform_message_id` already exists in the transcript for `session_key`.
pub async fn has_platform_message_id(
    pool: &SqlitePool,
    session_key: &str,
    platform_message_id: &str,
) -> Result<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM messages WHERE session_key = ? AND platform_message_id = ? LIMIT 1",
    )
    .bind(session_key)
    .bind(platform_message_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingWrite {
    pub id: String,
    pub kind: String,
    pub payload: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy)]
pub enum PendingWriteScope<'a> {
    Skills,
    Memory(&'a str),
}

impl PendingWriteScope<'_> {
    const fn kind(self) -> &'static str {
        match self {
            Self::Skills => "skill",
            Self::Memory(_) => "memory",
        }
    }

    fn permits(self, item: &PendingWrite) -> Result<bool> {
        if item.kind != self.kind() {
            return Ok(false);
        }
        match self {
            Self::Skills => Ok(true),
            Self::Memory(session) => {
                #[derive(Deserialize)]
                struct MemoryIdentity {
                    session_key: String,
                }
                let identity: MemoryIdentity = serde_json::from_str(&item.payload)
                    .map_err(|error| crate::OmonError::Database(error.to_string()))?;
                Ok(identity.session_key == session)
            }
        }
    }
}

pub async fn list_pending_writes_scoped(
    pool: &SqlitePool,
    scope: PendingWriteScope<'_>,
) -> Result<Vec<PendingWrite>> {
    let mut matching = Vec::new();
    for item in list_pending_writes(pool, Some(scope.kind())).await? {
        if scope.permits(&item)? {
            matching.push(item);
        }
    }
    Ok(matching)
}

pub async fn get_pending_write_scoped(
    pool: &SqlitePool,
    id: &str,
    scope: PendingWriteScope<'_>,
) -> Result<PendingWrite> {
    if let Some(item) = get_pending_write(pool, id).await? {
        if scope.permits(&item)? {
            return Ok(item);
        }
    }
    Err(crate::OmonError::Approval(
        "pending write unavailable in this scope".into(),
    ))
}

pub async fn approve_pending_write_scoped(
    pool: &SqlitePool,
    id: &str,
    scope: PendingWriteScope<'_>,
) -> Result<Option<String>> {
    let item = get_pending_write_scoped(pool, id, scope).await?;
    apply_pending_write(pool, item, None).await
}

pub async fn reject_pending_write_scoped(
    pool: &SqlitePool,
    id: &str,
    scope: PendingWriteScope<'_>,
) -> Result<bool> {
    let item = get_pending_write_scoped(pool, id, scope).await?;
    let result =
        sqlx::query("DELETE FROM pending_writes WHERE id = ? AND kind = ? AND payload = ?")
            .bind(&item.id)
            .bind(&item.kind)
            .bind(&item.payload)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() == 1)
}

pub fn write_approval_enabled() -> bool {
    std::env::var("WRITE_APPROVAL")
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub async fn stage_pending_write(pool: &SqlitePool, kind: &str, payload: &str) -> Result<String> {
    let raw_uuid = Uuid::new_v4().to_string();
    let short_id = raw_uuid.replace('-', "")[..8].to_string();
    let now = Utc::now();
    sqlx::query("INSERT INTO pending_writes (id, kind, payload, created_at) VALUES (?, ?, ?, ?)")
        .bind(&short_id)
        .bind(kind)
        .bind(payload)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(short_id)
}

pub async fn list_pending_writes(
    pool: &SqlitePool,
    kind: Option<&str>,
) -> Result<Vec<PendingWrite>> {
    if let Some(kind) = kind {
        let rows = sqlx::query_as::<_, PendingWrite>(
            "SELECT id, kind, payload, created_at FROM pending_writes WHERE kind = ? ORDER BY created_at ASC",
        )
        .bind(kind)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    } else {
        let rows = sqlx::query_as::<_, PendingWrite>(
            "SELECT id, kind, payload, created_at FROM pending_writes ORDER BY created_at ASC",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

pub async fn get_pending_write(pool: &SqlitePool, id: &str) -> Result<Option<PendingWrite>> {
    let row = sqlx::query_as::<_, PendingWrite>(
        "SELECT id, kind, payload, created_at FROM pending_writes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn delete_pending_write(pool: &SqlitePool, id: &str) -> Result<bool> {
    let res = sqlx::query("DELETE FROM pending_writes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn approve_pending_write(
    pool: &SqlitePool,
    id: &str,
    target_skills_dir: Option<&Path>,
) -> Result<Option<String>> {
    let Some(item) = get_pending_write(pool, id).await? else {
        return Ok(None);
    };
    apply_pending_write(pool, item, target_skills_dir).await
}

async fn apply_pending_write(
    pool: &SqlitePool,
    item: PendingWrite,
    target_skills_dir: Option<&Path>,
) -> Result<Option<String>> {
    let id = &item.id;
    #[cfg(test)]
    if let Ok(barrier) = REPLAY_BARRIER.try_with(Clone::clone) {
        barrier.wait().await;
    }

    let mut transaction = pool.begin().await?;
    let claimed =
        sqlx::query("DELETE FROM pending_writes WHERE id = ? AND kind = ? AND payload = ?")
            .bind(id)
            .bind(&item.kind)
            .bind(&item.payload)
            .execute(&mut *transaction)
            .await?;
    if claimed.rows_affected() == 0 {
        transaction.rollback().await?;
        return Ok(None);
    }

    let receipt = match item.kind.as_str() {
        "memory" => {
            let val: serde_json::Value = serde_json::from_str(&item.payload)
                .map_err(|e| crate::OmonError::Database(format!("invalid memory payload: {e}")))?;
            let session_key_str = val
                .get("session_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content = val.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let metadata = val
                .get("metadata")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let memory_id = Uuid::new_v4().to_string();
            let metadata_json = serde_json::to_string(&metadata)
                .map_err(|e| crate::OmonError::Database(e.to_string()))?;

            sqlx::query(
                "INSERT INTO memories (id, session_key, content, metadata_json) VALUES (?, ?, ?, ?)",
            )
            .bind(&memory_id)
            .bind(session_key_str)
            .bind(content)
            .bind(metadata_json)
            .execute(&mut *transaction)
            .await?;

            format!("Approved memory write [{id}]: \"{content}\"")
        }
        "skill" => {
            let val: serde_json::Value = serde_json::from_str(&item.payload)
                .map_err(|e| crate::OmonError::Database(format!("invalid skill payload: {e}")))?;
            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let content = val.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                return Err(crate::OmonError::ToolExecution(
                    "empty skill name in payload".into(),
                ));
            }

            let base_dir = if let Some(root) = val.get("skills_root").and_then(|v| v.as_str()) {
                PathBuf::from(root)
            } else if let Some(dir) = target_skills_dir {
                dir.to_path_buf()
            } else if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(&home).join(".omon").join("skills")
            } else {
                PathBuf::from(".omon").join("skills")
            };

            let skill_file = crate::tools::SkillsTool::validated_write_path(&base_dir, name)?;
            if let Some(destination) = val.get("destination").and_then(|v| v.as_str()) {
                if skill_file != Path::new(destination) {
                    return Err(crate::OmonError::ToolExecution(
                        "staged skill destination changed before approval".into(),
                    ));
                }
            }
            let skill_dir = skill_file.with_file_name("");
            std::fs::create_dir_all(&skill_dir).map_err(|e| {
                crate::OmonError::ToolExecution(format!("failed to create skill dir: {e}"))
            })?;
            std::fs::write(&skill_file, content).map_err(|e| {
                crate::OmonError::ToolExecution(format!("failed to write SKILL.md: {e}"))
            })?;

            format!(
                "Approved skill write [{id}]: skill '{name}' written to {}",
                skill_file.display()
            )
        }
        _ => format!("Discarded unknown pending write kind: {}", item.kind),
    };
    transaction.commit().await?;
    Ok(Some(receipt))
}

pub async fn reject_pending_write(pool: &SqlitePool, id: &str) -> Result<bool> {
    delete_pending_write(pool, id).await
}

pub async fn persist_dead_target(
    pool: &SqlitePool,
    bot_id: &str,
    channel_id: u64,
    status_code: u16,
    error_message: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO dead_targets (bot_id, channel_id, status_code, error_message, dead_since)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (bot_id, channel_id) DO UPDATE SET
            status_code = excluded.status_code,
            error_message = excluded.error_message,
            dead_since = excluded.dead_since",
    )
    .bind(bot_id)
    .bind(channel_id as i64)
    .bind(status_code as i64)
    .bind(error_message)
    .bind(now)
    .execute(pool)
    .await
    .map_err(OmonError::from)?;
    Ok(())
}

pub async fn remove_dead_target(pool: &SqlitePool, bot_id: &str, channel_id: u64) -> Result<()> {
    sqlx::query("DELETE FROM dead_targets WHERE bot_id = ? AND channel_id = ?")
        .bind(bot_id)
        .bind(channel_id as i64)
        .execute(pool)
        .await
        .map_err(OmonError::from)?;
    Ok(())
}

pub async fn remove_dead_targets_for_channel(pool: &SqlitePool, channel_id: u64) -> Result<()> {
    sqlx::query("DELETE FROM dead_targets WHERE channel_id = ?")
        .bind(channel_id as i64)
        .execute(pool)
        .await
        .map_err(OmonError::from)?;
    Ok(())
}

pub async fn load_dead_targets(
    pool: &SqlitePool,
) -> Result<Vec<(String, u64, u16, String, String)>> {
    let rows: Vec<(String, i64, i64, String, String)> = sqlx::query_as(
        "SELECT bot_id, channel_id, status_code, error_message, dead_since FROM dead_targets",
    )
    .fetch_all(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(rows
        .into_iter()
        .map(|(bot, chan, code, msg, since)| (bot, chan as u64, code as u16, msg, since))
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct WorkItem {
    pub id: String,
    pub title: String,
    pub thread_id: Option<String>,
    pub delegate_pane: Option<String>,
    pub status: String,
    pub last_progress_at: String,
    pub receipt_path: Option<String>,
    #[sqlx(default)]
    pub created_at: Option<String>,
    #[sqlx(default)]
    pub updated_at: Option<String>,
}

impl WorkItem {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        thread_id: Option<String>,
        delegate_pane: Option<String>,
        last_progress_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            thread_id,
            delegate_pane,
            status: "in_progress".to_string(),
            last_progress_at: last_progress_at.into(),
            receipt_path: None,
            created_at: None,
            updated_at: None,
        }
    }
}

pub async fn insert_work_item(pool: &SqlitePool, item: &WorkItem) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO work_items (
            id, title, thread_id, delegate_pane, status, last_progress_at, receipt_path, created_at, updated_at
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?,
            COALESCE(?, CURRENT_TIMESTAMP),
            COALESCE(?, CURRENT_TIMESTAMP)
        )
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            thread_id = excluded.thread_id,
            delegate_pane = excluded.delegate_pane,
            status = excluded.status,
            last_progress_at = excluded.last_progress_at,
            receipt_path = excluded.receipt_path,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&item.id)
    .bind(&item.title)
    .bind(&item.thread_id)
    .bind(&item.delegate_pane)
    .bind(&item.status)
    .bind(&item.last_progress_at)
    .bind(&item.receipt_path)
    .bind(&item.created_at)
    .bind(&item.updated_at)
    .execute(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(())
}

pub async fn get_work_item(pool: &SqlitePool, id: &str) -> Result<Option<WorkItem>> {
    let item = sqlx::query_as::<_, WorkItem>(
        r#"
        SELECT id, title, thread_id, delegate_pane, status, last_progress_at, receipt_path, created_at, updated_at
        FROM work_items
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(item)
}

pub async fn mark_work_item_done(
    pool: &SqlitePool,
    id: &str,
    receipt_path: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE work_items
        SET status = 'done',
            receipt_path = COALESCE(?, receipt_path),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(receipt_path)
    .bind(id)
    .execute(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(())
}

pub async fn update_work_item_progress(
    pool: &SqlitePool,
    id: &str,
    last_progress_at: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE work_items
        SET last_progress_at = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(last_progress_at)
    .bind(id)
    .execute(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(())
}

pub async fn get_stale_in_progress_work_items(
    pool: &SqlitePool,
    cutoff_time: &str,
) -> Result<Vec<WorkItem>> {
    let items = sqlx::query_as::<_, WorkItem>(
        r#"
        SELECT id, title, thread_id, delegate_pane, status, last_progress_at, receipt_path, created_at, updated_at
        FROM work_items
        WHERE status IN ('in_progress', 'running')
          AND (last_progress_at <= ? OR datetime(last_progress_at) <= datetime(?))
        ORDER BY last_progress_at ASC
        "#,
    )
    .bind(cutoff_time)
    .bind(cutoff_time)
    .fetch_all(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(items)
}

#[cfg(test)]
tokio::task_local! {
    static REPLAY_BARRIER: std::sync::Arc<tokio::sync::Barrier>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use sqlx::Row;
    use tokio::sync::Barrier;

    use super::Database;

    #[tokio::test]
    async fn staged_memory_claim_is_atomic() {
        // Given two approvers holding the same real pending payload before claim.
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('u07', 'discord', 'channel7', 'user8', '{}')").execute(pool).await.unwrap();
        let payload = serde_json::json!({"session_key":"u07","content":"u07-memory","metadata":{"source":"qa"}});
        let id = super::stage_pending_write(pool, "memory", &payload.to_string())
            .await
            .unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(
                super::REPLAY_BARRIER.scope(
                    barrier.clone(),
                    super::approve_pending_write(pool, &id, None)
                ),
                super::REPLAY_BARRIER.scope(barrier, super::approve_pending_write(pool, &id, None))
            )
        })
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(pool)
            .await
            .unwrap();
        println!("surface approvals={a:?},{b:?} memory_rows={count}");
        assert_eq!(count, 1);
        assert_eq!(
            usize::from(a.unwrap().is_some()) + usize::from(b.unwrap().is_some()),
            1
        );
        assert!(super::get_pending_write(pool, &id).await.unwrap().is_none());
        pool.close().await;
    }

    #[tokio::test]
    async fn staged_memory_failure_retains_pending_without_fabricating_session() {
        // Given a staged legacy payload with no trustworthy session row.
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let payload = serde_json::json!({
            "session_key": "missing-u07",
            "content": "rollback-sentinel",
            "metadata": {}
        });
        let id = super::stage_pending_write(pool, "memory", &payload.to_string())
            .await
            .unwrap();

        // When applying it fails its real SQLite foreign-key boundary.
        assert!(super::approve_pending_write(pool, &id, None).await.is_err());

        // Then both the pending intent and the original empty stores are retained.
        assert!(super::get_pending_write(pool, &id).await.unwrap().is_some());
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM sessions), (SELECT COUNT(*) FROM memories)",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(counts, (0, 0));
        pool.close().await;
    }

    #[tokio::test]
    async fn applies_all_migrations_to_an_in_memory_database() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(database.pool())
        .await
        .expect("schema should be queryable");
        let tables: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        for expected in [
            "sessions",
            "messages",
            "delivery_ledger",
            "cron_jobs",
            "memories",
            "cron_runs",
            "delivery_obligations",
            "approval_allowlist",
            "pending_writes",
        ] {
            assert!(tables.contains(expected), "missing table {expected}");
        }
        assert!(tables.contains("_sqlx_migrations"));
    }

    #[tokio::test]
    async fn scoped_approval_does_not_apply_replaced_payload() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let original = serde_json::json!({
            "session_key": "A", "content": "original", "metadata": {}
        });
        let id = super::stage_pending_write(pool, "memory", &original.to_string())
            .await
            .unwrap();
        let checked =
            super::get_pending_write_scoped(pool, &id, super::PendingWriteScope::Memory("A"))
                .await
                .unwrap();
        let replacement = serde_json::json!({
            "session_key": "B", "content": "replacement", "metadata": {}
        });
        sqlx::query("UPDATE pending_writes SET payload = ? WHERE id = ?")
            .bind(replacement.to_string())
            .bind(&id)
            .execute(pool)
            .await
            .unwrap();
        assert!(super::apply_pending_write(pool, checked, None)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            super::get_pending_write(pool, &id)
                .await
                .unwrap()
                .unwrap()
                .payload,
            replacement.to_string()
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn migration_creates_pending_writes_table_and_indexes() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'pending_writes'",
        )
        .fetch_all(database.pool())
        .await
        .expect("indexes should be queryable");

        let indexes: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(indexes.contains("idx_pending_writes_kind_created"));
    }

    #[tokio::test]
    async fn test_pending_writes_store_round_trip() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        // 1. Stage memory write
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('sess:test', 'discord', 'channel-test', 'user-test', '{}')")
            .execute(pool)
            .await
            .unwrap();
        let mem_payload = serde_json::json!({
            "session_key": "sess:test",
            "content": "User prefers dark mode",
            "metadata": {"source": "test"}
        });
        let mem_id = super::stage_pending_write(pool, "memory", &mem_payload.to_string())
            .await
            .unwrap();

        // List pending writes
        let pending = super::list_pending_writes(pool, Some("memory"))
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, mem_id);
        assert_eq!(pending[0].kind, "memory");

        // Approve memory write
        let approve_res = super::approve_pending_write(pool, &mem_id, None)
            .await
            .unwrap();
        assert!(approve_res.is_some());
        assert!(approve_res.unwrap().contains("User prefers dark mode"));

        // Verify removed from pending
        let pending_after = super::list_pending_writes(pool, Some("memory"))
            .await
            .unwrap();
        assert!(pending_after.is_empty());

        // Verify inserted into memories
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM memories WHERE session_key = 'sess:test'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(count, 1);

        // 2. Stage and Reject memory write
        let mem_id2 = super::stage_pending_write(pool, "memory", &mem_payload.to_string())
            .await
            .unwrap();
        let rejected = super::reject_pending_write(pool, &mem_id2).await.unwrap();
        assert!(rejected);
        let pending_after2 = super::list_pending_writes(pool, Some("memory"))
            .await
            .unwrap();
        assert!(pending_after2.is_empty());

        // 3. Stage and Approve skill write
        let skill_dir = tempfile::tempdir().unwrap();
        let skill_payload = serde_json::json!({
            "name": "super-agent",
            "content": "# Super Agent Skill\n\nInstructions here."
        });
        let skill_id = super::stage_pending_write(pool, "skill", &skill_payload.to_string())
            .await
            .unwrap();

        let pending_skills = super::list_pending_writes(pool, Some("skill"))
            .await
            .unwrap();
        assert_eq!(pending_skills.len(), 1);
        assert_eq!(pending_skills[0].id, skill_id);

        let approve_skill = super::approve_pending_write(pool, &skill_id, Some(skill_dir.path()))
            .await
            .unwrap();
        assert!(approve_skill.is_some());
        assert!(skill_dir
            .path()
            .join("super-agent")
            .join("SKILL.md")
            .exists());
        let written_content =
            std::fs::read_to_string(skill_dir.path().join("super-agent").join("SKILL.md")).unwrap();
        assert_eq!(written_content, "# Super Agent Skill\n\nInstructions here.");

        // 4. Stage and Reject skill write
        let skill_id2 = super::stage_pending_write(pool, "skill", &skill_payload.to_string())
            .await
            .unwrap();
        let rejected_skill = super::reject_pending_write(pool, &skill_id2).await.unwrap();
        assert!(rejected_skill);
        let pending_skills2 = super::list_pending_writes(pool, Some("skill"))
            .await
            .unwrap();
        assert!(pending_skills2.is_empty());
    }

    #[tokio::test]
    async fn mark_session_suspended_fails_on_malformed_state_without_overwriting() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES ('corrupt-key', 'discord', 'c1', 'u1', 'INVALID_JSON_CORRUPT')"
        )
        .execute(pool)
        .await
        .unwrap();

        let result = super::mark_session_suspended(pool, "corrupt-key", true).await;
        assert!(
            result.is_err(),
            "malformed state must return typed serialization error"
        );

        let raw: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = 'corrupt-key'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(
            raw, "INVALID_JSON_CORRUPT",
            "corrupt state must not be overwritten with defaults"
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn migration_creates_cron_runs_indexes() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'cron_runs'",
        )
        .fetch_all(database.pool())
        .await
        .expect("indexes should be queryable");

        let indexes: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(indexes.contains("idx_cron_runs_job_status"));
        assert!(indexes.contains("idx_cron_runs_job_attempt"));
        assert!(indexes.contains("idx_cron_runs_job_started"));
        assert!(indexes.contains("idx_cron_runs_active_lease"));
    }

    #[tokio::test]
    async fn migration_creates_cron_runs_owner_pid_column() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row = sqlx::query("SELECT owner_pid FROM cron_runs LIMIT 0")
            .fetch_optional(database.pool())
            .await;
        assert!(row.is_ok(), "owner_pid column should exist on cron_runs");
    }

    #[tokio::test]
    async fn file_database_serializes_concurrent_writers_without_lock_errors() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join("writers.db");
        let database = Database::connect(&format!("sqlite://{}", path.display()))
            .await
            .expect("file database should initialize");
        assert_eq!(database.pool().options().get_max_connections(), 1);
        assert_eq!(database.pool().options().get_min_connections(), 1);

        sqlx::query("CREATE TABLE concurrent_writes (value INTEGER NOT NULL)")
            .execute(database.pool())
            .await
            .expect("test table should be created");

        // The former 10-connection file pool made this workload flaky because
        // independently acquired writers could collide and return SQLITE_BUSY.
        const WRITERS: usize = 32;
        let barrier = Arc::new(Barrier::new(WRITERS));
        let mut tasks = Vec::with_capacity(WRITERS);
        for value in 0..WRITERS {
            let pool = database.pool().clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                sqlx::query("INSERT INTO concurrent_writes (value) VALUES (?)")
                    .bind(value as i64)
                    .execute(&pool)
                    .await
            }));
        }

        for task in tasks {
            task.await
                .expect("writer task should not panic")
                .expect("serialized writer should not return SQLITE_BUSY");
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_writes")
            .fetch_one(database.pool())
            .await
            .expect("written rows should be queryable");
        assert_eq!(count, WRITERS as i64);

        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(database.pool())
            .await
            .expect("journal mode should be queryable");
        let autocheckpoint: i64 = sqlx::query_scalar("PRAGMA wal_autocheckpoint")
            .fetch_one(database.pool())
            .await
            .expect("WAL autocheckpoint should be queryable");
        assert_eq!(journal_mode, "wal");
        assert_eq!(autocheckpoint, 1000);
    }

    #[tokio::test]
    async fn enforces_foreign_keys_after_migration() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let result = sqlx::query(
            "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, ?, ?)",
        )
        .bind("message-1")
        .bind("missing-session")
        .bind("user")
        .bind("hello")
        .execute(database.pool())
        .await;

        assert!(result.is_err(), "orphan messages must be rejected");
    }

    #[tokio::test]
    async fn migrations_are_idempotent_for_repeated_connections() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(database.pool())
            .await
            .expect("migration ledger should be queryable");

        super::MIGRATOR
            .run(database.pool())
            .await
            .expect("reapplying migrations should be safe");

        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(database.pool())
            .await
            .expect("migration ledger should be queryable");
        assert_eq!(count_after, count_before);
    }

    #[tokio::test]
    async fn message_sequence_preserves_causality_and_recent_history_window() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        sqlx::query(
            "INSERT INTO sessions (
                session_key, platform, channel_id, user_id, state_json
             ) VALUES ('session-order', 'test', 'channel', 'user', '{}')",
        )
        .execute(database.pool())
        .await
        .unwrap();

        // Give every row the same timestamp and IDs whose lexical order is the
        // reverse of insertion order. Neither field may be used as causality.
        for index in 0..105 {
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content, created_at)
                 VALUES (?, 'session-order', 'user', ?, '2026-08-15T00:00:00.000Z')",
            )
            .bind(format!("message-{:03}", 104 - index))
            .bind(index.to_string())
            .execute(database.pool())
            .await
            .unwrap();
        }

        let sequence: Vec<(i64, String)> = sqlx::query_as(
            "SELECT sequence, content FROM messages
             WHERE session_key = 'session-order' ORDER BY sequence",
        )
        .fetch_all(database.pool())
        .await
        .unwrap();
        assert_eq!(sequence.first(), Some(&(1, "0".into())));
        assert_eq!(sequence.last(), Some(&(105, "104".into())));

        let recent: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM (
                SELECT sequence, content FROM messages
                WHERE session_key = 'session-order'
                ORDER BY sequence DESC LIMIT 100
             ) ORDER BY sequence ASC",
        )
        .fetch_all(database.pool())
        .await
        .unwrap();
        assert_eq!(recent.len(), 100);
        assert_eq!(recent.first().map(|row| row.0.as_str()), Some("5"));
        assert_eq!(recent.last().map(|row| row.0.as_str()), Some("104"));
    }

    #[tokio::test]
    async fn migration_creates_discord_channel_cursors_table() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row = sqlx::query(
            "SELECT channel_id, last_message_id, updated_at FROM discord_channel_cursors LIMIT 0",
        )
        .fetch_optional(database.pool())
        .await;
        assert!(
            row.is_ok(),
            "discord_channel_cursors table should exist with correct columns"
        );
    }

    #[tokio::test]
    async fn migration_creates_discord_bot_cursors_table() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row = sqlx::query(
            "SELECT bot_id, channel_id, last_message_id, updated_at FROM discord_bot_cursors LIMIT 0",
        )
        .fetch_optional(database.pool())
        .await;
        assert!(
            row.is_ok(),
            "discord_bot_cursors table should exist with correct columns"
        );
    }

    #[tokio::test]
    async fn migration_creates_pairing_tables_and_indexes() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row_codes = sqlx::query(
            "SELECT code, user_id, created_at, expires_at, attempts FROM pairing_codes LIMIT 0",
        )
        .fetch_optional(database.pool())
        .await;
        assert!(
            row_codes.is_ok(),
            "pairing_codes table should exist with correct columns"
        );

        let row_paired = sqlx::query("SELECT user_id, paired_at FROM paired_users LIMIT 0")
            .fetch_optional(database.pool())
            .await;
        assert!(
            row_paired.is_ok(),
            "paired_users table should exist with correct columns"
        );

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'pairing_codes'",
        )
        .fetch_all(database.pool())
        .await
        .expect("indexes should be queryable");

        let indexes: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(indexes.contains("idx_pairing_codes_user_id"));
    }

    #[tokio::test]
    async fn migration_creates_resume_pending_column_and_index() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row = sqlx::query("SELECT resume_pending FROM sessions LIMIT 0")
            .fetch_optional(database.pool())
            .await;
        assert!(row.is_ok(), "resume_pending column should exist");

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'sessions'",
        )
        .fetch_all(database.pool())
        .await
        .expect("indexes should be queryable");

        let indexes: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(indexes.contains("idx_sessions_resume_pending"));
    }

    #[tokio::test]
    async fn test_resume_pending_flag_lifecycle_and_queries() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES ('sess-1', 'discord', 'c1', 'u1', '{}'),
                    ('sess-2', 'discord', 'c2', 'u2', '{}')",
        )
        .execute(pool)
        .await
        .unwrap();

        // Initially none are resume_pending
        let pending = super::fetch_resume_pending_session_keys(pool)
            .await
            .unwrap();
        assert!(pending.is_empty());
        assert_eq!(super::count_resume_pending_sessions(pool).await.unwrap(), 0);

        // Mark sess-1 as resume_pending
        super::mark_session_resume_pending(pool, "sess-1")
            .await
            .unwrap();
        let pending = super::fetch_resume_pending_session_keys(pool)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].channel_id, "c1");
        assert_eq!(super::count_resume_pending_sessions(pool).await.unwrap(), 1);

        // Clear sess-1
        let cleared = super::clear_session_resume_pending(pool, "sess-1")
            .await
            .unwrap();
        assert!(cleared);
        let cleared_again = super::clear_session_resume_pending(pool, "sess-1")
            .await
            .unwrap();
        assert!(!cleared_again); // already cleared
        assert_eq!(super::count_resume_pending_sessions(pool).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_find_last_unfinished_user_turn() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES ('sess-turn', 'discord', 'c1', 'u1', '{}')",
        )
        .execute(pool)
        .await
        .unwrap();

        // No messages -> None
        let turn = super::find_last_unfinished_user_turn(pool, "sess-turn")
            .await
            .unwrap();
        assert!(turn.is_none());

        // User message -> Some
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-u1', 'sess-turn', 'user', 'hello agent', '[]')",
        )
        .execute(pool)
        .await
        .unwrap();

        let turn = super::find_last_unfinished_user_turn(pool, "sess-turn")
            .await
            .unwrap()
            .expect("should find turn");
        assert_eq!(turn.content, "hello agent");
        assert_eq!(turn.message_id, "msg-u1");

        // Assistant message completes it -> None
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-a1', 'sess-turn', 'assistant', 'hello user', '{}')",
        )
        .execute(pool)
        .await
        .unwrap();

        let turn = super::find_last_unfinished_user_turn(pool, "sess-turn")
            .await
            .unwrap();
        assert!(turn.is_none());
    }

    #[tokio::test]
    async fn migration_creates_messages_platform_id_column_and_index() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");

        let row = sqlx::query("SELECT platform_message_id FROM messages LIMIT 0")
            .fetch_optional(database.pool())
            .await;
        assert!(row.is_ok(), "platform_message_id column should exist");

        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'messages'",
        )
        .fetch_all(database.pool())
        .await
        .expect("indexes should be queryable");

        let indexes: HashSet<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        assert!(indexes.contains("idx_messages_session_platform_id"));
    }

    #[tokio::test]
    async fn test_has_platform_message_id_dedup_query() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES ('sess-dedup', 'discord', 'c1', 'u1', '{}')",
        )
        .execute(pool)
        .await
        .unwrap();

        // Empty transcript -> false
        assert!(
            !super::has_platform_message_id(pool, "sess-dedup", "plat-msg-123")
                .await
                .unwrap()
        );

        // Insert message with platform_message_id
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, platform_message_id)
             VALUES ('m-dedup-1', 'sess-dedup', 'user', 'hello', 'plat-msg-123')",
        )
        .execute(pool)
        .await
        .unwrap();

        // Existing platform_message_id on same session -> true
        assert!(
            super::has_platform_message_id(pool, "sess-dedup", "plat-msg-123")
                .await
                .unwrap()
        );

        // Different platform_message_id -> false
        assert!(
            !super::has_platform_message_id(pool, "sess-dedup", "plat-msg-456")
                .await
                .unwrap()
        );

        // Same platform_message_id on different session -> false
        assert!(
            !super::has_platform_message_id(pool, "sess-other", "plat-msg-123")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_persist_session_binding_persists_durable_binding() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        let key = crate::SessionKey::new(
            "discord",
            Some("guild-1"),
            "chan-1",
            None::<String>,
            "user-1",
        );
        let session = crate::SessionContext::new(key.clone());

        super::persist_session_binding(pool, &session, "remote-thread-xyz")
            .await
            .expect("persist_session_binding should succeed");

        let state_json: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(key.storage_key())
                .fetch_one(pool)
                .await
                .expect("session row must exist");

        let state: crate::SessionState =
            serde_json::from_str(&state_json).expect("valid state_json");
        assert_eq!(
            state.metadata.get("omo_thread_id").and_then(|v| v.as_str()),
            Some("remote-thread-xyz"),
            "omo_thread_id must be durably persisted into state_json metadata"
        );
    }

    #[tokio::test]
    async fn resume_pending_preserves_bot_identity() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("database should initialize");
        let pool = database.pool();

        let bot_a = crate::SessionKey::new(
            "discord",
            None::<String>,
            "dm-100",
            None::<String>,
            "user-200",
        )
        .with_bot_id("42");
        let bot_b = crate::SessionKey::new(
            "discord",
            None::<String>,
            "dm-100",
            None::<String>,
            "user-200",
        )
        .with_bot_id("84");

        // Seed both sessions with identical DM coordinates:
        // Bot-A row is NOT pending (resume_pending = 0).
        // Bot-B row IS pending (resume_pending = 1).
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, resume_pending)
             VALUES (?, ?, ?, ?, ?, ?, '{}', 0)",
        )
        .bind(bot_a.storage_key())
        .bind(&bot_a.platform)
        .bind(&bot_a.guild_id)
        .bind(&bot_a.channel_id)
        .bind(&bot_a.thread_id)
        .bind(&bot_a.user_id)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, resume_pending)
             VALUES (?, ?, ?, ?, ?, ?, '{}', 1)",
        )
        .bind(bot_b.storage_key())
        .bind(&bot_b.platform)
        .bind(&bot_b.guild_id)
        .bind(&bot_b.channel_id)
        .bind(&bot_b.thread_id)
        .bind(&bot_b.user_id)
        .execute(pool)
        .await
        .unwrap();

        // Seed messages for each session
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-42', ?, 'user', 'completed turn for bot 42', '[]')",
        )
        .bind(bot_a.storage_key())
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('msg-84', ?, 'user', 'pending turn for bot 84', '[]')",
        )
        .bind(bot_b.storage_key())
        .execute(pool)
        .await
        .unwrap();

        // Include outbound-ledger identity case with its owner:
        // Record an obligation for bot-B from a dead process (recoverable)
        // and an obligation for bot-A that was already delivered.
        let dead_pid = 999_999_i64;
        let ledger = crate::DeliveryLedgerService::new(pool.clone());
        ledger
            .record_obligation("obl-bot-84", &bot_b, "outbound reply for bot 84")
            .await
            .unwrap();
        sqlx::query("UPDATE delivery_obligations SET owner_pid = ? WHERE id = 'obl-bot-84'")
            .bind(dead_pid)
            .execute(pool)
            .await
            .unwrap();

        ledger
            .record_obligation("obl-bot-42", &bot_a, "completed reply for bot 42")
            .await
            .unwrap();
        ledger
            .mark_obligation_delivered("obl-bot-42")
            .await
            .unwrap();

        // Step 1: fetch_resume_pending_session_keys must return exact bot-B key.
        // Under unpatched recovery, the returned key lacks B (bot_id is None).
        let pending_keys = super::fetch_resume_pending_session_keys(pool)
            .await
            .unwrap();
        assert_eq!(
            pending_keys.len(),
            1,
            "only bot-B should be returned as pending"
        );
        assert_eq!(
            pending_keys[0].bot_id.as_deref(),
            Some("84"),
            "returned key must retain bot identity '84'"
        );
        assert_eq!(
            pending_keys[0], bot_b,
            "returned key must be exact bot-B SessionKey"
        );

        // Step 2: Real SQLite recovery sweep and recording dispatch identity.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Option<String>, String)>(4);

        struct MockBotRecoveryRunner {
            tx: tokio::sync::mpsc::Sender<(Option<String>, String)>,
        }

        #[async_trait::async_trait]
        impl crate::AgentRunner for MockBotRecoveryRunner {
            async fn run(
                &self,
                session: &mut crate::SessionContext,
                event: crate::InboundEvent,
            ) -> crate::Result<()> {
                let _ = self
                    .tx
                    .send((session.key.bot_id.clone(), event.content))
                    .await;
                Ok(())
            }
        }

        let multiplexer = crate::SessionMultiplexer::new(
            pool.clone(),
            std::sync::Arc::new(MockBotRecoveryRunner { tx }),
            crate::MultiplexerConfig::default(),
        );

        let recovered = super::recover_resume_pending_sessions(pool, &multiplexer)
            .await
            .unwrap();

        // Under unpatched recovery, the botless key does not match bot-B's row in SQLite,
        // resulting in 0 recovered runs and bot-B remaining pending.
        assert_eq!(recovered, 1, "only pending bot-B session must be recovered");

        let (dispatched_bot, dispatched_content) =
            tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .expect("timed out waiting for dispatched event")
                .expect("channel closed");

        assert_eq!(
            dispatched_bot.as_deref(),
            Some("84"),
            "bot 84 turn must be dispatched with unchanged bot identity"
        );
        assert_eq!(dispatched_content, "pending turn for bot 84");

        assert!(
            rx.try_recv().is_err(),
            "bot-A was not pending and must never be dispatched"
        );

        // Step 3: Only B recovery eligibility changed; no blanket reset of other rows.
        let (pending_b,): (i64,) =
            sqlx::query_as("SELECT resume_pending FROM sessions WHERE session_key = ?")
                .bind(bot_b.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(
            pending_b, 0,
            "only bot-B resume_pending must be cleared to 0"
        );

        let (pending_a,): (i64,) =
            sqlx::query_as("SELECT resume_pending FROM sessions WHERE session_key = ?")
                .bind(bot_a.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(
            pending_a, 0,
            "bot-A resume_pending was 0 and must remain 0 (no blanket reset)"
        );

        // Step 4: Outbound-ledger identity case with its owner.
        let recoverable = ledger.sweep_recoverable(3, 86400).await.unwrap();
        assert_eq!(
            recoverable.len(),
            1,
            "only dead-owner obl-bot-84 should be recoverable"
        );
        assert_eq!(recoverable[0].id, "obl-bot-84");

        let parsed_owner_key = crate::SessionKey::from_storage_key(&recoverable[0].session_key)
            .expect("canonical storage key in delivery obligation must parse");
        assert_eq!(
            parsed_owner_key, bot_b,
            "outbound obligation owner key must match exact bot-B key"
        );
        assert_eq!(
            parsed_owner_key.bot_id.as_deref(),
            Some("84"),
            "outbound ledger owner bot identity must be preserved"
        );

        let (obl_a_state,): (String,) =
            sqlx::query_as("SELECT state FROM delivery_obligations WHERE id = 'obl-bot-42'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(
            obl_a_state, "delivered",
            "bot-A obligation remains delivered"
        );
    }

    #[tokio::test]
    async fn durable_thread_ownership_persists_across_restart() {
        let database = super::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();

        assert_eq!(
            super::get_thread_owner(pool, 8).await.unwrap(),
            None,
            "unregistered thread has no owner"
        );

        super::record_thread_owner(pool, 8, 84).await.unwrap();
        assert_eq!(
            super::get_thread_owner(pool, 8).await.unwrap(),
            Some(84),
            "thread owner is recorded"
        );

        // Update ownership
        super::record_thread_owner(pool, 8, 42).await.unwrap();
        assert_eq!(
            super::get_thread_owner(pool, 8).await.unwrap(),
            Some(42),
            "thread owner can be updated"
        );

        super::record_thread_owner(pool, 9, 84).await.unwrap();
        let all = super::load_all_thread_owners(pool).await.unwrap();
        assert_eq!(all.get(&8), Some(&42));
        assert_eq!(all.get(&9), Some(&84));
    }

    #[tokio::test]
    async fn inbound_preserves_original_platform_timestamp() {
        use crate::discord::adapter::{message_to_inbound_with_config, InboundFilterConfig};
        use crate::render_user_prompt;
        use crate::InboundEvent;
        use async_trait::async_trait;
        use serenity::all::{ChannelType, Message, UserId};
        use std::sync::Arc;

        let original_ts_str = "2020-01-01T00:00:00Z";
        let expected_time = chrono::DateTime::parse_from_rfc3339(original_ts_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        let raw_msg = serde_json::json!({
            "id": "1001",
            "channel_id": "2001",
            "guild_id": null,
            "author": {
                "id": "3001",
                "username": "alice",
                "discriminator": "0001",
                "avatar": null,
                "bot": false,
                "system": false,
                "mfa_enabled": false,
                "banner": null,
                "accent_color": null,
                "locale": null,
                "verified": false,
                "email": null,
                "flags": 0,
                "premium_type": 0,
                "public_flags": 0,
                "global_name": null,
                "avatar_decoration_data": null,
                "collectibles": null,
                "primary_guild": null
            },
            "content": "hello source time",
            "timestamp": original_ts_str,
            "edited_timestamp": null,
            "tts": false,
            "mention_everyone": false,
            "mentions": [],
            "mention_roles": [],
            "mention_channels": [],
            "attachments": [],
            "embeds": [],
            "reactions": [],
            "nonce": null,
            "pinned": false,
            "webhook_id": null,
            "type": 0,
            "activity": null,
            "application": null,
            "application_id": null,
            "message_reference": null,
            "flags": null,
            "referenced_message": null,
            "message_snapshots": [],
            "interaction": null,
            "interaction_metadata": null,
            "thread": null,
            "components": [],
            "sticker_items": [],
            "position": null,
            "role_subscription_data": null,
            "member": null,
            "poll": null
        });

        let message: Message = serde_json::from_value(raw_msg).unwrap();
        let bot_user_id = UserId::new(42);
        let config = InboundFilterConfig {
            allow_all_users: true,
            primary_bot_id: Some(42),
            ..Default::default()
        };

        let event = message_to_inbound_with_config(
            &message,
            bot_user_id,
            Some(ChannelType::Private),
            &config,
        )
        .expect("message should be converted to InboundEvent");

        // S.F17 assertion: Converted event MUST carry Discord message.timestamp, not Utc::now()
        assert_eq!(
            event.received_at, expected_time,
            "InboundEvent::received_at must preserve source platform timestamp (2020-01-01T00:00:00Z), not ingestion time"
        );

        // Verify prompt rendering uses source time
        let rendered = render_user_prompt(&event);
        assert!(
            rendered.contains("2020-01-01"),
            "rendered prompt must reflect source timestamp: got {rendered}"
        );

        // Verify recovery created_at preserves source time
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool();

        let session_key = event.session.storage_key();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json, resume_pending)
             VALUES (?, 'discord', NULL, '2001', NULL, '3001', '{}', 1)",
        )
        .bind(&session_key)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at, platform_message_id)
             VALUES (?, ?, 'user', ?, '[]', ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(&session_key)
        .bind(&event.content)
        .bind(event.received_at)
        .bind(&event.platform_message_id)
        .execute(pool)
        .await
        .unwrap();

        struct CheckRunner {
            ran_tx: tokio::sync::mpsc::UnboundedSender<chrono::DateTime<chrono::Utc>>,
        }
        #[async_trait]
        impl crate::AgentRunner for CheckRunner {
            async fn run(
                &self,
                _session: &mut crate::SessionContext,
                ev: InboundEvent,
            ) -> crate::Result<()> {
                let _ = self.ran_tx.send(ev.received_at);
                Ok(())
            }
        }
        let (ran_tx, mut ran_rx) = tokio::sync::mpsc::unbounded_channel();
        let runner = Arc::new(CheckRunner { ran_tx });
        let multiplexer = crate::SessionMultiplexer::new(
            pool.clone(),
            runner.clone(),
            crate::MultiplexerConfig::default(),
        );

        let recovered_count = super::recover_resume_pending_sessions(pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered_count, 1);
        let recovered_ts = tokio::time::timeout(std::time::Duration::from_secs(5), ran_rx.recv())
            .await
            .expect("recovered turn must reach the runner within 5s")
            .expect("runner channel must yield the recovered turn");
        assert_eq!(
            recovered_ts, expected_time,
            "recovered turn created_at must match original source timestamp"
        );
    }
}

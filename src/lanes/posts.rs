//! Single-owner durable post delivery; uncertain sends are never retried blindly.
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, CreateMessage, MessageId, Nonce};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::lanes::discord_threads::{safe_allowed_mentions, DiscordThreads};
use crate::lanes::scrub::scrub;
use crate::storage::Lane;
use crate::{OmonError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostState {
    Pending,
    Sending,
    Sent,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub source: String,
    pub thread: Option<String>,
    pub body: String,
    pub nonce: String,
    pub state: PostState,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

fn json_error(error: serde_json::Error) -> OmonError {
    OmonError::Config(format!("lane post JSON: {error}"))
}

pub async fn enqueue(
    tx: &mut Transaction<'_, Sqlite>,
    lane: &Lane,
    source: &str,
    body: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let post = Post {
        source: source.into(),
        thread: lane.thread_channel_id.clone(),
        body: scrub(body).chars().take(2000).collect(),
        nonce: uuid::Uuid::new_v4().simple().to_string()[..25].into(),
        state: PostState::Pending,
        message_id: None,
        error: None,
    };
    sqlx::query("INSERT INTO lane_transcript(lane_key, at, role, text) SELECT ?, ?, 'thread_post', ? WHERE NOT EXISTS (SELECT 1 FROM lane_transcript WHERE lane_key = ? AND role = 'thread_post' AND json_valid(text) AND json_extract(text, '$.source') = ?)")
        .bind(&lane.key).bind(now).bind(serde_json::to_string(&post).map_err(json_error)?)
        .bind(&lane.key).bind(source).execute(&mut **tx).await?;
    Ok(())
}

#[async_trait]
pub trait PostSink: Send + Sync {
    async fn send(&self, post: &Post) -> Result<String>;

    /// False means the message was deleted, not an ambiguous transport error.
    async fn edit_status(&self, _thread: u64, _message: u64, _body: &str) -> Result<bool> {
        Err(OmonError::Config(
            "status edits are unsupported by this sink".into(),
        ))
    }

    async fn rename_status(
        &self,
        _thread: u64,
        _status: super::discord_threads::ThreadStatus,
        _title: &str,
    ) -> Result<()> {
        Err(OmonError::Config(
            "status renames are unsupported by this sink".into(),
        ))
    }
}

#[async_trait]
impl PostSink for DiscordThreads {
    async fn send(&self, post: &Post) -> Result<String> {
        let thread = post
            .thread
            .as_deref()
            .and_then(|id| id.parse::<u64>().ok())
            .filter(|id| *id != 0)
            .ok_or_else(|| OmonError::Config("lane post has no valid thread destination".into()))?;
        let message = ChannelId::new(thread)
            .send_message(
                self.http(),
                CreateMessage::new()
                    .content(&post.body)
                    .allowed_mentions(
                        safe_allowed_mentions()
                            .all_users(false)
                            .empty_users()
                            .replied_user(false),
                    )
                    .nonce(Nonce::String(post.nonce.clone()))
                    .enforce_nonce(true),
            )
            .await
            .map_err(|error| OmonError::Config(format!("lane post transport: {error}")))?;
        Ok(message.id.to_string())
    }

    async fn edit_status(&self, thread: u64, message: u64, body: &str) -> Result<bool> {
        match ChannelId::new(thread)
            .edit_message(
                self.http(),
                MessageId::new(message),
                serenity::all::EditMessage::new()
                    .content(body)
                    .allowed_mentions(
                        safe_allowed_mentions()
                            .all_users(false)
                            .empty_users()
                            .replied_user(false),
                    ),
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(error) => {
                let error = super::discord_threads::DiscordThreadError::from(error);
                if error.status_code() == Some(reqwest::StatusCode::NOT_FOUND)
                    && error.discord_code() == Some(10008)
                {
                    Ok(false)
                } else {
                    Err(OmonError::Config(format!("lane status edit: {error}")))
                }
            }
        }
    }

    async fn rename_status(
        &self,
        thread: u64,
        status: super::discord_threads::ThreadStatus,
        title: &str,
    ) -> Result<()> {
        self.set_status_with_title(ChannelId::new(thread), status, &scrub(title))
            .await
            .map_err(|error| OmonError::Config(format!("lane status rename: {error}")))?;
        Ok(())
    }
}

/// Call once at service startup, before starting its sole delivery worker.
pub async fn recover(pool: &SqlitePool) -> Result<()> {
    sqlx::query("UPDATE lane_transcript SET text = json_set(text, '$.state', 'uncertain', '$.error', 'process stopped before delivery acknowledgement') WHERE role = 'thread_post' AND json_valid(text) AND json_extract(text, '$.state') = 'sending'")
        .execute(pool).await?;
    sqlx::query("UPDATE lane_transcript SET text = json_set(text, '$.post.state', 'uncertain', '$.post.error', 'process stopped before status acknowledgement') WHERE role = 'status_receipt' AND json_valid(text) AND json_extract(text, '$.post.state') = 'sending'")
        .execute(pool).await?;
    Ok(())
}

/// One event-driven drain; receipts, not queued intents, determine success.
pub async fn drain(pool: &SqlitePool, sink: &dyn PostSink) -> Result<usize> {
    drain_inner(pool, sink, None).await
}

/// The sole delivery worker uses this entry point for milestone-triggered edits.
pub async fn drain_with_status(
    pool: &SqlitePool,
    sink: &dyn PostSink,
    now: DateTime<Utc>,
) -> Result<usize> {
    drain_inner(pool, sink, Some(now)).await
}

async fn drain_inner(
    pool: &SqlitePool,
    sink: &dyn PostSink,
    status_at: Option<DateTime<Utc>>,
) -> Result<usize> {
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, text FROM lane_transcript WHERE role = 'thread_post' AND json_valid(text) AND json_extract(text, '$.state') = 'pending' ORDER BY id")
        .fetch_all(pool).await?;
    let mut sent = 0;
    for (id, original) in rows {
        let mut post: Post = serde_json::from_str(&original).map_err(json_error)?;
        if post
            .thread
            .as_deref()
            .and_then(|id| id.parse::<u64>().ok())
            .filter(|id| *id != 0)
            .is_none()
        {
            post.error = Some("missing or invalid thread destination; no send attempted".into());
            sqlx::query("UPDATE lane_transcript SET text = ? WHERE id = ? AND text = ?")
                .bind(serde_json::to_string(&post).map_err(json_error)?)
                .bind(id)
                .bind(original)
                .execute(pool)
                .await?;
            continue;
        }
        post.state = PostState::Sending;
        let sending = serde_json::to_string(&post).map_err(json_error)?;
        if sqlx::query("UPDATE lane_transcript SET text = ? WHERE id = ? AND text = ?")
            .bind(&sending)
            .bind(id)
            .bind(&original)
            .execute(pool)
            .await?
            .rows_affected()
            == 0
        {
            continue;
        }
        match tokio::time::timeout(std::time::Duration::from_secs(30), sink.send(&post)).await {
            Ok(Ok(message_id)) => {
                post.state = PostState::Sent;
                post.message_id = Some(message_id);
                post.error = None;
                sent += 1;
            }
            Ok(Err(error)) => {
                post.state = PostState::Uncertain;
                post.error = Some(error.to_string());
            }
            Err(error) => {
                post.state = PostState::Uncertain;
                post.error = Some(error.to_string());
            }
        }
        sqlx::query("UPDATE lane_transcript SET text = ? WHERE id = ? AND text = ?")
            .bind(serde_json::to_string(&post).map_err(json_error)?)
            .bind(id)
            .bind(sending)
            .execute(pool)
            .await?;
        if post.state == PostState::Sent && !post.source.starts_with("stale:") {
            if let Some(now) = status_at {
                let key: String =
                    sqlx::query_scalar("SELECT lane_key FROM lane_transcript WHERE id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await?;
                refresh_lane_status(pool, sink, &key, now, true).await?;
            }
        }
    }
    Ok(sent)
}

#[derive(Serialize, Deserialize)]
struct StatusReceipt {
    post: Post,
    at: DateTime<Utc>,
    status: String,
}

/// Cheap local timer target: no model calls, and no active-lane edit before 15 minutes.
pub async fn refresh_statuses(
    pool: &SqlitePool,
    sink: &dyn PostSink,
    now: DateTime<Utc>,
) -> Result<()> {
    let keys: Vec<String> =
        // Closed lanes have archived threads; Discord rejects edits there (50083).
        sqlx::query_scalar(
            "SELECT key FROM lanes WHERE thread_channel_id IS NOT NULL AND status != 'closed'",
        )
        .fetch_all(pool)
        .await?;
    for key in keys {
        // One lane's Discord failure must not stop the lane service (or startup).
        if let Err(error) = refresh_lane_status(pool, sink, &key, now, false).await {
            tracing::warn!(%key, %error, "lane status refresh failed");
        }
    }
    Ok(())
}

/// Single-owner status refresh. Creation intents and acknowledgements are durable;
/// an uncertain creation blocks replacement until its receipt is reconciled.
pub async fn refresh_lane_status(
    pool: &SqlitePool,
    sink: &dyn PostSink,
    key: &str,
    now: DateTime<Utc>,
    milestone: bool,
) -> Result<()> {
    let Some(lane) = crate::storage::get_lane(pool, key).await? else {
        return Ok(());
    };
    let Some(thread) = lane
        .thread_channel_id
        .as_deref()
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id != 0)
    else {
        return Ok(());
    };
    let prior: Option<String> = sqlx::query_scalar("SELECT text FROM lane_transcript WHERE lane_key = ? AND role = 'status_receipt' ORDER BY id DESC LIMIT 1")
        .bind(key).fetch_optional(pool).await?;
    let prior = prior
        .map(|text| serde_json::from_str::<StatusReceipt>(&text))
        .transpose()
        .map_err(json_error)?;
    if prior.as_ref().is_some_and(|receipt| {
        matches!(
            receipt.post.state,
            PostState::Sending | PostState::Uncertain
        )
    }) {
        return Ok(());
    }
    let changed = prior
        .as_ref()
        .is_none_or(|receipt| receipt.status != lane.status);
    let active = matches!(
        lane.status.as_str(),
        "working" | "not_done" | "queued" | "blocked"
    );
    if !milestone
        && !changed
        && (!active
            || prior
                .as_ref()
                .is_some_and(|receipt| now - receipt.at < chrono::Duration::minutes(15)))
    {
        return Ok(());
    }
    let status = match lane.status.as_str() {
        "verified_done" | "closed" => super::discord_threads::ThreadStatus::Done,
        "blocked" | "question" | "queued" | "failed" | "not_done" => {
            super::discord_threads::ThreadStatus::Waiting
        }
        _ => super::discord_threads::ThreadStatus::Working,
    };
    if changed {
        sink.rename_status(thread, status, &lane.title).await?;
    }
    let elapsed = (now - lane.created_at.unwrap_or(now)).num_minutes().max(0);
    let body: String = scrub(&format!(
        "⏳ {} · {}m · turn {}",
        lane.title, elapsed, lane.turns_run
    ))
    .chars()
    .take(2000)
    .collect();
    let mut receipt = StatusReceipt {
        post: Post {
            source: format!("status:{}", uuid::Uuid::new_v4()),
            thread: lane.thread_channel_id.clone(),
            body,
            nonce: uuid::Uuid::new_v4().simple().to_string()[..25].into(),
            state: PostState::Sending,
            message_id: None,
            error: None,
        },
        at: now,
        status: lane.status.clone(),
    };
    let existing = lane
        .status_message_id
        .as_deref()
        .map(|id| id.parse::<u64>())
        .transpose()
        .map_err(|error| OmonError::Config(format!("invalid lane status message id: {error}")))?;
    let edited = if let Some(message) = existing {
        sink.edit_status(thread, message, &receipt.post.body)
            .await?
    } else {
        false
    };
    if edited {
        receipt.post.state = PostState::Sent;
        receipt.post.message_id = lane.status_message_id;
    }
    let row = sqlx::query(
        "INSERT INTO lane_transcript(lane_key, at, role, text) VALUES (?, ?, 'status_receipt', ?)",
    )
    .bind(key)
    .bind(now)
    .bind(serde_json::to_string(&receipt).map_err(json_error)?)
    .execute(pool)
    .await?
    .last_insert_rowid();
    if !edited {
        match tokio::time::timeout(std::time::Duration::from_secs(30), sink.send(&receipt.post))
            .await
        {
            Ok(Ok(id)) => {
                receipt.post.state = PostState::Sent;
                receipt.post.message_id = Some(id);
            }
            Ok(Err(error)) => {
                receipt.post.state = PostState::Uncertain;
                receipt.post.error = Some(scrub(&error.to_string()));
            }
            Err(error) => {
                receipt.post.state = PostState::Uncertain;
                receipt.post.error = Some(scrub(&error.to_string()));
            }
        }
    }
    let mut tx = pool.begin().await?;
    if receipt.post.state == PostState::Sent {
        sqlx::query("UPDATE lanes SET status_message_id = ? WHERE key = ?")
            .bind(&receipt.post.message_id)
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE lane_transcript SET text = ? WHERE id = ?")
        .bind(serde_json::to_string(&receipt).map_err(json_error)?)
        .bind(row)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

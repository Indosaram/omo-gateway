use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, SqlitePool};

use crate::{InboundEvent, OmonError, Result, SessionKey};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageSearchDocument {
    pub platform: String,
    pub guild_id: Option<String>,
    pub channel_id: String,
    pub thread_id: Option<String>,
    pub message_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub attachment_names: Vec<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageSearchHit {
    pub document: MessageSearchDocument,
    pub rank: f64,
}

#[derive(Clone)]
pub struct MessageSearchIndex {
    pool: SqlitePool,
}

impl MessageSearchIndex {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Validates that `query` has at least one searchable term without touching the database.
    pub fn validate_query(query: &str) -> Result<()> {
        build_fts_query(query).map(|_| ())
    }

    pub async fn upsert(&self, document: &MessageSearchDocument) -> Result<()> {
        let attachment_names = serde_json::to_string(&document.attachment_names)
            .map_err(|error| OmonError::Database(error.to_string()))?;
        let metadata_json = serde_json::to_string(&document.metadata)
            .map_err(|error| OmonError::Database(error.to_string()))?;
        sqlx::query(
            "INSERT INTO message_search_documents (
                platform, guild_id, channel_id, thread_id, message_id,
                author_id, author_name, content, attachment_names, timestamp, metadata_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(platform, channel_id, message_id) DO UPDATE SET
                guild_id = excluded.guild_id,
                thread_id = excluded.thread_id,
                author_id = excluded.author_id,
                author_name = excluded.author_name,
                content = excluded.content,
                attachment_names = excluded.attachment_names,
                timestamp = excluded.timestamp,
                metadata_json = excluded.metadata_json",
        )
        .bind(&document.platform)
        .bind(&document.guild_id)
        .bind(&document.channel_id)
        .bind(&document.thread_id)
        .bind(&document.message_id)
        .bind(&document.author_id)
        .bind(&document.author_name)
        .bind(&document.content)
        .bind(attachment_names)
        .bind(document.timestamp)
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_many(&self, documents: &[MessageSearchDocument]) -> Result<usize> {
        if documents.is_empty() {
            return Ok(0);
        }
        let mut transaction = self.pool.begin().await?;
        for document in documents {
            let attachment_names = serde_json::to_string(&document.attachment_names)
                .map_err(|error| OmonError::Database(error.to_string()))?;
            let metadata_json = serde_json::to_string(&document.metadata)
                .map_err(|error| OmonError::Database(error.to_string()))?;
            sqlx::query(
                "INSERT INTO message_search_documents (
                    platform, guild_id, channel_id, thread_id, message_id,
                    author_id, author_name, content, attachment_names, timestamp, metadata_json
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(platform, channel_id, message_id) DO UPDATE SET
                    guild_id = excluded.guild_id,
                    thread_id = excluded.thread_id,
                    author_id = excluded.author_id,
                    author_name = excluded.author_name,
                    content = excluded.content,
                    attachment_names = excluded.attachment_names,
                    timestamp = excluded.timestamp,
                    metadata_json = excluded.metadata_json",
            )
            .bind(&document.platform)
            .bind(&document.guild_id)
            .bind(&document.channel_id)
            .bind(&document.thread_id)
            .bind(&document.message_id)
            .bind(&document.author_id)
            .bind(&document.author_name)
            .bind(&document.content)
            .bind(attachment_names)
            .bind(document.timestamp)
            .bind(metadata_json)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(documents.len())
    }

    pub async fn get(
        &self,
        platform: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<Option<MessageSearchDocument>> {
        let row = sqlx::query_as::<_, MessageSearchRow>(
            "SELECT platform, guild_id, channel_id, thread_id, message_id,
                    author_id, author_name, content, attachment_names, timestamp, metadata_json
             FROM message_search_documents
             WHERE platform = ? AND channel_id = ? AND message_id = ?",
        )
        .bind(platform)
        .bind(channel_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(MessageSearchRow::into_document).transpose()
    }

    pub async fn search(
        &self,
        platform: &str,
        channel_id: &str,
        query: &str,
        before_message_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MessageSearchHit>> {
        let fts_query = build_fts_query(query)?;
        if let Some(before) = before_message_id {
            // A non-numeric cursor would CAST to 0 and silently exclude everything.
            if before.is_empty() || !before.chars().all(|c| c.is_ascii_digit()) {
                return Err(OmonError::ToolExecution(
                    "before_message_id must be a numeric message id".into(),
                ));
            }
        }
        let rows = sqlx::query_as::<_, MessageSearchHitRow>(
            "SELECT d.platform, d.guild_id, d.channel_id, d.thread_id, d.message_id,
                    d.author_id, d.author_name, d.content, d.attachment_names,
                    d.timestamp, d.metadata_json, bm25(message_search_fts) AS rank
             FROM message_search_fts
             JOIN message_search_documents d ON d.rowid = message_search_fts.rowid
             WHERE message_search_fts MATCH ?
               AND d.platform = ?
               AND d.channel_id = ?
               AND (d.thread_id IS NULL OR d.thread_id = d.channel_id)
               AND (? IS NULL OR CAST(d.message_id AS INTEGER) < CAST(? AS INTEGER))
             ORDER BY rank ASC, d.timestamp DESC
             LIMIT ?",
        )
        .bind(fts_query)
        .bind(platform)
        .bind(channel_id)
        .bind(before_message_id)
        .bind(before_message_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(MessageSearchHitRow::into_hit)
            .collect()
    }

    pub async fn count(&self, platform: &str, channel_id: &str) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM message_search_documents WHERE platform = ? AND channel_id = ?",
        )
        .bind(platform)
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn index_inbound_with_metadata(
        &self,
        session: &SessionKey,
        event: &InboundEvent,
        metadata: serde_json::Value,
    ) -> Result<bool> {
        if event.platform_message_id.trim().is_empty() {
            return Ok(false);
        }
        let attachment_names = event
            .attachments
            .iter()
            .map(|attachment| attachment.filename.clone())
            .collect::<Vec<_>>();
        let mut meta_map = match metadata {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        meta_map.insert("source".into(), json!("ingress"));
        if let Some(delivery_id) = &event.delivery_id {
            meta_map.insert("delivery_id".into(), json!(delivery_id));
        }
        let author_id = event
            .author_id
            .as_ref()
            .filter(|a| !a.is_empty())
            .cloned()
            .unwrap_or_else(|| session.user_id.clone());
        // A thread's messages are keyed by the thread id (as the REST backfill does),
        // never by the parent channel, so a parent-channel search cannot return
        // private-thread content.
        let document = MessageSearchDocument {
            platform: session.platform.clone(),
            guild_id: session.guild_id.clone(),
            channel_id: session
                .thread_id
                .clone()
                .unwrap_or_else(|| session.channel_id.clone()),
            thread_id: session.thread_id.clone(),
            message_id: event.platform_message_id.clone(),
            author_id,
            author_name: session.user_id.clone(),
            content: event.content.clone(),
            attachment_names,
            timestamp: event.received_at,
            metadata: serde_json::Value::Object(meta_map),
        };
        self.upsert(&document).await?;
        Ok(true)
    }

    pub async fn index_inbound(&self, session: &SessionKey, event: &InboundEvent) -> Result<bool> {
        let mut metadata = json!({});
        if let Some(ref_id) = &event.referenced_message_id {
            metadata["referenced_message_id"] = json!(ref_id);
        }
        self.index_inbound_with_metadata(session, event, metadata)
            .await
    }
}

fn build_fts_query(query: &str) -> Result<String> {
    let terms = query
        .split_whitespace()
        .map(|term| term.trim_matches('"').replace('"', ""))
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Err(OmonError::ToolExecution(
            "message search query must contain at least one searchable term".into(),
        ));
    }
    Ok(terms.join(" AND "))
}

#[derive(FromRow)]
struct MessageSearchRow {
    platform: String,
    guild_id: Option<String>,
    channel_id: String,
    thread_id: Option<String>,
    message_id: String,
    author_id: String,
    author_name: String,
    content: String,
    attachment_names: String,
    timestamp: DateTime<Utc>,
    metadata_json: String,
}

impl MessageSearchRow {
    fn into_document(self) -> Result<MessageSearchDocument> {
        Ok(MessageSearchDocument {
            platform: self.platform,
            guild_id: self.guild_id,
            channel_id: self.channel_id,
            thread_id: self.thread_id,
            message_id: self.message_id,
            author_id: self.author_id,
            author_name: self.author_name,
            content: self.content,
            attachment_names: serde_json::from_str(&self.attachment_names).unwrap_or_default(),
            timestamp: self.timestamp,
            metadata: serde_json::from_str(&self.metadata_json).unwrap_or_else(|_| json!({})),
        })
    }
}

#[derive(FromRow)]
struct MessageSearchHitRow {
    platform: String,
    guild_id: Option<String>,
    channel_id: String,
    thread_id: Option<String>,
    message_id: String,
    author_id: String,
    author_name: String,
    content: String,
    attachment_names: String,
    timestamp: DateTime<Utc>,
    metadata_json: String,
    rank: f64,
}

impl MessageSearchHitRow {
    fn into_hit(self) -> Result<MessageSearchHit> {
        let rank = self.rank;
        let row = MessageSearchRow {
            platform: self.platform,
            guild_id: self.guild_id,
            channel_id: self.channel_id,
            thread_id: self.thread_id,
            message_id: self.message_id,
            author_id: self.author_id,
            author_name: self.author_name,
            content: self.content,
            attachment_names: self.attachment_names,
            timestamp: self.timestamp,
            metadata_json: self.metadata_json,
        };
        Ok(MessageSearchHit {
            document: row.into_document()?,
            rank,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn document(id: &str, content: &str) -> MessageSearchDocument {
        MessageSearchDocument {
            platform: "discord".into(),
            guild_id: Some("1".into()),
            channel_id: "2".into(),
            thread_id: None,
            message_id: id.into(),
            author_id: "3".into(),
            author_name: "alice".into(),
            content: content.into(),
            attachment_names: vec!["notes.txt".into()],
            timestamp: Utc::now(),
            metadata: json!({"source":"test"}),
        }
    }

    #[tokio::test]
    async fn fts_upsert_search_and_update_round_trip() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let index = MessageSearchIndex::new(db.pool().clone());
        index
            .upsert(&document("100", "alpha project status"))
            .await
            .unwrap();
        index
            .upsert(&document("101", "beta release checklist"))
            .await
            .unwrap();

        let alpha = index
            .search("discord", "2", "alpha", None, 10)
            .await
            .unwrap();
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].document.message_id, "100");

        let mut updated = document("100", "gamma project status");
        updated.author_name = "bob".into();
        index.upsert(&updated).await.unwrap();
        assert!(index
            .search("discord", "2", "alpha", None, 10)
            .await
            .unwrap()
            .is_empty());
        let gamma = index
            .search("discord", "2", "gamma", None, 10)
            .await
            .unwrap();
        assert_eq!(gamma[0].document.author_name, "bob");
    }

    #[tokio::test]
    async fn fts_scopes_channel_and_honors_discord_before_cursor() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let index = MessageSearchIndex::new(db.pool().clone());
        index.upsert(&document("100", "needle old")).await.unwrap();
        index.upsert(&document("200", "needle new")).await.unwrap();
        let mut other = document("150", "needle other channel");
        other.channel_id = "9".into();
        index.upsert(&other).await.unwrap();

        let hits = index
            .search("discord", "2", "needle", Some("200"), 10)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document.message_id, "100");
    }

    #[tokio::test]
    async fn non_numeric_before_cursor_is_rejected() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let index = MessageSearchIndex::new(db.pool().clone());
        index.upsert(&document("100", "needle")).await.unwrap();

        assert!(index
            .search("discord", "2", "needle", Some("abc"), 10)
            .await
            .is_err());
    }

    #[test]
    fn validate_query_requires_a_term() {
        assert!(MessageSearchIndex::validate_query("  \"\" ").is_err());
        assert!(MessageSearchIndex::validate_query("needle").is_ok());
    }

    #[tokio::test]
    async fn thread_messages_are_not_returned_to_parent_channel_search() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let index = MessageSearchIndex::new(db.pool().clone());
        let parent = SessionKey::new("discord", Some("1"), "2", None::<String>, "u");
        let thread = SessionKey::new("discord", Some("1"), "2", Some("77"), "u");
        let mut parent_event = InboundEvent::message(parent.clone(), "100", "needle public");
        parent_event.received_at = Utc::now();
        let mut thread_event = InboundEvent::message(thread.clone(), "101", "needle private");
        thread_event.received_at = Utc::now();
        assert!(index.index_inbound(&parent, &parent_event).await.unwrap());
        assert!(index.index_inbound(&thread, &thread_event).await.unwrap());

        let parent_hits = index
            .search("discord", "2", "needle", None, 10)
            .await
            .unwrap();
        assert_eq!(parent_hits.len(), 1);
        assert_eq!(parent_hits[0].document.message_id, "100");

        let thread_hits = index
            .search("discord", "77", "needle", None, 10)
            .await
            .unwrap();
        assert_eq!(thread_hits.len(), 1);
        assert_eq!(thread_hits[0].document.message_id, "101");
        assert_eq!(thread_hits[0].document.thread_id.as_deref(), Some("77"));
    }

    #[tokio::test]
    async fn transcript_trigger_thread_rows_are_hidden_from_parent_search() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let index = MessageSearchIndex::new(db.pool().clone());
        // The legacy transcript trigger stores thread rows under the parent channel id.
        let mut legacy = document("300", "needle legacy thread");
        legacy.thread_id = Some("77".into());
        index.upsert(&legacy).await.unwrap();
        index
            .upsert(&document("301", "needle plain"))
            .await
            .unwrap();

        let hits = index
            .search("discord", "2", "needle", None, 10)
            .await
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document.message_id, "301");
    }
}

//! Read-only inbox collection. Sources return metadata and bounded snippets only.

pub mod cli;
pub mod digest;
pub mod health;
pub mod list;
pub mod owner_alert;
pub mod prefilter;
pub mod proposals;
pub mod set;
pub mod sources;
pub mod triage;
pub mod watchers;

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::{OmonError, Result};

pub const MAX_SNIPPET_CHARS: usize = 1000;
pub const DEFAULT_FILTERS_PATH: &str = "~/.omon/inbox-filters.json";

pub fn default_filters_path() -> std::path::PathBuf {
    let raw = DEFAULT_FILTERS_PATH
        .strip_prefix("~/")
        .expect("home-relative default");
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(raw)
}

pub fn default_filters() -> Result<prefilter::PrefilterRules> {
    prefilter::PrefilterRules::load_or_create(default_filters_path())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InboxSourceKind {
    Mail,
    Kakao,
    Discord,
    Imessage,
    Slack,
    Github,
}

impl InboxSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mail => "mail",
            Self::Kakao => "kakao",
            Self::Discord => "discord",
            Self::Imessage => "imessage",
            Self::Slack => "slack",
            Self::Github => "github",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InboxItem {
    pub source: InboxSourceKind,
    pub source_ref: String,
    pub thread_ref: Option<String>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub room_type: Option<String>,
    pub mail_categories: Vec<String>,
    pub snippet: String,
    pub received_at: DateTime<Utc>,
    pub link: Option<String>,
    /// Stable content digest; the raw body is deliberately not part of this type.
    pub raw_hash: String,
}

impl InboxItem {
    pub fn new(
        source: InboxSourceKind,
        source_ref: impl Into<String>,
        snippet: impl AsRef<str>,
        received_at: DateTime<Utc>,
    ) -> Self {
        let snippet = cap_chars(snippet.as_ref(), MAX_SNIPPET_CHARS);
        let raw_hash = format!("{:x}", Sha256::digest(snippet.as_bytes()));
        Self {
            source,
            source_ref: source_ref.into(),
            thread_ref: None,
            author: None,
            title: None,
            room_type: None,
            mail_categories: Vec::new(),
            snippet,
            received_at,
            link: None,
            raw_hash,
        }
    }

    pub fn id(&self) -> String {
        let digest = Sha256::digest(format!("{}\0{}", self.source.as_str(), self.source_ref));
        format!("{:x}", digest)
    }
}

fn cap_chars(value: &str, cap: usize) -> String {
    value.chars().take(cap).collect()
}

#[async_trait]
pub trait InboxSource: Send + Sync {
    fn name(&self) -> InboxSourceKind;
    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CollectStats {
    pub inserted: usize,
    pub duplicates: usize,
    pub dropped: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCollectResult {
    pub source: InboxSourceKind,
    pub stats: CollectStats,
    pub error: Option<String>,
}

pub async fn collect_all(
    pool: &SqlitePool,
    sources: &[&dyn InboxSource],
    rules: &prefilter::PrefilterRules,
) -> Vec<SourceCollectResult> {
    let mut results = Vec::with_capacity(sources.len());
    for source in sources {
        let kind = source.name();
        let fetched = tokio::time::timeout(Duration::from_secs(60), async {
            let cursor = sqlx::query_scalar::<_, String>(
                "SELECT cursor FROM inbox_cursors WHERE source = ?",
            )
            .bind(kind.as_str())
            .fetch_optional(pool)
            .await?;
            source.fetch_since(cursor.as_deref()).await
        })
        .await;

        let result = match fetched {
            Ok(Ok((items, next_cursor))) => {
                match commit_source(pool, kind, items, next_cursor, rules).await {
                    Ok(stats) => SourceCollectResult {
                        source: kind,
                        stats,
                        error: None,
                    },
                    Err(error) => {
                        record_source_error(pool, kind, &error.to_string()).await;
                        SourceCollectResult {
                            source: kind,
                            stats: CollectStats::default(),
                            error: Some(error.to_string()),
                        }
                    }
                }
            }
            Ok(Err(error)) => {
                record_source_error(pool, kind, &error.to_string()).await;
                SourceCollectResult {
                    source: kind,
                    stats: CollectStats::default(),
                    error: Some(error.to_string()),
                }
            }
            Err(_) => {
                let message = "source collection timed out after 60 seconds";
                record_source_error(pool, kind, message).await;
                SourceCollectResult {
                    source: kind,
                    stats: CollectStats::default(),
                    error: Some(message.to_owned()),
                }
            }
        };
        results.push(result);
    }
    results
}

async fn commit_source(
    pool: &SqlitePool,
    source: InboxSourceKind,
    items: Vec<InboxItem>,
    next_cursor: String,
    rules: &prefilter::PrefilterRules,
) -> Result<CollectStats> {
    let mut tx = pool.begin().await?;
    let mut stats = CollectStats::default();
    for mut item in items {
        if item.source != source {
            return Err(OmonError::Config(format!(
                "{} source returned {} item",
                source.as_str(),
                item.source.as_str()
            )));
        }
        item.snippet = cap_chars(&item.snippet, MAX_SNIPPET_CHARS);
        if item.source_ref.trim().is_empty() || item.snippet.trim().is_empty() {
            prefilter::record_drop(&mut tx, source, "empty_source_ref_or_snippet").await?;
            stats.dropped += 1;
            continue;
        }
        if let Some(rule) = rules.drop_reason(&item) {
            prefilter::record_drop(&mut tx, source, rule).await?;
            stats.dropped += 1;
            continue;
        }
        if source == InboxSourceKind::Mail {
            // Mail items are keyed by Gmail thread id, so a follow-up message in an
            // already-collected thread arrives as the same source_ref with a new
            // snippet. Refresh the row and drop stale decisions so it is re-triaged.
            let existing: Option<(String, String)> = sqlx::query_as(
                "SELECT id, raw_hash FROM inbox_items WHERE source = ? AND source_ref = ?",
            )
            .bind(source.as_str())
            .bind(&item.source_ref)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some((existing_id, existing_hash)) = existing {
                if existing_hash == item.raw_hash {
                    stats.duplicates += 1;
                } else {
                    sqlx::query(
                        "UPDATE inbox_items SET author = ?, title = ?, snippet = ?, received_at = ?, \
                         link = ?, raw_hash = ?, collected_at = ? WHERE id = ?",
                    )
                    .bind(&item.author)
                    .bind(&item.title)
                    .bind(&item.snippet)
                    .bind(item.received_at)
                    .bind(&item.link)
                    .bind(&item.raw_hash)
                    .bind(Utc::now())
                    .bind(&existing_id)
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query(
                        "DELETE FROM inbox_decisions WHERE item_id = ? \
                         AND status IN ('open', 'done', 'dismissed')",
                    )
                    .bind(&existing_id)
                    .execute(&mut *tx)
                    .await?;
                    stats.inserted += 1;
                }
                continue;
            }
        }
        let item_id = item.id();
        let result = sqlx::query(
            "INSERT INTO inbox_items(id, source, source_ref, thread_ref, author, title, snippet, received_at, link, raw_hash, collected_at) \
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(source, source_ref) DO NOTHING",
        )
        .bind(&item_id)
        .bind(source.as_str())
        .bind(item.source_ref)
        .bind(item.thread_ref)
        .bind(item.author)
        .bind(item.title)
        .bind(item.snippet)
        .bind(item.received_at)
        .bind(item.link)
        .bind(item.raw_hash)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            stats.duplicates += 1;
        } else {
            stats.inserted += 1;
        }
    }
    sqlx::query(
        "INSERT INTO inbox_cursors(source, cursor, updated_at, error) VALUES(?, ?, ?, NULL) \
         ON CONFLICT(source) DO UPDATE SET cursor=excluded.cursor, updated_at=excluded.updated_at, error=NULL",
    )
    .bind(source.as_str())
    .bind(next_cursor)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM source_health WHERE source = ?")
        .bind(source.as_str())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(stats)
}

async fn record_source_error(pool: &SqlitePool, source: InboxSourceKind, error: &str) {
    let _ = sqlx::query(
        "INSERT INTO inbox_cursors(source, cursor, updated_at, error) VALUES(?, '', ?, ?) \
         ON CONFLICT(source) DO UPDATE SET updated_at=excluded.updated_at, error=excluded.error",
    )
    .bind(source.as_str())
    .bind(Utc::now())
    .bind(error)
    .execute(pool)
    .await;
}

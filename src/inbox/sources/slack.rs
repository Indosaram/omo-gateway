use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

#[derive(Clone)]
pub struct SlackSource {
    pool: SqlitePool,
    owner_id: String,
    bot_id: String,
}

impl SlackSource {
    pub fn new(pool: SqlitePool, owner_slack_id: String, bot_slack_id: String) -> Self {
        Self {
            pool,
            owner_id: owner_slack_id,
            bot_id: bot_slack_id,
        }
    }
}

#[async_trait]
impl InboxSource for SlackSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Slack
    }

    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let after_rowid: i64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
        // Use rowid instead of message_id as the cursor: message_id is a Slack timestamp
        // ordered by creation, but rows can be indexed out of order (reconcile, debounce),
        // so a message_id cursor skips late-indexed messages forever.
        let rows = sqlx::query(
            "SELECT d.rowid, d.guild_id, d.thread_id, d.message_id, d.author_id, d.author_name, \
                    d.content, d.timestamp, \
                    COALESCE( \
                      json_extract(d.metadata_json, '$.channel_id'), \
                      CASE WHEN d.thread_id IS NOT NULL AND d.channel_id = d.thread_id THEN ( \
                        SELECT s.channel_id FROM sessions s \
                        WHERE s.platform = 'slack' AND s.thread_id = d.thread_id LIMIT 1 \
                      ) END, \
                      d.channel_id \
                    ) AS real_channel, \
                    CASE \
                      WHEN d.guild_id IS NULL OR d.guild_id = '' THEN 'dm' \
                      WHEN EXISTS ( \
                        SELECT 1 FROM message_search_documents parent \
                        WHERE parent.platform = 'slack' \
                          AND parent.author_id = ? \
                          AND parent.message_id IN ( \
                            COALESCE(d.thread_id, ''), \
                            COALESCE(json_extract(d.metadata_json, '$.referenced_message_id'), '') \
                          ) \
                          AND parent.message_id <> d.message_id \
                      ) THEN 'owner_reply' \
                      ELSE 'channel' \
                    END AS calculated_room_type \
             FROM message_search_documents d \
             WHERE d.platform = 'slack' \
               AND d.rowid > ? \
               AND d.author_id <> '' \
               AND d.author_id <> ? \
               AND d.author_id <> ? \
               AND d.author_id NOT IN (SELECT bot_id FROM bot_profiles) \
               AND ( \
                 (d.guild_id IS NULL OR d.guild_id = '') \
                 OR instr(d.content, ?) > 0 \
                 OR EXISTS ( \
                   SELECT 1 FROM message_search_documents parent \
                   WHERE parent.platform = 'slack' \
                     AND parent.author_id = ? \
                     AND parent.message_id IN ( \
                       COALESCE(d.thread_id, ''), \
                       COALESCE(json_extract(d.metadata_json, '$.referenced_message_id'), '') \
                     ) \
                     AND parent.message_id <> d.message_id \
                 ) \
               ) \
             ORDER BY d.rowid",
        )
        .bind(&self.owner_id)
        .bind(after_rowid)
        .bind(&self.owner_id)
        .bind(&self.bot_id)
        .bind(format!("<@{}>", self.owner_id))
        .bind(&self.owner_id)
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::new();
        let mut next_rowid: i64 = after_rowid;
        for row in rows {
            use sqlx::Row;
            let rid: i64 = row.try_get("rowid")?;
            let ts: String = row.try_get("message_id")?;
            let content: String = row.try_get("content")?;
            let received = match ts_to_datetime(&ts) {
                Some(at) => at,
                None => {
                    let at: String = row.try_get("timestamp")?;
                    DateTime::parse_from_rfc3339(&at)
                        .map_err(|error| {
                            OmonError::ToolExecution(format!(
                                "invalid Slack indexed timestamp: {error}"
                            ))
                        })?
                        .with_timezone(&Utc)
                }
            };
            let thread: Option<String> = row.try_get("thread_id")?;
            let channel: String = row.try_get("real_channel")?;
            let author: String = row.try_get("author_id")?;
            let author_name: String = row.try_get("author_name")?;
            let mut item = InboxItem::new(InboxSourceKind::Slack, ts.clone(), content, received);
            item.link = Some(permalink(&channel, &ts, thread.as_deref()));
            item.thread_ref = thread;
            item.title = Some(if author_name.trim().is_empty() {
                author.clone()
            } else {
                author_name
            });
            item.author = Some(author);
            item.room_type = Some(row.try_get("calculated_room_type")?);
            next_rowid = next_rowid.max(rid);
            items.push(item);
        }
        Ok((items, next_rowid.to_string()))
    }
}

pub fn permalink(channel: &str, ts: &str, thread_ts: Option<&str>) -> String {
    let base = format!(
        "https://slack.com/archives/{channel}/p{}",
        ts.replace('.', "")
    );
    match thread_ts.filter(|thread| *thread != ts) {
        Some(thread) => format!("{base}?thread_ts={thread}&cid={channel}"),
        None => base,
    }
}

fn ts_to_datetime(ts: &str) -> Option<DateTime<Utc>> {
    let (secs, frac) = ts.split_once('.')?;
    let secs = secs.parse::<i64>().ok()?;
    let micros = format!("{frac:0<6}").get(..6)?.parse::<u32>().ok()?;
    DateTime::from_timestamp(secs, micros * 1_000)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::storage::{MessageSearchDocument, MessageSearchIndex};

    const OWNER: &str = "U0OWNER";
    const BOT: &str = "U0BOT";

    async fn pool() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn doc(
        guild: Option<&str>,
        channel: &str,
        thread: Option<&str>,
        ts: &str,
        author: &str,
        content: &str,
    ) -> MessageSearchDocument {
        MessageSearchDocument {
            platform: "slack".into(),
            guild_id: guild.map(str::to_owned),
            channel_id: thread.unwrap_or(channel).to_owned(),
            thread_id: thread.map(str::to_owned),
            message_id: ts.into(),
            author_id: author.into(),
            author_name: String::new(),
            content: content.into(),
            attachment_names: Vec::new(),
            timestamp: Utc::now(),
            metadata: json!({"source": "ingress"}),
        }
    }

    #[tokio::test]
    async fn collects_dm_mention_and_owner_thread_reply_only() {
        let pool = pool().await;
        let index = MessageSearchIndex::new(pool.clone());
        sqlx::query(
            "INSERT INTO sessions(session_key, platform, guild_id, channel_id, thread_id, user_id) \
             VALUES('k1', 'slack', 'T1', 'C1', '1700000100.000100', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        index
            .upsert_many(&[
                doc(
                    None,
                    "D1",
                    None,
                    "1700000001.000100",
                    "U0ALICE",
                    "hi from dm",
                ),
                doc(
                    Some("T1"),
                    "C1",
                    None,
                    "1700000002.000100",
                    "U0ALICE",
                    "ping <@U0OWNER> please",
                ),
                doc(
                    Some("T1"),
                    "C1",
                    None,
                    "1700000003.000100",
                    "U0ALICE",
                    "unrelated chatter",
                ),
                doc(
                    Some("T1"),
                    "C1",
                    None,
                    "1700000100.000100",
                    OWNER,
                    "owner root",
                ),
                doc(
                    Some("T1"),
                    "C1",
                    Some("1700000100.000100"),
                    "1700000101.000100",
                    "U0BOB",
                    "reply to owner",
                ),
                doc(
                    None,
                    "D1",
                    None,
                    "1700000004.000100",
                    OWNER,
                    "owner's own dm",
                ),
                doc(None, "D1", None, "1700000005.000100", BOT, "bot dm"),
            ])
            .await
            .unwrap();
        let source = SlackSource::new(pool.clone(), OWNER.into(), BOT.into());

        let (items, cursor) = source.fetch_since(None).await.unwrap();

        let refs: Vec<_> = items.iter().map(|item| item.source_ref.as_str()).collect();
        assert_eq!(
            refs,
            [
                "1700000001.000100",
                "1700000002.000100",
                "1700000101.000100"
            ]
        );
        assert_eq!(cursor, "5");
        assert!(items
            .iter()
            .all(|item| item.source == InboxSourceKind::Slack));
        assert_eq!(items[0].room_type.as_deref(), Some("dm"));
        assert_eq!(items[1].room_type.as_deref(), Some("channel"));
        assert_eq!(items[2].room_type.as_deref(), Some("owner_reply"));
        assert_eq!(
            items[0].link.as_deref(),
            Some("https://slack.com/archives/D1/p1700000001000100")
        );
        assert_eq!(
            items[2].link.as_deref(),
            Some("https://slack.com/archives/C1/p1700000101000100?thread_ts=1700000100.000100&cid=C1")
        );
        assert_eq!(items[2].thread_ref.as_deref(), Some("1700000100.000100"));
        assert_eq!(items[1].author.as_deref(), Some("U0ALICE"));
        assert_eq!(items[0].received_at.timestamp(), 1_700_000_001);

        let (again, next) = source.fetch_since(Some(&cursor)).await.unwrap();
        assert!(again.is_empty());
        assert_eq!(next, cursor);
    }

    #[tokio::test]
    async fn collects_older_message_indexed_after_cursor_advanced() {
        let pool = pool().await;
        let index = MessageSearchIndex::new(pool.clone());
        let source = SlackSource::new(pool, OWNER.into(), BOT.into());
        let newer = doc(None, "D1", None, "1700000002.000100", "U0ALICE", "newer");
        index.upsert(&newer).await.unwrap();
        let (_, cursor) = source.fetch_since(None).await.unwrap();
        index
            .upsert(&doc(
                None,
                "D1",
                None,
                "1700000001.000100",
                "U0ALICE",
                "late",
            ))
            .await
            .unwrap();
        index.upsert(&newer).await.unwrap();
        let (items, next) = source.fetch_since(Some(&cursor)).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_ref, "1700000001.000100");
        assert!(source.fetch_since(Some(&next)).await.unwrap().0.is_empty());
        // Existing timestamp cursors rescan once instead of skipping the entire index.
        assert_eq!(
            source
                .fetch_since(Some("1700000002.000100"))
                .await
                .unwrap()
                .0
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn collect_all_commits_slack_items_through_prefilter() {
        let pool = pool().await;
        MessageSearchIndex::new(pool.clone())
            .upsert(&doc(
                None,
                "D1",
                None,
                "1700000001.000100",
                "U0ALICE",
                "need your review",
            ))
            .await
            .unwrap();
        let source = SlackSource::new(pool.clone(), OWNER.into(), BOT.into());
        let rules = crate::inbox::prefilter::PrefilterRules {
            owner_slack_id: OWNER.into(),
            ..Default::default()
        };

        let results =
            crate::inbox::collect_all(&pool, &[&source as &dyn InboxSource], &rules).await;

        assert_eq!(results[0].error, None);
        assert_eq!(results[0].stats.inserted, 1);
        let stored: (String, String) = sqlx::query_as("SELECT source, link FROM inbox_items")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored.0, "slack");
        assert_eq!(stored.1, "https://slack.com/archives/D1/p1700000001000100");
    }
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

#[derive(Clone)]
pub struct DiscordSource {
    pool: SqlitePool,
    owner_id: String,
    bot_id: Option<String>,
}

impl DiscordSource {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            owner_id: "414011632306618368".into(),
            bot_id: std::env::var("DISCORD_BOT_ID").ok(),
        }
    }

    pub fn with_ids(pool: SqlitePool, owner_id: impl Into<String>, bot_id: Option<String>) -> Self {
        Self {
            pool,
            owner_id: owner_id.into(),
            bot_id,
        }
    }
}

#[async_trait]
impl InboxSource for DiscordSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Discord
    }
    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let after = cursor.unwrap_or("");
        let rows = sqlx::query(
            "SELECT d.platform, d.guild_id, d.channel_id, d.thread_id, d.message_id, \
                    d.author_id, d.author_name, d.content, d.timestamp, \
                    CASE \
                      WHEN d.guild_id IS NULL OR d.guild_id = '' THEN 'dm' \
                      WHEN EXISTS ( \
                        SELECT 1 FROM message_search_documents parent \
                        WHERE parent.platform = 'discord' \
                          AND parent.channel_id = d.channel_id \
                          AND parent.message_id = json_extract(d.metadata_json, '$.referenced_message_id') \
                          AND parent.author_id = ? \
                      ) THEN 'owner_reply' \
                      ELSE 'channel' \
                    END AS calculated_room_type \
             FROM message_search_documents d \
             WHERE d.platform = 'discord' \
               AND d.message_id > ? \
               AND d.author_id <> ? \
               AND (? IS NULL OR d.author_id <> ?) \
               AND d.author_id NOT IN (SELECT bot_id FROM bot_profiles) \
               AND ( \
                 (d.guild_id IS NULL OR d.guild_id = '') \
                 OR instr(d.content, ?) > 0 \
                 OR instr(d.content, ?) > 0 \
                 OR EXISTS ( \
                   SELECT 1 FROM message_search_documents parent \
                   WHERE parent.platform = 'discord' \
                     AND parent.channel_id = d.channel_id \
                     AND parent.message_id = json_extract(d.metadata_json, '$.referenced_message_id') \
                     AND parent.author_id = ? \
                 ) \
               ) \
             ORDER BY d.timestamp, d.message_id",
        )
        .bind(&self.owner_id)
        .bind(after)
        .bind(&self.owner_id)
        .bind(self.bot_id.as_deref())
        .bind(self.bot_id.as_deref())
        .bind(format!("<@{}>", self.owner_id))
        .bind(format!("<@!{}>", self.owner_id))
        .bind(&self.owner_id)
        .fetch_all(&self.pool)
        .await?;
        let mut items = Vec::new();
        let mut next = after.to_owned();
        for row in rows {
            use sqlx::Row;
            let id: String = row.try_get("message_id")?;
            let content: String = row.try_get("content")?;
            let at: String = row.try_get("timestamp")?;
            let received = DateTime::parse_from_rfc3339(&at)
                .map_err(|error| {
                    OmonError::ToolExecution(format!("invalid Discord indexed timestamp: {error}"))
                })?
                .with_timezone(&Utc);
            let mut item = InboxItem::new(InboxSourceKind::Discord, id.clone(), content, received);
            item.thread_ref = row.try_get("thread_id")?;
            item.author = Some(row.try_get("author_id")?);
            item.title = Some(row.try_get("author_name")?);
            let channel: String = row.try_get("channel_id")?;
            item.link = Some(format!(
                "https://discord.com/channels/{}/{}/{}",
                row.try_get::<Option<String>, _>("guild_id")?
                    .unwrap_or_else(|| "@me".into()),
                channel,
                id
            ));
            let room_type: String = row.try_get("calculated_room_type")?;
            item.room_type = Some(room_type);
            next = next.max(id);
            items.push(item);
        }
        Ok((items, next))
    }
}

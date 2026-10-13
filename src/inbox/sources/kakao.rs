use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{array, optional_string, required_string};
use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

#[derive(Clone, Debug)]
pub struct KakaoSource {
    client: reqwest::Client,
    base_url: String,
    rooms: Option<Vec<String>>,
}

impl Default for KakaoSource {
    fn default() -> Self {
        Self::new(
            std::env::var("OMON_PIRATETALK_URL")
                .unwrap_or_else(|_| "https://indo.taila0cef2.ts.net".into()),
            std::env::var("OMON_INBOX_KAKAO_ROOMS").ok(),
        )
    }
}

impl KakaoSource {
    pub fn new(base_url: impl Into<String>, rooms: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            rooms: rooms.map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|room| !room.is_empty())
                    .map(str::to_owned)
                    .collect()
            }),
        }
    }

    pub fn parse_cursor(cursor: Option<&str>) -> HashMap<String, String> {
        cursor
            .and_then(|raw| serde_json::from_str::<HashMap<String, String>>(raw).ok())
            .unwrap_or_default()
    }

    pub fn serialize_cursor(cursor_map: &HashMap<String, String>) -> String {
        serde_json::to_string(cursor_map).unwrap_or_default()
    }

    pub fn parse_messages(
        room_id: &str,
        room_title: Option<&str>,
        room_type: &str,
        page: &Value,
        floor: &str,
    ) -> Result<(Vec<InboxItem>, Option<String>, Option<String>)> {
        let messages = array(page, "PirateTalk message page")?;
        let mut items = Vec::new();
        let mut page_oldest: Option<String> = None;
        let mut page_newest: Option<String> = None;
        let mut seen = HashSet::new();
        for message in messages {
            let message_id = message
                .get("id")
                .or_else(|| message.get("logId"))
                .and_then(Value::as_str)
                .ok_or_else(|| OmonError::ToolExecution("PirateTalk message missing id".into()))?;
            if !seen.insert(message_id.to_owned()) {
                return Err(OmonError::ToolExecution(
                    "PirateTalk paging repeated a message id".into(),
                ));
            }
            if !floor.is_empty() && message_id <= floor {
                continue;
            }
            page_oldest = Some(page_oldest.map_or_else(
                || message_id.to_owned(),
                |old| old.min(message_id.to_owned()),
            ));
            page_newest = Some(page_newest.map_or_else(
                || message_id.to_owned(),
                |new| new.max(message_id.to_owned()),
            ));
            // Attachment-only messages have no text; skip them (paging bounds were
            // already advanced above) instead of failing the whole room fetch.
            let Some(text) = optional_string(message, &["text", "message", "content", "msg"])
            else {
                continue;
            };
            // kakao-archive.ts defines `time` as Unix seconds, not sendAt or logId.
            let received = if let Some(time) = message.get("time") {
                let seconds = time.as_i64().ok_or_else(|| {
                    OmonError::ToolExecution(
                        "PirateTalk message time must be integral Unix seconds".into(),
                    )
                })?;
                DateTime::<Utc>::from_timestamp(seconds, 0).ok_or_else(|| {
                    OmonError::ToolExecution("PirateTalk message time is out of range".into())
                })?
            } else {
                let timestamp =
                    optional_string(message, &["createdAt", "created_at", "timestamp", "date"])
                        .ok_or_else(|| {
                            OmonError::ToolExecution("PirateTalk message missing timestamp".into())
                        })?;
                parse_date(&timestamp)?
            };
            let mut item = InboxItem::new(InboxSourceKind::Kakao, message_id, text, received);
            item.thread_ref = Some(room_id.to_owned());
            item.room_type = Some(room_type.to_owned());
            item.title = room_title.map(str::to_owned);
            item.author = optional_string(message, &["senderName", "authorName", "userName"]);
            items.push(item);
        }
        Ok((items, page_oldest, page_newest))
    }

    pub async fn fetch_with_client(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<InboxItem>, String)> {
        let response = self
            .client
            .get(format!("{}/api/rooms", self.base_url))
            .send()
            .await
            .map_err(|error| {
                OmonError::ToolExecution(format!("PirateTalk GET /api/rooms failed: {error}"))
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(OmonError::ToolExecution(format!(
                "PirateTalk GET /api/rooms returned {status}"
            )));
        }
        let rooms_value: Value = response.json::<Value>().await.map_err(|error| {
            OmonError::ToolExecution(format!("PirateTalk rooms returned invalid JSON: {error}"))
        })?;
        let rooms = array(&rooms_value, "PirateTalk rooms")?;
        let configured = self
            .rooms
            .as_ref()
            .map(|values| values.iter().cloned().collect::<HashSet<_>>());
        let mut cursor_map = Self::parse_cursor(cursor);
        let mut all_items = Vec::new();
        for room in rooms {
            let id = required_string(room, "chatId", "PirateTalk room")?;
            let unread = room.get("unreadCount").and_then(Value::as_i64).unwrap_or(0);
            let room_type = required_string(room, "type", "PirateTalk room")?;
            let title = optional_string(room, &["title", "name"]);
            if configured.as_ref().is_some_and(|set| !set.contains(id))
                || (configured.is_none()
                    && unread == 0
                    && !matches!(room_type, "DirectChat" | "MultiChat"))
            {
                continue;
            }
            let floor = cursor_map.get(id).cloned().unwrap_or_default();
            let mut before = String::new();
            let mut room_newest = floor.clone();
            loop {
                let url = if before.is_empty() {
                    format!(
                        "{}/api/rooms/{}/messages",
                        self.base_url,
                        urlencoding::encode(id)
                    )
                } else {
                    format!(
                        "{}/api/rooms/{}/messages?before_id={}",
                        self.base_url,
                        urlencoding::encode(id),
                        urlencoding::encode(&before)
                    )
                };
                let response = self.client.get(url).send().await.map_err(|error| {
                    OmonError::ToolExecution(format!(
                        "PirateTalk GET messages for room {id} failed: {error}"
                    ))
                })?;
                if !response.status().is_success() {
                    return Err(OmonError::ToolExecution(format!(
                        "PirateTalk GET messages for room {id} returned {}",
                        response.status()
                    )));
                }
                let page: Value = response.json::<Value>().await.map_err(|error| {
                    OmonError::ToolExecution(format!(
                        "PirateTalk message page returned invalid JSON: {error}"
                    ))
                })?;
                let (page_items, page_oldest, page_top) =
                    Self::parse_messages(id, title.as_deref(), room_type, &page, &floor)?;
                if let Some(top) = page_top {
                    room_newest = room_newest.max(top);
                }
                all_items.extend(page_items);
                let Some(oldest) = page_oldest else { break };
                if oldest >= before && !before.is_empty() {
                    return Err(OmonError::ToolExecution(
                        "PirateTalk paging cursor did not move backwards".into(),
                    ));
                }
                before = oldest;
            }
            if !room_newest.is_empty() {
                cursor_map.insert(id.to_owned(), room_newest);
            }
        }
        Ok((all_items, Self::serialize_cursor(&cursor_map)))
    }
}

fn parse_date(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| {
            OmonError::ToolExecution(format!("invalid PirateTalk message timestamp: {error}"))
        })
}

#[async_trait]
impl InboxSource for KakaoSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Kakao
    }
    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        self.fetch_with_client(cursor).await
    }
}

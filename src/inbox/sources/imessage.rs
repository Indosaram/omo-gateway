use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{optional_string, run_json};
use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

#[derive(Clone, Debug, Default)]
pub struct IMessageSource;

impl IMessageSource {
    pub fn chats_args() -> Vec<String> {
        vec![
            "chats".into(),
            "--limit".into(),
            "50".into(),
            "--json".into(),
        ]
    }
    pub fn history_args(chat_id: &str) -> Vec<String> {
        vec![
            "history".into(),
            "--chat-id".into(),
            chat_id.into(),
            "--limit".into(),
            "50".into(),
            "--json".into(),
        ]
    }

    pub fn parse(
        chats: &Value,
        histories: &[(String, Value)],
        cursor: Option<&str>,
    ) -> Result<(Vec<InboxItem>, String)> {
        let rows = objects(chats, "imsg chats")?;
        let since = cursor.and_then(|v| v.parse::<DateTime<Utc>>().ok());
        let mut newest = since;
        let mut items = Vec::new();
        for chat in rows {
            let chat_id = identifier(
                chat.get("id")
                    .ok_or_else(|| OmonError::ToolExecution("imsg chat missing id".into()))?,
                "imsg chat id",
            )?;
            let last = chat
                .get("last_message_at")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    OmonError::ToolExecution("imsg chat missing last_message_at".into())
                })?;
            let chat_time = DateTime::parse_from_rfc3339(last)
                .map_err(|error| {
                    OmonError::ToolExecution(format!("invalid imsg last_message_at: {error}"))
                })?
                .with_timezone(&Utc);
            if since.is_some_and(|cursor| chat_time <= cursor) {
                continue;
            }
            let history = histories
                .iter()
                .find(|(id, _)| id == &chat_id)
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    OmonError::ToolExecution(format!("imsg history missing for chat {chat_id}"))
                })?;
            let messages = objects(history, "imsg history")?;
            for message in messages {
                if message
                    .get("is_from_me")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }
                let timestamp = message
                    .get("created_at")
                    .or_else(|| message.get("date"))
                    .or_else(|| message.get("timestamp"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| OmonError::ToolExecution("imsg message missing date".into()))?;
                let received = DateTime::parse_from_rfc3339(timestamp)
                    .map_err(|error| {
                        OmonError::ToolExecution(format!("invalid imsg message date: {error}"))
                    })?
                    .with_timezone(&Utc);
                if since.is_some_and(|cursor| received <= cursor) {
                    continue;
                }
                let id = message
                    .get("id")
                    .or_else(|| message.get("rowid"))
                    .ok_or_else(|| {
                        OmonError::ToolExecution("imsg message missing id/rowid".into())
                    })?;
                let id = identifier(id, "imsg message id")?;
                newest = Some(newest.map_or(received, |old| old.max(received)));
                // Attachment-only messages carry no text; skip them (cursor still
                // advances) instead of wedging the whole source on every tick.
                let Some(text) = optional_string(message, &["text", "text_content", "body"]) else {
                    continue;
                };
                let mut item = InboxItem::new(InboxSourceKind::Imessage, id, text, received);
                item.thread_ref = Some(chat_id.to_owned());
                item.author =
                    optional_string(message, &["sender", "handle", "from"]).or_else(|| {
                        chat.get("identifier")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    });
                item.title = optional_string(chat, &["name", "display_name"]);
                items.push(item);
            }
        }
        let next = newest
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| cursor.unwrap_or("").to_owned());
        Ok((items, next))
    }
}

#[async_trait]
impl InboxSource for IMessageSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Imessage
    }
    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let chats = run_json("imsg", Self::chats_args()).await?;
        let mut histories = Vec::new();
        for chat in objects(&chats, "imsg chats")? {
            let id = identifier(
                chat.get("id")
                    .ok_or_else(|| OmonError::ToolExecution("imsg chat missing id".into()))?,
                "imsg chat id",
            )?;
            let at = chat
                .get("last_message_at")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    OmonError::ToolExecution("imsg chat missing last_message_at".into())
                })?;
            let after_cursor = cursor
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_none_or(|value| {
                    DateTime::parse_from_rfc3339(at)
                        .map(|at| at.with_timezone(&Utc) > value)
                        .unwrap_or(false)
                });
            if after_cursor {
                histories.push((
                    id.to_owned(),
                    run_json("imsg", Self::history_args(&id)).await?,
                ));
            }
        }
        Self::parse(&chats, &histories, cursor)
    }
}

fn objects<'a>(value: &'a Value, context: &str) -> Result<Vec<&'a Value>> {
    match value {
        Value::Array(values) => Ok(values.iter().collect()),
        Value::Object(_) => Ok(vec![value]),
        Value::Null => Ok(Vec::new()),
        _ => Err(OmonError::ToolExecution(format!(
            "{context} response must contain JSON objects"
        ))),
    }
}

fn identifier(value: &Value, field: &str) -> Result<String> {
    match value {
        Value::String(text) if !text.is_empty() => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        _ => Err(OmonError::ToolExecution(format!(
            "{field} must be a non-empty string or number"
        ))),
    }
}

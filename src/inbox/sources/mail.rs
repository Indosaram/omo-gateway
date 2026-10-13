use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{array, optional_string, run_json};
use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

const ACCOUNT: &str = "freedomzero91@gmail.com";
const FILTER: &str = "is:unread -category:promotions -category:social -category:updates";

#[derive(Clone, Debug, Default)]
pub struct MailSource;

impl MailSource {
    fn args(cursor: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "mail".into(),
            "list".into(),
            "--folder".into(),
            "inbox".into(),
            "--account".into(),
            ACCOUNT.into(),
            "--filter".into(),
            FILTER.into(),
            "--limit".into(),
            "100".into(),
        ];
        if let Some(timestamp) = cursor.and_then(|v| v.parse::<i64>().ok()) {
            args[7] = format!("{FILTER} after:{timestamp}");
        }
        args
    }

    pub fn command_args(cursor: Option<&str>) -> Vec<String> {
        Self::args(cursor)
    }

    pub fn parse(value: &Value, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let root_items = value.get("items").ok_or_else(|| {
            OmonError::ToolExecution("zele mail list response missing items".into())
        })?;
        let rows = array(root_items, "zele mail list items")?;
        let mut items = Vec::new();
        let mut newest_ts: Option<i64> = cursor.and_then(|v| v.parse::<i64>().ok());
        for row in rows {
            let id = row
                .get("thread_id")
                .and_then(Value::as_str)
                .or_else(|| row.get("id").and_then(Value::as_str))
                .ok_or_else(|| OmonError::ToolExecution("mail thread missing thread_id".into()))?;
            let thread = id;
            // A thread without a snippet is dropped by commit_source's empty-snippet
            // rule instead of failing the whole collection batch.
            let snippet =
                optional_string(row, &["snippet", "summary", "preview"]).unwrap_or_default();
            let received_at = date(row)?;
            let item_secs = received_at.timestamp();
            newest_ts = Some(newest_ts.map_or(item_secs, |cur| cur.max(item_secs)));
            let mut item = InboxItem::new(InboxSourceKind::Mail, thread, snippet, received_at);
            item.thread_ref = Some(thread.to_owned());
            item.author = optional_string(row, &["from", "sender"]);
            item.title = optional_string(row, &["subject", "title"]);
            item.mail_categories = row
                .get("categories")
                .or_else(|| row.get("labels"))
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_lowercase)
                        .collect()
                })
                .unwrap_or_default();
            item.link = Some(format!("https://mail.google.com/mail/u/0/#all/{thread}"));
            items.push(item);
        }
        let next_cursor = newest_ts
            .map(|ts| ts.to_string())
            .unwrap_or_else(|| cursor.unwrap_or("").to_owned());
        Ok((items, next_cursor))
    }
}

fn date(row: &Value) -> Result<DateTime<Utc>> {
    if let Some(value) = row
        .get("internal_date")
        .or_else(|| row.get("date"))
        .and_then(Value::as_str)
    {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
            return Ok(parsed.with_timezone(&Utc));
        }
        if let Ok(millis) = value.parse::<i64>() {
            let millis = if millis > 10_000_000_000 {
                millis
            } else {
                millis.saturating_mul(1000)
            };
            if let Some(dt) = DateTime::from_timestamp_millis(millis) {
                return Ok(dt);
            }
        }
    }
    if let Some(millis) = row
        .get("internal_date")
        .or_else(|| row.get("date"))
        .and_then(Value::as_i64)
    {
        let millis = if millis > 10_000_000_000 {
            millis
        } else {
            millis.saturating_mul(1000)
        };
        if let Some(dt) = DateTime::from_timestamp_millis(millis) {
            return Ok(dt);
        }
    }
    Err(OmonError::ToolExecution(
        "mail thread has invalid date".into(),
    ))
}

#[async_trait]
impl InboxSource for MailSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Mail
    }
    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let value = run_json("zele", Self::args(cursor)).await?;
        Self::parse(&value, cursor)
    }
}

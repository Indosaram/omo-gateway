use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
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
        Self::parse_at(value, cursor, Utc::now())
    }

    pub fn parse_at(
        value: &Value,
        cursor: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<(Vec<InboxItem>, String)> {
        let root_items = value.get("items").ok_or_else(|| {
            OmonError::ToolExecution("zele mail list response missing items".into())
        })?;
        let rows = array(root_items, "zele mail list items")?;
        let mut items = Vec::new();
        let mut newest_ts: Option<i64> = cursor.and_then(|v| v.parse::<i64>().ok());
        // A row whose date cannot be read is a hard error, never a guess: committing it at a made-up
        // timestamp would either drop real mail (cursor too late) or silently file fresh mail as
        // stale backlog, and `(source, source_ref)` dedupe would keep that wrong verdict forever.
        let mut unreadable: Vec<(String, String)> = Vec::new();
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
            let received_at = match date(row, now) {
                Some(at) => at,
                None => {
                    let raw = row
                        .get("internal_date")
                        .or_else(|| row.get("date"))
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "<missing>".to_owned());
                    unreadable.push((thread.to_owned(), raw));
                    continue;
                }
            };
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
        if !unreadable.is_empty() {
            // Fail the whole batch: `collect_all` records this as the cursor error and commits
            // nothing, so the cursor stays where it was and the same rows are retried instead of
            // being written with an invented timestamp.
            let shown: Vec<String> = unreadable
                .iter()
                .take(3)
                .map(|(thread, raw)| format!("thread {thread}: {raw}"))
                .collect();
            return Err(OmonError::ToolExecution(format!(
                "zele mail list returned an unreadable date for {} thread(s) [{}]; expected \
                 \"just now\", \"<n>m ago\", \"<n>h ago\", \"<n>d ago\", \"Mon D\" or \
                 \"Mon D, YYYY\" - nothing was committed and the cursor did not advance",
                unreadable.len(),
                shown.join(", ")
            )));
        }
        let next_cursor = newest_ts
            .map(|ts| ts.to_string())
            .unwrap_or_else(|| cursor.unwrap_or("").to_owned());
        Ok((items, next_cursor))
    }
}

/// Reads a thread's timestamp from the fields zele emits. `None` means the value is in no form
/// this gateway understands, and the caller must refuse the batch rather than substitute one:
/// an invented timestamp is either earlier than the mail (skipped forever) or later than it
/// (filed as stale backlog under a permanent dedupe key).
fn date(row: &Value, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if let Some(value) = row
        .get("internal_date")
        .or_else(|| row.get("date"))
        .and_then(Value::as_str)
    {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
            return Some(parsed.with_timezone(&Utc));
        }
        if let Some(parsed) = display_date(value, now) {
            return Some(parsed);
        }
        if let Ok(millis) = value.parse::<i64>() {
            let millis = if millis > 10_000_000_000 {
                millis
            } else {
                millis.saturating_mul(1000)
            };
            if let Some(dt) = DateTime::from_timestamp_millis(millis) {
                return Some(dt);
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
            return Some(dt);
        }
    }
    None
}

/// zele 0.8 `mail list` prints only its display date ("just now", "14m ago", "3h ago", "2d ago",
/// "Oct 3", "Oct 3, 2025"), formatted in the locale of the process that ran it. Each form names a
/// range, so return its lower bound: the cursor is sent back as Gmail `after:`, and a too-late
/// timestamp would skip mail. Re-fetched threads are deduplicated by (source, source_ref).
/// Anything else (a localized month name, a future zele format) returns `None` on purpose.
fn display_date(value: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("just now") {
        return Some(now - Duration::minutes(1));
    }
    if let Some(amount) = value.strip_suffix(" ago") {
        let unit = amount.chars().last()?;
        let n: i64 = amount[..amount.len() - unit.len_utf8()].parse().ok()?;
        if n < 0 {
            return None;
        }
        let count = n.checked_add(1)?;
        let span = match unit {
            'm' => Duration::try_minutes(count)?,
            'h' => Duration::try_hours(count)?,
            'd' => Duration::try_days(count)?,
            _ => return None,
        };
        return now.checked_sub_signed(span);
    }
    let today = now.with_timezone(&Local).date_naive();
    // `toLocaleDateString` pads the day differently per platform, so accept both spellings.
    let parse_day = |text: &str| {
        ["%b %e, %Y", "%b %d, %Y"]
            .iter()
            .find_map(|format| NaiveDate::parse_from_str(text, format).ok())
    };
    let day = match parse_day(value) {
        Some(day) => day,
        None => {
            let this_year = parse_day(&format!("{value}, {}", today.year()))?;
            if this_year > today {
                this_year.with_year(today.year() - 1)?
            } else {
                this_year
            }
        }
    };
    let start = Local
        .from_local_datetime(&day.and_hms_opt(0, 0, 0)?)
        .earliest()?;
    Some(start.with_timezone(&Utc))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zele_display_dates_parse_to_their_lower_bound() {
        let now = DateTime::parse_from_rfc3339("2026-10-20T03:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let payload = serde_json::json!({"items": [
            {"id": "a", "snippet": "s", "date": "14m ago"},
            {"id": "b", "snippet": "s", "date": "1h ago"},
            {"id": "c", "snippet": "s", "date": "3d ago"},
            {"id": "d", "snippet": "s", "date": "just now"},
            {"id": "e", "snippet": "s", "date": "Oct 3"},
            {"id": "f", "snippet": "s", "date": "Dec 30, 2025"},
        ]});
        let (items, cursor) = MailSource::parse_at(&payload, None, now).unwrap();
        let at = |id: &str| {
            items
                .iter()
                .find(|item| item.source_ref == id)
                .unwrap()
                .received_at
        };
        assert_eq!(at("a"), now - Duration::minutes(15));
        assert_eq!(at("b"), now - Duration::hours(2));
        assert_eq!(at("c"), now - Duration::days(4));
        assert_eq!(at("d"), now - Duration::minutes(1));
        assert_eq!(
            at("e").with_timezone(&Local).date_naive(),
            NaiveDate::from_ymd_opt(2026, 10, 3).unwrap()
        );
        assert_eq!(
            at("f").with_timezone(&Local).date_naive(),
            NaiveDate::from_ymd_opt(2025, 12, 30).unwrap()
        );
        // The newest row is a lower bound, so the next `after:` never runs past real mail.
        assert_eq!(cursor, (now - Duration::minutes(1)).timestamp().to_string());
        assert!(display_date("sometime", now).is_none());
        assert!(display_date("9223372036854775807d ago", now).is_none());
        assert!(display_date("-1m ago", now).is_none());
    }

    #[test]
    fn an_unreadable_date_fails_the_batch_instead_of_guessing() {
        let now = DateTime::parse_from_rfc3339("2026-10-20T03:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cursor = (now - Duration::hours(6)).timestamp().to_string();
        // A localized month name is what `toLocaleDateString` prints off en-US. Guessing a
        // timestamp for it would file fresh mail as stale backlog under a permanent
        // (source, source_ref) dedupe, so the batch must fail with the raw value instead.
        let payload = serde_json::json!({"items": [
            {"id": "a", "snippet": "s", "date": "10월 3일"},
            {"id": "b", "snippet": "s", "date": "14m ago"},
        ]});
        let error = MailSource::parse_at(&payload, Some(cursor.as_str()), now)
            .unwrap_err()
            .to_string();
        assert!(error.contains("thread a"), "{error}");
        assert!(error.contains("10월 3일"), "{error}");
        assert!(error.contains("1 thread(s)"), "{error}");
        assert!(!crate::inbox::health::is_auth_failure(&error), "{error}");

        // A missing date is the same failure, not an epoch timestamp.
        let payload = serde_json::json!({"items": [{"id": "c", "snippet": "s"}]});
        let error = MailSource::parse_at(&payload, None, now)
            .unwrap_err()
            .to_string();
        assert!(error.contains("<missing>"), "{error}");
    }
}

//! Owner alerts that do not depend on an agent turn.
//!
//! Incident 2026-10-21: a purchase-quote reply was triaged `owner`, handed to the lead as an
//! action-loop event, and the lead turn died at its 45s first-response gate after recording
//! "surfaced to owner". Nothing reached the owner. A triage verdict that needs the owner is now
//! announced by the gateway itself, as one plain line in the owner's home DM, before any agent
//! turn is involved. The agent turn may still follow with a summary; the line is the guarantee.

use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::{DeliveryLedgerService, OutboundAction, OutboundDispatcher, Result, SessionKey};

/// Obligation id prefix; one row per inbox item, so a resend never duplicates the alert.
pub const OBLIGATION_PREFIX: &str = "obl:owner-alert:";

/// Items older than this when first seen are backlog (an outage, a first deploy) and are not
/// announced one by one.
pub const MAX_ALERT_AGE_HOURS: i64 = 24;

/// Upper bound per sweep so a burst cannot flood the owner's DM.
pub const MAX_ALERTS_PER_SWEEP: i64 = 10;

/// Sources whose owner items are announced. Cron problems already reach the owner through the
/// cron report path, so alerting them here would double every escalation. KakaoTalk is left out:
/// the owner reads it on his phone, and its ~50 owner items a day would bury the DM.
const ALERT_SOURCES: &str = "'mail','imessage','slack','discord','github'";

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PendingAlert {
    pub item_id: String,
    pub source: String,
    pub author: Option<String>,
    pub title: Option<String>,
    pub snippet: String,
    pub link: Option<String>,
}

/// Owner items that have no alert obligation yet, or whose last attempt failed.
pub async fn pending_alerts(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Vec<PendingAlert>> {
    let since = now - Duration::hours(MAX_ALERT_AGE_HOURS);
    let query = format!(
        "SELECT i.id AS item_id, i.source, i.author, i.title, i.snippet, i.link
           FROM inbox_decisions d
           JOIN inbox_items i ON i.id = d.item_id
          WHERE d.class = 'owner'
            AND d.status = 'open'
            AND i.source IN ({ALERT_SOURCES})
            AND julianday(d.decided_at) >= julianday(?)
            AND NOT EXISTS (
                SELECT 1 FROM delivery_obligations o
                 WHERE o.id = '{OBLIGATION_PREFIX}' || i.id
                   AND o.state IN ('pending', 'attempting', 'delivered')
            )
          ORDER BY d.decided_at ASC
          LIMIT {MAX_ALERTS_PER_SWEEP}"
    );
    Ok(sqlx::query_as::<_, PendingAlert>(&query)
        .bind(since)
        .fetch_all(pool)
        .await?)
}

/// One line: source, who, what, and where to open it. Text is the item's own metadata only.
pub fn alert_line(alert: &PendingAlert) -> String {
    let label = match alert.source.as_str() {
        "mail" => "메일",
        "kakao" => "카톡",
        "imessage" => "문자",
        "slack" => "슬랙",
        "discord" => "디스코드",
        "github" => "GitHub",
        other => other,
    };
    let who = one_line(alert.author.as_deref().unwrap_or("알 수 없음"), 60);
    let what = alert
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(|title| one_line(title, 90))
        .unwrap_or_else(|| one_line(&alert.snippet, 90));
    let mut line = format!("📬 [{label}] {who} — {what}");
    if let Some(link) = alert.link.as_deref().filter(|link| !link.trim().is_empty()) {
        line.push_str(&format!("\n{link}"));
    }
    line
}

fn one_line(text: &str, max_chars: usize) -> String {
    let flat: String = text
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        return flat;
    }
    let mut cut: String = flat.chars().take(max_chars).collect();
    cut.push('…');
    cut
}

/// Sends every pending alert to `owner` as one message per sweep and records each item as a
/// delivery obligation, so a crash mid-send is recovered by the boot sweep and a delivered alert
/// is never sent twice. One message per sweep keeps a burst of chat items from flooding the DM.
pub async fn sweep(
    pool: &SqlitePool,
    dispatcher: &dyn OutboundDispatcher,
    owner: &SessionKey,
    now: DateTime<Utc>,
) -> Result<usize> {
    let alerts = pending_alerts(pool, now).await?;
    if alerts.is_empty() {
        return Ok(0);
    }
    let ledger = DeliveryLedgerService::new(pool.clone());
    let mut ids = Vec::with_capacity(alerts.len());
    let mut lines = Vec::with_capacity(alerts.len());
    for alert in &alerts {
        let id = format!("{OBLIGATION_PREFIX}{}", alert.item_id);
        let line = alert_line(alert);
        ledger.record_obligation(&id, owner, &line).await?;
        ledger.mark_obligation_attempting(&id).await?;
        ids.push(id);
        lines.push(line);
    }
    let content = if lines.len() == 1 {
        lines.remove(0)
    } else {
        format!(
            "확인이 필요한 새 연락 {}건\n{}",
            lines.len(),
            lines.join("\n")
        )
    };
    match dispatcher
        .dispatch(OutboundAction::SendMessage {
            session: owner.clone(),
            content,
            reply_to: None,
        })
        .await
    {
        Ok(()) => {
            for id in &ids {
                ledger.mark_obligation_delivered(id).await?;
            }
            tracing::info!(sent = ids.len(), "owner alerts delivered");
            Ok(ids.len())
        }
        Err(error) => {
            tracing::warn!(items = ids.len(), %error, "owner alert dispatch failed");
            for id in &ids {
                ledger
                    .mark_obligation_failed(id, &error.to_string())
                    .await?;
            }
            Ok(0)
        }
    }
}

/// Whether an owner alert for this inbox item reached the owner.
pub async fn delivered(pool: &SqlitePool, item_id: &str) -> Result<bool> {
    let state: Option<String> =
        sqlx::query_scalar("SELECT state FROM delivery_obligations WHERE id = ?")
            .bind(format!("{OBLIGATION_PREFIX}{item_id}"))
            .fetch_optional(pool)
            .await?;
    Ok(state.as_deref() == Some("delivered"))
}

/// Enabled unless `OMON_OWNER_ALERTS_ENABLED` is falsy.
pub fn enabled() -> bool {
    std::env::var("OMON_OWNER_ALERTS_ENABLED")
        .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OmonError, OutboundAction};
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Capture {
        sent: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait]
    impl OutboundDispatcher for Capture {
        async fn dispatch(&self, action: OutboundAction) -> Result<()> {
            if self.fail {
                return Err(OmonError::Config("slack down".into()));
            }
            if let OutboundAction::SendMessage { content, .. } = action {
                self.sent.lock().unwrap().push(content);
            }
            Ok(())
        }
    }

    async fn pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn item(pool: &SqlitePool, id: &str, source: &str, class: &str, decided: DateTime<Utc>) {
        sqlx::query(
            "INSERT INTO inbox_items (id, source, source_ref, author, title, snippet, received_at, link, raw_hash, collected_at)
             VALUES (?, ?, ?, 'REPROZ <r@reproz.co.jp>', 'Re: 買取いちばん', 'free pickup', ?, 'https://mail.google.com/x', 'h', ?)",
        )
        .bind(id).bind(source).bind(id).bind(decided).bind(decided)
        .execute(pool).await.unwrap();
        sqlx::query(
            "INSERT INTO inbox_decisions (item_id, class, reason, status, decided_at) VALUES (?, ?, 'r', 'open', ?)",
        )
        .bind(id).bind(class).bind(decided)
        .execute(pool).await.unwrap();
    }

    fn owner() -> SessionKey {
        SessionKey::new(
            "slack",
            None::<String>,
            "D0OWNER",
            None::<String>,
            "U0OWNER",
        )
    }

    #[tokio::test]
    async fn an_owner_item_is_announced_once_without_an_agent_turn() {
        let pool = pool().await;
        let now = Utc::now();
        item(&pool, "mail-1", "mail", "owner", now).await;
        item(&pool, "fyi-1", "mail", "fyi", now).await;
        item(&pool, "cron-1", "cron", "owner", now).await;
        let dispatcher = Capture::default();

        assert_eq!(sweep(&pool, &dispatcher, &owner(), now).await.unwrap(), 1);
        assert_eq!(sweep(&pool, &dispatcher, &owner(), now).await.unwrap(), 0);

        let sent = dispatcher.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].starts_with("📬 [메일] REPROZ"), "{}", sent[0]);
        assert!(sent[0].contains("買取いちばん"));
        assert!(delivered(&pool, "mail-1").await.unwrap());
    }

    #[tokio::test]
    async fn a_failed_send_is_retried_on_the_next_sweep() {
        let pool = pool().await;
        let now = Utc::now();
        item(&pool, "mail-1", "mail", "owner", now).await;

        let down = Capture {
            fail: true,
            ..Capture::default()
        };
        assert_eq!(sweep(&pool, &down, &owner(), now).await.unwrap(), 0);
        assert!(!delivered(&pool, "mail-1").await.unwrap());

        let up = Capture::default();
        assert_eq!(sweep(&pool, &up, &owner(), now).await.unwrap(), 1);
        assert!(delivered(&pool, "mail-1").await.unwrap());
    }

    #[tokio::test]
    async fn a_burst_is_one_message() {
        let pool = pool().await;
        let now = Utc::now();
        item(&pool, "m-1", "mail", "owner", now).await;
        item(&pool, "m-2", "imessage", "owner", now).await;
        item(&pool, "k-1", "kakao", "owner", now).await;
        let dispatcher = Capture::default();
        assert_eq!(sweep(&pool, &dispatcher, &owner(), now).await.unwrap(), 2);
        let sent = dispatcher.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert!(
            sent[0].starts_with("확인이 필요한 새 연락 2건"),
            "{}",
            sent[0]
        );
    }

    #[tokio::test]
    async fn backlog_older_than_a_day_is_not_announced() {
        let pool = pool().await;
        let now = Utc::now();
        item(
            &pool,
            "old",
            "mail",
            "owner",
            now - Duration::hours(MAX_ALERT_AGE_HOURS + 1),
        )
        .await;
        let dispatcher = Capture::default();
        assert_eq!(sweep(&pool, &dispatcher, &owner(), now).await.unwrap(), 0);
    }

    #[test]
    fn the_line_is_flat_and_bounded() {
        let alert = PendingAlert {
            item_id: "x".into(),
            source: "imessage".into(),
            author: Some("여신님".into()),
            title: None,
            snippet: format!("첫 줄\n둘째 줄 {}", "가".repeat(200)),
            link: None,
        };
        let line = alert_line(&alert);
        assert!(line.starts_with("📬 [문자] 여신님 — 첫 줄 둘째 줄"));
        assert!(!line.contains('\n'));
        assert!(line.ends_with('…'));
    }
}

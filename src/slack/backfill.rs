//! Missed-message backfill: replays Slack messages that arrived while the gateway was down.
//!
//! Targets are free-response channels, the owner DM, every stored cursor and every known thread
//! session of this bot. Each target is read from its cursor (`conversations.history` for
//! channels, `conversations.replies` for threads), mapped with the live ingress rules and routed
//! with `route_awaiting_turn`. The cursor advances only past a message whose turn succeeded (or
//! that was skipped on purpose), so a failed turn is retried on the next start. A target with no
//! cursor yet is seeded at its newest message instead of replaying its whole history.

use std::collections::{BTreeSet, HashMap};

use serde_json::{json, Value};

use super::ingress::{read_cursor, ts_key, write_cursor, SlackIngress};
use super::PLATFORM;
use crate::{Result, SessionKey};

const PAGE_LIMIT: u32 = 200;

pub async fn run(ingress: &SlackIngress) -> Result<usize> {
    let pool = ingress.pool();
    let bot = ingress.bot_user_id().to_owned();
    let config = ingress.config();

    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT channel_id, thread_ts, last_ts FROM slack_cursors WHERE bot_id = ?")
            .bind(&bot)
            .fetch_all(pool)
            .await?;
    let mut snapshot: HashMap<(String, String), String> = HashMap::new();
    let mut targets: BTreeSet<(String, String)> = BTreeSet::new();
    for (channel, thread_ts, last_ts) in rows {
        targets.insert((channel.clone(), thread_ts.clone()));
        snapshot.insert((channel, thread_ts), last_ts);
    }
    for channel in &config.free_response_channels {
        targets.insert((channel.clone(), String::new()));
    }
    match ingress.api().open_im(config.owner_id()).await {
        Ok(channel) => {
            targets.insert((channel, String::new()));
        }
        Err(error) => tracing::warn!(%error, "slack backfill could not open the owner DM"),
    }
    let sessions: Vec<String> = sqlx::query_scalar(
        "SELECT session_key FROM sessions WHERE platform = ? AND thread_id IS NOT NULL",
    )
    .bind(PLATFORM)
    .fetch_all(pool)
    .await?;
    for key in sessions
        .iter()
        .filter_map(|raw| SessionKey::from_storage_key(raw).ok())
    {
        if key.bot_id.as_deref() == Some(bot.as_str()) {
            if let Some(thread) = key.thread_id {
                targets.insert((key.channel_id, thread));
            }
        }
    }
    targets.retain(|(channel, _)| !config.ignored_channels.contains(channel));

    let mut routed = 0;
    for (channel, thread_ts) in targets {
        let cursor = match snapshot.get(&(channel.clone(), thread_ts.clone())) {
            Some(cursor) => Some(cursor.clone()),
            None => read_cursor(pool, &bot, &channel, &thread_ts).await?,
        }
        .filter(|cursor| !cursor.trim().is_empty());
        match backfill_target(ingress, &channel, &thread_ts, cursor).await {
            Ok(count) => routed += count,
            Err(error) => {
                tracing::warn!(%error, channel, thread_ts, "slack backfill target failed");
            }
        }
    }
    Ok(routed)
}

async fn backfill_target(
    ingress: &SlackIngress,
    channel: &str,
    thread_ts: &str,
    cursor: Option<String>,
) -> Result<usize> {
    let pool = ingress.pool();
    let bot = ingress.bot_user_id();
    let api = ingress.api();
    let mut messages = if thread_ts.is_empty() {
        api.conversations_history(channel, cursor.as_deref(), None, PAGE_LIMIT)
            .await?
    } else {
        api.conversations_replies(channel, thread_ts, cursor.as_deref(), PAGE_LIMIT)
            .await?
    };
    messages.retain(|message| {
        message.get("ts").and_then(Value::as_str).is_some_and(|ts| {
            ts != thread_ts
                && cursor
                    .as_deref()
                    .is_none_or(|cursor| ts_key(ts) > ts_key(cursor))
        })
    });
    messages.sort_by_key(|message| {
        ts_key(
            message
                .get("ts")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
    });

    if cursor.is_none() {
        if let Some(newest) = messages
            .last()
            .and_then(|message| message.get("ts"))
            .and_then(Value::as_str)
        {
            write_cursor(pool, bot, channel, thread_ts, newest).await?;
        }
        return Ok(0);
    }

    let mut routed = 0;
    for message in messages {
        let ts = message["ts"].as_str().unwrap_or_default().to_owned();
        let event = replay_event(message, channel, thread_ts, bot);
        let known_thread = ingress.is_known_thread(&event).await?;
        if let Some(inbound) = ingress.map_replayed(&event, known_thread) {
            match ingress.route_awaiting_turn(inbound).await {
                Ok(true) => routed += 1,
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%error, channel, thread_ts, ts, "slack backfill turn failed; cursor held");
                    break;
                }
            }
        }
        write_cursor(pool, bot, channel, thread_ts, &ts).await?;
    }
    Ok(routed)
}

/// History messages carry no channel; a channel @mention is replayed as `app_mention` because
/// live ingress only answers mentions through that event.
fn replay_event(mut message: Value, channel: &str, thread_ts: &str, bot: &str) -> Value {
    let dm = channel.starts_with('D');
    let mentioned = message
        .get("text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains(&format!("<@{bot}>")));
    let kind = if mentioned && !dm {
        "app_mention"
    } else {
        "message"
    };
    if let Some(map) = message.as_object_mut() {
        map.insert("type".into(), json!(kind));
        map.insert("channel".into(), json!(channel));
        if !thread_ts.is_empty() {
            map.entry("thread_ts").or_insert_with(|| json!(thread_ts));
        }
        map.insert(
            "channel_type".into(),
            json!(if dm { "im" } else { "channel" }),
        );
    }
    message
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::response::Response;
    use sqlx::SqlitePool;

    use super::*;
    use crate::slack::SlackApi;
    use crate::{Database, InboundEvent, MultiplexerConfig, OmonError, SessionMultiplexer};

    const BOT: &str = "UBOT";
    const OWNER: &str = "UOWNER";

    struct Runner(tokio::sync::mpsc::UnboundedSender<String>);

    #[async_trait]
    impl crate::AgentRunner for Runner {
        async fn run(&self, _: &mut crate::SessionContext, event: InboundEvent) -> Result<()> {
            let _ = self.0.send(event.content.clone());
            if event.content == "fail" {
                return Err(OmonError::Multiplexer("boom".into()));
            }
            Ok(())
        }
    }

    fn msg(ts: &str, text: &str) -> Value {
        json!({ "type": "message", "user": OWNER, "text": text, "ts": ts })
    }

    async fn fake() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new().fallback(|req: Request| async move {
            let method = req.uri().path().trim_start_matches("/api/").to_owned();
            let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                .await
                .unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let reply = match (method.as_str(), body["channel"].as_str()) {
                ("conversations.open", _) => json!({ "ok": true, "channel": { "id": "D1" } }),
                ("conversations.history", Some("CFREE")) => json!({ "ok": true, "messages": [
                    msg("103.000000", "fail"), msg("102.000000", "/stop"),
                    msg("101.000000", "hello"), msg("100.000000", "already seen"),
                ]}),
                ("conversations.history", Some("D1")) => json!({ "ok": true, "messages": [
                    msg("50.000000", "old dm"), msg("40.000000", "older dm"),
                ]}),
                ("conversations.replies", Some("C1")) => json!({ "ok": true, "messages": [
                    msg("1700.000000", "root"), msg("1700.500000", "seen reply"),
                    msg("1701.000000", "thread reply"),
                ]}),
                _ => json!({ "ok": true, "messages": [] }),
            };
            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::from(reply.to_string()))
                .unwrap()
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        base
    }

    async fn cursor(pool: &SqlitePool, channel: &str, thread: &str) -> Option<String> {
        read_cursor(pool, BOT, channel, thread).await.unwrap()
    }

    #[tokio::test]
    async fn replays_since_cursor_holds_on_failure_and_seeds_new_targets() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(Runner(tx)),
            MultiplexerConfig::default(),
        );
        let base = fake().await;
        let config = crate::slack::SlackConfig {
            allowed_users: vec![OWNER.into()],
            free_response_channels: vec!["CFREE".into()],
            ..crate::slack::SlackConfig::test_default()
        };
        let ingress = SlackIngress::new(
            Arc::new(config),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            pool.clone(),
            BOT.into(),
            "T1".into(),
        );
        write_cursor(&pool, BOT, "CFREE", "", "100.000000")
            .await
            .unwrap();
        write_cursor(&pool, BOT, "C1", "1700.000000", "1700.500000")
            .await
            .unwrap();
        let thread =
            SessionKey::new(PLATFORM, Some("T1"), "C1", Some("1700.000000"), "").with_bot_id(BOT);
        sqlx::query("INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id) VALUES (?, 'slack', 'T1', 'C1', '1700.000000', '')")
            .bind(thread.storage_key())
            .execute(&pool)
            .await
            .unwrap();

        let routed = run(&ingress).await.unwrap();
        assert_eq!(routed, 2);

        let mut seen = Vec::new();
        while let Ok(content) = rx.try_recv() {
            seen.push(content);
        }
        seen.sort();
        assert_eq!(seen, ["fail", "hello", "thread reply"]);
        assert_eq!(
            cursor(&pool, "CFREE", "").await.as_deref(),
            Some("102.000000")
        );
        assert_eq!(
            cursor(&pool, "C1", "1700.000000").await.as_deref(),
            Some("1701.000000")
        );
        assert_eq!(cursor(&pool, "D1", "").await.as_deref(), Some("50.000000"));
    }

    #[test]
    fn replay_event_marks_channel_mentions_and_dms() {
        let event = replay_event(msg("1.0", "<@UBOT> hi"), "C1", "", BOT);
        assert_eq!(event["type"], "app_mention");
        assert_eq!(event["channel_type"], "channel");
        let event = replay_event(msg("1.0", "<@UBOT> hi"), "D1", "", BOT);
        assert_eq!(event["type"], "message");
        assert_eq!(event["channel_type"], "im");
        assert_eq!(event["channel"], "D1");
        let event = replay_event(msg("2.0", "reply"), "C1", "1.0", BOT);
        assert_eq!(event["thread_ts"], "1.0");
    }
}

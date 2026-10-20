//! Missed-message backfill: replays Slack messages that arrived while the gateway was down.
//!
//! Targets are free-response channels, the owner DM, every stored cursor and every known thread
//! session of this bot. Thread targets older than [`THREAD_TARGET_WINDOW_DAYS`] are dropped, so a
//! long-lived gateway does not sweep every thread it ever answered. Each target is read from its
//! cursor (`conversations.history` for channels, `conversations.replies` for threads) for up to
//! [`MAX_PAGES`] pages, paged on Slack's own `has_more` / `next_cursor` metadata rather than on
//! page length, mapped with the live ingress rules and routed with `route_awaiting_turn`.
//! The cursor advances past every message whose turn ran (even one
//! that failed or was stopped, so a reconcile pass never re-runs it) or that was skipped on
//! purpose; only a turn the gateway never admitted (draining for a restart) holds the cursor for
//! the next pass. A target with no
//! cursor yet is seeded at its newest message instead of replaying its whole history.
//!
//! A backlog deeper than [`MAX_PAGES`] pages is refused rather than truncated: the pass processes
//! nothing, holds the cursor and warns, so the messages stay replayable instead of being stepped
//! over by a cursor that can only move forward.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{json, Value};

use super::ingress::{read_cursor, ts_key, write_cursor, SlackIngress};
use super::PLATFORM;
use crate::{Result, SessionKey};

const PAGE_LIMIT: u32 = 200;
/// Pages read per target in one pass. This is a safety valve, not an operating mode: a backlog
/// deeper than this many pages is reported instead of silently truncated, because the cursor can
/// only move forward and a message it steps over is never replayed.
const MAX_PAGES: usize = 50;
/// Thread targets (cursors and thread sessions) untouched for this long leave the sweep.
const THREAD_TARGET_WINDOW_DAYS: i64 = 7;

pub async fn run(ingress: &SlackIngress) -> Result<usize> {
    run_settled(ingress, None).await
}

/// Like [`run`], but leaves messages younger than `settle` to live ingress (still in its
/// debouncer or being acknowledged), so a periodic pass never races a live delivery.
pub async fn run_settled(
    ingress: &SlackIngress,
    settle: Option<std::time::Duration>,
) -> Result<usize> {
    let pool = ingress.pool();
    let bot = ingress.bot_user_id().to_owned();
    let config = ingress.config();

    // Channel-level cursors are bounded by the channel count; thread cursors would otherwise
    // accumulate one row per thread the bot ever answered. A timestamp SQLite cannot parse stays
    // in the sweep: backfilling an old thread costs one call, dropping a live one loses a message.
    let window = format!("-{THREAD_TARGET_WINDOW_DAYS} days");
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT channel_id, thread_ts, last_ts FROM slack_cursors
         WHERE bot_id = ?
           AND (thread_ts = ''
                OR julianday(updated_at) IS NULL
                OR julianday(updated_at) >= julianday('now', ?))",
    )
    .bind(&bot)
    .bind(&window)
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
        "SELECT session_key FROM sessions
         WHERE platform = ? AND thread_id IS NOT NULL
           AND (julianday(updated_at) IS NULL
                OR julianday(updated_at) >= julianday('now', ?))",
    )
    .bind(PLATFORM)
    .bind(&window)
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
    let mut failed = 0usize;
    for (channel, thread_ts) in targets {
        let cursor = match snapshot.get(&(channel.clone(), thread_ts.clone())) {
            Some(cursor) => Some(cursor.clone()),
            None => read_cursor(pool, &bot, &channel, &thread_ts).await?,
        }
        .filter(|cursor| !cursor.trim().is_empty());
        match backfill_target(ingress, &channel, &thread_ts, cursor, settle).await {
            Ok(count) => routed += count,
            Err(error) => {
                failed += 1;
                tracing::warn!(%error, channel, thread_ts, "slack backfill target failed");
            }
        }
    }
    // One warning per pass, so a Slack outage that fails every target is visible as a whole
    // instead of only as N per-target lines an operator has to count.
    if failed > 0 {
        tracing::warn!(
            failed,
            routed,
            "slack backfill pass left targets unread; the next pass retries them"
        );
    }
    Ok(routed)
}

async fn backfill_target(
    ingress: &SlackIngress,
    channel: &str,
    thread_ts: &str,
    cursor: Option<String>,
    settle: Option<std::time::Duration>,
) -> Result<usize> {
    backfill_target_paged(ingress, channel, thread_ts, cursor, settle, MAX_PAGES).await
}

/// [`backfill_target`] with an explicit page budget, so a test can drive the backlog-over-budget
/// path without 50 network round trips.
async fn backfill_target_paged(
    ingress: &SlackIngress,
    channel: &str,
    thread_ts: &str,
    cursor: Option<String>,
    settle: Option<std::time::Duration>,
    pages: usize,
) -> Result<usize> {
    let settled_before = settle.map(|settle| {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(settle).unwrap_or_default();
        (
            cutoff.timestamp().max(0) as u64,
            cutoff.timestamp_subsec_micros(),
        )
    });
    let pool = ingress.pool();
    let bot = ingress.bot_user_id();
    let Some(cursor) = cursor else {
        seed_target(ingress, channel, thread_ts).await?;
        return Ok(0);
    };
    let fetched = fetch_messages(ingress, channel, thread_ts, Some(&cursor), pages).await?;
    let Some(mut messages) = backfillable(fetched) else {
        tracing::warn!(
            channel,
            thread_ts,
            pages,
            "slack backfill window did not reach the cursor; holding it so nothing is skipped"
        );
        return Ok(0);
    };
    messages.retain(|message| {
        message
            .get("ts")
            .and_then(Value::as_str)
            .is_some_and(|ts| ts != thread_ts && ts_key(ts) > ts_key(&cursor))
    });

    let mut routed = 0;
    for message in messages {
        let ts = message["ts"].as_str().unwrap_or_default().to_owned();
        if settled_before.is_some_and(|cutoff| ts_key(&ts) > cutoff) {
            break;
        }
        let event = replay_event(message, channel, thread_ts, bot);
        // intercept_lane_reply advances the cursor itself when it consumes the reply.
        if ingress.intercept_replayed_lane_reply(&event).await? {
            continue;
        }
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

/// First sight of a target: record its newest message so the next pass starts from there instead
/// of replaying the whole history.
async fn seed_target(ingress: &SlackIngress, channel: &str, thread_ts: &str) -> Result<()> {
    let api = ingress.api();
    let page = if thread_ts.is_empty() {
        api.conversations_history(channel, None, None, PAGE_LIMIT)
            .await?
    } else {
        api.conversations_replies(channel, thread_ts, None, PAGE_LIMIT)
            .await?
    };
    if let Some(newest) = page
        .iter()
        .filter_map(|message| message.get("ts").and_then(Value::as_str))
        .max_by_key(|ts| ts_key(ts))
    {
        write_cursor(
            ingress.pool(),
            ingress.bot_user_id(),
            channel,
            thread_ts,
            newest,
        )
        .await?;
    }
    Ok(())
}

/// One target's messages for one pass, plus whether the walk reached back to the cursor.
struct Fetched {
    messages: Vec<Value>,
    /// The page budget ran out while Slack still reported more *older* messages, so the messages
    /// between the cursor and this window were never fetched. Only `conversations.history` pages
    /// backwards from the newest message, so only there can the unfetched tail sit below the
    /// window and be lost by advancing the cursor.
    older_unfetched: bool,
}

/// The messages one pass may process, or `None` when the walk never reached the cursor.
///
/// `conversations.history` pages backwards from the newest message, so a window that ran out of
/// page budget has an unfetched gap between the cursor and its oldest message. Processing that
/// window would advance the monotonic cursor over the gap and lose those messages for good, so
/// the pass declines and leaves the cursor where it was.
fn backfillable(fetched: Fetched) -> Option<Vec<Value>> {
    if fetched.older_unfetched {
        return None;
    }
    Some(fetched.messages)
}

/// Reads the target's messages after `cursor` for up to `pages` pages, following Slack's own
/// `has_more` (channels) and `next_cursor` (threads) instead of inferring the end from page
/// length. `conversations.history` returns the newest page first, so channels page backwards with
/// `latest`; `conversations.replies` returns the oldest page first, so threads page forwards.
async fn fetch_messages(
    ingress: &SlackIngress,
    channel: &str,
    thread_ts: &str,
    cursor: Option<&str>,
    pages: usize,
) -> Result<Fetched> {
    let api = ingress.api();
    let mut messages: Vec<Value> = Vec::new();
    if thread_ts.is_empty() {
        let mut latest: Option<String> = None;
        let mut older_unfetched = false;
        for page_index in 0..pages {
            let page = api
                .conversations_history_page(channel, cursor, latest.as_deref(), PAGE_LIMIT)
                .await?;
            let oldest = oldest_ts(&page.messages);
            let has_more = page.has_more;
            messages.extend(page.messages);
            match oldest {
                // A boundary that does not move would re-read the same window forever.
                Some(oldest) if has_more && latest.as_deref() != Some(oldest.as_str()) => {
                    latest = Some(oldest);
                    // Budget spent with more older messages still reported: the window does not
                    // reach back to the cursor.
                    older_unfetched = page_index + 1 == pages;
                }
                _ => {
                    older_unfetched = false;
                    break;
                }
            }
        }
        return Ok(Fetched {
            messages: sorted_by_ts(dedupe_by_ts(messages)),
            older_unfetched,
        });
    }
    // Threads walk forward from the cursor, so an unfetched tail is always newer and the next
    // pass picks it up from the advanced cursor.
    let mut next: Option<String> = None;
    let mut first = true;
    for _ in 0..pages {
        let oldest = if first { cursor } else { None };
        first = false;
        let page = api
            .conversations_replies_page(channel, thread_ts, next.as_deref(), oldest, PAGE_LIMIT)
            .await?;
        let advanced = page
            .next_cursor
            .as_deref()
            .is_some_and(|next_cursor| next.as_deref() != Some(next_cursor));
        messages.extend(page.messages);
        if !advanced {
            break;
        }
        next = page.next_cursor;
    }
    Ok(Fetched {
        messages: sorted_by_ts(dedupe_by_ts(messages)),
        older_unfetched: false,
    })
}

fn oldest_ts(messages: &[Value]) -> Option<String> {
    messages
        .iter()
        .filter_map(|message| message.get("ts").and_then(Value::as_str))
        .min_by_key(|ts| ts_key(ts))
        .map(str::to_owned)
}

/// Page boundaries can repeat one message; keep a single copy of each ts.
fn dedupe_by_ts(messages: Vec<Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    messages
        .into_iter()
        .filter(|message| {
            message
                .get("ts")
                .and_then(Value::as_str)
                .is_none_or(|ts| seen.insert(ts.to_owned()))
        })
        .collect()
}

/// Oldest first, so the cursor advances in the order the messages arrived and a pass that stops
/// early (settle window, failed turn) holds the cursor below everything it did not process.
fn sorted_by_ts(mut messages: Vec<Value>) -> Vec<Value> {
    messages.sort_by_key(|message| {
        ts_key(
            message
                .get("ts")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
    });
    messages
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
            let body: Value = crate::slack::api::decode_test_body(&body);
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
    async fn replays_since_cursor_never_reruns_a_failed_turn_and_seeds_new_targets() {
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
        // "fail" ran and ended with an error: it counts as handled and is not retried.
        assert_eq!(routed, 3);

        let mut seen = Vec::new();
        while let Ok(content) = rx.try_recv() {
            seen.push(content);
        }
        seen.sort();
        assert_eq!(seen, ["fail", "hello", "thread reply"]);
        assert_eq!(
            cursor(&pool, "CFREE", "").await.as_deref(),
            Some("103.000000")
        );
        assert_eq!(
            cursor(&pool, "C1", "1700.000000").await.as_deref(),
            Some("1701.000000")
        );
        assert_eq!(cursor(&pool, "D1", "").await.as_deref(), Some("50.000000"));
    }

    #[tokio::test]
    async fn settled_pass_leaves_fresh_messages_to_live_ingress() {
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

        // A settle window reaching back before the fake timestamps: nothing is old enough yet.
        let far = std::time::Duration::from_secs(
            u64::try_from(chrono::Utc::now().timestamp()).unwrap() - 1000,
        );
        assert_eq!(run_settled(&ingress, Some(far)).await.unwrap(), 0);
        assert!(rx.try_recv().is_err());
        assert_eq!(
            cursor(&pool, "C1", "1700.000000").await.as_deref(),
            Some("1700.500000")
        );

        // Once settled, the missed reply is routed and the cursor moves past it.
        assert_eq!(
            run_settled(&ingress, Some(std::time::Duration::from_secs(60)))
                .await
                .unwrap(),
            1
        );
        assert_eq!(rx.try_recv().unwrap(), "thread reply");
        assert_eq!(
            cursor(&pool, "C1", "1700.000000").await.as_deref(),
            Some("1701.000000")
        );
    }

    /// Slack `conversations.history` semantics: newest first, `oldest` and `latest` exclusive, at
    /// most `limit` messages, and `has_more` set while older messages remain. Messages are
    /// `1..=total`.
    async fn history_fake(total: u64) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new().fallback(move |req: Request| async move {
            let method = req.uri().path().trim_start_matches("/api/").to_owned();
            let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                .await
                .unwrap();
            let body: Value = crate::slack::api::decode_test_body(&body);
            let reply = match method.as_str() {
                "conversations.open" => json!({ "ok": true, "channel": { "id": "D1" } }),
                "conversations.history" => {
                    let oldest = body["oldest"].as_str().map(ts_key);
                    let latest = body["latest"].as_str().map(ts_key);
                    let limit = body["limit"].as_u64().unwrap_or(200) as usize;
                    let in_range: Vec<u64> = (1..=total)
                        .filter(|index| {
                            let ts = ts_key(&format!("{index}.000000"));
                            oldest.is_none_or(|oldest| ts > oldest)
                                && latest.is_none_or(|latest| ts < latest)
                        })
                        .collect();
                    let has_more = in_range.len() > limit;
                    let messages: Vec<Value> = in_range
                        .iter()
                        .rev()
                        .take(limit)
                        .map(|index| msg(&format!("{index}.000000"), &format!("m{index}")))
                        .collect();
                    json!({ "ok": true, "has_more": has_more, "messages": messages })
                }
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

    #[derive(Default)]
    struct HistoryPageScript {
        messages: Vec<Value>,
        has_more: bool,
    }

    #[derive(Default)]
    struct RepliesPageScript {
        messages: Vec<Value>,
        next_cursor: Option<String>,
    }

    /// Serves scripted pages in order, so a test can hand Slack a short page that still reports
    /// more (channels) or a short page with a `next_cursor` (threads).
    #[derive(Clone, Default)]
    struct Script {
        history: Arc<parking_lot::Mutex<Vec<HistoryPageScript>>>,
        replies: Arc<parking_lot::Mutex<Vec<RepliesPageScript>>>,
    }

    async fn scripted_fake(script: Script) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new().fallback(move |req: Request| {
            let script = script.clone();
            async move {
                let method = req.uri().path().trim_start_matches("/api/").to_owned();
                let _ = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let reply = match method.as_str() {
                    "conversations.open" => json!({ "ok": true, "channel": { "id": "D1" } }),
                    "conversations.history" => {
                        let mut scripted = script.history.lock();
                        let page = if scripted.is_empty() {
                            HistoryPageScript::default()
                        } else {
                            scripted.remove(0)
                        };
                        drop(scripted);
                        json!({ "ok": true, "has_more": page.has_more, "messages": page.messages })
                    }
                    "conversations.replies" => {
                        let mut scripted = script.replies.lock();
                        let page = if scripted.is_empty() {
                            RepliesPageScript::default()
                        } else {
                            scripted.remove(0)
                        };
                        drop(scripted);
                        json!({ "ok": true, "messages": page.messages,
                                "response_metadata": { "next_cursor": page.next_cursor.unwrap_or_default() } })
                    }
                    _ => json!({ "ok": true, "messages": [] }),
                };
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(reply.to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        base
    }

    #[tokio::test]
    async fn history_backfill_reads_every_page_past_the_first() {
        let base = history_fake(205).await;
        let (ingress, _pool) = ingress_and_pool(&base).await;

        let fetched = fetch_messages(&ingress, "CPAGED", "", Some("0.000000"), MAX_PAGES)
            .await
            .unwrap();
        assert!(!fetched.older_unfetched, "205 messages fit in one pass");
        // 205 messages: one full 200-message page plus the remaining 5.
        assert_eq!(fetched.messages.len(), 205);
        let seen: Vec<&str> = fetched
            .messages
            .iter()
            .map(|message| message["ts"].as_str().unwrap())
            .collect();
        assert_eq!(seen.first().copied(), Some("1.000000"));
        assert_eq!(seen.last().copied(), Some("205.000000"));
    }

    async fn ingress_and_pool(base: &str) -> (SlackIngress, sqlx::SqlitePool) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(Runner(tx)),
            MultiplexerConfig::default(),
        );
        let ingress = SlackIngress::new(
            Arc::new(crate::slack::SlackConfig::test_default()),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            pool.clone(),
            BOT.into(),
            "T1".into(),
        );
        (ingress, pool)
    }

    #[tokio::test]
    async fn history_backlog_over_the_page_budget_holds_the_cursor_instead_of_skipping() {
        // 2600 missed messages: more than 10 pages of 200.
        let base = history_fake(2600).await;
        let (ingress, pool) = ingress_and_pool(&base).await;
        write_cursor(&pool, BOT, "CHUGE", "", "0.000000")
            .await
            .unwrap();

        // Ten pages cannot reach the cursor, so the pass declines and the cursor does not move:
        // the 600 messages it never fetched stay replayable.
        assert_eq!(
            backfill_target_paged(&ingress, "CHUGE", "", Some("0.000000".into()), None, 10)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            cursor(&pool, "CHUGE", "").await.as_deref(),
            Some("0.000000"),
            "a held pass must leave the cursor exactly where it was"
        );

        // With the real budget the whole backlog is walked in one pass and the cursor reaches the
        // newest message. CHUGE is not free-response, so these plain messages route nothing; what
        // this pins is that the walk reaches ts 1.000000 instead of stopping 600 messages short.
        assert_eq!(
            backfill_target(&ingress, "CHUGE", "", Some("0.000000".into()), None)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            cursor(&pool, "CHUGE", "").await.as_deref(),
            Some("2600.000000")
        );
    }

    #[tokio::test]
    async fn a_2600_message_backlog_fits_the_production_budget() {
        let base = history_fake(2600).await;
        let (ingress, _pool) = ingress_and_pool(&base).await;
        let fetched = fetch_messages(&ingress, "CHUGE", "", Some("0.000000"), MAX_PAGES)
            .await
            .unwrap();
        assert!(
            !fetched.older_unfetched,
            "MAX_PAGES must be deep enough for a 2600-message backlog"
        );
        assert_eq!(fetched.messages.len(), 2600);
        assert_eq!(fetched.messages[0]["ts"], "1.000000");
    }

    #[tokio::test]
    async fn a_short_history_page_does_not_end_the_walk_when_slack_reports_more() {
        let script = Script::default();
        script.history.lock().push(HistoryPageScript {
            messages: vec![msg("3.000000", "m3"), msg("2.000000", "m2")],
            has_more: true,
        });
        script.history.lock().push(HistoryPageScript {
            messages: vec![msg("1.000000", "m1"), msg("0.500000", "m0")],
            has_more: false,
        });
        let base = scripted_fake(script).await;
        let (ingress, _pool) = ingress_and_pool(&base).await;

        let fetched = fetch_messages(&ingress, "CSHORT", "", None, MAX_PAGES)
            .await
            .unwrap();
        assert_eq!(
            fetched.messages.len(),
            4,
            "a two-message page with has_more must not end the walk"
        );
        assert!(!fetched.older_unfetched);
        assert_eq!(fetched.messages[0]["ts"], "0.500000");
        assert_eq!(fetched.messages[3]["ts"], "3.000000");
    }

    #[tokio::test]
    async fn thread_paging_follows_next_cursor_not_page_length() {
        let script = Script::default();
        script.replies.lock().push(RepliesPageScript {
            messages: vec![msg("10.000000", "root"), msg("11.000000", "r1")],
            next_cursor: Some("page2".into()),
        });
        script.replies.lock().push(RepliesPageScript {
            messages: vec![msg("12.000000", "r2")],
            next_cursor: None,
        });
        let base = scripted_fake(script).await;
        let (ingress, _pool) = ingress_and_pool(&base).await;

        let fetched = fetch_messages(&ingress, "C1", "10.000000", Some("10.500000"), MAX_PAGES)
            .await
            .unwrap();
        assert_eq!(
            fetched.messages.len(),
            3,
            "a short replies page with next_cursor must not end the walk"
        );
        assert_eq!(fetched.messages[2]["ts"], "12.000000");
    }

    #[test]
    fn backfillable_keeps_every_message_when_the_walk_reached_the_cursor() {
        let messages = vec![msg("1.000000", "a"), msg("2.000000", "b")];
        let processed = backfillable(Fetched {
            messages: messages.clone(),
            older_unfetched: false,
        });
        assert_eq!(
            processed.expect("a complete window is processable").len(),
            2
        );
    }

    /// Records every API call and answers with empty message lists.
    async fn recording_fake() -> (String, Arc<parking_lot::Mutex<Vec<(String, Value)>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let app = axum::Router::new().fallback(move |req: Request| {
            let recorded = recorded.clone();
            async move {
                let method = req.uri().path().trim_start_matches("/api/").to_owned();
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body: Value = crate::slack::api::decode_test_body(&body);
                let reply = match method.as_str() {
                    "conversations.open" => json!({ "ok": true, "channel": { "id": "D1" } }),
                    _ => json!({ "ok": true, "messages": [] }),
                };
                recorded.lock().push((method, body));
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(reply.to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        (base, calls)
    }

    #[tokio::test]
    async fn reconcile_skips_thread_targets_idle_for_more_than_a_week() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(Runner(tx)),
            MultiplexerConfig::default(),
        );
        let (base, calls) = recording_fake().await;
        let ingress = SlackIngress::new(
            Arc::new(crate::slack::SlackConfig::test_default()),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            pool.clone(),
            BOT.into(),
            "T1".into(),
        );
        for (channel, thread, age) in [
            ("CTNEW", "1.000000", "-1 hours"),
            ("CTOLD", "2.000000", "-30 days"),
        ] {
            let key =
                SessionKey::new(PLATFORM, Some("T1"), channel, Some(thread), "").with_bot_id(BOT);
            sqlx::query(
                "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, updated_at)
                 VALUES (?, 'slack', 'T1', ?, ?, '', datetime('now', ?))",
            )
            .bind(key.storage_key())
            .bind(channel)
            .bind(thread)
            .bind(age)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO slack_cursors (bot_id, channel_id, thread_ts, last_ts, updated_at)
                 VALUES (?, ?, ?, '1.000000', datetime('now', ?))",
            )
            .bind(BOT)
            .bind(channel)
            .bind(thread)
            .bind(age)
            .execute(&pool)
            .await
            .unwrap();
        }

        assert_eq!(run(&ingress).await.unwrap(), 0);
        let threads: Vec<String> = calls
            .lock()
            .iter()
            .filter(|(method, _)| method == "conversations.replies")
            .map(|(_, body)| body["channel"].as_str().unwrap_or_default().to_owned())
            .collect();
        assert_eq!(threads, ["CTNEW"], "a thread idle for 30 days is not swept");
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

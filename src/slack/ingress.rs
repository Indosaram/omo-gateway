//! Events API ingress: maps Slack message events to `InboundEvent`s and routes them.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

use super::{SlackApi, SlackConfig, PLATFORM};
use crate::{DeliveryLedgerService, InboundEvent, MessageAttachment, Result, SessionKey};

const PROCESSING_REACTION: &str = "eyes";

pub struct SlackIngress {
    config: Arc<SlackConfig>,
    api: SlackApi,
    multiplexer: crate::SessionMultiplexer,
    pool: SqlitePool,
    bot_user_id: String,
    team_id: String,
}

impl SlackIngress {
    pub fn new(
        config: Arc<SlackConfig>,
        api: SlackApi,
        multiplexer: crate::SessionMultiplexer,
        pool: SqlitePool,
        bot_user_id: String,
        team_id: String,
    ) -> Self {
        Self {
            config,
            api,
            multiplexer,
            pool,
            bot_user_id,
            team_id,
        }
    }

    pub async fn handle(&self, event: Value) -> Result<()> {
        let known_thread = match thread_candidate(&self.bot_user_id, &self.team_id, &event) {
            Some(key) => session_exists(&self.pool, &key).await?,
            None => false,
        };
        let Some(mut inbound) = map_event(
            &self.config,
            &self.bot_user_id,
            &self.team_id,
            &event,
            known_thread,
        ) else {
            return Ok(());
        };
        let delivery_id = inbound
            .delivery_id
            .clone()
            .unwrap_or_else(|| format!("slack:{}", inbound.platform_message_id));
        let ledger = DeliveryLedgerService::new(self.pool.clone());
        if ledger.is_duplicate(&delivery_id).await? {
            tracing::info!(delivery_id, "Ignoring duplicate Slack delivery");
            return Ok(());
        }

        if inbound.content.trim().eq_ignore_ascii_case("/stop") {
            let interrupted = self.multiplexer.stop(&inbound.session).await?;
            tracing::info!(session = %inbound.session, interrupted, "processed Slack text stop command");
            return Ok(());
        }

        let channel = inbound.session.channel_id.clone();
        if let Err(error) = self
            .api
            .add_reaction(&channel, &inbound.platform_message_id, PROCESSING_REACTION)
            .await
        {
            tracing::warn!(%error, channel, "failed to add Slack processing reaction");
        }

        if inbound.attachments.is_empty() && !inbound.content.trim().is_empty() {
            match steer_claimed_event(&self.pool, &self.multiplexer, &inbound, &delivery_id).await
            {
                Ok(true) => {
                    tracing::info!(session = %inbound.session, "steered Slack message into active turn");
                    return Ok(());
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::error!(session = %inbound.session, %error, "failed to steer Slack message into active turn");
                }
            }
        }

        if !ledger.record_incoming_as(&inbound, &delivery_id).await? {
            tracing::info!(delivery_id, "Ignoring duplicate Slack delivery");
            return Ok(());
        }
        self.download_attachments(&mut inbound).await;
        inbound.delivery_id = Some(delivery_id.clone());
        if let Err(error) = self.multiplexer.route(inbound).await {
            ledger.mark_failed(&delivery_id, error.to_string()).await?;
            return Err(error);
        }
        Ok(())
    }

    async fn download_attachments(&self, event: &mut InboundEvent) {
        let dir = self
            .config
            .attachment_dir
            .join(safe_component(&event.session.channel_id));
        for attachment in &mut event.attachments {
            let dest = dir.join(format!(
                "{}-{}",
                safe_component(&event.platform_message_id),
                safe_component(&attachment.filename)
            ));
            match self.api.download_file(&attachment.url, &dest).await {
                Ok(()) => attachment.local_path = Some(dest),
                Err(error) => tracing::warn!(
                    attachment_id = %attachment.id,
                    filename = %attachment.filename,
                    %error,
                    "failed to download Slack attachment; routing remote metadata only"
                ),
            }
        }
    }
}

/// Injects the message into a turn already running in the session. Returns `Ok(false)` when no
/// turn is active; the ledger is written only once the steer landed.
async fn steer_claimed_event(
    pool: &SqlitePool,
    multiplexer: &crate::SessionMultiplexer,
    event: &InboundEvent,
    delivery_id: &str,
) -> Result<bool> {
    if !multiplexer.steer(&event.session, &event.content).await? {
        return Ok(false);
    }
    let ledger = DeliveryLedgerService::new(pool.clone());
    if let Err(error) = ledger.record_incoming_as(event, delivery_id).await {
        tracing::warn!(delivery_id, %error, "failed to record steered Slack delivery");
    }
    if let Err(error) = crate::multiplexer::record_inbound_transcript(pool, event).await {
        tracing::warn!(%error, "failed to mirror steered Slack message into transcript");
    } else {
        let index = crate::MessageSearchIndex::new(pool.clone());
        match index.index_inbound(&event.session, event).await {
            Ok(true) => {
                if let Some(sender) = multiplexer.inbox_event_sender() {
                    if let Err(error) = crate::inbox::watchers::on_gateway_message(&sender).await
                    {
                        tracing::warn!(%error, "Slack inbox event receiver is closed after message index commit");
                    }
                }
            }
            Ok(false) => {}
            Err(error) => tracing::warn!(%error, "failed to index steered Slack message"),
        }
    }
    let _ = ledger.mark_delivered(delivery_id).await;
    Ok(true)
}

async fn session_exists(pool: &SqlitePool, key: &SessionKey) -> Result<bool> {
    let row: Option<i64> = sqlx::query_scalar("SELECT 1 FROM sessions WHERE session_key = ?")
        .bind(key.storage_key())
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

fn thread_candidate(bot_user_id: &str, team_id: &str, event: &Value) -> Option<SessionKey> {
    let channel = str_field(event, "channel")?;
    if is_dm(event, channel) {
        return None;
    }
    let thread_ts = reply_thread_ts(event)?;
    Some(
        SessionKey::new(PLATFORM, Some(team_id), channel, Some(thread_ts), "")
            .with_bot_id(bot_user_id),
    )
}

/// Pure mapping from a Slack `message` / `app_mention` event to an inbound event. `None` drops it.
pub fn map_event(
    config: &SlackConfig,
    bot_user_id: &str,
    team_id: &str,
    event: &Value,
    known_thread: bool,
) -> Option<InboundEvent> {
    let kind = str_field(event, "type")?;
    match kind {
        "message" => {
            if !matches!(
                str_field(event, "subtype"),
                None | Some("file_share" | "thread_broadcast")
            ) {
                return None;
            }
        }
        "app_mention" => {}
        _ => return None,
    }
    if event.get("bot_id").is_some_and(|value| !value.is_null()) {
        return None;
    }
    let user = str_field(event, "user")?;
    if user == bot_user_id || !config.is_allowed(user) {
        return None;
    }
    let channel = str_field(event, "channel")?;
    let ts = str_field(event, "ts")?;
    let text = str_field(event, "text").unwrap_or_default();
    let mention = format!("<@{bot_user_id}>");
    let mentioned = text.contains(&mention);
    let reply_thread = reply_thread_ts(event);

    let session = (if is_dm(event, channel) {
        SessionKey::new(PLATFORM, None::<String>, channel, reply_thread, user)
    } else {
        if kind == "message" {
            if mentioned {
                return None;
            }
            let free = config.free_response_channels.iter().any(|id| id == channel);
            if !free && !known_thread {
                return None;
            }
        }
        let thread = reply_thread.or_else(|| {
            (config.auto_thread && kind == "app_mention").then_some(ts)
        });
        SessionKey::new(PLATFORM, Some(team_id), channel, thread, user)
    })
    .with_bot_id(bot_user_id);

    let content = text.replace(&mention, "").trim().to_owned();
    let attachments = attachments(event);
    if content.is_empty() && attachments.is_empty() {
        return None;
    }

    let mut inbound = InboundEvent::message(session, ts, content)
        .with_delivery_id(format!("slack:{channel}:{ts}"))
        .with_author_id(user)
        .with_attachments(attachments);
    if let Some(received_at) = ts_to_datetime(ts) {
        inbound = inbound.with_received_at(received_at);
    }
    Some(inbound)
}

fn str_field<'a>(event: &'a Value, field: &str) -> Option<&'a str> {
    event.get(field).and_then(Value::as_str)
}

fn is_dm(event: &Value, channel: &str) -> bool {
    match str_field(event, "channel_type") {
        Some(channel_type) => channel_type == "im",
        None => channel.starts_with('D'),
    }
}

fn reply_thread_ts(event: &Value) -> Option<&str> {
    let thread_ts = str_field(event, "thread_ts")?;
    (Some(thread_ts) != str_field(event, "ts")).then_some(thread_ts)
}

fn attachments(event: &Value) -> Vec<MessageAttachment> {
    let Some(files) = event.get("files").and_then(Value::as_array) else {
        return Vec::new();
    };
    files
        .iter()
        .filter_map(|file| {
            let id = str_field(file, "id")?;
            let url = str_field(file, "url_private_download")
                .or_else(|| str_field(file, "url_private"))?;
            Some(MessageAttachment {
                id: id.to_owned(),
                filename: str_field(file, "name").unwrap_or(id).to_owned(),
                url: url.to_owned(),
                content_type: str_field(file, "mimetype").map(str::to_owned),
                size_bytes: file.get("size").and_then(Value::as_u64),
                local_path: None,
                text_content: None,
            })
        })
        .collect()
}

fn ts_to_datetime(ts: &str) -> Option<DateTime<Utc>> {
    let (secs, frac) = ts.split_once('.').unwrap_or((ts, ""));
    let secs = secs.parse::<i64>().ok()?;
    let micros = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<6}").get(..6)?.parse::<u32>().ok()?
    };
    DateTime::from_timestamp(secs, micros * 1_000)
}

fn safe_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | '\0') { '_' } else { c })
        .collect();
    match cleaned.as_str() {
        "" | "." | ".." => "_".to_owned(),
        _ => cleaned,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::response::Response;
    use parking_lot::Mutex;
    use serde_json::json;

    use super::*;
    use crate::{Database, MultiplexerConfig, SessionMultiplexer};

    const BOT: &str = "UBOT";
    const TEAM: &str = "T1";
    const OWNER: &str = "UOWNER";

    fn config() -> SlackConfig {
        SlackConfig {
            bot_token: "xoxb-test".into(),
            app_token: "xapp-test".into(),
            allowed_users: vec![OWNER.into()],
            free_response_channels: vec!["CFREE".into()],
            auto_thread: true,
            attachment_dir: PathBuf::from("/tmp/omon-slack-test"),
        }
    }

    fn channel_mention(kind: &str) -> Value {
        json!({
            "type": kind,
            "channel": "C1",
            "channel_type": "channel",
            "user": OWNER,
            "text": "<@UBOT> hello there",
            "ts": "1700000000.000100",
        })
    }

    #[test]
    fn top_level_dm_maps_to_dm_session_without_thread() {
        let event = json!({
            "type": "message",
            "channel": "D1",
            "channel_type": "im",
            "user": OWNER,
            "text": "hi bot",
            "ts": "1700000000.000200",
        });
        let inbound = map_event(&config(), BOT, TEAM, &event, false).unwrap();
        assert_eq!(
            inbound.session,
            SessionKey::new("slack", None::<String>, "D1", None::<String>, OWNER).with_bot_id(BOT)
        );
        assert_eq!(inbound.content, "hi bot");
        assert_eq!(inbound.platform_message_id, "1700000000.000200");
        assert_eq!(inbound.delivery_id.as_deref(), Some("slack:D1:1700000000.000200"));
        assert_eq!(inbound.author_id.as_deref(), Some(OWNER));
        assert_eq!(inbound.received_at.timestamp(), 1_700_000_000);
        assert_eq!(inbound.received_at.timestamp_subsec_micros(), 200);
    }

    #[test]
    fn dm_thread_reply_uses_thread_ts() {
        let event = json!({
            "type": "message",
            "channel": "D1",
            "channel_type": "im",
            "user": OWNER,
            "text": "follow up",
            "ts": "1700000001.000000",
            "thread_ts": "1700000000.000200",
        });
        let inbound = map_event(&config(), BOT, TEAM, &event, false).unwrap();
        assert_eq!(inbound.session.thread_id.as_deref(), Some("1700000000.000200"));
        assert_eq!(inbound.session.guild_id, None);
        assert_eq!(inbound.session.user_id, OWNER);
    }

    #[test]
    fn channel_mention_auto_threads_at_mention_ts_and_strips_mention() {
        let inbound = map_event(&config(), BOT, TEAM, &channel_mention("app_mention"), false)
            .unwrap();
        assert_eq!(
            inbound.session,
            SessionKey::new("slack", Some(TEAM), "C1", Some("1700000000.000100"), "")
                .with_bot_id(BOT)
        );
        assert_eq!(inbound.content, "hello there");

        let mut no_thread = config();
        no_thread.auto_thread = false;
        let inbound =
            map_event(&no_thread, BOT, TEAM, &channel_mention("app_mention"), false).unwrap();
        assert_eq!(inbound.session.thread_id, None);
    }

    #[test]
    fn channel_message_with_mention_is_left_to_app_mention() {
        assert!(map_event(&config(), BOT, TEAM, &channel_mention("message"), false).is_none());
        assert!(map_event(&config(), BOT, TEAM, &channel_mention("message"), true).is_none());
        let a = map_event(&config(), BOT, TEAM, &channel_mention("app_mention"), false).unwrap();
        assert_eq!(a.delivery_id.as_deref(), Some("slack:C1:1700000000.000100"));
    }

    #[test]
    fn thread_reply_without_mention_needs_known_thread_or_free_channel() {
        let reply = json!({
            "type": "message",
            "channel": "C1",
            "channel_type": "channel",
            "user": OWNER,
            "text": "more context",
            "ts": "1700000005.000000",
            "thread_ts": "1700000000.000100",
        });
        assert!(map_event(&config(), BOT, TEAM, &reply, false).is_none());
        let inbound = map_event(&config(), BOT, TEAM, &reply, true).unwrap();
        assert_eq!(inbound.session.thread_id.as_deref(), Some("1700000000.000100"));
        assert_eq!(inbound.session.guild_id.as_deref(), Some(TEAM));

        let mut free = reply.clone();
        free["channel"] = json!("CFREE");
        free.as_object_mut().unwrap().remove("thread_ts");
        let inbound = map_event(&config(), BOT, TEAM, &free, false).unwrap();
        assert_eq!(inbound.session.channel_id, "CFREE");
        assert_eq!(inbound.session.thread_id, None);
    }

    #[test]
    fn bot_self_unauthorized_and_unhandled_events_are_dropped() {
        let base = json!({
            "type": "message",
            "channel": "D1",
            "channel_type": "im",
            "user": OWNER,
            "text": "hi",
            "ts": "1.2",
        });
        assert!(map_event(&config(), BOT, TEAM, &base, false).is_some());

        let mut bot = base.clone();
        bot["bot_id"] = json!("B1");
        assert!(map_event(&config(), BOT, TEAM, &bot, false).is_none());

        let mut own = base.clone();
        own["user"] = json!(BOT);
        assert!(map_event(&config(), BOT, TEAM, &own, false).is_none());

        let mut stranger = base.clone();
        stranger["user"] = json!("USTRANGER");
        assert!(map_event(&config(), BOT, TEAM, &stranger, false).is_none());

        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove("user");
        assert!(map_event(&config(), BOT, TEAM, &missing, false).is_none());

        for subtype in ["message_changed", "message_deleted", "channel_join"] {
            let mut edited = base.clone();
            edited["subtype"] = json!(subtype);
            assert!(map_event(&config(), BOT, TEAM, &edited, false).is_none(), "{subtype}");
        }
        let mut broadcast = base.clone();
        broadcast["subtype"] = json!("thread_broadcast");
        assert!(map_event(&config(), BOT, TEAM, &broadcast, false).is_some());

        let mut reaction = base.clone();
        reaction["type"] = json!("reaction_added");
        assert!(map_event(&config(), BOT, TEAM, &reaction, false).is_none());

        let mut empty = base;
        empty["text"] = json!("   ");
        assert!(map_event(&config(), BOT, TEAM, &empty, false).is_none());
    }

    #[test]
    fn files_become_attachments_and_allow_empty_text() {
        let event = json!({
            "type": "message",
            "subtype": "file_share",
            "channel": "D1",
            "channel_type": "im",
            "user": OWNER,
            "text": "",
            "ts": "1.2",
            "files": [
                {
                    "id": "F1",
                    "name": "a.png",
                    "mimetype": "image/png",
                    "size": 42,
                    "url_private": "https://files.slack.com/a",
                    "url_private_download": "https://files.slack.com/a/download",
                },
                { "id": "F2", "url_private": "https://files.slack.com/b" },
                { "id": "F3" },
            ],
        });
        let inbound = map_event(&config(), BOT, TEAM, &event, false).unwrap();
        assert_eq!(inbound.content, "");
        assert_eq!(
            inbound.attachments,
            vec![
                MessageAttachment {
                    id: "F1".into(),
                    filename: "a.png".into(),
                    url: "https://files.slack.com/a/download".into(),
                    content_type: Some("image/png".into()),
                    size_bytes: Some(42),
                    local_path: None,
                    text_content: None,
                },
                MessageAttachment {
                    id: "F2".into(),
                    filename: "F2".into(),
                    url: "https://files.slack.com/b".into(),
                    content_type: None,
                    size_bytes: None,
                    local_path: None,
                    text_content: None,
                },
            ]
        );
    }

    #[test]
    fn thread_candidate_only_for_channel_replies() {
        let reply = json!({ "channel": "C1", "channel_type": "channel", "ts": "2.0", "thread_ts": "1.0" });
        assert_eq!(
            thread_candidate(BOT, TEAM, &reply),
            Some(SessionKey::new("slack", Some(TEAM), "C1", Some("1.0"), "").with_bot_id(BOT))
        );
        let parent = json!({ "channel": "C1", "channel_type": "channel", "ts": "1.0", "thread_ts": "1.0" });
        assert_eq!(thread_candidate(BOT, TEAM, &parent), None);
        let dm = json!({ "channel": "D1", "channel_type": "im", "ts": "2.0", "thread_ts": "1.0" });
        assert_eq!(thread_candidate(BOT, TEAM, &dm), None);
    }

    #[test]
    fn safe_component_blocks_path_traversal() {
        assert_eq!(safe_component("../etc/passwd"), ".._etc_passwd");
        assert_eq!(safe_component(".."), "_");
        assert_eq!(safe_component("a.png"), "a.png");
    }

    struct SignalRunner(tokio::sync::mpsc::UnboundedSender<String>);

    #[async_trait]
    impl crate::AgentRunner for SignalRunner {
        async fn run(
            &self,
            _session: &mut crate::SessionContext,
            event: InboundEvent,
        ) -> Result<()> {
            let _ = self.0.send(event.content);
            Ok(())
        }
    }

    async fn fake_slack() -> (String, Arc<Mutex<Vec<(String, Value)>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let app = axum::Router::new().fallback(move |req: Request| {
            let recorded = recorded.clone();
            async move {
                let path = req.uri().path().to_owned();
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
                recorded.lock().push((path, body));
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "ok": true }).to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        (base, calls)
    }

    #[tokio::test]
    async fn handle_routes_once_reacts_and_dedupes_redelivery() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            db.pool().clone(),
            Arc::new(SignalRunner(tx)),
            MultiplexerConfig::default(),
        );
        let (base, calls) = fake_slack().await;
        let ingress = SlackIngress::new(
            Arc::new(config()),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            db.pool().clone(),
            BOT.into(),
            TEAM.into(),
        );

        let mention = channel_mention("app_mention");
        ingress.handle(mention.clone()).await.unwrap();
        let routed = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("turn should reach the runner")
            .expect("runner channel open");
        assert_eq!(routed, "hello there");

        ingress.handle(mention).await.unwrap();
        ingress.handle(channel_mention("message")).await.unwrap();

        let calls = calls.lock().clone();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0].0, "/api/reactions.add");
        assert_eq!(calls[0].1["name"], "eyes");
        assert_eq!(calls[0].1["channel"], "C1");
        assert_eq!(calls[0].1["timestamp"], "1700000000.000100");

        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM delivery_ledger WHERE message_id = 'slack:C1:1700000000.000100'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(rows, 1);

        let thread_key = SessionKey::new("slack", Some(TEAM), "C1", Some("1700000000.000100"), "")
            .with_bot_id(BOT);
        assert!(session_exists(db.pool(), &thread_key).await.unwrap());
    }

    #[tokio::test]
    async fn handle_accepts_reply_in_known_bot_thread_only() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let multiplexer = SessionMultiplexer::new(
            db.pool().clone(),
            Arc::new(SignalRunner(tx)),
            MultiplexerConfig::default(),
        );
        let (base, calls) = fake_slack().await;
        let ingress = SlackIngress::new(
            Arc::new(config()),
            SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api")),
            multiplexer,
            db.pool().clone(),
            BOT.into(),
            TEAM.into(),
        );
        let reply = json!({
            "type": "message",
            "channel": "C1",
            "channel_type": "channel",
            "user": OWNER,
            "text": "and another thing",
            "ts": "1700000009.000000",
            "thread_ts": "1700000000.000100",
        });

        ingress.handle(reply.clone()).await.unwrap();
        assert!(calls.lock().is_empty());

        ingress.handle(channel_mention("app_mention")).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();

        ingress.handle(reply).await.unwrap();
        let routed = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("reply in known thread should be routed")
            .expect("runner channel open");
        assert_eq!(routed, "and another thing");
    }
}

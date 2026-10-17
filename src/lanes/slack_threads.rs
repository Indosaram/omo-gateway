//! Slack lane threads. A Slack thread has no channel object of its own: the lane
//! starter message posted to the parent channel is the thread root, and the lane
//! thread ref stored in `lanes.thread_channel_id` is `slack:<channel>:<root_ts>`.
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;

use super::discord_threads::{
    strip_status_prefix, DiscordThreadError, DiscordThreads, ThreadStatus,
};
use super::posts::{Post, PostSink};
use super::runner::LaneLifecycle;
use super::scrub::scrub;
use crate::slack::mrkdwn::{split_message, to_mrkdwn};
use crate::slack::{SlackApi, SlackConfig, MESSAGE_LIMIT};
use crate::{OmonError, Result};

pub const REF_PREFIX: &str = "slack:";
const CLOSED_REPLY: &str = "✅ lane closed";

pub fn thread_ref(channel: &str, thread_ts: &str) -> String {
    format!("{REF_PREFIX}{channel}:{thread_ts}")
}

pub fn is_slack_ref(thread: &str) -> bool {
    thread.starts_with(REF_PREFIX)
}

pub fn parse_ref(thread: &str) -> Option<(&str, &str)> {
    let (channel, ts) = thread.strip_prefix(REF_PREFIX)?.split_once(':')?;
    (is_channel_id(channel)
        && !ts.is_empty()
        && ts.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
    .then_some((channel, ts))
}

pub fn parse_parent(parent: &str) -> Option<&str> {
    let channel = parent.strip_prefix(REF_PREFIX).unwrap_or(parent);
    is_channel_id(channel).then_some(channel)
}

fn is_channel_id(id: &str) -> bool {
    id.len() >= 2
        && matches!(id.as_bytes()[0], b'C' | b'G' | b'D')
        && id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

pub fn valid_thread_ref(thread: &str) -> bool {
    parse_ref(thread).is_some() || thread.parse::<u64>().is_ok_and(|id| id != 0)
}

fn reaction(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Working => "arrows_counterclockwise",
        ThreadStatus::Waiting => "double_vertical_bar",
        ThreadStatus::Done => "white_check_mark",
    }
}

/// Slack may echo status emoji back as shortcodes; drop either form.
fn strip_slack_status(text: &str) -> &str {
    let mut rest = text.trim();
    for code in [
        ":arrows_counterclockwise:",
        ":double_vertical_bar:",
        ":white_check_mark:",
    ] {
        if let Some(stripped) = rest.strip_prefix(code) {
            rest = stripped;
            break;
        }
    }
    strip_status_prefix(rest.trim_start_matches('\u{fe0f}'))
        .trim_start_matches('\u{fe0f}')
        .trim()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Lane posts never ping: user, special and subteam mentions are shown literally.
fn no_pings(text: &str) -> String {
    text.replace("<!", "&lt;!").replace("<@", "&lt;@")
}

fn starter_text(status: ThreadStatus, key: &str, title: &str) -> String {
    let title = escape(scrub(title).trim());
    let key = escape(scrub(key).trim());
    if key.is_empty() {
        format!("{}lane: {title}", status.prefix())
    } else {
        format!("{}lane {key}: {title}", status.prefix())
    }
}

fn invalid_ref(thread: &str) -> OmonError {
    OmonError::Config(format!("invalid Slack lane thread ref: {thread}"))
}

#[derive(Clone)]
pub struct SlackLaneThreads {
    api: SlackApi,
    bot_user_id: Arc<tokio::sync::OnceCell<String>>,
}

impl SlackLaneThreads {
    pub fn new(api: SlackApi, bot_user_id: String) -> Self {
        Self {
            api,
            bot_user_id: Arc::new(tokio::sync::OnceCell::from(bot_user_id)),
        }
    }

    pub fn lazy(api: SlackApi) -> Self {
        Self {
            api,
            bot_user_id: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    pub fn from_env() -> Option<Self> {
        SlackConfig::from_env(std::path::Path::new("."))
            .map(|config| Self::lazy(SlackApi::new(config.bot_token)))
    }

    pub fn api(&self) -> &SlackApi {
        &self.api
    }

    pub async fn bot_user_id(&self) -> Result<String> {
        self.bot_user_id
            .get_or_try_init(|| async { Ok::<_, OmonError>(self.api.auth_test().await?.user_id) })
            .await
            .cloned()
    }

    pub async fn create_thread(&self, parent: &str, key: &str, title: &str) -> Result<String> {
        let channel = parse_parent(parent)
            .ok_or_else(|| OmonError::Config(format!("invalid Slack lane parent: {parent}")))?;
        let ts = self
            .api
            .post_message(
                channel,
                &starter_text(ThreadStatus::Working, key, title),
                None,
                None,
            )
            .await?;
        Ok(thread_ref(channel, &ts))
    }

    pub async fn delete_thread(&self, thread: &str) -> Result<()> {
        let (channel, ts) = parse_ref(thread).ok_or_else(|| invalid_ref(thread))?;
        self.api.delete_message(channel, ts).await
    }

    async fn starter(&self, channel: &str, ts: &str) -> Result<Option<String>> {
        Ok(self
            .api
            .conversations_replies(channel, ts, None, 1)
            .await?
            .into_iter()
            .find(|message| message["ts"].as_str() == Some(ts))
            .and_then(|message| message["text"].as_str().map(str::to_owned)))
    }

    pub async fn set_status(
        &self,
        thread: &str,
        status: ThreadStatus,
        title: Option<&str>,
    ) -> Result<()> {
        let (channel, ts) = parse_ref(thread).ok_or_else(|| invalid_ref(thread))?;
        let text = match (self.starter(channel, ts).await?, title) {
            (Some(current), _) => format!("{}{}", status.prefix(), strip_slack_status(&current)),
            (None, Some(title)) => starter_text(status, "", title),
            (None, None) => {
                return Err(OmonError::Config(format!(
                    "Slack lane starter message not found: {thread}"
                )))
            }
        };
        self.api.update_message(channel, ts, &text, None).await?;
        for other in ThreadStatus::ALL
            .into_iter()
            .filter(|other| *other != status)
        {
            if let Err(error) = self.api.remove_reaction(channel, ts, reaction(other)).await {
                tracing::warn!(%thread, %error, "lane status reaction removal failed");
            }
        }
        if let Err(error) = self.api.add_reaction(channel, ts, reaction(status)).await {
            tracing::warn!(%thread, %error, "lane status reaction failed");
        }
        Ok(())
    }
}

#[async_trait]
impl PostSink for SlackLaneThreads {
    async fn send(&self, post: &Post) -> Result<String> {
        let thread = post.thread.as_deref().unwrap_or_default();
        let (channel, ts) = parse_ref(thread)
            .ok_or_else(|| OmonError::Config("lane post has no valid thread destination".into()))?;
        let mut first = None;
        for chunk in split_message(&no_pings(&to_mrkdwn(&post.body)), MESSAGE_LIMIT) {
            let sent = self
                .api
                .post_message(channel, &chunk, Some(ts), None)
                .await?;
            first.get_or_insert(sent);
        }
        first.ok_or_else(|| OmonError::Config("lane post body is empty".into()))
    }

    async fn edit_status(&self, thread: &str, message: &str, body: &str) -> Result<bool> {
        let (channel, _) = parse_ref(thread).ok_or_else(|| invalid_ref(thread))?;
        match self
            .api
            .update_message(channel, message, &no_pings(&to_mrkdwn(body)), None)
            .await
        {
            Ok(()) => Ok(true),
            Err(OmonError::Multiplexer(error)) if error.ends_with(": message_not_found") => {
                Ok(false)
            }
            Err(error) => Err(OmonError::Config(format!("lane status edit: {error}"))),
        }
    }

    async fn rename_status(&self, thread: &str, status: ThreadStatus, title: &str) -> Result<()> {
        self.set_status(thread, status, Some(title))
            .await
            .map_err(|error| OmonError::Config(format!("lane status rename: {error}")))
    }
}

#[async_trait]
impl LaneLifecycle for SlackLaneThreads {
    async fn bot_id(&self) -> std::result::Result<String, DiscordThreadError> {
        Ok(self.bot_user_id().await?)
    }

    async fn status(
        &self,
        thread: &str,
        status: ThreadStatus,
    ) -> std::result::Result<(), DiscordThreadError> {
        Ok(self.set_status(thread, status, None).await?)
    }

    /// Slack has no archive: closing posts a final reply, reopening needs nothing
    /// beyond the 🔄 starter the status call already wrote.
    async fn archived(
        &self,
        thread: &str,
        archived: bool,
    ) -> std::result::Result<(), DiscordThreadError> {
        if !archived {
            return Ok(());
        }
        let (channel, ts) = parse_ref(thread).ok_or_else(|| invalid_ref(thread))?;
        self.api
            .post_message(channel, CLOSED_REPLY, Some(ts), None)
            .await?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct LaneTransports {
    pub discord: Option<Arc<DiscordThreads>>,
    pub slack: Option<Arc<SlackLaneThreads>>,
}

impl LaneTransports {
    fn slack(&self) -> Result<&SlackLaneThreads> {
        self.slack
            .as_deref()
            .ok_or_else(|| OmonError::Config("Slack lane transport unavailable".into()))
    }

    fn discord(&self) -> Result<&DiscordThreads> {
        self.discord
            .as_deref()
            .ok_or_else(|| OmonError::Config("Discord lane transport unavailable".into()))
    }

    fn lifecycle(&self, thread: &str) -> Result<&dyn LaneLifecycle> {
        let transport: &dyn LaneLifecycle = if is_slack_ref(thread) {
            self.slack()?
        } else {
            self.discord()?
        };
        Ok(transport)
    }

    fn sink(&self, thread: &str) -> Result<&dyn PostSink> {
        let sink: &dyn PostSink = if is_slack_ref(thread) {
            self.slack()?
        } else {
            self.discord()?
        };
        Ok(sink)
    }
}

#[async_trait]
impl PostSink for LaneTransports {
    async fn send(&self, post: &Post) -> Result<String> {
        self.sink(post.thread.as_deref().unwrap_or_default())?
            .send(post)
            .await
    }

    async fn edit_status(&self, thread: &str, message: &str, body: &str) -> Result<bool> {
        self.sink(thread)?.edit_status(thread, message, body).await
    }

    async fn rename_status(&self, thread: &str, status: ThreadStatus, title: &str) -> Result<()> {
        self.sink(thread)?
            .rename_status(thread, status, title)
            .await
    }
}

#[async_trait]
impl LaneLifecycle for LaneTransports {
    async fn bot_id(&self) -> std::result::Result<String, DiscordThreadError> {
        match (&self.discord, &self.slack) {
            (Some(discord), _) => discord.bot_id().await,
            (None, Some(slack)) => slack.bot_id().await,
            (None, None) => {
                Err(OmonError::Config("lane lifecycle transport unavailable".into()).into())
            }
        }
    }

    async fn bot_id_for(&self, thread: &str) -> std::result::Result<String, DiscordThreadError> {
        self.lifecycle(thread)?.bot_id().await
    }

    async fn status(
        &self,
        thread: &str,
        status: ThreadStatus,
    ) -> std::result::Result<(), DiscordThreadError> {
        self.lifecycle(thread)?.status(thread, status).await
    }

    async fn archived(
        &self,
        thread: &str,
        archived: bool,
    ) -> std::result::Result<(), DiscordThreadError> {
        self.lifecycle(thread)?.archived(thread, archived).await
    }
}

/// Records an owner reply posted in a Slack lane thread. True means the event was
/// consumed by the lane (reopen/close) and must not be routed further.
#[allow(clippy::too_many_arguments)]
pub async fn intercept_owner_reply(
    pool: &SqlitePool,
    channel: &str,
    thread_ts: &str,
    bot_user_id: &str,
    user: &str,
    owner: &str,
    ts: &str,
    text: &str,
) -> Result<bool> {
    let thread = thread_ref(channel, thread_ts);
    if crate::storage::get_lane_by_thread(pool, &thread)
        .await?
        .is_none()
    {
        return Ok(false);
    }
    let consumed = crate::storage::lanes::record_owner_reply(
        pool,
        &thread,
        bot_user_id,
        user,
        owner,
        &thread_ref(channel, ts),
        text,
    )
    .await?;
    if consumed {
        if let Err(error) = super::runner::wake(pool).await {
            tracing::warn!(%thread, %error, "lane wake after Slack owner reply failed");
        }
    }
    Ok(consumed)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::extract::Request;
    use chrono::Utc;
    use parking_lot::Mutex;
    use serde_json::{json, Value};

    use super::*;
    use crate::storage::{insert_lane, Database, Lane};

    type Responder = dyn Fn(&str, &Value) -> Value + Send + Sync;

    struct Fake {
        base: String,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
    }

    impl Fake {
        fn threads(&self) -> SlackLaneThreads {
            SlackLaneThreads::new(
                SlackApi::with_base_url("xoxb-test".into(), format!("{}/api", self.base)),
                "UBOT".into(),
            )
        }

        fn calls(&self) -> Vec<(String, Value)> {
            self.requests.lock().clone()
        }

        fn methods(&self) -> Vec<String> {
            self.calls().into_iter().map(|(method, _)| method).collect()
        }
    }

    async fn fake(responder: impl Fn(&str, &Value) -> Value + Send + Sync + 'static) -> Fake {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responder: Arc<Responder> = Arc::new(responder);
        let state = (requests.clone(), responder);
        let app = axum::Router::new().fallback(move |req: Request| {
            let (requests, responder) = state.clone();
            async move {
                let method = req.uri().path().trim_start_matches("/api/").to_owned();
                let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                requests.lock().push((method.clone(), body.clone()));
                let mut reply = responder(&method, &body);
                if reply.get("ok").is_none() {
                    reply["ok"] = json!(true);
                }
                axum::http::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(reply.to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake { base, requests }
    }

    #[test]
    fn refs_and_parents_parse_only_slack_shapes() {
        let thread = thread_ref("C0123", "1700000000.000100");
        assert_eq!(thread, "slack:C0123:1700000000.000100");
        assert_eq!(parse_ref(&thread), Some(("C0123", "1700000000.000100")));
        assert_eq!(parse_ref("slack:c01:1.2"), None);
        assert_eq!(parse_ref("slack:C01:"), None);
        assert_eq!(parse_ref("12345"), None);
        assert_eq!(parse_parent("slack:C0123"), Some("C0123"));
        assert_eq!(parse_parent("G9ABC"), Some("G9ABC"));
        assert_eq!(parse_parent("1474231559895646240"), None);
        assert!(valid_thread_ref("1474231559895646240"));
        assert!(valid_thread_ref(&thread));
        assert!(!valid_thread_ref("0"));
        assert!(!valid_thread_ref("slack:C01"));
    }

    #[test]
    fn status_prefix_is_replaced_in_unicode_and_shortcode_form() {
        assert_eq!(strip_slack_status("🔄 lane k: t"), "lane k: t");
        assert_eq!(
            strip_slack_status(":white_check_mark: lane k: t"),
            "lane k: t"
        );
        assert_eq!(strip_slack_status("⏸\u{fe0f} lane k: t"), "lane k: t");
        assert_eq!(
            starter_text(ThreadStatus::Working, "k", "a <b> & c"),
            "🔄 lane k: a &lt;b&gt; &amp; c"
        );
    }

    #[tokio::test]
    async fn create_posts_starter_to_parent_and_returns_root_ref() {
        let server = fake(|_, _| json!({ "ts": "1700.0001" })).await;
        let thread = server
            .threads()
            .create_thread("slack:C42", "fix-x", "Fix X")
            .await
            .unwrap();
        assert_eq!(thread, "slack:C42:1700.0001");
        let calls = server.calls();
        assert_eq!(calls[0].0, "chat.postMessage");
        assert_eq!(calls[0].1["channel"], "C42");
        assert_eq!(calls[0].1["text"], "🔄 lane fix-x: Fix X");
        assert!(calls[0].1.get("thread_ts").is_none());
    }

    #[tokio::test]
    async fn status_rewrites_starter_emoji_and_swaps_reaction() {
        let server = fake(|method, _| match method {
            "conversations.replies" => json!({ "messages": [{ "ts": "1700.0001", "text": ":arrows_counterclockwise: lane fix-x: Fix X" }] }),
            _ => json!({}),
        })
        .await;
        LaneLifecycle::status(&server.threads(), "slack:C42:1700.0001", ThreadStatus::Done)
            .await
            .unwrap();
        let calls = server.calls();
        let update = calls
            .iter()
            .find(|(method, _)| method == "chat.update")
            .unwrap();
        assert_eq!(update.1["ts"], "1700.0001");
        assert_eq!(update.1["text"], "✅ lane fix-x: Fix X");
        let removed: Vec<_> = calls
            .iter()
            .filter(|(method, _)| method == "reactions.remove")
            .map(|(_, body)| body["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(removed, ["arrows_counterclockwise", "double_vertical_bar"]);
        let added = calls
            .iter()
            .find(|(method, _)| method == "reactions.add")
            .unwrap();
        assert_eq!(added.1["name"], "white_check_mark");
        assert_eq!(added.1["timestamp"], "1700.0001");
    }

    #[tokio::test]
    async fn close_posts_final_reply_and_reopen_is_a_no_op() {
        let server = fake(|_, _| json!({ "ts": "1700.0009" })).await;
        let threads = server.threads();
        threads.archived("slack:C42:1700.0001", true).await.unwrap();
        threads
            .archived("slack:C42:1700.0001", false)
            .await
            .unwrap();
        let calls = server.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1["thread_ts"], "1700.0001");
        assert_eq!(calls[0].1["text"], CLOSED_REPLY);
        assert_eq!(LaneLifecycle::bot_id(&threads).await.unwrap(), "UBOT");
    }

    #[tokio::test]
    async fn lazy_bot_id_comes_from_auth_test_once() {
        let server = fake(|_, _| json!({ "user_id": "UAUTH" })).await;
        let threads = SlackLaneThreads::lazy(SlackApi::with_base_url(
            "xoxb-test".into(),
            format!("{}/api", server.base),
        ));
        assert_eq!(threads.bot_user_id().await.unwrap(), "UAUTH");
        assert_eq!(threads.bot_user_id().await.unwrap(), "UAUTH");
        assert_eq!(server.methods(), ["auth.test"]);
    }

    #[tokio::test]
    async fn post_is_threaded_converted_and_never_pings() {
        let server = fake(|_, _| json!({ "ts": "1700.0002" })).await;
        let post = Post {
            source: "s".into(),
            thread: Some("slack:C42:1700.0001".into()),
            body: "**done** <!channel> <@U1>".into(),
            nonce: "n".into(),
            state: super::super::posts::PostState::Pending,
            message_id: None,
            error: None,
        };
        assert_eq!(server.threads().send(&post).await.unwrap(), "1700.0002");
        let calls = server.calls();
        assert_eq!(calls[0].1["thread_ts"], "1700.0001");
        let text = calls[0].1["text"].as_str().unwrap();
        assert!(text.starts_with("*done* "), "{text}");
        assert!(!text.contains("<!") && !text.contains("<@"), "{text}");
    }

    #[tokio::test]
    async fn status_edit_maps_message_not_found_to_false() {
        let server = fake(|_, body| {
            if body["ts"] == "gone" {
                json!({ "ok": false, "error": "message_not_found" })
            } else if body["ts"] == "bad" {
                json!({ "ok": false, "error": "cant_update_message" })
            } else {
                json!({})
            }
        })
        .await;
        let threads = server.threads();
        let thread = "slack:C42:1700.0001";
        assert!(threads
            .edit_status(thread, "1700.0003", "⏳ x")
            .await
            .unwrap());
        assert!(!threads.edit_status(thread, "gone", "⏳ x").await.unwrap());
        assert!(threads.edit_status(thread, "bad", "⏳ x").await.is_err());
        assert_eq!(server.calls()[0].1["channel"], "C42");
    }

    #[tokio::test]
    async fn delete_thread_removes_the_starter() {
        let server = fake(|_, _| json!({})).await;
        server
            .threads()
            .delete_thread("slack:C42:1700.0001")
            .await
            .unwrap();
        let calls = server.calls();
        assert_eq!(calls[0].0, "chat.delete");
        assert_eq!(calls[0].1["ts"], "1700.0001");
    }

    #[tokio::test]
    async fn router_sends_slack_refs_to_slack_and_refuses_missing_discord() {
        let server = fake(|_, _| json!({ "ts": "1700.0004" })).await;
        let router = LaneTransports {
            discord: None,
            slack: Some(Arc::new(server.threads())),
        };
        assert_eq!(
            router.bot_id_for("slack:C42:1700.0001").await.unwrap(),
            "UBOT"
        );
        assert!(router.bot_id_for("1474231559895646240").await.is_err());
        assert!(router.status("123", ThreadStatus::Done).await.is_err());
        assert!(server.calls().is_empty());
    }

    async fn slack_lane(status: &str) -> Database {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let mut lane = Lane::new(
            "sl",
            "Slack lane",
            "lead",
            "/tmp",
            "/tmp/brief",
            "lane",
            "file x",
        );
        lane.status = status.into();
        lane.thread_channel_id = Some(thread_ref("C42", "1700.0001"));
        insert_lane(db.pool(), &lane).await.unwrap();
        crate::storage::lanes::bind_lane_thread(db.pool(), "sl", "UBOT")
            .await
            .unwrap();
        db
    }

    #[tokio::test]
    async fn drain_delivers_queued_post_into_the_slack_thread() {
        let db = slack_lane("working").await;
        let lane = crate::storage::get_lane(db.pool(), "sl")
            .await
            .unwrap()
            .unwrap();
        let mut tx = db.pool().begin().await.unwrap();
        super::super::posts::enqueue(&mut tx, &lane, "milestone:1", "half way", Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let server = fake(|_, _| json!({ "ts": "1700.0005" })).await;
        assert_eq!(
            super::super::posts::drain(db.pool(), &server.threads())
                .await
                .unwrap(),
            1
        );
        let calls = server.calls();
        assert_eq!(calls[0].1["channel"], "C42");
        assert_eq!(calls[0].1["thread_ts"], "1700.0001");
        assert_eq!(calls[0].1["text"], "half way");
    }

    #[tokio::test]
    async fn owner_reply_in_a_closed_lane_thread_is_consumed() {
        let db = slack_lane("closed").await;
        let pool = db.pool();
        assert!(!intercept_owner_reply(
            pool,
            "C42",
            "1700.0001",
            "UBOT",
            "UOTHER",
            "UOWNER",
            "1700.0006",
            "again"
        )
        .await
        .unwrap());
        assert!(!intercept_owner_reply(
            pool,
            "C42",
            "9999.0001",
            "UBOT",
            "UOWNER",
            "UOWNER",
            "1700.0007",
            "again"
        )
        .await
        .unwrap());
        assert!(intercept_owner_reply(
            pool,
            "C42",
            "1700.0001",
            "UBOT",
            "UOWNER",
            "UOWNER",
            "1700.0008",
            "again"
        )
        .await
        .unwrap());
        let (message, lane, text): (String, String, String) =
            sqlx::query_as("SELECT message_id, lane_key, text FROM lane_owner_replies")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(
            (message.as_str(), lane.as_str(), text.as_str()),
            ("slack:C42:1700.0008", "sl", "again")
        );
    }
}

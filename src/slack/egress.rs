use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::time::Instant;
use uuid::Uuid;

use super::api::SlackApi;
use super::mrkdwn::{split_message, to_mrkdwn};
use super::MESSAGE_LIMIT;
use crate::models::{filter_reasoning, is_explicit_silence};
use crate::{OutboundAction, OutboundDispatcher, Result, SessionKey, StreamChunk};

const STREAM_PLACEHOLDER: &str = "…";
const STREAM_UPDATE_INTERVAL: Duration = Duration::from_millis(1200);
/// How long a finished stream id is remembered so a retried final chunk does not repost.
const COMPLETED_RETENTION: Duration = Duration::from_secs(15 * 60);
const APPROVAL_COMMAND_LIMIT: usize = 2800;
const APPROVAL_REASON_LIMIT: usize = 300;
const APPROVAL_EXPIRED_TEXT: &str = "⌛ Approval expired";
const STATUS_REACTIONS: [&str; 3] = ["eyes", "white_check_mark", "x"];

type StreamKey = (String, Uuid);

#[derive(Default)]
struct StreamState {
    ts: Option<String>,
    last_sequence: Option<u64>,
    last_update: Option<Instant>,
    final_sent: usize,
}

pub struct SlackEgress {
    api: SlackApi,
    streams: Mutex<HashMap<StreamKey, Arc<tokio::sync::Mutex<StreamState>>>>,
    completed: Mutex<HashMap<StreamKey, Instant>>,
    approvals: Mutex<HashMap<Uuid, (String, String)>>,
    update_interval: Duration,
}

impl SlackEgress {
    pub fn new(api: SlackApi) -> Self {
        Self {
            api,
            streams: Mutex::new(HashMap::new()),
            completed: Mutex::new(HashMap::new()),
            approvals: Mutex::new(HashMap::new()),
            update_interval: STREAM_UPDATE_INTERVAL,
        }
    }

    #[cfg(test)]
    fn with_update_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    /// Replaces the approval buttons with the decision once a button was handled.
    pub async fn mark_approval_decided(&self, request_id: Uuid, label: &str) -> Result<()> {
        let Some((channel, ts)) = self.approvals.lock().remove(&request_id) else {
            return Ok(());
        };
        self.api
            .update_message(&channel, &ts, label, Some(text_blocks(label)))
            .await
    }

    async fn send_message(&self, session: &SessionKey, content: &str) -> Result<()> {
        let filtered = filter_reasoning(content);
        if filtered.is_empty() || is_explicit_silence(&filtered) {
            return Ok(());
        }
        let thread = session.thread_id.as_deref();
        for chunk in render(&filtered) {
            self.api
                .post_message(&session.channel_id, &chunk, thread, None)
                .await?;
        }
        Ok(())
    }

    async fn edit_message(&self, session: &SessionKey, ts: &str, content: &str) -> Result<()> {
        let filtered = filter_reasoning(content);
        if filtered.is_empty() || is_explicit_silence(&filtered) {
            return Ok(());
        }
        let Some(text) = render(&filtered).into_iter().next() else {
            return Ok(());
        };
        self.api
            .update_message(&session.channel_id, ts, &text, None)
            .await
    }

    async fn stream(&self, session: &SessionKey, chunk: StreamChunk) -> Result<()> {
        let key = (session.storage_key(), chunk.stream_id);
        if self.completed.lock().contains_key(&key) {
            return Ok(());
        }
        let cell = self.streams.lock().entry(key.clone()).or_default().clone();
        let mut state = cell.lock().await;
        if self.completed.lock().contains_key(&key) {
            return Ok(());
        }
        let channel = session.channel_id.as_str();
        let thread = session.thread_id.as_deref();
        let filtered = filter_reasoning(&chunk.content);

        if !chunk.is_final {
            if state
                .last_sequence
                .is_some_and(|sequence| chunk.sequence <= sequence)
            {
                return Ok(());
            }
            state.last_sequence = Some(chunk.sequence);
            let Some(ts) = state.ts.clone() else {
                let ts = self
                    .api
                    .post_message(channel, STREAM_PLACEHOLDER, thread, None)
                    .await?;
                state.ts = Some(ts);
                state.last_update = Some(Instant::now());
                return Ok(());
            };
            let due = state
                .last_update
                .is_none_or(|at| at.elapsed() >= self.update_interval);
            if !due || filtered.is_empty() || is_explicit_silence(&filtered) {
                return Ok(());
            }
            let Some(text) = render(&filtered).into_iter().next() else {
                return Ok(());
            };
            state.last_update = Some(Instant::now());
            if let Err(error) = self.api.update_message(channel, &ts, &text, None).await {
                tracing::debug!(%error, channel, "failed to update slack stream preview");
            }
            return Ok(());
        }

        let chunks = if is_explicit_silence(&filtered) {
            Vec::new()
        } else {
            render(&filtered)
        };
        if chunks.is_empty() {
            if let Some(ts) = &state.ts {
                self.api.delete_message(channel, ts).await?;
            }
        }
        for (index, text) in chunks.iter().enumerate().skip(state.final_sent) {
            match (index, state.ts.clone()) {
                (0, Some(ts)) => self.api.update_message(channel, &ts, text, None).await?,
                (0, None) => {
                    let ts = self.api.post_message(channel, text, thread, None).await?;
                    state.ts = Some(ts);
                }
                _ => {
                    self.api.post_message(channel, text, thread, None).await?;
                }
            }
            state.final_sent = index + 1;
        }
        drop(state);
        self.finish_stream(key);
        Ok(())
    }

    fn finish_stream(&self, key: StreamKey) {
        self.streams.lock().remove(&key);
        let mut completed = self.completed.lock();
        completed.retain(|_, at| at.elapsed() < COMPLETED_RETENTION);
        completed.insert(key, Instant::now());
    }

    async fn react(
        &self,
        session: &SessionKey,
        ts: &str,
        emoji: &str,
        remove_others: bool,
    ) -> Result<()> {
        let Some(name) = reaction_name(emoji) else {
            tracing::debug!(emoji, "no slack reaction name for emoji");
            return Ok(());
        };
        let channel = session.channel_id.as_str();
        if remove_others {
            for other in STATUS_REACTIONS.iter().filter(|other| **other != name) {
                if let Err(error) = self.api.remove_reaction(channel, ts, other).await {
                    tracing::debug!(%error, channel, reaction = other, "failed to remove slack reaction");
                }
            }
        }
        if let Err(error) = self.api.add_reaction(channel, ts, &name).await {
            tracing::debug!(%error, channel, reaction = %name, "failed to add slack reaction");
        }
        Ok(())
    }

    async fn request_approval(
        &self,
        session: &SessionKey,
        request_id: Uuid,
        command: &str,
        reason: &str,
    ) -> Result<()> {
        let fallback = format!(
            "Approval needed: {}",
            truncate_chars(reason, APPROVAL_REASON_LIMIT)
        );
        let ts = self
            .api
            .post_message(
                &session.channel_id,
                &fallback,
                session.thread_id.as_deref(),
                Some(approval_blocks(request_id, command, reason)),
            )
            .await?;
        self.approvals
            .lock()
            .insert(request_id, (session.channel_id.clone(), ts));
        Ok(())
    }

    async fn expire_approval(&self, request_id: Uuid) -> Result<()> {
        let Some((channel, ts)) = self.approvals.lock().remove(&request_id) else {
            return Ok(());
        };
        self.api
            .update_message(
                &channel,
                &ts,
                APPROVAL_EXPIRED_TEXT,
                Some(text_blocks(APPROVAL_EXPIRED_TEXT)),
            )
            .await
    }

    async fn upload(&self, session: &SessionKey, path: &Path) -> Result<()> {
        self.api
            .upload_file(
                &session.channel_id,
                session.thread_id.as_deref(),
                path,
                None,
            )
            .await
            .map(drop)
    }
}

#[async_trait]
impl OutboundDispatcher for SlackEgress {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action {
            OutboundAction::SendMessage {
                session, content, ..
            } => self.send_message(&session, &content).await,
            OutboundAction::EditMessage {
                session,
                platform_message_id,
                content,
            } => {
                self.edit_message(&session, &platform_message_id, &content)
                    .await
            }
            OutboundAction::DeleteMessage {
                session,
                platform_message_id,
            } => {
                self.api
                    .delete_message(&session.channel_id, &platform_message_id)
                    .await
            }
            OutboundAction::UploadFile { session, path } => self.upload(&session, &path).await,
            OutboundAction::Stream { session, chunk } => self.stream(&session, chunk).await,
            OutboundAction::Typing { .. } => Ok(()),
            OutboundAction::React {
                session,
                message_id,
                emoji,
                remove_others,
            } => {
                self.react(&session, &message_id, &emoji, remove_others)
                    .await
            }
            OutboundAction::ApprovalRequest {
                session,
                request_id,
                command,
                reason,
            } => {
                self.request_approval(&session, request_id, &command, &reason)
                    .await
            }
            OutboundAction::ExpireApproval { request_id } => self.expire_approval(request_id).await,
        }
    }
}

fn render(text: &str) -> Vec<String> {
    split_message(&to_mrkdwn(text), MESSAGE_LIMIT)
        .into_iter()
        .filter(|chunk| !chunk.trim().is_empty())
        .collect()
}

fn reaction_name(emoji: &str) -> Option<String> {
    let emoji = emoji.trim().trim_end_matches('\u{fe0f}');
    let name = match emoji {
        "👀" => "eyes",
        "✅" => "white_check_mark",
        "❌" => "x",
        "⏳" => "hourglass_flowing_sand",
        other => {
            let name = other.strip_prefix(':')?.strip_suffix(':')?;
            let valid = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '\''));
            return valid.then(|| name.to_owned());
        }
    };
    Some(name.to_owned())
}

fn approval_blocks(request_id: Uuid, command: &str, reason: &str) -> Value {
    let reason = escape(&truncate_chars(reason, APPROVAL_REASON_LIMIT));
    let command =
        escape(&truncate_chars(command, APPROVAL_COMMAND_LIMIT)).replace("```", "`\u{200b}``");
    let button = |label: &str, choice: &str, style: Option<&str>| {
        let id = format!("omon:approval:{request_id}:{choice}");
        let mut button = json!({
            "type": "button",
            "text": { "type": "plain_text", "text": label },
            "action_id": id,
            "value": id,
        });
        if let Some(style) = style {
            button["style"] = json!(style);
        }
        button
    };
    json!([
        {
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("*Approval needed*\n{reason}") },
        },
        {
            "type": "section",
            "text": { "type": "mrkdwn", "text": format!("```\n{command}\n```") },
        },
        {
            "type": "actions",
            "block_id": format!("omon:approval:{request_id}"),
            "elements": [
                button("Allow Once", "once", Some("primary")),
                button("Allow Session", "session", Some("primary")),
                button("Always Allow", "always", None),
                button("Deny", "deny", Some("danger")),
            ],
        },
    ])
}

fn text_blocks(text: &str) -> Value {
    json!([{ "type": "section", "text": { "type": "mrkdwn", "text": escape(text) } }])
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(limit.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;

    struct Fake {
        base: String,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
    }

    impl Fake {
        fn requests(&self) -> Vec<(String, Value)> {
            self.requests.lock().clone()
        }

        fn paths(&self) -> Vec<String> {
            self.requests().into_iter().map(|(path, _)| path).collect()
        }

        fn egress(&self) -> SlackEgress {
            SlackEgress::new(SlackApi::with_base_url(
                "xoxb-test".into(),
                self.base.clone(),
            ))
        }
    }

    async fn fake() -> Fake {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<(String, Value)>::new()));
        let recorded = requests.clone();
        let app = axum::Router::new().fallback(move |req: axum::extract::Request| {
            let recorded = recorded.clone();
            async move {
                let path = req.uri().path().trim_start_matches('/').to_owned();
                let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
                let posts = {
                    let mut requests = recorded.lock();
                    requests.push((path.clone(), body));
                    requests
                        .iter()
                        .filter(|(p, _)| p == "chat.postMessage")
                        .count()
                };
                let reply = if path == "chat.postMessage" {
                    json!({ "ok": true, "ts": format!("100.{posts:04}") })
                } else {
                    json!({ "ok": true })
                };
                axum::Json(reply)
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake { base, requests }
    }

    fn session() -> SessionKey {
        SessionKey::new("slack", Some("T1"), "C1", Some("1699.0001"), "")
    }

    fn stream(stream_id: Uuid, sequence: u64, content: &str, is_final: bool) -> OutboundAction {
        OutboundAction::Stream {
            session: session(),
            chunk: StreamChunk {
                stream_id,
                sequence,
                content: content.into(),
                is_final,
                reply_to: Some("1699.0001".into()),
            },
        }
    }

    fn long_reply() -> String {
        (0..700)
            .map(|line| format!("line {line:04} of the reply"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[tokio::test]
    async fn send_message_posts_converted_text_in_thread_and_skips_silence() {
        let server = fake().await;
        let egress = server.egress();
        let content = "<think>plan</think>**Done** & see [docs](https://example.com)";

        egress
            .dispatch(OutboundAction::SendMessage {
                session: session(),
                content: content.into(),
                reply_to: Some("1699.0002".into()),
            })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::SendMessage {
                session: session(),
                content: "NO_REPLY".into(),
                reply_to: None,
            })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let (path, body) = &requests[0];
        assert_eq!(path, "chat.postMessage");
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["thread_ts"], "1699.0001");
        let expected = to_mrkdwn(&filter_reasoning(content));
        assert_eq!(body["text"], expected.as_str());
        assert!(!expected.contains("plan"));
    }

    #[tokio::test]
    async fn stream_posts_placeholder_updates_and_finishes_with_overflow() {
        let server = fake().await;
        let egress = server.egress().with_update_interval(Duration::ZERO);
        let id = Uuid::new_v4();
        let reply = long_reply();
        let expected = render(&reply);
        assert!(expected.len() >= 2);

        egress.dispatch(stream(id, 0, "Hel", false)).await.unwrap();
        egress
            .dispatch(stream(id, 1, "Hello **world**", false))
            .await
            .unwrap();
        egress
            .dispatch(stream(id, 1, "stale", false))
            .await
            .unwrap();
        egress.dispatch(stream(id, 2, &reply, true)).await.unwrap();
        egress.dispatch(stream(id, 2, &reply, true)).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2 + expected.len());
        assert_eq!(requests[0].0, "chat.postMessage");
        assert_eq!(requests[0].1["text"], STREAM_PLACEHOLDER);
        assert_eq!(requests[0].1["thread_ts"], "1699.0001");
        assert_eq!(requests[1].0, "chat.update");
        assert_eq!(requests[1].1["ts"], "100.0001");
        assert_eq!(requests[1].1["text"], to_mrkdwn("Hello **world**").as_str());
        assert_eq!(requests[2].0, "chat.update");
        assert_eq!(requests[2].1["ts"], "100.0001");
        assert_eq!(requests[2].1["text"], expected[0].as_str());
        for (request, chunk) in requests[3..].iter().zip(&expected[1..]) {
            assert_eq!(request.0, "chat.postMessage");
            assert_eq!(request.1["text"], chunk.as_str());
            assert_eq!(request.1["thread_ts"], "1699.0001");
        }
        assert!(egress.streams.lock().is_empty());
    }

    #[tokio::test]
    async fn stream_preview_updates_are_throttled() {
        let server = fake().await;
        let egress = server
            .egress()
            .with_update_interval(Duration::from_secs(3600));
        let id = Uuid::new_v4();

        egress.dispatch(stream(id, 0, "a", false)).await.unwrap();
        egress.dispatch(stream(id, 1, "ab", false)).await.unwrap();
        egress.dispatch(stream(id, 2, "abc", false)).await.unwrap();
        egress.dispatch(stream(id, 3, "abcd", true)).await.unwrap();

        assert_eq!(server.paths(), ["chat.postMessage", "chat.update"]);
        assert_eq!(server.requests()[1].1["text"], to_mrkdwn("abcd").as_str());
    }

    #[tokio::test]
    async fn silent_final_deletes_placeholder() {
        let server = fake().await;
        let egress = server.egress();
        let id = Uuid::new_v4();

        egress.dispatch(stream(id, 0, "NO_", false)).await.unwrap();
        egress
            .dispatch(stream(id, 1, "NO_REPLY", true))
            .await
            .unwrap();
        egress
            .dispatch(stream(Uuid::new_v4(), 0, "[SILENT]", true))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(server.paths(), ["chat.postMessage", "chat.delete"]);
        assert_eq!(requests[1].1["channel"], "C1");
        assert_eq!(requests[1].1["ts"], "100.0001");
    }

    #[tokio::test]
    async fn approval_posts_buttons_then_expires_once() {
        let server = fake().await;
        let egress = server.egress();
        let request_id = Uuid::new_v4();

        egress
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id,
                command: "rm -rf <dir> && echo ```".into(),
                reason: "deletes files".into(),
            })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let (path, body) = &requests[0];
        assert_eq!(path, "chat.postMessage");
        assert_eq!(body["thread_ts"], "1699.0001");
        let blocks = body["blocks"].as_array().unwrap();
        assert!(blocks[0]["text"]["text"]
            .as_str()
            .unwrap()
            .starts_with("*Approval needed*\ndeletes files"));
        let code = blocks[1]["text"]["text"].as_str().unwrap();
        assert!(code.starts_with("```\nrm -rf &lt;dir&gt; &amp;&amp; echo "));
        assert_eq!(code.matches("```").count(), 2);
        let buttons = blocks[2]["elements"].as_array().unwrap();
        let ids: Vec<&str> = buttons
            .iter()
            .map(|b| b["value"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            ["once", "session", "always", "deny"]
                .map(|choice| format!("omon:approval:{request_id}:{choice}"))
        );
        for button in buttons {
            assert_eq!(button["action_id"], button["value"]);
        }
        let styles: Vec<Option<&str>> = buttons.iter().map(|b| b["style"].as_str()).collect();
        assert_eq!(
            styles,
            [Some("primary"), Some("primary"), None, Some("danger")]
        );

        let (path, body) = &requests[1];
        assert_eq!(path, "chat.update");
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["ts"], "100.0001");
        assert_eq!(body["text"], APPROVAL_EXPIRED_TEXT);
        assert!(body["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|block| block["type"] != "actions"));
    }

    #[test]
    fn approval_command_is_truncated() {
        let blocks = approval_blocks(Uuid::new_v4(), &"a".repeat(5000), "why");
        let code = blocks[1]["text"]["text"].as_str().unwrap();
        assert!(code.chars().count() <= APPROVAL_COMMAND_LIMIT + 8);
        assert!(code.contains('…'));
    }

    #[tokio::test]
    async fn mark_approval_decided_replaces_buttons_with_label() {
        let server = fake().await;
        let egress = server.egress();
        let request_id = Uuid::new_v4();
        egress
            .dispatch(OutboundAction::ApprovalRequest {
                session: session(),
                request_id,
                command: "ls".into(),
                reason: "list".into(),
            })
            .await
            .unwrap();

        egress
            .mark_approval_decided(request_id, "✅ Allowed once by <@U1>")
            .await
            .unwrap();
        egress
            .dispatch(OutboundAction::ExpireApproval { request_id })
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].0, "chat.update");
        assert_eq!(requests[1].1["text"], "✅ Allowed once by <@U1>");
        assert_eq!(requests[1].1["blocks"][0]["type"], "section");
    }

    #[tokio::test]
    async fn react_maps_emoji_and_removes_other_status_reactions() {
        let server = fake().await;
        let egress = server.egress();
        let react = |emoji: &str, remove_others| OutboundAction::React {
            session: session(),
            message_id: "1699.0005".into(),
            emoji: emoji.into(),
            remove_others,
        };

        egress.dispatch(react("✅", true)).await.unwrap();
        egress.dispatch(react(":rocket:", false)).await.unwrap();
        egress.dispatch(react("🦀", true)).await.unwrap();

        let requests = server.requests();
        let calls: Vec<(&str, &str)> = requests
            .iter()
            .map(|(path, body)| (path.as_str(), body["name"].as_str().unwrap()))
            .collect();
        assert_eq!(
            calls,
            [
                ("reactions.remove", "eyes"),
                ("reactions.remove", "x"),
                ("reactions.add", "white_check_mark"),
                ("reactions.add", "rocket"),
            ]
        );
        assert!(requests
            .iter()
            .all(|(_, body)| body["channel"] == "C1" && body["timestamp"] == "1699.0005"));
    }

    #[test]
    fn reaction_names() {
        assert_eq!(reaction_name("👀").as_deref(), Some("eyes"));
        assert_eq!(reaction_name("❌").as_deref(), Some("x"));
        assert_eq!(
            reaction_name("⏳").as_deref(),
            Some("hourglass_flowing_sand")
        );
        assert_eq!(
            reaction_name("✅\u{fe0f}").as_deref(),
            Some("white_check_mark")
        );
        assert_eq!(reaction_name(":thumbsup:").as_deref(), Some("thumbsup"));
        assert_eq!(reaction_name("::"), None);
        assert_eq!(reaction_name("🦀"), None);
    }
}

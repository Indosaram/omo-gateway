//! Slack side of the generic `omon:btn:<namespace>:<action>:<arg>` buttons. Parsing, the
//! button-actions file and command execution are shared with Discord. Block Kit buttons cannot
//! be disabled, so a locked button is removed and its label shown in a context block instead.

use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::task::JoinHandle;

use super::interactive::payload_channel;
use super::{SlackApi, SlackConfig};
use crate::discord::buttons::{
    button_actions_path, load_button_action, parse_button_custom_id, reply_text, run_button_action,
    ActionOutcome, BUTTON_PREFIX,
};
use crate::Result;

const PENDING_LABEL: &str = "⏳ 처리 중…";
const DEFAULT_DONE_LABEL: &str = "✓ 완료";
const EXECUTED_ACTION_LIMIT: usize = 500;
const NOT_AUTHORIZED: &str = "이 버튼을 누를 권한이 없습니다.";
const ALREADY_RUNNING: &str = "이미 처리 중입니다.";
const CONFIG_UNREADABLE: &str = "버튼 설정 파일을 읽지 못했습니다.";

static IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static EXECUTED_ACTIONS: LazyLock<Mutex<ActionLru>> =
    LazyLock::new(|| Mutex::new(ActionLru::default()));

#[derive(Default)]
struct ActionLru {
    ids: HashSet<String>,
    order: VecDeque<String>,
}

impl ActionLru {
    fn contains(&mut self, id: &str) -> bool {
        if !self.ids.contains(id) {
            return false;
        }
        self.order.retain(|existing| existing != id);
        self.order.push_back(id.to_owned());
        true
    }

    fn insert(&mut self, id: String) {
        if self.ids.insert(id.clone()) {
            self.order.push_back(id);
            if self.order.len() > EXECUTED_ACTION_LIMIT {
                if let Some(oldest) = self.order.pop_front() {
                    self.ids.remove(&oldest);
                }
            }
        }
    }
}

struct ButtonMessage {
    api: SlackApi,
    channel: String,
    ts: String,
    thread_ts: String,
    text: String,
    blocks: Value,
}

impl ButtonMessage {
    async fn set_blocks(&self, blocks: Option<Value>) {
        let Some(blocks) = blocks else { return };
        // A failed edit after a terminal action would leave the button clickable, so retry.
        for attempt in 1..=3u32 {
            match self
                .api
                .update_message(&self.channel, &self.ts, &self.text, Some(blocks.clone()))
                .await
            {
                Ok(()) => return,
                Err(error) => {
                    tracing::warn!(%error, channel = %self.channel, ts = %self.ts, attempt, "failed to update slack button state");
                    if attempt < 3 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
    }
}

/// Runs every `omon:btn:` action in a `block_actions` payload. Commands run in the background;
/// the result is posted as a threaded reply under the button message.
pub async fn handle_button_actions(
    payload: &Value,
    config: &SlackConfig,
    api: &SlackApi,
) -> Result<()> {
    start_button_actions(payload, config, api, &button_actions_path())
        .await
        .map(drop)
}

async fn start_button_actions(
    payload: &Value,
    config: &SlackConfig,
    api: &SlackApi,
    actions_path: &Path,
) -> Result<Vec<JoinHandle<()>>> {
    if payload.get("type").and_then(Value::as_str) != Some("block_actions") {
        return Ok(Vec::new());
    }
    let mut handles = Vec::new();
    for action in payload
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(value) = button_value(action) else {
            continue;
        };
        if let Some(handle) = start_click(payload, action, value, config, api, actions_path).await?
        {
            handles.push(handle);
        }
    }
    Ok(handles)
}

fn button_value(action: &Value) -> Option<&str> {
    ["value", "action_id"]
        .iter()
        .filter_map(|key| action.get(*key).and_then(Value::as_str))
        .find(|value| value.starts_with(BUTTON_PREFIX))
}

async fn start_click(
    payload: &Value,
    action: &Value,
    value: &str,
    config: &SlackConfig,
    api: &SlackApi,
    actions_path: &Path,
) -> Result<Option<JoinHandle<()>>> {
    let Some(click) = parse_button_custom_id(value) else {
        tracing::warn!(value, "ignoring malformed Slack button value");
        return Ok(None);
    };
    let user = payload
        .pointer("/user/id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let action_ts = action
        .get("action_ts")
        .or_else(|| payload.get("trigger_id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if !action_ts.is_empty() && EXECUTED_ACTIONS.lock().contains(&action_ts) {
        tracing::warn!(action_ts, value, "duplicate Slack button action ignored");
        return Ok(None);
    }
    let channel = payload_channel(payload);
    if !config.is_allowed(user) {
        tracing::warn!(user, value, "unauthorized Slack button click");
        if let Some(channel) = channel {
            api.post_ephemeral(channel, user, NOT_AUTHORIZED).await?;
        }
        return Ok(None);
    }
    let Some(channel) = channel else {
        tracing::warn!(value, "Slack button click without a channel");
        return Ok(None);
    };
    let Some(message_ts) = payload
        .pointer("/container/message_ts")
        .or_else(|| payload.pointer("/message/ts"))
        .and_then(Value::as_str)
    else {
        tracing::warn!(value, "Slack button click without a message ts");
        return Ok(None);
    };
    let action_config = match load_button_action(actions_path, &click.namespace) {
        Ok(Some(action_config)) => action_config,
        Ok(None) => {
            let content = format!("`{}` 버튼 동작이 설정되어 있지 않습니다.", click.namespace);
            api.post_ephemeral(channel, user, &content).await?;
            return Ok(None);
        }
        Err(error) => {
            tracing::warn!(%error, "button actions config is unreadable");
            api.post_ephemeral(channel, user, CONFIG_UNREADABLE).await?;
            return Ok(None);
        }
    };

    let lock = format!("{channel}:{message_ts}");
    if !IN_FLIGHT.lock().insert(lock.clone()) {
        api.post_ephemeral(channel, user, ALREADY_RUNNING).await?;
        return Ok(None);
    }
    if !action_ts.is_empty() {
        EXECUTED_ACTIONS.lock().insert(action_ts);
    }

    let message = ButtonMessage {
        api: api.clone(),
        channel: channel.to_owned(),
        ts: message_ts.to_owned(),
        thread_ts: payload
            .pointer("/message/thread_ts")
            .and_then(Value::as_str)
            .unwrap_or(message_ts)
            .to_owned(),
        text: payload
            .pointer("/message/text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        blocks: payload
            .pointer("/message/blocks")
            .cloned()
            .unwrap_or(Value::Null),
    };
    message
        .set_blocks(rebuild_blocks(
            &message.blocks,
            value,
            Some(PENDING_LABEL),
            true,
        ))
        .await;

    let extra_env = vec![
        ("OMON_BUTTON_USER_ID".to_owned(), user.to_owned()),
        ("OMON_BUTTON_CHANNEL_ID".to_owned(), message.channel.clone()),
        ("OMON_BUTTON_MESSAGE_ID".to_owned(), message.ts.clone()),
        (
            "OMON_BUTTON_PLATFORM".to_owned(),
            super::PLATFORM.to_owned(),
        ),
    ];
    tracing::info!(user, namespace = %click.namespace, action = %click.action, arg = %click.arg, "running slack button action");
    let clicked = value.to_owned();
    Ok(Some(tokio::spawn(async move {
        let started = Instant::now();
        let outcome = run_button_action(&action_config, &click, &extra_env)
            .await
            .unwrap_or_else(|error| ActionOutcome {
                success: false,
                code: None,
                stdout: String::new(),
                stderr: format!("failed to start command: {error}"),
                timed_out: false,
            });
        tracing::info!(
            namespace = %click.namespace,
            action = %click.action,
            success = outcome.success,
            code = ?outcome.code,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "slack button action finished"
        );
        let blocks = if outcome.success {
            let label = action_config
                .done_labels
                .get(&click.action)
                .map_or(DEFAULT_DONE_LABEL, String::as_str);
            rebuild_blocks(
                &message.blocks,
                &clicked,
                Some(label),
                !action_config.nonterminal_actions.contains(&click.action),
            )
        } else {
            message.blocks.as_array().map(|_| message.blocks.clone())
        };
        message.set_blocks(blocks).await;
        if let Err(error) = message
            .api
            .post_message(
                &message.channel,
                &escape(&reply_text(&outcome)),
                Some(&message.thread_ts),
                None,
            )
            .await
        {
            tracing::warn!(%error, channel = %message.channel, ts = %message.ts, "failed to post slack button action result");
        }
        IN_FLIGHT.lock().remove(&lock);
    })))
}

/// `locked` removes every action button (link buttons stay) and shows `label` in a context
/// block after the clicked button's row; otherwise the clicked button is relabelled in place.
/// `None` when the message carried no blocks.
fn rebuild_blocks(
    blocks: &Value,
    clicked: &str,
    label: Option<&str>,
    locked: bool,
) -> Option<Value> {
    let blocks = blocks.as_array()?;
    let mut out = Vec::with_capacity(blocks.len() + 1);
    for block in blocks {
        if block.get("type").and_then(Value::as_str) != Some("actions") {
            out.push(block.clone());
            continue;
        }
        let elements = block
            .get("elements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let holds_clicked = elements.iter().any(|element| is_clicked(element, clicked));
        let kept: Vec<Value> = elements
            .into_iter()
            .filter_map(|mut element| {
                let action_button = element.get("type").and_then(Value::as_str) == Some("button")
                    && element.get("url").is_none();
                if !action_button {
                    return Some(element);
                }
                if locked {
                    return None;
                }
                if let Some(label) = label.filter(|_| is_clicked(&element, clicked)) {
                    element["text"] = json!({ "type": "plain_text", "text": label, "emoji": true });
                }
                Some(element)
            })
            .collect();
        if !kept.is_empty() {
            let mut block = block.clone();
            block["elements"] = Value::Array(kept);
            out.push(block);
        }
        if let Some(label) = label.filter(|_| locked && holds_clicked) {
            out.push(json!({
                "type": "context",
                "elements": [{ "type": "mrkdwn", "text": escape(label) }],
            }));
        }
    }
    Some(Value::Array(out))
}

fn is_clicked(element: &Value, clicked: &str) -> bool {
    ["value", "action_id"]
        .iter()
        .any(|key| element.get(*key).and_then(Value::as_str) == Some(clicked))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;

    const OWNER: &str = "UOWNER";
    const PUBLISH: &str = "omon:btn:content-intel:publish:86";
    const PREVIEW: &str = "omon:btn:content-intel:preview:86";

    struct Fake {
        api: SlackApi,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
    }

    impl Fake {
        fn requests(&self) -> Vec<(String, Value)> {
            self.requests.lock().clone()
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
                recorded.lock().push((path.clone(), body));
                let reply = match path.as_str() {
                    "chat.postMessage" => json!({ "ok": true, "ts": "200.0001" }),
                    _ => json!({ "ok": true }),
                };
                axum::Json(reply)
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake {
            api: SlackApi::with_base_url("xoxb-test".into(), base),
            requests,
        }
    }

    fn config() -> SlackConfig {
        SlackConfig {
            allowed_users: vec![OWNER.into()],
            ..SlackConfig::test_default()
        }
    }

    fn actions_file(dir: &Path, body: &str) -> PathBuf {
        let script = dir.join("action.sh");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        let path = dir.join("button-actions.json");
        let config = json!({
            "content-intel": {
                "command": ["/bin/sh", script.to_string_lossy()],
                "timeout_secs": 30,
                "done_labels": { "publish": "✓ 발행됨" },
                "nonterminal_actions": ["preview"],
            }
        });
        std::fs::write(&path, config.to_string()).unwrap();
        path
    }

    fn message_blocks() -> Value {
        json!([
            { "type": "section", "text": { "type": "mrkdwn", "text": "draft 86" } },
            {
                "type": "actions",
                "elements": [
                    { "type": "button", "text": { "type": "plain_text", "text": "발행" }, "action_id": PUBLISH, "value": PUBLISH },
                    { "type": "button", "text": { "type": "plain_text", "text": "미리보기" }, "action_id": PREVIEW, "value": PREVIEW },
                    { "type": "button", "text": { "type": "plain_text", "text": "대시보드" }, "url": "http://127.0.0.1:4321" },
                ],
            },
        ])
    }

    fn click(user: &str, channel: &str, value: &str, action_ts: &str) -> Value {
        json!({
            "type": "block_actions",
            "user": { "id": user },
            "channel": { "id": channel },
            "container": { "channel_id": channel, "message_ts": "100.0001" },
            "message": { "ts": "100.0001", "text": "draft 86", "blocks": message_blocks() },
            "actions": [{ "action_id": value, "value": value, "action_ts": action_ts }],
        })
    }

    async fn run(payload: &Value, fake: &Fake, path: &Path) -> usize {
        let handles = start_button_actions(payload, &config(), &fake.api, path)
            .await
            .unwrap();
        let count = handles.len();
        for handle in handles {
            handle.await.unwrap();
        }
        count
    }

    fn element_texts(blocks: &Value) -> Vec<String> {
        blocks
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|block| block["elements"].as_array().cloned().unwrap_or_default())
            .map(|element| {
                element["text"]["text"]
                    .as_str()
                    .or_else(|| element["text"].as_str())
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn button_value_reads_value_or_action_id_with_prefix() {
        assert_eq!(button_value(&json!({ "value": PUBLISH })), Some(PUBLISH));
        assert_eq!(
            button_value(&json!({ "value": "x", "action_id": PUBLISH })),
            Some(PUBLISH)
        );
        assert_eq!(
            button_value(&json!({ "value": "omon:approval:1:once" })),
            None
        );
    }

    #[test]
    fn locked_rebuild_keeps_links_and_shows_label() {
        let pending =
            rebuild_blocks(&message_blocks(), PUBLISH, Some(PENDING_LABEL), true).unwrap();
        assert_eq!(element_texts(&pending), ["대시보드", PENDING_LABEL]);
        assert_eq!(pending[1]["elements"][0]["url"], "http://127.0.0.1:4321");
        assert_eq!(pending[2]["type"], "context");
        assert_eq!(pending[0], message_blocks()[0]);
    }

    #[test]
    fn nonterminal_rebuild_relabels_clicked_button_in_place() {
        let done = rebuild_blocks(&message_blocks(), PREVIEW, Some("✓ Preview"), false).unwrap();
        assert_eq!(element_texts(&done), ["발행", "✓ Preview", "대시보드"]);
        assert_eq!(done[1]["elements"][1]["value"], PREVIEW);
        assert!(rebuild_blocks(&Value::Null, PREVIEW, None, true).is_none());
    }

    #[test]
    fn executed_action_lru_deduplicates_and_evicts_oldest() {
        let mut lru = ActionLru::default();
        lru.insert("41.1".into());
        assert!(lru.contains("41.1"));
        for id in 0..EXECUTED_ACTION_LIMIT - 1 {
            lru.insert(format!("{id}.0"));
        }
        assert!(lru.contains("41.1"));
        lru.insert("last".into());
        assert!(lru.contains("41.1"));
        assert!(!lru.contains("0.0"));
    }

    #[tokio::test]
    async fn owner_click_runs_command_marks_done_and_replies_in_thread_once() {
        let fake = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let path = actions_file(
            dir.path(),
            r#"echo "argv=$1,$2 user=$OMON_BUTTON_USER_ID platform=$OMON_BUTTON_PLATFORM <!here>""#,
        );
        let payload = click(OWNER, "CBTN1", PUBLISH, "1700.111");

        assert_eq!(run(&payload, &fake, &path).await, 1);
        let requests = fake.requests();
        let paths: Vec<&str> = requests.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["chat.update", "chat.update", "chat.postMessage"]);
        assert_eq!(requests[0].1["channel"], "CBTN1");
        assert_eq!(requests[0].1["ts"], "100.0001");
        assert_eq!(
            element_texts(&requests[0].1["blocks"]),
            ["대시보드", PENDING_LABEL]
        );
        assert_eq!(
            element_texts(&requests[1].1["blocks"]),
            ["대시보드", "✓ 발행됨"]
        );
        assert_eq!(requests[2].1["thread_ts"], "100.0001");
        assert_eq!(
            requests[2].1["text"],
            "argv=publish,86 user=UOWNER platform=slack &lt;!here&gt;"
        );
        assert!(!IN_FLIGHT.lock().contains("CBTN1:100.0001"));

        assert_eq!(run(&payload, &fake, &path).await, 0);
        assert_eq!(fake.requests().len(), 3);
    }

    #[tokio::test]
    async fn failed_command_restores_buttons_and_reports_retry() {
        let fake = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let path = actions_file(dir.path(), "echo 'token expired'; exit 3");

        run(&click(OWNER, "CBTN2", PUBLISH, "1700.222"), &fake, &path).await;

        let requests = fake.requests();
        assert_eq!(requests[1].0, "chat.update");
        assert_eq!(requests[1].1["blocks"], message_blocks());
        let reply = requests[2].1["text"].as_str().unwrap();
        assert!(
            reply.contains("exit 3") && reply.contains("token expired"),
            "{reply}"
        );
    }

    #[tokio::test]
    async fn nonterminal_action_keeps_buttons_clickable() {
        let fake = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let path = actions_file(dir.path(), "echo ok");

        run(&click(OWNER, "CBTN3", PREVIEW, "1700.333"), &fake, &path).await;

        let requests = fake.requests();
        assert_eq!(
            element_texts(&requests[1].1["blocks"]),
            ["발행", DEFAULT_DONE_LABEL, "대시보드"]
        );
    }

    #[tokio::test]
    async fn unauthorized_unconfigured_and_in_flight_clicks_do_not_run() {
        let fake = fake().await;
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("ran");
        let path = actions_file(dir.path(), &format!("touch {}", marker.display()));

        run(
            &click("USTRANGER", "CBTN4", PUBLISH, "1700.401"),
            &fake,
            &path,
        )
        .await;
        run(
            &click(OWNER, "CBTN4", "omon:btn:other:publish:86", "1700.402"),
            &fake,
            &path,
        )
        .await;
        IN_FLIGHT.lock().insert("CBTN4:100.0001".into());
        run(&click(OWNER, "CBTN4", PUBLISH, "1700.403"), &fake, &path).await;
        IN_FLIGHT.lock().remove("CBTN4:100.0001");

        assert!(!marker.exists());
        let requests = fake.requests();
        let texts: Vec<&str> = requests
            .iter()
            .map(|(path, body)| {
                assert_eq!(path, "chat.postEphemeral");
                body["text"].as_str().unwrap()
            })
            .collect();
        assert_eq!(
            texts,
            [
                NOT_AUTHORIZED,
                "`other` 버튼 동작이 설정되어 있지 않습니다.",
                ALREADY_RUNNING,
            ]
        );
        assert_eq!(requests[0].1["user"], "USTRANGER");

        run(&click(OWNER, "CBTN4", PUBLISH, "1700.404"), &fake, &path).await;
        assert!(marker.exists());
    }
}

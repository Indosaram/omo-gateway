use std::path::Path;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, CONTENT_TYPE, RETRY_AFTER};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

use crate::{OmonError, Result};

const DEFAULT_BASE_URL: &str = "https://slack.com/api";
const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(1);
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);
const MAX_DOWNLOAD_BYTES: u64 = 50 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AuthInfo {
    pub user_id: String,
    #[serde(default)]
    pub bot_id: String,
    #[serde(default)]
    pub team_id: String,
    #[serde(default)]
    pub team: String,
}

/// One page of `conversations.replies`, with Slack's continuation cursor when more remain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepliesPage {
    pub messages: Vec<Value>,
    pub next_cursor: Option<String>,
}

/// One page of `conversations.history`, with Slack's `has_more` flag.
///
/// `messages.len() < limit` is not an end-of-history signal either: Slack may answer a full-size
/// request with a short page. Only `has_more` says whether older messages remain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryPage {
    pub messages: Vec<Value>,
    pub has_more: bool,
}

/// Slack Web API client. Deliberately not `Debug`: it holds the bot token.
#[derive(Clone)]
pub struct SlackApi {
    http: reqwest::Client,
    token: String,
    base: String,
}

impl SlackApi {
    pub fn new(token: String) -> Self {
        Self::with_base_url(token, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(token: String, base: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            token,
            base: base.into().trim_end_matches('/').to_owned(),
        }
    }

    pub async fn auth_test(&self) -> Result<AuthInfo> {
        let value = self.call("auth.test", json!({})).await?;
        serde_json::from_value(value)
            .map_err(|error| slack_error("auth.test", format!("invalid response: {error}")))
    }

    pub async fn post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
        blocks: Option<Value>,
    ) -> Result<String> {
        let mut body = json!({ "channel": channel, "text": text });
        if let Some(thread_ts) = thread_ts {
            body["thread_ts"] = json!(thread_ts);
        }
        if let Some(blocks) = blocks {
            body["blocks"] = blocks;
        }
        let value = self.call("chat.postMessage", body).await?;
        string_field("chat.postMessage", &value, "ts")
    }

    pub async fn update_message(
        &self,
        channel: &str,
        ts: &str,
        text: &str,
        blocks: Option<Value>,
    ) -> Result<()> {
        let mut body = json!({ "channel": channel, "ts": ts, "text": text });
        if let Some(blocks) = blocks {
            body["blocks"] = blocks;
        }
        self.call("chat.update", body).await.map(drop)
    }

    pub async fn delete_message(&self, channel: &str, ts: &str) -> Result<()> {
        self.call("chat.delete", json!({ "channel": channel, "ts": ts }))
            .await
            .map(drop)
    }

    /// `already_reacted` counts as success so status reactions stay idempotent.
    pub async fn add_reaction(&self, channel: &str, ts: &str, name: &str) -> Result<()> {
        let body = json!({ "channel": channel, "timestamp": ts, "name": name });
        self.call_tolerating("reactions.add", body, &["already_reacted"])
            .await
            .map(drop)
    }

    pub async fn remove_reaction(&self, channel: &str, ts: &str, name: &str) -> Result<()> {
        let body = json!({ "channel": channel, "timestamp": ts, "name": name });
        self.call_tolerating("reactions.remove", body, &["no_reaction"])
            .await
            .map(drop)
    }

    pub async fn post_ephemeral(&self, channel: &str, user: &str, text: &str) -> Result<()> {
        let body = json!({ "channel": channel, "user": user, "text": text });
        self.call("chat.postEphemeral", body).await.map(drop)
    }

    pub async fn conversations_info(&self, channel: &str) -> Result<Value> {
        let mut value = self
            .call("conversations.info", json!({ "channel": channel }))
            .await?;
        Ok(value["channel"].take())
    }

    pub async fn users_info(&self, user: &str) -> Result<Value> {
        let mut value = self.call("users.info", json!({ "user": user })).await?;
        Ok(value["user"].take())
    }

    pub async fn open_im(&self, user: &str) -> Result<String> {
        let value = self
            .call("conversations.open", json!({ "users": user }))
            .await?;
        value["channel"]["id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| slack_error("conversations.open", "missing channel.id"))
    }

    pub async fn conversations_history(
        &self,
        channel: &str,
        oldest: Option<&str>,
        latest: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Value>> {
        Ok(self
            .conversations_history_page(channel, oldest, latest, limit)
            .await?
            .messages)
    }

    /// One `conversations.history` page plus Slack's `has_more`.
    ///
    /// Callers that walk a channel of unknown length must page with this: a short page does not
    /// prove there is nothing older, so only `has_more` ends the walk. When Slack omits the flag,
    /// a full page counts as "may have more", which can only cost one extra request.
    pub async fn conversations_history_page(
        &self,
        channel: &str,
        oldest: Option<&str>,
        latest: Option<&str>,
        limit: u32,
    ) -> Result<HistoryPage> {
        let limit = limit.clamp(1, 200);
        let mut body = json!({ "channel": channel, "limit": limit, "inclusive": false });
        if let Some(oldest) = oldest {
            body["oldest"] = json!(oldest);
        }
        if let Some(latest) = latest {
            body["latest"] = json!(latest);
        }
        let mut value = self.call("conversations.history", body).await?;
        let messages = messages(&mut value);
        let has_more = value["has_more"]
            .as_bool()
            .unwrap_or(messages.len() >= limit as usize);
        Ok(HistoryPage { messages, has_more })
    }

    pub async fn conversations_replies(
        &self,
        channel: &str,
        thread_ts: &str,
        oldest: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Value>> {
        Ok(self
            .conversations_replies_page(channel, thread_ts, None, oldest, limit)
            .await?
            .messages)
    }

    /// One `conversations.replies` page plus Slack's `response_metadata.next_cursor`.
    ///
    /// Callers that walk a thread of unknown length must page with this: `messages.len() < limit`
    /// is not an end-of-thread signal (Slack may answer a full-size request with a short page that
    /// still has a `next_cursor`), so only the cursor says whether more replies remain.
    pub async fn conversations_replies_page(
        &self,
        channel: &str,
        thread_ts: &str,
        cursor: Option<&str>,
        oldest: Option<&str>,
        limit: u32,
    ) -> Result<RepliesPage> {
        let mut body = json!({ "channel": channel, "ts": thread_ts, "limit": limit.clamp(1, 200) });
        if let Some(cursor) = cursor {
            body["cursor"] = json!(cursor);
        }
        // `oldest` is only meaningful on the first request: a cursor already encodes the position.
        if let Some(oldest) = oldest.filter(|_| cursor.is_none()) {
            body["oldest"] = json!(oldest);
            body["inclusive"] = json!(false);
        }
        let mut value = self.call("conversations.replies", body).await?;
        let next_cursor = value["response_metadata"]["next_cursor"]
            .as_str()
            .map(str::trim)
            .filter(|cursor| !cursor.is_empty())
            .map(str::to_owned);
        Ok(RepliesPage {
            messages: messages(&mut value),
            next_cursor,
        })
    }

    pub async fn conversations_members(&self, channel: &str) -> Result<Vec<String>> {
        let value = self
            .call(
                "conversations.members",
                json!({ "channel": channel, "limit": 200 }),
            )
            .await?;
        Ok(value["members"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn permalink(&self, channel: &str, ts: &str) -> Result<String> {
        let value = self
            .call(
                "chat.getPermalink",
                json!({ "channel": channel, "message_ts": ts }),
            )
            .await?;
        string_field("chat.getPermalink", &value, "permalink")
    }

    pub async fn upload_bytes(
        &self,
        channel: &str,
        thread_ts: Option<&str>,
        filename: &str,
        bytes: Vec<u8>,
        title: Option<&str>,
    ) -> Result<()> {
        let dir = std::env::temp_dir().join(format!("omon-slack-upload-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|error| slack_error("files.upload", error))?;
        let path = dir.join(sanitize_filename(filename));
        let result = match tokio::fs::write(&path, bytes).await {
            Ok(()) => self
                .upload_file(channel, thread_ts, &path, title)
                .await
                .map(drop),
            Err(error) => Err(slack_error("files.upload", error)),
        };
        let _ = tokio::fs::remove_dir_all(&dir).await;
        result
    }

    pub async fn upload_file(
        &self,
        channel: &str,
        thread_ts: Option<&str>,
        path: &Path,
        title: Option<&str>,
    ) -> Result<String> {
        let bytes = tokio::fs::read(path).await.map_err(|error| {
            slack_error(
                "files.getUploadURLExternal",
                format!("failed to read {}: {error}", path.display()),
            )
        })?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file")
            .to_owned();
        let ticket = self
            .call_form(
                "files.getUploadURLExternal",
                &self.token,
                &[
                    ("filename", filename.clone()),
                    ("length", bytes.len().to_string()),
                ],
            )
            .await?;
        let upload_url = string_field("files.getUploadURLExternal", &ticket, "upload_url")?;
        let file_id = string_field("files.getUploadURLExternal", &ticket, "file_id")?;

        let response = self
            .send("files.upload", || {
                self.http.post(upload_url.as_str()).body(bytes.clone())
            })
            .await?;
        if !response.status().is_success() {
            return Err(slack_error(
                "files.upload",
                format!("http {}", response.status()),
            ));
        }

        let mut body = json!({
            "files": [{ "id": file_id, "title": title.unwrap_or(&filename) }],
            "channel_id": channel,
        });
        if let Some(thread_ts) = thread_ts {
            body["thread_ts"] = json!(thread_ts);
        }
        self.call("files.completeUploadExternal", body).await?;
        Ok(file_id)
    }

    /// Downloads a `url_private` file with bot auth, following redirects, capped at 50 MB.
    pub async fn download_file(&self, url_private: &str, dest: &Path) -> Result<()> {
        self.download_capped(url_private, dest, MAX_DOWNLOAD_BYTES)
            .await
    }

    /// Posts to an interaction `response_url`; these URLs carry their own auth.
    pub async fn post_response_url(&self, url: &str, body: Value) -> Result<()> {
        let response = self
            .send("response_url", || self.http.post(url).json(&body))
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(slack_error(
                "response_url",
                format!("http {}", response.status()),
            ))
        }
    }

    pub async fn connections_open(&self, app_token: &str) -> Result<String> {
        let value = self
            .call_form("apps.connections.open", app_token, &[])
            .await?;
        string_field("apps.connections.open", &value, "url")
    }

    async fn download_capped(&self, url: &str, dest: &Path, cap: u64) -> Result<()> {
        const LABEL: &str = "files.download";
        let response = self
            .send(LABEL, || self.http.get(url).bearer_auth(&self.token))
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(slack_error(LABEL, format!("http {status}")));
        }
        let is_html = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"));
        if is_html {
            return Err(slack_error(
                LABEL,
                "received an HTML page instead of the file (missing files:read scope?)",
            ));
        }
        if response.content_length().is_some_and(|length| length > cap) {
            return Err(slack_error(
                LABEL,
                format!("file exceeds the {cap} byte limit"),
            ));
        }
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|error| {
                slack_error(
                    LABEL,
                    format!("failed to create {}: {error}", parent.display()),
                )
            })?;
        }
        let outcome = write_capped(response, dest, cap).await;
        if outcome.is_err() {
            let _ = tokio::fs::remove_file(dest).await;
        }
        outcome
    }

    async fn call(&self, method: &str, body: Value) -> Result<Value> {
        self.call_tolerating(method, body, &[]).await
    }

    async fn call_tolerating(
        &self,
        method: &str,
        body: Value,
        tolerated: &[&str],
    ) -> Result<Value> {
        let url = format!("{}/{method}", self.base);
        let response = if form_only(method) {
            let form = form_fields(&body);
            self.send(method, || {
                self.http
                    .post(url.as_str())
                    .bearer_auth(&self.token)
                    .form(&form)
            })
            .await?
        } else {
            self.send(method, || {
                self.http
                    .post(url.as_str())
                    .bearer_auth(&self.token)
                    .json(&body)
            })
            .await?
        };
        parse_response(method, response, tolerated).await
    }

    async fn call_form(&self, method: &str, token: &str, form: &[(&str, String)]) -> Result<Value> {
        let url = format!("{}/{method}", self.base);
        let response = self
            .send(method, || {
                self.http.post(url.as_str()).bearer_auth(token).form(form)
            })
            .await?;
        parse_response(method, response, &[]).await
    }

    /// Sends the request, honouring `Retry-After` on HTTP 429 up to three times.
    async fn send(
        &self,
        method: &str,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        let mut retries = 0;
        loop {
            let response = build().send().await.map_err(|error| {
                slack_error(method, format!("request failed: {}", error.without_url()))
            })?;
            if response.status() != StatusCode::TOO_MANY_REQUESTS
                || retries >= MAX_RATE_LIMIT_RETRIES
            {
                return Ok(response);
            }
            retries += 1;
            let delay = retry_after(response.headers());
            tracing::warn!(
                method,
                retry = retries,
                delay_ms = delay.as_millis() as u64,
                "slack rate limited"
            );
            tokio::time::sleep(delay).await;
        }
    }
}

async fn parse_response(
    method: &str,
    response: reqwest::Response,
    tolerated: &[&str],
) -> Result<Value> {
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| {
        slack_error(
            method,
            format!("failed to read response: {}", error.without_url()),
        )
    })?;
    let value = serde_json::from_slice::<Value>(&bytes).ok();
    let Some(value) = value else {
        return Err(if status.is_success() {
            slack_error(method, "invalid JSON response")
        } else {
            slack_error(method, format!("http {status}"))
        });
    };
    if value["ok"].as_bool() == Some(true) {
        return Ok(value);
    }
    match value["error"].as_str() {
        Some(code) if tolerated.contains(&code) => Ok(value),
        Some(code) => Err(slack_error(method, code)),
        None if !status.is_success() => Err(slack_error(method, format!("http {status}"))),
        None => Err(slack_error(method, "unknown_error")),
    }
}

async fn write_capped(response: reqwest::Response, dest: &Path, cap: u64) -> Result<()> {
    const LABEL: &str = "files.download";
    let mut file = tokio::fs::File::create(dest).await.map_err(|error| {
        slack_error(
            LABEL,
            format!("failed to create {}: {error}", dest.display()),
        )
    })?;
    let mut total = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            slack_error(
                LABEL,
                format!("download stream failed: {}", error.without_url()),
            )
        })?;
        total = total.saturating_add(chunk.len() as u64);
        if total > cap {
            return Err(slack_error(
                LABEL,
                format!("file exceeds the {cap} byte limit"),
            ));
        }
        file.write_all(&chunk).await.map_err(|error| {
            slack_error(LABEL, format!("failed writing {}: {error}", dest.display()))
        })?;
    }
    file.flush().await.map_err(|error| {
        slack_error(
            LABEL,
            format!("failed flushing {}: {error}", dest.display()),
        )
    })
}

fn retry_after(headers: &HeaderMap) -> Duration {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(DEFAULT_RETRY_AFTER, Duration::from_secs)
        .min(MAX_RETRY_AFTER)
}

fn string_field(method: &str, value: &Value, field: &str) -> Result<String> {
    value[field]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| slack_error(method, format!("missing {field}")))
}

fn slack_error(method: &str, detail: impl std::fmt::Display) -> OmonError {
    OmonError::Multiplexer(format!("slack {method}: {detail}"))
}

// Slack's read methods (conversations.*, users.info, chat.getPermalink) ignore JSON bodies and
// report "missing required field"; form encoding is accepted by every method.
/// Test helper: decodes a recorded request body sent either as JSON or as form fields.
#[cfg(test)]
pub(crate) fn decode_test_body(bytes: &[u8]) -> Value {
    if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
        return value;
    }
    let text = String::from_utf8_lossy(bytes);
    let fields: serde_json::Map<String, Value> = text
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| {
            let decode = |raw: &str| {
                urlencoding::decode(&raw.replace('+', " "))
                    .map(|s| s.into_owned())
                    .unwrap_or_default()
            };
            (decode(key), Value::String(decode(value)))
        })
        .collect();
    if fields.is_empty() {
        Value::String(text.into_owned())
    } else {
        Value::Object(fields)
    }
}

fn form_only(method: &str) -> bool {
    method.starts_with("conversations.") || method == "users.info" || method == "chat.getPermalink"
}

fn form_fields(body: &Value) -> Vec<(String, String)> {
    body.as_object()
        .map(|map| {
            map.iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| {
                    let text = match value {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    };
                    (key.clone(), text)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn messages(value: &mut Value) -> Vec<Value> {
    match value["messages"].take() {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0') {
                '_'
            } else {
                c
            }
        })
        .collect();
    match cleaned.trim() {
        "" | "." | ".." => "upload.bin".to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{header, HeaderValue};
    use axum::response::Response;
    use parking_lot::Mutex;

    use super::*;

    #[derive(Clone, Debug)]
    struct Recorded {
        path: String,
        auth: Option<String>,
        content_type: Option<String>,
        body: Vec<u8>,
    }

    impl Recorded {
        fn json(&self) -> Value {
            super::decode_test_body(&self.body)
        }

        fn text(&self) -> String {
            String::from_utf8(self.body.clone()).unwrap()
        }
    }

    type Responder = dyn Fn(&Recorded, &str, usize) -> Response + Send + Sync;

    struct Fake {
        base: String,
        requests: Arc<Mutex<Vec<Recorded>>>,
    }

    impl Fake {
        fn requests(&self) -> Vec<Recorded> {
            self.requests.lock().clone()
        }

        fn api(&self, token: &str) -> SlackApi {
            SlackApi::with_base_url(token.to_owned(), format!("{}/api", self.base))
        }
    }

    async fn fake(
        responder: impl Fn(&Recorded, &str, usize) -> Response + Send + Sync + 'static,
    ) -> Fake {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::<Recorded>::new()));
        let responder: Arc<Responder> = Arc::new(responder);
        let state = (requests.clone(), responder, base.clone());
        let app = axum::Router::new().fallback(move |req: Request| {
            let (requests, responder, base) = state.clone();
            async move {
                let (path, auth, content_type) = {
                    let headers = req.headers();
                    let read_header = |name: header::HeaderName| {
                        headers
                            .get(name)
                            .and_then(|value: &HeaderValue| value.to_str().ok())
                            .map(str::to_owned)
                    };
                    (
                        req.uri().path().to_owned(),
                        read_header(header::AUTHORIZATION),
                        read_header(header::CONTENT_TYPE),
                    )
                };
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .to_vec();
                let recorded = Recorded {
                    path,
                    auth,
                    content_type,
                    body,
                };
                let seen = {
                    let mut requests = requests.lock();
                    let seen = requests.iter().filter(|r| r.path == recorded.path).count();
                    requests.push(recorded.clone());
                    seen
                };
                responder(&recorded, &base, seen)
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        Fake { base, requests }
    }

    fn json_response(status: u16, value: Value) -> Response {
        axum::http::Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .unwrap()
    }

    fn ok(mut value: Value) -> Response {
        value["ok"] = json!(true);
        json_response(200, value)
    }

    fn multiplexer_message(error: OmonError) -> String {
        match error {
            OmonError::Multiplexer(message) => message,
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_methods_send_form_fields_because_slack_ignores_json_there() {
        let server = fake(|_, _, _| ok(json!({ "messages": [{ "ts": "1.0" }] }))).await;
        let api = server.api("xoxb-test");

        let messages = api
            .conversations_replies("C1", "1.0", Some("0.5"), 5)
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        let request = &server.requests()[0];
        assert_eq!(request.path, "/api/conversations.replies");
        assert!(request
            .content_type
            .as_deref()
            .unwrap()
            .starts_with("application/x-www-form-urlencoded"));
        let body = request.json();
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["ts"], "1.0");
        assert_eq!(body["oldest"], "0.5");
        assert_eq!(body["limit"], "5");
    }

    #[tokio::test]
    async fn post_message_sends_bearer_json_and_returns_ts() {
        let server = fake(|_, _, _| ok(json!({ "ts": "1700000000.000100" }))).await;
        let api = server.api("xoxb-test");

        let ts = api
            .post_message(
                "C1",
                "hello",
                Some("1699.0001"),
                Some(json!([{ "type": "divider" }])),
            )
            .await
            .unwrap();

        assert_eq!(ts, "1700000000.000100");
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.path, "/api/chat.postMessage");
        assert_eq!(request.auth.as_deref(), Some("Bearer xoxb-test"));
        assert!(request
            .content_type
            .as_deref()
            .unwrap()
            .starts_with("application/json"));
        let body = request.json();
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["text"], "hello");
        assert_eq!(body["thread_ts"], "1699.0001");
        assert_eq!(body["blocks"][0]["type"], "divider");
    }

    #[tokio::test]
    async fn post_message_omits_absent_thread_and_blocks() {
        let server = fake(|_, _, _| ok(json!({ "ts": "1.2" }))).await;
        server
            .api("xoxb-test")
            .post_message("D1", "hi", None, None)
            .await
            .unwrap();

        let body = server.requests()[0].json();
        assert!(body.get("thread_ts").is_none());
        assert!(body.get("blocks").is_none());
    }

    #[tokio::test]
    async fn ok_false_maps_to_multiplexer_error_with_method_and_code() {
        let server = fake(|_, _, _| {
            json_response(200, json!({ "ok": false, "error": "message_not_found" }))
        })
        .await;

        let error = server
            .api("xoxb-test")
            .update_message("C1", "1.2", "edited", None)
            .await
            .unwrap_err();

        assert_eq!(
            multiplexer_message(error),
            "slack chat.update: message_not_found"
        );
        let body = server.requests()[0].json();
        assert_eq!(body["ts"], "1.2");
        assert_eq!(body["text"], "edited");
    }

    #[tokio::test]
    async fn error_message_never_contains_token() {
        let server =
            fake(|_, _, _| json_response(200, json!({ "ok": false, "error": "invalid_auth" })))
                .await;
        let message = multiplexer_message(
            server
                .api("xoxb-secret-value")
                .delete_message("C1", "1.2")
                .await
                .unwrap_err(),
        );
        assert!(!message.contains("xoxb-secret-value"));
        assert_eq!(message, "slack chat.delete: invalid_auth");
    }

    #[tokio::test]
    async fn reactions_tolerate_idempotent_codes_only() {
        let server = fake(|request, _, _| {
            let code = match request.path.as_str() {
                "/api/reactions.remove" => "no_reaction",
                "/api/reactions.add" if request.json()["name"] == "eyes" => "already_reacted",
                _ => "invalid_name",
            };
            json_response(200, json!({ "ok": false, "error": code }))
        })
        .await;
        let api = server.api("xoxb-test");

        api.remove_reaction("C1", "1.2", "eyes").await.unwrap();
        api.add_reaction("C1", "1.2", "eyes").await.unwrap();
        let error = api.add_reaction("C1", "1.2", "bogus").await.unwrap_err();

        assert_eq!(
            multiplexer_message(error),
            "slack reactions.add: invalid_name"
        );
        let body = server.requests()[0].json();
        assert_eq!(body["channel"], "C1");
        assert_eq!(body["timestamp"], "1.2");
        assert_eq!(body["name"], "eyes");
    }

    #[tokio::test]
    async fn rate_limit_retries_after_header_then_succeeds() {
        let server = fake(|_, _, seen| {
            if seen < 2 {
                axum::http::Response::builder()
                    .status(429)
                    .header(header::RETRY_AFTER, "0")
                    .body(Body::empty())
                    .unwrap()
            } else {
                ok(json!({ "channel": { "id": "D42" } }))
            }
        })
        .await;

        let channel = server.api("xoxb-test").open_im("U1").await.unwrap();

        assert_eq!(channel, "D42");
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests.iter().all(|r| r.path == "/api/conversations.open"));
        assert_eq!(requests[2].json()["users"], "U1");
    }

    #[tokio::test]
    async fn rate_limit_gives_up_after_three_retries() {
        let server = fake(|_, _, _| {
            axum::http::Response::builder()
                .status(429)
                .header(header::RETRY_AFTER, "0")
                .body(Body::empty())
                .unwrap()
        })
        .await;

        let error = server.api("xoxb-test").auth_test().await.unwrap_err();

        assert_eq!(server.requests().len(), 4);
        assert_eq!(
            multiplexer_message(error),
            "slack auth.test: http 429 Too Many Requests"
        );
    }

    #[test]
    fn retry_after_defaults_to_one_second_and_caps_at_thirty() {
        let mut headers = HeaderMap::new();
        assert_eq!(retry_after(&headers), Duration::from_secs(1));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));
        assert_eq!(retry_after(&headers), Duration::from_secs(7));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("120"));
        assert_eq!(retry_after(&headers), Duration::from_secs(30));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("soon"));
        assert_eq!(retry_after(&headers), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn auth_test_parses_identity() {
        let server = fake(|_, _, _| {
            ok(json!({ "user_id": "U0BOT", "bot_id": "B0BOT", "team_id": "T1", "team": "Team" }))
        })
        .await;

        let info = server.api("xoxb-test").auth_test().await.unwrap();

        assert_eq!(
            info,
            AuthInfo {
                user_id: "U0BOT".into(),
                bot_id: "B0BOT".into(),
                team_id: "T1".into(),
                team: "Team".into(),
            }
        );
    }

    #[tokio::test]
    async fn info_methods_return_inner_objects() {
        let server = fake(|request, _, _| match request.path.as_str() {
            "/api/conversations.info" => ok(json!({ "channel": { "id": "C1", "is_im": false } })),
            _ => ok(json!({ "user": { "id": "U1", "name": "indo" } })),
        })
        .await;
        let api = server.api("xoxb-test");

        let channel = api.conversations_info("C1").await.unwrap();
        let user = api.users_info("U1").await.unwrap();

        assert_eq!(channel["is_im"], false);
        assert_eq!(user["name"], "indo");
        let requests = server.requests();
        assert_eq!(requests[0].json()["channel"], "C1");
        assert_eq!(requests[1].json()["user"], "U1");
    }

    #[tokio::test]
    async fn post_ephemeral_sends_user() {
        let server = fake(|_, _, _| ok(json!({ "message_ts": "1.2" }))).await;
        server
            .api("xoxb-test")
            .post_ephemeral("C1", "U1", "only you")
            .await
            .unwrap();

        let request = &server.requests()[0];
        assert_eq!(request.path, "/api/chat.postEphemeral");
        assert_eq!(request.json()["user"], "U1");
        assert_eq!(request.json()["text"], "only you");
    }

    #[tokio::test]
    async fn upload_file_runs_three_step_external_flow() {
        let server = fake(|request, base, _| match request.path.as_str() {
            "/api/files.getUploadURLExternal" => ok(json!({
                "upload_url": format!("{base}/upload/F123"),
                "file_id": "F123",
            })),
            "/upload/F123" => axum::http::Response::builder()
                .status(200)
                .body(Body::from("OK - 11"))
                .unwrap(),
            "/api/files.completeUploadExternal" => ok(json!({ "files": [{ "id": "F123" }] })),
            _ => json_response(404, json!({ "ok": false, "error": "unknown_method" })),
        })
        .await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.txt");
        std::fs::write(&path, "hello slack").unwrap();

        let file_id = server
            .api("xoxb-test")
            .upload_file("C1", Some("1699.0001"), &path, Some("Report"))
            .await
            .unwrap();

        assert_eq!(file_id, "F123");
        let requests = server.requests();
        assert_eq!(requests.len(), 3);

        let ticket = &requests[0];
        assert_eq!(ticket.path, "/api/files.getUploadURLExternal");
        assert_eq!(ticket.auth.as_deref(), Some("Bearer xoxb-test"));
        assert_eq!(
            ticket.content_type.as_deref(),
            Some("application/x-www-form-urlencoded")
        );
        let form = ticket.text();
        assert!(form.contains("filename=report.txt"), "{form}");
        assert!(form.contains("length=11"), "{form}");

        let upload = &requests[1];
        assert_eq!(upload.path, "/upload/F123");
        assert_eq!(upload.auth, None);
        assert_eq!(upload.body, b"hello slack");

        let complete = &requests[2];
        assert_eq!(complete.path, "/api/files.completeUploadExternal");
        assert_eq!(complete.auth.as_deref(), Some("Bearer xoxb-test"));
        let body = complete.json();
        assert_eq!(body["files"], json!([{ "id": "F123", "title": "Report" }]));
        assert_eq!(body["channel_id"], "C1");
        assert_eq!(body["thread_ts"], "1699.0001");
    }

    #[tokio::test]
    async fn upload_file_defaults_title_to_filename_and_fails_on_upload_error() {
        let server = fake(|request, base, _| match request.path.as_str() {
            "/api/files.getUploadURLExternal" => ok(json!({
                "upload_url": format!("{base}/upload/F9"),
                "file_id": "F9",
            })),
            "/upload/F9" if request.body.is_empty() => axum::http::Response::builder()
                .status(500)
                .body(Body::empty())
                .unwrap(),
            "/upload/F9" => axum::http::Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap(),
            _ => ok(json!({})),
        })
        .await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, "# hi").unwrap();
        let api = server.api("xoxb-test");

        api.upload_file("D1", None, &path, None).await.unwrap();
        let complete = server.requests()[2].json();
        assert_eq!(complete["files"][0]["title"], "notes.md");
        assert!(complete.get("thread_ts").is_none());

        let empty = dir.path().join("empty.bin");
        std::fs::write(&empty, "").unwrap();
        let error = api.upload_file("D1", None, &empty, None).await.unwrap_err();
        assert_eq!(
            multiplexer_message(error),
            "slack files.upload: http 500 Internal Server Error"
        );
        assert_eq!(server.requests().len(), 5);
    }

    #[tokio::test]
    async fn download_file_follows_redirect_with_bot_auth() {
        let server = fake(|request, _, _| match request.path.as_str() {
            "/files-pri/T1-F1/a.png" => axum::http::Response::builder()
                .status(302)
                .header(header::LOCATION, "/stored/a.png")
                .body(Body::empty())
                .unwrap(),
            "/stored/a.png" if request.auth.as_deref() == Some("Bearer xoxb-test") => {
                axum::http::Response::builder()
                    .status(200)
                    .header(header::CONTENT_TYPE, "image/png")
                    .body(Body::from(vec![1_u8, 2, 3, 4]))
                    .unwrap()
            }
            _ => axum::http::Response::builder()
                .status(403)
                .body(Body::empty())
                .unwrap(),
        })
        .await;
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("nested").join("a.png");

        server
            .api("xoxb-test")
            .download_file(&format!("{}/files-pri/T1-F1/a.png", server.base), &dest)
            .await
            .unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), vec![1, 2, 3, 4]);
        let requests = server.requests();
        assert_eq!(requests[0].auth.as_deref(), Some("Bearer xoxb-test"));
        assert_eq!(requests[1].path, "/stored/a.png");
    }

    #[tokio::test]
    async fn download_rejects_oversize_and_html_and_removes_partial_file() {
        let server = fake(|request, _, _| match request.path.as_str() {
            "/login" => axum::http::Response::builder()
                .status(200)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from("<html>sign in</html>"))
                .unwrap(),
            _ => axum::http::Response::builder()
                .status(200)
                .body(Body::from(vec![0_u8; 64]))
                .unwrap(),
        })
        .await;
        let api = server.api("xoxb-test");
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("big.bin");

        let error = api
            .download_capped(&format!("{}/big", server.base), &dest, 16)
            .await
            .unwrap_err();
        assert_eq!(
            multiplexer_message(error),
            "slack files.download: file exceeds the 16 byte limit"
        );
        assert!(!dest.exists());

        let error = api
            .download_file(&format!("{}/login", server.base), &dest)
            .await
            .unwrap_err();
        assert!(multiplexer_message(error).contains("HTML page"));
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn post_response_url_sends_json_without_auth() {
        let server = fake(|request, _, _| {
            let status = if request.path == "/hooks/ok" {
                200
            } else {
                404
            };
            axum::http::Response::builder()
                .status(status)
                .body(Body::from("ok"))
                .unwrap()
        })
        .await;
        let api = server.api("xoxb-test");

        api.post_response_url(
            &format!("{}/hooks/ok", server.base),
            json!({ "text": "done", "response_type": "ephemeral" }),
        )
        .await
        .unwrap();
        let error = api
            .post_response_url(&format!("{}/hooks/gone", server.base), json!({}))
            .await
            .unwrap_err();

        let request = &server.requests()[0];
        assert_eq!(request.auth, None);
        assert_eq!(request.json()["text"], "done");
        assert_eq!(
            multiplexer_message(error),
            "slack response_url: http 404 Not Found"
        );
    }

    #[tokio::test]
    async fn connections_open_uses_app_token() {
        let server =
            fake(|_, _, _| ok(json!({ "url": "wss://wss-primary.slack.com/link/?ticket=t" })))
                .await;

        let url = server
            .api("xoxb-bot")
            .connections_open("xapp-app")
            .await
            .unwrap();

        assert_eq!(url, "wss://wss-primary.slack.com/link/?ticket=t");
        let request = &server.requests()[0];
        assert_eq!(request.path, "/api/apps.connections.open");
        assert_eq!(request.auth.as_deref(), Some("Bearer xapp-app"));
    }
}

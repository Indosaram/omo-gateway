use std::collections::BTreeMap;
use std::pin::Pin;

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{MessageAttachment, OmonError, StreamChunk};

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, OmonError>> + Send>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LlmProvider {
    OpenAi,
    Anthropic,
    DeepSeek,
    Ollama,
}

#[derive(Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

impl std::fmt::Debug for LlmConfig {
    /// Redacts `api_key` so tracing output and panics never carry the secret.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LlmConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("base_url", &self.base_url)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

impl LlmConfig {
    pub fn new(provider: LlmProvider, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            api_key: None,
            base_url: None,
            max_tokens: 32768,
            temperature: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<MessageAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            attachments: Vec::new(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn with_attachments(mut self, attachments: Vec<MessageAttachment>) -> Self {
        self.attachments = attachments;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// A provider that sends no newline would otherwise grow the line buffer unbounded.
const MAX_STREAM_LINE_BYTES: usize = 4 * 1024 * 1024;

/// True when the base URL already ends with a version path segment (for
/// example `/v1` or z.ai's `/v4`), so only `/chat/completions` needs appending.
fn ends_with_version_segment(base: &str) -> bool {
    base.rsplit('/').next().is_some_and(|segment| {
        let digits = segment.strip_prefix('v').unwrap_or(segment);
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    fn openai_endpoint(base: Option<&str>) -> String {
        let mut config = LlmConfig::new(LlmProvider::OpenAi, "glm-5.3-flash");
        config.base_url = base.map(str::to_string);
        LlmClient::new(config).unwrap().endpoint()
    }

    #[test]
    fn versioned_base_urls_skip_v1_append() {
        assert_eq!(
            openai_endpoint(Some("https://api.z.ai/api/coding/paas/v4")),
            "https://api.z.ai/api/coding/paas/v4/chat/completions"
        );
        assert_eq!(
            openai_endpoint(Some("http://127.0.0.1:8317/v1")),
            "http://127.0.0.1:8317/v1/chat/completions"
        );
        assert_eq!(
            openai_endpoint(Some("https://llm.example.com")),
            "https://llm.example.com/v1/chat/completions"
        );
    }
}

#[derive(Clone)]
pub struct LlmClient {
    config: LlmConfig,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self, OmonError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Credentials are marked sensitive so a Debug print of the client or request
        // never shows them.
        if config.provider == LlmProvider::Anthropic {
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            if let Some(api_key) = &config.api_key {
                let mut value = HeaderValue::from_str(api_key)
                    .map_err(|error| OmonError::Config(error.to_string()))?;
                value.set_sensitive(true);
                headers.insert("x-api-key", value);
            }
        } else if let Some(api_key) = &config.api_key {
            let mut value = HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| OmonError::Config(error.to_string()))?;
            value.set_sensitive(true);
            headers.insert(AUTHORIZATION, value);
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            // reqwest strips `Authorization` on a cross-host redirect but not `x-api-key`;
            // a provider endpoint never legitimately redirects an API call.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|error| OmonError::Llm(error.to_string()))?;
        Ok(Self { config, http })
    }

    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    pub fn endpoint(&self) -> String {
        if let Some(base) = &self.config.base_url {
            let base = base.trim_end_matches('/');
            return match self.config.provider {
                LlmProvider::Anthropic if base.ends_with("/v1") => format!("{base}/messages"),
                LlmProvider::Anthropic => format!("{base}/v1/messages"),
                LlmProvider::Ollama if base.ends_with("/api") => format!("{base}/chat"),
                LlmProvider::Ollama => format!("{base}/api/chat"),
                _ if ends_with_version_segment(base) => format!("{base}/chat/completions"),
                _ => format!("{base}/v1/chat/completions"),
            };
        }
        match self.config.provider {
            LlmProvider::OpenAi => "https://api.openai.com/v1/chat/completions".into(),
            LlmProvider::Anthropic => "https://api.anthropic.com/v1/messages".into(),
            LlmProvider::DeepSeek => "https://api.deepseek.com/v1/chat/completions".into(),
            LlmProvider::Ollama => "http://localhost:11434/api/chat".into(),
        }
    }

    pub fn build_payload(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Value {
        match self.config.provider {
            LlmProvider::Anthropic => self.anthropic_payload(messages, tools),
            LlmProvider::Ollama => self.openai_payload(messages, tools, true),
            LlmProvider::OpenAi | LlmProvider::DeepSeek => {
                self.openai_payload(messages, tools, false)
            }
        }
    }

    pub async fn stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmStream, OmonError> {
        let (stream, _tool_calls) = self.stream_with_tool_calls(messages, tools).await?;
        Ok(stream)
    }

    pub async fn stream_with_tool_calls(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<
        (
            LlmStream,
            oneshot::Receiver<Result<Vec<ToolCall>, OmonError>>,
        ),
        OmonError,
    > {
        let response = self
            .http
            .post(self.endpoint())
            .json(&self.build_payload(messages, tools))
            .send()
            .await
            .map_err(|error| OmonError::Llm(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(OmonError::Llm(format!(
                "provider returned {status}: {body}"
            )));
        }

        let provider = self.config.provider;
        let (sender, receiver) = mpsc::channel(32);
        let (tool_calls_tx, tool_calls_rx) = oneshot::channel();
        tokio::spawn(async move {
            let stream_id = Uuid::new_v4();
            let mut sequence = 0;
            let mut buffer = String::new();
            let mut pending = Vec::new();
            let mut tool_calls = BTreeMap::new();
            let mut terminated = false;
            let mut bytes = response.bytes_stream();
            while let Some(next) = bytes.next().await {
                let bytes = match next {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        let _ = sender.send(Err(OmonError::Llm(error.to_string()))).await;
                        return;
                    }
                };
                pending.extend_from_slice(&bytes);
                decode_stream_bytes(&mut pending, &mut buffer);
                while let Some(index) = buffer.find('\n') {
                    let line = buffer[..index].trim_end_matches('\r').to_owned();
                    buffer.drain(..=index);
                    if let Some(message) = stream_error(&line) {
                        let _ = tool_calls_tx.send(Err(OmonError::Llm(message.clone())));
                        let _ = sender.send(Err(OmonError::Llm(message))).await;
                        return;
                    }
                    terminated |= is_stream_terminator(provider, &line);
                    accumulate_stream_tool_calls(provider, &line, &mut tool_calls);
                    for content in parse_stream_line(provider, &line) {
                        let chunk = StreamChunk {
                            stream_id,
                            sequence,
                            content,
                            is_final: false,
                            reply_to: None,
                        };
                        sequence += 1;
                        if sender.send(Ok(chunk)).await.is_err() {
                            return;
                        }
                    }
                }
                if buffer.len() > MAX_STREAM_LINE_BYTES {
                    let message = format!(
                        "provider stream line exceeded {MAX_STREAM_LINE_BYTES} bytes without a newline"
                    );
                    let _ = tool_calls_tx.send(Err(OmonError::Llm(message.clone())));
                    let _ = sender.send(Err(OmonError::Llm(message))).await;
                    return;
                }
            }
            if !pending.is_empty() {
                buffer.push_str(&String::from_utf8_lossy(&pending));
            }
            if !buffer.trim().is_empty() {
                if let Some(message) = stream_error(buffer.trim()) {
                    let _ = tool_calls_tx.send(Err(OmonError::Llm(message.clone())));
                    let _ = sender.send(Err(OmonError::Llm(message))).await;
                    return;
                }
                terminated |= is_stream_terminator(provider, buffer.trim());
                accumulate_stream_tool_calls(provider, buffer.trim(), &mut tool_calls);
                for content in parse_stream_line(provider, buffer.trim()) {
                    let _ = sender
                        .send(Ok(StreamChunk {
                            stream_id,
                            sequence,
                            content,
                            is_final: false,
                            reply_to: None,
                        }))
                        .await;
                    sequence += 1;
                }
            }
            if !terminated {
                // A clean close without the provider's terminator is a truncated
                // reply (dropped connection or proxy cut); reporting it as complete
                // would let callers such as /compress persist a partial result.
                let message = "provider stream ended without a completion marker".to_string();
                let _ = tool_calls_tx.send(Err(OmonError::Llm(message.clone())));
                let _ = sender.send(Err(OmonError::Llm(message))).await;
                return;
            }
            let _ = tool_calls_tx.send(finish_stream_tool_calls(tool_calls));
            let _ = sender
                .send(Ok(StreamChunk {
                    stream_id,
                    sequence,
                    content: String::new(),
                    is_final: true,
                    reply_to: None,
                }))
                .await;
        });
        Ok((Box::pin(tokio_stream(receiver)), tool_calls_rx))
    }

    fn openai_payload(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        ollama: bool,
    ) -> Value {
        let include_vision = self.config.provider == LlmProvider::OpenAi && !ollama;
        let messages: Vec<Value> = messages
            .iter()
            .map(|message| openai_message(message, include_vision))
            .collect();
        let tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({"type": "function", "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema
                }})
            })
            .collect();
        let mut payload = Map::from_iter([
            ("model".into(), json!(self.config.model)),
            ("messages".into(), Value::Array(messages)),
            ("stream".into(), Value::Bool(true)),
        ]);
        if !tools.is_empty() {
            payload.insert("tools".into(), Value::Array(tools));
        }
        if ollama {
            payload.insert(
                "options".into(),
                json!({"temperature": self.config.temperature}),
            );
        } else {
            payload.insert("max_tokens".into(), json!(self.config.max_tokens));
            if let Some(temperature) = self.config.temperature {
                payload.insert("temperature".into(), json!(temperature));
            }
        }
        Value::Object(payload)
    }

    fn anthropic_payload(&self, messages: &[ChatMessage], tools: &[ToolDefinition]) -> Value {
        let system = messages
            .iter()
            .filter(|message| message.role == "system")
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let messages: Vec<Value> = messages
            .iter()
            .filter(|message| message.role != "system")
            .map(anthropic_message)
            .collect();
        let tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({"name": tool.name, "description": tool.description, "input_schema": tool.input_schema})
            })
            .collect();
        let mut payload = Map::from_iter([
            ("model".into(), json!(self.config.model)),
            ("messages".into(), Value::Array(messages)),
            ("max_tokens".into(), json!(self.config.max_tokens)),
            ("stream".into(), Value::Bool(true)),
        ]);
        if !system.is_empty() {
            payload.insert("system".into(), Value::String(system));
        }
        if !tools.is_empty() {
            payload.insert("tools".into(), Value::Array(tools));
        }
        if let Some(temperature) = self.config.temperature {
            payload.insert("temperature".into(), json!(temperature));
        }
        Value::Object(payload)
    }

    pub fn parse_tool_calls(&self, value: &Value) -> Result<Vec<ToolCall>, OmonError> {
        parse_tool_calls(self.config.provider, value)
    }
}

fn openai_message(message: &ChatMessage, include_vision: bool) -> Value {
    let images = if include_vision && message.role == "user" {
        message
            .attachments
            .iter()
            .filter_map(openai_image_block)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let content = if images.is_empty() {
        Value::String(message.content.clone())
    } else {
        let mut content =
            Vec::with_capacity(images.len() + usize::from(!message.content.is_empty()));
        if !message.content.is_empty() {
            content.push(json!({"type": "text", "text": message.content}));
        }
        content.extend(images);
        Value::Array(content)
    };
    let mut value = json!({"role": message.role, "content": content});
    if !message.tool_calls.is_empty() {
        value["tool_calls"] = Value::Array(
            message
                .tool_calls
                .iter()
                .map(|call| {
                    json!({"id": call.id, "type": "function", "function": {
                        "name": call.name, "arguments": call.arguments.to_string()
                    }})
                })
                .collect(),
        );
    }
    if let Some(id) = &message.tool_call_id {
        value["tool_call_id"] = json!(id);
    }
    value
}

fn anthropic_message(message: &ChatMessage) -> Value {
    let mut content = Vec::new();
    if message.role == "user" {
        content.extend(message.attachments.iter().filter_map(anthropic_image_block));
    }
    if !message.content.is_empty() && message.tool_call_id.is_none() {
        content.push(json!({"type": "text", "text": message.content}));
    }
    content.extend(message.tool_calls.iter().map(|call| {
        json!({"type": "tool_use", "id": call.id, "name": call.name, "input": call.arguments})
    }));
    if let Some(id) = &message.tool_call_id {
        content.push(json!({"type": "tool_result", "tool_use_id": id, "content": message.content}));
    }
    // Anthropic has no `tool` role: a tool result is a `tool_result` block in a user turn.
    let role = if message.tool_call_id.is_some() {
        "user"
    } else {
        message.role.as_str()
    };
    json!({"role": role, "content": content})
}

fn openai_image_block(attachment: &MessageAttachment) -> Option<Value> {
    let (media_type, data) = encoded_image(attachment)?;
    Some(json!({
        "type": "image_url",
        "image_url": {
            "url": format!("data:{media_type};base64,{data}")
        }
    }))
}

fn anthropic_image_block(attachment: &MessageAttachment) -> Option<Value> {
    let (media_type, data) = encoded_image(attachment)?;
    Some(json!({
        "type": "image",
        "source": {
            "type": "base64",
            "media_type": media_type,
            "data": data
        }
    }))
}

fn encoded_image(attachment: &MessageAttachment) -> Option<(&'static str, String)> {
    let media_type = image_media_type(attachment)?;
    let path = attachment.local_path.as_ref()?;
    let bytes = std::fs::read(path).ok()?;
    Some((media_type, BASE64_STANDARD.encode(bytes)))
}

fn image_media_type(attachment: &MessageAttachment) -> Option<&'static str> {
    let content_type = attachment
        .content_type
        .as_deref()
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    match content_type.as_deref() {
        Some("image/png") => Some("image/png"),
        Some("image/jpeg" | "image/jpg") => Some("image/jpeg"),
        Some("image/webp") => Some("image/webp"),
        Some("image/gif") => Some("image/gif"),
        _ => {
            let path = attachment
                .local_path
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new(&attachment.filename));
            match path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("png") => Some("image/png"),
                Some("jpg" | "jpeg") => Some("image/jpeg"),
                Some("webp") => Some("image/webp"),
                Some("gif") => Some("image/gif"),
                _ => None,
            }
        }
    }
}

#[derive(Default)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
    value: Option<Value>,
}

fn stream_value(line: &str) -> Option<Value> {
    let data = line
        .strip_prefix("data:")
        .map(str::trim)
        .unwrap_or(line.trim());
    if data.is_empty() || data == "[DONE]" || data.starts_with(':') {
        return None;
    }
    serde_json::from_str(data).ok()
}

/// Provider error frame delivered inside an otherwise successful stream:
/// Anthropic `{"type":"error","error":{...}}` or an OpenAI-compatible /
/// Ollama top-level `"error"` field.
fn stream_error(line: &str) -> Option<String> {
    let value = stream_value(line)?;
    let error = value.get("error")?;
    if error.is_null() {
        return None;
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| error.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| error.to_string());
    Some(format!("provider stream error: {message}"))
}

/// Whether a stream line is the provider's end-of-reply marker.
fn is_stream_terminator(provider: LlmProvider, line: &str) -> bool {
    let data = line
        .strip_prefix("data:")
        .map(str::trim)
        .unwrap_or(line.trim());
    if data == "[DONE]" {
        return true;
    }
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return false;
    };
    match provider {
        LlmProvider::Anthropic => value.get("type").and_then(Value::as_str) == Some("message_stop"),
        LlmProvider::Ollama => value.get("done").and_then(Value::as_bool) == Some(true),
        LlmProvider::OpenAi | LlmProvider::DeepSeek => value
            .pointer("/choices/0/finish_reason")
            .is_some_and(|reason| !reason.is_null()),
    }
}

fn accumulate_stream_tool_calls(
    provider: LlmProvider,
    line: &str,
    calls: &mut BTreeMap<usize, PendingToolCall>,
) {
    let Some(value) = stream_value(line) else {
        return;
    };
    match provider {
        LlmProvider::Anthropic => {
            let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
            if value.get("type").and_then(Value::as_str) == Some("content_block_start") {
                let block = &value["content_block"];
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    let call = calls.entry(index).or_default();
                    call.id = block
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    call.name = block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    call.value = block.get("input").cloned();
                }
            }
            if let Some(fragment) = value.pointer("/delta/partial_json").and_then(Value::as_str) {
                calls.entry(index).or_default().arguments.push_str(fragment);
            }
        }
        LlmProvider::OpenAi | LlmProvider::DeepSeek => {
            let Some(items) = value
                .pointer("/choices/0/delta/tool_calls")
                .and_then(Value::as_array)
            else {
                return;
            };
            for (position, item) in items.iter().enumerate() {
                let index = item
                    .get("index")
                    .and_then(Value::as_u64)
                    .map(|index| index as usize)
                    .unwrap_or(position);
                let call = calls.entry(index).or_default();
                if let Some(id) = item.get("id").and_then(Value::as_str) {
                    call.id = id.to_owned();
                }
                // Providers differ: some send the name once, others repeat it on every
                // delta. Assigning (not appending) is right for both.
                if let Some(name) = item
                    .pointer("/function/name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                {
                    call.name = name.to_owned();
                }
                if let Some(arguments) = item.pointer("/function/arguments").and_then(Value::as_str)
                {
                    call.arguments.push_str(arguments);
                }
            }
        }
        LlmProvider::Ollama => {
            let Some(items) = value
                .pointer("/message/tool_calls")
                .and_then(Value::as_array)
            else {
                return;
            };
            // Ollama delivers each tool call whole, possibly across several chunks;
            // append after the calls already seen instead of overwriting index 0.
            for item in items {
                let function = item.get("function").unwrap_or(item);
                let call = calls.entry(calls.len()).or_default();
                call.id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                call.name = function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                call.value = function.get("arguments").cloned();
            }
        }
    }
}

fn finish_stream_tool_calls(
    calls: BTreeMap<usize, PendingToolCall>,
) -> Result<Vec<ToolCall>, OmonError> {
    calls
        .into_iter()
        .map(|(index, call)| {
            let arguments = if call.arguments.is_empty() {
                call.value.unwrap_or_else(|| json!({}))
            } else {
                serde_json::from_str(&call.arguments).map_err(|error| {
                    OmonError::Llm(format!("invalid streamed tool arguments: {error}"))
                })?
            };
            Ok(ToolCall {
                id: if call.id.is_empty() {
                    format!("tool-{index}")
                } else {
                    call.id
                },
                name: call.name,
                arguments,
            })
        })
        .collect()
}

fn parse_stream_line(provider: LlmProvider, line: &str) -> Vec<String> {
    let data = line
        .strip_prefix("data:")
        .map(str::trim)
        .unwrap_or(line.trim());
    if data.is_empty() || data == "[DONE]" || data.starts_with(':') {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    match provider {
        LlmProvider::Anthropic => value
            .pointer("/delta/text")
            .and_then(Value::as_str)
            .map(|text| vec![text.to_owned()])
            .unwrap_or_default(),
        LlmProvider::Ollama => value
            .pointer("/message/content")
            .and_then(Value::as_str)
            .map(|text| vec![text.to_owned()])
            .unwrap_or_default(),
        LlmProvider::OpenAi | LlmProvider::DeepSeek => value
            .pointer("/choices/0/delta/content")
            .and_then(Value::as_str)
            .map(|text| vec![text.to_owned()])
            .unwrap_or_default(),
    }
}

fn parse_tool_calls(provider: LlmProvider, value: &Value) -> Result<Vec<ToolCall>, OmonError> {
    let raw: Vec<Value> = match provider {
        LlmProvider::Anthropic => value
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("tool_use"))
            .cloned()
            .collect(),
        LlmProvider::Ollama => value
            .pointer("/message/tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        LlmProvider::OpenAi | LlmProvider::DeepSeek => value
            .pointer("/choices/0/message/tool_calls")
            .or_else(|| value.get("tool_calls"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    };

    raw.iter()
        .enumerate()
        .map(|(index, item)| {
            if provider == LlmProvider::Anthropic {
                return Ok(ToolCall {
                    id: item
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    arguments: item.get("input").cloned().unwrap_or(Value::Null),
                });
            }
            let function = item.get("function").unwrap_or(item);
            let arguments = function.get("arguments").cloned().unwrap_or(Value::Null);
            let arguments = match arguments {
                Value::String(value) => serde_json::from_str(&value)
                    .map_err(|error| OmonError::Llm(format!("invalid tool arguments: {error}")))?,
                value => value,
            };
            Ok(ToolCall {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("tool-{index}")),
                name: function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                arguments,
            })
        })
        .collect()
}

/// Appends every complete UTF-8 character available in `pending` to `buffer`,
/// keeping an incomplete trailing sequence in `pending` for the next chunk so
/// multi-byte characters split across chunk boundaries survive intact.
fn decode_stream_bytes(pending: &mut Vec<u8>, buffer: &mut String) {
    loop {
        let error = match std::str::from_utf8(pending) {
            Ok(text) => {
                buffer.push_str(text);
                pending.clear();
                return;
            }
            Err(error) => error,
        };
        let valid = error.valid_up_to();
        if let Ok(text) = std::str::from_utf8(&pending[..valid]) {
            buffer.push_str(text);
        }
        match error.error_len() {
            // Genuinely invalid bytes: replace them and keep decoding.
            Some(length) => {
                buffer.push('\u{FFFD}');
                pending.drain(..valid + length);
            }
            // Truncated trailing character: keep it for the next chunk.
            None => {
                pending.drain(..valid);
                return;
            }
        }
    }
}

fn tokio_stream<T: Send + 'static>(
    receiver: mpsc::Receiver<T>,
) -> impl Stream<Item = T> + Send + 'static {
    futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    })
}

#[cfg(test)]
mod stream_decode_tests {
    use super::*;

    #[test]
    fn multibyte_split_across_chunks_decodes_intact() {
        let original = "안녕하세요 오몬! 스트리밍 테스트입니다.";
        let bytes = original.as_bytes();
        // Split in the middle of a multi-byte character.
        let split = 4;
        assert!(!original.is_char_boundary(split));

        let mut pending = Vec::new();
        let mut buffer = String::new();
        pending.extend_from_slice(&bytes[..split]);
        decode_stream_bytes(&mut pending, &mut buffer);
        pending.extend_from_slice(&bytes[split..]);
        decode_stream_bytes(&mut pending, &mut buffer);

        assert!(
            !buffer.contains('\u{FFFD}'),
            "decoded stream contains replacement characters: {buffer:?}"
        );
        assert_eq!(buffer, original);
        assert!(pending.is_empty());
    }

    #[test]
    fn llm_config_debug_redacts_api_key() {
        let mut config = LlmConfig::new(LlmProvider::OpenAi, "glm-5.3-flash");
        config.api_key = Some("sk-super-secret-value".to_string());
        let rendered = format!("{config:?}");
        assert!(
            !rendered.contains("sk-super-secret-value"),
            "debug output leaked the api key: {rendered}"
        );
        assert!(rendered.contains("glm-5.3-flash"), "{rendered}");
    }
}

#[cfg(test)]
mod fixr1_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serves one canned SSE response to a single request, then closes.
    async fn serve_sse_once(body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                request.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&request).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text[..end]
                        .lines()
                        .find_map(|line| {
                            let lower = line.to_ascii_lowercase();
                            lower
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn collect(
        provider: LlmProvider,
        body: &'static str,
    ) -> Vec<Result<StreamChunk, OmonError>> {
        let mut config = LlmConfig::new(provider, "m");
        config.base_url = Some(serve_sse_once(body).await);
        let client = LlmClient::new(config).unwrap();
        let stream = client
            .stream(&[ChatMessage::new("user", "hi")], &[])
            .await
            .unwrap();
        stream.collect().await
    }

    fn assert_error_without_final(items: &[Result<StreamChunk, OmonError>], needle: &str) {
        assert!(
            items
                .iter()
                .all(|item| !matches!(item, Ok(chunk) if chunk.is_final)),
            "error frame must not be followed by a normal final chunk: {items:?}"
        );
        assert!(
            matches!(items.last(), Some(Err(OmonError::Llm(message))) if message.contains(needle)),
            "expected trailing OmonError::Llm containing {needle:?}: {items:?}"
        );
    }

    #[tokio::test]
    async fn fixr1_anthropic_in_stream_error_frame_yields_err() {
        let items = collect(
            LlmProvider::Anthropic,
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"par\"}}\n\nevent: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n",
        )
        .await;
        assert!(matches!(items.first(), Some(Ok(chunk)) if chunk.content == "par"));
        assert_error_without_final(&items, "Overloaded");
    }

    #[tokio::test]
    async fn fixr1_openai_in_stream_error_frame_yields_err() {
        let items = collect(
            LlmProvider::OpenAi,
            "data: {\"error\":{\"message\":\"rate limited upstream\",\"type\":\"rate_limit\"}}\n\n",
        )
        .await;
        assert_error_without_final(&items, "rate limited upstream");
    }

    #[tokio::test]
    async fn fixr1_normal_stream_still_ends_with_final_chunk() {
        let items = collect(
            LlmProvider::OpenAi,
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n",
        )
        .await;
        assert!(matches!(items.first(), Some(Ok(chunk)) if chunk.content == "ok"));
        assert!(matches!(items.last(), Some(Ok(chunk)) if chunk.is_final));
    }

    #[tokio::test]
    async fn fixr5_stream_closed_without_terminator_is_an_error() {
        for (provider, body) in [
            (
                LlmProvider::OpenAi,
                "data: {\"choices\":[{\"delta\":{\"content\":\"par\"}}]}\n\n",
            ),
            (
                LlmProvider::Anthropic,
                "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"par\"}}\n\n",
            ),
            (
                LlmProvider::Ollama,
                "{\"message\":{\"content\":\"par\"},\"done\":false}\n",
            ),
        ] {
            let items = collect(provider, body).await;
            assert!(matches!(items.first(), Some(Ok(chunk)) if chunk.content == "par"));
            assert_error_without_final(&items, "without a completion marker");
        }
    }

    #[tokio::test]
    async fn fixr5_provider_specific_terminators_complete_the_stream() {
        for (provider, body) in [
            (
                LlmProvider::OpenAi,
                "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\n",
            ),
            (
                LlmProvider::Anthropic,
                "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
            ),
            (
                LlmProvider::Ollama,
                "{\"message\":{\"content\":\"ok\"},\"done\":true}\n",
            ),
        ] {
            let items = collect(provider, body).await;
            assert!(matches!(items.first(), Some(Ok(chunk)) if chunk.content == "ok"));
            assert!(
                matches!(items.last(), Some(Ok(chunk)) if chunk.is_final),
                "{provider:?}: {items:?}"
            );
        }
    }

    #[test]
    fn fixr5_openai_repeated_tool_name_is_not_concatenated() {
        let mut calls = BTreeMap::new();
        for line in [
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"foo","arguments":"{\"a\""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"foo","arguments":":1}"}}]}}]}"#,
        ] {
            accumulate_stream_tool_calls(LlmProvider::OpenAi, line, &mut calls);
        }
        let calls = finish_stream_tool_calls(calls).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "foo");
        assert_eq!(calls[0].arguments["a"], 1);
    }

    #[test]
    fn fixr5_ollama_tool_calls_across_chunks_are_all_kept() {
        let mut calls = BTreeMap::new();
        for line in [
            r#"{"message":{"tool_calls":[{"function":{"name":"first","arguments":{"x":1}}}]}}"#,
            r#"{"message":{"tool_calls":[{"function":{"name":"second","arguments":{"y":2}}}]}}"#,
        ] {
            accumulate_stream_tool_calls(LlmProvider::Ollama, line, &mut calls);
        }
        let names: Vec<_> = finish_stream_tool_calls(calls)
            .unwrap()
            .into_iter()
            .map(|call| call.name)
            .collect();
        assert_eq!(names, vec!["first", "second"]);
    }

    #[test]
    fn fixr5_anthropic_tool_result_is_a_user_message() {
        let mut message = ChatMessage::new("tool", "42");
        message.tool_call_id = Some("call-1".into());
        let value = anthropic_message(&message);
        assert_eq!(value["role"], "user");
        assert_eq!(value["content"][0]["type"], "tool_result");
        assert_eq!(value["content"][0]["tool_use_id"], "call-1");
        assert_eq!(value["content"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn fixr5_credentials_are_marked_sensitive() {
        for provider in [LlmProvider::OpenAi, LlmProvider::Anthropic] {
            let mut config = LlmConfig::new(provider, "m");
            config.api_key = Some("sk-super-secret-value".into());
            let client = LlmClient::new(config).unwrap();
            let rendered = format!("{:?}", client.http);
            assert!(
                !rendered.contains("sk-super-secret-value"),
                "{provider:?} client debug leaked the api key: {rendered}"
            );
        }
    }

    #[test]
    fn fixr1_http_client_has_read_timeout_and_no_total_timeout() {
        let client = LlmClient::new(LlmConfig::new(LlmProvider::OpenAi, "m")).unwrap();
        let rendered = format!("{:?}", client.http);
        assert!(rendered.contains("read_timeout: 120s"), "{rendered}");
        assert!(!rendered.contains(" timeout:"), "{rendered}");
    }
}

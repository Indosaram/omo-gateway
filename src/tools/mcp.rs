use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use super::Tool;
use crate::OmonError;

#[derive(Clone, Debug)]
pub enum McpTransport {
    Stdio {
        program: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    },
    Sse {
        url: String,
        bearer_token: Option<String>,
    },
}

#[derive(Clone)]
pub struct McpClientTool {
    name: String,
    remote_tool: String,
    description: String,
    input_schema: Value,
    transport: McpTransport,
    timeout: Duration,
    next_id: Arc<AtomicU64>,
    http: reqwest::Client,
    stdio_lock: Arc<Mutex<()>>,
    pub requires_approval: bool,
}

impl McpClientTool {
    pub fn new(
        name: impl Into<String>,
        remote_tool: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        transport: McpTransport,
    ) -> Self {
        Self {
            name: name.into(),
            remote_tool: remote_tool.into(),
            description: description.into(),
            input_schema,
            transport,
            timeout: Duration::from_secs(600),
            next_id: Arc::new(AtomicU64::new(1)),
            http: reqwest::Client::new(),
            stdio_lock: Arc::new(Mutex::new(())),
            requires_approval: false,
        }
    }

    pub fn with_approval(mut self, enabled: bool) -> Self {
        self.requires_approval = enabled;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn request(&self, params: Value) -> Result<Value, OmonError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": self.remote_tool, "arguments": params}
        });
        let response = match &self.transport {
            McpTransport::Stdio { program, args, cwd } => {
                let _guard = self.stdio_lock.lock().await;
                self.request_stdio(program, args, cwd.as_ref(), &request)
                    .await?
            }
            McpTransport::Sse { url, bearer_token } => {
                self.request_sse(url, bearer_token.as_deref(), &request)
                    .await?
            }
        };
        decode_response(response, id)
    }

    async fn request_stdio(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&PathBuf>,
        request: &Value,
    ) -> Result<Value, OmonError> {
        let mut command = Command::new(program);
        let augmented_path = crate::tools::augmented_path_from_environment();
        if !augmented_path.is_empty() {
            command.env("PATH", augmented_path);
        }
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        // Own process group so wrapper launchers (npx, uv run, sh -c) and their
        // children are killed together with the server.
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn().map_err(mcp_error)?;
        let group_id = child.id();
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| mcp_error("MCP stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| mcp_error("MCP stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| mcp_error("MCP stderr unavailable"))?;
        let encoded = serde_json::to_vec(request).map_err(mcp_error)?;
        stdin.write_all(&encoded).await.map_err(mcp_error)?;
        stdin.write_all(b"\n").await.map_err(mcp_error)?;
        stdin.shutdown().await.map_err(mcp_error)?;

        // MCP servers frequently log to stderr. If that pipe is not drained, a
        // verbose server can fill the OS pipe buffer and block before it writes
        // its JSON-RPC response to stdout. Drain to a sink so logging cannot
        // deadlock the protocol and cannot become an unbounded memory buffer.
        let stderr_task = tokio::spawn(async move {
            let mut stderr = BufReader::new(stderr);
            let mut sink = tokio::io::sink();
            tokio::io::copy(&mut stderr, &mut sink).await
        });

        let read = read_stdio_response(stdout);
        // A timeout must not return early: the child is killed on every exit path.
        let result = tokio::time::timeout(self.timeout, read)
            .await
            .unwrap_or_else(|_| Err(mcp_error("MCP stdio request timed out")));

        // Each request owns a fresh stdio server process. Terminate anything
        // that remains after the response so helpers/servers cannot accumulate.
        // A grandchild may still hold the stderr pipe open, so the drain task is
        // aborted rather than awaited (awaiting could hang while holding stdio_lock).
        #[cfg(unix)]
        if let Some(pgid) = group_id {
            // SAFETY: killpg only sends a signal; the group was created above.
            unsafe {
                libc::killpg(pgid as libc::pid_t, libc::SIGKILL);
            }
        }
        let _ = group_id;
        let _ = child.kill().await;
        stderr_task.abort();
        result
    }

    async fn request_sse(
        &self,
        url: &str,
        bearer_token: Option<&str>,
        request: &Value,
    ) -> Result<Value, OmonError> {
        let mut builder = self
            .http
            .post(url)
            .header("accept", "application/json, text/event-stream")
            .json(request);
        if let Some(token) = bearer_token {
            builder = builder.bearer_auth(token);
        }
        let response = tokio::time::timeout(self.timeout, builder.send())
            .await
            .map_err(|_| mcp_error("MCP HTTP request timed out"))?
            .map_err(mcp_error)?;
        if !response.status().is_success() {
            return Err(mcp_error(format!(
                "MCP server returned {}",
                response.status()
            )));
        }
        if response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("text/event-stream"))
        {
            return parse_sse_response(response, self.timeout).await;
        }
        let body = read_bounded_body(response.bytes_stream()).await?;
        serde_json::from_slice(&body).map_err(mcp_error)
    }
}

#[derive(Clone, Default)]
pub struct McpTool {
    clients: Vec<McpClientTool>,
}

impl McpTool {
    pub fn new(clients: Vec<McpClientTool>) -> Self {
        Self { clients }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Call a configured Model Context Protocol tool"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "tool": {"type": "string"},
                "arguments": {"type": "object"}
            },
            "required": ["tool"]
        })
    }

    fn requires_approval(&self, args: &Value) -> Option<String> {
        let tool_name = args
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let client = self.clients.iter().find(|client| client.name == tool_name);
        if let Some(client) = client {
            if client.requires_approval {
                // The registry hashes this reason; bind it to the registered client.
                // JSON keeps the client/method pair unambiguous.
                return Some(format!(
                    "MCP external tool execution: {}",
                    json!([client.name, client.remote_tool])
                ));
            }
        }
        None
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        let name = args
            .get("tool")
            .and_then(Value::as_str)
            .ok_or_else(|| mcp_error("missing string argument: tool"))?;
        let client = self
            .clients
            .iter()
            .find(|client| client.name() == name)
            .ok_or_else(|| mcp_error(format!("unknown MCP tool: {name}")))?;
        client
            .execute(args.get("arguments").cloned().unwrap_or_else(|| json!({})))
            .await
    }
}

#[async_trait]
impl Tool for McpClientTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    fn requires_approval(&self, _args: &Value) -> Option<String> {
        if self.requires_approval {
            Some(format!(
                "MCP external tool execution: '{}'",
                self.remote_tool
            ))
        } else {
            None
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        self.request(args).await
    }
}

/// Pre-decode byte limits. A single frame (stdio line or SSE event) and the
/// aggregate response body are both bounded so a hostile or broken MCP server
/// cannot force unbounded buffering.
const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

async fn read_stdio_response<R>(reader: R) -> Result<Value, OmonError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(reader.take(MAX_RESPONSE_BYTES as u64));
    let mut line = Vec::new();
    loop {
        line.clear();
        // Bound the read itself: a server that never emits a newline must not be
        // able to grow this buffer without limit.
        let read = (&mut reader)
            .take(MAX_FRAME_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)
            .await
            .map_err(mcp_error)?;
        if read == 0 {
            return Err(mcp_error("MCP server closed without a JSON-RPC response"));
        }
        if read > MAX_FRAME_BYTES {
            return Err(mcp_error("MCP stdio line exceeded byte limit"));
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&line) {
            return Ok(value);
        }
    }
}

async fn read_bounded_body<S, B, E>(stream: S) -> Result<Vec<u8>, OmonError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    let mut stream = std::pin::pin!(stream);
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(mcp_error)?;
        if body.len() + chunk.as_ref().len() > MAX_RESPONSE_BYTES {
            return Err(mcp_error("MCP response exceeded byte limit"));
        }
        body.extend_from_slice(chunk.as_ref());
    }
    Ok(body)
}

async fn parse_sse_response(
    response: reqwest::Response,
    timeout: Duration,
) -> Result<Value, OmonError> {
    let read = parse_sse_stream(response.bytes_stream());
    tokio::time::timeout(timeout, read)
        .await
        .map_err(|_| mcp_error("MCP SSE response timed out"))?
}

async fn parse_sse_stream<S, B, E>(stream: S) -> Result<Value, OmonError>
where
    S: futures_util::Stream<Item = Result<B, E>>,
    B: AsRef<[u8]>,
    E: std::fmt::Display,
{
    let mut stream = std::pin::pin!(stream);
    // Raw bytes are accumulated and decoded only at character boundaries: a
    // multi-byte UTF-8 sequence split across two network chunks must not be
    // replaced with U+FFFD and returned as a successful response.
    let mut pending: Vec<u8> = Vec::new();
    let mut buffer = String::new();
    let mut consumed = 0usize;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(mcp_error)?;
        consumed += chunk.as_ref().len();
        if consumed > MAX_RESPONSE_BYTES {
            return Err(mcp_error("MCP SSE response exceeded byte limit"));
        }
        pending.extend_from_slice(chunk.as_ref());
        let decodable = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            Err(error) => match error.error_len() {
                // Invalid bytes: decode lossily up to and including them.
                Some(len) => error.valid_up_to() + len,
                // Incomplete trailing sequence: keep it for the next chunk.
                None => error.valid_up_to(),
            },
        };
        buffer.push_str(&String::from_utf8_lossy(&pending[..decodable]));
        pending.drain(..decodable);
        while let Some(index) = buffer.find('\n') {
            let line = buffer[..index].trim_end_matches('\r').to_owned();
            buffer.drain(..=index);
            if let Some(data) = line.strip_prefix("data:").map(str::trim) {
                if let Ok(value) = serde_json::from_str::<Value>(data) {
                    return Ok(value);
                }
            }
        }
        if buffer.len() + pending.len() > MAX_FRAME_BYTES {
            return Err(mcp_error("MCP SSE frame exceeded byte limit"));
        }
    }
    Err(mcp_error(
        "MCP SSE stream closed without a JSON-RPC response",
    ))
}

fn decode_response(response: Value, id: u64) -> Result<Value, OmonError> {
    if response.get("id").and_then(Value::as_u64) != Some(id) {
        return Err(mcp_error("MCP response ID did not match request"));
    }
    if let Some(error) = response.get("error") {
        return Err(mcp_error(format!("MCP JSON-RPC error: {error}")));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| mcp_error("MCP response omitted result"))
}

fn mcp_error(error: impl std::fmt::Display) -> OmonError {
    OmonError::ToolExecution(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sse_stream_decodes_multibyte_utf8_split_across_chunks() {
        let payload = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"text\":\"한글\"}}\n";
        // Split in the middle of the first multi-byte character.
        let split = payload.find('한').expect("payload contains 한") + 1;
        let bytes = payload.as_bytes();
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> =
            vec![Ok(bytes[..split].to_vec()), Ok(bytes[split..].to_vec())];

        let value = parse_sse_stream(futures_util::stream::iter(chunks))
            .await
            .expect("split multi-byte SSE frame must decode");

        assert_eq!(value["result"]["text"], json!("한글"));
    }

    #[tokio::test]
    async fn sse_stream_rejects_oversized_frame() {
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> = (0..(MAX_FRAME_BYTES / 1024 + 2))
            .map(|_| Ok(vec![b'x'; 1024]))
            .collect();

        let error = parse_sse_stream(futures_util::stream::iter(chunks))
            .await
            .expect_err("an unbounded SSE frame must be rejected");

        assert!(
            error.to_string().contains("exceeded byte limit"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn json_body_stream_is_bounded() {
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> = (0..(MAX_RESPONSE_BYTES / 65536 + 2))
            .map(|_| Ok(vec![b'x'; 65536]))
            .collect();

        let error = read_bounded_body(futures_util::stream::iter(chunks))
            .await
            .expect_err("an oversized JSON body must be rejected");

        assert!(
            error.to_string().contains("exceeded byte limit"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn stdio_line_is_bounded() {
        let mut payload = vec![b'x'; MAX_FRAME_BYTES + 1024];
        payload.push(b'\n');

        let error = read_stdio_response(payload.as_slice())
            .await
            .expect_err("an oversized stdio line must be rejected");

        assert!(
            error.to_string().contains("exceeded byte limit"),
            "unexpected error: {error}"
        );
    }
}

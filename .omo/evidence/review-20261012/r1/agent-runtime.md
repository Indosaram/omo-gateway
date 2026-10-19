# agent-runtime review

### [P1] Autospawned omo daemon is started with `--ws-auth off`, so a configured auth token is silently ignored
- Location: src/agent/omo_daemon.rs:162 (`cmd.args(["app-server", "--listen", listen_url, "--ws-auth", "off"])`)
- Trigger: The operator sets `OMON_OMO_APPSERVER_AUTH_TOKEN` (or `OMON_OMO_AUTH_TOKEN`) and leaves autospawn on, which is the default. `src/main.rs:844` and `src/dashboard_runtime.rs:123` call `OmoDaemonSupervisor::ensure`. `daemon_command` (called from `ensure` and the watcher restart path) hardcodes auth off and never looks at `cfg.auth_token`. The backend still sends `Authorization: Bearer <token>` (omo_backend.rs:130), and an auth-off daemon ignores it.
- Impact: The daemon runs agents that can execute commands. It accepts any WebSocket client on the loopback port, including other local processes and any local web page that opens `ws://127.0.0.1:19742`. This holds unless the omo daemon checks the Origin header, which I could not verify. The operator believes the token protects the daemon, and it does not.
- Fix: Give `daemon_command` the `OmoBackendConfig` and only pass `--ws-auth off` when `auth_token` is `None`. When a token is set, start the daemon in its token-auth mode, or refuse to autospawn and log why.

### [P1] LLM HTTP client has no timeout, so a stalled provider hangs callers and leaks the stream task
- Location: src/agent/llm.rs:167 (`let http = reqwest::Client::builder()`; no `connect_timeout`, `timeout` or `read_timeout`)
- Trigger: The provider accepts the connection and then stalls, or stops sending bytes mid-stream. Both `.send().await` (llm.rs:~200) and `bytes.next().await` (the spawned reader loop in `stream_with_tool_calls`) wait forever. Callers on this path:
  - `Draft` in src/inbox/proposals.rs:146, reached from `execute_approved` at proposals.rs:452.
  - `/compress` in src/discord/commands.rs:1079.
- Impact:
  - For an approved Draft proposal, the row was already committed as `state='running'` at proposals.rs:430, before the effect runs. The approval call never returns, and the proposal and its decision stay "running" indefinitely.
  - The `/compress` interaction hangs.
  - The spawned reader task and the connection stay alive until the process exits. The task only exits when the receiver is dropped and a send fails, and a stalled stream never reaches a send.
- Fix: Build the client with `.connect_timeout(..)` and `.read_timeout(..)`, for example 30s and 120s. A total timeout would cut off long streams, so avoid one. Alternatively wrap `stream_with_tool_calls` and the stream consumers in `tokio::time::timeout`.

### [P1] In-stream provider error events are dropped, so a failed generation looks like a successful (possibly empty or partial) reply
- Location: src/agent/llm.rs:647 (`fn parse_stream_line`; only content deltas are extracted and `error` payloads fall through to `Vec::new()`)
- Trigger: The provider returns HTTP 200 and then sends an error in the SSE body. Anthropic does this with `data: {"type":"error","error":{"type":"overloaded_error",...}}`, and OpenAI-compatible proxies send `data: {"error":{...}}`. `parse_stream_line` and `accumulate_stream_tool_calls` ignore these frames, and the task then emits a normal `is_final: true` chunk.
- Impact:
  - `ReadOnlyDirect::draft` (proposals.rs:146-150) returns `Ok("")` or a truncated draft. The proposal is recorded as `completed` and the owner gets an empty or partial draft.
  - `/compress` (commands.rs:1079-1086) accepts a partial summary. `replace_history_with_summary` then deletes the original history.
- Fix: In the stream task, detect `value["type"] == "error"` or a top-level `value["error"]` for every provider. Send `Err(OmonError::Llm(..))` on the channel, then return without the final chunk.

### [P2] Zero is accepted for timeout env vars that are documented as "positive integer"
- Location: src/agent/omo_config.rs:252 (`v.trim().parse::<u64>().map_err(|_| {` in the `OMON_OMO_TURN_TOTAL_TIMEOUT_SECS` branch). The same pattern is at omo_config.rs:237 (`OMON_OMO_TURN_TIMEOUT_SECS`), :328 (`OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS`) and :354 (`OMON_LANE_TURN_SECS`).
- Trigger: The env var is set to `0` (for example `OMON_OMO_TURN_TOTAL_TIMEOUT_SECS=0`). `from_env` accepts it and returns `Duration::ZERO`. In `OmoBackend::run` the deadline is then `now + 0` (omo_backend.rs, `effective_total_timeout`), and `setup_turn` fails with "turn exceeded total deadline" before doing any work.
- Impact: Every interactive, cron or lane turn fails immediately. Boot succeeds and the error text promises "positive integer" validation, so the misconfiguration is not caught.
- Fix: Reject `0` after parsing with `.filter(|n| *n > 0)` or an explicit check, using the existing error message.

### [P2] Workspace slugs collide for IDs that differ only by ASCII case
- Location: src/agent/agent_workspace.rs:57 (`if sanitized != raw_lower || initial_slug.len() > 48 {`)
- Trigger: `sanitize` lowercases ASCII, and `raw_lower` is also lowercase. For raw values `Digest` and `digest`, `sanitized == raw_lower`, so the hash suffix is skipped. Both map to `cron-digest` and share `agents/cron-digest/`, including the `memory.agent` slug written by `OmoBackend::run` at omo_backend.rs:~396-440. The callers are `agent_workspace_slug(&session.key.platform, &session.key.user_id, session.key.bot_id.as_deref())`. This needs case-differing user, cron-job or bot ids. Discord snowflakes cannot differ by case, so only cron job IDs and similar string IDs are affected.
- Impact: Two distinct cron jobs or users with case-differing IDs share one cwd, one `.omo/omo.json` and one agent memory namespace.
- Fix: Compare against the original string, not its lowercase: use `sanitized != raw_value` instead of `sanitized != raw_lower`. The hash is then added whenever case was folded.

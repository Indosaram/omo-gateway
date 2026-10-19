# agent-runtime review
### [P1] Daemon supervisor gives up permanently after a short outage or one slow restart, and nothing surfaces it
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:405 (restart circuit), :443 (unready deadline), :283 (`is_available`)
- Trigger: A restart attempt gets only a 2s backoff, so three failed spawns exhaust the budget within about 8s. Two realistic ways to hit it:
  - The `omo` binary is briefly missing (ENOENT) while `bun add -g omo` replaces it, or the daemon crashes on a transient fault.
  - One replacement daemon takes longer than `READY_WAIT_AFTER_SPAWN` (30s) to answer `/readyz`, for example under load after wake. That path kills it and sets `shutdown` at :443.
  Either path sets `shutdown=true` and the watcher returns. Nothing ever retries.
- Impact: The agent backend stays down until someone restarts the gateway by hand. The gateway process keeps running, so launchd never restarts it. `is_available()` is never read outside tests, and every `ensure()` caller binds the supervisor to `_daemon_supervisor` (main.rs:899, dashboard_runtime.rs:129). The failure is silent apart from one error log. Every turn then fails with connect errors.
- Fix: Do not make the give-up terminal. Either keep retrying with capped exponential backoff, or on circuit-open make the gateway exit non-zero so launchd restarts it. If the state must stay, have the health and readiness checks consume `is_available()`. Count only post-ready crashes against the budget, and treat spawn ENOENT and slow starts separately.

### [P2] `OmoBackendConfig` derives `Debug` and `Serialize` with a plaintext `auth_token`
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_config.rs:38 (derive) and :41 (`auth_token`)
- Trigger: Any `{:?}`, `dbg!`, panic payload or serde dump of the config or of a struct holding it. `LlmConfig` already got a redacting `Debug` for exactly this reason, but this struct did not. I found no current logging call.
- Impact: Latent leak of the app-server bearer token, which grants tool execution.
- Fix: Replace the derived `Debug` with a manual impl that masks `auth_token`. Add `#[serde(skip_serializing)]` to the field, or drop `Serialize` if nothing uses it.

### [P2] Lane, kibitzer, triage and judge-fallback backends never get a default workspace root
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_config.rs:297; callers /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs:149, /Users/indo/code/project/omon-gateway/src/judge/mod.rs:710, /Users/indo/code/project/omon-gateway/src/main.rs:2028
- Trigger: `OMON_WORKSPACE_ROOT` is unset, which is the default deployment. main.rs:153 computes `~/.omon/workspace` locally but never exports it. These callers use `from_env()` directly, so `workspace_root` is `None` and `resolve_thread_id` skips `cwd`, `runtimeWorkspaceRoots` and the `.omo/omo.json` provisioning.
- Impact: Those sessions start in the daemon's cwd with no per-agent isolation and no memory-agent config, unlike the main and cron backends. Exposure is limited because they set `enabled_toolsets` to an empty list.
- Fix: Apply the same `~/.omon/workspace` default inside `from_env()`, or pass `with_workspace_root` at those call sites. Also reject an empty `OMON_WORKSPACE_ROOT` (`PathBuf::from("")`).

### [P2] `wipe_omo_thread_bindings` is never called, and would be unsafe to call as written
- Location: /Users/indo/code/project/omon-gateway/src/agent/workspace_migration.rs:13
- Trigger: It is only defined, re-exported (agent/mod.rs:23) and tested. The plan's boot migration (.omo/plans/daemon-multiplexer-agent-workspaces.md task 4) is marked done but is not wired in. Sessions that already have `omo_thread_id` therefore resume their legacy global-workspace threads, because the resume path passes no `cwd`.
- Impact: Per-agent isolation applies only to new threads. If someone later wires it as a plain boot call, it wipes every thread binding on every restart, because it has no run-once ledger. A single row with malformed `state_json` also makes `json_extract` abort the whole UPDATE.
- Fix: Wire it once behind a migration-ledger row, run before any actor loads sessions. Guard the WHERE clause with `json_valid(state_json)`.

### [P2] Autospawned daemon log and token directory use default permissions
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:244 (log open), :164 (`create_dir_all` in `write_token_file`)
- Trigger: The log is opened with `create(true).append(true)` and no mode, so it is 0644 under the common umask. `~/.omon` is created with the default mode. The log is never rotated.
- Impact: Daemon stdout and stderr, which can contain prompts and tool output, are readable by other local users. The log also grows without bound. The token file itself is correctly 0600.
- Fix: Open the log with `OpenOptionsExt::mode(0o600)` and create `~/.omon` as 0700. Add simple size-based rotation or truncate on spawn.

### [P2] Zero-config daemon runs with `--ws-auth off` on loopback
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:228
- Trigger: No token is configured, which is the default. The daemon accepts unauthenticated websockets on 127.0.0.1. Any local process can connect, and a web page in the operator's browser could attempt a cross-origin websocket to it. I could not verify whether `omo` checks the Origin header.
- Impact: A connecting client can start threads and run tools on the daemon. The backend auto-approves tool requests (omo_backend.rs:1336).
- Fix: When no token is configured and the gateway is autospawning, generate a random token, write it to the 0600 token file and use it for the backend as well. That removes the unauthenticated mode.

### [P2] Provider credentials in `LlmClient` are not protected
- Location: /Users/indo/code/project/omon-gateway/src/agent/llm.rs:156 (`x-api-key`), :163 (`Authorization`)
- Trigger: The headers are not marked `set_sensitive(true)`, and the client uses reqwest's default redirect policy. reqwest strips `Authorization` on a cross-host redirect but not `x-api-key`.
- Impact: A redirect from a configured Anthropic base URL, such as an http-to-https proxy hop to another host, forwards the key. A `Debug` print of the reqwest client also exposes both headers.
- Fix: Call `.set_sensitive(true)` on both header values. Build the client with `redirect(Policy::none())`.

### [P2] Streamed OpenAI tool-call names are appended on every delta
- Location: /Users/indo/code/project/omon-gateway/src/agent/llm.rs:617
- Trigger: An OpenAI-compatible provider that repeats `function.name` in each delta chunk. This code concatenates the names (`foofoo`). The Ollama branch also overwrites the call at each index whenever tool calls arrive in separate chunks.
- Impact: Tool calls come back with a wrong name. The only current callers use no tools, so this is latent.
- Fix: Assign the name on first sight or when it is non-empty rather than appending it. Accumulate Ollama calls by position across chunks.

### [P2] `LlmClient` has four unchecked edge cases
- Location: /Users/indo/code/project/omon-gateway/src/agent/llm.rs:188, :265, :312, :465
- Trigger and impact:
  - **Endpoint (:188).** A base URL ending in a non-numeric segment, such as Gemini's `/v1beta/openai`, gets `/v1/chat/completions` appended and 404s.
  - **Line buffer (:265).** The line buffer is unbounded if a provider sends no newline.
  - **Clean EOF (:312).** A stream that closes cleanly without `[DONE]` or `message_stop` ends with a normal final chunk and is treated as complete. `/compress` (discord/commands.rs:1072) would then replace history with a truncated summary.
  - **Tool result (:465).** `anthropic_message` emits `role:"tool"` for tool results, which Anthropic rejects. This is dead today.
- Fix: Make URL joining explicit or configurable. Cap the line buffer. Return an error on EOF without a provider terminator. Emit tool results as `role:"user"`.

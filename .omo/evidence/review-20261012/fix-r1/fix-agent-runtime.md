# fix-agent-runtime - fix round r1

Logs (same directory):
- `agent-runtime-red.log`: RED for defects 2-5 (unfixed code, tests added first).
- `agent-runtime-red-d1.log`: RED for defect 1 (test written against the fixed API before the fix existed).
- `agent-runtime-green.log`: GREEN for all defects.

RED command (defects 2-5): `cargo test --lib -- fixr1`
GREEN command (all): `cargo test --lib -- fixr1 test_daemon_command_arguments`

GREEN summary (agent-runtime-green.log):
```
test agent::omo_config::tests::fixr1_zero_timeouts_are_rejected ... ok
test agent::agent_workspace::tests::fixr1_case_folded_ids_do_not_collide ... ok
test agent::llm::fixr1_tests::fixr1_http_client_has_read_timeout_and_no_total_timeout ... ok
test agent::llm::fixr1_tests::fixr1_openai_in_stream_error_frame_yields_err ... ok
test agent::llm::fixr1_tests::fixr1_normal_stream_still_ends_with_final_chunk ... ok
test agent::llm::fixr1_tests::fixr1_anthropic_in_stream_error_frame_yields_err ... ok
test agent::omo_daemon::tests::test_daemon_command_arguments ... ok
test agent::omo_daemon::tests::fixr1_daemon_command_uses_token_file_when_token_configured ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.02s
EXIT:0
```

---

## 1. [P1] daemon spawned with `--ws-auth off` even when a token is configured - FIXED

- File: `src/agent/omo_daemon.rs`
- Flag source: `omo app-server` usage says `[--ws-auth <token-file|off>]`, so token mode means passing a path to a token file.
- Changes:
  - New `listen_port()`. It extracts the port, which the existing log-file naming already did inline and now reuses.
  - New `write_token_file(dir, listen_url, token)`. It writes `~/.omon/omo-appserver-<port>.token`, creates the file with mode 0600 and re-applies 0600 if the file already existed.
  - New `prepare_token_file(cfg)`. It returns `None` when `cfg.auth_token` is unset, so behavior is unchanged in that case. When a token is set it writes the file and returns its path. If `HOME` is missing or the write fails, it returns a clear `OmonError::Config` and refuses to spawn rather than starting an unauthenticated daemon.
  - `daemon_command(bin, url, token_file: Option<&Path>)` passes `--ws-auth <token-file>` when there is a token file, and `--ws-auth off` only when there is none.
  - `OmoDaemonSupervisor::ensure` calls `prepare_token_file`. `spawn_watcher` takes the token file, so restarts keep token mode. The four existing test call sites pass `None`.
- Tests:
  - `agent::omo_daemon::tests::fixr1_daemon_command_uses_token_file_when_token_configured`: checks the file content, mode 0600, and that the argument list is exactly `app-server --listen <url> --ws-auth <file>` with no `off`.
  - `test_daemon_command_arguments` (existing, now called with `None`) still asserts `--ws-auth off` when no token is set.
- RED (`agent-runtime-red-d1.log`, command `cargo test --lib -- fixr1`): the test targets the new API, so on the unfixed code it fails to compile:
  ```
  error[E0425]: cannot find function `write_token_file` in this scope
     --> src/agent/omo_daemon.rs:847:13
  error[E0061]: this function takes 2 arguments but 3 arguments were supplied
     --> src/agent/omo_daemon.rs:852:19
  150 | fn daemon_command(bin: &str, listen_url: &str) -> Command {
  ```
  The unfixed `daemon_command` (2 arguments) hardcoded `"--ws-auth", "off"` at line 162, so it had no token path at all.
- GREEN: `fixr1_daemon_command_uses_token_file_when_token_configured ... ok` and `test_daemon_command_arguments ... ok`.
- Not verified: no live daemon was spawned in token mode, because the rules forbid touching the running gateway or daemon. Only argument construction and the token file were verified.

## 2. [P1] LLM reqwest client has no connect/read timeout - FIXED

- File: `src/agent/llm.rs`, `LlmClient::new`.
- Change: the builder now sets `.connect_timeout(30s)` and `.read_timeout(120s)`. No total `.timeout()` is set, so long streams are not cut off.
- Test: `agent::llm::fixr1_tests::fixr1_http_client_has_read_timeout_and_no_total_timeout`. It inspects the client's `Debug` output and asserts `read_timeout: 120s` is present and no total ` timeout:` is set.
  - Limitation: reqwest's `Debug` output does not include `connect_timeout`, so the 30s connect timeout is verified only by reading the code.
- RED (`agent-runtime-red.log`):
  ```
  thread '...fixr1_http_client_has_read_timeout_and_no_total_timeout' panicked at src/agent/llm.rs:928:9:
  Client { accepts: Accepts, proxies: [Matcher], referer: true, default_headers: {"accept": "*/*", "content-type": "application/json"} }
  ```
- GREEN: `fixr1_http_client_has_read_timeout_and_no_total_timeout ... ok`

## 3. [P1] in-stream provider error frames ignored, normal final chunk emitted - FIXED

- File: `src/agent/llm.rs`
- Changes:
  - New `stream_error(line)`. It detects a non-null top-level `"error"` in an SSE or NDJSON frame. This covers Anthropic `{"type":"error","error":{...}}` and the OpenAI-compatible and Ollama top-level `error`.
  - In the `stream_with_tool_calls` task, both the per-line loop and the trailing-buffer path check `stream_error` first. On an error frame the task sends `Err(OmonError::Llm("provider stream error: <message>"))` to both the chunk stream and the tool-calls oneshot, then returns without the final chunk.
  - `parse_stream_line` itself is unchanged. The check sits in the stream task, which is where the final chunk is emitted.
- Tests:
  - `fixr1_anthropic_in_stream_error_frame_yields_err`
  - `fixr1_openai_in_stream_error_frame_yields_err`
  - `fixr1_normal_stream_still_ends_with_final_chunk` (control case)
  - All three run the real `LlmClient::stream` against a local one-shot TCP server that replays fixed SSE bodies. There are no sleeps, so the tests are deterministic.
- RED (`agent-runtime-red.log`):
  ```
  ...fixr1_anthropic_in_stream_error_frame_yields_err panicked at src/agent/llm.rs:880:9:
  error frame must not be followed by a normal final chunk: [Ok(StreamChunk { ... content: "par", is_final: false ... }), Ok(StreamChunk { ... content: "", is_final: true ... })]
  ...fixr1_openai_in_stream_error_frame_yields_err panicked at src/agent/llm.rs:880:9:
  error frame must not be followed by a normal final chunk: [Ok(StreamChunk { ... content: "", is_final: true ... })]
  test agent::llm::fixr1_tests::fixr1_normal_stream_still_ends_with_final_chunk ... ok
  ```
- GREEN: all three `... ok`.

## 4. [P2] `0` accepted for positive-integer timeouts - FIXED

- File: `src/agent/omo_config.rs`
- Change: `OmoBackendConfig::from_env` (`OMON_OMO_TURN_TIMEOUT_SECS`, `OMON_OMO_TURN_TOTAL_TIMEOUT_SECS`), `cron_from_env` (`OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS`) and `lane_turn_timeout_from_env` (`OMON_LANE_TURN_SECS`) now parse with `.parse::<u64>().ok().filter(|secs| *secs > 0).ok_or_else(..)`. A value of 0 now returns the same existing "expected a positive integer" `OmonError::Config`.
- Test: `agent::omo_config::tests::fixr1_zero_timeouts_are_rejected`. It holds the module's `ENV_LOCK`, sets each variable to `"0"` and asserts an error.
- RED (`agent-runtime-red.log`):
  ```
  thread '...fixr1_zero_timeouts_are_rejected' panicked at src/agent/omo_config.rs:614:9:
  assertion failed: OmoBackendConfig::from_env().is_err()
  ```
- GREEN: `fixr1_zero_timeouts_are_rejected ... ok`

## 5. [P2] case-folded workspace ids collide (no hash suffix) - FIXED

- File: `src/agent/agent_workspace.rs`, `agent_workspace_slug`.
- Change: the old code compared `sanitized` against `raw_value.to_lowercase()`. It now compares against `raw_value`, so an id whose case was folded (`Alice`) gets the `-<sha8>` suffix and no longer collides with `alice`.
- Compatibility: ids whose sanitized form already equals the raw value (lowercase `[a-z0-9_]` with no stray separators) produce the same slug as before. Only mixed-case ids change slug, as the lead noted.
- Test: `agent::agent_workspace::tests::fixr1_case_folded_ids_do_not_collide`. It asserts `alice` maps to `user-alice` (unchanged), while `Alice` maps to `user-alice-<8 hex>`, which differs from `alice`.
- RED (`agent-runtime-red.log`):
  ```
  thread '...fixr1_case_folded_ids_do_not_collide' panicked at src/agent/agent_workspace.rs:169:9:
  assertion `left != right` failed
    left: "user-alice"
   right: "user-alice"
  ```
- GREEN: `fixr1_case_folded_ids_do_not_collide ... ok`

---

Scope: only `src/agent/omo_daemon.rs`, `src/agent/llm.rs`, `src/agent/omo_config.rs` and `src/agent/agent_workspace.rs` were edited. No BLOCKED-SCOPE items. No `cargo fmt` or clippy run, per instructions; the lead runs those. The new `daemon_command` signature line is about 98 characters, so rustfmt may wrap it.

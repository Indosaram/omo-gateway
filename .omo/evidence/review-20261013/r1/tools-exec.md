# tools-exec review

### [P1] web_search never parses DuckDuckGo results, so it always returns an error
- Location: /Users/indo/code/project/omon-gateway/src/tools/web.rs:79 (also :88)
- Trigger: any `web_search` call. I fetched `https://html.duckduckgo.com/html/?q=rust+tokio` with the tool's user agent. Each result is wrapped in `<div class="links_main links_deep result__body">`. The code splits on the exact string `<div class="result__body">`, which never matches, so `results` stays empty.
- Impact: the tool always fails with "search provider response contained no parseable results".
- If the split is fixed, the output is still wrong:
  - The "title" is taken from `result__url`, which holds the display host (`tokio.rs`), not the title in `result__a`.
  - `href` is the first `href="` in the block, which is the `//duckduckgo.com/l/?uddg=...` redirect link rather than the target URL.
  - `&#x27;` is not decoded by `clean_html`.
- Fix: split on `result__body` (for example `class="result__body"` or the substring `result__body">`). Take the title from `result__a`. Take the URL from `result__a`'s href and decode `uddg=`. Add `&#x27;` to the entity list.
- Smaller mismatch: the default `max_results` is 20 (line 48), but the schema text says 5.

### [P1] file write is never approval-gated, so it bypasses the terminal's protected-path approvals
- Location: /Users/indo/code/project/omon-gateway/src/tools/file.rs:60 and :299. Runtime construction is at /Users/indo/code/project/omon-gateway/src/main.rs:874 and /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs:107.
- Trigger:
  - `FileTool::requires_approval` returns a reason only when `require_write_approval` is true.
  - `with_write_approval` has no callers anywhere in `src`, so the flag is always false.
  - `OMON_TOOL_ROOTS` defaults to `$HOME` (README and dashboard_runtime).
  - A model call `{"operation":"write","path":"/Users/<u>/.zshrc","content":"..."}` therefore writes with no prompt. The same applies to `~/.ssh/authorized_keys`, `~/.hermes/.env` and similar.
- Impact: the terminal tool classifies the equivalent `tee`, `>>`, `cp`, `mv` and `sed -i` writes to those paths as dangerous and asks first. The file tool does the same write silently, so a prompt-injected agent gets persistent code execution without approval. Deny globs and hardline rules do not apply to this tool either.
- Fix: enable `.with_write_approval(true)` in both runtime constructions (or make it the default). Alternatively, always require approval for writes outside the primary workspace root or onto the terminal's protected-path set.

### [P1] terminal SIGKILLs the process group when stdout or stderr exceeds max_output_bytes
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:422 and :430. The capture and truncation logic is `read_capped` and `capture` in the same file.
- Trigger:
  - Run any legitimate command that prints more than `max_output_bytes` (default 100,000) on one pipe. Examples: `cargo build`, `npm install`, `pytest -v`, a verbose migration or deploy script, or `git log`.
  - `read_capped` stops at cap+1 bytes and returns, and the loop then calls `kill_group()` and `start_kill()`.
- Impact:
  - A mutating command is killed partway, leaving half-applied state, and the result is `success:false` with `exit_code:null`.
  - Truncation is mislabeled. The "tail" is just the last 60% of the first cap+1 bytes, so the marker reports about "1 bytes omitted" for a run that was killed.
  - The head/tail design in `capture` is effectively dead code for the live path.
- Fix: keep draining after the cap, retaining a head buffer plus a ring-buffer tail and a total byte count, and discard the rest. Kill only on timeout. If a kill on overflow is wanted, report it explicitly as "killed: output limit".

### [P1] browser navigate's redirect and DNS-rebinding check runs before the navigation can have finished, and fails open
- Location: /Users/indo/code/project/omon-gateway/src/tools/browser.rs:310 (check) and :336 (tab registration)
- Trigger:
  - The model calls `navigate` to `https://evil.example/r`, which passes the host check and then 302s to `http://169.254.169.254/...` or `http://192.168.0.1/`.
  - `/json/new` returns as soon as the tab is created. `/json/list` is re-read immediately afterwards.
  - At that moment the tab normally still reports the requested URL or `about:blank`.
  - `ensure_public_final_url` passes both, so the check is vacuous. The test only passes because the fake CDP serves a static list.
  - If the re-list request or JSON parse fails, `.ok()` and `and_then` yield `None`. The check is skipped and the tab is still registered as owned.
- Impact:
  - The operator's shared, logged-in Chrome issues GETs to loopback and private services after the redirect.
  - A later `snapshot` returns the internal page's title and URL.
  - The tab is never closed.
- Fix:
  - Do not rely on a single immediate re-list. Poll `/json/list` until the tab settles, or use a CDP WebSocket with request interception.
  - Treat a failure to verify as an error: close the tab and refuse.
  - Re-validate in `snapshot` and close any owned tab sitting on an internal host.

### [P2] MCP stdio: awaiting the stderr drain after kill can hang forever while holding stdio_lock
- Location: /Users/indo/code/project/omon-gateway/src/tools/mcp.rs:154-155 (the stdio_lock guard is taken in `request`)
- Trigger:
  - The MCP server is launched through a wrapper that forks a child, such as `npx`, `uv run` or `sh -c`.
  - `child.kill()` only kills the direct child. The grandchild keeps the stderr pipe open, so `stderr_task.await` (outside the timeout) never returns.
  - Every later call on that client blocks on `stdio_lock`.
  - On the timeout path the early `?` return leaves the grandchild running, because `kill_on_drop` only covers the direct child.
- Impact: hung tool calls and leaked server processes.
- Fix:
  - Put the child in its own process group and killpg it.
  - Call `stderr_task.abort()` instead of awaiting it.
  - Kill the child on every exit path.
- Severity: kept at P2 because `McpClientTool` is not constructed anywhere in the runtime. `McpTool::default()` is registered with no clients (main.rs:877 and dashboard_runtime.rs:108), and `McpClientTool::new` appears only in tests.

### [P2] MCP client skips the initialize handshake and returns the first JSON message instead of the one matching the request id
- Location: /Users/indo/code/project/omon-gateway/src/tools/mcp.rs:135 (stdin closed right after the request), :299-320 (`read_stdio_response`), `parse_sse_stream`, and `decode_response`
- Trigger:
  - There is no `initialize` or `notifications/initialized` exchange anywhere in `src`, and no `Mcp-Session-Id` handling.
  - A spec-compliant server that rejects `tools/call` before initialization fails.
  - A server that exits on stdin EOF may do so before answering.
  - The readers return the first parseable JSON line or `data:` event. A server notification (log or progress) arriving first makes `decode_response` fail with "response ID did not match".
- Impact: calls fail against standard MCP servers.
- Fix: send initialize and initialized. Keep reading until a message with `id == request id` arrives, and ignore notifications. Do not close stdin before the response.
- Related hang risk: `read_bounded_body` has no total timeout, so a server that trickles bytes can hang the non-SSE HTTP path.
- Severity: P2 for the same reason as above (no runtime client).

### [P2] Caller `env` denylist is incomplete, so an approved argv can run different code
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:377 (override loop, with `is_protected_env_key` in the same file)
- Trigger: `{"program":"cargo","args":["check"],"env":{"RUSTC_WRAPPER":"./x.sh"}}`.
  - Similar variables that are not blocked: `HOME` (git/ssh config), `XDG_CONFIG_HOME`, `NPM_CONFIG_*`, `PAGER`, `EDITOR`, `SSH_ASKPASS`, `LESSOPEN`, `CARGO_*`.
  - Approval classifies only the benign `cargo check`.
  - The file tool can write `x.sh` without approval (see the file-write finding).
- Impact: approval-classification bypass. It is subsumed by the file-write gap while that gap exists.
- Fix: switch to an allowlist of caller env keys, or block a much broader set.

### [P2] Caller `env` can overwrite the injected OMON_SESSION_* and HERMES_SESSION_* variables
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:359-360, then :377
- Trigger: the caller's `env` is applied after the session environment, and these keys are not protected.
- Impact: child scripts that trust those variables for attribution can be spoofed.
- Fix: apply the session environment last, or protect the `OMON_SESSION_` and `HERMES_SESSION_` prefixes.

### [P2] Terminal children inherit the full gateway environment
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:352-380 (no `env_clear` or filtering)
- Trigger: `printenv` or `env`, which are not classified as dangerous.
- Impact: `DISCORD_BOT_TOKEN(S)`, LLM API keys and `OMON_OMO_APPSERVER_AUTH_TOKEN` enter the model context and can be echoed to chat.
- Fix: strip a secret-name denylist from child env, or pass an allowlist.

### [P2] Terminal leaks readers and group members when the execute future is dropped, and can hang until timeout after both pipes close
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:326-450 (`execute_inner`)
- Trigger 1: the caller cancels or drops the future.
  - `kill_on_drop` kills only the direct child.
  - The spawned reader tasks and any process-group members holding the pipes survive.
- Trigger 2: the child closes stdout and stderr but keeps running. The final `child.wait().await` is bounded only by the 600s timeout, and the timeout path discards the captured output.
- Fix: use a drop guard that calls `killpg`. Treat a pipe-closed child with a short grace period before `wait`.

### [P2] Other hardening gaps: unbounded memory and hangs, and SSRF filter coverage
- Locations: /Users/indo/code/project/omon-gateway/src/tools/web.rs, /Users/indo/code/project/omon-gateway/src/tools/browser.rs, /Users/indo/code/project/omon-gateway/src/tools/file.rs
- Items:
  - Internal-IP filter coverage:
    - The `is_internal_ipv4` filters in `web.rs` and `browser.rs` miss 192.0.0.0/24, 198.18.0.0/15, multicast and 240/4.
    - The IPv6 filters miss NAT64 (64:ff9b::/96) and 6to4 (2002::/16), which embed IPv4 addresses.
  - `web.rs` `ensure_public_destination` uses `lookup_host` with no timeout, unlike the browser's 5s limit. `web_fetch` returns raw HTML despite its description promising readable text.
  - `browser.rs` `owned_tabs` (line 23, insert at :336) grows without bound because closed tabs are never removed.
  - `file.rs` `search` (`search_files`, around line 361):
    - An empty query matches every line.
    - Results are unsorted when the limit is hit.
    - There is no truncation flag.
    - The walk is blocking and has no depth or time bound.
  - `file.rs` `write` uses non-atomic `fs::write`, so a crash mid-write leaves a truncated file.
- Fix: address as time allows.

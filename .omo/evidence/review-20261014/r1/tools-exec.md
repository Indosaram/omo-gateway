# Lane tools-exec

Reachability note, applying to every finding below: no production code calls `ToolRegistry::execute` / `execute_with_context` for `terminal`, `file`, `web_fetch`, `web_search`, `browser` or `mcp`. The only production call sites are `src/discord/commands.rs:575/609/634/745`, and they call only `skills` and `cron`. `build_cron_tools` (`src/main.rs:614`) has no caller, and `src/agent/` never references `ToolRegistry`. Production also registers `McpTool::default()` with no clients (`src/main.rs:973`). Under the trigger-based rubric, each finding is therefore capped at P2 until these tools are wired to a real caller. Already-recorded items are not repeated here: search FIFO/walk bounds, write TOCTOU, MCP initialize/id-matching/body timeout, snapshot closing a tab on a DNS error, and the env denylist gaps.

### [P2] Terminal secret stripping misses the `OMON_OMO_AUTH_TOKEN` alias of the app-server auth token
- Location: src/tools/terminal.rs:716
- Trigger: The operator sets the app-server token as `OMON_OMO_AUTH_TOKEN`, which is accepted as an alias at src/agent/omo_config.rs:303. The agent then runs `{"program":"env"}` or `printenv`.
- Impact: The gateway's app-server bearer token reaches the child and appears in output the model can see. That output can then be relayed to Discord. Stripping this token was the point of the R2 fix ("Children inherit gateway secrets -> FIXED").
- Evidence: `const SECRET_SUFFIXES: &[&str] = &["_BOT_TOKEN", "_BOT_TOKENS", "_APPSERVER_AUTH_TOKEN"];`, and `SECRET_NAMES` (line 710) lists only `OMON_OMO_APPSERVER_AUTH_TOKEN`. `OMON_OMO_AUTH_TOKEN` matches neither the names nor the suffixes.
- Fix: Add `"OMON_OMO_AUTH_TOKEN"` to `SECRET_NAMES`, or add the suffix `"_AUTH_TOKEN"`.

### [P2] A grandchild that leaves the process group keeps the pipes open, so a finished command waits the full timeout and returns only "process timed out"
- Location: src/tools/terminal.rs:440
- Trigger: A command backgrounds a process in its own session or process group while it inherits stdout/stderr. Examples: `sh -c "setsid long_daemon &"` on Linux, or `python -c "import subprocess,sys; subprocess.Popen(['sleep','1000'], start_new_session=True)"`. The direct child exits immediately.
- Impact: After `PIPE_DRAIN_GRACE`, `killpg` reaches only the original group, so the escaped process keeps the pipe write ends open. With `group_killed = true` the sleep arm is disabled and the loop blocks until `self.timeout` (600s by default). The tool then returns `Err("process timed out ...")`, and the real exit status and captured output are thrown away.
- Evidence: `_ = tokio::time::sleep(PIPE_DRAIN_GRACE), if status.is_some() && !group_killed => { kill_group(); group_killed = true; }`. After this arm fires, nothing ends the wait on the reader tasks except the outer timeout.
- Fix: After the group kill, allow one more bounded grace period. Then abort `out_task`/`err_task` and return the child's status with whatever was captured. This needs the readers to write into shared buffers, or the abort to be swapped for a cancellation signal that makes `read_capped` return its partial capture.

### [P2] `web_fetch` defaults to 100,000 characters while its schema tells the model the default is 8,000
- Location: src/tools/web.rs:131
- Trigger: The model calls `web_fetch` without `max_chars`, relying on the advertised default ("Maximum characters to return (default 8000)." at src/tools/web.rs:116).
- Impact: Each call returns up to 12.5x more raw HTML than the model was told to expect, which inflates its context and token cost. This is the same class of bug as the web_search `max_results` mismatch fixed in R2; this sibling was missed.
- Evidence: `.unwrap_or(100_000)` against the schema text `"Maximum characters to return (default 8000)."`.
- Fix: Change the default to `8_000`, or change the schema text to match the intended default.

### [P2] MCP stdio writes the request outside the timeout and before stderr is drained, so the call can hang forever while holding `stdio_lock`
- Location: src/tools/mcp.rs:138
- Trigger: A stdio MCP client gets a request larger than the OS pipe buffer (about 64 KiB), for example a large `arguments` object. Either the server is slow to read stdin, or it fills its stderr pipe with startup logs before reading.
- Impact: `stdin.write_all` blocks with no deadline, and the stderr drain task is not spawned until after the write (line 146). A server blocked on a full stderr pipe and a gateway blocked on a full stdin pipe deadlock. The `tokio::time::timeout(self.timeout, read)` at line 154 never starts. Because `stdio_lock` is held, every later call to that MCP client also hangs.
- Evidence: `stdin.write_all(&encoded).await.map_err(mcp_error)?;` runs before `let stderr_task = tokio::spawn(...)`, and only `read_stdio_response` is wrapped in the timeout.
- Fix: Spawn the stderr drain before writing to stdin. Put the write, `shutdown` and read together inside one `tokio::time::timeout(self.timeout, ...)`, and keep the existing kill path on every exit.
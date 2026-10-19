# tools-exec review

### [P1] Terminal hangs until the full timeout and leaks reader tasks when a grandchild keeps stdout/stderr open
- Location: src/tools/terminal.rs:398
- Trigger: The agent calls `terminal` through `ToolRegistry::execute_with_context` (registered in `src/main.rs:790` and `src/dashboard_runtime.rs:90`), for example with `{"program":"sh","args":["-c","npm run dev &"]}` or anything that backgrounds or daemonizes a process. The shell exits right away, but the backgrounded grandchild inherits the stdout/stderr pipes. `read_capped` waits for EOF, so the `while out_done.is_none() || err_done.is_none()` loop never finishes. Nothing waits on the child's own exit status in parallel.
- Impact: A command that finished at once blocks the agent turn for the whole `self.timeout` (600s by default), then returns "process timed out" and discards all output. `kill_on_drop` only signals the direct child, which has already exited. The grandchild survives, and the detached `out_task`/`err_task` keep running and holding buffers until it exits. The same hang happens after an overflow `start_kill()` when a grandchild holds the other pipe.
- Fix: Spawn the child in its own process group (`process_group(0)`) and kill the whole group on overflow or timeout. Also wait on `child.wait()` alongside the readers, then give the readers a short grace period and `abort()` them once the child has exited.

### [P1] Browser SSRF guard only checks the first URL, so redirects and DNS rebinding reach internal hosts
- Location: src/tools/browser.rs:247
- Trigger: The model calls `browser` with `{"action":"navigate","url":"https://<any public open redirector>/?to=http://169.254.169.254/latest/meta-data/"}`. The tool is registered in `src/main.rs:827` and `src/dashboard_runtime.rs:107`. `ensure_public_destination` resolves and approves only the first public host. Chrome then follows the 30x to the internal address, or re-resolves a low-TTL rebinding name to 127.0.0.1.
- Impact: The loopback, link-local and private block that this guard exists to enforce is bypassed. The browser issues GET requests to internal services and cloud metadata. A later `snapshot` returns the internal page's final URL and title from `/json/list` to the model.
- Fix: Enforce the destination check inside the browser rather than before it. Either attach over CDP and use `Fetch.enable` to check each request or redirect hop against `is_internal_ip`, or launch Chrome with a host-resolver/proxy policy that blocks private ranges. If neither is possible, re-check the tab's final URL after load and close the tab when it points to an internal host.

### [P2] Browser tab ownership is shared across all sessions, so snapshot exposes other sessions' pages
- Location: src/tools/browser.rs:213
- Trigger: A single `BrowserTool::default()` is registered for the whole gateway (`src/main.rs:827`), and `owned_tabs` is one shared `Arc<Mutex<HashSet>>`. The tool does not implement `execute_with_context`, so it never sees the `SessionKey`. If session A navigates and session B (another Discord channel or user) later runs `{"action":"snapshot"}`, B gets A's tab.
- Impact: Session B's snapshot returns the URLs and titles of pages session A opened. The comment on lines 20-21 says filtering isolates "every other session", but it does not, so data leaks between sessions.
- Fix: Implement `execute_with_context` and key the owned-tab set by `session.storage_key()`, for example `HashMap<String, HashSet<String>>`. Record tabs and filter snapshots per session.

### [P2] One unreadable or vanished entry aborts the whole file search
- Location: src/tools/file.rs:338
- Trigger: The model calls `file` with `{"operation":"search","query":"x"}` (path defaults to the workspace root). Two cases fail: the tree contains a directory that `read_dir` cannot open (permission denied), or an entry disappears between `read_dir` and the `symlink_metadata` on line 333, which is common while a build is rewriting `target/` or `node_modules/`.
- Impact: The `?` on `read_dir` / `entry` / `symlink_metadata` makes the whole search return an error and drops every match already found. The agent gets a failure instead of results. Unreadable files, by contrast, are already skipped quietly (`let Ok(content) = ... else { continue }`).
- Fix: Treat per-entry errors the same way as unreadable files: `continue` on `symlink_metadata`, `read_dir` and `entry` errors inside the walk. Only return an error when the starting path itself fails.

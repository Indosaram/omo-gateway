# tools-context review

### [P1] cron `update` skips the imported-job read-only guard
- Location: src/tools/cron.rs:394
- Trigger: The agent calls `cron` with `action:"update"` and `id:"hermes:default:x"` (or any row whose authority is `hermes_mirror` or `hermes_synced`) plus a new `script`, `prompt` or `expression`. `delete`, `pause` and `resume` all call `check_imported_read_only` (lines 253, 268, 303). `update` goes from the `SELECT` at line 397 straight to the `UPDATE` at about line 480 without that check.
- Impact: Imported or mirrored Hermes jobs, documented as read-only in the tool description, can be rewritten, including swapping in an arbitrary shell script that the scheduler later runs.
- Fix: Call `check_imported_read_only(&self.pool, id).await?` at the top of the `"update"` arm.

### [P1] cron `add`/`create` silently upserts over an existing or imported job id
- Location: src/tools/cron.rs:593
- Trigger: `cron add` with `id:"hermes:default:x"`, or the id of any other existing job. The scheduler path calls `register_with_id`, whose SQL is `INSERT ... ON CONFLICT(id) DO UPDATE` (`src/cron/scheduler.rs`, `register_with_id`). The non-scheduler path has the same upsert at `src/tools/cron.rs:618`. Neither path calls `check_imported_read_only` or checks that the id is free. Production binds the scheduler at `src/main.rs:1006`.
- Impact: The read-only guard on imported jobs is bypassed. A job owned by another session is overwritten, including its `session_key` and payload.
- Fix: In the add arm, call `check_imported_read_only` first. Also reject the call if a row with that id already exists, so that update is the only way to modify one.

### [P1] Reminders created from a Discord thread session are never delivered
- Location: src/tools/cron.rs:192
- Trigger: A user in a Discord thread asks for a reminder. Ingress sets `session.channel_id` to the parent channel and `thread_id` to the thread (`src/discord/adapter.rs` ~1966-1988). `execute_with_context` then stores `deliver = "discord:<parent>:<thread>"`. The native payload has no `schedule` key, so `delivery_destinations` takes the generic branch (`src/cron/scheduler.rs`, `strip_prefix("discord:")` ~2173). That branch does not split on `:`, so `chat_id` becomes `"<parent>:<thread>"`. `deliver()` then fails at `parse::<u64>()` (`scheduler.rs:1940`) with "invalid Discord channel ID".
- Impact: Every run of the job fails and the failure notification fails the same way. Thread reminders never reach the user, and the only trace is incident rows.
- Fix: In `delivery_destinations`, split `discord:<chat>:<thread>` into `chat_id` and `thread_id`. Alternatively, have the add path store an `origin` object (`platform`, `chat_id`, `thread_id`) in the payload and set `deliver` to `"origin"`.

### [P1] Agent-created cron scripts run shell commands with no approval and no deny-rule check
- Location: src/tools/cron.rs:542
- Trigger: The model, following a user request or injected content, calls `cron add` with `script:"<dangerous command>"` and then `trigger`, or waits for the schedule. `CronTool` does not override `requires_approval` (the only overrides are `McpTool` and a test tool). The only check is `check_gateway_lifecycle`. The executor then runs `sh -c <script>` (`src/cron/executor.rs`, `execute_native_cron` ~414). `TerminalTool` enforces the hardline blocklist, `APPROVALS_DENY` globs (not even under yolo), and the Smart/Always approval flow. Cron skips all of that.
- Impact: The terminal approval and deny controls are bypassed by wrapping the same command in a cron job.
- Fix: Give `CronTool` a `requires_approval` that returns a reason whenever `script` or `ack_command` is set on add/update. Alternatively, run `detect_dangerous_command` and `match_user_deny_rule` on the script at registration time.

### [P1] message_context reads the parent channel, not the thread, inside thread sessions
- Location: src/tools/message_context.rs:389
- Trigger: In a Discord thread session, the model calls `message_context` with `operation:"recent"` or `"search"` and no `channel_id`. The target defaults to `session.channel_id`, which for threads is the parent channel (see `adapter.rs` ~1966-1988). `authorize_target` (line 397) treats the parent as the current conversation. The thread's own id (`session.thread_id`) is only reachable as an explicit `channel_id`. That counts as a cross-channel read and is denied unless the thread id is listed in `DISCORD_ALLOWED_CHANNELS`.
- Impact: The agent cannot read its own thread history by default and instead returns and indexes the parent channel's messages. This is the wrong context. The parent channel may also have a wider audience than a private thread.
- Fix: When no channel is requested and `session.thread_id` is `Some`, default the target to the thread id. Treat `target == thread_id` as the current conversation in `authorize_target`.

### [P1] web_fetch has no private-address or loopback restriction (SSRF)
- Location: src/tools/web.rs:169
- Trigger: The model calls `web_fetch` with `url:"http://127.0.0.1:<dashboard port>/api/sessions/<id>/messages"` (or `/api/bots`, `/api/config`), or `http://169.254.169.254/...`. Only the scheme is checked (lines ~146-153). The redirect policy re-checks only the scheme (`redirect_is_allowed`, line 221). The dashboard has no authentication beyond loopback and host checks (`src/dashboard.rs` `validate`, routes at 734-772), and a request to 127.0.0.1 passes those checks.
- Impact: Any user who can talk to the bot can read other sessions' messages, bot system prompts and cloud metadata. This bypasses the `message_context` cross-channel policy.
- Fix: Resolve the host and reject loopback, private, link-local and metadata addresses, both before the request and on every redirect hop. Do this with a custom resolver, not a string check, to avoid DNS rebinding.

### [P1] web_search parser does not match DuckDuckGo's current result markup
- Location: src/tools/web.rs:77
- Trigger: Every `web_search` call. The code splits on the exact string `<div class="result__body">`. Third-party scrapers match `<div class="result__body links_main links_deep">` (gh code search: yologdev/yoyo-evolve `src/commands_web.rs`, alpesh12345/web-scrapping-automation). The title is read from the `result__url` anchor (the display URL, not `result__a`). The URL comes from the first `href="` in the snippet, which on this page style is a `//duckduckgo.com/l/?uddg=` redirect.
- Caveat: My live curl from this environment returned DuckDuckGo's bot-challenge page, so I could not see real result markup. The class mismatch rests on the third-party evidence above.
- Impact: With the exact-class split the tool returns "no parseable results" for every query. Even if it matched, titles would be display URLs and links would be unusable redirects.
- Fix: Split on `class="result__body` as a prefix match. Take the title from `result__a`. Decode `uddg=` to get the real URL. Detect the anomaly or challenge page and return a distinct error.

### [P2] read_bounded_text can underflow `remaining` and drop its limit
- Location: src/tools/web.rs:293
- Trigger: `web_fetch` on a page with invalid UTF-8 where the character budget is exhausted exactly at the byte before an invalid sequence, for example `max_chars:1` on `"a\xFF..."`. `push_chars` takes all valid bytes and leaves `remaining == 0`. The `error_len() == Some(len)` branch then runs `*remaining -= 1` on 0. `Cargo.toml` has no profile override, so release builds wrap to `usize::MAX`. Debug and test builds panic.
- Impact: In release, the cap is disabled. The body is read until the 15s client timeout, and the returned text and `length` exceed `max_chars`. In debug or test builds this is a panic.
- Fix: Guard the invalid-byte branch with `if *remaining == 0 { break; }`, or use `saturating_sub`. Check the budget before pushing U+FFFD.

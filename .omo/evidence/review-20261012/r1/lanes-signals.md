# lanes-signals review

### [P1] retry_queued does Discord HTTP inside an open transaction on the single-connection pool
- Location: src/lanes/guard.rs:631
- Trigger: A lane was started under a host hold with `--thread`. `cli.rs` `start` leaves it `queued` with `parent_channel_id` set and `thread_channel_id` None. When the 60s guard tick in `runner.rs:1018` calls `retry_queued` and the hold has cleared, the code runs `pool.begin()` (line 601) and the first UPDATE, which takes the write lock. It then awaits `threads.create_thread(...)` (line 631, two Discord REST calls) before the commit at line 652. `storage/db.rs:182` sets `max_connections(1)` with no acquire timeout, so the default 30s applies. `DiscordThreads::new` (the `Http::new` path) sets no request timeout; only the loopback fixture sets 30s (`discord_threads.rs:393`). A slow, rate-limited or hung Discord call therefore holds the only DB connection.
- Impact: Every other DB user in the gateway (lane loops, settle, posts, kibitzer, inbound handling) blocks and then fails with PoolTimedOut after 30s. Those errors propagate through `?` in the runner and kibitzer `serve`. Any lane service task exiting makes `lane_service_exit` fatal in `main.rs`, so a Discord stall can take the gateway down. If the commit never happens after the thread was created, the next tick creates a second thread and starter message.
- Fix: Create the thread before `pool.begin()`, outside any transaction and under `tokio::time::timeout`. Then open the transaction, re-check `status = 'queued'` and the limit, and write the thread id. If the re-check fails after the thread exists, record or archive the orphan.

### [P1] Queued lanes promoted by retry_queued never get a lane_thread_bindings row
- Location: src/lanes/guard.rs:639
- Trigger: Same path as above. `cli.rs:406` calls `bind_lane_thread(pool, key, bot)` only for threads created at start time. `retry_queued` creates the thread at line 631 and stores the id at lines 639 and 644 without binding a bot. `lane_thread_bindings` is only filled by `bind_lane_thread`, which is called from `cli.rs:407`, `runner.rs:176` and `adapter.rs:1743`. The runner and adapter calls only bind if `discord_thread_owners` already has a row for that thread, and only `mark_thread_owner` writes it (on a bot mention in the thread, or on threads the bot auto-creates from a mention or command). The lane thread made here is never marked.
- Impact: `storage/lanes.rs:244` (`record_owner_reply`) joins `lane_thread_bindings`, so owner replies in that thread are silently dropped (`Ok(false)`). `runner.rs:thread_effect` fails with "lane has no evidenced bot binding", so close and reopen of that lane's thread fail.
- Fix: Fetch the bot id with `http.get_current_user()` as `cli.rs:start` does, and call `bind_lane_thread` for the lane in the same transaction or immediately after the commit.

### [P1] One failing queued lane aborts the whole retry pass and re-posts a starter message every tick
- Location: src/lanes/guard.rs:638
- Trigger: `create_thread` posts the starter message first, then `create_thread_from_message` (`discord_threads.rs:213-225`). If the second call fails (missing CREATE_PUBLIC_THREADS permission, deleted parent channel, rate limit), line 638 returns `Err` through `?`, and the same applies to the parse and `from_env` errors at lines 628 and 630. The transaction rolls back and the lane stays `queued`. The runner only logs a warning (`runner.rs:1019`) and retries 60s later.
- Impact: Each tick posts another orphan starter message in the parent channel. Every other queued lane behind the failing one is never promoted, because the loop is aborted rather than moved on to the next lane.
- Fix: Handle the per-lane error inside the loop: roll back, log, and `continue` to the next lane. Mark the lane `failed`, or back off, after repeated failures.

### [P1] PKCS#8 "BEGIN PRIVATE KEY" blocks are not redacted by scrub
- Location: src/lanes/scrub.rs:37
- Trigger: The pattern is `-----BEGIN [A-Z ]+ PRIVATE KEY-----`, which requires at least one word before " PRIVATE KEY". The standard PKCS#8 header `-----BEGIN PRIVATE KEY-----` has no such word, so it never matches. This is the format of service-account keys and `openssl genpkey` output. Nothing else in the chain catches PEM bodies: `redact_approval_display` only matches key/value credentials, and its pattern list has no `private_key` name. A lane agent or command output that prints such a key reaches scrub through `posts::enqueue`, `lane_events` bodies (`activity.rs` `event`, `settle.rs:145`), the Discord thread, and the external judge input.
- Impact: A private key is posted verbatim to Discord threads and stored in events or sent to the judge, defeating the scrub guarantee.
- Fix: Make the middle group optional: `-----BEGIN (?:[A-Z ]+ )?PRIVATE KEY-----.*?-----END (?:[A-Z ]+ )?PRIVATE KEY-----`.

### [P2] Queued lanes are promoted newest-first
- Location: src/lanes/guard.rs:600
- Trigger: `retry_queued` iterates `list_lanes(pool, &["queued"])`, which is `ORDER BY created_at DESC` (`storage/lanes.rs:399` and `:418`). The `break` on the first Hold (line 622) means that when only some slots free up, the newest queued lane launches first.
- Impact: Older queued lanes can starve while newer lanes keep taking freed slots.
- Fix: Sort the queued list by `created_at ASC, key` within `retry_queued` before iterating.

### [P2] Failing command signals put their entire stdout and stderr into the lane detail, objection and agent inbox
- Location: src/lanes/signals.rs:1181
- Trigger: A `command ["cargo","test"]` signal (the documented example) exits non-zero. The failure string interpolates the whole stdout and stderr, and the stdout-regex mismatch at line 1204 does the same. `settle.rs` joins this into `detail` and writes it unbounded to `lanes.objection`, `lane_history.reason`, a `lane_events` body (scrubbed but not truncated) and `lane_inbox.text` as `[LEAD] not done: ...` (`settle.rs:148-152`). The inbox text is steered into the agent's next turn.
- Impact: Large outputs bloat the database and can overflow the agent's prompt or context. That produces turn errors, which feed `runner.rs`'s 3-consecutive-error `fail`, so the lane can end up `failed` instead of `not_done`.
- Fix: Truncate stdout and stderr in `SignalResult.detail` to a few KB, keeping the tail.

### [P2] Command-signal timeout kills only the direct child, leaving grandchildren running
- Location: src/lanes/signals.rs:1127
- Trigger: A command signal such as `["cargo","test"]` or `["sh","-c",...]` exceeds the 120s `command_timeout` (line 1153). The future is dropped and `kill_on_drop(true)` kills only the spawned PID. rustc and test binaries keep running, and a retry after "claim again" starts another set. Other spawn sites in the repo (`cron/executor.rs:427`, `discord/buttons.rs:187`) use `process_group(0)` to avoid this.
- Impact: Orphaned process trees consume CPU on a host the guard is trying to protect. An orphan can hold the cargo build-directory lock, so the next signal check also times out.
- Fix: Call `cmd.process_group(0)` and kill the whole group on timeout (`libc::killpg`).

### [P2] Kibitzer consumes the armed generation before the judge or model turn, so transient failures drop the wake permanently
- Location: src/lanes/kibitzer.rs:238
- Trigger: `evaluate` commits `consumed = generation` at line 238, then runs the judge and agent turn. Errors at line 257 (judge), line 293 (turn failed), line 301 (60s timeout) and line 308 (non-JSON reply) all `return Ok(false)`. A later trigger only fires after a new arm and the 30-minute `SPACING`.
- Impact: A transient judge or backend failure silently loses the proactive wake for the inbox item or lane event that armed it.
- Fix: On the judge, turn or parse failure paths, restore the unconsumed state, for example set `consumed` back to its previous value, keeping `admitted_ms` for spacing. Alternatively, mark `consumed` only after the judge and turn succeed.

### [P2] Home-directory scrub rewrites ordinary URL paths
- Location: src/lanes/scrub.rs:8
- Trigger: `HOME_DIR_SLASH_RE` and `HOME_DIR_BARE_RE` match any `/home/<word>` or `/Users/<word>` with no anchor on a path boundary. `scrub("https://example.com/home/dashboard")` becomes `https://example.com~`, and `.../home/dashboard/x` becomes `https://example.com~/x`. The scrub covers lane posts, events and judge input.
- Impact: Links and web routes in Discord posts and events are corrupted. This does not affect secrets.
- Fix: Require a preceding start of text or whitespace/quote/colon with `(?:^|[\s'"(=:])` and keep that prefix in the replacement, or skip matches inside `scheme://host/...`.

# lanes-surface review

Scope: src/lanes/discord_threads.rs, src/lanes/posts.rs, src/lanes/cli.rs, src/lanes/mod.rs, src/storage/lanes.rs. Nothing was compiled or run; every finding comes from reading the code and its callers. No findings in src/lanes/mod.rs or src/lanes/discord_threads.rs.

### [P1] Forced close after a failed close attempt is silently a no-op
- Location: src/storage/lanes.rs:202
- Trigger: A lane is `verified_done`. `lane close K` (non-force) creates a `lane_close_intents` row, and `LaneRunner::close_intent` fails, for example on worktree removal or a lifecycle timeout. The row is now `state='failed'`, `force=0`. The operator then runs `lane close K --force --note "forced: ..."`, which reaches `request_close` from src/lanes/cli.rs:210. The `ON CONFLICT(lane_key,generation) DO UPDATE ... WHERE state='failed' AND original_status=excluded.original_status AND force=excluded.force` clause does not match because `force` differs (0 vs 1), so the upsert changes nothing. The following `SELECT id` returns the old failed row's id. The serve loop only picks up `state='pending'` intents (runner.rs, the `lane_close_intents WHERE state='pending'` query). Its auto-close check at runner.rs:975-979 skips lanes that already have an intent for this generation.
- Impact: The CLI prints `{"status":"close_pending"}` with exit 0, but nothing is scheduled. The lane stays `verified_done` with a permanently failed intent. The `--force` escape hatch fails silently in exactly the situation it exists for.
- Fix: When the upsert leaves the row not pending, return an error. Alternatively, make the conflict update also overwrite `force`/`note`/`original_status` for a failed row, and check `rows_affected()`.

### [P1] A transient status-refresh error after a successful post aborts the whole lane service
- Location: src/lanes/posts.rs:244
- Trigger: `drain_inner` marks a milestone post `Sent`. It then calls `refresh_lane_status(..., true).await?` for that lane. That function propagates any `sink.rename_status` or `sink.edit_status` error with `?`; `edit_status` errors on everything except Discord 10008. Examples are a network blip, a 5xx, a 403, or the thread being archived or deleted between the send and the edit. The `?` returns out of `drain_with_status`. Its callers use `?` too: runner.rs:1044 in the `posts_ready` branch of the serve loop, and runner.rs:957 at startup. The error therefore exits `service_result`, cancels every lane task, and `serve` returns Err. `refresh_statuses` (posts.rs:273) deliberately catches this same error per lane with the comment "One lane's Discord failure must not stop the lane service (or startup)". At startup the same failure can repeat while pending posts remain, so the service can crash-loop.
- Impact: One lane's Discord hiccup shuts down the lane service for all lanes. Remaining pending posts stay unsent until the restart.
- Fix: Catch the error at this call site the way `refresh_statuses` does: `if let Err(e) = refresh_lane_status(...).await { tracing::warn!(...) }`.

### [P1] One uncertain status send permanently stops all status edits and thread renames for that lane
- Location: src/lanes/posts.rs:309
- Trigger: In `refresh_lane_status`, the send path writes a `status_receipt` row as `Sending`, then calls `sink.send` with a 30s timeout. Any send error, including definite failures such as 403 or a transient 5xx, and any timeout, sets the receipt to `Uncertain`. A crash during sending does the same through `recover()` (posts.rs:157). The next call reads the latest receipt, sees `Sending | Uncertain` at line 309, and returns `Ok(())` before reaching `rename_status`, `edit_status` or `send`. Nothing ever clears an Uncertain receipt. The module comment says it "blocks replacement until its receipt is reconciled", but `status_receipt` is written only in posts.rs and no code reconciles it.
- Impact: After a single failed status send, the lane's status message is never updated again. The thread name also stays frozen, for example at 🔄 while the lane is `verified_done`, until the lane is closed.
- Fix: Treat definite failures as retryable by recording them as failed rather than uncertain. For genuinely uncertain sends, add a reconciliation or expiry path, for example retry with the same nonce while Discord's nonce dedupe window applies, or unblock after a bounded interval.

### [P2] Posts for lanes without a valid thread stay `pending` forever and are re-scanned on every drain
- Location: src/lanes/posts.rs:193
- Trigger: A lane started without `--thread` has `thread_channel_id=None`. `activity.rs` (the milestone call at line 173, the stale call at line 311) still calls `posts::enqueue`, which snapshots `thread: None`. `drain_inner` hits the invalid-destination branch, writes `error` into the JSON, and `continue`s without changing `state`. The row remains `state='pending'` and matches `drain_inner`'s `json_extract(text,'$.state')='pending'` query on every `posts_ready` wake.
- Impact: Unbounded growth of permanently pending rows, each re-parsed and re-UPDATEd on every drain. There is no terminal state to show the post was skipped.
- Fix: Set a terminal state in this branch (for example `Uncertain`, or a new `Failed` variant). Alternatively, skip enqueueing when `lane.thread_channel_id` is None.

### [P2] Discord thread is created before the lane row is durably inserted
- Location: src/lanes/cli.rs:381
- Trigger: `lane start K --thread P`, with no hold decision, creates the Discord starter message and thread. Only afterwards does it call `insert_lane` (line 394). Two concurrent `lane start K` commands both pass the `get_lane` existence check at line 312. The loser's `insert_lane` then fails on the primary key. SQLITE_BUSY after the 5s busy timeout, or a `capture_lane_start` error at line 395, fails the same way.
- Impact: An orphan "🔄 K" thread and starter message remain in the parent channel with no lane behind them. The CLI reports `ok:false` and the thread is never cleaned up.
- Fix: Insert the lane first as `queued` (or reserve the key), create the thread, then update `thread_channel_id`. Alternatively, archive the created thread when the later persistence fails.

### [P2] Concurrent `lane start` calls can exceed the active-lane cap
- Location: src/lanes/cli.rs:141
- Trigger: `execute(Start)` computes `guard::can_launch(pool)` at line 141. `start()` inserts the lane as `working` much later (line 394), with no lock held across the two steps. Two parallel `lane start` invocations, each its own CLI process, both read a count below the limit and both launch. `retry_queued` in guard.rs:595 takes a write lock before counting, so the limit is only authoritative on that path, not here.
- Impact: The configured concurrent-lane cap is exceeded under parallel launches.
- Fix: Make the count check and the insert one `BEGIN IMMEDIATE` transaction, or insert as `queued` and let the guard promote the lane.

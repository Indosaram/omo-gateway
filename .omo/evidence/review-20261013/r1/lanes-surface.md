# lanes-surface review

### [P1] Discord status rename and edit have no timeout and can stall the whole lane service loop
- Location: /Users/indo/code/project/omon-gateway/src/lanes/posts.rs:355 (`sink.rename_status`), :385 (`sink.edit_status`). Callers: /Users/indo/code/project/omon-gateway/src/lanes/runner.rs:958 and :988 (`refresh_statuses`), :957 and :1044 (`drain_with_status`).
- Trigger: A lane's mapped Discord status flips repeatedly, e.g. claim, then object, then claim again. `done_claimed` maps to Working and `not_done` to Waiting, so each flip sets `changed == true` and calls `rename_status`. Discord limits thread renames to about 2 per 10 minutes. I did not verify that limit against Discord. The third rename inside that window gets a 429. The gateway's serenity `Http` has its ratelimiter enabled outside loopback mode. I read `ratelimiting.rs` in serenity 0.12.5 and it does `sleep(retry_after)` inside the request future. A hung HTTP call has no deadline either. By contrast, `sink.send` in the same file is wrapped in `timeout(30s)`.
- Impact: `refresh_statuses` and `drain_with_status` are awaited inline in the single `serve` loop. The loop stops processing close intents, owner replies, wake events, lane spawns, the guard tick and settlements for the length of the sleep, potentially several minutes. A single stalled Discord call blocks every lane.
- Fix: Wrap `rename_status` and `edit_status` in the same `tokio::time::timeout(30s)` used for `send`. Map a timeout to an error that skips the lane for this tick, so the receipt-based retry logic is unaffected.

### [P2] `start` holds the BEGIN IMMEDIATE write transaction across Discord I/O when the insert fails
- Location: /Users/indo/code/project/omon-gateway/src/lanes/cli.rs:398 (tx begin), :420 (`insert_lane`), and the orphan archive block after it.
- Trigger: Two concurrent `lane start` calls with the same key, or any insert error. `tx` is only moved by `commit()` on the success path. On failure it stays alive while the code awaits `threads.archive(...)`. That contradicts the comment about not holding BEGIN IMMEDIATE across network I/O.
- Impact: With a 1-connection pool and a 5s busy_timeout, other writers (the service) hit "database is locked" until the archive call returns or hangs. The archive call has no timeout.
- Fix: Call `tx.rollback().await` (or `drop(tx)`) immediately when `inserted.is_err()`, before any Discord call.

### [P2] `claim-done` evidence is stored unscrubbed in `lanes.claim_evidence` and `lane_history.reason`
- Location: /Users/indo/code/project/omon-gateway/src/lanes/cli.rs:523 (UPDATE binds raw `text`). The `lane_history` INSERT right after it also binds the raw `text`.
- Trigger: A lane agent passes evidence containing a token or secret to `claim-done`.
- Impact: Only the `lane_events` body is scrubbed and truncated. The raw text persists in `lanes` and `lane_history`. It is also printed by `lane list`, which serializes the full `Lane`, and may be read by any consumer of history or lanes.
- Fix: Scrub and truncate once, then use that value for the `claim_evidence` column, the history reason and the event body.

### [P2] `start` reports failure after the lane is already committed and running
- Location: /Users/indo/code/project/omon-gateway/src/lanes/cli.rs:440 (`capture_lane_start?`), :452 (`bind_lane_thread?`), before the `wake` call.
- Trigger: A DB error or bot-binding mismatch in the post-commit steps.
- Impact: The CLI exits 2 with `ok:false` although the lane row is `working` and a Discord thread exists. A retry fails with "lane key already exists". No wake is sent, so the lane is picked up only on the next guard tick (up to 60s).
- Fix: Treat the post-commit steps as best-effort: log them, always call `wake`, and report partial success in the JSON.

### [P2] `send` enqueues and transitions non-atomically
- Location: /Users/indo/code/project/omon-gateway/src/lanes/cli.rs:168 (`enqueue`), followed by `transition(.. Blocked -> Working)`.
- Trigger: The lane leaves `blocked` between the status read and the transition. Another actor moves it to `working`, or the lane object path does.
- Impact: The message is already in `lane_inbox`, but the CLI returns an error. The caller retries and the message is delivered twice.
- Fix: Do the status check, inbox insert and status change in one transaction. Alternatively, ignore an illegal-transition result after the enqueue.

### [P2] `transition_lane` takes a deferred transaction and reads before writing
- Location: /Users/indo/code/project/omon-gateway/src/storage/lanes.rs:445
- Trigger: The CLI process calls it while the service process commits a write between the SELECT and the UPDATE. SQLite returns SQLITE_BUSY_SNAPSHOT immediately and ignores busy_timeout.
- Impact: A spurious "database is locked" error on `lane send` for a blocked lane. The window is tiny, so this is rare.
- Fix: Use `pool.begin_with("BEGIN IMMEDIATE")` as the other writers in this module do.

### [P2] `retry_queued` can create a new Discord thread on every tick when the rollback path is hit
- Location: /Users/indo/code/project/omon-gateway/src/lanes/guard.rs:679 (thread creation), :692 (tx and recheck).
- Trigger: The pre-check passes, the thread is created, then the in-tx recheck holds because a concurrent `start` took the last slot. The code records a `launch_orphan_thread` event, not `launch_failed`, so nothing suppresses the retry. The thread id is not saved and the thread is not archived.
- Impact: Each tick while the cap stays hit leaves another orphan thread in the parent channel.
- Fix: Persist the created `thread_channel_id` on the queued row, or archive the orphan thread. Alternatively, check the cap before creating the thread and treat the orphan event as a retry suppressor.

### [P2] `OMON_HEAVY_SLOTS=0` makes the heavy slot wait forever
- Location: /Users/indo/code/project/omon-gateway/src/lanes/guard.rs:812 and :882 (`acquire_heavy_slot`). `lane heavy` in cli.rs reaches it via `run_with_heavy_slot`.
- Trigger: `OMON_HEAVY_SLOTS=0`. `try_acquire_heavy_slot` iterates over zero slots and returns `None`, and `acquire_heavy_slot` loops forever.
- Impact: `lane heavy` hangs with no diagnostic.
- Fix: Reject a zero slot count (clamp it to at least 1 or return a config error).

### [P2] The CLI's default `DATABASE_URL` is cwd-relative
- Location: /Users/indo/code/project/omon-gateway/src/lanes/cli.rs, in `run` (`unwrap_or_else(|_| "sqlite://omon_gateway.db".into())`).
- Trigger: A lane agent runs the `lane` CLI from its own cwd without `DATABASE_URL` set.
- Impact: It may open or create a different, empty database. Commands then fail with "lane not found", and `wake` targets a different socket path (derived from the DB path).
- Fix: Require `DATABASE_URL`, or resolve the default against a fixed location shared with the service.

# Lane lanes-surface

### [P1] A queued lane whose first launch attempt hits a transient Discord error is never retried
- Location: src/lanes/guard.rs:680
- Trigger: A lane is started with `--thread` while the host guard holds, so it is stored as `queued` with `parent_channel_id` set. On a later tick `retry_queued` calls `create_launch_thread`, and `get_current_user` or `create_thread` returns a 5xx or 429, or times out after 30s (`LAUNCH_DISCORD_TIMEOUT`). `record_launch_problem(..., "launch_failed", ...)` is written at guard.rs:703.
- Impact: The skip is keyed on `launch_failed` events newer than `lane.updated_at`. Nothing in the service updates a queued lane row, so the lane is skipped on every later tick. It stays `queued` for good and the work never starts. `lane send` rejects `queued` lanes, and the claim and object paths reject them as illegal transitions. The only exit is `lane close --force` followed by a new key. The lead is told only through a non-critical `launch_failed` event. `critical()` in dispatch.rs:211 does not list it, so the `noul` judge can drop it as `jev_skip`, and the owner may never find out.
- Evidence: `"SELECT EXISTS(SELECT 1 FROM lane_events WHERE lane_key = ? AND kind = 'launch_failed' AND julianday(created_at) >= julianday(?))"` followed by `if failed_since_change { continue; }`. The skip is permanent for transient errors as well as permanent ones such as a bad token or parent id.
- Fix: Bound the skip in time, for example `AND created_at > now - 10 minutes`, or count attempts and skip only after N failures. Alternatively, mark the lane `failed` after N failures so it becomes reopenable.

### [P2] `lane start` leaks a created Discord thread and starter message when the write transaction fails
- Location: src/lanes/cli.rs:413
- Trigger: `start()` creates the thread (cli.rs:398) and then runs `pool.begin_with("BEGIN IMMEDIATE").await?`. This returns early on SQLITE_BUSY after the 5s busy_timeout, which is plausible when the gateway or other lane CLIs are writing on a loaded host. The same applies to the `count(*)` query at cli.rs:420 and to `tx.commit().await?` at cli.rs:437.
- Impact: The CLI returns an error and no lane row exists. The thread and its starter message stay in the parent channel, unarchived. A retry creates a second thread. The orphan-archive path at cli.rs:446-457 is reached only after `insert_lane` has been attempted.
- Evidence: `let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;` and `tx.commit().await?;`. Both use `?` with `lane.thread_channel_id` already set and nothing archived.
- Fix: Route any error after thread creation (begin, count, commit) through the same orphan-archive block. For example, capture the result of the whole transaction section in a `Result` and archive the thread when it is `Err`.

### [P2] The host guard hard-requires macOS tools, so `lane start`, `can-launch`, `heavy` and the queued-lane retry fail on Linux and Docker
- Location: src/lanes/guard.rs:431
- Trigger: Run on Linux or the shipped Docker image (debian-slim, no procps). `sample()` calls `sample_load_1m()`, which spawns `sysctl -n vm.loadavg` (guard.rs:320). Then `sample_mem_free_pct()` spawns `memory_pressure` (guard.rs:359). Neither exists there, and the README lists Linux and Docker as supported platforms.
- Impact: `sample()` returns `Err`. `LaneCommand::Start` and `CanLaunch` then fail with exit 2 (cli.rs:149-151, 250-252). `Heavy` fails because `wait_for_load_ok` returns the sysctl error. The guard tick logs "host guard sample failed" every 60s and never runs `retry_queued`. Load is not used by `can_launch` at all, yet a missing `sysctl` blocks every launch.
- Evidence: `let load_1m = sample_load_1m().await?; let mem_free_pct = sample_mem_free_pct().await?;`. Any single metric failure fails the whole sample.
- Fix: Add `#[cfg(target_os = "linux")]` probes reading `/proc/loadavg` and `/proc/meminfo`. Alternatively, treat a failed metric as unavailable and skip only that rule, instead of aborting the sample.

### [P2] A persistent edit or rename error on a lane status message repeats on every service-loop iteration, with no receipt, backoff or fallback
- Location: src/lanes/posts.rs:417
- Trigger: A lane thread that is archived (for example by Discord auto-archive after inactivity) or otherwise unwritable while the lane is `working`, `blocked` or `not_done`. `edit_status` returns an error other than 10008, such as 50083. The repo's own comment at posts.rs:278 says Discord rejects edits in archived threads with 50083. I could not verify that Discord behavior here.
- Impact: `Ok(result) => result?` (line 417, and line 378 for rename) returns before any receipt row is written. `prior` therefore never advances, `changed` stays true, and `refresh_statuses` (runner.rs:1056, once per loop iteration) retries the rename and edit for that lane on every iteration. Each failed or rate-limited rename can stall the loop for up to 30s, and the log fills with warnings. The only fallback to a fresh send is the deleted-message case.
- Evidence: `Ok(result) => result?,` after `sink.edit_status(thread, message, &receipt.post.body)`. The `Err` arm records Uncertain only for timeouts.
- Fix: On a non-timeout error, write an `Uncertain` receipt (as the timeout path does) so the 15-minute bound applies. For 50083, unarchive the thread first, or fall back to `send`.
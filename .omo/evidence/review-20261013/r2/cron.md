# cron review

I read the current files for src/cron/*.rs, src/mirror.rs, src/storage/db.rs, src/ledger/service.rs and src/memory/store.rs. I did not run `git diff`, because the tools available this turn were read-only search and read. I reviewed the current files rather than this round's delta. I ran no build or test commands.

Most of this round's fixes hold up. These were checked and found correct:
- The single-statement notepad quota.
- The `ActiveRunGuard` / `active_run_ids` reclaim logic.
- `retire_overdue_oneshot`'s `NOT EXISTS running` guard.
- The `reconcile_job_after_run` compare-and-swap fallback.
- Ack ownership between the executor and the scheduler: the ack runs exactly once, and only after delivery succeeded.
- Single-connection safety: no code holds a transaction while querying `&self.pool`.
- The `acknowledge_digest` redaction SQL offsets.
- The mirror briefing peek/mark split.

The defects below are in or around those fixes.

### [P1] One-time owner approval token reaches the agent through the digest mirror
- Location: /Users/indo/code/project/omon-gateway/src/cron/remediate.rs (`needs_owner`, the outcome text containing `--owner-token {token}`); /Users/indo/code/project/omon-gateway/src/cron/report.rs:207 (`digest_section`) and :231 (`acknowledge_digest`); /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs:2354-2361 (`deliver` mirroring); /Users/indo/code/project/omon-gateway/src/mirror.rs:211 (`peek_pending_cron_briefings`).
- Trigger:
  1. A publishing problem reaches `needs_owner`.
  2. `queue_outcome` stores the full message, token included. `scrub` does not redact `--owner-token <hex>`.
  3. The inbox digest (`omon:inbox-digest`) puts every undelivered outcome into its text.
  4. The digest has no `attach_to_session`, so it defaults to true. `deliver` mirrors the text into the owner's home-DM session as an assistant message with `mirror_source = cron:omon:inbox-digest`.
  5. On the owner's next turn, `peek_pending_cron_briefings` injects that text into the agent prompt.
- Impact:
  - The agent can read the token and run `omo-gateway cron approve-rerun ... --owner-token <token>` itself. That defeats the owner-approval gate for publishing reruns, which is the point of this round's token fix.
  - `acknowledge_digest` only redacts `cron_remediation_outcomes`. The token stays in `messages` and in `cron_outputs.output`.
- Fix:
  - Keep the token out of the digest text. Deliver it only through a channel that is not mirrored, or emit a placeholder in the digest and send the token separately.
  - At minimum, set `attach_to_session: false` on the digest job spec (`inbox/digest.rs::spec`), and strip the token from `messages`/`cron_outputs` when acknowledging.

### [P1] A monitor-mode job loses its detected change when delivery fails
- Location: /Users/indo/code/project/omon-gateway/src/cron/executor.rs (`AgentCronExecutor::execute`: `pending_monitor_state` is committed right after `backend.run`); /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs (`execute_job_with_outcome`: delivery happens after `execute` returns).
- Trigger:
  1. A job with `monitor_script` or `monitor_url` sees new output and the agent run succeeds.
  2. The executor upserts `cron_monitor_states` with the new hash.
  3. The scheduler's Discord `deliver` fails with a transient outage. The run is recorded failed and a retry is scheduled.
  4. On the retry, the monitor output hash equals the stored one, so the executor returns `Ok(None)` ("monitor state unchanged").
- Impact: The change is never delivered. The comment "a failed run does not consume the detected change" is false for delivery failures, which happen after the executor has already committed.
- Fix: Commit the monitor state from the scheduler after delivery succeeds, for example in `complete_success`. Alternatively, key the state to the run and write it only when the run succeeds.

### [P1] Backoff retries of cron-expression jobs are skipped and raise spurious "missed" problems
- Location: /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs:2771-2786 (`next_run_after_failure` returns `max(scheduled_next, now + backoff)`), together with the catch-up branch in `claim_job` at :1351-1440 (`newest_missed_occurrence`, the `None` arm).
- Trigger:
  1. A cron-expression job such as `* * * * *` fails 5 or more times in a row, so the backoff (160s or more) exceeds the interval to the next cron mark.
  2. `next_run_at` is set to an off-grid instant T.
  3. When T is due, `newest_missed_occurrence` searches for cron marks in `[T-1s, now]`. There are none, so it returns `None`.
  4. `claim_job` takes the "out-of-window" arm: it calls `record_missed_occurrence(next_run, next_future)` and moves `next_run_at` to the next mark without running.
- Impact:
  - The backoff retry never runs.
  - Each cycle inserts a new `missed` problem (the signature is the occurrence time) plus a `cron_problem` lane event. That triggers a judge call and possibly a remediation lane for a non-fault.
  - Interval jobs are unaffected, because their catch-up math is anchored at `scheduled_from`.
- Fix: When `next_run` is not a schedule occurrence, such as after a backoff, treat it as an explicit retry time. For example, run with `target_occurrence = None` when `next_run` was set by the failure path, or persist a `retry_at` marker in the payload and skip the missed-window logic for it.

### [P2] Sync overwrites the payload and drops `last_occurrence`
- Location: /Users/indo/code/project/omon-gateway/src/cron/store.rs (the `HermesStoreSynchronizer::sync_at` upsert, `payload_json=CASE ...`).
- Trigger: Every sync tick for a mirrored job whose DB payload has `last_status`. The `json_set(...)` branch preserves `last_status`, `last_run_at`, `last_error` and `last_delivery_error`, but not `last_occurrence`.
- Impact: The occurrence-keyed dedup in `claim_job` (`last_occ == occ_key`) is effectively dead for Hermes mirrors. It only matters if `next_run_at` is reset, for example by an edit to the source job.
- Fix: Add `'$.last_occurrence'` to the preserved keys.

### [P2] `NativeServer::bind` changes the process-wide umask
- Location: /Users/indo/code/project/omon-gateway/src/cron/report.rs (`NativeServer::bind`, the `libc::umask(0o177)` window).
- Trigger: Another thread creates a file or directory during the bind window under the multi-threaded runtime.
- Impact: Those files get 0600 and those directories get 0700 permissions.
- Fix: Bind inside a private 0700 directory, or `bind` and then `chmod` while the parent directory is 0700. Do not toggle the process umask.

### [P2] Obligation recovery can claim rows owned by another live process
- Location: /Users/indo/code/project/omon-gateway/src/ledger/service.rs (`sweep_recoverable`).
- Trigger: A row whose `owner_pid` is alive but whose `owner_started_at` differs from this process's start stamp, such as an overlapping old instance during a restart.
- Impact: The new process takes the old process's in-flight `pending`/`attempting` obligations, which can cause duplicate sends. It is safe only when the old instance has exited before the new one boots.
- Fix: Skip a live `owner_pid` (other than the current one) unless the row is older than a staleness bound.

### [P2] `needs_owner` mints and queues a token for an already-fixed problem
- Location: /Users/indo/code/project/omon-gateway/src/cron/remediate.rs (`needs_owner`).
- Trigger: The `UPDATE ... status!='fixed'` matches zero rows (the problem was fixed concurrently).
- Impact: The inbox row and the outcome message still carry a token that can never match anything.
- Fix: Return early when the update affects zero rows.

### [P2] One bad payload fails the whole health report
- Location: /Users/indo/code/project/omon-gateway/src/cron/report.rs (`health`, `serde_json::from_str(&payload)?`).
- Trigger: A single `cron_jobs` row with invalid JSON.
- Impact: `omo-gateway cron health` fails for all jobs.
- Fix: Skip the row, or report it as an error entry.

### [P2] `catch_up_hours` can make a poll arbitrarily slow
- Location: /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs:1383 (the `catch_up_hours` window) and `newest_missed_occurrence`.
- Trigger: A payload with a huge `catch_up_hours` and a high-frequency expression, after a long `next_run_at` gap.
- Impact: The synchronous per-occurrence iteration can stall the poll loop.
- Fix: Cap the iteration count, or compute the newest occurrence from `now` going backward.

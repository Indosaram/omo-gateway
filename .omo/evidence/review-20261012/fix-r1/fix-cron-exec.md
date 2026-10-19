# fix-cron-exec

RED/GREEN: deferred to single final integration gate.

An earlier attempt of this node captured RED on the unfixed code: `fix-cron-exec-red.log`, command `cargo test --test test_fix_fix_cron_exec`, result `FAILED. 0 passed; 8 failed`. After the fixes, the GREEN run was started but killed on the user's stop-builds directive. Nothing was compiled or run after the fixes, so none of them is verified by a test run. `fix-cron-exec-green.log` is empty or partial; ignore it.

Files changed: `src/cron/store.rs`, `src/cron/executor.rs`, `src/cron/remediate.rs`, `src/cron/report.rs`. Tests are in `tests/test_fix_fix_cron_exec.rs` (8 tests, written earlier, not re-run). rustfmt was applied to the four source files only.

## Defect -> fix mapping

1. [P1] Non-ASCII `deliver` panic: FIXED.
   - Location: `store.rs`, `HermesJob::has_explicit_discord_destination` and `HermesJob::discord_destinations`.
   - Fix: `trimmed[..8]` replaced by `trimmed.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("discord:"))`.
   - Test: `non_ascii_deliver_target_does_not_panic`.
   - Earlier RED: `byte index 8 is not a char boundary ... of 디스코드:123`.

2. [P1] Re-opened problem never re-notifies the owner: FIXED.
   - Location: `remediate.rs`, `Remediator::needs_owner`.
   - Fix: the `inbox_decisions` upsert now also sets `status='open'` and `decided_at=excluded.decided_at` on conflict.
   - Test: `needs_owner_reopens_closed_inbox_decision`, covering both `done` and `dismissed`.

3. [P1] Ack ran before delivery: CONFIRMED and FIXED.
   - The ack was passed to the backend as `cron_ack_command` session metadata. `omo_backend.rs` ran it right after producing output, before the scheduler delivered anything. The scheduler also runs it after successful delivery, so the ack ran twice and the first run was not delivery-gated.
   - Fix, location: `executor.rs`, `AgentCronExecutor::execute` and `execute_native_cron`. They no longer insert `cron_ack_command` into session metadata, so the backend never acks.
   - Fix, ack path: the scheduler's existing post-delivery ack (`scheduler.rs`, `execute_job_with_outcome`) is the single owner when it will deliver. When the scheduler will not deliver (no destination, or empty/silence content), the new helper `ack_unless_scheduler_delivers` runs the ack once, after the agent run succeeded.
   - Test: `ack_runs_only_after_successful_scheduler_delivery`. It checks that a failed delivery runs no ack, a successful one acks exactly once, and the backend never acks. It also covers the native path.
   - Earlier RED: `failed delivery must not ack left: 1 right: 0`.
   - Out of scope, untouched: `omo_backend.rs` still contains its own `cron_ack_command` handling. It is now dead for cron jobs launched through these executors, and removal is a separate cleanup.

4. [P1] Monitor snapshot upserted before the run succeeded: CONFIRMED and FIXED.
   - Location: `executor.rs`, `AgentCronExecutor::execute`.
   - Fix: the `cron_monitor_states` upsert is deferred (`pending_monitor_state`) and committed only after `backend.run` returns Ok.
   - Test: `monitor_snapshot_is_committed_only_after_successful_run`.
   - Earlier RED: `failed run must not record the snapshot left: 1 right: 0`.
   - Limitation: a delivery failure after a successful run still consumes the snapshot. The executor cannot see delivery; fixing that needs a scheduler change.

## P2 items

- FIXED, `executor.rs` `context_from`: predecessor outputs are now looked up with `job.profile()`, the same key the scheduler writes under. Previously it used `origin.platform`. Test: `context_from_reads_scheduler_written_output_key`.
- FIXED, `store.rs` `HermesStoreSynchronizer::sync_at`, two cases:
  - A missing `jobs.json` skips the store, so its mirrors are not deleted.
  - An entry that fails validation keeps its existing mirror (its id is added to `live`).
  - Test: `sync_keeps_mirror_jobs_when_entry_invalid_or_store_missing`.
- FIXED, `report.rs` `NativeServer::serve`: an `accept()` error is logged, followed by a 100ms pause and `continue`, instead of ending the server. No test, because a deterministic accept failure cannot be produced; this is verified by code reading only.
- FIXED, `remediate.rs` `log_excerpt`: it now calls the new `read_log_tail` (bounded to `LOG_TAIL_BYTES`, 4 MiB), which seeks to the tail and drops the partial first line. Test: `log_tail_is_bounded`.
- FIXED, `remediate.rs` `make_lane`: the hard-coded developer path fallback is replaced by `OMON_WORKSPACE_ROOT`, else `std::env::temp_dir()`. Test: `lane_cwd_fallback_is_not_a_developer_path`.

## Deferred / BLOCKED-SCOPE

- Defect 4, delivery-failure case: needs a scheduler change in `scheduler.rs`, which is outside this node's write scope.
- The dead ack handling in `omo_backend.rs` (see defect 3) is out of scope.

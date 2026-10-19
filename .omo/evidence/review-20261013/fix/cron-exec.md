# cron-exec fix report (round 5)

No compile, test, fmt or clippy command was run (user directive). Everything below is unverified until the lead's integration gate.

Files edited:
- `src/cron/health.rs`
- `src/cron/remediate.rs`
- `src/cron/report.rs`
- `src/cron/ack.rs`
- `src/cron/store.rs`
- `src/cron/executor.rs`
- `src/ledger/service.rs`
- `src/memory/store.rs`

Test files:
- New `tests/test_fix_r5_cron_exec.rs`.
- Unit tests were added in `health.rs`, `ack.rs`, `store.rs`, `executor.rs`, `ledger/service.rs` and `memory/store.rs`.

`guard.rs` needed no change.

## cron-store-exec lane

### [P1] Owner-approval gate bypassable by any same-user process - FIXED, with a residual limitation
- `remediate.rs`:
  - `needs_owner` mints a one-time 256-bit token.
  - It stores only `sha256(token)` in `Detail.approval_hash`.
  - The token is delivered only inside the owner-facing outcome message: `... cron approve-rerun <id> --owner-approved --owner-token <token>`.
  - `approve_rerun_with_token` checks the token with a constant-time compare.
  - `approve_rerun(id, token)` is the module-level entry point.
  - The hash is cleared on approval, so each token is single-use.
  - The old `Remediator::approve_rerun(id)` stays for trusted in-process callers (settlement and existing tests).
- `report.rs`:
  - The `approve_problem` socket request now requires `owner_token`.
  - The CLI has a new required `--owner-token`. It is sent over the socket and no longer discarded.
  - `health()` strips `approval_hash` from problem detail, so `cron health --json` no longer exposes it.
  - `acknowledge_digest` redacts the plaintext token from delivered outcome rows.
- Tests:
  - `tests/test_fix_r5_cron_exec.rs::approval_requires_the_one_time_owner_token`
  - `::health_output_never_exposes_the_approval_digest`
  - `::native_socket_rejects_tokenless_approval_and_bounds_request_size`
- **Residual limitation:**
  - The plaintext token still sits in `cron_remediation_outcomes` and the inbox until the digest is delivered. A same-user process that reads the SQLite DB directly could still obtain it.
  - Full isolation needs an out-of-band credential, such as a Discord DM that never touches an agent-readable table, or a separate OS user. That is a design change outside this node.
  - This fix closes the reported path: id enumeration via `health --json`, plus a bare `approve-rerun` over the socket.

### [P1] Problem stranded in `remediating` never escalated - FIXED
- `remediate.rs`:
  - New `STALE_REMEDIATING_SECS` (3600).
  - New `Remediator::sweep_stale_remediating` and `remediate::sweep_stale_remediating()`.
  - The sweep moves `remediating` rows with no live lane and no recent update to `needs_owner`.
  - `handle_event` runs the sweep on every `cron_problem` event.
- `health.rs`: the upsert also reopens a stale `remediating` row on recurrence (same staleness and no-live-lane condition), which re-emits the event. Recurrences are no longer suppressed.
- Tests:
  - `stale_remediating_rows_without_a_live_lane_are_escalated`
  - `varying_error_noise_maps_to_one_problem_and_stale_remediating_reopens`
- **BLOCKED-SCOPE (wiring):** there is no startup or periodic caller. `handle_event` only covers rows when some other problem event arrives. The lead needs to add `omon_gateway::cron::remediate::sweep_stale_remediating().await` to startup in `src/main.rs` (after `remediate::install`), and optionally to the scheduler tick in `src/cron/scheduler.rs`.

### [P1] Signature includes full error text - FIXED
- `health.rs::problem_signature`: `kind` plus the first and last non-empty line of the detail, each normalized and bounded to 160 chars.
  - UUIDs become `<uuid>`.
  - Hex tokens of 8 or more chars become `<hex>`.
  - Digits become `#`.
  - Text is lowercased and whitespace-collapsed.
- `on_run_finished` uses it for both the insert and the id lookup.
- Tests:
  - `health.rs::tests::signature_ignores_timestamps_pids_and_ids_but_keeps_the_fault`
  - `::signature_is_bounded_and_uses_first_and_last_line`
  - `varying_error_noise_maps_to_one_problem_and_stale_remediating_reopens`
- Existing rows keep their old signatures. They dedup under the new scheme only after the old rows fix or ignore.

### [P2] Raw error text reaches digest, inbox and lane brief unscrubbed - FIXED
- `report.rs::digest_section` scrubs the cause.
- `remediate.rs::needs_owner` scrubs the action. That covers the inbox snippet, `inbox_decisions` and the outcome message.
- `make_lane` scrubs the cause, runs, output and log fields. Script paths and the done command are deliberately left unscrubbed.
- Test: `error_text_is_scrubbed_in_inbox_brief_and_digest`.

### [P2] Duplicate and contradictory escalation after failed transient rerun - FIXED
- `remediate.rs`: the second `needs_owner` call in the `transient_retry_once` branch is removed, because `verify_once_inner` already escalates.
- Test: `failed_transient_rerun_escalates_exactly_once`.

### [P2] Approving after budget exhausted is a no-op - FIXED
- `remediate.rs::approve_rerun`: `detail.reruns` is capped at `MAX_RERUNS - 1`. Each owner approval grants exactly one more attempt.
- Test: `approval_after_exhausted_budget_grants_one_more_attempt`.

### [P2] Retention days panic, prune only at boot - PARTIAL
- FIXED: `store.rs::prune_terminal_cron_runs` uses `TimeDelta::try_days` and `checked_sub_signed`, and returns `OmonError::Config` on overflow.
- Test: `store.rs::tests::prune_rejects_retention_that_overflows_instead_of_panicking`.
- **BLOCKED-SCOPE:**
  - Periodic pruning needs a caller outside this node: `src/main.rs:~971` or the scheduler loop in `src/cron/scheduler.rs`.
  - A call to `prune_terminal_cron_runs(pool, days, Utc::now())` on an interval.

### [P2] Ack timeout kills only `sh` - FIXED
- `ack.rs::run_ack_command`: the child is spawned with `process_group(0)` on unix and `libc::kill(-pid, SIGKILL)` on timeout, as `executor.rs` does for scripts.
- Test: `ack.rs::tests::timeout_kills_grandchildren_of_the_ack_shell`. It records the grandchild pid and waits on `kill(pid, 0) == ESRCH` with a bounded deadline, with no sleeps.

### [P2] Native socket hardening and shutdown - FIXED
- `report.rs::NativeServer::bind`: umask `0o177` around `bind`, so the socket is created 0600 with no wider window.
- The request line is capped at 4096 bytes via `.take`.
- Shutdown gives in-flight requests a 5 s grace period, then `abort_all`, so socket cleanup no longer hangs.
- Test: `native_socket_rejects_tokenless_approval_and_bounds_request_size`. It asserts mode 0600, the oversized-request path, and bounded shutdown.
- Caveat: the umask swap is process-global for the duration of one synchronous `bind` call.

### [P2] Notepad quota non-atomic, keys mismatch - PARTIAL
- FIXED (quota race): `store.rs::set_cron_notepad` is now a single `INSERT ... SELECT ... WHERE (SUM of other keys) + len <= cap ON CONFLICT DO UPDATE`. Zero affected rows returns a `Config` error.
- Test: `store.rs::tests::notepad_total_quota_holds_under_concurrent_writers`.
- FIXED (profile mismatch): `executor.rs::collect_notepads` merges the tool-default `"default"` profile with the job's derived profile. The job profile wins on key conflict.
- Test: `executor.rs::tests::notepads_written_under_default_profile_reach_jobs_with_derived_profile`.
- SKIPPED: injecting notepads into `execute_native_cron`. It has no pool parameter. Adding one changes a public signature used by `tests/test_cron_executor_parity.rs` (outside scope). Needed change: add a `pool: &SqlitePool` argument, call `collect_notepads(pool, &job.profile(), &job.id)`, and append a `[Notepad]` block to `task`.

## storage-ledger lane (my files only)

### [P2] Debounced batch cannot reclaim failed deliveries, one seen constituent drops batch - FIXED
- `ledger/service.rs::record_incoming_with_constituents`: the precheck is now `is_claimed` (any row whose status is not `failed`). Failed batches are reclaimable.
- Already-claimed constituents are skipped individually. The batch is no longer dropped.
- Test: `ledger::service::tests::failed_batch_is_reclaimed_and_seen_constituent_does_not_drop_the_rest`.

### [P2] Constituent writes swallowed, not transactional - FIXED
- `record_incoming_with_constituents`: the parent claim and all constituent writes run in one transaction with errors propagated (`claim_delivery_row` helper takes any executor).
- `complete()`: the parent and constituent UPDATEs run in one transaction. `ensure_constituents_table` and the second UPDATE now propagate errors instead of `let _ =`.
- Covered by the same test as above. The pre-existing `tests/test_discord_adapter.rs` debounce scenarios exercise the same paths.

### [P2] Recovery loop aborts on single `?`, can re-send a delivered reply - FIXED
- `recover_pending_delivery_obligations`: per-obligation errors are logged and the loop continues. A failed `attempting` write skips that obligation's dispatch.
- The first error is returned at the end, so `tests/test_review_agent.rs`, which asserts that persistence faults reach the caller, is preserved.
- A failed delivered-checkpoint is logged and reported instead of aborting the remaining recoveries.
- Test: `ledger::service::tests::recovery_continues_past_one_failing_obligation_and_reports_the_error`.
- Residual: if the delivered checkpoint itself fails after a successful dispatch, that one row can still be re-sent next boot. The checkpoint is not retried. Making that exactly-once needs a two-phase send, which is out of scope.

### [P2] `sweep_recoverable` skips legacy NULL-`owner_started_at` rows with recycled PID - FIXED
- `sweep_recoverable`: only a row stamped with this process start is treated as live-owned. An unstamped legacy row falls through to the stale/abandon/claim logic.
- Test: `ledger::service::tests::legacy_unstamped_obligation_with_current_pid_is_not_skipped_forever`.

### [P2] Memory store edge cases - PARTIAL
- FIXED (`memory/store.rs`):
  - One corrupt `metadata_json` row no longer fails the whole search. The row is logged and skipped.
  - Search is capped to the 10k most recent rows per session (`LIMIT`).
  - Test: `memory::store::tests::search_skips_a_memory_with_corrupt_metadata`.
- SKIPPED: the approval-mode return of a pending id as a `Memory`. Changing the return type of `remember` breaks `tools/skills.rs` and `tests/test_agent_tools.rs` (outside scope).
- BLOCKED-SCOPE: the 8-hex-char pending id in `src/storage/db.rs::stage_pending_write` needs a longer id there.

## Not in my scope
Items from the storage-ledger lane that live in `db.rs`, `mirror.rs`, `message_search.rs` and `messenger_policy.rs` belong to other nodes (cron-scheduler / tools). I did not touch them.

## Gate notes for the lead
- `Detail` gained `approval_hash` (serde default), so old rows still parse.
- `report::CronCommand::ApproveRerun` gained a required `owner_token`. No other callers or docs reference the CLI.
- `remediate::approve_rerun` now takes `(id, owner_token)`. The only caller is `report.rs`.
- The new test file installs the global `Remediator` via `OnceLock` in exactly one test. Do not add a second `install` in that binary.

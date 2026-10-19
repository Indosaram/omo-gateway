# fix-sessions-judge

Scope: src/sessions/external.rs, src/judge/mod.rs, tests/test_fix_fix_sessions_judge.rs.

**RED/GREEN: deferred to single final integration gate.** Per the user's stop-builds directive, no cargo command was run after the directive and I killed the sessions I had started. The earlier RED log is in fix-sessions-judge-red.log (7 of 8 tests failed on the unfixed code). The GREEN run never completed because it was blocked on the shared cargo build-dir lock, and fix-sessions-judge-green.log holds no result.

## Defect -> fix mapping

1. [P1] remote_checks_with: control.output(...) errors no longer propagate with `?`. All three call sites (command signal, file check, worktree check) push a reason ("remote command unavailable: ...", "remote file check unavailable: ...", "remote worktree check unavailable: ...") and continue. Test: remote_check_transport_error_becomes_reason.
2. [P1] drive: introduced MAX_CONSECUTIVE_ERRORS = 3. Each failing step (send_goal, then read_new + send_pending) is logged with tracing::warn! and counted. The lane fails only on ControlError::NotRunning or after 3 consecutive errors, and a success resets the counter. Tests: drive_tolerates_two_transient_errors, drive_fails_immediately_when_session_not_running.
3. [P1] send_goal / send_pending no longer hold BEGIN IMMEDIATE across control.send. send_goal reads with a plain pool query, sends (the idempotency send_key is unchanged), then runs a guarded UPDATE (sent_at IS NULL). send_pending reads and updates in a short tx and commits before sending. After each send it runs the guarded UPDATE (consumed_at IS NULL). Test: sends_run_outside_transactions. Its fake send() takes BEGIN IMMEDIATE on the single-connection pool under a 5s bounded timeout, so it fails if a tx is still held.
4. [P1] release: in the same tx, a non-terminal lane (not verified_done, closed or failed) is set to 'closed' with a status-guarded UPDATE. A lane_history row is inserted with reason 'external session released' (from_status = previous status). Test: release_closes_lane_and_writes_history.
5. [P1] judge diff scrub: scrub_for_external now delegates to scrub_with_cap(input, 4000) with behaviour unchanged. A new scrub_diff_for_external uses MAX_DIFF_SCRUB_CHARS = 200_000. It is applied to the diff in JevCli::diff (scrubbed_diff) and to the "state" field in fallback_diff. Test: done_diff_over_4000_chars_reaches_judge_untruncated (a 9000+ char diff must reach the judge stdin with END_MARKER).
6. [P2]
   - KST boundary (judge/mod.rs, sync_persisted_counts): now bound as DateTime<Utc> (dt.and_utc() - 9h). Test: persisted_counts_exclude_calls_before_kst_midnight.
   - Reopen generation (external.rs, read_new): captured_item now gets the current reopen generation (MAX(id) of lane_history with reason 'reopened'), not 0. Test: read_new_records_activity_after_reopen.
   - --lead validation with SessionKey::from_storage_key: NOT APPLIED. The existing test tests/test_sessions_control.rs:248 calls adopt(..., "owner"), which is not a valid storage key, so the change would break that test. It is also better done at the CLI layer (src/sessions/cli.rs, outside my write scope). Needed change if wanted: in src/sessions/cli.rs Adopt, call SessionKey::from_storage_key(&lead) before adopt.

## Files changed
- src/sessions/external.rs: release, send_goal, read_new, drive, send_pending, remote_checks_with, ControlError import, MAX_CONSECUTIVE_ERRORS.
- src/judge/mod.rs: scrub_for_external, scrub_with_cap, scrub_diff_for_external, MAX_DIFF_SCRUB_CHARS, fallback_diff, JevCli::diff, sync_persisted_counts.
- tests/test_fix_fix_sessions_judge.rs (new): 8 tests, names listed above.

## Risks to check at the final gate
- tests/test_sessions_control.rs and the existing external.rs/judge tests were not re-run.
- In drive, send_goal is now also retried each loop iteration, and its guarded no-op when already sent is relied on.

# r2 cron fixes

No cargo, bun or npm command was run, as instructed. Edits are checked by reading only; the lead's gate must compile and run the tests.

## P1-1 Owner token leaks to the agent: FIXED
- `src/inbox/digest.rs::spec`: the digest job payload now sets `attach_to_session: false`, so `deliver` does not mirror it into the owner's DM session. The Discord message to the owner still carries the token.
- `src/mirror.rs::peek_pending_cron_briefings`: skips mirror rows whose `mirror_source` is `cron:omon:inbox-digest`. This also covers rows mirrored before this fix.
- `src/cron/scheduler.rs::redact_owner_tokens` (new, regex `--owner-token <alnum>` becomes `--owner-token <redacted>`). It is applied in two places:
  - `deliver`, to the mirrored text only; the dispatched Discord content is unchanged.
  - `complete_success`, to the text stored in `cron_outputs`.
- Tests:
  - `digest::tests::digest_job_is_never_attached_to_the_session_transcript`
  - `mirror::tests::inbox_digest_mirror_rows_are_never_briefed_to_the_agent`
  - `scheduler::tests::redact_owner_tokens_strips_only_the_token`
  - `scheduler::tests::owner_token_is_redacted_from_cron_outputs_and_mirrored_transcript` (also asserts the notification/Discord content keeps the token)

## P1-2 Monitor change lost on delivery failure: FIXED
- `src/cron/scheduler.rs`: new `CronRunOutcome { output, monitor_state: Option<(job_id, hash, snapshot)> }`. `CronTaskExecutor` gains `execute_with_outcome`, whose default wraps `execute`. `execute_job_with_outcome` returns the outcome, and `execute_claim` commits the monitor state after `complete_success` succeeds. A failed delivery returns `Err`, so nothing is committed.
- `src/cron/executor.rs`: `AgentCronExecutor::execute_with_outcome` no longer writes `cron_monitor_states` after `backend.run`. The write moved to `commit_monitor_state` (pub(crate)). `execute()` keeps the old commit-on-success behaviour for direct callers, so `tests/test_fix_fix_cron_exec.rs` is unaffected.
- `src/inbox/digest.rs::DigestExecutor` forwards `execute_with_outcome` to the inner executor. Without this, the wrapper would hide the pending state and the inner `execute` would commit immediately.
- `src/cron/mod.rs` re-exports `CronRunOutcome`.
- Test: `scheduler::tests::monitor_change_is_committed_only_after_successful_delivery`. A failing dispatcher leaves 0 rows. A healthy retry delivers the change and leaves 1 row.

## P1-3 Backoff retry treated as a missed occurrence: FIXED
- `src/cron/scheduler.rs`:
  - `complete_failure` stores `payload.retry_at` (the backoff `next_run_at`), including in the `reconcile_job_after_run` path.
  - `claim_job` skips the whole catch-up/missed block when `is_backoff_retry` is true (`retry_at == next_run_at`). The run then claims with `occurrence = None`.
  - `complete_success` clears `retry_at` for scheduled runs. A manual run keeps a pending marker.
  - `reconcile_job_after_run` copies or clears `retry_at` together with the schedule.
- `src/cron/store.rs` sync: preserves `retry_at`, and excludes it from the payload-diff comparison so a sync does not reset `next_run_at`.
- Test: `scheduler::tests::backoff_retry_of_cron_expression_job_is_not_a_missed_occurrence`. It checks the marker is set on failure, an off-grid retry is claimed with no `missed` problem, and success clears the marker.

## P2 Sync overwrites payload, drops `last_occurrence`: FIXED
- `src/cron/store.rs::HermesStoreSynchronizer::sync_at` upsert: `json_set` now also preserves `$.last_occurrence`.
- Test: `store::tests::synchronization_preserves_scheduler_last_occurrence`.

## P2 `needs_owner` mints a token for an already-fixed problem: FIXED
- `src/cron/remediate.rs::needs_owner`: if the `status!='fixed'` UPDATE affects 0 rows, it rolls back and returns `Ok(())`. No inbox row, outcome or token is created.
- Test: `remediate::tests::needs_owner_mints_no_token_for_an_already_fixed_problem`.

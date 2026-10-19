# fix: cron-scheduler (write scope: src/cron/scheduler.rs, src/mirror.rs, src/storage/db.rs)

No compile/test command was run (per directive). All edits are by hand; the lead's integration gate must verify.

## cron-scheduler lane (CONFIRMED P1)

- **[P1] complete_success next_run_tz()? rolls back the completion / claim_job error blocks later jobs** -> FIXED
  - src/cron/scheduler.rs: `complete_success` computes the next run before the CAS UPDATEs; on error it logs and takes the existing "disable job" path (same behavior as `complete_failure`), so the run commits as succeeded.
  - src/cron/scheduler.rs: new `next_future_or_retire` used by the three `next_run_tz(..)?` calls in `claim_job` (catch_up 0, out-of-window, already-fired occurrence): an uncomputable next occurrence disables the job instead of erroring.
  - src/cron/scheduler.rs: `run_due_jobs` no longer `?`-propagates a per-job `claim_job` error; it logs and continues with the remaining due jobs (covers errors from `newest_missed_occurrence` / bad timezone / DB).
  - Tests: `complete_success_retires_job_with_no_further_occurrence`, `run_due_jobs_isolates_a_job_whose_claim_errors`.
- **[P1] should_reclaim trusts a live owner_pid (PID reuse / same-process executor panic)** -> FIXED (no migration needed)
  - src/cron/scheduler.rs: process-wide `ACTIVE_RUN_IDS` registry (inserted at the end of `claim_job`, removed by `ActiveRunGuard` drop at the end/unwind of `execute_claim`). In `claim_job`'s reclaim loop, when `owner_pid == current pid` liveness means "this run id is executing in this process"; otherwise it uses `ledger::is_process_alive`. An orphan row owned by our own PID (Docker PID reuse, panicked task) is reclaimed immediately. Rows from another live PID are still protected until the 120 min safety net. `should_reclaim` / `should_reclaim_with` signatures unchanged.
  - Residual: a PID reused by a *different* live process (not ours) still waits for the 120 min safety net; fixing that needs an `owner_started_at` column on cron_runs = migrations node (BLOCKED-SCOPE below, optional).
  - Test: `expired_lease_owned_by_this_pid_but_not_running_here_is_reclaimed` (existing live-process test `test_claim_job_stores_owner_pid_and_reclaim_skips_live_process` still holds: its claim stays registered).
- **[P1] resolve_predecessor_output legacy fallback `LIKE '%:<id>'` cross-session disclosure** -> FIXED
  - src/cron/scheduler.rs: fallback pattern is now `%:cron:<escaped id>`, so only cron-owned sessions match (exact `cron:<id>` / `discord:<profile>:cron:<id>` unchanged).
  - Test: `predecessor_fallback_does_not_read_non_cron_sessions` (existing `test_resolve_predecessor_output_from_db_and_disk` and the wildcard tests still match).
- **[P2] obligation id ms collision** -> FIXED: `deliver` id is now `obl:cron:<job>:<ms>:<uuid>`. Test: `fan_out_deliveries_in_one_millisecond_keep_distinct_obligations`.
- **[P2] cron_outputs insert error swallowed** -> FIXED: error is logged at error level (kept non-fatal on purpose: failing inside the completion tx would re-run an already executed job).
- **[P2] interval overflow panic** -> FIXED: `next_run` uses `checked_add_signed` (returns Config error); `newest_missed_occurrence` uses i64 checked millisecond math instead of `delta * (count as i32)`. Test: `interval_arithmetic_does_not_panic_on_overflow`.
- **[P2] ShellAndPayload executor has no timeout (not prod)** -> SKIPPED: not production path per lane report; a timeout needs a policy decision on the max runtime.

## storage-ledger items in mirror.rs / db.rs

- **[P1] find_last_unfinished_user_turn masked by mirror rows** -> FIXED
  - src/storage/db.rs: SQL skips rows with `$.mirror = 1` (guarded by `json_valid`, so legacy non-JSON metadata rows count as normal rows) before `ORDER BY sequence DESC LIMIT 1`. Also stops a mirror `user` row being re-dispatched as a fabricated turn.
  - Test: `find_last_unfinished_user_turn_skips_mirror_rows`.
- **[P2] take_pending_cron_briefings: json_valid guard, mark only rendered rows** -> FIXED
  - src/mirror.rs: all `json_extract` are inside `CASE WHEN json_valid(..)`; selection renders newest-first within `MAX_BRIEFING_CHARS`; only rows actually rendered are marked injected; older overflow stays pending. New `peek_pending_cron_briefings` (no side effects, returns `CronBriefings{text,message_ids}`) + `mark_cron_briefings_injected`; `take_pending_cron_briefings` kept as peek+mark wrapper (same signature/behavior for existing caller and test).
  - Test: `cron_briefings_survive_malformed_metadata_and_oversize_backlog`.
- **[P2] fetch_resume_pending_session_keys rebuilds a different key for unparsable keys** -> FIXED: query selects the raw key only; unparsable keys are skipped with a warn (cannot be turned into a usable `SessionKey`; returning a rebuilt key was the bug). Existing test `test_resume_pending_flag_lifecycle_and_queries` updated to canonical storage keys (it used `'sess-1'`, which is noncanonical). Test: `fetch_resume_pending_skips_unparsable_keys`.
  - Note: a skipped noncanonical row keeps `resume_pending=1` and is no longer reported to recovery or the restart-loop gate. The real fix for that row is a data repair; the API returns `SessionKey` so the raw string cannot be used for SQL without a signature change that touches main.rs/actor.rs (outside scope).
- **[P2] is_session_suspended fails open** -> FIXED: parse failure returns `true` (suspended) with a warn. Test: `is_session_suspended_fails_closed_on_malformed_state`.
- **[P2] mark_session_suspended non-atomic RMW** -> FIXED: single `UPDATE ... json_set(COALESCE(NULLIF(state_json,''),'{}'), '$.suspended', json(?))`; malformed JSON still errors and is not overwritten (existing test `mark_session_suspended_fails_on_malformed_state_without_overwriting` holds). Test: `mark_session_suspended_preserves_other_state_fields`. `router.rs` `set_model` has the same RMW pattern but is outside scope.
- **[P2] in-memory pool recycles its only connection** -> FIXED: `max_lifetime(None).idle_timeout(None)` when `in_memory`. Test: `in_memory_pool_never_recycles_its_only_connection`.
- **[P2] dead `recover_resume_pending_sessions` in db.rs** -> SKIPPED: two existing db.rs tests (`resume_pending_preserves_bot_identity`, the created_at recovery test) exercise it; deleting it means deleting those tests, which is more than a minimal fix. Recommend a follow-up.
- **[P2] pending write 8-hex id / memory store / message_search / messenger_policy edges** -> SKIPPED: `stage_pending_write` id length is user-typed in Discord commands (changing it is a UX decision); the others live in files outside this node's scope (memory/store.rs, message_search.rs, messenger_policy.rs).
- ledger/service.rs items (debounced batch reclaim, constituent writes, recovery loop `?`, sweep_recoverable NULL owner_started_at) -> not in this node's scope (src/ledger/service.rs).

## BLOCKED-SCOPE

1. **src/agent/omo_backend.rs:727** (agent-backend P2 "briefings marked injected before turn/start" + storage-ledger P2): replace `take_pending_cron_briefings` with
   `let briefings = crate::mirror::peek_pending_cron_briefings(pool, &key).await`, prepend `briefings.text` to the prompt, and call `crate::mirror::mark_cron_briefings_injected(pool, &briefings.message_ids)` only after `turn/start` is accepted (take once in `run()`, reuse for the model-reject fallback attempt). The new API is already in mirror.rs.
2. **migrations (optional)**: add `owner_started_at TEXT` to `cron_runs` and write it in `claim_job` to also defend against PID reuse by a different live process (currently covered only by the 120 min safety net). Not required for the confirmed triggers (same-process panic and restart with the same PID are covered by the in-process registry).

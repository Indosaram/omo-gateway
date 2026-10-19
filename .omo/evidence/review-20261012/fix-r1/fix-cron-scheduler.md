# fix-cron-scheduler

Files changed: `src/cron/scheduler.rs` (fixes + updated 2 existing unit-test call sites), new `tests/test_fix_fix_cron_scheduler.rs` (6 tests, one per behavior).
Logs: RED `fix-cron-scheduler-red.log` (unfixed source, same test file), GREEN `fix-cron-scheduler-green-final.log` (foreground rerun), related `fix-cron-scheduler-related.log`.

RED command: `cargo test --test test_fix_fix_cron_scheduler` (against unfixed scheduler.rs) -> `test result: FAILED. 0 passed; 6 failed`
GREEN command: `cargo test --test test_fix_fix_cron_scheduler` -> `test result: ok. 6 passed; 0 failed`
Regression: `cargo test --lib cron::scheduler` -> `test result: ok. 28 passed; 0 failed`

## 1. [P1] huge catch_up_hours panic — FIXED
- `CronScheduler::claim_job`: `window_start` now uses `i64::try_from` + `TimeDelta::try_hours` + `checked_sub_signed`, falling back to `DateTime::MIN_UTC` (= catch up everything) instead of panicking.
- Test: `huge_catch_up_hours_does_not_panic_scheduler`
- RED: `panicked at chrono-0.4.45/src/lib.rs:717:17: TimeDelta::hours out of bounds`
- GREEN: `test huge_catch_up_hours_does_not_panic_scheduler ... ok`

## 2. [P1] stale-payload CAS skips schedule advance — FIXED
- `complete_success` / `complete_failure`: every `UPDATE ... AND payload_json = ?` now checks `rows_affected()`; on 0 it calls new helper `CronScheduler::reconcile_job_after_run`, which re-reads the current row, merges last_status/last_run_at/last_occurrence/last_error/last_delivery_error into the current payload (keeping the mid-run edit), bumps repeat counter, and writes next_run_at (if enabled) / disable by `WHERE id = ?`. Schedule is only advanced if the expression is unchanged.
- Tests: `success_advances_schedule_after_mid_run_payload_edit`, `failure_advances_schedule_after_mid_run_payload_edit` (also assert a second `run_due_jobs` claims 0 => no double delivery).
- RED: `next_run_at must advance past the executed occurrence, got Some(2026-08-16T11:55:00Z)` (both)
- GREEN: both `ok`

## 3a. [P2] failure backoff / resume ignore schedule.timezone — FIXED
- `next_run_after_failure` gained `timezone: Option<&str>` and uses `next_run_tz` (pub fn; only other reference is the re-export in `src/cron/mod.rs`, signature-agnostic); `complete_failure` passes payload schedule.timezone; `resume` reads payload timezone and uses `next_run_tz`. Two existing unit tests updated with `, None`.
- Test: `failure_backoff_and_resume_respect_schedule_timezone`
- RED: `failure path left: Some(2026-08-17T09:00:00Z) right: Some(2026-08-17T00:00:00Z)`
- GREEN: `ok`

## 3b. [P2] repeat limit reached via manual runs never disables — FIXED
- `claim_job`: when `should_disable_after`, now `UPDATE cron_jobs SET enabled = 0, next_run_at = NULL, updated_at = ? WHERE id = ?` before returning `Ok(None)`.
- Test: `repeat_limit_reached_by_manual_run_disables_job`
- RED: `exhausted job must be disabled`
- GREEN: `ok`

## 3c. [P2] predecessor LIKE wildcard injection — FIXED
- `resolve_predecessor_output`: SQL is `session_key LIKE ? ESCAPE '\'`; bind value escapes `\`, `%`, `_` in job_id; suffix match `%:<id>` preserved.
- Test: `predecessor_lookup_escapes_like_wildcards`
- RED: `left: Some("other job output") right: None`
- GREEN: `ok`

No BLOCKED-SCOPE items. No git commit/stash, no .env/DB touched. fmt/clippy not run (per instructions); formatting kept rustfmt-style by hand.

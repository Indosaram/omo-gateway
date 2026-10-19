# fix-migrate report (fix-r1)

RED/GREEN: deferred to single final integration gate (user directive: STOP ALL BUILDS).
Earlier runs from this node, captured before the stop directive, are kept as logs and summarized below.
Nothing was compiled or run after the directive.

## Files changed (write scope only)
- src/migrate/mod.rs - `run_after_config`
- src/migrate/cron_cutover.rs - `canonical_job_payload_matches`
- src/migrate/config_import.rs - `ACCESS_CONTROL_KEYS` const, `map_values`, unit tests
- tests/test_fix_fix_migrate.rs - new regression tests

## Defect -> fix mapping

### 1. [P1] Hermes stopped before cutover validation (mod.rs) - FIXED
- Fix: `run_after_config` now calls `cutover_cron_stores(env, &paths.hermes_root, &pool, true)` (dry-run) before `bring_gateway_down`. If any store has `unverified_jobs`, it returns `step_error("cron cutover", ...)` and leaves processes and plists untouched.
- Test: `tests/test_fix_fix_migrate.rs::cutover_validation_failure_leaves_hermes_gateway_running`
- RED (fix-migrate-red.log, `cargo test --test test_fix_fix_migrate`): `panicked at tests/test_fix_fix_migrate.rs:58:5: Hermes must not be stopped`
- GREEN (fix-migrate-green.log, same command): `test result: ok. 3 passed; 0 failed`

### 2. [P1] Cutover refused for import-injected keys (cron_cutover.rs) - FIXED
- Fix: in `canonical_job_payload_matches`, the normalized DB payload drops `_omon_discord_home_channel` and `_omon_discord_home_thread` when the source job lacks them. It also drops `schedule.timezone` (or sets it to the source's null) when the source has no timezone. A timezone that the source sets explicitly is still compared.
- Tests: `cutover_accepts_jobs_imported_with_config_timezone_and_home_channel` (fixture config.yaml has `timezone: Asia/Seoul` and `discord.home_channel`/`thread_id`), plus the guard `cutover_still_rejects_changed_source_timezone`.
- RED (fix-migrate-red.log): `assertion failed: summary.stores[0].unverified_jobs.is_empty()` with `unverified_jobs: ["daily"]`
- GREEN (fix-migrate-green.log): `3 passed; 0 failed`

### 3. [P1 security] Profile .env access-control leaked globally (config_import.rs) - FIXED
- Fix: in `map_values`, keys in `ACCESS_CONTROL_KEYS` (DISCORD_ALLOWED_USERS/CHANNELS/ROLES, DISCORD_ALLOW_ALL_USERS, DISCORD_IGNORED_CHANNELS, DISCORD_FREE_RESPONSE_CHANNELS, DISCORD_HOME_CHANNEL) never fall back to `profile_value`. A root value is inserted only when the key is not already set from root config.yaml.
- Tests: new `migrate::config_import::tests::profile_access_control_never_imported_globally`. The existing `root_scalar_values_win_over_profile_fallbacks` expectation changed from `Some("99")` (the leaked profile value) to `None`.
- RED (fix-migrate-red-config.log, `cargo test --lib migrate::`): `left: Some("99") right: Some("42")` and `left: Some("99") right: None`
- GREEN (run in this session before the stop directive, `cargo test --lib migrate::`): `test result: ok. 51 passed; 0 failed`

## Lead finding: tests/test_migrate.rs regressions - NOT caused by these edits (analysis only, not run)
Failing tests: `cutover_survives_startup_sync` (claimed 0, expected 1) and `cutover_second_store_failure_preserves_bytes_and_receipt` (claimed 0, expected 2). Reproduced once in this session before the stop directive.

Why my edits are not the cause:
- Both assertions fail at the final `scheduler.run_due_jobs()` claim, after migration succeeded, authority is `omon_owned`, and the receipt is `committed`. Those earlier asserts pass, so cutover ran correctly.
- The defect 1 precheck is a dry run: `reconcile_pending_cutover` and the locks are skipped, and it returns before any write. The defect 2 normalization only changes the comparison and never writes rows.

Most likely cause (time-dependent test, pre-existing since commit 754e2f4 "catch-up ... parity", Oct 8):
- The tests set `next_run_at = Utc::now() - 120s` (or `- 60s`) for the job `0 9 * * *`.
- `claim_job` then calls `newest_missed_occurrence(expr, scheduled_from = next_run, window_start = now - 6h, now)`.
- That function searches only from `max(scheduled_from, window_start)`, so it finds a 09:00 occurrence only if the wall clock is within 1-2 minutes after 09:00 UTC. Otherwise it returns `None`, the job is rescheduled, and nothing is claimed.
- The same logic exists in HEAD (`git show HEAD:src/cron/scheduler.rs`, line ~1246/2275). The only recorded passes are from Sep 7-8 evidence, before that commit.

Suggested follow-up for the owner of tests/test_migrate.rs or src/cron/scheduler.rs (outside my write scope): set `next_run_at` to the actual most recent `0 9 * * *` occurrence within the catch-up window, or inject the scheduler clock (`with_options`) so the tests are deterministic. Confirm at the final integration gate by running `cargo test --test test_migrate` on HEAD without these edits.

## Pending for the final integration gate
- `cargo test --test test_fix_fix_migrate`
- `cargo test --lib migrate::`
- `cargo test --test test_migrate` (the 2 failures above are expected to be pre-existing and time-dependent)
- fmt/clippy are run by the lead.

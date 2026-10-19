# fix-r4-cron

Both failures were stale test fixtures. Production code is correct and was not changed.

## 1) tests/test_cron_executor_parity.rs::agent_result_uses_scheduler_delivery

**Root cause (test fixture):**
- Case 1 and Case 2 both force `next_run_at = datetime('now','-1 minute')`. SQLite `datetime()` has one-second resolution.
- Both runs happen within the same second, so both forced timestamps are identical.
- With an `every 1h` job, `claim_job` (src/cron/scheduler.rs ~1260-1330) computes the occurrence through `newest_missed_occurrence`. For interval schedules that is `scheduled_from + n*delta`, which equals `next_run_at` here.
- The first run persisted that occurrence as payload `last_occurrence`.
- The second claim computes the same `occ`, so `last_occ == Some(&occ_key)` (scheduler.rs ~1304-1332). It records a missed occurrence, advances `next_run_at`, and returns `Ok(None)`. `run_due_jobs` therefore returns 0, which is the observed `left: 0, right: 1` at line 330.
- This is the intended occurrence-keyed dedup, so production is right. The test reused one occurrence and expected a second delivery.
- I did not verify the string-comparison hint (`datetime()` text vs RFC3339). The due query binds a `DateTime<Utc>`, and the claim path decoded `datetime()` text fine in Case 1.

**Fix (test only):** Case 2 now forces `datetime('now','-2 minutes')`, a distinct occurrence, with a comment explaining why. Assertions are unchanged.

**Result:** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out`

## 2) tests/test_cron_boundary_parity.rs::sync_isolates_bad_jobs_and_rearms_changed_once

**Root cause (test fixture):**
- The profile `config.yaml` sets `timezone: Asia/Seoul`, and the job `0 9 * * *` has no per-job timezone. The cron wall clock is therefore evaluated in Seoul.
- The phase 6 fixture used `next_run_at` and clock `2026-09-06T09:00:00Z`, which is 18:00 in Seoul.
- In `claim_job`, `next_run <= now` holds, so it calls `newest_missed_occurrence(expr, next_run, window_start, now, tz)` (scheduler.rs ~2461-2540). That function only accepts instants `>= scheduled_from`.
- The only Seoul 09:00 occurrence nearby is 2026-09-06T00:00Z, which is before `scheduled_from` 09:00Z. The result is `None`, so the job is skipped as "out of window" and `run_due_jobs` returns 0.
- Production behavior is correct. The fixture mixed UTC 09:00 with a Seoul-timezone cron expression.
- The sync-import hint (src/cron/store.rs ~1052-1081) is not involved: the explicit `next_run_at` was stored fine.

**Fix (test only):** Phase 6 now uses `2026-09-06T00:00:00Z`, which is 09:00 Asia/Seoul, for the clock, the `next_run_at` in jobs.json, and `completed_at` (`00:01:00Z`). The assertion message now says "09:00 Asia/Seoul". Assertions are not weakened.

**Result:** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out`

## Scope
Only tests/test_cron_executor_parity.rs and tests/test_cron_boundary_parity.rs were edited. Each target test was run once, singly, with cargo, and no build, clippy or full suite was run. No commit was made.

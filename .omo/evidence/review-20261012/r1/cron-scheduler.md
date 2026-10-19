# cron-scheduler review

### [P1] Large `catch_up_hours` panics the scheduler loop and stops all cron jobs
- Location: src/cron/scheduler.rs:1245
- Trigger: A job payload has a very large `catch_up_hours` (for example 10000000000), set through Hermes jobs.json or a dashboard payload. Hermes sync stores it as a `u64` via store.rs and nothing bounds it. When the job is overdue, the poll loop runs `run_due_jobs` -> `claim_job`, which computes `TimeDelta::hours(catch_up_hours as i64)` and `now - ...`. That call panics when the value is out of range, or the date subtraction panics on overflow.
- Impact: The panic happens inside the task spawned by `start()`, so the whole scheduler loop dies. No cron job runs again until the process restarts, and the same job re-panics the loop on every restart.
- Fix: Clamp `catch_up_hours` to a sane maximum (for example 24*365), or use `TimeDelta::try_hours` plus `checked_sub_signed` and skip or log the job when the value is out of range.

### [P1] Completion is lost and the same occurrence re-runs if the payload is edited during a run
- Location: src/cron/scheduler.rs:1545
- Trigger: A recurring job is running (agent runs take minutes). During that time the cron tool's `update` action (tools/cron.rs:479) changes the prompt, which rewrites `payload_json`. The update keeps `next_run_at` at its old past value because the expression did not change. When the run finishes, `complete_success` issues `UPDATE ... WHERE ... AND payload_json = ?` against the stale payload, so it affects 0 rows. `rows_affected` is never checked and the transaction commits.
- Impact: `next_run_at`, `last_occurrence`, `last_status` and the repeat counter are never written. On the next poll, `claim_job` finds the same missed occurrence with no matching `last_occurrence`, so the job runs again and delivers its output a second time. The failure path has the same compare-and-swap pattern (lines 1676-1765).
- Fix: Check `rows_affected()`. On a mismatch, re-read the row and apply the schedule advance and `last_*` fields onto the current payload with `json_set`, or at least advance `next_run_at` with `WHERE id = ?`.

### [P2] Retry after failure ignores the job's timezone and reschedules at the wrong wall-clock time
- Location: src/cron/scheduler.rs:1704
- Trigger: A job with `schedule.timezone` fails (for example the inbox digest, `0 9 * * *` in Asia/Seoul, registered via digest.rs). `complete_failure` calls `next_run_after_failure`, which uses UTC-only `next_run` (line 2427) instead of `next_run_tz`.
- Impact: A failure at 09:00 KST (00:00 UTC) reschedules the job for 09:00 UTC, which is 18:00 KST on the same day, instead of 09:00 KST the next day. Any timezone-aware job runs off-schedule after a failure.
- Fix: Pass the payload's timezone into `next_run_after_failure` and use `next_run_tz` there.

### [P2] `resume` ignores the job's timezone
- Location: src/cron/scheduler.rs:964
- Trigger: Dashboard `resume` (dashboard.rs:1611, 1559) or cron tool `resume` (tools/cron.rs:307) on a job that has `schedule.timezone`. The method computes `next_run(&job.expression, now)`, which is evaluated in UTC.
- Impact: The first run after resume fires at the UTC interpretation of the cron fields, hours away from the intended local time.
- Fix: Read the timezone from the payload and call `next_run_tz`, the same way `register`/`register_with_id` do.

### [P2] Hitting the repeat limit through manual triggers leaves the job enabled and re-claimed every poll
- Location: src/cron/scheduler.rs:1200
- Trigger: A job has `repeat.times = N` and is run manually N times through dashboard `trigger` or the cron tool `run` (`advance_schedule = false`). `complete_success` and `complete_failure` increment `completed` but take the payload-only update branch, so the job is never disabled. Once its `next_run_at` passes, `run_due_jobs` selects it on every poll. `claim_job` logs the "repeat limit reached" warning and returns `None` without disabling the job or advancing `next_run_at`.
- Impact: The job shows as enabled but never runs. It also emits a warning and makes several DB queries every poll interval (1s) indefinitely.
- Fix: When `should_disable_after` is true in `claim_job` with `require_due`, set `enabled = 0, next_run_at = NULL`, or apply the limit check in the manual-run completion branch.

### [P2] Predecessor-context fallback uses an unescaped suffix LIKE and can pull another profile's output
- Location: src/cron/scheduler.rs:153
- Trigger: `AgentCronExecutor` calls `resolve_predecessor_output` with a Hermes-native `context_from` id such as `daily`. The exact lookup in `cron_outputs` misses, because jobs are stored as `hermes:{profile}:daily`, so the code falls back to `session_key LIKE '%:daily'`. That pattern matches any profile's `cron:hermes:<profile>:daily` session. It also treats `_` and `%` in ids as wildcards.
- Impact: The newest assistant message from a different profile's job, or an unrelated job, can be injected into the prompt and then delivered to this job's Discord destination. This leaks content across profiles.
- Fix: Escape LIKE metacharacters (`ESCAPE '\'`), and scope the fallback to the exact profile-qualified job id, or drop the suffix match.

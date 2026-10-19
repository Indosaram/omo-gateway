# cron-store-exec review
### [P1] Owner-approval gate for publishing reruns is bypassable by any same-user process, including the remediation lane agent
- Location: /Users/indo/code/project/omon-gateway/src/cron/report.rs:289 (server handler for `approve_problem`), :383 (CLI `ApproveRerun`), :85 (health problem query that exposes ids); /Users/indo/code/project/omon-gateway/src/cron/remediate.rs:410 (`approve_rerun`)
- Trigger: A publishing problem (e.g. `omon-playstore-daily-publish`) is in `needs_owner` with an open lane. The lane agent has a shell as the same user. It runs `omo-gateway cron health --json`, which lists `problems[].id`. It then runs `omo-gateway cron approve-rerun <id> --owner-approved`. The socket is only mode 0600 and the request is just `{"approve_problem":id}`. The `--owner-approved` flag is `required = true` but its value is discarded (`owner_approved: _`) and never sent. The handler calls `remediate::approve_rerun`, which sets `owner_approved=true` and reruns the publishing job.
- Impact: The "explicit owner approval before an external-effect rerun" control is only a prompt instruction ("Do not execute the Done command yourself"). Any local process or agent can approve its own publish and trigger the real publish. Non-publishing `run-now` is correctly blocked at report.rs:~293 for publishing jobs, but this path is not.
- Fix: Authenticate approvals with something an agent cannot forge. Options: a per-approval nonce or token delivered only via the owner's Discord/inbox channel, or a separate privileged socket/credential not exposed to lane or cron subprocess environments. At minimum, have the server reject `approve_problem` unless the request carries an owner token, and drop problem ids from agent-reachable `health --json`.

### [P1] A problem stranded in `remediating` is never escalated, and recurrences of the same failure are silently suppressed
- Location: /Users/indo/code/project/omon-gateway/src/cron/remediate.rs:260 (claim to `remediating`), :340-:390 (`verify_once_inner` reserves a rerun, then awaits `runtime.rerun`); /Users/indo/code/project/omon-gateway/src/cron/health.rs:85-86 (upsert reopens only `fixed`/`ignored`)
- Trigger: The gateway restarts or crashes (launchd restart, restart-loop guard) after the claim at :260 but before the final status write at :381. A `transient_retry_once` rerun can run up to the script timeout, 30 min by default, so the window is large. The problem stays `remediating` with the rerun already counted. The un-receipted `cron_problem` event is redelivered on restart, but `on_problem_inner` only handles `status='open'` and returns `Ok(())`, so the event is receipted and dropped. For non-publishing transient problems there is no `lane_key`, so no lane settlement will ever call `verify_once` again.
- Impact: The problem is stuck in `remediating` with no owner notification. The job keeps failing with the same signature, and health.rs:85-86 does not reopen or re-emit an event for a `remediating` row. The failure is invisible except in `cron health`, which shows it as "remediating".
- Fix: Add a startup or periodic sweep. For `remediating` rows with no live lane and `updated_at` older than a bound, either move them to `needs_owner` via `needs_owner`, or reset them to `open` so `on_problem` re-runs. Alternatively, have the health upsert re-emit an event when the row is `remediating` and stale.

### [P1] Problem signature includes the full error text, so a flapping job fans out into unbounded problems, judge calls and lanes
- Location: /Users/indo/code/project/omon-gateway/src/cron/health.rs:49-58 (`detail` is the run error), :73-74 (`signature = "{kind}:{detail}"`), :80-86 (dedup is `UNIQUE(job_id, kind, signature)`)
- Trigger: A script job fails every few minutes with stderr that varies per run. Examples are a timestamp, pid, request id or elapsed time in a traceback or log line. The executor error is `"cron script failed with {code}: {stderr}"` (executor.rs). Each run produces a distinct signature, so a new `cron_problems` row is inserted and a `cron_problem` lane event is enqueued. `on_problem` then makes a judge call per row, and `script_bug_needs_fix` opens a new agent lane with a new `lane_key`.
- Impact: Unbounded growth of problems, judge/LLM spend, inbox items and agent lanes for one underlying fault. The per-problem `MAX_RERUNS` budget is meaningless because every occurrence is a fresh problem.
- Fix: Derive the signature from a normalized, bounded form of the error. For example, `kind` plus a hash of the first line with digits, hex and timestamps stripped. Alternatively, skip inserting a new problem for a job that already has an open, remediating or needs_owner problem of the same kind.

### [P2] Raw error text and logs reach Discord digest, inbox and lane briefs without scrubbing
- Location: /Users/indo/code/project/omon-gateway/src/cron/report.rs:175-177 (`digest_section` renders `Detail::parse(..).cause` raw); /Users/indo/code/project/omon-gateway/src/cron/remediate.rs:315 (`needs_owner` writes `action`, which embeds `detail.cause`, into `inbox_items.snippet` and `inbox_decisions`); `make_lane` brief (remediate.rs ~:465-:500, embeds runs' errors, output, script and 80 log lines)
- Trigger: A script's stderr, for example a curl error line containing a URL token or an echoed env value, becomes the run error. Only `queue_outcome` (report.rs:171) and the judge input (remediate.rs:255) call `lanes::scrub::scrub`. The digest section, inbox snippet and lane brief file do not.
- Impact: Secrets or home paths in cron error text are posted to Discord in the daily digest and persisted in the inbox and brief files.
- Fix: Apply `lanes::scrub::scrub` to `cause`, `action` and the brief fields before rendering or persisting them.

### [P2] Duplicate and contradictory owner escalation after a failed transient rerun
- Location: /Users/indo/code/project/omon-gateway/src/cron/remediate.rs:285-:290 together with :395-:405
- Trigger: In the `transient_retry_once` branch, `verify_once` already calls `needs_owner` with "Verification failed..." when the receipt is unverified. `on_problem_inner` then calls `needs_owner` a second time with a different message when `!receipt.verified`.
- Impact: Two outcome rows (the `UNIQUE(problem_id,message)` key differs) and two digest lines. The `inbox_decisions` upsert overwrites `proposed_action`.
- Fix: Drop the second `needs_owner` call at :288, since `verify_once_inner` already escalates.

### [P2] Approving after the rerun budget is exhausted can never succeed
- Location: /Users/indo/code/project/omon-gateway/src/cron/remediate.rs:362 (budget check), :410-:430 (`approve_rerun` sets `owner_approved` but does not reset `reruns`)
- Trigger: A problem has `reruns >= 3` and is in `needs_owner`. The message says "Approve further remediation; rerun budget exhausted". The owner approves. `approve_rerun` flips the status to `remediating` and commits, then `verify_once_inner` sees `reruns >= MAX_RERUNS`, rolls back, re-escalates and returns `None`. The CLI then reports "cron rerun budget exhausted or already fixed".
- Impact: The owner is told to approve, but approval is a no-op that flaps the status. Only a manual `run-now` helps.
- Fix: Reset or extend the `reruns` budget on owner approval, for example `detail.reruns = 0`. Alternatively, change the escalation text.

### [P2] Retention days can panic at startup, and pruning only runs at boot
- Location: /Users/indo/code/project/omon-gateway/src/cron/store.rs:99-:127 (`cron_runs_retention_days_from_environment`, `prune_terminal_cron_runs`); /Users/indo/code/project/omon-gateway/src/main.rs:971-:972 (only call site)
- Trigger: `CRON_RUNS_RETENTION_DAYS=999999999999` passes the `>= 0` check. `now - chrono::TimeDelta::days(retention_days)` at :127 then panics on overflow instead of returning a `Config` error. Separately, pruning runs only once at startup, so a long-lived gateway grows `cron_runs` unbounded between restarts.
- Impact: Startup panic on a mis-set env var. Unbounded table growth over long uptime.
- Fix: Use `TimeDelta::try_days` and `checked_sub_signed` and return `OmonError::Config` on overflow. Schedule the prune periodically, for example from the scheduler loop.

### [P2] Ack timeout kills only the `sh` process, leaving grandchildren running
- Location: /Users/indo/code/project/omon-gateway/src/cron/ack.rs:31-:46
- Trigger: An ack command such as `sh -c "python commit.py"` hangs. `kill_on_drop` kills only the direct `sh` child. Unlike the script runners in executor.rs, no `process_group(0)` is set and there is no group kill.
- Impact: The orphaned process keeps running past the 60 s bound, and stacked acks can accumulate.
- Fix: Spawn with `process_group(0)` on unix and `libc::kill(-pid, SIGKILL)` on timeout, as executor.rs does for scripts.

### [P2] Native socket hardening and shutdown behavior
- Location: /Users/indo/code/project/omon-gateway/src/cron/report.rs:248-:250 (bind then chmod), :280-:285 (unbounded `read_line`, unbounded task spawn), :305-:310 (shutdown drains all connections)
- Trigger: (a) The socket is created under the process umask, and `set_permissions(0o600)` runs afterwards. With a permissive umask there is a window when other local users could connect. (b) `read_line` has no size cap. (c) On shutdown, `while let Some(..) = connections.join_next().await` waits for in-flight `trigger_wait` runs, which can take up to the job timeout. The socket file is not removed until then, so graceful shutdown can hang.
- Impact: Local hardening gap, memory pressure from a long line, and slow or stalled shutdown.
- Fix: Set umask or create the parent dir 0700 before bind. Cap the request with `take(N)`. On shutdown, abort or timebox connection tasks.

### [P2] Notepad quota check is not atomic, and notepad keys can mismatch between writer and reader
- Location: /Users/indo/code/project/omon-gateway/src/cron/store.rs:1125-:1155 (read-then-upsert); /Users/indo/code/project/omon-gateway/src/cron/executor.rs:235-:236 (reads with `hermes.profile()` and `hermes.id`); /Users/indo/code/project/omon-gateway/src/tools/cron.rs:814-:817 (tool defaults `profile` to "default")
- Trigger: (a) Two concurrent `set` calls for different keys each pass the 64 KiB total check against the same snapshot, then both write. (b) For a Hermes job with `origin.bot_id` or `origin.platform`, `HermesJob::profile()` returns that value (store.rs, `profile()`), not "default". The executor therefore reads a different `(profile, job_id)` than a tool call that omits `profile` writes to. In addition, `execute_native_cron` never injects notepads at all.
- Impact: The total-size cap can be exceeded. Notes written via the tool default are never shown to some Hermes jobs, and notes for native jobs are never shown.
- Fix: Do the check and upsert in one transaction, or one SQL statement with a SUM guard. Use one canonical profile and job-id derivation for both the tool and the executor.

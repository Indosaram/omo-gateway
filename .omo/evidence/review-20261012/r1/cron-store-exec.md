# cron-store-exec review

I did not run cargo build or tests. Every finding was traced by reading the code and its callers.

### [P1] Ack command runs before delivery, so a failed delivery loses the checkpoint
- Location: src/cron/executor.rs:357 (same pattern at src/cron/executor.rs:561 in `execute_native_cron`)
- Trigger:
  - A Hermes or native cron job has `ack_command` (for example the kakao digest checkpoint commit). `AgentCronExecutor::execute` puts it into `session.state.metadata["cron_ack_command"]`.
  - It also sets `cron_suppress_direct_emission=true`, so `OmoBackend::deliver_rendered_result` returns early without delivering anything (src/agent/omo_backend.rs:628-637).
  - The caller (src/agent/omo_backend.rs:1043-1057 and :1650-1662) then runs `cron::ack::run_ack_logged(ack_command)` right after the turn completes. At that point nothing has been delivered yet.
  - Real delivery happens afterwards in `CronScheduler::execute_job_with_outcome` (src/cron/scheduler.rs ~1800-1852). If `self.deliver` fails (Discord outage, bad destination), the job is marked failed, but the checkpoint was already committed.
- Impact:
  - This breaks the contract stated in src/cron/ack.rs ("runs AFTER the job's output was successfully delivered"). Items the digest producer checkpointed are never re-sent, and the user loses that content permanently.
  - On success the ack also runs twice: once in the backend and once at scheduler.rs:1852.
- Fix: Stop setting `cron_ack_command` in the session metadata for scheduler-delivered jobs (both executor.rs sites). The scheduler already runs the ack after confirmed delivery at scheduler.rs:1852. Alternatively, have the backend skip its ack when `cron_scheduler_delivery` is set.

### [P1] Monitor snapshot is committed before the agent run and delivery succeed
- Location: src/cron/executor.rs:166
- Trigger:
  - A job has `monitor_script` or `monitor_url`, and the monitored output changes.
  - `AgentCronExecutor::execute` upserts the new hash into `cron_monitor_states` (lines 166-177) before the remaining steps.
  - Any later step can still fail:
    - the `brief_file` read at ~182
    - `load_cron_skills`
    - the prompt-injection scan
    - `backend.run`
    - the scheduler's delivery, which runs after `execute` returns
  - The job is then recorded as failed.
  - On the next run the hash equals the stored one, so line ~152 returns `Ok(None)` ("monitor state unchanged").
- Impact: The change that should have triggered the agent is silently dropped. Retries after backoff are suppressed and the notification is never produced.
- Fix: Persist the new hash only after `execute` has succeeded and its output was delivered. For example, carry the pending hash in the result and commit it from the scheduler's success path, or have the executor write it after `backend.run` returns Ok. At minimum, move the upsert to the end of `execute`.

### [P1] Panic on non-ASCII `deliver` strings (byte-slice at a non-char boundary)
- Location: src/cron/store.rs:640 (same defect at src/cron/store.rs:599)
- Trigger:
  - A Hermes job's `deliver` or `failure_deliver` is any string of 8 or more bytes whose 8th byte falls inside a multibyte character. Examples: `"디스코드:123"` (Korean, 3 bytes per character) or `"discordé"`.
  - `trimmed.len() >= 8 && trimmed[..8]` then slices at a non-boundary and panics.
  - Callers:
    - `delivery_destination` -> `delivery_destinations` -> `HermesJob::discord_destinations` (scheduler.rs ~2115) in `execute_job_with_outcome`.
    - `has_explicit_discord_destination` (scheduler.rs:1793) in the same function.
    - `discord_destination()` at executor.rs:287.
  - `deliver` comes from external `jobs.json`. `validate()` does not check it, so such jobs are imported.
- Impact:
  - The claim task panics. The run row stays `running` until the lease expires, and no failure or health record is written.
  - Each subsequent claim panics the same way, so the job never delivers or records an error.
- Fix: Replace the slice with a boundary-safe check, for example `trimmed.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("discord:"))` with `trimmed.get(8..)`. Or use `strip_prefix` on a lowercase copy. Apply it at both sites.

### [P1] Re-opened problem reuses a stale `done`/`dismissed` inbox decision, so the owner is never notified
- Location: src/cron/remediate.rs:321
- Trigger:
  - A problem is raised and `needs_owner` creates the inbox row `cron:<id>` with status `open`.
  - The owner marks it `done` or `dismissed` (src/inbox/set.rs `update_status`).
  - The same job later fails with the same signature. The health upsert (health.rs:80-90) re-opens the same `cron_problems` row after `fixed` or `ignored`, so the id is unchanged.
  - `needs_owner` runs `INSERT INTO inbox_decisions ... ON CONFLICT(item_id) DO UPDATE SET proposed_action, lane_key`. That update does not reset `status`.
  - `INSERT INTO inbox_items ... ON CONFLICT(id) DO NOTHING` also keeps the old snippet.
- Impact:
  - The new owner request stays `done` or `dismissed` and is filtered out of the inbox listing, which shows only `open`/`proposed` (src/inbox/list.rs:51).
  - A recurring failure that needs the owner goes unreported.
- Fix: In the DO UPDATE clause, set `status='open'` and `decided_at=excluded.decided_at`, and update the item's snippet and received_at. Or key the inbox item per occurrence, for example `cron:<id>:<run_id or updated_at>`.

### [P2] `context_from` lookup uses the wrong profile key, so predecessor output is silently omitted
- Location: src/cron/executor.rs:234
- Trigger:
  - A Hermes job has `context_from` and an `origin` with `platform: "discord"`.
  - Lookup uses `profile = origin.platform` (`"discord"`) in `resolve_predecessor_output` (executor.rs:238).
  - Outputs are stored by `complete_success` with `claim.job.profile()` (scheduler.rs:1572). That is payload `profile`, else the `session_key` segment, else `""`. Imported Hermes jobs have no `session_key` and no `profile` key (the sync writes `_omon_hermes_profile`, not `profile`), so they are stored under `""`.
  - Both queries miss. The messages fallback also misses, because cron sessions with suppressed emission do not persist messages (omo_backend.rs:635).
  - `get_cron_notepads` at executor.rs:250 uses `hermes.profile()`, which is likewise not the profile that wrote the data.
- Impact: Context blocks and notepads are silently dropped from the prompt for discord-origin Hermes jobs. The agent runs with less context and no error is logged. I did not confirm what real `jobs.json` files carry in a `profile` key, so the miss depends on that data.
- Fix: Use one profile-resolution function for both reads and writes, for example prefer `extra["_omon_hermes_profile"]`. Apply it in `CronJob::profile` and `HermesJob::profile`, and use it at executor.rs:231-238.

### [P2] Sync deletes mirrored jobs (and cascades run history) when a job transiently fails validation or the store file is absent
- Location: src/cron/store.rs:1074
- Trigger:
  - `HermesStoreSynchronizer::sync_at` runs periodically (scheduler.rs:684).
  - A job that fails `validate()` (store.rs:1028) is `continue`d before `live.insert`. Example: a `brief_file` that does not exist at sync time.
  - A missing `jobs.json` makes `load()` return `Ok(vec![])` (store.rs ~893), so `live` is empty.
  - In either case the loop at 1070-1080 runs `DELETE FROM cron_jobs WHERE id=? AND authority='hermes_mirror'`.
  - `cron_runs` has `ON DELETE CASCADE` and `foreign_keys` is enabled (src/storage/db.rs:165).
- Impact:
  - The job, its `cron_runs` history, and any in-flight run row are deleted.
  - Re-import resets the payload, including `repeat.completed` and `last_status`, so `repeat.times` jobs can run again.
- Fix: Add the id to `live` before validation, so a job that exists in the source but is currently invalid is kept (and possibly disabled) rather than deleted. Skip the delete pass when the source file is missing.

### [P2] Digest ack marks remediation outcomes delivered even when the digest run delivered nothing
- Location: src/cron/report.rs:200
- Trigger:
  - A second `omon:inbox-digest` run happens on the same KST day after a successful delivery. Examples: a manual trigger or `run-now`, or a catch-up run.
  - `inbox::digest::build` returns `Ok(None)` at its `delivered > 0` check, before `digest_section` runs (src/inbox/digest.rs:80).
  - The run still succeeds, and `run_health_hook` (scheduler.rs:1430) calls `acknowledge_digest`. That UPDATE sets `delivered_at` on every outcome with `created_at <= run.started_at`, including outcomes created after the first digest was built.
- Impact: Those outcome messages are never shown in any digest.
- Fix: Acknowledge only when this run actually produced a delivered output, for example require a `cron_outputs` row for `run_id`. Better, record the outcome ids included in the message and mark only those.

### [P2] Outcome dedupe on `(problem_id, message)` suppresses repeat notices
- Location: src/cron/report.rs:170
- Trigger:
  - A re-opened problem reaches `needs_owner` again with the same action text. `owner_message` is deterministic, and the text embeds `detail.cause`.
  - `INSERT ... ON CONFLICT(problem_id,message) DO NOTHING` hits the existing, already delivered row.
- Impact: No new outcome row is created and the owner gets no digest entry for the recurrence.
- Fix: On conflict, reset `delivered_at=NULL` and refresh `created_at`, or include `run_id` or a timestamp in the message or key.

### [P2] A single `accept()` error shuts down the whole gateway
- Location: src/cron/report.rs:269
- Trigger:
  - `listener.accept()` returns a transient error such as EMFILE or ECONNABORTED.
  - `accepted.map_err(...)?` returns `Err` from `serve`. The socket file is not removed.
  - `serve_native` runs in `lane_tasks` (main.rs:1115), and the `lane_tasks.join_next()` arm in main.rs:1151 treats any exit as a lane-service failure and shuts the gateway down.
- Impact: A transient accept error stops the gateway.
- Fix: Log and continue on accept errors (with a short backoff), and break only on shutdown.

### [P2] `log_excerpt` reads the entire gateway log into memory
- Location: src/cron/remediate.rs:569
- Trigger:
  - `Remediator::on_problem_inner` calls `GatewayRuntime::log_excerpt` for each `script_bug_needs_fix` or publishing problem.
  - It runs `read_to_string` on `~/.omon/gateway.log`, which grows without a bound in this code.
  - A non-UTF-8 byte in the file makes the read return an error, which aborts remediation into `needs_owner`.
- Impact: Large allocation per problem on a long-running gateway, or failed remediation on a log containing invalid UTF-8.
- Fix: Read only the last N MiB (seek from the end) and decode lossily.

### [P2] Hard-coded developer path used as the lane working directory fallback
- Location: src/cron/remediate.rs:474
- Trigger: A failing job has no `workdir`, `cwd`, or absolute `script` path, so `make_lane` falls back to `PathBuf::from("/Users/indo/code/project/omon-gateway")`.
- Impact: On any other host or user, the remediation lane starts with a nonexistent or wrong cwd.
- Fix: Take the workspace root from configuration (for example pass `config.workspace_root` into `Remediator` or `GatewayRuntime`) instead of a literal path.

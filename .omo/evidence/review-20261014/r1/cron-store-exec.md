# Lane cron-store-exec

### [P2] Gateway lifecycle guard misses `launchctl bootout/bootstrap/kill/disable` and the real binary name `omo-gateway`
- Location: src/cron/guard.rs:10 (and :19)
- Trigger: A Hermes-imported job, or a script body or `ack_command`, contains `launchctl bootout gui/501/ai.omon.gateway`. Another example is `pkill -f omo-gateway` or `killall omo-gateway`. The Cargo binary is named `omo-gateway` (Cargo.toml `[[bin]] name = "omo-gateway"`).
- Impact: The self-stop and self-restart guard is bypassed. A cron script can take the gateway down and nothing restarts it. `check_gateway_lifecycle` is the only check on these paths (executor.rs:451, :944, :348, :609).
- Evidence: Line 10 is `launchctl\s+(?:kickstart|unload|load|stop|restart)\b[^\n]*\b(?:omon|hermes)`, with no `bootout|bootstrap|kill|disable|remove`. Line 19 is `p?kill(?:all)?\b[^\n]*\b(?:omon-gateway|hermes-gateway)\b`. `omo-gateway` contains neither `omon` nor `hermes`, so no kill pattern matches it.
- Fix: Add `bootout|bootstrap|kill|disable|remove` to the launchctl verb list. Add `omo-gateway` to the kill patterns, for example `(?:omon?-gateway|hermes-gateway)`.

### [P2] Publishing owner-approval gate matches bare job ids only, so Hermes-mirrored publish jobs bypass it
- Location: src/cron/remediate.rs:215
- Trigger: A publish job is imported or mirrored from Hermes. Its stored id is `hermes:{profile}:{id}` (store.rs:1085 and src/migrate/cron_cutover.rs:354), for example `hermes:default:omon-playstore-daily-publish`. Its payload has no `external_effect: true`.
- Impact: `publishing()` returns false. A failing publish job is then auto-rerun by `transient_retry_once` (remediate.rs:326). The lane `Done` command also skips the owner-approval variant (remediate.rs:603). The `run-now` block in report.rs:336 is skipped as well. This causes an unapproved external publish.
- Evidence: `PUBLISHING_JOBS.contains(&id) || payload.get("external_effect")...` compares the full id against bare names. I could not confirm whether the production publish jobs are native, with bare ids, or mirrored.
- Fix: Compare the id's final segment, for example `id.rsplit(':').next()`, against `PUBLISHING_JOBS`.

### [P2] Silent agent run no longer runs `ack_command`, so a producer-ack job re-delivers the same item forever
- Location: src/cron/executor.rs:658
- Trigger: A job has an `ack_command` (the producer-ack pattern). The agent replies `[SILENT]` because there is nothing to report.
- Impact: The silence path in omo_backend.rs returns early without setting `cron_agent_output`, so `result` is `None` (executor.rs:356-361). The ack helper then never runs the ack. The previous behaviour acked on silence through `cron_ack_command`, which this change removed. The producer never marks the item consumed and re-queues it every run.
- Evidence: `if !scheduler_delivers && result.is_some() { ...run_ack_logged(ack_command)` skips the ack when `result` is `None`.
- Fix: Treat a successful `backend.run` with no output as "not delivered" and run the ack. Drop the `result.is_some()` condition, or set `cron_agent_output` to the silence token.

### [P2] Redaction of delivered owner tokens silently skips rows because of `UPDATE OR IGNORE` and `UNIQUE(problem_id,message)`
- Location: src/cron/report.rs:235
- Trigger: One problem escalates to `needs_owner` twice with the same action text, for example after a budget-exhausted approval. The two outcome rows differ only in the token, so the insert's UNIQUE key does not collide. After the digest is delivered, the redaction rewrites both rows to the same redacted text.
- Impact: The first row is redacted. The second row violates `UNIQUE(problem_id,message)` and `OR IGNORE` skips it on this and every later ack. A live plaintext `--owner-token` stays in `cron_remediation_outcomes`, which a same-user agent can read. That defeats the token-gating fix.
- Evidence: `UPDATE OR IGNORE cron_remediation_outcomes SET message = substr(...) || '<redacted>' WHERE delivered_at IS NOT NULL ...`.
- Fix: Redact per row without the UNIQUE collision. For example, put the outcome id into the redacted text (`'<redacted:' || id || '>'`), or delete the token-bearing rows once delivered.

### [P2] `cron health` forgets healthy jobs whose last run is older than the retention window
- Location: src/cron/report.rs:86 (with src/cron/store.rs:135-137)
- Trigger: A monthly or quarterly job is healthy, but its last run is older than the 14-day default retention (store.rs:12). `prune_terminal_cron_runs` deletes that `cron_runs` row.
- Impact: `last_status` comes only from `cron_runs`, so it is `None` and the job is never counted in `ok_jobs` (report.rs:103). The summary lists it as non-OK ("never run" or a stale payload status), and `last_success_kst` is "none". This is false alarming in the digest and in `cron health`.
- Evidence: Line 86 is `"SELECT status FROM cron_runs WHERE job_id=? ORDER BY started_at DESC LIMIT 1"`. Lines 135-137 are `DELETE FROM cron_runs WHERE status IN ('succeeded','failed') AND COALESCE(completed_at, started_at) < ?`.
- Fix: Keep the latest terminal run per job when pruning, for example by adding `AND run_id NOT IN (SELECT run_id of the newest run per job)`. Alternatively, fall back to the payload `last_status` before counting a job as OK.

### [P2] One unreadable or vanished entry aborts the whole skill search, so an existing skill is reported "not found and skipped"
- Location: src/cron/executor.rs:883 (and :885)
- Trigger: The skills tree has a directory that is unreadable or removed mid-walk (a permission error, a racing editor, a broken mount). It is visited before the target skill is found.
- Impact: `find_skill_file` returns `None` for the whole search. The job runs with "Skill(s) not found and skipped". Because that text is only a warning, the job runs without its skill.
- Evidence: `for entry in std::fs::read_dir(directory).ok()?.flatten()` and `let metadata = std::fs::symlink_metadata(&path).ok()?;`. `?` inside the loop returns from the function.
- Fix: Use `let Ok(entries) = read_dir(directory) else { continue };` and `let Ok(metadata) = ... else { continue };`.

### [P2] A cron script that leaves a background child holding its pipes blocks the run until the full script timeout
- Location: src/cron/executor.rs:489 (and :1013)
- Trigger: A script starts a daemonized or background process without redirecting stdout/stderr, for example `nohup server &`. The script itself exits.
- Impact: `child.wait_with_output()` waits for pipe EOF, which the background grandchild keeps open. The run hangs for the script timeout (1800s by default) and is then marked failed, even though the script succeeded. The job slot and its next run stay blocked. The terminal tool bounds this with `PIPE_DRAIN_GRACE`, but cron does not.
- Evidence: `tokio::time::timeout(timeout, child.wait_with_output()).await`.
- Fix: Wait for the child exit status first. Then drain the pipes with a short bounded grace, and kill the process group afterwards.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/cron/guard.rs - launchctl and pkill pattern gaps (lines 10, 19)
- /Users/indo/code/project/omon-gateway/src/cron/remediate.rs - publishing gate on bare ids (line 215)
- /Users/indo/code/project/omon-gateway/src/cron/executor.rs - silent-run ack skipped (658), skill search abort (883, 885), pipe-drain hang (489, 1013)
- /Users/indo/code/project/omon-gateway/src/cron/report.rs - token redaction skipped (235), health vs prune (86)
- /Users/indo/code/project/omon-gateway/src/cron/store.rs - retention prune that drops last-run state (135-137)
</files>
<answer>
Seven P2 findings, none P0 or P1, none repeating an item recorded in the CONFIRMED files. Every citation was re-opened and the quoted code confirmed on its line. The remediate.rs:215 finding is uncertain: I could not confirm whether the production publish jobs use bare or `hermes:`-prefixed ids. The report above is the deliverable.
</answer>
<next_steps>
Nothing is blocking. The owner may want to settle the id question for finding 2 before deciding whether to fix it.
</next_steps>
</results>
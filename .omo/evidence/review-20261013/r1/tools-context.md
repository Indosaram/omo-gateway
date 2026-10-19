# tools-context review

### [P1] Cron scripts and `ack_command` bypass the terminal approval policy and user deny globs
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:68 (`check_cron_script`), :526-620 (`add`: `check_cron_script` calls for `script` and `ack_command`), and /Users/indo/code/project/omon-gateway/src/tools/mod.rs:45 (default `requires_approval` returns `None`; `CronTool` does not override it)
- Trigger: The operator runs with `APPROVAL_MODE=always`, or sets `approvals_deny` such as `kubectl delete *`. These are enforced by `TerminalTool` at terminal.rs:213 and :248-262. A model turn, for example one steered by prompt injection, then calls `cron add` with `delay: "1m"` and `script: "kubectl delete pod x"` or any command that is not hardline or "dangerous".
- Impact: The command runs unattended through `sh -c` (cron/scheduler.rs:451, cron/executor.rs:427). It gets no interactive prompt, and the deny-glob and external-scanner (Tirith) checks never run. The `CronTool` doc comment only justifies refusing hardline and dangerous commands. Under `Always` or deny-glob configuration, the operator's policy is silently void for anything routed through cron.
- Fix: Give `CronTool` the same policy as `TerminalTool` (deny globs, approval policy, requester). Alternatively, override `Tool::requires_approval` to return a reason whenever `add` or `update` carries a `script` or `ack_command` and the policy is not `Never`. Apply `match_user_deny_rule` inside `check_cron_script`.

### [P1] `message_context search` in a parent channel returns messages from threads under it, including private threads
- Location: /Users/indo/code/project/omon-gateway/src/tools/message_context.rs:914-990 (`search`), together with /Users/indo/code/project/omon-gateway/src/storage/message_search.rs:190-233 (`index_inbound_with_metadata`) and :143-170 (`search` SQL)
- Trigger: The bot participates in a private (or otherwise restricted) thread under `#general`. Ingress indexing stores each thread message with `channel_id = session.channel_id`, which is the parent channel, plus `thread_id`. A user in `#general` who cannot see that thread runs `message_context search` with the default target. `authorize_target` allows it because the target is the current channel. The FTS query filters only on `platform` and `channel_id`, never on `thread_id`.
- Impact: Content and author data from the private thread are returned to the parent-channel session, which breaks per-conversation isolation. The reverse mismatch also occurs. REST-indexed thread messages are stored under `channel_id = thread id`. A thread session searching its own thread therefore cannot find ingress-indexed history older than the REST `scan_limit`, and duplicate documents are possible.
- Fix: Index ingress thread messages with `channel_id` set to the thread id, matching the REST path. Alternatively, add `AND d.thread_id IS NULL` when the target is a non-thread channel, and match on `thread_id` when the target is a thread.

### [P2] `cron add enabled=false` can run once through the scheduler before it is paused
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:636-646
- Trigger: `add` with `enabled: false` and `once_at` up to 120 seconds in the past, which `next_run` accepts. `register_with_id` (scheduler.rs:850-890) inserts with `enabled = 1`, a due `next_run_at`, and calls `wake.notify_one()`. The scheduler's `run_due_jobs` can claim the job before the separate `scheduler.pause(id)` call lands.
- Impact: A job the caller asked to create disabled executes once.
- Fix: Insert the row with the requested `enabled` value in a single statement, for example by adding an `enabled` field to `CronJobSpec`.

### [P2] `cron update` does a read-modify-write with no compare guard, and skips expression validation when disabled
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:418-520
- Trigger 1: The job row is read at about line 421 and written back unconditionally at line 504. If the scheduler claims and completes the job in between, it advances `next_run_at` and writes `last_status` and run counters into `payload_json`. The tool then restores the stale `next_run_at` and payload, so the job re-runs and loses its run state. The scheduler's own writes guard with `payload_json = ?`; this tool does not.
- Trigger 2: `update` with `expression: "garbage", enabled: false` stores the invalid expression, because `next_run` is only evaluated when `enabled` is true (:490-495). Resume later fails.
- Fix: Add `AND payload_json = ?` (and the expression) to the `UPDATE` and retry on zero rows. Always validate a changed expression.

### [P2] The "already exists" check in `cron add` is a check-then-act race
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:526-540 and :636-640 (`register_with_id` upsert)
- Trigger: Two concurrent `add` calls with the same id both pass the `SELECT`. The second `ON CONFLICT DO UPDATE` silently overwrites the first, and also races with an authority change.
- Fix: Use a plain `INSERT` and map the constraint error to "already exists".

### [P2] Ingress-indexed documents are returned from search without neutralization
- Location: /Users/indo/code/project/omon-gateway/src/storage/message_search.rs:190-233 and /Users/indo/code/project/omon-gateway/src/tools/message_context.rs:1139-1170 (`message_from_search_document`)
- Trigger: A message is indexed from ingress, where `content` is raw and `author_name = session.user_id`, which is empty for guild sessions. It later falls outside the REST rescan window that would overwrite it with sanitized content.
- Impact: Search results hand the model raw `<|...|>`, backticks and ANSI sequences. The REST path applies `neutralize_untrusted_inline_text`; this path does not.
- Fix: Apply `neutralize_untrusted_inline_text` in `message_from_search_document`.

### [P2] One bad Discord token makes `message_context` unusable for every bot
- Location: /Users/indo/code/project/omon-gateway/src/tools/message_context.rs:483-515 (`clients`)
- Trigger: `DISCORD_BOT_TOKENS` contains a revoked or invalid secondary token. `get_current_user` fails inside `get_or_try_init`, and every call returns that error.
- Fix: Skip tokens that fail validation, with a warning, and fail only when no token resolves.

### [P2] The lazy provider opens a second SQLite pool and re-runs migrations at first tool call
- Location: /Users/indo/code/project/omon-gateway/src/tools/message_context_lazy.rs:32-40, with `Database::connect` at storage/db.rs:158-190
- Trigger: The first `message_context` call. A second single-connection pool is created and `MIGRATOR.run` plus `upgrade_legacy_guild_session_keys` run while the main pool is writing. Under write load the call can fail after the 5s busy timeout. If `DATABASE_URL` is `sqlite::memory:`, the policy overrides set on the main pool are invisible to this one.
- Fix: Inject the application's existing pool into the provider instead of calling `init_pool`.

### [P2] `search` runs the FTS query twice and discards the first result
- Location: /Users/indo/code/project/omon-gateway/src/tools/message_context.rs:924-933 (`let _ = self.search_index.search(...)`)
- Impact: A wasted query that serves only to validate the query string. If that is the intent, validate with `build_fts_query` instead.

### [P2] `cron runs` exposes lease `claim_token` to the model, and `notepad` has no job or profile authorization
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:687-720 (`claim_token` in `runs`) and :812-870 (`notepad`)
- Impact: The `runs` output includes the lease capability token, which the model does not need. `notepad` accepts arbitrary `profile` and `id` without checking that the job exists or belongs to the caller.
- Fix: Drop `claim_token` from the `runs` output. Validate that the job exists for `notepad`.

### [P2] Cron `deliver` accepts any Discord channel the bot can reach, with no policy check
- Location: /Users/indo/code/project/omon-gateway/src/tools/cron.rs:190-230 (defaults only when `deliver` is absent or `local`) and :526-600 (`deliver` and `origin` taken from args)
- Impact: A model, for example one steered by prompt injection, can schedule output to a channel in another guild. `message_context` enforces an allowlist and a same-workspace rule; cron delivery does not.
- Fix: Restrict `deliver` and `origin` to the current session's destination, or apply the same allowlist.

### [P2] A written skill can be shadowed by an older nested skill of the same name
- Location: /Users/indo/code/project/omon-gateway/src/tools/skills.rs:82-94 (`find_all_skills`, `sort_by` plus `dedup_by`)
- Trigger: `write name=foo` creates `<root>/foo/SKILL.md` while `<root>/category/foo/SKILL.md` exists. Which one `read` returns depends on `read_dir` order, so the write can report "saved" while `read` returns the old content.
- Fix: Reject the write on a name collision, or prefer the shallowest path deterministically.

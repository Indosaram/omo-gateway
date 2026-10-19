# Lane dashboard

### [P1] DELETE /api/sessions/{id} silently cascade-deletes the cron jobs bound to that session, and their run history
- Location: src/dashboard.rs:1216
- Trigger: In the dashboard, delete a session that a cron job is bound to. The cron tool binds every job it creates to the creating session (`args["_session_key"] = session.storage_key()`, src/tools/cron.rs:312). The inbox digest job binds to the owner's home DM session (src/inbox/digest.rs:44). Deleting that Discord session from the sessions page, or `DELETE /api/sessions/<key>`, removes those jobs.
- Impact: `cron_jobs.session_key` is `FOREIGN KEY ... REFERENCES sessions(session_key) ON DELETE CASCADE` (migrations/0001_initial.sql:48). `cron_runs` cascades from `cron_jobs` (migrations/0004_cron_leases.sql:11). The pool opens with `foreign_keys(true)` (src/storage/db.rs:165). So the bare `DELETE` also destroys the scheduled jobs and their run history. The confirmation text only promises to remove "the conversation and all of its messages" (web/src/i18n/en.ts:156). A deleted digest job is gone until the next gateway boot re-registers it.
- Evidence: `sqlx::query("DELETE FROM sessions WHERE session_key = ?")`. The handler does nothing about dependent `cron_jobs` rows. src/inbox/digest.rs:56 inserts the session row before `register_with_id`, which shows the FK is enforced.
- Fix: Before the delete, either reject with 409 when `SELECT 1 FROM cron_jobs WHERE session_key = ?` returns a row, or run `UPDATE cron_jobs SET session_key = NULL WHERE session_key = ?` in the same transaction as the delete. Mention the jobs in the confirmation text.

### [P2] Cron create/update accept any `session_key`; a non-existent or empty one on create hits the FK and returns 500
- Location: src/dashboard.rs:1654 (create); src/dashboard.rs:1721 (update)
- Trigger: `POST /api/cron/jobs {"expression":"...","payload":{...},"session_key":""}` or a `session_key` that has no row in `sessions`. `PUT` with a non-existent key does the same.
- Impact: `cron_jobs.session_key` is an FK to `sessions`. The INSERT or upsert fails with `FOREIGN KEY constraint failed`. `OmonError::Database` maps to HTTP 500 (the `From<OmonError>` impl), instead of a 400 for bad input. Update normalizes an empty key to `None`, but create does not, so the two endpoints disagree.
- Evidence: `session_key: input.session_key,` (create, no trim or existence check). `Some(key) => Some(key),` (update, no existence check).
- Fix: In both handlers, treat an empty key as `None`. Check `fetch_session_row` for non-empty keys and return 400 `session not found` if it is absent.

### [P2] Skill description shows the YAML `name:` line for skills that have frontmatter
- Location: src/dashboard.rs:2010
- Trigger: Open `/api/skills` or the Skills page with any `SKILL.md` that starts with a `---` frontmatter block (`---\nname: foo\ndescription: ...\n---`).
- Impact: The extractor skips only blank lines, lines starting with `#`, and lines starting with `---`. It returns the first remaining line, which is `name: foo`, so the UI shows that as the description. The real `description:` field is never read.
- Evidence: `.find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("---"))`
- Fix: If the head starts with `---`, parse the block up to the closing `---` and use its `description:` value. Otherwise fall back to the current heuristic.

### [P2] An empty `DASHBOARD_PORT=` silently enables the dashboard
- Location: src/dashboard.rs:78
- Trigger: `DASHBOARD_PORT` is set but empty, for example a compose or env template `DASHBOARD_PORT=${DASHBOARD_PORT:-}`, with `DASHBOARD_ENABLED` unset or false.
- Impact: `port_raw.is_some()` is true for an empty string, so `enabled` is true. The port then fails to parse and falls back to 9119 with only a warning. An unauthenticated control surface (chat, approval resolve, cron and session delete) starts on loopback even though the operator configured nothing.
- Evidence: `let enabled = env_bool("DASHBOARD_ENABLED", false) || port_raw.is_some();`
- Fix: Treat a blank or whitespace-only value as unset: `env::var("DASHBOARD_PORT").ok().filter(|v| !v.trim().is_empty())`.

### [P2] A bracketed IPv6 `DASHBOARD_HOST` passes `validate()` but cannot be bound
- Location: src/dashboard.rs:144 (validation); src/dashboard.rs:719 (bind)
- Trigger: `DASHBOARD_HOST=[::1]`, or `--host "[::1]"`.
- Impact: `is_loopback_host` strips the brackets before `IpAddr::parse`, so validation succeeds. `spawn_server` then calls `TcpListener::bind(("[::1]", port))` with the raw string. tokio treats it as a hostname, the lookup fails, and startup errors with "failed to bind dashboard". In combined mode this aborts the gateway start.
- Evidence: `host.trim_matches(|c| c == '[' || c == ']')` in validation, versus `TcpListener::bind((settings.host.as_str(), settings.port))` using the unstripped host.
- Fix: Normalize `host` once, stripping the brackets in `from_env` and `from_args`, so validation and bind use the same value.
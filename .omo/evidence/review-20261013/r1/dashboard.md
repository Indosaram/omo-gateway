# dashboard review
### [P1] PUT /api/cron/jobs/{id} wipes the job payload and session_key when those fields are omitted
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1598 (`update_cron_job`); input struct at :1529, `payload` field at :1533
- Trigger: `PUT /api/cron/jobs/<id>` with a body like `{"expression":"0 * * * *"}`, for example a schedule-only edit. `CronJobInput.payload` is `#[serde(default)]`, so an omitted payload becomes `Value::Null`. `session_key` becomes `None`. `update_cron_job` passes both to `scheduler.register_with_id`. That function (`/Users/indo/code/project/omon-gateway/src/cron/scheduler.rs:850`) upserts `payload_json = excluded.payload_json` and `session_key = excluded.session_key`. The dashboard then runs a second `UPDATE cron_jobs SET session_key = ?` with the same `None`.
- Impact: The job's prompt, script and delivery config are overwritten with `null`, and its session binding is erased. A full-payload PUT also overwrites scheduler-maintained payload fields (`last_status`, repeat counters read by `claim_job`). So any edit resets "repeat N times" progress. This is the same omitted-field-erases-config pattern the bot update path already fixed (comments at `update_bot`, :2025).
- Fix: Make `payload` and `session_key` `Option` in the update input. Load the existing job and merge, keeping the stored value when a field is omitted. Preserve scheduler-owned payload keys when the caller supplies a new payload. Drop the redundant second `UPDATE`.

### [P1] /api/readiness never fails on a dead agent backend, and reads the wrong env var
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:872-880 (`api_readiness`)
- Trigger: The backend address comes from `state.config["appserver_url"]`, then `OMON_APPSERVER_URL`. Neither is ever set. Neither config view (`config_view_from_gateway` at :659, `dashboard_config_view` in dashboard_runtime.rs) emits an `appserver_url` key. The real setting is `OMON_OMO_APPSERVER_URL` (`/Users/indo/code/project/omon-gateway/src/agent/omo_config.rs:224`), and I found no other reader of `OMON_APPSERVER_URL`. So `appserver_url` is `None`, and `ready = db_ok && workspace_ok && disk_ok && (backend_ok || appserver_url.is_none())` ignores the backend result. Trigger it by stopping the omo app-server, or by letting the supervisor's restart circuit open.
- Impact: `/api/readiness` and `/api/ready` return 200 "ready" while the agent backend is down, and `chat_runtime` is true whenever a multiplexer exists. A load balancer or operator health check gets a false positive. With a remote `OMON_OMO_APPSERVER_URL`, the probe also hits the default `127.0.0.1:19742` instead of the configured host.
- Fix: Resolve the URL from the same source as `OmoBackendConfig::from_env` (`OMON_OMO_APPSERVER_URL`, default `ws://127.0.0.1:19742`), or put the configured URL into the config view. Require `backend_ok` for readiness whenever a chat runtime is configured.

### [P2] /api/cron/runs exposes lease `claim_token`
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1695 (`CronRunRow`), :1715 (`list_cron_runs`)
- Trigger: `GET /api/cron/runs` serializes every row, including the `claim_token` fencing secret for running leases.
- Impact: It is an unnecessary secret leak. Anything that can read the dashboard (log scrapers, XSS in the SPA) gets the fencing tokens for in-flight runs.
- Fix: Remove `claim_token` from the SELECT and the row struct, or mark it `#[serde(skip)]`.

### [P2] Cron create and update are non-atomic multi-step writes
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1549 (`create_cron_job`), :1598 (`update_cron_job`)
- Trigger:
  - (a) Two concurrent `POST` requests with the same `id` both pass the `get()` check, and the second silently upserts over the first because `register_with_id` uses `ON CONFLICT DO UPDATE`.
  - (b) `POST` with `enabled:false` registers the job enabled (with `wake.notify_one()`), then pauses it in a separate statement. A failed pause leaves an enabled job after an error response.
  - (c) `PUT` on a paused job: `register_with_id` sets `next_run_at` to a non-NULL value while `enabled=0`, and only a later pause call clears it.
  - (d) An empty `id` (`""`) is accepted.
- Impact: Lost updates, a brief window where a job meant to be disabled is eligible to run, and an inconsistent paused-job state.
- Fix: Do the insert-if-absent, the enabled flag and the next_run_at handling in one SQL statement or transaction. Reject empty ids.

### [P2] One corrupt cron payload breaks the whole job list
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1540 (`list_cron_jobs`, `collect::<Result<Vec<_>,_>>()?`), `cron_job_view` (just above :1695)
- Trigger: A single `cron_jobs` row with invalid `payload_json`. `CronJob::payload()` returns an error, so `GET /api/cron/jobs` returns 500 for all jobs.
- Impact: The UI cron page cannot list jobs, so the bad job cannot be found to fix or delete.
- Fix: Degrade per row. Emit the job with `payload: null` and an `error` field instead of failing the request.

### [P2] trigger_cron_job returns 409 for a missing job, and runs side effects in a detached dashboard
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1645
- Trigger:
  - (a) `POST .../trigger` on a nonexistent id returns 409 "does not exist or is already running" instead of 404.
  - (b) In standalone or detached mode, the scheduler's dispatcher is the web-only dispatcher. Triggering a job with a Discord delivery destination runs the agent (side effects) and then fails delivery. `reject_detached_messenger_session` is not applied here, unlike chat and stop.
- Impact: Misleading status code. In detached mode the run is recorded as a delivery failure, and the operator-visible output never reaches Discord.
- Fix: Return 404 when the job is absent. Refuse or warn on triggering Discord-delivery jobs when `detached_from_gateway` is true.

### [P2] Standalone dashboard handles Ctrl+C only, not SIGTERM
- Location: /Users/indo/code/project/omon-gateway/src/dashboard_runtime.rs:20-30 (`run_standalone_cli`)
- Trigger: `systemctl stop` or `docker stop` sends SIGTERM. Only `tokio::signal::ctrl_c()` is wired up. The gateway path handles SIGTERM (main.rs:1229).
- Impact: The process dies without running `scheduler.shutdown`, `pool.close` or the `OmoDaemonSupervisor` `Drop`. A daemon it spawned stays running as an orphan and is later adopted as an "external" daemon. Running cron leases stay held until they expire. A standalone dashboard's in-flight turns are also dropped, because `mux` is never shut down in the normal path either (:195-220).
- Fix: Also listen for `SignalKind::terminate()` on unix and cancel the shutdown token. Shut the multiplexer down before `pool.close()`.

### [P2] Log websocket loses entries between snapshot and subscribe
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:2282 (`handle_logs_socket`)
- Trigger: It sends up to 100 snapshot entries (each `send().await`) and only then calls `logs.subscribe()`. Entries logged during those sends are never delivered to that client. In `DashboardLogStore::push`, `fetch_add` and the broadcast happen outside the lock, so ids can also reach `entries` out of order.
- Impact: Gaps in the live log view on a busy server or a slow client.
- Fix: Call `subscribe()` before taking the snapshot. De-duplicate by `id` on the client or server.

### [P2] ExpireApproval never reaches websocket clients
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:570 (`action_session` returns None for `ExpireApproval`), :1303 (`handle_session_socket` filter)
- Trigger: The session websocket forwards only events whose `action_session` matches the session. `ExpireApproval` carries no session, so it is dropped.
- Impact: A websocket client never learns that an approval was resolved or timed out. It stays stale until it re-polls `/api/approvals/pending`.
- Fix: Keep `request_id` to session mapping in the dispatcher's `pending` map and forward expiries to that session's websocket. Or broadcast expiry to all subscribers.

### [P2] update_bot is a non-transactional read-modify-write
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:2025
- Trigger: Two concurrent `PUT /api/bots/{id}` calls changing different fields. Each reads the row, merges, and writes all columns, so the later write overwrites the earlier one's field.
- Impact: Lost update.
- Fix: Use one `UPDATE ... SET col = COALESCE(?, col)` style statement, or a transaction.

### [P2] config_view_from_environment is dead code and reports the wrong approval setting
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:704-718
- Trigger: It reads `APPROVAL_POLICY` and defaults to "ask". The real knob is `APPROVAL_MODE`, default smart. I found no callers.
- Impact: No runtime effect today. It would misreport policy if wired up.
- Fix: Delete it, or align it with `dashboard_config_view`.

### [P2] Hardening notes
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:66-110 (`DashboardSettings`), :1795 (`list_skills`), :1046 (`list_sessions`)
- Trigger and impact:
  - `DASHBOARD_PORT` that fails to parse silently falls back to 9119.
  - The `--insecure` flag is parsed and shown in the config view, but `validate()` rejects every non-loopback host regardless, so the flag does nothing.
  - `list_skills` reads whole SKILL.md files, and `.unwrap_or_default()` hides spawn_blocking failures.
  - The search `LIKE` patterns do not escape `%` and `_`.
- Fix: Warn on an invalid port. Either remove `--insecure` or honor it. Read only the first lines of skill files. Escape LIKE wildcards.

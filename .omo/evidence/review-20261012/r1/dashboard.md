# dashboard review

### [P1] Dashboard cron scheduler marks Discord deliveries as delivered without sending them
- Location: src/dashboard_runtime.rs:171
- Trigger: The dashboard builds its `CronScheduler::with_dispatcher` with `dispatcher` = `Arc::new(WebDashboardDispatcher)` (line 63). Two paths use it:
  - In standalone mode (`omon dashboard`/`serve`, `src/entry.rs:119`, `start_scheduler = true`), every due job runs on this scheduler.
  - In combined mode (`run` with `DASHBOARD_ENABLED`/`DASHBOARD_PORT`, `src/entry.rs:201`), `POST /api/cron/jobs/{id}/trigger` (`trigger_cron_job`, src/dashboard.rs:1588) runs the job through this scheduler.
  
  `CronScheduler::deliver` (`src/cron/scheduler.rs:1987-2001`) then calls `WebDashboardDispatcher::dispatch`. That function only broadcasts to dashboard websockets and always returns `Ok(())` (src/dashboard.rs:492-493). The scheduler then calls `mark_obligation_delivered`.
- Impact: Cron output meant for a Discord channel never reaches Discord, yet the delivery ledger records it as delivered. Nothing retries it and no failure notification is sent. Because a dispatcher is present, the `dispatcher.is_none()` guard for explicit Discord destinations (`scheduler.rs:1790`) is bypassed, so this loss happens silently.
- Fix: Pass the scheduler a dispatcher that fails (or skips the run) for non-`web` sessions. Alternatively, in combined mode, have the trigger route call the gateway's real scheduler/egress (for example, give `run_standalone` the gateway's dispatcher) instead of a web-only dispatcher.

### [P1] Combined-mode dashboard approval endpoints use a separate, empty approval guard
- Location: src/dashboard_runtime.rs:56
- Trigger: In `run` with the dashboard enabled (`src/entry.rs:201`), the dashboard creates `SmartApprovalGuard::new().with_pool(pool.clone())`. The gateway creates its own guard at `src/main.rs:772`. A Discord tool call that needs approval registers a pending entry only in the gateway's guard.
- Impact: `GET /api/approvals/pending` (src/dashboard.rs:2105) always shows zero items for real Discord approvals. `POST /api/approvals/{id}/resolve` returns 404 for them, because `resolve_custom_id` only checks the dashboard's guard. The operator cannot see or resolve gateway approvals from the dashboard, and gets no error explaining why.
- Fix: In combined mode, have `run_standalone` take the gateway's `SmartApprovalGuard` and its approval event source instead of creating new ones.

### [P1] Combined-mode dashboard chat runs a second multiplexer whose replies never reach Discord
- Location: src/dashboard_runtime.rs:131
- Trigger: In `run` with the dashboard enabled, call `POST /api/sessions/{id}/chat` or use the session websocket (src/dashboard.rs `post_chat` / `handle_session_socket`) with the storage key of an existing Discord session. `resolve_session_key` parses it as a Discord `SessionKey`, and the event is routed into the dashboard's own `SessionMultiplexer::with_dispatcher(..., Some(dispatcher /* WebDashboardDispatcher */))`.
- Impact: The agent turn runs in a second multiplexer at the same time as the gateway's multiplexer for the same session row and backend thread. Its reply (`SendMessage` for a `discord` session) goes only to the web broadcast, so the Discord channel never receives it. The two concurrent multiplexers can also interleave turns and session state on the same Discord session.
- Fix: In combined mode, reuse the gateway's multiplexer, or reject chat/stop requests for non-`web` sessions in the dashboard runtime.

### [P2] POST /api/cron/jobs with an existing id silently overwrites that job
- Location: src/dashboard.rs:1510
- Trigger: `POST /api/cron/jobs` with a body whose `"id"` matches an existing job. `create_cron_job` calls `scheduler.register_with_id`, which runs `INSERT ... ON CONFLICT(id) DO UPDATE` (`src/cron/scheduler.rs:861-869`).
- Impact: The existing job's expression, payload and session_key are replaced, and the handler still returns `201 Created`. A create request therefore destroys an existing job's definition with no conflict error. It also keeps the old `enabled` flag, so a paused job gets a new `next_run_at` while staying disabled.
- Fix: In `create_cron_job`, return `409 Conflict` when `state.scheduler.get(&id)` already returns a job before calling `register_with_id`, or use a plain-INSERT variant for creation.

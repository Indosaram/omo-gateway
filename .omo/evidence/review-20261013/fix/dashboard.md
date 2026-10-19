# fix: dashboard node (src/dashboard.rs, src/dashboard_runtime.rs, web/src/**)

No compile/test command was run (lead gate pending). Tests are written, not executed.

## dashboard lane

- [P1] PUT /api/cron/jobs/{id} wipes payload/session_key -> FIXED. dashboard.rs: new `CronJobUpdateInput` (all fields Option) and `update_cron_job` merges with the stored job (expression, payload, session_key kept when omitted; empty session_key clears), `preserve_scheduler_state` keeps scheduler-owned payload keys (last_*) and repeat.completed/completed on a new payload; redundant second UPDATE removed; a paused job stays paused (re-pause after register_with_id so next_run_at is NULL). Tests: fix_r5_cron_put_without_payload_keeps_payload_and_session_key, fix_r5_cron_put_on_paused_job_keeps_it_paused_without_next_run.
- [P1] /api/readiness wrong env var / ignores backend -> FIXED. dashboard.rs: `readiness_backend_url` (config.appserver_url, then OMON_OMO_APPSERVER_URL, then ws://127.0.0.1:19742); probe always runs against it; ready requires backend_ok whenever a chat runtime (multiplexer) exists. Tests: fix_r5_readiness_backend_url_prefers_config_then_env_then_default, fix_r5_readiness_fails_when_chat_backend_is_down (dead loopback port, 503 with chat, 200 without).
- [P2] /api/cron/runs exposes claim_token -> FIXED. dashboard.rs: removed from CronRunRow and all 4 SELECTs. Test: fix_r5_cron_runs_do_not_expose_claim_token.
- [P2] Cron create/update non-atomic -> PARTIAL. Fixed: (c) paused-job PUT leaves next_run_at set (above), (d) empty id rejected with 400 (test fix_r5_cron_create_rejects_empty_id). SKIPPED (a)(b): a true insert-if-absent / create-disabled in one statement needs a new CronScheduler API (src/cron/scheduler.rs, outside scope). BLOCKED-SCOPE: add `CronScheduler::register_new(id, spec, enabled)` doing INSERT ... ON CONFLICT DO NOTHING with enabled and next_run_at set in one statement (return None on conflict); then create_cron_job calls it.
- [P2] One corrupt payload breaks the job list -> FIXED. dashboard.rs: `cron_job_view` is infallible, emits payload null + `error` string. Test: fix_r5_cron_list_degrades_per_row_on_corrupt_payload.
- [P2] trigger_cron_job 409 for missing job / detached Discord delivery -> FIXED. 404 for a missing job; in detached mode a job with a messenger delivery destination (omon_gateway::delivery_destinations) is refused with 409 before any run starts. Test: fix_r5_trigger_missing_job_is_404_and_detached_messenger_job_is_refused.
- [P2] Standalone dashboard handles Ctrl+C only -> FIXED. dashboard_runtime.rs: `wait_for_shutdown_signal` (Ctrl+C or unix SIGTERM); run_standalone now calls mux.shutdown() before pool.close(). No unit test (process signal; not deterministically testable in-module).
- [P2] Log websocket loses entries between snapshot and subscribe -> FIXED. dashboard.rs: subscribe before snapshot, skip ids <= last sent id; `DashboardLogStore::push` now assigns id, stores and broadcasts under one lock so ids are monotonic. Test: fix_r5_log_ids_are_strictly_increasing_in_snapshot_and_broadcast.
- [P2] ExpireApproval never reaches websocket clients -> FIXED. dashboard.rs: `session_event_matches` tracks request ids of approvals seen (and pending at connect) for that session socket and forwards the matching ExpireApproval. Test: fix_r5_session_socket_forwards_expiry_of_its_own_approvals_only.
- [P2] update_bot read-modify-write -> FIXED. dashboard.rs: single UPDATE with COALESCE/CASE flags (omitted keeps column, empty clears; 404 via rows_affected). Covered by existing omitted-field retention test (fix for update_bot comments) in the module; no new test (same observable contract).
- [P2] config_view_from_environment dead + wrong approval knob -> FIXED by deletion (no callers).
- [P2] Hardening notes:
  - invalid DASHBOARD_PORT -> FIXED (tracing::warn, still falls back to default); no test (logging only).
  - LIKE wildcards unescaped -> FIXED (`like_contains_pattern` + ESCAPE '\\' on all 6 LIKE queries). Tests: fix_r5_like_pattern_escapes_wildcards, fix_r5_session_search_matches_wildcards_literally.
  - list_skills reads whole files / hides spawn_blocking failure -> FIXED (reads first 8 KiB; join error is a 500). No new test.
  - --insecure is a no-op -> SKIPPED: validate() deliberately refuses every non-loopback bind (no transport auth); removing the flag is a CLI-surface change, left for a follow-up.

## web-deploy lane (web/src item only)

- [P2] web playground auto-loads remote images -> FIXED. web/src/lib/remote-image.ts (`isAutoLoadableImageSrc`: data:/blob:/same-origin only), App.tsx `GuardedImage` used for markdown images, extracted image links (no longer a clickable-through <a>) and the streaming bubble; remote images render a "Load remote image from <host>" button, loaded with referrerPolicy=no-referrer. Test: web/src/lib/remote-image.test.ts. Note: BotsPage avatarUrl still renders a configured URL directly (operator-configured, not chat content) - not changed.
- Other web-deploy P2s (scripts/, migrations, FTS) are outside this node's scope.

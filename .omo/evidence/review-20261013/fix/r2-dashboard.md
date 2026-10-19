# r2 dashboard P2 fixes

No cargo/bun/tsc was run (per instructions); all changes are unverified by the compiler until the lead's gate.

1. Duplicate log lines - FIXED. web/src/lib/log-merge.ts (appendLogs, id dedupe), used for seed, socket entries and the late-seed case in web/src/App.tsx LiveLogsPage. Test: web/src/__tests__/dashboard-fixes.test.ts "log merging".
2. HTTP fallback ends turn on 202 - FIXED. web/src/lib/chat-turn.ts (awaitReply hook; a `queued:true` ack waits), App.tsx polls persisted messages via hasAssistantReplyAfter (1s, <=120 tries); postChat type corrected in web/src/api.ts. Tests: "HTTP fallback queued turn".
3. Mid-turn status invisible - FIXED. classifyChatFrame returns kind 'message' for event.type 'send_message'; App.tsx shows it as its own assistant message (not persisted, placed after the newest user message via mergeStatusMessages); stale header comment updated. Tests: "mid-turn status messages".
4. Stop then send ends new turn early - FIXED. handleStopTurn abandons the old turn socket, keeps input locked (isSending + isStopping) until the stop socket reports stopped/error (isStopSettled), then unlocks and reloads. Tests: "stop settlement".
5. Create-bot empty strings - FIXED. src/dashboard.rs create_bot normalizes blank model/system_prompt/empty toolsets to NULL. Test: fix_r2_create_bot_normalizes_blank_overrides_to_null.
6. Create-cron without id - FIXED. src/dashboard.rs create_cron_job always uses register_new with a generated UUID when id is absent; null/missing payload returns 400. Test: fix_r2_cron_create_without_id_honors_disabled_and_requires_payload.
7. Overview/Approvals fields - FIXED. src/dashboard.rs api_status adds "model" (config["model"]); App.tsx renders approvalSessionLabel(a.session), model falls back to "unknown", disk shows "unknown" with no bar when the sample is null (web/src/lib/status-view.ts). Tests: fix_r2_status_reports_configured_model, "overview and approval views".
8. isAutoLoadableImageSrc backslash - FIXED. web/src/lib/remote-image.ts rejects values containing backslash/tab/CR/LF. Test: web/src/lib/remote-image.test.ts (existing file; it imports from vitest, not bun:test).

Note: the existing web/src/lib/remote-image.test.ts and cron-job tests etc. use vitest imports; only web/src/__tests__/dashboard-fixes.test.ts uses bun:test.

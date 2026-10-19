# fix-tools-context report

RED/GREEN: deferred to single final integration gate (user directive: no further builds/tests).

Evidence that exists from earlier attempts (not re-run):
- `fix-tools-context-red.log`: `cargo test --test test_fix_fix_tools_context` on the unfixed code: 1 passed, 6 failed (thread default target, web internal destination, script hardline, cron thread-delivery, update imported, add overwrite).
- A later foreground run on the fixed code passed 6 of 7. The only failure was `cron_add_from_thread_session_delivers_to_thread`, with `FOREIGN KEY constraint failed` (test setup: `cron_jobs.session_key` references `sessions`). The test now inserts the `sessions` row first. That edit has NOT been run; the final re-run (`fix-tools-context-green.log`) was cut off by the stop directive and holds no result.

## Files changed
- src/tools/cron.rs
- src/tools/message_context.rs
- src/tools/web.rs
- tests/test_fix_fix_tools_context.rs (new)
- src/tools/message_context_lazy.rs: not changed

## Defect -> fix
1. [P1] cron add overwrote existing ids: `add|create` in `CronTool::execute` now calls `check_imported_read_only` and rejects when the id already exists ("already exists; use 'update'"). FIXED.
2. [P1] update skipped the imported-job guard: confirmed. `update` now calls `check_imported_read_only` first. FIXED.
3. [P1] thread delivery `discord:{channel}:{thread}`: confirmed it is not the accepted form. Native payloads parse that string as one chat id. Thread sessions now emit `deliver: "origin"` plus `origin: {platform, chat_id, thread_id}`, and `add` copies `origin` into the payload. scheduler.rs/store.rs untouched. FIXED. The test calls `omon_gateway::delivery_destinations` on the stored payload and expects chat_id 100 / thread_id 200.
4. [P1] cron script ran shell without hardline/dangerous checks: confirmed. New `check_cron_script` (lifecycle check, then `detect_hardline_command`, then `detect_dangerous_command`) is used by `add` and `update`. FIXED.
5. [P1] message_context default target: `authorize_target` defaults to `session.thread_id` when present, and treats the thread as the current conversation. The same-workspace check in `DiscordMessageContextProvider` also treats the thread as current. FIXED.
6. [P1] web_fetch SSRF: no restriction existed. Added `ensure_public_destination` (literal and DNS resolution check on the initial URL), a literal-host check inside `redirect_is_allowed` for every hop, and a `PublicOnlyResolver` DNS resolver on the client so resolved redirect hosts are checked too. FIXED.
7. [P2] `read_bounded_text` underflow: `*remaining = remaining.saturating_sub(1)`. FIXED. The DDG markup item (web.rs:77) was optional and is not done.

## Tests added (written, not run in final form)
tests/test_fix_fix_tools_context.rs:
- message_context_defaults_to_thread_for_thread_sessions
- message_context_defaults_to_channel_without_thread
- cron_add_rejects_existing_and_imported_ids
- cron_update_rejects_imported_jobs
- cron_add_from_thread_session_delivers_to_thread
- cron_script_rejects_hardline_and_dangerous_commands
- web_fetch_rejects_internal_destinations

src/tools/web.rs `#[cfg(test)]`:
- read_bounded_text_invalid_byte_after_budget_does_not_underflow
- redirect_hops_to_internal_addresses_are_blocked

## Notes
- The crate also had compile errors in src/agent/omo_daemon.rs (owned by fix-omo-backend) during an earlier attempt; see `fix-tools-red.log`.
- web_fetch tests use IP literals and `localhost` only (no network). The `localhost` case relies on the literal-name check, not DNS.

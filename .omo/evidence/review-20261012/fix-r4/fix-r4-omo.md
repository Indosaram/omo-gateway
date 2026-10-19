# fix-r4-omo: tests/test_omo_backend.rs (3 pre-existing failures on HEAD a486a7f)

Files touched: `src/agent/omo_backend.rs` (production, one test-relevant behavior change), `tests/test_omo_backend.rs` (3 fixture edits). No other files, no commit, no .env/DB.

## 3) test_omo_backend_e2e_thread_lifecycle_and_streaming - TEST fixture stale
- Root cause: commit ddf2215 ("auto-approve daemon tool requests under default gateway policy") made `src/agent/omo_backend.rs` (~1310-1326) answer every approval request with `approval_allow_response` unless `APPROVAL_MODE`/`APPROVAL_POLICY` is `always`, or the toolset gate (enabled_toolsets without terminal/bash/shell) denies a command. The test (tests/test_omo_backend.rs ~2076-2089) still expects a denial for request id 999 (`execCommandApproval`, command "rm -rf /") from a default session, so it got `allow: true`.
- Fix (test only): the session sets `enabled_toolsets = Some(Vec::new())` so the denial comes from the documented toolset gate. Assertions unchanged (`allow == Some(false)`, decision decline/deny). Auto-approve is the current intended policy (ddf2215 commit message), so production is right and the fixture was stale. Not using env vars: `std::env::set_var` would race the parallel tests in this binary.

## 1) test_omo_backend_aborts_turn_on_repeated_approval_denials - fixture stale + production gap
- Root cause A (stale fixture): same ddf2215 policy. The default session auto-approved all 8 requests, nothing was ever denied, `approval_denials` never reached `APPROVAL_DENIAL_TURN_LIMIT` (5), the server went quiet after the last approval, and the 3s `request_timeout` inactivity path fired: "timeout during turn streaming: no activity from server" (omo_backend.rs ~1114-1123).
- Root cause B (production): the toolset-gate denial branch did `ws.send(approval_denial_response); continue;` BEFORE `approval_denials += 1`, so denials from that gate bypassed the denial-loop guard. A daemon spamming commandExecution requests against a restricted toolset would never hit the abort and would run until the deadline, which is exactly what the guard exists to stop. Setting only enabled_toolsets in the test (without B) would still time out.
- Fix:
  - Production (`omo_backend.rs`, approval block ~1282-1350): the toolset gate now sets `toolset_blocked = true` instead of `continue`. YOLO and auto-approve are skipped when `toolset_blocked`; control falls through to the shared `approval_denials += 1` / limit check / denial response. Behavior is unchanged for a single toolset denial (still answered with a denial, still beats YOLO); repeated toolset denials now abort the turn after 5 like any other policy denial. `approval_denials` still resets on turn/item frames.
  - Test: session sets `enabled_toolsets = Some(Vec::new())` so the loop is driven by the toolset gate under the default policy. Assertion "approval denied" unchanged.
- Verify note: other tests that send one toolset-denied approval (`profile_tool_restrictions_are_enforced`, `lane_silent_turn_not_interrupted_after_50s`, `discord_silent_turn_interrupted_after_45s`) only trigger 1 denial, below the limit of 5. I did not run them (rules: only target tests).

## 2) test_omo_backend_deadline_interrupts_looping_turn - TEST fixture stale vs. current deadline semantics
- Root cause: for non-cron, non-lane (interactive) sessions the read loop renews the deadline on every inbound text frame (omo_backend.rs ~1166-1173: `deadline = last_activity_at + effective_total_timeout`; invariant comment "Non-cron interactive turns auto-renew their 30m deadline while actual work events arrive"). The fake server streams a delta every 150ms for 60 iterations, so a `user-1` Discord session never reached its 600ms budget; it only ended when the loop finished (~9s) and the server then closed the stream / `turn/completed` arrived (~9.9s). Instrumenting the loop showed `to_deadline` pinned at ~599.9ms at every iteration (instrumentation removed).
- Also checked: `SessionKey::new` blanks `user_id` when a guild_id is present (src/models/session.rs:53-57), so a `cron:` user id on a guild key would not mark the session as cron.
- Fix (test only): the test now marks the session cron via `metadata["cron_scheduler_delivery"] = true`, which keeps the absolute total deadline (omo_backend.rs ~1738-1741 `is_cron_session`). The deadline, the `elapsed < 5s` assertion and the interrupt assertion are unchanged. Interactive renewal is intentional product behavior (user directive), so production is not changed.

## Final results (each run once, sequentially, shared target dir)
```
test test_omo_backend_aborts_turn_on_repeated_approval_denials ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.01s
test test_omo_backend_deadline_interrupts_looping_turn ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.66s
test test_omo_backend_e2e_thread_lifecycle_and_streaming ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.01s
```
Temporary debug prints were added and removed (`grep DBG` on both files is empty). No cargo build/check/clippy/fmt or full-suite run.

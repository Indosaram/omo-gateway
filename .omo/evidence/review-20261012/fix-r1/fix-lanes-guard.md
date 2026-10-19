# fix-lanes-guard report

RED/GREEN: deferred to single final integration gate.
At the user's directive (STOP ALL BUILDS) no cargo command was run after the edits. One earlier RED attempt was killed while still compiling and produced no test output, so its log was removed. Nothing here is compiled or test-verified yet.

## Files changed (write scope only)
- src/lanes/guard.rs: `retry_queued` rewritten in place; new private helpers `active_lane_count`, `create_launch_thread`, `record_launch_problem`; new const `LAUNCH_DISCORD_TIMEOUT` (30s)
- src/lanes/scrub.rs: `PRIVATE_KEY_BLOCK_RE`, `HOME_DIR_SLASH_RE`, `HOME_DIR_BARE_RE` and their replacement strings; 2 unit tests
- src/lanes/signals.rs: `DoneSignal::Command` branch of the runner; new `output_tail` helper and `OUTPUT_TAIL_BYTES` (4096); 2 unit tests
- src/lanes/kibitzer.rs: `evaluate` failure paths; new `restore_admitted`; 1 unit test plus a `FailingJudge` test double
- tests/test_fix_fix_lanes_guard.rs (new): runs the real Serenity HTTP client against an in-process loopback REST peer through `OMON_DISCORD_API_BASE`, never live Discord. Each request can be held open at a gate so tests are deterministic, with no sleeps.

## Defect -> fix -> test
1. [P1] Discord call held inside a tx on the single-connection pool. **FIXED (unverified).** The thread is now created (bot lookup plus `create_thread`, each under `tokio::time::timeout`) before `pool.begin()`. The tx then re-checks `status='queued'` and recounts active lanes against the limit. If the re-check fails after a thread was created, it rolls back, logs a warning and writes a `launch_orphan_thread` lane_event that names the thread id.
   Tests: `discord_thread_creation_does_not_hold_the_db_connection`, `lane_closed_during_thread_creation_is_recorded_not_launched` (tests/test_fix_fix_lanes_guard.rs).
2. [P1] The created thread was never bound to the bot. **FIXED (unverified).** The bot id comes from `http.get_current_user()`, the same way cli.rs start does. After commit it calls `crate::storage::lanes::bind_lane_thread`; a failure is recorded as a `launch_bind_failed` event. Storage files were not edited.
   Test: `promoted_thread_is_bound_to_the_creating_bot`.
3. [P1] Per-lane `?` aborted the whole pass, and the starter was re-posted every tick. **FIXED (unverified).** A Discord, parse or token failure is now per-lane: it logs, writes a `launch_failed` lane_event and moves to the next lane. A lane with a `launch_failed` event newer than its `updated_at` is skipped on later ticks, so no starter is re-posted until its row changes. DB errors still propagate as before, and existing tests rely on that.
   Test: `one_lanes_discord_failure_neither_aborts_the_pass_nor_reposts`.
4. [P1] PKCS#8 `BEGIN PRIVATE KEY` blocks were not redacted. **FIXED (unverified).** BEGIN and END each get the optional group `(?:[A-Z ]+ )?`.
   Test: `lanes::scrub::tests::test_redacts_pkcs8_and_rsa_private_key_blocks` (covers both PKCS#8 and RSA blocks).
5. [P2] All five sub-items are contained to the write scope and **FIXED (unverified)**:
   - guard.rs: queued lanes are promoted oldest-first (`list_lanes` returns newest-first, so the list is reversed). Test: `queued_lanes_are_promoted_oldest_first`.
   - signals.rs: the failure detail keeps only the last 4 KB of stdout/stderr, cut on a char boundary; the regex-mismatch detail gets the same cut. Test: `lanes::signals::tests::test_command_failure_detail_keeps_bounded_output_tail`.
   - signals.rs: `process_group(0)` on unix, plus `killpg(SIGKILL)` on timeout, so grandchildren die too. Test: `lanes::signals::tests::test_command_timeout_kills_whole_process_group` (unix; it reuses the existing `helper_signals_hung_child` socket handshake).
   - scrub.rs: the home-dir regexes now match only where a path starts (string start, whitespace, quote, `(=[{<,;:`), so URL paths such as `https://host/home/x` are left alone. Test: `lanes::scrub::tests::test_home_dir_regex_does_not_rewrite_url_paths`.
   - kibitzer.rs: on judge error, turn error, turn timeout or a reply that does not parse, `restore_admitted` puts back `consumed` (only when no newer arm landed in the meantime). SPACING still applies, so a failing judge is not hammered. Test: `lanes::kibitzer::tests::judge_failure_keeps_generation_armed`.

## Commands for the final gate (not run)
```
cargo test --lib -- lanes::scrub lanes::signals::tests::test_command_failure_detail_keeps_bounded_output_tail lanes::signals::tests::test_command_timeout_kills_whole_process_group lanes::kibitzer::tests::judge_failure_keeps_generation_armed
cargo test --test test_fix_fix_lanes_guard
cargo test --test test_lane_guard_alerts
```
`test_lane_guard_alerts` is the existing `retry_queued` coverage. By reading the code it should still pass: the active-lane limit and rollback on launch-event failure are unchanged.

## Risks for the gate reviewer
- No diagnostics or compile check has been done. The LSP timed out on every edit. Pay attention to the guard.rs `Executor` generic in `active_lane_count`, called with both `pool` and `&mut *tx`.
- The test file sets `OMON_DISCORD_API_BASE` process-wide through a `OnceLock` server. That is safe within its own test binary only.

# fix-discord-adapter (review-20261012 / fix-r1)

File changed: `src/discord/adapter.rs` only (no other files edited). The `handle_event` call sites pass the bot id from `ctx.cache.current_user()`.
Full logs: `fix-discord-adapter-red.log` (defects 2, 3 RED), `fix-discord-adapter-red-d1.log` (defect 1 RED), `fix-discord-adapter-green.log` (all GREEN).

## Defect 1 [P1] thread update/delete used a session key that never matches inbound
- Status: **FIXED**
- Changed: new private `thread_session_key(guild_id, parent_id, thread_id, bot_id)` that builds the same key as `message_to_inbound_with_config` (guild, parent channel, `thread_id=Some(thread)`, `.with_bot_id(bot)`). `handle_thread_update` and `handle_thread_delete` now take `bot_id: u64` and use it. `multiplexer.stop` errors are logged with `tracing::warn!` instead of being dropped with `let _`.
- Test: `discord::adapter::tests::thread_delete_stops_running_turn_keyed_like_inbound_thread_message`. It starts a blocking turn under the inbound-style thread key, calls `handle_thread_delete`, then waits (5s timeout) for the runner's `cancel` hook to fire. The first version of this test passed on buggy code, so it was rewritten to observe the cancel signal.
- RED: `cargo test --lib -- discord::adapter::tests::thread_delete_stops_running_turn`
  ```
  test discord::adapter::tests::thread_delete_stops_running_turn_keyed_like_inbound_thread_message ... FAILED
  panicked at src/discord/adapter.rs:5009:14:
  thread delete must stop the running thread turn: Elapsed(())
  test result: FAILED. 0 passed; 1 failed
  ```
- GREEN: see the combined command below. `thread_delete_stops_running_turn_keyed_like_inbound_thread_message ... ok`

## Defect 2 [P1] UPDATE statements wrote the nonexistent `sessions.metadata` column
- Status: **FIXED**
- Changed: `handle_thread_update` now runs one query, `UPDATE sessions SET state_json = json_set(COALESCE(state_json,'{}'), '$.metadata.thread_archived', json(?))`, with `'true'`/`'false'` bound. `handle_thread_delete` now runs `json_remove(COALESCE(state_json,'{}'), '$.metadata.omo_thread_id')`. Query failures are logged with `tracing::warn!`.
- Tests: these use `Database::connect("sqlite::memory:")`, which runs the real migrations.
  - `thread_archive_and_unarchive_write_state_json_metadata`: checks archive sets `true` and unarchive sets `false`, while `omo_thread_id` and other metadata keys are preserved.
  - `thread_delete_removes_omo_thread_id_from_state_json`: checks `omo_thread_id` is removed and other keys are kept.
- RED: `cargo test --lib -- discord::adapter::tests::thread_ discord::adapter::tests::steer_claimed_event_ledger_failure`
  ```
  test ...thread_archive_and_unarchive_write_state_json_metadata ... FAILED
  panicked at src/discord/adapter.rs:4901:9: assertion `left == right` failed
    left: Null
   right: Bool(true)
  test ...thread_delete_removes_omo_thread_id_from_state_json ... FAILED
  panicked at src/discord/adapter.rs:4949:9:
  assertion failed: state["metadata"].get("omo_thread_id").is_none()
  ```
- GREEN: both tests `... ok` (combined command below).

## Defect 3 [P2] ledger error after a successful steer caused a duplicate re-enqueue
- Status: **FIXED**
- Changed: in `steer_claimed_event`, after `multiplexer.steer` returns `Ok(true)`, a `record_incoming_as` error is now logged (`tracing::warn!`) and the function still returns `Ok(true)`, so the caller (`handle_event`) does not re-enqueue the message.
- Test: `steer_claimed_event_ledger_failure_after_steer_still_reports_steered`. A `BEFORE INSERT` trigger on `delivery_ledger` raises ABORT while a turn is active. The test asserts `Ok(true)` and that the guidance was steered exactly once.
- RED (same command as defect 2):
  ```
  panicked at src/discord/adapter.rs:4815:9:
  a landed steer must report Ok(true) so the caller does not re-enqueue it:
  Err(Database("error returned from database: (code: 1811) ledger unavailable"))
  test result: FAILED. 1 passed; 3 failed
  ```
  (The 1 that passed in that run was the defect 1 test before it was rewritten. Its real RED is shown above.)
- GREEN: `... ok`

## GREEN (all)
`cargo test --lib -- discord::adapter::tests::thread_ discord::adapter::tests::steer_claimed_event`
```
test discord::adapter::tests::thread_delete_removes_omo_thread_id_from_state_json ... ok
test discord::adapter::tests::thread_archive_and_unarchive_write_state_json_metadata ... ok
test discord::adapter::tests::steer_claimed_event_injects_into_active_turn_and_falls_back_when_idle ... ok
test discord::adapter::tests::steer_claimed_event_ledger_failure_after_steer_still_reports_steered ... ok
test discord::adapter::tests::thread_delete_stops_running_turn_keyed_like_inbound_thread_message ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 528 filtered out; finished in 0.38s
```

## Notes
- The editor auto-ran rustfmt on adapter.rs. `git diff -U0` shows hunks only in `handle_event` (thread arms), `handle_thread_update`, `handle_thread_delete`, `steer_claimed_event` and the tests module, with no unrelated reformatting.
- Not run, per the rules: the full suite, clippy, release build. Archive-path stop (`handle_thread_update` with archived=true) uses the same key builder but has no separate stop test.
- No BLOCKED-SCOPE items.

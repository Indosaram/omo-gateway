# fix-main report (src/main.rs)

The only file edited is `src/main.rs`. The tests live in `mod runner_tests`, which compiles through `src/entry.rs` (`include!("main.rs")`, bin target `omo-gateway`).
Full logs: `fix-main-red.log` and `fix-main-green.log` in this directory.

- RED command: `cargo test --bin omo-gateway fix_main`. It ran on the unfixed code, which already had the tests and the stub helpers from the previous attempt. Result: 0 passed, 4 failed.
- GREEN command: `cargo test --bin omo-gateway fix_main`. Result: 4 passed, 0 failed. `cargo test --bin omo-gateway recovery` also passed 4/4, including the existing `recovery_rechecks_current_owner_authorization` and `recovery_preserves_bot_identity`.

Note: one GREEN attempt failed to compile because of another agent's in-progress edit in `src/discord/adapter.rs:1444` (`thread.parent_id` Option mismatch). That is outside my scope. A later re-run compiled and passed.

## 1. [P1] Resume marker lost when route fails: FIXED
- Function: `recover_resume_pending_sessions_with_auth`. When `multiplexer.route(event)` returns Err, it now calls `omon_gateway::storage::mark_session_resume_pending(pool, &storage_key)` and logs the error, so the next boot retries the turn.
- Test: `legacy::runner_tests::fix_main_recovery_route_error_keeps_resume_pending`. It uses a draining multiplexer (`with_drain_receiver(true)`), which makes `route()` return Err.
- RED excerpt:
  ```
  assertion `left == right` failed: route failure must keep the session resume_pending for the next boot
    left: []
    right: [SessionKey { platform: "discord", ..., user_id: "42", bot_id: None }]
  ```
- GREEN: `fix_main_recovery_route_error_keeps_resume_pending ... ok`

## 2. [P1] JoinError from a panicked Discord task ignored: FIXED
- In `run_gateway`'s select arm, `if let Ok(Err(err)) = res` became `discord_client_failed(&res)`. The new helper `discord_client_failed` returns true for both `Ok(Err(_))` and `Err(JoinError)`, and logs each case. With that, `discord_task_failed` is set and the process exits with code 1.
- Test: `legacy::runner_tests::fix_main_panicked_discord_client_counts_as_failure`. It produces a real panicked JoinSet task, then checks `Ok(Ok)` gives false and `Ok(Err)` gives true.
- RED excerpt: `assertion failed: super::discord_client_failed(&res)` (src/main.rs:2914)
- GREEN: `fix_main_panicked_discord_client_counts_as_failure ... ok`
- Not verified: the actual `std::process::exit(1)` path in `run_gateway`, because it needs a live Discord client. The test covers the decision function that sets `discord_task_failed`.

## 3. [P2] Unparsable allowlist entries dropped silently: FIXED
- `parse_u64_list` now skips empty entries and logs a `warn!` for each non-empty entry it can't parse.
- New `parse_allowlist(name, raw)` returns `OmonError::Config` when the variable is set and non-empty but yields no ids.
- Only `DISCORD_ALLOWED_CHANNELS` in config load now goes through `parse_allowlist(...)?`, because it is the only allowlist that calls `parse_u64_list`.
- Other callers keep their old behavior apart from the new warning: `DISCORD_ALLOWED_ROLES` and `DISCORD_IGNORED_CHANNELS`. An empty `ALLOWED_ROLES` adds no access; the lead should confirm that.
- `DISCORD_ALLOWED_USERS` has its own inline parser (around :106), which this change does not touch. An all-invalid value there gives an empty list, which denies everyone unless `allow_all_users` is set, so it stays fail-closed.
- Test: `legacy::runner_tests::fix_main_allowlist_rejects_all_invalid_entries`. All-invalid input gives Err, mixed input `"1, bad, 2"` gives `[1, 2]`, and unset gives an empty list.
- RED excerpt: `assertion failed: super::parse_allowlist("DISCORD_ALLOWED_CHANNELS", Some("abc, chan-x")).is_err()`
- GREEN: `fix_main_allowlist_rejects_all_invalid_entries ... ok`

## 4. [P2] Paired users ignored by startup recovery: FIXED
- `StartupAuthorization` gets a `paired_users: Vec<u64>` field. `is_authorized_actor` now accepts actors in `allowed_users` or `paired_users`.
- `run_gateway` fills `paired_users` before `count_authorized_pending_recoveries` and recovery run. It does this with a temporary `omon_gateway::discord::PairingStore::new(pool.clone())`, then `init_cache()` and `get_paired_user_ids()`. That store reads the same `paired_users` table.
- The existing struct literals in the tests got `paired_users: vec![]` added.
- Test: `legacy::runner_tests::fix_main_recovery_authorizes_paired_users`
- RED excerpt:
  ```
  assertion `left == right` failed: paired user's pending turn must be recovered
    left: 0
    right: 1
  ```
- GREEN: `fix_main_recovery_authorizes_paired_users ... ok`

## Not done (per rules)
- I did not run `cargo fmt` or clippy; the edits were formatted by hand.
- I did not run the full suite.

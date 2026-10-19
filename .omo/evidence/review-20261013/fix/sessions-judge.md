# sessions-judge fix report

Nothing was compiled, tested or run, per the directive. rustfmt `--check` was run on the edited files (formatting only; two new tests had formatting diffs, which I fixed by hand).

## P1

- **A failed host listing is reported as "not running"** -> FIXED.
  - Code: `src/sessions/control.rs` `SessionControl::session`. It now looks up the target machine's entry in `list.errors`.
  - If that listing failed, it returns `ControlError::Command{code, message}`, which is transient and counted by the 3-error rule in `drive`. `NotRunning` is returned only when the listing succeeded and the session is absent.
  - Tests: `control.rs` unit tests `failed_listing_is_not_reported_as_not_running` and `successful_listing_without_the_session_is_not_running`. Integration test `tests/test_fix_r5_sessions_judge.rs::failed_remote_listing_does_not_fail_an_adopted_lane`.
- **Remote Done-signal checks capped at 20 s** -> FIXED.
  - Code: `control.rs` `output()` now delegates to a new `output_with_timeout(target, words, timeout)`. `external.rs` `remote_checks_with` runs remote `command` signals with `REMOTE_COMMAND_TIMEOUT` (120 s, matching the local runner). List, send, read and the file/worktree checks keep 20 s.
  - Tests: `control.rs::output_budget_is_per_call`. Integration test `remote_command_done_signal_gets_more_than_twenty_seconds` (paused time, a 60 s command).
- **`adopt` accepts a multi-line goal for a remote target** -> FIXED.
  - Code: `external.rs` `send_goal`. For non-local targets the whole composed text (goal, footer and Done text) is flattened with `" | "`, not only the footer.
  - I fixed it at send time rather than rejecting at `adopt`, so existing adopted lanes also recover.
  - Test: `remote_multiline_goal_is_flattened_and_sent`. It asserts the send happened with no newline and that `sent_at` is set.

## P2

- **`drive` fails a lane after 3 consecutive errors counted per wake** -> FIXED.
  - Code: `external.rs`, new `ErrorStreak` struct and `ERROR_WINDOW` (120 s). A lane fails only after at least 3 errors spanning at least 120 s.
  - `NotRunning` is still fatal immediately.
  - Test: `external.rs::error_streak_needs_both_count_and_elapsed_time`.
- **`assistant_items` poisons the read cursor on a non-text assistant entry** -> FIXED.
  - Code: `external.rs` `assistant_items` and the new `entry_text`. Entries with null or missing text, or without text blocks, are skipped. Content-block arrays are joined. The cursor then advances.
  - Test: `external.rs::non_text_assistant_entries_are_skipped_not_fatal`.
- **`Instant::now() - age` can panic after boot** -> FIXED.
  - Code: `registry.rs` `list_all` uses `checked_sub` and skips the cache entry on `None`.
  - No dedicated test: the failing condition needs a process clock younger than the cache age.
- **`ssh` invoked without `--`; hosts starting with `-`** -> FIXED.
  - Code: `registry.rs`.
    - New `valid_host()` is used by `configured_hosts`, `read_seeded_hosts` and `write_hosts_file`.
    - `probe_ssh_hosts` skips hosts starting with `-`.
    - `ProcessCommandRunner::run` passes `--` before the machine.
  - Test: `registry.rs::option_like_hosts_are_rejected_everywhere`.
- **JevCli daily caps are per instance and not restored on restart** -> FIXED (mostly) in `src/judge/mod.rs`.
  - New `refresh_counts()` runs at the start of `choice`, `noul` and `diff`. It raises the in-memory counters (`max`) to the `judge_calls` totals for the KST day.
  - Every instance that has a pool now shares one cap, and counts survive restarts without `init()`. `sync_persisted_counts` and `init()` still work as before; the query was extracted into `query_persisted_counts`.
  - Instances built with `pool=None` stay per-instance. This is by design, and the tests use it.
  - Test: `judge::tests::daily_cap_counts_calls_made_by_other_instances`.
- **Non-trigger failures reset the breaker counter; substring matching on "401"/"429"** -> FIXED.
  - Code: `judge/mod.rs`. `record_failure(false, ..)` no longer resets the streak; only `record_success` does.
  - `is_breaker_trigger()` uses a word-boundary regex (`\b(401|429)\b`, unauthorized, rate-limit variants) at all 3 call sites.
  - Tests: `breaker_trigger_matches_status_tokens_not_substrings`, `non_trigger_failures_do_not_reset_the_breaker_streak`, `success_still_clears_the_breaker_streak`.
- **Temp file default permissions; `set_var` from async threads** -> FIXED.
  - Code: `judge/mod.rs`.
    - `prepare_state_arg` creates the file with `OpenOptions` `create_new` and mode 0o600.
    - `load_typesafe_env` is replaced by a `OnceLock`-cached `.zshenv` read (`typesafe_env_from_zshenv`).
    - The values are passed to the child via `Command::env` in `apply_typesafe_env`, only when not already set in the process environment.
    - `set_var` is gone, so the API key is no longer inherited by unrelated children. `ensure_typesafe_env()` is kept (public API) and now primes the cache.
  - Test: `judge::tests::state_temp_file_is_owner_only`.
  - Behavior change to be aware of: `.zshenv` values are no longer visible via `std::env::var` elsewhere. `kibitzer.rs:140-142` logs `*_present` flags from the process env. Those flags will read false when the values come only from `.zshenv`. This is a log-only effect, and the file is outside my scope.
- **`external.sql` duplicates `migrations/0040_external_sessions.sql`** -> FIXED.
  - Code: `external.rs` `MIGRATION` now does `include_str!("../../migrations/0040_external_sessions.sql")`. `src/sessions/external.sql` is deleted; it was byte-identical (diffed).
  - `MIGRATION` is still exported, so the tests that import it keep working.

## BLOCKED-SCOPE

None.

## Not done

- The CONFIRMED.md P1 about `kibitzer.rs:245-250` (state document truncated by `scrub_for_external`'s 4000-char cap) lives in the lanes node's file. In `judge/mod.rs` I left `scrub_for_external` and its 4000 cap unchanged, because other callers rely on that cap.
- If the lanes node needs a higher cap for kibitzer state, the minimal change on my side would be a new `pub fn scrub_for_external_capped(input, cap)`. I did not add it, because nothing in scope uses it.

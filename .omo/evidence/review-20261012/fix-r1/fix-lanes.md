# fix-lanes (review-20261012 / fix-r1)

The regression tests are in `tests/test_fix_fix_lanes.rs`, with one test per defect.

- RED log: `fix-lanes-red.log`. It was captured by an earlier attempt of this node, on the unfixed code.
- GREEN log: `fix-lanes-green-final.log`. It was run in the foreground on the final code this session.
  - It also includes the neighbor suites `test_lane_cli`, `test_lane_events`, `test_lane_post_transport` and `test_lane_status_transport`.

RED command (earlier attempt, unfixed code):
`cargo test --test test_fix_fix_lanes`

The RED run shows `test result: FAILED. 0 passed; 5 failed`.

GREEN command (final code):
`cargo test --test test_fix_fix_lanes --test test_lane_status_transport --test test_lane_post_transport --test test_lane_events --test test_lane_cli`

GREEN results:
- test_fix_fix_lanes: 5 passed
- test_lane_cli: 7 passed
- test_lane_events: 25 passed, 1 ignored (a pre-existing helper)
- test_lane_post_transport: 2 passed
- test_lane_status_transport: 1 passed

The exit code was 0.

## Out-of-scope edit to flag (from an earlier attempt)

`tests/test_lane_events.rs:356` (`missing_destination_never_calls_sink`) was changed from `PostState::Pending` to `PostState::Failed`.

That test asserted the buggy behavior from defect 4, where a post stays pending forever. It failed after the fix (see `fix-lanes-neighbors.log`: `left: Failed right: Pending`).

The change is one line and only flips the expected state to match the fix. The lead must accept or revert it. If it is reverted, that neighbor test fails against fix 4.

## 1. [P1] request_close: forced close after a failed non-force intent — FIXED

- **File and function:** `src/storage/lanes.rs`, `request_close`.
- **Change:** on conflict with a failed row, the upsert now overwrites `original_status`, `force` and `note`. The `WHERE` guard is only `state='failed'`; the old guard required `force` to equal its previous value.
- **Test:** `forced_close_after_failed_unforced_intent_rearms_with_force`
- **RED:** `assertion left == right failed  left: "failed"  right: "pending"` (tests/test_fix_fix_lanes.rs:135)
- **GREEN:** `test forced_close_after_failed_unforced_intent_rearms_with_force ... ok`

## 2. [P1] drain_inner: per-lane status error aborts the whole drain — FIXED

- **File and function:** `src/lanes/posts.rs`, `drain_inner`.
- **Change:** the milestone `refresh_lane_status` error is now logged with `tracing::warn!` for that lane instead of being propagated with `?`. This matches what `refresh_statuses` already does.
- **Test:** `milestone_status_failure_does_not_abort_drain`
- **RED:** `called Result::unwrap() on an Err value: Config("status renames are unsupported by this sink")` (tests/test_fix_fix_lanes.rs:156)
- **GREEN:** `test milestone_status_failure_does_not_abort_drain ... ok`

## 3. [P1] refresh_lane_status: Sending/Uncertain receipt freezes status forever — FIXED (bounded unblock)

- **File and function:** `src/lanes/posts.rs`, `refresh_lane_status`, plus the new constant `STATUS_UNCERTAIN_BOUND_MINUTES = 15`.
- **Change:** a prior `Sending`/`Uncertain` receipt blocks a resend only while `now - receipt.at < 15 min`. The timestamp is the one already stored in the receipt, and `now` is injected, so the test uses no sleeps. After that window the receipt is treated as unconfirmed and the status flow resumes: rename, edit or send.
- **Not done: separate failed state for definite send errors.** Definite send errors are still recorded as `Uncertain`. I did not reclassify them for two reasons:
  1. The out-of-scope test `tests/test_lane_status_transport.rs:194` pins `"uncertain"` for a rejected status creation.
  2. The bounded unblock already removes the permanent freeze for both kinds of error.

  If the lead wants that split, the needed changes are:
  - In `refresh_lane_status`, set the `Ok(Err(error))` arm to `PostState::Failed`.
  - Change the assertion in `test_lane_status_transport.rs:194` to `"failed"`.
- **Test:** `uncertain_status_receipt_unblocks_after_bounded_interval`. It checks that:
  - the receipt still blocks at +1 minute;
  - a resend happens at +16 minutes;
  - `status_message_id` becomes `777`.
- **RED:** `assertion left == right failed  left: 1  right: 2` (tests/test_fix_fix_lanes.rs:190; no resend after the bound)
- **GREEN:** `test uncertain_status_receipt_unblocks_after_bounded_interval ... ok`

The neighbor `milestones_status_replacement_and_restart_use_owned_loopback` still passes. That test covers the 14 and 15 minute refresh cadence and the "no blind retry right after restart" behavior.

## 4. [P2] posts with a missing or invalid thread stay pending forever — FIXED

- **File and function:** `src/lanes/posts.rs`, `PostState` (new terminal variant `Failed`) and `drain_inner`.
- **Change:** the missing or invalid thread branch now sets `post.state = PostState::Failed` before persisting. The drain selects only pending posts, so these rows are no longer picked up again.
- **Test:** `post_without_thread_reaches_terminal_state`
- **RED:** `assertion left == right failed  left: ["pending"]  right: ["failed"]` (tests/test_fix_fix_lanes.rs:212)
- **GREEN:** `test post_without_thread_reaches_terminal_state ... ok`. The neighbor test `missing_destination_never_calls_sink` also passes (see the out-of-scope note above).

## 5. [P2] lane start: cap check and insert are not atomic; Discord thread created before insert — FIXED

- **Files and functions:**
  - `src/lanes/cli.rs`, `start`.
  - `src/storage/lanes.rs`, `insert_lane`. The signature is generalized to `impl sqlx::Executor`. Existing `&SqlitePool` callers still compile; all neighbor suites built and passed.
- **Change:**
  - `start` opens `BEGIN IMMEDIATE` and re-counts active lanes inside the transaction. It uses the same status set as `guard::query_active_lanes_sql`. If the count reaches `lanes_max`, the decision becomes `Hold` (exit 4, lane stays `queued`).
  - `insert_lane` and `commit` run in the same transaction.
  - If the insert fails after a Discord thread was created, the thread is archived and the failure is logged before the error is returned.
- **Limitation:** the Discord thread is still created while the write transaction is held. That keeps creation from racing, but it holds the SQLite write lock for the length of the Discord call.
- **Test:** `start_rechecks_lane_cap_atomically_with_insert`. It passes a stale `LaunchDecision::Ok` when `lanes_max` lanes are already active, and expects exit 4 with the lane `queued`.
- **RED:** `assertion left == right failed  left: 0  right: 4` (tests/test_fix_fix_lanes.rs:241)
- **GREEN:** `test start_rechecks_lane_cap_atomically_with_insert ... ok`
- **Not unit-tested:** the archive-on-insert-failure path needs live Discord thread creation. It was only checked by reading the code. The cap-recheck path is covered by the test above.

## Not run (per rules)
- Full suite, clippy and rustfmt were not run; the lead runs them once at the end.

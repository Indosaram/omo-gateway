# Implementation Report: Todo 7 (Report known startup failures on the first lane turn)

## Summary of Changes
Implemented early detection and reporting for known startup failures during the lane's very first turn (`first_turn` where `lane.turns_run == 0`), distinguishing permanent configuration/environment failures from transient rate/quota limits according to decision Q3 and the dori-mode contract.

## Files Changed
1. `src/lanes/runner.rs`
   - Added `let first_turn = lane.turns_run == 0;` before prompt building.
   - After joined turn outcome recording and paused/shutdown handling (before error count update):
     - Checked if `outcome.is_err() && first_turn`.
     - Lowercased outcome text and matched:
       - Permanent failure substrings (`api key`, `unknown provider`, `model_not_available`, `all fallback models failed`, `cannot find module`, `session crashed`): immediately calls `self.fail(key, "startup_error", &outcome_text, generation)`.
       - Transient failure substrings (`rate limit`, `quota`, `usage limit`): logs one `lane_events` row of kind `startup_error` indicating retry (`format!("{}: will retry", outcome_text)`), and continues standard error retry counting.
2. `src/lanes/dispatch.rs`
   - Added `"startup_error"` to `fn critical(event: &LaneEvent) -> bool` list.
   - Added unit test assertion in `owner_visible_failure_events_skip_the_wake_judgment`: `assert!(critical(&event("startup_error")));`.
3. `tests/test_lane_runner.rs`
   - Extended `Script` struct with `error_text: Option<&'static str>` (defaults to `"scripted turn failure"` via `script` or specified via `script_with_error`).
   - Added new test functions:
     - `permanent_startup_error_fails_on_first_turn`: verifies error with `"No API key for provider x"` fails immediately on first turn (`turns_run == 1`, status `"failed"`) and emits 1 `startup_error` event.
     - `transient_startup_error_reports_and_retries`: verifies error with `"rate limit exceeded"` emits 1 `startup_error` event and continues retrying until 3 errors fail with `"dead"`.
     - `ordinary_first_turn_error_still_retries`: verifies ordinary errors (e.g. `"connection reset"`) emit 0 `startup_error` events and continue standard 3-error retry behavior.

## Tests Added or Updated (exact fn names)
- `owner_visible_failure_events_skip_the_wake_judgment` (in `src/lanes/dispatch.rs` tests module, updated)
- `permanent_startup_error_fails_on_first_turn` (in `tests/test_lane_runner.rs`, new)
- `transient_startup_error_reports_and_retries` (in `tests/test_lane_runner.rs`, new)
- `ordinary_first_turn_error_still_retries` (in `tests/test_lane_runner.rs`, new)

## Deviations from the Plan
None. All implementations match the exact specifications, keywords, and behavior prescribed in Todo 7.

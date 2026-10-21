# impl-3: confirm a pause before reporting it, report output after a confirmed pause

Nothing was compiled or tested, per the owner rule. The single build and test run is todo 9.

## Files changed
- migrations/0055_lane_pause_ack.sql (new): `ALTER TABLE lane_pauses ADD COLUMN acked_at DATETIME; ALTER TABLE lane_pauses ADD COLUMN seen_working_at DATETIME;`
- src/storage/lanes.rs:
  - New `pub enum PauseOutcome { Paused, AlreadyPaused, NotPausable }` and `pub async fn try_pause_lane(pool, key, by) -> Result<PauseOutcome>`. It reads `status, kind`, and for `kind = 'external'` it rolls back and returns `NotPausable` without writing a row. The insert/upsert now resets `acked_at = NULL, seen_working_at = NULL`.
  - `pause_lane` keeps its `Result<bool>` signature for its existing callers (tests a-e, test_lane_done) and wraps `try_pause_lane`. An external lane becomes `Err(Config("NOT_PAUSABLE: ..."))`.
  - New `ack_pause(executor, key) -> Result<bool>`, which runs the exact UPDATE from the plan, and `pause_acked(pool, key) -> Result<bool>`.
  - New `unacked_pause_keys(pool) -> Result<Vec<String>>`, which runs the SELECT that ack point (d) uses.
- src/lanes/cli.rs:
  - `LaneCommand::Pause` now calls `try_pause_lane`. `NotPausable` gives exit 2 with `{"ok":false,"key","error":"NOT_PAUSABLE"}`.
  - After `wake`, it calls `confirm_pause`. Success gives exit 0 with `{"key","paused":true,"confirmed":true,"changed"}`. Timeout gives exit 3 with `{"ok":false,"key","error":"PAUSE_UNVERIFIED","paused":true}`, and the row is kept.
  - New `confirm_pause`: it checks `pause_acked` first, then re-reads the row on a 250 ms tokio interval until a deadline. The deadline is 30 s by default and can be overridden with `OMON_LANE_PAUSE_CONFIRM_SECS` (u64). The select also wakes on `sleep_until(deadline)`, so the deadline is exact.
- src/lanes/runner.rs: four ack points.
  - (a) At `run_lane` start, when the lane is already paused.
  - (b) At the turn-boundary return. The `paused` check was split out of the combined condition so that only the paused branch acks.
  - (c) At `shutdown.is_cancelled() || paused_now`, when `paused_now` is set, for both Ok and Err outcomes.
  - (d) On every service loop pass, after the working/not_done spawn loop and gated on `lane_control_enabled()` like `paused()`: each key from `unacked_pause_keys` that has no entry in `active` is acked.
- src/lanes/activity.rs `capture`: inside the same tx, after the lane row update and the replay check, it runs `UPDATE lane_pauses SET seen_working_at = ? WHERE lane_key = ? AND resumed_at IS NULL AND acked_at IS NOT NULL AND seen_working_at IS NULL`. When one row is affected, it inserts one `working_while_paused` lane_events row through the existing `self.event` helper (lead taken from the lane row). The body is `lane <key> produced output after its pause was confirmed`. No other state changes. This covers `captured`, `captured_generation` and `captured_item`.
- src/lanes/dispatch.rs: the test `owner_visible_failure_events_skip_the_wake_judgment` now also asserts `!critical(&event("working_while_paused"))`. The `critical` match itself is unchanged; the event is already non-critical.
- bot_profiles/blocks/lanes.md "Lane events": one line each for PAUSE_UNVERIFIED, NOT_PAUSABLE, `working_while_paused`, `startup_error` and the `unpushed <path> (no remote)` objection.

## Tests (tests/test_lane_control.rs)
- Updated `f_resume_on_an_unpaused_lane_exits_2_with_not_paused`: the pause step now expects exit 3 with `error == "PAUSE_UNVERIFIED"` and `paused == true`. The list and resume steps are unchanged.
- Updated helper `cli()`: it now delegates to the new `run_cli(url, dir, args, confirm_secs)` with `OMON_LANE_PAUSE_CONFIRM_SECS=1`. `run_cli(..., None)` removes the variable so the 30 s default applies.
- New helpers: `file_database`, `insert_ctl`, `working_while_paused_events`.
- New `h_pause_without_runner_exits_3_unverified`: exit 3, PAUSE_UNVERIFIED, the row stays open, and it is not acked.
- New `i_pause_with_running_service_confirms`: file DB, `LaneRunner::serve` with the blocking `Fake` backend (turn 1 pends until cancelled). The CLI pause runs with the 30 s default: exit 0, confirmed true, changed true, `last_turn_outcome == "paused"`. A second pause gives exit 0 with changed false, and the backend was called once.
- New `j_output_after_confirmed_pause_emits_one_event`:
  - Output before the ack produces no event.
  - After `ack_pause`, two `captured` calls produce exactly one event, whose lead is the lane's lead. The lane stays paused and working.
  - On the external path, a lane row and an acked pause row are inserted with SQL, then two `captured_item` calls produce exactly one event.
- New `k_pause_of_done_claimed_lane_confirms_via_service_pass`: a done_claimed lane (claim_at = now, inside the 5-minute window) with the service running. The CLI pause exits 0 with confirmed true, the status stays done_claimed, and the backend is never called. This exercises ack point (d).
- New `l_pause_of_external_lane_is_refused`: exit 2 with NOT_PAUSABLE, no lane_pauses row is written, and `pause_lane` returns Err.
- No sleeps are used. The tests await the CLI child exit, which its own deadline bounds, and the existing `BOUND` timeouts.

## Deviations
- I added `try_pause_lane` / `PauseOutcome` as the "typed error" rather than adding a new `OmonError` variant. `src/error.rs` is outside the write list, and `pause_lane` keeps its existing signature for its other callers.
- I added `unacked_pause_keys` to storage/lanes.rs. The plan put this SELECT inline in the runner; keeping it in storage keeps the lane_pauses SQL in one module, and the behaviour is the same.
- Test j does not compare the exact event body text, because the repo test rules forbid pinning prose. It asserts the kind, the count, the lead, and that the body contains the lane key.

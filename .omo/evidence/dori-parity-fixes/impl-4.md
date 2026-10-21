# Implementation Report: Todo 4 (Alert when active lanes reach the maximum)

## Files Changed
- `src/lanes/guard.rs`:
  - Extended `AlertMark` with `lanes: bool` field.
  - Updated `worth_alerting`, `got_worse`, and `mark_alert` methods on `GuardWatcher` to track `lanes` saturation status alongside `load`, `mem`, and `disk`.
  - In `check_at`:
    - Evaluated `let lanes_alert = sample.active_lanes >= self.thresholds.lanes_max`.
    - Added reason format `"active lanes ({}) >= maximum ({})"`.
    - Included `lanes_cleared = sample.active_lanes < self.thresholds.lanes_max` in the `GuardState::Alert` transition back to `Clear`.
  - Added unit test `lane_count_alerts_and_clears` in `src/lanes/guard.rs` tests module.
- `tests/test_lane_guard_alerts.rs`:
  - Added integration test `lane_saturation_alert_once_then_clear` verifying alert reason on reaching `lanes_max`, deduplication on repeated sample, and emission of `CLEAR` once active lanes drop below `lanes_max`.

## Tests Added or Updated
- `src/lanes/guard.rs`:
  - `lane_count_alerts_and_clears` (new)
- `tests/test_lane_guard_alerts.rs`:
  - `lane_saturation_alert_once_then_clear` (new)

## Deviations from the Plan
None. All requirements implemented exactly as specified in Todo 4.

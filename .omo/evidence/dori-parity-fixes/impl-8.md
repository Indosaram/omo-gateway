# Implementation Report: Todo 8 - Show paused lanes as Paused in the tracker

## 1. Overview
Implemented Todo 8 of the approved work plan `.omo/plans/dori-parity-fixes.md`. Paused lanes are now reflected with a dedicated "Paused" workflow state in the AGT (Agents) system team in Tracker. The state transitions correctly between Paused and active lane statuses, preserves closed/failed/verified statuses, and guards against manual moves to the Paused state.

## 2. Changed Files
- `migrations/0056_tracker_lane_paused.sql` (new)
  - Added `lane_paused INTEGER NOT NULL DEFAULT 0` column to `tracker_states`.
  - Inserted the initial 'Paused' state row (`type = 'started'`, `color = '#9b8afb'`, `position = 2.5`, `lane_paused = 1`) for existing `AGT` team.
- `src/tracker/seed.rs`
  - Updated `seed_agent_states` to set `lane_paused = 0` for standard agent states and insert the 'Paused' row with `lane_paused = 1` for newly initialized AGT teams.
- `src/tracker/lanes.rs`
  - In `sync_inner`, queried `paused_state` (AGT state with `lane_paused = 1`) and `paused_lane_keys` (`crate::storage::lanes::paused_lane_keys`).
  - Derived `is_paused`: true only when open pause row exists and status is in `('queued', 'working', 'blocked', 'done_claimed', 'not_done')`.
  - Handled `effective_status` (`"paused"` or original `lane_status`) for `status_changed` check, target state selection, and `tracker_activity` recording (`to_value = "paused"`).
  - Applied the same paused state and activity mapping to both existing-issue path and new-issue branch.
- `src/tracker/issues.rs`
  - In state PATCH handler (`apply_patch`), before calling `lanes::handle_lane_state_transition` for lane-linked issues, queried `lane_paused` on target `state_id`.
  - Returns HTTP 409 `illegal_lane_transition` ("Paused is set by lane pause") if `lane_paused == 1`.
- `tests/test_tracker_api.rs`
  - Added integration tests covering paused state mirroring, transitions back to active status, closed lane pause behavior, and guarded manual moves.

## 3. Added or Updated Tests (Exact Function Names)
- `tests/test_tracker_api.rs`:
  - `bootstrap_seeds_workspace_and_agent_team` (updated): asserts AGT team contains a 'Paused' state.
  - `paused_lane_mirrors_to_paused_state_and_back` (new): working lane with open pause row mirrors to Paused state on sync; setting `resumed_at` moves it back to Working.
  - `closed_lane_with_open_pause_stays_done` (new): closed lane with an open pause row stays in Done (`closed`), does not move to Paused.
  - `moving_a_closed_lane_issue_to_paused_is_refused` (new): manual PATCH moving a lane-linked issue to the Paused state returns HTTP 409 `illegal_lane_transition`, lane status remains `closed`.

## 4. Deviations from the Plan
None. All requirements, constraints, and acceptance criteria were implemented strictly as specified.

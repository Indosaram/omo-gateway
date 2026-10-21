# dori-parity-fixes review r1

Scope: `git diff` vs main c96cf4c1 in wt/dori-parity, plus the untracked migrations 0055 and 0056. Read-only review; no cargo run.

Todos 1-8: all acceptance items are present in code and tests. Checked: settle and keep_reason refuse no-remote commits and pass an unborn HEAD; the paused guard is gone from settle; the pause ack happens at points a-d, each only after the turn task is joined or no task exists; the CLI returns exit 0, 3 or 2 as specified; working_while_paused is emitted once per pause inside the capture tx; the guard alerts at `>=` and CLEAR waits for the lane count to clear; signal parsing; lane_lead_or_default is used at both call sites; startup_error handling; the tracker Paused state plus its migration and seed. Migration 0056 is safe on existing DBs: sqlx runs it once, the INSERT...SELECT adds Paused only to an already-seeded AGT team, and on a fresh DB the table is empty when 0056 runs, so seed_agent_states adds the single Paused row. Neither path creates a duplicate.

## Findings

### 1. Transient `startup_error` event body is not scrubbed or capped (src/lanes/runner.rs:2039-2048)
- **Why it is wrong:** the transient branch inserts `format!("{outcome_text}: will retry")` straight into `lane_events.body`. `outcome_text` is the raw backend error (`error: {error}`). Every other lane event goes through `scrub()` and `.chars().take(2000)` before it is stored: `fail()` at runner.rs:2174, `LaneActivity::event` at activity.rs:363, and settle.rs:145. A provider quota or rate-limit error can contain home paths or tokens, and the text is forwarded unredacted to the lead and from there to Slack. The permanent branch uses `fail()` and is scrubbed, so only the transient path leaks.
- **Minimal fix:** `let body: String = super::scrub::scrub(&format!("{outcome_text}: will retry")).chars().take(2000).collect();`

### 2. Pause then resume re-sends the tracker status notification (src/tracker/lanes.rs:282-305, and the same pattern at 412)
- **Why it is wrong:** `status_changed` now compares against `effective_status`. Take a lane that is `blocked` or `done_claimed`, pause it, then resume it. The sync records `blocked -> paused` and then `paused -> blocked`. On the resume sync, `!is_paused && lane_status in (blocked, done_claimed, verified_done, failed)` holds, so a second `lane_blocked` or `lane_done_claimed` notification "Lane X entered status blocked" is inserted even though the lane status never changed. Before this diff a pause produced no notification at all, so this is a regression: every pause/resume cycle re-notifies the owner.
- **Minimal fix:** add `&& last_lane_status.as_deref() != Some("paused")` to the notification condition at :282, or compare against the last non-paused status. The new-issue branch at :412 is unaffected because it has no previous status.

No other concrete defects found. The pause-ack races (the service-pass ack skips keys in `active`; the turn-boundary ack runs after the join), the lane_pauses upsert (resets `acked_at` and `seen_working_at` on a re-pause), and the migration idempotence all hold.

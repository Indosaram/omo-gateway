# Focused Behavior Validation Results: dori-next (20261011T140523Z)

## Execution Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next` (Frozen, zero source edits)
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo test -j 2 --all-features --test test_lane_done --test test_lane_dispatch --test test_lane_posts --test test_lane_runner --test test_lane_cli --test test_lane_events --test test_lane_reopen --no-fail-fast`
- **Exit Code**: `101`
- **Raw Log**: `next-focused-20261011T140523Z.log`
- **Status Ledger**: `next-focused-20261011T140523Z-status.tsv`
- **Overall Stats**: 67 run, 61 passed, 6 failed, 1 ignored

## Target Breakdown

| Test Target | Category / Ownership | Run | Passed | Failed | Ignored | Status |
|---|---|---|---|---|---|---|
| `test_lane_cli` | Todo 13 | 7 | 7 | 0 | 0 | PASS |
| `test_lane_runner` | Todo 13 | 12 | 11 | 0 | 1 | PASS |
| `test_lane_done` | Todo 13 | 7 | 6 | 1 | 0 | FAIL |
| `test_lane_dispatch` | Todo 14 | 8 | 8 | 0 | 0 | PASS |
| `test_lane_posts` | Todo 18 | 8 | 5 | 3 | 0 | FAIL |
| `test_lane_reopen` | Existing Lane Integration | 4 | 4 | 0 | 0 | PASS |
| `test_lane_events` | Existing Lane Integration | 24 | 22 | 2 | 0 | FAIL |

---

## Failures Grouped by Ownership

### 1. Todo 13 Failures (1 failed test across 3 targets)
*Targets: `test_lane_done`, `test_lane_runner`, `test_lane_cli`*

- **Test Name**: `unpushed_commit_with_local_bare_remote_returns_not_done`
  - **Target**: `tests/test_lane_done.rs`
  - **File:Line**: `tests/test_lane_done.rs:393:5`
  - **Panic**: `assertion failed: reason.contains(&format!("unpushed {}", fixture.lane.cwd))`

---

### 2. Todo 14 Failures (0 failed tests across 1 target)
*Targets: `test_lane_dispatch`*

- None (All 8 tests passed).

---

### 3. Todo 18 Failures (3 failed tests across 1 target)
*Targets: `test_lane_posts`*

1. **Test Name**: `test_repeated_drain_does_not_repost_milestone`
   - **Target**: `tests/test_lane_posts.rs`
   - **File:Line**: `tests/test_lane_posts.rs:307:5`
   - **Panic**: `assertion left == right failed (left: 0, right: 1)`

2. **Test Name**: `test_two_milestones_produce_two_posts_and_two_status_edits`
   - **Target**: `tests/test_lane_posts.rs`
   - **File:Line**: `tests/test_lane_posts.rs:109:5`
   - **Panic**: `assertion left == right failed (left: 0, right: 1)`

3. **Test Name**: `test_status_edit_404_creates_new_message_and_stores_id`
   - **Target**: `tests/test_lane_posts.rs`
   - **File:Line**: `tests/test_lane_posts.rs:270:5`
   - **Panic**: `assertion left == right failed (left: 0, right: 1)`

---

### 4. Existing Lane Integration Failures (2 failed tests across 2 targets)
*Targets: `test_lane_events`, `test_lane_reopen`*

1. **Test Name**: `acknowledged_post_is_not_sent_again_after_recovery`
   - **Target**: `tests/test_lane_events.rs`
   - **File:Line**: `tests/test_lane_events.rs:152:5`
   - **Panic**: `assertion failed: received.try_recv().is_err()`

2. **Test Name**: `ambiguous_post_stays_uncertain_without_retry`
   - **Target**: `tests/test_lane_events.rs`
   - **File:Line**: `tests/test_lane_events.rs:175:5`
   - **Panic**: `assertion failed: received.try_recv().is_err()`

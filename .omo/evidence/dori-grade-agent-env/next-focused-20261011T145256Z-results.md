# Focused Test Batch (runner + reopen) Results: dori-next (20261011T145256Z)

## Execution Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next` (Frozen, zero source edits)
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo test -j 2 --all-features --test test_lane_runner --test test_lane_reopen --no-fail-fast`
- **Exit Code**: `0`
- **Raw Log**: `next-focused-20261011T145256Z.log`
- **Status Ledger**: `next-focused-20261011T145256Z-status.tsv`
- **Overall Statistics**: 16 total (15 passed, 0 failed, 1 ignored)

---

## Target Breakdown

| Target Name | Category / Ownership | Total | Passed | Failed | Ignored | Status |
|---|---|---|---|---|---|---|
| `test_lane_reopen` | Existing Lane Integration / Shared | 4 | 4 | 0 | 0 | **PASS** |
| `test_lane_runner` | Todo 13 | 12 | 11 | 0 | 1 | **PASS** |

---

## Ignored Tests
In `tests/test_lane_runner.rs` (1 test):
- **Test Name**: `live_qa_runner_process`
- **Reason**: `requires a disposable app-server and explicit QA database`

---

## Verified Fix Invariant
- `close_reopen_discards_old_turn_and_delivers_new_owner_text_first` in `tests/test_lane_reopen.rs`: **PASSED**.
- Runner race repair and scoped notification fixture verified under live test execution.
- Zero test failures and zero panics.

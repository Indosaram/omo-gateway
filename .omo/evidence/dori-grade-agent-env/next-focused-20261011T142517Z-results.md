# Focused Test Batch (reopen + inbox) Results: dori-next (20261011T142517Z)

## Execution Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next` (Frozen, zero source edits)
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo test -j 2 --all-features --test test_lane_reopen --test test_inbox_sources --no-fail-fast`
- **Exit Code**: `101`
- **Raw Log**: `next-focused-20261011T142517Z.log`
- **Status Ledger**: `next-focused-20261011T142517Z-status.tsv`
- **Overall Statistics**: 21 total (16 passed, 1 failed, 4 ignored)

---

## Target Breakdown

| Target Name | Total | Passed | Failed | Ignored | Status |
|---|---|---|---|---|---|
| `test_inbox_sources` | 17 | 13 | 0 | 4 | **PASS** |
| `test_lane_reopen` | 4 | 3 | 1 | 0 | **FAIL** |

---

## Ignored Tests
In `tests/test_inbox_sources.rs` (4 tests):
1. `live_discord_source_fetch_once` (ignored)
2. `live_imessage_source_fetch_once` (ignored)
3. `live_kakao_source_fetch_once` (ignored)
4. `live_mail_source_fetch_once` (ignored)

---

## Test Failure Details
- **Target**: `tests/test_lane_reopen.rs`
- **Test Name**: `close_reopen_discards_old_turn_and_delivers_new_owner_text_first`
- **Panic Location**: `tests/test_lane_reopen.rs:196:9` (JoinError unwrapped at line 239:10)
- **Panic Message**: `the reopening message must not be steered into the old turn`

# Strict Clippy Diagnostics: dori-next (20261011T142121Z)

## Execution Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next` (Frozen, zero source edits)
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo clippy -j 2 --all-targets --all-features -- -D warnings`
- **Exit Code**: `101`
- **Raw Log**: `next-clippy-20261011T142121Z.log`
- **Status Ledger**: `next-clippy-20261011T142121Z-status.tsv`
- **Total Diagnostics in this run**: 2 errors

---

## Crate Stage Analysis
- **Core Library (`src/lib.rs`)**: **PASSED** (0 errors, 0 warnings).
- **Target `test_lane_posts` (`tests/test_lane_posts.rs`)**: **PASSED** (0 clippy errors; lexical scope fix verified).
- **Failing Target**: `tests/test_lane_reopen.rs` (test target `test_lane_reopen`).
- **Downstream Visibility Note**: Because cargo clippy stops when a crate target fails (`warning: build failed, waiting for other jobs to finish...`), subsequent test targets—including `tests/test_inbox_sources.rs:528/552` (unit-struct defaults)—remain masked until `test_lane_reopen.rs` is resolved.

---

## Diagnostic Locations Detail

### 1. `tests/test_lane_reopen.rs:121:9`
- **Lint**: `clippy::await_holding_lock` (implied by `-D warnings`)
- **Severity**: error
- **Message**: `this `MutexGuard` is held across an await point`
- **Await Points**: Line 130 (`.await`) and Line 137 (`get_lane_history(...).await`)
- **Owning Todo**: **Shared** (Existing lane integration)
- **Help / Suggested Fix**: Drop `seen` before calling `.await`, or clone the needed state into local values.

### 2. `tests/test_lane_reopen.rs:242:9`
- **Lint**: `clippy::await_holding_lock` (implied by `-D warnings`)
- **Severity**: error
- **Message**: `this `MutexGuard` is held across an await point`
- **Await Points**: Line 246 (`.fetch_one(db.pool()).await`) and Line 249 (`.fetch_one(db.pool()).await`)
- **Owning Todo**: **Shared** (Existing lane integration)
- **Help / Suggested Fix**: Drop `seen` before calling `.await`, or clone the needed state into local values.

---

## Grouped Summary by Ownership

| Category / Ownership | Targets / Files | Clippy Errors | Details |
|---|---|---|---|
| **Todo 13** | `src/lanes/{runner,cli,state,settle}.rs`, `tests/test_lane_{done,runner,cli}.rs` | 0 | clean |
| **Todo 14** | `src/lanes/dispatch.rs`, `src/models/*`, `src/multiplexer/*`, `src/storage/db.rs`, `src/main.rs`, `src/discord/adapter.rs`, `tests/test_lane_dispatch.rs` | 0 | clean |
| **Todo 18** | `src/lanes/{posts,discord_threads,activity}.rs`, `src/storage/lanes.rs`, `tests/test_lane_posts.rs` | 0 | **CLEAN** (`test_lane_posts.rs` fix verified, 0 errors) |
| **Shared** | `tests/test_lane_reopen.rs` | 2 | `tests/test_lane_reopen.rs:121:9`, `tests/test_lane_reopen.rs:242:9` (`await_holding_lock`) |
| **Masked Downstream** | `tests/test_inbox_sources.rs:528, 552` | (masked) | `default_constructed_unit_structs` waiting behind `test_lane_reopen` |

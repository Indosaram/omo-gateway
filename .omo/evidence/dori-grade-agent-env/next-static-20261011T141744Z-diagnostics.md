# Static Diagnostics: dori-next (20261011T141744Z)

## Execution Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next` (Frozen, zero source edits)
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Status Ledger**: `next-static-20261011T141744Z-status.tsv`
- **Exit Codes**:
  - `cargo fmt --check`: **1**
  - `cargo clippy -j 2 --all-targets --all-features -- -D warnings`: **101**
- **Evidence Files**:
  - `next-static-20261011T141744Z-fmt.log`
  - `next-static-20261011T141744Z-clippy.log`
  - `next-static-20261011T141744Z-status.tsv`
  - `next-static-20261011T141744Z-diagnostics.md`

---

## 1. `cargo fmt --check` Diff Locations
- **Exit Code**: `1`
- **Diff Count**: 1 file / 1 location
- **Locations**:
  - `/Users/indo/code/project/omon-gateway-wt/dori-next/src/agent/backend.rs:61`
    - Classification: **Shared** (Known baseline diff: trailing newline adjustment)
- **Note**: No other source or test files have formatting discrepancies. All todo13, todo14, and todo18 files are clean.

---

## 2. `cargo clippy` Diagnostics
- **Exit Code**: `101`
- **Crate Stage**: Core library (`src/lib.rs`) passed clippy with 0 errors/warnings.
- **Failing Target**: `tests/test_lane_posts.rs` (Test target `test_lane_posts`).
- **Downstream Visibility Note**: Because cargo stops compilation/checking on failure in `test_lane_posts`, clippy diagnostics in subsequent test targets are not yet visible.

### Clippy Errors Detail (2 errors)

1. **Error 1**:
   - **Lint**: `clippy::await_holding_lock` (implied by `-D warnings`)
   - **File:Line**: `tests/test_lane_posts.rs:279:9`
   - **Message**: `this `MutexGuard` is held across an await point`
   - **Await Point**: `tests/test_lane_posts.rs:285:10`
   - **Owning Todo**: **Todo 18** (`tests/test_lane_posts.rs`)
   - **Help / Suggested Fix**: Ensure the `MutexGuard` is dropped before `.await`, or extract the cloned value into a local variable before the async call.

2. **Error 2**:
   - **Lint**: `clippy::await_holding_lock` (implied by `-D warnings`)
   - **File:Line**: `tests/test_lane_posts.rs:397:19`
   - **Message**: `this `MutexGuard` is held across an await point`
   - **Await Points**: `tests/test_lane_posts.rs:417:46` and `tests/test_lane_posts.rs:420:10`
   - **Owning Todo**: **Todo 18** (`tests/test_lane_posts.rs`)
   - **Help / Suggested Fix**: Clone or extract the message tuple value before calling `insert_lane` and subsequent `.await` operations.

---

## 3. Grouped Summary by Ownership

### Todo 13
*Ownership: `src/lanes/runner.rs`, `src/lanes/cli.rs`, `src/lanes/state.rs`, `src/lanes/settle.rs`, `tests/test_lane_done.rs`, `tests/test_lane_runner.rs`, `tests/test_lane_cli.rs`*
- **fmt diffs**: 0
- **clippy errors**: 0

### Todo 14
*Ownership: `src/lanes/dispatch.rs`, `src/models/*`, `src/multiplexer/*`, `src/storage/db.rs`, `src/main.rs`, `src/discord/adapter.rs`, `tests/test_lane_dispatch.rs`, InboundEvent literal sites*
- **fmt diffs**: 0
- **clippy errors**: 0

### Todo 18
*Ownership: `src/lanes/posts.rs`, `src/lanes/discord_threads.rs`, `src/lanes/activity.rs`, `src/storage/lanes.rs`, `tests/test_lane_posts.rs`*
- **fmt diffs**: 0
- **clippy errors**: 2
  - `tests/test_lane_posts.rs:279:9`: `clippy::await_holding_lock`
  - `tests/test_lane_posts.rs:397:19`: `clippy::await_holding_lock`

### Shared
*Ownership: Any file not covered by Todo 13, 14, or 18*
- **fmt diffs**: 1
  - `src/agent/backend.rs:61` (Known baseline formatting diff)
- **clippy errors**: 0

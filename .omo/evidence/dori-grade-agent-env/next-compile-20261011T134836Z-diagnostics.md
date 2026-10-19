# Compile Gate Diagnostics: dori-next (20261011T134836Z)

## Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next`
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo test -j 2 --all-targets --all-features --no-run`
- **Exit Code**: `101`
- **Raw Log**: `next-compile-20261011T134836Z.log`
- **Status Ledger**: `next-compile-20261011T134836Z-status.tsv`
- **Total Diagnostics**: 2 errors, 0 warnings

## Crate Stage Analysis
- **Library Stage (`src/lib.rs`)**: **SUCCEEDED**. No compilation errors occurred in the core library or binary targets.
- **Failing Stage**: Test target **`test_lane_posts`** (`tests/test_lane_posts.rs`).
- **Downstream Targets Visibility**: Because Cargo aborts on test target failure (`warning: build failed, waiting for other jobs to finish...`), any potential errors in test targets queued after `test_lane_posts` are **not yet visible** until this target is unblocked.

## Diagnostics by Owning Todo

### Todo 18
*Ownership: `src/lanes/posts.rs`, `src/lanes/discord_threads.rs`, `src/lanes/activity.rs`, `src/storage/lanes.rs`, `tests/test_lane_posts.rs`*

| Severity | Code | File:Line | Message | Suggested Fix / Context |
|---|---|---|---|---|
| error | E0599 | `tests/test_lane_posts.rs:79:34` | no method named `now` found for struct `MockClock` in the current scope | Add `use omon_gateway::Clock;` to bring `Clock` trait into scope |
| error | E0599 | `tests/test_lane_posts.rs:80:35` | no method named `now` found for struct `MockClock` in the current scope | Add `use omon_gateway::Clock;` to bring `Clock` trait into scope |

### Todo 13
*Ownership: `src/lanes/runner.rs`, `src/lanes/cli.rs`, `src/lanes/state.rs`, `src/lanes/settle.rs`, `tests/test_lane_done.rs`, `tests/test_lane_runner.rs`, `tests/test_lane_cli.rs`*
- None (0 errors, 0 warnings)

### Todo 14
*Ownership: `src/lanes/dispatch.rs`, `src/models/*`, `src/multiplexer/*`, `src/storage/db.rs`, `src/main.rs`, `src/discord/adapter.rs`, `tests/test_lane_dispatch.rs`, InboundEvent literal sites*
- None (0 errors, 0 warnings)

### Shared
*Ownership: Any file not covered by Todo 13, 14, or 18*
- None (0 errors, 0 warnings)

## Raw Diagnostics Excerpt
```
error[E0599]: no method named `now` found for struct `MockClock` in the current scope
  --> tests/test_lane_posts.rs:79:34
   |
79 |     lane.created_at = Some(clock.now());
   |                                  ^^^ method not found in `MockClock`
   |
  ::: /Users/indo/code/project/omon-gateway-wt/dori-next/src/work_heartbeat.rs:71:8
   |
71 |     fn now(&self) -> DateTime<Utc>;
   |        --- the method is available for `MockClock` here
   |
   = help: items from traits can only be used if the trait is in scope
help: trait `Clock` which provides `now` is implemented but not in scope; perhaps you want to import it
   |
 1 + use omon_gateway::Clock;
   |

error[E0599]: no method named `now` found for struct `MockClock` in the current scope
  --> tests/test_lane_posts.rs:80:35
   |
80 |     lane.last_activity_at = clock.now();
   |                                   ^^^ method not found in `MockClock`
   |
  ::: /Users/indo/code/project/omon-gateway-wt/dori-next/src/work_heartbeat.rs:71:8
   |
71 |     fn now(&self) -> DateTime<Utc>;
   |        --- the method is available for `MockClock` here
   |
   = help: items from traits can only be used if the trait is in scope
help: trait `Clock` which provides `now` is implemented but not in scope; perhaps you want to import it
   |
 1 + use omon_gateway::Clock;
   |
```

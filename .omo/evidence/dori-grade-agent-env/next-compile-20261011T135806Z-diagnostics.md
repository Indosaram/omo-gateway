# Compile Gate Diagnostics: dori-next (20261011T135806Z)

## Summary
- **Tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next`
- **Target Dir**: `/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1`
- **Command**: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-next/target-n1 CARGO_BUILD_JOBS=2 nice -n 10 cargo test -j 2 --all-targets --all-features --no-run`
- **Exit Code**: `0`
- **Raw Log**: `next-compile-20261011T135806Z.log`
- **Status Ledger**: `next-compile-20261011T135806Z-status.tsv`
- **Total Diagnostics**: 0 errors, 1 warning (1 duplicate across lib/lib-test)

## Crate Stage Analysis
- **Library Stage (`src/lib.rs`)**: **SUCCEEDED** (0 errors, 1 unused import warning).
- **Test Targets**: **ALL SUCCEEDED**. All test targets compiled and linked successfully into executables.

## Diagnostics by Owning Todo

### Todo 18
*Ownership: `src/lanes/posts.rs`, `src/lanes/discord_threads.rs`, `src/lanes/activity.rs`, `src/storage/lanes.rs`, `tests/test_lane_posts.rs`*

| Severity | Code | File:Line | Message | Suggested Fix / Context |
|---|---|---|---|---|
| warning | unused_imports | `src/lanes/posts.rs:9:49` | unused import: `DiscordThreadError` | Remove `DiscordThreadError` from imports in `src/lanes/posts.rs` |

### Todo 13
*Ownership: `src/lanes/runner.rs`, `src/lanes/cli.rs`, `src/lanes/state.rs`, `src/lanes/settle.rs`, `tests/test_lane_done.rs`, `tests/test_lane_runner.rs`, `tests/test_lane_cli.rs`*
- None (0 errors, 0 warnings)

### Todo 14
*Ownership: `src/lanes/dispatch.rs`, `src/models/*`, `src/multiplexer/*`, `src/storage/db.rs`, `src/main.rs`, `src/discord/adapter.rs`, `tests/test_lane_dispatch.rs`, InboundEvent literal sites*
- None (0 errors, 0 warnings)

### Shared
*Ownership: Any file not covered by Todo 13, 14, or 18*
- None (0 errors, 0 warnings)

## Raw Warning Excerpt
```
warning: unused import: `DiscordThreadError`
 --> src/lanes/posts.rs:9:49
  |
9 |     none_allowed_mentions, strip_status_prefix, DiscordThreadError, DiscordThreads, ThreadStatus,
  |                                                 ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `omo-gateway` (lib) generated 1 warning (run `cargo fix --lib -p omo-gateway` to apply 1 suggestion)
warning: `omo-gateway` (lib test) generated 1 warning (1 duplicate)
```

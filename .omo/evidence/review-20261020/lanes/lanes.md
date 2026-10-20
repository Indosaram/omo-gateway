# Lane lanes — fixes (worker hephaestus, st_01a1580c)

Scope written: src/lanes/runner.rs, src/lanes/cli.rs, src/lanes/dispatch.rs.

B13 FIXED src/lanes/runner.rs:139 — apply_reopen's BEGIN IMMEDIATE transaction now runs `UPDATE lane_pauses SET resumed_at = ? WHERE lane_key = ? AND resumed_at IS NULL`, so a pause that survived `lane close` no longer keeps a reopened lane skipped by the service loop and the stale check; test reopen_clears_a_pause_that_outlived_close
B14 FIXED src/lanes/cli.rs:560 — a failed register_owned_worktree on `lane start` no longer reroutes the lane into the shared repository root. The new block_lane_after_worktree_failure (cli.rs:611) removes only the freshly created, owned tree, moves the committed lane row to `failed` through state::transition (so no service loop picks it up and `lane reopen` still accepts the status), and enqueues a scrubbed `launch_failed` lane event carrying the registration error and, when cleanup also fails, the surviving path and branch. `launch_failed` is in dispatch.rs:214's `critical()` list, so the failure notice cannot be marked `jev_skip` by the wake judgment and reaches the owner's lead turn like `done_claimed`. The lane row keeps its isolated cwd; there is no `UPDATE lanes SET cwd` in production code at all. discard_auto_worktree (runner.rs:519) now returns Result instead of logging errors away, so every cleanup failure is observable: the block event names it, the thread-creation failure path returns it in the error, and the failed-insert path logs it (no row exists to carry an event); tests registration_failure_blocks_the_lane_in_its_own_tree, worktree_cleanup_failure_is_surfaced_not_swallowed, owner_visible_failure_events_skip_the_wake_judgment
B15 FIXED src/lanes/runner.rs:1163 — close_intent computes unmerged_lane_branch (runner.rs:531) after remove_owned and records one scrubbed `unmerged_branch` lane event inside the close transaction, carrying the branch and its commit gap; nothing is merged into any target. The comparison uses the lane's recorded start commit from lane_usage_context as the base (verified ancestor with `git merge-base --is-ancestor`) and falls back to HEAD only when no base is recorded or it is unreachable, so an unrelated or moved HEAD cannot inflate the count. src/lanes/dispatch.rs:214 adds `unmerged_branch` to `critical()`, so the event skips the wake judgment that can mark an event `jev_skip` and is admitted to the owner's lead turn like `done_claimed`; tests close_surfaces_the_unmerged_lane_branch, unmerged_branch_counts_only_work_beyond_the_recorded_base, owner_visible_failure_events_skip_the_wake_judgment

## Delivery path checked (B15)

`EventDispatcher::tick` groups undelivered lane_events by lead_session_key, `deliver_group` collapses per (lane_key, kind), judges only non-critical events, renders `[LANE_EVENT]` lines and routes them to the lead session, then writes a receipt. `unmerged_branch` and `launch_failed` are now critical, so neither is ever judged away; both are still subject to the ordinary batching gates (event at least 30s old, lead session idle), which delay delivery but never drop it. The service loop auto-closes `verified_done` lanes (runner.rs:1454), so the unmerged-branch event also lands right after settle.

## Notes / gaps

- No cargo build or test was run: the shared `target` lock was held by other lanes and the lead's integrated run owns compilation. My own cargo process had already exited; nothing of mine is running.
- Compile-relevant facts were checked by reading definitions in this session (register_owned_worktree, discard_auto_worktree, AutoWorktree, enqueue_lane_event, capture_lane_start, state::transition, LaneStatus::allowed_from, Lane, is_lane_paused, LaneEvent fields), and each edited hunk was re-read after editing. Compiler confirmation is still pending.
- Tests are `#[tokio::test]`/`#[test]` in the existing (or new, for dispatch.rs) `#[cfg(test)]` modules of the same files and exercise real temp git repositories: the B15 close test fails without the event insert, and the base test fails without the base comparison (the fallback path reports 2 commits where the base reports 1).
- Deliberate non-changes: B15 does not merge or push the lane branch (owner decision); the Slack close reply text was left alone because the lane event already reaches the owner through the dispatcher.
- Known B14 caveat: after a successful cleanup the blocked lane's cwd no longer exists, so `lane reopen` would stop at restore_cwd ("lane cwd is not a directory"). The lane stays diagnosable and closable (`close --force`), and the `launch_failed` event names the removed path and branch, but reopening requires the owner to start a fresh lane or register a worktree by hand. Deleting the tree was chosen over leaving an unregistered one on disk, which is the defect B14 reports; the alternative (keep the tree so reopen works) would restore that orphan.

## Gate round: compile and fmt corrections (integrated1)

From `.omo/evidence/review-20261019/gate/review-20261020-integrated1/clippy.log`, all errors in my files are now fixed:

- cli.rs:485, 630, 641 — invalid `tracing` field syntax `%error = %x` replaced with `error = %x` (the message keeps `error` as the field name and the value is still `Display`-formatted). No `%error = %` remains in src/lanes.
- cli.rs:893, 946, 952 — the test module's `super::runner` resolved to nothing (the `tests` module's parent is `cli`, whose `super` is `lanes`, but path resolution inside the test module looked one level lower); both `AutoWorktree` literals and the `discard_auto_worktree` call now use `crate::lanes::runner::…`. The non-test `tree: &super::runner::AutoWorktree` parameter at cli.rs:614 is unchanged and correct: it sits in the `cli` module where `super` is `lanes`.
- The cli.rs:952 `E0282 type annotations needed` was a knock-on of the unresolved path and is resolved by the same correction.

From `fmt.log`, the formatting diffs in my files are applied: `src/lanes/kibitzer.rs:168` (single-line `from_storage_key` call) and `src/lanes/runner.rs:2318, 2488, 2501` (the three test assertions/awaits). A narrow `rustfmt --edition 2021 --check` over src/lanes/{cli,runner,kibitzer,dispatch}.rs now exits 0; no whole-repo formatter run.

Errors left in that log belong to other scopes and were not touched: src/agent/omo_backend.rs:3568 (async test), src/agent/omo_backend.rs:2206 (E0502), src/autonomy/loop.rs:1184/1204/1253/1261/1268/1274 (E0308 missing `Some`).

## Restart verification (st_01a15881, 2026-10-20)

The replacement worker read the current implementation and existing regression tests. All three requested fixes already exist in the successor's changes, so no Rust source was rewritten. Existing edits and active QA workers were left untouched.

- Pause recovery: runner.rs:139 updates unresumed pause rows inside `apply_reopen`'s transaction; runner.rs:2299 tests reopening a closed, paused lane with `reopen_clears_a_pause_that_outlived_close`.
- Registration cleanup: cli.rs:567 calls `block_lane_after_worktree_failure`; cli.rs:617 invokes `discard_auto_worktree`, runner.rs:519 removes the fresh tree and branch, and cli.rs:629-644 records failed state and a `launch_failed` event. Tests are `registration_failure_blocks_the_lane_in_its_own_tree` (cli.rs:891) and `worktree_cleanup_failure_is_surfaced_not_swallowed` (cli.rs:943).
- Branch visibility: runner.rs:1163 calculates the retained branch notice and runner.rs:1186-1197 records it with close completion. dispatch.rs:220 treats this event as critical, and dispatch.rs:170-199 routes the notice to the lead session. runner.rs:1478-1482 schedules close for verified lanes. Tests are `close_surfaces_the_unmerged_lane_branch` (runner.rs:2327), `unmerged_branch_counts_only_work_beyond_the_recorded_base` (runner.rs:2408), and `owner_visible_failure_events_skip_the_wake_judgment` (dispatch.rs:243).

Static inspection only: this restart ran no build, test, formatter, live notification, deployment, publishing, or calendar action. Earlier gate results above are historical and were not rerun. Compilation, regression-test results, and actual notification delivery remain for the lead's integrated verification. The documented removed-cwd limitation after failed registration remains; there is no additional implementation blocker for the requested fixes.

The existing report was confirmed to contain exactly three ID status lines, one each for the requested IDs. This restart changes only this evidence report.

## Direct recovery inspection (2026-10-20)

The direct retry inspected the current source and preserved all successor fixes without rewriting Rust. No child was spawned. The only file changed by this retry is this requested evidence report.

- `src/lanes/runner.rs:139` clears outstanding pauses in the reopen transaction; `reopen_clears_a_pause_that_outlived_close` at line 2299 checks the stored pause state after reopening.
- `src/lanes/cli.rs:567` invokes the registration-failure handler, which calls cleanup at line 617, records failed state at line 629 and queues `launch_failed` at lines 632-643. `src/lanes/runner.rs:519-523` removes the fresh worktree and branch. The existing tests at cli.rs:891 and cli.rs:943 cover cleanup success and cleanup failure reporting.
- `src/lanes/runner.rs:1163` computes the branch notice, and lines 1183-1197 queue it in the close transaction. Lines 1479-1487 connect verified completion to close processing. `src/lanes/dispatch.rs:214-223` classifies the notice as critical, bypassing the optional wake judgment; lines 170-199 route the event to the lead session. Existing tests at runner.rs:2327, runner.rs:2408 and dispatch.rs:243 cover branch reporting, the recorded base and critical classification.

The report already existed and a read-only content count returned three ID status lines, one for each requested defect. No build, test, formatter, commit, push, deployment, live gateway operation or external action was run. Runtime correctness, compilation and actual owner notification delivery remain unverified in this retry. The previously documented removed-cwd limitation after registration cleanup remains; no additional implementation gap was found within the requested scope.

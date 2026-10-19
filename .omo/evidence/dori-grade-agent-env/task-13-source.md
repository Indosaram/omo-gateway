# Todo 13 source receipt

Implemented in `/Users/indo/code/project/omon-gateway-wt/dori-next` only. The main checkout and frozen trees were not edited; this receipt is the sole evidence-file write in dori-work. No git commands, commits, cargo build/check/test/clippy commands, or live Discord actions were executed.

## Files changed

- `src/lanes/cli.rs`: injectable `change_claim_at` timestamp, persisted deadline guidance in KST, and clearing the previous objection on a new claim. Existing generation-fenced transactional claim/object SQL remains intact.
- `src/lanes/runner.rs`: settlement clock/signal injection, 30-second settlement scheduling in a separate JoinSet so live turns and socket guidance stay responsive, named `verified_done_ready` close hook, and allowing claim guidance to steer an already-running turn. Existing backend JoinSet, CAPTURE_GENERATION, generation fences, turn cancellation, and connection pool configuration remain intact.
- `src/lanes/settle.rs`: new persisted-deadline settlement implementation, live signal checks, worktree selection and dirty/unpushed checks, transactional generation-fenced results, failure event and inbox guidance.
- `tests/test_lane_done.rs`: new scripted backend / MockClock acceptance tests.
- `src/lanes/mod.rs`: exactly one additive `pub mod settle;` line.
- `Cargo.toml`: exactly one additive test registration block.

No migration or changes to existing assertions were needed. `state.rs`, existing runner/CLI tests, and peer-owned files were not changed.

## Tests added (not executed)

1. `claim_then_object_queues_reasons_and_returns_not_done`
2. `passing_signals_and_clean_tree_verify_only_after_five_minutes`
3. `failing_signal_returns_not_done_with_live_detail`
4. `dirty_tracked_file_returns_uncommitted_reason`
5. `runner_restart_uses_persisted_deadline_and_settles_once`

The backend commits a real claim through the CLI storage path and emits a pre-established channel signal. Tests use bounded channel/task timeouts and MockClock advancement, with no fixed sleeps or polling. The git fixture operations are test source only; the tests have not been run.

## Restart and settlement behavior

`claim_at` remains the deadline authority. Claims younger than five minutes do not run signals. Each due claim reserves a `settlement_started` history row before invoking `SignalRunner::check_all` once. The reservation is scoped to the current claim history and reopen generation, without a new schema column. The result CAS requires the same claim timestamp, status, and reopen generation; objections/reopens during checking prevent stale results from applying. Successful settlement stops at `verified_done`; later scans do not run signals again.

On a crash after reservation but before result persistence, the restarted runner returns the claim to `not_done` with `settlement interrupted; claim again to recheck`, without replaying potentially side-effecting command signals. A fresh claim gets a new claim history boundary. Gateway shutdown aborts the settlement task and thus uses this recovery rule. A normal restart between claim and deadline resumes the persisted deadline and checks once.

Git checks use argv, bounded execution and kill-on-drop. They inspect the cwd plus worktrees under it or whose filesystem birth time is at/after lane creation. Tracked dirt produces `uncommitted <path>`; when remotes exist, local commits absent from all remotes produce `unpushed <path>`. Non-git directories have no worktree checks. Git/metadata check errors reject verification with their detail rather than declaring success.

## Cross-owner hooks

- Todo 15: attach closure scheduling at `LaneRunner::verified_done_ready(&[String])`. Startup recovery for previously persisted `verified_done` rows is owned by todo 15. No lane is closed here.
- Todo 35: insert the optional done-diff judge gate in `Settler::settle_due`, after signal/worktree reasons are collected and before computing `target` / committing the result.
- Todos 14 and 18: no additional owner-file changes requested.

## Verification and outstanding work

`rustfmt --edition 2021` on runner.rs, cli.rs, settle.rs, and test_lane_done.rs completed with exit code 0, including a second formatting pass after the settlement JoinSet change. Automatic tool-triggered LSP diagnostics timed out; no explicit compilation or cargo commands were run. Compilation, clippy, the five new tests, existing regression tests, and real-surface QA remain for the parent's batch gate. This receipt claims source completion, not passing runtime validation.

## Grouped pre-review fixes

Read `omon-gateway/.omo/evidence/dori-grade-agent-env-todo13-pre-review.md` in full. Changed only `src/lanes/settle.rs` and `tests/test_lane_done.rs` in the released source tree for this increment; updated this receipt. The review's suggested `modified()` fallback was explicitly rejected per the owner's instruction.

Worktree selection now short-circuits for cwd and descendants before polling the creation-metadata future. External paths preserve existing macOS/APFS behavior: use the path's filesystem birth time when available. If birth time is unsupported, resolve the worktree's actual Git administrative directory using `git rev-parse --absolute-git-dir`, read and verify its `gitdir` registration back-pointer against the external worktree's `.git`, then inspect that registration directory's birth time as creation evidence. Git's HEAD, index, gitdir, commondir and reflog modification timestamps are not treated as creation evidence. Git does not persist a general worktree-creation timestamp in those record contents. If a valid linked registration with birth time cannot establish creation, verification reports a path-specific `unsupported worktree creation-time check` rather than guessing, using mtime, or silently excluding the path. Missing external-path metadata also names the affected path.

Added deterministic tests (not run):

- `interrupted_settlement_recovers_without_replaying_signals`: reserves settlement_started, confirms recovery to not_done with zero signal calls and one failure event, then proves a fresh claim can settle once.
- `unpushed_commit_with_local_bare_remote_returns_not_done`: pushes the fixture baseline to a temp local bare remote, makes one tracked committed change without pushing, and asserts unpushed rather than uncommitted rejection.
- `settle::tests::cwd_and_descendants_do_not_query_creation_metadata`: a creation future panics if polled, proving short-circuit selection for cwd and descendant paths even when the lane start is unavailable.
- `settle::tests::external_selection_uses_creation_time_not_modification_time`: exact pre-start, boundary and post-start timestamps plus unsupported evidence propagation.

Scoped `rustfmt --edition 2021` on settle.rs and test_lane_done.rs completed with exit code 0. No Cargo commands or git commands were executed. Automatic edit-hook LSP requests were cancelled or timed out; compile and test validation remain with the parent's grouped gate.

source-settled

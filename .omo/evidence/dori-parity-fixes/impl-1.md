# Implementation Report: Todo 1 (dori-parity-fixes)

## Summary
Todo 1 of `/Users/indo/code/project/omon-gateway/wt/dori-parity/.omo/plans/dori-parity-fixes.md` ("Refuse commits that exist on no remote, in settle and in the close sweep") has been implemented in `/Users/indo/code/project/omon-gateway/wt/dori-parity`.

## Files Changed
1. `src/lanes/settle.rs`
   - Modified `worktree_reasons`: dropped the `git remote` empty guard. Added check for unborn HEAD (`git -C <path> rev-parse --verify -q HEAD` exiting 1 -> continue/nothing to push). For commits reachable from HEAD, counted unpushed commits via `git -C <path> rev-list --count HEAD --not --remotes`. When count > 0, returns `unpushed <path> (no remote)` if `git -C <path> remote` prints nothing, else `unpushed <path>`. All other git command failures return Err.
   - Updated test `prunable_worktree_registration_does_not_block_verification` with a bare remote `bare.git` and pushed fixture commit so that it does not fail on unpushed no-remote commits.

2. `src/lanes/runner.rs`
   - Modified `keep_reason`: dropped the `git remote` guard. Added unborn HEAD check (`git -C <path> rev-parse --verify -q HEAD` exiting 1 -> `Ok(None)`). When count > 0 from `rev-list --count HEAD --not --remotes`, returns `Ok(Some("unpushed"))` so the close sweep keeps the worktree even when no remote exists.
   - Added unit test `keep_reason_keeps_a_no_remote_worktree_with_commits` to `mod activity_generation_tests` asserting that a worktree with commits on no remote returns `Some("unpushed")`.

3. `tests/test_lane_done.rs`
   - Updated `Fixture` construction: `Fixture::new` now configures a bare remote `remote.git`, adds `origin`, and pushes `HEAD:refs/heads/fixture`. Added `Fixture::new_without_remote` (commits with no remote) and `Fixture::new_unborn` (unborn HEAD without commits or remote).
   - Updated existing test `unpushed_commit_with_local_bare_remote_returns_not_done` to use `Fixture::new` (already having `origin`) and asserted that the objection contains `unpushed <path>` without `(no remote)`.
   - Added new test `unpushed_commit_without_any_remote_returns_not_done` asserting that settlement returns `not_done` and the objection contains `(no remote)`.
   - Added new test `repo_without_commits_and_remote_is_not_refused` asserting that a repo with unborn HEAD and no remote is settled as `verified_done` without objection.

4. `tests/test_lane_close.rs`
   - Updated `Fixture::new` to initialize a bare remote `remote.git`, add `origin`, and push `main` so that worktrees are not kept as unpushed due to commits on no remote.

5. `tests/test_lane_worktree_auto.rs`
   - Updated `temp_repo` helper to initialize a bare remote `remote.git`, add `origin`, and push `main` so that clean orphan worktrees can be swept.

6. `tests/test_usage_done_gate.rs`
   - Verified: `settle_case` creates a repository without commits (unborn HEAD), which correctly bypasses the unpushed check via the unborn HEAD check without requiring modification.

7. `tests/test_fix_fix_settle.rs`
   - Verified: `other_branches_unpushed_commits_do_not_block_lane` at line 90 already has a bare remote and pushes to `origin`, requiring no modification.

## Exact Test Functions Added or Updated
- `src/lanes/settle.rs`:
  - `prunable_worktree_registration_does_not_block_verification` (updated)
- `src/lanes/runner.rs`:
  - `keep_reason_keeps_a_no_remote_worktree_with_commits` (added)
- `tests/test_lane_done.rs`:
  - `unpushed_commit_with_local_bare_remote_returns_not_done` (updated)
  - `unpushed_commit_without_any_remote_returns_not_done` (added)
  - `repo_without_commits_and_remote_is_not_refused` (added)
- `tests/test_lane_close.rs`:
  - `Fixture::new` (updated fixture setup)
- `tests/test_lane_worktree_auto.rs`:
  - `temp_repo` (updated fixture setup)

## Deviations from Plan
None. All implementations and tests strictly adhere to the plan specifications, scope boundaries, and owner rules.

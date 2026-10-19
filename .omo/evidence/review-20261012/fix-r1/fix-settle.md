# fix-settle report

Write scope: `src/lanes/settle.rs`, `tests/test_fix_fix_settle.rs` (new file, written by the earlier attempt and reused here as-is).
Logs: `fix-settle-red.log` (unfixed code), `fix-settle-green.log` (fixed code).
Note: the editor hook ran rustfmt automatically on `src/lanes/settle.rs` only. No other files were touched.

## Defect 1 [P1]: a per-lane check error stopped the whole settle loop
- Status: FIXED
- Change: `src/lanes/settle.rs`, `Settler::settle_due`
  - `remote_checks(...).await?` became a match. On `Err(error)` it now yields `Some(vec![format!("remote checks: {error}")])`, so the lane is rejected (`not_done`, with the error as the objection) and the loop moves on to the next lane.
  - `cron::remediate::settle_lane(&lane.key).await?` became a match. On `Err(error)` it pushes `format!("cron settle: {error}")` into `reasons` (the lane goes `not_done`). `Ok(None)` still falls back to `signals.check_all`.
  - `runner.rs` was not edited.
- Test: `tests/test_fix_fix_settle.rs::per_lane_check_error_rejects_lane_and_settle_continues`. It sets up two done_claimed lanes. One is external with an unparsable `external_target`, so `remote_checks` returns Err. The test asserts that `settle_due` returns Ok, the healthy lane is verified, and the broken lane is `not_done` with an objection containing "remote checks".
- Only the `remote_checks` error path has a deterministic test. The `settle_lane` error path needs an installed global `REMEDIATOR` (a OnceLock) whose job lookup fails. That cannot be set up in isolation without touching `cron/remediate.rs`, so that branch was checked by reading the code only. It is the same pattern as the tested branch.
- RED command: `cargo test --test test_fix_fix_settle` (unfixed settle.rs)
  ```
  test per_lane_check_error_rejects_lane_and_settle_continues ... FAILED
  panicked at tests/test_fix_fix_settle.rs:74:52:
  called `Result::unwrap()` on an `Err` value: Config("target must be <machine>:<session> without options or shell syntax")
  test result: FAILED. 0 passed; 2 failed
  ```
- GREEN command: `cargo test --test test_fix_fix_settle` (fixed settle.rs)
  ```
  test per_lane_check_error_rejects_lane_and_settle_continues ... ok
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
  ```

## Defect 2 [P2]: the unpushed check scanned every local branch
- Status: FIXED
- Change: `src/lanes/settle.rs`, `worktree_reasons`. Replaced `git log --branches --not --remotes --oneline` with `git log HEAD --not --remotes --oneline`.
- Test: `tests/test_fix_fix_settle.rs::other_branches_unpushed_commits_do_not_block_lane`. It builds a real repo with a bare remote. HEAD is pushed, and a separate branch `other` has a local-only commit. The test asserts the lane is verified. It then adds an unpushed commit on HEAD itself and asserts the lane is rejected with "unpushed".
- RED command: `cargo test --test test_fix_fix_settle` (unfixed)
  ```
  test other_branches_unpushed_commits_do_not_block_lane ... FAILED
  panicked at tests/test_fix_fix_settle.rs:104:5:
  assertion `left == right` failed
    left: []
   right: ["lane"]
  ```
- GREEN command: `cargo test --test test_fix_fix_settle` (fixed)
  ```
  test other_branches_unpushed_commits_do_not_block_lane ... ok
  test result: ok. 2 passed; 0 failed
  ```

## Out of scope (follow-up, not changed)
- `src/sessions/external.rs` `remote_checks_with` has a Python script that also uses `git log --branches --not --remotes` for remote lanes. Defect 2 exists there too. The needed change is to replace `'--branches'` with `'HEAD'` in that script. That file is outside this node's write scope (BLOCKED-SCOPE).

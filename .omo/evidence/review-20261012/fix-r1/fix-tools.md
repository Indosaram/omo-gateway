# fix-tools report (fix-r1)

RED/GREEN: deferred to single final integration gate

RED was captured. GREEN was not: the GREEN run (`cargo test --lib -- tools::terminal:: tools::browser:: tools::file::`) only ever printed `Blocking waiting for file lock on build directory` (see `fix-tools-green.log`) and was killed. Then the user's stop-all-builds order came in. No cargo processes from this node are still running.

Files changed: `src/tools/terminal.rs`, `src/tools/browser.rs`, `src/tools/file.rs`. All edits are inside WRITE SCOPE, and none were needed outside it. `tests/test_fix_fix_tools.rs` was not created because all the tests are unit tests in the existing `#[cfg(test)]` modules.
Snapshots of the fixed sources: `fix-tools-fixed-{terminal,browser,file}.rs`. The current working-tree files are identical to them. The `RED stub` is gone, checked with `grep -c "RED stub" src/tools/browser.rs` = 0.

## How RED was produced
RED used the HEAD (unfixed) production code with the new test modules appended. For browser.rs only, a no-op `ensure_public_final_url` stub was added so the new test would compile. After the run the fixed files were restored.
Command: `cargo test --lib -- backgrounded_grandchild_does_not_hold_tool_until_timeout navigate_closes_tab_whose_final_url_is_internal final_url_check_rejects_internal_and_ignores_non_http snapshot_is_scoped_to_the_calling_session search_skips_unreadable_entries_instead_of_aborting`
Result: `test result: FAILED. 0 passed; 5 failed; ... finished in 10.32s`. The full output is in `fix-tools-red.log`. The old `fix-tools-red.log` from an earlier attempt only showed an unrelated omo_daemon.rs compile error, and this run overwrote it.

## 1. [P1] terminal: backgrounded grandchild hangs the tool until timeout - FIXED (GREEN deferred)
- Changed: `TerminalTool` run/spawn path in `src/tools/terminal.rs`.
  - The child runs in its own process group (`process_group(0)` on unix).
  - A new `kill_group` closure sends `libc::killpg(pgid, SIGKILL)`.
  - `child.wait()` is now one branch of the reader `select!` loop.
  - After the child exits, the readers get `PIPE_DRAIN_GRACE` (500ms), then the group is killed so the inherited pipes close.
  - The group is also killed on pipe overflow, next to the existing `start_kill`. On timeout it is killed and the reader tasks are aborted.
  - Overflow semantics are unchanged.
- Test: `tools::terminal::approval_tests::backgrounded_grandchild_does_not_hold_tool_until_timeout`. It runs `sh -c 'echo hi; sleep 30 &'` with a 10s tool timeout and expects `success=true` and `stdout="hi\n"`. A timeout would make it fail, so no wall-clock sleep is needed in the test.
- RED excerpt: `must return once the direct child exits, not time out: ToolExecution("process timed out after 10s")`
- Note: `libc` must be a direct dependency in Cargo.toml for `libc::killpg`. The lead should confirm this in the final gate. Cargo.toml is outside my scope and I did not edit it.

## 2. [P1] browser: navigate only checks the initial URL (redirect / DNS rebinding) - FIXED (GREEN deferred)
- Changed: new `ensure_public_final_url(url)` in `src/tools/browser.rs`.
  - It parses http(s) URLs and reuses `ensure_public_destination`.
  - Non-http URLs such as `about:blank` are accepted.
- Navigate path: after `/json/new`, the tool reads `/json/list` and finds this tab's final `url`. If that URL is internal, it calls `/json/close/{id}`, returns the error, and does not record the tab as owned.
- Tests:
  - `tools::browser::tests::final_url_check_rejects_internal_and_ignores_non_http` (unit test of the check function)
  - `tools::browser::tests::navigate_closes_tab_whose_final_url_is_internal`. It uses a fake CDP that serves `/json/close/{id}` and records the closes.
- RED excerpts:
  - `assertion failed: ensure_public_final_url("http://127.0.0.1:9222/json").await.is_err()`
  - `final internal URL must be refused, got Ok(Object {"status": String("navigated"), ...})`
- Residual risk: if `/json/list` cannot be read after navigation, no final URL is found and the check is skipped. This is fail-open, matching the minimal-fix brief. A sub-resource or later in-page navigation is not re-checked.

## 3. [P2] browser: owned_tabs shared across all sessions - FIXED (GREEN deferred)
- Changed: `owned_tabs` is now `Arc<Mutex<HashMap<String, HashSet<String>>>>`, keyed by `SessionKey::storage_key()`.
  - New `execute_with_context` and `execute` both delegate to `BrowserTool::execute_inner(args, session)`.
  - Calls with no session context share the anonymous `""` bucket.
  - Snapshot filters to the caller's bucket, and navigate inserts into it.
- Test: `tools::browser::tests::snapshot_is_scoped_to_the_calling_session`
- RED excerpt: `leaked: {"active_pages_count":1,...}  left: Number(1) right: 0`

## 4. [P2] file: search aborts on one unreadable or vanished entry - FIXED (GREEN deferred)
- Changed: `search_files` in `src/tools/file.rs`.
  - `symlink_metadata` and `read_dir` errors are now skipped with `continue` for any path other than the start path. The start path still returns the error.
  - Failed directory entries are skipped by `entries.flatten()`.
- Test: `tools::file::tests::search_skips_unreadable_entries_instead_of_aborting` (unix).
  - It makes a 0o000 subdirectory, expects the search to return only `hit.txt`, and expects searching with that subdirectory as `path` to still error.
  - It restores the permissions afterwards.
- RED excerpt: `one unreadable directory must not abort the search: ToolExecution("Permission denied (os error 13)")`

## Lead final gate: what to run
- `cargo test --lib -- tools::terminal:: tools::browser:: tools::file::` (the 5 new tests plus the existing tests in those modules)
- rustfmt and clippy on the three files. The formatting was kept rustfmt-compatible by hand, but rustfmt has not been run on them.

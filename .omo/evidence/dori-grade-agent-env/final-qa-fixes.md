# Final QA fixes for binary 4196efab

Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Inputs: `final-qa-summary.md` and `task{15,28,29,31,32}-live-final.md` in this evidence directory; todo 27 and 29 in the main checkout's plan.

## Changes

- Headless startup now constructs the native cron scheduler/executor, installs the same remediator as Discord mode, binds the run-now socket before reporting readiness, and serves it in the lane JoinSet. Shutdown cancels that server, stops the scheduler, and joins services before closing SQLite. Discord wiring is not duplicated. Socket binding creates its parent directory and retains mode 0600 and existing-owner refusal.
- Session listings retain each machine's last successful result independently. Results younger than 15 seconds are reused without resetting their timestamp. On enumeration timeout, results at most 300 seconds old are returned with JSON `stale` entries containing `machine`, `stale: true`, and `age_seconds`. A timeout without usable data retains exit code 3; command and parser failures retain their prior taxonomy. Enumeration stays parallel with a maximum 20-second timeout. Production `sessions list` persists successful listings under `~/.omon/session-list-cache` so separate CLI processes can reuse them; fake runners remain isolated. Mutation commands do not enable this persistent listing cache.
- The previous done-diff path sent `git diff <lane_usage_context.start_commit>`, which covers committed changes since start but excludes untracked files. It also truncated title plus brief to 1500 characters. Settlement now passes title plus the complete brief to the Judge interface and constructs a private Git index from the lane start commit, adding current tracked and nonignored untracked files without modifying the lane index. It reads at most 256 KiB of diff output, presents that artifact through a temporary repository to the existing Judge diff interface, and cleans up the temporary directory. The shared Jev adapter's existing outbound scrubbing limits remain unchanged.
- Jev errors still record `done_diff skipped: jev unavailable` and do not reject an otherwise successful settlement. Missing start revisions, missing briefs, and unavailable diff inputs are recorded as skipped rather than blocking closure. Existing verdict thresholds remain unchanged.

## Regression coverage written, not executed

- `qa_gaps_headless_cron_socket_is_bound_and_removed_on_shutdown`: real bound Unix socket, successful connection, cancellation, joined service, removed path.
- Existing 15-second fresh-cache fake-runner coverage remains; added deterministic paused-clock cases for timeout with stale rows, timeout without data, and expired stale rows.
- Updated settlement fake judge captures the generated Git diff and asserts it contains the lane's untracked `done.txt`. Threshold and unavailable-judge cases use this same input fixture; no assertion pins brief prose.

## Validation

All implementation and tests were written before validation. Rustfmt on the five touched Rust files completed with exit code 0. No cargo test, live QA, release build, commit, push, environment edit, or model-selection change was performed.

One prescribed clippy invocation started through monitor `mon_0THCEXEY9KKMMZS6` (terminal `bash_4`):

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo clippy --offline -j4 --all-targets --all-features -- -D warnings
```

Clippy result: PASS, exit code 0. The monitor reported `Finished dev profile [unoptimized + debuginfo] target(s) in 45.10s`. No findings required changes and no duplicate batch was launched. Automatic LSP diagnostics timed out, so no clean LSP result is claimed. Tests and product behavior remain unexecuted under the requested validation scope.

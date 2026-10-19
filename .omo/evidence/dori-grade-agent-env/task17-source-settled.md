# Task 17 source evidence

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Status: SOURCE-SETTLED for the focused guard scope as of the 2026-10-11 verification follow-up below. Runner wiring remains an unapplied patch; integration and live QA are not completion claims. The earlier blocked verification history is preserved below.

## Inspection and implementation

Read the full canonical `.omo/plans/dori-grade-agent-env.md`, including global Must NOT have, task 17 Must NOT do, acceptance and live QA requirements. Existing task 6 evidence and source already implement host probes, hysteresis, CPU-independent launch decisions and heavy slots. `lane heavy` already calls `run_with_heavy_slot`; this work does not duplicate that implementation.

Changes are limited to `src/lanes/guard.rs`, the new `tests/test_lane_guard_alerts.rs`, and these task 17 evidence artifacts. Guard transitions enqueue scrubbed `guard` events and advance watcher state only after successful persistence. Queued promotion rechecks the authoritative active count while holding the SQLite write lock, ignores CPU as a launch hold, and commits working status, history and `launched` event together. Queued lanes with a parent channel reuse the existing thread helper before promotion.

`task17-runner-wiring.patch` is an **unapplied apply_patch-format patch** for the task 13 owner. It adds the 60-second cheap local host check, routes transition events to `OMON_LANE_DEFAULT_LEAD`, and retries queued lanes. The existing scheduler then picks up promoted working lanes. Runner and CLI were inspected but not edited. No task 16 kibitzer or task 18 post/status files were edited.

## Commands and results

- `rustfmt --edition 2021 src/lanes/guard.rs tests/test_lane_guard_alerts.rs && rustfmt --edition 2021 --check src/lanes/guard.rs tests/test_lane_guard_alerts.rs`: exit 0.
- Fresh LSP diagnostics on each owned Rust file: unavailable. Both returned `Timed out waiting for fresh diagnostics ... within 3000ms.` The empty diagnostics arrays are not proof of a clean LSP result.
- Exactly one focused invocation started: `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w17 cargo test -j2 --offline --test test_lane_guard_alerts`. Monitor `mon_J2TWXXMZ83XWHGPH`, terminal `bash_2`; watcher reported `timed_out (exit code 1)` during dependency compilation. No test result or project compiler verdict was reached. The monitor's default 300000 ms deadline was insufficient for this cold target directory. No second invocation was launched under the user's ONE-test constraint.
- After timeout, `ps -axo pid,command | /usr/bin/grep -E "[c]argo test.*test_lane_guard_alerts|[r]ustc.*target-w17"` exited 1 with no matches: no matching cargo or rustc process remains to attach to.
- `git diff --check`: exit 2 from existing unrelated `Cargo.toml:123: new blank line at EOF.` Left unchanged under file ownership.

Focused tests cover transition-only persistence, failed-event state preservation, low-disk recovery at high CPU, authoritative capacity recount and duplicate prevention, and rollback when a launch event cannot be written. They use scratch SQLite databases, no models or Discord, and no sleeps.

## SHA-256 at verification start

```text
7c1118cfc34e5a98a6ff8bb7b40feab793a0c6e2808d71d889e17830125675ba  src/lanes/guard.rs
f2df1e836a22ef74d7ecc98091e118c2b50270afc04c85a16574c6d875f8cf56  tests/test_lane_guard_alerts.rs
34db725227552f0d4f80a886589cc1dfd7f3e64ea434557e073a3129fb2ff455  .omo/evidence/dori-grade-agent-env/task17-runner-wiring.patch
d55af2c5e94a1580396380baab9119ba90cf7a721a29b69b60d8c4357ca7c615  src/lanes/runner.rs (context only, owned by task 13)
3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510  src/lanes/cli.rs (context only, owned by task 13)
```

## Pending integration and live QA

Task 13 owner must apply/reconcile the runner patch; its scheduling is not compiled or exercised in this checkout. Fresh LSP diagnostics and the focused test timeout are concrete verification blockers. The source is not declared source-settled, and LSP cleanliness or passing tests are not claimed. Work stops at this concrete blocker as requested. A later authorized verification attempt needs a longer command deadline and must reconcile the runner patch first if it intends to exercise scheduling.

Pending authorized later QA: force disk Hold and observe `lane start` exit 4; restart without the override and observe queued -> working within 2 minutes; verify real guard transition delivery to the configured lead; run two concurrent heavy commands and check serialization. Thread creation for queued lanes with parent channels also remains live-unverified. No services, models, environment files, Discord actions, full suite or git commits were run for this task.

## Authorized focused verification follow-up - 2026-10-11

The earlier 300000 ms harness timeout above remains part of the history; it was not a compiler or test verdict. This follow-up resolves the focused-test blocker only, not the pending runner integration or live QA.

- Ran exactly one additional invocation: `cd /Users/indo/code/project/omon-gateway-wt/dori-work && CARGO_TARGET_DIR=$PWD/target-w17 cargo test -j2 --offline --test test_lane_guard_alerts`.
- Monitor deadline: `timeout_ms: 1800000` (30 minutes). Monitor: `mon_ASV5TZAS4PJN9457`; terminal: `bash_1`.
- Cargo completed compilation in `10m 01s` and the monitor completed with exit code `0`. No compiler errors appeared, including no `triage.rs` errors. No file-change wait, retry, or source fix was needed.
- Actual test result: `ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s`.

Passing tests:

```text
failed_event_write_preserves_watcher_state
launch_event_failure_rolls_back_lane_and_history
events_are_emitted_once_per_transition
queued_lane_launches_when_guard_recovers_even_at_high_cpu
retries_recount_active_lanes_and_do_not_duplicate_launches
```

Verdict: focused task 17 guard verification PASS, including the scratch SQLite persistence, recovery, capacity recount, duplicate prevention, and rollback checks. No Rust source files were edited by this follow-up. `runner.rs` and `cli.rs` were not edited, and `task17-runner-wiring.patch` remains unapplied by this worker. No services, models, environment changes, git commands, full-suite runs, or live Discord actions were performed. Fresh LSP diagnostics were not rerun; prior diagnostic limitations and pending integration/live QA above remain unverified.

### Actual verification confirmation and current source hashes

The user independently confirmed that the focused Cargo process (PID `56524`) finished and built `target-w17/debug/deps/test_lane_guard_alerts-d573b3cd281607c3`. The user executed that binary once from the worktree root and reported all five tests listed above passing: `5 passed; 0 failed`, exit `0`. This confirmation is user-reported evidence, separate from the successful Cargo monitor result observed above. Cargo was not rerun after this confirmation.

Current SHA-256 values, computed with `shasum -a 256 src/lanes/guard.rs tests/test_lane_guard_alerts.rs` (exit `0`):

```text
7c1118cfc34e5a98a6ff8bb7b40feab793a0c6e2808d71d889e17830125675ba  src/lanes/guard.rs
f2df1e836a22ef74d7ecc98091e118c2b50270afc04c85a16574c6d875f8cf56  tests/test_lane_guard_alerts.rs
```

Both hashes match the original verification-start values. The focused guard source is now SOURCE-SETTLED. This status does not include scheduling integration: runner wiring stays in the unapplied `task17-runner-wiring.patch`, and the prior live-QA and LSP limitations remain recorded.

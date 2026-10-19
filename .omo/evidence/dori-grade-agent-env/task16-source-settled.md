# Task 16 source settlement

Status: owned source settled. The single focused Cargo run completed with exit
code 0: 9 passed, 0 failed. Final-source LSP diagnostics are clean. Shared
producer/startup integration and live QA below remain parent-owned and unverified.
Run receipt: monitor `mon_Z37QSTTRBPT7SAV8` / session `bash_2`.

## Owned changes

- `src/lanes/kibitzer.rs`: durable arm generation, atomic admission spacing,
  KST hours/day cap, existing Jev noul gate, isolated small-model session and
  capture, strict decision parsing, bounded Unicode state document, enqueue-only
  wake decision. Task-owned schema/INSERT triggers are installed by explicit
  `initialize()` before producers; no shared migration file was modified.
- `tests/test_lane_kibitzer.rs`: nine focused tests using real scratch SQLite,
  explicit MockClock, fake judge and backend, and no model/network invocation.
- `src/lanes/mod.rs`: one additive module registration. Cargo auto-discovers
  the new integration target; no Cargo registration edit was necessary.
- `task16-runner-wiring.patch`: exact unapplied optional Notify fan-out hunks.
  Neither `runner.rs` nor `cli.rs` was edited by this task.

## SHA256 snapshot

```text
dce7ad02a453e5f074d0af00ed70acefa81baac7feaf8ac842ee7fea203e0f71  src/lanes/kibitzer.rs
b8dfda38b2de420cf263877a8aa418e28fd89c2a2ff12a211f1e93069261eba8  tests/test_lane_kibitzer.rs
aeea8a41c1b6e3ad0323135015ab3152ae6f4eb985131b0cd58413ce41e536dd  src/lanes/mod.rs
3795eec627fc8f3731384a821a0a13c9c166c80a8c03a2449a650c5a448d6edc  .omo/evidence/dori-grade-agent-env/task16-runner-wiring.patch
```

## Actual commands and results

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

1. Initial `rustfmt --edition 2021 --check src/lanes/kibitzer.rs` reported
   formatting differences. Formatter stdout was applied through apply_patch.
2. Final scoped checks completed successfully with no output:

   ```sh
   rustfmt --edition 2021 --check src/lanes/kibitzer.rs tests/test_lane_kibitzer.rs
   rustfmt --edition 2021 --config skip_children=true --check src/lanes/mod.rs
   git diff --check -- src/lanes/mod.rs
   ```

3. Initial LSP on the first module draft returned no diagnostics. Subsequent
   fresh diagnostics for the final module and test repeatedly timed out:
   `LSP daemon request timed out after 30000ms: the daemon did not respond in time.`
   Socket: `/Users/indo/.omo/lsp-daemon/v0.1.0/daemon.sock`.
   After Cargo compilation completed, final-source `lsp_diagnostics` with
   severity `all` returned `No diagnostics found` separately for
   `src/lanes/kibitzer.rs` and `tests/test_lane_kibitzer.rs` (0 diagnostics each).
   The earlier timeout was transient and is not the final result.
4. Exactly one focused Cargo invocation was started:

   ```sh
   CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w16 cargo test --test test_lane_kibitzer -j2 --offline
   ```

   Actual completion receipt:

   ```text
   Finished `test` profile [unoptimized + debuginfo] target(s) in 15m 35s
   running 9 tests
   test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.98s
   watcher completed (exit code 0)
   ```

   The command was not repeated. A host restart before this launch did not
   duplicate it. No source edits followed the passing run.
5. `shasum -a 256` produced the snapshot above.

No full suite, release build, services, models, Discord direct QA, env edits,
or git commits were run.

## Remaining shared integration and live QA

The owned module alone is not a deployed job. The parent/shared owners must:

1. Construct Kibitzer and Notify before producers, call initialize, apply the
   runner fan-out patch, run serve, and join its shutdown lifecycle.
2. Ordinary owner-message, active-turn steer, and intercepted lane reply paths
   must qualify actual owner/User origin and bot identity plus newly inserted
   row count, call arm_owner in the same transaction, and wake after commit.
   System/assistant/replay/non-owner inserts must not arm. These files are not
   owned by this task and were not edited.
3. Inbox, activity, settlement, generic enqueue and cron health owners must hint
   runner::wake after qualifying committed inserts. SQL triggers cover durable
   arming, not live notifications. CLI Object's existing post-commit wake is
   retained. Datagram hints remain best-effort; startup recovers durable arms.
4. Run deterministic transport/producer checks after shared wiring, then the
   plan's scratch-DB live inbox scenario and idle no-new-input observation.
   Verify one isolated fresh small-model turn, parsed decision, queued event,
   and dispatcher delivery without direct Discord UI QA.

Passing tests prove the module boundary only, not these unapplied producer or
startup hooks. Task 16 must not be called live-complete on this snapshot.

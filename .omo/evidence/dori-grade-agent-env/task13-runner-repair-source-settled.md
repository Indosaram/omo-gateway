# Task 13 runner repair source settlement

Status: repair source settled; final strengthened targeted check passed exit 0. Not final Task13 confirmation. Parent strict gate follows this source-settled handoff. Owner edits are limited to src/lanes/runner.rs and tests/test_lane_done.rs; CLI untouched. No full suite, standalone build, models, services, env, git or Discord actions.

## Repair boundary

serve captures its entire running loop in an async Result block (service_result), then sets first_error = service_result.err(), cancels shutdown, aborts and explicitly drains settlements, and cooperatively drains lane tasks. All six reviewer-named exits inside the block (list, generation, posts drain, socket receive, lane JoinError, fail persistence) therefore resolve to this same cleanup; no postspawn ? returns from serve itself. Subsequent join errors do not replace the initiating error. Startup WakeSocket::bind, posts::recover and initial posts::drain remain before either JoinSet is constructed or any task spawned; direct startup errors drop the owned socket safely and have no spawned task to cancel/join. run_lane and generation/claim CAS behavior unchanged.

## Covered and uncovered injection

service_storage_errors_cancel_and_join_backend_and_checker subscribes before serve; holds a due checker and separate active backend; injects working-list turns_run decoding, generation history-ID decoding, malformed pending Post decoding, fail persistence trigger, and lane-task panic via the report judge. Each awaits backend cancellation and checker teardown, proves serve remains unfinished while backend completion is gated, releases completion, awaits serve, asserts initiating fault, and verifies held lane turns_run persistence. Generation fixture installs its bad-ID view atomically before inserting the newly eligible lane, avoiding launch-before-fault races; it restores scratch schema before releasing held persistence. Returned generation error must be OmonError::Database with column0 decoding context and BLOB/i64 mismatch, not arbitrary Err. No sleeps/polling. Expected injected panic output is caught by real Tokio JoinSets, not an unhandled test failure.

Socket receive has source-level, not injected runtime proof: socket.socket.recv result.map_err(io_error)? is lexically inside the identical service_result block; its Err exits that block, initializes first_error and executes the same cancellation/drain code. WakeSocket and descriptor are private and owned inside serve, with no safe external method to make recv return Err. Unlinking its pathname does not break an open Unix datagram descriptor. Closing/replacing another task's owned FD is unsafe ownership interference; a configurable socket factory/fault hook would add production scope solely for injection. No production seam added. Startup recovery errors are pretask by construction; startup injection not claimed.

## Actual attempt ledger

Common cwd /Users/indo/code/project/omon-gateway-wt/dori-work; target dir target-w4.

1. Authoritative original mon_JBPXZ7GT5HKSJE08/bash_1: rustfmt --edition2021 --check --config skip_children=true src/lanes/runner.rs tests/test_lane_done.rs && CARGO_TARGET_DIR=<cwd>/target-w4 CARGO_BUILD_JOBS=2 cargo test --offline -j2 --all-features --test test_lane_done --test test_lane_runner --test test_lane_reopen --no-fail-fast -- --nocapture. Exit101. Done10 pass/1 new regression fail on cancellation timeout; reopen5 pass; runner12 pass/1 existing live QA ignore. Compile1m27s. Never relaunched this gate.
2. mon_1YP7CXSARHTX4ZVJ/bash_2: only cargo test --offline -j2 --all-features --test test_lane_done service_storage_errors_cancel_and_join_backend_and_checker -- --nocapture, CARGO_TARGET_DIR=<cwd>/target-w4. Exit101, generation fixture race, cancellation timeout; first list fixture repaired before run. Compile15.37s.
3. mon_5S6BHQRHGBFQKT9R/bash_3: scoped rustfmt then same single-test command. Exit101. All generation cancellation/join assertions reached, but expected substring decode did not match SQLx decoding column0. Compile32.01s; runtime2.16s. Atomic fixture setup fixed previous race.
4. mon_EN82AD22915J3KC6/bash_4: scoped rustfmt then same single-test command. Exit0,1 passed/10 filtered,4.08s, compile1m25s. Two expected injected panic logs. This verifies corrected decoding assertion but predates the added Database/BLOB/i64 identity match, so not attributed to the newest test hash.
5. mon_XY3SBZY1PWKERQA4/bash_5: scoped rustfmt then same single-test command, strengthened assertion. Exit 1 at rustfmt layout check; Cargo/test never started. Corrected only assertion wrapping through formatter stdout and apply_patch.
6. mon_ZJK8GBXSVT9Q72CG/bash_6: scoped rustfmt then same single-test command on formatted strengthened assertion. Exit 0; 1 passed, 0 failed, 10 filtered out; test runtime 6.63s, test-profile compilation 2m 22s. Expected injected lane-turn/lane-task panic logs at test lines 592/611 were caught and all five error-class iterations passed, including Database/BLOB/i64 identity, cancellation delivery, checker teardown and joined persistence before return. This is the final targeted evidence; no original multi-target gate relaunch.

LSP requested initially and retried as instructed on runner/test: both repeatedly returned freshness_timeout after3000ms. Empty stale diagnostic arrays are NOT a clean verdict. Rust compiler evidence above is real; no compiler errors observed. Scoped rustfmt passed before each run that reached Cargo.

## Final source identities

- src/lanes/runner.rs: d55af2c5e94a1580396380baab9119ba90cf7a721a29b69b60d8c4357ca7c615
- tests/test_lane_done.rs: c696d24721e25030108937acf3551ce335445e416f19d9ade05775f8ae1abda9
- src/lanes/settle.rs: 8bd0c6362fcae5d77daf9c3869558afa410c0a92d6e459af8dd42932e73972c5
- src/agent/omo_backend.rs: 04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2
- src/multiplexer/actor.rs: 1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3
- src/main.rs: c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a
- .omo/evidence/dori-grade-agent-env/task13-actual-d162f780-pass.md: 533ba5cf59b0cac59ba72e8e562ab6882636a690cf1fa53d0216702bd1f0733c

Live happy302.097042s/failure315.396277s receipt task13-actual-d162f780-pass.md preserved byte-for-byte; helper0/gateway0/PID29306/socket absent attributed to that prior binaryd162f780 receipt, not rerun/new-source proof. Its binary and earlier broad strict/build gates cannot automatically certify this repair. Claim-ID CAS settler8bd0, backend04b8, actor1d3b, mainc7af unchanged. Seven parallel workers own other todos; runner/CLI remain exclusively owned until this handoff.

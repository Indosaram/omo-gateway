# Todo 8 verification ledger

Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Initial focused run: `CARGO_TARGET_DIR=target-w4 CARGO_BUILD_JOBS=2 cargo test --test test_lane_runner --test test_lane_cli` exited 0: 2 CLI tests and 6 runner tests passed. This predates final edits and is not final-source evidence.

Expanded focused run: same command exited 0: 2 CLI tests and 11 runner tests passed. This also predates final edits and is not final-source evidence.

Strict gate: `CARGO_TARGET_DIR=target-w4 CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings` printed `Finished dev profile ... in 4m 53s`; the chained scoped-test compile then exceeded the monitor deadline. Monitor `bash_6` returned timed_out, exit 1; no surviving target-w4 compiler was found. The interrupted scoped command was restarted alone with an explicit 1200000 ms monitor deadline (`bash_8`). No full suite was launched.

Formatting: explicit `rustfmt --edition 2021 --check --config skip_children=true` over runner.rs, cli.rs, lanes/mod.rs, entry.rs, main.rs and both new tests exited 0 after the final source edits. `cargo fmt --check` exited 1 only for the pre-existing extra blank line at src/agent/backend.rs:61. That owner/sibling file was not edited.

LSP: directory diagnostics on src/lanes reported zero errors; subsequent individual checks frequently timed out against the LSP daemon. Compilation remains the decisive type check.

Real QA preflight: required endpoint 19742 had no listener and failed readiness. Full debug gateway, with dashboard disabled and no Discord credentials, exited 1 with `missing required environment variable DISCORD_BOT_TOKEN`. A QA-owned engine at 29748 returned HTTP 200 `ok` from /readyz. This is a disposable fallback, not evidence of the named live endpoint or full Discord gateway surface.

Actual CLI start on the scratch DB exited 0: qa-1, status working, turns_run 0, cwd /private/tmp/omon-task8-z4iBTA/qa-repo, model inferhub/cb/deepseek-v4.1-flash. This is launch evidence only; turn/file/report/claim evidence and cleanup remain pending.

Invalid-signal real CLI receipt is in task-8-fail.txt.

## Final-source results

The resumed scoped run (`bash_8`) exited 0: 3 CLI tests and 11 runner tests passed. The explicit live harness was ignored only in the ordinary run, then executed separately (`bash_9`) against the real disposable app-server; it exited 0 after SIGTERM and printed `TODO8_QA_RUNNER_CLEAN_EXIT`.

The exact final-source strict clippy command (`bash_12`) exited 0 in 1m 17s. No source edits followed the final scoped run. Source SHA-256 values and all nine adversarial classes are in task-8-doneclaim.json.

Real scratch QA completed one turn, emitted the milestone, created `done.txt` containing `qa complete`, and persisted `done_claimed`, evidence `done.txt created`, thread `01a129c1-f616-7b69-81d1-39fd8efa683e`, and outcome `completed`. Full CLI transcript and SQLite snapshot are in task-8.json. Actual CLI forced close exited 0 and a subsequent CLI list read back `closed`, turns_run 1. Runner PID 1475 and engine PIDs 81208/81211 were absent; port 29748 had no listener. Scratch directory, wake socket and lock were removed and verified absent.

## Operational incident: incorrect early database target

The earlier shell-prefix environment assignment in Bun.$ did not override inherited DATABASE_URL=sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db. It reported qa-1 creation at 06:52:53.346587Z while direct scratch inspection found zero lanes. This must be treated as an unintended live DB write, including Database::connect migration application. No live cleanup was attempted. A later read-only system SQLite probe failed with `unable to open database file (14)`, so the live row was not independently re-read. The coordinator must inspect the exact qa-1 row and migrations and authorize cleanup.

Final scratch runs used Bun.spawn with explicit env and confirmed the same scratch DB: qa-1 created at 06:58:59.428846Z, actual turn and claim completed, then forced closed. The earlier statement that no live DB writes occurred was incorrect and is superseded by this incident record. The exact named live/full-gateway QA remains unmet; successful disposable QA is not mislabeled as that surface.

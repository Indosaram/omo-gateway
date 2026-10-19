# Task 13 applied source settlement

Applied 2026-10-11 after parent task14 bridge cleanup release. Not final confirmation or runtime/deploy approval. Corrected artifact c7f9b1cc4b7b19aa5a9df9d6979e485009103a24d7470ebcf7062ea70aeb2179 applied to current dori-work with zero fuzz; only six product paths changed.

## Current base and preserved scope

205-entry base manifest a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d. Runner5/CLI3/mod1/Cargo1 unique contexts matched, additions absent. Main/omo_backend/actor exact byte hashes matched the base; no hunks touched them. No-token sink initialization and all run_lane/CAPTURE_GENERATION logic retained. No owner WIP, environment, model, service, Discord or git operation changed. Existing test fixture setup uses only owned synthetic temporary repositories/local bare remotes; it is not permission to commit/push owner work.

## Applied files and frozen gate-input SHA-256

- src/lanes/runner.rs: 0838a05d91938a7113e179b62b8716aa3a744473ed494cffd933231875f9b71c
- src/lanes/cli.rs: 3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510
- src/lanes/mod.rs: 8ddbf9e21666cfe4146e066f678ed749fb7b6aa5a657f3a32dffb9539b17f2f3
- Cargo.toml: 1d63dce8c69395ae113f923446f850cad5822978e34a1601b7145fe4f9a26563
- src/lanes/settle.rs: 8bd0c6362fcae5d77daf9c3869558afa410c0a92d6e459af8dd42932e73972c5
- tests/test_lane_done.rs: e4de3dcfcaa5458aeb6b4d80e4eaf31c5bf2a55e28cd6be376d35714b8d22a71

Only formatter-reported changes after application: runner match-arm layout, authoritative Lane query layout, and done-test line wrapping, applied through apply_patch. Original artifact remains preserved and is not rewritten to impersonate formatted source.

## Two gate findings addressed

Settler reloads authoritative done_claimed Lane under its SQLite write lock; reads latest persisted claim history ID and reopen generation in the same reservation transaction. Final result CAS includes latest claim ID plus status/time/reopen. Dependent history/event/inbox writes only follow an affected row. Same timestamp does not imply same claim. Existing history AUTOINCREMENT identity from migration0031 suffices; migration0033 supplies inbox. No new migration/timestamp uniqueness.

Settlement errors retain first_error and break to shared cancellation; settlement tasks abort and explicitly drain; lane tasks cooperatively drain; return original error only after joins. Same-timestamp success/failure and joined error/normal shutdown deterministic regressions were added along with event-scrub regression; MockClock and pre-subscribed channel/semaphore boundaries, no sleeps. Generated not_done event scrub preserves local diagnostic detail and exact owner text. Canonical unpushed fixture repair retained.

## Actual checks so far

Initial scoped rustfmt check exit1 found only applied-unit layout differences. Formatter stdout produced minimal apply_patch edits; repeated scoped rustfmt --edition2021 --check --config skip_children=true over runner/cli/mod/settle/test_lane_done exit0 (bash_2). LSP all-severity checks before and after formatting: runner, cli, settle and test_lane_done each zero diagnostics.

Focused gate completed exit0: cd /Users/indo/code/project/omon-gateway-wt/dori-work && CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4 CARGO_BUILD_JOBS=2 cargo test --offline -j2 --all-features --test test_lane_done --test test_lane_runner --test test_lane_reopen --no-fail-fast -- --nocapture. Monitor mon_7BD7ARGDM8S190X0/session bash_3, explicit1800000ms deadline. Test-profile compilation finished in34.52s. One run, no compiler or behavioral correction/retry needed.

Actual results: test_lane_done10 passed/0 failed (0.80s), including same_timestamp_reclaim_rejects_old_success_and_failure_results and settlement_error_and_normal_stop_join_active_backend_and_checker; test_lane_reopen5 passed/0 failed (0.14s); test_lane_runner12 passed/0 failed/1 pre-existing ignored (0.29s). The ignored live_qa_runner_process requires a disposable app-server and explicit QA database; no new ignore/skip was added. Runner startup child-process checks also returned exit-success one-test results. Total main targets27 passed/0 failed/1 ignored. Watcher completion reported exit0.

Exact-byte comparison during frozen gate confirmed all six applied hashes above unchanged, and protected main c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a, omo_backend04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2 and actor1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3 unchanged. Source remains frozen at the listed identities. Same-timestamp and joined-shutdown regressions now have actual passing evidence, beyond the earlier source-only gate correction.

Disposition: applied source settled for parent handoff, NOT final task13 confirmation. No full Cargo suite, standalone/release build, live scheduling/restart QA, strict clippy or fresh-versus-existing external scratch migration exercise was run. Temporary fixture SQLite execution passed through the focused tests; source schema inspection establishes no additional migration is required, not live owner-DB proof. No owner DB, model, environment, service, Discord, commit or push action occurred. Earlier gate report remains historical; it is not rewritten as a new independent review verdict.

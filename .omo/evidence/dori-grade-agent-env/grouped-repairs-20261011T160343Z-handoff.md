# Complete grouped results - build withheld

One sequential batch PID63734, monitor mon_XAP3CEXXFNPJ4RBZ, existing target-w4, 1800000 ms subscription. All requested check results collected before any fixes. No product edits during checks. Individual command exits authoritative; collection harness completed exit0.

Results:
- cargo fmt --check: exit0.
- cargo clippy -j2 --all-targets --all-features -- -D warnings: exit0, no diagnostics.
- cargo test -j2 --all-features --lib creation_metadata --no-fail-fast -- --nocapture: exit0, 1 passed, 1 ignored subprocess helper invoked by passing parent, 481 filtered.
- cargo test -j2 --all-features --bin omo-gateway session_cli_output_preserves_aggregate_exit_and_projection --no-fail-fast -- --nocapture: exit0, 1 passed.
- cargo test -j2 --all-features --bin omo-gateway tests::cli_ --no-fail-fast -- --nocapture: exit0, 6 passed.
- cargo test -j2 --all-features --test test_lane_runner --test test_lane_reopen --test test_lane_events --test test_session_registry --no-fail-fast -- --nocapture: exit101. Events25 passed/1 ignored, reopen5 passed, registry7 passed; runner11 passed/1 failed/1 ignored.

Complete unique failure: startup_discord_flag_controls_token_requirement at tests/test_lane_runner.rs:485, disabled child reached runner.serve(stop) with SQLite memory pool and panicked Config("lane service requires a file-backed database"). Parent at :533 reports child exit101. The test correctly passed no-token runner construction, then exercised service with an invalid fixture for existing WakeSocket contract. This is a regression-fixture mistake introduced by this startup track, not evidence of missing-token product failure. Correct next change is owned file-backed scratch SQLite fixture, not relaxing the service contract. No fix or rerun made in this batch.

Frozen191 product inputs pre/post identical (cmp0). Prior task-7 and repair/race receipt hashes preserved (cmp0). TMPDIR fixture root empty and rmdir0. Raw command outputs, exact argv/statuses, manifests and cleanup receipt under this prefix. No build or fresh prepare executed because checks failed; no new source-linked binary claimed. Existing debug binary may have been produced incidentally by Cargo tests and is not a release artifact for this batch. No full suite/inbox/shell/state replay, runtime/service/backend probe, external DB, git or Discord action. Consumed old QA root remains retained.

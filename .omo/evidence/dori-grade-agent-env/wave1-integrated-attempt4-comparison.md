# Attempt 4 failure and source comparison

## Baseline names

The baseline failure list is `/Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt`. It contains 16 failures over six targets. Attempt 4's full raw run reports the same 16 test names, over the same six targets. This is identity-by-name comparison only; it does not establish that every failure has the same cause.

Baseline and Attempt 4 names:

- `test_cron_boundary_parity::sync_isolates_bad_jobs_and_rearms_changed_once`
- `test_cron_executor_parity::agent_result_uses_scheduler_delivery`
- `test_discord_adapter::final_output_filters_controls`
- `test_migrate::cutover_second_store_failure_preserves_bytes_and_receipt`
- `test_migrate::cutover_survives_startup_sync`
- `test_omo_backend::test_omo_backend_accepts_received_completion_at_deadline_edge`
- `test_omo_backend::test_omo_backend_emits_hermes_activity_lines`
- `test_omo_backend::test_omo_backend_omits_workspace_when_per_agent_disabled`
- `test_omo_backend::test_omo_backend_provisions_agent_workspace_and_passes_to_thread_start`
- `test_omo_backend::test_omo_backend_waits_for_final_message_after_tool_activity`
- `test_omo_backend::test_omo_backend_requires_turn_terminal_even_after_final_agent_message`
- `test_omo_backend::test_omo_backend_aborts_turn_on_repeated_approval_denials`
- `test_omo_backend::test_omo_backend_deadline_interrupts_looping_turn`
- `test_omo_backend::test_omo_backend_e2e_thread_lifecycle_and_streaming`
- `test_omo_backend::test_omo_backend_retries_once_after_connection_drop`
- `test_wiring_e2e::test_e2e_actor_omo_backend_persists_assistant_message`

The cron-boundary panic detail is identical in the baseline and Attempt 4 raw logs: `tests/test_cron_boundary_parity.rs:538`, `valid-daily must be due at 09:00`.

The wiring test has a different panic despite the same failing test name. Baseline raw log: `tests/test_wiring_e2e.rs:304`, `assertion failed: final_chunk.is_final`. Attempt 4 raw log: `tests/test_wiring_e2e.rs:302`, `Must emit stream chunks to dispatcher`. Record this as same-name baseline failure with changed panic detail; it is not classified as new by name, and causal equivalence is not established.

The baseline full-run log and Attempt 4 raw log contain the same 16 failure names. Attempt 4's definitive target summary is 36 passed, 10 failed, 1 ignored for `test_omo_backend`, after 1801.15 seconds; those ten failure names are the ten baseline backend failures.

## Exact source identity for this run

The exact manifest command in both `wave1-integrated-attempt4-pre-manifest.sha256` and `wave1-integrated-attempt4-post-manifest.sha256` is:

```sh
find src tests migrations -type f -print0 | sort -z | xargs -0 shasum -a 256
```

Each manifest contains 145 sorted per-file hashes. The paths and hash lines compare byte-for-byte equal, so there was no drift among the included source, test, and migration files during Attempt 4. Cargo.toml and Cargo.lock hashes, compiler version, target directory, jobs, and backend executable hash are recorded in `wave1-integrated-attempt4-build-provenance.md`.

The workspace still has the eight pre-existing tracked Wave 1 changes relative to HEAD: `Cargo.toml`, `src/agent/mod.rs`, `src/agent/omo_backend.rs`, `src/agent/omo_config.rs`, `src/lib.rs`, `src/multiplexer/actor.rs`, `src/storage/mod.rs`, and `tests/test_omo_backend.rs`. These were present before this run. Existing untracked tests, migrations, source, evidence, and build outputs were preserved.

## Attempt 4 verdict

The full Cargo command completed with exit 101; it is not a passing suite. `cargo clippy --all-targets --all-features -- -D warnings` passed, the dashboard target passed 59/59, and `cargo fmt --check` retained the existing diff at `src/agent/backend.rs:61`. The raw log, actual Cargo exit sentinel, and all six failed target summaries are in `wave1-integrated-attempt4-raw.log`.

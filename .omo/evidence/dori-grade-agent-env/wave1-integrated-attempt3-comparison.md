# Wave 1 Integrated Verification - Attempt 3

- **Status**: Full target coverage across interrupted run plus recovery; test gate failed.
- **Worktree**: /Users/indo/code/project/omon-gateway-wt/dori-work
- **HEAD**: 245a5876138acdd1f4a2c31416121f39e95bf416 (unchanged)
- **Pre-run product fingerprint**: 077f1a1eebc25c85015801e8914f60c343504fab83d14380855038328ea66f85 (145 files)
- **Post-run product fingerprint**: 4c1337ced6e222431f024e1d9ab74bff3520635c6c71bf9dab9a74282e1afc9f (145 files). The fingerprints differ; source identity is not certified unchanged.
- **Metadata**: 29 Cargo targets. There are 30 target invocations in raw output because the backend target was repeated in recovery; filtered review-agent invocations have additional summaries.
- **Raw log**: wave1-integrated-attempt3-raw.log; SHA-256 0fe3eadd654f40976ab77cd519513919acafa15b6fd32ec9fed14f353164106c.

## Gate results

| Gate | Result | Evidence |
|---|---|---|
| cargo fmt --check | FAILED, exit 1 | Only known baseline formatting diff at src/agent/backend.rs:61. |
| cargo clippy --all-targets --all-features -- -D warnings | PASSED, exit 0 | Captured Attempt 3 receipt. |
| cargo test --all-targets --all-features --no-fail-fast | FAILED / interrupted then recovered | Initial process has no cargo exit or sentinel. Recovery reran test_omo_backend (1801.14s, 36 passed, 10 failed, 1 ignored) and every queued remaining target; recovery_cargo_exit=101, SENTINEL_ATTEMPT3_RECOVERY_DONE exit=101. |
| test_review_dashboard | PASSED | Included in recovery: 59 passed, 0 failed, 0 ignored, 3.26s. |

## Target summaries

There are 30 Running lines and 31 test-result summaries due to filtered review-agent invocations. The first 19 targets completed in the original run; it then stopped in test_omo_backend with no exit evidence. Recovery reran the backend target and ran all remaining queued targets through test_work_heartbeat. All output and summaries are in the raw log. Recovery's explicit cargo exit is 101; do not represent the initial full invocation as having a known exit.

Full-suite failures in recovery:
- test_cron_boundary_parity: sync_isolates_bad_jobs_and_rearms_changed_once
- test_cron_executor_parity: agent_result_uses_scheduler_delivery
- test_discord_adapter: final_output_filters_controls
- test_migrate: cutover_survives_startup_sync; cutover_second_store_failure_preserves_bytes_and_receipt
- test_omo_backend (10): accepts_received_completion_at_deadline_edge; emits_hermes_activity_lines; omits_workspace_when_per_agent_disabled; provisions_agent_workspace_and_passes_to_thread_start; waits_for_final_message_after_tool_activity; requires_turn_terminal_even_after_final_agent_message; aborts_turn_on_repeated_approval_denials; deadline_interrupts_looping_turn; e2e_thread_lifecycle_and_streaming; retries_once_after_connection_drop
- test_wiring_e2e: test_e2e_actor_omo_backend_persists_assistant_message

## Exact baseline reconciliation (corrected)

The exact baseline list is `/Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt` (16 tests). Attempt 3 has the same 16 failing test names across the same six targets. Therefore all 16 failure identities are baseline-matching, not new failures by name. Panic evidence was compared against `task-2-baseline-full.log`:

- 15/16 panic sites and messages match (including the ten `test_omo_backend` failures); the elapsed duration of `test_omo_backend_deadline_interrupts_looping_turn` differs only numerically (10.137300542s baseline, 10.260226166s Attempt 3).
- The wiring test differs in exact panic detail: baseline fails at `tests/test_wiring_e2e.rs:304` with `assertion failed: final_chunk.is_final`; Attempt 3 fails at line 302 with `Must emit stream chunks to dispatcher`. Its failure name and target match baseline, but its assertion/panic is materially different and is not an exact panic match.
- Thus 15 exact panic matches plus one same-name baseline failure with changed panic detail. This does not establish causal equivalence for the wiring failure.

## Fingerprint provenance (corrected)

No original pre/post per-file manifests or fingerprint command were found in either evidence directory. The saved pre-run digest is `077f1a1eebc25c85015801e8914f60c343504fab83d14380855038328ea66f85`; the reproducible post-run command was `find src tests migrations -type f -print0 | sort -z | xargs -0 shasum -a 256 | shasum -a 256`, yielding `4c1337ced6e222431f024e1d9ab74bff3520635c6c71bf9dab9a74282e1afc9f` across 145 files. Repeating the command yields the same post digest. Without the pre-run manifest and exact command, changed file content cannot be distinguished from a different input set, path/order normalization, or hash procedure. The precise unresolved cause is **missing pre-run per-file manifest and command provenance**; do not assert source mutation or unchanged source from these digests. Git HEAD remains `245a5876138acdd1f4a2c31416121f39e95bf416`; the eight tracked Wave 1 modifications pre-exist relative to HEAD. No evidence ties them to this fingerprint discrepancy.


## Source identity and cleanup

The pre/post source fingerprints differ. HEAD is unchanged; eight tracked Wave 1 files have existing changes versus HEAD, and no source edits were made by this verification. The cause cannot be diagnosed further because the saved pre-run per-file manifest and exact fingerprint command are absent; no evidence identifies a specific changed source path. Recovery owner exited after its sentinel and no cargo/backend test process remained in the final check. Build artifacts remain under target-w33 and target-w6; untracked test files are retained. No services stopped, .env modified, commits, or pushes. Attempt 1 and 2 artifacts remain untouched.

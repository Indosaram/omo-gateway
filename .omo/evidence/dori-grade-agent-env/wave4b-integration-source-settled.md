# Wave4b integration source settlement

Workspace: /Users/indo/code/project/omon-gateway-wt/dori-work.
Status: integrated source and strict gate settled; RELEASE BLOCKED by two failure names outside the canonical backend baseline. No release or deployment occurred.

## Integration

Applied in order: qa-loopback-sink-runner.patch, task35-wiring.patch, task25-wiring.patch, task26-wiring.patch. Reconciled old patch contexts against current runner, CLI and main. Task35 triage/judge context failures and task26 entry context failure were repaired only in the failed files. No duplicate wiring was introduced.

Preserved runner service_result/first_error cleanup, claim-ID CAS, generation fences, no-token Discord-disabled startup, kibitzer hint, guard sampling/retry, status-aware drain, task15 close/reopen lifecycle, and main headless inbox/kibitzer startup. Wiring adds no independent spawned work: existing kibitzer/lane tasks remain owned by the shared cancellation/JoinSet path, and inbox remains on its existing watch shutdown and bounded join path.

Migration numbers are unique: 0038_lane_lifecycle.sql and 0039_model_usage.sql. Task25/26 wiring adds no migration. The task25 owner-DM comment preserves the supplied patch's private destination constraint.

Scoped rustfmt succeeded. All-target/all-feature offline cargo check collected all diagnostics before repair: exit 101, only UsageArgs missing Debug (also reported through test_review_dashboard's included entry). Added Debug without changing CLI behavior. LSP requests timed out; no clean LSP receipt is claimed.

## Pre-fix focused gate

One monitored batch on warm target-w4, RUSTC_WRAPPER empty, -j4, offline, no-fail-fast. The 18 integration targets ran without a filter, then --lib --bin omo-gateway qa_gaps ran in the same batch. Both Cargo invocations exited 0. Splitting the filter prevents accidentally filtering all integration tests to zero.

These results precede the later clippy tuple fix. Their source identity is wave4b-source-pre-1.sha256, SHA-256 0149e5c296c2945d44add7e6f8b583e233cd9633141416f1d5a899e556963a43, 224 entries. Parent target counts exclude separately printed subprocess helper summaries.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| test_lane_close | 27 | 0 | 0 |
| test_lane_runner | 12 | 0 | 1 |
| test_lane_reopen | 5 | 0 | 0 |
| test_lane_done | 11 | 0 | 0 |
| test_lane_kibitzer | 9 | 0 | 0 |
| test_lane_guard_alerts | 5 | 0 | 0 |
| test_lane_status_transport | 1 | 0 | 0 |
| test_inbox_triage | 8 | 0 | 0 |
| test_inbox_store | 10 | 0 | 0 |
| test_inbox_cli_digest | 9 | 0 | 0 |
| test_inbox_proposals | 9 | 0 | 0 |
| test_usage_done_gate | 9 | 0 | 0 |
| test_cron_health | 12 | 0 | 2 |
| test_session_registry | 7 | 0 | 0 |
| test_session_registry_task27 | 2 | 0 | 0 |
| test_lane_dispatch | 10 | 0 | 0 |
| test_lane_events | 25 | 0 | 1 |
| test_profile_import | 7 | 0 | 0 |
| Integration total | 178 | 0 | 4 |

Actual qa_gaps result lines, preserved in wave4b-qa-gaps.log:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 485 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out; finished in 0.05s
```

## Strict fix and post-fix coverage

Strict attempt 1: fmt 0; clippy 101; build not run. All diagnostics were collected: record_notification had 8 arguments, above clippy's 7-argument limit. No warning was suppressed.

Exact diff: wave4b-post-focused-clippy-fix.patch. Files: src/usage.rs, src/agent/omo_backend.rs, tests/test_usage_done_gate.rs. The function now accepts the thread/turn identity as one (&str, &str) tuple and destructures it immediately. All seven callers were updated, including the production backend call. Assertions and notification semantics are unchanged. Final manifest comparison confirms exactly these three paths changed after the pre-fix gate.

Post-fix test_usage_done_gate: exit 0, 9 passed / 0 failed / 0 ignored (0.20s). This covers counter snapshot replacement, identity rejection, aggregation and done gates. Additional coverage for the changed backend file ran only test_lane_events and test_omo_backend: events 25 passed / 0 failed / 1 ignored; backend 43 passed / 5 failed / 1 ignored. Combined backend coverage exit 101. No unfinished test remains in this run.

Strict attempt 2 ran once: fmt 0, strict all-target/all-feature offline clippy 0, offline binary build 0, pre/post source cmp 0. Owned TMPDIR /tmp/wave4b-strict.WDVBJF was retained through build completion. The strict build completed before the later baseline-bounded release instruction arrived.

## Baseline comparison and release stop

Sources: durability-fixtures-20261011T180128Z-baseline-policy.md and its canonical batch-20261011T135508Z-handoff.md (10 backend names). task-2-failing.txt was not found in either inspected workspace; the preserved canonical handoff explicitly maps its failure names to that original baseline.

| Current failure name | Canonical baseline |
| --- | --- |
| discord_silent_turn_interrupted_after_45s | NO: new failure; policy previously records PASS |
| lane_silent_turn_not_interrupted_after_50s | NO: new failure; policy previously records PASS |
| test_omo_backend_aborts_turn_on_repeated_approval_denials | YES |
| test_omo_backend_deadline_interrupts_looping_turn | YES |
| test_omo_backend_e2e_thread_lifecycle_and_streaming | YES |

The JSON records all 10 canonical names with their current status: seven passed, three failed. The previously unfinished lifecycle test failed in this completed run; retries_once_after_connection_drop passed. Neither is misreported as unfinished or assumed passed.

New failure errors:

```text
discord_silent_turn_interrupted_after_45s:
error message must mention initial deadline interruption, got: LLM error: timeout waiting for turn/start acknowledgement from omo app-server

lane_silent_turn_not_interrupted_after_50s:
lane turn must NOT be interrupted after 50s of silence
```

Per the latest instruction, two new names require stopping, not fixing or releasing. No further build, source repair, or release was attempted after comparison.

## Built candidate identity (not released)

Binary: target-w4/debug/omo-gateway.
SHA-256: 5b765f9394354022dde3006252f8d874c74801ee2c54bd373758b54b7eb20f97.
Source manifest: wave4b-source-post-2.sha256, 224 entries.
Manifest SHA-256: 3c5d7ff8e4cdbaaf1f3ce1e21ee0d4038aa62e5a089cad400f3b1e6fa537194e.
Pre manifest has the same hash. source_cmp=0.

Machine evidence: wave4b-strict-gate-release.json, status blocked-new-backend-failures, release_allowed=false. Raw logs and strict exit receipts remain beside it. No services, live Discord, model calls, env-file edits, or git commands were performed.

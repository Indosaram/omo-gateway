# Wave 4 integration source settlement

Status: four authorized wiring artifacts integrated; combined gate passed exit 0. Source settled for strict clippy/build gate, not runtime approval. Only runner.rs/main.rs modified by this integration; CLI unchanged, no posts.rs edit.

## Reconciled unit in requested order

1. task16-runner-wiring.patch: optional kibitzer_hint field, builder and wake fan-out added against repaired runner. No separate producer/consumer lifecycle invented beyond this artifact.
2. task17-runner-wiring.patch: cheap 60-second guard interval with missed-tick Skip, host sample, guard transition to OMON_LANE_DEFAULT_LEAD when available, queued retry. Inline work resides inside captured service_result; no new detached runner task. Guard sampling/transition/retry errors retain artifact's warning semantics.
3. task18-runner-wiring.patch: startup status-aware drain plus refresh; refresh on running-loop wake boundaries; status-aware drain on posts notification. Fallible runtime refresh/drain stays inside service_result and shared first_error/cancel/drain boundary. Startup work remains pre-lane/settlement-spawn. Task18 posts.rs read only and never modified while its worker run is active.
4. task24-wiring.patch main hunk: construct production Triage with OMON_LANE_DEFAULT_LEAD and pass collector progress receiver to Triage::serve using existing watch shutdown; joins in collector's existing JoinSet. Removed unnecessary mut on moved progress receiver. Retained other main/headless/actor shutdown work.

No-token runner construction untouched. run_lane/CAPTURE_GENERATION and CLI claim/reopen fences untouched. Settler remains8bd0 claim-history-ID CAS. No new schema/dependency/model/env/service/Discord/git action.

## Diagnostics and command ledger

Collected all three LSP all-severity requests for runner/cli/main before fixes: each freshness_timeout3000ms; automatic main check later daemon request timeout30000ms. These are unavailable fresh diagnostics, not zero-error verdicts. Scoped rustfmt initially exit1 solely on added main map_err layout; applied exact formatter layout. No other reported formatting issue.

Single requested combined command: cd /Users/indo/code/project/omon-gateway-wt/dori-work && rustfmt --edition 2021 --check --config skip_children=true src/lanes/runner.rs src/lanes/cli.rs src/main.rs && CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4 CARGO_BUILD_JOBS=2 cargo test --offline -j2 --all-features --test test_lane_done --test test_lane_runner --test test_lane_reopen --test test_lane_kibitzer --test test_lane_guard_alerts --test test_lane_status_transport --test test_inbox_triage --test test_inbox_store --test test_cron_health --test test_session_registry --test test_session_registry_task27 --test test_lane_dispatch --test test_lane_events --no-fail-fast -- --nocapture. Monitor mon_DD7SH4CQ8QQRERXT/bash_2, explicit timeout1800000ms. Scoped rustfmt passed before Cargo; test-profile compile finished1m54s without reported compiler errors/warnings. No standalone build/full suite. Combined completed exit 0; final counts below. No failing-target rerun needed.

## Gate-input hashes

- src/lanes/runner.rs: 0eb9920fc25bb657aab160fc4ecde10950ba425c7f6241373a32cc3250ec97cd
- src/lanes/cli.rs: 3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510
- src/main.rs: 54461d5eb4b2d2b4b5538c9891315dbc2200b1c86b2a5a9eb7ea1be3dc17db8c
- src/lanes/settle.rs: 8bd0c6362fcae5d77daf9c3869558afa410c0a92d6e459af8dd42932e73972c5
- src/lanes/posts.rs: e2562ae76e241c5d30096cbb834cf8a28c1ca8073df81bf08a47a94d9567e77c
- src/lanes/guard.rs: 7c1118cfc34e5a98a6ff8bb7b40feab793a0c6e2808d71d889e17830125675ba
- src/lanes/kibitzer.rs: dce7ad02a453e5f074d0af00ed70acefa81baac7feaf8ac842ee7fea203e0f71
- src/inbox/triage.rs: e05c33fadb6f1817a3b13de3021413c912a5e627872f79b8c6eb6dfec1ef57d9
- Cargo.toml: 1d63dce8c69395ae113f923446f850cad5822978e34a1601b7145fe4f9a26563

These hashes describe captured gate inputs, not an entire frozen repository: other worker files remain independently owned. Post-gate exact-byte comparison verified all nine listed hashes unchanged. Strict clippy/build is parent's next gate, not claimed here.

## Final actual combined results

Monitor mon_DD7SH4CQ8QQRERXT/bash_2 completed exit 0. All thirteen requested targets ran in the single combined invocation; no failing targets, integration fixes or reruns required. Scoped rustfmt passed at command entry; test-profile compilation finished in 1m 54s. No compiler warning/error was reported in captured output.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| test_cron_health | 12 | 0 | 2 |
| test_inbox_store | 10 | 0 | 0 |
| test_inbox_triage | 8 | 0 | 0 |
| test_lane_dispatch | 10 | 0 | 0 |
| test_lane_done | 11 | 0 | 0 |
| test_lane_events | 25 | 0 | 1 |
| test_lane_guard_alerts | 5 | 0 | 0 |
| test_lane_kibitzer | 9 | 0 | 0 |
| test_lane_reopen | 5 | 0 | 0 |
| test_lane_runner | 12 | 0 | 1 |
| test_lane_status_transport | 1 | 0 | 0 |
| test_session_registry | 7 | 0 | 0 |
| test_session_registry_task27 | 2 | 0 | 0 |

Total main targets: 117 passed, 0 failed, 4 existing ignored. Startup/helper child results are not double-counted. Ignored tests: two cron subprocess helpers invoked by parent tests, configured-host subprocess helper in lane events invoked by its parent test, and disposable live app-server runner QA. No skip/ignore added or changed. The two injected panic logs in service_storage_errors_cancel_and_join_backend_and_checker are expected real JoinSet fault inputs; that regression passed and proves preserved cleanup. Status transport ran on owned loopback, not live Discord. No services/models/env/Discord/git action by this integration track; test-owned fixtures/loopback/mock surfaces are the verification boundary.

LSP fresh diagnostics remain unavailable as recorded above; no clean LSP claim. No broad suite, strict clippy, standalone/release build or live QA performed. Artifacts 16/17/18/24 are now reconciled source wiring, not independent runtime certification; task16 optional hint only is the supplied runner artifact, and production consumer startup beyond that artifact is not claimed. Source handoff complete; stop at this receipt for parent strict gate.

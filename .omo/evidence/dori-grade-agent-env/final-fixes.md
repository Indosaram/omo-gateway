# Final verification fixes

Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Plan references below refer to `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.

## Mechanical clippy fixes

- `src/cron/report.rs`: pass `job` directly to `bind`, `publishing`, and `trigger_wait`. `request.as_str()` already returns `&str`, so these changes remove an extra reference without moving owned data.
- `src/lanes/settle.rs`: collapse the nested remote-check branch into `else if let`. Reservation handling, remote checks, local signals, worktree checks, and diff gating retain their original order and behavior.

## Judge contract decisions

Todo 33 (plan lines 524-530) establishes generic small-model fallback and bounded persistent accounting. Todo 35 specializes done-claim verification: line 548 says "block a close when jev is unavailable" is forbidden, and requires logging `done_diff skipped` and continuing with signals only; line 552 explicitly requires an unavailable judge not to prevent closing. The owner-approved DoneDiff exception is therefore retained. No judge product behavior is changed.

- `test_diff_caps_ordering_disabled_jev`: expect the typed skip error, zero fallback calls, and unchanged daily counts. Disabled jev takes precedence over a zero jev quota; it does not authorize a DoneDiff chat-model fallback.
- `test_diff_sanitized_input_path`: a broken repo now expects a typed skip and no fallback call, while still proving the CLI never inherits the gateway cwd. Empty-diff verdicts, secret scrubbing, bounded stdin, valid dirty-diff scores, and isolated CLI cwd assertions remain intact.
- `test_malicious_machine_contract_boundaries`: retain choice and noul rejection/fallback assertions. Use an actual dirty tracked file so each malformed diff output reaches the CLI parser, assert one additional CLI call, and expect a typed skip without an additional fallback call. The old empty non-git directory never exercised malformed diff parsing.
- `test_restart_persistent_accounting`: expect `DailyCapExceeded { purpose: DoneDiff, cap: 2 }` when the jev cap is exhausted, and the typed skip when jev is disabled. Preserve the exact restored `(2, 2)` counters and other choice/noul cap assertions; additionally assert DoneDiff did not consume fallback calls. This follows todo 33's daily-cap contract and todo 35's signal-only skip exception.
- `test_fallback_turn_cancellation`: inspection found that it already drives `fallback_choice(Purpose::LaneSize)`, a reachable generic fallback under todo 33, not DoneDiff. The actual blocker is the mock ignoring the new `thread/settings/update` RPC. Acknowledge that RPC and assert the DeepSeek small model so the test can reach `turn/start` and observe the exact interrupt acknowledgement. Existing error, elapsed-bound, thread ID, and turn ID assertions remain intact.

## Model-routing mocks

- `test_owner_dm_turn_start_routes_to_opus_mock_appserver`: accept and acknowledge `thread/settings/update` before `turn/start`; capture the routed model from settings, preserving the owner-DM Opus assertion and completed turn flow.
- `test_unknown_small_model_fallback_to_deepseek_never_opus_mock_appserver`: acknowledge settings on both attempts and capture both models there. Preserve the unknown-model turn error that triggers retry, the successful fallback turn, and the exact DeepSeek/never-Opus assertions. This preserves todo 34's routing intent (plan lines 535, 541-542) at the RPC where the current backend actually applies the model.

## Adopted-lane signal

- `adopted_lane_reads_claim_then_settles_after_objection_window`: create `proof.txt` in the test's isolated cwd and declare `file proof.txt` instead of the rejected `command ["true"]`. Preserve adoption, send deduplication, report/claim distinction, the mock-clock objection window, settlement, and release assertions.

## Validation and exclusions

The five initial touched Rust files were formatted with rustfmt. One prescribed offline clippy run followed the initial batch:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo clippy --offline -j4 --all-targets --all-features -- -D warnings
```

No tests are run in this fix batch; the parent owns the single subsequent verification rerun. Test pass status remains unverified here. Existing baseline failures in `test_cron_boundary_parity`, `test_cron_executor_parity`, `test_migrate`, `test_wiring_e2e`, and the three `test_omo_backend` cases are untouched. No commits, deployments, or environment-file edits are performed.

rustfmt completed successfully (exit code 0) for both the initial batch and the additional lock-scope fix. Fresh LSP diagnostics for the judge, routing, and cron-remediation test files timed out after 3000 ms; no clean LSP result is claimed. The all-targets clippy result is recorded below.

Validation result: the single prescribed clippy run exited 101. The four originally reported lint errors no longer appeared. It exposed one additional `clippy::await_holding_lock` error in `tests/test_cron_remediation.rs:118`: the publishing test retained a synchronous lanes guard through the SQL query await. A lexical block now releases that guard before the await, preserving every assertion. This is not one of the excluded baseline-failure targets. The sixth touched Rust file is also formatted. No second clippy invocation is run because the owner explicitly restricted this batch to ONE invocation; the final lock-scope fix is therefore not compiler/lint reverified here. A clean final clippy result remains for the parent's verification pass.

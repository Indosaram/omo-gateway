# Todo 33 gate evidence

Working checkout: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## REAUDIT 9B RETRACTIONS AND CORRECTIONS
1. **RETRACTED: Empty diff manufactured fulfills=1.0**:
   - *Prior false claim*: Claimed empty diff returns `fulfills: 1.0, clean: 1.0, risk: 0.0`.
   - *Correction*: Returning `fulfills: 1.0` on an empty diff falsely certified unimplemented requested changes. Fixed in source: an empty diff returns safe conservative `DiffVerdict { fulfills: 0.0, clean: 1.0, risk: 1.0 }` without invoking CLI. Broken repo diff routes to fallback (`risk: 2.0`). Non-empty diffs run from `temp_dir()` with scrubbed stdin (<=4000 chars). Verified in `test_diff_sanitized_input_path`.
2. **RETRACTED: Disabled diff quota ordering gap**:
   - *Prior false claim*: Claimed 100% fidelity on disabled Jev accounting.
   - *Correction*: In prior source at lines 1274-1316, `diff()` evaluated `capped_outcome` before `must_fallback`, causing `with_caps(0, 50)` with Jev disabled to skip fallback because `jev_daily_cap == 0`. Fixed in source: `diff()` now evaluates `must_fallback` before the Jev daily cap, properly routing disabled Jev to fallback and enforcing the fallback cap. Verified in `test_diff_caps_ordering_disabled_jev`.
3. **RETRACTED: Fabricated turn/completed client consumption**:
   - *Prior false claim*: Claimed server emitted `turn/completed(interrupted)` and client consumed it.
   - *Correction*: Retracted. The cancellation test genuinely proves bounded abort on timeout (<5s), invocation of `backend.cancel()`, mock server receipt of `turn/interrupt` with matching `threadId` (`mock-th-cancel-1`) and `turnId` (`mock-turn-cancel-1`), and verified successful server write of the interrupt acknowledgement (`id: 9001`). Client terminal event consumption is NOT asserted and is faithfully recorded as an unexercised client lifecycle limitation.
4. **RETRACTED: Semantic prompt injection immunity**:
   - *Prior false claim*: Claimed adversarial prompt injection resistance.
   - *Correction*: Retracted and qualified solely as **machine-contract output boundary enforcement**: outputs outside allowed options, malformed JSON, and out-of-range floats are rejected by the parser and mapped to conservative defaults.
5. **Diff Metric Bounds Enforcement**:
   - Added validation in `parse_diff_output` and `fallback_diff`: requires finite `fulfills` in [0.0, 1.0], `clean` in [0.0, 1.0], and `risk >= 0.0`. Out-of-bounds values (e.g. fulfills=9, clean=2, negative risk, NaN) are strictly rejected. Verified in `test_diff_metric_boundary_validation`.

## Focused Test Suite (22 passed, 0 failed, 3 ignored)
`CARGO_TARGET_DIR=$PWD/target-w33 CARGO_BUILD_JOBS=2 cargo test --test test_judge`: exit 0; 22 passed, 0 failed, 3 ignored in 155.25s.
All 22 unit and adversarial integration tests pass cleanly, covering:
- Malformed CLI outputs and probability bounds
- Secret scrubbing from state and instructions
- Single owner alert per day on circuit breaker trip
- Circuit breaker tripping after 5 failures and preventing subsequent CLI spawns
- Disabled Jev switch (`OMON_JEV_ENABLED=false`) routing directly to fallback
- Hung CLI timeout routing to fallback
- State size thresholding (under 500 chars inline `-s`, over 500 chars via `--file`)
- Daily cap (301st call returning capped outcome without spawning CLI)
- SQLite database persistence of judge call telemetry (`judge_calls` table)
- **Machine-contract output boundary enforcement**: non-option outputs, code fences, and injected options strictly neutralized and falling back to conservative choice (`test_prompt_injection_resistance_choice`)
- **Malicious machine-contract boundaries**: unclosed JSON, ANSI control codes, float overflows, NaN, and negative infinity rejected (`test_malicious_machine_contract_boundaries`)
- **Active fallback turn cancellation**: dual-connection WebSocket mock asserting `turn/interrupt` delivery with matching thread/turn IDs and checked ack write (`test_fallback_turn_cancellation`)
- **Restart-persistent accounting**: mixed attempts (successful Jev, failed Jev, fallback, and capped calls across choice, noul, and diff) accurately persisted and restored without quota distortion (`test_restart_persistent_accounting`)
- **Concurrent cap enforcement**: 25 concurrent tasks on `tokio::sync::Barrier` enforcing exact Jev daily cap (10) and 15 concurrent tasks enforcing fallback exhaustion cap (5) (`test_concurrent_daily_caps_and_fallback_exhaustion`)
- **Diff sanitized input path**: clean diff returns safe conservative `fulfills: 0.0` without invoking CLI; broken repo takes fallback without CLI; dirty repo runs CLI from temp_dir with scrubbed stdin (`test_diff_sanitized_input_path`)
- **Diff caps ordering in disabled mode**: `with_caps(0, 50)` with Jev disabled routes to fallback and does not skip due to Jev cap 0 (`test_diff_caps_ordering_disabled_jev`)
- **Diff metric boundary validation**: verifies strict range enforcement [0,1] for fulfills/clean and >=0 for risk, rejecting overflows and negative risks (`test_diff_metric_boundary_validation`)
- **Breaker alert secret scrubbing**: stderr containing auth tokens scrubbed with `[REDACTED_TOKEN]` before owner DM dispatch (`test_circuit_breaker_alert_scrubs_secrets_in_stderr`)

## Live QA & Fallback Verification
- **Real Small-Model Fallback Inference**: Executed live with `inferhub/cb/deepseek-v4.1-flash` via real `Fallback` engine directly on isolated test port 19749 (`test_live_small_model_fallback_inference`). Response `small` in 11150ms with `is_fallback: true` (target model `inferhub/cb/deepseek-v4.1-flash`). Child daemon terminated cleanly, leaving port 19749 free.
- **Live 12 JevCli Samples**: All 12 classification and scoring samples passed through live `typesafe-judge` CLI (`test_live_12_samples_replay`, exit 0, 6.99s).
- **Outage Probe**: Outage routing to fallback when `TYPESAFE_URL` is dead verified (`test_dead_url_fallback_probe`, exit 0, 0.06s).

## Formatting & Lint Gates
- `rustfmt --edition 2021 --check src/judge/mod.rs tests/test_judge.rs`: exit 0, clean, 0 diffs.
- `cargo fmt --check`: only pre-existing baseline diff at `src/agent/backend.rs:61`; zero diffs from judge files.
- `cargo clippy --test test_judge -- -D warnings`: exit 0, clean, 0 warnings.

## Cleanup Receipt
- All live test processes (test app-server on port 19749, WebSocket test connections, SQLite memory pools) were cleaned up immediately upon test completion.
- Verified `lsof -iTCP:19748,19749` is empty.
- No services were stopped; no `.env` files were modified; no git commits or pushes performed.

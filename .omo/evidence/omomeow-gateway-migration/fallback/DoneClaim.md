# DoneClaim: Model-Rejected Fallback Migration to DeepSeek

- **Scope**: Exclusive to `src/agent/omo_backend.rs`, `src/agent/omo_config.rs`, and `tests/test_omo_backend.rs`.
- **Target Worktree**: `/Users/indo/code/project/omon-gateway-wt/omomeow-migration-w1` on branch `omomeow-migration-w1`.

## Implemented Capabilities
1. **Fallback Eligibility on Model Rejection**:
   - Enhanced `is_model_rejected_error` substring matching for daemon/RPC errors: `model not found`, `unknown model`, `unknown_model`, `model rejected`, `model update rejected`, `rejected the model`, `invalid model`, `unsupported model`, `model_not_found`, and `thread/settings/update` rejected handshakes.
   - Initialized `experimentalApi: true` capability in backend WebSocket initialization handshake (`do_initialize`) so daemon accepts `thread/settings/update` requests.
   - Handshakes `thread/settings/update` when switching to fallback model and verifies JSON-RPC acknowledgment.

2. **Default Fallback Model Configuration**:
   - `DEFAULT_FALLBACK_MODEL` defined as `"inferhub/cb/deepseek-v4.1-flash"`.
   - Env var resolution supports `OMON_OMO_FALLBACK_MODEL` and fallback `FALLBACK_MODEL`, defaulting to DeepSeek when empty or unset.
   - Added unit test `test_fallback_model_env_contract`.

3. **One-Shot Guard & Persistence Marker Semantics**:
   - Guard ensures fallback executes at most once per turn and never falls back when the primary model is already the fallback model (`fallback_one_shot_guard_never_refalls_back_from_fallback`).
   - Persists `omo_fallback_model_active: true` in session metadata and SQLite session binding upon fallback, so daemon restarts or subsequent turns restore and track state correctly (`model_rejected_failure_triggers_fallback_to_deepseek_and_persists_marker`).
   - Restores primary model settings cleanly when `omo_fallback_model_active` is reset.

## Verification Evidence
- **Format**: `rustfmt --edition 2021 --check src/agent/omo_backend.rs src/agent/omo_config.rs tests/test_omo_backend.rs` -> PASS (clean, 0 diffs).
- **Clippy**: `cargo clippy --test test_omo_backend -- -D warnings` -> PASS (clean, 0 warnings).
- **Target Unit Tests**:
  - `fallback_one_shot_guard_never_refalls_back_from_fallback` -> PASS
  - `model_rejected_failure_triggers_fallback_to_deepseek_and_persists_marker` -> PASS
  - `test_fallback_model_env_contract` -> PASS
- **Test Suite Results**:
  - `cargo test --test test_omo_backend` -> 39 passed; 2 pre-existing failures (identical to baseline prior to any edits):
    - `test_omo_backend_e2e_thread_lifecycle_and_streaming`: Pre-existing assertion failure (`left == right`, `Some(true)` vs `Some(false)` on line 1693).
    - `test_omo_backend_aborts_turn_on_repeated_approval_denials`: Pre-existing approval denial timeout under default approval configuration.
- **Untouched Peer Scope**:
  - No changes made to `src/work_heartbeat.rs`, `tests/test_work_heartbeat.rs`, `migrations/0030_work_items.sql`, `src/lib.rs`, `src/main.rs`, `src/storage/*`, or `src/cron/*`.

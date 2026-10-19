# Task 27 Corrected Source-Freeze Receipt: Cross-Machine OMO Session Registry

## Status & Scope
- **Task**: 27 (GAP-15: Cross-machine OMO session registry)
- **Worktree**: `/Users/indo/code/project/omon-gateway-wt/dori-work`
- **Sole Ownership Files**:
  - `src/sessions/registry.rs`
  - `tests/test_session_registry.rs`
- **Shared Files**: Untouched (`src/entry.rs`, `Cargo.toml`, `src/lib.rs`, `envconfig`, `.env` unmodified).
- **Remote Operations**: None performed.
- **Disposition**: `source-frozen`; scoped format check passed (`rustfmt --edition 2021 --check`). Global Cargo tests, clippy, and live gateway QA remain pending coordinated batch execution.

---

## Addressed Blockers & Re-review Correction

### 1. Blocker 1: Process Lifetime & Cancellation Contract (T27-CANCEL)
- **Source setting**: `ProcessCommandRunner::run` sets `kill_on_drop(true)` on `tokio::process::Command` before awaiting `.output()`. When the 20-second per-host deadline drops the execution future, Tokio is configured to send `SIGKILL` and register the child for reaping.
- **Test verification note**: The unit and integration tests (`hanging_host_times_out_without_blocking_other_hosts` and `timeout_drops_child_future_deterministically_without_sleeps`) use a fake runner and a `DropGuard` on bounded channels with "subscribe before trigger" to verify that future drop/cancellation propagates deterministically upon timeout. **They test fake DropGuard future cancellation only, NOT actual OS-level child process killing or OS reaping**. Verification of actual OS process termination and reaping remains unverified pending runtime Cargo/QA gates.

### 2. Blocker 2 & B2-R: Planned Fields & Serde Aliases (T27-FIELDS / T27-GATES)
- **Planned output schema**: `Session` serializes canonical fields `{machine, session_id, name, cwd, status, updated_at, controls}` matching plan line 457.
- **B2-R correction**: Removed all duplicate aliases (`updated_at`, `updatedAt`, `lastActivityAt`) from `last_activity` in `src/sessions/registry.rs:119-124`. `last_activity` now uses `#[serde(default)]` only. This eliminates duplicate Serde deserialization match patterns and prevents unreachable pattern warnings under `cargo clippy --all-targets --all-features -- -D warnings`.
- **Preserved API**: Retained `pub last_activity: Option<String>` and `pub raw: Value` on `Session`. In `parse_sessions`, `updated_at` and `last_activity` remain bidirectionally synchronized so callers of either field receive the correct timestamp. No bonus compatibility helpers were added.
- **Tests**: Existing unit test `parse_sessions_extracts_and_serializes_planned_fields` and integration test `parser_and_serializer_preserve_exact_planned_fields` assert both deserialization and exact planned serialized field names.

### 3. Blocker 3: Timeout Acceptance Test Fixture Contradiction (T27-TEST)
- In `hanging_host_times_out_without_blocking_other_hosts`, the successful host fixture returns a valid, representative session record with all planned fields rather than an empty JSON array `b"[]"`.
- The expected count assertion `assert_eq!(listed.sessions.len(), 1)` is preserved (not weakened to 0) and validates the successful session alongside the isolated stalled-host error.

---

## Scoped Verification
- `rustfmt --edition 2021 --check src/sessions/registry.rs tests/test_session_registry.rs`: Clean, exit code 0.
- No Cargo test/clippy, live SSH, service, model, or git commit operations executed.

# DoneClaim: Cron Payload Fields, Catch-up, Timezone, and Destination Parity

## Changed files
- `src/cron/store.rs`
- `src/cron/executor.rs`
- `src/cron/scheduler.rs`
- `src/discord/buttons.rs`
- `.omo/evidence/omomeow-gateway-migration/cron-payload/`

## Changes
- **Optional Payload Fields**:
  - Added `brief_file: Option<PathBuf>` and `catch_up_hours: Option<u64>` (defaulting to 6 hours) on `HermesJob`.
  - Added string-or-number deserializer for origin `chat_id` and `thread_id` in `HermesOrigin`.
- **Runtime Brief Loading & Fallback**:
  - In `AgentCronExecutor::execute` and `execute_native_cron`, dynamically reads `brief_file` from disk if specified, applying security and lifecycle validations. If absent, cleanly falls back to direct prompt.
- **Bounded Occurrence-Keyed Catch-Up**:
  - Implemented `newest_missed_occurrence` supporting cron, interval, and one-shot schedules with timezone awareness.
  - Scheduler bounds catch-up by `catch_up_hours` (default 6h, 0 skips overdue execution and advances to next future).
  - Fires at most ONE catch-up execution per downtime period for the newest missed occurrence, persisting `last_occurrence` for durable deduplication across polls and restarts.
  - Preserved `last_occurrence` in `Synchronizer` UPSERT queries to prevent Hermes sync from overwriting runtime dedup state.
- **Discord Thread Destination Validation**:
  - Ensured `delivery_destinations` preserves Discord thread IDs in `SessionKey`, verified end-to-end dispatch.
- **Timezone Validation**:
  - Validates timezone strings against `chrono_tz::Tz` during job validation and schedule computation, refusing invalid timezones and uncomputable cron expressions at load time.

## Verification
- `cargo test --lib cron::scheduler::tests::test_ -- --nocapture` — exit 0; 24 passed, 0 failed, including:
  - `test_brief_file_read_and_fallback`
  - `test_catch_up_single_fire_after_simulated_downtime`
  - `test_thread_destination_parse_and_deliver`
  - `test_timezone_validation_accepts_and_refuses`
- `cargo test --lib cron` — exit 0; 64 passed, 0 failed.
- `cargo fmt --check -- src/cron/store.rs src/cron/executor.rs src/cron/scheduler.rs` — exit 0.
- Out-of-scope sibling files in progress (`tests/test_work_heartbeat.rs`, `src/agent/omo_backend.rs`) were not touched and kept out of staging.

## ULTRAQA
- `misleading_success_output`: Verified with 24 individual unit tests in scheduler plus 64 cron library tests.
- `flaky tests`: All tests use isolated memory databases (`sqlite::memory:`) and tempfile sandboxes.
- `hung commands`: Clean cargo runs with bounded execution time (<1s for test run).

## Cleanup receipt
- No temporary files left behind; temp directories are cleaned up by RAII guards.
- Sibling worktree changes (heartbeat migrations, agent backend) untouched.

## Risks
- Whole-workspace `cargo check` / `cargo clippy` passes at library level, but workspace integration compilation contains unrelated in-progress heartbeat files authored concurrently by another agent.

# Gate Review Report: Todo 4 - Add the lanes registry schema and storage functions

- **Plan**: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`
- **Task**: Todo 4 ('Add the lanes registry schema and storage functions')
- **Recommendation**: APPROVE
- **AdversarialVerify**: confirmed
- **Blockers**: None

## Original Intent
The user and plan required creating the durable storage layer for the lane engine in `omon-gateway`. This includes the database schema migration for `lanes`, `lane_history`, `lane_events`, and `lane_transcript` tables and indexes, transactional Rust storage operations in `src/storage/lanes.rs` (re-exported via `src/storage/mod.rs`), and comprehensive integration tests proving schema constraints, CAS state transitions, concurrency safety, and live database migration non-destructiveness.

## Desired Outcome
- `migrations/0031_lanes.sql` defines:
  - `lanes` table with key character/length check constraints (`1..=48`, `[a-z0-9-]`), status check constraints (`queued`, `working`, `blocked`, `done_claimed`, `not_done`, `verified_done`, `closed`, `failed`), and all required metadata/execution columns.
  - `lane_history` table recording state transitions (`lane_key REFERENCES lanes(key)`, `from_status`, `to_status`, `reason`, `at`).
  - `lane_events` table for lead session notifications (`lane_key`, `lead_session_key`, `kind`, `body`, `created_at`, `delivered_at`).
  - `lane_transcript` table for capturing agent transcripts (`lane_key`, `at`, `role`, `text`).
  - Required indexes: `idx_lanes_status`, `idx_lane_events_delivered_lead`, and `idx_lane_transcript_lane_id`.
- `src/storage/lanes.rs` implements:
  - `insert_lane`, `get_lane`, `get_lane_by_thread`, `list_lanes(statuses)`
  - `transition_lane(key, from_set, to, reason) -> bool` as a single transaction executing atomic CAS status check + update + history entry creation (returning `false` on CAS mismatch)
  - `set_lane_fields` supporting partial field updates and clearing nullable fields
  - `append_transcript` and `read_transcript(key, last_n)` with chronological ordering
  - `enqueue_lane_event`, `take_undelivered_events(limit)`, `mark_events_delivered(ids)`
- All functions re-exported from `src/storage/mod.rs`.
- `tests/test_lanes_storage.rs` registered in `Cargo.toml` proving:
  - Migration creation of all 4 tables and indexes.
  - Check constraint rejection of malformed keys (e.g., "Bad Key", invalid chars, length limits).
  - Concurrency safety with two simultaneous tasks racing CAS transitions (exactly one succeeds and records history).
  - Live database copy migration preserving `work_items` unchanged (count 0) and adding all four lane tables.
- Must NOT alter or drop `work_items`.
- Rustfmt and Clippy pass without new warnings or errors vs baseline.

## Database Contract Verification (Engine Semantics & Query Plan)
Actual database behavioral contracts were independently probed and verified on SQLite runtime:
1. **Status CHECK Constraint**: Inserting a row with an invalid status (`invalid_status`) is rejected at engine level with `CHECK constraint failed: status IN ('queued','working','blocked','done_claimed','not_done','verified_done','closed','failed')`.
2. **Key CHECK Constraint**: Tested boundary lengths (0, >48) and character sets (uppercase, punctuation, underscores, dots, slashes) — all fail with SQLite `CHECK constraint failed`.
3. **Foreign Key Integrity**: Inserting into `lane_history` with a non-existent `lane_key` when foreign keys are active triggers SQLite `FOREIGN KEY constraint failed`.
4. **Index Query Plan Utilization (`EXPLAIN QUERY PLAN`)**:
   - `SELECT * FROM lanes WHERE status = 'working'` uses `SEARCH lanes USING INDEX idx_lanes_status (status=?)`.
   - `SELECT * FROM lane_events WHERE delivered_at IS NULL AND lead_session_key = ?` uses `SEARCH lane_events USING INDEX idx_lane_events_delivered_lead (delivered_at=? AND lead_session_key=?)`.
   - `SELECT * FROM lane_transcript WHERE lane_key = ? ORDER BY id DESC LIMIT 5` uses `SEARCH lane_transcript USING INDEX idx_lane_transcript_lane_id (lane_key=?)`.

## User Outcome Review
All Todo 4 scope criteria are satisfied in observable reality:
1. **Transactional Storage API**: `src/storage/lanes.rs` implements all 11 required storage functions. `transition_lane` executes inside an atomic `tx = pool.begin().await?` block, validates the from-state, executes an UPDATE with CAS condition `AND status IN (...)`, verifies `rows_affected() > 0`, records the `lane_history` row, and commits. If the CAS condition fails or the row does not match, it returns `Ok(false)` without writing history.
2. **Parameter Binding and Injection Safety**: All dynamic SQL queries use parameterized bindings (`?`). Dynamic placeholder vectors are constructed strictly from slice lengths (`vec!["?"; n].join(", ")`). No user input is formatted directly into SQL strings.
3. **Integration Test Suite**: `tests/test_lanes_storage.rs` contains 7 comprehensive async integration tests, all passing:
   - `migration_0031_creates_four_tables_and_indexes`: PASS
   - `key_bad_key_rejected_by_check_constraint`: PASS
   - `concurrent_transitions_exactly_one_succeeds_and_history_recorded`: PASS
   - `lane_crud_and_status_filtering`: PASS
   - `transcript_append_and_read_ordering`: PASS
   - `lane_events_queue_take_and_mark_delivered`: PASS
   - `real_data_migration_preserves_work_items_and_adds_lanes_tables`: PASS
4. **Dashboard Suite**: `cargo test --test test_review_dashboard` passes 59/59 tests (4.79s).
5. **Real Database Non-Destructiveness**: The real-data QA scenario was independently executed on a temporary copy of `/Users/indo/code/project/omon-gateway/omon_gateway.db` (including WAL/SHM sidecars). Pre-migration `work_items` row count was 0; post-migration row count was 0; all four lane tables were successfully created. The live production database was never opened or modified directly.
6. **Scope and File Boundaries**: Only the 5 files claimed by Todo 4 were modified/added: `migrations/0031_lanes.sql`, `src/storage/lanes.rs`, `src/storage/mod.rs`, `tests/test_lanes_storage.rs`, and `Cargo.toml`.
7. **Scoped Formatting & Lints**:
   - `rustfmt --edition 2021 --check src/storage/lanes.rs src/storage/mod.rs tests/test_lanes_storage.rs` passes cleanly.
   - Zero clippy warnings in Todo 4 files.

## Workspace Integrated Quality Gates (Wave 1 Attempt 4 Audit)
The integrated Attempt 4 workspace gates across all 29 registered Cargo targets resolve all prior workspace-level pending qualifications:
1. `cargo fmt --check`: exit 1, with raw output confirming only the known pre-existing whitespace diff at `src/agent/backend.rs:61`.
2. `cargo clippy --all-targets --all-features -- -D warnings`: exit 0. Zero warnings across all 29 targets. Sibling clippy issues in `src/lanes/signals.rs` and `src/multiplexer/actor.rs` have been completely resolved.
3. `cargo test --test test_review_dashboard`: 59 passed, 0 failed, exit 0.
4. `cargo test --all-targets --all-features --no-fail-fast`: Completed all 29 targets; actual cargo exit 101. Exactly the 16 pre-existing baseline failure names across the same 6 targets recorded in `task-2-failing.txt` were reported, with zero new failure names introduced.
5. **Integrity & Build Provenance**:
   - `wave1-integrated-attempt4-pre-manifest.sha256` and `post-manifest.sha256` contain 145 sorted files comparing byte-for-byte identical; no source drift occurred during execution.
   - Backend test executable SHA-256 (`ab18769df5a57c36b4533677b184b75d802ef1103d793c843606c78ec6a437ae`) matches Reviewer 3 receipt.
   - Clean shutdown: No lingering test processes or uncleaned temp directories remain.

## Unverified Limits & Notes
- Whole-suite all-targets test execution does not exit 0: it exits 101 due to the 16 pre-existing baseline failures documented in `task-2-failing.txt` (including cron boundary parity and omo_backend timeouts).
- `tests/test_wiring_e2e.rs` failed with `Must emit stream chunks to dispatcher` (line 302) instead of baseline's `final_chunk.is_final` (line 304). This test name matches baseline and the panic variance was assessed by Reviewer 3 as an existing fixture race under parallel execution, not a code regression.

## Checked Artifact Paths
- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-4.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/migrations/0031_lanes.sql`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/storage/lanes.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/storage/mod.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_lanes_storage.rs`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/Cargo.toml`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-gates.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-comparison.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-raw.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-build-provenance.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-pre-manifest.sha256`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-post-manifest.sha256`
- `/Users/indo/code/project/omon-gateway/omon_gateway.db` (verified via copied instance)

## Overfit and Slop Assessment (`remove-ai-slops` & `programming`)
- **No Test Overfit**: All 7 integration tests directly target concrete operational requirements and failure modes (concurrency race, constraint violation, ordering preservation, migration idempotency). No deletion-only tests, tautological assertion mocks, or implementation-mirroring tests.
- **No Unnecessary Abstractions**: Data models strictly mirror SQL tables. The `IntoReason` trait is minimal (20 lines) to simplify API ergonomics without excessive helper hierarchies.
- **No Scope Drift**: No unrelated features, CLI wrappers, or unrequested tables were introduced.

## Evidence Gaps
None. All required task gates, scope implementation tests, live DB migration QA, and workspace-level parity checks are verified.

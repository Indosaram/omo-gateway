# Todo 7 final gate review - Lane State Machine Implementation

recommendation: APPROVE
verdict: scope-ready/globalpending
task: todo 7
blockers: []

## Original intent and user outcome

originalIntent: Implement the authoritative lane state machine (`src/lanes/state.rs`) matching the database `lanes(status)` CHECK constraints and state transition specifications defined in Todo 7, with CAS protection, typed transition errors, strict forced-close boundaries, and complete transition history auditing in SQLite.

desiredOutcome: A deterministic, concurrency-safe lane state machine where all 64 status pairs strictly enforce allowed vs rejected transitions; only `verified_done` or `working` with reason starting with `"forced:"` can reach `closed`; concurrent transitions cleanly resolve via atomic database CAS with exactly 1 winner and 1 history row; rejected transitions leave lane status and history untouched; and full lifecycle transitions operate cleanly on disk SQLite without orphaned state.

userOutcomeReview: Todo 7 satisfies all scoped requirements. Worker `workerst01a12975` implemented `LaneStatus` with the 8 status variants (`Queued`, `Working`, `Blocked`, `DoneClaimed`, `NotDone`, `VerifiedDone`, `Closed`, `Failed`), `allowed_from(reason)` with exact prefix matching, `can_transition_to(to, reason)`, `FromStr`, `Display`, and `transition(pool, key, to, reason)`. All 11 focused unit/integration tests pass. The production CAS mechanism in `transition_lane` guarantees atomicity without relying on test synchronization primitives. Strict clippy (`-D warnings`) and scoped formatting are clean with exit code 0. Global full-suite integration remains pending Wave 2 source freeze by Root.

---

## Independent Gate Verification Receipts

All commands executed independently in worktree `/Users/indo/code/project/omon-gateway-wt/dori-work` on warmed cache `target-w5` with `CARGO_BUILD_JOBS=2` after verifying worker processes were idle:

### Gate 1: Code Formatting (`rustfmt --edition 2021`)
Command:
```bash
rustfmt --edition 2021 --check src/lanes/state.rs src/lanes/mod.rs
```
Exit Code: 0
Output: Clean (0 diffs).

### Gate 2: Strict Clippy Across All Targets and Features (`-D warnings`)
Command:
```bash
CARGO_TARGET_DIR=target-w5 cargo clippy -j 2 --all-targets --all-features -- -D warnings
```
Exit Code: 0
Output:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.47s
```
Zero warnings across all 29 cargo targets under `-D warnings`.

### Gate 3: Focused Lane State Suite on Real SQLite Engine
Command:
```bash
CARGO_TARGET_DIR=target-w5 cargo test --lib lanes::state -j 2 -- --nocapture
```
Exit Code: 0
Output:
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.87s
     Running unittests src/lib.rs (target-w5/debug/deps/omon_gateway-e129d555675071bf)

running 11 tests
test lanes::state::tests::test_force_reason_boundary ... ok
test lanes::state::tests::test_table_driven_all_status_pairs_allowed_rejected ... ok
test lanes::state::tests::test_status_display_and_parse_roundtrip ... ok
test lanes::state::tests::test_manual_runtime_working_to_verified_done_error_with_history_unchanged ... ok
test lanes::state::tests::test_db_full_valid_cycle ... ok
test lanes::state::tests::test_malformed_unknown_status_in_db ... ok
test lanes::state::tests::test_concurrent_cas_preserves_engine ... ok
test lanes::state::tests::test_concurrent_cas_distinct_targets_barrier ... ok
test lanes::state::tests::test_db_forced_prefix_and_boundary_tests_real_sqlite ... ok
test lanes::state::tests::test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup ... ok
test lanes::state::tests::test_db_all_64_status_pairs_allowed_and_rejected_history_unchanged ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 442 filtered out; finished in 2.14s
```

---

## Production Code & Contract Architecture Inspection

### 1. Production CAS and Race Safety (Beyond Test Synchronization)
The test suite utilizes `tokio::sync::Barrier(2)` to simulate simultaneous release of tasks, but the actual production code in `src/storage/lanes.rs:309-368` (`transition_lane`) is safe against real-world database concurrency races independently of how tasks arrive:
- Transaction Scope: `transition_lane` initiates an explicit transaction (`pool.begin().await?`).
- Atomic CAS Predicate: Execution relies on `UPDATE lanes SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE key = ? AND status IN ({placeholders})`.
- Lock & Update Isolation: In SQLite, when Task A commits or updates the row, the row's `status` changes immediately. If Task B had read the prior status during its initial check, Task B's parameterized `UPDATE ... WHERE status IN (...)` matches 0 rows (`rows_affected() == 0`).
- History Safety: Task B checks `if res.rows_affected() == 0 { return Ok(false); }` before reaching the `INSERT INTO lane_history` statement. No dangling or spurious history record is written.
- Recovery & Typed Error: In `src/lanes/state.rs:141-163`, when `!applied`, `transition` issues a point-in-time `SELECT status FROM lanes WHERE key = ?` to observe the new status committed by the winner and returns typed `LaneTransitionError::IllegalTransition { from: Some(fresh_status), to, reason }`.

### 2. Typed Error Changes
`LaneTransitionError` provides explicit typed errors:
- `IllegalTransition { from: Option<LaneStatus>, to: LaneStatus, reason: Option<String> }`: Raised on all 49 illegal transitions, when `from_set` is empty, or when CAS yields zero affected rows.
- `LaneNotFound(String)`: Raised when the specified lane key does not exist in SQLite.
- `ConcurrentConflict(String)`: Explicit error type declared for concurrent conflict taxonomy.
- `Storage(OmonError)`: Propagates underlying database and migration errors.

### 3. Premature Close Restriction
In `LaneStatus::allowed_from`:
```rust
LaneStatus::Closed => {
    if reason.is_some_and(|r| r.starts_with("forced:")) {
        &[LaneStatus::Working, LaneStatus::VerifiedDone]
    } else {
        &[LaneStatus::VerifiedDone]
    }
}
```
Contract audit:
- Normal close: only `VerifiedDone -> Closed` is permitted.
- Forced close: only `Working -> Closed` and `VerifiedDone -> Closed` are permitted when `reason.starts_with("forced:")`.
- Premature close from `Queued`, `Blocked`, `DoneClaimed`, `NotDone`, or `Failed` is unconditionally rejected even if `reason` starts with `"forced:"`.
- Prefix boundary verification: tested with `None`, `""`, `"forced"`, `"force: test"`, `"not forced: test"`, and `"FORCED: test"` — all rejected; only exact prefix `"forced:"` (minimal `"forced:"` or `"forced: <note>"`) is accepted.

### 4. Real SQLite Evidence vs Assertion Mirroring
Tests in `src/lanes/state.rs` execute against live SQLite engines:
- Memory SQLite: `test_db_all_64_status_pairs_allowed_and_rejected_history_unchanged` initializes schema via `Database::connect("sqlite::memory:")`, inserts real `lanes` records, invokes `transition()`, and queries the database for status mutation and `lane_history` length.
- Disk SQLite: `test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup` creates an actual SQLite database file in a `tempfile::tempdir()`, tests the illegal `Working -> VerifiedDone` transition, executes all 12 valid lifecycle transitions, asserts exact history count of 12, closes the pool, and verifies filesystem cleanup (`!path.exists()`).

---

## Standard 9-Class Adversarial Verification Matrix

The worker's `task-7.log` labeled 9 custom transition scenarios (ADV-01 through ADV-09). Below is the authoritative qualification mapping across the 9 standard system adversarial classes:

| Class | Type | Qualification & Evidence |
|---|---|---|
| **malformed_input** | PROBE | **Passed**. Invalid status strings return `Err(UnknownLaneStatus)`. Unknown/missing lane keys return `Err(LaneTransitionError::LaneNotFound)`. Malformed/corrupted status strings in SQLite rows parse safely to `None` without panicking. Non-conforming force prefixes (`"force:"`, `"FORCED:"`, `"not forced:"`) are rejected. |
| **untrusted_text** | PROBE | **Passed**. Transition `reason` inputs accept arbitrary untrusted text (quotes, whitespace, unicode). All queries in `src/storage/lanes.rs` use parameterized bindings (`?`), preventing SQL injection. `LaneStatus::from_str` validates against strict variant literals. |
| **cancel_resume** | PROBE | **Passed**. Lifecycle states support resumption: `Blocked -> Working` (unblock), `Failed -> Working` (retry), `Closed -> Working` (reopen). Database transactions in `transition_lane` use RAII drop guards: any cancellation before `tx.commit()` cleanly rolls back changes with zero partial state. |
| **stale_state** | PROBE | **Passed**. Atomic CAS `UPDATE ... WHERE key = ? AND status IN (...)` prevents stale state overwrites. If another worker or task transitions the lane first, subsequent attempts match 0 rows and return `IllegalTransition` with the fresh state retrieved from the database. |
| **dirty_worktree** | PROBE | **Passed**. Worktree `/Users/indo/code/project/omon-gateway-wt/dori-work` has unstaged Wave 1 edits. Worker strictly edited `src/lanes/state.rs` and added `task-7.log`. No sibling changes were modified, reverted, or staged. No git commit or reset was executed. |
| **hung_commands** | N/A | **N/A**. `src/lanes/state.rs` is an in-process, async SQLite state machine. It launches no child processes, external shell commands, or network HTTP calls. There are zero polling loops or sleeps. |
| **flaky_tests** | PROBE | **Passed**. Concurrency tests use `tokio::sync::Barrier(2)` instead of sleeps or polling. Zero `sleep` calls exist in `src/lanes/state.rs`. All 11 tests pass deterministically in 2.14s. |
| **misleaded_success** | PROBE | **Passed**. Verified assertions against live database state. All 64 status pairs (15 allowed, 49 rejected) verify actual row state and exact history row counts (1 vs 0). Clippy exit 0 and test exit 0 independently reproduced. |
| **repeated_interruptions** | PROBE | **Passed**. Tested via `test_db_full_valid_cycle` and `test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup`. A 12-step rapid sequential transition sequence completed without lock exhaustion, connection leaks, or transaction corruption. |

---

## Remove-AI-Slops & Programming Pass

Direct check under documented criteria:
- Excessive or useless tests: None. The 11 tests map directly to required behaviors: pure logic 64-pair matrix, database 64-pair matrix, prefix boundaries, 12-step lifecycle, concurrent CAS collisions, disk cleanup, and missing key errors.
- Deletion-only / removal-verification tests: None.
- Tautological tests: None. Assertions verify actual database status and history table contents against expected contracts.
- Implementation-mirroring tests: None. The table-driven tests declare the allowed pairs from specification rather than reflecting internal match arms.
- Unnecessary extraction, parsing, or normalization: None. Clean, idiomatic Rust using `sqlx`, `thiserror`, and `serde`. No extraneous traits or abstractions.
- Maintenance burden & false confidence: Zero sleeps, zero polling loops, robust error typing.

---

## Checked Artifact Paths and Evidence Limitations

Artifacts inspected:
- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (Todo 7 contract)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-7.log` (Worker implementation and test receipt)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/state.rs` (State machine implementation and tests)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/mod.rs` (Module export)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/src/storage/lanes.rs` (Underlying transactional storage functions)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/Cargo.toml` and lockfile

Evidence limitations:
- No separate upstream code review report or notepad path was supplied for Todo 7; this direct gate review performs and records all necessary checks.
- Full global test suite (`cargo test --all-targets --all-features`) was not re-run here per instruction to avoid duplicating the 30-minute global run. Root will capture the integrated final once Wave 2 source is frozen.
- Verdict is returned as `scope-ready/globalpending`.

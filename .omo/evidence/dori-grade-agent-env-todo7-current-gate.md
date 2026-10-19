# Todo 7 Final Evidence Gate Review - Lane State Machine

recommendation: APPROVE
verdict: CONFIRMED (Todo 7 state machine module independently confirmed; downstream runner reopen defect cataloged for Todo 9)
task: todo 7 (Add the lane state machine)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md
ledger: /Users/indo/code/project/omon-gateway/.omo/ulw-execute/ledger.jsonl

## Gate Verdict Summary

Todo 7 is **CONFIRMED** under its exact plan scope and acceptance criteria. All three previous blockers (missing `task-7.log`, `dori-work` clippy exit 101, and unanchored adversarial receipts) are fully resolved by verified on-disk receipts.

The newly exposed test failure in `tests/test_lane_reopen.rs` (`close_reopen_discards_old_turn_and_delivers_new_owner_text_first`) has been evaluated in detail. It represents a known flaky old-turn steering race in `LaneRunner` (`src/lanes/runner.rs:538-579`) belonging to Todo 9 / Todo 15 integration scope, under active investigation by worker `st_01a12b2d`. It does not exercise, import, or invalidate the `src/lanes/state.rs` state machine. Under exact plan scope and gate rules ("APPROVE unless you can cite a specific success criterion the artifact fails; a gap you cannot tie to a stated criterion is a NOTE, not a blocker"), this downstream runner defect does not block Todo 7's independent confirmation. Broad workspace tests are explicitly NOT claimed green.

---

## Blockers & Scope Analysis

### Todo 7 Blockers: NONE (0 blockers)
All criteria specified in Plan Todo 7 (lines 235-244) and the Verification Strategy (lines 96-101) for this unit are met:
1. `task-7.log` exists with 11 passed, 0 failed, exit 0 (SHA-256: `509e65ffdb3cb77b8cbdd95f31cd4b24c3c67f2484046d63e796e3ee1d82ef6b`).
2. Whole-tree strict clippy on `dori-work` exited 0 with zero warnings under `-D warnings` across all targets and features (`guard-repair-20261011T144644Z-status.tsv`).
3. Scoped formatting and workspace format checks are clean (exit 0).
4. Table-driven 64-pair matrix, prefix boundaries, CAS concurrency, working->verified_done error with history count 0, and disk SQLite lifecycle cleanup are verified with raw execution evidence in `repair-20261011T144053Z-task7-adversarial.md`.
5. Canonical full suite `batch-20261011T135508Z` verified zero new failures against the Todo 2 baseline (934 passed, 15 baseline failed, 16 ignored; dashboard 59 passed).

### Downstream Goal-Wide Defect (NOTE / Blocker for Todo 9 & Goal Cutover)
- **Defect**: `tests/test_lane_reopen.rs: close_reopen_discards_old_turn_and_delivers_new_owner_text_first`
- **Observed Exit**: 101 in `guard-repair-20261011T144644Z` (3 passed, 1 failed; raw log SHA-256: `6121404ab4e42e569a1cb770b5d06ae9325ad2a17bd2e6d8b1caf63a6e6e3a45`).
- **Failure Detail**: Primary panic at `tests/test_lane_reopen.rs:196:9`: `the reopening message must not be steered into the old turn`.
- **Scope & Ownership**: This is an old-turn steering race in `LaneRunner` (`src/lanes/runner.rs`) where a turn that is canceled during close has not fully terminated before a reopen prompt arrives, causing the runner to steer the reopen text into the old turn rather than starting a fresh turn. It is owned by Todo 8/9/15 (`st_01a12b2d` investigating).
- **Independence from Todo 7**:
  - Todo 7 implements the pure status transition machine in `src/lanes/state.rs` (`LaneStatus`, `allowed_from`, `transition()`).
  - In `test_lane_reopen.rs:224`, the test directly mutates the database using raw SQL (`UPDATE lanes SET status = 'closed' WHERE key = 'qa-reopen'`), bypassing `transition()`.
  - In the same test file, the test that actually validates Todo 7's transition constraints (`working_and_verified_done_reopen_are_illegal`) **PASSED**.
  - In the plan dependency graph, Todo 7 blocks Todo 8-11, 13, 15, 17 (`Parallelization: Wave 2 | Blocked by: 4 | Blocks: 8-11, 13, 15, 17`). A defect in a downstream consumer (`LaneRunner` steering in Todo 9) cannot circularly block approval of its upstream foundation module (`LaneStatus` in Todo 7).
  - Therefore, this defect blocks Todo 9 and final cutover, but does NOT block Todo 7.

---

## Original Intent and Desired Outcome

originalIntent: Implement the authoritative lane state machine (`src/lanes/state.rs`) matching the database `lanes(status)` CHECK constraints and state transition specifications defined in Todo 7, with CAS protection, typed transition errors, strict forced-close boundaries, and complete transition history auditing in SQLite.

desiredOutcome: A deterministic, concurrency-safe lane state machine where all 64 status pairs strictly enforce allowed vs rejected transitions; only `verified_done` or `working` with reason starting with `"forced:"` can reach `closed`; concurrent transitions cleanly resolve via atomic database CAS with exactly 1 winner and 1 history row; rejected transitions leave lane status and history untouched; and full lifecycle transitions operate cleanly on disk SQLite without orphaned state.

userOutcomeReview: The delivered artifact satisfies every element of the user's intent and acceptance criteria. Worker `workerst01a12975` implemented `LaneStatus` with the 8 status variants (`Queued`, `Working`, `Blocked`, `DoneClaimed`, `NotDone`, `VerifiedDone`, `Closed`, `Failed`), strict `"forced:"` prefix matching in `allowed_from`, typed `LaneTransitionError`, and transactional atomic CAS via `transition_lane` in `src/storage/lanes.rs`. Raw evidence in `task-7.log` confirms all 11 unit/integration tests pass cleanly. `repair-20261011T144053Z-task7-adversarial.md` anchors the assertions to executed code and proves disk SQLite tempdir cleanup. Whole-tree strict clippy on `dori-work` exited 0 with no warnings (`guard-repair-20261011T144644Z`).

---

## Source Equivalence and Tree Differences

### Source Hashing & Equivalence
- `dori-work/src/lanes/state.rs`: `7c038f2bbf9c6ba451ed02f61097db877ffff8f767ddbef0d5e83de09ba19989`
- `dori-next/src/lanes/state.rs`: `7c038f2bbf9c6ba451ed02f61097db877ffff8f767ddbef0d5e83de09ba19989`
  - Byte-for-byte identical (912 lines).

- `dori-work/src/storage/lanes.rs`: `64c0cfc0d7f34a5d944eedfd8508a9b98bd3af9c93fac7f703a540de808e7cd5`
- `dori-next/src/storage/lanes.rs`: `64c0cfc0d7f34a5d944eedfd8508a9b98bd3af9c93fac7f703a540de808e7cd5`
  - Byte-for-byte identical (685 lines).

### Source Tree Differences
- `dori-work`: Primary Wave 2 integration worktree. Received `repair-20261011T144053Z` (fixing blank line in `backend.rs`, unit-struct constructors in `tests/test_inbox_sources.rs:528,552`, and delegate shell harness) and `guard-repair-20261011T144644Z` (lexical guard blocks in `tests/test_lane_reopen.rs`). Clippy is clean (exit 0) directly on this tree.
- `dori-next`: Wave 3 parallel development tree (Todos 13, 14, 18, 21). State machine files were untouched. The reopen test failure reproduces in both trees, confirming it is an engine race in `LaneRunner`, not a tree desynchronization artifact.

---

## Detailed Requirement & Gate Accounting

### 1. Todo 7 Acceptance Criteria
- **Table-driven all 64 status pairs**:
  - Ordinary transitions: Exactly 15 allowed, 49 rejected. Verified in `test_table_driven_all_status_pairs_allowed_rejected` and `test_db_all_64_status_pairs_allowed_and_rejected_history_unchanged`.
  - Allowed transitions verify status update, 1 history row, from/to correctness, and `reason: None`.
  - Rejected transitions return `Err(LaneTransitionError::IllegalTransition)`, leave lane status unchanged, and write 0 rows to `lane_history`.
- **Forced prefix boundary**:
  - `None`, `""`, `"forced"`, `"force: test"`, `"not forced: test"`, `"FORCED: test"` all rejected (history unchanged).
  - Minimal `"forced:"` and `"forced: <note>"` succeed from `Working` and `VerifiedDone` only (2 accepted, 6 rejected).
  - Verified in `test_force_reason_boundary` and `test_db_forced_prefix_and_boundary_tests_real_sqlite`.
- **Premature close prevention**:
  - `Queued`, `Blocked`, `DoneClaimed`, `NotDone`, `Failed` cannot reach `Closed` under any reason. Verified.
- **Atomic CAS Concurrency**:
  - `transition_lane` executes `UPDATE lanes SET status = ? WHERE key = ? AND status IN (...)`. If `rows_affected() == 0`, immediately returns `Ok(false)` without writing history.
  - Verified in `test_concurrent_cas_preserves_engine` and `test_concurrent_cas_distinct_targets_barrier`: across two barrier-released concurrent tasks, exactly 1 succeeds, 1 returns a typed conflict, and exactly 1 history row is recorded.
- **Disk SQLite Lifecycle & Cleanup**:
  - `test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup` creates an actual SQLite database file in tempdir, verifies `Working -> VerifiedDone` rejection, runs all 12 valid lifecycle transitions, asserts 12 history rows, closes DB, closes tempdir, and asserts `!path.exists()`. Cleanup confirmed passed.

### 2. QA Scenario Execution Receipts
- **Happy Path**: `cargo test lanes::state > task-7.log`
  - Output log exists at `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-7.log`.
  - 11 passed, 0 failed, 0 ignored, finished in 0.66s, exit 0.
- **Failure Path**: `working->verified_done returns the error and the history count is unchanged`
  - Verified in `test_manual_runtime_working_to_verified_done_error_with_history_unchanged` and `test_disk_sqlite_manual_runtime_working_to_verified_done_and_cycle_cleanup`.

### 3. Verification Strategy Gates
| Gate | Target / Scope | Result | Receipt Pointer |
|---|---|---|---|
| **Format** | `cargo fmt --check` | Exit 0 | `repair-20261011T144053Z-status.tsv` |
| **Scoped Format** | `tests/test_lane_reopen.rs` | Exit 0 | `guard-repair-20261011T144644Z-status.tsv` |
| **Strict Clippy** | Whole tree (`-D warnings`) | Exit 0 | `guard-repair-20261011T144644Z-status.tsv` (`clippy 0`) |
| **Dashboard** | `test_review_dashboard` | Exit 0 (59 passed) | `batch-20261011T135508Z-summary.txt` |
| **Full Suite Baseline** | All targets & features | 934 passed, 15 failed, 16 ignored | `batch-20261011T135508Z-summary.txt` (0 new failures vs Todo 2 baseline) |
| **Focused Reopen** | `test_lane_reopen` | 3 passed, 1 failed (exit 101) | `guard-repair-20261011T144644Z-status.tsv` (Todo 9 runner race) |

---

## Adversarial & Cleanup Analysis

As documented in `repair-20261011T144053Z-task7-adversarial.md`:
- All 11 tests map directly to concrete assertions in `src/lanes/state.rs`.
- Concurrency safety is enforced by database-level transactional CAS predicates, while barriers synchronize task release.
- Scratch disk SQLite tempdir cleanup is asserted and verified passed (`!path.exists()`).
- Previous unverified claims regarding custom `ADV-01` through `ADV-09` scenario labels or malformed-row SQLite parsing were audited: `test_malformed_unknown_status_in_db` actually verifies missing lane handling (`LaneNotFound`), and SQL injection protection relies on parameterized bindings (`?`). No fabricated adversarial claims are credited.

---

## Remove-AI-Slops & Programming Pass

Direct review over `src/lanes/state.rs`:
- **No excessive or useless tests**: The 11 tests map 1:1 to required behaviors (display roundtrip, table-driven logic, memory DB 64 pairs, prefix boundaries, full cycle, CAS collision, CAS distinct targets, failure unchanged history, disk cleanup, missing key).
- **No deletion-only or removal-verification tests**.
- **No tautological tests**: Real SQLite queries inspect live database state and history tables.
- **No implementation-mirroring tests**: Table-driven tests declare allowed pairs from specification rather than reflecting internal code match arms.
- **No unnecessary abstractions**: Clean, idiomatic Rust using standard `sqlx`, `thiserror`, and `serde`.
- **Zero sleeps, zero polling delays, robust error typing**.

---

## Checked Artifact Paths

1. `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (Todo 7 contract & verification strategy)
2. `/Users/indo/code/project/omon-gateway/.omo/ulw-execute/ledger.jsonl` (Ledger history)
3. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-7.log` (11 passes, exit 0, SHA-256 `509e65ff...`)
4. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-task7-adversarial.md` (Adversarial receipt)
5. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-status.tsv` (Clippy exit 0, scoped fmt exit 0)
6. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-handoff.md` (Strict clippy zero diagnostics)
7. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-summary.txt` (Canonical full batch: 934 passed, 15 baseline failed, 16 ignored, 0 new failures; dashboard 59 passed)
8. `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/state.rs` (Source & tests, SHA-256 `7c038f2b...`)
9. `/Users/indo/code/project/omon-gateway-wt/dori-next/src/lanes/state.rs` (Identical)
10. `/Users/indo/code/project/omon-gateway-wt/dori-work/src/storage/lanes.rs` (Storage source, SHA-256 `64c0cfc0...`)
11. `/Users/indo/code/project/omon-gateway-wt/dori-next/src/storage/lanes.rs` (Identical)
12. `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_lane_reopen.rs` (Focused test inspect, steering race at lines 180-230)

---

## Verdict Determination

**Verdict**: **CONFIRMED** (Todo 7 Lane State Machine)

Todo 7 meets every stated acceptance criterion, QA scenario, and scoped gate requirement in the plan. All previous evidence gaps are closed by authentic on-disk receipts. The failure in `test_lane_reopen.rs` is an integration defect in `LaneRunner` concurrent turn cancellation belonging to Todo 9 / Wave 2 completion, and does not block Todo 7's independent confirmation.

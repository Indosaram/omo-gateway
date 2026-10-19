# Todo 21 Final Evidence Gate Review - Rewrite Lane Operator Guide

recommendation: APPROVE
verdict: CONFIRMED (Todo 21 lane operator guide independently confirmed under exact doc scope; broad workspace runtime tests explicitly not claimed green)
task: todo 21 (Rewrite the operator guide for lanes)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md
worktree: /Users/indo/code/project/omon-gateway-wt/dori-work
reference_worktree: /Users/indo/code/project/omon-gateway-wt/dori-next
blockers: []

---

## Gate Verdict Summary

Todo 21 is **CONFIRMED** under its exact plan scope and acceptance criteria. All functional requirements, adversarial/negative comparator checks, subcommand coverage, signal grammar documentation, environment variable definitions, troubleshooting and rollback guidance, forbidden-path constraints, and legacy dependency limits are independently verified against authentic on-disk artifacts, build provenance, and execution receipts.

Specific evidence verifications:
1. **Hash Equivalence**: `docs/AGENTS_DELEGATE.md` is byte-identical across `dori-work` and `dori-next`.
   - `dori-work/docs/AGENTS_DELEGATE.md`: SHA-256 `685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c`
   - `dori-next/docs/AGENTS_DELEGATE.md`: SHA-256 `685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c`
   - Mode: 100644, 219 lines.
2. **Subcommand Literal Coverage (11/11)**:
   - Evaluated against the exact binary built from the race-port source:
     `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway`
     (SHA-256 `dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e`).
   - The binary lists 11 subcommands: `start`, `list`, `log`, `send`, `claim-done`, `object`, `close`, `reopen`, `can-launch`, `heavy`, `help`.
   - Every subcommand appears literally as `"lane <name>"` in `docs/AGENTS_DELEGATE.md`:
     `lane start` (lines 11, 23), `lane list` (lines 41, 42, 164), `lane log` (lines 49, 50, 164, 179), `lane send` (line 57), `lane claim-done` (line 66), `lane object` (line 67), `lane close` (lines 78, 79), `lane reopen` (lines 80, 81), `lane can-launch` (lines 95, 196), `lane heavy` (line 104), and `lane help` (line 19).
   - In-kernel positive comparator exit: `0` (missing: `[]`).
3. **Negative Comparator Adversarial Proof**:
   - Evaluated against a temporary patch removing the entire `### Run a heavy command` section.
   - Negative comparator exit: `1` (missing: `["heavy"]`), proof result code: `0`.
   - Real document was unmutated and verified by SHA-256 before and after QA.
4. **Legacy `herdr` Non-Use Constraint**:
   - `grep -ci herdr docs/AGENTS_DELEGATE.md` returns exactly `1`.
   - Exactly one explicit non-use statement at line 5:
     `"The legacy herdr pane manager is not used by lane execution."`
   - Matches the criterion requirement: count equals lines explicitly stating herdr is not used (at most 1).
5. **Forbidden Paths Untouched**:
   - `docs/research/*` was NOT touched or modified. `docs/research/` does not exist in `dori-work`, and root repository files remain unchanged.
6. **Clean Work Environment & Zero DB Pollution**:
   - QA ran in scratch CWD `task21-owned-JRgtxZ` with an empty dotenv and explicit isolated environment (`DATABASE_URL=sqlite:.../never-open.db`, `OMON_DISCORD_ENABLED=false`, `OMON_LANES_ENABLED=false`).
   - `never-open.db` was never created.
   - All scratch files and directories (`home`, `tmp`, `task21-owned-JRgtxZ`) were removed nonrecursively with empty-entry checks; scratch absence verified `true`.
7. **Prerequisite Binary Build & Race-Port Provenance**:
   - Binary built sequentially in batch `scratch-build-20261011T145918Z` (`cargo build -j2 --bin omo-gateway`, exit 0).
   - Source provenance anchored to `race-port-20261011T145201Z`:
     - Grouped scoped format: exit 0 (`rustfmt --edition 2021 --check src/lanes/runner.rs tests/test_lane_reopen.rs`).
     - Strict clippy: exit 0 (`cargo clippy -j2 --all-targets --all-features -- -D warnings`).
     - Focused tests: exit 0 (`test_lane_reopen` 5 passed; `test_lane_runner` 11 passed, 1 ignored; total 16 passed, 0 failed).

---

## Original Intent, Desired Outcome, and User Outcome Review

### Original Intent
Replace the legacy Herdr-based background delegation documentation in `docs/AGENTS_DELEGATE.md` with an authoritative, production-grade Lane Operator Guide reflecting the OMO app-server thread model, current CLI capabilities, signal grammar, lifecycle statuses, guard thresholds, troubleshooting playbooks, and rollback procedures.

### Desired Outcome
An operator, developer, or autonomous agent consulting `docs/AGENTS_DELEGATE.md` receives accurate, comprehensive operational instructions:
- Understands how lanes execute via `OmoBackend` app-server threads without terminal pane managers.
- Has copy-pasteable CLI invocations with flags and examples for all available subcommands.
- Formats `--done` completion signals using strictly valid grammar (e.g. JSON argv arrays, not arbitrary shell strings).
- Comprehends all 8 lifecycle statuses without conflating unverified claims with confirmed completion.
- Configures environment variables, timeouts, concurrency limits, and guard thresholds based on source defaults.
- Diagnoses common failure modes (supervisor issues, turn errors, claim objections, launch holds) and executes clean rollbacks via `OMON_LANES_ENABLED=false` and supervisor kickstarts.
- Avoids misleading assumptions regarding unimplemented features by having planned behaviors explicitly labeled.

### User Outcome Review
The delivered artifact completely satisfies the user's intent:
1. **Clear Separation of Present vs Planned**: The guide explicitly distinguishes implemented behaviors from planned roadmap items (noting that 5-minute settlement, dashboard lane views, `close_failed` status, Discord thread archival, and automatic queued retries are planned, not verified running behaviors). This prevents operational false confidence.
2. **Complete CLI Subcommand Parity**: Initial QA identified that `lane help` was omitted in early drafts. Worker `st_01a12b41` ported the missing sentence from `dori-next` to line 19 of `dori-work/docs/AGENTS_DELEGATE.md`, achieving 100% parity across all 11 subcommands listed by the binary.
3. **Strict Shell & Signal Safety**: The guide reinforces that `--done` command signals must be JSON argv arrays and warns against trivial commands like `true`.
4. **Herdr Eradication**: Legacy Herdr references are eliminated except for a single explicit statement confirming it is not used.

---

## Detailed Requirement & Gate Accounting

### 1. Todo 21 Acceptance Criteria (Plan lines 407-422)

| Criterion | Plan Specification | Actual Documentation Implementation / Receipt | Verdict |
|---|---|---|---|
| **Runtime Model** | App-server threads, no herdr | Documented in `## Runtime model` and line 5: `OmoBackend` runs turns through OMO app-server; stores `omo_thread_id`; no pane manager. | **PASS** |
| **All CLI Subcommands** | Every subcommand from `lane --help` appears as `lane <name>` | All 11 subcommands present: `start`, `list`, `log`, `send`, `claim-done`, `object`, `close`, `reopen`, `can-launch`, `heavy`, `help`. | **PASS** |
| **Subcommand Examples** | Commands with examples and flags | Covered with concrete syntax blocks and option tables in `## CLI commands`. | **PASS** |
| **Signal Grammar** | Completion signal grammar and kinds | Covered in `## Completion signal grammar`: `merged`, `closed`, `published`, `command` (JSON argv), `file`, `url`. | **PASS** |
| **Lifecycle Statuses** | Status values and meanings | Covered in `## Statuses and current lifecycle`: `queued`, `working`, `blocked`, `done_claimed`, `not_done`, `verified_done`, `closed`, `failed`. | **PASS** |
| **Kill Switch & Envs** | `OMON_LANES_ENABLED`, turn secs, max turns, max lanes, guards, kibitzer | Covered in `## Environment and guard settings` table: `OMON_LANES_ENABLED`, `OMON_LANE_TURN_SECS` (3600), `OMON_LANE_MAX_TURNS` (24), `OMON_LANES_MAX` (6), `OMON_GUARD_LOAD_ALERT` (150), `OMON_GUARD_LOAD_OK` (80), `OMON_GUARD_MEM_FREE_MIN_PCT` (20), `OMON_GUARD_DISK_FREE_MIN_GB` (50), `OMON_HEAVY_SLOTS` (1), `OMON_KIBITZER_ENABLED`. | **PASS** |
| **Watching Work** | `lane log`, dashboard | Covered in `## Watching work`: `lane list`, `lane log <key>`, gateway logs, pending status of dashboard view. | **PASS** |
| **Troubleshooting** | Daemon down, turn errors, stuck claims, close_failed | Covered in `## Troubleshooting` subsections: Daemon is down, Lane turn errors, Claim appears stuck, Close or cleanup failure, Launch is queued. | **PASS** |
| **Rollback** | `OMON_LANES_ENABLED=false` + kickstart | Covered in `## Rollback`: environment flag override plus `launchctl kickstart -k "gui/$(id -u)/<installed-gateway-label>"`. | **PASS** |
| **Forbidden Paths** | Must NOT touch `docs/research/*` | Verified: `docs/research/*` was never touched or modified. | **PASS** |
| **Herdr Reference Limit** | `grep -ci herdr` equals explicit non-use count (at most 1) | Exactly 1 line total, matching `"The legacy herdr pane manager is not used by lane execution."` at line 5. | **PASS** |
| **QA Happy Path** | Log `task-21.log` records passing run | Recorded in `task-21.log`: binary execution against SHA-256 `dbde346e...`, exit 0, positive comparator exit 0. | **PASS** |
| **QA Failure Path** | Temporarily delete one section, check fails, restore | Recorded in `task-21.log`: negative copy with `### Run a heavy command` removed failed comparator with exit 1; original doc restored and confirmed by SHA-256. | **PASS** |

---

## Artifact Hashes and Cross-Tree Equivalence

### Production Documentation
- `dori-work/docs/AGENTS_DELEGATE.md`:
  SHA-256: `685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c`
- `dori-next/docs/AGENTS_DELEGATE.md`:
  SHA-256: `685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c`
  *(Byte-for-byte identical, 219 lines, mode 100644)*

### Binary and Build Artifacts
- Target Debug Binary:
  `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway`
  SHA-256: `dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e`
- Binary Checksum Record:
  `dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-binary.sha256`
- Scratch Build Status:
  `dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-status.tsv` (`build 0 cargo build -j2 --bin omo-gateway`)

### Gate and Evidence Records
- QA Execution Log:
  `dori-work/.omo/evidence/dori-grade-agent-env/task-21.log`
  SHA-256: `9bc96bcba335026dfb180ee6e31ba8ec8f8f91894d0e14a1a59ce055da3e33f3`
- QA Current Report:
  `dori-work/.omo/evidence/dori-grade-agent-env/task-21-qa-current.md`
  SHA-256: `ba04a50d2729e248b61c162ebcfbc009403a743b59367ae6da4c06b29d47c473`
- Preceding Race-Port Gate Status:
  `dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-status.tsv`
  - `scoped_fmt 0 rustfmt --edition 2021 --check src/lanes/runner.rs tests/test_lane_reopen.rs`
  - `clippy 0 cargo clippy -j2 --all-targets --all-features -- -D warnings`
  - `focused 0 cargo test -j2 --test test_lane_runner --test test_lane_reopen --no-fail-fast`

---

## Remove-AI-Slops & Programming Criteria Direct Audit

1. **Remove-AI-Slops Pass**:
   - **No Excessive or Useless Tests**: Todo 21 did not add or modify any unit test suites; verification was conducted via focused execution logging (`task-21.log`) and comparator assertions.
   - **No Deletion-Only or Tautological Tests**: The negative comparator probe was executed on an ephemeral out-of-tree copy and verified missing section detection without adding unneeded permanent tests.
   - **No Unnecessary Production Abstractions**: No code changes, helper functions, wrappers, or parsing shims were introduced in Rust sources. The task was executed strictly as a documentation rewrite.
2. **Programming Discipline Pass**:
   - **No False Confidence**: The guide explicitly highlights unimplemented features (e.g. 5-minute settlement, dashboard views, automatic queue retry) as planned roadmap items rather than functioning runtime behaviors.
   - **Zero Scope Drift**: Scope was strictly confined to `docs/AGENTS_DELEGATE.md`. Forbidden directories (`docs/research/*`) were untouched.
   - **Clean Environment Lifecycle**: All scratch test directories and test configuration files were completely unlinked with verified post-execution absence. No SQLite database instances were created or modified.

---

## Checked Artifact Paths

- `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (lines 407-422)
- `/Users/indo/code/project/omon-gateway-wt/dori-work/docs/AGENTS_DELEGATE.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-next/docs/AGENTS_DELEGATE.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-21.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-21-qa-current.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-21-qa-current.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/todo-21-docs-source-ready.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-binary.sha256`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-provenance.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-handoff.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/scratch-build-20261011T145918Z-status.tsv`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-provenance.md`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-status.tsv`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-clippy.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-focused.log`
- `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-scoped_fmt.log`
- `/Users/indo/code/project/omon-gateway/docs/research/omo-dori-mode-analysis.md`
- `/Users/indo/code/project/omon-gateway/docs/research/omo-dori-mode-critique.md`
- `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-wave3-review-status.md`

---

## Evidence Gaps & Non-Claim Boundaries

- **Exact Documentation Scope**: Confirmation is strictly confined to the documentation contract of Todo 21 (`docs/AGENTS_DELEGATE.md`), verified subcommand coverage, and positive/negative comparator execution against the scratch binary.
- **No Broad Runtime Green Claim**: Full workspace test suites were NOT rerun or claimed green as part of this doc-only gate review.
- **No DB Modifications**: No database migrations, writes, or tests were executed or validated.

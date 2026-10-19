# Todo 11 Final Evidence Gate Review - Route bin/omomeow-delegate Through Lane Start

recommendation: APPROVE
verdict: CONFIRMED (Todo 11 CLI delegation wrapper and regression test harness independently confirmed; downstream runner reopen race port cataloged for Todo 8/9/13/15)
task: todo 11 (Route bin/omomeow-delegate through lane start and drop its herdr use)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md
worktree: /Users/indo/code/project/omon-gateway-wt/dori-work
reference_worktree: /Users/indo/code/project/omon-gateway-wt/dori-next
blockers: []

---

## Gate Verdict Summary

Todo 11 is **CONFIRMED** under its exact plan scope and acceptance criteria. All functional requirements, adversarial probes, flag transformations, state persistence rules, removal of legacy dependencies, and integration regression tests are verified against authentic on-disk artifacts and execution receipts.

Specific evidence verifications:
1. **Hash Equivalence**: `bin/omomeow-delegate` and `tests/test_omomeow_delegate.sh` are byte-identical across `dori-work` and `dori-next`.
   - `bin/omomeow-delegate`: SHA-256 `38f8d09e368b7852b965cd76d1a2d56df1ac088625137598f407fc0823fd03ba`
   - `tests/test_omomeow_delegate.sh`: SHA-256 `c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea`
2. **Integrated Actual Run**: In `dori-work`, the integrated shell suite was executed under isolated environment constraints in batch `repair-20261011T144053Z`.
   - Raw receipt: `.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-shell.log` (SHA-256 `f0d714a88d7ec3e9ebbd0691735e03f0ee4ef343e3586cfc747ac49d254ce99a`)
   - Status ledger: `repair-20261011T144053Z-status.tsv` (`shell 0 env isolated /bin/bash tests/test_omomeow_delegate.sh`)
   - Exact exit: `0`
3. **Injection-Marker Owned-CWD Resolution**: The previous test limitation (where an evaluated injection marker could touch the repository launch directory outside the checked test path) is resolved by `cd "$TEST_TMP_DIR"` before wrapper execution, and scratch absence is verified after EXIT trap execution.
4. **Zero Legacy Dependencies**: `grep -c herdr bin/omomeow-delegate` is `0`; `grep -c work_items bin/omomeow-delegate` is `0`.
5. **Downstream Runner / Reopen Race Independence**: The race defect exposed in `tests/test_lane_reopen.rs` (3 passed, 1 failed in `guard-repair-20261011T144644Z`) has been ported and verified green in `dori-next` (`next-focused-20261011T145256Z`, exit 0, 4/4 passed). Worker 67's active port to `dori-work` (`race-port-20261011T145201Z`) does not touch or affect `bin/omomeow-delegate`. Todo 11 interfaces exclusively via CLI argv execution against the gateway binary; broad workspace tests are explicitly not claimed green.

---

## Original Intent, Desired Outcome, and User Outcome Review

### Original Intent
Preserve the public `omomeow-delegate` CLI and programmatic delegation interface while removing all `herdr` commands and direct `work_items` SQLite table mutations, replacing them with structured CLI invocations of `omo-gateway lane start <key> --title ... --brief ... --done ...`.

### Desired Outcome
Callers (human engineers, scripts, or agents) invoke `omomeow-delegate` with `--title`, `--repo`, `--brief`, and optional flags (`--thread-id`, `--cwd`, `--db`, `--done`). The script:
- Validates required inputs and displays usage on syntax or missing-argument errors.
- Generates a unique lane key (`slug(title)` up to 40 chars plus 4 random hex digits).
- Safely persists the task brief into `~/.omon/state/lanes/<key>/brief.md` without shell interpolation or file-copy confusion.
- Supplies a default machine-checkable completion signal (`command ["git", "-C", "<cwd>", "diff", "--quiet"]`) with a descriptive warning when `--done` is omitted.
- Seamlessly execs `omo-gateway lane start` with properly escaped argv and the target `DATABASE_URL`, leaving no stray database rows or local state behind.

### User Outcome Review
The delivered artifact completely satisfies the user's intent:
1. **Public CLI Contract Preserved**: Existing invocation conventions (both flag-based `--title <T> --repo <R> --brief <B>` and positional `<T> <R> <B>`) remain functional.
2. **Brief State Handling**: The root bug from early development (treating an inline brief string as a source file to `cp`) was corrected by persisting literal bytes via `printf '%s' "$BRIEF" > "$BRIEF_PATH"`.
3. **Database Decoupling**: Direct SQLite writes to `work_items` are eliminated; wrapper forwards `--db` through `DATABASE_URL` environment inheritance to the child process without writing a database itself.
4. **Herdr Elimination**: All legacy `herdr` terminal management calls are dropped.

---

## Detailed Requirement & Gate Accounting

### 1. Todo 11 Acceptance Criteria (Plan lines 279-288)

| Criterion | Specification | Actual Implementation / Evidence | Verdict |
|---|---|---|---|
| **Preserve Flags** | `--title --repo --brief --thread-id --cwd --db` | `bin/omomeow-delegate:33-72` parses all six flags plus `-h/--help` and positional arguments. | **PASS** |
| **Add `--done`** | Accept `--done <SIGNAL>` | Handled at lines 49-52; forwarded verbatim to gateway argv. | **PASS** |
| **Key Generation** | `slug(title) ([a-z0-9-], max 40) + "-" + 4 random hex` | `bin/omomeow-delegate:102-106`: lowercase conversion, non-alphanumeric replacement with `-`, trimmed to 40 chars, fallback `task`, plus `od -An -N2 -tx1 /dev/urandom` producing 4 hex digits. Matches regex `^[a-z0-9-]{1,40}-[0-9a-f]{4}$`. | **PASS** |
| **Brief Persistence** | Write brief to `~/.omon/state/lanes/<key>/brief.md` | `bin/omomeow-delegate:107-110`: `mkdir -p` and `printf '%s' "$BRIEF" > "$BRIEF_PATH"`. Validated by test assertion `$SAVED_BRIEF == $TEST_BRIEF`. | **PASS** |
| **Default Done Signal** | `command ["git","-C","<cwd>","diff","--quiet"]` + warning | `bin/omomeow-delegate:96-100`: JSON-escapes target CWD, constructs exact JSON array string, and prints warning to stderr: `"Warning: defaulting to a git diff signal; a real completion signal is better."`. | **PASS** |
| **Exec Gateway** | `exec omo-gateway lane start` with argv | `bin/omomeow-delegate:112-117`: sets `DATABASE_URL="$DB_TARGET"` and executes `exec env DATABASE_URL="$DB_TARGET" "$GATEWAY_BIN" "${GATEWAY_ARGS[@]}"`. | **PASS** |
| **Delete Herdr** | `grep -c herdr bin/omomeow-delegate == 0` | Verified on `dori-work` and `dori-next`: literal count is `0`. Legacy lines 104-109 deleted. | **PASS** |
| **Delete `work_items`** | Do not write `work_items` rows | Verified: literal count of `work_items` is `0`. No SQL statements in wrapper. | **PASS** |
| **Update Tests** | `bash tests/test_omomeow_delegate.sh` passes | Verified: stub records argv and environment; test validates all 4 PASS groups; exit code `0`. | **PASS** |
| **QA Scenario: Happy** | Test log `task-11.log` | Recorded in `dori-work/.omo/evidence/dori-grade-agent-env/task-11.log` with before/after trace, exit `0`. | **PASS** |
| **QA Scenario: Failure** | Missing `--title` exits non-zero with usage | Handled at `bin/omomeow-delegate:90-93`; verified independently: exit `1`, stderr reports required args, stdout outputs `Usage:`. | **PASS** |

### 2. Injection-Marker Owned-CWD Resolution

- **The Problem**: In initial revisions of `tests/test_omomeow_delegate.sh`, the test injected shell-eval strings like `$(touch SHOULD_NOT_RUN)`. The wrapper was executed from the repository root, but the test checked `$TEST_TMP_DIR/SHOULD_NOT_RUN`. Because the wrapper forwards `--cwd` to the child gateway without changing the wrapper process's current working directory, an unintended subshell evaluation would create `SHOULD_NOT_RUN` in the invocation working directory rather than the temporary folder, escaping detection and leaving untracked test debris.
- **The Resolution**:
  - `tests/test_omomeow_delegate.sh:30`: Added `cd "$TEST_TMP_DIR"` prior to wrapper execution.
  - Absolute `BIN_PATH` and stub script paths are retained.
  - Any relative file creation from injection is strictly confined to `$TEST_TMP_DIR`.
  - The EXIT trap (`trap 'rm -rf "$TEST_TMP_DIR"' EXIT`) guarantees cleanup of the directory.
- **Verification**:
  - `repair-20261011T144053Z-shell.log` proves `cd /var/folders/.../test_omomeow_delegate_XXXXXX.DYWWA5T4gS` ran before all wrapper executions.
  - Post-execution checks verified `$TEST_TMP_DIR/SHOULD_NOT_RUN` was absent.
  - Cleanup of `$TEST_TMP_DIR` exited `0` and verified complete.

---

## Worktree Equivalence and Execution Receipts

### Source Hashes Across Trees
- `dori-work/bin/omomeow-delegate`:
  `38f8d09e368b7852b965cd76d1a2d56df1ac088625137598f407fc0823fd03ba`
- `dori-next/bin/omomeow-delegate`:
  `38f8d09e368b7852b965cd76d1a2d56df1ac088625137598f407fc0823fd03ba`
  *(Byte-for-byte identical, 120 lines, mode 100755)*

- `dori-work/tests/test_omomeow_delegate.sh`:
  `c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea`
- `dori-next/tests/test_omomeow_delegate.sh`:
  `c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea`
  *(Byte-for-byte identical, 153 lines, mode 100644)*

### Verified Command Exits and Artifacts

| Phase / Batch | Command | Exit Code | Artifact Pointer | Notes |
|---|---|---|---|---|
| **Integrated Shell** | `/bin/bash tests/test_omomeow_delegate.sh` | **0** | `repair-20261011T144053Z-shell.log` (SHA-256 `f0d714a8...`) | Executed in `dori-work` under isolated environment; 4 PASS groups. |
| **Independent QA** | `/bin/bash tests/test_omomeow_delegate.sh` | **0** | `task-11-qa-current.log` (SHA-256 `854c224b...`) | Executed in `dori-next`; verified missing-title exit 1 and usage output. |
| **Scoped Format** | `rustfmt --edition 2021 --check tests/test_lane_reopen.rs` | **0** | `guard-repair-20261011T144644Z-scoped_fmt.log` | Clean. |
| **Strict Clippy** | `cargo clippy -j2 --all-targets --all-features -- -D warnings` | **0** | `guard-repair-20261011T144644Z-clippy.log` (SHA-256 `04ff196d...`) | Clean whole-tree clippy in `dori-work`. |
| **Full Baseline** | `cargo test -j2 --all-targets --all-features --no-fail-fast` | **101** | `batch-20261011T135508Z-summary.txt` | 934 passed, 15 baseline failed, 16 ignored; 0 new baseline failures. |
| **Dashboard** | `cargo test -j2 --test test_review_dashboard` | **0** | `batch-20261011T135508Z-dashboard.log` | 59 passed. |
| **Focused Reopen** | `cargo test -j2 --test test_lane_reopen` | **101** | `guard-repair-20261011T144644Z-handoff.md` | Known runner reopen steering race; 3 passed, 1 failed in `dori-work`. |
| **Next Focused** | `cargo test -j2 --all-features --test test_lane_runner --test test_lane_reopen` | **0** | `next-focused-20261011T145256Z-results.md` | Fixed in `dori-next`: 15 passed, 0 failed, 1 ignored (reopen 4/4). |

---

## Downstream Runner / Reopen Race Dependency Impact

1. **Defect Characterization**:
   - `tests/test_lane_reopen.rs: close_reopen_discards_old_turn_and_delivers_new_owner_text_first`
   - Primary failure panic: `the reopening message must not be steered into the old turn` at line 196:9.
   - Root cause: An asynchronous steering race in `LaneRunner` (`src/lanes/runner.rs:538-579`) where an in-flight turn being canceled during lane closure did not yield before the reopen message arrived.

2. **Resolution State**:
   - Resolved and verified green in `dori-next` (`next-focused-20261011T145256Z-results.md`, exit `0`, all 4 reopen tests passed).
   - Actively being ported to `dori-work` by worker 67 (`race-port-20261011T145201Z`).

3. **Impact on Todo 11**:
   - **Zero Architectural or Runtime Coupling**: `bin/omomeow-delegate` is an external POSIX bash utility. It communicates with the gateway exclusively by spawning `omo-gateway lane start <LANE_KEY> ...` as a subprocess via `exec`.
   - **Zero Test Coupling**: `tests/test_omomeow_delegate.sh` does not start the Rust `LaneRunner` or touch the live database. It uses a mock stub on `PATH` (`OMO_GATEWAY_TEST_STUB=1`) to capture and assert CLI arguments and environment variables.
   - **Plan Dependency Semantics**: Under the plan's dependency hierarchy (`Blocked by: 8 | Blocks: 21`), Todo 11 delegates to the CLI interface of the gateway. A transient steering defect during thread reopen does not block confirmation of the delegation CLI wrapper.
   - **Broad-Green Non-Claim**: As instructed, workspace-wide test status is explicitly NOT claimed green until worker 67's port of the runner race fix completes.

---

## Adversarial Class Evaluation

| Adversarial Class | Evaluation & Evidence |
|---|---|
| **Malformed Input** | Missing `--title`, `--repo`, or `--brief` immediately exits with code `1`, logs `Error: --title, --repo, and --brief are required.` to stderr, and prints usage. Unknown options (`-*`) exit `1`. Missing flag values fail predictably with shell parameter errors. |
| **Prompt Injection** | Test inputs containing semicolons, shell expansions (`$(touch SHOULD_NOT_RUN)`), single quotes, and double quotes were tested. The wrapper persists the brief via `printf '%s'`, avoiding evaluation. Confined working directory (`cd "$TEST_TMP_DIR"`) ensures no external files are created. |
| **Cancel / Resume** | N/A for wrapper: `omomeow-delegate` executes `exec env ...`, replacing the process. Lane lifecycle management belongs to the runner. |
| **Stale State** | Keys include 4 random hex characters (`od -An -N2 -tx1 /dev/urandom`), avoiding collision across repeated runs with identical titles. |
| **Dirty Worktree** | The wrapper is read-only with respect to the repository. The default completion signal (`git -C <repo> diff --quiet`) is emitted as a literal argv string; it is never executed by the wrapper. |
| **Hung Commands** | The wrapper executes no resident loop or polling logic. It immediately delegates execution via `exec`. |
| **Flaky Tests** | The test suite contains zero `sleep`, timeout loops, or timing-dependent assertions. Lane key format is asserted with regex `^[a-z0-9-]{1,40}-[0-9a-f]{4}$`. |
| **Misleading Success** | Test asserts both positive cases (argv match, brief content match, warning text) and negative cases (usage on missing arguments, absence of database creation, absence of injection marker). |
| **Repeated Interruptions** | N/A: wrapper is non-stateful and atomic; no resume journal required. |

---

## Direct Remove-AI-Slops & Programming Evaluation

1. **Test Suite Discipline**:
   - **No excessive or useless tests**: The 4 test blocks in `tests/test_omomeow_delegate.sh` directly verify documented contracts (help text, missing required options, argv assembly + database override + brief persistence + injection safety, default done fallback).
   - **No deletion-only or removal-verification tests**: The suite tests observable behavior of `omomeow-delegate`, not merely the absence of deleted tokens.
   - **No tautological tests**: The test executes the real wrapper, captures stdout/stderr, inspects generated files on disk, and verifies exact argv written by the stub.
   - **No implementation-mirroring**: Tests assert CLI outputs and persisted file contents, not internal variable names.

2. **Production Code Architecture**:
   - **No unnecessary extraction or abstractions**: `bin/omomeow-delegate` is a lean 120-line script using core POSIX/bash utilities.
   - **Strict Shell Discipline**: Runs under `set -euo pipefail`.
   - **Safety**: Proper quoting on all array expansions (`"${GATEWAY_ARGS[@]}"`) and variable assignments.

---

## Checked Artifact Paths

1. `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md` (Main plan, Todo 11 lines 279-288)
2. `/Users/indo/code/project/omon-gateway-wt/dori-work/bin/omomeow-delegate` (Production wrapper, SHA-256 `38f8d09e...`)
3. `/Users/indo/code/project/omon-gateway-wt/dori-next/bin/omomeow-delegate` (Byte-identical)
4. `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_omomeow_delegate.sh` (Test harness, SHA-256 `c86f6af4...`)
5. `/Users/indo/code/project/omon-gateway-wt/dori-next/tests/test_omomeow_delegate.sh` (Byte-identical)
6. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-11.log` (Executor evidence log)
7. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-11-qa-current.md` (Current QA report)
8. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-11-qa-current.log` (Current QA execution log, SHA-256 `854c224b...`)
9. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-11-doneclaim.json` (Done claim)
10. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-handoff.md` (Handoff notes)
11. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-shell.log` (Integrated shell test log, SHA-256 `f0d714a8...`)
12. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/repair-20261011T144053Z-status.tsv` (Exit statuses)
13. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-status.tsv` (Clippy 0, scoped fmt 0)
14. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/guard-repair-20261011T144644Z-handoff.md` (Guard repair summary)
15. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-summary.txt` (Canonical baseline summary)
16. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/next-focused-20261011T145256Z-results.md` (dori-next reopen race pass)
17. `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-provenance.md` (Worker 67 race port receipt)

---

## Verdict Determination

**Verdict**: **APPROVE / CONFIRMED** (Todo 11 Scoped Gate)

Every success criterion in Plan Todo 11 and its verification strategy is met and backed by verified on-disk execution logs. The injection-marker test weakness is resolved, hash equivalence between trees is confirmed, `repair-20261011T144053Z-shell.log` proves clean execution in `dori-work` (exit 0), strict Clippy on `dori-work` is clean (exit 0), and the downstream runner reopen defect does not couple to or block Todo 11.

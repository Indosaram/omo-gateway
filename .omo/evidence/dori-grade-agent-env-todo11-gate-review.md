# Todo 11 independent gate re-review

recommendation: APPROVE (todo11 scope only)
verdict: scope-ready/globalpending
blockers: []
globalConfirmation: pending required combined Wave 2 batch gates
reviewer: st_01a12a24

## Original intent and outcome

originalIntent: Preserve the public omomeow-delegate inline-instructions interface while replacing work_items/herdr operations with argv-based omo-gateway lane start.

desiredOutcome: Existing callers provide title, repository and literal brief instructions; the wrapper persists those instructions under ~/.omon/state/lanes/<key>/brief.md and forwards cwd, thread, database and explicit/default completion signal correctly.

userOutcomeReview: The prior T11-KEEP-FLAGS-BRIEF blocker is resolved. bin/omomeow-delegate:110 now writes `printf '%s' "$BRIEF"` instead of treating instructions as a filename. The original documented inline example independently exits 0 and its persisted bytes compare exactly. Shell-looking instructions also remain literal. Filename-input compatibility is not part of the public contract and is not required. No todo11 criterion failure remains established.

## Checked artifact paths

Root: /Users/indo/code/project/omon-gateway-wt/dori-work.

- bin/omomeow-delegate: reread in full and inspected current git diff.
- tests/test_omomeow_delegate.sh: reread in full and inspected current git diff.
- .omo/evidence/dori-grade-agent-env/task-11.log: read exact corrected evidence artifact, including before/after reproduction, focused output and cleanup claims.
- .omo/evidence/dori-grade-agent-env/task-11-doneclaim.json: read; claims independently checked where described below.
- Original read-only plan retained from first review: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:279-287, exact todo11, plus its verification strategy.
- Original contract retained from first review: docs/AGENTS_DELEGATE.md:38-51 and baseline test diff, both use inline task instructions.

The earlier missing task-11.log gap is now closed. Main plan was not modified. This report supersedes the earlier needs-fix verdict.

## Independent focused run

Ran exactly once during this re-review, from worktree root:

```sh
bash tests/test_omomeow_delegate.sh
```

Exact output and captured exit:

```text
=== Running omomeow-delegate integration tests ===
PASS: Help output verified
PASS: Missing argument validation verified
PASS: lane start argv, brief state, and no database writes verified
PASS: default completion signal and warning verified
=== All integration tests PASSED successfully ===
focused_test_exit=0
```

The updated stub records its own DATABASE_URL. The passing run proves --db overrides a deliberately conflicting parent DATABASE_URL. The focused run also checks explicit done JSON survives unchanged, argv includes lane start and thread/cwd/title, key shape, literal persisted instructions, default done command/warning, and no wrapper database file creation.

Before the run, git index mode was 100755 for bin/omomeow-delegate and filesystem executable check returned `executable_exit=0`. Thus the test's permission-repair fallback was not needed. Test script mode is 100644 in the current diff; the stated invocation is bash tests/test_omomeow_delegate.sh.

## Independent original-regression and literal-byte probes

Used a fresh mktemp HOME and OMO_GATEWAY_BIN=/usr/bin/true. The stub cannot access DB or Discord. Original regression call:

```sh
HOME="$T" OMO_GATEWAY_BIN=/usr/bin/true bin/omomeow-delegate \
  --title 'Review brief compatibility' --repo /tmp \
  --brief 'Implement whisper-cli and ffmpeg fallback hook for Discord voice notes' \
  --done 'file done.txt'
```

Compared its saved file against `printf '%s' "$BRIEF"` using cmp and process substitution (no trailing-newline normalization). Repeated this separate manual probe with brief `Run analysis; $(touch SHOULD_NOT_RUN) "literally"`, then checked the actual invoking cwd for the injection sentinel. These are manual reproductions, not additional suite runs.

Exact output:

```text
documented_inline_exit=0
documented_exact_bytes_exit=0
literal_inline_exit=0
literal_exact_bytes_exit=0
actual_cwd_sentinel_absent_exit=0
scratch_cleanup_exit=0
```

## Criteria and nine adversarial classes

All named flags remain accepted. Repo defaults cwd through `${CUSTOM_CWD:-$REPO}`. Explicit done text is forwarded in a quoted array; default ordinary cwd produces the specified git diff command and warning. No herdr or work_items operations remain in the read production file/diff. Key generation remains a maximum-40-character slug plus four random hex digits. The wrapper execs the gateway with DATABASE_URL set, preserving gateway exit status.

| Class | Probe / disposition |
| --- | --- |
| Malformed input | Current focused run passes missing-required-argument rejection with usage. Prior independent missing-title, all seven missing-value and unknown-option probes remain applicable because the flag parser is unchanged: exit 1; missing values report unbound variable. No new malformed-input handling claimed. |
| Prompt injection | Current focused test preserves shell-looking title/brief/signal; independent cmp proves literal brief bytes and actual-cwd sentinel check passes. Production contains no eval or command-string execution. |
| Cancel/resume | N/A: lane lifecycle belongs to runner/reopen; wrapper only execs lane start. |
| Stale state | Inspected per-key brief persistence with random suffix; registry reconciliation is outside todo11. |
| Dirty worktree | Source inspection confirms wrapper forwards the git diff signal, does not execute it or mutate repository state; probes use isolated HOME. |
| Hung commands | N/A: no polling/resident worker in wrapper; runner command timeout is outside this criterion. |
| Flaky tests | One focused run passed; no sleeps or polling. Random key asserted by shape, not value. |
| Misleading success output | Original formerly failing call independently exits 0 with exact saved bytes; passing test includes actual child DB environment rather than parent-only assertion. Stub approval is not claimed as real gateway integration. |
| Repeated interruptions | N/A: no wrapper recovery/retry loop; lifecycle recovery belongs to runner. |

## Direct remove-ai-slops / programming review

Skills were unavailable in the initial exposed skill list and targeted skill lookup; applied the prompt's documented criteria directly again to current production, tests and diff.

No production extraction, redundant compatibility layer, or additional parsing was introduced by the correction. The inline printf fixes the root contract. Tests exercise observable public behavior; no deletion-only or requested-removal-only suite, tautological result checks, or new implementation-mirroring requirement was found. No test pins prose.

Nonblocking maintenance/false-confidence notes:

- The saved brief is compared twice using command substitution. The second check is redundant, and both trim trailing newlines, so the test alone does not establish byte-exact behavior. Independent cmp checks above establish exact bytes for the reviewed examples; printf '%s' directly preserves the input.
- The suite's injection sentinel check looks in TEST_TMP_DIR although the wrapper runs in the invoking cwd. Independent actual-cwd check above closes that observation for this review.
- The test still repairs executable permissions when git mode is executable. Independent pre-run filesystem and git checks passed here.
- Argument membership assertions are weaker than ordered key/value assertions. Source inspection confirms the array order; no demonstrated criterion failure.

These are NOTES, not blockers under the criterion-based review instruction. The corrected test now preserves the original inline contract rather than overfitting the erroneous cp implementation. Child DATABASE_URL capture closes the prior parent-environment false-confidence finding.

No separate todo11 code-review report explicitly showing both skill perspectives was supplied; cannot credit such a report. This review directly performs and records those checks. The executor log has an adversarial matrix, not a substitute for this independent review.

## Evidence gaps and global gate status

Combined Wave 2 fmt/clippy/all-targets-all-features/dashboard batch verification remains pending as explicitly stated in task-11-doneclaim.json and the user's re-review instruction. No global Cargo commands were run, and no global completion is claimed. No additional notepad or separate review artifact was supplied; those are reported gaps, not new product blockers. Final global confirmation waits the combined batch.

No eval/toolkit status or apply_patch tool is exposed here; used the specifically requested report path and available write tool. This is the only reviewer-owned artifact updated.

## Cleanup

The focused test removes its scratch directory through its EXIT trap. Reviewer probes used one fresh mktemp HOME, removed it, and verified absence (`scratch_cleanup_exit=0`). No services, background workers, live DB/Discord actions, product edits or commits were performed. No additional work remains in this scoped re-review.

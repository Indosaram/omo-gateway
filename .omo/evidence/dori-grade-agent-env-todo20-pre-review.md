# Todo20 focused source boundary re-review

recommendation: APPROVE (source scope only)
verdict: scope-review-ready-tests-pending
confirmation: NOT CONFIRMED; no Cargo, CLI, manual QA, or global gate execution.
blockers: []

originalIntent: Apply lane persona rules transactionally without altering non-prompt columns or owner-authored text outside the marked region.
desiredOutcome: Accepted block files append once or replace only their marked region, preserving stored owner prefix/suffix bytes and remaining idempotent.
userOutcomeReview: The latest actual API resolves the original boundary defect. Source-ready for coordinated execution; no remaining concrete blocker found in this focused correction review.

## Contract finding

Re-read the complete latest `src/profiles/lanes_block.rs`. This verdict applies to `marked_block` extraction, NOT the superseded `starts_with(block)` guard.

Evidence: `src/profiles/lanes_block.rs:63-86,108-111,126-162`.

Validation requires exactly one ordered marker pair. `marked_block` extracts only the inclusive begin-to-end marker slice; `apply_block` uses that slice before both append and replacement. Thus arbitrary accepted input headers, footers, and terminal newlines cannot be inserted outside the stored markers.

Let M be the extracted slice and P/S the existing owner prefix/suffix. Replacement yields P + M + S. Repeating the same input retains that exact prompt. Changing the incoming block yields P + M2 + S without newline/suffix growth. The append branch also receives only M, so an incoming prefix cannot accumulate after the first application. No owner prefix/suffix trimming or normalization occurs. Malformed, reversed, or duplicate marker pairs remain rejected.

T20-AC-IDEMPOTENCE and T20-AC-REPLACEMENT are supported by this source reasoning. This is not an executed reproduction or passing-test claim.

## Updated artifacts inspected

- `src/profiles/lanes_block.rs`: complete latest API, including marker extraction and transactional prompt-only update.
- `tests/test_profile_import.rs`: complete updated tests read during this correction review. BLOCK is newline-terminated; replacement test retains an owner suffix and compares repeat hashes; `replacement_ignores_block_framing_and_preserves_owner_prefix_suffix` supplies incoming header/footer and checks exact stored owner boundaries plus repeated equality. These cases address the actual defect. They have not been run. No separate append-with-header case is present; common extraction before both branches supports that behavior by inspection.
- `bot_profiles/blocks/lanes.md` and `bot_profiles/omomeow.json`: updated persona read during this correction review. Both explicitly contain report grammar, lane object, and a concrete per-integration read-only consent question. No new contradiction observed; parsed-byte parity and exact upstream dori wording are unverified.
- `.omo/evidence/dori-grade-agent-env/task-20-doneclaim.json`: latest record re-read after the owner's update. Status is `source-boundary-correction-pending-coordinated-tests-independent-review-and-qa`; its implemented entry now explicitly describes extraction of the marked region and preservation of stored prefix/suffix. Its pending list names executable framing/newline/preservation checks. No live database access/export claimed.

Authoritative criteria remain todo20 at `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:378-388`, read in the initial review. No broad plan re-review performed.

## Direct slop/programming check

Applied the prompt's documented criteria; these skills were unavailable in the prior targeted lookup. The small extraction helper aligns incoming and stored replacement boundaries and is necessary for the demonstrated contract, not speculative normalization. Updated tests assert persisted behavior and contain no prose pinning, deletion-only assertions, tautological expected values, or timing dependence. No separate todo20 code-review report documenting both skill perspectives was supplied.

## Nine-class disposition carried through this focused pass

| Class | Disposition |
| --- | --- |
| Malformed input | Validation/extraction inspected; execution unverified. |
| Prompt injection | Bound SQL parameters retained; runtime/model probe unverified. |
| Cancel/resume | Transaction cancellation unverified. |
| Stale state | Immediate transaction still precedes read; concurrency unverified. |
| Dirty worktree | Shared source changes observed; reviewer made no product edits. |
| Hung commands | No subprocess added; database lock/runtime behavior unverified. |
| Flaky tests | Updated tests have no sleeps/polling; execution unverified. |
| Misleading success output | Latest doneclaim is explicitly provisional; no test/CLI success credited. |
| Repeated interruptions | Interrupted writes unverified; successful repeat application analyzed above, not executed. |

## Exact evidence gaps and scope

Compilation/diagnostics, focused tests, actual CLI wiring/output/exit codes, parsed artifact parity, scratch-gateway lane-start/event QA, live backup/application, and global gates remain unverified. No manual QA matrix, separate todo20 code-review artifact, or notepad path was supplied. These are pending integration evidence, not a concrete failure of the corrected source contract. Transient coordinator wiring is excluded.

Only this report was written. No Cargo commands, product edits, live DB access, Discord/service operations, or commits occurred. No resources created; cleanup none. Stop condition met: updated independent partial report delivered.

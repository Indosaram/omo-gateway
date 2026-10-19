# Todo9 four-blocker source re-review

recommendation: APPROVE
status: source-ready-tests-QA-global-pending
blockers: []
approvalBoundary: Independent source review of the four previously reported blockers and the directly changed todo9 generation fences only. Not implementation completion, runtime approval, or a global gate.

originalIntent: Closed/failed lanes resume recorded work with fresh turn/error budgets and the reopening message as the first new turn; stale prior-run effects must not defeat that reset or clear the newer binding.
desiredOutcome: The owner can reopen a lane without the old turn failing the new run, losing its continuity, or consuming its first message.
userOutcomeReview: The four identified defects are repaired in the inspected frozen source. The regression now describes a runnable old turn followed by a separate owner-message turn, and the transaction paths no longer contain the reported stale failure/fallback writes or nested pool write. Actual behavior remains unverified until coordinated tests and scratch QA run.

## Disposition of the four blockers

1. **B1 / T9-RESET-FIRST: resolved by source inspection.** `src/lanes/runner.rs:771-787` takes the originating generation into fail and obtains a generation-conditioned writer before changing status, history, and the dead event in the same transaction. The third-error call at `:697-699` passes its turn generation. Service task results carry the generation captured at task launch (`:352-355`) into service failure handling (`:369-373`), preventing an old task error from failing a newer reopened generation. The service token is the task-start generation, rather than a newly queried generation at error handling; this conservatively rejects stale failures. The new `old_third_error_cannot_fail_reopened_lane` source regression (`:808-838`) calls stale-generation fail and checks working state and no event. It is unrun.

2. **B2 / T9-CONTINUITY: resolved by source inspection.** `clear_failed_resume` (`src/lanes/runner.rs:750-764`) uses MAX(history.id) filtered by reason='reopened', checks rows_affected, and rolls back/returns false on zero rows before touching sessions or adding history. Those writes occur only after the successful guarded update, in the same transaction. The caller clears the in-memory binding only when the helper returns true (`:627-630`). `stale_fallback_preserves_new_durable_binding` (`:841-884`) seeds both durable and registry bindings and asserts stale fallback changes neither. It is unrun; the no-history-on-miss behavior is inspected directly in the helper, not claimed as a test assertion.

3. **B3 / T9-REGRESSION: resolved by source inspection.** `tests/test_lane_reopen.rs:200-258` now seeds a runnable old turn with last_turn_outcome NULL, constructs the channel before spawning, bounds the first started receive and task join, and reopens only after the old backend signals entry. ClosingBackend records each prompt; the second call claims done (`:178-190`). The test asserts exactly two observed turns, exact owner text on the second, one counted turn, and done_claimed. Its steer panic detects misdelivery into the old turn. No fixed sleep or idle/start circular wait remains. Cancellation and permit release both produce an old-turn error; either select outcome exercises discard. The test remains unrun.

4. **B4 / T9-RESET-FIRST: resolved by source inspection.** Outcome bookkeeping obtains a writer, checks generation/status, inserts outcome transcript using `&mut *tx`, updates the counter/binding through that same transaction, then commits (`src/lanes/runner.rs:642-686`). It no longer awaits append_transcript through self.pool while holding the SQLite writer. The corrected reopen test's bounded completion will exercise this path when run. No successful execution is asserted here.

## Directly changed fence review

- Loop-entry generation refresh resets local errors, reopen metadata and fallback state (`runner.rs:428-436`); stale completed turns are discarded and reset local state (`:588-605`).
- Prompt transcript and idle reset use the reopened-generation SQL predicate (`:523-526`). Steer acknowledgement consumption also includes it in the UPDATE (`:575-577`), rather than relying only on a prior read.
- The backend future runs under immutable CAPTURE_GENERATION (`:542-548`). This pass does not expand into a generic backend or transport audit.
- Successful inbox consumption retains the originating-generation predicate (`:688-692`).
- `activity.rs:121-203` keeps capture state/report effects in a generation-conditioned write transaction; `:256-278` fences post-Judge state effects; `:298-323` fences quiet latch/event/post enqueue. These inspected fences use the same reason='reopened' definition. No additional reset-first blocker was found in these changed paths.

## Test and skill-perspective assessment

The available skills do not expose remove-ai-slops or programming. Applied the task's supplied criteria directly to the changed fixes and source regressions. The small failure/fallback helpers have production callers and enable testing actual DB behavior; they are not test-only extraction. New tests assert state and durable bindings rather than copying SQL or verifying deletion/removal. The active-turn test uses an event and bounded synchronization, not timing luck. No new tautological tests, unnecessary parsing/normalization, or scope drift was identified in this limited diff review. Existing broader test/style observations were not re-audited. The integrated receipt is an implementation report, not an independent skill-perspective code review; no separate such report was supplied.

## Evidence and snapshot identity

Root: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Checked artifacts:
- `.omo/evidence/dori-grade-agent-env/task-12-integrated.md`, fully read, explicitly labels current tests UNRUN and completion unconfirmed.
- `src/lanes/runner.rs`, service and generation lifecycle through new helper tests (`:340-884`), plus generation-reference search.
- `src/lanes/activity.rs`, capture/Judge/quiet transactional fence portions (`:115-326`).
- `tests/test_lane_reopen.rs`, corrected ClosingBackend and active-turn regression (`:150-258`).
- Original plan todo9 and intent were read in the prior review; no new broad plan or unrelated-source audit was performed.

User declared the integration frozen for this review. One observational SHA-256 command returned:

| Artifact | SHA-256 |
| --- | --- |
| src/lanes/runner.rs | 07b70c5a55fe52d77342ff81d009449aa428b6f32264a7f34e381acc40d0c60c |
| src/lanes/activity.rs | b483c4320fb7678038d3bb83b32f00150f5cc3c8c139c56279f4ec025b9289c1 |
| tests/test_lane_reopen.rs | 623e1ade66f7c91164d44d4ef2f9d0e2ab0a73c8a8d118c67260ce3108897b0a |
| task-12-integrated.md | a2323742b498a5c9201ac3b5fa9311aa9af25f91b3051f8e6c23b368b3bc9d60 |

Hashes identify source artifacts, not tested binaries. No Cargo command, diagnostic run, test execution, manual QA, live DB/Discord/service operation, or commit occurred. Only this report was modified. No eval/status tool is available; the explicitly requested same pre-review path is retained.

## Exact pending evidence

Current-tree compilation, focused test execution including the new regressions, global formatting/clippy/tests, standalone scratch CLI/database-path validation, and actual recorded-thread reuse/fallback QA remain pending. Earlier focused passes are not evidence for these hashes. This source-only approval closes the four review findings, not todo9 or todo12 completion. The prior nine-class runtime boundaries remain unverified; this requested limited re-review does not claim a new nine-class QA pass. No bonus generic audit was performed.

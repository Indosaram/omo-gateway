# Todo 5 final scoped review

recommendation: REJECT (full gate only; current integrated verification pending)
scopedRecommendation: APPROVE
verdict: scope-ready/global-pending
scopeReady: true

## Intent and outcome

originalIntent: Implement todo 5's exact done-signal grammar and live checking without trusting worker success claims.

desiredOutcome: Correct six-kind parsing and checking, argv/cwd/stdout behavior, timeout and caller-cancellation child cleanup, meaningful deterministic tests, and complete current integrated gates.

userOutcomeReview: All previously identified scoped grammar and cleanup-proof blockers are resolved. No further signals source changes are requested. Final integrated approval remains pending the parent's current global verification after judge33 repairs.

## Final localized correction audit

Read `/Users/indo/code/project/omon-gateway-wt/dori-work/src/lanes/signals.rs:2025-2179` and the complete current task-5.log.

- Both tautological post-action `kill` assertions are removed. Tests no longer claim that accepting either syscall return value proves death or reaping.
- Both pre-action negative EOF checks now call `try_read` once and require WouldBlock after the bounded PID handshake. There is no fixed 50 ms observation window or polling loop.
- WouldBlock is only a current nonblocking observation, not standalone proof of process lifetime. Its use is valid here in combination with the previously inspected fixture: the child retains the socket across its blocked lifetime, and post-action EOF is the actual termination witness.
- The timeout test still checks the child is alive before releasing the readiness barrier with notify_one. The single-waiter Notify permit prevents a lost wake if notification arrives before waiter registration.
- Caller cancellation still asserts a cancelled JoinError before awaiting EOF.
- Post-action EOF is bounded by five seconds in both tests. No immediate ESRCH check was added that would race OS reaping.
- Per-child Command.env and retained socket behavior, inspected in the preceding audit, remain the basis of the lifecycle proof. The reported final changes are localized test corrections rather than production contract changes.

## Previous blocker disposition

- B1 grammar: resolved; quote-aware option scanning and typed regression cases cover the cited spaces, character classes, and nested JSON failures.
- B2 command behavior: resolved; success/failure, stdout constraints, cwd, and literal argv are exercised.
- B2 false EOF: resolved; socket ownership is retained across the child's blocked lifetime.
- B2 readiness/environment: resolved; bounded readiness before timeout activation, notify_one permit, and child-specific environment avoid the cited races.
- B2 tautological PID checks: resolved in this final source inspection.
- B2 negative time window: resolved in this final source inspection.

## Verification evidence

Current executor receipt at `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-5.log` records:

```text
CARGO_TARGET_DIR=target-w5 CARGO_BUILD_JOBS=2 cargo test --lib lanes::signals
11 passed; 0 failed; 2 ignored; 429 filtered out; finished in 1.08s
exit 0
rustfmt --edition 2021 --check src/lanes/signals.rs: exit 0
```

The immediately preceding reviewer-owned focused run independently exited 0 with 11 passed and 2 ignored in 1.07s. That run predates these final two localized test corrections; it is not misrepresented as a run of the final revision. This turn inspected their source and current receipt, without another build or suite. The ignored tests are the native child helper and historical live GitHub QA.

The historical real-run artifact reports merged PR 9896 true and open PR 9907 false. It remains historical, not freshly reproduced. Library clippy and historical exact global clippy receipts do not replace the current integrated gate.

## Remaining full-gate blocker

blockers:
- violatedCriterion: REVIEW-5 / plan Verification strategy
  evidencePointer: task-5.log `Historical Gate Receipt` and parent's current integrated-gate update
  observation: The current combined global gate has not completed after all sibling and signals repairs.
  requiredAction: Parent supplies current integrated fmt/clippy/test/dashboard results with baseline attribution after judge33 settles. Do not run a duplicate broad suite for this scoped reviewer.

There are no remaining scoped fix requests.

## Direct programming and remove-ai-slops pass

The named skills were unavailable in the supplied skill list/local discovery during the initial review; their prompt-defined criteria were applied directly throughout these audits. The scanner serves multiple real consumers and addresses demonstrated failures. Parser tests assert machine-consumed values; command tests exercise actual argv/cwd/process behavior. The final corrections remove the identified tautology and fixed-window inference. No new deletion-only, implementation-mirroring, or useless tests were introduced by these localized edits. Test-only readiness/setup hooks preserve the actual child ownership and kill/wait path; the default no-hook timeout scenario remains tested.

A separate code-review artifact explicitly documenting identical skill coverage has not been supplied. This is an evidence note for final coordination, not a newly discovered signals product defect. Task-5.log still has older prose saying negative EOF is tested with a bounded timeout; the later correction bullet and actual source clearly show try_read. Its historic formatting section also labels a sibling judge formatting finding as baseline; do not use that prose for current baseline attribution.

## Checked artifacts and limits

This turn checked the current final test source range and full task-5.log. Prior turns checked the complete module, scanner, runner, fixture, plan todo 5, review brief, baseline list, ownership diff, and real-QA receipt. External-command data and dirty-worktree considerations were assessed, not automatically marked inapplicable. No product files were edited in this review.

Only this gate report was written. No JS eval tool is exposed to obtain currentAttemptDir, so the goal-specific fallback report path is retained. Scope approval is not full task confirmation; global verification remains pending.

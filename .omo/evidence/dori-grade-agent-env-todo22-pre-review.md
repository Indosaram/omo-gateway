# Todo 22 bounded four-finding rereview

recommendation: APPROVE (source readiness for the four reviewed fixes only)
status: SOURCE-READY; TESTS, QA AND GLOBAL GATES PENDING
blockers: []

This report supersedes the earlier four-blocker source review. Reviewed current source in `/Users/indo/code/project/omon-gateway-wt/dori-work` on 2026-10-11 while coordinated batch `20261011T121136Z` runs frozen. This is not todo completion or runtime approval.

## originalIntent, desiredOutcome and userOutcomeReview

originalIntent: Exact todo 22 in the main repository `.omo/plans/dori-grade-agent-env.md:401-410` requires read-only, push-first, model-free inbox collection. This rereview is restricted to the four previously reported source blockers: closure ownership/build defects, single-consumer startup ordering, focused test defects, and mail watch cadence/help contract.

desiredOutcome: The initial all-source pass and subsequent events run through one collection owner, source wiring is ownership-correct, focused tests express the intended contracts, and mail uses the specified 60-second watch interval.

userOutcomeReview: All four previously identified defects are corrected at the inspected source level. Source readiness does not establish passing compilation, tests, actual watcher behavior, shutdown QA, or the user's complete inbox outcome. No new general audit or todo23 adapter audit was performed.

## Four original findings

### T22-BUILD - resolved by source inspection

Evidence: `src/main.rs:1063-1125`; `src/discord/adapter.rs:1341-1348`; `src/inbox/watchers.rs:6,102-110`.

The pool is cloned before the static spawned closure, and a distinct `watcher_tx` clone is moved into it. The original `event_tx` remains outside for the return value. `inbox_sources(&pool)` borrows the owned pool only while building owned source objects. These changes remove both the escaping borrowed-pool and moved-returned-sender defects previously reported. The steered callback now uses `crate::inbox::watchers`; watcher imports and unnecessary receiver mutability identified by the coordinator are corrected. Actor test call-site changes are reported in the latest receipt but not independently audited in this four-finding rereview. No compiler success is claimed.

### T22-SINGLE - resolved by source inspection

Evidence: `src/main.rs:1071-1104`; `src/inbox/watchers.rs:12-17,58-64,92-100,121-148`.

Main creates the bounded channel, queues `Startup` synchronously with `try_send`, and only then creates the spawned supervisor and exposes the sender. There is one event receiver and one collection loop. `Startup` selects all sources; that loop awaits `process_event` to completion before receiving another event. Main no longer spawns a separate initial `collect_once` task or queues a redundant initial Kakao pass. Source events cannot overtake the already-enqueued startup event, so the specific concurrent-startup cursor overwrite race is removed.

This proves receiver/channel registration and serial collection ordering by source, not that both external CLI children have completed their subscriptions before the startup fetch begins. Their scheduling is still concurrent; no child-readiness handshake is claimed or newly required by this rereview.

### T22-TEST - original defects resolved by source inspection

Evidence: `tests/test_inbox_store.rs:3-5,23-26,89-90,127-139,221-248` and `imessage_cursor_timestamp_and_watcher_rowid_consistency`.

The `watchers` import exists, the fake uses the existing Config error, and the formatted snippet is borrowed. The ordinary GitHub fixture now has an ordinary notification subject and its persisted FYI assertion is consistent; a separate review-request subject still asserts that direct classification returns None. The invalid-source test now checks the empty/unadvanced cursor, recorded error, and absence of stored items, rather than incorrectly requiring no error-cursor row. No test was removed or skipped in these corrections.

Residual coverage note, not a new blocker: the review-request fixture is checked through `direct_class`, not independently collected and asserted persisted in that test. The event-dispatch test still has unbounded progress receives as noted in the original review. Neither observation is represented as a newly reproduced test failure. No focused test execution occurred.

### T22-MAIL - code omission resolved; live evidence remains pending

Evidence: `src/main.rs:1276-1287` (mail argv block); `.omo/evidence/dori-grade-agent-env/task-22-batch-fixes.md:10`.

The actual mail watch argument array now includes `--interval`, `60`, alongside folder/account/filter options. The latest receipt explicitly reports checking zele 0.8.0 watch help and says `--interval [interval]` is supported with default 15, consistent with the original plan. No timer fallback was added.

Evidence boundary: the receipt is a recorded executor assertion, not raw help output independently reproduced by this reviewer. The existing `tool-help.txt` artifact previously inspected documents mail list, not watch. No separate raw watch-help file was found in the bounded filename/content search. The original demonstrable source omission is fixed; this remaining evidence limitation is disclosed rather than upgraded into an unsupported assertion that the corrected CLI is invalid. CLI execution is prohibited in this rereview.

## Direct slop/programming check on these fixes

The named skills remain unavailable; applied their supplied criteria directly. The startup enum variant removes concurrent collection rather than introducing a second orchestration layer. Ownership clones serve the actual spawned lifetime. Test repairs assert persisted behavior/error reporting and notification escalation rather than testing removals, prose or copies of implementation. No new deletion-only, tautological or removal-only test was introduced in the inspected corrections. No newly demonstrated maintenance/scope defect blocks these four fixes.

The latest receipt is an implementation repair handoff, not an independent code-review report with explicit overfit/slop coverage. That previously disclosed evidence gap remains; this direct check does not pretend otherwise.

## Checked paths and verification limits

Read current:
- `src/main.rs:1060-1364`.
- Complete `src/inbox/watchers.rs` and `tests/test_inbox_store.rs`.
- `.omo/evidence/dori-grade-agent-env/task-22-shared-wiring.md`.
- Latest repair receipt `.omo/evidence/dori-grade-agent-env/task-22-batch-fixes.md`.
- Narrow steered callback at `src/discord/adapter.rs:1341-1348`.

The older shared-wiring receipt still describes the superseded separate initial pass and extra Kakao refresh. The latest repair receipt and actual source supersede that description; old prose was not treated as proof of current behavior.

Coordinator-provided batch status at request time: `20261011T121136Z` fmt exit 1, clippy exit 101, full suite in progress. These are not passing results. This reviewer did not inspect or reclassify their failures, assign them to baseline, wait for the full run, or claim any runtime pass. Current compilation, focused/full tests, global checks, live CLI QA, and process cleanup behavior remain unverified here. No pending result is represented as complete.

The original nine adversarial classes were not re-audited: initial-pass overlap is source-resolved above; numeric cursor separation, Kakao timer, commit-before-wake, saturation/shutdown, child lifecycle, transactional isolation, and filter/snippet behavior retain their prior source-observed/runtime-unrun status. Mail cadence is source-resolved with live CLI evidence pending. This bounded rereview adds no runtime confirmation to any class.

Cleanup: Only this main-repository report was written. No Cargo, tests, runtime, CLI, network or source-changing commands ran; no resources were started. Worktree inputs, frozen manifest, coordinated driver and batch were untouched. The requested four-finding review is complete.

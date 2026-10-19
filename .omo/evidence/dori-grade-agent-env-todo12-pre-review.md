# Task 12 integrated source-only gate review

recommendation: APPROVE (source-only pre-review)
status: scope-ready-tests-QA-global-pending
blockers: []
scope: Bounded re-review of B12-1 following the initial integrated source audit. Not runtime approval, test confirmation, or completion of the plan checkbox.

## Original intent and desired outcome

originalIntent: Implement exact todo 12 of `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`: convert captured lane reports and quiet turns into persistent state, selective lead events and curated thread posts, replacing Herdr liveness.

desiredOutcome: Questions/blockers pause chaining until lead input through send or object; milestones post without waking the lead; unreported successful turns use the typed Judge; done prose never establishes completion; quiet running turns produce one persistent stale episode across restart; actual receipts distinguish delivery from uncertainty; non-lane backend behavior is preserved.

userOutcomeReview: The sole source blocker B12-1 is resolved by the inspected correction. The source now supports blocked-object recovery through the existing blocked-to-working transition with atomic guidance enqueue and idle-latch clearing. Done-claimed objections retain their not-done path; other states remain invalid. No new blocker was found within this bounded review. All current-tree tests, diagnostics, strict gates and named scratch QA remain pending and unconfirmed.

## B12-1 resolution and bounded re-review

Original violatedCriterion: T12-BLOCKED-RESUME, exact todo 12 at main plan line 291: blocker/question pauses until the lead answers via send or object.

Original evidence: the former `src/lanes/cli.rs:430-449` always selected NotDone for object, while `src/lanes/state.rs:62` allowed NotDone only from DoneClaimed. Thus blocked object rejected before guidance enqueue or wake. This historical finding is preserved here and superseded by the following source correction, not by success prose.

Frozen source root for all relative paths: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

- **Allowed blocked recovery:** `src/lanes/cli.rs:430-506` acquires the SQLite writer before reading status, identifies `!claim && status == Blocked`, and selects Working for that case. It uses the existing state table instead of broadening transitions.
- **Claimed and invalid-state behavior:** the same function selects NotDone for other objections and checks `can_transition_to`. DoneClaimed therefore retains NotDone, its objection event and lead guidance. Queued, Working, NotDone, VerifiedDone, Closed and Failed still reject. Errors are mapped to exit 2 by `cli.rs:100-124`. Rejected transitions return before state/history/event/inbox mutations; the initial no-op writer acquisition rolls back.
- **Generation fence and atomic enqueue:** the writer-held transaction reads the latest reopened-history generation and conditions the status update on both prior status and generation. It writes objection/history and enqueues the exact supplied owner answer within the same transaction. A failed update or later SQL error cannot commit only half of the recovery. This is source analysis, not a runtime race or rollback test result.
- **Idle and notifications:** the objection update clears last_turn_outcome. Blocked answers omit a redundant lead event. `cli.rs:178-185` awaits change_claim completion, then invokes wake and reports the resulting status. `runner.rs:203-219` sends the wake hint through the existing Unix datagram path; missing/refused socket is tolerated because durable DB intent survives. The earlier integrated audit traced service socket wake to active-lane Notify and scheduling. This re-review did not run that service.
- **Meaningful regression source:** `tests/test_lane_cli.rs:28-96` captures a real question into SQLite, invokes the actual CLI object command, checks working/cleared idle state, and runs a scripted backend under a five-second bound. The subscribed channel verifies the exact next owner-text prompt and the backend ends by claiming. `:99-151` invokes the CLI for all remaining statuses and checks exit status, resulting lane status and inbox count. These would expose the original defect, unlike a helper-only assertion.

Test limitations: these tests are UNRUN. They do not establish delivery of a wake to an already running service or a concurrent reopen race. CLI subprocess output awaits are not separately timeout-wrapped; the runner is bounded, and no sleeps, polling waits or circular handshake were introduced. This is a boundedness note, not evidence of a failed product criterion. The claimed-path test checks status and inbox count rather than every event/guidance field; preservation of those fields was checked directly in production source.

## Preserved integrated source assessment

The following findings come from the initial integrated source audit. This bounded follow-up did not repeat that complete audit or the separate todo-9 four-blocker review.

- **Capture identity and snapshots:** `activity.rs:27-48` replaces cumulative snapshots and finalizes increasing sequences once. `runner.rs:128-151` treats nonfinal snapshots as activity only. Completed backend items use lane-item turn/item identity (`omo_backend.rs:554-568,1323-1325`); generation/item markers and report effects are transactional (`activity.rs:142-184`). Equal text from different item IDs remains distinct. Final aggregate delivery is suppressed after completed items (`omo_backend.rs:584-587`). Runner does not enqueue milestones a second time at turn end.
- **Actual task-local path:** service tasks spawn at `runner.rs:353-356`, but generation scope is installed inside run_lane around the awaited backend future (`:543-547`). Default AgentBackend::run_cancelable directly polls run (`src/agent/backend.rs:39-52`); inspected OmoBackend dispatches captures inline and has no spawned capture task. No assumption of automatic task-local inheritance across spawn was made. Arbitrary detached future backends remain outside this proof.
- **State/Judge/claim precedence:** capture serializes report state/history/events. Claimed/non-working lanes suppress report effects. Judge runs after successful unreported turns only; post-Judge status/generation recheck fences concurrent claims/reopens (`activity.rs:253-278`, `runner.rs:701-729`). Input is scrubbed and capped at 2,000 characters. Outcome preserves idle_without_claim (`runner.rs:684`) and idle waits for input (`:488-494`). No timer invokes Judge or an agent to inspect routine activity.
- **Stale episodes:** quiet requires running, working state, 20-minute inactivity and NULL stale latch. Latch, event and durable post intent share one transaction (`activity.rs:298-333`); capture resets the latch. This is persisted episode idempotence, not indefinite exactly-once external delivery.
- **Post delivery:** `posts.rs:38-55` persists a scrubbed bounded body and 25-character nonce. Real sink sends with safe mentions, nonce and enforce_nonce and returns the actual message ID (`:79-89`). Drain conditionally claims pending rows, bounds transport to 30 seconds and records sent only on acknowledgement; error/timeout becomes uncertain (`:126-166`). Restart converts interrupted sending to uncertain (`:94-98`), never blindly to pending. Invalid destination never calls transport. The sole service drains at startup and on notifications.
- **Current API bounds:** Cargo.lock identifies Serenity 0.12.5. Its installed `src/builder/create_message.rs:279-293` accepts Nonce and documents enforce_nonce uniqueness only for the past few minutes. The source call matches; this is neither compilation nor remote API proof. Missing item IDs use text-hash identity, which cannot distinguish identical ID-less items in one turn; no stronger guarantee is asserted.
- **Pause/progress:** question commits blocked; the current turn may finish but chaining stops. Send already queued guidance, transitioned to Working and woke the runner. Object now does so through the corrected path above. Judge progress permits chaining without an event.
- **Herdr removal:** initial all-src case-insensitive search found no herdr matches. Main/work_heartbeat diff removed the concrete checker and main startup/shutdown, preserving generic clocks and abstractions. Coordinator receipt is now independently housed at `.omo/evidence/dori-grade-agent-env/task-12-shared-wiring.md`; the restored owner receipt is `.omo/evidence/dori-grade-agent-env/task-12-integrated.md`. The coordinator receipt's older statements that object repair/restoration remain pending are superseded by inspected source and restored owner receipt, not treated as current blockers.
- **Non-lane behavior:** initial backend diff showed new completed-item dispatch, metadata reset and final suppression lane-gated. Removal of the existing `if false` tool-progress block did not establish a live behavior change. Non-lane runtime regressions remain pending the coordinated run.

## Nine adversarial classes

| Class | Source assessment and remaining limit |
| --- | --- |
| Malformed input | Event parser tests reject foreign key, unknown kind and empty/missing text and preserve embedded pipes. Invalid Judge choice preserves registry. New object tests cover every invalid registry status with exit 2 and no inbox entry. All UNRUN. |
| Prompt injection | Scrubbed 2,000-character Judge boundary, typed options and no new shell evaluation. Model resistance is unverified. Blocked owner answer is passed as prompt data, not shell code. |
| Cancel/resume | Actual generation scope/capture path and old-generation regression inspected initially. Corrected object update is fenced under a writer-held generation read and status update. Full todo-9 re-audit remains separate; no race execution claim. |
| Stale state | MockClock boundary/reconstruction/rearm and atomic stale post/event/latch tests remain meaningful source evidence. Corrected object clears the idle latch; new test begins with a pre-existing idle latch. No live restart QA. |
| Dirty worktree | Task-12 DB/post/capture operations do not mutate repository worktrees. Reviewer changed only this report. No invented worktree-cleanliness requirement. |
| Hung commands/transport | Post send bounded to 30 seconds; event and new object runner tests use five-second bounds. New CLI subprocess awaits have no separate bound, noted without claiming runtime success. |
| Flaky tests | No sleeps, polling waits or circular handshake found in task-12 event tests or the new object regressions. Channels exist before runner action. Some subprocess/top-level awaits remain unbounded; tests are UNRUN. |
| Misleading success | Done prose cannot verify/close; claim precedence tested in source. Post sent state requires receipt. This recommendation is source-only and never labels tests or QA confirmed. |
| Repeated interruptions | Recovery distinguishes sent from uncertain without blind retry; SQLite trigger tests assert rollback. Object transaction includes state/history/inbox atomically. Process-crash, transport and concurrent-reopen runtime proofs remain pending. |

## Programming and overfit/slop review

The available skills do not expose remove-ai-slops or programming; applied the supplied criteria directly. Initial event tests and new object tests exercise observable contracts using real SQLite and actual CLI/backend seams. No deletion-only, removal-only, tautological or prose-pinning tests were found. Exact owner text here is machine-consumed prompt data; checking it verifies guidance propagation rather than stylistic prose.

The fake backend terminates by invoking the real claim storage path and allows observation of the actual next prompt. It does not mock away CLI transition/inbox behavior. The fixed fake sink receipt from the initial tests proves delivery-state logic only, not Discord receipt. New branch logic remains inline and uses the existing state machine; no unnecessary parser, normalization or abstraction was introduced.

Preserved nonblocking notes: ActivityDecision.thread_posts is redundant informational data alongside durable posts. Component tests are not an end-to-end real sink proof. No separate task-12 code-review document explicitly covering these same skill perspectives was supplied; implementation receipts do not replace this direct review. None of those notes establishes a violated task criterion.

## Checked artifacts and exact evidence gaps

Initial audit inspected: exact main plan todo 12 and relevant todo 7 constraints; activity.rs/posts.rs; runner capture/service/turn loop; agent/backend.rs; backend seam/source searches/full diff; CLI and state transition table; all test_lane_events.rs; runner fake/chaining/cap/dead tests; Cargo registration and matching Serenity source; main/work_heartbeat removal diff; mod and HTTP accessor; initial owner receipts and batch removal receipt.

Bounded follow-up additionally read: corrected cli.rs object/change_claim and error-exit paths; new test_lane_cli.rs lines 1-151; runner wake implementation; `.omo/evidence/dori-grade-agent-env/task-12-blocked-object.md`; restored task-12-integrated.md; separate task-12-shared-wiring.md; and this prior report before replacement. Prior finding B12-1 and all substantive nonblocking observations are preserved above.

Still missing/unverified: current-tree compiler and clean LSP diagnostics; focused lane_events, lane_runner, lane_reopen, lane_cli, runner-unit and backend regression results; repository fmt/clippy/all-target/all-feature/dashboard gates; successful task-12.log; named scratch question/pause/send-object resume QA; actual sink receipt; stale restart and dead-event QA; recorded-thread close/reopen QA; final manual QA matrix. Worker reports rustfmt parsing/output only and LSP timeout. All current-tree tests remain UNRUN, not green. No notepad path was supplied. No eval tool is exposed for an attempt-directory SDK query; used the explicitly requested main report path.

Cleanup: this review performed read-only source inspection and updated only this report. No source edit, Cargo/test/formatter/QA command, live DB/Discord access, commit or service operation occurred. No process, scratch DB, listener or worktree was created; nothing needs teardown.

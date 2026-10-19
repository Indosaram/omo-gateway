# Independent final Task 12 gate

## Verdict and findings

**APPROVE / INDEPENDENTLY CONFIRMED for full main-plan Task 12. Task 12 blockers: none established.** This is an independent evidence-only review, not an implementer receipt, a whole-backend all-green claim, or approval of Task 14. The main plan and ledger were not changed.

Scope is Task 12 at `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:290-297`: captured reports and unreported terminal turns become persistent activity decisions; question/blocker pauses until send/object; milestones post without lead events; done prose requires a claim; 20-minute silence yields one durable stale episode across restart; three backend errors fail with a dead event; Herdr liveness is removed; an actual question event is created within two minutes.

Residual limitations, not Task 12 blockers:

- The helper first observed the paused state at **120003ms**, but the event itself was created at **7855ms** after the trigger. This is late observer visibility, not a missed two-minute product event deadline. No low-latency observer claim is made.
- Both actual question events were digest-skipped (`jev_skip`). The lead export contains no matching lead session, messages, or delivery obligations. The actual producer-to-lead-turn-and-reply bridge remains **Task 14 pending**, not a Task 12 false completion claim.
- Historical backend tests `test_omo_backend_e2e_thread_lifecycle_and_streaming` and `test_omo_backend_retries_once_after_connection_drop` remain unverified after the retained timeout. Neither the current scoped pass nor this gate makes them green.

## Provenance and review boundary

All source references below are relative to `/Users/indo/code/project/omon-gateway-wt/dori-work`. Let `E` denote that worktree's `.omo/evidence/dori-grade-agent-env`, and `Q` denote `E/task-12-question-cujzOD-1791745207199`.

Read-only byte verification found all **205** entries of `Q/source-manifest.sha256` equal to current worktree bytes, with no mismatches. Manifest SHA256 is `a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d`. Current `target-w4/debug/omo-gateway` SHA256 independently equals prepared/released identity `f81c077ee6a1252d647b7fc8f45758f8499b687dc9d2bb84d57bb3b512f15912`.

The actual scratch DB identity is `/private/tmp/omon-lane-scratch-qa-cujzOD/qa.db`, supported by prepared metadata, retained descriptor output, and serialized post-cleanup/export PRAGMA results. This reviewer did not open any DB. No tests, shell commands, network/model/backend/Discord calls, service changes, product edits, environment edits, or plan/ledger edits were performed. Only this Markdown verdict was created.

## Current source assessment

| Requirement | Current source and retained observable proof | Result |
| --- | --- | --- |
| Report parsing and captured activity | `src/lanes/activity.rs:33-71,126-216` replaces cumulative snapshots, accepts complete matching-key reports, persists activity/report state, and fences generation/item identity. `src/lanes/runner.rs:99-185` captures completed messages and notifies the post worker. Events tests cover stable item replay, malformed/foreign reports, rollback and backend completed-item capture. | Confirmed |
| Question/blocker pause | `activity.rs:187-203,339-373` commits blocked state/history/event together. `runner.rs:442-448` starts turns only for working/not_done; captured question can finish the current turn but cannot chain. Actual first snapshot is blocked with one completed turn and one question event. | Confirmed |
| Send and object recovery | `src/lanes/cli.rs:154-184` enqueues exact send text, resumes blocked state and wakes; `cli.rs:430-514` selects Working for blocked object, clears idle outcome, and atomically queues exact guidance under status/generation fencing. `runner.rs:488-540` uses owner inbox text as the next prompt. Actual send receipt and transcript prove exact owner text and same-thread terminal resume. Object support is directly inspected source, not claimed as this live send scenario. | Confirmed |
| Milestone and done semantics | `activity.rs:179-213` queues a milestone post without inserting a lead event; done creates idle_without_claim, not verified completion. Current milestone, done and claimed-lane suppression regressions pass. | Confirmed |
| Unreported terminal Judge | `activity.rs:219-278` uses typed finished/progress/blocked_or_question on scrubbed last-message input capped at 2000 characters, with post-Judge status/generation fencing. `runner.rs:734-770` invokes it only after a successful unreported working turn. Progress chains without an event; finished idles; blocked pauses. Current typed/bounded and concurrent-claim regressions pass. | Confirmed |
| Quiet/restart episode | `activity.rs:280-334` requires an active working turn, 20-minute inactivity and a NULL stale latch; latch, stale event and post intent share a transaction. Captured activity resets the latch. `runner.rs:539-585` performs the cheap local check during an active turn. Quiet threshold/restart, activity rearm and rollback regressions pass. | Confirmed |
| Actual post acknowledgement and uncertainty | `src/lanes/posts.rs:38-166` persists curated body/nonce, sends through actual Serenity with safe mentions/enforced nonce, marks sent only on returned message ID, and makes interrupted/error/timeout sends uncertain without blind retry. Existing HTTP receipts establish acknowledgement and restart behavior. | Confirmed |
| Three errors and dead event | `runner.rs:728-733` fails after three consecutive errors. Current `three_errors_fail_with_dead_event` passes in the released runner batch, rather than being inferred from events-only tests. | Confirmed |
| No Herdr source use | Bounded `tool.grep` search for `herdr` over current `src` searched 106 files and returned zero matches, with no timeout/truncation. Existing integrated source review records removal of the concrete checker and main startup/shutdown wiring. | Confirmed |

## Actual question, owner answer and cleanup

Primary evidence: `Q/task-12-question.json`, `Q/question-receipts.json`, `Q/question-postcleanup-db.json`, `Q/prepared.json`, `Q/execution-started.json`, and `Q/task-12-question-lead-export.json`.

- Lane `qa-1`; worker session `4:lane|-|4:qa-1|-|9:lane:qa-1`; daemon thread `01a12c56-32b4-717f-b592-b5b5d83e0bde`; recorded model `inferhub/cb/deepseek-v4.1-flash`.
- First actual agent report is exactly `[REPORT] qa-1 | question | pick A or B`. Event **ID 1**, kind question, body `pick A or B`, created `2026-10-11T19:00:15.214552+00:00`. First snapshot has status blocked, turns_run 1, last_turn_outcome completed, and exactly one event. Transcript outcome ID 4 is completed at `19:00:15.219068+00:00`.
- `eventElapsedMs=7855` is trigger-to-event creation. `elapsedToPauseMs=120003` is trigger-to-helper observation. These are different measurements; the actual event meets the two-minute criterion.
- Successful scratch CLI send queued this exact owner answer, also preserved verbatim as user transcript ID 5:

```text
A. Emit exactly [REPORT] qa-1 | question | A received; waiting for next instruction and end the turn. Do not use tools or claim.
```

- The same thread resumed. Agent transcript ID 7 is `[REPORT] qa-1 | question | A received; waiting for next instruction`; terminal outcome ID 8 is completed at `19:02:10.633499+00:00`. Second snapshot has turns_run 2, blocked, last_turn_outcome completed.
- **Event ID 2 is another actual question**, body `A received; waiting for next instruction`, created `19:02:10.621108+00:00`. It is not idle_without_claim, not a duplicate first question, and not an extra automatic idle turn. First-turn event count is one; post-answer total question count is two.
- Post-cleanup snapshot is closed with two turns, two completed outcomes, the same thread and preserved event IDs/bodies/created_at plus exact owner/agent transcripts. The close was explicitly forced for QA cleanup, not verified_done or a successful done signal.
- Gateway **PID 43600 exited 0 and was absent**; all listed owned child cleanup records likewise report absent. Retained cleanup says sharedDaemonUntouched true and scratch retained.
- Numeric helper success is grounded in the original execution session, not inferred only from generated JSON: `.omo/senpi-task/children/st_01a12b72/sessions/st_01a12b72/2026-10-11T14-51-23.861Z_eecd75e8-efca-4ae4-952f-017f0f4cd47b.jsonl` under the main workspace has the `2026-10-11T19:04:07.548Z` terminal notification: `bash_32 ... completed (exit code 0)` and `QUESTION_SEND_RESUME_OBSERVED` with this exact archive/root. The helper source `E/lane-question-qa.mjs:130-172` emits that result only after joined owned cleanup and post-cleanup preservation checks.

## Existing machine gates and transport receipts

`E/task-12.log` preserves the actual broader focused command, exit 0, events **25 passed / 0 failed / 1 subprocess helper ignored**, and post transport **2 passed / 0 failed**. It explicitly labels itself derived from `combined-current-20261011T173003Z`, not an exact standalone events command. No rerun is implied.

The newer `E/duplicate-final-20261011T184722Z-lanes.log` and `-status.tsv` bind current released source: events **25 passed / 1 helper ignored**, runner **12 passed / 1 live test ignored**, including the three-error/dead regression. Ignored tests are not counted as passes. The supplied actual question receipts fill the live question scenario independently.

`E/task-12-http.json` is a declared replay of a preserved completed REPORT through **real local Serenity HTTP and persisted post ledger**, not fresh backend capture or live Discord. Acknowledged case records actual message ID **400**, sent state after restart, request_count 1 and restart_send_count 0. Accepted-request-without-ack case records uncertain state, null message ID, request_count 1 and restart_send_count 0. Both report listener closed, server joined and owned DB closed. Its posts source SHA256 `7cc0a2caf6a8ccc288f9001737e2c59ddecc83993d5232ffc00028ea2ddd9230` independently matches current `src/lanes/posts.rs`. Its older pending Clippy annotation is superseded by the current strict gate, not silently removed.

Current `duplicate-final-20261011T184722Z-status.tsv` records fmt 0, strict offline Clippy all-targets/all-features with `-D warnings` 0, completed_ regressions 0, failed-final-delivery regression 0, headless 0, and lanes 0. Raw Clippy and build recovery logs show finished profiles. Original build **101** is retained as the harness-owned TMPDIR removal failure; corrected build-only recovery is **0**, completed in 5.24 seconds, without replaying tests. Source identity remains the 205-entry a957 manifest.

## Task 14 separation and final disposition

The actual lead key is `4:lane|-|4:lead|-|9:lane:lead`. Event 1 has delivered_at `19:00:47.322961+00:00`, reason jev_skip; event 2 has delivered_at `19:02:42.323144+00:00`, reason jev_skip. `Q/task-12-question-lead-export.json` explicitly contains empty leadSessions, leadMessages and deliveryObligations. Non-null delivered_at is digest disposition, not proof of a lead turn or reply. Preserve these rows; no retry/reset or synthetic bridge was performed by this review.

**Final actual verdict: full Task 12 confirmed, no scoped blocker. Task 14's actual produced-event-to-lead-reply integration remains pending. Historical backend timeout limitations remain unverified. No global plan completion or ledger promotion is claimed.**

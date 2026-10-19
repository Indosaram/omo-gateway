# Task 14 current evidence map

Reconciled 2026-10-11 from existing files and read-only pathname/hash checks. Disposition: **NOT CONFIRMED**. Task 14 implementation is in dori-next, not the current dori-work binary. Historical fake acceptance receipts exist; actual lead reply/log and live marker refusal/recovery evidence remain missing. Product remains frozen during actual8/9 same-thread reopen QA.

This document is the only intentional edit. No tests, Cargo, DB opens/writes, backend/model calls, Discord operations, service controls, drain markers, product edits, commits or deployments were performed. Existing QA commands quoted below were read, not executed. Hashes are a point-in-time observation, not a source-equivalence claim.

## Authorities and scope

- Main plan: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:312-324`. Task 14 remains unchecked. Lines 598/600 associate tasks 12/14 with IS-4 delivered lead turn and IS-6 milestone silence/coalescing/30-second cap.
- Main ledger: `/Users/indo/code/project/omon-gateway/.omo/ulw-execute/ledger.jsonl:152,158,162`. Line 152 separates frozen dori-work from wave-3 dori-next implementation; line 158 records source-settled dispatcher, eight tests and migration0037; line 162 explicitly limits approval to source pre-review and denies final confirmation/runtime/global-gate implications. The unrelated ledger line17 task14 is an older inbound cutover, not this task.
- Independent review: main `.omo/evidence/dori-grade-agent-env-todo14-pre-review.md`, reviewer st_01a12b35, APPROVE/CLEAR for the reviewed next snapshot. Its claims are source review, not live receipts.
- Worker receipt: `E/task-14-source.md`, explicitly names dori-next and originally leaves compilation/tests/live QA to parent gates.
- `W=/Users/indo/code/project/omon-gateway-wt/dori-work`; `N=/Users/indo/code/project/omon-gateway-wt/dori-next`; `E=W/.omo/evidence/dori-grade-agent-env` below.
- Exact `task-14.log` was not found under the main, work or next `.omo/evidence` trees. Do not create a passing named log by renaming narrative source review or imply the planned command ran merely because another raw log contains the target.

## Current provenance: work is not next

Read-only SHA-256 observations:

| Relative input | W SHA-256 | N SHA-256 |
| --- | --- | --- |
| src/lanes/dispatch.rs | ABSENT (read returned ENOENT) | c7c983f5b79acc4e8245be5cf6a47c9678f8cd384bc138313a7b90e95260b102 |
| tests/test_lane_dispatch.rs | ABSENT | 57fbeb82349fae2d9602a5bf2983fc02e5831439b1077c4cca55b05540aa4f79 |
| migrations/0037_lane_event_delivery.sql | ABSENT | f3d1447a96432e4f83c110538c3b194fc27cd8dc07cb0644ef490e2d0ab57554 |
| src/main.rs | fa983673a42e48fdf254fd2768d9193252b09eb10bee9d1500c6162687de1650 | ffc827adf403c351da02753a8beb8ede915b78497413cf60a93a6590e662170c |
| src/multiplexer/router.rs | c2b06e721bcac7a801057c2e4f6613781292f9fd8ee9847c96f8756c5e85d307 | dfa5e769e40ffb8afad751ab432c794353526e02f50c7df369200190801ccc5b |
| src/multiplexer/actor.rs | 09f1bf14b3b5b3146bb7059abac0c5005f72765c848fe257252d31536b599dfe | 0186ec9acd9c7e13ecf4224b73c68fceecc498ac7cddcb33a8dd52ef077fd434 |
| src/models/events.rs | 92d4ca4ce1ee3614dbec8b2a36c557d91d26084e91d78754033cf8787cf578a1 | c4ac34a772bd087543e26fbe6473a74ad00ea33416a93bca09f4daaae942ad68 |
| src/lanes/mod.rs | e8c05cd90f6c8f26b288c8ec908de5658288527e929ec0d068224033a172cfae | 8ddbf9e21666cfe4146e066f678ed749fb7b6aa5a657f3a32dffb9539b17f2f3 |

Measured existing `W/target-w4/debug/omo-gateway` SHA-256 is `60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32`. `E/fixture-repair-20261011T161103Z-handoff.md:5` records its once-only build exit0, unchanged pre/post work source manifests and manifest SHA-256 `45200128b3b5cd63fbc956b36e01daaa82f817b63607d176ef6751d89f486718`. That is work build provenance, not proof that the binary contains next-only dispatcher changes. No new build or complete current-source manifest comparison was run here.

Runtime wiring read directly:

- `W/src/main.rs:1050-1061` spawn_lane_service starts only LaneRunner behind OMON_LANES_ENABLED (default true); no EventDispatcher exists in W. W router has drain-receiver support at :300 and refusal at :327 onward, but the ordinary W main creates its watcher later at :962; the ordinary multiplexer is not wired with that receiver by task14's next change.
- `N/src/main.rs:764-776` creates DrainWatcher before the ordinary multiplexer and attaches the cloned drain receiver. `:961-976` adds JevCli and EventDispatcher to lane_tasks under OMON_LANES_ENABLED; cancellation joins the same task lifecycle. This kill switch is source wiring only, not an assertion in test_lane_dispatch.
- **Both trees' Discord-free run_gateway branch returns before ordinary multiplexer/dispatcher construction.** W `:646-680` and N `:649-683` initialize the explicit DB and call spawn_lane_service only. N spawn_lane_service still starts only LaneRunner. Thus setting OMON_DISCORD_ENABLED=false in the existing actual8/9 harness does not exercise task14 even if a next binary were substituted. Do not enable Discord to work around this during the freeze.

## Requirement -> actual assertion -> raw receipt

All eight names below appear as `test <name> ... ok` in `E/next-focused-20261011T140523Z.log:25-38`, followed by:

```text
Running tests/test_lane_dispatch.rs (target-n1/debug/deps/test_lane_dispatch-9991346d53ef1605)
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
```

Its companion `next-focused-20261011T140523Z-status.tsv` records **focused exit101**, for the multi-target all-feature/no-fail-fast command. Other targets failed (done, events and later targets); a posts unused-import warning is also present. The dispatcher target passed; the batch did not. These are historical next-target fake-runner receipts, not a current-source rerun or a binary60fce5ae acceptance pass. The raw log carries no complete source identity proving equivalence to today's hashes.

| Requirement | N test/source locator and actual assertions | What remains outside that receipt |
| --- | --- | --- |
| Three events in 10s, 30s coalescing | test_lane_dispatch.rs:201, `three_events_within_ten_seconds_coalesce_into_one_turn`: enqueue at0/5/10; tick10 no observed start; tick30 one observed event, four lines including header, pending0; after finish no extra observed start | No actual gateway 5s scheduler or live three-line reply evidence. |
| Immediate done claim when idle and slot available | :222, `done_claimed_delivers_immediately_after_idle_delivery_slot`: tick0 delivery, finish, second claim/tick31; received_at==31; judge calls0 with score0 | dead/kibitzer immediate eligibility is implemented in dispatch.rs:73-82 but not separately exercised by this test. |
| Immediate kinds still limited to one turn/30s | :238, `two_done_claims_five_seconds_apart_wait_thirty_seconds`: first at0, second at5, ticks5/29 pending1/no observed start; tick30 second received_at-first>=30 | No restart test or separate critical-kind matrix. Durable MAX(delivered_at), excluding jev_skip, is source at dispatch.rs:83-91. |
| In-flight lead stays queued until completion | :265, `event_during_in_flight_turn_waits_until_turn_ends`: blocked User turn observed; done_claim tick30 remains pending1/no start; finish via oneshot plus typing-end signal and idle probe; tick30 starts queued turn, pending0 | Probe/admission race not independently forced by this test. N router:330-360 and actor:245-253/:356 implement atomic EventIfIdle refusal. |
| Dedup latest (lane,kind), receipt all IDs | :288, `duplicate_lane_kind_pairs_collapse_to_latest_body`: old at0/latest at5; tick30 event two lines, body latest, pending0, judge calls1 | Source picks newest (created_at,id) and tracks all IDs at dispatch.rs:97-108; no separate equal-timestamp tie test. |
| Route refusal retains events and retry | :307-equivalent source function `route_error_keeps_events_undelivered_for_retry` (within full 332-line file): watch drain=true; direct System mux.route returns Err; dispatcher tick leaves pending1/no observed start; drain=false/tick5 produces turn/pending0 | This test injects watch state, not a filesystem marker. Dispatcher returns before route while draining. It does not force a general post-selection route error, logging failure, or failed receipt transaction. |
| System no outbound reaction, transcript marker | `system_event_triggers_no_outbound_reaction`: event.origin==System; after completion FakeOutbound React count0; messages content starts `[system] ` | FakeRunner completes successfully; error/interruption reaction paths and real ingress authorization are not exercised. Source receipt explains owner checks/start reactions live upstream in Discord ingress, which direct System mux routing bypasses. |
| jev gate below0.5 becomes digest-only | :319, `jev_below_threshold_marks_digest_only_without_turn`: FakeJudge0.49, tick30 pending0, delivery_reason==jev_skip, noul Purpose::EventWake assertion, calls1/no observed start | No actual jev invocation here. Score0.9 tests selection; no exact0.5/error test. Critical bypass test covers done_claimed, not every critical kind. Digest rendering itself is not asserted. |
| 5s local queue scan / grouping / payload shape | dispatch.rs:32-60/:149-179: Skip missed ticks, BTreeMap lead groups, lane-event:first-last, System, body max600 with CR/LF flattened, route then lead_turn receipt | Tick is called explicitly with injected timestamps by tests; no serve timing, multiple-lead, max600, full ID-range or critical-kind matrix assertion. |
| Kill switch | N main.rs:963 guard OMON_LANES_ENABLED; shared lane_shutdown cancellation | No kill-switch test in this eight-test target; disabled LaneRunner evidence in fixture-repair handoff does not assert dispatcher disabled. |

Exact current test function declaration lines, authoritative over the approximate row locators above: coalescing187; immediate205; rate limit220; in-flight244; dedup267; route refusal283; System reaction301; jev318. Function names are the stable receipt join keys.

Test synchronization is pre-established mpsc/oneshot, bounded 2-second channel receives, and actor idle observation; no fixed sleeps or polling-delay assumptions were found in this target. FakeRunner never writes an assistant reply: observing its started event and messages inbound marker does not satisfy the planned live reply-row QA.

## Missing actual lead/log/drain evidence and current72 boundary

Required live evidence remains unestablished in the inspected artifacts:

1. A fresh todo12 question event linked by its exact lead_session_key and event IDs to a gateway lead-turn log, inbound System message and **assistant reply row in the same scratch DB**, including timestamps and session/thread identity. delivered_at/lead_turn is admission bookkeeping, not completed reply proof; lane claim rows and done.txt are not lead replies.
2. Actual workspace-root `.drain_request.json` recognized by DrainWatcher, the same queued event remaining undelivered during drain, then the same event delivered after marker removal/recovery. The raw fake test substitutes watch::Sender<bool>; neither it nor existing source approval is live marker proof.
3. A provenance-backed exact task14 QA record. Missing task-14.log must stay missing rather than being fabricated from next review or unrelated task8 outputs.

Existing actual8/9 artifacts must not be replayed or repurposed:

- `E/task-8-9-live-GhvMrT-1791737140862/receipts.json` records mode execute-8-9, binary60fce5ae, failure `8 turn file report claim: deadline exceeded` at16:55:41Z, owned gateway21976 exit0/absent and retained scratch cleanup. This is cleanup of that failed invocation, not a completed same-thread recovery cleanup.
- The **current scratch** `/private/tmp/omon-lane-scratch-qa-GhvMrT/receipts.json` instead records a later `lane reopen qa-1 --text ...` exit0 at16:56:55.136Z. Its private log records DB identity and service ready16:56:55, then instant steer16:57:15 on thread `01a12bdb-1fe5-7345-ae81-738e3573330f`. No current72 final recovery cleanup receipt was found at the inspected root; DB/WAL/SHM and task9-recovery-started.json existed. These files do not prove the process is currently alive or dead; no live process or DB inspection was performed.
- Treat current72 cleanup as an unmet handoff prerequisite, not a task14 action. Do not stop that run, write into its root, create its drain marker, reopen again, replay the original turns or replace its binary. A prior invocation's absent PID receipt is insufficient for this later invocation.

## Safe scratch-only extension after current72 cleanup (unrun)

This is a concrete future boundary, not permission or an executed QA result:

1. Receive current72's final recovery/cleanup artifacts proving its owned gateway exited, lane/control socket and lock released, and evidence retained. Preserve actual8/9 artifacts unchanged. No timer/model polling and no replay of prior initial/reopen turns.
2. Allocate a distinct private root with explicit scratch DB, HOME/XDG/TMPDIR/workspace/cwd and minimal allowlisted environment. Never inherit owner DATABASE_URL or dotenv; no production DB, Discord tokens, shared daemon restart or endpoint/model reconfiguration. Pin the exact candidate binary, source manifest and harness bytes. Binary60fce5ae is not a task14 candidate.
3. Resolve the wiring prerequisite before execution: neither existing Discord-free gateway branch starts EventDispatcher. A separately authorized isolated QA harness can compose the real next dispatcher, multiplexer, database and owned sink with Discord disabled; label it harness QA, not ordinary gateway QA. Ordinary debug-gateway acceptance remains open until a separately reviewed/authorized Discord-free integration supplies that path. This reconciliation does not port/build/edit product sources or propose switching Discord on.
4. Use fresh lane/lead keys and a unique task14 sentinel. Subscribe to the exact event insertion and runner/reply completion before triggering one fresh question; retain event ID/created_at, lead key, System inbound ID, raw lead log and assistant reply row from that scratch DB. Use a real released lead runner for live lead proof; fake started-event observation is insufficient. Model/backend execution requires a later release and is not performed now.
5. For drain QA, the owned watcher must use only the new root's workspace path. Subscribe before adding a marker there; retain watcher transition, route refusal and unchanged delivered_at for the exact event. Remove only that owned marker and retain delivery/reply evidence. Main ordinary gateway shutdowns on drain_rx.changed; if it exits, removal alone cannot restart it: capture shutdown and an owned same-scratch restart/recovery, or use a clearly labeled isolated watcher/mux harness that stays alive. Do not describe a watch boolean toggle as live marker QA.
6. Retain raw stdout/stderr, exits, hashes, correlated DB extracts and owned PID/socket/lock cleanup for the new invocation. Keep historical 8/8 receipt separate. Do not rerun unaffected previous turns or whole suites to manufacture this missing evidence. Close task14 only after the required surface and provenance are actually established; until then keep NOT CONFIRMED and the plan/ledger untouched.

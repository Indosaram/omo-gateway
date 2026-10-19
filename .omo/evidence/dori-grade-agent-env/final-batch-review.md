# Independent final batch source review

Reviewed 2026-10-12 in `/Users/indo/code/project/omon-gateway-wt/dori-work` against the requested plan's What to do, Must NOT do, and Acceptance sections. No implementation edits, cargo commands, runtime commands, or QA were performed. This report is the only written artifact. Live QA is separate and is not graded here.

Verdict: 8 CONFIRMED-SOURCE, 6 BLOCKER. CONFIRMED-SOURCE means the specified source acceptance is supported by code and existing acceptance tests, not that deployment or live behavior has been proven.

## Test evidence and scope

Evidence is `.omo/evidence/dori-grade-agent-env/final-verify.json`: fmt=0, clippy=0, build=0, source_cmp=0, baseline_comparison=0, `new_failures=[]`. The overall status is `failed`, tests exit 101, and final exit 1; do not describe the entire suite as green. Recorded baseline failures are cron boundary parity (1), cron executor parity (1), migrate (2), OMO backend (3), and wiring E2E (1). Todo 29's original sentinel failed; its recorded post-resync result is 0. The profile-only resync did not change the binary. The source manifests were identical during the recorded run; this review does not independently hash the currently shared checkout.

Relevant recorded passing targets: lane_close 27, lane_cli 7, lane_reopen 5, lane_dispatch 10, lane_done 11, lane_guard_alerts 5, lane_kibitzer 9, lane_status_transport 1, lane_post_transport 2, profile_import 7, inbox_triage 9, inbox_cli_digest 9, inbox_proposals 10, sessions_control 6, session_registry 7, cron_health 12 (2 ignored subprocess helpers), cron_remediation 6, cron_report 5, usage_done_gate 9, judge 22 (3 ignored). These counts establish execution, not completeness of the assertions.

Todo 35 is reviewed only for done_diff and judge_calls. The plan annotation and `qa3-owner-scope.md:1` remove token accounting. No missing token notification or live token count is graded as a failure. Remaining token code is separately reported as retained removed-scope implementation.

## Todo 15 - CONFIRMED-SOURCE

- Source: `src/lanes/runner.rs:352` removes only registered, identity-validated lane-owned worktrees using `git worktree remove` without force; branch read-back preserves branches. `:767` enforces the original verified state/force intent, applies thread lifecycle, records close failures, and commits closed only after successful removal. `:937` recovers intents, starts working/not_done lanes, settles claims, cancels and joins active lane tasks before close effects. `src/discord/adapter.rs:997,1652` consumes admitted owner replies before ordinary chat routing. Owner reply generation and deduplication are durable.
- Tests: `tests/test_lane_close.rs:171,255,283` cover close/reopen exactly once, dirty untracked preservation, refusal without force; its event_restart test reconstructs the dispatcher and proves one redelivery. The cancellation service test proves no thread/git effects before the canceled backend finishes. Recording sink checks Done/archive and Working/unarchive. Source adapter returns after interception.
- No branch deletion or forced dirty removal was found in the inspected close path.
- Live-only: actual Discord archive/unarchive, worktree disappearance after real verification, and dirty-lane end-to-end rejection.

## Todo 16 - CONFIRMED-SOURCE

- Source: `src/lanes/kibitzer.rs:168` installs durable producer triggers; `:208` checks arming, 30-minute spacing, KST hours and the six-wake daily cap; `:247` admits one generation, gates with jev, runs a tool-disabled small-model fresh session, parses strict JSON and atomically caps events. The state builder limits input to 6000 characters and removes oldest DM messages first. The capture has no owner transport.
- Tests: `tests/test_lane_kibitzer.rs:139` onward cover unarmed/disabled zero calls, threshold/low/nonfinite scores, invalid JSON, one wake, seventh-wake suppression, spacing/reinitialization, and Unicode bounds. Backend test asserts small-model routing. Local arm scans in serve do not invoke a model while idle; they are cheap durable-state checks, not periodic model evaluation.
- No direct owner message or external inbox read is present in this path.
- Live-only: one inserted scratch inbox item produces one real small-model decision; idle gateway produces zero kibitzer model turns.

## Todo 17 - CONFIRMED-SOURCE

- Source: `src/lanes/runner.rs:954,997` ticks the guard every 60 seconds and retries queued lanes. `src/lanes/guard.rs:577` commits watcher transitions only after event insertion; `:601` promotes queued lanes and emits launched atomically while recounting authoritative active rows. `HostSample::can_launch` at `:163` ignores CPU load as a hold criterion. Heavy commands use file-lock slots.
- Tests: `tests/test_lane_guard_alerts.rs:28,82` cover one event per transition and promotion after disk recovery even at high CPU, plus rollback and active-lane limit protection. Guard module tests cover slot acquisition.
- Live-only: high-disk-threshold CLI exit 4, promotion within two minutes after restart, and actual heavy-command serialization timing.

## Todo 18 - CONFIRMED-SOURCE

- Source: `src/lanes/posts.rs:178` drains each durable milestone intent once; `:237` refreshes status on milestone acknowledgment; `:275` keeps one status ID, rate-limits ordinary active edits to 15 minutes, renames prefixes on state changes, and replaces a deleted status message. HTTP edit maps Discord 404/10008 to replacement. Send/edit explicitly disable user, role, everyone and reply mentions.
- Tests: `tests/test_lane_status_transport.rs:93` uses production HTTP transport against owned loopback: two milestones/two edits, blocked and done prefixes, 15-minute boundary, 404 replacement ID persistence, scrubbed payloads and empty allowed_mentions. Uncertain creation is not blindly retried after restart.
- Live-only: actual Discord milestone message IDs and replacement after deleting the status message on Discord.

## Todo 20 - CONFIRMED-SOURCE

- Source: `src/profiles/lanes_block.rs:98` uses BEGIN IMMEDIATE and updates only system_prompt; preserves prefix/suffix, rejects malformed markers, returns unknown-bot exit 2, and emits digests. `src/entry.rs:75` exposes named apply-block flags and export. `bot_profiles/blocks/lanes.md:1` contains routing, brief, event, verification, kibitzer and consent-first onboarding rules. The current repo JSON contains both machine sentinels and the same marked block.
- Tests: `tests/test_profile_import.rs:11,119,145` exercise actual CLI flags, repeated idempotency, changed-region replacement, all other row columns byte-identical, and unknown ID exit 2. Fake block prose is test input, not an assertion pinning shipped wording.
- Residual context: repo JSON's unmarked original prompt still references omomeow-delegate, and custom_settings still name that legacy command. The block must govern routing when applied; the intended live DB prompt is deliberately not overwritten. This is a retained legacy instruction, not evidence of a newly invoked legacy execution path.
- Live-only: copied scratch profile applied to a new session invokes lane start; synthetic lane event produces lane send/object or an owner decision rather than raw forwarding. Existing-thread prompt refresh and live profile backup/application belong to deployment, not this source verdict.

## Todo 24 - BLOCKER

- Happy-path source and tests exist: `src/inbox/triage.rs:148` arms nonempty collection batches, classifies up to 20 items per chunk with jev, writes only owner/bot with a small model, validates JSON, caps payload at 6000 characters, and atomically stores decisions/urgent events. `tests/test_inbox_triage.rs:158-267` cover all listed batch, zero-work, malformed/unknown-ID, urgency and kill-switch cases.
- **Pending actionable items can be stranded indefinitely:** `src/inbox/triage.rs:169-177` clears the deadline before flushing. Unknown/missing IDs are left undecided by `:280-286`, but a successful partial flush never re-arms them. A malformed response or writer error at deadline similarly leaves no deadline. Future zero-insert collector passes return at `:160` and cannot recover them; a restart's deduplicated startup collection also has zero inserts. The test at `tests/test_inbox_triage.rs:243` only asserts zero stored rows, not eventual next-pass handling.
- Concrete fix: persist pending/admitted work and arrange a bounded event-driven recovery pass after a partial/error response and on startup, without idle model polling. Add a test where unknown/malformed writing is followed by valid writing with no additional source insert, and prove exactly one decision/event is eventually stored.
- No source action/send was found in triage itself.
- Live-only: real-source first collection triggers classification; repeated empty collection triggers no turn.

## Todo 25 - BLOCKER

- Source/tests support CLI filtering, due/received ordering, set statuses, unknown exit 2, normal owner-home-DM cron routing at 09:00 KST, no full snippets and auth suppression: `src/inbox/list.rs:40`, `src/inbox/set.rs:24`, `src/inbox/digest.rs:17`; `tests/test_inbox_cli_digest.rs:53-324`.
- **Empty task lists can still produce a routine digest:** `src/inbox/digest.rs:83-101` permits a digest solely because yesterday has judge_calls or inbox collection statistics (or retained token tiers). Thus all-ignore/FYI collection with no owner/bot tasks, auth problem or cron problem sends an empty task report. The empty acceptance test at `tests/test_inbox_cli_digest.rs:277` uses a completely empty DB and misses this case.
- Concrete fix: decide whether to send from actionable owner/bot lists, auth problems and the separately requested cron section only. Append judge statistics only after a useful digest is warranted. Test empty lists with nonzero judge/filter/inbox counts returning None.
- Live-only: real collected owner list and actual owner-DM digest delivery. Unknown-item exit 2 is already covered by source acceptance tests.

## Todo 26 - BLOCKER

- Approval subsystem is safe once a proposal exists: `src/inbox/proposals.rs:364-465` requires authenticated reference-matched approval, commits a CAS before effects, stores lane/outcome, and does not retry uncertain execution; `tests/test_inbox_proposals.rs:92-266` cover approval, duplicate/concurrent execution, unauthenticated replies and unapproved zero work.
- **Normal bot list/triage does not produce the numbered executable proposals:** `src/inbox/list.rs:27-87` neither includes proposal numbers nor calls propose. `src/inbox/triage.rs:294` stores bot decisions only. The only source caller of `Proposals::propose` is the separate `run_proposal` CLI at `src/inbox/proposals.rs:30`; the persona block's Inbox tasks section never instructs that command. `src/inbox/set.rs:25-43` simply marks an item approved when no proposal exists and starts nothing. Existing successful approval tests explicitly pre-create proposals. The numbered `제안` adapter path only displays already pending proposals.
- Concrete fix: wire trusted lead proposal creation/presentation into normal bot-item handling, include stable numbers in the requested task list, and ensure normal inbox set approved starts that item's prepared lane/direct action. Add an integration test beginning with a triaged bot item, obtaining the ordinary list, approving it once, and observing the linked lane.
- `OMON_INBOX_AUTO_LOOKUP=true` is not implemented: `src/inbox/proposals.rs:472` is an unconditional no-op, has no production caller, and both flag settings are tested as no-op. This is a dead compatibility surface and a deviation from the plan's optional approved low-risk auto mode. Remove the unused API if owner policy forbids auto lookup, or implement the explicitly configured read-only branch; do not silently advertise a functioning flag.
- No send/reply/react-on-owner-behalf API was found in the direct action interfaces.
- Live-only: approved bot item creates an actual scratch lane; unapproved item creates none.

## Todo 28 - CONFIRMED-SOURCE

- Source: `src/sessions/control.rs:113,228` builds local/escaped individual SSH argv, validates target and text, refuses remote multiline, checks fresh advertised controls, sends one UUID idempotency key and maps exit 3. `src/sessions/external.rs:59,147,189,280` registers external lanes, sends goal once, consumes cursor/deduped assistant output, accepts explicit claims, applies lane activity and supports release. `src/lanes/runner.rs:1093` uses the external driver rather than managed app-server turns. Remote fallback reads use a five-minute local timer, no model polling.
- Tests: `tests/test_sessions_control.rs:70-299` cover exact local/SSH argv, shell-sensitive text, exit 3, missing controls, multiline refusal, and working -> done_claimed -> verified_done under fake reads and MockClock. Source contains no process-kill operation or retry-on-owner-typing loop.
- Live-only: scratch local session pong/read and not-running mapping; actual remote interrupt/control support and absence of a usable remote event surface (required to justify fallback) must be documented by the separate QA track. No claim of remote live correctness is made.

## Todo 29 - CONFIRMED-SOURCE

- Source: `bot_profiles/blocks/lanes.md:39-44` directs immediate sessions list, at most 15-second cache reuse, compact machine grouping/home-relative paths/KST activity, busy last report, reachable-host results, and owner-only privacy. The current repo JSON contains `omo-gateway sessions list`; final-verify records sentinel-after-resync=0.
- Acceptance is the machine sentinel, not a new prose-pinning test. This todo explicitly requests no new Rust beyond the session controls/registry.
- Live-only: reply within 30 seconds, transcript sessions-list call, and no session listing in a shared-channel session. Those are deliberately not judged here.

## Todo 30 - BLOCKER

- Hook source is synchronous after terminal commit and fault-isolated: `src/cron/scheduler.rs:1400-1435,1181`; `src/cron/health.rs:7` classifies failures, delivery/stuck, expect-only empty success, fixes clean successes and emits durable events. Startup/missed hooks use existing scheduler boundaries. `tests/test_cron_health.rs:124-544` cover stated acceptance cases using the real scheduler, including startup missed, kill switch, hook DB failures and no timer loop.
- **A recurring identical failure disappears after its first recovery:** `src/cron/health.rs:81-91` uses UNIQUE(job_id,kind,signature) with ON CONFLICT DO NOTHING for all statuses. After `:116` marks that row fixed, another identical failure inserts zero rows, leaves the old row fixed, and emits zero events. No inspected test performs fail -> clean success -> identical fail. This defeats health/remediation for repeated incidents.
- Concrete fix: atomically reopen a fixed/ignored occurrence (or give incidents occurrence-specific identity) and enqueue exactly one fresh event while retaining deduplication for repeated callbacks of the same run. Reset remediation attempt state for the new incident. Add a real-scheduler fail/success/same-fail test and a same-run duplicate callback test.
- Live-only: cron problem/event timestamp near terminal commit and healthy run's zero model invocation.

## Todo 31 - BLOCKER

- Source supports template briefs, publishing ready-file/owner approval, direct quota check, native receipt verification, bounded rerun counter and natural success: `src/cron/remediate.rs:205,302,399,473,539`. Existing six tests cover paths, fourth-attempt refusal and natural success. No dashboard trigger, .env edit, model change or job deletion is performed by the remediation controller.
- **Diagnostic preparation errors strand a claimed problem:** `src/cron/remediate.rs:226` changes open -> remediating and assigns lane_key before fallible runs/log lookup. `:261` propagates a missing/unreadable gateway.log error with `?`; no lane exists and no rollback occurs. The rollback at `:265` handles only open_lane errors. A retry of the original event returns immediately because on_problem selects only open rows at `:205`. The native rerun error at `:338` likewise exits after a reserved attempt without recording owner action or recoverable error state.
- Concrete fix: prepare diagnostics before the durable claim, or encompass all preparation/execution errors in a recovery transition with machine-readable detail and a durable next event/owner task. Preserve the reserved counter and avoid blind reruns after uncertain execution. Test log_excerpt failure and rerun transport failure, asserting no nonexistent remediating lane or silent stranded problem.
- **Acceptance assertion missing:** `tests/test_cron_remediation.rs:97-110` verifies cwd and Done argv but never asserts that the lane brief contains the fixture error and log excerpt, although the plan expressly requires it. Add data-value assertions for the fixture cause and runtime excerpt in the generated request (not fixed prose wording).
- Live-only: actual scratch script remediation/marker creation, single native rerun with successful receipt, publishing no-rerun and owner task.

## Todo 32 - CONFIRMED-SOURCE

- Source: `src/cron/report.rs:65,121` builds enabled-job health, KST last success and problem/lane listing with OK collapse; `:151` validates a successful delivered receipt before building run_id/evidence outcome; `:177-228` durably queues scrubbed outcomes and consolidates/acknowledges them in the digest. Persona includes the health command sentinel and suppresses routine all-good/duplicate alerts.
- Tests: `tests/test_cron_report.rs:17-94` cover seeded report, lane keys/KST, run_id/evidence, failed-delivery rejection, empty cron section and digest outcomes.
- Outcomes are intentionally consolidated into the owner-home-DM daily digest rather than sent individually, consistent with the owner's standing consolidation instruction and the shipped persona block. The original immediate-per-outcome wording is superseded by that instruction.
- Live-only: health JSON following actual remediation and destination delivery/acknowledgment of the consolidated digest.

## Todo 35 - BLOCKER

- Remaining requested behavior exists: `src/lanes/settle.rs:128,187` runs done_diff after signals/worktree checks, uses lane-start revision and <=1500-character title/brief, rejects fulfills<0.65 or risk>1.5, stores numbers in history/objection and queues not-done; judge errors log/store skipped and proceed. `src/usage.rs:115-137` counts judge/fallback attempts. `tests/test_usage_done_gate.rs:287-351` cover low fulfills, high risk, exact thresholds, unavailable judge and disabled/capped no model fallback; seeded judge counts are asserted at `:106-156`.
- **Removed token-accounting implementation remains active and visible:** `src/agent/omo_backend.rs:1309-1324` still handles thread/tokenUsage/updated and awaits token storage; `src/usage.rs:46-80` still records model_usage, `:115` still queries token tiers for usage JSON, and `:153` still emits Opus/flash token totals. `src/inbox/digest.rs:83,108` invokes that retained line daily, so absence of notifications yields misleading zero-token reporting instead of removal. Four retained token-focused tests remain in test_usage_done_gate. These are not failed token requirements; they are code for scope the owner explicitly removed.
- Concrete fix: retain lane-start commit capture/context needed by done_diff and judge_calls reporting, but remove the token notification/storage aggregation, token CLI fields and digest token line; remove/update only tests for that removed machine-consumed behavior. Keep done_diff and seeded judge counts covered. This also removes unnecessary fallible token storage from the backend turn path.
- Live-only: real jev rejection of the unrelated README-only diff with numbers and persisted judge_calls; no live token assertion is required or evaluated.

## Review boundaries

No new live side effects were executed by this reviewer. No source fix is included. Passing target counts do not override the BLOCKER cases above. No additional clear branch deletion, forced worktree removal, unbounded retry or ping-everyone path was found in the inspected requested implementation. Unknown secret formats and general repository issues outside these paths were not exhaustively audited. The separate QA track owns actual Discord/API delivery, runtime timings and remote engine compatibility.

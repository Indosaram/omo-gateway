# Task10/12 current evidence map

Read-only reconciliation, 2026-10-11. No product/shared-file edits, tests, build, service changes, model turns or Discord operations were performed. Tasks10/12 remain open. Owner prohibition on direct Discord QA supersedes the plan's ignored LIVE manipulation scenario. This document prepares the local-owned QA path, not execution receipts.

## Authoritative inputs

Main .omo/evidence/dori-grade-agent-env-todo10-gate-review.md is absent. The actual review is dori-work/.omo/evidence/dori-grade-agent-env-todo10-gate-review.md. Read that review, task-10-evidence-map.md and full canonical main .omo/plans/dori-grade-agent-env.md tasks10/12. Historical executor task-10.log contains create/Done rename/archive/GET archived and invalid-channel HTTP404/code10003 output. This is historical bot API evidence, not fresh owner confirmation or current transport validation. Review APPROVE/scope-ready/globalpending is not permission to manipulate Discord. Name formatting tests alone do not close10.

Named task-10.json, task-10-fail.log and task-12.log are absent in the work evidence directory. Preserve task-10.log; do not rename it into a fresh execution artifact.

## Source identity and gates

Current source hashes follow. Old prepared manifest45200128 covers191 inputs but differs now in10 paths: Cargo.toml, src/drain_control.rs, src/inbox/sources/kakao.rs, src/lanes/mod.rs, src/models/events.rs, src/models/mod.rs, src/multiplexer/actor.rs, src/multiplexer/router.rs, src/storage/db.rs, tests/test_inbox_sources.rs. This is an observed point-in-time list, not an attribution to task14/23 or a current global green verdict. Old grouped gate summary has checks_failed=1; fixture-repair superseded its runner fixture issue, not later source changes. Old test passes are scoped historical proof only.

- src/lanes/discord_threads.rs: 1fae41cfbc3ad5598c0e729b408939181c504a2fcb4501be63afaac255002d0a
- src/lanes/activity.rs: 8ded771250c7c1f6a095b7e3a27522bd30683805d0b4732030dd81f872c0780c
- src/lanes/posts.rs: 7cc0a2caf6a8ccc288f9001737e2c59ddecc83993d5232ffc00028ea2ddd9230
- src/lanes/runner.rs: 9e9925c96befc31db85ed6df60cdf577ce86c904f2ce584eb3e30fe8c4a8b592
- src/agent/omo_backend.rs: 624a3ab6340e7171ffcbdf7361cfabd14615a6675cfc53a65b33405d12a14a6b
- tests/test_lane_events.rs: c60891e2488b0294d868e383d3691a3ebee8ddab0221732ac94017e36398e797
- tests/test_lane_reopen.rs: b6b255f85049762a2a900c49cab708c6c2e14375dd1f65642da73afb6a25bb1a
- src/lanes/cli.rs: cf5fa7d73c08f9677b890bf39b9059d0c5a67be4ff50f27ad46c8bdd7c46102d

Installed serenity0.12.5 client.rs SHA256: 1ae670d48264b6c241cc69565cf3990294bd22a7c9b8ed060afb052bd582d831.

## Existing task12 actual REPORT proof reused

GhvMrT real thread01a12bdb-1fe5-7345-ae81-738e3573330f completed two turns. Milestone REPORT at16:46:25.303987Z persisted a typed thread_post row7 with source identity and nonce. Its thread=null, state=pending, message_id=null, error=null. Two done_claimed events exist; no milestone event exists. That proves actual capture/persistence and selective event behavior, NOT a delivered thread message. aJ4w9r task8 claim/report remains separately proven. Reuse task-8-9-live-GhvMrT-1791737815001/task8.json, task9.json and authenticated-tool-proof.json, plus task-8-9-live-aJ4w9r-1791736412932/task8.json. Do not replay a backend turn to recreate this report.

Current source tests in test_lane_events.rs cover acknowledged_post_is_not_sent_again_after_recovery, ambiguous_post_stays_uncertain_without_retry, interrupted_send_becomes_uncertain_on_restart, missing_destination_never_calls_sink, stable_item_replay_does_not_duplicate_post, runner_question_pauses_after_one_turn_without_outcome_lock_deadlock, backend_captures_completed_lane_items_with_stable_identity and MockClock stale/restart cases. They are in-process fake PostSink/source coverage, not actual Serenity HTTP receipts. tests/test_lane_runner.rs three_errors_fail_with_dead_event asserts3 turns, failed and exactly1 dead event; this reconciliation did not run it.

Missing integration proof: current-source actual question REPORT -> blocked + exactly1 question event within2min + terminal turn with no chained turn; answer send/object -> exact resumed owner prompt; real HTTP sink acknowledgement -> sent plus returned message ID; stale restart latch/one post/event; bounded three-error failure/dead; interrupted send and no-blind-retry actual sink boundary. Historical source-ready receipts do not fill these runtime gaps.

## Allowed task10 local-owned real HTTP seam

Existing DiscordThreads::with_http(Arc<Http>) and free helper functions accept injected Http. Installed HttpBuilder::proxy at client.rs:129 redirects API requests to a supplied base (not CONNECT). Build an isolated evidence-only Rust harness linked to the current library with HttpBuilder::new("local-fixture-only").proxy("http://127.0.0.1:<owned-port>").build(), request timeout and loopback-only recording server. Validate exact installed route rewriting before executing; reject all unexpected host/path/method requests. No genuine token or forwarding proxy. Local fixture token belongs only to this isolated HTTP seam, never gateway env/config.

Subscribe server request signals before helper calls. Record method/path/body/order and bounded responses. Call actual create_thread -> set_status(Done) -> archive -> get_thread/is_archived, using valid Serenity Message/GuildChannel JSON fixtures with IDs. Assert initial Working/renamed Done names and100 Unicode cap, returned IDs and GET archived=true. Capture safe allowed_mentions on starter/post/edit; separately exercise unarchive and successful post/edit. Invalid ID returns local404 JSON code10003; assert typed DiscordThreadError status/code, not just Err. Never forward to discord.com.

Output task-10.json with source/dependency/harness hashes, owned endpoint, request trace, actual IDs from local responses, parsed outputs and numeric test exit. Output task-10-fail.log with exact404/code10003 machine receipt and exit. Labels must say local Serenity HTTP seam, not live Discord. This can close missing local integration proof; the plan's external LIVE requirement remains historical-only/owner-waived or owner-performed, never invented.

## Allowed task12 integration path

Use separate owned scratch SQLite and same local recording HTTP server. Inject DiscordThreads::with_http through existing PostSink and LaneRunner::with_post_sink, avoiding LaneRunner::from_env live-token construction. Feed existing real REPORT text/source identity into actual captured_item, preserving provenance; set only the fixture lane's destination to local fixture ID. Drain actual posts::drain with actual DiscordThreads sink: capture POST payload, nonce and enforce_nonce, returned fixture ID, then verify persisted sent state. This is reused-input integration, not another model turn or live delivery.

Restart recover/drain must make zero requests for sent records. For missing destination assert pending/error and zero requests. For request-observed connection loss/ambiguous failure assert uncertain, no message ID and zero second-send on later drain; interrupted sending -> recover uncertain. No automatic retry of uncertain records is promised. Subscribe exact channels/barriers before trigger and use bounded waits/MockClock; no sleeps or timer model polling. Record failure/retry counts and persisted records in task-12 local sink receipts. Run question/pause/resume and3-error fake backend against actual runner/DB seams; these are local integration tests, not a real model question lane. A fresh real model question QA remains missing unless separately released.

## Queued work, no fixes applied

No product defect is proven by this reconciliation. Evidence harness/tests can use existing with_http/PostSink seams without changing shared14 files. If actual local HTTP or runner integration reveals a source defect, queue exact failing request/state/source location and stop before product edits. Compile/gates must bind a newly settled source set; do not carry forward old global green against changed191 inputs. Local server and owned child only are cleanup targets; preserve historical artifacts and sole app-server. Final receipts require numeric child exit and PIDgone. Do not update plan/Boulder/todo completion from this map.

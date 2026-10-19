# Todo12 source handoff

DoneClaim: source-ready activity core and deterministic test sources only.
Integration, manual QA, focused batch tests and global gates are PENDING. This is
not todo12 completion and must not mark the authoritative plan checkbox complete.

## Owned source

- `src/lanes/activity.rs`: complete matching-key report parsing; persistent report
  and activity update; transactional blocked/history/event decisions; milestone
  post decisions without lead events; done-without-claim event and idle latch;
  bounded scrubbed ReportIntent Judge choice; concurrent claim wins; persistent
  20-minute stale latch, reset by any captured message.
- `tests/test_lane_events.rs`: 11 tests using actual in-memory SQLite, MockClock,
  and a subscribed channel for Judge observations. No sleeps or remote calls.
- `src/lanes/mod.rs`: added `pub mod activity;` during authorized integration.

The module intentionally returns thread-post intents, not transport receipts.
No new schema, model configuration, environment files, external service changes,
commits, live database access or Discord actions were performed.

## Runner edits already applied before ownership freeze

Line numbers below were captured when the parent froze runner edits. Todo9 may
move them while repairing its generation race. Do not overwrite its changes.

- Imports at 16-23: Judge/JevCli, read_transcript, SystemClock, LaneActivity.
- CapturingDispatcher at 121-125: calls captured after append_transcript and
  appends returned posts as transcript role `thread_post`.
- LaneRunner at 217-235: Judge field, default JevCli constructor, with_judge seam.
- run_lane at 348: activity instance.
- At 390: idle_without_claim waits for Notify or shutdown when inbox is empty.
- At 426: clears idle_without_claim latch when starting a new prompted turn.
- At 435-450: transcript start cursor and 60-second SQL-only quiet checks during
  an active turn, persisting post intents as `thread_post` transcript rows.
- Outcome SQL no longer advances last_activity_at: only captured output should.
- At 525-541: successful unreported turn invokes Judge; done reports cause a
  persistent idle latch. Three-error handling remains the existing runner path.

No runner edits occurred after the parent's explicit ownership freeze.

## Exact integration requirements for parent

1. Finish todo9's run-generation guard first. Old generation captures, quiet
   checks, Judge results and outcome writes must not mutate a reopened run.
   Core methods assume their caller supplies activity from the current run.
2. Preserve reopened owner text as the first new turn and missing-thread fallback.
3. Feed captured complete messages once. The existing dispatcher treats Stream
   chunks as messages: assemble complete report lines before parsing, reset
   activity for every chunk, and avoid duplicate Send/Edit replay effects.
4. Keep blocker/question pause until send/object. Current dispatcher writes
   blocked immediately; runner stops chaining at turn completion. If mid-turn
   cancellation is required, subscribe to capture decisions before backend run.
5. Route returned posts (currently transcript role thread_post) through todo18/19
   scrubbed thread delivery. A transcript intent is NOT an actual thread post.
   Stale latch plus event are atomic; post transport acknowledgement is not.
   A crash between commit and consuming the returned intent can lose the post;
   coordinate a durable post outbox rather than claiming exactly-once delivery.
6. Preserve idle_without_claim until lead input, including restart. Do not
   overwrite its latch in outcome bookkeeping before persisting the final
   decision; avoid processing the same message both at capture and turn end.
7. Inject shared Judge/clock into runner and dispatcher for deterministic
   integration coverage. Core tests inject MockClock, but current runner uses
   SystemClock. Judge is injectable with with_judge.
8. Register test_lane_events in Cargo.toml if required by coordinator convention.
   Run one coordinated batch including lane_events, runner, reopen and CLI after
   todo30 compilation repairs. No per-worker Cargo run was performed.
9. Coordinator owns main.rs heartbeat removal: remove HerdrCliLivenessChecker
   construction/import and legacy spawn if work_items has no remaining use;
   otherwise preserve WorkHeartbeatRunner without herdr. Retain reusable Clock,
   MockClock and SystemClock or coordinate their new import location.
10. Named scratch gateway QA after batch compile: emit a question report, read
    blocked status and exactly one question event from the scratch DB, observe
    no chained turn, send the answer, observe resume. Capture curated thread sink
    payload, stale restart evidence, and three-error dead event. No live DB or
    Discord UI QA is authorized by this handoff.

## Verification

`rustfmt --edition 2021 --check src/lanes/activity.rs tests/test_lane_events.rs`
exited 0. This validates formatting/parsing only, not Rust type correctness.
LSP diagnostics repeatedly timed out, then reported EPIPE; no clean LSP result.
Cargo tests, clippy, full gates and scratch QA were intentionally not run.
The parent-reported todo30 compilation failures are not independently reproduced.

## Nine adversarial classes

| Class | Observable source coverage / limitation |
| --- | --- |
| Malformed input | Foreign keys, unknown kinds, missing fields, empty report, pipes in body; unrun test |
| Prompt injection | Bounded scrubbed external excerpt; no shell evaluation; model resistance unverified |
| Cancel/resume | Caller generation fencing required; todo9 owns current lifecycle defect |
| Stale state | MockClock boundary, persisted latch after reconstruction, rearm after plain message; unrun |
| Dirty worktree | N/A: core performs no filesystem mutations |
| Hung commands | N/A core commands; Judge timeout/fallback belongs to existing Judge; runner hung-turn QA pending |
| Flaky tests | Fake clock and pre-created channel, no sleeps; runtime pass pending |
| Misleading success | Done report cannot verify/close; existing claim suppresses late reports; unrun |
| Repeated interruptions | Transaction rollback on event insertion failure; process interruption/generation integration pending |

Cleanup receipt: no child processes or services left running; no scratch lane,
external database or worktree created. Formatting processes exited. Test fixture
databases have not been instantiated because tests remain unrun.

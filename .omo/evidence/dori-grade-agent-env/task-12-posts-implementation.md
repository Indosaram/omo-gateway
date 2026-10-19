# Todo12 posts/capture implementation receipt

Source-settled implementation unit; integrated runner, compile batch, tests and
scratch QA remain pending. This is not completion of plan todo12.

## Implemented

- NEW src/lanes/posts.rs: typed JSON pending/sending/sent/uncertain post records
  in lane_transcript, transaction-local enqueue, stable stored nonce, conditional
  send claim, actual DiscordThreads-backed HTTP send, 30-second bounded send,
  returned message ID acknowledgement, interrupted-send recovery to uncertain.
  Missing destinations remain pending with explicit errors and no transport call.
  All transport errors/timeouts are conservatively uncertain, never retried.
- activity.rs: milestone and stale post enqueue inside the activity transaction;
  stale event/latch/post roll back together. captured_item deduplicates stable
  completed-item IDs per generation transactionally. Existing todo9 generation
  predicates remain intact. Unreported finished event now persists idle latch
  in the same transaction. StreamCapture replaces cumulative snapshots and
  finalizes once; it never concatenates snapshots.
- omo_backend.rs: narrow lane-only completed agent item dispatch via EditMessage
  with platform_message_id `lane-item:<turn-id>:<item-id>`. Also handles the
  bounded interrupt-drain completion branch. Non-lane behavior unchanged.
  Lane final aggregate emission is suppressed when completed items were already
  captured, preventing a duplicate final report. Missing item IDs use a stable
  text hash within the turn for compatibility with current fake backend tests;
  distinct identical-text ID-less items cannot be distinguished by that fallback.
- tests/test_lane_events.rs: source tests cover acknowledged send/recovery,
  ambiguous send/no retry, interrupted send recovery, stable item replay,
  stale enqueue rollback, cumulative snapshots, missing destination, generation
  rejection and a real local WebSocket fake proving completed-item identities.

## Exact parent wiring (runner remains todo9 owned)

1. Export `pub mod posts;` in lanes/mod.rs. Not edited in this unit because current
   ownership grants posts/activity/tests/backend only. Until export, references
   to super::posts and integration test imports cannot compile.
2. At startup, before sole post worker starts, call posts::recover(pool). Then
   posts::drain(pool, &DiscordThreads) once for pending restart work. Drain on
   committed post notifications thereafter; no model polling. The real sink uses
   DiscordThreads::http(), safe_allowed_mentions and the verified nonce API.
3. DELETE runner's plain-text thread_post append loops. Activity already inserts
   typed intents atomically. ActivityDecision.thread_posts remains informational
   compatibility data, NOT permission to enqueue again or proof of delivery.
4. Preserve platform_message_id on EditMessage. For `lane-item:` actions, call
   captured_item(key, content, generation, platform_message_id). The generation
   must be the immutable generation captured before starting that backend turn,
   not looked up from the row after dispatch. Its predicate uses latest reopened
   history ID (0 before first reopen), matching todo9's fence.
5. Keep a StreamCapture per active stream UUID. Touch activity for accepted
   non-final snapshots without parsing reports (captured_generation(key, "",
   Some(generation)) is the existing seam). Only process a final fallback stream
   as a complete message if no completed item was emitted; backend already
   suppresses the normal duplicate aggregate. Clear stream state at turn end.
6. Do not also reprocess completed messages in the after-turn transcript scan.
   Track accepted report_count and final assembled message for one unreported
   Judge call. Use unreported_turn_generation with the captured generation.
7. Preserve idle_without_claim set by activity through turn outcome bookkeeping;
   clear only for a new lead-driven turn under the same generation fence.
8. Bind quiet_generation to the running generation. Returned post strings are
   informational only; quiet already durably enqueues its post.

## API evidence and guarantees

Installed serenity 0.12.5 builder/create_message.rs:279-293 supports Nonce::String
and enforce_nonce(true). Official Discord docs corroborate deduplication only
"in the past few minutes":
https://github.com/discord/discord-api-docs/blob/main/developers/resources/message.mdx

No exactly-once or indefinite nonce guarantee is claimed. No automatic history
reconciliation was added: uncertain records retain an explicit error and require
resolution. This does not block ordinary pending posts from actual delivery.
The send uses the existing DiscordThreads HTTP handle because its convenience
post method cannot accept a nonce; existing helper source was not modified.

## Verification and cleanup

Rustfmt parser/formatter exited 0 for exclusively owned posts.rs and test file.
LSP still timed out. Concurrent activity/backend files were not whole-file
reformatted to avoid clobbering another worker's patches. Parent batch must run
formatting and type checks after ownership freeze. No Cargo command ran.
No live Discord/DB/service operations, no migration, no environment/config/model
changes, no runner/CLI/main/Cargo edits and no commits in this unit. No processes
left running. Local WebSocket fixture is source only, not executed yet.

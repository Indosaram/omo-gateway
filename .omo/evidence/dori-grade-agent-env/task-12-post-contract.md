# Todo12 proposed post and capture contract

Status: proposal only; no source edits in this inspection unit. activity.rs,
test_lane_events.rs and mod.rs remain frozen at the previous handoff. runner.rs
belongs solely to todo9 until the parent returns ownership. No posts.rs or
migration has been created. No Cargo run or live Discord call was made.

## Findings

- migrations/0031_lanes.sql: lane_transcript provides id/lane_key/at/role/text,
  with no role CHECK, delivery status, unique source identity or receipt field.
- src/lanes/discord_threads.rs:304-313 and :403-405: post returns a Discord
  Message, so successful delivery can record its ID. It has no idempotency key.
- src/models/events.rs:309-317: StreamChunk carries stream_id, sequence, content
  and is_final. src/agent/omo_backend.rs:1325-1335 emits full_content.clone(),
  NOT incremental delta text. Final emission passes rendered complete text.
- Backend item/completed replaces full_content with that item's text. The same
  stream_id spans the turn, not each agent message. Intermediate lane messages
  with no delta notifications are not dispatched in the current item/completed
  branch (:1280-1312). A capture-only change cannot recover those missing messages.
- Existing delivery_obligations is not a drop-in queue: startup recovery in
  src/ledger/service.rs:673-736 redispatches through generic outbound Stream,
  creates a new UUID and can add a recovered-reply prefix. It neither persists a
  Discord receipt for this purpose nor provides cross-crash deduplication.

## Recommended smallest contract, pending parent acceptance

Use one NEW posts.rs module and the existing transcript table, with one typed
JSON record per post under role thread_post. This needs no schema reservation.
The record contains source identity, frozen destination thread ID, scrubbed body,
state (pending/sending/sent/uncertain), and optional returned Discord message ID.
Do not label a pending or uncertain record delivered.

1. Insert the post record inside the SAME transaction as milestone report state
   or stale event/latch. Return its transcript ID, not an ephemeral text-only
   intent. This closes the current commit-to-intent gap.
2. Only the existing single lane-service owner drains posts, in ID order, using a
   conditional pending-to-sending update before transport. No second daemon,
   general queue framework, per-post task or model polling.
3. A narrow fakeable sink accepts destination/body and returns MessageId. The
   real sink delegates to existing DiscordThreads::post with safe mentions.
   Commit sent plus message ID only after an actual successful return.
4. Known pre-send validation failures remain pending with an explicit error.
   A timeout, connection loss, or process death while sending becomes uncertain,
   not a blind retry. This prevents duplicate delivery but cannot guarantee
   eventual delivery without reconciliation. Never claim exactly-once from SQL.
5. Parent must choose the uncertainty policy before implementation: recommend
   reconciliation through a stable transport-visible post identity and bounded
   Discord history lookup. That requires a small DiscordThreads sink extension
   and a settled marker/nonce contract; its feasibility and API syntax need a
   focused documentation/source check. A nonce alone must not be assumed to
   deduplicate indefinitely. If transport extension is not authorized, retain
   uncertain rows for explicit operator resolution and disclose that limit.
6. Stable source identity is run generation + completed item ID + report ordinal;
   stale uses generation + persisted stale episode identity. Under the single
   service writer, lookup/insert occurs in the same write transaction. Do not
   deduplicate by text: identical reports in distinct episodes can be legitimate.

This is preferable to adding a table now. If the parent wants DB-enforced source
uniqueness, indexed queue state, or multiple consumers, request one migration
number first and use a dedicated lane_posts table instead of hiding more SQL
machinery in transcript JSON. Existing plain-text thread_post intents from the
previous unexecuted implementation must not be parsed as sent receipts.

## Capture/assembly changes after todo9 release

- Bind capture to the generation token established by todo9 before backend run.
  Reject old-generation activity and outcomes transactionally.
- Touch last_activity_at/reset stale on every accepted chunk. Do not parse each
  cumulative snapshot as a new report and do not concatenate snapshots.
- Track the highest sequence for each active stream; ignore duplicate or older
  snapshots. Keep only the latest snapshot. Final content is authoritative.
- Process complete agent messages using an explicit completed-item identity.
  This is a required backend capture seam, not safely inferable from stream
  snapshot prefix changes. Parent should authorize the narrow omo_backend.rs
  lane-only completed-item dispatch or provide an equivalent notification seam.
  Finalization flushes an unterminated final report once; partial lines never
  trigger blocker or milestone actions.
- SendMessage is already complete; EditMessage replaces its named message, not
  a second append-only report. Persist accepted source identity together with
  activity/event/post effects so replay after restart is idempotent.
- On a successful unreported turn, invoke Judge once on the final assembled
  message. Persist idle_without_claim atomically with its event. Outcome updates
  must not overwrite that latch. Lead input clears it under the generation guard.
- Keep stream state bounded to active turns and release it on completion,
  cancellation or generation replacement. No timeout sleeps in tests.

## Deterministic sink/capture tests for implementation batch

Subscribe before action; use channels/barriers and MockClock. Required cases:
split report snapshots produce one post; repeated/older sequence produces none;
same text from different completed items remains distinct; final report without
newline is accepted; old-generation chunk is rejected; claim wins over Judge;
post intent and stale latch roll back together on insert failure; acknowledged
post survives restart without repost; pending post resumes; ambiguous send is
not blindly retried; reconciliation reuses the actual receipt; sink failure
never records sent; missing destination remains an explicit undelivered error.

Main heartbeat removal stays with coordinator27. Current tests and actual
question-lane/sink/DB QA remain pending the coordinated compile batch. Cleanup:
no processes, scratch databases, external messages or service state created.

# Task 16 implementation preflight

Status: read-only preparation, not implementation or a test receipt. The parent
must release implementation after frozen repair/static checks finish.

Source inspected: `/Users/indo/code/project/omon-gateway-wt/dori-next`.
Plan inspected: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.
All source references below are relative to dori-next and reflect the inspected
snapshot, not a promise that subsequent repairs preserve line numbers.
Only this evidence document was created in dori-work. No source edits, Cargo,
service operations, database connections, Discord actions, judge invocations,
or model/API calls were performed. Test references are inspected patterns, not
passing results.

## Exact scope and dependencies

Plan lines 334-341 require an event-armed `src/lanes/kibitzer.rs`: new inbox row,
lane event of kind `blocked`, `question`, `stale`, `not_done`, `cron_problem`, or
new owner message arms it. At most one evaluation per 30 minutes, only
08:00 <= KST time < 24:00; disabled means immediate return. Build open lanes,
home DM last 30 messages, last 10 lane events, and owner-preference memory into
at most 6000 characters, dropping oldest messages first. Gate with
`Judge::noul` >= 0.7, then one small-model turn in `cron:kibitzer`, fresh remote
thread, strict decision JSON. A positive decision queues a `kibitzer` event to
`OMON_LANE_DEFAULT_LEAD`; no direct owner message. At most six such events per
KST day, counted from midnight, not six model turns.

Named dependency sections read in full:

- Task 12, plan 290-298: meaningful report/quiet/dead activity becomes persisted
  lane events; milestones and routine activity do not wake the lead.
- Task 14, plan 312-320: queued events become coalesced System turns; drain,
  in-flight admission, and a 30-second lead-delivery bound remain authoritative.
- Task 22, plan 401-409: deduplicated inbox insert and cursor advance share one
  transaction; failed inserts do not advance cursors; metadata/snippets only.
- Task 33, plan 523-531: typed jev APIs, scrubbed external excerpts <= 4000
  characters, bounded fallback/caps/accounting, no large-model fallback.
- Task 34, plan 534-541: `ModelPurpose::Kibitzer` resolves small model; explicit
  conversation overrides and existing cron payload models are not changed.

The dependency table at plan 143 formally lists 12/14, but inbox arming and
judge/small-model execution also consume the task 22/33/34 contracts above.

## Identity: verified owner, unresolved canonical home DM

The human owner is **414011632306618368**, not whichever user happens to be first
in a plural admission allowlist. Evidence:

- Main workspace `.env:16` currently has that single `DISCORD_ALLOWED_USERS`
  value; this describes admission configuration, not an owner identity API.
- `src/inbox/prefilter.rs:27` defaults `owner_discord_id` to that exact human.
  `src/inbox/sources/discord.rs:17-20` also hardcodes it and separately reads
  `DISCORD_BOT_ID`.
- Stored reference `reference/omon-gateway-split-brain-2026-10-08.md` records the
  human ID, bot ID **1465631383862120451**, and live bot profile name `wawabot`.
  These are recorded historical facts, not a current database probe.
- `src/multiplexer/actor.rs:882-898` recognizes that bot ID, `omomeow`/`wawabot`,
  and optional `OMON_OMOMEOW_BOT_ID`. But 904-914 selects the first nonempty
  `DISCORD_ALLOWED_USERS` member as owner. Do not copy that heuristic into task
  16; report/coordinate its correction with the task 34 owner.

Main workspace `.env:18-19` configures `DISCORD_HOME_CHANNEL=1529738911377657967`
and an empty `DISCORD_HOME_CHANNEL_THREAD_ID`. That is a home channel setting,
not a verified private DM channel ID or canonical session key. No
`OMON_LANE_DEFAULT_LEAD` assignment appears in that file or in the inspected
`ai.omon.gateway.plist`. The plist runs the main workspace release binary with
that workspace as WorkingDirectory (plist 28-40); `src/main.rs:582` loads dotenv.
This inspection does not establish the live process's inherited environment.

Task 16 must validate the supplied `OMON_LANE_DEFAULT_LEAD` using
`SessionKey::from_storage_key` and require the intended Discord owner DM and
correct bot identity. Session keys retain the DM channel, user and optional bot
identity (`src/models/session.rs:43-105,110-241`); a plain `discord:<user>` string
is not the canonical format. Guild session construction clears `user_id`
(53-56), so owner messages in shared channels require actual `event.author_id`,
not the session user. Bind the state/memory lookup to the exact home DM key;
never select a newest arbitrary session or aggregate every admitted user's DM.
The parent must supply/resolve that exact key after release without guessing the
DM channel from the home channel. No live DB lookup was attempted here.

## Existing APIs and missing event contract

| Boundary | Existing evidence | Required task 16 hook |
| --- | --- | --- |
| Inbox insert | `src/inbox/mod.rs:185-259` private `commit_source`: `ON CONFLICT(source,source_ref) DO NOTHING`, `rows_affected`, item/decision/cursor in one transaction, commit at 258 | Arm only for actual new rows; rollback/duplicate/drop/error alone must not arm. Signal only after commit. |
| Inbox watcher | `src/inbox/watchers.rs:12-24,112-149` has one mpsc consumer of source-change events and optional progress results; `src/main.rs:1097-1111` consumes results only to log | Do not steal this receiver or arm on Startup/Periodic/GatewayMessage alone. `stats.inserted > 0` is a post-commit local signal opportunity, but not durable arming by itself. |
| Generic lane event | `src/storage/lanes.rs:610-633` `enqueue_lane_event(pool,lane_key,lead_session_key,kind,body)->Result<i64>` | Durable arm plus post-commit notification for eligible kinds; adding a hook only here misses direct SQL producers. |
| Transactional lane producers | `src/lanes/activity.rs:123-204,276,321,343-352`; `src/lanes/settle.rs:109-120`; `src/lanes/runner.rs:844-847`; `src/lanes/cli.rs:505-524`; `src/cron/health.rs:103-121,218-226` | Preserve each transaction; arm atomically with insert and notify after commit. CLI is a separate process, so local Notify alone is insufficient. |
| Ordinary owner message | `src/multiplexer/actor.rs:802-833` `persist_inbound` inserts transcript, then indexes/sends inbox watcher event | Qualify `origin=User` and actual owner author/DM identity; arm on new committed message, not assistant/System or conflict no-op. Current code discards insert row count. |
| Active-turn owner steer | `src/discord/adapter.rs:1333-1352` calls public `record_inbound_transcript`; implementation `src/multiplexer/actor.rs:1093-1129` | Must cover this second insert path too; it bypasses `persist_inbound`. Arm after transcript commit even if the separate search-index step fails. |

There is **no common committed-row notification API** in these inspected
boundaries. Existing inbox mpsc means source refresh, not committed kibitzer
input. Lane event inserts have no observer. Role `user` alone is insufficient:
System events are persisted as user-role rows with `[system]` text
(`src/multiplexer/actor.rs:1132-1137`); messages metadata stores attachments, not
a durable author/origin field (`802-820,1112-1128`). Do not identify owners by
parsing rendered message text or replay System rows as owner input.

## Minimal file ownership after parent release

Task 16 owns `src/lanes/kibitzer.rs`, one new migration at the next free number
(0037 is already occupied), and `tests/test_kibitzer.rs`. It needs additive
`src/lanes/mod.rs` module export and `Cargo.toml` test registration only if the
repository's explicit target convention requires it. The parent owns shared
startup/shutdown wiring in `src/main.rs`; start one worker and subscribe before
input producers, restore persisted pending work once, join on shutdown.

Concrete cross-owner changes needed, not optional future abstractions:

1. Inbox owner (22/23): expose committed-new-item signaling without changing
   collector semantics, and participate in durable arming in the insert transaction.
2. Lane owners (12/13/14 and CLI) and cron health owner (30): all direct SQL
   insertion sites above must participate; enqueueing `kibitzer` itself must not
   re-arm. Do not rewrite these modules from the task 16 branch.
3. Actor/Discord wiring owner: ordinary and steered owner insert paths need the
   same insert-count/identity-aware durable arm and post-commit signal contract.
   The public mirror currently returns `Result<()>`; its caller needs a way to
   distinguish a newly committed row from a duplicate.
4. Parent: settle an interprocess committed-event signal for CLI-originated
   writes before implementation. An in-process Notify cannot observe another
   process's SQL commit. A durable SQL arm plus only a startup scan does not
   satisfy timely live CLI arming. No existing IPC notification API was found
   in these producers; this preflight does not invent one.

Keep task-specific SQL/state logic in kibitzer; no generic event bus, alternate
backend trait, new daemon, recurring model cron, or source-collector redesign.
If the parent chooses SQLite insert triggers for inbox/lane durable arming, they
must filter actual inserted rows/kinds and still provide the post-commit signal.
Owner-message arming must use actual ingress identity because the messages schema
cannot make that decision reliably by itself.

## Persisted scheduling and concurrency requirements

`src/storage/db.rs:180-184` fixes the gateway SQLite pool to **one connection**.
`migrations/0031_lanes.sql:36-44` allows NULL lane_key and unrestricted kind;
0037 adds delivery_reason. There is no kibitzer scheduling state in the inspected
migrations. A MAX(kibitzer-event.created_at) lookup cannot enforce evaluation
spacing: a low score, false wake, invalid JSON or failed turn leaves no event.

Use a persisted row keyed by validated lead, with an arm generation, consumed
generation and last evaluation admission timestamp. These are required data,
not an existing API. Advance the arm generation atomically with the input insert.
Reserve an eligible generation and the 30-minute slot in one short conditional
transaction before any judge/model work; a simultaneous arm must survive the
evaluation's eventual completion. Never clear an unconditional boolean after
the model returns: that loses events arriving during the turn. A reserved
generation is consumed even for score < 0.7, malformed JSON or execution failure;
do not create automatic model retry polling from that outcome. Restart must not
erase the last admission or grant a second slot for the same consumed event.

One worker serializes evaluation. At decision insertion, atomically count today's
persisted kibitzer events for this owner lead and insert only if < 6. Count queued,
delivered and jev-skipped events alike. Do not couple the cap to delivered_at or
an in-memory counter. Use a half-open UTC range for KST day boundaries, derived
from UTC+09:00; persist/bind UTC timestamps consistently. Daily cap does not reset
the rolling 30-minute evaluation bound. At cap, pending real input may wait for
the next permitted KST window; it must not cause further model calls that day.

Never hold a SQL transaction or acquired connection across MemoryStore queries,
judge/model awaits, channel sends that may block, or pool-taking helpers such as
`enqueue_lane_event`. With one connection, a helper querying the pool while its
caller holds a transaction will wait on itself. For atomic cap-plus-insert use
SQL on the same transaction executor, not that pool-only helper.

Worker waits on committed-input notification, shutdown, or **one deadline only
while a persisted unconsumed arm exists**. Compute next eligibility from last
admission + 30 minutes, next 08:00 KST when outside hours, and next permitted day
if capped. Unarmed means no deadline, no judge/model calls. New signals coalesce;
recheck durable eligibility after wake. Restore pending state once at startup;
do not scan on an unconditional timer. Task 14's existing local 5-second queue
scan (`src/lanes/dispatch.rs:30-43`) is delivery infrastructure, not permission
to run kibitzer judgment every tick.

## State document and execution

- Query open lane status/last_report from lanes (schema 0031:13,20); choose
  nonterminal statuses explicitly, and keep owner/lead scope explicit.
- Query exact home-DM messages ordered newest first with a deterministic tie
  breaker, limit 30, then render chronologically. Last 10 lane events similarly
  use `created_at,id`; the existing undelivered API is not a history query.
- Call `MemoryStore::new(pool.clone()).search(&home_dm_key,
  "owner preferences", limit)` (`src/memory/store.rs:97-140`): it is session-scoped
  lexical ranking, not global memory or a file-memory reader. No matching terms
  legitimately returns no memory. Do not claim the persistent agent notes are
  available through this SQL API.
- Count Rust Unicode scalar characters, including section labels/newlines;
  drop whole oldest message entries first until <= 6000. If non-message sections
  alone overflow, bound those field excerpts/entries too, deterministically;
  dropping messages alone cannot guarantee the limit. Preserve valid Unicode.
- Gate with `Judge::noul(Purpose::KibitzerWake, state, instruction)`
  (`src/judge/mod.rs:283-288`). `scrub_for_external` masks tokens/paths and caps
  input at 4000 characters (`20-51`): the 6000-character internal document and
  the external judge excerpt are distinct limits. Do not bypass the 4000 cap.
  Accept only finite score >= 0.7. Typed errors produce no speculative wake.
- Jev may itself use a small-model fallback (`JevCli::new`, 663-705; fallback
  noul 480-501); conservative noul is 1.0 (107-109). Thus “one small-model turn”
  means one kibitzer decision turn after the gate, not a guarantee of one total
  network turn including judge fallback. Do not create another fallback layer.
- `AgentBackend::run/run_cancelable` returns `Result<()>`, not text
  (`src/agent/backend.rs:33-50`). Use `OmoBackend::new(config,capture_dispatcher)`
  (`src/agent/omo_backend.rs:88`) with a task-local capture only, never the real
  Discord dispatcher. Existing one-turn precedent is private
  `judge::Fallback::answer` (`src/judge/mod.rs:368-424`); it is not callable as a
  public generic writer API and uses a loose JSON parser unsuitable here.
- Resolve `model_for_purpose(ModelPurpose::Kibitzer)`
  (`src/agent/omo_config.rs:98-120`); set session active_model and config primary
  and fallback to that small tier. Empty enabled_toolsets, bounded cancellation,
  no reused owner SessionContext. Use a guild-less SessionKey with user_id
  `cron:kibitzer`; “cron:kibitzer” names the logical session, not a hand-written
  canonical storage key. Backend cron detection clears cached binding and does
  not persist a new one (`src/agent/omo_backend.rs:243-254,506-518`), yielding a
  fresh OMO thread each run. No Discord thread creation is implied.
- Capture completed output, not append snapshot stream text. Existing
  `StreamCapture::accept` (`src/lanes/activity.rs:29-48`) documents snapshot
  semantics; lane CapturingDispatcher writes lane_transcript and is not the
  appropriate store for this non-lane job. Judge fallback capture is private.

Strict contract is exactly
`{"wake":bool,"reason":string,"suggested":string,"target_lane":string|null}`.
Parse the complete trimmed reply with serde_json, reject unknown/missing keys,
wrong types, fenced/prose wrappers, trailing content and duplicate keys. Require
`target_lane` to be present even when null: a derived optional field alone may
accept missing as None. Invalid means warning and no event, not salvaging a JSON
substring. `parse_json_loose` (`src/judge/mod.rs:350-360,412`) must not be reused.
Warn without dumping raw private state/reply into logs. wake=false queues nothing;
wake=true queues the parsed decision body, scrubbed/bounded for event transport,
with kind `kibitzer`, NULL lane_key unless a validated existing lane is intended,
and the validated home DM lead. target_lane is advice, not permission to start
work or send externally.

Task 14 already recognizes kibitzer as immediate-batch eligible
(`src/lanes/dispatch.rs:75-91`) but **not critical** (`193-198`); it therefore
passes through its EventWake gate as specified by task 14. Do not silently add a
bypass. It routes a System event and records receipt only after route succeeds
(`164-189`); task 16 must enqueue only, leaving coalescing, drain and idle
admission to this existing owner. No direct owner send/reaction is needed.

## Deterministic tests to implement after release

Existing reusable evidence: `work_heartbeat::Clock/MockClock`
(`src/work_heartbeat.rs:70-107`); explicit timestamps and pre-established
mpsc/oneshot turn-start/release/end channels in `tests/test_lane_dispatch.rs:59-66,
106-171`; injected insert failure and transaction rollback in
`tests/test_inbox_store.rs:185-212`; fresh-cron binding assertion in
`tests/test_omo_backend.rs:1767-1803`. Inspect fixture suitability before copying;
none was executed for this preflight.

Task 16 tests should assert machine behavior, not prompt prose:

1. Unarmed 24-hour clock advance -> zero judge/decision calls; kill switch same.
2. Real committed inbox insertion arms once; duplicate/drop/rollback does not.
   Eligible lane kinds arm; milestone/dead/kibitzer do not. Owner ordinary and
   steered insert both arm; replay/System/assistant/non-owner do not.
3. Score 0.69 -> no decision turn; 0.70 -> exactly one. Positive valid reply ->
   one persisted event; false/invalid reply -> none. Verify small active model,
   no toolsets, no Discord dispatch and fresh thread binding.
4. 07:59 KST waits; 08:00 eligible; 23:59 allowed; next 00:00 waits until 08:00.
   29:59 since admission waits; 30:00 eligible only with a newer real arm.
5. Reconstruct worker on same scratch database after low score/false/invalid or
   positive admission: 30-minute bound survives. No new input does not retrigger.
   Arm while fake turn is blocked survives completion; simultaneous admissions
   cannot double-run or double-insert the sixth event.
6. Six today's queued/delivered/skipped events -> seventh absent. KST date
   rollover resets daily count but not rolling spacing. Use explicit UTC/KST
   instants, not the workstation date or ambient timezone.
7. Korean/multibyte oversized fields and messages -> <=6000 characters with
   oldest messages removed first; remaining section overflow still bounded.
   Judge gets scrubbed <=4000-character excerpt; home-memory query stays scoped.
8. Strict parser rejects omitted nullable key, duplicate/unknown keys, wrong
   types, fenced/prose JSON, trailing data. Assert parsed values, not prose.
9. Register completion/commit notification before triggering; use oneshot,
   barrier or bounded channel receipt, never sleeps or polling delays. A
   one-connection scratch pool test must expose nested-pool deadlocks. Parent's
   interprocess event hook needs its own deterministic committed-write receipt
   test, not a timer-based scan masquerading as notification.

## Release prerequisites and unverified work

The follow-up below resolves the canonical key and identifies the existing
cross-process transport. Parent release still gates implementation and shared
producer wiring. No implementation, diagnostics, test pass or live delivery is
claimed. The frozen tree remains untouched; this brief is the deliverable.

## Follow-up prerequisite resolution: current registry and existing wake transport

This section supersedes the initial unresolved-key/transport findings above.
The original inspection made no database connection. In this follow-up, the
user authorized current registry inspection: a short `bun:sqlite` connection
opened `/Users/indo/code/project/omon-gateway/omon_gateway.db` with
`{readonly:true,create:false}` and was closed in `finally`. Only routing ID/key
columns and bot ID/name were selected; no message content, state_json, profile
prompts, credential columns, updates, schema changes or service/network/model
calls were requested. No product files were edited and no Cargo ran.

### Canonical human-owner home DM key: resolved

Use this exact existing bot-partitioned key as the task 16 owner lead:

```text
7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451
```

Source: the current main-workspace SQLite **sessions registry**, not inferred
from a home channel or first allowlisted user. The read-only queries were:

```sql
SELECT session_key, platform, guild_id, channel_id, thread_id, user_id
FROM sessions
WHERE platform = 'discord' AND guild_id IS NULL AND user_id = ?;
-- parameter: 414011632306618368

SELECT bot_id, name FROM bot_profiles;
SELECT DISTINCT lead_session_key FROM lanes;
```

The matching row has platform `discord`, NULL guild and thread, channel
`1474231559895646240`, user `414011632306618368`; the canonical key's final
component is bot `1465631383862120451`. The profile row for that bot is named
`wawabot`. Other returned owner DM rows belong to other bots; a legacy unqualified
key exists for the same DM channel but must not replace the bot-partitioned key.
The registry returned zero lane lead rows, so no existing lane row independently
confirms its lead selection. No configured `OMON_LANE_DEFAULT_LEAD` assignment
was found in the previously inspected config. Therefore the key is resolved as
the **existing owner's wawabot DM identity**, while adoption of that key as the
default lane lead remains an implementation/config-resolution step. Do not edit
.env merely to satisfy that step; bind the confirmed identity through the
parent-approved supported lead resolution, honoring an explicit valid override.

Validation after release must round-trip via `SessionKey::from_storage_key`
(`src/models/session.rs:110-241`) and verify all six dimensions. Bot-profile
ownership is bot ID `1465631383862120451`; human owner is user ID
`414011632306618368`. `DISCORD_HOME_CHANNEL=1529738911377657967` remains a
separate guild/home-channel setting, not this DM channel. No message history or
network lookup was needed to identify it.

### Existing cross-process wake: resolved transport, missing fan-out

The initial producer-only scan missed the transport implemented in runner.rs:

- `src/lanes/runner.rs:187-201` private `socket_path(pool)` queries
  `PRAGMA database_list`, canonicalizes the main DB file, hashes it with SHA256,
  and uses `/tmp/omon-lanes-<first 24 hex chars>.sock`. A file-backed pool is
  required; different DB paths produce different endpoints.
- Public `runner::wake(pool)->Result<()>` at 203-217 sends literal `wake` through
  an unbound `tokio::net::UnixDatagram`. Missing socket and connection refused
  return Ok; other errors propagate. It is a **best-effort hint**, not a receipt
  that a consumer saw committed rows.
- `WakeSocket::bind` at 226-255 takes an exclusive file lock, replaces the socket
  path and binds one receiver. `LaneRunner::serve` owns it at 351-357. Do not bind
  another receiver or unlink its path from kibitzer.
- The receiver at 395-398 ignores payload and calls `notify_one` on active lane
  notifiers; the following loop re-reads runnable lanes (368-379). It currently
  does not forward a kibitzer notification.
- CLI calls wake **after** committed mutations: Object at
  `src/lanes/cli.rs:182-183` follows `change_claim`'s commit at 524; ClaimDone at
  174-175, Close at 243, Reopen at 247-250 and start at 406-408 use the same
  transport. `change_claim` inserts not_done/done_claimed at 505-506. Thus
  cross-process CLI input does not require a new IPC service.
- Activity, settler, generic enqueue and health producers listed above do not
  all call this wake after event insertion. Settler completion notifies lane
  tasks in the service (387-390), but that is not a kibitzer commit notification.

### Minimal concrete committed-insert contract

Reuse the existing endpoint and payload unchanged. Add one optional shared
`Arc<tokio::sync::Notify>` kibitzer hint to the existing lane service; runner's
sole socket receiver forwards `notify_one()` to it as well as retaining current
lane notifications. Startup wiring constructs the Notify before producers and
passes it to task 16's worker and runner. This is a task-specific fan-out, not a
generic event bus or new receiver. It must not perform judge/model work inside
the runner select branch. The worker treats a hint only as reason to re-read
persisted arm eligibility; unrelated CLI wakes cannot create an arm.

Persisted arm generation is authoritative, not the datagram. A minimal shared
SQL boundary is:

1. Task 16's migration supplies its persisted scheduling row and INSERT triggers
   for `inbox_items` and eligible `lane_events`. Triggers increment the pending
   generation **within the existing producer transaction**. This covers direct
   SQL inserts without replacing their existing CAS/history/cursor logic.
   `ON CONFLICT DO NOTHING`, filtered input and rollback yield no committed arm.
   Persist/configure the resolved singleton owner lead as state consumed by
   triggers; do not put an environment lookup or a guessed session into SQL.
   A disabled worker must not call models; retained arms are evaluated only
   under the normal enabled/eligible policy after release.
2. For eligible lane events, filter the five required kinds; never `kibitzer`
   or milestone/dead/close_failed. Owner lane events can scope by the confirmed
   lead. **Health currently writes lead_session_key=`cron:<job_id>`**, not an
   owner SessionKey (`src/cron/health.rs:103-108,218-222`). A cron_problem trigger
   must explicitly arm the configured owner lead for these health rows, rather
   than treating that synthetic key as a Discord destination. Leave task 30's
   health event format unchanged. A cron_problem is required input regardless
   of whether it ultimately needs direct owner attention.
3. Owner messages cannot be safely qualified by a messages-table trigger alone.
   The two ingress persistence owners must retain the insert's row count, check
   actual owner author/User origin and correct bot, and increment task 16's arm
   row using the **same transaction executor** as the transcript INSERT.
   Arm once on successful new insertion, not after search indexing. Shared
   guild sessions require `event.author_id`; DM session identity can qualify
   the known private owner path. Both ordinary and active-turn steer paths are
   mandatory. No messages/body parsing or model-based ownership decision.
4. After transaction commit, and after relinquishing its sole pool connection,
   a producer with a newly inserted qualifying row calls existing
   `runner::wake(pool)`. Ordinary/steer must signal even if later search-index
   work fails; inbox signals only if stats.inserted > 0; lane/settler/health
   signal only if their transaction actually inserted eligible events. CLI
   Object already signals after commit; keep it, avoiding a duplicate send
   if a deeper helper is also changed. Generic enqueue may send after its
   autocommitted insert. Producer owners retain their transaction boundaries.
5. The receiver hints the kibitzer worker. Worker registers/observes notification
   before checking state, or uses Notify's retained permit correctly, then
   atomically reserves a new generation and spacing slot. Duplicate hints do
   not double-run. Startup performs one durable pending-state recovery after
   socket binding; deadlines are installed only for actual pending arms.

One-connection constraint applies twice: arm SQL must execute on `&mut *tx`,
never a pool-taking helper while tx is alive; `runner::wake` itself queries the
pool through socket_path and therefore must run **after commit**, never from
inside that transaction. Bind/test transport against a scratch file-backed DB;
an in-memory pool cannot exercise its path contract.

Reliability boundary: current wake has no acknowledgement and suppresses absent
receiver errors. Commit-then-send has a crash window; a lost hint while the
gateway stays alive does not by itself get rediscovered until another commit
hint or restart. Persistent arms prevent loss across restart but are not proof
of guaranteed immediate live delivery. Do not label this best-effort contract
exactly-once notification or solve it with recurring model polling. If guaranteed
live receipt across producer crashes is a required stronger acceptance condition,
the parent must separately select a durable commit-observer/receipt contract;
none exists in the inspected Unix datagram API. The minimal proposal above uses
the repository's current wake semantics and makes that limitation explicit.

### Disjoint implementation ownership: task 16 versus task 15 race track

| Owner | Exclusive implementation paths and responsibilities |
| --- | --- |
| Task 16 | `src/lanes/kibitzer.rs`, its own scheduling/trigger migration and `tests/test_kibitzer.rs`: arm state, eligibility, document, judge gate, capture/strict JSON, cap/event enqueue. No runner lifecycle, close/reopen, settlement CAS or Discord adapter edits. |
| Task 15 runner-race track | `src/lanes/runner.rs` lifecycle/active task bookkeeping/generation/cancellation/close ordering and its race tests; the small optional Notify setter/forward in the existing socket receiver is delivered by this owner in that same file. Task 16 consumes it, never edits the select loop concurrently. |
| Parent/shared ingress owner | `src/main.rs` startup/shutdown plus additive module/test registration; `src/multiplexer/actor.rs` ordinary+public-mirror insert transaction/row-count hook; `src/discord/adapter.rs` steer caller wiring. If task 15 edits the adapter's earlier lane-reopen interception, this owner coordinates sequential merge rather than two writers. |
| Existing producer owners | Inbox `commit_source` post-commit wake; activity and settler post-commit wake; health post-commit wake; generic enqueue post-commit wake. CLI lifecycle edits stay task 15-owned; existing Object wake is preserved. Triggers mean no task 16 writer needs to patch these peer transactions. |

Task 15's preflight identifies runner::serve/verified_done_ready and CLI close,
reopen and adapter lane-reply interception as its race/lifecycle surface
(`task-15-implementation-preflight.md`, "Exact integration hooks and file
ownership"). Kibitzer does not modify `reopen_generation`, active task removal,
turn cancellation, or verified_done -> closed -> working ordering. Task 15's
schema changes for worktree ownership/close receipts and task 16's scheduling
migration need distinct reserved numbers assigned by the parent; no shared
migration writer. A `close_failed` event remains task 15 output and is not an
extra task 16 input kind.

When task 15 consumes an owner lane reply before chat persistence, it must report
that genuinely new owner input through the same durable arm boundary if the
task 16 requirement includes all owner messages. Its deduplicated lane-reply
receipt/inbox transaction is then the authoritative producer, not a fabricated
home DM message; arm within that transaction and wake after commit. This is a
cross-owner hook, not permission for task 16 to alter reopening logic.

Deterministic additions: use a file-backed scratch DB, subscribe to worker's
commit-observed/turn-start channel before invoking each producer, and receive
bounded signals rather than sleep. Assert rollback and duplicates leave the
generation unchanged; CLI Object signals the existing socket after durable
not_done; health's synthetic lead arms the resolved owner row; a datagram without
new generation makes zero judge/model calls; trigger-created arms survive worker
reconstruction; owner ordinary, steer and intercepted lane-reply commits each
arm once; an index failure after committed transcript cannot erase its arm.
Socket forwarding tests belong to the runner owner and must preserve all prior
lane-task notifications/cancellation-race assertions. No tests were run here.

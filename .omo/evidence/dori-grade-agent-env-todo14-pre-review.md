# Independent Source Pre-Review Gate: Plan Todo 14 (Deliver Lane Events as Coalesced Lead Turns)

- **codeQualityStatus**: CLEAR
- **recommendation**: APPROVE
- **reportPath**: `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo14-pre-review.md`
- **target**: Plan Todo 14 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:312-324`
- **implementation tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next`
- **pristine base**: `/Users/indo/code/project/omon-gateway-wt/dori-next-base`
- **worker receipt**: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-14-source.md`
- **review mode**: READ-ONLY source pre-review (no source edits, no cargo execution, no git execution in target trees)

---

## 1. Executive Summary

Plan Todo 14 implements the lane event delivery pipeline:
1. `src/lanes/dispatch.rs` (`EventDispatcher`):
   - Runs a 5-second tick loop taking undelivered lane events grouped by `lead_session_key`.
   - Delivers a group when its oldest event is at least 30 seconds old, or immediately when it contains `done_claimed`, `dead`, or `kibitzer`.
   - Enforces a durable rate limit of at most one delivered turn per lead session per 30 seconds (immediate kinds included; subsequent immediate events wait for the next 30-second delivery slot).
   - Gates turns when the lead session has a turn in flight, keeping events queued until the turn completes.
   - Collapses duplicate `(lane_key, kind)` pairs inside one group to the latest body while aggregating all constituent event IDs for delivery receipts.
   - Gates non-critical events through `JevCli::noul` (`Purpose::EventWake`, threshold 0.5); events below threshold are receipted as `jev_skip` and appear only in the digest without triggering a lead turn.
   - Exempts critical event kinds (`done_claimed`, `dead`, `not_done`, `inbox_urgent`, and `cron_problem` containing `"needs_owner"`) from the judgment gate.
   - Formats a single `InboundEvent` with `platform_message_id: "lane-event:<first>-<last>"`, `content: "[LANE_EVENT]\n<key> <kind>: <body max 600 chars>"` (replacing newlines with spaces), and `origin: EventOrigin::System`.
   - Routes via `SessionMultiplexer::route`; marks events delivered in SQLite only after `route` returns `Ok(())`; on error or drain, logs a warning and leaves events undelivered for subsequent ticks.
2. `migrations/0037_lane_event_delivery.sql`:
   - Adds nullable column `delivery_reason TEXT` to `lane_events` to distinguish digest-only skips (`jev_skip`) from delivered turns (`lead_turn`) without altering the byte-identical migrations `0031`..`0033`.
3. `src/models/events.rs` & `src/models/mod.rs`:
   - Defines `EventOrigin { User, System }` with `#[derive(Default)]` (`User` default) and `#[serde(default)]` on `InboundEvent.origin`.
   - Re-exports `EventOrigin` through `models/mod.rs` and crate root.
4. `src/multiplexer/actor.rs` & `src/multiplexer/router.rs`:
   - Adds atomic idle admission via `ActorCommand::EventIfIdle` and public idle probe `SessionMultiplexer::session_is_idle`.
   - For `EventOrigin::System`, sets `platform_message_id` to empty string inside the actor run loop, suppressing completion and error emoji reactions (`PROCESSING_SUCCESS_EMOJI` / `PROCESSING_FAILURE_EMOJI`).
   - Prefixes the inbound transcript row with `"[system] "` in both `persist_inbound` and `record_inbound_transcript`.
   - Skips inbound message search indexing and inbox event emission for System events, preventing recursive kibitzer loops.
5. `src/main.rs`:
   - Moves `DrainWatcher` initialization prior to `SessionMultiplexer` construction to pass `drain_rx` via `.with_drain_receiver(...)`.
   - Spawns `EventDispatcher.serve` into `lane_tasks` under `lane_shutdown` cancellation when `OMON_LANES_ENABLED` is true.
6. `tests/test_lane_dispatch.rs`:
   - Provides 8 deterministic, mock-time acceptance tests exercising the full behavioral envelope with zero sleeps or polling loops.

The implementation is verified to be sound, safe against single-connection SQLite deadlocks, race-free, robustly typed, and faithful to the plan specification. **Zero CRITICAL and zero HIGH blockers exist.** Approval is recommended.

---

## 2. Blockers (CRITICAL & HIGH Findings)

*None. Zero blocking issues identified.*

---

## 3. Medium & Low Findings (Non-blocking)

### Finding 1: Unreachable pattern arm for `ActorCommand::EventIfIdle` in actor main loop
- **Severity**: LOW (Informational / Code Cleanliness)
- **File & Lines**: `/Users/indo/code/project/omon-gateway-wt/dori-next/src/multiplexer/actor.rs:668-672`
- **Description**:
  In `SessionActor::run`, incoming commands are pre-processed at lines 245-253:
  ```rust
  ActorCommand::EventIfIdle { event, reply } => {
      if !pending_events.is_empty() {
          let _ = reply.send(Err(OmonError::Multiplexer("lead session is busy".into())));
          continue;
      }
      let _ = reply.send(Ok(()));
      ActorCommand::Event(event)
  }
  ```
  If `pending_events` is not empty, `reply.send(Err(...))` is sent and `continue` jumps to the next loop iteration. If empty, `reply.send(Ok(()))` is sent and the command is converted to `ActorCommand::Event(event)`.
  As a result, when the subsequent `match command` is executed (lines 264-672), `command` is never `ActorCommand::EventIfIdle`. The match arm at lines 668-672:
  ```rust
  ActorCommand::EventIfIdle { reply, .. } => {
      let _ = reply.send(Err(OmonError::Multiplexer(
          "idle event was not dispatched".into(),
      )));
  }
  ```
  is dead code required only to satisfy the Rust compiler's pattern exhaustiveness check.
- **Note**: This is standard Rust practice when transforming variants in a pre-match step without introducing a separate internal enum. No fix required.

### Finding 2: Clock-skew conservatism in rate limit duration check
- **Severity**: LOW (Design Observation)
- **File & Lines**: `/Users/indo/code/project/omon-gateway-wt/dori-next/src/lanes/dispatch.rs:88-91`
- **Description**:
  ```rust
  let last: Option<DateTime<Utc>> = sqlx::query_scalar(
      "SELECT MAX(delivered_at) FROM lane_events WHERE lead_session_key = ? AND delivered_at IS NOT NULL AND (delivery_reason IS NULL OR delivery_reason <> 'jev_skip')",
  ).bind(lead).fetch_one(&self.pool).await?;
  if last.is_some_and(|last| now.signed_duration_since(last).num_seconds() < 30) {
      return Ok(());
  }
  ```
  If system wall clock time rewinds across process restarts or NTP adjustments such that `now < last`, `now.signed_duration_since(last).num_seconds()` yields a negative integer, which is `< 30`. Delivery is held until `now >= last + 30s`. This behavior is conservative and desirable: it prevents burst turn spam if the host clock jumps backward.

---

## 4. Detailed Evaluation

### Check 1: Plan Fidelity
Every requirement from `- [ ] 14. Deliver lane events as coalesced lead turns` was checked against the implementation:
- **Tick 5 s**: Implemented via `tokio::time::interval(Duration::from_secs(5))` with `MissedTickBehavior::Skip` in `EventDispatcher::serve` (`src/lanes/dispatch.rs:32-34`).
- **Grouping by `lead_session_key`**: Implemented using `BTreeMap<String, Vec<LaneEvent>>` in `EventDispatcher::tick` (`src/lanes/dispatch.rs:52-58`).
- **Oldest >= 30 s or immediate for `done_claimed`/`dead`/`kibitzer`**: Implemented at `src/lanes/dispatch.rs:73-82`.
- **At most one delivered turn per lead session per 30 s**: Implemented at `src/lanes/dispatch.rs:83-91` using `MAX(delivered_at)` with `(delivery_reason IS NULL OR delivery_reason <> 'jev_skip')`. Immediate events wait for the next 30-second slot if a turn occurred within the window.
- **Never while lead session has turn in flight**: Implemented with two layers:
  1. Fast probe `self.multiplexer.session_is_idle(&session).await?` (`src/lanes/dispatch.rs:93-95`).
  2. Atomic admission inside the actor via `ActorCommand::EventIfIdle` (`src/multiplexer/actor.rs:245-253, 354-356`).
- **Duplicate `(lane_key, kind)` collapse**: Implemented at `src/lanes/dispatch.rs:97-108`. A `BTreeMap<(Option<String>, String), (LaneEvent, Vec<i64>)>` tracks the newest body by `(created_at, id)` while accumulating all constituent IDs in `Vec<i64>`.
- **Jev `noul` gate (threshold 0.5)**: Implemented at `src/lanes/dispatch.rs:110-141` for non-critical kinds. Events below 0.5 threshold are marked delivered with `delivery_reason = 'jev_skip'` via `self.receipt(&ids, now, "jev_skip")`.
- **Critical kinds definition**: `critical(event)` matches `done_claimed`, `dead`, `not_done`, `inbox_urgent`, and `cron_problem` containing `"needs_owner"` (`src/lanes/dispatch.rs:198-204`).
- **`InboundEvent` construction**: Exact shape matching plan:
  - `id: Uuid::new_v4()`
  - `session: SessionKey::from_storage_key(lead)?`
  - `platform_message_id: format!("lane-event:{first}-{last}")`
  - `delivery_id: None`
  - `referenced_message_id: None`
  - `author_id: None`
  - `origin: EventOrigin::System`
  - `content: format!("[LANE_EVENT]\n{lines}")` with one line per event `format!("{} {}: {}", key, kind, body)` (body limited to 600 chars, `\n`/`\r` replaced with `' '`)
  - `attachments: vec![]`
  - `received_at: now`
- **Route via `SessionMultiplexer::route`**: Invoked at `src/lanes/dispatch.rs:178`.
- **Mark delivered only after Ok**: `self.receipt(&ids, now, "lead_turn").await` occurs strictly after `self.multiplexer.route(event).await?` (`src/lanes/dispatch.rs:178-179`).
- **Never deliver while draining**: Checked in `tick` (`src/lanes/dispatch.rs:48-50`), before `jev_skip` receipt (`src/lanes/dispatch.rs:130-132`), and inside `SessionMultiplexer::route` (`src/multiplexer/router.rs:331-335`).

### Check 2: Worker Claim Verification (Actor Authorization & Start Reactions)
The worker's receipt stated:
> "The current actor contains no owner-authorization check or start-reaction dispatch: these occur upstream in the real Discord ingress adapter. System events enter directly through the multiplexer and bypass those ingress checks/reactions; actor completion and interruption reactions are explicitly disabled. No Discord adapter change is needed. Discord egress already ignores nonnumeric reply references, so `lane-event:<first>-<last>` is safe without modifying the peer-owned backend."

Source verification:
1. **Owner Authorization**: Inspected `src/multiplexer/actor.rs`. The only authorization-adjacent function in `actor.rs` is `is_owner_lead_session`, which is used exclusively to select the default model (`ModelPurpose::Lead` vs bot profile) when no model is explicitly set. The actual authorization rejection logic resides in `src/discord/adapter.rs:921+` (`is_user_authorized`) and `src/main.rs:440`. Because system events route directly through `SessionMultiplexer::route`, they bypass the Discord ingress filter entirely.
2. **Start Reaction (👀)**: Dispatched exclusively in `src/discord/adapter.rs:986-1002` (`new_message.channel_id.create_reaction(..., PROCESSING_START_EMOJI)`). `SessionActor` has never dispatched start reactions.
3. **Completion & Interruption Reactions**: In `src/multiplexer/actor.rs:324-328`:
   ```rust
   let platform_message_id = if event.origin == crate::EventOrigin::System {
       String::new()
   } else {
       reply_to.clone()
   };
   ```
   Because `platform_message_id` is set to `""` for `EventOrigin::System`, `reaction_msg_id` resolves to `None` in both completion (line 533) and stop/interruption (line 569) handlers. No outbound reactions are dispatched.
4. **Discord Egress Reply Reference**: In `src/discord/adapter.rs:2948-2951`:
   ```rust
   let reply_id = reply_to
       .as_deref()
       .and_then(|id| id.parse::<u64>().ok())
       .map(MessageId::new);
   ```
   A non-numeric `reply_to` (e.g. `"lane-event:1-5"`) returns `None` from `parse::<u64>()`, causing Discord egress to send a standard channel message rather than failing on an invalid Discord snowflake ID.
Worker claim is fully verified.

### Check 3: InboundEvent Construction Scan & Serde Compatibility
Every `InboundEvent` construction site across `src/` and `tests/` was inspected:
1. `src/lanes/dispatch.rs:164`: `origin: EventOrigin::System`
2. `src/main.rs:455`: `origin: omon_gateway::EventOrigin::User`
3. `src/models/events.rs:268` (`InboundEvent::message` helper): `origin: EventOrigin::User`
4. `src/multiplexer/actor.rs:161`: `origin: crate::EventOrigin::User`
5. `src/storage/db.rs:565`: `origin: crate::EventOrigin::User`
Zero direct struct-literal instantiations exist in `tests/` or other modules; all test and adapter call-sites use `InboundEvent::message(...)` and inherit `EventOrigin::User`.
Serde compatibility:
- `EventOrigin` derives `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]` with `#[default] User`.
- `InboundEvent.origin` is annotated with `#[serde(default)]`.
Historical or external payloads lacking `"origin"` deserialize cleanly to `EventOrigin::User`.

### Check 4: Correctness (SQL, Concurrency, SQLite max_connections 1)
1. **Single-Connection SQLite Pool Safety**:
   - `omon-gateway` enforces `max_connections(1)` on its `SqlitePool` (`src/storage/db.rs:182`).
   - Holding a transaction across an `.await` that acquires another connection or invokes `route`/`judge` causes an immediate fatal deadlock.
   - In `EventDispatcher`:
     - `take_undelivered_events` acquires, reads, and drops the connection before grouping.
     - `MAX(delivered_at)` query in `deliver_group` executes standalone and releases the connection.
     - `self.multiplexer.session_is_idle` and `self.judge.noul` execute without holding any SQLite transaction.
     - `self.multiplexer.route` executes without holding any SQLite transaction.
     - `self.receipt(&ids, now, ...)` opens its own dedicated short transaction (`self.pool.begin().await?`), executes batch updates, and commits immediately (`src/lanes/dispatch.rs:181-190`).
   - SQLite connection safety is preserved without deadlocks.
2. **In-Flight Detection & Admission Race**:
   - If a user prompt arrives after `session_is_idle` returns `true` but before `route` is called, the actor command channel handles it atomically:
     - `SessionMultiplexer::route` dispatches `ActorCommand::EventIfIdle` (`src/multiplexer/router.rs:341-346`).
     - If the actor has begun processing a turn or has pending events in its queue, `EventIfIdle` is rejected with `Err(OmonError::Multiplexer("lead session is busy"))` (`src/multiplexer/actor.rs:247-249, 354-356`).
     - `route` propagates the error, `deliver_group` returns `Err`, and the events remain undelivered in `lane_events` to be retried on the next tick once the turn completes.
3. **Clock Handling**:
   - `EventDispatcher::tick(&self, now: DateTime<Utc>)` takes an explicit timestamp. Production `serve()` passes `Utc::now()`, while unit/integration tests inject exact timestamps, eliminating race conditions and timing jitter.
4. **Error Handling & Panics**:
   - All `Result`s are propagated with `?` or logged with `tracing::warn!`.
   - No `unwrap()` or `expect()` calls in production code paths (`src/lanes/dispatch.rs`, `src/multiplexer/actor.rs`, `src/multiplexer/router.rs`).
   - Event body slicing uses `.chars().take(600)` on Unicode scalar values, avoiding UTF-8 boundary slicing panics on multi-byte characters (Korean text, emojis).

### Check 5: Compilation & Cross-File Interactions
- **Exports & Types**:
  - `src/models/mod.rs` re-exports `EventOrigin`.
  - `src/lib.rs` has `pub use models::*;`, exporting `EventOrigin` at the crate root.
  - `src/lanes/mod.rs` contains `pub mod dispatch;`.
  - `Cargo.toml` registers `[[test]] name = "test_lane_dispatch" path = "tests/test_lane_dispatch.rs"`.
- **Interactions with Todo 13 & Todo 18**:
  - Todo 13 added `src/lanes/settle.rs` and `test_lane_done.rs`; modified `cli.rs`, `runner.rs`, and `Cargo.toml`.
  - Todo 18 modified `posts.rs`, `discord_threads.rs`, `storage/lanes.rs`, and `test_lane_posts.rs`.
  - Todo 14's changes to `Cargo.toml`, `src/lanes/mod.rs`, and `InboundEvent` are strictly additive and non-conflicting. `LaneEvent` queries in `storage/lanes.rs` specify explicit column projections and remain compatible with migration `0037`.

### Check 6: Test Suite Discipline & Determinism
`tests/test_lane_dispatch.rs` was audited for test discipline:
1. `three_events_within_ten_seconds_coalesce_into_one_turn`: Verifies coalescing window holds at t=10s and delivers 1 turn with 4 lines at t=30s; verifies all 3 events marked delivered.
2. `done_claimed_delivers_immediately_after_idle_delivery_slot`: Verifies immediate bypass of queue wait and judge exemption (`judge.calls == 0`); verifies second immediate event after 31s also delivers.
3. `two_done_claims_five_seconds_apart_wait_thirty_seconds`: Verifies 30s per-lead rate limit holds even for immediate kinds (second claim held at t=5 and t=29, delivered at t=30).
4. `event_during_in_flight_turn_waits_until_turn_ends`: Verifies delivery is held while a user turn is in flight, and delivers immediately upon user turn completion.
5. `duplicate_lane_kind_pairs_collapse_to_latest_body`: Verifies deduplication to latest body ("latest"), single judge evaluation, and delivery receipting of both IDs.
6. `route_error_keeps_events_undelivered_for_retry`: Verifies drain rejection retains pending events in DB, and clearing drain delivers on the subsequent tick.
7. `system_event_triggers_no_outbound_reaction`: Verifies zero reaction dispatches and verifies `messages` table contains `[system] ` marker.
8. `jev_below_threshold_marks_digest_only_without_turn`: Verifies score 0.49 (< 0.5) marks event as `jev_skip` in DB without starting a runner turn.

All tests utilize `tokio::time::timeout(Duration::from_secs(2), ...)` on channel receivers, mock clock timestamps via `f.at(seconds)`, and terminal typing observer synchronization via `f.finish()`. Zero `sleep` calls exist.

---

## 5. Skills-Perspective Audit (`remove-ai-slops` & `programming`)

- **Skill Loading Status**: The `remove-ai-slops` and `programming` skills were not available as specialized skill tools in this runtime environment (only `gpt-image-gen` and `bun-1-4` were registered). Consequently, their documented criteria were evaluated directly from prompt instructions and architectural principles.
- **`remove-ai-slops` Pass**:
  - *Deletion-only / removal-verification tests*: None.
  - *Tautological tests*: None. Every test in `tests/test_lane_dispatch.rs` exercises end-to-end event flow across the database, dispatcher, multiplexer, and actor.
  - *Implementation constant mirroring*: None. Constants tested (30s window, 5s interval, 0.5 threshold, 600-char truncation) are protocol requirements defined in the plan specification.
  - *Unnecessary production data manipulation*: None. Parsing and formatting are strictly bounded to the plan's message format requirements.
- **`programming` Pass**:
  - *Brittle prompt tests*: None. Tests assert machine-readable structural properties (line counts, sentinel prefixes, database row states), not LLM prose.
  - *Untyped escape hatches*: None. `EventOrigin` is strongly typed with exhaustive enum pattern matches.
  - *Needless abstraction*: None. `EventDispatcher` is a lean concrete struct directly managing its tick and delivery loop without unnecessary intermediate trait hierarchies.
  - *Validation inside production code*: Session parsing occurs strictly at the storage/system boundary via `SessionKey::from_storage_key`.
- **Verdict**: Diff fully conforms to both perspectives; no violations found.

---

## 6. Final Status & Summary

- **codeQualityStatus**: CLEAR
- **recommendation**: APPROVE
- **reportPath**: `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo14-pre-review.md`
- **blockers**: None

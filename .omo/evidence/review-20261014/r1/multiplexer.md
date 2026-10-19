# Lane multiplexer

### [P1] Messages queued behind a running turn are lost on a graceful restart
- Location: src/multiplexer/actor.rs:869
- Trigger:
  1. A user sends a second message while a turn is running, and it cannot be steered into that turn. A message with an attachment is the clearest case, because adapter.rs:1279 only tries the steer path for attachment-free text. A steer that is rejected also falls through.
  2. The message goes through debounce, then `route_claimed_event_with_constituents`, then `SessionMultiplexer::route`. The actor queues it in `pending_events` (actor.rs, in-turn `Some(ActorCommand::Event(next))` arm). It is not written to `messages`, because `persist_inbound` only runs when the event is taken off the queue.
  3. As soon as `route()` accepts the event, the live path advances the backfill cursor past this message (src/discord/adapter.rs:1690, `advance_live_bot_cursor(&data.pool, &cursor_event).await;`).
  4. The operator restarts the gateway with SIGTERM, Ctrl+C or a drain marker, all normal deploy paths. `service_shutdown` fires, and the post-loop drain at actor.rs:869 only calls `complete_delivery(..., Err("session actor shutting down"))` for the queued event, then marks the session `resume_pending`.
- Impact: the message is permanently dropped and the user gets no reply and no error. Each recovery path misses it:
  - Restart recovery (`main.rs::recover_resume_pending_sessions_with_auth` via `storage::find_last_unfinished_user_turn`) rebuilds only the last transcript row. The queued message has no row, so recovery either replays the interrupted turn or, if that turn had already answered, clears the marker.
  - Startup backfill starts after a cursor that is already past the message.
  - The ledger row is now `failed`, which is reclaimable, but nothing re-fetches the message to reclaim it.

  Prior rounds recorded this as a P2 "unverified: depends on backfill" item. The cursor advance at adapter.rs:1690 confirms backfill cannot recover it. It meets the rubric's message-loss bar; I graded it P1 rather than P0 only because it needs a restart while a message is queued.
- Evidence (actor.rs:869-874):
  ```rust
  ActorCommand::Event(event) => {
      self.complete_delivery(
          event.delivery_id.as_deref(),
          &Err(OmonError::Multiplexer("session actor shutting down".into())),
      )
  ```
  This is the only thing that happens to a queued plain event at shutdown. The mid-turn `TurnOutcome::Shutdown` arm breaks into this same drain.
- Fix: in the shutdown drain, and when queueing during a turn, persist each `ActorCommand::Event`/`EventWithAck` as a user transcript row with its event id before exit. Recovery would then need to replay every unanswered user row, not just the last one. A smaller alternative: move the live cursor advance into the actor's successful `complete_delivery`, so backfill re-fetches failed queued messages.

### [P2] Restart-loop guard never prunes boot timestamps that are in the future
- Location: src/multiplexer/restart_loop_guard.rs:89
- Trigger: the wall clock steps backwards (NTP correction, a VM or host clock reset, a manual time change) after a boot was recorded. `check_and_record` then runs with `now` earlier than one or more stored boots.
- Impact: `boot_chain_at` skips stored boots with `boot > now` without breaking the chain or moving `previous`, and `drain(..start)` never removes them because they sort to the end. Every later restart counts them as part of the current chain until real time passes them. The breaker can therefore trip after fewer real restarts than `max_restarts` and skip auto-resume of in-flight sessions; the markers are kept, so turns wait for manual input.
- Evidence (restart_loop_guard.rs:89-90):
  ```rust
  if boot > now {
      return false;
  ```
  This keeps future entries inside the returned chain, and `record_boot_at` then saves them back.
- Fix: drop entries with `boot > now` in `boot_chain_at`, for example with `boots.retain(|&b| b <= now)` before computing the chain.
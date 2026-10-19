# multiplexer review

### [P1] /model during an active turn is reverted when the turn completes
- Location: src/multiplexer/actor.rs:542
- Trigger: A user runs `/model X` while a turn is running. The path is `execute_model_command` (src/discord/commands.rs:722), then `SessionMultiplexer::set_model` (router.rs:578-597). That writes `active_model=X` to `sessions.state_json`, then sends `ActorCommand::SetModel`. Inside the turn's select loop, the actor sets `self.context.state.active_model` (actor.rs:484). When the turn finishes successfully, `self.context = turn_context;` (actor.rs:542) replaces the actor's context with the clone taken before the turn started (actor.rs:364). `flush_if_dirty` then writes that state over the row.
- Impact: The user is told "Model switched to X", but the old model comes back both in memory and in SQLite. Every later turn runs on the wrong model.
- Fix: Before `self.context = turn_context`, carry over control-plane fields changed mid-turn, such as `active_model`. For example, record a pending model override in the SetModel arm and re-apply it after the assignment, before flushing.

### [P1] /reset during an active turn brings back the remote conversation binding
- Location: src/multiplexer/actor.rs:536
- Trigger: A user runs `/reset confirm:true` while a turn is running. `execute_reset_command` (src/discord/commands.rs:752-753) first sets `state_json='{}'` and deletes messages, then calls `SessionMultiplexer::reset`. The actor's mid-turn Reset arm (actor.rs:488-495) clears the state, sets `binding_cleared = true`, and breaks with `TurnOutcome::Shutdown`. Right after that, actor.rs:533-537 unconditionally copies `omo_thread_id` from `turn_context` back into `self.context`. The backend stores that binding in turn metadata at turn start (omo_backend.rs:337). The outer loop then breaks, and the final `flush_if_dirty` (actor.rs:791) writes a state that contains the restored `omo_thread_id`. `binding_cleared` only disables the SQL preserve branch, so the binding is written anyway.
- Impact: The user is told "Session context and memory cleared", but the next turn resumes the pre-reset remote conversation. The Shutdown arm also marks the session `resume_pending` (actor.rs:640-650), which a reset should not do.
- Fix: Skip the `omo_thread_id` carry-over at 533-537 when `self.binding_cleared` is true. Also, give a mid-turn reset its own outcome instead of reusing `TurnOutcome::Shutdown`, so it does not mark `resume_pending` or exit the actor.

### [P1] Queued user messages are silently dropped on shutdown, reset, or actor exit
- Location: src/multiplexer/actor.rs:788
- Trigger: A user sends message B while turn A is running. `route_claimed_event_with_constituents` (adapter.rs) records B's ledger row as 'in_progress' and routes it, and the actor queues B in `pending_events` (actor.rs:425-428). Then a SIGTERM or Ctrl+C runs `SessionMultiplexer::shutdown` (service_shutdown), or a mid-turn `/reset` arrives. The loop exits through the `TurnOutcome::Shutdown` break or the `is_cancelled()` check at the loop head, and neither drains plain events. The final drain only answers `EventWithAck` and `EventIfIdle`. Plain `ActorCommand::Event` hits `_ => {}` (actor.rs:788) with no `complete_delivery` call.
- Impact: B is permanently lost:
  - It was never written to `messages`, so restart recovery (`find_last_unfinished_user_turn`, db.rs:488) cannot see it.
  - Its ledger row stays 'in_progress', so `record_incoming_as` (ledger/service.rs:126-135, reclaims only 'failed') treats any redelivery as a duplicate.
  - The live path already moved the channel cursor past it (adapter.rs:907), so backfill will not fetch it again.
- Fix: In the final drain, and in the shutdown/reset outcome paths, persist each pending `ActorCommand::Event` as an inbound user message before exit so recovery can replay it. At minimum, call `complete_delivery(p.delivery_id, Err(..))` so the ledger row becomes 'failed' and can be reclaimed.

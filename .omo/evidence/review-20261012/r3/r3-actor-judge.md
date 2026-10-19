# r3-actor-judge review

### [P2] Events queued before a mid-turn /reset still run after it
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:504 (Reset arm in the turn loop) and :663 (TurnOutcome::Reset handler)
- Trigger: Turn A is running, the user sends B (pushed to `pending_events`), then sends `/reset`. The Reset arm breaks out of the turn without draining `pending_events`. The main loop then pops B and runs it against the freshly reset context.
- Impact: A message the user sent before the reset is processed as the first turn of the "cleared" session. The Stop arm and the shutdown arm both drain and fail queued events, so reset is the odd one out. Nothing is lost or corrupted, and B's delivery and ack still resolve through the normal turn path. It is only a semantic question of whether pre-reset queued input should survive.
- Fix: If pre-reset input should be discarded, drain `pending_events` in `TurnOutcome::Reset` the same way the Stop arm does: `complete_delivery` with a "session reset" error, plus `ack.send(Err(..))` for `EventWithAck`. If survival is intended, add a one-line comment saying so.

### Items checked with no finding
- **Reset handling (mid-turn and idle):**
  - `reset_state` rebuilds the profile defaults through `fresh_context`, clears `omo_thread_id`, and sets `binding_cleared` and `dirty`.
  - The mid-turn Reset arm breaks to `TurnOutcome::Reset`, so the loop continues and the actor keeps running.
  - The `binding_cleared` guard skips copying the thread id back after the break.
  - `interrupt_turn` runs before `reset_state`, so `runner.cancel` still sees the old context. It rolls back partial history and marks the delivery failed.
  - Typing is released, and the pending ack resolves with Err("session reset").
  - `flush_if_dirty` writes with `preserve_binding=0`, so the old binding is not resurrected.
  - If that flush fails, `dirty` and `binding_cleared` both stay true and the error goes to the reply.
  - `GetContext` after reset returns the fresh state.
  - No resume_pending is marked, because the actor never reaches the Shutdown arm.
- **`fresh_context`:** It is a faithful extraction of the old `None =>` branch of `load_context`. It cannot return Err, so the `SessionState::default()` fallback in `reset_state` is unreachable but harmless. The bot_profiles columns match migration 0014.
- **Tests:**
  - `idle_reset_keeps_profile_toolset_restriction` would fail on the old reset (toolsets would be None), so it does exercise the fix.
  - `mid_turn_reset_keeps_actor_serving_the_session` is sound. The "blocking" turn is dropped on cancel, and "after-reset" doesn't touch the barrier.
  - The re-pointed `shutdown_drain_fails_queued_plain_event_delivery` still exercises the drain. Buffered messages are received before the `None` from dropping `cmd_tx`, so the queued event lands in `pending_events` and the `None` arm marks it failed through `complete_delivery`.
- **Judge diff:**
  - `write` and `child.wait_with_output()` are joined inside one `tokio::time::timeout(15s)`. Stdout and stderr are drained concurrently with the stdin write, so there is no pipe-full deadlock.
  - `child.stdin` is taken before `wait_with_output`, and the local `stdin` drops after `flush`, so a reading child gets EOF.
  - If the child exits early without reading stdin, `write_all` returns EPIPE, which is ignored (Rust ignores SIGPIPE). The write future then completes and the wait returns, so there is no deadlock.
  - On timeout the future is dropped. `Child` lives inside it and `kill_on_drop(true)` is set, so the child is killed.
  - The last test uses `exec sleep 120`, so the killed process is the sleeper itself and holds no stray pipe. It asserts only that `diff` returns inside an outer 60s timeout. That is correct, since DoneDiff then returns `FallbackFailed` regardless.
  - The test takes about 15s of wall time by design, and `Duration` is already imported.

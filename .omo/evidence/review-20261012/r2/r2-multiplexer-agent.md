# r2-multiplexer-agent review

### [P1] Idle `/reset` wipes profile-derived session state (toolsets, system prompt, lead model)
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:740-747 (compare `load_context` at :1077-1185)
- Trigger: A bot session whose profile limits `enabled_toolsets` (for example no terminal), or sets a system_prompt or lead model. The actor is live and idle, and the user runs `/reset`.
- Impact: The idle Reset arm sets `self.context.state = SessionState::default()` and never re-applies the profile defaults. Until the actor is evicted and reloaded, turns run with `enabled_toolsets=None`, no system prompt and no lead model. The omo_backend terminal approval gate relies on `enabled_toolsets` and is skipped in that state, so a toolset restriction stops being enforced.
- Fix: On Reset, rebuild the state through the same profile and ProfileRouter defaults that `load_context` uses. Do the same in the in-turn Reset arm (:499-508).

### [P1] Mid-turn `/reset` terminates the actor, so messages sent right after it are failed or dropped
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:499-508 (`break TurnOutcome::Shutdown`), :662-686 (Shutdown arm), :799-830 (final drain)
- Trigger: `/reset` while a turn is running, then a new message within the roughly 5s that `interrupt_turn` takes.
- Impact: The Reset arm replies Ok and then exits the actor. Anything already queued, or sent before `receiver.close()`, is drained and completed with "session actor shutting down". Events with a delivery_id are marked failed, so the user gets no reply. Events without one vanish. None of them were persisted, so `find_last_unfinished_user_turn` cannot recover them.
- Fix: Do not exit the actor on a mid-turn Reset. Cancel the turn, clear the state, keep the loop running, and process pending events against the fresh state. Alternatively, re-route the drained events through the router.

### [P2] Zero-config daemon is spawned with `--ws-auth off`
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs:225-229, called from /Users/indo/code/project/omon-gateway/src/main.rs:895 and :1020
- Trigger: A default autospawn install, where `OmoBackendConfig::from_env()` leaves `auth_token` as None. Any local process, or a web page opening `ws://127.0.0.1:19742`, can then connect.
- Impact: The token-file fix only applies when a token is configured. By default the daemon accepts unauthenticated connections and the gateway auto-approves tool requests. I did not verify the omo daemon's Origin handling, so I grade this P2. It becomes P1 or higher if Origin is not checked.
- Fix: Generate a random token when none is configured, write it with the existing 0600 token-file path, and give the same token to the backend. Also avoid sending the Bearer token to a squatter on the port when `probe_readyz` succeeds.

### [P2] Fallback retry ignores `messages_delivered`
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:1605-1617 (fallback_eligible gate)
- Trigger: A turn has already delivered a message mid-turn (relay, status or first agentMessage), then `turn/completed` fails with 429/5xx and `total_tool_calls == 0`.
- Impact: The turn is resubmitted on the fallback model and may post a duplicate reply. It also gets a fresh full deadline, which can double the total time.
- Fix: Require `messages_delivered == 0` in the gate, and carry the remaining time into the retry.

### [P2] Interrupt drain stops at the first non-text frame
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs (interrupt drain, about :958; `while let Some(Ok(Message::Text))`)
- Trigger: The server sends a Ping, Pong or Binary frame while the drain is waiting for the interrupted turn to finish.
- Impact: The drain ends early and a turn that could still complete is reported as a deadline failure.
- Fix: Skip non-text frames and keep draining until the deadline. Related: `is_model_rejected_error` matches the bare substring "thread/settings/update", so any error text that mentions that method is treated as a model rejection.

### [P2] Router holds a DashMap guard across `.await`
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs:612, :620, :627, :634 (`set_model`, `set_yolo`, `reset` and the adjacent method at :634, via `self.sessions.get(key)`). Contrast :480, :500, :514, :528, which clone the Arc first.
- Trigger: `/reset`, `/model` or `/yolo` is parked awaiting an actor reply while `remove_handle`, or the actor-exit `remove_if` in `spawn_actor`, writes the same shard.
- Impact: The writer blocks a runtime worker thread, and can deadlock on a current_thread runtime. This is most reachable with the mid-turn Reset, which exits the actor.
- Fix: Use `.get(key).map(|e| e.clone())` before awaiting, as the other methods do.

### [P2] Actor lifecycle edge cases
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:468 and :530 (in-turn `_ => {}` arms), :712 (Stop `_ => {}`), :757-759 (idle WaitIdle), :832 (final drain `_ => {}`)
- Trigger:
  - `wait_for_session_idle` is called, and then Stop, a mid-turn Reset or a shutdown arrives while the WaitIdle reply is queued.
  - `/yolo` is toggled mid-turn.
  - A turn runs longer than the 10 minute idle_timeout.
- Impact:
  - The queued WaitIdle reply is discarded, so the caller gets "actor closed before idle barrier".
  - The in-turn `/yolo` update changes `self.context` but not the running `turn_context`, so the current turn ignores it.
  - `last_active_at` is not refreshed when a turn completes, so the actor can be GC-evicted right after a long turn finishes.
- Fix: Reply `Ok(())` to queued WaitIdle on those paths. Refresh `last_active_at` on turn completion. Propagate yolo to the running turn if mid-turn `/yolo` is meant to apply immediately.

### [P2] llm.rs stream edge cases
- Location: /Users/indo/code/project/omon-gateway/src/agent/llm.rs:169-170 (`read_timeout(120s)`), and the SSE parse loop
- Trigger: A reasoning model is silent for more than 120s between chunks. Separately, an SSE body ends without `[DONE]` or `message_stop`.
- Impact:
  - The 120s timeout is per read, so long silences fail the request. This is a trade-off rather than a bug.
  - A stream that ends cleanly without a terminator is reported as success with a final chunk.
  - The line buffer is unbounded if the server never sends a newline.
- Fix: Treat EOF without a terminator as a truncation error. Cap the buffer size. Make the read timeout configurable.

Verified OK, no finding:
- Mid-turn SetModel and SetYolo survive turn completion through the `mid_turn_*` snapshot compare.
- `binding_cleared` prevents `omo_thread_id` from being resurrected in `flush()` and in the post-turn copy.
- The Shutdown arm skips resume_pending when `binding_cleared` is set.
- The omo_backend active_turns cleanup is correct on every error path I read. The stream-error and connection-closed paths keep ownership deliberately.
- Discord `stream()` keys streams by (identity, stream_id), removes the entry on a final chunk, and a later chunk with the same stream_id starts a new message (adapter.rs:3053-3150). Repeated final chunks with the same stream_id in `deliver_rendered_result` therefore post separate messages instead of overwriting. The reused obligation id `obl:turn:{stream_id}` only affects recovery-sweep content, which is minor.
- In-stream `stream_error` frames in llm.rs ignore `error: null`, and the error reaches both the stream and the tool_calls channel.
- The agent_workspace slug is ASCII-only after sanitizing, `.` and `..` are hashed, and zero timeouts are rejected in omo_config.
- The default `AgentBackend::run_cancelable` races `run()` against the cancellation token and calls `cancel()`, so it is cooperative.

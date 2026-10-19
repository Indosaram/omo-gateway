# Lane agent-backend

### [P1] A tool call that stays quiet for 180s aborts the turn, SIGKILLs the shared daemon and resubmits the prompt
- Location: src/agent/omo_backend.rs:2020
- Trigger: In an interactive Discord turn, the agent starts one tool that sends no frames for more than 180s. Examples are an omo `task`/subagent or MCP call that takes minutes, or a long command with no output. The only `item/started` frame comes at the start, and nothing else arrives until the tool completes. For interactive sessions the inactivity timeout is `request_timeout.min(180s)` (line 1212). When it expires, the code at 1216-1229 returns `"timeout during turn streaming: no activity from server"`.
- Impact: `is_backend_stall` (line 1857, matching `"no activity from server"` at 1860) classifies this as a wedged daemon. `force_restart_owned_daemon` then sends SIGKILL to the gateway-owned app-server (omo_daemon.rs:685), and the same prompt is re-run through `execute_turn`.
  - **Repeated side effects:** tools the first run already finished execute again, which breaks the "NEVER resubmit" invariant stated at line 1994.
  - **Collateral failures:** every other session's in-flight turn on that daemon loses its socket and fails with "connection closed before turn completion".
  - **Likely repeat failure:** the retried turn probably hits the same quiet tool and fails again; the 120s restart throttle only stops a second kill.
- Evidence: `if result.as_ref().is_err_and(is_backend_stall) && crate::agent::force_restart_owned_daemon(&self.config.appserver_url).await`. A healthy turn with a long tool cannot be told apart from a wedged daemon, because the only signal is a frame gap.
- Fix: Treat inactivity as a stall only when no item is open. Track `item/started` ids that have no matching `item/completed`, and skip both the inactivity abort and the stall restart while any of them is outstanding. Otherwise, limit the restart-and-retry to the case where `total_tool_calls == 0`.

### [P2] Cancelling before the turn/start ack returns "cancelled" while the remote turn keeps running
- Location: src/agent/omo_backend.rs:2087
- Trigger: A user sends /stop (or the actor cancels) after turn/start is sent but before the ack. This window can last a long time when the prompt is queued behind a background run; the ack window keeps extending at line 1517.
- Impact: `cancel` returns `Err("cancellation unresolved ...")`. `AgentBackend::run_cancelable` (src/agent/backend.rs:47) discards that with `let _ = self.cancel(session).await;` and reports the turn as cancelled. The prompt still runs on the app-server, tools and side effects included, but its output never reaches the user. The leftover `ActiveTurn` has `turn_id: None`, so the next turn's Occupied branch has no turn to interrupt and the new prompt queues behind the orphaned run.
- Evidence: `let Some(turn_id) = active.turn_id.as_deref() else { return Err(OmonError::Llm(format!("cancellation unresolved: ..."` together with `let _ = self.cancel(session).await;`.
- Fix: When `turn_id` is None, interrupt by thread instead: open a socket, wait briefly for the `turn/started` of that thread, then interrupt it. At minimum, surface the unresolved-cancel error from `run_cancelable` instead of discarding it.

### [P2] Pre-ack relayed messages and the final reply share one delivery obligation id
- Location: src/agent/omo_backend.rs:727
- Trigger: The thread is busy with a background run when the user's prompt is submitted. That run's completed agent messages are relayed before the ack (line 1534: `session, stream_id, sequence, &scrubbed, None,`), and then this turn's final reply is delivered with the same `stream_id`.
- Impact: Every delivery uses `obl:turn:{stream_id}`. `record_obligation` upserts with `ON CONFLICT(id) DO UPDATE SET content = excluded.content, state = 'pending'` (ledger/service.rs:391), so the final reply overwrites the obligation row of the message already delivered. If the final dispatch then fails, the shared obligation becomes `failed`. `rollback_partial_history` (multiplexer/actor.rs:1036-1047) then deletes every assistant row linked to it, including the pre-ack message the user already saw, and any recovery resends only the last content.
- Evidence: `let obl_id = format!("obl:turn:{}", stream_id);` is the only id used, whatever the number of deliveries per turn.
- Fix: Give each delivery its own id, for example `obl:turn:{stream_id}:{sequence}`, or a fresh UUID for pre-ack relays.

### [P2] Waiting for the thread/settings/update ack is bounded only by the whole turn deadline
- Location: src/agent/omo_backend.rs:840
- Trigger: The app-server accepts the socket but never answers request id 4, for example while busy, or when it drops or ignores the request on a resumed thread.
- Impact: The turn waits for up to the full turn deadline (30 min for interactive turns, longer for lanes) with no request-level timeout. The user gets no typing indicator or feedback during that time, because Typing is dispatched only after turn/start. This differs from initialize, resume and start, which each use `deadline.min(now + request_timeout)`.
- Evidence: `let frame = tokio::time::timeout_at(deadline, ws.next())` followed by `.map_err(|_| OmonError::Llm("model update timed out".into()))?`.
- Fix: Use `deadline.min(tokio::time::Instant::now() + self.config.request_timeout)` per read, as the other setup RPCs do.

Nothing was built or run, per the read-only constraint. Every cited path:line was re-opened and its quoted code confirmed. src/agent/omo_protocol.rs and src/agent/omo_activity.rs had no findings: truncation is char-based, so it is safe on non-ASCII text, and the dead omo_activity module was already recorded in an earlier review.
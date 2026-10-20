# Lane agent-backend

### [P1] A transport-reset retry re-runs the whole prompt even after tools have already run
- Location: src/agent/omo_backend.rs:2516
- Trigger: An interactive Slack turn has already run tools. It may also have sent interim status messages, for example after a `git push`, a file edit or a `relay_status` message. During that turn, the owned app-server gets force-restarted. Two paths can do this: another session's stall path (`force_restart_owned_daemon`) or the liveness supervisor (omo_daemon.rs:725-726 stamps `LAST_FORCED_RESTART`). The current turn's socket then fails with `ws streaming error: ...`. `is_transport_reset` matches that error and `forced_restart_since(turn_started)` is true, so the same `event` is sent through `execute_turn` again on a fresh setup.
- Impact: The agent repeats side effects that already happened, such as a second push or edit, or a second outbound action. The user also gets the interim status messages and the reply a second time. The stall-retry path just above avoids this: it only retries when no tool ran ("server went silent after tool calls" is deliberately not a stall). The comment on this path says "duplicate execution cannot occur", but that is only true for the killed daemon process. It is not true for effects that process already completed.
- Evidence:
  ```
  if !retried
      && result.as_ref().is_err_and(is_transport_reset)
      && crate::agent::forced_restart_since(&self.config.appserver_url, turn_started)
  ...
  // Resubmitting here is safe because the daemon instance that held the turn was killed,
  // so duplicate execution cannot occur.
  ```
  and in omo_daemon.rs:862 vs 869 the restart stamp is written before the ownership and child checks, so `forced_restart_since` can be true even though nothing was killed:
  ```
  last.insert(url.to_owned(), std::time::Instant::now());
  ...
  tracing::warn!(%url, "omo app-server is not owned by this gateway; cannot restart it");
  return false;
  ```
- Fix: Make `execute_turn` report whether any tool ran or any message was delivered, the same way it sets `fallback_eligible`. For example, set an out-flag when `total_tool_calls == 0 && messages_delivered == 0`, and retry only when that flag is set. Also move the `LAST_FORCED_RESTART` insert in `force_restart_owned_daemon` to after the kill succeeds, or record "actually killed" separately.

### [P1] The same engine-turn reply is sent twice when the user writes while an engine-started turn is running
- Location: src/agent/omo_backend/follow.rs:185
- Trigger: After an interactive turn, the follower is subscribed. A background cell or monitor then starts an engine turn, so the follower sets `current = turn`. While that turn is running, the owner sends a Slack message. `run()` calls `stop_following`. Because `current` is set, the follower only sets `stopping = true` and keeps reading until that turn completes. Meanwhile the new gateway turn has subscribed and is waiting for its turn/start ack, because the thread is busy with the engine run. In that pre-ack window, `execute_turn` relays every completed `agentMessage` of the engine run, including the last one (omo_backend.rs:1922-1923). When the engine run finishes, the follower's `followed` check is true and it delivers `final_agent_text` again (follow.rs:214).
- Impact: The final message of the background turn shows up twice in Slack, and both copies are written to the transcript. The `persist_message` dedup does not catch it because the obligation ids differ: `stream_id` on one side, `Uuid::new_v4()` on the other.
- Evidence:
  ```
  // follow.rs
  _ = token.cancelled(), if !stopping => {
      if current.is_none() { return Ok(()); }
      stopping = true;
  ...
  let followed = current.is_some() && turn == current;
  ...
  self.deliver_rendered_result(session, Uuid::new_v4(), 0, &rendered, None)
  ```
  ```
  // omo_backend.rs:1919-1923 (pre-ack path)
  if let Some(text) = completed_agent_message_text(&method, &val) {
      ...
      .deliver_rendered_result(
          session, stream_id, sequence, &scrubbed, None,
  ```
- Fix: Give only one side ownership of the delivery. Option A: in the follower, skip delivery when `stopping` (or `gateway_turn_running`) is true, because the pre-ack relay of the gateway turn already covers it. Option B: have `stop_following` hand over the in-flight turn id so `execute_turn`'s pre-ack relay skips it. A is the smaller change.

### [P2] The follower may re-deliver the gateway's own just-finished reply as a "fresh replay"
- Location: src/agent/omo_backend/follow.rs:187
- Trigger: The follower subscribes right after the gateway turn ends. If `thread/resume` replays the last turn's `item/completed` or `turn/completed`, two things happen. `item/completed` with `current.is_none() && !gateway_turn_running` adopts the gateway turn's id (line 176-177). And `fresh_replay` accepts any completion within 30s of `started`, which always includes the gateway turn that just ended. `last_delivered` starts as `None`, so nothing stops it.
- Impact: Every interactive reply could be posted a second time. Whether this happens depends on the app-server's replay behavior, which I could not confirm from this repo, so I graded it P2.
- Evidence: `let fresh_replay = current.is_none() && !gateway_turn_running && val.pointer("/params/turn/completedAt")...is_some_and(|at| at + REPLAY_TOLERANCE_SECS >= started);`
- Fix: Pass the gateway turn's final `turn_id` (or its final text) into `follow_thread`, and skip that id in both the `item/completed` adoption and the `turn/completed` handling.

### [P2] An owner approval card can outlive its turn, and a late answer is dropped without notice
- Location: src/agent/omo_backend.rs:1731
- Trigger: With the ledger enabled, the Ask path spawns `request_for_proposal`. The turn can then end without that request being answered: the user cancels, a new message preempts it, or the 30-minute work deadline fires (the deadline is only extended once the ask resolves). The spawned task keeps the card live. When the owner answers later, `finalize_ask_result` records the decision, but `tx.send` fails because `approval_rx` is gone, and the result is discarded with `let _`.
- Impact: The owner approves an action that never runs, and the agent is never told.
- Evidence: `tokio::spawn(async move { let res = requester.request_for_proposal(...) ... let _ = tx.send((req_id_owned, tool_id, ask_started, resolution)); });`
- Fix: Keep the `JoinHandle`s and abort them, expiring the proposal, on every return path of `execute_turn`. Alternatively, check `tx.is_closed()` and expire the proposal instead of finalizing it.

### [P2] Workspace preset provisioning reads the env var instead of the backend's ledger override
- Location: src/agent/omo_backend.rs:458
- Trigger: A backend built with `with_ledger_enabled(true)` while `OMON_APPROVAL_LEDGER_ENABLED` is unset (tests or an embedded runtime). In that case `permissionPreset: auto` is never provisioned, so the engine may never send approval frames for the ledger to handle. The same pattern appears at line 647.
- Impact: The ledger path silently gets no frames in that configuration.
- Evidence: `crate::agent::agent_workspace::approval_ledger_enabled(),` passed to `ensure_workspace_permission_preset`, instead of `self.is_ledger_enabled()`.
- Fix: Pass `self.is_ledger_enabled()` at both call sites.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - transport-reset retry (2516), Ask spawn (1731), pre-ack relay (1922), preset provisioning (458, 647)
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend/follow.rs - follower delivery and replay logic (176, 185-214)
- /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs - LAST_FORCED_RESTART stamped before the ownership check (862 vs 869); forced_restart_since (914)
</files>
<answer>There are two P1 findings. First, the transport-reset retry re-runs a turn whose tools already ran, which repeats side effects and messages. Second, an engine-started turn's final reply is sent twice when the user messages while that turn is running. There are also three P2 findings. Every claim comes from reading the current tree; nothing was compiled or run.</answer>
<next_steps>For the retry, apply the "no tools ran and nothing was delivered" condition. For the duplicate, make the follower skip delivery while stopping. Both fixes are small and local.</next_steps>
</results>
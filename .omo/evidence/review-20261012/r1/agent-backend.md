# agent-backend review

### [P1] Progress messages sent mid-turn are counted as delivered but Discord throws them away, so some turns end with no reply
- Location: src/agent/omo_backend.rs:1464
- Trigger: In an interactive Discord turn, the agent finishes a status message (`item/completed` with an `agentMessage`), then a tool `item/started` arrives. That reaches the block at lines 1452-1466, which calls `emit_chunk(..., is_final=false)`, sets `messages_delivered += 1`, then runs `full_content.clear()`. The same happens on the `agentMessage` path at lines 1515-1524. The turn then finishes (`turn/completed` with status `completed`) without any closing agent text.
- Impact: `DiscordEgress::stream` drops every chunk that is not final. In `src/discord/adapter.rs:3026`, `completed_stream_content` returns `None`, so the adapter only calls `keep_typing` and returns. The user never sees the status text, and the buffer holding it has already been cleared. At line 1638 `scrubbed_content` is empty and `messages_delivered > 0`, so the backend sends neither the text nor the "✅ Done." placeholder. The turn returns `Ok(())` and the user gets nothing. The progress watchdog that nudges the agent to post a status update (line 1140) and the 45-second "no output yet" check also turn themselves off on a delivery that never happened.
- Fix: Only count a message as delivered if it went out as a final chunk. Either send those mid-turn messages through `deliver_rendered_result` / `is_final=true`, or leave `messages_delivered` unchanged and keep the text in `full_content` instead of clearing it.

### [P2] `/reset` does not clear the in-memory thread cache, so the next turn resumes the old remote conversation
- Location: src/agent/omo_backend.rs:280
- Trigger: A user runs Discord `/reset confirm:true` (`execute_reset_command` → `reset_session` sets `state_json='{}'` → `multiplexer.reset` → actor `Reset` removes `metadata.omo_thread_id`), then sends another message. In `resolve_thread_id`, `metadata.omo_thread_id` is now missing, so `.or_else(|| self.thread_ids.lock().get(&storage_key).cloned())` returns the pre-reset id. Nothing outside `omo_backend.rs` ever clears `thread_ids`.
- Impact: The gateway sends `thread/resume` for the old thread, and `persist_session_binding` writes it back to the database. The conversation the user was told was cleared comes back until the gateway restarts. The reset itself does not fail.
- Fix: Drop the cache fallback and treat session metadata as the only source of truth. Alternatively, add a backend hook that removes `thread_ids[storage_key]` on reset and call it from the reset path.

### [P2] `active_turns` entries are left behind on several error paths that return early
- Location: src/agent/omo_backend.rs:1123
- Trigger: `execute_turn` returns while the session's `active_turns` entry is still in place in these cases:
  - inactivity timeout (line 1123, "timeout during turn streaming: no activity from server")
  - approval-denial limit (line 1343)
  - a `turn/error` frame or an `error` frame carrying a turn id (line 1685)
  - a `?` failure from websocket, JSON, `deliver_rendered_result` or `capture_lane_item`
  - the run future being dropped by `run_with_cancellation` before the start ACK arrives (entry left with `turn_id: None`)
- Impact: The stale entry outlives the turn, and `active_turn_entry_is_stale` is dead code, so nothing reclaims it. Later `cancel()` calls (judge, kibitzer, actor `interrupt_turn`) send `turn/interrupt` for a dead turn and report an error, or fail with "cancellation unresolved" when `turn_id` is `None`. Each affected session keeps one stale entry until its next turn replaces it.
- Fix: Remove the `storage_key` entry on these error returns, matching what the other terminal paths already do. A simple way is a scope guard that removes the entry unless ownership is deliberately kept, as on the connection-closed path.

### [P2] A rejected model update never triggers the fallback model
- Location: src/agent/omo_backend.rs:758
- Trigger: The session's `active_model` (for example set with `/model <invalid>` through `execute_model_command` → `set_model`) is rejected by `thread/settings/update`. Line 758 returns `Err("model update rejected: …")` without setting `*fallback_eligible = true`. `is_model_rejected_error` explicitly matches "model update rejected" and "thread/settings/update", but it is only ever checked on the `turn/start` and turn-failed paths.
- Impact: The fallback retry in `run()` never fires for this error, so every turn in the session fails until someone changes the model by hand.
- Fix: On this error path, set `*fallback_eligible = is_model_rejected_error(&error.to_string())` before returning.

### [P2] Cron output on the deadline-drain path skips `filter_intent_gate`, and a bad `settings.json` gets overwritten
- Location: src/agent/omo_backend.rs:1033
- Trigger: A cron turn reaches its work deadline, the drain sees `turn/completed` with status `completed`, and the code at lines 1023-1046 runs. `scrubbed_content` is filtered with `filter_intent_gate` only for the silence check. Line 1033 then shadows it with `let scrubbed_content = filter_reasoning(&full_content);`, so the result written to `cron_agent_output` still contains the intent-gate text that the normal `turn/completed` path (line 1606) strips.
- Impact: The cron executor delivers output that still contains the intent-gate line.
- Fix: Remove the shadowing `let` on line 1033 and reuse the already-filtered `scrubbed_content`.
- Related, same function family: at line 457 (`.unwrap_or_default()`), a cron run in `per_agent_workspace` mode that hits any read or parse error on `.omo/settings.json` (for example comments or trailing commas) replaces the file with only `disabledBuiltinExtensions`. Line 470 (`let _ = tokio::fs::write(&settings_path, …)`) also discards write failures silently. Fix: skip the rewrite when an existing file cannot be read or parsed, and log the write error instead of discarding it.

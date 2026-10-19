/END block with concrete triggers and absolute paths.
</analysis>

REPORT-BEGIN
# agent-backend review
### [P1] Progress steer defeats the inactivity and total-deadline watchdogs
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:1164 (steer send), :1186-1193 (activity and deadline renewal)
- Trigger: An interactive Discord turn whose agent has started a tool or reasoning but has not completed any agentMessage, for example a hung command or a stalled provider. `messages_delivered == 0` stays true, so the gateway sends `turn/steer` every 12s (line 1164). The server's ack for each steer (ids 9100+) is a Text frame.
- Impact: Every Text frame, including those acks, sets `last_activity_at` and renews `deadline` and `work_deadline` (line 1192), before any filtering by method or id. The 180s inactivity timeout and the 30m total cap therefore never fire. `initial_response_timeout` is disarmed once `first_progress_at` is set. The turn and the session actor stay blocked until the user cancels, and the steer notes pile up in the thread history.
- Fix: Renew `last_activity_at` and the deadline only for frames that show real work: turn-bearing frames for this thread and turn that pass `is_agent_progress`. Skip responses to gateway-issued ids (>= 9000), thread-status frames and frames for other turns. Alternatively give steer acks a separate cap.

### [P1] Mid-turn "delivered" chunks are non-final, and the Discord adapter drops non-final chunks
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:1483-1496 (tool-start branch, emits `is_final=false`, increments `messages_delivered`, then clears `full_content`); :1548-1558 (first intermediate agentMessage); :1577 (deltas). Adapter: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3054-3057 and :3145-3147.
- Trigger: A Discord turn where the agent sends a status or progress message (including the reply to the gateway's own steer) and then keeps running tools. `completed_stream_content` returns `None` for every non-final chunk, so the adapter only refreshes typing and shows nothing.
- Impact: The backend counts the message as delivered, so the progress steer and the initial-response guard stop. The user sees silence until the final answer, which defeats the purpose of the code. In the tool-start branch the text is also cleared from `full_content`. If the turn then ends with no further text, `has_content` is true via `messages_delivered`, no "Done." is sent, and the turn returns Ok having shown the user nothing.
- Fix: Count `messages_delivered` only when a final-style delivery succeeds, for example via `deliver_rendered_result`/SendMessage as the pre-ack relay does. Or teach the adapter to render intermediate chunks. Do not clear `full_content` unless the text was actually shown. Stop emitting a non-final chunk per delta (the full buffer is cloned and dispatched each time).

### [P2] Settings-update rejection never reaches the model fallback
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:777 (returns Err directly); :40-52 (classifier lists "model update rejected" and "thread/settings/update"); :1241 and :1622 (the only places `fallback_eligible` is set).
- Trigger: `active_model` is an unknown model and the app-server rejects `thread/settings/update`. The classifier patterns suggest this is expected to happen.
- Impact: `fallback_eligible` stays false, so the turn fails and the fallback model is never tried. Every later turn fails the same way until `/model` is changed. Whether this server rejects at settings/update rather than turn/start is not verified here, which is why this is P2.
- Fix: At line 777, run `is_model_rejected_error` on the error and set `*fallback_eligible = true` before returning, as the turn/start path does.

### [P2] Cron briefings are consumed before the turn is sent
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:727 (`take_pending_cron_briefings` marks the rows injected before turn/start); fallback re-run at :1885-1895.
- Trigger: Pending cron briefings exist, then the turn fails before the prompt is accepted. Examples: the model is rejected and the fallback runs `execute_turn` again, or the ack times out, or `turn/start` fails to send.
- Impact: The first attempt marks the briefings `mirror_injected` in a transaction. The fallback or retry finds nothing, so the agent never sees the cron context and cannot answer follow-ups about it.
- Fix: Take the briefings once in `run()` and pass the rendered text into both attempts. Or mark them consumed only after the turn/start ack.

### [P2] Deadline drain loop aborts on any non-Text frame
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:953 (`while let Some(Ok(Message::Text(text)))`)
- Trigger: A Ping, Pong or Binary frame arrives while draining after `turn/interrupt` at the work deadline.
- Impact: The pattern fails to match, the drain ends with None, and the code skips the confirmation and completed-turn handling. It goes straight to `clear_stuck_thread` and a deadline error, dropping the persisted thread binding and any completed output. The main loop handles non-Text frames with `_ => None`, so the two loops are inconsistent.
- Fix: Loop on `ws.next()` and `continue` on non-Text frames; stop only on None or Err.

### [P2] Approvals default to allow-all; the yolo flag and toolset gate have gaps
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:1335 (`auto_approve` unless `APPROVAL_MODE == always`) and the toolset check just above it; /Users/indo/code/project/omon-gateway/src/agent/omo_protocol.rs:151.
- Trigger: `APPROVAL_MODE` is unset, which is the default.
- Impact: Every approval request is accepted whether or not `session.state.yolo` is set, so `/yolo off` has no effect. The `enabled_toolsets` restriction only blocks command-like requests (`is_command` check). File-change and other approval kinds are auto-approved even when the toolset excludes them. The comment documents the single-user assumption, but in a multi-user guild channel any prompt is effectively auto-approved.
- Fix: Make the default "ask" (or require yolo for auto-approve), and extend the toolset gate to file-change and network approvals.

### [P2] `steer()` reports "not delivered" when the outcome is unknown
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:1990-2040 (`Ok(None)` and timeout return `Ok(false)`).
- Trigger: The app-server applies the steer but the ack is lost: the socket closes, or the 5s window passes under load.
- Impact: The caller falls back to a fresh turn with the same user message. The peer does not deduplicate, so the message runs twice (once as the steer, once as a new turn).
- Fix: Return an error or a distinct "ambiguous" result for timeout and close, and have the caller decide. Only a definite rejection should map to Ok(false).

### [P2] Cron `settings.json` is rewritten with parse failures treated as empty
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:449-470
- Trigger: A cron session's `.omo/settings.json` is not strict JSON (comments, a trailing comma), is unreadable, or is concurrently modified.
- Impact: `.ok().and_then(parse).unwrap_or_default()` yields an empty map and the file is overwritten with only `disabledBuiltinExtensions`, losing the user's settings. The write is not atomic and its error is ignored. It also runs on every cron run.
- Fix: On a read or parse error leave the file untouched. Write only when the entry is missing, and write via temp file plus rename.

### [P2] Dead code: `omo_activity` module and the stale-turn helper
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_activity.rs:6 (`#![allow(dead_code)]`); /Users/indo/code/project/omon-gateway/src/agent/mod.rs:4; /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:38-39 and the stale-margin constant and `active_turn_entry_is_stale` (both marked `#[allow(dead_code)]`).
- Trigger: A grep for `format_activity_line`, `reasoning_blockquote` and `command_output_excerpt` finds no caller outside the module. `active_turn_entry_is_stale` is used only by its own tests.
- Impact: Nothing runs this code, so the described activity lines and stale-entry reclaim do not exist. `execute_turn` replaces an occupied entry unconditionally instead. If wired up later, `format_activity_line` puts raw command, tool and type strings into Discord markdown and backticks without escaping.
- Fix: Wire the module in with markdown/mention escaping, or delete it and the stale helper.

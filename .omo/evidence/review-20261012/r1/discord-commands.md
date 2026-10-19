# discord-commands review

### [P1] /yolo and /stop write yolo state straight to SQLite, but the live session actor keeps a stale copy and overwrites it
- Location: src/discord/commands.rs:1272 (`state.yolo = new_yolo;`). The same pattern is at src/discord/commands.rs:820 (`state.yolo = false;`) in `stop_session`.
- Trigger:
  - `/yolo on` in a session whose actor is already running calls `yolo_toggle`. It updates `sessions.state_json` and `approvals.set_yolo`. It never tells the actor, unlike `set_model`, which calls `handle.set_model`.
  - The actor holds its own `context.state`. `src/multiplexer/actor.rs` clones it as `turn_context` for each turn, and `flush()` serialises the whole `context.state` over `state_json`.
  - The next message turn in that session therefore rewrites `state_json` with `yolo=false`.
  - `src/agent/omo_backend.rs:1300` checks `session.state.yolo`, which is the actor copy. It never sees the toggle.
  - `grep yolo src/multiplexer` has no matches, so nothing syncs the actor.
- Impact:
  - `/yolo on` is lost from the DB after the next turn, so `load_persisted_yolo` does not restore it after a restart.
  - The daemon-approval path ignores it. Under `APPROVAL_MODE=always` it is not honoured.
  - The reverse case is a safety regression. If the actor loaded `yolo=true`, then `/yolo off` or `/stop` clears the DB and the in-memory guard. The next turn flush rewrites `yolo=true` to the DB, and omo_backend keeps auto-approving.
- Fix: route yolo changes through the actor, for example a `SetYolo` actor command mirroring `set_model`. Alternatively make `flush()` preserve the DB's `yolo` field the way it preserves `omo_thread_id`.

### [P1] /retry deletes the last exchange before re-routing, so a routing failure loses it permanently
- Location: src/discord/commands.rs:1034 (`DELETE FROM messages WHERE session_key = ? AND sequence >= ?`). The re-route is at src/discord/commands.rs:1042.
- Trigger:
  - `/retry` runs the DELETE on the pool, committed and non-transactional with the route. It then calls `ctx.data().multiplexer.route(event).await?`.
  - `SessionMultiplexer::route` returns `Err` when `is_draining()` is true (src/multiplexer/router.rs:339-343) or when admission is closed.
  - A `/retry` issued during a deploy or shutdown drain hits this.
- Impact: the user message and the assistant reply are already deleted, and no turn was queued. The command returns an error and the last exchange is gone from history.
- Fix: check `multiplexer.is_draining()` before deleting. Alternatively capture the deleted rows and restore them in a transaction if `route` fails.

### [P1] /compress treats a failed or truncated LLM stream as a complete summary and deletes the original history
- Location: src/discord/commands.rs:1083 (`if let Ok(chunk) = chunk {`)
- Trigger:
  - `compress` drains `llm.stream(...)` and silently drops every `Err` chunk. A mid-stream error such as a timeout or connection reset leaves a non-empty partial `acc`, which is accepted as the summary.
  - If the initial `llm.stream` call fails, the code falls back to `fallback_summary`. That is only the first 80 characters of each message's first line.
  - Either way, `replace_history_with_summary` (src/discord/commands.rs:1506) then deletes every message at or below `cutoff_sequence` and inserts the degraded summary.
- Impact: irreversible loss of conversation history, replaced by a truncated or stub summary. The reply still says "Conversation Compressed".
- Fix: abort without touching the DB if any stream chunk errors or the stream ends in error. Only replace the history after a full successful summary. Warn the user and do not delete when a fallback was used.

### [P1] A gateway restart mid-action leaves the button message stuck on "처리 중" with no result and no retry
- Location: src/discord/buttons.rs:395 (`tokio::spawn(async move {`)
- Trigger:
  - A click calls `handle_button_click`. It claims `IN_FLIGHT`, edits the buttons to the disabled pending label at line 385, and runs the action in a spawned task. Only that task restores the buttons, posts the result and clears `IN_FLIGHT`.
  - `IN_FLIGHT` is an in-process static, and nothing is persisted. If the gateway restarts during the run (default timeout 600s, restarts are plausible with launchd or deploys), the task dies.
  - The line 190 comment says the child is meant to survive a restart (no `kill_on_drop`).
  - The child's stdout and stderr are piped to the dead parent. If it writes after the parent exits, it may get SIGPIPE or EPIPE, which could cut off the publish this design is meant to protect.
- Impact:
  - The message keeps the disabled "⏳ 처리 중…" button forever.
  - No success or failure reply is posted, and the retry path ("버튼을 다시 누르면 재시도합니다") is unreachable.
  - The outcome of a possibly half-run irreversible action is invisible to the user.
- Fix: persist in-flight button state, meaning message ID, channel ID, clicked ID and original components. On startup, restore or mark those messages as "interrupted – status unknown". Redirect the child's stdout and stderr to a file or `/dev/null` so its fate does not depend on the gateway pipe.

### [P2] Route failures are silently ignored after the user is told the action happened
- Location: src/discord/commands.rs:601 (`let _ = data.multiplexer.route(event).await;`)
- Trigger:
  - `/skill action:run` announces "Injecting skill ... into current OMO session..." and then discards the `route` result.
  - `/thread` with a first message does the same at src/discord/commands.rs:1197 (`let _ = ctx.data().multiplexer.route(event).await;`) and then replies "Created thread".
  - In both cases `route` fails if the gateway is draining or admission is closed (src/multiplexer/router.rs:339-343).
- Impact: the skill is never run, or the starter message is dropped, and the user is told it worked.
- Fix: propagate the error with `?`, or send an ephemeral failure message.

### [P2] Thread ownership is set in memory first, and the persist error is dropped in a detached task
- Location: src/discord/commands.rs:123 (`let _ = crate::Database::record_thread_owner(&pool, thread_id, bot_id).await;`)
- Trigger:
  - `mark_thread_owner` is called from `/thread` (src/discord/commands.rs:1178) and from adapter.rs:1139 and :1159.
  - It updates the maps, then spawns a fire-and-forget task for the DB write. A DB error is discarded. A restart before the task runs also loses the write.
- Impact: ownership exists only in memory and the DB row is missing. After a restart the thread owner is unknown, and message routing or mention handling for that thread changes. The adapter only re-marks ownership on an explicit mention.
- Fix: await the DB write before updating the in-memory maps, or at least log the error and retry.

### [P2] A /memory list DB error is shown as "No memories stored"
- Location: src/discord/commands.rs:644 (`.unwrap_or_default();`)
- Trigger: the `SELECT id, content FROM memories ...` query fails, for example on a locked or busy DB or a missing table. `.unwrap_or_default()` yields an empty Vec.
- Impact: the user is told there are no memories when the read actually failed. The error is not logged.
- Fix: propagate the error with `?`, or log it and tell the user the lookup failed.

### [P2] /skill search overflows the 2000-character Discord limit
- Location: src/discord/commands.rs:560 (`ctx.say(format!("🔍 **Skill Search Results for `{query_val}`** ({count} matches):\n- {list_str}")).await?;`)
- Trigger:
  - `/skills action:search` with an empty or very common query matches every skill, because `src/tools/skills.rs:170-180` uses `contains(&query)`.
  - The joined match list is sent as one unchunked `ctx.say`. The `list` branch caps at 30 entries and chunks, so `search` is the inconsistent one.
  - A long skill catalogue pushes the message past 2000 characters.
- Impact: Discord returns a 400 error and the command fails with an error instead of showing results.
- Fix: cap and chunk with `chunk_slash_reply`, the same as the list branch.

VERIFY: each Location line above was re-checked against its cited line.

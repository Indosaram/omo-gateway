# discord-adapter review

### [P1] Thread archive/delete handlers build a session key that never matches the real thread session, so active turns are never stopped
- Location: src/discord/adapter.rs:1395
- Trigger: A user archives or deletes a Discord thread while a turn is running in it. `handle_event` then calls `handle_thread_update` or `handle_thread_delete` (lines 1364/1367). Both build `SessionKey::new("discord", None, thread_id, None, "")` with no guild, channel_id set to the thread id, no thread_id and no bot_id. Inbound thread messages are keyed differently by `message_to_inbound_with_config` (lines 1975-1985): `guild_id=Some(guild)`, `channel_id=parent`, `thread_id=Some(thread)`, plus `.with_bot_id(bot)`. `storage_key()` length-prefixes every component and appends bot_id, so the keys never match.
- Impact: `data.multiplexer.stop(&session_key)` (lines 1403/1430) finds no actor. It calls `mark_session_suspended` on a key that doesn't exist and returns Ok(false); that result is discarded with `let _`. The real thread turn keeps running and keeps trying to post into an archived or deleted thread. No session is ever marked archived or unbound. The failure is silent.
- Fix: Rebuild the key the same way inbound does, using `new.guild_id`, `new.parent_id` and the thread id as thread_id, plus `.with_bot_id(ctx current user)`. Alternatively, look up sessions with `thread_id = ?` in the `sessions` table, stop each one, and log a stop error instead of discarding it.

### [P1] Thread archive/delete SQL writes a nonexistent `metadata` column; the error is swallowed
- Location: src/discord/adapter.rs:1405
- Trigger: The same `ThreadUpdate`/`ThreadDelete` events. The UPDATE statements at lines 1405, 1412 and 1432 reference `sessions.metadata`. The `sessions` table (migrations/0001_initial.sql) only has `state_json`; session metadata lives at `$.metadata.*` inside `state_json`, and no migration adds a `metadata` column.
- Impact: Every statement fails with "no such column: metadata", and `let _ =` discards the error. `thread_archived` is never recorded. On delete, `omo_thread_id` is never cleared, so the stale remote thread binding survives (it is read from `state_json` `$.metadata.omo_thread_id` in src/agent/omo_backend.rs). This defect is independent of the key mismatch above, so it stays broken even if the key is fixed.
- Fix: Use `UPDATE sessions SET state_json = json_set(COALESCE(NULLIF(state_json,''),'{}'), '$.metadata.thread_archived', json('true'|'false'))`, and for delete use `json_remove(state_json, '$.metadata.omo_thread_id')`. Log failures instead of discarding them.

### [P2] Text `/stop` is not recognized once channel topic or reply context is prepended
- Location: src/discord/adapter.rs:1198
- Trigger: `data.channel_topic_context` is enabled and a user types plain-text `/stop` in a guild channel or thread whose channel or forum parent has a topic. Line 1067 rewrites `event.content` to `"{topic_block}\n\n/stop"` before the check at line 1198. The same happens when the `/stop` is an inline reply, because `compose_reply_context` prefixes a quote block.
- Impact: The `/stop` check at line 1198, and again at line 1499, compares the whole trimmed content, so it fails. The stop message is steered into the running turn as guidance, or debounced and routed as a new prompt. The running turn is not interrupted, and the debounce buffer is not cancelled.
- Fix: Detect stop on the raw stripped user text, `strip_bot_mention(&new_message.content, bot_user_id)`, before any topic, context or reply prefixing. Carry it as a flag, or run the check before lines 1050-1100.

### [P2] A steered message is re-enqueued and processed a second time when the ledger write fails after a successful steer
- Location: src/discord/adapter.rs:1466
- Trigger: In `handle_event`, a text message is sent to `steer_claimed_event` while a turn is active. `multiplexer.steer` returns Ok(true), so the guidance has already been injected. Then `ledger.record_incoming_as(...).await?` hits a SQLite error, such as a busy or locked database. The `Err` goes back to the caller at lines 1214-1221, which logs it and calls `global_debouncer().enqueue(event, ...)`.
- Impact: No ledger row exists for the delivery, so the debounced path claims and routes the same message as a new turn. The user's message is processed twice: once as steer guidance and once as a separate turn, which can produce duplicate replies or actions.
- Fix: After a successful steer, don't propagate ledger errors as `Err`. Log them and return Ok(true). Alternatively, make the caller fall back to enqueue only when the steer itself failed.

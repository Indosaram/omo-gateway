# discord-commands review

### [P1] `/skill|/skills|/memory` pending/review output is posted publicly, exposing cross-session pending writes
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:491 (`skill_dispatch` calls `ctx.defer()`), :621 (`memory` calls `ctx.defer()`), :418 (`pending_command` builds the attachment)
- Trigger: With write approval enabled, an authorized operator runs `/skills action:pending` or `/skills action:review name_or_query:<id>` in a guild channel. The same applies to `/memory action:pending` or `review`.
- Impact:
  - The command is deferred non-ephemeral, so the reply is public. Poise follow-ups after `defer()` inherit that visibility.
  - `pending_command` attaches the full JSON payloads as `pending-writes.json`.
  - `PendingScope::Skills` is global, not per-session. Staged skill payloads from any session, including other users' DMs, are therefore uploaded to whoever can read the channel.
  - The channel may include members who are not in the allow-list.
  - The same applies to full pending memory payloads for the channel session.
  - Errors from `?` after the defer are also shown publicly.
- Fix: Use `ctx.defer_ephemeral()` for the pending, review, approve and reject actions, deciding visibility before deferring. Alternatively, keep the generic defer and send pending replies only via an ephemeral follow-up path.

### [P2] `/retry` deletes and restores history non-transactionally, and the restore drops `platform_message_id`
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:1475 (`retry_last_user_message`, SELECT, DELETE and restore loop near :1500-1540)
- Trigger:
  - (a) The actor persists a new message between the snapshot SELECT and the DELETE. The new row has `sequence >= user_seq`, is deleted, and is not in the snapshot, so a route-failure rollback cannot restore it.
  - (b) `route()` fails after the delete. The re-insert column list omits `platform_message_id`, so the restored user message loses the id used by `has_platform_message_id` dedup.
- Impact: Lost transcript rows. Restored messages are no longer recognized as already processed, so a redelivered or backfilled Discord message can run a duplicate turn.
- Fix: Run SELECT, DELETE and re-insert in one transaction, and include `platform_message_id` in the snapshot and restore. Better still, route first and delete only on success.

### [P2] `/reset` wipes the DB before interrupting the actor, and swallows the actor reset error
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:746-755
- Trigger: `/reset` while a turn is running. After `reset_session` commits, the user row is gone. The actor's `rollback_partial_history` (`sequence > (SELECT sequence ... WHERE id = event_id)`) then evaluates against NULL and deletes nothing. Assistant or tool rows written between the wipe and the cancel survive.
- Related trigger: `multiplexer.reset` returns Ok without acting for a Retiring or Closed actor, and `let _ =` ignores any error. The user is told "cleared" while a live actor's stale state (yolo, `omo_thread_id` binding) can be flushed back.
- Impact: Post-reset context or binding resurrects despite the confirmation message.
- Fix: Reset or cancel the actor first and require success, then wipe the DB. Propagate the multiplexer error.

### [P2] Interaction id used as `platform_message_id` for synthetic events
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:593-600 (`/skill run`), `/steer`, `/retry` (`ctx.id().to_string()`), and `/thread` starter (`InboundEvent::message(thread_key, ctx.id()...)`)
- Trigger: Any of these commands routes an event whose `platform_message_id` is the interaction snowflake.
- Impact: Replies are sent with `reply_to` = that id. The adapter's dispatch retries without the reference on failure, but each reply costs a failed API call. The processing-reaction path (`OutboundAction::React`) targets a non-existent message. Bogus ids also land in `messages.platform_message_id` and the search index. I did not verify the Stream path.
- Fix: Give these events an empty `platform_message_id`, or a distinct delivery id.

### [P2] Slow slash commands are not deferred (3s interaction deadline)
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs `stop` (:841), `reset` (:789), `retry` (:1016), `steer` (:956), `yolo`, `model`, `thread`
- Trigger: `multiplexer.stop` waits for the runner's cancel cleanup, and `reset` does a DB wipe plus an actor round trip. `/retry` and `/steer` call `route`, which can block on a busy or retiring actor. Each also calls `to_channel` via `session_key`. None of them call `defer()`.
- Impact: The interaction expires and the final `ctx.say`/`ctx.send` fails with Unknown interaction, even though the state change happened.
- Fix: `ctx.defer_ephemeral()` first in the commands that touch the actor or DB.

### [P2] `/compress` silently replaces history with a lossy fallback when `llm` is None, and builds an unbounded prompt
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:1048-1085 (`fallback_summary` branch at :1082), :1420
- Trigger: `PoiseData.llm` is None. `main.rs` sets it with `LlmClient::new(...).ok()`, so a construction failure becomes None silently. `/compress` then permanently replaces the whole history with first-line snippets labeled as a conversation summary.
- Related trigger: With a very long history, `build_compression_prompt` concatenates the entire transcript with no size cap. The LLM call fails, which is safe but means compress never works on exactly the sessions that need it.
- Impact: Irreversible loss of conversation detail, with an "achieved X% reduction" message.
- Fix: Refuse to compress when `llm` is None. Cap or window the transcript sent to the model.

### [P2] Skill approve/reject has global scope; any admitted user can approve any session's staged skill
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:486-505 (`PendingScope::Skills`), with `PendingWriteScope::permits` in /Users/indo/code/project/omon-gateway/src/storage/db.rs:640
- Trigger: A paired or role-authorized user (not an operator) runs `/skills action:approve name_or_query:<id>`.
- Impact: The skill file is written and later executed by agents. There is no per-session ownership check and no audit of who approved.
- Fix: Restrict approve/reject to `allowed_users` (not paired users or roles). Alternatively scope skill pending writes to the originating session.

### [P2] Button restart and restore handling
- Location: /Users/indo/code/project/omon-gateway/src/discord/buttons.rs:362-437 (spawned task), :259-284 (`rebuild_buttons`)
- Trigger and impact:
  - (a) A gateway restart or shutdown while an action runs kills the spawned task. The child keeps running because there is no `kill_on_drop`, but nothing edits the message back. The button stays disabled as "⏳ 처리 중…" forever, and the result is never reported. Also, the child's stdout and stderr are piped to the dying gateway, so its next write can hit SIGPIPE or EPIPE, contradicting the "must not cut off" intent.
  - (b) On failure, or a nonterminal success, `rebuild_buttons` enables every non-link button via `.disabled(false)`. Buttons that were disabled in the original message are re-enabled.
  - (c) If the final `set_buttons` edit fails (rate limit or transient error) after a successful terminal action, the button stays enabled or pending, and a click re-runs an "irreversible" action. The pending-state edit failure is likewise ignored.
  - (d) `wait_with_output` buffers unbounded stdout and stderr. A grandchild that inherits the pipes keeps `wait_with_output` blocked until the timeout even though the main process exited, and the result is then reported as a timeout failure.
- Fix:
  - Persist in-flight button state, or reconcile on startup.
  - Restore the original per-button `disabled` flags from `original`.
  - Retry or verify the final edit.
  - Cap captured output, and redirect child output to a file or use bounded readers.

### [P2] `/memory` and `/skill` `pending` list is all-or-nothing on one malformed payload
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:418-430 via `list_pending_writes_scoped` (/Users/indo/code/project/omon-gateway/src/storage/db.rs:668-677)
- Trigger: One pending memory row has a payload that is not JSON or has no `session_key`. `permits()` returns Err and the `?` in the loop aborts the whole listing.
- Impact: The operator cannot list any pending writes for that kind. Review and reject by id still work.
- Fix: Skip and log unparsable rows instead of failing the list.

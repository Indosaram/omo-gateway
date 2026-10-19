# multiplexer review

### [P1] Retried or previously failed inbound messages are skipped as "already replayed" and reported as success
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:313 (the `has_platform_message_id` dedup branch of `ActorCommand::Event`). It interacts with `persist_inbound`, which inserts the transcript row before the turn runs and before `index_inbound`.
- Trigger: A Discord message turn fails. Examples are a backend timeout, an error in `Completed(Err)`, or `index_inbound` returning an error after the `messages` row was inserted in `persist_inbound`. The delivery is then marked failed. `record_incoming_as` deliberately lets a failed delivery be reclaimed, so startup backfill or a redelivery routes the same `platform_message_id` again. The actor finds the row from the first attempt and takes the "skipping replayed inbound turn" path. That path calls `complete_delivery(Ok)` and `resolve_pending_ack(Ok)` without running anything.
- Impact: The user's message is never processed. The delivery ledger flips from failed to delivered, and `route_awaiting_turn` returns Ok. The backfill cursor then advances past a message whose turn never succeeded, which is exactly what `route_awaiting_turn` is documented to prevent. The plain `route()` path has no cleanup of the orphan row. `route_claimed_event_awaiting_turn` does delete it, but only in its own error branch.
- Fix: Do not treat the transcript row alone as proof that the turn completed. Either dedupe on a completed-turn marker, such as an assistant row or a delivery-ledger `delivered` state for that platform id. Or delete or roll back the inbound row, and its index entry, whenever the turn aborts or fails. Run `persist_inbound` and the index write in one transaction.

### [P1] Live actor can silently drop `/reset`, `/yolo`, and `/model` during the GC retire window
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs:111-190 (`if outcome != SendOutcome::Sent { return Ok(()) }` in `set_model`, `set_yolo`, `reset`), router.rs:619-628, and /Users/indo/code/project/omon-gateway/src/multiplexer/gc.rs:85. Read for context: `/Users/indo/code/project/omon-gateway/src/discord/commands.rs:754-756` and `:1274-1276`.
- Trigger: Every GC tick calls `try_retire()` on every session, including busy ones. `accepting` stays false until the actor answers `EvictIfIdle`. That reply can take seconds while the actor is in a post-turn tail awaiting dispatcher calls or DB writes. During that window the user runs `/reset` or `/yolo off`. The command updates the DB first, then calls `multiplexer.reset()` or `set_yolo()`. Both return `Ok(())` on `Retiring`, and `router.reset` also ignores errors with `let _ =`. `GC` then calls `resume()` because the actor was not idle.
- Impact: The command reports success but the live actor keeps its old in-memory state. The next `flush()` overwrites the DB row from that stale state. A `/yolo off` is undone, which re-enables auto-approval of tool requests (security-relevant). A `/reset` brings back the `omo_thread_id` binding and model that the user was told were cleared. This defeats the intent of the `binding_cleared` and `preserve_binding` logic.
- Fix: On `SendOutcome::Retiring`, do what `route` and `stop` do: `wait_until_reusable()` and retry. On `Closed`, remove the handle and re-resolve it, since there is no live actor left to go stale. Do not return `Ok(())` for state-mutating commands that were not delivered.

### [P1] Graceful shutdown marks every resident session `resume_pending`, so sessions whose last turn failed or produced no reply are re-run on restart
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs:656 (`mark_in_flight_resume_pending`). The consumer is `find_last_unfinished_user_turn` plus the restart recovery in `/Users/indo/code/project/omon-gateway/src/main.rs:464-520`. Both were read.
- Trigger: Any session with an actor still resident (idle timeout is 10 minutes) whose last turn failed or ended silently. The `Completed(Err)` path leaves the user row as the last transcript row. The error text is sent through the dispatcher and is not persisted. The same holds for turns that end in an explicit silence. Ctrl+C, SIGTERM, or a drain request calls `mark_in_flight_resume_pending`, which iterates all of `self.sessions`, not only sessions with an active turn. On boot, recovery sees "last row is a user message" and re-dispatches the old prompt.
- Impact: Work the user already saw fail, or already received, is re-executed on every graceful restart while the actor is resident. Side effects such as tool runs and posts are repeated. The restart-loop guard only trips after repeated boots.
- Fix: Mark only sessions that have an active turn or queued events. The actor already marks those itself in its shutdown path. Alternatively, track a "turn in progress" flag in the handle, or persist a terminal marker for failed or silent turns so they are not recovered.

### [P2] Mid-turn steered user messages are deleted by interrupt rollback
- Location: `rollback_partial_history` and `interrupt_turn` in /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs (about lines 880-930, not re-read after the restart). The writer is `record_inbound_transcript` at actor.rs:1254, called from /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1495.
- Trigger: A user steers a running turn, which inserts a user transcript row with a higher `sequence`. They then run `/stop` or `/reset`, or the service shuts down. The rollback `DELETE ... sequence > <turn user message>` removes every later row that has no delivered or attempting obligation. That includes the steered user row.
- Impact: The steered guidance disappears from history. After a shutdown, recovery re-runs only the original prompt.
- Fix: Restrict the delete to `role = 'assistant'` rows, or to rows created by this turn.

### [P2] A GC flush error aborts the whole GC pass
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/gc.rs:85-88
- Trigger: One session's `flush_if_dirty` fails persistently, for example a corrupt `state_json`, in the `EvictIfIdle` handler. `collect` calls `resume()` and then `return Err(error)`.
- Impact: Sessions later in DashMap iteration order are never evicted on any tick. Memory and actor tasks leak until restart.
- Fix: Log the error and `continue` to the next session. Return an aggregate error at the end if needed.

### [P2] `is_user_allowed_for_bot_or_route` fails open for guild-scoped routes (not called in production)
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/profile_routing.rs:228-247
- Trigger: It calls `match_route(None, channel_id, None)` with no guild id. `matches()` requires `guild_id == Some(req_guild)`, so any route that sets `guild` never matches. Its `allowed_users` restriction is skipped and the global allowlist applies. In `src`, the only call is the definition; its other uses are in `tests/test_profile_routing.rs`.
- Impact: If it is wired into ingress later, a per-route allowlist on guild-scoped routes is silently ignored.
- Fix: Take guild and thread ids as parameters and use `match_route_with_parent`.

### [P2] Profile-route config parsing silently widens or drops restrictions
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/profile_routing.rs:23-28 and :401-402
- Trigger: A negative number in `guild`, `channel`, or `thread` deserializes to `None`, which means "match any". One malformed entry makes `parse_profile_routes` return an empty list, so all routes, including `enabled_toolsets` and `allowed_users`, are dropped with only a warning.
- Impact: Misconfiguration removes toolset restrictions, or applies a route globally, instead of failing at startup.
- Fix: Reject negative values with a deserialization error. Make a parse failure a startup error, or fall back per entry.

### [P2] `SessionActor::recover_resume_pending_sessions` can lose recovery and pick the wrong delivery id (not called in production)
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:137-235
- Trigger: In `src`, the only reference to this function is its own definition. Its one external call is in `tests/test_multiplexer.rs:828`; `main.rs` uses its own `recover_resume_pending_sessions_with_auth` instead. It clears `resume_pending` before `route`, and on a route error it only logs, so the marker is not restored. The ledger lookups use `.unwrap_or(None)`, which hides DB errors. When `platform_message_id` is `None`, it attaches the latest ledger row for the session, which may belong to a different message. It also has no authorization check.
- Impact: Misleading if used as a library API. `main.rs` restores the marker on a route error; this function does not.
- Fix: Delete it, or make it delegate to the `main.rs` logic.

### [P2] `set_model` can wipe a corrupt or unparsable stored state
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs:590-611
- Trigger: `state_json` fails to parse, so `.ok()` and `unwrap_or_default()` turn it into a default state. The function then writes only `active_model` back over the whole column. The read-modify-write is also not atomic with respect to the actor flush.
- Impact: Stored metadata, including `omo_thread_id`, `yolo`, and the system prompt, is overwritten.
- Fix: Return an error on a parse failure. Use `json_set(state_json, '$.active_model', ?)` in a single UPDATE.

### [P2] Queued events are dropped on shutdown (unverified: depends on backfill)
- Location: /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:421-453 and the shutdown drain after the main loop.
- Trigger: Events queued behind a running turn are not persisted. `persist_inbound` only runs when an event is dequeued. At shutdown their deliveries are marked failed, and the code marks the session `resume_pending`, but recovery only reconstructs the last transcript user row.
- Impact: These messages survive only if Discord backfill re-claims the failed deliveries. I did not verify that the backfill cursor covers them.
- Fix: Persist the inbound row at enqueue time, or confirm that backfill covers this case.

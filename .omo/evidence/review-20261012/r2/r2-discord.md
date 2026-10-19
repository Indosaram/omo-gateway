# r2-discord review
### [P2] /retry restore drops platform_message_id and is not atomic with the delete
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:1497-1530
- Trigger: `/retry` where `multiplexer.route` fails after the DELETE (actor closed, admission closed). The restore INSERT writes only `sequence, id, session_key, role, content, metadata_json, created_at`. A concurrent actor insert landing between the SELECT at 1497 and the DELETE at 1503 is deleted but was never captured in `removed`.
- Impact: restored user rows have `platform_message_id = NULL`, so the transcript-level replay dedupe in the actor (actor.rs:308) no longer protects that Discord message. Rows inserted in the SELECT-to-DELETE window are lost permanently.
- Fix: select and restore `platform_message_id` too. Do the select, delete and restore inside one transaction, or delete by the exact `sequence` list that was selected.

### [P2] /retry can resend a "[system] ..." or mirrored row as a user message
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:1483-1486 (and 1555 for `/undo`)
- Trigger: the last `role='user'` row in the session is a system-origin event (the transcript writer prefixes these with "[system] ", actor.rs:1259-1265) or a `mirror_to_session` row written with role "user" (mirror.rs:35-47). Running `/retry` then sends that text back as a normal user-origin event.
- Impact: the agent receives the internal "[system] ..." text as if the user typed it, and the real last user message is deleted.
- Fix: filter on `metadata_json` not marking a mirror row, and skip content starting with "[system] ". Alternatively add an origin column.

### [P2] Instant-steer and coalesced messages keep the start reaction forever
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1100-1125 (start reaction), 1206-1213 (steer branch)
- Trigger: with `processing_reactions` on (the default), a user sends a message while a turn is running. The start emoji is added at ingress. The actor only swaps it for the success or failure emoji on the turn's own `platform_message_id` (actor.rs:600-610). The steered message is never that id.
- Impact: steered messages, and all but the last message of a debounced batch, keep the "processing" emoji indefinitely. This is cosmetic but misleading.
- Fix: after a steer lands, swap the reaction on the steered message to the success emoji. Do the same for constituent ids on flush.

### [P2] Debounced or flush_all failures are only logged; shutdown-drain flush always fails
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:645-651, 663-695, with /Users/indo/code/project/omon-gateway/src/main.rs:1250-1253
- Trigger: the drain marker is detected (`*drain_rx.borrow()` is true), so `is_draining()` is already true when `flush_all` runs. Any message still in the 600ms debounce buffer is claimed and then refused by `multiplexer.route`. The same applies to any message arriving during a drain.
- Impact: the user message is dropped, the ledger entry is marked failed, and the user sees no reply and keeps the start reaction. It is recovered on the next boot only if `missed_backfill` is enabled (default false in `PoiseData::new`). The `flush_all` doc comment promises "no inbound messages are lost", which does not hold on the drain path.
- Fix: on route failure, add the failure reaction and send a short "gateway is restarting, please resend" reply. For the drain path, flush before the drain flag takes effect, or persist the buffered events to the transcript and ledger so recovery can replay them.

### [P2] Thread archive handler now really stops running turns, and its metadata write is fragile
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1417-1437, 1453
- Trigger (stop): the key fix makes `multiplexer.stop` hit the real session. A thread with a 60-minute auto-archive and a turn that runs longer than that without posting is stopped and marked suspended when Discord auto-archives it.
- Trigger (write): the `json_set(... '$.metadata.thread_archived' ...)` UPDATE is a silent no-op when `state_json` lacks a `metadata` object. That is the case after `/reset` (`reset_session` writes `'{}'`, commands.rs:775) or when `ensure_session` creates the row. A live actor's `flush` (actor.rs:967-995) also overwrites `state_json` wholesale from its in-memory state, so the flag is lost, and `json_remove(omo_thread_id)` on delete is undone the same way.
- Impact: a long silent turn can be killed by an auto-archive. `thread_archived` and the delete cleanup are unreliable. Nothing currently reads `thread_archived`, so only the stop behavior is user-visible.
- Fix: stop only on a manual archive, or skip the stop when a turn is active. Route the metadata update through the actor, or use `json_set(json_insert(...))` with a `$.metadata` default.

### [P2] Interaction ids are used as platform_message_id for slash-command events
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:~960 (`/steer`), 1530 (`/retry`), `skill run` and `/thread` (`ctx.id().to_string()`)
- Trigger: any `/steer`, `/retry`, `/thread message:` or `/skill run` that falls through to `multiplexer.route`.
- Impact: the actor indexes these into the message search table as Discord messages (actor.rs:942). Completion reactions target a non-existent message id, and error replies use it as `reply_to`. The adapter then retries without the reference, so the visible effect is only a failed send plus a retry, but the search and inbox index get synthetic rows.
- Fix: give these events an empty `platform_message_id`, or a clearly non-Discord prefix that the indexer skips.

### [P2] steer_claimed_event treats a previously failed delivery as a duplicate
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1481-1485, with /Users/indo/code/project/omon-gateway/src/ledger/service.rs:154-166
- Trigger: the same Discord message id arrives again (gateway resume replay or a backfill overlap) after an earlier delivery was marked `failed`. `is_duplicate` returns true for failed rows, but `record_incoming_as` deliberately reclaims failed rows.
- Impact: the replayed message is silently dropped and logged as a duplicate. The caller also logs it as "steered".
- Fix: use a probe that ignores failed rows, such as `is_completed` or a status check, or return a distinct enum so the caller does not log a steer.

### [P2] /compress and /skill read can exceed the 2000-char limit on an error or long name
- Location: /Users/indo/code/project/omon-gateway/src/discord/commands.rs:1073-1076 (`/compress` failure reply); `skill_dispatch` "read" at ~588-606
- Trigger: a long LLM error string in the compress failure reply. For `/skill read`, a very long `name_or_query` echoed in both the success and error wrappers, or skill content containing a ``` fence inside the ```markdown wrapper.
- Impact: Discord rejects the message, the command errors out, and the user gets no useful reply. An embedded fence also breaks the rendering.
- Fix: run these replies through `chunk_slash_reply` or `preview_text`, and neutralize backticks in the preview.

# discord-adapter review

### [P1] Live bot cursor is advanced at claim time, defeating "hold the cursor for retry" in backfill and live failure replay
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1715 (`route_claimed_event_awaiting_turn`), :1646 (`route_claimed_event_with_constituents`), :1544 (`advance_live_bot_cursor`), :4553 (caller in backfill)
- Trigger: With `DISCORD_MISSED_BACKFILL=true`, backfill replays missed message M. `route_claimed_event_awaiting_turn` claims the ledger row and calls `advance_live_bot_cursor` (:1715) before `route_awaiting_turn`. The turn then fails (LLM error or actor closed). It returns `Err`, marks the ledger row failed, and backfill sets `channel_stopped` with the log "halting channel backfill for retry".
- Impact: The cursor is already at M, and `update_bot_channel_cursor` only moves forward. The next startup fetches strictly after M, so M is never retried, even though its ledger row is `failed` and reclaimable. This contradicts the doc comment on `route_claimed_event_awaiting_turn` (cursor held on turn failure) and the comment in the backfill loop. The live path has the same shape: a debounce flush during drain hits `route()` returning "gateway is draining" at :1646, the row is marked failed, and the cursor has already moved past it.
- Fix: Do not advance the cursor in `route_claimed_event_awaiting_turn`; backfill already advances it after `Ok`. In the live path, advance only after `multiplexer.route` returns `Ok`.

### [P1] Backfill reads each channel's cursor lazily, so live traffic now steals the offline window
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:4414 (`get_bot_channel_cursor` inside the per-channel loop), with the live writes at :1544 and :1646
- Trigger: A startup backfill spawned on `Ready` scans channels sequentially. Each message awaits its whole turn via `route_claimed_event_awaiting_turn`, which can take minutes. The channel-2 cursor is read only after channel 1 finishes. Meanwhile a live message in channel 2 is claimed and `advance_live_bot_cursor` moves channel 2's bot cursor to the live message id. When backfill reaches channel 2 it reads the already-advanced cursor and fetches only messages after it.
- Impact: Everything sent while the bot was offline in that channel is skipped permanently. This is new with the live bot cursor write; previously the live path never touched `discord_bot_cursors`.
- Fix: Snapshot all cursors once, before any replay. `stored_cursor_rows` is already read at the top of `run_missed_message_backfill_with_fetcher`; use that snapshot, and treat a missing row as `None`.

### [P1] Final success/failure reaction targets the parent channel for messages posted in an existing thread
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3900 (`OutboundAction::React` arm, `session.channel_id.parse()`), with the session built in `message_to_inbound_with_config` (`channel_id = parent_channel_id` when `is_thread`)
- Trigger: Any message in an existing thread, with `processing_reactions` on (the default). The session has `channel_id = parent` and `thread_id = thread`. The actor (src/multiplexer/actor.rs ~:648) dispatches `React` with `message_id` = the thread message. The arm uses `session.channel_id` (the parent) as the reaction channel.
- Impact: The reaction call goes to the parent channel with a thread message id and gets Unknown Message. The error is only logged at debug level, so the 👀 is never replaced by ✅ or ❌ in threads. The dead-target check also runs against the parent channel id. The arm's comment is correct only for auto-created threads, where the original message does live in the parent.
- Fix: Carry the message's actual channel in the `React` action or in the event metadata. Otherwise try `thread_id` first and fall back to `channel_id`.

### [P2] A text "/stop" is not recognized when topic or channel context is prepended
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1275 (and :1624)
- Trigger: With `DISCORD_CHANNEL_TOPIC_CONTEXT` on and a channel that has a topic, `event.content` becomes `topic_block + "\n\n/stop"` before the `eq_ignore_ascii_case("/stop")` check. The same happens with `DISCORD_CHANNEL_CONTEXT` on an explicit mention.
- Impact: The stop is treated as a prompt. With the new instant-steer path it is injected into the running turn instead of stopping it. The slash `/stop` command still works.
- Fix: Evaluate the stop check on the raw stripped message content, before the context decoration.

### [P2] A steered message keeps the 👀 reaction forever
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1279-1290 (steer arm); the reaction is added earlier in `handle_event`
- Trigger: A message is steered into an active turn. The 👀 start reaction was already added in `handle_event`. The actor only reacts for the platform message id of the turn's own originating event.
- Impact: The steered message never gets ✅ or ❌.
- Fix: Add a success reaction after a successful steer, or skip the start reaction on the steer path.

### [P2] A media or table error after the text is sent leaves state that causes duplicate side effects on retry
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs `DiscordEgress::stream` (the "Keep the stream entry when a media/table problem is surfaced" block, ~:3400-3440) and `dispatch` SendMessage (`media_error` returned after the text is sent)
- Trigger: A reply contains a MEDIA directive whose upload fails or whose path is rejected, and the text itself sends.
- Impact:
  - The `streams` map entry is never removed, so it leaks.
  - The caller sees an `Err`: cron marks the job failed and may retry, resending the whole text and re-uploading the files.
  - The same re-upload happens if `throttler.update` fails after `deliver_media_directives` already uploaded.
- Fix: Treat a media failure as a non-fatal note, which the code already appends to the text. Alternatively remove the stream entry regardless and do not return `Err` once the text is delivered.

### [P2] Dead-target persistence tasks can land out of order
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs `DeadTargetRegistry::mark_dead_for_bot` / `clear_for_bot` (each does a fire-and-forget `tokio::spawn`)
- Trigger: A mark followed quickly by a clear (a probe succeeds) spawns two unordered DB writes.
- Impact: The persisted row can stay dead while memory is clean. After a restart the channel is short-circuited until the 60s probe interval passes. The `React` arm also calls `is_dead_for_bot` (:3900), which can consume the single probe slot meant for a real send.
- Fix: Await the persistence or serialize it. Use `is_dead_for_bot_peek` in the `React` arm.

### [P2] Backfill scan set grows without bound
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs `run_missed_message_backfill_with_fetcher` (scans every `discord_bot_cursors` row for the bot) combined with `advance_live_bot_cursor`
- Trigger: Live traffic now creates a cursor row for every thread, DM and channel the bot has spoken in.
- Impact: Each startup with backfill on issues at least one REST fetch per historical channel. Each fetched message also triggers a `get_channel` call, which risks rate limiting.
- Fix: Bound the scan by recency (`updated_at`) or drop cursor rows for deleted or archived threads.

### [P2] Instant-steer bypasses multiplexer admission and drain
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs `steer_claimed_event` -> `multiplexer.steer` (src/multiplexer/router.rs:504)
- Trigger: A message arrives during drain or shutdown while a turn is still active.
- Impact: `steer` takes no admission read lock and does not check `is_draining`, unlike `route`, so the steer lands after the drain decision. Because it returns early, the message also skips `mark_in_flight_resume_pending` bookkeeping for that delivery.
- Fix: Check `multiplexer.is_draining()` before steering and fall through to the normal path.

### [P2] Thread-archive state update fails on an empty `state_json`
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs `handle_thread_update` / `handle_thread_delete` (`COALESCE(state_json, '{}')`)
- Trigger: A session row with `state_json = ''`. Other call sites in the repo use `NULLIF(state_json, '')`.
- Impact: `json_set` errors and the failure is only logged as a warning.
- Fix: Use `COALESCE(NULLIF(state_json, ''), '{}')`.

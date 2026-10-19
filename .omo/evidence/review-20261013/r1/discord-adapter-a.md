# discord-adapter-a review

I did not read lines 4145-4220 or the other backfill code past line 2800. Any finding that depends on that code is graded P2 for that reason.

### [P1] MEDIA directive upload allows the gateway's working directory, which holds `.env` and the SQLite DB
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:44-80 (callers at :3111-3117 and :3241-3247)
- Trigger: A model reply, or a prompt-injected reply, contains `MEDIA:.env` or `MEDIA:omon_gateway.db`. The path is relative and resolves against the gateway's cwd.
  - `main.rs:638` loads `.env` from the cwd.
  - `main.rs:188-189` defaults `DATABASE_URL` to `sqlite://omon_gateway.db`, relative to the cwd.
  - `validate_media_path` accepts anything under `cwd` (`is_in_cwd`). Its denylist only covers `/etc`, `/proc`, `/sys`, `.ssh` and `.gnupg`.
- Impact: Bot tokens, API keys and the full message and session DB can be uploaded to a Discord channel. Anyone who can influence model output, for example through fetched content or channel messages, can trigger it. The configured `workspace_root` (default `~/.omon/workspace`) is not an allowed root unless it happens to sit under the cwd or the temp dir. Legitimate workspace files are therefore rejected while the secret-bearing cwd is allowed.
- Fix: Replace the cwd root with the configured workspace root, which `DiscordEgress` already stores as `workspace_root`, plus the temp dir. Add an explicit denylist for `.env*`, `*.db` and `*.sqlite*`. Compare path components rather than string prefixes: `starts_with("/tmp")` also matches `/tmpfoo`.

### [P1] Any prose containing "media: <word>" aborts the whole reply
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:31-36, 85-128, 3111-3117, 3241-3247
- Trigger: The agent writes ordinary text such as "Social media: Twitter and Reddit". `MEDIA_DIRECTIVE_RE` is case-insensitive and has no word boundary or line-start anchor, so it captures `Twitter` as a path. `validate_media_path(...)?` then fails with "media file does not exist". `dispatch(SendMessage)` and the final `stream()` return `Err` before any text is sent.
- Impact: The reply is never delivered.
  - On the stream path the `"\u{200b}"` placeholder message has already been created. It stays as a blank message, and the entry in `self.streams` leaks because the early `?` skips the removal at 3131-3137.
  - Files from earlier directives in the same reply were already uploaded, so a retry uploads them again.
- Fix: Anchor the directive: require `MEDIA:` at the start of a line or after whitespace, with a word boundary. Alternatively, treat a path that fails validation as plain text and keep sending the message instead of returning `Err`. Never `?`-abort the text delivery because of a media failure.

### [P1] Coalesced (debounced) events drop `author_id` and `referenced_message_id`
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:565-571 (inputs set at :2017-2051)
- Trigger: A user sends two or more messages in the same session within the 600 ms debounce window. This is common with split pastes. `coalesce_inbound_events` rebuilds the event with `InboundEvent::message(...)`. That sets only `id`, `received_at` and `delivery_id`, so `author_id` and `referenced_message_id` end up `None`. Single-event batches are returned unchanged, so only multi-message batches are affected.
- Impact: The indexing code (`message_search.rs:206-211`) falls back to `session.user_id`, which is `""` in guild channels. The indexed document therefore has an empty author. The reply link, `metadata.referenced_message_id`, is missing. `inbox/sources/discord.rs` uses both fields to classify `owner_reply` and to exclude the owner's and bots' own messages. A split reply to the owner's message is therefore never surfaced as an owner reply, and the owner's and bots' own coalesced messages can pass the author exclusions.
- Fix: Carry the fields over from the last event, or from the first for `author_id`: `coalesced.author_id = last.author_id.clone(); coalesced.referenced_message_id = unique_events.iter().rev().find_map(|e| e.referenced_message_id.clone());`. Also preserve `origin`.

### [P1] Gateway receive watchdog is never started in the production startup path
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:764-769, 2393-2445
- Trigger: A zombie gateway session, meaning the connection is up but no dispatch events arrive. `spawn_receive_watchdog` is called only from `DiscordAdapter::start`. Production uses `adapter.client(token)` and `client.start()` directly (`main.rs:1135-1136, 1224`), and I found no caller of `DiscordAdapter::start`. `shard_manager` is only used for shutdown there.
- Impact: The documented self-heal never runs, so the bot stays silently deaf until someone restarts the process.
  - Even if it were wired, `LAST_DISCORD_EVENT_MS` is process-global and shared by all bot clients. Events on one healthy bot would mask a zombie shard on another.
  - It also only restarts `ShardId(0)`.
- Fix: Spawn the watchdog per client in `main.rs`, using each client's `shard_manager`. Track a last-event timestamp per bot instead of one global.

### [P2] Any 403 on any action marks the whole channel dead, and probe slots are consumed by non-send checks
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:2550-2585, 2771-2799, 3024-3031, 3319-3330, 3459-3470
- Trigger: A 403 that is not channel-wide, for example 50005 "Cannot edit a message authored by another user", 50013 on one operation, or a 404 with error code 0. `is_discord_dead_target_error` treats every 403 as dead, and `main.rs:949-951` uses a 60 s probe interval with no TTL. `is_dead_for_bot` also consumes the one-shot probe (`probed_at`) from callers that never send, such as `keep_typing` and the typing arm. A real send afterwards still sees the channel as dead.
- Impact: All replies to the channel fail with "dead target short-circuited" for up to a probe interval, and a recovered channel stays dead longer than needed.
- Fix: Mark dead only for channel-level codes: 10003, 50001, and 50013 only when it came from a send. Make the probe consume its slot only on a real send attempt.

### [P2] `clear_for_bot` and `mark_dead_for_bot` spawn unordered DB tasks on every send
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:2589-2648
- Trigger: Every successful chunk send calls `clear_for_bot`, which does `tokio::spawn(remove_dead_target)` even when the channel was never dead. A `mark_dead` persist task and a later `clear` task are independent spawns, so they can run in either order. Errors are dropped with `let _ =`.
- Impact: This costs one DB write task per sent chunk. A stale dead row can survive a recovery, and `load_from_db` revives it as dead after the next restart.
- Fix: Spawn only when the in-memory entry was actually removed, or use the awaited `*_and_persist` variants. Serialise persistence per key.

### [P2] `self.streams` mutex held across the network placeholder send
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3081-3101
- Trigger: Creating a stream's placeholder message (`send_message_with_reference(...).await`) happens while holding the global `streams` lock. A slow or rate-limited send for one session blocks `stream()` for every other session and bot on that egress. Holding the lock does serialise duplicate placeholder creation for the same `stream_id`.
- Impact: Cross-session head-of-line blocking.
- Fix: Use a per-key `OnceCell`, or insert a pending entry and release the lock before the send.

### [P2] Debounce batches can be routed out of order
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:594-654, 1522-1592
- Trigger: A user posts an image, and a text message follows more than 600 ms later. Each flush runs `route_claimed_event_with_constituents` in its own spawned task, which downloads attachments and transcribes audio before `multiplexer.route`. If the image batch's download or transcription outlasts the text batch's flush, the text batch is routed first. Steer-injected messages (lines 1228-1250) also bypass any batch still pending in the buffer.
- Impact: The model sees messages in the wrong order, for example "analyze this" before the image.
- Fix: Serialise routing per session, or hydrate before the debounce window starts.

### [P2] A "/stop" handled live leaves no ledger row and can be replayed as a prompt by backfill
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1527-1531 and 4233-4244
- Trigger: `missed_backfill` is enabled and a user sends `/stop`. It is handled and returns before any ledger claim, so no row exists. Live messages only advance the legacy `discord_channel_cursors` table (line 907). `get_bot_channel_cursor` falls back to that table only when `bot_id` is empty. The bot cursor therefore only moves during backfill. After a restart, backfill refetches the message, `record_incoming_as` succeeds, and the text "/stop" runs as an agent prompt. I did not read the backfill body at 4145-4220, so I could not confirm it has no filter.
- Impact: Spurious agent turns on old control messages after a restart.
- Fix: Make `route_claimed_event_awaiting_turn` treat "/stop" as a no-op, and record a delivered ledger row for handled control messages.

### [P2] `thread_sessions_per_user` is accepted and passed through but never read
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:419, 439, 790, 988
- Trigger: The operator sets `DISCORD_THREAD_SESSIONS_PER_USER`. `message_to_inbound_with_config` always uses `user_id = String::new()` for threads (lines 1990-2010), and `thread_session_key` does the same.
- Impact: The option has no effect, and per-user thread sessions can never be enabled.
- Fix: Wire the flag into session-key construction in both places, or remove the option.

### [P2] Per-message REST calls and message-content logging before any authorization filter
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:907-920, 936-943, 951-973
- Trigger: Every message in every channel the bot can see goes through `update_channel_cursor`, an INFO log of the full `content`, a `to_channel` call, `ctx.http.get_current_user()` (a REST call although `ctx.cache.current_user()` is available), and possibly `get_member`. This happens before any allowlist check.
- Impact: Message content from unauthorized users and channels is written to logs, and a DB row is written per channel. A busy guild generates one REST round trip per message, which risks 429 backlog that delays handling of authorized messages. If `to_channel` fails transiently, the message is dropped (lines 927-934) after the cursor has already advanced.
- Fix: Move the log, the cursor write and the REST calls after the authorization filter. Use the cache for the bot user ID. Do not log message content at INFO.

### [P2] `OutboundAction::UploadFile` bypasses `validate_media_path`
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3523-3535 (uploader at :2295-2350)
- Trigger: An `UploadFile` action with an arbitrary `path`. `SerenityFileUploader::upload` reads the whole file with `tokio::fs::read` without path validation or a size cap, and clones the bytes again for the voice fallback. I found no production producer of this action in `src`, so this is hardening.
- Impact: Path restrictions are enforced only on the MEDIA route, and a very large file is read fully into memory.
- Fix: Validate the path inside the uploader, and check the size before reading.

### [P2] Hard-coded owner ID fallback grants owner semantics to a literal user
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1721-1731
- Trigger: `DISCORD_ALLOWED_USERS` is unset or empty in a deployment. `intercept_lane_owner_reply` then treats the literal `414011632306618368` as the owner. It uses the first entry of the comma list otherwise, which can differ from `status_reply::owner_id()`.
- Impact: Lane owner-reply interception is tied to a specific account in any deployment. It still requires that account's own message and the normal authorization checks.
- Fix: Fail closed when no owner is configured, and share one owner-resolution function.

### [P2] `bind_lane_thread` mismatch aborts message handling for owner messages in a lane thread
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:1760-1766 (callee `storage/lanes.rs:214-221`)
- Trigger: A lane was bound to bot Y, and later the thread owner changes to bot X. This can happen when `mark_thread_owner` is called on an explicit mention at lines 1275-1283. Each later owner message calls `bind_lane_thread`, which returns `Err("lane bot binding mismatch")`. `handle_event` propagates it with `?`.
- Impact: Every later owner message in that thread is dropped, with an error returned from the event handler, and never reaches normal routing.
- Fix: Treat a mismatch as "not intercepted" (`Ok(false)`) and log it.

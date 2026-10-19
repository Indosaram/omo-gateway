# discord-adapter-b review

### [P1] A `MEDIA:` false positive in an ordinary reply drops the whole message, and the stream path leaves a blank placeholder
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3241 (SendMessage), :3111 (stream final). The regex and path check are at :37 and :44.
- Trigger: A final reply contains text like "Social media: Instagram is ..." or "multimedia: foo". `MEDIA_DIRECTIVE_RE` is case-insensitive with no word boundary, so it captures `Instagram` as a media path. `validate_media_path` then returns Err because the file does not exist, and `?` aborts the whole dispatch.
- Impact:
  - In `dispatch(SendMessage)`, nothing is sent and the reply text is lost.
  - In `stream()`, the `"\u{200b}"` placeholder message has already been created. It stays blank in the channel, the stream entry stays in `streams`, and each retry fails the same way.
- Fix: Anchor the directive at line start or after whitespace, using `(?im)(?:^|\s)MEDIA:`. Treat a bad or nonexistent path as a warning: skip that file, keep the text, and still send it. In `stream()`, validate and upload media before creating the placeholder, or edit the placeholder with the error.

### [P1] `MEDIA:` upload allows any file under the process cwd, so a prompt-injected reply can exfiltrate secrets
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:52-70 (`is_in_cwd` and the denylist), called from :3113 and :3243.
- Trigger: Untrusted content the agent reads (a web page or message) makes the model emit `MEDIA:.env`, `MEDIA:omon.db` or similar. The relative path resolves against the gateway cwd and passes `is_in_cwd`. The denylist only blocks `/etc`, `/proc`, `/sys`, `.ssh` and `.gnupg`. `SerenityFileUploader::upload` then posts the file to the channel.
- Impact: Gateway secrets (`.env`, token files, the SQLite database) can leak to every channel member. Any file in `/tmp` is also uploadable.
- Fix: Allowlist an explicit output or workspace directory instead of the whole cwd and `/tmp`. Add a denylist for dotfiles, `*.db` and `*.env`. Replace the string-prefix `/tmp` checks with component-wise `starts_with` on the canonical path.

### [P2] The live message path never advances the bot cursor that backfill reads
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:4094 and :4238; the live write is at :907.
- Trigger:
  - Backfill is enabled and a channel has no backfill-routed message yet. The gateway is down while more than 50 messages arrive in a free-response channel.
  - Live handling writes only `discord_channel_cursors` through `update_channel_cursor` (:907).
  - Backfill reads `discord_bot_cursors` through `get_bot_channel_cursor`. The legacy fallback fires only when `bot_id` is empty, and `bot_id_str` is never empty.
- Impact:
  - With no cursor, the fetch is "latest 50", so older missed messages are silently skipped.
  - With a stale cursor, backfill rescans everything since then. It makes a `get_channel` and a `get_member_roles` API call per message and relies on ledger dedup.
  - Filtered messages are never persisted as cursor progress.
- Fix: Call `update_bot_channel_cursor` from the live path after a message is durably claimed or routed. Alternatively, have backfill read the max of both cursors.

### [P2] A text `/stop` has no ledger row, so backfill can replay it to the agent as a prompt
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:4233 (`route_claimed_event_awaiting_turn`, defined at :1598). The live `/stop` early return is at :1527-1531.
- Trigger: The user sends `/stop`, which is handled live and returns before any ledger write. The gateway restarts with `DISCORD_MISSED_BACKFILL` on, and the message is within the replay window (see the previous finding). `route_claimed_event_awaiting_turn` has no `/stop` handling and records the claim as a normal inbound turn.
- Impact: The agent receives "/stop" as a new user turn.
- Fix: Skip or handle `/stop` in the backfill route, and record a ledger row for stop commands.

### [P2] The `streams` mutex is held across a network call
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3082-3104 (`send_message_with_reference` at :3094 is awaited while the guard is held).
- Trigger: Any new stream is created, or Discord is rate-limiting sends.
- Impact: Every other session's `stream()` call blocks on `self.streams.lock()` until the send returns, so one slow session stalls all of them.
- Fix: Clone the entry or reserve a slot under the lock, drop the guard, send the message, then re-lock and insert. Use `entry` plus an `Arc<OnceCell>` to avoid double creation.

### [P2] `filter_reasoning` is used only for the silence check, so `<think>` text is sent as-is
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3206 and :3415. The send path builds from `content` at roughly :3239-3241. `EditMessage` uses `.content(content)`. `derive_forum_post_title(&content)` also reads the raw content.
- Trigger: Any `SendMessage` or `EditMessage` whose content contains `<think>...</think>` or similar tags. The stream path correctly uses `filtered_content`.
- Impact: Reasoning text reaches the channel. Edits are also unchunked and unvalidated, so content over 2000 characters gets a Discord 400.
- Fix: Use `filtered_content` as the base for decoration, forum titles and edits. Truncate or chunk edit content.

### [P2] `OutboundAction::Typing` consumes the dead-target probe slot
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3561 (Typing); the probe side effect is in `is_dead_for_bot` at :2550-2583.
- Trigger: A channel was marked dead and has since recovered. A turn starts, and `Typing{active:true}` calls `is_dead_for_bot`, which returns false once per 60s and sets `probed_at`. The typing result never clears or marks the channel. The reply `SendMessage` a few seconds later sees `elapsed < 60s` and short-circuits with Err.
- Impact: Replies in a recovered channel can be dropped for several turns until a probe lands on a real send.
- Fix: Make the probe check read-only for Typing and keep_typing, or call `is_dead_for_bot` only on send paths. Alternatively, clear the dead mark when typing succeeds.

### [P2] `ChannelId::new(0)` in React panics when the session channel ID is not numeric
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:3594
- Trigger: A `React` action for a session where both `session.channel_id` and the thread target fail to parse as u64. `serenity::ChannelId::new(0)` panics on zero, which would abort the actor task.
- Impact: Panic, though I found no current normal-path producer: web sessions short-circuit in `CompositeDispatcher`.
- Fix: Return `OmonError::Config` instead of falling back to 0, consistent with `Self::target`.

### [P2] Any 403 or generic 404 marks the whole channel dead, and cursor writes can hit SQLITE_BUSY
- Location: /Users/indo/code/project/omon-gateway/src/discord/adapter.rs:2781-2796 (`is_discord_dead_target_error`) and :3889 (deferred `pool.begin()` in `update_bot_channel_cursor`).
- Trigger:
  - A single-operation 403 such as 50013 (no Attach Files) on `UploadFile`, or any 404 with code 0, marks the whole channel dead. Text replies are then short-circuited until a probe.
  - Two writers hit the SQLite pool while a deferred transaction upgrades from SELECT to INSERT. The failure is swallowed by `let _` at :4238.
- Impact: Temporary blackout of the channel, or a silently skipped cursor advance.
- Fix: Mark dead only on 10003, 50001 and 404 on channel routes, not on 50013. Use `BEGIN IMMEDIATE` or a single `INSERT ... ON CONFLICT ... WHERE excluded > current` statement for the cursor.

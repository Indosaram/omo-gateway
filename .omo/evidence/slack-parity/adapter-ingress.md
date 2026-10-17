st_01a1474c [completed] model mahoquot/claude-opus-5-5
Ask: Build an exhaustive Discord→Slack parity gap list for adapter-ingress (`src/discord/adapter.rs` lines 1-2300), checked against `src/slack/*`. - wanted: a table mapping each behavior to its Discord location, Slack status, what Slack needs and the files to change, plus a list of hardcoded Discord coupling. For you: The table is complete for the whole scope (1/1). Slack covers the basic routing pipeline: dedupe, `/stop`, steer, the 👀 reaction, attachment download, mention and thread gating, DM keys, and reconnect. It has nothing for debounce, channel and topic context, reply context, forwarded messages, voice and text-file processing, session-status and lane interception, thread lifecycle, backfill and cursors, or a receive watchdog. Several shared helpers only accept Discord. Everything was checked by reading code; no cargo build or test was run. Now: complete. Next: none.

<analysis>
**Literal Request**: Parity gap table for Discord adapter ingress against the Slack adapter.
**Actual Need**: A concrete implementation backlog for Slack ingress, with file and line anchors and write scope.
**Success Looks Like**: Every behavior in scope is classified and cited, and the Discord-hardcoded coupling is listed.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/adapter.rs - the Discord ingress being compared (handle_event at 892, filter at 1924, debouncer at 558-765, routing at 1589-1790, backfill and cursors at 4237-4700, watchdog at 2537-2640).
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/ingress.rs - the whole Slack ingress (handle at 42, map_event at 189).
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/runtime.rs - Socket Mode dispatch and supervisor.
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/socket.rs - reconnect, ping and ack handling.
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/api.rs - available Web API methods (no conversations.history or conversations.replies).
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/mod.rs - SlackConfig (no context or channel-list flags).
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/platform_router.rs - outbound routing by platform; the Discord dispatcher is the fallback.
- /Users/indo/code/project/omon-gateway-wt/slack/src/sessions/status_reply.rs - owner and status-intent logic, partly Slack-aware.
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/actor.rs - cursor rewind and owner-lead logic, Discord-only.
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/attachments.rs - text-file inlining and STT in hydrate().
- /Users/indo/code/project/omon-gateway-wt/slack/src/inbox/proposals.rs - hardcoded owner ID.
- /Users/indo/code/project/omon-gateway-wt/slack/docs/slack-app-manifest.json - bot_events and scopes.
</files>

<answer>
## Parity table

| Behavior | Discord impl | Slack status | What Slack needs | Write scope |
|---|---|---|---|---|
| Inbox triage "제안" list and "<번호> 해" approve-by-reply (owner DM, primary bot) | adapter.rs:899-1017 | MISSING | Owner-DM branch in `SlackIngress::handle` before mapping. Post chunks with `chat.postMessage`, call `Proposals::presented(number, channel, ts)`, and use `thread_ts` (or the parent `ts`) as the reference. Owner must come from `SlackConfig::owner_id()`, not `proposals::OWNER_ID`. | slack/ingress.rs, inbox/proposals.rs |
| Legacy per-channel live cursor on every message | adapter.rs:1019-1024 | MISSING | Slack-specific cursor write. The existing helper is Discord-table-only (see the backfill row). | slack/ingress.rs, new slack cursor storage |
| Guild channel metadata fetch; drop if it fails | adapter.rs:1033-1050, 1969-1971 | PARTIAL: uses event `channel_type` (ingress.rs:262-267) | Fine as is. Note that `mpim` counts as a channel (guild = team), so group DMs need a mention or a free channel. Treat `mpim` as a DM if parity with Discord group DMs is wanted. | slack/ingress.rs |
| Webhook and bot drop; `allow_bots` None/Mentions/All | adapter.rs:1929-1949 | PARTIAL: drops every `bot_id` (ingress.rs:209-211) | Add an `allow_bots` mode to SlackConfig (env `SLACK_ALLOW_BOTS`). With Mentions, accept `bot_id` events whose text contains `<@bot>`. | slack/mod.rs, slack/ingress.rs |
| Drop self | adapter.rs:1933 | DONE ingress.rs:213 | - | - |
| Only regular and reply message kinds (system messages ignored) | adapter.rs:1952-1958 | DONE ingress.rs:196-205 (subtype allowlist: none, file_share, thread_broadcast) | - | - |
| Channel allow and ignore lists | adapter.rs:1973-1981 (`is_channel_authorized`) | MISSING | `allowed_channels` and `ignored_channels` in SlackConfig, checked in `map_event`. | slack/mod.rs, slack/ingress.rs |
| User auth: allowlist, roles, allow_all, paired users | adapter.rs:1983-1995 | PARTIAL: allowlist only (ingress.rs:213, mod.rs:79) | Optional `allow_all_users`. Slack user-group check (`usergroups.users.list`) to stand in for roles. Pairing store keyed by string. | slack/mod.rs, slack/ingress.rs, discord/pairing.rs (u64 ids) |
| Unauthorized-DM pairing prompt with throttling | adapter.rs:1337-1360, 1797-1850 | PARTIAL: silent drop | Effectively at parity by default: Discord only prompts when no allowlist exists, and Slack always has one (falls back to DEFAULT_OWNER_ID). If allow-all or pairing is added, need a prompt via `chat.postMessage` in the DM plus `PairingStore`, which needs non-u64 ids. | slack/ingress.rs, discord/pairing.rs |
| Mention gating: other bots mentioned → drop; DM, free channel and active thread are implicit | adapter.rs:1997-2056 | DONE/PARTIAL ingress.rs:216-238 | Uses `app_mention` for mentions; plain `message` needs a free channel or a known thread. Missing: `thread_require_mention` and a parent-free-channel equivalent (free-channel thread replies already pass because the channel id matches). | slack/mod.rs, slack/ingress.rs |
| Durable thread ownership and active-thread tracking | adapter.rs:1063-1075, 1243-1249 | DONE (different mechanism): `sessions` row exists (ingress.rs:43-46, 168-174) | Works per bot user. There is no per-bot owner for multi-bot setups (single bot, so N/A). | - |
| Primary-bot gating for multiple bots | adapter.rs:2044-2054 | N/A (one Slack app) | - | - |
| Strip bot mention | adapter.rs:2061, 2191-2197 | DONE ingress.rs:242 | - | - |
| Session key shape (DM per user; thread uses parent channel plus thread id) | adapter.rs:2127-2160 | DONE ingress.rs:220-239, mod.rs:3-6 | - | - |
| Delivery id and author id | adapter.rs:2162-2168 | DONE ingress.rs:249-250 (`slack:{channel}:{ts}`) | - | - |
| received_at from platform timestamp | adapter.rs:2152-2157 | DONE ingress.rs:252-254, 304-314 | - | - |
| `referenced_message_id` (feeds owner-reply classification and coalescing) | adapter.rs:2169-2179 | MISSING | Set it to `thread_ts` for thread replies (`with_referenced_message_id` or a field assignment). | slack/ingress.rs |
| Reply context: quoted parent capped at 500 chars, plus parent attachment summary | adapter.rs:2107-2127, 502-528 | MISSING | When the thread root is not in the transcript (for example a free-channel thread on a human root), fetch the root with `conversations.replies(channel, ts=thread_ts, limit=1)`. Prefix with `compose_reply_context(users.info name, text, body)`. Add `conversations_replies` to api.rs. | slack/api.rs, slack/ingress.rs |
| Union of the referenced parent's attachments | adapter.rs:2085-2102 | MISSING | Same fetch as above; map the root's `files` with `attachments()`. | slack/ingress.rs |
| Forwarded snapshots → "[Forwarded]" block plus attachments | adapter.rs:417-480, 2058-2083 | MISSING | Parse `event.attachments[]` entries with `is_share` / `is_msg_unfurl` (fields `text`, `author_name`, `files`, `message_blocks`). The Slack payload shape is unverified, so check against a live event. Reuse the block format. | slack/ingress.rs |
| Session-status interception (owner DM or private owner thread → direct reply) | adapter.rs:1110-1136 | MISSING in ingress. Helpers are Slack-aware: status_reply.rs:49-53 `owner_dm`; router.rs:446 `intercept_session_status` | After `map_event`, compute `private_owner = status_reply::owner_dm(&inbound.session, user, "")` (the Slack branch ignores the `owner` arg) and call `multiplexer.intercept_session_status`. `owner_thread` is Discord-only (status_reply.rs:66-75). Replies use `dispatch_direct_reply` → SlackEgress SendMessage. | slack/ingress.rs (optionally sessions/status_reply.rs) |
| Lane owner reply interception | adapter.rs:1138-1149, 1852-1922 | MISSING / likely N/A | Lanes bind Discord thread channels (storage `lanes.thread_channel_id`, `Database::get_thread_owner(u64)`). Only relevant if lanes get Slack threads. Then call `storage::lanes::record_owner_reply(pool, thread_ts, bot, user, owner, ts, text)` with the Slack owner, followed by `lanes::runner::wake`. | slack/ingress.rs, storage/lanes.rs |
| Channel topic and forum description context | adapter.rs:1154-1178, 228-250 | MISSING (`conversations_info` exists at api.rs:122 but nothing calls it) | Add a `channel_topic_context` flag. Read `channel.topic.value` and `channel.purpose.value` from `conversations.info` and pass them to `format_channel_topic_context`. Cache per channel. | slack/mod.rs, slack/ingress.rs |
| Recent channel context on mention (limit 10, max 25) | adapter.rs:1180-1214, 202-226 | MISSING | Add `channel_context` and `channel_context_limit` flags. `conversations.history(channel, latest=ts, inclusive=false, limit)`, or `conversations.replies` inside a thread. Skip the bot's own messages, map names with `users.info` (api.rs:129), and pass the result to `format_channel_context`. Add `conversations_history` to api.rs. | slack/api.rs, slack/mod.rs, slack/ingress.rs |
| Processing-start reaction (gated by flag) | adapter.rs:1216-1238 | DONE ingress.rs:72-79, always on (no `processing_reactions` toggle) | Optional config toggle. | slack/mod.rs |
| Auto-thread on mention (not in free channels, not on replies) | adapter.rs:1239-1303, 503-511 | PARTIAL ingress.rs:234-238 | Slack does auto-thread top-level mentions inside free channels; Discord excludes them. Add `!free` to the condition. Thread naming and create-failure handling are N/A because Slack needs no API call. | slack/ingress.rs |
| Forum post title, forum parent | adapter.rs:344-386 | N/A (no forums in Slack) | - | - |
| Text `/stop`: cancel the debounce batch and stop immediately | adapter.rs:1305-1309, 1654-1658 | DONE ingress.rs:66-70 (nothing to cancel because there is no debouncer) | Cancel the batch once a debouncer exists. | slack/ingress.rs |
| Instant steer into the active turn | adapter.rs:1310-1332, 1601-1647 | DONE ingress.rs:81-93, 134-166 | - | - |
| Split-message debounce and coalescing (600 ms, constituent ledger ids) | adapter.rs:536-765, 1313-1336 | MISSING | Generalize `SplitMessageDebouncer` off `PoiseData` (it takes PoiseData and falls back to "discord:" ids at 611, 692, 737). Alternatively add a Slack debouncer that routes via `record_incoming_with_constituents`. Also, runtime.rs:77 spawns one task per envelope, so a burst can arrive out of order; a per-session debounce or queue fixes that too. | discord/adapter.rs (generalize) or new slack/debounce.rs, slack/ingress.rs, slack/runtime.rs |
| Flush debounce batches at shutdown and drain | adapter.rs:722-760; main.rs:1528-1573 | MISSING | Call the Slack debouncer flush in the same `select!` arms. | main.rs |
| Ledger claim, then route, then mark failed | adapter.rs:1650-1722 | DONE ingress.rs:56-64, 95-105 | - | - |
| Attachment download | adapter.rs:1679-1690 (`AttachmentDownloader::hydrate`) | DONE (basic) ingress.rs:99, 109-130; api.rs:199 (50 MB cap, versus Discord 25 MB at attachments.rs:12) | - | - |
| Inline text attachments (≤100 KB into `text_content`) | attachments.rs:335-350 | MISSING | After download, if `is_text_attachment` and size ≤ `MAX_INLINED_ATTACHMENT_BYTES`, read the file into `text_content`. | slack/ingress.rs |
| Voice and audio transcription ("🎤 [음성 전사]" prefix, failure note) | adapter.rs:1688-1712; attachments.rs:259-333 | MISSING | After download, if `voice_transcription::is_audio_attachment(mimetype, name)` is true (Slack clips are `audio/webm` or `audio/mp4`, subtype `slack_audio`; unverified), call `voice_transcription::transcribe_audio(local_path)` (platform-agnostic, voice_transcription.rs:68) and prepend the same prefix. | slack/ingress.rs |
| Advance per-bot live cursor after routing | adapter.rs:1572-1587, 1720 | MISSING | Slack cursor write keyed by (bot, channel or thread_ts) with the ts. `update_bot_channel_cursor` rejects non-numeric ids (adapter.rs:4301-4303), so it is a silent no-op for Slack ts values. | new slack cursor fn plus migration, slack/ingress.rs |
| Missed-message startup backfill (on Ready) | adapter.rs:1450-1465, 4444-4700 | MISSING | After `auth_test` in `runtime::build` or `serve`, scan stored Slack cursors, free channels and known thread sessions. Use `conversations.history(oldest=cursor)` and `conversations.replies(ts, oldest)` for threads, run each message through `map_event`, and route with `multiplexer.route_awaiting_turn` (same as `route_claimed_event_awaiting_turn` at 1729-1781, skipping `/stop`). Advance the cursor only after the turn completes. | slack/api.rs, new slack/backfill.rs, slack/runtime.rs, slack/mod.rs (flag) |
| Rewind cursor for queued messages dropped at shutdown | actor.rs:31-80 | MISSING (returns for non-discord, actor.rs:32) | Slack branch against the Slack cursor table, using ts decrement or string compare. | multiplexer/actor.rs |
| Thread lifecycle: archive stops the turn and sets `metadata.thread_archived`; delete stops the turn and clears `omo_thread_id` | adapter.rs:1467-1567 | MISSING | Subscribe to `channel_archive`, `channel_unarchive`, `channel_deleted`, `group_archive`, and `message` subtype `message_deleted` where `deleted_ts == thread root`. Map them to the session keys (all thread sessions in the channel) and run the same SQL. Add the events to manifest `bot_events`. | slack/ingress.rs (or slack/lifecycle.rs), docs/slack-app-manifest.json |
| Thread-create log | adapter.rs:1483-1490 | N/A | - | - |
| Receive watchdog (silence → shard restart, env `OMON_DISCORD_RECEIVE_WATCHDOG_SECS`, default 1200 s) | adapter.rs:799, 824, 2537-2640 | PARTIAL: reconnect on close, error or `disconnect` (socket.rs:86-117, 202-206) and pong (144-149), but `ws.next()` has no read timeout | Wrap `ws.next()` in `tokio::time::timeout(threshold)` inside `session()` and return `Dropped("receive watchdog")`. Add env `OMON_SLACK_RECEIVE_WATCHDOG_SECS`. | slack/socket.rs |
| Startup identity check | serenity Ready | DONE runtime.rs:33-35 | - | - |

## Hardcoded Discord coupling (affects Slack ingress)

- `inbox/proposals.rs:10` hardcodes `OWNER_ID = "414011632306618368"`. The triage gate at adapter.rs:899-904 also requires that ID, `guild_id.is_none()` and the primary bot. Users: `inbox/list.rs:46`, `inbox/set.rs:32,37,59`, `inbox/triage.rs:357,446`, `main.rs:1310-1311`.
- `sessions/status_reply.rs:36-47`: `owner_id()` reads `DISCORD_ALLOWED_USERS` and falls back to the Discord owner ID. The lane interceptor uses it at adapter.rs:1858, and adapter.rs:1110.
- `sessions/status_reply.rs:66-75`: `owner_thread` requires `platform == "discord"` and `DISCORD_HOME_CHANNEL_THREAD_ID`.
- `multiplexer/actor.rs:31-33`: cursor rewind is Discord-only; it uses table `discord_bot_cursors` and a `'discord:%'` constituent prefix at 45-46.
- `multiplexer/actor.rs:1250-1325`: `is_owner_lead_session` has a Slack branch, but the Discord path hardcodes bot `1465631383862120451`, the owner fallback `414011632306618368` (1288), `DISCORD_HOME_CHANNEL` (1295) and `DISCORD_HOME_CHANNEL_THREAD_ID` (1310).
- `slack/mod.rs:25`: `DEFAULT_OWNER_ID = "U04M1JNJM8T"` is a hardcoded Slack owner fallback.
- `discord/adapter.rs:4237-4300`: cursor tables are named `discord_*`, and `update_bot_channel_cursor` only accepts numeric ids (4301).
- `discord/adapter.rs:611, 692, 737, 1605, 1663, 1741`: the shared debouncer and route helpers fall back to "discord:" delivery ids and take `PoiseData`.
- `discord/adapter.rs:1502, 2144`: `SessionKey` platform is the literal "discord".
- `main.rs:1528, 1538, 1544, 1552, 1573`: the debouncer flush only covers Discord `poise_data`.
- `main.rs:408-430`: recovery author provenance is Discord-only (`platform = 'discord'`, `discord_rest`). Slack guild sessions fall through to `legacy_guild_pending_auth`.
- `main.rs:674-678`: inbox digest owner is `OMON_OWNER_ID` or the Discord `OWNER_ID`. `inbox/digest.rs:20,40` requires `home.platform == "discord"` and pins the origin platform to Discord.
- `lanes/kibitzer.rs:124-129`: lead must be the Discord owner DM (user `414011632306618368`, bot `1465631383862120451`, channel `1474231559895646240`).
- `multiplexer/platform_router.rs:31-42`: any platform without a route falls back to the Discord dispatcher. Delivery works only because `main.rs:1187-1189` registers Slack.
- `discord/pairing.rs:70-90`: the pairing store is keyed by `u64`, so it can't hold Slack user IDs.
- `slack/commands.rs:135-137`: slash-command DM detection uses the `'D'` channel prefix while ingress uses `channel_type`. The two can disagree for `mpim` (group DMs).
- Delivery targets (outside this scope, already Slack-aware): `cron/store.rs:822` `slack_destination`, `cron/scheduler.rs:2357-2362`, `tools/cron.rs:314-320`. Discord-literal handling remains in `cron/scheduler.rs:413, 2404, 2561-2634` and `cron/store.rs:556-798`.
</answer>

<next_steps>
Suggested order:
1. Session-status interception and `referenced_message_id`. These are one-line wiring changes; the helpers already handle Slack.
2. Text-file inlining and audio transcription in `download_attachments`.
3. A debouncer plus per-session ordering. Generalize `SplitMessageDebouncer` off `PoiseData`.
4. Add `conversations.history` and `conversations.replies` to api.rs. That unlocks channel context, reply context and backfill.
5. A Slack cursor table with ts-aware comparison, then backfill and the actor cursor rewind.
6. Lifecycle events: add them to the manifest first.
7. Socket receive watchdog.
8. Config flags: allowed and ignored channels, allow_bots, thread_require_mention, context toggles.
</next_steps>
</results>
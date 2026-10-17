st_01a1474f [completed] model mahoquot/claude-opus-5-5
Ask: Build the Discord-to-Slack parity gap list for the lanes area. Wanted: one table covering every lane behavior, plus every place lanes are hardcoded to Discord. For you: the table is complete. Slack has almost nothing for lanes yet. `src/slack/*` never mentions lanes, kibitzer or owner replies (a grep for `lane|kibitzer|owner_repl|thread_owner|bind_lane` found 0 matches). Only the low-level Slack API calls and the routing of lead-session replies already work. Now: nothing; this was a read-only audit. Next: nothing; the report below is the deliverable.

<analysis>
**Literal Request**: Build a Discord-to-Slack parity table for `src/lanes/*`.
**Actual Need**: A concrete work list for making lanes work on Slack: where lanes are tied to Discord, and what Slack calls and files are needed.
**Success Looks Like**: Each behavior has a citation, a Slack status, the exact fix and the files to change.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/discord_threads.rs - the only lane transport (serenity): create, rename, archive, post, edit
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/posts.rs - durable post outbox and status message; `PostSink` impl for DiscordThreads
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/runner.rs - `LaneLifecycle` trait, `thread_effect` (close/reopen), choice of transport in `from_env`
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/cli.rs - `lane start --thread <u64>` creates the thread inline; orphan archive
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/guard.rs - queued-lane launch creates the thread; orphan archive
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/kibitzer.rs - hardcoded Discord owner home-DM lead and validator
- /Users/indo/code/project/omon-gateway-wt/slack/src/lanes/dispatch.rs, activity.rs, settle.rs - platform-agnostic (they go through the multiplexer or the outbox)
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/api.rs - Slack Web API calls available for reuse
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/ingress.rs - would need lane owner-reply interception
- /Users/indo/code/project/omon-gateway-wt/slack/src/storage/lanes.rs, storage/db.rs, discord/adapter.rs - owner-reply recording and thread-owner storage (Discord-only callers)
</files>

<answer>
## Key fact before the table

A lane "thread" is stored as a single numeric Discord channel id in `lanes.thread_channel_id`, and it is parsed with `parse::<u64>()` everywhere. A Slack thread is two strings: the channel id (like `C0123`) and a `thread_ts` (like `1700.0001`). Neither parses as a u64. So every Slack item below first needs a platform-tagged thread reference. My recommendation is to store `slack:<channel>:<thread_ts>` in the existing `thread_channel_id` text column, and change the transport traits from `u64` to `&str`. The traits that need this are `PostSink` and `LaneLifecycle`.

## Parity table

| Behavior | Discord impl (file:line) | Slack status | What Slack needs | Write-scope files |
|---|---|---|---|---|
| Lane transport handle and token loading | `discord_threads.rs:165-190` (`parse_bot_tokens`/`first_bot_token` read `DISCORD_BOT_TOKEN(S)`), `DiscordThreads::from_env` ~`:412` | PARTIAL: `SlackApi::new` at `slack/api.rs:41`, `SlackConfig::from_env` at `slack/mod.rs:46` | A new `SlackLaneThreads { api: SlackApi, bot_user_id }` built from `SlackConfig::from_env` + `auth_test` | new `src/lanes/slack_threads.rs`, `src/lanes/mod.rs` |
| Choosing the transport for the runner | `runner.rs:~985-1003` (only `OMON_DISCORD_ENABLED` or the Discord loopback sets `post_sink`/`lifecycle`) | MISSING | Choose per lane platform, or keep a router sink holding both Discord and Slack sinks and dispatch on the thread-ref prefix | `runner.rs`, new `lanes/slack_threads.rs` |
| Create lane thread (starter message `lane <key>: <title>`, then a thread on it) | `discord_threads.rs:210-239` (`send_message` + `create_thread_from_message`), metadata `:197-208` | MISSING (only the primitive exists: `post_message` at `api.rs:63` returns the ts) | `create_thread(parent_channel, key, title)`: `chat.postMessage` of the starter text to the parent channel. The returned `ts` becomes the thread root, giving thread ref = (channel, ts). No separate thread object exists. | `lanes/slack_threads.rs` |
| Thread creation in `lane start --thread` | `cli.rs:383-410` (`get_current_user`, `create_thread(ChannelId::new(parent))`), arg type `cli.rs:94` (`thread: Option<u64>`), check `cli.rs:~316` (`thread == Some(0)`) | MISSING | Change `--thread` to a string (Slack `C...` ids are not u64), or add `--slack-channel`. Use `auth.test` user_id as the bot id (`api.rs:58`) and call the Slack create. | `cli.rs`, `lanes/slack_threads.rs` |
| Choosing the lead from the thread session | `cli.rs:~345-348` (`sessions WHERE thread_id=? OR channel_id=?` bound with the u64 string) | PARTIAL: Slack sessions store `channel_id` = C id and `thread_id` = ts (`slack/mod.rs:3-6`) | Bind the Slack channel string; the query itself works once the arg is a string | `cli.rs` |
| Queued-lane launch creates the thread | `guard.rs:607-632` (`create_launch_thread`, `parse::<u64>` of parent, `get_current_user`), call at `guard.rs:701` | MISSING | Same Slack create, chosen from the `lane.parent_channel_id` format | `guard.rs`, `lanes/slack_threads.rs` |
| Archive orphan thread (cap re-check or insert failed) | `cli.rs:446-457`, `guard.rs:635-649` and `:732` | MISSING (Slack has no thread archive) | Best effort: `chat.delete` of the starter ts (`api.rs:96`) or `chat.update` to "lane not started". Recommend delete. | `cli.rs`, `guard.rs`, `lanes/slack_threads.rs` |
| Status marker in the title (🔄/⏸/✅, 100-char cap) | `discord_threads.rs:87-97` (`format_thread_name`), `set_status` `:249-279`, `set_status_with_title` `:281-291` | MISSING (Slack threads have no title) | Use `chat.update` (`api.rs:82`) on the starter message to rewrite `"<emoji> lane key: title"`. Optionally also add a status reaction (`reactions.add` at `api.rs:103`, e.g. `arrows_counterclockwise`/`double_vertical_bar`/`white_check_mark`). Keep the starter ts in the thread ref. | `lanes/slack_threads.rs` |
| `PostSink::rename_status` | `posts.rs:148-158` | MISSING | Implement it as the starter-message `chat.update` above | `lanes/slack_threads.rs`, `posts.rs` (trait signature: `thread: &str`) |
| Durable outbox post (milestone/stale reports into the lane thread) | `posts.rs:92-116` (`send_message` with nonce + `enforce_nonce`), enqueue `posts.rs:46-66`, producers `activity.rs:176`, `activity.rs:314` | MISSING (`chat.postMessage` + `thread_ts` exists at `api.rs:63`) | `PostSink::send` calls `chat.postMessage(channel, body, Some(thread_ts))` and returns the ts. Slack has no nonce/idempotency for bot posts, so the uncertain-state rule (no blind retry) stays the only guard. Convert the text with `slack::mrkdwn::to_mrkdwn` and chunk at `MESSAGE_LIMIT` (`slack/mod.rs:27`). | `lanes/slack_threads.rs` |
| Drain validates the destination as u64 | `posts.rs:~197-212` (`parse::<u64>().filter(!=0)`, otherwise marks the post Failed) | MISSING (Slack refs would always be marked Failed) | Replace with a platform-aware thread-ref parser | `posts.rs` |
| Status message (`⏳ title · Nm · turn N`): post once, then edit | `posts.rs:~330-336` (thread parse u64), `~395` (body), `~408-413` (`status_message_id` parse u64), edit `posts.rs:118-145` | MISSING (`chat.update` at `api.rs:82`) | `edit_status(thread, ts, body)` calls `chat.update`. Map `message_not_found` to `Ok(false)` (Discord maps 10008 at `posts.rs:139`). The error is surfaced as the string `"slack chat.update: message_not_found"` (`api.rs:330`). The message id must become a string ts, so remove `parse::<u64>` on `status_message_id`. | `posts.rs`, `lanes/slack_threads.rs` |
| Skip status refresh for closed lanes (Discord rejects edits to archived threads, 50083) | `posts.rs:278-282` | N/A for Slack (no archive). The rule is harmless. | none | none |
| No pings in lane posts | `discord_threads.rs:11-16` (`lane_allowed_mentions`), `posts.rs:104-109` | PARTIAL / unverified | Slack only pings on `<@U>`, `<!here>`, `<!channel>`. I did not check whether `to_mrkdwn` (`slack/mrkdwn.rs`) escapes `<`/`>`/`&` in plain text; `escape` exists only for approval blocks (`egress.rs:~395`). The lane sink must escape them. | `lanes/slack_threads.rs` |
| Plain post/edit helpers | `discord_threads.rs:326-347` | PARTIAL (primitives at `api.rs:63`, `:82`) | Wrappers in the Slack lane module | `lanes/slack_threads.rs` |
| Lifecycle bot identity (`LaneLifecycle::bot_id`) | `runner.rs:118-121`, `:137-141` (`get_current_user`) | PARTIAL: `auth_test().user_id` at `api.rs:58`; the runtime keeps `bot_user_id` (`slack/runtime.rs:21`) | `impl LaneLifecycle for SlackLaneThreads` returning `auth.test` user_id | `runner.rs` (trait takes u64), `lanes/slack_threads.rs` |
| Close: rename to ✅, then archive | `runner.rs:166-225` (`thread_effect`), called from `close_intent` `runner.rs:~851` | MISSING | `status`: starter-message `chat.update`. `archived(true)`: no Slack equivalent. Post a final "lane closed" reply in the thread and add a `white_check_mark` reaction on the starter. Make it a no-op success so close does not fail. | `runner.rs`, `lanes/slack_threads.rs` |
| Reopen: unarchive and rename to 🔄 | `runner.rs:47-64` (`reopen` hard-calls `DiscordThreads::from_env` at `:59`), `apply_owner_replies` `runner.rs:897-~948` (`thread_effect` at `~907`) | MISSING | `reopen` must choose the transport by thread-ref platform. On Slack, unarchive is a no-op plus starter `chat.update` to 🔄. | `runner.rs`, `lanes/slack_threads.rs` |
| Bot binding of the lane thread (only the creating bot may close/reopen) | `runner.rs:175-193` (`Database::get_thread_owner(pool, u64)`, `lane_thread_bindings`), `storage/db.rs:275-297` (`discord_thread_owners` table), `storage/lanes.rs:214-221` (`bind_lane_thread`) | MISSING (Slack never writes a thread owner or a binding) | On Slack create, call `bind_lane_thread(pool, key, bot_user_id)` directly. Skip `get_thread_owner` when the ref is not numeric (today non-numeric refs skip the lookup at `runner.rs:176`, but `invalid lane thread` at `:187-190` would then fail). Note: `cli.rs` stores `created_bot` but where it binds is past `:470`, which I did not read. | `runner.rs`, `cli.rs`, `guard.rs` |
| Owner replies in a closed/verified lane thread reopen or close it | Discord interception `discord/adapter.rs:1895-1921` (`get_lane_by_thread` then `record_owner_reply`), storage `storage/lanes.rs:223-263` | MISSING | In `SlackIngress::handle` (`slack/ingress.rs:42`), before routing: if the event has `thread_ts`, look up `get_lane_by_thread(pool, "slack:<channel>:<thread_ts>")` and call `record_owner_reply(pool, ref, bot_user_id, user, config.owner_id(), ts, text)` (owner from `slack/mod.rs:73`). If it returns true, consume the event and call `lanes::runner::wake`. Also, `map_event` drops thread replies that have no mention unless the thread is a known session (`ingress.rs:230-233`). Lane threads have no session, so interception must happen before `map_event`, or the lane thread must count as known. | `slack/ingress.rs`, `storage/lanes.rs` (none if the ref string matches) |
| Local CLI reopen (owner reply `local-cli`) | `cli.rs:233-244` | DONE (platform-agnostic; the runner applies it, but that needs the Slack lifecycle above) | none beyond lifecycle | none |
| Lane-agent output capture (no transport) | `runner.rs:604-694` (`CapturingDispatcher`, `platform == "lane"`) | DONE (platform-agnostic) | none | none |
| Lane events to the lead session turn | `dispatch.rs:80-201` (`multiplexer.route` to the lead `SessionKey`) | DONE if the lead is a Slack session: replies go through `PlatformRouter` (`multiplexer/platform_router.rs:59-63`) to `SlackEgress` (`slack/egress.rs:276`) | Only the lead key must be a Slack one (see kibitzer/default lead) | none |
| Activity / stale / settlement | `activity.rs` (only the outbox enqueue at `:176`, `:314`), `settle.rs` (no transport; `"lane"` key at `:779`) | DONE (agnostic) | none | none |
| Kibitzer lead = verified owner home DM | `kibitzer.rs:21-22` (`DEFAULT_LEAD` Discord DM), validator `kibitzer.rs:124-134` (requires `platform=="discord"`, user `414011632306618368`, bot `1465631383862120451`, channel `1474231559895646240`) | MISSING (any Slack lead is rejected; `from_env` at `:157-159` errors) | Accept a Slack owner DM: `platform=="slack"`, `guild_id==None`, `thread_id==None`, `user_id==slack::owner_id()` (`slack/mod.rs:80`), `bot_id==Some(auth.test user_id)`, channel = a `D...` id from `open_im(owner)` (`api.rs:133`). Recommend replacing the hardcoded ids with env-driven owner checks for each platform. | `kibitzer.rs`, `main.rs` (`:1705-1710` test constructor) |
| Kibitzer wake delivery to the owner | `kibitzer.rs:~343-347` inserts a `kibitzer` lane_event; delivered by `dispatch.rs` | DONE once the lead is Slack (routes through `PlatformRouter`) | none | none |
| Kibitzer arming on owner DM messages | `multiplexer/actor.rs:1178-1190` (`is_kibitzer_lead` compares against `DEFAULT_LEAD`/env) | PARTIAL: works for Slack only if `OMON_LANE_DEFAULT_LEAD` is set to a Slack key, and then the validator above rejects it | Fix the validator | `kibitzer.rs` |
| Default lead for lanes, cron, inbox, proposals | `main.rs:697-702`, `cron/health.rs:294-298`, `inbox/proposals.rs:197` all fall back to Discord `DEFAULT_LEAD` | PARTIAL (overridable with `OMON_LANE_DEFAULT_LEAD`) | Optional: a Slack default derived from `SLACK_ALLOWED_USERS[0]` + `open_im` | `kibitzer.rs` (constant), `main.rs` |
| Rate-limit/hang bounds on lane calls | `posts.rs:27-29` (30s), `runner.rs:~792` (`LIFECYCLE_TIMEOUT` 15s), `guard.rs:591` (30s) | DONE: `SlackApi` retries 429 up to 3 times with Retry-After capped at 30s (`api.rs:283-311`); the existing timeouts wrap any sink | none | none |
| QA loopback sink | `discord_threads.rs:~360-406` (`OMON_DISCORD_API_BASE`) | PARTIAL: `SlackApi::with_base_url` (`api.rs:45`) exists, but no env opt-in | Optional `OMON_SLACK_API_BASE` loopback for lanes QA | `lanes/slack_threads.rs` |

## Hardcoded Discord coupling

- `kibitzer.rs:21-22`: `DEFAULT_LEAD = "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451"`.
- `kibitzer.rs:124-129`: `platform != "discord"`, user `414011632306618368`, bot `1465631383862120451`, channel `1474231559895646240`.
- Consumers of the Discord `DEFAULT_LEAD`: `main.rs:701`, `main.rs:1707`, `cron/health.rs:298`, `inbox/proposals.rs:197`, `multiplexer/actor.rs:1180`.
- `runner.rs:59`: `DiscordThreads::from_env()` in `reopen`.
- `runner.rs:135-163`: `impl LaneLifecycle for DiscordThreads` with `serenity::ChannelId::new`.
- `runner.rs:118-133`: `LaneLifecycle` uses `u64` threads and returns `DiscordThreadError`.
- `runner.rs:176`, `:187-190`: thread ref parsed as u64 and `get_thread_owner` (`discord_thread_owners`).
- `runner.rs:~985-1003`: `OMON_DISCORD_ENABLED` and Discord-only transport selection.
- `posts.rs:5`, `:8`: serenity and DiscordThreads imports.
- `posts.rs:70-90`: `PostSink` uses u64 thread/message ids and the `discord_threads::ThreadStatus` type.
- `posts.rs:92-158`: Discord `PostSink` (nonce, `enforce_nonce`, error code 10008).
- `posts.rs:~199`, `~330`, `~408`: u64 parsing of thread and status message ids.
- `cli.rs:94`: `--thread: Option<u64>`.
- `cli.rs:383-410`: serenity create and `get_current_user`.
- `cli.rs:446-457`: archive with `serenity::ChannelId`.
- `guard.rs:591`, `:607-649`: Discord launch thread and orphan archive.
- `guard.rs:731`: u64 parse.
- `discord_threads.rs` (whole module): serenity-only, `DISCORD_BOT_TOKEN(S)`, `DISCORD_THREAD_NAME_MAX_CHARS`, `DISCORD_HOME_CHANNEL` (live test only, `:~640`).
- `storage/db.rs:275-297`: `discord_thread_owners` table, keyed on u64.
- `discord/adapter.rs:1895-1921`: the only caller of `record_owner_reply`.
</answer>

<next_steps>
1. Decide the Slack thread-ref encoding. I recommend storing `slack:<C>:<ts>` in `thread_channel_id` and changing `PostSink` and `LaneLifecycle` to take `&str`. Every MISSING row depends on this.
2. Add `src/lanes/slack_threads.rs` implementing `PostSink` and `LaneLifecycle` over `SlackApi`:
   - starter `chat.postMessage` to create a thread
   - starter `chat.update` (plus a reaction) for status
   - a no-op archive with a final post
   - `chat.update` for status edits, with `message_not_found` mapped to `Ok(false)`
3. Make transport selection platform-aware in `runner.rs` (`from_env`, `reopen`), `cli.rs` and `guard.rs`.
4. Add owner-reply interception to `slack/ingress.rs` before `map_event`.
5. Generalize the kibitzer lead validator in `kibitzer.rs:124-134`.
6. Two things I did not check: the end of `cli.rs` after line 470 (where `created_bot` gets bound), and whether `slack/mrkdwn.rs::to_mrkdwn` escapes `<!channel>`-style text.
</next_steps>
</results>
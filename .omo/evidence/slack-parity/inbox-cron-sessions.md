st_01a14750 [completed] model mahoquot/claude-opus-5-5
I read this as a read-only parity audit of inbox, cron, sessions and the related wiring. I read all of `src/slack/*`, `platform_router.rs` and every in-scope source, then built the table. I stop when the table covers the whole scope.

<analysis>
**Literal Request**: For every behavior in the inbox, cron, sessions, message_context, main, usage, readiness, dashboard and voice areas, compare what Discord does with what Slack has.
**Actual Need**: A concrete work list for Slack parity, with file:line citations, the Slack API calls and the files to change.
**Success Looks Like**: A behavior table plus a list of hardcoded Discord coupling.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/mod.rs - Slack config and owner: `SLACK_ALLOWED_USERS`, `DEFAULT_OWNER_ID` (:25, :85). There is no home channel setting.
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/api.rs - Web API client. `open_im` (:134), `post_message` (:64), `upload_file` (:144), `conversations_info` / `users_info` (:122/:129). There is no `conversations.history` or `conversations.replies`.
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/ingress.rs - Incoming messages. It has no status intercept, no proposal handling and no audio transcription.
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/commands.rs - `/omo-sessions` (:836-972) and `/omo-cron list` (:706).
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/egress.rs - Outgoing SendMessage, approval and stream handling.
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/platform_router.rs - Routes by `session.platform`. Anything without a route goes to Discord.
- /Users/indo/code/project/omon-gateway-wt/slack/src/inbox/{digest,proposals,triage,watchers,prefilter,mod}.rs and sources/discord.rs
- /Users/indo/code/project/omon-gateway-wt/slack/src/cron/{scheduler,store,executor,remediate,report,health}.rs
- /Users/indo/code/project/omon-gateway-wt/slack/src/sessions/{status_reply,watch,overview}.rs
- /Users/indo/code/project/omon-gateway-wt/slack/src/tools/{message_context,message_context_lazy,mod}.rs
- /Users/indo/code/project/omon-gateway-wt/slack/src/{main,usage,readiness,dashboard,dashboard_runtime,voice_transcription}.rs, src/voice/*
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/actor.rs - Owner-lead check (:1250-1330) and message search indexing (:1163).
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/adapter.rs - The Discord side of the proposal flow (:899-1017) and the status intercept (:1106-1137).
- /Users/indo/code/project/omon-gateway-wt/slack/migrations/0034_inbox.sql - The source CHECK constraint (:3, :37).
</files>

<answer>
## Parity table

| # | Behavior | Discord impl (file:line) | Slack status | What Slack needs | Write-scope files |
|---|---|---|---|---|---|
| 1 | **Inbox digest job spec** (09:00 KST, sent only to the owner's DM, `attach_to_session:false`) | inbox/digest.rs:19-46. `spec()` rejects anything where `home.platform != "discord"` (:20) and hardcodes `"origin":{"platform":"discord"}` (:40) | MISSING | `spec()` should accept `platform=="slack"` when `guild_id None`, `thread_id None` and `user_id==owner`, and copy `home.platform` into `origin.platform`. Delivery already works through the scheduler (row 9) and Slack egress (`chat.postMessage`). | src/inbox/digest.rs |
| 2 | **Digest executor guard** (origin must stay the owner DM) | inbox/digest.rs:128-147. It recomputes `spec(&home, &home.user_id)` | MISSING (follows from row 1) | Fixed by row 1. No separate change. | src/inbox/digest.rs |
| 3 | **Digest registration at boot** | main.rs:1279-1297. The query requires `platform='discord'` and `session_key=default_lead()` with the Discord owner (`owner_id_from(OMON_OWNER_ID)` at :675) | MISSING | Add a fallback. If the Discord home DM is absent and Slack is configured, look up the Slack DM session (`platform='slack'`, `guild_id IS NULL`, `user_id=slack::owner_id()`), or build its key with `api.open_im(owner)`, `SessionKey::new("slack",None,D…,None,owner).with_bot_id(bot_user_id)` and an INSERT. Then call `digest::register(..., &slack_owner)`. This needs `bot_user_id` from `slack::runtime::build`, which currently runs after the scheduler (:1486), so the order must change. | src/main.rs |
| 4 | **Digest content** (owner tasks, proposals, usage, cron, auth errors) | inbox/digest.rs:68-120 | DONE (no platform dependency) | `##` headings are converted by `slack/mrkdwn.rs:40` (test :379). | — |
| 5 | **Proposal owner identity** | inbox/proposals.rs:10 `OWNER_ID="414011632306618368"`. Used at triage.rs:357/:446, list.rs:46, set.rs:32-59, main.rs:1311 | PARTIAL | `owner_id` is used as a logical owner key in the DB, so keeping the Discord constant works. The Slack handler (row 6) must check the author against `slack::owner_id()` and then pass `OWNER_ID` as `Reply.author_id`, which is what Discord does at adapter.rs:973. Optional: make the owner configurable. | src/inbox/proposals.rs (optional) |
| 6 | **Owner DM proposal flow**: "제안" lists pending proposals in chunks and binds each one with `presented()`; a reply "<n> 해" calls `approve()` and the result is sent back to the DM | discord/adapter.rs:899-1017 (pending :912, presented :953, approve :968, footer :915, chunking with owner_dm_chunks :1003) | MISSING | In `SlackIngress::handle` (ingress.rs:43, before the steer at :81): if `triage::enabled()`, the channel is a DM and `user == config.owner_id()`, handle "제안" by posting each group with `api.post_message(channel, text, None, None)` and calling `proposals.presented(n, channel, ts)`. For approval, use the Slack thread as the reply reference: `referenced_message_id = event.thread_ts`. The footer must say "reply in the thread". This needs an `inbox_actions: Option<Arc<GatewayActions>>` dependency in `SlackIngress` / `SlackDeps`. Note that `map_event` folds `thread_ts` into the session key; the raw `thread_ts` is still needed. | src/slack/ingress.rs, src/slack/runtime.rs, src/main.rs |
| 7 | **Triage proposals bound to the lead channel** | inbox/triage.rs:393-401 uses `SessionKey::from_storage_key(&self.lead).channel_id` | PARTIAL | This works only when `OMON_LANE_DEFAULT_LEAD` is set to the Slack DM key. The default is a Discord key (lanes/kibitzer.rs:21-22). Needs a Slack-aware default lead (see row 23). | src/main.rs (`default_lead` :699) |
| 8 | **Inbox source for chat messages** (owner DMs, mentions and replies to the owner become inbox items) | inbox/sources/discord.rs:15-117: `platform='discord'` SQL (:47, :55, :66), owner `<@id>` mentions, `discord.com/channels` link (:102). Registered at main.rs:2394. `InboxSourceKind::Discord` at inbox/mod.rs:46 | MISSING | Slack messages are already indexed into `message_search_documents` (actor.rs:1163, ingress.rs:150). Add `inbox/sources/slack.rs` (a `SlackSource` with `platform='slack'`, owner `<@Uxxx>` mention, DM = `guild_id IS NULL`, link `https://slack.com/app_redirect?channel=C&message_ts=ts`, or `chat.getPermalink` if added to api.rs). Add `InboxSourceKind::Slack` and a migration that extends the CHECK at migrations/0034_inbox.sql:3,:37 and 0041:4. Register it in `inbox_sources()` at main.rs:2389. Ordering caveat: the cursor `message_id > ?` works for Slack ts only while ts strings keep the same length. | src/inbox/sources/{mod,slack}.rs, src/inbox/mod.rs, new migration, src/main.rs |
| 9 | **Gateway message event triggers the chat source** | inbox/watchers.rs:71-75 filters `GatewayMessage` to `InboxSourceKind::Discord` only | PARTIAL | Slack already sends `on_gateway_message` (ingress.rs:153, actor.rs:1169), but this only re-collects the Discord source. Add `Slack` to the filter. | src/inbox/watchers.rs |
| 10 | **Prefilter rules for chat** (exclude owner and bot; require a mention, DM or owner reply) | inbox/prefilter.rs:19-20, :30 (hardcoded owner id), :140-162 | MISSING | Add `owner_slack_id` / `bot_slack_id` fields and a `InboxSourceKind::Slack` arm with the same logic using `<@U…>`. | src/inbox/prefilter.rs |
| 11 | **Cron delivery to a channel/thread** (`deliver: slack:C…[:ts]`) | cron/scheduler.rs:2350-2463 deliver, :2357-2361 platform select | DONE (scheduler.rs:2357, store.rs:648 + :822 `slack_destination`, scheduler.rs:2586) | — | — |
| 12 | **Cron `deliver: origin` with a Slack origin** | cron/store.rs:724-742, scheduler.rs:2593-2609 | DONE (store.rs:729-731, scheduler.rs:2598-2600) | — | — |
| 13 | **Cron home-channel fallback** (`origin`/`discord` with no origin uses the configured home) | cron/store.rs:520-570 `configured_discord_home` (keys `home_channel`, `discord_home_channel`, …), used at :703-721 and :745-762, always as `platform:"discord"` | MISSING | Add `configured_slack_home()` (extra keys such as `slack_home_channel` or a `slack` object, or a `SLACK_HOME_CHANNEL` env var) and a fallback branch that emits a `platform:"slack"` origin. Also accept a bare `"slack"` deliver target, mirroring `"discord"` at :726. | src/cron/store.rs, src/slack/mod.rs (env) |
| 14 | **Cron failure alerts** | cron/scheduler.rs:2311-2328 using `failure_destinations()` (store.rs:571-579) | DONE (resolves through the same parser, so `slack:` works) | Only gaps: Slack-only home fallback (row 13). | — |
| 15 | **Cron agent session key** (agent runs with the destination context) | cron/executor.rs:286-300 hardcodes `SessionKey::new("discord", …)` for **any** destination, including Slack | PARTIAL (bug) | Use `target.platform.to_ascii_lowercase()`. | src/cron/executor.rs |
| 16 | **Payload (non-Hermes) cron executor destination** | cron/executor.rs:585-595 only strips `discord:` | MISSING | Also parse `slack:` using `store::slack_destination` and set platform `"slack"`. | src/cron/executor.rs |
| 17 | **Missing-transport guard** (fail when no dispatcher exists) | cron/scheduler.rs:2201-2214 `has_explicit_discord_destination` (store.rs:600) | MISSING | A Slack job with no dispatcher reports success (dispatch is skipped at scheduler.rs:2421-2430). Add `has_explicit_slack_destination` or generalize the check. | src/cron/store.rs, src/cron/scheduler.rs |
| 18 | **CronNotification broadcast** | cron/scheduler.rs:2362-2374, Discord only (`u64` channel) | N/A | The only `subscribe` is the definition (scheduler.rs:734); grep found no consumer. Nothing needed unless a consumer is added. | — |
| 19 | **Cron profile and `context_from` session pattern** | cron/scheduler.rs:413 (`parts[0]=="discord"`), :169 `discord:{profile}:cron:` | PARTIAL (minor) | Slack session keys give an empty profile. The `%:cron:<id>` LIKE fallback still matches. Accept `slack` at :413 if profiles matter. | src/cron/scheduler.rs |
| 20 | **`last_delivery_error` classification** | cron/scheduler.rs:1908-1913 (`deliver` / `discord` substring) | DONE in practice | Slack errors come wrapped as "delivery failed for…" (scheduler.rs:2268), which already matches `deliver`. | — |
| 21 | **Remediation owner escalation and approval token** | cron/remediate.rs:349-399 `needs_owner` → `queue_outcome` (report.rs:193); approval through the CLI `cron approve-rerun --owner-token` | PARTIAL | The token reaches the owner only through the digest (report.rs:207 `digest_section`, acknowledged at :230). On Slack this is blocked by rows 1-3. Approval itself is CLI-based and has no platform dependency. | (rows 1-3) |
| 22 | **Cron health problem → lead event** | cron/health.rs:157-166, :288-305 (`OMON_LANE_DEFAULT_LEAD` or DEFAULT_LEAD) | PARTIAL | Follows the lead key (row 23). | — |
| 23 | **Default lead / owner home DM** (kibitzer, triage, session watch, health) | main.rs:699-702 `default_lead()`; lanes/kibitzer.rs:21-22 Discord storage key; actor.rs:1176-1182 `is_kibitzer_lead` | PARTIAL | Everything works if `OMON_LANE_DEFAULT_LEAD` is set to the Slack DM storage key. There is no automatic Slack default. Add a Slack-derived default: when Discord is disabled or unset, use `open_im(owner)` plus the bot id. | src/main.rs, possibly src/lanes/kibitzer.rs |
| 24 | **Owner lead-session detection** | multiplexer/actor.rs:1250-1330 (Discord owner, `DISCORD_HOME_CHANNEL`, `_THREAD_ID`) | DONE for DMs (actor.rs:1251-1253) / MISSING for home channel | Slack has no home-channel or thread equivalent. Add `SLACK_HOME_CHANNEL` and check it in the Slack branch. | src/multiplexer/actor.rs, src/slack/mod.rs |
| 25 | **Session watch notices to the owner DM** (question, blocked, completion) | sessions/watch.rs:296-347 `serve(dispatcher, owner_dm)`; main.rs:1438-1462 uses `default_lead()` | PARTIAL | Delivery goes through PlatformRouter to Slack egress when the lead key is Slack (row 23). The hint text at watch.rs:162 uses Discord syntax (`/sessions answer target:… text:…`); Slack needs `/omo-sessions answer <target> <text>`. Pick the text by `owner_dm.platform`. | src/sessions/watch.rs |
| 26 | **`/sessions` overview, read, send, stop, answer, watch, unwatch** | discord/session_commands.rs | DONE (slack/commands.rs:774-972, owner gate :580-584) | — | — |
| 27 | **"세션 상태" natural-language status intercept in the owner DM** | discord/adapter.rs:1106-1137 → router.rs:446 `intercept_session_status`; `status_reply::owner_dm` | MISSING (helper exists, not wired) | The `owner_dm` Slack branch exists (status_reply.rs:50-53) and so does `dispatch_direct_reply` (platform_router.rs:95), but `SlackIngress::handle` never calls the intercept. After dedupe in ingress.rs:~64, call `multiplexer.intercept_session_status(&inbound, status_reply::owner_dm(&inbound.session, user, ""))` and return early on `true`. | src/slack/ingress.rs |
| 28 | **Status intercept in an owner private home thread** | status_reply.rs:67-76 `owner_thread` (Discord + `DISCORD_HOME_CHANNEL_THREAD_ID`); egress check adapter.rs:3515 | MISSING | Optional. Slack threads are never private, so this only fits a private Slack home channel (`conversations_info` → `is_private`). It would need an `owner_thread_is_private` impl in SlackEgress. | src/sessions/status_reply.rs, src/slack/egress.rs |
| 29 | **Status reply length** | status_reply.rs:91 `DISCORD_LIMIT` 1900 (overview.rs:10) | DONE (works; it is just shorter than needed) | Slack's command path uses `MESSAGE_LIMIT` (commands.rs:846). The intercept could also use it. | — |
| 30 | **`owner_id()` for status and commands** | status_reply.rs:36-47 (`DISCORD_ALLOWED_USERS` or the Discord constant) | DONE for Slack via `slack::owner_id()` (slack/mod.rs:85) | — | — |
| 31 | **message_context tool** (history, search, replies) | tools/message_context.rs:252-262, :271-284; message_context_lazy.rs:46-47 `"discord"`; registered in tools/mod.rs:78; `validate_discord_id` :392 | MISSING | A Slack session errors with "no provider for platform 'slack'" (:306-310). Add `SlackMessageContextProvider` (`platform()="slack"`) backed by `conversations.history` and `conversations.replies` (new methods in slack/api.rs) plus local FTS (`MessageSearchIndex::search("slack",…)`; Slack messages are already indexed). Apply Slack id validation (`[CDG][A-Z0-9]+`) and register it in `ToolRegistry::new` when `SlackConfig` is present. | src/tools/message_context_slack.rs (new), src/tools/mod.rs, src/slack/api.rs |
| 32 | **Usage purpose attribution** | usage.rs:222 | DONE (usage.rs:222 includes `slack::PLATFORM`) | — | — |
| 33 | **FDA-missing DM alert to the owner** | main.rs:791-856 `notify_owner_fda_denied_if_needed` (serenity `create_private_channel`), called at :1157-1162 | MISSING | Add a Slack variant: `api.open_im(slack_owner)` then `api.post_message(D…, msg, None, None)`, using the same `.fda_dm_prompted` identity marker. Call it after the SlackConfig build (:1178). | src/main.rs |
| 34 | **Startup requires `DISCORD_BOT_TOKEN`** | main.rs:149-152 (error when there are no tokens); :1146-1147 requires a default bot id; the `OMON_DISCORD_ENABLED=false` headless path (:904-1026) never builds Slack | MISSING | A Slack-only deploy is impossible. Make the token optional when `SlackConfig::from_env` is `Some`, skip the Discord client and egress (PlatformRouter needs a fallback, so register Slack as the fallback), or start the Slack runtime inside the headless branch. | src/main.rs, src/multiplexer/platform_router.rs (fallback optional) |
| 35 | **Readiness gateway probe** | readiness.rs:253-258 `probe_gateway(bot_count)`; main.rs:1372-1376 passes `clients.len()` (Discord only) and runs before Slack starts | MISSING | Count Slack (`+ slack_config.is_some()`) or add a `"slack"` check that uses the `auth_test` result. | src/main.rs, src/readiness.rs |
| 36 | **Dashboard config view** | dashboard.rs:687-697 `"discord"` block; dashboard_runtime.rs:278-341 `configured_bot_count` (Discord env only) and the `"discord"` block | MISSING | Add a `"slack"` block: configured, `allowed_users`, `free_response_channels`, `auto_thread` (no tokens). | src/dashboard.rs, src/dashboard_runtime.rs |
| 37 | **Dashboard message-context policy store** | dashboard_runtime.rs:206-207 `"discord"` only | MISSING (follows row 31) | Add `"slack"` once the provider exists. | src/dashboard_runtime.rs |
| 38 | **Dashboard cron trigger guard** (detached dashboard) | dashboard.rs:1789-1801 | DONE (uses generic `delivery_destinations`, which includes Slack) | — | — |
| 39 | **Dashboard web status intercept** | dashboard.rs:1271-1278, :1383-1388 | N/A (web keys only) | — | — |
| 40 | **Audio attachment transcription** | discord/adapter.rs:1688-1702 (`voice_transcription::is_audio_attachment` / `transcribe_audio`) | MISSING | After `download_attachments` (ingress.rs:99), for each audio attachment with a `local_path`, call `transcribe_audio` and set `text_content`. Slack audio clips are `file_share` with an audio mimetype. | src/slack/ingress.rs |
| 41 | **Voice channel pipeline** (songbird) | voice/mod.rs:11, voice/pipeline.rs | N/A | Slack huddles have no bot audio API. | — |
| 42 | **Free-response home channel** | main.rs:96-104 merges `DISCORD_HOME_CHANNEL` into free-response | PARTIAL | Slack has `SLACK_FREE_RESPONSE_CHANNELS` (slack/mod.rs:66) but no home-channel concept (rows 13, 24). | src/slack/mod.rs |

## Hardcoded Discord coupling
- inbox/proposals.rs:10 `OWNER_ID = "414011632306618368"`; inbox/sources/discord.rs:19 (same id); inbox/prefilter.rs:30 (same id); actor.rs:1287 (same id)
- inbox/sources/discord.rs:20 `DISCORD_BOT_ID`; :47/:55/:66 `platform='discord'`; :102 `discord.com/channels`
- inbox/digest.rs:20 `home.platform != "discord"`; :40 `"platform": "discord"`
- inbox/watchers.rs:74 GatewayMessage → `InboxSourceKind::Discord` only
- inbox/mod.rs:46,55 enum has no Slack; migrations/0034_inbox.sql:3,37 and 0041_cron_remediation.sql:4 CHECK constraints
- inbox/proposals.rs:169 `parent_channel_id: Option<u64>` (Discord snowflake)
- lanes/kibitzer.rs:21-22 `DEFAULT_LEAD = "7:discord|…|414011632306618368|…"` (consumed by main.rs:699, cron/health.rs:298, proposals.rs:197, actor.rs:1177)
- main.rs:96 `DISCORD_HOME_CHANNEL`; :149-152 required `DISCORD_BOT_TOKEN`; :674-679 owner = Discord id; :791-856 FDA alert through serenity; :1283-1284 `platform='discord'` home-DM query; :1372-1376 readiness counts Discord clients only; :2394 DiscordSource only
- cron/store.rs:520-570 `configured_discord_home`; :600 `has_explicit_discord_destination`; :703-721, :745-762, :766-806 bare/numeric targets → `"discord"`
- cron/scheduler.rs:169 `discord:{profile}:cron:`; :413 profile only from `discord`; :2202-2213 Discord-only transport guard; :2362 CronNotification Discord-only; :2556-2566 `channel_id` → discord
- cron/executor.rs:286-291 session platform `"discord"` for every destination; :588-591 `discord:` only
- sessions/status_reply.rs:37 `DISCORD_ALLOWED_USERS`; :56 `platform=="discord"`; :69-72 `DISCORD_HOME_CHANNEL_THREAD_ID`
- sessions/watch.rs:162 Discord slash syntax `/sessions answer target:… text:…`
- sessions/overview.rs:10 `DISCORD_LIMIT` (name only)
- multiplexer/actor.rs:1280-1330 `DISCORD_ALLOWED_USERS`, `DISCORD_HOME_CHANNEL`, `DISCORD_HOME_CHANNEL_THREAD_ID`
- tools/message_context.rs:257, :271, :284, :358-359, :392 `validate_discord_id`; message_context_lazy.rs:47; tools/mod.rs:78
- dashboard.rs:687; dashboard_runtime.rs:206-207, :280, :331-341
- multiplexer/platform_router.rs:20-24, test :137: an unrouted platform falls back to Discord egress
</answer>

<next_steps>
Suggested order:
1. Rows 27 and 40: wire the existing helpers into `src/slack/ingress.rs`. The code already exists, so these are the smallest fixes.
2. Rows 15 and 16: fix the cron executor platform in `src/cron/executor.rs`. Row 15 is an actual bug today.
3. Rows 1-3 and 23: Slack owner DM and default lead in `digest.rs` and `main.rs`. Remediation escalation (row 21) depends on these.
4. Row 6: Slack proposal flow in ingress, using the thread ts as the reply reference.
5. Rows 8-10: `SlackSource`, the prefilter arm, the watcher filter and a migration.
6. Row 31: Slack message_context provider, which needs `conversations.history` and `conversations.replies` in `api.rs`.
7. Rows 33-37: startup without Discord, the Slack FDA alert, readiness and the dashboard.

Unverified: I found no consumer of `CronNotification`, but that rests on one grep. I did not check whether the Slack ts cursor ordering in row 8 holds over time.
</next_steps>
</results>
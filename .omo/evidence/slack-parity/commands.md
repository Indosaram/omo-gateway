st_01a1474e [completed] model mahoquot/claude-opus-5-5
Ask: build an exhaustive Discord-to-Slack parity gap list for the commands area. Wanted: a table of every behavior, plus a list of hardcoded Discord coupling. For you: the table covers all four scope files. Slack already does 17 of 19 Discord commands. The real gaps are `/pair` and pairing admission, channel and role admission, `destructive_slash_confirm` not being passed through, thread creation and title behavior, the pending-write payload format, and Discord-only owner, home channel and cron delivery coupling. Now: done. Next: none.

I read every file under `src/slack/`, `platform_router.rs`, the four Discord files in scope, plus the Slack wiring in `src/main.rs`, `sessions/status_reply.rs`, `tools/cron.rs` and the manifest. I made no edits and ran no cargo.

<results>
<files>
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/commands.rs - the Slack slash-command parser and executor (`/omo-*`)
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/mod.rs - `SlackConfig`, `owner_id`, `is_allowed`, `DEFAULT_OWNER_ID`
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/interactive.rs - approval button clicks
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/egress.rs - approval blocks and expiry
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/ingress.rs - message admission (no pairing) and the text `/stop`
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/api.rs - available Slack Web API wrappers
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/runtime.rs - dispatches slash commands and interactive payloads
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/platform_router.rs - routes approvals and expiry per platform
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/commands.rs - all Discord slash commands, admission, pair
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/session_commands.rs - `/sessions` subcommands, owner gate
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/pairing.rs - `PairingStore` (u64 ids, "discord" lockout key)
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/approval.rs - `SmartApprovalGuard`, `resolve_session_deny`, button ids
- /Users/indo/code/project/omon-gateway-wt/slack/src/main.rs - Slack deps wiring (1487-1500), home DM query (1283)
- /Users/indo/code/project/omon-gateway-wt/slack/src/sessions/status_reply.rs - `owner_id()` reads `DISCORD_ALLOWED_USERS`
- /Users/indo/code/project/omon-gateway-wt/slack/src/tools/cron.rs - cron delivery target is filled in only for discord
- /Users/indo/code/project/omon-gateway-wt/slack/docs/slack-app-manifest.json - 18 commands registered, bot scopes
</files>

<answer>
Discord paths below are relative to `src/discord/` (`commands.rs` = C, `session_commands.rs` = SC, `pairing.rs` = P, `approval.rs` = A). Slack paths are relative to `src/slack/` (`commands.rs` = S, plus `interactive.rs`, `egress.rs`, `ingress.rs`, `mod.rs`).

| Behavior | Discord impl | Slack status | What Slack needs | Write-scope |
|---|---|---|---|---|
| Command registry | C:178-200 `all()`, 19 commands | DONE: S:32-53 `COMMANDS` (18), manifest; Slack-only `Help` at S:450 | Add `pair` once pairing exists | S, manifest |
| User admission: allowed users | C:207-226, C:356-422 `command_check` | PARTIAL: S:428 checks only `config.is_allowed` | Paired users, allowed roles and allow-all are missing. Slack has no roles; add `SLACK_ALLOW_ALL_USERS` and paired-user lookup to `SlackConfig::is_allowed` or `SlackCommands::handle` | `slack/mod.rs`, S |
| Channel allow/ignore lists | C:242-276, C:330-345, reply "This command cannot be used in this channel." | MISSING | Add `allowed_channels`/`ignored_channels` to `SlackConfig::from_env` (`SLACK_ALLOWED_CHANNELS`, `SLACK_IGNORED_CHANNELS`). Check them in `handle` with the same ephemeral text; DMs (`D…`) are exempt | `slack/mod.rs`, S, ingress.rs |
| Missing-guild-metadata rejection | C:318-320, C:414-419 | Not applicable: Slack payloads always carry `channel_id` | none | - |
| Unauthorized reply is ephemeral | C:390-397 | DONE: S:28, S:429 (`Reply::private`) | - | - |
| `/model <name>` public, `ensure_session` | C:798-810, C:787-796 | DONE: S:451-456 (reuses `execute_model_command`) | - | - |
| `/reset [confirm]` gated by `destructive_slash_confirm` | C:816-833, C:864-880 | PARTIAL: S:457-469. `SlackCommands::new` builds `PoiseData::new` (S:405), which defaults `destructive_slash_confirm=true` (C:97). The configured value is never copied (main.rs:1305 sets it only on Discord `poise_data`) | Add `destructive_slash_confirm` to `SlackCommandDeps` (S:58), set `poise.destructive_slash_confirm` in `new`, pass it from main.rs:1490 | S, main.rs |
| `/stop` ephemeral; interrupts and clears YOLO | C:883-915, C:917-940 | DONE: S:470-484 (`stop_session`) | - | - |
| Text `/stop` in a message | (adapter, out of scope) | PARTIAL: ingress.rs:68-72 calls only `multiplexer.stop`, so YOLO is not cleared and nothing is acknowledged | Call `discord::commands::stop_session` there | ingress.rs |
| `/status` public stats | C:942-955 | DONE: S:485-494 | - | - |
| `/tools` public | C:957-972 | DONE: S:495-510 | - | - |
| `/skill <action>` (action required), `/skills` default list | C:513-533 | DONE: S:225-232 parse | - | - |
| Skill list, search, read | C:563-647 | DONE: S:613-668 (mrkdwn `*bold*`; usage hint changed to `/omo-skill read`) | - | - |
| Skill run | C:648-660 (posts "Injecting" first, then routes, and posts an error separately) | DONE with an ordering difference: S:670-676 routes first, then sends one reply | Cosmetic only | - |
| Skill pending-write operator gate | C:435-437, C:548-558 ("Only operators…") | DONE implicitly: every admitted Slack user is in `allowed_users` (S:428) | Once pairing or allow-all is added, add a `can_manage_skill_writes` check in `skill()` S:607 | S |
| Pending-write actions are ephemeral | C:426-433, C:541-545, C:687-691 | DONE: `pending_reply` always uses `Reply::private` (S:1014+) | - | - |
| Pending-write payload delivery | C:500-510 sends `pending-writes.json` as an attachment | PARTIAL: JSON is inlined in a code fence and chunked over 5+ ephemeral posts. Large payloads get split mid-fence | Optional: DM the file via `api.open_im` + `upload_file` (`files.getUploadURLExternal`/`completeUploadExternal`); ephemeral file uploads are not possible in Slack. Or keep it inline and fence each chunk | S |
| `/memory` list (10, public) and pending (session-scoped) | C:680-730 | DONE: S:683-704 | - | - |
| `/cron [list]` public, 15 jobs | C:733-785 | DONE: S:706-744 | - | - |
| `/steer` ephemeral; steers or queues | C:1032-1060 | DONE: S:511-525 | - | - |
| `/undo` public | C:1063-1090 | DONE: S:526-545 | - | - |
| `/retry` ephemeral; refused while draining | C:1093-1123, C:1572+ | DONE: S:546-562 | - | - |
| `/compress` public; requires LLM; window of 120k chars | C:1126-1196 | DONE: S:746-788 (constants and `select_compression_window` duplicated, S:25-26 and S:375) | Could export the Discord ones instead (cleanup only) | - |
| `/title` renames the thread; ephemeral error outside a thread | C:1198-1225 | PARTIAL: S:563-565 always returns an ephemeral "Discord-only" message | If inside a thread whose root is the bot's `🧵 *name*` message (from `/omo-thread`), `chat.update` that root via `api.update_message`. This needs the root `ts`, which is `invocation.thread_ts`. Whether the slash payload actually carries `thread_ts` is unverified, see note 1 | S |
| `/thread` rejects inside a thread (ephemeral) | C:1238-1246 | DONE: S:797-799 (only if `thread_ts` is present, see note 1) | - | - |
| `/thread` only in server text channels (rejects DMs) | C:1288-1294 | DIFFERENT: Slack allows it in DMs (`D…`) | Decide: either keep it (a Slack DM thread is valid) or reject `invocation.is_dm()` | S |
| `/thread` marks the thread owned/active so later unmentioned replies are accepted | C:1255-1256 `mark_thread_owner` (C:114-128, durable) | MISSING: S:790-827 only routes when a starter message is given. With no starter, no `sessions` row is written, so `ingress.rs:44-47 session_exists` is false and later thread replies without @mention are dropped (unless the channel is a free-response channel) | In `thread()`, always persist the session row for `thread_key` (expose `discord::commands::ensure_session`, C:1015, or insert directly) | S, C:1015 (visibility) |
| `/thread` reply links the thread (`<#id>`) | C:1271-1284 | PARTIAL: S:808-826 says "Created thread *name*" with no link | Add `chat.getPermalink` to api.rs and include the link | api.rs, S |
| `/deny [reason]`: public on success, ephemeral if none pending | C:1300-1331, A:609-623 | DONE: S:569-585 | - | - |
| `/yolo [on/off]` ephemeral, persisted | C:1336-1407 | DONE: S:586-597 (`**` converted by `mrkdwn_bold`) | - | - |
| `/pair <code>`: operator or authorized user approves code; outcomes Success (public) / Invalid / Expired / LockedOut (ephemeral) | C:1410-1475; `command_check` bypass for pair at C:363-365; operator check C:352-354 | MISSING | Add `SlackCommand::Pair` plus a manifest entry `/omo-pair`. `PairingStore` is keyed by `u64` user ids (P:22, P:40-41, P:72) and lockout key `"discord"` (P:17), so it needs a string-id plus platform-key variant (e.g. user id `slack:U…`, platform key `"slack"`). The success reply should use `<@U…>` | P, S, `slack/mod.rs`, manifest |
| Pairing prompt to unauthorized DM users (code + "/pair" hint, rate-limited) | adapter.rs:1343-1356, adapter.rs:1818-1840 using `check_and_record_notification_at` (P:348) | MISSING: ingress.rs:206-208 silently drops users who are not allowed | In `SlackIngress::handle`, for DMs from unknown users call the generalized `check_and_record_notification`, then `api.post_message` (or `post_ephemeral`) with the code | ingress.rs, P |
| `/sessions` owner-only (Korean reject, always ephemeral) | SC:10-29 | DONE: S:598-602 against `config.owner_id()` (`slack/mod.rs`:73) | - | - |
| `/sessions` ephemeral in guild, public in DM | SC:14-16 | DONE: S:602 `!invocation.is_dm()` | - | - |
| sessions overview / read (limit 20, probe fallback) / send (`--mode`, auto-watch) / stop / answer (`--request`) / watch / unwatch (fallback to unresolved target) | SC:76-455 | DONE: S:830-959 (Slack text parsing at S:263-306) | - | - |
| sessions error replies `⚠️ {e}` with same visibility | SC:54-66 | DONE: S:822-828 | - | - |
| Reply chunking (Discord 2000 / `DISCORD_LIMIT` 1900; "(내용 없음)") | C:1502, SC:31-52 | DONE: S:962-995 at 3900 chars; first 5 via `response_url`, then `chat.postMessage`/`chat.postEphemeral` | Ephemeral overflow `post_ephemeral` sends no `thread_ts` (api.rs `post_ephemeral`), so overflow lands outside the thread | api.rs, S |
| Slash session key (thread → parent channel + thread id; DM user id) | C:974-1004 | DONE: S:329-349 (`team_id` as guild) | - | - |
| Approval buttons Once/Session/Always/Deny; `omon:approval:` ids | A:634-678 | DONE: `egress.rs` `approval_blocks`, `interactive.rs` | Style differs (Always is default-styled, not green); cosmetic | - |
| Approval click authorization | (adapter) | DONE: `interactive.rs:39-46` (`is_allowed`, ephemeral reject) | - | - |
| Approval decided label | (adapter) | PARTIAL: `interactive.rs:83-90` labels omit who decided | Optionally append `by <@user>` | interactive.rs |
| Approval request routing and expiry broadcast | A:333-366 | DONE: `platform_router.rs:61-88` | - | - |
| YOLO / allowlist persistence shared | A:416-560 | DONE: same `SmartApprovalGuard` (main.rs:1361, 1493) | - | - |

Hardcoded Discord coupling:
- `discord/commands.rs:997` and `:1265`: session keys use the literal `"discord"` (correct for Discord, not reusable).
- `discord/commands.rs:352-354` `is_pairing_operator`, and `discord/session_commands.rs:10-12` `owner_allowed`, both use `sessions/status_reply.rs:36-47` `owner_id()`, which reads `DISCORD_ALLOWED_USERS` and falls back to `inbox::proposals::OWNER_ID`.
- `discord/pairing.rs:17` `DEFAULT_PLATFORM_KEY = "discord"`; user ids are `u64` throughout (`:22`, `:40-41`, `:58-66`, `:72`, `:448`).
- `discord/session_commands.rs:7`, `sessions/overview.rs:10`: `DISCORD_LIMIT` (Slack passes its own `MESSAGE_LIMIT`, so that path is fine).
- `discord/approval.rs:8`, `:57`, `:596`: the shared `SmartApprovalGuard::request_with_session` always builds serenity `CreateActionRow` components, even for Slack prompts. Harmless, but Slack carries Discord types. `DiscordApprovalRequester` (`:196`) is the generic requester despite its name.
- `discord/commands.rs:1502` `chunk_slash_reply` is reused by Slack (`slack/commands.rs:963`); its markdown chunker is Discord-flavored.
- `slack/mod.rs:23` `DEFAULT_OWNER_ID = "U04M1JNJM8T"`: a hardcoded Slack owner fallback when `SLACK_ALLOWED_USERS` is unset.
- `main.rs:1283-1285`: the owner home DM for the inbox digest is queried only for `platform='discord'`; `inbox/digest.rs:20`, `:40` hardcode a discord origin. No Slack home channel exists.
- `tools/cron.rs:291-305`: cron `deliver`/`origin` is auto-filled only when `session.platform == "discord"`. Reminders created from Slack (via the agent; `/omo-cron` itself is list-only) default to `local` and are never delivered. `cron/scheduler.rs:413`, `:2360-2404`, `:2561` and `cron/store.rs:673-706` parse and deliver only discord targets. This is outside the four scope files but is the delivery-target gap you asked about.
- `main.rs:1305`: `destructive_slash_confirm` is applied only to Discord `poise_data`; Slack's `PoiseData` keeps the default (`discord/commands.rs:97`).
- `sessions/status_reply.rs:51-58` `owner_dm` already has a Slack branch (no gap).

Note 1 (unverified): Slack slash-command payloads historically do not include `thread_ts`. `SlashInvocation` reads it at `slack/commands.rs:122`, but if Slack never sends it, then the in-thread `/thread` rejection, `/title`, and thread-scoped session keys for every slash command will resolve to the channel session, not the thread session. Check this against a live payload before relying on any thread-scoped command behavior.
</answer>

<next_steps>
1. Highest-value fixes, in this order: pass `destructive_slash_confirm` through (S:58, S:405, main.rs:1490); persist the thread session row in `/omo-thread` (S:790); check a live slash payload for `thread_ts`.
2. Pairing parity needs a design call first: generalize `PairingStore` to string ids plus a platform key, then add `/omo-pair` and the ingress prompt.
3. The cron and home-DM delivery coupling (`tools/cron.rs:291`, `main.rs:1283`) belongs to the delivery/cron area audit, not commands.
</next_steps>
</results>
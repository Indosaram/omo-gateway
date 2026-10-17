st_01a1474d [completed] model mahoquot/claude-opus-5-5
Ask: list every Discord egress behavior in scope (`src/discord/adapter.rs` 2300-end, `buttons.rs`, `table_render`, `throttler.rs`), say whether Slack already has it, and say what Slack needs if not. For you: the table covers the whole scope. Slack already handles streaming, sequence dedupe, silence, edit/delete, reactions, the approval button UI and expiry. It's missing MEDIA directives, the runtime footer, mention safety, typing, dead targets, upload guards, `owner_thread_is_private` and the generic `omon:btn:` buttons. It only partly covers tables, chunk limits, reply threading and the watchdog. One finding matters more than the rest: Slack egress passes `<!here>`, `<!channel>` and `<!everyone>` through unchanged. Now: done. This was a read-only audit; I edited nothing, did not run cargo, and every cite comes from files I read this session.

<analysis>
**Literal Request**: A parity table between DiscordEgress and the Slack adapter, plus a list of hardcoded Discord coupling.
**Actual Need**: A concrete list of what to build for Slack, with files and Slack APIs.
**Success Looks Like**: Each row has a Discord cite, a Slack status with a cite, the concrete fix and the files to write.
</analysis>

<results>
<files>
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/adapter.rs - DiscordEgress (2652-4210), uploader (2277-2508), receive watchdog (2510-2650), dead-target registry (2688-3048), media helpers (48-176), footer (276-341)
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/throttler.rs - `LiveEditThrottler` (133-263), `chunk_markdown` and pagination (281-327), `bound_split_messages` (331-348)
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/table_render/mod.rs - `transform_markdown_tables_to_images` (368), `fallback_text` (413)
- /Users/indo/code/project/omon-gateway-wt/slack/src/discord/buttons.rs - `omon:btn:` handler for the content-intel and kakao-cal buttons
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/egress.rs - SlackEgress, the main place Slack changes go
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/api.rs - Slack Web API client; new methods go here
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/interactive.rs - Slack button handling (approvals only today)
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/mrkdwn.rs - Slack markdown conversion and splitting
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/socket.rs - Socket Mode loop (relevant to the watchdog row)
- /Users/indo/code/project/omon-gateway-wt/slack/src/slack/mod.rs - SlackConfig and Slack owner id
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/platform_router.rs - routes by platform; anything not Slack falls back to Discord
- /Users/indo/code/project/omon-gateway-wt/slack/src/multiplexer/actor.rs - `OutboundDispatcher` defaults (85-99), owner lead session (1250-1325)
- /Users/indo/code/project/omon-gateway-wt/slack/src/main.rs - egress wiring (1170-1192)
- /Users/indo/code/project/omon-gateway-wt/slack/src/sessions/status_reply.rs - owner id, owner DM and owner thread checks
</files>

<answer>
### Parity table

Abbreviations: `D:` = `src/discord/adapter.rs`, `S/` = `src/slack/`.

| Behavior | Discord impl | Slack status | What Slack needs | Write-scope files |
|---|---|---|---|---|
| Strip reasoning; explicit silence sends nothing | D:3570-3574, 3410-3417 | **DONE**: S/egress.rs:72-76, 86-90, 133, 146-155 | none | none |
| Streaming: one placeholder per stream, sequence dedupe, final split into overflow messages | D:3388-3507, throttler.rs:168-238 (Discord sends only the final chunk, D:3509-3511; non-final chunks just refresh typing) | **DONE, and more**: S/egress.rs:101-177 shows a live preview, edited at most every 1.2s (:19, :129-140); dedupe at :116-122; a completed-stream map stops a retried final from reposting (:20, :172-177) | Optional: a final-only mode for cron, since Discord's cron is final-only per the migration notes | S/egress.rs |
| `MEDIA:` directives on SendMessage and on the stream's final chunk: extract, validate the path (temp dirs + workspace root, secrets denied), upload, append a failure note, report the error after the text is delivered | D:3192-3231, 3597-3612, 3462-3470; helpers D:48-176 | **MISSING**: S/egress.rs:71-83 and :143-167 send the text as-is, so `MEDIA:/path` reaches users literally | In `send_message` and the stream final: call `extract_media_directives` and `validate_media_path_in` (both public, D:133, D:94), then `api.upload_file(channel, thread_ts, path, None)` (S/api.rs:144). Pass the workspace root into SlackEgress. Add a Slack failure note, because Discord's `append_media_failure_note` (D:120, private) uses Discord-only `-# ` subtext. Keep the "text first, error after" order. | S/egress.rs, S/mod.rs or main.rs:1181 (workspace root), optionally make D:120 public or move the helper to a shared module |
| Markdown tables rendered as PNG images, with a raw-markdown fallback if the upload fails | D:3125-3157; table_render/mod.rs:368, 413 | **PARTIAL**: tables become code blocks (S/mrkdwn.rs:57-66, test :427) | If PNGs are wanted: call `transform_markdown_tables_to_images` before `to_mrkdwn`, upload each image with a new `SlackApi::upload_bytes(channel, thread_ts, filename, bytes, title)` (same `files.getUploadURLExternal`, upload URL, `files.completeUploadExternal` flow; `files` takes an array). On failure, post `fallback_text`. The placeholder text is Korean and Discord-flavoured (table_render:388). The code-block form is a reasonable choice for Slack too. | S/api.rs, S/egress.rs |
| Runtime footer (`model · % · cwd`) on SendMessage | D:3597-3604, 276-341; wired at main.rs:1174-1176 | **MISSING** | Add `with_runtime_footer`, `with_default_model` and `with_workspace_root` to SlackEgress, call `append_runtime_footer` (D:321, public; its `_..._` is valid Slack italics), and wire it at main.rs:1181 from `config.runtime_footer` and `config.default_model` | S/egress.rs, main.rs |
| Chunking: 2000-char limit, at most 8 messages plus a truncation notice, `(i/N)` page headers, fences reopened | throttler.rs:281-348; D:3615-3618 | **PARTIAL**: `split_message` at 3900 chars keeps fences intact (S/mrkdwn.rs:78, S/mod.rs:26), but there is no message cap and no pagination | Cap the number of chunks (port `bound_split_messages` with a Slack limit) so a runaway reply can't post dozens of messages. Pagination is optional. | S/mrkdwn.rs or S/egress.rs |
| Edit stays one message | D:877-883, 3779-3845 | **DONE**: S/egress.rs:86-98 uses the first chunk | none | none |
| Delete | D:3847-3885 | **DONE**: S/egress.rs:274-281 | none | none |
| Reply to the triggering message (first chunk references it; retry without the reference on failure) | D:3592-3595, 3694-3739, 179-190 | **PARTIAL**: `reply_to` is ignored (S/egress.rs:263-265); replies thread only when `session.thread_id` is set | Decide whether a top-level free-response reply should thread under `reply_to` (set `thread_ts`). Today it posts flat. | S/egress.rs |
| Mention safety (users allowed; `@everyone` and roles blocked) | D:192-198, used on every send | **MISSING (real risk)**: `to_mrkdwn` keeps `<!here>`, `<!channel>`, `<!everyone>` and `<!subteam^…>` (S/mrkdwn.rs:14, test :394) | Neutralise `<!here\|channel\|everyone>` and `<!subteam^…>` in `render()` (S/egress.rs:308), e.g. escape them to `&lt;!here&gt;`, while keeping `<@U…>` | S/egress.rs or S/mrkdwn.rs |
| Typing indicator: start/stop, refreshed every 5s during a stream, skipped for dead channels | D:3941-3959, 3359-3386, 2672-2686 | **MISSING**: `Typing { .. } => Ok(())` (S/egress.rs:284) | The Slack Web API has no general bot typing indicator. Options: `assistant.threads.setStatus` (AI-app threads only; needs the assistant scope) or rely on the existing 👀 reaction (S/ingress.rs:12, 73-79). The stream's live preview partly substitutes. | S/egress.rs, S/api.rs if setStatus is used |
| Reactions: add; with `remove_others`, remove 👀 | D:3961-4044 (tries the thread first, then the parent) | **DONE**: S/egress.rs:179-209 maps emoji to names and removes the other status reactions; a Slack `ts` is unique per channel, so no parent fallback is needed | Discord returns an error on a dead target; Slack only logs | none |
| Approval request: embed plus Allow Once / Session / Always / Deny buttons; truncated command and reason | D:4046-4109, 4142-4211 | **DONE**: S/egress.rs:212-237, 324-368 (Block Kit buttons, 2800/300-char truncation, escaping) | none | none |
| Approval mentions (`<@user>` for allowed users when enabled) | D:4186-4211, 3289-3293; main.rs:1173 | **MISSING** | Add `with_approval_mentions(Vec<String>, bool)` to SlackEgress and prefix `fallback` and the first section with `<@U…>`. `SlackConfig.allowed_users` (S/mod.rs:33) has the ids; there is no Slack "approval mentions" config flag yet. | S/egress.rs, S/mod.rs, main.rs:1181 |
| Approval expiry: strip the buttons, never overwrite a decision | D:4111-4133 | **DONE**: S/egress.rs:239-252; a decided approval is removed first (:63-70), so expiry can't overwrite it | none | none |
| Decision label names who clicked | D:1411-1427 (`… by {user_name}`) | **PARTIAL**: S/interactive.rs:83-90 shows only the decision | Add `by <@user>` (ingress/interactive area) | S/interactive.rs |
| Dead-target registry: classify 403/404, skip known-dead channels, retry after 60s, persist to SQLite, clear on success | D:2688-3048 and every dispatch arm; main.rs:1164-1172 | **MISSING**: every Slack failure just propagates | Slack-side classifier for `channel_not_found`, `is_archived`, `not_in_channel`, `account_inactive`, mapped from errors raised at S/api.rs:338-341. The registry and storage are keyed by `u64` channel ids (D:2690, 2705), and Slack ids are strings like `C0123`, so it needs a string-keyed variant (or a generalised key) before Slack can reuse it. | S/egress.rs, S/api.rs (typed error code), D:2688-3048 or a new shared module, storage dead-target fns |
| Upload guards: secret-file denylist, 100 MB cap read before loading, dead-target marking | D:3887-3936, 2436-2472, 48-70 | **MISSING**: S/egress.rs:254-265 and S/api.rs:150 read any path fully into memory | Check `media_path_is_denied` (D:48, private, needs to be public or moved) and stat the size before `tokio::fs::read`. Slack's own per-file limit is set by Slack. | S/egress.rs, S/api.rs |
| Native voice messages and forum posts | D:2345-2421, 3630-3692 | **N/A**: Slack has no bot voice-note or forum equivalent; `files.*` covers audio | none | none |
| Multiple bot identities (one HTTP client per `session.bot_id`) | D:3076-3103, 3346-3357 | **PARTIAL**: one `SlackApi` per egress (S/egress.rs:38) | Only needed for multiple Slack apps | S/egress.rs |
| `owner_thread_is_private` (a private thread whose members are only the owner and the bot) | D:3515-3543 | **MISSING**: uses the trait default `false` (actor.rs:88-90); the router forwards it (platform_router.rs:88-93) | Implement it on SlackEgress: `conversations.info` (S/api.rs:120) checks `is_private`/`is_im`, and a new `conversations.members` API method checks members ⊆ {owner, bot}, with a 2s timeout. This also needs a Slack branch in `status_reply::owner_thread` (status_reply.rs:67-75), which today requires `platform == "discord"` and `DISCORD_HOME_CHANNEL_THREAD_ID`. | S/egress.rs, S/api.rs, sessions/status_reply.rs |
| `dispatch_direct_reply` (lane replies with lane mention rules) | D:3545-3561 | **DONE via default**: actor.rs:92-99 turns it into SendMessage | Inherits the mention-safety gap above | none |
| Retry failed deliveries after reconnect | D:3248-3287 | **N/A**: no caller anywhere (grep finds only the definition). Boot recovery `recover_pending_delivery_obligations` (main.rs:1209) goes through PlatformRouter, so Slack is covered. | none | none |
| Receive watchdog (restart a silent gateway session) | D:2510-2650 | **PARTIAL**: S/socket.rs:116-170 reconnects on stream end, read error or disconnect frame, with no silence or zombie timer | Optionally add an idle timeout (e.g. reconnect if no frame or ping for N minutes) | S/socket.rs |
| Generic `omon:btn:<ns>:<action>:<arg>` buttons (content-intel publish/preview/skip, kakao-cal): fail-closed auth, load config, run the command with a cleared env, "⏳ 처리 중…" pending state, done labels, non-terminal actions, threaded result reply, in-flight lock, interaction dedupe | buttons.rs:1-430; routed from D:1373-1376 | **MISSING**: S/interactive.rs:61-73 keeps only `omon:approval:` values | New Slack handler in interactive.rs. Reusable as-is: `parse_button_custom_id`, `button_actions_path`, `load_button_action`, `run_button_action`, `reply_text`. Not reusable because they're serenity- or u64-coupled: `is_button_clicker_allowed`, `rebuild_buttons`, `IN_FLIGHT`, `EXECUTED_INTERACTIONS`, `handle_button_click`. Slack version: auth with `config.is_allowed`; dedupe on `action_ts`/`trigger_id` with string keys; rebuild state from `payload.message.blocks` and push it with `chat.update`; post the result with `chat.postMessage` (`thread_ts` = message ts); env vars get Slack ids; `post_ephemeral` for refusals. Socket Mode already acks before forwarding. Unverified: whether the content-intel and kakao-cal producers can post Slack Block Kit buttons; they live outside this repo. | S/interactive.rs, a new `S/buttons.rs`, optionally move the platform-neutral half of buttons.rs to a shared module |

### Hardcoded Discord coupling (affects Slack delivery and owner identity)
- `main.rs:1181-1183`: `SlackEgress::new(api)` gets no dead targets, approval mentions, footer, model or workspace root, unlike Discord at :1171-1176.
- `platform_router.rs:31-41` with `main.rs:1186`: the fallback is the Discord egress, so any platform without its own route (cron and others) goes to Discord.
- `sessions/status_reply.rs:36-46`: `owner_id()` reads `DISCORD_ALLOWED_USERS`, and `dashboard.rs:1271-1276` uses it.
- `sessions/status_reply.rs:67-75`: `owner_thread` requires `platform == "discord"` and `DISCORD_HOME_CHANNEL_THREAD_ID`.
- `sessions/status_reply.rs:55-57`: `owner_dm` non-Slack branch covers only discord and web.
- `multiplexer/actor.rs:1250-1325`: the Slack lead session is DM-only (:1251-1253). Discord has hardcoded bot id `1465631383862120451`, fallback owner `414011632306618368`, and `DISCORD_HOME_CHANNEL(_THREAD_ID)`. Slack has no home-channel concept.
- `slack/mod.rs:23`: hardcoded Slack `DEFAULT_OWNER_ID` `U04M1JNJM8T`.
- `cron/scheduler.rs:2357-2405`: any platform other than slack becomes discord; `CronNotification` is Discord-only (:2362); profile→bot_id only for Discord (:2404).
- `tools/cron.rs:291-305`: the delivery target is auto-filled only for Discord sessions, so Slack-created reminders get no origin auto-fill.
- `inbox/digest.rs:20, 31` and `main.rs:1279-1292`: the digest home DM must be `platform='discord'`.
- `main.rs:1157-1161`: the Full Disk Access-denied owner notice uses Discord HTTP and `OMON_OWNER_ID` only.
- `D:120-130`: the media failure note uses Discord `-# ` syntax. `table_render/mod.rs:388`: the placeholder points to a Discord attachment.
- `D:2688-2705` and the storage dead-target functions: channel ids are `u64`, which doesn't fit Slack ids.
- `buttons.rs:116-118, 28-30, 349-430`: user, message and interaction ids are `u64`; everything goes through serenity.
- `throttler.rs:281-288`: pagination is controlled by `DISCORD_CHUNK_PAGINATION`, a Discord-specific env var.
</answer>

<next_steps>
Recommended order of impact:
1. Mention safety in `S/egress.rs` `render()` (a real ping risk).
2. `MEDIA:` directives and the upload guards (denylist, size cap).
3. Wire footer, model, workspace root and approval mentions at `main.rs:1181`.
4. Slack `omon:btn:` handler.
5. `owner_thread_is_private` together with the Slack branch in `status_reply::owner_thread`.
6. Dead targets, after deciding how to key the registry by string.
7. Message-count cap, then table PNGs if wanted.

Typing has no real Slack equivalent outside assistant threads; decide whether to accept the 👀 reaction and live preview instead.
</next_steps>
</results>
# L-main implementation report

Scope: `src/main.rs`, `src/slack/{mod,runtime,home}.rs`, `src/readiness.rs`, `src/dashboard.rs`, `src/dashboard_runtime.rs`, `src/multiplexer/{platform_router,actor}.rs`. Per the lane rules, no cargo command was run. The first compile is the lead's integration build. `rustfmt` was run on the new `src/slack/home.rs`. When I resumed, the write scope had no partial edits from the earlier attempt, so all of this is fresh work.

## Fixed contracts

| Contract | What was done | file:line |
|---|---|---|
| 1 `SlackConfig` fields | Added every listed field. `AllowBots` is a new enum (None/Mentions/All). Lookup order is `SLACK_<NAME>`, then `DISCORD_<NAME>`. The defaults match the contract, and the context limit is capped at 25. Id-valued settings (users, channels, home channel) never inherit Discord values, because a Discord snowflake cannot name a Slack conversation and an inherited allowlist would silence Slack. `destructive_slash_confirm` also honours `APPROVALS_DESTRUCTIVE_SLASH_CONFIRM`, matching Discord. `home_channel` is merged into `free_response_channels`, mirroring main.rs's handling of `DISCORD_HOME_CHANNEL`. `test_default()` and `home_channels()` were added. | src/slack/mod.rs:38, :68-82, :112, :159, :188 |
| 4 `home::owner_dm_key` | `conversations.open` with the owner, then a DM `SessionKey` (`slack`, no guild, no thread, user = owner, bot = bot user id), persisted to `sessions` with `ON CONFLICT DO NOTHING`. | src/slack/home.rs:13 |
| 5 ingress builders and `flush` (called) | `runtime::build` chains `.with_approvals(..)` and `.with_pairing(..)`, and adds `.with_inbox_actions(..)` when triage proposals exist. `SlackRuntime::flush()` delegates to `ingress.flush()`. | src/slack/runtime.rs:29, :46-50, :84 |
| 6 `backfill::run` (called) | `SlackRuntime::backfill()` runs once when `missed_backfill` is set. main.rs spawns it after build, before `supervise`. | src/slack/runtime.rs:72; src/main.rs:1798 |
| 8 `SlackEgress` builders (called) | `with_workspace_root`, `with_runtime_footer`, `with_default_model`, `with_approval_mentions(allowed_users, approval_mentions)`, `with_pool`. | src/main.rs (slack_egress block before `for_transports`, ~:1400) |
| 9 Slack message-context provider (registered) | Registered on the `MessageContextTool`, using the configured allowed and ignored channel lists. The Discord provider stays registered when Discord tokens exist. | src/main.rs:1299 |
| 10 `SlackSource` (registered) | `inbox_sources(pool, Some((owner, bot)))` appends `SlackSource::new(pool, owner_slack_id, bot_user_id)`. | src/main.rs:2770 |
| 11 `SlackCommandDeps.destructive_slash_confirm` and `pairing` | Both are set. `SlackPairingStore::init_cache()` runs at boot. | src/main.rs:1646-1647 |

## Home / lead section

- `boot_slack` runs right after the pool opens and **before** the kibitzer, inbox triage, lanes, session watcher, digest and remediation start (src/main.rs:787, called at :1224). It calls `auth.test`, then copies the persona, then:
  - if `home::is_home(OMON_HOME_PLATFORM, discord_configured)` is true, it resolves the owner DM key;
  - if `OMON_LANE_DEFAULT_LEAD` is unset, it publishes that key there. Every existing lead reader goes through that env var: the kibitzer, `cron/health`, proposals, the actor and `lanes/runner`.
- Resulting `default_lead()` order:
  1. explicit `OMON_LANE_DEFAULT_LEAD`;
  2. the Slack owner DM when `OMON_HOME_PLATFORM=slack` or Discord is unconfigured;
  3. the Discord default.

  `home::is_home` lives at src/slack/home.rs:47.
- Persona: when no `bot_profiles` row exists for the Slack bot user id, it copies the row from `OMON_SLACK_PERSONA_FROM` (default `1465631383862120451`). It never overwrites an existing row. src/slack/home.rs:57, :80; called in `boot_slack`.

## inbox-cron-sessions.md rows

| Row | Status | file:line |
|---|---|---|
| 3 Digest registration at boot | Closed. The home-DM query accepts `platform IN ('discord','slack')` for the lead key. The owner comes from `digest_owner()`: the Slack owner when the lead is a Slack key, otherwise `OMON_OWNER_ID` or the Discord owner. The Slack lead is resolved before the scheduler registration. | src/main.rs:842, :1540 |
| 7 Triage proposals bound to the lead channel | Closed through row 23: triage reads `default_lead()`, which is now the Slack DM when Slack is home. | src/main.rs:787 |
| 23 Default lead / owner home DM | Closed (Home / lead above). | src/main.rs:787; src/slack/home.rs:13 |
| 24 Owner lead-session detection, Slack home channel | Closed. A Slack session counts as the lead when it is the owner DM, or any conversation (including threads) in `SLACK_HOME_CHANNEL`. | src/multiplexer/actor.rs:1402 |
| 33 FDA alert | Closed. `FdaAlertChannel::{Discord,Slack}` uses the same identity marker. The alert goes to the Slack owner DM when Slack is home, otherwise to the Discord DM as before. | src/main.rs:951, :997, :1376 |
| 34 Boot without `DISCORD_BOT_TOKEN` | Closed. `require_a_transport` rejects only when neither Discord nor Slack is configured. With no Discord, there is no Discord egress, no `primary_bot_id` and no clients. `PlatformRouter::for_transports` falls back to Slack. Boot fails only if the Slack runtime then cannot start either. | src/main.rs:292, :1424, ~:1676; src/multiplexer/platform_router.rs:28 |
| 35 Readiness gateway probe | Closed. `collect_runtime_readiness_with_slack` counts a connected Slack bot in `gateway` and adds a `slack` check (ok with bot and team ids, or degraded with the error; no tokens). The Slack runtime is now built before readiness runs. The old function keeps its signature and its Discord-only output. | src/readiness.rs:263, :270, :349; src/main.rs:1631, :1680 |
| 36 Dashboard config view | Closed. A `slack` block lists the settings and never the tokens, in both the gateway-attached view (`config_view_from_gateway`) and the standalone dashboard (`attach_slack_view`). The standalone `bot_connections` count includes Slack. | src/dashboard.rs:667, :727; src/dashboard_runtime.rs:204, :300 |
| 37 Dashboard message-context policy store | Closed. A separate `slack_message_context_policy` block holds the `"slack"` override and effective matrix. The existing `message_context_policy` (Discord) is unchanged. | src/dashboard_runtime.rs:210, :300 |
| 42 Free-response home channel | Closed. `SLACK_HOME_CHANNEL` is merged into `free_response_channels`. | src/slack/mod.rs:112 (`from_lookup`) |

## adapter-ingress / adapter-egress coupling items (main.rs / actor.rs)

| Item | Status | file:line |
|---|---|---|
| Debounce flush covers only Discord at shutdown (main.rs select arms) | Closed. `flush_slack(slack_runtime)` runs in all 5 arms: lane exit, Discord client exit, Ctrl+C, drain and SIGTERM. | src/main.rs:1820, :1831, :1838, :1847, :1869 |
| Cursor rewind is Discord-only (actor.rs:31) | Closed. `rewind_slack_backfill_cursor` moves `slack_cursors.last_ts` to just before the earliest dropped `slack:` constituent, keyed by (bot, channel, thread_ts or ''). It assumes the same table key L-ingress writes. | src/multiplexer/actor.rs:32, :88 |
| Recovery author provenance is Discord-only (main.rs:408) | Closed. Slack sessions take the author from `message_search_documents` with `source='ingress'`. `StartupAuthorization.slack` applies the Slack allowlist, pairing, allow-all and channel lists. When Slack is off, a Slack session is never auto-recovered. | src/main.rs:375, :472 |
| `SlackEgress::new(api)` without footer, model, mentions, workspace or pool | Closed (contract 8 row). | src/main.rs ~:1400 |
| PlatformRouter falls back to Discord | Closed. Slack is the fallback when Discord is absent. | src/multiplexer/platform_router.rs:28 |
| FDA notice is Discord-only | Closed (row 33). | src/main.rs:951 |

## Unit tests added (not run)

- slack/mod.rs: contract defaults; SLACK-over-DISCORD precedence and the context-limit cap; ids never inherited; home channel is also free-response; `AllowBots` parsing.
- slack/home.rs: owner DM key is persisted and idempotent, against a fake `conversations.open`; `is_home` matrix; persona copied once and never overwritten; persona source default.
- platform_router.rs: Slack-only fallback, and both transports with Discord as the fallback.
- actor.rs: Slack home-channel lead; ts predecessor arithmetic; Slack cursor rewind to the earliest constituent.
- readiness.rs: Slack probe states; a Slack-only gateway counts its bot, and a Discord-only report has no `slack` key.
- dashboard.rs / dashboard_runtime.rs: Slack view has no tokens; policy block appears only when Slack is configured.
- main.rs: transport requirement; digest owner follows the lead platform; Slack recovery rules; Slack inbox source registration.

## Not closed, or dependent on other lanes

- **Contracts 5 and 6 compile against L-ingress.** At the time of writing, `src/slack/backfill.rs` and `src/slack/lifecycle.rs` do not exist, and `SlackIngress` does not yet have `with_inbox_actions`, `with_approvals`, `with_pairing` or `flush`. `debounce.rs` exists. `mod.rs` declares `backfill`, `debounce`, `lifecycle` and `buttons` per the plan's file list, and the calls use the contract names exactly. If L-ingress decides not to create `lifecycle.rs`, drop `pub mod lifecycle;` at src/slack/mod.rs:17.
- **Headless branch (`OMON_DISCORD_ENABLED=false`) does not start Slack.** That branch is the lane-only runtime with local transcript output. A Slack-only deploy now goes through the normal path with no Discord token, which is what the plan specifies.
- **`api.rs` `post_ephemeral` thread_ts** (from impl-l-commands) is outside my write scope.

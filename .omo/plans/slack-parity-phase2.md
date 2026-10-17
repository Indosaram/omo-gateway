# Slack parity phase 2: full migration off Discord

Base: branch `slack-channel` @ `ee802c2`. Phase 1 there already added the Slack adapter, the platform router, and Slack cron destinations.
Audit evidence: `.omo/evidence/slack-parity/{adapter-ingress,adapter-egress,commands,lanes,inbox-cron-sessions}.md`. Each lane reads the audit file for its area; those files list every gap with file:line references.

Goal: everything the owner does or receives on Discord also works on Slack. Production then switches the following to Slack:
- owner home
- lead session
- digests and alerts
- lane threads
- cron deliveries

Discord stays available as an optional transport, and the gateway must start without it.

## Already in place (written by the lead)

- `migrations/0045_slack_channel.sql`
  - Adds `slack` to the inbox CHECK constraints and recreates the kibitzer trigger.
  - New tables: `slack_cursors(bot_id, channel_id, thread_ts, last_ts, updated_at)`, `slack_dead_targets(channel_id, reason, marked_at, probed_at)`, `slack_paired_users(user_id, paired_at, operator_id)`.
- `src/slack/api.rs` gained:
  - `conversations_history(channel, oldest, latest, limit) -> Vec<Value>`
  - `conversations_replies(channel, thread_ts, oldest, limit) -> Vec<Value>`
  - `conversations_members(channel) -> Vec<String>`
  - `permalink(channel, ts) -> String`
  - `upload_bytes(channel, thread_ts, filename, Vec<u8>, title)`
- `discord::adapter::{media_path_is_denied, append_media_failure_note}` are now `pub`. The note text uses Discord `-# ` syntax, so Slack needs its own note.

## Lanes (write scopes are disjoint)

| Lane | Write scope (only these files) |
|---|---|
| L-ingress | src/slack/ingress.rs, src/slack/debounce.rs (new), src/slack/backfill.rs (new), src/slack/lifecycle.rs (new), src/slack/socket.rs |
| L-egress | src/slack/egress.rs, src/slack/mrkdwn.rs, src/slack/buttons.rs (new), src/slack/interactive.rs |
| L-commands | src/slack/commands.rs, src/discord/pairing.rs (add a Slack string-id store; Discord behavior must stay unchanged), docs/slack-app-manifest.json |
| L-lanes | src/lanes/** (new slack_threads.rs), src/storage/lanes.rs |
| L-inbox-cron | src/inbox/**, src/cron/**, src/sessions/**, src/tools/message_context*.rs (new message_context_slack.rs), src/tools/mod.rs (module declaration only) |
| L-main | src/main.rs, src/slack/mod.rs, src/slack/runtime.rs, src/slack/home.rs (new), src/readiness.rs, src/dashboard.rs, src/dashboard_runtime.rs, src/multiplexer/platform_router.rs, src/multiplexer/actor.rs |

## Fixed contracts (implement or call exactly these)

1. **`crate::slack::SlackConfig`** pub fields (L-main adds them; other lanes read them):
   - `home_channel: Option<String>` (from `SLACK_HOME_CHANNEL`)
   - `approval_mentions: bool` (true)
   - `allowed_channels: Vec<String>`
   - `ignored_channels: Vec<String>`
   - `allow_bots: AllowBots` (enum None/Mentions/All in slack/mod.rs; default None)
   - `thread_require_mention: bool` (false)
   - `channel_context: bool` (true)
   - `channel_context_limit: usize` (10, max 25)
   - `channel_topic_context: bool` (true)
   - `processing_reactions: bool` (true)
   - `runtime_footer: bool` (false)
   - `missed_backfill: bool` (true)
   - `destructive_slash_confirm: bool` (true)
   - `allow_all_users: bool` (false)

   Each one is read from `SLACK_<NAME>`. When that is unset, fall back to the matching `DISCORD_<NAME>`. `SlackConfig` must also gain a `pub fn test_default() -> Self` that the other lanes' unit tests can construct.
2. `crate::lanes::slack_threads::intercept_owner_reply(pool:&SqlitePool, channel:&str, thread_ts:&str, bot_user_id:&str, user:&str, owner:&str, ts:&str, text:&str) -> crate::Result<bool>`. The lane thread ref is `format!("slack:{channel}:{thread_ts}")`. Owned by L-lanes; called by L-ingress before `map_event`.
3. `crate::lanes::slack_threads::SlackLaneThreads::new(api: SlackApi, bot_user_id: String)`. It implements the lane transport traits. L-lanes also makes transport selection in runner/cli/guard choose by the thread-ref prefix (`slack:`) and build `SlackLaneThreads` from `SlackConfig::from_env` plus `auth_test`.
4. `crate::slack::home::owner_dm_key(api:&SlackApi, config:&SlackConfig, bot_user_id:&str, pool:&SqlitePool) -> crate::Result<SessionKey>`. Owned by L-main.
5. `SlackIngress` keeps `new(...)` and adds builders:
   - `with_inbox_actions(Arc<dyn crate::inbox::proposals::Actions>)`
   - `with_approvals(SmartApprovalGuard)`
   - `with_pairing(crate::discord::pairing::SlackPairingStore)`

   It also adds `pub async fn flush(&self)` (debouncer flush). All owned by L-ingress; L-main calls them.
6. `crate::slack::backfill::run(ingress:&SlackIngress) -> crate::Result<usize>`. Owned by L-ingress; L-main calls it once after build when `missed_backfill` is set.
7. `crate::slack::buttons::handle_button_actions(payload:&Value, config:&SlackConfig, api:&SlackApi) -> crate::Result<()>`. Owned by L-egress and called from `interactive::handle_block_actions`.
8. `SlackEgress` builders, owned by L-egress and called by L-main:
   - `with_workspace_root(PathBuf)`
   - `with_runtime_footer(bool)`
   - `with_default_model(Option<String>)`
   - `with_approval_mentions(Vec<String>, bool)`
   - `with_pool(SqlitePool)` (used for dead targets)
9. `crate::tools::message_context_slack::SlackMessageContextProvider::new(api: SlackApi, pool: SqlitePool)`, implementing `MessageContextProvider` with `platform()=="slack"`. Owned by L-inbox-cron; L-main registers it.
10. `crate::inbox::sources::slack::SlackSource::new(pool: SqlitePool, owner_slack_id: String, bot_slack_id: String)`, with `InboxSourceKind::Slack`. Owned by L-inbox-cron; L-main registers it in `inbox_sources()`.
11. `crate::discord::pairing::SlackPairingStore::new(pool)`, which mirrors the PairingStore API with `&str` user ids. Owned by L-commands. `SlackCommandDeps` gains `destructive_slash_confirm: bool` and `pairing: SlackPairingStore`; L-main sets both.
12. `crate::inbox::digest::spec` accepts a Slack owner DM home (`platform=="slack"`, no guild, no thread, `user_id==owner`). The origin platform is taken from the home. Owned by L-inbox-cron.
13. `crate::lanes::kibitzer` lead validation accepts the Slack owner DM. Owned by L-lanes. The rule: `platform=="slack"`, no guild, no thread, `user_id == crate::slack::owner_id()`, `bot_id` present.

## Home / lead (L-main)

`default_lead()` resolves in this order:
1. `OMON_LANE_DEFAULT_LEAD`.
2. When `OMON_HOME_PLATFORM=slack`, or Discord is not configured: the Slack owner DM key. It is resolved at boot after `auth.test` and before lanes, kibitzer, the session watcher, the digest, remediation and inbox triage start.
3. Otherwise the Discord default.

The gateway must boot with no `DISCORD_BOT_TOKEN` when Slack is configured. In that case the PlatformRouter falls back to Slack.

## Production migration (lead, after deploy)

1. **Slack channels.** Create `#omo-home`, `#work` and `#aside-ideas`. Set `SLACK_HOME_CHANNEL` and `OMON_HOME_PLATFORM=slack` in the launchd plist.
2. **Cron deliveries.** Re-point them in the DB JSON, after taking a backup:
   - `discord:1529738911377657967` → `slack:<#work>`
   - `discord:1548270340537323540` → `slack:<#aside-ideas>`
   - inbox digest origin → Slack owner DM
3. **External scripts.** Move `gov-support-2026/tools/deadline_alert.ts` and `content-intel-dashboard/scripts/publish-ready-digest.ts` to Slack. Buttons keep the `omon:btn:` ids as Block Kit action_id/value.
4. **Persona.** Copy the wawabot `bot_profiles` row to the Slack bot user `U0C9ST69M8B`.

## Verification (once, at the end)

1. `cargo clippy --all-targets -D warnings` and the full `cargo test`. One test, `test_review_dashboard::dash_001`, already fails on the base commit and is not counted.
2. Deploy, then run live QA through `agent-slack` (owner user token) and Aside, checking each of these:
   - a DM turn answered with the persona
   - a channel mention answered in a thread
   - "세션 상태" handled by direct reply
   - "제안" listing proposals
   - `/omo-status`
   - a `MEDIA:` upload
   - a table rendered to PNG
   - lane thread create, status, and owner reply
   - cron run-now delivered to Slack
   - a session-watch notice in the Slack DM
   - backfill after restart
   - a Slack-only boot with no Discord token (scratch DB, CLI)

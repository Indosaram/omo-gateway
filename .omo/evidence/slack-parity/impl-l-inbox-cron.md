# L-inbox-cron implementation report (resumed run)

The earlier attempt was cut off by a quota error. This run re-checked every row it claimed against the current code in `/Users/indo/code/project/omon-gateway-wt/slack`. Those rows were all present and complete, including the row 31 provider. The only change this run made was `rustfmt --edition 2021` on the lane files, and `--check` now exits 0.

No cargo command was run, as the lane rules require, so nothing below is compiled or tested yet. The lead's integration build will be the first compile.

## Closed gap rows (inbox-cron-sessions.md)

| Row | Change | file:line |
|---|---|---|
| 1 | `digest::spec` accepts a Slack owner DM home (no guild, no thread, `user_id == owner`) and takes `origin.platform` from `home.platform`. | src/inbox/digest.rs:19, :40 |
| 2 | The executor guard recomputes `spec` from the stored home. A Slack digest passes it and a tampered origin fails it. | src/inbox/digest.rs:213 (test `executor_guard_accepts_registered_slack_digest`); guard logic in `DigestExecutor::execute` |
| 5 | No change needed. `OWNER_ID` stays the logical DB owner key. Mapping a Slack author to it is done in the Slack proposal handler (row 6, L-ingress). | none |
| 8 | Added `SlackSource::new(pool, owner_slack_id, bot_slack_id)`, which reads `message_search_documents` with `platform='slack'`. It collects DMs, `<@owner>` mentions, and replies in threads the owner started. The cursor is the message `ts`. The link is `https://slack.com/archives/<C>/p<ts>`, with `?thread_ts=&cid=` added for replies. The real channel is taken from metadata `channel_id` or from the owning session. `InboxSourceKind::Slack` maps to `"slack"`, which migration 0045 allows. | src/inbox/sources/slack.rs:9, :16, :28, :130; src/inbox/sources/mod.rs:5; src/inbox/mod.rs:48, :58 |
| 9 | A `GatewayMessage` event now triggers Discord and Slack sources. | src/inbox/watchers.rs:74 (test :280) |
| 10 | Prefilter has a new `InboxSourceKind::Slack` arm and new fields `owner_slack_id` (default `slack::owner_id()`) and `bot_slack_id`. The struct is `serde(default)`, so older rules files still load. | src/inbox/prefilter.rs:21-22, :34-35, :168 (tests :259, :289) |
| 13 | Added `configured_slack_home()`. It reads payload `slack_home_channel`/`slack_home_thread`, then a `slack{channel,thread_ts}` object, then the `SLACK_HOME_CHANNEL` env var. `origin`/`all` falls back to the Slack home when no Discord home is set. A bare `slack` deliver resolves to the Slack origin, otherwise to the Slack home, in both the Hermes parser and the native payload parser. A bare `discord` deliver never falls back to Slack. | src/cron/store.rs:571, :575, :669, :714, :845; src/cron/scheduler.rs:2605 |
| 15 | Bug fix: the Hermes agent session platform now comes from `target.platform` instead of the hardcoded `"discord"`. | src/cron/executor.rs:291 (test :1316) |
| 16 | The native payload executor accepts `slack:<C>[:<ts>]` through `payload_session_key`. | src/cron/executor.rs:585, :636 (test :1278) |
| 17 | Missing-transport guard: when no dispatcher is configured, an explicit `slack:` destination (Hermes `deliver`/`failure_deliver`, or native `deliver`) now fails with `unconfigured delivery transport: slack`. Previously it reported success. | src/cron/store.rs:656; src/cron/scheduler.rs:2205 (test :4722) |
| 19 | `CronJob::profile()` accepts `slack:` session keys. | src/cron/scheduler.rs:413 (test :4779) |
| 25 | When `owner_dm.platform == "slack"`, the session-watch question hint reads `` `/omo-sessions answer <target> …` ``. The dedupe key is unchanged. | src/sessions/watch.rs:164, :313 (test :615) |
| 28 | `status_reply::owner_thread` has a Slack branch: the author is the owner, the message is in a channel, and the conversation is in `SLACK_HOME_CHANNEL`. Privacy is still decided by `dispatcher.owner_thread_is_private`. That check now exists in `SlackEgress` (src/slack/egress.rs:622, L-egress): the channel must be private and its members must be only the owner and the bot. | src/sessions/status_reply.rs:68, :88 (test :211) |
| 31 | `SlackMessageContextProvider::new(api, pool)` with `platform()=="slack"`. Operations: recent, get_message and replies use `conversations_history`/`conversations_replies`; search backfills over REST and then queries `MessageSearchIndex::search("slack")`. It validates ids: channels must match `[CDG][A-Z0-9]+` and ts must look like `<secs>.<frac>`. Policy matches Discord: ignored channels, no cross-channel reads from DM sessions, a cross-channel allowlist (`SLACK_ALLOWED_CHANNELS`), and the policy-store override under `"slack"`. Output is neutralized, and a thread session reads that thread. | src/tools/message_context_slack.rs:24, :46, :67, :286, :322, :386, :489, :503 (tests :713, :854); src/tools/mod.rs:7 |

## Plan contracts

- **Contract 9:** `crate::tools::message_context_slack::SlackMessageContextProvider::new(api: SlackApi, pool: SqlitePool)` (src/tools/message_context_slack.rs:46). The module is `pub` (src/tools/mod.rs:7).
- **Contract 10:** `crate::inbox::sources::slack::SlackSource::new(pool: SqlitePool, owner_slack_id: String, bot_slack_id: String)` (src/inbox/sources/slack.rs:16), with `InboxSourceKind::Slack`.
- **Contract 12:** `crate::inbox::digest::spec` accepts the Slack owner DM, and the origin platform comes from the home (src/inbox/digest.rs:19, :40).

Supporting in-scope edits: `MessageContextRequest::apply_policy_limits` is now `pub(crate)` (src/tools/message_context.rs:128), and `message_from_search_document` is now `pub(super)` (:1155). Discord behavior is unchanged.

## Not closable in this lane

- **Row 3** (digest registration at boot) and **row 23** (default lead): src/main.rs, owned by L-main.
- **Row 31 wiring** (registering the provider in `ToolRegistry`/main) and **contract 10 registration** (`inbox_sources()`): L-main, per contracts 9 and 10. This lane only added the module declaration.
- **Rows 6, 27 and 40:** L-ingress. **Rows 33-37:** L-main.

## Unit tests (written, not run)

- digest.rs: Slack spec, rejection matrix, executor guard
- sources/slack.rs: collection rules and links against an in-memory DB, plus a `collect_all` round trip
- prefilter.rs: Slack arm, and a legacy rules JSON loading with defaults
- watchers.rs: GatewayMessage filter
- store.rs: Slack home sources, origin/bare-slack fallback, explicit Slack detection
- scheduler.rs: Slack destination with no dispatcher fails, Slack dispatch keeps the thread, Slack profile
- executor.rs: payload session platform, Hermes session platform
- watch.rs: Slack hint text
- status_reply.rs: Slack home candidate
- message_context_slack.rs: four tests against a fake axum Slack API built with `SlackApi::with_base_url`

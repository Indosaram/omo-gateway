# impl L-lanes (resumed)

Previous attempt had already written most of the lane; this resume verified each row against source, fixed one test assertion that pinned an escape form `to_mrkdwn` does not produce (it already neutralises `<!channel>` itself), ran rustfmt on lane files, and wrote this report. No cargo build/check/test was run (lane rule).

Thread ref encoding: `slack:<channel>:<root_ts>` in `lanes.thread_channel_id`; Slack parent stored as `slack:<channel>` in `parent_channel_id`. `PostSink` and `LaneLifecycle` now take `&str` thread/message ids.

## Closed

| Gap row (lanes.md) / contract | Where |
|---|---|
| Contract 2 `intercept_owner_reply(pool, channel, thread_ts, bot_user_id, user, owner, ts, text)`; ref `slack:{channel}:{thread_ts}`; wakes runner when consumed | src/lanes/slack_threads.rs:369 |
| Contract 3 `SlackLaneThreads::new(api, bot_user_id)` (+ `lazy`/`from_env` = `SlackConfig::from_env` + `auth.test` on first use) | src/lanes/slack_threads.rs:112-141 |
| Transport handle / module | src/lanes/slack_threads.rs, src/lanes/mod.rs:12 |
| Create lane thread (starter `chat.postMessage` to parent, root ts = thread) | src/lanes/slack_threads.rs:143 |
| Status marker (starter `chat.update` 🔄/⏸/✅ + status reaction swap) / `rename_status` | src/lanes/slack_threads.rs:173, :237 |
| Durable outbox post (threaded, mrkdwn, chunked at MESSAGE_LIMIT, no pings) | src/lanes/slack_threads.rs:207 |
| Status message edit, `message_not_found` -> `Ok(false)` | src/lanes/slack_threads.rs:222 |
| No pings in lane posts (`<!…>`/`<@…>` escaped) | src/lanes/slack_threads.rs:88 |
| `LaneLifecycle` for Slack: bot id from auth.test, close = final reply, reopen = no-op | src/lanes/slack_threads.rs:245-276 |
| Platform router sink/lifecycle by ref prefix (`LaneTransports`) | src/lanes/slack_threads.rs:278-365 |
| `PostSink` trait to `&str`; Discord impl parses ids | src/lanes/posts.rs:73-160, `discord_id` :164 |
| Drain destination check accepts Slack refs | src/lanes/posts.rs:207 |
| Status refresh: thread/message ids no longer u64-only | src/lanes/posts.rs:326, :411 |
| `LaneLifecycle` trait to `&str`, `bot_id_for(thread)` | src/lanes/runner.rs:134-156, Discord impl :170-197 |
| `reopen` chooses transport by ref prefix | src/lanes/runner.rs:59, `transport_for` :118 |
| `thread_effect` accepts Slack refs; Discord thread-owner lookup only for numeric ids | src/lanes/runner.rs:210-232 |
| Runner `from_env` builds Discord+Slack router; Slack-only boot tolerated on `MissingToken` | src/lanes/runner.rs:1025-1047 |
| `lane start --thread` takes a string (Discord id or `slack:C…`/`C…`), validation, lead lookup by Slack channel | src/lanes/cli.rs:95, :316, :354 |
| CLI thread creation + bot binding via shared helper | src/lanes/cli.rs:396, binding :470 |
| Queued-lane launch creates Slack thread | src/lanes/guard.rs:608 (`create_lane_thread`), call :733 |
| Orphan thread: Discord archive / Slack starter `chat.delete` | src/lanes/guard.rs:653 (`discard_orphan_thread`), calls cli.rs:448, guard.rs:764 |
| Bot binding for Slack lanes (`bind_lane_thread` with auth.test user id) | cli.rs:470, guard.rs:783 (unchanged callers, now fed the Slack bot id) |
| Contract 13: kibitzer lead accepts Slack owner DM (`platform=="slack"`, no guild/thread, `user_id==slack::owner_id()`, bot present) | src/lanes/kibitzer.rs:105, used :144 |

Unit tests: src/lanes/slack_threads.rs tests (ref parsing, starter text, create, status+reactions, close/reopen, lazy auth.test, post conversion/no pings, edit not-found mapping, delete, router dispatch, outbox drain into Slack thread, owner reply interception) and src/lanes/kibitzer.rs:589.

## Not closable in this scope (for the lead)

- `src/inbox/proposals.rs:169` (`parent_channel_id: Option<u64>`) and `:238` (`thread: self.parent_channel_id`): `StartArgs.thread` is now `Option<String>`. Needs `.map(|id| id.to_string())` at :238 (or make the field `Option<String>` to allow a Slack parent). Owner: L-inbox-cron.
- Trait impls in integration tests still use `u64`; change to `&str`:
  - tests/test_lane_close.rs:36, :50 (`LaneLifecycle::status`/`archived`)
  - tests/test_fix_r5_lanes.rs:34, :43 (`LaneLifecycle`)
  - tests/test_fix_fix_lanes.rs:76 (`PostSink::rename_status`). No other u64 trait overrides exist in tests/.
- Slack owner-reply interception call site in `src/slack/ingress.rs` (before `map_event`): L-ingress (contract 2 is provided).
- Default lead fallback to a Slack DM (`main.rs`, `cron/health.rs`, `inbox/proposals.rs:197`, `multiplexer/actor.rs:1180` still read Discord `kibitzer::DEFAULT_LEAD`): L-main per plan "Home / lead".
- `OMON_SLACK_API_BASE` loopback for lanes QA: optional row; not added (no contract for it).
- Archive on Slack: no API equivalent; replaced with a final "lane closed" reply plus ✅ starter/reaction (by design).

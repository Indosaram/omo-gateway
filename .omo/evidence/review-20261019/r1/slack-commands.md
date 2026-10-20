# Lane slack-commands

Ask: review the `src/slack/commands.rs` changes since `e57b472`, read-only - wanted: findings rated by severity, each checked against the current file. For you: I reviewed every hunk of `slack-commands.diff` (2276 added lines, about half of them tests) and found 2 P1s and 1 P2. Both P1s have the same cause: in a channel, slash commands act on the channel-level session, but channel conversations run in thread sessions. I traced each cited line in the current files. Nothing was edited or run (no cargo, bun or npm). The uncommitted changes in `omo_backend.rs` remove only the automatic progress steer that the backend injected on its own. They do not touch `multiplexer.steer`, so `/omo-steer` is unaffected. Now: none - the report is complete. Next: none.

The P1s come from wiring that main.rs uses in production: `main.rs:1678`, then `runtime.rs` `SocketEvent::SlashCommand` calls `SlackCommands::handle`.

### [P1] Channel slash commands create a channel-level session, and the bot then answers every message in that channel
- Location: src/slack/commands.rs:436
- Trigger: The owner (an allowed user) runs `/omo-model opus` or `/omo-yolo on` at the top level of an ordinary channel, say `#general`. That channel is not a free-response channel, has no cron or lane session, and `THREAD_REQUIRE_MENTION` is at its default of false. Afterwards, any allowed or paired user posts a top-level message there without @mentioning the bot.
- Impact:
  - `session_key` builds the channel-level key `(team, channel, None, "")`.
  - `execute_model_command` and `yolo_toggle` call `ensure_session`, which inserts a `sessions` row for that key (discord/commands.rs:792 and :1343). `/omo-steer` with no running turn, `/omo-retry` and `/omo-skill run` also route turns to that same key.
  - Ingress `known_thread` treats an existing channel-level session as "a channel the bot reports into" (ingress.rs:142-153). `map_event` then admits unmentioned top-level messages, because `implicit_thread = known_thread && !config.thread_require_mention` (ingress.rs:927-928).
  - Result: one model or yolo slash command quietly turns the channel into a free-response channel. The bot starts replying to every allowed user's message, and that state survives restarts.
- Evidence:
  ```rust
  } else {
      SessionKey::new(
          PLATFORM,
          Some(invocation.team_id.clone()),
          invocation.channel_id.clone(),
          invocation.thread_ts.clone(),
          "",
      )
  ```
  and in ingress.rs:144-153:
  ```rust
  if reply_thread_ts(event).is_none() && !is_dm(event, channel) {
      let key = SessionKey::new(PLATFORM, Some(self.team_id.as_str()), channel, None::<String>, "")
          .with_bot_id(&self.shared.bot_user_id);
      return session_exists(&self.shared.pool, &key).await;
  ```
- Fix: In `execute`, reject state-creating commands (`model`, `yolo`, `steer`, `retry`, `skill run`) when the invocation is in a channel with no thread and no channel-level session already exists, and reply with a private message saying they only work in an existing conversation. The more complete fix is to resolve the target conversation explicitly instead of defaulting to the channel key; that is a design change.

### [P1] `/omo-stop`, `/omo-deny` and `/omo-yolo` in a channel miss the running conversation
- Location: src/slack/commands.rs:610
- Trigger: `SLACK_AUTO_THREAD` is at its default (true; slack/mod.rs:149). The owner @mentions the bot at the top level of `#work`. Ingress keys that turn as a thread session (`thread = reply_thread.or_else(|| config.auto_thread.then_some(ts))`, ingress.rs:934). The turn then asks for approval or runs long, and the owner types `/omo-deny` or `/omo-stop` in the channel.
- Impact:
  - The slash command builds the key from `invocation.thread_ts`. A top-level invocation has none, so the key is always the channel key, never the thread session ingress created.
  - `/omo-deny` replies "No pending approval found to deny." while the approval is still pending.
  - `/omo-stop` replies "No active turn running; session marked suspended." while the turn keeps running.
  - `/omo-yolo` changes a different session than the one the user is looking at.
  - The same happens inside the thread if Slack's slash payload lacks `thread_ts` (see the P2 below).
  - Net effect: the user is told the wrong status, and the stop and deny controls do nothing for channel conversations.
- Evidence:
  ```rust
  let key = session_key(invocation, &self.bot_user_id);
  ```
  and commands.rs:740:
  ```rust
  .resolve_session_deny(&key, reason.clone())
  ```
  This only matches pending entries where `entry.session == Some(session)` (discord/approval.rs:812).
- Fix: For `Deny` and `Stop` invoked in a channel without `thread_ts`, fall back to the single pending approval or active turn among the sessions for this channel and bot (same channel id and bot id, any thread id). If more than one matches, reply privately with "run this inside the thread". At minimum, change the "No pending approval" and "No active turn" replies so they say the command only targets the channel-level session.

### [P2] `/omo-title`, the in-thread `/omo-thread` rejection and thread-scoped keys depend on an unverified `thread_ts` in the slash payload
- Location: src/slack/commands.rs:137
- Trigger: `/omo-title New` run inside a thread created by `/omo-thread`.
- Impact: Every thread-aware branch depends on `thread_ts` being present in the slash payload. The repo's own parity audit (`.omo/evidence/slack-parity/commands.md:88`) says Slack slash payloads have historically not included `thread_ts`, and that this has not been checked against a live payload. If Slack omits it, `/omo-title` always replies "can only be used inside a thread", and nested `/omo-thread` creates a new root message instead of being rejected. The tests inject `thread_ts` by hand (commands.rs:1606, :1846, :1969), so they would not catch this. I rated it P2 because I could not confirm from inside the repo what the payload actually contains.
- Evidence:
  ```rust
  thread_ts: optional("thread_ts"),
  ```
- Fix: Capture one real `slash_commands` envelope sent from inside a thread. If `thread_ts` is missing, remove `/omo-title` from `COMMANDS` and the manifest, or document that it doesn't work, rather than shipping a command that cannot succeed.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/slack/commands.rs - file under review; `session_key` (436), the `execute` key (610), deny (740), thread and title (1012-1090), `thread_ts` parsing (137)
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - `known_thread` (142-153), the implicit-thread gate in `map_event` (927-928) and the auto-thread key (934); shows the key mismatch
- /Users/indo/code/project/omon-gateway/src/discord/commands.rs - `ensure_session` is called from `execute_model_command` (792) and `yolo_toggle` (1343)
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - `resolve_session_deny` matches on the exact session key (808-812)
- /Users/indo/code/project/omon-gateway/src/slack/mod.rs - defaults: `auto_thread` true (149), `thread_require_mention` false (156)
- /Users/indo/code/project/omon-gateway/src/slack/runtime.rs - production dispatch from `SlashCommand` to `SlackCommands::handle`
- /Users/indo/code/project/omon-gateway/src/main.rs - `SlackCommandDeps` wiring (1678)
</files>
<answer>2 P1s and 1 P2. Both P1s come from the slash command key: in a channel without `thread_ts` it is always the channel-level session, but channel conversations run in auto-thread sessions. Model and yolo commands therefore create a channel session that makes ingress answer every unmentioned message in the channel. Stop and deny miss the thread conversation's turns and approvals. The P2: whether Slack's slash payload contains `thread_ts` is unverified, and `/omo-title` depends on it.</answer>
<next_steps>Fix the session targeting in `execute` (commands.rs:610) and check a live slash payload for `thread_ts`.</next_steps>
</results>
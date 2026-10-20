# Lane slack-interactive

Ask: review the slack-interactive diff since e57b472 (src/slack/buttons.rs, src/slack/interactive.rs, src/tools) - wanted: findings with a reachable trigger and correct severity. For you: all 6 file hunks in the diff are reviewed (buttons.rs, interactive.rs, tools/cron.rs, tools/message_context.rs, tools/message_context_slack.rs, tools/mod.rs). There are no P0 or P1 findings, and two P2 findings, both in the Slack message_context provider. Now: report delivered. Next: none.

### [P2] get_message cannot fetch a thread reply
- Location: src/tools/message_context_slack.rs:262
- Trigger: In production, `main.rs:1336` registers `SlackMessageContextProvider`. The agent calls `message_context` with `{"operation":"get_message","message_id":"<ts of a thread reply>"}`, using an id it got from a `replies` result or a thread-session `recent` result.
- Impact: Slack's `conversations.replies` with a reply's ts returns the thread starting from its parent message. With `limit` set to 1, only the parent comes back. The `find(ts == requested)` lookup then fails, so the tool reports "Slack message ... not found" for every thread reply. Only top-level messages and thread roots resolve. The agent gets an error instead of the message. It does not cause a wrong or missing chat reply, so this stays P2.
- Evidence:
  ```rust
  .conversations_replies(&conversation.channel, ts, None, 1)
  .await?
  .iter()
  .find(|message| str_field(message, "ts") == ts)
  ```
- Fix: Either raise the limit, or call `conversations_history(channel, Some(ts), Some(ts), 1)` with `inclusive: true` for top-level messages and fall back to `conversations_replies(channel, ts, Some(ts)-ish, ...)`. The simplest version: pass `PAGE_LIMIT as u32` instead of `1`, keeping the existing `find`. That covers replies within the first 200 messages of a thread.
- Basis: the parent-first behaviour comes from Slack API semantics, not from a run. The unit test fake only returns `body["ts"]` as the root, so it does not cover this case.

### [P2] Thread reads only see the oldest 200 messages, so "recent" in long threads returns old messages
- Location: src/tools/message_context_slack.rs:184
- Trigger: There is a Slack thread session (or a `replies` call) on a thread with more than 200 messages. The agent calls `message_context` `recent` or `replies`.
- Impact: `conversations.replies` returns messages oldest-first and is called with no cursor. The provider therefore only ever sees messages 1-200, then reverses and truncates them. "Recent" returns the oldest messages in the thread, `before_message_id` paging cannot reach anything newer, and search backfill never indexes the tail. This needs a thread with more than 200 messages, so it stays P2.
- Evidence:
  ```rust
  .conversations_replies(
      &conversation.channel,
      thread_ts,
      None,
      PAGE_LIMIT as u32,
  )
  .await?;
  replies.retain(|message| {
      before.is_none_or(|before| ts_less(str_field(message, "ts"), before))
  });
  replies.reverse();
  replies.truncate(limit as usize);
  ```
- Fix: Follow `response_metadata.next_cursor` until the thread is exhausted (or a bounded cap is hit) before applying the `before` filter and reversing. This needs a cursor parameter on `SlackApi::conversations_replies`. Alternatively, pass `latest = before` so the newest window is fetched.

### Reviewed with no finding
- **interactive.rs:** approval clicks are restricted to `config.owner_id()` (and `is_allowed`) and fail closed when the user is empty. `parse_custom_id` rejects extra segments. A failed `resolve_custom_id` replaces a stale card rather than resolving anything. The WIP egress change keeps `mark_approval_decided` and the `approvals` map intact (`src/slack/egress.rs:146`, `:636`). Routing goes through `runtime.rs:142`.
- **buttons.rs:** the click must come from an allowed user (an empty list allows nobody). `action_ts` LRU dedupe and the `IN_FLIGHT` per-message lock (`:199`, released at `:292`) prevent double runs. The failure path restores the original blocks, and the reply is escaped before it is threaded under the button message.
- **tools/cron.rs:313-326:** a Slack-session cron create defaults `deliver` to `slack:<channel>[:<thread_ts>]`. That format is parsed correctly by `cron/store.rs:923` `slack_destination` (a ts has no `:`).
- **tools/message_context.rs and mod.rs:** visibility-only changes.

All cited lines were re-read in the current files.
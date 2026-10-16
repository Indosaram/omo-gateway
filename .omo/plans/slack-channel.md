# Slack channel for omo-gateway (Discord parity)

Started 2026-10-17. Tier HEAVY: new external integration and a new channel module, plus cross-cutting routing changes.
Worktree: `/Users/indo/code/project/omon-gateway-wt/slack`, branch `slack-channel`, base commit `4969507`.

## Established facts

- **Slack app.** "OmOMeow" was created from `docs/slack-app-manifest.json` with Socket Mode on.
  - App ID `A0CA2QMJQN8`, team `T04LYPT5VN1`.
  - Bot user `U0C9ST69M8B`, bot id `B0C9X5R26DU`.
- **Tokens.** Both are 0600 files and both were checked against the live API.
  - `~/.omon/secrets/slack-bot-token` (xoxb-): `auth.test` ok.
  - `~/.omon/secrets/slack-app-token` (xapp-): `apps.connections.open` ok.
- **Owner.** Slack user id `U04M1JNJM8T`.
- **Manifest contents.**
  - Slash commands: `/omo-<name>` for model, reset, stop, status, tools, skill, skills, memory, cron, steer, undo, retry, compress, title, thread, deny, yolo, sessions.
  - Bot events: `app_mention`, `message.channels`, `message.groups`, `message.im`, `message.mpim`.
  - Interactivity is enabled.

## Design

### Session key

| Field | Value |
|---|---|
| `platform` | `"slack"` |
| `guild_id` | team id in channels, `None` in DMs |
| `channel_id` | the Slack channel id (C…, D…, or G…) |
| `thread_id` | `thread_ts` when the conversation is threaded |
| `user_id` | the user (U…) in DMs |
| `bot_id` | the bot user id |

- A channel @mention starts a thread rooted at the mention's ts. This matches Discord's auto-thread behavior.
- A top-level DM message belongs to the DM session itself, with no thread.

### New module `src/slack/`

Each file is one implementation lane.

- **`api.rs`: Web API client** (reqwest).
  - Methods: `chat.postMessage`, `chat.update`, `chat.delete`, `reactions.add`/`reactions.remove`, external file upload, `conversations.info`, `users.info`, `auth.test`, `apps.connections.open`, `chat.postEphemeral`.
  - On HTTP 429, waits for `Retry-After` and retries.
  - Treats `ok:false` as an error.
- **`socket.rs`: Socket Mode client** over tokio-tungstenite.
  - Acks every `envelope_id` immediately.
  - Handles `hello` and `disconnect`, and reconnects with backoff.
  - Emits typed events on an mpsc channel and stops through a `CancellationToken`.
- **`mrkdwn.rs`: Markdown to Slack mrkdwn.**
  - Converts bold, links, headings, and strikethrough.
  - Escapes `&<>` outside code.
  - Splits messages at 3900 characters on line boundaries without breaking code fences.
- **`ingress.rs`: Events API event to `InboundEvent`.**
  - Ignores bot and self messages, edits, deletes, and unhandled subtypes.
  - Gates senders through the owner allowlist.
  - Rules for channels:
    - A message without a mention is ignored unless the channel is free-response or the thread already belongs to a bot session.
    - The mention text is stripped from the prompt.
  - Dedupes through `DeliveryLedgerService` with the key `slack:<channel>:<ts>`.
  - A `/stop` message calls `stop`. Otherwise it steers into the active turn first and falls back to `route`.
  - Downloads attached files and adds a 👀 reaction.
- **`egress.rs`: `SlackEgress` as `OutboundDispatcher`.**
  - Message actions: SendMessage, EditMessage, DeleteMessage.
  - UploadFile uses the external upload flow.
  - Stream posts a placeholder first, then throttles `chat.update` to at least 1.2 s apart. The final chunk carries the final text; overflow becomes follow-up messages. A silence response deletes the placeholder.
  - Typing is a no-op.
  - React maps emoji to Slack names. `remove_others` removes the other status reactions.
  - ApprovalRequest posts Block Kit buttons whose value is `omon:approval:<uuid>:<choice>`. ExpireApproval updates that message.
- **`interactive.rs`: approval buttons.**
  - Accepts clicks only from allowed users.
  - Calls `resolve_custom_id`, then updates the message to show the decision.
- **`commands.rs`: the `/omo-*` slash commands**, behaving like the Discord commands.
  - Reuses the `execute_*` and helper functions.
  - Includes the session-control commands.
  - Replies through `response_url`.
- **`mod.rs`.**
  - `SlackConfig::from_env()` defaults to the secrets files plus the owner id. The kill switch is `OMON_SLACK_ENABLED=false`.
  - `SlackRuntime::spawn` supervises the runtime with restart backoff.

### Cross-cutting changes

- A platform-routing dispatcher picks the Discord or Slack egress by `session.platform`.
  - It is used everywhere the Discord egress was used before: the shared dispatcher, approvals, cron, the session watcher, and lanes.
- Cron delivery supports `slack:<channel>[:<thread_ts>]` and origin platform `slack`.
  - `deliver()` stops parsing ids as u64 for Slack.
  - The cron tool auto-fills Slack origins.
- Owner-DM and lead detection plus actor message indexing also accept `platform == "slack"` with the Slack owner. This covers `status_reply`, `usage`, `is_owner_lead_session`, and `actor` indexing.
- `main.rs` starts Slack only when it is configured. A Slack failure never stops Discord.

## Verification (run once, after every lane is merged)

1. Static checks and tests:
   - `cargo build` and `cargo clippy --all-targets -- -D warnings`.
   - Focused tests for mrkdwn, ingress, egress (against a fake HTTP server), socket ack (against a fake WS server), and cron Slack destinations.
   - The full `cargo test`.
2. Live: install the release build with `scripts/deploy.sh`, then check each of these:
   - (a) The gateway log shows Slack connected.
   - (b) A DM from the owner to the bot, sent with `agent-slack` (user token), gets a bot reply.
   - (c) A channel @mention gets a threaded reply.
   - (d) The 👀 → ✅ reactions appear.
   - (e) The approval button round trip works.
   - (f) Discord is still healthy (deploy.sh verify).

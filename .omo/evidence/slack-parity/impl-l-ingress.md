# L-ingress implementation: Slack ingress parity

Scope: `src/slack/{ingress,debounce,backfill,lifecycle,socket}.rs`. `src/slack/mod.rs` was not touched: L-main had already added `pub mod debounce; pub mod backfill; pub mod lifecycle;`.
cargo was not run, per lane rules. Every file passes `rustfmt --edition 2021`. Compile, clippy and tests run in the final integration gate.

## Closed rows (adapter-ingress.md, plus inbox-cron-sessions.md rows 6, 27 and 40)

| Gap | Implementation |
|---|---|
| Inbox triage: "제안" list plus "<n> 해" approval (inbox row 6) | src/slack/ingress.rs:241. Requires `triage::enabled()` and `user == config.owner_id()`, and maps the author to `proposals::OWNER_ID`. The list is packed into `MESSAGE_LIMIT`-sized chunks, and each chunk calls `presented(n, channel, ts)`. The approval reference is `thread_ts`; the reply is posted in the same thread. |
| Session-status interception (inbox row 27) | src/slack/ingress.rs:192: `status_reply::owner_dm` + `multiplexer.intercept_session_status` |
| Lane owner-reply interception before `map_event` (contract 2) | src/slack/ingress.rs:151 → src/slack/ingress.rs:696. Calls `lanes::slack_threads::intercept_owner_reply`, then `lanes::runner::wake`. |
| `referenced_message_id` = `thread_ts` | src/slack/ingress.rs:929 |
| Reply context and parent attachments (thread root unknown) | src/slack/ingress.rs:456. Fetches the root with `conversations_replies(limit 1)`, applies `compose_reply_context`, and unions the root's files into the attachments. |
| Forwarded and shared messages (`attachments[].is_share` / `is_msg_unfurl`) | src/slack/ingress.rs:947. Produces a `[Forwarded]` block plus the shared files. |
| Channel topic context | src/slack/ingress.rs:397. `conversations_info` topic (falling back to purpose), passed to `format_channel_topic_context`. |
| Recent channel context on a mention (limit ≤25) | src/slack/ingress.rs:418. Uses `conversations_history(latest=ts)`, or `conversations_replies` inside a thread, then `format_channel_context`. Names come from `users_info` and are cached at src/slack/ingress.rs:505. |
| Text-file inlining (≤100 KB) | src/slack/ingress.rs:630 |
| Audio transcription (inbox row 40) | src/slack/ingress.rs:625. Calls `voice_transcription::is_audio_attachment` / `transcribe_audio` and prepends the "🎤 [음성 전사]" prefix, or appends a failure note in Slack italics. |
| `allow_bots` None/Mentions/All | src/slack/ingress.rs:857 |
| Allowed and ignored channel lists | src/slack/ingress.rs:938 (DMs are exempt from the allowlist; ignored always rejects) |
| `thread_require_mention` | src/slack/ingress.rs:894 |
| No auto-thread in free channels | src/slack/ingress.rs:900 |
| `allow_all_users` and paired users admitted | src/slack/ingress.rs:857 (block after it) |
| Text `/stop`: cancel the debounce batch, then `discord::commands::stop_session` (with_approvals) | src/slack/ingress.rs:200 |
| Unauthorized-DM pairing prompt (`SlackPairingStore`, throttled) | src/slack/ingress.rs:342 |
| `processing_reactions` toggle | src/slack/ingress.rs:214 |
| `slack_cursors` write after routing (forward-only, ts-ordered) | src/slack/ingress.rs:778; called at src/slack/ingress.rs:596 and after a successful steer |
| Split-message debouncer (600 ms, ts-sorted, per-session ordering, constituents) | src/slack/debounce.rs:59, src/slack/debounce.rs:160; sink at src/slack/ingress.rs:574 uses `record_incoming_with_constituents` (src/slack/ingress.rs:582) |
| Debounce flush on shutdown | src/slack/ingress.rs:104 → src/slack/debounce.rs:103 (L-main's `runtime.flush()` already calls it) |
| Builders (contract 5): `with_inbox_actions`, `with_approvals`, `with_pairing`, `flush` | src/slack/ingress.rs:79 onward |
| `backfill::run(&SlackIngress) -> Result<usize>` (contract 6) | src/slack/backfill.rs:20. Targets: stored cursors, free channels, owner DM (`open_im`) and known thread sessions; ignored channels are dropped. Replays since the cursor with `route_awaiting_turn` (src/slack/ingress.rs:529), skips `/stop`, and holds the cursor when a turn fails (src/slack/backfill.rs:134). A target with no cursor is seeded at its newest message, so its whole history is never replayed. |
| Lifecycle: `channel_archive`/`group_archive`/unarchive, `channel_deleted`, `message_deleted` of a thread root | src/slack/lifecycle.rs:18, src/slack/lifecycle.rs:44; called first in `handle` (src/slack/ingress.rs:148). The SQL is identical to Discord `handle_thread_update`/`handle_thread_delete` (`$.metadata.thread_archived` / `json_remove $.metadata.omo_thread_id`), and the turn is stopped first. |
| Socket receive watchdog (`OMON_SLACK_RECEIVE_WATCHDOG_SECS`, default 1200) | src/slack/socket.rs:21; `tokio::time::timeout` around `ws.next()` at src/slack/socket.rs:167 |

## Tests written (in-file, real behavior; not run under the lane rules)

- `debounce.rs`: ts ordering, dedupe and constituents; per-session independence; cancel plus `flush_all`. Timer tests use `start_paused`.
- `lifecycle.rs`: event classification; archive, unarchive, root-delete and channel-delete against `sqlite::memory:` (other bots and other channels stay untouched).
- `backfill.rs`: fake axum Slack. Replays after the cursor, skips `/stop`, holds the cursor on a failed turn, catches up thread replies, and seeds a target that had no cursor.
- `socket.rs`: watchdog env parsing; a silent socket is dropped by the watchdog and the client reconnects.
- `ingress.rs`: mapping rules (channel lists, bots, pairing, thread_require_mention, free-channel auto-thread, forwarded shares); dedupe and reaction; constituent coalescing; `/stop` cancel; reply, topic and channel context with the user-name cache; text inlining via a real download; pairing prompt throttle; lane owner-reply consumption; proposal list and approval in a thread.

## Not closable in this lane (reasons)

| Row | Reason |
|---|---|
| Cursor rewind for queued messages dropped at shutdown (actor.rs:31-80) | `src/multiplexer/actor.rs` belongs to L-main. The Slack branch should rewind `slack_cursors` (`last_ts` < the dropped ts). |
| Manifest `bot_events` for lifecycle (`channel_archive`, `channel_deleted`, `group_archive`, `message` subtypes) | `docs/slack-app-manifest.json` belongs to L-commands. The code handles these events once they are subscribed. |
| Generalizing Discord's `SplitMessageDebouncer` | Out of scope (`discord/adapter.rs`). Slack has its own debouncer that reuses `discord::adapter::coalesce_inbound_events`. |
| Slack user-group roles (Discord `allowed_roles`) | Not in the contract-1 fields (no `allowed_roles`); admission uses allow-list, `allow_all_users` and pairing. |
| Forwarded payload shape and audio clip mimetypes | Implemented from the documented fields (`is_share`/`is_msg_unfurl`, `audio/*`); still needs confirmation in live QA. |

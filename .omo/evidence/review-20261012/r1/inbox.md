# inbox review

### [P1] Inbox event send blocks the Discord inbound path when triage backs up
- Location: src/inbox/watchers.rs:30
- Trigger: `on_gateway_message` does `sender.send(InboxWatcherEvent::GatewayMessage).await` on a bounded channel (`DEFAULT_EVENT_CHANNEL_CAPACITY` = 64). It is awaited inline from `MessageActor::persist_inbound` (src/multiplexer/actor.rs:905) and from the steer path (src/discord/adapter.rs:1476). The chain that fills the channel:
  - `Triage::serve` handles `collected` inline. `flush(now, false)` in src/inbox/triage.rs loops over every full 20-item chunk in one call.
  - Each chunk makes up to 20 sequential judge calls plus a writer turn with a 180 s timeout.
  - So a backlog of a few hundred items (for example the first Discord collection, which starts from an empty cursor and reads all indexed history) keeps `serve` from draining `progress_rx` (capacity 16) for many minutes.
  - `run_event_watch_with_shutdown` then blocks on `tx.send(results).await` (watchers.rs:143) and stops draining `event_rx`.
  - Discord messages, iMessage and mail watcher events, and the 120 s Periodic tick then fill the 64-slot channel.
- Impact: the session actor (and the adapter steer path) blocks inside `persist_inbound` before running the user's turn. Discord replies stall until triage finishes. An optional background feature ends up back-pressuring the chat path. `try_on_gateway_message` already exists, is unused, and is the non-blocking variant.
- Fix: use `try_send` for the gateway-message path, treating a full channel as already pending (`GatewayMessage` is idempotent). Call `try_on_gateway_message` from actor.rs and adapter.rs and ignore `TrySendError::Full`. Also use `try_send` for `progress_tx` in watchers.rs:143, or drop stale results when it is full.

### [P1] Timed-out source CLI child processes are never killed and accumulate
- Location: src/inbox/sources/mod.rs:46
- Trigger: `run_json` runs `Command::new(program)...output()` without `.kill_on_drop(true)`. It is called by `MailSource::fetch_since` (`zele mail list`) and `IMessageSource::fetch_since` (`imsg chats`, then `imsg history` per chat). `collect_all` (src/inbox/mod.rs:139) wraps the fetch in `tokio::time::timeout(60s)`. If `zele` or `imsg` hangs (network stall, auth prompt, locked Messages DB), the timeout drops the future, but a tokio `Child` is not killed on drop. The hung process keeps running. The next mail or iMessage event, or the next Startup/GatewayMessage fallback, spawns another one.
- Impact: leaked, hung `zele`/`imsg` processes pile up, along with their file handles and DB locks. For `imsg` they may keep contending on the Messages DB, so later collections also time out. The same repo uses `kill_on_drop(true)` for the equivalent timeout case (src/cron/ack.rs:20-36).
- Fix: add `.kill_on_drop(true)` to the `Command` in `run_json`.

### [P2] Digest "login required" detection matches any error containing "401" or "login"
- Location: src/inbox/digest.rs:87
- Trigger: the query selects `inbox_cursors` rows where `error LIKE '%401%'` or `lower(error) LIKE '%login%'`. Collector errors embed arbitrary text, and `record_source_error` stores them. Example: "imsg history missing for chat 4012" (src/inbox/sources/imessage.rs), or "PirateTalk GET messages for room <chatId> failed" where the numeric `chatId` contains `401`. `run_json` errors also embed raw stderr. `digest::build` is called from `DigestExecutor::execute` at the 09:00 KST run.
- Impact: the owner DM digest reports "- [source] login required" for a non-auth failure. It also makes `build` return `Some(..)` when it would otherwise return `None` (digest.rs:93), so a spurious digest is sent.
- Fix: match only auth markers the collector emits. Use `error LIKE 'auth_expired%'` or `LIKE '%auth_expired:%'`, plus an explicit "returned 401"/"unauthorized" check. Better, store a structured auth flag or error kind instead of substring-matching free text.

# Lane slack-transport

### [P1] Periodic reconcile replays owner lane-thread replies the live path already handled, starting an extra agent turn

- Location: src/slack/runtime.rs:100
- Trigger: This is reachable through the main.rs wiring: main.rs:1841 spawns `reconcile`.
  1. While a Slack lane is running, the owner replies in its bot-rooted lane thread. `intercept_lane_reply` returns false because the lane is not closed and has no pending reply. The message is routed as a normal Slack thread turn, which creates a `sessions` row and sets the cursor for `(channel, lane_ts)`.
  2. The lane later reaches `closed` or `verified_done`. The owner replies in the same thread, for example to reopen it.
  3. Live ingress hands that reply to the lane: `record_owner_reply` returns true, so `handle` returns at ingress.rs:172-173. This path neither advances the cursor nor writes a delivery-ledger row.
  4. The next reconcile pass, 60-180 s later, targets the thread because its session row exists (backfill.rs:56-67). It reads replies after the cursor and finds the reply. `map_replayed` then accepts it: it comes from the owner, `known_thread` is true, and no mention is required. It routes the reply through `route_awaiting_turn`, which bypasses `intercept_lane_reply`.
- Impact: One owner reply triggers both the lane reopen/close and a separate agent turn that answers in the lane thread (an unwanted extra reply). Before this change, backfill ran only at startup, so it never saw messages that had already been handled live. The periodic sweep is what creates this replay. Commit c943813 fixed the same problem for proposals and session-status commands, but not for this path.
- Evidence:
  - runtime.rs:100: `match super::backfill::run_settled(&self.ingress, Some(RECONCILE_SETTLE)).await {`
  - ingress.rs:172-173: `if self.intercept_lane_reply(&event).await? { return Ok(()); }`
  - backfill.rs:149-150: `if let Some(inbound) = ingress.map_replayed(&event, known_thread) { match ingress.route_awaiting_turn(inbound).await {`
- Fix: When `intercept_lane_reply` returns true, advance the cursor before returning, as c943813 did for proposals. The event is not mapped yet at that point, so call `write_cursor(&s.pool, &s.bot_user_id, channel, thread_ts, ts)` inside `intercept_lane_reply` just before `Ok(true)` (ingress.rs:768).

### [P2] Startup backfill and the first reconcile pass can overlap, letting a failed replayed turn be skipped

- Location: src/slack/runtime.rs:91
- Trigger: main.rs:1840-1841 spawns `backfill()` and `reconcile()` at the same time with no ordering. If the startup replay of message M is still waiting on its turn after 120 s (`route_awaiting_turn` blocks until the turn ends), reconcile reads the old cursor. M's claim is still `in_progress`, so reconcile gets `Ok(false)` and moves the cursor past M (backfill.rs:152-159). If M's turn then fails, the startup pass holds its cursor, but the cursor has already been moved past M, so M is never retried.
- Impact: This breaks the "failed turn is retried" guarantee in backfill.rs:6-7. It is rated P2 because it needs both a long replay and a turn failure, and the owner is already told when a turn fails.
- Evidence: `pub async fn reconcile(self: Arc<Self>, shutdown: CancellationToken) {` / `_ = tokio::time::sleep(RECONCILE_INTERVAL) => {}`, and in main.rs `tokio::spawn(async move { backfill.backfill().await }); tokio::spawn(runtime.clone().reconcile(lane_shutdown.clone()));`
- Fix: Run the startup backfill at the start of `reconcile`, then enter the loop, and drop the separate spawn at main.rs:1840. Each pass then finishes before the next one starts.

### [P2] Slack `allow_all_users` falls back to `DISCORD_ALLOW_ALL_USERS`

- Location: src/slack/mod.rs:167
- Trigger: An operator sets `DISCORD_ALLOW_ALL_USERS=true` to open up Discord and does not set the Slack variable.
- Impact: Every member of the Slack workspace is admitted to DMs (ingress.rs:900) and becomes a slash-command "operator" who can approve `/omo-pair` codes (commands.rs:584). This contradicts the doc comment, which says access-scoping settings are Slack-only. It is not currently set in .env or the launchd plist, so it depends on configuration (P2).
- Evidence: `allow_all_users: flag(lookup, "ALLOW_ALL_USERS", false),`, where `setting()` falls back to `DISCORD_{name}`.
- Fix: Read the Slack variable only: `allow_all_users: parse_flag(non_empty(lookup("SLACK_ALLOW_ALL_USERS")).as_deref(), false),`

Reviewed with no findings: all of api.rs (form vs JSON routing, 429 retry, download cap and HTML check, token redaction), home.rs, lifecycle.rs (classification and SQL scoped to this bot), socket.rs (ack before forwarding, watchdog, reconnect and backoff), and `serve` / `supervise` in runtime.rs.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/slack/runtime.rs - the reconcile loop (line 91, run_settled at line 100) behind the P1 and P2 findings
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - intercept_lane_reply returns early without advancing the cursor (lines 172-173, function ends at line 768)
- /Users/indo/code/project/omon-gateway/src/slack/backfill.rs - replay path at lines 142-160 that bypasses lane interception
- /Users/indo/code/project/omon-gateway/src/main.rs - lines 1840-1841 spawn backfill and reconcile at the same time
- /Users/indo/code/project/omon-gateway/src/slack/mod.rs - line 167, the allow_all_users fallback to Discord
- /Users/indo/code/project/omon-gateway/src/storage/lanes.rs - record_owner_reply (line 296) decides when a reply counts as handled by the lane
</files>
<answer>One P1: the periodic reconcile replays owner replies in lane threads that intercept_lane_reply already handled, because that path never advances the cursor, so the agent answers in the thread on top of the lane action. Two P2s: startup backfill and reconcile can overlap, and the Slack allow_all_users setting falls back to the Discord one.</answer>
<next_steps>Add write_cursor before Ok(true) in intercept_lane_reply (ingress.rs:768). Optionally run the startup backfill inside reconcile, and make allow_all_users read only the Slack variable.</next_steps>
</results>
# Lane slack-ingress

I reviewed every hunk of `.omo/evidence/review-20261019/diffs/slack-ingress.diff` (backfill.rs, debounce.rs, ingress.rs), plus the uncommitted WIP hunk in `src/slack/ingress.rs:765-769` that writes the cursor after a lane reply. Each cited line was re-checked against the current file. All production paths are wired in `src/main.rs:1840-1841`: startup backfill runs, then a reconcile loop runs every 120s with a 60s settle (`src/slack/runtime.rs:14-16, 91-110`). Both are on by default (`missed_backfill` defaults to true, `src/slack/mod.rs:165`). `auto_thread` also defaults to true (`src/slack/mod.rs:149`).

### [P1] Reconcile re-runs a replayed turn the owner stopped (or that failed) every 2 minutes
- Location: src/slack/backfill.rs:153
- Trigger: The socket misses an owner DM, as in the 2026-10-19 incident. The next reconcile pass replays it with `route_awaiting_turn`, and the agent starts working. The owner sends `/stop`. The actor resolves the ack with `Err("stopped by user")` (`src/multiplexer/actor.rs:942-944`) and `interrupt_turn` marks the ledger row `failed`. Backfill breaks without moving the cursor. 120s later the next pass fetches the same message. `record_incoming_as` reclaims the `failed` row (`ON CONFLICT ... WHERE delivery_ledger.status = 'failed'`, `src/ledger/service.rs:777-781`) and the turn runs again. A turn that fails for good (quota, model error) loops the same way, and each round posts `owner_failure_text` ("⚠️ 응답을 만들지 못했어요 ...") again.
- Impact: `/stop` does not stick, so a stopped request restarts every 2 minutes. A persistently failing message spams failure notices and blocks every later message in that target. The loop only ends when a live message in the same session happens to move the cursor forward. Before this change, a held cursor was only retried at the next restart.
- Evidence:
  ```rust
  match ingress.route_awaiting_turn(inbound).await {
      Ok(true) => routed += 1,
      Ok(false) => {}
      Err(error) => {
          tracing::warn!(%error, channel, thread_ts, ts, "slack backfill turn failed; cursor held");
          break;
      }
  }
  ```
- Fix: Hold the cursor only when the turn was never admitted (for example `"gateway admission closed"` or a routing error before the ack). Once the turn reached a terminal outcome (stopped, failed, reset), call `write_cursor(pool, bot, channel, thread_ts, &ts)` and continue. The simplest version is to have `SlackIngress::route_awaiting_turn` return `Ok(true)` after a terminal turn error and keep `Err` for admission failures only.

### [P1] Top-level messages in free-response channels never advance the channel cursor, so reconcile re-runs turns that were stopped or failed live
- Location: src/slack/ingress.rs:781
- Trigger: The owner posts a top-level message in a free-response channel `CFREE` (auto_thread on, the default). `map_event` gives it the thread session `(CFREE, ts)`. `advance_cursor` therefore writes the cursor row `(CFREE, ts)`, never `(CFREE, "")`. The turn fails, or the owner `/stop`s it within about 1-3 minutes, so the ledger row becomes `failed`. The next reconcile walks target `(CFREE, "")` (`backfill.rs:45-46`) from its older cursor. `conversations.history` returns the message, and `route_awaiting_turn` reclaims the `failed` row and runs the turn again in the same thread.
- Impact: A message the owner explicitly stopped runs again by itself, and a failed turn is retried without being asked (duplicate or unwanted reply). DMs are not affected because their live session key `(D1, "")` matches the backfill target.
- Evidence:
  ```rust
  let thread = event.session.thread_id.as_deref().unwrap_or_default();
  if let Err(error) = write_cursor(
      pool,
      bot_id,
      &event.session.channel_id,
      thread,
      &event.platform_message_id,
  )
  ```
- Fix: In `advance_cursor`, also write the channel-level cursor when the message is the root of its own auto-thread: `if event.session.thread_id.as_deref() == Some(event.platform_message_id.as_str()) { write_cursor(pool, bot_id, &event.session.channel_id, "", &event.platform_message_id).await }`.

### [P1] Text `/stop` drops the debounced batch without moving the cursor, so reconcile replays the cancelled message
- Location: src/slack/ingress.rs:222
- Trigger: The owner DMs "do X" and then "/stop" within the 600ms debounce window, and sends nothing else for 2 minutes. `cancel` drops "do X" before any ledger claim. The `/stop` branch returns without `advance_cursor`. Reconcile then fetches `(D1, "")` from the old cursor. It skips "/stop" (`is_stop` in `route_awaiting_turn`), but "do X" is unclaimed and older than the 60s settle, so it is routed as a new turn.
- Impact: A request the owner cancelled is executed anyway 1-2 minutes later.
- Evidence:
  ```rust
  if is_stop(&inbound.content) {
      self.debouncer.cancel(&inbound.session).await;
      let interrupted = crate::discord::commands::stop_session(
  ```
- Fix: Call `advance_cursor(&s.pool, &inbound).await;` in the stop branch after `stop_session`. The `/stop` ts is newer than every cancelled event of the same session, so the cursor moves past all of them.

### [P1] Reconcile routes a missed lane-thread owner reply as an ordinary agent turn instead of the lane reopen/close path
- Location: src/slack/backfill.rs:149
- Trigger: The owner has replied live in a Slack lane thread once before. The WIP `write_cursor` at `ingress.rs:767` then creates the cursor row `(C, lane_ts)`, which makes that thread a backfill target. The lane closes again, the owner replies "reopen please", and the socket misses the event. Reconcile replays it. `is_known_thread` returns true because the history message has `parent_user_id == bot` (`rooted_by_bot`), and `map_replayed` plus `route_awaiting_turn` start an agent turn in session `(C, lane_ts)`. `intercept_lane_reply` never runs.
- Impact: The lane is not reopened, and the agent posts an unrelated reply in the lane thread. This hits exactly the incident case that reconcile was added for (missed owner replies). A missed owner-DM status question has the same gap: it is replayed as an agent turn rather than a status reply, which is milder.
- Evidence:
  ```rust
  let event = replay_event(message, channel, thread_ts, bot);
  let known_thread = ingress.is_known_thread(&event).await?;
  if let Some(inbound) = ingress.map_replayed(&event, known_thread) {
      match ingress.route_awaiting_turn(inbound).await {
  ```
- Fix: Before `map_replayed`, call `ingress.intercept_lane_reply(&event)` through a `pub(crate)` wrapper. On `Ok(true)`, write the cursor and `continue`.

### [P2] Channel backfill reads one page only, so more than 200 missed messages lose the oldest
- Location: src/slack/backfill.rs:107
- Trigger: More than 200 messages land in a free-response channel or the owner DM while the gateway is down. `conversations.history` returns the newest 200 after `oldest`. The loop then moves the cursor to the newest one, so the older messages are never fetched. `has_more` is ignored, because `messages()` in `api.rs:562` drops it.
- Impact: Missed messages are silently skipped. This is unlikely in a personal workspace.
- Evidence: `api.conversations_history(channel, cursor.as_deref(), None, PAGE_LIMIT)`
- Fix: Page with `latest` until `has_more` is false (or follow `response_metadata.next_cursor`) before processing.

### [P2] Reconcile work grows with every thread session ever created
- Location: src/slack/backfill.rs:54
- Trigger: Every auto-threaded mention creates a thread session. Each reconcile pass makes one `conversations.replies` call per Slack thread session of this bot, plus every cursor row, and nothing is ever pruned.
- Impact: Each pass slowly takes longer. It eventually hits Slack's per-method rate limit for `conversations.replies`, which `enrich` and the lane starter also use.
- Evidence: `"SELECT session_key FROM sessions WHERE platform = ? AND thread_id IS NOT NULL"`
- Fix: Limit targets to sessions or cursors with `updated_at` inside a recent window (for example 7 days).

### [P2] Debouncer per-session lane mutex map is never pruned
- Location: src/slack/debounce.rs:137
- Trigger: Every new `SessionKey` adds an entry (every auto-threaded top-level message makes a new key).
- Impact: Memory grows without bound, by a few hundred bytes per thread over the daemon's lifetime.
- Evidence: `.entry(session.clone()).or_default().clone()`
- Fix: After a flush, remove the entry when `Arc::strong_count(&lane) == 2` (the map plus the current holder) and the buffer has no batch for the session.

Ask: Read-only review of lane slack-ingress (ingress.rs, debounce.rs, backfill.rs since e57b472, including the WIP) - wanted: a severity-rated findings report. For you: The review is complete, with 4 P1 and 3 P2 findings. Three of the P1s share one cause: the periodic reconcile (every 120s) either re-runs a turn that was stopped or failed, because the ledger lets it reclaim a `failed` row, or skips the live-only handling for lane replies. I confirmed every finding by reading the current files and the wiring in main.rs and runtime.rs. Per the task, I ran nothing: no cargo, no tests, no edits. Now: none - report delivered. Next: none - the stop condition holds.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/slack/backfill.rs - the replay loop: holding the cursor on failure (P1), skipping lane interception (P1), single-page history (P2), unbounded targets (P2)
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - advance_cursor writes the wrong key for auto-threaded roots (P1); /stop branch never moves the cursor (P1); WIP lane-reply write_cursor at 765-769
- /Users/indo/code/project/omon-gateway/src/slack/debounce.rs - lane map never pruned (P2)
- /Users/indo/code/project/omon-gateway/src/ledger/service.rs - claim_delivery_row lets a `failed` row be reclaimed, which makes the replays possible
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - the Stopped outcome sends Err to the ack and marks the ledger row failed
- /Users/indo/code/project/omon-gateway/src/slack/runtime.rs - reconcile interval and settle window; production wiring
- /Users/indo/code/project/omon-gateway/src/main.rs - lines 1840-1841 spawn startup backfill and reconcile
</files>
<answer>The report above is the deliverable: 4 P1 findings (replayed /stop or failed turns retried every 2 min; free-channel cursor key mismatch; debouncer cancel without moving the cursor; missed lane reply sent to the agent instead of the lane) and 3 P2 findings.</answer>
<next_steps>Hand the findings to the fix lane. The smallest fixes are: write the channel cursor for auto-thread roots in advance_cursor, call advance_cursor on the /stop branch, move the backfill cursor past turns that ran and then failed, and run intercept_lane_reply in backfill before routing.</next_steps>
</results>
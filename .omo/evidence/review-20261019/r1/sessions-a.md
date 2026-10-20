# Lane sessions-a

I reviewed every hunk of the diff: the new `overview.rs` (791 lines), the new `watch.rs` (641 lines) and the `status_reply.rs` changes. I checked each one against the current files and against how `src/main.rs`, the router, Slack ingress and the dashboard call them. The WIP in `omo_backend.rs` and `egress.rs` doesn't affect these paths: the watcher sends plain `SendMessage`, not streaming. I made no edits and ran no cargo, bun or npm commands.

### [P1] The chat "session status" reply now times out whenever one remote host stalls
- Location: src/sessions/overview.rs:458
- Trigger: A configured host accepts the SSH connection but the remote command hangs (stuck `omo`, overloaded box, half-open VPN). `ConnectTimeout=5` doesn't help once connected. The owner sends "세션 상태" in a Discord DM or a Slack DM, which reaches `status_reply::reply()` through `intercept_session_status` (src/multiplexer/router.rs:454).
- Impact: `collect` runs the registry listing first, which can wait the full 20s `MACHINE_TIMEOUT` for the stuck host. Only then does it run the probe, which can wait another 20s `PROBE_TIMEOUT` on the same host. That totals up to 40s, but the reply is capped at 25s (`REPLY_TIMEOUT`). The owner gets only "세션 상태 조회 오류: 25초 제한 시간 초과", even though local data was ready. The stale listing cache can't help because the probe step isn't cached. Before this change, the reply only ran the listing (at most 20s, under 25s) and showed the other machines, so this is a regression.
- Evidence:
  ```rust
  // overview.rs:458
  let list = control.registry().list_all().await;
  ...
  // overview.rs:474
  let snap_results = probe.snapshot_all(&machines, text_limit).await;
  ```
  ```rust
  // status_reply.rs:116 / 123
  let overview = collect(control, 120).await;
  match tokio::time::timeout(REPLY_TIMEOUT, fetch).await {
  ```
  The constants are `MACHINE_TIMEOUT = 20s` (registry.rs:15), `PROBE_TIMEOUT = 20s` (probe.rs:12) and `REPLY_TIMEOUT = 25s`.
- Fix: In `collect`, run the listing and the probe concurrently with `tokio::join!`. Probe `local` plus the configured hosts, i.e. the registry's machine list, so the probe doesn't need the listing's output. Alternatively, give each step its own budget (for example `probe.snapshot_all` wrapped in roughly 10s per machine) so the total stays under `REPLY_TIMEOUT`.

### [P1] A watched session's completion notice is lost for good if delivery fails or it finishes during quiet hours
- Location: src/sessions/watch.rs:345
- Trigger: The production watcher is started in src/main.rs:1795. A watched session (sending a message through `/sessions send` watches it automatically) goes from streaming to idle. Then either (a) `dispatcher.dispatch` returns an error (Slack/Discord rate limit or network blip), or (b) the turn finishes between 00:00 and 08:00 KST, the default quiet hours.
- Impact: The owner never gets the "✅ … 턴 완료" message. The log even says "will retry next tick", but no retry happens. A completion notice exists only on the tick where streaming turns into idle, because `self.streaming` is overwritten with the new `false` state before delivery. On the next tick `was_streaming` is `Some(false)`, so the notice is never rebuilt. Questions don't have this problem because they are rebuilt from live state every tick.
- Evidence:
  ```rust
  // watch.rs:175
  if watched && was_streaming == Some(true) && !session.streaming {
  ```
  ```rust
  // watch.rs:257, 278-279
  self.streaming = Some(current);
  ...
  if quiet {
      continue;
  }
  ```
  ```rust
  // watch.rs:344-345
  Err(error) => {
      tracing::warn!(%error, key = %notice.dedupe_key, "session notice delivery failed; will retry next tick");
  ```
- Fix: Add a field `pending_completions: HashMap<String, Notice>` to `Watcher`. In `tick_at`, put each completion notice into it, and add all pending ones to the candidate list on every tick. The existing `session_watch_notices` dedupe check prevents duplicates. Remove an entry only after `mark_sent` succeeds in `serve`.

### [P2] The Slack home-channel branch of `owner_thread` never runs in production
- Location: src/sessions/status_reply.rs:69
- Trigger: The owner types "세션 상태" in a channel listed in `SLACK_HOME_CHANNEL`.
- Impact: The message goes to the agent instead of getting the direct status reply that `slack_home_conversation` was written for. Slack ingress only checks `owner_dm` (src/slack/ingress.rs:212) and never calls `owner_thread`. The dashboard does call `owner_thread`, but with the Discord owner id as `author` (dashboard.rs:1323/1327), so `author == slack owner_id()` fails. The same author mismatch makes the dashboard's `owner_dm` false for Slack DM sessions. This isn't a regression (it didn't work before either), so I rated it P2.
- Evidence:
  ```rust
  if key.platform == crate::slack::PLATFORM {
      return slack_home_conversation(
          key,
          author,
          &crate::slack::owner_id(),
  ```
  ```rust
  // slack/ingress.rs:211-212
  let private_owner = event_is_plain(&event)
      && crate::sessions::status_reply::owner_dm(&inbound.session, &user, "");
  ```
- Fix: In Slack ingress, OR in `matches_intent(&inbound.content) && owner_thread(&inbound.session, &user, "") && s.multiplexer.owner_thread_is_private(&inbound.session).await`. In the dashboard, pass `owner_author_for(&key)` as the author instead of the Discord owner id.

### [P2] A machine whose probe fails disappears from the overview even when its listing worked
- Location: src/sessions/overview.rs:152
- Trigger: The probe fails on a host while `omo thread list` succeeds. Examples: no `python3` on the remote host, or the probe hits its 20s timeout.
- Impact: All of that host's listed sessions, and its stale marker, are replaced by a single warning line. The old `format_reply` would still have shown those sessions. The view gets worse, but nothing shown is wrong.
- Evidence:
  ```rust
  if let Some(Err(err_msg)) = snap_res {
      lines.push(format!("⚠️ {}: {}", compact(m_str, 32), compact(err_msg, 70)));
      continue;
  }
  ```
- Fix: Drop the `continue`. Show the warning line, then still show the listing-only sessions (the loop at overview.rs:277 already handles `snap_sessions == None`).

### [P2] A failed snapshot restarts the 30-minute wait, and a failed `mark_sent` sends the same question again
- Location: src/sessions/watch.rs:264
- Trigger: (a) A host's snapshot fails on one tick while a question is still waiting there. (b) `mark_sent` fails after a successful dispatch.
- Impact: (a) That question's keys are missing from `present`, so `first_seen` drops them and the `notify_after` wait starts over. This keeps happening on a flaky host. (b) The same question gets sent again on the next tick. The code logs this as "sent but not recorded".
- Evidence:
  ```rust
  self.first_seen
      .retain(|key, _| present.contains(key.as_str()));
  ```
- Fix: For (a), when a machine's snapshot is `Err`, keep the `first_seen` entries whose key contains `:{machine}:`, the same way `current` keeps that machine's previous streaming state (watch.rs:233-243). (b) only needs the existing warning.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/sessions/overview.rs - `collect` runs listing then probe one after the other (lines 458/474); a failed probe hides the listing (line 152)
- /Users/indo/code/project/omon-gateway/src/sessions/status_reply.rs - 25s reply cap (lines 116/123); Slack home-channel branch never reached (line 69)
- /Users/indo/code/project/omon-gateway/src/sessions/watch.rs - completion edge lost on dispatch failure or quiet hours (lines 175/257/278/345); `first_seen` reset (line 264)
- /Users/indo/code/project/omon-gateway/src/sessions/probe.rs - `PROBE_TIMEOUT` of 20s per machine
- /Users/indo/code/project/omon-gateway/src/sessions/registry.rs - `MACHINE_TIMEOUT` of 20s per machine
- /Users/indo/code/project/omon-gateway/src/main.rs - production watcher wiring (lines 1789-1810)
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - only `owner_dm` is checked (line 212)
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - passes the Discord owner as author for every platform (lines 1323-1328, 1436-1440)
</files>
<answer>
I found two P1s and three P2s. P1: the chat status reply now times out when one remote host stalls, because listing and probe each wait up to 20s in turn under a 25s cap. P1: a watched session's completion notice is lost for good if delivery fails or the turn finishes during quiet hours. The P2s are a Slack home-channel branch that production never reaches, a host disappearing from the overview when only its probe fails, and the question wait timer restarting after a failed snapshot.
</answer>
<next_steps>
Ready to proceed - no follow-up needed. Every cited line was re-opened and matches the quoted code.
</next_steps>
</results>
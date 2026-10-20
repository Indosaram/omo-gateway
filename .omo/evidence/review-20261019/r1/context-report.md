# Lane context-report

I reviewed all 5 files in the diff (`src/context/mod.rs`, `src/context/pack.rs`, `src/report/claims.rs`, `src/report/mod.rs`, `src/report/status.rs`) against the current working tree, including the uncommitted WIP in `src/slack/egress.rs` and `src/agent/omo_backend.rs`. Production callers I checked: `omo_backend.rs:925-932` (`pack_for_turn`, `pack_has_foreign_text`, and `turn_tainted`, which goes into `evaluate_pre_ask` at :1670), `slack/egress.rs:161,233-240,325` (claim stripping, `filter_chunk`, `record_and_schedule_claims`), `multiplexer/actor.rs:556-822` (`StatusDriver`) and `main.rs:1252` (stale-claim recovery). I ran no cargo or tests; everything below comes from reading the code.

### [P1] Thread lines from bots or webhooks can label themselves "owner", so the turn is not marked tainted
- Location: src/context/pack.rs:405
- Trigger: An incoming webhook or a bot without a `user` field posts into the owner's Slack lead thread with `username: "owner"` (or its `bot_profile.name` is "owner"). The owner then sends a message in that thread. `unseen_thread_lines` renders that post as `- owner: <injected text>`. `pack_has_foreign_text` returns false, so `turn_tainted` stays false at `omo_backend.rs:931`.
- Impact: Text from someone else in the thread is passed to `evaluate_pre_ask` as untainted. When `OMON_FULL_ACCESS` is off and the ledger is on, two safeguards are lost:
  - `find_matching_precedent` no longer refuses delete and vcs_protected actions for this turn.
  - `requires_always_ask` no longer forces a question for external_send, delete or vcs_protected.
  
  As a result, an existing class precedent can auto-allow an action the injected text asked for. I rated it P1, not P0, because it needs non-default config, an existing precedent, and a third party who can post with a custom username.
- Evidence:
  ```rust
  let author = match user {
      Some(id) if id == settings.slack_owner => "owner".to_owned(),
      Some(id) => format!("<@{id}>"),
      None => message
          .get("username")
          .or_else(|| message.pointer("/bot_profile/name"))
          .and_then(Value::as_str)
          .unwrap_or("unknown")
          .to_owned(),
  };
  ```
  and at pack.rs:296: `.any(|line| !line.starts_with("- owner:"))`
- Fix: Give user-less authors a prefix that can never match, e.g. `None => format!("bot {}", message.get("username")...unwrap_or("unknown"))`. Better: have `unseen_thread_lines` return whether any non-owner line was included, and use that value instead of re-parsing the rendered text.

### [P1] StreamClaimFilter keeps the full reply text forever for every stream that never gets a final chunk
- Location: src/report/claims.rs:78
- Trigger: Every `item/agentMessage/delta` sends a cumulative non-final chunk (`omo_backend.rs:2092-2098`, `is_final: false`). Slack egress calls `filter_chunk` before it returns early on non-final chunks (`egress.rs:233-240`, then `if !chunk.is_final { return Ok(()) }`). If the turn then fails, gets interrupted (owner stop), or hits the total deadline (`omo_backend.rs:2109-2114`, `:1497`), it returns `Err` and no final `Stream` chunk is ever sent.
- Impact: The `(session_key, stream_id)` entry, holding the whole partial reply, is never removed. The map grows with every failed or interrupted streamed Slack turn for as long as the process runs. With the WIP, the map is also useless: partial chunks are never shown, and the final chunk overwrites the entry anyway.
- Evidence:
  ```rust
  let entry = map.entry(key.clone()).or_default();
  // Gateway stream chunks are cumulative: each one carries the whole reply so far.
  *entry = chunk_text.to_string();
  let current = entry.clone();
  if is_final {
      map.remove(&key);
  }
  ```
- Fix: Chunks are cumulative, so the map isn't needed. Make `filter_chunk` work only on `chunk_text`: `if is_final { strip_and_extract_claims(chunk_text) } else { (strip_and_extract_claims(hold_incomplete_marker_suffix(chunk_text)).0, Vec::new()) }`. Then drop the `accumulated` field.

### [P2] An unclosed or quote-unbalanced `[[measure` marker deletes the rest of the reply
- Location: src/report/claims.rs:140
- Trigger: The final reply contains `[[measure` with no closing `]]`. Another case is a marker with an odd number of unescaped `"`, such as `expect~"say "hi""`: `find_marker_end` stays inside quotes, never finds `]]`, and returns `None`.
- Impact: Everything from `[[measure` to the end of the message is silently dropped from what Slack posts (`egress.rs:161`, `:235`), and Discord does the same (`adapter.rs:3410`). This takes malformed model output, so I rated it P2. The user still loses reply text without any notice.
- Evidence:
  ```rust
  let Some(end_rel) = find_marker_end(after_open) else {
      break;
  };
  ...
  if let Some(unclosed) = rem.find("[[measure") {
      cleaned.push_str(&rem[..unclosed]);
  ```
- Fix: For the final text, keep the unclosed remainder (`cleaned.push_str(rem)`) and cut only a marker that is actually closed. The hold-back for unclosed markers is only needed for non-final chunks, and `hold_incomplete_marker_suffix` already does that.

### [P2] Precedent lookup queries a table that does not exist, and the error is swallowed
- Location: src/context/pack.rs:95
- Trigger: Any owner turn in a thread bound to a lane runs `LedgerPrecedents::precedents`.
- Impact: No migration creates `lane_worktrees`; the table is `lane_owned_worktrees` (migrations/0038). The query always fails, and `.unwrap_or_default()` (line 99) hides the error, so the lane's worktree repos and paths never become precedent targets. Precedents scoped to those worktrees are missing from the pack.
- Evidence: `sqlx::query_as("SELECT repo, path FROM lane_worktrees WHERE lane_key = ?")` ... `.unwrap_or_default();`
- Fix: Use `SELECT repo, path FROM lane_owned_worktrees WHERE lane_key = ? AND state = 'registered'` and pass the error up with `?`, as the `cwd` query above it does.

### [P2] The thread fetch reads only the oldest 200 replies, so long threads lose their newest unseen lines
- Location: src/context/pack.rs:35
- Trigger: The owner posts in a Slack thread with more than 200 replies.
- Impact: `conversations.replies` without a cursor returns the first 200 messages. The newest unseen messages, which are the most relevant ones and possibly foreign, never reach the pack or the taint check.
- Evidence: `self.conversations_replies(channel, thread_ts, None, REPLIES_FETCH_LIMIT)`
- Fix: Pass `oldest` set to the newest stored `platform_message_id` for the session, or paginate to the end.

### [P2] Tool-activity entries are cleared only by a StatusDriver, so other sessions leak them and show stale tools
- Location: src/report/status.rs:50
- Trigger: `record_tool_activity` runs for every session's tool start (`omo_backend.rs:2001`, `:1680`). That includes lanes, cron and System-origin turns, which never create a `StatusDriver` (`actor.rs:556-562`). `clear_tool` only runs in `finish`/`abandon`.
- Impact: The map grows by one small string per distinct session key and never shrinks. Also, a System-origin turn on the owner's lead session leaves its tool name behind, and the next owner turn's status edit shows it as current until a new tool starts.
- Evidence: `tool_registry().lock().insert(session.storage_key(), tool.chars().take(60).collect());`
- Fix: Clear the session's entry when a turn starts (in `StatusDriver::start`, and wherever a turn starts without a driver), or keep the latest tool on the active turn instead of in a global map.

### [P2] Claims still pending at shutdown and younger than 10 minutes stay `pending` forever
- Location: src/report/claims.rs:509
- Trigger: The gateway restarts within `STALE_CLAIM_AGE_SECS` (600s) of a claim being recorded. The spawned re-measure task is lost, and the startup recovery (`main.rs:1252`) only marks claims older than 10 minutes.
- Impact: Those claims are never measured and never resolved; the effect is limited to stale DB state.
- Evidence: `WHERE state = 'pending' AND julianday(created_at) <= julianday(?)`
- Fix: At startup, mark every `pending` row `unmeasurable`, since no task survives a restart, or re-schedule the young ones.

What I checked and found correct: `is_owner_message` gating, the pack size cap, the `wrap_prompt` placement, the `remeasure_allowed` allowlist and env scrubbing, where corrections are sent, and `StatusDriver`'s adopt-on-restart, quiet hours and finish/abandon handling. These are consistent with the WIP "post only the final reply" egress.
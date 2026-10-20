st_01a1569b [completed] model mahoquot/gemini-3.8-flash-high
### Delta Code Review: Confirmed Fixes & Scoped Areas

This review cross-examines `.omo/evidence/review-20261019/CONFIRMED-r1.md` and related review reports (`r1/slack-ingress.md`, `r1/slack-egress.md`, `r1/slack-transport.md`, `r1/slack-commands.md`, `r1/sessions-a.md`, `r1/sessions-b.md`, `r1/context-report.md`) against current `HEAD` across the scoped paths:
- `src/slack/`
- `src/sessions/`
- `src/context/pack.rs`
- `src/report/claims.rs`

---

### Part 1. Verification Verdict for Matching Confirmed Fixes

| ID | Issue & Source Location | Verification Status | Code & Test Evidence |
|---|---|---|---|
| **F1** | `slack-commands`: Channel slash commands create channel-level session & miss auto-threads (`src/slack/commands.rs:438, 612-646`) | **VERIFIED** | In `src/slack/commands.rs:618-646`, top-level channel invocations with no thread ts return `THREAD_ONLY` for state-creating commands (`Model`, `Yolo`, `Steer`, `Retry`, `Skill run`). For `Stop` and `Deny`, `channel_thread_sessions()` searches active threads for that channel and bot. Tested in `commands.rs:1880-1940`. |
| **F2** | `sessions-a`: `collect` timeout exceeding `REPLY_TIMEOUT` (25s) on remote stall (`src/sessions/overview.rs:458, 474`) | **VERIFIED** | In `src/sessions/overview.rs:449-485`, `COLLECT_BUDGET` is enforced at `Duration::from_secs(23)`. `probe_budget` is dynamically computed as `COLLECT_BUDGET.saturating_sub(started.elapsed())`, and `probe.snapshot_all` is wrapped with `tokio::time::timeout(probe_budget, ...)`. Listing + probe cannot exceed the 25s threshold. |
| **F3** | `sessions-a`: Watched session completion notice dropped on quiet hours / dispatch failure (`src/sessions/watch.rs:260-310`) | **VERIFIED** | In `src/sessions/watch.rs:260-310`, `self.pending_completions: HashMap<String, Notice>` holds completion notices until confirmed sent via DB check. Retries every tick across quiet hours or transient network failures. Verified by test `completion_whose_first_dispatch_fails_is_delivered_on_next_tick_exactly_once` (`watch.rs:722`). |
| **F4** | `slack-transport`: `intercept_lane_reply` returns `Ok(true)` without advancing cursor (`src/slack/ingress.rs:777-781`) | **VERIFIED** | In `src/slack/ingress.rs:777-780`, `write_cursor(&s.pool, &s.bot_user_id, channel, thread_ts, ts).await` is called immediately before waking the lane runner and returning `Ok(true)`. |
| **F14** | `slack-ingress`: `route_awaiting_turn` holds cursor on terminal error (`src/slack/ingress.rs:584-590`) | **VERIFIED** | In `src/slack/ingress.rs:584-590`, terminal errors return `Ok(true)` to let the cursor advance past the failed/stopped message. Only errors matching `"gateway admission closed"` return `Err(error)` to keep the cursor held. Tested in `backfill.rs:269-320`. |
| **F15** | `slack-ingress`: `/stop` in Slack ingress does not advance cursor (`src/slack/ingress.rs:228-235`) | **VERIFIED** | In `src/slack/ingress.rs:234`, `advance_cursor(&s.pool, &inbound).await;` is called directly after `stop_session` in the `is_stop` block. |
| **F16** | `slack-ingress`: Auto-thread root channel cursor mismatch (`src/slack/ingress.rs:792-805`) | **VERIFIED** | In `src/slack/ingress.rs:793-805`, `roots_own_thread` checks `!thread.is_empty() && thread == event.platform_message_id`. Iterates over `std::iter::once(thread).chain(roots_own_thread.then_some(""))` to advance both thread cursor and top-level channel cursor. |
| **F17** | `slack-ingress`: Backfill ignores lane reply interception (`src/slack/backfill.rs:150-153`) | **VERIFIED** | In `src/slack/backfill.rs:150-153`, `ingress.intercept_replayed_lane_reply(&event).await?` is called before checking `known_thread` and calling `map_replayed`. If intercepted, it skips turn routing. |
| **F19** | `context-report`: User-less author (webhook/bot) renders as "owner" causing untainted pack (`src/context/pack.rs:405-416`) | **VERIFIED** | In `src/context/pack.rs:405-416`, user-less entries explicitly format as `"bot {}"` using `username` or `/bot_profile/name`, preventing spoofing of `"owner"`. `pack_has_foreign_text` properly flags foreign text. |
| **F20** | `context-report`: `filter_chunk` leaks memory on interrupted turns without final chunk (`src/report/claims.rs:50-80`) | **VERIFIED** | In `src/report/claims.rs:50-80`, `StreamClaimFilter` is stateless (`struct StreamClaimFilter;`). `accumulated` map was removed entirely. Cumulative chunks are processed directly without accumulating state. Tested in `tests/test_report_claims.rs:140-155`. |
| **F23** | `slack-egress`: Partial chunks allocate per-stream state never cleaned up (`src/slack/egress.rs:222-225`) | **VERIFIED** | In `src/slack/egress.rs:222-225`, `if !chunk.is_final { return Ok(()); }` is placed at the very top of `stream()`, prior to allocating any `StreamState` or locking `streams`. |
| **F24** | `slack-egress`: Multiple final chunks with same `stream_id` silently dropped (`src/slack/egress.rs:228-232, 285-290`) | **VERIFIED** | In `src/slack/egress.rs:228-232, 289`, `done` tuple is `(key.0.clone(), key.1, chunk.sequence)`. Includes `sequence` u64, preventing subsequent distinct final chunks from being deduped away. |
| **F28** | `sessions-b`: External session adoption seq cursor breaks past ~60 messages (`src/sessions/external.rs:258-265, 590-610`) | **VERIFIED** | In `src/sessions/external.rs:258-265`, numeric cursors trigger `None` cursor read. Assistant items are identified by content hash `h:<sha256(assistant:text)>` and deduplicated against `external_session_items`, completely removing the broken `seq > last_seq` filter. Tested in `external.rs:765-780`. |

---

### Part 2. Detailed Inspection of Scoped Behaviors

#### 1. Cursor Admission Errors
- **Implementation**: In `src/slack/ingress.rs:584`, `route_awaiting_turn` checks:
  ```rust
  if !error.to_string().contains("gateway admission closed") {
      tracing::warn!(%error, session = %session_key, "Slack replayed turn ended without a reply; not retrying");
      return Ok(true);
  }
  return Err(error);
  ```
- **Finding**: Terminal turn failures (stopped by user, quota error, timeout) resolve with `Ok(true)`, permitting the cursor to advance in `backfill.rs:165`. Only when the gateway rejects admission during graceful shutdown (`"gateway admission closed"`, originating from `src/multiplexer/router.rs:368/588`) is `Err` returned, intentionally holding the cursor for post-restart replay. This matches intended behavior.

#### 2. Final Stream Dedup
- **Implementation**: In `src/slack/egress.rs:228, 289`:
  - `let done = (key.0.clone(), key.1, chunk.sequence);`
  - In `finish_stream(key, done)`: `self.streams.lock().remove(&key); completed.insert(done, Instant::now());`
- **Finding**: The dedup key contains `(session_storage_key, stream_id, chunk.sequence)`. Distinct relayed final messages advance their sequence number and will not collide with previous final chunks of the same turn. Identical redelivered chunks (same sequence) are skipped.

#### 3. External Session Cursor & Adoption
- **Implementation**: In `src/sessions/external.rs`:
  - `assistant_items` generates an id from the SHA-256 hash of `assistant:{text}` if no stable ID is present (`format!("h:{}", hex_prefix(&digest))`).
  - Read loop uses `INSERT OR IGNORE INTO external_session_items(lane_key, item_id)` to deduplicate.
  - Omo's rolling `seq` (1..60) is no longer used for pagination filtering.
- **Finding**: External transcript ingestion is resilient to sliding windows and tail truncation.

#### 4. Claim Filtering
- **Implementation**: In `src/report/claims.rs:58-75`, `StreamClaimFilter` is stateless.
  - `hold_incomplete_marker_suffix` holds back unclosed `[[` markers during streaming.
  - `strip_and_extract_claims` extracts claims and strips `[[measure ...]]` markers on finalized chunks.
- **Finding**: Leaking memory maps have been eliminated.

---

### Part 3. Discovery of Reachable P0 / P1 / P2 Defects

#### [P2] `context/pack.rs`: Precedent worktree query targets non-existent table `lane_worktrees`
- **Location**: `/Users/indo/code/project/omon-gateway/src/context/pack.rs:95`
- **Trigger**: Any turn in an owner thread bound to a lane invokes `build_pack` -> `PrecedentSource::precedents`.
- **Evidence**:
  ```rust
  let wt_rows: Vec<(String, String)> =
      sqlx::query_as("SELECT repo, path FROM lane_worktrees WHERE lane_key = ?")
          .bind(key)
          .fetch_all(pool)
          .await
          .unwrap_or_default();
  ```
  In migrations (`migrations/0038_lane_lifecycle.sql:24`), the table is named `lane_owned_worktrees`. `lane_worktrees` does not exist in any migration or SQLite schema.
- **Impact**: The query consistently fails on SQLite with `no such table: lane_worktrees`. Because it calls `.unwrap_or_default()`, the error is swallowed and lane-associated worktree paths are never added to `targets`, omitting worktree-scoped precedents from the context pack.
- **Severity**: P2 (functional omission, silent failure without crash).

#### [P2] `report/claims.rs`: Unclosed or quote-mismatched `[[measure` marker drops entire message remainder
- **Location**: `/Users/indo/code/project/omon-gateway/src/report/claims.rs:125-131`
- **Trigger**: Model outputs an unclosed `[[measure` or an unescaped double quote inside the marker arguments (e.g., `[[measure ["echo", "hello"] expect~"foo "bar""]]`).
- **Evidence**:
  ```rust
  if cursor < text.len() {
      let rem = &text[cursor..];
      if let Some(unclosed) = rem.find("[[measure") {
          cleaned.push_str(&rem[..unclosed]);
      } else {
          cleaned.push_str(rem);
      }
  }
  ```
  `find_marker_end` fails to find `]]` due to unbalanced quotes, returning `None` and breaking the extraction loop. `rem.find("[[measure")` matches the unclosed marker and truncates everything from `[[measure` to the end of the text.
- **Impact**: Trailing reply text is dropped from the message delivered to Slack and Discord.
- **Severity**: P2 (requires malformed model generation).

#### Reachable P0 / P1 Assessment
- **Assessment**: No newly introduced reachable P0 (crash, data loss, misdelivery, broken build) or P1 (user-visible hang, silent drop of message, regression) was found in `src/slack/`, `src/sessions/`, `src/context/pack.rs`, or `src/report/claims.rs`. All 13 targeted fixes (F1-F4, F14-F17, F19-F20, F23-F24, F28) are sound and verified against current code and tests.

<results>
<files>
- /Users/indo/code/project/omon-gateway/.omo/evidence/review-20261019/CONFIRMED-r1.md - reference register of confirmed fixes
- /Users/indo/code/project/omon-gateway/src/slack/commands.rs - verified F1 fix for channel slash command isolation
- /Users/indo/code/project/omon-gateway/src/sessions/overview.rs - verified F2 collect budget timeout safeguard
- /Users/indo/code/project/omon-gateway/src/sessions/watch.rs - verified F3 persistent pending_completions logic
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - verified F4, F14, F15, F16 cursor advancement and terminal error handling
- /Users/indo/code/project/omon-gateway/src/slack/backfill.rs - verified F17 replayed lane reply interception
- /Users/indo/code/project/omon-gateway/src/context/pack.rs - verified F19 userless author prefix; identified P2 table name mismatch
- /Users/indo/code/project/omon-gateway/src/report/claims.rs - verified F20 stateless StreamClaimFilter; identified P2 unclosed marker truncation
- /Users/indo/code/project/omon-gateway/src/slack/egress.rs - verified F23 early partial chunk drop and F24 sequence-keyed stream completion
- /Users/indo/code/project/omon-gateway/src/sessions/external.rs - verified F28 content-hash item identity and numeric cursor avoidance
</files>

<answer>
All 13 confirmed fixes within scope (F1, F2, F3, F4, F14, F15, F16, F17, F19, F20, F23, F24, F28) have been audited against the codebase and are VERIFIED with line-level code and test citations. Cursor admission error discrimination, final stream sequence deduplication, external session content-hash adoption, and stateless claim filtering were all verified as cleanly implemented. No new reachable P0/P1 defects were identified; two P2 items (non-existent `lane_worktrees` query in `pack.rs:95` and unclosed marker text truncation in `claims.rs:125`) were noted.
</answer>

<next_steps>
Ready to proceed - no follow-up needed.
</next_steps>
</results>

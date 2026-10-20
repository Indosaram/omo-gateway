# Consolidated Delta Review Report: F37-F42

**Date**: 2026-10-20  
**Scope**: `src/agent/`, `src/autonomy/`, `src/lanes/guard.rs`, `src/inbox/sources/slack.rs`, and related tests  
**Reference Baseline**: `CONFIRMED-r1.md` / `e57b472..c943813` + follow-up delta  

---

## Executive Summary of Verdicts

| ID | Component / File | Subject | Initial Verdict | Final Amended Verdict |
|---|---|---|---|---|
| **F37** | `src/agent/omo_backend.rs`, `src/agent/omo_daemon.rs` | Transport-reset retry gated by `side_effects`; un-stamping forced restarts | **VERIFIED** | **VERIFIED** |
| **F38** | `src/agent/omo_backend/follow.rs` | Engine-turn follower suppression of final delivery when `stopping` | **VERIFIED** | **VERIFIED** |
| **F39** | `src/autonomy/loop.rs` | Action-loop seen marking deferred to emit tx & cap handling | *VERIFIED (flawed)* | **VERIFIED (amended)** |
| **F40** | `src/autonomy/ledger.rs` | Expire engine/tool proposals on restart; check live waiter on short replies | **VERIFIED** | **VERIFIED** |
| **F41** | `src/lanes/guard.rs` | Host guard `got_worse()` check evaluated independently of 24h realert window | **VERIFIED** | **VERIFIED** |
| **F42** | `src/inbox/sources/slack.rs` | Late-indexed Slack inbox cursor: monotonic SQLite `rowid` cursor | **VERIFIED** | **VERIFIED** |

---

## Detailed Findings and Verification Analysis

### F37: Transport-Reset Retry Side Effects & Forced Restart Stamp Gate
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs` (lines 920, 1931, 1998, 2031, 2472-2555)
  - `/Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs` (lines 850-875, 920-935)
- **Problem**:
  When a stalled turn in one session triggered a forced kill and restart of the owned daemon, in-flight turns in concurrent sessions failed with transport reset errors (`is_transport_reset`). If the gateway blindly retried, any turn that had already invoked tools or dispatched messages to the user repeated those operations. In addition, `force_restart_owned_daemon` stamped `LAST_FORCED_RESTART` before confirming process ownership and liveness, creating false positives for `forced_restart_since`.
- **Implementation & Trace**:
  1. `execute_turn` takes `side_effects: &mut bool` (line 920).
  2. `*side_effects = true` is set on:
     - Outbound progress/rendered result delivery (line 1931, 2031).
     - Tool invocation start (`line 1998`: `if !matches!(item_type, "agentMessage" | "userMessage" | "reasoning" | "")`).
  3. In `omo_backend.rs` (line 2525), transport-reset retries require:
     ```rust
     if !retried
         && !side_effects
         && result.as_ref().is_err_and(is_transport_reset)
         && crate::agent::forced_restart_since(&self.config.appserver_url, turn_started)
     ```
  4. In `omo_daemon.rs` (lines 868-875), `unstamp()` removes the URL entry from `LAST_FORCED_RESTART` if the child process was not owned or could not be killed.
- **Verdict**: **VERIFIED**.

---

### F38: Follower Engine-Turn Duplicate Delivery While Stopping
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/agent/omo_backend/follow.rs` (lines 118-132, 203-225)
- **Problem**:
  When a user submitted an interactive message while an engine-initiated turn was running, a gateway turn was queued behind it. The gateway turn's pre-ack relay delivers messages from the engine turn. When the follower was cancelled upon user turn start, the follower's `turn/completed` frame handling also delivered the engine turn's final output, resulting in duplicate delivery.
- **Implementation & Trace**:
  1. In `run_follower`, token cancellation sets `stopping = true` when `current.is_some()` (lines 124-129) rather than immediately aborting.
  2. At `lines 205-225`, final delivery is gated by `!stopping`:
     ```rust
     if !stopping
         && val.pointer("/params/turn/status").and_then(Value::as_str) == Some("completed")
         && !rendered.trim().is_empty()
         && !is_explicit_silence(&rendered)
         && last_delivered.as_deref() != Some(rendered.as_str())
     {
         self.deliver_rendered_result(session, Uuid::new_v4(), 0, &rendered, None).await?;
         last_delivered = Some(rendered);
     }
     if stopping {
         return Ok(());
     }
     ```
  3. When `stopping` is true, the pre-ack relay handles the output; the follower drops its delivery and exits cleanly without duplicating text.
- **Verdict**: **VERIFIED**.

---

### F39: Action-Loop Seen Marking, Scheduling Stalls, and Cap Deferral
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/autonomy/loop.rs` (lines 83-185, 230-265)
  - `/Users/indo/code/project/omon-gateway/tests/test_action_loop.rs` (lines 135-180)
- **Initial Defect**:
  Items collected during `collect_new_items` were marked seen in SQLite prior to delivery. If the process crashed during the 120s coalesce window or if daily cap was reached, items were permanently dropped.
- **Initial Review & Missed Defect (Initial Verdict: VERIFIED)**:
  The lead moved the write of `action_loop_seen` into the emit transaction in `flush_coalesced`. However, on hitting `self.daily_cap`, the lead initially reinserted deferred items into `self.pending_batch`:
  `self.pending_batch.lock().await.extend(items);`
  *Reachable scheduling stall*: In `serve()`, `coalesce_deadline` was reset to `None` upon firing. On the next tick, `collect_new_items()` saw that every item in `candidates` was already present in `pending_batch`, returning `added = 0`. Because `coalesce_deadline` is only set when `added > 0 && coalesce_deadline.is_none()`, the timer was never rearmed without a brand-new arrival, causing deferred items to stall indefinitely even after the day rolled over.
- **Amended Fix & Trace (Final Verdict: VERIFIED)**:
  1. The lead removed `self.pending_batch.lock().await.extend(items);` from the cap branch in `flush_coalesced` (`src/autonomy/loop.rs:125-135`).
  2. Because the items were never written to `action_loop_seen` and `pending_batch` was drained by `std::mem::take`, the next periodic scan (`sweep_interval.tick()`, line 254) finds the unseen items in the database and adds them to `pending_batch`.
  3. `collect_new_items()` returns `added = items.len() > 0`.
  4. In `serve()` (line 256), `added > 0 && coalesce_deadline.is_none()` evaluates to true, rearming `coalesce_deadline` after 30 seconds without requiring external wake events.
  5. Tested and verified in `tests/test_action_loop.rs:163-167`:
     ```rust
     assert_eq!(
         worker.collect_new_items(daytime_utc()).await.unwrap(),
         1,
         "the next sweep must re-arm the coalesce timer without a new arrival"
     );
     ```
  6. **Restart & Baseline Safety**:
     `baseline_if_first_run` checks `SELECT COUNT(*) FROM action_loop_seen`. If any events were previously processed on day 1 (which reaching the daily cap guarantees), `seen > 0`, so baseline returns `Ok(0)` immediately. The deferred items are never marked seen on restart and are processed on schedule.
- **Verdict**: **VERIFIED (amended)**.

---

### F40: Expire Carded Engine Approvals Across Restart & Validate Short Replies
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/autonomy/ledger.rs` (lines 230-278, 693-725)
  - `/Users/indo/code/project/omon-gateway/tests/test_action_ledger.rs` (lines 1040-1085)
- **Problem**:
  After a gateway restart, pending `engine_approval` / `tool_approval` proposals were retained in SQLite. If the owner wrote an ordinary affirmative short reply ("응", "해"), `try_resolve_short_reply` matched the database row, marked it allowed, and swallowed the message, even though the turn and WebSocket frame that requested approval were already dead.
- **Implementation & Trace**:
  1. In `recover_pending_proposals_on_startup` (lines 693-722), all pending proposals with `origin IN ('engine_approval', 'tool_approval')` are transitioned to `state = 'expired'` and `decided_at = CURRENT_TIMESTAMP`.
  2. In `try_resolve_short_reply` (lines 242-246):
     ```rust
     let db_pending: Vec<ActionProposal> = match pool {
         Some(p) => list_pending_proposals_for_session(p, &session.storage_key())
             .await?
             .into_iter()
             .filter(|row| !matches!(row.origin.as_str(), "engine_approval" | "tool_approval"))
             .collect(),
         None => Vec::new(),
     };
     ```
     Engine/tool approvals are strictly filtered out of `db_pending`. They can only resolve if an active waiter exists in `guard.pending_for_session()`.
  3. If `decide_proposal` returns an error (lines 268-275), it logs a warning and returns `Ok(false)` so the owner's message is passed through to the conversation instead of being discarded.
  4. Validated by test `d3_restart_expires_every_engine_approval_even_with_a_card`.
- **Verdict**: **VERIFIED**.

---

### F41: Host Guard Worsening Alerts After 24-Hour Alert Episodes
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/lanes/guard.rs` (lines 550-590, 650-675, 1330-1360)
- **Problem**:
  When a host stayed in `GuardState::Alert` for longer than 24 hours (`GUARD_REALERT_WINDOW`), the Alert state branch previously required `now - mark.at < GUARD_REALERT_WINDOW`. After 24 hours elapsed, `got_worse()` was bypassed, silencing alerts even if free disk space fell by another 10+ GB or memory degraded further.
- **Implementation & Trace**:
  1. In `src/lanes/guard.rs:654-673`:
     ```rust
     GuardState::Alert => {
         let load_cleared = sample.load_1m < self.thresholds.load_ok;
         let mem_cleared = sample.mem_free_pct >= self.thresholds.mem_free_min_pct;
         let disk_cleared = sample.disk_free_gb >= self.thresholds.disk_free_min_gb;

         if load_cleared && mem_cleared && disk_cleared {
             self.current_state = GuardState::Clear;
             let reported = std::mem::replace(&mut self.episode_reported, false);
             return reported.then_some(GuardReport::Clear);
         }
         if !alert_reasons.is_empty()
             && self.last_alert.is_some()
             && self.got_worse(load_alert, mem_alert, disk_alert, sample)
         {
             self.mark_alert(load_alert, mem_alert, disk_alert, sample, now);
             return Some(GuardReport::Alert(alert_reasons.join("; ")));
         }
         None
     }
     ```
  2. `got_worse(...)` (lines 566-586) evaluates only delta worsening: new alerting metric, disk dropping >= 10 GB (`GUARD_REALERT_DISK_DROP_GB`), or memory dropping >= 10% (`GUARD_REALERT_MEM_DROP_PCT`), completely free of time restrictions.
  3. Validated by test `guard_alert_episode_longer_than_window_still_reports_a_worse_disk`.
- **Verdict**: **VERIFIED**.

---

### F42: Late-Indexed Slack Inbox Cursor Stability
- **Target Files**:
  - `/Users/indo/code/project/omon-gateway/src/inbox/sources/slack.rs` (lines 31-85, 295-325)
- **Problem**:
  `SlackSource::fetch_since` used the Slack timestamp (`message_id`) as its cursor. Messages inserted out of order (such as backfill reconcile passes recovering dropped socket messages, or extended debounce flushes) had timestamps earlier than already-collected messages and were permanently skipped.
- **Implementation & Trace**:
  1. `fetch_since` parses the cursor as an integer rowid:
     `let after_rowid: i64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);`
     Passing an older floating-point timestamp cursor cleanly fails `parse::<i64>()` and resets to `0`, triggering an idempotent rescan.
  2. The SQL query orders and filters by rowid:
     ```sql
     SELECT d.rowid, ...
     FROM message_search_documents d
     WHERE d.platform = 'slack'
       AND d.rowid > ?
       ...
     ORDER BY d.rowid
     ```
  3. In SQLite, every newly inserted row receives an auto-incremented `rowid` greater than the previous maximum. Even if the message timestamp is in the past, its `rowid` is higher than `after_rowid`, ensuring late-indexed messages are captured.
  4. Validated by test `collects_older_message_indexed_after_cursor_advanced`.
- **Verdict**: **VERIFIED**.

---

## Conclusion
All targeted fixes (F37, F38, amended F39, F40, F41, and F42) have been thoroughly traced through code, tests, and execution flows. Every defect is confirmed resolved with no remaining P0 or P1 blockers.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - F37 transport-reset retry gating
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend/follow.rs - F38 follower stopping deduplication
- /Users/indo/code/project/omon-gateway/src/autonomy/loop.rs - F39 amended action loop deferral & scheduling
- /Users/indo/code/project/omon-gateway/src/autonomy/ledger.rs - F40 proposal restart expiration & short reply validation
- /Users/indo/code/project/omon-gateway/src/lanes/guard.rs - F41 alert worsening check beyond 24h
- /Users/indo/code/project/omon-gateway/src/inbox/sources/slack.rs - F42 rowid cursor for late-indexed Slack messages
</files>

<answer>
The consolidated review report for findings F37 through F42 is finalized above, with complete verification traces and explicit distinction between initial and amended verdicts for F39.
</answer>

<next_steps>
Ready to proceed - no follow-up needed
</next_steps>
</results>

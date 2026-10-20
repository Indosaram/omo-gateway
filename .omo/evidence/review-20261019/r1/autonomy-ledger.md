# Lane autonomy-ledger

### [P1] Action-loop items are marked seen before they are delivered, so a restart, a failed flush, or the daily cap loses them for good
- Location: src/autonomy/loop.rs:89
- Trigger: With `OMON_ACTION_LOOP_ENABLED=true`, a new inbox bot/owner item, GitHub item, or lane `blocked`/`dead` event arrives. The 30s sweep or a wake collects it. Then one of three things happens: the gateway restarts or deploys inside the 120s coalesce window (`shutdown.cancelled() => break` at :220), `flush_coalesced` errors after `std::mem::take` (:111), or the day's cap is already reached (:134).
- Impact: The `action_loop_seen` row is committed at collect time, but the item itself only lives in the in-memory `pending_batch`. The next scan skips it because of `NOT EXISTS (... action_loop_seen ...)`. The lead never gets an `[ACTION_LOOP]` event for that item, and nothing tells the owner.
- Evidence:
  ```
  "INSERT INTO action_loop_seen (item_key, seen_at) VALUES (?, ?)
   ON CONFLICT(item_key) DO NOTHING",
  ...
  if inserted.rows_affected() == 1 {
      batch.push(item);
  ```
  ```
  _ = shutdown.cancelled() => break,
  ```
  ```
  "action_loop daily cap reached; dropping coalesced batch"
  ```
- Fix: Write the seen rows in the same transaction as the `lane_events` INSERT inside `flush_coalesced`, so nothing is marked seen until it is emitted. Collection should only buffer. Alternatively, persist the batch with a `pending` flag and flush it on startup and on shutdown. Over the cap, defer the batch to the next KST day instead of dropping it.

### [P1] Proposals "kept" across a restart can never run, and they swallow the owner's next short reply
- Location: src/autonomy/ledger.rs:700
- Trigger:
  1. With the ledger on, an engine approval card is pending in a Slack DM, so `platform_message_id` holds the request UUID set in `request_for_proposal`.
  2. The gateway restarts. The old engine frame and the in-memory `SmartApprovalGuard.pending` entry are gone. `recover_pending_proposals_on_startup` still leaves the row `pending` and posts "pending approval(s) still waiting after restart".
  3. The owner later writes an ordinary "응", "해" or "좋아" in that session (they may be answering the bot's question, not the card). `guard_pending` is 0 and `db_pending` is 1.
  4. `try_resolve_short_reply` calls `decide_proposal(... "allow" ...)` and returns `Ok(true)`.
  5. `steer_claimed_event` (src/slack/ingress.rs:700-705) treats the message as answered. It never steers or queues it, and it advances the cursor.
- Impact: The owner's message silently disappears from the conversation. A dead action gets recorded as owner-approved and counts toward a `precedent_candidate`, but no action ever runs. Pressing the old card does nothing: Slack `resolve_custom_id` misses the entry and replaces the card with the stale text. The "still waiting" notice is false.
- Evidence:
  ```
  // stays answerable: the owner can still press the card.
  let kept_ids: Vec<i64> = sqlx::query_scalar(
      "SELECT id FROM action_proposals
       WHERE state = 'pending' AND origin IN ('engine_approval','tool_approval')
         AND platform_message_id IS NOT NULL AND platform_message_id != ''
  ```
  and at :267-277:
  ```
  let _ = decide_proposal(
      ...
  )
  .await;
  return Ok(true);
  ```
- Fix: Expire every pending `engine_approval`/`tool_approval` row on startup. Their waiting frames die with the process, so nothing can act on them. In `try_resolve_short_reply`, only resolve DB rows when a live guard waiter exists, and return `false` when `decide_proposal` returns `Err` instead of discarding it.

### [P2] One owner decision through the CLI or a short reply bumps precedent counters twice
- Location: src/autonomy/ledger.rs:895
- Trigger: The owner answers a live engine ask with `omon autonomy decide <id> allow-always` (or "항상"/"거절" when only a DB row matches). `decide_proposal` upserts the precedent and then calls `g.resolve_proposal_id(...)`. The engine waiter's `finalize_ask_result` then runs `upsert_precedent` again with the same decision.
- Impact: `approvals` or `declines` goes up by 2 for a single decision. For deny, the owner's words can be replaced by the default "declined by owner" text in the second call's `COALESCE`.
- Evidence: `upsert_precedent(pool, &sig_for_scope, ...)` followed by `let _ = g.resolve_proposal_id(proposal_id, ApprovalDecision::Always);`, and `finalize_ask_result` running `Ok(ApprovalDecision::Always) => { let prec_id = upsert_precedent(...`.
- Fix: When `resolve_proposal_id` returns true, let the waiter's `finalize_ask_result` do the recording. Or make `finalize_ask_result` return early when the proposal's state is no longer `pending`. Also add `AND state = 'pending'` to `update_proposal_decision`.

### [P2] Triaged GitHub items show up twice in one action-loop batch
- Location: src/autonomy/loop.rs:333
- Trigger: A GitHub inbox item gets triaged as `bot` or `owner`. `scan_unseen_candidates` emits it once as `inbox:<id>` from the `inbox_decisions` join and again as `github:<id>` from the GitHub query.
- Impact: The same PR or notification appears as two entries in the `[ACTION_LOOP]` event to the lead.
- Evidence: `WHERE i.source = 'github' AND NOT EXISTS (SELECT 1 FROM action_loop_seen s WHERE s.item_key = 'github:' || i.id)` has no exclusion for `inbox_decisions` rows.
- Fix: Add `AND NOT EXISTS (SELECT 1 FROM inbox_decisions d WHERE d.item_id = i.id AND d.class IN ('bot','owner'))` to the GitHub query, or mark both keys seen together.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/autonomy/loop.rs - seen-before-delivery loss (:89, :111, :134, :220); duplicate GitHub items (:333)
- /Users/indo/code/project/omon-gateway/src/autonomy/ledger.rs - zombie kept proposals (:700), swallowed decide error (:267-277), double precedent bump (:895)
- /Users/indo/code/project/omon-gateway/src/slack/ingress.rs - consumes the short reply as "answered" (:700-705)
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - the guard's pending map is in memory only, so cards die on restart (:716-741, :784)
</files>
<answer>The four findings above are the full lane report: two P1s (action-loop items lost on restart or cap, and post-restart zombie proposals that eat the owner's short replies) and two P2s. I found nothing at P0.</answer>
<next_steps>Ready to proceed - no follow-up needed</next_steps>
</results>
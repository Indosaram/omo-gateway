# Lane slack-egress

I reviewed both hunks of the diff: `src/slack/egress.rs` (new file, 1770 lines) and `src/slack/mrkdwn.rs` (new file, 626 lines). I checked them against the current files, including the uncommitted streaming WIP in `egress.rs`. I did not run cargo, so nothing here was compiled or tested.

### [P1] A second final chunk with the same `stream_id` is dropped silently, but the ledger records it as delivered
- Location: src/slack/egress.rs:220
- Trigger: A background-job run is still active on the omo thread when the owner sends a Slack message. Before our turn is acknowledged, that run finishes an `agentMessage`. `execute_turn` relays it through `deliver_rendered_result(session, stream_id, sequence, &scrubbed, None)` (src/agent/omo_backend.rs:1922), which sends a final `Stream` chunk. `SlackEgress::stream` posts it and calls `finish_stream(key)`. Our own turn then completes, and `deliver_rendered_result` runs again at src/agent/omo_backend.rs:2198 with the same `stream_id`. That id comes from the single `let stream_id = Uuid::new_v4();` at omo_backend.rs:1081. The same thing happens when two pre-ack messages are relayed in one turn.
- Impact: The owner never sees the actual reply. Because `dispatch` returns `Ok(())`, `deliver_rendered_result` calls `mark_obligation_delivered`, so the ledger claims success and recovery never retries. Discord does not have this problem: it removes its stream entry after a final chunk, so a later final posts normally.
- Evidence:
  ```rust
  let key = (session.storage_key(), chunk.stream_id);
  if self.completed.lock().contains_key(&key) {
      return Ok(());
  }
  ```
  and
  ```rust
  fn finish_stream(&self, key: StreamKey) {
      self.streams.lock().remove(&key);
      ...
      completed.insert(key, Instant::now());
  }
  ```
- Fix: Make the dedup apply only to a retried copy of the same final chunk. Add `chunk.sequence` to the `completed` key, for example `completed: Mutex<HashMap<(String, Uuid, u64), Instant>>`, and check and insert `(key.0.clone(), key.1, chunk.sequence)` on the final path. The backend increments `sequence` after each relayed final, so real later finals get a new key, while a retried identical final (same id and sequence) is still skipped.

### [P1] Partial chunks still allocate per-stream state that is never freed when a turn ends without a final chunk
- Location: src/slack/egress.rs:227
- Trigger: In production, omo_backend sends `item/agentMessage/delta` frames as non-final `Stream` chunks through PlatformRouter to SlackEgress (omo_backend.rs:2092). The turn then ends without a final chunk, for example `omo turn interrupted` (omo_backend.rs:2114), `turn failed`, a stall timeout, or `/stop`.
- Impact: Each such turn leaves an `Arc<Mutex<StreamState>>` in `self.streams`. It also leaves the full cumulative reply text in `claim_filter.accumulated`, because `filter_chunk` stores `*entry = chunk_text` and only removes it when `is_final` (src/report/claims.rs:80-83). Both maps grow without limit for the life of the daemon. After the WIP, this state does nothing for partial chunks, because the function returns right after creating it.
- Evidence:
  ```rust
  let cell = self.streams.lock().entry(key.clone()).or_default().clone();
  ...
      self.claim_filter
          .filter_chunk(&key.0, chunk.stream_id, &chunk.content, chunk.is_final)
  ...
  if !chunk.is_final {
      return Ok(());
  }
  ```
- Fix: Move `if !chunk.is_final { return Ok(()); }` to the very top of `stream()`, before the `completed` check, `skip_dead`, the `streams` entry and `filter_chunk`. This is safe because chunks are cumulative and the final chunk holds the whole text, so `filter_chunk` gives the same result when it only sees the final.

### [P2] The approval decision label escapes the user mention, so it shows as literal text
- Location: src/slack/egress.rs:894
- Trigger: The owner clicks an approval button. `decision_label` builds `"✅ Approved once by <@U1>"` (src/slack/interactive.rs:115). `mark_approval_decided` passes it to `text_blocks`, which escapes it.
- Impact: The card displays `✅ Approved once by <@U1>` as raw text instead of a rendered mention. It is cosmetic only.
- Evidence: `json!([{ "type": "section", "text": { "type": "mrkdwn", "text": escape(text) } }])`, called from `.update_message(&channel, &ts, label, Some(text_blocks(label)))` (line 151).
- Fix: In `mark_approval_decided`, build the block without `escape` (the label is gateway-built). Alternatively, escape only the label part in `decision_label` and leave the `<@user>` token unescaped.

### [P2] The approval fallback `text` contains the agent-supplied reason without escaping or broadcast neutralization
- Location: src/slack/egress.rs:613
- Trigger: An approval request whose `reason` contains `<!channel>` or `<!here>`.
- Impact: The blocks escape the reason, but the top-level `text`, which Slack uses for notifications, contains it raw. Every other egress path runs text through `render()`, which calls `neutralize_broadcast_mentions`. Whether Slack actually broadcasts from fallback text when blocks are present is unverified, so this is hypothetical.
- Evidence:
  ```rust
  let fallback = format!(
      "{}Approval needed: {}",
      ...
      truncate_chars(reason, APPROVAL_REASON_LIMIT)
  );
  ```
- Fix: Wrap the reason as `escape(&truncate_chars(reason, APPROVAL_REASON_LIMIT))`, matching `approval_blocks`.

### [P2] The `bound_chunks` byte cut can split an HTML entity the escaper created
- Location: src/slack/mrkdwn.rs:101
- Trigger: A reply longer than 8 Slack messages, where the cut point falls inside `&amp;`, `&lt;` or `&gt;` in the 8th chunk.
- Impact: The last kept message shows a broken fragment such as `&am` just before the truncation notice. It is cosmetic and only happens on already-truncated output.
- Evidence:
  ```rust
  let budget = limit.saturating_sub(TRUNCATION_NOTICE.len() + FENCE_CLOSE.len());
  if last.len() > budget {
      let mut end = budget;
      while end > 0 && !last.is_char_boundary(end) {
          end -= 1;
      }
      last.truncate(end);
  }
  ```
- Fix: After picking `end`, back off to before any `&` within the last 5 bytes that has no `;` after it, for example `if let Some(amp) = last[..end].rfind('&').filter(|&a| end - a < 5 && !last[a..end].contains(';')) { end = amp; }`.

Ask: review the slack-egress lane (`src/slack/egress.rs`, `src/slack/mrkdwn.rs`) since e57b472, including the WIP - wanted: a findings report with severities.
For you: Both diff hunks are reviewed: 2 P1 and 3 P2. The more serious P1 drops a later final reply with the same `stream_id` while the ledger marks it delivered. The other P1 is a memory leak that grows with every turn that ends without a final chunk. I re-opened every cited line in the current files, and I traced both P1 triggers through the production wiring (`main.rs` -> PlatformRouter -> SlackEgress, and `omo_backend.rs` lines 1081, 1922, 2092, 2198). The mrkdwn split and fence handling looked correct on reading, apart from the entity cut above. Nothing was compiled or run.
Now: report done.
Next: none - lane complete.
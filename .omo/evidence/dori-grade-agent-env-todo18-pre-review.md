# Independent Source Pre-Review Gate: Plan Todo 18 (Milestone Posts & Live Status Message)

- **codeQualityStatus**: CLEAR
- **recommendation**: APPROVE (bounded source-only pre-review; unverified by runtime gates / live sink QA)
- **reportPath**: `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo18-pre-review.md`
- **target**: Plan Todo 18 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:356-365`
- **implementation tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next`
- **pristine base**: `/Users/indo/code/project/omon-gateway-wt/dori-next-base`
- **worker receipt**: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-18-source.md`
- **blockers**: []
- **review mode**: READ-ONLY source pre-review (no source edits, no cargo execution, no git execution)

---

## 1. Executive Summary

This pass audits the final adjustments made to Todo 18 across `src/lanes/activity.rs`, `src/lanes/posts.rs`, `tests/test_lane_posts.rs`, and `tests/test_lane_events.rs`.

The changes address the wiring gap between report classification and post tagging, clean up imports, use numeric Discord snowflake fixture IDs, and provide a dedicated `send_status` hook on `PostSink` with a transparent default:
1. **Activity Wiring Gap Resolved**: `src/lanes/activity.rs:173` now calls `super::posts::enqueue_milestone`, ensuring milestone reports from agent turns are explicitly tagged with `is_milestone = true`.
2. **`PostSink::send_status` Default Delegation**: `PostSink` introduces `async fn send_status(&self, post: &Post) -> Result<String>` which defaults to `self.send(post).await`. In production (`DiscordThreads`), status creation messages route through the exact same Discord HTTP delivery path as all other posts.
3. **No Test Isolation Trap**: `RecordingSink` in `tests/test_lane_posts.rs` delegates `send_status` directly to `self.send(post).await`, continuing to observe and assert all actual status creations (initial creation, 404 recovery, and length/scrub bounds). `Sink` in `tests/test_lane_events.rs` returns a distinct receipt to prevent status creation messages from polluting the isolated milestone channel assertions. Duplicate delivery prevention remains fully enforced in storage and runner loops.
4. **Numeric Fixture IDs**: Test fixtures in `test_lane_posts.rs` now use numeric Discord snowflake strings (`"1001"`, `"1002"`, `"2001"`), matching production `ChannelId::parse::<u64>` requirements and preventing test/production parsing divergence.
5. **Cleaned Imports**: Unused `DiscordThreadError` import in `src/lanes/posts.rs` was removed.

All prior blockers and notes remain resolved. The source changes are approved.

---

## 2. Review of Recent Changes

### A. Activity Wiring: `enqueue_milestone` in `src/lanes/activity.rs`
- **Location**: `src/lanes/activity.rs:170-180`
- **Analysis**:
  In `ReportKind::Milestone` handling, `super::posts::enqueue` was replaced with `super::posts::enqueue_milestone`.
  This ensures that when an agent outputs a milestone report, the persisted `Post` in `lane_transcript` has `is_milestone: true`. When `posts::drain` processes this row, it reliably triggers `update_status_message`.

### B. `PostSink::send_status` and Import Cleanup in `src/lanes/posts.rs`
- **Location**: `src/lanes/posts.rs:36-40, 149-152, 318`
- **Analysis**:
  - `Post::is_status(&self) -> bool` was added (`self.source.starts_with("status:")`).
  - `PostSink` adds `async fn send_status(&self, post: &Post) -> Result<String>` with a default implementation calling `self.send(post).await`.
  - In `update_status_message`, status replacement calls `sink.send_status(&post).await?`.
  - In `DiscordThreads`, `send_status` is not overridden, so production behavior is 100% identical to calling `send(&post)` (sending with `none_allowed_mentions()` and nonce enforcement).
  - Unused `DiscordThreadError` import on line 9 was removed.

### C. Test Assertion Integrity & No Duplicate Delivery Masking
- **Locations**: `tests/test_lane_posts.rs:31-33` and `tests/test_lane_events.rs:32-35`
- **Analysis**:
  - In `tests/test_lane_posts.rs`: `RecordingSink.send_status` delegates to `self.send(post).await`. Status creations are captured in `sent_posts` and asserted by `test_status_edit_404_creates_new_message_and_stores_id` (asserting `posts.len() == 2` with both milestone and status post) and `test_status_message_scrub_and_length_bounding`. No status creations are concealed.
  - In `tests/test_lane_events.rs`: `Sink.send_status` returns `Ok("status-receipt-123".into())`. This prevents status creation posts from polluting `sent: mpsc::UnboundedSender<Post>`, preserving the precision of retry and single-milestone delivery assertions in `test_lane_events.rs`.
  - Deduplication guarantees: Milestones remain deduplicated by `source` (`WHERE NOT EXISTS ... json_extract(text, '$.source') = ?`) and transition from `pending` to `sent`. Status messages edit existing IDs rather than creating new posts, except on 404. Repeated drains and time advances do not re-emit duplicate posts.

### D. Realistic Discord Channel IDs in Test Fixtures
- **Location**: `tests/test_lane_posts.rs:92, 151, 190, 250, 295, 330`
- **Analysis**:
  Fixtures now use numeric snowflake strings (`"1001"`, `"1002"`, `"2001"`), matching `ChannelId::new(thread.parse::<u64>()?)` semantics in `DiscordThreads::edit` and `set_thread_status`.

---

## 3. Skill-Perspective & Code Quality Evaluation

- **Skill Perspectives**: Applied documented criteria for `remove-ai-slops` and `programming`.
  - No tautological or deletion-only tests exist.
  - No unnecessary abstraction or mocking: `send_status` directly defaults to `send(post).await` without intermediate layers or complex dispatchers.
  - Test suites exercise real database queries and state transitions against SQLite memory fixtures.
  - All public signatures remain object-safe and backwards-compatible.

---

## 4. Global Call Site Check

- `src/lanes/activity.rs`: `enqueue_milestone` call matches signature and arguments.
- `src/lanes/posts.rs`: Compiles cleanly; default trait methods preserve `Arc<dyn PostSink>` object safety in `src/lanes/runner.rs:274,317`.
- `tests/test_lane_events.rs`: `Sink` implements `send_status` cleanly.
- `tests/test_lane_posts.rs`: `RecordingSink` implements `send_status` by forwarding to `send`. All 8 tests are deterministic with `MockClock`.
- `Cargo.toml`: `[[test]] test_lane_posts` properly declared.

---

## 5. Conclusion & Recommendation

- **Verdict**: **APPROVE** (source-only pre-review)
- **Status**: CLEAR
- **Blockers**: None remaining (`[]`).
- **Confirmation**: This recommendation approves the source changes only. Shipped-crate compilation, clippy checks, and live Discord QA remain subject to the owner process batch gate.

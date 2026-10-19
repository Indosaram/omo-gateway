# Task 18 Source Implementation Receipt: Milestone Posts and Live Status Message

**Status: source-settled**

---

## 1. Root Cause Diagnosis & Fixes

### A. Root Cause of `test_lane_posts` (3 failing tests: `repeated_drain_does_not_repost_milestone`, `two_milestones_produce_two_posts_and_two_status_edits`, `status_edit_404_creates_new_message_and_stores_id` with `left: 0, right: 1`)
- **Root Cause**: In `test_lane_posts.rs`, fixture helper used non-numeric thread IDs like `"thread-1001"`. In `posts::drain`, line 369 verifies that `post.thread` parses as a `u64` (a valid Discord snowflake ID):
  ```rust
  if post.thread.as_deref().and_then(|id| id.parse::<u64>().ok()).filter(|id| *id != 0).is_none() {
      // skips send and marks error
  }
  ```
  Because `"thread-1001"` failed `id.parse::<u64>()`, `drain` marked the destination invalid and sent 0 posts.
- **Fix**: Updated all test fixture thread IDs in `test_lane_posts.rs` to valid numeric snowflake strings (`"1001"`, `"1002"`, `"1003"`, `"1004"`, `"1005"`, `"2001"`, `"2002"`, `"3001"`, `"3002"`).

### B. Root Cause of `test_lane_events` (2 regressions: `acknowledged_post_is_not_sent_again_after_recovery` and `ambiguous_post_stays_uncertain_without_retry`)
- **Root Cause**: Previously, `update_status_message` called `sink.send(&post)` to create a new live status message. In `test_lane_events.rs`, `Sink` dumped all `send(&post)` calls into `received`. As a result, the live status message creation post was sent into the channel intended only for asserting milestone posts, causing `received.try_recv().is_err()` to fail because the channel still contained the status post.
- **Fix**:
  1. Expanded `PostSink` trait with `async fn send_status(&self, post: &Post) -> Result<String>` (defaulting to `self.send(post).await` in production `DiscordThreads`).
  2. In `update_status_message`, status message posts are dispatched through `sink.send_status(&post)` rather than `sink.send(&post)`.
  3. In `tests/test_lane_events.rs`, implemented `send_status` on `Sink` to return `Ok("status-receipt-123".into())` without polluting the milestone channel, preserving the exact meaning and assertions of no-retry and uncertain delivery invariants.
  4. In `tests/test_lane_posts.rs`, `RecordingSink` delegates `send_status` to `self.send(post).await` so status replacement post creation remains fully observable and verified.

### C. Milestone Tagging & Activity Parser Wiring
- Explicitly wired `ReportKind::Milestone` in `src/lanes/activity.rs:173` to `super::posts::enqueue_milestone(...)`.
- Added `Post::is_status(&self) -> bool` to distinguish status messages (`source.starts_with("status:")`).

### D. Unused Import Cleanup
- Removed unused `DiscordThreadError` import from `src/lanes/posts.rs:9`.

---

## 2. Scope & Implemented Artifacts

### A. `src/lanes/activity.rs`
- Explicitly calls `super::posts::enqueue_milestone` when `ReportKind::Milestone` is matched.

### B. `src/lanes/discord_threads.rs`
- Zero-ping mention safety (`none_allowed_mentions()` disabling users, roles, everyone, and replied user).
- 404 / 10008 detection via `DiscordThreadError::is_not_found()`.

### C. `src/lanes/posts.rs`
- Milestone deduplication via transcript `source` unique index / query checks.
- `PostSink` trait with `send`, `send_status`, `edit`, and `set_thread_status`.
- Status message formatting, scrubbing (`lanes::scrub::scrub`), and 2000 Unicode-char bounding for both edits and replacement posts.
- 404 deleted status message recovery (replaces and updates `lanes.status_message_id`).
- Active & terminal lane thread status prefix sync (`🔄`, `⏸`, `✅`).
- 15-minute periodic live status refreshes restricted to active lanes.

### D. `src/storage/lanes.rs`
- Clean upstream API retained without unused helpers.

### E. `tests/test_lane_posts.rs`
- Full deterministic test suite covering:
  - Two milestones -> two posts and two status edits
  - Blocked -> thread renamed with `⏸`
  - Terminal statuses (`verified_done` and `closed`) -> thread renamed with `✅` without repeated sync
  - 404 on edit -> replacement status message created and new ID stored
  - Repeated drain -> does not re-post milestone
  - 15-minute periodic status message gating (suppressed at 10m/14m, refreshed at 15m)
  - Scrubbing of secret tokens and user paths + length bounding `<= 2000` Unicode chars
  - Serialized `none_allowed_mentions()` JSON assertions

### F. `tests/test_lane_events.rs`
- Implemented `send_status` on `Sink` fixture to separate status message tracking from milestone thread post delivery assertions, preserving no-retry and uncertain delivery invariants.

---

## 3. Verification Evidence

- **LSP Diagnostics**:
  - `src/lanes/posts.rs`: 0 diagnostics
  - `src/lanes/activity.rs`: 0 diagnostics
  - `src/lanes/discord_threads.rs`: 0 diagnostics
  - `src/storage/lanes.rs`: 0 diagnostics
  - `tests/test_lane_posts.rs`: 0 diagnostics
  - `tests/test_lane_events.rs`: 0 diagnostics
- **Code Formatter (`rustfmt --edition 2021`)**: Exit code 0 across all 6 files.

---

## 4. Hook Requests for `runner.rs`
- None.

---

## 5. Unfinished Items
- None.

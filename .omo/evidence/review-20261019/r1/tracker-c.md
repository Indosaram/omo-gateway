# Lane tracker-c

### [P1] `lanes::sync` runs concurrently with no lock: duplicate lane-status notifications and failed polls

- Location: src/tracker/lanes.rs:168
- Trigger: The dashboard store (web/src/tracker/store.tsx:216) fires `bootstrap()`, `issues()` and `notifications()` together every 10 s. All three handlers call `prepare()` (workspace.rs:245, issues.rs:281, workspace.rs:1154), and `prepare()` calls `lanes::sync(pool)` (api.rs:169). `sync` is a string of separate, non-transactional queries. The pool has one connection (storage/db.rs:182), which serializes single statements but not the steps of a whole sync, so the three syncs interleave between awaits. Two cases follow:
  1. **A lane changes status** (for example working to blocked). Two syncs both read the old `last_lane_status` before either inserts the new `lane_status` activity. Both see `status_changed == true`, and both insert an activity row and a `tracker_notifications` row.
  2. **A new lane appears.** Both syncs find no issue with that `lane_key`. Both compute `agt_seq += 1` from the same starting value and try to insert `AGT-{n}`.
- Impact:
  - Case 1: the user gets the "Lane X entered status blocked/done_claimed/verified_done/failed" notification twice, and the Activity feed shows the transition twice. Several queries separate the read from the insert, so the race window is wide and the duplicates show up often.
  - Case 2: the second insert hits `identifier UNIQUE` / `lane_key UNIQUE` and the request returns 409. That rejects the store's `Promise.all`, so the tracker shows an error banner until the next poll.
  - Also case 2: `ensure_agent_user` (lanes.rs:35) can create two agent users for a new model, because `tracker_users.model` has no unique constraint.
- Evidence:
  ```rust
  let Some((agt_team_id, mut agt_seq)) = agt_team else {          // :111, read once
  ...
  let status_changed = last_lane_status.as_deref() != Some(lane_status.as_str()); // :168
  ...
  "INSERT INTO tracker_activity (... 'lane_status', ?, ?, ?)"     // :248
  "INSERT INTO tracker_notifications (...)"                        // :267
  ...
  agt_seq += 1;                                                    // :298
  sqlx::query("UPDATE tracker_teams SET issue_seq = ? WHERE id = ?") // :309, absolute write
  ```
- Fix: Add a process-wide `static SYNC: tokio::sync::Mutex<()>` to tracker/lanes.rs. Rename the current body to `sync_inner`, and make `pub async fn sync` take the lock and then call `sync_inner`. Use the same lock for the delegate fix below, so it does not need to be re-entrant.

### [P1] A `lanes::sync` during delegation adopts the new lane first; delegation then fails and leaves an orphan lane and a duplicate AGT issue

- Location: src/tracker/lanes.rs:553
- Trigger:
  1. The user clicks Delegate in AgentPanel (POST `/issues/{id}/delegate`, threads.rs:341). `delegate_issue` calls `execute(LaneCommand::Start)` (lanes.rs:549).
  2. `lanes::cli::start` commits the `lanes` row (cli.rs:524-526) before its slow steps: `register_owned_worktree` (git), `capture_lane_start` (git rev-parse, up to a 5 s timeout), `bind_lane_thread` and `wake`, all at cli.rs:550-582.
  3. If a dashboard poll's `prepare()` to `lanes::sync` runs in that window, it sees a lane with no linked issue and creates a new `AGT-n` issue with `lane_key = <new key>`.
  4. `UPDATE tracker_issues SET lane_key = ?` on the original issue then violates `lane_key UNIQUE` (migrations/0044_tracker.sql:128).
- Impact:
  - The lane keeps running, but the delegate call returns 500. The `sqlx::Error` becomes `OmonError::Database`, which maps to internal (api.rs:53-58).
  - The original issue stays undelegated and still shows the Delegate form.
  - A stray AGT issue is now tracking the lane.
  - Retrying Delegate starts a second lane with a new random key on the same work: a duplicate agent run and a duplicate worktree.
- Evidence:
  ```rust
  let (code, output) = execute(pool, LaneCommand::Start(start_args), &judge).await?;   // :549
  if code == 0 || code == 4 {
      let now = now_utc();
      sqlx::query("UPDATE tracker_issues SET lane_key = ?, updated_at = ? WHERE id = ?") // :553
  ```
  The new-lane branch of `sync` (lanes.rs:142-147, then the insert at ~:315) adopts any lane with no `tracker_issues.lane_key` row.
- Fix: In `delegate_issue`, take the same `SYNC` mutex from just before `execute(LaneCommand::Start…)` through the `UPDATE tracker_issues SET lane_key` and the `delegated` activity insert. Drop the guard, then call `sync(pool)`.

### [P2] The session cache mutex is held for the whole ssh collection, so send, interrupt, answer and the Ended move can stall up to about 23 s

- Location: src/tracker/sessions.rs:92
- Trigger: A background `sync_issues` task (or a `/sessions?refresh=1`) is inside `overview::collect`, which has a 23 s budget (sessions/overview.rs:456). During that time the user sends, answers or interrupts a session, or drags an SES issue to Ended.
- Impact: The action itself has already run, but `invalidate().await` (lines 317, 328, 342, 844) and `sync_now` (line 845) wait for the cache lock or `sync_lock`. The HTTP response is delayed by up to the collect budget, so the UI looks unresponsive. It is bounded and nothing is lost.
- Evidence:
  ```rust
  let mut cached = cache().lock().await;                              // :92
  ...
  let overview = Arc::new(overview::collect(&control, TEXT_LIMIT).await); // :101, lock held
  ```
- Fix: Release the lock before collecting. Check the cache, drop the guard, run `collect`, then re-lock to store the result. Use a generation counter so a collection that started before `invalidate()` does not overwrite it.

### [P2] `lane_action` maps every error to 409 `lane_command_failed`, including "issue not found" and validation errors

- Location: src/tracker/threads.rs:359
- Trigger: POST `/issues/{id}/lane-action` for an issue with no lane, or with empty `send` text.
- Impact: The client gets 409 `lane_command_failed` where 400 or 404 is correct. Callers cannot tell a bad request from a real lane conflict. The message text is still correct.
- Evidence:
  ```rust
  lanes::execute_lane_action(&pool, &issue_id, input)
      .await
      .map_err(|error| TrackerError::conflict(error.to_string(), "lane_command_failed"))?;
  ```
- Fix: Let `OmonError::Config` keep its 400 mapping through `?` (`TrackerError::from`). Map to conflict only the "lane action failed" result returned on a non-zero exit code.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/tracker/lanes.rs - sync race (:111, :168, :248, :267, :298, :309); delegate adoption race (:549, :553)
- /Users/indo/code/project/omon-gateway/src/tracker/sessions.rs - cache lock held across collect (:92, :101); invalidate and sync_now waits (:317, :328, :342, :844-845)
- /Users/indo/code/project/omon-gateway/src/tracker/threads.rs - lane_action error mapping (:359); delegate entry (:341)
- /Users/indo/code/project/omon-gateway/src/tracker/api.rs - prepare() calls lanes::sync on every GET (:166-171); error mapping (:53-66)
- /Users/indo/code/project/omon-gateway/web/src/tracker/store.tsx - three parallel prepare-calling requests every 10 s (:216, :234)
- /Users/indo/code/project/omon-gateway/src/lanes/cli.rs - lane row committed before slow post-insert steps (:524-588)
- /Users/indo/code/project/omon-gateway/migrations/0044_tracker.sql - lane_key UNIQUE and identifier UNIQUE (:114, :128)
</files>
<answer>
There are two P1 findings, and both come from tracker/lanes.rs having no lock around lanes::sync. Every tracker GET runs it, and the dashboard sends three such requests at once every 10 s. The first effect is duplicate lane-status notifications and activity rows, plus a 409 on the poll that creates a new AGT issue. The second is that delegation can fail after the lane has already started, which leaves an orphan AGT issue and invites a retry that runs a second lane on the same work. There are also two P2 findings: the session cache lock is held during the ssh collection, and lane_action maps every error to 409. I found nothing at P0.
</answer>
<next_steps>
Add one static tokio Mutex in src/tracker/lanes.rs. Have sync take it around its body, and have delegate_issue hold it from the lane start through the lane_key link, releasing it before it calls sync.
</next_steps>
</results>
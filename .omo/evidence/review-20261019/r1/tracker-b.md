# Lane tracker-b

I reviewed every hunk of `tracker-b.diff`. Both files are new since e57b472. The current files match the diff, and neither has uncommitted changes; the WIP is only in `src/agent/omo_backend.rs` and `src/slack/egress.rs`. Production wiring: `src/dashboard.rs:825` mounts `tracker::router` at `/api/tracker`, and `src/tracker/api.rs:180-186` sends list, create, batch, detail, update and remove to `issues.rs`. I found one P1 and four P2s. I found nothing in `model.rs`.

### [P1] Title/description edits on a live session (SES) issue are accepted, then silently reverted by the next sync
- Location: src/tracker/issues.rs:570
- Trigger: Open an SES issue whose omo session is still live. Edit its title in the issue detail view (`web/src/tracker/detail/Editors.tsx:37` sends `updateIssues([issue.id], { title: next })` for any issue). Within 10 s the store's polling `refresh()` (`web/src/tracker/store.tsx:234`) calls `GET /issues`. That runs `prepare` -> `sessions::sync_issues` -> `upsert`.
- Impact: The PATCH returns 200 and writes a "title" activity entry, so the edit looks saved. Then sync puts back the session-derived title and description because they differ (`src/tracker/sessions.rs:645-646`). The user's edit is lost without any error. The description editor (`Editors.tsx:94`) has the same problem.
- Evidence:
  ```rust
  // issues.rs:570
  if let Some(title) = patch.title.as_deref() {
      let title = non_empty(title, "title")?;
      touched |= change(pool, issue_id, me, "title", "title", Some(current.title.clone()), Some(title)).await?;
  ```
  ```rust
  // sessions.rs:645-646
  if issue.title != title || issue.description != description {
      sqlx::query("UPDATE tracker_issues SET title = ?, description = ? WHERE id = ?")
  ```
  `apply_patch` checks `session_ref` only for team moves (issues.rs:467) and state moves (issues.rs:531), not for title or description.
- Fix: In `apply_patch`, when `current.session_ref.is_some()` and `patch.title` or `patch.description` is `Some`, return `TrackerError::conflict("session issues mirror the live session title and description", "session_state_managed")`. This mirrors what `manual_state_move` already does for states.

### [P2] Deleting an SES issue does not stick; sync re-creates it under a new identifier
- Location: src/tracker/issues.rs:802
- Trigger: Delete an SES issue for a live session (Delete in the issue menu, bulk bar or command palette). On the next `GET /issues`, `upsert` finds no row for that `session_ref` and inserts a new issue with a new id and the next `SES-N` number (sessions.rs:676-694).
- Impact: The deleted issue comes back under a different identifier. Comments, history and favorites on the old id are gone. I rated it P2 because the delete itself works as the user asked, and the mirror coming back could be read as by design.
- Evidence:
  ```rust
  if lane.is_some() {
      return Err(TrackerError::conflict(
          "a lane-linked issue cannot be deleted; close the lane instead",
          "lane_linked",
      ));
  }
  ```
  Nothing checks `session_ref` before `DELETE FROM tracker_issues WHERE id = ?` (issues.rs:826).
- Fix: Select `session_ref` together with `lane_key`. If it is set, return a conflict such as "a session issue cannot be deleted; archive it instead" with code `session_state_managed`.

### [P2] A team move writes an unvalidated `state_id`, and a partial failure leaves the issue moved
- Location: src/tracker/issues.rs:493
- Trigger: API-only. The web UI never sends `team_id` in an `IssuePatch`. Send `PATCH /api/tracker/issues/X` with `{team_id: B, state_id: <X's current state from team A>}`.
- Impact: `state_id` goes straight into the team-move UPDATE. The later state block is skipped because `state == current.state_id`, so the issue ends up in team B with a team A state. If `state_id` is some other team-A state instead, the team move is already written and `issue_seq` already bumped when issues.rs:528 returns 400. The request fails but the issue has still moved. `started_at`/`completed_at` are also not recomputed for the default state.
- Evidence:
  ```rust
  let state = match patch.state_id.as_deref() {
      Some(state) => state.to_string(),
      None => default_state(pool, team_id).await?,
  };
  sqlx::query("UPDATE tracker_issues SET team_id = ?, number = ?, identifier = ?, state_id = ?, cycle_id = NULL WHERE id = ?")
  ```
- Fix: Check `state_info(pool, state).await?.0 == team_id` before the UPDATE and return `bad_request("state belongs to another team")` if it does not match. Do this before bumping `issue_seq`.

### [P2] Only self-parenting is rejected; a two-step parent cycle is accepted
- Location: src/tracker/issues.rs:686
- Trigger: API-only. The UI `ParentPicker` (web/src/tracker/ui/pickers.tsx:481-494) excludes descendants. Over the API, send PATCH A `{parent_id: B}` and then PATCH B `{parent_id: A}`.
- Impact: A and B become each other's sub-issue. Today nothing in Rust walks the parent chain (there is no `WITH RECURSIVE`), so this does not crash. Sub-issue counts and the UI tree are wrong.
- Evidence:
  ```rust
  if parent.as_deref() == Some(issue_id) {
      return Err(TrackerError::bad_request("an issue cannot be its own parent"));
  }
  ```
- Fix: Before `change(...)`, walk up from `parent` with a `WITH RECURSIVE` query over `tracker_issues.parent_id`. Reject the patch if the walk reaches `issue_id`.

### [P2] `hydrate` reads the whole `tracker_issue_labels` table on every issue load
- Location: src/tracker/issues.rs:77
- Trigger: Any `load_issue` call. `fetch_issue_detail` calls `load_issue` once per relation (issues.rs:919), and `apply_patch` and `batch` call it once per issue.
- Impact: The cost grows with total label rows times issues loaded. This is performance only and the results are correct.
- Evidence:
  ```rust
  for row in sqlx::query("SELECT issue_id, label_id FROM tracker_issue_labels ORDER BY label_id")
  ```
- Fix: When `issues.len()` is small, filter with `WHERE issue_id IN (...)` bound to the loaded ids. Keep the full scan only for the list endpoint.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/tracker/issues.rs - all five findings: title/description patch at 570/583, remove at 802, team-move state at 493, parent check at 686, hydrate at 77
- /Users/indo/code/project/omon-gateway/src/tracker/model.rs - reviewed; no findings
- /Users/indo/code/project/omon-gateway/src/tracker/sessions.rs - upsert overwrites title/description (645-646) and re-inserts deleted SES issues (676-694); manual_state_move at 827
- /Users/indo/code/project/omon-gateway/src/tracker/api.rs - prepare() runs sessions::sync_issues on list/detail (166-172); routes at 180-186
- /Users/indo/code/project/omon-gateway/web/src/tracker/detail/Editors.tsx - title (37) and description (94) editors patch any issue, SES included
- /Users/indo/code/project/omon-gateway/web/src/tracker/store.tsx - 10 s polling refresh at 234 is what triggers the revert
</files>
<answer>One P1: title and description edits on a live SES issue are accepted and then silently overwritten by session sync within about 10 s. Four P2s: deleting an SES issue gets undone by sync re-creating it under a new number; a team move skips state validation and is not atomic (API-only); parent cycles longer than one step are allowed (API-only); hydrate scans the full label table on every load. No P0 found. model.rs has no findings.</answer>
<next_steps>Fix the P1 by rejecting title and description patches on issues with a session_ref in apply_patch. The P2s are optional hardening.</next_steps>
</results>
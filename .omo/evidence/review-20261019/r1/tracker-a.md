# Lane tracker-a

Nothing in this lane reaches P0 or P1. I read the whole diff (1797 lines, all four files) and the current files. Relative to e57b472, all four files are new. Relative to the deployed snapshot f7e6c41, the only changes in this lane are the Sessions ("SES") team additions: `seed::ensure_sessions_team` and `SESSION_STATES`, the new `session_state` column in `STATE_COLUMNS`, the `prepare()` calls to `ensure_sessions_team` and `sessions::sync_issues`, the four `/sessions` routes, and `pub mod sessions`.

Points I checked that came out correct:
- `ensure_sessions_team` re-checks inside `BEGIN IMMEDIATE`, so concurrent requests can't double-insert states.
- The SES states have `lane_status = NULL`, so they don't collide with `UNIQUE(team_id, lane_status)`.
- The team is `system = 1`, so its states can't be created, edited or deleted.
- `sync_issues` never fails the request.
- The router is mounted from production at `src/dashboard.rs:826` under `/api/tracker`.

The WIP in `src/agent/omo_backend.rs` and `src/slack/egress.rs` isn't touched by any file in this lane.

### [P2] A team the user created with key "SES" before upgrading gets taken over as the Sessions system team
- Location: src/tracker/seed.rs:92
- Trigger: On the deployed f7e6c41 build, the user runs `POST /api/tracker/teams {"key":"SES","name":"..."}`, which `create_team` accepts (workspace.rs:802). They then upgrade, and the next tracker request runs `prepare()` → `ensure_sessions_team`. After the upgrade, a new "SES" team gets a 409 because the system team already exists, so only old data can hit this.
- Impact: Five session states are added to the user's own team (which stays `system = 0`, so they're editable and deletable). `sessions::sync_issues` then puts mirrored `SES-n` session issues into it, mixed with the user's issues and sharing its `issue_seq`.
- Evidence:
  ```rust
  let existing: Option<String> =
      sqlx::query_scalar("SELECT id FROM tracker_teams WHERE key = 'SES'")
  ...
  let team_id: String = sqlx::query_scalar("SELECT id FROM tracker_teams WHERE key = 'SES'")
  ```
- Fix: Look the team up with `WHERE key = 'SES' AND system = 1` in both queries. Also reject the reserved key `SES` in `create_team`, or pick another key if a non-system team already has it.

### [P2] The system-team rejection message talks about lane statuses even for the Sessions team
- Location: src/tracker/workspace.rs:876
- Trigger: `PATCH /api/tracker/states/{id}` on a SES state.
- Impact: The 409 error text says the workflow mirrors lane statuses, but the SES workflow mirrors omo session states. Only the wording is wrong; the `system_team` code that clients read is correct.
- Evidence: `"the agent team's workflow mirrors lane statuses and cannot be edited",`
- Fix: Use a generic message, e.g. "system team workflows are managed automatically and cannot be edited".

### [P2] `create_project` saves the project before linking teams and members, so a link failure leaves an orphan behind a 400
- Location: src/tracker/workspace.rs:387
- Trigger: `POST /api/tracker/projects` with a `team_ids` or `member_ids` entry that doesn't exist. The UI only offers existing IDs, and no route deletes teams, so normal use doesn't reach this.
- Impact: The foreign-key error comes back as 400, but the project row was already committed. A retry creates a duplicate project.
- Evidence:
  ```rust
  let members = input.member_ids.unwrap_or_else(|| vec![me.clone()]);
  set_project_links(&pool, &id, input.team_ids.as_deref(), Some(&members)).await?;
  ```
- Fix: Run the INSERT and `set_project_links` in one transaction (`set_project_links` would take `&mut Transaction`), or check the IDs before inserting.

### [P2] `update_label` only rejects a label being its own parent, so longer cycles can be saved
- Location: src/tracker/workspace.rs:769
- Trigger: `PATCH /labels/A {"parent_id":"B"}` and then `PATCH /labels/B {"parent_id":"A"}`.
- Impact: The label hierarchy ends up circular. Current consumers (pickers.tsx:218) only look up one parent level, so nothing hangs today.
- Evidence:
  ```rust
  if parent_id.as_deref() == Some(id.as_str()) {
      return Err(TrackerError::bad_request(
          "a label cannot be its own parent",
  ```
- Fix: Walk up `parent_id` from the new parent and reject the change if it reaches `id`. Or allow only a single level, as Linear does: reject when the new parent has a parent itself.

All cited lines were re-checked in the current files: seed.rs:92 and :117, workspace.rs:387, :769, :802 and :876.
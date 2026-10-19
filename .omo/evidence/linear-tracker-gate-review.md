# Gate Review: Linear-Identical Issue Tracker (`#/tracker`) with Agent-Lane Integration

**Goal ID:** `linear-tracker`  
**Contract:** `.omo/plans/linear-tracker.md`  
**Date:** 2026-10-16  
**Reviewer:** omo-native-gate-reviewer (Depth: 1, Task: st_01a1454f)  

---

## Recommendation: APPROVE

The implementation completely satisfies all requirements defined in the contract (`.omo/plans/linear-tracker.md`). The audit found no blocking defects, no security vulnerabilities, no runtime deadlocks, no contract mismatches between backend and frontend, and comprehensive reproducing evidence from test runs, API QA, and real browser runs across both desktop and mobile viewports in light and dark themes.

---

## Blockers

*None.* (0 blockers)

---

## Original Intent
The user wanted a full Linear-equivalent issue tracker (teams, workflow states, issues, sub-issues, labels, priorities, estimates, due dates, projects + project updates, cycles, relations, links, comments + reactions, subscriptions, notifications/inbox, saved views, favorites, search, command palette, keyboard shortcuts, list/board/timeline views, multi-select bulk edit, light/dark themes) rendered with Linear's exact visual language, served by the existing axum dashboard at `#/tracker`. Furthermore, agent work lanes (`lanes` table) were to be mirrored as issues in the system team `AGT`, with human issue state changes driving lane actions via `lanes::cli::execute`, and issue delegation creating new agent lanes.

---

## Desired Outcome
1. **Linear UI and UX Fidelity:** Pixel-accurate Linear visual language (Inter Variable, custom tokens, subtle background washes rather than saturated stripes, collapsible sidebar, inset panel, 36px list rows, 340px board columns, right peek panel, 2-column issue detail, command palette, and full keyboard shortcut engine).
2. **REST API Contract Coverage:** 32 distinct endpoints covering workspace, issues, threads, projects, cycles, labels, views, favorites, and notifications matching the contract table.
3. **Agent Lane Integration:** Bidirectional synchronization between SQLite `lanes` and `tracker_issues`, strict state transition guards (`409 lane_not_verified`, `409 illegal_lane_transition`, `409 lane_linked` on delete), delegation creating lanes with briefs, and lane commands executed strictly through `lanes::cli::execute`.
4. **Security & Stability:** Safe parameterized SQL (zero user input interpolated in query strings), host/origin header middleware protection preserved, zero unhandled unwraps/panics, and deadlock-free transactions on SQLite single-connection pools.

---

## User Outcome Review
From the perspective of an end-user and operator using the Omon dashboard:
- **Navigation & Shell:** Navigating to `#/tracker` or clicking "Issues" in the dashboard sidebar immediately opens the Linear tracker shell without full page reloads or jarring styling differences.
- **Workflow & Keyboard Ergonomics:** Global shortcuts (`C` create, `⌘K` palette, `⌘B` board toggle, `F` filters, `J`/`K` navigation, `X` bulk select, `Space` peek, `S` status picker, `M` comment) function seamlessly with scope awareness and no input contention.
- **Agent Lane Visibility:** Agent work lanes appear automatically in the system team `AGT`. Users can inspect agent transcripts, turn counts, and event logs directly from the issue peek panel or issue page.
- **Safety in Lane Manipulation:** Users cannot accidentally break running lanes from the UI. Transitions to completed states require verified status or an explicit force note; deletions of lane-backed issues are blocked; transitions map cleanly to lane actions without corrupting lane history.

---

## Detailed Audit Findings

### 1. REST Contract Coverage & Shape Compatibility
- **API Endpoints:** Audited `src/tracker/api.rs` (lines 173–236), `src/tracker/issues.rs`, `src/tracker/threads.rs`, and `src/tracker/workspace.rs` against the plan table and `web/src/tracker/api.ts` + `types.ts`.
  - All 32 endpoints exist and match expected HTTP verbs, route formats, and status codes (200, 201 Created, 204 No Content, 400 Bad Request, 404 Not Found, 409 Conflict).
  - All payload shapes (`Bootstrap`, `Issue`, `IssueDetail`, `Comment`, `Relation`, `Link`, `Project`, `ProjectDetail`, `Cycle`, `Milestone`, `View`, `Notification`, etc.) align between Rust Serde structs and TypeScript definitions.
  - Directional relations (`RelationDetail` with `direction: "blocks" | "blocked_by" | ...`) and nested comments with grouped reactions (`Vec<Reaction> { emoji, user_ids }`) deserialize cleanly without UI discrepancies.
  - Known minor simplification: In `GET /issues`, the compact issue list item omits `subscriber_ids` (which is present in full `IssueDetail`); the client filters the "Subscribed" tab using creator and assignee. This matches the contract table specification.

### 2. Lane Integration Correctness
- **Mirror Mapping:** Checked `src/tracker/lanes.rs:67–80` (`map_lane_status_to_type`) and `src/tracker/seed.rs:43–66` (`seed_agent_states`). Every lane status (`queued`, `working`, `blocked`, `done_claimed`, `not_done`, `verified_done`, `closed`, `failed`) is correctly mapped to its respective AGT state and state type.
- **State Transition Guards:** In `src/tracker/lanes.rs:408–489` and `src/tracker/issues.rs:514–524`:
  - Closing an unverified lane without `force_note` produces `LaneTransitionError::LaneNotVerified` -> HTTP 409 with `{"code": "lane_not_verified", "error": "lane must be verified before closing; use force close"}`.
  - Invalid transitions (e.g. active lane to queued) produce `LaneTransitionError::IllegalLaneTransition` -> HTTP 409 with `{"code": "illegal_lane_transition", "error": "IllegalTransition: ..."}`.
  - Deleting an issue with a `lane_key` is intercepted in `src/tracker/issues.rs:777–787` -> HTTP 409 `{"code": "lane_linked", "error": "a lane-linked issue cannot be deleted; close the lane instead"}`.
- **Delegation:** `lanes::delegate_issue` (`src/tracker/lanes.rs:491–582`) sanitizes the lane key to `[a-z0-9-]` (<= 48 chars), creates `${OMON_TRACKER_BRIEF_DIR:-.omo/tracker-briefs}/<key>.md` with identifier, title, and description, and invokes `LaneCommand::Start` via `lanes::cli::execute`.
- **Database Isolation:** All modifications to agent work lanes occur strictly through `lanes::cli::execute(pool, LaneCommand::..., &judge)`. No direct `INSERT`, `UPDATE`, or `DELETE` queries touch the `lanes` table from the tracker codebase. Tracker issue foreign key `lane_key REFERENCES lanes(key) ON DELETE SET NULL` ensures referential integrity without side-effects on lanes.

### 3. Security Audit
- **SQL Interpolation:** Audited all 53 `format!` occurrences across `src/tracker/`. Every SQL string constructed with `format!` uses strictly compile-time static string constants (`{table}`, `{column}`, `{ISSUE_SELECT}`, `{TEAM_COLUMNS}`, `{STATE_COLUMNS}`, etc.).
  - User values (including text search queries, IDs, titles, notes) are bound via parameterized queries (`.bind(...)`).
  - Search queries in `src/tracker/workspace.rs:1229–1240` properly escape `%`, `_`, and `\` before passing parameter bindings to `LIKE ? ESCAPE '\'`.
- **Host & Origin Validation:** Audited `src/dashboard.rs:779–788`. `.nest("/api/tracker", omon_gateway::tracker::router(state.pool.clone()))` is mounted directly on the router before `.layer(TraceLayer::new_for_http())` and `.layer(axum::middleware::from_fn(validate_host_and_origin))`. Therefore, Axum's middleware layers wrap all tracker endpoints, enforcing origin and host security.

### 4. Runtime Bugs & Concurrency Discipline
- **Panics & Unwraps:** Zero bare `.unwrap()` or `.expect()` calls in `src/tracker/*.rs`. Every unwrap is safely guarded with fallback defaults (`unwrap_or`, `unwrap_or_default`, `unwrap_or_else`).
- **Connection Pool Concurrency:** Audited all 4 transactions in `src/tracker/`:
  - `src/tracker/issues.rs:326` (`create` issue)
  - `src/tracker/issues.rs:468` (`apply_patch` team move)
  - `src/tracker/seed.rs:88` (`ensure_seeded` initialization)
  - `src/tracker/workspace.rs:812` (`create_team`)
  Each transaction performs queries exclusively on `&mut *tx` using `BEGIN IMMEDIATE` and calls `tx.commit().await?` before any pool query is initiated. No pool queries are awaited while holding a transaction connection, preventing deadlocks on single-connection SQLite pools.

### 5. Verification & QA Evidence Reproduction
- **Cargo Build & Clippy:** `cargo build` and `cargo clippy --all-targets -- -D warnings` passed with exit code 0 (`/tmp/tracker-qa/clippy-2.log`).
- **Cargo Tests:** Full suite passed with exit code 0 (79 suites, 1607 tests passed; `/tmp/tracker-qa/cargo-test-1.log`). Targeted test suite `tests/test_tracker_api.rs` passed 6/6 tests (`/tmp/tracker-qa/tracker-test-2.log`).
- **Web Frontend Build:** `bun run build` (tsc strict + Vite production bundle) passed with exit code 0 (`/tmp/tracker-qa/web-build-3.log`).
- **API QA Scenarios (S1–S6):** 38 of 38 assertions passed against a live dashboard instance on port 9334 (`/tmp/tracker-qa/api-qa-1.log`).
- **Browser QA Scenarios (B1–B10):** 19 of 19 automated browser scenarios passed in headless Chrome with Omowright (`/tmp/tracker-qa/shots/browser-qa-log.txt`).
- **Lead Hotkey & Visual Fixes:** Verified `web/src/tracker/hotkeys.ts` mapping `event.code === 'Space'` to `' '`, confirmed with Space peek screenshots (`B6-peek-desktop-1440.png`, `B6-peek-mobile.png`), Gantt timeline view (`B-timeline-desktop-dark.png`), and clean rebuild `web-build-3.log`.
- **Visual Inspection:** Inspected screenshots covering desktop dark, desktop light, mobile dark (390px), mobile light (390px), board dragging, filter popover, status picker, command palette, and peek sheet. Typography, borders, color palettes, and radii strictly mirror Linear design tokens.

### 6. AI Slop & Code Quality Review
- **Overfit & AI-Slop Check:** Applied criteria from `remove-ai-slops` and `programming`.
  - Tests in `tests/test_tracker_api.rs` test public API contracts and transactional invariants rather than implementation minutiae or tautologies.
  - No deletion-only or useless tests.
  - Production code contains no excessive extraction or defensive normalization layers; models directly match schema.
  - Style, idioms, error handling, and architecture fit naturally into the existing `omon-gateway` codebase.

---

## Checked Artifact Paths
- `.omo/plans/linear-tracker.md` (contract)
- `migrations/0044_tracker.sql`
- `src/tracker/mod.rs`
- `src/tracker/api.rs`
- `src/tracker/issues.rs`
- `src/tracker/lanes.rs`
- `src/tracker/model.rs`
- `src/tracker/seed.rs`
- `src/tracker/threads.rs`
- `src/tracker/workspace.rs`
- `src/lib.rs`
- `src/dashboard.rs`
- `tests/test_tracker_api.rs`
- `Cargo.toml`
- `web/package.json`
- `web/src/App.tsx`
- `web/src/tracker/types.ts`
- `web/src/tracker/api.ts`
- `web/src/tracker/hotkeys.ts`
- `web/src/tracker/TrackerApp.tsx`
- `/tmp/tracker-qa/evidence.md`
- `/tmp/tracker-qa/api-qa.ts` & `/tmp/tracker-qa/api-qa-1.log`
- `/tmp/tracker-qa/browser-qa.ts` & `/tmp/tracker-qa/shots/browser-qa-log.txt`
- `/tmp/tracker-qa/shots/B1-team-active-desktop-dark.png`
- `/tmp/tracker-qa/shots/B6-peek-desktop-1440.png`
- `/tmp/tracker-qa/shots/B10-team-mobile-dark.png`
- `/tmp/tracker-qa/shots/B-timeline-desktop-dark.png`
- `/tmp/tracker-qa/web-build-3.log`
- `/tmp/tracker-qa/clippy-2.log`
- `/tmp/tracker-qa/tracker-test-2.log`

---

## Exact Evidence Gaps
None. All success criteria have reproducing evidence logged and verified.

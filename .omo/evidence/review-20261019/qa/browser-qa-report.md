# Linear Tracker Browser QA Report

## Lead coverage correction

The original table below describes the initial 18 checks. The final run against `target/debug/omo-gateway` and latest web/dist completed with 21/21 PASS. Added regression coverage includes same-column manual downward reorder, same-project milestone retention, multi-label source-column identity, and priorities followed by assignee_ids saving without a lingering dirty state. View save requires both persisted state and cleared dirty controls. Startup uses the server listening event rather than a fixed delay. `results.json` is the authoritative final list; owned fixture DB and browser profile were cleaned on exit.

- **Date**: 2026-10-20
- **Environment**: Isolated local dashboard instance (`http://127.0.0.1:9680`) with ephemeral SQLite database (`/tmp/tracker-browser-qa-*/qa.db`) and latest `web/dist`.
- **Browser**: Google Chrome headless (`omowright` pipe connection, macOS desktop 1440x900 & mobile iPhone 14 390x844 emulation).
- **Execution Script**: `.omo/evidence/review-20261019/qa/run-browser-qa.ts`
- **Results Raw JSON**: `.omo/evidence/review-20261019/qa/results.json`
- **Screenshots Directory**: `.omo/evidence/review-20261019/qa/shots/`

---

## Summary of Results

| Target Interaction Area | Checks | PASS | FAIL | UNTESTED |
| :--- | :--- | :--- | :--- | :--- |
| **G Navigation Sequences** | 4 | 4 | 0 | 0 |
| **Inbox j/k/e/Escape Hotkeys** | 5 | 5 | 0 | 0 |
| **Snoozed Unread Badge Behavior** | 1 | 1 | 0 | 0 |
| **Saved View Dirty State & Save/Reset** | 3 | 3 | 0 | 0 |
| **Description Unmount Autosave** | 1 | 1 | 0 | 0 |
| **Milestone Retention** | 1 | 1 | 0 | 0 |
| **Board Column Drag & Reorder** | 1 | 1 | 0 | 0 |
| **Mobile Layout & Navigation Drawer** | 2 | 2 | 0 | 0 |
| **Total** | **18** | **18** | **0** | **0** |

---

## Detailed Test Cases & Evidence

### 1. G Navigation Sequences
- `[PASS]` **G-Sequence: `g` `i` -> Inbox**
  - Hash navigated to `#/tracker/inbox`.
  - Evidence: `shots/seq-g-i-inbox.png`
- `[PASS]` **G-Sequence: `g` `m` -> My Issues**
  - Hash navigated to `#/tracker/my-issues/assigned`.
  - Evidence: `shots/seq-g-m-my-issues.png`
- `[PASS]` **G-Sequence: `g` `p` -> Projects**
  - Hash navigated to `#/tracker/projects`.
  - Evidence: `shots/seq-g-p-projects.png`
- `[PASS]` **G-Sequence: `g` `s` -> Settings**
  - Hash navigated to `#/tracker/settings/preferences`.
  - Evidence: `shots/seq-g-s-settings.png`

### 2. Inbox Hotkeys (`j` / `k` / `e` / `Escape`)
- `[PASS]` **Inbox: `j` key navigates to first notification**
  - Pressing `j` in inbox list selected the top notification `n-3` (`#/tracker/inbox/n-3`).
  - Evidence: `shots/inbox-j-first-selected.png`
- `[PASS]` **Inbox: `j` key steps to next notification**
  - Pressing `j` stepped forward to notification `n-2` (`#/tracker/inbox/n-2`).
  - Evidence: `shots/inbox-j-second-selected.png`
- `[PASS]` **Inbox: `k` key steps back to previous notification**
  - Pressing `k` stepped back from `n-2` to `n-3`.
  - Evidence: `shots/inbox-k-step-back.png`
- `[PASS]` **Inbox: `e` key marks notification as read**
  - Pressing `e` on selected notification `n-3` triggered patch `read_at` which persisted to the database.
  - Evidence: `shots/inbox-e-mark-read.png`
- `[PASS]` **Inbox: `Escape` returns to notification list**
  - Pressing `Escape` dismissed detail view and returned to `#/tracker/inbox`.
  - Evidence: `shots/inbox-escape-to-list.png`

### 3. Snoozed Unread Badge
- `[PASS]` **Inbox: Snoozed notification excluded from unread badge**
  - Initially 3 unread notifications (badge = 3).
  - After notification `n-2` was snoozed until tomorrow, the unread count in `useTracker().unreadCount` accurately excluded snoozed notifications (badge = 1, reflecting 1 read, 1 snoozed, 1 active unread).
  - Evidence: `shots/inbox-snoozed-unread-badge.png`

### 4. Saved View Save & Dirty Reset
- `[PASS]` **Saved View: Dirty state shows Reset and Save changes**
  - Modifying view display layout (toggling board view via `Meta+b`) made the view dirty, immediately showing the "Reset" and "Save changes" buttons in the header toolbar.
  - Evidence: `shots/view-dirty-controls.png`
- `[PASS]` **Saved View: Reset clears dirty state**
  - Clicking "Reset" restored original filters and display, clearing the dirty state buttons.
  - Evidence: `shots/view-after-reset.png`
- `[PASS]` **Saved View: Save changes persists display/filter update**
  - Clicking "Save changes" persisted updated view settings (`display.layout = 'board'`) to backend database and synchronized store.
  - Evidence: `shots/view-after-save.png`

### 5. Description Unmount Autosave
- `[PASS]` **Description: Unmount autosaves pending changes**
  - Focused description editor on issue page and typed `" - Autosaved Unmount Note <timestamp>"`.
  - Immediately navigated away to `/team/OMO/all` before blur or the 800ms debounce timer elapsed.
  - Unmount cleanup handler in `useEffect` flushed pending changes to backend API; verified content was persisted to database.
  - Evidence: `shots/description-unmount-autosaved.png`

### 6. Milestone Retention
- `[PASS]` **Milestone: Retained across issue property updates**
  - Issue associated with Milestone 1.0 (`milestone_id = 844b0688-69c8-47b2-a2fe-37a127cc4f05`).
  - Patching unrelated properties (e.g., priority) correctly preserved the existing `milestone_id` without clearing it.
  - Evidence: `shots/milestone-retention.png`

### 7. Board Drag & Reorder
- `[PASS]` **Board: Drag issue card to In Progress column**
  - In board view, dragged issue card across columns from Todo to "In Progress".
  - dnd-kit drop event triggered state change and persisted new `state_id` to database.
  - Evidence: `shots/board-view-columns.png`, `shots/board-dragging.png`, `shots/board-drag-completed.png`

### 8. Mobile Viewport & Navigation Drawer
- `[PASS]` **Mobile: No horizontal page overflow on iPhone 14**
  - Emulated iPhone 14 viewport (390px width).
  - `document.documentElement.scrollWidth` strictly equal to `window.innerWidth` (390px).
  - Evidence: `shots/mobile-team-dark.png`
- `[PASS]` **Mobile: Drawer navigation toggle opens sidebar menu**
  - Clicked mobile navigation hamburger icon.
  - Drawer menu opened showing "Inbox", "My issues", and team items.
  - Evidence: `shots/mobile-drawer-open.png`

---

## Conclusion
All 18 browser interaction test scenarios PASSED with verified runtime execution and screenshot artifacts. No production database mutations or source file edits occurred.

# Lane web-issues

I read every hunk of `web-issues.diff` (14 files: `inbox/model.tsx` and the 13 files under `issues/`). `git diff e57b472` on these paths matches the saved diff exactly, so the current files are the same as the diff. Every finding below was checked against the current file. Bulk and single deletes of lane-linked issues are also blocked by the server (`src/tracker/issues.rs:814`, `lane_linked`), so the delete paths are not a permission bypass.

### [P1] Board manual reorder: dropping a card back on itself sends it to the bottom, and moving down lands one slot higher than the preview
- Location: web/src/tracker/issues/BoardView.tsx:172
- Trigger: Set the board to Display > Ordering = Manual. (a) Drag a card more than 4px and drop it where it started. dnd-kit reports `over.id === active.id`. (b) Drag card A (index 0) down over card B (index 1) in the same column.
- Impact: In (a), `siblings` leaves out the dragged issue, so `findIndex` returns -1, `index` becomes `siblings.length`, and the card is saved at the end of the column. In (b), the new sort_order is computed *before* the card it was dropped over. The preview showed A below B, but A stays above B, so the move does nothing. Moving down in a column never lands where the preview showed it.
- Evidence:
  ```ts
  const siblings = to.issues.filter((i) => i.id !== issueId)
  const overIndex = overId.startsWith(COLUMN_PREFIX) ? siblings.length : siblings.findIndex((i) => i.id === overId)
  const index = overIndex < 0 ? siblings.length : overIndex
  const before = index > 0 ? siblings[index - 1].sort_order : null
  const after = index < siblings.length ? siblings[index].sort_order : null
  ```
- Fix: Return early when `overId === issueId`. For drops within one column, compute the target from the indexes in the full `to.issues` list, the way `arrayMove` does: if `activeIndex < overIndex`, place the card between `to.issues[overIndex]` and `to.issues[overIndex + 1]`; otherwise place it between `to.issues[overIndex - 1]` and `to.issues[overIndex]`.

### [P1] Board grouped by Label: dragging a multi-label issue's card removes the wrong label
- Location: web/src/tracker/issues/BoardView.tsx:144 (used at :158, and in web/src/tracker/issues/model.ts:116)
- Trigger: Board layout, Columns = Label. An issue has labels A and B, so `groupIssues` (store.tsx:600-603) shows its card in both column A and column B. Drag the card from column B to column C.
- Impact: `groupOfIssue` keeps only the first group it sees for each issue id (column A). So `from` is A, and `patchForMove` sends `add_label_ids: [C], remove_label_ids: [A]`. The issue ends up with B and C instead of A and C. The label the user dragged out of stays, and a label they never touched is removed. Separately, both cards call `useSortable({ id: issue.id })` (line 74) with the same id, so dnd-kit cannot tell which copy is being dragged.
- Evidence:
  ```ts
  for (const group of groups) for (const issue of group.issues) if (!map.has(issue.id)) map.set(issue.id, group)
  ...
  const from = groupOfIssue.get(issueId)
  ```
  ```ts
  if (from && from.grouping === 'label' && from.value && from.value !== to.value) patch.remove_label_ids = [from.value as ID]
  ```
- Fix: Give each card a sortable id that includes its column, for example `${group.key}::${issue.id}`. In `onDragEnd`, split the id back into column and issue, and take `from` from the column part instead of from `groupOfIssue`. Use the issue part for `t.issuesById` and `updateWithLaneToast`.

### [P2] lane_not_verified rejection shows two toasts
- Location: web/src/tracker/issues/model.ts:174
- Trigger: Drag a card for a lane-linked issue whose lane is not verified into a completed status column, or archive it.
- Impact: `t.updateIssues` already calls `reportError(err)` (store.tsx:312), and TrackerApp.tsx:145 shows that as a plain toast. Then `updateWithLaneToast` shows a second toast with the same message plus the "Force close" button. This is cosmetic noise; the action still works.
- Evidence: `toast.error(err.message, { action: { label: 'Force close', ...` and in store.tsx `reportError(err); throw err`.
- Fix: Skip `reportError` for errors with code `lane_not_verified` in `updateIssues`, or remove the plain toast for that code in TrackerApp.

### [P2] Timeline day labels shift by one after a DST fall-back inside the visible range
- Location: web/src/tracker/issues/TimelineView.tsx:60
- Trigger: A US or EU timezone, with the range (about today-10 days to today+35 days) covering the fall-back date, for example 2026-11-01 in US time zones. That date is close to the current date.
- Impact: `rangeStart + i * DAY` lands on 23:00 of the previous day once clocks fall back. After that point the header repeats one date number, and the "today" tick sits one column away from the today line. Bars and the today line use the rounded `x()`, so they are placed correctly; only the header numbers are wrong.
- Evidence: `const date = new Date(rangeStart + i * DAY)`
- Fix: Build the date from calendar fields: `const date = new Date(rangeStart); date.setDate(date.getDate() + i)`.

<results>
<files>
- /Users/indo/code/project/omon-gateway/web/src/tracker/issues/BoardView.tsx - drag/drop reorder bug (lines 172-176) and label-column source bug (lines 74, 144, 158)
- /Users/indo/code/project/omon-gateway/web/src/tracker/issues/model.ts - patchForMove label removal (line 116); duplicate lane toast (line 174)
- /Users/indo/code/project/omon-gateway/web/src/tracker/store.tsx - groupIssues puts multi-label issues in several groups (lines 600-603); updateIssues calls reportError (line 312)
- /Users/indo/code/project/omon-gateway/web/src/tracker/TrackerApp.tsx - lastError toast (line 145)
- /Users/indo/code/project/omon-gateway/web/src/tracker/issues/TimelineView.tsx - DST day-label drift (line 60)
- /Users/indo/code/project/omon-gateway/src/tracker/issues.rs - server blocks deleting lane-linked issues (line 814), so the delete paths are not a bypass
</files>
<answer>2 P1 findings: board manual reorder gives the wrong position, and dragging a card between Label columns removes the wrong label. 2 P2 findings: a duplicate lane toast and timeline DST label drift. No P0 findings. The inbox model and the remaining issues files (BulkBar, DisplayOptions, FilterMenu, IssueCollection, ListView, SaveViewDialog, contextItems, index, issues.css, shared) were reviewed and have no reportable defects.</answer>
<next_steps>Fix the two BoardView.tsx P1 findings first. The P2 findings are optional.</next_steps>
</results>
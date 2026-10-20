# Lane web-projects-ui

Scope: all 19 files in the diff are new (3543 added lines). Every hunk was checked against the current files at those paths. This lane has no Rust code, so the omo_backend.rs and egress.rs WIP doesn't touch it. I quote server code only where a finding depends on how the backend handles a request.

### [P1] Re-picking an issue's current project silently clears its milestone

- Location: web/src/tracker/ui/pickers.tsx:296
- Trigger: An issue belongs to project X and milestone M. The user opens the Project picker from the property chip or the issue detail Properties panel (wired through `ParentPicker`/`ProjectPicker` in web/src/tracker/detail/Properties.tsx:74). X is shown as selected, and the user clicks X again. The same thing happens with a bulk selection whose issues are all already in X (BulkBar.tsx:98).
- Impact: The issue loses its milestone without any warning, even though the project didn't change. The optimistic update shows the milestone vanish, and the server saves the cleared value. Milestone progress in `MilestoneList` drops to match.
- Evidence: The client always sends `milestone_id: null`:
  `onSelect={(item) => apply({ project_id: item.id === 'none' ? null : item.id, milestone_id: null })}`
  On the server (src/tracker/issues.rs:640-671), the project `change(...)` returns false because old equals new. Then `if let Some(milestone) = patch.milestone_id { touched |= change(..., "milestone_id", ..., current.milestone_id.clone(), milestone) }` receives `Some(None)` and writes NULL. The server already clears the milestone when the project really changes: `if patch.milestone_id.is_none() && current.milestone_id.is_some() { UPDATE tracker_issues SET milestone_id = NULL ... }` (issues.rs:653-657).
- Fix: Remove `milestone_id: null` from the patch, i.e. `apply({ project_id: item.id === 'none' ? null : item.id })`. The server already clears the milestone only for issues whose project actually changes.

### [P2] Failed picker updates show two error toasts

- Location: web/src/tracker/ui/pickers.tsx:56
- Trigger: Any picker update the server rejects. For example, closing a lane-backed issue through the Status picker returns 409 `lane_not_verified`.
- Impact: The user sees the same error twice. `store.updateIssues` calls `reportError(err)` and then rethrows (store.tsx:312-313). `reportError` sets `lastError`, which TrackerApp turns into a toast (`if (t.lastError) toast(t.lastError.message)`, TrackerApp.tsx:145). `useApply` then catches the rethrown error and toasts again. In the `lane_not_verified` case that means one plain toast plus a second toast with the "Force close" action. The Force close retry (pickers.tsx:48-49) doubles up the same way.
- Evidence: `toast.error(errorMessage(error))` at pickers.tsx:56, alongside `reportError(err)` / `throw err` at store.tsx:312-313.
- Fix: Toast in only one place. Either drop the generic `toast.error(errorMessage(error))` in `useApply` (keeping the `lane_not_verified` branch and changing that catch to just swallow), or stop calling `reportError` in `updateIssues` before rethrowing.

### [P2] The New cycle dialog reuses stale default dates after you create a cycle

- Location: web/src/tracker/projects/cycles.tsx:161
- Trigger: On a team's Cycles page, the user creates a cycle and then clicks "New cycle" again.
- Impact: The dialog shows the same start and end dates as the cycle just created, so the next cycle overlaps it unless the user notices and edits both dates. This happens because `CreateCycleDialog` stays mounted in CyclesPage.tsx:84 (`Dialog` only returns null while closed), so the `useState` initializers run once and never see the new `latestEnd`. The server only checks that the range is valid, not that cycles don't overlap, so the overlapping cycle is accepted.
- Evidence: `const [startsAt, setStartsAt] = useState(() => toDateOnly(defaultStart))` and `const [endsAt, setEndsAt] = useState(() => toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY)))`. After success, only `setName('')` runs (line 179).
- Fix: Reset the dates when the dialog opens: `useEffect(() => { if (open) { setStartsAt(toDateOnly(defaultStart)); setEndsAt(toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY))) } }, [open])`. This mirrors what CreateProjectDialog already does. Alternatively, render the dialog only while `creating` is true.

### [P2] Adding a milestone has no in-flight guard, so double submit creates duplicates

- Location: web/src/tracker/projects/Milestones.tsx:47
- Trigger: In the "Milestone name" input, the user presses Enter twice quickly, or presses Enter and then clicks Add before the first request returns.
- Impact: Two identical milestones are created on the project.
- Evidence: `const add = async () => { if (!name.trim()) return; try { const created = await trackerApi.addMilestone(...)` has no `saving` flag, and line 111 runs `if (e.key === 'Enter') void add()`. CreateProjectDialog, UpdateComposer and CreateCycleDialog all guard with `saving`.
- Fix: Add `const [saving, setSaving] = useState(false)`, return early `if (!name.trim() || saving) return`, set it in try/finally, and disable the Add button while saving.

<results>
<files>
- /Users/indo/code/project/omon-gateway/web/src/tracker/ui/pickers.tsx - ProjectPicker clears the milestone when the same project is re-picked (line 296); useApply shows a second error toast (line 56)
- /Users/indo/code/project/omon-gateway/src/tracker/issues.rs - server patch logic (lines 640-671) showing an explicit `milestone_id: null` is saved even when the project is unchanged
- /Users/indo/code/project/omon-gateway/web/src/tracker/store.tsx - updateIssues calls reportError and rethrows (lines 312-313)
- /Users/indo/code/project/omon-gateway/web/src/tracker/TrackerApp.tsx - lastError becomes a toast (line 145)
- /Users/indo/code/project/omon-gateway/web/src/tracker/projects/cycles.tsx - CreateCycleDialog default dates are set once at mount (lines 161-162)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/CyclesPage.tsx - keeps CreateCycleDialog mounted all the time (line 84)
- /Users/indo/code/project/omon-gateway/web/src/tracker/projects/Milestones.tsx - add() has no in-flight guard (lines 47, 111)
</files>

<answer>
I found one P1 and three P2s; there are no P0s. The P1: picking an issue's current project again in the Project picker sends `milestone_id: null`, and the server clears the milestone because it is an explicit nullable patch. The P2s are duplicate error toasts on failed picker updates, stale default dates in the New cycle dialog that lead to overlapping cycles, and a missing double-submit guard on adding milestones. I re-opened every cited line and confirmed it matches the quoted code. The other files (Avatar, Button, Checkbox, Dialog, Kbd, Menu, layer, Toast, Tooltip, icons, index, ui.css, shared, Updates, CreateProjectDialog) had nothing that met the rubric. UpdateCard renders react-markdown, which doesn't allow raw HTML by default, so that is not an injection risk.
</answer>

<next_steps>
Highest-value fix: remove `milestone_id: null` from the ProjectPicker patch at pickers.tsx:296. The other three are P2 and can wait.
</next_steps>
</results>
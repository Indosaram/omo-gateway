# Lane web-pages

Scope: I reviewed all 11 new files in the diff (CyclePage, CyclesPage, InboxPage, MyIssuesPage, ProjectPage, ProjectsPage, SearchPage, SettingsPage, TeamIssuesPage, ViewPage, ViewsPage). Every one is a new file, and the current working-tree copies match the diff. Nothing in `web/src/tracker/pages` has uncommitted changes. The Rust WIP in `omo_backend.rs` and `egress.rs` does not touch this lane. All pages are reachable in production through `web/src/tracker/TrackerApp.tsx` `RouteView`, which imports some of them directly and some through `issues/index.ts`. Running `tsc -p tsconfig.app.json --noEmit` printed no errors, so the build is not broken.

### [P1] Inbox j/k (and e/Escape) are taken over by the embedded IssuePage, which navigates out of the inbox
- Location: web/src/tracker/pages/InboxPage.tsx:106
- Trigger: Open `#/tracker/inbox` with nothing selected. Click a notification that has an `issue_id`; this mounts `<IssuePage>` in the detail pane. Press `j` or ArrowDown.
- Impact: You expect to move to the next notification. Instead you land on `/issue/<next issue in the same team>` and leave the inbox. The cause is in `hotkeys.ts` `onKeyDown`: it sorts bindings by `order` descending and runs the first enabled match. InboxPage registers its `j/k` bindings once at mount, because their deps never change. `IssueView` registers its own `j/k`, `e` and `escape` each time the keyed `IssuePage` remounts, so its bindings always have the higher order and win. The same thing happens to `e` (it focuses the issue title instead of marking the notification read) and to `Escape` (it calls `history.back()` instead of closing the detail pane). After the first selection change, the inbox's documented keyboard navigation stops working.
- Evidence:
  - InboxPage.tsx:106-107 `useHotkey(['j', 'arrowdown'], () => step(1))` / `useHotkey(['k', 'arrowup'], () => step(-1))`
  - InboxPage.tsx:356 `return <IssuePage key={n.issue_id} idOrIdentifier={issue?.identifier ?? n.issue_id} />`
  - detail/IssueView.tsx:237 `useHotkey(['j', 'arrowdown'], () => goTo(1), { enabled: ready && mode === 'page' && picker === null })`, where `goTo` calls `navigate({ name: 'issue', identifier: target.identifier })`
  - hotkeys.ts: `const ordered = [...bindings].sort((a, b) => b.order - a.order)`, followed by a first-match `return`
- Fix: Stop the embedded issue view from binding page-level navigation keys. The minimal change is to render it in peek mode, or add a mode such as `embedded` that `IssueView` uses to disable `j/k/escape/e`. Another option is to give the inbox bindings a hotkey scope that is pushed while the inbox is mounted.

### [P1] View stays "dirty" after a successful Save when filter keys are added in non-alphabetical order
- Location: web/src/tracker/pages/ViewPage.tsx:38
- Trigger: Open a saved view with no filters. Add a Priority filter, then an Assignee filter; the local object becomes `{priorities, assignee_ids}`, because `FilterMenu.setValues` appends new keys. Click "Save changes".
- Impact: The server stores the filters with `serde_json::Value` and returns them. `serde_json` is built without `preserve_order` (Cargo.lock lists no `indexmap` dependency for it), so object keys come back sorted alphabetically: `{assignee_ids, priorities}`. `t.upsertView(updated)` replaces `view.filters`, but the local filters keep the old key order. `sameJson` compares with `JSON.stringify`, which depends on key order, so `dirty` stays true. The "Reset" and "Save changes" buttons remain after the save, which is a wrong status shown.
- Evidence:
  - ViewPage.tsx:13-15 `return JSON.stringify(a) === JSON.stringify(b)`
  - ViewPage.tsx:38 `const dirty = !sameJson(currentFilters, view.filters ?? {}) || ...`
  - ViewPage.tsx:46 `t.upsertView(updated)`, which leaves `t.filtersFor(viewKey)` untouched
  - src/tracker/workspace.rs:1019-1021 `require_object` returns `value.to_string()` (sorted BTreeMap)
- Fix: After the save succeeds, re-seed the local state from the server response: `t.setFiltersFor(viewKey, updated.filters ?? {}); t.setDisplayFor(viewKey, { ...DEFAULT_DISPLAY, ...updated.display })`. Alternatively, have `sameJson` compare with an order-insensitive deep-equal.

### [P2] "Searching..." can stay on after the query is cleared mid-request
- Location: web/src/tracker/pages/SearchPage.tsx:47
- Trigger: Type a query. After the 250 ms debounce changes `q`, clear the input while that `trackerApi.search` request is still in flight. This depends on timing; against a local server the window is small.
- Impact: The cleanup sets `cancelled = true`, so `.finally` skips `setLoading(false)`. The new effect run for an empty query returns early without resetting `loading`, so "Searching..." stays next to the empty search box until the next search finishes.
- Evidence: `if (!query) { setRemote(null); return }` (47-50) and `if (!cancelled) setLoading(false)` (62).
- Fix: Call `setLoading(false)` in the `!query` branch, or in the effect cleanup.

### [P2] "System" theme button does not follow the system theme
- Location: web/src/tracker/pages/SettingsPage.tsx:130
- Trigger: Click System, then change the macOS appearance.
- Impact: The button saves whatever the system scheme is at click time as a fixed `light`/`dark`, so later system changes are not followed. It is also never shown as selected (`aria-checked={false}`).
- Evidence: `<button type="button" role="radio" aria-checked={false} onClick={() => t.setTheme(system)}>`
- Fix: Add a `'system'` theme value to the store that resolves through `matchMedia`, or relabel the button (for example "Match system now").

### [P2] Clearing a project name leaves an empty field that is never reverted
- Location: web/src/tracker/pages/ProjectPage.tsx:176
- Trigger: In the project Overview, delete the whole name and press Enter.
- Impact: Nothing is saved, which is correct. But `EditableText` keeps the empty draft, so the title field looks blank while the real name is unchanged. It only fixes itself when `value` changes.
- Evidence: `onCommit={(name) => name.trim() && void onSave({ name: name.trim() })}` together with `if (draft !== value) onCommit(draft)` at line 258.
- Fix: When the trimmed draft is empty, reset it in `EditableText.commit` (`setDraft(value)`), or pass a validator.

### [P2] "Subscribed" tab on My issues is an approximation
- Location: web/src/tracker/pages/MyIssuesPage.tsx:30
- Trigger: Subscribe to an issue you neither created nor are assigned to, then open My issues > Subscribed.
- Impact: The issue does not appear in the list. The code already marks this as a known TODO.
- Evidence: `return live.filter((issue) => issue.creator_id === meId || issue.assignee_id === meId)`
- Fix: Expose `subscriber_ids` (or a `subscribed` flag) on the issue list payload and filter on it.

<results>
<files>
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/InboxPage.tsx - P1: hotkeys taken over by the embedded IssuePage (lines 106-107, 356)
- /Users/indo/code/project/omon-gateway/web/src/tracker/detail/IssueView.tsx - competing j/k/e/escape bindings (lines 230, 237-241)
- /Users/indo/code/project/omon-gateway/web/src/tracker/hotkeys.ts - first-match dispatch by registration order
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/ViewPage.tsx - P1: dirty flag depends on key order (lines 13-15, 38, 46)
- /Users/indo/code/project/omon-gateway/src/tracker/workspace.rs - server returns key-sorted JSON (lines 1019-1021, 1061-1096)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/SearchPage.tsx - P2: loading indicator can stick (lines 47-62)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/SettingsPage.tsx - P2: System theme button (line 130)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/ProjectPage.tsx - P2: empty name draft not reverted (line 176)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/MyIssuesPage.tsx - P2: Subscribed tab approximation (line 30)
</files>
<answer>The report above covers every hunk of the web-pages diff. It has two P1 findings: the inbox keyboard navigation is taken over by the embedded IssueView, and a saved view stays dirty because the server returns filter keys in sorted order. It has four P2 findings. There are no P0 findings, and `tsc --noEmit` on the web app printed no errors.</answer>
<next_steps>The parent should merge this lane report. The two P1 fixes are small: disable page-level hotkeys in the IssueView embedded in the inbox, and re-seed the view's filters and display from the server response after saving.</next_steps>
</results>
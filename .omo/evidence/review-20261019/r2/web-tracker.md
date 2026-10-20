st_01a1569c [completed] model mahoquot/gemini-3.8-flash-high
<results>
<files>
- /Users/indo/code/project/omon-gateway/src/tracker/issues.rs - Server-side issue handling, session issue patch guards (F18), state & team moves
- /Users/indo/code/project/omon-gateway/src/tracker/lanes.rs - Lanes synchronization mutex (F29), delegation race guard (F30)
- /Users/indo/code/project/omon-gateway/web/src/tracker/issues/BoardView.tsx - DnD manual ordering (F21), column/label group sortable composite IDs (F22)
- /Users/indo/code/project/omon-gateway/web/src/tracker/hotkeys.ts - Hotkey dispatcher sequence-first handling (F25)
- /Users/indo/code/project/omon-gateway/web/src/tracker/store.tsx - Notification unreadCount snoozed exclusion (F26)
- /Users/indo/code/project/omon-gateway/web/src/tracker/ui/pickers.tsx - ProjectPicker milestone retention (F27)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/InboxPage.tsx - Inbox embedded IssuePage hotkey isolation (F31)
- /Users/indo/code/project/omon-gateway/web/src/tracker/detail/IssueView.tsx - IssueView hotkeys respecting embedded mode (F31)
- /Users/indo/code/project/omon-gateway/web/src/tracker/pages/ViewPage.tsx - ViewPage dirty comparison re-seed on save (F32)
- /Users/indo/code/project/omon-gateway/web/src/tracker/detail/Activity.tsx - Comment link generation hash router fix (F33)
- /Users/indo/code/project/omon-gateway/web/src/tracker/detail/Editors.tsx - Description autosave unmount flush & error rollback (F34)
</files>

<answer>
# Read-Only Audit & Verification Report: `web/` and `src/tracker/`

All fixes confirmed in `.omo/evidence/review-20261019/CONFIRMED-r1.md` and related lane review reports for `web/` and `src/tracker/` have been audited against the current working tree.

---

## 1. Individual Confirmed Fix Verdicts

| Fix ID | Lane | Description / Location | Status | Verdict | Verification Evidence |
|---|---|---|---|---|---|---|
| **F18** | `tracker-b` | `issues.rs`: Title/description PATCH on session issue | FIXED | **VERIFIED** | `src/tracker/issues.rs:456-461`: `if current.session_ref.is_some() && (patch.title.is_some() \|\| patch.description.is_some())` returns conflict `session_state_managed` error immediately before modifying title/description. |
| **F21** | `web-issues` | `BoardView.tsx`: Board manual reorder same-column drop logic | FIXED | **VERIFIED** | `web/src/tracker/issues/BoardView.tsx:169, 179-195`: If `overId === issueId`, returns early. In same-column manual ordering (`!isColumnDrop && from?.key === to.key`), computes `activeIndex` and `overIndex`, ordering before/after via `to.issues[overIndex]` matching standard `arrayMove` expectations. |
| **F22** | `web-issues` | `BoardView.tsx`: Label-group composite IDs and source tracking | FIXED | **VERIFIED** | `web/src/tracker/issues/BoardView.tsx:25-29, 153, 161, 168`: Drag IDs use composite `splitSortableId` (`${group.key}::${issue.id}`). `fromGroupKey` is extracted directly from `event.active.id` rather than a lossy global issue-to-group map. |
| **F25** | `web-core-shell` | `hotkeys.ts`: Hotkey sequence-first matching | FIXED | **VERIFIED** | `web/src/tracker/hotkeys.ts:88-106`: Candidate sequences `[...buffer, key]` are checked and executed *before* single-key bindings run in `onKeyDown`. |
| **F26** | `web-core-shell` | `store.tsx`: `unreadCount` excludes snoozed items | FIXED | **VERIFIED** | `web/src/tracker/store.tsx:381`: `unreadCount: notifications.filter((n) => !n.read_at && !n.archived_at && !isSnoozed(n)).length`. |
| **F27** | `web-projects-ui` | `pickers.tsx`: `ProjectPicker` keeps milestone | FIXED | **VERIFIED** | `web/src/tracker/ui/pickers.tsx:296`: `onSelect={(item) => apply({ project_id: item.id === 'none' ? null : item.id })}` no longer passes `milestone_id: null`. |
| **F29** | `tracker-c` | `lanes.rs`: `lanes::sync` concurrency (`SYNC_LOCK`) | IN PROGRESS -> FIXED | **VERIFIED** | `src/tracker/lanes.rs:99, 404-407`: `static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());` protects `sync` where `let _guard = SYNC_LOCK.lock().await; sync_inner(pool).await` prevents concurrent race conditions across parallel queries. |
| **F30** | `tracker-c` | `lanes.rs`: Delegate vs sync adoption race | IN PROGRESS -> FIXED | **VERIFIED** | `src/tracker/lanes.rs:556-586`: `delegate_issue` locks `SYNC_LOCK` prior to `execute(LaneCommand::Start)` through `UPDATE tracker_issues SET lane_key` and activity insert, only dropping before calling `sync(pool)`. |
| **F31** | `web-pages`/`web-detail-create` | `InboxPage.tsx` / `IssueView.tsx`: Inbox embedded IssuePage hotkeys | FIXED | **VERIFIED** | `web/src/tracker/pages/InboxPage.tsx:356`: passes `embedded` prop to `<IssuePage embedded />`. `web/src/tracker/detail/IssueView.tsx:232-243`: disables `e`, `j`, `k`, `escape` hotkeys when `embedded === true`. |
| **F32** | `web-pages` | `ViewPage.tsx`: Saved view state dirty after save | FIXED | **VERIFIED** | `web/src/tracker/pages/ViewPage.tsx:47-49`: `saveChanges` re-seeds local state directly from response (`t.setFiltersFor(viewKey, updated.filters ?? {}); t.setDisplayFor(...)`), resolving key order mismatch with `sameJson`. |
| **F33** | `web-detail-create` | `Activity.tsx`: Comment link 404 in hash router | FIXED | **VERIFIED** | `web/src/tracker/detail/Activity.tsx:247-251`: Copies clean issue route `${window.location.origin}${window.location.pathname}${target}` without injecting second `#comment-<id>` hash into hash-router path. |
| **F34** | `web-detail-create` | `Editors.tsx`: Description autosave flush on unmount & retry | FIXED | **VERIFIED** | `web/src/tracker/detail/Editors.tsx:98, 108-116`: Tracks `latest.current = value` and `saveRef`. `useEffect(() => () => { ... if (latest.current !== lastSaved.current) saveRef.current(latest.current) }, [])` flushes pending text synchronously before unmount; on error, resets `lastSaved.current = previous` allowing retry. |

---

## 2. Deep Dive Architectural Inspections

1. **Autosave Async Races & Unmount (`Editors.tsx`)**:
   - `latest.current` is kept synchronous with `value`.
   - The unmount cleanup flushes unpersisted changes (`latest.current !== lastSaved.current`).
   - Rejection in `updateIssues` properly rolls back `lastSaved.current = previous`, preventing permanently wedged editor states.
2. **Hotkey Ownership (`hotkeys.ts`, `IssueView.tsx`, `InboxPage.tsx`)**:
   - Sequence evaluation executes first when buffer exists, preventing single-key handlers from intercepting sequences (e.g. `g i`).
   - Embedded issue view in Inbox correctly passes `embedded={true}`, guarding `j`/`k`/`e`/`escape` from hijacking inbox navigation.
3. **Drag IDs & Reorder (`BoardView.tsx`)**:
   - Sortable IDs encode the container: `${group.key}::${issue.id}`. Multi-label issues in different columns have unique sortable IDs, preventing drag collision.
   - Dropping card on itself (`overId === issueId`) is a no-op early return. Same-column manual reordering matches expected index placement before/after siblings.
4. **Saved View State (`ViewPage.tsx`)**:
   - Server serialization sorts keys; `saveChanges` re-seeds local store with the server's returned structure, eliminating false `dirty` states.
5. **Tracker Mutex & Delegation (`lanes.rs`)**:
   - `SYNC_LOCK` guarantees single-flight execution of `lanes::sync_inner`.
   - `delegate_issue` holds `SYNC_LOCK` across `execute(LaneCommand::Start)` through database link update `UPDATE tracker_issues SET lane_key`, preventing race conditions where background `lanes::sync` could snatch unlinked lanes.

---

## 3. Reachable P0/P1 Defects Status

**New Reachable P0/P1 Defects Found in Audited Scope**: **NONE (0)**.
All confirmed P1 defects (F18, F21, F22, F25, F26, F27, F29, F30, F31, F32, F33, F34) are **VERIFIED** fixed with precise code evidence.
</answer>

<next_steps>
Ready to proceed - all scoped fixes in `web/` and `src/tracker/` are VERIFIED, with 0 reachable P0/P1 regressions. No follow-up needed.
</next_steps>
</results>

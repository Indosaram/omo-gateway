# Lane web-detail-create

I reviewed all 15 files in the diff (every file in both directories is new since e57b472). I re-opened each cited line in the current tree. The backend WIP in `omo_backend.rs` and `slack/egress.rs` doesn't touch these files. There was no working TypeScript install for diagnostics, and I ran nothing, so there is no type-check evidence.

### [P1] The embedded issue view's page hotkeys override the Inbox's j/k/e/Escape
- Location: web/src/tracker/detail/IssueView.tsx:237
- Trigger: Open Inbox, select a notification that has an `issue_id` (InboxPage.tsx:356 mounts `<IssuePage>`, which renders `IssueView` with `mode="page"`), then press `j`.
- Impact: The hotkey registry runs only the binding with the highest `order` (`[...bindings].sort((a, b) => b.order - a.order)`, then `return` after the first match). `IssueView` mounts after `InboxPage`, so its bindings always win.
  - `j`/`k` call `goTo`, which navigates to the next team issue and leaves the Inbox, instead of moving to the next notification.
  - `e` focuses the title instead of marking the notification read.
  - Escape runs `leave()` (`history.back()`) instead of `navigate('/inbox', { replace: true })`.
- Evidence:
  ```ts
  useHotkey('e', () => title.current?.focus(), hotkeyOpts)
  ...
  useHotkey(['j', 'arrowdown'], () => goTo(1), { enabled: ready && mode === 'page' && picker === null })
  useHotkey(['k', 'arrowup'], () => goTo(-1), { enabled: ready && mode === 'page' && picker === null })
  useHotkey('escape', leave, {
  ```
- Fix: Add an `embedded` prop (or a `mode: 'inbox'`) to `IssuePage`/`IssueView`, set it from InboxPage.tsx:356, and turn off the `j`/`k`/`e`/`escape` bindings when it is set. Alternatively, give these bindings a hotkey scope that the Inbox doesn't push.

### [P1] "Copy link to comment" builds a URL that opens "Issue not found"
- Location: web/src/tracker/detail/Activity.tsx:246
- Trigger: On an issue page (`#/tracker/issue/ABC-1`), hover a comment, click "Copy link to comment", then open the copied URL.
- Impact: The router is hash-based, so the result is `#/tracker/issue/ABC-1#comment-<id>`. `parseRoute` then sees the identifier as `ABC-1#comment-<id>`, and `trackerApi.issue` (with `encodeURIComponent`) returns 404, so the page shows "Issue not found". From peek mode, the link points at the list page instead. Nothing ever scrolls to `id="comment-…"`.
- Evidence:
  ```ts
  onClick={() => void copyText(`${window.location.href.split('#comment-')[0]}#comment-${comment.id}`, 'Comment link copied')}
  ```
- Fix: Build the link from the issue route as a query parameter, for example `` `${location.origin}${location.pathname}${hrefFor({ name: 'issue', identifier })}?comment=${comment.id}` `` (`parseRoute` already strips `?…`). Better still, copy the plain issue URL until there is scroll-to-comment handling.

### [P1] Description keystrokes typed in the last 800 ms are dropped when the peek is closed by clicking outside
- Location: web/src/tracker/detail/Editors.tsx:102
- Trigger: Open an issue in peek, edit the description, and within 800 ms press the mouse button on the area outside the panel (`ld-peek-overlay onMouseDown={close}`, IssueView.tsx:435).
- Impact: `close()` runs in the mousedown handler. React 18 flushes that update synchronously, so the editor unmounts before the browser moves focus. The wrapper's `onBlur` → `finish()` never runs, and the unmount cleanup only clears the pending autosave timer. The last edits are silently lost.
- Evidence:
  ```ts
  useEffect(() => () => window.clearTimeout(timer.current), [])
  ```
- Fix: Keep the latest value in a ref and flush it on unmount:
  ```ts
  useEffect(() => () => { window.clearTimeout(timer.current); if (latest.current !== lastSaved.current) save(latest.current) }, [])
  ```
  Update `latest.current = next` in `onChange`.

### [P2] The lane panel offers actions the backend always rejects
- Location: web/src/tracker/detail/AgentPanel.tsx:23
- Trigger: A lane is `queued` or `done_claimed` and the user clicks "Send message"; or the lane is `verified_done` and the user clicks "Object".
- Impact: The action always fails, and the panel shows an error.
  - `LaneCommand::Send` requires `working | blocked | not_done` (src/lanes/cli.rs:217).
  - Object calls `change_claim`, which targets `NotDone`, and `NotDone` is only allowed from `DoneClaimed` (src/lanes/state.rs:61).
- Evidence:
  ```ts
  queued: ['send', 'close'],
  ...
  done_claimed: ['object', 'send'],
  ...
  verified_done: ['close', 'object'],
  ```
- Fix: Remove `send` from `queued` and `done_claimed`, and remove `object` from `verified_done`.

### [P2] A failed description save is never retried, and the typed text is then discarded
- Location: web/src/tracker/detail/Editors.tsx:92
- Trigger: The PATCH for a description save fails (server or network error), and the user then blurs the editor.
- Impact:
  - `lastSaved.current` is set before the request and never reset on failure, so `finish()` → `save(value)` returns early.
  - `updateIssues` rolls the store back.
  - The `!editing` effect then resets `value` to the old description. The user's text disappears; the only notice is the error toast.
- Evidence:
  ```ts
  lastSaved.current = next
  setSaveState('saving')
  updateIssues([issueId], { description: next })
    .then(() => setSaveState('saved'))
    .catch((err) => {
      setSaveState('idle')
  ```
- Fix: In the `.catch`, restore `lastSaved.current` to its previous value (capture `const prev = lastSaved.current` before assigning).

### [P2] The create modal preselects the system (Sessions) team when the draft names it
- Location: web/src/tracker/create/CreateIssueModal.tsx:65
- Trigger: Open the Sessions system team's issue list and press `c` (GlobalHotkeys passes `team_id` from the route), then submit.
- Impact: The backend rejects the create with `system_team` conflict (src/tracker/issues.rs:316). The user gets an error toast after filling in the form, even though `selectableTeams` already excludes system teams.
- Evidence:
  ```ts
  const team_id = draft.team_id && t.teamsById.has(draft.team_id) ? draft.team_id : fallbackTeam
  ```
- Fix: Only accept `draft.team_id` when it is a non-system team: `draft.team_id && selectableTeams.some((tm) => tm.id === draft.team_id)`. This needs `selectableTeams` passed into `initialForm`.

### [P2] Home directory is hard-coded in the session panel's path shortening
- Location: web/src/tracker/detail/SessionAgentPanel.tsx:33
- Trigger: Any machine or user whose home directory isn't `/Users/indo`.
- Impact: Paths are shown in full instead of `~`-shortened. Cosmetic only.
- Evidence:
  ```ts
  return path.replace(/^\/Users\/indo(?=\/|$)/, '~')
  ```
- Fix: Use a generic pattern, `path.replace(/^\/(?:Users|home)\/[^/]+(?=\/|$)/, '~')`.
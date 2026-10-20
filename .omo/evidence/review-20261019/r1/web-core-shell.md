# Lane web-core-shell

I reviewed every hunk of the web-core-shell diff: App.tsx, TrackerApp, api, format, hotkeys, linear.css, router, shell/*, store and types. The CSS hunks were only skimmed: a grep found no global selectors outside `.lin`/`.ls-*` except `:where(.lin) button`. The Rust WIP in `src/agent/omo_backend.rs` and `src/slack/egress.rs` is not reached by any file in this lane. I made no edits and ran no builds or tests.

### [P1] Two-key "G then X" shortcuts on an issue page run the single-key issue action instead (pressing G I assigns the issue to you)
- Location: web/src/tracker/hotkeys.ts:88
- Trigger: Open `#/tracker/issue/ABC-1` (or peek an issue) and press `g` then `i`, which the shortcuts dialog and palette list as "Go to Inbox". Second keys `a`, `s`, `p` and `m` behave the same way.
- Impact: The dispatcher checks single-key bindings before it checks whether the key completes a two-key sequence. On the second key, IssueView's own `i` binding matches first and runs. So `g i` assigns the issue to the current user (a real write to the server) instead of going to Inbox. `g s` and `g p` open the status and priority pickers, `g a` opens the assignee picker, and `g m` focuses the comment box. The list view avoids this with its own G-prefix guard (`useGPrefix` in IssueCollection.tsx:403-404); IssueView has no such guard.
- Evidence:
  ```ts
  for (const binding of ordered) {
    ...
    if (matches(binding.combo, event)) {
      if (binding.opts.preventDefault !== false) event.preventDefault()
      binding.handler(event)
      buffer = []
      return
  ```
  and web/src/tracker/detail/IssueView.tsx:223-229:
  ```ts
  useHotkey(
    'i',
    () => {
      if (issue && t.me) t.updateIssues([issue.id], { assignee_id: t.me.id }).catch(notifyError)
    },
    hotkeyOpts,
  )
  ```
- Fix: In `onKeyDown`, before the bindings loop, check whether the key completes an enabled sequence given the buffer. If it does, run the sequence and return. That also makes the per-component `useGPrefix` workaround unnecessary. Alternatively, add `!gPending` to IssueView's `hotkeyOpts`.

### [P1] Sidebar Inbox badge counts snoozed notifications
- Location: web/src/tracker/store.tsx:380
- Trigger: Snooze an unread notification from the Inbox.
- Impact: The notification leaves the Inbox list, because `filterNotifications` hides snoozed items, but the sidebar badge still counts it. The badge stays too high until the snooze expires. The server's own `unread_count` (src/tracker/workspace.rs:235-236) excludes snoozed rows, so the client count disagrees with the backend. Snoozing does not mark the notification read, and `/notifications` still returns snoozed rows (it only filters out archived ones).
- Evidence: `unreadCount: notifications.filter((n) => !n.read_at && !n.archived_at).length,`
- Fix: Also exclude snoozed items: `&& !(n.snoozed_until && new Date(n.snoozed_until).getTime() > Date.now())`. Reusing `isSnoozed` from inbox/model would also work.

### [P2] Command palette acts on a stale issue focused on a previous page
- Location: web/src/tracker/shell/CommandPalette.tsx:69
- Trigger: Hover a row in a team issue list (`onRowHover` sets `focusedIssueId`). Then go to Projects via the sidebar and press Cmd+K.
- Impact: The palette shows that issue as context and offers Status, Assign, Archive and Delete for it, on a page where it is not visible. On route change the Shell resets the selection and the peeked issue but not the focused issue. The context chip shows which issue is targeted, which is why this is P2.
- Evidence: `if (t.focusedIssueId) return [t.focusedIssueId]`. In TrackerApp.tsx:138-142, the route-change effect clears only `setMobileNavOpen(false)`, `setPeekIssueId(null)` and `setSelection(new Set())`.
- Fix: Add `setFocusedIssueId(null)` to the route-change effect in TrackerApp.tsx:138.

### [P2] 10-second poll can briefly undo a fresh edit or bring back a deleted issue
- Location: web/src/tracker/store.tsx:222
- Trigger: Change an issue's status while a poll's `/issues` request is in flight that read the database before the PATCH committed.
- Impact: The stale list replaces the whole `issues` array after the optimistic update and the server response. The issue shows its old value, or a just-deleted issue reappears, until the next poll about 10s later.
- Evidence: `setData(boot)` / `setIssues(list.items)` inside `refresh`, which runs on `window.setInterval(..., 10_000)` (line 234).
- Fix: Keep a mutation counter or timestamp. Throw away a poll result if a mutation started after the poll's request was sent.

### [P2] Non-JSON error responses become a JSON SyntaxError message
- Location: web/src/tracker/api.ts:51
- Trigger: Any plain-text or HTML error response, for example the plain-text 403 from `validate_host_and_origin` (src/dashboard.rs:228), Axum's plain-text JSON-body rejection, or a proxy 502 page.
- Impact: `JSON.parse` throws before the `!response.ok` branch, so the toast or retry screen shows "Unexpected token ..." instead of the status and server text.
- Evidence: `const payload = text ? JSON.parse(text) : null`
- Fix: Wrap the parse in try/catch. If it fails, use `{ error: text }` for non-OK responses and rethrow for OK responses.

### [P2] Malformed percent-escape in the tracker URL blanks the app
- Location: web/src/tracker/router.ts:42
- Trigger: Open a hand-edited or truncated link such as `#/tracker/issue/%E0`.
- Impact: `decodeURIComponent` throws a URIError inside the `useState` initializer in `useRoute`. There is no error boundary (main.tsx renders `<App />` directly), so the whole UI unmounts. The app itself never produces such URLs, which is why this is P2.
- Evidence: `const parts = pathname.split('/').filter(Boolean).map(decodeURIComponent)`
- Fix: Map each segment through a safe decode: `(s) => { try { return decodeURIComponent(s) } catch { return s } }`.

### [P2] Tab title stays "... - Omon" after going back to the dashboard
- Location: web/src/tracker/TrackerApp.tsx:189
- Trigger: Open the tracker, then use "Back to dashboard".
- Impact: The tracker sets `document.title` but never restores it, and DashboardApp never sets one. The dashboard tab keeps the last tracker title.
- Evidence: `document.title = \`${title} - Omon\``
- Fix: Save the previous title when the Shell mounts and restore it in the effect's unmount cleanup.

### [P2] Due-date shortcuts and the "due this week" filter can be off by a day across DST
- Location: web/src/tracker/format.ts:91
- Trigger: In a timezone with daylight saving time, pick "In one week" (or apply the "week" due filter) when the range crosses a clock change.
- Impact: Adding multiples of 24h to local midnight lands at 23:00 the previous day, so `toDateOnly` returns a date one day early. This does not happen in KST, the current machine's zone.
- Evidence: `const add = (days: number) => toDateOnly(new Date(today.getTime() + days * DAY))`. The same pattern is in store.tsx:451, `weekAhead`.
- Fix: Use calendar math: `const d = new Date(today); d.setDate(d.getDate() + days)`.
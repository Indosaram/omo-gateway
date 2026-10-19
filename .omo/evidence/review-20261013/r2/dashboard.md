# dashboard review

Fixes I traced and found correct and complete:
- Origin check on unsafe HTTP methods and WebSocket upgrades.
- Detached-dashboard rejection of messenger chat, stop and cron trigger.
- Cron PUT partial-update semantics, including preserved scheduler state.
- Single-statement partial bot update.
- Create-bot conflict handling.
- Log id ordering and the snapshot/subscribe dedupe on the server.
- Readiness gating on the agent backend.
- Escaped LIKE search.
- The fail-closed web dispatcher.
- Expiry forwarding on the session socket.
- Latest-message-pages loading.
- The remote-image guard on both markdown paths.

### [P2] Duplicate log lines in the Live Logs page
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx:1448 (api.logs seed plus the `ws.onmessage` handler); server side is `handle_logs_socket` in /Users/indo/code/project/omon-gateway/src/dashboard.rs
- Trigger: Open the Logs page while the server holds at least one log entry. `api.logs()` seeds up to 250 entries. The WebSocket then replays the last 100 snapshot entries unconditionally. The client appends them with no id dedupe, and the rows are keyed by `log.id ?? idx`.
- Impact: The newest ≤100 lines appear twice, with duplicate React keys. If `api.logs()` resolves after the socket starts, it overwrites the live state instead.
- Fix: Dedupe by `id` on the client (ignore `entry.id <= lastId`). Alternatively, drop the server-side replay or the HTTP seed.

### [P2] HTTP fallback ends the turn on the 202, so the reply never shows
- Location: /Users/indo/code/project/omon-gateway/web/src/lib/chat-turn.ts (`socketError`); wired at /Users/indo/code/project/omon-gateway/web/src/App.tsx:608 and :619
- Trigger: The WebSocket fails before opening, for example when a proxy rewrites Host or Origin. `socketError` falls back to `api.postChat`. The server returns 202 `{queued:true}` immediately, so `finish({})` runs, `onDone` fires and `loadMessages` runs before the agent has replied.
- Impact: The assistant reply is not shown until the user clicks Reload.
- Fix: After a queued fallback, poll messages until the assistant message appears, or reopen the socket. The `postChat` type `{response:string}` also does not match the server body.

### [P2] Mid-turn status messages are invisible and unpersisted in the web playground
- Location: /Users/indo/code/project/omon-gateway/web/src/lib/chat-turn.ts (`classifyChatFrame`); producer is `relay_status` in /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:638
- Trigger: A web-session turn where the agent narrates before a tool call. The backend relays that text as `OutboundAction::SendMessage`. The dashboard forwards it as `{type:"event",event:{type:"send_message",content}}`, and the classifier returns `ignore`.
- Impact: The text is never shown. It is not persisted either (see the `relay_status` doc comment), so the final reply is the only visible output. The header comment in chat-turn.ts still says mid-turn messages arrive as `is_final` stream chunks, which is stale.
- Fix: Classify `event.type === 'send_message'` as a chunk or append it as its own message, and update the comment.

### [P2] Stopping a turn and sending again can end the new turn early
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx (`handleStopTurn`, about line 627); stop path at /Users/indo/code/project/omon-gateway/src/multiplexer/router.rs:476 and `release_typing` in /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:677
- Trigger: Click Stop. `setIsSending(false)` re-enables input at once, but the stopped turn's `Typing{active:false}` is emitted at the start of `stop()` and again by `release_typing` after the cancel and rollback finish. The user sends a new message before the second one arrives, and the new socket is already subscribed.
- Impact: The stale `typing=false` classifies as `end`. The new turn is closed, shown as done and reloaded while its agent turn is still running. The reply appears only after a manual reload. The same happens across two tabs on one session.
- Fix: Keep `isSending` true until the stop socket reports `stopped` and the old turn socket closes, or tag typing events with a turn id.

### [P2] Create-bot stores empty strings that the actor treats as overrides
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:2128 (create_bot); consumer at /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1280-1288
- Trigger: `POST /api/bots` through the API with `"model":""`, `"system_prompt":""` or `"enabled_toolsets":[]`. The BotsPage UI sends null, so it does not hit this.
- Impact: The row stores `""`, unlike update, which clears to NULL. The actor then sees `Some("")`, so it sets an empty `active_model` and an empty system prompt. `[]` joins to `""`, which becomes an empty toolset list (no tools) rather than "all defaults".
- Fix: Apply the same empty-to-NULL normalization as `update_bot` in `create_bot`.

### [P2] Create-cron without an id and disabled is briefly enabled, and a missing payload becomes null
- Location: /Users/indo/code/project/omon-gateway/src/dashboard.rs:1664 (pause after `register`); `CronJobInput.payload` has `#[serde(default)]`
- Trigger: `POST /api/cron/jobs {"expression":"...","enabled":false}` with no id. `register` inserts the job enabled with a `next_run_at`. A separate `pause` runs afterwards. If the expression is due almost immediately (for example a `once:` time about now), the scheduler can claim it in between. The id path was fixed with `register_new(..., enabled)`.
- Impact: A job created disabled can run once. Separately, an omitted payload is stored as JSON `null`, which creates a job that fails at run time instead of failing the request with a 400.
- Fix: Add an `enabled` parameter to `register`, or reuse `register_new` with a generated id. Reject a null payload.

### [P2] The Overview and Approvals pages read fields the API never returns
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx:321 (`status.model`), :1373 (`a.session_id`)
- Trigger: Open Overview or Security. `api_status` has no `model` key, and `PendingApprovalView` serializes `session` and not `session_id`.
- Impact: The Active Model card and the approval session label render blank. When the disk sample is None, Overview computes `(1-0)/1` and shows 100% used in red.
- Fix: Add `model` to `/api/status` (from `config["model"]`), render `a.session` fields, and show "unknown" when the disk sample is null.

### [P2] `isAutoLoadableImageSrc` auto-loads `/\host/x.png` (latent)
- Location: /Users/indo/code/project/omon-gateway/web/src/lib/remote-image.ts
- Trigger: A src starting with `/\` or `\`. The function treats it as a scheme-less relative path and returns true, and browsers resolve `/\evil.com/x.png` as protocol-relative.
- Impact: It is not reachable today. The markdown path percent-encodes backslashes in `normalizeUri`, and the regex path only matches `https?://` URLs. Any future caller passing a raw src would leak the viewer's IP.
- Fix: Return false when the value contains a backslash, tab or newline, or resolve it with `new URL(value, origin)` and compare origins for every input.

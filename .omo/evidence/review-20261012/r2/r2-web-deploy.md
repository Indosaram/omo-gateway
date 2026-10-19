# r2-web-deploy review

### [P1] Chat turn ends on the first `is_final` stream chunk, but the backend emits several final chunks per turn
- Location: /Users/indo/code/project/omon-gateway/web/src/lib/chat-turn.ts:52
- Trigger: A web-chat turn where the agent writes preface text and then calls a tool, for example "Let me check..." followed by a terminal call.
  - The backend sends that preface through `deliver_rendered_result`, which emits `Stream{is_final:true}` (/Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:649-657). It is called mid-turn at omo_backend.rs:1464 and 1534.
  - Each such chunk ends the turn in the UI.
  - The real end-of-turn signal is the actor's `Typing{active:false}` (/Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:870-880).
- Impact:
  - On the first preface message the UI closes the socket, clears the streaming bubble and re-enables the input.
  - The agent keeps running, and its later messages, tool results and final answer are never shown live.
  - Nothing polls for them, so they appear only after the user clicks Reload or sends another message.
  - The user can also send a second prompt that queues behind the still-running turn.
- Fix: Stop treating `ev.chunk.is_final` as the end of the turn. Treat it as a replace-style chunk and end only on `typing active:false`, `stopped`, `error` or socket close. In `classifyChatFrame`, replace line 52 with `return { kind: 'chunk', content: String(ev.chunk.content ?? ''), replace: true }`. Also drop `ev.is_final` from line 57, since `ev` is not a stream chunk there.

### [P2] HTTP fallback "completes" on the 202 queue ack, so the reply is never shown
- Location: /Users/indo/code/project/omon-gateway/web/src/lib/chat-turn.ts:125-136 (fallback wired at /Users/indo/code/project/omon-gateway/web/src/App.tsx:573)
- Trigger: The WebSocket handshake fails, for example a proxy blocks upgrades. `socketError` then calls `api.postChat`.
  - The backend returns 202 `{queued:true}` immediately (/Users/indo/code/project/omon-gateway/src/dashboard.rs:1262-1269) and the turn runs asynchronously.
  - `finish({})` fires at once, so `loadMessages` runs before the assistant reply exists.
- Impact: The user sees only their own message, `isSending` goes false, and the reply appears only after a manual Reload.
- Fix: After a successful fallback POST, either refetch with a bounded retry until a new assistant message appears (not a fixed sleep), or surface "queued, press Reload". At minimum the comment at App.tsx:583 should stop claiming the reply is awaited.

### [P2] Leaving the Playground mid-turn leaves the socket open and can raise an alert on another page
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx:520
- Trigger: Send a message, then click another nav item such as Overview before the turn ends.
  - `ChatPlaygroundPage` unmounts, but nothing on unmount calls `turnRef.current?.abandon()`. The effect at lines 520-533 only cleans up on a session change.
  - The turn stays "current" because `currentSessionRef` and `turnRef` are unchanged.
  - On the terminal frame, `onDone` runs after unmount and calls `alert('Chat error: ...')` if the turn failed.
- Impact: The WebSocket stays open until the turn ends, and a modal alert can pop up over an unrelated page.
- Fix: Add an unmount cleanup, `useEffect(() => () => { turnRef.current?.abandon(); turnRef.current = null }, [])`.

### [P2] Switching sessions keeps the previous session's transcript on screen until the new load resolves
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx:520-533
- Trigger: Click another conversation on a slow link or with a large session.
  - The effect resets `streamingContent` and `isSending` but not `messages`.
  - The header shows the new session id while the old transcript stays visible, and the input is enabled.
  - A message sent in that window is appended optimistically to the old session's list.
- Impact: The user sees the wrong history under the new id and a user bubble in the wrong thread. It is cosmetic and self-corrects once `loadMessages` resolves.
- Fix: Call `setMessages([])` in that effect before `loadMessages(currentSessionId)`.

### [P2] Docker image ships no `web/dist`, so the dashboard falls back to the placeholder page
- Location: /Users/indo/code/project/omon-gateway/Dockerfile:26
- Trigger: Run the container with `DASHBOARD_ENABLED=1` or `DASHBOARD_PORT` set.
  - The default web root is `web/dist`, relative to `WORKDIR /app` (/Users/indo/code/project/omon-gateway/src/dashboard.rs:114-118).
  - The image copies only the binary, so `/app/web/dist` does not exist.
  - The server then serves the "web/dist was not found" HTML (dashboard.rs:2392).
- Impact: The React UI, including every file changed in this round, is not reachable from the Docker deployment.
  - The dashboard also binds loopback only. `validate()` rejects non-loopback hosts (dashboard.rs:74-83), and compose publishes no ports, so it cannot be reached from the host either.
  - That second restriction is by design, but together the two mean the container has no usable web UI.
- Fix: Add a Node build stage that runs `npm ci && npm run build` in `web/`. Then add `COPY --from=web /web/dist /app/web/dist` to the runtime stage and `DASHBOARD_WEB_ROOT=/app/web/dist` to its `ENV`. Alternatively, document that the container is gateway-only.

Checked and found correct:
- **Bot editor clear contract:** `buildBotUpdatePayload` and `update_bot` (dashboard.rs:2024-2086) agree. Empty string and `[]` clear to NULL, an omitted field keeps the stored value, and an empty name returns 400.
- **Newest-page loading:** `latestMessagePages` matches the `ORDER BY sequence ASC`, `per_page` and `total` behaviour of `list_messages` (dashboard.rs:1152-1198). The 50-per-page default is within the 200 cap.
- **Socket and session handling:**
  - A fallback in flight is not cut short by `onclose`.
  - Abandon and session-switch both close the socket once, and a stale session's frames are dropped.
  - The `onerror` then `onclose` ordering is handled by the `fallingBack` guard.
- **Docker and Cargo:**
  - The Dockerfile copies `tests/`, and all 15 `[[test]]` paths exist.
  - Compose's named volumes inherit the `omon:omon` ownership set by the image's `chown`.
  - The `rust-version = "1.88"` matches the 1.92 builder and is consistent with `time 0.3.55` and `home 0.5.12` in `Cargo.lock`.

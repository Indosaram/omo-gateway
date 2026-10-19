# fix-web — web dashboard P1 fixes

Scope: `web/src/**` only. No Rust, dependency, .env, DB or gateway changes. No commit.

## Per-defect status

| # | Defect | Status |
|---|--------|--------|
| 1 | Chat turn never completes on non-`final` terminal shapes; socket leak; `setTimeout` fallback | FIXED |
| 2 | `loadMessages` only loads page 1 (oldest 50) | FIXED |
| 3 | Switching conversations mid-turn writes into the newly opened session | FIXED |
| 4 | Bot edit: clearing Model / Persona / Toolsets is ignored | Persona: FIXED. Model and Toolsets: BLOCKED-SCOPE (needs a backend change, see below) |

### 1. Turn completion — FIXED
- New `web/src/lib/chat-turn.ts`:
  - `classifyChatFrame(raw)` maps every backend frame to `chunk | end | error | ignore`. Backend shapes were checked in `src/dashboard.rs` `handle_session_socket` and `src/models/events.rs` `OutboundAction` (`#[serde(tag="type")]`). Terminal shapes are: `{type:"error"}`, a bare `{error}`, `{type:"event",event:{type:"error"}}`, `final`, `stopped`, `{type:"accepted",direct_reply:true}` (direct reply; the backend sends no stream after it), a stream chunk with `is_final`, and `{type:"event",event:{type:"typing",active:false}}`. The last one is emitted by `actor.rs` `release_typing` at the end of every turn, including silent turns.
  - `createChatTurn(hooks)` completes the turn exactly once and always calls `close()`. `ws.onclose` with no terminal frame also completes the turn. `socketError()` before the socket opens falls back to `api.postChat` and completes only after that POST resolves or rejects; the `setTimeout(1500)` is gone. A socket error after the message was sent completes the turn with an error and does not post a duplicate.
- `web/src/App.tsx` `ChatPlaygroundPage.handleSendMessage` now wires `onopen`, `onmessage`, `onclose` and `onerror` to the turn object. The old inline JSON branch logic was removed.

### 2. Latest messages — FIXED
- The backend `list_messages` orders by `sequence ASC` with `page`/`per_page` (default 50, max 200) and returns `total`.
- `web/src/api.ts` `sessionMessages(id, page = 1, perPage = 50)` now sends `?page=&per_page=`.
- `latestMessagePages(total, perPage)` in `chat-turn.ts` returns the last page plus the one before it when the last page is partial. That keeps at least `perPage` of the newest messages in chronological order.
- `App.tsx` `loadMessages` fetches page 1 to learn `total`, then fetches those latest pages. It only calls `setMessages` if the session id is still current. The redundant duplicate `web-*` branch was collapsed because both branches were identical.

### 3. Session binding — FIXED
- `App.tsx`: `currentSessionRef` tracks the open session and `turnRef` holds the active turn. Each turn captures `turnSessionId`. `isCurrent()` is true only while that session is open and the turn is still the active one. A frame that arrives for a session that is no longer current closes the socket and is dropped, so no chunk or completion is applied.
- The `currentSessionId` effect calls `turnRef.current?.abandon()`, which closes the socket without completing. It then resets `isSending` and `streamingContent` before loading the new session's messages.

### 4. Bot clear — Persona FIXED, Model/Toolsets BLOCKED-SCOPE
- New `web/src/lib/bot-payload.ts` `buildBotUpdatePayload(form)` is used by `BotsPage.tsx` `handleSave`.
- **Persona (`system_prompt`)**: an empty prompt is now sent as `""`. Backend `update_bot` (`src/dashboard.rs:2048`) runs `Some("").or(existing)` and stores `""`. Downstream, `actor.rs:1079/1137` copies `Some("")` and `omo_protocol::thread_start_request` sends `developerInstructions: ""`, which is an empty persona. The clear now works.
- **Model**: still sent as `null`, which the backend keeps (`.or(existing.model)`). Sending `""` would be wrong. `actor.rs:1093/1151` would set `active_model = Some("")`, and `omo_backend.rs:352-356/716-720/1845-1849` uses `active_model.as_deref().or(default_model)`. `Some("")` therefore suppresses the gateway default and sends `model: ""` to omo. The frontend cannot express a clear without that regression.
- **Toolsets**: still sent as `null`. Sending `[]` would store `""`, and `actor.rs:1082-1083/1140-1141` would then produce `enabled_toolsets = Some([])`. That means "no tools": `omo_backend.rs:1284-1298` denies every commandExecution and `enabledToolsets: []` is sent. It does not mean "All default toolsets" as the UI says (`BotsPage.tsx:210`).
- **Required backend change (BLOCKED-SCOPE)**, in `src/dashboard.rs` `update_bot` (around lines 2047-2052). Distinguish an explicit empty value from an omitted field and store SQL NULL for it:
  ```rust
  let model = match payload.model { Some(m) if m.trim().is_empty() => None, Some(m) => Some(m), None => existing.model };
  let system_prompt = match payload.system_prompt { Some(p) if p.trim().is_empty() => None, Some(p) => Some(p), None => existing.system_prompt };
  let toolsets_str = match payload.enabled_toolsets { Some(ts) if ts.is_empty() => None, Some(ts) => Some(ts.join(",")), None => existing.enabled_toolsets };
  ```
  Once that lands, change `buildBotUpdatePayload` to send `model: ""` and `enabled_toolsets: []` when the fields are empty. The existing `bot_update_with_only_name_preserves_other_fields` test (omitted field means keep) stays valid.

## Files / functions changed
- `web/src/lib/chat-turn.ts` (new): `classifyChatFrame`, `createChatTurn`, `latestMessagePages`
- `web/src/lib/bot-payload.ts` (new): `buildBotUpdatePayload`
- `web/src/App.tsx`: import, `ChatPlaygroundPage` (`currentSessionRef`, `turnRef`, `loadMessages`, session-switch effect, `handleSendMessage`)
- `web/src/api.ts`: `sessionMessages(id, page, perPage)`
- `web/src/pages/BotsPage.tsx`: import, `handleSave`
- `web/src/__tests__/dashboard-fixes.test.ts` (new)

## Tests (`web/src/__tests__/dashboard-fixes.test.ts`, bun test, no new dependency)
- chat turn completion > direct-reply ack ends the turn
- chat turn completion > plain accepted ack does not end the turn
- chat turn completion > typing release (silent turn) ends the turn
- chat turn completion > error frame without message field still ends the turn with an error
- chat turn completion > final stream chunk ends the turn
- chat turn completion > socket close without a terminal frame completes and closes once
- chat turn completion > terminal frame closes the socket and completes exactly once
- chat turn completion > HTTP fallback completes the turn only after the POST resolves
- chat turn session binding > events for a session that is no longer open are ignored and the socket is closed
- chat turn session binding > abandoning a turn closes it without completing the new session
- latest message pages > short session loads the single page
- latest message pages > long session loads the newest pages, not page 1
- bot update payload > clearing the persona sends an explicit empty string, not null

## RED
The original logic was inline in React components and had no helper to call. For RED, the test file was copied unchanged to `/tmp/fix-web-red/__tests__/`. Its `lib/chat-turn.ts` and `lib/bot-payload.ts` there are transcriptions of the **pre-fix** behaviour from the original `App.tsx` and `BotsPage.tsx`: the same onmessage branches, no `onclose`, no session binding, `setTimeout` after the POST, page 1 only, and `|| null` for the prompt.

Command: `cd /tmp/fix-web-red && bun test` — exit=1. Full log: `fix-web-red.log`.
```
(fail) chat turn completion > direct-reply ack ends the turn            Expected kind "end", Received "ignore"
(fail) chat turn completion > typing release (silent turn) ends the turn Expected "end", Received "ignore"
(fail) chat turn completion > error frame without message field ...      Expected error "boom", Received "ignore"
(fail) chat turn completion > socket close without a terminal frame ...  Expected [{}], Received []
(fail) chat turn completion > HTTP fallback completes ... after the POST resolves   Expected [{}], Received []
(fail) chat turn session binding > events for a session that is no longer open ...  Expected [], Received ["leak"]
(fail) chat turn session binding > abandoning a turn closes it ...       Expected closes 1, Received 0
(fail) latest message pages > long session loads the newest pages, not page 1  Expected [2,3], Received [1]
(fail) bot update payload > clearing the persona sends an explicit empty string, not null  Expected "", Received null
 4 pass
 9 fail
```

## GREEN
Command: `cd web && bun test src/__tests__` — exit=0. Full log: `fix-web-green.log`.
```
 13 pass
 0 fail
 24 expect() calls
Ran 13 tests across 1 file.
```

## Build
Command: `cd web && npx tsc -b && npx vite build` — **exit=0**. Full log: `fix-web-build.log`.
```
vite v6.4.3 building for production...
✓ 1983 modules transformed.
dist/assets/index-BehPAAmy.js   361.95 kB │ gzip: 108.32 kB
✓ built in 39.61s
```
Note: the build rewrote `web/dist/` as its normal output.

## Not verified
- No browser or live-gateway exercise of the chat page. Per the task rules, no dev servers were started.
- The `typing active:false` end signal relies on the actor emitting it after the reply is persisted. The turn then reloads messages from the DB. If the reply row commits after the typing release, that reload could miss it until the next load.

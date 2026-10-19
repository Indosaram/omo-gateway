# gateway-autonomy-lanes - Work Plan

## TL;DR
Today two OmOMeow brains answer Discord at once: the Rust gateway and the legacy herdr session (pane w8:p1). The legacy one is where the "Dori-like" autonomy lives (herdr work tabs, work-map.json, watch.ts nudges); the gateway only answers when spoken to. Plan: stop the second brain first, then move the delegation loop (lanes) into the gateway, reusing the `work_items` table and `WorkHeartbeatRunner` it already has but never feeds.

## Verified facts (2026-10-08)
- `ai.omomeow.herdr` launchd job is **enabled and running** (started 12:27:38 KST-UTC log), with `herdr server`, `listen.ts` loop and the OmOMeow pi session in herdr pane `w8:p1` (status `working`). `inbound.log` is still receiving owner messages (12:55, 12:56 UTC).
- The gateway (`ai.omon.gateway`) receives the same messages on the same bot token. Both sides can post as OmOMeow, each with its own conversation history.
- Owner complaints at 12:55-12:56 UTC ("왜 갑자기 이걸 보냄?", "이런걸 지시한적이없는데 뭔 세션임") followed an OmOMeow post at 12:53:48 that arrived while the gateway was restarting (SIGTERM 12:53:11). Most likely sender: the legacy session. Not proven from logs alone.
- Gateway has a lane scaffold that is dead code: table `work_items(id, title, thread_id, delegate_pane, status, last_progress_at, receipt_path)` and `WorkHeartbeatRunner` (60s tick, 15m threshold, checks pane liveness via `herdr agent list`, posts to the thread). `insert_work_item` has **no callers**, the table is empty.
- Gateway turns are request/response only: a turn starts on an inbound message or a cron, ends at `turn/completed` or the 1800s deadline. Nothing starts a turn on a lane event.
- Legacy rules that define the wanted behavior: `~/.omon/omomeow/OMOMEOW-MODE.md` items 11, 14, 17-21 (thread per job, herdr tab per job with a written brief, progress every 15 min, watch for blocked/done sessions, review and close).

## Verdict on the earlier analysis
- Right: the gateway alone is reactive; it has no lane spawning, no lane-event-triggered turns, no done verification.
- Wrong: "we have no autonomy at all". The autonomy exists, in the legacy herdr session, and it is still live. The real current defect is the split brain, which also explains part of the "context isn't kept" symptom (two sessions, two histories, one bot).
- Wrong emphasis: we do not need a new design. The migration plan already chose "hybrid: keep herdr delegation"; the gateway half of that was scaffolded but never wired.

## Steps

### 0. One brain (needs owner OK - stops a running service)
- `launchctl bootout gui/501/ai.omomeow.herdr` and `launchctl disable gui/501/ai.omomeow.herdr`; stop `listen.ts` loop. Keep `herdr server` itself only if step 2 launches lanes through it (start it from the gateway or a plain launchd job without the OmOMeow session resume in `herdr-boot.sh`).
- Find what re-enabled the job (it was disabled earlier today) - likely the legacy session itself or `herdr-boot.sh` respawn - and remove that path.
- Done when: `launchctl print-disabled gui/501` shows `ai.omomeow.herdr => disabled`, `pgrep -f listen.ts` is empty, no new lines in `inbound.log` after an owner test message, and only gateway-logged OmOMeow posts appear.

### 1. Lane launch tool (gateway)
- Add a gateway-side tool the OMO session can call: `lane_start { key, title, cwd, brief, done_signals[] }`.
- It creates the Discord thread (or reuses the current one), opens a herdr tab in `cwd` running `omo --model <pinned model> "<brief>"` (argv, never a shell string), and inserts a `work_items` row (`delegate_pane`, `thread_id`, `status=in_progress`).
- Reply in-thread within seconds: one line with the lane key and the plan.
- Done when: a real owner request produces a thread, a live herdr pane, a `work_items` row, and an ack message, with the chat session free to take the next message.

### 2. Lane events wake the lead (gateway)
- Extend `WorkHeartbeatRunner.step`: for each `in_progress` item read `herdr agent list` status; on `done`, `blocked`/`waiting`, or no progress past threshold, start a **gateway-initiated turn** in the item's thread session with a synthetic event (`[LANE_EVENT] <key> done|blocked|stale` plus the last 40 lines of the pane).
- The lead decides: answer the lane via `herdr pane send-text`, ask the owner, or close.
- Keep the existing 15-minute two-line progress post (OMOMEOW-MODE 11).
- Done when: killing or finishing a test lane produces a thread message without any owner input.

### 3. Done verification (gateway, from dori's done protocol)
- `done_signals` kinds: `command` (argv + expected exit 0), `merged`/`closed` via `gh`, `file` exists/hash, `url` 200.
- On a lane's done claim, the gateway runs every signal itself; only all-green moves the item to `done`, posts evidence, archives the thread and closes the tab. Any red posts the failing signal and keeps the lane open.
- Done when: a lane that claims done with a failing test is kept open and the failure is posted.

### 4. Cutover + rollback
- Order: 0 -> 1 -> 2 -> 3, each shipped and owner-tested before the next.
- Rollback for 1-3: feature flag `OMON_LANES_ENABLED` (default on once shipped); off returns to today's reactive gateway. Rollback for 0: `launchctl enable` + `bootstrap` the plist (unchanged on disk).

## Out of scope
- Rewriting dori-mode's TypeScript CLI or adopting its 15/20-minute silence defaults.
- Changing `~/.omo/agent/omo.json` or `~/.omo/omo.json`.
- Model routing changes (stay on `mahoquot/gemini-3.8-flash-high`).

## Verification per step
- `cargo test` (backend + discord adapter suites) green, release build, Developer ID codesign, `launchctl kickstart`, then one real owner message per step checked in Discord and in `messages`/`work_items` rows.

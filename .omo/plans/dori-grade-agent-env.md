# dori-grade-agent-env - Work Plan

## TL;DR (For humans)

**Who this is for and what changes for them:** You (the owner) keep talking to one OmOMeow bot on Discord. Today it only answers when spoken to, and two copies of it are running at once. After this plan, one copy runs; it hands long jobs to separate worker sessions, follows each to the end on its own, tells you only what matters, and closes a job only after the result is checked by machine.

**What you'll get:** the old duplicate bot retired for good; a job system where every long task gets its own Discord thread and worker session; automatic wake-ups for the bot when a worker finishes, gets stuck, asks a question, dies or goes quiet; a "done" check that re-runs the stated proof (merged PR, passing command, file, URL) and reopens the job if it fails; a periodic "what would the owner want next" check; protection against overloading the machine; a consent-first onboarding routine; an inbox triage that reads your mail, KakaoTalk, Discord and iMessage/SMS and sorts each item into "you must do this", "the bot can do this" or "just FYI", with a list you can ask for any time and a morning digest; a session console that shows every running OMO session on your Mac and your other machines, and lets the bot read, message, stop or take over and finish any of them; a cron watchdog that checks each scheduled job's result the moment it finishes (no periodic polling), notices failed, missed, stuck or silently-empty runs, investigates, fixes what it can, re-runs once and proves the re-run worked, and tells you only the result or what needs your hand; and updated bot rules that teach the bot all of this.

**How it saves tokens:** a fast typed judge (jev) answers every small question - which pile does this message go in, does this event need the main bot, how big is this job, is this failure a fluke, did the work really get done - in under a second without a chat model; a cheap model writes short things and does small jobs; the expensive model only talks with you and does real engineering. You can see how much each tier used every morning.

**Why this approach:** everything is built inside the existing gateway program instead of adding a second always-on script, because a second daemon is exactly what caused the duplicate-bot problem. Worker sessions are ordinary agent threads on the same OMO engine the bot already talks to, so their progress arrives as structured events - no terminal manager, no screen scraping, nothing new to install.

**What it will NOT do:** no model changes; no Slack, Huddle/Meet or GitHub-notification integrations; it never sends, replies or marks anything read in your mail, KakaoTalk, iMessage or Discord without your OK; no sending as your personal Discord account; no multi-device memory sync.

**Effort:** XL
**Risk:** High - long-running background control of real sessions, Discord threads and git worktrees; mitigated by kill switches, read-back checks and no-shell argv everywhere.
**Decisions I made for you:** (1) build in the gateway (Rust) and port the public dori-mode rules, not its code; (2) workers are separate OMO app-server threads (own folder, own session), driven by the gateway; herdr is not used anywhere; (3) the bot controls jobs through a new gateway subcommand it calls from its shell; (4) worker events reach the bot as system messages, batched within 30 s, only for done/blocked/question/dead/quiet; (5) 5-minute objection window, then automatic close if every check passes; (6) the "what would the owner want" check runs only when something new happened (new inbox items, a lane or cron event, a quiet owner since the last check) - never on a fixed timer - max 6 wake-ups a day, on the current default model; (7) machine limits from dori defaults (20% free memory, 50 GB disk, 20 panes); (8) the old herdr-based OmOMeow launchd job and its herdr server are removed with a backup; (9) kill switches OMON_LANES_ENABLED / OMON_KIBITZER_ENABLED, on by default; (10) models: Opus 5.5 (`mahoquot/claude-opus-5-5`) for the main bot and big work, DeepSeek v4.1 flash (`inferhub/cb/deepseek-v4.1-flash`) for small work, jev for every decision.

Your next move: run it with `/ulw-execute dori-grade-agent-env` (the one owner-only step is marked OWNER in the first todo). Full execution detail follows below.

---

> TL;DR (machine): XL, High risk; one-brain cutover + Rust lane engine on OMO app-server threads (registry, runner, event-wake, done protocol), kibitzer cron, host guard/heavy slots, thread lifecycle, persona rules, onboarding, multi-source inbox triage (mail/KakaoTalk/Discord/iMessage, read-only), cross-machine OMO session console via `omo thread` over ssh, event-driven cron health hook (on run completion, no polling) with remediation lanes; jev decision layer + Opus/deepseek tiering; no herdr; 37 todos + F1-F4.

## Scope
### Affected user and ideal state
**Affected user:** (1) the owner, Indo, talking to OmOMeow in Discord DM, channels and auto-created threads; (2) the lead OMO session that answers him (consumes lane/watch events); (3) lane OMO threads run by the gateway on the app-server (consume briefs and the done rule); (4) other people in shared Discord channels where the bot sits.
Today: an owner message runs one request/response turn (cap 1800 s, `src/agent/omo_backend.rs:76`); long work blocks or is lost; a legacy herdr OmOMeow also answers on the same bot token. After: one brain; long work becomes a lane with its own thread; the lead is woken by lane state; done is machine-checked.

| Row | Statement | Reason |
| --- | --- | --- |
| IS-1 | Exactly one process answers as OmOMeow, and nothing re-enables the legacy one | two brains post contradicting messages (memory split-brain note; regression seen 2026-10-10) |
| IS-2 | Every owner message gets an answer or a one-line plan within 60 s in its thread, also while lanes run | OMOMEOW-MODE rule 11; owner instruction "선제 보고" |
| IS-3 | Long work runs in a lane (own OMO app-server thread and folder, own Discord thread, registry row); the chat stays free | dori-mode SKILL routing; 1800 s cap |
| IS-4 | Lane state changes (done-claimed, blocked/question, dead, quiet) wake the lead without owner input | 비가와요 10-09 23:58 "상태 변화 ... 도리류가 일어나서" |
| IS-5 | Done = every declared signal read back live by the gateway; failing claims return to the lane with reasons; unobjected valid claims close after 5 min | dori done-protocol.md; owner rule "기계 판독" |
| IS-6 | The lead is woken only by events worth a turn, coalesced, never by routine heartbeats | 노고노고 10-09 23:32 "깰때마다 캐시먹는다 ... 500만" |
| IS-7 | An event-armed kibitzer (runs only after something new happened) infers what the owner would want next from memory, lanes and recent messages and wakes the lead only when action is warranted | 공식냥 09-28 11:09-11:10 |
| IS-8 | No lane starts when memory/disk/concurrent-lane limits are crossed (queued with a reason); heavy commands are serialized | dori guard / can-launch / heavy |
| IS-9 | A closed lane can be resumed in its thread with its recorded agent session | OMOMEOW-MODE 19; dori sessions.md |
| IS-10 | Lane threads show status (🔄/⏸/✅ prefix), are archived on close and reopen on owner write; gateway-posted lane text never leaks home paths, pane ids or secrets | OMOMEOW-MODE 14; dori freshness scrub |
| IS-11 | Onboarding asks consent first, goes tool by tool, read-only, writes memory, reports gaps | dori onboarding.md |
| IS-12 | Lanes, claims, deadlines and undelivered events survive gateway restarts | dori watch deadline in registry |
| IS-13 | New items from mail, KakaoTalk, Discord and iMessage/SMS are picked up as they arrive (watchers; no model call unless something is new), pass free rule filters first (promotions, open-chat noise, short-code SMS dropped; measured ~20-40 items/day remain on 2026-10-10 data), read (never sent, never marked read), and each is classified as "owner must act", "bot can handle", or "FYI/ignore", with a reason and a source link; the owner can ask "내 할일 정리해줘" and gets the current list at once, plus one daily digest | owner request 2026-10-10 (1) |
| IS-14 | "bot can handle" items become a proposed lane or a direct bot action that the owner approves with one reply; nothing is sent to other people without the owner's OK | owner request (1); OMOMEOW-MODE rule 16; standing rule "외부 발송은 승인" |
| IS-15 | Asking "세션들 상태" lists every running OMO session on every reachable machine (machine, folder, name, live/idle/busy, last activity) within 30 s; the bot can read one, send it a message, interrupt it, or take over finishing it | owner request 2026-10-10 (2) |
| IS-16 | A session the owner hands over is driven to completion with the same done rule as lanes (signals or an explicit owner-confirmed finish), and the owner hears only the result | owner request (2) "스스로 관리해서 마무리" |
| IS-17 | Every enabled cron job's health is known the moment a run finishes: failed runs, runs stuck "running" past their lease, "succeeded" runs whose destination delivery failed or whose machine-readable output says it did nothing, and fires that never happened (seen at catch-up/start-up) - with no periodic polling and no model call spent on healthy runs | owner request 2026-10-10 "크론 성공/실패 여부도 모니터"; memory rule "DB 상태만 믿지 말고 실제 목적지 수신 여부를 검증" and "기계 판독 결과로 판정" |
| IS-19 | Every routing or yes/no decision in the system is made by jev (`typesafe-judge`, ~0.5-1 s, measured 12/12 + 8/8 correct on 2026-10-10 samples) instead of an LLM turn; the expensive model (Opus) is called only for owner conversation and large real work, the cheap model (deepseek-v4.1-flash) for short writing and small lanes; daily token use per tier is visible to the owner | owner request 2026-10-10 "jev를 전체 아키텍처에서 사용해서 효율화", models fixed by owner |
| IS-18 | A detected cron problem becomes a remediation lane automatically (diagnose from logs and the run's error, fix the script/config or open a repo lane, re-run the job once, verify the re-run with machine evidence) and the owner gets one message with cause, fix and evidence; problems that need him (login, payment, quota top-up) are put on his task list instead | owner request "추가 작업 필요하면 그것까지 맡아서"; memory rules "크론 실패는 자동 재시도되지 않음", "쿼터 소진 보고 전 mahoquot 직접 검증" |
| GAP-1 | `ai.omomeow.herdr` is enabled and running (pid 66062), listen.ts loop (67652/67659) and herdr pane w8:p1 live; herdr-boot.sh restarts listener + session. The owner does not use herdr (2026-10-10); herdr on this machine exists only for the legacy bot | launchctl print-disabled + pgrep, 2026-10-10 |
| GAP-2 | No lane launch path: `insert_work_item` has no Rust callers (`src/storage/db.rs:998`); `bin/omomeow-delegate` only inserts rows and opens a herdr tab (`bin/omomeow-delegate:104-109`); `WorkHeartbeatRunner` checks liveness through `herdr agent list` (`src/work_heartbeat.rs:123-168`) | code read |
| GAP-3 | Turns start only from inbound messages, cron or restart recovery (`src/multiplexer/router.rs:312`, `src/main.rs:455`) | code read |
| GAP-4 | No done protocol or status machine (`migrations/0030_work_items.sql` has a status text only) | code read |
| GAP-5 | `WorkHeartbeatRunner` posts every stale item straight to Discord, never wakes the lead (`src/work_heartbeat.rs:317-380`) | code read |
| GAP-6 | No kibitzer | code read |
| GAP-7 | No host guard / heavy slots | code read |
| GAP-8 | No status naming/archive/reopen for lane threads (only archived-flag tracking, `src/discord/adapter.rs:1254-1282`) | code read |
| GAP-9 | No outbound scrub for lane text | code read |
| GAP-10 | No onboarding routine | `bot_profiles/omomeow.json` |
| GAP-11 | Persona prompt has no lane/event/done rules | `bot_profiles/omomeow.json` |
| GAP-12 | Restart recovery covers user turns only, not lane deadlines or events | `src/multiplexer/actor.rs:86` |
| GAP-13 | No unified inbox: mail is only checked by `omocat-calendar-mail-alerts-v1` (script alerts), KakaoTalk is only archived for one room, iMessage/SMS is not read at all, Discord messages are stored but never triaged | cron_jobs rows (DB copy 2026-10-10); `imsg`, `zele`, PirateTalk API reachable |
| GAP-14 | No owner/bot task classification or task list | code read |
| GAP-15 | The gateway cannot see or talk to OMO sessions outside its own app-server threads; `omo thread` (list/send/read/interrupt, `--json`, exit codes) exists but is unused; 17 live local terminal sessions and 1 on omarchy were found on 2026-10-10 | `omo thread list --all-scope --json`; ssh probe |
| GAP-16 | No "adopt and finish" path for an existing session | code read |
| GAP-17 | Cron failure handling stops at one alert: `src/cron/scheduler.rs:1720-1790` inserts `cron_incidents(job_id, error_signature)` and posts "Cron job X failed: <error>" to the job's failure destinations; success deletes incidents. Nothing detects missed fires for recurring jobs (only one-shot "missed", `:960-1012`), stuck `running` rows, delivery failures on succeeded runs (`last_delivery_error` is stored but never alerted), or "succeeded but did nothing". 7-day DB copy (2026-10-10): 760 succeeded, 5 failed (kakao-archive HTTP error x2, notion-todo and aside-feature-request "omo turn ended with no content", playstore traceback) | code read + DB copy |
| GAP-19 | No decision layer: every "should we wake / which class / how big / is it done" choice would be an LLM turn; one model for everything (`OMON_DEFAULT_MODEL=mahoquot/nekos-claude-sonnet-5-5` in launchd env, `src/agent/omo_config.rs:62-68`); no per-purpose model selection, no usage accounting; jev is installed (`typesafe-judge` choice/noul/score/diff/eval, key in `~/.zshenv`) but unused by the gateway | code read + launchctl env + CLI probe |
| GAP-18 | No remediation: no diagnosis, no fix, no verified re-run; acknowledged signatures are suppressed forever (`:1725-1782`) | code read |

### Must have
- One brain (IS-1) before any lane code is deployed.
- All external commands as argv arrays (`tokio::process::Command::new(..).arg(..)`), never `sh -c`; pane text containing `$(` is refused.
- Lanes run as OMO app-server threads through the existing `OmoBackend` (`src/agent/omo_backend.rs`); lane progress comes from app-server notifications (`turn/started`, `item/completed`, `turn/completed`, `turn/error`), never from terminal text. No herdr dependency anywhere in new code; existing herdr calls in `src/work_heartbeat.rs` and `bin/omomeow-delegate` are removed.
- Every todo ships cargo tests with fakes (a fake `AgentBackend`, fake gh/npm, `MockClock`), plus the real-surface QA it names.
- Release build signed with "Developer ID Application: Indo Yoon (5DUM8WPB4C)", never ad-hoc.
- Kill switches `OMON_LANES_ENABLED`, `OMON_KIBITZER_ENABLED` (default true, zero-config, no .env edits).
- Tiered decisions (owner rule 2026-10-10): T0 Rust/SQL (no model) -> T1 jev for every classification, routing, wake and verification decision -> T2 `inferhub/cb/deepseek-v4.1-flash` for short writing and small lanes -> T3 `mahoquot/claude-opus-5-5` only for the owner-facing lead and large lanes/root-cause fixes. A jev failure or timeout (5 s) falls back to T2 with the same strict JSON contract, never straight to T3. Daily caps and a circuit breaker bound jev and fallback usage (todo 33); `jev-1.13-free` has no published price or limit (checked 2026-10-10), so the caps, not the price, are the guarantee. jev receives only excerpts (max 4000 chars), never full message bodies, secrets or tokens (its endpoint is external: `TYPESAFE_URL`).
- Event-first, no wasted tokens (owner rule 2026-10-10): a model is called only when a new event exists (a new inbox item, a finished cron run with a problem, a lane report, an owner message). Nothing calls a model on a timer to "check". Timers are allowed only for cheap local checks with no model call (SQL, a process exit status, a file watcher) and must say so in their todo.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Models are exactly the two the owner named on 2026-10-10: `mahoquot/claude-opus-5-5` (lead, large lanes, root-cause remediation) and `inferhub/cb/deepseek-v4.1-flash` (everything small); set through new gateway env defaults `OMON_MODEL_LEAD`, `OMON_MODEL_LARGE`, `OMON_MODEL_SMALL` compiled in as defaults (zero-config, no `.env` edits); no edits to `~/.omo/agent/omo.json` or `~/.omo/omo.json`; cron jobs keep their own `model` field.
- No Slack, Huddle/Google Meet or gh-notification integrations (owner selected none on 2026-10-10). Mail/calendar READ access through zele, iMessage/SMS read through imsg, KakaoTalk read through PirateTalk are now IN scope (owner request 2026-10-10, item 1).
- No katok; no KakaoTalk sending; no agent-discord (user account) sending.
- No bundling of the bun dori CLI and no second always-on daemon/script; no herdr (owner does not use it) and no terminal-scraping lane control.
- No edits to .env files.
- Do not modify or commit the owner's untracked/dirty files: `docs/research/*`, `.omo/plans/gateway-autonomy-lanes.md`, `.omo/thread-tools/`, and existing WIP in `src/agent/omo_backend.rs` beyond the lines a todo names. Run `git status --short` before each commit and stage only the todo's files.
- No Discord UI-driving QA (owner verifies in Discord); evidence comes from gateway logs, DB rows (on copies) and captured sink payloads.
- No tests that pin prose (persona wording).
- Inbox triage is read-only toward sources: no sending, replying, reacting, archiving or marking read in mail, KakaoTalk, iMessage or Discord on the owner's behalf; replies to other people are drafted and need the owner's explicit OK.
- KakaoTalk only through the PirateTalk API (katok forbidden); no zele `mail send|reply|archive|read-mark`; no `imsg send|read|react`.
- Remote machines only through the owner's existing `~/.ssh/config` hosts with `BatchMode=yes`; no new keys, no agents installed on remote machines, no port exposure.
- Cron remediation never edits `.env` files, never changes a job's model, never disables or deletes a job, never re-runs a job more than once per incident without the owner, and never re-runs jobs whose effect is external publishing (`omon-playstore-daily-publish`, `omon-vpn-mall-daily-publish-v2`, `omon-content-intel-publish-ready`, any job whose payload has `"external_effect": true`) without the owner's OK; quota claims are verified against mahoquot `127.0.0.1:18801` before being reported.

## Verification strategy
> Zero human intervention - all verification is agent-executed, except the two OWNER confirmations explicitly marked.
- Test decision: tests-after per todo, `cargo test` (tokio). New integration test files under `tests/` registered in `Cargo.toml` the way `tests/test_voice_transcription.rs` is (`Cargo.toml:65-66`).
- Fakes: a scripted `AgentBackend` (pattern: the test runners in `src/multiplexer/actor.rs:1000-1320`) that emits turn outcomes and agent messages; existing `MockClock` (`src/work_heartbeat.rs:84`); a `SignalRunner` trait with fake gh/npm/command/url.
- Gates per todo: `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all-targets --all-features`; `cargo test --test test_review_dashboard` (dashboard is not in the lib, memory note). Known flaky/pre-existing: `agent::omo_daemon` tests under load and ~10 `test_omo_backend` failures - compare to the todo-2 baseline, never "fix" them in this plan.
- Reading the live DB: the running gateway (pid from `launchctl list | grep ai.omon.gateway`) opens `/Users/indo/code/project/omon-gateway/omon_gateway.db` (verified with `lsof -p <pid>` on 2026-10-10; the files under `~/.omon/*.db` are 0 bytes). Copy `omon_gateway.db{,-wal,-shm}` to a `mktemp -d` dir first (live file is locked, memory note). Never write to the live DB from QA.
- Real-surface QA uses a throwaway git repo at `$(mktemp -d)/qa-repo` and the live app-server daemon on `ws://127.0.0.1:19742` via a debug gateway pointed at a scratch DB (`DATABASE_URL=sqlite://<tmp>/qa.db`); evidence: `.omo/evidence/dori-grade-agent-env/task-<N>.{log,json,txt}` with a cleanup receipt line per QA.
- Deploy QA: `cargo build --release`; `codesign --force --options runtime --sign "Developer ID Application: Indo Yoon (5DUM8WPB4C)" target/release/omo-gateway`; `codesign --verify --verbose=2`; `launchctl kickstart -k gui/501/ai.omon.gateway`; `rg` in `~/.omon/gateway.log` (UTC; KST = +9 h).

## Execution strategy
### Lane runtime (decided 2026-10-10 after the owner said he does not use herdr)
A lane is a separate OMO app-server thread, driven by the gateway through the existing `OmoBackend` (`src/agent/omo_backend.rs`), on the same daemon (`ws://127.0.0.1:19742`, single-daemon multiplexer rule) the bot already uses:
- Identity: lane session key `SessionKey { platform: "lane", guild_id: None, channel_id: <lane key>, thread_id: None, user_id: "lane:<key>", bot_id: None }`; the app-server thread id is persisted in that session's `metadata.omo_thread_id` exactly like chat sessions (`src/agent/omo_backend.rs:262-300`), so `thread/resume` works after restarts and for reopen.
- Folder: `thread/start` carries `cwd` = the lane's working directory (`src/agent/omo_protocol.rs:26-56`), so the lane's bash/edit tools operate in the target repo; memory identity follows cwd.
- Turn budget: lane turns use `OMON_LANE_TURN_SECS` (default 3600) instead of the 1800 s interactive cap (`INTERACTIVE_TURN_CAP`, `src/agent/omo_backend.rs:76`) and skip the 45 s initial-response fail-fast and Discord typing (both gated on `is_cron_session` today, `:735-760`, `:950-975`); a lane runs as a chain of turns: the runner starts the next turn ("continue; report with [REPORT] lines; claim when done") until the lane claims done, blocks, is objected/closed, or exceeds `OMON_LANE_MAX_TURNS` (default 24) -> status failed with an event.
- Output: the lane's agent messages are not sent to Discord directly; a capturing `OutboundDispatcher` (trait at `src/multiplexer/actor.rs:26-28`) collects them so the runner can parse `[REPORT]` lines and post curated text to the lane thread.
- Talking to a lane: `turn/steer` into the running turn (`OmoBackend::steer`, `src/agent/omo_backend.rs:1811`), or the message becomes the next turn's prompt when idle.
- Observing a lane: turn outcomes (`turn/completed`, `turn/error`, deadline) returned by `AgentBackend::run` plus captured messages - structured, no terminal text.
- Inspecting a lane by a human: `omo-gateway lane log <key>` prints the captured transcript; the dashboard session view lists `lane:*` sessions (existing `/api/sessions`, `src/dashboard.rs:720-728`).

### Parallel execution waves
- Wave 0 (serial): 1, 2
- Wave 1 (foundation, parallel): 3, 4, 5, 6, 33, 34
- Wave 2 (lane engine): 7, 8, 9, 10, 11
- Wave 3 (events + done): 12, 13, 14, 15
- Wave 4 (proactive + guard + front of house): 16, 17, 18, 19, 20, 21
- Wave 4b (inbox triage + session console, parallel with Wave 4 where dependencies allow): 22, 23, 24, 25, 26, 27, 28, 29
- Wave 4c (cron watchdog + usage): 30, 31, 32, 35
- Wave 5 (cutover): 36, 37

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | owner go | 36 | 2 |
| 2 | - | 3-37 | 1 |
| 3 | 2 | 8, 9, 12 | 4, 5, 6 |
| 4 | 2 | 7-19 | 3, 5, 6 |
| 5 | 2 | 8, 13 | 3, 4, 6 |
| 6 | 2 | 8, 17 | 3, 4, 5 |
| 7 | 4 | 8-11, 13, 15, 17 | - |
| 8 | 3, 4, 5, 6, 7, 10, 33, 34 | 9, 11, 12, 13, 18, 19 | - |
| 9 | 3, 8 | 15 | 10, 11 |
| 10 | 7 | 8, 12, 15, 18 | 9, 11 |
| 11 | 8 | 21 | 9, 10 |
| 12 | 3, 8, 10 | 13, 14, 16, 17, 18, 19 | - |
| 13 | 5, 7, 12 | 15 | 14 |
| 14 | 12 | 16, 17 | 13 |
| 15 | 7, 9, 10, 13 | 36 | - |
| 16 | 12, 14 | 36 | 17-21 |
| 17 | 6, 7, 12, 14 | 36 | 16, 18-21 |
| 18 | 8, 10, 12 | 36 | 16, 17, 19-21 |
| 19 | 8, 12 | 36 | 16-18, 20, 21 |
| 20 | - (persona; schedule in Wave 4) | 36 | 16-19, 21 |
| 21 | 11 | 36 | 16-20 |
| 22 | 2 | 23-26 | 27 |
| 23 | 22 | 24, 26 | 27 |
| 24 | 14, 22, 23 | 25, 26 | 27 |
| 25 | 24 | 31, 32, 36 | 26, 27, 28 |
| 26 | 8, 24 | 36 | 25, 27, 28 |
| 27 | 2 | 28, 29 | 22-26 |
| 28 | 8, 12, 13, 27 | 29, 36 | 25, 26 |
| 29 | 20, 27, 28 | 36 | 25, 26 |
| 30 | 4, 8 | 31, 32 | 22-29 |
| 31 | 8, 13, 25, 30 | 32, 36 | 26-29 |
| 32 | 25, 30, 31 | 36 | - |
| 33 | 2 | 8, 12, 14, 16, 24, 31, 34, 35 | 3-6, 34 |
| 34 | 2 | 8, 16, 24, 35, 36 | 3-6, 33 |
| 35 | 13, 33, 34 | 36 | 30-32 |
| 36 | 1, 15-21, 25, 26, 28, 29, 31, 32, 34, 35 | 37 | - |
| 37 | 36 | F1-F4 | - |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. Retire the legacy herdr-based OmOMeow (bot session, listener and its herdr server)
  What to do: OWNER - this stops a running service; get the owner's go in the execute session before step (b). (a) backup: `mkdir -p ~/.omon/omomeow-backup-<YYYY-MM-DD>/launchagents && cp ~/Library/LaunchAgents/ai.omomeow.herdr.plist` into it; (b) `launchctl bootout gui/501/ai.omomeow.herdr`; `launchctl disable gui/501/ai.omomeow.herdr`; (c) move (not delete) the plist into the backup dir so RunAtLoad cannot revive it; (d) kill the listener: `pkill -f "/Users/indo/.omon/omomeow/bin/listen.ts"` (bash loop and bun) and `pkill -f "/Users/indo/.omon/omomeow/bin/watch.ts"`; (e) the herdr server was started by `herdr-boot.sh` only for the legacy bot and the owner does not use herdr: before stopping it, run `herdr agent list` and save it to the evidence file; if any agent other than the OmOMeow pane (cwd `/Users/indo/.omon/omomeow`) is `working`, stop and ask the owner; otherwise `herdr server stop`; (f) find the re-enable path: `rg -n "launchctl (enable|bootstrap|load|kickstart).*omomeow" ~/.omon ~/code/project/omon-gateway ~/.omo/agent 2>/dev/null` and search the legacy session transcript (session id `01a0f31c-bace-7f58-90c9-87da816f7762`, under `~/.omo/agent/sessions`) for "launchctl"; record findings; remove only references that re-enable ai.omomeow.herdr.
  Must NOT do: delete `~/.omon/omomeow`; uninstall herdr; touch `ai.omon.gateway`.
  Closes: GAP-1
  Parallelization: Wave 0 | Blocked by: owner go | Blocks: 36
  References: `~/.omon/omomeow/bin/herdr-boot.sh` (starts herdr server, listen.ts loop, resumes session 01a0f31c-...); plist (KeepAlive true, RunAtLoad true, ThrottleInterval 10); memory `reference/omon-gateway-split-brain-2026-10-08.md`; `herdr agent list` on 2026-10-10 showed only w8:p1 (OmOMeow, working) and w8:p38 (pixel-vfx-local, done)
  Acceptance criteria: `launchctl print-disabled gui/501 | grep omomeow.herdr` shows disabled or nothing; `test ! -e ~/Library/LaunchAgents/ai.omomeow.herdr.plist`; `pgrep -f "omomeow/bin/(listen|watch).ts"` exits 1; `pgrep -f "herdr server"` exits 1 (or the owner explicitly kept it in step e).
  QA scenarios: happy - run all acceptance commands, stdout to `.omo/evidence/dori-grade-agent-env/task-1.txt`; failure - re-run the pgrep/launchctl checks 10 minutes later (catches respawn) and append; any hit = FAIL, reopen (f).
  Commit: N (machine state only; listed in the final report)
  Recommended task executor category: unspecified-low - exact operational commands, no code.

- [x] 2. Record the test and build baseline before any code change
  What to do: on current HEAD run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features 2>&1 | tee .omo/evidence/dori-grade-agent-env/task-2-baseline.log`, `cargo test --test test_review_dashboard`; write the failing test names to `task-2-failing.txt`.
  Must NOT do: fix or skip failing tests; stash or commit the owner's WIP.
  Closes: verification precondition for all GAP rows
  Parallelization: Wave 0 | Blocked by: - | Blocks: 3-31
  References: memory `reference/omon-gateway.md` (dashboard test trap, omo_daemon flakiness)
  Acceptance criteria: both files exist; the failing list is explicit (may be empty).
  QA scenarios: happy - files written; failure - if HEAD does not compile, report BLOCKED with the first compiler error.
  Commit: N
  Recommended task executor category: quick - run commands and save logs.

- [x] 3. Let OmoBackend run lane sessions (cwd, budget, no Discord side effects)
  What to do: in `src/agent/omo_backend.rs` add a lane classification next to the cron one: `is_lane = session.key.platform == "lane"`. For lane sessions: (1) workspace = `session.state.metadata["lane_cwd"]` (absolute path; create nothing - the runner validated it) instead of the per-agent workspace slug (`:330-372`); `runtimeWorkspaceRoots` = `[lane_cwd]`; (2) total timeout = `OMON_LANE_TURN_SECS` (default 3600) instead of `INTERACTIVE_TURN_CAP` (`:1602-1615`); (3) skip the Discord typing dispatch and the 45 s initial-response fail-fast and the progress steer (treat like `is_cron_session` at `:735-760`, `:950-975`); (4) keep the thread binding persistent like chat sessions (do NOT clear `omo_thread_id` as cron does at `:262-270`). Add `OMON_LANE_TURN_SECS` parsing in `src/agent/omo_config.rs` with the zero-config default. Model: the lane row's `model` (set by todo 8 from the jev size decision: `OMON_MODEL_LARGE` or `OMON_MODEL_SMALL`), passed via `session.state.active_model` so `thread/start`/`turn/start` carry it (existing path, `src/agent/omo_backend.rs:241-247, 637-662`).
  Must NOT do: change behavior for discord/cron/web sessions; change model routing; touch the user's unrelated WIP hunks in this file (diff only the lines above; check `git diff src/agent/omo_backend.rs` before and after).
  Closes: GAP-2, GAP-3 (foundation)
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 8, 9, 12
  References: `src/agent/omo_backend.rs:76, 236-300, 330-372, 735-760, 950-975, 1602-1615`; `src/agent/omo_protocol.rs:26-56` (`cwd`, `runtimeWorkspaceRoots`); `src/agent/agent_workspace.rs:40-60`; memory `reference/omo-daemon-multiplexer.md` (thread/start cwd semantics)
  Acceptance criteria: new tests in `tests/test_omo_backend.rs` style using the existing fake app-server harness there: a lane session's `thread/start` request carries `cwd` = `lane_cwd`; no Typing action is dispatched; a 50 s silent turn is not interrupted (a discord session with the same script is); the thread id is persisted and the next turn sends `thread/resume`. Existing discord/cron tests unchanged vs the todo-2 baseline.
  QA scenarios: happy - `cargo test --test test_omo_backend lane_ > task-3.log`; real - a `#[ignore]` test running one lane turn against the live daemon with `lane_cwd` = a mktemp git repo and the prompt "run pwd and reply with the output" asserts the reply contains that path; failure - `lane_cwd` pointing to a missing dir returns Err (no panic), logged.
  Commit: Y | feat(backend): lane sessions with their own cwd and turn budget
  Recommended task executor category: deep-high - touches the shared backend's turn lifecycle.

- [x] 4. Add the lanes registry schema and storage functions
  What to do: `migrations/0031_lanes.sql`: table `lanes(key TEXT PRIMARY KEY CHECK(length(key) BETWEEN 1 AND 48 AND key NOT GLOB '*[^a-z0-9-]*'), title TEXT NOT NULL, lead_session_key TEXT NOT NULL, thread_channel_id TEXT, parent_channel_id TEXT, status_message_id TEXT, cwd TEXT NOT NULL, brief_path TEXT NOT NULL, session_key TEXT NOT NULL, omo_thread_id TEXT, model TEXT, status TEXT NOT NULL CHECK(status IN ('queued','working','blocked','done_claimed','not_done','verified_done','closed','failed')), done_signals TEXT NOT NULL, claim_evidence TEXT, claim_at DATETIME, objection TEXT, turns_run INTEGER NOT NULL DEFAULT 0, last_turn_outcome TEXT, last_report TEXT, last_activity_at DATETIME NOT NULL, stale_posted_at DATETIME, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)`; `lane_history(id INTEGER PRIMARY KEY AUTOINCREMENT, lane_key TEXT NOT NULL REFERENCES lanes(key), at DATETIME NOT NULL, from_status TEXT, to_status TEXT NOT NULL, reason TEXT)`; `lane_events(id INTEGER PRIMARY KEY AUTOINCREMENT, lane_key TEXT, lead_session_key TEXT NOT NULL, kind TEXT NOT NULL, body TEXT NOT NULL, created_at DATETIME NOT NULL, delivered_at DATETIME)`; `lane_transcript(id INTEGER PRIMARY KEY AUTOINCREMENT, lane_key TEXT NOT NULL, at DATETIME NOT NULL, role TEXT NOT NULL, text TEXT NOT NULL)`; indexes on `lanes(status)`, `lane_events(delivered_at, lead_session_key)`, `lane_transcript(lane_key, id)`. Storage functions in `src/storage/lanes.rs`: `insert_lane`, `get_lane`, `get_lane_by_thread`, `list_lanes(statuses)`, `transition_lane(key, from_set, to, reason) -> bool` as one transaction (`UPDATE ... WHERE key=? AND status IN (...)` + history insert; false when the CAS misses), `set_lane_fields`, `append_transcript`, `read_transcript(key, last_n)`, `enqueue_lane_event`, `take_undelivered_events(limit)`, `mark_events_delivered(ids)`. Re-export from `src/storage/mod.rs`.
  Must NOT do: alter or drop `work_items` (empty in the live DB copy of 2026-10-10; re-check with `SELECT count(*) FROM work_items` on a DB copy and record it in task-4.log - if non-zero, list the rows and leave them for the owner).
  Closes: GAP-2, GAP-4, GAP-12
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 7-19
  References: `migrations/0030_work_items.sql`; `src/storage/db.rs:962-1110` (WorkItem functions, sqlx style); `tests/test_work_heartbeat.rs:16-40` (migration test pattern)
  Acceptance criteria: new `tests/test_lanes_storage.rs`: migration creates the four tables; two concurrent tokio tasks transitioning the same lane from the same state -> exactly one true; history count equals successful transitions; key "Bad Key" rejected.
  QA scenarios: happy - `cargo test --test test_lanes_storage > task-4.log`; real-data - copy `/Users/indo/code/project/omon-gateway/omon_gateway.db*` to a temp dir, open it with `Database::connect` (applies migrations), log `SELECT count(*) FROM work_items` before/after (unchanged) and the new tables' presence.
  Commit: Y | feat(lanes): registry schema, history, transcript and event queue
  Recommended task executor category: deep-low - schema plus transactional storage.

- [x] 5. Implement done-signal parsing and checking
  What to do: `src/lanes/signals.rs` (create `src/lanes/mod.rs`, `pub mod lanes;` in `src/lib.rs`). Parse the dori grammar exactly: signals joined by `;`; kinds `merged <owner/repo>#N`, `closed <owner/repo>#N`, `published <pkg>@<version>`, `command ["argv",...] [stdout~"regex"]`, `file <path> [sha256=<hex>] [json:.a.b=<json>]`, `url <http(s)://...> [status=200] [body~"regex"]`. Reject unknown kinds, an empty list, and uncheckable commands (`["true"]`, `[":"]`). Trait `SignalRunner` with the real impl: merged -> `gh pr view N -R owner/repo --json state,mergeCommit` (state MERGED and mergeCommit non-null); closed -> `gh issue view N -R owner/repo --json state` (CLOSED); published -> `npm view <pkg>@<version> version` equals; command -> argv in the lane cwd, 120 s timeout, kill on timeout, exit 0 and optional stdout regex; file -> relative to cwd, `~` = HOME, sha256 (sha2 crate), JSON path compared as `serde_json::Value`; url -> reqwest GET 15 s, status and optional body regex. Return `Vec<SignalResult{signal, ok, detail}>`.
  Must NOT do: run through a shell; trust lane-reported results.
  Closes: GAP-4
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 8, 13
  References: upstream `sisyphuslabs/omo-dori-mode-experimental@021f93fd` `skills/dori-mode/references/sessions.md` (signal table) and `done-protocol.md`; `Cargo.toml` (add sha2/regex/reqwest only if missing)
  Acceptance criteria: unit tests for every kind: parse ok, parse reject (unknown kind, `command ["true"]`, malformed JSON value), check pass and fail with fake gh/npm, a local `tokio::net::TcpListener` for url, and a timeout test (`["sleep","5"]` with a 1 s override).
  QA scenarios: happy - `cargo test lanes::signals > task-5.log`; real - pick a merged PR with `gh pr list -R code-yeongyu/oh-my-openagent --state merged --limit 1 --json number` and an open one with `--state open`; run the real runner on both (`#[ignore]` test) - merged passes, open fails; log both.
  Commit: Y | feat(lanes): done-signal parser and live checker
  Recommended task executor category: deep-low - exact grammar parsing.

- [x] 6. Implement the host guard and heavy slots
  What to do: `src/lanes/guard.rs`. `sample()` reads `sysctl -n vm.loadavg` (first number), `memory_pressure` (line "System-wide memory free percentage: NN%"), `df -k /` (available GB), and the count of lanes in working|blocked|not_done|done_claimed from the DB. Thresholds (consts, override via `OMON_GUARD_LOAD_ALERT`, `OMON_GUARD_LOAD_OK`, `OMON_GUARD_MEM_FREE_MIN_PCT`, `OMON_GUARD_DISK_FREE_MIN_GB`, `OMON_LANES_MAX`): 150, 80, 20, 50, 6. `can_launch()` -> Ok or Hold(reasons) from memory, disk and active-lane count only (CPU never holds). `GuardWatcher` remembers the last state and reports ALERT/CLEAR only on transitions. Heavy slots: `~/.omon/state/heavy/slot.<n>` (n < `OMON_HEAVY_SLOTS`, default 1) locked with fs2 `try_lock_exclusive` in `spawn_blocking`, holder pid written, stale holder (pid gone) taken over; wait until load < loadOk before running.
  Must NOT do: shell strings; block the tokio runtime.
  Closes: GAP-7
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 8, 17
  References: upstream `scripts.md` (guard, can-launch, heavy) and `config.example.json` guard block (panesMax 20 there; lanes share one app-server daemon here, so the default cap is 6 concurrent lanes - reversible by env)
  Acceptance criteria: unit tests with injected samples: Hold at mem 10%, at disk 10 GB, at 7 active lanes; no Hold at load 300 alone; transition-only reporting; slot takeover when the holder pid is dead.
  QA scenarios: happy - `cargo test lanes::guard` plus `#[ignore]` live sample printing real numbers (`task-6.log`); failure - live sample with `OMON_GUARD_DISK_FREE_MIN_GB=100000` returns Hold with a "disk" reason.
  Commit: Y | feat(lanes): host guard and heavy slots
  Recommended task executor category: deep-low - system probes with fixed thresholds.

- [x] 7. Add the lane state machine
  What to do: `src/lanes/state.rs`: `enum LaneStatus` matching the CHECK list; allowed transitions: queued->working|failed; working->blocked|done_claimed|failed|closed(force only); blocked->working|done_claimed|failed; done_claimed->verified_done|not_done; not_done->working|done_claimed; verified_done->closed; closed->working (reopen); failed->working (relaunch/reopen). `transition(pool, key, to, reason)` calls `transition_lane` with the allowed from-set and returns a typed `IllegalTransition` otherwise.
  Must NOT do: allow closing without verified_done except `lane close --force` (reason must start with "forced:").
  Closes: GAP-4
  Parallelization: Wave 2 | Blocked by: 4 | Blocks: 8-11, 13, 15, 17
  References: upstream `sessions.md` registry statuses; todo 4 storage
  Acceptance criteria: table-driven test over all (from,to) pairs: allowed pairs succeed; others return IllegalTransition and write no history row.
  QA scenarios: happy - `cargo test lanes::state > task-7.log`; failure - working->verified_done returns the error and the history count is unchanged (asserted).
  Commit: Y | feat(lanes): lane status machine
  Recommended task executor category: quick - small table-driven module.

- [x] 8. Implement the lane runner and the `omo-gateway lane` control commands
  What to do: (A) `src/lanes/runner.rs` `LaneRunner` owned by the gateway process (spawned in `src/main.rs` after the cron backend is built, `src/main.rs:817-835`, unless `OMON_LANES_ENABLED=false`), holding its own `OmoBackend` built with the lane config (same appserver URL, `OMON_LANE_TURN_SECS`) and a `CapturingDispatcher` that appends every agent message for `lane:*` sessions to `lane_transcript` (never to Discord). One tokio task per active lane: loop { build `SessionContext` for the lane key with metadata `lane_cwd`, `omo_thread_id` (from the row); prompt = first turn: launch keywords (`OMON_LANE_KEYWORDS`, default "ulw set goal and work") + "\n\n" + brief file content (read by the runner, passed as data, never through a shell); later turns: the queued lead message if any, else "Continue the lane. Report progress with [REPORT] lines; when every Done signal holds, run the claim command."; run via `AgentBackend::run_cancelable`; record outcome, `turns_run += 1`, persist the thread id; stop the loop when status leaves working/not_done, on `turns_run >= OMON_LANE_MAX_TURNS` (default 24, -> failed + event), or on 3 consecutive turn errors (-> failed + event `dead`) }. Resume all working/not_done lanes on gateway start. (B) CLI: add `Command::Lane(LaneArgs)` to `src/entry.rs` (binary entry, `:43-55`); subcommands print one JSON object; exit 0 ok, 2 invalid input, 4 hold. Because the runner lives in the gateway process, the CLI only writes intent to the DB and the runner picks it up within 2 s (poll `lanes`/a `lane_commands` view is NOT needed - use the `lanes` row fields plus `lane_events`/a small `lane_inbox(lane_key, at, text, consumed_at)` table added to migration 0031 if todo 4 has not shipped, else `0032_lane_inbox.sql`). Subcommands: `start <key> --title T --brief FILE --done "..." [--thread ID] [--cwd DIR] [--lead SESSION_KEY]` (validate key; parse `--done` with todo 5 -> exit 2, no row; `can_launch` Hold -> row `queued`, exit 4; append the brief footer: key, `Done = ...`, report grammar `[REPORT] <key> | milestone|blocker|question|done | <text>`, claim command `omo-gateway lane claim-done <key> --evidence "..."`, the upstream done-protocol text; when `--thread` names a channel, create the lane thread via todo 10; insert row status working; runner starts it); `list [--status S]`; `log <key> [--last N]` (transcript); `send <key> --text T` (inbox; the runner steers the running turn via `OmoBackend::steer`, else uses it as the next prompt); `claim-done <key> --evidence T` (called by the lane agent itself from its bash tool; key also read from `OMON_LANE_KEY` env when the runner passes it - verify whether app-server thread env is configurable; if not, the key is in the brief footer and is required); `object <key> --reason T`; `close <key> [--force --note T]`; `reopen <key>`; `can-launch`; `heavy <label> -- <argv...>`. Lane model: `--model small|large|auto` (default auto): auto asks jev (todo 33) `choice` small vs large on the brief's first 2000 chars with the instruction "small = one file, obvious, under 30 min; large = multi-file, design, debugging or unclear" -> `OMON_MODEL_SMALL` (`inferhub/cb/deepseek-v4.1-flash`) or `OMON_MODEL_LARGE` (`mahoquot/claude-opus-5-5`); stored in `lanes.model`; a small lane that fails its done check twice is escalated once to large (history reason "escalated"). Lead session key: `--lead`, else the newest `sessions.session_key` whose `thread_id` or `channel_id` equals the thread/channel id, else `OMON_LANE_DEFAULT_LEAD` (owner home DM key).
  Must NOT do: shell strings; post the lane agent's raw output to Discord; change the model; start a second app-server daemon.
  Closes: GAP-2, GAP-3, GAP-9 data
  Parallelization: Wave 2 | Blocked by: 3, 4, 5, 6, 7, 10, 33, 34 | Blocks: 9, 11, 12, 13, 18, 19
  References: `src/entry.rs:37-80` (clap Command); `src/agent/backend.rs:30-62` (`AgentBackend::run`, `run_cancelable`, `steer`); `src/agent/omo_backend.rs:105-110` (`OmoBackend::new`), `:1811-1860` (steer); `src/multiplexer/actor.rs:26-28` (`OutboundDispatcher`), `:1300-1320` (capturing dispatcher test pattern); `src/main.rs:817-835` (cron backend construction pattern); `bin/omomeow-delegate` (old contract); upstream `scripts.md` launch/claim-done
  Acceptance criteria: `tests/test_lane_runner.rs` with a scripted fake `AgentBackend`: start -> first turn prompt contains the brief and the footer; the runner chains turns until the fake claims (fake calls the storage claim path); 24-turn cap -> failed + event; 3 consecutive errors -> failed + `dead` event; `send` while a turn runs calls steer, while idle becomes the next prompt; invalid `--done` exits 2 with no row; Hold exits 4 with a queued row; gateway restart (new runner on the same DB) resumes a working lane with the stored thread id.
  QA scenarios: happy - debug gateway on a scratch DB with the live daemon: `QA=$(mktemp -d); git -C $QA init qa-repo`; brief asking the agent to create `done.txt`, emit one milestone and run the claim command; `target/debug/omo-gateway lane start qa-1 --title "QA lane" --brief $QA/brief.md --done 'file done.txt' --cwd $QA/qa-repo --lead <scratch session key>`; within 10 min the row shows turns_run >= 1, transcript has a `[REPORT]` line and `done.txt` exists; save JSON + `lane log qa-1` to `task-8.json`; failure - `lane start qa-2 ... --done 'command ["true"]'` exits 2 (`task-8-fail.txt`). Cleanup: `lane close qa-1 --force --note "forced: qa"`, stop the debug gateway (kill pid, verify `kill -0` fails), `rm -rf $QA`; record the receipt.
  Commit: Y | feat(lanes): lane runner on app-server threads and lane control commands
  Recommended task executor category: deep-high - long-lived async task orchestration across backend, DB and CLI.

- [x] 9. Implement lane reopen
  What to do: `reopen <key>` allowed from closed|failed: transition to working with history "reopened", reset `turns_run` to 0 and the error counter, keep `omo_thread_id` so the runner resumes the same app-server thread (`thread/resume`); first prompt after reopen = the owner's reopening message (when triggered from Discord, todo 15) or "The lane was reopened; re-check state and continue". If `thread/resume` fails (thread gone), clear `omo_thread_id`, start a fresh thread with the original brief plus "Resumed lane; previous thread unavailable", and record that in history.
  Must NOT do: reopen verified_done lanes without passing through closed.
  Closes: GAP-9 (IS-9 data path)
  Parallelization: Wave 2 | Blocked by: 3, 8 | Blocks: 15
  References: `src/agent/omo_backend.rs:262-330` (thread/resume path, `clear_stuck_thread` at `:594-600`); upstream `sessions.md` ("reopen its recorded session")
  Acceptance criteria: fake-backend tests: reopen from closed resumes with the stored thread id; a resume failure falls back to a fresh thread and records history; reopen on working returns IllegalTransition.
  QA scenarios: happy - after todo 8's QA lane, force-close then `lane reopen qa-1 --text "add a line to done.txt"`; the next transcript entries show the agent continuing with prior context (it references done.txt) (`task-9.log`); failure - `lane reopen nope` exits 2.
  Commit: Y | feat(lanes): reopen closed lanes on their recorded thread
  Recommended task executor category: deep-low.

- [x] 10. Add the Discord lane-thread helper
  What to do: `src/lanes/discord_threads.rs` with serenity `Http` from `DISCORD_BOT_TOKEN` (or the first of `DISCORD_BOT_TOKENS`, same parsing as `src/main.rs:128-147`): `create_thread(parent_channel, title)` (starter message "lane <key>: <title>" then a public thread named "🔄 <title>", `safe_allowed_mentions()`); `set_status(thread, Working|Waiting|Done)` renaming the prefix to 🔄/⏸/✅ (cap 100 chars, truncate on char boundaries); `archive(thread)`, `unarchive(thread)`; `post(thread, text)` and `edit(thread, msg_id, text)`. Errors are returned, never swallowed.
  Must NOT do: use agent-discord or a user token; mention @everyone/@here/roles.
  Closes: GAP-8
  Parallelization: Wave 2 | Blocked by: 7 | Blocks: 8, 12, 15, 18
  References: `src/discord/adapter.rs:12` (CreateThread, EditMessage), `:144` `safe_allowed_mentions`, `:211-310` thread naming, `:1254-1282` archived tracking, `:3104-3150` edit flow; OMOMEOW-MODE rule 14
  Acceptance criteria: unit tests for name formatting (prefix replace, 100-char cap, Korean text truncated on a char boundary).
  QA scenarios: happy - `#[ignore]` live test in `DISCORD_HOME_CHANNEL`: create "🔄 qa-lane-thread", rename to ✅, archive; log returned ids and the archived flag from GET channel (`task-10.log`); failure - an invalid channel id returns Err with the Discord error code logged.
  Commit: Y | feat(lanes): discord lane thread lifecycle helper
  Recommended task executor category: unspecified-low - thin API wrapper.

- [x] 11. Route bin/omomeow-delegate through `lane start` and drop its herdr use
  What to do: keep its flags (`--title --repo --brief --thread-id --cwd --db`) and add `--done`; key = slug(title) ([a-z0-9-], max 40) + "-" + 4 random hex; write the brief to `~/.omon/state/lanes/<key>/brief.md`; when `--done` is absent use `command ["git","-C","<cwd>","diff","--quiet"]` and print a warning that a real signal is better; exec `omo-gateway lane start` with argv. Delete the herdr branch (`bin/omomeow-delegate:104-109`) and the `work_items` insert. Update `tests/test_omomeow_delegate.sh`.
  Must NOT do: keep writing `work_items` rows; call herdr.
  Closes: GAP-2
  Parallelization: Wave 2 | Blocked by: 8 | Blocks: 21
  References: `bin/omomeow-delegate`; `tests/test_omomeow_delegate.sh`; `docs/AGENTS_DELEGATE.md`
  Acceptance criteria: `bash tests/test_omomeow_delegate.sh` passes with a stub `omo-gateway` on PATH that records argv; recorded argv contains `lane start` and the done flag; `grep -c herdr bin/omomeow-delegate` is 0.
  QA scenarios: happy - test log `task-11.log`; failure - missing `--title` exits non-zero with usage.
  Commit: Y | refactor(delegate): route omomeow-delegate through lane start
  Recommended task executor category: quick.

- [x] 12. Turn lane activity into lane events and thread posts (replace the herdr heartbeat)
  What to do: in the runner (todo 8), after every turn and every captured message: parse `[REPORT] <key> | kind | text` lines from captured agent messages (store `last_report`, update `last_activity_at`); kinds blocker/question -> status blocked + event (`blocked`/`question`) and the runner pauses the loop until the lead answers via `send` or `object`; kind done without a claim -> event `idle_without_claim`; milestone -> thread post (todo 18), no event. Turn ended without any `[REPORT]` line and no claim -> jev `choice` (finished / progress / blocked_or_question) on the turn's last agent message (max 2000 chars): progress -> next turn (no event); finished -> event `idle_without_claim`; blocked_or_question -> as blocker/question above. Quiet: no captured message for 20 min while a turn runs -> post `last_report` to the lane thread once (`stale_posted_at`) and event `stale`; activity resets it. 3 consecutive turn errors -> `dead` (from todo 8). Remove the herdr dependency from `src/work_heartbeat.rs`: delete `HerdrCliLivenessChecker` (`:123-168`) and its use in `src/main.rs:919-940`; keep `WorkHeartbeatRunner` only if `work_items` is still referenced, otherwise remove its spawn (record which in the commit body).
  Must NOT do: wake the lead for milestones or routine activity; call herdr.
  Closes: GAP-3, GAP-5
  Parallelization: Wave 3 | Blocked by: 3, 8, 10 | Blocks: 13, 14, 16-19
  References: `src/work_heartbeat.rs:118-168, 290-400`; `src/main.rs:919-940`; upstream `scripts.md` freshness/dead-panes (semantics only); IS-6 evidence
  Acceptance criteria: `tests/test_lane_events.rs` with the scripted fake backend + MockClock: question report -> status blocked, one event, runner paused; milestone -> one thread post, zero events; 20 min silence -> one post + one `stale` event, not repeated on restart (`stale_posted_at` honored); `rg -n herdr src` returns no matches.
  QA scenarios: happy - `cargo test --test test_lane_events > task-12.log`; live - QA lane whose brief prints `[REPORT] qa-1 | question | pick A or B` and stops: a `lane_events` row kind `question` within 2 min (scratch DB), logged; failure - a fake backend that errors 3 times -> status failed + `dead` event (test).
  Commit: Y | feat(lanes): lane reports become events and thread posts; remove herdr liveness
  Recommended task executor category: deep-high - lifecycle/timing with restart idempotency.

- [x] 13. Implement the done flow: claim, object, five-minute settle
  What to do: `claim-done` -> done_claimed with `claim_at`, `claim_evidence`; event `done_claimed`; the runner stops starting new turns and steers/queues the lane message "[LEAD] claim received; stay idle; closes at <HH:MM KST> unless objected". `object` -> not_done with `objection`; queue the lane message "[LEAD] not done: <reasons>; keep working, claim again when fixed" and the runner resumes turns. Settler (runs every 30 s in the runner) for done_claimed lanes with `claim_at <= now - 5 min`: run all signals (todo 5) once, plus worktree checks for the lane cwd and every worktree from `git -C <cwd> worktree list --porcelain` whose path is under the lane cwd or was created after lane start: `git -C <wt> status --porcelain --untracked-files=no` non-empty -> "uncommitted <path>"; when the repo has a remote, `git -C <wt> log --branches --not --remotes --oneline` non-empty -> "unpushed <path>". All ok -> verified_done, then close (todo 15). Otherwise not_done with reasons, lane message, event `not_done`. The deadline is the persisted `claim_at`, so restarts resume.
  Must NOT do: close on lane self-report; settle before 5 min; run signals more than once per settle.
  Closes: GAP-4, GAP-12
  Parallelization: Wave 3 | Blocked by: 5, 7, 12 | Blocks: 15
  References: upstream `done-protocol.md`, `scripts.md` (watch/close)
  Acceptance criteria: tests with fakes: claim then object -> not_done and the queued lane message contains the reasons; passing signals + clean tree -> verified_done after 5 min (MockClock); failing signal -> not_done with its detail; dirty tree -> "uncommitted"; a runner restart between claim and settle settles exactly once.
  QA scenarios: happy - live QA lane with `--done 'file done.txt'` creates the file and claims; 5 min later the row is verified_done (scratch DB) - `task-13.log`; failure - a QA lane that claims before creating the file -> not_done with the "file done.txt" detail, and `lane log` shows the agent receiving the "[LEAD] not done" message.
  Commit: Y | feat(lanes): done claim, objection and five-minute settle
  Recommended task executor category: deep-high - correctness argued from invariants across restarts.

- [x] 14. Deliver lane events as coalesced lead turns
  What to do: `src/lanes/dispatch.rs` `EventDispatcher`, tick 5 s: take undelivered events grouped by `lead_session_key`; deliver a group when its oldest event is at least 30 s old, or immediately when it contains `done_claimed`, `dead` or `kibitzer` - but never more than one delivered turn per lead session per 30 s (immediate kinds included; within the window they wait for the next slot) and never while that lead session has a turn in flight (keep queued until it ends); duplicate (lane_key, kind) pairs inside one group collapse to the latest body. Before delivering, each non-critical event (kinds other than done_claimed, dead, not_done, cron_problem with needs_owner, inbox_urgent) passes a jev `noul` gate "Does this event need the lead agent's judgment now?" (threshold 0.5, measured 0.92 vs 0.21 on a question vs milestone sample); events below threshold are marked delivered with `reason=jev_skip` and appear only in the digest. Build one `InboundEvent{ id: Uuid::new_v4(), session: SessionKey::from_storage_key(lead_session_key)?, platform_message_id: "lane-event:<first>-<last>", delivery_id: None, content: "[LANE_EVENT]\n" + one line per event "<key> <kind>: <body, max 600 chars>", attachments: vec![], received_at: now }` and call `SessionMultiplexer::route`; mark delivered only after Ok; on Err keep and log. Add `#[serde(default)] pub origin: EventOrigin` (`User` default, `System`) to `InboundEvent` (`src/models/events.rs:173`); for System the actor skips the 👀 reaction and owner-authorization checks and records the transcript line with a "[system]" marker in `record_inbound_transcript` (`src/multiplexer/actor.rs:923`). Update the struct-literal sites (`src/main.rs:455`, `src/multiplexer/actor.rs:134`, `src/storage/db.rs:565`) and `InboundEvent::message(...)` to set `origin: EventOrigin::User`.
  Must NOT do: deliver while draining (route refuses - keep undelivered).
  Closes: GAP-3, GAP-5, GAP-12
  Parallelization: Wave 3 | Blocked by: 12 | Blocks: 16, 17
  References: `src/multiplexer/router.rs:312-334`; `src/models/events.rs:173-200`; `src/main.rs:440-475` (synthetic resume event); `src/multiplexer/actor.rs:923`; `tests/test_multiplexer.rs` (fake runner pattern)
  Acceptance criteria: `tests/test_lane_dispatch.rs`: 3 events within 10 s -> one turn with 3 lines; `done_claimed` -> immediate turn when the session is idle and the last delivery was over 30 s ago; two `done_claimed` 5 s apart -> two turns at least 30 s apart; an event arriving during an in-flight turn waits until it ends; duplicate (lane, kind) collapse; route error -> events stay undelivered and are retried; a System event triggers no reaction call on the fake outbound.
  QA scenarios: happy - `cargo test --test test_lane_dispatch > task-14.log`; live - with todo 12's QA lane, the debug gateway log shows the lead turn for the lane-event session and a reply row in the scratch DB, logged; failure - create the drain marker used by `src/drain_control.rs`, enqueue an event, confirm it stays undelivered, remove the marker, confirm delivery.
  Commit: Y | feat(lanes): coalesced lane events wake the lead session
  Recommended task executor category: deep-high - changes the multiplexer/actor contract.

- [x] 15. Close lanes completely and reopen on owner reply
  What to do: `close(key)` requires verified_done (or `--force` with a "forced:" note): stop the lane task (cancel the running turn via `run_cancelable`'s token), thread `set_status(Done)` + `archive` (todo 10), remove lane-owned worktrees with `git worktree remove <path>` (no `--force`; on failure keep verified_done and enqueue `close_failed` with the git error), transition closed. Discord adapter: when an owner message arrives in a thread equal to a lane's `thread_channel_id` whose status is closed or verified_done, call reopen (todo 9) with that message as the first prompt, `set_status(Working)`, `unarchive`, and do not also route it to the chat session. Restart: the runner's start-up resumes working/not_done lanes, re-checks done_claimed deadlines and the dispatcher re-delivers undelivered events (assert by test).
  Must NOT do: delete branches; force-remove dirty worktrees.
  Closes: GAP-8, GAP-9, GAP-12
  Parallelization: Wave 3 | Blocked by: 7, 9, 10, 13 | Blocks: 36
  References: `src/discord/adapter.rs:860-1060` (inbound handler), `:1254-1282`; todos 9, 10, 13
  Acceptance criteria: tests: close refuses non-verified without force; dirty worktree -> `close_failed`, lane not closed; an inbound message in a closed lane's thread calls reopen exactly once and is not routed to the chat session (fake); restart test re-delivers an undelivered event.
  QA scenarios: happy - live QA lane end to end: verified_done -> closed; worktree gone from `git worktree list`; thread archived (GET channel via the ignored live helper) - `task-15.log`; failure - an uncommitted file in the lane worktree keeps the lane open (not_done "uncommitted", no close).
  Commit: Y | feat(lanes): full close and reopen-on-reply
  Recommended task executor category: deep-high.

- [x] 16. Add the kibitzer job that wakes the lead only when warranted
  What to do: `src/lanes/kibitzer.rs`, triggered by events, not by a timer: it is armed when a new `inbox_items` row (todo 22), a `lane_events` row of kind blocked/question/stale/not_done/cron_problem, or a new owner message is written; it fires at most once per 30 min while armed and only between 08:00 and 24:00 KST (a cheap in-process check, no model call), and does nothing when not armed. Returns immediately when `OMON_KIBITZER_ENABLED=false`. It builds a compact state document: open lanes (status, last_report), the owner home DM's last 30 messages (messages table), the last 10 lane_events, and `MemoryStore::search("owner preferences")` results; caps the document at 6000 characters (oldest messages dropped first); asks jev `noul` "Would the owner want the agent to proactively do or ask something right now?" on the document (measured 0.85 for a due tax task vs 0.07 for an idle owner); only when the score is >= 0.7 does it run ONE turn on `OMON_MODEL_SMALL` in session key `cron:kibitzer` (fresh thread each run) to write `{"wake":bool,"reason":string,"suggested":string,"target_lane":string|null}`; parses strictly (invalid JSON -> no wake, warn); when wake and fewer than 6 kibitzer events since 00:00 KST, enqueue a `kibitzer` lane_event for `OMON_LANE_DEFAULT_LEAD` (owner home DM session key).
  Must NOT do: message the owner directly; read mail/calendar/GitHub; change the model.
  Closes: GAP-6
  Parallelization: Wave 4 | Blocked by: 12, 14 | Blocks: 36
  References: `src/memory/store.rs:97`; todos 14 (event path), 22 (inbox items); archive 공식냥 09-28 11:10 "메모리를 살펴보고 사용자라면했을행동을 유추"
  Acceptance criteria: tests with a fake runner: no new events for 24 h (MockClock) -> zero turns; one new inbox item with a jev score >= 0.7 -> exactly one small-model turn; a score < 0.7 -> zero model turns; wake=true -> one event; the 7th wake of the day -> none; non-JSON reply -> none + warning; kill switch -> no turn; state document never exceeds 6000 chars.
  QA scenarios: happy - on the debug gateway insert one inbox item into the scratch DB and log the single kibitzer turn and its parsed decision (`task-16.log`); failure - leave the debug gateway idle for 10 min with nothing new: `gateway.log` shows zero kibitzer turns (logged).
  Commit: Y | feat(kibitzer): event-armed owner-intent check that wakes the lead
  Recommended task executor category: deep-low.

- [x] 17. Wire guard alerts and queued lanes into the lead
  What to do: the runner ticks `GuardWatcher` (todo 6) every 60 s; ALERT/CLEAR transitions enqueue a `guard` event for `OMON_LANE_DEFAULT_LEAD`; queued lanes are retried each minute with `can_launch` and started (event `launched`); `lane heavy` (todo 8) uses the slots.
  Must NOT do: hold on CPU load alone.
  Closes: GAP-7
  Parallelization: Wave 4 | Blocked by: 6, 7, 12, 14 | Blocks: 36
  References: upstream `scripts.md` guard / can-launch / heavy
  Acceptance criteria: tests: one event per transition, none for repeated samples; a queued lane starts when the fake guard turns Ok.
  QA scenarios: happy - with `OMON_GUARD_DISK_FREE_MIN_GB=100000`, `lane start qa-3 ...` exits 4 (queued); restart the debug gateway without it and within 2 min the lane is working (`task-17.log`); failure - two concurrent `lane heavy qa -- sleep 3` print start times at least 3 s apart.
  Commit: Y | feat(lanes): guard alerts and queued lane launch
  Recommended task executor category: deep-low.

- [x] 18. Post milestones and a live status message in lane threads
  What to do: milestone reports (todo 12) are posted to the lane thread, one message per milestone; one per-lane status message "⏳ <title> · <elapsed> · turn <n>" (id in `lanes.status_message_id`) is edited at each milestone and at most every 15 min; the thread prefix follows status (working 🔄, blocked/question ⏸, verified_done/closed ✅). If the status message was deleted (404), create a new one and store its id.
  Must NOT do: post more than one message per milestone; ping anyone.
  Closes: GAP-8, IS-2 progress
  Parallelization: Wave 4 | Blocked by: 8, 10, 12 | Blocks: 36
  References: OMOMEOW-MODE rules 11 and 14; `src/discord/adapter.rs:3104-3150`
  Acceptance criteria: tests with a recording sink: two milestones -> two posts and two status edits; blocked -> rename to ⏸; a 404 on edit -> new status message stored.
  QA scenarios: happy - live QA lane with two milestones -> two thread messages (ids logged) - `task-18.log`; failure - delete the status message via the API in the ignored live test, emit a milestone, a new status id is stored.
  Commit: Y | feat(lanes): milestone posts and live status message in lane threads
  Recommended task executor category: unspecified-low.

- [x] 19. Scrub gateway-posted lane text
  What to do: `src/lanes/scrub.rs` `scrub(text)`: `/Users/<name>/` and `$HOME` -> `~/`; secrets via the existing redaction helpers in `src/security/` (locate with `rg -n "redact" src/security`) -> `<redacted>`; hostnames from `OMON_SCRUB_HOSTS` (comma list, default empty). Apply to every lane text the gateway posts to Discord or puts into event bodies.
  Must NOT do: scrub owner-authored messages.
  Closes: GAP-9
  Parallelization: Wave 4 | Blocked by: 8, 12 | Blocks: 36
  References: `src/security/*.rs`; upstream `scripts.md` freshness ("home paths ... scrubbed")
  Acceptance criteria: unit tests per pattern, including Korean text around paths.
  QA scenarios: happy - `cargo test lanes::scrub > task-19.log`; failure - a report containing a fake token ("ghp_" + 36 chars) is recorded in the sink payload as `<redacted>`.
  Commit: Y | feat(lanes): scrub paths and secrets from lane posts
  Recommended task executor category: quick.

- [x] 20. Update the OmOMeow persona with lead, lane, event and onboarding rules
  What to do: the runtime reads the persona from the DB table `bot_profiles` (`src/multiplexer/actor.rs:825,872`, by `bot_id`), and NO code imports `bot_profiles/omomeow.json` (verified 2026-10-10: only `src/dashboard.rs:1777-1950` CRUD touches the table). The live row for the OmOMeow bot id `1465631383862120451` has name `wawabot` and a 1089-char prompt (DB copy), different from the repo JSON. So the owner's live prompt is never overwritten: (1) write the new rules as a block file `bot_profiles/blocks/lanes.md` wrapped by `<!-- omon:lanes:begin -->` / `<!-- omon:lanes:end -->`, and append the same block to `bot_profiles/omomeow.json` `system_prompt` for the repo copy; (2) add `omo-gateway profile apply-block --bot-id <id> --block-file <path>` (new subcommand in `src/entry.rs`) that, in one transaction, replaces the text between the markers in the DB row's `system_prompt` (appends when absent) and changes no other column; prints before/after `length(system_prompt)` and sha256; exits 2 when the bot id has no row; (3) back up first with `omo-gateway profile export <id>` (prints the row as JSON) to `.omo/evidence/dori-grade-agent-env/task-20-profile-before.json`. Block content: Routing (answer directly for questions, status, lookups, small edits; `omo-gateway lane start` for PR-bound, multi-step or long work; reuse an open lane for the same repo via `lane send`; on HOLD queue and tell the owner why); Lane brief template (ulw keywords, location, evidence, ideal end state, do-not-touch, report grammar, a checkable `Done =`); Events (`[LANE_EVENT]` messages come from the gateway, not the owner: answer the lane with `omo-gateway lane send`, object, or let it close, and tell the owner only what needs him); Done rule (never call work done before verified_done; quote the evidence); Kibitzer events (act or ask, never forward raw); Onboarding (on the owner's request: ask consent first in the dori wording, go tool by tool naming the integration and what it reads, read-only, memory as you go, report gaps, remember declined tools). Because `developerInstructions` bind at `thread/start` (`src/agent/omo_backend.rs:217-235`), state in the commit body that existing threads keep the old prompt until `/reset` or a new thread.
  Must NOT do: change model; add rules contradicting OMOMEOW-MODE 14; add tests pinning wording.
  Note: todos 25, 26, 28, 29 and 32 append their rules to the same `bot_profiles/blocks/lanes.md` block; todo 36 applies the final block once.
  Closes: GAP-10, GAP-11
  Parallelization: Wave 4 | Blocked by: - | Blocks: 36
  References: `bot_profiles/omomeow.json`; `src/dashboard.rs:1867,1950` (insert/update SQL); upstream `SKILL.md` (Routing, Inbound, Sessions) and `onboarding.md`; `~/.omon/omomeow/OMOMEOW-MODE.md`
  Acceptance criteria: `jq -e '.system_prompt | contains("[LANE_EVENT]") and contains("omo-gateway lane start")' bot_profiles/omomeow.json` exits 0 (machine-consumed sentinels: the event prefix the gateway emits and the CLI name); `tests/test_profile_import.rs`: apply-block on a row without markers appends once; applying twice is idempotent (same sha256); a changed block replaces only the marked region; name/model/toolsets columns are byte-identical before and after; unknown bot id exits 2.
  QA scenarios: happy - debug gateway on a scratch DB, apply the block to a copied bot row, new session via the dashboard chat endpoint (`POST /api/sessions/{id}/chat`, loopback) with "이 레포 테스트 실패 다 고쳐줘 (repo: <QA repo>)": the transcript contains a `lane start` tool call (`task-20.log`); failure - deliver a synthetic `[LANE_EVENT] qa-1 question: A or B?` to that session: the reply invokes `lane send` or asks the owner with a recommendation, not a verbatim forward (QA-by-read, logged).
  Commit: Y | feat(persona): lead routing, lane events, done rule and onboarding
  Recommended task executor category: deep-low - persona block authoring plus a small transactional CLI.

- [x] 21. Rewrite the operator guide for lanes
  What to do: turn `docs/AGENTS_DELEGATE.md` into the lane operator guide: how lanes run (app-server threads, no herdr), subcommands with examples, signal grammar, statuses, kill switches and env vars (`OMON_LANES_ENABLED`, `OMON_LANE_TURN_SECS`, `OMON_LANE_MAX_TURNS`, `OMON_LANES_MAX`, guard vars, `OMON_KIBITZER_ENABLED`), how to watch a lane (`lane log`, dashboard), troubleshooting (daemon down, turn errors, stuck claims, close_failed), rollback (`OMON_LANES_ENABLED=false` + kickstart).
  Must NOT do: touch `docs/research/*`.
  Closes: GAP-2 (operability)
  Parallelization: Wave 4 | Blocked by: 11 | Blocks: 36
  References: todos 5, 8, 13, 17
  Acceptance criteria: scripted check: every subcommand listed by `target/debug/omo-gateway lane --help` appears as "lane <name>" in the doc; `grep -ci herdr docs/AGENTS_DELEGATE.md` equals the count of lines that explicitly say herdr is not used (at most 1).
  QA scenarios: happy - check log `task-21.log`; failure - temporarily delete one section, the check fails, restore.
  Commit: Y | docs(lanes): operator guide
  Recommended task executor category: writing.

- [x] 22. Add the inbox item store and the source-collector framework
  What to do: `migrations/0033_inbox.sql` (or the next free number): `inbox_items(id TEXT PRIMARY KEY, source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage')), source_ref TEXT NOT NULL, thread_ref TEXT, author TEXT, title TEXT, snippet TEXT NOT NULL, received_at DATETIME NOT NULL, link TEXT, raw_hash TEXT NOT NULL, collected_at DATETIME NOT NULL, UNIQUE(source, source_ref))`; `inbox_decisions(item_id TEXT PRIMARY KEY REFERENCES inbox_items(id), class TEXT NOT NULL CHECK(class IN ('owner','bot','fyi','ignore')), reason TEXT NOT NULL, due_at DATETIME, proposed_action TEXT, lane_key TEXT, status TEXT NOT NULL CHECK(status IN ('open','proposed','approved','done','dismissed')), decided_at DATETIME NOT NULL, model TEXT)`; `inbox_cursors(source TEXT PRIMARY KEY, cursor TEXT NOT NULL, updated_at DATETIME NOT NULL)`. `src/inbox/mod.rs`: trait `InboxSource { fn name(); async fn fetch_since(cursor) -> (Vec<InboxItem>, new_cursor) }`; Collection is push-first and model-free: iMessage via a long-running `imsg watch --json --since-rowid <cursor>` child (file-system events on chat.db, no polling by us); mail via a long-running `zele mail watch --folder inbox --filter is:unread` child restarted after each hit (zele itself checks Gmail every 15 s by default - pass `--interval 60`; this is an HTTP check, no model); Discord via a hook where the gateway already stores inbound messages (no extra reads); KakaoTalk via the PirateTalk API: no event stream was found (2026-10-10: `/api/events` and `/api/stream` return the web app HTML), so it is a cheap HTTP cursor check every 2 min with no model call, replaced by a stream if PirateTalk exposes one. `collect_all()` (used by the watchers and once at start-up) runs each enabled source with a 60 s timeout, upserts items (dedupe by `(source, source_ref)`), advances the cursor only after the upsert commits; a failing source is logged and recorded in `inbox_cursors` error text without blocking the others. Snippet is capped at 1000 chars; full bodies are never stored. Before an item is stored it passes the T0 prefilter (`src/inbox/prefilter.rs`, pure rules, no model; rules in `~/.omon/inbox-filters.json` created with these defaults on first run): mail - drop Gmail categories promotions/social/updates (query `-category:promotions -category:social -category:updates`; measured 281 -> 62 threads/7 days on 2026-10-10), and route known notification senders (`github.com`, `discord.com`, `linkedin.com`) to class `fyi` directly unless the subject matches `failed|error|review requested|mentioned you`; KakaoTalk - keep only rooms of type `DirectChat` and `MultiChat` (2 rooms active in 24 h on 2026-10-10); `OM`/`OD`/`PlusChat` rooms only when a message mentions the owner's KakaoTalk name or a keyword in `kakao_keywords`; the 포모단 room (chatId 18491246151592761, ~5,900 messages/day) is excluded; iMessage/SMS - drop senders matching short codes (`^\d{2,6}$`, e.g. 114) and ad prefixes (`[Web발신]`, `(광고)`); Discord - only mentions of the owner (`414011632306618368`) or DMs to the bot from other people. Dropped items are counted per rule (`inbox_filter_stats(day, source, rule, count)`), never stored.
  Must NOT do: store full message bodies or attachments; write to any source.
  Closes: GAP-13
  Parallelization: Wave 4 | Blocked by: 2 | Blocks: 23-26
  References: `migrations/0030_work_items.sql` (style); `src/storage/db.rs:962-1110`; `src/cron/executor.rs:385` (native job pattern)
  Acceptance criteria: `tests/test_inbox_store.rs`: two collects of the same fake items insert once; prefilter fixtures: a promotions mail, an OM-room message without mention, a 114 SMS and the 포모단 room are dropped and counted; a DirectChat message, a GitHub 'review requested' mail and an owner mention pass; a failing fake source does not stop another; cursor not advanced when the upsert fails (fault-injected).
  QA scenarios: happy - `cargo test --test test_inbox_store > task-22.log`; failure - fake source returning an error leaves its cursor unchanged and the other source's items stored (asserted, logged).
  Commit: Y | feat(inbox): item store and source collector framework
  Recommended task executor category: deep-low - schema plus a small async framework.

- [x] 23. Implement the four read-only inbox sources
  What to do: in `src/inbox/sources/`: (a) `mail.rs` - `zele mail list --json` (verify the exact flag set with `zele mail list --help` first; the account is `freedomzero91@gmail.com`, google, gmail+calendar per `zele whoami`), query unread + inbox since cursor (Gmail `after:` epoch), `source_ref` = thread id, link `https://mail.google.com/mail/u/0/#all/<threadId>`; never call `mail read` (it does not mark read, but bodies are not needed) - snippet comes from the list output; when `zele whoami` shows the token expired and refresh fails, the source reports `auth_expired` (surfaced to the owner once a day, see todo 25). (b) `kakao.rs` - PirateTalk HTTP API at `OMON_PIRATETALK_URL` (default `https://indo.taila0cef2.ts.net`), `GET /api/rooms` then `GET /api/rooms/:id/messages?before_id=` paging back to the cursor per room (same API and paging as `~/code/project/scripts/kakao-archive.ts`); rooms filtered by `OMON_INBOX_KAKAO_ROOMS` (comma list; default: all rooms with unread > 0 or DM rooms - confirm the field names from one `/api/rooms` response saved to the evidence dir). (c) `imessage.rs` - `imsg chats --limit 50 --json` + `imsg history --chat-id <id> --limit 50 --json` for chats with `last_message_at` > cursor, skipping messages from the owner (`is_from_me`). (d) `discord.rs` - the gateway's own `messages` / `message_search_documents` tables: messages in guild channels/threads that mention the owner (`<@414011632306618368>`) or reply to him, plus DMs to the bot from people other than the owner; no Discord API calls. Every source passes argv to `tokio::process::Command` (no shell) and parses JSON strictly.
  Must NOT do: katok; `zele mail send|reply|forward|archive|read-mark|trash`; `imsg send|read|react`; any PirateTalk write endpoint; KakaoTalk protocol tricks.
  Closes: GAP-13
  Parallelization: Wave 4 | Blocked by: 22 | Blocks: 24, 26
  References: `zele --help` (mail list/search/read, whoami); `imsg --help` (chats, history, watch; `--json`); memory `notes/facts/2026-10-kakao-archive.md` (PirateTalk endpoint, paging, katok ban); `~/code/project/scripts/kakao-archive.ts`; owner Discord id 414011632306618368 (from `~/.omon/omomeow/inbound.log` author_id); `src/storage/message_search.rs`
  Acceptance criteria: unit tests parse recorded fixtures (one real response per source saved under `tests/fixtures/inbox/` with personal text replaced by placeholders); each source's argv is asserted to contain no write subcommand (test enumerates forbidden words: send, reply, forward, archive, read-mark, trash, react, read for imsg).
  QA scenarios: happy - `#[ignore]` live test per source fetching once and printing counts only (no content) to `task-23.log`; failure - `OMON_PIRATETALK_URL=http://127.0.0.1:9` makes the kakao source fail with a clear error while the other three still return.
  Commit: Y | feat(inbox): read-only mail, kakao, imessage and discord sources
  Recommended task executor category: deep-low - four adapters with strict read-only contracts.

- [x] 24. Classify inbox items as owner task, bot task, FYI or ignore
  What to do: `src/inbox/triage.rs`: triggered when the collector (todo 22/23) inserts new items, never on a timer and never when nothing is new (kill switch `OMON_INBOX_ENABLED`); new items are batched for up to 2 min (or 20 items) so a burst costs one turn; classify each new item with jev `choice` (owner_must_act / bot_can_handle / fyi / ignore) on `"<source> from <author>: <title> - <snippet max 500 chars>"` with the fixed instruction measured 8/8 on 2026-10-10 samples ("owner_must_act = needs the owner's own decision, signature, payment or a personal reply; bot_can_handle = a coding/ops task an agent can do alone without speaking as the owner; fyi = information only; ignore = ads/promotions"); `due_at` by a regex/date parser (no model); only `owner` and `bot` items then get ONE batched `OMON_MODEL_SMALL` turn (session `cron:inbox-triage`, fresh thread) to write `reason` and `proposed_action` as strict JSON `[{"id","reason","proposed_action"|null}]`; fyi/ignore never reach a chat model; validate strictly (unknown id -> left for the next pass, warn); rules in the prompt: owner = needs his decision, money, signature, a reply only he can give, a meeting; bot = can be done with the bot's own tools without speaking to other people as him (look something up, prepare a draft, open a lane on a repo, file an issue); fyi = information; ignore = ads/notifications. Owner-class items with `due_at` within 24 h are enqueued as a `lane_event` kind `inbox_urgent` for `OMON_LANE_DEFAULT_LEAD` (rate-limited by todo 14).
  Must NOT do: act on items here; send anything; exceed 6000 chars of item text per turn.
  Closes: GAP-14
  Parallelization: Wave 4 | Blocked by: 22, 23, 14 | Blocks: 25, 26
  References: todo 16 (same cron + strict JSON pattern); `src/memory/store.rs:97`
  Acceptance criteria: tests with a fake runner: a collector pass with zero new items -> zero turns; 25 new items within 2 min -> 25 jev calls and at most two small-model turns (only owner/bot items); all-ignore batch -> zero chat-model turns; valid JSON -> decisions stored; an unknown id -> ignored + warning; malformed JSON -> nothing stored; an urgent owner item -> one `inbox_urgent` event; kill switch -> no turn.
  QA scenarios: happy - on the debug gateway with the live sources, the first collect inserts items and triggers one triage turn; log counts per class (no content) to `task-24.log`; failure - a second collect with nothing new triggers no turn (gateway.log shows none).
  Commit: Y | feat(inbox): owner/bot/fyi triage
  Recommended task executor category: deep-low.

- [x] 25. Give the owner the task list on request and a daily digest
  What to do: `omo-gateway inbox list [--class owner|bot|fyi] [--status open] [--json]` and `omo-gateway inbox set <item-id> --status done|dismissed|approved`; persona rule (in the todo-20 block, appended by this todo): when the owner asks for his tasks ("할일", "정리해줘", "뭐 해야돼"), run `inbox list --class owner --status open` and `--class bot` and answer as two short lists (source icon word, who, what, due, link), most urgent first; when he says an item is done or not his, call `inbox set`. Daily digest: native cron `omon:inbox-digest` at `0 9 * * *` Asia/Seoul posts the same two lists to the owner home DM through the normal outbound path (one message; empty lists -> no message); source auth problems (todo 23) are listed at the end once per day.
  Must NOT do: include full message bodies; post in shared channels.
  Closes: GAP-14
  Parallelization: Wave 4 | Blocked by: 24 | Blocks: 31, 32, 36
  References: todo 20 (persona block mechanism); `src/cron/scheduler.rs` delivery to `origin`/home channel
  Acceptance criteria: CLI tests on a seeded DB: list filters and ordering (due first, then received); set transitions; digest builder returns None for empty lists.
  QA scenarios: happy - `inbox list --class owner --json` on the debug gateway after todo 24's run (counts only in `task-25.log`); failure - `inbox set nope --status done` exits 2.
  Commit: Y | feat(inbox): task list command and morning digest
  Recommended task executor category: unspecified-low.

- [x] 26. Turn bot-class items into proposals the owner approves with one reply
  What to do: for `bot` items, the triage turn's `proposed_action` is shown in the list as a numbered proposal ("#3 메일: 세금계산서 요청 -> 초안 작성"); the owner approves with "3 해" (persona rule) -> `inbox set <id> --status approved`, then the lead runs it: a lane (`lane start`, todo 8) for repo work, or a direct bot action for lookups/drafts; drafts addressed to other people are posted to the owner for copy/send, never sent. Low-risk internal actions (look-up only, no external side effect) may run without approval when `OMON_INBOX_AUTO_LOOKUP=true` (default false). The lane key and outcome are stored in `inbox_decisions`.
  Must NOT do: send, reply or react to anyone on the owner's behalf.
  Closes: GAP-14
  Parallelization: Wave 4 | Blocked by: 8, 24 | Blocks: 36
  References: todos 8, 20, 25
  Acceptance criteria: tests: approve -> status approved and (fake) lane start called with the item's link in the brief; without approval nothing runs; `OMON_INBOX_AUTO_LOOKUP=false` blocks the auto path.
  QA scenarios: happy - seeded bot item approved on the debug gateway produces a lane row (`task-26.log`); failure - an unapproved item produces no lane after a triage cycle.
  Commit: Y | feat(inbox): approve-to-run proposals for bot-class items
  Recommended task executor category: deep-low.

- [x] 27. Build the cross-machine OMO session registry
  What to do: `src/sessions/registry.rs`: machines = `local` + every host in `OMON_SESSION_HOSTS` (comma list; default empty) - seed the default list during this todo by probing `~/.ssh/config` hosts once with `ssh -o BatchMode=yes -o ConnectTimeout=5 <host> 'command -v omo'` and writing the hosts that answer to `~/.omon/session-hosts.json` (2026-10-10 probe: `omarchy` has omo; noveling and oracle-vps reachable without omo; Windows hosts maho-win/maho-scalper/maho-steam answer PowerShell; vuev denied; maho-mac and xmr-miner timed out). `list_all()` runs, in parallel with a 20 s timeout per machine: local `omo thread list --all-scope --json`; remote `ssh -o BatchMode=yes -o ConnectTimeout=5 <host> omo thread list --all-scope --json`; parses the JSON (fields used: `sessionId`, `name`, `cwd`, `status`, `alive`, `updated_at`, `surface`, `controls`) and returns `[{machine, session_id, name, cwd, status, updated_at, controls}]` plus per-machine errors (unreachable, no omo, exit code). Exit codes follow the documented contract (0 done, 1 refused, 2 usage, 3 nothing answering, 4 unsupported, 5 internal). Cache for 15 s.
  Must NOT do: install anything on remote hosts; add keys; use a shell string for the remote command (pass `omo thread list --all-scope --json` as separate ssh argv words).
  Closes: GAP-15
  Parallelization: Wave 4 | Blocked by: 2 | Blocks: 28, 29
  References: `omo thread` usage (`omo thread <list|send|read|...> [--json]`); omo-ai CHANGELOG #9143/#9660 (exit codes, 24 h retention for non-running targets, controls send/read/rename/set_model/set_reasoning/interrupt); 2026-10-10 probe (17 live local tui sessions, 1 on omarchy)
  Acceptance criteria: tests with a fake command runner: local + one remote merged; a timed-out host appears in errors while others return; exit code 3 maps to "nothing answering".
  QA scenarios: happy - `omo-gateway sessions list --json` on this Mac lists >= 1 local session and the omarchy session (`task-27.json`, names and cwd only); failure - add an unreachable host to `OMON_SESSION_HOSTS`, the call still returns within 25 s with that host in `errors`.
  Commit: Y | feat(sessions): cross-machine omo session registry
  Recommended task executor category: deep-low.

- [x] 28. Read, message, interrupt and take over sessions on any machine
  What to do: `omo-gateway sessions <list|read|send|interrupt|adopt|release> ...` built on todo 27: `read <machine>:<session> [--limit N]` -> `omo thread read <session> --limit N --json` (local or over ssh argv); `send <machine>:<session> --text T [--mode auto|steer|follow_up]` -> `omo thread send <session> <text> --mode <m> --idempotency-key <uuid> --json` (the text is one argv element; over ssh the remote command is passed as argv words with the text escaped by the ssh client only - refuse text containing a newline over ssh and send it via a temp file `--text-file` on the remote only if `omo thread send` supports reading stdin; verify on omarchy first and record); `interrupt` -> the thread interrupt control (`omo thread` subcommand or the `interrupt` control listed in `controls`; record which); `adopt <machine>:<session> --goal "..." --done "..."` registers a `lanes` row of kind `external` (new nullable column `external_target` in the lanes table via the next migration) that the runner (todo 8) drives by `send`/`read` instead of app-server turns: it sends the goal + done footer once, then follows the session through `omo thread` event delivery (the thread SDK exposes follow/subscribe; use `--json` reads with `--cursor` only when woken by the session's completion/report event, verified on omarchy first) - a cheap `omo thread read --cursor` every 5 min is the fallback only when no event surface works, with no model call unless new output arrived; applies todo 12's report parsing to new output, and todo 13's done flow; `release` stops driving it. Persona rule (append to the todo-20 block): on "세션들 상태"/"뭐 돌아가" run `sessions list` and answer per machine; on "<세션>에 X 보내줘" use `sessions send`; on "그거 마무리해" use `sessions adopt` with a done signal the owner agrees to (ask once if none is obvious).
  Must NOT do: send while the owner is typing in that terminal (the engine already holds such messages - do not retry in a loop); kill processes; send to sessions whose `controls` lack `send`.
  Closes: GAP-15, GAP-16
  Parallelization: Wave 4 | Blocked by: 8, 12, 13, 27 | Blocks: 29, 36
  References: `omo thread` usage text (send `<target> <text> [--mode ...] [--idempotency-key]`, read `<target> [--limit] [--max-bytes] [--cursor]`); CHANGELOG #9662/#9664 (interrupt, set_model, sender line shown in the receiving terminal); todos 8, 12, 13
  Acceptance criteria: tests with a fake runner: send builds the exact argv (local and ssh forms) with an idempotency key; exit 3 surfaces "not running - kept 24 h"; an adopted external lane goes working -> done_claimed -> verified_done using fake reads.
  QA scenarios: happy - start a scratch interactive session on this Mac that is not one of the owner's: in a `mktemp -d` dir run `script -q /dev/null omo --session-id <new uuid>` as a background process owned by the QA (never send to the owner's existing sessions), then `sessions send local:<uuid> --text "reply with the word pong"` and `sessions read` shows "pong" (`task-28.log`); cleanup kills only that QA process; failure - `sessions send local:<random-uuid> --text hi` reports not running (exit 3 mapping).
  Commit: Y | feat(sessions): read, send, interrupt and adopt sessions across machines
  Recommended task executor category: deep-high - remote control with quoting, idempotency and the done flow.

- [x] 29. Let the owner ask for session status in chat and get it in 30 seconds
  Owner scope change 2026-10-12 (29): the persona-only short path measured 37.6 s and 45.4 s (engine forces todo init; model ignores the compact format), so the owner approved a gateway direct reply: when an owner-DM/owner-thread message matches the session-status intent, the gateway answers from `sessions list` (15 s cache) without a model turn. This supersedes "No new Rust beyond todos 27-28". Acceptance/QA otherwise unchanged (reply within 30 s, never in shared channels).
  What to do: persona block rule (todo 20 mechanism) + a short-path: when the owner's message matches the session-status intent, the lead calls `sessions list` (cached 15 s) and replies with one compact message per machine: name, folder (home-relative), status, last activity (KST relative), and errors per unreachable host in one line. Busy sessions show the last `[REPORT]` or the last assistant line from `sessions read --limit 1`. No new Rust beyond what todos 27-28 add; this todo is the persona rule plus an end-to-end check.
  Must NOT do: list sessions in shared channels (owner DM / owner threads only).
  Closes: GAP-15
  Parallelization: Wave 4 | Blocked by: 20, 27, 28 | Blocks: 36
  References: todos 20, 27, 28
  Acceptance criteria: persona block contains the sentinel `omo-gateway sessions list` (jq check like todo 20).
  QA scenarios: happy - debug gateway, owner-scoped scratch session, prompt "지금 돌아가는 세션들 상태 알려줘": the transcript shows a `sessions list` call and the reply arrives within 30 s of the prompt (timestamps logged, `task-29.log`); failure - same prompt in a shared-channel session key yields no session list (rule check, logged).
  Commit: Y | feat(persona): session status on request
  Recommended task executor category: unspecified-low.

- [x] 30. Detect cron problems at the moment a run result is written (event-driven, no polling, no LLM)
  What to do: `src/cron/health.rs` exposes `on_run_finished(pool, job, run_id, outcome)` and is called synchronously, after the transaction commits, from the exact places a run reaches a terminal state: `complete_success` (`src/cron/scheduler.rs:1316-1330`), `complete_failure` (`:1458-1475`), the delivery step that writes `last_delivery_error` (`:1505-1535`), and the stale-lease reclaim that marks a run `failed` with "lease expired before completion" (`:1080-1095`, `:3056`). It is pure Rust + SQL: no model call, no timer, no extra task. It classifies: (a) `failed` - outcome failed (the existing incident row + one alert stay unchanged); (b) `stuck` - the reclaim path fired; (c) `delivery_failed` - success but `last_delivery_error` non-null; (d) `empty_success` - only for jobs whose payload has the new optional `"expect": {"output_regex": "..."}`, checked against the run's `cron_outputs.output`; (e) `missed` - detected where the scheduler already computes missed occurrences for catch-up (`newest_missed_occurrence`, `:1132-1160`, `:2129`) and once at gateway start-up for each recurring job whose newest expected occurrence before now has no run row (one start-up pass, not a loop). Problems go into a new table `cron_problems(id INTEGER PRIMARY KEY AUTOINCREMENT, job_id TEXT NOT NULL, kind TEXT NOT NULL, run_id TEXT, signature TEXT NOT NULL, detail TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('open','remediating','fixed','needs_owner','ignored')), lane_key TEXT, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL, UNIQUE(job_id, kind, signature))` (next free migration number). A success with no delivery error and a passing `expect` marks that job's open problems `fixed` in the same hook. A new problem enqueues a `lane_event` kind `cron_problem`, which wakes the remediation step (todo 31) through the same event path as lanes - nothing polls `cron_problems`. Kill switch `OMON_CRON_WATCH_ENABLED` (default true) makes the hook a no-op.
  Must NOT do: add a periodic checker or timer; call a model in the hook; change the existing failure alert or incident semantics; let a hook error fail the cron run (log and continue).
  Closes: GAP-17
  Parallelization: Wave 4c | Blocked by: 4, 8 | Blocks: 31, 32
  References: `src/cron/scheduler.rs:1033-1095` (run_due_jobs, claim_job, stale lease reclaim), `:1316-1330` (complete_success), `:1458-1535` (complete_failure, last_status/last_delivery_error), `:1720-1790` (incidents + alert), `:1132-1160`, `:2129` (missed occurrences), `:3056`; `migrations/0004_cron_leases.sql`, `0021_cron_outputs.sql`, `0026_cron_incidents.sql`
  Acceptance criteria: `tests/test_cron_health.rs` drives the real `CronScheduler` with a fake executor on an in-memory DB: a failing run creates exactly one `failed` problem in the same call (no sleep, no tick); a success with a delivery error creates `delivery_failed`; an expired lease reclaim creates `stuck`; a job with `expect` and a non-matching output creates `empty_success`, without `expect` never; a start-up pass after simulated downtime creates one `missed`; a later clean success marks problems `fixed`; with the kill switch off no rows are written; a hook DB error does not change the run's recorded status. A test asserts no new `tokio::time::interval`/sleep loop exists in `src/cron/health.rs` (grep in test).
  QA scenarios: happy - debug gateway on a scratch DB with a QA job `qa-cron-fail` (temp script that exits 1): within the same second as the run's `completed_at`, a `cron_problems` row and a `cron_problem` lane_event exist (timestamps logged to `task-30.log`); failure - a QA job that succeeds produces no problem row and `gateway.log` shows no model call for it.
  Commit: Y | feat(cron): event-driven health hook on run completion
  Recommended task executor category: deep-low - hooks at existing commit points with clear rules.

- [x] 31. Remediate cron problems with a lane and a verified single re-run
  What to do: when a `cron_problem` event arrives for a new `open` problem, jev `choice` (transient_retry_once / script_bug_needs_fix / needs_owner_login_or_quota) on the error + last output excerpt decides the path (measured correct on a traceback and an OAuth 401 sample): transient -> one verified re-run with no lane and no chat model (never for publishing jobs); needs_owner -> owner task item (after the mahoquot quota check when the text mentions quota/429), no lane; script_bug -> the runner opens a remediation lane (model `auto` -> jev size decision) (todo 8) with key `cron-fix-<job-id-slug>-<4hex>`, cwd = the job's `workdir` if set, else the directory of its `script`, else `/Users/indo/code/project/omon-gateway`; brief generated from a template containing: job id/name/expression, problem kind and detail, the last 3 `cron_runs` rows (status, error), the last `cron_outputs.output` (max 4000 chars), the job's `script` path or prompt, the matching `~/.omon/gateway.log` lines (`rg` for the run_id and job id, last 80 lines, UTC note), the standing rules (no .env edits, no model change, no job disable/delete, quota claims verified at mahoquot `127.0.0.1:18801`, katok forbidden), and the instruction "diagnose, fix the root cause in the script/config/repo, then claim done"; `Done =` signals: `command ["omo-gateway","cron","run-now","<job-id>","--wait","--json"] stdout~"\"status\":\"succeeded\""` (new CLI wrapper over the existing native run-now path in `src/tools/cron.rs`, which waits for the run and prints its row) plus, for jobs with `expect.output_regex`, the same regex on the printed output, plus for delivery problems `"last_delivery_error":null`. Exactly one verification re-run per problem comes from that signal; if the signal fails the lane gets "not done" (todo 13) and may fix again, but the settle step will not re-run more than 3 times per problem (counter in `cron_problems.detail` JSON) before marking `needs_owner`. Jobs listed in the Must-NOT-have publishing set (or with `"external_effect": true`) get a lane that diagnoses and prepares the fix but whose `Done =` is `file .omo/cron-fix/<job>.ready` and whose re-run waits for the owner's OK (owner task item via todo 25). Problems whose diagnosis needs the owner (expired login, payment, provider quota after mahoquot check) are set `needs_owner` and added to the owner task list (`inbox_decisions` class owner, source `cron`) with the exact action needed.
  Trigger: the `cron_problem` lane_event from todo 30 (no polling of `cron_problems`).
  Must NOT do: re-run publishing jobs without the owner; more than 3 re-runs per problem; edit `.env`; change models; disable/delete jobs; use the dashboard trigger API (it drops Discord delivery, memory rule).
  Closes: GAP-18
  Parallelization: Wave 4c | Blocked by: 8, 13, 25, 30 | Blocks: 32, 34
  References: `src/tools/cron.rs` (run-now path, incidents ack at `:730-750`); `src/cron/executor.rs:385, 816` (`execute_native_cron`, `run_cron_script`); todos 8, 13, 25; memory rules on cron retries, destination verification and mahoquot quota checks
  Acceptance criteria: `tests/test_cron_remediation.rs` with fakes: a failed non-publishing job -> one lane whose brief contains the error and log excerpt and whose Done line contains the run-now command; a publishing job -> lane with the `.ready` file signal and an owner task item, no run-now in Done; a 4th settle attempt -> `needs_owner`; a problem fixed by a natural later success closes its lane as verified_done without a re-run.
  QA scenarios: happy - on the debug gateway with a scratch DB, seed a QA cron job `qa-cron-fail` whose script is `["bash","-c","exit 1"]`-style via a temp script file that fails until a marker file exists; the health check opens a lane, the lane creates the marker and claims, the settle re-runs once and the problem becomes fixed (`task-31.log`); failure - seed a publishing-flagged QA job: no run-now happens (cron_runs count unchanged) and an owner task item exists. Cleanup: delete the QA jobs from the scratch DB, stop the debug gateway, remove temp files; receipt in the log.
  Commit: Y | feat(cron): remediation lanes with one verified re-run
  Recommended task executor category: deep-high - automated repair loop with safety limits across cron, lanes and the done flow.

- [x] 32. Report cron health to the owner and on request
  What to do: `omo-gateway cron health [--json]` prints per enabled job: last status, last success time (KST), problems open/remediating/needs_owner with lane key; persona rule (todo-20 block): on "크론 상태"/"크론 다 잘 돌아?" run it and answer in one message (OK jobs collapsed to a count, problems listed). One message per remediation outcome to the owner home DM: "<job>: <cause> -> <fix> (re-run <run_id> succeeded, evidence: <output or delivery check>)" or "<job>: needs you - <action>". The 09:00 digest (todo 25) gains a "cron" section listing problems from the last 24 h. Existing per-failure "Cron job X failed" alerts stay as they are.
  Must NOT do: post routine "all good" messages; duplicate the existing failure alert.
  Closes: GAP-17, GAP-18
  Parallelization: Wave 4c | Blocked by: 25, 30, 31 | Blocks: 34
  References: todos 25, 30, 31; `src/cron/scheduler.rs:1750-1775` (existing failure alert)
  Acceptance criteria: CLI test on a seeded DB: OK jobs collapsed, problems listed with lane keys; outcome message builder includes run_id and evidence; persona block contains the sentinel `omo-gateway cron health`.
  QA scenarios: happy - `omo-gateway cron health --json` against the debug gateway after todo 31's QA (`task-32.log`); failure - with zero problems the digest builder returns no cron section.
  Commit: Y | feat(cron): health report command, outcome messages and digest section
  Recommended task executor category: unspecified-low.

- [x] 33. Add the jev decision layer used by every routing and yes/no decision
  What to do: `src/judge/mod.rs`: trait `Judge { async fn choice(&self, purpose: Purpose, state: &str, instructions: &str, options: &[&str]) -> Result<Choice>; async fn noul(&self, purpose: Purpose, state: &str, instructions: &str) -> Result<f64>; async fn diff(&self, purpose: Purpose, prompt: &str, repo: &Path, revision: Option<&str>) -> Result<DiffVerdict{fulfills, clean, risk}> }`. `JevCli` runs `typesafe-judge` (argv, `-q`, `--file <tmp>` for state longer than 500 chars - never a shell string, never raw multiline in `--state`), timeout 5 s (`diff` 15 s), env from the gateway process (TYPESAFE_API_KEY/URL/MODEL come from `~/.zshenv`; the launchd job must have them - add them to the gateway's launchd plist EnvironmentVariables only if missing, never into `.env`); state truncated to 4000 chars and passed through todo 19's `scrub` first (jev's endpoint is external). `Fallback`: on timeout/error/unparseable output, the same question goes to ONE `OMON_MODEL_SMALL` turn (session `judge:fallback`, fresh thread) with a strict JSON answer; never to the large model. Every call writes `judge_calls(id, purpose, kind, ms, outcome, used_fallback, created_at)`. Purposes (enum, one per call site): inbox_class, event_wake, kibitzer_wake, lane_size, report_intent, cron_cause, owner_intent, done_diff. Kill switch `OMON_JEV_ENABLED` (default true; false = fallback path always). Cost guards: daily caps `OMON_JEV_DAILY_CAP` (default 300 calls, KST day) and `OMON_JUDGE_FALLBACK_DAILY_CAP` (default 50 flash turns); over the jev cap, inbox items are stored as `unclassified` (shown as a count in the 09:00 digest) and other purposes take their conservative branch without a model call; over the fallback cap, the same conservative branch. Circuit breaker: 5 consecutive jev failures with HTTP 401/429 or timeout open the breaker for 30 min (fallback path, still capped), and one owner DM line is sent once per day ("jev 응답 실패: <reason>, flash 대체 중").
  Must NOT do: send full message bodies, secrets or file contents beyond the scrubbed excerpt; call the large model; let a judge failure block the caller (callers get a typed error and choose the conservative branch: wake the lead / classify owner / treat as large).
  Closes: GAP-19
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 8, 12, 14, 16, 24, 31, 34, 35
  References: `typesafe-judge --help` (choice `-s/-f/-i/-o/-q`, noul `-t` threshold, score `-l`, diff `--prompt --revision --staged --fulfills-threshold 0.65 --clean-threshold 0.5 --max-risk 1.5`); skill `~/.omo/agent/skills/typesafe-ai/SKILL.md` (multiline input via `--file`); 2026-10-10 measurements: 20/20 correct on inbox, lane size, wake, kibitzer, cron cause, report intent and owner intent samples, 0.48-0.71 s per call (first call 2.2 s), `diff` 1.1 s
  Acceptance criteria: `tests/test_judge.rs` with a fake `typesafe-judge` script on PATH: the 301st call of a day returns the capped outcome without spawning the CLI; 5 fake 429s open the breaker and the 6th call does not spawn the CLI; one owner alert per day; choice/noul/diff parse the `-q` outputs (`owner_must_act`, `0.920`, `fulfills=0.01 clean=0.02 risk=1.57`); state > 500 chars goes via `--file`; a 6 s fake triggers the timeout and the fallback path once; scrub applied (a fake token never appears in the argv or file); `judge_calls` rows written with purpose and `used_fallback`.
  QA scenarios: happy - `#[ignore]` live test running the 12 measured samples through `JevCli` and asserting the same answers (`task-33.log` with ms per call); failure - `TYPESAFE_URL=http://127.0.0.1:9` -> every call falls back, the fallback count equals the call count, no large-model call in the log.
  Commit: Y | feat(judge): jev decision layer with scrubbed input, timeout and small-model fallback
  Recommended task executor category: deep-low - typed CLI adapter with fallback and accounting.

- [x] 34. Route models by purpose: Opus for the lead and big work, DeepSeek flash for small work
  What to do: `src/agent/omo_config.rs`: compiled-in defaults `OMON_MODEL_LEAD = "mahoquot/claude-opus-5-5"`, `OMON_MODEL_LARGE = "mahoquot/claude-opus-5-5"`, `OMON_MODEL_SMALL = "inferhub/cb/deepseek-v4.1-flash"` (env overrides allowed, no `.env` edits). Use sites: the owner-facing lead sessions (Discord DM/owner threads/home channel for the OmOMeow bot id) take `OMON_MODEL_LEAD` when the session has no explicit `/model` override (`session.state.active_model` stays the override mechanism); other shared-channel sessions keep today's resolved default; lanes take the model chosen in todo 8; kibitzer, inbox writing, digests, judge fallback and transient cron checks take `OMON_MODEL_SMALL`; cron jobs keep their own `model` field (never changed). The existing fallback model (`DEFAULT_FALLBACK_MODEL`) stays as the outage fallback. The lead can delegate small sub-tasks with `omo-gateway lane start --model small`; the persona block (todo 20) states the rule "answer and plan yourself; send small, obvious work to a small lane; never run long work in your own turn".
  Must NOT do: change cron job models; edit `~/.omo/*.json`; change the bot_profiles `model` column for other bots.
  Closes: GAP-19
  Parallelization: Wave 1 | Blocked by: 2 | Blocks: 8, 16, 24, 35, 36
  References: `src/agent/omo_config.rs:55-68, 207-219`; `src/agent/omo_backend.rs:241-247, 637-662` (per-turn model via active_model / turn/start); `src/multiplexer/actor.rs:825-872` (bot profile model load); launchd env `OMON_DEFAULT_MODEL=mahoquot/nekos-claude-sonnet-5-5` (2026-10-10)
  Acceptance criteria: tests: an owner DM session's `turn/start` carries `mahoquot/claude-opus-5-5`; a `/model` override wins; a shared-channel session keeps the old default; a small lane carries `inferhub/cb/deepseek-v4.1-flash`; a cron session keeps its payload model.
  QA scenarios: happy - debug gateway: one owner-DM prompt and one `lane start --model small` lane; `gateway.log` shows the two model ids on their `turn/start` (`task-34.log`); failure - `OMON_MODEL_SMALL` set to an unknown id -> the turn falls back to `DEFAULT_FALLBACK_MODEL` with a logged warning, never to Opus.
  Commit: Y | feat(models): purpose-based model routing (lead/large Opus, small DeepSeek flash)
  Recommended task executor category: deep-low - config plus a few call-site switches.

- [x] 35. Account token and judge usage per tier and add jev gates on done claims
  Owner scope change 2026-10-12: token accounting (part a: model_usage token rows, usage CLI token totals, digest token line) is REMOVED by owner decision ("토큰 사용량을 왜집계함"; installed senpi app-server does not emit thread/tokenUsage/updated). Remaining scope: part (b) jev done_diff gate on done claims and judge_calls counts. Acceptance/QA below apply only to part (b).
  What to do: (a) usage: record per turn `model`, `purpose` (lead, lane_large, lane_small, kibitzer, inbox_write, digest, judge_fallback, cron_fix), input/output/cached tokens from the app-server `thread/tokenUsage/updated` notification (method seen in the engine bundle on 2026-10-10) into `model_usage(id, session_key, purpose, model, input_tokens, output_tokens, cached_tokens, created_at)`; `omo-gateway usage [--since 24h] [--json]` prints totals per tier plus `judge_calls` counts and fallback rate; the 09:00 digest gets one line "어제: Opus N턴/xxK토큰, flash N턴/xxK, jev N회(폴백 n, 상한 도달 여부), 인박스 수집 N건 중 필터 통과 M건" (from `inbox_filter_stats` and `judge_calls`); (b) done gate: in todo 13's settle step, after all signals pass and when the lane cwd is a git repo, run `jev diff --prompt "<lane title + brief goal, max 1500 chars>" --revision <lane start commit>`; `fulfills < 0.65` or `risk > 1.5` turns the claim into not_done with the verdict numbers as the reason (one extra lane turn instead of a wrong close); verdicts stored in `lane_history`.
  Must NOT do: block a close when jev is unavailable (log `done_diff skipped` and continue with signals only); count tokens by estimation when the notification exists.
  Closes: GAP-19
  Parallelization: Wave 4 | Blocked by: 13, 33, 34 | Blocks: 36
  References: todo 13 settle; `typesafe-judge diff` (measured 1.1 s, `fulfills=0.01 clean=0.02 risk=1.57` on an unrelated diff); engine method list includes `thread/tokenUsage/updated`; todo 25 digest
  Acceptance criteria: tests: token notifications aggregate per purpose; `usage --json` totals match seeded rows; a low `fulfills` verdict from a fake judge reopens the claim with the numbers; judge unavailable -> close proceeds on signals and logs the skip.
  QA scenarios: happy - after todo 8's QA lane closes, `omo-gateway usage --since 1h --json` shows the lane's tokens under lane_small or lane_large and the judge calls (`task-35.log`); failure - a QA lane whose diff is unrelated to its brief (it only edits a README) gets not_done with a jev reason.
  Commit: Y | feat(usage): per-tier token accounting and jev diff gate on done claims
  Recommended task executor category: deep-low.

- [x] 36. Release build, sign, deploy and run an end-to-end lane on the live gateway
  What to do: all gates vs the todo-2 baseline (no new failures); `cargo build --release`; Developer ID codesign + verify; export the bot row and run `omo-gateway profile apply-block --bot-id 1465631383862120451 --block-file bot_profiles/blocks/lanes.md`; `launchctl kickstart -k gui/501/ai.omon.gateway`; re-check todo 1's acceptance; run a real lane for the owner home DM: `omo-gateway lane start e2e-1 --title "E2E lane" --brief <brief: create done.txt in a temp repo, emit two milestones, claim> --done 'file done.txt' --thread <DISCORD_HOME_CHANNEL> --lead <owner home DM session key>`; observe in `gateway.log` + DB copy: thread created, milestones posted, claim, settle, verified_done, closed, lead turn delivered for done_claimed. OWNER: optional look in Discord afterwards (non-blocking).
  Must NOT do: ad-hoc signing; skip the baseline comparison.
  Closes: verifies GAP-1..GAP-12 together
  Parallelization: Wave 5 | Blocked by: 1, 15-21, 25, 26, 28, 29, 31, 32, 34, 35 | Blocks: 37
  References: memory INSTRUCTION (Developer ID signing); `~/.omon/gateway.log` (UTC)
  Acceptance criteria (machine, blocking): `codesign -dv target/release/omo-gateway` shows TeamIdentifier=5DUM8WPB4C; a DB copy shows the block markers in bot 1465631383862120451's prompt; lane e2e-1 history = working -> done_claimed -> verified_done -> closed; its `done_claimed` event has `delivered_at`; `gateway.log` has the Discord API success lines for thread create, two milestone posts and the archive; `pgrep -f "herdr server"` still exits 1 (nothing reintroduced herdr); `omo-gateway inbox list --class owner --json` and `omo-gateway sessions list --json` and `omo-gateway cron health --json` and `omo-gateway usage --since 1h --json` on the release binary exit 0; the launchd environment contains TYPESAFE_API_KEY/URL/MODEL (names only checked) and an owner-DM turn in `gateway.log` carries `mahoquot/claude-opus-5-5` (counts logged); no new failing tests vs baseline.
  QA scenarios: happy - evidence bundle `task-36/` (gate logs, codesign output, lane history JSON, gateway.log excerpt); failure - lane e2e-2 whose brief claims without creating the file -> not_done with the reason and the lane continues; then `close --force --note "forced: e2e"`; clean temp repos with receipts.
  Commit: N (deploy)
  Recommended task executor category: unspecified-high - deploy plus end-to-end verification.

- [x] 37. Record operating facts in memory and write the evidence report
  What to do: memory tool: update `reference/omon-gateway.md` with the lane system (app-server lane threads, subcommands, tables, kill switches, QA invocations from todos 8, 12, 13, 23, 27, 28, 30, 31, 33, 34, 35, 36), the model tiers and the jev purposes, the inbox sources and session hosts and todo 1's re-enable-path finding; record that the owner does not use herdr; write `.omo/evidence/dori-grade-agent-env/REPORT.md` with one row per IS-1..IS-19 and its evidence path.
  Must NOT do: commit unrelated owner files.
  Closes: IS evidence ledger
  Parallelization: Wave 5 | Blocked by: 36 | Blocks: F1-F4
  References: this plan's Success criteria table
  Acceptance criteria: a script runs `test -e` on every evidence path in REPORT.md and exits 0.
  QA scenarios: happy - script log; failure - a renamed evidence file makes the script exit 1 (run once, restore).
  Commit: N
  Recommended task executor category: writing.

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [x] F1. Plan compliance audit
- [x] F2. Code quality review
- [x] F3. Real manual QA
- [x] F4. Ideal-state fidelity

## Commit strategy
- One commit per todo marked Commit: Y, Conventional Commits matching history (`feat(scope): ...`, `refactor(...)`, `docs(...)`; see `git log --oneline -20`, e.g. `245a587 feat(discord,backend): ...`).
- Stage only the todo's files after `git status --short`; never stage `docs/research/*`, `.omo/plans/gateway-autonomy-lanes.md`, `.omo/thread-tools/`.
- Owner rule (memory): commits and pushes only after the owner says so in the execute session; without that, stage per todo and save drafted messages in the evidence dir.
- Footer on every commit: `Plan: .omo/plans/dori-grade-agent-env.md`

## Success criteria
> One row per IS row. The plan is complete only when every IS row has a delivering todo and a proving QA scenario; F4 checks the delivered behavior against these rows 1:1, and a shortfall becomes new `- [ ] N.` rows, never a note.
| IS | Delivering todo(s) | Proving QA scenario | Evidence |
| --- | --- | --- | --- |
| IS-1 | 1 | todo 1 acceptance + 10-minute respawn re-check | .omo/evidence/dori-grade-agent-env/task-1.txt |
| IS-2 | 3, 14, 18, 20, 36 | todo 36 e2e: owner DM reply latency while e2e-1 runs (gateway.log); lane turns never block chat (todo 3 separate session) | .omo/evidence/dori-grade-agent-env/task-36/ |
| IS-3 | 3, 8, 10, 11 | todo 8 live `lane start` on the app-server + transcript | .omo/evidence/dori-grade-agent-env/task-8.json |
| IS-4 | 12, 14 | todo 12 live question event; todo 14 delivered lead turn | .omo/evidence/dori-grade-agent-env/task-12.log, task-14.log |
| IS-5 | 5, 13, 15 | todo 13 verified_done + not_done-with-reason | .omo/evidence/dori-grade-agent-env/task-13.log |
| IS-6 | 12, 14 | milestone -> zero events; 3 events -> one turn; 30 s cap | .omo/evidence/dori-grade-agent-env/task-12.log, task-14.log |
| IS-7 | 16 | todo 16 one-item-one-turn + idle-zero-turns + daily cap | .omo/evidence/dori-grade-agent-env/task-16.log |
| IS-8 | 6, 17 | todo 17 queued -> working; heavy serialization | .omo/evidence/dori-grade-agent-env/task-17.log |
| IS-9 | 9, 15 | todo 9 live reopen with prior context; todo 15 reopen-on-reply test | .omo/evidence/dori-grade-agent-env/task-9.log, task-15.log |
| IS-10 | 10, 18, 19 | todo 10 live rename/archive; todo 19 redaction | .omo/evidence/dori-grade-agent-env/task-10.log, task-19.log |
| IS-11 | 20 | todo 20 sentinel check + onboarding/lane transcript | .omo/evidence/dori-grade-agent-env/task-20.log |
| IS-12 | 4, 8, 12, 13, 15 | runner-restart resume, settle-after-restart, re-delivery tests | .omo/evidence/dori-grade-agent-env/task-8.json, task-13.log, task-15.log |
| IS-13 | 22, 23, 24, 25 | todo 23 live per-source fetch; todo 24 triage run; todo 25 list | .omo/evidence/dori-grade-agent-env/task-23.log, task-24.log, task-25.log |
| IS-14 | 26 | todo 26 approve -> lane; unapproved -> nothing | .omo/evidence/dori-grade-agent-env/task-26.log |
| IS-15 | 27, 28, 29 | todo 27 list incl. omarchy; todo 29 reply within 30 s | .omo/evidence/dori-grade-agent-env/task-27.json, task-29.log |
| IS-16 | 28 | todo 28 send/read pong + adopted external lane to verified_done (fake) | .omo/evidence/dori-grade-agent-env/task-28.log |
| IS-17 | 30, 32 | todo 30 hook tests (same-call detection, no timers) + scratch-gateway QA timestamps | .omo/evidence/dori-grade-agent-env/task-30.log |
| IS-18 | 31, 32 | todo 31 failing QA job -> lane -> one re-run -> fixed; publishing job -> owner task, no re-run | .omo/evidence/dori-grade-agent-env/task-31.log, task-32.log |
| IS-19 | 8, 12, 14, 16, 24, 31, 33, 34, 35 | todo 33 live 12-sample replay; todo 34 model ids on turn/start; todo 35 usage per tier + jev diff gate | .omo/evidence/dori-grade-agent-env/task-33.log, task-34.log, task-35.log |

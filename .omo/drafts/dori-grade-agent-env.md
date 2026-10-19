---
slug: dori-grade-agent-env
status: reviewed-approved
intent: unclear
review_required: true
plan_path: .omo/plans/dori-grade-agent-env.md
plan_sha256: 4935c99a8b2ce98062bc64fa727e00f9a4ee1a48e39a2c60ef6d2062bbfb0d75
review_round_id: r9-1791665883412
review_round_limit: 5
pending-action: none (handoff to /ulw-execute)
review:
  plan_reviewer:
    status: approved
    workspace_root: /Users/indo/code/project/omon-gateway
    runtime_home: null
    target: .omo/plans/dori-grade-agent-env.md
    round_id: r9-1791665883412
    plan_sha256: 4935c99a8b2ce98062bc64fa727e00f9a4ee1a48e39a2c60ef6d2062bbfb0d75
    launch_id: null
    session: st_01a1279b
    result: OKAY
approach: Rust-native Dori lead inside omon-gateway - lanes are OMO app-server threads run by a gateway LaneRunner (own cwd, chained turns, captured output), lane reports become coalesced lead turns, mechanical done protocol with 5-min objection window, kibitzer cron, host guard; NO herdr (owner does not use it); legacy herdr OmOMeow + its herdr server retired first
---

# Draft: dori-grade-agent-env

## Affected user and ideal state
Users: (1) owner (Indo) talking to OmOMeow on Discord DM/channels/threads; (2) the lead OMO session (consumer of lane/watch events); (3) lane OMO sessions (consumers of briefs, done rules); (4) other people in Discord channels where the bot sits (room presence).
IS-1 | exactly one brain answers as OmOMeow | split brain = duplicate/contradictory posts (memory split-brain note; regression 10-10)
IS-2 | any owner message gets an answer or a one-line plan within 60s, in its thread | OMOMEOW-MODE 11, user instruction (선제 보고)
IS-3 | long work leaves the chat free: lane per job, own thread, owner can keep talking | dori SKILL routing; 1800s cap src/agent/omo_backend.rs:76
IS-4 | lane state changes (done/blocked/question/dead/stale) reach the lead without owner input | 비가와요 10-09 23:58 "상태 변화" + 노고노고 wake list
IS-5 | "done" means machine-verified live signals; claims that fail are reopened with reasons | dori done-protocol.md; user instruction (기계 판독)
IS-6 | lead is woken only by events worth a turn (completion/question/blocker/alerts), not every heartbeat | 노고노고 10-09 23:32 cache burn
IS-7 | proactive: the agent notices what the owner would have done (mail, calendar, GitHub, stale work) and acts or asks | 공식냥 kibitzer 09-28 11:09-11:10
IS-8 | host protected: no new lane when memory/disk/pane limits are hit; heavy builds serialized | dori guard/heavy
IS-9 | closed job can be resumed in its thread with its recorded session | OMOMEOW-MODE 19; dori sessions.md registry
IS-10 | in shared channels: answers only on mention/name/watched triggers, never spams; outbound text scrubbed of secrets/paths | 잡도리 10-09 11:32; dori standing rules
IS-11 | onboarding is consent-gated and read-only, results in memory | dori onboarding.md
IS-12 | everything survives gateway restart (lanes, deadlines, claims) | dori watch deadline in registry; gateway resume-pending pattern
GAP-1 | ai.omomeow.herdr re-enabled & live (pid 66062, listen.ts 67652/67659, pane w8:p1) | regression after 10-08 disable | todo 1-2
GAP-2 | no lane launch path; insert_work_item no callers (src/storage/db.rs:998) ; omomeow-delegate bash script inserts but no lifecycle | todo 4-6
GAP-3 | gateway turns only start from inbound/cron; no gateway-initiated event turn | src/multiplexer/router.rs:312 route only; todo 7-8
GAP-4 | no done protocol; work_items has no status machine/signals | migrations/0030_work_items.sql; todo 9-11
GAP-5 | heartbeat posts every stale item to Discord (no wake filter, no lead decision) | src/work_heartbeat.rs:317-380; todo 8
GAP-6 | no proactive watchers (mail/calendar/GitHub) in gateway | legacy watch.ts only; todo 12-13
GAP-7 | no kibitzer | todo 14
GAP-8 | no host guard / heavy slots | todo 15
GAP-9 | no thread status lifecycle (emoji names, archive on done, reopen) beyond archived flag tracking src/discord/adapter.rs:1254-1282 | todo 16
GAP-10 | no name/watched-target trigger, no outbound scrub | todo 17
GAP-11 | no onboarding flow | todo 18
GAP-12 | persona prompt in bot_profiles/omomeow.json does not carry lane/done rules | todo 19

## Components (topology ledger)
A | one brain: legacy herdr OmOMeow retired and cannot come back | active | launchctl evidence 10-10
B | lane engine: start/adopt/list/send/close lanes in herdr tabs, registry in SQLite, thread binding, resume | active | work_items + omomeow-delegate
C | event wake: lane events + watchers become coalesced lead turns | active | router.rs, work_heartbeat.rs
D | done protocol: signals, claim/object, 5-min auto-close, worktree checks | active | dori done-protocol
E | proactive kibitzer + host guard | active | 공식냥 quotes; dori guard
F | front-of-house: thread lifecycle, room triggers, outbound scrub, onboarding, persona rules | active | OMOMEOW-MODE, dori SKILL

## Open assumptions (announced defaults)
lane runtime | OMO app-server threads via existing OmoBackend (thread/start cwd, thread/resume, turn/steer); NO herdr | owner: "나는 herdr를 안쓰고있는데?" (2026-10-10); structured events; no new daemon | yes
lead control surface | new subcommands of the gateway binary `omo-gateway lane ...`, called by the OMO session via bash | OMO backend sessions already have bash; no new protocol | yes
lane model | gateway's resolved default model (no new model) | memory: model change forbidden | yes
event delivery | synthetic InboundEvent with content prefix `[LANE_EVENT]`/`[WATCH]` routed through SessionMultiplexer::route into the lead session key | reuses actor queue, ordering, steer | yes
wake filter | only done-claimed, blocked/question, dead, stale>15m, guard alert, kibitzer wake=true; coalesce within 30s | cost (노고노고) | yes
done window | 5 min, signals re-run by gateway, argv only, 120s per command | dori | yes
kibitzer | cron-scheduled every 30 min 08:00-24:00 KST, cheap model, JSON output, max 6 wakes/day | cost cap; reversible | yes
guard thresholds | dori defaults: mem free 20%, disk 50GB, panes 20, load alert 150 | upstream defaults | yes
feature flag | OMON_LANES_ENABLED default true after ship; OMON_KIBITZER_ENABLED default true | zero-config rule | yes

## Findings (cited - path:lines)
- legacy live: launchctl print-disabled shows ai.omomeow.herdr enabled; pids 66062, 67652, 67659; herdr pane w8:p1 working (10-10 21:5x KST)
- src/agent/omo_backend.rs:76 INTERACTIVE_TURN_CAP 1800s; :934 45s initial response timeout
- src/multiplexer/router.rs:312 route(event) is the only entry; main.rs:455 builds InboundEvent for resume (pattern for synthetic events)
- src/work_heartbeat.rs:317-380 step(): stale in_progress -> post to Discord, update progress
- src/storage/db.rs:998 insert_work_item; migrations/0030_work_items.sql schema
- bin/omomeow-delegate + docs/AGENTS_DELEGATE.md existing CLI (bash)
- src/discord/adapter.rs: EditMessage, reactions, broadcast_typing, archived tracking 1254-1282
- bot_profiles/omomeow.json persona; model inferhub/cb/deepseek-v4.1-flash
- tools installed: herdr, omo, aside, zele, gh, agent-messenger, agent-slack, typesafe-judge
- upstream sisyphuslabs/omo-dori-mode-experimental @021f93fd (2026-10-07): SKILL.md, done-protocol.md, sessions.md, onboarding.md, scripts.md
- docs/research/omo-dori-mode-{analysis,critique}.md (untracked user WIP); .omo/plans/gateway-autonomy-lanes.md (untracked)
- archive: 잡도리 10-10 13:11 "도리 프라이빗 베타가 끝나서" ; 공식냥 kibitzer 09-28 11:09-11:10; 멀티디바이스·메모리동기화 09-29 11:15

## Decisions (with rationale)
- Rust-native in gateway, not bundling the bun dori CLI (critique doc: double daemon); port its contracts (signals, registry fields, can-launch, guard) not its code.
- Keep herdr as lane runtime (user uses it; 연규 09-29) but read state only through herdr JSON API.

## Scope IN
Components A-F; Discord as the only messenger surface; Slack only as a lane-tracking post target via agent-slack if owner approves.

## Scope OUT (Must NOT have)
- No model routing changes; no edits to ~/.omo/agent/omo.json or ~/.omo/omo.json
- No katok; no KakaoTalk sending by protocol hacking
- No agent-discord user-account sending
- No Huddle/Google Meet joining unless owner approves (question)
- No multi-device memory sync / OmO cloud (not available)
- Do not touch untracked docs/research/*, .omo/plans/gateway-autonomy-lanes.md, src/agent/omo_backend.rs WIP beyond needed lines

## Open questions
- Q-approach (approval gate)
- Q-external: Huddle/Meet join, Slack tracking, KakaoTalk send - owner-decision (accounts/spend/ban risk)

## Approval gate
status: drafting
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->

## Review log
- 2026-10-10T13:47:08.154Z approval: owner "승인" (no external integrations selected -> all OUT). plan written; plan-consultant (gemini) findings folded: persona DB sync (no import path; apply-block CLI), herdr agent start --kind pi + @brief, dispatch rate cap, kibitzer context cap, machine-only acceptance for todo 23; legacy work_items empty in live DB copy. round r1-1791640028149 launching.
- 2026-10-10T13:47:36.858Z round 1 plan-reviewer st_01a12611 verdict OKAY; live sha matches recorded: true
- 2026-10-10T14:21:20.383Z SCOPE CHANGE: owner does not use herdr. Lane runtime switched to app-server threads (todos 3,8,9,12 rewritten; herdr client/pane-send todos removed; delegate+heartbeat herdr calls removed; todo 1 stops the legacy herdr server). 23 todos. Prior round-1 OKAY invalidated (plan changed). round r2-1791642080379 launching.
- 2026-10-10T14:22:51.892Z round 2 plan-reviewer st_01a12630 OKAY; sha match true

## Scope change 2026-10-10 (owner): "1. 메일,카톡,디스코드,imsg 등을 보고 내가 해야될 일인지 봇이 처리해야되는 일인지도 판단해서 정리 2. 돌아가고 있는 오모 세션들(내 모든 연결된 머신) 상태를 내가 물어보면 즉시 확인하고 해당 세션에 메시지를 보내거나 스스로 관리해서 마무리" + "지금은 계획만"
Findings:
- omo thread CLI (omo-ai plugin, #9143/#9660): list|send|read|report|answer|bind..., --json, exit codes 0 done/1 refused/2 usage/3 nothing answering/4 unsupported/5 internal; controls per session: send, read, rename, set_model, set_reasoning, interrupt; messages to non-running sessions kept 24h; never interrupts typing. JS SDK at omo-ai/plugin/runtime/thread-sdk/sdk.js.
- Local: `omo thread list --all-scope --json` -> 17 live interactive tui sessions (fields alive,status,cwd,name,updated_at,socket,controls).
- Remote over ssh BatchMode: omarchy (omo at /usr/local/bin/omo, 1 live session /home/projects/mhc); noveling, oracle-vps reachable but no omo sessions; maho-win/maho-scalper/maho-steam are Windows (PowerShell), omarchy-pirate/-erd have RemoteCommand config, vuev permission denied, maho-mac + xmr-miner timeout.
- Inbox sources available: mail = zele (freedomzero91@gmail.com, gmail+calendar, auth expires and refreshes), existing cron omocat-calendar-mail-alerts-v1 (*/5, script ~/code/project/omocat-alerts/alerts.ts); iMessage/SMS = imsg 0.13.4 (chats/history/watch --json); KakaoTalk = PirateTalk API https://indo.taila0cef2.ts.net (200) + existing archive cron (pomodan only), katok forbidden; Discord = gateway's own message store (messages + message_search_documents).
Defaults adopted: machines = ssh hosts from ~/.ssh/config where omo exists (probe), Windows hosts out until omo found; triage read-only, never sends/marks read; owner-task list delivered on request + daily digest 09:00 KST via existing home DM; bot-task items become lanes only after owner OK unless low-risk internal.
- 2026-10-10T14:59:52.259Z plan revised: +IS-13..16, +GAP-13..16, todos 22-29 (inbox store/sources/triage/list+digest/approve, session registry/control/status), deploy->30, record->31. round r3-1791644392258 launching.
- 2026-10-10T15:00:24.258Z round 3 plan-reviewer st_01a12653 OKAY; sha match true
- 2026-10-10T15:03:31.064Z scope add (owner): "크론 성공/실패 여부도 모니터해서 추가 작업 필요하면 그것까지 맡아서". Evidence: scheduler.rs:1720-1790 incidents + one alert only; 7d DB copy 760 ok / 5 failed. +IS-17/18, +GAP-17/18, todos 30-32 (health checker, remediation lanes with one verified re-run, report), deploy->33, record->34. round r4-1791644611063 launching.
- 2026-10-10T15:04:11.927Z round 4 plan-reviewer st_01a12657 OKAY; sha match true
- 2026-10-10T15:07:49.546Z owner correction: "크론 잡 결과가 나오면 감시를 해야지 왜 5분마다 쓸데없이 토큰을 낭비함?" -> todo 30 rewritten as completion hooks in scheduler (complete_success/complete_failure/delivery/lease reclaim), no timer, no model; plan-wide event-first rule: kibitzer armed by events, inbox triage only on new items (imsg watch, zele mail watch, gateway hook; kakao cheap HTTP cursor 2 min, no model), adopted sessions via omo thread events. round r5-1791644869546 launching.
- 2026-10-10T15:08:32.061Z round 5 plan-reviewer st_01a1265b OKAY; sha match true
- 2026-10-10T15:11:10.232Z owner asked for another review; round r6-1791645070232 on unchanged sha e3fd67b945ab4be5fa38e7d2bb9fe77bee41787e9c682eb62d310101b6971888
- 2026-10-10T15:12:25.089Z round 6 plan-reviewer st_01a1265e OKAY; sha match true
- 2026-10-10T15:32:29.721Z owner: "jev를 저 전체 아키텍처에서 사용해서 효율화" + models mahoquot/claude-opus-5-5 (main) / inferhub/cb/deepseek-v4.1-flash (small). jev measured 20/20 on decision samples (0.48-0.71 s), diff 1.1 s. +IS-19/GAP-19, todos 33 judge layer, 34 model routing, 35 usage + jev diff gate; jev wired into todos 8,12,14,16,24,31; deploy->36, record->37. round r7-1791646349720 launching.
- 2026-10-10T15:33:34.246Z round 7 plan-reviewer st_01a12671 OKAY; sha match true
- 2026-10-10T20:55:56.576Z owner OK on cost guards: T0 prefilter (mail categories, kakao DirectChat/MultiChat only, pomodan excluded, short-code SMS, discord mentions), jev daily cap 300 + fallback cap 50 + breaker, filter stats in digest. Volumes 7d: mail 281->62, kakao 260 rooms (pomodan ~5,900 msg/day), imsg 53, discord 0 targeted. round r8-1791665756576.
- 2026-10-10T20:58:03.412Z round 8 st_01a12799 REJECT: duplicated first half pasted into todo 22 (JS String.replace treated "$`" in replacement as pre-match pattern). Fixed by exact-boundary cut; 614 lines, 37 todos. round r9-1791665883412.
- 2026-10-10T21:00:15.080Z round 9 plan-reviewer st_01a1279b OKAY; sha match true

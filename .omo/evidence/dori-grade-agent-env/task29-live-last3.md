# Task 29 LAST3: updated compact profile, one measurement

Verdict: **FAIL**. POST to assistant row: **45,404.201 ms** (rounded elapsedMs **45404**). POST to obligation marked delivered: **45,404.487 ms**. The unchanged requirement is delivery within 30,000 ms. Compact-format compliance also **FAIL**: five tool calls, no requested CLI invocation, and 44 reply lines.

## Single invocation

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
QA29_OBSERVATION_MS=180000 bun qa-last-readonly-run.mjs 29 '/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json'
```

Exactly one live invocation. Same binary, request text, owned app-server, read-only omo wrapper, scratch HOME/database, and 180-second observation as LAST2. No model override, retry, product source change, or rebuild.

Preparation found that the existing task29 branch did NOT load bot_profiles/omomeow.json: it inserted only the isolated-QA system prompt. To measure the requested updated data, qa-last-readonly-extra.mjs was minimally changed to read the worktree JSON and prepend its system_prompt to the scratch session prompt. Profile model and enabled_toolsets were not applied, preserving the previous measurement's model/tool configuration. The applied prompt is captured in measurement-profile.json, matches the inspected updated JSON, and the persisted scratch session contains the exact-one-call rule. Neither profile source file was edited by this measurement.

This verifies the profile supplied in gateway session state; it does not prove effective instruction precedence inside the OMO engine. The session transcript also contains a senpi.todo-first-turn custom instruction at 05:31:55.216 requiring todo init before continuing. That conflicts with the profile's no-todo rule. The observed engine still used todo/search/thread_list; no unproven claim that the compact rule was effective is made.

## Authoritative timeline (2026-10-12 UTC)

| Event | UTC | POST offset |
| --- | --- | ---: |
| POST starts, immediately before chat fetch | 05:31:51.849000 | 0 ms |
| User row persisted | 05:31:51.849753 | 0.753 ms |
| Original session created | 05:31:51.870 | 21 ms |
| OMO user input accepted | 05:31:55.216 | 3,367 ms |
| First recorded assistant text | 05:31:59.945 | 8,096 ms |
| Workspace list result | 05:32:05.855 | 14,006 ms |
| Whole-machine list result | 05:32:13.260 | 21,411 ms |
| Final todo result | 05:32:20.104 | 28,255 ms |
| Final assistant text, stopReason=stop | 05:32:37.247 | 45,398 ms |
| Assistant DB row, sequence 2 | 05:32:37.253201 | **45,404.201 ms** |
| Obligation state delivered | 05:32:37.253487 | **45,404.487 ms** |

The first-assistant time is the original transcript event time, not an independently measured first streaming token. DB timestamps carry microseconds, while POST instrumentation has millisecond resolution.

Raw helper elapsedMs=180001 is observer resolution time, NOT reply latency. As in LAST2, the observer discovered the already-persisted delivered response at its final authoritative query. The read-only SQLite query independently confirms the row and delivered obligation at 45.404 seconds. No observer fix or second execution was performed.

## All tool calls

Durations are assistant toolCall event to matching toolResult event. Parallel calls can overlap; these are transcript round trips, not internal execution-only timings.

| Tool | Arguments | Start offset | Duration |
| --- | --- | ---: | ---: |
| todo | {"op":"init","list":[{"phase":"조회","items":["로컬 실행 중 세션 목록 조회하기","세션 상태 목록 정리해 보고하기"]}]} | 8096 ms | 26 ms |
| tool_search | {"query":"list local running sessions status"} | 8096 ms | 26 ms |
| thread_list | {} | 10971 ms | 3035 ms |
| thread_list | {"all_scope":true} | 19367 ms | 2044 ms |
| todo | {"op":"done","phase":"조회"} | 28249 ms | 6 ms |

Breakdown: POST to input acceptance 3.367 s; acceptance to first assistant text 4.729 s; initial todo/search round trips 0.026 s; next assistant event at 10.971 s; workspace list 3.035 s; next list starts at 19.367 s and takes 2.044 s; next todo starts at 28.249 s and takes 0.006 s; last tool result to final assistant text **17.143 s**; final text to delivered state **6.487 ms**. From final session lookup result to final reply is **23.987 s**, including the todo round trip and model/orchestration gaps. These gaps cannot be attributed solely to a tool or token generation from this evidence.

## Compact rule compliance

- Exactly one tool call, omo-gateway sessions list --json: **FAIL**. Five calls: todo init, tool_search, thread_list workspace, thread_list all_scope, todo done. No terminal/CLI invocation occurred.
- No todo bookkeeping or tool search: **FAIL**.
- Reply under 15 lines: **FAIL**, **44 lines** including blanks.
- Busy/working sessions individually, other statuses summarized as counts: **FAIL**. Reply expands live and resumable sessions into tables, uses UTC rather than KST-relative activity, and contains conflicting counts followed by corrections. It explicitly notes that live does not establish an active turn.
- Read-only behavior: no send, interrupt, adopt, or external mutation tools observed.

### Delivered reply (verbatim)

Ask: 지금 돌아가는 로컬 세션 상태 조회 - wanted: send/interrupt/adopt 없이 목록만 보기. For you: 이 머신에는 세션이 25개 있습니다. 실제로 살아 있는 TUI 세션이 19개이고, 응답하지 않는 resumable 자식 세션이 6개입니다. 지금 있는 워크스페이스에는 세션이 하나도 없어서 머신 전체 범위로 다시 조회했습니다. 조회만 했고 send, interrupt, adopt는 하지 않았습니다. Now: 남은 작업 없음. Next: 없음.

**살아 있는 TUI 세션 (live, 19개)** — 최근 업데이트 순, 시각은 UTC

| # | 이름 | 작업 디렉터리 | 최근 업데이트 |
|---|---|---|---|
| 1 | 리모트 채팅 메시지 전송 오류 수정 | /Volumes/T9-Mac/project/ferryx | 10-12 05:32 |
| 2 | Check PirateTalk MCP chat history range | ~/code/project/omon-gateway | 10-12 05:32 |
| 3 | Troubleshoot Release Notes Download Error | ~/code/project/maho-workspace | 10-12 05:30 |
| 4 | (이름이 경로) …sessions/--Users-indo-code-project-co | ~/code/project/content-intel-dashboard | 10-12 05:30 |
| 5 | LAN 윈도우 머신 접속 확인 | ~/code/project | 10-12 05:28 |
| 6 | (이름이 경로) /var/folders/…/ferryx-past | /Volumes/T9-Mac/project/ferryx | 10-12 03:10 |
| 7 | 패스트캠퍼스 스타일 상세페이지 검토 | ~/code/project/lectures | 10-12 01:45 |
| 8 | Start Development Server | ~/code/project/lectures | 10-12 00:55 |
| 9 | (이름 없음) | /Volumes/T9-Mac/project/ferryx | 10-12 00:39 |
| 10 | Clean up stale worktrees and add rule | /Volumes/T9-Mac/project/ferryx | 10-11 23:31 |
| 11 | CLIProxyAPI 업스트림 기능 캐시 프록시 비교 | /Volumes/T9-Mac/project/mahoquot | 10-11 23:07 |
| 12 | Resume Lectures Project Session | ~/code/project/lectures | 10-11 22:56 |
| 13 | 슬라이드 소스코드 책 페이지 참조로 변경 | ~/code/project/lectures | 10-11 22:43 |
| 14 | 인프런 슬라이드 시각화 개선안 | ~/code/project/lectures | 10-11 22:37 |
| 15 | Debug invalid native tool call order | /Volumes/T9-Mac/project/mahoquot | 10-11 21:59 |
| 16 | Personal Identity Ontology Update | ~/code/project | 10-11 14:49 |
| 17 | mass ulw research plan 국내/해외 크라우드펀딩… | ~/code/project | 10-11 14:49 |
| 18 | Oliver Heldens AI Music Production | ~/code/project | 10-11 14:49 |

| 19 | (위 표에 이어서) — 전체 live 세션은 위 18개에 아래 1개를 더한 19개입니다 | | |

정정하면, 위 표에 실제로 나온 live 세션은 18개이고 위 "19" 행은 잘못 넣은 것입니다. live TUI 세션은 **18개**, 전체 세션은 **24개**입니다.

**응답 없는 자식 세션 (resumable, `live_unresponsive`, 6개)** — 모두 ferryx 프로젝트이고, 같은 RPC shard(p-b22f7592…)에 묶여 있습니다.

| # | 이름 (잘린 상태) | 최근 업데이트 |
|---|---|---|
| 1 | Goal: run one already-built Ferryx test binary inside the IN… | 10-11 15:03 |
| 2 | Ferryx repo … (shared tree; pre… | 10-11 15:03 |
| 3 | Ferryx repo … (shared tree; pre… | 10-11 14:45 |
| 4 | Ferryx repo … (shared tree; pre… | 10-11 14:43 |
| 5 | User requires batch repair of all existing diagnostics befor… | 10-11 14:40 |
| 6 | Investigate Windows live SSH test failed74587 A/B candidate. | 10-11 14:38 |
| 7 | User requires all diagnostics repaired as one batch followed… | 10-11 14:37 |

이 표도 실제로는 7개입니다. 최종 집계는 **live TUI 18개 + 응답 없는 resumable 자식 7개 = 25개**입니다.

참고로 "live"는 세션 소켓이 살아 있다는 뜻이지, 지금 턴을 돌리고 있다는 뜻은 아닙니다. 위쪽 5개만 몇 분 안에 업데이트됐고, 나머지는 몇 시간 이상 업데이트가 없습니다. 응답 없는 자식 7개는 어제 14:37~15:03 사이에 멈췄고, 그 뒤로 업데이트가 없습니다.

## Release identity and isolation

Binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`.
Pre SHA-256: `f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323`.
Post SHA-256: `f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323`.
Both match final-release.json and the required f892eebb binary. A separate pre-invocation hash check also matched.

Scratch: `/private/tmp/omon-task29-live-final-W7BFw1`; DB: `/private/tmp/omon-task29-live-final-W7BFw1/qa.db`. Both owned gateways exited 0, PIDs absent, sockets removed. Owned app-server PID 92554, endpoint ws://127.0.0.1:54992 (not shared port 19742), was cleaned up: no remaining group PIDs, listener rebind succeeded, private auth scratch removed. Cleanup required SIGKILL after its 15-second graceful bound. Scenario scratch evidence remains retained.

Read-only omo wrapper retains real HOME only for thread list/read and rejects all other omo commands with exit 64. Gateway HOME stays private. No shared daemon, owner DB, env files, production gateway, or live Discord was changed. TYPESAFE_* values remained in memory and were not recorded. Delivery is local transcript acceptance, not live Discord.

Runner status blocked/exit 3 is the existing human-review marker about session listing, not a timeout or final latency verdict. Human review is recorded here; latency and format both fail.

## Evidence and checks

- `/private/tmp/omon-task29-live-final-W7BFw1/owner-session-status.json`: POST timestamp, delivered row, obligation and raw observer elapsed.
- `/private/tmp/omon-task29-live-final-W7BFw1/measurement-profile.json`: applied worktree prompt; modelApplied=false.
- `/private/tmp/omon-task29-live-final-W7BFw1/task29-live-final.json`: SHA identity, wrapper, owned cleanup.
- Original transcript: `/Users/indo/.omo/agent/sessions/--private-tmp-omon-task29-live-final-W7BFw1-workspace-agents-bot-1465631383862120451--/2026-10-12T05-31-51-870Z_01a12e98-8cbe-754c-81b1-8dd406767903.jsonl`.
- Read-only SQLite queries independently confirmed one user row, one assistant row, and one delivered obligation.

No additional tests, builds, or second live runs were performed for this measurement. The prior default-deadline regression passed in LAST2, before this prompt-loading change; it was not rerun or represented as validating this new change. JS LSP initialization remains unavailable because the workspace has no valid TypeScript installation. No dependencies were added. Rust checks do not apply to this data/harness measurement.


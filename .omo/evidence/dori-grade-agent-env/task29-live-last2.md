# Task 29 LAST2: one extended-observation latency measurement

Verdict: **FAIL**. The delivered assistant row arrived **37,581.921 ms** after POST; the obligation became delivered at **37,582.180 ms**. Both exceed the unchanged **30,000 ms** plan requirement. Observation was permitted up to 180,000 ms, not a relaxed PASS deadline. Exactly one live scenario invocation was made; no retry, build, model override, or product fix was performed.

## Invocation and scope

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
QA29_OBSERVATION_MS=180000 bun qa-last-readonly-run.mjs 29 '/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json'
```

The same runner imports `qa-last-readonly-extra.mjs`. Only the observation duration was selected as 180,000 ms for this run; extra timestamps and authoritative rows were captured for measurement. Default observation remains 30,000 ms. The existing prompt, persona, wrapper, gateway environment, and owned app-server setup were preserved.

Scratch root: `/private/tmp/omon-task29-live-final-mVfPZ2`; database: `/private/tmp/omon-task29-live-final-mVfPZ2/qa.db`.

Gateway HOME was scratch HOME. The wrapper permits only `thread list` and `thread read` with real `HOME=/Users/indo`, rejecting all other omo commands with exit 64. The session actually used built-in read-only `thread_list` twice; it did not invoke the wrapper. No shared daemon, owner DB, env file, production gateway, or live Discord was accessed for mutation. TYPESAFE_* were loaded in memory and their values were not recorded. Delivery here means local transcript acceptance and a durable delivered obligation, as in the prior scenario, not a Discord send.

## Authoritative latency

| Event | UTC time | Offset from POST |
| --- | --- | ---: |
| POST begins (instrument immediately before chat fetch) | 05:24:03.972000 | 0 ms |
| User row persisted | 05:24:03.973263 | 1.263 ms |
| OMO session created | 05:24:03.981 | 9 ms |
| OMO user message accepted | 05:24:05.781 | 1,809 ms |
| First assistant text recorded | 05:24:11.318 | 7,346 ms |
| Final assistant text recorded, stopReason=stop | 05:24:41.547 | 37,575 ms |
| Assistant row persisted | 05:24:41.553921 | **37,581.921 ms** |
| Obligation marked delivered | 05:24:41.554180 | **37,582.180 ms** |

Date: 2026-10-12. elapsedMs from POST to the delivered assistant row is **37581.921** (rounded integer **37582**). Original session timestamps have millisecond resolution; DB timestamps have microsecond resolution. First assistant text means the first text event preserved in the session, not an independently instrumented streaming token.

The helper's raw `owner-session-status.json.elapsedMs=180001` is **observer resolution time**, not real reply latency. The response observer missed the earlier change notification and resolved on its final authoritative query at 180 seconds. Read-only SQLite inspection confirms the assistant row and delivered obligation at 37.582 seconds, well before cleanup. This finding was recorded, not fixed or rerun.

## Every tool call in the transcript

Durations are recorded assistant toolCall event to matching toolResult event. Calls sharing an assistant event can overlap; these are transcript round-trip durations, not separately instrumented internal CPU time.

| Tool | Arguments / purpose | Start offset | Duration |
| --- | --- | ---: | ---: |
| todo | {"op":"init","list":[{"phase":"조회","items":["로컬 세션 조회 도구 찾기","실행 중인 로컬 세션 목록 조회"]},{"phase":"보고","items":["세션 상태 목록 정리해 보고"]}]} | 7346 ms | 10 ms |
| tool_search | {"query":"list running local agent sessions status"} | 7346 ms | 11 ms |
| todo | {"op":"done","task":"로컬 세션 조회 도구 찾기"} | 11008 ms | 2681 ms |
| thread_list | {} | 11008 ms | 2681 ms |
| thread_list | {"all_scope":true} | 17188 ms | 1907 ms |
| todo | {"op":"done","phase":"조회"} | 40218 ms | 4 ms |
| todo | {"op":"done","phase":"보고"} | 40218 ms | 4 ms |

The last two todo calls occur after the delivered reply, in the session's automatic continuation. They did not delay the first delivery or cause a second gateway assistant row. That continuation ended at 05:24:48.329; the DB still contains exactly one user and one assistant row.

Timing breakdown to first delivery: startup/input acceptance 1.809 s; input acceptance to first assistant text 5.537 s; initial todo/search round trips at 7.346 s take up to 0.011 s; next assistant tool batch begins at 11.008 s; workspace thread_list/todo round trips take 2.681 s; next listing begins at 17.188 s and takes 1.907 s; final listing result at 19.095 s to final assistant event takes 18.480 s; final assistant event to delivered obligation takes 0.007180 s. Gaps between assistant/tool events include model generation and orchestration and are not attributed to a tool without evidence.

## Session-list requirement

**Satisfied for the delivered reply:** it lists 18 live sessions in a table containing names (explicitly marking unnamed sessions), project folders/cwd, and IDs; the section labels all 18 as live. It separately lists 7 resumable/live_unresponsive child sessions with names/IDs and their shared ferryx project. It distinguishes live processes from proof of an actively running turn. Thus names, folder, and status are supplied, while latency still fails.

### Delivered reply (verbatim)

Ask: 실행 중인 로컬 세션 상태를 목록으로 보여주되 send/interrupt/adopt는 하지 않기 - wanted: 읽기 전용으로 조회한 세션 목록. For you: 실행 중(live, 응답 있음)인 로컬 세션은 18개이고, 모두 TUI 대화형 세션입니다. 그 밖에 응답이 없는 ferryx 자식 세션 7개가 `resumable` 상태로 남아 있습니다. Now: 세션 상태 목록 정리해 보고. Next: 없음.

조회 방법은 `thread_list` 하나만 썼습니다. 이 도구는 로컬 머신의 세션만 읽습니다. 지금 작업 디렉터리 범위로 먼저 조회했더니 결과가 0개여서, 같은 머신의 모든 workspace로 범위를 넓혀 다시 조회했습니다. 아래 시각은 마지막 업데이트 시각(UTC)이고, ID는 앞 8자리만 적었습니다.

**실행 중 (live, 18개)**

| # | 마지막 업데이트 | 프로젝트 (cwd) | 세션 이름 | ID |
|---|---|---|---|---|
| 1 | 10-12 05:24 | code/project/omon-gateway | Check PirateTalk MCP chat history range | 01a123f9 |
| 2 | 10-12 05:24 | T9-Mac/project/ferryx | 리모트 채팅 메시지 전송 오류 수정 | 01a12db5 |
| 3 | 10-12 05:20 | code/project/content-intel-dashboard | (이름 없음, 경로 문자열로 표시됨) | 01a104d8 |
| 4 | 10-12 03:10 | T9-Mac/project/ferryx | (이름 없음, 임시 파일 경로로 표시됨) | 01a12cfc |
| 5 | 10-12 01:45 | code/project/lectures | 패스트캠퍼스 스타일 상세페이지 검토 | 01a09ee8 |
| 6 | 10-12 00:55 | code/project/lectures | Start Development Server | 01a12026 |
| 7 | 10-12 00:52 | code/project/maho-workspace | Troubleshoot Release Notes Download Error | 01a1191b |
| 8 | 10-12 00:39 | T9-Mac/project/ferryx | (이름 없음) | 01a12d8c |
| 9 | 10-11 23:31 | T9-Mac/project/ferryx | Clean up stale worktrees and add rule | 01a12cf7 |
| 10 | 10-11 23:07 | T9-Mac/project/mahoquot | CLIProxyAPI 업스트림 기능 캐시 프록시 비교 | 01a128b6 |
| 11 | 10-11 22:56 | code/project/lectures | Resume Lectures Project Session | 01a0e9cd |
| 12 | 10-11 22:43 | code/project/lectures | 슬라이드 소스코드 책 페이지 참조로 변경 | 01a0ecbb |
| 13 | 10-11 22:37 | code/project/lectures | 인프런 슬라이드 시각화 개선안 | 01a0a7a8 |
| 14 | 10-11 21:59 | T9-Mac/project/mahoquot | Debug invalid native tool call order | 01a12b77 |
| 15 | 10-11 21:59 | code/project | (이름 없음) | 01a12cfa |
| 16 | 10-11 14:49 | code/project | Personal Identity Ontology Update | 01a11b4e |
| 17 | 10-11 14:49 | code/project | mass ulw research plan (크라우드펀딩 노션 템플릿) | 01a0de23 |
| 18 | 10-11 14:49 | code/project | Oliver Heldens AI Music Production | 01a11b4f |

방금 전까지 업데이트가 있었던 세션은 1~3번뿐입니다. 4~18번은 프로세스는 살아 있지만 몇 시간 이상 업데이트가 없어 대기 중인 것으로 보입니다. 다만 이 목록만으로 지금 턴이 돌고 있는지까지는 알 수 없습니다.

**응답 없음 (`resumable`, 응답 없음 표시 `live_unresponsive`, 7개)**

모두 ferryx 프로젝트의 자식(subagent) 세션이고, 10-11 14:37~15:03 이후로 업데이트가 없습니다.

- **01a12b75**: Goal: run one already-built Ferryx test binary…
- **01a12b77**: Ferryx repo … (shared tree)
- **01a12b68**: Ferryx repo … (shared tree)
- **01a12b69**: Ferryx repo … (shared tree)
- **01a129dd-3b35**: User requires batch repair of all existing diagnostics…
- **01a129dd-43c3**: User requires all diagnostics repaired as one batch…
- **01a128e5**: Investigate Windows live SSH test failed74587 A/B candidate

어느 세션에도 send, interrupt, adopt를 하지 않았습니다.

## Binary identity and isolation cleanup

Release path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json`.
Binary path: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`.
Before SHA-256: `f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323`.
After SHA-256: `f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323`.
Both match final-release.json. This release differs from the previous report's 0924e30d... binary; the requested current final-release.json was used.

Both owned gateway processes exited 0, their PIDs disappeared, and their sockets were removed. Owned app-server PID 15991 used ws://127.0.0.1:52664, not shared port 19742. Its process group has no remaining PIDs, listener rebind succeeded, and private auth scratch was removed. Cleanup used SIGKILL after its 15-second graceful-shutdown bound; no shared process was stopped. Scenario scratch evidence remains retained.

The runner exited 3/status blocked solely for the unchanged human-review marker about session-list meaning; this is not the latency verdict. That review is completed above and the final latency verdict is FAIL.

## Evidence and validation

- `/private/tmp/omon-task29-live-final-mVfPZ2/owner-session-status.json`: POST timestamp, raw observer elapsed, assistant rows and delivered obligation.
- `/private/tmp/omon-task29-live-final-mVfPZ2/task29-live-final.json`: wrapper, pre/post SHA checks and owned cleanup receipt.
- `/private/tmp/omon-task29-live-final-mVfPZ2/task-29-live-final.log`: configured owned endpoint and gateway stop at 05:27:03.975682.
- Original session: `/Users/indo/.omo/agent/sessions/--private-tmp-omon-task29-live-final-mVfPZ2-workspace-agents-bot-1465631383862120451--/2026-10-12T05-24-03-981Z_01a12e91-690d-7b8c-b555-43c11c7414fb.jsonl`.
- Read-only SQLite query independently confirmed sequence 2 assistant row and delivered obligation timestamps.
- Existing offline regression `bun test ./.omo/evidence/dori-grade-agent-env/task29-deadline.test.mjs`: 1 pass, 0 fail, exit 0. It confirms the default 30-second observer and subscription before POST.
- JS LSP diagnostics could not initialize because no valid TypeScript installation exists in this workspace. No dependency was added. Rust build/clippy/tests were not run because no Rust source or binary changed.

No second live run was performed.


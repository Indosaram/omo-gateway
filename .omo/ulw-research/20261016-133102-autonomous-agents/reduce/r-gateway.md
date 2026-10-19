<!-- task=st_01a14528 -->
st_01a14528 [completed] model mahoquot/claude-opus-5-5
Ask: 리포트 10개를 하나로 합쳐 역량 지도와 갭 목록 만들기 - wanted: 역량 표, 자율형 플랫폼 기준 갭 목록, 문서와 코드 불일치 목록, 그리고 file:line 근거 15개 이상을 실제 소스와 대조한 결과. For you: 정리를 마쳤습니다. 근거 22개를 소스에서 직접 확인했고, 그중 2개는 리포트가 틀렸고 1개는 줄 번호가 약간 어긋나 있었습니다. Now: 없음. Next: 없음.

# omon-gateway 현재 역량 지도와 갭 (wave1 리포트 10개 통합)

출처 약어: 리포트 파일은 모두 `/Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/` 아래에 있습니다. 이하 [L]=code-lanes.md, [C]=code-cron.md, [I]=code-inbox.md, [M]=code-memory.md, [CH]=code-channels.md, [A]=code-agent.md, [S]=code-security.md, [T]=code-tools.md, [W]=code-web.md, [D]=code-docs.md 로 줄여 씁니다. 코드 경로는 모두 `/Users/indo/code/project/omon-gateway/` 기준입니다. [V]는 이번에 소스를 직접 읽어 확인한 근거이고, 표시가 없는 항목은 리포트 내용(SECONDARY)입니다.

## 0. 소스 대조 결과 (22개)

| # | 리포트 주장 | 실제 소스 | 결과 |
|---|---|---|---|
| 1 | lane 안의 ApprovalRequest를 거부함 (runner.rs:661) [L] | `src/lanes/runner.rs:661-665` "lane approval requires lead intervention" | 맞음 |
| 2 | 턴 상한 24, 고정값 (runner.rs:30) [L] | `runner.rs:30` `DEFAULT_MAX_TURNS=24`이지만 `runner.rs:1005` `OMON_LANE_MAX_TURNS`로 바꿀 수 있음 | **부분 오류**: 고정값이 아니라 env로 조정 가능한 기본값 |
| 3 | drain을 확인하는 곳은 dispatch뿐 [L] | grep 결과 `is_draining`은 `src/lanes/dispatch.rs:48,139`에만 있고 runner에는 없음 | 맞음 |
| 4 | kibitzer의 lead가 하드코딩됨 (kibitzer.rs:21-22) [L][M] | `src/lanes/kibitzer.rs:21-22` `DEFAULT_LEAD`에 owner, bot, channel ID가 박혀 있음. 30분 간격 상수는 :23, KST 상수는 :24 | 맞음 |
| 5 | kibitzer는 도구 없이 돌고 approval을 거부함 [L][M] | `kibitzer.rs:300` `enabled_toolsets = Some(Vec::new())`, :94 ApprovalRequest 거부, :147 `OMON_KIBITZER_ENABLED` | 맞음. 도구 설정 줄은 [L]이 302라고 했지만 실제는 300 |
| 6 | 기본 정책이 모든 요청을 자동 승인 [A] | `src/agent/omo_backend.rs:1456-1470`: `APPROVAL_MODE`가 `always`가 아니면 자동 승인 | 맞음. 줄 번호는 [A]의 1438-1452보다 약 18줄 뒤 |
| 7 | inbox source를 SQL CHECK가 4종으로 제한 [I][T] | `migrations/0034_inbox.sql:3,37` mail/kakao/discord/imessage | 맞음 |
| 8 | tracker 모듈이 컴파일되지 않음, api.rs 없음 [T] | `src/lib.rs`에 `pub mod tracker` 없음. `src/tracker/`에는 lanes.rs, model.rs, seed.rs만 있음 | 맞음 |
| 9 | `model_usage` 테이블을 읽거나 쓰는 코드가 없음 [W] | src 전체 grep 결과 0건 | 맞음 |
| 10 | 실시간 voice 리스너를 생성하는 코드가 없음 [CH] | `SongbirdAudioEventListener::new` src 전체 grep 결과 0건 | 맞음 |
| 11 | `build_cron_tools`를 호출하는 곳이 없음 [T] | `src/main.rs:614` 정의 한 곳뿐 | 맞음 (src 기준) |
| 12 | MCP 클라이언트 목록이 비어 있음 [T] | `src/main.rs:1082`와 `src/dashboard_runtime.rs:126`이 모두 `McpTool::default()` | 맞음. [T]는 1083이라고 했지만 실제는 1082 |
| 13 | CompositeDispatcher는 web이 아니면 전부 Discord로 보냄 [CH][W] | `src/dashboard.rs:557-565` | 맞음 |
| 14 | `approval_allowlist`에 TTL, 사용자, 감사 정보가 없음 [S] | `migrations/0007_approval_allowlist.sql:1-4` 컬럼은 pattern_key와 created_at뿐 | 맞음 |
| 15 | memory 검색이 세션 단위의 lexical 방식 [M] | `src/memory/store.rs:108-133` `WHERE session_key = ?` 후 cosine + overlap + phrase 점수 | 맞음 |
| 16 | dashboard는 loopback에만 바인딩 [W] | `src/dashboard.rs:108-113` | 맞음 |
| 17 | cron 전송은 Discord u64 ID만 받음 [C] | `src/cron/scheduler.rs:2357` `chat_id.parse::<u64>()` | 맞음 |
| 18 | guard 임계값 [L] | `src/lanes/guard.rs:10-15` lanes 최대 6, heavy 슬롯 1 | 맞음 |
| 19 | 메일 계정 하드코딩 [I] | `src/inbox/sources/mail.rs:9` | 맞음 |
| 20 | lead가 바쁘면 시스템 이벤트를 버림 [A] | `src/multiplexer/actor.rs:360-364` "lead session is busy" | 맞음 |
| 21 | Owner ID 하드코딩 [I] | `src/inbox/proposals.rs:10` | 맞음 |
| 22 | 승인 대기 시간: terminal 120s, registry 900s [S][T] | `src/tools/terminal.rs:69` 120s, `src/tools/mod.rs:70` 900s | 맞음. 경로마다 값이 다름 (3장 참고) |

추가 확인:
- settle loop와 queued retry가 실제로 연결되어 있습니다: `runner.rs:1033`, `:1086`, `:1093-1095` [V].
- 승인 요청을 처리하는 프로덕션 구현은 Discord 하나뿐입니다: `src/discord/approval.rs:239` [V].
- 음성 메시지 로컬 전사 경로는 `src/discord/adapter.rs:1688-1693`, 최종 텍스트만 보내는 스트리밍 처리는 `src/discord/adapter.rs:3509`입니다 [V].
- `src/main.rs:955`의 `CronScheduler::new`도 확인했습니다 [V]. 다만 이 headless 경로에 dispatcher가 붙지 않는다는 [C]의 주장은 해당 줄만으로는 판단할 수 없어 미검증입니다.

## 1. 역량 표

| 영역 | 현재 할 수 있는 것 | 근거 file:line | 성숙도 |
|---|---|---|---|
| Lanes (장시간 작업) | CLI로 start/list/send/claim-done/object/close/reopen. 상태 8개짜리 state machine을 CAS로 전이 | `src/lanes/cli.rs:24-76`, `src/lanes/state.rs:9-77,132-185` [L] | 운영중 |
| Lanes 완료 검증 | 5분 objection window 후 done-signal(merged/closed/published/command/file/url)과 judge의 diff 검토 | `src/lanes/settle.rs:47-176`, `src/lanes/signals.rs:25-59` [L]. 연결 위치 `runner.rs:1033,1093` [V] | 운영중 |
| Lanes 자원 guard | 부하, 메모리, 디스크, lane 수를 검사하고 queued lane을 10분 backoff로 재시도 | `guard.rs:10-15` [V], `guard.rs:671` [V], `runner.rs:1086` [V] | 운영중 |
| Lead 알림 | 이벤트를 30초 batch로 모아 lead가 idle일 때만 `[LANE_EVENT]` 턴으로 주입. judge가 걸러냄 | `src/lanes/dispatch.rs:30,82-198` [L] | 운영중 |
| 선제 제안 (kibitzer) | inbox나 lane 이벤트가 오면 트리거됨. 30분 간격, 하루 6회, judge 0.7 이상일 때 JSON 제안을 lane_events에 기록 | `kibitzer.rs:180-198,242-364` [L][M]. 상수 `:21-24` [V] | 부분 (제안만 함) |
| Cron | cron, interval, once 스케줄. lease로 중복 실행 방지, catch-up, backoff, 변화 감지(monitor), 스크립트 실행 | `src/cron/scheduler.rs:23-85,1386-1478,2794-2882`, `src/cron/executor.rs:96-178` [C] | 운영중 |
| Cron 자가 수리 | 문제 분류 후 LLM 원인 진단, 재실행 검증, fix lane 생성, owner 승인 토큰 | `src/cron/health.rs:50-179`, `src/cron/remediate.rs:299-395` [C] | 운영중 |
| Inbox 수집 | mail(zele), kakao(PirateTalk), iMessage(imsg), Discord 로컬 인덱스. 모두 읽기 전용 | `src/inbox/mod.rs:116-196`, `src/inbox/sources/*` [I], `0034_inbox.sql:3` [V] | 운영중 (읽기만) |
| Inbox triage / 제안 | jev가 owner/bot/fyi/ignore로 분류, 작성 모델이 제안 생성. 실행 action은 Repo/Lookup/Draft 세 가지, DM의 "<n> 해"로 승인 | `src/inbox/triage.rs:296-372`, `src/inbox/proposals.rs:51-57,430-505` [I] | 운영중 |
| Digest | 매일 09:00 KST에 owner DM으로 전송 | `src/inbox/digest.rs:18-125` [I] | 운영중 |
| Agent backend | omo app-server WS JSON-RPC로 thread와 turn 실행, steer, daemon이 멈추면 재시작, model fallback | `src/agent/omo_backend.rs:183,2054-2172`, `src/agent/omo_protocol.rs:5-121` [A] | 운영중 |
| Multiplexer | 세션마다 actor 하나, 재시작 후 resume, idle 세션 GC(10분), profile 라우팅 | `src/multiplexer/router.rs:14-28`, `src/multiplexer/actor.rs:197-300`, `src/multiplexer/profile_routing.rs:52-190` [A] | 운영중 |
| 원격 세션 제어 | SSH로 원격 omo 세션 read/send/interrupt/answer. external lane으로 adopt, watch는 60초 polling | `src/sessions/control.rs:118-465`, `src/sessions/external.rs:62-441`, `src/sessions/watch.rs:15` [A] | 부분 (관련 plan 미완료, [D]) |
| Memory (gateway) | 세션 단위 저장, lexical 검색. WRITE_APPROVAL로 쓰기 staging. agent가 쓸 수 있는 tool은 없음 | `store.rs:108-133` [V], `src/storage/db.rs:730-860` [M] | 부분 |
| Memory (agent) | agent마다 workspace를 두고 `.omo/omo.json`의 `memory.agent=slug`로 omo에 위임 | `omo_backend.rs:481-533`, `src/agent/agent_workspace.rs:40-80` [M][A] | 운영중 (내부 동작은 gateway 밖) |
| Persona | bot_profiles row, 관리형 lanes prompt block | `src/multiplexer/actor.rs:1344-1444`, `src/profiles/lanes_block.rs:62-189` [M] | 운영중 |
| Discord 채널 | 봇 여러 개, thread/forum, 편집, reaction, 업로드, 음성 메시지 전송, pairing, backfill, dead target 관리 | `src/discord/adapter.rs:2651-4690` [CH] | 운영중 |
| 음성 | 음성 메시지 STT는 Whisper API와 로컬 whisper-cli 두 경로가 모두 돎. 실시간 음성은 scaffold만 있음 | `adapter.rs:1688-1693` [V], `src/voice/pipeline.rs:154-308` [CH]. listener 생성 0건 [V] | 부분 / 실시간은 없음 |
| 승인 (terminal) | hardline 차단 → deny glob → 정책 → Tirith 스캐너 → 위험 regex → Discord 버튼 (120s) | `src/tools/terminal.rs:205-307` [S], `terminal.rs:69` [V] | 운영중 |
| 승인 (agent turn) | 기본값이 자동 승인. `APPROVAL_MODE=always`일 때만 거부 | `omo_backend.rs:1456-1470` [V] | 사실상 꺼짐 |
| Judge (jev) | purpose 8개, 하루 상한, circuit breaker, fallback 모델 | `src/judge/mod.rs:68-126,689-870` [S] | 운영중 |
| 비밀값 처리 | 자식 프로세스 env에서 토큰 제거, approval 표시 redaction, 외부 전송용 scrub | `terminal.rs:658-719`, `src/security/approval_display.rs:7-42`, `judge/mod.rs:20-63` [S] | 부분 |
| Tools | terminal/file/mcp/cron/web/browser/skills/message_context. 단 실제 agent turn은 omo 쪽 도구를 씀 | `src/main.rs:1046-1096`, `src/tools/mod.rs:39-195` [T] | 부분 (MCP 목록 비어 있음 [V]) |
| Dashboard | loopback 전용. 세션 chat과 WS, cron CRUD, 메모리 승인, bot CRUD, 로그 | `src/dashboard.rs:742-786` [W], `:108-113` [V] | 운영중 (로컬) |
| 사용량 | judge와 inbox 지표만 집계, CLI만 제공 | `src/usage.rs:46-165` [W]. model_usage 사용 0건 [V] | 부분 |
| Tracker (Linear 클론) | 스키마와 UI 클라이언트만 있음 | `migrations/0044_tracker.sql`, `src/lib.rs`에 모듈 없음 [V] | 없음 (dead code) |

## 2. 자율형 에이전트 플랫폼 기준 갭

| 갭 | 영향 | 확장 지점 file:line | 필요한 DB 변경 |
|---|---|---|---|
| **G1. 24/7 선제 작업: 트리거가 시간뿐** (webhook·이벤트 없음) | 외부 이벤트에 반응해 작업을 시작할 수 없음 | `src/cron/scheduler.rs:1295` `claim_job(require_due=false)`, wake Notify `:586` [C]. `src/multiplexer/router.rs:358` [A]. 웹 클라이언트는 이미 `/api/webhooks`를 호출함 `web/src/lib/api.ts:596` [W] | `triggers`/`events` 테이블 (source, payload, consumed_at) |
| **G2. kibitzer가 제안만 하고 실행 안 함** | 판단까지만 하고 lane을 띄우지 못함 | `kibitzer.rs:340-361`의 Decision을 `cli::start`에 연결하고 guard 검사 [L]. 상한 `kibitzer.rs:229-238` [I] | `kibitzer_state`를 runtime 생성에서 migration으로 옮김 (`kibitzer.rs:182-191` [M]) |
| **G3. lead가 바쁘면 시스템 이벤트를 버림** | cron이나 선제 이벤트가 유실됨 | `src/multiplexer/actor.rs:360-364` [V] | 대기 큐 테이블 (선택) |
| **G4. drain이 lane runner에 닿지 않음** | 배포·재시작 중에도 lane이 새로 실행됨 | `runner.rs:1060` 부근에 drain 검사 추가 [L], [V] grep | 없음 |
| **G5. 장기 실행 상한** | 기본 24턴(env로 조정 가능 [V]), interactive 턴 30분 [A] | `runner.rs:1005,1241`, `omo_backend.rs:181` | lane에 budget과 checkpoint 컬럼 |
| **G6. 전용 실행환경 없음** | 모든 실행이 호스트 Mac에서 이루어짐. sandbox, VM, egress 통제 없음 | `appserver_url`을 세션별로 결정 `omo_config.rs:40` / `omo_backend.rs:207` [A]. `terminal.rs:346` [S]. `dashboard_runtime.rs:115-138` [W]. `ExternalDriver` `runner.rs:1172-1181` [L]. `CronTaskExecutor` `scheduler.rs:466` [C] | `computers`/`sandboxes` (owner, url, status), 세션과 sandbox 바인딩 |
| **G7. 커넥터·OAuth 계층 없음** | Gmail은 CLI에 계정 하드코딩, MCP 설정 없음, credential vault 없음 | `src/main.rs:1082` MCP 설정 로더 [V]. `src/tools/mcp.rs:24-27` bearer [T]. `InboxSource` `src/inbox/mod.rs:116` [I]. 자식 프로세스 env 제거 목록 `terminal.rs:709-719` [S] | `connectors`, `credentials` (암호화, scope, 만료), `mcp_servers` |
| **G8. 승인이 없거나 동기식** | agent turn은 기본 자동 승인 [V]. terminal은 120s 지나면 실패, 오프라인 대기 큐 없음. lane과 kibitzer는 승인 요청 자체를 거부 [V] | `omo_backend.rs:1456-1470`, `terminal.rs:290-296`, `approval.rs:74,130,149` [S], `runner.rs:661` [V], `src/storage/mod.rs:9-18` pending_writes 일반화 [T] | `approval_requests` (상태, risk, actor, 만료, 결정 기록). `approval_allowlist`에 ttl, actor, reason 추가 [V] |
| **G9. 승인 정책 단계가 없음** | 커넥터, 결제, 메시지 발송에 대한 risk 등급 없음. publishing 목록은 하드코딩 | `src/cron/remediate.rs:17-21,214-217` [C], `src/tools/terminal.rs:14` [S], `src/inbox/proposals.rs:539-571` [I] | `approval_policies` |
| **G10. memory가 세션 단위·lexical·쓰기 tool 없음** | 다른 채널에서 배운 사실을 공유하지 못함. turn에 memory가 주입되지 않음 | `store.rs:111` WHERE 절 [V]. `src/tools/`에 memory tool 추가 [M]. `actor.rs:1344` 주입 [M]. triage 문맥 `triage.rs:100-123` [I] | `memories`에 owner_id/scope, embedding(sqlite-vec), importance, ttl 추가 |
| **G11. 결과를 memory에 다시 쓰지 않음** | 승인이나 기각 결과에서 배우지 못함 | settle 성공 시점 `settle.rs:171-173` [L], `src/inbox/proposals.rs:468-478` [I] | 위 G10과 같음 |
| **G12. 멀티채널: Discord만 양방향** | iMessage와 Kakao는 읽기만, SMS·Telegram·Slack 없음, cron 전송은 u64만 받음 | `dashboard.rs:557` 라우팅 [V], `main.rs:1171` [CH], `scheduler.rs:2357` [V], `src/inbox/sources/imessage.rs:13-32` [CH] | 플랫폼별 대상 테이블. inbox CHECK 완화 [V]. 채널을 가리지 않는 owner identity 매핑 (`identities`) |
| **G13. 승인 UI와 pairing이 Discord에만 있음** | 다른 채널에서는 승인할 방법이 없음 | `src/discord/approval.rs:229,239` [V], `src/discord/pairing.rs:70-90` [CH] | `paired_users`를 u64 대신 platform + handle로 |
| **G14. 실시간 음성 없음** | 통화나 실시간 대화 불가 | `src/voice/pipeline.rs:258-266`에 TTS/LLM 구현, `src/voice/mod.rs:23` 리스너 연결 [CH]. 중복 전사 정리 `attachments.rs:259` / `adapter.rs:1689` | 없음 (선택: voice_sessions) |
| **G15. 단일 owner 하드코딩** | 다중 사용자나 테넌트 불가 | `proposals.rs:10` [V], `kibitzer.rs:21` [V], `mail.rs:9` [V], `src/sessions/status_reply.rs:36` [A] | 모든 owner 관련 테이블에 `users`/`tenant_id` |
| **G16. dashboard 인증 없음** | 원격·모바일에서 접근 불가 | `dashboard.rs:108,140,784` [W][V] | `users`/`auth_sessions` |
| **G17. 비용·감사 기록 없음** | 자율 행동을 추적할 수 없음. `model_usage`를 쓰는 코드 없음 [V]. 로그는 메모리 링 버퍼 2000개 | `src/dashboard.rs:40,285` [W], 백엔드 turn 완료 시점 | `model_usage` writer 추가, `action_audit` |
| **G18. tracker가 dead code** | agent 작업 보드가 없음 | `src/lib.rs`에 `pub mod tracker`와 api.rs 추가 [T][V], `web/src/tracker/api.ts:39` [W] | 0044는 이미 있음 |
| **G19. scrub과 injection 방어가 좁음** | 커넥터로 들어온 콘텐츠의 prompt injection을 막지 못함 | `src/judge/mod.rs:19-63`, `src/security/scan.rs:135,162` [S], `triage.rs:298` | 없음 |
| **G20. Tirith가 기본 fail-open** | 무인 운영에 위험 | `src/main.rs:1060` [S] | 없음 |

## 3. 문서와 코드 불일치, 리포트 간 충돌

1. **`docs/AGENTS_DELEGATE.md`가 낡았습니다** [D][V]. 69, 132, 136, 155, 177, 185행이 settle loop, queued retry, kibitzer가 없다거나 검증되지 않았다고 씁니다. 실제로는 셋 다 연결되어 있습니다: `runner.rs:1033,1086,1093` [V], `kibitzer.rs:147` [V].
2. **README의 승인 timeout 900s** (README.md:105, [D] SECONDARY) 는 tool registry 경로에만 맞습니다 (`tools/mod.rs:70`). terminal tool은 120s입니다 (`terminal.rs:69`) [V].
3. **README는 approvals가 동작한다고 설명하지만** 실제 agent turn은 기본 자동 승인입니다 (`omo_backend.rs:1456-1470`) [V][A]. README의 smart/always/never 설명은 gateway ToolRegistry/terminal 경로에만 해당합니다.
4. **README가 "Voice via Songbird"를 기능으로 내세우지만** (README.md:98, [D]) 리스너와 pipeline을 생성하는 코드가 없습니다 [V][CH].
5. **README가 MCP stdio/SSE를 기능으로 내세우지만** (README.md:104, [D]) 프로덕션에는 등록된 클라이언트가 없습니다 (`main.rs:1082`) [V].
6. **README에 lanes, inbox, jev, session console이 나오지 않습니다** [D] (SECONDARY).
7. **web 클라이언트가 호출하는 API에 대응하는 백엔드가 없습니다.** `web/src/lib/api.ts`의 webhooks, blueprints, Telegram, OAuth, `PUT /api/config` 등과 `/api/tracker/*`가 그렇습니다 [W] (SECONDARY. import 경로를 전부 추적하지는 않았다고 리포트가 밝힘).
8. **docs 리포트는 "token accounting은 owner가 뺐다"고 씀** (dori-grade-agent-env.md:547, [D]). 그런데 `model_usage` 테이블(0039)은 남아 있고 쓰는 코드가 없습니다 [V]. 의도된 잔재로 보입니다.

리포트 사이의 충돌:
- **턴 상한.** [L]은 24턴을 고정 상한으로 적었지만 env로 바꿀 수 있습니다 [V]. [A]가 쓴 "lane 1h"(`omo_config.rs:84`)는 미검증입니다.
- **kibitzer 조용한 시간.** [L][I][M]은 "08:00 KST 이전 금지", [D]는 "08:00-24:00"이라고 씁니다. 같은 제한을 다르게 표현한 것으로 보이지만 끝 시각(24:00)은 미검증입니다.
- **줄 번호 차이.** 실제 소스 기준으로:
  - kibitzer memory query: [M]은 385-387, [L]은 387-389.
  - kibitzer toolsets 설정: [L]은 302, 실제는 300 [V].
  - McpTool 등록: [T]는 1083, 실제는 1082 [V].
  - 자동 승인 분기: [A]는 1438-1452, 실제는 1456-1470 [V].
- **plan 상태.** [D]는 "모든 plan 완료, `discord-omo-session-control.md`만 미완료"라고 합니다. [A]는 세션 제어 기능이 존재한다고 서술합니다. 둘이 다르므로 interrupt와 `--all-scope` 결함([D] plan:41-50)은 미검증으로 둡니다.
- **voice transcription 중복 실행** [CH]. adapter 쪽 경로는 확인했지만 [V], attachments.rs:259 쪽 경로는 미검증입니다.

## 4. 미검증 (SECONDARY)으로 남긴 주요 항목

- `main.rs:955` headless 경로에 dispatcher가 붙지 않는다는 주장 [C]
- inbox watcher 함수(`watchers.rs:240-276`)가 쓰이지 않는다는 주장 [I]
- 스트리밍 테스트가 `adapter.rs:4723`에 있다는 주장 [CH]
- judge의 하루 상한 300/50 [S]
- `custom_settings_json`의 heartbeat 값을 읽는 코드가 없다는 주장 [M]

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/lanes/runner.rs - lane 안 approval 거부(661), settle과 retry 연결(1033-1095), 턴 상한 env(1005)
- /Users/indo/code/project/omon-gateway/src/lanes/kibitzer.rs - 하드코딩된 lead(21-22), 도구 없는 실행(300), 기능 스위치(147)
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - 기본 자동 승인(1456-1470)
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - CompositeDispatcher(557), loopback 전용(108)
- /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs - Discord u64 전송(2357)
- /Users/indo/code/project/omon-gateway/src/memory/store.rs - 세션 단위 lexical 검색(108-133)
- /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs - 바쁠 때 이벤트 버림(360-364)
- /Users/indo/code/project/omon-gateway/src/lib.rs - tracker 모듈 없음
- /Users/indo/code/project/omon-gateway/src/main.rs - MCP 목록 비어 있음(1082), build_cron_tools 미사용(614)
- /Users/indo/code/project/omon-gateway/migrations/0034_inbox.sql - source CHECK
- /Users/indo/code/project/omon-gateway/migrations/0007_approval_allowlist.sql - 빈약한 allowlist 스키마
- /Users/indo/code/project/omon-gateway/docs/AGENTS_DELEGATE.md - 낡은 문서
</files>
<answer>코드는 단일 owner가 Mac 한 대에서 쓰는 Discord 중심 에이전트이고, 핵심은 운영 중입니다. lanes(검증된 완료 판정), cron 자가 수리, inbox triage, judge가 여기에 해당합니다. 자율 플랫폼으로 가는 데 가장 큰 갭은 일곱 가지입니다. 이벤트 트리거가 없고, kibitzer가 제안만 하며, 전용 sandbox가 없고, 커넥터와 OAuth가 없습니다. 또 agent turn 승인이 사실상 꺼져 있고 지속되는 승인 큐가 없으며, memory가 세션 단위·lexical이고, Discord만 양방향 채널입니다. 근거 22개를 소스와 대조했고, 리포트가 틀린 곳은 두 군데입니다. 턴 상한은 고정값이 아니라 env로 바꿀 수 있고, 자동 승인 분기의 줄 번호가 약 18줄 어긋나 있습니다.</answer>
<next_steps>DB 설계는 G6-G8, G10, G15에 걸린 테이블(computers, connectors/credentials, approval_requests, memories scope/embedding, users/tenant)부터 시작하면 됩니다. AGENTS_DELEGATE.md 문서 수정은 후속 작업입니다.</next_steps>
</results>
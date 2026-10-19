<!-- task=st_01a14527 -->
st_01a14527 [completed] model mahoquot/claude-opus-5-5
Ask: wave1 리서치 12개 파일을 테마별 설계 패턴 카탈로그로 축약 - wanted: 테마마다 옵션 비교표와 Mac용 Rust 데몬 권장안, 출처 유지. For you: 12개 파일을 모두 끝까지 읽고 10개 테마로 축약을 끝냈습니다. 파일 간 충돌은 §11에 모았고 SECONDARY/미검증은 본문에 표시했습니다. Now: 없음. Next: 없음.

# 상시 가동 개인 에이전트 게이트웨이: 재사용 패턴 카탈로그 (wave1 축약)

출처가 없는 사실은 뺐습니다. **[S]** = SECONDARY 또는 미검증(헤드라인, 유출 문서, 커뮤니티 일화), **[C]** = 파일 간 충돌입니다.

## 1. 자율 루프 / 하트비트

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| OpenClaw 하트비트: main 세션에서 주기 턴(기본 30m), 할 일 없으면 `NO_REPLY`, `activeHours`+TZ | 사전 게이트(scratch가 비었거나 채널이 죽으면 모델 호출 skip), `isolatedSession`+`lightContext`로 약 100K → 2-5K 토큰, 싼 전용 모델 | 반복 작업은 결국 automations로 옮겨짐 | O (MIT, TS) | https://docs.openclaw.ai/gateway/heartbeat.md |
| 구조화 결과 `heartbeat_respond{notify, notificationText, scratch}` | 텍스트 sentinel보다 파싱이 확실함 | 툴 호출을 지원해야 함 | O | 위와 같음 |
| Hermes `/heartbeat every 10m`: 세션 단위, idle일 때만, 누락분 coalesce, 사용자 메시지 우선, "일 지어내지 말 것" 가드 | 대화를 방해하지 않음 | 대화를 리셋하면 상태가 사라짐 | O (MIT, Py) | https://raw.githubusercontent.com/NousResearch/hermes-agent/b65e078cdc77ebe83979a2d5355179535f340e89/website/docs/user-guide/features/heartbeat.md |
| 이벤트 wake 레이트리밋: 30s 간격, 60s당 5회 flood guard, 바쁘면 defer | 폭주 방지 | — | O | https://docs.openclaw.ai/gateway/heartbeat.md |
| Manus식 루프: `todo.md` 재작성, append-only 컨텍스트, KV-cache 적중률을 핵심 지표로(cached $0.30 vs $3/MTok) | 비용 10배 차이 | 프롬프트 prefix를 안정적으로 고정해야 함 | 패턴만 | https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus |

**권장안:** 하트비트는 "싼 사전 게이트 → 격리·경량 세션 → 구조화 결과" 순서로 둡니다. 할 일이 없으면 LLM을 호출하지 않습니다. 반복 업무는 하트비트에 넣지 말고 cron 잡으로 분리합니다. 프롬프트는 append-only로 쓰고 prefix를 고정합니다.

## 2. 스케줄 · 트리거

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| 단일 `GatewayScheduler`: 호스트 타이머 하나, 슬립 후 각 잡을 1회만 발화(coalesce), 데드라인은 wall time, 상대 지연은 monotonic | Mac 슬립과 시계 보정에 안전 | "Events are not replayed" | O | https://docs.openclaw.ai/concepts/architecture.md |
| Hermes cron: preflight(키·스킬·전달 대상·MCP를 LLM 호출 전 검증, 경보 1회), no-agent 스크립트 모드, cron이 cron을 못 만듦, 전송 중단 시 "unknown"으로 두고 재시도 안 함 | 비용 0인 잡, 중복 전송 없음 | — | O | https://hermes-agent.nousresearch.com/docs/user-guide/features/cron |
| OpenClaw automations: one-shot/반복/cron, `POST /hooks/wake`, `/hooks/agent`, Gmail Pub/Sub, standing intents | 다양한 트리거 | 복잡함 | O | https://docs.openclaw.ai/automation/cron-jobs.md |
| 레인 큐: `session:<key>` 직렬, `main` 동시성 상한, `cron`/`subagent` 레인 분리, `steer/followup/collect/interrupt`, 500ms debounce, 우선순위 head start 15s/30s로 starvation 방지 | 긴 작업이 답장을 막지 않음 | — | O | https://docs.openclaw.ai/concepts/queue.md |
| 내구성 inbox: ack 전에 저장, 크래시 시 "interrupted"로 표시해 재전송 유도, idempotency key와 dedupe 캐시 | 크래시에 안전 | 메모리 큐는 replay 안 됨 | O | https://docs.openclaw.ai/concepts/queue.md, https://docs.openclaw.ai/concepts/architecture.md |
| Poke inbound API `POST /api/v1/inbound/api-message`: 임의 JSON을 에이전트 컨텍스트로 전달 | CI, 모니터링 연동 | 클라우드 전용 | X (패턴만) | https://poke.com/docs/api |
| 트리거 생성 시 "매번 알릴까?"를 한 번 물어 저장 | 소음 정책을 처음에 결정 | — | 패턴 | [S] 유출 프롬프트 https://raw.githubusercontent.com/x1xhlol/system-prompts-and-models-of-ai-tools/116ddce43d456432726e4ff474d3c6be721c4523/Poke/Poke_p2.txt |

**권장안:** tokio 기반 단일 스케줄러(wall time과 monotonic 분리, 누락분 coalesce)를 둡니다. 여기에 SQLite 내구성 inbox(ack 전 저장, idempotency key)와 레인 큐(세션 직렬, cron·omo 레인 분리, 우선순위 aging)를 붙입니다. cron은 preflight, no-agent 모드, 재귀 생성 금지, "unknown" 상태 비재시도를 갖추고, 인증된 `/hooks/*` 웹훅을 받습니다.

## 3. 실행 샌드박스 / 전용 컴퓨터

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| Apple `container`: Linux 경량 VM, OCI | Apache-2.0, 로컬 | macOS 26 필요, Linux 게스트만 | O | https://github.com/apple/container |
| Tart: macOS·Linux VM, OCI 레지스트리 | macOS 게스트 가능 | **[C]** 라이선스가 FSL-1.1-ALv2로, "더 관대하게 바꾸겠다"던 약속과 다름. OpenAI 소유 | O (내부 사용 허용) | https://raw.githubusercontent.com/openai/tart/main/LICENSE, https://cirruslabs.org/ |
| Cua (Spaces, Driver, Lume): 로컬 macOS VM, 포커스를 뺏지 않는 백그라운드 앱 조작, Keyvault | 가장 강한 Mac 네이티브 선택지 | Spaces는 FSL-1.1-MIT | O (개인 무료) | https://github.com/trycua/cua |
| seatbelt/bubblewrap + Unix 소켓 프록시(Anthropic sandbox-runtime) | 파일시스템과 네트워크를 동시에 격리, 권한 프롬프트 84% 감소 | 프록시 직접 운영 | O (Apache-2.0) | https://www.anthropic.com/engineering/claude-code-sandboxing, https://github.com/anthropics/sandbox-runtime |
| E2B: 메모리째 pause, resume 약 1s, paused는 TTL 없음 | 빠른 warm 재개 | Linux 클라우드, 과금 | 일부 (BYOC는 Enterprise) | https://docs.e2b.dev/sandbox/persistence.md, https://e2b.dev/pricing.md |
| Daytona: VM fork와 hot snapshot | 기능 풍부 | macOS는 서드파티 use.computer 경유. self-host는 미검증 [S] | ? | https://www.daytona.io/docs/en/sandboxes.md |
| 브라우저: Steel(Apache-2.0, Docker), Stagehand(MIT) / Browserbase·Anchor는 클라우드 전용 | 프로필 지속, HITL pause | 클라우드형은 세션 데이터가 외부로 감 | Steel O | https://docs.steel.dev/llms.txt, https://github.com/browserbase/stagehand, https://docs.anchorbrowser.io/llms.txt |
| OpenClaw 샌드박스: 기본 off, 도구 실행만 Docker 등으로 이동 | 단순함 | 문서 스스로 "not a perfect security boundary"라고 함 | O | https://docs.openclaw.ai/gateway/sandboxing.md |
| 봇 전용 macOS 사용자 | 저비용 격리 | 약한 격리 | O | https://docs.openclaw.ai/channels/imessage.md |

**권장안:** 백엔드를 `Sandbox` trait(create, exec, snapshot, pause, resume, kill)으로 추상화합니다.
- 기본 실행(omo 코딩): 작업마다 Apple `container` Linux VM을 새로 띄우고, 결과를 diff로 머지합니다.
- GUI 작업: Tart 또는 Lume macOS VM에 Cua Driver를 붙입니다.
- 공통: 모든 레인에 seatbelt와 데몬 소유 egress 프록시를 적용합니다. 레인마다 TTL, idle pause, 하드 kill을 둡니다.
- 베이스 이미지는 자체 레지스트리에 미러링합니다(Tart 소유권이 바뀌었기 때문).

## 4. 자격증명 · 결제

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| 자격증명 주입 egress 프록시(OneCLI): `HTTPS_PROXY`만 설정, host·path 매칭으로 placeholder를 실제 비밀로 치환 | MCP, CLI, curl 어디서 호출해도 강제됨 | 정당한 접근 권한의 오용은 못 막음. `ee/`는 엔터프라이즈 라이선스 | O (Rust 게이트웨이) | https://github.com/onecli/onecli, https://news.ycombinator.com/item?id=49023427 |
| Agent IAP: allow/deny/ask 규칙, `op://` 참조, Once 기본, 타임아웃 시 deny, 해시체인 감사 로그, kill switch | 내 설계와 거의 같음 | 프록시가 모든 비밀을 메모리에 보유, bearer 토큰, 개발용 | O (Rust, MIT) | https://vpetersson.com/2026/09/23/little-snitch-meets-1password-for-ai-agents/ |
| Bitwarden Agent Access (`aac run`으로 env 주입, Noise 터널) | 비밀이 LLM 컨텍스트에 들어가지 않음 | early preview | O (Rust, Apache-2.0) | https://github.com/bitwarden/agent-access |
| Claude Code web의 git 프록시: 실제 토큰은 밖에 두고, 작업 내용(브랜치 등)을 검증한 뒤 토큰 첨부 | 검증된 패턴 | — | 패턴 | https://www.anthropic.com/engineering/claude-code-sandboxing |
| Valmis: 자격증명 ID를 호스트에 요청, 컨테이너는 인터넷 차단 | egress를 아예 끌 수 있음 | — | O | https://github.com/valmishq/valmis |
| AP2 Open/Closed mandate(VDC) | 지출 정책 모델로 좋음 | **[C]** v0.2(문서)와 v0.1(서드파티 보고) 불일치, FIDO로 이관 중 | 스펙 | https://raw.githubusercontent.com/google-agentic-commerce/AP2/main/docs/index.md, https://github.com/custena/agent-payment-protocols |
| ACP (OpenAI와 Stripe, beta) | Delegate Payment 스펙 | Instant Checkout 종료 보도 [S] | 스펙 | https://github.com/agentic-commerce-protocol/agentic-commerce-protocol |
| Visa TAP(RFC 9421), Mastercard Agent Pay(SD-JWT) | — | [S] 이해충돌이 공개된 2차 출처뿐 | — | https://github.com/custena/agent-payment-protocols |
| 선불·가상카드에 상한 정책 | HN 실무 합의 | — | O | https://news.ycombinator.com/item?id=47371289 |
| 1Password Claude 연동 | — | [S] WSJ 헤드라인만 확인 | — | https://news.ycombinator.com/item?id=48933975 |

**권장안:**
- 데몬에 Agent IAP식 Rust egress 프록시를 내장합니다. 비밀은 Keychain, `op://`, `aac` 같은 포인터로만 다루고, 디스크에 캐시하지 않습니다.
- 레인 단위로 grant를 주고 각 grant에 만료를 둡니다. 판단이 필요한 요청은 채팅으로 ask를 보내고, 응답이 없으면 deny합니다.
- 해시체인 감사 로그를 남기고 `/panic` 명령을 둡니다.
- 결제는 AP2 Open mandate 형태의 자체 정책 객체(상한, 가맹점, 만료)와 가상카드로 처리합니다. AP2·ACP wire 포맷은 상대 가맹점이 요구할 때 도입합니다.
- MCP OAuth는 issuer별로 토큰을 바인딩하고 CIMD를 우선 사용합니다. DCR은 피합니다.

## 5. 에이전트 간 프로토콜

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| A2A v1.0 (2026-03-12): `/.well-known/agent-card.json`, 인증된 extended card, 스킬별 OAuth scope, 상태 머신, push/stream | 활발함, PKCE와 device code 지원 | v1.0에 breaking change. 거버넌스는 미검증 [S] | O | https://github.com/a2aproject/A2A/releases/tag/v1.0.0, https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/enterprise-ready.md |
| MCP 2026-07-28: stateless, `server/discover`, tasks는 확장으로 분리, `input_required` MRTR, `traceparent` | 도구 노출의 표준 | Sampling, Roots, Logging, HTTP+SSE, DCR deprecated | O | https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/changelog.mdx |
| ANP (did:wba, E2EE) | 탈중앙 신원 | 인가·협상·결제는 draft, 상호운용 미검증 | O | https://raw.githubusercontent.com/agent-network-protocol/AgentNetworkProtocol/main/README.md |
| Entra Agent ID: 에이전트별 신원, sponsor, 단명 신원, OBO | 위임·감사 모델 | MS 종속, Agent 365 라이선스 | X (모델만) | https://learn.microsoft.com/en-us/entra/agent-id/what-are-agent-identities |
| Poke식 "프런트 에이전트 + 이름 붙은 stateful 서브에이전트" 병렬 fan-out | UX 단순 | [S] 유출 프롬프트와 제3자 추론 | 패턴 | https://www.philschmid.de/the-rise-of-subagents |

**권장안:**
- MCP 2026-07-28 서버로 게이트웨이 도구를 노출합니다. `input_required`가 오면 채팅으로 소유자에게 질문합니다.
- A2A 서버는 기본 off로 두고, peer 토큰은 레인·스킬에 매핑합니다. peer 입력은 untrusted로 취급합니다.
- omo 에이전트마다 TTL과 sponsor를 기록한 단명 토큰을 발급합니다(Entra 모델을 로컬로 흉내).
- `traceparent`를 레인 끝까지 전파합니다. ANP는 보류합니다.

## 6. 메모리

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| OpenClaw: Markdown(`USER.md`, `MEMORY.md`, 일자 파일, `DREAMS.md`) + SQLite 하이브리드 인덱스, 압축 전 flush, untrusted는 승격 금지 | 단순, 이식성 높음 | — | O | https://docs.openclaw.ai/concepts/memory.md |
| Letta MemFS: git 리포의 Markdown, 쓰기마다 커밋, dreaming은 worktree에서, 벡터 없음 | 버전·diff·복원 | review 모드도 사용자 승인은 아님 | O | https://docs.letta.com/concepts/memfs/index.md, https://docs.letta.com/configuration/memory/index.md |
| Letta sleep-time agent: 비동기 통합, 강한 모델 사용 | 대화 지연 없음 | — | O | https://www.letta.com/blog/sleep-time-compute/ |
| Graphiti: bi-temporal 사실, 삭제 대신 invalidate, episode provenance | 모순 처리 | Neo4j 계열 필요, 작은 모델은 추출 실패 | O | https://github.com/getzep/graphiti |
| Mem0: ADD-only 단일 패스, semantic + BM25 + entity + 시간 | 간단 | 누적 drift. 벤치 점수는 자체 보고 [S] | O (Apache) | https://github.com/mem0ai/mem0 |
| Supermemory: 자동 망각, static/dynamic 프로필, 로컬 바이너리 | 오프라인 가능 | "#1" 주장은 자체 보고 [S] | O | https://github.com/supermemoryai/supermemory |
| ChatGPT: saved와 history 분리, 버전 복원, 민감정보는 요청 시에만, Temporary Chat | UX 기준점 | 소스 대화를 지워야 완전 삭제 | X | https://help.openai.com/en/articles/8590148-memory-faq (Wayback 2026-01-08) |

**권장안:** git으로 관리하는 Markdown(루트 인덱스는 항상 주입)에 SQLite FTS5, 로컬 임베딩, 태그를 결합한 하이브리드 검색을 붙입니다. 시간에 따라 바뀌는 사실은 `valid_from`/`valid_to` 테이블에 둡니다.
- dreaming은 cron 레인에서 worktree로 돌리고, 결과는 diff 제안으로만 냅니다. 소유자가 "Remember: X? 👍/👎"로 승인해야 반영됩니다.
- 모든 사실에 원본 메시지 provenance를 남기고, untrusted 출처는 승격하지 않습니다.
- scope(work/personal/repo)와 TTL을 두고, 민감정보는 opt-in으로만 저장합니다.
- `/remember`, `/forget`, temporary 모드, 내보내기를 기본 기능으로 둡니다.

## 7. 선제성 · 알림 정책

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| LangChain 인터럽트 3종(notify/question/review), Agent Inbox 스키마(`allow_*` 4비트, accept/edit/response/ignore) | 채널 독립 envelope | Slack 백로그 문제 때문에 inbox UI를 따로 만듦 | O | https://www.langchain.com/blog/introducing-ambient-agents, https://github.com/langchain-ai/agent-inbox/blob/f1616f3e7998e7e4fd574545b9558554d88d9c49/README.md |
| EAIA triage 버킷(no/notify/act), cron 수집 | 소스별 정책 파일 | — | O | https://github.com/langchain-ai/executive-ai-assistant/blob/main/README.md |
| 자율성 5단계(operator, collaborator, consultant, approver, observer) | 레인별 다이얼 | — | 개념 | https://arxiv.org/abs/2506.12469 |
| PRISM 비용 비대칭 게이트(오경보 22.78% 감소), yes/no 토큰 확률(macro-F1 +0.249), 약 11ms 경량 게이트 | 침묵 비용 0 | 자체 보고 수치, 재현 안 됨 | O | https://arxiv.org/abs/2602.01532, https://arxiv.org/abs/2609.07099, https://arxiv.org/abs/2605.30152 |
| Cowork: 예약 작업은 초안까지만 만들고 미발송, 결정은 폰 푸시, ask가 기본이고 위험 시에만 체크인은 opt-in | 검증된 UX | 클라우드 | X | https://claude.com/blog/cowork-web-mobile, https://claude.com/blog/cowork-is-now-claude |
| 소유자 DM 전용 기본값, 그룹은 금지 | 오발송 방지 | — | O | https://docs.openclaw.ai/gateway/heartbeat.md |
| 아침·저녁 브리핑, Pulse식 야간 다이제스트 | 습관이 됨 | Pulse 가치는 논쟁 중 | O | https://hn.algolia.com/api/v1/search?query=Martin%20AI%20assistant%20iMessage&tags=story&hitsPerPage=8 (2024), https://news.ycombinator.com/item?id=48062680 |
| 리액션 이모지로 승인 | 원탭 | [S] 유출 프롬프트 | 패턴 | Poke_p2.txt (위 URL) |

**권장안:**
- 모든 소유자 대상 메시지를 `notify | question | review` 타입 envelope로 보내고, pending은 SQLite에 저장합니다.
- 렌더링은 채널별로 합니다. Discord는 버튼, iMessage와 KakaoTalk은 키워드나 tapback을 씁니다.
- 승인은 action+args 해시에 바인딩해서, 내용을 고치면 기존 승인이 무효가 되게 합니다.
- 알림 게이트는 규칙과 소형 모델 yes/no 확률 순으로 두고, 시간대·채널별 임계치를 적용합니다. 억제한 알림은 로그로 남기고 dedupe합니다.
- `/inbox`와 일일 다이제스트를 제공하고, 레인마다 자율성 레벨을 저장합니다.

## 8. 보안 · 프롬프트 인젝션

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| Lethal trifecta(사적 데이터, untrusted 입력, 외부 통신) 회피 | 명확한 판단 기준 | 유용성 손실 | — | https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/ |
| Rule of Two(A untrusted, B 민감, C 상태 변경)에서 세션당 최대 2개, 3개면 HITL | 상태 변경까지 포괄 | **[C]** Meta는 "safe"를 "lower risk"로 바꿈. Willison은 A+C도 위험하다고 반박 | — | https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/ |
| CaMeL: 신뢰된 쿼리에서 제어 흐름을 뽑고, capability 태그로 데이터 흐름 제한 | 증명 가능한 보안 | 과제 성공률 77% vs 84% | O (코드 공개) | https://arxiv.org/abs/2503.18813 |
| 탐지 분류기 | — | 적응 공격이 12개 방어를 90% 이상 우회 | — | https://simonwillison.net/2025/Nov/2/new-prompt-injection-papers/ |
| 결정론적 정책(Cedar), 런타임 scope 계약 | 모델 비의존 | — | O | https://securetrajectories.substack.com/p/openclaw-rm-rf-policy-as-code [S], https://arxiv.org/abs/2610.12463 |
| Poke: 이메일과 자동화 결과는 승인 없이 행동하지 않음 | 단순 | [S] 유출 프롬프트 | 패턴 | Poke_p1(유출) |

사건 기록:
- Cline 인젝션으로 OpenClaw가 사용자 PC에 설치됨: https://www.theverge.com/ai-artificial-intelligence/881574/cline-openclaw-prompt-injection-hack
- OpenClaw chat-history spoofing: https://news.ycombinator.com/item?id=46973193
- CVE-2026-33579: https://nvd.nist.gov/vuln/detail/CVE-2026-33579
- Cowork 파일 유출 데모 [S, 미열람]: https://www.promptarmor.com/resources/claude-cowork-exfiltrates-files
- Comet 인젝션: https://brave.com/blog/comet-prompt-injection/
- OWASP Agentic Top 10 항목명은 미검증 [S]

**권장안:**
- 레인마다 A/B/C 프로필을 선언하고, 데몬이 A+B+C 조합 세션을 거부합니다.
- inbox 레인은 untrusted로 두고 비밀, egress, 쓰기를 모두 막습니다. 결과는 크기 제한이 있는 타입 구조체로만 특권 레인에 넘깁니다.
- cron과 inbox 흐름은 CaMeL-lite로 운영합니다. 계획은 결정론적 코드로 고정하고, LLM은 슬롯만 채웁니다. email에서 온 taint가 붙은 데이터는 승인 없이 send로 갈 수 없습니다.
- 소유자 인증은 메시지 내용이 아니라 발신자 신원으로 합니다.
- 플러그인과 MCP는 pin과 hash로 고정합니다.
- lockdown 브레이커를 둡니다. 낯선 도메인, 전송 폭주, 자격증명 경로 읽기 같은 신호가 나오면 전체를 read-only로 전환합니다.
- 분류기는 텔레메트리 용도로만 씁니다.

## 9. 음성 · 기기

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| 기기를 thin client로: R1에 Hermes를 올리고 Tailscale 전용, 오프라인 웨이크워드 | 검증된 해킹 사례 | 루팅 필요 | O | https://github.com/sidmohan0/pomi |
| LiveKit 음성 스킬 | 모델 자유 | — | O | https://github.com/ShayneP/rabbit-r1-livekit-skill |
| Siri App Intents/Shortcuts(iOS 27 `system.open`) | OS 통합 | 개발자 블로그만 확인 [S]. Siri 평판 나쁨 | 앱 필요 | https://emredegirmenci.substack.com/p/iOS-27-app-intents-journey |
| 로컬 모델 + 클라우드 escalation(Cactus) | 지연 감소, 프라이버시 | — | O | https://github.com/cactus-compute/cactus |
| 펜던트 전사 ingest와 이식 가능한 KB | 벤더 리스크 회피 | — | O | https://thoughts.jock.pl/p/limitless-pendant-eu-ban-what-i-did-with-voice-data [S] |

환경 리스크: Apple이 AI 에이전트 때문에 Full Disk Access를 강화합니다(기사 본문은 미열람 [S], https://techcrunch.com/2026/10/02/apple-says-its-tightening-macos-full-disk-access-controls-due-to-new-risks-from-ai-agents/).

**권장안:**
- Tailscale 위에 `voice` 레인(WS 엔드포인트, 텍스트 입출력, 기기 키로 신원 고정)을 둡니다.
- Shortcuts 브리지는 보조 수단입니다.
- 데몬은 전용 launchd 사용자로 돌리고 TCC 권한은 최소로 줍니다. `doctor` 명령으로 FDA, Accessibility, Automation 권한을 점검합니다.
- GUI 조작은 작업 단위 승인, kill switch, 액션 수 상한 아래에서만 허용합니다.

## 10. 한국 생태계

| 선택지 | 장점 | 단점 | 자가호스팅 | 출처 |
|---|---|---|---|---|
| Kakao PlayMCP: OAuth2 mcp-gateway `https://playmcp.kakao.com/mcp`, toolbox당 최대 10개 서버, OTT 교환(`/api/v1/auths/otts:exchange`) | 공식 경로, 약 500개 도구 | 토큰을 평문 JSON에 저장하는 예시 | 클라이언트 O | https://playmcp.kakao.com/llms/mcp-connection-guide.md, https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december |
| KakaoTalk 개인 채팅 읽기 API | — | 공식 API 미발견 [S, 배경지식] | X | https://developers.kakao.com |
| KakaoTalk 채널 봇이나 "나에게 보내기"를 발신 전용으로 | 가능성 높음 | 미검증 [S] | O | 위와 같음 |
| Kakao "AI for Everyone": 예약 전 조건과 금액 재확인 | 2단계 커밋 근거 | 2026-12 출시 예정 | X | https://en.sedaily.com/technology/2026/10/14/kakao-to-launch-ai-for-everyone-service-in-december |
| Naver: 쇼핑 데이터 외부 비공개 [S, 헤드라인], AI탭 1천만 사용자 | — | API 경로 없음 | 브라우저 레인 | https://news.google.com/rss/search?q=네이버+에이전트+쇼핑+예약+네이버페이+when:120d&hl=ko&gl=KR&ceid=KR:ko |

규제 [S, 헤드라인만]:
- AI 기본법 2026년 1월 시행
- 금융위의 신용정보법·망분리 완화 방침
- 개인정보위의 에이전트 개인정보 기준 마련
- Kakao "개인정보 레벨" 제도와 SPIRIT 윤리 원칙

출처: https://news.google.com/rss/search?q=AI+기본법+에이전트+개인정보위원회+에이전트+when:180d&hl=ko&gl=KR&ceid=KR:ko

KAIST 세미나는 프롬프트 인젝션을 최대 위협으로 꼽고, 활동 로그와 책임 추적을 권고했습니다: https://en.sedaily.com/technology/2026/10/15/ai-agents-that-cannot-tell-commands-from-data-are

**권장안:**
- PlayMCP를 MCP upstream으로 붙입니다. OTT 교환과 refresh는 네이티브로 구현하고, 토큰은 Keychain에 둡니다.
- KakaoTalk은 발신 채널로만 쓰고, 수신(ingest)은 지원하지 않는 것으로 명시합니다.
- 결제와 예약은 2단계 승인을 거칩니다(조건과 금액 재확인 후 소유자 탭).
- Naver는 소유자 세션 브라우저 레인으로 처리합니다.
- 메모리에 개인정보 등급 scope(공개, 개인, 금융)를 둡니다.

## 11. 파일 간 충돌 및 불확실 항목

- **[C] "Pickle"의 정체가 파일마다 다름:**
  - x-a2a: pickle.com, 한국어 랜딩 페이지, "Pickle 1" AR 글래스와 Memory OS (https://pickle.com/)
  - x-credentials: Pickle Browser, Nu Terra Labs, 비공개 배포 (https://picklebrowser.com/)
  - x-korea: 미해결, "피클플러스" 컨소시엄만 확인
  - 세 파일 모두 에이전트 간 기능은 미검증입니다.
- **[C] Limitless:** x-memory는 Meta 인수(2025-12-05, TechCrunch)와 Rewind 앱 차단(HN 댓글 [S])을 적었습니다. x-voice_device는 FT 기사, Pendant 판매 중단과 약 1년 지원(사용자 글 [S]), EU 금지(블로그 [S])를 적었습니다. 인수 사실은 일치하고 후속 조치는 출처마다 다릅니다.
- **[C] Wrtn 기업가치:** 약 1조 원 vs 약 1.4조 원(서울경제)로 엇갈립니다.
- **[C] AP2 버전:** v0.2와 v0.1. **[C] Tart:** 관대한 라이선스 약속과 실제 FSL. **[C] Poke 호스팅 도메인:** `.poke.site`와 `.poke.com`.
- **[C] 샌드박스 철학:** OpenClaw는 기본 off(https://docs.openclaw.ai/gateway/sandboxing.md)이고, Anthropic은 파일시스템과 네트워크 격리가 둘 다 필수라고 봅니다. 권장안은 후자를 따릅니다.
- **[S] Anthropic 서드파티 하네스 과금 변경과 철회:** HN 출처만 있습니다(https://news.ycombinator.com/item?id=47844269).
- **미확인:** Hark, Hey Boo, Sesame의 2026년 현황, OpenAI 1차 문서(ChatGPT agent, Tasks).

<results>
<files>
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-openclaw.md - 하트비트, 스케줄러, 레인 큐, 내구성 inbox, 메모리의 1차 근거
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-poke.md - 메시징 네이티브 UX, inbound API, untrusted 이메일 규칙(유출 [S])
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-chatgpt_agent.md - Cowork 초안 미발송과 승인 모드, Valmis 프록시
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-manus.md - KV-cache와 todo.md 루프, wide fan-out, 벤더 리스크
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-sandbox_infra.md - Apple container, Tart, Cua, E2B, Daytona, Steel 비교
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-credentials.md - 주입 프록시(OneCLI, Agent IAP, aac), AP2·ACP, MCP OAuth
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-a2a.md - A2A v1.0, MCP 2026-07-28, ANP, Entra
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-memory.md - MemFS, sleep-time, Graphiti, Mem0, ChatGPT memory
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-proactive.md - 인터럽트 타입, Agent Inbox, 자율성 레벨, 게이트 연구
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-security.md - trifecta, Rule of Two, CaMeL, sandbox-runtime, 사건
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-voice_device.md - 기기 thin client, FDA 강화, App Intents
- /Users/indo/code/project/omon-gateway/.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-korea.md - PlayMCP, Kakao·Naver·SKT, 규제 헤드라인
</files>
<answer>
위 1-10절이 테마별 옵션표와 Mac용 Rust 데몬 권장안이고, 11절이 충돌·불확실 목록입니다. 핵심 골격은 다음과 같습니다.
- 단일 tokio 스케줄러, SQLite 내구성 inbox, 레인 큐
- 사전 게이트를 거친 하트비트
- Rule of Two로 레인 프로필 강제
- 데몬 내장 egress와 자격증명 주입 프록시
- `Sandbox` trait(Apple container, Tart/Cua)
- git Markdown 메모리와 승인 기반 dreaming
- `notify | question | review` envelope 기반 채팅 승인
- 상태 없는 MCP 서버, 기본 off인 A2A
- PlayMCP upstream과 발신 전용 KakaoTalk
</answer>
<next_steps>
상위 세션은 11절의 미해결 항목 중 Pickle의 정체, KakaoTalk 수신 API, Apple FDA 정책 원문을 wave2 검증 대상으로 넘기면 됩니다. 이 외에는 추가 작업이 필요 없습니다.
</next_steps>
</results>
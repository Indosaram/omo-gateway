# 에이전트 작업 현황용 칸반 도구 조사 (2026-10-16)
## 요약 및 추천

> 아래 추천은 리드가 공식 페이지에서 직접 2차 확인한 사실(verify-lead.md)만 근거로 함. 전체 비교표는 다음 절.

**목적별 추천 (2026-10-16 기준)**

| 목적 | 1순위 | 대안 | 근거 |
|---|---|---|---|
| 무료·오픈소스로 에이전트 작업을 칸반 보드로 관리 | Backlog.md (MIT, Markdown 파일 기반 칸반, TUI+로컬 웹 보드, MCP/CLI) | Vibe Kanban (Apache-2.0, 칸반+에이전트 실행, 단 서비스 종료 예정) | README·GitHub API |
| 휴대폰에서 에이전트 상태 확인·승인 | Happy (MIT, iOS·Android 무료) | Omnara (Apache-2.0, iOS·Android, 플랫폼 무료·셀프호스팅 가능) | App Store/Play, 공식 가격 페이지 |
| 여러 에이전트 병렬 실행 + 모바일 + 팀 협업(유료 감수) | Conductor (Free / Pro $50/월, iOS 앱은 Pro 이상) | Orca (MIT 무료, iOS·Android 컴패니언) | conductor.build/pricing |
| 기존 이슈 트래커에 에이전트를 붙이기(SaaS) | Linear (Free 이슈 250개 / Basic $10 / Business $16, Linear Agent) | GitHub Copilot 클라우드 에이전트 + GitHub Mobile (Free / Pro $10 / Pro+ $39 / Max $100) | linear.app/pricing, github.com/features/copilot/plans |
| 셀프호스팅 범용 칸반에 MCP로 에이전트 연결 | Plane (OSS, Free / Pro $6 / Business $13, Cloud·Self-hosted) | Vikunja·WeKan·Kanboard (무료 OSS 셀프호스팅) | plane.so/pricing, 각 공식 사이트 |
| 에이전트 전용 칸반 + 셀프호스팅(팀용) + 모바일 | Multica (스타 약 5.3만, Docker Compose·Helm, 셀프호스팅 무료, 소스공개 라이선스. 단 iOS 앱은 소스 빌드만·App Store 미출시) | AgentsMesh (CE 무료·Docker, iOS 앱, 클라우드 $14/사용자/월, BSL-1.1) | github.com/multica-ai/multica README·GitHub API(리드 직접 확인), verify-wave2.json |
| 에이전트 전용 칸반 + 모바일 웹/PWA(가벼운 셀프호스팅) | Agent Orchestrator (Apache-2.0, 스타 약 1.3만, LAN·Tailscale QR 페어링) | Paca (Apache-2.0, Docker/K8s, PWA) | 노드 보고서 + 2차 판정 |

**주의할 점**
- Vibe Kanban: 운영사 Bloop이 2026-04-10 종료를 발표했고 README에 'sunsetting'이 명시됨. 오픈소스(Apache-2.0, 2026-09-19 마지막 push)로 계속 쓸 수는 있으나 유지보수 리스크가 큼.
- Omnara: 과거 '월 10세션 무료 / $9' 요금은 폐지됨. 현재는 플랫폼 $0 + 관리형 자원 종량제.
- Cursor: 2026-10 기준 요금표가 Grok 번들 형태(Free / Plus $8 / Premium $30 / Super $100 / Ultra $200 / Teams $30+/사용자)로 바뀌었음. 모바일·셀프호스팅 머신 문서는 존재.
- Shrimp Task Manager(마지막 push 2025-08-21)는 사실상 정체 상태.

## 비교표

열 설명: 칸반 뷰 = 작업·에이전트를 보드로 볼 수 있는지. 검증 = '2차 확인'은 리드가 공식 페이지에서 직접 재확인한 행, 그 밖에는 조사 노드 신뢰도.

| # | 도구 | 분류 | 칸반 뷰 | 모바일 | 가격 | 셀프호스팅 | 라이선스 | 에이전트 연동 | 상태 | 검증 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | [1Code](https://1code.dev) | 에이전트 전용 | 예 | PWA | 무료(OSS) / Pro $20/월 | 가능(로컬 데스크톱, SQLite) | Apache-2.0 | Claude Code·Codex CLI, REST API, MCP 클라이언트, 웹훅 | GitHub 아카이브됨(비활성) | 2차: 모바일·셀프호스팅·라이선스 확인 |
| 2 | [5dive](https://5dive.ai) | 에이전트 전용 | 예 | Telegram 봇·미니앱(iOS·Android) | 무료(OSS) / 관리형 VM 약 $15/월 | 가능(Linux systemd) | MIT | CLI·MCP 플러그인, Claude Code·Codex 헤드리스 실행 | 활성(매일 커밋) | 2차: 모바일·셀프호스팅·라이선스 확인 |
| 3 | [Agent 007](https://github.com/bill10/agent-007) | 에이전트 전용 | 예 | 반응형 웹+모바일 음성 통화 | 무료(OSS) | 가능(Node/Python 데몬) | MIT | 헤드리스 CLI 오케스트레이션; Claude Code·Codex | 초기·활발(★5, v0.69.4.4) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 4 | [Agent Dashboard (dustinblack)](https://github.com/dustinblack/agent-dashboard) | 에이전트 전용 | 부분 | 반응형 웹·SSH TUI | 무료(OSS) | 가능(Podman·Docker Compose) | Apache-2.0 | tmux·PTY 제어, OTLP 텔레메트리; Claude Code·Codex 등 | 업데이트 중(~17 stars) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 5 | [Agent Deck](https://github.com/asheshgoplani/agent-deck) | 에이전트 전용 | 부분 | 반응형 웹 + 텔레그램/슬랙 봇 | 무료(OSS) | 로컬 전용 | MIT | CLI/TUI·MCP 관리; Claude Code·OpenCode·Codex·Gemini·Pi | v1.16+ 활성, ★1,092 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 6 | [Agent Kanban (saltbo)](https://github.com/saltbo/agent-kanban) | 에이전트 전용 | 예 | 반응형 웹(모바일 테스트 완료) | 무료(셀프호스트) / 클라우드 가격 미확인 | 가능(Cloudflare Workers·Node.js) | FSL-1.1-ALv2 | REST API(OpenAPI), 에이전트의 작업 클레임·리뷰 제출 | 활성(2026년 10월 커밋) | 2차: 셀프호스팅 확인 |
| 7 | [Agent Kanban PM (kronos3610)](https://github.com/kronos3610/agent-kanban-pm) | 에이전트 전용 | 예 | 반응형 웹(로컬 네트워크) | 무료(OSS) | 가능(pip·SQLite 로컬) | MIT | MCP 서버·REST/WebSocket, Claude·Codex 오케스트레이터 | v0.8.0 릴리스(~8 stars) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 8 | [Agent of Empires](https://www.agent-of-empires.com/) | 에이전트 전용 | 아니오 | PWA·반응형 웹 | 무료(OSS) | 가능(로컬 바이너리·CityHall 서버) | MIT | tmux CLI로 Claude Code·Codex 등 12종 이상 지원, ACP, REST API, 웹훅 | 활발(v1.20.0, 2026-10-15) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 9 | [Agent Orchestrator](https://ao-agents.com) | 에이전트 전용 | 예 | iOS·Android 앱 | 무료(OSS) / AO Cloud 유료(가격 미확인) | 가능(로컬 데몬·원격 VM) | Apache-2.0 | ao CLI·REST API, Claude Code·Codex 등 25+ 하네스, ACP | 매우 활발(v0.14.2) | lead 2차 확인(README·GitHub API) |
| 10 | [Agent Teams](https://agentteams.live) | 에이전트 전용 | 예 | 반응형 웹(개발 중) | 무료(OSS), 내장 무료 모델 / BYOK | 가능(로컬 데스크톱·Docker) | AGPL-3.0 | 내장 MCP 서버, HTTP API, Claude Code·Codex·Cursor 등 | 활발(v2.17.10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 11 | [Agent Teams AI](https://agentteams.live) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용(데스크톱 앱) | AGPL-3.0 | Claude Code·Codex·Cursor 등 연동, 내장 MCP 서버 | 활발(스타 2,265+) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 12 | [Agent4Discord](https://github.com/raravel/Agent4Discord) | 에이전트 전용 | 아니오 | Discord 앱 | 무료(OSS) | 가능(로컬 Node.js) | MIT | Claude Code CLI, Discord 버튼·메뉴 인터랙션 | 2026.10 푸시, 활발 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 13 | [AgentBox](https://agent-box.sh/) | 에이전트 전용 | 아니오 | 반응형 웹(네이티브 앱 없음) | 무료(OSS), 클라우드는 사용자 계정 종량제 | 가능(로컬 Docker/원격 SSH/VPS 허브) | MIT | CLI·REST API·SSE; Claude Code, Codex, OpenCode, Pi, OpenClaw | 활발한 개발 중(2026) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 14 | [AgentKanban](https://agentkanban.io) | 에이전트 전용 | 예 | 반응형 웹 | 무료 / Pro $10/사용자/월 | 불가(SaaS) | 상용 | MCP 서버(Claude Code·Codex·Cursor), VS Code 확장, REST API, 웹훅 | 활발(2026 릴리스) | 2차: 공식 근거 없음 |
| 15 | [AgentRQ](https://agentrq.com) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) / 클라우드 무료 티어 | 가능(Docker) | AGPL-3.0 | MCP 서버, ACP, Chrome 확장(Claude Code·Gemini CLI·Cursor) | 활발(스타 1.1k+) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 16 | [AgentsMesh](https://agentsmesh.ai) | 에이전트 전용 | 예 | iOS 클라이언트(SwiftUI)·웹·데스크톱; Android 미언급 | CE 무료 / 클라우드 Team $14/사용자/월 | 가능(Docker) | BSL-1.1 | 모든 CLI 에이전트; Claude Code·Codex·Gemini·Aider, gRPC | 활성, ★2,361, 2026-09 커밋 | lead 2차 확인(README·GitHub API) |
| 17 | [Agetor](https://agetor.dev) | 에이전트 전용 | 예 | 반응형 웹(주로 macOS 데스크톱) | 무료(OSS), GitHub Sponsors | 로컬 전용(macOS 바이너리) | MIT | CLI: Claude Code·Codex·Cursor·Gemini CLI·Antigravity; 훅 비침투 | 활발(v1.0.2, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 18 | [AGINEAR](https://github.com/Legorobotdude/AGINEAR) | 에이전트 전용 | 예 | 반응형 웹(로컬 네트워크) | 무료(OSS) | 가능(Node.js/npx, 로컬) | MIT | stdio MCP·CLI(Claude Code·Claude Desktop·Cursor) | 초기 단계(2026.2 공개) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 19 | [Agno AgentOS UI](https://agno.com) | 에이전트 전용 | 아니오 | 반응형 웹 | 무료(OSS) / 클라우드 무료 티어, 엔터프라이즈 유료 | 가능(Docker Compose, Railway, AWS) | Apache-2.0 | Python SDK, 공식 MCP 서버, REST API, Slack 등 연동 | 42,760★, 활발 | 2차: 셀프호스팅·라이선스 확인 |
| 20 | [AI Agent Board (DanWahlin)](https://github.com/DanWahlin/ai-agent-board) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Node·Docker) | MIT | CLI 어댑터: Copilot CLI·Claude Code·Codex·OpenCode·Hermes·OpenClaw | 활발(2026-09 푸시) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 21 | [AI Maestro](https://ai-maestro.23blocks.com/) | 에이전트 전용 | 예 | 반응형 웹(Tailscale 경유 접속) | 무료(OSS) | 가능(Node.js·Docker·AWS Terraform) | MIT | CLI·AMP 등 프로토콜; Claude Code·Codex·Aider·Cursor | GitHub 822★, v0.81.6, 2026-10 활동 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 22 | [AIF Handoff](https://aif.cutcode.dev) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Node.js·Docker) | MIT | Claude Code·Codex·Cursor CLI 어댑터, 서브에이전트 오케스트레이션 | 활발(★308) | 2차: 가격·라이선스 확인 |
| 23 | [Aizen](https://aizen.win/) | 에이전트 전용 | 아니오 | 없음(macOS 전용) | 무료(GPL) / Pro $5.99/월·$59/년 / 평생 $179 | 로컬 전용 | GPL-3.0 | ACP·MCP 마켓·CLI; Claude Code·Codex·OpenCode·Gemini CLI | v1.0.83 최종, 2.0 개발 중 | 2차: 모바일·가격·라이선스 확인 |
| 24 | [amux](https://amux.io/) | 에이전트 전용 | 예 | iOS 앱·PWA | 무료(OSS) / Cloud $20/사용자/월 | 가능(단일 Rust 바이너리) | MIT + Commons Clause | REST API·CLI; Claude Code·Codex·Gemini CLI·OpenCode | GitHub 534★, 2026-10 활동 | 2차: 모바일·가격·셀프호스팅 확인 |
| 25 | [Aperant](https://aperant.com/) | 에이전트 전용 | 예 | 미확인(공식 근거와 상충: 없음(데스크톱 전용)) | 무료(OSS) / 클라우드 v3.0 가격 미정 | 로컬 전용 | AGPL-3.0 | Claude Code CLI·멀티모델 API·MCP 클라이언트; Linear·GitHub·GitLab | v2.7.6 안정판, v3.0 재작성 중 | 2차: 가격·라이선스 확인 |
| 26 | [Asana AI Studio & AI Teammates](https://asana.com/product/ai/ai-studio) | 에이전트 전용 | 예 | iOS·Android 앱(Asana 앱 경유) | Asana 유료 플랜 필요 + 추가 요청당 $0.50~$0.60 | 불가(SaaS) | 상용 | 앱 내장 에이전트(OpenAI·Anthropic), REST API·웹훅 연동 | 활발(2024 말 출시) | 2차: 라이선스 확인 |
| 27 | [Atlassian Rovo Agents & Rovo Dev / Coding Agent](https://www.atlassian.com/software/rovo) | 에이전트 전용 | 부분 | Jira Cloud iOS·Android 앱, 반응형 웹 | 유료 Jira 플랜 크레딧 포함, 초과 $0.01/크레딧 | 불가(SaaS) | 상용 | 원격 MCP(양방향)·twg CLI·REST; Copilot·Claude Code·Cursor | 활발(빠른 확장) | 2차: 모바일·셀프호스팅·라이선스 확인 |
| 28 | [Atrium](https://getatrium.dev/) | 에이전트 전용 | 예 | iOS·iPadOS 앱(TestFlight 베타) | 무료(얼리 액세스) / 유료 가격 미공개 | 로컬 전용(Linux 헤드리스 데몬) | 상용(어댑터는 오픈소스) | CLI·훅·MCP(하네스 경유); Claude Code·Codex·OpenCode 등 12+ | v0.345.0, 수일 간격 업데이트 | 2차: 공식 근거 없음 |
| 29 | [AutoGen Studio](https://microsoft.github.io/autogen/stable/user-guide/autogenstudio-user-guide/index.html) | 에이전트 전용 | 아니오 | 없음(데스크톱 전용) | 무료(OSS) | 가능(pip, SQLite/PostgreSQL) | MIT(코드)/CC-BY-4.0(문서) | Python SDK, CLI, REST API, MCP 클라이언트(McpWorkbench) | 61,434★, 유지보수 모드(MAF 전환) | lead 2차 확인(공식 페이지) |
| 30 | [Automagik Forge](https://forge.automag.ik) | 에이전트 전용 | 예 | 반응형 웹(Android 앱은 사양 단계) | 무료(OSS) | 가능(npx 로컬 실행) | Apache-2.0 | MCP 서버·CLI·REST, Claude Code·Gemini CLI·Genie | 활성(~92 stars) | 2차: 가격·라이선스 확인 |
| 31 | [Automaker](https://automaker.app/) | 에이전트 전용 | 예 | 반응형 웹·PWA(웹 모드) | 무료 | 가능(로컬·Docker) | 미확인(GitHub NOASSERTION, MIT 아님) | Claude Agent SDK·Codex·Copilot 등; REST/WS API; MCP | 유지보수 중단(as-is 제공) | lead 2차 확인(공식 페이지·GitHub) |
| 32 | [Backlog.md](https://backlog.md) | 에이전트 전용 | 예 | 반응형 웹(기본 127.0.0.1, 터널 필요) | 무료 | 로컬 전용 | MIT | MCP 서버·CLI·JSON 출력; Claude Code·Codex·Gemini CLI·Kiro·Cursor | 활발(v1.53.0, 2026-09) | 리드 직접 2차 확인 |
| 33 | [Cairn](https://try-cairn.com) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS, BYO API 키) | 가능(로컬/사설 서버) | MIT | LLM API(Claude·GPT·Gemini); GitHub App으로 PR 자동 제출 | 정체, ★220, 2025-07 커밋 | 2차: 라이선스 확인 |
| 34 | [Catnip](https://github.com/wandb/catnip) | 에이전트 전용 | 부분 | 반응형 웹·iOS 앱(TestFlight) | 무료(OSS) | 가능(로컬 바이너리·Docker) | Apache-2.0 | CLI 기반, Claude Code 중심(Codex 등은 로드맵) | 원본 404, 미러 유지 | 2차: 공식 근거 없음 |
| 35 | [Caudex](https://caudex.sh/) | 에이전트 전용 | 아니오 | PWA(터널 경유 원격 접속) | 무료(비공개 프리웨어) | 로컬 전용 | 상용(비공개 프리웨어) | Claude Code 훅·statusLine 연동; MCP 관리자; 공개 API 없음 | 활발(v0.1.52) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 36 | [cc-connect](https://github.com/chenhg5/cc-connect) | 에이전트 전용 | 아니오 | 메신저 앱(Telegram·Discord·Slack 등) | 무료(OSS) | 가능(Go 바이너리) | MIT | Claude Code·Codex·Gemini CLI, MCP 도구, 웹훅 | 별 15,959개, v1.5.1-beta.1 | 2차: 모바일·셀프호스팅 확인 |
| 37 | [cc-haha](https://cchaha.ai) | 에이전트 전용 | 아니오 | H5 모바일 웹 + 메신저 봇 | 개인·사내 무료 / 상업 배포는 유료 라이선스 | 가능(데스크톱 로컬) | 소스공개(CSL 1.0) | Claude Code·Codex·DeepSeek·Ollama 등, MCP 관리 | 별 15,025개, 매우 활발 | 2차: 공식 근거 없음 |
| 38 | [CCManager](https://github.com/kbwo/ccmanager) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | MIT | CLI로 Claude Code·Gemini·Codex·Cursor 등 8종 지원, 상태 훅 | 매우 활발(v4.4.7, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 39 | [ccpoke](https://kaida-palooza.github.io/ccpoke/) | 에이전트 전용 | 아니오 | Telegram 앱 | 무료(OSS) | 가능(npm) | MIT | Claude Code·Codex·Cursor CLI, Stop 훅·tmux | 별 103개, 2026.3 푸시 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 40 | [ChatDev 2.0 (DevAll)](https://chatdev.modelbest.cn) | 에이전트 전용 | 아니오 | 없음(웹 콘솔) | 무료(OSS) / 호스팅 SaaS 무료 체험 크레딧 | 가능(Docker Compose) | Apache-2.0 | Python SDK, YAML 워크플로, LiteLLM 다중 LLM | 34,527★, 2026.1 2.0 출시 | 2차: 라이선스 확인 |
| 41 | [Chorus](https://doc.chorus-ai.dev) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker Compose) | AGPL-3.0 | CLI, REST API, Claude Code·Codex·Cursor, git PR 연동 | 1,199★, 활발(v0.22.2) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 42 | [Claude Code Agent Monitor](https://hoangsonww.github.io/Claude-Code-Agent-Monitor/) | 에이전트 전용 | 예 | 반응형 웹(+macOS·Windows 앱) | 무료(OSS) | 가능(로컬 Node.js+SQLite) | MIT | Claude Code·Codex·Cursor 훅 스트리밍, MCP | 활발(★1.08k) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 43 | [Claude Code on the web](https://code.claude.com/docs/en/claude-code-on-the-web) | 에이전트 전용 | 아니오 | Claude 모바일 앱 Code 탭·웹 | Claude 유료 구독(Pro/Max/Team/Enterprise) 포함 | 일부(조직 자체 실행 환경 라우팅 가능) | 상용 | Claude Code(공식); GitHub 저장소 연동 | 활발(공식) | 리드 직접 2차 확인 |
| 44 | [Claude Code Telegram Bot (overwirehq/claude-code-telegram)](https://github.com/overwirehq/claude-code-telegram) | 에이전트 전용 | 아니오 | Telegram 앱 | 무료(OSS) | 가능(pip/uv) | MIT | Claude Code CLI 서브프로세스, 웹훅 리스너 | v1.3.0, 별 2,815개 | 2차: 모바일·가격·셀프호스팅 확인 |
| 45 | [Claude Code UI (CloudCLI)](https://cloudcli.ai) | 에이전트 전용 | 예 | 반응형 웹·PWA (네이티브 앱 개발 중) | 무료(OSS) / 클라우드 €7~€39/월, 엔터프라이즈 별도 | 가능(npx·Docker 샌드박스·PM2) | AGPL-3.0-or-later | Claude Code·Codex·Cursor·Gemini CLI·OpenCode; REST·MCP | 활발(~14.1k★, v1.37.4) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 46 | [Claude Remote Runner](https://remote-claude.shaniai.tech) | 에이전트 전용 | 아니오 | PWA | 무료(OSS) | 가능(Docker) | MIT | Claude Code CLI 래퍼+훅, WebRTC 스트리밍 | 별 11개, 2026.4 푸시 | 2차: 가격·라이선스 확인 |
| 47 | [Claude Squad](https://smtg-ai.github.io/claude-squad) | 에이전트 전용 | 아니오 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | AGPL-3.0 | CLI(tmux 세션)로 Claude Code·Codex·Aider·Gemini 실행; MCP/API 없음 | 활발(v1.0.20, 2026-08-20) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 48 | [Claude Task Viewer](https://github.com/L1AD/claude-task-viewer) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 로컬 전용 | MIT | ~/.claude 세션 파일 직접 모니터링(읽기 전용); Claude Code 전용 | 베타, 2026-02 이후 정체 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 49 | [Clave](https://github.com/antasphere/clave) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | MIT | Claude Code·Codex·Antigravity·Pi CLI; 플러그인; MCP 브리지 | 매우 활발(v2.4.0, v3 베타) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 50 | [Claw-Kanban](https://github.com/GreenSheep01201/Claw-Kanban) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Node.js 22+·Docker) | Apache-2.0 | Claude Code·Codex·Gemini·OpenCode·Copilot·Antigravity | 활발(v1.0.3, ★72) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 51 | [ClawBridge](https://clawbridge.app/) | 에이전트 전용 | 아니오 | PWA | 무료(OSS) | 가능(Docker) | MIT | OpenClaw 런타임, WebSocket 스트림, 프로세스 API | v1.2.3, 별 231개, 속도 둔화 | 2차: 공식 근거 없음 |
| 52 | [ClickUp Brain & Agents](https://clickup.com/brain) | 에이전트 전용 | 부분 | iOS·Android 앱 | 무료 / $7 / $12 /사용자/월(연간) | 불가(SaaS) | 상용 | @Brain 멘션·AI 자동화·외부 MCP 커넥터 소비 | 활발(상용, Brain² 출시) | 리드 직접 2차 확인 |
| 53 | [CliDeck](https://clideck.dev/) | 에이전트 전용 | 아니오 | PWA·반응형 웹(E2E 암호화 릴레이) | 무료(OSS) | 로컬 전용 | MIT | PTY CLI 래퍼·OTLP 수신·clideck ask; Claude Code·Codex·Gemini 등 | 활발 / v2.4.0 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 54 | [Cline Kanban](https://github.com/cline/kanban) | 에이전트 전용 | 예 | 반응형 웹(LAN) | 무료(OSS) | 로컬 전용 | Apache-2.0 | CLI(Claude Code·Cline CLI·Codex 자동 감지) | 리서치 프리뷰(스타 1.3k+) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 55 | [cmux](https://cmux.com/) | 에이전트 전용 | 아니오 | iOS 앱(TestFlight 베타) | 무료(OSS) / Founder's Edition $100 일회성 | 가능(로컬 macOS 앱) | GPL-3.0-or-later | 소켓 API·CLI·Claude Code 훅·tmux 심, 다수 에이전트 스킬 | 활발(스타 2.8만, 일일 빌드) | 2차: 모바일·가격·셀프호스팅 확인 |
| 56 | [CNVS](https://cnvs.dev/) | 에이전트 전용 | 아니오 | iOS 앱(CNVS Pocket) | 유료 일회성 $99→$129→$169(구독 없음) | 로컬 전용 | 상용 | CLI 서브프로세스 제어·트랜스크립트 파싱, 8종 에이전트 | 활발(v1.5.2, 2026-09) | 2차: 가격·라이선스 확인 |
| 57 | [CodeAgent Mobile](https://www.codeagent-mobile.com) | 에이전트 전용 | 부분 | iOS·Android 앱 + 웹 대시보드 | 무료 / Pro $19.99/월($191.99/년) | 일부(Pro: 자체 서버, 릴레이는 SaaS) | 상용 | IDE 확장·CLI(ACP); Claude Code·Codex·Copilot 등 | 활발(2026-10 업데이트) | 2차: 모바일 확인 |
| 58 | [CodeAgentSwarm](https://www.codeagentswarm.com) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(베타 중 Pro 포함) / 정식 Pro 미확인 | 로컬 전용 | 상용(미확인) | 병렬 코딩 에이전트; Muse Code·Pi·Devin CLI 지원 | 오픈 베타(v2.4.0) | 2차: 모바일·가격·셀프호스팅 확인 |
| 59 | [CodeLayer](https://humanlayer.dev/code) | 에이전트 전용 | 예 | 반응형 웹 | 무료(3인까지) / Pro $100/사용자/월 / 엔터프라이즈 별도 | 로컬 데몬 가능; 클라우드는 엔터프라이즈만 | Apache-2.0(저장소) | MCP 서버·CLI·API, Claude Code·Codex·Copilot·pi | 활발(주간 릴리스, v0.183.0) | lead 2차 확인(공식 페이지·GitHub) |
| 60 | [CodeNomad](https://github.com/NeuralNomadsAI/CodeNomad) | 에이전트 전용 | 아니오 | PWA·반응형 웹(OpenTunnel 원격), 네이티브 앱 없음 | 무료(OSS) | 가능(데스크톱·npm 서버·Docker) | MIT | OpenCode V2 데몬/CLI 연동, MCP 서버 점검·도구 토글 | 활발(v0.20.0, 2026-10, 2.6k★) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 61 | [Coder Agents (formerly Coder Tasks)](https://coder.com/docs/ai-coder/agents) | 에이전트 전용 | 아니오 | 반응형 웹 | Community 무료(동시 5개) / Premium 영업 문의 | 가능(K8s·Docker·바이너리) | AGPL-3.0(코어)+상용 확장 | MCP 거버넌스·CLI·REST·웹훅; Agent Relay(Claude Code·Cursor) | 활발(주간 릴리스) | 2차: 셀프호스팅·라이선스 확인 |
| 62 | [Coder Mux (Xum)](https://github.com/coder/mux) | 에이전트 전용 | 아니오 | 반응형 웹(xum server 경유) | 무료(OSS) | 가능(로컬 앱·헤드리스 서버) | AGPL-3.0 | MCP·ACP·CLI·REST/WS; Claude·GPT·Grok·Ollama 등 | 활발(v0.31.0, 2026-10) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 63 | [Codex Monitor](https://www.codexmonitor.app/) | 에이전트 전용 | 아니오 | iOS 앱(TestFlight), 반응형 웹; Android 없음 | 무료(OSS) | 로컬 전용(Tauri 앱·헤드리스 데몬) | MIT | Codex app-server(JSON-RPC), gh CLI, MCP 상태 점검 | 활발(v0.7.67, 2026-10, 4.3k★) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 64 | [Collattice (MrBildo)](https://github.com/MrBildo/collattice) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(단일 바이너리) | MIT | 내장 MCP(HTTP); Claude Code·Cursor·Codex | 활발(★8, 2026-10 v3.2.3) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 65 | [Conductor](https://conductor.build) | 에이전트 전용 | 예 | iOS 앱(Pro 이상) | 무료 / Pro $50/월 / Teams $60/사용자/월 | 로컬 전용(클라우드 셀프호스팅 미제공) | 상용 | MCP·REST API·CLI·웹훅; Claude Code·Codex·Cursor·OpenCode | 활발(v0.91.0, 2026년 10월) | 리드 직접 2차 확인 |
| 66 | [Constellagent](https://constellagent.vercel.app/) | 에이전트 전용 | 아니오 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용(macOS Electron) | ISC | Claude Code CLI·훅, Codex 훅, GitHub GraphQL/gh CLI | 활동 중(v1.0.0, 217★) | 2차: 가격 확인 |
| 67 | [CrewAI AMP & Studio](https://crewai.com/amp) | 에이전트 전용 | 아니오 | 반응형 웹 | 무료(OSS) / AMP 무료 티어, 유료·엔터프라이즈 문의 | 코어 가능 / AMP 온프레미스는 엔터프라이즈만 | MIT(코어) / 상용(AMP) | Python SDK, CLI, REST API, 문서 MCP, Claude Code 스킬 | 59,750★, 매일 활발히 개발 | 2차: 라이선스 확인 |
| 68 | [Crystal (stravu/crystal) / Nimbalyst](https://nimbalyst.com/) | 에이전트 전용 | 예 | iOS·Android 앱 + 반응형 웹 콘솔 | 무료(OSS) / Teams $20/사용자/월(베타 무료) | 로컬 전용(팀 동기화는 Cloudflare) | MIT | MCP·REST/WS·CLI(nim); Claude Code·Codex·Gemini 등 | 활발(v0.81.6, 2026-10) | 2차: 라이선스 확인 |
| 69 | [Cursor Cloud Agents (formerly Background Agents) / Cursor Agents Dashboard](https://cursor.com/agents) | 에이전트 전용 | 부분 | iOS·iPadOS 앱, Android는 PWA | Premium $30/월 이상 필요(Free·Plus 제외), 모델 토큰 별도 | 일부(하이브리드, 셀프호스트 워커) | 상용 | Desktop·CLI·REST API·SDK·MCP, Slack·GitHub·Linear 트리거 | 활발(상용, API 공개 베타) | 2차: 공식 근거 없음 |
| 70 | [Dev 3.0](https://github.com/h0x91b/dev-3.0) | 에이전트 전용 | 예 | 반응형 웹(LAN/VPN 경유) | 무료(OSS) | 로컬 전용(tmux·git worktree) | Apache-2.0 | CLI/PTY: Claude Code·Codex·Gemini CLI·OpenCode | 312★, 활발(v1.59.0) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 71 | [Dev Boards (Agent Relay)](https://github.com/GumbyEnder/agent-relay) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) / 호스팅 클라우드 베타 | 가능(Node.js·Docker) | MIT | 5동사 REST API·토큰, TUI CLI, MCP 지원 | 초기(v0.3.x) 활발 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 72 | [Devin (Cognition) Sessions Dashboard](https://app.devin.ai) | 에이전트 전용 | 부분 | 반응형 웹 | 무료 / Pro $20/월 / Max $200/월 / Teams 최소 $80/월 | 일부(하이브리드, Outposts) | 상용 | REST API v3, 공식 MCP 서버, CLI, Handoff(Claude Code·Codex·Cursor) | 활발(2026-10 릴리스) | 2차: 공식 근거 없음 |
| 73 | [Dify](https://dify.ai) | 에이전트 전용 | 아니오 | 반응형 웹(배포 앱만, 관리 콘솔 데스크톱) | 무료(OSS) / Pro $59/월, Team $159/월 | 가능(Docker Compose) | Apache-2.0 파생(source-available, 상업 제한) | REST API, 웹훅, 플러그인, 도구 연동 | 158,596★, 매우 활발 | lead 2차 확인(공식 페이지) |
| 74 | [Dispatch Board](https://www.dispatchboard.ai) | 에이전트 전용 | 예 | 반응형 웹 | 무료 베타(대기자 등록) | 불가(SaaS) | 상용 | 클라우드 에이전트 러너, REST API, 웹훅 | 베타/대기자 | 2차: 공식 근거 없음 |
| 75 | [dmux](https://dmux.ai/) | 에이전트 전용 | 아니오 | 없음(터미널 전용) | 무료(OSS) | 로컬 전용(npm 설치) | MIT | CLI; Claude Code, Codex, Gemini CLI 등 12개 이상 에이전트 | 활발(v5.11.1, 2026) | 2차: 가격·라이선스 확인 |
| 76 | [Dorothy](https://dorothyai.app/) | 에이전트 전용 | 예 | 없음(데스크톱 전용, Telegram·Slack 원격) | 무료(OSS) | 로컬 전용 | MIT | Claude Code·Codex·Gemini CLI 등, MCP(칸반 등), REST API | 최종 릴리스 1.2.9(2026-07) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 77 | [Ductor](https://ductor.dev) | 에이전트 전용 | 아니오 | Telegram·Matrix·Slack 앱 | 무료(OSS) | 가능(pip·Docker) | MIT | Claude Code·Codex·Gemini·Antigravity CLI·Grok Build | 별 460개, 2026.7 푸시 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 78 | [Emdash](https://emdash.sh) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS); 팀용 Emdash Cloud 별도(가격 미확인) | 로컬 전용(원격 SSH 머신 지원) | Apache-2.0 | CLI 에이전트 37종(ACP 23종), MCP 설정 동기화, 이슈 트래커 12종 | 활발(v1.2.8, 2026-10) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 79 | [Factory.ai Droids (Software Factory & Missions)](https://factory.ai) | 에이전트 전용 | 부분 | 반응형 웹(네이티브 앱 없음) | Pro $20/월 / Plus $100 / Max $200 / Teams $60+$40/석 | 엔터프라이즈만(하이브리드·에어갭) | 상용 | MCP·REST API·CLI·SDK·ACP, Linear/Jira·Slack 위임 | 활발(주간 릴리스, 2026.10) | 2차: 라이선스 확인 |
| 80 | [Flux (sirsjg)](https://github.com/sirsjg/flux) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker/Bun/Node, Git 동기화) | MIT | MCP(stdio·HTTP) 25+ 도구, CLI·REST; Claude Code·Codex | 활발(★104, 2026-08 v0.6.0) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 81 | [Fusion](https://runfusion.ai/) | 에이전트 전용 | 예 | iOS·Android 앱, PWA | 무료(OSS, LLM API 키 별도) | 가능(Docker·npm·Homebrew) | MIT | CLI·REST/SSE API·ACP·MCP; Claude Code·Codex·Gemini 등 | 매우 활발 / 0.78.0-beta.7 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 82 | [Gander](https://github.com/weellio/gander) | 에이전트 전용 | 아니오 | PWA·/phone 웹·Telegram 알림 | 무료(OSS) | 가능(로컬 Node.js) | MIT | Claude Code 훅, gander-wrap(Codex·Gemini·aider), Telegram 봇 | 별 10개, 초기 릴리스 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 83 | [Gas Town](https://github.com/steveyegge/gastown) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS), 모델 비용 별도 | 가능(Go 바이너리·Docker) | MIT | CLI 중심(gt), Claude Code·Codex 등, 웹 대시보드 | 활발(2026 후반) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 84 | [GitHub Copilot Agents Panel / Agent HQ (Mission Control)](https://github.com/copilot/agents) | 에이전트 전용 | 부분 | iOS·Android 앱, 반응형 웹 | Copilot 유료 플랜 포함; 세션·조정은 AI 크레딧 차감 | 불가(SaaS) | 상용 | 웹 UI·MCP(assign_copilot_to_issue)·REST/GraphQL API | 활발(2026년 10월 업데이트) | 리드 직접 2차 확인 |
| 85 | [GitHub Copilot Coding Agent (Cloud Agent)](https://github.com/features/copilot/agents) | 에이전트 전용 | 아니오 | iOS·Android 앱, 반응형 웹 | 유료 Copilot 포함: Pro $10/월~, Business $19/사용자/월 | 불가(SaaS) | 상용 | MCP·REST/GraphQL API·gh CLI; Claude·Codex 위임(프리뷰) | 활발(상용, 잦은 업데이트) | 리드 직접 2차 확인 |
| 86 | [GitHub Mobile (Copilot Workspace / Agent PRs)](https://github.com/mobile) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 앱 무료 / Copilot $10/월, $19·$39/사용자/월 | 불가(SaaS, GHES 일부 지원) | 상용 | Copilot·Copilot Workspace, REST/GraphQL API, Actions 웹훅 | 매우 활발(1~2주 업데이트) | 2차: 모바일 확인 |
| 87 | [Google Jules](https://jules.google) | 에이전트 전용 | 부분 | 반응형 웹(네이티브 앱 없음) | 무료 / Pro ~$19.99/월 / Ultra ~$249.99/월 | 불가(SaaS) | 상용 | REST API·CLI·MCP(선별)·GitHub 앱; 외부 에이전트는 API/CLI로 연동 | 활발(2025.8 베타 종료) | 2차: 셀프호스팅 확인 |
| 88 | [Granular](https://granular.build/) | 에이전트 전용 | 예 | 없음(데스크톱 전용, Telegram 알림) | 무료 / Pro $12.99/사용자/월, 평생 $259 | 로컬 전용 | 상용 | Claude Code·Codex·Gemini CLI, MCP 커넥터 1,400+, roadmap.md | 상용 활동 중(v0.3.30) | 2차: 모바일·셀프호스팅 확인 |
| 89 | [Hamloy](https://hamloy.com) | 에이전트 전용 | 예 | PWA | 무료(1개 프로젝트) / Pro €8/월(€80/년) | 불가(SaaS) | 상용 | MCP(claim·tick 등)·OAuth; Claude Code·Codex·Cursor·ChatGPT | 운영 중(Smithery 등록) | 2차: 가격 확인 |
| 90 | [Happy (formerly Happy Coder / slopus/happy)](https://happy.engineering) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 무료 | 가능(Docker 릴레이·헤드리스 데몬) | MIT | CLI 래퍼·데몬, ACP·MCP; Claude Code·Codex·Grok | 매우 활발(★24k, 2026-10 푸시) | 리드 직접 2차 확인 |
| 91 | [Height](https://height.app) | 에이전트 전용 | 부분 | iOS·Android 앱(과거 제공) | 무료 / Team $6.99/월(연간) / Business $11.99/월 | 불가(SaaS) | 상용 | REST API(API 키·OAuth)·웹훅, 공식 MCP 없음 | 2025-09-24 서비스 종료 | 2차: 공식 근거 없음 |
| 92 | [Herdr](https://herdr.dev/) | 에이전트 전용 | 아니오 | 없음(SSH 터미널 클라이언트로만 접근) | 무료(OSS) / Herdr Cloud 대기자 명단(가격 미공개) | 로컬 전용(단일 바이너리, SSH 원격 연결) | Apache-2.0 | 소켓 JSON API·CLI·스킬; Claude Code·Codex 등 22+ 에이전트 | 활발(v0.9.5, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 93 | [Herdr Connect](https://github.com/Tomyail/herdr-connect) | 에이전트 전용 | 아니오 | iOS 앱(TestFlight 베타) | 무료(OSS) | 로컬 전용 | Apache-2.0 | Herdr 데몬 JSON-RPC/WebSocket; Claude Code·Codex·aider | TestFlight 베타, 별 19개 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 94 | [Herdr Mobile Relay](https://github.com/0cv/herdr-mobile-relay) | 에이전트 전용 | 아니오 | PWA | 무료(OSS) | 가능(Herdr 플러그인) | AGPL-3.0 | Herdr 플러그인, Herdr 서버 프로토콜 연동 | v0.22.10, 별 297개 | 2차: 모바일·셀프호스팅 확인 |
| 95 | [Hermes Agent Kanban (NousResearch)](https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban) | 에이전트 전용 | 부분 | 커뮤니티 WebUI로 반응형 웹(코어 CLI) | 무료(OSS) | 로컬 전용(Python·SQLite) | MIT | hermes kanban CLI·A2A·kanban_* 툴 콜 | 매우 활발(★254k) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 96 | [Hermes KanbanWebUI](https://github.com/PriuS2/HermesKanban) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(로컬 Python/Node 서버) | Apache-2.0 | Hermes Agent CLI·공유 SQLite DB 직접 접근 | 활발(★39) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 97 | [hubd](https://hubd.net) | 에이전트 전용 | 부분 | 반응형 웹 | 무료(OSS) | 로컬 전용(파일 기반) | MIT | MCP 서버·CLI·파일 훅; Claude Code·Cursor·Aider | 초기(★1, v0.9.54) | 2차: 가격·라이선스 확인 |
| 98 | [is.team](https://is.team) | 에이전트 전용 | 예 | 반응형 웹 | 무료(3석) / Starter $8~Max $39/월(얼리어답터) | 불가(SaaS) | 상용 | MCP(Claude·Cursor·ChatGPT), HMAC 웹훅, GitHub·Slack 연동 | 활발(2026 PH 출시) | 2차: 가격·라이선스 확인 |
| 99 | [jat](https://jat.tools/) | 에이전트 전용 | 아니오 | 반응형 웹(Tailscale/SSH 경유) | 무료(OSS) | 가능(로컬·원격 VPS) | MIT | jt 태스크 CLI(JSON)·메일박스·슬래시 명령; Claude Code·Codex·Aider 등 | 활발(1인 개발, 태그 릴리스 없음) | 2차: 셀프호스팅 확인 |
| 100 | [Jean](https://jean.build/) | 에이전트 전용 | 부분 | 반응형 웹(Web Access) | 무료(OSS) | 가능(Docker·헤드리스 서버) | Apache-2.0 | Claude·Codex·Cursor 등 CLI, MCP, REST/WebSocket API | 매우 활발(v1.0.11, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 101 | [Kaiban Board (KaibanJS)](https://www.kaibanjs.com) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(npm·Docker) | MIT | JS SDK, React 컴포넌트, LangChain.js·OpenAI·Anthropic 연동 | 유지보수 중(스타 1.4k+) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 102 | [KaibanJS](https://www.kaibanjs.com/) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(npm 패키지) | MIT | LangChainJS, OpenAI·Claude·Gemini·로컬 LLM | 활발(★1.48k) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 103 | [Kanban Pro (Good Guy Apps)](https://goodguyapps.com) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(가입·구독 없음) | 로컬 전용 | 미확인 | Markdown 파일 직접 읽기/쓰기; 내장 터미널로 Claude Code·Codex 등 | Vibe Kanban 종료 후 대안으로 언급 | 2차: 모바일·가격·셀프호스팅 확인 |
| 104 | [kanban-mcp (multidimensionalcats)](https://github.com/multidimensionalcats/kanban-mcp) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(pipx/Docker, 로컬) | MIT | MCP(stdio) 40+ 도구; Claude Code·Cursor·Codex·OpenCode | 활발(★83, 2026-03 v0.2.0) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 105 | [kanban-md](https://github.com/antopolskiy/kanban-md) | 에이전트 전용 | 예 | 없음(SSH로 TUI 접속만) | 무료(OSS) | 로컬 전용(Go 바이너리) | MIT | CLI·스킬(Claude Code·Codex·Cursor·OpenClaw), 클레임 잠금 | 활발(v0.39.0, 2026.10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 106 | [KanBots](https://kanbots.dev) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(OSS) / 클라우드 $19/석/월 | 로컬 전용(Electron), 클라우드는 호스팅 | MIT / 상용(클라우드) | 11개 CLI(Claude Code·Codex 등)+ACP, MCP 서버 | 활발(v1.2.0, ★615) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 107 | [Kander](https://github.com/dualface/kander) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Go 바이너리+SQLite) | MIT | Claude Code·Codex 등 CLI, 리뷰어 에이전트, worktree | 매우 활발(v0.9.7, ★148) | 2차: 라이선스 확인 |
| 108 | [Kandev](https://github.com/kdlbs/kandev) | 에이전트 전용 | 예 | 반응형 웹(모바일 원격 접속 지원) | 무료(OSS) | 로컬 전용 | AGPL-3.0 | Claude Code·Codex·Aider·OpenCode CLI 어댑터, MCP | 954★, 활발 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 109 | [KanVibe](https://github.com/rookedsysc/kanvibe) | 에이전트 전용 | 예 | 반응형 웹(터미널 조작은 데스크톱) | 무료(OSS) | 미확인(공식 근거와 상충: 가능(start.sh·Docker)) | AGPL-3.0 | Claude Code·Gemini CLI·Codex·OpenCode 훅, worktree·tmux | 활발(v1.4.0, ★147) | 2차: 가격·라이선스 확인 |
| 110 | [Kiro (AWS)](https://kiro.dev) | 에이전트 전용 | 부분 | iOS 앱(TestFlight 프리뷰); Android는 웹 | 무료 50크레딧 / Pro $20/월 ~ Power $200/월 | 불가(SaaS, 클라이언트만 로컬) | 상용 | MCP·ACP·CLI·REST/GraphQL; Claude Code·Codex·Cursor 연동 | 활발(CLI V3.x 릴리스 중) | 2차: 공식 근거 없음 |
| 111 | [LangGraph Studio & LangSmith](https://smith.langchain.com) | 에이전트 전용 | 아니오 | 없음(macOS 앱·웹 대시보드) | OSS 무료 / LangSmith 무료, Plus $39/사용자/월 | LangGraph 가능 / LangSmith 엔터프라이즈만 | MIT(LangGraph) / 상용(LangSmith) | Python/TS SDK, CLI, REST API, MCP 연동 | 43,332★, 매우 활발 | 2차: 라이선스 확인 |
| 112 | [Maestro](https://maestro-build.com) | 에이전트 전용 | 아니오 | PWA(Maestro Web, 푸시 알림) | 무료 | 가능(로컬 앱, 릴레이 자체호스팅) | 소스 공개 프리웨어 | CLI 병렬 구동; Claude Code·Codex·Cursor | 신생, ★3, 2026-10 커밋 | 2차: 모바일·가격·라이선스 확인 |
| 113 | [Maude](https://maude.pilotdev.fr) | 에이전트 전용 | 아니오 | iPhone·iPad·Android 앱 | 유료 구독(주 $2.99 / 연 $29.99, 무료 체험) | 일부(하이브리드) | 상용 | SSH 원격 데몬; Claude Code·Codex·OpenCode·Grok·Antigravity | 활발(v2.1.0, 2026-10) | lead 2차 확인(공식 페이지) |
| 114 | [MetaGPT & MGX](https://mgx.dev) | 에이전트 전용 | 아니오 | 없음(웹 전용) | 무료(OSS) / MGX 프리미엄 크레딧 구독 | MetaGPT 가능 / MGX 불가(SaaS) | MIT(OSS) / 상용(MGX) | Python SDK, CLI, REST API, Git·웹훅 | 70,873★, 활발(v0.8+) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 115 | [MindFlock](https://mindflock.ai) | 에이전트 전용 | 부분 | 반응형 웹(로컬 대시보드) | 무료(OSS) | 가능(로컬 데스크톱/CLI) | Apache-2.0 | 트래커 API 동기화; Claude Code·Codex·Cursor·OpenCode | 프리릴리스, ★15 | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 116 | [mngr (by Imbue)](https://imbue.com/product/mngr) | 에이전트 전용 | 아니오 | 없음(SSH 터미널 접속만 가능) | 무료(OSS, 컴퓨팅·모델 비용 별도) | 가능(로컬·Docker·SSH·Modal) | MIT | CLI(JSON 출력)·SSH·플러그인; Claude Code·Codex·OpenCode | 활발한 개발 중 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 117 | [monday AI Agents (Platform Agents)](https://developer.monday.com/api-reference/docs/build-on-monday-with-ai.md) | 에이전트 전용 | 예 | iOS·Android 앱(monday 앱 내장), 반응형 웹 | 유료 플랜 필요($12~19/사용자/월)+AI 크레딧 | 불가(SaaS) | 상용 | Platform MCP 도구·GraphQL(프리뷰)·웹훅·Claude Managed Agents | 베타/GA 출시 진행 중(2026) | 2차: 공식 근거 없음 |
| 118 | [Moo Tasks](https://mootasks.dev) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS, 공식 클라우드 무료) | 가능(Docker Compose) | MIT | 내장 MCP(SSE/HTTP); Claude Desktop·Claude Code·Cursor·Cline | 활성, ★45, 2026-06 커밋 | 2차: 라이선스 확인 |
| 119 | [Multica](https://multica.ai) | 에이전트 전용 | 예 | iOS·iPadOS 앱(소스 빌드만, App Store 미출시)·반응형 웹; Android 미언급 | 무료(셀프호스팅) / 클라우드·엔터프라이즈 문의 | 가능(Docker Compose·Helm) | Apache-2.0 변형(소스공개) | 26+ CLI(Claude Code·Codex·Cursor 등), WebSocket 데몬 | 매우 활발(★52.9k, v0.4.44) | lead 2차 확인(README·GitHub API) |
| 120 | [multidimensionalcats/kanban-mcp](https://github.com/multidimensionalcats/kanban-mcp) | 에이전트 전용 | 예 | 반응형 웹(로컬 네트워크) | 무료(OSS) | 가능(npx·Docker) | MIT | MCP(stdio/HTTP) 40+ 도구; Claude Code·Cursor·Codex | 활발(83 stars) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 121 | [Nerve (OpenClaw Nerve)](https://nerve.zone) | 에이전트 전용 | 예 | 반응형 웹·PWA | 무료(OSS) | 가능(원라인 설치, Node.js) | MIT | OpenClaw 데몬 네이티브, WebSocket·음성 명령 | 활발(★867) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 122 | [Nimbalyst](https://nimbalyst.com/) | 에이전트 전용 | 예 | iOS 앱(Android 예정) | 무료(OSS) | 미확인(공식 근거와 상충: 로컬 전용(데스크톱 앱)) | MIT | CLI: Claude Code·Codex·OpenCode·Copilot·Gemini; MCP 클라이언트/서버 | 활발(v0.80.6, 2026-10) | 2차: 모바일·가격·라이선스 확인 |
| 123 | [Nodiom Cloud](https://nodiom.md) | 에이전트 전용 | 예 | 반응형 웹+Slack/Discord 알림 | 무료(월 1만 쓰기) / Pro $49/월 | 일부(코어 엔진 OSS) | MIT(코어)/상용(클라우드) | MCP 17개 도구·REST API; Claude Code·Cursor·Codex | 프라이빗 베타 | 2차: 셀프호스팅·라이선스 확인 |
| 124 | [Notion AI & Custom Agents](https://www.notion.com/product/ai) | 에이전트 전용 | 부분 | iOS·Android 앱(Notion 내), 모바일 웹 | Business 이상 포함 / Custom Agents $10/1,000 크레딧 | 불가(SaaS) | 상용 | Agent REST API·Workers(베타)·Agent Skills API·원격 MCP | 활발(2026 베타/상용) | 2차: 가격·라이선스 확인 |
| 125 | [Offrun](https://offrun.dev/) | 에이전트 전용 | 부분 | 없음(macOS 데스크톱 전용) | 무료(프리웨어) | 로컬 전용 | 상용(프리웨어) | CLI 통합; Claude Code·Codex CLI·Antigravity CLI | 2026-10 Show HN 공개 | 2차: 모바일·가격·셀프호스팅 확인 |
| 126 | [Omnara](https://www.omnara.com/) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 플랫폼 무료, 관리형 자원만 종량제 | 가능(Docker Compose) | Apache-2.0 | MCP 서버·REST API·SDK·CLI; Claude Code·Codex·Cursor | 활발(★2,921, 2026-10 릴리스) | 리드 직접 2차 확인 |
| 127 | [opcode (formerly Claudia, getAsterisk, Gooey)](https://github.com/winfunc/opcode) | 에이전트 전용 | 아니오 | 반응형 웹(opcode-web, 네이티브 앱 없음) | 무료(OSS, API 키/구독 별도) | 가능(로컬 데스크톱·웹서버 바이너리) | AGPL-3.0 | Claude Code CLI 연동, MCP 관리, REST/WebSocket API, 커스텀 에이전트 | 활발(★22.4k, 2026 커밋) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 128 | [OpenAI Agents SDK Tracing](https://platform.openai.com/docs/guides/agents) | 에이전트 전용 | 아니오 | 없음(데스크톱 웹) | 무료(OSS) / 트레이싱 무료, API 토큰 과금 | SDK 가능 / 트레이싱은 OpenAI 클라우드 | MIT(SDK) / 상용(대시보드) | Python·JS SDK, MCP 도구, Responses API, 100+ LLM | 30,063★, 매우 활발(2025 출시) | 2차: 라이선스 확인 |
| 129 | [OpenAI Codex Cloud Tasks Dashboard](https://chatgpt.com/codex) | 에이전트 전용 | 부분 | iOS·Android 앱(ChatGPT 내), 반응형 웹 | ChatGPT 플랜 기반: Plus ₩29,000/월부터 Cloud 포함 | 불가(SaaS), CLI·App Server만 로컬 | 상용 | Codex SDK·App Server(JSON-RPC)·CLI, GitHub/Linear/Slack | 활성 GA 운영 중 | 2차: 공식 근거 없음 |
| 130 | [OpenClaw Nerve](https://github.com/daggerhashimoto/openclaw-nerve) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker) | MIT | OpenClaw 런타임·WebSocket·MCP 브리지 | 활발(2026-10-13 커밋) | 2차: 모바일·가격·라이선스 확인 |
| 131 | [OpenHands](https://openhands.dev) | 에이전트 전용 | 아니오 | 반응형 웹 + Slack 봇 | 무료(OSS) / Cloud 무료+사용량 기반 유료 | 가능(Docker·Helm) | MIT | Docker 샌드박스, REST/WebSocket API, GitHub App, Slack, MCP | 별 91,014개, 매우 활발 | 2차: 셀프호스팅·라이선스 확인 |
| 132 | [OpenHands Agent Canvas (All-Hands-AI)](https://www.openhands.dev) | 에이전트 전용 | 부분 | 반응형 웹(Tailscale/ngrok 경유, 네이티브 앱 없음) | 무료(OSS, BYOK) / 클라우드 개인 무료 / 엔터프라이즈 맞춤 | 가능(npm·Docker·VM·Helm·Modal) | MIT | REST API·WebSocket·ACP(Claude Code·Codex·Gemini 등)·CLI·MCP | 활발(v1.27.1, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 133 | [OpenKanban](https://github.com/TechDufus/openkanban) | 에이전트 전용 | 예 | 없음(터미널 TUI) | 무료(OSS) | 로컬 전용(Go 바이너리) | AGPL-3.0 | Claude Code·Codex 등 CLI, 티켓별 worktree | 활발(★151) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 134 | [OpenTerra (KaiStephens/OpenTerra)](https://github.com/KaiStephens/OpenTerra) | 에이전트 전용 | 미확인 | 미확인 | 무료(OSS) | 가능(소스 실행) | 미확인 | Claude Code 에이전트 | 사실상 중단(마지막 push 2025-07) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 135 | [Orca](https://www.onorca.dev) | 에이전트 전용 | 예 | iOS·Android 컴패니언 앱 | 무료 | 가능(로컬 데스크톱, 릴레이 Docker 자가호스팅) | MIT | CLI·MCP/스킬·JSON-RPC, Claude Code·Codex 등 35+종 | 활발(일일 릴리스, ★93.6k) | 리드 직접 2차 확인 |
| 136 | [Ouijit](https://ouijit.com) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | AGPL-3.0 | CLI(Claude Code·Codex·Aider), 상태 전환 훅, ouijit CLI | 활발(2026) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 137 | [OverClick](https://github.com/ustoppble/overclick) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker Compose) | MIT | MCP 서버(Claude Code·Codex·Gemini CLI 등); REST API | 베타(v0.4.0, 2026-09) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 138 | [Paca](https://paca-ai.org) | 에이전트 전용 | 예 | 반응형 웹·PWA | 무료(OSS) | 가능(Docker Compose·Kubernetes/Helm) | Apache-2.0 | MCP 서버(paca-mcp)·REST·웹훅; Claude Code·Cursor·Codex | 활발(★1,928, v0.19.0) | lead 2차 확인(README·GitHub API) |
| 139 | [Pane](https://runpane.com) | 에이전트 전용 | 아니오 | 휴대폰 브라우저 원격 제어(웹) | 무료(OSS) | 가능(VM/원격 호스트 실행) | AGPL-3.0(공식 페이지 표기) | MCP·CLI(77개 명령)·REST; Claude Code·Codex·Cursor 등 | 활발(거의 매일 릴리스) | 리드 직접 2차 확인 |
| 140 | [Paperclip](https://paperclip.ing) | 에이전트 전용 | 예 | 반응형 웹(모바일 최적화) | 무료(OSS) / Cloud 대기·베타 | 가능(Node.js·Docker) | MIT | Claude Code·Codex·Cursor 등 어댑터, MCP·REST·CLI | 활발(PR 2,700+ 병합) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 141 | [Parallel Code](https://parallelcode.app/) | 에이전트 전용 | 아니오 | Android 앱, PWA(iOS 푸시 지원) | 무료(OSS) | 로컬 전용(선택적 Docker 샌드박스) | MIT | 내장 MCP 서버; Claude Code·Codex·Gemini·Copilot CLI 등 | 활발(v4.0.0, 2026) | 2차: 가격·라이선스 확인 |
| 142 | [PlateSpinner](https://github.com/moridinamael/platespinner) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(로컬 Node.js) | MIT | CLI 스폰; Claude Code·Codex CLI·Gemini CLI, WebSocket 스트리밍 | 초기, ★13, 2026-04 커밋 | 2차: 가격·셀프호스팅·라이선스 확인 |
| 143 | [Platypus](https://platypus.chat) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS), LLM 별도 | 가능(Docker) | MIT | MCP·A2A·REST API·웹훅, 다양한 LLM 제공자 | 활발(2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 144 | [PocketShell](https://pocketshell.app) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 무료 / Pro $4.99 | 일부(호스트 데몬만) | Apache-2.0(호스트)/앱 상용 | WebRTC로 Claude Code·Codex CLI stdio 스트리밍 | v0.x 베타, 별 10개 | lead 2차 확인(공식 페이지) |
| 145 | [Proliferate](https://proliferate.com/) | 에이전트 전용 | 아니오 | Expo 앱 스캐폴드, 반응형 웹 | 무료(OSS) / 클라우드 좌석제 유료(가격 미확인) | 가능(Docker Compose, AWS) | AGPL-3.0 | AnyHarness SDK·MCP; Claude Code·Codex·OpenCode·Cursor·Grok | 활발(v0.4.29, 2026-08) | 2차: 라이선스 확인 |
| 146 | [QodFlow](https://www.qodflow.com) | 에이전트 전용 | 예 | 반응형 웹 | 무료(3명·10작업) / Premium $19/월 / Diamond $49/월 | 불가(SaaS) | 상용 | MCP 서버(Claude Code·Cursor·OpenCode), REST API | 상용 운영 중(2026) | lead 2차 확인(공식 페이지) |
| 147 | [Ralph TUI](https://ralph-tui.com/) | 에이전트 전용 | 아니오 | 없음(터미널 UI 전용) | 무료(OSS) | 가능(로컬/헤드리스 원격 모드) | MIT | CLI 러너(Claude Code·Codex 등), Linear·Jira 트래커, WebSocket | 매우 활발(v0.12.0, 2026-05) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 148 | [Relayd](https://relayd.dev) | 에이전트 전용 | 아니오 | PWA(iOS·Android 홈 화면, 웹 푸시) | 무료 티어 / 상위 플랜 유료(금액 미확인) | 일부(하이브리드) | 상용 | 릴레이 데몬/웹훅; OpenAI Codex CLI 중심 | 운영 중, 2026-01 Show HN | 2차: 공식 근거 없음 |
| 149 | [Replit Agent (Replit Mobile)](https://replit.com/mobile) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 무료 / Core $15/월(연간)·$20/월 / Teams $30/사용자/월 | 불가(SaaS) | 상용 | 자체 Replit Agent 런타임, WebSocket 이벤트 스트림 | 매우 활발(주간 릴리스) | 2차: 모바일·라이선스 확인 |
| 150 | [Roadmap Skill](https://www.npmjs.com/package/roadmap-skill) | 에이전트 전용 | 예 | 반응형 웹(로컬 네트워크) | 무료(OSS) | 로컬 전용(npx) | MIT | MCP 서버·Agent Skills, Claude Code·Codex·Cursor·OpenCode | 활동 중(★81) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 151 | [Roamgate](https://roamgate.dev/) | 에이전트 전용 | 부분 | PWA·반응형 웹 | 무료(OSS) | 가능(Docker) | MIT | Herdr 서버 API; Claude Code·Codex 등 모니터링 | 별 298개, 활발 유지 | 2차: 모바일·가격·라이선스 확인 |
| 152 | [Routa](https://github.com/phodal/routa) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker Compose, 데스크톱 빌드) | MIT | MCP, ACP, A2A, CLI, REST API; Claude Code·Codex | 1,828★, 활발(v0.19.0) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 153 | [Saga MCP](https://www.npmjs.com/package/saga-mcp) | 에이전트 전용 | 예 | 반응형 로컬 웹 | 무료(OSS) | 로컬 전용(SQLite) | MIT | MCP 도구 41개; Claude Code·Cursor·Claude Desktop | 활발(★40, v1.19.0) | 2차: 가격·라이선스 확인 |
| 154 | [Sculptor (by Imbue)](https://imbue.com/product/sculptor) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS, 자체 API 키 사용) | 로컬 전용(Docker·원격 백엔드 지원) | MIT | CLI(sculpt)·로컬 REST API; Claude Code·Codex·Pi 등 | 활발(베타, v0.48.0) | 2차: 모바일·가격·라이선스 확인 |
| 155 | [Shrimp Task Manager](https://cjo4m06.github.io/mcp-shrimp-task-manager) | 에이전트 전용 | 부분 | 반응형 웹(선택적 GUI) | 무료(OSS) | 로컬 전용 | MIT | MCP 서버(Claude Code·Cline·Claude Desktop·Cursor) | 활발(스타 2.1k+) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 156 | [Sortie](https://sortie-ai.com/) | 에이전트 전용 | 아니오 | 반응형 웹 | 무료(OSS) | 가능(Go 바이너리·Docker·Helm) | Apache-2.0 | Claude Code·Codex·Gemini 등 어댑터(ACP), HTTP API, MCP 설정 주입 | 활발(v1.27.0, 2026-10-11) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 157 | [Star-Office-UI](https://github.com/ringhyacinth/Star-Office-UI) | 에이전트 전용 | 부분 | 미확인(공식 근거와 상충: 반응형 웹 + 데스크톱 펫(Tauri)) | 무료(OSS) | 가능(Flask+Phaser) | MIT | OpenClaw 스킬·REST API·Python 푸시 스크립트 | 매우 활발(★7.5k, v1.0.0) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 158 | [Supacode](https://supacode.sh/) | 에이전트 전용 | 아니오 | 없음(macOS 26+ 데스크톱 전용) | 무료(로컬 워크트리 5개) / Basic $99·Standard $129 영구 | 로컬 전용(SSH 원격 베타) | FSL-1.1-ALv2 | CLI(supacode)·훅·딥링크; Claude Code·Codex 등 CLI 에이전트 | 활발(v0.11.4, 2026-10) | 2차: 모바일·가격·셀프호스팅 확인 |
| 159 | [Superset](https://superset.sh) | 에이전트 전용 | 예 | iOS·iPadOS 앱(Pro 필요), Android 대기자 명단 | 무료 / Pro $20/사용자/월(연간 $15) / 엔터프라이즈 별도 | 가능(Docker, 내부 사용) | ELv2 | 내장 MCP 서버·CLI·TS SDK, CLI 에이전트 22종+ 훅 | 매우 활발(v1.38.0, 2026-10) | 2차: 라이선스 확인 |
| 160 | [Synara](https://www.trysynara.com/) | 에이전트 전용 | 예 | 미확인(공식 근거와 상충: iOS 앱(TestFlight 베타)) | 무료(OSS) | 가능(로컬/헤드리스 서버) | MIT | 내부·외부 MCP 서버, REST API, CLI; Claude Code·Codex·Cursor 등 | 매우 활발(v1.0.5, 2026-10) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 161 | [T3 Code (pingdotgg/t3code)](https://t3.codes) | 에이전트 전용 | 아니오 | iOS·Android 앱 | 무료 | 로컬 전용 | MIT | Claude Code·Codex·Cursor·Grok Build·OpenCode·Antigravity 제어 | 활발(스타 27,801, 2026-10-16 push) | 리드 직접 2차 확인 |
| 162 | [Taskade AI Agents](https://www.taskade.com) | 에이전트 전용 | 예 | iOS·Android 앱 | 무료 / Pro $10/월(연간) / Business $25/월~ | 불가(SaaS) | 상용 | 공식 MCP·REST API·웹훅, 멀티 에이전트 내장 | 활발(주간 릴리스) | 2차: 공식 근거 없음 |
| 163 | [Taskboard (tcarac)](https://github.com/tcarac/taskboard) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(단일 Go 바이너리, Homebrew) | MIT | 내장 MCP 22개 도구, CLI; Claude Code·opencode | 안정(★37, 2026-02 v0.6.0) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 164 | [Tasklix](https://tasklix.dev) | 에이전트 전용 | 예 | 반응형 웹 | 무료 / Starter $9.99/월 / Growth $49.99/월 | 불가(SaaS) | 상용 | MCP 29개 도구·REST API; Claude Code·Cursor·Codex | 운영 중(Smithery 등록) | 2차: 공식 근거 없음 |
| 165 | [Taskmaster AI (eyaltoledano/claude-task-master)](https://github.com/eyaltoledano/claude-task-master) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS) / Hamster Team $40/크리에이터/월 | 가능(로컬 파일·Docker) | MIT + Commons Clause | MCP·CLI·JSON 파일; Claude Code·Cursor·Codex 등 13종+ | 활발(~28k 스타, v0.43.1) | 2차: 셀프호스팅 확인 |
| 166 | [Taskmaster Kanban VS Code Extension (Hamster.task-master-hamster)](https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료 | 로컬 전용 | MIT | 로컬 task-master-ai MCP(stdio); Claude Code·Cursor·Codex | 활발(v1.x+, 공식) | 2차: 가격·셀프호스팅 확인 |
| 167 | [Terragon (Terragon Labs)](https://www.terragonlabs.com/) | 에이전트 전용 | 부분 | PWA(iOS·Android 홈화면 추가) | 14일 체험 / Core $25·Pro $50/사용자/월(종료 전) | 가능(OSS 스냅샷, 복잡한 구성) | Apache-2.0 | terry CLI·MCP 서버; Claude Code·Codex·Amp·Gemini CLI·OpenCode | SaaS 2026-02-09 종료 | 2차: 라이선스 확인 |
| 168 | [Tirith](https://eabz.github.io/tirith/) | 에이전트 전용 | 부분 | 없음(헤드리스 MCP/CLI) | 무료(OSS) | 가능(Rust 단일 바이너리) | MIT | MCP(stdio/SSE); Claude Code·Cursor·Codex·Pi | 초기(★4, v1.3.0) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 169 | [Traycer AI](https://traycer.ai) | 에이전트 전용 | 예 | iOS·Android 앱 | 무료 / Sync $16/사용자/월(연간) | 일부(하이브리드) | 상용 | Claude Code·Codex CLI·Cursor·OpenCode 직접 연동, 원격 호스트 | 활발(2025-2026) | 2차: 모바일 확인 |
| 170 | [Unpeel](https://unpeel.com/) | 에이전트 전용 | 아니오 | iOS 앱(TestFlight) | 무료(OSS) / Link 릴레이 미확인(초기 $59/좌석) | 가능(macOS/Linux 서버 데몬) | MIT | 통합 MCP 서버, CLI; Claude Code·Codex·Gemini CLI 등 | 활발(v0.7.5, 2026-10) | 2차: 모바일·셀프호스팅·라이선스 확인 |
| 171 | [Uzi](https://github.com/devflowinc/uzi) | 에이전트 전용 | 부분 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | MIT | CLI로 Claude·Codex·Aider·Cursor·커스텀 에이전트 병렬 실행 | 정체/유지보수(v0.0.2) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 172 | [Veritas Kanban](https://github.com/BradGroux/veritas-kanban) | 에이전트 전용 | 예 | 반응형 웹 | 무료(OSS) | 가능(로컬 우선, Docker) | MIT | 공식 MCP 서버, CLI, 웹훅, REST API; Claude Code 등 | 841★, 매우 활발(v6.2.1) | 2차: 가격·셀프호스팅·라이선스 확인 |
| 173 | [Vibe Kanban](https://vibekanban.com) | 에이전트 전용 | 예 | 반응형 웹(네이티브 앱 없음) | 무료(OSS) | 가능(npx·Docker Compose) | Apache-2.0 | CLI, Claude Code·Codex·Gemini·Cursor 등 10종 이상 | 서비스 종료 예정(2026-04 Bloop 종료) | 리드 직접 2차 확인 |
| 174 | [Vibeyard](https://github.com/elirantutia/vibeyard) | 에이전트 전용 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용(데스크톱 앱) | MIT | CLI/PTY 래퍼; Claude Code·Codex CLI·Gemini CLI | 유지(★1,387, v0.3.8) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 175 | [VS Code Agent Kanban / AgentKanban.io](https://agentkanban.io) | 에이전트 전용 | 예 | 반응형 웹 | 무료 / Pro $10/사용자/월 | 일부(하이브리드) | ELv2(확장)·상용(클라우드) | MCP·REST API·웹훅·Copilot 채팅(Claude Code·Codex·Cursor) | 활발(v2.1.3, 2026.5) | 2차: 공식 근거 없음 |
| 176 | [WorkBuddy Board](https://board-widget-download.app.workbuddy.host/) | 에이전트 전용 | 예 | 없음(Windows 데스크톱 위젯) | 무료(OSS) | 로컬 전용(단일 exe) | MIT | MCP 도구 15개(Python 서버), 원클릭 등록 | 초기(★1, v1.1.0) | 2차: 모바일·가격·셀프호스팅·라이선스 확인 |
| 177 | [Yao Agents](https://yaoagents.com) | 에이전트 전용 | 예 | Android 앱(베타) | 무료(OSS) / 엔터프라이즈 별도 | 가능(Go 바이너리·Docker) | 수정 Apache-2.0 | Open API(SSE·WebSocket), DeepSeek Harness, Jev 엔진 | 활발(스타 8.1k+) | 2차: 셀프호스팅 확인 |
| 178 | [@suekou/mcp-notion-server (Community MCP Server)](https://github.com/suekou/mcp-notion-server) | MCP 서버 | 아니오 | 없음(로컬 프로세스) | 무료(OSS) | 가능(npx·로컬 빌드) | MIT | MCP(stdio); Claude Desktop·Cursor·Windsurf 등 | 활발(v2.0.2, 2026-06) | 노드 신뢰도 높음 |
| 179 | [ClickUp Community MCP Servers (hauptsacheNet & taazkareem)](https://github.com/hauptsacheNet/clickup-mcp) | MCP 서버 | 부분 | iOS·Android 앱 | 무료 / $7 / $12 /사용자/월(연간) | 가능(npx·Docker) | MIT | MCP(STDIO·HTTP); Claude Code·Cursor·Codex·Windsurf·n8n | 활동 중(2026-10 푸시) | 리드 직접 2차 확인 |
| 180 | [Container-use](https://github.com/dagger/container-use) | MCP 서버 | 아니오 | 없음(데스크톱 전용) | 무료(OSS) | 가능(Docker/Dagger 로컬) | Apache-2.0 | MCP 서버로 Claude Code·Cursor·Goose·VS Code 등 연동 | 활발(v0.4.2, 2026-10) | 노드 신뢰도 높음 |
| 181 | [GitHub Kanban MCP Server (Sunwood AI Labs)](https://github.com/Sunwood-ai-labs/github-kanban-mcp-server) | MCP 서버 | 부분 | 없음(로컬 MCP 서버) | 무료(OSS) | 로컬 전용(npm/npx) | MIT | MCP: Claude Desktop·Cursor·Cline·Claude Code | 활동 중(★10) | 노드 신뢰도 높음 |
| 182 | [MCP Server Trello (delorenj)](https://github.com/delorenj/mcp-server-trello) | MCP 서버 | 부분 | iOS·Android 앱(Trello) | 무료 / Trello Standard $5/사용자/월~ | 일부(MCP만 로컬, Trello는 SaaS) | MIT | MCP 57개 도구, 공식 레지스트리 등재; Claude Code·Cursor | 매우 활발(★446, 2026-10 커밋) | 노드 신뢰도 높음 |
| 183 | [mcp-server-asana (Community MCP Server by roychri)](https://github.com/roychri/mcp-server-asana) | MCP 서버 | 부분 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | MIT | MCP(stdio), Claude Code·Cursor·Codex·Claude Desktop | 활발(★149, 2026-10 커밋) | 노드 신뢰도 높음 |
| 184 | [Notion MCP (Official Remote Server)](https://developers.notion.com/guides/mcp/overview.md) | MCP 서버 | 아니오 | 클라이언트 의존(Notion 앱으로 확인) | 워크스페이스 포함(고급 기능 Business 이상) | 불가(SaaS) | 상용(호스팅 서비스) | MCP(Streamable HTTP/SSE); Claude Code·Cursor·Codex·Copilot 등 | 활발(운영 서비스) | 노드 신뢰도 높음 |
| 185 | [notion-mcp-server (Official Open-Source Local Server)](https://github.com/makenotion/notion-mcp-server) | MCP 서버 | 아니오 | 없음(로컬 서버) | 무료(OSS), Notion 계정 필요 | 가능(npx·Docker) | MIT | MCP(stdio/HTTP); Claude Desktop·Cursor·Zed·Copilot CLI | 지원 중단(유지보수 없음) | 노드 신뢰도 높음 |
| 186 | [Planka MCP (gogogadgetbytes)](https://github.com/gogogadgetbytes/planka-mcp) | MCP 서버 | 부분 | 반응형 웹(Planka 웹 UI) | 무료 / Planka Pro €7.20/사용자/월~ | 가능(Planka Docker/K8s, npx) | MIT | MCP(stdio, npx) 13개 도구; Claude Code·Claude Desktop | 안정(★11, 2026-02 v1.0.0) | 노드 신뢰도 높음 |
| 187 | [Shrimp Task Manager MCP (cjo4m06/mcp-shrimp-task-manager)](https://github.com/cjo4m06/mcp-shrimp-task-manager) | MCP 서버 | 부분 | 반응형 웹 | 무료(OSS) | 로컬 전용 | MIT | MCP(15개 툴)·REST API·SSE·CLI; Claude Code·Cline·Cursor 등 | 활성(최근 커밋 2025-08-21) | 노드 신뢰도 높음 |
| 188 | [Trello MCP Server (m0xai)](https://github.com/m0xai/trello-mcp-server) | MCP 서버 | 부분 | iOS·Android 앱 | 무료(10명) / $5 / $10 / $17.50 /사용자/월 | 불가(SaaS) | MIT(LICENSE 파일 없음) | MCP(stdio, uv); 카드 CRUD; Claude Desktop·Claude Code | 안정(★54, 2026-06 커밋) | 리드 직접 2차 확인 |
| 189 | [WeKan MCP Server (namar0x0309)](https://github.com/namar0x0309/wekan-mcp) | MCP 서버 | 부분 | 반응형 웹(WeKan, PWA 지원) | 무료(OSS) / WeKan 상용 지원 €50/년~ | 가능(WeKan Docker/Snap 등) | ISC | MCP(stdio); 카드 생성·이동·댓글; 토큰 생성 스크립트 | 안정(★6, 2026-06 커밋) | 노드 신뢰도 높음 |
| 190 | [zenhub-cli (`zh`) & Bundled MCP Server](https://github.com/daniel-pittman/zenhub-cli) | MCP 서버 | 아니오 | 없음(터미널 CLI) | 무료(OSS) | 로컬 전용 | MIT | FastMCP 약 68개 도구(stdio)·Claude Code 에이전트·CLI | 활발(v2.4.0, 2026-08) | 노드 신뢰도 높음 |
| 191 | [zenhub-mcp (TypeScript)](https://github.com/derekbar90/zenhub-mcp) | MCP 서버 | 아니오 | 없음(로컬 Node.js) | 무료($0) | 로컬 전용 | 미명시(라이선스 파일 없음) | MCP 54개 도구(stdio)·Claude Desktop·Cursor | 정체(마지막 푸시 2025-07) | 노드 신뢰도 높음 |
| 192 | [Airtable (Kanban view + AI + MCP)](https://www.airtable.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / Team $20/사용자/월(연간) / Business $45 | 불가(SaaS) | 상용 | 공식 원격 MCP·CLI·REST API; Claude Code·Codex·Cursor | 활발한 상용 SaaS(2026) | 노드 신뢰도 높음 |
| 193 | [Asana](https://asana.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(최대 2인) / Starter $10.99~, Advanced $24.99/사용자/월 | 불가(SaaS) | 상용 | REST API·웹훅, 커뮤니티 MCP(Claude Code·Cursor 등), 내장 AI | 활발(상장사 SaaS) | 노드 신뢰도 높음 |
| 194 | [Backlog (Nulab)](https://backlog.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(10명) / Starter 약 ¥2,970/월~ | 엔터프라이즈만(온프레미스) | 상용 | Backlog AI·REST API v2·웹훅·커뮤니티 MCP | 운영 중(주간 릴리스) | 노드 신뢰도 중간 |
| 195 | [Basecamp card table](https://basecamp.com/) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / $25~$299/월(정액, 사용자 무제한) | 불가(SaaS) | 상용 | 공식 CLI·stdio MCP·REST API; Claude·Codex 스킬 | 매우 활발(2026 CLI·MCP 출시) | 노드 신뢰도 높음 |
| 196 | [BoardMark](https://www.boardmark.ai) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(2인) / Pro ฿249/석/월 / Premium ฿329 | 불가(SaaS) | 상용 | MCP 50+ 도구·OAuth; Claude·ChatGPT·Cursor | 운영 중(다중 레지스트리) | 노드 신뢰도 높음 |
| 197 | [Businessmap (formerly Kanbanize)](https://businessmap.io) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 유료 $10.60/사용자/월(연간), 최소 15석; 무료 없음 | 불가(SaaS) | 상용 | 공식 MCP 서버·REST API v2·웹훅; Claude Code·Cursor 연동 | 활발(상용 SaaS, 2026) | 노드 신뢰도 높음 |
| 198 | [ClickUp](https://clickup.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / $7 / $12 /사용자/월(연간) | 불가(SaaS) | 상용 | 공식 원격 MCP(OAuth 2.1)·REST API·웹훅; Claude Code·Cursor 등 | 활발(상용 SaaS) | 리드 직접 2차 확인 |
| 199 | [craaft](https://craaft.io) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(3개 프로젝트) / Pro $14/월 | 불가(SaaS) | 상용 | MCP 74개 도구·REST·웹훅; Claude·Cursor·Windsurf | 운영 중(Smithery·Glama 등록) | 노드 신뢰도 높음 |
| 200 | [devBoard](https://getdevboard.app) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 얼리 액세스 기간 무료 | 불가(SaaS) | 상용 | 원격 MCP 29개 도구; Claude Code·Cursor·VS Code | 얼리 액세스 베타 | 노드 신뢰도 높음 |
| 201 | [Devin + Jira](https://www.atlassian.com/software/jira) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(10명) / Standard $7.75 / Premium $15.25/사용자/월 | 일부(Data Center, 연동 제한) | 상용 | Forge 앱(Rovo 에이전트), OAuth, REST API v3, 웹훅, MCP | 매우 활발 | 노드 신뢰도 높음 |
| 202 | [Devin + Linear](https://linear.app) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / Standard $8~10/사용자/월 / Plus $14~16 (Devin 별도) | 불가(SaaS) | 상용 | 네이티브 Agent 세션, OAuth, GraphQL API, CLI, 웹훅, MCP | 매우 활발 | 노드 신뢰도 높음 |
| 203 | [Devin + Slack](https://slack.com) | 범용 칸반+에이전트 연동 | 부분 | iOS·Android 앱 | 무료 / Pro $7.25~8.75 / Business+ $12.50~15/사용자/월 | 불가(SaaS) | 상용 | 공식 Devin Slack 앱, Web API, Bolt SDK, CLI, 웹훅, MCP | 활발 | 노드 신뢰도 높음 |
| 204 | [Dooray! Project](https://dooray.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(25명) / 비즈니스 2,000원/사용자/월~ | 엔터프라이즈만(프라이빗·온프렘) | 상용 | Dooray AI·Open API(REST)·웹훅·봇 | 운영 중(주간 업데이트) | 노드 신뢰도 높음 |
| 205 | [EasyKanban](https://easykanb.app) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(읽기 전용) / Pro €6/월(쓰기 도구) | 불가(SaaS) | 상용 | 원격 MCP(OAuth 2.1); Claude·ChatGPT·Cursor·VS Code | 운영 중(Smithery 등록) | 노드 신뢰도 높음 |
| 206 | [Forgejo](https://forgejo.org/) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹·PWA (서드파티 GitNex Android 앱) | 무료(OSS, 유료 티어 없음) | 가능(바이너리·Docker·Helm) | GPL-3.0-or-later | Gitea 호환 REST API·Webhook·tea/forgejo-cli·커뮤니티 MCP | 매우 활발(v17.0.0, 2026-10-15) | 노드 신뢰도 높음 |
| 207 | [Framedeck MCP](https://framedeck.app) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | MCP 무료 / SaaS 무료 티어+유료(가격 미확인) | 불가(SaaS, 커넥터만 로컬) | 상용(커넥터는 MIT) | 공식 MCP·REST; Claude Code·ChatGPT·Gemini·Copilot | 소규모(★1, 커밋 2026-04-08) | 노드 신뢰도 높음 |
| 208 | [Gitea](https://about.gitea.com/) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹·PWA (서드파티 GitNex Android 앱) | 무료(OSS) / Enterprise $9.50~$19/사용자/월 | 가능(바이너리·Docker·Helm) | MIT | REST API·Webhook·tea CLI·커뮤니티 MCP(Claude Code, Codex, Cursor 등) | 활발(v28.1.0, 2026-10-06) | 노드 신뢰도 높음 |
| 209 | [GitHub Projects (Projects v2) Boards](https://github.com/features/issues) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료 / Team $4/사용자/월, Enterprise $21/사용자/월 | 가능(GHES) | 상용 | MCP projects 툴셋·GraphQL/REST API·gh CLI; Claude Code·Cursor 등 | 매우 활발(MCP v2.0.2, 2026년 10월) | 노드 신뢰도 높음 |
| 210 | [GitLab Issue Boards (CE Self-Host & SaaS)](https://about.gitlab.com/) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹(공식 네이티브 앱 없음) | 무료(CE) / Premium $29/사용자/월 / Ultimate $99+ | 가능(Omnibus·Docker·Helm, CE 무료) | MIT(CE, 오픈코어) | REST·GraphQL API, glab CLI, 웹훅, MCP(Claude Code 등) | 활발(18.x 릴리스, 2026.10) | 노드 신뢰도 높음 |
| 211 | [Groma.md (MrLesk/Groma.md)](https://github.com/MrLesk/Groma.md) | 범용 칸반+에이전트 연동 | 부분 | 없음 | 무료(OSS) | 로컬 전용(Git 저장소) | MIT | 에이전트가 소스 스캔·큐레이션; Backlog.md 연동 | 271스타·최근 push 2026-10 | 노드 신뢰도 중간 |
| 212 | [Hamster (tryhamster.com, Task Master 클라우드)](https://tryhamster.com) | 범용 칸반+에이전트 연동 | 부분 | 미확인(웹 앱) | 무료 월 10 브리프 / 유료 미확인 | 불가(SaaS) | 상용 | 사용자 에이전트 연동; Task Master MCP/CLI 연결 | 활발 | 노드 신뢰도 중간 |
| 213 | [Huly](https://huly.io) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹(네이티브 앱·PWA 없음) | 무료(OSS 셀프호스팅) / 공식 클라우드 종료 | 가능(Docker, 15개+ 컨테이너) | EPL-2.0 | MCP·CLI(Claude Code·Codex·Cursor 등), REST/WS API | 원 저장소 동결, 커뮤니티 포크 활발 | 노드 신뢰도 높음 |
| 214 | [It's a Plan (ItsAPlan)](https://itsaplan.dev) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(OSS), 사용자당 과금 없음 | 가능(Docker Compose, Coolify, Railway) | AGPL-3.0 | 공식 MCP 서버, REST API, 웹훅; Claude Code 등 | 983★, 활발 | 노드 신뢰도 높음 |
| 215 | [Jira Boards](https://www.atlassian.com/software/jira) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료(10명) / Standard $8.48·Premium $15.56/사용자/월 | 일부(Data Center, Rovo 미포함) | 상용 | Rovo MCP·twg-cli·REST API; Claude Code·Cursor·Copilot·Codex | 활발(주간 릴리스) | 노드 신뢰도 높음 |
| 216 | [Kaiten](https://kaiten.ru) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(5명까지) / Start 185₽/사용자/월부터 | 엔터프라이즈만(Docker, 300명+) | 상용 | REST API·웹훅·텔레그램 봇; 공식 MCP/Skills 출시 예정 | 활발(상용, 2026 업데이트) | 노드 신뢰도 높음 |
| 217 | [Kan (kan.bn)](https://kan.bn) | 범용 칸반+에이전트 연동 | 예 | PWA·반응형 웹(네이티브 앱 없음) | 무료 / Teams $8/사용자/월 / Enterprise 별도 | 가능(Docker Compose, Railway) | AGPL-3.0 | 공식 MCP(@kan/mcp, 46개 도구)·REST API·웹훅; Claude·Codex·Cursor | 활발(v0.6.0, 2026-10 커밋) | 노드 신뢰도 높음 |
| 218 | [Kanbn CLI](https://github.com/basementuniverse/kanbn) | 범용 칸반+에이전트 연동 | 부분 | 없음(CLI·VS Code 전용) | 무료(OSS) | 로컬 전용 | MIT | CLI·Node API·커뮤니티 MCP; Claude Code·Cursor·Codex 지원 | 활발(v2.5.3, 2026-09) | 노드 신뢰도 높음 |
| 219 | [Kanera](https://www.kanera.app) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료 / Pro $5/사용자/월 | 불가(SaaS) | 상용 | MCP 45+ 도구; Claude·ChatGPT·Codex·Cursor·Gemini CLI | 운영 중(2026-10 업데이트) | 노드 신뢰도 높음 |
| 220 | [Leantime](https://leantime.io) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹(네이티브 앱은 TestFlight 개발 중) | 무료(OSS) / Pro $10/사용자/월(연간 $8) | 가능(Docker, PHP/MySQL) | AGPL-3.0 | 공식 MCP(HTTP/stdio)·JSON-RPC API·CLI; Claude Code·Cursor 등 | 활발(v3.12.0, 2026-10) | 노드 신뢰도 높음 |
| 221 | [Linear (Linear for Agents)](https://linear.app) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 네이티브 앱 + 반응형 웹 | 무료(이슈 250개) / Basic $10 / Business $16 /사용자/월(연간) | 불가(SaaS) | 상용 | 공식 원격 MCP·GraphQL API·SDK·웹훅; Claude Code·Codex·Cursor·Devin 등 | 활발(고객 4만+ , 2026 릴리스) | 리드 직접 2차 확인 |
| 222 | [LokalBoards (florian-strasser)](https://github.com/florian-strasser/LokalBoards) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(OSS) | 가능(Node.js+MySQL, Docker) | MIT | 내장 MCP(/mcp), API 키·OAuth; 에이전트 계정 | 활발(★17, 2026-10 v0.40.2) | 노드 신뢰도 높음 |
| 223 | [Mattermost Boards](https://mattermost.com/marketplace/boards/) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(Free/Community) / Professional·Enterprise 별도 견적 | 가능(Docker) | AGPL-3.0 | REST API, Mattermost Agents 공식 MCP 서버, mmctl CLI | 활발히 개발 중 | 노드 신뢰도 높음 |
| 224 | [monday.com boards](https://monday.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(2석) / $9 / $12 / $19 /석/월 | 불가(SaaS) | 상용 | 호스티드 MCP·GraphQL API·mcli CLI; Claude·Cursor 등 연동 | 활발(상용 SaaS) | 리드 직접 2차 확인 |
| 225 | [Motion AI](https://www.usemotion.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 7일 체험 / 개인 $19/월(연간) / 팀 $12/사용자/월 | 불가(SaaS) | 상용 | REST API·커뮤니티 SDK·Zapier, MCP 없음 | 활발(상용) | 노드 신뢰도 높음 |
| 226 | [Nextcloud Deck](https://apps.nextcloud.com/apps/deck) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱(커뮤니티/서드파티), 반응형 웹 | 무료(OSS) / 엔터프라이즈 지원 €71.29~/사용자/년 | 가능(Docker, K8s, AIO) | AGPL-3.0 | REST/OCS API, 커뮤니티 MCP, CLI(ncdeck); Claude Code·Cursor·Codex | 활발(v1.19.0, 2026-10) | 노드 신뢰도 높음 |
| 227 | [Notion Boards & Databases](https://www.notion.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료 / Plus $10/사용자/월 / Business $20–25 | 불가(SaaS) | 상용 | REST API·원격 MCP·CLI(ntn)·웹훅; Claude Code·Cursor·Codex 등 | 활발(상용 SaaS) | 노드 신뢰도 높음 |
| 228 | [OpenProject](https://www.openproject.org) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱(베타), 반응형 웹 | 무료(Community) / Enterprise €5.95~/사용자/월 | 가능(Docker·DEB/RPM·Helm) | GPL-3.0 | 내장 MCP(유료)·REST API v3·웹훅; Claude Code·Cursor 등 | 활발(v17.9.2, 2026-10) | 노드 신뢰도 높음 |
| 229 | [Phabricator / Phorge Workboards](https://phorge.it) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 무료(OSS) | 가능(LAMP/LEMP·Docker) | Apache-2.0 | Conduit API, Arcanist CLI, 커뮤니티 MCP 서버(Claude Code·Codex·Cursor) | Phorge 활발(Phabricator 2021 종료) | 노드 신뢰도 높음 |
| 230 | [Plane](https://plane.so) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / Pro $6 / Business $13 /좌석/월 | 가능(Docker) | AGPL-3.0 | REST·GraphQL API, MCP 연동(Claude Code·Cursor) | 매우 활발(60.9k stars) | 리드 직접 2차 확인 |
| 231 | [Redmine (with Agile/Kanban plugins)](https://www.redmine.org) | 범용 칸반+에이전트 연동 | 부분 | 반응형 웹 + 서드파티 iOS·Android 앱 | 무료(OSS) / Agile PRO 플러그인 $499 일회성~ | 가능(Docker·소스 설치) | GPL-2.0 | REST API, 웹훅(플러그인), 커뮤니티 MCP 서버(Claude Code·Cursor·Codex CLI) | 활발(2026-10 커밋) | 노드 신뢰도 높음 |
| 232 | [scrumboy](https://scrumboy.com) | 범용 칸반+에이전트 연동 | 예 | PWA | 무료(OSS) | 가능(Go 바이너리·Docker) | AGPL-3.0 | 내장 MCP·REST API; Claude·Cursor 등 | 매우 활발(v3.38.8, 2026-10-16) | 노드 신뢰도 높음 |
| 233 | [Shortcut](https://shortcut.com) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹·PWA(네이티브 앱 내려감) | 무료 / Team $8 / Business $12 | 불가(SaaS) | 상용 | 호스팅 MCP(OAuth)·REST API v3·웹훅; Claude Code·Cursor·Devin | 활발(MCP 저장소는 아카이브) | 리드 직접 2차 확인 |
| 234 | [Super Productivity](https://super-productivity.com) | 범용 칸반+에이전트 연동 | 부분 | iOS·Android 앱+PWA | 무료(OSS) | 가능(데스크톱/동기화 서버 Docker) | MIT | 로컬 동기화 서버 REST·WebSocket·CLI·플러그인 | 매우 활발(★22,859, v19.2.0) | 노드 신뢰도 높음 |
| 235 | [Taiga](https://taiga.io/) | 범용 칸반+에이전트 연동 | 예 | 공식 앱 없음(부분 반응형 웹, 커뮤니티 Android 앱) | 무료(OSS 셀프호스팅) / Cloud 5€·20€·60€/월 | 가능(Docker) | MPL-2.0 | 커뮤니티 MCP 서버·REST API; Claude Code·Cursor·Codex 연동 | 활발한 유지보수(2026-10 커밋) | 노드 신뢰도 높음 |
| 236 | [Teamhood](https://teamhood.com/) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹 | 유료 €5/$6/사용자/월(연간, 최소 3명) | 엔터프라이즈만 | 상용 | 공식 원격 MCP(읽기/쓰기·읽기전용)·REST API·에이전트 스킬 | 활발(격주 업데이트) | 노드 신뢰도 높음 |
| 237 | [TickTick](https://ticktick.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료 / Premium $3.99/월 또는 $35.99/년 | 불가(SaaS) | 상용 | 커뮤니티 MCP(비공식)·공식 Open API v1·비공식 V2 API | 활발(SaaS 유지보수 중) | 노드 신뢰도 높음 |
| 238 | [Todoist](https://todoist.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료 / Pro $5~7/월 / Business $8~10/사용자/월 | 불가(SaaS) | 상용 | 공식 호스팅 MCP·CLI(td)·REST/Sync API·SDK; Claude Code·Cursor·Codex 등 | 활발(MCP v13.5.0, 2026.10) | 노드 신뢰도 높음 |
| 239 | [Trello (Atlassian)](https://trello.com) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료(10명) / $5 / $10 / $17.50 /사용자/월 | 불가(SaaS) | 상용 | 공식 원격 MCP·커뮤니티 MCP·REST API·웹훅 | 활발(Atlassian 운영 SaaS) | 리드 직접 2차 확인 |
| 240 | [Vikunja](https://vikunja.io) | 범용 칸반+에이전트 연동 | 예 | PWA·반응형 웹(서드파티 iOS 앱 mDone) | 무료(OSS) / Cloud 4€/월~, 조직 5€/사용자/월 | 가능(바이너리·Docker·K8s) | AGPL-3.0-or-later | 내장 MCP(/api/v2/mcp)·REST API v2·웹훅; Claude Code·Codex 가이드 | 활발(v2.7.0, 2026-10) | 노드 신뢰도 높음 |
| 241 | [WeKan](https://wekan.fi) | 범용 칸반+에이전트 연동 | 예 | PWA·반응형 웹, Android TWA 래퍼(iOS 앱 없음) | 무료(OSS) / 호스팅 SaaS 무료 또는 50 EUR/년 | 가능(Docker·K8s·Snap 등) | MIT | REST API·웹훅·Python CLI·Bots, 커뮤니티 n8n MCP | 매우 활발(~19k★, v12.48) | 노드 신뢰도 높음 |
| 242 | [Wrike](https://www.wrike.com/) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱 | 무료 / Team $10, Business $25/사용자/월 | 불가(SaaS) | 상용 | 공식 원격 MCP·REST API v4; Claude Code·Cursor·ChatGPT | 매우 활발(2026 지속 릴리스) | 노드 신뢰도 높음 |
| 243 | [YouTrack (JetBrains)](https://www.jetbrains.com/youtrack/) | 범용 칸반+에이전트 연동 | 예 | iOS·Android 앱, 반응형 웹 | 무료(10명까지) / Cloud $4.50~/사용자/월(연간) | 가능(Docker·ZIP/MSI·JAR) | 상용(독점) | 내장 원격 MCP·REST API·CLI / Claude·Cursor·Codex | 활발(정기 릴리스, 2025.3 MCP 도입) | 노드 신뢰도 높음 |
| 244 | [ZenHub](https://www.zenhub.com) | 범용 칸반+에이전트 연동 | 예 | 반응형 웹(네이티브 앱 단종) | 무료(50명) / Teams $8.33~12.50/사용자/월 | 엔터프라이즈만 | 상용 | GraphQL API·웹훅·커뮤니티 MCP/CLI(Claude Code·Cursor·Codex) | 활발(상용 SaaS) | 노드 신뢰도 높음 |
| 245 | [4ga Boards](https://4gaboards.com) | 범용 칸반 | 예 | 반응형 웹 | 무료(OSS) / 관리형 호스팅 유료(가격 미확인) | 가능(Docker) | MIT | REST API·웹훅(API 토큰으로 에이전트 조작) | 유지(★728, v3.3.13) | 노드 신뢰도 높음 |
| 246 | [AppFlowy (Board View)](https://appflowy.com) | 범용 칸반 | 예 | iOS·Android 앱 | 무료 / Pro $12/멤버/월(연간), 셀프호스트 유료 티어 | 가능(Docker Compose, 다중 사용자는 상용) | AGPL-3.0 | REST OpenAPI(JWT); 상용 백엔드 내부 MCP, 공개 MCP 서버 없음 | 활발(데스크톱 v0.14.8, 서버 v0.19.6) | 노드 신뢰도 높음 |
| 247 | [Focalboard](https://www.focalboard.com) | 범용 칸반 | 예 | 반응형 웹 | 무료(OSS) | 가능(Docker) | AGPL-3.0 | REST API(v2)만 지원, MCP·CLI 없음; HTTP로 에이전트 연동 | 유지보수 중단(커뮤니티 이관) | 노드 신뢰도 높음 |
| 248 | [Kanban Tool](https://kanbantool.com) | 범용 칸반 | 예 | iOS·Android 앱 | 무료(2명) / Team $6, Enterprise $11/사용자/월 | 엔터프라이즈만(VM 어플라이언스, 10명+) | 상용 | REST API v1/v3·JS SDK·웹훅; MCP 서버 없음 | 활발(상용, 고객 25,000+) | 노드 신뢰도 높음 |
| 249 | [KanbanFlow](https://kanbanflow.com) | 범용 칸반 | 예 | PWA(모바일 웹, 네이티브 앱 없음) | 무료 / Premium $5/사용자/월 | 불가(SaaS) | 상용 | REST API·웹훅(Premium 전용), Zapier; HTTP 호출로 에이전트 연동 | 안정 운영(상용 SaaS) | 노드 신뢰도 높음 |
| 250 | [Kanboard](https://kanboard.org) | 범용 칸반 | 예 | 반응형 웹(공식 네이티브 앱 없음) | 무료(OSS) | 가능(Docker) | MIT | JSON-RPC 2.0 API·CLI·웹훅; 커뮤니티 MCP(Claude Code·Cursor 등) | 유지보수 모드(v1.2.54, 2026-08) | 노드 신뢰도 높음 |
| 251 | [Kaneo](https://kaneo.app/) | 범용 칸반 | 예 | 반응형 웹·PWA | 무료(OSS) / 클라우드 무료 티어 | 가능(Docker/K8s) | MIT | REST API·웹훅·GitHub/Gitea 연동 | 매우 활발(★9,494, v2.46.2) | 노드 신뢰도 높음 |
| 252 | [Kanri](https://kanriapp.com) | 범용 칸반 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | GPL-3.0 | 로컬 JSON 파일 직접 편집; API·MCP 없음 | 활발(v0.8.2, 2025-09) | 노드 신뢰도 높음 |
| 253 | [MeisterTask](https://www.meistertask.com/) | 범용 칸반 | 예 | iOS·Android 앱 | 무료 / Pro $12.50/사용자/월 | 불가(SaaS) | 상용 | REST API·웹훅; 공식 MCP 없음(HTTP 직접 호출) | 활발(상용 SaaS) | 노드 신뢰도 높음 |
| 254 | [Nullboard](https://nullboard.io) | 범용 칸반 | 예 | 반응형 웹(모바일 미검증) | 무료(개인·비상업적 사용) | 가능(단일 HTML·정적 호스팅) | BSD-2 + Commons Clause | 로컬 데몬 HTTP 동기화·.nbx JSON 편집; MCP 없음 | 휴면(마지막 커밋 2023-11) | 노드 신뢰도 높음 |
| 255 | [Obsidian Kanban Plugin](https://github.com/community-archive/obsidian-kanban) | 범용 칸반 | 예 | iOS·Android 앱(Obsidian 모바일) | 무료(OSS) / Obsidian Sync·Publish 선택 유료 | 로컬 전용 | GPL-3.0 | 마크다운 파일 직접 편집(MCP·CLI·API 없음) | 커뮤니티 아카이브·메인테이너 모집 | 노드 신뢰도 높음 |
| 256 | [Planka](https://planka.app) | 범용 칸반 | 예 | 반응형 웹·PWA(공식 네이티브 앱 없음) | Community 무료 / Pro €7.20~/사용자/월 | 가능(Docker·Helm·수동 설치) | PLANKA Community License | REST API·웹훅·CLI(plnk)·커뮤니티 MCP; Claude Code·Cursor 등 | 활발(v2.2.1, 2026-08) | 노드 신뢰도 높음 |
| 257 | [Restyaboard](https://restya.com/board/) | 범용 칸반 | 예 | 반응형 웹(네이티브 앱 중단) | 무료(OSS) / 상용 €7~€197/사용자/월(과거 기준) | 가능(Docker·설치 스크립트) | OSL-3.0 / 상용 듀얼 | REST API·웹훅; MCP 없음, 커스텀 HTTP 스크립트로 연동 | 비활성(마지막 릴리스 2022-03) | 노드 신뢰도 높음 |
| 258 | [Tasks.md](https://github.com/BaldissaraMatheus/Tasks.md) | 범용 칸반 | 예 | PWA·반응형 웹(네이티브 앱 없음) | 무료(OSS) | 가능(Docker) | MIT | MCP 없음; 파일시스템 직접 조작·무인증 REST API로 에이전트 연동 | 활발(v3.3.0, 2026-03) | 노드 신뢰도 높음 |
| 259 | [Unified Kanban (VS Code Extension)](https://github.com/divisionseven/unified-kanban) | 범용 칸반 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | MIT | 마크다운 파일 직접 편집(MCP·CLI·API 없음) | 유지 중(v0.7.3, 2026.4) | 노드 신뢰도 높음 |
| 260 | [vscode-kanban (mkloubert)](https://github.com/mkloubert/vscode-kanban) | 범용 칸반 | 예 | 없음(데스크톱 전용) | 무료(OSS) | 로컬 전용 | LGPL-3.0 | JSON 파일·JS 이벤트 스크립트(MCP 없음) | 아카이브·지원 중단(2022) | 노드 신뢰도 높음 |

## 에이전트 전용 도구 상세

### 1Code
- Claude Code·Codex를 격리된 git worktree에서 실행하는 오픈소스 데스크톱/웹 클라이언트
- 장점: 세션별 격리 worktree; 실시간 diff·hunk 롤백; MCP 관리자·플러그인 마켓
- 단점: 저장소 아카이브(비활성); Claude Code·Codex 위주 지원; 바이너리·클라우드는 유료
- 출처: <https://1code.dev>, <https://yetanotherorchestrator.app/apps/1code/>, <https://github.com/21st-dev/1code>, <https://api.github.com/repos/21st-dev/1code>, <https://raw.githubusercontent.com/21st-dev/1code/main/package.json>, <https://api.github.com/repos/21st-dev/1code/releases/tag/v0.0.84>

### 5dive
- systemd로 격리된 경량 멀티 에이전트 운영 플랫폼, 텔레그램으로 제어
- 장점: Unix 네이티브의 가벼운 설계; 텔레그램 기반 탭 한 번 승인; 지출·파괴적 작업에 사람 승인 게이트
- 단점: Linux·systemd 및 root 권한 필요; 웹 UI는 별도 플러그인으로 분리 중; 모바일 알림에 텔레그램 필수
- 출처: <https://github.com/5dive-ai/5dive>, <https://5dive.ai>, <https://t.me/FiveDiveBot>

### Agent 007
- 매니저 에이전트가 잡 보드에 작업 게시 후 Claude Code·Codex 워커를 배치하는 픽셀아트 가상 오피스
- 장점: 매니저-워커 자율 계층; 작업별 worktree 격리; 모바일 음성 통화 인터페이스
- 단점: 리소스 소모 큼; 초기 프로젝트; 음성 기능 추가 설정 필요
- 출처: <https://github.com/bill10/agent-007>, <https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md>

### Agent Dashboard (dustinblack)
- 여러 호스트의 컨테이너 기반 코딩 에이전트를 배포·모니터링하는 대시보드
- 장점: 멀티 호스트 통합 제어; OTLP 기반 토큰·비용 추적; rootless 컨테이너 보안
- 단점: 구조화된 칸반 기능 약함; 네이티브 모바일 앱 없음; 각 노드에 데몬 배포 필요
- 출처: <https://github.com/dustinblack/agent-dashboard>, <https://github.com/dustinblack/agent-dashboard/blob/main/docs/agent-profiles.md>

### Agent Deck
- 다중 코딩 에이전트 세션 관리 터미널 커맨드 센터, 봇 원격 제어
- 장점: 활발한 커뮤니티; 봇 통한 폰 모니터링·승인; 세션 포크·웹 GUI
- 단점: 열 기반 칸반 아닌 세션 목록 중심; 네이티브 앱 아닌 봇/웹; 봇 토큰 설정 필요
- 출처: <https://github.com/asheshgoplani/agent-deck>, <https://news.ycombinator.com/item?id=46272551>

### Agent Kanban (saltbo)
- 에이전트가 API로 작업을 클레임·수행하고 사람이 승인하는 에이전트 우선 보드
- 장점: 에이전트를 일급 행위자로 설계; 자기 작업 승인 금지로 직무 분리; 서버리스 구조로 유휴 비용 0
- 단점: Realmroot·Enbor 플랫폼에 강하게 결합; FSL은 OSI 오픈소스 아님; 네이티브 앱·컨테이너 런타임 없음
- 출처: <https://github.com/saltbo/agent-kanban>, <https://agent-kanban.dev>

### Agent Kanban PM (kronos3610)
- 사람과 CLI 에이전트를 조율하는 로컬 우선 1인용 칸반 PM 서버
- 장점: MCP로 보드를 프로그래밍 방식으로 조작; SQLite 기반 로컬 프라이버시; 리소스 부담이 적음
- 단점: 1인 전용이라 공개 노출 위험; 컨테이너 샌드박스 없음; 모바일 앱·푸시 알림 없음
- 출처: <https://github.com/kronos3610/agent-kanban-pm>, <https://pypi.org/project/agent-kanban-pm/>

### Agent of Empires
- tmux 세션과 git 워크트리·컨테이너에서 여러 AI 코딩 에이전트를 병렬 실행하는 Rust 기반 세션 관리자
- 장점: 키보드 중심 Rust TUI로 리소스 사용량 매우 낮음; 설치형 PWA와 원격 터널로 모바일 지원; 워크트리·컨테이너로 작업 격리
- 단점: 드래그 앤 드롭 칸반 보드 없음; Windows는 WSL2에서만 지원; Jira·Linear 직접 연동과 자동 PR 생성 없음
- 출처: <https://www.agent-of-empires.com/>, <https://yetanotherorchestrator.app/apps/agent-of-empires/>, <https://www.agent-of-empires.com/docs/quick-start/>, <https://www.agent-of-empires.com/guides/web-dashboard/>, <https://www.agent-of-empires.com/guides/web/dashboard/>, <https://www.agent-of-empires.com/docs/api/>, <https://github.com/agent-of-empires/agent-of-empires>, <https://github.com/agent-of-empires/agent-of-empires/releases>

### Agent Orchestrator
- 격리 worktree에서 병렬 AI 코딩 에이전트를 계획·실행·감독하는 로컬 오케스트레이터
- 장점: 25+ 에이전트 하네스 지원; CI·PR 리뷰 피드백 자동 반영; 네이티브 모바일 앱
- 단점: Electron+Go 이중 구조로 복잡; 원격 페어링에 터널/VPN 필요; 열린 이슈 880+
- 출처: <https://ao-agents.com>, <https://yetanotherorchestrator.app/apps/agent-orchestrator/>, <https://github.com/OrchestratorInc/agent-orchestrator>, <https://api.github.com/repos/OrchestratorInc/agent-orchestrator>, <https://github.com/OrchestratorInc/agent-orchestrator/blob/main/docs/cli/README.md>, <https://github.com/OrchestratorInc/agent-orchestrator/releases>, <https://github.com/OrchestratorInc/agent-orchestrator/blob/main/docs/self-hosted-remote.md>

### Agent Teams
- 멀티 에이전트 코딩 팀이 자율 협업하는 무료 오픈소스 데스크톱 앱(실시간 칸반)
- 장점: API 키 없이 무료 모델로 시작; 팀 간 메시징 멀티 에이전트 협업; hunk 단위 코드 리뷰
- 단점: 네이티브 모바일 앱 없음; AGPL 카피레프트 제약; 무거운 Electron 용량
- 출처: <https://agentteams.live>, <https://yetanotherorchestrator.app/apps/agent-teams/>, <https://github.com/777genius/agent-teams-ai>, <https://api.github.com/repos/777genius/agent-teams-ai>, <https://raw.githubusercontent.com/777genius/agent-teams-ai/main/package.json>, <https://api.github.com/repos/777genius/agent-teams-ai/releases/tag/v2.17.10>, <https://raw.githubusercontent.com/777genius/agent-teams-ai/main/mcp-server/README.md>

### Agent Teams AI
- 300+ 모델·코딩 하네스를 지원하는 데스크톱용 멀티 에이전트 칸반
- 장점: 실시간 5열 칸반과 diff 리뷰; API 키 없이 무료 모델 사용; 에이전트별 git worktree 격리
- 단점: 모바일 미지원; AGPL 카피레프트 의무; Electron 리소스 과다
- 출처: <https://agentteams.live>, <https://github.com/777genius/agent-teams-ai>

### Agent4Discord
- Claude Code 세션을 Discord 채널로 관리하는 셀프호스트 봇
- 장점: 버튼으로 도구 승인; 도구 호출별 스레드로 가독성; 토큰·비용 추적
- 단점: Claude Code 전용; Discord 봇 설정 필요; 2,000자 제한으로 분할
- 출처: <https://github.com/raravel/Agent4Discord>

### AgentBox
- AI 코딩 에이전트를 격리된 Docker 샌드박스·클라우드 microVM에서 실행하는 오케스트레이터
- 장점: 컨테이너/microVM 격리로 호스트 보호; 1초 미만 체크포인트 복원·자동 일시정지; 로컬·멀티클라우드 지원
- 단점: 칸반 컬럼 보드 없음; Docker Desktop/OrbStack 필요; 초기 이미지 약 1GB
- 출처: <https://agent-box.sh/>, <https://agent-box.sh/docs>, <https://agent-box.sh/docs/cli>, <https://agent-box.sh/docs/hub>, <https://agent-box.sh/docs/api>, <https://agent-box.sh/docs/core-concepts>, <https://github.com/madarco/agentbox>, <https://yetanotherorchestrator.app/apps/agentbox/>

### AgentKanban
- 에이전트 개발용 칸반·컨텍스트 관리, VS Code·CLI 에이전트 연결
- 장점: Plan/TODO/Implement 워크플로; 카드별 Git worktree 자동화; AGENTS.md·메모리 동기화
- 단점: 무료 티어 하루 250 요청 제한; 셀프호스팅 불가; 네이티브 앱 없음
- 출처: <https://agentkanban.io>, <https://agentkanban.io/pricing>, <https://github.com/appsoftwareltd/vscode-agent-kanban>, <https://www.appsoftware.com/blog/introducing-vs-code-agent-kanban-task-management-for-the-ai-assisted-developer>

### AgentRQ
- 승인 요청 등 휴먼인더루프 지원 에이전트 작업 보드·컨트롤 플레인
- 장점: 워크스페이스 격리 MCP; 민감 작업 승인 요청; 워크플로 그래프
- 단점: AGPL 라이선스; 단일 에이전트엔 무거움; 네이티브 앱 없음
- 출처: <https://agentrq.com>, <https://agentrq.com/pricing>, <https://github.com/agentrq/agentrq>, <https://raw.githubusercontent.com/agentrq/agentrq/main/README.md>

### AgentsMesh
- 100+ 코딩 에이전트를 분산 스케줄링하는 엔터프라이즈 플랫폼, 칸반 티켓
- 장점: 네이티브 iOS 앱 제공; 대규모 분산 실행; 칸반-파드-PR 양방향 연동
- 단점: BSL 제한 라이선스; 인프라 무거움; 개인 사용엔 과도
- 출처: <https://agentsmesh.ai>, <https://agentsmesh.ai/docs>, <https://agentsmesh.ai/pricing>, <https://github.com/AgentsMesh/AgentsMesh>, <https://news.ycombinator.com/item?id=47249381>

### Agetor
- 여러 코딩 에이전트를 worktree에서 병렬 실행하는 로컬 오케스트레이터·칸반
- 장점: 설정 주입 없는 비침투 실행; tmux로 재시작 안전; 캔버스 파이프라인과 카드 연동
- 단점: macOS Apple Silicon 중심; 단일 사용자·팀 협업 없음; 모바일 푸시 없음
- 출처: <https://github.com/alamops/agetor>, <https://agetor.dev>, <https://github.com/alamops/agetor/releases>

### AGINEAR
- todo.md 기반 CLI·웹 칸반·MCP 서버를 결합한 마크다운 프로젝트 관리 도구
- 장점: 에이전트용 MCP로 토큰 낭비 감소; CST 파서로 무손실 마크다운 보존; 실시간 동기화 웹 GUI
- 단점: 소규모·초기 프로젝트(1인 개발); 멀티 에이전트 잠금 없음; 네이티브 모바일 앱 없음
- 출처: <https://www.adityabawankule.io/projects/aginear>, <https://github.com/Legorobotdude/AGINEAR>, <https://api.github.com/repos/Legorobotdude/AGINEAR>

### Agno AgentOS UI
- 에이전트를 서비스로 실행하는 런타임·컨트롤 플레인 UI
- 장점: 가볍고 빠른 런타임; MCP 서버 내장; 셀프호스팅 시 데이터 완전 소유
- 단점: 칸반 없음; 운영 모니터링 중심 UI; Phidata 리브랜딩으로 문서 혼선
- 출처: <https://github.com/agno-agi/agno>, <https://docs.agno.com>, <https://agno.com>

### AI Agent Board (DanWahlin)
- 드래그앤드롭으로 코딩 에이전트 CLI에 작업을 위임하는 오픈소스 칸반
- 장점: 설치된 에이전트 자동 감지; Task Groups 일괄 생성; worktree 격리·PR 자동 생성
- 단점: 신생·소규모 커뮤니티; 모바일 앱·PWA 없음; 수동 할당 방식
- 출처: <https://github.com/DanWahlin/ai-agent-board>

### AI Maestro
- 여러 머신의 터미널 코딩 에이전트 군단 오케스트레이션 OS, 칸반 포함
- 장점: MIT 완전 무료·텔레메트리 없음; 중앙 서버 없는 다중 머신 메시; 영구 메모리·코드 그래프
- 단점: 네이티브 모바일 앱 없음; Node.js 18+·tmux 3.0+ 필요; 멀티머신·클라우드 설정 복잡
- 출처: <https://ai-maestro.23blocks.com/>, <https://yetanotherorchestrator.app/apps/ai-maestro/>, <https://github.com/23blocks-OS/ai-maestro>, <https://ai-maestro.23blocks.com/mobile.html>

### AIF Handoff
- AI가 계획·구현·리뷰하는 자율 칸반, 사람 인계 게이트 포함
- 장점: 수렴 인식 자동 리뷰; 명시적 사람 인계 게이트; 역할별 서브에이전트 분리
- 단점: AI Factory 생태계 결합; 작은 커뮤니티; 로컬 CLI 인증 사전 설정 필요
- 출처: <https://github.com/lee-to/aif-handoff>, <https://aif.cutcode.dev>

### Aizen
- 브랜치별 격리 환경에서 AI 코딩 에이전트를 병렬 실행하는 네이티브 macOS 작업공간
- 장점: libghostty 기반 고성능 네이티브 터미널; ACP로 임의 에이전트·MCP 연결; git worktree 병렬 환경·tmux 세션 유지
- 단점: 칸반·작업 보드 없음; Apple Silicon macOS 전용; 2.0 구조 개편 중·팀 동기화 없음
- 출처: <https://aizen.win/>, <https://yetanotherorchestrator.app/apps/aizen>, <https://github.com/vivy-company/aizen>, <https://github.com/vivy-company/aizen/releases>

### amux
- 병렬 AI 코딩 에이전트 세션 멀티플렉서·대시보드, 원자적 작업 할당 칸반
- 장점: 외부 의존 없는 경량 Rust 바이너리; iOS 앱·PWA로 모바일 제어; 원자적 작업 클레임으로 경합 방지
- 단점: Commons Clause로 상업 재판매 제한; Android 네이티브 앱 없음; macOS/Linux·tmux 3.2+ 필요
- 출처: <https://amux.io/>, <https://yetanotherorchestrator.app/apps/amux/>, <https://github.com/mixpeek/amux>, <https://amux.io/pricing/>, <https://raw.githubusercontent.com/mixpeek/amux/main/LICENSE>, <https://apps.apple.com/us/app/amux-agent-multiplexer/id6760410435>

### Aperant
- 계획·구현·테스트·QA를 자동 수행하는 멀티 에이전트 코딩 프레임워크(구 Auto Claude)
- 장점: 병렬 에이전트용 칸반 보드; 계획→AI QA→사람 검토 자동화; 작업별 worktree 격리·토큰 관리
- 단점: 웹·모바일 없음; v3.0 재작성으로 PR 중단; Electron 리소스 부담·Claude Code 편중
- 출처: <https://aperant.com/>, <https://yetanotherorchestrator.app/apps/aperant>, <https://github.com/AndyMik90/Aperant>, <https://github.com/AndyMik90/Aperant/releases>

### Asana AI Studio & AI Teammates
- Asana 보드에 통합된 노코드 에이전트 자동화 빌더 및 자율 AI 에이전트
- 장점: 노코드 설정 간편; 보드·필드·담당자와 깊은 통합; 엔터프라이즈 보안
- 단점: 폐쇄형 생태계; 요청당 변동 비용 높음; 코드/CLI 실행 에이전트 대비 유연성 부족
- 출처: <https://asana.com/product/ai/ai-studio>, <https://asana.com/product/ai/ai-teammates>, <https://asana.com/pricing>, <https://help.asana.com/s/article/ai-studio>, <https://investors.asana.com/news-releases/news-release-details/asana-announces-ai-studio-no-code-builder-designing-and>

### Atlassian Rovo Agents & Rovo Dev / Coding Agent
- Teamwork Graph 기반 엔터프라이즈 AI 에이전트 플랫폼, Jira 코딩 에이전트 포함
- 장점: Jira·Confluence·30+ 커넥터 통합; 컬럼 진입·@멘션 등 다양한 트리거; 양방향 MCP 지원
- 단점: 클라우드 종속; 종량제 크레딧으로 비용 예측 어려움; Rovo Studio는 Premium 이상
- 출처: <https://www.atlassian.com/software/rovo>, <https://support.atlassian.com/rovo/docs/rovo-usage-limits/>, <https://support.atlassian.com/atlassian-ai-gateway/docs/use-atlassian-rovo-mcp-server/>, <https://support.atlassian.com/jira-software-cloud/docs/collaborate-on-work-items-with-ai-agents/>, <https://support.atlassian.com/jira-software-cloud/docs/what-is-the-jira-coding-agent/>, <https://support.atlassian.com/organization-administration/docs/add-an-external-mcp-server-from-atlassian-administration/>, <https://support.atlassian.com/atlassian-ai-gateway/docs/supported-tools/>, <https://support.atlassian.com/atlassian-ai-gateway/docs/set-up-clients/>

### Atrium
- 최대 49개 AI 코딩 에이전트를 병렬 실행하는 로컬 우선 작업공간·멀티플렉서
- 장점: 카드 내 실시간 에이전트 채팅 칸반; 가장 폭넓은 CLI 에이전트 지원; iOS 앱·스크립트형 JSON CLI
- 단점: 코어 클라이언트 비공개; GUI는 macOS·Windows만; 모바일은 iOS 비공개 베타·학습 곡선 가파름
- 출처: <https://getatrium.dev/>, <https://yetanotherorchestrator.app/apps/atrium>, <https://getatrium.dev/docs>, <https://getatrium.dev/docs/panes/tasks>, <https://getatrium.dev/docs/concepts/tasks>, <https://getatrium.dev/docs/reference/cli>, <https://getatrium.dev/docs/mobile>, <https://getatrium.dev/docs/adapters>

### AutoGen Studio
- AutoGen 기반 노코드 멀티에이전트 프로토타이핑 GUI(채팅 플레이그라운드)
- 장점: 완전 무료 로컬 UI; SQLite·PostgreSQL 지원; MCP 워크벤치 내장
- 단점: 칸반 없음; 유지보수 모드; 프로덕션용 아닌 연구 프로토타입
- 출처: <https://github.com/microsoft/autogen>, <https://raw.githubusercontent.com/microsoft/autogen/main/python/packages/autogen-studio/README.md>

### Automagik Forge
- 칸반으로 작업을 구성해 Git 워크트리별로 다중 에이전트에 배분하는 플랫폼
- 장점: 네이티브 MCP로 에이전트가 작업을 직접 조작; 작업별 격리 워크트리; Genie 계획 보조 기능
- 단점: 커뮤니티 규모가 작음; 모바일 앱 미출시; Node.js/pnpm 툴체인 필요
- 출처: <https://github.com/automagik-dev/forge>, <https://forge.automag.ik>

### Automaker
- 칸반 카드를 진행 중으로 옮기면 git 워크트리에서 코딩 에이전트가 자율 실행되는 스튜디오
- 장점: 카드 이동 시 에이전트 자동 실행; 격리된 git 워크트리; REST·WebSocket API 확장성
- 단점: 공식적으로 유지보수 중단; 로컬 Node/Electron 설정 부담; 네이티브 모바일 앱 없음
- 출처: <https://automaker.app/>, <https://yetanotherorchestrator.app/apps/automaker/>, <https://github.com/webdevcody/automaker>, <https://github.com/AutoMaker-Org/automaker>

### Backlog.md
- Git 저장소 내 마크다운 기반 작업 관리·칸반 보드(인간+AI 에이전트 협업)
- 장점: Git 네이티브·로컬 우선, 클라우드 종속 없음; Claude Code·Codex·Cursor용 MCP·CLI 지원; TUI·웹 칸반·JSON watch로 간편 설정
- 단점: 실시간 다중 사용자 협업·클라우드 동기화 없음; 모바일 앱·푸시 알림 없음; 원격 열람 시 터널링 필요
- 출처: <https://github.com/MrLesk/Backlog.md>, <https://backlog.md>, <https://github.com/MrLesk/Backlog.md/releases>, <https://raw.githubusercontent.com/MrLesk/Backlog.md/main/CLI-INSTRUCTIONS.md>, <https://github.com/MrLesk/Backlog.md/blob/main/backlog/docs/doc-003%20-%20Running-Backlog-Browser-as-a-Service.md>, <https://api.github.com/repos/MrLesk/Backlog.md>, <https://raw.githubusercontent.com/MrLesk/Backlog.md/main/package.json>

### Cairn
- GitHub 연동 백그라운드 에이전트가 작업 수행하는 칸반형 OSS
- 장점: 상용 백그라운드 에이전트의 OSS 대안; GitHub App 자체 제어; 다중 LLM 지원
- 단점: 2025년 중반 이후 업데이트 정체; 기능 단순; 모바일 앱 없음
- 출처: <https://try-cairn.com>, <https://github.com/cairn-dev/cairn>, <https://news.ycombinator.com/item?id=44415849>

### Catnip
- W&B가 만든 컨테이너 기반 하네스·대시보드로, Claude Code 다중 병렬 실행
- 장점: Docker/Apple Container 샌드박스 격리; Git 워크트리 자동 생성·동기화; 포트 자동 감지 및 프리뷰
- 단점: 원본 저장소 404로 미러에 의존; Claude Code 외 에이전트 미지원; iOS 앱은 TestFlight 베타만 제공
- 출처: <https://github.com/wandb/catnip>, <https://github.com/HarleyCoops/catnip>, <https://github.com/barstoolbluz/agentic-development-with-flox>, <https://github.com/bradAGI/awesome-cli-coding-agents>, <https://github.com/gabrielmoreira/awesome-ai-rabbit-holes>, <https://catnip.run>

### Caudex
- Claude Code 전용 경량 데스크톱 터미널·오케스트레이션 작업공간(탭 그리드, 칸반 없음)
- 장점: 무설정 Claude Code 훅 통합; 토큰·비용 실시간 모니터링; 터널 기반 PWA 원격 접속
- 단점: 비공개 소스; 칸반 보드 없음; Claude Code 전용(다중 에이전트 미흡)
- 출처: <https://caudex.sh/>, <https://caudex.sh/docs>, <https://yetanotherorchestrator.app/apps/caudex/>

### cc-connect
- 로컬 코딩 에이전트를 Telegram·Discord·Slack 등 메신저에 연결하는 게이트웨이
- 장점: 기존 메신저 앱에서 바로 사용; 채팅 버튼으로 권한 승인; 다중 세션 전환
- 단점: 호스트 데몬 필요; 시각적 칸반 없음; 봇 토큰 설정 필요
- 출처: <https://github.com/chenhg5/cc-connect>, <https://github.com/chenhg5/cc-connect/releases>

### cc-haha
- Claude Code용 로컬 데스크톱 워크스페이스(H5 모바일 웹·메신저 봇 포함)
- 장점: QR H5 웹+다중 메신저 봇; worktree·diff·MCP 관리; 화면 잠금에도 연결 유지
- 단점: 비OSI 라이선스(상업 재판매 제한); 데스크톱 앱 상시 실행 필요; 주로 중국어 UI
- 출처: <https://github.com/NanmiCoder/cc-haha>, <https://cchaha.ai>

### CCManager
- tmux 없이 병렬 AI 코딩 에이전트를 worktree별로 관리하는 경량 TUI
- 장점: tmux 의존성 없음; 8종 이상 에이전트 상태 감지; 상태 변경 훅으로 확장 용이
- 단점: 웹·원격 인터페이스 없음; 모바일 미지원; MCP 관리 계층 없음
- 출처: <https://github.com/kbwo/ccmanager>, <https://raw.githubusercontent.com/kbwo/ccmanager/main/README.md>, <https://raw.githubusercontent.com/kbwo/ccmanager/main/docs/status-hooks.md>, <https://raw.githubusercontent.com/kbwo/ccmanager/main/package.json>

### ccpoke
- 코딩 에이전트를 Telegram과 잇는 양방향 알림·채팅 브리지
- 장점: npx로 무설치 실행; 이벤트 기반 훅(폴링 없음); tmux로 양방향 응답·승인
- 단점: tmux 필요(Windows 복잡); Telegram 전용; 웹 대시보드·칸반 없음
- 출처: <https://github.com/kaida-palooza/ccpoke>

### ChatDev 2.0 (DevAll)
- 노드 기반 캔버스의 제로코드 멀티에이전트 워크플로 플랫폼
- 장점: 드래그앤드롭 워크플로 캔버스; YAML 제로코드 오케스트레이션; HITL 검토
- 단점: 칸반 없음; 토큰·연산 소모 큼; 백엔드·프론트 별도 실행
- 출처: <https://github.com/OpenBMB/ChatDev>, <https://chatdev.modelbest.cn/>

### Chorus
- AI-DLC 보드로 에이전트 제안·인간 검증·배포를 구조화하는 하네스
- 장점: AI-DLC 방법론; 제안·검증 상태 대시보드; 명시적 리뷰 게이트
- 단점: AGPL-3.0; 빠른 프로토타이핑엔 과정 부담; 전체 서버 스택 필요
- 출처: <https://github.com/Chorus-AIDLC/Chorus>, <https://doc.chorus-ai.dev>

### Claude Code Agent Monitor
- Claude Code·Cursor·Codex 세션 실시간 모니터링·칸반 대시보드
- 장점: 토큰·비용 분석 통합; 한국어·중국어 문서; 로컬 우선
- 단점: 작업 배분보다 관측 중심; 로컬 데몬 필요; 데스크톱 앱은 래퍼
- 출처: <https://hoangsonww.github.io/Claude-Code-Agent-Monitor/>, <https://github.com/hoangsonww/Claude-Code-Agent-Monitor>

### Claude Code on the web
- Anthropic 클라우드 샌드박스에서 Claude Code를 비동기 실행·세션 상태 확인
- 장점: 설치 없이 클라우드 병렬 실행; 공식 지원; 구독에 포함
- 단점: 칸반 아닌 세션 목록; Claude 전용; 셀프호스팅 불가
- 출처: <https://code.claude.com/docs/en/claude-code-on-the-web>

### Claude Code Telegram Bot (overwirehq/claude-code-telegram)
- Claude Code CLI를 원격 사용하는 경량 Python Telegram 봇
- 장점: uv로 가벼운 설치; 세션 컨텍스트 유지; 디렉터리 샌드박싱·푸시 알림
- 단점: Telegram 전용; 칸반·diff 뷰어 없음; 대형 코드 리뷰 불편
- 출처: <https://github.com/overwirehq/claude-code-telegram>

### Claude Code UI (CloudCLI)
- Claude Code·Codex 등 AI 코딩 에이전트 CLI 세션을 관리하는 웹 GUI/IDE
- 장점: 다중 에이전트 CLI 지원; TaskMaster 칸반 내장; npx 한 줄 로컬 실행
- 단점: 네이티브 모바일 앱 미출시; AGPL 카피레프트 제약; 팀 공유·상시 실행은 유료 클라우드 필요
- 출처: <https://github.com/siteboon/claudecodeui>, <https://cloudcli.ai>, <https://cloudcli.ai/docs>, <https://cloudcli.ai/claude-code-mobile>, <https://cloudcli.ai/download>, <https://github.com/siteboon/claudecodeui/releases>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/package.json>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/LICENSE>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/server/index.ts>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/server/modules/taskmaster/taskmaster.routes.ts>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/src/modules/task-master/TaskBoard.tsx>, <https://raw.githubusercontent.com/siteboon/claudecodeui/main/src/modules/task-master/utils/taskKanban.ts>, <https://github.com/szmidtpiotr/claude-github-issue>, <https://github.com/TadMSTR/cloudcli-plugin-task-queue>

### Claude Remote Runner
- Claude Code 세션을 PWA로 원격 제어하는 E2E 암호화 CLI 래퍼
- 장점: QR 즉시 페어링; WebRTC 저지연 스트리밍; 일시정지·승인 푸시 알림
- 단점: 정지 감지가 휴리스틱 의존; P2P/TURN 네트워크 조건 필요; 작은 커뮤니티
- 출처: <https://github.com/sohampawar1866/claude-remote-runner>, <https://www.npmjs.com/package/@sohampawar1866/remote-claude>

### Claude Squad
- tmux·git worktree 기반으로 여러 AI 코딩 에이전트를 병렬 관리하는 Go TUI
- 장점: 가벼운 단일 바이너리 TUI; worktree 자동 격리; 내장 diff·실시간 미리보기
- 단점: 칸반·웹 대시보드 없음; 모바일·원격 접근 불가; 팀 협업 기능 없음
- 출처: <https://github.com/smtg-ai/claude-squad>, <https://smtg-ai.github.io/claude-squad/>, <https://api.github.com/repos/smtg-ai/claude-squad>, <https://api.github.com/repos/smtg-ai/claude-squad/releases/latest>, <https://formulae.brew.sh/formula/claude-squad>

### Claude Task Viewer
- Claude Code 작업을 실시간 관찰하는 경량 웹 칸반 보드
- 장점: 설정 없이 즉시 사용; 의존성·블로커 시각화; 완료 알림음·브라우저 알림
- 단점: 관찰 위주로 세션 실행 불가; Claude Code 전용; 2026년 초 이후 개발 둔화
- 출처: <https://github.com/L1AD/claude-task-viewer>

### Clave
- 여러 AI 코딩 에이전트를 병렬 실행·관리하는 macOS 중심 오픈소스 데스크톱 워크스테이션
- 장점: 다중 CLI 에이전트 병렬 PTY 실행; MCP 도구 브리지 플러그인; MagicSync Git 패널
- 단점: macOS 중심(Windows 초기, Linux 미지원); 모바일·웹 클라이언트 없음; 커뮤니티 규모 작음
- 출처: <https://github.com/antasphere/clave>, <https://yetanotherorchestrator.app/apps/clave/>, <https://raw.githubusercontent.com/antasphere/clave/prod/README.md>, <https://raw.githubusercontent.com/antasphere/clave/prod/docs/adr/0003-server-and-client.md>

### Claw-Kanban
- 6개 에이전트 CLI에 역할 기반으로 작업을 배분하는 칸반
- 장점: 역할 기반 자동 배분; 한국어 문서 제공; AI 프롬프트로 설치
- 단점: 작은 커뮤니티; 로컬 CLI 인증 의존; 의존성 시각화 부족
- 출처: <https://github.com/GreenSheep01201/Claw-Kanban>, <https://github.com/GreenSheep01201/Claw-Kanban/blob/main/README.ko.md>

### ClawBridge
- OpenClaw 에이전트용 모바일 우선 미션 컨트롤 대시보드
- 장점: 모바일 우선 레이아웃; Cloudflare 터널·QR 무설정 접속; 토큰 비용 관리
- 단점: OpenClaw 생태계 종속; 네이티브 푸시 없음; 개발 속도 둔화
- 출처: <https://github.com/dreamwing/clawbridge>, <https://clawbridge.app/>

### ClickUp Brain & Agents
- ClickUp 내장 AI 에이전트 계층, 상태 업데이트·요약·작업 자동화
- 장점: 기존 보드에 설정 없이 통합; 조직 컨텍스트 자동 활용; 멀티모델(Claude·GPT·Gemini) 지원
- 단점: ClickUp 전용 폐쇄 생태계; 사용자별 추가 요금; 복잡한 자동화 시 크레딧 소모
- 출처: <https://clickup.com/brain>, <https://clickup.com/brain/pricing>, <https://clickup.com/pricing>, <https://clickup.com/security>

### CliDeck
- CLI 코딩 에이전트를 한 창에서 실행·모니터링하는 로컬 브라우저 터미널 대시보드
- 장점: 완전 로컬 실행, 텔레메트리 유출 없음; OTLP로 유휴/작업 상태 감지; 에이전트 간 위임(clideck ask)
- 단점: 칸반 보드 없음; v2에서 Autopilot 제거; 네이티브 모바일 앱·푸시 없음
- 출처: <https://clideck.dev/>, <https://yetanotherorchestrator.app/apps/clideck/>, <https://docs.clideck.dev>, <https://docs.clideck.dev/mobile>, <https://docs.clideck.dev/agents/overview.md>, <https://docs.clideck.dev/features/telemetry.md>, <https://docs.clideck.dev/features/sessions.md>, <https://docs.clideck.dev/features/autopilot.md>, <https://github.com/rustykuntz/clideck>

### Cline Kanban
- npx kanban으로 실행, CLI 에이전트를 worktree 병렬 실행하는 로컬 보드
- 장점: 무설정 즉시 실행; 카드별 worktree 자동 생성; 의존성 체인·자동 커밋
- 단점: 실험적·높은 권한 필요; 1인 로컬 전용; 네이티브 앱 없음
- 출처: <https://github.com/cline/kanban>, <https://raw.githubusercontent.com/cline/kanban/main/README.md>

### cmux
- 여러 AI 코딩 에이전트를 병렬로 실행하는 GPU 가속 macOS 네이티브 터미널
- 장점: Swift 네이티브 고성능; Claude Code Teams용 tmux 심; 세션 복원·에이전트용 브라우저
- 단점: macOS 전용; 칸반 보드 없음; 릴레이 서버 상용 운영 시 BUSL 라이선스 필요
- 출처: <https://cmux.com/>, <https://yetanotherorchestrator.app/apps/cmux/>, <https://github.com/manaflow-ai/cmux>, <https://raw.githubusercontent.com/manaflow-ai/cmux/main/LICENSE>, <https://cmux.com/docs/api>, <https://cmux.com/docs/notifications>, <https://cmux.com/docs/ios>, <https://cmux.com/docs/skills>, <https://cmux.com/docs/agent-integrations/claude-code-teams>

### CNVS
- AI 코딩 에이전트 다수를 감독하는 macOS 공간 캔버스 앱(음성 제어)
- 장점: 캔버스 기반 시각적 정리; 음성 제어; 일회성 결제
- 단점: macOS 전용; 비공개 소스; 칸반 보드 없음
- 출처: <https://cnvs.dev/>, <https://yetanotherorchestrator.app/apps/cnvs/>, <https://cnvs.dev/release/>, <https://cnvs.dev/release/1-5-2-distraction-guard/>, <https://cnvs.dev/release/1-4-0-shorthand/>

### CodeAgent Mobile
- 티켓·채팅을 에이전트 실행으로 전환하는 모바일 감독·원격 제어 플랫폼
- 장점: QR로 IDE/CLI 간편 연동; Jira·Linear·Slack에서 작업 시작; 모바일 PR 리뷰·병합
- 단점: 전통적 칸반 대신 작업 그래프(Beads); 무료 티어 제한(일 3작업); 릴레이 완전 자체 호스팅 불가
- 출처: <https://www.codeagent-mobile.com>, <https://www.codeagent-mobile.com/pricing>, <https://www.codeagent-mobile.com/faq>, <https://www.codeagent-mobile.com/terms>, <https://marketplace.visualstudio.com/items?itemName=CodeAgentMobile.codeagent-mobile>, <https://plugins.jetbrains.com/plugin/30697-codeagent-mobile/versions/stable/1181091>

### CodeAgentSwarm
- 여러 코딩 에이전트를 병렬 실행하는 데스크톱 ADE, 에이전트용 작업 보드 내장
- 장점: 에이전트가 직접 쓰는 작업 보드; 실시간 diff·대화 기록 검색; 3개 OS 지원
- 단점: 모바일 없음; 베타 이후 가격 불명; 소스 비공개
- 출처: <https://www.codeagentswarm.com/en>

### CodeLayer
- 워크트리 기반 병렬 에이전트 세션과 리뷰를 지원하는 멀티플레이어 코딩 IDE
- 장점: RPI 단계별 칸반 보드; 작업별 git 워크트리 격리; Linear·GitHub·Jira 연동
- 단점: Pro 가격 높음; 구성 요소가 복잡함; 클라우드 백엔드 비공개
- 출처: <https://humanlayer.dev/code>, <https://humanlayer.dev/>, <https://yetanotherorchestrator.app/apps/codelayer/>, <https://docs.humanlayer.com/>, <https://docs.humanlayer.com/release-notes.html>, <https://docs.humanlayer.com/explanation/tasks>, <https://docs.humanlayer.com/guide/automation-sessions>, <https://docs.humanlayer.com/reference/skills-workflows>, <https://docs.humanlayer.com/guide/linear-integration>, <https://docs.humanlayer.com/explanation/remote-daemons>, <https://github.com/humanlayer/homebrew-humanlayer>

### CodeNomad
- OpenCode V2용 멀티 인스턴스 데스크톱·웹 오케스트레이터(병렬 세션·워커·git 브랜치)
- 장점: OpenCode V2 네이티브 통합·서브에이전트 오케스트레이션; Git worktree 격리·멀티 인스턴스; 웹 프리뷰·원격 터널 지원
- 단점: 칸반 보드 없음; OpenCode V2 전용; macOS 바이너리 미서명
- 출처: <https://github.com/NeuralNomadsAI/CodeNomad>, <https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/README.md>, <https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/package.json>, <https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/packages/ui/src/components/instance/instance-shell2.tsx>, <https://yetanotherorchestrator.app/apps/codenomad/>

### Coder Agents (formerly Coder Tasks)
- Coder 플랫폼 내장 엔터프라이즈 셀프호스트 AI 코딩 에이전트 실행·오케스트레이션
- 장점: 격리 샌드박스·에이전트 방화벽 거버넌스; Coder 템플릿 기반 환경; Agent Relay로 외부 에이전트 안전 실행
- 단점: 칸반 아닌 대화/타임라인 UI; Coder 인프라 구축 부담; 고급 보안·무제한 동시성은 유료
- 출처: <https://coder.com/pricing>, <https://coder.com/pricing.md>, <https://coder.com/docs/ai-coder/agents>, <https://coder.com/docs/ai-coder/agents/licensing-usage>, <https://coder.com/docs/ai-coder/agents/tasks-to-chats-migration>, <https://coder.com/docs/ai-coder/agent-relay>, <https://coder.com/blog/launch-week-2025-introducing-coder-tasks>, <https://coder.com/blog/automate-coder-tasks-via-cli-and-api>, <https://github.com/coder/coder>

### Coder Mux (Xum)
- 병렬 코딩 에이전트용 오픈소스 데스크톱/헤드리스 멀티플렉서(격리 워크스페이스·git 모니터링)
- 장점: 에이전트 전용 병렬 워크스페이스 오케스트레이션; worktree·SSH·Docker 지원; MCP·ACP 표준 프로토콜 지원
- 단점: 칸반 보드 아닌 사이드바/채팅 UI; 모바일은 브라우저만(푸시 없음); 신생 프로젝트(Mux→Xum 개명)
- 출처: <https://github.com/coder/mux>, <https://github.com/coder/xum>, <https://raw.githubusercontent.com/coder/mux/main/README.md>, <https://mux.coder.com>, <https://xum.cdr.dev/llms.txt>, <https://xum.cdr.dev/index.md>, <https://xum.cdr.dev/reference/cli.md>, <https://xum.cdr.dev/config/server-access.md>, <https://xum.cdr.dev/config/mcp-servers.md>, <https://xum.cdr.dev/integrations/acp.md>, <https://xum.coder.com/integrations/vscode-extension.md>

### Codex Monitor
- OpenAI Codex용 경량 데스크톱·모바일 컴패니언(워크스페이스·worktree·diff 리뷰)
- 장점: Rust/Tauri 기반 고성능; Codex 서브에이전트·포크 완전 지원; Tailscale 원격 iOS 모니터링
- 단점: 칸반 보드 없음; OpenAI Codex 전용; iOS 앱 정식 출시 전(TestFlight)
- 출처: <https://www.codexmonitor.app/>, <https://github.com/Dimillian/CodexMonitor>, <https://raw.githubusercontent.com/Dimillian/CodexMonitor/main/README.md>, <https://yetanotherorchestrator.app/apps/codexmonitor/>

### Collattice (MrBildo)
- SQLite·SSE·MCP 내장 단일 실행파일 인간-에이전트 협업 칸반
- 장점: Docker·DB 없는 단일 바이너리 배포; SSE로 에이전트 작업 실시간 표시; HMAC 웹훅
- 단점: 신생 프로젝트로 사용자 적음; 다중 프로젝트 관리 부족; 분산 구성 미지원
- 출처: <https://github.com/MrBildo/collattice>

### Conductor
- 격리된 Git 워크트리·클라우드 샌드박스에서 병렬 코딩 에이전트를 실행·관리하는 플랫폼
- 장점: Git 워크트리 격리로 에이전트 간 충돌 방지; 주요 코딩 에이전트 기본 지원; 데스크톱·iOS·웹·MCP로 상태 모니터링
- 단점: 로컬 실행은 macOS 전용; 클라우드·API·모바일은 유료($50+/월); 비공개 소스
- 출처: <https://conductor.build>, <https://conductor.build/docs>, <https://conductor.build/pricing>, <https://conductor.build/changelog>, <https://conductor.build/docs/api>, <https://conductor.build/docs/api/mcp>, <https://conductor.build/docs/concepts/workspaces-and-branches>, <https://conductor.build/docs/concepts/parallel-agents>, <https://conductor.build/docs/concepts/workflow>, <https://conductor.build/docs/concepts/git-worktrees>, <https://conductor.build/docs/cloud>, <https://conductor.build/terms>, <https://conductor.build/privacy>, <https://conductor.build/docs/reference/privacy>, <https://apps.apple.com/us/app/conductor-build/id6791228564>, <https://conductor.build/blog>

### Constellagent
- Claude Code 다중 인스턴스를 git worktree별로 실행하는 macOS 데스크톱 환경
- 장점: Claude Code 병렬 실행·worktree 자동 격리; Monaco 에디터·diff 뷰어 내장; 크론 스케줄링·PR 모니터링
- 단점: 칸반 보드 없음; macOS 전용; 배포 바이너리 없어 소스 빌드 필요
- 출처: <https://constellagent.vercel.app/>, <https://github.com/owengretzinger/constellagent>, <https://raw.githubusercontent.com/owengretzinger/constellagent/main/README.md>, <https://raw.githubusercontent.com/owengretzinger/constellagent/main/package.json>, <https://yetanotherorchestrator.app/apps/constellagent/>

### CrewAI AMP & Studio
- 역할 기반 멀티에이전트 팀 오케스트레이션·배포·모니터링 플랫폼
- 장점: 역할 기반 협업·자율 위임; 이벤트 기반 Flows; AMP 실행 트레이스
- 단점: 칸반 보드 없음; AMP 온프레미스 상용·비공개; Python 생태계 의존
- 출처: <https://github.com/crewAIInc/crewAI>, <https://docs.crewai.com>, <https://docs-platform.crewai.com/>, <https://crewai.com/amp>, <https://crewai.com/pricing>

### Crystal (stravu/crystal) / Nimbalyst
- Claude Code·Codex 등 병렬 AI 세션 관리 워크스페이스, 세션·태스크 칸반
- 장점: 전용 세션·태스크 칸반 보드; 원클릭 git worktree 격리; 네이티브 모바일 앱·푸시 승인
- 단점: Crystal→Nimbalyst 리브랜딩 혼란; 무거운 Electron 번들; OpenCode·Copilot 연동 알파
- 출처: <https://github.com/stravu/crystal>, <https://github.com/Nimbalyst/nimbalyst>, <https://nimbalyst.com/>, <https://nimbalyst.com/features/session-management/>, <https://nimbalyst.com/features/task-management/>, <https://nimbalyst.com/kanban-for-claude-code/>, <https://nimbalyst.com/teams/>, <https://docs.nimbalyst.com/>, <https://api.github.com/repos/Nimbalyst/nimbalyst>, <https://api.github.com/repos/stravu/crystal>

### Cursor Cloud Agents (formerly Background Agents) / Cursor Agents Dashboard
- 격리된 클라우드 VM에서 병렬 코딩 에이전트를 실행·모니터링하고 PR까지 생성하는 대시보드
- 장점: 작업별 전용 VM 샌드박스; 웹·데스크톱·iOS·Slack·GitHub·Linear 다중 제어; 스크린샷·녹화·로그·PR diff 아티팩트 제공
- 단점: 중앙 오케스트레이터는 비공개 SaaS로 완전 자체호스팅 불가; 최소 $30/월 플랜 및 별도 토큰 과금; 전통적 칸반 보드 없음, Android 네이티브 앱 없음
- 출처: <https://cursor.com/pricing>, <https://docs.cursor.com>, <https://cursor.com/docs/agent>, <https://cursor.com/docs/agent/agents-window>, <https://cursor.com/docs/cloud-agent>, <https://cursor.com/docs/cloud-agent/mobile>, <https://cursor.com/docs/cloud-agent/self-hosted>, <https://cursor.com/docs/cloud-agent/capabilities>, <https://cursor.com/docs/cloud-agent/api/endpoints>, <https://cursor.com/docs/cloud-agent/setup>, <https://cursor.com/docs/cloud-agent/automations>, <https://cursor.com/docs/integrations/linear>, <https://cursor.com/docs/integrations/slack>, <https://cursor.com/docs/cli/overview>, <https://apps.apple.com/app/cursor/id6767085653>

### Dev 3.0
- 카드마다 worktree·tmux에서 코딩 에이전트가 실행되는 미션 컨트롤 칸반
- 장점: 카드-에이전트 프로세스 1:1 매핑; worktree 격리; 카드 내 터미널 스트리밍
- 단점: Unix/macOS 종속; 코딩 작업 전용; 와이드 데스크톱 지향 UI
- 출처: <https://github.com/h0x91b/dev-3.0>, <https://raw.githubusercontent.com/h0x91b/dev-3.0/main/README.md>

### Dev Boards (Agent Relay)
- poll·claim·heartbeat·escalate·deliver 계약 기반 AI 에이전트용 칸반
- 장점: 단순한 5동사 REST 프로토콜; 409 기반 원자적 작업 점유; 내장 PGLite로 무설정 시작
- 단점: 초기 단계로 사양 변동; 모바일 앱 없음; 에이전트 실행 샌드박스 제한
- 출처: <https://github.com/GumbyEnder/agent-relay>, <https://raw.githubusercontent.com/GumbyEnder/agent-relay/main/README.md>, <https://raw.githubusercontent.com/GumbyEnder/agent-relay/main/LICENSE>

### Devin (Cognition) Sessions Dashboard
- Devin 자율 코딩 세션 실시간 대시보드(IDE·셸·브라우저·작업로그)
- 장점: IDE·셸·브라우저 통합 작업공간; 공식 MCP·핸드오프로 타 에이전트 연동; RBAC·감사·Outposts
- 단점: 독점 코어, 완전 셀프호스팅 불가; 대량 작업 시 고비용; 네이티브 모바일 앱 없음
- 출처: <https://devin.ai>, <https://docs.devin.ai>, <https://docs.devin.ai/admin/billing/self-serve.md>, <https://docs.devin.ai/work-with-devin/devin-session-tools.md>, <https://docs.devin.ai/work-with-devin/devin-mcp.md>, <https://docs.devin.ai/work-with-devin/devin-handoff.md>, <https://docs.devin.ai/cloud/outposts/overview.md>, <https://github.com/club-cog/devin-handoff>, <https://github.com/CognitionAI/devin-outpost-k8s>, <https://docs.devin.ai/release-notes/overview.md>

### Dify
- DAG 워크플로·RAG·에이전트를 통합한 오픈소스 LLM 앱 개발 플랫폼
- 장점: RAG·에이전트·워크플로 올인원; 직관적 DAG 캔버스; 간편한 Docker 셀프호스팅
- 단점: 칸반 없음; 셀프호스팅 리소스 부담 큼; 관리 콘솔 모바일 부적합
- 출처: <https://github.com/langgenius/dify>, <https://dify.ai>, <https://docs.dify.ai>

### Dispatch Board
- 카드를 사람 또는 AI 에이전트에 배정해 자동 실행하는 협업 칸반
- 장점: Trello형 익숙한 UI; 사람·AI 담당자 선택; 프롬프트 지식 불필요
- 단점: 베타 단계; 클라우드 전용; CLI 에이전트 연동 정보 부족
- 출처: <https://www.dispatchboard.ai>

### dmux
- tmux와 git worktree로 여러 AI 코딩 에이전트를 병렬 실행하는 터미널 멀티플렉서
- 장점: worktree 격리로 충돌 방지; 12개 이상 CLI 에이전트 지원; AI 충돌 해결 자동 병합
- 단점: 칸반 보드 없음; tmux·단축키 숙련 필요; Windows 지원 제한(WSL 필요)
- 출처: <https://dmux.ai/>, <https://github.com/formkit/dmux>, <https://github.com/standardagents/dmux>, <https://yetanotherorchestrator.app/apps/dmux/>

### Dorothy
- 10개 이상의 AI 코딩 에이전트를 병렬 조율하는 오픈소스 데스크톱 오케스트레이터
- 장점: 칸반 전용 MCP 서버(8개 도구); 스킬 기반 자동 작업 할당; Telegram·Slack 원격 제어
- 단점: macOS 빌드 위주(Linux는 소스 빌드); 웹·모바일 보드 없음; 2026-07 이후 업데이트 없음
- 출처: <https://dorothyai.app/>, <https://yetanotherorchestrator.app/apps/dorothy/>, <https://github.com/Charlie85270/Dorothy>, <https://raw.githubusercontent.com/Charlie85270/Dorothy/main/README.md>

### Ductor
- 공식 코딩 에이전트 CLI를 Telegram·Matrix·Slack으로 제어하는 게이트웨이
- 장점: 공식 CLI 직접 실행(구독 활용); Matrix·Slack 지원; Docker 샌드박스
- 단점: 채팅 출력이 장황; 시각적 작업 보드 없음; 봇 자격증명 설정 필요
- 출처: <https://github.com/PleasePrompto/ductor>, <https://ductor.dev>

### Emdash
- 격리된 Git 워크트리에서 여러 AI 코딩 에이전트를 병렬 실행하는 오픈소스 데스크톱 ADE
- 장점: 완전 무료·Apache-2.0; macOS·Windows·Linux 지원; 가장 넓은 에이전트 지원과 MCP 중앙 관리
- 단점: 모바일·웹·PWA 없음; 외부 에이전트용 인바운드 MCP 서버 없음; 전통적 다중 컬럼 칸반 아님
- 출처: <https://emdash.sh>, <https://github.com/generalaction/emdash>, <https://raw.githubusercontent.com/generalaction/emdash/main/README.md>, <https://raw.githubusercontent.com/generalaction/emdash/main/AGENTS.md>, <https://emdash.sh/docs>, <https://emdash.sh/docs/providers>, <https://emdash.sh/docs/tasks>, <https://emdash.sh/docs/issues>, <https://emdash.sh/docs/library/mcp>, <https://emdash.sh/docs/library/skills>, <https://emdash.sh/docs/remote-projects>, <https://emdash.sh/docs/in-app-browser>, <https://emdash.sh/docs/automations>, <https://raw.githubusercontent.com/generalaction/emdash/main/LICENSE.md>, <https://github.com/generalaction/emdash/releases>

### Factory.ai Droids (Software Factory & Missions)
- 자율 AI 에이전트(Droid)가 계획·구현·리뷰·테스트·배포하는 에이전트 네이티브 개발 플랫폼
- 장점: Missions 기반 멀티에이전트 오케스트레이션; Linear/Jira 카드 상태 자동 전환; Mission Control 실시간 대시보드
- 단점: 핵심 플랫폼 폐쇄형; 네이티브 모바일 앱 없음; 범용 칸반 아님(Linear/Jira 의존)
- 출처: <https://factory.ai>, <https://docs.factory.com/llms.txt>, <https://docs.factory.com/welcome/index.md>, <https://docs.factory.com/pricing/individuals.md>, <https://docs.factory.com/pricing/organizations.md>, <https://docs.factory.com/missions/overview.md>, <https://docs.factory.com/missions/running-app.md>, <https://docs.factory.com/delegations/index.md>, <https://docs.factory.com/delegations/linear.md>, <https://docs.factory.com/harness/mcp.md>, <https://docs.factory.com/api-reference.md>, <https://docs.factory.com/api-reference/sessions.md>, <https://docs.factory.com/ide-integrations.md>, <https://docs.factory.com/enterprise/network-and-deployment.md>, <https://docs.factory.com/enterprise/airgapped-deployment.md>, <https://docs.factory.com/changelog/release-notes.md>, <https://docs.factory.com/factory-app/overview.md>, <https://docs.factory.com/software-factory/overview.md>, <https://github.com/Factory-AI/factory>, <https://docs.factory.com/droid-exec/overview.md>

### Flux (sirsjg)
- 인간·AI용 Git 네이티브 칸반, 단일 JSON 파일에 작업 저장, MCP·CLI·웹 UI
- 장점: Git 기반 저장으로 CI·오프라인 친화; 에이전트 메모리·의존성 지원; CLI와 MCP 기능 동등
- 단점: 대규모 다중 에이전트 시 병합 충돌 우려; 모바일 알림 없음
- 출처: <https://github.com/sirsjg/flux>

### Fusion
- 명세·계획·실행·리뷰·자동 병합을 자동화하는 멀티노드 에이전트 오케스트레이터
- 장점: 칸반·의존성 그래프·비용 관측; 다단계 품질 게이트와 worktree 격리; 모바일·데스크톱·웹 지원
- 단점: 내장 PostgreSQL 등 무거운 구성; 학습 곡선 가파름; 리뷰 게이트가 까다로움
- 출처: <https://runfusion.ai/>, <https://yetanotherorchestrator.app/apps/fusion/>, <https://github.com/Runfusion/Fusion>, <https://github.com/Runfusion/Fusion/blob/main/CHANGELOG.md>, <https://github.com/Runfusion/Fusion/blob/main/package.json>, <https://github.com/Runfusion/Fusion/blob/main/packages/core/package.json>

### Gander
- 픽셀아트 오피스형 AI 에이전트 관제 대시보드(/phone 콘솔·Telegram 알림)
- 장점: 배터리 절약형 /phone 페이지; Telegram에서 승인·/stop; 비용 폭주 모니터
- 단점: 초기 단계 소수 사용자; 픽셀아트 뷰는 폰에서 복잡; 원격 접속 시 토큰 설정 필요
- 출처: <https://github.com/weellio/gander>

### Gas Town
- Beads 이슈 추적으로 20~30개 코딩 에이전트를 조율하는 Git 기반 시스템
- 장점: 에이전트 크래시에도 작업 보존; 확장성 있는 감시 구조; 머지 큐·OpenTelemetry 내장
- 단점: 용어가 낯설어 학습 곡선 가파름; tmux·Go·Dolt 필요; 드래그앤드롭 칸반 아님
- 출처: <https://github.com/steveyegge/gastown>, <https://github.com/steveyegge/gastown/blob/main/docs/glossary.md>

### GitHub Copilot Agents Panel / Agent HQ (Mission Control)
- 여러 저장소의 비동기 에이전트 세션을 시작·관찰·조정하는 중앙 대시보드
- 장점: 저장소 전반 세션 통합 가시성; 실시간 로그·진행 중 조정; 세션-커밋 감사 추적
- 단점: 세션 기록 영구 삭제 불가; 네이티브 칸반 컬럼 없음; 조정 메시지 추가 과금
- 출처: <https://github.com/features/copilot/agents>, <https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents>, <https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/kick-off-a-task>, <https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/overview>, <https://apps.apple.com/us/app/github/id1477376905>

### GitHub Copilot Coding Agent (Cloud Agent)
- GitHub Actions 임시 환경에서 코드 작성·테스트 후 PR을 여는 클라우드 코딩 에이전트
- 장점: GitHub 네이티브 PR·브랜치 자동화; 보안 스캔·테스트 자동 검증; Claude·Codex 위임 지원
- 단점: 셀프호스팅 불가; AI 크레딧·Actions 분 과금; 서드파티 에이전트 조정 미지원
- 출처: <https://github.com/features/copilot/agents>, <https://github.com/features/copilot/plans>, <https://docs.github.com/en/copilot/concepts/agents/cloud-agent>, <https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/kick-off-a-task>, <https://docs.github.com/en/copilot/how-tos/copilot-on-github/use-copilot-agents/manage-and-track-agents>, <https://apps.apple.com/us/app/github/id1477376905>

### GitHub Mobile (Copilot Workspace / Agent PRs)
- Copilot 에이전트 PR 모니터링·diff 리뷰·승인을 하는 공식 모바일 앱
- 장점: 즉시 푸시 알림; 고품질 모바일 diff 뷰어; 엔터프라이즈 SSO·위젯
- 단점: 로컬 CLI 에이전트 실행 불가; 리뷰·승인 위주; 상용 클라우드
- 출처: <https://github.com/mobile>, <https://github.com/features/copilot>

### Google Jules
- Google Labs의 비동기 자율 코딩 에이전트·작업 대시보드(클라우드 VM 실행, PR 생성)
- 장점: 격리된 클라우드 VM에서 자율 실행; GitHub 이슈 라벨·PR·CI 실패 자동 수정 연동; 웹 대시보드·TUI·REST API 지원
- 단점: 셀프호스팅 불가 클라우드 전용; 유료 플랜은 개인 @gmail.com 계정만 가능; Gemini 모델 전용, MCP는 Google 선별 목록만
- 출처: <https://jules.google/>, <https://jules.google/docs/>, <https://jules.google/docs/running-tasks/>, <https://jules.google/docs/review-plan/>, <https://jules.google/docs/environment/>, <https://jules.google/docs/usage-limits/>, <https://jules.google/docs/changelog/>, <https://jules.google/docs/faq/>, <https://jules.google/docs/api/reference/>, <https://jules.google/docs/api/reference/overview/>, <https://jules.google/docs/api/reference/sessions/>, <https://jules.google/docs/api/reference/activities/>, <https://jules.google/docs/api/reference/types/>, <https://jules.google/docs/cli/reference/>, <https://jules.google/docs/integrations/>, <https://jules.google/docs/integrations/render/>, <https://jules.google/docs/tasks-repos/>, <https://jules.google/docs/repo/>, <https://jules.google/docs/scheduled-tasks/>, <https://jules.google/docs/suggested-tasks/>, <https://jules.google/docs/errors/>, <https://jules.google/docs/code/>, <https://jules.google/docs/guides/continuous-ai-overview/>, <https://github.com/google-labs-code/jules-awesome-list>, <https://policies.google.com/terms>, <https://one.google.com/about/plans>

### Granular
- 비개발자용 로컬 우선 데스크톱 워크스페이스로 AI 에이전트 팀을 운영하는 도구
- 장점: roadmap.md 기반 칸반 양방향 동기화; git worktree 병렬 세션; BYO 키·로그인 지원
- 단점: 비공개 소스 유료 라이선스; 모바일·웹 보드 없음; Windows 베타
- 출처: <https://granular.build/>, <https://yetanotherorchestrator.app/apps/granular/>, <https://granular.build/pricing/>, <https://granular.build/deal/>, <https://granular.build/download/>, <https://granular.build/docs/>, <https://granular.build/docs/how-the-roadmap-board-works/>

### Hamloy
- 에이전트 조정 작업 보드+질문함, 막힐 때 인간에 선택지 질의
- 장점: 에이전트→인간 질의 메커니즘; 가정·완료 로그 투명; 모바일 결정 UI
- 단점: 비공개 SaaS; 무료는 1개 프로젝트; 컬럼 커스터마이즈 제한
- 출처: <https://hamloy.com>, <https://hamloy.com/pricing>, <https://api.smithery.ai/servers/hamloy/hamloy>

### Happy (formerly Happy Coder / slopus/happy)
- Claude Code·Codex·Grok를 모바일에서 원격 제어·승인하는 E2E 암호화 OSS 하네스
- 장점: E2E 암호화(제로지식 릴레이); 기존 구독 재사용(추가 과금 없음); 네이티브 모바일 앱·알림·diff 리뷰
- 단점: 칸반 보드 아님(세션 목록 방식); CLI 설치·QR 페어링 필요; 음성 기능 유료 제한
- 출처: <https://happy.engineering>, <https://happy.engineering/welcome/>, <https://happy.engineering/how-it-works/>, <https://happy.engineering/mobile/>, <https://happy.engineering/voice/>, <https://happy.engineering/multiplayer/>, <https://happy.engineering/docs/>, <https://happy.engineering/docs/security/>, <https://happy.engineering/guides/remote-server/>, <https://github.com/slopus/happy>, <https://github.com/slopus/happy/blob/main/packages/happy-cli/README.md>, <https://github.com/slopus/happy/blob/main/packages/happy-server/README.md>, <https://api.github.com/repos/slopus/happy>, <https://api.github.com/repos/slopus/happy-desktop>, <https://api.github.com/repos/slopus/happy-agent>, <https://api.github.com/repos/slopus/happy/releases/latest>, <https://apps.apple.com/us/app/happy-codex-claude-code-app/id6748571505>, <https://play.google.com/store/apps/details?id=com.ex3ndr.happy>

### Height
- AI 워커가 작업을 자동 정리·분류·갱신·종료하는 자율형 프로젝트 관리 도구
- 장점: 자율 AI PM의 선구자; 스프레드시트+칸반형 UI; 자동 분류·중복 감지
- 단점: 서비스 영구 종료; 셀프호스팅 불가; 공식 MCP 서버 없음
- 출처: <https://height.app>, <https://github.com/api-evangelist/height>, <https://raw.githubusercontent.com/adamski/height-to-linear-export/main/README.md>, <https://raw.githubusercontent.com/adamski/height-to-linear-export/main/IMPORT_GUIDE.md>

### Herdr
- Rust 기반 에이전트 전용 터미널 런타임·멀티플렉서 데몬(지속 세션 관리)
- 장점: 클라이언트 분리 후에도 세션 유지; 화면 분석으로 상태 자동 추론; SSH 기반 다중 머신 연동
- 단점: 칸반/보드 UI 없음; GUI·모바일 앱 없음; 휴리스틱 상태 감지 오분류 가능
- 출처: <https://herdr.dev/>, <https://herdr.dev/docs/>, <https://herdr.dev/docs/agents/>, <https://herdr.dev/docs/socket-api/>, <https://herdr.dev/docs/connecting-machines/>, <https://herdr.dev/blog/herdr-raised-a-seed/>, <https://herdr.dev/compare/>, <https://github.com/herdrdev/herdr>, <https://github.com/herdrdev/herdr/releases>, <https://yetanotherorchestrator.app/apps/herdr/>

### Herdr Connect
- LAN/Tailscale로 Herdr 에이전트를 모니터링·제어하는 iOS 앱
- 장점: 네이티브 iOS·햅틱 완료 알림; 클라우드·계정·텔레메트리 없음; 폰에서 턴 중단·후속 지시
- 단점: iOS 전용; 같은 Wi-Fi 또는 Tailscale/VPN 필요; Herdr 설치 필요
- 출처: <https://github.com/Tomyail/herdr-connect>, <https://testflight.apple.com/join/ZkRzJ6rm>

### Herdr Mobile Relay
- 여러 호스트의 Herdr 에이전트를 하나의 모바일 PWA로 통합 제어
- 장점: 다중 컴퓨터 통합 모니터링; 계정 없는 WebRTC 게이트웨이; 웹 푸시·QR 설정
- 단점: Windows 미지원; Herdr 0.7.5+ 필요; AGPL 카피레프트
- 출처: <https://github.com/0cv/herdr-mobile-relay>, <https://herdr.dev>

### Hermes Agent Kanban (NousResearch)
- Hermes Agent 내장 멀티에이전트 작업 보드·스웜 실행 엔진
- 장점: 에이전트 런타임 내장; 워커→검증→합성 패턴; 영속 SQLite 큐
- 단점: 기본 UI는 CLI뿐; 프로필·스킬 설정 필요; 토큰 소모 큼
- 출처: <https://zenn.dev/zennai_ryutaro/articles/20260525-hermes-kanban-swarm-usage>, <https://juejin.cn/post/7661325178647330856>, <https://github.com/NousResearch/hermes-agent>, <https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban>

### Hermes KanbanWebUI
- Hermes Agent 칸반 DB용 트렐로형 WebUI, AI 워크플로 DAG 설계
- 장점: 기존 Hermes DB 공유; 실행 전 DAG 미리보기; 부모-자식 의존관계 시각화
- 단점: Hermes 생태계 종속; 엔터프라이즈 접근제어 없음; 네이티브 모바일 앱 없음
- 출처: <https://news.hada.io/topic?id=29268>, <https://github.com/PriuS2/HermesKanban>, <https://zenn.dev/zennai_ryutaro/articles/20260525-hermes-kanban-swarm-usage>

### hubd
- 사람·AI 혼합 팀용 플레인 파일 기반 프로젝트 추적 허브. 읽기 전용 칸반 뷰
- 장점: git 커밋 가능한 플레인 파일; 에이전트 러너와 분리; 역할별 큐
- 단점: 칸반 읽기 전용; 초기 단계; 파일 스키마 준수 필요
- 출처: <https://github.com/bzdOS/hubd>, <https://hubd.net>, <https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md>

### is.team
- 에이전트가 팀원으로 참여하는 무한 캔버스형 칸반·문서 공간
- 장점: 무한 캔버스+칸반; 좌석당 아닌 정액 요금; 카드 내 에이전트 채팅
- 단점: 작은 화면에서 복잡; 비공개 소스; 오프라인 모드 없음
- 출처: <https://is.team>, <https://is.team/pricing>

### jat
- tmux 기반 로컬 우선 에이전트 IDE, 20+ 병렬 에이전트 감독·병합 큐
- 장점: 에디터·diff·태스크·터미널 통합 경량 IDE; Slack·Telegram 등 트리거; worktree 격리와 직렬 병합 큐
- 단점: 칸반 보드 없음; macOS·Linux 전용; 1인 개발로 버스 팩터 위험
- 출처: <https://jat.tools/>, <https://jat.tools/docs/jat>, <https://jat.tools/reference>, <https://jat.tools/docs/jst>, <https://jat.tools/docs/jat-pilot>, <https://jat.tools/docs/jat-appliance>, <https://jat.tools/docs/jat-secret>, <https://jat.tools/docs/jat-run>, <https://jat.tools/docs/jatstation>, <https://yetanotherorchestrator.app/apps/jat/>, <https://rywalker.com/research/jat>, <https://github.com/jatstack/jat>

### Jean
- git worktree별로 여러 코딩 에이전트를 병렬 실행하는 오픈소스 개발 환경
- 장점: 완전 무료·기능 제한 없음; 헤드리스 서버·Docker로 원격 웹 접근; GitHub·Linear 이슈 연동
- 단점: 전통적 다열 칸반 아님(Project Canvas); Windows·Linux 빌드 테스트 부족; 네이티브 모바일 앱 없음
- 출처: <https://jean.build/>, <https://yetanotherorchestrator.app/apps/jean/>, <https://github.com/coollabsio/jean>, <https://raw.githubusercontent.com/coollabsio/jean/main/README.md>, <https://jean.build/docs>, <https://jean.build/docs/workflows/project-canvas-and-chat>, <https://jean.build/docs/core-concepts/projects-worktrees-and-sessions>, <https://jean.build/docs/manage/web-access-and-headless>

### Kaiban Board (KaibanJS)
- 멀티 에이전트 상태를 칸반으로 시각화하는 JS 프레임워크·보드
- 장점: 에이전트 디버깅용 칸반; React 임베딩 용이; 사고 과정 실시간 관찰
- 단점: PM 도구 아닌 개발자용; worktree·터미널 미관리; 네이티브 앱 없음
- 출처: <https://www.kaibanjs.com>, <https://github.com/kaiban-ai/KaibanJS>, <https://github.com/kaiban-ai/kaiban-board>

### KaibanJS
- 칸반 보드를 내장한 JS/TS 멀티에이전트 프레임워크
- 장점: JS/TS 네이티브; 임베드 가능한 칸반 컴포넌트; 토큰·비용 관측
- 단점: 기존 CLI 에이전트 관리용 아님; JS 코딩 필요; 비개발자에 부적합
- 출처: <https://news.hada.io/topic?id=17823>, <https://www.kaibanjs.com/>, <https://github.com/kaiban-ai/kaibanjs>

### Kanban Pro (Good Guy Apps)
- 로컬 우선 데스크톱 칸반, Markdown 티켓·5가지 뷰·내장 에이전트 터미널
- 장점: 완전 무료·계정 불필요; 에이전트가 Markdown 직접 읽고 씀; 5가지 뷰와 내장 터미널
- 단점: 모바일 앱 없음; 오픈소스 여부 불명; 팀 협업은 클라우드 폴더 동기화 의존
- 출처: <https://goodguyapps.com>

### kanban-mcp (multidimensionalcats)
- AI 코딩 에이전트용 DB 기반 칸반, 40+ MCP 도구·로컬 시맨틱 검색·웹 UI
- 장점: 에이전트 워크플로 전용 설계; API 키 없는 로컬 ONNX 벡터 검색; 에픽·의존성 등 40+ 도구
- 단점: 모바일은 localhost 웹뿐; SQLite 모드에 다중 사용자 권한 없음; 첫 검색 시 약 140MB 모델 다운로드
- 출처: <https://github.com/multidimensionalcats/kanban-mcp>

### kanban-md
- AI 코딩 에이전트 협업용 파일 기반 칸반 엔진 및 Go TUI
- 장점: 멀티 에이전트 리스 잠금(--claim); 토큰 절약 --compact·--json 출력; 마우스 지원 TUI
- 단점: 모바일 앱·웹 GUI 없음; 공식 MCP 서버 없음; 기기마다 Go 바이너리 설치 필요
- 출처: <https://github.com/antopolskiy/kanban-md>, <https://github.com/antopolskiy/kanban-md/releases/tag/v0.39.0>, <https://api.github.com/repos/antopolskiy/kanban-md>

### KanBots
- 11개 에이전트 CLI를 worktree에서 병렬 실행하는 로컬 칸반 앱
- 장점: 11개 CLI 병렬 지원; 사람 개입 결정 프롬프트; 로컬 우선·텔레메트리 없음
- 단점: 클라우드 협업 유료; 모바일·웹 없음; 에이전트 CLI 별도 설치 필요
- 출처: <https://news.hada.io/topic?id=29830>, <https://github.com/leodavinci1/kanbots>, <https://kanbots.dev>

### Kander
- 규칙 기반 멀티에이전트 병렬개발 칸반, 2단계 독립 리뷰 게이트
- 장점: 실전 1,726건 처리 검증; 구현·리뷰 에이전트 분리; 충돌 인식 스케줄링
- 단점: 엄격한 워크플로; 초기 버전(v0.9.x); 문서 주로 중·영
- 출처: <https://github.com/dualface/kander>, <https://github.com/dualface/kander/releases>, <https://quicktui.ai/>

### Kandev
- 에디터·diff·터미널 내장, 다중 에이전트를 조율하는 AI 칸반 개발 환경
- 장점: 칸반+파이프라인+통합 작업공간; 모바일 원격 모니터링; 텔레메트리 없음
- 단점: AGPL-3.0; 로컬 개발 도구 필요; 호스팅 클라우드 없음
- 출처: <https://github.com/kdlbs/kandev>, <https://raw.githubusercontent.com/kdlbs/kandev/main/README.md>

### KanVibe
- AI 코딩 에이전트용 키보드 중심 칸반, CLI 훅으로 상태 자동 추적·카드별 터미널
- 장점: 훅 기반 카드 자동 이동; 내장 웹 터미널; git worktree 자동 생성·정리
- 단점: 모바일 조작 제약; PostgreSQL 셀프호스팅 필요; tmux/zellij 의존
- 출처: <https://news.hada.io/topic?id=26737>, <https://github.com/rookedsysc/kanvibe>, <https://github.com/rookedsysc/kanvibe/releases>

### Kiro (AWS)
- AWS의 AI 코딩 IDE·에이전트 환경, 명세 기반 개발과 에이전트 훅 제공
- 장점: 명세 기반 태스크 진행률 추적; 이벤트 기반 에이전트 훅; ACP·MCP 양방향 지원
- 단점: AWS 클라우드 종속으로 자체 호스팅 불가; 크레딧 소모 빠르고 과금 예측 어려움; 전통적 칸반 보드 아님(확장 필요)
- 출처: <https://kiro.dev>, <https://kiro.dev/pricing/>, <https://kiro.dev/mobile/>, <https://kiro.dev/docs/specs/>, <https://kiro.dev/docs/hooks/>, <https://kiro.dev/docs/hooks/types/>, <https://kiro.dev/docs/acp/>, <https://kiro.dev/license/>, <https://github.com/grg-lab/kiro-specs-dashboard>

### LangGraph Studio & LangSmith
- 상태 기반 에이전트 그래프용 IDE와 프로덕션 관측 플랫폼
- 장점: 그래프 기반 오케스트레이션 표준; 타임트래블 디버거; 상세 트레이스·지표
- 단점: 칸반 없음; LangSmith 셀프호스팅은 고가 엔터프라이즈; 학습 곡선 가파름
- 출처: <https://github.com/langchain-ai/langgraph>, <https://docs.langchain.com/langsmith/studio>, <https://docs.smith.langchain.com/>, <https://www.langchain.com/pricing>

### Maestro
- worktree 병렬 에이전트 데스크톱 앱 + 모바일 PWA 원격 제어
- 장점: 모바일 경험 집중; 모바일 diff 리뷰·라인 피드백; 크로스 플랫폼
- 단점: 커뮤니티 매우 작음; GitHub 로그인 릴레이 의존; 네이티브 앱 아닌 PWA
- 출처: <https://maestro-build.com>, <https://github.com/PlathsOven/maestro>, <https://app.maestro-build.com>, <https://news.ycombinator.com/item?id=49018431>

### Maude
- 사용자 VPS에서 AI 코딩 에이전트를 24시간 실행·감독하는 모바일 우선 원격 제어 앱
- 장점: 노트북 없이 24시간 실행; 푸시 알림으로 원탭 승인/거부; 서버 자동 설정
- 단점: 칸반 보드 없음; 유료 구독 필요; 자체 VPS 필요·공개 MCP/API 없음
- 출처: <https://maude.pilotdev.fr>, <https://apps.apple.com/app/id6759869262>, <https://play.google.com/store/apps/details?id=com.pilotdev.maude>, <https://maude.pilotdev.fr/compare/claude-code-vs-codex-vs-opencode/>, <https://maude.pilotdev.fr/compare/maude-vs-claude-code-remote-control/>

### MetaGPT & MGX
- SOP로 가상 소프트웨어 회사를 모델링하는 프레임워크와 SaaS MGX
- 장점: 역할 기반 협업 선구자; SOP로 실제 산출물 생성; MGX 노코드 빌드
- 단점: 칸반 없음; 출력이 터미널 로그 위주; 토큰 소모 큼
- 출처: <https://github.com/geekan/MetaGPT>, <https://mgx.dev>, <https://atoms.dev>

### MindFlock
- 트래커 할당 티켓으로 worktree·에이전트 세션을 자동 생성하는 워크스페이스
- 장점: Jira·Linear·GitHub 핸즈프리 연동; worktree·프롬프트 자동 세팅; GUI·웹 동시 지원
- 단점: 커뮤니티 작음; 모바일 앱 없음; 단독 칸반 기능 부차적
- 출처: <https://mindflock.ai>, <https://github.com/MindFlock/MindFlock>, <https://news.ycombinator.com/item?id=47926410>

### mngr (by Imbue)
- 로컬·Docker·클라우드에서 다수 코딩 에이전트를 실행·관리하는 Unix식 CLI
- 장점: 가벼운 Unix 철학 CLI; 로컬~클라우드 샌드박스 확장; Git식 스냅샷·포크
- 단점: 칸반·GUI 대시보드 없음; 모바일 미지원; 터미널 숙련 필요
- 출처: <https://imbue.com/product/mngr>, <https://github.com/imbue-ai/mngr>, <https://raw.githubusercontent.com/imbue-ai/mngr/main/README.md>

### monday AI Agents (Platform Agents)
- monday 보드 안에서 작업을 자율 감시·갱신하는 네이티브 AI 에이전트
- 장점: 에이전트가 담당자·댓글 작성자로 보드에 참여; 컬럼·상태·일정 기반 트리거; 보드별 세분화된 권한 관리
- 단점: 좌석 요금 외 AI 크레딧 소모; 관리 API 일부 프리뷰 상태; 온프레미스 실행 불가
- 출처: <https://developer.monday.com/api-reference/docs/manage-agent.md>, <https://developer.monday.com/api-reference/docs/build-on-monday-with-ai.md>, <https://monday.com/pricing>, <https://developer.monday.com/api-reference/docs/mondaycom-mcp.md>, <https://developer.monday.com/api-reference/llms.txt>

### Moo Tasks
- 웹 칸반과 보드별 MCP 서버를 결합한 멀티유저 작업 관리
- 장점: MCP 완전 내장; 보드별 토큰·공개 제어; Correction Task 기능
- 단점: MySQL 필요; 에이전트 실행·worktree 러너 없음; 네이티브 모바일 앱 없음
- 출처: <https://mootasks.dev>, <https://github.com/dizlexic/moo-tasks>, <https://news.ycombinator.com/item?id=47954002>

### Multica
- 사람과 26+ AI 코딩 에이전트가 worktree에서 협업하는 Linear형 칸반
- 장점: 26+ CLI 통합 어댑터; 엄격한 worktree 격리; 데스크톱·모바일 앱 지원
- 단점: 상업 호스팅·브랜딩 제한; 높은 아키텍처 복잡도; 0.4.x 잦은 호환성 변경
- 출처: <https://juejin.cn/post/7686331923204292623>, <https://juejin.cn/post/7662325201879318562>, <https://github.com/multica-ai/multica>, <https://multica.ai>

### multidimensionalcats/kanban-mcp
- DB 기반 에이전트용 칸반+MCP 서버, 웹 UI·40+ 도구
- 장점: 40+ 전용 MCP 도구; 로컬 시맨틱 검색; SQLite로 의존성 없음
- 단점: 개인 OSS 프로젝트; 네이티브 모바일 앱 없음; 멀티테넌트 권한 없음
- 출처: <https://github.com/multidimensionalcats/kanban-mcp>, <https://glama.ai/mcp/servers/multidimensionalcats/kanban-mcp>

### Nerve (OpenClaw Nerve)
- OpenClaw용 실시간 웹 콕핏, 에이전트 칸반·음성·세션 트리
- 장점: 칸반+음성+파일탐색 통합; 모바일 웹 UI 우수; 60초 설치
- 단점: OpenClaw 생태계 종속; 리소스 요구 큼; 음성은 권한·API 필요
- 출처: <https://nerve.zone>, <https://github.com/daggerhashimoto/openclaw-nerve>

### Nimbalyst
- Claude Code·Codex·OpenCode 오케스트레이션용 시각적 워크스페이스·세션 칸반
- 장점: iOS 앱으로 푸시·diff 승인; git worktree 격리; 로컬 우선 텍스트 저장
- 단점: 초기 단계(v0.80.x); Android 앱 미출시; 확장 SDK 생태계 미성숙
- 출처: <https://github.com/nimbalyst/nimbalyst>, <https://nimbalyst.com/>, <https://docs.nimbalyst.com/>

### Nodiom Cloud
- 다중 에이전트용 공유 상태 칸반, 충돌 없는 작업 점유·인계·승인
- 장점: 멀티 에이전트 충돌 방지; 모델별 작성 기록 추적; Slack/Discord 승인 알림
- 단점: 신생으로 생태계 작음; Pro $49 가격 격차 큼; 네이티브 모바일 앱 없음
- 출처: <https://nodiom.md>, <https://nodiom.md/pricing>, <https://api.smithery.ai/servers/synexiom-labs/nodiom-cloud>

### Notion AI & Custom Agents
- Notion 내장 AI 에이전트; 스케줄·웹훅 트리거로 DB 다단계 워크플로 자동화
- 장점: Notion 칸반·DB에서 직접 동작; 트리거·cron으로 무인 자율 실행; 크레딧 한도·제로 데이터 보존 등 거버넌스
- 단점: 2026년 5월부터 유료 크레딧 필요; 로컬 오픈웨이트 모델 실행 불가; Notion 워크스페이스에 종속
- 출처: <https://www.notion.com/product/ai>, <https://www.notion.com/pricing>, <https://developers.notion.com/guides/notion-agent-apis/overview.md>, <https://developers.notion.com/guides/notion-agent-apis/quickstart.md>, <https://developers.notion.com/guides/agent-skills/overview.md>

### Offrun
- Mac에서 다중 에이전트를 worktree 병렬 실행·계정 자동 전환·피어 리뷰
- 장점: 쿼터 소진 시 계정 자동 전환; 에이전트 간 자동 피어 리뷰; 완전 로컬 프라이버시
- 단점: macOS 전용; 비공개 소스; 칸반보다 세션 관리자 구조
- 출처: <https://offrun.dev/>, <https://news.ycombinator.com/item?id=49247385>

### Omnara
- Claude Code·Codex 원격 제어에서 출발한 오픈소스 관리형 에이전트 플랫폼
- 장점: iOS·Android·Watch·데스크톱 네이티브 앱; Postgres 이벤트 로그로 세션 유지·클라우드 이전; 모델·샌드박스 무관, 플랫폼비 0
- 단점: 전통적 칸반 보드 아님; 빠른 구조 변화로 설정 추적 필요; 셀프호스팅 시 Postgres·Redis·스토리지 필요
- 출처: <https://www.omnara.com/>, <https://www.omnara.co/>, <https://www.omnara.com/pricing>, <https://github.com/omnara-ai/omnara>, <https://api.github.com/repos/omnara-ai/omnara>, <https://api.github.com/repos/omnara-ai/omnara/releases>, <https://docs.omnara.com/introduction>, <https://docs.omnara.com/tools/mcp>, <https://docs.omnara.com/api/authentication>, <https://docs.omnara.com/changelog>, <https://www.omnara.com/blog/mobile-coding-landscape>

### opcode (formerly Claudia, getAsterisk, Gooey)
- Claude Code용 오픈소스 GUI 데스크톱 앱·웹서버(에이전트·세션·MCP·비용 관리)
- 장점: Tauri 기반 빠른 네이티브 UI; 분기 체크포인트·타임라인 복원; 웹서버 모드로 폰 원격 제어
- 단점: 네이티브 모바일 앱 없음; 릴리스 바이너리 지연(소스 빌드 필요); Claude Code 생태계에 종속
- 출처: <https://github.com/winfunc/opcode>, <https://github.com/winfunc/opcode/blob/main/README.md>, <https://github.com/winfunc/opcode/blob/main/LICENSE>, <https://github.com/winfunc/opcode/blob/main/web_server.design.md>, <https://github.com/winfunc/opcode/blob/main/cc_agents/README.md>, <https://github.com/winfunc/opcode/blob/main/src-tauri/Cargo.toml>, <https://github.com/winfunc/opcode/blob/main/src-tauri/src/web_main.rs>, <https://github.com/winfunc/opcode/blob/main/src-tauri/src/web_server.rs>, <https://winfunc.com/>, <https://claudelog.com/mcps-and-add-ons/claudia>

### OpenAI Agents SDK Tracing
- 핸드오프·가드레일·트레이싱을 제공하는 OpenAI 공식 멀티에이전트 SDK
- 장점: 공식·경량 설계; 핸드오프·가드레일 지원; 설정 없는 트레이스 트리
- 단점: 칸반 없음; 트레이싱 UI가 OpenAI 플랫폼에 종속; 백로그 관리 없음
- 출처: <https://github.com/openai/openai-agents-python>, <https://openai.github.io/openai-agents-python/>

### OpenAI Codex Cloud Tasks Dashboard
- 클라우드·로컬 Codex 코딩 태스크의 상태·Diff·승인·로그를 모니터링하는 대시보드
- 장점: PC 오프라인에도 클라우드 백그라운드 실행; 네이티브 앱·위젯으로 모바일 승인 원활; 로컬·클라우드 통합 조회
- 단점: 커스텀 칸반 아닌 목록형 콘솔 UI; ChatGPT 유료 구독 종속; 타사 에이전트용 공개 REST 제한적
- 출처: <https://chatgpt.com/codex>, <https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan>, <https://chatgpt.com/pricing/>, <https://learn.chatgpt.com/docs/cloud>, <https://learn.chatgpt.com/docs/mobile>, <https://learn.chatgpt.com/docs/codex-sdk>, <https://learn.chatgpt.com/docs/codex/cli>, <https://learn.chatgpt.com/docs/changelog>, <https://github.com/openai/codex>

### OpenClaw Nerve
- OpenClaw 에이전트용 실시간 웹 관제 대시보드·자동 칸반
- 장점: 실시간 칸반 전환 표시; 음성 상호작용; 서브에이전트·토큰 추적
- 단점: OpenClaw 생태계 종속; 네이티브 모바일 앱 없음; 로컬 런타임 필요
- 출처: <https://github.com/daggerhashimoto/openclaw-nerve>

### OpenHands
- 오픈소스 자율 AI 개발 플랫폼(반응형 웹·Slack 봇 제공)
- 장점: 편집·터미널·브라우징 등 포괄적 자율 기능; Slack으로 폰에서 작업 지시; 대규모 생태계
- 단점: 셀프호스트 리소스 부담; 모바일 웹 UI 복잡; Slack 봇 기능 축소
- 출처: <https://github.com/All-Hands-AI/OpenHands>, <https://openhands.dev>, <https://www.all-hands.dev/pricing>

### OpenHands Agent Canvas (All-Hands-AI)
- 자율 코딩 에이전트용 웹 워크스페이스·대화/상태 대시보드 및 자동화 관리
- 장점: ACP로 다양한 외부 CLI 에이전트 오케스트레이션; 로컬·Docker·VM·클라우드 백엔드 전환; cron·웹훅 자동화 내장
- 단점: 네이티브 모바일 앱 없음(터널링 필요); 다중 저장소·Node 24+ 등 복잡한 구성; 전통적 칸반 보드 아님
- 출처: <https://github.com/OpenHands/OpenHands>, <https://github.com/OpenHands/OpenHands/releases>, <https://raw.githubusercontent.com/OpenHands/OpenHands/main/LICENSE>, <https://www.openhands.dev/pricing>, <https://docs.openhands.dev/llms.txt>, <https://docs.openhands.dev/openhands/usage/agent-canvas/overview.md>, <https://docs.openhands.dev/openhands/usage/agent-canvas/acp-agents>, <https://docs.openhands.dev/openhands/usage/agent-canvas/prebuilt-automations>, <https://docs.openhands.dev/openhands/usage/agent-canvas/mobile-access.md>, <https://docs.openhands.dev/openhands/usage/agent-canvas/backends>, <https://docs.openhands.dev/openhands/usage/agent-canvas/backend-setup/docker>, <https://docs.openhands.dev/openhands/usage/agent-canvas/backend-setup/vm>, <https://docs.openhands.dev/openhands/usage/cloud/cloud-api>, <https://docs.openhands.dev/openhands/usage/agent-canvas/conversations>, <https://docs.openhands.dev/openhands/usage/agent-canvas/managing-automations>

### OpenKanban
- AI 코딩 에이전트 오케스트레이션용 Go TUI 칸반
- 장점: 터미널·SSH에서 동작; 카드별 내장 터미널; 초경량
- 단점: 웹·모바일 불가; 드래그앤드롭 없음; 외부 연동 적음
- 출처: <https://github.com/TechDufus/openkanban>, <https://yetanotherorchestrator.app/apps/openkanban/>

### OpenTerra (KaiStephens/OpenTerra)
- 백그라운드 Claude Code 에이전트 Terragon의 오픈소스 버전
- 장점: Terragon 대안으로 셀프호스팅 가능; 무료
- 단점: 개발 중단 상태; 커뮤니티 매우 작음; 문서 부족
- 출처: <https://github.com/KaiStephens/OpenTerra>

### Orca
- 격리된 git 워크트리에서 병렬 AI 코딩 에이전트를 오케스트레이션하는 오픈소스 ADE
- 장점: 워크트리 격리로 병렬 에이전트 충돌 방지; 에이전트 대시보드 칸반(실험적)과 모바일 앱 푸시 알림; 무료 MIT, 에이전트 종속 없음
- 단점: 로컬 데스크톱/SSH 호스트가 계속 실행돼야 함; 자체 릴레이는 별도 서비스 운영 필요; 잦은 릴리스로 UI/CLI 플래그 변경 잦음
- 출처: <https://www.onorca.dev>, <https://github.com/stablyai/orca>, <https://apps.apple.com/us/app/orca-ide/id6766130217>, <https://www.onorca.dev/docs/model/agents-sessions>, <https://www.onorca.dev/docs/model/worktrees>, <https://www.onorca.dev/docs/cli/overview>, <https://www.onorca.dev/docs/cli/orchestration>, <https://www.onorca.dev/docs/mobile>, <https://www.onorca.dev/docs/agents/supported>, <https://raw.githubusercontent.com/stablyai/orca/main/cloud/README.md>, <https://raw.githubusercontent.com/stablyai/orca/main/LICENSE>

### Ouijit
- 카드별 worktree·터미널, 카드 이동 시 에이전트 훅 실행 데스크톱 앱
- 장점: 카드 상태와 에이전트 훅 연동; 카드별 샌드박스 터미널; 완전 오프라인
- 단점: 데스크톱 전용·Windows 미지원; 모바일 없음; 1인 사용 중심
- 출처: <https://ouijit.com>, <https://github.com/ouijit/ouijit>, <https://raw.githubusercontent.com/ouijit/ouijit/main/README.md>

### OverClick
- 에이전트가 MCP로 카드를 실행·보고하는 프라이버시 우선 셀프호스트 작업 보드
- 장점: MCP 에이전트가 카드 자율 수령; 모델별 실측 토큰 추적; 텔레메트리 전무
- 단점: 초기 단계(v0.4.0); 모바일 앱·푸시 없음; Docker·AUTH_SECRET 설정 필요
- 출처: <https://github.com/ustoppble/overclick>, <https://github.com/ustoppble/overclick/releases>

### Paca
- AI 에이전트와 사람이 스크럼 보드·스프린트를 공유하는 AI 네이티브 PM 플랫폼
- 장점: 에이전트를 스크럼 팀원으로 취급; 완전한 MCP 서버; 단일 컨테이너 셀프호스팅
- 단점: 엔터프라이즈 지표·SSO 미완성; 네이티브 앱 없음; MCP 설정 필요
- 출처: <https://github.com/Paca-AI/paca>, <https://paca-ai.org>, <https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md>

### Pane
- 여러 AI 코딩 에이전트를 git 워크트리로 격리해 병렬 실행하는 데스크톱 앱·CLI
- 장점: 계정 없는 로컬 우선 구조; git 워크트리 자동 격리; 풍부한 CLI/MCP 연동
- 단점: 칸반 보드 없음; 메모리 사용량 큼(~780MB); 텔레메트리 기본 활성화
- 출처: <https://runpane.com>, <https://runpane.com/pricing.md>, <https://runpane.com/agents.md>, <https://runpane.com/docs/runpane-cli>, <https://runpane.com/.well-known/mcp/server-card.json>, <https://runpane.com/docs/security>, <https://runpane.com/compare/vibe-kanban>, <https://runpane.com/compare/superset>, <https://github.com/greenfield-inc/Pane>

### Paperclip
- 7열 칸반·조직도·예산 게이트를 갖춘 멀티 에이전트 오케스트레이션 플랫폼
- 장점: 기능 제한 없는 MIT 오픈소스; 원자적 작업 체크아웃 칸반; 예산 상한·거버넌스 내장
- 단점: Node.js 24+·PostgreSQL 필요; 네이티브 모바일 앱 없음; 에이전트별 API 키 설정 필요
- 출처: <https://paperclip.ing>, <https://github.com/paperclipai/paperclip>, <https://docs.paperclip.ing>

### Parallel Code
- 작업별 git 브랜치·워크트리로 여러 AI 코딩 에이전트를 병렬 실행하는 무료 데스크톱 앱
- 장점: 완전 무료·텔레메트리 없음; 풍부한 내장 MCP 서버; Android 앱·PWA 모바일 페어링
- 단점: 칸반 보드 없음(타일 레이아웃); Windows 미지원; 다수 에이전트 시 로컬 자원 부담
- 출처: <https://yetanotherorchestrator.app/apps/parallel-code/>, <https://parallelcode.app/>, <https://github.com/johannesjo/parallel-code>, <https://github.com/johannesjo/parallel-code/releases>, <https://raw.githubusercontent.com/johannesjo/parallel-code/main/electron/mcp/server.ts>, <https://raw.githubusercontent.com/johannesjo/parallel-code/main/src/components/TilingLayout.tsx>

### PlateSpinner
- 제안→계획→실행 3단계로 헤드리스 에이전트 구동, 웹 칸반에 실시간 스트리밍
- 장점: 읽기/쓰기 단계 안전 격리; AI 판사 Autoclicker 모드; diff 뷰어·토큰 비용 추적
- 단점: 커뮤니티 매우 작음; 모바일 푸시 알림 없음; 로컬 CLI 사전 설치 필요
- 출처: <https://github.com/moridinamael/platespinner>, <https://news.ycombinator.com/item?id=47281358>

### Platypus
- 칸반·대시보드·크론·A2A 통신을 갖춘 셀프호스팅 AI 에이전트 플랫폼
- 장점: 칸반 포함 올인원 스택; MCP·A2A 일급 지원; Ollama로 완전 오프라인 가능
- 단점: Docker Compose 필수; 모바일 앱 없음; SSO 미지원·Git worktree 없음
- 출처: <https://platypus.chat>, <https://github.com/willdady/platypus>, <https://docs.platypus.chat>

### PocketShell
- E2E 암호화 WebRTC로 로컬 Claude Code·Codex 세션을 제어하는 모바일 터미널
- 장점: P2P WebRTC·DTLS 암호화; 실제 PTY·터치 제스처; 푸시 알림
- 단점: 모바일 앱·시그널링 비공개; Rust 호스트 데몬 필요; 작은 커뮤니티
- 출처: <https://github.com/yashagldit/PocketShell>, <https://pocketshell.app>

### Proliferate
- 로컬·클라우드 샌드박스에서 병렬 코딩 에이전트를 관리하는 오픈소스 에이전트 IDE·컨트롤 플레인
- 장점: Docker·AWS 원클릭 셀프호스팅; LiteLLM 게이트웨이로 비용 관리; 예약·이벤트 기반 자동 워크플로
- 단점: AGPL 카피레프트 제약; 칸반 보드 없음; 셀프호스팅 인프라 부담 큼
- 출처: <https://yetanotherorchestrator.app/apps/proliferate/>, <https://proliferate.com/>, <https://github.com/proliferate-ai/proliferate>, <https://github.com/proliferate-ai/proliferate/releases>, <https://raw.githubusercontent.com/proliferate-ai/proliferate/main/guides/deploying/self-hosted-deploy.md>, <https://raw.githubusercontent.com/proliferate-ai/proliferate/main/apps/packages/product-client/src/pages/WorkspacesPage.tsx>

### QodFlow
- 사람과 AI 에이전트가 같은 카드에서 협업하는 AI 네이티브 칸반
- 장점: 사람·에이전트 동등한 카드 작업; 무설정 MCP; 로그인 없는 QR 상태 추적
- 단점: 클라우드 전용; 무료 티어 작업 10개 제한; 네이티브 앱 없음
- 출처: <https://www.qodflow.com>, <https://www.qodflow.com/pricing>

### Ralph TUI
- 백로그 완료까지 에이전트를 반복 실행하는 Ralph 루프 TUI 오케스트레이터
- 장점: 완료 감지 포함 자율 실행 루프; Beads·Linear·Jira 등 트래커 호환; 병렬 worktree·AI 충돌 해결
- 단점: 칸반 카드/열 보드 없음; PRD·Beads 학습 곡선 가파름; Node/Bun 의존 스택
- 출처: <https://ralph-tui.com/>, <https://github.com/subsy/ralph-tui>, <https://github.com/subsy/ralph-tui/releases>, <https://yetanotherorchestrator.app/apps/ralph-tui/>

### Relayd
- 로컬 코딩 에이전트 세션을 폰에 연결해 푸시 알림·후속 지시하는 PWA
- 장점: 모바일 푸시 알림 특화; 독립형 PWA 설치; 가벼운 설정
- 단점: Codex 중심 연동; 칸반 기능 없음; 비공개 백엔드
- 출처: <https://relayd.dev>, <https://relayd.dev/manifest.webmanifest>, <https://news.ycombinator.com/item?id=46678461>

### Replit Agent (Replit Mobile)
- Replit Agent를 폰에서 실행·모니터링·조종하는 공식 모바일 IDE
- 장점: 네이티브 앱, 설정 불필요; 체크포인트 검토·라이브 미리보기; 푸시 알림·롤백
- 단점: Replit 클라우드 종속; 크레딧 빠른 소진; 셀프호스트·오프라인 불가
- 출처: <https://replit.com/mobile>, <https://replit.com/pricing>

### Roadmap Skill
- 칸반+DAG 그래프와 MCP 서버를 갖춘 로컬 계획 도구
- 장점: 칸반+DAG 이중 뷰; npx 한 줄 실행; VS Code MCP 원클릭 설치
- 단점: 단일 사용자 중심; 클라우드 동기화 없음; 기능 최소
- 출처: <https://github.com/shiquda/roadmap-skill>, <https://www.npmjs.com/package/roadmap-skill>

### Roamgate
- Herdr 워크스페이스·터미널·diff 리뷰를 폰으로 가져오는 모바일 웹 클라이언트
- 장점: 파일트리·diff·작업보드 모바일 뷰; PWA 홈화면 설치; 데스크톱→폰 인계
- 단점: Herdr 데몬 필요; 자체 푸시 알림 없음; 가상 키보드 터미널 입력 불편
- 출처: <https://github.com/powerfooI/roamgate>, <https://roamgate.dev/>

### Routa
- 스펙과 칸반 오케스트레이션을 결합한 멀티에이전트 소프트웨어 딜리버리 플랫폼
- 장점: 멀티에이전트용 칸반 오케스트레이션; MCP·ACP·A2A 지원; 스펙-카드 연결
- 단점: API·UI 급변; 튜토리얼 부족; Rust+Node 다중 서비스 구성
- 출처: <https://github.com/phodal/routa>

### Saga MCP
- SQLite 기반 Jira형 작업 추적 MCP 서버. 에픽·태스크·의존성 영속화, 로컬 웹 칸반
- 장점: 41개 세분화된 MCP 도구; 설정·외부 DB 불필요; 자동 블로킹·의존성 해결
- 단점: 단일 프로젝트 중심; 네이티브 앱 없음; LLM 판단에 의존
- 출처: <https://github.com/spranab/saga-mcp>, <https://www.npmjs.com/package/saga-mcp>, <https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md>

### Sculptor (by Imbue)
- 격리된 Git 워크트리·컨테이너에서 병렬 코딩 에이전트를 조율·모니터링하는 데스크톱 앱
- 장점: 에이전트별 워크트리/컨테이너 격리; 라이브 diff·턴 상태로 멀티 에이전트 감독; 모델 종속 없음
- 단점: 모바일 미지원; Windows 미지원; 공유 파일 잠금 없음
- 출처: <https://imbue.com/product/sculptor>, <https://imbue.com/blog/sculptor-announce>, <https://imbue.com/blog/sculptor>, <https://github.com/imbue-ai/sculptor>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/README.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/getting_started.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/workspaces.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/integrated_harnesses.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/experimental/container_backend.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/help/extensions.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/sculptor/sculptor-plugin/skills/sculpt-cli/SKILL.md>, <https://raw.githubusercontent.com/imbue-ai/sculptor/main/docs/development/cli.md>

### Shrimp Task Manager
- 작업 분해·추론·세션 간 메모리를 제공하는 MCP 작업 관리자
- 장점: 깊은 작업 분해·의존성 관리; 세션 간 컨텍스트 유지; 넓은 MCP 호환
- 단점: GUI는 보조 수준; 로컬 JSON·다중 사용자 불가; 네이티브 앱 없음
- 출처: <https://cjo4m06.github.io/mcp-shrimp-task-manager>, <https://github.com/cjo4m06/mcp-shrimp-task-manager>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/README.md>

### Sortie
- Linear·Jira·GitHub 등 이슈 트래커의 티켓을 받아 격리 워크스페이스에서 코딩 에이전트에 배분하는 데몬
- 장점: 내장 SQLite를 쓰는 의존성 없는 단일 Go 바이너리; 기존 트래커와 동기화해 티켓 중복 없음; 티켓별 토큰 예산 상한과 비용 추정
- 단점: 자체 UI에 칸반 보드 없음(트래커에 위임); 네이티브 모바일 앱·PWA 없음; HTTP 서버에 기본 인증 없음
- 출처: <https://docs.sortie-ai.com/>, <https://yetanotherorchestrator.app/apps/sortie/>, <https://docs.sortie-ai.com/reference/dashboard/>, <https://docs.sortie-ai.com/reference/http-api/>, <https://docs.sortie-ai.com/guides/monitor-with-logs/>, <https://github.com/sortie-ai/sortie>, <https://github.com/sortie-ai/sortie/releases>

### Star-Office-UI
- AI 에이전트 작업 상태를 픽셀아트 오피스로 시각화하는 보드
- 장점: 직관적 픽셀아트 시각화; 멀티에이전트 참여 키; 중·영·일 3개 언어
- 단점: 코드리뷰·머지 기능 없음; 아트 에셋 상업 사용 제한; worktree 연동 부족
- 출처: <https://juejin.cn/post/7613971395929112603>, <https://github.com/ringhyacinth/Star-Office-UI>

### Supacode
- libghostty 기반 macOS 네이티브 앱, 격리된 Git 워크트리에서 50개 이상의 CLI 코딩 에이전트 병렬 실행
- 장점: 네이티브 Swift UI로 빠름; zmx 세션 유지; 워크트리 병렬 격리
- 단점: 칸반 뷰 없음; macOS 26+ 전용; 워크트리 5개 이후 유료
- 출처: <https://supacode.sh/>, <https://yetanotherorchestrator.app/apps/supacode/>, <https://github.com/supabitapp/supacode>, <https://github.com/supabitapp/supacode/releases>, <https://raw.githubusercontent.com/supabitapp/supacode/main/LICENSE>, <https://raw.githubusercontent.com/supabitapp/supacode/main/AGENTS.md>, <https://github.com/supabitapp/supacode/commits/main>

### Superset
- 100개 이상 코딩 에이전트를 워크트리별로 병렬 실행하는 에이전트 IDE·오케스트레이터
- 장점: MCP 서버·CLI·SDK 완비; 네이티브 iOS·iPad 앱; 목록·칸반 Tasks 뷰(Linear·GitHub 연동)
- 단점: 모바일은 유료 Pro·iOS 전용; 데스크톱은 macOS 위주(Windows 미지원); ELv2는 OSI 오픈소스 아님
- 출처: <https://superset.sh>, <https://superset.sh/pricing>, <https://github.com/superset-sh/superset>, <https://docs.superset.sh>, <https://docs.superset.sh/llms.txt>, <https://docs.superset.sh/tasks>, <https://docs.superset.sh/mcp-server>, <https://docs.superset.sh/cli/getting-started>, <https://docs.superset.sh/cli/cli-reference>, <https://docs.superset.sh/agent-status>, <https://docs.superset.sh/supported-agents>, <https://docs.superset.sh/browser>, <https://docs.superset.sh/faq>, <https://apps.apple.com/us/app/id6788926383>, <https://raw.githubusercontent.com/superset-sh/superset/main/README.md>, <https://raw.githubusercontent.com/superset-sh/superset/main/LICENSE.md>, <https://raw.githubusercontent.com/superset-sh/superset/main/package.json>, <https://github.com/superset-sh/superset/releases>

### Synara
- 로컬 우선 AI 코딩 에이전트 워크스페이스, 워크트리·디프·PR 지원, Attention 칸반 보드 제공
- 장점: 에이전트용 네이티브 칸반 보드; MCP로 작업 생성·갱신; 크로스플랫폼
- 단점: 코드베이스가 무거움(Electron/Bun); 베타 단계에서 호환성 깨지는 변경 잦음; Android 미출시
- 출처: <https://www.trysynara.com/>, <https://yetanotherorchestrator.app/apps/synara/>, <https://github.com/Emanuele-web04/synara>, <https://github.com/Emanuele-web04/synara/releases>, <https://www.trysynara.com/llms.txt>, <https://www.trysynara.com/docs/features/tasks-and-inbox.md>, <https://www.trysynara.com/docs/workflows/agent-gateway.md>, <https://www.trysynara.com/docs/workflows/external-mcp.md>, <https://raw.githubusercontent.com/Emanuele-web04/synara/main/LICENSE>, <https://github.com/Emanuele-web04/synara/commits/main>

### T3 Code (pingdotgg/t3code)
- 로컬 코딩 에이전트를 데스크톱·웹·모바일에서 제어하는 오픈소스 컨트롤 서피스
- 장점: iOS·Android 모바일 앱 지원; 지원 에이전트 다수; MIT 무료
- 단점: 칸반 뷰 없음(세션 중심); 팀 협업 기능 제한적; 모바일 원격 시 데스크톱 상시 실행 필요
- 출처: <https://raw.githubusercontent.com/pingdotgg/t3code/main/README.md>, <https://t3.codes>

### Taskade AI Agents
- AI 앱 생성·24시간 자율 에이전트 팀·칸반 뷰를 갖춘 협업 워크스페이스
- 장점: 공식 MCP 서버 제공; 에이전트 무제한·영구 메모리; 칸반 등 다양한 뷰와 네이티브 앱
- 단점: 온프레미스 셀프호스팅 불가; 크레딧 소진형 과금; 복잡한 요금 체계
- 출처: <https://www.taskade.com>, <https://www.taskade.com/pricing>, <https://www.taskade.com/create>, <https://www.taskade.com/downloads>

### Taskboard (tcarac)
- 칸반 웹 UI·CLI·내장 터미널·22개 MCP 도구의 로컬 우선 PM 도구
- 장점: 보드에서 에이전트 직접 실행하는 내장 터미널; 의존성 없는 단일 바이너리; CLI·MCP 동등
- 단점: 로컬 1인 개발 중심; 모바일 푸시·네이티브 앱 없음
- 출처: <https://github.com/tcarac/taskboard>

### Tasklix
- AI 에이전트 플릿용 공유 보드·작업 큐, 승인 게이트·크레딧 계량
- 장점: 에이전트 레지스트리·능력 탐색; 중요 작업 인간 승인; 실행 감사 로그
- 단점: 크레딧 과금 예측 어려움; 비공개 SaaS; 네이티브 모바일 앱 없음
- 출처: <https://tasklix.dev>, <https://tasklix.dev/pricing>, <https://api.smithery.ai/servers/hello-n43p/tasklix>

### Taskmaster AI (eyaltoledano/claude-task-master)
- AI 코딩 에이전트용 CLI·MCP 작업관리: PRD를 작업으로 분해, 복잡도·의존성 관리
- 장점: 13개+ IDE·에이전트 MCP 지원; PRD 파서·의존성·복잡도 점수; 파일 잠금으로 다중 에이전트 안전
- 단점: Commons Clause로 상용 재호스팅 금지; 칸반은 VS Code 확장·유료 클라우드 의존; 모바일 앱 없음
- 출처: <https://github.com/eyaltoledano/claude-task-master>, <https://tryhamster.com/pricing>, <https://tryhamster.com/product/taskmaster>, <https://tryhamster.com/product/studio>, <https://raw.githubusercontent.com/eyaltoledano/claude-task-master/main/LICENSE>, <https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster>, <https://api.github.com/repos/eyaltoledano/claude-task-master>

### Taskmaster Kanban VS Code Extension (Hamster.task-master-hamster)
- VS Code·Cursor에서 .taskmaster 작업을 드래그앤드롭 칸반으로 표시, MCP 실시간 동기화
- 장점: IDE 내 드래그앤드롭 칸반; 에이전트 작업과 실시간 동기화; 하위작업·의존성 UI
- 단점: VS Code·Cursor 생태계 한정; 모바일 접근 불가; 로컬 Node.js 필요
- 출처: <https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster>, <https://github.com/eyaltoledano/claude-task-master>

### Terragon (Terragon Labs)
- 격리 샌드박스에서 원격 AI 코딩 에이전트에 비동기 작업 위임·PR 생성하는 클라우드 오케스트레이터
- 장점: 샌드박스 격리·브랜치·PR 자동 Git 워크플로; 다양한 에이전트+BYO 구독 지원; CLI·MCP로 로컬 핸드오프(terry pull --resume)
- 단점: 공식 SaaS 종료·신규 가입 불가; 셀프호스팅에 Postgres·Redis·R2 등 복잡한 인프라 필요; 드래그앤드롭 칸반 아닌 상태 목록 중심
- 출처: <https://www.terragonlabs.com/>, <https://github.com/terragon-labs/terragon-oss>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/README.md>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/mobile.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/resources/billing.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/resources/shutdown.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/integrations/cli.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/configuration/mcp-setup.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/docs/content/docs/tasks/managing-tasks.mdx>, <https://raw.githubusercontent.com/terragon-labs/terragon-oss/main/apps/cli/src/mcp-server/index.ts>, <https://registry.npmjs.org/@terragon-labs/cli>

### Tirith
- 병렬 코딩 에이전트 충돌 방지용 경량 MCP 조정 서버·태스크 보드 데몬
- 장점: TTL 파일 클레임으로 경합 방지; 빠른 Rust 구현; 의존성 그래프 자동 해제
- 단점: UI 최소한; 커뮤니티 작음; 에이전트가 규칙 준수해야 함
- 출처: <https://github.com/eabz/tirith>, <https://eabz.github.io/tirith/>, <https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md>

### Traycer AI
- 스펙 우선(계획→실행→검증→커밋) 단계형 칸반 코딩 허브
- 장점: iOS·Android 네이티브 앱; 스펙 우선 단계 보드; 토큰 마크업 없음
- 단점: 팀 협업·동기화는 유료; 독점 바이너리; 비오픈소스
- 출처: <https://traycer.ai>, <https://traycer.ai/pricing>

### Unpeel
- Rust 기반 에이전트 전용 터미널 멀티플렉서, 워크트리별 영속 세션과 macOS·iOS 클라이언트 제공
- 장점: 영속 헤드리스 PTY 세션; 네이티브 iOS 터미널 앱; 통합 MCP 서버
- 단점: 설계상 칸반 없음; 원격 연결 시 클라우드 릴레이 의존; 다른 세션 쓰기에 수동 승인 필요
- 출처: <https://unpeel.com/>, <https://yetanotherorchestrator.app/apps/unpeel/>, <https://unpeel.com/docs>, <https://unpeel.com/docs/cli>, <https://unpeel.com/docs/unpeel-mcp>, <https://unpeel.com/privacy>, <https://github.com/unpeel-com/unpeel>, <https://raw.githubusercontent.com/unpeel-com/unpeel/main/crates/Cargo.toml>, <https://raw.githubusercontent.com/unpeel-com/unpeel/main/LICENSE>, <https://raw.githubusercontent.com/unpeel-com/unpeel/main/AGENTS.md>, <https://raw.githubusercontent.com/unpeel-com/unpeel/main/clients/ios/UnpeelIOS/project.yml>, <https://github.com/unpeel-com/unpeel/commits/main>

### Uzi
- Git worktree·tmux로 다수 AI 코딩 에이전트를 병렬 실행하는 CLI
- 장점: 한 명령으로 다중 에이전트 생성; 에이전트별 개발 서버 포트 할당; 자동 승인·체크포인트 병합
- 단점: tmux 필수; 표 형태 목록만 제공(칸반·GUI 없음); 개발 활동 둔화
- 출처: <https://github.com/devflowinc/uzi>, <https://raw.githubusercontent.com/devflowinc/uzi/main/README.md>, <https://www.uzi.sh>

### Veritas Kanban
- MCP·스쿼드 챗·감사 로그를 갖춘 로컬 우선 에이전트 작업 보드
- 장점: MCP로 에이전트가 작업 직접 조작; 로컬 우선 프라이버시; 거버넌스 로그
- 단점: 고급 자동화에 MCP/웹훅 설정 필요; 코드 샌드박스 아님; 커뮤니티 작음
- 출처: <https://github.com/BradGroux/veritas-kanban>, <https://bradgroux.github.io/veritas-kanban/demo/>

### Vibe Kanban
- 10종 이상의 코딩 에이전트를 Git 브랜치별로 조율하는 오픈소스 칸반
- 장점: 가장 폭넓은 에이전트 호환성; 브랜치 연동 칸반과 diff 뷰어; 28k+ 스타의 대규모 커뮤니티
- 단점: 호스팅 클라우드 종료; 네이티브 모바일 앱 없음; 네이티브 MCP 서버 부재
- 출처: <https://github.com/BloopAI/vibe-kanban>, <https://vibekanban.com>, <https://github.com/p-wegner/agentic-kanban>

### Vibeyard
- Claude Code·Codex·Gemini CLI 세션을 감싸는 데스크톱 IDE. 스웜 그리드·칸반 제공
- 장점: 칸반과 터미널 세션 직접 연결; 스웜 그리드 모니터링; WebRTC P2P 공유
- 단점: 모바일·웹 UI 없음; 특정 CLI에 종속; 보드 컬럼 커스터마이즈 제한
- 출처: <https://github.com/elirantutia/vibeyard>, <https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md>

### VS Code Agent Kanban / AgentKanban.io
- AI 코딩 에이전트용 칸반·계획 워크플로·Git 워크트리 격리 VS Code 확장
- 장점: 공식 MCP 서버로 작업 직접 조작; Git 워크트리 자동 격리; AGENTS.md 컨텍스트 자동 주입
- 단점: 클라우드 협업은 유료 SaaS; ELv2로 상업 재배포 제한; 네이티브 모바일 앱 없음
- 출처: <https://agentkanban.io>, <https://agentkanban.io/pricing>, <https://github.com/appsoftwareltd/vscode-agent-kanban>, <https://marketplace.visualstudio.com/items?itemName=AppSoftwareLtd.vscode-agent-kanban>, <https://api.github.com/repos/appsoftwareltd/vscode-agent-kanban>

### WorkBuddy Board
- MCP 에이전트용 상시 표시 데스크톱 4열 칸반 위젯. 정체 작업 자동 강조
- 장점: 항상 위 위젯으로 창 전환 불필요; 정체 작업 노란색 강조; 외부 의존성 없음
- 단점: Windows 우선(macOS/Linux는 소스 실행); 모바일 없음; 1인 유지보수
- 출처: <https://github.com/b52tot/workbuddy-board>, <https://board-widget-download.app.workbuddy.host/>, <https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md>

### Yao Agents
- 대화 지시를 칸반 작업으로 바꿔 여러 기기에서 실행하는 플랫폼
- 장점: Android 네이티브 앱; 다중 기기 분산 실행; 대화 기반 칸반
- 단점: 수정 라이선스 재배포 조건; 설정 복잡; iOS 앱 없음
- 출처: <https://yaoagents.com>, <https://github.com/YaoApp/yao>, <https://raw.githubusercontent.com/YaoApp/yao/main/README.md>

## MCP 서버 상세

### @suekou/mcp-notion-server (Community MCP Server)
- AI 친화 커뮤니티 Notion MCP 서버; 스키마 인식 고수준 도구로 DB·칸반 조회/편집
- 장점: 값 기반 고수준 도구로 복잡한 JSON 불필요; select/status 스키마 검증으로 잘못된 쓰기 방지; 컴팩트 응답·인터랙티브 MCP Apps
- 단점: 커뮤니티 유지, 공식 지원 없음; 통합 토큰 수동 설정 필요; 기본 stdio 전용(원격 HTTP 없음)
- 출처: <https://github.com/suekou/mcp-notion-server>, <https://raw.githubusercontent.com/suekou/mcp-notion-server/main/README.md>, <https://raw.githubusercontent.com/suekou/mcp-notion-server/main/docs/tools.md>, <https://api.github.com/repos/suekou/mcp-notion-server/releases>

### ClickUp Community MCP Servers (hauptsacheNet & taazkareem)
- API 키로 코딩 에이전트를 ClickUp에 연결하는 커뮤니티 MCP 서버
- 장점: 정적 API 키 인증 지원; 로컬 이미지 업로드; read-minimal 모드로 토큰 절약
- 단점: ClickUp API 속도 제한 적용; taazkareem판 유료 라이선스; 공식 스키마 변경 반영 지연 가능
- 출처: <https://github.com/hauptsacheNet/clickup-mcp>, <https://github.com/taazkareem/clickup-mcp-server>, <https://api.github.com/repos/hauptsacheNet/clickup-mcp>, <https://api.github.com/repos/taazkareem/clickup-mcp-server>

### Container-use
- Dagger 기반으로 에이전트별 컨테이너 샌드박스·Git 브랜치를 제공하는 MCP 서버/CLI
- 장점: 강력한 컨테이너 격리; MCP 네이티브 구조; Git 브랜치로 결과 확인·폐기 용이
- 단점: 칸반 등 시각적 대시보드 없음; 컨테이너 런타임 자원 부담; 환경 시작 지연
- 출처: <https://github.com/dagger/container-use>, <https://raw.githubusercontent.com/dagger/container-use/main/README.md>, <https://raw.githubusercontent.com/dagger/container-use/main/rules/agent.md>, <https://container-use.com>

### GitHub Kanban MCP Server (Sunwood AI Labs)
- LLM 에이전트가 GitHub 이슈·Projects를 칸반으로 관리하는 MCP 서버
- 장점: GitHub에 바로 연결; 표준 MCP 즉시 사용; 별도 인프라 불필요
- 단점: 자체 시각 대시보드 없음; GitHub 이슈 기능에 한정; 1인 개발
- 출처: <https://github.com/Sunwood-ai-labs/github-kanban-mcp-server>, <https://www.pulsemcp.com/servers/sunwood-ai-labs-github-kanban>

### MCP Server Trello (delorenj)
- Trello 보드·카드·체크리스트를 57개 도구로 관리하는 TS/Bun MCP 서버
- 장점: 가장 성숙한 57개 도구; Trello 요청 한도 처리 내장; 네이티브 모바일 앱 활용
- 단점: 독점 SaaS(Trello) 의존; API 키·토큰 수동 발급 필요; 무료 플랜 10명 제한
- 출처: <https://github.com/delorenj/mcp-server-trello>, <https://trello.com/pricing>, <https://registry.modelcontextprotocol.io>

### mcp-server-asana (Community MCP Server by roychri)
- AI 에이전트가 Asana 작업·프로젝트를 MCP로 조회·생성·수정하는 오픈소스 서버
- 장점: MCP 표준 준수로 손쉬운 연동; 섹션 이동으로 칸반 상태 변경; 읽기 전용 모드 지원
- 단점: 커뮤니티 유지(공식 지원 없음); PAT 수동 관리 필요; 고급 엔터프라이즈 기능 미지원
- 출처: <https://github.com/roychri/mcp-server-asana>, <https://www.npmjs.com/package/@roychri/mcp-server-asana>

### Notion MCP (Official Remote Server)
- Notion 공식 호스팅 MCP 서버; 검색·마크다운 페이지 편집·SQL 쿼리·에이전트 세션 관리
- 장점: 로컬 인프라 불필요; 토큰 효율적 마크다운 편집; 칸반 DB 대상 SQL 쿼리 지원
- 단점: 셀프호스팅·에어갭 불가; Bearer 토큰·다중 DB SQL은 Business 이상; 클라우드 지연·속도 제한
- 출처: <https://developers.notion.com/guides/mcp/overview.md>, <https://developers.notion.com/guides/mcp/get-started-with-mcp.md>, <https://developers.notion.com/guides/mcp/mcp-supported-tools.md>, <https://developers.notion.com/guides/mcp/hosting-open-source-mcp.md>

### notion-mcp-server (Official Open-Source Local Server)
- Notion 공식 오픈소스 로컬 MCP 서버(npm·Docker), 통합 토큰으로 DB·페이지 조작
- 장점: 로컬·Docker 셀프호스팅 가능; OAuth 없이 토큰으로 헤드리스 실행; 22개 도구·마크다운 읽기/수정
- 단점: 공식 지원 중단; 시맨틱 AI 검색 없음; JSON 응답이 컨텍스트 토큰 많이 소모
- 출처: <https://github.com/makenotion/notion-mcp-server>, <https://github.com/makenotion/notion-mcp-server/blob/main/README.md>, <https://developers.notion.com/guides/mcp/hosting-open-source-mcp.md>

### Planka MCP (gogogadgetbytes)
- Planka 2.0+ 자체 호스팅 칸반을 REST API로 AI 에이전트에 연결하는 MCP 서버
- 장점: Planka의 다중 사용자 칸반 UI 활용; Zod 기반 타입 안전; npx로 빌드 없이 실행
- 단점: 실행 중인 Planka 인스턴스 필요; 실시간 이벤트 푸시 없음; 에이전트 전용 계정 필요
- 출처: <https://github.com/gogogadgetbytes/planka-mcp>, <https://planka.app>, <https://planka.app/pricing>, <https://github.com/plankanban/planka>

### Shrimp Task Manager MCP (cjo4m06/mcp-shrimp-task-manager)
- AI 에이전트 작업 계획·분해·실행·검증 및 세션 간 메모리 유지 MCP 서버
- 장점: MCP 클라이언트와 높은 호환성; 의존성 그래프 GUI+Task Viewer 제공; Git 자동 백업·스냅샷 복원
- 단점: 네이티브 모바일 앱 없음; RBAC·팀 클라우드 동기화 부재; 2025-08 이후 커밋 둔화
- 출처: <https://github.com/cjo4m06/mcp-shrimp-task-manager>, <https://cjo4m06.github.io/mcp-shrimp-task-manager>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/package.json>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/tools/task-viewer/package.json>, <https://github.com/cjo4m06/mcp-shrimp-task-manager/blob/main/tools/task-viewer/README.md>, <https://github.com/cjo4m06/mcp-shrimp-task-manager/blob/main/docs/tools.md>, <https://github.com/cjo4m06/mcp-shrimp-task-manager/blob/main/docs/api.md>, <https://github.com/cjo4m06/mcp-shrimp-task-manager/blob/main/docs/agents.md>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/src/index.ts>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/src/web/webServer.ts>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/src/public/style.css>, <https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/tools/task-viewer/src/index.css>, <https://glama.ai/mcp/servers/@cjo4m06/mcp-shrimp-task-manager>, <https://smithery.ai/server/@cjo4m06/mcp-shrimp-task-manager>

### Trello MCP Server (m0xai)
- Trello 보드·리스트·카드를 관리하는 경량 Python MCP 서버
- 장점: Node/Bun 없이 Python으로 구현; uv로 간단 설정; 최소한의 도구 구성
- 단점: delorenj 대비 도구 수 크게 적음; 인수조건 파싱·요청 한도 백오프 없음
- 출처: <https://github.com/m0xai/trello-mcp-server>, <https://trello.com/pricing>

### WeKan MCP Server (namar0x0309)
- 자체 호스팅 WeKan 보드·스윔레인·카드에 에이전트 접근을 주는 MCP 서버
- 장점: 텔레메트리 없는 완전 OSS 칸반 연결; 스윔레인 지원; API 토큰 자동 추출 스크립트
- 단점: 커뮤니티 작음; 기본 CRUD만 지원; 카드 선점·의존성 기능 없음
- 출처: <https://github.com/namar0x0309/wekan-mcp>, <https://wekan.fi>, <https://wekan.fi/commercial-support/>, <https://github.com/wekan/wekan>

### zenhub-cli (`zh`) & Bundled MCP Server
- AI 코딩 에이전트용 ZenHub 보드·스프린트·하위이슈 관리 CLI 겸 MCP 서버
- 장점: 에이전트용 68개 MCP 도구; 임베딩 기반 중복 티켓 방지; 닫힌 상위 이슈 고아화 방지 안전장치
- 단점: ZenHub 전용 카드 미지원; gh CLI와 GraphQL·REST 토큰 필요; 임베딩 모델 약 500MB·Python 3.10+ 필요
- 출처: <https://github.com/daniel-pittman/zenhub-cli>, <https://raw.githubusercontent.com/daniel-pittman/zenhub-cli/develop/README.md>, <https://raw.githubusercontent.com/daniel-pittman/zenhub-cli/develop/CLAUDE.md>, <https://api.github.com/repos/daniel-pittman/zenhub-cli/releases>

### zenhub-mcp (TypeScript)
- ZenHub GraphQL API를 MCP로 제공하는 TypeScript 서버
- 장점: GraphQL 작업 54개 기본 지원; npx·Claude Desktop 간편 설정; 임의 GraphQL 쿼리 도구 제공
- 단점: 오픈소스 라이선스 미선언; 중복 감지·안전장치 없음; 유지보수 정체
- 출처: <https://github.com/derekbar90/zenhub-mcp>, <https://raw.githubusercontent.com/derekbar90/zenhub-mcp/main/README.md>, <https://api.github.com/repos/derekbar90/zenhub-mcp>

## 범용 칸반 + 에이전트 연동 도구 상세

### Airtable (Kanban view + AI + MCP)
- 클라우드 관계형 DB·앱 빌더, 칸반 등 다중 뷰와 내장 AI 제공
- 장점: 공식 MCP 서버·스킬 플러그인 제공; 유연한 스키마와 상태 컬럼; 칸반 지원 네이티브 모바일 앱
- 단점: 셀프호스팅·오프라인 불가; API 속도 제한(베이스당 초당 5회); 좌석당 요금 부담
- 출처: <https://www.airtable.com>, <https://www.airtable.com/pricing>, <https://github.com/Airtable/airtable-mcp-cli>, <https://github.com/Airtable/skills>, <https://github.com/domdomegg/airtable-mcp-server>, <https://apps.apple.com/us/app/airtable/id914172636>, <https://play.google.com/store/apps/details?id=com.formagrid.airtable&hl=en>

### Asana
- 칸반·리스트·타임라인·간트 뷰를 갖춘 엔터프라이즈 협업 업무관리 플랫폼
- 장점: 성숙한 엔터프라이즈 칸반; 내장 AI Studio·AI Teammates; 충실한 REST API·웹훅
- 단점: 셀프호스팅 불가; 사용자당 요금 부담; 공식 MCP 서버 없음
- 출처: <https://asana.com>, <https://asana.com/pricing>, <https://asana.com/product/ai/ai-studio>, <https://asana.com/product/ai/ai-teammates>, <https://developers.asana.com/docs/overview>, <https://apps.apple.com/us/app/asana-work-in-one-place/id489969512>, <https://play.google.com/store/apps/details?id=com.asana.app>, <https://help.asana.com/s/article/ai-studio-smart-workflows?language=en_US>

### Backlog (Nulab)
- Nulab의 일본 대표 PM·이슈 칸반, Backlog AI 탑재
- 장점: 성숙한 UX; 네이티브 모바일 앱; Git·간트·위키 통합
- 단점: 상용 유료 SaaS; 자율 코딩 에이전트 아님; 엔터프라이즈 라이선스 비용 큼
- 출처: <https://backlog.com/ja/lp-ai-assistant/>, <https://backlog.com/ja/blog/backlog-ai-issue-creation/>, <https://qiita.com/tie_rrific/items/8df560b1cb27945bd4c9>, <https://backlog.com/ja/mobile/>

### Basecamp card table
- Basecamp 프로젝트 내장 칸반(Card Table), 에이전트를 일급 주체로 지원
- 장점: 에이전트 전용 주체·MCP·CLI 제공; 사용자 무제한 정액 요금; 무료 플랜에도 에이전트 기능 포함
- 단점: 셀프호스팅 불가; 정형화된 칸반 모델; 무료→$25/월 가격 격차
- 출처: <https://basecamp.com/pricing>, <https://basecamp.com/ai>, <https://basecamp.com/via>, <https://github.com/basecamp/bc3-api>, <https://github.com/basecamp/bc3-api/blob/master/sections/card_tables.md>, <https://github.com/basecamp/bc3-api/blob/master/sections/card_table_cards.md>, <https://github.com/basecamp/basecamp-cli>, <https://github.com/basecamp/skills>

### BoardMark
- Markdown 기반 애자일 이슈·칸반, 공식 MCP로 스프린트 관리
- 장점: Markdown 네이티브; 에이전트 수정 출처 표시; Pro 2석 무료
- 단점: 클라우드 전용; 네이티브 모바일 앱 없음; 태국 바트 과금
- 출처: <https://www.boardmark.ai>, <https://www.boardmark.ai/pricing>, <https://api.smithery.ai/servers/i-c-develop/boardmark>

### Businessmap (formerly Kanbanize)
- 전략과 일상 실행을 연결하는 엔터프라이즈 린 포트폴리오·다단계 칸반 플랫폼
- 장점: 공식 1st-party MCP 지원; 다단계 스윔레인·포트폴리오 계층; 강력한 린/애자일 분석(CFD, 몬테카를로)
- 단점: 최소 15석($190/월) 진입장벽; 영구 무료 플랜 없음; 셀프호스팅 불가
- 출처: <https://businessmap.io>, <https://businessmap.io/plans-pricing>, <https://businessmap.io/about/pricing-parameters>, <https://businessmap.io/api>, <https://apps.apple.com/us/app/kanbanize/id563967812>, <https://play.google.com/store/apps/details?id=com.kanbanize.android>

### ClickUp
- 칸반·리스트·문서를 갖춘 올인원 PM 플랫폼, 공식 MCP로 에이전트 연동
- 장점: 풍부한 보드·상태·자동화; 공식 원격 MCP로 설치 불필요; 네이티브 모바일 앱 완비
- 단점: 셀프호스팅 불가; 공식 MCP 24시간 호출 한도; 공식 MCP는 OAuth만 허용(API 키 불가)
- 출처: <https://clickup.com/pricing>, <https://developer.clickup.com/docs/connect-an-ai-assistant-to-clickups-mcp-server>, <https://developer.clickup.com/docs/mcp-tools>, <https://clickup.com/security>, <https://apps.apple.com/us/app/clickup-tasks-chat-docs-ai/id1535098836>, <https://play.google.com/store/apps/details?id=co.mangotechnologies.clickup>

### craaft
- 실시간 협업 칸반, 74개 도구 공식 MCP 서버 제공
- 장점: 74개 MCP 도구; 실시간 보드 동기화; Trello·Jira·Asana 가져오기
- 단점: 비공개 소스·셀프호스팅 불가; 네이티브 모바일 앱 없음; 에이전트 안전장치 없음
- 출처: <https://craaft.io>, <https://craaft.io/mcp>, <https://craaft.io/pricing>, <https://api.smithery.ai/servers/craaft/craaft>

### devBoard
- 개발자용 트래커·칸반, 네이티브 앱과 29개 도구 원격 MCP
- 장점: iOS·Android·macOS 네이티브 앱; 시간 기록·릴리스 도구; 현재 무료
- 단점: 비공개 SaaS; 셀프호스팅 불가; 향후 가격 미공개
- 출처: <https://getdevboard.app>, <https://getdevboard.app/mcp>, <https://api.smithery.ai/servers/darko-bacic/devboard>

### Devin + Jira
- Jira 칸반/스크럼 카드로 Devin 세션 트리거, 상태·PR 회신
- 장점: 엔터프라이즈 컴플라이언스·워크플로; 카드 내 에이전트 패널; 다양한 트리거
- 단점: 설정·권한 복잡; Rovo 기능은 Cloud 전용; UI 무거움
- 출처: <https://docs.devin.ai/integrations/jira.md>, <https://marketplace.atlassian.com/4162823004>, <https://docs.devin.ai/product-guides/automations.md>, <https://www.atlassian.com/software/jira/pricing>

### Devin + Linear
- Linear 이슈·칸반에서 Devin 세션 배정, 진행·PR을 카드에 표시
- 장점: 뛰어난 UI·키보드 중심 칸반; 카드 내 실시간 에이전트 진행·PR; 보드에서 중지·프롬프트 가능
- 단점: 셀프호스팅 불가; Devin 비용 외 좌석 비용; 고급 자동화는 유료
- 출처: <https://docs.devin.ai/integrations/linear.md>, <https://docs.devin.ai/product-guides/automations.md>, <https://linear.app/pricing>

### Devin + Slack
- Slack 채널·스레드로 Devin 세션 배정·상태 모니터링
- 장점: 팀 대화 공간에서 즉시 가시성; Code Channels 상태 배지; 인라인 승인 버튼
- 단점: 정식 칸반 보드 아님; 스레드 난잡; 알림 피로
- 출처: <https://docs.devin.ai/integrations/slack.md>, <https://docs.devin.ai/product-guides/automations.md>, <https://docs.devin.ai/tutorial-library/slack-bug-fix.md>, <https://slack.com/pricing>

### Dooray! Project
- NHN 기업용 협업 스위트, 프로젝트 칸반+Dooray AI 요약·초안
- 장점: 한국어 성숙 UX; 네이티브 모바일 앱; 메일·메신저·캘린더·위키 통합
- 단점: 상용 소프트웨어; 코딩 에이전트보다 요약·초안 중심; 고급 자동화엔 관리자 토큰 필요
- 출처: <https://dooray.com>, <https://inside.nhn.com/news/935>, <https://dooray.com/main/mobile>

### EasyKanban
- 집중형 온라인 칸반, 원격 HTTP MCP·OAuth 2.1로 AI가 카드 관리
- 장점: 설치 불필요 원격 MCP; 토큰 없는 OAuth 로그인; 넉넉한 무료 읽기
- 단점: 클라우드 전용; 쓰기 기능은 Pro 유료; 작업 잠금 등 스웜 기능 없음
- 출처: <https://easykanb.app>, <https://easykanb.app/mcp>, <https://easykanb.app/#pricing>, <https://api.smithery.ai/servers/e-goetmann/easykanban>

### Forgejo
- Codeberg e.V. 주도 비영리 커뮤니티의 Gitea 하드포크 포지, 이슈 칸반 보드 제공
- 장점: 비영리 커뮤니티 거버넌스, 오픈코어 없음; 유료 제한 없는 Gitea 동급 보드; REST API로 에이전트 카드 관리 가능
- 단점: 에이전트 계측 기능 없음; 공식 모바일 앱 없음; Windows 서버 공식 지원 중단(2024)
- 출처: <https://forgejo.org/>, <https://forgejo.org/faq/>, <https://forgejo.org/compare-to-gitea/>, <https://codeberg.org/forgejo/forgejo>, <https://codeberg.org/api/v1/repos/forgejo/forgejo/releases>, <https://codeberg.org/gitnex/GitNex>, <https://codeberg.org/>

### Framedeck MCP
- 영상 제작자용 칸반 콘텐츠 파이프라인 Framedeck의 공식 MCP 서버
- 장점: 미디어 제작 워크플로 특화; 공식 MCP로 주요 AI 클라이언트 지원; 체크리스트·단계 전환 관리
- 단점: 코어가 클로즈드 SaaS; 소프트웨어 개발용 아님; 커넥터 규모 작음
- 출처: <https://github.com/Lukaris/framedeck-mcp>, <https://framedeck.app>, <https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md>

### Gitea
- Go로 작성된 경량 자체호스팅 Git·DevOps 서비스, 사용자/저장소/조직 단위 칸반 보드 내장
- 장점: 매우 가볍고 저사양에서도 구동; 보드 전체 수명주기 REST API 지원; 사용자 제한 없이 칸반 무료
- 단점: 에이전트 네이티브 기능(로그·트레이스) 없음; 공식 모바일 앱 없음; 보드 실시간 갱신 미지원
- 출처: <https://about.gitea.com/>, <https://about.gitea.com/pricing/>, <https://docs.gitea.com/>, <https://gitea.com/api/v1/swagger>, <https://gitea.com/swagger.v1.json>, <https://github.com/go-gitea/gitea>, <https://github.com/go-gitea/gitea/releases/tag/v28.1.0>, <https://gitea.com/gitea/tea>, <https://codeberg.org/gitnex/GitNex>

### GitHub Projects (Projects v2) Boards
- 이슈·PR과 연동된 칸반·테이블·로드맵 뷰 기반 계획·이슈 추적 도구
- 장점: 드래그앤드롭 칸반·스프린트 관리; MCP로 에이전트가 컬럼 조작; 넉넉한 무료 티어
- 단점: 에이전트 전용 시각화 아님; 실시간 텔레메트리 없음; GraphQL 복잡·API 속도 제한
- 출처: <https://github.com/features/issues>, <https://docs.github.com/en/issues/planning-and-tracking-with-projects/learning-about-projects/about-projects>, <https://docs.github.com/en/rest/projects/projects>, <https://github.com/github/github-mcp-server>, <https://raw.githubusercontent.com/github/github-mcp-server/main/pkg/github/projects.go>, <https://github.com/github/github-mcp-server/releases>, <https://apps.apple.com/us/app/github/id1477376905>

### GitLab Issue Boards (CE Self-Host & SaaS)
- 저장소·MR·CI/CD와 통합된 GitLab 이슈 칸반/스크럼 보드
- 장점: 코드·MR·CI/CD와 긴밀한 통합; REST·GraphQL·CLI·MCP로 에이전트 자동화 용이; MIT CE로 셀프호스팅 시 데이터 주권 확보
- 단점: WIP 제한·스윔레인 등 고급 기능은 유료; 셀프호스팅 시 RAM 4~8GB 필요·운영 부담 큼; 공식 모바일 앱 없음
- 출처: <https://docs.gitlab.com/ee/user/project/issue_board.html>, <https://about.gitlab.com/pricing/>, <https://docs.gitlab.com/ee/api/boards.html>, <https://docs.gitlab.com/ee/user/project/issues/managing_issues.html>, <https://raw.githubusercontent.com/modelcontextprotocol/servers-archived/main/src/gitlab/README.md>, <https://raw.githubusercontent.com/gitlabhq/gitlabhq/master/README.md>, <https://raw.githubusercontent.com/gitlabhq/gitlabhq/master/LICENSE>, <https://raw.githubusercontent.com/profclems/glab/trunk/README.md>

### Groma.md (MrLesk/Groma.md)
- Git 속 Markdown·C4 맵으로 아키텍처 관리, Backlog.md 작업을 맵에 투영
- 장점: Backlog.md와 연계; Git 기반 에이전트 친화적; 무료
- 단점: 칸반 아닌 아키텍처 맵; 모바일 없음; 신생 프로젝트
- 출처: <https://github.com/MrLesk/Groma.md>

### Hamster (tryhamster.com, Task Master 클라우드)
- 목표를 전략·이니셔티브·브리프로 쪼개는 팀용 AI 기획 플랫폼
- 장점: Task Master와 연동; 목표→이니셔티브 구조화; 무료 시작 가능
- 단점: 유료 가격 불명; 셀프호스팅 불가; 순수 칸반 도구는 아님
- 출처: <https://tryhamster.com>

### Huly
- Jira·Linear·Slack·Notion 대체 오픈소스 올인원 프로젝트 관리 플랫폼
- 장점: Svelte 기반의 빠른 UI와 부드러운 칸반 정렬; 칸반·문서·채팅·화상·캘린더 통합; MCP·CLI 등 AI 에이전트 도구 풍부
- 단점: 공식 저장소·클라우드가 자금 문제로 종료; 15개+ 컨테이너의 복잡한 셀프호스팅; 네이티브 모바일 앱 없음
- 출처: <https://github.com/hcengineering/platform>, <https://github.com/Platform-Collective/platform>, <https://huly.io>, <https://huly.io/pricing>, <https://huly.io/download>, <https://docs.huly.io/getting-started/self-host>, <https://github.com/hcengineering/huly-selfhost>, <https://github.com/dearlordylord/huly-mcp>, <https://github.com/hcengineering/huly-examples>

### It's a Plan (ItsAPlan)
- AI 에이전트를 담당자로 지정하는 셀프호스팅 Linear/Plane 대안
- 장점: 에이전트가 실제 담당자인 이슈 트래커; 공식 MCP 서버; 사용자당 비용 없음
- 단점: AGPL 카피레프트; 개발 중으로 변경 가능성; Docker/Postgres 필요
- 출처: <https://github.com/croffasia/itsaplan>, <https://itsaplan.dev>

### Jira Boards
- WIP 제한·자동화 지원 칸반/스크럼 보드, 보드에 AI 에이전트 협업 내장
- 장점: 업계 표준 칸반·커스텀 워크플로; 컬럼 이동 시 에이전트 트리거; Agent Sessions 추적 뷰
- 단점: 복잡한 설정·높은 학습 곡선; AI 기능은 유료·기업 이메일 필요; 크레딧 초과 과금
- 출처: <https://www.atlassian.com/software/jira>, <https://www.atlassian.com/software/jira/pricing>, <https://www.atlassian.com/agile/kanban/boards>, <https://www.atlassian.com/software/jira/mobile-app>, <https://apps.apple.com/us/app/jira-cloud-by-atlassian/id1006972087>, <https://play.google.com/store/apps/details?id=com.atlassian.android.jira.core>, <https://support.atlassian.com/atlassian-ai-gateway/docs/supported-tools/>

### Kaiten
- 여러 보드를 한 화면에 보여주는 러시아산 칸반·프로젝트 관리 플랫폼(AI 어시스턴트 내장)
- 장점: 다중 보드 동시 보기; 엔터프라이즈용 Docker 온프레미스; 넉넉한 무료 플랜(5명)
- 단점: 러시아/CIS 중심(문서·서버); 셀프호스팅은 300명+ 계약만; MCP는 아직 출시 예정
- 출처: <https://kaiten.ru>, <https://kaiten.ru/tariffs>, <https://apps.apple.com/us/app/kaiten/id6739029367>, <https://play.google.com/store/apps/details?id=ru.kaiten.app>

### Kan (kan.bn)
- Next.js 기반 오픈소스 미니멀 Trello 대안, 네이티브 MCP·REST API 지원
- 장점: 공식 MCP 서버 46개 도구; 간단한 Docker 셀프호스팅; OpenAPI REST API·웹훅 제공
- 단점: 네이티브 모바일 앱 없음; 호스팅 MCP는 Teams 유료 필요; API 분당 100회 제한
- 출처: <https://kan.bn>, <https://kan.bn/pricing>, <https://github.com/kanbn/kan>, <https://github.com/kanbn>, <https://docs.kan.bn>, <https://docs.kan.bn/introduction>, <https://docs.kan.bn/integrations/mcp-server>, <https://docs.kan.bn/guides/self-hosting/introduction>, <https://docs.kan.bn/guides/self-hosting/s3>, <https://docs.kan.bn/api-reference/introduction>, <https://docs.kan.bn/api-reference/webhooks/create-a-webhook>, <https://github.com/kanbn/kan/releases>, <https://raw.githubusercontent.com/kanbn/kan/main/AGENTS.md>, <https://raw.githubusercontent.com/kanbn/kan/main/LICENSE>

### Kanbn CLI
- Markdown+YAML 파일로 저장하는 Git 친화적 터미널 칸반 CLI
- 장점: 전용 MCP 서버로 에이전트 연동 우수; Git으로 버전 관리; 스프린트·번다운 등 풍부한 기능
- 단점: 웹·모바일 UI 없음; Git 병합 충돌 가능; 커뮤니티 작음
- 출처: <https://github.com/basementuniverse/kanbn>, <https://api.github.com/repos/basementuniverse/kanbn>, <https://api.github.com/repos/basementuniverse/kanbn/releases>, <https://github.com/kjduling/kanbn-mcp>, <https://github.com/basementuniverse/vscode-kanbn>, <https://raw.githubusercontent.com/kjduling/kanbn-mcp/main/README.md>

### Kanera
- 멀티 보드·포트폴리오 PM, 공식 MCP로 카드·자동화 관리
- 장점: 포트폴리오 뷰; 폭넓은 AI 클라이언트 지원; 저렴한 가격
- 단점: 클라우드 전용; 네이티브 모바일 앱 없음; 고급 기능은 팀 계정 필요
- 출처: <https://www.kanera.app>, <https://api.smithery.ai/servers/kanera/kaneramcp>

### Leantime
- 신경다양성 팀 친화적 목표 중심 오픈소스 PM·칸반(마일스톤·간트·AI)
- 장점: 공식 MCP 서버 지원; 셀프호스트 가볍고 사용자 수 제한 없음; JSON-RPC API·개인 토큰
- 단점: 셀프호스트 MCP 플러그인 유료($29~); 공개 모바일 앱 미출시; 기본 웹훅 제한적
- 출처: <https://leantime.io>, <https://leantime.io/pricing>, <https://github.com/Leantime/leantime>, <https://github.com/Leantime/leantime/releases>, <https://github.com/Leantime/leantime-mcp>, <https://docs.leantime.io>, <https://docs.leantime.io/installation/leantime-mcp.md>, <https://docs.leantime.io/api/usage.md>, <https://docs.leantime.io/development/commandline.md>, <https://docs.leantime.io/installation/frequently-asked-questions.md>, <https://marketplace.leantime.io/product/mcp-server/>, <https://github.com/Leantime/leantime/issues/3398>

### Linear (Linear for Agents)
- 소프트웨어 개발용 이슈 트래커·칸반, 에이전트를 1급 참여자로 다루는 에이전트 플랫폼
- 장점: 에이전트를 비과금 위임 대상으로 지원; 공식 원격 MCP 서버; 세션 상태·계획 실시간 스트리밍
- 단점: 클라우드 전용(셀프호스팅 불가); Loops·Coding Sessions는 유료·AI 크레딧 소모; 보드가 사이클/프로젝트 구조에 종속
- 출처: <https://linear.app>, <https://linear.app/pricing>, <https://linear.app/mobile>, <https://linear.app/docs/linear-agent>, <https://linear.app/docs/agent-platform>, <https://linear.app/docs/mcp>, <https://linear.app/docs/coding-sessions>, <https://linear.app/docs/loops>, <https://linear.app/developers/agents>, <https://linear.app/developers/agent-interaction>, <https://linear.app/developers/agent-best-practices>, <https://linear.app/integrations/cursor-mcp>, <https://linear.app/integrations/claude>, <https://linear.app/integrations/codex>, <https://linear.app/integrations/devin>, <https://linear.app/integrations/github-copilot>, <https://github.com/linear/linear>, <https://github.com/linear/claude-managed-agents-demo>, <https://linear.app/terms>, <https://linear.app/docs/security>

### LokalBoards (florian-strasser)
- 실시간 협업·SSO·MCP 내장 Nuxt 기반 자체 호스팅 칸반, 카드 원자적 선점
- 장점: 원자적 카드 선점으로 인간·에이전트 충돌 방지; SSO 무료 포함; 실시간 접속 표시
- 단점: MySQL 필요; 단일 바이너리 대비 운영 부담 큼; 모바일은 웹 전용
- 출처: <https://github.com/florian-strasser/LokalBoards>

### Mattermost Boards
- Mattermost 협업 플랫폼에 통합된 채널 연동 칸반·작업 관리 플러그인
- 장점: Mattermost 채널·Agents와 긴밀 통합; RBAC·ABAC 엔터프라이즈 권한; 공식 MCP 서버
- 단점: 전체 Mattermost 서버 필요; 고급 기능은 유료 Enterprise; 모바일 칸반 UI 제한적
- 출처: <https://mattermost.com/pricing/>, <https://mattermost.com/blog/mattermost-mcp-server/>, <https://github.com/mattermost/mattermost-plugin-boards>, <https://github.com/mattermost/mattermost-plugin-agents>, <https://raw.githubusercontent.com/mattermost/mattermost-plugin-boards/master/README.md>, <https://raw.githubusercontent.com/mattermost/mattermost-plugin-boards/master/plugin.json>

### monday.com boards
- 보드·칸반·자동화·AI를 갖춘 엔터프라이즈 클라우드 업무 OS
- 장점: 설치 불필요한 호스티드 MCP; 60개 이상 MCP 도구와 mcli CLI; 성숙한 iOS·Android 앱
- 단점: 셀프호스팅·오프라인 불가; 유료 플랜 최소 3석; 요금제별 API 호출 제한
- 출처: <https://monday.com/pricing>, <https://monday.com/blog/product/monday-com-pricing-which-plan-is-right-for-you/>, <https://developer.monday.com/api-reference/llms.txt>, <https://developer.monday.com/api-reference/docs/mondaycom-mcp.md>, <https://developer.monday.com/api-reference/docs/compatible-mcp-clients.md>, <https://developer.monday.com/api-reference/docs/integrate-with-monday-mcp.md>, <https://developer.monday.com/api-reference/docs/mcp-api-token.md>, <https://developer.monday.com/api-reference/docs/build-on-monday-with-ai.md>, <https://developer.monday.com/api-reference/docs/manage-agent.md>, <https://developer.monday.com/api-reference/docs/mcli-cli-guide.md>, <https://developer.monday.com/api-reference/docs/rate-limits.md>, <https://github.com/mondaycom/mcp>, <https://github.com/mondaycom/mcli>, <https://registry.npmjs.org/@mondaydotcomorg/monday-api-mcp>

### Motion AI
- AI가 작업 우선순위를 정해 캘린더·칸반에 자동 배치하는 생산성 플랫폼
- 장점: 뛰어난 자동 타임블로킹; 칸반·문서·캘린더 통합; REST API로 외부 에이전트 연동
- 단점: 공식 MCP 서버 없음; 영구 무료 플랜 없고 비쌈; 셀프호스팅 불가
- 출처: <https://www.usemotion.com>, <https://www.usemotion.com/pricing>, <https://docs.usemotion.com>, <https://github.com/rhymiz/python-motion>

### Nextcloud Deck
- Nextcloud 통합 오픈소스 온프레미스 칸반 작업·프로젝트 관리 앱
- 장점: 완전한 데이터 주권·자체 호스팅; 성숙한 REST API와 MCP·CLI 도구; Nextcloud 생태계 깊은 통합
- 단점: 대규모 인스턴스 DB 성능 병목; 코어에 실시간 WebSocket 없음; Nextcloud 서버 운영 필요
- 출처: <https://apps.nextcloud.com/apps/deck>, <https://github.com/nextcloud/deck>, <https://github.com/nextcloud-releases/deck/releases>, <https://deck.readthedocs.io/en/latest/API/>, <https://nextcloud.com/pricing/>, <https://github.com/cbcoutinho/nextcloud-mcp-server>, <https://github.com/cbcoutinho/nextcloud-mcp-server/blob/master/docs/deck.md>, <https://github.com/fastbs/mcp-nextcloud-deck>, <https://github.com/raspbeguy/ncdeck>, <https://github.com/stefan-niedermann/nextcloud-deck>, <https://github.com/holger-dev/nextdeck>, <https://github.com/hweihwang/nextcloud-deck-desktop-releases>

### Notion Boards & Databases
- 칸반·테이블·캘린더·타임라인 뷰를 갖춘 유연한 워크스페이스·DB, 작업/에이전트 실행 추적
- 장점: 칸반 뷰 커스터마이즈 자유도 높음; 카드에 마크다운 실행 로그 기록 가능; 공식 MCP·웹훅·API 생태계 성숙
- 단점: 셀프호스팅 불가; API 속도 제한(평균 3회/초)이 잦은 상태 갱신에 병목; 블록 JSON 스키마 복잡
- 출처: <https://www.notion.com/pricing>, <https://www.notion.com/mobile>, <https://developers.notion.com/reference/intro>, <https://developers.notion.com/guides/get-started/overview.md>, <https://github.com/makenotion/notion-sdk-js>

### OpenProject
- 칸반·간트·스크럼·시간/비용 추적을 갖춘 오픈소스 프로젝트 관리(Jira 대안)
- 장점: 공식 내장 MCP 서버로 작업 검색·생성·수정 지원; 완전 셀프호스팅 가능, GDPR·데이터 주권 중시; 칸반·간트·스크럼 등 기능 풍부
- 단점: MCP는 Professional 이상 유료 플랜 전용; 유료 플랜 최소 25명 이상 조건; Rails/PostgreSQL 기반으로 무겁고 설치·운영 복잡
- 출처: <https://www.openproject.org>, <https://www.openproject.org/pricing/>, <https://www.openproject.org/docs/api/>, <https://www.openproject.org/docs/system-admin-guide/integrations/mcp-server/>, <https://www.openproject.org/docs/mobile-app-guide/>, <https://www.openproject.org/docs/installation-and-operations/installation/>, <https://www.openproject.org/docs/faq/>, <https://github.com/opf/openproject>, <https://github.com/opf/openproject/releases>

### Phabricator / Phorge Workboards
- Phabricator 후속 포크, 코드리뷰·버그추적·저장소·칸반 워크보드 통합 포지
- 장점: 코드리뷰와 작업 보드의 긴밀한 통합; 구조화되고 트랜잭션 안전한 Conduit API; 기능 유료화가 없는 Apache 2.0 라이선스
- 단점: 학습 곡선이 가파르고 용어가 생소함; 레거시 PHP 구조로 데몬 유지보수 필요; 네이티브 앱·푸시 알림 없음
- 출처: <https://phorge.it>, <https://github.com/phorgeit/phorge>, <https://github.com/phacility/phabricator>, <https://github.com/phorgeit/arcanist>, <https://github.com/cortex-app/conduit>, <https://projects.clusterlabs.org/book/phorge/article/conduit_edit/>, <https://projects.clusterlabs.org/book/phorge/article/projects/>

### Plane
- 오픈소스 엔터프라이즈 PM·이슈 트래커로 칸반·스프린트와 AI 연동 제공
- 장점: 네이티브 iOS·Android 앱; 풍부한 API와 MCP 커넥터; 성숙한 엔터프라이즈 기능
- 단점: 에이전트 샌드박스·워크트리 미지원; 인프라 부담이 큼; AGPL 컴플라이언스 검토 필요
- 출처: <https://plane.so>, <https://github.com/makeplane/plane>, <https://plane.so/pricing>

### Redmine (with Agile/Kanban plugins)
- Ruby on Rails 기반 오픈소스 이슈 트래커, Agile 플러그인으로 칸반·스크럼 보드 제공
- 장점: 성숙하고 안정적인 플러그인 생태계; 포괄적인 공식 REST API; 커뮤니티 MCP 서버가 다양함
- 단점: 구식 UI; 기본 코어에 드래그앤드롭 칸반 없음(플러그인 필요); 공식 네이티브 모바일 앱 없음
- 출처: <https://www.redmine.org>, <https://www.redmine.org/plugins/redmine_agile>, <https://www.redmineup.com/pages/plugins/agile>, <https://github.com/redmine/redmine>, <https://github.com/jztan/redmine-mcp-server>, <https://redmine-mcp-server.jztan.com/>

### scrumboy
- Go·SQLite 경량 셀프호스팅 애자일 칸반, 내장 MCP 서버
- 장점: 계정 없이 즉시 보드 생성; 단일 바이너리 셀프호스팅; PWA 지원
- 단점: AGPL 기업 사용 제약; 레트로 디자인; 에이전트 충돌 방지 없음
- 출처: <https://scrumboy.com>, <https://github.com/markrai/scrumboy>

### Shortcut
- 엔지니어링 팀용 이슈 트래킹·칸반/스프린트 SaaS, AI 에이전트 협업 지원
- 장점: 호스팅 MCP로 OAuth만으로 즉시 연동; 방대한 MCP 도구 세트·git 브랜치 연계; 10명까지 무료 티어
- 단점: 셀프호스트 불가로 폐쇄망 부적합; 네이티브 모바일 앱 없음; API 속도 제한 200회/분
- 출처: <https://www.shortcut.com/>, <https://www.shortcut.com/pricing>, <https://www.shortcut.com/agents/>, <https://www.shortcut.com/blog/shortcut-for-agents/>, <https://www.shortcut.com/blog/why-we-built-the-shortcut-mcp-server/>, <https://www.shortcut.com/help/integrations/mcp-server/>, <https://mcp.shortcut.com/mcp>, <https://developer.shortcut.com>, <https://github.com/useshortcut/mcp-server-shortcut>, <https://raw.githubusercontent.com/useshortcut/mcp-server-shortcut/main/package.json>, <https://raw.githubusercontent.com/useshortcut/mcp-server-shortcut/main/LICENSE>, <https://raw.githubusercontent.com/useshortcut/mcp-server-shortcut/main/src/server.ts>, <https://raw.githubusercontent.com/useshortcut/mcp-server-shortcut/main/src/server-http.ts>, <https://raw.githubusercontent.com/useshortcut/mcp-server-shortcut/main/src/tools/stories.ts>, <https://registry.npmjs.org/@shortcut/mcp/latest>, <https://www.shortcut.com/blog/announcing-clubhouse-for-ios-c91bab1b74f7/>

### Super Productivity
- 타임박싱·시간 추적·개인 칸반을 갖춘 할 일 앱. Jira·GitHub·GitLab 연동
- 장점: iOS·Android 네이티브 앱; 타임박싱·포모도로·시간 추적; 외부 트래커 연동 우수
- 단점: 개인 생산성 중심; 칸반은 부가 모드; 에이전트 연동은 REST/플러그인 필요
- 출처: <https://github.com/super-productivity/super-productivity>, <https://super-productivity.com>, <https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md>

### Taiga
- 칸반·스크럼 백로그·이슈 추적을 제공하는 오픈소스 애자일 프로젝트 관리 플랫폼
- 장점: OSS로 셀프호스팅 기능 제한 없음; WIP 제한·스윔레인 등 풍부한 칸반; 클라우드 전 플랜 사용자 무제한
- 단점: 공식 모바일 앱 없음; 공식 MCP 서버 없음; 셀프호스팅 구성 요소 복잡
- 출처: <https://taiga.io/>, <https://taiga.io/deployment-pricing-options/>, <https://github.com/taigaio/taiga-back>, <https://github.com/taigaio/taiga-docker>, <https://docs.taiga.io/>, <https://docs.taiga.io/api.html>, <https://community.taiga.io/t/proposal-mobile-app/257>, <https://community.taiga.io/t/feature-request-official-mcp-server-for-taiga-ai-agent-integration/8931>, <https://community.taiga.io/t/mobile-android-app-for-taiga/8707>, <https://github.com/talhaorak/pytaiga-mcp>, <https://github.com/adriapedralbes/mcpTAIGA>, <https://www.npmjs.com/package/taiga-mcp-server>

### Teamhood
- 스윔레인·WIP 제한·간트·용량 계획을 갖춘 시각적 칸반 업무 관리 도구
- 장점: 읽기/쓰기·읽기전용 MCP 및 일괄 처리; 저렴한 단일 요금제; 엔터프라이즈 온프레미스 지원
- 단점: 최소 3명 구매 필요; 영구 무료 플랜 없음; 네이티브 모바일 앱 없음
- 출처: <https://teamhood.com/>, <https://teamhood.com/pricing/>, <https://teamhood.com/deployment-and-hosting/>, <https://teamhood.com/capabilities/>, <https://teamhood.com/open-api/>, <https://teamhood.com/knowledge-base/integrations/open-api/>, <https://teamhood.com/release-notes/>, <https://help.teamhood.com/integrations/mcp/>, <https://help.teamhood.com/how-it-works/mcp/>, <https://help.teamhood.com/integrations/connect-mcp/>, <https://help.teamhood.com/skills/teamhood/SKILL.md>

### TickTick
- GTD 작업·칸반·캘린더·습관·뽀모도로 통합 개인 생산성 앱
- 장점: 칸반·캘린더·습관·뽀모도로 올인원; 저렴한 프리미엄 요금; 커뮤니티 MCP로 칸반 컬럼 CRUD 가능
- 단점: 공식 MCP·CLI 없음; 공식 API에 칸반 컬럼 관리 없음; 2FA가 V2 세션 MCP와 충돌
- 출처: <https://ticktick.com>, <https://blog.ticktick.com/2020/07/03/kanban-manage-projects-on-the-go>, <https://help.ticktick.com/articles/7055782353376903168>, <https://ticktick.com/about/download>, <https://github.com/dev-mirzabicer/ticktick-sdk>, <https://github.com/liadgez/ticktick-mcp-server>, <https://developer.ticktick.com>, <https://pcmag.com/reviews/ticktick>

### Todoist
- 리스트·캘린더·보드(칸반) 뷰를 갖춘 개인/팀 작업 관리 서비스
- 장점: 공식 호스팅 MCP와 OAuth 무설정 연결; 에이전트 스킬 설치 지원 공식 CLI; 무료·유료 모두 모바일 보드 뷰
- 단점: 셀프호스팅 불가; 무료는 프로젝트 5개 제한; 커스텀 스윔레인·WIP 제한 없음
- 출처: <https://developer.todoist.com>, <https://github.com/Doist/todoist-mcp>, <https://github.com/Doist/todoist-cli>, <https://www.todoist.com/pricing>, <https://www.todoist.com/help/account-and-billing/plans/todoist-plans-pricing-and-billing-faq-Vq2z0HWL6>, <https://todoist.com/help/articles/view-a-project-as-a-board-8g6Yj4>, <https://lifehacker.com/organize-your-life-with-todoists-new-kanban-boards-1845170016>

### Trello (Atlassian)
- 보드·리스트·카드 기반 클라우드 칸반 도구, Butler 자동화 지원
- 장점: 직관적인 칸반 모델; 공식 원격 MCP 엔드포인트 제공; 넉넉한 무료 플랜과 오프라인 동기화 모바일 앱
- 단점: 셀프호스팅 불가; 공식 MCP는 연결당 워크스페이스 1개만 지원; API 속도 제한(토큰당 10초 100회)
- 출처: <https://trello.com/pricing>, <https://trello.com/platforms>, <https://trello.com/mcp>, <https://developer.atlassian.com/cloud/trello/rest/>, <https://developer.atlassian.com/cloud/trello/guides/rest-api/api-introduction/>, <https://developer.atlassian.com/cloud/trello/guides/rest-api/rate-limits/>, <https://github.com/atlassian/trello-mcp-server>, <https://github.com/delorenj/mcp-server-trello>, <https://apps.apple.com/us/app/trello-manage-anything/id461504587>, <https://play.google.com/store/apps/details?id=com.trello>

### Vikunja
- Go·Vue 기반 오픈소스 작업관리 도구(칸반·간트·테이블·리스트), 내장 MCP 서버
- 장점: 내장 MCP 서버와 Claude Code·Codex 연동 가이드; 단일 바이너리/Docker 셀프호스팅, SQLite 내장; 칸반 WIP 제한·간트 등 다중 뷰
- 단점: 공식 네이티브 모바일 앱 없음; ChatGPT 개발자 모드는 OAuth 미지원으로 불가; 감사로그·관리패널은 유료 Pro
- 출처: <https://vikunja.io/>, <https://vikunja.io/pricing/>, <https://vikunja.io/pro/>, <https://vikunja.io/features/>, <https://vikunja.io/help/>, <https://vikunja.io/help/mcp/>, <https://vikunja.io/help/views/>, <https://vikunja.io/help/desktop/>, <https://vikunja.io/help/quick-add-magic/>, <https://vikunja.io/help/projects/>, <https://vikunja.io/docs/>, <https://vikunja.io/docs/installing/>, <https://vikunja.io/docs/api-documentation/>, <https://vikunja.io/docs/api-v2/>, <https://vikunja.io/docs/cli/>, <https://vikunja.io/docs/integrations/>, <https://vikunja.io/docs/webhooks/>, <https://vikunja.io/docs/config-options/>, <https://vikunja.io/docs/desktop-packages/>, <https://vikunja.io/changelog/>, <https://vikunja.io/changelog/vikunja-2.7.0-was-released/>, <https://github.com/go-vikunja/vikunja>, <https://raw.githubusercontent.com/go-vikunja/vikunja/main/README.md>, <https://raw.githubusercontent.com/go-vikunja/vikunja/main/CHANGELOG.md>, <https://raw.githubusercontent.com/go-vikunja/vikunja/main/frontend/package.json>, <https://raw.githubusercontent.com/go-vikunja/vikunja/main/frontend/vite.config.ts>, <https://marco308.github.io/mdone/>

### WeKan
- Trello/Jira 대안인 오픈소스 셀프호스팅 칸반·프로젝트 관리 플랫폼
- 장점: MIT 완전 오픈소스, 유료 기능 없음; 다양한 셀프호스팅 패키징; Trello/Jira/GitHub API 호환 모드
- 단점: 모바일은 웹뷰/PWA만(오프라인 동기화 없음); Meteor 기반으로 메모리 사용량 큼; 공식 MCP 패키지 없음
- 출처: <https://wekan.fi>, <https://github.com/wekan/wekan>, <https://wekan.fi/app>, <https://wekan.fi/saas/>, <https://wekan.fi/commercial-support/>, <https://github.com/wekan/wekan/blob/main/docs/API/REST-API.md>, <https://github.com/wekan/wekan/blob/main/docs/API/Cards.md>, <https://raw.githubusercontent.com/wekan/wekan/main/CHANGELOG.md>, <https://raw.githubusercontent.com/wekan/wekan/main/package.json>

### Wrike
- 칸반·간트·테이블 뷰를 갖춘 엔터프라이즈 협업 업무 관리 플랫폼
- 장점: 공식 MCP 서버 제공; 칸반·간트·워크로드 등 다양한 뷰; GitHub·Jira 양방향 동기화
- 단점: 사용자당 비용 높음; 셀프호스팅 불가; 최소 좌석 단위 구매 부담
- 출처: <https://www.wrike.com/>, <https://www.wrike.com/price/>, <https://developers.wrike.com/>, <https://developers.wrike.com/docs/wrike-mcp-server-overview.md>, <https://developers.wrike.com/docs/setup-claude-with-wrike-mcp.md>

### YouTrack (JetBrains)
- JetBrains 이슈 트래커·프로젝트 관리, 애자일/칸반 보드와 내장 MCP 제공
- 장점: 내장 원격 MCP로 로컬 게이트웨이 불필요; 10명까지 영구 무료(Cloud·Server); Docker 셀프호스팅·iOS/Android 앱
- 단점: 소규모 에이전트용으론 복잡하고 무거움; 독점 코드로 포크 불가; 번들 AI 크레딧 주간 한도
- 출처: <https://www.jetbrains.com/youtrack/>, <https://www.jetbrains.com/youtrack/buy>, <https://blog.jetbrains.com/youtrack/2025/06/new-youtrack-prices-starting-from-october-2025/>, <https://www.jetbrains.com/help/youtrack/cloud/model-context-protocol-server.html>, <https://www.jetbrains.com/youtrack/features/ai_assistant.html>, <https://www.jetbrains.com/help/youtrack/server/youtrack-mobile.html>, <https://github.com/JetBrains/youtrack-mobile>, <https://github.com/tonyzorin/youtrack-mcp>, <https://hub.docker.com/r/jetbrains/youtrack>, <https://www.jetbrains.com/legal/docs/youtrack/license/>, <https://www.jetbrains.com/help/youtrack/devportal/youtrack-rest-api.html>

### ZenHub
- GitHub 네이티브 애자일 칸반·스프린트·로드맵·자동 리포팅 플랫폼
- 장점: GitHub 이슈·PR 양방향 동기화; 5단계 계층과 AI 스프린트 플래닝; 커뮤니티 MCP·CLI 지원
- 단점: 네이티브 모바일 앱 없음; 셀프호스팅은 고가 엔터프라이즈 전용; GitHub 인증·토큰 의존
- 출처: <https://www.zenhub.com>, <https://www.zenhub.com/pricing>, <https://developers.zenhub.com>, <https://developers.zenhub.com/graphql-api-docs/getting-started>, <https://github.com/ZenHubIO/API>, <https://www.zenhub.com/terms-of-service>, <https://github.com/daniel-pittman/zenhub-cli>, <https://github.com/derekbar90/zenhub-mcp>

## 범용 칸반 상세

### 4ga Boards
- 접이식 할 일 목록과 다크 모드를 갖춘 경량 실시간 칸반 보드
- 장점: 깔끔한 UI·다크 모드; WebSocket 실시간 동기화; 가벼운 Docker 배포
- 단점: 에이전트 전용 기능 없음; 공식 MCP 서버 없음; 네이티브 앱 없음
- 출처: <https://github.com/RARgames/4gaBoards>, <https://4gaboards.com>, <https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md>

### AppFlowy (Board View)
- Flutter·Rust 기반 오픈소스 Notion/Trello 대안, 데이터베이스 보드(칸반) 뷰 제공
- 장점: 로컬 우선·오프라인 지원; 전 플랫폼 네이티브 앱; OpenAPI 기반 REST API
- 단점: 셀프호스트 다중 사용자는 상용 라이선스 필요; 즉시 쓸 MCP 플러그인 없음; 셀프호스트에 컨테이너 7개 이상 필요
- 출처: <https://appflowy.com>, <https://appflowy.com/pricing>, <https://appflowy.com/blog/appflowy-kanban-in-project-management>, <https://docs.appflowy.io/docs/documentation/software-contributions/architecture/frontend/database-view/kanban-board>, <https://github.com/AppFlowy-IO/AppFlowy>, <https://github.com/AppFlowy-IO/AppFlowy/releases>, <https://github.com/AppFlowy-IO/AppFlowy-SelfHost-Commercial>, <https://raw.githubusercontent.com/AppFlowy-IO/AppFlowy-SelfHost-Commercial/main/docs/OPENAPI.md>, <https://raw.githubusercontent.com/AppFlowy-IO/AppFlowy-SelfHost-Commercial/main/docs/AUDIT.md>

### Focalboard
- Mattermost가 만든 오픈소스 칸반·테이블·갤러리·캘린더 프로젝트 관리 도구
- 장점: 완전 무료·셀프호스팅; 칸반·테이블·캘린더 등 다중 뷰; OpenAPI 기반 REST API
- 단점: 공식 유지보수 중단; 네이티브 모바일 앱 없음; MCP 서버 없음
- 출처: <https://github.com/mattermost/focalboard>, <https://raw.githubusercontent.com/mattermost/focalboard/main/README.md>, <https://raw.githubusercontent.com/mattermost/focalboard/main/CHANGELOG.md>, <https://raw.githubusercontent.com/mattermost/focalboard/main/LICENSE.txt>, <https://github.com/mattermost-community/focalboard/issues/5038>

### Kanban Tool
- 실시간 칸반 보드·스윔레인·시간 추적·Power-Up을 제공하는 워크플로 관리 플랫폼
- 장점: VirtualBox/VMware/KVM 셀프호스팅 어플라이언스; 모바일 앱이 셀프호스팅 인스턴스 연결 지원; 2인 무료 플랜
- 단점: UI가 다소 구식; 온사이트는 연간·최소 10명 필요; 기본 MCP 서버 없음(REST API 직접 사용)
- 출처: <https://kanbantool.com>, <https://kanbantool.com/pricing>, <https://kanbantool.com/kanban-tool-on-site>, <https://kanbantool.com/developer/>

### KanbanFlow
- WIP 제한·뽀모도로·시간 추적을 갖춘 경량 린 칸반 보드
- 장점: 깔끔하고 빠른 UI; 내장 뽀모도로/시간 추적; 저렴한 가격($5)과 견고한 REST API
- 단점: 네이티브 모바일 앱 없음; API·웹훅은 유료 플랜 전용; 셀프호스팅 불가
- 출처: <https://kanbanflow.com>, <https://kanbanflow.com/pricing>, <https://kanbanflow.com/mobile-app>, <https://kanbanflow.com/api-docs>

### Kanboard
- WIP 제한 중심의 경량·미니멀 오픈소스 칸반 프로젝트 관리 도구
- 장점: 매우 가볍고 빠름; 배치 지원 JSON-RPC API로 에이전트 일괄 업데이트 용이; SQLite로 즉시 로컬 구동
- 단점: 신규 기능 개발 중단(유지보수 모드); UI가 구식; 실시간 업데이트 없음(폴링/새로고침)
- 출처: <https://kanboard.org>, <https://github.com/kanboard/kanboard>, <https://docs.kanboard.org/v1/api/>, <https://docs.kanboard.org/v1/admin/faq/>, <https://docs.kanboard.org/v1/dev/webhooks/>, <https://docs.kanboard.org/v1/admin/docker/>, <https://github.com/kanboard/kanboard/releases>, <https://github.com/ErnestoCorona/kanboard-mcp>, <https://github.com/bivex/kanboard-mcp>

### Kaneo
- 단순함과 속도에 집중한 미니멀 오픈소스 프로젝트 관리·칸반 플랫폼
- 장점: 빠르고 미니멀한 UI; REST·웹훅으로 자동화 용이; 큰 커뮤니티
- 단점: 에이전트 토큰·트레이스 개념 없음; 공식 MCP 없음; 네이티브 앱 없음
- 출처: <https://github.com/usekaneo/kaneo>, <https://kaneo.app/>, <https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md>

### Kanri
- Tauri·Rust 기반의 오프라인 우선 개인용 데스크톱 칸반 앱
- 장점: 가볍고 깔끔한 UI; 완전 오프라인·텔레메트리 없음; 에이전트가 파싱하기 쉬운 JSON
- 단점: 모바일·웹 접근 불가; 에이전트 동시 쓰기 시 충돌 위험; 다중 사용자 미지원
- 출처: <https://github.com/kanriapp/kanri>, <https://kanriapp.com/about>, <https://kanriapp.com/download>, <https://api.github.com/repos/kanriapp/kanri>, <https://api.github.com/repos/kanriapp/kanri/releases>

### MeisterTask
- 애자일 워크플로용 협업 칸반 기반 작업·프로젝트 관리 도구
- 장점: 깔끔하고 직관적인 칸반 UI; 넉넉한 무료 플랜; iOS·Android 네이티브 앱
- 단점: 공식 MCP 서버 없음; 전체 API·자동화는 Pro 이상; 셀프호스팅 불가
- 출처: <https://www.meistertask.com/>, <https://www.meistertask.com/pricing>, <https://www.meistertask.com/apps>, <https://developers.meistertask.com/reference/getting-started-1>, <https://developers.meistertask.com/llms.txt>, <https://developers.meistertask.com/reference/authentication.md>

### Nullboard
- 단일 HTML 파일로 동작하는 초미니멀 키보드 중심 칸반 보드
- 장점: 초경량·즉시 로딩; 한 화면에 많은 작업 표시; 실행 취소 50단계
- 단점: 상업적 이용 제한; 모바일 터치 불편; 백업 데몬 없으면 데이터 유실 위험
- 출처: <https://github.com/apankrat/nullboard>, <https://api.github.com/repos/apankrat/nullboard>, <https://nullboard.io>, <https://raw.githubusercontent.com/apankrat/nullboard/master/nullboard.html>, <https://github.com/justinpchang/nullboard-agent-express>

### Obsidian Kanban Plugin
- Obsidian 마크다운 목록을 드래그앤드롭 칸반 보드로 변환하는 커뮤니티 플러그인
- 장점: 순수 마크다운 단일 파일 형식; 완전 오프라인·로컬 우선; 데스크톱·모바일 동시 지원
- 단점: 에이전트 전용 인터페이스·잠금 없음; 에이전트와 UI 동시 쓰기 시 충돌; 2026년 메인테이너 사임
- 출처: <https://github.com/community-archive/obsidian-kanban>, <https://publish.obsidian.md/kanban/>, <https://obsidian.md/pricing>, <https://obsidian.md/mobile>, <https://github.com/community-archive/obsidian-kanban/releases/tag/2.0.51>, <https://api.github.com/repos/community-archive/obsidian-kanban>

### Planka
- React/Node.js 기반 실시간 협업 칸반 보드(커스텀 필드·시간 추적)
- 장점: 실시간 동기화되는 경량 Trello 대안; OpenAPI REST API·MCP 서버·CLI 생태계; 자체 호스팅 내부 사용 무료
- 단점: 비OSI 라이선스로 상업 호스팅 제한; SSO·태블릿 최적화 등 Pro 유료화; 공식 네이티브 모바일 앱 없음
- 출처: <https://github.com/plankanban/planka>, <https://planka.app/pricing>, <https://planka.app/pro>, <https://docs.planka.cloud/docs/welcome/>, <https://docs.planka.cloud/docs/community-or-pro>, <https://docs.planka.cloud/docs/community/projects>, <https://docs.planka.cloud/docs/api-reference/python>, <https://plankanban.github.io/planka/swagger-ui/swagger.json>, <https://raw.githubusercontent.com/plankanban/planka/master/LICENSES/PLANKA%20Community%20License%20EN.md>, <https://raw.githubusercontent.com/plankanban/planka/master/LICENSES/PLANKA%20Commercial%20License%20EN.md>, <https://raw.githubusercontent.com/plankanban/planka/master/LICENSES/PLANKA%20License%20Guide%20EN.md>, <https://github.com/plankanban/planka/releases>, <https://github.com/gogogadgetbytes/planka-mcp>, <https://github.com/HexiDev/planka-mcp-kanban>, <https://github.com/bradrisse/kanban-mcp>, <https://github.com/plattnum/planka-cli>, <https://github.com/LouisHDev/planka_app>

### Restyaboard
- PHP·PostgreSQL 기반의 셀프호스팅 오픈소스 Trello 클론
- 장점: Trello 수준 기능; 포괄적 REST API; 플러그인 확장
- 단점: 구식 기술 스택; 2022년 이후 릴리스 없음; MCP 미지원
- 출처: <https://github.com/RestyaPlatform/board>, <https://api.github.com/repos/RestyaPlatform/board>, <https://api.github.com/repos/RestyaPlatform/board/releases>, <https://marketplace.digitalocean.com/apps/restyaboard>, <https://forum.cloudron.io/topic/1202/restyaboard-trello-like-kanban-board>, <https://raw.githubusercontent.com/RestyaPlatform/board/master/server/php/R/r.php>

### Tasks.md
- 마크다운 파일 디렉터리를 레인·카드로 보여주는 경량 셀프호스팅 칸반 보드
- 장점: 레인=폴더, 카드=.md로 DB 불필요; 간단한 무인증 REST API; 가벼운 Docker·PWA 지원
- 단점: 외부 변경 시 자동 새로고침 없음; 인증·권한 관리 없음; 공식 MCP·웹훅 없음
- 출처: <https://github.com/BaldissaraMatheus/Tasks.md>, <https://raw.githubusercontent.com/BaldissaraMatheus/Tasks.md/main/README.md>, <https://raw.githubusercontent.com/BaldissaraMatheus/Tasks.md/main/LICENSE>, <https://raw.githubusercontent.com/BaldissaraMatheus/Tasks.md/main/backend/server.js>, <https://raw.githubusercontent.com/BaldissaraMatheus/Tasks.md/main/frontend/vite.config.js>, <https://raw.githubusercontent.com/BaldissaraMatheus/Tasks.md/main/frontend/src/card-content-utils.js>, <https://hub.docker.com/v2/repositories/baldissaramatheus/tasks.md>, <https://github.com/BaldissaraMatheus/Tasks.md/releases>, <https://github.com/BaldissaraMatheus?tab=repositories>, <https://github.com/BaldissaraMatheus/Tasks.md/issues/49>

### Unified Kanban (VS Code Extension)
- Obsidian Kanban 형식과 호환되는 VS Code 칸반 보드 확장
- 장점: Obsidian Kanban 문법 100% 호환; 보드·표·목록 보기; 클라우드 의존성 없음
- 단점: 에이전트 훅·잠금 없음; 동시 편집 시 리로드 경합; MCP·CLI 미지원
- 출처: <https://github.com/divisionseven/unified-kanban>, <https://open-vsx.org/extension/divisionseven/unified-kanban>, <https://open-vsx.org/api/divisionseven/unified-kanban/0.7.3/file/readme.md>, <https://api.github.com/repos/divisionseven/unified-kanban>

### vscode-kanban (mkloubert)
- Mermaid·Toggl·JS 이벤트 자동화를 지원하는 고전 VS Code 칸반 확장
- 장점: 카드 이동 시 커스텀 스크립트 실행; Mermaid·Toggl 내장; 카드 마크다운 내보내기
- 단점: 2022년 이후 미유지; 기본 저장 형식이 JSON; AI 에이전트·MCP 미지원
- 출처: <https://github.com/mkloubert/vscode-kanban>, <https://marketplace.visualstudio.com/items?itemName=mkloubert.vscode-kanban>, <https://raw.githubusercontent.com/mkloubert/vscode-kanban/master/package.json>

## 미확인·검증 결과


### Omnara — 2026-10-16 확인
- 1차 출처(omnara.com/pricing): 플랫폼 요금 $0, 셀프호스팅 무료, Apache-2.0 (github.com/omnara-ai/omnara), Omnara 관리형 모델·머신만 종량제($0.0414/GiB-hour), 엔터프라이즈 문의.
- 모바일: App Store id6748426727, Google Play com.omnara.app (2026-04-09 업데이트).
- 판정: HN(2025-08) '10세션 무료/$9' 및 codeagent-mobile '$19.99/mo'는 과거 요금 → 현재는 무료+종량제. 포지셔닝이 'Claude Managed Agents 오픈소스 대안'으로 바뀜.
### Vibe Kanban — 2026-10-16 확인
- vibekanban.com: 'Vibe Kanban is sunsetting' 배너, 가격 페이지 Free $0 / $30 user/mo(2-49).
- nimbalyst.com/blog/vibe-kanban-after-bloop-whats-next: Bloop 2026-04-10 종료 발표, OSS 지속.
- github.com/BloopAI/vibe-kanban: 셀프호스팅 가이드 존재.
- 판정: 로컬 OSS(Apache-2.0 여부는 wave1 보고서로 교차확인) 사용 가능, 클라우드 기능은 셀프호스팅 필요, 유지보수 리스크 높음.
- README(raw.githubusercontent.com/BloopAI/vibe-kanban/main/README.md): 'Vibe Kanban is sunsetting', npx vibe-kanban 로컬 실행, Docker 셀프호스팅 가이드(vibekanban.com/docs/self-hosting/deploy-docker), 에이전트 10+종 지원, 내장 MCP 서버(MCP_HOST/MCP_PORT).
- GitHub API: license=Apache-2.0, stars=28376, pushed_at=2026-09-19T06:52:48Z, archived=false.

### Conductor — 2026-10-16 확인 (conductor.build/pricing)
- Free $0(로컬 Mac 워크스페이스, 병렬 에이전트, BYO 키) / Pro $50/월(클라우드, 멀티플레이어 5명, API, 모바일 앱) / Teams $60/사용자/월 / Enterprise 문의.
- 모바일 앱은 Pro 이상 전용. 셀프호스팅 클라우드는 미제공(엔터프라이즈용 개발 중). SOC 2 Type II.
- 판정: wave1 보고서 conductor.md와 일치 → CONFIRMED.
### Happy — 2026-10-16 확인
- happy.engineering/docs/faq: iPhone/iPad App Store 무료, 100% 오픈소스. Google Play com.ex3ndr.happy(2026-10-03 갱신). App Store id6748571505.
- github.com/slopus/happy: MIT(GitHub API), stars 24166, push 2026-10-15. E2E 암호화, iOS/Android/웹.
- 판정: 무료·OSS·iOS/Android 네이티브 CONFIRMED. Claude/Codex/Grok 지원. 칸반 뷰는 아님(세션 목록형).

### Linear — 2026-10-16 (linear.app/pricing)
- Free $0(무제한 멤버, 2팀, 이슈 250개, Agent platform/Linear Agent 포함) / Basic $10/사용자/월 / Business $16/사용자/월 / Enterprise 문의. 셀프호스팅 불가(SaaS).
### Backlog.md — 2026-10-16 (README)
- Git 저장소 Markdown 파일 기반 칸반, 'backlog board' TUI + 'backlog browser' 로컬 웹 칸반(드래그앤드롭), MCP/CLI 지원, npm i -g backlog, MIT(GitHub API). 모바일 앱 없음(로컬 웹을 모바일 브라우저로 접근 가능 수준).
### Task Master — 2026-10-16 (README)
- MCP 서버(claude mcp add taskmaster-ai -- npx -y task-master-ai) + CLI, Cursor 원클릭 설치. 라이선스 GitHub API NOASSERTION(커스텀: MIT with Commons Clause로 알려짐 → 보고서 값 확인 필요). 자체 칸반 UI 없음(Hamster 연동).
### Orca — 2026-10-16 (yetanotherorchestrator.app/apps/orca)
- 무료·MIT 오픈소스 ADE, macOS/Windows/Linux, iOS·Android 컴패니언 앱, Git worktree 격리, 원격 서버 지원.

### GitHub Copilot cloud agent — 2026-10-16 (github.com/features/copilot/plans, docs.github.com copilot-on-github)
- 요금: Free $0(제한적 에이전트 사용, CLI·agent mode) / Pro $10/월 / Pro+ $39/월 / Max $100/월 (+ 비즈니스 플랜 별도). 문서 목차에 'Use cloud agent on GitHub Mobile' 존재 → GitHub Mobile(iOS/Android)로 에이전트 작업 지시·확인 가능. 셀프호스팅: GitHub Enterprise Server 별도(클라우드 에이전트는 GitHub.com 기준).
### Cursor (cloud/background agents) — 2026-10-16 (cursor.com/pricing, cursor.com/docs)
- 주의: 2026-10 cursor.com/pricing은 xAI 통합 후 Grok 번들 요금표(Free $0 / Plus $8 / Premium $30 / Super $100 / Ultra $200 / Teams $30+/사용자). 문서 목차에 'Mobile', 'Self-Hosted Machines', 'API', 'CLI' 존재 → 모바일 웹/앱에서 에이전트 확인 가능, 에이전트 실행 머신 셀프호스팅 가능. (요금 구조가 바뀐 상태라 보고서 값과 교차확인 필요)
### OpenAI Codex cloud — 2026-10-16 (developers.openai.com/codex/cloud → learn.chatgpt.com/docs/cloud)
- ChatGPT 웹·모바일 앱·데스크톱 앱에서 Codex 클라우드 작업 확인·리뷰 가능('On mobile, open Codex'). ChatGPT 요금제에 포함. 셀프호스팅 불가(Codex CLI는 OSS 로컬).

### 범용 칸반 공식 페이지 직접 수집 — 2026-10-16 (원문: verify-general-raw.txt)
- Plane(plane.so/pricing): Free $0 / Pro $6 / Business $13 /좌석/월, Cloud·Self-hosted 탭, 오픈소스.
- Trello(trello.com/pricing): Free(10명) / Standard $5 / Premium $10 / Enterprise $17.50 /사용자/월(연간), iOS·Android 앱. 셀프호스팅 불가.
- Notion(notion.com/pricing): AI 크레딧 $10/1,000크레딧, 데스크톱·모바일 앱. (좌석 요금은 페이지 동적 렌더로 미추출)
- ClickUp(clickup.com/pricing): Free / Unlimited $7 / Business $12(연간; 정가 $17~25) /사용자/월, iOS·Android.
- Asana(asana.com/pricing): iOS·Android·데스크톱 앱, AI 요청 $0.50~0.60/건(엔터프라이즈). 좌석 요금 미추출.
- monday.com: Free(2석) / Basic $9 / Standard $12 / Pro $19 /석/월, iOS·Android.
- Huly(huly.io/pricing): Free / Rare $19 / Epic $99 / Legendary $399(워크스페이스 단위), OSS(EPL-2.0) 셀프호스팅.
- Shortcut: Free $0 / Team $8 / Business $12 / Enterprise 문의.
- Vikunja: 오픈소스, 셀프호스팅 또는 호스팅 서비스. WeKan: 오픈소스 셀프호스팅. Kanboard: 무료 오픈소스 셀프호스팅.
- Jira·Planka 가격 페이지: JS 렌더로 추출 실패 → wave1 보고서 값 사용, 표에 '보고서 기준' 표기.

### 보고서 대조 (wave1 vs lead 직접 확인)
- vibe-kanban.md: Apache-2.0, 28,376 stars, sunsetting, Docker 셀프호스팅 → lead 확인(GitHub API·README)과 일치 → CONFIRMED. 모바일 앱 없음(반응형 웹/원격 접속).
- cursor-bg-agents.md: Grok 번들 요금표(Free/Plus $8/Premium $30/Super $100/Ultra $200/Teams $30+)·Self-Hosted Machines → lead가 cursor.com/pricing·docs 목차에서 본 내용과 일치 → CONFIRMED. 보고서 추가 사실: iOS 앱(id6767085653), Android는 PWA, Cloud Agents는 Premium 이상.
- omnara.md: 일치(앞 기록).

### 대조: 새 보고서 vs lead 직접 확인
- github-agent-hq.md: Pro $10 / Pro+ $39 / Max $99~100(연간) — lead가 github.com/features/copilot/plans에서 본 Free/Pro $10/Pro+ $39/Max $100과 일치(Max 표기 $99 vs $100 차이는 연간·월간 표기 차이). GitHub Mobile 'Copilot agent sessions' — lead가 docs 목차의 'Use cloud agent on GitHub Mobile'에서 확인한 내용과 일치 → CONFIRMED.
- devin.md 내 Linear 가격(Standard $8/$10, Plus $14/$16) vs lead 직접 확인(linear.app/pricing: Basic $10, Business $16 /사용자/월) → 상충: 플랜 이름이 다름(보고서는 연간가와 월간가를 함께 적음). 문서에는 lead 1차 출처 값(Free / Basic $10 / Business $16)을 쓴다.

### 추천 신규 도구 직접 확인 — 2026-10-16 (README·GitHub API)
- Multica: 칸반 보드(에이전트가 컬럼 이동), Docker Compose·Helm 셀프호스팅 확인. 모바일은 iOS·iPadOS 앱을 소스로 직접 빌드해야 하며 App Store에는 없음, Android 언급 없음 → 노드 보고서의 'iOS·Android 앱'은 과장이라 정정. ★52,949, 라이선스 NOASSERTION(커스텀).
- AgentsMesh: BSL-1.1, Docker, Web/Desktop/iOS(SwiftUI) 클라이언트 확인. ★2,361.
- Agent Orchestrator: Apache-2.0, ★13,217, 2026-10-16 push.
- Paca: Apache-2.0, Docker Compose·K8s/Helm 셀프호스팅, 무료 확인. ★1,928.


## 출처 및 방법론

- 규모(최종): 조사 노드 87개(1차 70 + 2차 디렉터리 14 + 검증 3), 결과 보고서 82개+리드 직접 조사 4건, 도구 블록 331개 → 중복 제거 후 비교표 255개 도구. 출처 URL 1418개(고유 도메인 300개, sources-ledger.md).
- 2차 검증 결과: 에이전트 전용 도구 172개 전부 공식 페이지·GitHub API(·디렉터리) 근거로 재판정(verify-wave2.json). 상충은 공식 값으로 정정하거나 '미확인(공식 근거와 상충: …)'으로 표기. 리드가 공식 가격·README를 직접 읽어 확정한 행은 '리드 직접 2차 확인'으로 표시.
- 노드 보고서가 과장했던 사례(정정함): Multica 모바일('iOS·Android 앱' → 실제 iOS 소스 빌드만), Pane 라이선스·셀프호스팅, Claude Code on the web 셀프호스팅, Automaker 라이선스, Maude 가격, Linear 플랜명.
- 조사 방식: mass ulw research. 1차 웨이브는 DAG 2개(에이전트 전용 30개 + 발굴 스윕 8개, 범용 칸반 32개)입니다. 2차 웨이브는 에이전트 오케스트레이터 디렉터리(yetanotherorchestrator.app, 48개)에서 나온 신규 도구 42개를 14개 노드로 나눠 조사했습니다. 끝으로 노드 보고서를 한국어 값으로 정규화하는 단계를 거쳤습니다.
- 2차 검증: 리드가 공식 가격 페이지와 README, App Store/Play 상점 목록을 직접 확인했습니다. 근거는 `verify-lead.md`와 `verify-general-raw.txt`에 있습니다. 오픈소스 32개 저장소의 라이선스·스타 수·마지막 push는 GitHub API로 교차 확인했습니다(`github-facts.txt`).
- 규칙: 보고서에 없거나 확인하지 못한 값은 '미확인'으로 표기했습니다. 가격은 조사일(2026-10-16) 기준이며 수시로 바뀔 수 있습니다.
- 제약: 조사 도중 웹 검색 API에 레이트 리밋(429)이 걸려, 일부는 공식 페이지를 직접 가져오는 방식으로 대체했습니다. Jira와 Planka의 가격 페이지는 JS 렌더링 때문에 직접 추출하지 못해 노드 보고서 값을 사용했습니다.

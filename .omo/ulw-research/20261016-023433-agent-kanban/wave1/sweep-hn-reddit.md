# Hacker News & Reddit Sweep: AI Agent Kanban & Multi-Agent Dashboards (2025–2026)
Date: 2026-10-16
Survey Scope: Hacker News (Show HN / Ask HN / Algolia), Reddit (r/ClaudeAI, r/ChatGPTCoding, r/selfhosted, r/LocalLLaMA, r/ClaudeCode), and developer communities for "kanban for AI agents", "manage multiple coding agents", and "agent dashboard mobile".

Seen tools (already known / obvious): Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md (기존 wave1 개별 프로파일링 도구: Omnara, Conductor, Terragon, Devin, Sculptor, Shrimp Task Manager, Happy Coder, Orca, Factory, Claude Squad, GitHub Agent HQ, Cursor BG agents, Kiro, Claude Code UI, Crystal).

---

### KaibanJS
- URL: https://www.kaibanjs.com/
- Kind: agent-native
- What: JavaScript/TypeScript 기반의 네이티브 멀티 에이전트 오케스트레이션 프레임워크로, 에이전트의 사고 과정·상태·작업 단계를 실시간으로 시각화하고 제어하는 오픈소스 Kaiban Board UI를 기본 제공한다.
- Agent integration: JavaScript/TypeScript SDK(`npm install kaibanjs`), React 컴포넌트(`@kaibanjs/board`), Node.js 이벤트 훅; OpenAI, Anthropic, Ollama 등 모든 주요 LLM 백엔드 및 LangChain 도구 생태계와 연동; 에이전트가 칸반 보드의 각 열(Todo, In Progress, Done)을 따라 구조화된 태스크를 단계별로 자동 실행 및 상태 갱신.
- Mobile: responsive web (Kaiban Board 플레이그라운드 및 React 컴포넌트가 반응형 웹 UI를 지원하여 모바일 브라우저에서 보드 조회 및 워크플로 실행 가능; 전용 iOS/Android 네이티브 앱은 미제공, cite: 공식 문서 및 GitHub).
- Pricing: 100% 무료 및 오픈소스 (MIT License, BYO API 키).
- Self-host: yes (로컬 Vite 개발 서버 `npm run kaiban` 또는 `npx kaibanjs@latest init`으로 로컬 즉시 실행 가능, React/Next.js 웹앱에 내장 임베딩 가능, Docker 컨테이너 호환).
- License: MIT
- Activity: GitHub 1,480 stars, 마지막 커밋 2026-05-15, npm 패키지 활성 유지보수.
- Pros: AI 에이전트 워크플로 전용으로 설계된 시각적 칸반 보드 UI(Kaiban Board); JS/TS 친화적으로 프론트엔드/Node.js 프로젝트에 손쉽게 임베딩 가능; 에이전트의 사고 프로세스 및 툴 호출 결과 실시간 실시간 인스펙션; 완전 무료 MIT 라이선스.
- Cons: Git worktree 기반의 병렬 코딩 에이전트(Conductor/Orca 스타일)보다는 워크플로 파이프라인 에이전트 실행에 특화; 전용 네이티브 모바일 앱 부재; Python 생태계(CrewAI/AutoGen)에 비해 상대적으로 작은 커뮤니티 규모.
- Sources: https://www.kaibanjs.com/, https://github.com/kaiban-ai/KaibanJS, https://github.com/kaiban-ai/kaiban-board, https://docs.kaibanjs.com/
- Confidence: high

### Agent Kanban
- URL: https://agent-kanban.dev/
- Kind: agent-native
- What: 사람이 에이전트를 일일이 감시하지 않고(Take human out of the loop) 자율적으로 개발할 수 있도록 설계된 에이전트 전담 미션 컨트롤 칸반 보드로, 에이전트를 1급 액터로 취급하여 작업 할당, 의존성 추적, 검증된 세션 증적(Provenance), 독립 리뷰 게이트를 제공한다.
- Agent integration: OpenAPI 기반 라이브 REST 계약, Realmroot Toolbox 인증 도구 연동, Enbor 런타임 세션 바인딩, GitHub App 연동; 에이전트가 보드의 태스크를 직접 검색(discover)하고 클레임(claim)하여 작업을 수행하며 리뷰 제출(review submission)까지 자율 처리.
- Mobile: responsive web (Cloudflare Workers + React/Tailwind 기반의 반응형 모바일 웹 UI 지원; 전용 iOS/Android 네이티브 앱 없음, cite: 공식 사이트 agent-kanban.dev 및 GitHub saltbo/agent-kanban).
- Pricing: 무료 오픈소스 (FSL-1.1-ALv2 라이선스, 개인/자체 배포 무료, schema.org offer price $0).
- Self-host: yes (Node.js 24 + pnpm, Cloudflare Workers / Wrangler D1 또는 로컬 개발 서버 환경에서 셀프호스팅 가능).
- License: FSL-1.1-ALv2 (Functional Source License, 2년 후 Apache-2.0 전환).
- Activity: GitHub 492 stars, 마지막 커밋 2026-10-02, 활성 개발 중.
- Pros: 에이전트를 1급 액터로 모델링한 엄격한 작업 수명주기(Todo → Claim → In Progress → Review → Done); 작업 담당 에이전트와 승인 리뷰어의 권한 분리로 탈주/오작동 방지; 작업 간 선후행 의존성(Dependency-aware) 그래프 관리; Cloudflare D1 기반 경량 배포.
- Cons: 완전한 자율 디스패치를 위해 Realmroot OIDC 및 Enbor 세션 인프라 연계 필요; 완전한 영구 오픈소스가 아닌 FSL 라이선스; 네이티브 모바일 앱 미제공.
- Sources: https://agent-kanban.dev/, https://github.com/saltbo/agent-kanban
- Confidence: high

### KanVibe
- URL: https://github.com/rookedsysc/kanvibe
- Kind: agent-native
- What: 흩어진 터미널 탭 대신 실시간 칸반 보드에서 브랜치 기반 작업을 추적하고 내장 tmux/zellij 터미널을 제어하며, 에이전트 수명주기 훅을 통해 카드가 자동으로 이동하는 키보드 우선(Keyboard-first) 데스크톱 칸반 워크스페이스.
- Agent integration: 생명주기 훅(Hook-driven)을 통한 자동 카드 상태 전이(에이전트 시작/종료 시 Todo → In Progress → Review → Done 자동 이동); 내장 tmux 및 zellij 터미널 제어; Claude Code, Gemini CLI, Codex CLI, OpenCode 공식 지원; 보드 카드에서 실시간 서브태스크 트리 및 에이전트 계정별 남은 쿼터(Claude 주간 한도 등) 직접 표시.
- Mobile: none (macOS 데스크톱 Electron/Tauri 앱 중심, Homebrew Cask `brew install --cask rookedsysc/kanvibe/kanvibe` 배포, 로컬 LAN 브라우저 접속 외 네이티브 모바일 앱 없음, cite: 공식 README 사전요구사항 및 Homebrew 가이드).
- Pricing: 무료 오픈소스 (AGPL-3.0, Buy Me a Coffee 자발적 후원).
- Self-host: yes (로컬 데스크톱 앱 및 로컬 tmux/git 기반 완전 로컬 우선 동작, 외부 클라우드 불필요).
- License: AGPL-3.0
- Activity: GitHub 147 stars, 마지막 커밋 2026-10-15, 활발한 릴리스 유지보수.
- Pros: Vim 스타일의 키보드 단축키(`h/j/k/l`, `:move`)로 마우스 없이 칸반 보드 완벽 제어; 에이전트 실행 상태에 따른 자동 카드 이동으로 수동 드래그앤드롭 불필요; 터미널을 떠나지 않고 계정별 AI 잔여 쿼터/사용량 실시간 모니터링; 태스크별 격리된 git 브랜치 및 worktree 자동 생성.
- Cons: 네이티브 모바일 앱 없음; git, tmux, gh CLI 등 호스트 머신 로컬 종속성 설치 필수; 기업 환경에서 AGPL-3.0 라이선스 검토 필요.
- Sources: https://github.com/rookedsysc/kanvibe, https://news.ycombinator.com/item?id=47031388 (Show HN)
- Confidence: high

### OpenKanban
- URL: https://github.com/TechDufus/openkanban
- Kind: agent-native
- What: Go 언어로 작성된 초경량 터미널 사용자 인터페이스(TUI) 칸반 보드로, 복수의 AI 코딩 에이전트를 프로젝트별로 편성하고 각 티켓마다 독립된 Git worktree와 내장 터미널을 할당하여 오케스트레이션한다.
- Agent integration: TUI 내부 내장 터미널 및 프로세스 스폰; OpenCode, Claude Code, Gemini CLI, Codex CLI, Aider 또는 사용자 정의 임의 CLI 에이전트 지원(`~/.config/openkanban/config.json` 설정); 단축키 `s`로 해당 티켓 브랜치에서 에이전트를 즉시 실행하고 `enter`로 터미널에 어태치.
- Mobile: none (Go 기반 단일 바이너리 TUI, macOS/Linux 터미널 전용, SSH/Termius 원격 접속 외 전용 모바일 앱 없음, cite: 공식 README 및 릴리스 아티팩트).
- Pricing: 100% 무료 오픈소스 (AGPL-3.0).
- Self-host: yes (Go 단일 바이너리 `brew install TechDufus/tap/openkanban` 또는 `go install`, 로컬 파일 기반 설정).
- License: AGPL-3.0
- Activity: GitHub 151 stars, 마지막 커밋 2026-06-12, v0.2.x 릴리스.
- Pros: 단일 Go 바이너리로 실행되는 제로 블로트 초고속 TUI; 칸반 티켓과 Git worktree의 1:1 완벽 격리 매핑; 브라우저나 무거운 Electron 없이 터미널 안에서 다중 에이전트 전환; 범용 CLI 에이전트 확장성.
- Cons: GUI 웹 보드 및 네이티브 모바일 앱 없음; 훅 기반의 완전 자동 카드 전이 대신 수동 키조작(`space`) 중심; 단일 개발자 중심 프로젝트.
- Sources: https://github.com/TechDufus/openkanban, https://news.ycombinator.com/item?id=46892301
- Confidence: high

### PlateSpinner
- URL: https://github.com/moridinamael/platespinner
- Kind: agent-native
- What: 로컬 프로젝트를 지정하고 원하는 기능을 자연어로 지시하면, 헤드리스 AI 세션을 스폰하여 제안(Propose) → 계획(Plan) → 실행(Execute)의 3단계 파이프라인을 거치며 코드를 작성하고 드래그 앤 드롭 칸반 보드에서 진행 상황을 실시간 스트리밍하는 도구.
- Agent integration: Claude Code, Codex CLI, Gemini CLI 헤드리스 자식 프로세스 스폰; WebSocket 기반 표준 입출력, 변경 파일, 토큰 비용 실시간 양방향 스트리밍; 작업별 격리 브랜치(`kanban/task-...`) 자동 생성; AI 심사관(Judge)이 자율적으로 루프를 돌리는 'Autoclicker' 모드 내장.
- Mobile: responsive web (로컬 Node.js 서버 웹 인터페이스가 반응형 레이아웃을 제공하여 모바일 브라우저에서 보드 조회 및 진행 확인 가능; 네이티브 모바일 앱 없음, cite: 공식 README 시스템 요구사항).
- Pricing: 100% 무료 오픈소스 (MIT License, 로컬 CLI 및 사용자 API 키 활용).
- Self-host: yes (Node.js 18+ 환경에서 `git clone` 및 `npm start`로 로컬 머신에서 완전 셀프호스팅).
- License: MIT
- Activity: GitHub 13 stars, 마지막 커밋 2026-04-20, Show HN 2026년 3월 공개.
- Pros: 읽기 전용 분석/계획(Propose/Plan)과 쓰기 권한 실행(Execute)의 명확한 안전 3단계 격리; AI 판사가 예산 한도 내에서 자율 작업하는 Autoclicker 모드; 내장 diff 뷰어 및 작업별 실시간 토큰/비용 추적; 깔끔한 드래그앤드롭 웹 칸반 보드.
- Cons: 커뮤니티 규모가 매우 작음 (13 stars); 모바일 푸시 알림 부재; 호스트 환경에 로컬 CLI(Claude Code 등)가 사전 설치되어 있어야 함.
- Sources: https://github.com/moridinamael/platespinner, https://news.ycombinator.com/item?id=47281358 (Show HN)
- Confidence: high

### Moo Tasks
- URL: https://mootasks.dev
- Kind: agent-native
- What: 드래그 앤 드롭 웹 칸반 보드 인터페이스와 내장 MCP(Model Context Protocol) 서버를 결합한 멀티유저 작업 관리 시스템으로, 사람은 브라우저에서 태스크를 정리하고 AI 에이전트는 MCP 도구를 통해 태스크를 프로그래밍 방식으로 생성·수락·갱신한다.
- Agent integration: 보드별 전용 네이티브 MCP 서버 내장(SSE 및 HTTP 전송, Bearer 토큰 인증); `list_tasks`, `get_task`, `create_task`, `update_task_status`, `add_task_comment` 등 완전한 MCP 도구 세트 제공; Claude Desktop, Claude Code, Cursor, Cline 등 표준 MCP 지원 클라이언트 연동; 완료 태스크 기반 AI 체인지로그 자동 생성.
- Mobile: responsive web (Nuxt 4 + Tailwind CSS v4 기반 반응형 웹 UI로 스마트폰 모바일 브라우저에서 터치 드래그 및 보드 열람 최적화; 전용 네이티브 앱 없음, cite: mootasks.dev 공식 사이트 및 기술 스택 명세).
- Pricing: 무료 오픈소스 (MIT License, 공식 클라우드 인스턴스 무료 제공).
- Self-host: yes (Docker Compose 지원, Nuxt 4 + MySQL + Drizzle ORM 기반 자체 서버 호스팅).
- License: MIT
- Activity: GitHub 45 stars, 마지막 커밋 2026-06-21, 2026년 4월 Show HN 발표.
- Pros: MCP 프로토콜 표준을 최초 수준으로 온전히 내장한 순수 에이전트 친화적 칸반 보드; 보드별 토큰 분리 및 공개/비공개 제어로 안전한 멀티 에이전트 협업; 작업 수정 요구를 하위 태스크로 연결하는 Correction Task 기능; Nuxt 4 기반의 빠른 반응형 웹 UI.
- Cons: 백엔드 실행을 위해 MySQL 데이터베이스 인프라 필요; 에이전트 CLI 프로세스를 자체적으로 실행하거나 worktree를 관리하는 런너 기능은 없음(작업 상태 보드 전용); 네이티브 모바일 앱 미제공.
- Sources: https://mootasks.dev, https://github.com/dizlexic/moo-tasks, https://news.ycombinator.com/item?id=47954002 (Show HN)
- Confidence: high

### Sortie
- URL: https://sortie-ai.com/
- Kind: agent-native
- What: 단일 Go 바이너리로 실행되는 오픈소스 코딩 에이전트 오케스트레이터로, 기존 이슈 트래커(GitHub, Linear, Jira 등)의 티켓을 감지하여 독립된 작업 공간에서 다중 에이전트를 병렬 실행하고 상태를 모니터링 대시보드에 실시간 반영한다.
- Agent integration: Claude Code, GitHub Copilot, OpenCode, Codex, Kiro, Gemini 공식 어댑터 내장; GitHub Issues, GitLab Issues, Gitea, Linear, Jira 등 트래커 플러그형 연동; 단일 `WORKFLOW.md` 선언형 워크플로(핫 리로딩 지원); 실시간 변경 스트림을 제공하는 JSON REST API 및 Prometheus 메트릭 엔드포인트 내장.
- Mobile: responsive web (단일 포트로 서빙되는 내장 모바일 반응형 웹 대시보드; 전용 네이티브 모바일 앱은 없음, cite: docs.sortie-ai.com 및 공식 아키텍처).
- Pricing: 100% 무료 오픈소스 (Apache-2.0).
- Self-host: yes (단일 정적 Go 바이너리 `brew install --cask sortie-ai/tap/sortie` 또는 `curl | sh`, Docker 지원, 로컬 머신 또는 사설 서버에서 완전 자가 호스팅).
- License: Apache-2.0
- Activity: GitHub 203 stars, 마지막 커밋 2026-10-15, v0.x 활성 릴리스.
- Pros: 기존 엔터프라이즈 이슈 트래커(Linear, Jira, GitHub)와 에이전트 런타임 간의 매끄러운 양방향 상태 동기화; `WORKFLOW.md` 파일 하나로 끝나는 직관적인 선언형 구성; CI 실패 및 코드 리뷰 피드백 시 에이전트 자동 재시도 루프; 단일 Go 바이너리의 가벼운 배포.
- Cons: 독립형 인터랙티브 드래그앤드롭 칸반 보드라기보다는 외부 트래커를 소스로 삼는 실행 모니터링 대시보드 성격; 네이티브 모바일 앱 없음; 외부 트래커가 연동되지 않으면 티켓 생성이 제한적.
- Sources: https://sortie-ai.com/, https://docs.sortie-ai.com/, https://github.com/sortie-ai/sortie
- Confidence: high

### Agent Deck
- URL: https://github.com/asheshgoplani/agent-deck
- Kind: agent-native
- What: Go 언어로 개발된 다중 AI 코딩 에이전트(Claude Code, OpenCode, Codex, Gemini, Pi) 전용 터미널 커맨드 센터로, 수십 개 프로젝트의 병렬 세션을 그룹화·포크·검색하고 비용을 추적하며 모바일 폰 텔레그램/슬랙 봇(Conductor)으로 원격 제어할 수 있다.
- Agent integration: CLI 터미널 멀티플렉싱 및 프로세스 관리; Claude Code, OpenCode, Codex CLI, Gemini CLI, Pi 공식 지원; 네이티브 대화 포크(Fork with history inheritance); MCP 서버 동적 어태치/디태치 관리자(`m` 키); 세션별 Git worktree 자동 생성; 로컬 웹 대시보드(`agent-deck web`, 포트 8420).
- Mobile: responsive web (`agent-deck web` 실행 시 모바일 웹 브라우저 접속 지원) 및 폰 원격 제어용 Conductor 봇(Telegram, Slack, Discord 봇을 통해 모바일 폰으로 에이전트 상태 조회, 실시간 알림 수신, 승인 및 지시 전송 지원, ntfy 푸시 알림 연계, cite: 공식 README Conductor 섹션 및 docs/conductor).
- Pricing: 100% 무료 오픈소스 (MIT License).
- Self-host: yes (단일 컴파일 Go 바이너리 `brew install asheshgoplani/tap/agent-deck` 또는 `curl | bash`, macOS, Linux, WSL 완전 로컬 실행).
- License: MIT
- Activity: GitHub 1,092 stars, 마지막 커밋 2026-10-14, 활발한 v1.16+ 릴리스.
- Pros: 1,000스타 이상의 검증된 완성도와 활발한 오픈소스 커뮤니티; 텔레그램/슬랙 봇(Conductor)을 통한 실용적인 스마트폰 원격 모니터링 및 승인; 컨텍스트를 유지하는 세션 즉각 포크(Forking) 기능; 내장 로컬 웹 GUI(`agent-deck web`)와 터미널 TUI의 동시 지원.
- Cons: 전통적인 열 기반 드래그앤드롭 칸반 보드보다는 세션 목록 및 그리드 뷰 중심; 모바일 네이티브 스토어 앱이 아닌 텔레그램/슬랙 봇 또는 웹 기반; 원격 봇 연동 시 봇 토큰 설정 필요.
- Sources: https://github.com/asheshgoplani/agent-deck, https://news.ycombinator.com/item?id=46272551 (Show HN)
- Confidence: high

### Offrun
- URL: https://offrun.dev/
- Kind: agent-native
- What: Mac 환경에서 Claude Code, OpenAI Codex, Google Antigravity CLI 등 다중 코딩 에이전트를 독립된 Git worktree에서 병렬 실행하고, 쿼터 소진 시 계정을 자동 전환하며 동료 에이전트 간 상호 코드 리뷰(Peer Review)를 수행하는 네이티브 데스크톱 워크스페이스.
- Agent integration: Claude Code, OpenAI Codex CLI, Google Antigravity CLI 직접 프로세스 통합; 태스크별 격리된 Git worktree 및 브랜치 자동 할당; 에이전트 쿼터 소진 시 복수 계정 자동 롤오버(Account rotation); 사용자 승인 전 보조 에이전트가 diff를 먼저 읽고 검증하는 자동 피어 리뷰 파이프라인.
- Mobile: none (macOS 14 Sonoma 이상 Apple Silicon/Intel 전용 네이티브 데스크톱 앱, 모바일 앱 및 외부 클라우드 미제공, cite: 공식 FAQ "What do I need to run this? A Mac running macOS 14 Sonoma or later").
- Pricing: 100% 무료 ("Free. All of it. Free and unlimited. No credit card.", 사용자 자체 CLI 구독 및 API 키 활용).
- Self-host: yes (로컬 macOS 데스크톱 애플리케이션으로 코드와 프로젝트 메모리가 로컬 Mac 밖으로 유출되지 않음).
- License: 상용 무료웨어(Proprietary freeware, 비공개 소스).
- Activity: 2026년 10월 Show HN 발표, 최신 macOS 14+ 대응 활성 배포 중.
- Pros: 단일 에이전트 사용 한도 초과 시 자동으로 다중 계정을 전환하여 중단 없는 개발 가능; 에이전트 작성 코드를 다른 에이전트가 사전 검증하는 자동 피어 리뷰 기능; 완벽한 로컬 프라이버시(음성 인식 및 프로젝트 메모리 전부 Mac 로컬 처리); Git worktree 충돌 방지.
- Cons: macOS 전용 (Windows/Linux 미지원); 소스코드가 공개되지 않은 상용 프리웨어; 네이티브 모바일 컴패니언 없음; 드래그 앤 드롭 열 형태의 칸반이라기보다는 병렬 작업 세션 관리자 구조.
- Sources: https://offrun.dev/, https://news.ycombinator.com/item?id=49247385 (Show HN)
- Confidence: high

### Agent Orchestrator (Orchestrator.inc)
- URL: https://ao-agents.com/
- Kind: agent-native
- What: 10개 이상의 병렬 코딩 에이전트를 충돌 없는 Git worktree에서 팀 단위로 실행·감독하는 오픈소스 데스크톱 워크스페이스로, 실시간 운영 칸반 보드(Working, Needs you, In review, Ready to merge)를 통해 작업 진행, PR, CI 및 코드 리뷰를 총괄한다.
- Agent integration: 25개 이상의 코딩 에이전트 하네스 지원(Claude Code, OpenAI Codex, Cursor, OpenCode, Aider, GitHub Copilot, Grok, Kimi, Pi, Amp, Auggie, Droid, Cline, Goose 등); 전체 프로젝트 계획을 수립하고 하위 작업자에게 위임하는 상위 기획 오케스트레이터 에이전트 내장; 작업자별 격리된 인앱 브라우저 프리뷰 및 Git diff 뷰어.
- Mobile: responsive web & mobile companion ("Connect Mobile" 기능 제공: 인증된 LAN/Tailscale 리스너 및 QR 코드 페어링을 통해 모바일 브라우저/PWA에서 실시간 모니터링 및 승인, macOS 잠자기 방지 wakefulness 코디네이터 내장, cite: docs/adr/0001-lan-listener-for-mobile.md 및 docs/adr/0005-mobile-closed-lid-wake.md).
- Pricing: 100% 무료 오픈소스 (Apache-2.0, BYOK 모델).
- Self-host: yes (macOS, Windows, Linux 데스크톱 앱 및 Go 기반 로컬 데몬, 개발자 하드웨어에서 100% 독립 실행).
- License: Apache-2.0
- Activity: GitHub 13,216 stars, 마지막 커밋 2026-10-16, 초대형 활성 오픈소스 프로젝트.
- Pros: 작업자 상태·PR·CI 통과 여부와 직결되는 실전형 운영 칸반 보드; 현존 최다 수준(25종 이상)의 코딩 에이전트 하네스 지원; 상위 오케스트레이터와 하위 워커의 체계적인 기획-위임 모델; 모바일 LAN/Tailscale QR 페어링과 맥북 덮개 닫힘 상태 유지(Wakefulness) 등 뛰어난 모바일 연동 엔지니어링.
- Cons: 일렉트론 데스크톱 앱과 백엔드 데몬이 결합된 상대적으로 무거운 아키텍처; 10대 이상의 에이전트 동시 구동 시 호스트 시스템 자원(RAM/CPU) 소모 큼; 앱스토어 정식 등록 네이티브 앱은 아니며 모바일 웹/PWA 페어링 방식.
- Sources: https://ao-agents.com/, https://orchestrator.inc/, https://github.com/OrchestratorInc/agent-orchestrator
- Confidence: high

### MindFlock
- URL: https://mindflock.ai
- Kind: agent-native
- What: 이슈 트래커(GitHub Issues, Jira, Linear, Shortcut, Asana)에 자신에게 할당된 티켓을 감지하여 로컬 머신에 격리된 Git worktree와 의존성 설치 및 티켓 맥락이 주입된 AI 에이전트 세션을 타이핑 없이 자동 생성해 주는 오픈소스 워크스페이스.
- Agent integration: 외부 트래커 API 양방향 동기화(GitHub, Jira, Linear, Shortcut, Asana); Claude Code, Codex, Cursor, OpenCode 등 로컬 에이전트 세션 자동 생성 및 맥락 시딩(티켓 제목, 본문, 인수 기준, 댓글 주입); 격리된 Git worktree(`feature/<ticket>/...`) 자동 관리; 내장 로컬 웹 서버(`http://127.0.0.1:8765`).
- Mobile: responsive web (데스크톱 GUI 외에 헤드리스 `mindflock serve` 명령으로 모바일 브라우저에서 로컬 포트 8765 웹 대시보드 접근 가능; 전용 네이티브 모바일 앱은 없음, cite: 공식 README Getting Started).
- Pricing: 100% 무료 오픈소스 (Apache-2.0).
- Self-host: yes (macOS, Windows, Linux 데스크톱 앱 또는 Python 3.12+ CLI `curl | sh`, 완전 로컬 셀프호스팅).
- License: Apache-2.0
- Activity: GitHub 15 stars, 마지막 커밋 2026-10-15, 활발한 프리릴리스 업데이트.
- Pros: Jira, Linear, GitHub Issues 등 실무 이슈 트래커와 로컬 코딩 에이전트 간의 완벽한 핸즈프리 연동; 티켓 할당 시 자동으로 worktree와 브랜치, 초기 프롬프트가 세팅됨; 데스크톱 GUI 및 헤드리스 웹 대시보드 동시 지원; 로컬 우선 보안.
- Cons: 초기 프로젝트로 커뮤니티 규모가 아직 작음; 모바일 전용 앱 미제공; 외부 티켓 큐가 없는 단독 칸반 생성 기능은 부차적.
- Sources: https://mindflock.ai, https://github.com/MindFlock/MindFlock, https://news.ycombinator.com/item?id=47926410 (Show HN)
- Confidence: high

### Maestro
- URL: https://maestro-build.com
- Kind: agent-native
- What: 여러 코딩 에이전트(Claude Code, Codex, Cursor 등)를 Git worktree 단위로 병렬 구동하는 데스크톱 앱으로, 스마트폰 전용 PWA인 'Maestro Web'을 통해 주머니 속에서 에이전트 실시간 로그 확인, 지시 전송, diff 리뷰 및 푸시 알림을 수신할 수 있는 원격 제어 도구.
- Agent integration: Claude Code, OpenAI Codex, Cursor 등 다양한 CLI 코딩 에이전트 프로세스 병렬 구동; 작업공간별 독립된 Git worktree 분리; 데스크톱과 Maestro Web 간의 종단간 암호화 실시간 상태 동기화; 모바일 카메라 롤의 스크린샷이나 사진을 즉시 에이전트 작업 지시로 전달 가능.
- Mobile: PWA (Maestro Web `https://app.maestro-build.com`, 홈 화면 추가 시 앱처럼 동작, 실시간 대화 스트리밍, 카메라 롤 사진 지시 전송, 코드 diff 라인 단위 리뷰, 에이전트 작업 완료/대기 푸시 알림 지원, cite: 공식 README 및 app.maestro-build.com).
- Pricing: 100% 무료 ("Free desktop app... Free, with your GitHub sign-in").
- Self-host: yes (macOS, Windows, Linux 데스크톱 앱 로컬 구동, Maestro Web 클라우드 중계 릴레이 또는 자체 호스팅 지원).
- License: 소스 공개 프리웨어(GitHub `PlathsOven/maestro` 저장소公开).
- Activity: GitHub 3 stars, 마지막 커밋 2026-10-12, 2026년 9월 Show HN 공개.
- Pros: 스마트폰 모바일 경험(Maestro Web PWA)에 가장 집중한 코딩 에이전트 컨트롤러; 모바일에서 코드 diff 리뷰 및 라인별 피드백 전송; 13가지의 감각적인 UI 테마(Catppuccin, Nord, Synthwave 등); 크로스 플랫폼(macOS/Win/Linux) 지원.
- Cons: 신생 프로젝트로 GitHub 스타 및 커뮤니티 레퍼런스가 적음; 모바일 푸시 및 동기화를 위해 GitHub 로그인 기반 릴레이 중계 사용; 네이티브 앱스토어 설치형이 아닌 PWA 방식.
- Sources: https://maestro-build.com, https://github.com/PlathsOven/maestro, https://app.maestro-build.com, https://news.ycombinator.com/item?id=49018431 (Show HN)
- Confidence: high

### Relayd
- URL: https://relayd.dev
- Kind: agent-native
- What: OpenAI Codex 등 로컬 코딩 에이전트 세션을 모바일 폰에 연결하여, 에이전트 작업 완료 및 대기 상태를 실시간 웹 푸시 알림으로 수신하고 스마트폰에서 바로 후속 작업을 이어갈 수 있게 해주는 모바일 대시보드 PWA.
- Agent integration: OpenAI Codex CLI 및 코딩 에이전트 워크스페이스 연동; 로컬 릴레이 데몬/웹훅을 통해 에이전트 상태 변경 시 모바일 푸시 알림 트리거; 모바일 인터페이스에서 에이전트에 직접 후속 지시 및 피드백 프롬프트 전달.
- Mobile: PWA (Progressive Web App, `manifest.webmanifest` 내장, `display: standalone`, Apple 터치 아이콘, iOS Safari 및 Android Chrome 홈 화면 설치 및 백그라운드 웹 푸시 알림 지원, cite: relayd.dev/manifest.webmanifest 및 Show HN).
- Pricing: 무료 티어 제공 (Free tier available, 상위 무제한/팀 플랜 유료, cite: HN 제출문).
- Self-host: yes (로컬 릴레이 데몬 자체 구동 가능, 클라우드 호스팅 PWA 연계).
- License: 상용 서비스(Proprietary / freemium).
- Activity: 2026년 1월 Show HN 발표, 활성 PWA 서비스 운영 중.
- Pros: 에이전트 완료 시점을 놓치지 않도록 설계된 모바일 특화 푸시 알림; 스마트폰 홈 화면에 완벽하게 설치되는 독립형 PWA; 책상을 벗어나 이동 중에도 Codex 에이전트 제어 가능; 매우 가벼운 설정.
- Cons: OpenAI Codex 중심의 연동 구조; 독립적인 칸반 보드 기능보다는 모바일 알림 및 세션 연속성에 집중; 비공개 상용 서비스 백엔드.
- Sources: https://relayd.dev, https://relayd.dev/manifest.webmanifest, https://news.ycombinator.com/item?id=46678461 (Show HN)
- Confidence: high

### CodeNomad
- URL: https://github.com/NeuralNomadsAI/CodeNomad
- Kind: agent-native
- What: OpenCode V2 및 다중 에이전트 작업을 위한 데스크톱 및 브라우저 워크스페이스("AI Coding Cockpit")로, 복수 프로젝트, 중첩 세션 트리, Git worktree, MCP 도구 관리 및 OpenTunnel을 통한 모바일 원격 제어를 통합 지원한다.
- Agent integration: OpenCode V2 백그라운드 서비스 공식 직접 연결; Claude Code 호환성 및 터미널 클라이언트 연동; 세션 타임라인 브랜칭/포킹; MCP 커넥션 관리자 및 플러그인 동적 활성화; 에이전트 주도 브라우저 프리뷰(Electron/Tauri); Git worktree 변경 내역 실시간 diff 뷰어.
- Mobile: responsive web & remote control (로컬 CodeNomad 서버의 반응형 HTTPS 웹 UI 접속 및 OpenTunnel 릴레이를 통한 포트 개방 없는 모바일 폰 1:1 종단간 암호화 페어링 지원; 네이티브 앱스토어 앱은 없음, cite: 공식 README Remote Access 섹션).
- Pricing: 100% 무료 오픈소스 (MIT License).
- Self-host: yes (Electron 및 Tauri 기반 데스크톱 앱(macOS, Windows, Linux) 또는 Node.js 기반 `npx @neuralnomads/codenomad` 서버로 완전 셀프호스팅 가능).
- License: MIT
- Activity: GitHub 2,625 stars, 마지막 커밋 2026-10-16, v0.20 활성 릴리스.
- Pros: 2,600스타 이상의 높은 인기와 뛰어난 완성도; Electron과 Tauri 빌드를 모두 제공하는 유연한 아키텍처; 포트 포워딩 없이 스마트폰을 연결하는 OpenTunnel 원격 페어링; MCP 도구 및 플러그인 관리자 완벽 통합.
- Cons: OpenCode V2 백엔드에 강하게 최적화되어 타 에이전트 단독 구동 시 제약; 열 기반 칸반 보드 레이아웃보다는 다중 세션 탭 및 타임라인 계층 구조 중심; 네이티브 스토어 모바일 앱 미제공.
- Sources: https://github.com/NeuralNomadsAI/CodeNomad, https://neuralnomadsai.github.io/CodeNomad/
- Confidence: high

### AgentsMesh
- URL: https://agentsmesh.ai
- Kind: agent-native
- What: 한 명의 오퍼레이터가 사설 인프라 머신 전반에 걸쳐 100개 이상의 AI 코딩 에이전트를 스케줄링하고 격리 실행할 수 있도록 지원하는 엔터프라이즈급 인력 플랫폼으로, 칸반 티켓 보드, 실시간 터미널 스트리밍, 자율 치유(Autopilot) 및 네이티브 iOS 앱을 제공한다.
- Agent integration: 모든 터미널 기반 CLI 에이전트 수용(Claude Code, OpenAI Codex CLI, Gemini CLI, Aider, OpenCode, 자체 에이전트); AgentPod 단위의 독립된 PTY 터미널 및 Git worktree 격리 샌드박스; 칸반 티켓과 에이전트 파드 및 PR/MR 실시간 연계; gRPC+mTLS 기반의 안전한 제어 평면과 분산 릴레이 클러스터; 에이전트 간 협업을 위한 채널 및 멘션 토폴로지.
- Mobile: native iOS app (SwiftUI + The Composable Architecture 기반 공식 네이티브 iOS 앱 제공, UniFFI 바인딩을 통해 데스크톱/웹과 동일한 Rust 코어 dylib 공유; Next.js 기반 모바일 웹 콘솔 병행 지원, cite: 공식 아키텍처 및 README 클라이언트 계층).
- Pricing: 오픈소스 커뮤니티 에디션 무료 (BSL-1.1, BYOK API 키); 호스팅 클라우드는 Free 티어($0), Team 티어(사용자당 월 $14 또는 파드 기반), Enterprise 티어로 구성 (2026년 10월 기준, cite: agentsmesh.ai/pricing).
- Self-host: yes (Docker Compose `./dev.sh`로 PostgreSQL, Redis, MinIO, Backend, Relay, Next.js 전체 스택 1분 배포 가능, 각 워커 머신에는 `agentsmesh-runner` 데몬 설치).
- License: BSL-1.1 (Business Source License 1.1, 향후 오픈소스 전환).
- Activity: GitHub 2,361 stars, 마지막 커밋 2026-09-23, 엔터프라이즈 활성 릴리스.
- Pros: 유일하게 네이티브 iOS 앱(SwiftUI)을 Rust 공용 코어로 직접 제공하여 모바일에서 터미널 및 파드 완벽 제어; 100개 이상의 대규모 분산 에이전트 실행에 특화된 gRPC+mTLS 엔터프라이즈 아키텍처; 칸반 티켓 보드가 에이전트 파드 및 PR과 직접 양방향 바인딩; 자율 에러 치유(Autopilot) 메커니즘.
- Cons: BSL-1.1 제한적 소스 라이선스; Go 백엔드, Rust 코어, Relay, Redis, Postgres 등 설치 및 운용 인프라가 상대적으로 무거움; 개별 개발자의 로컬 단독 사용에는 다소 과도한 엔터프라이즈 스펙.
- Sources: https://agentsmesh.ai, https://agentsmesh.ai/docs, https://agentsmesh.ai/pricing, https://github.com/AgentsMesh/AgentsMesh, https://news.ycombinator.com/item?id=47249381 (Show HN)
- Confidence: high

### Cairn
- URL: https://try-cairn.com
- Kind: agent-native
- What: GitHub 저장소와 연동하여 백그라운드에서 엔드-투-엔드 소프트웨어 엔지니어링 작업을 자동 수행하는 오픈소스 프로젝트 관리/칸반 스타일의 백그라운드 에이전트 시스템.
- Agent integration: Anthropic Claude(Claude 3.5/3.7/4), OpenAI GPT-4o/4.1, Google Gemini 등 다중 LLM API 직접 연동; GitHub App 양방향 연동을 통한 브랜치 생성, 코드 편집, 커밋 및 PR 자동 제출; 제품 관리 툴 스타일의 직관적인 칸반 작업 뷰.
- Mobile: responsive web (FastAPI 백엔드와 React 프론트엔드로 구성된 모바일 반응형 웹 UI 지원; 네이티브 모바일 앱은 없음, cite: try-cairn.com 및 공식 아키텍처).
- Pricing: 100% 무료 오픈소스 (MIT License, BYO LLM API 키).
- Self-host: yes (Python FastAPI 백엔드 + Node.js React 프론트엔드, `./run.sh` 스크립트로 로컬 머신 또는 사설 서버에서 완전 자가 호스팅).
- License: MIT
- Activity: GitHub 220 stars, 마지막 커밋 2025-07-02, 2025년 6월 Show HN 발표("Kanban-Board agent management for software dev agents, open source").
- Pros: 상용 백그라운드 에이전트(Codex, Cursor Background Agents)의 초기 오픈소스 대안; GitHub App 자체 제어로 사설 저장소에 안전하게 접근; 직관적인 칸반 보드 스타일의 인터페이스; Claude, OpenAI, Gemini 동시 지원.
- Cons: 2025년 중반 이후 업데이트가 다소 정체됨; 최신 Git worktree 다중 격리 도구들에 비해 기능이 단순함; 전용 모바일 앱 미제공.
- Sources: https://try-cairn.com, https://github.com/cairn-dev/cairn, https://news.ycombinator.com/item?id=44415849 (Show HN)
- Confidence: high

---

## EXPAND
- LEAD: cmux (https://cmux.com) — WHY: 다중 코딩 에이전트 병렬 멀티태스킹을 위해 설계된 현대적 터미널이자 모바일 동기화 클라이언트를 제공하여 터미널과 스마트폰 간의 작업 현황 연속성을 지원함 — ANGLE: cmux coding agent mobile companion dashboard
- LEAD: Greenlight (Show HN: Greenlight – Manage your AI coding agents from your phone, HN item 47334316) — WHY: 코딩 에이전트의 작업 현황 모니터링 및 승인 권한 행사를 스마트폰 모바일에서 직접 수행하도록 설계된 모바일 특화 제어 도구 — ANGLE: Greenlight AI coding agents phone mobile dashboard
- LEAD: QodFlow (https://www.qodflow.com) — WHY: MCP(Model Context Protocol)를 통해 AI 에이전트가 직접 칸반 보드를 조작하고 태스크를 운용하도록 설계된 최신 에이전트 구동형 칸반 보드 — ANGLE: QodFlow kanban board MCP server agent
- LEAD: Paseo (https://github.com/getpaseo/paseo, https://paseo.sh) — WHY: 데스크톱, 모바일, CLI 전반에서 다중 코딩 에이전트를 오케스트레이션할 수 있도록 설계된 신흥 오픈소스 에이전트 인터페이스 — ANGLE: Paseo open source coding agent interface mobile desktop

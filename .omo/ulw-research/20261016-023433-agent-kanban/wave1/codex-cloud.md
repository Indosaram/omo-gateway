### OpenAI Codex Cloud Tasks Dashboard
- URL: https://chatgpt.com/codex
- Kind: agent-native
- What: OpenAI의 클라우드 기반 소프트웨어 엔지니어링 에이전트 작업 대시보드로, 클라우드 가상 환경 및 로컬 연결 머신에서 실행되는 코딩 태스크의 큐, 상태, Diff 변경 사항, 권한 승인, 실행 로그를 웹 및 모바일에서 모니터링하고 제어합니다.
- Agent integration: official Codex SDK (@openai/codex-sdk Node.js/TypeScript 라이브러리), Codex App Server (웹소켓/JSON-RPC 기반 세션·승인·스트리밍 이벤트 프로토콜; 구형 codex-mcp-server 바이너리는 폐기되고 App Server로 대체됨), Codex CLI (`codex`, `codex exec`, `/status`), GitHub Actions 및 Linear/Slack 연동, 엔터프라이즈 Compliance/Analytics REST API 지원. 외부 에이전트(Claude Code, Cursor 등)는 직접 내장 대시보드 뷰어 API가 없으나 Codex App Server 및 SDK/CLI 이벤트 파이프라인 또는 GitHub PR/Linear를 매개로 상태 연계 가능.
- Mobile: native iOS app (ChatGPT for iOS 앱 내 'Codex' 전용 탭/화면 내장; Live Activities 백그라운드 진행 표시, 홈 화면/잠금 화면 위젯 지원), native Android app (ChatGPT for Android 앱 내 Remote Control 및 Codex 작업 지원), responsive web (chatgpt.com/codex 브라우저 반응형 대시보드). (출처: learn.chatgpt.com/docs/mobile, learn.chatgpt.com/docs/changelog 2026-10)
- Pricing: ChatGPT 계정 티어 기반 과금 (2026-10-16 기준). 개인용: Free (월 ₩0, 로컬/기본 한정 접근, Cloud 미포함), Go (월 ₩13,000, Cloud 미포함), Plus (월 ₩29,000, Codex Cloud 포함/확장 한도), Pro (월 ₩159,000부터 3개 사용량 티어, GPT-6 Astra 및 장기 세션 지원). 비즈니스/엔터프라이즈: Business (월 ₩28,900/유저, 연간 결제 기준, 월 결제 시 ₩36,100), Enterprise (맞춤형 크레딧/토큰 기반 종량제 종량 요금제). 표준 Codex Cloud 환경은 출시 초기 별도 VM 인프라 추가 요금 없이 플랜 모델 사용량 한도 내에서 차감.
- Self-host: no (Codex Cloud 작업 실행 인프라 및 대시보드 호스팅은 OpenAI 매니지드 클라우드 전용; 다만 로컬 에이전트 실행기인 Codex CLI 및 Codex App Server는 로컬 머신에서 바이너리/Node/Docker로 자체 구동 가능하며 데스크톱 Remote 연결로 제어 가능).
- License: proprietary (클라우드 대시보드, Codex Web, ChatGPT 모바일 앱, 백엔드 서비스) / open-source Apache-2.0 (로컬 터미널 에이전트인 `openai/codex` CLI 및 앱 서버 코어).
- Activity: GitHub `openai/codex` 129,417 stars, 최근 릴리스 `rust-v0.163.0` (2026-10-15 배포); 클라우드 및 모바일 서비스는 활성 정식 운영(Active GA/Rollout) 중.
- Pros: PC가 절전 모드이거나 오프라인이어도 클라우드 컨테이너에서 독립적으로 백그라운드 코딩 태스크 완주 가능; iOS/Android 네이티브 앱 및 위젯/Live Activities를 통해 이동 중에도 코드 Diff 검토와 터미널 실행 권한 승인이 매우 원활함; GitHub 저장소 및 환경 사전 구성을 클라우드 환경 단위로 템플릿화하여 팀 단위로 공유 가능; 로컬 PC(Mac/Windows/WSL)와 클라우드 인스턴스를 단일 대시보드 인터페이스에서 통합 조회 가능.
- Cons: 순수 칸반 보드 형태(커스텀 컬럼 드래그앤드롭 등)보다는 호스트/작업별 목록 및 상태 필터(Tasks/Approve/Review/Pinned) 중심의 에이전트 콘솔 UI에 가까움; OpenAI ChatGPT 생태계(Plus/Pro/Team/Enterprise) 구독 계정에 종속되어 무료/저가 티어에서는 클라우드 기능 사용 불가; 자체 호스팅 클라우드 대시보드 구축이 불가능한 완전 독점(Proprietary) SaaS 서비스임; 타사 범용 에이전트(Claude Code, Cursor 등)가 임의로 상태 카드를 푸시할 수 있는 공개 칸반 REST 엔드포인트가 제한적임.
- Sources: https://chatgpt.com/codex, https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan, https://chatgpt.com/pricing/, https://learn.chatgpt.com/docs/cloud, https://learn.chatgpt.com/docs/mobile, https://learn.chatgpt.com/docs/codex-sdk, https://learn.chatgpt.com/docs/codex/cli, https://learn.chatgpt.com/docs/changelog, https://github.com/openai/codex
- Confidence: high (공식 사이트, 최신 2026-10 문서, 요금제 페이지 및 GitHub 저장소를 직접 렌더링·검증 완료)

## EXPAND
- LEAD: Codex App Server WebSocket Protocol vs Legacy MCP Server — WHY: OpenAI가 `codex mcp-server`를 폐기하고 `Codex App Server` JSON-RPC/WebSocket 아키텍처로 전면 개편하여 타사 도구 연동 인터페이스 규격이 변경됨 — ANGLE: https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server

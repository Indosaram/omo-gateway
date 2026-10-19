### CodeAgentSwarm
- URL: https://www.codeagentswarm.com
- Kind: agent-native
- What: 여러 코딩 에이전트를 병렬로 돌리는 데스크톱 ADE. 에이전트가 읽고 갱신·완료하는 'Agentic Task Board'(칸반형 작업 보드)를 내장한다.
- Agent integration: 병렬 코딩 에이전트를 지원하며 v2.4.0에서 Muse Code, Pi, Devin CLI 지원이 추가됐다. 에이전트가 보드의 작업을 직접 읽고 갱신한다.
- Mobile: 없음(macOS 12+/Windows 10/11/Linux 데스크톱)
- Pricing: 무료 다운로드, 오픈 베타 기간에는 Pro 기능 포함. 정식 Pro 가격은 미확인.
- Self-host: 로컬 데스크톱 앱
- License: 상용(오픈소스 표기 없음, 미확인)
- Activity: 오픈 베타, v2.4.0
- Pros: 에이전트가 직접 쓰는 작업 보드; 실시간 diff·대화 기록 검색; 3개 OS 지원
- Cons: 모바일 없음; 베타 이후 가격 불명; 소스 비공개
- Sources: https://www.codeagentswarm.com/en
- Confidence: medium

### Hamster (tryhamster.com, Task Master 클라우드)
- URL: https://tryhamster.com
- Kind: general-kanban-with-agent-integration
- What: 목표를 전략, 이니셔티브, 브리프로 쪼개 전달까지 이끄는 팀용 AI 기획 플랫폼. Task Master의 클라우드 대응 제품이다.
- Agent integration: 사용자의 에이전트와 연동된다(공식 홈 'Works with your agents'). Task Master MCP/CLI와 연결된다.
- Mobile: 미확인(웹 앱)
- Pricing: 무료로 월 10 브리프까지(카드 불필요), 유료 플랜은 미확인
- Self-host: 불가(SaaS)
- License: 상용
- Activity: 활발
- Pros: Task Master와 연동; 목표부터 이니셔티브까지 이어지는 구조화; 무료 시작 가능
- Cons: 유료 가격 불명; 셀프호스팅 불가; 순수 칸반 도구는 아님
- Sources: https://tryhamster.com
- Confidence: medium

### OpenTerra (KaiStephens/OpenTerra)
- URL: https://github.com/KaiStephens/OpenTerra
- Kind: agent-native
- What: Terragon(백그라운드 Claude Code 에이전트)의 오픈소스 버전.
- Agent integration: Claude Code 에이전트
- Mobile: 미확인
- Pricing: 무료(오픈소스)
- Self-host: 가능(소스 실행)
- License: 미확인(GitHub 라이선스 표기 없음)
- Activity: GitHub 스타 5, 마지막 push 2025-07-17(사실상 중단)
- Pros: Terragon 대안으로 셀프호스팅 가능; 무료
- Cons: 개발 중단 상태; 커뮤니티가 매우 작음; 문서 부족
- Sources: https://github.com/KaiStephens/OpenTerra
- Confidence: low

### Claude Code on the web
- URL: https://code.claude.com/docs/en/claude-code-on-the-web
- Kind: agent-native
- What: Anthropic이 운영하는 클라우드 샌드박스에서 Claude Code 작업을 비동기로 실행하고 웹에서 세션 목록과 상태를 보는 공식 기능.
- Agent integration: Claude Code(공식), GitHub 저장소 연동
- Mobile: Claude 모바일 앱과 웹에서 확인 가능(공식 문서 'Claude Code in the cloud' 섹션; 모바일 세부 내용은 노드 보고서와 교차확인 필요)
- Pricing: Claude 유료 구독(Pro/Max/Team/Enterprise)에 포함
- Self-host: 불가(Anthropic 관리형)
- License: 상용
- Activity: 활발(공식)
- Pros: 별도 설치 없이 클라우드에서 병렬 실행; 공식 지원; 구독에 포함
- Cons: 칸반 보드가 아니라 세션 목록; Claude 전용; 셀프호스팅 불가
- Sources: https://code.claude.com/docs/en/claude-code-on-the-web
- Confidence: medium

### Groma.md (MrLesk/Groma.md)
- URL: https://github.com/MrLesk/Groma.md
- Kind: general-kanban-with-agent-integration
- What: 아키텍처를 Git 속 Markdown(OKF)과 C4 맵으로 관리하는 도구. Backlog.md 작업을 아키텍처 맵 위에 투영한다(칸반 자체가 아니라 보조 도구).
- Agent integration: 에이전트가 소스를 스캔하고 큐레이션한다. Backlog.md와 연동된다.
- Mobile: 없음
- Pricing: 무료(오픈소스)
- Self-host: 로컬(Git 저장소)
- License: MIT (GitHub API)
- Activity: GitHub 271스타, 마지막 push 2026-10-16
- Pros: Backlog.md와 연계; Git 기반이라 에이전트 친화적; 무료
- Cons: 칸반이 아니라 아키텍처 맵; 모바일 없음; 신생 프로젝트
- Sources: https://github.com/MrLesk/Groma.md
- Confidence: medium

## EXPAND
none — 리드 직접 조사

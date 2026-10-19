### Kiro (AWS)
- URL: https://kiro.dev
- Kind: agent-native
- What: Amazon Web Services(AWS)가 개발한 AI 코딩 IDE 및 에이전트 개발 환경으로, 단순 프롬프트 기반 코드 작성을 넘어 명세(Requirements/Design/Tasks) 기반 개발(Spec-driven development)과 이벤트 기반 에이전트 훅(Agent Hooks)을 핵심 워크플로우로 제공합니다.
- Agent integration: 자체 Agent Client Protocol(ACP) 및 Model Context Protocol(MCP) 서버/클라이언트를 완벽 지원하며, IDE 전용 GUI Specs/Tasks 패널, Kiro CLI V3(`kiro-cli`, `curl -fsSL https://cli.kiro.dev/install | bash`), REST/GraphQL 클라우드 세션 연동, Web UI, 모바일 앱을 지원합니다. ACP 규격을 통해 Claude Code, Codex, Cursor, Zed, JetBrains 등 외부 호환 클라이언트와 통합 가능하며, 저장소 내 `.kiro/specs/<feature>/tasks.md` 및 `.kiro/hooks/*.json`을 통해 파일/git/세션 수명주기 훅(File Save/Create/Delete, Prompt Submit, Agent Stop, Pre/Post Task Execution, Pre/Post Tool Use)을 트리거합니다.
- Mobile: native iOS app(Apple TestFlight 프리뷰 제공, iOS 26+ 요구 및 Kiro Pro/Pro+/Pro Max/Power 유료 계정 대상, 클라우드 샌드박스 세션 원격 제어 및 diff 검토 지원) / Android 전용 네이티브 앱은 미출시이며 웹 반응형 인터페이스(Web session)로 접근 가능 (참조: https://kiro.dev/mobile/, https://kiro.dev/docs/mobile/)
- Pricing: 무료 플랜(Kiro Free) 영구 50 크레딧 제공(Claude Sonnet 4.5 및 오픈웨이트 모델 접근); Kiro Pro 월 $20(1,000 크레딧, 프리미엄 모델 및 초과 크레딧당 $0.04 구매 가능); Kiro Pro+ 월 $40(2,000 크레딧); Kiro Pro Max 월 $100(5,000 크레딧); Kiro Power 월 $200(10,000 크레딧); 소셜 로그인 또는 AWS Builder ID 첫 유료 전환 시 $20 가입 보너스 크레딧 제공; 엔터프라이즈 플랜 별도 문의 (2026-10-16 기준 확인)
- Self-host: no (부분 로컬 실행; IDE 및 CLI 클라이언트는 로컬 머신에서 구동되고 로컬 파일시스템 `.kiro/`을 다루지만, 추론 엔진·세션 관리·크레딧 시스템은 AWS Bedrock/Kiro 클라우드 서비스에 종속되어 완전 온프레미스 자체 호스팅 불가)
- License: proprietary (Kiro IDE 및 CLI는 "AWS Content"로 라이선스 부여되며 AWS Customer Agreement 및 AWS Intellectual Property License 적용; Chromium 엔진 기반 일부 오픈소스 라이선스 혼합)
- Activity: 공식 클라이언트는 비공개 소스/바이너리 배포이며 커뮤니티 에코시스템 활발(커뮤니티 `grg-lab/kiro-specs-dashboard`, `requix/kiro-team`, `aws-samples/sample-ai-powered-sdlc-patterns-with-aws` 등 활성 개발 중, CLI V3.x 릴리스 진행 중)
- Pros: 체계적인 명세 기반 개발(`.kiro/specs/` 내 requirements.md, design.md, tasks.md 분할 관리 및 진행률 추적); 이벤트 기반 에이전트 훅(Pre/Post Task, File Save, Tool Use 등 다양한 트리거에 CLI 명령 및 서브에이전트 프롬프트 자동 연계); 멀티 플랫폼 생태계(IDE, CLI V3, Web, iOS TestFlight 네이티브 앱); ACP 및 MCP 양방향 프로토콜 지원으로 외부 도구 확장성 탁월; AWS 인프라(AWS Builder ID, IAM Identity Center, GovCloud)와의 깊은 엔터프라이즈 통합
- Cons: 핵심 에이전트 하네스 및 백엔드가 AWS 클라우드 SaaS에 종속되어 완전한 자체 호스팅(에어갭 환경) 불가; 복잡한 프롬프트 및 태스크 실행 시 크레딧 차감 속도가 빠르고 모델별 크레딧 멀티플라이어 적용으로 과금 예측에 주의 필요; Android 네이티브 앱 부재(iOS 위주 모바일 선공개); 순수 칸반 보드(열 기반 드래그앤드롭) 형태보다는 체크리스트/태스크 트리 중심 UI에 가까워 전통적 PM 보드로 쓰려면 커뮤니티 확장(Dashboard) 필요
- Sources: https://kiro.dev, https://kiro.dev/pricing/, https://kiro.dev/mobile/, https://kiro.dev/docs/specs/, https://kiro.dev/docs/hooks/, https://kiro.dev/docs/hooks/types/, https://kiro.dev/docs/acp/, https://kiro.dev/license/, https://github.com/grg-lab/kiro-specs-dashboard
- Confidence: high (공식 웹사이트, 가격표, 기술 문서 Next.js 페이로드 파싱, 라이선스 전문, 모바일 랜딩 페이지 및 깃허브 커뮤니티 저장소를 교차 검증함)

## EXPAND
- LEAD: grg-lab/kiro-specs-dashboard — WHY: Kiro의 .kiro/specs 파일들을 IDE 내에서 칸반/카드형 대시보드 및 팀 속도 메트릭 차트로 시각화해 주는 독립 커뮤니티 확장도구 — ANGLE: https://github.com/grg-lab/kiro-specs-dashboard

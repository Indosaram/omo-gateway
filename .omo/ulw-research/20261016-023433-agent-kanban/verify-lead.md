# Lead 직접 검증 (성공 기준 2)
## Omnara — 2026-10-16 확인
- 1차 출처(omnara.com/pricing): 플랫폼 요금 $0, 셀프호스팅 무료, Apache-2.0 (github.com/omnara-ai/omnara), Omnara 관리형 모델·머신만 종량제($0.0414/GiB-hour), 엔터프라이즈 문의.
- 모바일: App Store id6748426727, Google Play com.omnara.app (2026-04-09 업데이트).
- 판정: HN(2025-08) '10세션 무료/$9' 및 codeagent-mobile '$19.99/mo'는 과거 요금 → 현재는 무료+종량제. 포지셔닝이 'Claude Managed Agents 오픈소스 대안'으로 바뀜.
## Vibe Kanban — 2026-10-16 확인
- vibekanban.com: 'Vibe Kanban is sunsetting' 배너, 가격 페이지 Free $0 / $30 user/mo(2-49).
- nimbalyst.com/blog/vibe-kanban-after-bloop-whats-next: Bloop 2026-04-10 종료 발표, OSS 지속.
- github.com/BloopAI/vibe-kanban: 셀프호스팅 가이드 존재.
- 판정: 로컬 OSS(Apache-2.0 여부는 wave1 보고서로 교차확인) 사용 가능, 클라우드 기능은 셀프호스팅 필요, 유지보수 리스크 높음.
- README(raw.githubusercontent.com/BloopAI/vibe-kanban/main/README.md): 'Vibe Kanban is sunsetting', npx vibe-kanban 로컬 실행, Docker 셀프호스팅 가이드(vibekanban.com/docs/self-hosting/deploy-docker), 에이전트 10+종 지원, 내장 MCP 서버(MCP_HOST/MCP_PORT).
- GitHub API: license=Apache-2.0, stars=28376, pushed_at=2026-09-19T06:52:48Z, archived=false.

## Conductor — 2026-10-16 확인 (conductor.build/pricing)
- Free $0(로컬 Mac 워크스페이스, 병렬 에이전트, BYO 키) / Pro $50/월(클라우드, 멀티플레이어 5명, API, 모바일 앱) / Teams $60/사용자/월 / Enterprise 문의.
- 모바일 앱은 Pro 이상 전용. 셀프호스팅 클라우드는 미제공(엔터프라이즈용 개발 중). SOC 2 Type II.
- 판정: wave1 보고서 conductor.md와 일치 → CONFIRMED.
## Happy — 2026-10-16 확인
- happy.engineering/docs/faq: iPhone/iPad App Store 무료, 100% 오픈소스. Google Play com.ex3ndr.happy(2026-10-03 갱신). App Store id6748571505.
- github.com/slopus/happy: MIT(GitHub API), stars 24166, push 2026-10-15. E2E 암호화, iOS/Android/웹.
- 판정: 무료·OSS·iOS/Android 네이티브 CONFIRMED. Claude/Codex/Grok 지원. 칸반 뷰는 아님(세션 목록형).

## Linear — 2026-10-16 (linear.app/pricing)
- Free $0(무제한 멤버, 2팀, 이슈 250개, Agent platform/Linear Agent 포함) / Basic $10/사용자/월 / Business $16/사용자/월 / Enterprise 문의. 셀프호스팅 불가(SaaS).
## Backlog.md — 2026-10-16 (README)
- Git 저장소 Markdown 파일 기반 칸반, 'backlog board' TUI + 'backlog browser' 로컬 웹 칸반(드래그앤드롭), MCP/CLI 지원, npm i -g backlog, MIT(GitHub API). 모바일 앱 없음(로컬 웹을 모바일 브라우저로 접근 가능 수준).
## Task Master — 2026-10-16 (README)
- MCP 서버(claude mcp add taskmaster-ai -- npx -y task-master-ai) + CLI, Cursor 원클릭 설치. 라이선스 GitHub API NOASSERTION(커스텀: MIT with Commons Clause로 알려짐 → 보고서 값 확인 필요). 자체 칸반 UI 없음(Hamster 연동).
## Orca — 2026-10-16 (yetanotherorchestrator.app/apps/orca)
- 무료·MIT 오픈소스 ADE, macOS/Windows/Linux, iOS·Android 컴패니언 앱, Git worktree 격리, 원격 서버 지원.

## GitHub Copilot cloud agent — 2026-10-16 (github.com/features/copilot/plans, docs.github.com copilot-on-github)
- 요금: Free $0(제한적 에이전트 사용, CLI·agent mode) / Pro $10/월 / Pro+ $39/월 / Max $100/월 (+ 비즈니스 플랜 별도). 문서 목차에 'Use cloud agent on GitHub Mobile' 존재 → GitHub Mobile(iOS/Android)로 에이전트 작업 지시·확인 가능. 셀프호스팅: GitHub Enterprise Server 별도(클라우드 에이전트는 GitHub.com 기준).
## Cursor (cloud/background agents) — 2026-10-16 (cursor.com/pricing, cursor.com/docs)
- 주의: 2026-10 cursor.com/pricing은 xAI 통합 후 Grok 번들 요금표(Free $0 / Plus $8 / Premium $30 / Super $100 / Ultra $200 / Teams $30+/사용자). 문서 목차에 'Mobile', 'Self-Hosted Machines', 'API', 'CLI' 존재 → 모바일 웹/앱에서 에이전트 확인 가능, 에이전트 실행 머신 셀프호스팅 가능. (요금 구조가 바뀐 상태라 보고서 값과 교차확인 필요)
## OpenAI Codex cloud — 2026-10-16 (developers.openai.com/codex/cloud → learn.chatgpt.com/docs/cloud)
- ChatGPT 웹·모바일 앱·데스크톱 앱에서 Codex 클라우드 작업 확인·리뷰 가능('On mobile, open Codex'). ChatGPT 요금제에 포함. 셀프호스팅 불가(Codex CLI는 OSS 로컬).

## 범용 칸반 공식 페이지 직접 수집 — 2026-10-16 (원문: verify-general-raw.txt)
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

## 보고서 대조 (wave1 vs lead 직접 확인)
- vibe-kanban.md: Apache-2.0, 28,376 stars, sunsetting, Docker 셀프호스팅 → lead 확인(GitHub API·README)과 일치 → CONFIRMED. 모바일 앱 없음(반응형 웹/원격 접속).
- cursor-bg-agents.md: Grok 번들 요금표(Free/Plus $8/Premium $30/Super $100/Ultra $200/Teams $30+)·Self-Hosted Machines → lead가 cursor.com/pricing·docs 목차에서 본 내용과 일치 → CONFIRMED. 보고서 추가 사실: iOS 앱(id6767085653), Android는 PWA, Cloud Agents는 Premium 이상.
- omnara.md: 일치(앞 기록).

## 대조: 새 보고서 vs lead 직접 확인
- github-agent-hq.md: Pro $10 / Pro+ $39 / Max $99~100(연간) — lead가 github.com/features/copilot/plans에서 본 Free/Pro $10/Pro+ $39/Max $100과 일치(Max 표기 $99 vs $100 차이는 연간·월간 표기 차이). GitHub Mobile 'Copilot agent sessions' — lead가 docs 목차의 'Use cloud agent on GitHub Mobile'에서 확인한 내용과 일치 → CONFIRMED.
- devin.md 내 Linear 가격(Standard $8/$10, Plus $14/$16) vs lead 직접 확인(linear.app/pricing: Basic $10, Business $16 /사용자/월) → 상충: 플랜 이름이 다름(보고서는 연간가와 월간가를 함께 적음). 문서에는 lead 1차 출처 값(Free / Basic $10 / Business $16)을 쓴다.

## 추천 신규 도구 직접 확인 — 2026-10-16 (README·GitHub API)
- Multica: 칸반 보드(에이전트가 컬럼 이동), Docker Compose·Helm 셀프호스팅 확인. 모바일은 iOS·iPadOS 앱을 소스로 직접 빌드해야 하며 App Store에는 없음, Android 언급 없음 → 노드 보고서의 'iOS·Android 앱'은 과장이라 정정. ★52,949, 라이선스 NOASSERTION(커스텀).
- AgentsMesh: BSL-1.1, Docker, Web/Desktop/iOS(SwiftUI) 클라이언트 확인. ★2,361.
- Agent Orchestrator: Apache-2.0, ★13,217, 2026-10-16 push.
- Paca: Apache-2.0, Docker Compose·K8s/Helm 셀프호스팅, 무료 확인. ★1,928.

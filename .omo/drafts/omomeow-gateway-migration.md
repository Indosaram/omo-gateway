---
slug: omomeow-gateway-migration
status: drafting
intent: clear
review_required: true
plan_path: .omo/plans/omomeow-gateway-migration.md
plan_sha256: null
review_round_id: null
review_round_limit: 5
pending-action: write and review .omo/plans/omomeow-gateway-migration.md
review:
  plan_reviewer:
    status: approved (OKAY, round 1, 2026-10-07 — st_01a116e8, mahoquot/gemini-3.8-flash-high, 18.5s)
    summary: "affected user 명확, 5개 IS 행 → todos/QA 1:1 매핑, 참조 코드 경로 유효, 차단 모순 없음"
approach: 병렬 읽기전용 탐색(explore×2: gateway 구현면·omomeow 인벤토리) + architect/ultrabrain 자문 레인 → 토폴로지 락 → 오너 포크 배치 질문 → 승인 → .omo/plans/omomeow-gateway-migration.md 작성 → plan-reviewer 고정밀 리뷰
---

# Draft: omomeow-gateway-migration

## Affected user and ideal state
- IS-1 | 소유자(indosaram): Discord #work에서 오모냥에 말하면 수 초 내 응답 | 이유: 대화별 세션 분리로 컨텍스트 소형 유지(오늘: 단일 307K 메가세션, 50~90초 지연)
- IS-2 | 소유자: 크론·모니터 이벤트가 채팅 응답을 절대 막지 않음 | 이유: 대화/크론 레인 분리(오늘: 단일 세션 직렬 큐)
- IS-3 | 소유자: 봇이 조용히 죽지 않고, 전달 실패가 보고와 불일치하지 않음 | 이유: 턴 fail-fast + 전달검증 ack + 네이티브 SLA 감시(오늘: 72분 웨지 수동발견, UNANSWERED가 죽은 세션으로 발행)
- IS-4 | 소유자: 모든 작업 deepseek 단일 모델, 전역 omo 설정 무변경 | 이유: 2026-10-05/07 소유자 지시 (오늘: 전역 원복 완료, 오모냥 전용 설정으로 운영)
- IS-5 | herdr 작업 탭 계층의 병렬 실행 패턴 유지 | 이유: 오모냥의 강점(수십 개 동시 탭 실측) — 전환 중 기능 저하 금지
- GAP-1 | 307K 메가세션 → 대화별 소형 세션 | reason: IS-1 | closed by: 세션 수명주기 컴포넌트
- GAP-2 | watch.ts 이벤트 직렬 큐 점유 → 레인 분리 | reason: IS-2 | closed by: 스케줄/레인 컴포넌트
- GAP-3 | SLA/전달검증 게이트웨이 네이티브화 | reason: IS-3 | closed by: 전송계층+부가기능 컴포넌트
- GAP-4 | 오모냥 기능 전수 이관(버튼·음성전사·메일/캘린더·jobs 8종·work-map) — silent drop 금지 | reason: IS-5 | closed by: 인벤토리 매핑
- GAP-5 | 전환 중 봇 ID 이중 접속/토큰 충돌 방지 | reason: IS-5 | closed by: 컷오버 절차

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->
- C1 Discord 전송/수신 계층 — serenity 어댑터: 수신·스레드 수명주기·전달·전달검증 ack | pending-lock | explore:gw-surface
- C2 세션 수명주기·컨텍스트 격리 — 스레드↔세션 매핑, 컴팩션 정책 | pending-lock | explore:gw-surface
- C3 작업 위임 계층 — herdr 탭 유지 vs gateway-native (오너 포크) | pending-lock | 자문:architect
- C4 스케줄/크론 — watch.ts JOBS 8종 + legacy-crons → gateway cron | pending-lock | explore:omomeow-inventory
- C5 부가 인터랙션 — 버튼·음성전사·메일/캘린더·SLA 감시 | pending-lock | explore:omomeow-inventory
- C6 모델 라우팅·설정 스코프 — deepseek 강제, 전역 무변경 | pending-lock | 자문:ultrabrain
- C7 컷오버/롤백 — 봇 ID 1465631383862120451 단일 접속 전환 | pending-lock | 자문:architect

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->

## Findings (cited - path:lines)
- [세션 관측] gateway 미가동: 프로세스 없음, launchd 서비스 없음, 빌드 target/release/omo-gateway 9/29 (ps/launchctl 실측 2026-10-07)
- [세션 관측] WIP 4파일 미커밋: src/agent/omo_backend.rs, omo_config.rs, omo_protocol.rs, tests/test_omo_backend.rs (git status)
- [세션 관측] 오모냥 메인세션 컨텍스트 ~307K 토큰, 응답 50~90초 실측 (Discord 타임스탬프 13:48→13:49:46, 13:53:25→13:54:15 UTC)
- [세션 관측] 오모냥 세션 resume 시 저장모델(nekos) 우위, settings defaultModel 무시 → 기동 --model 플래그 필요 (10/7 프로브 실증)
- [메모리] gateway 설계: DM/채널/크론별 독립세션, 대화 300s fail-fast, cron 레인 분리, 전달검증 ack, SQLite (reference/omon-gateway.md)
- [메모리] 오모냥 체인: listen.ts→inbound.log→watch.ts→본체세션(herdr w8:p1), SLA/state/watch-state.json, work-map.json (reference/omomeow-unresponsive-diagnosis.md)
- [explore:omomeow-inventory 완료] 기능 매트릭스 확보 — 핵심 신규 사실:
  - JOBS 10종으로 증가(오늘 새벽 봇이 추가): notion-tasks, benchmark-maho, maho-weekly, content-intel-collect, vpn-mall-publish, playstore-publish, benchmark-ferryx, content-intel-publish-ready, ontology-sync(3h), kakao-calendar-hourly
  - 버튼: omon:btn:<ns>:<action>:<arg> / ns=content-intel(publish·preview·pass), kakao-cal(approve·reject), 설정 ~/.omon/button-actions.json, 3초 ack 계약(type 6 deferred)
  - 음성: ffmpeg→16kHz WAV→whisper-cli(ggml-large-v3-turbo), listen.ts:31-48
  - 자격증명: ~/.config/agent-messenger/discordbot-credentials.json bots[1465631383862120451].token + omomeow/.env DISCORD_BOT_TOKEN
  - 카카오: bin/kakao-hourly.ts(매시 :05)→MiMo 추출→Discord 제안 스레드+버튼, bin/kakao-calendar-action.ts
  - 상태 스키마: watch-state.json(due/mail/herdr/sla/progress), work-map.json(thread_id/tab_id/pane_id/session_id/status/last_progress_at)
  - 봇 토큰 이중화 주의: agent-messenger 크리덴셜과 .env 둘 다 존재 — 컷오버 시 단일 소스 확정 필요
- [explore:gw-surface 완료, 팩트시트 전문 수신] gateway 구현면:
  - 세션키: platform/guild/channel/thread/user/bot 6차원, 길드 내 user_id 정규화(공유 레인), DM은 유저별 분리 (models/session.rs:27-53)
  - 스레드 세션 분리 설정: DISCORD_THREAD_SESSIONS_PER_USER (discord/adapter.rs:419)
  - 세션 영속: SQLite sessions 표, state_json.metadata.omo_thread_id, thread/resume→thread/start 폴백 (agent/omo_backend.rs:242-325)
  - 크론: user_id="cron:<job.id>", 매 실행 fresh thread (omo_backend.rs:225-235) — 오모냥의 fresh-cron 관행과 동일
  - ★세션 모델 핀 네이티브: SessionState.active_model + 프로필 라우터(thread>channel>guild) + bot_profiles 표 오버라이드 (models/session.rs:270, multiplexer/profile_routing.rs:50-95) — 전역 설정 무관 오모냥 모델 라우팅 가능
  - 크론 스키마 HermesJob: deliver="discord:<ch>[:<thread>]" 팬아웃, ack_command(전달성공 후에만 실행), script/prompt, workdir, timeout_secs, failure_deliver, incident dedup (cron/store.rs:230-305, scheduler.rs:1545-1635)
  - 어댑터: 멘션 시 스레드 자동 생성(adapter.rs:983-999), 👀 처리 리액션→완료/실패 전환(942-1015, 3108-3135), typing 유지(2572-2605), 버튼 InteractionCreate→외부 스크립트 디스패처 OMON_BUTTON_ACTIONS_FILE(buttons.rs:83-125), 쓰로틀 스트리밍 편집(throttler.rs:180-250), 크론은 final-only, 에러 알림+incident dedup
  - ★버튼: gateway 버튼 디스패처가 오모냥 buttons.ts+button-actions.json과 거의 1:1 — 포트 비용 최소
  - 레인: interactive 19742 / cron 19743 분리, active_turns stale reclamation
  - env: OMON_DEFAULT_MODEL, OMON_OMO_* 타임아웃(1800s/600s), OMON_PER_AGENT_WORKSPACE(기본 true), OMON_BUTTON_ACTIONS_FILE
- [검증 정정] gw-surface의 "working tree clean" 주장은 거짓 — 직접 재검증: 4파일 M(+163/−32: omo_backend/config/protocol/test_omo_backend) vs HEAD ad5722c. 소유자 WIP일 수 있으므로 계획은 이 변경을 보존·분리 대상으로 명시 (dirty_worktree 리스크)
- [gw-surface: gateway 결측 기능] 음성 STT(voice/pipeline.rs 스텁만), 메일/캘린더 모니터, SLA 워치독, select menu/modal, 1회성 at-예약, 스레드 자동 아카이브 수명주기 — 이관 계획에서 명시 대응 필요

## Capability → Gateway mapping (초안, 자문 통합 전)
- 오모냥 수신/오너-타인 구분/reply-to → adapter 기본 수신 + 세션키 (길드 user 정규화). 오너 전용 필터(DISCORD_ALLOWED_USERS 대응) 확인 필요
- 👀 리액션 → DISCORD_PROCESSING_REACTIONS 네이티브 (완료/실패 전환 포함)
- 음성 전사 → 신규: 첨부 오디오 → ffmpeg+whisper-cli 셸아웃 훅 (voice/pipeline.rs 스텁 활용)
- reply-to 컨텍스트 → 스레드=세션 매핑으로 자연 해결
- 버튼(content-intel·kakao-cal) → OMON_BUTTON_ACTIONS_FILE 디스패처 + button-actions.json 포트
- JOBS 10종 → cron_jobs: script 6종(collect·vpn-mall·playstore·ferryx-bench·ontology·kakao-hourly) + agent 3종(notion·benchmark-maho·maho-weekly, 프롬프트=brief md) + publish-ready(script); deliver=discord:<#work 또는 스레드>; ack_command=producer ack 패턴
- SLA UNANSWERED → 신규 설계 필요: 어댑터 내부 무응답 추적 vs cron script 폴링 (ultrabrain 자문 대기)
- PROGRESS_DUE/herdr 감시 → herdr 유지 시: cron script가 herdr agent list 폴링·보고 / gateway-native 시 불필요
- 메일(zele)·캘린더(omocat)·govsupport·release → cron script jobs 직접 매핑
- work-map → gateway sessions 테이블+스레드로 대체 (herdr 유지 시 병행)
- 모델 라우팅(deepseek 전용) → OMON_DEFAULT_MODEL + bot_profiles (전역 설정 무관)
- 폴백 사슬(nekos→…→deepseek) → OMON_DEFAULT_MODEL=deepseek이면 불필요
- 봇 토큰/정체성 → gateway .env DISCORD_TOKEN = OmOMeow 토큰 단일 소스, 컷오버 시 listen.ts 루프 정지 선행
- 부팅 체인: ai.omomeow.herdr launchd → gateway launchd 서비스로 전환

## Decisions (with rationale)
- intent: clear, review_required: true (아키텍처 규모 기본 적용)
- 전역 omo 설정 변경 금지 확정(소유자 10/7 지시) — 계획상 모든 모델 라우팅은 gateway 자체 설정(env/.env/기동 플래그) 레인으로 설계. 검증: gateway 워크스페이스 프로비저닝은 에이전트 CWD의 .omo/omo.json에만 기록(글로벌 무손대), 슬러그 bot-1465631383862120451 네이티브 지원 (architect 자문)
- 렌더러: 배치(batch) — 콜드스타트 기본은 one-by-one이나 본 세션 소유자의 위임형 페이스에 맞춤 (거부 시 one-by-one 전환)
- [architect 자문 수신] 추천안: 방안 2 계층형 하이브리드 — gateway가 I/O·세션 멀티플렉싱·크론·Rust 레벨 SLA 감시(60s 턴 타이머 + 15분 하트비트, LLM 비의존 직접 Discord REST) 전담, 장시간 작업은 위임 하네스(herdr 탭/CLI delegate) 유지 + 영수증 파일. 31개 기능 커버리지: Moved 29 / Stayed 2 / Dropped 1(307K 고드세션 자체) — 무단 폐기 0. 4단계 컷오버: ①크론/알람 분리(D-3) ②섀도우 봇 검증(D-1) ③인바운드 컷오버(D-Day, listen.ts 정지→토큰 전환) ④위임 하네스 연동(D+1). 대안 1(네이티브 모놀리식, 비용 최상)·3(썬 게이트웨이, 현행 문제 온존) 기각 — 상세는 자문 보고서(스폰 기록 st_01a116bf)
- [작업 위임 해결 메커니즘(자문)] 비동기 위임 핸드오프: 대화 세션은 tool.delegate/CLI 위임 후 즉시 접수 응답(3~5초) 하고 IDLE 복귀, work_items 원장 등록, 15분 하트비트가 스레드에 직접 보고 — 대화 세션의 컨텍스트 비대화 원천 차단

## Scope IN

## Scope OUT (Must NOT have)

## Open questions
- F1 작업 실행 계층: (a) 하이브리드—herdr/위임 하네스 유지+gateway 접수·원장·하트비트 [추천] (b) gateway-native 워커 풀 신규 개발 (c) 현행 유지 → 소유자 질문 중
- F2 컷오버 방식: (a) 4단계 계층 전환 [추천] (b) 빅뱅 → 소유자 질문 중
- F3 섀도우 봇 토큰: (a) 테스트 서브 봇 신규 생성 [추천] (b) 섀도우 생략 직접 전환 (c) 소유자 계정 세션 테스트 → 소유자 질문 중

## Approval gate
status: approved (2026-10-07: F1=하이브리드, F2=4단계 계층, F3=섀도우 봇 생성, 플랜 작성 승인)
<!-- 포크 F1~F3 배치 질문 → 승인 후 플랜 작성 -->
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->

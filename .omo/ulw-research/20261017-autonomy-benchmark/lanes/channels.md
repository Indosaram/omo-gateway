# 멀티채널(Channels) 벤치마킹 및 도입 계획

## 판정

**채택 (Adopt Now)**

omon-gateway는 현재 단일 오너의 macOS 데스크톱 환경에서 운영되고 있으며, 양방향 대화 및 승인 채널이 Discord 단 하나에 강력하게 종속되어 있다. 오너가 이미 명시적으로 Slack 워크스페이스(`T04LYPT5VN1`, 일시불 프로그래밍) 연동을 요청했고 자격증명(`~/.omon/secrets/slack-*-token`)까지 생성된 상태이므로, Discord와 동일한 권한·스레드 스티키 세션·승인 UI를 지원하는 **Slack 양방향 어댑터의 도입은 즉시 채택(Adopt Now)**한다. 

반면, Instinct가 표방하는 SMS/iMessage/전화(Voice Call)/WhatsApp 기반의 양방향 확장은 **보류 및 제외**한다. 오너의 가치 기준(fail-fast, 불필요한 모델 폴링 금지, 머신 검증 가능한 결과, 야간 KST 0-8시 알림 억제)에 비추어 볼 때, 비공식 스크립트 기반의 iMessage/카카오톡 발신은 계정 정지 및 프로세스 행(Hang) 위험이 높고, SMS/전화는 높은 통신 비용과 스팸 노출 위험이 크다. 따라서 iMessage와 카카오는 현재처럼 신뢰도 높은 읽기 전용 인박스(Inbox) 수집 채널로 한정하고, 양방향 제어/승인은 안전한 소켓/웹훅 프로토콜을 지원하는 Discord와 Slack 2대 플랫폼 체제로 집약한다.

## 벤치마킹 원천

1. **Instinct**
   - **채널 구성**: "No new interfaces" 철학 아래 별도 전용 앱 없이 SMS, WhatsApp, iMessage, 전화 통화로만 접근 가능 `[1차]`, `[2차]` (Vellum, TechCrunch).
   - **한계 및 교훈**: 모든 작업과 알림이 단 하나의 연속된 단일 스레드(Single stream)로 쏟아져 사용자 피로도가 극에 달함. 또한 이메일/문자를 통한 프롬프트 인젝션 및 무승인 발신(Unapproved sends) 사고가 보고됨 `[2차]`.
   - **벤치마킹 포인트**: 단일 스트림을 지양하고, 작업/세션별 격리된 스레드(Thread)를 분리하여 대화 맥락을 보존하는 정책을 수립함.

2. **OpenAI Dots**
   - **채널 구성**: 공식적으로 ChatGPT, Slack, Teams 채널 지원 `[1차]` (OpenAI Help Center), `[미검증/2차]` (myclaw.ai).
   - **특징 및 아키텍처**: 채널 접근 권한과 도구(앱/인박스) 권한을 철저히 분리함. Slack에 봇을 초대했다고 해서 이메일이나 파일에 자동 접근하지 못하도록 경계를 둠. 반복 작업(Scheduled brief) 시 출처, 시간, 타임존, 수신 대상 채널을 명시적으로 바인딩하고 되묻는 스케줄 계약(Schedule contract)을 강제함 `[미검증/2차]`.
   - **벤치마킹 포인트**: 채널 라우팅 식별자(`SessionKey`, `HermesOrigin`)를 도구 실행 권한과 분리하고, 크론 작업의 목적지 채널을 사전에 검증된 안전한 채널 ID로 고정 바인딩.

3. **xAI Grok Bot**
   - **채널 구성**: Desktop, iOS, Slack, 이메일 연동 `[1차]`, `[2차]`.
   - **사고 사례**: CFO 봇이 이름이 같은 공개 Slack 채널에 개인 은행 보고서를 게시하는 심각한 오발송 사고 발생 `[2차]` (Business Insider). 이후 xAI는 채널 간 정보 이동 시 명시적 허가를 받도록 수정함.
   - **우선순위 제어**: 오너의 직접 메시지만 실행 중인 턴을 즉시 인터럽트(steer/stop)하고, 그룹/외부 채널 메시지는 큐 뒤에 대기시킴 `[1차]`.
   - **벤치마킹 포인트**: 채널명이나 표시명이 아닌 불변 고유 ID(Channel ID / User ID)로 목적지를 고정(fail-closed)하고, 오너의 명령만 우선적으로 인터럽트 가능하도록 통제.

4. **Hark / Meta Muse / Pickle**
   - **Hark**: iOS, Android, Web 전용 앱만 지원하며 모바일/웹 중심 `[1차]`.
   - **Meta Muse**: Muse 앱 및 WhatsApp, Mac 앱 지원 `[1차]`.
   - **Pickle**: iPhone 단일 앱 기반 음성/전화 및 화면 공유 `[1차]`.
   - **시사점**: 모바일 전용 앱이나 독자 GUI를 요구하는 방식은 macOS 데스크톱 상주형 단일 데몬인 omon-gateway 환경에 맞지 않으며, 오너가 이미 상주하는 Discord/Slack에 얹는 것이 최적임.

## 현재 코드

### 1. 양방향 채널 및 디스패처 구조
- `src/models/session.rs:30-48`: `SessionKey` 구조체는 `platform`, `guild_id`, `channel_id`, `thread_id`, `user_id`, `bot_id` 필드를 갖추어 플랫폼 중립적으로 설계되어 있음.
- `src/models/events.rs:346-390`: `OutboundAction` 열거형(`SendMessage`, `EditMessage`, `DeleteMessage`, `UploadFile`, `Stream`, `Typing`, `React`, `ApprovalRequest`, `ExpireApproval`)이 정의되어 있음.
- `src/multiplexer/actor.rs:85-103`: `OutboundDispatcher` 트레이트가 선언되어 있으며, `dispatch(&self, action: OutboundAction)`와 `owner_thread_is_private`를 규정함.
- `src/dashboard.rs:520-565`: `CompositeDispatcher`가 구현되어 있으나, `session.platform == "web"`인 경우만 웹 대시보드로 라우팅하고 그 외는 전부 단일 `primary` 디스패처로 전달함.
- `src/main.rs:1170-1185`: `primary` 디스패처로 `DiscordEgress` 단 하나만 인스턴스화되어 `SharedDispatcher` 및 `approval_requester`에 등록됨.

### 2. 크론 스케줄러의 Discord u64 채널 ID 하드코딩
- `src/cron/scheduler.rs:442-447`: `CronNotification` 구조체에 `pub channel_id: u64`로 정의되어 있어 숫자가 아닌 Slack 채널 ID(`C0C9Z1Z4G02` 등)를 담을 수 없음.
- `src/cron/scheduler.rs:2353-2357`: `destination.chat_id.parse::<u64>()`를 수행하여, 숫자로 파싱되지 않는 채널 ID가 들어오면 `OmonError::Config("invalid Discord channel ID: ...")` 에러를 내며 발송을 거부함.
- `src/cron/scheduler.rs:2378-2384`: 세션 키 생성 시 `SessionKey::new("discord", ...)`로 플랫폼이 하드코딩되어 있음.

### 3. Discord 어댑터의 u64 의존성
- `src/discord/adapter.rs:3336-3343`: `target(session)` 메서드에서 `session.thread_id` 또는 `session.channel_id`를 `u64`로 파싱하여 Serenity의 `ChannelId::new(id)`로 변환함. 숫자가 아니면 실패함.
- `src/discord/adapter.rs:3514-3580`: `DiscordEgress`가 `OutboundDispatcher`를 구현하고 있으며, Serenity HTTP 클라이언트만을 사용하여 메시지를 발송함.

### 4. 읽기 전용 인박스(Inbox) 수집기 현황
- `src/inbox/mod.rs:56`: `InboxSource`로 `imessage`, `kakao`, `mail`, `discord`가 등록되어 있음.
- `src/inbox/sources/imessage.rs:37-45`: `imsg` CLI를 통해 로컬 SQLite/메시지 데이터베이스에서 읽어오기만 하는 단방향 수집기임.
- `src/inbox/sources/kakao.rs`: PirateTalk 기반 단방향 수집기임.
- 인박스 수집기는 수신 전용이며, 송신(`OutboundDispatcher`) 경로는 전혀 존재하지 않음.

### 5. 이전 계획서(autonomous-agent-platform-plan.md) 내용과의 대조 및 정정
- **이전 계획서의 주장**: "G11: 양방향 채널이 Discord뿐 (src/dashboard.rs:557, src/cron/scheduler.rs:2357)" 및 "P5 멀티채널: `src/channels/`에 `send(dest, Envelope)` 트레이트 신설".
- **현재 코드와의 차이 및 정정**:
  1. 현재 코드 `src/dashboard.rs:545-565`에는 이미 `web` 플랫폼을 처리하는 `CompositeDispatcher`가 존재하므로 엄밀히는 Web 대시보드도 일부 관여하고 있음. 그러나 실질적인 양방향 챗 및 승인 상호작용은 Discord에만 묶여 있음.
  2. 이전 계획서는 `src/channels/`라는 완전히 새로운 계층과 `Envelope` 타입을 제안했으나, 현재 코드베이스는 이미 `SessionKey`, `InboundEvent`, `OutboundAction`, `DeliveryLedgerService`(중복 방지 원장), `SmartApprovalGuard`가 `OutboundDispatcher` 트레이트를 중심으로 긴밀하게 결합되어 있음.
  3. 따라서 별도의 `src/channels/` 트레이트를 만들어 기존 코드를 뒤엎는 대신, `SessionKey.platform`("discord" vs "slack")에 따라 다중 디스패처로 분기하는 `MultiChannelDispatcher`를 도입하고, 크론 스케줄러의 `destination.chat_id` 검증을 플랫폼별 검증기로 추상화하는 것이 훨씬 작고 안전하며 회귀 없는 구현 경로임.

## 구현 단계

### 단계 1: 크론 스케줄러 및 알림의 플랫폼 중립적 채널 ID 추상화
- **변경 파일**: `src/cron/scheduler.rs`
- **작업 내용**:
  - `CronNotification` 구조체(`src/cron/scheduler.rs:444`)의 `channel_id: u64`를 `channel_id: String`으로 변경.
  - `deliver()`(`src/cron/scheduler.rs:2353-2385`)에서 `chat_id.parse::<u64>()` 강제를 제거하고, 목적지 플랫폼(`destination.bot_id` 또는 `destination.chat_id` 패턴)에 따라 `SessionKey::new("slack", ...)` 또는 `SessionKey::new("discord", ...)`를 분기 생성하도록 수정.
- **DB 마이그레이션**: 불필요 (기존 `cron_jobs` 테이블의 `destination` 컬럼은 JSON 문자열로 저장됨).
- **킬 스위치**: 해당 없음 (기존 Discord 동작 100% 하위 호환 유지).
- **검증 시나리오**:
  - 문자열 채널 ID를 가진 모의 크론 잡 생성 및 발송 테스트:
  - 검증 기준: `destination.chat_id = "C0C9Z1Z4G02"`인 크론 잡이 `invalid Discord channel ID` 오류 없이 `deliver()` 통과.

### 단계 2: 다중 플랫폼 아웃바운드 라우터(MultiChannelDispatcher) 구현
- **변경 파일**: `src/multiplexer/actor.rs`, `src/dashboard.rs`, `src/main.rs`
- **신규 파일**: `src/channels/mod.rs`, `src/channels/router.rs`
- **작업 내용**:
  - `MultiChannelDispatcher` 구현: `session.platform`을 확인하여 `"discord"`는 `DiscordEgress`로, `"slack"`은 `SlackEgress`로, `"web"`은 `WebDashboardDispatcher`로 라우팅.
  - 플랫폼별 디스패처가 미등록되었거나 알 수 없는 플랫폼인 경우 명확한 에러(`OmonError::Config("unsupported platform: ...")`) 반환.
- **킬 스위치**: `OMON_SLACK_ENABLED=false` (기본값 off). off일 경우 기존과 동일하게 Discord 전용 디스패처로만 동작.
- **검증 시나리오**:
  - `sqlite3 ~/.omon/gateway.db "SELECT id, session_key FROM delivery_obligations LIMIT 5;"`
  - 로그 확인: `MultiChannelDispatcher routing action platform=discord` 및 `platform=slack` 디버그 로그 정상 출력.

### 단계 3: Slack 어댑터 (인그레스 소켓 모드 및 이그레스 API) 구현
- **신규 디렉토리 및 파일**: `src/slack/mod.rs`, `src/slack/adapter.rs`, `src/slack/egress.rs`, `src/slack/approval.rs`
- **자격증명 참조**: `~/.omon/secrets/slack-bot-token`, `~/.omon/secrets/slack-app-token` (오너 슬랙 앱 OmOMeow, App ID `A0CA2QMJQN8`, Bot `U0C9ST69M8B`).
- **작업 내용**:
  - Socket Mode 클라이언트(WebSocket)를 통해 멘션 및 DM 이벤트를 수신하여 `InboundEvent`로 변환 후 `multiplexer.route(event)` 호출.
  - `SlackEgress`(`OutboundDispatcher` 구현): Slack Web API(`chat.postMessage`, `chat.update`, `reactions.add` 등)를 통해 메시지 전송.
  - 블록 키트(Block Kit)를 활용한 승인 버튼(Approve Once / Session / Always / Deny) UI 렌더링 및 인터랙션 페이로드를 `SmartApprovalGuard`로 연동.
  - 디스코드와 동일하게 오너 DM 및 특정 채널(`#omo-bot-test`, `C0C9Z1Z4G02`)에 스티키 세션 지원.
- **DB 마이그레이션**: `migrations/0045_slack_channels.sql`
  ```sql
  CREATE TABLE IF NOT EXISTS slack_channel_cursors (
      channel_id TEXT PRIMARY KEY,
      last_read_ts TEXT NOT NULL,
      updated_at TEXT NOT NULL
  );
  ```
- **킬 스위치**: `OMON_SLACK_ENABLED=true`
- **검증 시나리오**:
  - 오너 Slack DM(`D0C9HNVUK5M`) 또는 `#omo-bot-test`(`C0C9Z1Z4G02`)에서 핑 발송:
  - 게이트웨이 로그: `slack event received channel=C0C9Z1Z4G02 user=U04M1JNJM8T` 및 `slack dispatch success ts=...` 확인.

### 단계 4: 양방향 승인(SmartApprovalGuard) 다중 채널 통합
- **변경 파일**: `src/discord/approval.rs`, `src/slack/approval.rs`
- **작업 내용**:
  - `ApprovalRequest` 액션 발생 시, 세션이 시작된 원본 채널(`session.platform`)로 대화형 승인 버튼 메시지 전송.
  - Slack 인터랙션 콜백(`block_actions`) 수신 시 `omon:approval:{request_id}:{decision}` 포맷을 파싱하여 `SmartApprovalGuard.resolve_custom_id()`로 원샷 채널 해제.
- **검증 시나리오**:
  - 슬랙 세션에서 터미널 보호 명령 실행 유도 -> Slack 채널에 버튼 4종 렌더링 -> "Once" 클릭 시 게이트웨이 로그에 `approval resolved request_id=... decision=Once` 기록되고 명령 실행 완료.

## 위험·의존성

1. **상호 의존 테마**:
   - `outbox-destination`: Grok Bot의 Slack 채널 오발송 사고처럼, 불변 ID가 아닌 표시명/유사 이름 채널로 오발송되는 위험을 원천 차단하기 위해 목적지 바인딩 및 Outbox 격리가 선행 또는 병행되어야 함.
   - `approval-effects` / `approval-grants`: Discord와 Slack 양쪽에서 승인 상태 캐시(`session_cache`, `always_cache`)가 일관되게 공유되어야 함.
   - `notify-envelope`: 크론 알림이나 상태 보고가 채널별 포맷(Discord Markdown vs Slack mrkdwn/Block Kit)에 맞게 안전하게 인코딩되어야 함.
   - `credentials`: `~/.omon/secrets/slack-*-token` 파일 권한(0600) 관리 및 부팅 시 파일 존재 여부 검증.

2. **주요 위험 및 대응책**:
   - **Grok Bot 식 채널 간 유출 위험**: 하나의 워크스페이스 내에서 개인 금융/내부 정보가 포함된 크론 결과가 공용 채널로 발송되는 사고 방지. 채널 ID 매핑은 설정 파일 및 DB에 사전 등록된 승인 목록(`allowed_channels`)만 허용하는 fail-closed 정책 적용.
   - **Slack Socket Mode 연결 안정성**: 네트워크 단절 시 WebSocket 재연결 루프가 게이트웨이 전체 런타임에 영향을 주지 않도록 독립된 백그라운드 태스크로 격리하고 지수 백오프 적용.
   - **비대칭 채널 지원으로 인한 혼란 방지**: iMessage와 카카오톡은 양방향 시도를 엄격히 금지하고 읽기 전용 인박스로 고정하여 불필요한 계정 정지 및 프로세스 블로킹 원천 차단.

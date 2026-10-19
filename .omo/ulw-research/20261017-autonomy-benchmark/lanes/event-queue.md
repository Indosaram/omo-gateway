# 벤치마크 테마 분석 리포트: 이벤트 큐 (event-queue)

- 작성일: 2026-10-17
- 테마 ID: `event-queue` (G3 비중단 이벤트 큐 및 오너 전용 인터럽트)
- 대상 저장소: `omon-gateway` (`/Users/indo/code/project/omon-gateway`)
- 핵심 검토 파일: `src/multiplexer/actor.rs`, `src/multiplexer/router.rs`, `src/lanes/dispatch.rs`, `src/agent/omo_backend.rs`, `src/storage/db.rs`

---

## 판정

**채택 (adopt now)**

omon-gateway는 단일 오너의 macOS 상에서 Discord, cron, inbox, lanes를 총괄하는 상시 가동 데몬이지만, 현재 활성 턴(turn) 실행 중에 인입되는 시스템 이벤트(`EventOrigin::System`)를 `lead session is busy` 에러와 함께 즉시 거부·드롭하여 백그라운드 자동화(크론 장애 보고, 레인 완료 알림, 제안 등)가 지연되거나 유실되는 치명적 병목을 겪고 있습니다. 또한 현행 액터 큐는 인메모리 `VecDeque`에만 의존하여 데몬 크래시/재시작 시 대기 턴이 소실되며, 오너가 긴급 개입하려 해도 기존 턴을 인터럽트하지 못하고 최대 30분 동안 대기해야 하는 구조적 결함이 존재합니다. 따라서 외부 사건(웹훅 G1), 자가점검(하트비트 G14), 자율 행동(키비처 G2)을 안정적으로 수용하기 위한 전제조건으로서 영속 비중단 이벤트 큐(non-interrupting durable queue)와 오너 전용 인터럽트(owner-only interrupt) 구조를 즉시 채택합니다.

---

## 벤치마킹 원천

### 1. xAI Grok Bot
- **[1차] 큐 우선순위 및 오너 인터럽트 (Queue and Interrupt)**: Lee Robinson의 Stanford CS146S 강연 (`how-grok-bot-works.md` transcript 17:19)에 따르면, 오너가 자신의 개인 봇에 보낸 메시지는 현재 실행 중인 에이전트 루프를 즉시 중단(interrupt)하고 최우선 순위로 실행됩니다.
- **[1차] 비중단 큐 (Non-interrupting queue)**: 반면 그룹 채팅, Slack 채널, routine(스케줄), 백그라운드 작업 등은 실행 중인 턴을 중단하지 않고 큐 뒤에 차례대로 쌓입니다 (transcript 17:19).
- **[1차] 메시지 디바운스 및 배치 (Batching / Debounce)**: 짧은 시간(수백 밀리초~수 초) 안에 연속으로 들어온 여러 메시지는 휴리스틱을 통해 단일 턴 프롬프트로 묶여(batching) 순차 실행됩니다 (transcript 17:19).
- **[1차] Temporal 기반 듀러블 워크플로 (Durable Workflow with Crash Resume)**: 크래시 발생 시 1단계부터 무작정 재실행하는 단순 작업 큐와 달리, Temporal 워크플로를 통해 마지막으로 성공한 단계(step 3)부터 안전하게 재개(resume)합니다 (transcript 16:17, 24:19).
- **[2차] 헬퍼 완료 메시지 연동 (HelperDone Queue)**: Peter Piekarczyk의 분석 아티클 ("What Actually Happens When You Message Grok Bot")에 따르면, 백그라운드 헬퍼(helper) 작업의 완료 통보는 비중단 `HelperDone` 메시지로 큐에 유입되어 사용자 작업 흐름을 방해하지 않고 반영됩니다.

### 2. OpenAI Dots
- **[1차] 백그라운드 비간섭 실행 (Proactive Research)**: 공식 제품 문서 (https://help.openai.com/articles/20001529, 20001530)에 따르면, 사용자가 부재중일 때 도는 프로액티브 리서치 및 이벤트 모니터링은 읽기 전용 도구로 제한되며 대화 턴을 불필요하게 선점하지 않습니다.
- **[1차] 오너 제어권 탈환 (Take Over)**: 사용자가 복귀하거나 Activity 화면에서 "Take over"를 누르면 에이전트 동작이 중단되고 즉시 사람에게 제어권이 넘어옵니다.

### 3. Meta Muse
- **[1차] 비동기 감시와 대화 루프 분리**: 보안 백서 (http://security.muse.ai/)에 따르면 백그라운드 감시 데몬(Sentinel, hatch)이 비동기 소켓으로 작업하며, 알림이나 승인이 필요할 때만 결정론적 UI 카드(out-of-band)를 통해 전달하여 메인 대화 세션의 컨텍스트를 오염시키거나 실행을 방해하지 않습니다.

---

## 현재 코드

### 1. 현재 존재하는 구현 사실 (path:line 증거)

- **`src/multiplexer/actor.rs:20`**:
  세션 액터당 대기 가능한 최대 이벤트 수가 하드코딩되어 있습니다 (`const MAX_PENDING_EVENTS: usize = 64;`).
- **`src/multiplexer/actor.rs:102-146`**:
  액터가 수신하는 명령 집합인 `pub(crate) enum ActorCommand`가 정의되어 있으며, 이벤트 관련 명령으로 `Event(Box<InboundEvent>)`, `EventIfIdle { event, reply }`, `EventWithAck { event, ack }`, `Stop { reply }`, `Steer { guidance, reply }` 등이 포함됩니다.
- **`src/multiplexer/actor.rs:339`**:
  대기 큐가 프로세스 힙 메모리상의 `let mut pending_events: VecDeque<ActorCommand> = VecDeque::new();`로 선언되어 있어 영속성이 없습니다.
- **`src/multiplexer/actor.rs:358-366`**:
  액터가 idle 상태일 때 `ActorCommand::EventIfIdle`를 수신하더라도, 대기 큐에 이벤트가 남아있으면(`!pending_events.is_empty()`) `reply.send(Err(OmonError::Multiplexer("lead session is busy".into())))`를 반환하고 즉시 폐기합니다.
- **`src/multiplexer/actor.rs:460`**:
  각 턴의 취소를 제어하기 위해 `let cancellation = CancellationToken::new();`가 인스턴스화됩니다.
- **`src/multiplexer/actor.rs:498-502`**:
  활성 턴이 **실행 중인 동안** `ActorCommand::EventIfIdle`가 도착하면 액터는 `reply.send(Err(OmonError::Multiplexer("lead session is busy".into())))`를 보내 즉시 거부합니다.
- **`src/multiplexer/actor.rs:509-537`**:
  활성 턴 실행 중 일반 이벤트(`ActorCommand::Event` 또는 `ActorCommand::EventWithAck`)가 인입되면 `pending_events`에 추가되어 활성 턴 뒤에 FIFO로 줄을 섭니다. 활성 턴에 대한 인터럽트는 발생하지 않습니다.
- **`src/multiplexer/actor.rs:542-567`**:
  활성 턴 중 `ActorCommand::Stop`을 받으면 `cancellation.cancel()`을 호출하여 실행 중인 턴을 취소하지만, 동시에 **`pending_events`에 대기 중이던 모든 이벤트를 꺼내 `"stopped by user"` 에러로 강제 드롭**합니다.
- **`src/multiplexer/actor.rs:571-578`**:
  `ActorCommand::Steer` 명령 수신 시 `tokio::spawn`을 통해 비동기로 `runner.steer(&context, &guidance).await`를 호출합니다.
- **`src/multiplexer/actor.rs:900-930`**:
  서비스 셧다운 시 리시버를 비우며 `has_pending_turn`이 있으면 `sessions.resume_pending = 1`을 기록하지만, `pending_events` 내부의 모든 커맨드는 `"session actor shutting down"` 에러와 함께 드롭되고 오직 Discord 백필 커서만 되감깁니다 (`rewind_backfill_cursor`).
- **`src/multiplexer/actor.rs:1964-2030`**:
  단위 테스트 `async fn actor_queues_turns_in_order_without_cancellation()`가 작성되어 있어, 일반 유저 턴들이 앞선 턴을 취소하지 않고 순차 큐잉됨이 회귀 검증되고 있습니다.
- **`src/multiplexer/actor.rs:2032-2055`**:
  단위 테스트 `async fn actor_stop_cancels_active_turn_immediately()`가 작성되어 있어 `Stop` 명령이 활성 턴을 즉시 취소함을 검증합니다.
- **`src/multiplexer/router.rs:89-92`**:
  `send_event` 함수는 `ActorCommand::Event(Box::new(event))`를 전송합니다.
- **`src/multiplexer/router.rs:369-389`**:
  `event.origin == crate::EventOrigin::System`인 경우 일반 `send_event` 대신 `ActorCommand::EventIfIdle`를 보내며, 액터가 바쁘면 `lead session is busy` 에러를 반환받아 호출자에게 에러를 전파합니다.
- **`src/lanes/dispatch.rs:105-107`**:
  레인 이벤트 디스패처는 이벤트를 보내기 전에 `self.multiplexer.session_is_idle(&session).await?`를 확인하여 바쁘면 발송 자체를 생략합니다.
- **`src/lanes/dispatch.rs:187-198`**:
  `origin: EventOrigin::System`으로 `InboundEvent`를 생성하여 `self.multiplexer.route(event).await?`를 호출하고, 성공 시에만 `self.receipt(&ids, now, "lead_turn").await`로 SQLite의 `lane_events`에 전달 완료 처리를 합니다.
- **`src/agent/omo_backend.rs:2185-2240`**:
  `cancel_turn_on_peer` 메서드가 구현되어 있어, 5초 타임아웃 내에 omo app-server에 JSON-RPC `turn/interrupt`를 보내 활성 턴을 실제로 중단시킵니다.
- **`src/agent/omo_backend.rs:2270-2300`**:
  `steer` 메서드가 구현되어 있어, 5초 타임아웃 내에 omo app-server에 JSON-RPC `turn/steer`를 보내 진행 중인 턴에 가이던스를 주입합니다.
- **`src/models/events.rs:174-192`**:
  `EventOrigin` 열거형(`User`, `System`)과 `InboundEvent` 구조체가 정의되어 있으나, 이벤트 간 우선순위(Priority)나 소스 구분(Source) 필드는 존재하지 않습니다.
- **`src/storage/db.rs:95-105`, `322-330`, `332-340`**:
  `sessions.resume_pending` 컬럼을 1로 설정하거나 0으로 해제하는 헬퍼 함수들이 구현되어 있습니다.
- **`src/storage/db.rs:520-565`**:
  부팅 시 복구를 수행하는 `recover_resume_pending_sessions`는 오직 `messages` 테이블에서 이미 기록된 마지막 미완료 유저 턴(`find_last_unfinished_user_turn`) 1개만 복원합니다.
- **`migrations/0008_resume_pending.sql:1-4`**:
  `sessions` 테이블에 `resume_pending INTEGER NOT NULL DEFAULT 0` 컬럼과 인덱스만 추가되어 있으며, 대기 이벤트를 저장하는 별도 테이블은 없습니다.

### 2. 기존 계획서(플랜)의 오류 및 변경점 정정

1. **시스템 이벤트 거부 위치의 불일치 정정**:
   - *기존 계획서 주장*: `src/multiplexer/actor.rs:360-364`에서 시스템 이벤트를 버린다고 명시함.
   - *실제 코드 현황*: 라인 358-366은 액터 루프가 **idle**일 때 `pending_events`가 남아있는 경우의 체크입니다. 실제 턴이 **실행 중(running)**일 때 시스템 이벤트가 즉시 거부되는 지점은 `src/multiplexer/actor.rs:498-502` (`Some(ActorCommand::EventIfIdle { reply, .. }) => { reply.send(Err("lead session is busy")) }`)입니다.
2. **레인 이벤트 유실 여부의 과장 정정**:
   - *기존 계획서 주장*: 시스템 이벤트가 버려져 "cron·선제 결과 유실"이 발생한다고 기술함.
   - *실제 코드 현황*: 레인 이벤트의 경우 `src/lanes/dispatch.rs:105, 195-197`에서 사전에 `session_is_idle`을 확인하고, 레이스로 `route()`가 실패하면 `self.receipt()`를 실행하지 않으므로 SQLite `lane_events` 테이블에 남아 다음 주기에 재시도됩니다. 즉, 영구 소실은 아니지만 세션 큐에 들어가지 못해 헤드오브라인 블로킹(심각한 지연)이 발생합니다. 반면 레인이 아닌 단발성 시스템 이벤트(향후 유입될 웹훅, 크론 즉시 알림 등)는 `route()` 실패 시 재시도 주체가 없어 영구 유실됩니다.
3. **인메모리 큐와 영속 큐의 구분 결여 정정**:
   - *기존 계획서 주장*: 단순히 `pending_events`에 넣으면 된다고만 서술함.
   - *실제 코드 현황*: 이미 `src/multiplexer/actor.rs:339`에 인메모리 `pending_events`가 구현되어 있습니다. 하지만 (1) `EventIfIdle`가 busy 시 큐 진입 자체를 거부당하고, (2) 큐가 메모리에만 있어 프로세스 강제 종료(SIGKILL)나 크래시 시 모두 소실되며, (3) 우선순위와 오너 인터럽트 구분이 없다는 점이 실질적인 핵심 결함입니다.

---

## 구현 단계

전체 구현은 기존 운영 안전 수칙에 따라 `OMON_EVENT_QUEUE_ENABLED` 환경변수(기본값: `false`) 킬스위치로 보호하며, 단계별로 명확한 기계 검증 시나리오를 충족하도록 설계합니다.

### 1단계: 영속 큐 DB 마이그레이션 (`migrations/0046_event_queue.sql`)
- **변경/생성 파일**: `migrations/0046_event_queue.sql`
- **테이블 스키마**:
  ```sql
  CREATE TABLE IF NOT EXISTS pending_events (
      id TEXT PRIMARY KEY,
      session_key TEXT NOT NULL,
      priority INTEGER NOT NULL DEFAULT 10,
      origin TEXT NOT NULL,
      source TEXT NOT NULL,
      event_json TEXT NOT NULL,
      status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
      queued_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
      started_at TEXT,
      consumed_at TEXT,
      error TEXT
  );
  CREATE INDEX IF NOT EXISTS idx_pending_events_lookup
      ON pending_events(session_key, status, priority DESC, queued_at ASC);
  ```
- **환경변수 킬스위치**: `OMON_EVENT_QUEUE_ENABLED` (기본값: `false`)
- **기계 검증 시나리오**:
  ```bash
  sqlite3 ~/.omon/gateway.db "INSERT INTO pending_events (id, session_key, priority, origin, source, event_json) VALUES ('test-ev-1', 'discord:dm:123', 20, 'System', 'cron', '{}'); SELECT id, status, priority FROM pending_events WHERE id='test-ev-1';"
  # 검증 기대 결과: test-ev-1|pending|20 1행 출력 확인 후 삭제
  sqlite3 ~/.omon/gateway.db "DELETE FROM pending_events WHERE id='test-ev-1';"
  ```

### 2단계: 이벤트 모델 우선순위 및 소스 확장 (`src/models/events.rs`)
- **변경/생성 파일**: `src/models/events.rs`
- **구현 내용**:
  - `EventPriority` 열거형 추가:
    `Low = 0`, `Normal = 10`, `System = 20`, `Owner = 30`, `OwnerInterrupt = 40`
  - `EventSource` 열거형 추가:
    `OwnerDirect`, `ChannelUser`, `Cron`, `Lane`, `Webhook`, `Heartbeat`, `SystemInternal`
  - `InboundEvent` 구조체(`src/models/events.rs:180`)에 `priority: EventPriority`, `source: EventSource` 필드 추가 (하위 호환 `#[serde(default)]` 적용).
- **기계 검증 시나리오**:
  단위 테스트 실행 또는 serde 직렬화 라운드트립 검증:
  `priority`와 `source`가 누락된 기존 JSON 페이로드가 기본값(`Normal`, `ChannelUser`)으로 안전하게 디코딩되는지 확인.

### 3단계: 영속 스토리지 접근 계층 구현 (`src/storage/event_queue.rs`, `src/storage/mod.rs`)
- **변경/생성 파일**: `src/storage/event_queue.rs`, `src/storage/mod.rs`
- **구현 내용**:
  - `enqueue_event(pool, event: &InboundEvent) -> Result<()>`: `pending_events` 테이블에 INSERT.
  - `fetch_next_pending_event(pool, session_key: &str) -> Result<Option<InboundEvent>>`: `status='pending'`인 이벤트를 `priority DESC, queued_at ASC`로 1건 조회.
  - `mark_event_running(pool, id: &str) -> Result<()>`: `status='running'`, `started_at` 갱신.
  - `mark_event_completed(pool, id: &str) -> Result<()>`: `status='completed'`, `consumed_at` 갱신.
  - `mark_event_cancelled(pool, id: &str, reason: &str) -> Result<()>`
- **기계 검증 시나리오**:
  SQL 쿼리로 큐 삽입 및 상태 전이 검증:
  ```bash
  sqlite3 ~/.omon/gateway.db "SELECT count(*) FROM pending_events WHERE status = 'pending';"
  # 삽입 후 count 1 증가, completed 마킹 후 count 0 복귀 확인
  ```

### 4단계: 멀티플렉서 비중단 큐잉 및 스케줄러 개편 (`src/multiplexer/router.rs`, `src/multiplexer/actor.rs`)
- **변경/생성 파일**: `src/multiplexer/router.rs`, `src/multiplexer/actor.rs`
- **구현 내용**:
  - `src/multiplexer/router.rs:369-389` 수정: `OMON_EVENT_QUEUE_ENABLED=true`일 때, `event.origin == EventOrigin::System`이라도 액터에게 `EventIfIdle`로 찔러 거부당하지 않고, 영속 큐에 즉시 삽입(`enqueue_event`)한 후 액터에게 `ActorCommand::NotifyPending`만 전달하여 즉시 `Ok(())` 반환.
  - `src/multiplexer/actor.rs:344` 턴 루프 수정: 턴이 끝나고 다음 커맨드를 찾을 때 인메모리 큐뿐만 아니라 DB의 `fetch_next_pending_event`를 우선순위 순으로 읽어 다음 턴으로 소비.
  - 연속 도착 이벤트에 대한 500ms 마이크로 디바운스(Grok Bot식 batch window) 옵션 적용.
- **기계 검증 시나리오**:
  로그 검증: 활성 턴 실행 중 시스템 이벤트 인입 시 `"lead session is busy"` 에러 로그가 0건이어야 하며, 다음 로그가 기록되어야 함:
  `"event queued non-interruptingly: session={}, source=lane, priority=20"`
  활성 턴 종료 직후 대기하던 시스템 이벤트가 연속하여 실행됨을 로그로 확인:
  `"starting pending turn from durable queue: event_id={}"`

### 5단계: 오너 전용 인터럽트 메커니즘 (`src/multiplexer/actor.rs`, `src/agent/omo_backend.rs`)
- **변경/생성 파일**: `src/multiplexer/actor.rs`, `src/agent/omo_backend.rs`
- **구현 내용**:
  - 오너 세션에서 `priority == EventPriority::OwnerInterrupt` (예: `/stop`, 긴급 중단 접두사, 또는 45초 무응답 타임아웃 감지) 이벤트가 인입된 경우:
  - `src/multiplexer/actor.rs:542`의 무차별 큐 드롭 동작과 달리, 현재 활성 턴의 `cancellation.cancel()`만 호출하여 omo app-server에 `turn/interrupt`(`src/agent/omo_backend.rs:2185-2240`)를 전송하되, **`pending_events`에 이미 적재된 큐 항목은 일절 드롭하지 않고 보존**.
  - 인터럽트된 턴의 미완료 결과는 취소로 마킹하고, 방금 인입된 오너 메시지를 최우선(`priority = OwnerInterrupt`)으로 승격하여 즉시 다음 턴으로 실행.
- **기계 검증 시나리오**:
  장시간 블로킹 작업 중 오너 인터럽트 주입 시 로그 검증:
  `"turn/interrupt sent to peer on thread"` 발생 확인,
  `"active turn interrupted by owner; 1 queued events preserved"` 로그 확인,
  직후 대기 큐의 이벤트 손실 없이 오너 명령이 실행됨을 확인.

### 6단계: 크래시 복구(Crash Resume) 정합성 연동 (`src/main.rs`, `src/storage/db.rs`)
- **변경/생성 파일**: `src/main.rs`, `src/storage/db.rs`
- **구현 내용**:
  - `src/main.rs:520` 부팅 시 `recover_resume_pending_sessions_with_auth`에서 `sessions.resume_pending = 1` 복구 외에도, `pending_events`에 `status IN ('pending', 'running')`으로 남아있는 미소비 이벤트를 조회하여 `status = 'pending'`으로 되돌리고 멀티플렉서에 재스케줄링.
  - 강제 종료(SIGKILL) 후 재기동 시에도 대기 중이던 이벤트 유실 0건 보장.
- **기계 검증 시나리오**:
  게이트웨이 데몬 강제 종료(`kill -9`) 후 재기동 시 SQL 검증:
  ```bash
  sqlite3 ~/.omon/gateway.db "SELECT count(*) FROM pending_events WHERE status = 'pending';"
  # 재기동 후 백그라운드 턴 디스패치에 의해 수 초 내에 0으로 감소하는지 확인
  ```

---

## 위험·의존성

### 1. 테마 간 의존성 (Dependencies)
- **`trigger-webhook` (G1, 강한 상호 의존)**: 외부 웹훅은 비동기적이고 예측 불가능한 시점에 도착합니다. 비중단 이벤트 큐가 먼저 갖춰지지 않으면 웹훅 요청이 활성 대화 중 "lead session is busy"로 거부되거나 드롭되므로, `event-queue`가 `trigger-webhook`의 필수 선행 기반이 됩니다.
- **`heartbeat` (G14, 강한 의존)**: 주기적인 하트비트가 발생시킨 자가점검 이벤트가 오너의 대화 턴을 끊지 않고 큐에 들어가 순차 실행되어야 하므로 `event-queue`에 의존합니다.
- **`kibitzer-action` (G2, 강한 의존)**: 키비처가 독자적으로 백그라운드 레인 시작을 결정하고 완료했을 때, 완료 보고 이벤트(`HelperDone`)가 비중단 큐를 통해 리드 에이전트에 전달되어야 합니다.
- **`notify-envelope` (연계 의존)**: 큐에 적재된 시스템 이벤트가 턴 실행 후 오너에게 결과를 통보할 때, 야간(KST 0~8시) 발송 억제 및 salience gate(알릴 가치 판단)와 결합되어 불필요한 알림 스팸을 방지해야 합니다.
- **`channels` (기반 의존)**: Discord, Slack, iMessage 등 다중 채널에서 들어오는 이벤트가 표준화된 `InboundEvent` 구조체와 우선순위를 부여받아 큐에 전달되어야 합니다.

### 2. 주요 위험 요소 및 완화 방안 (Risks & Mitigations)
1. **이벤트 폭주(Event Storm) 및 큐 비대화 위험**:
   - *위험*: 크론 작업이 반복 실패하거나 외부 웹훅이 연속 호출될 경우 큐에 수백 개의 이벤트가 쌓여 컨텍스트 토큰 폭발 및 모델 비용 급증이 발생할 수 있습니다.
   - *완화 방안*: 세션당 최대 큐 크기(`MAX_PENDING_EVENTS = 64`)를 엄격히 강제하고, 동일한 레인/크론 이벤트는 `src/lanes/dispatch.rs:118`처럼 최신 상태로 병합(collapse/deduplicate)하며, 30분 이상 경과한 미처리 저우선순위 이벤트는 `expired` 처리합니다.
2. **무차별 오너 인터럽트 및 턴 전환 경합(Turn Race Condition) 위험**:
   - *위험*: Grok Bot의 방식을 무비판적으로 모방하여 오너가 보낸 모든 일반 메시지가 활성 턴을 인터럽트하게 만들면, 컴파일, 파일 편집, 배포 스크립트 실행 등 비가역적 작업을 수행 중이던 턴이 깨지고 불일치 상태가 유발될 수 있습니다. 특히 게이트웨이 운영 메모리(`reference/omon-gateway-background-turn-handoff-2026-10-10.md`)에 기록되었듯, 백그라운드 확장 턴과 큐잉된 턴 간의 레이스로 인해 거짓 인터럽트(false 45s interrupt)나 후속 턴의 응답 드롭 버그가 재발할 위험이 있습니다.
   - *완화 방안*: 오너의 일상적 텍스트는 일반 비중단 큐에 적재하되, 명시적 중단 명령(`/stop`, `/interrupt`, 긴급 접두사)이거나 게이트웨이 운영 원칙에 정의된 "45초 이상 무응답 행(hang)" 상황에서만 활성 턴을 `turn/interrupt`로 끊고 즉시 새 턴을 시작하도록 제한합니다. 또한 턴 인터럽트 후 새 턴으로 전환할 때 app-server의 `turnId` 승계 및 `active_turns` 매핑을 엄격히 동기화합니다.
3. **SQLite Write Lock 경합 위험**:
   - *위험*: 이벤트 큐 삽입과 턴 상태 업데이트가 빈번하게 발생할 경우 SQLite WAL 모드에서도 write lock 경합이 발생할 수 있습니다.
   - *완화 방안*: 단일 오너 데몬 아키텍처의 특성을 활용하여 트랜잭션 범위를 최소화하고, 인메모리 액터 알림(`NotifyPending`)과 write-ahead DB 기록을 비동기 분리하여 경합을 차단합니다.

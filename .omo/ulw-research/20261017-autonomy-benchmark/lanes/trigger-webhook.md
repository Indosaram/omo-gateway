# 벤치마크 테마 보고서: trigger-webhook (G1 외부 트리거)

## 판정
**채택 (adopt now)**

단일 소유자 맥 환경의 `omon-gateway`는 현재 시간 기반 cron 및 메신저 인바운드 외에 GitHub PR/이슈, 외부 CI, 모니터링 경보 등 외부 시스템 사건을 수신할 수단이 전무하여 비동기 협업 파이프라인 연계가 차단되어 있습니다. 소유자 철학인 "모델 주기적 폴링 금지, 이벤트 기반 구동, 실패는 게이트웨이 코드로 해결"에 가장 정확히 부합하며, Standard Webhooks(HMAC-SHA256, Replay 방지, 1MiB 상한, `202 Accepted` 비동기 큐잉) 패턴을 도입해 모델 호출 비용 0으로 외부 이벤트를 안전하게 수용할 수 있습니다.

---

## 벤치마킹 원천
1. **xAI Grok Bot [1차]**
   - **출처:** `docs/research/autonomous-agent-platform-plan.md` 104-105행, 196행, 475행 (`https://docs.x.ai/grok/automations/webhooks`, `https://x.ai/bot/guides/grok-bot-101`, Lee Robinson 강연 녹취록 `https://www.peterp.me/transcripts/how-grok-bot-works/`).
   - **구현 방식:** Webhook 수신 시 `whsec_` 프리픽스 비밀키 기반 Standard Webhooks HMAC-SHA256 서명 검증, 타임스탬프 허용 오차 5분(Replay 방어), 최대 페이로드 1MiB 제한, 즉시 `202 Accepted` 비동기 응답 반환. 수신된 이벤트는 트리거 라우터에서 중복 제거(dedupe) 후 비동기 워크플로 큐로 주입.
2. **OpenAI Dots [2차]**
   - **출처:** `docs/research/autonomous-agent-platform-plan.md` 180행, 247행, `.omo/ulw-research/20261016-133102-autonomous-agents/wave1/dots-arch.md` 48-49행 (`https://myclaw.ai/blog/dots-ai`).
   - **구현 방식:** 연결된 외부 서비스의 이벤트 모니터링을 통한 자율 트리거 및 후속 조치 수행. 단, 공식 공개 Webhook API/SDK는 제공되지 않는 사설 연동 방식(`dots-arch.md` 71행)임.

---

## 현재 코드
현재 저장소 코드베이스를 열람하여 확인한 상태와 이전 기획서의 차이점은 다음과 같습니다.

1. **HTTP 라우터 및 호스트·오리진 검증 (`src/dashboard.rs:742-788`, `src/dashboard.rs:208-229`, `src/dashboard.rs:253-267`)**
   - 대시보드 Axum 라우터는 `src/dashboard.rs:742-788`에 정의되어 있으며, `/api/cron/jobs/{id}/trigger` (`src/dashboard.rs:764`) 등 내부 제어용 엔드포인트만 존재합니다.
   - **핵심 차이 및 누락 지점:** `src/dashboard.rs:788`에서 라우터 전체에 `axum::middleware::from_fn(validate_host_and_origin)`이 전역 적용되어 있습니다. `src/dashboard.rs:208-229`는 `validate_host_and_origin` 함수에서 루프백(`localhost`, `127.0.0.1` 등)이 아닌 `Host` 헤더를 `403 Forbidden`으로 즉시 거부하며, `src/dashboard.rs:253-267`은 외부 `Origin`을 가진 상태 변경(POST 등) 요청을 차단합니다. 따라서 Tailscale Funnel이나 외부 터널/리버스 프록시를 통해 외부 Webhook을 받으려면, `validate_host_and_origin` 레이어 바깥에 별도 라우트로 분리하거나 전용 Webhook 경로(`/hooks/{id}`)에 대해 HMAC 기반 인증 우회 미들웨어를 설계해야 합니다. 이전 계획(`autonomous-agent-platform-plan.md` 310행)은 `POST /hooks/<trigger_id>` 추가만 언급하고 이 전역 보안 미들웨어와의 충돌을 간과했습니다.
2. **수동 크론 트리거 로직 (`src/dashboard.rs:1783-1814`, `src/cron/scheduler.rs:1114-1123`)**
   - 현재 외부에서 HTTP로 실행을 유발할 수 있는 유일한 경로는 `POST /api/cron/jobs/{id}/trigger` (`src/dashboard.rs:1783`)입니다.
   - 이는 스케줄러 인스턴스의 `trigger(&id)` (`src/cron/scheduler.rs:1114`)를 호출하여 해당 크론 작업의 리스를 획득하고 실행(`src/cron/scheduler.rs:1121`)할 뿐, 임의의 외부 페이로드를 전달하거나 서명 검증을 수행하는 Webhook 인터페이스가 아닙니다.
3. **스케줄러의 Wake Notify (`src/cron/scheduler.rs:586`, `src/cron/scheduler.rs:705`, `src/cron/scheduler.rs:759-765`)**
   - `CronScheduler` 구조체는 내부 `wake: Arc<Notify>` (`src/cron/scheduler.rs:586`, `src/cron/scheduler.rs:705`)를 보유하고 있으며, 스케줄러 루프(`src/cron/scheduler.rs:759-765`)에서 주기적 poll 간격(`poll_interval`) 대기와 함께 `wake.notified()`를 대기합니다.
   - 작업 등록/수정/삭제 시 `self.wake.notify_one()` (`src/cron/scheduler.rs:951`, `src/cron/scheduler.rs:992`, `src/cron/scheduler.rs:1024`, `src/cron/scheduler.rs:1063`, `src/cron/scheduler.rs:1090`, `src/cron/scheduler.rs:1103`)을 호출해 즉시 스케줄러를 깨웁니다. 외부 이벤트 발생 시 즉시 크론 실행 루프를 깨우는 통로로 재활용 가능합니다.
4. **멀티플렉서 라우팅 및 시스템 이벤트 드롭 한계 (`src/multiplexer/router.rs:358-385`, `src/multiplexer/actor.rs:360-368`)**
   - `SessionMultiplexer::route` (`src/multiplexer/router.rs:358`)는 이벤트를 세션 액터로 보냅니다.
   - 이벤트 출처가 `EventOrigin::System` (`src/models/events.rs:176`, `src/multiplexer/router.rs:371`)인 경우, `ActorCommand::EventIfIdle`을 액터에 전달합니다.
   - 그러나 `src/multiplexer/actor.rs:360-365`를 보면, 대기 중인 이벤트(`pending_events`)가 존재할 때 `ActorCommand::EventIfIdle`은 큐에 들어가지 않고 즉시 `Err("lead session is busy")`를 반환하며 버려집니다. 외부 Webhook 이벤트를 곧바로 멀티플렉서에 넣을 경우 소유자 세션이 작업 중이면 유실되므로, 영속 큐(DB `inbound_events`) 적재 후 idle 시 순차 소비하는 버퍼링이 필수적입니다.
5. **Discord 어댑터의 Webhook 메시지 완전 무시 (`src/discord/adapter.rs:1862`, `src/discord/adapter.rs:1930`)**
   - Discord 인바운드 처리 시 `message.webhook_id.is_some()`이면 즉시 `return None;`으로 필터링되어 게이트웨이 내부로 진입하지 못합니다.

---

## 구현 단계

### 1단계: DB 스키마 마이그레이션 (`migrations/0045_triggers.sql`)
- **변경/생성 파일:** `migrations/0045_triggers.sql` (최신 마이그레이션 `0044_tracker.sql` 다음 번호)
- **생성 테이블:**
  ```sql
  CREATE TABLE IF NOT EXISTS triggers (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      secret_hash TEXT NOT NULL,
      target_type TEXT NOT NULL CHECK(target_type IN ('cron_job', 'lane_profile')),
      target_id TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
      created_at DATETIME NOT NULL,
      rotated_at DATETIME
  );

  CREATE TABLE IF NOT EXISTS trigger_events (
      id TEXT PRIMARY KEY, -- webhook-id 헤더 값 또는 UUID (멱등성 보장)
      trigger_id TEXT NOT NULL REFERENCES triggers(id) ON DELETE CASCADE,
      status TEXT NOT NULL CHECK(status IN ('received', 'processed', 'failed', 'dropped')),
      payload_json TEXT NOT NULL,
      received_at DATETIME NOT NULL,
      processed_at DATETIME,
      error_message TEXT
  );
  CREATE INDEX IF NOT EXISTS idx_trigger_events_trigger_received ON trigger_events(trigger_id, received_at);
  ```
- **킬스위치:** `OMON_TRIGGER_WEBHOOK_ENABLED=false` (기본값: false, zero-config)
- **기계 검증 인수 시나리오:**
  - CLI 명령: `sqlite3 ~/.omon/state.db ".schema triggers" ".schema trigger_events"`
  - 기대 결과: 종료 코드 0 및 두 테이블의 DDL 스키마 출력 일치.

### 2단계: Standard Webhooks HMAC-SHA256 및 헤더 검증기 모듈 구현
- **변경/생성 파일:** `src/triggers/mod.rs`, `src/triggers/verifier.rs`, `src/lib.rs` (모듈 등록)
- **핵심 로직:**
  - 헤더 규격: `webhook-id`, `webhook-timestamp`, `webhook-signature` (Standard Webhooks v1 규격).
  - 타임스탬프 허용 오차: 현재 시각 기준 $\pm 300$초(5분) 초과 시 거부 (`401 Unauthorized`).
  - 서명 생성: `HMAC-SHA256(secret, "${webhook-id}.${webhook-timestamp}.${raw_body}")` 계산 후 `webhook-signature`와 상수 시간 비교(`subtle` 또는 타이밍 공격 방지 슬라이스 비교). 비밀키는 `whsec_` 접두사 처리.
  - 본문 크기 상한: 1MiB (1,048,576 bytes) 초과 시 즉시 `413 Payload Too Large` 반환.
- **기계 검증 인수 시나리오:**
  - 단위 테스트 작성: `tests/test_trigger_verifier.rs`
  - 검증 케이스:
    1. 유효한 서명과 2분 전 타임스탬프 $\rightarrow$ `Ok(())`
    2. 잘못된 비밀키로 서명된 페이로드 $\rightarrow$ `Err(InvalidSignature)`
    3. 6분 전 타임스탬프 $\rightarrow$ `Err(ReplayWindowExpired)`
    4. 1.5MiB 페이로드 $\rightarrow$ `Err(PayloadTooLarge)`

### 3단계: HTTP 수신 엔드포인트 및 미들웨어 통합 (`src/dashboard.rs`)
- **변경/생성 파일:** `src/dashboard.rs`, `src/triggers/router.rs`
- **핵심 로직:**
  - `src/dashboard.rs:742` 라우터 구성 시, `validate_host_and_origin` 미들웨어 영향권 밖의 공개 엔드포인트로 `/hooks/{trigger_id}` 라우트 등록 (`post(handle_webhook_trigger)`).
  - `OMON_TRIGGER_WEBHOOK_ENABLED`가 false인 경우 즉시 `503 Service Unavailable` 반환.
  - 페이로드 검증 통과 시 `trigger_events`에 `INSERT OR IGNORE` (중복 `webhook-id` 방지) 후 즉시 `202 Accepted` 반환 (`{"accepted": true, "event_id": id}`).
  - 타겟이 `cron_job`인 경우 `state.scheduler.trigger(&target_id)` 호출 및 `state.scheduler.wake.notify_one()` 연계.
- **기계 검증 인수 시나리오:**
  - HTTP 요청:
    ```bash
    curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:9119/hooks/trg_test \
      -H "Content-Type: application/json" \
      -H "webhook-id: msg_01" \
      -H "webhook-timestamp: $(date +%s)" \
      -H "webhook-signature: v1,bad_sig" \
      -d '{"event":"ping"}'
    ```
  - 기대 결과: `401`
  - 올바른 서명 요청 시 기대 결과: `202`, DB 확인: `SELECT count(*) FROM trigger_events WHERE id='msg_01'` 결과 `1`.

### 4단계: 비동기 큐 멱등성 및 액터 연동 (`src/multiplexer/actor.rs`)
- **변경/생성 파일:** `src/triggers/dispatcher.rs`, `src/multiplexer/actor.rs`
- **핵심 로직:**
  - `target_type`이 `lane_profile`인 경우, 액터가 busy(`src/multiplexer/actor.rs:360-365`) 상태여도 버려지지 않도록 `trigger_events` 상태를 `received`로 유지하고 백그라운드 태스크에서 액터 idle 시점에 디스패치.
  - 외부 Webhook 입력은 잠재적 위험 입력을 내포하므로 지시문(instruction)으로 직접 격상되지 않도록 이벤트 메타데이터에 `untrusted_webhook: true` 태그 부여.
- **기계 검증 인수 시나리오:**
  - 세션이 작업 중일 때 동일 `webhook-id` 2회 수신 시:
    - SQL 쿼리: `SELECT count(*), count(DISTINCT id) FROM trigger_events WHERE id='dup_01';`
    - 기대 결과: `1|1` (중복 저장 차단).

---

## 위험·의존성

### 관련 테마 의존성
- **taint:** Webhook으로 유입되는 외부 데이터(GitHub PR 본문, 외부 이슈, 제3자 알림 등)는 프롬프트 인젝션 벡터가 될 수 있습니다. 신뢰되지 않은 입력이 직접 셸 명령이나 시스템 지시문으로 승격되지 않도록 격리하는 `taint` 테마에 직접 의존합니다.
- **event-queue:** 현재 `src/multiplexer/actor.rs:360-365`는 세션이 busy일 때 시스템 이벤트를 즉시 거부(`OmonError::Multiplexer("lead session is busy")`)하므로, Webhook 이벤트를 유실 없이 처리하기 위해서는 버퍼링과 재시도를 보장하는 `event-queue` 테마와 결합되어야 합니다.
- **kibitzer-action:** Webhook 트리거가 특정 작업 레인을 자율 실행(`start_lane`)하려 할 때, 키비처 가드 및 권한 통제 메커니즘과 연계되어야 합니다.
- **approval-effects & approval-grants:** 외부 Webhook에 의해 시작된 작업이 파괴적 작업(파일 삭제, 외부 발송 등)을 일으킬 수 있으므로 무인 실행 시 승인 게이트 의존성이 발생합니다.
- **notify-envelope:** Webhook 수신 결과를 소유자(Discord/Slack)에게 보고할 때, 조용한 시간(KST 0~8시 알림 금지) 및 통지 중요도(salience) 규칙을 준수해야 합니다.

### 위험 요소 및 완화 방안
1. **SSRF 및 호스트 헤더 탈취 위험:**
   - 대시보드의 전역 보안 계층(`validate_host_and_origin`, `src/dashboard.rs:788`)을 부주의하게 해제하면 로컬 웹 대시보드가 외부 브라우징 공격에 노출됩니다.
   - **완화:** Webhook 수신 엔드포인트는 기존 대시보드 API와 분리된 전용 서브라우터로 격리하고, 엄격한 HMAC-SHA256 서명 검증을 통과한 요청만 처리하도록 국소화해야 합니다.
2. **Replay 공격 및 DoS 공격:**
   - 대량의 외부 HTTP 요청으로 인한 CPU 자원 소모 또는 중복 트리거 발생 가능성.
   - **완화:** 1MiB 크기 상한 강제, 5분 타임스탬프 슬라이딩 윈도우 검사, `trigger_events`의 `id` UNIQUE 인덱스를 통한 원자적 멱등성 보장으로 완화합니다.
3. **토큰 소모 및 비용 폭주 위험:**
   - 외부 웹훅이 빈번하게 호출되어 모델 턴을 무한정 가동시킬 위험.
   - **완화:** 웹훅 수신 및 큐잉 단계는 모델 호출 비용 0(Rust 네이티브)으로 완결하며, 실제 모델 실행 단계 진입 전 디바운스(debounce) 및 유효성 필터를 게이트웨이 레벨에서 처리합니다.

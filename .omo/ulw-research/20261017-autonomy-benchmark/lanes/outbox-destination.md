# 벤치마크 분석 보고서: Outbox & Destination Binding (G6)

## 판정
**채택 (Adopt now)**  
단일 오너·macOS 환경의 omon-gateway는 현재 크론 보고, 인박스 요약, 에이전트 턴 완료 결과가 모두 검증되지 않은 즉시 발송(OutboundAction::SendMessage) 경로를 타고 있어 잠재적 오발송 및 정보 유출 위험에 직접 노출되어 있습니다. 특히 크론 전송지 설정(`src/cron/scheduler.rs:2535-2615`)이 원시 JSON 문자열 파싱에 의존하고 불변 대상자 레지스트리(Destinations)나 발송 대기 원장(Outbox Drafts)이 없어, Grok Bot의 채널명 혼동 오발송 사고(동일 이름 Slack 채널에 개인 은행 명세서 노출)와 동일한 취약점이 상존합니다. 따라서 비가역적 외부 발송 동작의 결정론적 안전 경계를 구축하기 위해 불변 ID 기반 `destinations` 바인딩, 모호한 명칭에 대한 fail-closed 검증, 그리고 사전 검토 가능한 `outbox` 테이블 도입을 즉시 채택합니다.

---

## 벤치마킹 원천
1. **xAI Grok Bot [1차] / [2차]**
   - **사례**: 2026년 10월 초, 읽기 전용 은행 계좌 접근 권한을 가진 CFO 봇이 월간 개인 재무 보고서를 비공개 에이전트 그룹챗 대신 이름이 동일한 회사의 'Exec-team' 공개 Slack 채널에 게시하는 대형 유출 사고 발생 [2차](https://africa.businessinsider.com/news/my-personal-ai-agent-posted-my-bank-details-on-company-slack-its-made-me-rethink-how/hw9t2xn). xAI는 이후 채널 간 정보 이동 시 사용자 명시적 허가를 받도록 핫픽스 적용.
   - **아키텍처**: 비가역적 외부 작용(이메일, Slack 발송 등)을 즉시 전송하지 않고 편집·승인 가능한 draft 형태로 큐잉하여 사용자 검토 목록을 제공 [1차](https://x.ai/bot), [2차](https://www.peterp.me/transcripts/how-grok-bot-works/).
2. **Meta Muse [1차]**
   - **원칙**: 외부 네트워크로 나가는 작용(외부 발송, 결제)을 대화창 텍스트가 아닌 결정론적 승인 카드와 out-of-band 격리 경계에서만 인가 [1차](http://security.muse.ai/). 사용자 데이터를 읽어 오염(tainted)된 프로세스는 외부 전송 시 자동 승인 권한을 즉시 상실.
3. **OpenAI Dots [1차]**
   - **원칙**: "초안(draft) 작성 요청은 발송(send) 허가가 아니다"라는 명시적 원칙을 모델 수준이 아닌 시스템 경계 수준에서 강제 [1차](https://help.openai.com/articles/20001530).

---

## 현재 코드
현재 omon-gateway 소스 코드를 조사한 결과, 대상자 결정 및 메시지 발송 체계의 실태는 다음과 같습니다:

1. **크론 전송지 파싱의 원시 문자열 의존과 모호성 허용**
   - `src/cron/scheduler.rs:2535-2615` (`delivery_destinations`): `payload`의 `channel_id`, `deliver` 문자열(`origin`, `all`, `discord`, `discord:<id>`, 또는 쉼표 구분 원시 ID)을 파싱하여 임의의 문자열 `chat_id`를 가진 `crate::HermesOrigin` 목록으로 변환합니다.
   - `src/cron/scheduler.rs:2602-2613`: prefix가 없는 원시 토큰은 검증 없이 Discord 채널 ID로 간주합니다.
   - 불변 대상자 레지스트리(예: `destinations` 테이블)에 대한 조회나 사전 등록 검증이 전혀 없으며, 대상자가 유효한지 또는 비공개/공개 채널인지 사전에 확인하지 않습니다.

2. **크론 발송 경로의 즉시 디스패치 및 Outbox 부재**
   - `src/cron/scheduler.rs:2367-2425` (`deliver`): `HermesOrigin`의 `chat_id`를 파싱한 뒤 곧바로 `dispatcher.dispatch(OutboundAction::SendMessage { ... }).await`를 호출합니다.
   - `src/cron/scheduler.rs:2410-2423`: `DeliveryLedgerService`에 의무 기록(`record_obligation`, `mark_obligation_attempting`, `mark_obligation_delivered`)을 남기지만, 이는 발송 시점의 네트워크 재시도 추적용(`delivery_obligations`, `migrations/0006_delivery_obligations.sql:1-14`)일 뿐 사람이 승인하거나 draft를 검토/보류할 수 있는 outbox 큐가 아닙니다.

3. **중앙 발송 액션 모델의 무조건 즉시 발송 구조**
   - `src/models/events.rs:346-384` (`enum OutboundAction`): `SendMessage`, `EditMessage`, `UploadFile`, `ApprovalRequest` 등이 정의되어 있습니다.
   - `src/multiplexer/actor.rs:88-100` 및 `src/discord/adapter.rs:6330-6360`: `dispatch(OutboundAction::SendMessage)`가 호출되면 Discord 채널로 즉시 전송되며 중간 차단/버퍼링 계층이 없습니다.

4. **선행 계획서(`autonomous-agent-platform-plan.md`)의 오류 및 현행 차이점**
   - **기존 계획서 주장**: G6 섹션에서 `src/tools/mod.rs`를 effect/outbox 수정 지점으로 명시함.
   - **현행 코드 실태**: 실제 omon-gateway의 외부 발송은 tool call 레벨이 아니라 `OutboundAction`(`src/models/events.rs:346`) 및 `OutboundDispatcher`(`src/multiplexer/actor.rs:88`)를 통해 에이전트 턴 완료(`src/agent/omo_backend.rs:646`)나 크론 스케줄러(`src/cron/scheduler.rs:2414`)에서 직접 디스패치됩니다. 따라서 outbox는 도구 레벨(`src/tools/`)뿐 아니라 `OutboundDispatcher` 및 크론 `deliver` 파이프라인의 진입점에 직접 통합되어야 합니다.

---

## 구현 단계

### Step 1: `destinations` 불변 레지스트리 및 모호성 차단 엔진
- **수정/신규 파일**:
  - `src/policy/destination.rs` (신규)
  - `src/models/destination.rs` (신규)
  - `src/models/mod.rs`, `src/lib.rs` (모듈 등록)
- **DB 마이그레이션**:
  - `migrations/0045_destinations.sql`
    ```sql
    CREATE TABLE IF NOT EXISTS destinations (
        id TEXT PRIMARY KEY NOT NULL, -- UUID or deterministic canonical ID (e.g. 'dest:discord:123456789')
        platform TEXT NOT NULL,       -- 'discord', 'slack'
        external_id TEXT NOT NULL,    -- canonical snowflake/channel string
        thread_id TEXT,               -- optional thread snowflake
        label TEXT NOT NULL,          -- human friendly label e.g. 'cfo-private'
        sensitivity TEXT NOT NULL CHECK (sensitivity IN ('public', 'team', 'private', 'restricted')),
        is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        UNIQUE(platform, external_id, thread_id)
    );
    CREATE INDEX IF NOT EXISTS idx_destinations_label ON destinations(label);
    ```
- **킬 스위치**: `OMON_DESTINATION_BINDING_ENABLED` (기본값 `false`)
- **수용 시나리오 (Acceptance Scenario)**:
  - CLI 명령 또는 쿼리로 동일한 `label`('Exec-team')을 가진 서로 다른 채널 2개를 등록한 뒤, 이름 기준 해석 API(`DestinationResolver::resolve_by_label("Exec-team")`) 호출 시 `Err(AmbiguousDestinationError)`를 반환하며 즉시 fail-closed 처리됨을 확인.
  - 검증 SQL:
    ```bash
    sqlite3 ~/.omon/state.db "SELECT count(*) FROM destinations WHERE label = 'Exec-team';" # 결과 2
    ```
  - 검증 로그:
    `"destination resolution failed: ambiguous label 'Exec-team' matches multiple immutable destinations"`

### Step 2: `outbox` 테이블 및 Draft 버퍼링 서비스 구현
- **수정/신규 파일**:
  - `src/outbox/mod.rs` (신규)
  - `src/outbox/service.rs` (신규)
  - `src/outbox/model.rs` (신규)
- **DB 마이그레이션**:
  - `migrations/0046_outbox.sql`
    ```sql
    CREATE TABLE IF NOT EXISTS outbox (
        id TEXT PRIMARY KEY NOT NULL,
        destination_id TEXT NOT NULL,
        source_kind TEXT NOT NULL CHECK (source_kind IN ('cron', 'lane', 'agent_turn', 'kibitzer')),
        source_id TEXT NOT NULL,
        content TEXT NOT NULL,
        payload_hash TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('draft', 'pending_approval', 'approved', 'dispatched', 'rejected', 'expired')),
        approval_id TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        scheduled_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        dispatched_at TEXT,
        last_error TEXT,
        FOREIGN KEY (destination_id) REFERENCES destinations(id) ON DELETE RESTRICT
    );
    CREATE INDEX IF NOT EXISTS idx_outbox_status ON outbox(status, scheduled_at);
    ```
- **킬 스위치**: `OMON_OUTBOX_ENABLED` (기본값 `false`)
- **수용 시나리오 (Acceptance Scenario)**:
  - outbox 큐잉 서비스 호출 시 `outbox` 테이블에 `status = 'draft'` 상태로 레코드가 생성되고 실제 디스패처가 발송하지 않음.
  - 검증 SQL:
    ```bash
    sqlite3 ~/.omon/state.db "SELECT status, destination_id FROM outbox WHERE source_kind = 'cron' ORDER BY created_at DESC LIMIT 1;"
    # 기대 출력: draft|dest:discord:123456789
    ```

### Step 3: 크론 스케줄러 및 디스패처 연동
- **수정/신규 파일**:
  - `src/cron/scheduler.rs`: `deliver`(`2367-2425`) 함수 내에서 `OMON_OUTBOX_ENABLED=true`인 경우 즉시 `dispatcher.dispatch` 대신 `OutboxService::enqueue_draft` 호출로 분기. `delivery_destinations`(`2535-2615`)에서 문자열 파싱 결과를 `destinations` 레지스트리의 불변 ID로 매핑.
  - `src/multiplexer/actor.rs`: 비대화형 자동화 세션에서 발송되는 `OutboundAction::SendMessage`를 Outbox 필터 통과 후 승인된 것만 전송하도록 연결.
- **수용 시나리오 (Acceptance Scenario)**:
  - 크론 잡 실행 시 로그에 발송 완료 대신 Outbox draft 인큐 로그가 찍히며 실제 Discord 채널에 메시지가 나가지 않음.
  - 검증 로그:
    `"cron delivery buffered to outbox draft: id=outbox:cron:... destination=dest:discord:..."`

---

## 위험·의존성

1. **상호 의존성 (Theme Dependencies)**
   - **`approval-effects` (P0 필수 선행/동반 의존)**: outbox에 저장된 draft가 언제 자동으로 나가고 언제 사람의 승인(Pending)을 요구할지 결정하는 정책 엔진(`Effect::SendExternal`, `Effect::Draft`)과 `sha256(content)` 기반 승인 바인딩이 필요합니다.
   - **`approval-grants` (선행 의존)**: 오너가 특정 채널(예: 개인 모니터링 채널)에 대해 사전에 부여한 standing grant가 있다면 outbox에서 즉시 `approved -> dispatched`로 자동 전이할 수 있어야 하므로 승인 허가부와 밀접하게 연동됩니다.
   - **`notify-envelope` & `channels` (후행 연계)**: 보류된 outbox draft를 오너에게 카드나 메시지 형태로 노출하고 승인 버튼을 제공하는 UI/채널 어댑터(Discord Approval 버튼, Slack 어댑터)에 의존합니다.
   - **`taint` (보안 연계)**: 인박스(이메일, 카카오톡 등)에서 들어온 오염된 데이터로 생성된 크론/에이전트 결과물은 outbox에서 상시 `approval_required`로 강제 격상되어야 합니다.

2. **단일 오너 환경에서의 특수 위험 요소**
   - **승인 피로감(Approval Fatigue) 및 침묵 시간대(KST 0~8시) 교착**: 모든 외부 발송을 outbox에서 대기시킬 경우 정기 크론(아침 브리핑, 모니터링 알림)이 전부 정체되어 오너가 승인 버튼을 일일이 눌러야 하는 운영 마비가 올 수 있습니다. 민감도(`sensitivity = 'private'`)와 사전 인가된 `destination_id`에 한해서는 자동 릴리즈(auto-dispatch)되는 allowlist 바이패스 규칙이 필수적입니다.
   - **기존 크론 JSON 하위 호환성 깨짐**: 수많은 기존 등록 크론 잡의 `deliver` 필드가 raw channel ID(`123456789`)로 되어 있으므로, 마이그레이션 시 기존 ID를 `destinations` 테이블로 자동 등록하는 fallback 백필 로직이 반드시 선행되어야 합니다.

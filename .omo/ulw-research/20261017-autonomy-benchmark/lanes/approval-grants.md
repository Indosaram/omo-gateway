# 벤치마크 및 구현 결정 보고서: 지속형 승인 및 권한 부여 (approval-grants / G5)

- 작성일: 2026-10-17
- 대상 시스템: `omon-gateway` (macOS 상주 단일 소유자 자율 에이전트 데몬)
- 테마: `G5 persistent approval requests/grants bound to payload hash with Once/Task/Until/Standing scopes, out-of-band cards, lanes currently reject approvals`

---

## ## 판정

**채택 (adopt now)**

omon-gateway는 macOS 단일 오너의 로컬 워크스페이스, iMessage, 카카오톡, 크론, 자율 실행 레인(lane)을 통합 운영하는 실기기 데몬입니다. 현재 게이트웨이는 동기식 인메모리 Discord 버튼 승인만 지원하며(`src/discord/approval.rs:580`), 무인 백그라운드 작업 단위인 레인(lane)에서 승인 요청이 발생하면 이를 큐잉하거나 대기하지 않고 즉시 에러(`src/lanes/runner.rs:661-665`, `"lane approval requires lead intervention"`)로 폐기합니다. 또한 프로세스 재기동 시 대기 중인 모든 승인이 증발하고, 승인 allowlist 테이블은 TTL·주체·해시 바인딩이 없는 취약한 영구 허용(`migrations/0007_approval_allowlist.sql:1-4`)에 의존하고 있습니다.

단일 소유자의 수면 시간(KST 00~08시 방해 금지) 및 비동기 작업 특성상, 레인 및 크론이 작업을 중단하지 않고 비동기 승인 원장에 요청을 영속화(Payload hash 바인딩)한 뒤 소유자가 Discord·대시보드·iMessage 등 다채널에서 안심하고 스코프(Once/Task/Until/Standing)별로 인가할 수 있는 지속형 승인 아키텍처는 자율 에이전트 안전망의 필수 선결 과제입니다.

---

## ## 벤치마킹 원천

| 플랫폼 | 원천 메커니즘 및 벤치마크 패턴 | 출처 등급 |
|---|---|---|
| **Meta Muse** | **Sentinel 결정 엔진 & Out-of-band 승인 카드**: Sentinel이 정책 평가(`Decision: Allow / Deny / Ask(ApprovalId)`)를 수행하며, 승인은 모델 대화창 텍스트가 아닌 Out-of-band 카드 형태로 격리 전달. 승인 상태 머신(`Proposed → Pending → Approved / Denied / Expired → Executing → Settled / Failed`)을 영속화하고, `Scope`를 `Once`, `Session`, `Task(id)`, `Until(Instant)`, `Perpetual`로 세분화. 호출 인자 해시 및 대상 커넥터에 엄격히 바인딩하여 replay 방지 및 자체 승인(self-approval bypass, Tier 7 현상금 대상) 원천 차단. | `[1차]` / `[2차]` |
| **OpenAI Dots** | **3-List 정책 모델 & 비동기 Hand-off**: 커넥터 및 앱별로 `Allow / Require approval / Block` 3개 리스트로 규칙을 선언. 작업 성격을 읽기 전용(Proactive lane)과 변경/외부발송 작업으로 엄격히 분리하고, 변경 작업 필요 시 비동기 승인 핸드오프 카드를 사용자에게 푸시한 뒤 작업 상태를 보존(대기). 사용자 승인 피로(approval fatigue)를 줄이기 위해 행위 클래스(Action class) 단위로 스코프 관리. | `[1차]` / `[2차]` |
| **Hark** | **소유자 채널 동결 인가 & Hash-bound replay 방지**: 소유자 채널(Discord, iMessage, 카카오톡)로 변경 행위의 diff 및 페이로드를 담은 카드 발송. 각 승인은 정확한 액션 페이로드의 암호학적 해시(SHA-256)에 영속 바인딩되어 재사용을 방지하며, 응답 타임아웃 시 기본값 거절(default-deny) 강제. 크론/백그라운드 잡에 대해 불변 인가 스코프 부여. | `[1차]` / `[2차]` |
| **xAI Grok Bot** | **인간 개입 일시정지(Pause & Resume)**: 비가역적 외부 작용 시 작업을 즉시 에러 처리하지 않고 중단(Suspended) 상태로 스냅샷 저장 후, 관리자 승인 이벤트 도착 시 실행 컨텍스트 복원. | `[2차]` |

---

## ## 현재 코드

현재 omon-gateway 코드베이스를 직접 열어 분석한 결과와 사전 계획 문서(`docs/research/autonomous-agent-platform-plan.md`)와의 차이점 및 한계는 다음과 같습니다.

### 1. 인메모리 oneshot 기반 승인 가드 및 프로세스 재기동 시 유실
- `src/discord/approval.rs:580`: `SmartApprovalGuard`는 `HashMap<Uuid, PendingApprovalEntry>`와 `tokio::sync::oneshot::Sender<ApprovalDecision>`을 메모리에 유지합니다.
- `src/discord/approval.rs:18-35`: 지원 결정 타입은 `Once`, `Session`, `Always`, `Deny` 4종에 불과하며, Task 단위나 만료 시각(Until) 기반 인가는 없습니다.
- `src/discord/approval.rs:177-181`: `ApprovalScope` 열거형은 `Once`, `Session`, `Always` 3개만 정의되어 있습니다.
- `src/dashboard.rs:2358`: 대시보드 역시 `DETACHED_APPROVALS_WARNING` ("this dashboard runs its own approval guard; approvals pending in the messenger gateway are not listed here...")을 명시하며, 메모리 분리 시 게이트웨이 승인이 대시보드에 연동되지 못하는 메모리 결합 구조를 드러내고 있습니다.
- 데몬 프로세스가 재시작되거나 crash되면 대기 중이던 모든 승인 요청이 `Cancelled` 또는 타임아웃되어 작업 흐름이 영구 단절됩니다.

### 2. 레인(Lanes) 실행 중 승인 요청 원천 거절
- `src/lanes/runner.rs:661-665`:
  ```rust
  OutboundAction::ApprovalRequest { .. } => {
      return Err(OmonError::Config(
          "lane approval requires lead intervention".into(),
      ));
  }
  ```
  현재 레인 실행기(`LaneCaptureDispatcher`)는 에이전트 턴 중 발생한 `ApprovalRequest`를 만나면 비동기 승인 큐에 등록하지 않고 즉각 설정 에러를 반환하며 작업을 강제 종료시킵니다. 이로 인해 레인은 사실상 변경 승인이 필요한 작업을 자율적으로 수행할 수 없습니다.

### 3. 단일 주체 및 취약한 allowlist 테이블
- `migrations/0007_approval_allowlist.sql:1-4`:
  ```sql
  CREATE TABLE IF NOT EXISTS approval_allowlist (
      pattern_key TEXT PRIMARY KEY NOT NULL,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
  ```
  기존 DB 테이블은 단순 문자열 `pattern_key`만 보관하며 만료 시각(`expires_at`), 부여 주체(`granted_by`), 허용 스코프(`scope`), 바인딩된 페이로드 해시(`payload_hash`), 대상 채널/세션 식별자가 전혀 없습니다.

### 4. 위험 명령 키 파싱의 불완전성
- `src/security/dangerous.rs:393-419`: `derive_pattern_key`는 정규식 매칭 기반(`SQL DELETE without WHERE` 등)이거나 SHA-256 다이제스트를 결합하지만, 정형화된 Effect 타입(FileWrite, SendExternal, ExecCommand 등)이나 구조화된 인자 매처가 부재합니다.

### 5. 에이전트 턴 레벨의 자동 승인 우회
- `src/agent/omo_backend.rs:1456-1473`:
  ```rust
  let auto_approve = !matches!(
      std::env::var("APPROVAL_MODE")
          .or_else(|_| std::env::var("APPROVAL_POLICY"))
          .as_deref(),
      Ok("always")
  );
  if !toolset_blocked && auto_approve {
      let _ = ws.send(approval_allow_response(req_id)).await;
      continue;
  }
  ```
  환경변수가 `always`로 지정되지 않은 단일 사용자 세션의 경우 데몬 도구 요청을 전면 자동 승인(`auto_approve`) 처리하여, 백엔드 내부 승인 게이트가 사실상 무력화되어 있습니다.

### 6. 사전 계획 문서의 오류 및 변경점 정정
- 사전 계획(`autonomous-agent-platform-plan.md` 4절, 6절 P0)에서는 `approval_requests` 및 `approval_grants` 테이블이 이미 구상되었거나 일부 존재하는 것처럼 기술되었으나, 실제 소스 트리와 마이그레이션(`migrations/` 1번~44번) 전수 조사 결과 **해당 테이블은 현재 전혀 존재하지 않으며**, 오직 `migrations/0007_approval_allowlist.sql` 1개만 존재합니다.
- 사전 계획에서 `src/discord/approval.rs:239`를 Discord 전용 한계로 지목한 것은 정확하며, 추가로 `src/main.rs:2134-2136`의 headless dispatcher 역시 `ApprovalRequest`를 만나면 `"headless output has no approval recipient"` 에러로 거절하고 있음을 확인했습니다.

---

## ## 구현 단계

본 구현은 기존의 실시간 Discord 버튼 상호작용을 파괴하지 않으면서, SQLite 기반의 지속형 승인 원장(`approval_requests`, `approval_grants`)을 추가하고 레인(lane)이 승인 대기 시 중단되지 않고 `Blocked(ApprovalId)` 상태로 안전하게 전환되도록 단계별로 구축합니다.

### 환경 변수 킬 스위치
- `OMON_PERSISTENT_APPROVALS_ENABLED` (기본값: `false`)
- `false`일 때는 기존 인메모리 `SmartApprovalGuard` 경로 유지, `true`일 때 지속형 DB 원장 및 레인 비동기 일시정지 경로 활성화.

---

### Step 1: 데이터베이스 스키마 마이그레이션
- **작업 내용**: 지속형 승인 요청 원장(`approval_requests`) 및 권한 부여 원장(`approval_grants`) 생성.
- **신규 파일**: `migrations/0045_approval_grants.sql`
- **테이블 정의**:
  ```sql
  CREATE TABLE IF NOT EXISTS approval_requests (
      id TEXT PRIMARY KEY NOT NULL,             -- UUIDv4
      source_kind TEXT NOT NULL,                 -- 'session', 'lane', 'cron'
      source_id TEXT NOT NULL,                   -- session_key or lane_key
      effect_type TEXT NOT NULL,                 -- 'exec_command', 'send_external', 'fs_write'
      action_json TEXT NOT NULL,                 -- 직렬화된 구체적 액션 페이로드
      payload_hash TEXT NOT NULL,                -- SHA-256(action_json)
      status TEXT NOT NULL DEFAULT 'pending',    -- 'pending', 'approved', 'denied', 'expired'
      requested_scope TEXT NOT NULL,             -- 'once', 'task', 'until', 'standing'
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      expires_at TIMESTAMP NOT NULL,             -- 기본 생성 + 1800초 (30분)
      decided_at TIMESTAMP,
      decided_by TEXT,                           -- 'discord:<user_id>', 'dashboard', 'imessage:<addr>'
      decision_scope TEXT,                       -- 실제 인가된 스코프 ('once', 'task', 'until', 'standing')
      reason TEXT
  );
  CREATE INDEX IF NOT EXISTS idx_approval_requests_pending ON approval_requests(status, expires_at);
  CREATE INDEX IF NOT EXISTS idx_approval_requests_source ON approval_requests(source_kind, source_id);

  CREATE TABLE IF NOT EXISTS approval_grants (
      id TEXT PRIMARY KEY NOT NULL,
      scope_kind TEXT NOT NULL,                  -- 'once', 'task', 'until', 'standing'
      source_pattern TEXT NOT NULL,              -- 특정 lane_id, session_key 또는 '*'
      effect_type TEXT NOT NULL,
      matcher_json TEXT NOT NULL,                -- 조건 매칭 패턴 (e.g. prefix, regex, destination)
      payload_hash TEXT,                         -- Once/Task용 고정 해시 바인딩
      created_from_request TEXT REFERENCES approval_requests(id),
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      expires_at TIMESTAMP,                      -- Until/Standing 만료일 (null=무제한)
      revoked_at TIMESTAMP
  );
  CREATE INDEX IF NOT EXISTS idx_approval_grants_lookup ON approval_grants(effect_type, scope_kind, revoked_at);
  ```
- **수락 검증 시나리오**:
  ```bash
  # SQLite CLI 검증: 마이그레이션 적용 후 테이블 생성 확인
  sqlite3 ~/.omon/state/omon.db ".schema approval_requests" | grep -E "payload_hash|requested_scope"
  sqlite3 ~/.omon/state/omon.db ".schema approval_grants" | grep -E "matcher_json|revoked_at"
  ```

---

### Step 2: 도메인 모델 및 저장소 계층 구현
- **수정 및 생성 파일**:
  - 생성: `src/security/approval_store.rs`
  - 수정: `src/security/mod.rs`
- **구현 내용**:
  - `ApprovalRequestRecord`, `ApprovalGrantRecord`, `ApprovalScope`(`Once`, `Task(String)`, `Until(DateTime<Utc>)`, `Standing`) 정의.
  - 페이로드 SHA-256 해시 계산 유틸리티 (`hash_payload(&serde_json::Value) -> String`).
  - `create_request()`, `find_matching_grant()`, `decide_request()`, `expire_stale_requests()` 구현.
- **수락 검증 시나리오**:
  ```bash
  # SQL 쿼리 검증: 더미 요청 등록 후 해시 일치 여부 확인
  sqlite3 ~/.omon/state/omon.db "INSERT INTO approval_requests (id, source_kind, source_id, effect_type, action_json, payload_hash, expires_at, requested_scope) VALUES ('test-uuid', 'lane', 'lane-1', 'exec_command', '{\"cmd\":\"rm -rf /tmp/a\"}', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', datetime('now', '+30 minutes'), 'once');"
  sqlite3 ~/.omon/state/omon.db "SELECT id, status, payload_hash FROM approval_requests WHERE id='test-uuid';"
  # 출력: test-uuid|pending|e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  ```

---

### Step 3: 레인 실행기(Lane Runner) 비동기 일시정지(Blocked) 연동
- **수정 파일**:
  - `src/lanes/runner.rs:661-665`
  - `src/lanes/types.rs` (또는 `LaneStatus::Blocked` 지원 확장)
- **구현 내용**:
  - `OMON_PERSISTENT_APPROVALS_ENABLED=true` 환경에서 `OutboundAction::ApprovalRequest` 수신 시 즉각적인 에러 반환을 중지.
  - `approval_requests` 테이블에 `source_kind='lane'`, `source_id=session.channel_id`로 요청 삽입.
  - 레인 상태를 `LaneStatus::Blocked { approval_id, reason }`으로 변경하고, 레인 실행 턴을 일시 정지(yield).
  - 추후 소유자 승인 시 `LaneActivity` 또는 이벤트 큐를 통해 `LaneStatus::Running`으로 재개.
- **수락 검증 시나리오**:
  ```bash
  # 로그 라인 검증: 레인이 approval 발생 시 에러 종료되지 않고 블록 대기 상태로 진입하는지 확인
  tail -n 20 ~/.omon/logs/omon-gateway.log | grep -E "lane blocked waiting for persistent approval|approval_requests id="
  ```

---

### Step 4: 다채널 승인 및 대시보드 API 결합
- **수정 파일**:
  - `src/discord/approval.rs:634-649`: Discord 버튼에 스코프 선택지 확장(`Once`, `Task`, `Standing`, `Deny`).
  - `src/dashboard.rs:2378-2411`: `resolve_approval` 핸들러가 인메모리 뿐만 아니라 `approval_requests` DB 상태를 갱신하도록 통합.
  - `src/discord/adapter.rs:4046`: 승인 요청 메시지에 페이로드 해시 축약본(`hash8`) 및 만료 시간 명시.
- **수락 검증 시나리오**:
  ```bash
  # cURL 검증: 대시보드 API를 통한 비동기 지속형 승인 해결
  curl -s -X POST http://127.0.0.1:19741/api/approvals/test-uuid/resolve \
    -H "Content-Type: application/json" \
    -d '{"decision": "allow_once", "scope": "once"}' | grep '"resolved":true'
  # DB 상태 검증
  sqlite3 ~/.omon/state/omon.db "SELECT status, decided_by, decision_scope FROM approval_requests WHERE id='test-uuid';"
  # 출력: approved|dashboard|once
  ```

---

### Step 5: 만료 가비지 컬렉터 및 야간(KST 00~08시) 알림 억제
- **수정 파일**:
  - `src/cron/system_jobs.rs` 또는 백그라운드 워처
- **구현 내용**:
  - 만료 시간(`expires_at`)이 지난 pending 요청을 자동으로 `expired` 처리하여 무한 대기 차단.
  - 소유자 수면 시간인 KST 00:00~08:00 사이에는 긴급 보안 차단 외의 일반 승인 핑 발송을 대기(배치 큐잉)하고 아침 08:00에 요약 브리핑.
- **수락 검증 시나리오**:
  ```bash
  # 만료 쿼리 검증: 만료된 요청이 expired 상태로 전이되는지 확인
  sqlite3 ~/.omon/state/omon.db "UPDATE approval_requests SET expires_at=datetime('now', '-1 minute') WHERE id='test-uuid-expired';"
  # 게이트웨이 만료 tick 수행 후 상태 확인
  sqlite3 ~/.omon/state/omon.db "SELECT status FROM approval_requests WHERE id='test-uuid-expired';"
  # 출력: expired
  ```

---

## ## 위험·의존성

### 1. 상호 연관 테마 의존성
- **`approval-effects` (선행 의존)**: 페이로드 해시를 생성하고 적합한 스코프를 판단하기 위해서는 어떤 액션이 승인을 요구하는지 표준화된 Effect 분류(`Spend`, `SendExternal`, `FileWrite`, `CommandExec`, `HardlineDeny`)가 먼저 정의되어야 합니다.
- **`taint` (강한 결합)**: Meta Muse 패턴에서 보듯, 외부 인박스나 웹 검색을 통해 오염된(tainted) 세션/레인은 Standing(영구 허용) 권한을 획득할 수 없고 오직 Once(1회용 해시 바인딩)만 부여받을 수 있도록 제약되어야 합니다.
- **`event-queue` (재개 의존)**: 레인이 `Blocked` 상태에서 승인을 받은 후 다시 에이전트 턴을 가동하려면 `LaneWake` 이벤트를 신뢰성 있게 전달할 지속형 이벤트 큐가 필요합니다.
- **`kibitzer-action` (감시 연동)**: 키비처(kibitzer)가 자율 제안한 액션이나 신규 레인 시작 작업에 대한 승인 원장 연계.
- **`notify-envelope` & `outbox-destination` (채널 격리)**: 승인 요청 카드가 소유자의 안전 채널(Discord DM, 인증된 대시보드)로만 라우팅되고, 잘못된 공개 채널로 누출되지 않도록 전송지 바인딩이 보장되어야 합니다.

### 2. 운영 위험 및 대응책
1. **소유자 승인 피로(Approval Fatigue)**:
   - *위험*: 사소한 파일 수정이나 읽기 작업마다 승인을 요구하면 소유자가 무조건 허용(Always)을 누르게 되어 보안 모델이 붕괴됨.
   - *대응책*: 읽기 전용 작업은 전면 무인 통과시키고, 비가역적 외부 작용(`SendExternal`, 위험 쉘 명령어) 및 신규 대상 발송에 한해서만 승인을 요청. `Task` 스코프를 도입하여 단일 레인 실행 동안 동일한 명령 패턴의 반복 승인을 생략.
2. **프로세스 충돌 및 레이스 컨디션**:
   - *위험*: 복수의 인터페이스(Discord 버튼 클릭과 웹 대시보드 클릭)에서 동일한 요청을 동시에 승인/거절할 경우의 정합성 문제.
   - *대응책*: SQLite 단일 트랜잭션 내에서 `UPDATE approval_requests SET status = 'approved' WHERE id = ? AND status = 'pending'`의 영향받은 행 수가 1인 경우에만 인가를 확정하는 낙관적 락(optimistic concurrency control) 적용.
3. **오프라인/야간 작업 지연**:
   - *위험*: 소유자가 부재 중이거나 취침 중(KST 00~08시)일 때 레인이 블록되어 크론이나 긴급 백그라운드 태스크가 타임아웃 종료됨.
   - *대응책*: 레인의 타임아웃 시계를 `Blocked` 상태 동안 일시 중지(Pause)하고, 긴급하지 않은 작업은 아침 08:00 브리핑 후 일괄 처리되도록 생명주기 관리.

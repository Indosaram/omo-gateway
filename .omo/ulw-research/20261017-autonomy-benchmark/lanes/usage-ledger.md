# 벤치마크 및 구현 결정 리포트: usage-ledger (G13 비용·행동 감사 원장)

## 판정
**채택 (Adopt Now - 행동 감사 및 폭주 상한에 한정하여 선별 채택)**

오너의 핵심 가치("토큰 사용량 집계는 불필요", "결과 중심 기계 검증", "폭주 비용 방지")에 부합하도록, 단순 통계성 토큰 리포팅 파이프라인은 배제하고 **레인·세션 단위 폭주 방지 상한(Runaway Cap) 및 외부 영향 행동 감사 원장(Action Audit Ledger)**만을 즉시 구현 대상으로 채택한다. 단일 오너의 로컬 Mac 환경에서 백그라운드 크론/레인이 무한 루프에 빠져 API 크레딧을 소진하거나 통제 불능의 셸/외부 발송을 실행하는 사고는 즉각적인 금전적·시스템적 손실을 유발하므로, 실행 횟수/시간/위험 도구 호출을 추적하고 차단하는 가드레일이 필수적이다.

---

## 벤치마킹 원천
1. **OpenAI Dots (도츠)**
   - **사례 및 패턴**: 사용자 프롬프트가 전혀 없는 상태에서 서브에이전트를 자체 생성하며 하루 약 307,000 토큰(155개 activity trace, 2분간 최소 50개 trace)을 소모한 자율 폭주 사고 보고 **[2차]** (`.omo/ulw-research/20261016-133102-autonomous-agents/reduce/r-platforms-b.md:86`, `dots-safety.md:34, 91`, The Register 보도).
   - **시사점**: 백그라운드 자율 에이전트에 엄격한 턴(Turn) 상한, 단위 시간당 활동(Activity) 상한, 세션 수명 제한이 없으면 방치 시 비용 폭주가 발생함.

2. **xAI Grok Bot (그록 봇)**
   - **사례 및 패턴**: 가벼운 태스크를 수행하는 도중 불과 10분 만에 사용자의 API 크레딧 $50를 전액 소진($27 첫 실측 후 순식간에 $50 한도 도달)한 루프 사고 보고 **[1차]** (`.omo/ulw-research/20261016-133102-autonomous-agents/wave1/grokbot-critique.md:28`, Cursor 포럼 실측 보고).
   - **시사점**: 모델 호출 직전/직후의 rate-limit, 턴당 토큰/호출 상한, 비용 이상 급증 감지 및 회로 차단기(Circuit Breaker) 부재 시 파멸적 비용 발생.

3. **Hermes Agent (헤르메스 에이전트)**
   - **사례 및 패턴**: 크론 사전 점검(Preflight)을 통해 LLM 호출 전에 키, 스킬, 전달 대상의 유효성을 기계적으로 사전 검증하여 불필요한 LLM 호출 비용을 0으로 억제 **[1차]** (`.omo/ulw-research/20261016-133102-autonomous-agents/reduce/r-cross.md:26`, Hermes cron documentation).
   - **시사점**: LLM 호출 자체를 줄이는 게이트웨이 레벨의 사전 필터 및 비용 회피 설계가 최우선 감사 대상임.

---

## 현재 코드
현재 저장소의 비용 및 사용량, 감사 관련 코드 현황은 다음과 같다:

1. **`migrations/0039_model_usage.sql:1-17`**
   - `model_usage` 테이블(`id`, `session_key`, `purpose`, `model`, `input_tokens`, `output_tokens`, `cached_tokens`, `created_at`) 및 인덱스가 선언되어 있음.
   - `lane_usage_context` 테이블(`lane_key`, `start_commit`, `purpose`)이 선언되어 있음.
   - **괴리/오류 지점**: 전체 코드베이스(`src/`) 어디에서도 `model_usage` 테이블에 레코드를 삽입(`INSERT`)하거나 조회(`SELECT`)하는 코드가 전무함(Writer 미구현 고아 테이블).

2. **`src/usage.rs:1-252`**
   - `Purpose` 열거형(`Lead`, `LaneLarge`, `LaneSmall`, `Kibitzer`, `InboxWrite`, `Digest`, `JudgeFallback`, `CronFix`)이 정의되어 있음(`src/usage.rs:9-33`).
   - `lane_usage_context`에 시작 커밋을 기록(`src/usage.rs:189`)하고 세션의 용도를 매핑하는 `session_purpose`(`src/usage.rs:197-227`)가 구현되어 있음.
   - `report` 및 `yesterday` 함수(`src/usage.rs:45-91`, `src/usage.rs:229-251`)는 `model_usage`를 전혀 보지 않으며, 오직 `judge_calls` 테이블과 `inbox_items`/`inbox_filter_stats` 테이블만 조회하여 jev 호출 횟수 및 폴백 횟수, 상한 도달 여부만 리포팅함.

3. **`src/agent/omo_backend.rs:1660-1845`**
   - 백엔드의 턴 완료 핸들러(`turn/completed`)에서 세션 메타데이터 정리, 상태 디스코드 릴레이, cron ack 커맨드 실행만 수행하고 있으며 토큰 사용량이나 실행 비용, 모델 호출 횟수를 기록하는 로직이 완전히 누락되어 있음.

4. **`src/lanes/runner.rs:30, 806, 1243-1247`**
   - 레인 레벨의 유일한 폭주 가드레일로 `DEFAULT_MAX_TURNS = 24`(`src/lanes/runner.rs:30`) 및 `lane.turns_run >= self.max_turns` 도달 시 `turn_cap` 실패 처리(`src/lanes/runner.rs:1243-1247`)가 구현되어 있음.
   - 하지만 시간 단위(예: 10분 내 N회 턴), 도구 실행 횟수 폭주, 크론 세션에 대한 턴 제한은 부재함.

5. **`src/judge/mod.rs:756-850, 1102, 1254`**
   - Jev 판정에 대해서는 일일 호출 상한(`judge_calls` 기반)이 구현되어 상한 도달 시 `capped:...` 결과로 안전 폴백 처리됨(`src/judge/mod.rs:801, 1102`).

6. **감사 원장(`action_audit`) 부재**
   - `src/tools/terminal.rs:205-307`에서 위험 명령어 승인/차단을 검사하고 실행하지만, 승인/거부/실행 이력을 영속적으로 기록하는 `action_audit` 테이블이 존재하지 않아 사후 검증이 불가능함.

---

## 구현 단계

오너 지침("토큰 사용량 집계는 불필요, 비용 상한 및 액션 감사 중심")에 맞춘 구체적 구현 단계:

### 1단계: 위험 액션 감사 원장(`action_audit`) 스키마 및 기록 추가
- **변경/생성 파일**:
  - `migrations/0045_action_audit.sql` (신규 생성)
  - `src/audit.rs` (신규 생성)
  - `src/lib.rs` (`pub mod audit;` 등록)
  - `src/tools/terminal.rs:300-360` (명령어 실행 및 승인 결과 감사 로그 기록)
- **DB 테이블 (`migrations/0045_action_audit.sql`)**:
  ```sql
  CREATE TABLE action_audit (
      id TEXT PRIMARY KEY,
      session_key TEXT NOT NULL,
      actor TEXT NOT NULL,           -- 'agent', 'cron', 'human'
      action_kind TEXT NOT NULL,      -- 'terminal_exec', 'file_write', 'external_send'
      target TEXT NOT NULL,          -- command line or target path
      payload_hash TEXT NOT NULL,    -- sha256 of args
      decision TEXT NOT NULL,        -- 'allowed', 'denied', 'auto_approved'
      outcome TEXT NOT NULL,         -- 'success', 'failed:exit_code', 'error'
      created_at DATETIME NOT NULL
  );
  CREATE INDEX idx_action_audit_session ON action_audit(session_key, created_at);
  ```
- **킬 스위치**: `OMON_ACTION_AUDIT_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  - CLI: `cargo test --test audit_record` (통합 테스트 통과)
  - SQL:
    ```bash
    sqlite3 omon_gateway.db "SELECT count(*) FROM action_audit WHERE action_kind='terminal_exec' AND decision='allowed';"
    ```
    출력 결과 1 이상 확인.

### 2단계: 세션 및 레인 급증 폭주 차단기(Runaway Circuit Breaker)
- **변경/생성 파일**:
  - `src/usage.rs:120-180` (급증 속도 검사기 `RunawayGuard` 추가)
  - `src/agent/omo_backend.rs:1560-1610` (도구 연속 호출 및 턴 빈도 검사)
  - `src/lanes/runner.rs:1240-1250` (10분 윈도우 내 턴 수 상한 검사)
- **차단 규칙**:
  - 단일 세션에서 10분 내 15턴 이상 연속 호출 시 강제 차단 (`runaway_turns_exceeded`)
  - 단일 턴 내 도구 호출 50회 초과 시 턴 강제 중단 (`runaway_tool_calls_exceeded`)
- **킬 스위치**: `OMON_RUNAWAY_CAP_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  - 단위 테스트: `RunawayGuard`에 10분 내 16회 턴 진입 시 `Err(OmonError::RunawayCapped)` 반환 확인.
  - 로그:
    ```bash
    grep "runaway turn cap reached" /Users/indo/.omon/logs/gateway.log
    ```

### 3단계: 고아 `model_usage` 테이블 정리 또는 최소 호출 카운터 전환
- **방침**: 오너가 토큰 집계를 불필요하다고 지시했으므로 무거운 토큰 파싱/집계 라이터는 작성하지 않으며, `model_usage` 테이블은 턴 실행 메트릭(모델별 호출 횟수, 턴 성공/실패 여부)만 기록하는 경량 실행 레저로 축소 사용하거나 차기 정리 대상으로 둠.
- **변경 파일**:
  - `src/agent/omo_backend.rs:1770-1790` (`turn/completed` 시 호출 메트릭 1행 기록, 토큰 컬럼은 0 또는 필요 시만 채움)
- **기계 검증 시나리오**:
  - SQL:
    ```bash
    sqlite3 omon_gateway.db "SELECT count(*) FROM model_usage WHERE purpose='lane_small';"
    ```

---

## 위험·의존성

### 관련 테마 의존성
1. **`approval-effects` / `approval-grants` (P0 필수 선행/병행 의존)**
   - `action_audit`은 승인 시스템의 판정 결과(`ApprovalDecision`) 및 Effect 분류와 직접 연결된다. 정책 엔진(`src/policy/`)이 결정한 `Allow/Ask/Deny`가 `action_audit`의 `decision` 컬럼으로 기록되어야 유의미하다.
2. **`kibitzer-action` (의존)**
   - 키비처가 능동적 행동(제안을 넘어선 직접 레인 시작/도구 실행)을 취할 때, 폭주 방지 캡(`RunawayGuard`)이 적용되어 있지 않으면 백그라운드 자기 복제/연쇄 실행 위험이 발생한다.
3. **`heartbeat` (의존)**
   - 자율 에이전트가 중단되거나 캡에 걸려 멈췄을 때 이를 감지하는 헬스체크와 연계된다.

### 구현 위험 및 완화책
- **성능 오버헤드 (SQLite Lock 경합)**:
  - 도구 실행마다 `action_audit`에 동기식으로 INSERT할 경우 SQLite 쓰기 락 경합이 발생할 수 있음.
  - *완화책*: WAL 모드 활용 및 감사 로그 비동기 버퍼 채널(`tokio::sync::mpsc`)을 통한 배치 기록 처리.
- **정상적인 대규모 레인 작업 차단 (오탐)**:
  - 장시간 빌드나 대규모 코드 리팩토링 시 정상적인 턴 수가 폭주 캡에 걸릴 위험.
  - *완화책*: 10분 롤링 윈도우 기준의 '속도(Velocity)' 기반 캡을 적용하고, 오너의 명시적 재개(Reopen) 시 즉시 윈도우 카운터를 리셋.

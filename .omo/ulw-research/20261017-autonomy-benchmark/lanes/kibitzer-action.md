# 벤치마크 보고서: kibitzer-action (선제적 작업 실행)

## ## 판정

**보류(later)**

현재 omon-gateway에서 kibitzer가 자율적으로 lane을 직접 생성하거나 작업을 실행(kibitzer-action)하도록 연결하는 작업은 **P0 승인 프레임워크(approval-effects / approval-grants) 및 읽기 전용 실행 프로필(read_only_proactive / sandbox)이 구축될 때까지 보류**해야 합니다. 단일 오너의 macOS 호스트 데스크톱 환경에서 게이트웨이는 실제 셸 도구와 호스트 파일시스템 권한을 보유하고 있으며, 현재 turn 수준 승인은 사실상 비활성화되어 있습니다(`src/agent/omo_backend.rs:1456-1470`). 오너 승인 없이 백그라운드에서 임의로 파일 수정이나 외부 전송(SendExternal)이 가능한 lane이 자동 발주될 경우 시스템 파괴 및 토큰 폭주 위험이 극대화되므로, Pickle 식의 "제안 우선(suggestions-first)" 및 OpenAI Dots 식의 "read-only proactive research" 제약이 선행 격리 레이어로 마련된 뒤 P1 단계에서 단계적으로 도입해야 합니다.

---

## ## 벤치마킹 원천

| 플랫폼 | 메커니즘 및 접근 방식 | 출처 태그 및 증거 |
|---|---|---|
| **Hark (Hark Pro / Handoff)** | **Proactive Thinking & Handoff**: 이메일 및 이전 대화 관찰을 바탕으로 Action Cards/Buttons("Pay", "Cancel")를 자동 생성. 잔액 부족 감지 시 자동 충전 제안. "If you allow it" 조건 하에서 자율 해결을 수행하나 기본값은 동의(consent) 우선이며, 사용자의 선택을 점진적으로 학습하는 standing permission 적용. | [1차] https://hark.com/articles/introducing-hark-pro , https://hark.com/terms , https://hark.com/security |
| **Meta Muse** | **Goals & Salience Gate**: 선제적 행동 및 알림을 선별하기 위한 Salience Gate(돌출성 관문)와 Sentinel 승인 체계. 읽기(Read)와 쓰기(Write) 액션을 엄격히 구분하며, 쓰기 동작 및 비가역 액션(이메일 발송 등)은 오너 서명(sign off)이 반드시 요구됨. | [1차] mouse.dev muse-runtime-export, https://introducing.muse.ai , security.muse.ai |
| **Pickle** | **Suggestions-First & Action Class Gate**: 메일/캘린더를 감지하여 작업(분실 택배, 항공권 환불 등)을 먼저 **제안(Suggestions)**하고, 푸시 알림도 'suggestion alerts'로 분류. 결제 및 비가역 작업은 무조건 오너 승인 카드(주문 내역, 총액, 버튼)를 띄워 확인하며, 자율 무단 실행은 방지함. | [1차] KR App Store, https://pickle.com/assets/privacy-policy-BtYR0eAZ.js , https://pickle.com/assets/security-n1i5zLW2.js |
| **OpenAI Dots** | **Read-Only Proactive Research**: 오너가 자리에 없을 때 실행되는 선제적 리서치는 엄격하게 Read-only 도구로만 제한되며, Custom Rules로도 쓰기 제한을 해제할 수 없음. 4상태 승인 머신과 결합. | [1차] https://help.openai.com/articles/20001529 , 20001530 |
| **PRISM (학술)** | **Cost-Sensitive Selective Intervention**: 거짓 경보(False Alarm, 불필요한 개입)와 지원 누락(Missed Help)의 비대칭적 비용을 계산하여 수용 확률 임계치를 넘을 때만 개입. 개입 판단 게이트를 실행 정책과 완전 분리. | [1차] arXiv 2602.01532 (PRISM) |

---

## ## 현재 코드

현재 omon-gateway의 kibitzer 및 lane 실행 관련 구현 현황과 이전 통합 계획(`autonomous-agent-platform-plan.md`) 대비 검증 결과는 다음과 같습니다.

### 1. Kibitzer 구조 및 동작 현황
- **오너 하드코딩 및 주기 제약**: `src/lanes/kibitzer.rs:21-25`에서 기본 리드 세션 키 `DEFAULT_LEAD`(Discord DM)가 하드코딩되어 있으며, 평가 주기 간격 `SPACING`(30분) 및 `KST`(9시간) 기준 야간 침묵 시간(KST 0시~8시)이 적용됩니다(`src/lanes/kibitzer.rs:235-237`).
- **엄격한 제안 전용 프롬프트 및 도구 차단**: `src/lanes/kibitzer.rs:300-301`에서 `enabled_toolsets`를 빈 벡터(`Vec::new()`)로 설정하고, 시스템 프롬프트로 `"Return only JSON with exactly wake (boolean), reason (string), suggested (string), target_lane (string or null). Decide whether the owner needs proactive action. Do not perform actions."`를 강제하여 모델이 액션을 취하지 못하도록 제한합니다.
- **외부 액션 요청 금지**: `src/lanes/kibitzer.rs:94-98`의 `Capture` 구현체는 kibitzer 실행 중 `UploadFile`이나 `ApprovalRequest` 같은 외부 액션이 발생하면 `OmonError::Config("kibitzer cannot request external actions")` 에러를 반환합니다.
- **Jev 판사 게이트(Salience Gate)**: `src/lanes/kibitzer.rs:266-284`에서 모델 턴 호출 전에 Jev 판사를 호출(`Purpose::KibitzerWake`)하여 `"Would the owner want the agent to proactively do or ask something right now?"` 질문에 대한 점수가 `0.7` 미만이면 모델 턴 진입 없이 즉시 종료합니다.
- **이벤트 발행 및 일일 쿼터 상한**: `src/lanes/kibitzer.rs:338-364`에서 `decision.wake`가 참일 경우 일일 최대 6회까지만 `lane_events` 테이블에 `kind = 'kibitzer'` 레코드로 제안 본문을 삽입합니다.
- **이벤트 소비 경로**: `src/lanes/dispatch.rs:94,180-197`에서 `kind = 'kibitzer'` 이벤트는 lead 세션(오너의 Discord DM)의 인바운드 턴(`[LANE_EVENT]\n...`)으로 전달되어 오너에게 단순 메시지로 제안될 뿐, 자동으로 lane을 생성하거나 실행하지 않습니다.
- **스키마 초기화 위치**: `src/lanes/kibitzer.rs:180-199`의 `initialize()` 메서드 내에서 `CREATE TABLE IF NOT EXISTS kibitzer_state` 및 트리거 2종(`kibitzer_inbox_insert`, `kibitzer_lane_insert`)을 런타임 raw SQL로 직접 실행하고 있습니다.

### 2. Lane CLI 및 Start 현황
- **Lane Start 구조**: `src/lanes/cli.rs:24-30`에 `LaneCommand::Start(StartArgs)`가 정의되어 있으며, `src/lanes/cli.rs:85-101`에서 `StartArgs`(`key`, `title`, `brief`, `done`, `thread`, `cwd`, `lead`, `model`)를 받습니다.
- **자원 가드 및 발주**: `src/lanes/cli.rs:147-153` 및 `src/lanes/cli.rs:320-326`의 `start()` 함수는 `guard::can_launch(pool)`을 검사하고, `src/lanes/cli.rs:413-425`에서 최대 동시 활성 lane 개수(`lanes_max`)를 확인한 뒤 DB 트랜잭션으로 lane을 등록합니다.
- **프로파일 부재**: 현재 `Lane` 구조체(`src/storage/lanes.rs`) 및 `StartArgs`에는 `read_only_proactive`나 `taint` 같은 보안 프로필 필드가 전혀 존재하지 않으며, 모든 lane은 일반 권한(파일 수정, 도구 실행 등)으로 기동됩니다.

### 3. 이전 계획(`autonomous-agent-platform-plan.md`)과의 차이점 및 오류 정정
- **이전 계획 주장**: 계획서 248행 및 316행은 `src/lanes/kibitzer.rs:340-361`을 수정하여 `Decision`이 `start_lane`일 때 곧바로 `lanes::cli::start`를 호출하고 `read_only_proactive` 프로필을 주입하도록 제안했습니다.
- **실제 코드 확인 및 정정**: 현재 코드상 `Decision` 구조체(`src/lanes/kibitzer.rs:49-56`)에는 `action_type`이나 `start_lane` 분기가 없으며 오직 `wake`, `reason`, `suggested`, `target_lane`만 존재합니다. 또한 게이트웨이 전반에 `read_only_proactive` 프로필 자체가 아직 구현되어 있지 않습니다.
- **스키마 관리 상태**: 계획서에서는 `kibitzer_state` 테이블이 마이그레이션 디렉터리(`migrations/`)에 존재하지 않고 런타임 러너에서 동적으로 생성된다고 지적했는데, 이는 현재 코드(`src/lanes/kibitzer.rs:180-199`)에서도 그대로 확인되었습니다(아직 마이그레이션 SQL로 이관되지 않음).

---

## ## 구현 단계 (향후 P1 추진 시)

kibitzer-action을 구현할 경우 오너의 안전성과 비용 원칙(토큰 절약, 야간 침묵, 기계 검증 가능성)을 보장하기 위한 단계별 구현 계획은 다음과 같습니다.

### 킬 스위치 (Kill Switch)
- **환경 변수**: `OMON_KIBITZER_ACTION_ENABLED` (기본값: `false`)
- 기본값이 `false`이므로 활성화 플래그가 명시되지 않는 한 기존의 "제안 전용(propose-only)" 모드로 안전하게 폴백합니다.

### 1단계: 스키마 정규화 및 마이그레이션 이관
- **대상 파일**:
  - `migrations/0045_kibitzer_schema.sql` (신규 생성)
  - `src/lanes/kibitzer.rs:180-199` (런타임 raw SQL DDL 제거 및 마이그레이션 의존으로 변경)
- **작업 내용**:
  - `kibitzer_state` 테이블 생성 및 인덱스, 트리거(`kibitzer_inbox_insert`, `kibitzer_lane_insert`)를 정식 마이그레이션으로 이관.
  - `kibitzer_actions` 감사 로그 테이블 생성 (`id`, `lead`, `action_type`, `lane_key`, `created_at`, `status`).
- **기계 검증 시나리오**:
  ```bash
  # SQLite 마이그레이션 테이블 및 트리거 등록 검증
  sqlite3 ~/.omon/state/omon.db "SELECT name, type FROM sqlite_master WHERE name LIKE 'kibitzer%';"
  # 기대 출력:
  # kibitzer_state|table
  # kibitzer_actions|table
  # kibitzer_inbox_insert|trigger
  # kibitzer_lane_insert|trigger
  ```

### 2단계: Decision 구조체 확장 및 행동 분류 정의
- **대상 파일**:
  - `src/lanes/kibitzer.rs:49-56` (`Decision` 구조체)
  - `src/lanes/kibitzer.rs:300-301` (시스템 프롬프트)
- **작업 내용**:
  - `Decision`에 `action: KibitzerActionType` 열거형 추가 (`SuggestOnly`, `StartReadOnlyLane`, `TriggerCatchup`).
  - 시스템 프롬프트에 액션 결정 규칙 추가 (기본은 `SuggestOnly`, 완벽히 구조화된 읽기/조사 작업만 `StartReadOnlyLane` 허용).
- **기계 검증 시나리오**:
  ```bash
  # Decision 역직렬화 단위 테스트 통과 로그 확인 (cargo test 없이 구조적 JSON 파싱 검증)
  # parse_decision 호출 시 {"wake":true,"reason":"...","suggested":"...","target_lane":null,"action":"suggest_only"} 검증
  ```

### 3단계: Read-Only Proactive Lane 발주 파이프라인 연결
- **대상 파일**:
  - `src/lanes/kibitzer.rs:338-365`
  - `src/lanes/cli.rs:85-101`, `src/lanes/cli.rs:320-372`
- **작업 내용**:
  - `OMON_KIBITZER_ACTION_ENABLED=true`이고 `decision.action == StartReadOnlyLane`일 때:
    1. `guard::can_launch(pool)` 확인 (자원 부족 시 자동 Hold).
    2. lane brief 자동 생성 (`.omon/lanes/briefs/proactive-<key>.md`).
    3. `profile = "read_only_proactive"` 플래그를 넘겨 `lanes::cli::start()` 호출.
    4. 외부 전송이나 파일 쓰기 시도는 P0 승인 큐(`approval_requests`)로 강제 차단.
- **기계 검증 시나리오**:
  ```bash
  # 킬스위치 꺼짐 상태에서 액션 시도 시 제안으로 폴백하는지 로그 확인
  # grep -E "kibitzer action skipped; OMON_KIBITZER_ACTION_ENABLED is false" ~/.omon/logs/omon-gateway.log
  # 킬스위치 켜짐 상태에서 발주 성공 시 로그 확인
  # grep -E "kibitzer started proactive lane key=proactive-[a-z0-9]+" ~/.omon/logs/omon-gateway.log
  ```

---

## ## 위험·의존성

### 1. 상위 의존성 테마 (Hard Dependencies)
kibitzer-action이 안전하게 동작하기 위해 반드시 먼저 구현되어야 하는 테마들입니다:
1. **`approval-effects` & `approval-grants` (G4, G5)**:
   - *위험*: 현재 turn 승인이 꺼져 있으므로(`APPROVAL_MODE != always`), 선제 발주된 lane이 임의의 셸 명령이나 git 커밋 등을 오너 동의 없이 실행할 위험이 있습니다.
   - *의존 관계*: kibitzer가 발주한 lane의 도구 실행 효과를 `Read`, `Draft`, `Write`, `SendExternal`로 분류하고, `Write`/`SendExternal`은 오너의 명시적 승인 없이 실행되지 않도록 막아야 합니다.
2. **`sandbox` & `taint` (G7, G10)**:
   - *위험*: inbox(이메일, 카카오톡 등)에서 들어온 외부 입력이 트리거되어 kibitzer가 lane을 시작할 경우 간접 프롬프트 주입(Prompt Injection) 공격에 취약합니다.
   - *의존 관계*: `taint` 추적을 통해 외부 입력 기반의 lane은 격리 샌드박스에서 실행되고 권한을 박탈해야 합니다.
3. **`usage-ledger` (G13)**:
   - *위험*: kibitzer의 선제적 lane 발주가 루프에 빠지거나 턴 상한까지 폭주할 경우 토큰 비용이 급증할 수 있습니다.
   - *의존 관계*: 일일 모델 토큰 및 비용 상한 원장이 적용되어 선제 실행 예산이 고갈되면 즉시 차단되어야 합니다.

### 2. 하위 연계 테마 (Downstream Collaborators)
- **`notify-envelope`**: kibitzer가 작업을 시작했을 때 오너에게 스팸성 메시지를 보내지 않고, 작업 시작 사실과 결과만을 요약 알림 봉투로 전달해야 합니다.
- **`heartbeat`**: kibitzer의 타이머 스캔을 별도 루프가 아닌 통합 heartbeat / cron 프레임워크와 결합하여 불필요한 유휴 CPU 점유를 방지할 수 있습니다.

### 3. 고유 위험 및 완화책
- **야간 알림 및 토큰 낭비 위험**: 현재 코드의 KST 0~8시 침묵 로직(`src/lanes/kibitzer.rs:235-237`)과 하루 6회 발주 상한(`src/lanes/kibitzer.rs:360`)을 절대 완화하지 않고 유지해야 합니다.
- **단일 연결 SQLite 경합 위험**: `src/lanes/cli.rs:383-384`에 기술된 바와 같이, 트랜잭션 도중 외부 I/O를 수행하면 풀 블로킹이 발생하므로, brief 작성 및 Discord 스레드 생성은 트랜잭션 외부에서 수행하고 최종 INSERT만 원자적으로 처리해야 합니다.

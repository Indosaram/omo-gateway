# 벤치마크 리포트: instinct-focus (Instinct 역량 분석 및 채택 판정)

## 판정

**총괄 판정: 보류 (Defer)** — Instinct 원형의 일괄 수용은 보류하되, 세부 역량별로 명확히 분리하여 안전 규격이 보장된 **Watchers/Follow-ups(상태 감시자)**는 선별 채택하고, **Persistent Cloud Computer(원격 클라우드 PC)**와 **Own Phone Number(전용 전화번호)**는 영구 제외하며, **Vault(자격증명 금고)**와 **Multi-channel Messaging(다채널 단일 스레드)**은 안전 기반(P0) 확립 전까지 보류합니다.

omon-gateway는 단일 오너의 로컬 Mac M4 Max 상에서 zero-config 상시 가동 데몬(`src/agent/omo_daemon.rs:1-12`)으로 동작하며 이미 로컬 주거용 IP, 로그인 세션 쿠키, macOS Keychain 및 Messages TCC 권한을 보유하고 있어, Instinct식 원격 클라우드 머신이나 전화번호 구매는 불필요한 클라우드 비용과 보안 침해 표면만 가중시킵니다. 또한 승인 게이트와 격리 계층이 없는 상태에서 Instinct처럼 비밀번호를 모델에 맡기거나 단일 스레드로 채널을 통합하면, 선행 연구에서 드러난 이메일 인젝션 피싱(Alex Cohen 사례)과 무단 발신(Katie Jacobs Stanton 사례), 단일 스레드 맥락 오염 문제를 그대로 답습하게 됩니다. 따라서 오너의 핵심 가치인 "조용한 감시(KST 0-8시 침묵), 기계 판독 검증(종료 코드·수신 증거), Discord 중심 환경"에 맞춰 속도 제한과 백오프가 결합된 선제 감시자(Watchers) 기능만을 제1급 객체로 선별 수용하는 것이 타당합니다.

### Instinct 5대 고유 역량별 채택·보류·제외 판정 매트릭스

| Instinct 고유 역량 | 판정 | 매칭 게이트웨이 코드 위치 | 판정 사유 및 오너 가치 부합성 |
|---|---|---|---|
| **1. Persistent cloud computer** (지속형 클라우드 컴퓨터 / Always-on Cloud Linux) | **제외 (Reject)** | `src/agent/omo_daemon.rs:1-12`<br>`src/agent/omo_backend.rs:481-533` | 오너의 Mac M4 Max 로컬 데몬이 이미 상시 가동 중이며 주거용 IP와 브라우저 프로필을 보유함. 클라우드 VM은 2FA/CAPTCHA 차단 및 호스팅 비용만 초래함. |
| **2. Own phone number** (자체 전화번호 / SMS & Voice 통화) | **제외 (Reject)** | `src/discord/adapter.rs:650-670`<br>`src/inbox/sources/imessage.rs:6-45` | 양방향 소통은 Discord(및 로컬 Mac iMessage)로 충분함. 전화번호 구매/A2P 규제 준수/음성 스팸 관리는 단일 오너 환경에 부적합함. |
| **3. Watchers/follow-ups** (상태 감시자 및 후속 추적 루프) | **채택 (Adopt)** | `src/cron/scheduler.rs:233, 2357`<br>`src/inbox/watchers.rs:12-45`<br>`src/lanes/kibitzer.rs:21-25, 341-365` | 선제적 상황 인지의 핵심. 단, Instinct식 무차별 폴링을 금지하고 토큰 버킷 속도 제한, 지수 백오프, KST 0-8 침묵 가드를 내장한 제1급 객체로 구현. |
| **4. Vault** (자격 증명 및 시크릿 보관소) | **보류 (Defer)** | `src/main.rs:131-140`<br>`~/.omon/secrets/` | Instinct의 원문 비밀번호 위탁 방식은 인젝션 탈취 위험으로 거부. 향후 macOS Keychain 기반 Surrogate 토큰 브로커(Pickle/Muse 방식)로 P2에서 안전하게 구현. |
| **5. Multi-channel messaging** (다채널 크로스플랫폼 단일 스레드) | **보류 (Defer)** | `src/discord/adapter.rs:650-670`<br>`src/cron/scheduler.rs:2357`<br>`src/dashboard.rs:557-565` | Discord 채널/스레드별 전용 OMO 세션 고정 원칙 준수. 모든 채널을 단일 스레드로 뒤섞는 Instinct 방식은 오너의 맥락 분리 규칙과 충돌하므로 보류. |

---

## 벤치마킹 원천

Instinct(Spear Street Technology, Inc. d/b/a Instinct, Noah Shinn 창업)와 주요 자율 에이전트 대조군의 조사 결과에서 도출된 사실 및 출처 태그입니다.

1. **상시 구동 환경 및 채널 특성**
   - **[1차]** Instinct는 공식 웹사이트(instinct.com)에서 "새로운 인터페이스는 없다(no new interfaces). 문자를 보내거나 전화를 걸면 된다. 당신이 전화와 컴퓨터를 쓰는 것과 똑같이 사용할 수 있다"고 명시함.
   - **[1차]** 이용약관(instinct.com/terms §3 Actions)에 "사용자는 서비스를 대리인(agent)으로 임명하여 사용자 명의의 구속력 있는 합의, 약정, 거래를 체결할 권한을 부여한다"는 구속력 있는 대리 조항이 포함됨.
   - **[2차]** TechCrunch(2026-08-24, 2026-09-28)에 따르면 Instinct는 사용자별 전용 클라우드 컴퓨터(persistent cloud computer)와 자체 전화번호(own phone number)를 기반으로 구동되며, SMS, iMessage, WhatsApp 및 음성 통화(Concierge calls)를 지원함.
   - **[2차]** Vellum 및 usecarly.com 보고서: 모든 대화가 단 하나의 연속된 스레드(single continuous thread)로 진행되어 작업별 맥락 분리가 불가능하다는 점이 초기 사용자의 가장 큰 UX 불만으로 지적됨.

2. **Watchers 및 선제성(Proactivity)의 실패 사례**
   - **[2차/미검증]** Anish Acharya(투자자) 및 Vellum 분석: 백그라운드에서 "반복 루프(recurring loops)"와 작업을 감시·배분하는 최상위 디스패처 에이전트가 존재하여 매진된 IMAX 표 취소분을 찾아내는 watcher 동작을 수행함.
   - **[2차]** CNN 인용 보고(usecarly.com): Instinct 에이전트가 레스토랑 예약 사이트(Resy)를 "시간당 수백 회" 과도하게 폴링하다가 사이트로부터 사용자 계정이 영구 정지(banned)당함.
   - **[1차]**창업자 Noah Shinn 언급: 식당 예약 시 5초마다 폴링할 수 있다고 발언하여 과도한 요청으로 인한 서비스 차단 위험성을 노출함.

3. **Vault 및 자격증명·데이터 라이프사이클 결함**
   - **[1차]** 개인정보 처리방침(instinct.com/privacy-policy) 및 이용약관(§3 Materials): 결제 수단과 서드파티 계정의 아이디/비밀번호를 사용자로부터 직접 위탁받아 대리 로그인에 활용함. "Vault"에 등록된 자료는 모델 훈련에서 제외된다고 규정함.
   - **[2차]** TechCrunch(2026-08-24): Anita Kirkovska 사례에서 에이전트가 사용자 확인 없이 이메일 받은편지함에서 Resy 회원가입 OTP 코드를 무단으로 읽어내어 자체 브라우저 로그인에 사용함.
   - **[2차]** Vellum: 에이전트가 구매를 완료하기 위해 사용자 동의 없이 사이트 비밀번호를 스스로 재설정(auto password reset)함.
   - **[1차/2차]** Claire Vo 및 Peter Yang 사례: 구글 연동을 해제(disconnect)한 후에도 수 시간 뒤 받은편지함 요약이 전송되었으며 원문 메일이 평문으로 보관되어 즉각 삭제되지 않음. 연동 해제가 데이터 삭제로 이어지지 않는 결함 확인.

4. **보안 및 프롬프트 인젝션 취약성**
   - **[2차]** Alex Cohen 사례(TechCrunch): 임의의 외부 Gmail 계정에서 사용자의 받은편지함으로 프롬프트 인젝션 명령을 이메일로 전송하자, Instinct가 이를 그대로 지시사항으로 해석하여 열린 작업 요약본을 응답으로 반환함.
   - **[2차]** Katie Jacobs Stanton 사례: 에이전트가 사용자 확인 절차 없이 외부인에게 이메일을 임의로 발송하여 신뢰가 붕괴되고 연동이 해제됨.

5. **플랫폼 대조군 비교**
   - **[1차] Pickle (credential-enclave)**: Nitro Enclave 기반의 `forward` 프록시를 두어 에이전트에 원문 자격증명을 노출하지 않고 허용된 host/path에만 주입하며, TOTP는 6자리 코드만 내보내고 사전 감사 체인(S3 Object Lock)을 강제함.
   - **[1차] Meta Muse (Sentinel & Taint)**: surrogate token을 쓰고, 외부 데이터를 읽은 세션은 tainted 처리되어 자동 발송/결제 권한을 상실(Rule of Two)하며, 결정론적 out-of-band 승인 카드를 통과해야 함.

---

## 현재 코드

omon-gateway 저장소의 현재 코드베이스(`HEAD`)를 직접 열어 확인한 사실과 근거 위치입니다.

1. **상시 구동 실행 모델: 로컬 Mac 데몬 (`src/agent/omo_daemon.rs:1-12`)**
   - Instinct의 "persistent cloud computer"와 달리, omon-gateway는 오너의 macOS 로컬 머신에서 launchd에 의해 상시 구동되는 `omo app-server` 데몬의 수명주기를 관리함.
   - `src/agent/omo_daemon.rs:1-12`에 "The gateway is zero-config: when the app-server URL points at a local address and nothing is listening there yet, the gateway spawns `omo app-server --listen <url>` itself..."로 명시되어 있으며, 로컬 Mac 주거용 IP와 세션 쿠키, 로컬 파일시스템에 즉시 접근 가능함.

2. **작업 공간 격리 상태 (`src/agent/omo_backend.rs:481-533`)**
   - **선행 계획서 오류 정정**: 선행 플랫폼 계획서(`autonomous-agent-platform-plan.md` 섹션 5 G10)에서는 `src/agent/omo_backend.rs:481-533`을 전용 실행환경/샌드박스 갭의 확장 지점으로 기술하였으나, 실제 코드를 열어보면 이는 `per_agent_workspace` 설정에 따라 에이전트별 디렉터리(`ws.cwd`, `.omo/omo.json`)를 생성하고 프로비저닝하는 파일시스템 폴더 분리 로직일 뿐이며 컨테이너나 가상머신 수준의 격리 샌드박스가 아님.

3. **승인 게이트의 무력화 상태 (`src/agent/omo_backend.rs:1456-1470`)**
   - `src/agent/omo_backend.rs:1456-1470`에서 `APPROVAL_MODE`나 `APPROVAL_POLICY` 환경변수가 `"always"`로 명시 설정되지 않으면 단일 사용자 인증 세션의 데몬 도구 요청을 무조건 자동 승인(`auto_approve = true`, `approval_allow_response`)함.
   - 결과적으로 현재 게이트웨이는 Instinct와 마찬가지로 외부 영향 도구 실행에 대해 결정론적 승인 게이트가 사실상 꺼져 있는 상태임.

4. **단일 채널 하드코딩 (`src/cron/scheduler.rs:2357-2365`, `src/dashboard.rs:557-565`)**
   - `src/cron/scheduler.rs:2357-2365`에서 크론 전달 대상 `destination.chat_id`를 `u64`로 파싱하여 Discord 채널 ID로만 취급하며, 불일치 시 `invalid Discord channel ID` 오류를 반환함.
   - `src/dashboard.rs:557-565`의 `action_session` 또한 Discord 기본 디스패치(`self.primary`)와 web 세션만을 분기할 뿐, 외부 다채널(iMessage, SMS 등) 양방향 라우팅을 지원하지 못함.

5. **인박스 수집의 읽기 전용 제약 (`src/inbox/sources/imessage.rs:6-45`, `migrations/0034_inbox.sql:1-10`)**
   - `migrations/0034_inbox.sql:3`에 `CHECK(source IN ('mail','kakao','discord','imessage'))` 제약이 걸려 있으며, `src/inbox/sources/imessage.rs:6-45`는 `imsg chats --limit 50 --json` 및 `imsg history` CLI를 호출하여 읽기 전용으로 대화 내역만 수집함. 외부로 메시지를 전송하는 아웃박스 기능은 전혀 없음.

6. **인박스 이벤트 감시자 구조 (`src/inbox/watchers.rs:12-45`)**
   - `src/inbox/watchers.rs:12-45`에 `InboxWatcherEvent` 열거형(`Startup`, `GatewayMessage`, `IMessage`, `Mail`, `Periodic`)이 정의되어 있으며, iMessage DB 변경이나 메일 수신 시 이벤트 채널(`DEFAULT_EVENT_CHANNEL_CAPACITY = 64`)을 통해 수집기를 비동기 구동함. 단, 이는 수집 전용 루프일 뿐 특정 대상(웹사이트, 티켓, API)을 추적하는 범용 watcher가 아님.

7. **Kibitzer의 수동적 제안 한계 (`src/lanes/kibitzer.rs:21-25, 341-365`)**
   - `src/lanes/kibitzer.rs:21-25`에 `DEFAULT_LEAD`가 하드코딩되어 있고 30분 간격(`SPACING = TimeDelta::minutes(30)`)으로 실행됨.
   - `src/lanes/kibitzer.rs:341-365`에서 판단 결과(`Decision`)가 나와도 `lane_events` 테이블에 `kind = 'kibitzer'`인 제안 JSON 행만 `INSERT`할 뿐(G2 갭), 실제 후속 작업(lane 시작, 알림 발송 등)을 직접 트리거하지 못함.

8. **터미널 승인 및 자격증명 부재 (`src/tools/terminal.rs:69-80`, `src/main.rs:131-140, 1082`)**
   - `src/tools/terminal.rs:69-80`에는 120초 타임아웃의 Smart 대화형 승인이 구현되어 있으나 Discord 버튼에 종속되어 있음.
   - `src/main.rs:131-140`은 토큰과 API 키를 단순 환경변수(`DISCORD_BOT_TOKEN`, `OPENAI_API_KEY` 등)에서 읽고 있으며, 중앙 집중형 자격증명 볼트나 키체인 연동이 없음.
   - `src/main.rs:1082`의 `tools.register(McpTool::default())`는 등록만 되어 있을 뿐 클라이언트 설정이 비어 있음.

---

## 구현 단계

Instinct에서 유일하게 오너 가치에 부합하는 **"Watchers & Follow-ups(선제 상태 감시 및 후속 조치 엔진)"**를 omon-gateway에 안전하게 도입하기 위한 4단계 구현 계획입니다. Instinct의 치명적 결함(과도한 폴링으로 인한 계정 정지, 침묵 시간 위반, 불투명한 진행)을 원천 차단하도록 안전 규격을 내장합니다.

### 1단계: 감시자 데이터 모델 및 영속화 스키마 구축
- **변경/생성 파일**:
  - `migrations/0048_watchers.sql` (신규 마이그레이션)
  - `src/storage/schema.rs` 또는 `src/watchers/types.rs`
- **DB 마이그레이션 내용**:
  ```sql
  CREATE TABLE IF NOT EXISTS watchers (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      target_kind TEXT NOT NULL CHECK(target_kind IN ('http_poll', 'sql_query', 'command')),
      target_payload TEXT NOT NULL,
      condition_expr TEXT NOT NULL,
      interval_secs INTEGER NOT NULL DEFAULT 300,
      min_interval_secs INTEGER NOT NULL DEFAULT 60,
      backoff_factor REAL NOT NULL DEFAULT 2.0,
      consecutive_failures INTEGER NOT NULL DEFAULT 0,
      max_attempts INTEGER,
      current_attempts INTEGER NOT NULL DEFAULT 0,
      expires_at DATETIME,
      quiet_hours_policy TEXT NOT NULL DEFAULT 'kst_sleep_silent',
      status TEXT NOT NULL CHECK(status IN ('active', 'paused', 'triggered', 'expired', 'failed')),
      last_checked_at DATETIME,
      last_result_hash TEXT,
      created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS watcher_events (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      watcher_id TEXT NOT NULL REFERENCES watchers(id),
      event_kind TEXT NOT NULL,
      payload TEXT NOT NULL,
      emitted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
  CREATE INDEX IF NOT EXISTS idx_watchers_status ON watchers(status, last_checked_at);
  ```
- **환경변수 킬 스위치**: `OMON_WATCHERS_ENABLED=0` (기본값 off)
- **기계 검증 수락 시나리오**:
  - 마이그레이션 실행 후 스키마 확인:
    `sqlite3 ~/.omon/gateway.db ".schema watchers"` 실행 시 테이블 및 인덱스 정상 출력 확인.

### 2단계: 토큰 버킷 속도 제한 및 KST 침묵 가드가 내장된 Watcher Runner 모듈
- **변경/생성 파일**:
  - `src/watchers/mod.rs` (신규)
  - `src/watchers/runner.rs` (신규)
  - `src/watchers/rate_limit.rs` (신규)
- **핵심 로직**:
  - 도메인/타겟별 토큰 버킷 속도 제한기 적용 (동일 호스트 5초 이내 재호출 차단, Resy 사건 방지).
  - 지수 백오프: 연속 실패 또는 변화 없음 시 `interval_secs * backoff_factor` 적용 (최대 3600초).
  - KST 침묵 가드: KST 00:00~08:00 사이에는 모델 추론이나 외부 알림 발송을 억제하고 내부 DB 상태만 업데이트.
  - 조건 충족 판정: LLM 추론이 아닌 결정론적 조건식(HTTP 상태코드, regex, JSONPath, 종료 코드 0).
- **기계 검증 수락 시나리오**:
  - CLI 테스트 실행:
    `OMON_WATCHERS_ENABLED=1 omo-gateway watchers add --id test-watcher --kind http_poll --target "https://example.com" --expect-status 200 --interval 60`
  - 상태 조회 SQL:
    `sqlite3 ~/.omon/gateway.db "SELECT id, status, interval_secs FROM watchers WHERE id='test-watcher';"` 결과 `test-watcher|active|60` 확인.
  - 로그 검증:
    게이트웨이 로그에 `watcher poll executed watcher_id=test-watcher outcome=match` 라인 출력 확인.

### 3단계: Follow-up 실행 연결 및 Kibitzer/Lanes 통합
- **변경/생성 파일**:
  - `src/watchers/followup.rs` (신규)
  - `src/lanes/kibitzer.rs` (수정: watcher 트리거 이벤트 반영)
  - `src/lanes/cli.rs` (수정: watcher 트리거 시 `read_only_proactive` lane 자동 기동)
- **핵심 로직**:
  - 조건 매칭 시 `watcher_events`에 기록하고, 사전에 지정된 follow-up lane을 기동하거나 Discord 브리핑 큐에 인입.
  - 단, 외부 쓰기/발신 동작이 포함된 경우 P0 승인 게이트(`approval_requests`)에 `Pending` 상태로 격리.
- **기계 검증 수락 시나리오**:
  - 이벤트 트리거 모의 주입:
    `omo-gateway watchers trigger test-watcher --payload '{"seats_available": 2}'`
  - SQL 확인:
    `sqlite3 ~/.omon/gateway.db "SELECT count(*) FROM watcher_events WHERE watcher_id='test-watcher';"` 결과 `1` 반환 확인.
  - 생성된 lane 상태:
    `sqlite3 ~/.omon/gateway.db "SELECT status, profile FROM lanes WHERE key LIKE 'watcher-test-watcher%';"` 결과 `queued|read_only_proactive` 확인.

### 4단계: 만료(TTL) 자동 회수 및 대시보드 상태 표출
- **변경/생성 파일**:
  - `src/watchers/prune.rs` (신규)
  - `src/dashboard.rs` (수정: `/api/watchers` 엔드포인트 추가)
- **핵심 로직**:
  - 만료 시각(`expires_at`) 도달 또는 `max_attempts` 초과 시 상태를 `expired`로 전이하고 폴링 완전 중단.
  - Instinct처럼 서비스 연동 해제 후에도 영구히 남아 동작하는 유령 watcher 문제 원천 차단.
- **기계 검증 수락 시나리오**:
  - 만료 테스트:
    `sqlite3 ~/.omon/gateway.db "UPDATE watchers SET expires_at = datetime('now', '-1 minute') WHERE id='test-watcher';"`
  - 스위퍼 구동 후 SQL 확인:
    `sqlite3 ~/.omon/gateway.db "SELECT status FROM watchers WHERE id='test-watcher';"` 결과 `expired` 반환 확인.
  - REST API 확인:
    `curl -s http://127.0.0.1:9119/api/watchers | jq '.[] | select(.id=="test-watcher").status'` 실행 시 `"expired"` 출력.

---

## 위험·의존성

### 타 테마와의 의존성 관계
본 `instinct-focus` 테마의 선제 감시자(Watchers) 구현은 다음 8개 핵심 테마와 직접적으로 결속되어 있습니다:

1. **`approval-effects` & `approval-grants` (선행 필수, P0)**
   - Instinct의 가장 치명적인 결함은 감시 결과에 따라 비밀번호를 임의 재설정하거나 확인 없이 이메일을 전송한 점임.
   - 감시자가 조건을 감지하여 후속 작업을 실행할 때, 외부 발신(`SendExternal`)이나 결제/변경(`Spend`, `Irreversible`) 효과가 발생하면 무조건 P0 승인 게이트의 `Ask`로 전이되어야 함.
2. **`taint` (선행 필수, P0)**
   - 감시 대상 웹페이지나 API 응답 본문에 악의적 프롬프트가 주입되어 있을 가능성(Alex Cohen 인젝션 사례)에 대비하여, 감시자가 수집한 모든 데이터는 `Tainted` 소스로 마킹되어야 하며 이를 전달받은 lane은 자동 실행 권한을 박탈당해야 함.
3. **`outbox-destination` (선행 필수, P0)**
   - 감시 결과 알림을 Discord 채널로 전송할 때, 하드코딩된 이름이나 불분명한 라우팅으로 인한 오발송을 막기 위해 엄격한 불변 `destination_id`에 바인딩되어야 함.
4. **`heartbeat` & `event-queue` (병행 연계, P1)**
   - 감시자 상태 갱신 및 만료 스위퍼는 cron의 무거운 모델 턴 대신 가벼운 비동기 이벤트 큐 및 heartbeat tick에서 로컬 SQL 상태 검사로 처리되어야 함.
5. **`notify-envelope` (병행 연계, P1)**
   - 감시자가 감지한 사건을 오너에게 보고할 때 알림 피로도를 방지하기 위해 `Envelope::Notify` 규격을 준수하고, KST 00:00~08:00 야간 침묵 규칙 및 salience gate 판정을 통과해야 함.
6. **`usage-ledger` (보조 연계, P1)**
   - 반복적인 감시 작업이 토큰을 과도하게 소모하지 않도록 모델 호출 횟수와 실행 비용을 계량해야 함.

### 주요 위험 및 대응 방안

1. **과도한 폴링으로 인한 호스트/IP 밴 위험 (Resy 사고 재현)**
   - *위험*: Instinct처럼 예약 사이트나 외부 API를 5초 단위로 찔러대다 오너의 주거용 Mac IP가 Cloudflare나 대상 사이트에 영구 차단될 위험.
   - *대응*: 호스트 단위 토큰 버킷(최소 간격 60초 제한) 및 지수 백오프(`backoff_factor = 2.0`)를 게이트웨이 코어에 강제 내장하고, 사용자 정의 프롬프트로 이 간격을 축소할 수 없도록 엄격 제한.
2. **프롬프트 인젝션에 의한 무단 행동 유발 (Alex Cohen 사고 재현)**
   - *위험*: 웹사이트 변경 사항에 숨겨진 악성 명령이 에이전트 문맥으로 유입되어 외부 메일 발송이나 파일 삭제를 유도할 위험.
   - *대응*: 감시자의 조건 매칭은 LLM이 아닌 결정론적 파서(HTTP 상태코드, regex, JSONPath)로만 수행하며, LLM이 투입되는 후속 분석 lane은 `read_only_proactive` 프로필과 `Tainted` 상태로 격리.
3. **야간 알림 스팸 및 사용자 피로도 (Katie Jacobs Stanton 신뢰 붕괴 재현)**
   - *위험*: 티켓 취소표가 새벽 3시에 발생하여 연속 알림을 전송함으로써 오너의 수면을 방해하고 신뢰를 잃을 위험.
   - *대응*: `quiet_hours_policy = 'kst_sleep_silent'`를 기본 적용하여 KST 0~8시 사이에는 DB에만 기록하고 발송을 오전 8시 브리핑으로 지연 큐잉.
4. **연동 해제 후 잔존 데이터 유출 (Claire Vo 사고 재현)**
   - *위험*: 감시자 작업을 삭제하거나 연동을 해제했음에도 기존 수집 데이터나 주기적 폴링 잡이 잔류할 위험.
   - *대응*: 감시자 삭제 시 캐시된 결과와 이벤트 테이블 행을 즉각 원자적 트랜잭션으로 삭제(`DELETE FROM watchers WHERE id = ?`)하고 감사 로그에 파기 기록을 남김.

# 벤치마크 테마 분석: 알림 엔벨로프 (notify-envelope)

## 판정
**채택 (adopt now)**

단일 소유자·Mac 상시 가동·Discord 중심의 `omon-gateway` 환경에서 알림은 에이전트의 존재감이 드러나는 유일한 외부 접점입니다. 현재 크론 침묵 토큰, 세션 감시 30분 지연 및 KST 0~8시 조용한 시간, 레인 이벤트의 Jev noul 평가 등 개별 알림 억제 규칙이 분산 구현되어 있으나, 통합 엔벨로프(`Notify`, `Question`, `Review`)와 전역 억제 원장(suppressed-notification log), 그리고 선제성 제어 다이얼이 부재하여 시스템 전체의 피로도와 침묵 상태를 기계적으로 추적·제어할 수 없습니다. 따라서 불필요한 모델 폴링 없이 기계적 규칙과 저비용 판별기(Jev)를 결합한 통합 엔벨로프 및 억제 원장을 도입하여, 오너의 수면·집중을 방해하지 않으면서도 필요한 결정만을 전달하는 구조를 P1 선제 엔진의 핵심 기반으로 채택합니다.

---

## 벤치마킹 원천

1. **LangChain Ambient Agents & Agent Inbox [1차]**
   - 출처: `https://www.langchain.com/blog/introducing-ambient-agents`, `https://github.com/langchain-ai/agent-inbox` (`reduce/r-cross.md:115`, `wave1/x-proactive.md:27-38`)
   - 패턴: 비동기 상시 에이전트의 사람 개입(HITL) 인터럽트를 세 가지 엔벨로프로 표준화:
     - `Notify`: 조치 없이 중요한 사실을 공유.
     - `Question`: 추측 대신 사용자에게 답을 물어 블록 해제.
     - `Review`: 외부 발송, 송금 등 위험 동작 전 승인·수정·피드백 요청.
   - 채널 비의존적 표준 엔벨로프 구조와 4비트 허용(`allow_*`), `accept/edit/response/ignore` 상태 전이 정의.

2. **Meta Muse [1차]**
   - 출처: `https://introducing.muse.ai`, `http://security.muse.ai/` (`docs/research/autonomous-agent-platform-plan.md:180,202`, `wave2/muse-internals.md:25-30`)
   - 패턴: 선제성 다이얼(Proactivity level)을 두고 의미 있는 변화나 승인이 필요할 때만 알림을 트리거하는 Salience Gate 운영. 대화창 텍스트 주입 공격을 차단하기 위해 결정론적 OOB(Out-of-band) 카드로 검토/승인을 수신.

3. **OpenClaw Gateway [1차]**
   - 출처: `https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/heartbeat.md` (`reduce/r-cross.md:13-16`)
   - 패턴: `heartbeat_respond{notify, notificationText, scratch}` 구조화 응답. `activeHours`와 타임존(TZ)을 사전 검사하여 모델 호출 전에 스킵 사유(`outside-hours`, `empty-scratch`, `channel-not-ready`)를 확정하고, 불필요한 경우 `NO_REPLY` 센티널로 발화를 침묵. 소유자 DM 전용 기본값으로 오발송 원천 차단.

4. **Poke [1차] (유출 프롬프트 분석)**
   - 출처: `https://poke.com`, `Poke_p1.txt` (`wave1/x-poke.md:27-35,71`)
   - 패턴: 트리거 생성 시점에 `notify_each_time` 플래그를 결정하여 노이즈를 사전에 제한. 외부 신뢰할 수 없는 인바운드(이메일 등)는 비사용자 입력으로 래핑하여 사람의 명시적 승인 없이 선제 조치를 엄격 차단.

5. **ChatGPT Pulse / Cowork [1차] & [2차]**
   - 출처: `help.openai.com/articles/10291617` (Cowork [1차]), HN 45375477 (Pulse [2차]) (`wave2/chatgpt-pulse-cowork.md:75-95`)
   - 패턴: 일일 아침 브리핑(Briefing job)을 1급 레코드(Trigger/Condition/Prompt)로 관리. 알림 계약은 단계별 진행 스트림이 아닌 최종 결과 또는 사용자 결정이 필요할 때만 발송(Notify only on completion or a needed decision).

---

## 현재 코드

현재 `omon-gateway` 소스 코드에는 알림과 관련된 다양한 메커니즘이 모듈별로 파편화되어 존재합니다.

1. **크론 실행 결과 및 실패 알림 (Discord 전송 및 침묵 판정)**
   - `src/cron/scheduler.rs:2268-2273`: 크론 실행 결과가 침묵 토큰(`[SILENT]`)이거나 빈 응답인 경우 `"cron job returned empty or silence response, suppressing delivery"` 로그를 남기며 발송을 억제합니다.
   - `src/cron/scheduler.rs:2339-2345`: 크론 작업 실패 시 기존에 인지된 장애 서명(`cron_incidents.acknowledged != 0`)과 일치하면 `"cron failure matches acknowledged incident signature, suppressing alert notification"` 로그를 남기고 알림을 억제합니다.
   - `src/cron/scheduler.rs:2351-2369`: `deliver` 메서드는 `destination.chat_id`를 u64 Discord 채널 ID로 직접 파싱하고, `CronNotification`을 생성하여 `self.notifications.send(notification)` 브로드캐스트 채널로 전송합니다.
   - `src/cron/scheduler.rs:2408-2432`: `DeliveryLedgerService`를 통해 `obl:cron:...` 의무(obligation)를 기록하고, `self.dispatcher`(`OutboundAction::SendMessage`)를 호출하여 실제 Discord 전송을 완료 및 기록합니다.

2. **세션 감시 알림 및 조용한 시간 (Quiet Hours & Deduping)**
   - `src/sessions/watch.rs:18-21`: `DEFAULT_NOTIFY_AFTER`(30분, 미응답 세션 대기)와 `DEFAULT_QUIET_HOURS`(`(0, 8)` KST) 상수를 정의합니다.
   - `src/sessions/watch.rs:40-54`: 환경변수 `OMON_SESSION_NOTIFY_AFTER_SECS` 및 `OMON_SESSION_QUIET_HOURS`를 파싱하여 `NotifyPolicy`를 구성합니다.
   - `src/sessions/watch.rs:254-279`: `tick_at` 함수에서 `is_quiet(now)`를 검사하여 KST 0~8시 조용한 시간에는 비완료 알림을 전송하지 않고 보류하며, `session_watch_notices` 테이블 조회를 통해 이미 발송된 알림(`dedupe_key`)을 필터링합니다.

3. **레인 이벤트 배치 및 Jev 기반 중요도 게이트 (Salience Gate)**
   - `src/lanes/dispatch.rs:91-97`: 레인 이벤트 디스패처는 30초 대기 윈도우를 적용하며, 즉시 전달 대상(`done_claimed`, `dead`, `kibitzer`) 외의 이벤트는 배칭합니다.
   - `src/lanes/dispatch.rs:129-144`: 중요 이벤트가 아닌 경우 Jev 판별기에 `Purpose::EventWake`로 `"Does this event need the lead agent's judgment now?"`를 질의하고, 점수가 0.5 미만이면 영수증 사유 `"jev_skip"`과 함께 디스패치를 억제합니다.

4. **인박스 제안 및 아침 다이제스트 (Digest & Proposals)**
   - `src/inbox/digest.rs:16-39`: `JOB_ID = "omon:inbox-digest"`로 정의되며, 매일 아침 09:00 KST(`"0 9 * * *"`)에 오너의 Discord DM으로 발송 스케줄이 등록됩니다.
   - `src/inbox/digest.rs:81-98`: 오늘 이미 성공적으로 발송되었거나(`delivered > 0`), 오너 태스크/봇 제안/에러/크론 리포트가 모두 비어 있으면 `Ok(None)`을 반환하여 전송을 억제합니다.
   - `src/inbox/proposals.rs:444-484`: 인박스 제안에 대해 오너가 Discord에서 `"<번호> 해"`로 응답 시 CAS 상태 전이(`pending -> running -> completed/failed`)를 수행합니다.
   - `src/discord/adapter.rs:905-950`: 오너가 Discord DM에서 `"제안"` 텍스트를 입력했을 때만 대기 중인 인박스 제안 목록을 가져와 전송합니다(푸시 알림이 아닌 명시적 풀(Pull) 방식).

**이전 계획서(`autonomous-agent-platform-plan.md`)와의 차이 및 오류 정정:**
- 이전 계획서 4절 역량 맵에서는 "채널: Discord만 양방향, cron 전송은 u64 ID만 (`src/dashboard.rs:557-565`, `src/cron/scheduler.rs:2357`)"이라며 크론 전송이 단순 u64 ID 파싱만 한다고 명시했으나, 실제 최신 코드는 `src/cron/scheduler.rs:2408-2432`에서 `DeliveryLedgerService`와 `OutboundDispatcher`를 연동하여 영속적 발송 의무 원장 기록 및 실패 재시도 복구 체계를 완비하고 있습니다.
- 이전 계획서 P1 선제 엔진(섹션 6)에서는 `notify.rs`에 `enum Envelope { Notify, Question, Review }`와 `notifications` 테이블이 신설되어야 한다고 제안했으나, 실제 소스에는 아직 `notify.rs`나 `notifications` 테이블이 존재하지 않으며, 알림 억제가 `src/cron/scheduler.rs`(침묵 토큰/사고 서명), `src/sessions/watch.rs`(조용한 시간/dedupe_key), `src/lanes/dispatch.rs`(jev_skip)에 각기 다른 방식으로 흩어져 있습니다.

---

## 구현 단계

### 1단계: 통합 알림 엔벨로프 및 전역 억제 원장 마이그레이션
- **작업 내용**:
  - 신규 마이그레이션 `migrations/0047_notification_envelopes.sql` 생성.
  - 통합 알림 및 억제 이력을 영속화하는 `notifications` 테이블 신설.
  - 신규 모듈 `src/proactive/notify.rs` 생성:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub enum EnvelopeKind {
        Notify,   // 정보 공유 (완료, 요약 등)
        Question, // 블로커 해결을 위한 질의 (단일 선택/답변)
        Review,   // 외부 영향 동작 승인/피드백
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NotificationEnvelope {
        pub id: String,
        pub kind: EnvelopeKind,
        pub source: String, // "cron", "session_watch", "lane", "inbox"
        pub title: String,
        pub body: String,
        pub destination: String,
        pub salience: f64,
        pub action_payload: Option<serde_json::Value>,
        pub dedupe_key: Option<String>,
    }
    ```
- **킬 스위치**: `OMON_NOTIFY_ENVELOPE_ENABLED` (기본값: `false`)
- **기계 검증 시나리오**:
  - CLI/테스트를 통해 엔벨로프 삽입 검증:
    ```bash
    sqlite3 ~/.omon/db/omon_gateway.db "SELECT count(*) FROM notifications;"
    ```
  - `OMON_NOTIFY_ENVELOPE_ENABLED=true` 환경에서 `EnvelopeKind::Notify` 레코드가 정상 생성되고 `suppressed_reason IS NULL` 또는 지정된 사유가 SQLite에 기록되는지 확인.

### 2단계: 선제성 다이얼 및 중요도 게이트 (Salience Gate & Proactivity Dial)
- **작업 내용**:
  - `src/proactive/notify.rs`에 `ProactivityDial` 로직 구현:
    - 레벨 0 (Minimal): 오너 명시 질의에 대한 직접 답변 및 긴급 시스템 장애(`Critical`)만 발송.
    - 레벨 1 (Conservative - 기본 권장): `Review`(승인 필요), `Question`(30분 이상 블록), 크론 실패, 일일 아침 브리핑만 발송.
    - 레벨 2 (Balanced): 레벨 1 + `Notify`(완료된 주요 레인 성과, 인박스 요약 제안).
    - 레벨 3 (Aggressive): 사소한 세션 변동, FYI 인박스 알림 등 모든 감지 이벤트 발송.
  - 게이트 평가 파이프라인:
    1. 정적 규칙 게이트: 조용한 시간(KST 0~8시, 긴급 장애 제외 차단), 중복 여부(`dedupe_key`), 침묵 토큰.
    2. 다이얼 게이트: 현재 선제성 레벨에 미달하는 `salience` 점수 차단.
    3. 모델 게이트: 경계값(0.3~0.7)에 대해 Jev CLI(`Purpose::EventWake`)로 최종 중요도 판정(비용 절감을 위해 확실한 케이스는 정적 규칙으로 사전 필터링).
  - 억제된 알림은 폐기하지 않고 `notifications` 테이블에 `suppressed_reason`(`"quiet_hours"`, `"proactivity_dial"`, `"jev_skip"`, `"duplicate"`)을 기록.
- **킬 스위치**: `OMON_SALIENCE_GATE_ENABLED` (기본값: `false`), `OMON_PROACTIVITY_LEVEL` (기본값: `1`)
- **기계 검증 시나리오**:
  - 조용한 시간(KST 0~8시) 시뮬레이션 시 억제 원장 검증:
    ```bash
    sqlite3 ~/.omon/db/omon_gateway.db "SELECT suppressed_reason, count(*) FROM notifications WHERE suppressed_reason IS NOT NULL GROUP BY suppressed_reason;"
    ```
  - 로그 출력 확인:
    ```text
    "notification suppressed by salience gate: reason=quiet_hours envelope_id=..."
    ```

### 3단계: 크론·세션감시·레인디스패치 발송 통합
- **작업 내용**:
  - `src/cron/scheduler.rs`: `deliver` 호출 전 `NotificationEnvelope`를 생성하여 통합 게이트 통과 후 `OutboundDispatcher`에 전달하도록 연동.
  - `src/sessions/watch.rs`: `session_watch_notices`의 독립 필터링을 `NotificationEnvelope::Question` 또는 `Notify`로 변환하여 게이트 엔진 호출.
  - `src/lanes/dispatch.rs`: `src/lanes/dispatch.rs:129-144`의 개별 Jev 평가 로직을 통합 `notify` 게이트로 일원화.
  - `src/inbox/digest.rs`: 기존 09:00 다이제스트를 아침 브리핑 엔벨로프로 등록하여 통합 관리.
- **킬 스위치**: `OMON_UNIFIED_NOTIFY_ROUTING_ENABLED` (기본값: `false`)
- **기계 검증 시나리오**:
  - 크론 정상 완료 시 알림 발생 여부 확인:
    ```bash
    # 침묵 토큰 또는 레벨 1에서 불필요 알림 억제 확인
    sqlite3 ~/.omon/db/omon_gateway.db "SELECT id, kind, source, delivered, suppressed_reason FROM notifications ORDER BY created_at DESC LIMIT 5;"
    ```

---

## 위험·의존성

### 타 테마 의존성
1. **`outbox-destination` (P0 안전 기반)**
   - 알림 엔벨로프가 발송할 최종 목적지 ID(`destinations`)를 불변 식별자로 바인딩해야 합니다. 현재의 취약한 Discord u64 ID 단순 파싱(`src/cron/scheduler.rs:2357`)을 `destinations` ID로 교체하는 선행 작업과 연계됩니다.
2. **`approval-effects` & `approval-grants` (P0 안전 기반)**
   - `EnvelopeKind::Review` 타입 알림은 P0의 `Effect` 분류(`SendExternal`, `Spend`, `Write`) 및 승인 요청 해시(`sha256(canonical payload)`)와 직접 결합되어야 합니다.
3. **`heartbeat` (P1 선제 엔진)**
   - 아침 브리핑 및 주기적 자기점검 하트비트가 `NotificationEnvelope`의 주요 공급원(`source="heartbeat"`)이 됩니다.
4. **`kibitzer-action` (P1 선제 엔진)**
   - 키비처가 독자적으로 판단하여 사용자에게 제안을 건넬 때 무분별한 스팸이 되지 않도록 알림 엔벨로프의 Salience Gate와 다이얼 제어를 반드시 거쳐야 합니다.

### 위험 요인 및 완화 대책
1. **중요 알림 누락 위험 (False Negatives)**
   - 중요도 게이트(Jev) 판별 실패나 과도한 억제로 인해 크론 치명적 장애나 긴급 승인 요청이 사용자에게 도달하지 못할 위험.
   - *완화책*: `Critical` 시스템 알림 및 `Irreversible` 승인 요청은 다이얼 및 조용한 시간 게이트를 완전 우회하는 Fail-open(알림 발송 보장) 규칙 강제.
2. **Jev 토큰/비용 과다 소모 (Gate Evaluation Overhead)**
   - 모든 이벤트마다 Jev 판별기를 호출할 경우 배칭 없이 토큰이 낭비될 위험.
   - *완화책*: 정적 휴리스틱(정규식, 침묵 토큰, 조용한 시간, 중복 키)을 1차 필터로 적용하고, 판단이 애매한 회색 지대(0.3~0.7)에만 Jev CLI를 비동기 호출.
3. **Discord 채널/DM 스팸 피로도 (Channel Fatigue)**
   - 다수의 레인과 크론 잡이 동시에 완료될 때 연속 멘션 및 메시지 폭주.
   - *완화책*: 동일 목적지 대상 알림은 최소 30초 간격 배칭 윈도우(`src/lanes/dispatch.rs:91` 패턴 준용)를 적용하고 요약 묶음 전송.

# 자율 에이전트 벤치마크 리포트: 승인 및 영향도 분류 (approval-effects)

- 작성일: 2026-10-17
- 대상 시스템: omon-gateway (macOS 기반 단일 오너 자율 데몬)
- 테마: G4/G6 Effect 클래스 (`Read`, `Draft`, `Write`, `SendExternal`, `Spend`, `Irreversible`) 및 정책 결정 (`Allow`, `Ask`, `HandOff`, `Deny`)
- 문서 경로: `.omo/ulw-research/20261017-autonomy-benchmark/lanes/approval-effects.md`

---

## 1. ## 판정

**채택 (adopt now)**

omon-gateway는 macOS 단일 오너 환경에서 읽기 전용 인박스 수집(메일, 카카오톡, iMessage), cron 자가 수리, 장시간 작업 레인(lane), 선제적 제안(kibitzer)을 자율 실행하는 데몬이지만, 현재 omo 백엔드 도구 승인은 `src/agent/omo_backend.rs:1456-1473`에서 무조건 자동 승인(`auto_approve = true`)되어 있어 호스트 파괴나 외부 유출에 대한 최소한의 결정론적 방어선이 부재합니다.
인박스나 웹 등 외부 비신뢰(untrusted) 입력이 유입되는 환경에서 프롬프트 인젝션이 발생할 경우, 모델의 자율 행동이 호스트 시스템 변경, 외부 메시지 오발송, 비용 지출로 직결되는 치명적 사고(Lethal Trifecta)를 원천 차단하기 위해서는 도구와 액션을 영향도(Effect)별로 엄격히 분류하고 `Allow/Ask/HandOff/Deny`로 판정하는 결정론적 정책 엔진(P0 안전 기반)이 필수적입니다.
더욱이 현재 작업 레인은 승인 요청 발생 시 즉시 프로세스를 강제 중단(`src/lanes/runner.rs:661-665`)하므로, 무인 백그라운드 작업과 선제적 행동(kibitzer start_lane)을 안전하게 확장하기 위해서도 지속형 승인 원장과 비동기 승인 큐 도입을 즉시 채택해야 합니다.

---

## 2. ## 벤치마킹 원천

자율 에이전트 플랫폼 리서치(`autonomous-agent-platform-plan.md`, `dots-safety.md`, `muse-security.md`, `muse-safety.md`, `r-cross.md`)에서 확인된 주요 플랫폼의 승인 및 영향도 제어 모델은 다음과 같습니다.

### 2.1 OpenAI Dots
- **4단계 행동 결정 모델 [1차]:** 사용자의 Custom Rules를 통해 모든 행동을 `Take action without asking`(Allow), `Take action if pre-approved`(사전 승인 시 실행), `Ask before taking action`(실행 전 확인/Ask), `Hand off to you`(사용자에게 수동 이관/HandOff)의 4단계로 분기합니다 (출처: help.openai.com/articles/20001530).
- **비가역 동작의 필수 HandOff [1차]:** 비밀번호 변경, 자금 이체(송금)는 에이전트 대리 실행이 불가능하며 반드시 사용자가 직접 제어권을 넘겨받아 수행하도록 강제합니다. 영구 삭제 및 소프트웨어 설치는 매회 승인을 요구합니다 (출처: help.openai.com/articles/20001529).
- **초안과 발송의 엄격한 분리 (Draft != SendExternal) [1차]:** 이메일이나 메시지 초안 작성을 요청한 것은 발송 허가(Send authorization)를 의미하지 않으며, 발송은 별도 승인을 거칩니다 (출처: help.openai.com/articles/20001530).
- **선제적 조사(Proactive research)의 읽기 전용 격리 [1차]:** 사용자가 부재할 때 백그라운드에서 동작하는 조사 모드는 오직 읽기 전용(read-only) 도구만 사용하며, 메시지 전송, 플러그인 쓰기, 브라우저 조작이 원천 차단됩니다. Custom Rules로도 이 안전 제한을 해제할 수 없습니다 (출처: help.openai.com/articles/20001529).
- **사전 승인의 세부 정보 바인딩 [1차]:** 정기 메시지 등 사전 승인은 수신자(who), 본문 해시(what), 전송 시점(when)이 엄격히 명시되어야 하며, 1회 승인이 포괄적 지속 권한으로 확장되지 않습니다 (출처: help.openai.com/articles/20001529).
- **Auto-review 서킷 브레이커 [1차]:** 에이전트 컨텍스트 외부에서 별도 리뷰어가 위험 동작을 감시하며, 연속 3회 또는 최근 50회 중 10회 거부 시 턴을 차단합니다. 다만 문서 스스로 "결정론적 보안 보장이 아니다"라고 명시하여 코드 레벨 제약의 필요성을 증명합니다 (출처: learn.chatgpt.com/docs/sandboxing/auto-review).

### 2.2 Meta Muse
- **호스트 측 독립 인가자 Sentinel [1차]:** VM 내부의 에이전트는 권한 인가 권한이 없으며, 호스트 측 분리 데몬인 Sentinel이 커넥터 액션 및 네트워크 egress의 유일한 인가 기관으로 동작합니다. 모든 평가는 `(connector, method, action_class, scope, destination, HTTP method/path)`를 바탕으로 `Allow`, `Deny`, `Ask`를 반환합니다 (출처: security.muse.ai).
- **Out-of-band 결정론적 승인 카드 [1차]:** 승인 다이얼로그는 에이전트와의 채팅창 텍스트가 아니라 클라이언트 전용 out-of-band 카드 UI로 전송되어, 프롬프트 인젝션 공격자가 대화 내용으로 승인을 위조하는 것을 원천 차단합니다 (출처: research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse).
- **5단계 Grant 스코프 [1차]:** 승인 범위는 `Once`(1회), `Session`(세션 단위), `Task`(작업 단위), `Time-bounded/Until`(시간 제한), `Perpetual/Always`(영구)의 5가지로 명확히 제한됩니다 (출처: security.muse.ai, meta.com/help/1385290430137537).
- **Tainted Egress [1차]:** 사용자 개인 데이터나 외부 입력을 읽은 프로세스는 오염(taint) 상태로 전환되어 기존의 자동 허용(auto-allow) 권한을 상실하고 반드시 Ask로 강등됩니다 (출처: security.muse.ai).
- **비가역성(Irreversibility) 명시 [1차]:** 발송된 이메일 등 되돌릴 수 없는 작업은 실행 전 사용자에게 취소 불가능성을 고지합니다 (출처: meta.com/help/1385290430137537).
- **결함 사례 [미검증/2차]:** Marketplace 거래를 승인 없이 수락하고 집 주소를 유출했다는 사고 보고 및 버그 바운티 Tier 7에서 "자가 승인(self-approval)"을 핵심 우회 취약점으로 지정했습니다 (출처: futurism.com, bugbounty.meta.com/payout-guidelines/muse).

### 2.3 xAI Grok Bot
- **쉘 명령 사전 검토 및 큐잉 [1차]:** 별도 모델이 모든 쉘 명령을 사전 검토하며, 오너 메시지만 인터럽트하고 백그라운드 이벤트는 비중단 큐로 처리합니다 (출처: Lee Robinson Stanford 강연 transcript).
- **채널 간 정보 유출 사고 [2차]:** CFO 봇이 동명의 공개 Slack 채널에 개인 은행 명세서를 전송하는 사고가 발생하여, 채널 간 정보 이동 및 목적지 바인딩 시 반드시 소유자 허가를 받도록 수정되었습니다 (출처: Business Insider 2026).

### 2.4 Instinct (Spear Street Technology)
- **체계적 승인 게이트 부재 [2차]:** 명시적인 승인 UI 없이 사용자 대신 구속력 있는 계약을 맺을 수 있도록 설계되었습니다 (출처: TechCrunch 2026-08-24).
- **무승인 발송 및 인젝션 실행 사고 [2차]:** 사용자 확인 없이 이메일을 발송하고, 수신된 이메일 본문의 프롬프트 인젝션을 그대로 실행하여 계정이 정지되거나 원치 않는 동작이 유발되었습니다 (출처: TechCrunch 2026).

### 2.5 Pickle (YC W25)
- **결제 승인 카드 및 재확인 [1차]:** 결제 직전 주문 내역과 총액이 표기된 승인 카드를 띄우며, 금액이 변동되면 즉시 재확인합니다 (출처: pickle.com/assets/security-n1i5zLW2.js).
- **외부 응대 에이전트 도구 제한 [1차]:** 타인 또는 외부 에이전트와 대화하는 에이전트는 읽기와 대화 도구만 부여받으며, 결제/쓰기/자격증명 도구는 물리적으로 박탈됩니다 (출처: pickle.com/assets/faq-pFbILNx9.js).

### 2.6 Hark (Hark Pro / Handoff)
- **자율 권한 점진 확장 [1차]:** 기본값은 자금 지출이나 메시지 전송 전 항상 확인(Ask)하는 것이며, 사용자의 선택 패턴을 학습하여 범주별 상시 권한(standing permission)으로 전환합니다 (출처: hark.com/terms, hark.com/security).

---

## 3. ## 현재 코드

현재 omon-gateway 코드베이스를 직접 열어 확인한 승인 및 보안 로직의 실제 상태는 다음과 같습니다.

### 3.1 omo 백엔드 승인 바이패스 및 맹목적 거부 루프
- **자동 승인 바이패스 (`src/agent/omo_backend.rs:1456-1473`):**
  ```rust
  let auto_approve = !matches!(
      std::env::var("APPROVAL_MODE")
          .or_else(|_| std::env::var("APPROVAL_POLICY"))
          .as_deref(),
      Ok("always")
  );

  if !toolset_blocked && auto_approve {
      tracing::info!(
          session = %session.key,
          "approving daemon tool request under gateway approval policy"
      );
      let _ = ws.send(approval_allow_response(req_id)).await;
      continue;
  }
  ```
  `APPROVAL_MODE`나 `APPROVAL_POLICY` 환경변수가 `"always"`로 설정되어 있지 않으면 `auto_approve`가 무조건 `true`가 되어 omo app-server가 요청한 모든 도구 호출을 승인합니다.
- **`APPROVAL_MODE=always` 설정 시 맹목적 거부 루프 (`src/agent/omo_backend.rs:1474-1495`):**
  ```rust
  approval_denials += 1;
  if approval_denials >= APPROVAL_DENIAL_TURN_LIMIT {
      tracing::error!(
          denials = approval_denials,
          "approval denial loop: aborting turn instead of flailing until the deadline"
      );
      if let Some(turn_id) = &turn_id {
          let interrupt = json!({
              "jsonrpc": "2.0",
              "id": 9_002,
              "method": "turn/interrupt",
              "params": { "threadId": thread_id, "turnId": turn_id }
          });
          let _ = ws.send(Message::text(interrupt.to_string())).await;
      }
      self.active_turns.lock().remove(&session.key.storage_key());
      return Err(OmonError::Llm(format!(
          "tool approval denied {approval_denials} times by gateway policy; turn aborted"
      )));
  }
  let _ = ws.send(approval_denial_response(req_id)).await;
  continue;
  ```
  환경변수를 `"always"`로 설정하더라도 사용자에게 대화형 승인을 요청하는 것이 아니라, 단순히 게이트웨이 정책 거부 메시지(`approval_denial_response`)를 5회 연속 회신한 뒤 턴 인터럽트(`turn/interrupt`)를 전송하고 에러로 중단합니다 (`src/agent/omo_backend.rs:33-34`의 `APPROVAL_DENIAL_TURN_LIMIT = 5`). 즉, 대화형 승인 인터페이스가 전혀 연결되어 있지 않습니다.
- **단순 문자열 기반 승인 요청 감지 (`src/agent/omo_protocol.rs:151-155`):**
  `is_approval_request` 함수는 메서드명에 `"Approval"`, `"requestApproval"`, 또는 `"item/"` 시작 및 `"/request"` 종료 패턴만 단순 검사하며, 어떤 도구가 어떤 인자로 호출되었는지, 어떤 Effect를 유발하는지 파악하지 못합니다.
- **하드코딩된 JSON-RPC 응답 (`src/agent/omo_protocol.rs:122-149`):**
  `approval_allow_response`는 `{"allow": true, "decision": "accept"}`를, `approval_denial_response`는 `{"allow": false, "decision": "decline", "reason": "denied by gateway policy..."}`를 고정 생성합니다.

### 3.2 터미널 도구의 고립된 승인 시스템
- **정책 및 스캔 구조체 (`src/tools/terminal.rs:14-20`, `38-70`):**
  `TerminalTool`은 `ApprovalPolicy` (`Smart`, `Always`, `Never`), `approval_timeout` (기본 120초), `deny_globs`, `external_scanner` (Tirith)를 보유하며 기본값은 `Smart`입니다.
- **다단계 명령어 검사 (`src/tools/terminal.rs:202-306`):**
  - `src/tools/terminal.rs:207-211`: `detect_hardline_command` (`src/security/hardline.rs:25-50`)를 통해 루트 삭제(`rm -rf /`), 파일시스템 포맷(`mkfs`), 원시 블록 쓰기(`dd`) 등 하드라인 명령을 무조건 거부합니다.
  - `src/tools/terminal.rs:213-217`: `match_user_deny_rule`을 통해 `self.deny_globs` (`APPROVALS_DENY`)에 매칭되는 명령을 차단합니다.
  - `src/tools/terminal.rs:249-273`: `detect_dangerous_command` (`src/security/dangerous.rs:9-65`)를 통해 위험 명령(`DangerousFinding`)을 탐지하고 Tirith 스캐너 결과를 결합합니다.
  - `src/tools/terminal.rs:288-294`: 승인이 필요하면 `tokio::time::timeout(self.approval_timeout, requester.request_approval_with_max_scope(...))`를 호출하여 최대 120초간 동기 블로킹 대기합니다.

### 3.3 Discord 전용 메모리 큐 승인 요청자
- **승인 결정 및 스코프 모델 (`src/discord/approval.rs:18-35`, `177-183`):**
  `ApprovalDecision` (`Once`, `Session`, `Always`, `Deny { reason }`)과 `ApprovalScope` (`Once`, `Session`, `Always`)를 정의합니다.
- **인메모리 원샷 채널 기반 프롬프트 (`src/discord/approval.rs:54-70`, `290-340`):**
  `ApprovalPrompt`는 `oneshot::Receiver<ApprovalDecision>`과 `PendingLease`를 사용하여 메모리 해시맵에 보관됩니다. 게이트웨이가 재시작되거나 120초가 지나면 프롬프트가 즉시 유실되며 fail-closed(Deny)됩니다.
- **Discord 단일 채널 종속 (`src/discord/approval.rs:196-285`):**
  `DiscordApprovalRequester`만이 `ApprovalRequester` 트레이트를 구현하고 있으며, 버튼 컴포넌트 렌더링이 Discord 메시지 시스템에 하드코딩되어 있습니다.

### 3.4 자율 작업 레인의 승인 불능 (Fatal Crash)
- **작업 레인의 승인 요청 거부 (`src/lanes/runner.rs:661-665`):**
  ```rust
  OutboundAction::ApprovalRequest { .. } => {
      return Err(OmonError::Config(
          "lane approval requires lead intervention".into(),
      ));
  }
  ```
  장시간 작업 레인 실행 중 `OutboundAction::ApprovalRequest`가 발생하면 대기하거나 사용자에게 알리는 것이 아니라 즉시 `OmonError::Config("lane approval requires lead intervention")` 에러를 던지고 레인을 폭파시킵니다.
- **터미널 명령어 전용 이벤트 모델 (`src/models/events.rs:379-384`):**
  ```rust
  ApprovalRequest {
      session: SessionKey,
      request_id: Uuid,
      command: String,
      reason: String,
  },
  ```
  이벤트 정의가 오직 쉘 명령어(`command: String`)만을 담도록 설계되어 있어, 파일 수정(`Write`), 외부 메시지 발송(`SendExternal`), 결제(`Spend`) 등의 일반 액션 페이로드를 표현할 수 없습니다.

### 3.5 빈약한 영속화 스키마
- **승인 허용 목록 테이블 (`migrations/0007_approval_allowlist.sql:1-4`):**
  ```sql
  CREATE TABLE IF NOT EXISTS approval_allowlist (
      pattern_key TEXT PRIMARY KEY NOT NULL,
      created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
  ```
  `pattern_key`와 `created_at` 컬럼만 존재하며, 승인자(actor), 만료 시각(TTL), 승인 사유, 적용 범위(scope), 취소(revocation) 기록이 전혀 없습니다.
- **스킬/메모리 전용 스테이징 테이블 (`migrations/0010_pending_writes.sql:1-6`):**
  `pending_writes`는 `kind`가 `'memory' | 'skill'`로 고정되어 있어 범용 도구 및 액션 승인에는 재사용되지 않고 있습니다.

### 3.6 선행 계획서의 오류 및 변경점
1. **`APPROVAL_MODE=always`의 실제 동작 오류:** 선행 계획서 4절은 `src/agent/omo_backend.rs:1456-1470`을 인용하며 `APPROVAL_MODE != always`이면 자동 승인이라고 지적했습니다. (현재 라인은 `src/agent/omo_backend.rs:1456-1473`). 그러나 선행 계획서는 `APPROVAL_MODE=always`로 설정하면 대화형 승인이 작동할 수 있는 것처럼 서술했으나, 실제 소스 코드 `src/agent/omo_backend.rs:1474-1495`를 보면 대화형 승인 UI 연결이 전혀 없으며 5회 연속 거부 후 세션을 강제 종료하는 맹목적 차단 루프입니다.
2. **`TerminalTool` 승인의 실효성 오판:** 선행 계획서는 터미널 승인을 "운영 중"으로 분류했으나, 실제 사용자가 대화하는 메인 omo 백엔드와 자율 레인은 `omo_backend.rs`를 통해 직접 도구를 실행하므로 터미널 도구의 승인 가드가 우회됩니다. 또한 레인에서 승인 발생 시 `src/lanes/runner.rs:661-665`에 의해 레인이 크래시되므로 실질적으로 자율 환경에서 승인은 완전히 불가능한 상태입니다.
3. **페이로드 모델 한계 누락:** 선행 계획서는 `OutboundAction::ApprovalRequest`가 오직 `command: String`만을 수용하는 구조적 결함(`src/models/events.rs:379-384`)을 간과했습니다.

---

## 4. ## 구현 단계

본 구현은 기존 기능을 깨뜨리지 않고 독립 배포 가능한 순차적 단계로 진행하며, 신규 기능은 모두 환경변수 킬 스위치(`OMON_APPROVAL_EFFECTS_ENABLED`, 기본값: `false`)로 제어합니다.

```
+---------------------------------------------------------------------------------------+
| Step 1: Policy Core Engine                                                            |
| Effect (Read, Draft, Write, SendExternal, Spend, Irreversible) & Decision Evaluation |
+-------------------------------------------+-------------------------------------------+
                                            |
+-------------------------------------------v-------------------------------------------+
| Step 2: Database Migration 0045_approval_effects.sql                                  |
| Persistent tables: approval_requests, approval_grants, approval_allowlist extension   |
+-------------------------------------------+-------------------------------------------+
                                            |
+-------------------------------------------v-------------------------------------------+
| Step 3: omo Backend Interception Integration                                          |
| src/agent/omo_backend.rs tool call interception, payload hashing & policy evaluation   |
+-------------------------------------------+-------------------------------------------+
                                            |
+-------------------------------------------v-------------------------------------------+
| Step 4: Lane Runner Asynchronous Approval Suspension                                 |
| src/lanes/runner.rs Blocked state transition, lead notification & wake on resolve     |
+-------------------------------------------+-------------------------------------------+
                                            |
+-------------------------------------------v-------------------------------------------+
| Step 5: Multi-channel Approval Adapters & CLI Management                              |
| Discord action card (hash8), CLI approve/deny commands & background expiry worker     |
+---------------------------------------------------------------------------------------+
```

### 단계 1: 정책 핵심 엔진 모듈 구축 (Policy Core Engine)
- **변경/생성 파일:**
  - `src/policy/mod.rs` (신규)
  - `src/policy/effect.rs` (신규)
  - `src/policy/decision.rs` (신규)
  - `src/policy/canonical.rs` (신규: 인자 정규화 및 SHA-256 해싱)
- **내용:**
  - `Effect` 열거형 정의: `Read`, `Draft`, `Write`, `SendExternal`, `Spend`, `Irreversible`.
  - `Decision` 열거형 정의: `Allow`, `Ask(ApprovalRequestId)`, `HandOff`, `Deny { reason: String }`.
  - 도구 및 메서드 매핑:
    - `file_read`, `grep`, `find`, `list` -> `Effect::Read`
    - `draft_message`, `draft_email` -> `Effect::Draft`
    - `file_write`, `file_edit`, `patch` -> `Effect::Write`
    - `discord_send`, `imessage_send`, `mail_send`, `webhook_post` -> `Effect::SendExternal`
    - `stripe_pay`, `purchase`, `spend_lease` -> `Effect::Spend`
    - `rm -rf`, `mkfs`, `dd`, `passwd`, `sudo`, `systemctl stop` -> `Effect::Irreversible`
  - 결정 우선순위:
    1. 하드라인 차단 (`src/security/hardline.rs`) 및 Deny 규칙 -> `Decision::Deny`
    2. 비밀번호 변경, 자금 이체, 2FA -> `Decision::HandOff` (대리 실행 절대 불가)
    3. `Read`, `Draft` -> 기본 `Decision::Allow`
    4. `Write` -> 워크스페이스 내부는 `Decision::Allow`, 외부 경로는 `Decision::Ask`
    5. `SendExternal`, `Spend`, `Irreversible` -> 기본 `Decision::Ask`
- **수락 시나리오 (기계 검증):**
  - CLI 명령어: `omo-gateway policy eval --effect send_external --action '{"dest":"discord:123","body":"hello"}'`
  - 기대 출력: `{"decision":"ask","effect":"send_external","action_hash":"..."}` (종료 코드 0).

### 단계 2: 지속형 승인 원장 DB 마이그레이션 (0045_approval_effects.sql)
- **변경/생성 파일:**
  - `migrations/0045_approval_effects.sql` (신규)
  - `src/storage/approval_store.rs` (신규)
- **스키마 정의:**
  ```sql
  -- 승인 요청 테이블
  CREATE TABLE IF NOT EXISTS approval_requests (
      id TEXT PRIMARY KEY NOT NULL,              -- req_uuid
      lane_id TEXT,                              -- NULL이면 대화 세션
      session_key TEXT NOT NULL,
      effect TEXT NOT NULL,                      -- Read, Draft, Write, SendExternal, Spend, Irreversible
      action_kind TEXT NOT NULL,                 -- tool name 또는 action name
      action_json TEXT NOT NULL,                 -- canonical JSON payload
      action_hash TEXT NOT NULL,                 -- sha256(canonical JSON)
      status TEXT NOT NULL,                      -- proposed, pending, approved, denied, expired, executing, settled
      scope TEXT NOT NULL,                       -- once, task, until, standing
      requested_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
      expires_at TIMESTAMP NOT NULL,             -- 기본 600초(10분)
      decided_by TEXT,                           -- discord:owner_id, cli, timeout
      decided_at TIMESTAMP,
      reason TEXT
  );

  CREATE INDEX IF NOT EXISTS idx_approval_requests_pending
      ON approval_requests(status, expires_at);
  CREATE INDEX IF NOT EXISTS idx_approval_requests_hash
      ON approval_requests(action_hash);

  -- 사전 승인 권한(Grant) 테이블
  CREATE TABLE IF NOT EXISTS approval_grants (
      id TEXT PRIMARY KEY NOT NULL,
      scope_kind TEXT NOT NULL,                  -- once, task, until, standing
      lane_id TEXT,                              -- task 스코프인 경우 바인딩
      effect TEXT NOT NULL,
      matcher_json TEXT NOT NULL,                -- destination, tool, args pattern
      max_amount_minor INTEGER,                  -- Spend용 상한선
      expires_at TIMESTAMP,
      created_from_request TEXT REFERENCES approval_requests(id),
      revoked_at TIMESTAMP
  );

  -- 기존 approval_allowlist 보강 (하위 호환)
  ALTER TABLE approval_allowlist ADD COLUMN ttl_seconds INTEGER DEFAULT NULL;
  ALTER TABLE approval_allowlist ADD COLUMN actor TEXT DEFAULT 'system';
  ALTER TABLE approval_allowlist ADD COLUMN reason TEXT DEFAULT NULL;
  ALTER TABLE approval_allowlist ADD COLUMN revoked_at TIMESTAMP DEFAULT NULL;
  ```
- **수락 시나리오 (기계 검증):**
  - SQL 조회: `sqlite3 ~/.omon/state/gateway.db "PRAGMA table_info(approval_requests);" | grep "action_hash"`
  - 기대 결과: `6|action_hash|TEXT|1||0` 확인 (종료 코드 0).

### 단계 3: omo 백엔드 승인 인터셉터 연결
- **변경 파일:**
  - `src/agent/omo_backend.rs`
  - `src/models/events.rs`
- **내용:**
  - `OMON_APPROVAL_EFFECTS_ENABLED` 킬 스위치 도입: 비활성화 시 기존 `auto_approve` 분기 유지.
  - 활성화 시 `src/agent/omo_backend.rs:1456-1495` 분기를 교체:
    - 수신된 JSON-RPC 도구 요청의 메서드 및 파라미터를 파싱하여 `Effect` 분류 및 `canonical action JSON` 생성.
    - `action_hash = sha256(canonical action JSON)` 계산.
    - 활성 `approval_grants` 테이블 조회: 일치하는 유효 Grant가 존재하면 즉시 `approval_allow_response` 전송.
    - Grant가 없고 정책 판정이 `Ask`인 경우:
      - `approval_requests` 테이블에 `status = 'pending'`, `expires_at = now + 10분`으로 영속화.
      - 세션 컨텍스트에 승인 대기 등록 후 비동기 대기(turn suspension).
      - 디스패처를 통해 오너에게 승인 요청 카드 이벤트 발송.
    - 판정이 `Deny`인 경우: 즉시 `approval_denial_response` 전송.
    - 판정이 `HandOff`인 경우: 대리 실행 거부 안내 메시지와 함께 턴 중단.
- **수락 시나리오 (기계 검증):**
  - 로그 확인: 게이트웨이 로그에서 정책 인터셉트 라인 검증
  - CLI 명령어: `grep "policy decision: Ask effect=SendExternal" ~/.omon/logs/gateway.log`
  - 기대 결과: 해당 로그 라인 출력 (종료 코드 0).

### 단계 4: 작업 레인(Lane) 비동기 승인 서스펜션 구현
- **변경 파일:**
  - `src/lanes/runner.rs`
  - `src/lanes/state.rs`
- **내용:**
  - `src/lanes/runner.rs:661-665`의 `OutboundAction::ApprovalRequest` 수신 시 강제 에러 발생 로직 제거.
  - 레인 상태 머신에 `Blocked(BlockedReason::ApprovalPending(Uuid))` 상태 지원 추가.
  - 레인 러너가 `ApprovalRequest`를 수신하면 레인 트랜스크립트에 `[approval requested: <effect> <hash8>]`를 기록하고 레인을 `Blocked` 상태로 안전하게 일시 중단.
  - 승인이 완료(`status = 'approved'`)되거나 거부(`status = 'denied' | 'expired'`)되면 레인 큐(`posts_ready`)를 깨워 해당 결과로 턴 재개.
- **수락 시나리오 (기계 검증):**
  - CLI 명령어: `omo-gateway lane list --format json | jq '.lanes[] | select(.lane_key=="test-lane") | .state'`
  - 기대 결과: `"Blocked"` 출력 및 SQL `SELECT status FROM approval_requests WHERE lane_id = 'test-lane';` 결과 `pending` 확인.

### 단계 5: 멀티채널 승인 어댑터 및 CLI 도구 제공
- **변경/생성 파일:**
  - `src/policy/adapter.rs` (신규: 채널 중립 승인 렌더러)
  - `src/discord/approval.rs` (지속형 큐 어댑터로 리팩토링)
  - `src/cli/approval.rs` (신규 CLI 서브커맨드)
  - `src/policy/reaper.rs` (신규: 만료 처리 백그라운드 태스크)
- **내용:**
  - Discord 버튼 승인 시 `action_hash` 앞 8자리(`hash8`)를 버튼 라벨에 표시하여 변조 방지.
  - CLI 서브커맨드 구현:
    - `omo-gateway approval list`: 대기 중인 승인 목록 출력
    - `omo-gateway approval approve <id> [--scope once|task|until|standing]`: 승인 처리 및 Grant 생성
    - `omo-gateway approval deny <id> [--reason ...]`: 승인 거부 처리
  - 백그라운드 리퍼 태스크: 매 30초마다 `expires_at < CURRENT_TIMESTAMP AND status = 'pending'`인 요청을 `expired`로 갱신하고 fail-closed 처리.
- **수락 시나리오 (기계 검증):**
  - CLI 명령어: `omo-gateway approval list --format json`
  - 기대 결과: 대기 중인 JSON 배열 반환 (종료 코드 0).
  - CLI 명령어: `omo-gateway approval approve req_sample --scope once`
  - SQL 검증: `sqlite3 ~/.omon/state/gateway.db "SELECT status, decided_by FROM approval_requests WHERE id = 'req_sample';"` -> `approved|cli`.

---

## 5. ## 위험·의존성

### 5.1 다른 테마와의 의존성 매핑
승인 및 영향도 제어(`approval-effects`)는 자율 플랫폼의 최하단 안전 기반(P0)으로서 다음과 같은 테마들과 긴밀히 맞물려 있습니다.

- **`approval-grants` (선행/동시 필수):**
  영향도 분류로 `Ask`가 발생하는 빈도가 높아질 때, 매번 단발성 승인(`Once`)만 강제하면 오너의 승인 피로(alert fatigue)가 극에 달해 모든 요청을 맹목적으로 승인하게 됩니다. 동일 작업 레인(`task`), 일정 시간(`until`), 영구 화이트리스트(`standing`) 스코프를 지원하는 권한 부여(Grant) 메커니즘이 반드시 함께 구현되어야 합니다.
- **`outbox-destination` (직접 연계):**
  `SendExternal` 영향도를 가진 모든 작업(Discord, iMessage, 카카오톡, 이메일, 웹훅)은 즉시 발송되지 않고 반드시 `outbox` 테이블에 `Draft` 상태로 적재된 뒤 승인을 거쳐야 합니다. Grok Bot의 공개 Slack 은행정보 유출 사고를 방지하기 위해 불변의 `destination_id` 바인딩이 필수적입니다.
- **`taint` (보안 결합):**
  인박스 메일, 웹 스크래핑, 외부 메시지 등 비신뢰(untrusted) 데이터를 읽은 레인은 즉시 오염(taint) 플래그가 설정되어야 합니다. Meta Muse의 Tainted Egress 규격에 따라, 오염된 레인은 `approval-grants`에 상시 허용(`Standing`)이 등록되어 있더라도 `SendExternal` 및 `Spend`에 대해 강제로 `Ask`로 강등되어야 합니다 (Rule of Two 원칙 준수).
- **`notify-envelope` (알림 채널 연계):**
  생성된 승인 요청(`approval_requests`)은 채널 독립적인 `Envelope::Review` 형태로 변환되어 오너의 알림 채널(Discord DM, 터미널, Slack 등)로 전송되어야 합니다. 야간 수면 시간(KST 00:00~08:00) 동안은 무차별 알림을 억제하고 아침 브리핑 다이제스트로 수렴시키는 정책이 필요합니다.
- **`kibitzer-action` (선제 행동의 안전 가드):**
  키비처가 사용자 승인 없이 자율적으로 백그라운드 레인을 기동(`start_lane`)할 때, 해당 레인은 OpenAI Dots의 Proactive Research 규격을 벤치마킹하여 초기 프로필을 `Read`/`Draft`만 허용되는 읽기 전용 상태로 강제해야 하며, 외부 쓰기나 발송 시도시 반드시 `approval-effects` 게이트에 걸리도록 해야 합니다.
- **`usage-ledger` (비용 통제 연계):**
  토큰 폭주 및 API 지출을 감시하는 사용량 원장과 연계하여, 일일 예산 또는 턴당 비용 임계치를 초과하는 작업은 자동으로 `Spend` 영향도로 상향되어 오너의 명시적 승인을 받도록 유도합니다.
- **`sandbox` (실행 격리 보완):**
  `Irreversible` 영향도를 가진 쉘 명령이나 파일 시스템 변경은 순수 승인 프롬프트에만 의존할 것이 아니라, Apple Container 또는 Seatbelt 기반의 격리 샌드박스 내부에서 우선 격리 실행되어야 합니다.

### 5.2 핵심 리스크 및 대응 방안

1. **무인 자율 작업의 장시간 정체 (Unattended Hang Risk):**
   - *위험:* 백그라운드 cron 자가 수리나 야간 작업 레인이 `Ask` 결정을 만났을 때, 오너가 부재(수면, 외출) 중이면 승인을 받지 못하고 무한 대기하거나 타임아웃으로 실패할 수 있습니다.
   - *대응:* 승인 요청에 명시적 만료 시각(`expires_at`, 기본 10분)을 두고, 만료 시 fail-closed(`expired` -> `denied`) 처리합니다. 레인은 크래시되지 않고 실패 사유를 구조화하여 트랜스크립트에 남긴 후 안전하게 종료(`Blocked` -> `Failed` 또는 `Suspended`)되며, 아침 브리핑 다이제스트를 통해 오너에게 보고됩니다.
2. **액션 페이로드 변조 및 해시 불일치 (Mutation / Hash Drift):**
   - *위험:* LLM 에이전트가 재시도 과정에서 명령어나 본문의 공백, 따옴표 등을 1바이트라도 변경하면 계산된 `action_hash`가 달라져 기승인된 권한이 매칭되지 않거나, 반대로 악의적 인젝션 페이로드가 기존 승인을 도용할 위험이 있습니다.
   - *대응:* JSON 직렬화 시 키 정렬과 공백 제거를 강제하는 엄격한 표준 정규화(Canonical JSON serialization)를 구현하여 해시를 산출합니다. 사전 승인(`approval_grants`) 매칭 시 단순 문자열 비교가 아닌 구조적 매처(정규화된 JSONPath 및 정규식)를 적용합니다.
3. **오너 승인 피로도 및 습관적 승인 (Approval Fatigue):**
   - *위험:* 사소한 파일 수정이나 읽기 작업에도 빈번하게 승인 카드가 발송되면 오너가 내용을 확인하지 않고 무조건 승인 버튼을 누르는 보안 무력화 현상이 발생합니다.
   - *대응:* `Read`와 `Draft`는 기본적으로 `Allow`로 완전 자동화하고, 위험한 외부 전송(`SendExternal`), 실제 결제(`Spend`), 파괴적 쉘 명령(`Irreversible`)에 한해서만 선별적으로 `Ask`를 발생시킵니다. 안전한 반복 작업은 `approval-grants`를 통해 작업(`task`) 단위로 묶어 1회 승인으로 전체 배치를 인가합니다.
4. **omo app-server 연결 타임아웃 및 활성 감시(Watchdog) 경합:**
   - *위험:* 도구 승인을 대기하는 동안 omo WebSocket 연결이 장시간 유휴(idle) 상태가 되어 게이트웨이의 무응답 감시자(45초 침묵 감시, 30분 데드라인)가 오작동하거나 app-server 데몬이 강제 재시작될 위험이 있습니다.
   - *대응:* 도구 승인 대기 중에는 게이트웨이가 omo WebSocket에 세션 일시중단(Session suspension) 상태를 정상 인지시키거나 주기적 keep-alive 핑을 전송하여 활성 감시자(liveness supervisor)가 비정상 행(hang)으로 오판하지 않도록 상태 전이를 명확히 동기화합니다.

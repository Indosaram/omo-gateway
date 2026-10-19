# 벤치마크 보고서: payments-a2a (결제 임대·지출 원장 및 에이전트 간 통신)

- 작성일: 2026-10-17
- 대상 저장소: `omon-gateway` (단일 소유자 macOS 호스트 데몬)
- 테마: `G15 payments & A2A` (단일 사용 임대, 지출 원장, Stripe Agentic Commerce, Pickle 카드 확인, A2A/Agent Card 프로토콜)

---

## 판정

**보류 (later)**

omon-gateway는 단일 소유자(1-owner)의 개인 Mac 환경에서 실행되는 로컬 데몬으로, 주요 상호작용 채널은 Discord(및 Slack/iMessage/Kakao)이며 코딩 작업, 크론 자동화 및 받은 편지함 분류를 중심으로 동작합니다. 현재 시스템은 도구 실행 수준에서 자동 승인(`src/agent/omo_backend.rs:1456-1470`)에 의존하고 있어 비가역 동작을 차단하는 기본 안전 계층(`approval-effects`, `approval-grants`, `taint`)조차 갖춰지지 않았습니다. 이 상태에서 실제 금융 자산이 이동하는 결제(Stripe Link/SPT)나 외부 임의 에이전트와의 A2A 통신을 개방하는 것은 자금 탈취 및 프롬프트 인젝션 전파 위험이 극히 높습니다. 따라서 P0/P1 안전 프레임워크와 자격증명 격리(`credentials`)가 완료된 후, 사서함/코딩 레인 연동용 A2A 서버 규격과 Stripe Link 가상 임대(Lease) 지출 원장을 단계적으로 도입하는 것이 타당합니다.

---

## 벤치마킹 원천

| 플랫폼 / 표준 | 메커니즘 및 접근 방식 | 출처 태그 및 증거 |
|---|---|---|
| **Stripe Agentic Commerce (Link Agent Wallet / SPT)** | **단일 사용 지출 임대(Single-Use Spend Lease) 및 공유 결제 토큰(SPT)**: 에이전트는 결제 비밀키(카드번호/CVC)를 직접 보유하지 않고, `SpendRequest`를 생성하여 오너에게 승인을 요청. Link 앱 또는 웹 훅 승인 후 가맹점·금액·만료 시한에 바인딩된 일회용 카드 또는 공유 결제 토큰(SPT, `/v1/shared_payment/issued_tokens`)을 발급받아 결제 처리. 한 번 사용되거나 금액/목적지가 불일치하면 즉시 무효화. | `[1차]` https://docs.stripe.com/agentic-commerce/agents/link-agent-wallet/use-link-wallet-pay-online.md , https://docs.stripe.com/agentic-commerce/link-cli.md (`.omo/ulw-research/20261016-133102-autonomous-agents/wave2/payments-creds.md`) |
| **Pickle** | **결제 카드 대화형 재확인(Card Confirm)**: 물품 구매나 결제 실행 전 주문 내역, 총액, 결제 버튼이 포함된 결정론적 승인 카드를 띄우고, 최종 결제 금액이 최초 합의 금액보다 1원이라도 인상되면 재확인을 강제. 카드 접근 권한은 앱 사용 후 30일로 만료. | `[1차]` https://pickle.com/assets/faq-pFbILNx9.js (`docs/research/autonomous-agent-platform-plan.md` 섹션 3) |
| **Meta Muse** | **Stripe Link 일회용 가상카드 바인딩**: 결제 도구 호출 시 가맹점, 금액, 만료 시간이 엄격히 묶인 일회용 카드를 생성하여 전달. Sentinel 승인 및 Tainted egress 감시를 통해 인젝션된 공격자가 임의 가맹점으로 송금하지 못하도록 차단. | `[1차]` http://security.muse.ai/ , https://research.meta.ai/blog/security-and-safety-for-ai-agents-our-approach-with-muse |
| **Hark (Hark Pro / Handoff)** | **Action Cards 기반 선제 결제 제안**: 영화표 예매, 공과금 납부, 식비 카드 잔액 부족 감지 시 proactive thinking으로 충전을 제안하고 승인 버튼(Pay, Cancel)을 표시. 자율 집행은 오너의 명시적 동의(consent) 하에서만 점진적으로 허용. | `[1차]` https://hark.com/terms , https://hark.com/security (`.omo/ulw-research/20261016-133102-autonomous-agents/wave1/hark-safety.md`) |
| **A2A Protocol (Agent2Agent)** | **에이전트 카드 및 작업 기반 상호 통신**: `/.well-known/agent-card.json`을 통한 에이전트 기술(Skill) 디스커버리, OAuth 범위 기반 상호 인증, 태스크 생명주기 관리(상태 머신, SSE/웹훅 콜백). 비공개 확장 기술은 인증 뒤로 은닉. v1.0.0 (2026-03-12), v1.0.1 (2026-05-28) 릴리스. | `[1차]` https://github.com/a2aproject/A2A/releases (`.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-a2a.md`) |
| **AP2 (Agent Payment Protocol)** | **개방/폐쇄 위임(Open/Closed Mandate) 및 원장(Ledger)**: 사용자가 승인한 상위 규칙(개방 위임: 허용 수취인, 예산, 유효기간) 하에서 구체적인 거래(폐쇄 위임)를 매칭 검증. 트랜잭션 단위 지출 누적 원장을 기록하여 이중 지출(Double-spend) 방지. LLM을 잠재적 공격자로 간주하고 자격증명 직접 노출 배제. | `[1차]` AP2 보안 명세서 (`.omo/ulw-research/20261016-133102-autonomous-agents/wave2/payments-creds.md`) |

---

## 현재 코드

omon-gateway의 현행 코드베이스에는 금융 결제 및 A2A 통신을 위한 엔진이나 데이터 스키마가 전혀 존재하지 않으며, 결제 및 외부 통신과 관련된 로직은 일부 텍스트 안내 및 승인 모듈의 메모리 락 용어 수준에 국한되어 있습니다.

1. **도구 승인 정책의 사실상 무력화**:
   - `src/agent/omo_backend.rs:1456-1470`: 환경변수 `APPROVAL_MODE`가 `"always"`로 명시되지 않은 한 `auto_approve`가 `true`로 설정되어 데몬 도구 요청이 게이트웨이 정책에 의해 자동 승인(allow)됩니다. 즉, 결제나 외부 발송 같은 위험 도구가 등록되더라도 현재 상태로는 오너 승인 없이 즉시 실행될 위험이 있습니다.
2. **Discord 대화형 승인 프롬프트 및 `PendingLease`**:
   - `src/discord/approval.rs:55-75`: `ApprovalPrompt` 구조체 내부에서 `PendingLease`를 사용하여 미결(Pending) 상태의 승인 요청 생명주기를 메모리 상(`HashMap<Uuid, PendingApprovalEntry>`)에서 관리합니다. 하지만 이는 디스코드 버튼 응답 대기용 일회성 인메모리 뮤텍스 락일 뿐, 금융 지출 임대(Spend Lease)나 SQLite 기반 영속 임대와는 무관합니다.
3. **인박스 분류 기준에서의 수동 결제 안내**:
   - `src/inbox/triage.rs:23`: 분류 프롬프트(`INSTRUCTIONS`)에 `"owner_must_act = needs the owner's own decision, signature, payment or a personal reply"`로 명시되어 있습니다. 수신 메일/메시지 중 결제(payment)나 서명이 필요한 사안은 에이전트가 처리하지 않고 오너의 직접 행동(`owner_must_act`)으로 분류하여 차단하고 있습니다.
4. **크론 장애 복구에서의 결제/할당량 오너 이관**:
   - `src/cron/remediate.rs:315-322`: 크론 작업 실패 시 원인 분류 중 `"needs_owner_login_or_quota"`로 판정되면 `"Restore the job's login/credentials/payment, then approve a rerun"` 메시지와 함께 오너 수동 조치(`self.needs_owner`)로 이관합니다. 게이트웨이가 스스로 결제하거나 충전하는 경로는 원천 배제되어 있습니다.
5. **원장(Ledger) 모듈의 전달 보증 한정**:
   - `src/ledger/mod.rs:1-6`: 게이트웨이 내부의 `ledger` 모듈은 메시지 전달 보증(`DeliveryLedgerEntry`, `DeliveryObligationState`)만을 담당합니다(`src/ledger/service.rs:1-30`). 지출 금액, 누적 한도, 가맹점 정보를 추적하는 `spend_ledger`나 트랜잭션 예산 테이블은 존재하지 않습니다.
6. **보안 규칙의 정적 명령어 중심성**:
   - `src/security/hardline.rs:1-20` 및 `src/security/dangerous.rs:1-45`: 현재 보안 계층은 `rm -rf`, `sudo`, `chmod` 등 셸 명령어 패턴 필터링에 집중되어 있으며, 네트워크 발송이나 금융 API 호출 등 의미론적 `Effect`(예: `Spend`, `SendExternal`)를 평가하는 정책 엔진이 없습니다.
7. **기존 계획서(`docs/research/autonomous-agent-platform-plan.md`)와의 불일치 및 변경 사항**:
   - 기존 계획서 섹션 5(G15) 및 6(P6)에서는 신규 모듈 `src/spend/` 및 마이그레이션 `0051_spend.sql`을 P6 단계로 계획하고 있습니다. 그러나 현재 마이그레이션은 `0044_tracker.sql`까지만 존재하며, P0 단계의 전제 조건인 `src/policy/`(`effect.rs`, `decision.rs`, `taint.rs` 등)가 전혀 구현되어 있지 않으므로 P6를 독립적으로 착수하는 것은 불가능합니다.

---

## 구현 단계

결제 및 A2A 기능은 안전 계층(P0)이 정착된 이후 활성화되도록 단계적으로 구성합니다. 모든 단계는 기본 비활성화(`OMON_PAYMENTS_ENABLED=0`, `OMON_A2A_ENABLED=0`) 상태로 머지됩니다.

### 1단계: 지출 임대 및 누적 원장 스키마 구축 (SQLite DDL)
- **변경/생성 파일**:
  - `migrations/0055_spend_ledger.sql` (신규)
  - `src/spend/mod.rs` (신규)
  - `src/spend/lease.rs` (신규)
  - `src/spend/ledger.rs` (신규)
  - `src/lib.rs` (`pub mod spend;` 등록)
- **DB 테이블**:
  - `spend_leases`: `id TEXT PRIMARY KEY, principal TEXT NOT NULL, destination TEXT NOT NULL, max_amount_minor INTEGER NOT NULL, currency TEXT NOT NULL, expires_at TEXT NOT NULL, single_use INTEGER NOT NULL DEFAULT 1, state TEXT NOT NULL, idempotency_key TEXT UNIQUE NOT NULL, created_at TEXT NOT NULL`
  - `spend_events`: `id TEXT PRIMARY KEY, lease_id TEXT NOT NULL REFERENCES spend_leases(id), amount_minor INTEGER NOT NULL, merchant TEXT NOT NULL, status TEXT NOT NULL, receipt_hash TEXT, settled_at TEXT, created_at TEXT NOT NULL`
- **Env 킬 스위치**:
  - `OMON_PAYMENTS_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  - CLI 명령: `sqlite3 ~/.omon/data.db "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('spend_leases', 'spend_events');"`
  - 기대 결과: 두 테이블 이름이 정확히 출력됨.

### 2단계: Stripe Agentic Commerce (Link Agent Wallet) 연동 및 Ask 승인 카드
- **변경/생성 파일**:
  - `src/spend/stripe_link.rs` (신규: Stripe Link API 클라이언트, SPT 발급 요청)
  - `src/discord/approval.rs` (수정: `SpendRequest` 전용 금액/가맹점/유효기간 표시 UI 카드 추가)
  - `src/policy/effect.rs` (`Effect::Spend` 연동)
- **Env 설정**:
  - `OMON_STRIPE_LINK_TOKEN_FILE="~/.omon/secrets/stripe-link-token"` (0600 권한)
  - `OMON_SPEND_PER_TX_CAP_KRW="50000"` (건당 한도)
  - `OMON_SPEND_DAILY_CAP_KRW="200000"` (일일 한도)
- **기계 검증 시나리오**:
  - CLI 명령: `cargo test --test spend_lease_policy test_over_limit_triggers_ask` (단, 본 턴에서는 실행하지 않고 설계만 확정)
  - 로그 검증: `grep "spend lease created" ~/.omon/gateway.log`
  - SQLite 검증: `sqlite3 ~/.omon/data.db "SELECT state FROM spend_leases WHERE id = 'test-lease-1';"` -> `RequiresAction`

### 3단계: A2A 프로토콜 수신 서버 및 Agent Card 배포
- **변경/생성 파일**:
  - `src/a2a/mod.rs` (신규)
  - `src/a2a/card.rs` (신규: `/.well-known/agent-card.json` 핸들러)
  - `src/a2a/server.rs` (신규: A2A Task 수신 및 레인 디스패치)
  - `src/dashboard.rs` (수정: Axum 라우터에 `/.well-known/agent-card.json` 및 `/api/a2a/tasks` 라우트 등록)
- **Env 킬 스위치**:
  - `OMON_A2A_ENABLED=0` (기본값 off)
  - `OMON_A2A_PEER_ALLOWLIST=""` (신뢰 가능한 피어 식별자 목록)
- **기계 검증 시나리오**:
  - CLI 명령: `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:9119/.well-known/agent-card.json`
  - 기대 결과: `OMON_A2A_ENABLED=0`일 때 `404 Not Found`, `1`일 때 `200 OK` 및 스키마 검증.
  - Taint 검증: A2A를 통해 유입된 모든 메시지는 `TaintedSource::A2APeer(id)`로 마킹되어 외부 발송/지출 도구 호출 시 승인 강제.

---

## 위험·의존성

### 의존성 테마 (Dependencies)
1. **`approval-effects` (P0 선행 필수)**:
   - 모든 결제 시도는 `Effect::Spend`, 외부 에이전트 메시지 발송은 `Effect::SendExternal`로 태깅되어야 하며, 오너의 명시적 승인 없이 실행되지 않도록 차단하는 분류 엔진이 필수입니다.
2. **`approval-grants` (P0 선행 필수)**:
   - 가맹점별/일별 상한 및 유효기간에 바인딩된 승인(Once, Until, Standing) 생명주기 관리 체계가 선행되어야 합니다.
3. **`taint` (P0 선행 필수)**:
   - A2A 피어로부터 수신된 태스크나 메시지는 신뢰할 수 없는 외부 입력(`untrusted taint`)으로 처리되어야 합니다. Taint가 전파된 컨텍스트에서는 모든 `Spend`와 `SendExternal`이 자동으로 Ask(승인 대기)로 격상되어야 합니다.
4. **`credentials` (P2 선행)**:
   - Stripe Link 토큰 및 A2A 상호 인증용 개인키/시크릿이 LLM 프롬프트에 평문으로 유출되지 않도록 Keychain 및 Surrogate 토큰 치환 구조가 준비되어야 합니다.
5. **`notify-envelope` 및 `outbox-destination` (P1 선행)**:
   - Discord 채널에 결제 카드(주문명, 금액, 가맹점, 확인 버튼)를 안정적으로 렌더링하고, 승인 결정 후에만 결제 트랜잭션을 디스패치하는 신뢰성 있는 아웃박스가 필요합니다.

### 주요 위험 (Risks)
1. **금융 자금 탈취 및 과다 결제 (Financial Drain)**:
   - LLM이 웹 브라우징이나 외부 메시지 처리 중 프롬프트 인젝션에 노출될 경우, 공격자의 결제 링크로 송금하거나 불필요한 구독을 유도할 위험이 있습니다. 건당/일일 엄격한 하드 리밋과 일회용 임대(Single-use Lease) 강제가 필수적입니다.
2. **A2A를 통한 인젝션 바이러스 전파**:
   - 외부 에이전트와의 A2A 통신을 무제한 허용할 경우, 악의적인 피어가 악성 프롬프트를 전송하여 오너의 로컬 파일시스템을 탐색하거나 Discord로 스팸을 발송할 수 있습니다. 피어 허용목록(Allowlist)과 읽기 전용 스킬 격리가 요구됩니다.
3. **규제 및 국내 호환성 한계**:
   - Stripe Link Agent Wallet 및 SPT는 현재 미국 및 캐나다 고객 위주로 프리뷰 제공되고 있어, 국내 원화(KRW) 결제 대행이나 국내 쇼핑몰 결제에는 기술적 적용 한계가 존재합니다. 따라서 당장 프로덕션 결제를 열기보다는 가상 임대 및 원장 추적 구조의 사전 대비에 무게를 두어야 합니다.

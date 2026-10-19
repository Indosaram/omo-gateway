# browser-agent 벤치마킹 및 도입 결정 보고서

## 판정

**채택(adopt now)**

단, 클라우드 브라우저(Dots Cloud PC, Instinct Persistent Cloud Computer, Pickle Kernel) 방식을 신규 구축하는 것은 배제하고, 오너의 단일 Mac 로컬 데몬 환경에 맞춰 **"오너의 실제 로그인 브라우저(Aside CDP/CLI) 직접 연동 + SSRF/Taint/비용 방어선 구축"** 형태로 즉시 채택한다. 오너 1인의 Mac 데스크톱에서 Discord/Cron/Lanes를 통해 무인 작업을 수행할 때, 쿠키와 주거용 IP가 없는 클라우드 브라우저는 CAPTCHA, 2FA, 통신사/포털/예약 사이트 차단(Resy/Amazon/네이버)으로 인해 즉각 실패하는 반면, 이미 오너 세션이 로그인된 로컬 Aside 브라우저는 계정 로그인 유지와 즉각적인 사람 개입(Handoff)이 가능하여 압도적인 효용을 제공하기 때문이다.

---

## 벤치마킹 원천

1. **Instinct (Spear Street Technology)**
   - **클라우드 브라우저 실패 패턴 [1차, 2차]**: 지속형 cloud computer에 브라우저를 두고 대리 로그인을 시도했으나, 주거용 IP와 실제 세션 쿠키의 부재로 인해 CAPTCHA 및 2FA 벽에 막혔다. Resy를 시간당 수백 회 과도하게 폴링하다가 계정이 영구 정지(Ban)당하는 사고가 발생했다 (`.omo/ulw-research/20261016-133102-autonomous-agents/wave1/instinct-arch.md`, TechCrunch/Carly 보도).
   - **교훈**: 무인 브라우저 에이전트는 클라우드 VM보다 로컬 오너 환경의 실제 쿠키·주거용 IP를 타는 것이 필수적이며, 과도한 폴링 방지를 위한 쿨다운/레이트 리밋이 강제되어야 한다.

2. **OpenAI Dots & ChatGPT Agent**
   - **Takeover / Handoff 및 Secure Sign-in [1차]**: 모델이 2FA나 CAPTCHA, 비밀번호 변경에 직면했을 때 Activity view의 "Take over"로 제어권을 즉시 사람에게 넘기고, 사람이 해결한 뒤 "Return control"을 수행한다 (`docs/research/autonomous-agent-platform-plan.md` 2.1절, OpenAI help 20001529). 또한 모델 컨텍스트 밖에서 자격증명을 입력받는 Secure sign-in을 적용했다.
   - **교훈**: 브라우저 조작 도중 CAPTCHA/OTP 봉착 시 에이전트가 우회하려고 무한 재시도하지 않고 사용자(Discord 알림)에게 HandOff하는 상태 전이가 필수적이다.

3. **Pickle (YC W25)**
   - **Kernel 브라우저 + Nitro Enclave 격리 [1차]**: 사용자별 Kernel 브라우저와 격리 워크스페이스를 분리하고, 오픈소스 `credential-enclave`를 통해 모델에는 handle/surrogate만 제공하며 egress 프록시에서 호스트/경로 허용 목록을 검증한다 (`docs/research/autonomous-agent-platform-plan.md` 2.5절, `.omo/ulw-research/20261016-133102-autonomous-agents/wave2/pickle-product.md`).
   - **결제 및 비가역 액션 사전 승인 카드 [1차]**: 브라우저 조작 중 결제 화면 진입 시 사전 합의된 금액을 1원이라도 초과하면 결제를 중단하고 재승인 카드를 띄운다.
   - **교훈**: 웹 페이지 내 폼 자동 제출, 구매 버튼 클릭 등의 액션은 Effect 분류 상 `Spend` 또는 `Irreversible`로 묶여야 한다.

4. **Meta Muse**
   - **Tainted Egress 및 Sentinel 검증 [1차]**: 브라우저 CDP 제어는 컨테이너 외부 broker 프로세스가 전담하며, 웹 페이지나 이메일 등 비인가 외부 입력을 읽은 에이전트는 즉시 Taint되어 외부 송신/결제 권한을 상실한다 (`docs/research/autonomous-agent-platform-plan.md` 2.2절, http://security.muse.ai/). Amazon 등 주요 플랫폼에서 "unauthorized AI agent"로 차단당한 사례가 확인되었다 [2차].

5. **Anthropic Claude in Chrome & Cowork**
   - **Permissions Mode 및 One-way doors 보호 [1차]**: 사이트 단위 허용 목록(Permissions Mode)과 사전 검토기를 두며, 금융 거래나 구매 등 되돌릴 수 없는 결정은 반드시 사람의 승인을 대기한다 (`.omo/ulw-research/20261016-133102-autonomous-agents/wave1/x-chatgpt_agent.md`).

---

## 현재 코드

1. **`BrowserTool`의 존재 및 제한된 구현 상태**
   - `src/tools/browser.rs:22-30`: `BrowserTool` 구조체가 선언되어 있으며 `cdp_port`(기본 9333, `src/tools/browser.rs:34`)를 대상으로 통신한다.
   - `src/tools/browser.rs:283-290`: 툴 이름은 `"browser"`이며, CDP를 통해 조작 가능한 액션으로 `"navigate"`, `"snapshot"`, `"eval"`, `"screenshot"` 4가지를 스키마에 정의해두었다.
   - `src/tools/browser.rs:467-470`: 그러나 실제 구현 코드에서는 `"eval"`과 `"screenshot"` 호출 시 `"browser action `{action}` requires a CDP WebSocket session and is not configured"`라는 에러를 즉시 반환하며 실패 처리된다. 즉, 현재 동작하는 것은 `"navigate"`와 단순 HTTP `/json/list` 기반의 `"snapshot"` 뿐이며, 클릭, 타이핑, DOM 쿼리, 스크린샷 등 실제 브라우저 자동화 액션은 완전히 미구현 상태이다.

2. **엄격한 SSRF 방어 로직 내장**
   - `src/tools/browser.rs:135-217`: IPv4/IPv6 사설 대역, 루프백, 링크로컬, NAT64 등을 판정하는 `is_internal_ip` 함수군이 철저하게 구현되어 있다.
   - `src/tools/browser.rs:222-261`: `ensure_public_destination`을 통해 DNS resolve 결과가 내부망 IP인 경우 내비게이션을 즉시 차단한다.
   - `src/tools/browser.rs:47-79`: 탭 오픈 후 리다이렉트나 DNS rebinding으로 내부망에 도달하는 것을 막기 위해 `verify_tab_settled`로 최종 URL을 재검증한다.
   - `src/tools/browser.rs:81-90`: 검증 실패 시 `close_tab`으로 탭을 즉시 강제 종료한다.
   - `src/tools/browser.rs:9-10`: `OMON_BROWSER_ALLOW_INTERNAL_HOSTS` 환경변수가 활성화되지 않는 한 내부망 접근은 기본 차단된다.

3. **도구 등록 및 세션 격리**
   - `src/main.rs:1087` 및 `src/dashboard_runtime.rs:131`: `omon_gateway::BrowserTool::default()`가 서버 시작 시 기본 툴 레지스트리에 등록된다.
   - `src/tools/browser.rs:26-27` 및 `src/tools/browser.rs:328-330`: 세션별 스토리지 키(`SessionKey::storage_key`)로 `owned_tabs`를 관리하여 타 세션이나 오너가 띄운 탭을 임의로 스냅샷하거나 닫지 않도록 세션 격리가 되어 있다.

4. **Lanes 및 Aside 호출 현황**
   - `src/lanes/runner.rs:777-805`: LaneRunner는 `AgentBackend`(`OmoBackend`)를 통해 서브에이전트를 구동하며, 별도의 브라우저 전용 실행기를 직접 부르지 않는다.
   - `src/sessions/external.rs:94-135`: `ExternalDriver`를 통해 외부 프로세스 세션을 입양(adopt)하여 실행을 감시할 수 있다.
   - 소스코드 전수 검색 결과(`src/` 전역), 현재 Aside 브라우저 CLI(`~/.local/bin/aside`)를 직접 서브프로세스로 기동하거나 IPC를 주고받는 러스트 코드는 0건이다. Aside 조작은 런타임 외적인 스크립트나 로컬 환경 세팅(Aside CDP 9333 포트 오픈 등)에 의존하고 있다.

5. **이전 계획서(`autonomous-agent-platform-plan.md`)와의 불일치 및 오류 검증**
   - 이전 계획서 4절(게이트웨이 현재 역량 맵)에서는 브라우저 툴의 실태를 구체적으로 다루지 않고 단순히 "omo app-server agent backend", "terminal", "FileTool" 위주로만 기록하였다.
   - 실제 소스코드 `src/tools/browser.rs`를 검증한 결과, 계획서의 가정과 달리 이미 `BrowserTool`이 기본 등록되어 있으나, 핵심인 WebSocket 기반 클릭/스크립트/스크린샷이 미완성(stub error 반환) 상태였음이 밝혀졌다.

---

## 구현 단계

본 구현은 **클라우드 컨테이너 샌드박스를 새로 띄우지 않고, 오너의 Mac 상에서 동작 중인 Aside 브라우저 인스턴스(CDP 9333 및 aside CLI)를 안전하고 완전하게 제어**할 수 있도록 점진적으로 확장한다.

### 1단계: Aside CDP WebSocket 연결 및 기본 DOM/상호작용(Action) 완비
- **수정/신규 파일**: `src/tools/browser.rs`
- **구현 내용**:
  - `tokio-tungstenite` 기반으로 CDP WebSocket 세션(`Page`, `Runtime`, `Input`, `DOM` 도메인)을 연결.
  - 미구현이던 `"eval"`, `"screenshot"` 액션을 활성화하고, 신규 액션 `"click"`, `"type"`, `"get_text"`를 추가.
  - Aside 전용 프로필 지원: 기본 포트 9333 외에 `OMON_ASIDE_CDP_URL` 또는 `OMON_ASIDE_PROFILE` 연동.
- **환경변수 킬스위치**: `OMON_BROWSER_ACTIONS_ENABLED=0` (기본값 0, 켜질 때만 write/input CDP 액션 허용)
- **기계 검증 시나리오**:
  - Aside 브라우저 구동 후 CLI 테스트 실행:
    ```bash
    OMON_BROWSER_ACTIONS_ENABLED=1 omo-gateway tool test browser '{"action":"eval","script":"document.title"}'
    ```
  - 기대 결과: JSON 출력에 `"result": { "type": "string", "value": ... }` 및 exit code 0.

### 2단계: Browser Action Effect 분류 및 Handoff/승인 게이트 연동
- **수정/신규 파일**: `src/policy/effect.rs`, `src/policy/decision.rs`, `src/tools/browser.rs`
- **구현 내용**:
  - Browser Action을 Effect로 분류:
    - `"snapshot"`, `"get_text"`, `"screenshot"` -> `Effect::Read` (자동 허용)
    - `"navigate"` -> `Effect::Read` (SSRF 통과 시 자동 허용)
    - `"type"`, `"click"` -> `Effect::Write` (도메인 정책에 따라 Ask 또는 Allow)
    - 결제 버튼, 주문 제출, 송금, 비밀번호 변경 페이지 진입 시 -> `Effect::Spend` 또는 `Effect::HandOff` (Discord 승인 카드 및 사용자 직접 조작 대기)
  - CAPTCHA 및 Cloudflare 대기 화면 감지 시, 브라우저 조작 루프를 중단하고 `Decision::HandOff`를 발생시켜 Discord로 알림 발송:
    - 알림 내용: `"[HandOff] Aside 브라우저에서 CAPTCHA 인증이 필요합니다. 해결 후 '계속'을 입력하세요."`
- **DB 마이그레이션**: `migrations/0045_approval_system.sql` (P0 공통 원장 테이블 활용)
- **환경변수 킬스위치**: `OMON_BROWSER_APPROVAL_REQUIRED=1` (기본값 1, 안전 우선)
- **기계 검증 시나리오**:
  - 결제 관련 DOM 요소 클릭 시뮬레이션 시 Ask 판정 확인:
    ```bash
    omo-gateway policy eval --effect write --action '{"tool":"browser","action":"click","selector":"#buy-now"}'
    ```
  - 기대 결과: stdout에 `{"decision":"Ask", "requires_approval": true}` 출력.

### 3단계: 도메인 격리, 과도한 폴링 방지(Cool-down) 및 Taint 마킹
- **수정/신규 파일**: `src/tools/browser.rs`, `src/lanes/runner.rs`, `src/security/taint.rs`
- **구현 내용**:
  - **도메인 쿨다운 (Resy 차단 방지)**: 동일 도메인(예: `resy.com`, `amazon.com`)에 대해 1분당 최대 10회 요청 제한 및 연속 탐색 시 지수 백오프 강제.
  - **Taint 추적**: 브라우저로 외부 웹페이지를 방문하여 DOM 스냅샷을 읽은 Lane은 `TaintedBy::Web(url)`으로 마킹. 이후 해당 Lane이 외부 발송(`SendExternal`) 또는 파일 변경을 시도할 때 강제 승인(Ask)을 요구하여 Prompt Injection을 통한 데이터 유출 차단.
- **환경변수 킬스위치**: `OMON_BROWSER_TAINT_ENABLED=1` (기본값 1)
- **기계 검증 시나리오**:
  - SQLite 검증 쿼리:
    ```sql
    SELECT taint_status FROM lanes WHERE key = 'test-browser-lane';
    ```
  - 기대 결과: 브라우저 읽기 수행 후 `taint_status` 컬럼에 `web_tainted`가 영속화되어 있을 것.

---

## 위험·의존성

1. **상호 의존 테마**:
   - `approval-effects` & `approval-grants`: 브라우저의 `"click"`(결제/제출) 및 `"type"`(자격증명 입력) 액션이 임의로 수행되지 않도록 차단하는 핵심 선행 조건.
   - `taint`: 웹 서핑 중 악의적인 프롬프트 인젝션(간접 인젝션)에 노출된 에이전트가 오너의 개인 파일이나 Discord로 데이터를 유출하지 못하게 막는 방어선.
   - `outbox-destination`: 브라우저 조작 결과로 발생한 외부 송신 액션의 목적지 검증.
   - `usage-ledger`: 브라우저 비전/스크린샷 분석 시 대형 멀티모달 모델 호출로 인한 토큰/비용 폭주 감시.

2. **운영 및 보안 위험 요인**:
   - **오너 실제 세션 오염/파괴 위험**: Aside 브라우저에는 오너의 Google, Naver, 금융/쇼핑 계정이 실제 로그인되어 있다. 에이전트가 잘못된 폼 제출이나 '로그아웃' 버튼, '계정 탈퇴/삭제'를 클릭할 경우 비가역적인 피해 발생. -> *완화책: 파괴적 액션 셀렉터 사전 차단 및 클릭/제출 전 `Effect::Irreversible` 승인 필수화.*
   - **사이트 차단 및 CAPTCHA 폭탄**: 단시간에 다수의 요청을 Aside로 보낼 경우 사이트 차단(Cloudflare 1020, IP 차단) 발생 가능. -> *완화책: 최소 3~5초의 인간적 인터벌과 도메인별 레이트 리밋 강제.*
   - **CDP 연결 유실 및 탭 누수**: Aside 재시작 시 포트 9333 연결이 끊어지거나 미사용 탭이 백그라운드에 수십 개 방치될 위험. -> *완화책: 데몬의 `owned_tabs` 주기적 회수(`close_tab`) 및 연결 실패 시 자동 백오프.*

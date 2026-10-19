# 벤치마킹 분석 보고서: mcp-connectors (G8b)

## 판정

**보류 (Later)**

omon-gateway는 단일 소유자(1-owner), 로컬 Mac 데스크톱 데몬 환경이며, 현재 핵심 실행 백엔드로 자체 OMO app-server(`ws://127.0.0.1:19742`)를 구동하고 있습니다. 게이트웨이 내부에 완전한 범용 MCP 클라이언트와 4,000개 이상의 SaaS 커넥터 생태계를 직접 구축하는 것은 호스트 데몬 복잡도만 가중시키고 실질적인 소유자 가치(Discord/iMessage/Kakao 기반 비동기 알림, 장시간 작업 lane, 신뢰성 높은 cron 자가 치유)를 높이지 못합니다. 현재 게이트웨이에 이미 Stdio/SSE 기반 MCP JSON-RPC 호출 파이프라인(`src/tools/mcp.rs`)이 단위 테스트 통과 수준으로 완성되어 있으나 엔드포인트 목록이 비어 있는 상태(`McpTool::default()`)이므로, 사전 필수 안전 기반(P0 effect 분류, credential-vault, egress proxy)이 확립된 후 설정 파일/DB 기반 동적 로더와 필수 로컬 MCP(파일/브라우저/카카오 등)만 선택적으로 연동하는 단계적 보류 접근이 가장 타당합니다.

---

## 벤치마킹 원천

1. **OpenAI Dots**
   - **4,000+ 앱 커넥터 생태계**: 마케팅 및 시드 주장으로 언급되었으나 1차 공식 문서에서는 확인되지 않음 [미검증].
   - **Proactive Research & 읽기 전용 도구**: 사용자가 부재할 때 도는 사전 연구는 엄격히 읽기 전용(read-only) 도구로 격리되며, 메시지 발송이나 외부 앱 변동, 브라우저 조작이 원천 차단됨 [1차].
   - **권한 체계 (Allow / Require approval / Block)**: 작업 종류와 대상에 따라 3단계 규칙을 두어 승인 피로(approval fatigue)를 줄이고 비가역 동작에 사람 개입 보장 [1차].
   - **Secure Sign-in 자격증명 격리**: 모델을 중단한 상태에서 브라우저에 직접 자격증명을 입력하여 모델 컨텍스트에 토큰/패스워드가 남지 않도록 차단 [1차].

2. **xAI Grok Bot**
   - **커스텀 MCP 및 Cursor 인프라 공유**: Cursor 생태계와 동일한 MCP 서버, 플러그인, 스킬을 지원하며 Remote HTTPS MCP와 Command MCP(로컬 프로세스)를 모두 지원 [1차].
   - **Team Bot 자격증명/비밀 격리**: 봇당 최대 25개 비밀을 암호화하여 저장하며, 모델에게 비밀 원문을 노출하지 않고 `[REDACTED]` 처리 [1차].
   - **단일 머신 공유로 인한 보안 경계 실패 및 BI 사고**: 모든 봇이 동일한 클라우드 머신/세션을 공유하여 한 봇의 은행 조회가 회사 공개 Slack 채널에 보고서로 오발송되는 금융 정보 유출 사고 발생 [2차]. 이후 데이터 전송 시 영구 채널 ID 바인딩 및 교차 채널 전송 전 확인 정책 도입 [1차].

3. **OpenClaw & Hermes Agent**
   - **Preflight 유효성 검사**: cron 실행 전에 모델 호출 없이 자격증명, MCP 서버 상태, 전달 대상의 정상 여부를 확인하고 실패 시 1회만 경고하여 토큰 낭비 및 반복 에러 방지 [1차].
   - **No-agent 및 Stdio 래퍼 격리**: LLM 호출 없이 순수 스크립트/도구 출력만 전달하는 경량 실행 및 불변 식별자 바인딩 [1차].

---

## 현재 코드

1. **`src/main.rs:1082` 및 `src/dashboard_runtime.rs:126`**:
   `tools.register(McpTool::default());` 형태로 기본 생성자를 등록하고 있어, 실제 가동 중인 MCP 클라이언트 목록(`clients: Vec<McpClientTool>`)은 비어 있습니다.
2. **`src/tools/mcp.rs:18-28`**:
   `McpTransport` 열거형에 `Stdio { program, args, cwd }`와 `Sse { url, bearer_token }` 두 가지 전송 계층이 정의되어 있습니다.
3. **`src/tools/mcp.rs:30-43`**:
   `McpClientTool` 구조체에 `name`, `remote_tool`, `description`, `input_schema`, `transport`, `timeout`, `stdio_lock`, `requires_approval` 필드가 구현되어 있습니다.
4. **`src/tools/mcp.rs:106-157`**:
   `request_stdio` 메서드에서 프로세스 그룹 분리(`command.process_group(0)`), 파이프 버퍼 데드락 방지를 위한 백그라운드 stderr 배출(`tokio::io::copy(&mut stderr, &mut sink)`), 요청 종료 시 프로세스 트리 SIGKILL(`libc::killpg`) 및 타임아웃 처리가 견고하게 구현되어 있습니다.
5. **`src/tools/mcp.rs:213-287`**:
   `McpTool` 디스패처가 등록되어 있어 `"mcp"` 도구 호출 시 JSON 인자(`{"tool": "...", "arguments": {}}`)를 받아 해당 이름의 `McpClientTool`을 찾아 실행합니다.
6. **`src/tools/mcp.rs:322-339`**:
   프레임 상한(`MAX_FRAME_BYTES = 1MB`) 및 응답 상한(`MAX_RESPONSE_BYTES = 8MB`)이 상수로 강제되어 있어 악의적이거나 비정상적인 MCP 서버의 메모리 고갈 공격을 방어합니다.
7. **`src/tools/mod.rs:279-373`**:
   통합 테스트(`smart_approvals_persist_across_reloads`)에서 Axum 기반 모의 SSE MCP 서버를 띄워 MCP 호출 및 Always 승인 격리가 검증되고 있습니다.
8. **`src/discord/commands.rs:24, 76, 964-968`**:
   Discord 슬래시 커맨드 `/tools`에서 `ctx.data().mcp_endpoints`를 조회하나, `PoiseData::new` 시점에 빈 벡터(`Vec::new()`)로 초기화된 이후 아무런 엔드포인트도 추가되지 않아 항상 `(none)`으로 표시됩니다.
9. **기존 계획서 오류 및 정정**:
   - `docs/research/autonomous-agent-platform-plan.md` 4절에서는 "MCP 클라이언트 목록 비어 있음(`src/main.rs:1082`)"이라고 지적했는데, 이는 현재 코드에서도 그대로 사실입니다.
   - 단, 계획서의 갭 분석(G8)에서는 "커넥터, 자격증명, MCP 없음"으로 단순 묶여 기술되어 있어 MCP 전송 프로토콜 엔진 자체가 없는 것처럼 오해될 수 있으나, 실제 소스 코드 상으로는 `Stdio`와 `SSE` 클라이언트 전송 파이프라인(`src/tools/mcp.rs`)과 승인 바인딩 해시 로직(`requires_approval`)이 이미 완전히 구현되어 있습니다. 누락된 것은 **MCP 서버 설정 로더(JSON/DB)**, **엔드포인트 등록기**, 그리고 **서버 툴 동적 탐색(list_tools)** 계층입니다.

---

## 구현 단계

### 1단계: 정적 설정 기반 MCP 서버 등록 로더 구현
- **변경/생성 파일**:
  - `src/tools/mcp_config.rs` (신규 생성: `~/.omon/mcp.json` 또는 `config/mcp.json` 파서)
  - `src/tools/mod.rs` (모듈 노출)
  - `src/main.rs:1082`, `src/dashboard_runtime.rs:126` (설정 기반 `McpTool` 인스턴스화 연결)
  - `src/discord/commands.rs:76` (`mcp_endpoints`에 활성화된 서버 주소 목록 주입)
- **DB 마이그레이션**: 없음 (파일 기반 zero-config 시작)
- **환경변수 킬스위치**: `OMON_MCP_ENABLED=0` (기본값 0/비활성화)
- **기계 검증 시나리오**:
  - `~/.omon/mcp.json`에 Stdio 에코 도구 설정 추가 후 게이트웨이 기동
  - CLI 또는 curl로 확인:
    ```bash
    OMON_MCP_ENABLED=1 cargo test --lib tools::mcp_config::tests -- --nocapture
    ```
  - 로그 검증: `rg "registered MCP server name=" ~/.omon/logs/gateway.log`

### 2단계: DB 영속화 및 `mcp_servers` 관리 테이블 도입
- **변경/생성 파일**:
  - `migrations/0048_mcp_servers.sql` (신규 마이그레이션)
  - `src/tools/mcp_store.rs` (DB 기반 동적 로더 및 상태 관리)
  - `src/tools/mcp.rs` (`list_tools` JSON-RPC 호출을 통한 툴 자동 발견 추가)
- **DB 마이그레이션**:
  - 테이블: `mcp_servers(id TEXT PRIMARY KEY, name TEXT UNIQUE, transport TEXT NOT NULL, config_json TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL)`
  - 테이블: `mcp_tools(server_id TEXT, tool_name TEXT, description TEXT, schema_json TEXT, PRIMARY KEY(server_id, tool_name))`
- **환경변수 킬스위치**: `OMON_MCP_DB_SYNC_ENABLED=0` (기본값 0)
- **기계 검증 시나리오**:
  - SQLite 쿼리로 등록 확인:
    ```bash
    sqlite3 ~/.omon/state.db "SELECT id, name, transport, enabled FROM mcp_servers;"
    ```
  - 기대 결과: 종료 코드 0, 등록된 행 출력.

### 3단계: P0 정책 엔진(Effect / Taint / Approval) 및 Egress Vault 연동
- **변경/생성 파일**:
  - `src/tools/mcp.rs:252-270` (`requires_approval`과 effect 분류 연결)
  - `src/policy/effect.rs` (MCP 도구별 Effect 매핑: Read vs SendExternal vs Irreversible)
  - `src/policy/taint.rs` (외부 MCP 도구 결과 수신 시 context taint 마킹)
- **DB 마이그레이션**:
  - `approval_requests`, `action_audit` 테이블 (P0 공유 테이블 활용)
- **환경변수 킬스위치**: `OMON_MCP_STRICT_POLICY_ENABLED=1` (기본값 1)
- **기계 검증 시나리오**:
  - `SendExternal` 속성을 가진 MCP 도구 호출 시 승인 없이 실행되지 않고 Pending 상태로 대기:
    ```bash
    sqlite3 ~/.omon/state.db "SELECT status, effect FROM approval_requests WHERE action_json LIKE '%mcp%';"
    ```
  - 기대 결과: `Pending|SendExternal`

---

## 위험·의존성

1. **상호 의존 테마**:
   - `credentials`: MCP 서버가 외부 API 인증(Bearer token, API key)을 필요로 할 때 평문 환경변수 주입 대신 Keychain 및 Egress proxy 기반의 토큰 핸들(`cred://...`) 치환이 선행되어야 함.
   - `approval-effects`: Stdio 명령 실행(Command MCP)이나 외부 데이터 변경(SaaS MCP) 시 임의 명령 실행 및 비가역 변경을 막기 위해 Action/Effect 분리 필수.
   - `taint`: 제3자 MCP 서버가 반환한 비신뢰 데이터가 주입되었을 때 프롬프트 인젝션에 의한 2차 발송/외부 유출을 방지하기 위한 Taint 추적 필수.
   - `outbox-destination`: Grok Bot의 BI 사고와 같이 MCP 도구가 읽어온 민감 정보가 엉뚱한 Discord/Slack 채널에 유출되지 않도록 불변 목적지 ID 바인딩 필수.
   - `sandbox`: 임의의 Stdio MCP 서버 프로세스가 호스트 Mac 환경(파일, 키체인, 마이크 등)에 무제한 접근하는 것을 방지하기 위한 OS 레벨 Sandbox 격리 연계.

2. **주요 위험 요소**:
   - **데드락 및 프로세스 누수 위험**: Stdio MCP 서버가 종료되지 않거나 stderr 파이프가 차단될 경우 게이트웨이 스레드 풀 또는 자식 프로세스가 고아(orphan)로 남아 자원 고갈 유발 (현재 `libc::killpg`로 방어 중이나 완벽한 트리 정리는 OS 격리 필요).
   - **프롬프트 인젝션 통로**: MCP Tool의 `description` 및 리턴 페이로드가 공격자에 의해 오염될 경우 LLM의 지시 체계가 무력화될 수 있음.
   - **토큰 소모 급증**: MCP 서버에서 수십 개의 툴 스키마를 모델 컨텍스트에 상시 주입하면 턴당 기본 토큰 소비량이 크게 증가함 (Grok/Dots의 선별 주입 패턴 필요).

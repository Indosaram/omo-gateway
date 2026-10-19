# omon-gateway 자율 에이전트 벤치마크: memory-dream

## 판정

**보류 (Later)**

omon-gateway는 단일 소유자(Single Owner)가 macOS 로컬 머신에서 운영하며, 주 커뮤니케이션 채널은 Discord, 핵심 실행 엔진은 cron 자가수리·inbox 수집·검증 완료 기반 작업 레인(lanes)입니다. 현재 메모리 계층은 전역 사실(facts) 기반이 아닌 단순 세션 격리 문자열 테이블(`memories`, `session_key` 바인딩)에 머물러 있어, 정제할 기반 데이터(`episodes`, `facts`, 구조화된 claims) 자체가 부재한 상태입니다. 따라서 선행 과제인 `memory-facts`(G9a: 오너 스코프 bi-temporal 사실 저장소, provenance, 검색) 및 `approval-grants`/`approval-effects`(비가역 승인 프레임워크)가 안착되기 전까지 야간 dream/consolidation 작업 단독 도입은 실익 없이 야간 토큰 비용만 낭비하므로, P3 단계에서 `memory-facts` 직후 순차 도입하는 것으로 보류합니다.

---

## 벤치마킹 원천

1. **Meta Muse — claims 저장소와 야간 Dream 및 Retraction 파이프라인 [1차]**
   - **출처**: `mouse.dev/blog/muse-runtime-export` [1차] (연구자 런타임 덤프 및 복원), `about.fb.com/news/2026/09/introducing-muse/` [1차], `meta.com/help` [1차].
   - **핵심 패턴**:
     - 메모리는 마크다운 파일(`MEMORY.md`, `memory/YYYY-MM-DD.md`, `memory/bank/`)과 관계형 DB(`memory.claims`)로 이원화 관리.
     - `claims` 테이블에 `evidence`, `confidence`, `status`, `supersedes_claim_id`를 보관하며, 매시간 원본 대화 메시지와 교차 검증(hourly verifier) 수행.
     - 매일 밤 야간 "dream" 작업이 최근 대화를 검토하여 `~/dreams/<date>.md`를 생성하고 요약. 단, dream 산출물은 프롬프트에 무조건 자동 주입하지 않고(`prompt_hoisted: false`), 검토된 Standing guidance(`ALIGNMENT_SYNTHESIS.md`)로 승격될 때만 주입.
     - "forget" 요청 시 모델 가중치를 수정하는 것이 아니라, claim ID 철회(retraction), 연관 마크다운 청크 삭제, 임베딩/검색 인덱스 재빌드를 수행하는 결정론적 파이프라인 가동.

2. **xAI Grok Bot — 4계층 메모리 구조와 사전 압축(Compaction) [1차]**
   - **출처**: Lee Robinson의 Stanford 강연 녹취록 [1차] (`peterp.me/transcripts/how-grok-bot-works/`), xAI 공식 가이드 [1차] (`x.ai/bot/guides/grok-bot-101`).
   - **핵심 패턴**:
     - 메모리를 4개 계층으로 분리: ① 핵심 사실(Core Facts), ② 일자별 로그(Daily Logs), ③ 30~60분 임시 스크래치패드(Scratchpad), ④ grep 가능한 파일 시스템(Files).
     - 컨텍스트 캐시가 유지되는 시간 내에 선제적으로 압축(compaction)하여 긴 맥락을 유지하면서도 토큰 소모를 최소화.
     - 봇 간 컨텍스트 공유 및 템플릿 내보내기 시 비밀번호와 개인 메모리는 자동 배제(redaction).

3. **Hark Pro / Hark Handoff — 초상/설정 프로필 편집(Portrait/Preferences) 및 명시적 망각 [1차]**
   - **출처**: `hark.com/articles/introducing-hark-pro` [1차], `hark.com/terms` [1차].
   - **핵심 패턴**:
     - 사용자의 명시적 요청 시 완전 망각(forget on request) 처리.
     - 상시 능동적 페이싱(proactive pacing)을 위한 장기 메모리를 유지하되, 사용자가 원하지 않는 proactive 메시지나 왜곡된 메모리는 패널 UI에서 직접 편집/삭제 가능.

---

## 현재 코드

1. **`src/memory/store.rs:26-44`**:
   - `MemoryStore` 구조체가 정의되어 있으며 `remember()` 메서드는 `session_key`와 `content`, `metadata`를 받아 DB에 삽입합니다.
   - 단, `src/memory/store.rs:36-58`을 보면 `crate::storage::write_approval_enabled()`가 켜져 있으면 `crate::storage::stage_pending_write(&self.pool, "memory", ...)`로 승인 대기 단계로 보낼 뿐, 야간 주기 배치나 백그라운드 consolidation 로직은 전무합니다.

2. **`src/memory/store.rs:90-145`**:
   - `search()` 메서드가 구현되어 있으나 의미적 임베딩이나 벡터 검색이 아니라, 단어 분절 기반의 토큰 빈도 카운트 `term_counts()`와 코사인 유사도 + 단순 어구 포함 여부(`contains(&query_lower)`)로 점수를 매기는 원시적 lexical 검색에 불과합니다(`src/memory/store.rs:117-133`).
   - 검색 대상 또한 `session_key`로 엄격하게 바인딩되어 있어(`src/memory/store.rs:99-105`), 디스코드 DM, 특정 채널, 크론 작업, 레인 간에 메모리가 공유되지 않습니다.

3. **`src/multiplexer/actor.rs:1405-1445` (`fresh_context`)**:
   - 선행 연구 계획서(`docs/research/autonomous-agent-platform-plan.md:368`)에서는 `src/multiplexer/actor.rs:1344`에서 메모리를 주입할 수 있다고 언급했으나, 현재 코드 기준 실제 함수 위치는 `src/multiplexer/actor.rs:1405`입니다.
   - 현재 `fresh_context()` 내부에서는 `bot_profiles` 테이블로부터 `name, model, system_prompt, enabled_toolsets`만 읽어올 뿐(`src/multiplexer/actor.rs:1414-1428`), 과거 사실이나 `MemoryStore` 내용을 세션 프롬프트에 자동 주입하는 코드는 전혀 존재하지 않습니다.

4. **`src/lanes/kibitzer.rs:375-408` (`state_document`)**:
   - 현재 코드베이스에서 `MemoryStore`를 실제로 조회하는 유일한 상위 로직입니다.
   - `src/lanes/kibitzer.rs:385-387`에서 `MemoryStore::new(self.pool.clone()).search(&self.lead, "owner preferences", 10)`을 호출하여 키비처 프롬프트의 "Owner preferences:\n" 섹션에 문자열로 덧붙입니다. 하지만 이는 리드 세션 키(`self.lead`)로 한정된 단순 텍스트 매칭 검색일 뿐이며, consolidation이나 dream과는 무관합니다.

5. **`src/cron/scheduler.rs:324-375` (`CronJobSpec`, `CronJob`)**:
   - 게이트웨이 내의 주기적 백그라운드 작업은 `CronJob`과 `scheduler.rs`가 총괄하고 있습니다.
   - 현재 등록된 크론 작업은 인박스 다이제스트, 헬스체크, 자가치유(`src/cron/remediate.rs:1-50`) 등에 집중되어 있으며, 메모리 정리, 일일 로그 롤업, diff 제안 등을 담당하는 잡 종류(Job Kind)나 배치 스크립트는 존재하지 않습니다.

6. **`migrations/0001_initial.sql:47-59`**:
   - 메모리 관련 테이블은 초기 마이그레이션에 생성된 `memories(id, session_key, content, metadata_json, created_at, updated_at)` 테이블이 유일합니다.
   - 사실(fact) 관계, 참/거짓 상태(`active`/`superseded`/`retracted`), 출처 에피소드 ID(`provenance`), 유효 기간(`valid_at`, `invalid_at`)을 저장할 컬럼이나 보조 테이블이 없습니다.

---

## 구현 단계

이 테마는 선행 과제(`memory-facts`)가 구현되어 `facts`와 `episodes` 테이블이 존재한다는 전제하에 도입됩니다.

### 단계 1: 야간 Dream 배치 잡 및 Diff 제안 테이블 스키마 구축
- **신규/수정 파일**:
  - `migrations/0048_memory_dream.sql` (신규)
  - `src/memory/dream.rs` (신규: 대화 추출, LLM 기반 사실 추출 및 diff 생성)
  - `src/memory/mod.rs` (수정: `dream` 모듈 노출)
- **DB 마이그레이션 (`0048_memory_dream.sql`)**:
  ```sql
  CREATE TABLE IF NOT EXISTS memory_dream_proposals (
      id TEXT PRIMARY KEY NOT NULL,
      status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'approved', 'rejected', 'applied')),
      action_kind TEXT NOT NULL CHECK (action_kind IN ('insert_fact', 'update_fact', 'retract_fact')),
      target_fact_id TEXT,
      proposed_fact_json TEXT NOT NULL,
      diff_rationale TEXT NOT NULL,
      source_episode_ids_json TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      resolved_at TEXT,
      FOREIGN KEY (target_fact_id) REFERENCES facts(id) ON DELETE SET NULL
  );
  CREATE INDEX IF NOT EXISTS idx_memory_dream_status ON memory_dream_proposals(status, created_at);
  ```
- **환경 변수 Kill Switch**:
  - `OMON_MEMORY_DREAM_ENABLED=false` (기본값: off)
- **기계 검증 시나리오**:
  - CLI 테스트 실행:
    `cargo test test_dream_proposal_schema -- --nocapture` (또는 sqlite3 CLI 검증)
    ```bash
    sqlite3 ~/.omon/gateway.db "INSERT INTO memory_dream_proposals (id, action_kind, proposed_fact_json, diff_rationale, source_episode_ids_json) VALUES ('test-1', 'insert_fact', '{\"text\":\"오너는 TypeScript보다 Rust를 선호함\"}', '최근 대화 관찰', '[\"ep-1\"]'); SELECT status, action_kind FROM memory_dream_proposals WHERE id='test-1';"
    ```
    출력 결과: `proposed|insert_fact` 확인 후 삭제.

### 단계 2: 크론 스케줄러 연동 및 KST 심야 무소음 실행기 등록
- **신규/수정 파일**:
  - `src/cron/scheduler.rs` (수정: dream job 트리거 등록)
  - `src/cron/executor.rs` (수정: `memory_dream` 실행 핸들러 매핑)
  - `src/memory/dream.rs` (수정: 비용 효율적 모델 사용 및 KST 0~8시 알림 억제 로직)
- **핵심 동작 규칙**:
  - 매일 새벽 04:00 KST에 크론 잡이 기동됩니다.
  - 당일 발생한 `messages`와 `inbox_items`를 스캔하여 기존 `facts`와의 충돌/신규 사실을 분석합니다.
  - 고비용 추론 대신 저렴한 계층화 모델(`inferhub/cb/deepseek-v4.1-flash` 등)을 호출하여 토큰 낭비를 통제합니다.
  - **야간 알림 금지 규칙(KST 0~8시)**: 생성된 제안은 Discord로 즉시 DM을 보내지 않고, `memory_dream_proposals` 테이블에 저장만 해둔 뒤 오전 08:30 브리핑 크론 잡에서 일괄 보고합니다.
- **기계 검증 시나리오**:
  - 게이트웨이 로그 검증:
    ```bash
    grep "memory_dream consolidation completed proposals=" /Users/indo/.omon/logs/gateway.log
    ```
    로그 라인에 `proposals=N suppressed_notify=true (quiet_hours)` 확인.

### 단계 3: 소유자 승인 연동 (Diff 리뷰 및 Forget/Retraction 처리)
- **신규/수정 파일**:
  - `src/inbox/proposals.rs` 또는 `src/discord/approval.rs` (수정: 기억 제안 승인/거절 UI 처리)
  - `src/memory/store.rs` (수정: 승인된 diff를 `facts` 테이블에 반영하고 관련 벡터/FTS 인덱스 갱신)
- **핵심 동작 규칙**:
  - 소유자에게 "다음 사실을 기억에 추가/수정할까요? [승인] [거절]" 버튼이 Discord 카드로 전달됩니다.
  - 승인 시 `facts`에 반영되고, 소유자가 대화창에서 "그 사실 잊어줘(forget)"라고 명령하거나 거절하면 `retract_fact`가 실행되어 FTS 인덱스 및 active 상태에서 즉시 격리 삭제됩니다.
- **기계 검증 시나리오**:
  - 승인 실행 SQL 및 로그 검증:
    ```bash
    sqlite3 ~/.omon/gateway.db "SELECT count(*) FROM facts WHERE state = 'active' AND text LIKE '%Rust를 선호%';"
    ```
    카운트가 1로 전환되고 `audit_log`에 `fact_applied` 이벤트가 기록되었는지 확인.

---

## 위험·의존성

1. **상행 의존성 (Prerequisites)**:
   - `memory-facts`: **필수 선행**. 원천 에피소드(`episodes`) 및 시간/상태 기반 사실(`facts`) 테이블 구조가 먼저 구현되어 있어야 dream consolidation이 diff를 계산할 수 있습니다.
   - `approval-effects` / `approval-grants`: **필수 선행**. 메모리 수정 및 망각은 비가역적(Irreversible/Write) 변경에 해당하므로, 신규 승인 프레임워크를 통해 검증된 action hash 기반으로 승인되어야 합니다.
   - `notify-envelope`: **필수 선행**. 야간(0~8시 KST)에 소유자를 깨우지 않고 조용히 큐에 쌓았다가 아침 브리핑에 합쳐서 발송하기 위해 알림 억제 및 엔벨로프 격리가 필요합니다.

2. **하행 연계 테마 (Downstream consumers)**:
   - `kibitzer-action`: 정제된 오너 프로필/선호도 메모리를 바탕으로 키비처가 더 정확한 레인 가동 판단을 내릴 수 있습니다.
   - `instinct-focus`: 오너의 최신 관심사와 집중 영역을 dream 과정을 통해 갱신합니다.

3. **고유 위험 요인 (Risks)**:
   - **야간 모델 토큰 비용 낭비 (Cost Drift)**: 오너가 대화를 거의 하지 않은 날에도 대규모 컨텍스트를 스캔하여 LLM을 호출하면 일일 수십만 토큰이 소모될 수 있습니다(Dots 307k 토큰 사고). 대화 변경량이 임계치(예: 신규 메시지 5건 미만) 이하일 경우 모델 호출 없이 `skip`하는 결정론적 게이트가 필수적입니다.
   - **환각 및 잘못된 사실 영구 고착 (Memory Poisoning)**: AI가 대화의 농담이나 비유를 사실로 오인하여 저장할 경우 에이전트의 페르소나와 판단이 왜곡됩니다. 따라서 Muse와 Hark의 교훈에 따라 **자동 승격(Auto-hoisting)을 엄격히 금지**하고, 오너의 명시적 승인 전까지는 `proposed` 상태로만 머물러야 합니다.
   - **프롬프트 인젝션 전이 (Taint Contamination)**: 외부 이메일이나 웹 본문(`inbox`)에서 흘러들어온 untrusted 문자열이 야간 dream 요약 과정을 거치면서 신뢰할 수 있는 사실(core facts)로 승격될 위험이 있습니다. `taint` 추적이 결합되어 신뢰 출처(오너의 직접 발언)에서 유래된 에피소드만 사실 제안 대상에 포함시켜야 합니다.

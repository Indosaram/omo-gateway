# omon-gateway 자율 에이전트 벤치마크: memory-facts (G9a)

- 작성일: 2026-10-17
- 대상 시스템: `omon-gateway` (macOS 로컬 기반 단일 오너 자율 데몬)
- 테마: `G9a memory: owner-scoped bi-temporal facts, FTS/vector, editable USER.md/MEMORY.md, injection into turns`
- 결과 문서: `.omo/ulw-research/20261017-autonomy-benchmark/lanes/memory-facts.md`

---

## ## 판정

**보류 (Later)**

omon-gateway는 macOS 로컬 단일 오너 머신에서 Discord를 주 소통 채널로 사용하며 cron 자가치유, 인박스 수집, 검증 기반 작업 레인(lanes)을 총괄하는 데몬입니다. 현재 소유자의 핵심 기억(선호도, 규칙, 지침, 인물 정보)은 이미 OMO 에이전트 런타임의 git 기반 메모리 저장소(`~/.omo/memory/agents/<slug>/repo`)를 통해 `system/human.md` 및 `system/persona.md` 형태로 매 세션 턴에 직접 주입되고 있으며, 에이전트가 자체 `memory` 도구(생성·수정·패치 및 git 자동 커밋)를 통해 무지연·무비용으로 다루고 있습니다.
따라서 게이트웨이 레벨에서 무거운 Graphiti/Mem0식 bi-temporal 사실 추출 파이프라인이나 sqlite-vec 임베딩 인프라를 성급히 도입할 경우, 매 인입 메시지마다 LLM 추출 비용이 지속 발생하여 오너의 비용 계층화 및 이벤트 기반 원칙을 정면으로 위배하고 두 메모리 계층 간 데이터 불일치(Split-brain)를 초래합니다.
선행 과제인 **P0 안전 기반(`approval-effects`, `approval-grants`, `taint` 비신뢰 오염 추적)**이 확립되어 외부 인박스에 의한 사실 오염 위험이 차단된 이후, OMO의 마크다운 메모리를 손상시키지 않고 게이트웨이 내부 컴포넌트(키비처, 크론, 레인) 간 오너 스코프 사실을 공유할 수 있는 경량 SQLite 사실 테이블(`facts`, `episodes`) 및 FTS5 색인으로 단계적 도입하는 것이 타당합니다.

---

## ## 벤치마킹 원천

선행 자율 에이전트 플랫폼 연구에서 조사된 메모리 아키텍처 원천과 핵심 패턴은 다음과 같습니다.

1. **Meta Muse — claims 저장소와 마크다운 이원화 및 provenance 추적 [1차]**
   - **출처**: `mouse.dev/blog/muse-runtime-export` [1차] (연구자 런타임 덤프 및 복원), `about.fb.com/news/2026/09/introducing-muse/` [1차], `meta.com/help` [1차].
   - **핵심 패턴**:
     - 파일시스템 마크다운(`MEMORY.md`, `memory/YYYY-MM-DD.md`, `memory/bank/`)과 관계형 인덱스(`memory.claims`)의 명확한 역할 분리.
     - `claims` 테이블은 단순 텍스트가 아니라 `text`, `evidence_quote`, `message_ids` (출처 메시지 ID), `confidence`, `status`, `supersedes_claim_id`를 보관하여 사실의 변경 이력과 증거를 추적.
     - 대화 내용을 매시간 백그라운드 검증(hourly verifier)하고, 명시적 "forget" 명령 시 claim ID를 철회(retract)하고 연관 마크다운 청크 및 임베딩을 재색인.

2. **Graphiti / Zep — Bi-temporal 지식 그래프 및 시간 무효화(Invalidation) [1차]**
   - **출처**: `github.com/getzep/graphiti` [1차] (SHA `d50741d1bf463cbe3859eb73914cbff1b66839e6`), Graphiti 논문 (arXiv:2501.13956) [1차].
   - **핵심 패턴**:
     - `EntityEdge` 모델을 통해 이중 시간 축(Bi-temporal) 관리: 트랜잭션 시간(`created_at`, `expired_at`: 시스템이 사실을 인지하거나 폐기한 시점)과 이벤트 유효 시간(`valid_at`, `invalid_at`: 현실 세계에서 사실이 유효한 시점).
     - 과거 사실과 모순되는 새로운 사실(예: "내 생일은 3월 2일" -> "사실 3월 3일이야")이 인입되면 기존 에지에 `invalid_at`을 기록하고 새 에지로 대체(supersede)하여 모순 누적을 방지.
     - 모든 사실은 원천 사건 목록(`episodes[]`)과 `reference_time`에 묶여 감사 추적이 가능.

3. **xAI Grok Bot — 4계층 메모리 구조와 실시간 컨텍스트 압축 [1차]**
   - **출처**: Lee Robinson의 Stanford 강연 녹취록 [1차] (`peterp.me/transcripts/how-grok-bot-works/`), xAI 공식 가이드 [1차] (`x.ai/bot/guides/grok-bot-101`).
   - **핵심 패턴**:
     - ① Core Facts (항구적 핵심 사실), ② Daily Logs (일자별 대화 기록), ③ Scratchpad (30~60분 단기 작업 메모), ④ Files (grep 가능한 파일시스템)의 4계층 분리.
     - 컨텍스트 캐시 수명 내에서 선제적 압축(Compaction)을 수행하여 토큰 소모를 최소화하고, 다중 봇 간 컨텍스트 공유 시 민감 정보(비밀번호, 개인 메모리)를 자동 마스킹(Redaction).

4. **Letta (MemGPT) — Sleep-time Compute 및 코어 메모리 블록 [1차]**
   - **출처**: `docs.letta.com` [1차], `github.com/letta-ai/letta` [1차] (Letta 0.7.0).
   - **핵심 패턴**:
     - 에이전트 프롬프트에 항시 노출되는 고정 블록(`human`, `persona`)과 도구를 통해 온디맨드로 조회하는 아카이브 메모리(`archival_memory_search`)를 분리.
     - 사용자가 침묵하는 유휴 시간대에 비동기로 메모리를 요약·정리하는 슬립타임 연산(Sleep-time compute).

5. **Mem0 — 경량 오픈소스 메모리 계층 [1차]**
   - **출처**: `mem0.ai` [1차], `github.com/mem0ai/mem0` [1차], arXiv:2504.19413 [1차].
   - **핵심 패턴**:
     - 사용자(User), 세션(Session), 에이전트(Agent)의 3단계 스코프 분리.
     - 단순 키-값 및 벡터 임베딩 결합 구조로, 대화 턴마다 사실을 추출(Extract)하고 중복을 병합(Consolidate)하는 CRUD 파이프라인.

---

## ## 현재 코드

현재 `omon-gateway` 코드베이스에서 메모리와 관련된 실제 구현 상태와 파일:라인 증거는 다음과 같습니다. 선행 계획서(`docs/research/autonomous-agent-platform-plan.md`)에서 주장한 내용 중 현재 코드와 불일치하거나 변경된 부분도 명시합니다.

1. **세션 스코프 원시 어휘 검색에 머문 `MemoryStore` (`src/memory/store.rs:11-20, 30-70, 100-155`)**:
   - `src/memory/store.rs:11-20`의 `Memory` 구조체는 `id, session_key, content, metadata, created_at, updated_at, score` 필드만 가집니다.
   - `src/memory/store.rs:35-45`의 `remember()`는 세션 키(`session.storage_key()`)와 문자열을 받아 `memories` 테이블에 단순 저장합니다.
   - `src/memory/store.rs:108-115`의 `search()` 쿼리는 `SELECT ... FROM memories WHERE session_key = ? ORDER BY updated_at DESC LIMIT 10000`로, **단일 세션(`session_key`)으로 엄격히 격리**되어 있습니다. 다른 채널, 스레드, 레인 간 공유가 원천 불가능합니다.
   - `src/memory/store.rs:118-133`의 랭킹 로직은 벡터 임베딩이 아니라, 단순 단어 분절 빈도 카운트 `term_counts()`와 코사인 유사도, 어구 포함(`contains(&query_lower)`)을 메모리 상에서 계산하는 순수 lexical 검색입니다. `keyword_search`와 `similarity_search`는 이 `search()`의 단순 별칭(`src/memory/store.rs:157-173`)입니다.
   - `src/memory/store.rs:37-61`: 쓰기 승인이 켜져 있을 때(`write_approval_enabled()`) `pending_writes` 테이블로 스테이징되는 방어벽은 구현되어 있습니다.

2. **단일 오너 환경에서 OMO 런타임에 완전히 위임된 실제 에이전트 메모리 (`src/agent/omo_backend.rs:481-533`)**:
   - 게이트웨이는 `OMON_PER_AGENT_WORKSPACE`가 활성화되면(`src/agent/omo_backend.rs:481-488`), 워크스페이스 디렉토리 하위에 `.omo/omo.json`을 생성하고 `{"memory": {"agent": slug}}`를 기입합니다(`src/agent/omo_backend.rs:512-518`).
   - 이로 인해 실제 대화 및 코딩 에이전트(OMO)는 게이트웨이의 SQLite가 아니라, OMO 런타임이 관리하는 `~/.omo/memory/agents/<slug>/repo` git 저장소의 `system/human.md`, `system/persona.md`를 자동으로 프롬프트에 투영(<self>, <memory>)하여 사용합니다.
   - OMO 에이전트는 이미 `memory` 내장 도구(create, insert, str_replace, patch 등)로 마크다운 메모리를 자율 갱신하고 자동 커밋합니다.

3. **턴 실행 시 프롬프트 주입 부재 및 선행 계획서의 오류 (`src/multiplexer/actor.rs:1326-1445`)**:
   - **선행 계획서 오류 수정**: 선행 계획서(`docs/research/autonomous-agent-platform-plan.md:129, 368`)에서는 `src/multiplexer/actor.rs:1344`에서 메모리를 주입할 수 있다고 언급했으나, 현재 코드 기준 `src/multiplexer/actor.rs:1344`는 단순 `let mut bot_name: Option<String> = None;`이며, 세션 컨텍스트 로딩 시 `bot_profiles` 테이블에서 `system_prompt`와 `enabled_toolsets`를 읽어오는 로직(`src/multiplexer/actor.rs:1342-1361`)에 불과합니다.
   - 신규 세션을 생성하는 `fresh_context()`(`src/multiplexer/actor.rs:1405-1445`) 및 활성 턴을 기동하는 `run_cancelable()`(`src/multiplexer/actor.rs:460-482`) 어디에서도 `MemoryStore`나 외부 사실을 읽어 프롬프트에 주입하는 코드는 전무합니다.

4. **게이트웨이 내에서 `MemoryStore`를 조회하는 유일한 소비자 (`src/lanes/kibitzer.rs:376-410`)**:
   - 키비처의 상태 요약 문서 생성기인 `state_document()` 내부에서 `MemoryStore::new(self.pool.clone()).search(&self.lead, "owner preferences", 10)`을 호출하여 "Owner preferences:\n" 블록에 추가합니다(`src/lanes/kibitzer.rs:385-388`).
   - 그러나 현재 운영 중 프로덕션 코드에서 `MemoryStore::remember()`를 호출하여 사실을 쓰는 주체는 전무(테스트 코드 `src/tools/skills.rs:382`만 존재)하므로, 키비처가 읽어가는 메모리는 사실상 빈 상태이거나 과거 테스트 잔재에 불과합니다.

5. **단일 테이블에 불과한 초기 마이그레이션 (`migrations/0001_initial.sql:53-63`)**:
   - `memories` 테이블(`id`, `session_key`, `content`, `metadata_json`, `created_at`, `updated_at`)만 존재하며, 사실 관계(subject-predicate-object), 유효성 기간(`valid_at`, `invalid_at`), 출처 증거(`episodes`), 신뢰도(`confidence`), 대체 이력(`superseded_by`)을 담을 스키마가 없습니다.

---

## ## 구현 단계

이 테마(`memory-facts`)는 **P0 안전 기반(`approval-effects`, `approval-grants`, `taint`)이 구현된 이후 Phase 3에서 도입**됩니다. 오너의 요구사항(불필요한 LLM 폴링 금지, 저렴한 모델 계층화, 결정론적 기계 검증)에 부합하도록 Graphiti의 거대한 그래프 엔진 대신, SQLite FTS5 기반의 경량 bi-temporal 사실 저장소로 설계합니다.

### 단계 1: 오너 스코프 bi-temporal 사실 및 원천 에피소드 스키마 구축
- **신규/수정 파일**:
  - `migrations/0048_memory_facts.sql` (신규)
  - `src/memory/facts.rs` (신규: Fact, Episode, FactState, FactScope 도메인 모델)
  - `src/memory/mod.rs` (수정: facts 모듈 노출)
- **DB 마이그레이션 (`migrations/0048_memory_facts.sql`)**:
  ```sql
  -- 원본 입력 메시지 및 사건 불변 기록 (Content-Addressed)
  CREATE TABLE IF NOT EXISTS episodes (
      id TEXT PRIMARY KEY NOT NULL,
      content_hash TEXT UNIQUE NOT NULL,
      content TEXT NOT NULL,
      source_platform TEXT NOT NULL,
      source_ref TEXT,
      tainted INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
  );

  -- Bi-temporal 사실 테이블
  CREATE TABLE IF NOT EXISTS facts (
      id TEXT PRIMARY KEY NOT NULL,
      subject TEXT NOT NULL,
      predicate TEXT NOT NULL,
      object TEXT NOT NULL,
      fact_text TEXT NOT NULL,
      scope TEXT NOT NULL DEFAULT 'owner' CHECK (scope IN ('owner', 'channel', 'lane', 'system')),
      sensitivity TEXT NOT NULL DEFAULT 'personal' CHECK (sensitivity IN ('public', 'personal', 'secret')),
      state TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('proposed', 'active', 'retracted', 'superseded')),
      confidence REAL NOT NULL DEFAULT 1.0,
      valid_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      invalid_at TEXT,
      superseded_by TEXT,
      episode_id TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (episode_id) REFERENCES episodes(id),
      FOREIGN KEY (superseded_by) REFERENCES facts(id)
  );
  CREATE INDEX IF NOT EXISTS idx_facts_scope_state ON facts(scope, state);
  CREATE INDEX IF NOT EXISTS idx_facts_validity ON facts(valid_at, invalid_at);

  -- SQLite FTS5 전문 검색 테이블 (토큰 비용 없는 로컬 검색)
  CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
      fact_text,
      subject,
      predicate,
      object,
      content='facts',
      content_rowid='rowid'
  );
  ```
- **환경 변수 Kill Switch**:
  - `OMON_MEMORY_FACTS_ENABLED=false` (기본값: false)
- **기계 검증 시나리오**:
  - SQLite CLI를 통한 스키마 및 외래키 정합성 검증:
    ```bash
    sqlite3 ~/.omon/gateway.db "PRAGMA foreign_keys = ON; INSERT INTO episodes (id, content_hash, content, source_platform, tainted) VALUES ('ep-1', 'hash-1', '오너 생일은 3월 2일', 'discord:dm', 0); INSERT INTO facts (id, subject, predicate, object, fact_text, episode_id) VALUES ('fact-1', 'owner', 'birthday', '03-02', '오너의 생일은 3월 2일이다', 'ep-1'); SELECT f.fact_text, e.source_platform FROM facts f JOIN episodes e ON f.episode_id = e.id WHERE f.id='fact-1';"
    ```
    출력 결과: `오너의 생일은 3월 2일이다|discord:dm` 확인 후 롤백/정리.

### 단계 2: Bi-temporal Invalidation 및 버전 관리 로직 구현
- **신규/수정 파일**:
  - `src/memory/facts.rs` (수정: `insert_fact()`, `supersede_fact()`, `retract_fact()`)
  - `src/memory/store.rs` (수정: `FactStore` 연동 및 FTS5 검색 쿼리 매핑)
- **핵심 동작 규칙**:
  - 기존 사실과 충돌하는 새 사실이 등록될 때, 이전 사실의 `state`를 `superseded`로 변경하고 `invalid_at = Utc::now()`, `superseded_by = new_fact_id`를 하나의 트랜잭션(`BEGIN IMMEDIATE`)으로 원자적 기록.
  - 삭제("forget") 요청 시 물리 삭제가 아닌 `state = 'retracted'`, `invalid_at = Utc::now()`로 마킹하고 FTS5 인덱스에서 제거하여 감사 추적성 보존.
  - `tainted = 1`인 에피소드(이메일, 웹 등 외부 비신뢰 출처)에서 유래된 사실은 `state = 'proposed'`로만 등록되며, 오너 승인 전까지 검색(`active`) 결과에 포함되지 않음.
- **기계 검증 시나리오**:
  - 사실 대체 및 유효성 시점 검증 쿼리:
    ```bash
    sqlite3 ~/.omon/gateway.db "SELECT id, state, valid_at, invalid_at, superseded_by FROM facts WHERE subject='owner' AND predicate='birthday' ORDER BY created_at ASC;"
    ```
    출력 결과: 1행(`superseded`, `invalid_at` 기입, `superseded_by='fact-2'`), 2행(`active`, `invalid_at IS NULL`) 확인.

### 단계 3: 에이전트 메모리 도구(`memory_remember`, `memory_search`, `memory_forget`) 추가
- **신규/수정 파일**:
  - `src/tools/memory.rs` (신규: 도구 정의 및 실행기)
  - `src/tools/mod.rs` (수정: `memory` 도구셋 등록)
  - `bot_profiles/blocks/lanes.md` (수정: 도구 사용 가이드라인 갱신)
- **핵심 도구 사양**:
  - `memory_search(query: String, scope: Option<String>, limit: Option<usize>)`: FTS5를 통해 `state='active'`인 사실만 초고속 반환.
  - `memory_remember(subject: String, predicate: String, object: String, text: String)`: `approval-effects` 정책에 따라 오너 직접 대화는 자동 허용(`Allow`), 외부 인박스나 레인 출처는 승인 대기(`Ask`/`Proposed`).
  - `memory_forget(fact_id: String, reason: String)`: 사실 철회 실행 및 감사 로그 기록.
- **기계 검증 시나리오**:
  - 도구 실행 단위 테스트:
    `cargo test --test test_memory_facts_tools tool_remember_and_search_owner_facts -- --nocapture` (또는 인메모리 SQLite 테스트 검증, exit code 0 확인).

### 단계 4: 세션 컨텍스트 로딩 시 Core Facts 동적 주입 (Turn Injection)
- **신규/수정 파일**:
  - `src/multiplexer/actor.rs` (`load_context`, `fresh_context` 수정: Core Facts 조회 및 주입)
  - `src/lanes/kibitzer.rs` (`state_document` 수정: `MemoryStore` 대신 `FactStore`의 owner facts 주입)
- **핵심 동작 규칙**:
  - `load_context()`에서 세션을 시작할 때, `scope='owner' AND state='active' AND sensitivity!='secret'`인 핵심 사실 상위 N건(최대 500 토큰 엄격 제한)을 조회하여 시스템 프롬프트 하단 `<!-- omon:facts:begin --> ... <!-- omon:facts:end -->` 블록으로 주입.
  - 토큰 낭비 방지를 위해 상세 메모리는 프롬프트에 상시 적재하지 않고 에이전트가 필요 시 `memory_search` 도구로 자율 조회하도록 분리.
- **기계 검증 시나리오**:
  - 게이트웨이 기동 후 세션 컨텍스트 생성 로그 확인:
    ```bash
    grep "injected core facts into session context" ~/.omon/logs/gateway.log | tail -n 1
    ```
    로그 포맷: `session="discord:dm:..." facts_count=5 token_estimate=180` 확인.

---

## ## 위험·의존성

1. **상행 의존성 (Prerequisites)**:
   - `approval-effects`: **필수 선행**. 사실의 생성·수정·철회는 비가역적 쓰기(`Write` / `Irreversible`) 영향도를 가지므로, 결정론적 승인 분류 체계 하에서 통제되어야 합니다.
   - `approval-grants`: **필수 선행**. 자동화 레인이나 백그라운드 크론이 사실을 갱신할 때 신뢰할 수 있는 권한 부여(Grant)가 존재해야 작업이 블록되지 않습니다.
   - `taint`: **필수 선행**. 외부 인박스(이메일, 카카오톡 오픈채팅 등)에서 흘러들어온 비신뢰 입력이 사실 추출 파이프라인을 오염시켜 잘못된 사실을 `active`로 승격시키는 프롬프트 인젝션 공격을 차단해야 합니다.

2. **하행 연계 테마 (Downstream consumers)**:
   - `memory-dream`: **필수 기반 제공**. 야간에 대화와 사건을 재검토하여 사실 diff를 제안하는 dream 프로세스는 반드시 `episodes`와 `facts` 테이블이 존재해야만 동작할 수 있습니다.
   - `kibitzer-action`: 키비처가 소유자 선호도와 현재 제약 조건을 파악하여 무리한 행동 제안을 자제하도록 돕습니다.
   - `instinct-focus`: 오너의 단기 관심사와 장기 목표를 facts 스코프로부터 조회하여 포커스를 정렬합니다.

3. **고유 위험 요인 (Risks)**:
   - **OMO 마크다운 메모리와의 이원화 및 데이터 불일치 (Split-brain)**:
     - 현재 OMO 에이전트가 다루는 `~/.omo/memory/agents/<slug>/repo/system/human.md`와 게이트웨이의 SQLite `facts` 테이블이 서로 동기화되지 않고 각자 사실을 기록하면 소유자에 대한 정보가 분열됩니다.
     - **완화책**: 게이트웨이의 `facts`는 데몬 컴포넌트(키비처, 크론, 상태 감사, 채널 횡단 검색)를 위한 정형화된 메타데이터 색인으로 한정하고, 에이전트 대화의 심층 페르소나는 OMO의 마크다운 저장소를 신뢰 출처(Single Source of Truth)로 존중해야 합니다.
   - **무분별한 LLM 추출로 인한 비용 폭증 (Token Cost Blowup)**:
     - 인입되는 모든 일상적 대화마다 LLM 사실 추출(fact extraction) 파이프라인을 가동하면 비용이 낭비됩니다.
     - **완화책**: 상시 대화 중 자동 추출은 금지하고, 오너의 명시적 지시(`memory_remember`) 또는 야간에 저렴한 모델(`inferhub/cb/deepseek-v4.1-flash`)로 배치 처리하는 `memory-dream`을 통해서만 사실을 등록합니다.
   - **SQLite FTS5 및 문자열 인덱스 비대화**:
     - FTS5 인덱스가 커질 경우 단일 SQLite 파일 크기가 증가하고 I/O 지연이 발생할 수 있습니다.
     - **완화책**: 주기적인 `retracted` 사실 아카이빙 및 `optimize` 트랜잭션을 크론 정기 작업으로 연계합니다.

# Hermes Agent 잔여 패리티 감사: 2026-09-12

전체 85개 패리티 유닛 중 82개 유닛의 증거 수집과 구현 검증을 완료했다.
잔여 불일치는 U36, U37, U73 3개 유닛에서 총 9건의 잔여 사항(PARTIAL)으로 확인됐다.
의도적 불일치 14건과 범위 제외 10건을 제외한 모든 영역이 현행 트리(commit 1f75c65)와 Hermes 기준(commit 146f4ed) 사이에서 정합성을 만족한다.

## 1. 잔여 패리티 (REMAINING)

### 1.1 U36 'Live durable outbound obligations' (PARTIAL)

구현 확정 항목:
* 프로덕션 `record_obligation` 호출부 연동 완료 (`src/cron/scheduler.rs:1697-1698`, `src/agent/omo_backend.rs:705-710,1094-1099`)
* 최종 발송 실패 시 `mark_obligation_failed` 처리, `Err` 반환 및 ACK 스킵 정상 동작 (`src/agent/omo_backend.rs:726-745`)
* 부트 스윕 실행 시 `owner_pid` 및 `owner_started_at` 가드 적용 (`src/ledger/service.rs:487-495`, `migrations/0020_obligation_owner.sql`)
* 상태 머신 전이 및 재시도 횟수 소진 시 `abandoned` 전이 검증 완료 (`src/ledger/service.rs`)

잔여 결함 목록:
| 잔여 항목 | 판정 및 검증 노드 | 현행 트리 위치 | Hermes 참조 위치 | 영향도 |
| :--- | :--- | :--- | :--- | :--- |
| **U36.1** 터미널 정리 미호출 | CONFIRMED (w2-v36) | `src/ledger/service.rs:541-584` | `gateway/delivery_ledger.py:176-178,276-304` | `prune_terminal_obligations`의 프로덕션 호출부와 테스트가 모두 전무하여 종결된 obligation 행이 무한 누적됨 |

### 1.2 U37 'Scheduler-owned agent and gate results' (PARTIAL)

구현 확정 항목:
* 실행기 억제 플래그 및 메타데이터 반환 로직 (`src/cron/executor.rs:318-343`, `src/agent/omo_backend.rs:694-701`)
* `cron_outputs` 테이블 영속화 (`src/cron/scheduler.rs:1310-1318`)
* 침묵 센티널 이중 차단 메커니즘 (`src/cron/scheduler.rs:200-201,1543-1547`, `src/discord/adapter.rs:2614,2702,2911`)
* 다중 목적지 전송 및 세션 미러링 (`src/cron/scheduler.rs:1729-1736`)
* 에이전트 실행 전 웨이크 게이트 선행 평가 (`src/cron/executor.rs:63-70`)
* 스크립트 없는 `no_agent` 요청 거절 (`src/cron/executor.rs:54-58`)

잔여 결함 목록:
| 잔여 항목 | 판정 및 검증 노드 | 현행 트리 위치 | Hermes 참조 위치 | 영향도 |
| :--- | :--- | :--- | :--- | :--- |
| **U37.1** 게이트 침묵 시 페이로드 폴백 배송 | CONFIRMED (w2-v37) | `src/cron/scheduler.rs:1528-1542`, `src/cron/executor.rs:69,76,158,462` | `cron/scheduler.py:3844-3847` | wake-gate, 빈 stdout, 변경 없는 모니터가 `Ok(None)`을 반환할 때 폴백 로직이 기본 payload 문구를 전송해버리는 회귀 위험 존재 |
| **U37.2** 도구 전용 빈 턴 "Done." 마커 합성 | CONFIRMED (w2-v37) | `src/agent/omo_backend.rs:655-660,1044-1050` | `cron/scheduler.py:3844-3847` | 도구만 실행된 빈 cron 턴에 "Done."을 합성하여 성공 처리함. Hermes는 빈 응답을 soft failure로 처리 |
| **U37.3** 네이티브 cron 조기 스킵 누락 | CONFIRMED (w2-v37) | `src/cron/executor.rs:456-474` (비교: `71-77`) | `cron/executor.py` 대조 구간 | 프롬프트가 존재함에도 stdout이 비어있을 때 건너뛰는 방어 로직이 `execute_native_cron`에 없음 |

### 1.3 U73 'UP.IO-02 bot/owner-instance runtime failed-send claims + reconnect catch-up' (PARTIAL)

구현 확정 항목:
* `DeliveryLedgerService::sweep_failed_for_runtime` 및 `DiscordEgress::replay_failed_transport_obligations` 독립 메서드 구현 및 단위 테스트 작성 완료

잔여 결함 목록:
| 잔여 항목 | 판정 및 검증 노드 | 현행 트리 위치 | Hermes 참조 위치 | 영향도 |
| :--- | :--- | :--- | :--- | :--- |
| **U73.1** 실패 재전송 프로덕션 미배선 | CONFIRMED (w2-v73) | `src/discord/adapter.rs:2458` | 런타임 큐 sweep 호출부 | 메서드 호출이 오직 `tests/test_discord_adapter.rs:4300`에만 존재하여 실제 런타임에서 재전송이 동작하지 않음 |
| **U73.2** Resume 이벤트 미수신 | CONFIRMED (w2-v73) | `src/discord/adapter.rs:1141,1166` | 재연결 세션 처리기 | `Ready` 외의 `FullEvent::Resume`을 핸들링하지 않고 와일드카드 `_ => {}`로 무시하여 재연결 캐치업 누락 |
| **U73.3** 재연결 레이스 없는 무조건 실패 처리 | CONFIRMED (w2-v73) | `src/agent/omo_backend.rs:730-736`, `src/cron/scheduler.rs:1716-1721`, `src/ledger/service.rs:420-424` | `gateway/delivery_ledger.py` | 연결 복구 레이스 컨디션을 확인하지 않고 즉시 실패로 확정 |
| **U73.4** 스윕 시 무조건 attempt 소진 [^fn-u73-4] | CONFIRMED (w2-v73) | `src/ledger/service.rs:648-653` | `gateway/delivery_ledger.py:189-207` | 스윕 시 시도 횟수를 증가시키고 실패 상태로 돌릴 릴리스 API가 없어 시도 횟수가 조기 소진됨 |
| **U73.5** 비정형 문자열 에러 분류 [^fn-u73-5] | CONFIRMED (w2-v73) | `src/discord/adapter.rs:2322-2330`, `src/ledger/service.rs:624-636` | 정형 에러 분류 규약 | 에러를 단순 소문자 매칭 및 `last_error` 부분 문자열로 파악하여 정밀한 disposition 분류 불가 |

[^fn-u73-4]: w2-v73 검증 노드 정정 사항: Hermes는 별도 release API를 두는 대신 `sweep_recoverable`의 `deliverable_platforms` 필터링 단계에서 attempt 소진을 사전 방어함.
[^fn-u73-5]: w2-v73 검증 노드 확인 사항: 단순 에러 메시지 텍스트 검색에 의존하므로 전송망 특화 에러 분류체계 보강 필요.

## 2. 이번 조사에서 해소 확인된 항목

세부 정밀 감사 과정에서 기존 의심 항목 3건의 정상 구현을 입증했다.

1. **마이그레이션 0025 통합 확인**
   * `migrations/0025_cron_monitor_states.sql`이 코드베이스에 완전히 연결되어 있음을 확인했다.
   * `src/cron/executor.rs:145`의 SELECT 쿼리와 `src/cron/executor.rs:164`의 INSERT 쿼리를 통해 모니터 해시 스킵 기능이 정상 동작한다.

2. **U52/U53/U54 증거 기반 종결 (CLOSED-BY-EVIDENCE)**
   * 마이그레이션 대상 소스는 수신 확인 및 소유권 유효성 검증을 마친 뒤에만 비워진다 (`src/migrate/cron_cutover.rs:492-515`).
   * 큐에센스(quiesce) 처리가 `src/migrate/mod.rs:185-188`에 보장되어 있다.
   * 드라이런(dry-run) 검증은 `HermesJob::validate`를 실제 반영 경로와 동일하게 공유한다 (`src/migrate/mod.rs:265-296`, `src/cron/store.rs:320-398,940-943`).

3. **데몬 불변식 충돌 해소 (R.R14 / CFG.I03, U66)**
   * `src/agent/omo_config.rs:55`에 `CRON_APPSERVER_URL_DEFAULT = ws://127.0.0.1:19742`가 명시되어 있다.
   * 슈퍼바이저는 포트나 URL이 명시적으로 다를 때만 보조 데몬을 생성한다 (`src/main.rs:770-774`).
   * 기본 구성에서는 단일 멀티플렉싱 데몬이 통합 서빙을 담당하여 설계 불변식을 충족한다.

## 3. 증거 커버리지 (85유닛)

총 85개 유닛 중 82개 유닛에 대해 1,000건의 아티팩트와 61개 dual red/green exit pair, 12개 log pair 검증을 확보했다.

* **증거 갭 (EVIDENCE-GAP, 3유닛)**: U36, U37, U73 (1장의 잔여 목록).
* **Green-only 예외 5유닛**: U04, U07, U25, U45, U71 (리드 스팟 검증을 통해 코드 레벨 정합성 확인 완료).
* **마이그레이션 단위 증거 뉘앙스**: U52, U53, U54는 직접적인 cargo run 로그 대신 패치 및 동작 증거 아티팩트로 검증 종결.

## 4. 의도적 불일치 14건과 범위 제외 10건

### 4.1 의도적 불일치 (14건)
* `AP.AP17`: Discord 인터랙션 응답 타임아웃 차이 수용
* `AP.AP21`: 백엔드 에러 전파 프로토콜의 Rust enum 매핑
* `CFG.I01`: TOML 기반 정적 환경설정 로더 채택
* `CFG.I02`: 환경 변수 우선순위 오버라이드 단일화
* `CFG.I03`: 앱서버 기본 URL 단일 포트 멀티플렉싱
* `CR.C22`: 크론 실행 결과 반환 시 구조화된 메타데이터 포맷 적용
* `D.D28`: 데이터베이스 연결 풀 수명 주기 Rust 런타임 제어
* `D.D29`: SQLite WAL 모드 전용 동기화 플래그 적용
* `D.D31`: 세션 스토리지 영속화 방식의 차이
* `R.R20`: 에이전트 턴 제어 루프의 비동기 채널 설계
* `R.R21`: 중단 신호 처리 시 채널 드롭 방식 활용
* `S.S07`: 스케줄러 락 구현 시 SQLite 트랜잭션 의존
* `S.S09`: 부팅 시 고아 작업 회수 인터벌 간격 조정
* `S.S10`: 크론 재시도 백오프 지수 계산식 차이

### 4.2 범위 제외 (10건)
* `AP.AP20`: 레거시 HTTP 엔드포인트 호환 계층 제외
* `CFG.N01`: 미사용 레거시 환경 플래그
* `CFG.N02`: 다중 테넌트 파일 로더
* `CFG.N03`: 실험적 플래그 프로파일
* `CR.C23`: 분산 크론 조정 프로토콜
* `CR.C24`: 외부 웹훅 트리거 크론
* `D.D30`: 분산 데이터베이스 복제
* `R.R19`: 서드파티 원격 LLM 직결 통신
* `S.S02`: 분산 노드 간 작업 마이그레이션
* `S.S08`: 외부 워커 풀 밸런싱

### 4.3 아키텍처 명시적 배제 영역
1. **최종 결과 단일 프레젠테이션 (final-only presentation)**: 중간 청크 스트리밍을 배제하고 완료된 최종 메시지만 디스코드에 출력.
2. **봇별 영구 레인 (per-bot permanent lanes)**: 동적 워커 분기 대신 봇 단위의 정적 디스패치 레인 유지.
3. **OMO 위임 지능 (OMO-delegated intelligence)**: 지능형 추론과 도구 호출 결정을 OMO 코어 엔진에 위임.
4. **macOS launchd 및 Discord 전용 환경**: 리눅스 전용 systemd나 타 메신저 지원을 제외하고 단일 플랫폼 최적화.
5. **음성 채널 및 TTS 혼합 미지원 (no VC/TTS mixing)**: 텍스트 채널 통신에만 집중.
6. **전송 계층 한정 재시도 (transport-layer-only retries)**: 애플리케이션 레벨의 무한 재시도를 금지하고 네트워크 전송 실패에 한해서만 제어.

## 5. Hermes 참조 체크아웃 상태

* **HEAD commit**: `146f4ed`
* **Dirty 파일 5건**:
  * `agent/chat_completion_helpers.py`
  * `agent/codex_responses_adapter.py`
  * `agent/transports/codex.py`
  * `gateway/run.py`
  * `tests/agent/test_codex_responses_adapter.py`
* **드리프트 상태**: `reference-version.txt` 기준 코드 드리프트 없음.

## 6. 권고 후속 웨이브

위험도와 배선 누락 수준을 고려하여 순차적인 작업 착수를 권고한다.

1. **U37.1 페이로드 폴백 재배송 방지 (최우선 회귀 위험)**
   * 게이트 침묵, 빈 stdout, 모니터 미변경에 따른 `Ok(None)` 반환을 명확한 disposition enum으로 구분해야 한다.
   * 폴백 조건식(`src/cron/scheduler.rs:1528-1542`)이 게이트 침묵 건에 원본 payload를 배송하지 않도록 조치한다.

2. **U73.1 및 U73.2 Discord 복구 런타임 배선**
   * `FullEvent::Resume` 핸들러를 `src/discord/adapter.rs:1141` 부근에 추가한다.
   * 재연결 완료 시 `replay_failed_transport_obligations`를 호출하여 미발송 obligation을 비동기 전송한다.

3. **U36.1 prune_terminal_obligations 호출부 배선 및 단위 테스트**
   * `record_obligation` 주기 또는 백그라운드 태스크에서 정기적으로 종결 obligation을 정리하도록 배선한다.
   * 삭제 상한 및 보존 기간을 검증하는 테스트를 추가한다.

4. **U37.2 도구 전용 빈 턴 처리 정합성 확보**
   * 에이전트 턴에서 텍스트 출력이 없을 때 임의의 "Done." 마커를 합성하지 않고 Hermes 규약에 맞춰 soft failure 처리한다.

5. **U37.3 execute_native_cron 경로 조기 스킵 보강**
   * 프롬프트가 존재하는데 표준 출력이 빈 경우 불필요한 배송을 건너뛰는 검사를 네이티브 실행기에 적용한다.

6. **U73.3 ~ U73.5 안정성 보강**
   * 전송 에러 파싱을 단순 문자열 매칭에서 전용 에러 enum 기반 구조로 개편한다.
   * 스윕 시 불필요한 재시도 횟수 누적을 막고 재연결 레이스 보호 로직을 추가한다.

## 부록: 방법론 및 증거

* **수행 방식**: mass-ulw 3 waves
* **사용 모델**: `mahoquot/z-ai/glm-5.3-flash` (thinking max)
* **웨이브 구성**:
  * Wave 1 (수확): 7개 분석 노드 + 2개 복구 노드로 증거 수집
  * Wave 2 (반증): 3개 검증 노드 + 1개 복구 노드로 전수 교차 검증
  * 리드 수동 점검: 총 11개 항목 독립 확인 (불일치 0건)
* **중간 산출물 경로**:
  * 감사 디렉터리: `/Users/indo/code/project/omon-gateway/.omo/evidence/remaining-parity-20260912/`
  * 세부 보고서: `w1-*.md`, `w1b-*.md`, `w2-*.md`
  * 작업 메모장: `/tmp/ulw-remaining-parity-20260912.md`

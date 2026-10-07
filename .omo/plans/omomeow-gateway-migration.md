# omomeow-gateway-migration - Work Plan

## TL;DR (For humans)
<!-- Fill this LAST, after the detailed plan below is written, so it summarizes the REAL plan. -->
<!-- Plain English for a non-engineer: NO file paths, NO todo numbers, NO wave/agent/tool names. -->

**Who this is for and what changes for them:** 소유자(indosaram) — Discord #work에서 오모냥에게 말하면 오늘 50~90초 걸리던 응답이 수 초로 줄고, 크론/감시가 채팅을 막지 않으며, 봇이 조용히 죽는 사태(무응답 마비)가 구조적으로 불가능해짐.

**What you'll get:** Rust omon-gateway가 Discord 수신·스레드별 세션·크론 14종·버튼·음성전사·SLA 감시를 전담하고, 장시간 작업은 기존 herdr 탭 위임으로 병렬 실행되는 통합 운영 형태. 4단계 계층 전환으로 무중단 이관.

**Why this approach:** 오늘의 마비 원인이 307K 단일 세션+직렬 큐+순환 감시자임을 두 코드베이스 탐색으로 확정했고, gateway에 버튼·리액션·스레드·세션 모델 핀이 이미 네이티브로 존재해 이관 비용이 낮기 때문. 전역 omo 설정은 일절 건드리지 않음.

**What it will NOT do:** 오모냥 봇 ID/토큰 교체 없음, 신규 정식 봄 없음(섀도우는 테스트 채널 한정), 전역 omo 설정 수정 없음, 소유자 미커밋 WIP 덮어쓰기 없음, 오모냥 기능 무단 폐기 없음(폐기는 307K 고드세션뿐).

**Effort:** Large
**Risk:** Medium - 프로덕션 Discord 봇 전환이 포함되나 4단계 계층+롤백 드릴로 완화
**Decisions to sanity-check:** 작업 실행 계층=하이브리드(herdr 위임 유지), 4단계 계층 전환, 섀도우 봇 신규 생성 — 소유자 승인 완료(2026-10-07)

Your next move: /ulw-execute로 실행 세션 시작 (또는 본 세션에서 "시작" 지시). Full execution detail follows below.

---

> TL;DR (machine): <1 line - effort, risk, deliverables>

## Scope
### Affected user and ideal state
**Affected user:** 소유자 indosaram — 오늘 Discord #work에서 오모냥(307K 메가세션)에 말하면 50~90초 응답, 크론/모니터가 같은 큐를 점유, 무응답 사태 시 수동 복구. 이후: gateway가 Discord 전송·세션 격리·크론·SLA 감시를 담당하고, 장시간 작업은 기존 herdr 위임 하네스로 병렬 실행.

| Row | Statement | Reason |
| --- | --- | --- |
| IS-1 | 소유자가 #work에서 말하면 👀 즉시 반응 + 수 초 내 첫 응답 | 스레드별 격리 세션으로 컨텍스트 소형 유지 |
| IS-2 | 크론/알림 15종이 기존과 동일한 시간·채널·형식으로 발행되되 채팅 응답을 절대 막지 않음 | cron 레인 분리 + delivery-verified ack |
| IS-3 | 봇이 조용히 죽지 않음: 61초 무응답 감지·에스컬레이션, 16분 진행 하트비트, 턴 fail-fast | LLM 비의존 Rust 감시 루프 |
| IS-4 | 모든 세션·크론이 inferhub/cb/deepseek-v4.1-flash 단일 모델, 전역 omo 설정 무변경 | 소유자 지시(2026-10-05/07) |
| IS-5 | herdr 탭 병렬 위임 패턴과 기존 32개 기능 전수 보존(폐기는 308K 고드세션뿐) | 기능 저하 금지 |
| GAP-1 | 오늘: 단일 메가세션 직렬 → 이후: 스레드별 격리 세션 | IS-1 |
| GAP-2 | watch.ts 이벤트가 본체 큐 점유 → gateway 크론/레인 이관 | IS-2 |
| GAP-3 | UNANSWERED가 죽은 세션으로 발행 → sla_watch 네이티브 | IS-3 |
| GAP-4 | 버튼·음성전사·메일/캘린더·JOBS의 gateway 부재 → 구현/이관 | IS-5 |
| GAP-5 | 봇 토큰 이중 접속·전환 리스크 → 4단계 컷오버+롤백 | IS-5 |

### Must have
- OmOMeow 봇 정체성 유지: Bot ID 1465631383862120451, 토큰 단일 소스 컷오버
- 31개 기능 커버리지 매트릭스 100% (Moved 29 / Stayed 2 / Dropped 1 = 고드세션)
- 모든 세션·크론 모델 = inferhub/cb/deepseek-v4.1-flash (OMON_DEFAULT_MODEL + 세션 active_model)
- 4단계 컷오버 각 단계별 롤백 절차
- 전달-검증 ack 유지 (크론) + sla_watch 네이티브 감시

### Must NOT have (guardrails, anti-slop, scope boundaries)
- 전역 omo 설정(~/.omo/agent/omo.json, ~/.omo/omo.json, ~/.omo/settings.json) 수정 금지
- 오모냥 봇 ID/토큰 교체, 신규 정식 봇 생성 (섀도우 봇은 테스트 채널 한정)
- 소유자 WIP(미커밋 4파일 +163/−32) 덮어쓰기/소실 — Phase 0에 보존 필수
- 레거시 LlmBackend 재도입, dashboard 인증 우회
- 구세션 01a0f31c의 Discord 모니터가 gateway 가동 중 이중 접속하는 상태 방치

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: tests-after + 기존 유지 테스트 그린 유지 (cargo test — workspace는 tests/test_review_agent·dashboard가 고정 포트 29998 충돌하므로 `--test <name>` 개별 실행)
- Rust 게이트: `cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings` + `cargo check` 전부 exit 0
- 실서면 검증: 각 컷오버 단계에서 실 Discord 채널 시나리오 실행(아래 QA) — Discord API 응답/메시지 타임스탬프를 이vidence로 캡처
- Evidence: .omo/evidence/omomeow-gateway-migration/ (작업별 하위 디렉터리)

## Execution strategy
### Parallel execution waves
- Wave 1: 보존·준비 (todo 0) — 단독 선행
- Wave 2: gateway Rust 델타 병렬 (todo 1~7, 파일 스코프 비겹침: buttons.rs / omo_backend.rs / 신규 sla_watch·work_items·adapter 스레드 이벤트)
- Wave 3: 위임 CLI + 설정·데이터 마이그레이션 (todo 8~11)
- Wave 4: 컷오버 실행 (todo 12~15, 순차 — 프로덕션 운영 전이)

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | — | 1~15 | — |
| 2 (buttons parity) | 1 | 14 | 3,4,5,6,7,8 |
| 3 (fallback scope) | 1 | 14 | 2,4,5,6,7,8 |
| 4 (sla_watch) | 1 | 8,13 | 2,3,5,6,7 |
| 5 (thread lifecycle) | 1 | 11,14,15 | 2,3,4,6,7,8 |
| 6 (cron payload) | 1 | 11 | 2,3,4,5,7,8 |
| 7 (voice STT) | 1 | 14 | 2,3,4,5,6,8 |
| 8 (work_items+heartbeat) | 1,4 | 9,16 | 2,3,5,6,7 |
| 9 (delegate CLI) | 8 | 16 | 10,11,12 |
| 10 (env/profiles) | 1 | 13,14,15 | 1~8 |
| 11 (cron seed) | 5,6 | 13 | 9,10,12 |
| 12 (shadow bot) | — | 14 | 10,11 |
| 13 (Phase 2) | 4,11 | 14,15 | — |
| 14 (Phase 3) | 2,3,5,7,12,13 | 15 | — |
| 15 (Phase 4) | 10,14 | 16 | — |
| 16 (Phase 5) | 9,15 | — | — |

## Todos

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [x] 1. WIP 보존: git diff HEAD > .omo/evidence/omomeow-gateway-migration/owner-wip-base.patch 후 git stash push -m "owner-wip-20261007" — verify: git status --short가 untracked(drafts/plans 제외) 0이고 stash list에 항목 존재, cargo check exit 0. cargo check 실패 시(HEAD 미빌드) stash pop 후 git commit으로 WIP를 별도 커밋으로 보존하는 폴백 사용
  What to do / Must NOT do: 소유자 WIP(4파일 +163/−32)를 패치+스태시로 보존. 덮어쓰기/삭제 금지. 복원 절차(git stash pop 또는 patch apply)를 롤백 절차에 문서화
  Closes: GAP-5 (리스크: dirty_worktree)
  Parallelization: Wave 1 | Blocked by: — | Blocks: 1~15
  References: git status 실측(2026-10-07, 4파일 M +163/−32), HEAD ad5722c
  Recommended task executor category: quick
  Acceptance criteria (agent-executable): patch 파일 존재 && `git stash list | grep owner-wip-20261007` 성공 && `cargo check` exit 0
  QA scenarios: happy(스태시 후 클린 트리 빌드) + failure(HEAD 빌드 실패 시 커밋 폴백), Evidence .omo/evidence/omomeow-gateway-migration/owner-wip-base.patch
  Commit: N (보존 작업)
- [x] 2. buttons.rs 패리티 3종 수정: ①실행 인터랙션 dedup LRU(500) 추가 ②3초 ack 실패 시 "⚠️ 버튼 응답 확인에 실패해 실행하지 않았습니다" 게시+액션 미실행 ③nonterminal_actions(preview) 버튼 disabled=false 유지
  What to do / Must NOT do: src/discord/buttons.rs만 수정. button-actions.json 스키마 변경 최소화(nonterminal_actions 옵셔널 추가)
  Closes: GAP-4
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 14
  References: src/discord/buttons.rs:83-125,224-280,329-371; ~/.omon/button-actions.json; 오모냥 buttons.ts:80-118(3초 ack 계약), 148-230(dedup LRU)
  Recommended task executor category: deep-low
  Acceptance criteria (agent-executable): cargo test --test <buttons 테스트> exit 0 + 신규 유닛테스트 3건(dedup/ack실패/nonterminal) RED→GREEN
  QA scenarios: 실 Discord 테스트 채널에서 버튼 클릭 → 3초 내 ack + 라벨 갱신 확인(13단계에서 재검증), Evidence .omo/evidence/omomeow-gateway-migration/buttons/
  Commit: Y | feat(discord): button parity with omomeow — dedup, ack-failure reply, nonterminal actions
- [x] 3. omo_backend 폴백 스코프 확대: model-rejected/스레드 거부 에러를 fallback_eligible에 추가, FALLBACK_MODEL=inferhub/cb/deepseek-v4.1-flash 기본값, primary==FALLBACK 1회 가드 유지, metadata.omo_fallback_model_active 영속 유지
  What to do / Must NOT do: src/agent/omo_backend.rs + omo_config.rs만. 전역 omo 설정 건드리지 않음. 저장 모델 문제(오모냥 실증)와 달리 gateway는 턴마다 active_model을 명시 전달하므로 폴백은 일시 장애용
  Closes: GAP-3 (쿼터 사망 시 침묵 방지)
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 14
  References: src/agent/omo_backend.rs:215-220,600-610; omo_config.rs:180-184
  Recommended task executor category: deep-low
  Acceptance criteria: 신규 유닛테스트(model-rejected → fallback 시도) GREEN + 기존 tests/test_omo_backend.rs 전부 GREEN
  QA scenarios: happy(429 시뮬레이션 → 폴백 턴 성공) + failure(폴백도 실패 → 에러 카드 발행), Evidence .omo/evidence/omomeow-gateway-migration/fallback/
  Commit: Y | fix(agent): widen fallback eligibility to model-rejected errors, default FALLBACK_MODEL to deepseek
- [x] 4. sla_watch 구현: 마이그레이션 SQL(sla_watch 표, ultrabrain 계약서 DDL) + tokio 15초 인터벌 감시 태스크. 오너/페어드 메시지만 삽입, delivery_ledger delivered → answered, active_turn/resume_pending 진행 중이면 대기 연장, 미전달+턴 부재 시 alert 채널 1회 에스컬레이션. 부팅 그레이스 기간 에스컬레이션 스킵
  What to do / Must NOT do: LLM 턴 의존 금지(Rust 직접 Discord REST). 무한 재에스컬레이션 금지(1회 + 수동 re-arm)
  Closes: GAP-3
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 8,13
  References: ultrabrain 계약 3(sla_watch DDL 전문), src/discord/adapter.rs:2000-2006(타이핑 패턴 참고), storage/db.rs 마이그레이션 패턴
  Recommended task executor category: deep-low
  Acceptance criteria: 마이그레이션 적용 시 exit 0 + 감시 태스크 유닛테스트(미답변 60초 → escalated, answered 전이) GREEN
  QA scenarios: happy(무응답 60초 → 에스컬레이션 메시지 1회) + failure(응답 도착 → answered 전이, 이중 에스컬레이션 없음), Evidence .omo/evidence/omomeow-gateway-migration/sla/
  Commit: Y | feat(gateway): native SLA watchdog over delivery ledger
- [x] 5. 스레드 수명주기 이벤트: adapter handle_event에 ThreadCreate/Update/Delete 핸들러 추가. 아카이브=actor 드레인 후 suspend, 언아카이브=suspend 해제( lazy resume), 삭제=omo_thread_id 제거+상태 보존. 메시지 이벤트 우선원칙(suspend 스레드 메시지 도착 시 자동 reopen). 아카이브 상태 resume 실패 → 재시도 루프 대신 "스레드 닫힘" 오너 멘지
  What to do / Must NOT do: 세션 표 신규 생성 금지(기존 sessions.state_json 재사용). 턴 진행 중 suspend 금지(드레인 순서 엄수)
  Closes: GAP-1 (스레드=세션 컨텍스트 격리 완성)
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 11,14,15
  References: ultrabrain 계약 1(전문), discord/adapter.rs handle_event, agent/omo_backend.rs:242-325(resume/start), discord/adapter.rs:837-845(mark_thread_owner)
  Recommended task executor category: deep-high
  Acceptance criteria: 스레드 수명주기 유닛테스트(생성/아카이브/재개/삭제 4전이) GREEN + 기존 어댑터 테스트 GREEN
  QA scenarios: happy(테스트 스레드 아카이브→재개→메시지 재응답) + failure(아카이브된 스레드 resume 실패 → 명확한 오너 멘지 1회), Evidence .omo/evidence/omomeow-gateway-migration/thread-lifecycle/
  Commit: Y | feat(discord): thread lifecycle events — suspend/resume/delete session binding
- [x] 6. 크론페이로드 델타: HermesJob에 brief_file(Option<PathBuf>)·catch_up_hours(Option<u64>, 기본 6) 필드 추가. 실행자는 매 실행 brief_file을 읽고(없으면 prompt), catch_up은 발생당 1회 멕업 실행. 스레드 행선지 origin 형식 검증. timezone 태깅(Asia/Seoul/UTC) + 계산 불가 식 거절
  What to do / Must NOT do: 스키마 마이그레이션 없이 payload_json 레벨로만. 기존 잡 하위호환 유지
  Closes: GAP-2
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 11
  References: ultrabrain 계약 2(전문), cron/store.rs:230-305,525-705, cron/scheduler.rs:1545-1635
  Recommended task executor category: deep-low
  Acceptance criteria: 신규 유닛테스트(brief_file 로딩, catch_up 멕업 1회, timezone 태깅) GREEN + 기존 크론 테스트 GREEN
  QA scenarios: happy(brief_file 잡 정상 실행) + failure(멕업 중복 실행 없음), Evidence .omo/evidence/omomeow-gateway-migration/cron-payload/
  Commit: Y | feat(cron): brief_file + catch_up_hours payload fields, thread destination via origin
- [x] 7. 음성 전사 훅: 어댑터 첨부 처리에 오디오 감지(audio/*, .ogg, .mp3, .m4a) → ffmpeg 16kHz mono WAV 변환 → whisper-cli(ggml-large-v3-turbo) 셸아웃 → 트랜스크립트를 세션 턴 컨텍스트(voice_transcript) 주입. 타임아웃/실패 시 오류 표시 후 텍스트 첨부 폴백
  What to do / Must NOT do: Discord 보이스채널 연결 아님(첨부 파일만). 락 길이 제한(예: 25MB) 초과 시 폴백
  Closes: GAP-4
  Parallelization: Wave 2 | Blocked by: 1 | Blocks: 14
  References: 오모냥 listen.ts:31-48(ffmpeg/whisper 파이프라인), voice/pipeline.rs 스텁, discord/adapter.rs 첨부 처리부
  Recommended task executor category: deep-low
  Acceptance criteria: 유닛테스트(오디오 감지/변환 파이프라인 mock 실행) GREEN + 실 오디오 파일 1건 수동 변환 성공 로그
  QA scenarios: happy(짧은 m4a → 트랜스크립트 세션 주입) + failure(비오디오 첨부 무시), Evidence .omo/evidence/omomeow-gateway-migration/voice/
  Commit: Y | feat(discord): voice message transcription hook via whisper-cli
- [x] 8. work_items 테이블 + 15분 진행 하트비트: work_items(id, title, thread_id, delegate_pane, status, last_progress_at, receipt_path) 마이그레이션 + tokio 60초 스캔 루프 — 15분 경과 작업의 워커 생존 검사 후 Rust 직접 Discord REST로 2줄 진행 보고 게시, last_progress_at 갱신
  What to do / Must NOT do: LLM 턴 소모 금지(직접 REST). work-map.json은 유지(호환)하되 원장은 SQLite 우위
  Closes: GAP-3, IS-5
  Parallelization: Wave 2 | Blocked by: 1,4 | Blocks: 9,16
  References: architect 자문 4(B) 하트비트 설계, 오모냥 work-map.json 스키마, storage/db.rs
  Recommended task executor category: deep-low
  Acceptance criteria: 마이그레이션 적용 exit 0 + 하트비트 유닛테스트(15분 경과 → 게시, 갱신 → 스킵) GREEN
  QA scenarios: happy(장기 작업 15분 경과 → 스레드에 진행 보고) + failure(보고 실패 시 로그만), Evidence .omo/evidence/omomeow-gateway-migration/heartbeat/
  Commit: Y | feat(gateway): work_items ledger + 15-min progress heartbeat
- [x] 9. 위임 CLI(bom/bin/omomeow-delegate 또는 스크립트): 인자 title/repo/brief → herdr 워크스페이스·탭 생성, work_items 등록, 접수 확인 출력. 세션 페르소나(AGENTS.md 포트)에 tool 사용법 명시
  What to do / Must NOT do: 기존 herdr CLI 패턴 그대로(pane run "omo --model deepseek --session-id ..."), shell string 결합 금지(argv 배열)
  Closes: IS-5
  Parallelization: Wave 3 | Blocked by: 8 | Blocks: 16
  References: 오모냥 herdr-boot.sh/pane run 패턴, OMOMEOW-MODE.md gist 17/18, architect 자문 4(A) 핸드오프 프로토콜
  Recommended task executor category: deep-low
  Acceptance criteria: CLI 실행 → 탭 생성+work_items 행+접수 출력 실측, 실제 간단 작업 위임 E2E 1건
  QA scenarios: happy(간단 위임 → 접수 응답 → 완료 영수증) + failure(herdr 부재 시 명확한 오류), Evidence .omo/evidence/omomeow-gateway-migration/delegate/
  Commit: Y | feat(delegate): omomeow-delegate CLI for handoff tabs + work_items registration
- [x] 10. gateway 설정: .env(DISCORD_BOT_TOKENS=오모냥 토큰 — agent-messenger 크리덴셜에서 이관, OMON_DEFAULT_MODEL=inferhub/cb/deepseek-v4.1-flash, OMON_BUTTON_ACTIONS_FILE=~/.omon/button-actions.json, 레인 포트/타임아웃) + bot_profiles에 오모냉 프로필 생성(OMOMEOW-MODE·AGENTS.md 핵심 행동 규칙을 시스템 프롬프트로 포트)
  What to do / Must NOT do: 전역 omo 설정 무손대. 토큰은 파일에 평문 기록 시 퍼미션 600
  Closes: IS-1, IS-4
  Parallelization: Wave 3 | Blocked by: 1 | Blocks: 13,14,15
  References: omo_config.rs:139-238(env 키 전체), 오모냥 .env/credentials(인벤토리), OMOMEOW-MODE.md/AGENTS.md
  Recommended task executor category: unspecified-low
  Acceptance criteria: 게이트웨이 기동 시 프로필 로드 로그 + env 키 전부 인식 실측
  QA scenarios: happy(기동 → 오모냉 프로필 적용 확인) + failure(토큰 부재 시 명확한 부팅 에러), Evidence .omo/evidence/omomeow-gateway-migration/config/
  Commit: Y | chore(config): gateway .env and OmOMeow bot profile
- [x] 11. 크론 시드 임포트: watch.ts JOBS 10종 + 알림 4종(calendar 5분/mail 5분/govsupport 08:00/release 4회) + legacy-crons.json → cron_jobs 시드(스크립트/SQL). brief_file·workdir·deliver(스레드는 origin)·timezone 태깅·ack_command(katok ack 패턴) 전부 반영. 섀도우 검증 전까지 enabled=false로 임포트
  What to do / Must NOT do: 잡 로직 재작성 금지(기존 스크립트/브리프 파일 경로 유지). enabled=true 전환은 12단계에서
  Closes: GAP-2
  Parallelization: Wave 3 | Blocked by: 5,6 | Blocks: 13
  References: ultrabrain 계약 2(마이그레이션 매핑 표), watch.ts JOBS(인벤토리), legacy-crons.json
  Recommended task executor category: deep-low
  Acceptance criteria: 임포트 실행 → cron_jobs 14+행 검증 쿼리(스케줄/행선지/timezone) 실측
  QA scenarios: happy(전 잡 임포트) + failure(계산 불가 식 거절 확인), Evidence .omo/evidence/omomeow-gateway-migration/cron-seed/
  Commit: Y | feat(cron): seed importer for omomeow jobs and legacy crons
- [x] 12. 섀도우 봇 세팅 가이드: Discord Developer Portal에서 테스트 봇 생성 절차, 토큰을 gateway 섀도우 .env에 구성, 테스트 채널 초대 절차 문서화(.omo/evidence/omomeow-gateway-migration/shadow-bot-setup.md)
  What to do / Must NOT do: 섀도우 봇은 테스트 채널 한정. 프로덕션 채널 초대 금지
  Closes: IS-5
  Parallelization: Wave 3 | Blocked by: — | Blocks: 14
  References: 디스코드 Developer Portal(외부), 오모냥 .env 구조
  Recommended task executor category: unspecified-low
  Acceptance criteria: 가이드 문서 존재 + 섀도우 봇 토큰으로 게이트웨이 기동 성공 로그 (오너가 봇 생성 병행)
  QA scenarios: happy(섀도우 봇 게이트웨이 접속) + failure(토큰 오류 시 명확한 에러), Evidence .omo/evidence/omomeow-gateway-migration/shadow-bot/
  Commit: Y | docs(shadow): shadow bot setup guide
- [x] 13. Phase 1 크론 컷오버: gateway 크론 스케줄러만 기동(Discord ingress 비활성) → 시드 잡 2~3종 pilot enabled → 실측 발행 확인 → 잔여 enabled 전환 → watch.ts 해당 JOBS 비활성화. 롤백: watch.ts 재활성화+gateway 크론 disabled
  What to do / Must NOT do: 인바운드 전환은 하지 않음(13단계). 기존 발행 형식 유지 검증
  Closes: GAP-2, IS-2
  Parallelization: Wave 4 | Blocked by: 4,11 | Blocks: 14,15
  References: architect 컷오버 Phase 1, cron/scheduler.rs
  Recommended task executor category: unspecified-low
  Acceptance criteria: pilot 잡 1회 이상 실제 Discord 발행 실측 + watch.ts JOBS 비활성 확인
  QA scenarios: happy(pilot 잡 발행) + failure(실패 시 failure_deliver+백오프 확인), Evidence .omo/evidence/omomeow-gateway-migration/phase1/
  Commit: Y | chore(cron): phase 1 cron cutover
- [x] 14. Phase 2 섀도우 검증: 섀도우 봇으로 테스트 채널에서 👀 즉시 반응/3초 내 첫 응답/버튼 3종/음성 전사/스레드 아카이브-재개/SLA 에스컬레이션 전부 실측
  What to do / Must NOT do: 프로덕션 채널 무관. 실패 항목은 수정 후 재검증
  Closes: IS-1, IS-3 (섀도우 사전 검증)
  Parallelization: Wave 4 | Blocked by: 2,3,5,7,12,13 | Blocks: 15
  References: architect 컷오버 Phase 2, 본 플랜 QA 시나리오 전체
  Recommended task executor category: unspecified-low
  Acceptance criteria: 검증 체크리스트 전 항목 PASS 실측 기록
  QA scenarios: 전 시나리오 실측, Evidence .omo/evidence/omomeow-gateway-migration/phase2/
  Commit: N (검증 보고)
- [x] 15. Phase 3 인바운드 컷오버: ①pkill listen.ts 루프 ②herdr w8:p1 모니터 해제(세션 유지) ③gateway에 오모냉 정식 토큰 주입·기동 ④오너 DM/#work에서 👀+3초 첫 응답 실측. 롤백: gateway 정지→listen 루프 재시작 절차 문서화+시험
  What to do / Must NOT do: listen.ts 완전 정지 확인 전 gateway 인바운드 활성화 금지(이중 접속). 구세션 폐기는 15단계로 연기
  Closes: GAP-1, GAP-5, IS-1
  Parallelization: Wave 4 | Blocked by: 10,14 | Blocks: 16
  References: architect 컷오버 Phase 3, 리스크 순위 2위(토큰 충돌)
  Recommended task executor category: unspecified-low
  Acceptance criteria: 실측 — 오너 메시지 → 👀 3초 내 → 첫 응답 수 초 내, listen.ts 프로세스 0개 확인
  QA scenarios: happy(오너 메시지 E2E) + failure(롤백 드릴 1회 수행), Evidence .omo/evidence/omomeow-gateway-migration/phase3/
  Commit: Y | chore(cutover): phase 3 inbound cutover to gateway
- [x] 16. Phase 4 위임 하네스 연동+구세션 폐기: 스레드에서 장기 작업 요청 → omomeow-delegate 위임 → 5초 내 접수 응답 → 15분 하트비트 → 완료 영수증 E2E 실측. 이후 w8:p1 세션의 모니터·인바운드 역할 정식 폐기(세션 파일은 보존), herdr-boot.sh는 herdr 서버+작업 탭 계층만 유지
  What to do / Must NOT do: 구세션 트랜스크립트 삭제 금지(보존). work-map.json과 work_items 정합성 확인
  Closes: IS-2, IS-5
  Parallelization: Wave 4 | Blocked by: 9,15 | Blocks: —
  References: architect 컷오버 Phase 4, 자문 4(A) 핸드오프, work-map.json 스키마
  Recommended task executor category: unspecified-low
  Acceptance criteria: 위임 E2E 실측(접수→하트비트→영수증) + 구세션 모니터 0개 확인
  QA scenarios: happy(위임 E2E) + failure(워커 좀비 → 하트비트 경고), Evidence .omo/evidence/omomeow-gateway-migration/phase4/
  Commit: Y | chore(cutover): phase 4 delegation harness integration, retire god session

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [x] F1. 크론 계약 감사 — cron_jobs 14종(JOBS 10+알림 4)의 스케줄·행선지·timezone·ack_command 전수 대조, pilot 실제 발행 로그 대조
  Recommended task executor category: unspecified-high
- [x] F2. 코드 품질 리뷰 — cargo fmt --check + clippy -D warnings + cargo check + 전체 유지 테스트(개별 --test 실행) 전부 exit 0, 전체 diff 셀프 리뷰
  Recommended task executor category: unspecified-high
- [x] F3. 실제 manual QA — Phase 2/3 실측 증거 재확인: 👀 즉시 반응, 3초~수 초 첫 응답, 버튼 3종(ack/dedup/nonterminal), 음성 전사, 스레드 아카이브-재개, SLA 에스컬레이션 1회, 15분 하트비트 1회
  Recommended task executor category: unspecified-high
- [x] F4. 이상적 상태 정합성 — IS-1~5 대응 1:1 검증(GAP-1~5가 닫혔는지), 31개 기능 커버리지 매트릭스 대조(silent drop 0건)
  Recommended task executor category: unspecified-high
- [x] F5. 롤백 드릴 — listen.ts 복구 경로(게이트웨이 정지→루프 재시작→오모냥 응답 실측) 1회 수행 + WIP 복원 절차(stash pop) 문서 확인
  Recommended task executor category: unspecified-high

## Commit strategy
- 투두 단위 원자적 커밋(검증 그린 시점), Conventional Commits(로컬 관측 관례: feat/fix(chore)(scope): 영어 명령형 — git log 관례 일치)
- 소유자 WIP는 커밋에 포함하지 않음(stash 보존, 0번 항목)
- 최종 커밋 푸터: Plan: .omo/plans/omomeow-gateway-migration.md

## Success criteria
> One row per IS row. The plan is complete only when every IS row has a delivering todo and a proving QA scenario; F4 checks the delivered behavior against these rows 1:1, and a shortfall becomes new `- [ ] N.` rows, never a note.
| IS | Delivering todo(s) | Proving QA scenario | Evidence |
| --- | --- | --- | --- |
| IS-1 | 5,7,10,15 | Phase 2/3 섀도우+본선 E2E(👀 3초, 첫 응답 수 초) | .omo/evidence/omomeow-gateway-migration/phase3/ |
| IS-2 | 6,11,13 | Phase 1 pilot 잡 발행 + ack 실측 | .omo/evidence/omomeow-gateway-migration/phase1/ |
| IS-3 | 3,4,8,16 | SLA 에스컬레이션 1회 + 15분 하트비트 1회 실측 | .omo/evidence/omomeow-gateway-migration/sla/, heartbeat/ |
| IS-4 | 3,10 | OMON_DEFAULT_MODEL=deepseek 기동 로그 + 전 턴 deepseek 사용 실측 | .omo/evidence/omomeow-gateway-migration/config/ |
| IS-5 | 9,16 | 위임 E2E(접수 5초 내→하트비트→영수증) + 커버리지 매트릭스 대조 | .omo/evidence/omomeow-gateway-migration/phase4/ |

B6 FIXED src/report/claims.rs:109 — 기존 후속 구현 보존: 닫히지 않은 마커는 [[를 보존하고 스캔을 계속하며, 파싱 실패한 닫힌 마커도 원문을 보존함; test unclosed_measure_marker_keeps_rest_of_reply
B7 FIXED src/context/pack.rs:192 — 기존 후속 구현 보존: migrations/0038_lane_lifecycle.sql:24의 lane_owned_worktrees에서 registered 행을 조회하며, 오류를 warn 기록 후 반환함; test precedent_lookup_queries_lane_owned_worktrees
B8 FIXED src/report/status.rs:39 — 기존 후속 구현 보존: 600초 TTL, 기록 시 전체 만료 항목 정리, 조회 시 만료 항목 삭제, StatusDriver 시작/종료 정리 및 src/multiplexer/actor.rs:806의 드라이버 유무와 무관한 턴 종료 정리 확인; test tool_activity_clears_on_turn_end_and_bounds_ttl
B9 FIXED src/report/claims.rs:504 — 기존 후속 구현 보존: 나이 조건 없이 모든 pending을 unmeasurable로 변경하며 src/main.rs:1389에서 시작 복구로 호출함; test recover_stale_report_claims_sweeps_young_pending_claims_at_startup
C3a FIXED src/context/pack.rs:113 — 기존 후속 구현 보존: next_cursor를 따라 짧은 페이지도 계속 읽고 부모 메시지를 중복 제거함; 250개 답글 HTTP 테스트가 200개 이후 최신 답글과 두 번째 요청 cursor를 검증함; 기존 20페이지 및 pack 전체 5초 제한 내 조회이며 임의 길이 스레드의 끝까지 보장하지 않음; test thread_reader_walks_every_page_by_cursor_without_repeating_the_parent

재개 결과: 소스와 기존 회귀 테스트를 재작성하지 않고 현재 구현, 호출부, 마이그레이션 및 테스트 본문을 정적으로 확인했다. 이 재개 작업의 수정은 본 증거 보고서의 줄 번호, 실제 테스트명 및 제한 사항 갱신뿐이다. 빌드, 테스트, 커밋, 푸시, 배포, 실행 중 게이트웨이 변경 및 외부 발행/일정 작업은 실행하지 않았다. 실행 검증은 소유자의 통합 게이트에 남아 있으며, 이 작업을 막는 외부 의존성은 없다.

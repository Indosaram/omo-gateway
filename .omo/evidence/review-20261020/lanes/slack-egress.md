# slack-egress lane - 2026-10-20 (KST), second pass

Files I edited (all inside my write scope): src/slack/commands.rs, src/slack/egress.rs, src/slack/mrkdwn.rs, src/tools/message_context_slack.rs, and - per the lead's explicit scope expansion for C3 - src/context/pack.rs plus the cursor-aware read method in src/slack/api.rs. Evidence: the four r1 lane reports, re-reads of the current files at 5fcfc08 + the uncommitted tree, and simulation of every new test's fake in Python. Earlier workers' partial fixes were preserved and completed; I removed nothing they wrote except where it could not work (see C1 and C3b).

## C2 - the parent-ts rule (corrected)

Slack `conversations.replies` requires the thread *root* ts. A reply's own ts is not a valid root: Slack answers `thread_not_found` (or, with `limit=1`, returns just the parent). My earlier pass wrongly used the requested reply ts as the root; that is fixed.

C2 FIXED src/tools/message_context_slack.rs:405 (root resolution at :312) — `get_message` resolves the thread root before reading: `conversation.thread_ts` when the conversation is already a thread (the caller's thread session), otherwise `thread_root(ts)`, which reads the reply's `thread_id` (falling back to `metadata_json.$.referenced_message_id`) from the indexed document that recorded it; only a ts with no recorded thread - which is exactly a top-level message - is used as its own root. The read then pages to `ts` and keeps the exact-ts match. A reply whose root is still unknown now fails with Slack's own error instead of a misleading "not found"; the not-found message names the thread and points at `operation: "replies"`. test get_message_rejects_a_reply_ts_as_a_thread_root (the fake rejects a non-root ts with `thread_not_found`, modelling the real API rather than an imaginary tolerant one), thread_reads_page_to_newest_and_get_message_finds_replies (indexes the thread via `replies`, then fetches reply 230 from a channel session).

## C3b - thread pagination, provider and context pack

Both halves now page with Slack's real continuation signal instead of inferring from page length. The root-inclusion and timestamp-precision hazards are handled explicitly:

- Root inclusion: `conversations.replies` repeats the thread parent as the first message of *every* page, so both walks dedupe by ts (provider `seen`, pack `ReplyWalk::seen`) instead of counting messages. Without that, a parent repeated 5 times would look like 5 fresh messages and could keep a walk alive past the real end.
- Precision: pack's walk orders by `ts_parts` (u64 seconds + 6-digit micros). Ordering by `f64` collapses replies in the same second at these magnitudes (1.7e9 with 1e-6 resolution needs 16 significant digits), which would stop the walk early or, worse, make a page look unchanged. The provider already used `ts_parts`.
- No loops: a cursor that repeats itself ends the walk; the end of a cursor series ends the walk; the `oldest` fallback (for a peer that sends no cursor) ends as soon as a page adds no new ts; and both walks are bounded (`MAX_THREAD_PAGES = 50`, `MAX_REPLIES_PAGES = 20`).

C3 FIXED src/context/pack.rs:52 and src/tools/message_context_slack.rs:248 — thread reads no longer stop at the oldest 200 messages: both readers page forward on Slack's `response_metadata.next_cursor` (shared method `SlackApi::conversations_replies_page`, src/slack/api.rs:220), dedupe the thread parent that Slack repeats on every page, order by exact microsecond ts, end on a repeated cursor / end of cursor series / a page with nothing new, and are page-bounded (`MAX_THREAD_PAGES = 50`, `MAX_REPLIES_PAGES = 20`); `oldest` is only sent on the first request. test src/context/pack.rs thread_reader_walks_every_page_by_cursor_without_repeating_the_parent, paginate_thread_replies_follows_the_cursor_and_dedupes_the_parent, paginate_thread_replies_without_a_cursor_advances_by_ts_and_cannot_loop; src/tools/message_context_slack.rs thread_reads_follow_slacks_cursor_past_short_pages, thread_reads_page_to_newest_and_get_message_finds_replies. UNRUN: no `cargo check`/`cargo test` from this lane (shared target lock, then the lead's instruction to launch no builds); the lead's integrated run owns execution. Details split by file are in the C3b line below and need no separate action.

C3b FIXED src/tools/message_context_slack.rs:248 (context half: src/context/pack.rs:52, walk at :80, reader at :114; transport method: src/slack/api.rs:220) — both readers now page on Slack's `response_metadata.next_cursor` instead of inferring from page length. Provider: `thread_messages` continues while the cursor is present and different from the one just used, sends `oldest` only on the first request (a cursor already encodes the position), and still stops at `until` so `get_message` reads only as far as it must. Context pack: the `chunk.len() < REPLIES_FETCH_LIMIT` stop is replaced by `ReplyWalk`, which is cursor-driven, parent-deduped, microsecond-exact, loop-proof and page-bounded (`MAX_REPLIES_PAGES`). New shared method `SlackApi::conversations_replies_page(channel, thread_ts, cursor, oldest, limit) -> RepliesPage { messages, next_cursor }` (+ the `RepliesPage` type at api.rs:33); `conversations_replies` is now a thin wrapper over it, so every existing caller is unchanged. test provider thread_reads_follow_slacks_cursor_past_short_pages (5 pages of 60 replies for a `limit=200` request; asserts `at-60`/`at-240` and that recent returns 250/249), thread_reads_page_to_newest_and_get_message_finds_replies; pack thread_reader_walks_every_page_by_cursor_without_repeating_the_parent (real `SlackApi` against a loopback fake; 251 unique messages and request 2 carrying `cursor: "at-60"` - the old length break returns 61 and stops), paginate_thread_replies_follows_the_cursor_and_dedupes_the_parent, paginate_thread_replies_without_a_cursor_advances_by_ts_and_cannot_loop.

api.rs ownership note for the lead: the transport lane has meanwhile added `conversations_history_page`/`HistoryPage` to the same file (channel side); the two additions are independent. My hunks in api.rs are only `RepliesPage`, `conversations_replies_page`, and the `conversations_replies` delegation.

NOT DONE (deliberately, not mine): src/slack/backfill.rs still walks threads with `oldest` and treats a short page as the end (its own lane's file, and its `MAX_PAGES` bound is the transport lane's design). With `conversations_replies_page` available, that lane can switch to `next_cursor` the same way.

## C12 - indexed metadata truthfulness

C12 FIXED src/tools/message_context_slack.rs:187 (used at :151 metadata, :366 index) — the team is now a property of the conversation being read, never of the reading session: `conversation_guild` returns `None` only for a real `D…` conversation, otherwise the session team, and when the session key has no team (owner-DM, cron and lane keys can all name a channel) it recovers the channel's team from `sessions`, then from already-indexed documents for that channel, then from `auth_test().team_id` (resolved once per provider). `document()` no longer receives the session at all, so a future caller cannot reintroduce the leak, and `metadata()` uses the same resolution so the reported `guild_id` and the stored rows cannot disagree. test channel_reads_from_teamless_sessions_record_the_channel_team (team-less session reading channel C0WORK with a stored session row → rows carry `T1`; the DM read through its own DM session stays `NULL`).

Honest correction to the finding: the r1 trigger ("DM session reads a cross-channel target") is unreachable in the current code - `authorize` refuses it before any read: `if session.guild_id.is_none() { return Err(denied("cross-channel reads are not allowed from direct-message sessions")) }` (src/tools/message_context_slack.rs, `authorize`). So the defect as written could not fire; the reachable variant is a team-less session key reading a channel it is allowed to read, which did store `guild_id NULL` and was classified `dm` by src/inbox/sources/slack.rs:48. That variant is fixed, and the DM path is now asserted to stay `NULL` on purpose.

## Earlier items (unchanged in this pass)

C1 FIXED src/slack/commands.rs:1160 (helper :498, const :36) — `/omo-title` no longer depends on a `thread_ts` Slack never sends: `/omo-title <thread link or timestamp> <title>`, parsed by `explicit_thread` (bare ts, or a channel permalink with `?query`/`#frag`/`<link|label>` tolerated and the channel id enforced), and without either the command replies `TITLE_NO_THREAD` naming the syntax. The partial version could not compile (`url::Url`; `url` is not a dependency, only `urlencoding`), so the permalink is parsed by hand. test explicit_thread_accepts_ts_and_permalinks, title_without_thread_ts_names_the_thread_explicitly.

C9 FIXED src/slack/egress.rs:146 — `mark_approval_decided` builds the section block as mrkdwn with the gateway-built label verbatim instead of `text_blocks`/`escape`, so `<@U1>` renders as a mention. test mark_approval_decided_replaces_buttons_with_label (extended to assert the block text).

C10 FIXED src/slack/egress.rs:623 — the approval fallback `text` escapes the agent reason (`escape(&truncate_chars(reason, APPROVAL_REASON_LIMIT))`), matching `approval_blocks`, so `<!channel>`/`<!here>` cannot broadcast from the notification copy; `post()` forwards text unchanged, so no double escaping. test approval_fallback_text_escapes_agent_reason.

C11 FIXED src/slack/mrkdwn.rs:106 — `bound_chunks` backs the byte cut off a trailing `&` that would leave half of `&amp;`/`&lt;`/`&gt;` before the truncation notice. test bound_chunks_never_splits_an_html_entity.

## Verification - what I ran, and what I did not

Run: `rustfmt --edition 2021 --check` on all six edited files - parses clean, no syntax errors (this is a parse check, not a typecheck). Run: a Python re-implementation of each new fake and of both walks, confirming the expected page counts (provider 5 pages / 251 messages / cursors at-60..at-240; pack 5 pages / 251 unique messages; `get_message` 4 pages to reply 230; entity budget 25 bytes with offsets 1..5 landing inside `&amp;`).

NOT run: no `cargo check`/`cargo test` - the lead owns the single integrated run, and my earlier attempt only hit the shared target lock ("Blocking waiting for file lock on build directory"). I launched no further build, no sleep/poll loop, no nohup, and killed no other lane's process. So every claim above rests on reading the current code plus those simulations; treat compile and test status as unverified until the integrated run.

Compile risks I checked by reading: `tokio::sync::OnceCell` (tokio "full"), `MessageSearchIndex::pool()` exists, `MessageContextResult: Debug` so `unwrap_err` is fine, `Option::is_none_or`/`Option::filter` need >= 1.82 (repo pins 1.88), `HashSet` imported in both edited modules, `parking_lot`/`axum` available to pack's tests, and the fake now only defaults `ok: true` when a responder omits it (so a strict fake can fail a call on purpose).

Tests to run in the integrated pass: src/slack/commands.rs `explicit_thread_accepts_ts_and_permalinks`, `title_without_thread_ts_names_the_thread_explicitly`; src/slack/egress.rs `mark_approval_decided_replaces_buttons_with_label`, `approval_fallback_text_escapes_agent_reason`; src/slack/mrkdwn.rs `bound_chunks_never_splits_an_html_entity`; src/tools/message_context_slack.rs `thread_reads_page_to_newest_and_get_message_finds_replies`, `thread_reads_follow_slacks_cursor_past_short_pages`, `get_message_rejects_a_reply_ts_as_a_thread_root`, `channel_reads_from_teamless_sessions_record_the_channel_team`, `replies_and_thread_session_read_conversations_replies_without_root` (pre-existing, now makes one extra request because the no-cursor fallback walks once more); src/context/pack.rs `thread_reader_walks_every_page_by_cursor_without_repeating_the_parent`, `paginate_thread_replies_follows_the_cursor_and_dedupes_the_parent`, `paginate_thread_replies_without_a_cursor_advances_by_ts_and_cannot_loop`.

## Formatting pass (gate review-20261020-integrated1)

The gate's `fmt.log` (`.omo/evidence/review-20261019/gate/review-20261020-integrated1/fmt.log`) reported rustfmt diffs in my three files. I applied exactly those hunks, with `apply_patch` per file and no repo-wide formatter run:

- src/context/pack.rs:77 - collapsed the `let ... else` binding of `ts` onto one line.
- src/context/pack.rs:763 - expanded the second `paginate_replies` call in `paginate_thread_replies_follows_the_cursor_and_dedupes_the_parent` to one argument per line.
- src/slack/commands.rs:514 - split the `seconds.is_empty() || fraction.is_empty() || !seconds.bytes()...` guard in `explicit_thread` into rustfmt's three-condition layout.
- src/slack/commands.rs:2202 - expanded the `explicit_thread("...C2/p1700000000000100 X", "C1")` call in `explicit_thread_accepts_ts_and_permalinks`.
- src/tools/message_context_slack.rs:415 - collapsed the `thread_root(ts).await.unwrap_or_else(...)` fallback in `get_message` onto one line.
- src/tools/message_context_slack.rs:1235 - wrapped the `teamless` `SessionKey::new` binding.
- src/tools/message_context_slack.rs:1253 - expanded the `dm` `SessionKey::new` call and chained `.query(...).await.unwrap()`.

No token, no argument, no test assertion changed - layout only. `rustfmt --edition 2021 --check` now reports no diff for all three files (run per file, not repo-wide).

Formatting-only scope for the discord worker: the same fmt.log also lists src/discord/adapter.rs:6665 and src/discord/buttons.rs:185/403/638/847/869, which are that worker's files - formatting-only, and I did not touch them. The other entries in the log (src/agent/*, src/lanes/*, src/report/*, src/sessions/*, src/slack/backfill.rs, src/slack/ingress.rs, src/tracker/*) belong to their own lanes and are likewise untouched by me.

Still no build or test run from me: the gate is still collecting compiler errors, so `cargo check`/`test` status for these files remains unverified until the integrated run.

## Gate test failure fixed: precedent_lookup_queries_lane_owned_worktrees

The gate's test.log (`.omo/evidence/review-20261019/gate/review-20261020-integrated1/test.log:1139`) shows the exact panic: `src/context/pack.rs:829: called Result::unwrap() on an Err value: Database(SqliteError { code: 787, message: "FOREIGN KEY constraint failed" })`. Cause: `lane_owned_worktrees.lane_key` has `REFERENCES lanes(key)` (migrations/0038_lane_lifecycle.sql:26), and the test inserted the owned worktree without ever creating lane `test-lane`.

Fix: the test now inserts that lane first - `INSERT INTO lanes (key, title, lead_session_key, cwd, brief_path, session_key, status, done_signals, last_activity_at)` with `status = 'working'` and `CURRENT_TIMESTAMP` - before the worktree row. Foreign keys stay enabled (no pragma change, no `PRAGMA foreign_keys=OFF`); the fixture is simply valid now. No assertion, no query under test and no production code changed: `LedgerPrecedents::precedents` and `precedent_matches_targets` are untouched, so the test still proves the lane-owned-worktree lookup feeds `my-repo` into the precedent line.

Status: UNRUN by me - no `cargo test` while the gate holds the target directory. The gate's rerun is the verification.

## Gate2 clippy lints in my files

Both lints from `.omo/evidence/review-20261019/gate/review-20261020-integrated2/clippy.log` are fixed at the diagnostic's own line, with no suppression:

- src/slack/commands.rs:504 `manual_pattern_char_comparison` - `path.split(|c| c == '?' || c == '#')` -> `path.split(['?', '#'])`, exactly the array-of-char pattern the diagnostic suggests.
- src/context/pack.rs:667 `redundant_closure` - `.map(|n| json_ts(n))` -> `.map(json_ts)` in the pack test fake.

No `#[allow(...)]` was added to either file (`grep -n "allow(clippy" src/slack/commands.rs src/context/pack.rs` = none), and `rustfmt --edition 2021 --check` is clean for both. UNRUN: no `cargo clippy`/`cargo test` from this lane; the gate's rerun verifies.

The third error in that log (`items_after_test_module`, src/inbox/sources/mail.rs:220) and the two `type_complexity` errors (src/autonomy/loop.rs:978/1013) are other lanes' files - untouched by me.

## Replacement worker read-only source confirmation (2026-10-20)

WORKING: slack-egress - successor implementation confirmation complete.

The replacement session re-read the four original r1 findings reports and the current implementation and regression tests in all four scoped source files. All seven requested ID status lines above already exist exactly once; no source rewrite was needed. This session changed only this report, preserving all successor fixes and unrelated report history. The earlier sections' build/format/simulation claims belong to earlier workers, not this session: no builds, tests, formatter, external actions, commits, pushes, or deployments were run here.

Current source anchors: title targeting/guidance at `src/slack/commands.rs:1164`, no-thread success test at `:2219`, missing-target guidance test at `:2143`; indexed/thread-session reply root resolution at `src/tools/message_context_slack.rs:405`, exact reply 230 regression at `:1063`; cursor pagination at `:248`, short-page cursor regression at `:1098`; channel team resolution at `:187` and indexing at `:368`, channel/DM classification regression at `:1221`; approval decision mrkdwn at `src/slack/egress.rs:146`, regression at `:1207`; escaped approval fallback at `:622`, regression at `:1242`; entity-safe byte boundary at `src/slack/mrkdwn.rs:106`, regression at `:485`.

Remaining limitations, not new regressions: a channel-level reply lookup whose parent has never been indexed cannot discover its root from a reply ts alone; the existing strict-fake test `get_message_rejects_a_reply_ts_as_a_thread_root` (`src/tools/message_context_slack.rs:1129`) documents that case. The original reported trigger (reply ID obtained from a replies/thread recent result) records or already knows that parent. Thread reads remain intentionally bounded by `MAX_THREAD_PAGES`. Compilation, test execution, and live Slack behavior remain unverified by this replacement session and are left to the lead's integrated checks.

Restart routing evidence: the unqualified `gpt-6.1-sol` pin was ambiguous; retrying with `mahoquot/gpt-6.1-sol` reached the subagent nesting limit (`max_depth 1`). This replacement session therefore performed the source confirmation itself without spawning another worker or touching active QA/card-render/evidence work.

## 직접 재시도 확인 결과

이번 재시도는 자식 생성 없이 직접 수행했다. 요청된 네 소스 파일의 해당 구현과 회귀 테스트, 원래 r1 결함 보고서 네 개를 읽고 대조했다. 원래 재현 조건에 대한 미완료 수정은 발견하지 못했으므로 기존 소스는 변경하지 않았다. 이번에 수정한 경로는 이 보고서뿐이다.

- 슬래시 명령: `commands.rs:1164`에서 명시적 스레드 대상이 없으면 비공개 안내를 반환한다. `title_renames_bot_thread_root_and_explains_otherwise`와 `title_without_thread_ts_names_the_thread_explicitly`가 실제로 `thread_ts` 없는 페이로드를 사용한다.
- 답글 조회와 페이지 처리: `message_context_slack.rs:405`에서 현재 스레드 또는 색인된 부모를 찾고, `:248`에서 커서를 따라 읽는다. `thread_reads_page_to_newest_and_get_message_finds_replies`는 답글 230 조회 및 최신 250/249 반환을 검사하고, `thread_reads_follow_slacks_cursor_past_short_pages`는 짧은 페이지 다섯 개의 커서를 검사한다. 공유 `conversations_replies_page`의 실제 시그니처와 커서 반환 구현도 `src/slack/api.rs:220`에서 확인했다. 이 파일은 수정하지 않았다.
- 채널 분류: `message_context_slack.rs:187`의 대화별 팀 결정이 `index`에 연결되어 있고, `channel_reads_from_teamless_sessions_record_the_channel_team`는 채널 색인의 `T1`과 실제 DM의 `NULL`을 구별한다. 원래 보고서의 DM 교차 채널 요청은 현재 `authorize`에서 거절된다.
- 승인 출력: `egress.rs:146`은 신뢰된 결정 라벨을 mrkdwn에 그대로 넣고, `:622`는 에이전트 사유를 escape한다. 각각 `mark_approval_decided_replaces_buttons_with_label`, `approval_fallback_text_escapes_agent_reason`가 전송 JSON을 검사한다.
- 청크 경계: `mrkdwn.rs:106`은 잘릴 엔티티의 `&` 앞으로 경계를 옮긴다. `bound_chunks_never_splits_an_html_entity`가 `&amp;` 내부의 네 절단 위치를 검사한다.

보고서의 요청 ID 상태 행은 읽기 전용 grep count 결과 7개이며, 각 ID가 한 번씩 존재한다. 기존 상태 행과 이전 작업 이력은 보존했다. 남은 제한은 부모를 전혀 모르는 채널 단위 답글 ID 조회와 스레드 페이지 상한이며, 전자는 `get_message_rejects_a_reply_ts_as_a_thread_root`에 명시되어 있다. 원래 결함의 재현 조건은 이미 읽은 replies/thread recent 결과의 ID이므로 해당 부모를 알고 있다.

이번 재시도에서 빌드, 테스트, 포매터, 커밋, 푸시, 배포, 외부 발행 또는 캘린더 작업은 실행하지 않았다. 따라서 컴파일·테스트 통과 및 실제 Slack 동작은 이번 확인으로 증명하지 않았으며, 앞선 작업자의 실행 주장을 이번 실행 결과로 취급하지 않는다. 활성 QA/card-render/evidence 워커, 실행 중 게이트웨이 및 엔진 설치본은 건드리지 않았다.

# Todo 23 Evidence Gate Review - Read-Only Inbox Sources

recommendation: APPROVE
verdict: CONFIRMED_SCOPE23_BOUNDED (All Todo 23 inbox source adapters, 4-source live fetch proof, authentic capture lineage, read-only permissions, failure isolation invariants, and exact 26 passed / 0 failed / 4 ignored test execution results in test_inbox_sources are independently confirmed; broader multi-target batch exit 0 remains pending unrelated test_omo_backend baseline resolution)
task: todo 23 (Implement the four read-only inbox sources)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md
worktree: /Users/indo/code/project/omon-gateway-wt/dori-work
reference_receipt: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task23-fixture-wiring-receipt.md
live_proof: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task23.json
focused_log: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/durability-fixtures-20261011T180128Z-focused.log

blockers: []

---

## Gate Verdict & Status Summary

Todo 23 is **CONFIRMED** under its bounded scope and all acceptance criteria. The previous execution receipt blocker is resolved by the raw durable runner log (`durability-fixtures-20261011T180128Z-focused.log`), which records that `tests/test_inbox_sources.rs` completed with **26 passed, 0 failed, 4 ignored** in 0.15s.

All functional requirements, privacy boundaries, schema constraints, read-only query semantics, failure isolation, and fixture provenance are independently verified against authentic on-disk artifacts and frozen sources.

### Broader Batch Execution Status (Documented Without Inventing Global Exit 0)
The combined runner `durability-fixtures-20261011T180128Z` invoked:
```bash
cargo test --offline -j2 --all-features \
  --test test_lane_dispatch \
  --test test_inbox_sources \
  --test test_lane_events \
  --test test_omo_backend \
  --test test_multiplexer \
  --no-fail-fast -- --nocapture
```
From `durability-fixtures-20261011T180128Z-focused.log`:
- **`tests/test_inbox_sources.rs`**: **`test result: ok. 26 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 0.15s`** (Scope 23 Target: **CLEAN PASS**).
- **`tests/test_lane_dispatch.rs`**: `test result: ok. 10 passed; 0 failed; 0 ignored; finished in 0.86s` (Target: **PASS**).
- **`tests/test_lane_events.rs`**: `test result: ok. 25 passed; 0 failed; 1 ignored; finished in 1.57s` (Target: **PASS**).
- **`tests/test_multiplexer.rs`**: `test result: ok. 19 passed; 0 failed; 0 ignored; finished in 1.67s` (Target: **PASS**).
- **`tests/test_omo_backend.rs`**: Active / running. Exhibits 8 known baseline failures and 2 long-running tests (>60s), as documented in prior baseline reviews:
  1. `test_omo_backend_accepts_received_completion_at_deadline_edge` (deadline-edge LLM empty content)
  2. `test_omo_backend_emits_hermes_activity_lines` (final chunk layout assertion)
  3. `test_omo_backend_omits_workspace_when_per_agent_disabled` (upstream LLM failure)
  4. `test_omo_backend_provisions_agent_workspace_and_passes_to_thread_start` (upstream LLM failure)
  5. `test_omo_backend_waits_for_final_message_after_tool_activity` (dispatcher stream final chunk)
  6. `test_omo_backend_requires_turn_terminal_even_after_final_agent_message` (chunks final flag)
  7. `test_omo_backend_aborts_turn_on_repeated_approval_denials` (timeout during turn streaming)
  8. `test_omo_backend_deadline_interrupts_looping_turn` (deadline timing jitter: 10.05s vs 10s)
  Pending long tests: `test_omo_backend_e2e_thread_lifecycle_and_streaming`, `test_omo_backend_retries_once_after_connection_drop`.

Because the multi-test cargo process contains `test_omo_backend.rs`, the whole batch command has not exited 0. However, **Todo 23 is fully decoupled from `test_omo_backend`**, and `test_inbox_sources.rs` passed unconditionally with zero errors.

---

## Detailed Requirement & Gate Accounting

### 1. Plan 23 Acceptance Criteria & Source Audit

| Component | Plan Specification | Implemented Code & Integrity Check | Status |
|---|---|---|---|
| **Mail Source** (`src/inbox/sources/mail.rs`) | `zele mail list --json` for `freedomzero91@gmail.com`, unread inbox with categories excluded (`-category:promotions -category:social -category:updates`), query `after:` cursor; never call `mail read`; parse snippet from list; link to web Gmail. | Implemented in `MailSource::args` and `MailSource::parse`. Subprocess call via `run_json("zele", ...)`. Extracts `thread_id`, `snippet`, `internal_date`/`date`, and categories. Zero body fetches; link format `https://mail.google.com/mail/u/0/#all/{thread}`. | **PASS** |
| **Kakao Source** (`src/inbox/sources/kakao.rs`) | PirateTalk HTTP endpoint, GET `/api/rooms`, per-room messages with `before_id`, cursor map `{"<chatId>":"<lastLogId>"}`, numeric `time` as Unix seconds. | Implemented in `KakaoSource`. `parse_messages` accepts numeric `time` (integral Unix seconds), converts via `DateTime::<Utc>::from_timestamp`, enforces pagination ordering and floor filtering. Handles per-room cursors serialized to JSON map. | **PASS** |
| **iMessage Source** (`src/inbox/sources/imessage.rs`) | `imsg chats --limit 50 --json` + `imsg history --chat-id <id> --limit 50 --json`, skips `is_from_me`, parses NDJSON or JSON array, cursor is RFC3339 timestamp. | Implemented in `IMessageSource`. Calls `imsg` read-only subcommands via `run_json`. `parse` filters out `is_from_me: true`, decodes single objects or NDJSON streams via `decode_structured_output`, tracks newest message timestamp as cursor. | **PASS** |
| **Discord Source** (`src/inbox/sources/discord.rs`) | Local SQLite query on `message_search_documents` (`WHERE platform='discord' AND message_id > ? AND author_id <> ? AND (guild_id IS NULL OR guild_id = '' OR mentions owner OR reply to owner)`), owner ID `414011632306618368`. | Implemented in `DiscordSource::fetch_since`. Single parameterized `SELECT` query with `calculated_room_type` ('dm', 'owner_reply', 'channel'). Filters out owner's own messages and bot's messages. Computes Discord web link. Read-only query execution. | **PASS** |
| **Failure Isolation** | Kakao failure at `http://127.0.0.1:9` does not impede the other 3 sources. | Verified in `kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor` in `tests/test_inbox_sources.rs:289-410`. Failing Kakao returns error, preserves old cursor in DB, while Mail, iMessage, and Discord commit 1 item each and update cursors. | **PASS** |
| **Live QA Scenarios** | 4 `#[ignore]` tests fetching once and printing counts only (`task23.json`). | Verified in `task23.json`: 4 live tests executed against isolated test binary; exited 0; printed item counts (`mail: 0`, `imessage: 710`, `kakao: 2718`, `discord: 24`). Discord test strictly requires `OMON_QA_DISCORD_COPY_DB` with `read_only(true)`. | **PASS** |

---

### 2. Comprehensive Technical Evaluation

#### A. Provenance: Actual Numeric Time vs Synthetic Previous
- Prior historical capture `161359754Z` had omitted numeric `time` from staged message JSON before raw buffers were zeroed.
- Fresh capture `175050750Z` (`task23-kakao-numeric-20261011T175050750Z`) captured live PirateTalk GET `/api/rooms/{id}/messages?limit=1` returning exact integral Unix seconds (`kakao-messages-numeric.json`, SHA-256 `936b20cd30da29510a1332a2d0a1202e6eb1b13e6a9fcaf9c1eb3302a2f0c65b`).
- This matches the production contract in `/Users/indo/code/project/scripts/kakao-archive.ts` where `Msg.time` is Unix seconds.
- In `src/inbox/sources/kakao.rs:72-88`, numeric `time` is primary; invalid or out-of-range numeric time fails fast rather than guessing, and RFC3339 string aliases (`createdAt`, `timestamp`, `date`) are consulted only when numeric `time` is completely absent.
- Verified in `kakao_parses_authenticated_numeric_seconds_capture` (PASSED).

#### B. Real Empty Mail Legitimacy (Zero Phantom Requirements)
- Mailbox query for `freedomzero91@gmail.com` with `is:unread -category:promotions -category:social -category:updates` returned 0 unread messages in both live fetch and authenticated capture (`task23-capture-20261011T161359754Z/mail-lineage.json`).
- Plan line 413 explicitly allows legitimate unread-empty mailbox state.
- Fabricated populated email data was rightly rejected; `mail_parses_authenticated_empty_capture_without_populated_substitution` validates that `MailSource::parse` handles empty captures cleanly without corrupting the cursor.
- Verified in test suite (PASSED).

#### C. SQL Read-Only Permissions & Prohibited Argv Neutralization
- **Mail**: CLI args built in `MailSource::args` invoke only `mail list --folder inbox --account ... --filter ... --limit 100`. Prohibited subcommands (`mail read`, `send`, `delete`) are completely absent from the codebase.
- **iMessage**: CLI args built in `IMessageSource` invoke only `chats --limit 50 --json` and `history --chat-id ... --limit 50 --json`. Mutating subcommands (`send`) are completely absent.
- **Kakao**: `KakaoSource` uses `reqwest::Client` executing exclusively HTTP `GET` against `/api/rooms` and `/api/rooms/{id}/messages`. No mutating HTTP methods (POST, PUT, DELETE, PATCH) exist.
- **Discord**: `DiscordSource` executes a single parameter-bound `SELECT` statement on `message_search_documents`. Live QA test (`tests/test_inbox_sources.rs:1015-1045`) strictly connects via `SqliteConnectOptions::read_only(true)` and requires `OMON_QA_DISCORD_COPY_DB`.
- Mutating queries (`INSERT`, `UPDATE`, `DELETE`, `DROP`) are forbidden and absent from source adapters.

#### D. Bad Endpoint Isolation (Three-Source Failure Resilience)
- Evaluated in `kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor` (`tests/test_inbox_sources.rs:289-410`):
  - Injected dead endpoint `http://127.0.0.1:9` for Kakao.
  - Collector correctly captures Kakao error message in `inbox_cursors.error`.
  - Kakao cursor in `inbox_cursors` remains strictly at its prior value (`old_cursor`).
  - Mail, iMessage, and Discord each successfully commit their items to `inbox_items` (count = 1 each) and advance their cursors in `inbox_cursors`.
- Verified in test suite (PASSED).

#### E. Discord Historical Snapshot vs External Discord QA
- Live QA requires an offline owned copy at `/tmp/task23-discord-6joAvb/snapshot.db`, verified via `sqlite3 -readonly .backup` with SHA-256 `8b768f856d9aefd8346ff59cd6f444b980ecaa2f489a2f36ee225341096743f6`.
- Verified `mode=ro&immutable=1`, zero WAL/SHM sidecars, `PRAGMA query_only=ON`.
- External Discord API/UI connections are strictly forbidden and never attempted.

---

### 3. Source Freeze & Hash Verification Matrix

All worktree source and fixture hashes in `/Users/indo/code/project/omon-gateway-wt/dori-work` match the settled receipt hashes byte-for-byte:

| Path | Settled Hash | Verified Current Hash | Status |
|---|---|---|---|
| `tests/test_inbox_sources.rs` | `c0dbc0d76dcd16d5ac163d8ea62e53d1ac9cdc91ad10de7339fa9f5a80bd7c2d` | `c0dbc0d76dcd16d5ac163d8ea62e53d1ac9cdc91ad10de7339fa9f5a80bd7c2d` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/discord-indexed-row.json` | `a24c33f29caca7b54381acfb24b03094b9abe8f4bf22c4e8ae475130b4d358cf` | `a24c33f29caca7b54381acfb24b03094b9abe8f4bf22c4e8ae475130b4d358cf` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/imessage-chats.json` | `0b5fe088186e5ac0e9335ffac8409d77af6483b069b40c189ddacb3fc1017464` | `0b5fe088186e5ac0e9335ffac8409d77af6483b069b40c189ddacb3fc1017464` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/imessage-history.json` | `314eddaa6fdaa8dc423f9465c48d5607ec49b182bb0b96b1bee0df05b421a65a` | `314eddaa6fdaa8dc423f9465c48d5607ec49b182bb0b96b1bee0df05b421a65a` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/imessage-history.ndjson` | `a6b80e89e23b7d0a5330f5c72c3e72e48628c4b3ba5d300569e3de4559cb21d2` | `a6b80e89e23b7d0a5330f5c72c3e72e48628c4b3ba5d300569e3de4559cb21d2` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/kakao-messages-numeric.json` | `936b20cd30da29510a1332a2d0a1202e6eb1b13e6a9fcaf9c1eb3302a2f0c65b` | `936b20cd30da29510a1332a2d0a1202e6eb1b13e6a9fcaf9c1eb3302a2f0c65b` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/kakao-rooms-historical.json` | `014f2f102ac0c0dc5d48deddf3cbe4a22a7bb75d31cdefc42896208cfd9878e0` | `014f2f102ac0c0dc5d48deddf3cbe4a22a7bb75d31cdefc42896208cfd9878e0` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/kakao-rooms.json` | `5f5d2eaf4ec13aacc924c8898ad61e0283ece6abfe48311776fdb747e0ea4f9e` | `5f5d2eaf4ec13aacc924c8898ad61e0283ece6abfe48311776fdb747e0ea4f9e` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/mail-empty.json` | `5cdbe9fb42368950639ffcccdd290245d610998e531e2830e6d4f54e79d58ebb` | `5cdbe9fb42368950639ffcccdd290245d610998e531e2830e6d4f54e79d58ebb` | **MATCH** |
| `tests/fixtures/inbox/captured-task23/provenance.json` | `aa47955b0e39d43b53cc689d4abc23dc9e70a661154af678d71dc51a397253db` | `aa47955b0e39d43b53cc689d4abc23dc9e70a661154af678d71dc51a397253db` | **MATCH** |
| `src/inbox/sources/kakao.rs` | `b604647d15d32f97b0d63f88d86643252594882472f914354ce0b0c928b76250` | `b604647d15d32f97b0d63f88d86643252594882472f914354ce0b0c928b76250` | **MATCH** |
| `src/inbox/sources/discord.rs` | `cac3e132b1c302c973e7f070bc797df7738c92a904d906add1e41b9c96593cde` | `cac3e132b1c302c973e7f070bc797df7738c92a904d906add1e41b9c96593cde` | **MATCH** |
| `src/inbox/sources/imessage.rs` | `359c40b491d52c1d1050a2d54c53f2350ea8012fbf567f71dd13095772e4868a` | `359c40b491d52c1d1050a2d54c53f2350ea8012fbf567f71dd13095772e4868a` | **MATCH** |
| `src/inbox/sources/mail.rs` | `ad58b8787d5b11a9c37953ba9ed7cbe171f4e96828bcb1b6b2a3183a6ec6d413` | `ad58b8787d5b11a9c37953ba9ed7cbe171f4e96828bcb1b6b2a3183a6ec6d413` | **MATCH** |
| `src/inbox/sources/mod.rs` | `6ebaf534d85e42cfa0b8323c7b74c8e78bfc7c8729491bb280a63840fb69d237` | `6ebaf534d85e42cfa0b8323c7b74c8e78bfc7c8729491bb280a63840fb69d237` | **MATCH** |

---

### 4. Overfit & Slop Verification

- **No Useless or Tautological Tests**:
  - All 7 new tests directly execute production deserializers (`decode_structured_output`, `MailSource::parse`, `IMessageSource::parse`, `KakaoSource::parse_messages`, `DiscordSource::fetch_since`).
  - No mocks assert mocked behavior; real parsing algorithms are checked against captured structures.
- **No Unnecessary Production Abstractions**:
  - `decode_structured_output` in `src/inbox/sources/mod.rs` is shared between production CLI execution (`run_json`) and fixture testing, eliminating duplicate parsing code.
- **Zero Privacy Leakage**:
  - No unredacted message content, email bodies, phone numbers, or private user IDs are present in the repository or gate evidence.

---

## Final Recommendation

**APPROVE (CONFIRMED_SCOPE23_BOUNDED)**.
All Todo 23 source adapters, tests, live fetch evidence, and fixture lineages are 100% verified and confirmed green (`26 passed, 0 failed, 4 ignored`). The broader batch exit status remains accurately documented without invention.

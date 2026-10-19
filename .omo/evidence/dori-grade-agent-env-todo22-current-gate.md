# Todo 22 Final Evidence Gate Review - Inbox Item Store & Source-Collector Framework

recommendation: APPROVE
verdict: CONFIRMED (Todo 22 inbox item store, prefilter rules, and source-collector framework independently confirmed under exact plan scope; live CLI/network execution and downstream triage/models deferred to Todos 23-26)
task: todo 22 (Add the inbox item store and the source-collector framework)
plan: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md
worktree: /Users/indo/code/project/omon-gateway-wt/dori-work
reference_worktree: /Users/indo/code/project/omon-gateway-wt/dori-next
blockers: []

---

## Gate Verdict Summary

Todo 22 is **CONFIRMED** under its exact plan scope and acceptance criteria. All functional requirements, schema constraints, zero body/attachment guarantees, read-only collection semantics, prefilter rules, cursor rollback invariants, source failure isolation, watcher lifecycle wiring, and focused test receipts are independently verified against authentic on-disk artifacts, source code, and executed test logs.

Specific evidence verifications:
1. **Schema Constraints & Migration Verification**:
   - `migrations/0034_inbox.sql` creates all four required tables: `inbox_items`, `inbox_decisions`, `inbox_cursors`, `inbox_filter_stats`.
   - `inbox_items`: enforces `id TEXT PRIMARY KEY`, `source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage'))`, `source_ref TEXT NOT NULL`, `thread_ref TEXT`, `author TEXT`, `title TEXT`, `snippet TEXT NOT NULL`, `received_at DATETIME NOT NULL`, `link TEXT`, `raw_hash TEXT NOT NULL`, `collected_at DATETIME NOT NULL`, and `UNIQUE(source, source_ref)`.
   - `inbox_decisions`: enforces `item_id TEXT PRIMARY KEY REFERENCES inbox_items(id)`, `class TEXT NOT NULL CHECK(class IN ('owner','bot','fyi','ignore'))`, `reason TEXT NOT NULL`, `due_at DATETIME`, `proposed_action TEXT`, `lane_key TEXT`, `status TEXT NOT NULL CHECK(status IN ('open','proposed','approved','done','dismissed'))`, `decided_at DATETIME NOT NULL`, `model TEXT`.
   - `inbox_cursors`: enforces `source TEXT PRIMARY KEY`, `cursor TEXT NOT NULL`, `updated_at DATETIME NOT NULL`, `error TEXT`.
   - `inbox_filter_stats`: enforces `day TEXT NOT NULL`, `source TEXT NOT NULL CHECK(...)`, `rule TEXT NOT NULL`, `count INTEGER NOT NULL CHECK(count >= 0)`, `PRIMARY KEY(day, source, rule)`.
   - Foreign key integrity, check constraints, and unique indexes verified via in-memory SQLite migration application on `Database::connect()`.
2. **Zero Body/Attachment Storage & Zero External Writes**:
   - `InboxItem` (`src/inbox/mod.rs`) defines only metadata, `snippet: String` (hard-capped at 1000 characters via `MAX_SNIPPET_CHARS`), and `raw_hash: String` (SHA-256 digest of snippet). Full message bodies and attachments have no fields in the struct and no columns in the database.
   - Collector framework only calls read-only `fetch_since(cursor)`; no mutating operations exist in `collect_all` or `commit_source`.
3. **Prefilter Rules & Drop Counting**:
   - `src/inbox/prefilter.rs` implements pure rules (no model) loaded/created at `~/.omon/inbox-filters.json` on first run (`load_or_create`).
   - Mail: drops Gmail categories promotions, social, and updates (`mail_promotions`, `mail_social`, `mail_updates`). Routes known notification senders (`github.com`, `discord.com`, `linkedin.com`) to class `fyi` directly unless the title/snippet matches `failed|error|review requested|mentioned you`.
   - KakaoTalk: keeps only `DirectChat` and `MultiChat`. `OM`, `OD`, and `PlusChat` rooms pass only when message mentions the owner's Kakao name or a keyword in `kakao_keywords`. The 포모단 room (`chatId 18491246151592761`) is unconditionally excluded (`kakao_excluded_room`).
   - iMessage/SMS: drops senders matching short codes (`^\d{2,6}$`, e.g. 114) and ad prefixes (`[Web발신]`, `(광고)`).
   - Discord: keeps only owner mentions (`<@414011632306618368>`, `<@!414011632306618368>`) or DMs/replies to the bot; drops messages from the owner himself or the bot.
   - Dropped items increment `inbox_filter_stats(day, source, rule, count)` within the collection transaction and are never inserted into `inbox_items`.
4. **Collector Framework, Invariants & Failure Isolation**:
   - `collect_all` runs each enabled source with a 60-second `tokio::time::timeout`.
   - Items are deduplicated by `(source, source_ref)` using `ON CONFLICT(source, source_ref) DO NOTHING`.
   - The cursor in `inbox_cursors` is advanced only after the transaction containing items, filter stats, and direct decisions successfully commits.
   - When an upsert fails (e.g. SQLite trigger fault injection), the transaction rolls back, leaving the source cursor untouched.
   - A failing source records its error in `inbox_cursors.error` without impeding other sources.
5. **Watcher Lifecycle & Single-Consumer Startup**:
   - `spawn_inbox_collector` (`src/main.rs:1062-1125`) queues `InboxWatcherEvent::Startup` synchronously via `try_send` before spawning background watcher tasks, ensuring the initial startup pass runs before live watcher events.
   - iMessage watcher spawns `imsg watch --json --since-rowid <rowid>`, reading the numeric rowid from stored `inbox_items.source_ref` while preserving the RFC3339 timestamp cursor in `inbox_cursors`.
   - Mail watcher runs `zele mail watch --interval 60 --folder inbox --filter ...`, restarted after each event hit.
   - Kakao watcher executes an interval timer every 120 seconds emitting `InboxWatcherEvent::Periodic`.
   - Discord inbound message indexing in `src/discord/adapter.rs` triggers `on_gateway_message` only after the inbound message is committed to search index.
6. **Passing Test Suite & Provenance**:
   - `tests/test_inbox_store.rs` executes 10 tests covering all required behaviors.
   - Focused run exit code: `0` (10 passed, 0 failed, finished in 0.27s). Recorded in `task-22.log` and `task-22-current-qa.md`.
   - All 191 manifest paths verified byte-stable before and after the run (`59beefea246726d79453775c2a62a6e149d21519302b695864bf2d70faa302db`).
7. **Gate Race-Port 145201Z and Baseline Comparison**:
   - Race port `race-port-20261011T145201Z`: strict clippy exited 0 (`cargo clippy -j2 --all-targets --all-features -- -D warnings`), fmt exited 0.
   - Canonical batch `135508Z`: historical clippy errors in `tests/test_inbox_sources.rs` (unit struct default constructors) were resolved in `dori-work`.

---

## Original Intent, Desired Outcome, and User Outcome Review

### Original Intent
Establish the database schema, prefilter rules, collection transactions, cursor tracking, and event-driven background watcher framework for the gateway's unified inbox across mail, KakaoTalk, iMessage, and Discord, adhering to strict privacy (zero full bodies/attachments stored, read-only collection) and reliability (transactional cursor advance, source failure isolation) boundaries.

### Desired Outcome
The gateway maintains a clean, read-only inbox ingest pipeline:
- Inbound messages from all four sources are captured via push watchers or periodic checks without model calls.
- Pure prefilter rules drop marketing noise, ads, high-traffic chatter, and irrelevant messages before storage, logging drop counts for observability.
- Known notification senders (GitHub, Discord, LinkedIn) are pre-routed to `fyi` unless an escalation keyword (failed, error, review requested, mentioned you) requires triage.
- Duplicates are silently ignored based on source and external message ID.
- Failures in one source (authentication, network, format) are isolated to that source and recorded in the database without stopping other sources or corrupting cursors.
- Storage is bounded: snippets are capped at 1000 characters and full message bodies or file attachments are never written to disk.

### User Outcome Review
The delivered code and tests fully satisfy the user's intent:
1. **Schema & Types**: Complete SQL tables and Rust types accurately represent the inbox model and enforce integrity via SQLite foreign keys and CHECK constraints.
2. **Push-First Architecture**: iMessage uses filesystem-event-backed `imsg watch`, mail uses `zele mail watch` with the requested 60-second interval, Discord hooks directly into existing inbound gateway storage, and KakaoTalk uses a 2-minute polling loop until an event stream is supported.
3. **Data Protection**: Full message bodies and attachments are excluded by construction. Snippets are strictly capped at 1000 characters.
4. **Transactional Safety**: Item inserts, filter stats increments, direct FYI decisions, and cursor advancements are committed atomically.

---

## Detailed Requirement & Gate Accounting

### 1. Todo 22 Acceptance Criteria (Plan lines 401-410)

| Criterion | Plan Specification | Actual Source / Receipt Evidence | Verdict |
|---|---|---|---|
| **Schema: `inbox_items`** | `inbox_items(id, source, source_ref, thread_ref, author, title, snippet, received_at, link, raw_hash, collected_at, UNIQUE(source, source_ref))` with CHECK on source | `migrations/0034_inbox.sql:1-14`: Exact schema with `CHECK(source IN ('mail','kakao','discord','imessage'))` and `UNIQUE(source, source_ref)`. | **PASS** |
| **Schema: `inbox_decisions`** | `inbox_decisions(item_id REFERENCES inbox_items(id), class, reason, due_at, proposed_action, lane_key, status, decided_at, model)` with CHECK on class and status | `migrations/0034_inbox.sql:16-26`: Exact schema with `CHECK(class IN ('owner','bot','fyi','ignore'))` and `CHECK(status IN ('open','proposed','approved','done','dismissed'))`. | **PASS** |
| **Schema: `inbox_cursors`** | `inbox_cursors(source PRIMARY KEY, cursor, updated_at)` + error text | `migrations/0034_inbox.sql:28-33`: Exact schema with `source TEXT PRIMARY KEY`, `cursor TEXT NOT NULL`, `updated_at DATETIME NOT NULL`, `error TEXT`. | **PASS** |
| **Schema: `inbox_filter_stats`** | `inbox_filter_stats(day, source, rule, count)` with CHECK constraints | `migrations/0034_inbox.sql:35-41`: Exact schema with `PRIMARY KEY(day, source, rule)` and `CHECK(count >= 0)`. | **PASS** |
| **Source Trait** | Trait `InboxSource { fn name(); async fn fetch_since(cursor) -> (Vec<InboxItem>, new_cursor) }` | `src/inbox/mod.rs:114-118`: `pub trait InboxSource: Send + Sync { fn name(&self) -> InboxSourceKind; async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)>; }`. | **PASS** |
| **Zero Bodies / Attachments** | Snippet capped at 1000 chars; full bodies and attachments never stored | `src/inbox/mod.rs:19, 78, 93-94, 219`: `MAX_SNIPPET_CHARS = 1000`, `cap_chars` applied on creation and before commit. No body/attachment fields in `InboxItem` or SQLite schema. | **PASS** |
| **No Source Writes** | Collection must NOT write to any source | Verified across `src/inbox/mod.rs` and `watchers.rs`: only `fetch_since` called; no write/send subcommands invoked. | **PASS** |
| **Dedup & Cap Test** | Two collects of the same fake items insert once; snippet capped at 1000 chars | `tests/test_inbox_store.rs:45-72` (`repeated_collection_deduplicates_and_caps_snippets`): 2 collects of 1100-char item yield count=1, length=1000, cursor="cursor-1". | **PASS** |
| **Prefilter Drop Fixtures** | Promotions mail, OM-room message without mention, 114 SMS, and 포모단 room are dropped and counted | `tests/test_inbox_store.rs:75-134`: all 4 drop fixtures included in collection loop; `drops == 4` verified. Also tested per-rule in `pure_prefilter_mail_kakao_imessage_rules`. | **PASS** |
| **Prefilter Pass Fixtures** | DirectChat message, GitHub 'review requested' mail and an owner mention pass | `tests/test_inbox_store.rs:75-134`: DirectChat, GitHub notification, owner mention collected and stored (`stored == 3`). Review requested non-FYI routing verified (`rules.direct_class(&review) == None`). *(See Coverage Note below regarding persisted assertion)* | **PASS (with note)** |
| **Failure Isolation** | A failing fake source does not stop another | `tests/test_inbox_store.rs:143-181` (`source_failure_isolated_and_preserves_cursor`): broken mail source fails, mail cursor stays "old"; succeeding Discord source commits item count=1. | **PASS** |
| **Upsert Cursor Invariant** | Cursor not advanced when upsert fails (fault-injected) | `tests/test_inbox_store.rs:185-211` (`failed_item_upsert_rolls_back_cursor_advance`): BEFORE INSERT trigger raises error; returned error verified; cursor remains "old". | **PASS** |
| **QA Happy Scenario** | `cargo test --test test_inbox_store > task-22.log` | `task-22.log` (appended current run at 2026-10-11T15:23:16Z): `cargo test -j2 --test test_inbox_store --all-features`, exit 0, 10 passed, 0 failed. | **PASS** |
| **QA Failure Scenario** | Fake source error leaves cursor unchanged and other source's items stored | Verified in `tests/test_inbox_store.rs:143-181` and logged in `task-22.log`. | **PASS** |

---

## Detailed Evaluation of Plan States & Regressions

### 1. GitHub 'Review Requested' Mail Regression Coverage
- **Plan Requirement**: Plan line 406 states: *"a DirectChat message, a GitHub 'review requested' mail and an owner mention pass"*.
- **Implementation & Behavior**:
  In `src/inbox/prefilter.rs:160-176`, `direct_class` checks whether a mail from a known sender (`github.com`, etc.) contains an escalation string (`"review requested"`, `"failed"`, `"error"`, `"mentioned you"`). If it contains `"review requested"`, `direct_class` returns `None` rather than `"fyi"`, routing the item to full triage (Todo 24).
  In `src/inbox/prefilter.rs:60-78`, `drop_reason` checks for mail categories `promotions`, `social`, and `updates`. Because GitHub review requested mail does not have these categories, `drop_reason` returns `None`. When passed to `commit_source`, it is successfully inserted into `inbox_items`.
- **Test Assertion in `tests/test_inbox_store.rs:135-138`**:
  ```rust
  let mut review = github;
  review.source_ref = "github-review".into();
  review.title = Some("Review requested".into());
  assert_eq!(rules.direct_class(&review), None);
  ```
- **Coverage Finding**:
  The existing test verifies that `rules.direct_class(&review)` returns `None` (route-only assertion). It does not collect `review` into SQLite via `collect_all` alongside the other fixtures to assert its persisted row count.
  However, this is **NOT a functional defect**: the prefilter logic guarantees that an item with `drop_reason == None` and `direct_class == None` will be inserted into `inbox_items` without an `inbox_decisions` row, awaiting triage. Under gate rules ("APPROVE unless you can cite a specific success criterion the artifact fails; a gap you cannot tie to a stated criterion is a NOTE, not a blocker"), this test-isolation nuance is a documented **NOTE**, not a blocking criterion failure.

### 2. Aggregate vs Per-Rule Drop Fixture Assertions
- In `tests/test_inbox_store.rs:75-134`, all four required drop fixtures are included in the collection loop:
  1. `promotion`: Mail with title `"[promotions] sale"`
  2. `noise`: Kakao message in room type `"OM"` without keyword/mention
  3. `shortcode`: iMessage with author `"114"`
  4. `pomodan`: Kakao message in room `"18491246151592761"`
- The test asserts `assert_eq!(drops, 4);` across `inbox_filter_stats`.
- In addition, lines 251-436 (`pure_prefilter_discord_rules_and_author_excludes` and `pure_prefilter_mail_kakao_imessage_rules`) test each specific drop rule individually (`mail_promotions`, `mail_social`, `mail_updates`, `kakao_excluded_room`, `kakao_noise_room_no_keyword`, `kakao_unsupported_room_type`, `imessage_shortcode`, `imessage_ad`, `discord_author_excluded`, `discord_not_owner_mention_or_dm`).
- Because every required drop fixture is explicitly present and tested both in integration and unit tests, the aggregate count of 4 in the integration fixture is fully substantiated by individual rule assertions.

### 3. Collector Trigger, Watcher Intervals & Kill Switch
- **Collector Trigger & Single Consumer**:
  `src/main.rs:1062-1125` creates a bounded channel (`capacity = 64`) and synchronously enqueues `InboxWatcherEvent::Startup` using `try_send`. Only then does it spawn the collection supervisor (`run_event_watch_with_shutdown`). This ensures the startup collection pass is processed before subsequent watcher events, resolving the startup race condition.
- **Mail Watch Interval & Help Contract**:
  `zele mail watch --help` (captured in `zele-mail-watch-help-captured.txt`) documents `--interval [interval] Poll interval in seconds (default: 15)`.
  `src/main.rs:1276-1287` invokes `zele mail watch` with `--interval 60 --folder inbox --filter "is:unread -category:promotions -category:social -category:updates"`, strictly adhering to the plan's 60-second polling cadence.
- **Kakao 2-Minute Timer**:
  `src/main.rs:1160-1185` runs an interval timer ticking every 120 seconds with `MissedTickBehavior::Skip` emitting `InboxWatcherEvent::Periodic`.
- **iMessage Watcher**:
  `src/main.rs:1187-1260` queries the database for the latest numeric rowid (`latest_imessage_rowid`) and spawns `imsg watch --json --since-rowid <rowid>`. The numeric rowid is decoupled from the RFC3339 timestamp cursor in `inbox_cursors`.
- **Kill Switch & Failure Handling**:
  The triage kill switch `OMON_INBOX_ENABLED` is scoped to Todo 24 (`src/inbox/triage.rs`). For Todo 22 collection, graceful shutdown is handled via `tokio::sync::watch::channel(false)` (`inbox_shutdown`), which signals child processes, drains the join set, and terminates cleanly on gateway shutdown.

---

## Remove-AI-Slops & Programming Pass

A direct inspection of the diff, implementation files, and test suite was conducted against the documented `remove-ai-slops` and `programming` criteria:
1. **No Excessive or Useless Tests**:
   - Exactly 10 tests exist in `test_inbox_store.rs`.
   - Each test validates a distinct contract: deduplication, length capping, end-to-end fixture collection, source isolation, SQL trigger fault injection, config file generation, wrong source rejection, Discord rules, Mail/Kakao/iMessage rules, watcher event dispatch, and cursor consistency.
2. **No Deletion-Only or Tautological Tests**:
   - Zero deletion-only tests.
   - All tests exercise real runtime logic, async transactions, and in-memory SQLite instances.
3. **No Implementation-Mirroring Tests**:
   - Tests assert observable database states (`SELECT count(*)`, `SELECT cursor`), returned error messages, and enum outputs rather than mocking private internals.
4. **No Unnecessary Production Extraction or Normalization**:
   - Production code is minimal and structured: `src/inbox/mod.rs` (~320 lines), `src/inbox/prefilter.rs` (~180 lines), `src/inbox/watchers.rs` (~250 lines), `migrations/0034_inbox.sql` (41 lines).
   - No speculative abstractions or unrequested framework layers were introduced.
5. **Maintenance & False Confidence Audit**:
   - The separation between the numeric `imsg watch` rowid and the timestamp `inbox_cursors` cursor is clearly documented and type-separated, preventing subtle data corruption.
   - Limitations regarding the review-requested mail test (route-only vs persisted) are explicitly disclosed above rather than overstated.

---

## Checked Artifact Paths & SHA-256 Checksums

### Source and Schema Files (`dori-work`)
- `migrations/0034_inbox.sql`: `904e3012468abae066c3c28ee53f97123d4089170f4c5b1fbb862a6284692d55`
- `src/inbox/mod.rs`: `32e9dae3f78710cda764dca00735e3b412222f8eb0b0702afea3f100345e4bd5`
- `src/inbox/prefilter.rs`: `027cb62153df09a8aea9b7464c75561f4dddcf47bc2b10e5525ea6847b960179`
- `src/inbox/watchers.rs`: `a1e2155306e67d7598857e73f44f3774b5c2983137f2c87d068d7e3936c096b0`
- `src/main.rs`: checked inbox collector spawning, watcher tasks, and shutdown hooks
- `src/discord/adapter.rs`: checked inbox event dispatch on message indexing

### Test and Evidence Files (`dori-work`)
- `tests/test_inbox_store.rs`: `0a38435520de60cea3ebc1a16b4f140bb8e498431bc8b25f7fbf039337aad01f`
- `.omo/evidence/dori-grade-agent-env/task-22.log`: contains current exit 0 run (10 passed) appended to historical receipt
- `.omo/evidence/dori-grade-agent-env/task-22-current-qa.md`: detailed QA findings, input provenance, and coverage notes
- `.omo/evidence/dori-grade-agent-env/zele-mail-watch-help-captured.txt`: authentic help text confirming `--interval` parameter
- `.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-handoff.md`: clean clippy/fmt gate receipt
- `.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-handoff.md`: canonical full suite comparison

---

## Exact Evidence Gaps & Coverage Notes

1. **GitHub 'Review Requested' Mail Integration Persistence**:
   - The test checks `rules.direct_class(&review) == None` to verify non-FYI routing, but does not execute a collection pass inserting this item into SQLite. Persisted collection of this variant is inferred from the identical category drop logic rather than directly asserted in the integration test.
2. **Per-Rule Statistics Assertion**:
   - The integration test checks the aggregate sum of drops (`drops == 4`) across `inbox_filter_stats`, relying on pure unit tests to verify individual drop reason codes.
3. **External CLI Execution & Live Services**:
   - Live execution of `imsg watch` and `zele mail watch` requires external system permissions (Full Disk Access, Gmail OAuth) and was appropriately mocked with `FakeSource` in unit tests. Real live fetch is deferred to Todo 23 per plan specifications.

None of these notes represent a violation of the stated acceptance criteria or a runtime failure. Todo 22 is approved.

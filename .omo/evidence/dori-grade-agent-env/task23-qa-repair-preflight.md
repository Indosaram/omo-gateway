# Todo23 QA repair preflight

2026-10-11. Status: patch-ready proposal; NOT implemented, NOT executed, NOT confirmation of todo23.

## Execution boundary

The grouped product run remains frozen. This preparation used bounded source reads and filesystem metadata checks only. No commands, Cargo, live fetch, network, database open, service change, source/test edit or delegation was performed. Only this evidence document is added.

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`. Reference: `.omo/evidence/dori-grade-agent-env/task-23-current-evidence-map.md`. Apply repairs only after the coordinator releases the grouped-run freeze. Preserve prior logs and manifests; do not replay global gates or overwrite old receipts.

## Minimum allowed repair files

- `tests/test_inbox_sources.rs`: four ignored QA output changes; replace the weak bad-endpoint regression with collector-level coverage using the actual Kakao adapter and actual Discord SQLite adapter, plus fixture-backed mail/iMessage sources.
- `tests/fixtures/inbox/{mail.json,kakao-rooms.json,kakao-messages.json,imessage-chats.json,imessage-history.json,imessage-history.ndjson,discord.json}`: replace only fixtures for which a real capture and sanitization lineage can be recorded. Do not fabricate missing nonempty responses. NDJSON is supplementary framing coverage, not a mandatory extra live fetch if the real response is a single object.
- New lineage and QA artifacts under `.omo/evidence/dori-grade-agent-env/`, with fresh attempt-specific filenames.

No product source change is needed for these two repairs. `tests/test_inbox_store.rs` is a reference pattern, not an additional edit target. If these repairs reveal a product defect, report it separately rather than expanding this patch.

## Repair A: counts-only ignored QA

In each `live_{mail,kakao,imessage,discord}_source_fetch_once`, destructure the result as `(items, _cursor)` and print only `QA_COUNT source=<fixed source name> items=<usize>`. Remove every cursor from output. Legitimate zero items must pass. Do not print item IDs, room/chat IDs, authors, titles, bodies, links, excerpts, account tokens or database rows.

Keep `fetch_since(None)` and existing adapter construction unchanged. Discord must retain required `OMON_QA_DISCORD_COPY_DB` and `SqliteConnectOptions::read_only(true)`. No fallback to a live repository DB and no `Database::connect` migration path for this QA connection. Add a bounded outer timeout, for example 60 seconds, around each entire fetch so a stalled process/HTTP request cannot hang the QA run. Time is a bound, not an assertion; no sleeps or polling.

On failure, ordinary unwrap/debug error output can include URLs, room identifiers or subprocess stderr. Do not retain unrestricted failure output as counts-only evidence. Prefer a fixed source/status failure line in the harness and fail without including the underlying error value; report detailed diagnostics separately only after local sanitization. Do not convert failure to a passing count of zero. No new prose-pinning test is needed for these log changes.

## Repair B: deterministic bad-endpoint collector isolation

Observed patterns: `src/inbox/mod.rs:125-180` collects sources sequentially, captures errors and continues; `record_source_error` preserves an existing cursor. `tests/test_inbox_store.rs:10-29` has an `InboxSource` fake; `source_failure_isolated_and_preserves_cursor` seeds an old cursor, collects a broken source before a successful source, and checks rows/cursor. Current todo23 bad-endpoint test only calls Kakao then `MailSource::parse`.

Replace that test in `tests/test_inbox_sources.rs` with `kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor`:

1. Create `Database::connect("sqlite::memory:")`; seed `inbox_cursors` for Kakao with a known valid per-room JSON cursor. Use fixed fixture timestamps, not elapsed-time assertions.
2. Add one small test-local fixture-backed `InboxSource` implementing only mail and iMessage. Its `fetch_since` invokes `MailSource::parse` or `IMessageSource::parse` on existing fixture bytes and increments a per-instance `AtomicUsize`. Explicitly reject an unsupported kind. Reuse production parsers rather than manually assembling successful return items. This fake replaces only external CLI execution; it must not claim live mail/iMessage behavior.
3. Seed a synthetic other-author Discord DM through `InboundEvent` -> `MessageSearchIndex::index_inbound`, then use actual `DiscordSource::with_ids` on that same in-memory pool. Use `guild_id=None`, a non-owner/non-bot author, and a nonempty fixed snippet so default prefilters keep it. Mail fixture primary category and iMessage redacted non-shortcode sender are also eligible under `PrefilterRules::default()`; do not call `default_filters()` because it writes the owner's filter file.
4. Construct actual `KakaoSource::new("http://127.0.0.1:9", None)`. Call `collect_all(pool, &[&kakao, &mail_fixture_source, &imessage_fixture_source, &discord_source], &PrefilterRules::default())` once under a bounded timeout. Kakao first makes a fail-fast regression visible: all later sources must still be called.
5. Assert exactly four results in supplied order. Kakao must have the clear GET rooms failure; mail, iMessage and Discord must have no error and each must insert one item. Assert each CLI fixture source call count is exactly one, persisted item counts equal one per successful kind, their cursors advance to the returned known values, and Kakao's old cursor is unchanged with recorded error. Do not assert serialized HashMap key order; compare parsed maps.
6. Do not use sleeps, retries, real mail/imsg subprocesses, external HTTP, real Discord DB, owner Discord UI/API, or a model in this regression.

This closes collector continuation across all three other kinds using a real failing Kakao HTTP adapter, real Discord SQL adapter, and production mail/iMessage parsers behind source fakes. It is not evidence that three live sources fetched during a failure. Keep that distinction in the test receipt.

Port 9 is the plan's explicit negative endpoint, but a local listener there would invalidate the failure precondition. Do not stop or alter a service to make it pass. If the endpoint unexpectedly accepts requests, report the conflict; a separate closed ephemeral-port variant would be different evidence, not the specified port-9 proof. The outer timeout bounds this concern without timing luck.

## Locally discovered prerequisites, no secret values

- `zele` executable found at `/Users/indo/.bun/bin/zele` and its global package bin path. `/Users/indo/.zele` exists and contains `sqlite.db`; credentials were not read. Historical task-23 evidence records successful mail list with an earlier expired-token refresh warning. Current account/auth validity is UNKNOWN; file presence is not auth proof. Account selection is already in MailSource; do not ask the owner for it or modify credentials/.env.
- `imsg` executable found at `/opt/homebrew/bin/imsg`; `/Users/indo/Library/Messages/chat.db` exists and is filesystem-readable. Current actual CLI permission/FDA/schema access is UNKNOWN until the authorized fetch; no OS permission change or relogin is part of QA.
- PirateTalk uses the compiled default endpoint because no URL override is configured in this session. Historical HTTP success exists; current connectivity/auth availability is UNKNOWN. Never print auth headers or fetch a write endpoint.
- `/usr/bin/sqlite3` exists. Repository gateway DB and WAL exist and are readable. `OMON_QA_DISCORD_COPY_DB` is not configured in this session. No copy was created or verified here. Never infer a copied DB from a convenient filename.
- Cargo exists at `/Users/indo/.cargo/bin/cargo`. Its presence does not authorize running it during the freeze. Current build/global gates remain the coordinator's responsibility.

## Future minimum actual adapter QA invocations

These commands are proposed, not run. After the freeze is released, implement the test-only repairs and run one focused test command covering this target (no global replay):

```sh
cargo test --test test_inbox_sources
```

Then select each actual ignored adapter exactly once, retaining a fresh command/exit receipt and only sanitized counts/status output:

```sh
cargo test --test test_inbox_sources live_mail_source_fetch_once -- --ignored --exact --nocapture
cargo test --test test_inbox_sources live_kakao_source_fetch_once -- --ignored --exact --nocapture
cargo test --test test_inbox_sources live_imessage_source_fetch_once -- --ignored --exact --nocapture
OMON_QA_DISCORD_COPY_DB="$QA_DISCORD_URI" cargo test --test test_inbox_sources live_discord_source_fetch_once -- --ignored --exact --nocapture
```

Use the coordinator's existing approved build target/environment; do not invent another target or duplicate an active Cargo process. Counts-only capture should select only `QA_COUNT` lines plus fixed result/exit metadata, not unrestricted stderr. Do not save raw failure stderr automatically. These invocations call each adapter's `fetch_since`; direct CLI probes are not substitutes. Kakao's single fetch can page many rooms; imsg's single fetch calls chats and eligible histories. Do not reduce them to rooms/chats probes and call that adapter QA.

## Explicit verified read-only Discord copy

Future copy preparation is a local read, not Discord manipulation. Use a consistent SQLite backup, not independent live DB/WAL/SHM copies during writes. Open the source using a mode=ro URI and SQLite's backup API into a newly created private scratch path. Proposed fixed source URI: `file:/Users/indo/code/project/omon-gateway/omon_gateway.db?mode=ro`. The source must never be opened with a write/migration helper.

In a future local harness, call SQLite backup with the source mode=ro and a fresh scratch destination; alternatively invoke `/usr/bin/sqlite3` with that source URI and its `.backup` command to the scratch destination. Resolve the destination first and supply it as a quoted CLI argument, not a shell-concatenated SQL path. Include WAL state through SQLite's snapshot machinery. Do not set immutable=1 on the active WAL-backed source.

Before the Discord ignored test: record source identity, backup method/time, destination resolved path and SHA-256; assert destination is not source, is not a symlink, and is a standalone snapshot without unresolved WAL. Reopen the destination mode=ro, run `PRAGMA quick_check` and verify the `message_search_documents` schema exists using read-only SQL. Retain status/counts only. Build `QA_DISCORD_URI` as an explicit sqlx SQLite URI for that verified destination, and ensure the test also enforces read_only(true). No token, UI, Discord API, migration, or owner interaction is required. A zero eligible count is valid but cannot yield a nonempty real fixture.

## Real fixture lineage without another unrecorded probe

Adapter counts QA and fixture capture are two distinct evidence products. Existing harnesses return parsed items, not original bytes; do not pretend hashing InboxItem output authenticates a real API response. Use a separately identified bounded source capture with the same read-only command/endpoint contract, or add test-local stdout/HTTP capture instrumentation later if the coordinator explicitly accepts its extra scope. Do not alter production adapters just to capture data.

Minimum capture contracts, once later authorized:

- Mail: `zele mail list --folder inbox --account freedomzero91@gmail.com --filter 'is:unread -category:promotions -category:social -category:updates' --limit 100`; no --json, mail read or write command. Keep actual YAML framing as capture input; parse and preserve required field types before sanitization. Do not invent a nonempty response if empty.
- iMessage: `imsg chats --limit 50 --json`, then `imsg history --chat-id <id selected locally from that response> --limit 50 --json` for one eligible returned chat. IDs stay in memory/private capture handling, never public QA logs. Preserve actual object/NDJSON framing and numeric identifier types. Do not call send/read/react.
- Kakao: GET `/api/rooms`, then GET `/api/rooms/<locally selected returned room>/messages`; omit initial before_id. Prefer one eligible DM room over the excluded high-volume room; preserve the observed unread/type selection fields. No write method or endpoint. One real rooms fixture and one real message page are needed; actually consume the rooms fixture in the existing synthetic HTTP server response rather than merely declaring it in fixture().
- Discord: SELECT a minimum projected indexed row from the verified mode=ro copy, using the actual adapter eligibility conditions. Include only fields the fixture/test needs plus relevant reference metadata. No API or UI action. If no eligible row exists, record that absence and leave real-row fixture provenance missing instead of manufacturing a row.

Raw captures must not go into repo/evidence logs. Hold bytes locally in memory, compute SHA-256 before transformation, apply a documented allowlist projection/redaction, then write sanitized fixtures via apply_patch. Replace personal text/handles and remap IDs consistently (parent/reply/room relationships); preserve schema, framing, timestamp type, boolean flags and referential relationships. Do not strip required fields until the production parser no longer recognizes the real response. Use explicit REDACTED placeholders rather than ambiguous sample prose. Discard raw buffers; do not archive message bodies or auth data.

For each sanitized file record: source kind; read-only operation and CLI version from already captured help where available; capture UTC time; raw byte SHA-256; framing/row count; sanitizer/projection rules and consistent ID-remapping description; sanitized file SHA-256; exact consuming parser/test; command exit or HTTP status; cleanup receipt. Redact selected chat/room identifiers from the public invocation ledger. A raw digest ties the transform to an observed payload but is not independently reproducible after raw deletion; state that limitation. Never retrospectively assign a new capture digest to an existing synthetic fixture.

## Completion criteria for the later repair

The focused isolation regression passes in one run; all three successful collector results/rows/cursors are verified; four actual ignored fetch receipts have counts-only output and clear exits; Discord receipt names a verified read-only copy; per-source real fixture lineage is recorded or its genuine absence remains explicit. No live QA receipt is inferred from an ignored line, direct probe, mock result or compilation. Todo23 remains unconfirmed until its missing acceptance evidence is supplied and the coordinator verifies the integrated frozen source.

Read scope: collector implementation/failure persistence and existing FakeSource/failure test excerpts; prefilter eligibility rules; filesystem metadata for named prerequisites; previously read adapter/tests/fixture evidence retained from the accepted map. No credential contents, database contents or current remote authentication state were read in this preflight.

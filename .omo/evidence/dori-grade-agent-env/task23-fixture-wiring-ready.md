# Task23 real fixture wiring ready unit

Status: PREPARED ONLY. Execute after parent confirms task14 live QA cleanup and releases the source freeze. No source/test/fixture edits, fetches, tests, compilation, or model calls performed for this preparation. Existing accepted task23.log/json and historical capture receipts remain authoritative and must not be overwritten.

Root: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Scope and prerequisite

Only future test-code edit: `tests/test_inbox_sources.rs`. No production parser changes. Add fixtures under `tests/fixtures/inbox/captured-task23/` and an adjacent `provenance.json`. Keep all existing populated/synthetic product fixtures and their tests unchanged. Preserve `.omo/evidence/dori-grade-agent-env/task23-capture-20261011T161359754Z/` byte-for-byte. The real fixtures below are separate coverage, not a replacement or relabeling of synthetic data.

One outstanding prerequisite: authenticated Kakao numeric timestamp fixture. The owned capture directory contains only sanitized fixtures/lineage/report; owned `/tmp/task23-discord-6joAvb` contains only snapshot.db; no task23 transient remains under the session TMPDIR. Recorded rawRetention says response buffers were cleared. No retained raw message response was found in these bounded owned locations. The Discord snapshot is not Kakao raw data. Do not search unrelated private archives, derive times from IDs/sendAt, or fill missing captured time with synthetic regression values.

After release, if a capture owner supplies a previously retained owned raw buffer/file, inspect it privately, verify its full-byte SHA-256 against the messages receipt before selecting any rows, and sanitize exact observed numeric `time`. Otherwise perform one new read-only messages GET page, with supported `limit=1`, against an eligible room. Its original selection binding is not recoverable from the sanitized substituted room ID or selected-ID digest. If no private binding survives, one read-only rooms GET is necessary to select an eligible room; record it separately. No history paging or all-room fetch, no recapture of mail/iMessage/Discord, no write endpoint. A zero/invalid/nontext page is a reported prerequisite failure, not permission to fabricate or silently retry.

For new capture record request method, endpoint template with selected IDs omitted, HTTP status, capture time, exact raw-byte SHA-256 before projection, schema, row count, selected-ID digest, transformation description, and sanitized-byte SHA-256. Retain `time` exactly as an integral numeric Unix-seconds value; retain ID ordering/relations via consistent substitutions, not actual IDs. Keep only parser-needed message fields: substituted id/logId, exact time, placeholder nonempty text and author. Exclude sendAt and other personal fields. New room projection must come from the same selection and preserve actual type/unreadCount; substitute chatId/lastLogId consistently with its new message projection. Do not assert observed lastLogId equals page top unless actually verified. Clear raw bytes after digest/projection; reports contain no actual names, IDs, text or URL identifiers.

## Fixture additions and provenance

Add using apply_patch after release, not shell copy/redirection. Four files copy exact sanitized bytes from the historical capture fixtures:

| Destination in captured-task23 | Historical source | SHA-256 |
| --- | --- | --- |
| mail-empty.json | fixtures/mail.json | 5cdbe9fb42368950639ffcccdd290245d610998e531e2830e6d4f54e79d58ebb |
| imessage-chats.json | fixtures/imessage-chats.json | 0b5fe088186e5ac0e9335ffac8409d77af6483b069b40c189ddacb3fc1017464 |
| imessage-history.json | fixtures/imessage-history.json | 314eddaa6fdaa8dc423f9465c48d5607ec49b182bb0b96b1bee0df05b421a65a |
| discord-indexed-row.json | fixtures/discord.json | a24c33f29caca7b54381acfb24b03094b9abe8f4bf22c4e8ae475130b4d358cf |

Also preserve the exact historical rooms projection as `kakao-rooms-historical.json` (SHA-256 014f2f102ac0c0dc5d48deddf3cbe4a22a7bb75d31cdefc42896208cfd9878e0). It gets room-parser coverage below without pretending to correspond to a newly selected live message page.

Add `kakao-rooms.json` and `kakao-messages-numeric.json` only after the prerequisite above. If raw recovery matches the old digest, historical rooms may be reused with verified matching substitutions. If a new capture is required, both projections share its new binding. Do not edit the historical timestamp-incomplete message fixture; document it as nonreplayable for successful timestamp parsing.

`provenance.json` contains one entry per added fixture: relative path, source/operation, historical receipt path or fresh receipt identity, original digest/status/time, sanitized digest, mapping, and consumer test name. Mail entry references mail-lineage.json (supersedes earlier blocked receipt); others reference lineage.json. Include explicit limitations: empty mail is legitimate, iMessage framing normalized from NDJSON, identifiers substituted, Kakao numeric time exact observed, historical rooms and fresh page distinct unless proven matching, Discord actual indexed-row SELECT versus synthetic producer regression. No raw selected IDs in manifest. Record actual hashes, never placeholders at execution.

## Hunk F1: extend fixture lookup only

Append these arms before the existing unknown-fixture arm; do not rename existing arms:

```rust
"captured-mail-empty" => include_str!("fixtures/inbox/captured-task23/mail-empty.json"),
"captured-imessage-chats" => include_str!("fixtures/inbox/captured-task23/imessage-chats.json"),
"captured-imessage-history" => include_str!("fixtures/inbox/captured-task23/imessage-history.json"),
"captured-discord" => include_str!("fixtures/inbox/captured-task23/discord-indexed-row.json"),
"captured-kakao-rooms-historical" => include_str!("fixtures/inbox/captured-task23/kakao-rooms-historical.json"),
"captured-kakao-rooms" => include_str!("fixtures/inbox/captured-task23/kakao-rooms.json"),
"captured-kakao-numeric" => include_str!("fixtures/inbox/captured-task23/kakao-messages-numeric.json"),
```

## Hunk F2: actual empty mail and staged iMessage parser tests

Add adjacent to existing parser tests. Given/When/Then comments mark fixture provenance; assert machine fields, not prose.

```rust
#[test]
fn mail_parses_authenticated_empty_capture_without_populated_substitution() {
    // Given: actual empty mail list, distinct from synthetic populated mail.
    let payload = fixture("captured-mail-empty");
    for prior in [None, Some("1791676800")] {
        // When
        let (items, cursor) = MailSource::parse(&payload, prior).unwrap();
        // Then
        assert!(items.is_empty());
        assert_eq!(cursor, prior.unwrap_or(""));
    }
}

#[test]
fn imessage_parses_authenticated_sanitized_chat_and_history() {
    // Given: actual normalized captured history with consistently substituted IDs.
    let chats = fixture("captured-imessage-chats");
    let history = fixture("captured-imessage-history");
    let chat_id = chats["id"].as_i64().unwrap().to_string();
    let rows = history.as_array().unwrap();
    // When
    let (items, cursor) = IMessageSource::parse(
        &chats, &[(chat_id.clone(), history.clone())], None,
    ).unwrap();
    // Then
    assert_eq!(items.len(), 2);
    for (item, row) in items.iter().zip(rows) {
        assert_eq!(item.source, InboxSourceKind::Imessage);
        assert_eq!(item.source_ref, row["id"].as_i64().unwrap().to_string());
        assert_eq!(item.thread_ref.as_deref(), Some(chat_id.as_str()));
        assert_eq!(item.received_at,
            row["created_at"].as_str().unwrap().parse::<chrono::DateTime<Utc>>().unwrap());
    }
    let latest = items.iter().map(|item| item.received_at).max().unwrap();
    assert_eq!(cursor.parse::<chrono::DateTime<Utc>>().unwrap(), latest);
}
```

The two historical incoming messages remain two; do not replace them with product NDJSON fixture or mutate from-me/timestamp fields to make assertions pass.

## Hunk F3: actual Discord indexed-row SQL adapter

Add `#[tokio::test] async fn discord_parses_authenticated_sanitized_indexed_row()` adjacent to existing Discord cases. Construct migrated owned in-memory Database, never owner/snapshot migration. Read fixture("captured-discord"). Seed the exact indexed row using the existing INSERT in discord_behavioral_db_cases_and_cursor, replacing all bind values with fixture fields:

```rust
sqlx::query(
    "INSERT INTO message_search_documents (platform,guild_id,channel_id,thread_id,message_id,author_id,author_name,content,attachment_names,timestamp,metadata_json) VALUES (?,?,?,?,?,?,?,?, '[]',?,?)"
)
.bind(row["platform"].as_str().unwrap())
.bind(row["guild_id"].as_str())
.bind(row["channel_id"].as_str().unwrap())
.bind(row["thread_id"].as_str())
.bind(row["message_id"].as_str().unwrap())
.bind(row["author_id"].as_str().unwrap())
.bind(row["author_name"].as_str().unwrap())
.bind(row["content"].as_str().unwrap())
.bind(row["timestamp"].as_str().unwrap())
.bind(row["metadata_json"].as_str().unwrap())
.execute(pool).await.unwrap();
```

When: actual `DiscordSource::with_ids(pool.clone(), "414011632306618368", None).fetch_since(None)` once. Then: one item, source Discord, source_ref/cursor equal fixture message_id, author/title/snippet equal fixture fields, thread_ref None, room_type dm, received_at equals parsed fixture timestamp. Close pool. This is actual production SQL extraction from a sanitized captured row. Keep separate existing InboundEvent producer integration; routing a captured row through a synthetic producer would discard actual indexed metadata.

## Hunk F4: numeric Kakao parser and actual room fetch wiring

After authenticated numeric fixture exists, add ordinary `kakao_parses_authenticated_numeric_seconds_capture`: Given captured rooms/page; derive substituted room ID/title/type from first room. When actual KakaoSource::parse_messages with empty floor. Then nonempty items, item count equals retained page length, each received_at.timestamp() equals its corresponding exact fixture time.as_i64(), source_ref equals substituted row id, thread_ref equals room ID, oldest/newest equal min/max substituted message IDs (not time), author/title equal sanitized fixture fields. No RFC3339 insertion, no hardcoded fabricated seconds.

Add `#[tokio::test] async fn kakao_fetches_authenticated_room_and_numeric_page_on_owned_loopback()` adjacent to kakao_event_driven_paging_loop. It must use actual KakaoSource::fetch_since, not only fixture JSON parsing. Build an owned ephemeral loopback Axum server, with `tokio::task::JoinSet` and oneshot graceful shutdown; register handlers before calling source. Exact route plan:

1. GET `/api/rooms`: return captured-kakao-rooms-historical; first fetch passes a floor map for its sole substituted room above all retained historical message IDs. Its `/messages` handler returns `[]`. This exercises production room parsing without inventing historical timestamps. Assert no items and preserved parsed floor map. This terminator is explicitly synthetic HTTP control, not a captured empty response.
2. Switch to a second independently constructed ephemeral server/test for captured-kakao-rooms plus captured-kakao-numeric: GET rooms returns matching capture; GET selected substituted room messages without query returns exact numeric page; GET same route with `before_id` equal to page minimum substituted ID returns `[]`. Return fixed error for every other path/query. This empty terminator is synthetic paging control, not another captured page or proof of exhausted live history.
3. Call actual KakaoSource::new(loopback_url, None).fetch_since(None) once. Assert numeric item count/timestamps/room metadata and parsed per-room cursor equal max message ID. Assert exact three request events (rooms, initial messages, terminal messages). Keep recorded routes local to assertions; do not print identifiers.

Implement as TWO independent test functions, `kakao_fetches_authenticated_historical_room_on_owned_loopback` and `kakao_fetches_authenticated_room_and_numeric_page_on_owned_loopback`, not two actions in one test. Historical test expects two requests; numeric test expects three. Use Axum closure state holding JSON fixtures and expected substituted room/minimum IDs; validate path/query before response. Only GET routes, no external base_url or global environment mutation. Subscribe request events via bounded mpsc before fetch, assert received sequence using try_recv after completed fetch (handlers enqueue before responding), assert no extra event. Bound entire scenario at 10 seconds; send shutdown and join server explicitly; JoinSet drop aborts on assertion/timeout failure. No sleep, polling, detached server, or service change. Reuse existing dependencies; no Cargo.toml edit.

Historical lastLogId is only a substituted projection value. Compute test floor by appending a digit to that substituted lastLogId to guarantee lexical greater-than; no timestamp conversion. Historical server never claims captured message coverage. Numeric/new rooms use matched provenance, not silently mixing unrelated capture selections.

## Completion and future verification

Ready unit includes all four real sanitized sources: empty-mail parser, staged iMessage parser, captured Discord SQL adapter, historical room production fetch, authenticated numeric Kakao parser and matched room/page production fetch. Populated mail, broad Discord eligibility/producer, legacy RFC3339 and NDJSON framing remain separately synthetic behavior coverage. Do not narrow acceptance to prior live counts or to three fixture sources if numeric capture is unavailable.

After parent release and numeric prerequisite: apply minimal test/fixture patch, scoped rustfmt and LSP, then ONE focused `cargo test --offline -j2 --test test_inbox_sources` in existing target-w4 under parent's integration scheduling. No ignored live replay, global Cargo/clippy/build, source fetch beyond stated prerequisite, model, daemon, env file, git or source-state mutations. Parent owns next combined gate. Receipt records new fixture/lineage hashes, unchanged historical accepted proof hashes, actual focused exit/result, owned listener/tasks/DB cleanup, and post-run source hash stability. This preparation itself runs none of those checks.

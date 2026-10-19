# Todo23 source repair ready

Release gate: parent completes 8/9 cleanup and explicitly releases the QA source freeze. This document is preparation only. No source/test edit, Cargo, fetch, or harness execution occurred. Proposed edit scope is exactly `src/inbox/sources/kakao.rs` and `tests/test_inbox_sources.rs` under `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Evidence and contract

Real GET messages receipt: `task23-capture-20261011T161359754Z/lineage.json`, operation `messages`, HTTP 200, count 50, raw SHA-256 `f3285534c6b83bdf0831d22fa804694244341e14bca3f22d28983f3b3f884f1a`. Recorded schema includes numeric `time` and `sendAt`. Archive contract `/Users/indo/code/project/scripts/kakao-archive.ts:21,47-49,94,121-129` uses `time` as Unix seconds and `time * 1000`; logId is ordering/cursor identity, not timestamp. No evidence authorizes guessed sendAt or snowflake timestamp decoding.

Current parser `kakao.rs:92-97` only looks for RFC3339 strings under createdAt/created_at/timestamp/date. Missing these returns Err on the first eligible message; it does not silently skip individual rows. `fetch_with_client` propagates Err, so collect_all reports a source error and preserves its prior cursor, committing zero items for that failed source. No live parser/harness was run: measured current dropped count is UNKNOWN, not 50. The 50 count belongs to a direct HTTP response; staged two-message projection also lacks accepted string timestamps.

Staging omitted numeric values before raw buffers were cleared. Do not invent a time value and call it captured. Numeric test values below are explicitly synthetic contract inputs; the schema/raw digest authenticates field presence/type, not those synthetic values. Existing real-empty mail fixture is valid lineage; existing populated mail schema fixture stays synthetic, separate from `fixtures/mail.json` in capture staging. Do not replace the populated product fixture with empty data merely to claim lineage.

## Hunk K1: production numeric timestamp with retained compatibility

Path `src/inbox/sources/kakao.rs`; replace only the timestamp/received block inside `parse_messages` after text extraction:

```rust
let received = if let Some(time) = message.get("time") {
    let seconds = time.as_i64().ok_or_else(|| {
        OmonError::ToolExecution(
            "PirateTalk message time must be integral Unix seconds".into(),
        )
    })?;
    DateTime::<Utc>::from_timestamp(seconds, 0).ok_or_else(|| {
        OmonError::ToolExecution("PirateTalk message time is out of range".into())
    })?
} else {
    let timestamp =
        optional_string(message, &["createdAt", "created_at", "timestamp", "date"])
            .ok_or_else(|| {
                OmonError::ToolExecution("PirateTalk message missing timestamp".into())
            })?;
    parse_date(&timestamp)?
};
```

Invariants: numeric `time` is primary; accept signed integral JSON seconds representable by chrono; no float rounding, string-number coercion, millisecond heuristic, current-time default, logId derivation, or sendAt fallback. Present invalid/null/out-of-range time fails even if an RFC3339 alias exists. Only absent time uses the existing string contract. Keep parse_date and existing fixtures/HTTP RFC3339 cases intact; those consumers explicitly rely on createdAt. Preserve cursor floor filtering before timestamp parsing.

## Hunk T1: deterministic numeric/compatibility regressions

Path `tests/test_inbox_sources.rs`; add ordinary parser tests adjacent to `kakao_production_parser_and_per_room_cursor`:

- Synthetic real-shape contract row: string id/logId, numeric time `1_700_000_000`, placeholder text/authorName, no RFC3339 aliases. Assert one item and `received_at.timestamp() == 1_700_000_000`; oldest/newest remain message ID, not time. Label the timestamp input synthetic in test code.
- Numeric time plus conflicting createdAt proves numeric precedence. Independently preserve valid parsing for each existing string alias when time is absent (table of createdAt/created_at/timestamp/date).
- With valid createdAt also present, reject time values `1.5`, `"1700000000"`, null, `true`, `i64::MAX`, and unsigned JSON integer `9_223_372_036_854_775_808`. Assert error and no returned items; no narrowing/overflow/default.
- Only sendAt, with neither time nor existing alias, must fail. A row at/below cursor floor is still skipped before its timestamp is inspected.

Do not mutate staged provenance or existing captured fixtures to insert synthetic time. Do not pin prose/error wording; assert behavior and typed result failure.

## Hunk T2: all-three-source collector continuation

Replace `kakao_bad_endpoint_fails_clearly_without_other_source_dependency` at `tests/test_inbox_sources.rs:133-142` with `kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor`. Extend imports for `async_trait`, AtomicUsize/Ordering, `collect_all`, `PrefilterRules`, InboxItem/InboxSourceKind and crate Result/OmonError. One test-local fixture source follows FakeSource in `tests/test_inbox_store.rs:10-29`, restricted to Mail/Imessage; fetch invokes production parsers on existing synthetic sanitized fixtures and increments a per-source counter. No external CLI from those fakes.

Create migrated in-memory DB; seed old Kakao JSON cursor; seed an other-author DM via existing InboundEvent -> MessageSearchIndex::index_inbound pattern, then use actual DiscordSource::with_ids. Use actual KakaoSource at `http://127.0.0.1:9`. Under a 10-second outer timeout, collect `[kakao, mail_fake, imessage_fake, discord]` once with PrefilterRules::default (not default_filters which writes a file).

Assert four ordered results; Kakao error and preserved prior cursor/error; each other kind no error, one inserted row and expected persisted cursor; each fake called exactly once. Compare JSON cursor maps, not HashMap serialization order. Put Kakao first to expose fail-fast regressions. This is real failing Kakao + actual SQL Discord + fixture-parser CLI fakes; it is NOT three live-source fetch evidence. No sleep, retry, global env mutation, source DB, model, or external HTTP. Port 9 unexpected listener is a reported precondition conflict, never a service to stop.

## Hunk T3: counts-only ignored harnesses

In all four ignored live_* functions at `tests/test_inbox_sources.rs:525-583`, retain actual adapter construction and Discord explicit copy requirement/read_only(true). Replace fetch unwrap/cursor output with the same inline pattern, substituting fixed source name:

```rust
let result = tokio::time::timeout(
    std::time::Duration::from_secs(60),
    source.fetch_since(None),
)
.await;
let (items, _cursor) = match result {
    Ok(Ok(value)) => value,
    Ok(Err(_)) => panic!("QA fetch failed source=kakao"),
    Err(_) => panic!("QA fetch timeout source=kakao"),
};
eprintln!("QA_COUNT source=kakao items={}", items.len());
```

Zero count succeeds. No cursor/IDs/snippets/author/body/URL/underlying error Debug output. Fixed failures remain failures, not zero-count passes. For Discord also bound the connection await and replace `.expect(...)` on the returned connection error with a fixed failure message so SQLx cannot print private copy details. Keep missing config a failure; no migration or fallback live DB. Do not add a prose-output test.

## After release only

Apply the two-file patch, obtain diagnostics, then one focused `cargo test --test test_inbox_sources` under parent's existing target/environment. Parent controls integrated gates. No Cargo now, no full-suite replay, and no ignored live fetch until parent authorizes post-freeze harness QA. Reconcile baseline source hashes before editing concurrent work. Required result: numeric seconds parse correctly; existing RFC3339 tests stay valid; all-three collector continuation passes deterministically; ignored output is counts-only. Real source capture lineage and synthetic regression data remain visibly separate. Todo23 remains NOT confirmed by this preparation.

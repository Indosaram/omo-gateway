# Task23 fixture wiring receipt

Status: fixture wiring ready; focused inbox target is PARENT-OWNED PENDING in the grouped gate after task14 persistence settlement. This lane will not launch Cargo or await the earlier settlement question. No passing result claimed.

Allowed changes: tests/test_inbox_sources.rs plus nine files under tests/fixtures/inbox/captured-task23. No production adapters changed. Exact historical sanitized projections copied byte-for-byte; fresh numeric room/message projections copied byte-for-byte from task23-kakao-numeric-20261011T175050750Z. provenance.json records mappings, operations, capture statuses/times, raw digests and sanitized hashes. Existing synthetic populated mail and supplementary synthetic framing fixtures remain unchanged.

Seven new tests cover real-empty MailSource parsing/cursor stability, captured iMessage JSON parsing, captured rows re-serialized into observed NDJSON through the production structured decoder, captured Discord indexed-row extraction through actual SQL adapter on owned in-memory DB, captured numeric Kakao parser, historical captured room fetch and matched fresh captured room/numeric-page fetch through actual KakaoSource on ephemeral owned loopback. Empty pages are explicitly synthetic paging controls, not captured history. Loopback uses subscribed request events, bounded timeout, graceful shutdown and joined JoinSet; DB closes explicitly.

Scoped rustfmt --edition 2021 --check tests/test_inbox_sources.rs exited 0. All formatting edits used apply_patch. LSP hook and explicit diagnostics requests repeatedly timed out at daemon; diagnostics unavailable, not clean. No daemon change or global check attempted.

Shared-state observations: events.rs remained c4ac34a772bd087543e26fbe6473a74ad00ea33416a93bca09f4daaae942ad68. actor.rs changed from 4f980c4856826f7aa0e7f5811577ab4cb19766ab0790b2a199092207fa471be6 to 1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3 during this lane. No gateway Cargo process was observed at scoped preflight; unrelated processes were left alone. Parent owns source settlement and the single grouped invocation; earlier settlement question is superseded.

Expected inventory: seven byte-matching sanitized capture projections, one NDJSON derivative, one provenance manifest: nine files total. Expected fixture row counts: mail items 0; iMessage chats 1 object; iMessage history JSON 2 rows and NDJSON 2 identical projected rows (not four distinct messages); Discord indexed row 1 object; historical Kakao rooms 1; fresh matched Kakao rooms 1; numeric Kakao messages 1, UTC-valid count 1. Seven added tests plus nineteen existing ordinary tests give expected 26 ordinary passes, 4 live tests ignored, 30 total. These are expected counts, not an executed result.

Settled hashes:

```text
c0dbc0d76dcd16d5ac163d8ea62e53d1ac9cdc91ad10de7339fa9f5a80bd7c2d tests/test_inbox_sources.rs
a24c33f29caca7b54381acfb24b03094b9abe8f4bf22c4e8ae475130b4d358cf discord-indexed-row.json
0b5fe088186e5ac0e9335ffac8409d77af6483b069b40c189ddacb3fc1017464 imessage-chats.json
314eddaa6fdaa8dc423f9465c48d5607ec49b182bb0b96b1bee0df05b421a65a imessage-history.json
a6b80e89e23b7d0a5330f5c72c3e72e48628c4b3ba5d300569e3de4559cb21d2 imessage-history.ndjson
936b20cd30da29510a1332a2d0a1202e6eb1b13e6a9fcaf9c1eb3302a2f0c65b kakao-messages-numeric.json
014f2f102ac0c0dc5d48deddf3cbe4a22a7bb75d31cdefc42896208cfd9878e0 kakao-rooms-historical.json
5f5d2eaf4ec13aacc924c8898ad61e0283ece6abfe48311776fdb747e0ea4f9e kakao-rooms.json
5cdbe9fb42368950639ffcccdd290245d610998e531e2830e6d4f54e79d58ebb mail-empty.json
aa47955b0e39d43b53cc689d4abc23dc9e70a661154af678d71dc51a397253db provenance.json
```

Preserved evidence hashes:

```text
bfb61f87bda949eca6edc866da1757400c7e77c7d50f7f0895a049c90f147cea task23.log
adeeb5a92b25cf8908878d4844d8cea8e080486dace8924195f44481a02f676b task23.json
709dcb4863e92dd0583152fed05761d07af102fb28c8cb0ece8c4487298bea1f numeric capture lineage.json
6d33e906d3eac4c954d1d0be9fc076dede42b90ae88101870d69c62ec261a577 historical lineage.json
ba240aaaf5125bc7a224cf7a8d8aaf745afbd2e4bdb8f003233335138908fc60 historical mail-lineage.json
```

No captured value invented. NDJSON derivative contains unchanged staged rows but is not original raw bytes. No new live fetch, ignored replay, Cargo invocation, full build/clippy/global tests, model, daemon/env/git operation, ownerDB access or external Discord. No runtime scratch/listener was created because focused tests remain unrun. Parent grouped gate will include test_inbox_sources exactly once and provide numeric result/cleanup evidence afterward. Preserve this expected-count receipt separately from that actual result. Accepted four-fetch proof remains unchanged and needs no rerun after test-only fixture wiring.

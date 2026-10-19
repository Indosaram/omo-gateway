# Todo22 current-source focused QA

Result: **PASS, scoped evidence ready for independent review**, with the coverage limitations below. This is not closure of live-source or websocket QA, nor a whole-plan completion claim.

## Execution and prior evidence

Read the full todo22 section of the main plan (lines 401-410), the complete `tests/test_inbox_store.rs`, collector/prefilter/watcher implementation, migration, and existing task22 receipts before execution. The named `task-22.log` was a historical implementation receipt explicitly saying Rust tests were NOT RUN. Its original contents are preserved before the current run appendix; original SHA-256 was `5600b4caad7259dfdf352fca2924b351979349e677eccd41a3aeff9edcf05966`.

Canonical `batch-20261011T135508Z-full.log` lines 791-805 already contained 10 passing inbox tests (0 ignored, 0.91s), but six manifest inputs had changed since that batch. Its SHA-256 is `c5ae061e132fe9a8a07cc964a09e2f73021d404a13ca59d380b6942750d954d1`. The historical batch summary records full exit 101, fmt exit 1 and clippy exit 101; it is not a clean current global gate. No global gate was repeated here.

One pre-run process check found no active Cargo/rustc command referring to dori-work or target-w4. One permitted focused command was launched through async monitor `mon_77MBQQEBQ6CC0CEM` / `bash_8`, with `timeout_ms=1800000`:

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work
CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4 CARGO_BUILD_JOBS=2 cargo test -j2 --test test_inbox_store --all-features
```

Observed command exit **0**; **10 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out**. Compilation finished in 6.52s; tests in 0.27s. Exact test names and observed output are appended to `task-22.log`. Pre-run check was at `2026-10-11T15:22:29Z`; post-run hashes at `2026-10-11T15:23:16Z`. No warning or error appeared in this focused run output.

## Existing assertions and outcomes

| Behavior | Assertion-backed outcome | Reference |
| --- | --- | --- |
| Repeated collection | Two collects return no error; stored row count is 1; snippet length is 1000 after input length 1100; cursor is `cursor-1`. The test does not assert the returned duplicate counter. | `tests/test_inbox_store.rs:45-72` |
| Prefilter fixture collection | Seven fixtures produce 3 stored rows and aggregate drop count 4. Promotions mail, OM room noise, 114 SMS, and excluded room are the drop fixtures; DirectChat, ordinary GitHub notification, and owner mention are acceptance fixtures. All seven collection results have no error. | `tests/test_inbox_store.rs:75-139` |
| Notification classification | Persisted ordinary GitHub notification has class `fyi`. Changing its title to `Review requested` yields `direct_class=None`. | `tests/test_inbox_store.rs:129-139` |
| Source failure isolation | Broken mail result contains `fake failure`; mail cursor stays `old`; subsequent Discord result has no error and one Discord row is stored. | `tests/test_inbox_store.rs:143-181` |
| Failed upsert cursor invariant | SQLite BEFORE INSERT trigger raises `injected upsert failure`; returned error contains that text; mail cursor remains `old`, not `new`. | `tests/test_inbox_store.rs:185-211` |
| Invalid source record | Wrong-kind item returns error; mail cursor is empty/unadvanced and its recorded error equals the result; zero inbox rows stored. | `tests/test_inbox_store.rs:224-247` |
| Additional pure rules | Explicit mail category reasons; excluded/noise/unsupported Kakao rules; DirectChat/MultiChat and owner/keyword passes; SMS/ad rejection; Discord mention/DM/owner-reply passes and owner/bot/chatter exclusions. | `tests/test_inbox_store.rs:251-436` |
| Event dispatch and cursor separation | Subscribed progress channel observes one inserted item for each source-specific event; timestamp cursor stays unchanged while numeric watcher rowids/arguments are checked. | `tests/test_inbox_store.rs:440-581` |

These numbers are values enforced by the passing existing assertions, not new SQL dumps or independently invented probe scenarios. The collector's item writes, filter counts, decisions and successful cursor update share a transaction (`src/inbox/mod.rs:204-296`). Source error recording leaves existing cursor values intact (`src/inbox/mod.rs:298-310`).

## Coverage findings for review

1. The plan explicitly asks for a GitHub **review requested** mail to pass collection. The existing integration fixture collects an ordinary `Issue notification`; the review variant only asserts `direct_class=None`. Persisted acceptance of the review variant is therefore not directly demonstrated by this suite. No fixture or assertion was changed to hide this gap.
2. The persisted prefilter count assertion checks the aggregate sum of 4, not the individual `(day, source, rule)` counts. Pure rule tests identify drop reasons, but per-rule persisted counts are not individually asserted. Returned inserted/duplicate/drop counters are not asserted by the seven-fixture or dedup tests.
3. The fault-injected upsert test asserts the error and unchanged cursor, not a post-failure item count or rollback after an earlier successful item in the same transaction. No stronger rollback scenario is claimed.
4. Event test progress receives and watcher join have no local bounded timeout. This run completed; the test process had the external 1800-second monitor deadline. No sleep-based timing probe was added.

No failing assertion or product defect was observed in this focused run. Timeout/hung-source, malformed filter configuration, actual source adapters and live websocket behavior remain outside this evidence's demonstrated assertions.

## Scratch SQLite and cleanup

`db()` uses `Database::connect("sqlite::memory:")` (`tests/test_inbox_store.rs:40-42`). The real database constructor runs migrations with one SQLite connection and skips WAL mode for in-memory URLs (`src/storage/db.rs:158-191`). The suite exercises real SQLite queries and the injected trigger against synthetic fake-source data. No persistent scratch database, WAL or SHM file is created by these fixtures; SQLite memory is released when the test process exits. A persistent SQLite file hash is **not applicable**, not missing evidence.

The defaults-path test creates `inbox-filters.json` only inside a `tempfile::TempDir` (`tests/test_inbox_store.rs:215-221`); the guard performs cleanup on scope exit. No manual scratch deletion was needed or performed. No production/source database was used and no personal message data was collected.

## Exact source and executable provenance

All 191 paths listed in canonical `batch-20261011T135508Z-post-manifest.sha256` were hashed before and after this focused run. Every path's hash was unchanged during execution. Serialize them in the canonical listed order as `<sha256>  <path>\n`; current pre/post digest is `59beefea246726d79453775c2a62a6e149d21519302b695864bf2d70faa302db`.

The canonical post-manifest file SHA-256 is `5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777`. To reconstruct the current 191-path manifest exactly, retain that file's paths/order and hashes except for these six replacements:

```text
685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c  docs/AGENTS_DELEGATE.md
9c31fd7a7a16585fd1398797609c020e40eb75a16dd38b1c7dd949732725d9bd  src/agent/backend.rs
83c41285631ebe74eb63d91f77ccf6c967464139cd1622b43f8a616783d7136c  src/lanes/runner.rs
2aef0dd17a3182734bb6ba7cc88c40ad129d31d8cf95f92847a3c33037995a48  tests/test_inbox_sources.rs
b6b255f85049762a2a900c49cab708c6c2e14375dd1f65642da73afb6a25bb1a  tests/test_lane_reopen.rs
c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea  tests/test_omomeow_delegate.sh
```

Focused inputs and executed artifact SHA-256:

```text
0a38435520de60cea3ebc1a16b4f140bb8e498431bc8b25f7fbf039337aad01f  tests/test_inbox_store.rs
32e9dae3f78710cda764dca00735e3b412222f8eb0b0702afea3f100345e4bd5  src/inbox/mod.rs
027cb62153df09a8aea9b7464c75561f4dddcf47bc2b10e5525ea6847b960179  src/inbox/prefilter.rs
a1e2155306e67d7598857e73f44f3774b5c2983137f2c87d068d7e3936c096b0  src/inbox/watchers.rs
904e3012468abae066c3c28ee53f97123d4089170f4c5b1fbb862a6284692d55  migrations/0034_inbox.sql
53b701cdeba764fcdf4642e995ad5af340787e0ec4b914255eb1fdda08e39223  src/storage/db.rs
78d02103a7723b731a50f3c1d323639ca9d79b828db459f5f51bc4a7d242f13d  Cargo.toml
6129c4128398f0bbe50d9b2b69d725648fae5daa76a2915db2d6784fc3cfcba3  Cargo.lock
c7a1fc2b1c696647c15f32073ece0cc2e74405318cfc52c2d92249f6d72e8a73  target-w4/debug/deps/test_inbox_store-130565096f6e91dd
```

Only `task-22.log` and this QA markdown were edited in this unit. No product/test/schema edits, full-suite/clippy rebuilds, live-source/network/model/Discord/service actions or git operations were performed. Existing historical receipts remain historical; the current scoped pass does not erase their global failures or claim full todo22/live QA closure.

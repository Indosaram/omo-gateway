# Task12 local post transport harness ready - execution held

Owned new file: tests/test_lane_post_transport.rs
SHA256: a347f2debc764f63be06f103f20dcbb761076134112be0ef251768632803bde5
posts.rs SHA256: 7cc0a2caf6a8ccc288f9001737e2c59ddecc83993d5232ffc00028ea2ddd9230
DiscordThreads helper SHA256: 1fae41cfbc3ad5598c0e729b408939181c504a2fcb4501be63afaac255002d0a

No PostWorker constructor is needed: production posts::drain accepts &dyn PostSink, and existing DiscordThreads::with_http injects the actual Serenity Http sink. No public abstraction or product edit was added.

## Scenarios and proof boundaries

Two tests use owned tempfile SQLite, real LaneActivity::captured_item, real pending/sending/sent/uncertain JSON ledger, actual posts::drain and actual DiscordThreads sink. REPORT and completed item constants are declared replay of GhvMrT milestone at16:46:25, original thread01a12bdb-1fe5-7345-ae81-738e3573330f. This is reuse of proven captured machine seed, not a fresh capture/model turn. Fixture destination300 substitutes only in the newly owned DB; original retained scratch DB is untouched.

1. Actual loopback HTTP acknowledgement: subscribe request channel and response gate before worker. Peer reads exact POST /api/v10/channels/300/messages and full Content-Length JSON. While response is gated, assert persisted sending with no receipt. Inspect content/nonce/enforce_nonce/safe allowed_mentions. Release actual200 Message JSON id400, await actual worker success, assert sent/message_id400. Close pool, reconnect file DB, recover/drain and assert state/id/nonce unchanged, zero sends and no extra accepted request.
2. Ambiguous accepted-request closure: peer observes full request then closes without sending response. Actual HTTP failure drives ledger uncertain/error/no message ID. Reconnect/recover/drain and assert no retry. Peer continues accepting/reporting requests until cleanup, so duplicate sends cannot be hidden by a stopped server. Any internal transport retry also fails request-count assertion; no claim of HTTP exactly-once is made.

Reqwest no_proxy, redirects disabled, HTTP5-second bound; loopback ephemeral port and non-forwarding raw peer. Fixture token local client only; no gateway env/liveDiscord/model/ownerDB/daemon. Catch-unwind + total20-second bound, abort/join server and closed-listener check run on assertion failure/timeout too. Owned DB pool closes and tempfile cleans up. Output TASK12_LOCAL_POST_RECEIPT includes source/test hashes, original seed IDs, HTTP trace, ledger states, request count and cleanup only after checks succeed. Parent must attach actual numeric test process exit/PIDgone.

## Held execution and remaining task12 QA

Rustfmt --edition2021 --emit stdout syntax parsed exit0 with empty stderr; output applied through apply_patch. A later source inspection found Database::close consumes self; test uses pool.close for first reconnect so final cleanup still owns Database. LSP error diagnostics returned No diagnostics found before that one-line correction; no compile/type-test proof is claimed. Cargo not run. Manifest has default integration autodiscovery; no shared Cargo edit needed.

Combined parent release after10/14/23 stable:

cargo test --test test_lane_post_transport -- --nocapture

Capture actual stdout/stderr, numeric exit and cleanup in unique attempt artifacts before extracting receipt markers. Do not create a successful task12 runtime receipt from this source-ready document. Preserve25-pass test_lane_events receipts as scoped historical evidence, not this new target's result.

Missing question pause/resume, stale MockClock restart and3-error failed/dead cases remain with existing test_lane_events/test_lane_runner targets and their parent combined run. This target fills only actual HTTP ack + persistent restart/no-retry + ambiguous transport boundary. No real backend question or live Discord delivery is demonstrated. Product defects from execution are queued to parent; posts.rs/runner/shared14/Cargo files were not modified. No Cargo/process/server/DB runtime was started in this preparation.

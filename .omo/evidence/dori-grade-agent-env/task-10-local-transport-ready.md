# Task10 local transport QA harness - source settled, runtime held

Owned test: tests/test_lane_thread_transport.rs
Test SHA256: 6a0dddcacf5fda802dd84f91ad8e4bc79f515d6e14a17c83cf6d7f2ccd0729d2
Helper SHA256: 1fae41cfbc3ad5598c0e729b408939181c504a2fcb4501be63afaac255002d0a

Rustfmt --edition2021 --emit stdout parsed the source with exit0 and empty stderr; its formatted output was applied through apply_patch. LSP fresh diagnostics timed out after3000ms, so type diagnostics are unavailable. Cargo/tests have NOT run. No real/local HTTP server has started. No task-10.json or task-10-fail.log execution receipt is fabricated.

Cargo.toml has no autotests=false (edition2021); existing integration test autodiscovery registers tests/test_lane_thread_transport.rs. No Cargo registration/edit is needed and none was made. Task14's manifest/product files remain untouched.

## Combined parent run, only after task14 stable and release

Run in dori-work, using the parent's agreed target/Cargo batch, not a competing build:

cargo test --test test_lane_thread_transport -- --nocapture

Capture untouched stdout/stderr and actual process numeric exit in a unique evidence attempt directory. Test emits TASK10_LOCAL_RECEIPT=<JSON> only after all assertions and server cleanup succeed; TASK10_LOCAL_FAILURE=<JSON> describes the intentionally tested typed404/code10003, not a failed test. Parse these machine markers, require Cargo exit0 and test pass, then create named task-10.json and task-10-fail.log from actual output. Attach source/test/manifest/dependency hashes, numeric Cargo exit and test process PIDgone. Preserve task-10.log and both prior reviews. Existing named artifacts must be archived before replacement; never overwrite historical evidence silently. A compiler/assertion failure produces a failure receipt with actual exit, not a success artifact.

## Observable checks

Actual DiscordThreads::with_http and Serenity HttpBuilder::proxy route only to an ephemeral127.0.0.1 recording peer. Reqwest no_proxy and redirect none prevent ambient proxy/redirect forwarding; fixture token is isolated in this test client. No gateway environment/token/config changes. Ratelimiter disabled only for this owned local fixture. Installed request.rs replaces https://discord.com with supplied proxy base, preserving /api/v10 paths.

Request channel is installed before helper calls. Exact11-request method/path order is asserted; unexpected routes fail locally. Production helpers create starter/thread, GET current name, rename Done, archive, GET archived, unarchive, GET unarchived, post, edit and invalid-channel post. Korean150-character title must serialize capped100-character names on UTF8 boundaries. Message content and safe allowed_mentions (no everyone/roles) are inspected on starter/post/edit/error payload. Returned local IDs, name and archive metadata are asserted. Invalid ID must produce typed HTTP404 and Discord10003. These are local response fixture IDs, not real Discord resources.

Each request has5-second HTTP/event bounds, lifecycle30-second total bound. No sleeps/polling. Catch-unwind and JoinSet abort/join close the listener even on an assertion failure/timeout; successful output requires a failed connection to the closed owned listener. Test creates no DB/model/daemon/externalDiscord resource. Parent captures numeric process cleanup outside the in-process test.

## Scope boundary

Only the new test and this evidence doc were changed. No discord_threads.rs or shared14 file edit, git action, environment file change, external message, model call, Cargo run or service manipulation. Product fixes exposed by combined execution must be queued to parent with failing request/state and source reference, not applied here. Task10 remains open until actual local receipts plus parent reconciliation of historical LIVE requirement.

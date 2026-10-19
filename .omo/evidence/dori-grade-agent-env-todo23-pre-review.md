# Task 23 final bounded source review

recommendation: APPROVE
sourceVerdict: source-ready-tests-QAglobalpending
approvalScope: Only the five previously identified source blockers and their grouped fixes. This is not completion approval for todo 23 or the integrated system.
verification: tests UNRUN; Cargo UNRUN; live QA UNRUN; global gates pending
blockers: []

originalIntent: Deliver task 23's read-only source adapters and the actual Discord author/reply metadata seam, with meaningful production-path regressions and honest QA evidence.

desiredOutcome: A source-ready handoff for coordinator integration and later verification, without claiming that source inspection proves passing builds, tests, or real-source behavior.

userOutcomeReview: The five prior blockers are resolved at the inspected source level. Actual Discord conversion now fills author_id independently of its guild routing key, storage prefers that author, and both parent and reply are indexed through the production index method in the regression. NDJSON fixture bytes pass through the same decoder used by CLI output. The local HTTP paging test reads complete headers under a bounded test timeout and compares parsed cursor maps. The missing SessionKey literal field and unnecessary mut are gone. Explicit live Discord QA now fails when its required copy configuration is absent. No new generic audit was performed.

All relative paths below resolve under `/Users/indo/code/project/omon-gateway-wt/dori-work`. This report supersedes the previous rejection for these source blockers.

## Previous blocker dispositions

### B1: Actual Discord author/reply producer seam - resolved in source

Criterion: T23-DISCORD.
Evidence: `src/models/events.rs:181-184,269-271,293-300`; `src/discord/adapter.rs:1785-1797`; `src/storage/message_search.rs:212-240`; `tests/test_inbox_sources.rs:342-399`.

InboundEvent has optional serde-defaulted author_id and referenced_message_id fields. The actual Discord adapter assigns event.author_id from message.author.id and obtains the reference from message_reference or referenced_message. This is inspection of the real conversion code, not an inference from a test builder.

Storage now prefers the nonempty event.author_id, falling back to the session routing identity, and persists referenced_message_id into indexed metadata. Guild session.user_id remains empty, preserving conversation routing semantics.

The regression uses SessionKey::new with a guild and asserts its empty routing user_id. Both owner parent and other-author reply then pass through index_inbound; the parent is no longer inserted by handwritten SQL. It reads back the parent's indexed author and checks that the source selects the reply as owner_reply. Events still use builders rather than a live serenity Message conversion, so this test alone is not a test of Discord conversion. The independently inspected real assignments plus this meaningful index-to-adapter regression are sufficient to close the bounded source finding. Runtime producer wiring remains coordinator-owned and unverified.

### B2: Missing SessionKey field and unnecessary mut - resolved in source

Criterion: T23-GATES, source defect only.
Evidence: `tests/test_inbox_sources.rs:73,353-360`.

The test now uses SessionKey::new, which initializes bot_id, instead of the incomplete literal. The history binding is immutable. The earlier unused sqlx::Row import had already been removed. This closes the specific source-visible defects, not the compilation/clippy gate itself; those commands remain UNRUN.

### B3: Missing live QA configuration falsely passing - resolved in source

Criterion: T23-QA / explicit required QA must not silently pass without fetching.
Evidence: ignored live test section in `tests/test_inbox_sources.rs` (read through the file's end).

OMON_QA_DISCORD_COPY_DB is now required with expect. An explicitly selected test without it fails instead of returning success. The connection still uses read_only(true), and there is no fallback to the owner's database. The other live adapter tests no longer assert a nonempty cursor for legitimate empty results and now emit item counts and cursors without message text. These are unexecuted test harnesses, not QA receipts. The configured path being a genuine QA copy remains an execution precondition.

### B4: Nondeterministic HTTP paging regression - resolved in source

Criterion: deterministic production paging regression.
Evidence: `tests/test_inbox_sources.rs`, kakao_event_driven_paging_loop.

The whole test operation is bounded by tokio::time::timeout. The fake HTTP server accumulates request bytes until a header delimiter rather than assuming one TCP read contains the request. The source fetch still traverses descending pages, empty termination, a second room, and a repeated fetch using its returned cursor. The final assertion compares parsed maps rather than HashMap serialization order. There are no sleeps or timed polling. The prior reversed paging comparison would remain observable through this test. No run result is claimed.

### B5: Array-only test mislabeled NDJSON - resolved in source

Criterion: T23-FIXTURE / production byte-decoding regression.
Evidence: `tests/fixtures/inbox/imessage-history.ndjson`; `tests/test_inbox_sources.rs:113-123`; `src/inbox/sources/mod.rs:13-38,65`.

The new fixture contains two separate JSON object lines, not a JSON array. include_bytes feeds those actual bytes through decode_structured_output, then the decoded result through IMessageSource::parse. run_json invokes that same decoder on subprocess stdout. Removing NDJSON decoding would therefore affect the regression rather than leaving an array-only test green. Fixture framing and production-path use were inspected directly. This does not independently authenticate fixture provenance or reproduce the installed CLI's live output.

## Scope and evidence boundaries

Latest handoff read completely: `.omo/evidence/dori-grade-agent-env/task-23-wiring-request.txt`. Its source-ready/formatting prose was treated as a claim, not a command result.

Checked for this pass:
- The history/NDJSON regression section and the complete parent/reply, paging, and ignored-live sections of `tests/test_inbox_sources.rs`.
- Actual author/reference declarations, defaults, and builders in `src/models/events.rs` through exact source search results.
- Actual Discord conversion assignments in `src/discord/adapter.rs` through exact source search results.
- `src/storage/message_search.rs:207-243` for author priority and reference persistence.
- `src/inbox/sources/mod.rs:1-90` for the production decoder and run_json call.
- Complete raw `tests/fixtures/inbox/imessage-history.ndjson`.

Earlier full-plan and adapter reviews are not represented as fresh complete reads. No whole-repository audit, callback rewiring, or task 22 prefilter review was added. Coordinator main/inbound watcher callbacks, shared event struct-literal integration, task 22 eligibility consumption, and global verification remain pending. This bounded source approval does not assert those are integrated or complete.

## Direct maintenance/overfit check of these fixes

The new author test no longer hides the owner-parent producer behind a SQL seed, while actual Discord assignments were inspected separately. The NDJSON decoder extraction is shared with production rather than being test-only parsing. The HTTP regression reaches the production paging loop and no longer depends on socket chunking or map order. Missing QA configuration cannot create false green output. No deletion-only test, prose-pinning assertion, or new generic abstraction was required for approval. These source observations apply only to the grouped fixes; no passing test counts are claimed.

The remove-ai-slops/programming skill files remain unavailable in this session; supplied criteria were applied directly to these changes. No separate updated skill-perspective review artifact was supplied, and this report does not pretend otherwise. No JS eval/status tool is available; the explicitly requested main report path is used.

## Verification and cleanup

Tests UNRUN. Cargo UNRUN. Live QA UNRUN. Formatting, diagnostics, focused tests, shared-driver/batch checks, and global gates are not confirmed by this source review. Executor wording such as source-all-verified is not adopted. The next operational step belongs to the coordinator's shared driver after the remaining batch sources are ready; no command is started by this reviewer.

Cleanup receipt: Only artifact/source reads, source searches, and this report rewrite. No product edits, no task 22/main/watchers changes, no Cargo, no DB opens, no live requests, no services or background jobs. No cleanup resources were created and no personal message bodies are retained in this report.

# Task14 integration source receipt

Status: SOURCE_SETTLED FOR PARENT COMBINED GATE; NOT RUNTIME CONFIRMED. Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Scope and implementation

Owned product paths: `Cargo.toml`, `src/main.rs`, `src/models/events.rs`, `src/models/mod.rs`, `src/storage/db.rs`, `src/multiplexer/actor.rs`, `src/multiplexer/router.rs`, `src/drain_control.rs`, `src/lanes/mod.rs`, `src/lanes/dispatch.rs`, `migrations/0037_lane_event_delivery.sql`, `tests/test_lane_dispatch.rs`. This receipt is the only additional evidence file. No edits to runner, scrub, registry, inbox, test_lane_runner, backend configuration, env files, services, owner databases, or Discord transport.

Ported inspected next-to-work diff hunks for EventOrigin, atomic System idle admission, transcript marker, reaction suppression and User-only indexing. New dispatcher and eight neighboring tests came from inspected next source; two additional tests cover the idle-probe/admission race and legacy/default origin serialization. Migration0037 adds only delivery_reason. Existing newer work runner and producer behavior retained.

Headless main checks the lanes flag before constructing backend tasks and retains required DATABASE_URL. Its private constructor uses real OmoBackend.with_pool, SessionMultiplexer, existing profile/channel parsers, and a concrete local Stream sink. No dummy Discord token, SharedDispatcher, CapturingDispatcher, Discord client, daemon supervisor, inbox collector or ScaleToZero is constructed on that branch. Final output acceptance logs delivery=local_transcript; assistant persistence and obligation settlement remain backend-owned. Uploads, approvals and other transport operations explicitly return errors.

Headless drain admission is reversible: initial scan before admission, marker true keeps events queued, removal permits delivery; headless does not stop merely on marker activation. Ordinary main still selects the marker as its production shutdown trigger. Both branches own the cancellation-aware watcher and dispatcher tasks. Actor shutdown uses the existing shared CancellationToken pattern rather than a new generic resume executor: fence admission, cancel active run_cancelable and await it with a 20-second bound, retain remote binding/resume_pending, reject queued acknowledgments, finish flush, then signal the actual actor finished state. Runtime task/actor joins have a 30-second bound. Pool close follows successful joins; failed cleanup returns without explicitly closing a pool still needed by outstanding actors. Actor final flush errors propagate through handle shutdown evidence.

## Validation and parent commands

Final scoped rustfmt check exited 0 for every owned Rust path, using `--edition 2021 --check --config skip_children=true`. AST scans of `InboundEvent { $$$ }` and `crate::InboundEvent { $$$ }` across src/tests found the four explicit event sites; each includes origin. InboundEvent::message sets User. Worker23's missing-origin finding at main455 is fixed in this ownership.

LSP returned no diagnostics for main/router/actor/dispatcher/events/storage/drain/dispatch tests at an intermediate snapshot. Later LSP caught missing DeleteMessage in the exhaustive local sink; an explicit refusal arm fixed it. Final main LSP attempt timed out; actor/router fresh checks also timed out after final-flush error propagation. Do not describe final LSP as clean. Parent combined compile gate is still required.

One premature focused monitor launched before the updated parent instruction reached this worker. It was stopped immediately: monitor mon_YD6WHGSZG34H83JW, terminal bash_12, killed exit1, output only `Compiling omo-gateway` and build progress474/478. No tests completed and no passing result is claimed. The canceled command's `headless` filter would also have filtered dispatcher tests, so it must not be used as the dispatcher acceptance command. No further Cargo invocation was issued.

Exact headless command for the parent gate:

```sh
CARGO_TARGET_DIR="$PWD/target-w4" cargo test --offline -j2 --bin omo-gateway headless_tests -- --nocapture
```

Exact separate dispatcher target, without a headless filter:

```sh
CARGO_TARGET_DIR="$PWD/target-w4" cargo test --offline -j2 --test test_lane_dispatch
```

Headless tests are in the private main module included by entry.rs: `headless_marker_reply_and_joined_shutdown`, `headless_active_shutdown_joins_interrupt_and_preserves_binding`, `headless_output_rejects_upload_and_approval`, `headless_watcher_cancels_and_joins`. Real loopback WebSocket protocol and owned migrated scratch SQLite are used. The successful turn test checks System transcript, assistant sentinel, lead_turn admission receipt, delivered local obligation, binding and joined shutdown. Active cancellation checks actual interrupt protocol, admission fence, persisted resume_pending/binding and unsuspended state. Ping/Pong synchronizes turn/start ACK consumption before shutdown. Tests are source additions, not execution evidence.

Parent owns ONE combined fmt/strict-clippy/dispatch/headless/inbox/thread-transport gate after source settle. Inbox target must be included there once; worker23 receipt remains `task23-repair-receipt.md`. Worker72's `tests/test_lane_thread_transport.rs` is autodiscovered; no registration was added. No global clippy/build/full suite or actual model QA ran here.

Remaining actual QA: released actual gateway/lane lead wake and reply logs with scratch persisted reply; real filesystem marker refusal/removal at the running gateway surface. Local loopback tests cannot certify live model or Discord behavior. Admission receipt is not turn completion. No new automatic interrupted-System resume guarantee is asserted.

## Final source hashes

```text
4d7d8ad44f283880b2ab87b968f1133451c58a5fc43b1b3e9179100382434022  Cargo.toml
f9df98c5fbd13739669236cfc8115aa90bd907dccfc06e2eb583089cc291b7e4  src/main.rs
c4ac34a772bd087543e26fbe6473a74ad00ea33416a93bca09f4daaae942ad68  src/models/events.rs
3b3119e5cae9a7e9e1ae2ed1f5897cbaa451c2194bc7248185736bf903facd43  src/models/mod.rs
2f41851ccfecbd1348bcd07cad289b4652be79f92b860a419a46806ab602f569  src/storage/db.rs
dc74f1101f50c45f581cc913ef53f48be44bd9a35b53c59c0e92e36e3b3fb19f  src/multiplexer/actor.rs
6a38167b8e26024f068bf2234fa3050c8d22c762112dfeec855257bf497be676  src/multiplexer/router.rs
28ba7dca2c7203ac8a58a6a2437f51401b840977b11283ac397d266cc358425e  src/drain_control.rs
ddb4e4e50336431a3fd9038be084a55d5ab33966682c9af2efe95c50075a4363  src/lanes/mod.rs
c9fcb0d876458a54ad4418e76105b202a21cb5d551d2961890338dd0fb40f325  src/lanes/dispatch.rs
f3d1447a96432e4f83c110538c3b194fc27cd8dc07cb0644ef490e2d0ab57554  migrations/0037_lane_event_delivery.sql
d12a602fce4a64943842e124ceab7e62133766ba4e0e176d725102f4c275a3b2  tests/test_lane_dispatch.rs
```

The git diff stat against HEAD includes substantial pre-existing work; it is not this worker's delta. `git diff --check` reports the existing Cargo.toml blank line at EOF; no unrelated cleanup was performed. Product patch provenance is the applied scoped next/work hunks and lifecycle edits recorded in this session. No git commit or reset occurred.

Read-only scoped tracked diff SHA-256: `1a7a7b6c700955087bc23a8c6cf61dc175ccfd1c40943484570a5ca7788eaf65` (`git diff --` the complete owned list above). It includes pre-existing tracked changes and excludes untracked additions, whose full hashes are recorded separately. Rechecked main/actor/router/dispatcher/test hashes after writing this receipt; all five matched the final source manifest above.

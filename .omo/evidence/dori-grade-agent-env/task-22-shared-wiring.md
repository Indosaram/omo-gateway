# Task 22 Shared Watcher Wiring Receipt

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`
Status: Source handoff prepared; compile and runtime behavior remain unverified pending the coordinated batch gate.

## Owned production wiring

- `src/main.rs` creates one bounded inbox event channel and starts its event collector before any initial collection. It registers iMessage and Zele mail child watchers plus a local Kakao timer, then runs the initial all-source collection and queues a Kakao refresh through the same subscribed event channel. (The all-source initial collection is followed by the event-driven Kakao pass; no second event channel or collector is created.)
- The collector uses a bounded progress channel, watches shutdown while supervising children, signals each local child through a watch receiver, kills children on shutdown, and joins owned tasks. Gateway shutdown sends the inbox shutdown signal once and awaits the collector for up to five seconds before aborting it.
- `src/multiplexer/router.rs` carries an optional per-instance sender into newly spawned actors. Existing multiplexer users default to no sender.
- `src/multiplexer/actor.rs` persists a normal Discord transcript row, awaits `MessageSearchIndex::index_inbound`, and only then emits `GatewayMessage`. The index source uses the owner-provided real `author_id` and `referenced_message_id`; the event producer populates author ID from the actual Discord message.
- `src/discord/adapter.rs` covers the separate active-turn steering path: after its transcript insert it awaits the same metadata index upsert and emits the event only after `Ok(true)`. An index failure is logged and does not falsely emit the wake.
- `Cargo.toml` registers the owner-frozen `tests/test_lane_scrub.rs` target as `test_lane_scrub`.
- `.omo/evidence/dori-grade-agent-env/batch-gates.sh` includes root evidence `*.ndjson` files in its source manifest.

The separate `steer_claimed_event` path indexes and signals after its transcript mirror. The distinct `route_claimed_event_awaiting_turn` path falls through `SessionMultiplexer::route_awaiting_turn` to the same actor `persist_inbound` callback.

## Verification boundary

No Cargo command, formatter, clippy, test, service, or database write was run. The authorized coordinated driver was invoked once, but its original global ownership guard exited 2 before creating run evidence: `ERROR: Cargo/rustc process is active; establish target-w4 ownership before running`.

Read-only ownership evidence: Cargo PID 51798 (PPID 51791) ran `cargo test -p mahoquot-gateway --lib quota::tests::quota_consumer_does_not_dial_direct_after_proxy_build_failure -- --exact` with cwd `/Volumes/T9-Mac/project/mahoquot-wt/cliproxy-remediation-v2/mahoquot-proxy` and `CARGO_TARGET_DIR=/Volumes/T9-Mac/project/mahoquot-wt/cliproxy-remediation-v2/.omo/evidence/task2-cargo-target`. rustc PID 70410 (PPID 70408, sccache) had the same cwd and target. `lsof` reported no target-w4 files open for Cargo/rustc, and a target-specific process-argument check found no active process referencing this worktree's `target-w4`. The driver guard was therefore narrowed to those target-specific arguments; the driver was not retried. An exact-PID exit monitor is armed for Cargo 51798 and rustc 70410.

The first attempt produced no gate logs, status ledger, provenance, or manifests. LSP diagnostics had timed out while loading changed Rust files, so compile status remains unknown. A retry requires the authorized source-writer release and completion of the unrelated monitored Cargo/rustc process. No whole-tree test-pass claim is made here.

## Source files touched by this wiring

`src/main.rs`, `src/multiplexer/router.rs`, `src/multiplexer/actor.rs`, `src/discord/adapter.rs`, `Cargo.toml`, `.omo/evidence/dori-grade-agent-env/batch-gates.sh`.

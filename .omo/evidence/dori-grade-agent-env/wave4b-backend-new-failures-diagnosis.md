# Wave 4b backend new failures diagnosis

Date: 2026-10-12. Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Outcome: both new failure names are resolved by test synchronization only. Product source is unchanged. The full backend target ran once after the fix: 45 passed, 3 failed, 1 ignored; all three failures belong to the canonical baseline. This is not a whole-target green or release authorization.

## Prior released-source evidence

`durability-fixtures-20261011T180128Z-focused.log:125,133` explicitly records both silent-turn tests as `ok`. Its `-baseline-policy.md:7,14` also records both as PASS. That batch had 36 explicit backend passes, eight observed baseline failures and two unfinished historical tests at its monitor deadline; its overall Cargo exit is unknown. Do not describe that batch as green.

`batch-20261011T135508Z-full.log:1033,1038` records both tests passing as well. The sibling handoff identifies ten backend failure names from the todo-2 baseline and zero new failures. The original `task-2-failing.txt` was unavailable in the earlier evidence inspection; the preserved canonical handoff is the baseline authority used here.

`duplicate-final-20261011T184722Z-handoff.md` records a scoped release gate, not another run of these two tests. Its backend invocations covered `completed_` (four passes) and `failed_final_delivery_is_not_acknowledged` (one pass). Thus the earlier durability batch proves the two tests passed; duplicate-final does not independently rerun or certify them. Its build manifests prove the released-source backend identity:

```text
04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2  src/agent/omo_backend.rs
d5f33aa4b2c0c0558ae51d96290a62a681e4e57fb98b55958d4e37e618339e9d  tests/test_omo_backend.rs
```

The durability build manifests instead contain backend `6a34652776db607df263fb1cc9ff2c477efd7c406362d404742dbe34ae508aee` and tests `9c9fe74dfa23926e607688b4a554880957dd7f58fcf6feeb95f69d6bf36ca8ce`. Do not conflate those source sets. Task 3's `task-3.log:81-96` independently records both focused virtual-clock tests passing.

## Commit-free source comparison

No git command was used. Full file bytes were read and SHA-256 computed in memory. The current pre-fix hashes exactly match `wave4b-source-post-2.sha256`:

```text
adc2c2f5bd5649baaf423a8a12ea7c355da1d58eeb8becffb87d178371c92c0c  src/agent/omo_backend.rs
d5f33aa4b2c0c0558ae51d96290a62a681e4e57fb98b55958d4e37e618339e9d  tests/test_omo_backend.rs
```

Removing ONLY the `"thread/tokenUsage/updated"` match arm from current backend bytes, in memory, produces exactly `04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2`. This reconstructs the duplicate-final backend identity without a historical checkout: the usage arm is the entire backend delta. All other `src/agent/` hashes in the two manifests match, including `omo_config.rs`, `omo_protocol.rs`, `omo_daemon.rs`, `backend.rs` and workspace handling. `Cargo.lock` also matches. The test file was byte-identical to duplicate-final before this repair.

Task 35 adds usage recording only for a `thread/tokenUsage/updated` notification, after an accepted start ACK and only with a pool. These tests supply neither usage notifications nor a pool. The clippy tuple repair in `wave4b-post-focused-clippy-fix.patch` only groups `(thread_id, turn_id)` in that arm. Neither change runs on the failing paths.

Task 15's source ledger owns lane runner/CLI/storage, adapter interception, lifecycle migration and lane tests, not the backend. These tests call `OmoBackend::run` directly without the lane runner or cancellation lifecycle. Task 3's lane budget and the 45-second exclusion are present and unchanged in the hash-reconstructed backend.

The 30-second start ACK timeout (`src/agent/omo_backend.rs:862-881`), chat-only initial 45-second check (`1043-1047`) and lane-vs-chat total budget (`1624-1634`) did not change. Lane sessions are explicitly excluded from the initial-response and interactive inactivity rules; their default total budget is 3600 seconds. The recalled historical note about watchdog changes is not evidence of a new Wave 4b delta: the exact released-source hash comparison is authoritative for this request.

## Root cause

Both tests used a peer-side oneshot immediately after `ws.send(start_ack).await`. That event proves the ACK was written by the peer, not consumed by the backend. The test then paused Tokio time and advanced it by 50 seconds while the backend future could still have `turn_started_ack == false`. Its 30-second ACK deadline then fired before it could consume the socket frame. This explains Discord's exact ACK-timeout error and the lane test's unexpected ready error at the pending assertion.

They do not sleep for 45-50 real seconds. They use `tokio::time::pause` and `advance(50s)` over real loopback WebSockets. The nondeterminism is socket readiness/task scheduling at the moment the clock advances. Discord also used one poll plus an optional 100-millisecond virtual advance to guess when its cleanup sleep had completed; that was another scheduling-dependent completion assumption.

The requested pre-fix runs, once each, both passed in isolation (Discord 0.25s, lane 0.07s). Therefore the failures were not deterministic product regressions. The preserved Wave 4b full log records them failing under the integrated batch; no repeated pre-fix run was used to fish for a failure.

## Minimal test diff

Only `tests/test_omo_backend.rs` changed. In each of the two tests:

1. Set the session's enabled toolsets to an empty vector, so the barrier request deterministically follows the existing denial-response path, independently of approval environment policy.
2. After writing the start ACK, the peer writes one `execCommandApproval` RPC with ID 777 and command `echo barrier`, then receives and asserts its denial response before signaling the existing oneshot.
3. Await that event while polling the backend with a five-second real-time bound, then pause/advance virtual time exactly as before. Ordered WebSocket frames plus the RPC response prove ACK consumption. The denial path does not deliver content or increment the approval-denial counter because it exits at the toolset restriction.
4. Discord awaits the backend result with a one-second virtual bound instead of guessing its readiness with a fixed poll/100ms advance, then bounds the peer interrupt-receipt event by five real seconds after resuming time.

Core diff excerpts:

```rust
session.state.enabled_toolsets = Some(Vec::new());

// Peer, after start ACK:
ws.send(Message::text(
    json!({"jsonrpc": "2.0", "id": 777, "method": "execCommandApproval",
        "params": {"command": "echo barrier"}})
    .to_string(),
))
.await
.unwrap();
let response = ws.next().await.unwrap().unwrap();
let response: Value = serde_json::from_str(response.to_text().unwrap()).unwrap();
assert_eq!(response["id"], 777);
assert_eq!(response["result"]["allow"], false);
turn_started_tx.send(()).unwrap();

// Discord, after advance(50s):
let result = tokio::time::timeout(std::time::Duration::from_secs(1), &mut run_fut)
    .await
    .expect("discord turn must fail-fast after 45s silence");
tokio::time::resume();
```

Original assertions remain: Discord must return the initial-deadline interruption error, the peer must receive `turn/interrupt` for `t-silent`, no Discord message/content may be delivered; lane must stay pending after 50 virtual seconds and finish successfully when the peer signals completion. No assertion was weakened or skipped. The lane pending check now runs after proven ACK consumption.

Post-fix test SHA-256: `1f96e75ed3a2f0345d9394e845c7a55edb47dab67db9ee4e12e21e603a15e652`. Product backend remains `adc2c2f5bd5649baaf423a8a12ea7c355da1d58eeb8becffb87d178371c92c0c`; full-byte comparison confirms it is unchanged.

## Commands and results

All Cargo commands used the warm shared target, offline mode, four build jobs and an empty command-scoped wrapper:

```text
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_omo_backend <name> -- --exact --nocapture
```

Pre-fix monitor `mon_RM0XNF1PERBRZ4JZ` / `bash_1`: Discord exit 0, 1 passed; lane exit 0, 1 passed. Exactly one invocation per test.

Post-fix monitor `mon_D6R21QVMQRR8A6N6` / `bash_2`: Discord exit 0, 1 passed in 0.01s; lane exit 0, 1 passed in 0.01s. Exactly one invocation per test. The initial rustfmt check found only two JSON-expression line wraps; they were corrected with a patch. No semantic change followed the focused passes.

Full monitor `mon_FPMMS1EMXM7B58G0` / `bash_3`:

```text
rustfmt --edition 2021 --check tests/test_omo_backend.rs
FMT_FINAL_EXIT=0
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_omo_backend -- --nocapture
test discord_silent_turn_interrupted_after_45s ... ok
test lane_silent_turn_not_interrupted_after_50s ... ok
test result: FAILED. 45 passed; 3 failed; 1 ignored; 0 measured; 0 filtered out; finished in 31.02s
BACKEND_FULL_EXIT=101
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo clippy -j4 --offline --test test_omo_backend -- -D warnings
CLIPPY_EXIT=0
```

The compound monitor exit is 1 solely because the full test target returned 101. Compilation succeeded. Final LSP diagnostics for the edited test file: no diagnostics, including severity `all`.

| Remaining failure | Canonical baseline | Observed error |
| --- | --- | --- |
| test_omo_backend_e2e_thread_lifecycle_and_streaming | Yes | approval allow `Some(true)` vs expected `Some(false)` |
| test_omo_backend_aborts_turn_on_repeated_approval_denials | Yes | timeout during turn streaming: no activity from server |
| test_omo_backend_deadline_interrupts_looping_turn | Yes | deadline must fire promptly, took 9.912977667s |

The other seven canonical backend failure names passed. Relative to the Wave 4b recheck (43 passed / 5 failed / 1 ignored), the two new names now pass and the same three pre-existing failures remain. New failure count: 0. No unfinished test remains. Those baseline failures were left untouched.

## Scope and limits

No services, live Discord, model calls, environment files or git commands were used. Only synthetic loopback tests ran; the existing live-daemon test remained ignored. No release, deployment, full repository test run or all-feature gate was performed. The requested full backend target ran once, with its pre-existing failures explicitly retained. Historical receipts and the earlier release-blocked JSON were not overwritten.

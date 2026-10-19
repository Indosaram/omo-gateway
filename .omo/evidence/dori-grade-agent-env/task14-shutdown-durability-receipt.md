# Task14 shutdown durability fix: source settled

Root: OmoBackend independently persisted the terminal assistant and delivered obligation, then actor shutdown unconditionally deleted every message after the inbound event through rollback_partial_history. Actor final flush inserts history rows with INSERT OR IGNORE; it is not the deletion source. Shutdown prioritizes cancellation even if backend output has already completed; generic run_cancelable can drop the run future during dispatch. Thus both accepted output before transcript persistence and delivered output before actor settlement were vulnerable. Existing Y5YPVQ JSON and corrected actual receipt are preserved unchanged.

Read-only in-memory SQLite reproduction of old rollback SQL yielded assistant_count0 with delivered_obligation_count1. This matches actual Y5YPVQ final state. No old-source Cargo rerun was performed; the focused regression was executed once against the fixed source, as requested.

Minimal fix in src/agent/omo_backend.rs: persist terminal assistant before external/local emission, with metadata_json.delivery_obligation_id linking the existing obligation. Cancellation during emission now has a durable assistant. src/multiplexer/actor.rs rollback excludes linked replies whose obligation is attempting or delivered; partial unlinked/failed history remains eligible for rollback. No new migration, shutdown delay, generic automatic resume promise, warning allowance or broad lifecycle redesign. Attempting is intentionally not falsely marked delivered when cancellation prevents dispatch acknowledgment; it retains durable content for the existing obligation semantics.

Regression in existing private headless_tests of src/main.rs: headless_early_shutdown_preserves_accepted_reply uses real OmoBackend, actual loopback initialize/thread/start/turn/start/delta/turn-completed protocol and migrated file SQLite. Two explicit oneshot barriers test cancellation inside accepted local final dispatch (obligation attempting) and after the real backend returns delivery (obligation delivered), while the wrapper still keeps the actor turn active. Shutdown is triggered at the barrier, not after a sleep/idle delay. Both assert exactly one durable assistant reply after joined shutdown. Existing backend/actor machinery remains integrated; barriers affect only the precise async timing.

Exact focused command executed once:

```sh
CARGO_TARGET_DIR="$PWD/target-w4" cargo test --offline -j2 --bin omo-gateway headless_early_shutdown_preserves_accepted_reply -- --nocapture
```

Monitor mon_A8VQBP4F5YCZBA0C, terminal bash_33: exit0, one test passed, zero failed, 60 filtered, 1.77 seconds test execution. Scoped rustfmt --edition2021 --check --config skip_children=true exited0. Final LSP main/actor/backend each returned no diagnostics after earlier timeouts. No live model retry, gateway binary build, global tests/clippy, daemon/model/env/git/userDB or Discord operation. Parent combined gate with task23/task13 remains pending and owns further verification.

Owned product paths and settled SHA256:

```text
c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a  src/main.rs
1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3  src/multiplexer/actor.rs
6a34652776db607df263fb1cc9ff2c477efd7c406362d404742dbe34ae508aee  src/agent/omo_backend.rs
```

The old884a binary remains the last actual-runtime artifact, not a claim that this fix is deployed. Successful headless happy QA still needs an actor completion/idle subscription distinct from admission/output; the adversarial regression intentionally does not wait for that completion.

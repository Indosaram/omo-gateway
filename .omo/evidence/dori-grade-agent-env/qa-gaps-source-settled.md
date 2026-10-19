# QA gaps source settlement

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Baseline QA binary: `71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86` (receipt identity, not rebuilt or executed here).

## Changes

- `src/main.rs`: headless startup now starts the same collector/triage runtime as Discord startup. Both call the same `OMON_INBOX_ENABLED` gate (`inbox::triage::enabled`), skip all collector/triage construction when disabled, and use the same watch shutdown and bounded join/abort handling. The Discord branch starts it once and attaches its optional sender only when enabled.
- `src/lanes/discord_threads.rs`: optional `OMON_DISCORD_API_BASE` parses a loopback HTTP(S) origin. IPv4 loopback, IPv6 `::1`, and `localhost` are accepted; non-loopback hosts, credentials, paths, query, fragments, malformed input, and unsupported schemes are rejected. `localhost` resolution is pinned to `127.0.0.1`. The QA client disables environment proxies and redirects, uses a fixed fixture-only token, and never constructs a Discord gateway client or websocket. Serenity's proxy origin receives the existing `/api/v10/...` REST routes. Unset configuration retains the original `first_bot_token` / `Http::new` path.
- `qa-loopback-sink-runner.patch`: unapplied apply_patch-format runner integration hunk. This selects the opted-in sink and lifecycle before the normal Discord-enabled branch, including when `OMON_DISCORD_ENABLED=false`. `src/lanes/runner.rs` was not edited. Integration owner must apply this hunk before released-binary headless post QA; no claim of integrated live transport is made.
- `src/main.rs` also wires todo 16: both branches initialize the configured Kibitzer before inbox or lane producers can start. The enabled worker receives one shared `Arc<Notify>` also passed through `LaneRunner::with_kibitzer_hint`; its serve task belongs to the existing lane JoinSet and cancellation/join shutdown path. Disabled startup constructs no worker, creates no schema/triggers, and spawns no task. No runner or kibitzer internals were edited.
- Six focused `qa_gaps` tests cover fake enabled inbox startup and shutdown/join, disabled inbox startup without constructing the fake, accepted loopback sink construction, rejection cases, and enabled/disabled Kibitzer startup. Kibitzer tests query actual in-memory SQLite schema and state rows, assert task ownership, and join after cancellation without producing model-triggering rows. The fake inbox startup uses a pre-registered oneshot signal, not sleeps or polling. Tests do not call sources, models, or a REST server.

The two public API doc comments document the loopback-only trust boundary and fixture credential isolation; these are necessary configuration contracts.

## Commands and results

1. `rustfmt --edition 2021 --config skip_children=true src/main.rs src/lanes/discord_threads.rs` completed successfully, scoped to owned files.
2. `rustfmt --edition 2021 --config skip_children=true --check src/main.rs src/lanes/discord_threads.rs` completed successfully after the final source change.
3. Initial LSP diagnostics and patch-time requests returned `LSP daemon request timed out after 30000ms: the daemon did not respond in time.` Final-scope diagnostics succeeded for both owned files with `No diagnostics found` (error severity).
4. Focused cargo invocation: `CARGO_TARGET_DIR=target-wqa cargo test --offline -j2 --lib --bin omo-gateway qa_gaps -- --nocapture`, deadline 1800000 ms. Original-scope monitor `mon_T4H1QC8ZXQ3N0SHF` (`bash_4`) was cancelled when the user added todo 16 mid-run; it produced no test results and exited 1 on cancellation. One final-scope run includes all six tests via monitor `mon_6EK1SR81PRTAN5JR` (`bash_6`). Final result: **TIMED OUT**, exit code **1**, at the requested 1800-second deadline. Output first reported `Blocking waiting for file lock on package cache`, then dependency compilation reached `37/477: lock_api, bytes`. No test executed or passing test summary was produced; compilation and all six test outcomes remain unverified. No duplicate test run was launched after the deadline.
5. `shasum -a 256 src/main.rs src/lanes/discord_threads.rs .omo/evidence/dori-grade-agent-env/qa-loopback-sink-runner.patch` completed successfully.
6. The same SHA-256 command was run after the timeout; all three source/artifact identities below were unchanged.

## Source SHA-256

| Artifact | SHA-256 |
| --- | --- |
| `src/main.rs` | `ab0c3bb50b6f2970622acfcbb66851e61e68de421e4223754551c6ed2f5dbe32` |
| `src/lanes/discord_threads.rs` | `1a6ea344b09af40cb5eed08ce2ffe5c778a77d9585b643ea55d557340f150c47` |
| `qa-loopback-sink-runner.patch` | `a3439929a44ffe80c1f54f06f01c6e3e3bfc1fe5062d38b0451da358b35b4615` |

No services, live Discord, model calls, env-file edits, or git commands were used. No live QA binary was rebuilt or replaced. Runner integration and later released-binary QA remain the integration owner's work as requested.

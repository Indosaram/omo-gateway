# Task 29: gateway direct session-status reply

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Changes

- `src/sessions/status_reply.rs`: strict short whole-message Korean/English intent matching, owner checks, machine-grouped SessionList formatter, home-relative folders, relative activity with KST label, idle/status counts, unreachable-host errors, stale age, and a 15-line cap. Registry construction errors and the hard 25-second listing timeout produce a one-line reply.
- `src/sessions/mod.rs`: exports the shared module.
- `src/discord/adapter.rs`: intercepts authorized owner messages before context hydration, lane replies, debounce, or steering. Owner DM is accepted; guild threads require the existing configured owner lead-thread scope, PrivateThread type, and an actual member list containing only the owner and receiving bot. Privacy lookup is bounded to 2 seconds and fails closed. Direct egress uses the session's bot client and `lane_allowed_mentions()`.
- `src/dashboard.rs`: both HTTP chat and WebSocket chat intercept before model routing. Local web/dashboard conversations use the local owner transport boundary; Discord session keys retain DM/verified-private-thread restrictions. Composite dispatch forwards direct replies to the dashboard and the appropriate external transport.
- `src/multiplexer/actor.rs`, `src/multiplexer/router.rs`: a direct-delivery seam saves the inbound user and assistant transcript entries, records a delivery obligation, dispatches without actor/backend execution, and marks the obligation delivered or failed.

The retained owner privacy API documentation explains why bot thread ownership alone is insufficient to release session metadata. The generic listing future is a narrow fake-registry seam, not a model mock.

## Verification

No live QA, Discord calls, services, environment-file edits, or git operations were used.

Touched-file `rustfmt --edition 2021 --config skip_children=true` completed with exit code 0.

Exactly one requested clippy invocation was issued:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo clippy --offline -j4 --all-targets --all-features -- -D warnings
```

It exited 101 with E0433 at `src/discord/adapter.rs:3131`: the new privacy timeout referenced unqualified `Duration`. This was corrected to `std::time::Duration` and the file formatted again. Clippy was not rerun, honoring the one-invocation constraint; a clean post-fix clippy result is therefore not claimed. LSP attempts did not provide fresh diagnostics because the LSP daemon timed out.

Focused tests completed: status-reply module 8 passed, registry module 10 passed, multiplexer modules 20 passed, Discord adapter module 42 passed, dashboard module 11 passed. Total: 91 passed, 0 failed. Both verification command chains exited 0. The new tests cover matching and nonmatching intent, formatter grouping/counts/stale/errors/size, the exact 25-second timeout with paused Tokio time, owner identity exclusion, owner DM yielding one assistant reply with zero backend turns and a delivered SQLite obligation, and shared/nonmatching requests falling through to normal backend handling without evaluating the registry future.

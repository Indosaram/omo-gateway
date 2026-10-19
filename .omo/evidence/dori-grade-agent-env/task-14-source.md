# Todo 14 source receipt

Source tree: `/Users/indo/code/project/omon-gateway-wt/dori-next`.
Status: source-settled; compilation, tests and live QA await the parent's batch gates.

## Changed files

- `src/lanes/dispatch.rs` (new): five-second local queue scan, injected tick timestamp, thirty-second coalescing and durable per-lead delivery slot, latest-body collapse, critical-event bypass, existing `Judge::noul(Purpose::EventWake)` integration, 0.5 threshold, digest-only receipts, route-before-delivery receipts and logged retries.
- `migrations/0037_lane_event_delivery.sql` (new): nullable `lane_events.delivery_reason`; required to persist `jev_skip` and distinguish digest-only receipts from turn deliveries after restart. No changes to the peer-owned `LaneEvent` row type or storage queries are required.
- `src/models/events.rs`: serde-defaulted `EventOrigin::{User,System}` and User constructor default; existing author/reference fields preserved.
- `src/models/mod.rs`: one additive `EventOrigin` re-export is necessary because events.rs is private and the public event field needs a publicly reachable type. This is the only additional file beyond the listed owned/shared paths.
- `src/multiplexer/actor.rs`: signal-driven idle probes and atomic idle-only admission, no system completion/failure reactions, system transcript marker in both persistence paths, no system-message indexing/inbox feedback.
- `src/multiplexer/router.rs`: System route waits for idle admission acknowledgement and rejects active-session admission; public idle probe and drain query.
- `src/main.rs`: dispatcher joins the existing lane task set and cancellation lifecycle; the actual multiplexer now receives the existing drain watch channel.
- `src/storage/db.rs`: User origin on recovery construction.
- `src/lanes/mod.rs`: exactly one additive module line.
- `Cargo.toml`: exactly one additive test registration block.
- `tests/test_lane_dispatch.rs` (new): eight acceptance tests, bounded pre-established channels and explicit timestamps; no sleeps or polling delays.

## InboundEvent construction scan

Scanned all `src/` and `tests/`, ignoring ignore rules, for `InboundEvent\s*\{`.
Updated all pre-existing literal construction sites:

- `src/main.rs`: `recover_resume_pending_sessions_with_auth` recovery event -> User.
- `src/multiplexer/actor.rs`: `SessionActor::recover_resume_pending_sessions` recovery event -> User.
- `src/storage/db.rs`: recovery event -> User.
- `src/models/events.rs`: `InboundEvent::message` uses `Self { ... }` -> User.

New literal: `src/lanes/dispatch.rs` -> System. No direct literals existed in tests or the Discord adapter; they use `InboundEvent::message` and inherit User.

## Test names

1. `three_events_within_ten_seconds_coalesce_into_one_turn`
2. `done_claimed_delivers_immediately_after_idle_delivery_slot`
3. `two_done_claims_five_seconds_apart_wait_thirty_seconds`
4. `event_during_in_flight_turn_waits_until_turn_ends`
5. `duplicate_lane_kind_pairs_collapse_to_latest_body`
6. `route_error_keeps_events_undelivered_for_retry`
7. `system_event_triggers_no_outbound_reaction`
8. `jev_below_threshold_marks_digest_only_without_turn`

The fake runner waits on a per-turn oneshot release. The fake outbound signals terminal typing release; the fixture then asks the actor for its idle state before a subsequent tick. This observes actual actor completion rather than assuming runner return equals actor completion. System tests also assert the transcript marker. The drain test checks route refusal directly and queued delivery after clearing drain.

## API and owner boundaries

The current actor contains no owner-authorization check or start-reaction dispatch: these occur upstream in the real Discord ingress adapter. System events enter directly through the multiplexer and bypass those ingress checks/reactions; actor completion and interruption reactions are explicitly disabled. No Discord adapter change is needed. Discord egress already ignores nonnumeric reply references, so `lane-event:<first>-<last>` is safe without modifying the peer-owned backend.

No cross-owner hooks are requested. Todos 13 and 18 files were not edited. The minimal public type re-export in models/mod.rs is listed above for parent review.

## Verification performed

Only standalone rustfmt ran; final command exited 0:

```text
rustfmt --edition 2021 --config skip_children=true src/lanes/dispatch.rs src/models/events.rs src/models/mod.rs src/multiplexer/actor.rs src/multiplexer/router.rs src/storage/db.rs src/main.rs tests/test_lane_dispatch.rs
```

`skip_children=true` prevents formatting traversal into peer-owned module files. Shared lanes/mod.rs received only its additive declaration and was not recursively formatted. Source inspection and the final literal/origin scan were completed. Tool-triggered LSP diagnostics returned server cancellation / freshness timeout, not usable compiler diagnostics. No Cargo command, Git command, deployment, live database write or Discord UI operation was run. No compilation, test pass or live-surface verification is claimed.

The parent must run its compile-only gate, hand back diagnostics, and then its four-gate batch validation and live QA. Source implementation has no intentionally deferred hook or placeholder.

source-settled

# fix-r4-wiring

Test: `tests/test_wiring_e2e.rs::test_e2e_actor_omo_backend_persists_assistant_message`

## Root cause (test fixture stale, not production)
`src/agent/omo_backend.rs:722-790` (`execute_turn`): whenever a model is resolved
(the test sets `with_default_model` and a profile `model` override), the backend sends
`thread/settings/update` (id 4) before `turn/start` and blocks in a loop (lines ~745-768)
until it gets an id-4 response, or the deadline hits. The in-file `FakeAppServer`
(`tests/test_wiring_e2e.rs`, method match) handled only `initialize`, `thread/start`,
`turn/start`; `thread/settings/update` fell into `_ => {}` and was never answered.
So the turn never reached `turn/start`, no delta/`turn/completed` frames were consumed,
and no `OutboundAction::Stream` chunk was dispatched.

Other hypotheses ruled out: one dispatcher is shared by backend and multiplexer
(`dispatcher.clone()`); delta / `turn/completed` frame shapes match what the backend
consumes (`item/agentMessage/delta` at `/params/delta`, `turn/completed` at
`/params/turn/status`, `threadId`/`turnId` present, `turn/start` ack at `/result/turn/id`);
chunks do go through `OutboundAction::Stream` (`emit_chunk`, omo_backend.rs:~552).

## Fix (test code only)
1. `FakeAppServer`: added a `"thread/settings/update"` arm replying `{"jsonrpc":"2.0","id":<id>,"result":{}}`
   (same shape as the fake in `tests/test_omo_backend.rs:164`).
2. Removed a latent race: the existing bounded wait only observes `thread/start`
   (developer instructions), which precedes the turn. Added a deterministic event-based wait:
   `CapturingDispatcher` now has a `tokio::sync::Notify` signalled on every `dispatch`, and
   `wait_for_final_chunk(timeout)` arms the notification before each state check
   (no sleeps/polling), 10s bounded timeout. The test awaits it before reading
   `stream_chunks()`. No assertions were weakened.

## Result
```
running 1 test
test test_e2e_actor_omo_backend_persists_assistant_message ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.04s
```
Command: `cargo test --all-features --test test_wiring_e2e test_e2e_actor_omo_backend_persists_assistant_message`

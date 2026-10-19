# fix-omo-backend report

Write scope used: `src/agent/omo_backend.rs` (fixes) and `tests/test_fix_fix_omo_backend.rs` (new, 5 tests).
No files outside scope edited. No commit, no fmt/clippy run.

Tests are deterministic. They use a local in-process WebSocket peer that replies to the actual JSON-RPC methods. There are no sleeps. Each await has a bounded `timeout`.

RED command (unfixed code, all 5 failing): `cargo test --test test_fix_fix_omo_backend`
Log: `fix-omo-backend-red.log`. Defect 4 was re-captured alone in `fix-omo-backend-red-d4.log` (the first RED hit the unrelated 1s deadline error, so the test was adjusted to drain via `turn/interrupt`).
GREEN command: `cargo test --test test_fix_fix_omo_backend`
Log: `fix-omo-backend-green-final.log`, which is the foreground re-run in this session:

```
running 5 tests
test turn_error_and_invalid_frame_release_active_turn ... ok
test reset_thread_binding_starts_fresh_thread_instead_of_cached_one ... ok
test rejected_model_settings_retry_with_fallback_model ... ok
test mid_turn_status_before_tool_is_delivered_as_final ... ok
test deadline_drain_strips_intent_gate_for_cron ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Also ran `cargo test --lib agent::omo_backend`: 3 existing tests pass (`fix-omo-backend-lib-green.log`).

## 1. [P1] mid-turn status never shown - FIXED
- Function: the `item/started` tool-boundary flush in `OmoBackend::run_turn` (~1452) and the `item/completed` agentMessage path (~1515).
- Change: mid-turn text now goes through `deliver_rendered_result` (`is_final=true`) instead of `emit_chunk(.., false)`. `messages_delivered` increments only on `Ok`. On the agentMessage path `full_content` is cleared only on a successful delivery. Delivery failures are logged with `tracing::warn!` and the turn continues.
- Test: `mid_turn_status_before_tool_is_delivered_as_final`. It covers both the delta path (`STATUS_DELTA`) and the completed-item path (`STATUS_ITEM`).
- RED excerpt:
  ```
  assertion `left == right` failed
    left: []
   right: ["STATUS_DELTA"]
  ```
- GREEN: `ok` (see above).

## 2. [P2] active_turns entry leaked on early error returns - FIXED
- Function: `run_turn`.
- Change: `self.active_turns.lock().remove(&session.key.storage_key())` added before the error returns for:
  - streaming inactivity timeout (~1123)
  - invalid JSON frame (`map_err`)
  - `turn/start` ack missing turn id
  - approval-denial limit abort (~1343)
  - `turn/error` (~1685)
  - `lane_capture_error`
  - `capture_lane_item` failure
  - `deliver_rendered_result` failure on the completed-agentMessage path
- Test: `turn_error_and_invalid_frame_release_active_turn`. It covers the `turn/error` and invalid-JSON paths.
- RED excerpt:
  ```
  errored turn left active_turns entry: Err(Llm("omo app-server turn error: boom"))
  ```
- GREEN: `ok`.
- The other paths (inactivity timeout, approval-denial limit, missing turn id, lane capture) use the same one-line removal but have no dedicated tests. They need long timeouts or tool-approval frames, and I did not build that setup.

## 3. [P2] `model update rejected` did not set fallback_eligible - FIXED
- Function: `run_turn`, the `thread/settings/update` response (id 4) handling (~758).
- Change: `*fallback_eligible = is_model_rejected_error(&error.to_string());` before returning the error.
- Test: `rejected_model_settings_retry_with_fallback_model`. The peer rejects `custom/rejected` and the backend must retry with `custom/fallback`.
- RED excerpt:
  ```
  Err(Llm("model update rejected: {\"code\":-32603,\"message\":\"model not found: custom/rejected\"}"))
  ```
- GREEN: `ok`. The peer observed models `["custom/rejected", "custom/fallback"]` and the final text was `FALLBACK_OK`.

## 4. [P2] deadline-drain shadowed the intent-gate-filtered content - FIXED
- Function: `run_turn`, the deadline-drain completion path (~1033).
- Change: removed `let scrubbed_content = filter_reasoning(&full_content);`, so the earlier `filter_intent_gate`-filtered `scrubbed_content` is used.
- Test: `deadline_drain_strips_intent_gate_for_cron`. It uses a cron session with a 1s total timeout; the peer sends the message and `turn/completed` only after `turn/interrupt`.
- RED excerpt (`fix-omo-backend-red-d4.log`):
  ```
  left: ["> I read this as a digest. I'll stop when done.\n\nDIGEST_BODY"]
  right: ["DIGEST_BODY"]
  ```
- GREEN: `ok`.

## 5. [P2] resolve_thread_id resumed the pre-reset thread from cache - FIXED
- Function: `resolve_thread_id` (~277), plus a doc comment tweak.
- Change: removed the `.or_else(|| self.thread_ids.lock().get(&storage_key).cloned())` fallback. Metadata is now the sole source of truth, so a reset (which removes `omo_thread_id`) starts a fresh thread.
- Test: `reset_thread_binding_starts_fresh_thread_instead_of_cached_one`. It runs two turns and removes the metadata between them.
- RED excerpt:
  ```
  left: ["thread/start", "thread/resume"]
   right: ["thread/start", "thread/start"]
  ```
- GREEN: `ok`.
- The `thread_ids` cache is still written elsewhere. I did not touch that, to keep the change minimal. It is now only a write-side cache here.

## BLOCKED-SCOPE
None.

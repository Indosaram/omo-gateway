# agent review

### [P2] `clear_stuck_thread` does not clear the thread id held by the session actor
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:764 (called after a deadline or continuity error); /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:578-585 (adopts `omo_thread_id` from the turn context only when it is present, and not at all on `Err`).
- Trigger: an interactive turn exceeds its deadline or `thread/resume` fails with a continuity error. The backend removes `omo_thread_id` from the turn session and clears the DB binding. The actor's `self.context` still holds the old id, so the next turn resumes the same thread.
- Impact: the "release the stuck thread" intent is only half effective for actor-driven sessions. Each later turn repeats the same failure until `/reset` or an actor restart. This may be the intended stickiness, since the tests say "not silently start a fresh replacement thread", but the code and the helper name suggest otherwise.
- Fix: if clearing is intended, have the actor drop `omo_thread_id` from `self.context` when the turn context lacks it after an error. If stickiness is intended, rename or comment `clear_stuck_thread` so it does not claim to unbind.

### [P2] A pre-ack relayed message and the final reply share one `stream_id` and one obligation id
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:727 (`obl:turn:{stream_id}`), pre-ack relay at about :1532, final delivery at :1143 and :1768. The upsert is in /Users/indo/code/project/omon-gateway/src/ledger/service.rs:391-395, which overwrites `content` but does not reset `state`.
- Trigger: a non-cron, non-lane turn emits a completed agent message before the `turn/start` ack, then finishes normally.
- Impact: two assistant message rows point at one `delivery_obligations` row, and the second `record_obligation` overwrites the first message's text. A crash after the second `record_obligation` and before `mark_obligation_attempting` leaves the unsent final text recorded as `delivered`, because the state was not reset. `rollback_partial_history` in /Users/indo/code/project/omon-gateway/src/multiplexer/actor.rs:1030-1036 also treats both rows as one obligation. The window is narrow and the ack race is rare.
- Fix: give pre-ack relays their own id, for example `obl:turn:{stream_id}:{sequence}`. Alternatively, reset `state = 'pending'` in the `ON CONFLICT` branch of `record_obligation`.

### [P2] A rejected `thread/settings/update` still never reaches the model fallback
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:853 (returns `Err` directly). The classifier is at :40.
- Trigger: `active_model` names a model the daemon rejects at `thread/settings/update`, as in `rejected_model_settings_return_typed_error_without_starting_turn` in /Users/indo/code/project/omon-gateway/tests/test_omo_backend.rs:238.
- Impact: `fallback_eligible` stays false, so the primary-to-fallback model switch is skipped. Every turn fails until `/model` is changed. The test pins the typed error but not the fallback, so it can pass while this stays broken. This was skipped earlier; it is flagged again because the classifier already lists these patterns.
- Fix: run the model-rejection classifier on that error and set `fallback_eligible` before returning, as the `turn/start` path does.

### [P2] `is_local_url` accepts a URL with userinfo
- Location: /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs (the `is_local_url` helper that decides autospawn eligibility).
- Trigger: the configured URL is `ws://localhost:pw@evil.example`. The host parse can yield `localhost`, so the URL counts as local and eligible for autospawn, while the client connects to `evil.example`.
- Impact: the URL comes from operator config, so this is low risk. It does mean the "local only" guard can be bypassed by a misconfiguration.
- Fix: reject URLs where `url.username()` or `url.password()` is non-empty before the local-host check.

Known caveats (design or earlier decisions, not findings):
- The zero-config daemon runs with `--ws-auth off`.
- The default auto-approve ignores `/yolo off`.
- `steer()` returns `Ok(false)` on timeout or an ambiguous ack, so a steer that was actually delivered may be re-run as a new turn.
- `omo_activity.rs` is dead code under `#![allow(dead_code)]`.
- Because the interactive deadline now renews on every progress frame, an agent that loops while emitting progress frames never reaches the 30-minute cap. Only cron turns keep an absolute deadline. This contradicts the "hard ceiling" wording in `omo_config.rs`, but it is the intended trade-off for the watchdog fix.
- `llm.rs` now reports a stream with no terminator as truncated, which affects an OpenAI-compatible provider that sends neither `[DONE]` nor `finish_reason`.

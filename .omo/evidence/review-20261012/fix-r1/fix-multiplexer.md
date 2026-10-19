# fix-multiplexer (fix-r1)

The only file edited is `src/multiplexer/actor.rs`: production code in `SessionActor::run_with_shutdown`, plus new tests in its existing `#[cfg(test)] mod tests`.
Full logs are in `fix-multiplexer-red.log` and `fix-multiplexer-green.log`, both in this directory.

## GREEN build caveat (read first)
GREEN could not compile in the repo checkout. Another agent's uncommitted test in `src/agent/omo_daemon.rs` (around line 847, `fixr1_daemon_command_uses_token_file_when_token_configured`) calls a missing `write_token_file` and uses a 3-arg `daemon_command`. That file is out of my scope, so I did not touch it. It was still broken after about 20 minutes of retries.
To get a GREEN run, I made a scratch copy of the working tree at `/tmp/omon-fix-multiplexer-green` with these properties:
- It includes all current uncommitted changes. `src/multiplexer/actor.rs` was verified byte-identical with `cmp`.
- `src/agent/omo_daemon.rs` was restored to `HEAD`.
- It ran against the repo's default `target/` dir.

The lead should rerun the GREEN command in the repo once `omo_daemon.rs` compiles.

## Commands
- RED (repo, unfixed actor.rs): `cargo test --lib multiplexer::actor::tests::` -> exit 101, `6 passed; 3 failed`.
- GREEN (scratch copy above, fixed actor.rs): `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway/target cargo test --lib multiplexer::actor::tests::` -> exit 0, `test result: ok. 9 passed; 0 failed; ... 520 filtered out`.
- `rustfmt --edition 2021 --check src/multiplexer/actor.rs` -> exit 0.

New tests are deterministic. They use barriers, oneshot replies, and a queued `WaitIdle`, with no sleeps.

## Defect 1 [P1] Mid-turn SetModel reverted on turn completion - FIXED
- Change: `run_with_shutdown` records `model_at_turn_start` when it takes the `turn_context` clone. On the `Completed(Ok)` path, if `self.context.state.active_model` changed mid-turn, the new value is put back after `self.context = turn_context;`.
- Test: `multiplexer::actor::tests::mid_turn_set_model_survives_turn_completion`
- RED excerpt:
  ```
  test multiplexer::actor::tests::mid_turn_set_model_survives_turn_completion ... FAILED
  assertion `left == right` failed: a /model switch during a turn must not be reverted when the turn completes
    left: None
   right: Some("mid-turn-model")
  ```
- GREEN excerpt: `test multiplexer::actor::tests::mid_turn_set_model_survives_turn_completion ... ok`

## Defect 2 [P1] Mid-turn Reset resurrects omo_thread_id (and marks resume_pending) - FIXED
- Change 1: the post-turn `omo_thread_id` carry-over from `turn_context` is now guarded by `if !self.binding_cleared`.
- Change 2: in the `TurnOutcome::Shutdown` arm, `mark_session_resume_pending` is skipped when `self.binding_cleared` is set. A mid-turn reset breaks with `Shutdown`, and the turn it abandoned must not be resumed. Real shutdowns never set `binding_cleared`, so their behaviour is unchanged.
- Test: `multiplexer::actor::tests::mid_turn_reset_does_not_restore_thread_binding_or_mark_resume_pending`. It seeds `omo_thread_id = "old-thread"`, resets mid-turn, then asserts the DB `state_json` binding is NULL and `resume_pending = 0`.
- RED excerpt (the first assertion fails, so the resume_pending assertion was not reached in RED):
  ```
  test multiplexer::actor::tests::mid_turn_reset_does_not_restore_thread_binding_or_mark_resume_pending ... FAILED
  assertion `left == right` failed: reset must not resurrect the old binding
    left: Some("old-thread")
   right: None
  ```
- GREEN excerpt: `test multiplexer::actor::tests::mid_turn_reset_does_not_restore_thread_binding_or_mark_resume_pending ... ok` (both assertions pass).

## Defect 3 [P1] Final pending_events drain drops plain Event with ledger stuck in_progress - FIXED
- Change: the final drain loop after the main loop now has an `ActorCommand::Event(event)` arm. It calls the existing `self.complete_delivery(event.delivery_id.as_deref(), &Err(OmonError::Multiplexer("session actor shutting down".into())))`, which uses the same helper and message as the in-turn shutdown drain. The ledger row becomes `failed`, and `record_incoming_as` can then reclaim it.
- Test: `multiplexer::actor::tests::shutdown_drain_fails_queued_plain_event_delivery`. A blocking turn runs, then a plain `Event` with a ledger row is queued. A mid-turn `Reset` then ends the actor (`break TurnOutcome::Shutdown` -> loop break) with that event still in `pending_events`, and the test asserts the ledger status is `failed`.
- RED excerpt:
  ```
  test multiplexer::actor::tests::shutdown_drain_fails_queued_plain_event_delivery ... FAILED
  assertion `left == right` failed: dropped queued event must be released as failed, not left in_progress
    left: "in_progress"
   right: "failed"
  ```
- GREEN excerpt: `test multiplexer::actor::tests::shutdown_drain_fails_queued_plain_event_delivery ... ok`

## Notes
- No BLOCKED-SCOPE fixes. All three fixes fit inside `actor.rs`.
- Follow-up, not changed: a queued plain `Event` that a mid-turn Reset drops is failed in the ledger, but it is not re-dispatched automatically.

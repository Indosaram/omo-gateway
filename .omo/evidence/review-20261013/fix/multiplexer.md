# multiplexer fix report

Write scope: `src/multiplexer/*.rs` only. No cargo/bun/compile command was run (per instruction); the lead's integration gate is the first compile. Tests were added inside existing `#[cfg(test)]` modules (`actor.rs`, `profile_routing.rs`) and one new `#[cfg(test)] mod tests` at the end of `router.rs`.

## P1

### [P1] Retried or previously failed inbound messages are skipped as "already replayed"
FIXED - `actor.rs`: `SessionActor::inbound_turn_completed` (new), `discard_unfinished_inbound` (new), `remember_completed_inbound` (new), `ActorCommand::Event` dedup branch.
- A transcript row alone no longer counts as processed. The replay skip now needs completion evidence, checked in order: (1) the id was completed successfully by this actor (bounded in-memory set, 256 ids); (2) the delivery ledger row for that platform message, if any, is authoritative (`delivered` => skip; `failed`/`in_progress` => run); (3) otherwise an assistant row follows the user row before the next user row.
- When the message is not complete, the stale user row of the failed attempt is deleted before `persist_inbound` so the redelivered attempt does not leave a duplicate row.
- The existing integration test `transcript_level_inbound_dedup_skips_duplicate_platform_message_id` (no ledger row, runner writes no assistant row) stays green via the in-memory completed set.
- Tests: `actor::tests::redelivery_of_a_failed_turn_runs_again_instead_of_being_skipped`, `actor::tests::redelivery_of_a_completed_turn_is_still_skipped`.
- Not done (BLOCKED-SCOPE, optional hardening): the review also suggested running `persist_inbound` + `index_inbound` in one transaction; `index_inbound` lives in `src/storage/message_search.rs` (outside scope). The stale-row discard above covers the index-failure case because the next attempt re-inserts and re-indexes.

### [P1] Live actor silently drops `/reset`, `/yolo`, `/model` during the GC retire window
FIXED - `router.rs`: `SessionHandle::{set_model,set_yolo,reset}` now return `Result<SendOutcome>` (no more `Ok(())` on `Retiring`/`Closed`); new `SessionMultiplexer::apply_to_live_actor` loops like `stop()`: `Retiring` => `wait_until_reusable()` and retry, `Closed` => `remove_handle` + `mark_finished` and re-resolve (no live actor left => no-op). `SessionMultiplexer::{set_model,set_yolo,reset}` use it; `reset` no longer swallows errors (`let _ =` removed) and no DashMap guard is held across an await (handle is cloned first). Public signatures unchanged.
- Test: `router::tests::set_yolo_waits_out_a_retiring_actor_instead_of_dropping_the_change`.

### [P1] Graceful shutdown marks every resident session `resume_pending`
FIXED - `router.rs`: `SessionMultiplexer::mark_in_flight_resume_pending` now probes each actor with `IsIdle` (`has_unfinished_work`) and only marks sessions that report a running turn or queued events. The probe uses `try_send` (does not touch the `in_flight` counter that `shutdown()` waits on), treats a full mailbox or a 2s probe timeout as "busy" (never lose in-flight work), and a closed actor as not busy. The actor's own shutdown path already marks the active turn / queued events.
- Test: `router::tests::mark_in_flight_resume_pending_skips_idle_sessions`.

## P2

### Mid-turn steered user messages are deleted by interrupt rollback
FIXED - `actor.rs`: `rollback_partial_history` now `DELETE ... WHERE role = 'assistant' AND ...` so steered user rows (written by `record_inbound_transcript`) survive `/stop`, `/reset` and shutdown rollback. Test: `actor::tests::interrupt_rollback_keeps_steered_user_rows`.

### A GC flush error aborts the whole GC pass
FIXED - `gc.rs`: `collect` logs the per-session error (`tracing::error!`), calls `handle.resume()` and continues instead of `return Err`. Test: `router::tests::gc_keeps_evicting_after_one_session_fails_to_flush` (forced flush failure via trigger on one session; the healthy idle session is still evicted).

### `is_user_allowed_for_bot_or_route` fails open for guild-scoped routes
FIXED (additive, signature kept) - `profile_routing.rs`: new `ProfileRouter::is_user_allowed_in_context(bot_id, guild_id, channel_id, thread_id, parent_chat_id, user_id, global)` resolving the route via `match_route_with_parent`; the old method delegates to it with no guild/thread context (behavior unchanged, so `tests/test_profile_routing.rs` keeps compiling). The function is still not called in production; whoever wires ingress authz must call the new one with the real guild/thread. Test: `profile_routing::tests::allowlist_of_guild_scoped_route_is_enforced_with_full_context`.

### Profile-route config parsing silently widens or drops restrictions
FIXED (library level) / BLOCKED-SCOPE (startup wiring) - `profile_routing.rs`: negative `guild`/`channel`/`thread` now fail deserialization instead of becoming `None` ("match any"); `parse_profile_routes` skips only the malformed entry (logged at error) instead of dropping every route; new `try_parse_profile_routes` (re-exported in `multiplexer/mod.rs`) is a strict variant returning `Err` on any bad entry. Tests: `negative_ids_are_rejected_instead_of_widening_the_route`, `one_bad_route_entry_does_not_drop_the_other_routes`.
- BLOCKED-SCOPE: making a bad `DISCORD_PROFILE_ROUTES` a hard startup error requires `src/main.rs:180` and `src/main.rs:733` to call `omon_gateway::try_parse_profile_routes(&optional_env("DISCORD_PROFILE_ROUTES").unwrap_or_default())?` (map the `serde_json::Error` to `OmonError::Config`) instead of `parse_profile_routes(...)`. `try_parse_profile_routes` is exported from `src/multiplexer/mod.rs`; `src/lib.rs` re-export list (`pub use multiplexer::{...}`, line 37) would also need `try_parse_profile_routes` added.

### `SessionActor::recover_resume_pending_sessions` loses recovery / wrong delivery id (dead in production)
PARTIALLY FIXED - `actor.rs`: on a route error the `resume_pending` marker is now restored (mirrors `main.rs`). Test: `actor::tests::failed_recovery_route_keeps_resume_pending_marker`. SKIPPED: deleting the function or delegating to the `main.rs` logic (the integration test `tests/test_multiplexer.rs:828` calls it and is outside scope); the `.unwrap_or(None)` ledger lookups and the "latest ledger row for the session" fallback are left as is since the function has no production caller.

### `set_model` can wipe a corrupt or unparsable stored state
FIXED - `router.rs`: `SessionMultiplexer::set_model` now runs a single atomic `UPDATE sessions SET state_json = json_set(state_json, '$.active_model', ?) ...`; malformed `state_json` makes SQLite error (surfaced) instead of being replaced by a default state, and other fields (`yolo`, `system_prompt`, `metadata.omo_thread_id`) are preserved. Test: `router::tests::set_model_patches_only_the_model_and_refuses_corrupt_state`.

### Queued events are dropped on shutdown (unverified)
SKIPPED - unverified in the review; fixing it means persisting inbound rows at enqueue time (a design change) or confirming backfill coverage, neither a minimal in-scope change. Existing behavior (queued deliveries marked failed, session marked `resume_pending`) is unchanged.

## Notes for the integration gate
- Nothing was compiled. Most likely compile-risk spots: closure/generic shape of `apply_to_live_actor` (router.rs), the `tests` module in router.rs (new), the new tests in `actor.rs` (use `super::*` + `SessionMultiplexer`, `record_inbound_transcript`).
- `rollback_partial_history` now removes only `assistant` rows; partial `tool`/`system` rows of an interrupted turn are no longer rolled back (the review asked for assistant-only restriction).

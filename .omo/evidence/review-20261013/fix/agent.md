# fix r5 - node `agent` (lanes: agent-backend, agent-runtime)

Write scope used: `src/agent/*.rs` plus the new `tests/test_fix_r5_agent.rs`. No existing test file was edited. Nothing was compiled or run (per directive); the lead's integration gate is the first compile.

## agent-backend

- **[P1] Steer acks and unrelated frames renew the watchdogs** -> FIXED.
  - `omo_backend.rs`: new `frame_is_turn_progress` and the `Message::Text` branch of `execute_turn`. Only the turn/start ack (id 3), `turn/*` frames, agent-progress `item/*` frames and approval requests for this thread and turn renew `last_activity_at` and the interactive deadline.
  - Responses to gateway requests (ids >= 9000, no `method`), `thread/status/changed`, `userMessage` echoes, and frames for other threads or turns do not renew them. A successor `turn/started` counts only while `awaiting_successor_turn`.
  - `first_progress_at` is now set to `Instant::now()` (it copied `last_activity_at`).
  - Tests: `omo_backend.rs::fixr5_agent_tests::steer_acks_and_unrelated_frames_do_not_count_as_turn_progress`. Existing `test_omo_backend_deadline_fires_on_ping_only_stream` still holds, because a non-Text frame never renewed anything.

- **[P1] Mid-turn status invisible in Discord and counted as delivered** -> FIXED, per decision (2).
  - Only the tool-start branch relays mid-turn text. It now goes through the new `OmoBackend::relay_status`, which dispatches `OutboundAction::SendMessage`. This is a visible normal message, not a `StreamChunk` and not persisted as the reply.
  - `messages_delivered` increments and `full_content` is cleared only when dispatch succeeded. A failed relay keeps the text and leaves the steer guard armed.
  - Removed the "first intermediate agentMessage" relay (it would double-send the reply whenever that message turned out to be the final one). To keep the old steer-suppression, added `agent_message_seen`: no more progress steers once the agent has produced a message.
  - Tests: `tests/test_fix_r5_agent.rs::mid_turn_status_is_a_visible_message_not_a_stream_chunk_or_the_reply`. Checks that exactly one `SendMessage("Checking now.")` was dispatched, no stream chunk carries it, the only final chunk is `All done`, and `cron_agent_output` is `All done`.
  - **Existing tests**: no edits needed in `tests/test_omo_backend.rs` or `tests/test_wiring_e2e.rs`. Every `is_final` assertion there still holds. The only `actions.len()` asserts are in `src/main.rs`, outside the agent path. I traced these tests (`TASK14`, `U12`, `completed_item_finalizes_once_at_turn_terminal`) by hand but did not compile them.
  - Other dispatcher implementations that will now see mid-turn `SendMessage` (the kibitzer, triage and judge `Capture` sinks) overwrite their buffer on every `SendMessage`, so a status sent before the final stream chunk is overwritten by the final text. The `Capture` in `src/lanes/kibitzer.rs` handles only completed stream snapshots. Lane sessions never relay (`!is_lane`), so lanes are unaffected. The lead's gate should confirm.

- **[P2] Cron briefings consumed before turn/start** -> FIXED, per decision (3).
  - `run()` now calls `mirror::peek_pending_cron_briefings` once and passes `Option<&CronBriefings>` to both `execute_turn` attempts.
  - `mark_cron_briefings_injected` is called only after the turn/start ack.
  - Test: `tests/test_fix_r5_agent.rs::cron_briefings_survive_a_model_fallback_and_are_consumed_after_the_ack`. Both attempts' prompts contain the briefing; afterwards `peek_pending_cron_briefings` returns `None`.

- **[P2] Deadline drain stops on any non-Text frame** -> FIXED.
  - `omo_backend.rs`: the drain loop now `continue`s on Ping/Pong/Binary and breaks only on `None` or `Err`.
  - Covered by inspection only; no new test (no deterministic hook without a live ws).

- **[P2] Cron `settings.json` rewritten on parse error** -> FIXED.
  - New `ensure_cron_ttsr_disabled`: an unreadable or non-JSON file is left untouched, an already-correct file is not rewritten, and a rewrite goes through temp file + rename.
  - Test: `omo_backend.rs::fixr5_agent_tests::cron_settings_json_is_merged_atomically_and_never_clobbered`.

- **[P2] Settings-update rejection never reaches the model fallback** -> SKIPPED.
  - `tests/test_omo_backend.rs::rejected_model_settings_return_typed_error_without_starting_turn` pins the current behavior: the actual rejection error is returned with no fallback and no output.
  - Changing it means editing that test, which I was told to do only for action-counting breaks.
  - If the lead wants the fallback: set `*fallback_eligible = true` when `is_model_rejected_error(&error.to_string())` at the `model update rejected` return in `execute_turn`, and update that test.

- **[P2] Default approvals ignore `/yolo off`; toolset gate only covers commands** -> SKIPPED. This is a product and policy decision (single-user default auto-approve), not a local defect fix. One related robustness fix landed: a toolset-blocked request now counts toward the denial-loop guard instead of looping silently.

- **[P2] `steer()` reports "not delivered" on an ambiguous ack** -> SKIPPED. Returning `Err` also makes the Discord adapter re-enqueue the message (adapter.rs comment: a steer error is processed twice), so no change inside this scope removes the duplicate. Proper fix: a tri-state result in `AgentBackend::steer` (`src/agent/backend.rs` plus its callers in `multiplexer/router.rs`, `adapter.rs`, `lanes/runner.rs`). That is outside the agent write scope.

- **[P2] Dead `omo_activity` module and stale-turn helper** -> SKIPPED (dead-code cleanup only; no behavior change).

- **Extra hygiene fix, same file**: the `active_turns` entry is now removed on a few error returns that previously leaked it (lane capture error, invalid JSON, missing turn id in the ack, turn error frame, inactivity timeout, approval-denial abort, pre-ack relay failure). This follows the guard in `execute_turn` that already cleared it on other exits.

## agent-runtime

- **[P1] Daemon supervisor gives up permanently** -> FIXED, per decision (4).
  - `omo_daemon.rs`: removed the 3-restarts circuit and the "unready replacement sets shutdown" path.
  - The watcher now retries forever with capped exponential backoff (`restart_backoff`, 2s doubling to a 60s cap). The failure counter resets after a daemon stayed healthy for >= 60s.
  - A slow or unready replacement is killed and retried. A spawn error (ENOENT) is logged and retried.
  - `is_available()` is now false only after `kill()`.
  - `spawn_watcher_with_backoff` takes the timings so tests run fast.
  - Tests: `daemon_exiting_child_is_restarted_past_the_old_budget` (5 crashes, still available), `daemon_unready_replacement_is_killed_and_retried`, `restart_backoff_grows_exponentially_and_is_capped`.
  - Removed the two old tests that asserted the terminal circuit (`daemon_exiting_child_budget_local_surface`, `daemon_restart_budget_stops_spawn_failures`); the isolated-HOME match list was updated to the new names.

- **[P2] `OmoBackendConfig` auth_token in derived Debug/Serialize** -> FIXED.
  - `omo_config.rs`: manual redacting `Debug`; `#[serde(default, skip_serializing)]` on `auth_token`.
  - Test: `fixr5_backend_config_debug_and_serialize_hide_auth_token`.

- **[P2] Lane/kibitzer/triage/judge backends get no workspace root** -> FIXED.
  - `OmoBackendConfig::from_env()` now defaults to `$HOME/.omon/workspace` when `OMON_WORKSPACE_ROOT` is unset or empty (helper `default_workspace_root`).
  - Tests: `fixr5_default_workspace_root_precedence_and_empty_rejection`; the existing `test_per_agent_workspace_and_workspace_root_env_parsing` assertion was updated (workspace_root was `None` when unset).

- **[P2] `wipe_omo_thread_bindings` malformed `state_json` aborts the UPDATE** -> FIXED (the `json_valid` guard). Test extended in `workspace_migration.rs`. The "wire it once behind a ledger row" part -> BLOCKED-SCOPE. It needs a migration-ledger row or a new migration (migrations node) and a call site in `src/main.rs` before actors load sessions. Without a run-once ledger, calling it at boot would wipe every binding on every restart.

- **[P2] Daemon log/token dir permissions and growth** -> FIXED.
  - The log is opened 0600 (existing file tightened), `~/.omon` is created 0700, and the log rotates once to `.log.1` above 8 MiB.
  - Test: `fixr5_daemon_log_is_owner_only_and_rotated`.

- **[P2] Zero-config daemon runs with `--ws-auth off`** -> SKIPPED. Generating and persisting a token changes how the backend authenticates and how the shared external daemon is started. That is a deployment decision and interacts with the externally managed daemon at `ws://127.0.0.1:19742`.

- **[P2] `LlmClient` credentials** -> FIXED. `llm.rs`: both credential header values are `set_sensitive(true)`; `redirect(Policy::none())` so `x-api-key` cannot follow a redirect. Test: `fixr5_credentials_are_marked_sensitive`.

- **[P2] Streamed OpenAI tool-call name appended per delta; Ollama calls overwritten** -> FIXED (name is assigned, not appended; Ollama calls append by position). Tests: `fixr5_openai_repeated_tool_name_is_not_concatenated`, `fixr5_ollama_tool_calls_across_chunks_are_all_kept`.

- **[P2] `LlmClient` edge cases**:
  - Clean EOF without a terminator is now an error (`is_stream_terminator`: `[DONE]`, Anthropic `message_stop`, Ollama `done:true`, or OpenAI `finish_reason`). Tests: `fixr5_stream_closed_without_terminator_is_an_error`, `fixr5_provider_specific_terminators_complete_the_stream`.
  - The line buffer is capped at 4 MiB.
  - The Anthropic tool result is now role `user` with only the `tool_result` block. Test: `fixr5_anthropic_tool_result_is_a_user_message`.
  - Gemini-style `/v1beta/openai` endpoint joining -> SKIPPED (ambiguous without a per-provider rule).
  - Risk: an OpenAI-compatible provider that sends neither `[DONE]` nor a `finish_reason` will now be reported as truncated.

## Not run

Nothing was compiled or tested here. The lead's gate should watch for:
- `frame_is_turn_progress` and the `Message::Text` branch borrow order in `execute_turn`.
- `tests/test_fix_r5_agent.rs`: it imports `mirror_to_session` and `peek_pending_cron_briefings` through the crate root (`pub use mirror::*`).
- The new daemon tests, which re-exec themselves through `isolated_home` and need `python3` like the old ones.

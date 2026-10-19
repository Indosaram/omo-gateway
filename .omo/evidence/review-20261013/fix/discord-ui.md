# discord-ui fix report

Nothing was compiled or run (per directive). All edits were made by hand and rustfmt auto-ran on save.

## discord-commands

- **[P1] /skill|/skills|/memory pending/review output posted publicly** -> FIXED
  - `commands.rs`: new `is_pending_write_action`. `skill_dispatch` and `memory` now call `ctx.defer_ephemeral()` for pending/review/approve/reject, so replies and `?` errors are ephemeral.
  - New `can_manage_skill_writes`: skill pending actions (global scope) are operator-only (`allowed_users` or `allow_all_users`). This also covers the P2 "skill approve/reject has global scope".
  - Tests: `pending_write_actions_are_classified_for_ephemeral_defer`, `skill_pending_writes_are_operator_only`.
- **[P2] /retry non-transactional, restore drops platform_message_id** -> FIXED
  - `commands.rs`: `retry_last_user_message` now uses `take_messages_from_sequence` (a single `DELETE ... RETURNING` in a transaction, so no row can be deleted without being captured) and `restore_messages`, which restores `platform_message_id`.
  - Test: `retry_rollback_restores_platform_message_id_and_concurrent_rows`.
- **[P2] /reset wipes DB before interrupting the actor, swallows actor error** -> FIXED
  - `commands.rs`: `execute_reset_command` resets the actor first with `?` (error propagated), then wipes the DB, then resets the actor again so the wiped `state_json` is re-flushed with profile defaults.
  - Covered by the existing tests `execute_reset_command` (test_discord_adapter.rs ~4088) and `slash_controls_change_authoritative_session`. No new unit test: it needs a live actor with a mid-turn race, which is not deterministic without a gate.
  - Residual: `SessionMultiplexer::reset` / `ActorHandle::reset` return `Ok(())` on a Retiring/Closed actor (router.rs, multiplexer node). See BLOCKED-SCOPE.
- **[P2] Interaction id used as platform_message_id** -> FIXED
  - `commands.rs`: new `synthetic_command_event` (empty `platform_message_id`) is used by `/skill run`, `/steer` and the `/thread` starter. `/retry` passes `String::new()`.
  - The actor already skips the reaction and dedup when the id is empty (actor.rs:313, 604).
  - Test: `synthetic_command_events_carry_no_platform_message_id`.
- **[P2] Slow slash commands not deferred** -> FIXED
  - `commands.rs`: `defer()` was added to `/model` and `/reset`. `defer_ephemeral()` was added to `/stop`, `/steer`, `/retry` and `/yolo` (they already replied ephemerally). `/thread` still has no defer.
  - Not unit-testable (poise context).
- **[P2] /compress lossy fallback when llm None + unbounded prompt** -> FIXED
  - `commands.rs`: `/compress` now refuses when `llm` is `None`. It summarizes only the oldest window bounded by `MAX_COMPRESSION_PROMPT_CHARS` (`select_compression_window`), and newer rows are preserved via the existing `cutoff_sequence` logic.
  - Test: `compression_window_is_bounded_but_keeps_at_least_one_row`.
- **[P2] Skill approve/reject global scope** -> FIXED. Operator-only gate (see P1).
- **[P2] Button restart/restore (buttons.rs)** -> PARTIALLY FIXED
  - (b) Originally disabled buttons are no longer re-enabled: `rebuild_buttons` uses `disabled || button.disabled`. Test: `restore_keeps_originally_disabled_buttons_disabled`.
  - (c) `set_buttons` retries the edit 3 times (500ms apart), so the final state edit is less likely to leave a stale enabled button.
  - (a) Reconcile-on-restart and (d) bounded output capture / pipe-inheriting grandchildren -> SKIPPED. They need persisted in-flight state or a rewrite of `run_button_action`, which is a design change.
- **[P2] pending list all-or-nothing on one malformed payload** -> BLOCKED-SCOPE. The change is in `src/storage/db.rs` `list_pending_writes_scoped`: replace `scope.permits(&item)?` with a match that logs and skips `Err` rows when listing (keep `?` for get/approve/reject).

## discord-auth

- **[P1] Table lost when PNG upload fails** -> PARTIAL, needs adapter.rs
  - FIXED in scope (`table_render/mod.rs`):
    - Tables with malformed rows (more cells than headers), more than 300 rows, or an image over 32M pixels now stay as raw text instead of being replaced by a placeholder.
    - New `pub fn fallback_text(&[RenderedTable]) -> String` produces fenced original text.
    - `svg_to_png` uses a process-wide `OnceLock` font database (no more `load_system_fonts()` per call) and the pixel cap.
    - Tests: `malformed_rows_keep_raw_text`, `fallback_text_fences_original_tables`.
  - BLOCKED-SCOPE (`src/discord/adapter.rs`, discord-adapter node):
    - At the 3 `dispatch_rendered_tables` call sites (~3128, 3263, 3407) and the `post_channel` site (~3314), treat upload failure as non-fatal.
    - On error, send `table_render::fallback_text(&rendered_tables)` as a follow-up message (chunked), log a warning, and continue.
    - On the stream path, remove the stream entry before or regardless of the table result.
    - Also run `transform_markdown_tables_to_images` under `spawn_blocking`.
- **[P1] Table cell splitter ignores `\|` and pipes in backticks** -> FIXED
  - `table_render/mod.rs`: `split_table_cells` splits on unescaped `|` outside balanced backtick spans and unescapes `\|`. Rows with more cells than headers fall back to text (see above).
  - Test: `escaped_pipes_and_code_span_pipes_stay_in_one_cell`.
- **[P1] Typing failure aborts the final stream update** -> FIXED
  - `throttler.rs`: `start_typing` is best-effort (logged, `?` removed). It is still called before the final edit because the existing tests in tests/test_discord_adapter.rs expect `Typing` before the final `Edit`.
  - Test: `final_update_is_delivered_even_when_typing_fails`.
- **[P1] Unknown pairing code bumps attempts on ALL pending codes** -> FIXED
  - `pairing.rs`: the unknown-code branch only records the platform failure; the blanket `UPDATE pairing_codes SET attempts = attempts + 1` is removed. The platform lockout is the global brake.
  - Updated existing assertions in `pairing_preserves_lockout_across_fresh_requests` and `pairing_throttles_notifications_and_preserves_lockout` (attempts now stay 0; the platform lock and `LockedOut` assertions are unchanged).
  - Test: `operator_typos_do_not_invalidate_pending_codes_after_lockout`.
- **[P1] APPROVAL_MODE=always 3-token key pre-approves chained commands** -> BLOCKED-SCOPE
  - Security node owns `derive_pattern_key` (`src/security/dangerous.rs`); the fix has to land there.
  - The `approval.rs` callers (`request_approval` line ~250 and the tests) pass whatever key it returns, so no `approval.rs` change is needed unless the signature changes.
- **[P2] Failed Always persistence reported as approved** -> SKIPPED. The user-facing message is in `adapter.rs` (button handler); `approval.rs` already fails closed with `ApprovalError::Persistence`.
- **[P2] Caller waits up to 10s for the expiry edit** -> FIXED
  - `approval.rs` `request_with_scope`: after dropping `terminal`, the delivery task is awaited with a 2s timeout (the task still finishes on its own).
  - Test: `decision_is_not_delayed_by_a_hanging_expiry_edit` (start_paused, no real sleeps).
- **[P2] Attachment cache never pruned** -> FIXED
  - `attachments.rs`: `prune_attachment_cache` runs at construction and removes cached files older than 7 days and `.part` files older than 1 hour.
  - Test: `cache_prune_removes_only_stale_files` (injected `now`).
  - The "download only the types actually used" idea is SKIPPED (behavior change).
- **[P2] 15s timeout covers whole attachment body** -> FIXED
  - `attachments.rs`: the client uses `connect_timeout` + `read_timeout` (15s each), plus a `DISCORD_ATTACHMENT_TOTAL_TIMEOUT` (300s) around `stream_to_file`.
  - No unit test: it needs a real slow HTTP server.
- **[P2] WAV parser padding and non-16-bit PCM** -> FIXED
  - `attachments.rs` `decode_wav_pcm`: RIFF pad byte is skipped, only 16-bit PCM (tag 1 or 0xFFFE) is decoded, and the `data` size is clamped for streamed WAVs.
  - Tests: `wav_decoder_skips_pad_byte_after_odd_sized_chunk`, `wav_decoder_rejects_non_16_bit_or_non_pcm`, `wav_decoder_clamps_streamed_data_size`.
- **[P2] Table rasterisation blocks runtime, reloads fonts, no size bound** -> PARTIAL
  - FIXED: font DB cached in a `OnceLock`, rows and pixel cap.
  - `spawn_blocking` is BLOCKED-SCOPE (the call sites are in `adapter.rs`; `transform_markdown_tables_to_images` is a sync pub fn).
- **[P2] Pairing capacity eviction by unauthenticated DM flood** -> SKIPPED. Rejecting instead of evicting would lock legitimate users out under the same flood; the reviewer rated it hardening only.
- **[P2] Sync paired-user check returns false while write lock held** -> FIXED
  - `pairing.rs`: `paired_cache` is now a `std::sync::RwLock` that is never held across `.await`. Sync and async readers use the blocking `read()`, so the result is never spuriously `false`.
  - Covered by the existing `test_pairing_lifecycle_and_approval` (`is_user_paired` / `is_user_paired_sync`).
- **[P2] Approval and pairing grants cannot be revoked** -> SKIPPED. It is a new feature (commands plus an allowlist/paired-users DELETE API); it is not a defect fix.

## BLOCKED-SCOPE summary (exact changes for other nodes)

1. `src/discord/adapter.rs`: non-fatal table upload failure with `fallback_text` (details above); `spawn_blocking` for rendering.
2. `src/security/dangerous.rs` `derive_pattern_key`: no 3-token prefix grants for non-dangerous commands in always mode (security node).
3. `src/storage/db.rs` `list_pending_writes_scoped`: skip and log unparsable rows when listing.
4. `src/multiplexer/router.rs` `SessionMultiplexer::reset` and `ActorHandle::reset`: do not swallow the actor error or return `Ok` on `SendOutcome::Retiring`; use `wait_until_reusable()` and retry like `route`/`stop`. Until then, `/reset` still has the Retiring-window hole.

## Test-compat notes

- `tests/test_discord_adapter.rs` expects `Typing` before the final `Edit`, so typing is still called, but best-effort.
- `reset` is now called twice per `/reset` (before and after the DB wipe), both with `?`.

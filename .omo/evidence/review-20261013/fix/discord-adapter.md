# discord-adapter fix report (write scope: src/discord/adapter.rs)

Nothing was compiled or run. The file was only passed through `rustfmt --check`, which is clean. All new tests are in the new `fix_r5_discord_adapter_tests` module at the end of `src/discord/adapter.rs`.

## P1

### MEDIA directive false positive aborts the whole reply (a: P1, b: P1)
FIXED.
- `MEDIA_DIRECTIVE_RE` is now case-sensitive and anchored with `(^|\s)`, so "Social media: X" and "multimedia: foo" no longer match.
- New `DiscordEgress::deliver_media_directives` validates and uploads each path without aborting the text. A missing path is a false positive: it is only noted in the text. An unauthorized path or a failed upload is also noted in the text, and the first error is returned after the reply has been delivered.
- Both `stream()` (final) and `dispatch(SendMessage)` use it. Because the error comes after delivery, the stream entry is kept so a retry no-ops on `last_sequence`.
- Mid-turn status sent as `SendMessage` goes through the same path, so media or table problems can no longer drop its text.
- Tests: `media_directive_requires_boundary_and_uppercase`, `send_message_with_media_lookalike_prose_is_delivered`, `send_message_missing_media_still_delivers_text`, `send_message_unauthorized_media_delivers_text_then_errors_without_upload`, `final_stream_with_bad_media_keeps_text_and_does_not_repost_on_retry`.

### MEDIA upload allows the cwd, so `.env` and the DB can be exfiltrated (a: P1, b: P1)
FIXED.
- `validate_media_path` no longer treats the cwd as an authorized root.
- New `validate_media_path_in(raw, extra_roots)` allows the temp directories and the egress `workspace_root` (main.rs already sets it). Roots are compared as path components after canonicalization, not as string prefixes, and the target must be a regular file.
- New `media_path_is_denied` blocks `.env`, `.env.*`, `*.env`, `*.db`, `*.sqlite`, `*.sqlite3`, `*.db-wal`, `*.db-shm`, `*.pem`, `*.key`, and anything under `.ssh` or `.gnupg`.
- `OutboundAction::UploadFile` refuses denied files too.
- Tests: `media_path_rejects_cwd_and_secret_files`, `upload_file_refuses_secret_bearing_files`.

### `coalesce_inbound_events` drops `author_id` and `referenced_message_id` (a: P1)
FIXED in `coalesce_inbound_events`.
- It now carries the most recent non-empty `author_id` and `referenced_message_id` from the constituents, and keeps `origin`.
- Test: `coalesce_preserves_author_reference_and_origin`.

### Receive watchdog is never started in production (a: P1)
FIXED.
- `spawn_receive_watchdog` now runs inside `DiscordAdapter::client()`, which main.rs calls for every bot. `start()` no longer spawns it, so it is not started twice.
- `LAST_DISCORD_EVENT_MS` is now a per-client map keyed by the `ShardManager` pointer. The poise `event_handler` records the event through `record_dispatch_event`, so one healthy bot cannot mask a zombie shard on another.
- Test: `receive_watchdog_tracks_each_client_separately`. It checks the per-client slots only. Arming the watchdog through `client()` is not tested.

### Table replaced by placeholder, image upload failure loses the table (b: P1)
FIXED.
- `dispatch_rendered_tables` now posts `table_render::fallback_text` (the original markdown in a fence) when an image batch upload fails, then returns the error.
- On the stream path the entry is no longer lost to an early `?`: the entry is kept on an error and removed on success.
- Test: `table_upload_failure_falls_back_to_original_markdown`.

### Owner DM of a 64 KiB lookup result exceeds Discord's 2000-character limit (b: P1)
FIXED.
- The owner-proposal reply path in `handle_event` now sends `owner_dm_chunks(&text)`: `chunk_markdown` at the Discord limit, bounded to `MAX_SPLIT_MESSAGES` with the truncation notice.
- Test: `owner_dm_and_edit_content_fit_discord_limits`. It covers the chunking helper, not the `handle_event` send loop.

## agent-backend: mid-turn status visibility
HANDLED BY CONTRACT, no special handling added.
- `completed_stream_content` is unchanged: non-final chunks are still dropped, and the existing test `discord_stream_delivers_only_completed_content` still passes unchanged.
- Once omo-backend emits mid-turn status as `OutboundAction::SendMessage`, it goes through the normal send path, where media and table handling can no longer abort the text.

## P2 fixed
- **Any 403 or 404 marks the channel dead (a, b)**
  - Fixed. `is_discord_dead_target_error` now marks dead only for 404 Unknown Channel (10003) and 403 Missing Access (50001).
  - New `is_discord_send_dead_target_error` also treats 50013 as dead, and is used only by the SendMessage and ApprovalRequest paths.
  - A 404 with code 0 and 50005 or 50013 on edits or uploads no longer mark the channel dead.
  - Test: `dead_target_classification_is_channel_scoped`.
- **Typing and `keep_typing` consume the one-shot probe slot (a, b)**
  - Fixed. New read-only `DeadTargetRegistry::is_dead_for_bot_peek` is used in `keep_typing` and the Typing arm.
  - Test: `dead_target_peek_does_not_consume_probe_slot`.
- **`clear_for_bot` spawns a DB write on every send (a)**
  - Fixed. It now spawns only when an in-memory entry was actually removed.
  - Test: `clear_for_bot_reports_only_real_removals`. It checks the return value only, not whether a DB task is spawned.
- **`streams` mutex held across the placeholder network send (a, b)**
  - Fixed. The map holds `Arc<tokio::sync::OnceCell<Arc<ActiveDiscordStream>>>`, and the lock is released before the placeholder send. A failed init removes the empty cell.
  - Test: `slow_placeholder_send_does_not_block_other_streams`. It uses Notify with bounded timeouts.
- **`filter_reasoning` is used only for the silence check (b)**
  - Fixed. SendMessage now builds from `filtered_content`: runtime footer, media extraction, and the forum post title. Edits send the filtered text bounded to a single message through `edit_message_content`.
  - Tests: `send_message_strips_reasoning_before_delivery`, `owner_dm_and_edit_content_fit_discord_limits`.
- **`React` panics on `ChannelId::new(0)` (b)**
  - Fixed. React returns `OmonError::Config` for a non-numeric or zero channel ID, and `target()` rejects 0.
  - Test: `react_with_non_numeric_channel_errors_instead_of_panicking`.
- **`/stop` can be replayed as a prompt by backfill (a, b)**
  - Fixed. `route_claimed_event_awaiting_turn` returns `Ok(false)` for a text `/stop`.
  - Test: `backfill_replay_of_text_stop_is_not_routed_as_a_prompt`.
- **Live path never advances the bot cursor backfill reads (b)**
  - Fixed. New `advance_live_bot_cursor` is called after a successful ledger claim in `route_claimed_event_with_constituents` and `route_claimed_event_awaiting_turn`. It uses the session's `bot_id` and the thread ID if there is one.
  - Test: `live_routing_advances_the_bot_cursor_backfill_reads`.
- **Cursor write hits SQLITE_BUSY (b)**
  - Fixed. `update_bot_channel_cursor` now uses `begin_with("BEGIN IMMEDIATE")`.
  - Test: `bot_cursor_update_is_monotonic_under_immediate_transaction`. It checks monotonic updates only, not the BUSY behavior itself.
- **`bind_lane_thread` mismatch aborts message handling (a)**
  - Fixed. `intercept_lane_owner_reply` returns `Ok(false)` and logs on a `Config` mismatch error, so the message reaches normal routing.
  - Test: `lane_owner_reply_with_moved_thread_owner_falls_through`.
- **`UploadFile` bypasses validation and reads without a size cap (a)**
  - Fixed. Denied files are refused, and `SerenityFileUploader` checks size against `DISCORD_UPLOAD_MAX_BYTES` (100 MiB) before reading.
  - Test: `upload_file_refuses_secret_bearing_files`. The size cap itself is untested.
- **Per-message REST calls and content logging (a)**
  - Partially fixed. `get_current_user()` is replaced by `ctx.cache.current_user().id`, and the INFO log now records `content_len` instead of the content.
  - The cursor write and `to_channel` before the authorization filter are not moved (see Skipped).
  - No test.
- **Hard-coded owner ID fallback (a)**
  - Partially fixed. `intercept_lane_owner_reply` now uses the shared `status_reply::owner_id()`, so owner resolution is consistent.
  - That function still falls back to `inbox::proposals::OWNER_ID` when `DISCORD_ALLOWED_USERS` is unset, so it does not fail closed.
  - No test.

## SKIPPED
- **Debounce batches routed out of order (a)**
  - Needs a design decision (per-session routing serialization or hydrating before the debounce window). That is a larger change than a minimal fix.
- **`thread_sessions_per_user` never read (a)**
  - Needs a decision on whether the option should be wired into session keys or removed. That affects session-key stability.
- **Cursor write, `to_channel` and `get_member` before the authorization filter (a)**
  - Moving them changes the cursor semantics that backfill relies on, and the unauthorized-DM pairing flow needs the metadata.
- **`mark_dead_for_bot` and `clear_for_bot` persistence ordering (a)**
  - The ordering race is not fixed. The remaining clear-then-mark race needs per-key serialization, which is invasive.
  - Only the per-send spawn overhead was fixed.
- **Watchdog restarts only `ShardId(0)` (a)**
  - Single-shard deployment; left as is.

## BLOCKED-SCOPE
None. No change is needed outside `src/discord/adapter.rs`. In particular `main.rs` needs no change, because the watchdog is armed inside `DiscordAdapter::client()`.

## Notes for the gate
- **Existing test, `final_media_directive_uploads`:** it expects `Err` for `/etc/passwd`. That still holds: the unauthorized path is surfaced as an error after the text is delivered, and the uploader count stays at 1.
- **`classify_dead_target` is private and used only by in-file tests:** if the build is warning-strict, `is_discord_send_dead_target_error` is public, so there should be no dead-code warning.

# DoneClaim: Discord Thread Lifecycle Handling

## Summary
Handled Discord thread lifecycle events in `src/discord/adapter.rs`:
- `FullEvent::ThreadCreate`: logs creation with thread ID, parent ID, and thread name.
- `FullEvent::ThreadUpdate`: detects archive state from thread metadata, stops multiplexer on archive, and updates `sessions.metadata.thread_archived` to `true` (archive) or `false` (unarchive).
- `FullEvent::ThreadDelete`: stops multiplexer for the thread session and removes `omo_thread_id` from `sessions.metadata` while preserving session row.

## Tests
- Verified with `tests/test_discord_thread_lifecycle.rs`:
  - `test_thread_archive_updates_metadata_and_stops_multiplexer`: passed
  - `test_thread_unarchive_updates_metadata`: passed
  - `test_thread_delete_cleans_up_omo_thread_id_preserving_session`: passed
- `cargo test --test test_discord_thread_lifecycle` output: 3 passed, 0 failed.

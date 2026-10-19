# storage-db review

I found 5 defects. No P0s. I did not run anything, so F1 rests on SQLite's documented semantics rather than an executed repro.

### [P1] Migration 0041 drops the FK-parent `inbox_items` table while child rows exist; likely fails the whole migration at commit
- Location: migrations/0041_cron_remediation.sql:17 (`DROP TABLE inbox_items;`; the `PRAGMA defer_foreign_keys=ON;` is at line 1 and the copy is at line 16)
- Trigger:
  - A production DB that already holds `inbox_decisions` rows, which are populated by `src/inbox/triage.rs:258` and `src/cron/remediate.rs:321`. `inbox_decisions.item_id REFERENCES inbox_items(id)` (migrations/0034_inbox.sql:17). `inbox_proposals` also references it (src/inbox/proposals.rs:290).
  - The gateway starts and `Database::connect` runs `MIGRATOR.run`. `foreign_keys(true)` is set on the connection (src/storage/db.rs:~137), and sqlx runs each migration in a transaction.
  - Under SQLite semantics, `DROP TABLE` with FKs on does an implicit `DELETE FROM inbox_items`. Under `defer_foreign_keys` each child row bumps the deferred-violation counter. The rows were copied into `inbox_items_cron` before the drop, and `ALTER TABLE ... RENAME` does not decrement the counter. COMMIT should then fail with "FOREIGN KEY constraint failed".
  - Existing tests only run migrations on empty DBs, so they cannot catch this.
- Impact: Migration 0041 cannot apply on any DB that has triaged inbox items. `Database::connect` returns an error at boot (`MIGRATOR.run(&pool).await?`), so the gateway never starts on upgrade. Rolling back is the only way out.
- Fix: Use the table-rebuild pattern that does not delete parent rows while children exist. Either add a nullable-safe `source` CHECK by recreating the table with `PRAGMA foreign_keys=OFF` (needs `-- no-transaction`), or create the new table under the final name and move the rows in a way that re-inserts parents after the drop. Add a migration test that seeds `inbox_items` plus `inbox_decisions` before applying 0041.

### [P2] `is_silence_response` regex has an escaping typo, so legitimate short replies are treated as silence and dropped
- Location: src/models/events.rs:69
- Trigger:
  - The pattern is a raw string, so the trailing class `[\\s*_~`]*$` is `{ '\\', 's', '*', '_', '~', '`' }`, not whitespace. That means `no\s+response` followed by `s` matches. `^...(silent|silence|no response|no reply)...[...]*$` therefore matches "No responses", "silents", "silences" and similar.
  - A model reply of exactly "No responses" (64 characters or fewer) makes `is_explicit_silence` true.
  - The callers `src/discord/adapter.rs:3049`, `3185` and `3394` then return `Ok(())` without sending. The `src/agent/omo_backend.rs` call sites suppress the reply the same way.
- Impact: A legitimate one-line answer is silently swallowed and the user gets no reply. The turn is still recorded as delivered.
- Fix: Change the class to `[\s*_~`]*` (single backslash) in the trailing position.

### [P2] `prune_terminal_obligations` is never called, so `delivery_obligations` grows without bound
- Location: src/ledger/service.rs:541
- Trigger:
  - Every outbound reply calls `record_obligation` (src/agent/omo_backend.rs:644, src/multiplexer/router.rs:394, src/cron/scheduler.rs:1985). Each call stores the full reply `content` in `delivery_obligations`.
  - Rows reach `delivered` or `abandoned` and are never deleted. A grep of src/ and tests/ shows no caller of `prune_terminal_obligations`; `src/main.rs:916` prunes only cron runs.
- Impact: The table, which holds a full copy of every reply, grows linearly forever. This bloats the DB and WAL and slows the startup `sweep_recoverable` scan over `state IN (...)` rows.
- Fix: Call `prune_terminal_obligations(retention_secs, max_rows)` at boot next to `prune_terminal_cron_runs` in `src/main.rs`, and periodically after that.

### [P2] Ingress-indexed guild messages get an empty `author_name`
- Location: src/storage/message_search.rs:225
- Trigger:
  - `index_inbound_with_metadata` sets `author_name: session.user_id.clone()`.
  - For guild sessions, `SessionKey::new` and `from_storage_key` force `user_id` to `""` (src/models/session.rs, the `user_id` handling).
  - Every guild message indexed from `src/multiplexer/actor.rs:903` and `src/discord/adapter.rs:1472` therefore gets `author_name = ""`.
- Impact: The FTS `author_name` column is useless for these rows. `message_context` search and get return blank author names until a REST backfill overwrites the document. Re-ingesting the same message also overwrites the richer REST document with the minimal one.
- Fix: Take the name from the event or message author (for example, carry the author display name in `InboundEvent`). Do not use `session.user_id`. If the name is unavailable, use `event.author_id` rather than an empty string.

### [P2] Thread messages are indexed under the parent `channel_id`, so channel-scoped search mixes threads and parent
- Location: src/storage/message_search.rs:221
- Trigger:
  - For thread messages, the Discord adapter builds the session with `channel_id = parent_channel_id` and `thread_id = message.channel_id` (src/discord/adapter.rs ~1966-1983).
  - `index_inbound_with_metadata` copies `session.channel_id` (the parent) as the document `channel_id`, and `search()` filters only on `d.platform` and `d.channel_id` (message_search.rs:160).
  - A message_context search targeting the parent channel, from any session authorized for that channel, therefore returns messages from every thread under it. This includes private threads the bot was invited to.
  - Conversely, a search whose target is the thread id finds none of the ingress-indexed thread messages. REST-backfilled docs use `message.channel_id`, which is the thread id (src/tools/message_context.rs:1110).
- Impact: Thread content is visible to parent-channel readers regardless of Discord's thread membership. Search results for threads are also inconsistent between the ingress and REST sources, and the same message can be indexed twice under different channel keys.
- Fix: Key ingress documents by the actual conversation channel (`thread_id.unwrap_or(channel_id)`), matching the REST path. Keep the parent in a separate column or metadata if it is needed.

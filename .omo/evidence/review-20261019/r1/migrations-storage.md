# Lane migrations-storage

### [P2] New per-turn and per-item tables grow without bound because nothing ever deletes from them
- Location: migrations/0047_turn_status_messages.sql:1
- Trigger: Every turn that posts a status message upserts a row keyed by `(session_key, event_id)` (src/report/status.rs:193). Likewise, every action-loop item inserts into `action_loop_seen` (src/autonomy/loop.rs:89, 185), every watch notice into `session_watch_notices` (src/sessions/watch.rs:296), and every report claim into `report_claims` (src/report/claims.rs:276, 292). A search across `src` finds no `DELETE FROM` for any of these four tables.
- Impact: The SQLite file grows slowly on a long-running daemon. Nothing breaks and nobody sees it. Older tables (`lane_events`, `messages`) already grow the same way, so this is not a regression. That is why it is P2 and not an "accumulating leak" P1.
- Evidence:
  ```sql
  CREATE TABLE IF NOT EXISTS turn_status_messages (
      session_key TEXT NOT NULL,
      event_id TEXT NOT NULL,
      platform_message_id TEXT NOT NULL,
      started_at DATETIME NOT NULL,
      state TEXT NOT NULL DEFAULT 'running' CHECK (state IN ('running', 'done', 'failed')),
  ```
- Fix: Add a startup prune next to `prune_terminal_cron_runs` (src/main.rs:1478), for example `DELETE FROM turn_status_messages WHERE state <> 'running' AND julianday(updated_at) < julianday('now','-30 days')`, plus similar age-based deletes for `action_loop_seen.seen_at` and `session_watch_notices.sent_at`.

Every other hunk was reviewed and confirmed against the current files, with no issues found:

- **0045 / 0051 inbox table rebuilds:** The child FK `inbox_decisions.item_id REFERENCES inbox_items(id)` (0034_inbox.sql:17) survives the drop-and-recreate. `PRAGMA defer_foreign_keys=ON` plus re-inserting every row before commit keeps it consistent; the pool connects with `.foreign_keys(true)` (src/storage/db.rs:165). No indexes or triggers sit on `inbox_items` or `inbox_filter_stats`, so nothing is lost by the drop, and the column lists match 0041 for the `SELECT *` copy.
- **0049 legacy allowlist import:** It copies `approval_allowlist.pattern_key` rows as `allow`/`exact`. That matches what the old code already trusted: e57b472 loaded `approval_allowlist` into the always-allowed cache, and the current `load_persisted_allowlist` (src/discord/approval.rs:523) reads the same set. No new grants are created. The forward reference from `precedent_id` to `action_precedents` is legal in SQLite.
- **`release_at` filter** (src/storage/lanes.rs:863, 878): `julianday()` handles sqlx's RFC3339 strings with a timezone. NULL means deliver now. Production calls go through `take_undelivered_events` in src/lanes/dispatch.rs:52.
- **`pause_lane` / `resume_lane`** (src/storage/lanes.rs:234, 263): Both run in `BEGIN IMMEDIATE`. An early `Err` return drops the transaction, which rolls it back. The pause and resume history rows have `from_status = to_status`, so they are excluded from the `to_status='done_claimed' AND from_status != 'done_claimed'` check (src/lanes/settle.rs:83) and from the not_done count (src/lanes/runner.rs:1617). Nothing deletes or re-keys `lanes`, so the `lane_pauses` FK cannot be orphaned.
- **0043, 0044, 0046, 0050, 0052, 0053, the db.rs test-list hunk and the mod.rs re-export:** Additive schema and exports only. The partial unique index on `session_ref` matches its users in src/tracker/sessions.rs.

<results>
<files>
- /Users/indo/code/project/omon-gateway/migrations/0047_turn_status_messages.sql - table with no pruning (the one P2)
- /Users/indo/code/project/omon-gateway/migrations/0045_slack_channel.sql - inbox rebuild, FK safety confirmed
- /Users/indo/code/project/omon-gateway/migrations/0051_inbox_github.sql - inbox rebuild, FK safety confirmed
- /Users/indo/code/project/omon-gateway/migrations/0049_action_ledger.sql - precedents table, `release_at` column, legacy allowlist import
- /Users/indo/code/project/omon-gateway/src/storage/lanes.rs - pause/resume and `release_at` filtering
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - consumer of the imported precedents
- /Users/indo/code/project/omon-gateway/src/lanes/dispatch.rs - production caller of `take_undelivered_events`
</files>
<answer>One P2 (unbounded table growth). No P0 or P1 in the migrations-storage lane.</answer>
<next_steps>Ready to proceed - no follow-up needed</next_steps>
</results>
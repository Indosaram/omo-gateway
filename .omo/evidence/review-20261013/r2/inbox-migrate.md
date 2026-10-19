# inbox-migrate review

I read only the current files and could not run `git diff`, so "this round's fixes" are judged against `.omo/evidence/review-20261013/fix/inbox-migrate.md`. The fixes I checked are correct as written: mail follow-up refresh, per-item heartbeat handling, the 1800-char owner text bound, token preservation in the importer, the cutover failed-receipt settlement, the 10s SIGTERM grace, PCM frame concatenation, KST-day drop counts, and `kill_on_drop` in `run_json` and `run_bounded`. The new tests can fail, and I found no timing-luck tests. The two P1s and four P2s below are all the remaining defects I found.

### [P1] `message_search_transcript_ad` full-scans `message_search_documents` for every deleted message
- Location: /Users/indo/code/project/omon-gateway/migrations/0042_message_search_cleanup.sql:19-32
- Trigger: Any `DELETE FROM messages` on rows that have a `platform_message_id`. Examples are `replace_history_with_summary` (compaction), `/new` or reset in `discord/commands.rs`, and the deletion of a session's messages. The trigger body filters on `message_id = old.platform_message_id` with no `platform` or `channel_id` equality. The table's indexes are the PK `(platform, channel_id, message_id)` and `(platform, channel_id, timestamp)`, and neither has `message_id` as its leading column. No `ANALYZE` is ever run, so skip-scan does not apply. Each deleted message therefore scans the whole table. It also scans on the cascaded deletes when a session is deleted, because the session's `EXISTS` check is evaluated after the `message_id` match.
- Impact: Cost is deleted-rows × indexed-docs, all inside one write transaction. With about 100k indexed docs (REST-backfilled guild history) and a compaction removing about 150 user messages, the transaction can hold the write lock for many seconds. Other writers (cron, ledger, ingress) then exceed the 5s `busy_timeout` and fail with SQLITE_BUSY.
- Fix: Anchor the delete on the PK prefix using scalar subqueries, e.g. `WHERE platform=(SELECT s.platform FROM sessions s WHERE s.session_key=old.session_key) AND channel_id=(SELECT s.channel_id FROM sessions s WHERE s.session_key=old.session_key) AND message_id=old.platform_message_id AND json_extract(metadata_json,'$.source')='transcript'`. Alternatively, add an index on `message_search_documents(message_id)`.

### [P1] `rollback_phase1_cron.sh` aborts before restoring the legacy cron by default, and the cutover default DB path was changed without it
- Location: /Users/indo/code/project/omon-gateway/scripts/rollback_phase1_cron.sh:8 (`DB_PATH` default `data/gateway.db`), and the Step 1 `sqlite3 "${DB_PATH}" "UPDATE cron_jobs SET enabled = 0 ..."` call
- Trigger: This round changed the `cutover_phase1_cron.sh` default to `${REPO_ROOT}/omon_gateway.db`, but the rollback still defaults to `data/gateway.db`.
  - Case 1: The operator runs the rollback with defaults. The file does not exist (a find for `data/*.db` found none), so it prints "Target database file not found" and exits 1 under `set -e`.
  - Case 2: The path is correct but the gateway is running, which it must be for the jobs to be enabled. The `sqlite3` CLI has no busy timeout by default, so the UPDATE fails with "database is locked". The cutover script got `-cmd ".timeout 10000"` this round, and the rollback did not.
- Impact: In both cases Step 2 never runs, so the legacy launchd job and crontab are not restored. The operator needs a rollback in an emergency and gets an abort. The gateway cron jobs stay enabled, and the legacy side stays disabled.
- Fix: Use the same default path as the cutover script and add `-cmd ".timeout 10000"`. Preferably make Step 1 non-fatal (warn and continue to Step 2) so the legacy side is always restored.

### [P2] A mail refresh can be overwritten by an in-flight triage batch, losing the follow-up
- Location: /Users/indo/code/project/omon-gateway/src/inbox/mod.rs (`commit_source`, mail refresh branch) against /Users/indo/code/project/omon-gateway/src/inbox/triage.rs (`classify`, second transaction after the `Proposals::new` call at line 320)
- Trigger: Thread T is collected with snippet A, and `flush` reads it. During the judge calls (one per item) and the writer call (up to 180s), a reply arrives. The collector task (`run_event_watch_with_shutdown`) refreshes the item to snippet B and deletes decisions, but none exist yet. `classify` then runs `INSERT ... ON CONFLICT(item_id) DO NOTHING` using the class judged from A.
- Impact: The decision for A sticks to the refreshed item B. B is never re-triaged, so an "owner must act" follow-up can stay classified fyi.
- Fix: Make each decision insert conditional on the item hash it was judged from, e.g. `... WHERE EXISTS (SELECT 1 FROM inbox_items WHERE id=? AND raw_hash=?)`. Alternatively, have `classify` re-check `raw_hash` before committing.

### [P2] Re-triage after a mail refresh can leave a bot item stuck when an old proposal row exists
- Location: /Users/indo/code/project/omon-gateway/src/inbox/mod.rs (`commit_source` deletes decisions in `'open','done','dismissed'`), together with `Proposals::propose_in` in /Users/indo/code/project/omon-gateway/src/inbox/proposals.rs
- Trigger: A bot item has a proposal with state `pending`, and the owner runs `inbox set --status dismissed` (or `done`). `update_status` leaves the proposal row alone. A follow-up mail arrives in the thread, so the decision is deleted and the item is re-classified bot with a new `proposed_action`. `propose_in` uses `ON CONFLICT(item_id) DO NOTHING`, so the old proposal row, with its old `proposed_action`, survives.
- Impact: The new decision is marked `proposed`, but `pending()` and the `execute_approved` CAS both require `d.proposed_action = p.proposed_action`. The new proposal never shows in "제안" and can never be approved (it reports AlreadyClaimed).
- Fix: In the refresh branch, also delete the item's pending `inbox_proposals` row (or reset it to the new action) when deleting the decision.

### [P2] The search-cleanup session trigger misses thread-session documents
- Location: /Users/indo/code/project/omon-gateway/migrations/0042_message_search_cleanup.sql:4-16, against `MessageSearchIndex::index_inbound_with_metadata` in /Users/indo/code/project/omon-gateway/src/storage/message_search.rs
- Trigger: A thread session (sessions.channel_id is the parent channel and thread_id is set) has its messages indexed by `index_inbound`, which keys the document on `channel_id = session.thread_id` with source `ingress`. The `BEFORE DELETE ON sessions` trigger matches `channel_id = old.channel_id`, which is the parent channel. The one-time purge only touches `source='transcript'` documents.
- Impact: Deleting a thread session (dashboard delete, malformed-session erase) leaves the ingress documents for private-thread content searchable. The `message_search_documents_ad` trigger removes them from the FTS index only when a document row itself is deleted, and none is.
- Fix: Also purge `channel_id = old.thread_id` when `old.thread_id IS NOT NULL`.

### [P2] The `launchctl list | grep -q` guards can skip the bootout under `pipefail`
- Location: /Users/indo/code/project/omon-gateway/scripts/cutover_phase1_cron.sh, Step 2 (`launchctl list | grep -q "com.omomeow.cron"`), and the same pattern in /Users/indo/code/project/omon-gateway/scripts/cutover_phase3_inbound.sh, Step 1
- Trigger: `set -euo pipefail` is on, and the match appears before the end of a long `launchctl list` output. `grep -q` exits early and the writer gets SIGPIPE. The pipeline status becomes 141, so the `if` is false and the bootout is skipped.
- Impact: The plist is renamed with `mv`, but the loaded legacy job keeps running. That gives duplicate cron runs, or a duplicate Discord bot after phase 3. I did not confirm the output size at which this happens.
- Fix: Capture the output first (`out=$(launchctl list || true)`) and then `grep -q <<<"$out"`. Alternatively use `launchctl print "$SERVICE" >/dev/null 2>&1`.

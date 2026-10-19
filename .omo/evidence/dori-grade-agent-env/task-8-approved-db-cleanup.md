# Task 8 owner-approved database cleanup receipt

Executed 2026-10-11 UTC. Owner selected backup then delete row. Exactly one incident row was deleted from `/Users/indo/code/project/omon-gateway/omon_gateway.db`; COMMIT completed at `2026-10-11T13:54:21.141Z`. No migration, table, dependent row, configuration, service, or product source was changed. No git commit was made.

## Inspection and backup

The complete `task-8-live-db-incident.md` was read before database access. Inspection used `node:fs/promises.stat`, `/usr/sbin/lsof -nP` with the exact database/WAL/SHM paths, and `/bin/ps -axo pid,comm` (process names only, not arguments or environment). No open descriptors for the target were found. A worktree test executable was present, but no gateway writer was identified. The main file had inode 2166285453, size 157822976, UID 501, mode 0644, and mtime `2026-10-11T06:52:53.436Z`. Existing WAL was zero bytes; SHM was 32768 bytes; no rollback journal existed. Normal read-only access succeeded. Immutable access was never used for the source database.

Read-only SQL was run through `/usr/bin/sqlite3 -readonly` with `PRAGMA query_only=ON; BEGIN; ...; ROLLBACK;`. The complete incident row matched the report. `lane_history`, `lane_events`, `lane_transcript`, `lane_inbox`, and `judge_calls` each contained zero rows. Neither incident nor lead session key had a matching sessions row. Migration versions 31-33 had success=1 and the expected checksums.

Backup directory created with `fs.mkdir(path, {mode: 0o700})`; mode 0700 was verified. No existing permissions were altered.

Retained backup:

`/Users/indo/.omon/backups/qa-1-cleanup-2026-10-11T13-52-18-732Z/omon_gateway.db`

Actual online backup command (exit 0):

```sh
/usr/bin/sqlite3 -readonly /Users/indo/code/project/omon-gateway/omon_gateway.db ".backup '/Users/indo/.omon/backups/qa-1-cleanup-2026-10-11T13-52-18-732Z/omon_gateway.db'"
```

The completed backup was 157822976 bytes, inode 2457809202, mtime `2026-10-11T13:52:22.131Z`. A Bun read-only open and normal sqlite3 backup validation failed with error 14 because the backup has WAL-mode metadata but no WAL/SHM pathnames. Inspection confirmed no backup WAL, SHM, or journal; the only descriptor was the owned failed read-only Bun handle. The backup was the completed SQLite online backup, with no writer or ongoing backup operation. Validation therefore used only this quiescent checkpointed backup URI:

`file:/Users/indo/.omon/backups/qa-1-cleanup-2026-10-11T13-52-18-732Z/omon_gateway.db?immutable=1`

`/usr/bin/sqlite3 -readonly` against that URI returned `ok` for both `PRAGMA quick_check` and `PRAGMA integrity_check`, and exactly one `qa-1` row. The backup file, containing directory, and `/Users/indo/.omon/backups` directory were each opened and fsynced with `FileHandle.sync()` and closed before deletion.

SHA-256, computed by streaming the backup through `node:crypto.createHash('sha256')`:

`0215a037c00f882a56db254edfc756326f385bcbaff5baeaa8f05cf2a823304c`

The same hash was confirmed after the committed cleanup.

## Executed transaction and guards

The mutation used `bun:sqlite` `new Database(sourcePath, {readwrite:true, create:false})`. Foreign keys were enabled and read back as 1. The immutable read-only backup URI was attached as `saved`. One `BEGIN IMMEDIATE` transaction was used. No journal-mode changes, migrations, schema operations, or additional writes were issued.

Before DELETE, all 43 tables were compared to the backup and had zero differences; sqlite_schema also had zero differences. The following DELETE was extracted verbatim from the incident report, preserving every predicate:

```sql
PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;
DELETE FROM lanes
WHERE key = 'qa-1'
  AND title = 'QA lane'
  AND created_at = '2026-10-11T06:52:53.346587+00:00'
  AND updated_at = created_at AND last_activity_at = created_at
  AND cwd = '/private/tmp/omon-task8-z4iBTA/qa-repo'
  AND brief_path = '/private/tmp/omon-task8-z4iBTA/brief.md'
  AND lead_session_key = '4:lane|-|4:lead|-|9:lane:lead'
  AND session_key = '4:lane|-|4:qa-1|-|9:lane:qa-1'
  AND model = 'inferhub/cb/deepseek-v4.1-flash'
  AND done_signals = 'file done.txt'
  AND status = 'working' AND turns_run = 0
  AND omo_thread_id IS NULL AND thread_channel_id IS NULL
  AND parent_channel_id IS NULL AND status_message_id IS NULL
  AND claim_at IS NULL AND claim_evidence IS NULL AND objection IS NULL
  AND last_turn_outcome IS NULL AND last_report IS NULL
  AND stale_posted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM lane_history WHERE lane_key = 'qa-1')
  AND NOT EXISTS (SELECT 1 FROM lane_events WHERE lane_key = 'qa-1')
  AND NOT EXISTS (SELECT 1 FROM lane_transcript WHERE lane_key = 'qa-1')
  AND NOT EXISTS (SELECT 1 FROM lane_inbox WHERE lane_key = 'qa-1')
  AND NOT EXISTS (SELECT 1 FROM sessions WHERE session_key IN
    ('4:lane|-|4:qa-1|-|9:lane:qa-1', '4:lane|-|4:lead|-|9:lane:lead'));
SELECT changes() AS deleted_rows;
```

`deleted_rows` was exactly 1. Before COMMIT, every table was compared again to the backup, excluding only `saved.lanes.key='qa-1'`. All 43 comparisons returned zero differences, sqlite_schema remained unchanged, and `PRAGMA main.quick_check` returned `ok`. Only then was `COMMIT` executed. The exception branch would issue ROLLBACK on any failed transaction guard; no guard failed.

## Logical comparisons and post-checks

Table names came from `sqlite_schema WHERE type='table' ORDER BY name`; columns came from `PRAGMA table_info`. Each comparison grouped all columns with `count(*) AS multiplicity`, then used bidirectional EXCEPT and counted differences. This checks values and duplicate multiplicities rather than counts alone. Raw application contents were not printed. The schema comparison used bidirectional EXCEPT across all sqlite_schema columns.

The actual per-table comparison shape was:

```sql
SELECT 'TABLE' AS table_name,
 (SELECT count(*) FROM (
   SELECT ALL_COLUMNS,count(*) AS multiplicity FROM main."TABLE" GROUP BY ALL_COLUMNS
   EXCEPT
   SELECT ALL_COLUMNS,count(*) AS multiplicity FROM saved."TABLE" GROUP BY ALL_COLUMNS
 )) +
 (SELECT count(*) FROM (
   SELECT ALL_COLUMNS,count(*) AS multiplicity FROM saved."TABLE" GROUP BY ALL_COLUMNS
   EXCEPT
   SELECT ALL_COLUMNS,count(*) AS multiplicity FROM main."TABLE" GROUP BY ALL_COLUMNS
 )) AS differences;
```

TABLE and ALL_COLUMNS were generated from the inspected schema with quoted identifiers. For expected post-delete comparisons only, the saved lanes SELECT included `WHERE NOT (key='qa-1')` before GROUP BY. No other table was filtered.

After closing the write connection, a fresh normal read-only Bun connection used query_only and a read transaction, attached the backup, and repeated all comparisons. Results: incident row count 0; 43 expected table comparisons with zero differences; schema differences 0; quick_check `ok`; integrity_check `ok`. The entire `_sqlx_migrations` table, not just checksums 31-33, was unchanged.

Backup counts below remained identical after cleanup except lanes, which changed from 1 to 0:

```text
_sqlx_migrations 32
approval_allowlist 5
bot_profiles 3
cron_cutover_receipt_stores 0
cron_cutover_receipts 0
cron_incidents 5
cron_jobs 15
cron_monitor_states 0
cron_notepads 0
cron_outputs 974
cron_runs 1582
dead_targets 0
delivery_ledger 420
delivery_ledger_constituents 198
delivery_obligations 353
discord_bot_cursors 7
discord_channel_cursors 22
discord_thread_owners 0
judge_calls 0
lane_events 0
lane_history 0
lane_inbox 0
lane_transcript 0
lanes 1 -> 0
legacy_guild_pending_auth 0
legacy_guild_session_conflicts 0
memories 0
message_search_documents 1564
message_search_fts 1564
message_search_fts_config 1
message_search_fts_data 570
message_search_fts_docsize 1564
message_search_fts_idx 568
messages 3898
messenger_policy_overrides 0
paired_users 0
pairing_codes 0
pairing_notifications 0
pairing_platform_lockout 0
pending_writes 0
sessions 63
sqlite_sequence 2
work_items 0
```

Migration checksums confirmed after COMMIT, all success=1:

```text
31 614EB2CE5596C547539AC1E45DFF28718A13131D693C9915B2FB7C5A5C06C97DA3FCAE7D4B46D09A465F1BD0DE5243C3
32 F5188447EA5F72053F0B4D6467E59E91AC5991E5D2EC4BC5AE30AAB27765B4C157428C17C4431234E78D87B75A7700C4
33 09118977E0FEF3F7B07D138BDB32F08A88E0A4110D2425686183A2A5BECF2523993A8391C80E6B7CED84AB41625F62C5
```

## Resource cleanup and scope receipt

Main file inode, size, ownership, and permissions remained unchanged; mtime became `2026-10-11T13:54:21.143Z`. Existing WAL remained zero bytes (mtime `2026-10-11T13:54:21.144Z`), existing SHM remained 32768 bytes (mtime `2026-10-11T13:54:31.494Z`), and no rollback journal existed. These are SQLite transaction/checkpoint/reader sidecar effects, not additional logical changes. sqlite_sequence and FTS shadow-table contents were also compared and unchanged.

All successful SQLite connections and subprocesses were closed. Failed backup open left owned read-only FD 49 in Bun PID 99320; its closure was attempted immediately after verification. The host connection was then lost and reattached, so that cell's completion output was unavailable. A fresh final `/usr/sbin/lsof -nP` covering the source, source WAL/SHM, and backup returned no descriptors, no stderr, exit 1 (no matches). No shared descriptor was intentionally closed. No owned workers, sockets, monitors, or temporary scripts were created. The private backup is deliberately retained.

This receipt was created using apply_patch at the approved evidence path. Product files stayed read-only. No service was stopped, no permissions or configuration were changed, no credentials were output, and no commits were created. Cleanup and checks are complete.

# Task 8 inherited database incident

Investigation date: 2026-10-11 UTC. Read-only investigation; cleanup SQL below was NOT executed. Only this report was edited. Owner work in progress was preserved.

## Findings

1. **Proven unintended target and row:** `/Users/indo/code/project/omon-gateway/omon_gateway.db` contains exactly one lane, `qa-1`, created at `2026-10-11T06:52:53.346587+00:00`. Its identity matches the erroneous CLI output, not the later scratch row. The investigator's individually inspected `DATABASE_URL` is `sqlite:///Users/indo/code/project/omon-gateway/omon_gateway.db`; no complete environment or credentials were printed.
2. **Proven migrations before the reported start:** `_sqlx_migrations` versions 31, 32, and 33 have `installed_on = '2026-10-11 06:51:36'`, `success = 1`. This coincides with executor 8's earlier invalid-signal `qa-2` command, which exited 2 only after `Database::connect`. The successful `qa-1` command at 06:52:53 was not the first unintended database contact. Version 30 was already installed on `2026-10-07 22:55:29`.
3. **No persisted worker activity found:** the stray row remains `working`, `turns_run = 0`; all thread IDs, claim fields, last outcome, and last report are NULL. All rows in `lane_history`, `lane_events`, `lane_transcript`, `lane_inbox`, and `judge_calls` number zero. No `sessions` row matches either its lane or lead session key. This proves absence of persisted activity in the inspected database, not universal absence of an unrecorded external effect.
4. **Do not equate inherited target with active gateway:** process/open-file inspection found no `omo-gateway` process. The only open descriptor found for the repository database belonged to Bun PID 99320, the shared evaluation/RPC host (also holding the now-removed scratch DB). Bun PIDs 16518, 53146, and 93115 held `/Users/indo/.omo/agent/gateway/gateway.sqlite` and its WAL/SHM descriptors. This is evidence of a different runtime gateway resource, not evidence that it uses the Rust lane schema. Read-only access to its current pathname failed, so no row/schema comparison against that resource is claimed.

## Exact invocation provenance

Executor transcript:

`/Users/indo/code/project/omon-gateway/.omo/senpi-task/children/st_01a129a7/sessions/st_01a129a7/2026-10-11T06-29-10.660Z_49645c12-1449-41d4-a277-1f84390a665e.jsonl`

Lines 193-194, request 06:51:35.819Z, result 06:51:36.587Z:

```javascript
const invalidQa8 = await Bun.$`cd /tmp/omon-task8-z4iBTA && DATABASE_URL=sqlite:///tmp/omon-task8-z4iBTA/qa.db /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa-2 --title QA --brief /tmp/omon-task8-z4iBTA/brief.md --done 'command ["true"]' --cwd /tmp/omon-task8-z4iBTA/qa-repo --lead '4:lane|-|4:lead|-|9:lane:lead' --model small`.nothrow().quiet();
```

Result: exit 2, `configuration error: uncheckable command: uncheckable command: ["true"]`. No `qa-2` row exists. The existing `task-8-fail.txt` labels this as scratch QA, but that attribution is contradicted by the migration timing and environment incident; that owner artifact was not edited.

Lines 207-208, request 06:52:52.502Z, result 06:52:53.460Z:

```javascript
const startQa8 = await Bun.$`cd /tmp/omon-task8-z4iBTA && DATABASE_URL=sqlite:///tmp/omon-task8-z4iBTA/qa.db /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa-1 --title 'QA lane' --brief /tmp/omon-task8-z4iBTA/brief.md --done 'file done.txt' --cwd /tmp/omon-task8-z4iBTA/qa-repo --lead '4:lane|-|4:lead|-|9:lane:lead' --model small`.nothrow().quiet();
```

Result: exit 0, empty stderr, `created_at = 2026-10-11T06:52:53.346587Z`, status `working`. No `--thread` was passed. Lines 263-266 also show a repeated shell-prefix attempt returning `lane key already exists` and a lane-list call against the wrong target. Lines 269-273 show explicit `Bun.spawn(..., {env: {...process.env, DATABASE_URL: scratchUrl}})` reading an empty scratch DB and then inserting the distinct scratch row at `06:58:59.428846Z`.

The observed failed override is established by the transcript, conflicting scratch readback, and exact target-row identity. No additional CLI command was executed to reproduce the write.

## Row comparison

| Field | Unintended repository DB | Later scratch receipt (`task-8.json`) |
| --- | --- | --- |
| key/title | `qa-1` / `QA lane` | same |
| created_at | `2026-10-11T06:52:53.346587+00:00` | `2026-10-11T06:58:59.428846+00:00` |
| cwd | `/private/tmp/omon-task8-z4iBTA/qa-repo` | same |
| brief_path | `/private/tmp/omon-task8-z4iBTA/brief.md` | same |
| lead_session_key | `4:lane\|-\|4:lead\|-\|9:lane:lead` | same |
| session_key | `4:lane\|-\|4:qa-1\|-\|9:lane:qa-1` | same |
| status | `working` | `done_claimed` in claim receipt; later closed per verification ledger |
| turns_run | 0 | 1 |
| omo_thread_id | NULL | `01a129c1-f616-7b69-81d1-39fd8efa683e` |
| parent/thread/status-message IDs | all NULL | all NULL |
| model | `inferhub/cb/deepseek-v4.1-flash` | same |
| last_activity_at / updated_at | both equal original created_at | advanced after scratch turn |

## Query method, consistency, and error 14

Ordinary `/usr/bin/sqlite3 -readonly` access reproduced `unable to open database file (14)` for both database paths. The inherited target exists, is UID 501, mode 0644, size 157822976 bytes; parent directory is UID 501, mode 0755. Its WAL and SHM paths both return ENOENT. Runtime gateway directory is mode 0700 and database mode 0600, UID 501; that database is 380928 bytes and its current WAL/SHM paths also return ENOENT despite existing process descriptors. The latter descriptor/path mismatch prevents a reliable active-runtime snapshot through those pathnames. No chmod, sidecar creation, service interruption, or workaround write was attempted.

For the repository target only, read-only URI `file:/Users/indo/code/project/omon-gateway/omon_gateway.db?immutable=1` succeeded. Queries used `PRAGMA query_only=ON; BEGIN; ...; ROLLBACK;`. This avoids attempting to create absent WAL sidecars. No database copy was created. The file's size, inode 2166285453, and mtime `2026-10-11T06:52:53.436Z` were unchanged across observations, no gateway writer was identified, and `PRAGMA quick_check` returned `ok`. This is a read-only inspection of the observed stable, checkpointed file; immutable mode must NOT be generalized to the active runtime DB or a changing WAL database. The evidence supports missing sidecar access as the error-14 mechanism; filesystem permissions alone do not explain a missing main database, because the main file is readable. Exact OS-level open failure was not traced.

Representative executed queries (all read-only):

```sql
SELECT * FROM lanes;
SELECT version, description, installed_on, success, hex(checksum)
FROM _sqlx_migrations WHERE version >= 30;
SELECT count(*) FROM lane_history;
SELECT count(*) FROM lane_events;
SELECT count(*) FROM lane_transcript;
SELECT count(*) FROM lane_inbox;
SELECT count(*) FROM judge_calls;
SELECT session_key, thread_id, channel_id FROM sessions
WHERE session_key IN ('4:lane|-|4:qa-1|-|9:lane:qa-1',
                      '4:lane|-|4:lead|-|9:lane:lead');
```

Migration checksums recorded from `_sqlx_migrations`:

```text
31 614EB2CE5596C547539AC1E45DFF28718A13131D693C9915B2FB7C5A5C06C97DA3FCAE7D4B46D09A465F1BD0DE5243C3
32 F5188447EA5F72053F0B4D6467E59E91AC5991E5D2EC4BC5AE30AAB27765B4C157428C17C4431234E78D87B75A7700C4
33 09118977E0FEF3F7B07D138BDB32F08A88E0A4110D2425686183A2A5BECF2523993A8391C80E6B7CED84AB41625F62C5
```

## Affected resources and limits

Proven additions are the `lanes.qa-1` row and successful migration records 31-33. Those migrations create `lanes`, `lane_history`, `lane_events`, `lane_transcript`, `judge_calls`, `lane_inbox`, and their indexes: `idx_lanes_status`, `idx_lane_events_delivered_lead`, `idx_lane_transcript_lane_id`, `idx_judge_calls_created_at`, `idx_judge_calls_purpose_fallback`, `idx_lane_inbox_pending`. AUTOINCREMENT tables can also introduce `sqlite_sequence` bookkeeping; no claim is made that this incident first created that pre-existing SQLite system table.

`src/storage/db.rs:158-220` runs migrations and legacy guild-session upgrades and creates `discord_thread_owners`, `delivery_ledger_constituents`, and its parent index if missing. Therefore these were additional possible connection-time effects; without a pre-incident snapshot, their creation or legacy-row rewrites cannot be attributed to this incident. No broad rollback is justified.

`src/lanes/cli.rs:336-426` only creates a Discord thread when `--thread` is present. It was absent here and all external channel IDs are NULL, so the observed invocation did not enter that thread-creation branch. It inserts the lane and calls `wake`; it does not itself launch the worker. `src/lanes/runner.rs:85-116` hashes the database path into `/tmp/omon-lanes-90ddeae22659111d9b5ae11c.sock`; missing/refused wake destinations are treated as success. This socket was absent during investigation. A successful CLI exit therefore does not prove worker launch. A future lane runner attached to this target could discover the remaining `working` row; the scratch paths have since been removed according to executor cleanup evidence.

Bounded searches for `qa-1` and `omon-task8-z4iBTA` found no matches in `/Users/indo/.omo/agent/omo-debug.log` or RPC-host stderr logs for `bb5a6ff2534f36a8`, `c12d59ec1b7d76c7`, and `1d71fb2a7bac7888`. These are available host logs, not a complete historical Rust gateway log archive. No Discord UI was inspected. An unrecorded external launch cannot be excluded solely by these negative results. The later scratch thread ID must not be attributed to the stray row or deleted as its cleanup.

## Proposed targeted cleanup -- NOT EXECUTED

Obtain explicit owner authorization and a fresh consistent backup of the confirmed target before any mutation. Recheck process ownership, schema, row identity, and absence of worker activity at that time. Do not invoke `omo-gateway lane close` as an investigative read: CLI connection itself migrates the DB. Preserve migrations 31-33 and their tables for separate owner review; deleting migration records or dropping shared tables could corrupt upgrade expectations or delete subsequent work.

The following proposal only removes the exact untouched stray row and refuses to match if activity or dependent rows have appeared. Run against the confirmed repository database, never the unrelated runtime gateway database. The final statement is deliberately ROLLBACK; an authorized operator must inspect `deleted_rows = 1` and choose COMMIT instead before ending that transaction.

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
ROLLBACK;
```

No dependent-row deletion, external thread deletion, model changes, `.env` edits, service actions, or migration rollback is proposed automatically. If a guard fails, stop and inspect the new evidence rather than broadening the DELETE.

## Investigation cleanup receipt

No owned database snapshots, temporary files, worker processes, sockets, or monitors were created for this investigation. All investigator SQLite subprocesses exited and the attempted Bun read-only connection was closed. Consequently there are no owned copies to remove. Existing shared-host database descriptors and executor resources were not closed or deleted. The sole permitted artifact is this report.

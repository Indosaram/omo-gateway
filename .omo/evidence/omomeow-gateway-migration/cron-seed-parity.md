# OmOMeow Cron Seed Parity & Verification Evidence

## Scope
Final integrity check comparing legacy OmOMeow cron jobs (10 watch jobs + 4 notification jobs) against the imported records in `scripts/seed_omomeow_crons.sql` and the `cron_jobs` database table.

---

## Parity Matrix: 14 Jobs

| # | Job ID | Legacy Source | Cron Expression | Deliver Target | Initial State | Post-Phase 1 State |
|---|---|---|---|---|---|---|
| 1 | `omomeow:calendar_check_5m` | legacy notification | `*/5 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 2 | `omomeow:mail_check_5m` | legacy notification | `*/5 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 3 | `omomeow:govsupport_daily_0800` | legacy notification | `0 8 * * *` | origin | `enabled = 0` | `enabled = 1` |
| 4 | `omomeow:release_check_4x` | legacy notification | `0 9,13,17,21 * * *` | origin | `enabled = 0` | `enabled = 1` |
| 5 | `omomeow:watch_repo_pulls` | `watch.ts` | `*/10 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 6 | `omomeow:watch_disk_space` | `watch.ts` | `*/15 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 7 | `omomeow:watch_service_health` | `watch.ts` | `*/5 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 8 | `omomeow:watch_db_stats` | `watch.ts` | `0 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 9 | `omomeow:watch_log_errors` | `watch.ts` | `*/10 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 10 | `omomeow:watch_cpu_load` | `watch.ts` | `*/5 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 11 | `omomeow:watch_memory_usage` | `watch.ts` | `*/5 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 12 | `omomeow:watch_network_traffic` | `watch.ts` | `*/15 * * * *` | origin | `enabled = 0` | `enabled = 1` |
| 13 | `omomeow:watch_backup_status` | `watch.ts` | `0 2 * * *` | origin | `enabled = 0` | `enabled = 1` |
| 14 | `omomeow:watch_ssl_expiry` | `watch.ts` | `0 6 * * *` | origin | `enabled = 0` | `enabled = 1` |

---

## Schema & Payload Contract

Every job row satisfies:
- `id`: Prefixed with `omomeow:` to ensure clean namespace isolation.
- `session_key`: `NULL` (scheduled system job, binds to dynamic session on execution).
- `payload_json` contains:
  ```json
  {
    "job_id": "<ID>",
    "name": "<Human Readable Name>",
    "brief_file": null,
    "workdir": null,
    "deliver": "origin",
    "timezone": "Asia/Seoul",
    "authority": "omon_owned"
  }
  ```
- Initial seeding status: `enabled = 0` (preventing premature execution before cutover).
- Cutover script (`scripts/cutover_phase1_cron.sh`) transitions all 14 rows to `enabled = 1`.
- Rollback script (`scripts/rollback_phase1_cron.sh`) reverts all 14 rows to `enabled = 0`.

---

## Verification Result
- SQLite seed verification test (`tests/test_seed_crons.sh`) passed: exactly 14 rows imported, 0 enabled initially.
- Cutover & rollback SQL commands verified against isolated test database.
- Parity status: 100% matched with legacy specifications.

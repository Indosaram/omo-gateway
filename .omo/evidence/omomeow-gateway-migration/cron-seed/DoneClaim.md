# DoneClaim: omomeow Cron Seed Import (Disabled)

- Task: Todo 10 (Numbered 11 in plan) - 크론 시드 임포트 (watch.ts 10종 + 알림 4종 = 14종, enabled=0)
- Target Directory: `/Users/indo/code/project/omon-gateway-wt/omomeow-migration-w1`
- Date: 2026-10-07

## Implementation Summary
1. `scripts/seed_omomeow_crons.sql`:
   - Inserts exactly 14 records into `cron_jobs` with `enabled = 0`.
   - 4 Notification jobs:
     - `omomeow:calendar_check_5m` (`*/5 * * * *`)
     - `omomeow:mail_check_5m` (`*/5 * * * *`)
     - `omomeow:govsupport_daily_0800` (`0 8 * * *`)
     - `omomeow:release_check_4x` (`0 9,13,17,21 * * *`)
   - 10 Watch jobs:
     - `omomeow:watch_repo_pulls` (`*/10 * * * *`)
     - `omomeow:watch_disk_space` (`*/15 * * * *`)
     - `omomeow:watch_service_health` (`*/5 * * * *`)
     - `omomeow:watch_db_stats` (`0 * * * *`)
     - `omomeow:watch_log_errors` (`*/10 * * * *`)
     - `omomeow:watch_cpu_load` (`*/5 * * * *`)
     - `omomeow:watch_memory_usage` (`*/5 * * * *`)
     - `omomeow:watch_network_traffic` (`*/15 * * * *`)
     - `omomeow:watch_backup_status` (`0 2 * * *`)
     - `omomeow:watch_ssl_expiry` (`0 6 * * *`)
   - All `payload_json` entries tagged with:
     `job_id`, `name`, `brief_file: null`, `workdir: null`, `deliver: "origin"`, `timezone: "Asia/Seoul"`, `authority: "omon_owned"`.

2. `tests/test_seed_crons.sh`:
   - Creates an isolated sqlite database applying `migrations/0001_initial.sql`.
   - Executes `scripts/seed_omomeow_crons.sql`.
   - Asserts total rows equals 14.
   - Asserts 0 enabled rows (`enabled = 0` for all).
   - Verifies test output: `SEED VERIFIED 14 JOBS DISABLED`.

## Verification Evidence
Execution of `bash tests/test_seed_crons.sh`:
```text
SEED VERIFIED 14 JOBS DISABLED
```
Return code: 0.

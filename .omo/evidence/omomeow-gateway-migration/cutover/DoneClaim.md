# DoneClaim: Phase 1~4 Cutover and Rollback Automation with God-Session Retirement

- Tasks: Todo 12, 13, 14, 15, 16 (Wave 4 in OmOMeow Migration Plan)
- Target Directory: `/Users/indo/code/project/omon-gateway-wt/omomeow-migration-w1`
- Date: 2026-10-07

## Deliverables Summary

1. **Phase 1 Cron Cutover & Rollback**:
   - `scripts/cutover_phase1_cron.sh`: Unloads legacy launchd plist (`com.omomeow.cron.plist`), disables crontab entries, and activates 14 OmOMeow jobs (`enabled = 1`) in SQLite `cron_jobs` table.
   - `scripts/rollback_phase1_cron.sh`: Reverts 14 OmOMeow jobs (`enabled = 0`) in SQLite and reloads legacy launchd/crontab.

2. **Phase 2 Shadow Bot Verification**:
   - `scripts/verify_phase2_shadow.sh`: End-to-end verification runner validating:
     1. Environment and token configurations
     2. Gateway WebSocket API reachability
     3. Message dispatch contracts
     4. Discord interaction button handling (Approve, Reject, non-terminal Preview) & LRU deduplication
     5. Discord thread lifecycles (Create, Update/Archive, Delete)
     6. SLA work heartbeat runner (15m progress checks, dead-worker detection)

3. **Phase 3 Inbound Cutover & Rollback**:
   - `scripts/cutover_phase3_inbound.sh`: Shuts down legacy OmOMeow daemon (`com.omomeow`), extracts production bot token, applies to `.env`, and restarts `omon-gateway`.
   - `scripts/rollback_phase3_inbound.sh`: Unsets OmOMeow bot token in gateway `.env`, restarts `omon-gateway`, and restores legacy launchd daemon.

4. **Phase 4 God-Session Retirement**:
   - `docs/PHASE4_GOD_SESSION_RETIREMENT.md`: Comprehensive retirement architecture guide detailing drain procedures, process termination, multi-session isolation verification, and deprecation steps for `god-session.ts`.

5. **Cron Seed Parity & Integrity**:
   - `.omo/evidence/omomeow-gateway-migration/cron-seed-parity.md`: Parity matrix comparing 14 jobs (10 watch + 4 notification), verifying payload contracts and execution states.

## Verification & Dry Run
- Made all scripts executable (`chmod +x`).
- Executed dry-run tests for all cutover and rollback scripts:
  - `scripts/cutover_phase1_cron.sh --dry-run` -> Exit 0
  - `scripts/rollback_phase1_cron.sh --dry-run` -> Exit 0
  - `scripts/verify_phase2_shadow.sh --dry-run` -> Exit 0 (6/6 checks passed)
  - `scripts/cutover_phase3_inbound.sh --dry-run` -> Exit 0
  - `scripts/rollback_phase3_inbound.sh --dry-run` -> Exit 0
- Live mock tests against temporary SQLite database verified SQL transitions `enabled: 0 -> 1 -> 0`.

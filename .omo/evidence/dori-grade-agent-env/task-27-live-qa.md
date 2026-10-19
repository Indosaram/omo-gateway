# Task27 live gateway QA

Status: LIVE QA PASS on the pre-existing omo-ai 5.1.30 / senpi 2026.10.11 installation. Candidate activation and candidate production acceptance NOT established: preflight baseline mismatch prevented installation (task27-omoai-install-receipt.md).

Read main plan .omo/plans/dori-grade-agent-env.md:456-464 and task27-live-qa-preflight.md. Used that existing owned runner / real SSH shim pattern with the user-required released binary hash, not the older preflight binary identity. The happy and unreachable scenarios each ran once. The existing acceptance assertion was changed to retain a failed result and continue the second scenario instead of skipping it. No product source/test/build, .env, owner database, host seed, remote installation/key or shared process change occurred.

## Invocation

Binary: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway.
SHA-256: 71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86 (measured before execution, verified inside runner).
Owned root: /private/tmp/omon-task27-qa.7NWyFr, mode 700. Helpers created with apply_patch: run.mjs, ssh, direct.mjs.

```sh
cd /private/tmp/omon-task27-qa.7NWyFr && /usr/bin/env -i HOME=/private/tmp PATH=/usr/bin:/bin:/usr/sbin:/sbin TMPDIR=/private/tmp/omon-task27-qa.7NWyFr /Users/indo/.bun/bin/bun --no-env-file /private/tmp/omon-task27-qa.7NWyFr/run.mjs
```

Each gateway child argv: ["/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway", "sessions", "list", "--json"].
Each gateway child env is restricted to HOME=/Users/indo, PATH=/private/tmp/omon-task27-qa.7NWyFr:/Users/indo/.bun/bin:/usr/bin:/bin:/usr/sbin:/sbin, TMPDIR=/private/tmp/omon-task27-qa.7NWyFr, DATABASE_URL=sqlite:///private/tmp/omon-task27-qa.7NWyFr/forbidden.db and OMON_SESSION_HOSTS. Failure scenario additionally sets TASK27_STALL_PORT to the dynamically allocated owned loopback listener port. No provider tokens/model/Discord variables or SSH_AUTH_SOCK copied. Both cwd values are the owned root, avoiding repository dotenv discovery.

SSH uses separate argv: -o BatchMode=yes -o ConnectTimeout=5 <machine> omo thread list --all-scope --json. Owned shim execs /usr/bin/ssh with ClearAllForwardings=yes, StrictHostKeyChecking=yes, UpdateHostKeys=no, ControlMaster=no, ControlPath=none; loopback additionally uses -F /dev/null and -p <owned port>. No shell string remote command or interactive aliases. Real omarchy remote authentication; loopback accepts without an SSH banner, without contacting another host.

## Measured receipts

| Scenario | OMON_SESSION_HOSTS | Exit | Duration ms | Local rows | omarchy rows | Errors | Passed |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Happy | omarchy | 0 | 10948.827709 | 23 | 3 | none | true |
| Unreachable | omarchy,127.0.0.1 | 0 | 5725.626332999998 | 23 | 4 | 127.0.0.1 code 3: omo thread list failed with exit code 255 | true |

Both stderr byte counts 0. Runner/monitor mon_RRFMASB7XQQ1GA2N exit 0. Happy retained both machines within 20 s; failure returned within 25 s, retaining both successful machines and isolating the unavailable host. Counts vary because real remote sessions can change between calls. Sessions were checked for exact public schema; raw session IDs/controls were never persisted. Projected name/cwd data appears only in the monitor receipt. No messages were read or sent.

Cleanup receipt: remaining_owned_children=0, scratch_db_files=[], listener_closed=true. Owned gateway children exited normally; no shared daemon or session was stopped/restarted/killed. No registry entries were deleted. This verifies the plan's SSH handshake-failure isolation, not an exact 20 s cancellation deadline or candidate runtime behavior.

Backup/rollback paths: /Users/indo/.bun/task27-backup.cB97lM/global; /Users/indo/.bun/task27-backup.cB97lM/rollback.sh. Scratch helpers retained for reproducibility; no DB, WAL, SHM or journal created. On owner follow-up, the old failing task-27.json was renamed intact to task-27-historical-failure.json and a new passing task-27.json was written from these captured receipts, with session fields restricted to name and cwd grouped by machine. No QA invocation was repeated.


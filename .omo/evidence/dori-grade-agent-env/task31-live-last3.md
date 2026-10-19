# Todo 31 live Manual-QA last3: PASS

Exactly one invocation, from the evidence directory:

```sh
bun qa-last-run.mjs 31 /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json
```

Harness and monitor exit code: 0. Scenario budget: 1,500 seconds, unchanged from the previous round; lane observation 360 seconds and settlement observation 900 seconds. Owned app-server elapsed time including cleanup: 526.717 seconds. No second QA invocation or automatic second cron re-run occurred.

Release binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`.

SHA-256 independently verified before and after execution, also matched by the harness final receipt: `f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323`.

Scratch root: `/private/tmp/omon-task31-live-final-9I5ARS`. Authoritative database: `qa.db` under that root, independently opened read-only after shutdown.

## Machine verdict

- `lanes`: 1 row, `cron-fix-qa31-scratch-d7c2`, status `verified_done`. Its cwd is the scratch `workspace/cron-fixture`, not `/usr/bin`.
- The lane's retained brief contains the original traceback and `NameError: name 'total' is not defined`. The script changed from an undefined `total` to `total = 7`; the resulting file was inspected.
- `lane_history`: 3 rows: `working -> done_claimed`, `settlement_started`, and `done_claimed -> verified_done` with reason `cron natural clean success`.
- `cron_runs`: exactly 2 rows: initial failure `09ace715-450f-4dc4-926f-24abc5542d44` (attempt 1), then successful settlement re-run `39d7d4f3-008f-4013-8dfe-17a53cdf2814` (attempt 2). No third row after gateway shutdown.
- `cron_problems`: exactly 1 row, status `fixed`, detail `reruns=1`, with evidence linking the successful re-run. Problem ID: 1.
- `cron_outputs`: exactly 1 row, linked to the successful re-run, output `count=7\n`, matching the configured regex `count=7`.
- `cron_remediation_outcomes`: exactly 1 row, recording the verified native re-run success. `delivered_at=null`; live Discord is excluded from this scenario and was not contacted.

The remediation lane fixed the script, settlement performed exactly one verified cron re-run, output matched expectations, and the problem was resolved. This scenario exercises the explicit job cwd; the fallback for command-line absolute arguments was not separately exercised.

## Isolation and cleanup receipts

- Gateway PID 88758 exited 0; PID absent and lane socket removed, as recorded by the harness.
- Owned app-server PID 88667 used `ws://127.0.0.1:50207`, private 0600 authentication, and scratch HOME. Cleanup escalated to SIGKILL after the graceful shutdown deadline; receipt records `childExit.code=null`, `childExit.signal=SIGKILL`, no remaining process-group members, PID absent, exclusive port rebind success, and private auth scratch removal.
- Independent process inspection found two escaped RPC groups belonging to this QA: supervisor/worker PIDs 31258/31314 and 90682/90730. Each worker's socket path was under this exact scratch root. Group membership was inspected before sending SIGKILL only to groups 31258 and 90682. A subsequent `ps` exited 0 and found no processes with this scratch root or either group, and no members of the owned app-server group.
- Scratch DB, script, brief and logs are retained for evidence. Shared daemon, owner DB, env files, production gateway and live Discord were not used or modified. TYPESAFE values were loaded in memory by the existing helper; `valuesRecorded=false`, and persisted harness evidence uses its redaction helper.
- No runner or gateway source changes, rebuild, or second live QA were performed.

Raw evidence under the scratch root: `task31-live-final.json`, `single-verified-rerun.json`, `final-machine-state.json`, `initial-cron-failure.json`, `remediation-lane.json`, `descriptor.json`, `task-31-live-final.log`, and `workspace/.omo/cron-fix/briefs/cron-fix-qa31-scratch-d7c2.md`. The supplemental escaped-RPC inspection and cleanup receipts are recorded above; the original harness receipt remains unchanged.

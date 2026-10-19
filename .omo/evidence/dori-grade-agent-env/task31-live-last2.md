# Todo 31 live Manual-QA last2: FAIL

Exactly one invocation: `bun qa-last-run.mjs 31 /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json`. No retry. Scenario deadline 1,500 seconds; remediation-lane observation 360 seconds.

Release SHA-256 before execution and in the final receipt: `929dfcca73d9c23baaeb0a26569ab7813989bbd975a0bdb4801ba0e1c702ec15`.

Scratch root: `/private/tmp/omon-task31-live-final-7BiFHg`. Authoritative DB: `qa.db` in that root, opened read-only for verdict inspection.

## Machine verdict

- `cron_runs`: 1 row, failed; 0 succeeded rows. Initial script error: `NameError: name 'total' is not defined`.
- `cron_problems`: 1 row, status `needs_owner`, `lane_key=null`, detail `reruns=0`.
- `lanes`: 0 rows. No remediation brief was created or verified.
- `cron_remediation_outcomes`: 1 row, `delivered_at=null`, reporting `Remediation failed during diagnosis: configuration error: lane I/O: No such file or directory (os error 2)`.
- `cron_outputs`: 0 rows. Verified re-runs: 0, required: exactly 1.

The missing-gateway.log fix did not yield the requested flow in this run. The observed failure is now lane I/O during diagnosis; its underlying cause was not investigated or repaired within this QA-only scope. The harness ended with `scratch cron remediation lane deadline; final authoritative query failed` and exit 1.

## Isolation and cleanup receipts

Gateway PID 41639 exited 0, is absent, and its lane socket was removed. Owned app-server PID 41558 exited 0, its process group is empty, PID absent, port 62996 exclusively rebound, and private auth scratch removed. Scratch DB and logs retained. TYPESAFE values were loaded in memory only and redacted by the existing helper. Shared daemon, owner DB, env files, production gateway and live Discord were not used or modified.

Raw receipts: `task31-live-final.json`, `final-machine-state.json`, `initial-cron-failure.json`, `task-31-live-final.log`, and `descriptor.json` under the scratch root above.

After both executions, independent SHA verification again matched the expected release identity. Final process inspection found no processes containing either task24 or task31 scratch root; task24 escaped RPC cleanup is documented in `task24-live-last2.md`. No additional task31 execution was performed.

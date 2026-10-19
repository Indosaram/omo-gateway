# Todo 24 live Manual-QA last2: FAIL

Exactly one invocation: `bun qa-last-run.mjs 24 /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json`. No retry. Used qa-last-run.mjs, qa-last-scenarios.mjs, qa-final-common.mjs and qa-owned-appserver.mjs. The first collection observation was increased from 300 to 900 seconds and required a completed inbox_write usage row plus stored owner/bot/fyi/ignore decisions. Overall scenario ceiling remained 1,200 seconds. No product source, build or product tests changed or run.

Release SHA-256 independently verified before and after both executions, and also matched both final receipts: `929dfcca73d9c23baaeb0a26569ab7813989bbd975a0bdb4801ba0e1c702ec15`.

Scratch root: `/private/tmp/omon-task24-live-final-Y19q5d`. Authoritative DB: `qa.db` in that root, inspected read-only after shutdown.

## Machine verdict

- `inbox_items`: 2,396 rows.
- `inbox_decisions`: 77 rows: owner 38, bot 1, fyi 31, ignore 7. Unclassified items: 2,319.
- `model_usage WHERE purpose='inbox_write'`: 0 rows; no completed writing turn verified.
- `inbox_cursors`: 4 rows. Mail retains `tool execution error: mail thread has invalid date`; kakao, imessage and discord have no cursor error.
- Logs at 04:48:49 and 04:58:22 UTC explicitly report `turn exceeded total deadline of 180s; turn/interrupt sent`.
- Two batches settled with `classified=20 stored=20`, writing_items 16 and 15. This confirms batch size 20 in observable execution, but does not satisfy completed writing.

The required writing completion failed despite owner/bot decisions being persisted. Full triage did not complete during the 900-second window. The unchanged-input second collect was not reached, so no-turn-on-second-collect is unverified, not PASS. Exit 1; no rerun or fabricated input.

## Isolation and cleanup receipts

Gateway PIDs 41636 and 42391 exited 0, are absent, and their lane socket was removed. Owned app-server PID 41538 required SIGKILL after its 15-second graceful shutdown bound; its process group is empty, PID absent, port 62995 exclusively rebound, and private auth scratch removed.

Post-receipt inspection found four escaped detached RPC groups. Each group contained exactly its supervisor and one RPC host listening at this scratch root's `senpi-rpc-host-internal-*/host.sock`. Groups 31792, 61772, 70571 and 85006 were SIGKILLed only after that same-PGID ownership proof. Final `ps -axww -o pid=,pgid=,stat=,command=` inspection found no members of those groups and no processes containing either this scratch root or task31's root. DBs and logs retained.

TYPESAFE values remained in memory and were redacted by the helper. Live collection used read operations; the existing mail wrapper permits only mail list/watch with owner-HOME writes denied except its existing ~/.zele allowance. Shared daemon, owner DB, env files, production gateway and live Discord were not used or modified.

Raw evidence: `task24-live-final.json`, `final-machine-state.json`, `collector-initial.json`, `source-preflight.json`, `descriptor.json`, and `task-24-live-final.log` under the scratch root. `collector-first-counts.json` and `collector-second-counts.json` were not produced because first observation failed.

Harness diagnostics: LSP initialization could not run because no valid TypeScript installation is available. No dependency was installed. The helper itself executed through Bun and reached the live scenario.

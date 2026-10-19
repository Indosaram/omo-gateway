# LAST live Manual-QA: 0924e30d

Completed only tasks 15 clean, 24, 25 digest, 26, 28, 29, 31, 32 concurrently. Then reran only 28/29 concurrently with the explicitly authorized readonly OMO wrapper. Ten private roots; no dirty-refusal rerun. Overall requested-scenario verdict: FAIL. Failures are observations, not all proven product defects.

| Task | Verdict | Machine evidence and limitations |
| --- | --- | --- |
| 15 clean close/archive/reopen | FAIL | Real done_diff fulfills=0.190 clean=0.750 risk=0.000; done_claimed -> not_done -> working. Close/archive/reopen not reached. Judge appropriateness NEEDS-REVIEW. |
| 24 read-only collection + triage | FAIL | 2395 items, 25 stored decisions (18 fyi, 7 ignore), no completed inbox_write usage row. Mail cursor error: tool execution error: mail thread has invalid date. Full triage not reached within 300 s; stored decisions explicitly verified. |
| 25 native digest | FAIL | cron_jobs for omon:inbox-digest=[], nativeDigestRegistered=false, delivery outputs=0. No substitute digest. |
| 26 proposal -> approval -> lane | NEEDS-REVIEW | Machine local-approval flow PASS: numbered proposal 1 pending, lanes=0 before approval, exactly one linked lane and completed real turn after approval. Bot classification fixture and authenticated Discord numbered ingress not exercised. |
| 28 local list/read, authorized wrapper | FAIL | List PASS, exit 0 and errors=[]. Read exit 1 scope_denied for first local session outside scratch workspace. Initial host_unavailable was a harness issue and is superseded. No remote mutation or added scope flag. |
| 29 owner status within 30 s, authorized wrapper | FAIL | Exact 30 s window expired again: one user message, zero assistant messages, zero delivery obligations. Original scratch-HOME failure superseded as harness issue; no new root cause asserted. |
| 31 failing cron remediation + one rerun | FAIL | One native failed script run; diagnosis configuration error: No such file or directory (os error 2); problem needs_owner, lane_key=null, reruns=0. |
| 32 cron health | NEEDS-REVIEW | Machine health PASS: ok_jobs=1, successful job KST timestamp, never-run status=null. One owner reply delivered, but actual reply reports running /usr/sbin/cron instead of summarizing gateway health. Raw reply retained for human judgement. |

## Identity and cleanup

Release: final-release.json; binary /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway. SHA-256 before and after: 0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00 (match). Every receipt also matches. All 14 gateway lifetimes exited numeric 0; all PIDs absent and lane/cron sockets removed. All ten owned app-servers have PID/group absence and exclusive port rebind evidence. Ten escaped detached RPC groups were killed only after scratch host.sock and same-PGID ownership proof; final snapshot has no group members or processes containing any of the ten scratch roots. Full proof: last-qa-integrity.json. DBs and logs retained.

## Invocation and harness adjustments

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env
# Concurrent, exactly once per task:
bun qa-last-run.mjs <15|24|25|26|28|29|31|32> /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json
# After first round finishes, authorized concurrent reruns only:
bun qa-last-readonly-run.mjs <28|29> /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json
```

qa-last-scenarios.mjs preserves qa-final-scenarios.mjs except clean-only task 15 and removal of the stale demand that writing be finished before accepting persisted triage. qa-last-run.mjs uses qa-final-extra.mjs for 28/29/31/32. All use qa-final-common.mjs and qa-owned-appserver.mjs. Original helpers were not modified. The exact wrapper is recorded in rerun receipts: only thread:list/thread:read execute /Users/indo/.bun/bin/omo with HOME=/Users/indo; everything else exits 64; all remaining gateway environment remains scratch. Wrapper installed after private owned app-server readiness, so no shared app-server launch occurs.

## Deadlines and raw evidence

Scenario ceilings: 15/24 1200000 ms; 25/28/29/32 600000 ms; 26 900000 ms; 31 1500000 ms. Every receipt records deadlineAt before scenario work. Windows: startup 45000 ms, commands 60000 ms, 15 settlement 660000 ms (native five-minute settlement), 24 first triage 300000 ms, 26 approved lane 300000 ms, 29 reply 30000 ms, 31 remediation lane 360000 ms, 32 reply 240000 ms. Gateway cleanup 45000 ms; owned app-server cleanup 15000 ms then 5000 ms forced bound.

Individual documents: task15-live-last.md, task24-live-last.md, task25-live-last.md, task26-live-last.md, task28-live-last.md, task29-live-last.md, task31-live-last.md, task32-live-last.md. Each contains raw receipt, paths and applicable machine state. Superseded 28/29 receipts retained in their documents.

No tasks 16/17/18/20/35 rerun. No shared daemon, owner DB, main-checkout source, live profile, env file or production gateway modified; no live Discord API called. Mail helper preserves existing ~/.zele sandbox allowance for read-only mail commands. TYPESAFE values loaded only in memory and redacted in evidence. Source/product code unchanged, no build or product tests run. LSP could not initialize because a valid TypeScript installation is unavailable; no dependency installed.

## Retained roots

- Task 15: /private/tmp/omon-task15-live-final-ssEIbH
- Task 24: /private/tmp/omon-task24-live-final-ObzuN4
- Task 25: /private/tmp/omon-task25-live-final-xIB5GK
- Task 26: /private/tmp/omon-task26-live-final-3Cq4dM
- Task 28: /private/tmp/omon-task28-live-final-JXeIXg
- Task 29: /private/tmp/omon-task29-live-final-1lkxge
- Task 31: /private/tmp/omon-task31-live-final-oI0ps1
- Task 32: /private/tmp/omon-task32-live-final-RiFI5E
- Task 28: /private/tmp/omon-task28-live-final-PfIgXH (superseded initial)
- Task 29: /private/tmp/omon-task29-live-final-sqx30O (superseded initial)


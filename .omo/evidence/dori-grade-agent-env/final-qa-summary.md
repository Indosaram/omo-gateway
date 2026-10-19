# Final live Manual-QA: 4196efab observations

One concurrent round completed: 12 tasks, 14 private scenario roots. Overall acceptance for this binary: FAIL. Per owner update, fixes for 24/25/26/30/31/35 are underway elsewhere and final verdicts belong to a later binary. This receipt does not certify that later binary. Task 30 was not requested or executed in this round. No new scenario or rerun was started after the owner update.

| Task / scenario | Verdict on 4196efab | Evidence / limitation |
| --- | --- | --- |
| 15 clean close/archive/reopen | FAIL | done_diff fulfills=0.270 clean=0.890 risk=0.000 -> not_done; close/reopen not reached |
| 15 dirty refusal | PASS | >300s settlement -> uncommitted objection; close exit 2; worktree retained |
| 16 inbox insertion | PASS | generation=consumed=1; exactly one kibitzer_wake judge, 0.330; native <0.7 no-wake, zero model turns |
| 16 idle | PASS (prior) | Explicitly not rerun |
| 18 two milestones / deleted replacement | PASS | Both variants passed; exactly one send per milestone, replacement after 404, zero mentions |
| 20 persona | NEEDS-REVIEW | Delivered two real replies, profile CLI passed, lanes=0; delegation/question semantics raw |
| 24 collection + triage | FAIL | 2387 items, 0 decisions; mail cursor: tool execution error: mail thread has invalid date |
| 25 digest | NEEDS-REVIEW | Native omon:inbox-digest registration 0; native delivery unreachable, no substitute |
| 25 list | PASS (prior) | Explicitly not rerun |
| 26 approval | NEEDS-REVIEW | Local approval linked and real lane turn completed (PASS); authenticated numbered ingress untested; not a verified_done claim |
| 28 local list/read | FAIL | list exit 3, timed out after 20 seconds; read not reached; remote mutation excluded |
| 29 session status in 30s | FAIL | Only user row, zero delivered replies at deadline |
| 31 scratch cron remediation | FAIL | run-now exit 2: configuration error: No such file or directory (os error 2); runs/problems/lanes 0 |
| 32 cron health | FAIL | same native socket absence prevents execution fixture; read-only health exit 0, ok_jobs 0 |
| 35 unrelated diff | PASS | >300s -> not_done, done_diff fulfills=0.120 clean=0.670 risk=0.000, one real done_diff judge |
| Authentication / human semantics | NEEDS-REVIEW | Live Discord ingress excluded; raw evidence retained |

## Release identity and cleanup

Binary: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/omo-gateway

SHA-256 before and after: 4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52 (match). All 14 receipts match this hash. All 17 gateway lifetimes exited numeric 0, PID absent and lane socket removed. All 14 owned app-servers proved PID/group absence and port rebind. Escaped detached RPC groups required ownership-scoped cleanup: 14 groups, proof from each scratch host.sock and same-PGID membership; post-cleanup process snapshot has no scratch processes or those groups. Full evidence: final-qa-integrity.json. Scratch SQLite DBs retained.

## Evidence and interpretation

All task<N>-live-final.md receipts exist. Raw helper JSON/logs, SQLite and final-machine-state.json remain at their receipt paths. 16 raw helper status is failed because its stale predicate demanded a positive model decision event; actual code src/lanes/kibitzer.rs:261 returns without a model for scores below 0.7. The authoritative one admitted generation and exactly one real judge decision satisfy the requested insertion criterion; no raw result overwritten, no rerun. 15 clean fails the requested lifecycle but its jev appropriateness requires human inspection; the runner did not force a result. 32 read-only health CLI evidence is task32-health-readonly-final.json.

Shared daemon, owner DB, production gateway, live bot profiles and env files were not modified; live Discord API was not called. Existing profile files were read only to seed scratch DB. Mail wrapper retained the approved ~/.zele sandbox write allowance, source collection read only. TYPESAFE values loaded only in memory and redacted. LSP unavailable: no valid TypeScript installation; no package installed. No build or product-source change occurred in this QA round.

## Roots

- Task 25 task25-scenarios: /private/tmp/omon-task25-live-final-ykHscd
- Task 32 task32-final: /private/tmp/omon-task32-live-final-YhA5QX
- Task 31 task31-final: /private/tmp/omon-task31-live-final-LYUKOY
- Task 28 task28-final: /private/tmp/omon-task28-live-final-ARv0vh
- Task 29 task29-final: /private/tmp/omon-task29-live-final-DsolPm
- Task 18 two-milestones: /private/tmp/omon-task18-live-final-TgOc23
- Task 18 deleted-status-replaced: /private/tmp/omon-task18-live-final-LJxZ9H
- Task 16 inbox-insert-real-kibitzer: /private/tmp/omon-task16-live-final-AwdiyQ
- Task 24 task24-scenarios: /private/tmp/omon-task24-live-final-AzgSH2
- Task 26 task26-scenarios: /private/tmp/omon-task26-live-final-M4b0HH
- Task 20 task20-scenarios: /private/tmp/omon-task20-live-final-yqT9LF
- Task 35 real-unrelated-diff-rejected: /private/tmp/omon-task35-live-final-G6TR8O
- Task 15 close-archive-reply-unarchive: /private/tmp/omon-task15-live-final-7fqCI7
- Task 15 dirty-worktree-refused: /private/tmp/omon-task15-live-final-xw9vbz

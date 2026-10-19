# Wave 4 live Manual-QA summary

Overall: NOT ALL PASS. All six requested todo attempts completed; blockers and limited proofs are explicit below.

Binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway`

SHA256 before and after: `71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86` (matches requested released binary). No binary rebuilt or replaced. Release metadata wave4-strict-gate2-release.json.

| Todo | Scenario | Verdict | Machine evidence / limit |
|---|---|---|---|
|16|One inbox item -> one parsed kibitzer turn|FAIL|Scratch inbox count1; no decisions, kibitzer_state table or kibitzer_inbox_insert trigger; enabled headless runtime does not start service.|
|16|10 minutes idle -> zero turns|LIMITED PASS|600009 ms, zero decisions/judges/log calls; vacuous because service absent.|
|17|Disk threshold100000 queues, default restart works|PASS|CLI exit4, qa-3 queued; working history at23:03:17.075021, within milliseconds of restarted readiness. Model completion unverified; shutdown cancelled backend turn.|
|17|Concurrent heavy sleep3 serialize|PASS|Both CLI exit0; start separation3936 ms (initial) and3303 ms (persistent confirmation).|
|18|Two milestones -> two message IDs|BLOCKED|Existing loopback HTTP test transport cannot be injected into released CLI. No live Discord API used.|
|18|Deleted status -> stored replacement|BLOCKED|Same transport blocker; no synthetic pass.|
|20|Apply persona block on scratch bot_profiles|PASS WITH LIMIT|CLI exit0, stable repeated initial apply sha5656bd57e8a9f85fc9133c5754ed4ee6bcc627ef7e23153f3f73decdaec5909c. Concurrent block change means not frozen release-input proof; final prompt3966 chars.|
|20|Korean dashboard prompt -> lane start tool|BLOCKED|Existing authenticated app-server unreachable at127.0.0.1:19742; no new daemon/model override.|
|20|Synthetic question -> send/recommendation|BLOCKED|Same unavailable backend.|
|24|Read-only first collection and triage counts|FAIL|Mail/imsg read-only probes exit0, Kakao GET200; no headless collector runtime or released collect CLI. Scratch fallback cannot invoke real collector; counts unverified.|
|24|Duplicate collection -> no new turn|BLOCKED|First real collection unavailable; zero turns is not dedup proof.|
|30|Failed script -> same-second problem/event|PASS|failed run completed23:06:20.466569; matched problem23:06:20.467342 and event23:06:20.468565; exactly1 each by run_id.|
|30|Successful script -> no new problem|PASS|succeeded run, output {"count":1}, no run-matched problem; preexisting missed problem marked fixed.|
|30|Successful script -> no model call|LIMITED|Empty-prompt script path, no success-job model log. Earlier missed events caused unrelated lead connection attempts, so global zero-model assertion not proven.|

## Isolation and cleanup

Each todo owns a unique scratch root, SQLite DB, and lane socket. Descriptor lsof checks and PRAGMA database_list were performed before triggers. DB/WAL watchers were subscribed before actions with initial authoritative queries; the sole ten-minute timer measured the idle behavior itself. Scratch roots and DBs retained for audit.

- Todo 16: `/private/tmp/omon-wave4-task16-PM73Ue`, DB `qa.db`; owned socket `/tmp/omon-lanes-f0cfec1d8179f1f7127fa646.sock` removed.
- Todo 17: `/private/tmp/omon-wave4-task17-qFthYh`, DB `qa.db`; owned socket `/tmp/omon-lanes-3b943bb75a6a7c9c9869c94b.sock` removed.
- Todo 18: `/private/tmp/omon-wave4-task18-9s7z9y`, DB `qa.db`; owned socket `/tmp/omon-lanes-778742ec8712cf19b4dc544a.sock` removed.
- Todo 20: `/private/tmp/omon-wave4-task20-mh8Ez9`, DB `qa.db`; owned socket `/tmp/omon-lanes-21c9dc624f183ef43622d023.sock` removed.
- Todo 24: `/private/tmp/omon-wave4-task24-3dgqEt`, DB `qa.db`; owned socket `/tmp/omon-lanes-7d5f7f51bea0622432f97acf.sock` removed.
- Todo 30: `/private/tmp/omon-wave4-task30-BmUGd6`, DB `qa.db`; owned socket `/tmp/omon-lanes-d0f2b46aa51b3444955c5ff2.sock` removed.

Persistent gateways PIDs96068,96166,96128,96084,96153,96187 exited0; cron standalone pair961/1031 exited0 (SIGTERM/SIGINT respectively). All absent on final direct PID checks. Initial eval-owned gateway processes exited0 when eval lifetime ended; that was harness lifetime cleanup, not product failure. Initial CLI judge prerequisite timeout was exit137 with no lane inserted; corrected environment yielded exit4. No secret tokens logged.

Todo30 used two owned processes of the SAME binary: headless lane socket plus standalone dashboard scheduler, sharing only todo30 scratch DB. Loopback scratch trigger API HTTP200 for both jobs; output persisted in scratch DB; no Discord destination configured. Initial stale schedule yielded missed problems and unrelated lead attempts. Broad first harness assertions misgraded earlier events; corrected verdict matches exact run_id and preserves both in task-30.log.

Owner DB, main checkout, production gateway, live profiles and env files were not modified. No daemon was started; no model override; no live Discord API. Read-only sources limited to mail list, imsg chats, PirateTalk rooms GET; source content omitted from logs. Product source was not edited.

## Provenance limits and artifacts

Current entry.rs/main.rs/discord_threads.rs match release source manifest. runner.rs and persona block changed concurrently relative to manifest; source observations about those files are not asserted as frozen release code. Runtime binary stayed pinned. JS harness LSP unavailable (TypeScript not installed); execution parsed and completed exit0. No product build/test suite was requested or run.

Per-todo raw artifacts: task-16.log, task-17.log, task-18.log, task-20.log, task-24.log, task-30.log. Receipts: task16-live-qa.md, task17-live-qa.md, task18-live-qa.md, task20-live-qa.md, task24-live-qa.md, task30-live-qa.md. Final hash/cleanup ledger: wave4-live-qa-final.json. Helpers: wave4-live-qa-runner.mjs and task30-live-qa-runner.mjs. Each scratch root retains descriptor.json and actual QA SQLite state.


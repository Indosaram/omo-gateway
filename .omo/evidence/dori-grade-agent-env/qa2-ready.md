# Manual-QA round 2 preparation

Preparation only. No gateway binary, source collector, model, app-server, Discord API, git command, Cargo command or service was executed during preparation. The candidate in `wave4b-strict-gate-release.json` remains held with `release_allowed: false`. No live verdict is claimed.

## Helpers and future invocation

- `qa2-common.mjs`: release identity checks, fresh per-scenario scratch root, explicit gateway environment, DB/WAL subscriptions, descriptor proof, loopback REST recorder, bounded owned process cleanup and receipts.
- `qa2-scenarios.mjs`: todos 15, 16, 18, 20, 24, 25, 26 and 35. Each variant calls `runQA` separately; failures and blockers produce numeric exit codes.
- `qa2-predicates.test.mjs`: binary-free release refusal/hash checks, initial-authority and event observer checks, usage/post/close/diff predicates. Fixture bytes are not executable.
- `qa-owned-appserver.mjs`: reused ephemeral app-server owner. Optional tool environment now passes scratch DATABASE_URL, PATH and runtime roots into tools, while keeping normal HOME for existing OMO credentials. It never uses the shared listener at 19742. It stops only its own process group and proves listener disappearance by exclusive rebind.
- `task13-live-qa.mjs`, `task14-live-qa.mjs`, `lane-local-preflight.mjs`: read for settlement, durable output and descriptor patterns, not invoked. Their old shared-daemon and env-file assumptions are not copied into QA2.

After an independently released build exists, the execution owner may run:

```text
bun qa2-scenarios.mjs execute <15|16|18|20|24|25|26|35> /absolute/release.json
```

The release path is mandatory. Before any scratch directory, listener, app-server or child creation, release authority must be `status: released` or `release_allowed: true`, with an explicit false hold always taking precedence. Both legacy `binary` + `binarySha256` and structured `binary.path` + `binary.sha256` receipts are supported. Actual on-disk bytes must match SHA-256; a supplied manifest must match too. Checks repeat before child commands/start and at final receipt. Do not modify the held JSON to bypass the gate. Exit 0 means all exercised predicates passed; 1 means failure; 3 means incomplete/blocked coverage. Invalid arguments or refused release exit nonzero before execution.

## Isolation and environment

Each scenario uses a fresh `/private/tmp/omon-task<N>-qa2-*` root, `qa.db`, private home/bin/workspace and DB-derived lane socket. Gateway env sets `DATABASE_URL`, `OMON_RUNTIME_LOCK_DIR`, `OMON_WORKSPACE_ROOT`, `OMON_TOOL_ROOTS`, `TMPDIR`, explicit PATH, `OMON_DISCORD_ENABLED=false`, `OMON_LANES_ENABLED=true`, `OMON_INBOX_ENABLED=false`, `OMON_KIBITZER_ENABLED=false`, `OMON_OMO_AUTOSPAWN=off`, `DASHBOARD_ENABLED=false`, `OMON_LANE_DEFAULT_LEAD` and empty lane keywords. Todo 16 enables only kibitzer; todo 24 enables inbox only after initial descriptor proof. Todos 15/18 set `OMON_DISCORD_API_BASE` to the owned loopback recorder; no live Discord credential is passed to the gateway. Dashboard is opt-in loopback with a fresh reserved port.

Prerequisites for future execution: Bun, node, lsof, git for owned temporary fixture repos only, real `typesafe-judge` (`QA2_JUDGE` optional absolute path), existing normal OMO HOME/auth, and private `TYPESAFE_API_KEY/URL/MODEL` in the launching environment. Existing model env values are preserved, never overridden with new models. Todo 24 additionally needs read-only zele/imsg permissions and reachable PirateTalk (`OMON_PIRATETALK_URL`, `OMON_INBOX_KAKAO_ROOMS` preserved). Source content is not included in collector count receipts.

Root and WAL watchers are registered before scenario triggers; each predicate immediately queries authoritative DB state. On startup/restart, WAL mode, PRAGMA database_list, gateway lsof descriptor, socket and initial lane/message counts are established before proceeding. No fixed sleeps or model polling. The sole ten-minute idle timer measures time itself; the five-minute settlement predicate checks actual claim/history timestamps without changing deadlines. All other timers are bounded failure deadlines. Cleanup records numeric gateway exit, PID absence and socket removal. Scratch QA DBs/logs remain retained; the app-server's temporary secret token directory is removed by its existing owner.

## Scenarios and expected artifacts

Every root: `descriptor.json`, `task-<N>-qa2.log`, `task<N>-qa2.json`, `qa.db` and retained WAL/SHM when SQLite retains them. Receipt includes commands/numeric exits, cleanup, release hash, status and scratch path. Todos 15/18 also retain `http-recorder.json` with local REST acknowledgements. Model scenarios include owned app-server cleanup in the task receipt.

| Todo | Scenarios and machine proof | Additional artifact names / limits |
| --- | --- | --- |
| 15 | Real lane signal and five-minute verification; close, archive, owned worktree removed, branch retained; durable reply reopens once, unarchives and resumes recorded real thread. Separate dirty-worktree case rejects claim/close and retains files. | `settlement.json`, `dirty-retained.json`, `reopen-receipt.json`. Durable receipt injection is not authenticated Discord ingress; adapter routing remains explicitly blocked. No branch deletion or force removal. |
| 16 | Enabled trigger/schema required; one scratch inbox insertion yields one real kibitzer usage row and strict parsed decision; wake=true then requires delivered lead reply. Separate ten-minute idle case compares authoritative state and zero usage. | `kibitzer-initial.json`, `kibitzer-result.json`, `kibitzer-lead-turn.json`, `idle-ten-minutes.json`. Outside KST 08:00-24:00 or wake=false is blocked coverage, not synthetic PASS. No mail/calendar/GitHub or direct owner message. |
| 18 | Two actual milestone turns yield exactly two acknowledged posts and status PATCH; separate local deletion causes recorded 404 and changed durable status ID. | `milestone-result.json`, `http-recorder.json`. Recorder refuses mention-enabled posts. Release must include the runner sink integration noted as unapplied in `qa-gaps-source-settled.md`. |
| 20 | Scratch profile export/apply twice with hash idempotency and unchanged name/model/toolsets; missing profile exit 2; loopback Korean chat must yield a real lane-start tool record; question chat retains delivered response. | `profile-before.json`, `persona-lane-start.json`, `persona-question.json`. Recommendation semantics require QA-by-read; receipt stays blocked until separately reviewed, never a prose-pinning test. |
| 24 | Read-only source preflight and real enabled headless startup collection; nonzero items, all classified, no source errors, exactly one inbox writing turn; second startup must advance source cursors without new items/classes/turns. | `source-preflight.json`, `collector-initial.json`, `collector-first-counts.json`, `collector-second-counts.json`. New live arrivals invalidate unchanged-input proof; all-FYI/ignore cannot prove the requested writing turn. No source writes or full-body logging. |
| 25 | Owned seeded owner/bot items; filtered inbox list and unknown-item set exit 2. Native digest only if actual `omon:inbox-digest` exists; loopback trigger must correlate native run/output with delivered local obligation and accepted local-output log. | `inbox-list-counts.json`, `digest-delivery.json` or `digest-unreachable.json`. Missing native headless scheduler remains blocked, never replaced by a script/model digest. No shared-channel post. |
| 26 | Seed real proposal; unapproved lead turn must create no lane; real inbox approval CLI followed by persona-driven lead chat must create an owned real lane and persist item lane key. | `unapproved-initial.json`, `unapproved-real-turn.json`, `approved-lane.json`. Explicit CLI/lead path is not authenticated numbered Discord reply; that adapter trust boundary remains blocked, without fabricated approval CAS rows. No external sending/reply/reaction. |
| 35 | Real lane completion/token notifications, usage CLI, five-minute settlement and close; separate intentionally unrelated README diff gets not_done with numeric done_diff history/judge proof. | `real-token-rows.json`, `usage-totals.json`, `done-gate-result.json`. No estimated token counts, fabricated usage or unavailable-judge rejection claimed. |

Round-1 provenance: `wave4-live-qa-summary.md`, `task16-live-qa.md`, `task18-live-qa.md`, `task20-live-qa.md`, `task24-live-qa.md`. No matching round-1 task15/task25/task26/task35 receipt exists in this directory. The missing headless kibitzer/collector and unavailable backend caused the previous failures; QA2 prepares the owned backend and opt-in source/sink paths without claiming that the held binary fixes them.

## Local verification

Single local run completed with exit code 0: `node --check` passed for `qa2-common.mjs`, `qa2-scenarios.mjs`, `qa2-predicates.test.mjs`, `qa-owned-appserver.mjs`, `lane-local-preflight.mjs`, `task13-live-qa.mjs` and `task14-live-qa.mjs`. `bun test ./qa2-predicates.test.mjs` returned 4 pass, 0 fail. Monitor receipt: `mon_0G4R4355Y3KRWW9S`. Tests verified actual held-release refusal before binary reads, fixture-byte SHA checks, initial/event authority and machine predicates; no binary was executed. JS LSP cannot initialize because this workspace lacks a TypeScript installation; no dependency was installed. Syntax and predicate execution provide local validation, not product runtime validation.

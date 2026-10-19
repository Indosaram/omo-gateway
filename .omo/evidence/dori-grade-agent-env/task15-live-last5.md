# Task 15 LAST5 live QA: new done_diff release, clean lane only

Verdict: **FAIL** for the requested verified_done/close/archive/reply/reopen lifecycle. The new release was used without building. The real done_diff judge rejected the initial done.txt claim with fulfills=0.41 clean=0.95 risk=0.00. Automatic lane work resumed and claimed again near the existing settlement observer deadline. No verified_done transition occurred before the 660000 ms observer expired; close/archive/reopen was not reached. Exactly one clean scenario ran for this request, with no rerun, product edit, helper edit, or assertion weakening.

## Exact invocation

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
bun -e 'const common = await import("./qa-final-common.mjs"); common.scenarioDeadlines[15] = 1500000; process.argv = ["bun", "./qa-last-run.mjs", "15", "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json"]; await import("./qa-last-run.mjs");'
```

Same invocation and in-memory deadline override as LAST4. Conditional double-commit/claim fixes were present. Overall deadlineMs 1500000, deadlineAt `2026-10-12T06:25:03.668Z`; monitor timeout 1800000 ms. Existing per-settlement observer limit remained 660000 ms. The scenario ended on that observer's timeout, not its overall deadline:

```text
Error: qa15-clean real five-minute settlement deadline; final authoritative query failed
at qa-final-common.mjs:77:71
```

Scratch retained: `/private/tmp/omon-task15-live-final-xQ7Pqy`; DB: `qa.db` under that root.

## Release binary SHA before and after

final-release.json scope at start: `FINAL incl. done_diff real-path judge input (15) and compact session persona rule`.

Binary path:

`/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4903aad46ca3bbcd87b09c21573c98c1711bf96ddcde841fd2ca88a7f3925cb7/omo-gateway`

Release expected SHA, harness before SHA, harness final SHA, and independent after shasum all equal:

```text
4903aad46ca3bbcd87b09c21573c98c1711bf96ddcde841fd2ca88a7f3925cb7
```

No cargo/build command was executed. Concurrent todo 29 source work was not touched.

## DB transitions and judge numbers

Read-only SQLite queries agree with final-machine-state.json. Final lane status: done_claimed; done_signals `file done.txt`; turns_run 2; last_turn_outcome `error: multiplexer error: agent turn cancelled` during cleanup.

| Time UTC | Transition | Reason |
| --- | --- | --- |
| 06:00:32.254735 | working -> done_claimed | done.txt contains QA15_READY, commit 7a254b5, clean tree |
| 06:00:32.254735 | done_claimed -> done_claimed | settlement_started |
| 06:05:36.464498 | done_claimed -> done_claimed | done_diff fulfills=0.410 clean=0.950 risk=0.000 |
| 06:05:36.464498 | done_claimed -> not_done | same judge result |
| 06:05:37.236237 | not_done -> working | resuming work |
| 06:11:37.576315 | working -> done_claimed | done.txt expanded with fulfilled-brief prose, commit 3200474 |

The first settlement lasted 304210 ms, exceeding the required 300000 ms.

Real judge_calls done_diff row:

```text
id: 36a4dae3-a28d-4d57-b11f-033c4317bcbb
purpose: done_diff
kind: diff
ms: 597
outcome: fulfills=0.41 clean=0.95 risk=0.00
used_fallback: 0
created_at: 2026-10-12T06:05:37.216862+00:00
```

Only the initial done_diff result is persisted; no second done_diff result. The other judge row is lane_size/small. This QA observed rejection on the new binary; it did not capture the full judge input or explanatory rationale, so it does not infer why the score remains low.

No verified_done, closed, or reopened history row. lane_owner_replies count 0. No artificial status/deadline writes or forced claims were performed by this session. The second claim is the lane's automatic resumed work within the same scenario, not a second run.

## Git and registered worktree

Registration returned exit 0 and `{"key":"qa15-clean","registered":true}`. Harness `git status --porcelain -- done.txt` returned empty output, so no redundant commit ran. The harness skipped its claim because the lane had already claimed.

Final full git status is clean. Current branch history:

```text
3200474ebf8512230d7cc808c5c032e324e4c099 Add QA15_READY done marker for lane qa15-clean
done.txt
d2eb0fd8d95b4c4b23ad26112d64037ae353ed44 QA fixture baseline
README.md
```

Only done.txt differs from baseline in the current lane commit. It starts with QA15_READY and now contains lane/branch/worktree identifiers and brief-fulfillment prose. DB preserves the initial 7a254b5 claim, while current branch history contains 3200474 directly above baseline; no cause for that history difference is inferred here. This session made no modifying git call.

Git worktree list retains base on main and `/private/tmp/omon-task15-live-final-xQ7Pqy/workspace/lane-worktree` on refs/heads/qa-owned, HEAD 3200474ebf8512230d7cc808c5c032e324e4c099. No close/removal command was reached.

## Loopback recorder and lifecycle criteria

http-recorder.json contains 13 requests, including thread creation, initial/status posts, paused name after rejection and working name after resume. No request body contains archived. Thread ID: 800000000000000002.

| Required evidence | Result |
| --- | --- |
| verified_done | Absent; initial done_diff rejected |
| close removes registered worktree without --force | Not reached; worktree retained |
| close keeps branch | Close not reached; qa-owned currently exists |
| thread archived | No archive recorder request |
| owner reply reopens same branch | Not reached; owner reply rows 0 |
| thread unarchived | No unarchive recorder request |

settlement.json and reopen-receipt.json are absent (ENOENT), consistent with the timeout before their save points. Requested lifecycle remains unverified by LAST5.

## Isolation and cleanup

Unchanged scratch HOME/workspace/DB/runtime-lock harness, owned app-server, Discord gateway disabled and loopback recorder were used. TYPESAFE configuration stayed in process memory; receipt records only key names, valuesRecorded false. No env file, owner DB, shared daemon, production gateway or live Discord operation was invoked by this QA run.

Owned app-server: ws://127.0.0.1:59813, PID 1251, private 0600 token. Started `2026-10-12T06:00:03.749Z`, finished `2026-10-12T06:11:59.127Z`, total 715383 ms. Cleanup: remainingGroupPids [], pidAbsent true, portRebindSucceeded true, privateScratchRemoved true. Shutdown required SIGKILL after the owned-group grace period. Scratch gateway PID 1383 exited 0, pidAbsent true, socketRemoved true.

Retained artifacts: task15-live-final.json, final-machine-state.json, http-recorder.json, descriptor.json, qa15-clean-brief.md, task-15-live-final.log, scratch DB and git repository.

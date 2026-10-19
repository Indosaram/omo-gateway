# Task 15 LAST4 live QA: clean lane only

Verdict: **FAIL** for the requested lifecycle. Conditional commit and conditional claim passed. This run reached the real five-minute settlement and real done_diff judge, which returned fulfills=0.42 clean=0.82 risk=0.00 and transitioned the lane to not_done. Automatic work resumed and the lane claimed again near the settlement observer deadline. No verified_done transition occurred before the existing 660000 ms settlement observer expired. Close/archive/owner reply/reopen was never reached.

Exactly one clean scenario ran for this request. No helper line or product source was changed. The failure is a persisted judge rejection followed by settlement timeout, not an established single-line harness assumption error. At result delivery the gateway and owned app-server were already stopped, so continuation within the same live run was no longer valid. No new run was started and no assertion was weakened.

## Invocation and timing

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
bun -e 'const common = await import("./qa-final-common.mjs"); common.scenarioDeadlines[15] = 1500000; process.argv = ["bun", "./qa-last-run.mjs", "15", "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json"]; await import("./qa-last-run.mjs");'
```

Same runner and memory-only deadline override as prior runs. Only closeScenario(release, false) executed. Overall deadlineMs 1500000; deadlineAt `2026-10-12T05:50:11.013Z`; monitor timeout 1800000 ms. Existing settlement observer limit remained 660000 ms. Receipt error:

```text
Error: qa15-clean real five-minute settlement deadline; final authoritative query failed
at qa-final-common.mjs:77:71
```

Parent-reported syntax checks were not rerun or claimed as new checks.

Scratch: `/private/tmp/omon-task15-live-final-4qbaDS`; DB: `qa.db` in that root.

## Binary identity

Current final-release.json binary:

`/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`

Harness before SHA, harness final SHA, and independent after shasum agree:

```text
f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323
```

## Authoritative DB evidence

Read-only SQLite queries agree with final-machine-state.json. Lane qa15-clean final status is done_claimed; done_signals remains `file done.txt`; turns_run is 2. Last outcome records `error: multiplexer error: agent turn cancelled` during shutdown.

| Time (UTC) | Transition | Reason |
| --- | --- | --- |
| 05:25:43.973782 | working -> done_claimed | done.txt contains QA15_READY, commit 8f0c157 |
| 05:25:43.973782 | done_claimed -> done_claimed | settlement_started |
| 05:31:11.666807 | done_claimed -> done_claimed | done_diff fulfills=0.420 clean=0.820 risk=0.000 |
| 05:31:11.666807 | done_claimed -> not_done | same done_diff result |
| 05:31:12.422589 | not_done -> working | resuming work |
| 05:36:45.890105 | working -> done_claimed | done.txt contains QA15_READY plus lane deliverables, commit 59aa127 |

First settlement lasted 327693 ms, exceeding the required 300000 ms. Real judge_calls row: purpose done_diff, kind diff, ms 629, outcome `fulfills=0.42 clean=0.82 risk=0.00`, used_fallback 0, created_at `2026-10-12T05:31:12.414372+00:00`. Other judge row is lane_size/small. No second done_diff evaluation is persisted.

No verified_done, closed, or reopened transition. lane_owner_replies count is 0. No forced claim, synthetic status transition, or deadline modification was performed by this session. The second claim is the lane's automatic resumed work, not a second harness invocation.

## Git evidence

At inspection the worktree is clean and its current lane commit is:

```text
59aa12709d049d93aa40e1dafdab03fd695e3369 Add QA15_READY done marker
done.txt
707d8ea4fc3f5f74b56dfee201dd6b5cb6d5f142 QA fixture baseline
README.md
```

Current git log contains the final lane commit directly above the baseline, while DB preserves the earlier 8f0c157 claim. The earlier claimed commit is no longer in current branch history; no cause is inferred here. This session issued no modifying git commands.

done.txt currently starts with QA15_READY, followed by lane/branch identifiers and prose describing fulfillment of the brief. Only done.txt differs from the baseline in the current commit.

Both base and lane-worktree remain in git worktree list. Lane path: `/private/tmp/omon-task15-live-final-4qbaDS/workspace/lane-worktree`, branch refs/heads/qa-owned, HEAD 59aa12709d049d93aa40e1dafdab03fd695e3369. Registration was part of the run before settlement; no close or removal command was reached.

## Loopback recorder and requested criteria

http-recorder.json records thread creation and status/name updates, including paused state after rejection and working state after resume. There are 13 requests; none contains an archived field. Thread ID: 800000000000000002.

| Required behavior | Evidence/result |
| --- | --- |
| verified_done | Absent; real done_diff rejected first claim |
| close removes registered worktree without --force | Not reached; worktree remains |
| close keeps branch | Not reached; qa-owned currently exists |
| thread archived | No archive recorder request |
| owner reply reopens same branch | Not reached; zero owner reply rows |
| thread unarchived | No unarchive recorder request |

settlement.json and reopen-receipt.json are absent (ENOENT), consistent with the observer timeout before either save point. The requested lifecycle remains unverified.

## Isolation and cleanup

Existing scratch HOME/workspace/DB/runtime lock isolation, owned app-server and loopback recorder were retained unchanged. Discord gateway was disabled. TYPESAFE configuration was in process memory only; receipt records key names and valuesRecorded false. No env file, owner DB, shared daemon, production gateway or live Discord operation was invoked by this QA run.

Owned app-server: ws://127.0.0.1:53008, PID 29674, private 0600 token. Started `2026-10-12T05:25:11.051Z`; finished `2026-10-12T05:37:15.057Z`; total 724006 ms. Cleanup: remainingGroupPids [], pidAbsent true, portRebindSucceeded true, privateScratchRemoved true. Shutdown required SIGKILL after the owned-group grace period. Scratch gateway PID 29767 exited 0, pidAbsent true, socketRemoved true.

Retained artifacts: task15-live-final.json, final-machine-state.json, http-recorder.json, descriptor.json, qa15-clean-brief.md, task-15-live-final.log, scratch DB and git repository. A literal search of the gateway log for done_diff/settlement/fulfills found no matching log lines; judge and transition evidence above comes from the persisted DB, not an invented log explanation.

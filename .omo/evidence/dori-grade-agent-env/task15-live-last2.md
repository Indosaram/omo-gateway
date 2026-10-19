# Task 15 LAST2 live QA: clean lane only

Verdict: **FAIL** for the requested close/archive/reply/reopen lifecycle. The lane obeyed the corrected brief, committed exactly done.txt, and reported done. The unchanged harness then attempted a second commit, which returned exit 1 with `nothing to commit, working tree clean`. The command-exit assertion stopped the scenario before settlement, close, archive, owner reply, or reopen. No assertion was weakened, no product source was edited, and no rerun was performed.

## Change and syntax checks

Only the clean branch of the Task 15 brief in `qa-last-scenarios.mjs:53` changed. The dirty brief and done signal `file done.txt` are unchanged. New clean instructions:

> Create done.txt containing QA15_READY on your lane branch. Commit exactly done.txt with a short message. Write no other files. Then emit QA15_READY and report done. Do not remove worktrees or delete branches.

`node --check qa-last-scenarios.mjs` and `node --check qa-last-run.mjs` both passed (combined exit 0). Automatic LSP initialization could not run because the workspace has no valid TypeScript installation; no dependency or configuration was changed.

## Exact single invocation

Executed in `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`:

```sh
bun -e 'const common = await import("./qa-final-common.mjs"); common.scenarioDeadlines[15] = 1500000; process.argv = ["bun", "./qa-last-run.mjs", "15", "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json"]; await import("./qa-last-run.mjs");'
```

This imports the same runner once and selects only `closeScenario(release, false)`. The existing 1200000 ms harness deadline was overridden to 1500000 ms in memory only; monitor timeout was 1800000 ms. No helper deadline or assertion was edited. The run ended early due to the command assertion, not a timeout.

Scratch retained: `/private/tmp/omon-task15-live-final-VxrD87`.

## Binary identity

Release: `final-release.json` (the current final release, not the older release used by task15-live-last.md).

Binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`.

Before SHA, receipt final SHA, and independent after SHA all equal:

```text
f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323
```

## DB and git evidence

Read-only queries against scratch `qa.db` confirm:

- `lanes`: qa15-clean, status `done_claimed`, done_signals `file done.txt`, last_turn_outcome `completed`.
- `lane_history`: one transition, `working -> done_claimed` at `2026-10-12T05:19:02.126279+00:00`. No `verified_done`, `closed`, or `reopened` transition.
- `lane_events`: one delivered `done_claimed` event describing done.txt committed on qa-owned.
- `judge_calls`: only `lane_size` with outcome `small`; no `done_diff` evaluation was reached.
- `lane_owner_replies`: count 0.

Git independently confirms clean porcelain status and exactly one added file in the lane commit:

```text
0257ed35a1868ef52b00638864a7b567984bf345 Add QA15 done marker
done.txt
```

The file contains `QA15_READY` followed by a newline. Baseline commit is `4ef37136ad5cde45a25d9e11162ed0670c43d096` with README.md. Both base and lane worktrees remain registered; lane worktree HEAD is the above commit on `refs/heads/qa-owned`. The registration command returned exit 0 and `{"key":"qa15-clean","registered":true}`.

The existing harness command at `qa-last-scenarios.mjs:57` failed:

```text
git -C /private/tmp/omon-task15-live-final-VxrD87/workspace/lane-worktree -c user.name=Owned QA -c user.email=qa@example.invalid commit -m "QA owned completion signal"
exit 1
On branch qa-owned
nothing to commit, working tree clean
```

The assertion in `qa-final-common.mjs:154` correctly required exit 0. This duplicate harness commit is incompatible with asking the lane to commit first. It was left unchanged under the brief-only scope. A future authorized harness change must remove the redundant clean-scenario commit step before another run can reach lifecycle verification.

An exploratory read-only query for a guessed `lane_worktrees` table returned `no such table: lane_worktrees`; registration evidence above comes from the successful command receipt and actual git worktree listing, not that query.

## Recorder and pass criteria

`http-recorder.json` contains seven loopback requests: identity lookup, initial message, thread creation, and status/name updates. No request has an `archived` field. Thread ID is `800000000000000002`.

| Required evidence | Result |
| --- | --- |
| verified_done DB transition | Not reached |
| close removes registered worktree without --force | Not reached; worktree retained |
| branch retained after close | Close not reached; qa-owned currently exists |
| loopback archive | Not reached |
| owner reply reopens same branch | Not reached; zero owner reply rows |
| loopback unarchive | Not reached |

## Isolation and cleanup

The existing harness used scratch HOME, workspace, DB and runtime lock root, Discord gateway disabled, and a loopback REST recorder. TYPESAFE values were loaded into process memory from owner ~/.zshenv; values were not recorded and env files were not modified. No shared daemon, owner DB, production gateway, or live Discord operation was invoked by this QA run.

Owned app-server: `ws://127.0.0.1:51920`, PID 49884, private 0600 token. Receipt shows remainingGroupPids `[]`, pidAbsent true, portRebindSucceeded true, privateScratchRemoved true. Shutdown required SIGKILL after the owned-group grace period. Scratch gateway PID 49980 exited 0, pidAbsent true, socketRemoved true. App-server receipt spans `2026-10-12T05:18:43.927Z` to `2026-10-12T05:19:34.696Z` (50775 ms).

Raw retained artifacts: `task15-live-final.json`, `final-machine-state.json`, `http-recorder.json`, `descriptor.json`, `qa15-clean-brief.md`, and `task-15-live-final.log` under the scratch root. Lifecycle behavior remains unverified by this run.

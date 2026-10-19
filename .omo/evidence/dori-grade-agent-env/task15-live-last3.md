# Task 15 LAST3 live QA: clean lane only

Verdict: **FAIL** for the requested lifecycle. The corrected conditional commit worked: done.txt was already committed by the lane, so the harness skipped its commit. The lane had also claimed done. The harness's unconditional `lane claim-done` then returned exit 2: `configuration error: illegal lane transition: done_claimed -> done_claimed`. The unchanged exit-code assertion stopped the run before settlement, close, archive, reply, and reopen. Exactly one clean scenario was executed for this request; no additional helper or product change and no rerun was performed.

## Invocation and identity

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
bun -e 'const common = await import("./qa-final-common.mjs"); common.scenarioDeadlines[15] = 1500000; process.argv = ["bun", "./qa-last-run.mjs", "15", "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json"]; await import("./qa-last-run.mjs");'
```

This is the same runner and in-memory deadline override as LAST2. Only `closeScenario(release, false)` runs. Receipt deadlineMs is 1500000; monitor timeout was 1800000 ms. The assertion ended the run early; this was not a timeout. Parent-reported syntax checks were not repeated or claimed as a new check.

Scratch retained: `/private/tmp/omon-task15-live-final-6qEcMz`; DB: `qa.db` under that root.

Binary from current `final-release.json`:

`/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323/omo-gateway`

Harness before SHA, harness final SHA, and independent after `shasum -a 256` all match:

```text
f892eebb47faed669cffc8775937e238e63630f98f6d2795116ed888a377c323
```

## DB and git evidence

Read-only SQLite queries and `final-machine-state.json` agree:

- Lane qa15-clean: status `done_claimed`, done signal `file done.txt`, last turn outcome `completed`.
- One lane_history transition: `working -> done_claimed` at `2026-10-12T05:22:32.865660+00:00`, with evidence identifying commit 1315665 and a clean tree.
- One delivered lane_events entry of kind `done_claimed`.
- One judge_calls row, purpose `lane_size`, outcome `small`. No done_diff row and no verified_done transition.
- Zero lane_owner_replies rows.

Lane commit:

```text
1315665a755e0b5378e17b71eaff52d9e1051e6a Add QA15 done marker
done.txt
```

Exactly done.txt changed relative to baseline `eb33b02e34c9dadadf31f3b0b78619d5c9ede6df` (README.md baseline). done.txt contains `QA15_READY` and a newline. Full worktree porcelain status is empty.

`lane register-worktree` returned exit 0 and `{"key":"qa15-clean","registered":true}`. Git worktree list still contains base on main and lane-worktree on `refs/heads/qa-owned`, HEAD 1315665a755e0b5378e17b71eaff52d9e1051e6a. No removal or reopen was reached.

## Exact failure

Conditional `git status --porcelain -- done.txt` returned exit 0 and empty output, correctly avoiding the redundant commit.

Next command, from `qa-last-scenarios.mjs:65`:

```text
omo-gateway lane claim-done qa15-clean --evidence "QA15 actual file signal"
exit 2
{"error":"configuration error: illegal lane transition: done_claimed -> done_claimed","ok":false}
```

`qa-final-common.mjs:154` required exit 0 and rejected exit 2. The lane's earlier accepted claim is independently persisted in lane_history, lane_events and lanes; the failed command did not replace it. The scenario immediately cleaned up its gateway, so its five-minute settlement observer did not reach a result. `settlement.json` and `reopen-receipt.json` are absent (ENOENT), consistent with this control-flow stop. This run does not evaluate the done_diff gate.

The remaining harness mismatch is unconditional claiming after a lane may already claim. It was not changed under the instruction to change nothing else.

## Loopback recorder and lifecycle criteria

`http-recorder.json` has seven requests: identity lookup, initial message, thread creation, name/status updates. Thread ID: `800000000000000002`. No recorded request body contains `archived`.

| Required evidence | Result |
| --- | --- |
| verified_done DB transition | Not reached |
| close removes registered worktree without --force | Not reached; registered worktree remains |
| close keeps branch | Close not reached; qa-owned currently exists |
| loopback thread archived | Not reached |
| owner reply reopens on same branch | Not reached; owner reply count 0 |
| loopback thread unarchived | Not reached |

## Isolation and cleanup

Existing scratch-root harness and owned app-server were used unchanged, with Discord gateway disabled and loopback recorder enabled. TYPESAFE configuration was loaded in memory from ~/.zshenv; receipt records key names only, valuesRecorded false. No env file, owner DB, shared daemon, production gateway, or live Discord operation was invoked by this QA run.

Owned app-server: `ws://127.0.0.1:52396`, PID 89421, private 0600 token. Cleanup receipt: remainingGroupPids [], pidAbsent true, portRebindSucceeded true, privateScratchRemoved true. Owned-group shutdown required SIGKILL after its grace period. Scratch gateway PID 89506 exited 0, pidAbsent true, socketRemoved true. App-server started `2026-10-12T05:21:56.338Z`, finished `2026-10-12T05:23:05.656Z`, total 69320 ms.

Retained artifacts under scratch: `task15-live-final.json`, `final-machine-state.json`, `http-recorder.json`, `descriptor.json`, `qa15-clean-brief.md`, `task-15-live-final.log`, scratch DB and git repository. Requested lifecycle remains unverified by LAST3.

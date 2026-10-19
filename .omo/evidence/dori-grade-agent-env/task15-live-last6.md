# Task 15 LAST6 live QA: pinned deliverable-only done_diff binary

Verdict: **PASS for the requested isolated lifecycle**. Real done_diff returned fulfills=0.93 clean=0.97 risk=0.00; lane reached verified_done; non-forced close removed the registered worktree and retained qa-owned; loopback thread was archived; the durable owner reply reopened the lane once, restored the worktree on the same branch and same commit, reused the same OMO thread and unarchived the loopback thread. The reopen turn completed with QA15_REOPEN_REPLY.

Raw harness status is blocked/exit 3 solely because its existing final block() marks authenticated Discord inbound routing untested. This is not an assertion failure. The user-requested DB/git/loopback lifecycle passed all existing assertions. Live Discord authentication is outside this isolated test and remains unverified. Final lane status is blocked because the fixture explicitly asks the reopened turn for a blocker report; the persisted reopened transition proves reopening succeeded before that report.

## Single invocation and setup

Exactly one clean scenario ran; no rerun, cargo/build command or product edit. Stale final-release.json was not read. Added task15-last6-release.json with the exact owner-pinned path/SHA for the unchanged runner contract.

Changed only the qa15-clean settlement observer cap in qa-last-scenarios.mjs from 660000 to 1500000 ms; all other scenarios retain their old cap. No assertion or gate threshold changed. node --check qa-last-scenarios.mjs passed, exit 0. Automatic LSP initialization was unavailable because no valid TypeScript installation was found; no installation/configuration change was made.

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env`.

```sh
bun -e 'const common = await import("./qa-final-common.mjs"); common.scenarioDeadlines[15] = 2100000; process.argv = ["bun", "./qa-last-run.mjs", "15", "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task15-last6-release.json"]; await import("./qa-last-run.mjs");'
```

Settlement cap: 1500000 ms. Overall deadline: 2100000 ms in memory only, allowing setup/close/reply/reopen beyond settlement. Monitor deadline: 2400000 ms. Scratch retained: `/private/tmp/omon-task15-live-final-JjM31e`; DB: `qa.db` in that root.

## Binary identity

Binary:

`/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/d5d48e4d79d2a8a6239e4cb24ccdefa257537473d4052578b7d301b78a4d9b9b/omo-gateway`

Independent before shasum, expected manifest SHA, harness before/final SHA, and independent after shasum all equal:

```text
d5d48e4d79d2a8a6239e4cb24ccdefa257537473d4052578b7d301b78a4d9b9b
```

## Done_diff and authoritative DB lifecycle

Read-only SQLite queries agree with settlement.json and reopen-receipt.json.

| Time UTC | Transition | Reason |
| --- | --- | --- |
| 07:15:22.726017 | working -> done_claimed | done.txt contains QA15_READY, commit f288173, exactly done.txt, clean worktree |
| 07:15:22.726017 | done_claimed -> done_claimed | settlement_started |
| 07:20:44.061030 | done_claimed -> done_claimed | done_diff fulfills=0.930 clean=0.970 risk=0.000 |
| 07:20:44.061030 | done_claimed -> verified_done | verified completion |
| 07:20:46.654147 | verified_done -> closed | successful close |
| 07:21:14.376491 | closed -> working | reopened |
| 07:21:24.965562 | working -> blocked | requested blocker report from reopen fixture |

Settlement elapsed 321335 ms, exceeding the required 300000 ms. Judge_calls: purpose done_diff, outcome fulfills=0.93 clean=0.97 risk=0.00, used_fallback 0. Threshold was not changed. No synthetic status/deadline writes or forced claims were performed by this session.

lane_close_intents id 1: original_status verified_done, force 0, state completed, phase 2, attempt 1, error null.

lane_owned_worktrees retains removal_receipt with verified=true, branch qa-owned, intent 1 and the original worktree identity. After reopen it is registered again with a new identity and restoration_receipt identifying branch qa-owned and the prior removal receipt.

Owner receipt qa15-owned-reply: state applied, generation 0, reopen_history_id 6, error null. Exactly one reopened history row. Same OMO thread before and after: `01a12ef6-5071-703a-9fe9-3006c754a8ae`. Reopen observer required QA15_REOPEN_REPLY plus a subsequent completed outcome; it passed and saved reopen-receipt.json.

## Non-forced removal, branch retention and restoration

lane close command returned exit 0. The run's immediate post-close git worktree list contains only the base:

```text
worktree /private/tmp/omon-task15-live-final-JjM31e/workspace/base
HEAD 9751670ca00447dddd16e1868c7bbeaacd766a2d
branch refs/heads/main
```

Post-close branch --list qa-owned returned qa-owned. Existing closedProof assertion also required the lane path to be absent. Removal receipt verified=true and close intent force=0 corroborate non-forced close. Read-only source inspection of src/lanes/runner.rs:420 shows `git(&tree.repo, &["worktree", "remove", &tree.path]).await?;` without --force; this is supporting source evidence, not a claim of captured child-process argv. No source was changed or rebuilt.

After owner reply, independent git worktree list and branch --show-current confirm restoration at the same lane path on qa-owned with unchanged HEAD:

```text
f2881730ca2c5e704b0791d38e447059c0175cc6 QA15: add done.txt marker
done.txt
9751670ca00447dddd16e1868c7bbeaacd766a2d QA fixture baseline
README.md
```

Only done.txt changed in the lane commit. Its contents are exactly QA15_READY followed by a newline. Restored worktree porcelain status is empty.

## Loopback archive and unarchive

Same thread path `/api/v10/channels/800000000000000002`:

| Time UTC | Request | Status |
| --- | --- | --- |
| 07:20:46.394 | PATCH {"archived":true} | 200 |
| 07:21:14.376 | PATCH {"archived":false} | 200 |

Both are recorded in http-recorder.json. This is a durable lane_owner_replies boundary test, not a live authenticated Discord gateway inbound message. The harness's final existing block() records exactly that limitation and produces exit 3; every requested lifecycle assertion completed before it.

## Isolation and cleanup

Unchanged scratch HOME/workspace/DB/runtime-lock isolation, owned app-server, Discord gateway disabled, loopback REST recorder and in-memory TYPESAFE configuration. No shared daemon, owner DB, env file, production gateway or live Discord operation was invoked by this QA run.

Owned app-server ws://127.0.0.1:59676, PID 68341, private 0600 token. Started 2026-10-12T07:14:12.160Z, finished 2026-10-12T07:21:40.192Z, total 448033 ms. Cleanup: remainingGroupPids [], pidAbsent true, portRebindSucceeded true, privateScratchRemoved true. SIGKILL was required after the owned-group grace period. Scratch gateway PID 68490 exited 0, pidAbsent true, socketRemoved true. TYPESAFE receipt valuesRecorded=false.

Raw artifacts retained under scratch: task15-live-final.json, settlement.json, reopen-receipt.json, final-machine-state.json, http-recorder.json, descriptor.json, qa15-clean-brief.md, task-15-live-final.log, DB and git repository. A log search for worktree/force produced no matching lines; non-forced removal evidence comes from the close intent, removal receipt, immediate git snapshot, existing path-absence assertion and inspected source call.

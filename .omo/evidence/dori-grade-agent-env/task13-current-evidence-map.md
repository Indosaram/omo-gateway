# Task 13 current evidence map

Reconciled 2026-10-11 by read-only inspection. This evidence file is the sole intentional write. No subprocess, Cargo/test/format command, git command, database operation, model call, network request, service operation, or source/test edit was executed. Existing results below are attributed to their receipts, not to this reconciliation. Task 13 is not confirmed.

## Authority and inspected scope

- Main plan: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`, read in full in bounded chunks; task 13 at lines 301-309. Relevant companion contracts: todos 5, 7, 8, 9, 12, 14, 15, 19 and 35, verification strategy and owner safeguards.
- Main ledger: `/Users/indo/code/project/omon-gateway/.omo/ulw-execute/ledger.jsonl`, read in full in bounded chunks. Relevant records include `wave3-parallel-snapshot-and-dispatch`, `todo13-source-settled-pre-review-dispatched`, `todo13-pre-review-request-changes`, `wave3-focused-batch-failed-grouped-results`, `todo13-path-fixture-repair-source-settled`, and later runner race/8-9 confirmation records. The unrelated older migration-plan todo 13 record is not evidence for this task.
- Full worker receipt: `E/task-13-source.md`. Full independent source pre-review: `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo13-pre-review.md`.
- W = `/Users/indo/code/project/omon-gateway-wt/dori-work`; N = `/Users/indo/code/project/omon-gateway-wt/dori-next`; E = `W/.omo/evidence/dori-grade-agent-env`.
- Full bounded reads of W/N `src/lanes/runner.rs`, `src/lanes/cli.rs`, `src/lanes/mod.rs`, `Cargo.toml`, and N `src/lanes/settle.rs`, `tests/test_lane_done.rs`. W settle module and done test file are absent. Read W `src/main.rs` in bounded chunks, migration 0031 and 0033 definitions, exact focused-result receipts and relevant scratch QA artifacts. Discovery stayed within named plan, ledger, worktree and evidence directories.

## Current presence versus next

W already has transactional claim/object handling in `src/lanes/cli.rs:431-524`: persisted claim timestamp/evidence, done_claimed event, objection storage and queued reasons. Claims scrub agent evidence in event bodies; owner objection/blocked-answer text is preserved. W also has done_claimed steering eligibility, generation-fenced consumption, cancellation/reopen handling, task-local CAPTURE_GENERATION, and generation-fenced captured activity and failure handling in its runner. These are existing behavior, not new task 13 hunks.

W does not have `Settler`, settlement injection, the 30-second settlement task, five-minute automatic verification, interrupted-settlement recovery, the done acceptance target, or KST deadline guidance. W's claim update does not clear a previous objection. N supplies those additions. Presence in N is not integration into W.

The runner difference is narrow except for one dangerous backward change: N unconditionally constructs `DiscordThreads::from_env`, whereas W `src/lanes/runner.rs:318-331` constructs the sink only when OMON_DISCORD_ENABLED is enabled. Copying N's runner wholesale would break the current token-free lane startup contract. Current W main also has the newer headless lead runtime, dispatcher/drain watcher, joined shutdown, local transcript output, and explicit System origin handling. Those files are not part of the minimal task 13 port.

### Minimal post-freeze patch scope

This is a patch map, not permission to apply it during the freeze.

1. Add N `src/lanes/settle.rs`, with the boundary correction described below. Add N `tests/test_lane_done.rs`, preserving its canonicalized repository fixture and current failure assertions.
2. Add only `pub mod settle;` to W `src/lanes/mod.rs` and only the `[[test]]` registration for test_lane_done to W Cargo.toml. Do not copy N Cargo.toml: its test registration ordering and task 18 additions are unrelated.
3. In W CLI, add `change_claim_at` while keeping `change_claim` as the production-now wrapper; clear objection only for a fresh claim; compute the persisted claim's +5 minute KST guidance. Keep current write-lock transaction, legal transitions, blocked-answer semantics, generation CAS, scrubbed claim event, and exact owner text preservation.
4. In W runner, add the Settler field/default construction, `with_settlement`, `settle_due`, a separate settlement JoinSet and Skip interval, result notification, cancellation/join on shutdown, and verified_done_ready hook. Preserve W's conditional Discord sink initialization. Preserve backend-turn JoinSet ownership, CAPTURE_GENERATION inside the spawned turn, ready-turn/reopen cancellation fences, inbox ACK fencing, scrubbed failures, single-pool behavior and current socket/post scheduling. The inspected diff shows the current turn-generation logic already matches N; do not re-port that earlier repair.
5. Do not replace main.rs or any EventOrigin/shared headless wiring. No migration or model/config change is proposed. Todo 15 owns actual close/startup verified_done recovery; N's verified_done_ready currently only logs. Todo 35 owns its optional done-diff decision seam before result commit.

### New generated-event scrub boundary

N `src/lanes/settle.rs:97-111` joins raw signal/git errors into detail and inserts that raw detail into the gateway-generated not_done event. Git reasons include absolute paths; arbitrary signal failure detail can also contain secrets. Unlike W CLI's owner objection, this text is gateway-generated and must follow task 19. The minimal port must scrub the event-body value at that boundary, retaining useful local machine reasons and owner-authored text according to the current contract. Do not scrub all CLI input or weaken git reasons to hide a failure. Existing runner failure scrubbing does not cover this new insert. Add the appropriate behavior regression in the repository's existing scrub/done test seam after freeze; it is not proven by the source pre-review's APPROVE.

## Acceptance and result map

The concrete raw execution is `E/next-focused-20261011T140523Z.log`, with matching status TSV exit 101. The done target reports 6 passed, 1 failed, 0 ignored, 4.20 seconds. The seven-target batch reports 61 passed, 6 failed, 1 ignored. It is not a green batch.

| Contract | Existing test or source locator | Evidence and limit |
| --- | --- | --- |
| Claim then object gives not_done and queued reasons | `claim_then_object_queues_reasons_and_returns_not_done` | Raw test PASS. Asserts status, objection, pending inbox contains reasons and zero checks. Does not prove real agent receipt or subsequent resumed turn. |
| No settle before five minutes; verified at boundary | `passing_signals_and_clean_tree_verify_only_after_five_minutes` | Raw PASS: MockClock at 299 seconds leaves done_claimed/check count 0, at 300 seconds gives verified_done/check count 1. Fake signal succeeds without a real proof artifact. |
| Failure returns machine detail/event | `failing_signal_returns_not_done_with_live_detail` | Raw PASS: not_done, objection and event contain fake failure detail, count 1. Does not run the live file checker or assert the real agent receives guidance. |
| Dirty tracked work refuses verification | `dirty_tracked_file_returns_uncommitted_reason` | Raw PASS: existing tracked file modified, result not_done/uncommitted. No untracked-file refusal is asserted or required by the written command. |
| Restart before deadline settles once | `runner_restart_uses_persisted_deadline_and_settles_once` | Raw PASS: drop/reconstruct runner on the same fixture DB at four minutes, settle at five, subsequent calls do nothing, one signal/backend call and one verified history row. Fake object restart, not process restart or file-backed DB reopen. |
| Interrupted reservation avoids signal replay | `interrupted_settlement_recovers_without_replaying_signals` | Raw PASS: seeded history reservation leads to not_done and one failure event with zero checks; fresh claim later checks once. This is the documented at-most-once recovery policy, not automatic eventual successful retry. |
| Unpushed commits with remote refuse verification | `unpushed_commit_with_local_bare_remote_returns_not_done` | Raw FAIL at historical line 393: expected path-containing unpushed reason. Ledger records later source-only canonicalization of temp cwd (/var versus /private/var) and added actual-reason assertion message. Current N contains that fix, but no post-fix done-target run was found among the inspected named receipts. Root cause remains source-derived. |
| Cwd/descendant selection avoids metadata; external birth-time boundary | `settle::tests::cwd_and_descendants_do_not_query_creation_metadata`, `external_selection_uses_creation_time_not_modification_time` | Source present: panic-if-polled lazy future, pre/boundary/post timestamps, unsupported-error propagation. No named execution result for these two unit tests in the inspected focused logs. |

Fixture backend uses a pre-established mpsc claim signal, bounded channel/task waits and MockClock advancement. No fixed sleeps are used. Checks counts signal executions; all current fixture lanes declare one signal. Existing tests do not separately demonstrate a multi-signal claim, competing Settler instances, objection/reopen during an in-flight checker, or live 30-second scheduling through serve. These are residual proof limits, not fabricated failures.

N settlement commits a claim-scoped history reservation before async signals/git. Reservation scope uses reopen generation and latest claim history ID. Result update CAS checks status, claim_at and reopen generation. Successful status excludes subsequent scans. On interrupted reservation it fails closed to not_done and requires a new claim rather than replaying a potentially side-effecting command. Exactly-once invocation is not exactly-once external effect: a crash cannot prove whether a command's effect happened before termination.

## Historical gates versus current integration

- `E/next-compile-20261011T135806Z-{status.tsv,diagnostics.md}`: historical N all-target/all-feature no-run exit 0, with an unrelated posts unused-import warning. This establishes historical compilation, not current W integration.
- `E/next-static-20261011T141744Z-*`: historical fmt exit 1 (backend baseline), clippy exit 101 (posts test lock-across-await findings). The source pre-review's Compile & Types PASS is static assessment, not a executed clean gate.
- `E/next-clippy-20261011T145201Z-{status.tsv,diagnostics.md}`: subsequent N strict all-target/all-feature clippy exit 0. It does not rerun the failing unpushed behavior.
- `E/next-focused-20261011T145256Z-*`: runner/reopen exit 0, 15 passed and 1 ignored. No test_lane_done in that command. Earlier reopen failure is not still credited as unresolved once this narrowly matching repair evidence exists; neither does its fix close task 13.
- W fixture-repair provenance records token-free/enabled/default runner startup cases and runner target 12 passed/1 ignored. Preserve this newer W startup behavior instead of N's unconditional token path.
- The inspected `E/combined-current-20261011T173003Z-status.tsv` contains fmt 0, clippy 101, headless 0 and focused 0. Its focused command covers dispatch/inbox/thread/post transport/events, not test_lane_done. Do not infer a passing combined gate, await/restart a build here, or promote partial status into task 13 confirmation. This reconciliation does not alter that run or diagnose its unrelated clippy failure.

## Git scope, QA constraints and exact tradeoff

The plan names `git status --porcelain --untracked-files=no`: only tracked/index dirt rejects. An untracked done.txt alone is deliberately invisible to that check. The current scratch driver initializes qa-repo, forbids commits in the agent brief, and asks for done.txt. Therefore a clean initialized repo with only this untracked artifact does not inherently fail the dirty check. It can satisfy the file signal while proving nothing about committing/pushing a product change. Confirm repository status in a future authorized QA before assigning this conclusion to an actual run; no git command was executed here.

If a future QA changes an already tracked done.txt, stages the new file, or changes any tracked file, the exact same check returns uncommitted. The owner's no-commit/no-push rule must not be bypassed to make the owner repository or a task worktree clean. A scratch directory is not permission to commit owner code. Product W remains intentionally uncommitted under the ledger's explicit owner override; running a product lane's clean-git success QA there would legitimately fail, not authorize an automatic commit/push.

The written alternatives have distinct contracts:

- Preserve task 13's tracked-only cleanliness contract: untracked file-signal QA is a valid narrow signal/deadline scenario; tracked-dirt and unpushed cases remain separate synthetic tests. Do not describe it as all files clean or all work committed.
- Require untracked files to reject: removing `--untracked-files=no` is a contract change, not a fixture repair. Existing untracked done.txt happy QA would then fail unless the artifact is outside the git repo (with an appropriate file signal), intentionally ignored, or part of separately authorized synthetic fixture setup. Each alters what cleanliness is proven. Do not make this change silently.
- Tracked artifact QA under the existing no-commit agent brief: unchanged baseline artifact can pass, but modifying it must fail. There is no exception allowing the agent to commit it merely to turn QA green.

Synthetic git fixture source in N is explicitly owned by tempfile: it initializes repo, commits tracked.txt using test identity/no global config, creates a local bare remote, pushes only that fixture baseline, and leaves one local fixture commit unpushed. The source receipt says these were initially unexecuted; the historical raw focused run subsequently proves they were executed. A future authorized run of these existing tests necessarily performs those synthetic operations. That is a separate fixture scope, not authority to commit/push W, the main checkout, an owner's repo or a real remote. This turn's explicit no-command/no-git instruction excludes even synthetic execution now. Any broader commit prohibition applicable to a later run must be resolved explicitly rather than bypassed using test machinery.

Worktree scope in N: always canonical cwd, then Git worktree-list paths under cwd or whose creation evidence is at/after lane.created_at. External selection first uses filesystem birth time; if unsupported it checks the linked Git registration back-pointer and registration-directory birth time. No modification-time fallback; unsupported evidence rejects rather than silently excluding. Cwd/descendant paths short-circuit metadata. This is not a precise ownership registry: a newer unrelated sibling worktree can be included, and `git log --branches --not --remotes --oneline` checks all local branches, not only the lane branch. Preserve the written contract during minimal port; do not expand it into deletion/close logic. No fetch occurs, so unpushed judgment uses existing local remote-tracking refs, not live remote freshness.

## Explicit missing runtime, migration and integration proof

`E/task-13.log` is absent in the bounded pathname check. Existing `E/task-8-9-live-GhvMrT-1791737815001/{task8.json,task9.json,task-8-recovered.json}` prove real explicit-scratch claim and same-thread reopen/file append, not settlement. Their history goes done_claimed -> forced not_done/working -> closed, and contains no verified_done or settlement_started transition. Observed claim times 16:46:34 and 16:57:15 with later forced cleanup do not become successful five-minute settlement evidence merely because elapsed wall time exceeded five minutes. Old observer-timeout artifacts remain preserved; the later recovered authority is not overwritten here.

Outstanding proof after the freeze and minimal port:

1. Source-linked W compilation/strict checks and post-repair done-target execution, including unpushed test and selection unit tests. Use the parent's coordinated batch policy; no per-hunk broad reruns or duplicate active build.
2. Real task 13 scratch gateway happy case: file signal, persisted claim, no early verification, then actual verified_done after deadline through runner scheduling. Record real checker/result/history and exact DB/binary/source identity. Close remains todo 15's separate responsibility.
3. Real failed-file claim gives not_done with file detail and actual lane transcript/agent receipt of guidance, followed by resumed work. Existing fake reasons test is not this integration proof.
4. Real owned gateway restart on the same scratch DB before deadline; verify reservation/result durability and once-only behavior. Separately retain the interrupted-reservation safety policy, not an eventual-retry claim.
5. Fresh and representative pre-existing scratch database migration/startup compatibility for the integrated settler. Migrations 0031/0033 contain the required columns/tables and no new migration is proposed; schema source inspection and earlier task 8 DB initialization are not proof of task 13 migration/restart behavior. Never open or mutate owner DB to obtain this evidence.
6. New settlement-generated event body honors task 19 scrub contract; current owner's object/answer text remains intact. Headless/no-token startup, origin, dispatch, generation/cancellation and shutdown behavior must survive integration.

Conclusion: task 13 is partially present in current CLI/runner and implemented as an unintegrated N settlement addition. Six historical done cases pass; repaired unpushed behavior, real scheduling/restart/migration integration and safe generated-event scrubbing remain unproven. The bounded patch above preserves newer W contracts and does not imply confirmation, commit, push, deployment or authority to execute QA during the product freeze.

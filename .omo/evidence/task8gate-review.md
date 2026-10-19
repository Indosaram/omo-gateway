# Task 8 renewed independent gate review

recommendation: REJECT
scopeVerdict: scope-ready; overall closure blocked only by incident disposition
supersedes: Previous contents of this gate report, including B1 rejection.

## Original intent and desired outcome

originalIntent: Implement todo 8 in the MAIN checkout's `.omo/plans/dori-grade-agent-env.md`: persistent gateway-owned worker lanes and `omo-gateway lane` controls, keeping long work in separate sessions/folders without raw output to Discord or a second production daemon.

desiredOutcome: Actual CLI start reaches production LaneRunner/OmoBackend, executes the brief in the selected folder, captures progress and persists its thread/outcome/claim; restart, steering, turn/error limits, model controls and guard Hold behavior work at the specified boundaries. Settlement, report-to-Discord integration, complete worktree closure and advanced reopen behavior remain later todos.

userOutcomeReview: Scope-ready on the reviewed source and artifacts. The superseding receipt now exercises ordinary `omo-gateway run`, not the ignored test harness. Explicit Discord-disabled mode uses the shared production lane constructor, real service and cancellation/join semantics. The user explicitly authorizes disposable endpoint 29749 as the substitute for unavailable 19742. This resolves B1. The only blocking remainder is disposition of the earlier unintended repository-database writes; safe subsequent QA does not erase that incident.

## Remaining blocker

### B2: Unintended repository database changes await disposition

violatedCriterion: PLAN-QA-ISOLATION, Verification strategy: "Never write to the live DB from QA"; renewed review explicitly requires incident disposition before task 8 can fully close.

evidencePointer: `.omo/evidence/dori-grade-agent-env/task-8-live-db-incident.md`, Findings 1-4 and Proposed targeted cleanup; `task-8-revision-doneclaim.json`, incident.disposition; `task-8-review-followup.md`, final paragraph.

Observation: Read-only incident child c7 confirms migrations 31-33 and the stray qa-1 row in `/Users/indo/code/project/omon-gateway/omon_gateway.db`. It records zero persisted worker activity, not universal absence of external effects. Owner deletion approval and migration disposition are pending. No cleanup was performed. The report also correctly distinguishes the repository target from the separately observed runtime gateway resource; this review does not label it a confirmed running Rust gateway database.

Required next action: Obtain and record owner disposition for the exact stray row and migrations. If cleanup is approved, the authorized operator must perform the bounded action with identity/activity checks and attach its receipt. If the owner accepts retention instead, record that explicitly. Do not delete migration records or drop shared tables as an inferred cleanup. No additional product change or broad validation run is required by this gate before that decision.

Root-cause distinction: Rust CLI reads DATABASE_URL normally; Database::connect applies migrations before command validation. The earlier launcher failed to override inherited environment, including during an invalid command. This is not proof that production CLI ignores correctly supplied environment. The new launcher uses explicit Bun.spawn env, and receipts identify the scratch target. No new command was run against the repository DB by this reviewer.

## Resolved B1 and production-path assessment

Reviewed `src/main.rs` diff and current `src/lanes/runner.rs` lifecycle. `run_gateway` defaults to Discord enabled. Explicit `OMON_DISCORD_ENABLED=false` requires DATABASE_URL, opens/logs the target DB, calls shared `spawn_lane_service`, and waits for service termination or a shutdown signal. It then cancels/joins and closes the pool. The Discord-enabled branch calls the same constructor and now selects on lane-service termination, shuts down Discord clients and returns the failure after its normal teardown. `lane_service_exit` treats even an unexpected successful service return as failure.

The mode is a bounded production configuration path reached by the ordinary entry/CLI; it is not a test binary, alternate agent loop, or second always-on daemon. Shutdown orchestration is written in two branches, rather than literally being one shared function, but the actual lane initialization and stop contract are the same. The small constructor and exit-result helpers have two concrete callers and are justified, not gratuitous extraction. Discord transport and cron startup are intentionally omitted in this mode; no proof of their live integration is claimed or needed for the authorized task-8 substitute.

Checked actual-surface evidence:

- `task-8-qa-launcher.txt` invokes the ordinary binary with `run`, explicit scratch DATABASE_URL, OMON_DISCORD_ENABLED=false, OMON_LANES_ENABLED=true and engine URL 29749. It invokes real `omo app-server` for the owned QA engine and never the ignored runner harness.
- `task-8-real-gateway.json` records actual CLI start exit 0 and readback: gateway-qa, turns_run 1, completed, done_claimed, claim evidence, and persisted thread `01a129d1-437e-7c61-8747-161b450cf0b7`. The transcript identifies the production executable, contains the REPORT milestone, and ends in a completed outcome. The copied file readback is `qa complete`.
- Duplicate ordinary gateway startup on the same scratch DB logs `/private/tmp/omon-task8-review-Fy6m13/qa.db`, fails the runner ownership lock, and exits 1. This is real startup failure propagation, not proof of every runtime fault.
- Actual invalid done command exits 2; actual forced close exits 0. `task-8-revision-gates.txt` records real active-turn steering on the persisted thread and graceful gateway stop with exit 0. The follow-up identifies SIGTERM and PID 63508.
- Cleanup receipts report gateway 63508 and owned engine 61137 absent, port 29749 clear, scratch root/socket/lock absent. The raw process/port receipt is abbreviated, so command arguments and filesystem cleanup remain provenance from the follow-up/doneclaim rather than independent re-probes.

These inspected, mutually consistent artifacts resolve the required actual gateway QA gap with the explicitly authorized endpoint substitution. No services were launched or re-tested by this reviewer.

## Scope and acceptance review

Existing core implementation and tests are unchanged except runner readiness logging; hashes confirm this. Prior direct inspection and independent 14-test run remain relevant to those contracts.

- Chaining: fake backend drives multiple turns to a real storage claim; prompt includes brief/footer, cwd and selected model. Actual gateway receipt now backs real one-turn execution and claim.
- Limits: tests assert 24 turns produce failed plus turn_cap event, and three consecutive errors produce failed plus dead event. Runner reconstructs consecutive errors after reopen boundary and stops on nonworking state.
- Persistence: fresh service on the same file DB resumes stored thread state; durable session binding wins over stale lane binding. Service ownership prevents duplicate runners.
- Controls: actual CLI invalid syntax, Hold-to-queued and forced-close requirements are covered. In-flight send invokes steer and consumes only after acknowledgment; idle inbox becomes next prompt. Real claim acknowledgment steering is also logged.
- Models: small selection, row-to-session propagation and once-only escalation after two failed checks are inspected/tested. Auto delegates to Judge; conservative large on Judge failure follows todo 33. Additional auto/large/env-override CLI tests would improve coverage but their absence is not a demonstrated criterion failure.
- CapturingDispatcher has no Discord transport and does not interpret brief content as a shell. Raw stream snapshots remain transcript rows; milestone deduplication is a later report integration concern.
- Guard checking and insertion are not atomic across concurrent starts. Keep this prior note for the IS-8 integration owner; no new concurrent-start reproduction was run and the task-8 Hold criterion is satisfied. No assertion of a strictly reserved concurrent slot is made.

## Nine adversarial classes and exact boundaries

| Class | Checked evidence | Remaining boundary, not an additional blocker |
| --- | --- | --- |
| Malformed input | Binary invalid-done exit 2; test asserts no lane; renewed actual CLI receipt | Broad invalid-control/config matrix not supplied |
| Prompt injection | Literal `$()` is data and not shell-executed by runner | Not proof of model resistance to malicious instructions |
| Cancel/resume | Bounded fake cancellation and fresh file-DB service recovery | Actual gateway SIGTERM was after a completed turn, not during a remote tool |
| Stale state | Durable binding preferred over stale lane row | Remote missing-thread fallback belongs to later reopen scope |
| Dirty worktree | Claim preserves file and does not verify/close | Dirty-tree settlement checks are later scope |
| Hung commands | Semaphore-blocked fake backend cancels | Default no-op fake cancel does not prove remote process termination |
| Flaky tests | Raw final-source 14-pass run; existing handshake-based tests, no fixed sleeps | Some receives/joins are unbounded; model test assumes default override environment |
| Misleading success | Done-looking text cannot verify or close a lane | Does not implement later machine-check settlement |
| Repeated interruptions | Three cancellations retain working state/binding | No full gateway active-remote-interrupt series or near-cap restart receipt |

## Direct remove-ai-slops / programming pass

The named skills remain unavailable in the advertised skill set and previously inspected skill directories; supplied task criteria were applied directly again. No new tests were added in this revision. No deletion-only/removal tests, tautological tests or new implementation-mirroring assertions appeared. The new production helpers have actual shared use, no unnecessary parsing or normalization was introduced, and no generic supervisor layer was added.

Prior test-honesty notes stand: dirty-worktree setup is low-yield for a DB-only claim and cannot validate settlement; shell-text nonexecution is not broad prompt-injection safety; fake cancel is not remote interrupt proof. The follow-up explicitly acknowledges these boundaries. I found no new maintenance burden or scope drift that violates a stated task-8 criterion.

A separate code-review report explicitly enumerating all skill/slop criteria is still not supplied. The follow-up acknowledges the prior review and overclaim limits but is not an independent skill-perspective review. This report performs that direct check; the missing separate report is an evidence note, not an invented product blocker under the approval rule.

## Checked paths and verification provenance

Plan previously read in full: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.
Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Renewed artifacts inspected:
- `.omo/evidence/dori-grade-agent-env/task-8-revision-doneclaim.json`
- `.omo/evidence/dori-grade-agent-env/task-8-real-gateway.json` (opening state/prompt and final transcript/outcome/receipt sections; intermediate repeated stream snapshots were not all reread)
- `.omo/evidence/dori-grade-agent-env/task-8-review-followup.md`
- `.omo/evidence/dori-grade-agent-env/task-8-revision-tests.txt`
- `.omo/evidence/dori-grade-agent-env/task-8-revision-gates.txt`
- `.omo/evidence/dori-grade-agent-env/task-8-qa-launcher.txt`
- `.omo/evidence/dori-grade-agent-env/task-8-live-db-incident.md`

Current diff inspected for main; revised runner lifecycle read. All nine owned source hashes were independently computed and match revision-doneclaim, including main `9042612cab01242bb07a351c736fe5ece8a436387224f25bcae8940743384ffa` and runner `f2e24aa47e5d0e5f99d41cd2a91af032c73a037890f3d6667cc1c5fbdbe778cf`. The prior main-hash gap is resolved.

Independently executed in renewed review: `rustfmt --edition 2021 --check --config skip_children=true src/main.rs src/lanes/runner.rs`, exit 0. Inspected raw final-source scoped output: 3 CLI plus 11 runner passed, one explicit live harness ignored, exit 0. Inspected raw clippy completion: exit 0, 1m 06s; the preserved launcher supplies the exact `--all-targets --all-features -- -D warnings` argv. No test/clippy/broad suite was repeated in this review. Prior full fmt baseline difference in backend.rs and coordinator-owned full-suite/dashboard comparison are not claimed resolved by this task.

Incident findings are from the assigned read-only child artifact, not a duplicate investigation. Live actions and cleanup are inspected receipts, not newly reproduced operations. No notepad path or JS eval/status tool is available; the requested task-specific fallback report location is retained.

## Completion boundary

Only this gate report was rewritten. No product edits, checkbox changes, DB writes, cleanup, service actions, external messages, delegation or commits occurred. B1 is closed. Scope is ready. Overall recommendation remains REJECT solely until B2 owner disposition is recorded; task 8 must not be marked fully complete before then.

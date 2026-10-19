# Todo 30 final bounded source verdict

recommendation: APPROVE (source handoff only)
verdict: source-ready-tests-QAglobalpending
blockers: []

Never confirmed: Cargo, compilation, tests, scratch QA and global gates are UNRUN. This is approval of the bounded source corrections, not completion of todo30 or a runtime gate. Only this report was written; no product changes or live operations.

## Intent and outcome

originalIntent: Resolve the last startup-fixture eligibility finding while preserving the already-reviewed cron-health corrections and isolated kill-off fixture.

desiredOutcome: A required enabled-job startup test that observes the missed row before any clean terminal success, releases its fake executor deterministically, and shuts down/restarts without leaving a blocked execution.

userOutcomeReview: The last finding is resolved in the inspected source. The positive fixture remains an ordinary tokio test and now inserts enabled = 1. Startup synchronously completes the health scan before spawning the scheduler loop. The fake executor can start but cannot return its clean result until the fixture checks the missed-row count and sends the release signal. Cleanup joins that execution before restart. No remaining source blocker was found within this explicitly bounded review.

## Last finding and introduced synchronization

Evidence: `/Users/indo/code/project/omon-gateway-wt/dori-work/tests/test_cron_health.rs:10-26,369-427`.

- Eligibility: The missed-job INSERT now sets enabled = 1, so the already-inspected list_active enabled-only query includes it. The required positive test is neither skipped nor ignored.
- Window: scheduled = now - 2h and interval = 1h retain the completed [now - 2h, now - 1h) window. This arithmetic and the late-run negative at now - 90m were approved in the preceding pass, not expanded into another schedule audit.
- Ordering: FakeExecutor sends its started Notify, then awaits its optional release Notify before returning its result. The test checks the missed count immediately after start(). Production start() awaits detect_startup_missed before spawning the due loop (`src/cron/scheduler.rs:678-679`), so no wait-by-luck is needed to observe the row. The release gate prevents a clean completion before that observation.
- Lost-signal avoidance: notify_one stores a permit when the receiver has not begun awaiting. Both the started notification and release notification therefore tolerate either scheduling order; no sleep or polling is introduced.
- Bounded started wait: The test wraps execution_started.notified() in a two-second timeout. It saves the Result instead of immediately panicking, sends release_execution.notify_one(), awaits scheduler.shutdown(), and only then asserts the started wait succeeded. Thus that timeout path does not strand the gated executor.
- Shutdown: scheduler.shutdown() signals stop, joins the scheduler task, and then joins all tracked executions (`src/cron/scheduler.rs:754-763`). Release is sent before this join. On the successful expected path, the clean due run finishes and advances next_run_at using the fixed clock (`:1105,1503-1506`), leaving no due execution needing a second release.
- Restart: The second start still performs its synchronous scan before the loop checks shutdown. It does not introduce a new blocked execution: the first shutdown has joined the released execution and its successful completion moved the schedule into the future. The retained shutdown flag also prevents that restarted loop from proceeding. The subsequent shutdown joins the task. The test's second count remains an idempotence check, not evidence of a full scheduler restart/resume feature beyond this task's scope.
- Other FakeExecutor users shown in the changed fixture retain release: None; the gate is opt-in. No extra production behavior was introduced by this fixture patch.

No concrete newly introduced synchronization, cleanup or restart defect was found. This reasoning is source inspection, not an executed test result.

## Prior bounded approvals preserved

The immediately preceding review resolved: persisted completed-window calculation, removal of the periodic full health scan, primary-only delivery flag, eleven-value late-run INSERT, and direct parsing of the unwrapped String. Synchronous post-commit health awaits and non-fatal health preparation were previously inspected. Those conclusions are preserved without reopening unrelated code.

The isolated kill-off fixture source approval is unchanged: exact child helper selection avoids recursion, process-local environment avoids shared-env races, timeout/kill_on_drop bounds the child, and actual failed terminal state plus zero problem/event rows are asserted. Runtime remains UNRUN.

The plan-required timer source-pattern assertion is allowed machine-code checking, not prose pinning. The last false-confidence issue (excluding the positive fixture from production selection) is now removed. No new overfit/slop or unrelated cleanup finding is added by this bounded pass.

## Checked artifacts and limits

Read in this pass, under `/Users/indo/code/project/omon-gateway-wt/dori-work`:

- `tests/test_cron_health.rs:1-62`: optional fake-executor release gate and setup.
- `tests/test_cron_health.rs:367-465`: enabled positive fixture, notification ordering, release/shutdown/restart, and adjacent unchanged negative fixture.
- `src/cron/scheduler.rs:742-764`: shutdown joins.
- Targeted scheduler references at :678-681, :1105 and :1503-1506: synchronous startup, schedule-advancing due claim, and success schedule advance.

The preceding report's source/receipt/migration/kill-off checks are retained as prior evidence; no new generic audit was performed. No separate runtime evidence was produced.

## Evidence gaps and nine-class status

All nine runtime classes remain UNRUN: terminal idempotency; reclaim/late completion; primary delivery; expect/no-expect; recovery; startup/catch-up/late start; kill-off/error isolation; event atomicity/lead delivery; and deterministic lifecycle/no polling/model behavior. Source findings in this bounded correction track are resolved; that does not prove execution or event receipt. External service/resource adversaries remain N/A to this source-only review.

Required next gates outside this assignment: compile and focused tests, repository/global checks, named scratch-gateway QA with actual timestamps and captured event/delivery evidence, and cleanup receipts for any resources those runs create. No baseline comparison, compiler success, test success or live QA result is claimed. Todo30 is not confirmed complete.

Cleanup: no processes, databases, jobs, services, channels or temporary resources created; nothing left running. Only this report updated. Stop condition met: final bounded source verdict delivered.

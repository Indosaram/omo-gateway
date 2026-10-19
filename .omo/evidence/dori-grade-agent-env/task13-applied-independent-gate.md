# Task 13 applied independent source gate

Date: 2026-10-11. Independent reviewer; this reviewer did not implement the unit.

Verdict: SOURCE GATE BLOCKED on the shutdown error-exit finding below. The claim-identity and settlement logic receive bounded source approval subject to the limits stated here. This is not final Task 13 confirmation, runtime approval, or deployment approval. Strict clippy/build and live scheduling/restart QA remain pending in separate tracks.

## Finding requiring correction

### High: runner service errors can bypass cancellation and explicit joins

References: `src/lanes/runner.rs:379`, `:385`, `:415`, `:418`, `:422`, `:426`; shared cleanup at `:431-455`; regression at `tests/test_lane_done.rs:588-677`.

Normal shutdown and errors returned by the settlement JoinSet break to the shared cleanup. That cleanup cancels the shutdown token, aborts and drains settlement tasks, then cooperatively joins lane tasks before returning the first error. Those paths are correctly structured.

Other fallible operations inside the running service still use direct `?` exits: lane listing, reopen-generation lookup while launching lanes, post draining, socket receive, lane-task join failure, and failure persistence. Any such error with an active settlement or lane backend returns from `serve` before the shared cleanup. Dropping the JoinSets aborts their tasks but does not explicitly join their cancellation completion; the shutdown token is not cancelled by that path. In particular, a posts-drain error while both a checker and a backend are held can cause `serve` to return without the backend's cooperative cancellation completion or joined lane persistence. This contradicts an unqualified shutdown cancellation/join guarantee.

This is an existing structure exposed by the applied settlement integration, not a claim that the new normal-stop or settlement-error regression failed. The passing regression injects a settlement status-update error or requests normal stop; it never exercises these other service-error exits.

Required correction: route all failures after task launch through the same first-error/cancellation/drain path, preserving the initiating error. Required deterministic regression: subscribe before starting `serve`, hold an active checker and backend, inject a non-settlement service failure (for example post draining), assert cancellation is delivered and `serve` remains unfinished until backend completion is released, then assert checker teardown, joined lane completion and the original error. Also cover a lane-task join error or establish that the shared path handles it. No source correction was made by this reviewer.

## Reviewed invariants and bounds

### Persisted claim identity CAS and same-timestamp reclaim

`src/lanes/settle.rs:39-74` reloads the authoritative lane after obtaining SQLite's write lock, rather than reserving the earlier list snapshot. It reads the latest reopen history ID and latest history ID for an actual transition into `done_claimed` in that same transaction. `src/lanes/cli.rs:435-527` serializes claim/object mutations and inserts claim history with their lane update, event and guidance.

The result CAS at `src/lanes/settle.rs:101-110` checks status, claim time, reopen identity and claim history identity. An objection followed by a new claim at the same timestamp therefore invalidates both an old success and an old failure. A zero-row CAS rolls back before result history, event or inbox writes (`:111-124`). `tests/test_lane_done.rs:458-528` deliberately holds the old checker, objects and reclaims at the original timestamp, checks no old-result artifacts were added, and checks the fresh attempt can settle. This is the right regression for timestamp aliasing.

### Restart reservation and signal execution semantics

`src/lanes/settle.rs:68-79` commits `settlement_started` before checking signals. The reservation search excludes earlier claim/reopen identities through its history-ID lower bound. A restart after reservation returns the current claim to `not_done` with an interrupted-settlement reason and does not replay arbitrary command signals. A new claim creates a fresh identity and can check again. Tests at `tests/test_lane_done.rs:287-358` cover a restart before reservation, repeated settlement calls, interrupted-reservation recovery and a subsequent claim.

The exact guarantee is at-most-once signal-check initiation per reserved claim identity, with one result transition on successful CAS, not crash-proof exactly-once completion of every signal. A crash after reservation but before checking can run zero signals; a crash halfway through can leave a partial set. Both recover conservatively rather than replaying. `src/lanes/signals.rs:146-155` checks declared signals sequentially once in the default `check_all`. Cloned settlers share an in-process serial mutex (`src/lanes/settle.rs:20,35-36`), and the runner permits only one scheduled settlement task at a time (`src/lanes/runner.rs:394-398`). Database reservation prevents another settler from re-executing the same claim, but an independently constructed concurrent settler can interpret an in-progress reservation as interrupted and conservatively reject it. The service's database ownership lock (`src/lanes/runner.rs:227-256`) is therefore part of the intended single-runner operating boundary.

### Five-minute boundary and objection races

`src/lanes/settle.rs:36-63` captures a clock value and excludes claims newer than that value minus five minutes. Equality is eligible: 299 seconds does not check, 300 seconds does (`tests/test_lane_done.rs:227-246`). Capturing time before the scan can delay eligibility until a later tick; it cannot settle early. The service interval is 30 seconds and skips missed ticks (`src/lanes/runner.rs:374-375`).

An objection committed before the result CAS makes status/time fail that CAS. An objection and same-time reclaim additionally change the claim history identity. If verification commits first, a later object is rejected by the transition rules (`src/lanes/cli.rs:451-475`; `src/lanes/state.rs:55-60`); it cannot change `verified_done` back to `not_done`. This gives transaction-order race semantics, not priority for an objection merely started before verification committed. Objection while checks execute does not cancel already-running signals; it prevents their stale result from being persisted. The current tests cover objection before checks and objection/reclaim during held checks, but not an objection-only held-check race or both boundary commit orders directly.

### Lane cwd and worktree scope

The CLI canonicalizes and persists the lane cwd (`src/lanes/cli.rs:334-342`); the worker session carries that cwd (`src/lanes/runner.rs:472-476`). Settlement passes the stored cwd to signal checking (`src/lanes/settle.rs:82-84`) and canonicalizes it for git checks (`:156-160`). Git commands use explicit `-C`, timeout and kill-on-drop (`:133-154`, `:162-174`), not the gateway process cwd.

Worktree inspection always includes lane cwd, then listed worktrees under it or whose creation time is at least lane start (`src/lanes/settle.rs:181-235`). External selection uses filesystem birth time, with a checked linked-worktree registration birth-time fallback and an explicit failure if creation evidence is unavailable (`:238-277`). It does not silently substitute modification time or skip unsupported external candidates. Dirty tracked files and unpushed branches produce reasons (`:197-215`); nonrepository cwd is permitted (`:171-173`).

This matches Task 13's specified temporal/path predicate, not proof of exclusive worktree ownership: another lane's newly created external worktree can be included, and `git log --branches --not --remotes` is repository-wide. These are plan scope bounds, not new blocker findings. Task 13 performs no worktree removal or automatic close; `verified_done_ready` explicitly leaves close lifecycle to Task 15 (`src/lanes/runner.rs:459-464`).

### Shutdown paths that do have evidence

`src/lanes/runner.rs:398-410,431-455` correctly preserves a settlement error through cancellation and explicit draining. Lane turns receive a child cancellation token and are awaited through their turn JoinSet (`:617-643`); backend cooperation is required for graceful completion. The added test uses pre-subscribed channels and semaphores, not sleeps, and proves normal stop and settlement error wait for backend completion before returning (`tests/test_lane_done.rs:588-677`). It does not establish the uncovered service-error paths described above or real backend/process teardown.

## Evidence attribution, not rerun

Read `.omo/evidence/dori-grade-agent-env/task13-applied-source-settled.md` in full. That report attributes one focused offline all-features run of `test_lane_done`, `test_lane_reopen` and `test_lane_runner` to monitor `mon_7BD7ARGDM8S190X0` / session `bash_3`, exit 0:

- done10: 10 passed, 0 failed, including same-timestamp success/failure fencing and joined normal/settlement-error shutdown.
- reopen5: 5 passed, 0 failed.
- runner12: 12 passed, 0 failed, 1 pre-existing ignored live process test.

Total: 27 passed, 0 failed, 1 ignored. The same report records scoped rustfmt exit 0, zero LSP diagnostics on runner/cli/settle/done tests, and frozen six-file SHA-256 identities during that gate. These are the implementation track's reported results, not independently rerun tests, reproduced raw logs, or fresh hash verification by this reviewer. No commands, Cargo, DB queries, model turns, network, services or delegation were used in this review. The only repository edit is this new report.

## Exact pending QA before final confirmation

1. Correct the error-exit cleanup blocker and obtain the deterministic regression described above, then obtain evidence for the corrected source identities. Existing done10/reopen5/runner12 evidence belongs to the currently reported source set and cannot certify later edits automatically.
2. Receive the separate strict gate's exit codes and output for strict clippy, required formatting checks and standalone/build validation on the settled source. Strict clippy/build are pending here; none are inferred from test-profile compilation or LSP diagnostics. Attribute the relevant repository-required broader suite outcome or explicitly retain it as unverified.
3. Complete Task 13's happy live scheduling scenario in an explicitly disposable scratch environment: a real lane with `file done.txt` creates the file and claims; record persisted claim identity/time, `done_claimed` event and guidance, absence of new turns during the window, and `verified_done` no earlier than claim plus five minutes. Preserve `task-13.log` and authoritative persisted readback; notification delivery alone is not proof.
4. Complete the failure live scenario: claim without the file, wait for actual settlement, record `not_done` with the file-signal detail, event and queued guidance, and `lane log`/transcript evidence that the resumed agent actually receives the not-done message. Verify the failed claim does not replay its checks on a repeated scheduling pass.
5. Obtain live restart evidence before reservation and after a persisted reservation: before reservation retains the original deadline and settles once; after interruption does not replay commands, produces one recovery not-done event/message, and a new claim can settle. Subscribe to exact events before triggering and first inspect persisted state so a missed observer event is not misreported as failure.
6. Add or obtain deterministic objection-only in-flight and boundary commit-order coverage: objection wins before result CAS without stale artifacts; result commits first and later object is explicitly rejected; 299/300-second eligibility remains intact. Existing same-timestamp reclaim coverage is valuable but not identical to both cases.
7. Obtain focused worktree-selection evidence for cwd, descendant, external older/newer worktrees and unsupported creation-time evidence. Current done10 dirty/unpushed tests do not execute the settle module's selection tests. Verify explicit cwd behavior with a gateway process cwd different from the lane cwd. No owner worktree cleanup is authorized or required by this review.
8. Obtain real cancellation/teardown evidence for an active backend and checker during shutdown/error, including no remaining checker subprocess and completed lane persistence before service return. Do not treat fake backend cooperation as proof of real app-server cancellation.

Read scope: full Task 13 from main `.omo/plans/dori-grade-agent-env.md:301-311`, current complete `src/lanes/settle.rs`, `runner.rs`, `cli.rs`, `tests/test_lane_done.rs`, and the applied-source settlement report; supplemental transition rules and the signal-runner trait were read to resolve the race and once-only contracts. No final-confirmed claim is made.

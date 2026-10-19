# Todo 6 final gate review - frozen Wave 1 Attempt 4

recommendation: APPROVE
verdict: confirmed
task: todo 6
blockers: []

## Original intent and user outcome

originalIntent: Protect the owner's host with memory/disk/concurrent-lane admission controls, transition-only alerts and exclusive heavy slots without blocking Tokio.

desiredOutcome: Correct host sampling and launch policy, stale-slot recovery, cancellation-safe lock ownership, bounded metric commands with child cleanup, and trustworthy deterministic verification.

userOutcomeReview: Todo6 satisfies its scoped requirements. The earlier child-lifecycle and test-synchronization defects were corrected and their tests pass in the frozen integrated run. Integrated clippy and dashboard gates are proven; the full suite completed with the same 16 baseline failure names, not a green suite. The existing fmt difference remains explicitly accounted for. No specific outstanding todo6 criterion failure remains, so this review confirms todo6 under the plan's baseline-aware gate policy.

## Frozen integrated evidence independently inspected

Prefix for all Attempt 4 artifacts below: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave1-integrated-attempt4-.

- raw.log: read the actual command/exit header, searched every target result and failing-test line, inspected the final six-target failure summary and actual cargo sentinel. All 29 full-run target entries completed. The separate dashboard invocation is additional to those targets.
- gates.log: checked against raw output, not accepted by itself.
- comparison.md: checked its 16 failure names against the actual raw failure lines and the baseline task-2-failing.txt.
- pre-manifest.sha256 and post-manifest.sha256: independently compared with diff. All per-file hash rows match; only the descriptive first comment says pre-run versus post-run. Therefore whole manifest files are NOT byte-for-byte identical, but their 145 file/hash entries are identical. This is a wording correction, not source drift.
- build-provenance.md: inspected toolchain/input metadata, target/jobs, binary identity and cleanup provenance.

Independently computed hashes match the recorded artifacts:
- raw.log: 7b16de5848ad897bba196a8a9efa8e543ff586c07ddc9b02ec6ea2a12953bdc5.
- Cargo.toml: 17486e1195bc5a1b534e02845a58e7cad5e292abd9e784e18b3572d5f3d3d16e.
- Cargo.lock: 6129c4128398f0bbe50d9b2b69d725648fae5daa76a2915db2d6784fc3cfcba3.
- Current src/lanes/guard.rs: 2944cf4e3493d5f3c6304263359d9fda18eb21a7c97fbaa48a48050673ec0075, matching both manifest rows.
- Backend executable target-w4/debug/deps/test_omo_backend-a074f7c420765738: ab18769df5a57c36b4533677b184b75d802ef1103d793c843606c78ec6a437ae, matching recorded provenance.

Exact observed gates:
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0, raw.log:11-13.
- `cargo test --test test_review_dashboard`: 59 passed, exit 0, raw.log:14-80.
- `cargo fmt --check`: exit 1; only the documented pre-existing src/agent/backend.rs:61 blank-line diff, raw.log:3-10.
- `cargo test --all-targets --all-features --no-fail-fast`: actual cargo exit 101, raw.log:1249-1250; six failed targets. No claim of whole-suite success.
- Library: 437 passed, 0 failed, 5 ignored. Every guard behavior test passes at raw.log:285-301, including acquired-owner cancellation and native-child timeout cleanup. The three guard ignored entries are the child helper and two separately exercised live probes.

The 16 failing names match /Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env/.omo/evidence/dori-grade-agent-env/task-2-failing.txt exactly: one cron-boundary, one cron-executor, one Discord adapter, two migration, ten backend and one wiring test. Name identity does not prove causal identity. The wiring panic is explicitly different: Attempt 4 tests/test_wiring_e2e.rs:302 says 'Must emit stream chunks to dispatcher'; baseline is line 304, final_chunk.is_final. The root reports Reviewer 3 assessed an existing fixture race. This review does not independently reproduce that causal diagnosis or label the panic identical. No guard regression is demonstrated by this same-name baseline failure; it remains disclosed rather than blocking todo6 on an unsupported causal assumption.

## Scope, manual QA and adversarial proofs

Implementation was inspected directly in prior review rounds; final current guard hash matches the frozen evidence. The named final tests and manual receipts were rechecked in task-6.log section 7 and integrated raw output.

- Admission: memory and disk below minima and active lanes >= cap produce Hold. CPU is absent from Hold. Integrated injected tests pass for memory 10%, disk 10 GB, 7 lanes and load 300 alone.
- SQL: production count uses exactly working, blocked, not_done and done_claimed. Correct by code inspection. FakeActiveLaneCounter tests do not prove SQL integration; live active_lanes=0 is explicitly a fixture, not a measured DB count.
- Alerts: transition-only state machine with load hysteresis and memory/disk recovery; corresponding integrated tests pass.
- stale_state: actual filesystem flock contention and stale PID replacement after exclusive acquisition; integrated test passes. Lock inode is retained rather than unlinked.
- cancel/resume: oneshot is created before owner spawn; owner signals only after lock acquisition. Parent proves contention, aborts and awaits the owner, then directly acquires once. Guard remains live across pending; no pre-acquisition timeout or polling reacquisition can create the old false positive. Integrated test passes.
- hung commands: commands retain Child ownership, use kill_on_drop, and timeout explicitly calls start_kill then awaits child.wait. Native current_exe helper emits/flushed READY; parent awaits stdout lines under a bound before applying the deadline under test. PID disappearance is asserted afterward. Integrated test passes.
- Determinism/no shell: no shell string or readiness file polling remains in the corrected probe. The 50 ms duration is the timeout behavior under test, not a synchronization delay. READY is a machine-consumed sentinel, not prose pinning.
- Helper safety: child-only OMON_GUARD_TEST_HELPER=1 is set on Command; helper returns immediately without it. task-6.log records direct unmarked ignored-helper test exit 0 in 0.00 seconds. Broad ignored invocations therefore do not hang on this helper by default.
- BufReader handoff: into_inner can discard irrelevant buffered harness bytes; this test asserts neither subsequent output nor output equality, so the timeout/termination proof is unaffected. No extra extraction or buffering fix requested.
- malformed input: integrated empty/truncated/nonnumeric parser rejection tests pass.
- Runtime blocking: filesystem lock operations occur in spawn_blocking; load waiting uses async commands and Tokio sleep. No runtime-blocking load loop found.
- Manual live sample: task-6.log records exit 0, load 48.81, memory 43%, disk 212 GB, fixture active count 0.
- Manual disk failure scenario: OMON_GUARD_DISK_FREE_MIN_GB=100000 yields Hold with disk 212 GB, exit 0.
- misleading_success_output: old zero-timeout, acquisition-before-timeout and masked-clippy claims are retracted. The formerly failing exact clippy receipt in task-6.log is historical and superseded for integration by Attempt 4 raw exit 0, not silently rewritten.
- dirty_worktree: sibling changes were preserved; no assertion that aggregate Wave 1 changes belong to worker6. Matching final guard hash anchors this scoped review.
- Cleanup: task-6.log reports all target-w6 test invocations exited and tempfile cleanup; Attempt 4 gates/provenance report cargo exited 101 and no owned test process remained, distinguishing an unrelated FerryX cargo process. These are inspected historical cleanup receipts, not a new process census. No reviewer service stops, Discord actions, product edits, commits or checkbox changes occurred.

## Direct programming and overfit/slop review

remove-ai-slops and programming are not exposed in available skills; applied their criteria supplied in the task directly. No deletion-only, removal-only, prose-pinning or tautological regression tests were added. The corrected cancellation and timeout tests exercise real ownership, contention and process termination rather than mirroring implementation. Shared process drain/deadline handling is justified by three metric callers. Native helper is narrowly scoped and guarded. No unnecessary parsing/normalization or extraction finding remains that warrants changes. The fake-counter getter/setter test is low-value maintenance feedback only, not a criterion failure.

No separate upstream todo6 skill-perspective report or notepad path was supplied. This direct review supplies the requested check but does not claim that missing upstream coverage exists. Absence is recorded as an evidence limitation, not an invented product acceptance criterion.

## Checked paths and unverified limits

Checked in this final pass: all six named Attempt 4 artifacts, task-6.log:65 onward, baseline task-2-failing.txt, current guard/input/backend/raw hashes and matching guard manifest rows.

Previously read: /Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md (todo6 and gates), /Users/indo/code/project/omon-gateway/.omo/ulw-execute/review-brief.md, guard.rs and its corrected helper/cancellation/locking sections, lanes/mod.rs, relevant lib.rs/Cargo.toml/Cargo.lock exports/dependencies, prior task-6 logs and installed Tokio process/timeout documentation.

Limits: no broad build was repeated; final run and manual execution receipts were inspected rather than re-executed. The final raw log independently corroborates focused guard behavior on the frozen source. No live database active-count measurement was claimed. Baseline failure causal equivalence, particularly wiring, is not established by name equality. Full suite is exit 101 and fmt is exit 1 as disclosed, not green. These explained baseline results satisfy the plan's compare-to-baseline rule; no specific remaining todo6 failure is proven.

Artifact routing: earlier agentToolkit.status returned ULW_LOOP_PLAN_MISSING for this child; fallback evidence path retained. Only this report was written. Final recommendation: APPROVE; AdversarialVerify verdict: confirmed for todo6.

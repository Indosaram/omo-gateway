# Todo 3 final integrated audit

recommendation: REJECT pending current-source provenance bridge
AdversarialVerify: needs-fix
scopeStatus: scope-ready; attempt-3 backend behavior verified

## Original intent / desired outcome

originalIntent: Run lane sessions with their own cwd, configurable budget, persistent binding, and no interactive typing/fail-fast/steering, while preserving Discord/cron/web behavior.
desiredOutcome: Verify those behaviors and live cwd use on the shipped source, compare existing tests with todo2, and finish required broad gates with established baseline exceptions.
userOutcomeReview: Attempt 3 now proves the complete backend target's expected outcomes. The earlier missing-nine-results blocker is resolved. The remaining concrete gap is tying that tested executable to the current dirty source, not test coverage or a demonstrated todo3 defect. A targeted provenance bridge can avoid another slow suite.

## Artifact audit

Roots:
M = /Users/indo/code/project/omon-gateway
W = /Users/indo/code/project/omon-gateway-wt/dori-work
R = /Users/indo/code/project/omon-gateway-wt/dori-grade-agent-env
E = W/.omo/evidence/dori-grade-agent-env

Read current E/wave1-integrated-attempt3-comparison.md, E/wave1-integrated-attempt3-gates.log, and raw backend/recovery/wiring sections of E/wave1-integrated-attempt3-raw.log. Read exact baseline R/.omo/evidence/dori-grade-agent-env/task-2-baseline-full.log:809-874 again. Earlier audits read plan todo3/common review brief, source/test diff, task-3.log, executor raw live receipts, and independently executed paired silence tests (2 passed, 0 failed, 0.14s). No full suite was launched in this audit.

The raw backend recovery is at lines 898-1026, executable `target-w4/debug/deps/test_omo_backend-a074f7c420765738`, not target-w33 or target-w6. It ran all 47 tests: 36 passed, 10 failed, 1 ignored in 1801.14s. All original 31 passing cases appear as passes, along with four lane cases and the new Discord silence counterpart. The live test is ignored in this invocation; its separate actual direct-binary 18.52s pass remains the live receipt. It must not be relabeled a successful Cargo invocation; the original Cargo command timed out.

All ten backend failures match baseline panic sites and failure messages. The deadline-looping duration is 10.260226166s versus baseline 10.137300542s; this is the same deadline assertion, not byte-identical prose. The other nine are the same no-content, chunk-layout, missing-terminal, approval inactivity, or model-update-timeout assertions at the same lines. Therefore the nine previously unverified current outcomes are now verified for the attempt-3 executable. There is no backend test regression demonstrated by this completed run.

Recovery ends with `recovery_cargo_exit=101` and `SENTINEL_ATTEMPT3_RECOVERY_DONE exit=101` at raw lines 1216-1217. Initial invocation has no final exit; interrupted-plus-recovery must remain the description. Gates report strict broad clippy exit0, formatting only the established backend.rs:61 baseline difference, and dashboard59 passes. The supplied 29-target coverage accounting is not substituted for the directly inspected backend output.

## Changed wiring panic: concrete assessment

Read W/tests/test_wiring_e2e.rs:105-195,265-307 and the matching R file:265-307. The unchanged fixture waits only until `received_developer_instructions` becomes Some. The fake sets that field in `thread/start`, before returning the thread result and before receiving `turn/start`; delta and terminal messages are sent later in the separate turn/start arm. The assertion then immediately samples dispatcher chunks. There is no completion synchronization. Thus both an empty vector (current line302) and a nonfinal last chunk (baseline line304) are permitted by this existing race.

Attempt3 raw:1187-1190 fails at line302 `Must emit stream chunks to dispatcher`; baseline fails at line304 `final_chunk.is_final`. They are not exact panic matches. The source provides a specific existing race explaining the earlier assertion; this difference alone is not evidence of a new todo3 regression and is NOT a separate todo3 blocker. No flaky test was modified or repeatedly rerun to force green. It remains an explicitly documented baseline test defect, not a claimed successful wiring behavior.

## Current executable/source provenance

Read-only current hash inspection:
- target-w4 backend executable SHA256: ab18769df5a57c36b4533677b184b75d802ef1103d793c843606c78ec6a437ae
- target-w3-task3 backend executable SHA256: 99e1c3b9fccfdef21eb41c3ccf4d77d177e5517cd5caf54e17dbae279ae8e0e0
- Cargo.toml: 17486e1195bc5a1b534e02845a58e7cad5e292abd9e784e18b3572d5f3d3d16e
- Cargo.lock: 6129c4128398f0bbe50d9b2b69d725648fae5daa76a2915db2d6784fc3cfcba3
- src/agent/omo_backend.rs: ad57ecf76448d7f4b406b49bcd6ecdd8b476b6ccee4c2406c72b08abd553e874
- src/agent/omo_config.rs: cdcd39fd6fc452559186338f4b83712fd59bdf83c6deb5a6b58f03f3538662e8
- tests/test_omo_backend.rs: 9c9fe74dfa23926e607688b4a554880957dd7f58fcf6feeb95f69d6bf36ca8ce

Current target-w4 binary mtime is 2026-10-11T11:56:47+0900; the three source mtimes precede it. This supports plausibility but is not a content-to-executable certificate. Different target-directory binaries need not have identical hashes; no regression is inferred from that difference.

The supplied pre digest077f... has no input manifest or exact command. The reproducible post digest4c13... excludes Cargo.toml/Cargo.lock and build configuration. Neither unchanged HEAD nor those aggregate digests establishes which current dirty inputs produced the tested binary. Do not assert source mutation, source stability, or binary equality from them. The raw log supplies an executable path but no contemporaneous binary hash. Current files alone therefore cannot fully certify current-source identity retroactively.

## One remaining blocker and minimal coordinated resolution

blockers:
- violatedCriterion: T3-AC-PARITY / REVIEW-BRIEF-1 / PLAN-GATES evidence applies to the delivered artifact.
  evidencePointer: E/wave1-integrated-attempt3-comparison.md, Fingerprint provenance; E/wave1-integrated-attempt3-raw.log:898; current executable/source hashes above.
  observation: Completed backend outcomes are proven, but no saved pre manifest or build/executable identity bridge binds them to current source. This is now a narrow provenance gap, not a request for a blanket full-suite rerun.

Minimal next gate, coordinated by the parent with sources frozen:
1. Record the exact commands/environment and a per-file manifest for src/tests/migrations, Cargo.toml, Cargo.lock, any build.rs/.cargo config, plus compiler identity and relevant RUSTFLAGS/features. Record the target-w4 test executable hash before any build. Preserve the existing raw attempt3 evidence.
2. Use the SAME warmed target-w4 and attempt3 features: `CARGO_TARGET_DIR=target-w4 CARGO_BUILD_JOBS=2 cargo test --test test_omo_backend --all-features --no-run -vv`. Capture whether Cargo considers the target fresh, artifact path, exit, and resulting executable hash; verify the source manifest did not change. This is a build-only bridge, not another 30-minute test run.
3. If Cargo reports Fresh and executable bytes remain identical, that provides operational current-source/build provenance for the retained tested artifact (with the disclosed lack of historical binary hash). Run only the five current non-live acceptance tests (four lane tests plus Discord counterpart) via that artifact to provide an immediate current frozen receipt. Retain the already completed ten baseline panic results and31pass inventory; no duplicate slow failures are needed. Reconfirm broad formatting/clippy only if the bridge or changed configuration invalidates their existing receipts.
4. If Cargo recompiles/relinks the backend or binary bytes change, the old ten-failure comparison cannot simply be transferred. Parent coordinates ONE frozen `cargo test --test test_omo_backend --all-features` run with captured command, binary identity, all results, exit, and matching pre/post manifests. This is the minimum slow target needed for todo3, not another29-target run. Other targets need reruns only if the identified input change affects them.

For a stricter requirement of contemporaneous historical binary hashes, Fresh alone cannot recreate one: take option4 once under the frozen manifest. Do not repeat the old full suite merely to compensate for a missing hash procedure.

The live acceptance receipt remains valid functional evidence for the reviewed lane implementation. A fresh live rerun is needed only if identified rebuilt changes affect that path; do not spawn/stop a daemon merely for provenance paperwork. Existing cleanup was independently verified in the earlier audit: task-owned4966 absent, TCP19742 no listener, readyz connection refused. No current endpoint state is asserted beyond that prior check.

## Skill-perspective / maintenance review

remove-ai-slops and programming skills were unavailable; prompt criteria were directly applied. The paired fake-server tests exercise public backend behavior, not removal-only, prose-pinned, tautological, or implementation-mirroring assertions. The unchanged wiring fixture has a concrete nondeterministic wait defect described above; do not edit it green in this task. Cwd validation duplication and the1800-second timeout sentinel remain maintenance notes, not blockers. No unnecessary production extraction was added during counterpart remediation. No separate prior todo3 code-review/notepad artifact was supplied; direct skill coverage is recorded here rather than invented.

## Scope of this review

Current paired test implementation and baseline source race were read; current executable/source hashes were measured. Product files were not edited. Only this gate report was rewritten. No service action, external message, product build, or slow duplicate run was performed. No toolkit eval/currentAttemptDir facility was available; report stays at the prescribed fallback path. Prior reviewer async handles without completion receipts are not claimed as successes.

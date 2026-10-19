# Todo 13 independent FINAL gate

Date: 2026-10-11. Verdict: **CONFIRMED**.

All five acceptance criteria and both required QA scenarios in the main plan at `.omo/plans/dori-grade-agent-env.md:301-310` are met for the settled implementation in `/Users/indo/code/project/omon-gateway-wt/dori-work`. This reviewer did not implement the changes. No remaining blocker was found within todo 13's specified acceptance scope.

## Independently checked source identity

The reviewed workspace is dori-work, the workspace named by the release manifest and all supplied implementation evidence. Files were read and SHA-256 computed in memory, without commands or external processes. The current identities match `wave4-strict-gate2-post.sha256`:

| File | Current SHA-256, equal to manifest |
| --- | --- |
| `src/lanes/settle.rs` | `8bd0c6362fcae5d77daf9c3869558afa410c0a92d6e459af8dd42932e73972c5` |
| `src/lanes/runner.rs` | `0eb9920fc25bb657aab160fc4ecde10950ba425c7f6241373a32cc3250ec97cd` |
| `src/lanes/cli.rs` | `3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510` |
| `tests/test_lane_done.rs` | `c696d24721e25030108937acf3551ce335445e416f19d9ade05775f8ae1abda9` |

The manifest itself independently hashes to `60a3af1024eb2eae6975aae5a2e542c3ecad762bc918183c5e7b92f75e005517` and contains 212 entries, matching `wave4-strict-gate2-release.json`. This checks the requested source identities, not every current file or the binary bytes. The release receipt attributes binary `71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86`, formatting exit 0, strict all-target/all-feature offline clippy exit 0, build exit 0, and source comparison 0 to this release.

For scope clarity, the main workspace `/Users/indo/code/project/omon-gateway` is not this release tree. Its independently calculated hashes are settle `5ddbfa3e7fcab751e88eadbcc4de2b472382367ac2bc54bd1116a8cf0ee6e416`, runner `9c2b6225835b64178c93dcc89c8422a01a92628a70390d944ee1794b151fafec`, and CLI `1ee99bfe3b3dc5ee867a693b2c7265bd32f57280a8255d600fd785c6cd4e238c`; none match the dori-work manifest. CONFIRMED does not certify the main tree or claim that integration back to it occurred.

## Acceptance matrix

The current tests were inspected, not rerun. `wave4-integration-source-settled.md` records one combined 13-target run, exit 0, with 117 passed, 0 failed and 4 existing ignored. That includes all 11 `test_lane_done` tests, 12 runner tests and 5 reopen tests. The current done-test hash equals the repair and release identities, and the runner/CLI/settler hashes equal the integration gate inputs. The later strict-gate boolean-equivalent rewrites concern inbox triage and kibitzer, not these task 13 files.

| Required criterion | Evidence and assessment | Result |
| --- | --- | --- |
| Claim then object returns not_done and queues reasons | `tests/test_lane_done.rs:194-224` asserts not_done, stored objection, queued reason and zero checks. `src/lanes/cli.rs:435-530` locks and atomically persists claim/object state, history, event and guidance. | PASS |
| Passing signals and clean tree verify after five minutes | `tests/test_lane_done.rs:227-246` uses MockClock: 299 seconds remains done_claimed with zero checks; at 300 seconds verified_done with one check. Eligibility and reservation are in `src/lanes/settle.rs:35-79`. | PASS |
| Failing signal returns not_done with detail | `tests/test_lane_done.rs:249-265` checks status, objection and event detail, with one signal invocation. `src/lanes/settle.rs:80-124` retains failing signal name/detail and queues not-done guidance transactionally. | PASS |
| Dirty tree includes uncommitted | `tests/test_lane_done.rs:268-284` modifies a tracked file and asserts not_done with uncommitted. Settler checks explicit lane/worktree paths using tracked-only porcelain status and checks unpushed branches. | PASS |
| Runner restart between claim and settle settles once | `tests/test_lane_done.rs:287-314` reconstructs the runner using persisted claim state, honors the original deadline and asserts one checker invocation and one verified_done history across repeated calls and another reconstructed runner. | PASS |

The restart criterion explicitly calls for tests with fakes; it does not require a separate live restart scenario. Interrupted reservation recovery at `tests/test_lane_done.rs:317-358` additionally proves no signal replay, one recovery event, and successful fresh claim. The guarantee is at-most-once check initiation per persisted claim identity and one committed result, not crash-proof exactly-once completion of arbitrary external commands. Same-timestamp object/reclaim fencing is also covered by the passing done target.

Runner admission only starts working/not_done lanes; `run_lane` exits before another turn when status is done_claimed (`src/lanes/runner.rs:546`). Claim guidance is queued with its KST deadline and can be steered into an active turn. Settlement runs on the 30-second interval with one active settlement task, preserves the five-minute lower bound and never closes solely on a lane self-report. Actual close lifecycle remains todo 15's scope.

## Required live QA

Read `task13-actual-d162f780-pass.md` and the preserved `/private/tmp/omon-task13-live-KT2CJl/task13.json` command/state/transcript artifact. These are historical readbacks, not new DB queries. The live receipt independently hashes to `533ba5cf59b0cac59ba72e8e562ab6882636a690cf1fa53d0216702bd1f0733c`, equal to the repair reviewer identity. The historical live source manifest independently hashes to `6714ccfa445bd656ade0c9f23746152c6727004867463b712c83fe645e3b5243`.

**Happy: PASS.** The real agent created and verified `happy/done.txt`, then invoked the real claim CLI. The harness command ledger contains lane start and cleanup, not a synthetic claim. Claim history ID 2 is at `19:19:07.624316Z`; reservation ID 5 occurs once; result history ID 6 is verified_done at `19:24:09.721358Z`, 302.097042 seconds later. The pre-cleanup row is verified_done, with one completed agent turn, no objection, and fileExists true. Recorded tracked-only git status is clean. Cleanup's later closed row does not substitute for the verified_done result.

**Failure: PASS.** The real agent claimed without creating `failure/done.txt`. Claim ID 1 is at `19:18:54.325081Z`; reservation ID 3 occurs once; history ID 4 records not_done at `19:24:09.721358Z`, 315.396277 seconds later. The diagnostic is `file done.txt: failed to read file '/private/tmp/omon-task13-live-KT2CJl/failure/done.txt': No such file or directory (os error 2)`. Inbox ID 3 queues the full `[LEAD] not done` message. User transcript ID 19 contains that exact message; agent transcript ID 21 echoes `TASK13_NOT_DONE_RECEIVED` and the diagnostic; outcome ID 22 completes. The later not_done -> working -> blocked sequence proves resumed delivery rather than loss of the not_done result. Null consumed_at is not an acknowledgment; transcript and agent receipt establish the requested delivery. The file remains absent, with no second claim. The artifact records pass true, exitCode 0, gatewayExit 0, pidAbsent true and socketAbsent true.

## Applicability of the earlier live binary

The live scenarios ran on `d162f7808bf8e9c10699e3903a9bc494e7041af3c2b785a985ccb13fb56f1e0f`, not release binary `71c37a90...`. I accept them as evidence for the two task 13 scenarios, not as execution certification of the later binary.

The historical live and release manifests contain identical settler, CLI, signal runner, transition rules, scrubber, OMO backend and multiplexer actor identities. Thus claim persistence, deadline, signal checking, result CAS, failure detail and agent delivery mechanism exercised by live QA remain unchanged. The runner repair routes service-loop errors through shared cancellation and explicit joins; it does not change successful claim/settle/resume semantics or `run_lane`/CAPTURE_GENERATION. The current source retains the awaited `service_result` boundary at runner line 386 and cleanup starting at line 462, so the earlier error-exit bypass does not reappear.

Wave 4 runner additions are optional kibitzer wake hints, guard sampling/queued admission, and status-aware post drain/refresh; main wiring concerns triage progress. These are real changes, not an assertion that the complete runner is byte-identical. They leave task 13 settlement and turn fences intact, keep fallible service work inside shared cleanup, and are covered with the repaired runner by the subsequent 117-test integration gate and strict build receipt. Consequently the live evidence remains applicable to the requested happy/failure behavior when combined with current-source and integration evidence. It does not prove live error-path teardown or every optional production integration on the release binary.

## Earlier blocker and residual limits

`task13-applied-independent-gate.md` identified a service-error cleanup bypass. `task13-runner-repair-source-settled.md` records the corrected boundary and final strengthened regression pass. `task13-runner-repair-gate.md` independently approves that source repair. The integrated done target subsequently passes all 11 tests, including normal/settlement-error cleanup and the five injected service-error classes. The original blocker is resolved, not waived.

The earlier source review's additional live restart, objection boundary-order, exhaustive external-worktree selection and real-backend active error-teardown requests exceed the five stated fake-test acceptance rows and two live QA scenarios. They remain useful residual coverage limits, but their absence is not a blocker for this exact requested final gate. No whole-suite green, live owner objection race, OS socket-error injection, new-binary live execution, deployment, or main-workspace integration is claimed. Fresh LSP diagnostics were unavailable in the later tracks; strict compiler/clippy results are recorded separately. The reported 8 historical backend failures and 2 timeout-unverified tests remain outside task 13 and are not presented as fixed.

All seven supplied evidence reports, current dori-work settlement/runner/CLI/done-test source, both manifests, and preserved live JSON were inspected. Recorded test/build outcomes are attributed to their supplied receipts, not independently rerun. No commands, Cargo, DB access, models, services, environment edits, git actions or delegation occurred. The only file created by this review is this requested report.

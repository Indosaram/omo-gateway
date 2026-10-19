# Task 13 runner repair independent source gate

Date: 2026-10-11. Independent reviewer; this reviewer did not implement this repair. Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Verdict

**APPROVE (source scope).** The service-error cleanup blocker in `task13-applied-independent-gate.md` is resolved on the source identities verified below. No remaining blocker was found within this runner repair review. This is not final Task 13 confirmation, standalone build approval, runtime teardown proof, or deployment approval. Strict clippy/build on this exact repair source remain pending in a separate gate.

## Independently verified identities

Files were read and SHA-256 computed in memory, without shell commands or external processes. All identities match `task13-runner-repair-source-settled.md`:

| File | SHA-256 |
| --- | --- |
| `src/lanes/runner.rs` | `d55af2c5e94a1580396380baab9119ba90cf7a721a29b69b60d8c4357ca7c615` |
| `tests/test_lane_done.rs` | `c696d24721e25030108937acf3551ce335445e416f19d9ade05775f8ae1abda9` |
| `src/lanes/settle.rs` | `8bd0c6362fcae5d77daf9c3869558afa410c0a92d6e459af8dd42932e73972c5` |
| `src/agent/omo_backend.rs` | `04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2` |
| `src/multiplexer/actor.rs` | `1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3` |
| `src/main.rs` | `c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a` |
| `task13-actual-d162f780-pass.md` in this evidence directory | `533ba5cf59b0cac59ba72e8e562ab6882636a690cf1fa53d0216702bd1f0733c` |

The settler, backend, actor and main identities also match the earlier `task13-applied-source-settled.md` baseline. Supplemental verification of `src/lanes/cli.rs` gives `3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510`, also unchanged from that baseline. Runner/test identities are the new repair identities, not the earlier applied identities.

## Service error control flow

`src/lanes/runner.rs:374-430` encloses the running service loop in an awaited async `Result<()>` block. Both `?` and explicit `return Err(...)` inside that block resolve the block's result, not the enclosing `serve` function. The six previously uncovered paths are all inside that boundary:

| Initiating error | Current reference |
| --- | --- |
| Working/not-done lane listing | `src/lanes/runner.rs:379` |
| Reopen generation lookup before lane launch | `src/lanes/runner.rs:385` |
| Running-service post drain | `src/lanes/runner.rs:412-413` |
| Socket receive | `src/lanes/runner.rs:415-416` |
| Lane JoinError conversion | `src/lanes/runner.rs:419-420` |
| Failure persistence after a lane outcome error | `src/lanes/runner.rs:422-424` |

Settlement result errors and settlement JoinErrors likewise return from the inner block (`src/lanes/runner.rs:398-409`). Normal stop breaks the loop and yields `Ok(())` (`:376-377`, `:393`, `:429`). There is no fallible outer early return between completion of this block and shared cleanup.

The common sequence is exactly `service_result.err()` into `first_error` (`:431`), cancellation delivery (`:432`), settlement abort (`:433`) and explicit settlement drain (`:434-444`), then cooperative lane task joining (`:445-453`), followed by the final return (`:454-457`). Cleanup errors use `get_or_insert`/`get_or_insert_with`; they cannot replace an already captured initiating error (`:437`, `:440`, `:449`). Lane outcome errors during cleanup are logged, not substituted (`:447`). Thus the originating service error is returned only after both JoinSets have been drained. This guarantees the runner joins, not successful persistence in the presence of an independent storage failure or termination of an uncooperative backend.

Startup binding, post recovery and initial post draining occur at `src/lanes/runner.rs:362-365`, before either runner JoinSet is constructed (`:368-369`) or any lane/settlement task is spawned (`:386`, `:396`). A startup failure after binding drops the owned WakeSocket; its Drop removes the pathname and field destruction releases the descriptor/lock (`:220-224`, `:259-264`). One precision correction to the receipt's phrase "any task spawned": binding itself uses a `spawn_blocking` lock helper (`:230-244`) and awaits its result before continuing. The sound claim is that no lane/settlement task exists at a startup error; the awaited startup helper does not leave an unjoined runner task. This wording qualification is not a cleanup blocker.

## Socket receive argument

The source-level argument is sound for this source gate. `recv`'s `Err` is mapped by `io_error` and propagated at `src/lanes/runner.rs:416` within the same inner async block as the injected service errors; it must therefore enter `first_error` and the identical cancellation/abort/drain/join sequence. `io_error` preserves the mapped lane-I/O diagnostic (`:89-91`).

WakeSocket and its descriptor are private (`src/lanes/runner.rs:220-224`) and the bound instance is locally owned by `serve` (`:362`). The public wake function sends through its own unbound descriptor (`:203-217`); it does not expose the receiver. Unlinking a Unix datagram pathname does not close the already open receiving descriptor. Closing or replacing that owned descriptor externally would violate ownership and is not an appropriate safe injection method. A production socket factory/fault hook is not necessary to establish the lexical error-routing invariant. Socket-receive runtime injection was not performed and is not claimed; OS error reproduction and real process teardown remain outside this approval.

## Regression assessment

`tests/test_lane_done.rs:642-813` iterates all five injected classes: list decoding, generation decoding, malformed pending post, failure persistence, and lane-task JoinError (`:644`). Channels and zero-permit semaphores are created before `serve` starts (`:655-683`). The test awaits both backend and due-checker entry before injecting faults (`:684-691`), so it cannot pass merely because no work was active. All waits are bounded event waits; there are no fixed sleeps or polling delays in this regression.

The injections have distinct initiating-error assertions: list expects `turns_run` (`:693-705`), posts expects `lane post JSON` (`:707-709`), generation expects `decoding column 0` (`:711-724`), lane JoinError expects the injected judge panic text (`:726-732`, judge at `:600-611`), and failure persistence expects the trigger's `injected failure persistence` text (`:734-741`). These expectations are checked on the returned error for every iteration (`:784-792`), not merely on any `Err`. In addition, the strengthened generation assertion matches **`OmonError::Database`** and requires both **`BLOB`** and **`i64`** (`:793-802`), distinguishing the original BLOB-to-i64 fault from a later cancellation or join error. The Database/BLOB/i64 assertion is specifically for the generation class; the other four use their distinct error diagnostics, not that same type assertion.

Every iteration awaits cancellation delivery and checker teardown (`tests/test_lane_done.rs:759-766`), checks the shared stop token and unfinished service task while backend completion remains gated (`:777-778`), releases backend completion and awaits its exact event (`:779-783`), then awaits the service return (`:784-788`). Held-lane `turns_run == 1` is read after return (`:804-810`), proving joined persistence rather than merely receipt of the backend's pre-persistence completion notification. The backend receives cancellation before its release gate (`:829-835`), and checker teardown is a Drop event (`:531-550`). These assertions would detect the prior bypass.

The generation fixture installs its BLOB-ID view before inserting the newly eligible lane (`tests/test_lane_done.rs:712-723`), avoiding launch-before-fault. It restores the history schema after cancellation/checker teardown but before releasing held backend completion (`:767-779`), allowing joined persistence to exercise the normal schema. The real lane and turn JoinSets, rather than a mocked join operation, produce the injected panic errors. The runner's held turn joins at `src/lanes/runner.rs:620-643`, commits its turn result at `:769-783`, and returns on shutdown at `:791-792`.

## Recorded results, not reruns

The repair receipt's final attempt is `mon_ZJK8GBXSVT9Q72CG` / `bash_6`: scoped rustfmt followed by the single all-features offline targeted test `service_storage_errors_cancel_and_join_backend_and_checker`, exit **0**, **1 passed, 0 failed, 10 filtered out**, test runtime **6.63s**, test-profile compilation **2m 22s**. The receipt attributes all five class iterations and the strengthened error-identity assertions to this final source/test set. The expected injected panic output was caught by the JoinSets. This reviewer did not rerun it or independently reproduce its raw execution logs.

The receipt retains the unsuccessful earlier attempts: initial multi-target and two targeted failures (exit 101), an intermediate targeted pass before the identity assertion, and a rustfmt-only failure before Cargo started. Those attempts are not presented as a clean final multi-target gate. The final targeted pass resolves the repaired regression; the earlier 27-test passing baseline does not automatically certify every test on the new repair source. LSP freshness timeouts in the repair receipt are not clean diagnostics. Test-profile compilation is not strict clippy or a standalone build.

## Retained approvals and live evidence

The unchanged settler and CLI preserve the earlier bounded claim-ID CAS, restart-reservation and five-minute-boundary source approval. Current references are authoritative locked claim reload and eligibility at `src/lanes/settle.rs:39-66`, reservation committed before checks at `:68-79`, and status/time/reopen/claim-ID result CAS with zero-row rollback before artifacts at `:101-124`. The guarantees remain at-most-once signal-check initiation per reserved identity and transaction-order objection semantics, not crash-proof exactly-once signal completion or objection priority. Single-runner ownership remains an operating boundary. Earlier residual live restart, objection-order, worktree-selection and real-backend teardown questions are not closed by this repair gate.

The byte-identical `task13-actual-d162f780-pass.md` remains valid evidence for its recorded binary `d162f7808bf8e9c10699e3903a9bc494e7041af3c2b785a985ccb13fb56f1e0f`: happy `verified_done` **302.097042s** after the real agent claim, and failure `not_done` **315.396277s** after the premature claim. Failure delivery is supported by user transcript ID 19 with the exact not-done inbox text, agent ID 21 echoing `TASK13_NOT_DONE_RECEIVED`, and completed outcome ID 22. The receipt records one reservation per claim, helper/gateway exit 0 and owned PID/socket absence. These results remain attributed to that historical live run; the repair does not invalidate them, but its changed runner means they are not new-binary runtime certification. No live restart or real owner-objection race is inferred from that happy/failure receipt.

## Scope and pending gate

Only this report was created. No product/test edits, commands, Cargo, database queries, model calls, services, network actions or delegation were used. All source/hash inspection and verdict reasoning were read-only. The separate strict clippy/build gate must establish its own results on the exact current source before final confirmation. This report closes only the previous runner service-error cleanup source blocker and retains the stated evidence limits.

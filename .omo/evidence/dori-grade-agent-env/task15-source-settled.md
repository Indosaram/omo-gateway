# Task 15 source and verification ledger

Workspace: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Authority: todo 15 of the main plan, `task15-integrated-ready.md` and
`task15-current-evidence-map.md`, with the explicit implementation release in this session.

Status: source settled and the four requested focused targets passed, with the
existing live runner test ignored. This is not a live-QA completion claim.

## Source boundary

Owned files changed: `src/lanes/runner.rs`, `src/lanes/cli.rs`,
`src/storage/lanes.rs`, `src/discord/adapter.rs` (interception only),
`migrations/0038_lane_lifecycle.sql`, `tests/test_lane_close.rs`, and
`tests/test_lane_cli.rs` (stale immediate-success expectations).
Migration 0038 was absent before this implementation.

Close requests persist a generation-bound intent rather than closing early. The
socket-owning lane service cancels the active lane token and joins its task before
status/archive/Git effects, suppresses new turns during close, and reconstructs
interrupted intents and already verified lanes on startup. The existing settlement
claim CAS, generation fences, conditional Discord startup, joined shutdown and wave4
guard/kibitzer wiring are retained. No done signals are rerun by close.

Explicit local `lane register-worktree KEY --repo REPO --path PATH` validates linked
Git registration, surviving control cwd, common repository, existing branch and
registration identity. Owned cwd removal is non-force, branch-preserving, and
observed before closed commit. Unowned external cwd gets a skipped receipt. Reopen
requires verified removal proof and the same preserved branch and path; occupied
paths, changed linkage, missing branch/proof and ambiguous restoration fail closed.

Ordinary owner-text interception follows the actor's exact first nonempty string
resolver and existing channel/user/role/pairing admission. Persisted bot binding is
required; legacy binding uses only durable thread-owner evidence. Receipt identity
is global across clients. Exact first text, reopen history and applied receipt commit
together. `/stop`, snapshots and attachments remain on their existing paths.
`close_failed` stays under the existing Jev policy.

## Commands and receipts

1. Scoped `rustfmt --edition 2021 --config skip_children=true` and corresponding
   `--check` on the six modified Rust files: exit 0.
2. Initial LSP calls timed out at the daemon. Later LSP calls returned no diagnostics
   for runner, CLI, storage, adapter, close tests and CLI tests.
3. Requested batch:
   `CARGO_TARGET_DIR=target-w15 cargo test -j2 --offline --test test_lane_close --test test_lane_runner --test test_lane_reopen --test test_lane_done --no-fail-fast`.
   Monitor `mon_7ABDC7KTVAK07DGQ`, bash `bash_5`, 1800000 ms. Timed out with exit 1
   during dependency compilation; no test target ran and no pass is attributed.
   Process inspection confirmed this Cargo run had ended and no target-w15 test
   binary existed. Other owners' Cargo/sccache processes were left alone.
4. Failed-build continuation, same target and four unexecuted targets, with
   `RUSTC_WRAPPER=` to bypass the stalled sccache wrapper (command-scoped only;
   no env file or service changed): monitor `mon_257YYP822EZ8FQJH`, bash `bash_10`,
   1800000 ms. Compilation completed in 5m 20s; exit 101. Close: 26 passed/1 failed;
   done: 9 passed/2 failed; reopen: 5 passed; runner: 11 passed/1 failed/1 ignored.
   The initial cancellation early return incorrectly skipped existing turn
   persistence and was removed. The dirty-close test exposed a failure-receipt
   transaction attempting to reacquire the repository's single pool connection;
   lane lookup was moved before that transaction. Tests were not weakened.
5. Failed targets only:
   `RUSTC_WRAPPER= CARGO_TARGET_DIR=target-w15 cargo test -j2 --offline --test test_lane_close --test test_lane_runner --test test_lane_done --no-fail-fast`.
   Monitor `mon_X8WSPPP7C2VNSXT9`, bash `bash_15`, 1800000 ms, exit 101.
   Done: 11 passed; runner: 12 passed/1 ignored. Close: 26 passed/1 failed,
   with `database error: pool timed out while waiting for an open connection`.
   This run compiled before the failure-receipt lookup repair was applied.
6. Remaining failed target only:
   `RUSTC_WRAPPER= CARGO_TARGET_DIR=target-w15 cargo test -j2 --offline --test test_lane_close`.
   Monitor `mon_6YQVQ3ABQTB7EKEH`, bash `bash_17`, 1800000 ms, exit 0.
   Compilation 8.70s; 27 passed/0 failed, 1.87s. This includes the unchanged
   dispatcher tests via their neighboring fixture plus the new reconstruction test.
7. Final scoped rustfmt check and adapter `git diff --check`: exit 0.
   Final runner LSP request timed out at the daemon; prior zero-diagnostic results
   do not certify that final edit. The final close compilation does certify its
   type correctness. CLI tests were updated but not run separately, per the
   requested four-target verification scope. No clippy or whole-suite pass claimed.

Final target ledger: close 27 passed; runner 12 passed/1 ignored; reopen 5 passed;
done 11 passed. Reopen was not rerun after passing. Done and runner passed with the
cancellation fix; the last runner change only moves the close-error lookup outside
its single-connection transaction and was covered by the passing close target.

## Source SHA-256 (settled)

```text
d6103b808d2cec3efb2b7dd060e7b30e9b929bc44b0b2d295754026aa2059480  src/lanes/runner.rs
7309ab19614069c8aebaf29445164e35f7dc97e5af93b799b570da05e41fa109  src/lanes/cli.rs
07267ea490985351a2e7cb334301ce0b34c48545d0e353a67bb13c78272aec85  src/storage/lanes.rs
2631a639ff3dc5f00e344188ade48584456af8683e5a168e67d6721c16c27068  src/discord/adapter.rs
0f0f9435d7300d87dada65c1a73ef65bd91226715513c168b11f0a7b9cd47a8a  migrations/0038_lane_lifecycle.sql
cf5753272818a1dab933af93fc1d74d37e7c5aee5b79422035fc288c52fba52c  tests/test_lane_close.rs
cf3caa9c5410fb18f20bc9e0fe62cdd2a0a81e8fbe65b110d63e9350af461811  tests/test_lane_cli.rs
5d39a341a4025ea9ba84dcb0e1fc3058c7c3f7980d8b916cec4fb6509bdd4ebc  tests/test_lane_runner.rs
b6b255f85049762a2a900c49cab708c6c2e14375dd1f65642da73afb6a25bb1a  tests/test_lane_reopen.rs
c696d24721e25030108937acf3551ce335445e416f19d9ade05775f8ae1abda9  tests/test_lane_done.rs
```

## Live-QA needs

No live Discord/API/UI, model, production database, service restart, deployment or
git commit was performed. Synthetic tests use owned temporary Git repositories,
scratch SQLite and fake backend/thread lifecycle transports. Tests include primary
owned cwd removal/recreation, dirty removal, original forced status, explicit skip,
replacement/occupied path denial, receipt-gap handling, duplicate text receipt and
dispatcher reconstruction. These synthetic cases passed; they do not substitute
for owner-confirmed live Discord state. The service ordering test drives a fake
backend cancellation signal and joins service shutdown before completing any
remaining persisted close intent; it is not a live daemon receipt.

Owner/lead still needs the plan's authorized live lane trace in `task-15.log`:
verified_done -> closed; actual linked cwd absent from Git worktree list and disk;
preserved branch; GET thread confirms Done/archive; ordinary owner reply restores
the same cwd/branch and session and produces one lane prompt with no chat turn.
The separate pre-settlement dirty fixture must show not_done/uncommitted without
close. This agent must not perform live Discord QA.

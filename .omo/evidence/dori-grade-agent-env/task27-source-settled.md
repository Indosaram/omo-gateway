# Task 27 resume receipt

Date: 2026-10-11. Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Disposition: **concrete verification blocker; source settlement is not claimed**. The one authorized focused Cargo invocation timed out during dependency compilation before either test target ran. No duplicate invocation was issued after the host restart.

## Resumed source and evidence

Read the complete main plan task 27, including Must NOT do and acceptance, before editing. Inspected the existing registry, module declaration, tests, CLI output/exit-code wiring, Cargo registration, historical task-27 receipts, current evidence map, enumeration blocker, and upstream candidate gate. Earlier evidence was preserved, not overwritten or replayed.

Existing source already implements local plus remote parallel listing, 20-second per-machine timeouts, per-host error isolation, exit-3 nothing-answering mapping, aggregate exit taxonomy, exact public field projection, and a 15-second registry-instance cache. CLI wiring already consumes the aggregate exit code. Existing host seed `/Users/indo/.omon/session-hosts.json` was read and contains `["omarchy"]`; historical task-27.json records the previous one-time probes and persistence. No new SSH probe or seed write was needed.

Changes in this resume:

- `src/sessions/registry.rs`: parse upstream `alive` and `surface` as typed optional fields without adding them to the seven-field public output. Replace synchronous filesystem operations in the async seed helper with Tokio filesystem calls and propagate serialization failure.
- `tests/test_session_registry_task27.rs`: new focused local fake-runner tests. A gated local host proves all four host futures start concurrently; successful local and remote rows survive a stalled host's exact 20-second deadline and an exit-3 host. Channels are registered before listing, awaits are bounded, and paused time replaces sleeps. Parser coverage checks typed liveness, rejects invalid liveness input, and verifies the public output remains seven fields without private metadata.

No edits to `src/lanes/runner.rs`, `src/lanes/cli.rs`, shared CLI/lib/Cargo wiring, existing integration tests, services, environment files, or installed runtime. No remote writes, messages, interrupts, adoption, installs, activation, or Git commit.

## Migration slot decision

Rechecked migration inventories in both the main tree and worktree and read task 15 prepared artifacts (`task15-nonconflicting-patch-units.md`, `task15-integrated-ready.md`). Main ends at 0030; this worktree ends at 0037. Task 15 reserves **0038**. The next free number observed was **0039**. No migration was created: task 27 specifies a command-backed registry with an in-memory cache and JSON host seed, not a database schema or persistent session cache. Adding an unused table would invent scope. If a later integration requires a migration, recheck the slot before using 0039; this receipt does not reserve it globally.

## Exact verification results

Scoped formatting command:

```sh
rustfmt --edition 2021 --check --config skip_children=true src/sessions/registry.rs src/sessions/mod.rs tests/test_session_registry_task27.rs
```

Final scoped check completed successfully with no output (exit 0). Earlier formatting differences were corrected using apply_patch.

LSP diagnostics were requested before Cargo for the changed module and new test. Both returned `Timed out waiting for fresh diagnostics ... within 3000ms.` The symbol lookup also returned `EPIPE: broken pipe, send`. These are unavailable tooling results, not clean diagnostic claims.

Exactly one focused Cargo invocation:

```sh
CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w27 cargo test -j2 --offline --test test_session_registry --test test_session_registry_task27
```

Original monitor: `mon_C0RRJ8F3P0E5GACG`, background session `bash_3`, owning shell PID 9103. After restart, PID 9103 was confirmed running and the original output was inspected. Exit subscription `mon_8EGMAY5S3SBKCK4J` watched that existing PID only; it did not invoke Cargo.

Final original monitor event: `watcher timed_out (exit code 1)`. Session status: `timed_out exit_code: 1`. Last rendered output was dependency compilation at `292/478: read-fonts, primal-check`. No `Finished`, test-target `Running`, or `test result:` lines were recorded. Therefore no test pass count or Cargo/compiler failure is asserted. The follow-up exit subscription completed with `TASK27_EXISTING_CARGO_DONE`, exit 0; that proves only the existing process exited. A final process lookup found neither PID 9103 nor the focused Cargo command. No retry was performed under the one-run constraint.

## Remaining acceptance boundary

Current source changes and the new tests are **not Cargo-verified**, and LSP freshness is unresolved. Source-settled status requires a successful focused verification authorized after this timed-out run.

Live acceptance remains separately pending on the patched omo-ai runtime candidate described in `task27-candidate-receipt.md`. Historical live gateway QA lost local rows because local enumeration exceeded the registry's 20-second bound; the prepared upstream candidate addresses the redundant RPC probe. Owner activation approval is required. This resume neither installs nor activates that candidate and makes no new live local/omarchy acceptance claim. Existing historical receipts remain intact.

## Authorized follow-up: source settled

Date: 2026-10-11. The owner authorized one new focused invocation after the historical dependency-compilation timeout above, with a 1800-second monitor deadline. **Source settlement is now verified by scoped formatting and both focused test targets passing.** This addendum supersedes the earlier source-verification blocker only; the previous run and migration decision remain preserved as history.

Read the full main-plan todo 27, current registry source/module and both focused tests, `task27-local-enumeration-blocker.md`, `task27-candidate-receipt.md`, and this existing receipt. Located the registered target using a `session_registry` content search in `Cargo.toml`; `test_session_registry_task27` is automatically discovered from its existing test file. Both requested target names worked without substitutions or manifest changes.

Commands executed from `/Users/indo/code/project/omon-gateway-wt/dori-work`:

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work && rustfmt --edition 2021 --check --config skip_children=true src/sessions/registry.rs src/sessions/mod.rs tests/test_session_registry.rs tests/test_session_registry_task27.rs
```

Result: exit 0, no formatting differences. Background session `bash_1` completed normally.

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work && CARGO_TARGET_DIR=$PWD/target-w27 cargo test -j2 --offline --test test_session_registry --test test_session_registry_task27
```

Executed exactly once in this follow-up through monitor `mon_Q2MH10Z4ZTW1ZAWZ`, session `bash_2`, `timeout_ms: 1800000`. Completed normally, **exit 0**, after compilation reported `Finished test profile [unoptimized + debuginfo] target(s) in 9m 39s`.

- `test_session_registry`: **7 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out**, test execution 0.00s; binary `target-w27/debug/deps/test_session_registry-bbbd9b1d70fa690e`.
- `test_session_registry_task27`: **2 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out**, test execution 0.04s; binary `target-w27/debug/deps/test_session_registry_task27-660a6abb1a576060`.

No compiler warning or error diagnostics were observed in the captured output. No registry source or test fixes were necessary, and no failing-target rerun or duplicate full invocation occurred. No fresh LSP result is claimed; the historical unavailable diagnostics above remain recorded. Full-repository clippy, all-target tests, and live acceptance were not run in this scoped follow-up.

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work && shasum -a 256 src/sessions/registry.rs src/sessions/mod.rs tests/test_session_registry.rs tests/test_session_registry_task27.rs
```

Result: exit 0, session `bash_3`. Frozen registry file SHA-256 values:

```text
0792ebef1066b449242ac81c7129a384c58a6eafbcd8c2bcf85fe5abb7f9f267  src/sessions/registry.rs
9256c987523dcb5ab3483a114bfafd4d3c0728a530577280b78913ec402dc0a3  src/sessions/mod.rs
110e812e42376462f5005a3789fb4f8e503523bff8e99a2de21af5b38ae23576  tests/test_session_registry.rs
33ee903df548e90b54ca948477e7f6199f393fea91b04b29e5c83bb529f85ee6  tests/test_session_registry_task27.rs
```

Only this receipt was edited in this follow-up. Registry source/tests were left unchanged. No runner.rs, cli.rs, live DB, service, environment file, model, Git state, installed runtime, or host seed was changed. Cargo populated only the requested verification target as part of the test run.

**Live local+remote acceptance remains pending on the patched omo-ai runtime candidate and requires owner activation approval. The candidate was neither installed nor activated.** Passing fake-runner registry tests does not satisfy the actual gateway local+omarchy acceptance scenario. Todo 27's production acceptance remains open. Migration note remains unchanged: **0038 is reserved for task 15; task 27 needs no migration.**

## Owner-reported direct binary verification

Date: 2026-10-11. The owner reports that focused Cargo PID 31828 finished building both test binaries and that they executed each exact binary directly once from the worktree root:

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work
target-w27/debug/deps/test_session_registry-bbbd9b1d70fa690e
target-w27/debug/deps/test_session_registry_task27-660a6abb1a576060
```

Owner-reported actual results: `test_session_registry` **7 passed / 0 failed, exit 0**; `test_session_registry_task27` **2 passed / 0 failed, exit 0**. These direct executions are owner-supplied evidence, separate from the monitor-observed Cargo results above. No additional Cargo or binary execution was performed by this worker after receiving this report.

**Source-settled for the focused registry scope.** Historical timed-out verification remains preserved. LSP diagnostics remain unavailable, not clean. Live local+remote acceptance on the patched omo-ai runtime candidate remains pending owner activation approval; no installation or activation was performed. **0038 remains reserved for task 15; no task 27 migration is needed.**

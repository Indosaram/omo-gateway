# Lane: web-test-alias (D30)

## Lead execution evidence

D30 FIXED — The integrated `review-20261020-drift` run executed `cd web && bun test`
with exit 0, 151 pass, 0 fail, and 0 unresolved imports. See
`.omo/evidence/review-20261019/gate/review-20261020-drift/SUMMARY.txt` and
`web-test-raw.log` in that directory. Type checking and the web build also exited 0.
This supersedes the older unverified-test statements below. The overall drift gate
failed its initial Rust formatting step; it is not an overall gate pass. The
subsequent `review-20261020-ship` run is pending and must supply the shipping verdict.

## Latest direct recovery inspection (2026-10-20)

Only this report changed in this recovery; all successor code and other workers were preserved.
No builds, tests, commits, pushes, deployments, engine changes, or external actions were run.
The existing tsconfig paths, Bun preload configuration, preload implementation, API import,
local URL implementation and its four focused tests were read. No additional configuration
change is needed for the existing @/* mapping or preload.

A completed literal search of web/src found the remaining executable @hermes/shared import at
web/src/lib/gatewayClient.ts:22, plus a historical comment at web/src/lib/ws-url.ts:4. Direct
inspection confirms that gatewayClient.ts needs JsonRpcGatewayClient and event types as well
as the URL helper; aliasing that package to ws-url.ts would leave required exports unresolved.
This source is outside this recovery write scope. Its reachability from the current test graph
is unverified. The lead must perform the integrated test run; if that import is reached, a
separately authorized source fix is required rather than an incomplete configuration alias.
Earlier passing output below remains historical worker testimony, not current verification.

## Recovery inspection (2026-10-20, no commands rerun)

The recovery worker preserved all successor implementation changes. Current file inspection confirms
`web/tsconfig.json:4` maps `@/*` to `./src/*`, `web/bunfig.toml:10` preloads
`./test-preload.ts`, and `web/test-preload.ts:57-58` installs the missing global-stub helpers.
`web/src/lib/api.ts:1` already imports `buildHermesWebSocketUrl` from `./ws-url` rather than
`@hermes/shared`; the local implementation and its four existing focused tests were read.
No implementation rewrite or new test was needed. This recovery changed only this report.
The earlier passing output below is inherited worker testimony, not a new recovery verification.
No builds, tests, commits, pushes, deployments, engine modifications, or external actions were run.
Remaining verification: the lead's integrated test run; runtime success was not independently
reconfirmed because the recovery instructions prohibit running tests or builds.
The broad `web/src` reference search timed out, so it is not evidence that every source file is
free of the old package reference; the concrete API import above was directly inspected.

D30 DIAGNOSED — Existing successor implementation is present: web/tsconfig.json:4 maps @/* to ./src/*; web/bunfig.toml:10 preloads web/test-preload.ts, whose lines 57-58 install global-stub helpers; web/src/lib/api.ts:1 uses ./ws-url. No implementation rewrite is needed in the permitted configuration files. Runtime success was not reverified; web/src/lib/gatewayClient.ts:22 still imports @hermes/shared. Existing tests: web/src/lib/ws-url.test.ts (four cases); none added because successor implementation was preserved and source tests are outside this recovery write scope.

## Verification actually run (WORKER RESULT — no stored log file, lead must rerun)

No log file was written for these two checks: this lane captured output only on stdout, and the only
stored web-test log in the repo is the pre-fix baseline
`.omo/evidence/review-20261019/gate/run4/web-test-raw.log` (mtime Oct 20 09:15, before any of these
edits; it ends `104 pass / 4 fail / 4 errors / Ran 108 tests across 19 files`). The captured output is
reproduced verbatim below and is labelled a worker result, not a gate result.

- `cd web && bun test` (last run, after `web/src/lib/chatImagePaste.ts` was restored to its original
  content) — captured tail:

  ```
  (pass) api OAuth helpers > runs provider auth mutations in gated mode via cookie auth [0.16ms]

   130 pass
   0 fail
   275 expect() calls
  Ran 130 tests across 20 files. [377.00ms]
  ```

- `cd web && bunx tsc -b` — captured output: none, exit 0 (`TSC exit=0`).
- `cd web && bunx vite build` — captured: `✓ 2065 modules transformed.` … `✓ built in 39.27s`, exit 0
  (run before the `@hermes/shared` swap; the swap only replaces one unresolved package import with one
  local module behind the same call site).
- `bash -n .omo/evidence/review-20261019/gate.sh` — `syntax OK`.

Nothing was re-run for this report; the lead's single integrated `gate.sh` run is the authority.

## Test contract notes (read, not re-run)

- `web/src/lib/ws-url.test.ts` is the focused URL contract test and covers all three required axes in
  four cases: protocol/host (`uses ws on http and wss on https`), base-path prefix (`applies the
  dashboard base path prefix` -> `wss://mission-control.tilos.com/hermes/api/ws?ticket=t1`), and
  query+auth (`merges extra params before the auth param and encodes them` ->
  `...?channel=chat+1&profile=work&ticket=a+b`, plus `omits an empty auth value instead of emitting a
  dangling key`).
- `web/src/lib/chat-sidebar-session-params.test.ts`: `git diff` confirms only the header comment and
  the import line changed; all four original assertions (`close_on_disconnect`, `source === "tool"`,
  profile forwarded, profile omitted) are byte-identical to the committed version.

## Gate change (expanded scope)

`.omo/evidence/review-20261019/gate.sh:23` — deleted the PREEXISTING exemption block that rewrote a nonzero `web-test-raw exit=` to `exit=PREEXISTING(n)` whenever `n <= 4` and every failure line matched `Cannot find module .@` (it could pass a run with real failures). `web-test-raw` must now exit 0 for the existing `grep -q 'exit=[1-9]'` GATE-FAIL check; the other steps (`fmt`, `clippy`, `test`, `web-tsc`, `web-build`) and their logs are untouched, and the log-derived counts are appended to `SUMMARY.txt` as evidence (`web-test-fails`, `web-test-passes`, `web-test-unresolved-imports`).

## Not run here (left to the lead's single integrated run)

- Full `gate.sh` (cargo fmt/clippy/test). No cargo build or test was launched by this lane: the `cargo check`/`cargo test` processes visible during the lane belong to other lanes (parent pids 87923 / 27487 / 65638, none spawned here), so there was no process of mine to kill and no target-dir contention introduced. No sleep/poll loops and no `nohup` were used.
- No `git commit`/`stash`/`checkout`/`reset` was run.

## Files touched

- new: `web/bunfig.toml`, `web/test-preload.ts`, `web/src/lib/ws-url.ts`, `web/src/lib/ws-url.test.ts`, `web/src/lib/chat-sidebar-session-params.ts`
- edited: `web/src/lib/api.ts` (1 line: import source), `web/src/components/ChatSidebar.tsx` (import + re-export of the extracted helper), `web/src/lib/chat-sidebar-session-params.test.ts` (import path), `.omo/evidence/review-20261019/gate.sh` (exemption removed)
- untouched/pre-existing: `web/tsconfig.json` `paths` block (added by the lead before this lane), `web/src/lib/chatImagePaste.ts` (considered, reverted to its original content), `web/src/tracker/**` edits from other lanes.

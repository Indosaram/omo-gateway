# Todo27 upstream fix artifact: analysis and test specification

Verification update (2026-10-11): isolated application succeeded with fuzz0; the focused suite passed49/49 and package type checking passed from the canonical physical checkout path. The initial compiler run failed on `/tmp` versus `/private/tmp` type identity and required a compiler-only rerun. Owned checkout/dependencies were removed. See `task27-upstream-verification.md` for commands, exits, hashes and cleanup. The original unrun notes below describe the artifact-preparation stage, not current test status. No runtime candidate was built or activated.

## Review base and scope

Artifact: `task27-upstream-fix.patch`, a unified patch relative to the oh-my-openagent repository root. Base: **6dfb41974556051b432c30ed71b134ed826c883a**, tag `v5.1.29`, npm `omo-ai`5.1.29. Installed engine pin remains `@code-yeongyu/senpi`2026.10.10-11. The matching installed/release SDK SHA-256 was established in `task27-local-enumeration-blocker.md`.

Exact production source: https://github.com/code-yeongyu/oh-my-openagent/blob/6dfb41974556051b432c30ed71b134ed826c883a/packages/omo-senpi/src/components/thread/live-surface.ts . Exact neighboring test source: https://github.com/code-yeongyu/oh-my-openagent/blob/6dfb41974556051b432c30ed71b134ed826c883a/packages/omo-senpi/src/components/thread/live-surface-tui.test.ts . Also read the same revision's `live-surface-endpoints.test.ts` and `live-surface.test.ts` for socket lifecycle, unknown verdict and offline failure contracts.

Only evidence artifacts were created. No upstream checkout, installed/generated runtime, gateway product source, package dependency, endpoint registry or daemon was changed. No live probes, listings, session actions, model calls, installs, builds, commits, pushes, PRs or external writes occurred. Shared daemon and actual8/9 QA remain intact. Todo27 and its gateway20s requirement remain unmet pending a verified dependency repair.

## Smallest production change

Two hunks in `packages/omo-senpi/src/components/thread/live-surface.ts`:

- Line295: use the existing normalized legacy-endpoint match to preserve its reported `verdict`; use `{}` only when no report exists. Paths, socket identity and existing kind resolution stay as they are.
- Lines320-322: remove the TUI-only condition from the explicit `alive:false` short circuit and update its comment. An RPC host already classified by engine status goes through the existing `degraded` result without a second list request. Explicit negative reasons and claimed disk paths are retained. Unknown verdicts still probe; `reachable:false` alone is not treated as a negative liveness verdict.

No new helper or timeout configuration is introduced. Enumerations still use the existing5000ms instance cache, which is not a permanent endpoint exclusion. Direct `listTarget` RPC behavior is unchanged: it supplies an unknown verdict, whereas existing TUI cache reuse remains as before. All-unavailable non-offline listings still reject through the current first-failure path; offline views still include degraded host rows. This addresses the evidenced redundant10s RPC pass, not an unconditional aggregate bound for arbitrary engine delays.

## Included deterministic regressions

One60-line test insertion reuses `hostEndpoint`, `tuiEndpoint`, `surfaceFor`, `threadsOf`, `tempDir` and the existing event-bound cleanup. There are five cases after parameter expansion:

1. Negative legacy RPC verdict: an actually answering test host is reported `alive:false, live_unresponsive`. Assert zero frames and zero dials to it, preserved offline disk row/reason, and responsive RPC plus authenticated TUI sessions still listed.
2. Negative shard RPC verdict: the same assertions with the negative verdict on the non-legacy socket. Cases1/2 separately catch lost legacy verdicts and the RPC-only second probe.
3. Unknown RPC verdict: report `reachable:false` but no `alive` verdict. Assert one real observing `list_sessions` frame and a live result. This guards against incorrectly treating reachability as a definitive cached verdict.
4. All-unavailable `dead`: both RPC endpoints have explicit negative verdicts. Non-offline `listSessions` rejects with the reason; offline view retains both host errors and no live sessions. Assert no dials or frames.
5. All-unavailable `live_unresponsive`: the same failure/offline assertions with that reason.

The negative endpoints deliberately remain responsive test servers: if the production guard is missing they answer promptly and expose a dial/frame/live-row regression, rather than allowing the test to pass by timing luck or waiting10s. Listening and close signals are subscribed before triggering their events, with the neighboring fixtures' bounded2000ms abort signals. No sleeps, polling or timing thresholds are added. On the unpatched source, both mixed cases and both all-unavailable cases should fail semantically; that expectation is source analysis, not a claimed test run.

## Verification performed and unrun gates

Performed in memory against the exact remotely read base: every hunk's old context and old/new line counts matched at lines292,319 and367; two production hunks, one test insertion. The patch was not applied to any source checkout. This is an exact-context applicability check, not `git apply --check`, type checking or behavioral execution.

Not run: upstream Bun tests, type/lint checks, upstream build/release QA, artifact-generated runtime verification, and actual gateway local+remote QA. The release-matched upstream checkout/dependencies are not present in the gateway repo, and installation/build/source-checkout mutation was expressly excluded. No Cargo run is relevant to this artifact.

After owner approval to prepare an isolated matching upstream source checkout with its required existing dependencies, the focused command from that repository root is:

```sh
bun test packages/omo-senpi/src/components/thread/live-surface-tui.test.ts packages/omo-senpi/src/components/thread/live-surface-endpoints.test.ts packages/omo-senpi/src/components/thread/live-surface.test.ts
```

Run upstream-required diagnostics and release QA under that checkout's read instructions, and include any required tracker/changelog metadata then; this artifact is the minimal behavior/test diff, not a claimed publish-ready upstream release. The exact package build command has not been inspected and is intentionally not invented here.

## Activation boundary requiring owner approval

Safe next step is to apply this artifact only in an isolated source checkout at the pinned revision, run its focused and required upstream gates, and produce a versioned runtime candidate without replacing the global install. This does not activate the fix. If that needs dependencies, request authorization before installing them.

Activation separately requires **owner approval** because it replaces the shared OMO runtime, not the gateway deliverable. After candidate verification, obtain approval for a coordinated update to that verified package/version during an owner-selected window that preserves or completes the active8/9 QA lane. Do not hot-edit global `plugin/runtime/thread-sdk/sdk.js`, stop c12d59ec1b7d76c7, prune registry entries or restart the shared daemon as a shortcut. Preserve old package/version/hash for rollback and verify the activated candidate identity before repeating only the required todo27 read-only gateway QA with bounded child cleanup. Gateway20s stays fixed, and todo27 is complete only after actual local retention plus remote/failed-host behavior passes.

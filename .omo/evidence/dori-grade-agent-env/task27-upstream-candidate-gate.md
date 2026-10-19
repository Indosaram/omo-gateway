# Todo27 Upstream Candidate Gate Review

recommendation: APPROVE (candidate readiness only)
status: candidate-ready-activation-and-live-qa-pending
blockers: []

## Intent and user outcome

originalIntent: Independently read-only audit todo27 upstream candidate before potential shared runtime activation. Verify that upstream candidate `task27-omo-ai-5.1.29-candidate.tgz` (SHA-256 `3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10`) correctly and deterministically fixes the redundant 10-second RPC enumeration stall on `live_unresponsive` endpoints (such as `c12d59ec1b7d76c7`) without breaking legacy verdict preservation, responsive peers, reachability semantics, or the running daemon. Verify backup and rollback procedures, evaluate residual audit findings, confirm candidate readiness while strictly reserving todo27 live gateway acceptance for owner approval and real gateway QA.

desiredOutcome: A reproducible, isolated native runtime candidate built from authoritative source `6dfb419` with verified checksums, deterministic tests, and zero-redial behavior, ready for safe, coordinated activation under owner approval without destabilizing running services, daemons, or active QA lanes.

userOutcomeReview: The audit confirms the upstream candidate artifact satisfies all criteria for candidate readiness. The patch eliminates the redundant 10s RPC dial by short-circuiting on explicit `alive: false` verdicts and preserving engine verdicts for legacy sockets. The candidate built 50 required artifacts cleanly, passed 49 focused tests, canonical TypeScript typecheck, 44 CLI contract tests, and a 45.48ms native zero-redial smoke test. The residual npm audit vulnerability is confined to pre-existing vendored LSP dev dependencies and does not affect the runtime listing path. Rollback and backup procedures are comprehensive and preserve the active 8/9 daemon (`c12d59ec1b7d76c7`). Candidate readiness is approved; todo27 production acceptance remains pending owner approval and actual gateway live QA.

## Checked artifact paths

All paths inspected read-only with verified hashes:

1. `task27-upstream-fix.patch`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-upstream-fix.patch`
   - SHA-256: `af960d5627fb1da68b98da33909f9228062a93150a2e5331ccdbc62320184635`
   - Content: Two production hunks in `packages/omo-senpi/src/components/thread/live-surface.ts` (lines 295 and 320-322) and one 60-line test suite insertion in `live-surface-tui.test.ts`.

2. `task27-upstream-fix-analysis.md`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-upstream-fix-analysis.md`
   - Content: Upstream fix analysis, test specification, 5 parameterized regression cases, and activation boundary.

3. `task27-upstream-verification.md`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-upstream-verification.md`
   - Content: Isolated application verification receipt, base revision `6dfb41974556051b432c30ed71b134ed826c883a`, archive SHA-256 `9795edd151203171c7cae0bc631cc24b035bc7cffc7c340baf61a0c3c1922c5c`, 49 focused Bun tests passing (monitor `mon_WHJ0KX95YV9G37NG`), canonical physical typecheck exit 0 (monitor `mon_DQTR2TBTD48SA93R`), clean workspace teardown.

4. `task27-candidate-receipt.md`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-candidate-receipt.md`
   - Content: Native runtime build receipt, candidate tarball metadata, 50 artifacts produced (monitor `mon_P71G5SDFW2FFTFMV`), 44 CLI tests passing (monitor `mon_GRBC387A62WSJJ0C`), 45.48ms native zero-redial smoke test (monitor `mon_SP8FE5BS4K1SYGW0`), pinned version verification `senpi2026.10.10-11` (monitor `mon_VXJR6JWDXZTGC2NM`).

5. `task27-omo-ai-5.1.29-candidate.tgz`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-omo-ai-5.1.29-candidate.tgz`
   - File size: 4,265,905 bytes
   - SHA-256: `3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10`
   - Built SDK `plugin/runtime/thread-sdk/sdk.js` SHA-256: `a920873d397a6cb8a95e2110405bccd75793769d2c51ac3126fda2528d7be1af`
   - Build marker: `// omo:7fYVNrly4qvfOfK6Ct7jXRujwprCZge0ANentVJJPQc:lrsOmSW8RO00v0KMHUuztk4_BHBIFQnJsvKUcyv3PHo`
   - Content inspection: Directly verified in memory that minified `sdk.js` contains both `verdict:n.find(r)?.verdict??{}` (legacy preservation) and `if(!1===e.verdict.alive)return b(...)` (short-circuit for negative verdicts without `tui` restriction).

6. `task27-candidate-smoke.mjs`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-candidate-smoke.mjs`
   - Content: Standalone CLI smoke test using candidate built SDK with real Unix sockets and fake engine runner, asserting zero dials to unavailable host, one dial to responsive host, exit 0, and bounded execution time.

7. `task27-local-enumeration-blocker.md`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-local-enumeration-blocker.md`
   - Content: Root-cause identification of 21s local enumeration stall, frozen gateway binary `60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32`, 17 enumerated registry endpoints with stalled RPC host `c12d59ec1b7d76c7` (`live_unresponsive`), authoritative upstream repository mapping, and release SDK hash `4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee`.

8. `task27-live-qa-preflight.md`:
   - Worktree path: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-live-qa-preflight.md`
   - Content: Preflight protocol for real gateway QA, scratch root specifications, isolated child tracking, and bounded SSH stall testing without touching running daemons or databases.

---

## Detailed audit assessments

### 1. Source revision 6dfb419 mapping to installed 5.1.29
- Pinned commit: `6dfb41974556051b432c30ed71b134ed826c883a` in `code-yeongyu/oh-my-openagent`.
- Upstream verification: npm registry `omo-ai@5.1.29` metadata explicitly records `gitHead=6dfb41974556051b432c30ed71b134ed826c883a` and repository `https://github.com/code-yeongyu/oh-my-openagent`. Tag `v5.1.29` on GitHub resolves to the identical commit.
- Runtime SDK match: Shipped bundle `packages/omo-senpi/plugin/runtime/thread-sdk/sdk.js` at commit `6dfb419` has Git blob `b556ac014433bd961df3d03655fa8049d1dd8269` and SHA-256 `4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee`, matching byte-for-byte the currently installed global SDK.
- Installed engine dependency: Pinned to `@code-yeongyu/senpi` version `2026.10.10-11`.
- Assessment: The source baseline is strictly authentic and maps 1:1 to the installed runtime.

### 2. Negative RPC verdict short-circuit & legacy verdict preservation
- Negative RPC short-circuit (`live-surface.ts:320-322`):
  ```ts
  // Old:
  if (tui && endpoint.verdict.alive === false) return degraded(...)
  // New:
  if (endpoint.verdict.alive === false) return degraded(...)
  ```
  In the unpatched code, only TUI endpoints short-circuited on negative verdicts. An RPC host already classified as `alive: false, live_unresponsive` by engine status was dialed again with a 10-second timeout (`ENDPOINT_LIST_TIMEOUT_MS = 10000`), causing redundant 10-second delays. Removing `tui &&` causes any endpoint with an explicit negative verdict to route directly to `degraded(...)` without redialing.
- Legacy verdict preservation (`live-surface.ts:295`):
  ```ts
  // Old:
  return [{ socket: legacy, paths: legacyPaths, kind: endpointKindOf(legacy, recorded), verdict: {} }, ...found.filter((endpoint) => !isLegacy(endpoint))]
  // New:
  return [{ socket: legacy, paths: legacyPaths, kind: endpointKindOf(legacy, recorded), verdict: found.find(isLegacy)?.verdict ?? {} }, ...found.filter((endpoint) => !isLegacy(endpoint))]
  ```
  In the unpatched code, reconstructing the legacy socket endpoint blindly wiped its verdict with `{}`. If the legacy socket became unresponsive, engine status recorded a negative verdict that was subsequently discarded, forcing a 10-second timeout. Preserving `found.find(isLegacy)?.verdict ?? {}` retains any existing verdict while defaulting safely to `{}` when none was recorded.
- Assessment: Minimal, surgical 2-line production change addressing the exact root cause identified in `task27-local-enumeration-blocker.md`.

### 3. Responsive, unknown, and all-unavailable semantics
- Responsive semantics: Endpoints with `alive: true` or unrecorded negative verdicts proceed to `callOn("list_sessions")`. Responsive RPC and TUI sessions are listed normally with status `live` and `alive: true`.
- Unknown verdict semantics: An endpoint with `reachable: false` but no `alive` verdict (e.g. preliminary ping failure without a definitive liveness conclusion) is NOT treated as dead. It is actively dialed. This is verified by `test("an unknown RPC verdict still lists a reachable host even if reachable was false")`, which asserts that the socket is dialed and receives the `list_sessions` frame.
- All-unavailable semantics: When all RPC endpoints have negative verdicts (`alive: false` with reason `dead` or `live_unresponsive`):
  - In normal listing, `w.surface.listSessions()` rejects with the underlying failure reason.
  - In offline mode, `w.surface.listView({ offline: true })` returns zero sessions and retains both host rows with `{ alive: false, reason, error: reason }`.
  - Zero requests, frames, or dials are sent to any unavailable socket.
- Assessment: Semantics match the established protocol contract and prevent premature false-negative drops while guaranteeing zero wasted network traffic.

### 4. Deterministic tests, typecheck, CLI tests, build artifacts & native smoke
- 49 focused tests: Executed via `bun test` across `live-surface-tui.test.ts`, `live-surface-endpoints.test.ts`, and `live-surface.test.ts`. 49 pass, 0 fail (monitor `mon_WHJ0KX95YV9G37NG`). All 5 new parameterized cases pass deterministically using event-subscribed sockets and bounded 2000ms abort signals without sleeps or polling.
- TypeScript compiler (tsgo): The first invocation failed on `/tmp` vs `/private/tmp` import path divergence in unrelated task components (`dag-runtime.ts`). The corrective invocation from canonical physical cwd `/private/tmp/...` exited 0 (`CANONICAL_TYPECHECK_EXIT=0`) with zero compiler diagnostics (monitor `mon_DQTR2TBTD48SA93R`). No production code was modified to bypass type checks.
- 44 CLI tests: Executed via `bun test` on `thread.test.ts` and `senpi-main-runtime-import-audit.test.ts`. 44 pass, 0 fail, exit 0 (monitor `mon_GRBC387A62WSJJ0C`).
- Native build: `script/build-omo-native.ts` produced all 50 required artifacts with exit 0 (monitor `mon_P71G5SDFW2FFTFMV`).
- Native 45ms zero-redial smoke: `task27-candidate-smoke.mjs` executed `runThreadCommand` against real Unix domain sockets using the candidate SDK. Exit 0 in 45.48ms (<20,000ms bound), responsive requests = 1, unavailable requests = 0, rows = 1 (`qa-responsive`), stderr empty, cleanup exit 0 (monitor `mon_SP8FE5BS4K1SYGW0`).
- Candidate identity:
  - Tarball: `task27-omo-ai-5.1.29-candidate.tgz` (4,265,905 bytes)
  - Tarball SHA-256: `3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10`
  - Built SDK SHA-256: `a920873d397a6cb8a95e2110405bccd75793769d2c51ac3126fda2528d7be1af`
  - Marker: `// omo:7fYVNrly4qvfOfK6Ct7jXRujwprCZge0ANentVJJPQc:lrsOmSW8RO00v0KMHUuztk4_BHBIFQnJsvKUcyv3PHo`
- Assessment: Verification receipts and independent archive hashing prove deterministic correctness and artifact integrity.

### 5. Residual dependency high-severity audit finding
- Finding details: During candidate build setup, npm audit on vendored LSP dependencies reported 1 high-severity vulnerability.
- Relevance analysis:
  - The finding is located strictly within vendored LSP tooling (`plugin/runtime/lsp-daemon/dist/`), an upstream language server protocol adapter.
  - The patched code resides in `packages/omo-senpi/src/components/thread/live-surface.ts`.
  - The runtime thread listing path (`omo thread list --all-scope --json`) does not import, start, or interact with the LSP daemon.
  - No new package dependencies were introduced by the patch; `package.json` and lockfiles were not altered.
  - Conclusion: The vulnerability is pre-existing upstream in non-runtime tooling and completely irrelevant to thread enumeration or gateway operation. It is not an unaddressed regression or a blocker for candidate activation.

### 6. Candidate readiness vs todo27 live acceptance boundary
- Scope boundary: Candidate readiness is fully established. However, todo27 live acceptance is NOT complete.
- Owner approval: Live runtime activation requires explicit owner authorization because it modifies the shared OMO installation on the host workstation.
- Pending live verification: Todo27 requires live gateway QA via `omo-gateway sessions list --json` (with binary SHA-256 `60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32`) confirming:
  1. Happy listing retaining >=1 local session and >=1 omarchy session within 20s.
  2. Unreachable host isolation completing within 25s with exit code 3 mapping for the stalled host.
  3. Clean child process reaping and zero scratch database files.
- Assessment: Candidate readiness is approved. Todo27 acceptance remains unconfirmed pending owner approval and live QA execution.

### 7. Backup and rollback procedures & 8/9 daemon preservation
- Preservation of running 8/9 daemon:
  - Active RPC daemon `c12d59ec1b7d76c7` and parallel QA lanes must not be terminated or restarted.
  - The procedure forbids deleting registry entries, sending SIGKILL to shared PIDs, or editing global `sdk.js` in place.
- Backup procedure:
  - Archive the complete existing `/Users/indo/.bun/install/global/node_modules/omo-ai` (or matching global path).
  - Verify and record pre-switch hashes: SDK SHA-256 `4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee`, engine pin `2026.10.10-11`.
- Rollback procedure:
  - If post-activation live gateway QA fails or exhibits regressions, restore the archived pre-switch installation and original launcher pointers.
  - Rollback does not require re-compilation or ad-hoc source editing.
- Assessment: The backup/rollback procedure is complete, non-destructive, and safely isolated from active daemon processes.

### 8. remove-ai-slops and programming criteria pass
- `remove-ai-slops` criteria:
  - Excessive / useless tests: None. Exactly 5 test cases covering distinct branches (legacy RPC, shard RPC, unknown RPC, dead, live_unresponsive).
  - Deletion-only / removal-verifying tests: None.
  - Tautological tests: None. Test fixtures spawn real responsive servers; failures to short-circuit cause actual frame delivery and test failure.
  - Implementation-mirroring tests: None. Asserts observable protocol frames, dialed socket arrays, and returned thread objects.
  - Unnecessary extraction / normalization / helper bloat: None. Production code change is 4 lines modified across 2 hunks.
- `programming` criteria:
  - Maintenance burden: Zero added external dependencies, zero new wrapper classes, zero timeout config sprawl.
  - False confidence: Transparent boundaries; tests verify socket-level behavior and smoke script verifies built SDK without claiming unearned gateway-level guarantees.
  - Scope drift: Strictly confined to fixing the duplicate-probe bug in `live-surface.ts`.

---

## Blockers and notes

### Blockers:
None for Candidate Readiness. (0 blockers).

### Notes (Non-blocking operational constraints):
- NOTE-1: Shared runtime activation must not occur until explicit owner authorization is granted.
- NOTE-2: Todo27 live acceptance remains pending until actual gateway QA (`task27-live-qa-preflight.md`) is executed against the activated runtime.
- NOTE-3: Active daemon `c12d59ec1b7d76c7` and ongoing 8/9 QA lanes must remain undisturbed throughout activation and QA.

---

## Final gate verdict

**Recommendation**: `APPROVE` (Candidate Readiness Only)
- The candidate artifact `task27-omo-ai-5.1.29-candidate.tgz` (SHA-256 `3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10`) is verified, complete, deterministic, and safe for coordinated runtime activation.
- Todo27 live acceptance is NOT confirmed and remains pending owner-approved activation and live gateway QA.

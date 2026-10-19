# Todo27 isolated native runtime candidate

Date: 2026-10-11. Candidate built and related isolated CLI QA passed; not installed, activated, published or release-certified. Gateway20s remains unchanged; todo27 production local+remote acceptance remains unmet until separately approved activation and actual gateway QA. Shared daemon and actual8/9 QA were not changed.

## Retained artifact

- File: `task27-omo-ai-5.1.29-candidate.tgz`,4265905 bytes.
- SHA-256: `3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10`.
- Root archive layout: `package.json`, `bin/`, `plugin/`. Dependencies/node_modules are not packaged. Manifest remains `omo-ai`5.1.29 with exact `@code-yeongyu/senpi`2026.10.10-11 dependency. This is a locally patched candidate, not a newly published5.1.29 release; hash is required to distinguish it.
- Built SDK `plugin/runtime/thread-sdk/sdk.js` SHA-256: `a920873d397a6cb8a95e2110405bccd75793769d2c51ac3126fda2528d7be1af`.
- Build marker: `// omo:7fYVNrly4qvfOfK6Ct7jXRujwprCZge0ANentVJJPQc:lrsOmSW8RO00v0KMHUuztk4_BHBIFQnJsvKUcyv3PHo`.
- Exact base6dfb41974556051b432c30ed71b134ed826c883a plus `task27-upstream-fix.patch` SHA-256 `af960d5627fb1da68b98da33909f9228062a93150a2e5331ccdbc62320184635`. Patched `live-surface.ts` source SHA-256 remains `c5777c073dd641de268e172e2377e93356c0e3d92ae9680addd6a6b77cc405c6`, matching previous source verification.

## Build and recovery receipts

Previous verification cleanup removed archive/cache, requiring new owned root `/private/tmp/task27-candidate-xZJMq9`. Downloaded exact GitHub revision archive and ran one isolated `bun install --frozen-lockfile --ignore-scripts` with owned HOME/cache:456 packages,14.22s, SETUP_EXIT=0. No global installation.

Actual command: from pinned repository root, with owned HOME/TMPDIR/Bun/npm caches, `bun script/build-omo-native.ts --output <owned>/candidate/plugin`.

First build exited1 at `build:materialize-frontend`: source archive lacks `.git`, so `git submodule update --init --recursive` cannot run. LSP daemon and AST MCP inputs had already built successfully. One isolated `git clone --no-checkout --depth 1 --branch v5.1.29` recovered matching metadata; HEAD was exact6dfb419. Moved only that owned metadata into the owned archive tree. Second invocation exited1 because the no-checkout index lacked submodule paths. `git read-tree <exact revision>` populated only the owned index, without changing patched work files. Third invocation reused completed inputs and dependencies, materialized required upstream submodules, and succeeded: **50 required artifacts present, BUILD_EXIT=0**, monitor `mon_P71G5SDFW2FFTFMV`. No install/build was repeated after that success.

The standard build maps `src/extension/thread-sdk.ts` to `runtime/thread-sdk/sdk.js` through `build-extension-core.mjs`, using Bun node-target ESM plus the standard Terser pass and source/build digest marker. The entry imports the thread component SDK, which imports patched `live-surface.ts`. This is the normal complete native build, not hand-edited generated output or a replacement SDK copied into the global package.

Build setup's vendored LSP npm audit reported one high-severity dependency vulnerability. No unrelated dependency changes or audit fix were attempted; this remains a release review finding, not a patch-induced test failure.

## Related QA

Existing CLI contract tests and source import audit ran once from `packages/omo-senpi` using package-local hermetic preload:

```sh
bun test --timeout 20000 ../omo-native/test/thread.test.ts src/senpi-main-runtime-import-audit.test.ts
```

**44 pass,0 fail, RELEASE_CHECK_EXIT=0**, monitor `mon_GRBC387A62WSJJ0C`. Root manifest and CI expose no adapter ESLint/Biome lint command. The relevant existing import audit passed; no invented formatter/lint installation was added. Earlier49 focused source tests and canonical package typecheck0 were reused, not repeated, because source hashes stayed unchanged. No broader release/model session parity scenario was run: it creates model sessions and sends messages, outside this read-only listing scope.

`task27-candidate-smoke.mjs` uses the existing native `runThreadCommand` seam from `bin/lib/thread.js` and its normal built-SDK import. Only engine status enumeration is a fake-runner response; the SDK is real candidate output and the endpoints are owned real Unix socket servers. Owned layout2 registry has two correctly hashed endpoint directories. One host is responsive; the legacy host accepts connections but has an explicit engine `alive:false, live_unresponsive` verdict. Only `list --all-scope --json` is executed. No send/adopt/interrupt/model call.

Actual result: **CLI exit0,45.48025ms, responsive requests1, unavailable requests0, rows1, stderr empty**. Projection is names/cwd only: name `qa-responsive`, cwd `<owned>/qa`. Smoke asserts duration below20000ms and closes owned sockets/servers through pre-subscribed bounded close events: SMOKE_CLEANUP=0, SMOKE_EXIT=0, monitor `mon_SP8FE5BS4K1SYGW0`. This proves compiled-artifact negative-verdict reuse and responsive retention within the gateway cutoff in the fixture; it does not claim an actual gateway invocation, real engine-status10s pass or universal20s aggregate guarantee.

First version smoke from the unstaged dependency layout triggered Bun dependency auto-resolution and printed engine2026.10.11 with exit0. That was not accepted as pinned-engine evidence. A temporary candidate node_modules link to the already installed isolated root dependencies corrected resolution, then the same launcher printed **omo5.1.29 (engine: senpi2026.10.10-11), PINNED_VERSION_EXIT=0**, monitor `mon_VXJR6JWDXZTGC2NM`. The link and auto-resolution state were confined to owned candidate/HOME and removed. The retained archive was created before that link and contains no dependency link.

## Cleanup and approval request

Independent gate `st01a12beb` approved candidate readiness only. Activation remains on hold until explicit owner OK and completion of `72cleanup`; neither is supplied by readiness approval. No further build/test repeats or shared switch/service writes are authorized by that gate.

Preserve two separate identities: endpoint ID `c12d59ec1b7d76c7` was observed as live_unresponsive; the user identifies the actual QA-retained sole daemon as PID57142 with listener19742. These are not established to be the same endpoint/process. Preserve both independently, without stopping either, and never merge their identities without concrete mapping evidence. The PID/listener identity here is user-supplied, not a fresh process probe.

Candidate archived with `tar -czf <evidence>/task27-omo-ai-5.1.29-candidate.tgz -C <owned>/candidate .`, exit0. No process command matched the owned root after monitors completed. Removed owned checkout, Git metadata/submodules, build intermediates, dependencies/caches/HOME, registry fixtures and sockets; final stat of owned root returned ENOENT. Retained archive, smoke script, patch and receipts remain in dori-work evidence.

**Activation request, not executed:** authorize a coordinated switch to this hash-pinned candidate during an owner-selected window that preserves/completes actual8/9 QA. Before any switch, archive the current shared omo-ai installation and record its package, engine and SDK hashes (known current SDK `4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee`). Prepare candidate dependency resolution with the exact engine pin in an isolated prefix, validate package identity, and have the owner approve the shared launcher/runtime selection separately. Do not overwrite only global sdk.js, prune endpoint records or stop c12d59ec1b7d76c7. The install/switch mechanism must preserve running sessions; it has not been executed or validated here.

**Rollback request:** retain the pre-switch full installation and original launcher selection. If post-switch todo27 gateway local+remote or timeout-host QA fails, restore that complete hash-verified installation/launcher selection under the same coordinated owner approval, not an in-place generated-file edit. Gateway binary/source20s contract remains frozen. No claim of production completion before actual receipts.

Backup completeness contract for that future approved step: capture the complete installed omo-ai package (manifest, bin/launcher helpers, plugin payload and install scripts), its exact engine and resolved runtime dependency closure, and the user-facing launcher/shim or symlink with its target and original selection. Preserve file modes and symlink targets; produce a relative-path manifest with a SHA-256 for every regular file and an archive hash. Dependencies outside the package directory must be included in the backup or retained as an immutable, hash-inventoried closure; a package-only archive with unresolved external dependencies is not a full rollback backup. Record the original launcher resolution and package/engine identity. Rollback restores the full backed-up installation and original launcher selection as a coordinated operation; no in-place source or generated-file edits. Do not overwrite live agent state or endpoint registries, and preserve PID57142/listener19742 and endpoint c12d59ec1b7d76c7 independently throughout. No backup, switch or rollback command has been executed here.

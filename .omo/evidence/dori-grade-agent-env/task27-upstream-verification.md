# Todo27 isolated upstream verification receipt

Date: 2026-10-11. Result: focused tests and package type checking passed. No shared activation or package build occurred. Todo27 gateway live acceptance remains unmet pending a verified runtime candidate and separately approved activation. Actual8/9 shared-daemon QA was not altered.

## Source and artifact identity

- Base revision: `6dfb41974556051b432c30ed71b134ed826c883a` (`v5.1.29`). Archive URL: `https://codeload.github.com/code-yeongyu/oh-my-openagent/tar.gz/6dfb41974556051b432c30ed71b134ed826c883a`.
- Archive SHA-256: `9795edd151203171c7cae0bc631cc24b035bc7cffc7c340baf61a0c3c1922c5c`.
- Patch: `task27-upstream-fix.patch`, SHA-256 `af960d5627fb1da68b98da33909f9228062a93150a2e5331ccdbc62320184635`.
- Owned temporary root: `/tmp/task27-upstream-hxNWMX`; physical alias `/private/tmp/task27-upstream-hxNWMX`. Archive extracted into `oh-my-openagent-6dfb41974556051b432c30ed71b134ed826c883a` beneath it. No owner WIP checkout was used.
- Base `live-surface.ts` SHA-256: `da23ce3f21d34615893702fe8fef23d132c18c7dc9642a1b2e140672c1e9d71a`; patched: `c5777c073dd641de268e172e2377e93356c0e3d92ae9680addd6a6b77cc405c6`.
- Base `live-surface-tui.test.ts` SHA-256: `736149a8d87678249c325691fad53ab214fa00b8cf4802dcfb804e3388e2a306`; patched: `ba7ecb07bad499f1e2d64e5097eb52099e521dc33dee3ef7c1efa97b8456c988`.

## Setup and exact patch application

Read pinned root/package/component instructions, ROADMAP, manifests, package tsconfig, Bun configuration, and test isolation preloads. Read the Bun skill before installation. Upstream instructions require broader release QA; the user's explicit isolated-only scope excludes shared activation, commits, PRs, and broader live execution.

Download used `curl --fail --location --max-time 120 --silent --show-error <archive URL> -o <owned>/source.tar.gz`, then `tar -xzf <owned>/source.tar.gz -C <owned>`. Monitor `mon_V0N52ZZKBK6NTRRF` reported `ARCHIVE_READY`, exit0.

Dependencies: from the extracted root, `HOME=<owned>/home BUN_INSTALL_CACHE_DIR=<owned>/bun-cache bun install --frozen-lockfile --ignore-scripts`. Monitor `mon_AZSWP1HET6NJX73X` reported **456 packages installed [14.56s], INSTALL_EXIT=0**. No global install or lifecycle script execution. Dependencies/cache stayed in the owned tree.

Artifact translated to `apply_patch` update sections for exactly two isolated source files. Application result: **two applied operations, fuzz0, no failures or partial success**. Production diff: four lines removed/four added in two hunks; test insertion60 lines. No shell `git apply` was used. Auto-LSP initialization initially lacked a TypeScript installation while dependency setup was running; the actual compiler check below is the verification authority.

## Actual tests and compiler exits

Working directory: `<owned>/oh-my-openagent-<revision>/packages/omo-senpi`.

```sh
HOME=<owned>/home TMPDIR=<owned> OMO_DISABLE_POSTHOG=true bun test --timeout 20000 src/components/thread/live-surface-tui.test.ts src/components/thread/live-surface-endpoints.test.ts src/components/thread/live-surface.test.ts
```

Monitor `mon_WHJ0KX95YV9G37NG`: **49 pass,0 fail, TEST_EXIT=0**. This includes all five new parameter-expanded cases: negative legacy/shard verdict zero requests with mixed responsive RPC/TUI preservation; unknown verdict still lists; all-unavailable dead/live_unresponsive retains failure and offline rows. Only this one test batch was run. Package-local preload warms lazy dependencies and redirects HOME/agent discovery into hermetic test state. Socket fixtures use subscribed listening/close events and bounded aborts; no sleeps were added to the patch tests.

First compiler invocation from the `/tmp` spelling:

```sh
HOME=<owned>/home <root>/node_modules/.bin/tsgo --noEmit -p tsconfig.json
```

Actual **TYPECHECK_EXIT=1**, four TS2345 errors in unrelated files: `src/components/task/dag-runtime.ts:176,184,280` and `src/components/task/team-service-support.test.ts:22`. All described `TaskConcurrency.#config` as distinct between `/tmp/...` and `/private/tmp/...` imports. No diagnostic named either changed thread file. This failure was not suppressed or edited away.

Corrective compiler-only invocation used physical cwd and executable paths, without source/config edits or reinstall:

```sh
cd /private/tmp/task27-upstream-hxNWMX/oh-my-openagent-6dfb41974556051b432c30ed71b134ed826c883a/packages/omo-senpi
HOME=/tmp/task27-upstream-hxNWMX/home /private/tmp/task27-upstream-hxNWMX/oh-my-openagent-6dfb41974556051b432c30ed71b134ed826c883a/node_modules/.bin/tsgo --noEmit -p tsconfig.json
```

Monitor `mon_DQTR2TBTD48SA93R`: **CANONICAL_TYPECHECK_EXIT=0**, no compiler diagnostics. This resolves the first run's filesystem-alias collision. The source/artifact stayed unchanged throughout both compiler invocations.

## Cleanup and remaining boundary

After all monitors completed, `ps -axo pid=,command=` filtered for the unique owned root returned no process matches (exit1 from the filter). Removed only `/tmp/task27-upstream-hxNWMX`, including archive, checkout, node_modules, isolated install cache and scratch HOME/test state. Final stat returned ENOENT: **owned root absent**. No remaining owned run requires attachment or teardown.

No shared installed OMO package, daemon, live-unresponsive host, registry entry, gateway manifest or product source was modified. No model/session action, commits, pushes, PRs or external writes. Reviewable patch remains in dori-work evidence.

Not run: broad upstream release gates, lint, runtime/package build, generated candidate verification, real CLI harness QA and gateway local+remote acceptance. Thus this is green focused source verification, not a green release/package artifact or activated fix. A separate authorized isolated candidate-build/release-QA step must precede any owner-approved coordinated shared OMO installation. Preserve current runtime and actual8/9 QA until that explicit activation decision. Gateway20s is unchanged.

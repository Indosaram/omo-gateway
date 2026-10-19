# Todo27 local enumeration dependency blocker

Date: 2026-10-11. Status: external dependency finding accepted; todo27 remains unmet. Gateway per-machine timeout remains 20 seconds. This is not a blanket goal-blocked verdict: actual8/9 lifecycle QA continues independently. No installed/generated runtime, gateway source, active RPC host, endpoint registry or service was changed. No transport probes or listings were repeated while preparing this report.

## Actual gateway and direct-list evidence

Frozen gateway binary SHA-256: `60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32`. Exact actual gateway receipts are in `task-27.json` and `task-27-live-failure.log`.

| Invocation | Numeric exit | Duration ms | Local records | Remote records | Outcome |
| --- | ---: | ---: | ---: | ---: | --- |
| One gateway happy local+omarchy list | 0 | 20019.690833 | 0 | 3 | Local host code3: timed out after20s; happy acceptance failed |
| One gateway loopback handshake-failure list | 0 | 20011.155041 | 0 | 4 | Local code3 and loopback code3; <=25s and partial remote success, required local retention failed |
| One direct local list with explicit owner discovery | 0 | 21027.461875 | 12 | not applicable | Valid JSON array, stdout10703 bytes, stderr0 bytes; no model/session control |

Direct argv: `["/Users/indo/.bun/bin/omo","thread","list","--all-scope","--json"]`. Direct environment names only: `HOME, LOGNAME, NODE_COMPILE_CACHE, OMO_CODING_AGENT_DIR, PATH, SENPI_CODING_AGENT_DIR, TMPDIR, USER`. Owner HOME was retained in gateway QA; explicit agent state resolves to the same canonical directory as default HOME discovery. Correct owner discovery still exceeded the gateway cutoff. No arbitrary HOME fallback or cache change is supported by this evidence. No exact-QA-environment direct replay was needed.

Direct monitor `mon_CR6ED2SFCM8Q5NNX` exited0; owned child PID84494 exited normally with no kill needed and no remaining process group. Separate current `omo thread --help` returned successfully; supported list options are `--all-scope` and `--json`. Its owned child PID84963 also exited normally. Diagnostic scratch was removed. Earlier two gateway groups PID63717/PID65228 exited; owned loopback listener closed, zero sockets and scratch DB artifacts remained, and that scratch root was removed. No raw session IDs, controls values, credentials or auth contents are included here.

The earlier statement that two sequential10s probes explain every21s run was too broad. The evidence is a specific current stall plus duplicate probing in the SDK; fast endpoints answer in milliseconds, and no unavoidable21s baseline is claimed.

## Independent bounded endpoint observations

One concurrent wave used the installed engine's `requestOnSocket` transport with only `get_protocol_info` and `list_sessions`, `observe:true`, and 1500ms per independent request. Each addressable endpoint received these two requests concurrently; the endpoints were also concurrent. The transport correlates responses, destroys each socket on response/error/deadline, and preserves private authentication internally. No full host-status/thread-list command was run in this wave. The transport API returns a reply or undefined, not a numeric CLI exit; no transport exit code is invented.

17 registry entries were enumerated from disk,16 had addressable sockets,9 answered both requests. Successful list replies contained14 total session rows (not a deduplicated all-scope list). Only IDs of endpoint directories, endpoint kind, timing, answer boolean and numeric session counts were exposed.

| Endpoint directory ID | Kind | Protocol ms / answered | List ms / answered | Session rows |
| --- | --- | --- | --- | ---: |
| 0c8a315182d13484 | tui | 1505 / no | 1505 / no | unavailable |
| 1af2392e229ee6eb | rpc_host | unaddressable | unaddressable | unavailable |
| 1d71fb2a7bac7888 | rpc_host | 4 / yes | 5 / yes | 6 |
| 1f7491a45734a5c9 | rpc_host | 1 / no | 0 / no | unavailable |
| 5f3cede45bb1dc80 | tui | 48 / yes | 48 / yes | 1 |
| 5fa4fb49c6b68a75 | tui | 2 / yes | 2 / yes | 1 |
| 6d2f265046ef52ce | rpc_host | 1 / yes | 1 / yes | 0 |
| 8eead1ffd86fb7f2 | tui | 1 / yes | 1 / yes | 1 |
| 98865417b26803d4 | rpc_host | 1 / yes | 1 / yes | 0 |
| 9b591be37c5c7b89 | tui | 1 / yes | 1 / yes | 1 |
| 9c752fe15de80537 | tui | 1 / yes | 1 / yes | 1 |
| ae15b935e08348a9 | tui | 1501 / no | 1501 / no | unavailable |
| bb5a6ff2534f36a8 | rpc_host | 1 / yes | 2 / yes | 3 |
| c12d59ec1b7d76c7 | rpc_host | 1501 / no | 1501 / no | unavailable |
| c9dd545bbc3d198f | rpc_host | 0 / no | 0 / no | unavailable |
| d28c930695b6af20 | tui | 1502 / no | 1502 / no | unavailable |
| e5dbb3498e33a3af | rpc_host | 0 / no | 0 / no | unavailable |

Disk-only `judgeEndpointLiveness` classified the four budget-exhausting entries `0c8a315182d13484`, `ae15b935e08348a9`, `c12d59ec1b7d76c7`, and `d28c930695b6af20` as `live_unresponsive`. They are not established removable stale entries. The specific stalled RPC endpoint is **c12d59ec1b7d76c7**. Do not delete its registry or stop its host. The immediate-no-answer entries were not classified as dead here, so their precise refusal causes are not asserted. This wave identifies present behavior, not a retrospective trace of the earlier full list.

## Supported controls and concrete SDK branch

Installed `omo-ai` and `@code-yeongyu/omo-senpi` versions are5.1.29; installed engine dependency is `@code-yeongyu/senpi`2026.10.10-11. The installed `bin/lib/thread.js` list command has no value options and only common `--json/--all-scope` flags. Installed engine `dist/cli/host-command.js` rejects unknown options; its status options include `--all`, `--include-workers`, `--socket`, `--json`, not a timeout. The inspected listing/discovery path uses constants, not a supported public timeout env setting. Socket/agent-directory overrides change scope/discovery, not aggregate budget, and narrowing to one endpoint would violate all-scope local discovery.

Release-matching source establishes the more precise duplicate-probe bug:

`packages/omo-senpi/src/components/thread/live-surface.ts:318-325`:

```ts
const listEndpoint = async (endpoint: KnownEndpoint): Promise<EndpointListing> => {
  const tui = endpoint.kind === "tui"
  if (tui && endpoint.verdict.alive === false) return degraded(endpoint, new Error(endpoint.verdict.reason ?? "live_unresponsive"), endpoint.verdict.reason ?? "live_unresponsive")
  // ...
  callOn<{ sessions: ThreadHostSession[] }>(endpoint.socket, "list_sessions", tui ? {} : OBSERVE, tui ? TUI_REQUEST_TIMEOUT_MS : ENDPOINT_LIST_TIMEOUT_MS)
}
```

`ENDPOINT_LIST_TIMEOUT_MS` is10000 at line35, TUI budget1500 at line40, enumeration cache5000 at line42, and fallback engine-status child budget30000 at line43. Installed CLI `bin/lib/thread.js` supplies an `engineStatusAll` function which calls engine host status synchronously through `bin/lib/launcher.js`; that route does not use the fallback execFile timeout. Engine status first probes all endpoints. Then the SDK uses a fresh10s list budget even when an RPC host already has `alive:false`; only TUIs currently reuse the negative verdict. That is the smaller evidence-backed repair boundary.

An additional correctness detail must be included in that repair: `live-surface.ts:295` reconstructs the legacy endpoint with `verdict:{}` even if its engine report carried a verdict. Preserve the reported legacy verdict while retaining the current unknown-verdict behavior when no report exists. Otherwise removing the TUI condition alone still allows the legacy socket to be probed again. No claim is made that c12d59ec1b7d76c7 is that legacy socket.

## Authoritative matching source resolved read-only

The prior source-availability gap is resolved. Public release source is available; no checkout writes, fetch into git, installation or build occurred.

- npm exact metadata: https://registry.npmjs.org/omo-ai/5.1.29 reports `gitHead=6dfb41974556051b432c30ed71b134ed826c883a` and repository `https://github.com/code-yeongyu/oh-my-openagent`.
- GitHub tag `v5.1.29` independently resolves to the same commit: https://api.github.com/repos/code-yeongyu/oh-my-openagent/git/ref/tags/v5.1.29 .
- Exact source: https://github.com/code-yeongyu/oh-my-openagent/blob/6dfb41974556051b432c30ed71b134ed826c883a/packages/omo-senpi/src/components/thread/live-surface.ts . The pinned commit URL is the verified revision; the source blob hash is not recorded here.
- Shipped bundle at that commit: `packages/omo-senpi/plugin/runtime/thread-sdk/sdk.js`, Git blob `b556ac014433bd961df3d03655fa8049d1dd8269`. Its SHA-256 is `4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee`, byte-for-byte equal to the installed SDK read in memory.
- Source entry `packages/omo-senpi/src/extension/thread-sdk.ts` documents compilation into `plugin/runtime/thread-sdk/sdk.js` and imports the component SDK. This verifies the source-to-artifact ownership, not just a similarly named file.
- Existing test precedent: `packages/omo-senpi/src/components/thread/live-surface-tui.test.ts:261-282` injects an engine `alive:false, reason:live_unresponsive` report, checks resumable disk metadata, and asserts zero terminal frames and no dial. Neighboring source tests include `live-surface-endpoints.test.ts`, `live-surface.test.ts`, and `sdk.test.ts`.

Local `/Users/indo/code/oh-my-openagent` searches did not locate this source unit; do not equate that older local tree with the release. `/Users/indo/code/senpi` is version2026.9.23-5 at commit `fe8c564bf33a2cbbdbbba99c9bd8b45b21e37407`, older than the installed engine, and is not the owner of this OMO SDK fix. Authoritative editable base is the OMO repository at the pinned release commit above, to be prepared separately after authorization; not the installed global artifact or active runtime snapshot.

## Minimal feasible patch and deterministic tests

Cheapest observed-path fix is in **one OMO source unit**, `live-surface.ts`, plus neighboring tests:

1. Preserve the engine's liveness verdict on the reconstructed legacy endpoint when present.
2. Reuse an explicit `alive:false` verdict for either endpoint kind in `listEndpoint`, returning its existing degraded/offline result without a second dial. Unknown verdicts retain existing probing. Responsive endpoints remain independently listed.
3. Add deterministic fake-runner/transport tests patterned after the existing TUI test: negative RPC verdict performs no request and preserves reason/disk metadata; negative legacy RPC verdict is not lost; mixed responsive RPC/TUI sessions remain usable; unknown verdict still probes; all unavailable endpoints preserve existing failure vocabulary. Use exact request/connection events or fake-clock advancement, no sleeps or provider calls.

This removes the demonstrated redundant RPC10s pass and leaves the gateway20s contract unchanged. It is not a universal aggregate-time guarantee for arbitrary endpoint populations or engine status delays: a public aggregate-budget feature would be a separate broader upstream contract change if still needed after this minimal repair is verified. Do not declare todo27 green until repaired dependency and actual local+remote gateway behavior pass with cleanup receipts. No dependency upgrade, source patch, active-host manipulation or QA rerun is authorized by this report.

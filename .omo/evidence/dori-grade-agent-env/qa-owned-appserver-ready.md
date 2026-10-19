# Owned ephemeral app-server readiness

Status: verified and stopped. The reusable helper is
`qa-owned-appserver.mjs` in this directory. Verification ran once, with one
authenticated initialize request, zero threads, and zero model turns.

## Invocation and authentication

Installed version observed: `omo 5.1.30 (engine: senpi 2026.10.11)`.
Both `omo app-server --help` and `omo --help` were read. The former prints
the general CLI help in this installation; the specific authentication option
was established from installed source, not inferred from that help output.

Verification command:

```sh
cd /Users/indo/code/project/omon-gateway-wt/dori-work
bun .omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs --verify
```

Exact child argv:

```json
["omo","app-server","--listen","ws://127.0.0.1:51350","--ws-auth","/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-LZADWk/ws-token"]
```

CWD: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
HOME: `/Users/indo`. The child inherits the owner's environment without model
overrides or env-file edits. The owner retains piped child streams and awaits
exit. A dedicated process group permits tree cleanup; the child is not unref'd
and does not use the daemon subcommand.

Authentication uses a freshly generated bearer token in an owned temporary
file with mode 0600 inside a mode 0700 directory. `--ws-auth <path>` makes the
foreground server read that explicit file rather than auto-manage the shared
`~/.omo/agent/app-server/ws-token`. The WebSocket supplies
`Authorization: Bearer <private token>`. No token values are logged or retained
in this evidence. Child output is consumed privately rather than forwarded.

## Isolation source evidence

Installed source root:
`/Users/indo/.bun/install/global/node_modules/@code-yeongyu/senpi/dist`.

- `cli/app-server-command.js`: parsed `server` invokes `runAppServerMode`;
  parsed `daemon` invokes a separate daemon-management path.
- `modes/app-server/index.js`: foreground mode creates a local runtime and
  requested listener, then awaits its own shutdown signal. It does not invoke
  daemon registration, PID-file management, or host registration.
- `modes/app-server/cli-args.js`: accepts `--ws-auth <token-file|off>`.
- `modes/app-server/transports/websocket-auth.js`: explicit token-file auth
  calls `readTokenFile`; only default auth calls `ensureTokenFile` and can
  generate or repair the shared token file.
- `modes/app-server/rpc/registry.js`: method registry is an in-process Map.
  `threads/registry.js` constructs an empty in-process session Map; it creates
  or resumes sessions only on corresponding thread calls.
- `modes/app-server/server/server-core.js`: initialize updates the connection
  and builds its response without creating a session or running a model.
- The installed `omo-ai/bin/lib/launcher.js` forwards `app-server` into this
  foreground path and injects the normal plugin extension; extension loading
  into agent sessions is deferred until a thread is created.

Existing task14, task13, and lane-local-preflight QA helpers were read. They
use `OMON_OMO_APPSERVER_URL` and `OMON_OMO_APPSERVER_AUTH_TOKEN`, with a bearer
header for initialize. This helper exposes those same variables privately.
No shared daemon command was executed, including status (which the installed
implementation acquires a shared lock for). No shared registry or running
session was manipulated. No product source, Cargo, Discord, env file, or git
operation was used.

## Readiness and protocol receipt

The helper first obtains a free loopback port by an exclusive port-zero bind,
then releases it for the child. If another process wins the bind race, startup
fails; it never stops the occupant or falls back to a shared endpoint.
An EventEmitter readiness subscription is established before spawn. Piped
stdout/stderr are consumed for the exact listener line:
`senpi app-server listening on ws://127.0.0.1:51350`.
There is no fixed sleep or polling loop; deadlines only bound event waits.

Started: `2026-10-11T23:52:16.220Z`.
Finished: `2026-10-11T23:52:16.607Z`.
Elapsed measurements use the monotonic clock: readiness 337 ms, initialize
3 ms, cleanup 50 ms, total 390 ms (includes port selection and token setup).

Request:

```json
{"jsonrpc":"2.0","id":"qa-owned-initialize-1","method":"initialize","params":{"clientInfo":{"name":"qa-owned-appserver","version":"1"}}}
```

Response:

```json
{"id":"qa-owned-initialize-1","result":{"userAgent":"qa-owned-appserver/2026.10.11 (Darwin 27.0.0; arm64) senpi_app_server","codexHome":"/Users/indo/.omo/agent","platformFamily":"unix","platformOs":"macos"}}
```

Request and response IDs match. Bearer-authenticated WebSocket upgrade and
initialize succeeded. The client closed its socket before server shutdown.

## Cleanup receipt

```json
{"pid":28770,"membersBeforeShutdown":[28770],"remainingGroupPids":[],"pidAbsent":true,"childExit":{"code":0,"signal":null},"portRebindSucceeded":true,"privateScratchRemoved":true}
```

SIGTERM was sent only to the owned group. Its child exited with code 0;
SIGKILL was unnecessary. `kill(pid, 0)` returned ESRCH, process-group inspection
found no remaining members, and an exclusive loopback rebind on port 51350
succeeded and was closed. The private scratch directory and token were removed.
The verification monitor exited with code 0.

## Caller integration

```js
import { withOwnedAppServer } from './qa-owned-appserver.mjs';

await withOwnedAppServer(async ({ url, gatewayEnv, receipt }) => {
  // Merge gatewayEnv into the owned QA gateway's environment.
  // Complete QA, stop that gateway, and close its sockets before returning.
  // url and receipt contain no bearer token; never log gatewayEnv.
});
```

The callback is the server lifetime boundary. Cleanup also runs on callback
failure and SIGINT/SIGTERM; a synchronous exit hook kills only the owned
process group. `gatewayEnv` is non-enumerable on the handle to avoid accidental
handle serialization leaking credentials. Future real-turn QA remains to be
run by its caller; this verification intentionally did not test model routing
or credential validity through a model request.

`node --check` passed. LSP diagnostics could not initialize because the
workspace lacks a valid TypeScript installation; no dependency was installed
to alter that workspace. Actual Bun execution and all lifecycle assertions
passed in the single verification run.

# Bounded owned lane scope probe

`lane-scope-probe.mjs` accepts one absolute path to an actual fresh `omon-lane-qa-prepared-v1` receipt. It is a runtime client, not a preparation or gate command. Invoke only after the parent releases the exact source/binary gate and that fresh receipt. No model/provider routing overrides are sent.

Current separately released probe argv:

```text
/Users/indo/.bun/bin/bun
--no-env-file
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scope-probe.mjs
/private/tmp/omon-lane-scratch-qa-zarZ0e/prepared.json
```

Parent-supplied identities, to be checked against actual bytes before invocation:

- prepared SHA256: `46d9d839566922b0e54530c99bea70d073a4bdc830e6d0e45f22e4d1782db33e`
- binary SHA256: `dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e`
- manifest SHA256: `b617c3303f2ad0ba4286a6013d31f06c5bb2745e73946f9b7f0bb98169a73efe`

These are release inputs, not scope observations. Earlier pibpnv preparation stays unreleased and must not be rewritten or executed.

## Input and private authentication

The client validates canonical private root/repo/home/bin, exact scratch DB URL, empty dotenv, fixed worktree binary/source paths, binary/judge/manifest hashes and symlink identities, every manifest input, complete prepared environment, no DB/sidecars and no execution marker. It refuses an existing verified scope. The legacy failed receipt is preserved and permits a separately authorized changed attempt only when it proves no recorded RPC/thread/turn. Each changed attempt uses `scope-probe-receipts-<timestamp>-<nonce>.json`, exclusive creation mode0600 inside the owned root. It does not open a database, initialize a gateway, or invoke lane commands. Never retry automatically or rerun an attempt that established a remote thread without explicit disposition.

Private auth precedence matches `OmoConfig`: `OMON_OMO_APPSERVER_AUTH_TOKEN` if present, otherwise `OMON_OMO_AUTH_TOKEN`, then trim. A present empty primary masks the fallback. A nonempty token becomes the WebSocket Authorization Bearer header only; never argv or receipt content. The client does not read Discord or TYPESAFE credentials. Keep inherited private auth available to the client without printing values or environment dumps. Empty auth is not a claim that the live daemon disables auth; an authentication rejection fails the probe.

## Protocol and observations

Bun's built-in WebSocket connects only to `ws://127.0.0.1:19742`. No daemon lifecycle command is implemented. Message listeners are installed before initialize/thread/start/turn/start. RPC requests have unique numeric IDs; responses match pending IDs. `thread/start` sends cwd and runtimeWorkspaceRoots from the prepared repo, with no model override. Its returned cwd/roots must match; its actual thread ID is retained. The daemon automatically subscribes that connection in thread/start (`threads/handlers.js`).

Turn events are correlated by actual thread ID and actual turn ID from accepted/start events. The client accepts observations only from `item/completed` commandExecution with the exact requested command, matching cwd, completed status and exitCode0. It ignores agent/user/reasoning text. A random nonce prefix identifies exactly one machine JSON line in aggregatedOutput; prose cannot satisfy the contract. Other tool types, changed command, missing output, duplicate results or unexpected approvals fail closed. Approval requests are denied.

The exact backend bash command invokes `/Users/indo/.bun/bin/bun --no-env-file -e` with generated code. That code reads canonical effective cwd and inherited DATABASE_URL equality as a boolean without printing the URL value; hashes the exact released binary; then runs `[prepared.binary, "--help"]` through Bun.spawn with explicit prepared env and repo cwd. It awaits exit/stdout/stderr and prints only structured identity/boolean/hash/exit/byte-count fields. No lane/DB command, delegation, Discord, external network or model override is in this tool payload. The tool's actual results, not the final agent answer, become evidence.

Gateway client env does not automatically propagate into shared daemon tools. A false inherited URL equality is recorded honestly and is acceptable only with exact explicit override child identity, envOverride=true, databaseUrlEqual=true, matching binary hash, exitCode0 and nonempty help output. The subprocess environment assignment is observed in the backend code's actual Bun.spawn invocation, not inferred from a gateway setting. Thread/start cwd/roots are independently checked against daemon responses and the tool cwd.

## Bounds and disposition

Connection/RPC setup has 15-second bounds; turn/start acknowledgement45 seconds; terminal observation120 seconds; --help child10 seconds. Timers are cleared when each bound settles. On failure with an owned running turn ID, the client sends only that thread/turn's `turn/interrupt`, reconnecting to the same existing daemon if needed, and bounds acknowledgement and terminal notification by10 seconds each. It never stops a service. Unknown submitted turn identity or unconfirmed cancellation is recorded as failure, never converted to verified scope.

The client closes its own WebSocket and retains the owned remote thread ID. On success it requires a completed terminal notification and one valid machine result, rechecks DB absence and binary identity, writes an attempt-specific receipt, then `scope-probe.json` with schema `omon-lane-qa-scope-probe-v1`. Scope fields include observed cwd/roots/thread/time, inherited equality and explicit child result; the release consumer uses that entire scope object and adds its hashed evidence member. Failure creates only private sanitized receipts, no verified scope artifact. Connection receipts now preserve precise construct/open/initialize stage, open/error/close/timeout events, error code and sanitized message, and auth presence only. Private token/header contents are redacted, never serialized. Success leaves the completed remote probe thread idle/retained; no archive/delete/service stop is performed. A retained thread is not proof that all remote tools anywhere have terminated.

Read-only recovery check after the first failed attempt: `/usr/sbin/lsof -nP -iTCP:19742 -sTCP:LISTEN -Fpcn` returned exit1, empty stdout/stderr: no listener. Current evaluation environment has neither auth variable nor endpoint variable. Main repository `.env` has no OMON_OMO_APPSERVER_URL or either auth assignment; dori-work `.env` is absent. Prepared config still specifies ws://127.0.0.1:19742; inspected owner config provides no different supported endpoint. This is a concrete external listener blocker, not proven authentication rejection or WebSocket constructor misuse. No changed runtime probe was launched while the endpoint lacked a listener, and no service/configuration change was made.

Static verification must parse without running this module or importing it. Runtime QA of this client is restricted to the one separately authorized harmless probe. Task8/9 execute release remains separate; this client never invokes execute-8-9.

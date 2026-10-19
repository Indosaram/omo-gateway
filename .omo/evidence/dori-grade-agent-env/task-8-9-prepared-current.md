# Tasks 8/9 fresh preparation receipt

Result: **PREPARED_RUNTIME_UNRUN; RUNTIME_UNRELEASED**. Preparation completed on 2026-10-11 with exit 0 and empty stderr. Fresh owned root: `/private/tmp/omon-lane-scratch-qa-pibpnv`. No validate, execute, gate release, scope probe, gateway, judge execution, database open, lane command, model/backend request, network or Discord operation occurred. Existing daemon19742 was neither contacted nor changed. No Cargo, git, product edit, service control or owner environment/config modification occurred.

## Inspected safety boundary

Read `lane-scratch-qa.md`, the prepare branch and runtime boundary in `lane-scratch-qa.mjs`, and the complete imported `lane-scratch-qa.sink.mjs`. The sink module defines a function but performs no top-level I/O. The bun:sqlite import does not instantiate a database. In prepare mode, the harness checks binary bytes/hash, source manifest entries, native arm64 judge shape and executable permission; creates the private root/directories, empty dotenv, executable symlinks, manifest copy and prepared.json; prints its result and calls process.exit(0). It exits before release parsing, credential access, execution marker creation, child spawning, database construction and runtime scenarios. Bun.which performs only PATH lookup. Therefore this inspected prepare invocation does not reach prohibited operations.

The runtime execute branch also includes task12 Discord sink operations. No authorization for those operations is implied by this receipt. This preparation neither executes nor alters that branch.

## Actual supported prepare invocation

```json
[
  "/Users/indo/.bun/bin/bun",
  "--no-env-file",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs",
  "prepare",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-pre-manifest.sha256",
  "/Users/indo/.cargo/bin/typesafe-judge",
  "90fd6614354a83a71712101a9cb37c50ccc28dfa64cb249b9f803d6d652c7762"
]
```

Spawned via explicit Bun.spawn argv with cwd `/private/tmp` and complete child env `{"PATH":"/usr/bin:/bin:/usr/sbin:/sbin"}`. No owner env or credentials inherited. A 15-second failure deadline was armed and cleared after normal completion; the child was awaited. No persistent child remains from preparation.

Exact stdout:

```json
{"result":"PREPARED_RUNTIME_UNRUN","preparedPath":"/private/tmp/omon-lane-scratch-qa-pibpnv/prepared.json","root":"/private/tmp/omon-lane-scratch-qa-pibpnv","dbPath":"/private/tmp/omon-lane-scratch-qa-pibpnv/qa.db","binarySha256":"90fd6614354a83a71712101a9cb37c50ccc28dfa64cb249b9f803d6d652c7762","manifestSha256":"b617c3303f2ad0ba4286a6013d31f06c5bb2745e73946f9b7f0bb98169a73efe"}
```

Actual exit code 0; stderr zero bytes. Prepared createdAt: `2026-10-11T14:57:07.926Z`.

## Pinned artifacts, not release proof

| Artifact | Path | SHA-256 |
| --- | --- | --- |
| Actual prepared receipt | `/private/tmp/omon-lane-scratch-qa-pibpnv/prepared.json` | `0717db06e22d894771192341955c512583e3fd9e87e6679df31b614b99e3ad03` |
| Measured gateway binary | `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway` | `90fd6614354a83a71712101a9cb37c50ccc28dfa64cb249b9f803d6d652c7762` |
| Existing work-port source manifest | `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/race-port-20261011T145201Z-pre-manifest.sha256` | `b617c3303f2ad0ba4286a6013d31f06c5bb2745e73946f9b7f0bb98169a73efe` |
| Exact copied manifest | `/private/tmp/omon-lane-scratch-qa-pibpnv/source-manifest.sha256` | `b617c3303f2ad0ba4286a6013d31f06c5bb2745e73946f9b7f0bb98169a73efe` |
| Native judge | `/Users/indo/.cargo/bin/typesafe-judge` | `daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6` |

All 191 manifest inputs matched current source bytes during the read-only precheck; the harness independently enforced its manifest checks. The binary measured 107805248 bytes with mtime `2026-10-11T14:44:46.316Z`, earlier than the selected work-port manifest. Hashing an existing binary and a matching current manifest does not establish that this binary contains the port. Worker67's exact-binary gate is still running per coordinator context. This binary may be stale; no source-to-binary release or successful focused race repair is claimed. No gate or backendScope artifact was manufactured.

## Actual root and isolation inspection

The complete credential-free prepared.json was read through the read tool. Its schema is `omon-lane-qa-prepared-v1`; root and repo paths are canonical. Root, `home`, `qa-repo`, and `bin` each have mode0700. Root inventory is exactly `.env`, `home`, `bin`, `qa-repo`, `prepared.json`, `source-manifest.sha256`. Home and qa-repo are empty; no git initialization occurred. `.env` is exactly zero bytes, mode0600; prepared.json is1944 bytes, mode0600; manifest copy is17697 bytes, mode0600.

Bin contains only two symlinks: omo-gateway resolves to the measured work binary above; typesafe-judge resolves to the pinned native judge above. Shared executable targets are not owned scratch files to delete. Under the prepared PATH, Bun.which returned null for zele, imsg and omo.

Prepared environment identities:

```json
{
  "HOME": "/private/tmp/omon-lane-scratch-qa-pibpnv/home",
  "USER": "lane-qa",
  "LOGNAME": "lane-qa",
  "TMPDIR": "/private/tmp/omon-lane-scratch-qa-pibpnv",
  "PATH": "/private/tmp/omon-lane-scratch-qa-pibpnv/bin:/usr/bin:/bin:/usr/sbin:/sbin",
  "XDG_CONFIG_HOME": "/private/tmp/omon-lane-scratch-qa-pibpnv/home/.config",
  "XDG_DATA_HOME": "/private/tmp/omon-lane-scratch-qa-pibpnv/home/.local/share",
  "XDG_CACHE_HOME": "/private/tmp/omon-lane-scratch-qa-pibpnv/home/.cache",
  "DATABASE_URL": "sqlite:///private/tmp/omon-lane-scratch-qa-pibpnv/qa.db",
  "OMON_DISCORD_ENABLED": "false",
  "OMON_LANES_ENABLED": "true",
  "OMON_KIBITZER_ENABLED": "false",
  "DASHBOARD_ENABLED": "false",
  "OMON_WORKSPACE_ROOT": "/private/tmp/omon-lane-scratch-qa-pibpnv/workspaces",
  "OMON_LANE_KEYWORDS": "",
  "OMON_OMO_APPSERVER_URL": "ws://127.0.0.1:19742",
  "OMON_PIRATETALK_URL": "http://127.0.0.1:1",
  "OMON_INBOX_KAKAO_ROOMS": "",
  "RUST_LOG": "info",
  "NO_COLOR": "1"
}
```

DATABASE_URL equals the explicit new-root scratch target and no owner repository DB path is present in the prepared env. This is local prepared-env isolation, not a claim about the shared daemon's inherited environment or propagation into backend tools. Backend scope and effective remote cwd/roots remain UNRUN. Endpoint strings are configuration only; no connectivity check occurred. XDG/workspaces path identities are configured under the root but these nested directories were not created by prepare.

Read-only lstat checks returned ENOENT for qa.db, qa.db-wal, qa.db-shm, qa.db-journal and execution-started.json. No DB was opened to establish absence. Only the prepare harness performed its authorized scratch file creation; this receipt was added with apply_patch. The root remains intact and unconsumed.

## Ownership and exact future release boundary

The newly owned root is intentionally retained for coordinator handoff. Retain it through separately authorized scope probing and QA; eventually remove only this root after retaining evidence. No historical root, owner DB, backup, shared executable or daemon is eligible for cleanup under this ownership.

Before a next probe is released, worker67's exact-binary gate must establish the real source/binary identity. If the binary or source manifest changes, this preparation's hashes are no longer a usable release identity: use the existing contract to prepare a fresh root for the actual released identities rather than editing prepared.json or inventing scope/gate fields. Do not execute validate/execute using this markdown receipt as a release. No scope or runtime operation is authorized by this handoff; those require the separate coordinator release specified by the user.

# Todos 8/9/12 actual prepared-root receipt

Result: **PREPARED_RUNTIME_UNRUN**. Authorized prepare-only execution completed with exit 0 and empty stderr. Actual backend scope, judge execution, gateway startup, lane scenarios, daemon readiness, Discord delivery and DB identity probes remain **UNRUN**. No credentials were read or supplied; no gateway, daemon, Discord or DB operation occurred. No Cargo or product source/test/config edit occurred.

## Remeasured identities before preparation

| Artifact | Absolute path | SHA-256 | Bytes |
| --- | --- | --- | --- |
| Binary | `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway` | `dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec` | 107805248 |
| Canonical manifest | `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-pre-manifest.sha256` | `5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777` | 17697 |
| Native judge | `/Users/indo/.cargo/bin/typesafe-judge` | `daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6` | 3089200 |

Preparation verified every manifest entry against current dori-work source files, the supplied binary hash and native arm64 Mach-O judge shape/executable permission. It copied the manifest and pinned the judge hash. It did not execute either binary or judge.

## Actual invocation and result

Exact argv:

```json
[
  "/Users/indo/.bun/bin/bun",
  "--no-env-file",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs",
  "prepare",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-pre-manifest.sha256",
  "/Users/indo/.cargo/bin/typesafe-judge",
  "dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec"
]
```

Child cwd: `/private/tmp`. Explicit child env contained only `PATH=/usr/bin:/bin:/usr/sbin:/sbin`; no credentials or owner env were inherited. A 15-second failure deadline was armed and cleared after normal completion; the child was awaited. Bun version reported by the subsequent rejection was 1.4.0, macOS arm64.

Prepare stdout:

```json
{
  "result": "PREPARED_RUNTIME_UNRUN",
  "preparedPath": "/private/tmp/omon-lane-scratch-qa-ZyXFMr/prepared.json",
  "root": "/private/tmp/omon-lane-scratch-qa-ZyXFMr",
  "dbPath": "/private/tmp/omon-lane-scratch-qa-ZyXFMr/qa.db",
  "binarySha256": "dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec",
  "manifestSha256": "5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777"
}
```

Prepared creation timestamp: `2026-10-11T14:17:39.955Z`.

Exact prepared receipt SHA-256: `d2753dd317009365ad22c595ab33365aef62efc61a56c0498d8463a31dde4588`.

## Artifact inspection and isolation

The complete prepared.json was read. Root, home, qa-repo and bin modes are 0700; `.env`, prepared.json and source-manifest.sha256 modes are 0600. `.env` is exactly zero bytes. Home and qa-repo are empty; no git initialization occurred. Root inventory is exactly `.env`, `home`, `bin`, `qa-repo`, `prepared.json`, `source-manifest.sha256`.

Scratch bin symlinks point to the exact binary and native judge paths above. Copied source-manifest.sha256 hash equals the canonical manifest hash. DATABASE_URL in the credential-free prepared env is `sqlite:///private/tmp/omon-lane-scratch-qa-ZyXFMr/qa.db`. All HOME/XDG/TMPDIR/workspace paths are under this exact root. Supported Discord-free collector bypass remains configured: OMON_DISCORD_ENABLED=false, OMON_LANES_ENABLED=true, dashboard/kibitzer false, existing daemon endpoint `ws://127.0.0.1:19742`. Configuration is not a connectivity or environment-propagation probe.

Filesystem existence checks confirm qa.db, qa.db-wal, qa.db-shm, qa.db-journal and execution-started.json are absent. No database was opened to check this.

## Actual incomplete-receipt rejection

Using apply_patch, created only the owned throwaway `task-8-9-12-incomplete-release-throwaway.json` under dori-work evidence. It contained the actual prepared/root/DB/binary/manifest identities, the correct release schema, `gate.status="NOT_RELEASED"` and `backendScope=null`. It was deliberately incomplete and never represented success or forged scope evidence.

Exact rejection argv:

```json
[
  "/Users/indo/.bun/bin/bun",
  "--no-env-file",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs",
  "validate",
  "/private/tmp/omon-lane-scratch-qa-ZyXFMr/prepared.json",
  "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-8-9-12-incomplete-release-throwaway.json"
]
```

Same cwd and credential-free env as prepare; bounded by a 15-second failure deadline. Actual exit 1; stdout empty. Stderr recorded `AssertionError: Gate not released` at `lane-scratch-qa.mjs:93:8`, actual `NOT_RELEASED`, expected `released`. This proves the incomplete receipt was rejected after actual prepared-root/identity checks and before runtime. It does not independently exercise missing-scope rejection, because the gate rejection occurs first.

The throwaway was deleted through apply_patch; its absence was verified. The prepared.json hash and root inventory remained unchanged after rejection. All DB sidecars and execution marker remained absent. No successful release or scope receipt was created. An initial inspection cell could not find its conditional result variable; it was reconstructed from the already-completed prepare stdout without rerunning preparation.

## Ownership and handoff

The sole owned scratch root `/private/tmp/omon-lane-scratch-qa-ZyXFMr` is intentionally preserved for the subsequent authorized exact-root backend scope probe and QA. This track owns its preparation artifacts and cleanup responsibility; handoff to the coordinator transfers responsibility to retain it through probe/release/QA and eventually remove only this exact owned root after evidence retention is settled. Nothing outside this root is eligible for scratch cleanup. Binary/judge symlinks are references to shared executables, not owned executables to delete.

Both local harness children exited and were awaited. No persistent process, socket, observer, daemon, DB handle or network operation was created. The only removed file was the throwaway receipt. Main checkout owner WIP and product files remained read-only.

Later scope evidence must target `/private/tmp/omon-lane-scratch-qa-ZyXFMr/qa-repo`, be recorded after the preparation timestamp, and explicitly distinguish observed inherited DATABASE_URL equality from explicit control-child override equality. Use only the existing daemon19742. After real probe and gate disposition, the coordinator must produce the structured release receipt described in lane-scratch-qa.md and invoke validate/execute against the exact prepared.json path above. This receipt is not that release and must not be passed as one. Actual scope and all live QA remain UNRUN.

# Prepared scratch QA driver: todos 8, 9, 12

Source-settled preparation contract. No gateway, backend, Cargo, Discord request or QA scenario was executed in this preparation track. Actual task-12 sink and backend scope evidence remain UNRUN.

## Prepare, probe, validate, execute

Preparation creates a private exact root, empty dotenv, home/repo/bin, executable symlinks, source-manifest copy and credential-free `prepared.json`, then exits before any subprocess, database open, gateway, lane, daemon or Discord operation. It verifies the supplied binary hash, every supplied manifest entry and the native judge identity first. It does not initialize git. Preparation requires no credentials. The returned root is retained for the later probe and execute phase; execute never generates a replacement root.

Concrete preparation argv for the binary/manifest inspected on 2026-10-11 (recheck release identity if gates replace either):

```text
/Users/indo/.bun/bin/bun
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs
prepare
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T135508Z-pre-manifest.sha256
/Users/indo/.cargo/bin/typesafe-judge
dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec
```

These are individual argv elements, not a shell script. The manifest's measured SHA-256 is `5525bd6e6b1eed943005db6584d4e5b1d353acf2d6bfd17cf38c78965dcf5777`. Its complete listed inputs are checked both at prepare and validate/execute. This verifies the pinned list, not provenance of unlisted inputs; the coordinator supplies the complete released manifest. The earlier preflight report is historical; its exact-root/unchecked-receipt findings are superseded by this contract, while gate disposition and actual runtime probes remain outstanding.

After prepare, the coordinator can perform the previously authorized harmless owned backend scope probe against the returned `root` and `root/qa-repo` on the single existing daemon19742, without creating/opening qa.db. Capture actual effective cwd/roots and inherited DATABASE_URL equality as booleans, plus a harmless control-child environment check using the exact binary identity and explicit DATABASE_URL override. The control child must report equality and exit 0 without invoking lane commands or opening a DB. This is evidence of explicit control-child override, not proof the gateway propagates environment to daemon tools. The probe is external to this driver and remains UNRUN here. Do not fabricate its receipt from this example.

Record a private JSON artifact **inside that exact root** with `schema: "omon-lane-qa-scope-probe-v1"` and `scope` equal to the `backendScope` object below excluding its `evidence` member. Attach real observations, command/exit evidence and probe identifiers in additional artifact fields without credentials. Hash the completed artifact. The release receipt must use this structure; all values below are illustrative placeholders, not runnable evidence:

```json
{
  "schema": "omon-lane-qa-release-v1",
  "preparedSha256": "SHA256_OF_EXACT_PREPARED_JSON",
  "root": "/private/tmp/omon-lane-scratch-qa-EXACT",
  "dbPath": "/private/tmp/omon-lane-scratch-qa-EXACT/qa.db",
  "databaseUrl": "sqlite:///private/tmp/omon-lane-scratch-qa-EXACT/qa.db",
  "binary": "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
  "binarySha256": "EXACT_PREPARED_BINARY_SHA256",
  "manifestSha256": "EXACT_PREPARED_MANIFEST_SHA256",
  "gate": {
    "status": "released",
    "evidence": {"path": "/absolute/coordinator-gate-disposition-receipt", "sha256": "ACTUAL_FILE_SHA256"}
  },
  "backendScope": {
    "status": "verified",
    "endpoint": "ws://127.0.0.1:19742",
    "root": "/private/tmp/omon-lane-scratch-qa-EXACT",
    "dbPath": "/private/tmp/omon-lane-scratch-qa-EXACT/qa.db",
    "cwd": "/private/tmp/omon-lane-scratch-qa-EXACT/qa-repo",
    "runtimeWorkspaceRoots": ["/private/tmp/omon-lane-scratch-qa-EXACT/qa-repo"],
    "threadId": "ACTUAL_OWNED_PROBE_THREAD_ID",
    "observedAt": "ACTUAL_ISO_TIMESTAMP_NOT_BEFORE_PREPARATION",
    "inheritedDatabaseUrlEqual": false,
    "controlChild": {
      "executable": "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
      "cwd": "/private/tmp/omon-lane-scratch-qa-EXACT/qa-repo",
      "databaseUrl": "sqlite:///private/tmp/omon-lane-scratch-qa-EXACT/qa.db",
      "databaseUrlEqual": true,
      "envOverride": true,
      "exitCode": 0
    },
    "evidence": {"path": "/private/tmp/omon-lane-scratch-qa-EXACT/scope-probe.json", "sha256": "ACTUAL_FILE_SHA256"}
  }
}
```

`inheritedDatabaseUrlEqual` must be the observed boolean, not forced true. Explicit override equality must be true. Release fields must match the exact prepared JSON, scope artifact fields must match the release, and both evidence hashes must match existing files. JSON shape checks cannot authenticate an operator's observations: the coordinator must independently review actual probe evidence and gate disposition. Merely setting `verified` does not make a runtime claim true.

Read-only release-contract validation argv:

```text
/Users/indo/.bun/bin/bun
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs
validate
<absolute returned root/prepared.json>
<absolute actual structured release.json>
```

Validation needs no credentials and exits before subprocesses or DB access. It rejects malformed/arbitrary receipts, mismatched root/DB/binary/manifest/prepared identity, changed dotenv/env/judge, missing or mismatched scope artifacts, absent equality evidence and previously consumed roots. To execute later, use the identical argv with `execute` (tasks 8/9/12) or `execute-8-9` (tasks 8/9 only) instead of `validate`; no judge/hash argv is re-supplied, because those identities are pinned in prepared.json. Both modes retain all validation, verified scratch database checks and finally cleanup. After all validation and required private env checks, execution writes a one-use execution marker before its first subprocess. A failed actual execution cannot be rerun in the same root; prepare and probe a new one rather than replay uncertain sends.

Both execution modes require existing private TYPESAFE_API_KEY, TYPESAFE_URL and TYPESAFE_MODEL, plus OMON_OMO_APPSERVER_AUTH_TOKEN or OMON_OMO_AUTH_TOKEN if needed by the existing daemon. Only full `execute` requires or copies DISCORD_BOT_TOKEN. `execute-8-9` never reads or copies that token, never enters task12, never constructs the Discord sink or writable binding handle, and leaves sink/binding undefined so finally performs no Discord restore call. Credentials are copied only after receipt validation, never serialized into prepared.json, argv or summaries. No runtime probe occurs during prepare or validate.

Task8/9-only future argv (individual elements; not executed by source adaptation):

```text
/Users/indo/.bun/bin/bun
--no-env-file
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs
execute-8-9
<absolute freshly prepared root/prepared.json for the exact released binary>
<absolute actual structured release.json for that same root>
```

The last two paths must come from fresh exact-binary preparation and separately released actual scope/gate evidence; no current release path is available. Stale root `/private/tmp/omon-lane-scratch-qa-pibpnv` remains retained and unreleased, must not execute, and must not be rewritten to impersonate new preparation. No new daemon or Discord QA is part of `execute-8-9`. It still performs actual scratch lane/backend/judge operations when separately authorized; this option is not a runtime release. Scratch evidence remains retained after owned debug gateway/observer/DB cleanup, as in full mode.

The driver uses explicit Bun.spawn argv/env, the named existing daemon at 19742, isolated HOME/XDG/cwd/config, and an absolute scratch SQLx URL. Empty scratch dotenv prevents ancestor discovery. Gateway database-open and service-ready events, independent lsof descriptors, and scratch PRAGMA database_list must pass before any lane command/query. No inherited repository DB is opened or queried. Owner-fixed purpose models have no overrides.

Current supported `OMON_DISCORD_ENABLED=false` startup bypasses the inbox collector (`src/main.rs:646-675` returns before collector startup at :693). Keep it false for task 12 too: the lane sink is initialized independently. OMON_PIRATETALK_URL and OMON_INBOX_KAKAO_ROOMS are supported additional settings, not invented collector-disable switches; empty rooms alone would still fetch /api/rooms. No interval override is introduced. CLI-shaped evidence is argv/exit/raw private output; HTTP evidence uses curl -i. Credentials travel privately, never in argv or summary.

## Judge interpreter and whitelist inspection

Read-only inspection found `/Users/indo/.cargo/bin/typesafe-judge` is executable native arm64 Mach-O, SHA-256 `daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6`, with loader `/usr/lib/dyld` and dependencies `/usr/lib/libiconv.2.dylib` and `/usr/lib/libSystem.B.dylib`. It has no shebang or Bun/Node interpreter requirement. Prepare rejects script interpreters rather than silently expanding PATH. The existing judge symlink in scratch/bin plus `/usr/bin:/bin:/usr/sbin:/sbin` is sufficient for this inspected executable's interpreter/loader requirements; no `.cargo/bin`, `.bun/bin`, Node directory, interpreter env variable or owner env change is required. This does not prove judge authentication/network execution, which remains UNRUN.

The exact private env whitelist is the three TYPESAFE entries and two optional daemon auth entries, with DISCORD_BOT_TOKEN added only for full `execute`. The non-secret whitelist is the existing driver env object: scratch HOME, USER/LOGNAME, TMPDIR, PATH, XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_CACHE_HOME, DATABASE_URL, OMON_DISCORD_ENABLED, OMON_LANES_ENABLED, OMON_KIBITZER_ENABLED, DASHBOARD_ENABLED, OMON_WORKSPACE_ROOT, OMON_LANE_KEYWORDS, OMON_OMO_APPSERVER_URL, OMON_PIRATETALK_URL, OMON_INBOX_KAKAO_ROOMS, RUST_LOG and NO_COLOR. No owner env is changed. The daemon inherits neither this PATH nor this DATABASE_URL automatically.

## Scenario wiring

8: start qa-1 WITHOUT --thread; require real turn, done.txt, report and claim. Invalid done command must exit 2 and create no qa-2 row.

9: force-close/reopen, require same stored thread ID, preserved original contents plus appended line and new agent transcript references; nonexistent reopen must exit 2.

12: reopen qa-1 with question-only guidance. Before answering, require blocked status, exactly one question event, turns_run=1, last_turn_outcome=completed, and an outcome transcript ID. Current runner writes completed outcome and incremented turn count together after the backend turn returns. Capture the reopen generation before any HTTP call. This is an event-driven completed-turn boundary, not a sleep-based pause guess.

The parent authorized a narrow scratch-only binding. The driver now wires the exact existing QA thread `1558726010181132309`, parent `1529738911377657967`, through the following sequence:

1. Exact bot GET validates thread ID, parent, type, archive metadata and unlocked state. Unarchive only this existing QA thread if initially archived. These steps occur BEFORE binding or triggering.
2. Recheck blocked state and unchanged generation. Inspect actual scratch `PRAGMA table_info(lanes)` immediately before mutation. Actual schema columns are `thread_channel_id` and `parent_channel_id`, with lane primary key `key`; there is no assumed discord_thread_id column.
3. Open a separate write handle using only the already-verified absolute scratch pathname, create:false, and independently verify its main database path. One UPDATE binds both destination columns with CAS predicates for key qa-1, status blocked, turns_run=1, completed outcome, initially NULL destinations, and unchanged reopen generation. Require exactly one affected row. Store previous values and outcome/generation IDs. Never mutate any shared session or product DB.
4. Register DB/WAL/log event predicates BEFORE actual `lane object qa-1 --reason` guidance. Answer A, request exactly one unique curated milestone, append A, then claim and stop. No --thread CLI argument, starter post, new channel, nested thread, generic POST transport, or arbitrary destination is used.
5. Inspect only new `lane_transcript` thread_post rows after the binding boundary. Require exactly one, exact expected body and thread, state sent; uncertain sends fail without retry. GET its actual message ID and require channel, body and nonce equality. Summaries contain counts, IDs, nonce and body byte length, not body/credentials. The helper records private curl -i response artifacts.
6. Require the answered turn completed and claimed on the same stored backend thread, check there is still exactly one new post, restore previous NULL destination values through key/destination/generation CAS, then restore the original archive state. No question or unrelated scenario runs while bound.

Earlier no-destination post rows retain their serialized NULL destinations and are not rewritten. The binding changes only the owned lanes row, never historical post payloads. SQL mutation is explicitly scratch QA setup, not a claim that native CLI supports existing-thread attachment. Source inspection showed native --thread creates a new thread, so it is intentionally absent.

## Cleanup and limits

Every spawned child's `exited` promise is subscribed immediately and its actual numeric code is recorded before removal from the live-child registry. Gateway startup races the bounded service-ready observation against that exact exit promise. Early exit drains both streams, including a final line without a newline, records the startup exit and log hash, and fails immediately. Final cleanup records gateway exit and PID absence even if it exited before cleanup. Descriptor and PRAGMA checks still precede every lane operation, and the one-use execution marker is unchanged.

The 2026-10-11 robustness regression exercised these actual spawn/readiness functions with an owned `/bin/sh` child printing `immediate_failure` to stderr and exiting23. It observed exit23, captured stderr and rejected readiness through process exit rather than its timeout, with no remaining live child. No gateway, backend, database, judge, Discord or model operation ran in that regression. Historical GUgT9Z gateway exit was not recorded by the old harness and remains unknown; the new code does not repair that historical fact.

Finally cancels outstanding observation subscriptions, force-closes only verified scratch lanes, terminates/awaits only owned process handles, and verifies their PIDs are absent. After an incomplete turn the owned gateway is stopped before binding restoration. Destination restoration uses saved prior values and a generation CAS; failures are recorded rather than broadening writes. Archive restoration is attempted even when binding or delivery failed. The shared backend and existing services are never stopped. Private scratch artifacts are retained; no destructive git operations.

The sink helper imports without side effects. Every HTTP command has a 30-second maximum and is awaited; its child uses the driver's owned-process registry. The only timers in scenario logic are bounded failure deadlines, not success-by-delay. No actual sink, local scenario or fake three-error case passed during preparation. The latter requires the coordinated test_lane_events result separately. Runtime receipts are not a full manualQa verdict until all named cases and cleanup artifacts have been independently reviewed.

This revision edits only lane-scratch-qa.mjs and lane-scratch-qa.md through apply_patch. The sink helper and scenario body remain unchanged. No evidence-harness test convention exists in the inspected directory, so no new test file is introduced. Bounded local CLI rejection probes are the validation surface; real runtime scenarios, release/probe success, judge calls and delivery remain UNRUN. LSP cannot initialize because no TypeScript installation is available; no dependency is installed under the source freeze.

### Preparation validation receipt

JavaScript syntax/import scanning through Bun.Transpiler passed. Five actual local CLI rejection probes ran using explicit `/Users/indo/.bun/bin/bun --no-env-file`, cwd `/private/tmp`, a credential-free system-only env and a 5-second failure deadline. Missing mode, invalid prepare hash, wrong binary hash, arbitrary Markdown as prepared JSON and the obsolete execute argv each exited 1 with the expected rejection and empty stdout. Each rejected before scratch creation, database access or child-process invocation by the driver. All probe children exited and were awaited.

The actual release-validation block was also exercised in memory with synthetic file/hash bindings: one structurally valid fixture passed; arbitrary schema, root mismatch, manifest mismatch, missing backend scope, missing control-child equality and mismatch between release scope and recorded probe artifact all rejected deterministically. These are predicate checks, not real scope or disk-reuse verification. Initial fixture invocations failed because the validation bindings omitted the binary/dbPath variables; those fixture bindings were corrected, with no product or driver repair needed. Successful prepare, real prepared-root reuse, actual structured release acceptance, judge execution and every runtime probe remain UNRUN. No test file, scratch receipt, release receipt or fake scope artifact was written during these checks.

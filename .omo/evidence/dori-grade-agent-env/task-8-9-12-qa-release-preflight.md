# Todos 8/9/12 QA release preflight

Date: 2026-10-11. Checkout: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

Verdict: **BLOCKED for execution, source contracts inspected**. No live QA was executed. All daemon readiness, backend tool-scope, judge execution, gateway startup, scratch DB identity, lane turns, Discord delivery and cleanup probes below are **UNRUN**. This is not a backend isolation or manual-QA pass.

The complete `lane-scratch-qa.md`, all 176 lines of `lane-scratch-qa.mjs`, the sink helper, and `task-8-approved-db-cleanup.md` were read. Actual startup/configuration, lane CLI/runner, OMO protocol/configuration, collector startup/Kakao configuration, posting/activity and judge APIs were inspected. No Cargo, binary execution, gateway startup, daemon request, Discord request, DB access/write, credential inspection or dump was performed. Product source/tests/config and main checkout owner WIP were not edited. Only this report is an intentional file write.

## Release blockers and contract limits

1. **Gate disposition is still required.** Existing `batch-20261011T135508Z-status.tsv:1-2` records fmt exit 1 and strict clippy exit 101, without a completed full-suite/dashboard exit in that ledger. The fmt log identifies `src/agent/backend.rs:61`; clippy identifies unit-struct default calls at `tests/test_inbox_sources.rs:528` and `:552`. These are existing findings, not repairs performed here. Do not treat a successful compile-only gate or an incomplete batch as release. The coordinator must supply the actual released gate disposition and source/build receipt; this track does not wait for or restart gates.

2. **Actual backend tool scope is not established by gateway env.** `src/agent/omo_protocol.rs:26-51` sends `cwd` and `runtimeWorkspaceRoots` on thread/start, but no environment map; `:69-93` sends prompt/model/effort on turn/start, also without DATABASE_URL. `src/agent/omo_backend.rs:341-375` derives the lane workspace from lane_cwd. `src/lanes/runner.rs:396-401` supplies that metadata; `:502-513` places the actual executable and SQLx database path in the initial prompt footer. This is instruction-based control-command isolation, not remote tool environment propagation. Driver child env at `lane-scratch-qa.mjs:24-32,38-42` applies to the local gateway/CLI/judge/HTTP children, not automatically to tools in the already-running daemon.

   After release, the previously authorized harmless owned probe must inspect effective backend tool cwd/roots and DATABASE_URL equality without printing credentials or env contents. Probe the same execution mechanism and record thread/scope identifiers, expected scratch path, equality results and control-child exit evidence. An explicit control-child DATABASE_URL override can be safe even when the daemon's inherited DATABASE_URL differs; distinguish these two facts in the receipt. Do not claim that the gateway automatically propagates tool env. No new daemon, alternative port or global daemon configuration is needed or authorized.

3. **The release receipt is an external contract, not enforced content.** `lane-scratch-qa.mjs:11-15` verifies absolute argument shapes, receipt existence and binary hash, but never parses the receipt or verifies its gate/tool-scope contents. `lane-scratch-qa.md:11` nevertheless requires those contents. An arbitrary existing file would pass the mechanical receipt check. The coordinator must inspect the real receipt before invocation. Also, the driver generates a fresh scratch root only after checking receipt existence (`:14-17`), and has no prepare/probe pause before its first real lane (`:99-103`). A prior receipt cannot attest to this newly generated root's actual tool environment. A separately scoped owned probe may establish the mechanism, but exact driver-root verification needs an execution-time boundary or separately approved preparation change. Do not silently call prior generic probe evidence exact-root verification. No driver changes were made here.

4. **Binary provenance is consistent but not a signed source-to-binary release proof.** See the measurements below. The driver pins a binary hash, not a source manifest. A coordinator receipt binding this measured binary to the released source manifest is still required. Do not substitute dori-next/target-n1 receipts for the hardcoded dori-work/target-w4 binary.

## Existing binary and compile evidence

Read-only binary measurements (no execution):

- Path: `target-w4/debug/omo-gateway`.
- SHA-256: `dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec`.
- Size: 107805248 bytes; mtime: `2026-10-11T13:29:22.228Z`.
- `target-w4/debug/deps/omo_gateway-24761daf7d8f36be` has the identical hash, size and mtime. Its `.d` lists `src/entry.rs`, `src/main.rs`, `src/dashboard.rs`, `src/dashboard_runtime.rs`; Cargo.toml:17-19 selects entry.rs as the binary. Its existing fingerprint has features `[]`, rustflags `[]`, and invoked.timestamp mtime `2026-10-11T13:28:52.855Z`.
- `task-21-qa-current.md:43` independently records the same binary hash from earlier help-only QA. That verifies identity, not backend behavior.
- Existing `compile-20261011T132617Z-status.tsv:1` records compile-only exit 0. Its complete log starts with dori-work compilation, a successful test-profile finish and the target-w4 test executables. `batch-20261011T132937Z-full.log:1` records a subsequent finished test-profile compilation. Neither log is a new execution by this track.
- All 191 entries of `batch-20261011T135508Z-pre-manifest.sha256` were compared to current files by SHA-256: **0 mismatches**. The earlier `batch-20261011T132937Z-pre-manifest.sha256` is byte-identical. No listed input has an mtime later than the measured binary. This supports correspondence to the inspected frozen sources; timestamps and Cargo fingerprint metadata are not a substitute for the coordinator's source/hash release binding.
- `next-compile-20261011T135806Z.log` explicitly names dori-next and target-n1. Its successful compile cannot release this driver's binary.

## Exact supported startup and collector behavior

`src/entry.rs:100` loads dotenv before lane CLI dispatch or gateway startup. Every local child runs with cwd equal to the fresh scratch root and an explicit env (`lane-scratch-qa.mjs:38-42`). The driver creates an empty `.env` in that root at `:20`; the lane repo is a child directory and has no dotenv. Preserve that empty scratch dotenv and explicit env for every control invocation. Never launch from either repository cwd or rely on a shell assignment or inherited repository DATABASE_URL. A backend tool outside the supplied cwd must still be checked by the runtime scope probe.

`DASHBOARD_ENABLED=false` works because the whitelist omits DASHBOARD_PORT; `src/dashboard.rs:75-77` otherwise enables the dashboard whenever that variable exists. `OMON_DISCORD_ENABLED=false` takes the supported early path at `src/main.rs:646-675`: requires DATABASE_URL, initializes/logs the selected database, starts the real lane service, waits for termination and cancels/joins it. It returns before full Config parsing, Full Disk Access probing and collector startup at `:693`. Thus the prepared driver does **not start the collector at all** on matching current sources. No collector-disable switch or polling interval should be invented.

The normal Discord-enabled collector at `src/main.rs:1062-1123` starts event-driven collection, imsg watching, zele mail watching and the Kakao timer. The timer currently uses 120 seconds (`:1159-1169`); this is source description, not QA configuration. Supported Kakao settings are exactly OMON_PIRATETALK_URL and OMON_INBOX_KAKAO_ROOMS (`src/inbox/sources/kakao.rs:19-24`). Empty rooms becomes an empty allowlist (`:33-40`), but `fetch_with_client` still requests `/api/rooms` before filtering (`:112-143`). Therefore empty rooms is not a network-off switch. The driver's loopback port-1 URL and restricted PATH are additional isolation settings, not a general collector shutdown guarantee. They are redundant for the Discord-free startup path. Do not enable Discord startup to perform the task-12 sink scenario: the lane post sink is initialized independently at `src/lanes/runner.rs:302-315`.

## Exact future invocation prerequisites

The eventual argv is an explicit array with these elements, in order:

```text
<absolute available Bun executable>
/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs
execute
dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec
<absolute existing executable typesafe-judge path>
<absolute actual coordinator release receipt path>
```

The hash above is only usable if unchanged and approved at release. Resolve the two executable paths and receipt path concretely; do not invoke literal angle-bracket arguments. Use existing private TYPESAFE_API_KEY, TYPESAFE_URL, TYPESAFE_MODEL and DISCORD_BOT_TOKEN through env, plus OMON_OMO_APPSERVER_AUTH_TOKEN or OMON_OMO_AUTH_TOKEN if the existing daemon requires it. The driver demands all four required values (`:16`) and copies only its explicit whitelist (`:32`). Do not print values, add model overrides or dump the parent environment.

The inner child env is defined completely at driver `:24-32`: scratch HOME/XDG/TMPDIR, scratch-bin plus system PATH, absolute `sqlite://<fresh-root>/qa.db`, Discord off, lanes on, kibitzer/dashboard off, scratch workspace root, empty lane keywords, `ws://127.0.0.1:19742`, loopback PirateTalk URL and empty room list. It does not inherit parent flags. Confirm the supplied judge executable works with this restricted PATH and scratch HOME: a script using `/usr/bin/env bun` or another non-system runtime will not resolve merely because its script is symlinked. Judge CLI spawns typesafe-judge and inherits env (`src/judge/mod.rs:1028-1045`); presence of all three TYPESAFE values avoids home .zshenv fallback (`:862-869`). Driver import/syntax compatibility, judge executable dependencies/authentication and successful real judge selection remain UNRUN.

Use only the already-existing daemon19742. The driver performs one bounded readiness GET before gateway startup (`:86-87`), and never launches omo. Current Discord-free startup returns before `OmoDaemonSupervisor::ensure` at main.rs:752; OmoBackend directly connects. PATH exclusion of omo is not a universal autospawn guarantee: the supervisor can resolve absolute fallback locations (`src/agent/omo_daemon.rs:124-145`) if a different startup path is used. Do not use an alternate endpoint, create/stop a daemon or rely on that fallback. Readiness, daemon identity/authentication/model availability and continued readiness are runtime prerequisites, UNRUN here.

Before any scenario, require the driver's database-open/service-ready events, exact lsof main DB path and absence of repository DB descriptors, and scratch `PRAGMA database_list` identity (`:88-98`). The local lane CLI still defaults to relative `sqlite://omon_gateway.db` when DATABASE_URL is missing (`src/lanes/cli.rs:102-104`); explicit override is essential, including daemon-origin control children. Host launch guard admission is also UNRUN: start calls can_launch at cli.rs:135-138 and returns exit 4 on hold. Do not weaken guard thresholds to force QA while gate load is high.

## Lane and sink contracts checked

No definite mismatch was found between current source and the driver's lane argv, expected post JSON/body/nonce or blocked-answer path. Start args are supported at `src/lanes/cli.rs:76-92`; explicit lead is supplied and no --thread is used. Native --thread means create a new thread under the supplied parent (`:386-407`), not attach an existing thread.

Reopen resets turns_run and preserves backend thread binding (`src/lanes/runner.rs:46-82`). Completed outcome transcript and turns_run increment are committed in one transaction (`:683-705`), matching the task-12 completed blocked-turn observation. Blocked `object --reason` changes status to working and enqueues answer guidance (`src/lanes/cli.rs:446-452,493-509`). Milestone posts serialize thread/body/nonce/state/message_id (`src/lanes/posts.rs:23-30`), retain enqueue-time destinations (`:43-49`), send with enforced nonce (`:78-84`), and record uncertain without automatic retry (`:126-154`). Questions block and emit a lane event, not a thread post (`src/lanes/activity.rs:183-192,325-353`). These are source conclusions, not scenario results.

The driver questions/answers on generation-fenced qa-1, verifies exactly one newly bound milestone post and its fetched Discord message, and deliberately leaves previous NULL-destination payloads untouched. Task 8/9 completion predicates can observe a claim while subsequent backend completion cleanup is still in flight; they do not independently prove a completed-turn boundary as task 12 does. Report their actual receipts precisely rather than expanding them into stronger remote-termination claims. The three-consecutive-error case is explicitly outside this driver's scenarios (`lane-scratch-qa.mjs:161`) and still needs the coordinated test_lane_events evidence.

## Authorized destination and cleanup

Only destination: existing QA thread **1558726010181132309**, parent **1529738911377657967**, as documented by `lane-scratch-qa.md:25` and sink helper `:3-4`. No new thread, starter post, arbitrary channel, nested thread or generic POST transport is used. Future sink prepare GET must verify ID, parent, thread type, unlocked state and archive metadata before binding; unarchive only if initially archived (`lane-scratch-qa.sink.mjs:32-50`). Bind only scratch qa-1 through its completed blocked-turn/generation CAS (`lane-scratch-qa.mjs:130-139`). This does not mutate shared sessions or any repository DB.

After the one milestone, restore the saved NULL destination columns through destination/generation CAS (`:72-81,159`), restore original archive state (`sink helper:67-74`), and verify actual message ID/channel/body/nonce (`:53-65`). The driver retains the delivered QA milestone and private scratch artifacts; it contains no DELETE-message request. Cleanup authorization documented here does not imply permission to delete additional thread content.

On failure, finally cancels observations, closes only verified scratch lanes, stops/awaits owned registered children before restoring binding, checks owned PIDs absent, restores archive state and closes handles (`driver:164-175`). It never stops the shared daemon. Gateway shutdown source cancels/joins lane work; actual remote interruption/termination needs runtime evidence. Direct-child PID checks alone are not process-tree or daemon-tool termination proof. Preserve cleanup failures as failures rather than broadening writes or claiming success.

Owner-approved repository DB cleanup is **complete**, not an open prerequisite: `task-8-approved-db-cleanup.md` records the exact qa-1 deletion, COMMIT at 13:54:21.141Z, retained validated backup, all 43 table/schema comparisons, unchanged migrations and final descriptor cleanup. This report accepts that receipt and performed no new DB verification. Do not repeat cleanup or modify migrations.

## Next concrete coordinator step

Resolve gate disposition and bind the measured target-w4 binary hash to the frozen source manifest in the actual release receipt. Then, outside this read-only track, perform the already-authorized narrow backend scope probe on daemon19742, explicitly distinguish daemon tool env from control-child override, and settle how exact fresh driver-root scope will be evidenced before real lane execution. Validate the existing judge executable under the exact inner env without credential output. Only after those prerequisites are met run the existing driver with concrete argv and private whitelist env, collecting its startup, DB identity, scenario, exact sink and cleanup receipts. All of those execution probes remain UNRUN in this preflight.

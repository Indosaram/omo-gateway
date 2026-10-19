# Todo27 current evidence map

Date: 2026-10-11. Disposition: evidence-backed implementation and historical fake-test readiness, with CLI contract gaps and required live QA still missing. NOT CONFIRMED. Product remains frozen for actual8/9 QA.

## Scope and evidence identity

This reconciliation performed local read-only file inspection and in-memory comparisons/hashing. It started no command, Cargo, model, SSH, network request, service, or delegated task. Only this report was created. No product, test, host configuration, database, credential, or plan checkbox was changed. No session message content or credentials are reproduced; historical session examples are not recopied.

Roots used below:

- Main: `/Users/indo/code/project/omon-gateway`.
- Work: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
- Next: `/Users/indo/code/project/omon-gateway-wt/dori-next`.
- Evidence filenames without a root refer to Work `.omo/evidence/dori-grade-agent-env/`.

Read the complete main todo27 at `.omo/plans/dori-grade-agent-env.md:456-465`, including requirements, acceptance, happy/failure QA and guardrails; the main todo27 pre-review and next-action-status; both Work task-27-fix.md copies, task-27.log and task-27.json; current Work registry, module, integration tests, entry CLI, lib and Cargo registration; the matching Next registry/tests and session CLI sections; coordinated batch receipts and raw task27 results. Next's evidence directory contains only task-21-qa-current.md and no task27-specific receipt. Next-focused receipts stored in Work concern lane/inbox targets, not task27; they are not session-registry proof.

Current Work content SHA-256 values, computed from complete read text, match both pre/post manifests of `batch-20261011T135508Z`:

| Input | SHA-256 |
| --- | --- |
| src/sessions/registry.rs | feb345825b390e1ad9694e388c535bc43bab1404d67ccf278c6e2dfc4185e865 |
| src/sessions/mod.rs | 9256c987523dcb5ab3483a114bfafd4d3c0728a530577280b78913ec402dc0a3 |
| tests/test_session_registry.rs | a1bbed5d49a3799561c76ac8f17a393f0e2121535a091071447b54e169f013ba |
| src/entry.rs | 93480989ad417a5093e344300fc2bb05001dd8ebf668ad649658125a4ed04486 |
| src/lib.rs | b31058f8dad2fe6c646db3abd5bde6363dfeff374b8df906265633db51e81195 |
| Cargo.toml | 78d02103a7723b731a50f3c1d323639ca9d79b828db459f5f51bc4a7d242f13d |

Next registry and integration-test contents are identical to Work. Its inspected CLI declaration/dispatch has the same contract. This is not a whole-tree equality or Next test-pass claim.

## Requirement-to-implementation map

| Required behavior | Current implementation and actual test evidence | Remaining boundary |
| --- | --- | --- |
| Local plus configured remote merge | `registry.rs:204-221,229-279`: local is always included; hosts come from environment or seed file; `join_all` runs per-machine futures concurrently and appends successful records with machine identity. Unit `local_and_remote_sessions_merge_and_cache`; integration `list_all_merges_local_and_remote_and_reports_exit_three` passed in the historical batch. | Tests invoke a fake runner, not the gateway executable or actual SSH. No current aggregate gateway happy-list receipt. |
| 20-second per-machine bound and isolated errors | `registry.rs:14,240-258`: timeout wraps each runner; other successes remain in the result. Unit `hanging_host_times_out_without_blocking_other_hosts` and integration `timeout_drops_child_future_deterministically_without_sleeps` passed using readiness channels, paused time advanced to 21 seconds, and fake DropGuard cancellation. | No actual unreachable-host gateway duration <=25 seconds receipt. Fake tests do not prove OS SSH child termination/reaping or exact 20-second boundary behavior. Readiness and task joins in these tests have no explicit bounded await. |
| Exit 3 means nothing answering | `registry.rs:260-267`: host exit 3 becomes an error containing `nothing answering`; other nonzero codes become numeric host errors. Unit `exit_three_is_reported_as_nothing_answering` and the merged integration test passed. | This maps the child error text, not the gateway process exit code; see CLI drift below. No live exit-3 failure receipt. |
| 15-second cache | `registry.rs:15,229-235,277`: an instance stores a completed aggregate result and reuses it while elapsed time is <15 seconds. Unit immediate-reuse assertion passed, with two host calls total across two lists. | No expiry/boundary test. `run_sessions` creates a new instance, lists once and exits; separate CLI invocations do not share this cache. No session registry is retained by the inspected CLI. |
| Default host seed | `configured_hosts` at `registry.rs:311` gives a nonempty comma-separated environment override precedence; empty/unset falls back to HOME `.omon/session-hosts.json`; absent file means local only. Seed helper probes candidate aliases and writes JSON; it is not called by `from_environment` or the CLI. Historical receipt records a one-time probe followed by apply_patch persistence. Current names-only home-file read is `["omarchy"]`. Unit `hosts_file_round_trips_without_touching_home` passed. | Seed contents are currently observed; reachability/omo availability are historical, not re-probed. No automatic seed-on-list should be inferred. Environment/file loading is not exercised by the fake merge tests, which supply hosts directly. |
| SSH separate argv, BatchMode, no installs/keys | `registry.rs:170-186`: `ssh`, `-o`, `BatchMode=yes`, `-o`, `ConnectTimeout=5`, host, then separate `omo`, `thread`, `list`, `--all-scope`, `--json` elements. No `sh -c`. `stdin(null)` and `kill_on_drop(true)` are set. Historical remote listing receipt records exactly this argv. | Fake runner tests assert registry's inner omo argv only; they do not intercept ProcessCommandRunner's final SSH argv or verify OS lifecycle. No new SSH/install/key operation occurred here. |
| Strict JSON and planned fields | `registry.rs:98-126,282-308`: requires array, deserializable records and nonblank session ID; sets machine, synchronizes missing timestamp counterparts and preserves original record. Unit strict/parser field tests and integration parser/serializer tests passed. | `alive` and `surface` are not typed fields; they remain only in `raw` if supplied. The serializer has extra fields, and existing tests assert selected keys rather than an exact key set. Real receipts retain only identifiers/names/cwd, so expanded fixtures are not authenticated live field schemas. |

The host parser's wildcard/dedup test passed, but that behavior applies to SSH-config probing only. Environment/seed hosts are checked for empty/whitespace entries and do not receive that parser's wildcard filtering or deduplication. Do not describe all host input paths as deduplicated.

## Current CLI contract versus prior evidence

1. **Gateway exit mapping gap.** Work `src/entry.rs:183-207` prints a serialized SessionList and returns 0 for every successfully constructed registry, including an aggregate with no sessions and only host errors. Constructor/serialization errors return 1; the false-json branch returns 2. It does not map all-host nothing-answering to process exit 3, unsupported to 4, or internal failures to 5. Todo27 states the documented 0/1/2/3/4/5 contract. The precise policy for mixed successful/failed hosts is not stated there; do not invent it. The all-host-error success path is directly visible regardless of that ambiguity.

2. **Cache scope gap.** The 15-second library cache is real within one instance and has passing immediate-reuse fake coverage. It cannot reuse results between `omo-gateway sessions list --json` processes. The prior review's cache readiness must be read as instance-level only, not evidence of user-facing repeated-command caching.

3. **Output contract and privacy boundary.** CLI output is `{sessions:[...], errors:[...]}`, not the raw array produced by `omo thread list`. Each Session serializes the planned fields plus `last_activity` and `raw`. `raw` is the complete original session record, so output is not restricted to names/cwd and may preserve additional upstream fields. No assertion is made that these contain credentials or messages. A QA artifact must project to names/cwd and machine/count/error metadata rather than preserve full raw output. Prior repair wording about an exact planned schema overstates what the selected-key tests establish; the main pre-review explicitly accepted retaining these extra public fields.

4. **Invocation shape.** `src/entry.rs:85-90` makes `--json` required and offers only `sessions list`. This matches the named todo27 happy command. No plain-text list or seed subcommand is provided. Todo28's read/send/interrupt/adopt controls are a separate scope, not a todo27 regression.

5. **Stale receipt statements.** task-27.json still lists module/test registration as pending, but `src/lib.rs` declares sessions, `src/sessions/mod.rs` declares registry, and Cargo.toml explicitly registers `test_session_registry`. The coordinated raw log actually runs it. Main's source-only pre-review labels tests UNRUN within that review; the later canonical batch supplies historical passing task27 results. Neither receipt establishes gateway runtime QA.

## task-27.json live provenance

task-27.json and task-27.log are executor-written historical receipts for two separate raw-tool invocations: local `omo thread list --all-scope --json` recorded exit 0/valid array/9 sessions; separate argv SSH listing on omarchy recorded exit 0/valid array/3 sessions. Captured fields are sessionId/name/cwd; the JSON contains one example per machine, not all 12 rows. These establish attributed raw-tool observations, not a current replay, gateway aggregate output, timing, error isolation, cache behavior or full upstream schema.

The JSON has a historical `ssh_host_probe.seeded_file_written:false` alongside a later `host_persistence` entry recording an absent prior file, result `["omarchy"]`, and apply_patch persistence. The log and integration receipt also record that persistence, and the current home file agrees. Preserve these as distinct receipt stages rather than treating the early false flag as current absence. The seed helper's existence is not proof that it performed that historical write.

The JSON teardown text says no configuration changes remained, while its more specific cleanup receipt says the intended host-list file remains. The specific host-persistence/cleanup entry qualifies the broad teardown sentence. No timestamped complete raw stdout, executable hash or measured gateway duration is attached to these live entries. The main pre-review's historical source hash matches current registry content; it is a source assessment, not independent live authentication.

## Actual historical tests and gates

`batch-20261011T135508Z-full.log:392-398` records all seven current registry unit tests as `ok`: exit-three mapping, hanging-host isolation, merge/cache, planned field parsing/serialization, SSH-host parser, strict JSON, and host-file round-trip. Its integration section records all four current tests as `ok`, with summary 4 passed, 0 failed, 0 ignored. Current task27 source/test/CLI/wiring hashes match the batch pre/post manifests, so those historical results are attributable to these task27 inputs. No tests were run in this reconciliation.

The batch status ledger records fmt exit 1, strict clippy exit 101, full suite exit 101, dashboard exit 0. Its handoff reports 934 passed, 15 failures matching baseline by name, 16 ignored, dashboard 59 passed, and identical manifests. Baseline causal equivalence is not established by this reconciliation. The handoff names the format diff in backend.rs and clippy unit-struct defaults in inbox tests, not a task27 failure. Main next-action-status records later repairs and Next clippy progress; those do not create a current all-green Work gate receipt. No coordinated batch is restarted here.

## Exact remaining receipts and implementation gaps

Required plan QA remains missing from the inspected task27 evidence:

- **Gateway happy receipt in named `task-27.json`:** actual `omo-gateway sessions list --json`, binary/source identity, actual exit, and sanitized evidence of >=1 local session plus an omarchy session. Current named file contains raw omo/SSH receipts instead. Preserve their provenance when adding gateway evidence later.
- **Gateway unreachable-host failure receipt:** nonempty OMON_SESSION_HOSTS including an unreachable alias, actual elapsed time <=25 seconds, that alias in errors, and other responding-host results retained. Todo27 does not assign this failure scenario a separate filename; `task-27-failure.json` would be a proposed new receipt name, not a falsely claimed missing plan-mandated filename.
- **Gateway exit-contract failure receipt:** all-host exit-3 behavior and actual process status. Current implementation has a gap before such a receipt can establish the documented mapping. Mixed-host exit policy needs an explicit contract, not a guessed expected code.
- **Runtime cancellation receipt:** actual timeout/caller cancellation through ProcessCommandRunner with evidence that owned SSH child processes terminate and cleanup completes. Existing DropGuard receipts only prove fake-future drop.
- **Cache expiry/scope evidence:** exact before/at-15-second invalidation behavior is not covered; cross-invocation caching is absent from this CLI. Decide the required cache lifetime before claiming the CLI cache requirement satisfied.
- **Strict shape/error-isolation coverage:** existing direct parser rejection tests do not demonstrate malformed-host JSON alongside a valid host through the aggregate/CLI. No production-runner final-SSH-argv assertion exists. Typed alive/surface handling and the extra serialized raw record remain contract differences, not authenticated live-field behavior.
- **Current coordinated gate receipt:** a current Work fmt/clippy disposition tied to source identity remains necessary under the plan. Historical task27 fake tests are already present and passing; do not request a duplicate full suite merely because older task27 prose says UNRUN.

Readiness: the local/remote merge, fake timeout isolation, host exit-3 text mapping, immediate instance cache reuse, strict parser and seed-file round-trip have historical passing evidence tied to current task27 inputs. The host seed is present and SSH argv construction is visible in source. Gateway exit semantics, CLI cache lifetime, raw output projection, actual happy/failure gateway QA and OS-child cleanup are not confirmed. Stop here; no source edits, QA execution or checkbox closure is authorized by this reconciliation.

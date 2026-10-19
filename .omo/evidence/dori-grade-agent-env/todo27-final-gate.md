# Todo 27 independent FINAL gate

Verdict: **CONFIRMED** for todo 27 in the settled `dori-work` implementation and the recorded actual gateway QA on omo-ai 5.1.30 / senpi 2026.10.11. No blocking acceptance failure remains in this scope.

This reviewer did not implement the change. The complete 614-line main plan was read, including global guardrails, todo 27, acceptance, QA and success criteria. Review was read-only except for this requested report. No command, Cargo invocation, model call, service operation, remote probe, installation, key change or session control was performed. Hashes were computed in memory from existing artifacts. Existing QA was audited, not rerun.

## Scope and artifact identity

Plan: `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`.
Implementation: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Evidence base: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/` (all receipt names below are relative to this base).

The main checkout has no `src/sessions/registry.rs`, `src/sessions/mod.rs` or either registry integration test. CONFIRMED therefore applies to the requested settled worktree and its captured binary, not to a claim that the main checkout has already integrated or deployed these files.

Independently calculated current identities:

| Artifact | SHA-256 | Result |
| --- | --- | --- |
| `wave4-strict-gate2-post.sha256` | `60a3af1024eb2eae6975aae5a2e542c3ecad762bc918183c5e7b92f75e005517` | Matches release receipt |
| `target-w4/debug/omo-gateway` | `71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86` | Matches release and both live QA scenarios |
| `src/sessions/registry.rs` | `0792ebef1066b449242ac81c7129a384c58a6eafbcd8c2bcf85fe5abb7f9f267` | Matches manifest and focused settlement |
| `src/sessions/mod.rs` | `9256c987523dcb5ab3483a114bfafd4d3c0728a530577280b78913ec402dc0a3` | Matches manifest and focused settlement |
| `tests/test_session_registry.rs` | `110e812e42376462f5005a3789fb4f8e503523bff8e99a2de21af5b38ae23576` | Matches manifest and focused settlement |
| `tests/test_session_registry_task27.rs` | `33ee903df548e90b54ca948477e7f6199f393fea91b04b29e5c83bb529f85ee6` | Matches manifest and focused settlement |

`task27-source-settled.md` preserves the original dependency-compilation timeout and then records the authorized successful focused run: 7/7 and 2/2, exit 0. Its old candidate-activation warning is superseded for live acceptance by the later install and live QA receipts, not erased. `wave4-integration-source-settled.md` records the single 13-target gate, including both registry targets: 117 passed, 0 failed, 4 existing ignored, exit 0. `wave4-strict-gate2-release.json` records fmt 0, strict all-target/all-feature clippy 0, build 0 and source comparison 0. Integration tests preceded two boolean-equivalent changes in unrelated kibitzer/triage files; registry source and tests still match their verified hashes.

## Every explicit acceptance criterion

| Criterion | Verdict | Evidence and reasoning |
| --- | --- | --- |
| Fake runner merges local plus one remote | PASS | `tests/test_session_registry.rs` `list_all_merges_local_and_remote_and_reports_exit_three` checks machine identities, fields, both rows and aggregate exit 0; recorded 7/7 pass. |
| Timed-out host appears in errors while others return | PASS | `timeout_drops_child_future_deterministically_without_sleeps` checks retained local row and canceled stalled future. `tests/test_session_registry_task27.rs` `parallel_listing_retains_successes_when_one_host_times_out` subscribes before triggering, proves all four hosts start before releasing local, advances paused time to 20 seconds, retains local/remote and isolates stalled/exit-3 errors; recorded 2/2 pass. |
| Exit 3 means nothing answering | PASS | Explicit fake exit-3 assertions in both targets; `registry.rs:166-177,288-292` preserves code 3 and the message. Taxonomy tests cover 0-5, SSH 255, missing executable 127, unexpected codes and partial success. `src/entry.rs:185-219` prints the projection and returns the aggregate exit code. |

Additional planned behavior is present: local plus configured hosts, parallel `join_all`, 20-second per-machine timeout, `kill_on_drop(true)`, strict array/session-ID parsing, typed optional `alive`/`surface`, seven-field public projection and per-host errors. Fifteen-second cache reuse and exact expiration are tested with paused time. The cache is per registry instance; a new CLI process does not reuse another process's cache. This is not a persistent cross-invocation cache claim.

## Both real QA scenarios

Exact gateway argv in each receipt is `["/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway", "sessions", "list", "--json"]`.

| Scenario | Required result | Recorded observation | Verdict |
| --- | --- | --- | --- |
| Happy, hosts `omarchy` | At least one local session and an omarchy session | Exit 0, 10948.827709 ms, local 23 plus omarchy 3, errors empty, stderr 0 bytes | PASS |
| Unreachable, hosts `omarchy,127.0.0.1` | Return within 25 seconds with failed host in errors | Exit 0, 5725.626332999998 ms, local 23 plus omarchy 4 retained; `127.0.0.1` code 3, underlying SSH exit 255; stderr 0 bytes | PASS |

Artifacts: `task-27-live-qa.md` and new `task-27.json`, provenance `mon_RRFMASB7XQQ1GA2N`, runner exit 0. The new projection contains only name/cwd session fields grouped by machine, along with counts and errors. Live SSH was real for omarchy; the failing address was an owned loopback listener that accepted without an SSH banner. It tests an unavailable SSH host/handshake, not a model-backed fake or a remote install. Cleanup records zero owned children, no scratch DB files and closed listener. The live failure scenario does not prove exact 20-second cancellation; that boundary is covered by deterministic fake-runner tests.

The historical failure is preserved intact as `task-27-historical-failure.json`: local enumeration exceeded 20 seconds on the older runtime. `task27-local-enumeration-blocker.md` identifies the 5.1.29 SDK's redundant request after an unresponsive RPC verdict. `task27-omoai-install-receipt.md` establishes the pre-existing newer 5.1.30 / senpi 2026.10.11 runtime and deliberate decision not to install the patched 5.1.29 candidate. Downgrading is not required by todo 27. These passing observations establish current task behavior, not acceptance or installation of that candidate.

## Default host seed: implemented and evidenced

**PASS for the plan's one-time seed during this todo.**

`registry.rs:17-36` extracts explicit SSH Host aliases and skips patterns/deduplicates. `seed_hosts_file` at lines 39-79 probes candidates in parallel through `ProcessCommandRunner` with separate `command`, `-v`, `omo` words, BatchMode and ConnectTimeout, accepts successful nonempty output, and writes a JSON array. `configured_hosts` and `read_seeded_hosts` use the HOME-relative `.omon/session-hosts.json` when the environment host list is empty; nonempty `OMON_SESSION_HOSTS` overrides it.

Historical evidence distinguishes discovery from persistence: `task-27-historical-failure.json` `ssh_host_probe` records one probe per configured alias and only omarchy having omo (`seeded_file_written:false` for that probe stage); its separate `host_persistence` records the previously absent `/Users/indo/.omon/session-hosts.json` created with apply_patch as `["omarchy"]`, without overwriting owner entries. The reviewer independently read the current file and confirmed that exact host list. `task27-source-settled.md` also records reading the existing seed rather than probing/writing again.

The seed helper has no production caller: this is not automatic first-run discovery on a fresh machine, and a missing file yields an empty default list. The plan says to seed the default list once *during this todo*, which the existing discovery/persistence evidence satisfies. No claim is made that the helper itself produced the owner's existing file. Both live scenarios explicitly set `OMON_SESSION_HOSTS`; default-file operation is supported by code inspection, file evidence and the registry module's host-file round-trip test, not by a new env-unset live invocation.

## Must NOT do and applicable global guardrails

| Constraint | Verdict | Evidence |
| --- | --- | --- |
| No remote installation | PASS | Registry only runs `command -v omo` and `omo thread list`; live/install receipts explicitly record no remote package operation. The local 5.1.29 candidate was also not installed. |
| No added keys | PASS | No key-generation/write path exists in the reviewed registry; discovery and QA receipts record existing authentication and no key change. |
| Remote command as separate SSH argv words, not shell string | PASS | `ProcessCommandRunner` at `registry.rs:190-205` builds `-o`, `BatchMode=yes`, `-o`, `ConnectTimeout=5`, machine, then extends the individual command words. No `sh -c` or joined remote-command argument. Live receipt confirms that exact shape through an owned shim executing `/usr/bin/ssh`. This satisfies the requested argv contract; SSH's own server-side command handling is not a claim of direct remote execve. |
| Existing SSH configuration, BatchMode, no exposed port/service change | PASS | omarchy uses existing authentication/config. The isolated loopback listener is an owned QA failure fixture, not exposure on a remote machine; it was closed. |
| No models, .env changes, owner DB writes, Discord UI QA, session send/interrupt/adopt | PASS in reviewed scope | Source has no such operations; captured QA environments exclude provider/Discord variables and use an owned forbidden DB path that remained absent. Receipts record no shared service/session control. |

## Limits and final disposition

No blockers for todo 27's stated acceptance and one-time default seeding. Fresh LSP diagnostics were unavailable in the original verification; this is explicitly recorded, not called clean. The strict build/clippy and focused/integration passing receipts support the settled source. Historical whole-repository backend baseline failures and two timeout-unverified tests remain outside this task; no whole-suite green claim is made. The binary is the released gate artifact at a debug path, not evidence of Developer ID signing or production service deployment (todo 36).

CONFIRMED is scoped to todo 27. It does not close todos 28/29, the entire plan, candidate activation, main-checkout integration or deployment. No follow-up implementation is requested by this gate.

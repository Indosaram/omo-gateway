# Task 27 source-only gate review: final narrow correction

recommendation: APPROVE (source readiness only)
status: source-ready-tests-QAglobalpending
blockers: []

## Intent and user outcome

originalIntent: Exact todo 27 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:456-465`: list local and configured remote OMO sessions through the gateway, concurrently with a 20-second per-host deadline, strict JSON, planned session fields, isolated host errors, exit-3 mapping, and a 15-second cache. The review request also requires cancellation without leaked SSH processes.

desiredOutcome: `omo-gateway sessions list --json` uses the seeded host configuration and provides the planned aggregate session result. Tests and actual gateway QA must establish observable behavior before todo27 is confirmed.

userOutcomeReview: All three original source findings and the subsequent duplicate-alias finding are addressed. This final pass is limited to B2-R as instructed; it does not repeat the full audit or certify runtime behavior. The source is ready for the coordinated tests, QA, and global gates, not confirmed complete.

Relative paths below refer to `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## B2-R closure and current source identity

Directly read the current `src/sessions/registry.rs:98-131,287-310` and the complete updated `.omo/evidence/dori-grade-agent-env/task-27-fix.md`.

- `src/sessions/registry.rs:108-114`: `updated_at` retains the `lastActivityAt`, `lastActivity`, and `updatedAt` aliases and its canonical `updated_at` field name.
- `src/sessions/registry.rs:119-120`: `last_activity` now has only `#[serde(default)]`. The conflicting aliases are absent, so the duplicate Serde match labels identified by B2-R are removed.
- `src/sessions/registry.rs:121-122`: `raw` remains intact.
- `src/sessions/registry.rs:298-303`: the existing missing-value synchronization between updated_at and last_activity remains intact. This fills an absent counterpart; it does not overwrite two independently supplied values.
- `src/sessions/registry.rs:305`: the full original record remains assigned to raw.

No removal of existing public fields or additional compatibility design is needed. B2-R / T27-GATES / T27-FIELDS is closed at source level. No compiler or clippy result is claimed.

SHA-256 recorded after these reads:

```text
feb345825b390e1ad9694e388c535bc43bab1404d67ccf278c6e2dfc4185e865  src/sessions/registry.rs
603be39e1ad4e285856b0994472f2f55a8034039b9a215925802a06940f91dae  .omo/evidence/dori-grade-agent-env/task-27-fix.md
```

## Prior findings retained as resolved source findings

These conclusions come from the preceding full source re-review; this narrow pass did not rerun that review.

- T27-CANCEL: the real process runner sets kill_on_drop(true) before output(). The previous review inspected matching installed Tokio 1.53.1 process source, selected by Cargo.lock. Its output future owns the child, and the documented destructor contract kills the child when that setting is true. Unix reaping remains best effort; no bounded reaping result was observed.
- T27-FIELDS: the struct exposes canonical session_id, updated_at, and controls. Existing last_activity/raw are retained intentionally. The duplicate aliases introduced by that repair are now removed.
- T27-TEST: the successful-host fixture now returns a valid session, matching the assertion of one retained session alongside the stalled-host error. Channel readiness and paused time replace the earlier fixture contradiction and yield-based setup.

The corrected repair receipt explicitly states that DropGuard tests verify fake-future cancellation only, not OS-child termination or reaping. That distinction is accepted. These tests would not detect removal of kill_on_drop from ProcessCommandRunner; actual process cleanup remains runtime-unverified.

## Actual nine-class adversarial map

The map below preserves the actual classes in task-27.json and the prior source assessment; it is not a new test run.

| Class | Source assessment | Gate status |
| --- | --- | --- |
| host_timeout_isolated | Per-host deadline, valid successful-host fixture, fake cancellation signal, real runner cancellation setting | Tests and actual OS lifecycle unverified |
| exit_code_3_nothing_answering | Explicit host-error branch with fake coverage | UNRUN |
| strict_json_non_array_or_missing_session_id | Strict parser and planned field mapping; B2-R now closed | UNRUN |
| ssh_host_globs_and_duplicates | Existing wildcard/dedup parser and test | UNRUN |
| cache_deduplicates_recent_listing | 15-second cache and immediate-reuse call-count test | UNRUN; expiry not independently tested |
| host_config_file_round_trip | Existing scratch-file round-trip test | UNRUN |
| local_plus_remote_merge | Both fake layers assert aggregate records and machine identity | UNRUN |
| remote_installation_or_key_changes | No such operation in reviewed source or this review | None performed |
| existing_session_send_or_interrupt | No such operation in reviewed source or this review | None performed |

Prior review also checked exact local/remote list argv, join_all parallelism, default host-file loading, strict host-isolated parsing, and relevant CLI serialization. The first review directly read `/Users/indo/.omon/session-hosts.json` as `["omarchy"]`. This final pass did not re-read home configuration or probe any host. Existing module/test registration is ready per the latest coordinator instruction.

## remove-ai-slops / programming review disposition

The named skills were unavailable in the advertised skills and initial targeted searches, so their criteria supplied by the review prompt were applied directly during the preceding full review. That pass found no deletion-only, requested-removal-only, tautological, or prose-pinning tests. Field and argv assertions cover machine-consumed contracts. Duplicate unit/integration coverage, the unnecessary default-valued with_timeout test helper, and unbounded test start/join waits were recorded as nonblocking maintenance/test-discipline notes, not invented task27 requirements.

The final minimal correction removes the unnecessary conflicting parsing aliases while preserving established APIs and adds no new production layer or test. No full slop re-audit was repeated. No separate code-review artifact explicitly documenting equivalent skill coverage was supplied; the direct reviewer pass remains the coverage record. Repair prose does not substitute for source inspection.

## Artifact ledger and evidence gaps

Current narrow pass:

- `src/sessions/registry.rs:98-131,287-310` read directly.
- `.omo/evidence/dori-grade-agent-env/task-27-fix.md` read completely.
- SHA-256 of both artifacts recorded above.

Earlier review inputs retained for scope/provenance:

- Exact main plan todo27 and verification requirements.
- Complete registry/module/integration-test files and relevant src/entry.rs diff.
- `.omo/evidence/dori-grade-agent-env/task-27.json` and task-27.log.
- `.omo/evidence/dori-grade-agent-env/batch-integration-plan.md`, including superseding wiring/source-freeze receipts.
- Cargo.lock versions and matching installed Tokio process / Serde derive identifier source.
- Initial read of the default home host file.

Tests, Cargo, clippy, global fmt, dashboard, live SSH, gateway CLI QA, and actual OS-child cleanup verification remain UNRUN in this review. The owner's scoped rustfmt exit-0 statement is attributed receipt evidence, not a reproduced result. Prior 9-local/3-remote counts remain raw-tool evidence only, not gateway proof. Required gateway happy-listing and unreachable-host-within-25-seconds artifacts are still pending. No full raw live session output independently authenticates the expanded field fixtures.

No notepad artifact or separate manual QA matrix was supplied; the plan scenarios and task JSON's nine-class map were used. No eval/agentToolkit tool is exposed to obtain currentAttemptDir; the explicit main report destination requested by the coordinator is used.

## Cleanup and final disposition

Only this report was rewritten. No source/test/home configuration/database/key/remote changes were made. No Cargo, QA, SSH, service, model, or persistent process was started. Hashing created no temporary resource; no cleanup is needed.

APPROVE source readiness: source-ready-tests-QAglobalpending. There are no remaining source blockers from the requested review. Do not mark task27 confirmed until the coordinated tests, gateway QA, and global gates supply their actual results.

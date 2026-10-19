# Task 13 minimal integration unit

## Released current-context revalidation

Parent release received after task14 actual bridge Vgxg9i cleanup PASS (helper0/gateway0, PID84100 absent, owned socket gone). Actual product application and focused validation are now authorized; the older unapplied-only status below describes artifact preparation, not the current release.

Pinned current manifest: duplicate-final-20261011T184722Z-launch-manifest.sha256, 205 entries, SHA-256 a9570ea35fb0d696bffd5f7c5154f5f01a3663fd6c59ff26f0cd0acbedb3d06d. Exact-byte current main c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a, omo_backend 04b8fc3cdb541901e37431a7466ee78e3184407b3a8dc001a6c3f7f656dcfcb2 and actor 1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3 match that manifest. Earlier chunk-reconstructed main/backend hashes lost blank-line bytes and are not authoritative; these exact-byte identities correct them.

All six paths revalidated: runner five unique update hunks, CLI three, mod one, Cargo one; settle.rs and test_lane_done.rs additions absent. Four current update-file hashes match the captured table below. No patch context changes necessary; corrected patch hash remains c7f9b1cc4b7b19aa5a9df9d6979e485009103a24d7470ebcf7062ea70aeb2179. No main/actor/backend hunk is present, preserving their newer headless/origin/persistence work.

Schema source rechecked: migrations/0031_lanes.sql has lane key/status constraints, claim_at/evidence/objection, created/updated timestamps, history id INTEGER PRIMARY KEY AUTOINCREMENT with lane-key FK, history from/to/reason and event body/lead columns; 0033_lane_inbox.sql provides pending guidance and FK. No unique timestamp constraint is assumed. Reservation and final result use existing claim history ID and transaction fences; no migration is necessary for these SQL shapes. Actual fresh/existing scratch migration/runtime proof remains a focused-gate obligation, not a DB inspection performed by this revalidation. The same two preimplementation fixes remain included; no new confirmed-review or compiler verdict is asserted.

Prepared 2026-10-11. Status: artifact-ready, NOT applied, compiled, runtime-verified or confirmed. The parent releases this unit only after current task 12/14 runtime gating and the active task 14 persistence actor/backend and task 23 fixture work settle. This preparation executed no commands and changed only this note and task13-integration.patch. No product, Cargo, git, model, database or service operation occurred.

## Exact artifact and application boundary

- Evidence map: task13-current-evidence-map.md, read in full in bounded chunks.
- Patch: task13-integration.patch in this same directory, SHA-256 c7f9b1cc4b7b19aa5a9df9d6979e485009103a24d7470ebcf7062ea70aeb2179, 43140 bytes.
- Format: apply_patch envelope (not git unified diff); paths are relative to /Users/indo/code/project/omon-gateway-wt/dori-work. It is an unapplied text artifact. Parent must inspect and apply from that tree only after release; do not apply it from the main checkout.
- Exact unit: four updates (src/lanes/runner.rs, src/lanes/cli.rs, src/lanes/mod.rs, Cargo.toml) and two additions (src/lanes/settle.rs, tests/test_lane_done.rs). Ten uniquely matching update hunks were reconstructed against captured W inputs in memory. This was a textual check, not a tool application dry run, compiler result or behavioral pass.
- Do not replace whole runner/CLI/Cargo files from next. The patch changes only task 13 additions; current no-token startup remains verbatim. If current owners change one of its contexts, reconcile those small hunks with that owner after gating; no force/fuzzy whole-file overwrite. Do not write runner during the active freeze.

## Exact captured input identities

W = /Users/indo/code/project/omon-gateway-wt/dori-work. N = /Users/indo/code/project/omon-gateway-wt/dori-next. Full named source reads matched byte counts at inspection.

| Tree / input | SHA-256 |
| --- | --- |
| W/src/lanes/runner.rs | 9e9925c96befc31db85ed6df60cdf577ce86c904f2ce584eb3e30fe8c4a8b592 |
| N/src/lanes/runner.rs | c8e90824946a1458e0e55516a669229bad1c8ce042f902ffc53ed496b9b535bf |
| W/src/lanes/cli.rs | cf5fa7d73c08f9677b890bf39b9059d0c5a67be4ff50f27ad46c8bdd7c46102d |
| N/src/lanes/cli.rs | 3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510 |
| W/src/lanes/mod.rs | ddb4e4e50336431a3fd9038be084a55d5ab33966682c9af2efe95c50075a4363 |
| N/src/lanes/mod.rs | 8ddbf9e21666cfe4146e066f678ed749fb7b6aa5a657f3a32dffb9539b17f2f3 |
| W/Cargo.toml | 4d7d8ad44f283880b2ab87b968f1133451c58a5fc43b1b3e9179100382434022 |
| N/Cargo.toml | dc372194055980d3b90b9b966298767e5cf7ac46222ba4cb410c10ef0cd62c0f |
| N/src/lanes/settle.rs | 9f6465dcc7db7307ea64a541b364e397358b87adbcc6871bb214b26776d26ce0 |
| N/tests/test_lane_done.rs | 946fca8c1c2a147e2827db50d45e69c2a4cd47375e8eedecd5dd5415393b33a5 |

Hashes pin the inspected files, not active actor/backend or fixture provenance. Those independent owners may still change their own files. The parent must use its released combined manifest/binary evidence, not this artifact hash as release approval.

## Minimal hunks and preserved contracts

1. Runner: add Settler field/default clock/SystemSignalRunner, injectable with_settlement and settle_due; separate settlement JoinSet; 30-second interval with Skip; result notification; abort/join settlement shutdown; verified_done_ready hook. Five runner hunks. Keep current conditional OMON_DISCORD_ENABLED sink construction unchanged. No run_lane, capture, fail, model, backend or generation logic edits. Preserve backend-turn JoinSet session ownership, CAPTURE_GENERATION task scope, ready-turn cancellation, reopen generation CAS and inbox ACK fences, scrubbed failure events and socket/post scheduling.
2. CLI: production change_claim delegates to injectable change_claim_at; claim update clears old objection; guidance uses that same persisted timestamp + five minutes in KST. Three CLI hunks. Keep write lock/transaction, legal transitions, generation fence, claim-evidence scrub and owner object/blocked-answer text byte-preserving semantics.
3. Module and Cargo: add only pub mod settle and test_lane_done registration. No new dependency, lockfile or migration. Preserve existing task 14 registrations and current Cargo ordering.
4. Settler: port next's due scan, reservation, signals and worktree checks, CAS result/history/event/inbox write; add the event-boundary scrub below. Do not change tracked-only cleanliness, external worktree selection or timeout policy.
5. Done tests: port next's seven existing cases with canonical cwd repair, plus the event scrub, same-timestamp reclaim and joined shutdown regressions (ten test functions total). No task 23 fixture or current runner test modifications.
6. Main/headless/EventOrigin/dispatch/persistence actor/backend remain outside the patch. Current main's headless lead, System origin, local transcript delivery and joined shutdown are preserved by no edits. verified_done_ready only reports readiness; todo 15 owns close/recovery and todo 35 its optional diff seam. Do not label this unit end-to-end automatic close.

## Settlement event scrub fix included

Next currently binds raw joined detail to generated not_done lane_events.body. The patch computes body = super::scrub::scrub(&detail) inside the nonempty-reasons branch and binds body only to that event. Local objection/history and lane inbox guidance retain detail for diagnosis; owner-authored CLI object/answer text is untouched. No scrub helper, runtime env or configured-host mutation is introduced. Current outbound contracts must still scrub any later Discord posting of local guidance.

The proposed eighth test, settlement_event_scrubs_generated_detail_and_preserves_local_reason, injects a synthetic /Users/alice path and fake ghp token through the actual SignalRunner failure, claims through the fake backend/storage path, advances MockClock and invokes the real settler. It reads persisted event and objection: event must omit path/token and contain ~/proof.txt and <redacted>, local objection must retain diagnostic inputs, status not_done, exactly one check. It does not pin prose, mutate environment, sleep, bypass persistence or claim configured-host/live sink proof. A small existing Checks fixture field permits the failure detail; its normal default preserves the previous failure test. This test is source-only and UNRUN.

## Claim/object/deadline/restart invariants

- One transaction writes claim status, claim_at, evidence/history, event and queued idle guidance. KST display derives from that exact clock value; runtime wrapper uses Utc::now, fake cases use MockClock. Fresh claim clears prior objection, not owner input.
- Object before settlement returns not_done and persists/queues its exact reasons; blocked answer remains working with exact answer text. Runner's existing status/generation handling resumes work and does not start new turns while done_claimed.
- Due means claim_at <= now - five minutes. Tests retain 299-second no-check and 300-second verification assertions. Scheduling uses a separate JoinSet and skips missed ticks, not a model timer.
- Settlement reservation is committed before async signals/git, scoped by persisted claim_at plus reopen/latest claim history. check_all occurs once in the unreserved path; each failure detail and git reason feeds one result. No transaction is held across the async checks.
- Reservation reloads the authoritative Lane under the SQLite write lock and reads its latest claim-history ID and reopen generation in that transaction. Deadline, checker input, reservation and final result all refer to that same attempt. Result CAS checks status, claim_at, reopen generation AND latest claim-history ID. Same-timestamp reclaim changes the ID even without reopen; a stale success or failure updates zero rows and writes no result history/event/inbox. Successful status removes the lane from future due scans. Independent competing Settler instances are still not claimed proved safe.
- Restart before deadline reconstructs the runner on the same fixture pool, then gives one verification/history and one check at deadline. Existing interrupted reservation test fails closed to not_done without replay and permits a fresh claim. This is at-most-once command invocation recovery, not proof of exactly-once external effects or real process restart.

## Canonical unpushed repair and Git scope

The patch retains next fixture's tokio::fs::canonicalize(repo) before git init and before storing lane.cwd. Production settler canonicalizes cwd, so expected unpushed path now uses the same /private/var versus /var identity. Keep the local bare remote, baseline fixture push, unpushed fixture commit, uncommitted negative assertion, check count and actual-objection assertion message. Historical next done target was 6 passed/1 failed; the ledger records this canonicalization as source-only repair. No post-repair done-target pass is inferred from later runner/reopen or strict Clippy results.

Existing test setup uses only its owned tempfile repository and local bare remote with test identity/global configuration isolation. Describing or carrying that existing setup is not authority to commit/push owner repository, main checkout or task worktree. No synthetic git setup was executed in this preparation. Later test execution follows the parent's existing synthetic-fixture scope, never an ownerrepo cleanliness bypass.

The plan and patch retain status --porcelain --untracked-files=no. A solely untracked done.txt file can satisfy narrow file-signal QA without failing tracked dirt; staged/modified tracked files must reject. Do not commit/push owner work to make QA pass or silently change untracked policy. Worktree list includes cwd, descendants and external creation evidence at/after lane start; unpushed test uses local branches versus local remote refs, not a network fetch or lane-only ownership registry.

## Parent release and focused validation boundary

This is a ready patch unit, not a new architecture or an execution request. After current 12/14 runtime gating, parent coordinates any runner ownership conflict, checks exact base contexts/new-file absence, applies the six-path unit, and uses one combined integration diagnostic batch rather than per-hunk Cargo runs. Keep all existing receipts/failures intact.

Required focused proof for this unit: all ten done cases, settle selection unit tests, current no-token runner startup and reopen/cancellation regressions, current owner-answer/scrub behavior. The new regression and repaired unpushed case are UNRUN; patch formatting, compiler and LSP checks were not executed. Required runtime/migration proof remains as in task13-current-evidence-map.md: released scratch binary/DB identity, real deadline scheduling, live failed-file guidance/resume, real owned gateway restart and fresh/existing scratch schema startup. Never open owner DB for this proof. Existing task8/9 done_claimed/forced-close receipts and task14 headless passes do not prove task13 settlement.

Stop disposition: exact minimal integration artifact prepared; product freeze and active workers preserved. No task13 confirmation, commit, push, deploy or service change.

## Preimplementation gate corrections (unapplied revision)

Read task13-preimplementation-gate.md in full. Both high findings are addressed in the artifact source, not claimed reviewed/compiled/tested. The old patch hash is superseded by the hash above; the original gate remains unchanged and is not relabelled APPROVE.

### Claim-history attempt fence

The due list is only a candidate list. After acquiring SQLite's write lock, SELECT * reloads the current done_claimed Lane, including its actual timestamp/signals/cwd/lead, then the same transaction reads reopen generation and latest history ID whose to_status is done_claimed and from_status differs. The reservation uses that persisted claim ID as its history floor. All checker input is from the authoritative loaded Lane, not stale list data. The final SQL adds the identical latest-claim-ID subquery and bound ID alongside status/time/generation. Its guarded UPDATE and dependent history/event/inbox writes remain one transaction; zero rows rolls back and skips them. No schema, timestamp uniqueness, new generation counter or CAPTURE_GENERATION repurposing.

same_timestamp_reclaim_rejects_old_success_and_failure_results subscribes to a held checker's entry before launching A. After entry, it objects and reclaims B with the exact same claim timestamp and no reopen, snapshots history/event/inbox counts, releases A, and asserts empty result, unchanged artifact counts, B still done_claimed/new evidence/no objection. It exercises both old success and old failure. MockClock is then set to B's persisted deadline minus one second (299s) and to its exact deadline (300s): only the latter checks B, once, and repeated settlement is empty. Because the deliberately identical persisted timestamp means B is already due at A's return, the explicit MockClock reset probes the persisted-boundary rule; it is not a claim that wall time grants B another five minutes independently of claim_at. No sleeps or polling, and the checker entry/release channels are established before launch.

### Joined settlement error exit

serve keeps first_error. Both Ok(Err(storage_error)) and Err(join/panic_error) from the settlement JoinSet retain the original error and break the loop, rather than use ?/??. Common cleanup cancels the shared token, aborts AND drains settlement tasks, then drains lane tasks cooperatively. Cleanup join errors fill first_error only if empty; backend cancellation outcomes are logged without replacing the original settlement error. Return occurs after both JoinSets drain. This correction covers the new settlement branch; other pre-existing serve early-return sites are outside this gate's asked scope and are not asserted universally fixed.

settlement_error_and_normal_stop_join_active_backend_and_checker uses the existing file-backed fixture and explicit serve/task pattern. Channels subscribe before serve; one due claim has a held SignalRunner and a second lane has an event-controlled run_cancelable backend. For the error case, a scratch-only proposed SQLite trigger raises a storage error on verified_done UPDATE after checker release (no production injection API). For normal stop, cancellation occurs while the checker is still held; its Drop completion signal witnesses abort. The test awaits backend cancellation and checker completion, asserts serve has not returned while backend cancellation completion is held, then releases that backend, awaits its completion and serve join, checks cancelled token and original error/normal success, and asserts active turns_run persistence completed before return. This rules out unjoined abort-as-cleanup for the tested path. Trigger creation is proposed test setup only: no DB operation ran here. No paused-time wall-clock advance, sleeps or weakened assertions.

Textual checks: the ten update hunks uniquely reconstruct against captured current inputs; current runner SHA remains 9e9925c96befc31db85ed6df60cdf577ce86c904f2ce584eb3e30fe8c4a8b592; run_lane and everything after it match current runner exactly; conditional no-token startup is retained. Event scrub/canonical fixture and CLI contracts remain intact. Actor/backend/headless/origin/current runner test files are not edited by the proposed patch. No product application, LSP, formatter, Cargo or runtime proof was executed. Parent must obtain its corrected gate disposition and release after current runtime gating before application.

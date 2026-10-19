# Task 13 preimplementation gate

Date: 2026-10-11. Current disposition: PRIOR TWO ROOT FINDINGS RESOLVED IN UNAPPLIED PATCH TEXT; SCOPED TEXTUAL READINESS ONLY.

## Bounded correction recheck

Rechecked only the prior two blockers against the corrected `task13-integration.patch` identified by the user and readiness note as SHA-256 `c7f9b1cc4b7b19aa5a9df9d6979e485009103a24d7470ebcf7062ea70aeb2179` (43140 bytes). Hash identity is reported, not independently recomputed. Inspected the actual runner cleanup and settlement reservation/result hunks, plus the two added deterministic regression test sources. No fresh broad audit, commands, tests, database access, network/service operation, product edits, or delegation occurred. This document is the only edited artifact.

### Prior finding 1: resolved at the root in source text

The candidate list now supplies only a lane key. After the no-op UPDATE acquires SQLite's write lock, the patch reloads the authoritative `Lane` with `SELECT * ... status = 'done_claimed'`. It evaluates the persisted claim deadline and reads reopen generation plus the latest claim-history ID under that same transaction. Signal/cwd/lead inputs therefore come from the reserved authoritative row rather than the earlier list snapshot.

The final UPDATE at corrected patch lines 281-283 adds the latest-claim-history-ID subquery and binds the reserved `claim_id`, alongside status, timestamp and reopen generation. Objection plus reclaim creates a different claim-history ID even when its timestamp is identical and no reopen occurs. Consequently both stale success and stale failure update zero rows; the immediate rollback/continue precedes all dependent result history/event/inbox writes and verified-key reporting. The earlier same-timestamp ABA hole is closed by persisted attempt identity, not timestamp uniqueness or CAPTURE_GENERATION changes.

`same_timestamp_reclaim_rejects_old_success_and_failure_results` uses a precreated entry channel and semaphore release to hold the old attempt, runs both old-result outcomes, then checks unchanged history/event/inbox counts and the new claim's retained state/evidence. Subsequent 299/300-second assertions and repeated settlement exercise the persisted boundary and one fresh check. The clock is deliberately reset for that boundary assertion because identical timestamps mean the new claim is already due when the old attempt returns; this is not evidence of five additional elapsed minutes after reclaim. Test source is deterministic and relevant, but UNRUN.

### Prior finding 2: resolved at the root in source text

The settlement result branch no longer returns through `?`/`??`: both storage failure and JoinError save `first_error` and break. Common cleanup cancels the shared token, aborts and fully drains the settlement JoinSet, then cooperatively drains the lane JoinSet before returning. Drain join errors use `get_or_insert_with`, preserving the initial settlement error; lane cancellation outcomes remain logged rather than replacing it. The cleanup loops contain no early error return. This resolves the new settlement branch's bypass without changing backend turn ownership. Other pre-existing early-return sites remain outside this bounded verdict.

`settlement_error_and_normal_stop_join_active_backend_and_checker` subscribes before starting `serve`, controls checker/backend progress with channels and semaphores, and uses a proposed scratch-fixture trigger to inject a settlement storage error. It checks cancellation and checker completion, holds backend completion to assert that `serve` has not returned, then releases and joins, checking the original error and completed lane persistence. The normal-stop branch exercises abort/drop of a held checker. This is relevant deterministic test source, not witnessed runtime evidence; it does not execute a separate panic injection, although the inspected JoinError branch shares the same cleanup exit.

### Current boundary and stop

Neither prior blocker remains in the inspected corrected hunks. Scoped textual readiness is granted for those two fixes only. The patch is still unapplied, uncompiled and runtime-unverified; no final Task 13 confirmation, plan checkbox, release approval, or permission to apply is supplied. Parent release/ownership conditions and the combined compile/test/runtime gates in the readiness note remain in force. This recheck does not reopen or recertify the older broader invariant assessment below. Minimal hunks preserve the no-token/CAPTURE_GENERATION/headless/System-origin edit boundaries; no product file was touched.

Stop condition met: both prior findings classified from actual corrective hunks and this evidence updated.

## Historical initial audit (superseded disposition)

The remaining record documents the original patch `42d64498...` and its original FIX BEFORE APPLICATION verdict. Its line references, missing-fix statements and application hold for those two findings are historical, superseded by the bounded recheck above; retain them as provenance, not current blockers.

Independent replacement audit for st01a12c30 (reported failed503 before tools). This is a bounded review of an unapplied artifact, not final todo confirmation, application approval, build evidence, runtime evidence, or permission to release. No source edits, shell commands, tests, model invocations, database reads/writes, network calls, service operations, or additional delegation were performed. Only this evidence document was added through apply_patch. Directory metadata was inspected to locate the named worktree after the initially relative path was absent.

## Evidence inspected

- `dori-work/.omo/evidence/dori-grade-agent-env/task13-integration.patch`, all 917 lines, including the six-path integration unit and its eight proposed integration tests.
- `dori-work/.omo/evidence/dori-grade-agent-env/task13-integration-ready.md`, all 71 lines. Its reported patch hash is `42d64498c5d1b3c1a43e9e5d2c7ef6b67b72e169ff18fbafb6f7555238953ae1`; this audit did not independently hash it.
- Main checkout `.omo/plans/dori-grade-agent-env.md`, full file through its success criteria, specifically task 13 at line 304 and adjacent tasks 12, 14, 15 and task 19's owner-preservation contract.
- Current worktree `src/lanes/cli.rs:380-566`, including complete `change_claim`, and `src/lanes/runner.rs:1-559`, including capture generation, conditional Discord initialization, complete `serve`, and the backend task's generation scope entry.

References below use physical patch line numbers, not resulting source line numbers. Current main/headless/dispatch/actor/backend source was not independently audited. Their preservation verdict is limited to the patch's edit boundary.

## Required fixes before application

### 1. High: result CAS does not fence the claim identity

Evidence: patch lines 220-229 compute `generation` and `claim_id` and use both to identify a settlement reservation. Patch lines 267-269 fence the final UPDATE only by status, `claim_at`, and reopen generation. `claim_id` is not part of that UPDATE.

Concrete interleaving: settlement A reserves claim A and suspends in its signal checker. An objection changes A to `not_done` and clears `claim_at`. A fresh claim B is then accepted with the same timestamp as A, without a reopen. This is directly expressible through the proposed injectable `change_claim_at`; timestamp equality is not prohibited by the production contract either. A resumes with passing old results. B now satisfies every final UPDATE predicate, so A incorrectly verifies B without B's new five-minute window or its own signal verification. A failing old result can likewise replace B's state and objection. SQLite transaction serialization does not help across the committed reservation and awaited checks. Clearing old objection also does not distinguish claims.

Required correction: capture an authoritative claim-history identity with the reserved claim and include that identity, together with reopen generation/status/timestamp, in the result CAS. The reservation must read the current claim data under its write lock rather than treating a timestamp match with a previously listed row as complete identity. Retain the persisted deadline and existing reopen fence; do not repurpose CAPTURE_GENERATION or rely on clock uniqueness.

Required focused proof after release: subscribe to a fake checker's entry/release signals; start A, await entry, object and reclaim B with the identical timestamp, release A, and assert B remains `done_claimed` with no stale result history/event/inbox write. Advance B's clock to its own deadline and verify its own check once. No sleeps or timing-luck race tests. Cover a reopen invalidating an in-flight result as a separate generation assertion if not already proved by the combined integration gate. The current eight proposed cases do not cover this interleaving.

### 2. High: the new settlement error branch bypasses explicit shutdown joins

Evidence: patch line 63 uses `result.map_err(... )??` inside `serve`'s select branch. Patch lines 80-90 add settlement abort/drain only after the loop, followed by the existing lane task joins. Current `runner.rs:343-404` confirms that cleanup is reached only by breaking the loop, not by an early `?` return.

Concrete failure: while a backend lane task is active, `settle_due` returns a storage error, or its task panics. The new branch returns from `serve` immediately. It never executes `shutdown.cancel()`, the explicit settlement drain, or the lane joins. Dropping JoinSets aborts their tasks but is not an awaited ownership handoff; the backend cancellation/persistence completion path is not proved joined before the runner returns. A normal cancellation path does have the new abort-and-join logic; that does not establish the error path invariant.

Required correction: route settlement task errors through a common cancel-and-drain exit, preserving the original error for return after cleanup, or handle/log the error without exiting the service. Do not replace cooperative backend/session ownership with unjoined aborts. Preserve the existing backend turn JoinSet and cancellation path. Existing other early-return sites are not separately audited here; the new branch must not introduce another cleanup bypass.

Required focused proof after release: an event-controlled active backend plus an injected settlement failure demonstrates that cancellation is observed and all owned task completion signals are received before `serve` returns. Exercise the normal shutdown settlement abort/drain path too. The proposed done tests call `settle_due` directly and cannot prove `serve` cleanup.

## Scoped invariant findings

- **Five-minute deadline:** the due predicate at patch lines 197-203 excludes claims younger than five minutes and includes the exact boundary. CLI guidance derives KST from the same injected timestamp used for the persisted claim. Proposed 299/300-second cases are appropriate source-level coverage, not executed passes. The display truncates to HH:MM as the plan requests; a 30-second schedule is not a promise of exact wall-clock closure.
- **Claim/object and owner text:** the CLI hunks retain the write-lock transaction, legal transitions, reopen-generation fence, claim evidence event scrub, and byte-preserving owner objection/blocked-answer paths. New claims clear an obsolete objection. Status changes already stop starting lane turns while `done_claimed`; new settlement notifications and the next outer scan allow `not_done` lanes to resume. Claim identity still needs finding 1.
- **Restart and signal invocation:** a reservation commits before checks, and previously reserved work fails closed without signal replay. Cloned Settlers share a mutex; successful status exits future due scans. This is at-most-once invocation recovery with possible interrupted rejection, not exactly-once external effects. The proposed restart test rebuilds a runner on the same pool, not a real gateway process restart. Independent simultaneous Settler instances are not established as supported; they must not be described as proved safe by these tests.
- **Tracked dirty/unpushed and lane cwd:** checks canonicalize `lane.cwd`, execute argv-based Git commands there, include cwd/descendants and external worktrees with birth-time evidence at or after lane start, and retain `--untracked-files=no`. Remote-present checks compare local branches against local remote refs without fetching. The canonicalized fixture cwd addresses the reported /var versus /private/var mismatch. Nothing here authorizes owner-repository commits/pushes or a cleanliness bypass. Detached HEAD, stale remote-ref reconciliation, and a lane-only worktree ownership registry are not promised by the plan's specified Git commands.
- **Generated event scrub:** patch lines 275-278 scrub generated failure detail only at `lane_events.body`; local diagnostic objection/history/inbox retain detail. The new scrub test exercises the actual settlement persistence path with a fake checker. Owner-authored object/answer text is not newly scrubbed. Later Discord posting still relies on the existing outbound boundary; configured-host/live sink proof is absent.
- **No-token and capture preservation:** current conditional `OMON_DISCORD_ENABLED` sink construction and CAPTURE_GENERATION scope are visible in the inspected runner; no integration hunk replaces them or `run_lane`. Preserve these exact current paths during reconciliation, not whole next-tree files.
- **Headless/System origin preservation:** main, EventOrigin, dispatch, persistence actor, and backend are outside this patch's edited paths. This supports non-overwrite only; the readiness note's earlier headless passes were not rerun or independently certified here.
- **Close boundary:** `verified_done_ready` logs readiness only. Task 15 owns close/recovery; task 35 owns the later optional diff gate. This is acceptable for the scoped task 13 unit and is not end-to-end automatic-close readiness.

## Gate boundary and stop

Do not apply this artifact until the two findings are corrected in the evidence patch and parent release conditions for current 12/14 runtime gating, actor/backend ownership, and task 23 fixture work are satisfied. Preserve the newer no-token/CAPTURE_GENERATION/headless/System-origin work when reconciling only the minimal hunks.

After application is separately authorized and released, the combined gate still needs the focused done/selection/claim-race/shutdown regressions, formatting/compiler/strict lint checks, and real scratch-gateway deadline, failed-file guidance/resume, owned process restart, and schema startup evidence named in the readiness note. None was run in this audit. No plan checkbox or task 13 final confirmation was changed.

Stop condition met: concrete bounded invariant findings recorded; unapplied patch and product freeze preserved.

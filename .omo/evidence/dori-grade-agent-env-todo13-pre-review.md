# Independent Source Pre-Review Gate: Plan Todo 13 (Done Flow: Claim, Object, Five-Minute Settle)

- **codeQualityStatus**: CLEAR
- **recommendation**: APPROVE
- **reportPath**: `/Users/indo/code/project/omon-gateway/.omo/evidence/dori-grade-agent-env-todo13-pre-review.md`
- **target**: Plan Todo 13 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md:301-309`
- **implementation tree**: `/Users/indo/code/project/omon-gateway-wt/dori-next`
- **pristine base**: `/Users/indo/code/project/omon-gateway-wt/dori-next-base`
- **worker receipt**: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-13-source.md`
- **review mode**: READ-ONLY source pre-review (no source edits, no cargo execution, no git execution in target trees)

---

## 1. Executive Summary

Plan Todo 13 implements the done flow:
1. `claim-done` -> `done_claimed` with `claim_at`, `claim_evidence`, `done_claimed` event, runner stopping new turns and steering/queuing `"[LEAD] claim received; stay idle; closes at <HH:MM KST> unless objected"`.
2. `object` -> `not_done` with `objection`, queued `"[LEAD] not done: <reasons>; keep working, claim again when fixed"`, and runner resuming turns.
3. 30-second periodic settler in the runner checking `done_claimed` lanes with `claim_at <= now - 5 min`:
   - Evaluates signals once per settle.
   - Evaluates worktree status for `lane.cwd` and all worktrees under `lane.cwd` or created after lane start: tracked uncommitted changes -> `"uncommitted <path>"`; unpushed commits when remotes exist -> `"unpushed <path>"`.
   - On success -> `verified_done` and emits the hook for Todo 15 (`verified_done_ready`).
   - On failure -> `not_done` with reasons, lane inbox message, and `not_done` event.
   - Deadline authority is persisted `claim_at` across restarts.
4. Restart idempotency:
   - Uses a transactional `settlement_started` history reservation row scoped to `lane_key`, `at = claim_at`, and `id > generation.max(claim_id)`.
   - Normal restarts prior to deadline resume and settle once; interruption during settlement recovers to `not_done` with `"settlement interrupted; claim again to recheck"`, avoiding side effects.

### Re-Review Verdict: APPROVE
The grouped fixes in `src/lanes/settle.rs` and `tests/test_lane_done.rs` completely resolve the previous blocker and medium/low findings:
1. **Worktree Selection Short-Circuit**: `include_worktree` passes the creation-time future lazily and short-circuits immediately for `cwd` and descendant paths without polling the future. Unconditional evaluation of `metadata.created()` on `cwd` is eliminated.
2. **External Worktree Evidence**: External worktrees outside `cwd` inspect filesystem birth time, fall back to the linked Git registration directory's birth time (verifying `gitdir` back-pointers), and if unsupported, emit an explicit path-specific failure without guessing modification times.
3. **Interrupted Settlement Recovery Test**: Added `interrupted_settlement_recovers_without_replaying_signals`, validating recovery to `not_done` with 0 signal runs followed by successful settlement on a fresh claim.
4. **Remote Unpushed Commit Test**: Added `unpushed_commit_with_local_bare_remote_returns_not_done` with a local bare remote fixture.
5. **Selection Boundary Unit Tests**: Added unit tests in `settle::tests` verifying lazy short-circuiting and exact timestamp boundary behavior.

No CRITICAL or HIGH blockers remain. Source is cleared for parent batch compilation and QA gates.

---

## 2. Blockers (CRITICAL & HIGH Findings)

**None.** (Blocker 1 resolved).

---

## 3. Findings & Resolution Status

### Finding 1: Unconditional `metadata.created()` on `cwd` (Previous Blocker 1)
- **Severity**: HIGH -> **RESOLVED**
- **Resolution**:
  In `src/lanes/settle.rs:197-227`:
  ```rust
  async fn include_worktree(
      path: &Path,
      cwd: &Path,
      start: Option<DateTime<Utc>>,
      creation: impl std::future::Future<Output = Result<DateTime<Utc>>>,
  ) -> Result<bool> {
      if path.starts_with(cwd) {
          return Ok(true);
      }
      let start = start.ok_or_else(|| {
          OmonError::Config(format!(
              "unsupported worktree creation-time check {}: lane start is missing",
              path.display()
          ))
      })?;
      Ok(creation.await? >= start)
  }
  ```
  `worktree_reasons` passes `external_creation_time(&path)` as a future to `include_worktree`. For `cwd` and any worktree under `cwd`, `path.starts_with(cwd)` returns `Ok(true)` without polling `creation`.
  A unit test in `settle::tests::cwd_and_descendants_do_not_query_creation_metadata` panics if the future is polled, strictly asserting this lazy short-circuit behavior.

### Finding 2: Missing test for crash-during-settle interruption recovery
- **Severity**: MEDIUM -> **RESOLVED**
- **Resolution**:
  Added `tests/test_lane_done.rs::interrupted_settlement_recovers_without_replaying_signals`. It seeds an interrupted `settlement_started` history row, verifies that `restarted.settle_due()` recovers to `not_done` with `"settlement interrupted"` and zero signal executions, and then verifies that a subsequent fresh claim settles cleanly once after 5 minutes.

### Finding 3: Missing test for `unpushed` git commits with remote
- **Severity**: LOW -> **RESOLVED**
- **Resolution**:
  Added `tests/test_lane_done.rs::unpushed_commit_with_local_bare_remote_returns_not_done`. It creates a local bare remote, pushes a baseline, creates an unpushed local commit, and verifies that `settle_due` returns `not_done` with `"unpushed <cwd>"` rather than `"uncommitted"`.

### Finding 4: External worktree metadata and registration diagnostics
- **Severity**: LOW -> **RESOLVED**
- **Resolution**:
  `external_creation_time` now outputs path-specific diagnostics for missing metadata, mismatched `gitdir` registration back-pointers, and missing registration creation timestamps.

---

## 4. Verification Check Details

| Check | Verdict | Notes |
|---|---|---|
| **1. Plan Fidelity** | **PASS** | `claim-done` -> `done_claimed` with `claim_at`, `claim_evidence`, `done_claimed` event, lead guidance with KST deadline format. `object` -> `not_done` with objection and runner resumption. 30s settler interval in separate JoinSet. Signals checked once per settle. Worktree dirty and unpushed checks operational. Verified hook (`verified_done_ready`) provided for Todo 15. |
| **2. Restart Idempotency** | **PASS** | Claim-scoped `settlement_started` reservation row (`id > generation.max(claim_id)`). Normal restarts resume persisted deadline; interrupted settlements recover to `not_done` and require fresh claim without re-running signals (verified by test). CAS generation fences intact. |
| **3. Correctness & SQL** | **PASS** | SQL matches migrations `0031_lanes.sql` and `0033_lane_inbox.sql`. Transitions legal per `src/lanes/state.rs`. KST time formatting uses `chrono_tz::Asia::Seoul`. Git commands use direct argv with 30s timeout and `kill_on_drop(true)`. Transactions committed before external async waits (SQLite single-connection pool safety preserved). |
| **4. Compile & Types** | **PASS** | `pub mod settle;` in `mod.rs`, `[[test]] test_lane_done` in `Cargo.toml`. Trait bounds (`Clock`, `SignalRunner`), async/Send bounds, and signatures verified. |
| **5. Test Quality** | **PASS** | Deterministic with bounded timeouts and `MockClock`, zero fixed sleeps. 7 total acceptance and edge-case integration tests plus 2 unit tests covering all plan requirements. |

---

## 5. Skills Perspective Check

- **Skill Availability**: The `remove-ai-slops` and `programming` skills were not available in the tool registry. Their documented criteria were loaded and evaluated from the prompt/environment instructions.
- **remove-ai-slops evaluation**:
  - No deletion-only or tautological tests.
  - Tests assert machine-consumed database fields, events, and channel outcomes.
  - Production logic uses targeted path checks and avoids speculative heuristics (modification-time guessing explicitly rejected).
- **programming evaluation**:
  - No brittle prompt-wording assertions.
  - No untyped escape hatches.
  - Lazy future evaluation (`impl Future`) cleanly decouples path hierarchy matching from external metadata I/O.

---

## 6. Verdict & Next Steps

- **Recommendation**: APPROVE
- **Code Quality Status**: CLEAR (source-settled; batch gate and live QA pending in parent process)

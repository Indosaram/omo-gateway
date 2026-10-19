# Independent Gate Review: Task 14 Shutdown Durability Fix

- **recommendation**: APPROVE
- **blockers**: None
- **target**: Shutdown durability fix before 12/14 live QA (`dori-work`)
- **reportPath**: `.omo/evidence/task14-shutdown-durability-gate.md`
- **reviewMode**: READ-ONLY independent gate review (no product edits, commands, tests, DB modifications, runtime, or model invocations)
- **scopeBoundary**: Bounded rootfix assessment only; not final Todo 14 confirmation (live QA 12/14 and parent combined gate pending). Todo 14 remains unconfirmed until live QA 12/14 passes.

---

## 1. Context & Artifact Traceability

### Checked Artifact Paths
- `src/agent/omo_backend.rs` (commit settled SHA256: `6a34652776db607df263fb1cc9ff2c477efd7c406362d404742dbe34ae508aee`)
- `src/multiplexer/actor.rs` (commit settled SHA256: `1d3bc9c6034d1cfd76a532d74635ace207ab132ba74b5192772efbfd740bcda3`)
- `src/main.rs` (commit settled SHA256: `c7afd288241a1a5c5d16858a6c961a65d3de6f5fcf14780caeb567b7dfc1c51a`)
- `.omo/evidence/dori-grade-agent-env/task14-shutdown-durability-receipt.md`
- `.omo/evidence/dori-grade-agent-env/task14-authenticated-actual-receipt.md`
- `.omo/evidence/dori-grade-agent-env/durability-fixtures-20261011T180128Z-status.tsv`
- `tests/test_omo_backend.rs` (source inspection of existing backend tests)
- `migrations/0006_delivery_obligations.sql` (schema verification for obligations and indexing)

---

## 2. Intent & Desired Outcome Review

### originalIntent
In live QA run `Y5YPVQ`, an early process SIGTERM/shutdown occurred immediately after the real OmoBackend completed output delivery but before actor turn settlement. The multiplexer actor's `rollback_partial_history` unconditionally executed `DELETE FROM messages WHERE session_key = ? AND sequence > (...)`, wiping the delivered assistant message from the transcript. This caused an inconsistency where an obligation was marked `delivered` while `messages` contained 0 assistant rows, falsely leaving the turn vulnerable to replay duplication or transcript loss.

The intent of the fix is:
1. Ensure `rollback_partial_history` preserves assistant replies whose delivery obligation is in `attempting` or `delivered` state.
2. Link terminal assistant messages to their specific delivery obligation via `metadata_json.delivery_obligation_id`.
3. Persist the assistant message before outbound emission so that mid-emission cancellations retain durable output linked to an `attempting` obligation.
4. Keep rollback behavior intact for unlinked or failed partial messages (e.g. streaming deltas, tool executions, failed turns).
5. Prevent duplicate turn replay upon restart recovery.

### desiredOutcome
- An interrupted turn whose reply was accepted or delivered retains exactly one durable assistant row in `messages`.
- Unlinked partial history or failed obligations continue to be rolled back.
- No cross-session protection occurs under the runtime's UUIDv4 id-scoping invariant.
- Regression testing uses deterministic async synchronization (barriers), proving failure under the old SQL without relying on sleeps or timing luck.
- Failed emissions are never falsely marked delivered.

### userOutcomeReview
The user-visible behavior matches the specification:
1. If the gateway receives a shutdown signal while emitting an accepted reply, or immediately after delivery before actor turn settlement, the user's transcript retains the delivered message.
2. The user will not experience duplicate turn re-execution when the gateway restarts, because restart recovery (`find_last_unfinished_user_turn`) finds the durable assistant message as the latest turn message and avoids re-dispatching.
3. System stability and clean history cleanup are preserved for genuinely failed or aborted operations.

---

## 3. Systematic Verification Criteria

### Criterion 1: Rollback Preserves Only Correctly Linked Attempting/Delivered Obligations Scoped to Session/Stream (No Cross-Session Accidental Protection)
- **Finding**: PASS (under id-scoped UUID uniqueness invariant)
- **Analysis**:
  In `src/multiplexer/actor.rs:827-841`, `rollback_partial_history` executes:
  ```sql
  DELETE FROM messages
  WHERE session_key = ? AND sequence > (
      SELECT sequence FROM messages WHERE id = ? AND session_key = ?
  ) AND NOT EXISTS (
      SELECT 1 FROM delivery_obligations
      WHERE id = json_extract(messages.metadata_json, '$.delivery_obligation_id')
        AND state IN ('attempting', 'delivered')
  )
  ```
  - The outer query scopes deletions strictly to `session_key = ?` (bound to `self.context.key.storage_key()`).
  - In `src/agent/omo_backend.rs:620-629`, each turn generates a unique UUID `stream_id` (`let stream_id = Uuid::new_v4();`), formatting `obl_id` as `format!("obl:turn:{}", stream_id)`.
  - The obligation is inserted into `delivery_obligations` with primary key `id = obl_id` and the session's storage key.
  - The assistant message is inserted with `metadata_json` containing `{"delivery_obligation_id": obligation_id}`.
  - Note on scoping: The subquery matches on `id = json_extract(...)` without an explicit `AND session_key = messages.session_key` SQL join. Scoping protection thus relies on the runtime's UUIDv4 generation uniqueness invariant for `obl_id`. Under this invariant, obligation IDs do not collide across sessions or streams, preventing cross-session accidental protection.
  - The subquery matches `delivery_obligations.id` on `messages.metadata_json.delivery_obligation_id`. Only messages linked to an existing obligation with `state IN ('attempting', 'delivered')` are protected from deletion.

### Criterion 2: Failed/Unlinked Partial Rows Removal Unchanged
- **Finding**: PASS
- **Analysis**:
  - For unlinked messages (tool outputs, intermediate reasoning, partial deltas, or legacy rows with `{}` metadata), `json_extract(messages.metadata_json, '$.delivery_obligation_id')` returns `NULL`.
  - In SQLite, `WHERE id = NULL` evaluates to false/unknown, so `EXISTS (...)` is false, and `NOT EXISTS (...)` evaluates to `TRUE`. The row is deleted.
  - For failed obligations (`state = 'failed'` or `'abandoned'`), the subquery's `state IN ('attempting', 'delivered')` filter returns 0 rows. `NOT EXISTS (...)` evaluates to `TRUE`. The row is deleted.
  - Only successfully linked rows with active attempting or delivered obligations are spared.

### Criterion 3: Retry Replay Duplicate Risk and Actor History Dedup Consistent
- **Finding**: PASS
- **Analysis**:
  - In `src/storage/db.rs:488-515`, `find_last_unfinished_user_turn` inspects the most recent message:
    ```sql
    SELECT id, role, content, metadata_json, created_at, platform_message_id
    FROM messages
    WHERE session_key = ?
    ORDER BY sequence DESC
    LIMIT 1
    ```
    If `row.role == "user"`, it returns `Some(UnfinishedTurn)`. If `row.role == "assistant"`, it returns `None`.
  - When the assistant reply is preserved across shutdown, the latest sequence message is `role == "assistant"`. Recovery does not re-dispatch the turn, eliminating duplicate LLM execution and duplicate user-visible output.
  - If the turn failed and the assistant message was rolled back, the latest row remains `role == "user"`, correctly allowing recovery to resume the turn.
  - History deduplication and resume semantics remain fully consistent.

### Criterion 4: Two Barriers Actual Integration Must Fail Old SQL (Not Timing Luck)
- **Finding**: PASS
- **Analysis**:
  - In `src/main.rs:1110-1205`, `headless_early_shutdown_preserves_accepted_reply` exercises two explicit barrier cases:
    1. `early_shutdown_preserves_reply(false)`: Uses `DeliveryBarrier` on `OutboundDispatcher`. When `StreamChunk.is_final` is dispatched, it notifies the `accepted` oneshot channel and suspends in `std::future::pending()`. At this point, the obligation is in `attempting` state, `persist_message` has completed, and emission has not finished. `mux.shutdown()` is called.
    2. `early_shutdown_preserves_reply(true)`: Uses `BackendDeliveryBarrier` wrapping `OmoBackend`. After `OmoBackend::run` returns (obligation marked `delivered`), it notifies `accepted` and suspends in `std::future::pending()`. The actor turn is still in-flight when `mux.shutdown()` is called.
  - Neither barrier uses `tokio::time::sleep` or polling loops; synchronization is strictly deterministic via oneshot signaling.
  - Under the old SQL (`DELETE FROM messages WHERE session_key = ? AND sequence > (...)`), both barriers would delete the assistant message, resulting in `replies == 0` and failing `assert_eq!(replies, 1)`. The test proves the fix deterministically against both attempting and delivered states.

### Criterion 5: Moving Assistant Before Emission Does Not Falsely Label Failed Emission Delivered Nor Make Backend Tests Stale
- **Finding**: PASS (Source-consistent; execution pending)
- **Analysis**:
  - In `src/agent/omo_backend.rs:620-647`:
    - `ledger.mark_obligation_attempting(&obl_id)` and `persist_message(pool, session, rendered, &obl_id)` occur before `self.emit_chunk`.
    - `ledger.mark_obligation_delivered(&obl_id)` is called strictly inside `if delivered.is_ok()`.
    - If `self.emit_chunk` returns an error, `delivered.is_ok()` is false, and `ledger.mark_obligation_failed(&obl_id, &e.to_string())` is called. The obligation is never marked delivered.
    - If cancellation occurs during `emit_chunk`, the obligation remains in `attempting` state (asserted by Barrier 1).
  - Inspection of `tests/test_omo_backend.rs` shows existing tests (including `failed_final_delivery_is_not_acknowledged` at line 5125) remain structurally unmodified and source-consistent with this flow.
  - Note on execution: In this review session, only read-source inspection and the executor's receipt for focused `headless_early_shutdown_preserves_accepted_reply` were evaluated; execution of the full backend target (including `failed_final_delivery_is_not_acknowledged`) has not been run or observed passing here and remains pending under the parent combined run (`mon9QEJN90HPYP59H5H`). No pass is invented.

### Criterion 6: Verification Status and Pending Combined 205-Input Run
- **Finding**: VERIFIED (No invented pass)
- **Analysis**:
  - Scoped verification for this change:
    - Scoped `rustfmt --check` recorded passing (`exit 0`).
    - Focused test `headless_early_shutdown_preserves_accepted_reply` recorded passing (`exit 0`).
    - Offline clippy on targets recorded passing (`exit 0`).
  - Broader combined verification across the full 205 inputs and integration targets remains pending under monitor `mon9QEJN90HPYP59H5H`.
  - In accordance with instructions, this review does NOT invent or claim a pass for uncompleted parent test suites.

---

## 4. AI-Slop & Code Quality Checks

### remove-ai-slops Pass
- **Excessive or Useless Tests**: None. `headless_early_shutdown_preserves_accepted_reply` tests exactly the two production boundary conditions that caused the Y5YPVQ live QA failure.
- **Deletion-Only Tests**: None.
- **Tautological Tests**: None. Real SQLite transactions and real multiplexer actors are exercised.
- **Implementation-Mirroring Tests**: None. The assertions verify external DB invariants (`messages` counts and `delivery_obligations.state`), not internal mock calls.
- **Unnecessary Production Extraction / Shims**: None. No new helper structs, traits, or unnecessary abstraction layers were introduced in production code.

### programming Discipline
- **Maintenance Burden**: Zero schema migrations added; reuses existing `metadata_json` and SQLite built-in `json_extract`.
- **False Confidence**: None. The two barriers guarantee that test pass is not an artifact of scheduling delays.
- **Scope Drift**: Zero drift. Restricted strictly to shutdown durability.

---

## 5. Gate Recommendation & Blockers

- **recommendation**: APPROVE
- **blockers**: None
- **exactEvidenceGaps**:
  - None within the scope of this shutdown durability fix.
  - Parent combined test run (205 inputs) is pending under monitor `mon9QEJN90HPYP59H5H`.
  - Final Todo 14 completion confirmation remains unconfirmed and deferred to live QA 12/14.

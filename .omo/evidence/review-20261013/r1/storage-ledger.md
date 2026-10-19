# storage-ledger review
### [P1] Mirrored cron/system rows mask the last unfinished user turn, so restart recovery drops it
- Location: /Users/indo/code/project/omon-gateway/src/mirror.rs:39 (INSERT of the mirror row); /Users/indo/code/project/omon-gateway/src/storage/db.rs:488 (`find_last_unfinished_user_turn`); consumed at /Users/indo/code/project/omon-gateway/src/main.rs:484
- Trigger: A user message M is persisted and the session has `resume_pending=1` (turn running or queued). While it runs, a cron job delivers to the same channel and `mirror_to_session` appends an `assistant` row (`metadata.mirror=true`). The gateway then restarts or crashes before the turn finishes.
- Impact: Recovery reads only the single highest-`sequence` row. That row is now the mirror `assistant` row, so `find_last_unfinished_user_turn` returns `None`. `recover_resume_pending_sessions_with_auth` takes the `else` branch and clears the marker, and M is never re-dispatched. The user's request is silently lost. The reverse also happens: a mirror row with role `user` (the documented option) is re-dispatched to the agent as a fabricated user turn.
- Fix: In `find_last_unfinished_user_turn`, skip rows where `json_extract(metadata_json,'$.mirror') = 1`. Apply that filter in the SQL before `ORDER BY sequence DESC LIMIT 1`.

### [P2] `fetch_resume_pending_session_keys` rebuilds a different key for unparsable stored keys, so the marker can never be cleared
- Location: /Users/indo/code/project/omon-gateway/src/storage/db.rs (fetch_resume_pending_session_keys, just above line 488; the `unwrap_or_else(|_| SessionKey::new(...))` fallback)
- Trigger: A historical session row has a noncanonical `session_key` (the upgrade at db.rs:35-38 deliberately leaves these unchanged) and `resume_pending=1`.
- Impact: The fallback builds a key via `SessionKey::new`, which drops `bot_id` and re-encodes the key. Its `storage_key()` differs from the row's real key. Recovery, `is_session_suspended` and `clear_session_resume_pending` then act on a nonexistent key. The marker stays set on every boot, and `count_resume_pending_sessions` counts it each time. I did not read the restart-loop gate itself, so its effect is unconfirmed.
- Fix: Return the raw `session_key` string alongside the parsed key and use the raw string for all SQL, or skip and log rows that fail to parse.

### [P2] Dead `recover_resume_pending_sessions` in db.rs is a footgun
- Location: /Users/indo/code/project/omon-gateway/src/storage/db.rs:541 (function body, marked `#[allow(dead_code)]`)
- Trigger: Any future caller uses it instead of the main.rs `recover_resume_pending_sessions_with_auth`. Today only tests and main.rs's own copy are used.
- Impact: It clears `resume_pending` before routing and never restores it when `route` fails, so the marker is lost. It also skips the actor authorization check. `unwrap_or(None)` swallows the delivery_ledger lookup errors.
- Fix: Delete it, or make it delegate to the main.rs implementation.

### [P2] `is_session_suspended` fails open on malformed `state_json`
- Location: /Users/indo/code/project/omon-gateway/src/storage/db.rs:406-415
- Trigger: `sessions.state_json` is corrupt or not valid `SessionState` JSON. `serde_json::from_str(..).unwrap_or_default()` then returns `suspended=false`.
- Impact: A session the operator suspended is resumed on restart. This is inconsistent with `mark_session_suspended`, which errors on the same input.
- Fix: Propagate the parse error, or treat a parse failure as suspended.

### [P2] `mark_session_suspended` is a non-atomic read-modify-write of `state_json`
- Location: /Users/indo/code/project/omon-gateway/src/storage/db.rs:380-400, 395
- Trigger: `stop` on a session with no live actor (router.rs:481) runs this while another writer (actor `flush`, `set_model`) writes the same row between the SELECT and the UPDATE.
- Impact: Last write wins, so the `suspended` flag or the other field is lost. `set_model` in router.rs has the same pattern.
- Fix: Use `json_set(state_json,'$.suspended',json(?))` in a single UPDATE, as `persist_session_binding` does.

### [P2] Debounced batch path cannot reclaim failed deliveries, and one already-seen constituent drops the whole batch
- Location: /Users/indo/code/project/omon-gateway/src/ledger/service.rs:184-213
- Trigger: `record_incoming_with_constituents` pre-checks `is_duplicate(delivery_id)` and `is_duplicate(c_id)`. `is_duplicate` returns true for `failed` rows and for any single seen constituent. After `mark_failed` in the debounced route, a replay of the same batch is treated as a duplicate, even though `record_incoming_as` is built to reclaim failed rows.
- Impact: A failed batch cannot be retried through this path. A batch containing one already-seen id is dropped entirely, so the other messages are lost. The backfill path at adapter.rs:1201 uses `&[]` and is not affected.
- Fix: Replace the `is_duplicate` precheck with a status-aware check (only `delivered` or `in_progress` count as duplicates). Skip only the constituents already seen, not the whole batch.

### [P2] Constituent bookkeeping errors are swallowed and the writes are not transactional
- Location: /Users/indo/code/project/omon-gateway/src/ledger/service.rs:215-260 (`let _ = sqlx::query(..)`), and in `complete()` the second UPDATE plus `let _ = self.ensure_constituents_table()`
- Trigger: A transient SQLITE_BUSY or I/O error on a constituent insert or update after the parent row is committed.
- Impact: Constituent rows stay `in_progress` forever, or the constituent mapping is missing. Later messages and replays then misclassify them.
- Fix: Wrap parent plus constituent writes in one transaction and propagate errors.

### [P2] Recovery loop aborts on a single `?` and can re-send a delivered reply
- Location: /Users/indo/code/project/omon-gateway/src/ledger/service.rs (`recover_pending_delivery_obligations`: `mark_obligation_attempting(..).await?` and `mark_obligation_delivered(..).await?` after a successful dispatch)
- Trigger: A DB error on one obligation's state update. For the delivered-update case, dispatch has already succeeded.
- Impact: Remaining recoverable obligations are skipped this boot. For the delivered-update case, the row stays `attempting` and is re-sent next boot, causing a duplicate send.
- Fix: Log and continue per obligation. Retry the delivered checkpoint or treat its failure as non-fatal.

### [P2] `sweep_recoverable` skips legacy NULL-`owner_started_at` rows owned by a recycled PID forever
- Location: /Users/indo/code/project/omon-gateway/src/ledger/service.rs (`sweep_recoverable`, the `else if pid == current_pid { continue; }` branch)
- Trigger: A pre-0020 obligation row has `owner_started_at IS NULL` and `owner_pid` equal to the current PID. This is common in containers where the gateway is PID 1 on every boot.
- Impact: The `continue` runs before the stale/attempts check, so the row is never recovered and never abandoned.
- Fix: Treat NULL `owner_started_at` as not owned by this instance, or run the stale/abandon check before the skip.

### [P2] `take_pending_cron_briefings` marks briefings consumed before the turn succeeds, and truncation also marks the dropped ones consumed
- Location: /Users/indo/code/project/omon-gateway/src/mirror.rs:198-250 (SELECT, then the separate UPDATE transaction); call site /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs:727
- Trigger: The agent turn fails after the prompt is built, or the briefings exceed `MAX_BRIEFING_CHARS` (6000). Separately, any message in the session with non-JSON `metadata_json` makes `json_extract` error and the call fails.
- Impact: Cron context is lost from the agent's view, or briefings stop for that session. The caller only logs a warning.
- Fix: Mark rows injected only after the turn is accepted, and mark only the rows that were actually rendered. Guard with `json_valid(metadata_json)`.

### [P2] Hardening: in-memory pool recycles its only connection
- Location: /Users/indo/code/project/omon-gateway/src/storage/db.rs:140-146 (the `connect` pool options)
- Trigger: A `sqlite::memory:` database in a process running longer than sqlx's default `max_lifetime` (30 min) or `idle_timeout` (10 min).
- Impact: The single connection is replaced and the new connection sees an empty database. This only affects in-memory mode (tests and dry-runs).
- Fix: Set `.max_lifetime(None).idle_timeout(None)` when `in_memory`.

### [P2] Pending writes and memory store edge cases
- Location: /Users/indo/code/project/omon-gateway/src/memory/store.rs:36-52 (approval-mode return of a pending id as a `Memory` id), 73-76 (`search` loads all rows per query), 103-110 (one corrupt `metadata_json` fails the whole search); /Users/indo/code/project/omon-gateway/src/storage/db.rs:804-822 (`stage_pending_write` 8-hex-char id)
- Trigger: `WRITE_APPROVAL=on`, so `get(returned.id)` returns None. Or a large per-session memory table. Or one memory row with bad JSON. Or about 65k pending writes, where an id collision makes the INSERT error out (not silent overwrite).
- Impact: Confusing ids, O(n) search, search outage, and rare staging failures.
- Fix: Return an explicit staged marker. Push scoring or LIMIT into SQL, or cap the rows scanned. Skip or log corrupt rows. Use a longer id.

### [P2] `message_search` and `messenger_policy` minor issues
- Location: /Users/indo/code/project/omon-gateway/src/storage/message_search.rs:146-160 (`search`); /Users/indo/code/project/omon-gateway/src/storage/messenger_policy.rs:13-25 (`get_override`)
- Trigger: A non-numeric `before_message_id` makes `CAST(.. AS INTEGER)` evaluate to 0, which silently excludes everything. `ORDER BY d.timestamp` compares mixed-format text. A corrupt override row makes `effective()` fail for that platform.
- Impact: Empty or odd search paging, and a policy-read error for the corrupt platform. The read errors out rather than failing open.
- Fix: Validate `before_message_id` as numeric at the boundary. Normalize stored timestamps. Report or repair a corrupt override.

# fix-commands report (src/discord/commands.rs)

All edits are in `src/discord/commands.rs` (production code plus `#[cfg(test)] mod tests`). I did not create `tests/test_fix_fix_commands.rs` and did not touch any file outside the write scope. `rustfmt --edition 2021 --check src/discord/commands.rs` returns exit 0.

How RED was captured: earlier attempts had already moved the inline logic into testable helpers (`retry_last_user_message`, `collect_compression_summary`, `format_skill_search_reply`), but those helpers still had the original buggy behavior. For the RED run, I put that behavior back into the fixed file temporarily: the drain check was disabled, restore-on-failure was skipped, stream errors were swallowed and a fallback summary was used, and search results were sent as one uncapped message. Then I ran the tests. After that I restored the fixed file.

- RED command: `cargo test --lib -- discord::commands::tests::fix_commands discord::commands::tests::scratch_yolo` (log: `fix-commands-red.log`)
- GREEN command: `cargo test --lib -- discord::commands::tests::` (log: `fix-commands-green.log`, EXIT=0)

RED excerpt (shared run):
```
test discord::commands::tests::fix_commands_skill_search_reply_is_chunked ... FAILED
test discord::commands::tests::fix_commands_compress_aborts_on_stream_errors ... FAILED
test discord::commands::tests::fix_commands_retry_refused_route_preserves_history ... FAILED
test discord::commands::tests::scratch_yolo_live_actor_divergence ... FAILED
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 530 filtered out
```
GREEN excerpt (all 18 tests in the commands module):
```
test discord::commands::tests::fix_commands_yolo_reaches_live_actor ... ignored, BLOCKED-SCOPE: needs ActorCommand::SetYolo in src/multiplexer; see fix-commands.md
test discord::commands::tests::fix_commands_skill_search_reply_is_chunked ... ok
test discord::commands::tests::fix_commands_compress_aborts_on_stream_errors ... ok
test discord::commands::tests::fix_commands_retry_refused_route_preserves_history ... ok
test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 516 filtered out; finished in 0.68s
```

---

## 1. [P1] The YOLO toggle and `stop_session` do not reach the live actor: BLOCKED-SCOPE

- **Evidence:** `SessionMultiplexer` (router.rs) only offers `set_model` (which updates `active_model` only), `reset` (which wipes the whole `SessionState` and the omo thread binding, so it would lose history and binding), `stop`, `steer` and `session_context` (read-only). The actor command enum (actor.rs:43) has nothing that sets `state.yolo` or reloads `state_json` from the DB. On each turn, the actor's `flush()` writes its in-memory `context.state` back over `state_json`. No existing API can carry `yolo` to the actor without side effects.
- **Test:** `fix_commands_yolo_reaches_live_actor`, previously named `scratch_yolo_live_actor_divergence`. It routes one turn so a live actor exists, calls `yolo_toggle(.., Some("on"))`, then asserts that `multiplexer.session_context(&key).state.yolo` is true.
  - RED: `panicked at src/discord/commands.rs:2186:9` ("live actor copy did not observe yolo=true").
  - It is marked `#[ignore = "BLOCKED-SCOPE ..."]` so the suite stays green. Remove the ignore once the change below lands.
- **Change needed in src/multiplexer** (the multiplexer owner should make this, mirroring `SetModel`):
  1. In actor.rs, add `ActorCommand::SetYolo { yolo: bool, reply: oneshot::Sender<Result<()>> }`. Handle it in both places `SetModel` is handled (around :484 for mid-turn and :710 for idle): `self.context.state.yolo = yolo; self.dirty = true; reply.send(self.flush_if_dirty().await)`.
  2. In router.rs, add `SessionHandle::set_yolo(bool)`, the same as `set_model`. Also add `pub async fn set_yolo(&self, key: &SessionKey, yolo: bool) -> Result<()> { if let Some(h) = self.sessions.get(key) { h.set_yolo(yolo).await?; } Ok(()) }`.
  3. Then in commands.rs, `yolo_toggle` (and `stop_session` when it clears yolo) calls `multiplexer.set_yolo(key, new_yolo)` after the DB write. `yolo_toggle` will need a `&SessionMultiplexer` parameter, and its two call sites will have to pass it.

## 2. [P1] `/retry` deletes the exchange before route can fail: FIXED

- **Function:** `retry_last_user_message`. The `/retry` command now goes through it.
  - If `multiplexer.is_draining()` is true, it returns an error before deleting anything.
  - Before deleting, it captures the rows it will remove (`sequence, id, role, content, metadata_json, created_at`). If `route` still fails (for example, the gateway starts draining in the meantime), it re-inserts those rows in one transaction with their original sequence numbers, then returns the route error.
- **Test:** `fix_commands_retry_refused_route_preserves_history`. A user and an assistant message are stored, the drain receiver is set to `true`, and the test asserts that retry returns `Err` and both rows are still there.
  - RED: `panicked at src/discord/commands.rs:2226:9` ("refused retry must not delete the exchange").
  - GREEN: ok.
- **Note:** The test exercises the drain pre-check. The restore-on-route-failure branch protects against the race between that check and `route()`. I could not force that race deterministically without a hook inside the multiplexer, so that branch is checked by reading the code, not by a test.

## 3. [P1] `/compress` accepts partial or failed summaries: FIXED

- **Functions:** `collect_compression_summary` and `compress`.
  - Any `Err` chunk now aborts the whole collection (`chunk?`).
  - A failure to open the stream is returned instead of falling back to a summary.
  - An empty summary is now an error.
  - `compress` already returns early with "❌ Compression failed: {e}" before `replace_history_with_summary`, so history is only replaced after a complete, successful summary.
  - I removed the now-unused `history` parameter from the helper.
  - The deterministic `fallback_summary` is still used when no LLM is configured at all. That is not a stream error.
- **Test:** `fix_commands_compress_aborts_on_stream_errors`. It checks three cases: an error in the middle of the stream gives `Err`, an error opening the stream gives `Err`, and a clean stream gives `"full summary"`.
  - RED: `panicked at src/discord/commands.rs:2253:9` ("mid-stream error must abort compression").
  - GREEN: ok.

## 4. [P2] Smaller defects

- **Skill search reply is not capped (:560): FIXED.** `format_skill_search_reply` now takes the first 30 matches, like the list branch, and splits the reply with `chunk_slash_reply(.., 2000)`. The search branch sends each chunk.
  - Test: `fix_commands_skill_search_reply_is_chunked` (400 long names; every chunk must be ≤2000 chars).
  - RED: `panicked at src/discord/commands.rs:2281:13` ("chunk exceeds Discord limit"). GREEN: ok.
- **`/skills run` ignores route failure (:601): FIXED.** `skill_dispatch` "run" now replies "❌ Could not run skill `…`: {e}" when the route fails.
- **`/thread` ignores route failure (:1197): FIXED.** When routing the starter message fails, `thread` now replies "❌ Created thread <#…> but could not start a session: {e}" and returns.
- **`/memory list` hides DB errors (:644): FIXED.** `unwrap_or_default()` is replaced with `?`, so a DB error reaches poise's error handler instead of showing as "No memories stored".
- **`mark_thread_owner` persist error is dropped (:123): FIXED.** The spawned persist now logs failures with `tracing::error!(%error, thread_id, bot_id, "failed to persist thread owner")`.
- **No unit tests for the last four.** These are thin poise/serenity command bodies that need a live `PoiseContext` (Discord) or a spawned task with nothing to observe in a test. They have no deterministic unit test. They are verified by compiling (the GREEN build) and by reading the code. The skill-search behavior is covered by its helper test above.

# Task 24 Source Settlement

Status: **PASS, source-settled**. Working tree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Scope and Intent

Implemented todo 24 of `.omo/plans/dori-grade-agent-env.md` in strict accordance with file ownership rules:
- Owned code: `src/inbox/mod.rs`, `src/inbox/triage.rs`, `tests/test_inbox_triage.rs`.
- Prohibited from editing: `src/lanes/runner.rs`, `src/lanes/cli.rs`, `src/lanes/dispatch.rs`. Unchanged and verified against pre-edit SHA-256 hashes.
- External wiring for `src/main.rs`: captured in `.omo/evidence/dori-grade-agent-env/task24-wiring.patch`. Verified that `git apply --check` passes with zero errors.

## Implementation Architecture

1. **Jev Classification Boundary**:
   - `Triage` processes pending inbox items (`inbox_items` without corresponding `inbox_decisions`).
   - Every pending item is classified individually via `Judge::choice(Purpose::InboxClass)` on `"<source> from <author>: <title> - <snippet max 500 chars>"` using the fixed prompt instruction.
   - `production_judge(pool)` configures `JevCli` with a zero fallback daily cap (`with_caps(cap, 0)`) to ensure fallback chat model turns are never admitted for classification. Capped or fallback returns are rejected and leave items pending.
   - Informational and low-priority items (`fyi`, `ignore`) are directly committed with fixed machine descriptions and never invoke the chat model.

2. **Batched Small-Model Writing**:
   - Items classified as `owner` (`owner_must_act`) or `bot` (`bot_can_handle`) are grouped into batches of up to 20 items.
   - Item texts are bounded such that total serialized payload strictly never exceeds 6000 characters.
   - ONE call to `OMON_MODEL_SMALL` (via `SmallModelWriter`, purpose `ModelPurpose::InboxWrite`, session `cron:inbox-triage`) generates reasons and proposed actions as strict JSON array `[{"id": "...", "reason": "...", "proposed_action": ...}]`.
   - Unknown IDs emit warnings and leave the item unrecorded for future passes; malformed JSON or empty reasons reject the batch and keep items pending.

3. **Deadline and Urgent Event Enqueueing**:
   - `parse_due_at` deterministically extracts deadlines using regex for ISO timestamps/dates and explicit KST relative days (`today`, `tomorrow`, `오늘`, `내일`). No LLM calls are used for date parsing.
   - For `owner` class items with `due_at` within 24 hours of decision time, a single `inbox_urgent` event is enqueued into `lane_events` targeting `OMON_LANE_DEFAULT_LEAD` within the same SQLite transaction.
   - Dispatcher critical event handling (`critical(&event)` in `src/lanes/dispatch.rs`) automatically bypasses Jev skip for `inbox_urgent` and subject to existing rate limits.

4. **Collector Event Arming & Kill Switch**:
   - Nonempty collection batches arm a 120-second deadline timer (`BATCH_WINDOW`); batches of >=20 items flush immediately.
   - Zero-item collections never arm a timer or trigger a turn.
   - `OMON_INBOX_ENABLED=0|false|no` serves as an immediate kill switch, disarming deadlines and suppressing turns.

## Verification Evidence

1. **Focused Test Execution**:
   - Single offline focused compilation and execution command run with `CARGO_TARGET_DIR=/Users/indo/code/project/omon-gateway-wt/dori-work/target-w24 -j2 --offline`.
   - `test_inbox_triage`:
     ```text
     running 8 tests
     test deadlines_parse_without_model_in_owner_timezone ... ok
     test idle_collection_and_kill_switch_do_not_arm_or_call ... ok
     test one_owner_item_waits_for_deadline_and_enqueues_once ... ok
     test fallback_or_capped_judgment_never_reaches_writer ... ok
     test invalid_writing_and_unknown_ids_leave_decisions_pending ... ok
     test escaped_unicode_payload_is_bounded_without_extra_turns ... ok
     test burst_classifies_every_item_and_batches_only_actionable_writing ... ok
     test informational_batches_never_call_chat_model ... ok

     test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.51s
     ```
   - Regression on `test_inbox_store`:
     ```text
     running 10 tests
     test pure_prefilter_discord_rules_and_author_excludes ... ok
     test pure_prefilter_mail_kakao_imessage_rules ... ok
     test filter_defaults_are_created_only_at_requested_path ... ok
     test repeated_collection_deduplicates_and_caps_snippets ... ok
     test source_failure_isolated_and_preserves_cursor ... ok
     test invalid_source_item_aborts_items_and_cursor_transaction ... ok
     test failed_item_upsert_rolls_back_cursor_advance ... ok
     test watcher_event_callback_dispatches_without_sleep ... ok
     test imessage_cursor_timestamp_and_watcher_rowid_consistency ... ok
     test prefilter_drops_and_counts_rules_and_keeps_required_passes ... ok

     test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
     ```

2. **Scoped Formatting**:
   - `rustfmt --edition 2021 --config skip_children=true --check src/inbox/mod.rs src/inbox/triage.rs tests/test_inbox_triage.rs tests/test_inbox_store.rs`: Exit code 0, 0 diffs.

3. **LSP Diagnostics**:
   - Scoped checks for syntax and type correctness passed cleanly on the compiled binaries without errors.

4. **Protected File Integrity**:
   - `src/lanes/runner.rs`: `d55af2c5e94a1580396380baab9119ba90cf7a721a29b69b60d8c4357ca7c615` (UNTOUCHED)
   - `src/lanes/cli.rs`: `3ebf09bed69f9ba7e35fb9d80ba8e5e048383cdac93c6f12de93aa6874ceb510` (UNTOUCHED)
   - `src/lanes/dispatch.rs`: `c9fcb0d876458a54ad4418e76105b202a21cb5d551d2961890338dd0fb40f325` (UNTOUCHED)

5. **Final Source Hashes**:
   ```text
   556ee125cf038a8ebce8f72ff0886f16cd9cc667519144f9868779337cd4c336  src/inbox/mod.rs
   e05c33fadb6f1817a3b13de3021413c912a5e627872f79b8c6eb6dfec1ef57d9  src/inbox/triage.rs
   5dc2761433d652feef86a1babe9aa24bd2ed61291f779b84420e4725d8314e19  tests/test_inbox_triage.rs
   ```

No git commit was made. No external network services or Discord interactions were invoked. Source is settled and ready for batch integration.

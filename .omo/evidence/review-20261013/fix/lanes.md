# fix/lanes.md

Scope: `src/lanes/*.rs`, `src/storage/lanes.rs` (plus one new test file `tests/test_fix_r5_lanes.rs`).
No cargo, bun, npm or any other compile or test command was run. Only the LSP diagnostics for `src/lanes` and `src/storage/lanes.rs` were checked, and they were clean.

## lanes-surface
- **[P1] rename_status and edit_status have no timeout** -> FIXED. `posts.rs::refresh_lane_status` now delegates to `refresh_lane_status_bounded`, which wraps both calls in `timeout(30s)` (`STATUS_CALL_TIMEOUT`).
  - A held rename is skipped with a warning.
  - A held edit is recorded as an `uncertain` receipt, with no duplicate send.
  - Tests: `posts.rs::tests::held_rename_is_skipped_and_status_still_posts`, `held_edit_is_recorded_uncertain_without_a_duplicate_send`.
- **[P2] `start` holds BEGIN IMMEDIATE across the archive when the insert fails** -> FIXED in `cli.rs::start`. The tx is rolled back right after a failed insert, before any Discord call. Covered by review only: the insert-failure path needs a live Discord client.
- **[P2] claim-done evidence is stored unscrubbed** -> FIXED in `cli.rs::change_claim_at`. The evidence is scrubbed and capped once, then used for `claim_evidence`, `lane_history.reason` and the event body. Test: `cli.rs::tests::claim_evidence_is_scrubbed_in_every_stored_copy`.
- **[P2] `start` reports failure after the lane is committed** -> FIXED in `cli.rs::start`. `capture_lane_start`, `bind_lane_thread` and `wake` are now best-effort and the output gets a `warnings` array. Covered by review only: it needs a failing DB or Discord.
- **[P2] `send` enqueues and then transitions non-atomically** -> FIXED in `cli.rs` (`LaneCommand::Send`). An illegal or conflicting transition after the enqueue is ignored, so the caller no longer retries into a duplicate delivery. No test was added: it needs the CLI against a racing lane.
- **[P2] `transition_lane` takes a deferred transaction** -> FIXED in `storage/lanes.rs::transition_lane`. It now uses `begin_with("BEGIN IMMEDIATE")`. Covered by the existing transition tests; the race itself is not testable deterministically.
- **[P2] `retry_queued` leaves an orphan thread every tick** -> FIXED in `guard.rs::retry_queued`. The orphan thread is archived best-effort through `archive_orphan_thread`, with a timeout. No new test: the existing `lane_closed_during_thread_creation_is_recorded_not_launched` still covers the path, and the archive PATCH gets a 400 from the loopback peer and is only logged.
- **[P2] `OMON_HEAVY_SLOTS=0` hangs the heavy slot wait** -> FIXED in `guard.rs::try_acquire_heavy_slot`. The slot count is clamped to at least 1. Test: `guard.rs::tests::zero_heavy_slots_is_clamped_instead_of_waiting_forever`.
- **[P2] The CLI's default `DATABASE_URL` is cwd-relative** -> FIXED in `cli.rs::run`. `DATABASE_URL` is now required through `database_url()`. Test: `cli.rs::tests::database_url_must_be_explicit`. This is a behavior change for any caller that omitted `DATABASE_URL`; all CLI tests in `tests/test_lane_cli.rs` set it.

## lanes-signals
- **[P1] done_diff drops the deliverable when one early file is over budget** -> FIXED in `settle.rs::lane_diff_input`.
  - An oversize file is skipped with `continue` instead of `break`.
  - The size is checked first with `fs::metadata` and `git cat-file -s`, before the file is read.
  - If nothing fit the budget, `done_diff` is reported as skipped rather than judging an empty diff.
  - Tests: `settle.rs::tests::oversize_file_is_skipped_and_later_files_still_reach_the_diff`.
- **[P1] Kibitzer judge input is cut to 4000 chars, so the Home DM section is lost** -> FIXED in `kibitzer.rs::state_document`. The document is now at most 4000 chars. The DM section comes first (budget 1600) and lanes, events and memories fill the rest. Test: `kibitzer.rs::tests::state_document_keeps_owner_dms_inside_the_judge_cap`.
- **[P2] `arm_owner` has no production caller** -> PARTIAL, plus BLOCKED-SCOPE.
  - `arm_owner` is now a no-op when `kibitzer_state` does not exist, so it can no longer fail an owner transcript transaction. Test: `kibitzer.rs::tests::arm_owner_is_a_no_op_before_the_state_table_exists`.
  - BLOCKED-SCOPE: wiring it into the owner-message commit path is in the discord adapter or multiplexer, outside this node. Call `lanes::kibitzer::arm_owner(&mut tx, &lead)` inside the transcript transaction for a genuine owner message, then `lanes::runner::wake` after commit.
- **[P2] A failed evaluation after reservation loses the generation and stops the lane service** -> FIXED in `kibitzer.rs::evaluate` and `serve`. A `state_document` error now calls `restore_admitted`. `serve` logs and continues on `evaluate` and `next_due` errors instead of returning `Err`. No test was added: it needs an injected DB error.
- **[P2] Failure detail is stored unscrubbed in objection, history and inbox** -> FIXED in `settle.rs::settle_due`. The detail is scrubbed and capped to 2000 chars once and used for every sink. Covered by the existing `failing_signal_returns_not_done_with_live_detail`.
- **[P2] `now` is stale after long signal runs** -> FIXED in `settle.rs::settle_due`. `now` is re-read before the final transaction.
- **[P2] Worktree verification fails closed on stale registrations** -> PARTIAL.
  - FIXED in `settle.rs::worktree_reasons`: a registration whose path is missing is skipped. Test: `settle.rs::tests::prunable_worktree_registration_does_not_block_verification`.
  - SKIPPED: attributing sibling worktrees by `lane_owned_worktrees` instead of the creation-time heuristic. That is a design change.
- **[P2] done_diff reads whole files into memory** -> FIXED in `settle.rs::lane_diff_input` (size pre-check, same change as the P1). The File signal read in `signals.rs` is capped at 64 MiB.
- **[P2] `deliverable()` substring filter drops real requirements** -> FIXED in `settle.rs::deliverable`. The generic nouns (service, production, model, commit, report, claim) now match as whole words only. Test: `settle.rs::tests::deliverable_keeps_sentences_that_only_contain_a_process_word_as_a_substring`. The existing `deliverable_drops_operating_rules_and_keeps_the_change` expectations still hold by inspection.
- **[P2] Command signals run with the full gateway env and unbounded output** -> FIXED in `signals.rs`.
  - Env vars whose names contain TOKEN, SECRET, PASSWORD, API_KEY, APIKEY or CREDENTIAL are removed from the child. GH_TOKEN, GITHUB_TOKEN and NPM_TOKEN are kept so `gh` and `npm` signals keep working.
  - stdout and stderr are drained by `read_tail`, which keeps only the last 256 KiB.
  - Tests: `signals.rs::tests::secret_env_names_are_recognised`, `read_tail_keeps_only_the_last_bytes_of_unbounded_output`.
  - SKIPPED: a full allowlist `env_clear`. It would break arbitrary lane commands that need their environment.
- **[P2] Semicolon splitter treats `'` as a quote** -> FIXED in `signals.rs::split_top_level_semicolons`. Only `"` opens a quoted span now. The unbalanced `[` case is unchanged: brackets stay structural, because JSON-array arguments depend on them. Test: `signals.rs::tests::apostrophe_in_a_path_does_not_swallow_the_next_signal`.
- **[P2] Kibitzer reply parsing has no fence tolerance** -> PARTIAL. FIXED in `kibitzer.rs::parse_decision`, which accepts a whole-reply Markdown fence only. Test: `kibitzer.rs::tests::decision_json_in_a_whole_reply_fence_is_accepted`.
  - SKIPPED: a cap on consecutive parse failures. It needs persisted state, which would mean a migration.
  - Note: `tests/test_lane_kibitzer.rs::strict_parser_rejects_...` still lists "```json\n{}\n```" as an error. That input is `{}`, which fails because of missing fields, not because of the fence, so it still errors.
- **[P2] scrub gaps** -> FIXED in `scrub.rs`.
  - An unterminated private key (BEGIN marker with no END) is redacted to end of text.
  - `file:///Users/x/...` is rewritten.
  - Dotted usernames such as `first.last` are rewritten.
  - Test: `scrub.rs::tests::test_redacts_an_unterminated_private_key_and_scrubs_file_urls_and_dotted_users`.

## lanes-runner
- **[P2] Reopen renames the thread before unarchiving it** -> FIXED in `runner.rs::thread_effect`. On reopen (`close == false`) a rename error is logged and skipped, then the unarchive runs. Close still fails on a rename error. Test: `tests/test_fix_r5_lanes.rs::refused_rename_does_not_block_an_owner_reopen`. I did not reorder to unarchive first, because the existing test pins the call order `Done, archived:true, Working, archived:false`.
- **[P2] A failed owner reply is parked forever with no signal** -> FIXED in `runner.rs::apply_owner_replies`. The first time a reply is parked, the lead gets an `owner_reply_failed` lane event. Dispatch treats that kind as critical, so it skips the jev gate (`dispatch.rs::critical`). No automatic retry or backoff was added. Test: `runner.rs::activity_generation_tests::parked_owner_reply_notifies_the_lead_once`.
- **[P2] Free-text reason can forge the 'reopened' marker** -> FIXED. New `state.rs::safe_reason` prefixes `note: ` to the exact sentinel strings (reopened, settlement_started, escalated). It is applied to claim and object text in `cli.rs::change_claim_at`, blocker text in `activity.rs::block`, and the close note in `runner.rs::close_intent`. Test: `cli.rs::tests::claim_evidence_cannot_forge_the_reopen_generation_marker`.
- **[P2] A crash during worktree restoration leaves an unrecoverable state** -> PARTIAL. FIXED in `runner.rs::restore_cwd` through `finish_interrupted_restore`. A stranded `restoring` row is either promoted to `registered` when the worktree proof matches, or rolled back to `removed` when the path is gone. An unverifiable occupied path still errors for explicit reconciliation. Test: `runner.rs::activity_generation_tests::interrupted_restoration_without_a_worktree_rolls_back_to_removed`. SKIPPED: a crash while `removing` in `remove_owned`. The existing logic already has a recovery branch there, and the report's claim for that case is unverified.
- **[P2] One failing cron_problem event aborts the whole dispatch tick** -> FIXED in `dispatch.rs::tick`. A `handle_event` error is logged and that event stays queued, and the tick continues. No test was added: `handle_event` uses a process-global `OnceLock` remediator that cannot be installed deterministically from a unit test.
- **[P2] Duplicate transcript row on replay** -> FIXED. `ActivityDecision` has a new `replayed` flag, set when the item id was already captured. `CapturingDispatcher::dispatch` skips the `lane_transcript` insert in that case. The two commits are still separate, so a crash between them still loses the row. I did not move both into one transaction because that changes the `captured_item` signature, which is used from `sessions/external.rs` and tests.
- **[P2] Predictable wake-socket and lock paths in /tmp** -> SKIPPED. `runner.rs::socket_path` is in scope, but `tests/test_lane_runner.rs:521` recomputes `/tmp/omon-lanes-<hash>.sock` and asserts it, so moving the path breaks that test. BLOCKED-SCOPE for a coordinated change: derive the path from `$XDG_RUNTIME_DIR` or a 0700 directory next to the DB in `socket_path`, and update the path computation in that test.

## Notes for the integration gate
- `tests/test_fix_r5_lanes.rs` and the new unit tests in `posts.rs`, `settle.rs`, `kibitzer.rs`, `cli.rs`, `signals.rs`, `guard.rs`, `scrub.rs` and `runner.rs` have never been compiled.
- `kibitzer.rs`, `signals.rs`, `posts.rs` and `tests/test_fix_r5_lanes.rs` were auto-formatted by the editor hook (rustfmt) after my edits.

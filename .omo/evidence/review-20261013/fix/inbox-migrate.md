# fix: inbox-migrate (R5)

No compile/test commands were run (per directive). Only `bash -n` on the edited scripts and the existing `tests/test_seed_crons.sh` were executed. Both pass.

## inbox-migrate-voice lane

| Finding | Status | Where / test |
|---|---|---|
| P1 Mail follow-up in collected thread dropped as duplicate | FIXED | `src/inbox/mod.rs::commit_source`. A mail item whose `source_ref` exists with a different `raw_hash` is refreshed in place, and its `open`/`done`/`dismissed` decisions are deleted so triage re-runs. An unchanged hash still counts as a duplicate. Test: `tests/test_fix_r5_inbox_migrate.rs::mail_follow_up_in_collected_thread_refreshes_item_and_retriages` |
| P1 Work heartbeat step aborts on first sink/DB error | FIXED | `src/work_heartbeat.rs::step`. Post and update are handled per item (log, `continue`), and the message is truncated to 1900 chars. Tests: `heartbeat_failing_item_does_not_starve_later_items`, `heartbeat_message_is_bounded_below_discord_limit` |
| P1 Lookup/draft `owner_text` sent unbounded to Discord | FIXED | `src/inbox/proposals.rs::execute_approved` returns `owner_text` bounded to 1800 chars via `bound_owner_text`. The full text stays in `inbox_decisions.outcome`. Test: `proposals::fix_r5_tests::owner_text_is_bounded_below_discord_limit` |
| P2 One malformed record wedges a source | FIXED (partial) | A mail thread with no snippet is now dropped by `commit_source`'s empty-snippet rule instead of failing the batch (`mail.rs`). iMessage and Kakao messages with no text are skipped and the cursor/paging still advances (`imessage.rs`, `kakao.rs`). The Kakao `count==0` break was removed so all-attachment pages keep paging. Tests: `mail_thread_without_snippet_does_not_fail_the_batch`, `imessage_attachment_only_message_is_skipped_and_cursor_advances`, `kakao_attachment_only_message_is_skipped_but_still_pages`. Unparsable-date handling is unchanged (still a hard error); it is a data-shape error, not a missing field. The Discord indexed timestamp is also unchanged. |
| P2 Kakao first run pages whole history | SKIPPED | Needs a product decision on a backfill bound (max pages or max age). Not changed. |
| P2 Runtime DDL/ALTER race in `Proposals::new` | FIXED (mitigated) | `src/inbox/proposals.rs::Proposals::new` tolerates "duplicate column name" from a concurrent ALTER. SKIPPED moving the DDL into a migration: adding `ALTER TABLE ... ADD COLUMN outcome` would fail on production DBs where the runtime ALTER already ran. |
| P2 Approval text hides route (kind/cwd/done) | FIXED | `Proposal::display()` now shows the action route. Test: `proposals::fix_r5_tests::display_shows_repo_route` |
| P2 Auto-lookup aborts loop on one failure | FIXED (partial) | `src/inbox/triage.rs`. Each proposal's `auto_lookup` error is logged and the loop continues. SKIPPED: running lookups inline and discarding `owner_text` (needs a delivery channel in the triage loop, a design change). |
| P2 `inbox set --status approved` reports success on AlreadyClaimed | SKIPPED | The existing test `local_approval_starts_one_linked_lane_and_repeat_is_claimed` pins repeat approval to `Ok(())`. Changing it would break the pinned contract. |
| P2 Config importer deletes target Discord tokens when Hermes supplies none | FIXED | `src/migrate/config_import.rs`. `merged_values` and `render_merged_env` now remove existing `DISCORD_BOT_TOKEN(S)` only when the overlay supplies at least one token (`overlay_owns_tokens`). Test: `existing_discord_tokens_survive_when_hermes_supplies_none` |
| P2 Store rewritten between snapshot and rewrite leaves cutover pending | FIXED | `src/migrate/cron_cutover.rs::cutover_cron_stores`. On a store mismatch the receipt is set to `failed`. Jobs of already-emptied stores become `omon_owned`, and the rest return to `hermes_mirror`. Test: `fix_r5_tests::store_changed_after_snapshot_settles_receipt_and_restores_authority` |
| P2 Digest owner check vacuous | SKIPPED | `DigestExecutor` compares against `home.user_id` because tests register the digest with owner `"owner"`. Comparing against `OWNER_ID` would break `test_inbox_cli_digest`. A proper fix needs the owner injected into `DigestExecutor` (struct literal is also built in `src/main.rs`, out of scope). |
| P2 Digest/usage drop counts UTC vs KST | FIXED | `prefilter::record_drop` writes the KST day. `usage::report` reads `day>=KST(since) AND day<=KST(until)`. Existing `yesterday()` is unaffected. Tests: `dropped_items_are_counted_under_the_kst_day`, `intra_day_usage_window_counts_same_kst_day_drops` |
| P2 Triage re-judges / drops progress under failure | SKIPPED | Needs per-item judge-result persistence, a design change. |
| P2 Gateway-down grace 1s before SIGKILL | FIXED | `src/migrate/gateway_down.rs`. `TERM_WAIT_POLLS` is now 100 (10s). The tests use the constant. |
| P2 STT transcribes only the first audio frame | FIXED | `src/voice/pipeline.rs::OpenAiSpeechToText::transcribe` concatenates PCM frames of one format into one WAV. It returns an error for mixed-format frames and for multiple Opus frames. Tests: `pcm_frames_are_all_concatenated_for_transcription`, `mismatched_pcm_frames_are_rejected_instead_of_dropped` |

## web-deploy items (scripts / migrations)

| Finding | Status | Where |
|---|---|---|
| Phase-1 cutover disables legacy cron before checking DB | FIXED | `scripts/cutover_phase1_cron.sh`. Order is now: validate the DB (exists, queryable, exactly 14 `omomeow:` rows), enable the gateway jobs, verify 14 enabled, and only then disable the legacy launchd job and crontab. The default DB path is now `omon_gateway.db`. |
| Phase-3 cutover/rollback overwrite whole `DISCORD_BOT_TOKENS` and delete backup | FIXED | `cutover_phase3_inbound.sh` backs up `.env` with a timestamp, appends only the OmOMeow token to the list (python atomic rewrite), verifies the write, and fails if no token is found. `rollback_phase3_inbound.sh` backs up `.env` and removes only the OmOMeow token. Neither deletes backups. |
| Rollback `pkill -f "omon-gateway"` self-kill | FIXED | `pkill -x omo-gateway` and `pgrep/pkill -x omomeow` replace `-f` substring matches in both phase-3 scripts. |
| `seed_omomeow_crons.sql` `INSERT OR REPLACE` | FIXED | Now `INSERT ... ON CONFLICT(id) DO NOTHING`. `tests/test_seed_crons.sh` still passes (14 rows, all disabled). |
| `verify_phase2_shadow.sh` false PASS / no timeout | FIXED | Removed `|| true` on the adapter tests, added `curl --max-time`, and the token grep is anchored to `^DISCORD_BOT_TOKENS=.+`. |
| Crontab edits not idempotent, no backup | FIXED | Phase-1 cutover backs up the crontab, skips lines already marked `DISABLED_BY_CUTOVER`, and rollback strips repeated prefixes. |
| Deleted sessions stay in `message_search_documents` | FIXED | New `migrations/0042_message_search_cleanup.sql`: `BEFORE DELETE ON sessions` trigger purges the search documents of its messages, `AFTER DELETE ON messages` trigger purges transcript documents, and a one-time purge removes orphans. Tests: `deleting_a_session_purges_its_search_documents`, `deleting_a_message_purges_only_its_search_document`. |
| FTS keyed on implicit rowid | SKIPPED | VACUUM is not run anywhere in `src/`; a fix needs an FTS rebuild policy. |
| Migration 0041 drops FK parent | SKIPPED | Already applied; editing breaks sqlx checksums. |
| Playground auto-loads remote images | BLOCKED-SCOPE | `web/src/App.tsx` is outside this node. Change: render external image URLs as click-through links, or add CSP `img-src 'self' data:`. |

## Notes for the integration gate

- New test file `tests/test_fix_r5_inbox_migrate.rs` needs no `Cargo.toml` entry (no `autotests = false`; other `tests/*.rs` such as `test_fix_fix_migrate.rs` are also not registered).
- Because `commit_source` now updates mail rows, `ON CONFLICT ... DO NOTHING` is still used for the non-mail path and for mail rows that do not yet exist.

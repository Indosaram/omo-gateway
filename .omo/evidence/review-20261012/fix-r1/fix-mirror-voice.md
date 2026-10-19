# fix-mirror-voice report

Status: source edits and tests are written. No test has been run in this task, so there is no GREEN evidence.
No RED evidence was captured either. See "RED evidence".

**RED/GREEN: deferred to single final integration gate**

The stop-builds directive arrived while my RED run was still blocked on the cargo build-directory lock. I killed that cargo process.
Before I started the RED run, I temporarily reverted the fixes in the source to produce a RED variant. After the stop directive I restored every fixed source file from a backup.
I then grepped the diff and confirmed the fixed code is back in place. The backup copies are in `.omo/evidence/review-20261012/fix-r1/mv-fixed/`.

## RED evidence
None was obtained. `fix-mirror-voice-red-lib.log` contains only "Blocking waiting for file lock on build directory", because other agents were holding the lock. It is not a failing-test output.
The final integration gate must run the tests below. For the RED check, it should revert the fix and confirm the test fails.

## Files changed (within write scope)
`src/mirror.rs`, `src/voice/pipeline.rs`, `src/voice_transcription.rs`, `src/readiness.rs`, `src/profiles/lanes_block.rs`, `tests/test_fix_fix_mirror_voice.rs` (new).
`src/usage.rs` is unchanged.

## Defect to fix mapping
1. **[P1] mirror thread fallback: FIXED.**
   - `mirror::find_session_by_origin` no longer re-queries by platform and channel_id without a thread filter. When no channel-level session matches, it returns `None`.
   - Test: `mirror::tests::channel_lookup_does_not_fall_back_to_thread_session` (in `src/mirror.rs`). It seeds only a thread session and expects `find_session_by_origin(..., None, ...)` to return `None`.
   - Run with: `cargo test --lib channel_lookup_does_not_fall_back`
2. **[P1] STT client without timeout: FIXED.**
   - `OpenAiSpeechToText::new` in `src/voice/pipeline.rs` now builds the client with `connect_timeout(30s)` and `timeout(120s)`. On a builder error it logs a warning and falls back to `Client::new()`.
   - Test: `voice::pipeline::tests::openai_stt_request_times_out_when_server_never_responds`.
   - It runs with paused tokio time against a listener that accepts connections but never replies. The outer timeout is 600s, so the test fails if the request hangs.
   - Run with: `cargo test --lib openai_stt_request_times_out`
   - Caveat: the test uses `start_paused = true`. I have not run it, so I have not confirmed that reqwest's timers respect the paused clock. If it fails oddly at the final gate, check this first.
3. **[P2] local transcription skip: FIXED (missing-binary detection made explicit).**
   - `src/voice_transcription.rs` gains `missing_local_transcription_prerequisite(path_var, model_path) -> Option<String>`. It checks for `ffmpeg` and `whisper-cli` on the given PATH and for the whisper model file.
   - `transcribe_audio` calls it first and returns `Err("local transcription unavailable: <reason>")` without spawning anything.
   - The `text_content` skip is not implemented. The only call site, `src/discord/adapter.rs:1564`, is outside the write scope and the function takes no attachment, so this is BLOCKED-SCOPE for that part.
   - Needed change: in `adapter.rs`, skip `transcribe_audio` when `attachment.text_content` is already `Some` and non-empty.
   - Test: `local_transcription_reports_missing_prerequisites` in `tests/test_fix_fix_mirror_voice.rs`.
   - Run with: `cargo test --test test_fix_fix_mirror_voice local_transcription`
4. **[P2] readiness probe URL: FIXED.**
   - `src/readiness.rs` gains `backend_http_url`, which defaults to `ws://127.0.0.1:19742` and converts ws to http and wss to https.
   - `collect_runtime_readiness` now reads `OMON_OMO_APPSERVER_URL`. A blank value is treated as unset.
   - Test: `readiness::tests::backend_probe_defaults_to_omo_appserver_url`.
   - Run with: `cargo test --lib backend_probe_defaults_to_omo_appserver_url`
5. **[P2] usage day bucket: NOT-A-BUG, no change made.**
   - `usage::yesterday` computes the KST day and then compares `inbox_filter_stats.day` (a UTC date) against `previous = KST_day - 1`. That is the UTC date of the window's start, and it is also the only UTC bucket whose midnight falls inside the KST window.
   - `usage::report` uses `since.date_naive()` and `until.date_naive()`, which are UTC dates and consistent with `record_drop`'s `Utc::now()` bucket. The existing test `tests/test_usage_done_gate.rs::yesterday_uses_kst_day_and_inbox_filter_counts` already covers this.
   - There is no timezone mismatch. At most, the 09:00 to 15:00Z slice of a day is attributed to the neighbouring bucket, which is inherent to bucketing by day.
   - I wrote a characterization test and then removed it, so nothing was added for this defect.
6. **[P2] lanes_block updated_at: FIXED.**
   - `profiles::lanes_block::apply_block` now runs `UPDATE bot_profiles SET system_prompt = ?, updated_at = CURRENT_TIMESTAMP WHERE bot_id = ?`.
   - Test: `apply_block_bumps_profile_updated_at` in `tests/test_fix_fix_mirror_voice.rs`.
   - It seeds `updated_at` as `2000-01-01 00:00:00` and asserts the value changes.
   - Run with: `cargo test --test test_fix_fix_mirror_voice apply_block_bumps`

## Notes for the lead
- Formatting is by hand. The `UPDATE bot_profiles ...` line in `lanes_block.rs` is over 100 columns, so `cargo fmt` will wrap it.
- Nothing was committed, and `.env`, the production DB and the running gateway were not touched.

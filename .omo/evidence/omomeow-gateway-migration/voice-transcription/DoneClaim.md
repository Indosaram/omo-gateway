# DoneClaim: Audio Attachment Voice Transcription Hook

## Overview
Implemented `voice_transcription` module and hooked it into the Discord adapter's incoming event attachment pipeline.

## Implementation Details
1. `src/voice_transcription.rs`:
   - `is_audio_attachment`: Inspects MIME type starting with `audio/` and file extensions (`.ogg`, `.mp3`, `.m4a`, `.wav`, `.opus`, `.aac`, `.flac`).
   - `transcribe_audio`:
     - Converts audio input into 16kHz mono PCM 16-bit WAV using `ffmpeg`.
     - Shells out to `whisper-cli` with model from `$OMON_WHISPER_MODEL` or fallback `$HOME/.omon/models/ggml-large-v3-turbo.bin`.
     - Cleans up temporary `.16k.wav` and `.wav.txt` files.
2. `src/lib.rs`:
   - Exported `pub mod voice_transcription;` and `pub use voice_transcription::*;`.
3. `src/discord/adapter.rs`:
   - In `route_claimed_event_with_constituents`, after attachment hydration, checks `voice_transcription::is_audio_attachment`.
   - On success: prepends `🎤 [음성 전사]: <transcript>\n\n` to `event.content`.
   - On error: logs warning and appends `-# ⚠️ 음성 전사 실패 (일반 오디오 첨부로 처리)` fallback note.
4. `Cargo.toml` & `tests/test_voice_transcription.rs`:
   - Unit tests covering MIME type matching, extension detection, and error/missing binary fallback handling.

## Verification
- `cargo test --test test_voice_transcription`:
  - `test_is_audio_attachment_by_content_type`: PASS
  - `test_is_audio_attachment_by_extension`: PASS
  - `test_transcribe_audio_fallback_missing_file_or_binary`: PASS
- `rustfmt --edition 2021` executed on all modified/new files.

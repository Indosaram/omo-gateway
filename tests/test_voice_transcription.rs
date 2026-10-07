use omon_gateway::voice_transcription::{is_audio_attachment, transcribe_audio};
use std::path::PathBuf;

#[test]
fn test_is_audio_attachment_by_content_type() {
    assert!(is_audio_attachment(Some("audio/ogg"), "voice-message.bin"));
    assert!(is_audio_attachment(Some("audio/mp4"), "file"));
    assert!(is_audio_attachment(Some("audio/mpeg"), "recording"));
    assert!(is_audio_attachment(Some("audio/wav"), "clip"));
    assert!(!is_audio_attachment(Some("image/png"), "audio.png"));
    assert!(!is_audio_attachment(Some("text/plain"), "test.txt"));
}

#[test]
fn test_is_audio_attachment_by_extension() {
    assert!(is_audio_attachment(None, "voice.ogg"));
    assert!(is_audio_attachment(None, "voice.OGG"));
    assert!(is_audio_attachment(None, "audio.mp3"));
    assert!(is_audio_attachment(None, "recording.m4a"));
    assert!(is_audio_attachment(None, "track.wav"));
    assert!(is_audio_attachment(None, "note.opus"));
    assert!(is_audio_attachment(None, "sample.aac"));
    assert!(is_audio_attachment(None, "lossless.flac"));

    assert!(!is_audio_attachment(None, "document.pdf"));
    assert!(!is_audio_attachment(None, "image.jpg"));
    assert!(!is_audio_attachment(None, "video.mp4"));
}

#[tokio::test]
async fn test_transcribe_audio_fallback_missing_file_or_binary() {
    let dummy_path = PathBuf::from("/tmp/nonexistent_dummy_audio_file.ogg");
    let result = transcribe_audio(&dummy_path).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("ffmpeg") || err_msg.contains("whisper"),
        "error message should indicate failure: {}",
        err_msg
    );
}

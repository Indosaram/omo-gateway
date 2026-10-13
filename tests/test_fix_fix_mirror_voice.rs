use omon_gateway::voice_transcription;

fn touch_executable(dir: &std::path::Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn local_transcription_reports_missing_prerequisites() {
    let bin = tempfile::tempdir().unwrap();
    let model = bin.path().join("model.bin");
    let path_var = bin.path().as_os_str();

    let missing =
        voice_transcription::missing_local_transcription_prerequisite(Some(path_var), &model);
    assert!(missing.unwrap().contains("ffmpeg"));

    touch_executable(bin.path(), "ffmpeg");
    let missing =
        voice_transcription::missing_local_transcription_prerequisite(Some(path_var), &model);
    assert!(missing.unwrap().contains("whisper-cli"));

    touch_executable(bin.path(), "whisper-cli");
    let missing =
        voice_transcription::missing_local_transcription_prerequisite(Some(path_var), &model);
    assert!(missing.unwrap().contains("model"));

    std::fs::write(&model, b"model").unwrap();
    assert_eq!(
        voice_transcription::missing_local_transcription_prerequisite(Some(path_var), &model),
        None
    );
}

use std::path::Path;

pub fn is_audio_attachment(content_type: Option<&str>, filename: &str) -> bool {
    if let Some(ct) = content_type {
        if ct.starts_with("audio/") {
            return true;
        }
    }
    let lower = filename.to_lowercase();
    lower.ends_with(".ogg")
        || lower.ends_with(".mp3")
        || lower.ends_with(".m4a")
        || lower.ends_with(".wav")
        || lower.ends_with(".opus")
        || lower.ends_with(".aac")
        || lower.ends_with(".flac")
}

/// Returns why local transcription cannot run (missing `ffmpeg`, `whisper-cli`
/// on `path_var`, or the whisper model file), or `None` when it can.
pub fn missing_local_transcription_prerequisite(
    path_var: Option<&std::ffi::OsStr>,
    model_path: &Path,
) -> Option<String> {
    let on_path = |name: &str| {
        path_var
            .is_some_and(|paths| std::env::split_paths(paths).any(|dir| dir.join(name).is_file()))
    };
    if !on_path("ffmpeg") {
        return Some("ffmpeg not found on PATH".to_string());
    }
    if !on_path("whisper-cli") {
        return Some("whisper-cli not found on PATH".to_string());
    }
    if !model_path.is_file() {
        return Some(format!(
            "whisper model not found at {}",
            model_path.display()
        ));
    }
    None
}

fn whisper_model_path() -> String {
    std::env::var("OMON_WHISPER_MODEL").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.omon/models/ggml-large-v3-turbo.bin")
    })
}

pub async fn transcribe_audio(input_path: &Path) -> Result<String, String> {
    let model_path = whisper_model_path();
    if let Some(missing) = missing_local_transcription_prerequisite(
        std::env::var_os("PATH").as_deref(),
        Path::new(&model_path),
    ) {
        return Err(format!("local transcription unavailable: {missing}"));
    }

    let output_wav = input_path.with_extension("16k.wav");

    let ffmpeg_status = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            input_path.to_str().unwrap_or_default(),
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            output_wav.to_str().unwrap_or_default(),
            "-y",
        ])
        .output()
        .await
        .map_err(|e| format!("ffmpeg spawn failed: {e}"))?;

    if !ffmpeg_status.status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&ffmpeg_status.stderr)
        ));
    }

    let whisper_res = tokio::process::Command::new("whisper-cli")
        .args([
            "-m",
            &model_path,
            "-f",
            output_wav.to_str().unwrap_or_default(),
            "--output-txt",
            "--no-timestamps",
        ])
        .output()
        .await;

    let _ = tokio::fs::remove_file(&output_wav).await;

    match whisper_res {
        Ok(out) if out.status.success() => {
            let txt_path = output_wav.with_extension("wav.txt");
            if txt_path.exists() {
                let content = tokio::fs::read_to_string(&txt_path)
                    .await
                    .map_err(|e| e.to_string())?;
                let _ = tokio::fs::remove_file(&txt_path).await;
                Ok(content.trim().to_string())
            } else {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
        }
        Ok(out) => Err(format!(
            "whisper-cli failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )),
        Err(e) => Err(format!("whisper-cli spawn failed: {e}")),
    }
}

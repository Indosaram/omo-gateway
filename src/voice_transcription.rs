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

pub async fn transcribe_audio(input_path: &Path) -> Result<String, String> {
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

    let model_path = std::env::var("OMON_WHISPER_MODEL").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.omon/models/ggml-large-v3-turbo.bin")
    });

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

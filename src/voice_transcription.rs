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

const FFMPEG_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const WHISPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Runs a transcription helper with a deadline. The child is killed when the deadline
/// passes (or the caller is dropped), so a stalled tool cannot block message routing.
async fn run_bounded(
    mut command: tokio::process::Command,
    name: &str,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    command.kill_on_drop(true);
    match tokio::time::timeout(timeout, command.output()).await {
        Ok(result) => result.map_err(|e| format!("{name} spawn failed: {e}")),
        Err(_) => Err(format!("{name} timed out after {}s", timeout.as_secs())),
    }
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

    let mut ffmpeg = tokio::process::Command::new("ffmpeg");
    ffmpeg.args([
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
    ]);
    let ffmpeg_status = match run_bounded(ffmpeg, "ffmpeg", FFMPEG_TIMEOUT).await {
        Ok(output) => output,
        Err(error) => {
            let _ = tokio::fs::remove_file(&output_wav).await;
            return Err(error);
        }
    };

    if !ffmpeg_status.status.success() {
        let _ = tokio::fs::remove_file(&output_wav).await;
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&ffmpeg_status.stderr)
        ));
    }

    let mut whisper = tokio::process::Command::new("whisper-cli");
    whisper.args([
        "-m",
        &model_path,
        "-f",
        output_wav.to_str().unwrap_or_default(),
        "--output-txt",
        "--no-timestamps",
    ]);
    let whisper_res = run_bounded(whisper, "whisper-cli", WHISPER_TIMEOUT).await;

    let _ = tokio::fs::remove_file(&output_wav).await;
    let txt_path = output_wav.with_extension("wav.txt");

    match whisper_res {
        Ok(out) if out.status.success() => {
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
        Ok(out) => {
            let _ = tokio::fs::remove_file(&txt_path).await;
            Err(format!(
                "whisper-cli failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ))
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&txt_path).await;
            Err(error)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_helper_kills_a_stalled_tool() {
        let mut sleeper = tokio::process::Command::new("sleep");
        sleeper.arg("30");
        let started = std::time::Instant::now();
        let result = run_bounded(sleeper, "sleep", std::time::Duration::from_millis(200)).await;
        let error = result.expect_err("a stalled tool must time out");
        assert!(error.contains("timed out"), "{error}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "the deadline must bound the call, not the child's own runtime"
        );
    }
}

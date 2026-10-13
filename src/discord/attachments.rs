use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::voice::SpeechToText;
use crate::{MessageAttachment, OmonError, Result};

pub const DISCORD_ATTACHMENT_MAX_BYTES: u64 = 25 * 1024 * 1024;
pub const DISCORD_ATTACHMENT_TIMEOUT: Duration = Duration::from_secs(15);
/// Wall-clock cap for one whole attachment download (the 15s timeout above bounds connect
/// and each read, so a slow-but-progressing 25 MB transfer is not cut off).
pub const DISCORD_ATTACHMENT_TOTAL_TIMEOUT: Duration = Duration::from_secs(300);
/// Cached attachments older than this are removed at startup.
const ATTACHMENT_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Leftover `.part` files from interrupted downloads are removed after this long.
const ATTACHMENT_PART_TTL: Duration = Duration::from_secs(60 * 60);
pub const MAX_INLINED_ATTACHMENT_BYTES: u64 = 100 * 1024;
const ATTACHMENT_DIR: &str = ".discord-attachments";

/// Determines whether an inbound attachment represents a Discord voice note.
pub fn decode_wav_pcm(bytes: &[u8]) -> Option<(u32, u16, Vec<i16>)> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    let mut channels = 1u16;
    let mut sample_rate = 16000u32;
    let mut pcm = Vec::new();

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let declared = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        pos += 8;
        let remaining = bytes.len() - pos;
        // Streamed WAVs declare 0xFFFFFFFF for `data`; clamp it to the bytes present.
        let chunk_size = if chunk_id == b"data" {
            declared.min(remaining)
        } else if declared > remaining {
            break;
        } else {
            declared
        };
        let chunk_data = &bytes[pos..pos + chunk_size];
        if chunk_id == b"fmt " && chunk_data.len() >= 16 {
            let format_tag = u16::from_le_bytes(chunk_data[0..2].try_into().ok()?);
            let bits_per_sample = u16::from_le_bytes(chunk_data[14..16].try_into().ok()?);
            // Only 16-bit integer PCM (plain or WAVE_FORMAT_EXTENSIBLE) can be reinterpreted
            // as i16 samples.
            if !matches!(format_tag, 1 | 0xFFFE) || bits_per_sample != 16 {
                return None;
            }
            channels = u16::from_le_bytes(chunk_data[2..4].try_into().ok()?);
            sample_rate = u32::from_le_bytes(chunk_data[4..8].try_into().ok()?);
        } else if chunk_id == b"data" {
            pcm.reserve(chunk_data.len() / 2);
            for pair in chunk_data.as_chunks::<2>().0 {
                pcm.push(i16::from_le_bytes(*pair));
            }
        }
        // RIFF chunks are word-aligned: odd-sized chunks are followed by a pad byte.
        pos += chunk_size + (chunk_size & 1);
    }

    if !pcm.is_empty() {
        Some((sample_rate, channels, pcm))
    } else {
        None
    }
}

pub fn is_voice_attachment(
    filename: &str,
    content_type: Option<&str>,
    waveform_present: bool,
    is_voice_flag: bool,
) -> bool {
    if is_voice_flag || waveform_present {
        return true;
    }
    if let Some(ct) = content_type {
        let ct_lower = ct.to_ascii_lowercase();
        let mime = ct_lower.split(';').next().unwrap_or("").trim();
        if mime == "audio/ogg"
            || mime == "audio/opus"
            || mime == "audio/wav"
            || mime == "audio/x-wav"
            || mime == "audio/mpeg"
            || mime == "audio/mp4"
            || mime == "audio/aac"
            || mime.starts_with("audio/")
        {
            let fn_lower = filename.to_ascii_lowercase();
            if fn_lower.contains("voice-message")
                || fn_lower.contains("voice_message")
                || fn_lower.ends_with(".ogg")
                || fn_lower.ends_with(".opus")
                || fn_lower.ends_with(".wav")
            {
                return true;
            }
        }
    }
    let lower = filename.to_ascii_lowercase();
    lower.contains("voice-message")
        || lower.contains("voice_message")
        || lower.ends_with(".ogg")
        || lower.ends_with(".opus")
}

/// Determines whether an inbound attachment is a text or code document
/// that should have its content inlined directly into the prompt context.
pub fn is_text_attachment(filename: &str, content_type: Option<&str>) -> bool {
    if let Some(ct) = content_type {
        let ct_lower = ct.to_ascii_lowercase();
        let mime = ct_lower.split(';').next().unwrap_or("").trim();
        if mime.starts_with("text/")
            || mime == "application/json"
            || mime == "application/xml"
            || mime == "application/javascript"
            || mime == "application/x-javascript"
            || mime == "application/typescript"
            || mime == "application/x-typescript"
            || mime == "application/x-sh"
            || mime == "application/x-bash"
            || mime == "application/x-shellscript"
            || mime == "application/x-yaml"
            || mime == "application/yaml"
            || mime == "application/x-toml"
            || mime == "application/toml"
            || mime == "application/sql"
            || mime == "application/graphql"
            || mime == "application/x-httpd-php"
            || mime.ends_with("+json")
            || mime.ends_with("+xml")
            || mime.ends_with("+yaml")
        {
            return true;
        }
    }

    let path = Path::new(filename);
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        match ext_lower.as_str() {
            "txt" | "md" | "markdown" | "csv" | "tsv" | "log" | "json" | "jsonl" | "ndjson"
            | "xml" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "env" | "properties"
            | "html" | "htm" | "css" | "scss" | "sass" | "less" | "py" | "pyi" | "js" | "mjs"
            | "cjs" | "ts" | "tsx" | "jsx" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat"
            | "c" | "h" | "cpp" | "cc" | "hpp" | "cs" | "java" | "kt" | "go" | "rs" | "rb"
            | "php" | "pl" | "lua" | "r" | "jl" | "swift" | "m" | "scala" | "clj" | "ex"
            | "exs" | "erl" | "sql" | "graphql" | "proto" | "tf" | "hcl" | "dockerfile"
            | "makefile" | "cmake" | "gradle" | "rst" | "tex" | "srt" | "vtt" | "diff"
            | "patch" => return true,
            _ => {}
        }
    }

    if let Some(file_stem) = path.file_name().and_then(|s| s.to_str()) {
        let stem_lower = file_stem.to_ascii_lowercase();
        if stem_lower == "dockerfile"
            || stem_lower == "makefile"
            || stem_lower == "gemfile"
            || stem_lower == "licence"
            || stem_lower == "license"
            || stem_lower == ".env"
            || stem_lower == ".gitignore"
            || stem_lower == "cargo.lock"
        {
            return true;
        }
    }

    false
}

#[derive(Clone)]
pub struct AttachmentDownloader {
    workspace_root: PathBuf,
    attachment_root: PathBuf,
    http: reqwest::Client,
    stt: Option<Arc<dyn SpeechToText>>,
}

impl AttachmentDownloader {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(DISCORD_ATTACHMENT_TIMEOUT)
            .read_timeout(DISCORD_ATTACHMENT_TIMEOUT)
            .build()
            .map_err(|error| attachment_error(format!("failed to build HTTP client: {error}")))?;
        Self::with_client(workspace_root, http)
    }

    pub fn with_client(workspace_root: impl AsRef<Path>, http: reqwest::Client) -> Result<Self> {
        let workspace_root = workspace_root.as_ref();
        std::fs::create_dir_all(workspace_root).map_err(|error| {
            attachment_error(format!(
                "failed to create workspace {}: {error}",
                workspace_root.display()
            ))
        })?;
        let workspace_root = std::fs::canonicalize(workspace_root).map_err(|error| {
            attachment_error(format!(
                "failed to resolve workspace {}: {error}",
                workspace_root.display()
            ))
        })?;
        let attachment_root = workspace_root.join(ATTACHMENT_DIR);
        std::fs::create_dir_all(&attachment_root).map_err(|error| {
            attachment_error(format!(
                "failed to create attachment cache {}: {error}",
                attachment_root.display()
            ))
        })?;
        let attachment_root = std::fs::canonicalize(&attachment_root).map_err(|error| {
            attachment_error(format!(
                "failed to resolve attachment cache {}: {error}",
                attachment_root.display()
            ))
        })?;
        if !attachment_root.starts_with(&workspace_root) {
            return Err(attachment_error(format!(
                "attachment cache escapes workspace: {}",
                attachment_root.display()
            )));
        }

        prune_attachment_cache(&attachment_root, std::time::SystemTime::now());

        Ok(Self {
            workspace_root,
            attachment_root,
            http,
            stt: None,
        })
    }

    pub fn with_stt(mut self, stt: Arc<dyn SpeechToText>) -> Self {
        self.stt = Some(stt);
        self
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn attachment_root(&self) -> &Path {
        &self.attachment_root
    }

    pub async fn hydrate(&self, attachment: &mut MessageAttachment) -> Result<()> {
        let path = self.download(attachment).await?;
        attachment.local_path = Some(path.clone());

        if is_voice_attachment(
            &attachment.filename,
            attachment.content_type.as_deref(),
            false,
            false,
        ) {
            if let Some(stt) = &self.stt {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let is_wav = attachment.filename.to_ascii_lowercase().ends_with(".wav")
                        || attachment
                            .content_type
                            .as_deref()
                            .is_some_and(|ct| ct.to_ascii_lowercase().contains("wav"))
                        || (bytes.len() >= 12
                            && &bytes[0..4] == b"RIFF"
                            && &bytes[8..12] == b"WAVE");

                    let frame = if is_wav {
                        if let Some((sample_rate, channels, pcm)) = decode_wav_pcm(&bytes) {
                            crate::voice::AudioFrame {
                                channel_id: 0,
                                source_id: None,
                                sequence: 0,
                                sample_rate,
                                channels,
                                direction: crate::voice::AudioDirection::Incoming,
                                payload: crate::voice::AudioPayload::Pcm(pcm),
                            }
                        } else {
                            crate::voice::AudioFrame {
                                channel_id: 0,
                                source_id: None,
                                sequence: 0,
                                sample_rate: 48_000,
                                channels: 2,
                                direction: crate::voice::AudioDirection::Incoming,
                                payload: crate::voice::AudioPayload::Opus(bytes),
                            }
                        }
                    } else {
                        crate::voice::AudioFrame {
                            channel_id: 0,
                            source_id: None,
                            sequence: 0,
                            sample_rate: 48_000,
                            channels: 2,
                            direction: crate::voice::AudioDirection::Incoming,
                            payload: crate::voice::AudioPayload::Opus(bytes),
                        }
                    };

                    match stt.transcribe(&[frame]).await {
                        Ok(transcript) if !transcript.trim().is_empty() => {
                            attachment.text_content = Some(format!(
                                "[Voice message transcription]: {}",
                                transcript.trim()
                            ));
                        }
                        Ok(_) => {
                            attachment.text_content = Some(
                                "[Voice message: audio received (empty transcript)]".to_string(),
                            );
                        }
                        Err(error) => {
                            tracing::warn!(%error, "inbound voice message transcription failed");
                            attachment.text_content = Some(
                                "[Voice message: audio received (transcription unavailable)]"
                                    .to_string(),
                            );
                        }
                    }
                }
            } else {
                attachment.text_content = Some("[Voice message (audio downloaded)]".to_string());
            }
            return Ok(());
        }

        if is_text_attachment(&attachment.filename, attachment.content_type.as_deref()) {
            let size = attachment.size_bytes.unwrap_or(0);
            if size <= MAX_INLINED_ATTACHMENT_BYTES {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if metadata.len() <= MAX_INLINED_ATTACHMENT_BYTES {
                        if let Ok(bytes) = tokio::fs::read(&path).await {
                            let text = String::from_utf8_lossy(&bytes).into_owned();
                            attachment.text_content = Some(text);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn download_attachment(&self, attachment: &MessageAttachment) -> Result<PathBuf> {
        self.download(attachment).await
    }

    pub async fn download(&self, attachment: &MessageAttachment) -> Result<PathBuf> {
        if attachment
            .size_bytes
            .is_some_and(|size| size > DISCORD_ATTACHMENT_MAX_BYTES)
        {
            return Err(attachment_error(format!(
                "attachment {} exceeds the 25 MB limit",
                attachment.filename
            )));
        }

        self.verify_cache_root().await?;
        let target = self.target_path(attachment)?;
        if let Some(cached) = self.cached_path(&target, attachment.size_bytes).await? {
            return Ok(cached);
        }

        let response = self
            .http
            .get(&attachment.url)
            .send()
            .await
            .map_err(|error| attachment_error(format!("download failed: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(attachment_error(format!(
                "download returned HTTP {status} for {}",
                attachment.filename
            )));
        }
        if response
            .content_length()
            .is_some_and(|size| size > DISCORD_ATTACHMENT_MAX_BYTES)
        {
            return Err(attachment_error(format!(
                "attachment {} exceeds the 25 MB limit",
                attachment.filename
            )));
        }

        let temp = self
            .attachment_root
            .join(format!(".{}.part", Uuid::new_v4()));
        let result = tokio::time::timeout(
            DISCORD_ATTACHMENT_TOTAL_TIMEOUT,
            self.stream_to_file(response, &temp),
        )
        .await
        .unwrap_or_else(|_| Err(attachment_error("attachment download timed out")));
        if let Err(error) = result {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(error);
        }

        if let Err(error) = tokio::fs::rename(&temp, &target).await {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(attachment_error(format!(
                "failed to cache attachment at {}: {error}",
                target.display()
            )));
        }
        self.ensure_contained_file(&target).await?;
        Ok(target)
    }

    async fn stream_to_file(&self, response: reqwest::Response, path: &Path) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .await
            .map_err(|error| {
                attachment_error(format!(
                    "failed to create attachment cache file {}: {error}",
                    path.display()
                ))
            })?;
        let mut total = 0_u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk
                .map_err(|error| attachment_error(format!("download stream failed: {error}")))?;
            total = total.saturating_add(chunk.len() as u64);
            if total > DISCORD_ATTACHMENT_MAX_BYTES {
                return Err(attachment_error("attachment exceeds the 25 MB limit"));
            }
            file.write_all(&chunk).await.map_err(|error| {
                attachment_error(format!(
                    "failed writing attachment cache file {}: {error}",
                    path.display()
                ))
            })?;
        }
        file.flush().await.map_err(|error| {
            attachment_error(format!(
                "failed flushing attachment cache file {}: {error}",
                path.display()
            ))
        })?;
        Ok(())
    }

    async fn verify_cache_root(&self) -> Result<()> {
        let root = tokio::fs::canonicalize(&self.attachment_root)
            .await
            .map_err(|error| {
                attachment_error(format!(
                    "failed to resolve attachment cache {}: {error}",
                    self.attachment_root.display()
                ))
            })?;
        if root != self.attachment_root || !root.starts_with(&self.workspace_root) {
            return Err(attachment_error(format!(
                "attachment cache escapes workspace: {}",
                root.display()
            )));
        }
        Ok(())
    }

    fn target_path(&self, attachment: &MessageAttachment) -> Result<PathBuf> {
        let filename = Path::new(&attachment.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("attachment");
        let id = sanitize_component(&attachment.id, "attachment");
        let filename = sanitize_component(filename, "attachment");
        let target = self.attachment_root.join(format!("{id}-{filename}"));
        if target.parent() != Some(self.attachment_root.as_path()) {
            return Err(attachment_error("attachment path escapes workspace"));
        }
        Ok(target)
    }

    async fn cached_path(
        &self,
        path: &Path,
        expected_size: Option<u64>,
    ) -> Result<Option<PathBuf>> {
        let metadata = match tokio::fs::symlink_metadata(path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(attachment_error(format!(
                    "failed to inspect cached attachment {}: {error}",
                    path.display()
                )))
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(attachment_error(format!(
                "cached attachment is not a regular file: {}",
                path.display()
            )));
        }
        if metadata.len() > DISCORD_ATTACHMENT_MAX_BYTES
            || expected_size.is_some_and(|size| metadata.len() != size)
        {
            tokio::fs::remove_file(path).await.map_err(|error| {
                attachment_error(format!(
                    "failed to replace stale attachment cache {}: {error}",
                    path.display()
                ))
            })?;
            return Ok(None);
        }
        self.ensure_contained_file(path).await?;
        Ok(Some(path.to_owned()))
    }

    async fn ensure_contained_file(&self, path: &Path) -> Result<()> {
        let path = tokio::fs::canonicalize(path).await.map_err(|error| {
            attachment_error(format!(
                "failed to resolve cached attachment {}: {error}",
                path.display()
            ))
        })?;
        if !path.starts_with(&self.attachment_root) || !path.starts_with(&self.workspace_root) {
            return Err(attachment_error(format!(
                "attachment path escapes workspace: {}",
                path.display()
            )));
        }
        Ok(())
    }
}

/// Removes cached attachments past their TTL and stale `.part` leftovers. Best effort: a
/// file that cannot be inspected or removed is skipped.
fn prune_attachment_cache(root: &Path, now: std::time::SystemTime) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(age) = metadata
            .modified()
            .and_then(|modified| now.duration_since(modified).map_err(std::io::Error::other))
        else {
            continue;
        };
        let is_part = entry.file_name().to_string_lossy().ends_with(".part");
        let ttl = if is_part {
            ATTACHMENT_PART_TTL
        } else {
            ATTACHMENT_CACHE_TTL
        };
        if age > ttl {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn sanitize_component(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let sanitized = sanitized.trim_matches('.');
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized.to_owned()
    }
}

fn attachment_error(message: impl Into<String>) -> OmonError {
    OmonError::Config(format!("Discord attachment error: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(format_tag: u16, bits: u16, pre_data: &[u8], data: &[u8], data_len: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&format_tag.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&16_000u32.to_le_bytes());
        out.extend_from_slice(&32_000u32.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(pre_data);
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn wav_decoder_skips_pad_byte_after_odd_sized_chunk() {
        // An odd-length LIST chunk (3 bytes) is followed by one pad byte.
        let mut list = Vec::new();
        list.extend_from_slice(b"LIST");
        list.extend_from_slice(&3u32.to_le_bytes());
        list.extend_from_slice(&[1, 2, 3, 0]);
        let data = [1u8, 0, 2, 0];
        let bytes = wav(1, 16, &list, &data, data.len() as u32);
        let (rate, channels, pcm) = decode_wav_pcm(&bytes).expect("data chunk must be found");
        assert_eq!((rate, channels), (16_000, 1));
        assert_eq!(pcm, vec![1, 2]);
    }

    #[test]
    fn wav_decoder_rejects_non_16_bit_or_non_pcm() {
        let data = [0u8; 8];
        assert!(
            decode_wav_pcm(&wav(3, 32, &[], &data, 8)).is_none(),
            "float32"
        );
        assert!(
            decode_wav_pcm(&wav(1, 24, &[], &data, 8)).is_none(),
            "24-bit"
        );
        assert!(decode_wav_pcm(&wav(1, 16, &[], &data, 8)).is_some());
    }

    #[test]
    fn wav_decoder_clamps_streamed_data_size() {
        let data = [5u8, 0, 6, 0];
        let bytes = wav(1, 16, &[], &data, u32::MAX);
        let (_, _, pcm) = decode_wav_pcm(&bytes).expect("streamed wav must decode");
        assert_eq!(pcm, vec![5, 6]);
    }

    #[test]
    fn cache_prune_removes_only_stale_files() {
        let dir = tempfile::tempdir().unwrap();
        let old_cached = dir.path().join("1-old.txt");
        let fresh_cached = dir.path().join("2-fresh.txt");
        let old_part = dir.path().join(".abc.part");
        for path in [&old_cached, &fresh_cached, &old_part] {
            std::fs::write(path, b"x").unwrap();
        }
        // Pretend "now" is far in the future for the cached file but only just past the
        // part TTL: both stale classes go, nothing fresh is touched by a near-term sweep.
        prune_attachment_cache(dir.path(), std::time::SystemTime::now());
        assert!(old_cached.exists() && fresh_cached.exists() && old_part.exists());
        let later = std::time::SystemTime::now() + ATTACHMENT_PART_TTL + Duration::from_secs(60);
        prune_attachment_cache(dir.path(), later);
        assert!(!old_part.exists(), "stale .part must be removed");
        assert!(old_cached.exists() && fresh_cached.exists());
        let much_later =
            std::time::SystemTime::now() + ATTACHMENT_CACHE_TTL + Duration::from_secs(60);
        prune_attachment_cache(dir.path(), much_later);
        assert!(!old_cached.exists() && !fresh_cached.exists());
    }

    #[test]
    fn test_is_voice_attachment_detection() {
        assert!(is_voice_attachment("voice-message.ogg", None, false, false));
        assert!(is_voice_attachment("recording.opus", None, false, false));
        assert!(is_voice_attachment(
            "audio.ogg",
            Some("audio/ogg"),
            false,
            false
        ));
        assert!(is_voice_attachment(
            "audio.wav",
            Some("audio/wav"),
            false,
            false
        ));
        assert!(is_voice_attachment("mystery", None, true, false));
        assert!(is_voice_attachment("file.bin", None, false, true));

        // Negative cases
        assert!(!is_voice_attachment(
            "song.mp3",
            Some("audio/mpeg"),
            false,
            false
        ));
        assert!(!is_voice_attachment(
            "doc.pdf",
            Some("application/pdf"),
            false,
            false
        ));
        assert!(!is_voice_attachment(
            "photo.png",
            Some("image/png"),
            false,
            false
        ));
    }

    #[test]
    fn test_is_text_attachment_by_content_type() {
        assert!(is_text_attachment("unknown_file", Some("text/plain")));
        assert!(is_text_attachment("file.bin", Some("text/x-rust")));
        assert!(is_text_attachment("data", Some("application/json")));
        assert!(is_text_attachment(
            "data",
            Some("application/problem+json; charset=utf-8")
        ));
        assert!(is_text_attachment("build", Some("application/xml")));
        assert!(is_text_attachment("script", Some("application/javascript")));
        assert!(is_text_attachment("script", Some("application/x-sh")));
        assert!(is_text_attachment("config", Some("application/x-yaml")));
        assert!(is_text_attachment("config", Some("application/toml")));

        // Binary types must return false
        assert!(!is_text_attachment("photo.png", Some("image/png")));
        assert!(!is_text_attachment("doc.pdf", Some("application/pdf")));
        assert!(!is_text_attachment("archive.zip", Some("application/zip")));
        assert!(!is_text_attachment(
            "binary.bin",
            Some("application/octet-stream")
        ));
    }

    #[test]
    fn test_is_text_attachment_by_extension_and_name() {
        assert!(is_text_attachment("main.rs", None));
        assert!(is_text_attachment(
            "script.py",
            Some("application/octet-stream")
        ));
        assert!(is_text_attachment("app.ts", None));
        assert!(is_text_attachment("index.js", None));
        assert!(is_text_attachment("README.md", None));
        assert!(is_text_attachment("data.csv", None));
        assert!(is_text_attachment("Cargo.toml", None));
        assert!(is_text_attachment("config.yaml", None));
        assert!(is_text_attachment("config.yml", None));
        assert!(is_text_attachment("run.sh", None));
        assert!(is_text_attachment("Dockerfile", None));
        assert!(is_text_attachment("Makefile", None));
        assert!(is_text_attachment(".env", None));
        assert!(is_text_attachment(".gitignore", None));

        // Binary extensions must return false
        assert!(!is_text_attachment("photo.jpg", None));
        assert!(!is_text_attachment("document.pdf", None));
        assert!(!is_text_attachment("archive.tar.gz", None));
        assert!(!is_text_attachment("audio.mp3", None));
    }
}

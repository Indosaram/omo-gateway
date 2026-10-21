//! PCM WAV encoding and the text that is safe to speak aloud.

use crate::{OmonError, Result};

/// OpenAI `pcm` speech is 24 kHz, 16-bit, mono, little-endian.
pub const OPENAI_PCM_RATE: u32 = 24_000;
pub const OPENAI_PCM_CHANNELS: u16 = 1;
/// OpenAI accepts at most 4096 characters. Stay under that so a truncated reply still synthesizes.
pub const SPEECH_CHAR_LIMIT: usize = 4_000;

/// Writes a canonical PCM WAV for `samples` (interleaved, little-endian i16).
pub fn pcm_to_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Result<Vec<u8>> {
    if channels == 0 || sample_rate == 0 {
        return Err(OmonError::Config(
            "WAV sample rate and channel count must be non-zero".into(),
        ));
    }
    let data_bytes = samples.len().checked_mul(2).ok_or_else(|| {
        OmonError::Config("PCM sample buffer is too large for a WAV header".into())
    })?;
    let data_size = u32::try_from(data_bytes)
        .map_err(|_| OmonError::Config("PCM sample buffer is too large for a WAV header".into()))?;
    let mut wav = Vec::with_capacity(44 + data_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    wav.extend_from_slice(&(channels * 2).to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(wav)
}

/// Prefixes a transcript so the agent session can tell speech from typed text.
pub fn format_voice_transcript(speaker: Option<&str>, transcript: &str) -> String {
    let transcript = transcript.trim();
    if transcript.is_empty() {
        return String::new();
    }
    match speaker.map(str::trim).filter(|speaker| !speaker.is_empty()) {
        Some(speaker) => format!("[voice] {speaker}: {transcript}"),
        None => format!("[voice] {transcript}"),
    }
}

/// Turns an assistant reply into text that should be synthesized.
///
/// Reasoning blocks, silence sentinels, fenced code, and markdown chrome are removed.
/// The result is clipped to [`SPEECH_CHAR_LIMIT`].
pub fn spoken_text(raw: &str) -> String {
    let filtered = crate::filter_reasoning(raw);
    if filtered.is_empty() || crate::is_explicit_silence(&filtered) {
        return String::new();
    }
    let without_fences = strip_fenced_code(&filtered);
    let without_links = strip_markdown_links(&without_fences);
    let plain: String = without_links
        .chars()
        .filter(|ch| !matches!(ch, '*' | '_' | '`' | '#' | '>'))
        .collect();
    let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() || crate::is_explicit_silence(&collapsed) {
        return String::new();
    }
    truncate_chars(&collapsed, SPEECH_CHAR_LIMIT)
}

fn strip_fenced_code(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("```") {
        out.push_str(&rest[..start]);
        out.push(' ');
        let after = &rest[start + 3..];
        match after.find("```") {
            Some(end) => rest = &after[end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `[label](url)` becomes `label`. Other text is copied through.
fn strip_markdown_links(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('[') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(label_end) = after.find("](") else {
            out.push('[');
            rest = after;
            continue;
        };
        let label = &after[..label_end];
        let after_label = &after[label_end + 2..];
        let Some(url_end) = after_label.find(')') else {
            out.push('[');
            rest = after;
            continue;
        };
        out.push_str(label);
        rest = &after_label[url_end + 1..];
    }
    out.push_str(rest);
    out
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(limit.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_describes_the_pcm_body() {
        let wav = pcm_to_wav(16_000, 1, &[1, -2]).unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1);
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(i16::from_le_bytes([wav[44], wav[45]]), 1);
        assert_eq!(i16::from_le_bytes([wav[46], wav[47]]), -2);
    }

    #[test]
    fn voice_transcript_keeps_the_speaker_and_drops_blank_audio() {
        assert_eq!(
            format_voice_transcript(Some("<@7>"), " hello "),
            "[voice] <@7>: hello"
        );
        assert_eq!(format_voice_transcript(None, "hello"), "[voice] hello");
        assert!(format_voice_transcript(Some("x"), "  ").is_empty());
    }

    #[test]
    fn spoken_text_strips_markdown_code_and_clips() {
        let raw = "See [docs](https://example.com).\n```rust\npanic!()\n```\n**Done**";
        assert_eq!(spoken_text(raw), "See docs. Done");
        let long = "a ".repeat(SPEECH_CHAR_LIMIT);
        let spoken = spoken_text(&long);
        assert!(spoken.chars().count() <= SPEECH_CHAR_LIMIT);
        assert!(spoken.ends_with('…'));
        assert!(
            spoken_text("<think>hidden</think>").is_empty()
                || !spoken_text("<think>hidden</think>").contains("hidden")
        );
    }
}

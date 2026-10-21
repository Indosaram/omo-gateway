//! Voice provider settings, resolved the same way as the other LLM endpoints.

use std::time::Duration;

use super::wav::{OPENAI_PCM_CHANNELS, OPENAI_PCM_RATE};

/// Decoded Discord receive format. Songbird is configured to match these values.
pub const RECEIVE_SAMPLE_RATE: u32 = 16_000;
pub const RECEIVE_CHANNELS: u16 = 1;

const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_STT_MODEL: &str = "whisper-1";
const DEFAULT_TTS_MODEL: &str = "tts-1";
const DEFAULT_TTS_VOICE: &str = "alloy";

/// One OpenAI-compatible speech endpoint (STT or TTS).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechEndpoint {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

/// Gateway voice configuration.
///
/// Keys and base URLs follow the OpenAI-compatible provider already used for
/// voice-note transcription: `OPENAI_API_KEY` / `OPENAI_API_BASE`, with
/// `OMON_STT_*` and `OMON_TTS_*` overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceSettings {
    enabled: bool,
    stt: Option<SpeechEndpoint>,
    tts: Option<SpeechEndpoint>,
    tts_voice: String,
    slack_voice_reply: bool,
    silence: Duration,
    min_speech: Duration,
    max_speech: Duration,
}

impl VoiceSettings {
    pub fn from_env() -> Self {
        Self::from_lookup(&|name| std::env::var(name).ok())
    }

    pub fn from_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Self {
        let speech_key = first_nonempty(
            lookup,
            &["OMON_STT_API_KEY", "OMON_TTS_API_KEY", "OPENAI_API_KEY"],
        );
        let speech_base = first_nonempty(
            lookup,
            &["OMON_STT_BASE_URL", "OMON_TTS_BASE_URL", "OPENAI_API_BASE"],
        )
        .unwrap_or_else(|| DEFAULT_OPENAI_BASE.to_owned());
        let stt_key = first_nonempty(lookup, &["OMON_STT_API_KEY"]).or(speech_key.clone());
        let tts_key = first_nonempty(lookup, &["OMON_TTS_API_KEY"]).or(speech_key);
        let stt_base =
            first_nonempty(lookup, &["OMON_STT_BASE_URL"]).unwrap_or_else(|| speech_base.clone());
        let tts_base = first_nonempty(lookup, &["OMON_TTS_BASE_URL"]).unwrap_or(speech_base);
        let stt_model = first_nonempty(lookup, &["OMON_STT_MODEL"])
            .unwrap_or_else(|| DEFAULT_STT_MODEL.to_owned());
        let tts_model = first_nonempty(lookup, &["OMON_TTS_MODEL"])
            .unwrap_or_else(|| DEFAULT_TTS_MODEL.to_owned());
        let tts_voice = first_nonempty(lookup, &["OMON_TTS_VOICE"])
            .unwrap_or_else(|| DEFAULT_TTS_VOICE.to_owned());
        let explicit = first_nonempty(lookup, &["OMON_VOICE_ENABLED"]);
        let key_present = stt_key.is_some() && tts_key.is_some();
        let enabled = match explicit.as_deref().map(|value| value.to_ascii_lowercase()) {
            Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
            Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
            _ => key_present,
        };
        Self {
            enabled,
            stt: stt_key.map(|api_key| SpeechEndpoint {
                api_key,
                base_url: stt_base,
                model: stt_model,
            }),
            tts: tts_key.map(|api_key| SpeechEndpoint {
                api_key,
                base_url: tts_base,
                model: tts_model,
            }),
            tts_voice,
            slack_voice_reply: flag(lookup, "SLACK_VOICE_REPLY", false),
            silence: duration_ms(lookup, "OMON_VOICE_SILENCE_MS", 700),
            min_speech: duration_ms(lookup, "OMON_VOICE_MIN_SPEECH_MS", 400),
            max_speech: duration_ms(lookup, "OMON_VOICE_MAX_SPEECH_MS", 30_000),
        }
    }

    pub fn disabled() -> Self {
        Self::from_lookup(&|_| None)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Songbird receive and playback can run.
    pub fn ready(&self) -> bool {
        self.enabled && self.stt.is_some() && self.tts.is_some()
    }

    pub fn stt(&self) -> Option<&SpeechEndpoint> {
        self.stt.as_ref()
    }

    pub fn tts(&self) -> Option<&SpeechEndpoint> {
        self.tts.as_ref()
    }

    pub fn tts_voice(&self) -> &str {
        &self.tts_voice
    }

    pub fn stt_model(&self) -> &str {
        self.stt
            .as_ref()
            .map(|endpoint| endpoint.model.as_str())
            .unwrap_or(DEFAULT_STT_MODEL)
    }

    pub fn slack_voice_reply(&self) -> bool {
        self.slack_voice_reply && self.tts.is_some()
    }

    pub fn slack_voice_reply_requested(&self) -> bool {
        self.slack_voice_reply
    }

    pub fn silence(&self) -> Duration {
        self.silence
    }

    pub fn min_speech(&self) -> Duration {
        self.min_speech
    }

    pub fn max_speech(&self) -> Duration {
        self.max_speech
    }

    pub fn tts_sample_rate(&self) -> u32 {
        OPENAI_PCM_RATE
    }

    pub fn tts_channels(&self) -> u16 {
        OPENAI_PCM_CHANNELS
    }

    /// Why `/voice join` cannot start, when [`Self::ready`] is false.
    pub fn unavailable_reason(&self) -> Option<&'static str> {
        if self.ready() {
            return None;
        }
        if !self.enabled {
            return Some("Discord voice is disabled (OMON_VOICE_ENABLED=false).");
        }
        if self.stt.is_none() || self.tts.is_none() {
            return Some(
                "Discord voice needs an API key. Set OPENAI_API_KEY, or OMON_STT_API_KEY and OMON_TTS_API_KEY.",
            );
        }
        None
    }
}

fn first_nonempty(lookup: &dyn Fn(&str) -> Option<String>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        lookup(name)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn flag(lookup: &dyn Fn(&str) -> Option<String>, name: &str, default: bool) -> bool {
    match first_nonempty(lookup, &[name])
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
    {
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

fn duration_ms(lookup: &dyn Fn(&str) -> Option<String>, name: &str, default_ms: u64) -> Duration {
    let millis = first_nonempty(lookup, &[name])
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(default_ms);
    Duration::from_millis(millis)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let env: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |name: &str| env.get(name).cloned()
    }

    #[test]
    fn openai_key_enables_both_endpoints_with_documented_defaults() {
        let settings = VoiceSettings::from_lookup(&lookup(&[
            ("OPENAI_API_KEY", "sk-test"),
            ("OPENAI_API_BASE", "https://example.test/v1/"),
        ]));
        assert!(settings.ready());
        let stt = settings.stt().unwrap();
        assert_eq!(stt.api_key, "sk-test");
        assert_eq!(stt.base_url, "https://example.test/v1/");
        assert_eq!(stt.model, "whisper-1");
        let tts = settings.tts().unwrap();
        assert_eq!(tts.model, "tts-1");
        assert_eq!(settings.tts_voice(), "alloy");
        assert!(!settings.slack_voice_reply());
    }

    #[test]
    fn overrides_split_stt_and_tts_and_slack_reply_requires_a_key() {
        let settings = VoiceSettings::from_lookup(&lookup(&[
            ("OMON_STT_API_KEY", "stt-key"),
            ("OMON_STT_BASE_URL", "https://stt.example/v1"),
            ("OMON_STT_MODEL", "whisper-large"),
            ("OMON_TTS_API_KEY", "tts-key"),
            ("OMON_TTS_MODEL", "tts-1-hd"),
            ("OMON_TTS_VOICE", "nova"),
            ("SLACK_VOICE_REPLY", "true"),
            ("OMON_VOICE_ENABLED", "false"),
        ]));
        assert!(!settings.enabled());
        assert!(!settings.ready());
        assert_eq!(
            settings.unavailable_reason(),
            Some("Discord voice is disabled (OMON_VOICE_ENABLED=false).")
        );
        assert_eq!(settings.stt().unwrap().model, "whisper-large");
        assert_eq!(settings.tts().unwrap().api_key, "tts-key");
        assert_eq!(settings.tts_voice(), "nova");
        assert!(settings.slack_voice_reply());
    }

    #[test]
    fn missing_key_stays_off_until_voice_is_explicitly_requested() {
        let off = VoiceSettings::from_lookup(&lookup(&[]));
        assert!(!off.ready());
        assert_eq!(off.stt_model(), "whisper-1");
        let requested = VoiceSettings::from_lookup(&lookup(&[("OMON_VOICE_ENABLED", "true")]));
        assert!(requested.enabled());
        assert!(!requested.ready());
        assert!(requested.unavailable_reason().unwrap().contains("API key"));
    }
}

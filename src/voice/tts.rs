//! OpenAI-compatible text-to-speech (`POST /audio/speech`).

use async_trait::async_trait;

use super::pipeline::{AudioDirection, AudioFrame, AudioPayload, TextToSpeech};
use super::wav::{spoken_text, OPENAI_PCM_CHANNELS, OPENAI_PCM_RATE};
use crate::{OmonError, Result};

/// Synthesizes speech from an OpenAI-compatible `/audio/speech` endpoint.
///
/// The response format is raw PCM (24 kHz, mono, little-endian i16), which
/// Songbird can play and which a WAV header can wrap for Slack uploads.
#[derive(Clone)]
pub struct OpenAiTextToSpeech {
    api_key: String,
    base_url: String,
    model: String,
    voice: String,
    client: reqwest::Client,
}

impl OpenAiTextToSpeech {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: "tts-1".to_string(),
            voice: "alloy".to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(30))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, "failed to build TTS HTTP client; using defaults");
                    reqwest::Client::new()
                }),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.voice = voice.into();
        self
    }

    async fn request_pcm(&self, text: &str) -> Result<Vec<u8>> {
        #[derive(serde::Serialize)]
        struct Request<'a> {
            model: &'a str,
            input: &'a str,
            voice: &'a str,
            response_format: &'static str,
        }

        let url = format!("{}/audio/speech", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&Request {
                model: &self.model,
                input: text,
                voice: &self.voice,
                response_format: "pcm",
            })
            .send()
            .await
            .map_err(|error| {
                OmonError::Multiplexer(format!("OpenAI TTS network error: {error}"))
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OmonError::Multiplexer(format!(
                "OpenAI TTS error {status}: {body}"
            )));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| OmonError::Multiplexer(format!("OpenAI TTS body error: {error}")))
    }
}

#[async_trait]
impl TextToSpeech for OpenAiTextToSpeech {
    async fn synthesize(&self, channel_id: u64, text: &str) -> Result<Vec<AudioFrame>> {
        let text = spoken_text(text);
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = self.request_pcm(&text).await?;
        let mut samples = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }
        if samples.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![AudioFrame {
            channel_id,
            source_id: None,
            sequence: 0,
            sample_rate: OPENAI_PCM_RATE,
            channels: OPENAI_PCM_CHANNELS,
            direction: AudioDirection::Outgoing,
            payload: AudioPayload::Pcm(samples),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn speech_request_uses_the_configured_pcm_endpoint() {
        let body = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<u8>::new()));
        let auth = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));
        let recorded_body = body.clone();
        let recorded_auth = auth.clone();
        let app = axum::Router::new().route(
            "/audio/speech",
            axum::routing::post(
                |headers: axum::http::HeaderMap, bytes: axum::body::Bytes| async move {
                    *recorded_auth.lock().await = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_owned);
                    *recorded_body.lock().await = bytes.to_vec();
                    (
                        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
                        axum::body::Bytes::from(vec![0x01, 0x00, 0xff, 0xff]),
                    )
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let tts = OpenAiTextToSpeech::new("tts-secret", format!("http://{address}"))
            .with_model("tts-1-hd")
            .with_voice("nova");
        let frames = tts.synthesize(9, "Hello **there**").await.unwrap();

        let request: serde_json::Value =
            serde_json::from_slice(&body.lock().await).expect("tts body is json");
        assert_eq!(request["model"], "tts-1-hd");
        assert_eq!(request["voice"], "nova");
        assert_eq!(request["response_format"], "pcm");
        assert_eq!(request["input"], "Hello there");
        assert_eq!(auth.lock().await.as_deref(), Some("Bearer tts-secret"));
        let AudioPayload::Pcm(samples) = &frames[0].payload else {
            panic!("tts payload should be pcm");
        };
        assert_eq!(samples, &vec![1, -1]);
        assert_eq!(frames[0].sample_rate, OPENAI_PCM_RATE);
        assert_eq!(frames[0].channels, OPENAI_PCM_CHANNELS);
        assert_eq!(frames[0].direction, AudioDirection::Outgoing);
        assert_eq!(frames[0].channel_id, 9);
    }

    #[tokio::test]
    async fn silence_and_blank_replies_do_not_call_the_endpoint() {
        let tts = OpenAiTextToSpeech::new("tts-secret", "http://127.0.0.1:1");
        let frames = tts.synthesize(1, "  ").await.unwrap();
        assert!(frames.is_empty());
    }
}

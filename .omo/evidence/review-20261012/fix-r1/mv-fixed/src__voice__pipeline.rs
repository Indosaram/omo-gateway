use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};

use crate::{OmonError, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioDirection {
    Incoming,
    Outgoing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "codec", content = "samples", rename_all = "snake_case")]
pub enum AudioPayload {
    Opus(Vec<u8>),
    Pcm(Vec<i16>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AudioFrame {
    pub channel_id: u64,
    pub source_id: Option<u32>,
    pub sequence: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub direction: AudioDirection,
    pub payload: AudioPayload,
}

impl AudioFrame {
    pub fn pcm(channel_id: u64, source_id: Option<u32>, sequence: u64, samples: Vec<i16>) -> Self {
        Self {
            channel_id,
            source_id,
            sequence,
            sample_rate: 48_000,
            channels: 2,
            direction: AudioDirection::Incoming,
            payload: AudioPayload::Pcm(samples),
        }
    }

    pub fn serialized(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|error| OmonError::Config(error.to_string()))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|error| OmonError::Config(error.to_string()))
    }

    pub fn byte_len(&self) -> usize {
        match &self.payload {
            AudioPayload::Opus(bytes) => bytes.len(),
            AudioPayload::Pcm(samples) => samples.len() * std::mem::size_of::<i16>(),
        }
    }
}

#[derive(Debug)]
pub struct AudioFrameBuffer {
    frames: VecDeque<AudioFrame>,
    capacity: usize,
}

impl AudioFrameBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, frame: AudioFrame) -> Option<AudioFrame> {
        if self.capacity == 0 {
            return Some(frame);
        }
        let evicted = (self.frames.len() == self.capacity)
            .then(|| self.frames.pop_front())
            .flatten();
        self.frames.push_back(frame);
        evicted
    }

    pub fn pop(&mut self) -> Option<AudioFrame> {
        self.frames.pop_front()
    }

    pub fn drain(&mut self) -> Vec<AudioFrame> {
        self.frames.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn serialized(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.frames).map_err(|error| OmonError::Config(error.to_string()))
    }

    pub fn deserialize(bytes: &[u8], capacity: usize) -> Result<Self> {
        let frames: VecDeque<AudioFrame> =
            serde_json::from_slice(bytes).map_err(|error| OmonError::Config(error.to_string()))?;
        if frames.len() > capacity {
            return Err(OmonError::Config(format!(
                "audio buffer has {} frames but capacity is {capacity}",
                frames.len()
            )));
        }
        Ok(Self { frames, capacity })
    }
}

#[async_trait]
pub trait SpeechToText: Send + Sync + 'static {
    async fn transcribe(&self, frames: &[AudioFrame]) -> Result<String>;
}

#[derive(Clone)]
pub struct OpenAiSpeechToText {
    api_key: String,
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiSpeechToText {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: "whisper-1".to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(30))
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, "failed to build STT HTTP client; using defaults");
                    reqwest::Client::new()
                }),
        }
    }
}

#[async_trait]
impl SpeechToText for OpenAiSpeechToText {
    async fn transcribe(&self, frames: &[AudioFrame]) -> Result<String> {
        let Some(first) = frames.first() else {
            return Ok(String::new());
        };
        let (bytes, filename) = match &first.payload {
            AudioPayload::Opus(opus_bytes) => (opus_bytes.clone(), "audio.ogg"),
            AudioPayload::Pcm(pcm_samples) => {
                let mut wav = Vec::new();
                let sample_rate = first.sample_rate;
                let channels = first.channels;
                let data_size = (pcm_samples.len() * 2) as u32;
                wav.extend_from_slice(b"RIFF");
                wav.extend_from_slice(&(36 + data_size).to_le_bytes());
                wav.extend_from_slice(b"WAVE");
                wav.extend_from_slice(b"fmt ");
                wav.extend_from_slice(&16u32.to_le_bytes());
                wav.extend_from_slice(&1u16.to_le_bytes());
                wav.extend_from_slice(&channels.to_le_bytes());
                wav.extend_from_slice(&sample_rate.to_le_bytes());
                wav.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
                wav.extend_from_slice(&(channels * 2).to_le_bytes());
                wav.extend_from_slice(&16u16.to_le_bytes());
                wav.extend_from_slice(b"data");
                wav.extend_from_slice(&data_size.to_le_bytes());
                for sample in pcm_samples {
                    wav.extend_from_slice(&sample.to_le_bytes());
                }
                (wav, "audio.wav")
            }
        };

        let part = reqwest::multipart::Part::bytes(bytes).file_name(filename);
        let form = reqwest::multipart::Form::new()
            .text("model", self.model.clone())
            .part("file", part);

        let url = format!(
            "{}/audio/transcriptions",
            self.base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| OmonError::Multiplexer(format!("OpenAI STT network error: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OmonError::Multiplexer(format!(
                "OpenAI STT error {status}: {text}"
            )));
        }

        #[derive(serde::Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let body: TranscriptionResponse = response
            .json()
            .await
            .map_err(|e| OmonError::Multiplexer(format!("OpenAI STT JSON error: {e}")))?;

        Ok(body.text)
    }
}

#[async_trait]
pub trait VoiceLanguageModel: Send + Sync + 'static {
    async fn respond(&self, channel_id: u64, transcript: &str) -> Result<String>;
}

#[async_trait]
pub trait TextToSpeech: Send + Sync + 'static {
    async fn synthesize(&self, channel_id: u64, text: &str) -> Result<Vec<AudioFrame>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechPipelineOutput {
    pub transcript: String,
    pub response: String,
    pub audio: Vec<AudioFrame>,
}

#[derive(Clone)]
pub struct SpeechPipeline {
    stt: Arc<dyn SpeechToText>,
    llm: Arc<dyn VoiceLanguageModel>,
    tts: Arc<dyn TextToSpeech>,
}

impl SpeechPipeline {
    pub fn new(
        stt: Arc<dyn SpeechToText>,
        llm: Arc<dyn VoiceLanguageModel>,
        tts: Arc<dyn TextToSpeech>,
    ) -> Self {
        Self { stt, llm, tts }
    }

    pub async fn process(
        &self,
        channel_id: u64,
        frames: &[AudioFrame],
    ) -> Result<SpeechPipelineOutput> {
        let transcript = self.stt.transcribe(frames).await?;
        let response = self.llm.respond(channel_id, &transcript).await?;
        let mut audio = self.tts.synthesize(channel_id, &response).await?;
        for frame in &mut audio {
            frame.direction = AudioDirection::Outgoing;
        }
        Ok(SpeechPipelineOutput {
            transcript,
            response,
            audio,
        })
    }
}

#[derive(Clone)]
pub struct VoiceAudioPipeline {
    sender: mpsc::Sender<AudioFrame>,
    receiver: Arc<Mutex<mpsc::Receiver<AudioFrame>>>,
}

impl VoiceAudioPipeline {
    pub fn channel(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<AudioFrame> {
        self.sender.clone()
    }

    pub async fn submit(&self, frame: AudioFrame) -> Result<()> {
        self.sender
            .send(frame)
            .await
            .map_err(|_| OmonError::Config("voice audio pipeline is closed".into()))
    }

    pub async fn receive(&self) -> Option<AudioFrame> {
        self.receiver.lock().await.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn openai_stt_request_times_out_when_server_never_responds() {
        // The kernel backlog completes the TCP handshake, but nothing ever replies.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let stt = OpenAiSpeechToText::new("test-key", base_url);
        let frame = AudioFrame::pcm(0, None, 0, vec![0; 4]);

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(600),
            stt.transcribe(&[frame]),
        )
        .await;

        let result = outcome.expect("STT request must time out on its own instead of hanging");
        assert!(result.is_err());
        drop(listener);
    }
}

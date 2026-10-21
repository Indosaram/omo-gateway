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

/// Joins every PCM frame into one sample stream; frames must share one format
/// so the WAV header written from the first frame stays valid for all of them.
fn concat_pcm_frames(
    first: &AudioFrame,
    first_samples: &[i16],
    frames: &[AudioFrame],
) -> Result<Vec<i16>> {
    let mut samples = Vec::with_capacity(first_samples.len());
    samples.extend_from_slice(first_samples);
    for frame in &frames[1..] {
        match &frame.payload {
            AudioPayload::Pcm(more)
                if frame.sample_rate == first.sample_rate && frame.channels == first.channels =>
            {
                samples.extend_from_slice(more);
            }
            _ => {
                return Err(OmonError::Multiplexer(
                    "STT frames must all be PCM with one sample rate and channel count".into(),
                ));
            }
        }
    }
    Ok(samples)
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

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl SpeechToText for OpenAiSpeechToText {
    async fn transcribe(&self, frames: &[AudioFrame]) -> Result<String> {
        let Some(first) = frames.first() else {
            return Ok(String::new());
        };
        let (bytes, filename) = match &first.payload {
            AudioPayload::Opus(_) if frames.len() > 1 => {
                return Err(OmonError::Multiplexer(
                    "STT cannot transcribe multiple Opus frames as one stream".into(),
                ));
            }
            AudioPayload::Opus(opus_bytes) => (opus_bytes.clone(), "audio.ogg"),
            AudioPayload::Pcm(first_samples) => {
                let pcm_samples = concat_pcm_frames(first, first_samples, frames)?;
                let wav = super::wav::pcm_to_wav(first.sample_rate, first.channels, &pcm_samples)?;
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

    pub fn tts(&self) -> Arc<dyn TextToSpeech> {
        self.tts.clone()
    }

    pub async fn process(
        &self,
        channel_id: u64,
        frames: &[AudioFrame],
    ) -> Result<SpeechPipelineOutput> {
        self.process_utterance(channel_id, None, frames).await
    }

    /// Transcribes `frames`, asks the bound language model, and synthesizes the reply.
    ///
    /// `speaker` is a display label such as `<@user>`. An empty transcript skips the model.
    pub async fn process_utterance(
        &self,
        channel_id: u64,
        speaker: Option<&str>,
        frames: &[AudioFrame],
    ) -> Result<SpeechPipelineOutput> {
        let raw = self.stt.transcribe(frames).await?;
        let transcript = super::wav::format_voice_transcript(speaker, &raw);
        if transcript.is_empty() {
            return Ok(SpeechPipelineOutput {
                transcript,
                response: String::new(),
                audio: Vec::new(),
            });
        }
        let response = self.llm.respond(channel_id, &transcript).await?;
        if super::wav::spoken_text(&response).is_empty() {
            return Ok(SpeechPipelineOutput {
                transcript,
                response,
                audio: Vec::new(),
            });
        }
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

    #[test]
    fn pcm_frames_are_all_concatenated_for_transcription() {
        let frames = [
            AudioFrame::pcm(0, None, 0, vec![1, 2]),
            AudioFrame::pcm(0, None, 1, vec![3, 4]),
        ];
        let AudioPayload::Pcm(first) = &frames[0].payload else {
            unreachable!()
        };
        let joined = concat_pcm_frames(&frames[0], first, &frames).unwrap();
        assert_eq!(joined, vec![1, 2, 3, 4]);
    }

    #[test]
    fn mismatched_pcm_frames_are_rejected_instead_of_dropped() {
        let mut other = AudioFrame::pcm(0, None, 1, vec![3]);
        other.sample_rate = 16_000;
        let frames = [AudioFrame::pcm(0, None, 0, vec![1]), other];
        let AudioPayload::Pcm(first) = &frames[0].payload else {
            unreachable!()
        };
        assert!(concat_pcm_frames(&frames[0], first, &frames).is_err());
    }

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

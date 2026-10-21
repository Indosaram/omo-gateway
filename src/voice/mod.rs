mod config;
mod pipeline;
mod playback;
mod respond;
mod session;
mod slack_reply;
mod tts;
mod utterance;
mod wav;

pub use config::VoiceSettings;
pub use pipeline::{
    AudioDirection, AudioFrame, AudioFrameBuffer, AudioPayload, OpenAiSpeechToText, SpeechPipeline,
    SpeechPipelineOutput, SpeechToText, TextToSpeech, VoiceAudioPipeline, VoiceLanguageModel,
};
pub use playback::songbird_config;
pub use session::VoiceRuntime;
pub use slack_reply::SlackVoiceReplies;
pub use tts::OpenAiTextToSpeech;
pub use wav::{format_voice_transcript, pcm_to_wav, spoken_text};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use songbird::events::{Event, EventContext, EventHandler};
use tokio::sync::mpsc;

/// One decoded or signalling update from a Songbird receive handler.
#[derive(Clone, Debug)]
pub enum VoicePacket {
    Pcm { ssrc: u32, samples: Vec<i16> },
    Silent { ssrc: u32 },
    Speaker { ssrc: u32, user_id: u64 },
    Left { user_id: u64 },
    Opus { ssrc: u32, payload: Vec<u8> },
}

/// Songbird global event listener which forwards received Opus or decoded PCM
/// frames into the provider-independent voice pipeline.
#[derive(Clone)]
pub struct SongbirdAudioEventListener {
    sender: mpsc::Sender<VoicePacket>,
    generation: Arc<AtomicU64>,
    epoch: u64,
}

impl SongbirdAudioEventListener {
    pub fn new(channel_id: u64, sender: mpsc::Sender<VoicePacket>) -> Self {
        let _ = channel_id;
        Self::with_format(
            channel_id,
            sender,
            Arc::new(AtomicU64::new(1)),
            1,
            48_000,
            2,
        )
    }

    pub fn with_format(
        channel_id: u64,
        sender: mpsc::Sender<VoicePacket>,
        generation: Arc<AtomicU64>,
        epoch: u64,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        let _ = (channel_id, sample_rate, channels);
        Self {
            sender,
            generation,
            epoch,
        }
    }

    fn current(&self) -> bool {
        self.generation.load(Ordering::Acquire) == self.epoch
    }
}

#[async_trait]
impl EventHandler for SongbirdAudioEventListener {
    async fn act(&self, context: &EventContext<'_>) -> Option<Event> {
        if !self.current() {
            return Some(Event::Cancel);
        }
        match context {
            EventContext::VoiceTick(tick) => {
                for (ssrc, voice) in &tick.speaking {
                    if let Some(samples) = &voice.decoded_voice {
                        if let Some(cancel) = self.forward(VoicePacket::Pcm {
                            ssrc: *ssrc,
                            samples: samples.clone(),
                        }) {
                            return Some(cancel);
                        }
                    }
                }
                for ssrc in &tick.silent {
                    if let Some(cancel) = self.forward(VoicePacket::Silent { ssrc: *ssrc }) {
                        return Some(cancel);
                    }
                }
            }
            EventContext::SpeakingStateUpdate(update) => {
                if let Some(user_id) = update.user_id {
                    return self.forward(VoicePacket::Speaker {
                        ssrc: update.ssrc,
                        user_id: user_id.0,
                    });
                }
            }
            EventContext::ClientDisconnect(update) => {
                return self.forward(VoicePacket::Left {
                    user_id: update.user_id.0,
                });
            }
            EventContext::RtpPacket(packet) => {
                let end = packet.packet.len().saturating_sub(packet.payload_end_pad);
                if packet.payload_offset <= end {
                    return self.forward(VoicePacket::Opus {
                        ssrc: packet.rtp().get_ssrc(),
                        payload: packet.packet[packet.payload_offset..end].to_vec(),
                    });
                }
            }
            _ => {}
        }
        None
    }
}

impl SongbirdAudioEventListener {
    /// `try_send` so a slow consumer cannot stall Songbird's event task.
    /// A closed receiver unregisters the handler.
    fn forward(&self, packet: VoicePacket) -> Option<Event> {
        match self.sender.try_send(packet) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => None,
            Err(mpsc::error::TrySendError::Closed(_)) => Some(Event::Cancel),
        }
    }
}

//! Groups decoded PCM into utterances separated by silence.

use std::collections::HashMap;
use std::time::Duration;

use super::pipeline::{AudioDirection, AudioFrame, AudioPayload};
use super::VoicePacket;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utterance {
    pub ssrc: u32,
    pub user_id: Option<u64>,
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Utterance {
    pub fn frames(&self, channel_id: u64) -> Vec<AudioFrame> {
        vec![AudioFrame {
            channel_id,
            source_id: Some(self.ssrc),
            sequence: 0,
            sample_rate: self.sample_rate,
            channels: self.channels,
            direction: AudioDirection::Incoming,
            payload: AudioPayload::Pcm(self.samples.clone()),
        }]
    }

    pub fn speaker_label(&self) -> Option<String> {
        self.user_id.map(|user_id| format!("<@{user_id}>"))
    }
}

struct Speaker {
    samples: Vec<i16>,
    trailing_silence: usize,
    user_id: Option<u64>,
}

/// Buffers per-speaker PCM until the speaker has been quiet for long enough.
pub struct UtteranceAssembler {
    sample_rate: u32,
    channels: u16,
    samples_per_tick: usize,
    silence_samples: usize,
    min_samples: usize,
    max_samples: usize,
    speakers: HashMap<u32, Speaker>,
    users: HashMap<u32, u64>,
}

impl UtteranceAssembler {
    pub fn new(
        sample_rate: u32,
        channels: u16,
        silence: Duration,
        min_speech: Duration,
        max_speech: Duration,
    ) -> Self {
        let channels = channels.max(1);
        let sample_rate = sample_rate.max(1);
        let samples_per_tick =
            ((sample_rate as u64 * u64::from(channels) * 20) / 1000).max(1) as usize;
        Self {
            sample_rate,
            channels,
            samples_per_tick,
            silence_samples: samples_for(sample_rate, channels, silence).max(samples_per_tick),
            min_samples: samples_for(sample_rate, channels, min_speech),
            max_samples: samples_for(sample_rate, channels, max_speech).max(1),
            speakers: HashMap::new(),
            users: HashMap::new(),
        }
    }

    pub fn push(&mut self, packet: VoicePacket) -> Option<Utterance> {
        match packet {
            VoicePacket::Pcm { ssrc, samples } => self.push_samples(ssrc, samples),
            VoicePacket::Silent { ssrc } => self.push_silence(ssrc),
            VoicePacket::Speaker { ssrc, user_id } => {
                self.users.insert(ssrc, user_id);
                if let Some(speaker) = self.speakers.get_mut(&ssrc) {
                    speaker.user_id = Some(user_id);
                }
                None
            }
            VoicePacket::Left { user_id } => self.finish_user(user_id),
            VoicePacket::Opus { .. } => None,
        }
    }

    fn push_samples(&mut self, ssrc: u32, samples: Vec<i16>) -> Option<Utterance> {
        if samples.is_empty() {
            return None;
        }
        let user_id = self.users.get(&ssrc).copied();
        let speaker = self.speakers.entry(ssrc).or_insert_with(|| Speaker {
            samples: Vec::new(),
            trailing_silence: 0,
            user_id,
        });
        speaker.trailing_silence = 0;
        speaker.user_id = speaker.user_id.or(user_id);
        speaker.samples.extend(samples);
        if speaker.samples.len() >= self.max_samples {
            return self.take(ssrc);
        }
        None
    }

    fn push_silence(&mut self, ssrc: u32) -> Option<Utterance> {
        let speaker = self.speakers.get_mut(&ssrc)?;
        speaker.trailing_silence = speaker
            .trailing_silence
            .saturating_add(self.samples_per_tick);
        if speaker.trailing_silence < self.silence_samples {
            return None;
        }
        if speaker.samples.len() < self.min_samples {
            self.speakers.remove(&ssrc);
            return None;
        }
        self.take(ssrc)
    }

    fn finish_user(&mut self, user_id: u64) -> Option<Utterance> {
        let ssrc = self
            .users
            .iter()
            .find(|(_, id)| **id == user_id)
            .map(|(ssrc, _)| *ssrc);
        let ssrc = ssrc?;
        self.users.remove(&ssrc);
        let long_enough = self
            .speakers
            .get(&ssrc)
            .is_some_and(|speaker| speaker.samples.len() >= self.min_samples);
        if long_enough {
            self.take(ssrc)
        } else {
            self.speakers.remove(&ssrc);
            None
        }
    }

    fn take(&mut self, ssrc: u32) -> Option<Utterance> {
        let speaker = self.speakers.remove(&ssrc)?;
        if speaker.samples.is_empty() {
            return None;
        }
        Some(Utterance {
            ssrc,
            user_id: speaker.user_id.or_else(|| self.users.get(&ssrc).copied()),
            samples: speaker.samples,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }
}

fn samples_for(sample_rate: u32, channels: u16, duration: Duration) -> usize {
    let millis = duration.as_millis().min(u128::from(u64::MAX)) as u64;
    ((u64::from(sample_rate) * u64::from(channels) * millis) / 1000) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assembler() -> UtteranceAssembler {
        UtteranceAssembler::new(
            16_000,
            1,
            Duration::from_millis(40),
            Duration::from_millis(40),
            Duration::from_millis(100),
        )
    }

    fn pcm(ssrc: u32, n: usize) -> VoicePacket {
        VoicePacket::Pcm {
            ssrc,
            samples: vec![1; n],
        }
    }

    #[test]
    fn silence_after_speech_emits_one_utterance_and_drops_noise() {
        let mut assembler = assembler();
        // 40 ms of 16 kHz mono is the minimum. Two 20 ms silent ticks close it.
        assert!(assembler.push(pcm(1, 640)).is_none());
        assert!(assembler.push(VoicePacket::Silent { ssrc: 1 }).is_none());
        let utterance = assembler
            .push(VoicePacket::Silent { ssrc: 1 })
            .expect("silence threshold ends the utterance");
        assert_eq!(utterance.samples.len(), 640);
        assert_eq!(utterance.ssrc, 1);

        assert!(assembler.push(pcm(2, 10)).is_none());
        assert!(assembler.push(VoicePacket::Silent { ssrc: 2 }).is_none());
        assert!(assembler.push(VoicePacket::Silent { ssrc: 2 }).is_none());
    }

    #[test]
    fn speakers_stay_separate_and_map_to_user_ids() {
        let mut assembler = assembler();
        assert!(assembler
            .push(VoicePacket::Speaker {
                ssrc: 7,
                user_id: 42,
            })
            .is_none());
        assert!(assembler.push(pcm(7, 640)).is_none());
        assert!(assembler.push(pcm(8, 640)).is_none());
        assert!(assembler.push(VoicePacket::Silent { ssrc: 7 }).is_none());
        let utterance = assembler
            .push(VoicePacket::Silent { ssrc: 7 })
            .expect("speaker 7 ended");
        assert_eq!(utterance.user_id, Some(42));
        assert!(utterance.speaker_label().unwrap().contains("42"));
        assert!(assembler.speakers.contains_key(&8));
    }

    #[test]
    fn long_speech_flushes_without_waiting_for_silence() {
        let mut assembler = assembler();
        let utterance = assembler
            .push(pcm(1, 1600))
            .expect("max speech length flushes");
        assert_eq!(utterance.samples.len(), 1600);
        let frame = &utterance.frames(99)[0];
        assert_eq!(frame.channel_id, 99);
        assert_eq!(frame.sample_rate, 16_000);
    }
}

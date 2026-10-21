//! Play synthesized PCM back into a Songbird call.

use std::io::Cursor;
use std::sync::Arc;

use songbird::input::RawAdapter;
use tokio::sync::Mutex;

use super::config::{RECEIVE_CHANNELS, RECEIVE_SAMPLE_RATE};
use super::pipeline::{AudioFrame, AudioPayload};
use crate::{OmonError, Result};

/// Songbird receive configuration used when the Discord client is built.
pub fn songbird_config() -> songbird::Config {
    songbird::Config::default()
        .decode_mode(songbird::driver::DecodeMode::Decode)
        .decode_channels(songbird::driver::Channels::Mono)
        .decode_sample_rate(songbird::driver::SampleRate::Hz16000)
}

/// Interleaved little-endian `f32` samples, plus the rate and channel count Songbird needs.
pub fn frames_to_f32(frames: &[AudioFrame]) -> Result<(u32, u32, Vec<u8>)> {
    let Some(first) = frames.iter().find(|frame| frame.byte_len() > 0) else {
        return Err(OmonError::Config("no PCM samples to play".into()));
    };
    let mut bytes = Vec::new();
    for frame in frames {
        if frame.byte_len() == 0 {
            continue;
        }
        if frame.sample_rate != first.sample_rate || frame.channels != first.channels {
            return Err(OmonError::Config(
                "playback frames must share one sample rate and channel count".into(),
            ));
        }
        match &frame.payload {
            AudioPayload::Pcm(samples) => {
                for sample in samples {
                    let unit = f32::from(*sample) / 32768.0;
                    bytes.extend_from_slice(&unit.to_le_bytes());
                }
            }
            AudioPayload::Opus(_) => {
                return Err(OmonError::Config(
                    "playback expects decoded PCM, not Opus packets".into(),
                ));
            }
        }
    }
    if bytes.is_empty() {
        return Err(OmonError::Config("no PCM samples to play".into()));
    }
    Ok((first.sample_rate, u32::from(first.channels), bytes))
}

pub fn frames_to_input(frames: &[AudioFrame]) -> Result<songbird::input::Input> {
    let (sample_rate, channels, bytes) = frames_to_f32(frames)?;
    let adapter = RawAdapter::new(Cursor::new(bytes), sample_rate, channels);
    Ok(adapter.into())
}

pub async fn play_frames(call: &Arc<Mutex<songbird::Call>>, frames: &[AudioFrame]) -> Result<()> {
    let input = frames_to_input(frames)?;
    let mut handler = call.lock().await;
    handler.play_input(input);
    Ok(())
}

pub fn receive_format() -> (u32, u16) {
    (RECEIVE_SAMPLE_RATE, RECEIVE_CHANNELS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::AudioDirection;

    #[test]
    fn pcm_frames_become_interleaved_f32_and_reject_a_format_change() {
        let frame = AudioFrame {
            channel_id: 1,
            source_id: None,
            sequence: 0,
            sample_rate: 24_000,
            channels: 1,
            direction: AudioDirection::Outgoing,
            payload: AudioPayload::Pcm(vec![0, 16384, -16384]),
        };
        let (rate, channels, bytes) = frames_to_f32(std::slice::from_ref(&frame)).unwrap();
        assert_eq!((rate, channels), (24_000, 1));
        assert_eq!(bytes.len(), 12);
        assert_eq!(f32::from_le_bytes(bytes[4..8].try_into().unwrap()), 0.5);
        let mut other = frame.clone();
        other.sample_rate = 48_000;
        assert!(frames_to_f32(&[frame, other]).is_err());
        let input = frames_to_input(&[AudioFrame {
            channel_id: 1,
            source_id: None,
            sequence: 0,
            sample_rate: 24_000,
            channels: 1,
            direction: AudioDirection::Outgoing,
            payload: AudioPayload::Pcm(vec![0]),
        }])
        .unwrap();
        assert!(matches!(input, songbird::input::Input::Live(_, _)));
    }
}

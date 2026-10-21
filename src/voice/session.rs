//! Join a Discord voice channel, transcribe speech, and play the agent reply.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex as ParkingMutex;
use songbird::events::{CoreEvent, Event};
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use super::config::{VoiceSettings, RECEIVE_CHANNELS, RECEIVE_SAMPLE_RATE};
use super::pipeline::SpeechPipeline;
use super::playback::{play_frames, receive_format};
use super::respond::MultiplexerVoiceModel;
use super::tts::OpenAiTextToSpeech;
use super::utterance::UtteranceAssembler;
use super::{SongbirdAudioEventListener, VoicePacket};
use crate::{SessionKey, SessionMultiplexer};

struct GuildCall {
    cancel: CancellationToken,
    generation: Arc<AtomicU64>,
}

/// Live Discord voice for one gateway process.
#[derive(Clone)]
pub struct VoiceRuntime {
    settings: VoiceSettings,
    pipeline: Option<SpeechPipeline>,
    /// Present whenever TTS is configured, including when Discord voice itself is off.
    tts: Option<Arc<dyn super::pipeline::TextToSpeech>>,
    sessions: Arc<ParkingMutex<HashMap<u64, SessionKey>>>,
    guilds: Arc<ParkingMutex<HashMap<u64, GuildCall>>>,
    slack_replies: Option<Arc<super::SlackVoiceReplies>>,
}

impl VoiceRuntime {
    pub fn from_settings(
        settings: VoiceSettings,
        multiplexer: SessionMultiplexer,
        pool: sqlx::SqlitePool,
    ) -> Self {
        let sessions = Arc::new(ParkingMutex::new(HashMap::new()));
        let tts = settings.tts().map(|endpoint| {
            Arc::new(
                OpenAiTextToSpeech::new(&endpoint.api_key, &endpoint.base_url)
                    .with_model(&endpoint.model)
                    .with_voice(settings.tts_voice()),
            ) as Arc<dyn super::pipeline::TextToSpeech>
        });
        // Discord voice needs STT and TTS. Slack spoken replies only need TTS, so a
        // disabled Discord voice channel still synthesizes Slack audio-file replies.
        let pipeline = match (settings.ready(), settings.stt(), tts.clone()) {
            (true, Some(stt), Some(tts)) => {
                let stt = Arc::new(
                    crate::OpenAiSpeechToText::new(&stt.api_key, &stt.base_url)
                        .with_model(&stt.model),
                );
                let model = Arc::new(MultiplexerVoiceModel::new(
                    multiplexer,
                    pool,
                    sessions.clone(),
                ));
                Some(SpeechPipeline::new(stt, model, tts))
            }
            _ => None,
        };
        let slack_replies = (settings.slack_voice_reply() && tts.is_some())
            .then(|| Arc::new(super::SlackVoiceReplies::new()));
        if settings.slack_voice_reply_requested() && slack_replies.is_none() {
            tracing::warn!(
                "SLACK_VOICE_REPLY is set but TTS is not configured; Slack voice replies stay off"
            );
        }
        Self {
            settings,
            pipeline,
            tts,
            sessions,
            guilds: Arc::new(ParkingMutex::new(HashMap::new())),
            slack_replies,
        }
    }

    pub fn disabled() -> Self {
        Self {
            settings: VoiceSettings::disabled(),
            pipeline: None,
            tts: None,
            sessions: Arc::new(ParkingMutex::new(HashMap::new())),
            guilds: Arc::new(ParkingMutex::new(HashMap::new())),
            slack_replies: None,
        }
    }

    pub fn ready(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn settings(&self) -> &VoiceSettings {
        &self.settings
    }

    pub fn slack_replies(&self) -> Option<Arc<super::SlackVoiceReplies>> {
        self.slack_replies.clone()
    }

    pub fn tts(&self) -> Option<Arc<dyn super::pipeline::TextToSpeech>> {
        self.tts.clone()
    }

    pub async fn join(
        &self,
        ctx: &poise::serenity_prelude::Context,
        guild_id: u64,
        voice_channel_id: u64,
        session: SessionKey,
        bot_user_id: u64,
    ) -> std::result::Result<String, String> {
        if let Some(reason) = self.settings.unavailable_reason() {
            return Err(reason.to_owned());
        }
        let Some(pipeline) = self.pipeline.clone() else {
            return Err("Discord voice is not configured.".into());
        };
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "Songbird is not registered on this Discord client.".to_owned())?;
        let guild = snowflake(guild_id)?;
        let channel = snowflake(voice_channel_id)?;
        let (generation, epoch) = self.next_epoch(guild_id);
        // Drop the previous driver so its receive handlers do not stack on the next join.
        if manager.get(guild).is_some() {
            if let Err(error) = manager.remove(guild).await {
                tracing::warn!(%error, guild_id, "failed to drop the previous voice call");
            }
        }
        let call = manager
            .join(guild, channel)
            .await
            .map_err(|error| format!("Could not join the voice channel: {error}"))?;
        let (sender, receiver) = mpsc::channel(256);
        {
            let mut handler = call.lock().await;
            if let Err(error) = handler.deafen(false).await {
                tracing::warn!(%error, "failed to undeafen the voice connection");
            }
            if let Err(error) = handler.mute(false).await {
                tracing::warn!(%error, "failed to unmute the voice connection");
            }
            let (sample_rate, channels) = receive_format();
            let listener = SongbirdAudioEventListener::with_format(
                voice_channel_id,
                sender,
                generation.clone(),
                epoch,
                sample_rate,
                channels,
            );
            handler.add_global_event(Event::Core(CoreEvent::VoiceTick), listener.clone());
            handler.add_global_event(
                Event::Core(CoreEvent::SpeakingStateUpdate),
                listener.clone(),
            );
            handler.add_global_event(Event::Core(CoreEvent::ClientDisconnect), listener);
        }
        self.sessions.lock().insert(voice_channel_id, session);
        let cancel = CancellationToken::new();
        self.guilds.lock().insert(
            guild_id,
            GuildCall {
                cancel: cancel.clone(),
                generation: generation.clone(),
            },
        );
        tokio::spawn(listen(
            receiver,
            call,
            pipeline,
            voice_channel_id,
            bot_user_id,
            cancel,
            generation,
            epoch,
            self.settings.silence(),
            self.settings.min_speech(),
            self.settings.max_speech(),
        ));
        Ok(format!(
            "Joined <#{voice_channel_id}>. I will answer in this text channel and play the audio in voice."
        ))
    }

    pub async fn leave(
        &self,
        ctx: &poise::serenity_prelude::Context,
        guild_id: u64,
    ) -> std::result::Result<String, String> {
        if let Some(reason) = self.settings.unavailable_reason() {
            return Err(reason.to_owned());
        }
        let existing = self.guilds.lock().remove(&guild_id);
        let Some(existing) = existing else {
            return Ok("I am not in a voice channel in this server.".into());
        };
        existing.cancel.cancel();
        existing.generation.fetch_add(1, Ordering::AcqRel);
        let manager = songbird::get(ctx)
            .await
            .ok_or_else(|| "Songbird is not registered on this Discord client.".to_owned())?;
        let guild = snowflake(guild_id)?;
        if manager.get(guild).is_some() {
            manager
                .remove(guild)
                .await
                .map_err(|error| format!("Could not leave the voice channel: {error}"))?;
        }
        Ok("Left the voice channel.".into())
    }

    fn next_epoch(&self, guild_id: u64) -> (Arc<AtomicU64>, u64) {
        let mut guilds = self.guilds.lock();
        if let Some(existing) = guilds.remove(&guild_id) {
            existing.cancel.cancel();
            let epoch = existing.generation.fetch_add(1, Ordering::AcqRel) + 1;
            (existing.generation, epoch)
        } else {
            (Arc::new(AtomicU64::new(1)), 1)
        }
    }
}

fn snowflake(id: u64) -> std::result::Result<std::num::NonZeroU64, String> {
    std::num::NonZeroU64::new(id).ok_or_else(|| "Discord ids must be non-zero.".to_owned())
}

#[allow(clippy::too_many_arguments)]
async fn listen(
    mut receiver: mpsc::Receiver<VoicePacket>,
    call: Arc<Mutex<songbird::Call>>,
    pipeline: SpeechPipeline,
    voice_channel_id: u64,
    bot_user_id: u64,
    cancel: CancellationToken,
    generation: Arc<AtomicU64>,
    epoch: u64,
    silence: std::time::Duration,
    min_speech: std::time::Duration,
    max_speech: std::time::Duration,
) {
    let mut assembler = UtteranceAssembler::new(
        RECEIVE_SAMPLE_RATE,
        RECEIVE_CHANNELS,
        silence,
        min_speech,
        max_speech,
    );
    let turn = Arc::new(AtomicU64::new(0));
    let mut worker: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        let packet = tokio::select! {
            _ = cancel.cancelled() => break,
            packet = receiver.recv() => match packet {
                Some(packet) => packet,
                None => break,
            },
        };
        if generation.load(Ordering::Acquire) != epoch {
            break;
        }
        let speaking = matches!(packet, VoicePacket::Pcm { .. });
        if speaking {
            if let Some(handle) = worker.take() {
                turn.fetch_add(1, Ordering::AcqRel);
                handle.abort();
                call.lock().await.stop();
            }
        }
        let Some(utterance) = assembler.push(packet) else {
            continue;
        };
        if utterance.user_id == Some(bot_user_id) {
            continue;
        }
        if let Some(handle) = worker.take() {
            handle.abort();
            call.lock().await.stop();
        }
        let mine = turn.fetch_add(1, Ordering::AcqRel) + 1;
        let pipeline = pipeline.clone();
        let call = call.clone();
        let turn = turn.clone();
        let generation = generation.clone();
        worker = Some(tokio::spawn(async move {
            if generation.load(Ordering::Acquire) != epoch || turn.load(Ordering::Acquire) != mine {
                return;
            }
            let label = utterance.speaker_label();
            let frames = utterance.frames(voice_channel_id);
            let output = match pipeline
                .process_utterance(voice_channel_id, label.as_deref(), &frames)
                .await
            {
                Ok(output) => output,
                Err(error) => {
                    tracing::warn!(%error, voice_channel_id, "voice turn failed");
                    return;
                }
            };
            if output.audio.is_empty()
                || generation.load(Ordering::Acquire) != epoch
                || turn.load(Ordering::Acquire) != mine
            {
                return;
            }
            if let Err(error) = play_frames(&call, &output.audio).await {
                tracing::warn!(%error, voice_channel_id, "failed to play the voice reply");
            }
        }));
    }
    if let Some(handle) = worker.take() {
        handle.abort();
    }
    call.lock().await.stop();
}

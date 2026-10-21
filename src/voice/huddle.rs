//! Slack huddle voice through a signed-in Chrome profile.
//!
//! The browser sidecar joins an already-active huddle in the normal Slack web
//! UI, taps remote WebRTC audio the page is already playing, and selects a
//! virtual microphone. This module turns that audio into the same
//! speech-to-text, session, and speech-synthesis loop as Discord voice, then
//! plays the reply into the virtual microphone.
//!
//! The control flow follows the OpenClaw `slack-huddles` plugin (MIT, OpenClaw
//! Foundation). This is an independent implementation: OpenClaw's sources are
//! not copied. See `sidecar/slack-huddle/index.mjs`.

use std::collections::{HashMap, VecDeque};
use std::io::ErrorKind;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWrite;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_util::sync::CancellationToken;

use super::config::{VoiceSettings, RECEIVE_CHANNELS, RECEIVE_SAMPLE_RATE};
use super::pipeline::{AudioFrame, AudioPayload, SpeechPipeline};
use super::respond::MultiplexerVoiceModel;
use super::utterance::UtteranceAssembler;
use super::wav::spoken_text;
use super::VoicePacket;
use crate::{SessionKey, SessionMultiplexer};

const TRANSCRIPT_LIMIT: usize = 40;
const DEFAULT_JOIN_TIMEOUT_MS: u64 = 30_000;

/// Operator-facing huddle settings. Off unless `OMON_SLACK_HUDDLE_ENABLED` is true.
#[derive(Clone, Debug)]
pub struct HuddleSettings {
    enabled: bool,
    profile: Option<String>,
    chrome: Option<String>,
    node: String,
    sidecar: Option<String>,
    cdp_port: Option<String>,
    mic_label: String,
    playback: String,
    join_timeout: Duration,
}

impl HuddleSettings {
    pub fn from_env() -> Self {
        Self::from_lookup(&|name| std::env::var(name).ok())
    }

    pub fn from_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Self {
        let enabled = flag(lookup, "OMON_SLACK_HUDDLE_ENABLED");
        let sidecar = nonempty(lookup, "OMON_SLACK_HUDDLE_SIDECAR").or_else(|| {
            let relative = "sidecar/slack-huddle/index.mjs";
            std::path::Path::new(relative)
                .is_file()
                .then(|| relative.to_owned())
        });
        let join_ms = nonempty(lookup, "OMON_SLACK_HUDDLE_JOIN_TIMEOUT_MS")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_JOIN_TIMEOUT_MS);
        Self {
            enabled,
            profile: nonempty(lookup, "OMON_SLACK_HUDDLE_PROFILE"),
            chrome: nonempty(lookup, "OMON_SLACK_HUDDLE_CHROME"),
            node: nonempty(lookup, "OMON_SLACK_HUDDLE_NODE").unwrap_or_else(|| "node".to_owned()),
            sidecar,
            cdp_port: nonempty(lookup, "OMON_SLACK_HUDDLE_CDP_PORT"),
            mic_label: nonempty(lookup, "OMON_SLACK_HUDDLE_MIC_LABEL")
                .unwrap_or_else(default_mic_label),
            playback: nonempty(lookup, "OMON_SLACK_HUDDLE_PLAYBACK")
                .unwrap_or_else(default_playback_command),
            join_timeout: Duration::from_millis(join_ms),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn blocked_reason(&self, voice: &VoiceSettings) -> Option<String> {
        if !self.enabled {
            return Some(
                "Slack huddles are off. Set OMON_SLACK_HUDDLE_ENABLED=true to use a signed-in Slack web client."
                    .to_owned(),
            );
        }
        if self.profile.is_none() && self.cdp_port.is_none() {
            return Some(
                "Set OMON_SLACK_HUDDLE_PROFILE to a Chrome profile you have signed into the dedicated Slack account."
                    .to_owned(),
            );
        }
        if self.sidecar.is_none() {
            return Some(
                "Set OMON_SLACK_HUDDLE_SIDECAR to sidecar/slack-huddle/index.mjs.".to_owned(),
            );
        }
        if voice.stt().is_none() || voice.tts().is_none() {
            return Some(
                "Slack huddles need STT and TTS keys. Set OPENAI_API_KEY, or OMON_STT_API_KEY and OMON_TTS_API_KEY."
                    .to_owned(),
            );
        }
        None
    }
}

fn default_mic_label() -> String {
    if cfg!(target_os = "macos") {
        "BlackHole 2ch".to_owned()
    } else {
        "Omo Gateway Meeting Audio".to_owned()
    }
}

fn default_playback_command() -> String {
    if cfg!(target_os = "macos") {
        "sox -t raw -r \"$OMO_HUDDLE_RATE\" -e signed -b 16 -c 1 - -t coreaudio \"BlackHole 2ch\""
            .to_owned()
    } else {
        "pw-cat --playback --target omo_meeting --format s16 --rate \"$OMO_HUDDLE_RATE\" --channels 1 -"
            .to_owned()
    }
}

fn flag(lookup: &dyn Fn(&str) -> Option<String>, name: &str) -> bool {
    matches!(
        nonempty(lookup, name).as_deref().map(|value| value.to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn nonempty(lookup: &dyn Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// A huddle the operator asked to join. The URL is always `https://app.slack.com/huddle/...`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HuddleTarget {
    pub team: Option<String>,
    pub channel: String,
    pub url: String,
}

impl HuddleTarget {
    fn identity(&self) -> String {
        match &self.team {
            Some(team) => format!("slack-huddle:{team}:{}", self.channel),
            None => format!("slack-huddle:{}", self.channel),
        }
    }

    fn route_id(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in self.identity().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

pub fn normalize_huddle_target(input: &str) -> Result<HuddleTarget, HuddleError> {
    let value = input.trim();
    let identity = parse_identity(value).ok_or_else(|| {
        HuddleError::new(
            "invalid-target",
            "Use a Slack Copy huddle link (https://app.slack.com/huddle/TEAM/CHANNEL), an uppercase channel id such as C0123ABCD or channel:C0123ABCD, or team:T0123ABCD:channel:C0123ABCD.",
        )
    })?;
    let url = match &identity.team {
        Some(team) => format!("https://app.slack.com/huddle/{team}/{}", identity.channel),
        None => format!("https://app.slack.com/huddle/{}", identity.channel),
    };
    Ok(HuddleTarget {
        team: identity.team,
        channel: identity.channel,
        url,
    })
}

struct Identity {
    team: Option<String>,
    channel: String,
}

fn parse_identity(input: &str) -> Option<Identity> {
    if input.is_empty() {
        return None;
    }
    let without_slack = strip_prefix_ci(input, "slack:").unwrap_or(input);
    if let Some(rest) = strip_prefix_ci(without_slack, "team:") {
        let (team, rest) = rest.split_once(':')?;
        let channel = strip_prefix_ci(rest, "channel:")?;
        if team_id(team) && channel_id(channel) {
            return Some(Identity {
                team: Some(team.to_owned()),
                channel: channel.to_owned(),
            });
        }
        return None;
    }
    let channel = strip_prefix_ci(without_slack, "channel:").unwrap_or(without_slack);
    if channel_id(channel) {
        return Some(Identity {
            team: None,
            channel: channel.to_owned(),
        });
    }
    parse_huddle_url(input)
}

fn parse_huddle_url(input: &str) -> Option<Identity> {
    let rest = input.strip_prefix("https://")?;
    if rest.contains('@') || rest.contains('?') || rest.contains('#') {
        return None;
    }
    let (host, path) = rest.split_once('/')?;
    if host.contains(':') || !slack_host(host) {
        return None;
    }
    let path = path.trim_end_matches('/');
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    if parts.next() != Some("huddle") {
        return None;
    }
    let first = parts.next()?;
    let second = parts.next();
    if parts.next().is_some() {
        return None;
    }
    match second {
        None if channel_id(first) => Some(Identity {
            team: None,
            channel: first.to_owned(),
        }),
        Some(channel) if team_id(first) && channel_id(channel) => Some(Identity {
            team: Some(first.to_owned()),
            channel: channel.to_owned(),
        }),
        _ => None,
    }
}

fn slack_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let Some(name) = host.strip_suffix(".slack.com") else {
        return false;
    };
    !name.is_empty()
        && !name.contains('.')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn channel_id(value: &str) -> bool {
    slack_id(value, &['C', 'G', 'D'])
}

fn team_id(value: &str) -> bool {
    slack_id(value, &['T', 'E'])
}

fn slack_id(value: &str, prefixes: &[char]) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !prefixes.contains(&first) {
        return false;
    }
    let rest = chars.collect::<String>();
    rest.len() >= 8
        && rest
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn strip_prefix_ci<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    if value.len() >= prefix.len() && value[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&value[prefix.len()..])
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub struct HuddleError {
    pub reason: String,
    pub message: String,
}

impl HuddleError {
    fn new(reason: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HuddleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.message, self.reason)
    }
}

impl std::error::Error for HuddleError {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct HuddleStatus {
    #[serde(default)]
    phase: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    microphone: String,
    #[serde(default)]
    speech_ready: bool,
    #[serde(default)]
    captioning: bool,
    #[serde(default)]
    participants: u32,
    #[serde(default)]
    manual_reason: Option<String>,
    #[serde(default)]
    manual_message: Option<String>,
}

impl HuddleStatus {
    fn format_report(&self) -> String {
        if self.phase.is_empty() || self.phase == "idle" {
            return "Not in a huddle.".to_owned();
        }
        let mut lines = vec![format!("Slack huddle: {}", self.phase.replace('_', " "))];
        if !self.url.is_empty() {
            lines.push(format!("URL: {}", self.url));
        }
        if !self.microphone.is_empty() {
            lines.push(format!("Microphone: {}", self.microphone));
        }
        lines.push(format!(
            "Talk-back: {}",
            if self.speech_ready { "ready" } else { "muted" }
        ));
        lines.push(format!(
            "Captions: {}",
            if self.captioning { "on" } else { "off" }
        ));
        lines.push(format!("Participants visible: {}", self.participants));
        if let Some(message) = &self.manual_message {
            lines.push(message.clone());
        }
        lines.join("\n")
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CaptionLine {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct Wire {
    id: Option<u64>,
    ok: Option<bool>,
    reason: Option<String>,
    message: Option<String>,
    status: Option<HuddleStatus>,
    captions: Option<Vec<CaptionLine>>,
    event: Option<String>,
    speaker: Option<String>,
    pcm: Option<String>,
    #[serde(default)]
    ms: u32,
}

#[derive(Clone, Debug)]
enum BrowserEvent {
    Audio { speaker: String, samples: Vec<i16> },
    Silence { speaker: String, ms: u32 },
}

#[derive(Serialize)]
struct OutRequest<'a> {
    id: u64,
    op: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
}

struct Hub {
    pending: std::sync::Mutex<HashMap<u64, oneshot::Sender<Wire>>>,
    next: AtomicU64,
    events: broadcast::Sender<BrowserEvent>,
}

struct LineLink<W> {
    stdin: Mutex<tokio::io::BufWriter<W>>,
    hub: Arc<Hub>,
}

impl<W> LineLink<W>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    fn pair<R>(reader: R, writer: W) -> Arc<Self>
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let (events, _) = broadcast::channel(256);
        let hub = Arc::new(Hub {
            pending: std::sync::Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            events,
        });
        let link = Arc::new(Self {
            stdin: Mutex::new(tokio::io::BufWriter::new(writer)),
            hub: hub.clone(),
        });
        tokio::spawn(read_sidecar(reader, hub));
        link
    }

    fn subscribe(&self) -> broadcast::Receiver<BrowserEvent> {
        self.hub.events.subscribe()
    }

    async fn request(
        &self,
        op: &str,
        url: Option<&str>,
        timeout: Duration,
    ) -> Result<Wire, HuddleError> {
        let id = self.hub.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.hub
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(id, sender);
        let line = serde_json::to_string(&OutRequest { id, op, url })
            .map_err(|error| HuddleError::new("browser-control-unavailable", error.to_string()))?;
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await.map_err(io_error)?;
            stdin.write_all(b"\n").await.map_err(io_error)?;
            stdin.flush().await.map_err(io_error)?;
        }
        match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(wire)) => interpret_reply(wire),
            Ok(Err(_)) => Err(HuddleError::new(
                "browser-control-unavailable",
                "The huddle sidecar closed the reply.",
            )),
            Err(_) => {
                self.hub
                    .pending
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .remove(&id);
                Err(HuddleError::new(
                    "browser-control-unavailable",
                    "The huddle sidecar did not answer in time.",
                ))
            }
        }
    }
}

fn interpret_reply(wire: Wire) -> Result<Wire, HuddleError> {
    if wire.ok == Some(false) {
        return Err(HuddleError::new(
            wire.reason
                .unwrap_or_else(|| "browser-control-unavailable".to_owned()),
            wire.message
                .unwrap_or_else(|| "The huddle sidecar rejected the command.".to_owned()),
        ));
    }
    Ok(wire)
}

async fn read_sidecar<R>(reader: R, hub: Arc<Hub>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => dispatch_line(&line, &hub),
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "huddle sidecar output closed");
                break;
            }
        }
    }
    let pending = hub
        .pending
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .drain()
        .map(|(_, sender)| sender)
        .collect::<Vec<_>>();
    drop(pending);
}

fn dispatch_line(line: &str, hub: &Hub) {
    let wire: Wire = match serde_json::from_str(line) {
        Ok(wire) => wire,
        Err(error) => {
            tracing::debug!(%error, "ignored a non-JSON huddle sidecar line");
            return;
        }
    };
    if let Some(event) = wire.event.as_deref() {
        if let Some(event) = browser_event(event, &wire) {
            let _ = hub.events.send(event);
        }
        return;
    }
    if let Some(id) = wire.id {
        if let Some(sender) = hub
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&id)
        {
            let _ = sender.send(wire);
        }
    }
}

fn browser_event(event: &str, wire: &Wire) -> Option<BrowserEvent> {
    let speaker = wire
        .speaker
        .clone()
        .filter(|speaker| !speaker.trim().is_empty())
        .unwrap_or_else(|| "participant".to_owned());
    match event {
        "audio" => {
            let encoded = wire.pcm.as_deref().unwrap_or("");
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()?;
            let samples = bytes
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            if samples.is_empty() {
                None
            } else {
                Some(BrowserEvent::Audio { speaker, samples })
            }
        }
        "silence" => Some(BrowserEvent::Silence {
            speaker,
            ms: wire.ms.max(20),
        }),
        _ => None,
    }
}

fn io_error(error: std::io::Error) -> HuddleError {
    HuddleError::new("browser-control-unavailable", error.to_string())
}

#[async_trait]
trait HuddleBrowser: Send + Sync {
    async fn join_url(&self, url: &str) -> Result<HuddleStatus, HuddleError>;
    async fn leave_call(&self) -> Result<HuddleStatus, HuddleError>;
    async fn current(&self) -> Result<HuddleStatus, HuddleError>;
    async fn captions(&self) -> Result<Vec<CaptionLine>, HuddleError>;
    fn listen(&self) -> broadcast::Receiver<BrowserEvent>;
}

struct ProcessBrowser {
    settings: HuddleSettings,
    link: Mutex<Option<Arc<LineLink<ChildStdin>>>>,
    child: Mutex<Option<Child>>,
    events: broadcast::Sender<BrowserEvent>,
}

impl ProcessBrowser {
    fn new(settings: HuddleSettings) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            settings,
            link: Mutex::new(None),
            child: Mutex::new(None),
            events,
        }
    }

    async fn link(&self) -> Result<Arc<LineLink<ChildStdin>>, HuddleError> {
        let mut slot = self.link.lock().await;
        if let Some(link) = slot.clone() {
            return Ok(link);
        }
        let sidecar = self.settings.sidecar.clone().ok_or_else(|| {
            HuddleError::new(
                "browser-control-unavailable",
                "OMON_SLACK_HUDDLE_SIDECAR is not set.",
            )
        })?;
        let mut command = tokio::process::Command::new(&self.settings.node);
        command
            .arg(&sidecar)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .env("OMO_HUDDLE_MIC_LABEL", &self.settings.mic_label)
            .env(
                "OMO_HUDDLE_JOIN_TIMEOUT_MS",
                self.settings.join_timeout.as_millis().to_string(),
            );
        if let Some(profile) = &self.settings.profile {
            command.env("OMO_HUDDLE_PROFILE", profile);
        }
        if let Some(chrome) = &self.settings.chrome {
            command.env("OMO_HUDDLE_CHROME", chrome);
        }
        if let Some(port) = &self.settings.cdp_port {
            command.env("OMO_HUDDLE_CDP_PORT", port);
        }
        let mut child = command.spawn().map_err(|error| {
            HuddleError::new(
                "browser-control-unavailable",
                format!(
                    "Could not start the huddle sidecar ({}): {error}",
                    self.settings.node
                ),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            HuddleError::new(
                "browser-control-unavailable",
                "huddle sidecar has no stdout",
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            HuddleError::new("browser-control-unavailable", "huddle sidecar has no stdin")
        })?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "slack_huddle", "{line}");
                }
            });
        }
        let link = LineLink::pair(stdout, stdin);
        // The process link has its own broadcast channel. Forward into the one
        // listeners already subscribed to before the process existed.
        let mut inbound = link.subscribe();
        let outbound = self.events.clone();
        tokio::spawn(async move {
            loop {
                match inbound.recv().await {
                    Ok(event) => {
                        let _ = outbound.send(event);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        *slot = Some(link.clone());
        *self.child.lock().await = Some(child);
        Ok(link)
    }
}

#[async_trait]
impl HuddleBrowser for ProcessBrowser {
    async fn join_url(&self, url: &str) -> Result<HuddleStatus, HuddleError> {
        let link = self.link().await?;
        let wire = link
            .request(
                "join",
                Some(url),
                self.settings.join_timeout + Duration::from_secs(10),
            )
            .await?;
        wire.status.ok_or_else(|| {
            HuddleError::new(
                "browser-control-unavailable",
                "The huddle sidecar joined without reporting status.",
            )
        })
    }

    async fn leave_call(&self) -> Result<HuddleStatus, HuddleError> {
        let link = self.link().await?;
        let wire = link.request("leave", None, Duration::from_secs(20)).await?;
        Ok(wire.status.unwrap_or_default())
    }

    async fn current(&self) -> Result<HuddleStatus, HuddleError> {
        let Some(link) = self.link.lock().await.clone() else {
            return Ok(HuddleStatus::default());
        };
        let wire = link
            .request("status", None, Duration::from_secs(15))
            .await?;
        Ok(wire.status.unwrap_or_default())
    }

    async fn captions(&self) -> Result<Vec<CaptionLine>, HuddleError> {
        let Some(link) = self.link.lock().await.clone() else {
            return Ok(Vec::new());
        };
        let wire = link
            .request("transcript", None, Duration::from_secs(15))
            .await?;
        Ok(wire.captions.unwrap_or_default())
    }

    fn listen(&self) -> broadcast::Receiver<BrowserEvent> {
        self.events.subscribe()
    }
}

#[async_trait]
trait VirtualMic: Send + Sync {
    async fn play(&self, sample_rate: u32, pcm: &[u8]) -> Result<(), String>;
    async fn stop(&self);
}

#[derive(Clone)]
struct CommandMic {
    command: String,
    child: Arc<Mutex<Option<Child>>>,
}

impl CommandMic {
    fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl VirtualMic for CommandMic {
    async fn play(&self, sample_rate: u32, pcm: &[u8]) -> Result<(), String> {
        self.stop().await;
        if pcm.is_empty() {
            return Ok(());
        }
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .env("OMO_HUDDLE_RATE", sample_rate.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("Could not play into the virtual microphone: {error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "virtual microphone playback has no stdin".to_owned())?;
        *self.child.lock().await = Some(child);
        if stdin.write_all(pcm).await.is_err() {
            self.stop().await;
            return Ok(());
        }
        drop(stdin);
        loop {
            {
                let mut slot = self.child.lock().await;
                let Some(child) = slot.as_mut() else {
                    return Ok(());
                };
                match child.try_wait() {
                    Ok(Some(status)) => {
                        *slot = None;
                        return if status.success() {
                            Ok(())
                        } else {
                            Err(format!("virtual microphone playback exited {status}"))
                        };
                    }
                    Ok(None) => {}
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

fn frames_to_s16le(frames: &[AudioFrame]) -> Result<(u32, Vec<u8>), String> {
    let Some(first) = frames.iter().find(|frame| frame.byte_len() > 0) else {
        return Err("no speech audio to play".into());
    };
    if first.channels != 1 {
        return Err("huddle playback expects mono PCM".into());
    }
    let mut pcm = Vec::new();
    for frame in frames {
        if frame.byte_len() == 0 {
            continue;
        }
        if frame.sample_rate != first.sample_rate || frame.channels != 1 {
            return Err("huddle playback frames must share one mono format".into());
        }
        match &frame.payload {
            AudioPayload::Pcm(samples) => {
                for sample in samples {
                    pcm.extend_from_slice(&sample.to_le_bytes());
                }
            }
            AudioPayload::Opus(_) => return Err("huddle playback expects PCM".into()),
        }
    }
    Ok((first.sample_rate, pcm))
}

struct TranscriptLine {
    source: &'static str,
    text: String,
}

struct RunState {
    target: Option<HuddleTarget>,
    route_id: u64,
    session: Option<SessionKey>,
    cancel: Option<CancellationToken>,
    turn: Arc<AtomicU64>,
    speech_ready: bool,
    status: HuddleStatus,
}

struct Inner {
    blocked: Option<String>,
    silence: Duration,
    min_speech: Duration,
    max_speech: Duration,
    pipeline: Option<SpeechPipeline>,
    browser: Option<Arc<dyn HuddleBrowser>>,
    mic: Arc<dyn VirtualMic>,
    sessions: Arc<parking_lot::Mutex<HashMap<u64, SessionKey>>>,
    state: Mutex<RunState>,
    transcripts: Mutex<VecDeque<TranscriptLine>>,
    gate: Mutex<()>,
}

/// Slash-command surface for [`HuddleRuntime`].
#[async_trait]
pub trait HuddleControl: Send + Sync {
    async fn join(&self, target: &str, session: SessionKey) -> Result<String, String>;
    async fn leave(&self) -> Result<String, String>;
    async fn status(&self) -> Result<String, String>;
    async fn speak(&self, text: &str) -> Result<String, String>;
    async fn transcript(&self) -> Result<String, String>;
}

/// One huddle at a time, for the dedicated Slack account.
#[derive(Clone)]
pub struct HuddleRuntime {
    inner: Arc<Inner>,
}

impl HuddleRuntime {
    pub fn from_voice(
        settings: HuddleSettings,
        voice: &VoiceSettings,
        multiplexer: SessionMultiplexer,
        pool: sqlx::SqlitePool,
    ) -> Self {
        let blocked = settings.blocked_reason(voice);
        let sessions = Arc::new(parking_lot::Mutex::new(HashMap::new()));
        let pipeline = if blocked.is_none() {
            match (voice.stt(), voice.tts()) {
                (Some(stt), Some(tts)) => {
                    let stt = Arc::new(
                        super::pipeline::OpenAiSpeechToText::new(&stt.api_key, &stt.base_url)
                            .with_model(&stt.model),
                    );
                    let tts = Arc::new(
                        super::tts::OpenAiTextToSpeech::new(&tts.api_key, &tts.base_url)
                            .with_model(&tts.model)
                            .with_voice(voice.tts_voice()),
                    );
                    let model = Arc::new(MultiplexerVoiceModel::new(
                        multiplexer,
                        pool,
                        sessions.clone(),
                    ));
                    Some(SpeechPipeline::new(stt, model, tts))
                }
                _ => None,
            }
        } else {
            None
        };
        let playback = settings.playback.clone();
        let browser = blocked
            .is_none()
            .then(|| Arc::new(ProcessBrowser::new(settings)) as Arc<dyn HuddleBrowser>);
        Self::assemble(
            blocked,
            voice.silence(),
            voice.min_speech(),
            voice.max_speech(),
            pipeline,
            browser,
            Arc::new(CommandMic::new(playback)),
            sessions,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn assemble(
        blocked: Option<String>,
        silence: Duration,
        min_speech: Duration,
        max_speech: Duration,
        pipeline: Option<SpeechPipeline>,
        browser: Option<Arc<dyn HuddleBrowser>>,
        mic: Arc<dyn VirtualMic>,
        sessions: Arc<parking_lot::Mutex<HashMap<u64, SessionKey>>>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                blocked,
                silence,
                min_speech,
                max_speech,
                pipeline,
                browser,
                mic,
                sessions,
                state: Mutex::new(RunState {
                    target: None,
                    route_id: 0,
                    session: None,
                    cancel: None,
                    turn: Arc::new(AtomicU64::new(1)),
                    speech_ready: false,
                    status: HuddleStatus::default(),
                }),
                transcripts: Mutex::new(VecDeque::new()),
                gate: Mutex::new(()),
            }),
        }
    }

    async fn remember(&self, source: &'static str, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let mut lines = self.inner.transcripts.lock().await;
        if lines
            .back()
            .is_some_and(|line| line.source == source && line.text == text)
        {
            return;
        }
        lines.push_back(TranscriptLine {
            source,
            text: text.to_owned(),
        });
        while lines.len() > TRANSCRIPT_LIMIT {
            lines.pop_front();
        }
    }
}

#[async_trait]
impl HuddleControl for HuddleRuntime {
    async fn join(&self, target: &str, session: SessionKey) -> Result<String, String> {
        if let Some(reason) = &self.inner.blocked {
            return Err(reason.clone());
        }
        let target = normalize_huddle_target(target).map_err(|error| error.message)?;
        let _guard = self.inner.gate.lock().await;
        {
            let state = self.inner.state.lock().await;
            if state.status.phase == "in_call" {
                if state
                    .target
                    .as_ref()
                    .is_some_and(|current| current == &target)
                {
                    return Ok("Already in this huddle.".into());
                }
                return Err(
                    "This account is already in a huddle. Leave it before joining another.".into(),
                );
            }
        }
        let browser = self
            .inner
            .browser
            .clone()
            .ok_or_else(|| "Slack huddle browser is not configured.".to_owned())?;
        let pipeline = self
            .inner
            .pipeline
            .clone()
            .ok_or_else(|| "Slack huddle speech is not configured.".to_owned())?;
        let events = browser.listen();
        let status = browser
            .join_url(&target.url)
            .await
            .map_err(|error| error.message)?;
        let route_id = target.route_id();
        self.inner.sessions.lock().insert(route_id, session.clone());
        let cancel = CancellationToken::new();
        {
            let mut state = self.inner.state.lock().await;
            if let Some(previous) = state.cancel.replace(cancel.clone()) {
                previous.cancel();
            }
            state.turn.fetch_add(1, Ordering::AcqRel);
            state.target = Some(target.clone());
            state.route_id = route_id;
            state.session = Some(session);
            state.speech_ready = status.speech_ready;
            state.status = status.clone();
        }
        let runtime = self.clone();
        tokio::spawn(listen_huddle(runtime, events, pipeline, route_id, cancel));
        let mut message = format!(
            "Joined the huddle for <#{}>. Speech in the huddle is transcribed into this conversation, and the reply is played through the virtual microphone.",
            target.channel
        );
        if let Some(note) = status.manual_message {
            message.push(' ');
            message.push_str(&note);
        }
        Ok(message)
    }

    async fn leave(&self) -> Result<String, String> {
        if let Some(reason) = &self.inner.blocked {
            return Err(reason.clone());
        }
        let _guard = self.inner.gate.lock().await;
        let cancel = {
            let mut state = self.inner.state.lock().await;
            state.speech_ready = false;
            state.session = None;
            state.target = None;
            state.status = HuddleStatus {
                phase: "left".into(),
                ..HuddleStatus::default()
            };
            state.turn.fetch_add(1, Ordering::AcqRel);
            state.cancel.take()
        };
        if let Some(cancel) = cancel {
            cancel.cancel();
        }
        self.inner.mic.stop().await;
        if let Some(browser) = &self.inner.browser {
            browser.leave_call().await.map_err(|error| error.message)?;
        }
        Ok("Left the huddle. The huddle keeps going for everyone else.".into())
    }

    async fn status(&self) -> Result<String, String> {
        if let Some(reason) = &self.inner.blocked {
            return Err(reason.clone());
        }
        let Some(browser) = &self.inner.browser else {
            return Ok("Not in a huddle.".into());
        };
        match browser.current().await {
            Ok(status) => {
                self.inner.state.lock().await.status = status.clone();
                Ok(status.format_report())
            }
            Err(error) => Err(error.message),
        }
    }

    async fn speak(&self, text: &str) -> Result<String, String> {
        if let Some(reason) = &self.inner.blocked {
            return Err(reason.clone());
        }
        let (route_id, speech_ready, phase, note) = {
            let state = self.inner.state.lock().await;
            (
                state.route_id,
                state.speech_ready,
                state.status.phase.clone(),
                state.status.manual_message.clone(),
            )
        };
        if phase != "in_call" {
            return Err("Not in a huddle.".into());
        }
        if !speech_ready {
            return Err(note.unwrap_or_else(|| {
                "The virtual microphone is not selected, so the huddle stays muted.".into()
            }));
        }
        let spoken = spoken_text(text);
        if spoken.is_empty() {
            return Err("Nothing to say.".into());
        }
        let pipeline = self
            .inner
            .pipeline
            .clone()
            .ok_or_else(|| "Slack huddle speech is not configured.".to_owned())?;
        let frames = pipeline
            .tts()
            .synthesize(route_id, &spoken)
            .await
            .map_err(|error| error.to_string())?;
        let (rate, pcm) = frames_to_s16le(&frames)?;
        self.inner
            .state
            .lock()
            .await
            .turn
            .fetch_add(1, Ordering::AcqRel);
        self.inner.mic.stop().await;
        self.inner.mic.play(rate, &pcm).await?;
        Ok("Spoke into the huddle.".into())
    }

    async fn transcript(&self) -> Result<String, String> {
        if let Some(reason) = &self.inner.blocked {
            return Err(reason.clone());
        }
        if let Some(browser) = &self.inner.browser {
            if let Ok(captions) = browser.captions().await {
                for caption in captions {
                    let text = match caption.speaker.trim() {
                        "" => caption.text,
                        speaker => format!("{speaker}: {}", caption.text),
                    };
                    self.remember("caption", &text).await;
                }
            }
        }
        let lines = self.inner.transcripts.lock().await;
        if lines.is_empty() {
            return Ok("No transcript yet.".into());
        }
        Ok(lines
            .iter()
            .map(|line| format!("[{}] {}", line.source, line.text))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

#[allow(clippy::too_many_arguments)]
async fn listen_huddle(
    runtime: HuddleRuntime,
    mut events: broadcast::Receiver<BrowserEvent>,
    pipeline: SpeechPipeline,
    route_id: u64,
    cancel: CancellationToken,
) {
    let mut assembler = UtteranceAssembler::new(
        RECEIVE_SAMPLE_RATE,
        RECEIVE_CHANNELS,
        runtime.inner.silence,
        runtime.inner.min_speech,
        runtime.inner.max_speech,
    );
    let turn = runtime.inner.state.lock().await.turn.clone();
    let mut worker: Option<tokio::task::JoinHandle<()>> = None;
    loop {
        let event = tokio::select! {
            _ = cancel.cancelled() => break,
            event = events.recv() => event,
        };
        let event = match event {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        };
        match event {
            BrowserEvent::Audio { speaker, samples } => {
                if let Some(handle) = worker.take() {
                    turn.fetch_add(1, Ordering::AcqRel);
                    handle.abort();
                    runtime.inner.mic.stop().await;
                }
                let ssrc = speaker_ssrc(&speaker);
                assembler.push(VoicePacket::Label {
                    ssrc,
                    name: speaker,
                });
                assembler.push(VoicePacket::Pcm { ssrc, samples });
            }
            BrowserEvent::Silence { speaker, ms } => {
                let ssrc = speaker_ssrc(&speaker);
                let ticks = (u64::from(ms) / 20).max(1);
                for _ in 0..ticks {
                    let Some(utterance) = assembler.push(VoicePacket::Silent { ssrc }) else {
                        continue;
                    };
                    if let Some(handle) = worker.take() {
                        handle.abort();
                        runtime.inner.mic.stop().await;
                    }
                    let mine = turn.fetch_add(1, Ordering::AcqRel) + 1;
                    let pipeline = pipeline.clone();
                    let turn = turn.clone();
                    let mic = runtime.inner.mic.clone();
                    let runtime = runtime.clone();
                    let speech_ready = runtime.inner.state.lock().await.speech_ready;
                    worker = Some(tokio::spawn(async move {
                        let label = utterance.speaker_label();
                        let frames = utterance.frames(route_id);
                        let output = match pipeline
                            .process_utterance(route_id, label.as_deref(), &frames)
                            .await
                        {
                            Ok(output) => output,
                            Err(error) => {
                                tracing::warn!(%error, "huddle voice turn failed");
                                return;
                            }
                        };
                        runtime.remember("stt", &output.transcript).await;
                        if output.audio.is_empty()
                            || turn.load(Ordering::Acquire) != mine
                            || !speech_ready
                        {
                            return;
                        }
                        match frames_to_s16le(&output.audio) {
                            Ok((rate, pcm)) => {
                                if turn.load(Ordering::Acquire) != mine {
                                    return;
                                }
                                if let Err(error) = mic.play(rate, &pcm).await {
                                    tracing::warn!(%error, "huddle playback failed");
                                }
                            }
                            Err(error) => tracing::warn!(%error, "huddle playback failed"),
                        }
                    }));
                }
            }
        }
    }
    if let Some(handle) = worker.take() {
        handle.abort();
    }
    runtime.inner.mic.stop().await;
}

fn speaker_ssrc(name: &str) -> u32 {
    let mut hash = 2166136261u32;
    for byte in name.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    hash.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::{AudioDirection, SpeechToText, TextToSpeech, VoiceLanguageModel};
    use crate::Result;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    #[test]
    fn huddles_are_off_until_the_flag_is_set() {
        let settings = HuddleSettings::from_lookup(&|_| None);
        assert!(!settings.enabled());
        let voice = VoiceSettings::disabled();
        let reason = settings.blocked_reason(&voice).unwrap();
        assert!(reason.contains("OMON_SLACK_HUDDLE_ENABLED"));
    }

    #[test]
    fn an_enabled_huddle_still_needs_a_profile_and_speech_keys() {
        let lookup = |name: &str| match name {
            "OMON_SLACK_HUDDLE_ENABLED" => Some("true".into()),
            "OMON_SLACK_HUDDLE_SIDECAR" => Some("sidecar/slack-huddle/index.mjs".into()),
            _ => None,
        };
        let settings = HuddleSettings::from_lookup(&lookup);
        let voice = VoiceSettings::disabled();
        let reason = settings.blocked_reason(&voice).unwrap();
        assert!(reason.contains("OMON_SLACK_HUDDLE_PROFILE"));
    }

    #[test]
    fn huddle_links_and_channel_refs_normalize_and_other_urls_do_not() {
        let link =
            normalize_huddle_target("https://app.slack.com/huddle/T0123ABCD/C0123ABCD").unwrap();
        assert_eq!(link.team.as_deref(), Some("T0123ABCD"));
        assert_eq!(link.channel, "C0123ABCD");
        assert_eq!(link.url, "https://app.slack.com/huddle/T0123ABCD/C0123ABCD");
        let channel = normalize_huddle_target("channel:C0123ABCD").unwrap();
        assert!(channel.team.is_none());
        assert_ne!(link.identity(), channel.identity());
        let qualified = normalize_huddle_target("slack:team:E0123ABCD:channel:G0123ABCD").unwrap();
        assert_eq!(qualified.team.as_deref(), Some("E0123ABCD"));
        assert_eq!(qualified.channel, "G0123ABCD");
        assert!(normalize_huddle_target("D0123ABCD").is_ok());
        for rejected in [
            "https://app.slack.com/client/T0123ABCD/C0123ABCD",
            "https://acme.slack.com/archives/C0123ABCD/p123",
            "http://app.slack.com/huddle/C0123ABCD",
            "https://user:pass@app.slack.com/huddle/C0123ABCD",
            "U0123ABCD",
            "c0123abcd",
            "slack://huddle",
        ] {
            assert!(normalize_huddle_target(rejected).is_err(), "{rejected}");
        }
    }

    #[test]
    fn sidecar_source_stays_on_the_web_ui() {
        let source = include_str!("../../sidecar/slack-huddle/index.mjs");
        for needle in [
            "xoxc",
            "xoxd",
            "document.cookie",
            "localStorage",
            "chime.aws",
            "amazonaws.com",
            "slack.com/api",
        ] {
            assert!(
                !source.to_ascii_lowercase().contains(needle),
                "{needle} must not appear in the sidecar"
            );
        }
        assert!(source.contains("127.0.0.1"));
        assert!(source.contains("RTCPeerConnection"));
    }

    #[tokio::test]
    async fn sidecar_lines_join_and_emit_audio_without_a_browser() {
        let (client, server) = tokio::io::duplex(8_192);
        let (client_read, client_write) = tokio::io::split(client);
        let (server_read, mut server_write) = tokio::io::split(server);
        let link = LineLink::pair(client_read, client_write);
        let mut events = link.subscribe();
        tokio::spawn(async move {
            let mut lines = BufReader::new(server_read).lines();
            let line = lines.next_line().await.unwrap().unwrap();
            assert!(line.contains("\"op\":\"join\""));
            assert!(line.contains("app.slack.com/huddle"));
            server_write
                .write_all(b"{\"id\":1,\"ok\":true,\"status\":{\"phase\":\"in_call\",\"speech_ready\":true}}\n")
                .await
                .unwrap();
            server_write
                .write_all(b"{\"event\":\"audio\",\"speaker\":\"Ada\",\"pcm\":\"AAD//w==\"}\n")
                .await
                .unwrap();
            server_write.flush().await.unwrap();
        });
        let reply = link
            .request(
                "join",
                Some("https://app.slack.com/huddle/C0123ABCD"),
                Duration::from_secs(2),
            )
            .await
            .unwrap();
        assert_eq!(reply.status.unwrap().phase, "in_call");
        match events.recv().await.unwrap() {
            BrowserEvent::Audio { speaker, samples } => {
                assert_eq!(speaker, "Ada");
                assert_eq!(samples, vec![0, -1]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    struct RecordingModel {
        seen: Mutex<Vec<(u64, String)>>,
    }

    #[async_trait]
    impl VoiceLanguageModel for RecordingModel {
        async fn respond(&self, channel_id: u64, transcript: &str) -> Result<String> {
            self.seen
                .lock()
                .await
                .push((channel_id, transcript.to_owned()));
            Ok("reply text".into())
        }
    }

    struct FixedStt;

    #[async_trait]
    impl SpeechToText for FixedStt {
        async fn transcribe(&self, _frames: &[AudioFrame]) -> Result<String> {
            Ok("hello".into())
        }
    }

    struct FixedTts;

    #[async_trait]
    impl TextToSpeech for FixedTts {
        async fn synthesize(&self, _channel_id: u64, text: &str) -> Result<Vec<AudioFrame>> {
            assert!(!text.is_empty());
            Ok(vec![AudioFrame {
                channel_id: 0,
                source_id: None,
                sequence: 0,
                sample_rate: 24_000,
                channels: 1,
                direction: AudioDirection::Outgoing,
                payload: AudioPayload::Pcm(vec![1000, -1000]),
            }])
        }
    }

    struct GateMic {
        started: tokio::sync::Notify,
        release: tokio::sync::Notify,
        plays: AtomicUsize,
        stops: AtomicUsize,
        hold: AtomicBool,
    }

    #[async_trait]
    impl VirtualMic for GateMic {
        async fn play(&self, _sample_rate: u32, pcm: &[u8]) -> Result<(), String> {
            assert!(!pcm.is_empty());
            self.plays.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            if self.hold.load(Ordering::SeqCst) {
                self.release.notified().await;
            }
            Ok(())
        }

        async fn stop(&self) {
            self.stops.fetch_add(1, Ordering::SeqCst);
            self.release.notify_waiters();
        }
    }

    struct MockBrowser {
        events: broadcast::Sender<BrowserEvent>,
        joined: Mutex<Option<String>>,
    }

    impl MockBrowser {
        fn new() -> Arc<Self> {
            let (events, _) = broadcast::channel(16);
            Arc::new(Self {
                events,
                joined: Mutex::new(None),
            })
        }
    }

    #[async_trait]
    impl HuddleBrowser for MockBrowser {
        async fn join_url(&self, url: &str) -> Result<HuddleStatus, HuddleError> {
            *self.joined.lock().await = Some(url.to_owned());
            Ok(HuddleStatus {
                phase: "in_call".into(),
                url: url.to_owned(),
                speech_ready: true,
                microphone: "Omo Gateway Meeting Audio".into(),
                ..HuddleStatus::default()
            })
        }

        async fn leave_call(&self) -> Result<HuddleStatus, HuddleError> {
            *self.joined.lock().await = None;
            Ok(HuddleStatus {
                phase: "left".into(),
                ..HuddleStatus::default()
            })
        }

        async fn current(&self) -> Result<HuddleStatus, HuddleError> {
            let joined = self.joined.lock().await.clone();
            Ok(HuddleStatus {
                phase: if joined.is_some() {
                    "in_call".into()
                } else {
                    "idle".into()
                },
                url: joined.unwrap_or_default(),
                speech_ready: true,
                ..HuddleStatus::default()
            })
        }

        async fn captions(&self) -> Result<Vec<CaptionLine>, HuddleError> {
            Ok(vec![CaptionLine {
                speaker: "Ada".into(),
                text: "from the captions".into(),
            }])
        }

        fn listen(&self) -> broadcast::Receiver<BrowserEvent> {
            self.events.subscribe()
        }
    }

    fn test_runtime(
        browser: Arc<MockBrowser>,
        mic: Arc<GateMic>,
        model: Arc<RecordingModel>,
    ) -> HuddleRuntime {
        let pipeline = SpeechPipeline::new(Arc::new(FixedStt), model, Arc::new(FixedTts));
        HuddleRuntime::assemble(
            None,
            Duration::from_millis(40),
            Duration::from_millis(40),
            Duration::from_millis(30_000),
            Some(pipeline),
            Some(browser as Arc<dyn HuddleBrowser>),
            mic as Arc<dyn VirtualMic>,
            Arc::new(parking_lot::Mutex::new(HashMap::new())),
        )
    }

    #[tokio::test]
    async fn remote_audio_uses_the_session_pipeline_and_a_new_utterance_stops_playback() {
        let browser = MockBrowser::new();
        let mic = Arc::new(GateMic {
            started: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
            plays: AtomicUsize::new(0),
            stops: AtomicUsize::new(0),
            hold: AtomicBool::new(true),
        });
        let model = Arc::new(RecordingModel {
            seen: Mutex::new(Vec::new()),
        });
        let runtime = test_runtime(browser.clone(), mic.clone(), model.clone());
        let session = SessionKey::new("slack", Some("T1"), "C0123ABCD", None::<String>, "");
        let joined = runtime
            .join("channel:C0123ABCD", session.clone())
            .await
            .unwrap();
        assert!(joined.contains("<#C0123ABCD>"));
        assert_eq!(
            runtime.inner.state.lock().await.session.as_ref(),
            Some(&session)
        );

        let samples = vec![1i16; 640];
        browser
            .events
            .send(BrowserEvent::Audio {
                speaker: "Ada".into(),
                samples,
            })
            .unwrap();
        browser
            .events
            .send(BrowserEvent::Silence {
                speaker: "Ada".into(),
                ms: 40,
            })
            .unwrap();

        tokio::time::timeout(Duration::from_secs(2), mic.started.notified())
            .await
            .expect("playback should start");
        let seen = model.seen.lock().await.clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].1.contains("Ada"));
        assert!(seen[0].1.contains("hello"));
        assert!(mic.plays.load(Ordering::SeqCst) >= 1);

        browser
            .events
            .send(BrowserEvent::Audio {
                speaker: "Ada".into(),
                samples: vec![1; 32],
            })
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while mic.stops.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("barge-in stops playback");

        let transcript = runtime.transcript().await.unwrap();
        assert!(transcript.contains("[stt]"));
        assert!(transcript.contains("[caption] Ada: from the captions"));
        let status = runtime.status().await.unwrap();
        assert!(status.contains("in call"));
        mic.hold.store(false, Ordering::SeqCst);
        let spoken = runtime.speak("hello huddle").await.unwrap();
        assert!(spoken.contains("Spoke"));
        runtime.leave().await.unwrap();
        assert!(browser.joined.lock().await.is_none());
        assert!(runtime
            .speak("too late")
            .await
            .unwrap_err()
            .contains("Not in a huddle"));
    }

    #[tokio::test]
    async fn playback_stop_ends_the_child_and_a_disabled_runtime_stays_quiet() {
        let settings = HuddleSettings::from_lookup(&|_| None);
        let reason = settings.blocked_reason(&VoiceSettings::disabled()).unwrap();
        assert!(reason.contains("off"));

        let mic = CommandMic::new("sleep 5");
        let play = {
            let mic = mic.clone();
            tokio::spawn(async move { mic.play(16_000, &[0, 1, 2, 3]).await })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        mic.stop().await;
        let result = tokio::time::timeout(Duration::from_secs(2), play)
            .await
            .expect("stopping the virtual microphone ends playback");
        assert!(result.unwrap().is_ok());
    }
}

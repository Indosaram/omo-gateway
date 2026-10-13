use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::{OmonError, Result};
use crate::storage::{get_stale_in_progress_work_items, update_work_item_progress};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkHeartbeatConfig {
    #[serde(default = "default_heartbeat_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(
        default = "default_heartbeat_interval",
        deserialize_with = "deserialize_duration_secs"
    )]
    pub interval: Duration,
    #[serde(
        default = "default_heartbeat_threshold",
        deserialize_with = "deserialize_duration_secs"
    )]
    pub threshold: Duration,
}

fn default_heartbeat_enabled() -> bool {
    true
}

fn default_heartbeat_interval() -> Duration {
    Duration::from_secs(60)
}

fn default_heartbeat_threshold() -> Duration {
    Duration::from_secs(15 * 60)
}

fn deserialize_duration_secs<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DurRepr {
        Secs(u64),
        Obj { secs: u64, nanos: Option<u32> },
    }
    match DurRepr::deserialize(deserializer)? {
        DurRepr::Secs(s) => Ok(Duration::from_secs(s)),
        DurRepr::Obj { secs, nanos } => Ok(Duration::new(secs, nanos.unwrap_or(0))),
    }
}

impl Default for WorkHeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: default_heartbeat_enabled(),
            channel_id: None,
            interval: default_heartbeat_interval(),
            threshold: default_heartbeat_threshold(),
        }
    }
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone)]
pub struct MockClock {
    current: Arc<parking_lot::Mutex<DateTime<Utc>>>,
}

impl MockClock {
    pub fn new(initial: DateTime<Utc>) -> Self {
        Self {
            current: Arc::new(parking_lot::Mutex::new(initial)),
        }
    }

    pub fn set(&self, time: DateTime<Utc>) {
        *self.current.lock() = time;
    }

    pub fn advance(&self, duration: chrono::Duration) {
        *self.current.lock() += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.current.lock()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegateLiveness {
    Alive,
    Dead,
    Unknown,
}

#[async_trait]
pub trait DelegateLivenessChecker: Send + Sync {
    async fn check_liveness(&self, pane: &str) -> DelegateLiveness;
}

#[derive(Clone, Default)]
pub struct MockLivenessChecker {
    statuses: Arc<parking_lot::Mutex<HashMap<String, DelegateLiveness>>>,
}

impl MockLivenessChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_status(&self, pane: impl Into<String>, status: DelegateLiveness) {
        self.statuses.lock().insert(pane.into(), status);
    }
}

#[async_trait]
impl DelegateLivenessChecker for MockLivenessChecker {
    async fn check_liveness(&self, pane: &str) -> DelegateLiveness {
        self.statuses
            .lock()
            .get(pane)
            .copied()
            .unwrap_or(DelegateLiveness::Unknown)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedDiscordPayload {
    pub method: String,
    pub path: String,
    pub body: serde_json::Value,
}

#[async_trait]
pub trait WorkHeartbeatSink: Send + Sync {
    async fn post_message(&self, channel_id: &str, content: &str)
        -> Result<CapturedDiscordPayload>;
}

pub struct SerenityHeartbeatSink {
    http: Arc<serenity::all::Http>,
}

impl SerenityHeartbeatSink {
    pub fn new(http: Arc<serenity::all::Http>) -> Self {
        Self { http }
    }
}

#[async_trait]
impl WorkHeartbeatSink for SerenityHeartbeatSink {
    async fn post_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<CapturedDiscordPayload> {
        let channel_num = channel_id.parse::<u64>().map_err(|e| {
            OmonError::Config(format!("invalid Discord channel_id '{channel_id}': {e}"))
        })?;
        let channel = serenity::all::ChannelId::new(channel_num);
        let builder = serenity::all::CreateMessage::new()
            .content(content)
            .allowed_mentions(crate::discord::adapter::safe_allowed_mentions());
        channel.send_message(&self.http, builder).await?;

        Ok(CapturedDiscordPayload {
            method: "POST".to_string(),
            path: format!("/api/v10/channels/{channel_id}/messages"),
            body: serde_json::json!({
                "content": content,
                "allowed_mentions": {
                    "all_users": true,
                    "everyone": false,
                    "all_roles": false,
                    "replied_user": true
                }
            }),
        })
    }
}

#[derive(Clone, Default)]
pub struct RecordingHeartbeatSink {
    payloads: Arc<parking_lot::Mutex<Vec<CapturedDiscordPayload>>>,
}

impl RecordingHeartbeatSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recorded_payloads(&self) -> Vec<CapturedDiscordPayload> {
        self.payloads.lock().clone()
    }
}

#[async_trait]
impl WorkHeartbeatSink for RecordingHeartbeatSink {
    async fn post_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<CapturedDiscordPayload> {
        let payload = CapturedDiscordPayload {
            method: "POST".to_string(),
            path: format!("/api/v10/channels/{channel_id}/messages"),
            body: serde_json::json!({
                "content": content,
                "allowed_mentions": {
                    "all_users": true,
                    "everyone": false,
                    "all_roles": false,
                    "replied_user": true
                }
            }),
        };
        self.payloads.lock().push(payload.clone());
        Ok(payload)
    }
}

pub struct WorkHeartbeatRunner<C, L, S> {
    pool: SqlitePool,
    config: WorkHeartbeatConfig,
    clock: Arc<C>,
    checker: Arc<L>,
    sink: Arc<S>,
}

impl<C: Clock + 'static, L: DelegateLivenessChecker + 'static, S: WorkHeartbeatSink + 'static>
    WorkHeartbeatRunner<C, L, S>
{
    pub fn new(
        pool: SqlitePool,
        config: WorkHeartbeatConfig,
        clock: Arc<C>,
        checker: Arc<L>,
        sink: Arc<S>,
    ) -> Self {
        Self {
            pool,
            config,
            clock,
            checker,
            sink,
        }
    }

    pub async fn step(&self) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        let now = self.clock.now();
        let threshold_duration = chrono::Duration::from_std(self.config.threshold)
            .unwrap_or_else(|_| chrono::Duration::minutes(15));
        let cutoff = now - threshold_duration;
        let cutoff_str = cutoff.to_rfc3339();

        let items = get_stale_in_progress_work_items(&self.pool, &cutoff_str).await?;
        let mut processed = 0;

        for item in items {
            let target_channel: Option<&str> = item
                .thread_id
                .as_deref()
                .or(self.config.channel_id.as_deref());
            let channel_id: &str = match target_channel {
                Some(ch) if !ch.trim().is_empty() => ch.trim(),
                _ => {
                    tracing::warn!(
                        work_item_id = %item.id,
                        "no thread_id or heartbeat alert channel configured; skipping heartbeat"
                    );
                    continue;
                }
            };

            let liveness = match item.delegate_pane.as_deref() {
                Some(pane) => self.checker.check_liveness(pane).await,
                None => DelegateLiveness::Unknown,
            };

            let content = match liveness {
                DelegateLiveness::Dead => {
                    format!(
                        "⚠️ Worker dead for work item '{}' ({}) in delegate pane {}",
                        item.title,
                        item.id,
                        item.delegate_pane.as_deref().unwrap_or("unknown")
                    )
                }
                _ => {
                    format!(
                        "⏳ Work item in progress: {} ({})\nStatus: in_progress | Pane: {} | Last progress: {}",
                        item.title,
                        item.id,
                        item.delegate_pane.as_deref().unwrap_or("none"),
                        item.last_progress_at
                    )
                }
            };

            // One bad channel/thread must not starve every later stale item.
            let content: String = content.chars().take(1900).collect();
            if let Err(error) = self.sink.post_message(channel_id, &content).await {
                tracing::warn!(work_item_id = %item.id, %error, "work heartbeat post failed; continuing");
                continue;
            }
            let next_progress_at = now.to_rfc3339();
            if let Err(error) =
                update_work_item_progress(&self.pool, &item.id, &next_progress_at).await
            {
                tracing::warn!(work_item_id = %item.id, %error, "work heartbeat progress update failed; continuing");
                continue;
            }
            processed += 1;
        }

        Ok(processed)
    }

    pub fn spawn(
        self,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let interval_duration = self.config.interval;
            let mut ticker = tokio::time::interval(interval_duration);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Err(error) = self.step().await {
                            tracing::error!(%error, "work heartbeat runner step failed");
                        }
                    }
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        })
    }
}

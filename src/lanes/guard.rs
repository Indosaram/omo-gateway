use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fs2::FileExt;
use sqlx::SqlitePool;
use thiserror::Error;

pub const DEFAULT_LOAD_ALERT: f64 = 150.0;
pub const DEFAULT_LOAD_OK: f64 = 80.0;
pub const DEFAULT_MEM_FREE_MIN_PCT: u32 = 20;
pub const DEFAULT_DISK_FREE_MIN_GB: u64 = 50;
pub const DEFAULT_LANES_MAX: u64 = 6;
pub const DEFAULT_HEAVY_SLOTS: usize = 1;
pub const DEFAULT_METRIC_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum GuardError {
    #[error("command execution failed: {0}")]
    Command(String),
    #[error("command timed out: {0}")]
    Timeout(String),
    #[error("metric parse failed: {0}")]
    Parse(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardThresholds {
    pub load_alert: f64,
    pub load_ok: f64,
    pub mem_free_min_pct: u32,
    pub disk_free_min_gb: u64,
    pub lanes_max: u64,
}

impl Default for GuardThresholds {
    fn default() -> Self {
        Self::from_env()
    }
}

impl GuardThresholds {
    pub fn from_env() -> Self {
        Self {
            load_alert: std::env::var("OMON_GUARD_LOAD_ALERT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LOAD_ALERT),
            load_ok: std::env::var("OMON_GUARD_LOAD_OK")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LOAD_OK),
            mem_free_min_pct: std::env::var("OMON_GUARD_MEM_FREE_MIN_PCT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_MEM_FREE_MIN_PCT),
            disk_free_min_gb: std::env::var("OMON_GUARD_DISK_FREE_MIN_GB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_DISK_FREE_MIN_GB),
            lanes_max: std::env::var("OMON_LANES_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_LANES_MAX),
        }
    }
}

#[async_trait]
pub trait ActiveLaneCounter: Send + Sync {
    async fn count_active_lanes(&self) -> Result<u64, GuardError>;
}

#[derive(Debug, Clone, Default)]
pub struct FakeActiveLaneCounter {
    pub count: u64,
}

impl FakeActiveLaneCounter {
    pub fn new(count: u64) -> Self {
        Self { count }
    }

    pub fn set(&mut self, count: u64) {
        self.count = count;
    }
}

#[async_trait]
impl ActiveLaneCounter for FakeActiveLaneCounter {
    async fn count_active_lanes(&self) -> Result<u64, GuardError> {
        Ok(self.count)
    }
}

pub async fn query_active_lanes_sql(pool: &SqlitePool) -> Result<u64, GuardError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lanes WHERE status IN ('working','blocked','not_done','done_claimed')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| GuardError::Database(e.to_string()))?;

    Ok(count.max(0) as u64)
}

#[async_trait]
impl ActiveLaneCounter for SqlitePool {
    async fn count_active_lanes(&self) -> Result<u64, GuardError> {
        query_active_lanes_sql(self).await
    }
}

#[async_trait]
impl ActiveLaneCounter for crate::storage::Database {
    async fn count_active_lanes(&self) -> Result<u64, GuardError> {
        query_active_lanes_sql(self.pool()).await
    }
}

#[async_trait]
impl<T: ActiveLaneCounter + ?Sized> ActiveLaneCounter for &T {
    async fn count_active_lanes(&self) -> Result<u64, GuardError> {
        (**self).count_active_lanes().await
    }
}

#[async_trait]
impl<T: ActiveLaneCounter + ?Sized> ActiveLaneCounter for Arc<T> {
    async fn count_active_lanes(&self) -> Result<u64, GuardError> {
        (**self).count_active_lanes().await
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostSample {
    pub load_1m: f64,
    pub mem_free_pct: u32,
    pub disk_free_gb: u64,
    pub active_lanes: u64,
}

impl HostSample {
    pub fn new(load_1m: f64, mem_free_pct: u32, disk_free_gb: u64, active_lanes: u64) -> Self {
        Self {
            load_1m,
            mem_free_pct,
            disk_free_gb,
            active_lanes,
        }
    }

    pub fn can_launch(&self, thresholds: &GuardThresholds) -> LaunchDecision {
        let mut reasons = Vec::new();

        if self.mem_free_pct < thresholds.mem_free_min_pct {
            reasons.push(format!(
                "memory free ({}%) is below minimum ({}%)",
                self.mem_free_pct, thresholds.mem_free_min_pct
            ));
        }

        if self.disk_free_gb < thresholds.disk_free_min_gb {
            reasons.push(format!(
                "disk free ({} GB) is below minimum ({} GB)",
                self.disk_free_gb, thresholds.disk_free_min_gb
            ));
        }

        if self.active_lanes >= thresholds.lanes_max {
            reasons.push(format!(
                "active lanes ({}) reached maximum limit ({})",
                self.active_lanes, thresholds.lanes_max
            ));
        }

        if reasons.is_empty() {
            LaunchDecision::Ok
        } else {
            LaunchDecision::Hold(reasons)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchDecision {
    Ok,
    Hold(Vec<String>),
}

impl LaunchDecision {
    pub fn is_ok(&self) -> bool {
        matches!(self, LaunchDecision::Ok)
    }

    pub fn is_hold(&self) -> bool {
        matches!(self, LaunchDecision::Hold(_))
    }

    pub fn reasons(&self) -> &[String] {
        match self {
            LaunchDecision::Ok => &[],
            LaunchDecision::Hold(reasons) => reasons,
        }
    }

    pub fn has_reason(&self, keyword: &str) -> bool {
        match self {
            LaunchDecision::Ok => false,
            LaunchDecision::Hold(reasons) => {
                let lower = keyword.to_lowercase();
                reasons.iter().any(|r| r.to_lowercase().contains(&lower))
            }
        }
    }
}

impl std::fmt::Display for LaunchDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchDecision::Ok => write!(f, "Ok"),
            LaunchDecision::Hold(reasons) => write!(f, "Hold: {}", reasons.join("; ")),
        }
    }
}

pub fn parse_sysctl_loadavg(text: &str) -> Result<f64, GuardError> {
    let cleaned = text.trim().trim_start_matches('{').trim_end_matches('}');
    let first = cleaned
        .split_whitespace()
        .next()
        .ok_or_else(|| GuardError::Parse("empty sysctl vm.loadavg output".to_string()))?;
    first
        .parse::<f64>()
        .map_err(|e| GuardError::Parse(format!("failed to parse loadavg '{first}': {e}")))
}

pub async fn wait_child_with_timeout(
    mut child: tokio::process::Child,
    timeout_dur: Duration,
    description: &str,
) -> Result<std::process::Output, GuardError> {
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    let read_and_wait = async {
        let read_stdout = async {
            if let Some(ref mut pipe) = stdout_pipe {
                tokio::io::AsyncReadExt::read_to_end(pipe, &mut stdout_buf).await
            } else {
                Ok(0)
            }
        };
        let read_stderr = async {
            if let Some(ref mut pipe) = stderr_pipe {
                tokio::io::AsyncReadExt::read_to_end(pipe, &mut stderr_buf).await
            } else {
                Ok(0)
            }
        };
        let (status_res, out_res, err_res) = tokio::join!(child.wait(), read_stdout, read_stderr);
        let status = status_res?;
        out_res?;
        err_res?;
        Ok::<_, std::io::Error>(std::process::Output {
            status,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    };

    match tokio::time::timeout(timeout_dur, read_and_wait).await {
        Ok(res) => {
            res.map_err(|e| GuardError::Command(format!("failed to run {description}: {e}")))
        }
        Err(_) => {
            // Deadline reached: kill child and await exit to reap the process
            if let Err(e) = child.start_kill() {
                tracing::warn!(description, "failed to send kill to timed out child: {e}");
            }
            let _ = child.wait().await;
            Err(GuardError::Timeout(format!(
                "{description} timed out after {timeout_dur:?}"
            )))
        }
    }
}

pub async fn run_command_with_timeout(
    mut cmd: tokio::process::Command,
    timeout_dur: Duration,
    description: &str,
) -> Result<std::process::Output, GuardError> {
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    let child = cmd
        .spawn()
        .map_err(|e| GuardError::Command(format!("failed to spawn {description}: {e}")))?;

    wait_child_with_timeout(child, timeout_dur, description).await
}

pub async fn sample_load_1m() -> Result<f64, GuardError> {
    sample_load_1m_with_timeout(DEFAULT_METRIC_TIMEOUT).await
}

pub async fn sample_load_1m_with_timeout(timeout_dur: Duration) -> Result<f64, GuardError> {
    let mut cmd = tokio::process::Command::new("sysctl");
    cmd.arg("-n").arg("vm.loadavg");

    let output = run_command_with_timeout(cmd, timeout_dur, "sysctl vm.loadavg").await?;
    if !output.status.success() {
        return Err(GuardError::Command(format!(
            "sysctl exited with status {}",
            output.status
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_sysctl_loadavg(&text)
}

pub fn parse_memory_pressure(text: &str) -> Result<u32, GuardError> {
    for line in text.lines() {
        if line.contains("System-wide memory free percentage:") {
            if let Some(pos) = line.find(':') {
                let rest = line[pos + 1..].trim();
                let pct_str = rest.trim_end_matches('%').trim();
                return pct_str.parse::<u32>().map_err(|e| {
                    GuardError::Parse(format!(
                        "failed to parse memory percentage '{pct_str}': {e}"
                    ))
                });
            }
        }
    }
    Err(GuardError::Parse(
        "no 'System-wide memory free percentage:' line found".to_string(),
    ))
}

pub async fn sample_mem_free_pct() -> Result<u32, GuardError> {
    sample_mem_free_pct_with_timeout(DEFAULT_METRIC_TIMEOUT).await
}

pub async fn sample_mem_free_pct_with_timeout(timeout_dur: Duration) -> Result<u32, GuardError> {
    let cmd = tokio::process::Command::new("memory_pressure");
    let output_res = run_command_with_timeout(cmd, timeout_dur, "memory_pressure").await;
    let output = match output_res {
        Ok(out) => out,
        Err(GuardError::Command(_)) => {
            let fallback_cmd = tokio::process::Command::new("/usr/bin/memory_pressure");
            run_command_with_timeout(fallback_cmd, timeout_dur, "/usr/bin/memory_pressure").await?
        }
        Err(e) => return Err(e),
    };

    if !output.status.success() {
        return Err(GuardError::Command(format!(
            "memory_pressure exited with status {}",
            output.status
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_memory_pressure(&text)
}

pub fn parse_df_k(text: &str) -> Result<u64, GuardError> {
    let mut lines = text.lines();
    let _header = lines
        .next()
        .ok_or_else(|| GuardError::Parse("df output empty".to_string()))?;
    let data_line = lines
        .next()
        .ok_or_else(|| GuardError::Parse("df output missing data line".to_string()))?;
    let fields: Vec<&str> = data_line.split_whitespace().collect();
    if fields.len() < 4 {
        return Err(GuardError::Parse(format!(
            "df data line has < 4 fields: {data_line}"
        )));
    }
    let avail_kib = fields[3].parse::<u64>().map_err(|e| {
        GuardError::Parse(format!(
            "failed to parse df available kib '{}': {e}",
            fields[3]
        ))
    })?;

    Ok(avail_kib / (1024 * 1024))
}

pub async fn sample_disk_free_gb(mount_point: &str) -> Result<u64, GuardError> {
    sample_disk_free_gb_with_timeout(mount_point, DEFAULT_METRIC_TIMEOUT).await
}

pub async fn sample_disk_free_gb_with_timeout(
    mount_point: &str,
    timeout_dur: Duration,
) -> Result<u64, GuardError> {
    let mut cmd = tokio::process::Command::new("df");
    cmd.arg("-k").arg(mount_point);

    let output = run_command_with_timeout(cmd, timeout_dur, "df -k").await?;
    if !output.status.success() {
        return Err(GuardError::Command(format!(
            "df exited with status {}",
            output.status
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_df_k(&text)
}

pub async fn sample(
    counter: &(dyn ActiveLaneCounter + Send + Sync),
) -> Result<HostSample, GuardError> {
    let load_1m = sample_load_1m().await?;
    let mem_free_pct = sample_mem_free_pct().await?;
    let disk_free_gb = sample_disk_free_gb("/").await?;
    let active_lanes = counter.count_active_lanes().await?;
    Ok(HostSample {
        load_1m,
        mem_free_pct,
        disk_free_gb,
        active_lanes,
    })
}

pub async fn can_launch(
    counter: &(dyn ActiveLaneCounter + Send + Sync),
) -> Result<LaunchDecision, GuardError> {
    let thresholds = GuardThresholds::from_env();
    can_launch_with_thresholds(counter, &thresholds).await
}

pub async fn can_launch_with_thresholds(
    counter: &(dyn ActiveLaneCounter + Send + Sync),
    thresholds: &GuardThresholds,
) -> Result<LaunchDecision, GuardError> {
    let s = sample(counter).await?;
    Ok(s.can_launch(thresholds))
}

pub fn can_launch_sample(sample: &HostSample, thresholds: &GuardThresholds) -> LaunchDecision {
    sample.can_launch(thresholds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardState {
    Clear,
    Alert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardReport {
    Alert(String),
    Clear,
}

impl GuardReport {
    pub fn is_alert(&self) -> bool {
        matches!(self, GuardReport::Alert(_))
    }

    pub fn is_clear(&self) -> bool {
        matches!(self, GuardReport::Clear)
    }

    pub fn alert_message(&self) -> Option<&str> {
        match self {
            GuardReport::Alert(s) => Some(s.as_str()),
            GuardReport::Clear => None,
        }
    }
}

/// Within this window a repeated ALERT is suppressed unless the host got materially worse.
pub const GUARD_REALERT_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);
/// Disk must drop at least this many GB below the last alerted value to re-alert early.
pub const GUARD_REALERT_DISK_DROP_GB: u64 = 10;
/// Free memory must drop at least this many percentage points to re-alert early.
pub const GUARD_REALERT_MEM_DROP_PCT: u32 = 10;

/// Snapshot of the host at the last emitted ALERT.
#[derive(Debug, Clone, Copy)]
struct AlertMark {
    at: std::time::Instant,
    load: bool,
    mem: bool,
    disk: bool,
    mem_free_pct: u32,
    disk_free_gb: u64,
}

#[derive(Clone)]
pub struct GuardWatcher {
    thresholds: GuardThresholds,
    current_state: GuardState,
    last_alert: Option<AlertMark>,
    /// Whether the current Alert episode emitted an ALERT (so its CLEAR is worth sending).
    episode_reported: bool,
}

impl Default for GuardWatcher {
    fn default() -> Self {
        Self::new(GuardThresholds::from_env())
    }
}

impl GuardWatcher {
    pub fn new(thresholds: GuardThresholds) -> Self {
        Self {
            thresholds,
            current_state: GuardState::Clear,
            last_alert: None,
            episode_reported: false,
        }
    }

    pub fn state(&self) -> GuardState {
        self.current_state
    }

    pub fn thresholds(&self) -> &GuardThresholds {
        &self.thresholds
    }

    pub fn check(&mut self, sample: &HostSample) -> Option<GuardReport> {
        self.check_at(sample, std::time::Instant::now())
    }

    /// True when this sample deserves a fresh ALERT given the last emitted one.
    fn worth_alerting(
        &self,
        load: bool,
        mem: bool,
        disk: bool,
        sample: &HostSample,
        now: std::time::Instant,
    ) -> bool {
        let Some(mark) = self.last_alert else {
            return true;
        };
        if now.saturating_duration_since(mark.at) >= GUARD_REALERT_WINDOW {
            return true;
        }
        self.got_worse(load, mem, disk, sample)
    }

    /// The host is materially worse than at the last emitted ALERT, or a new metric started
    /// alerting. Elapsed time plays no part, so a long episode still reports real changes.
    fn got_worse(&self, load: bool, mem: bool, disk: bool, sample: &HostSample) -> bool {
        let Some(mark) = self.last_alert else {
            return true;
        };
        // A metric that was not part of the last alert is new information.
        if (load && !mark.load) || (mem && !mark.mem) || (disk && !mark.disk) {
            return true;
        }
        let disk_worse = disk
            && sample
                .disk_free_gb
                .saturating_add(GUARD_REALERT_DISK_DROP_GB)
                <= mark.disk_free_gb;
        let mem_worse = mem
            && sample
                .mem_free_pct
                .saturating_add(GUARD_REALERT_MEM_DROP_PCT)
                <= mark.mem_free_pct;
        disk_worse || mem_worse
    }

    fn mark_alert(
        &mut self,
        load: bool,
        mem: bool,
        disk: bool,
        sample: &HostSample,
        now: std::time::Instant,
    ) {
        self.last_alert = Some(AlertMark {
            at: now,
            load,
            mem,
            disk,
            mem_free_pct: sample.mem_free_pct,
            disk_free_gb: sample.disk_free_gb,
        });
        self.episode_reported = true;
    }

    /// Deduplicating state machine. While in Alert nothing is emitted unless disk/mem dropped
    /// by the re-alert margins (or a new metric alerts) since the last emitted ALERT. A
    /// Clear->Alert flip within [`GUARD_REALERT_WINDOW`] of the last ALERT is suppressed under
    /// the same rule; CLEAR is sent only for episodes whose ALERT was actually emitted, so
    /// flapping around a threshold yields one ALERT and one CLEAR.
    pub fn check_at(
        &mut self,
        sample: &HostSample,
        now: std::time::Instant,
    ) -> Option<GuardReport> {
        let mut alert_reasons = Vec::new();
        let load_alert = sample.load_1m >= self.thresholds.load_alert;
        let mem_alert = sample.mem_free_pct < self.thresholds.mem_free_min_pct;
        let disk_alert = sample.disk_free_gb < self.thresholds.disk_free_min_gb;

        if load_alert {
            alert_reasons.push(format!(
                "load average ({:.2}) >= alert threshold ({:.2})",
                sample.load_1m, self.thresholds.load_alert
            ));
        }
        if sample.mem_free_pct < self.thresholds.mem_free_min_pct {
            alert_reasons.push(format!(
                "memory free ({}%) < minimum ({}%)",
                sample.mem_free_pct, self.thresholds.mem_free_min_pct
            ));
        }
        if sample.disk_free_gb < self.thresholds.disk_free_min_gb {
            alert_reasons.push(format!(
                "disk free ({} GB) < minimum ({} GB)",
                sample.disk_free_gb, self.thresholds.disk_free_min_gb
            ));
        }

        match self.current_state {
            GuardState::Clear => {
                if alert_reasons.is_empty() {
                    return None;
                }
                self.current_state = GuardState::Alert;
                self.episode_reported = false;
                if self.worth_alerting(load_alert, mem_alert, disk_alert, sample, now) {
                    self.mark_alert(load_alert, mem_alert, disk_alert, sample, now);
                    Some(GuardReport::Alert(alert_reasons.join("; ")))
                } else {
                    None
                }
            }
            GuardState::Alert => {
                let load_cleared = sample.load_1m < self.thresholds.load_ok;
                let mem_cleared = sample.mem_free_pct >= self.thresholds.mem_free_min_pct;
                let disk_cleared = sample.disk_free_gb >= self.thresholds.disk_free_min_gb;

                if load_cleared && mem_cleared && disk_cleared {
                    self.current_state = GuardState::Clear;
                    let reported = std::mem::replace(&mut self.episode_reported, false);
                    return reported.then_some(GuardReport::Clear);
                }
                // Still alerting: quiet unless the host got materially worse (or a new metric
                // started alerting) since the last emitted ALERT. Time alone never re-alerts here.
                if !alert_reasons.is_empty()
                    && self.last_alert.is_some()
                    && self.got_worse(load_alert, mem_alert, disk_alert, sample)
                {
                    self.mark_alert(load_alert, mem_alert, disk_alert, sample, now);
                    return Some(GuardReport::Alert(alert_reasons.join("; ")));
                }
                None
            }
        }
    }

    pub fn update(&mut self, sample: &HostSample) -> Option<GuardReport> {
        self.check(sample)
    }

    /// Advance the watcher only after its transition event is durably queued.
    pub async fn enqueue_transition(
        &mut self,
        pool: &SqlitePool,
        lead: &str,
        sample: &HostSample,
    ) -> crate::Result<()> {
        let mut next = self.clone();
        if let Some(report) = next.check(sample) {
            let body = match report {
                GuardReport::Alert(reason) => format!("ALERT: {reason}"),
                GuardReport::Clear => "CLEAR".into(),
            };
            crate::storage::enqueue_lane_event(pool, None, lead, "guard", &super::scrub(&body))
                .await?;
        }
        // Suppressed transitions still advance the state machine.
        *self = next;
        Ok(())
    }
}

pub const GUARD_INTERVAL: Duration = Duration::from_secs(60);
const LAUNCH_DISCORD_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a failed queued-lane launch waits before the next attempt.
pub const LAUNCH_RETRY_BACKOFF: Duration = Duration::from_secs(10 * 60);

async fn active_lane_count<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Sqlite>,
) -> crate::Result<u64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lanes WHERE status IN ('working','blocked','not_done','done_claimed')",
    )
    .fetch_one(executor)
    .await?;
    u64::try_from(count).map_err(|error| crate::OmonError::Config(error.to_string()))
}

/// Creates the lane thread with no DB connection held; returns (bot id, thread ref).
/// A Slack parent (`slack:C…` or `C…`) roots a Slack thread; a numeric parent a Discord one.
pub async fn create_lane_thread(
    lane: &crate::storage::Lane,
    parent: &str,
) -> Result<(String, String), String> {
    if super::slack_threads::parse_parent(parent).is_some() {
        let threads = super::slack_threads::SlackLaneThreads::from_env()
            .ok_or_else(|| "Slack is not configured".to_string())?;
        let bot = tokio::time::timeout(LAUNCH_DISCORD_TIMEOUT, threads.bot_user_id())
            .await
            .map_err(|_| "slack bot lookup timed out".to_string())?
            .map_err(|error| error.to_string())?;
        let thread = tokio::time::timeout(
            LAUNCH_DISCORD_TIMEOUT,
            threads.create_thread(parent, &lane.key, &lane.title),
        )
        .await
        .map_err(|_| "slack thread creation timed out".to_string())?
        .map_err(|error| error.to_string())?;
        return Ok((bot, thread));
    }
    let parent = parent.parse::<u64>().map_err(|error| error.to_string())?;
    let threads =
        super::discord_threads::DiscordThreads::from_env().map_err(|error| error.to_string())?;
    let bot = tokio::time::timeout(LAUNCH_DISCORD_TIMEOUT, threads.http().get_current_user())
        .await
        .map_err(|_| "discord bot lookup timed out".to_string())?
        .map_err(|error| error.to_string())?
        .id
        .to_string();
    let created = tokio::time::timeout(
        LAUNCH_DISCORD_TIMEOUT,
        threads.create_thread(
            serenity::all::ChannelId::new(parent),
            &lane.key,
            &lane.title,
        ),
    )
    .await
    .map_err(|_| "discord thread creation timed out".to_string())?
    .map_err(|error| error.to_string())?;
    Ok((bot, created.thread_channel_id.to_string()))
}

/// Best effort: the lane service must not stall on a transport failure here.
/// Discord archives the orphan thread; Slack deletes its starter message.
pub async fn discard_orphan_thread(thread: &str) {
    if super::slack_threads::is_slack_ref(thread) {
        let Some(threads) = super::slack_threads::SlackLaneThreads::from_env() else {
            return;
        };
        match tokio::time::timeout(LAUNCH_DISCORD_TIMEOUT, threads.delete_thread(thread)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(thread, %error, "orphan lane thread delete failed"),
            Err(_) => tracing::warn!(thread, "orphan lane thread delete timed out"),
        }
        return;
    }
    let Ok(id) = thread.parse::<u64>() else {
        return;
    };
    let Ok(threads) = super::discord_threads::DiscordThreads::from_env() else {
        return;
    };
    match tokio::time::timeout(
        LAUNCH_DISCORD_TIMEOUT,
        threads.archive(serenity::all::ChannelId::new(id)),
    )
    .await
    {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => tracing::warn!(thread, %error, "orphan lane thread archive failed"),
        Err(_) => tracing::warn!(thread, "orphan lane thread archive timed out"),
    }
}

async fn record_launch_problem(
    pool: &SqlitePool,
    lane: &crate::storage::Lane,
    kind: &str,
    detail: &str,
) -> crate::Result<()> {
    tracing::warn!(key = %lane.key, kind, detail, "queued lane launch problem");
    crate::storage::enqueue_lane_event(
        pool,
        Some(&lane.key),
        &lane.lead_session_key,
        kind,
        &super::scrub(&format!("{}: {detail}", lane.key)),
    )
    .await?;
    Ok(())
}

/// Promote queued lanes atomically with their history and launch event.
/// The write lock makes the active-lane limit authoritative across callers.
pub async fn retry_queued(
    pool: &SqlitePool,
    host: &HostSample,
    thresholds: &GuardThresholds,
) -> crate::Result<Vec<String>> {
    let mut launched = Vec::new();
    let mut lanes = crate::storage::list_lanes(pool, &["queued"]).await?;
    // list_lanes is newest-first; the longest-waiting lane gets the free slot.
    lanes.reverse();
    for lane in lanes {
        // A failed launch is not retried (no repeated starter posts) until the lane row changes
        // or the backoff passes, so a transient Discord error cannot strand the lane in `queued`.
        let failed_since_change: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM lane_events WHERE lane_key = ? AND kind = 'launch_failed' AND julianday(created_at) >= julianday(?) AND julianday(created_at) > julianday('now') - ?)",
        )
        .bind(&lane.key)
        .bind(lane.updated_at)
        .bind(LAUNCH_RETRY_BACKOFF.as_secs_f64() / 86_400.0)
        .fetch_one(pool)
        .await?;
        if failed_since_change {
            continue;
        }
        let mut current = host.clone();
        current.active_lanes = active_lane_count(pool).await?;
        if current.can_launch(thresholds).is_hold() {
            break;
        }
        let mut bot = None;
        let thread = match (&lane.parent_channel_id, &lane.thread_channel_id) {
            (Some(parent), None) => match create_lane_thread(&lane, parent).await {
                Ok((bot_id, thread)) => {
                    bot = Some(bot_id);
                    Some(thread)
                }
                Err(error) => {
                    record_launch_problem(pool, &lane, "launch_failed", &error).await?;
                    continue;
                }
            },
            _ => lane.thread_channel_id.clone(),
        };
        let mut tx = pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE lanes SET updated_at = updated_at WHERE key = ? AND status = 'queued'",
        )
        .bind(&lane.key)
        .execute(&mut *tx)
        .await?;
        current.active_lanes = active_lane_count(&mut *tx).await?;
        if changed.rows_affected() == 0 || current.can_launch(thresholds).is_hold() {
            tx.rollback().await?;
            if bot.is_some() {
                let detail = format!(
                    "thread {} created but lane was no longer launchable",
                    thread.as_deref().unwrap_or_default()
                );
                record_launch_problem(pool, &lane, "launch_orphan_thread", &detail).await?;
                // The thread id is not persisted, so the next tick creates another
                // one: archive this orphan instead of leaving one per tick behind.
                if let Some(thread) = thread.as_deref() {
                    discard_orphan_thread(thread).await;
                }
            }
            if changed.rows_affected() == 0 {
                continue;
            }
            break;
        }
        let now = chrono::Utc::now();
        sqlx::query("UPDATE lanes SET status = 'working', thread_channel_id = ?, updated_at = ?, last_activity_at = ? WHERE key = ? AND status = 'queued'")
            .bind(thread).bind(now).bind(now).bind(&lane.key).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'queued', 'working', 'host guard cleared')")
            .bind(&lane.key).bind(now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, 'launched', ?, ?)")
            .bind(&lane.key).bind(&lane.lead_session_key)
            .bind(super::scrub(&format!("{}: queued lane launched", lane.key)))
            .bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        if let Some(bot) = bot {
            if let Err(error) = crate::storage::lanes::bind_lane_thread(pool, &lane.key, &bot).await
            {
                record_launch_problem(pool, &lane, "launch_bind_failed", &error.to_string())
                    .await?;
            }
        }
        launched.push(lane.key);
    }
    Ok(launched)
}

pub fn default_heavy_slots_dir() -> Result<PathBuf, GuardError> {
    let home = std::env::var("HOME").map(PathBuf::from).map_err(|_| {
        GuardError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "HOME environment variable not set",
        ))
    })?;
    Ok(home.join(".omon").join("state").join("heavy"))
}

#[cfg(unix)]
pub fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe {
        let res = libc::kill(pid as libc::pid_t, 0);
        if res == 0 {
            true
        } else {
            let err = std::io::Error::last_os_error();
            err.raw_os_error() == Some(libc::EPERM)
        }
    }
}

#[cfg(not(unix))]
pub fn is_pid_alive(_pid: u32) -> bool {
    true
}

pub fn read_slot_pid(slot_path: &Path) -> Option<u32> {
    std::fs::read_to_string(slot_path)
        .ok()
        .and_then(|content| content.trim().parse::<u32>().ok())
}

pub struct HeavySlotGuard {
    slot_index: usize,
    slot_path: PathBuf,
    file: Option<std::fs::File>,
}

impl HeavySlotGuard {
    pub fn slot_index(&self) -> usize {
        self.slot_index
    }

    pub fn slot_path(&self) -> &Path {
        &self.slot_path
    }
}

impl Drop for HeavySlotGuard {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            // Explicitly unlock the flock before closing descriptor.
            // Keep the PID for stale-owner diagnostics; never unlink the lock inode.
            // fs2's flock unlock; std's inherent File::unlock needs Rust 1.89 (MSRV is 1.88).
            let _ = FileExt::unlock(&file);
            drop(file);
        }
    }
}

pub async fn try_acquire_heavy_slot(
    slots_dir: Option<&Path>,
    max_slots: Option<usize>,
) -> Result<Option<HeavySlotGuard>, GuardError> {
    let dir = match slots_dir {
        Some(d) => d.to_path_buf(),
        None => default_heavy_slots_dir()?,
    };
    tokio::fs::create_dir_all(&dir).await?;

    // Zero slots could never be acquired and would hang `acquire_heavy_slot`.
    let slots_count = max_slots
        .unwrap_or_else(|| {
            std::env::var("OMON_HEAVY_SLOTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_HEAVY_SLOTS)
        })
        .max(1);

    for slot_idx in 0..slots_count {
        let slot_path = dir.join(format!("slot.{}", slot_idx));
        let path_clone = slot_path.clone();

        let acquired = tokio::task::spawn_blocking(
            move || -> Result<Option<std::fs::File>, std::io::Error> {
                use std::io::{Read, Seek, SeekFrom, Write};

                let mut file = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path_clone)?;

                match file.try_lock_exclusive() {
                    Ok(()) => {
                        let mut content = String::new();
                        file.read_to_string(&mut content)?;
                        if let Ok(stale_pid) = content.trim().parse::<u32>() {
                            if !is_pid_alive(stale_pid) {
                                tracing::info!(
                                    slot = slot_idx,
                                    stale_pid,
                                    "taking over stale heavy slot from dead process"
                                );
                            }
                        }

                        file.seek(SeekFrom::Start(0))?;
                        file.set_len(0)?;
                        writeln!(file, "{}", std::process::id())?;
                        file.flush()?;

                        Ok(Some(file))
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock
                            || err.raw_os_error() == Some(libc::EWOULDBLOCK)
                            || err.raw_os_error() == Some(libc::EAGAIN)
                        {
                            Ok(None)
                        } else {
                            Err(err)
                        }
                    }
                }
            },
        )
        .await
        .map_err(|e| GuardError::Internal(e.to_string()))??;

        if let Some(file) = acquired {
            return Ok(Some(HeavySlotGuard {
                slot_index: slot_idx,
                slot_path,
                file: Some(file),
            }));
        }
    }

    Ok(None)
}

pub async fn acquire_heavy_slot(
    slots_dir: Option<&Path>,
    max_slots: Option<usize>,
    poll_interval: Duration,
) -> Result<HeavySlotGuard, GuardError> {
    loop {
        if let Some(guard) = try_acquire_heavy_slot(slots_dir, max_slots).await? {
            return Ok(guard);
        }
        tokio::time::sleep(poll_interval).await;
    }
}

pub async fn wait_for_load_ok(load_ok: f64, poll_interval: Duration) -> Result<f64, GuardError> {
    loop {
        let load = sample_load_1m().await?;
        if load < load_ok {
            return Ok(load);
        }
        tokio::time::sleep(poll_interval).await;
    }
}

pub async fn run_with_heavy_slot<F, Fut, T>(
    label: &str,
    slots_dir: Option<&Path>,
    thresholds: &GuardThresholds,
    work: F,
) -> Result<T, GuardError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _guard = acquire_heavy_slot(slots_dir, None, Duration::from_millis(500)).await?;
    let _load = wait_for_load_ok(thresholds.load_ok, Duration::from_millis(500)).await?;
    tracing::info!(label, "starting heavy slot execution");
    Ok(work().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_launch_hold_at_mem_10_pct() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let sample = HostSample::new(10.0, 10, 100, 1);
        let decision = sample.can_launch(&thresholds);
        assert!(decision.is_hold(), "expected Hold for mem 10%");
        assert!(
            decision.has_reason("memory"),
            "expected memory reason, got: {:?}",
            decision.reasons()
        );
    }

    #[test]
    fn test_can_launch_hold_at_disk_10_gb() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let sample = HostSample::new(10.0, 50, 10, 1);
        let decision = sample.can_launch(&thresholds);
        assert!(decision.is_hold(), "expected Hold for disk 10 GB");
        assert!(
            decision.has_reason("disk"),
            "expected disk reason, got: {:?}",
            decision.reasons()
        );
    }

    #[test]
    fn test_can_launch_hold_at_7_active_lanes() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let sample = HostSample::new(10.0, 50, 100, 7);
        let decision = sample.can_launch(&thresholds);
        assert!(decision.is_hold(), "expected Hold for 7 active lanes");
        assert!(
            decision.has_reason("lanes"),
            "expected lanes reason, got: {:?}",
            decision.reasons()
        );
    }

    #[test]
    fn test_can_launch_no_hold_at_load_300_alone() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let sample = HostSample::new(300.0, 50, 100, 1);
        let decision = sample.can_launch(&thresholds);
        assert!(
            decision.is_ok(),
            "expected Ok because CPU load alone never holds launch, got: {:?}",
            decision
        );
    }

    #[test]
    fn test_guard_watcher_transition_only_reporting() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let mut watcher = GuardWatcher::new(thresholds);
        assert_eq!(watcher.state(), GuardState::Clear);

        let s1 = HostSample::new(50.0, 50, 100, 0);
        assert_eq!(watcher.check(&s1), None);
        assert_eq!(watcher.state(), GuardState::Clear);

        let s2 = HostSample::new(160.0, 50, 100, 0);
        let rep2 = watcher.check(&s2);
        assert!(rep2.is_some(), "expected report on transition to Alert");
        assert!(rep2.unwrap().is_alert());
        assert_eq!(watcher.state(), GuardState::Alert);

        let s3 = HostSample::new(170.0, 50, 100, 0);
        assert_eq!(watcher.check(&s3), None);
        assert_eq!(watcher.state(), GuardState::Alert);

        let s4 = HostSample::new(100.0, 50, 100, 0);
        assert_eq!(watcher.check(&s4), None);
        assert_eq!(watcher.state(), GuardState::Alert);

        let s5 = HostSample::new(75.0, 50, 100, 0);
        let rep5 = watcher.check(&s5);
        assert_eq!(rep5, Some(GuardReport::Clear));
        assert_eq!(watcher.state(), GuardState::Clear);

        let s6 = HostSample::new(50.0, 50, 100, 0);
        assert_eq!(watcher.check(&s6), None);
        assert_eq!(watcher.state(), GuardState::Clear);
    }

    #[test]
    fn test_guard_watcher_memory_and_disk_alerts() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let mut watcher = GuardWatcher::new(thresholds);

        let s_low_mem = HostSample::new(50.0, 10, 100, 0);
        let rep = watcher.check(&s_low_mem);
        assert!(rep.is_some() && rep.unwrap().is_alert());
        assert_eq!(watcher.state(), GuardState::Alert);

        let s_ok_mem = HostSample::new(50.0, 30, 100, 0);
        let rep_clear = watcher.check(&s_ok_mem);
        assert_eq!(rep_clear, Some(GuardReport::Clear));
        assert_eq!(watcher.state(), GuardState::Clear);

        let s_low_disk = HostSample::new(50.0, 50, 20, 0);
        let rep_disk = watcher.check(&s_low_disk);
        assert!(rep_disk.is_some() && rep_disk.unwrap().is_alert());
        assert_eq!(watcher.state(), GuardState::Alert);

        let s_ok_disk = HostSample::new(50.0, 50, 80, 0);
        let rep_disk_clear = watcher.check(&s_ok_disk);
        assert_eq!(rep_disk_clear, Some(GuardReport::Clear));
        assert_eq!(watcher.state(), GuardState::Clear);
    }

    #[test]
    fn guard_disk_flapping_within_hour_alerts_once_until_material_drop() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let mut watcher = GuardWatcher::new(thresholds);
        let t0 = std::time::Instant::now();
        let mut alerts = 0;
        let mut clears = 0;
        for (minute, gb) in [48u64, 51, 47, 52, 46].into_iter().enumerate() {
            let now = t0 + Duration::from_secs(minute as u64 * 10 * 60);
            match watcher.check_at(&HostSample::new(10.0, 50, gb, 0), now) {
                Some(GuardReport::Alert(_)) => alerts += 1,
                Some(GuardReport::Clear) => clears += 1,
                None => {}
            }
        }
        assert_eq!(alerts, 1, "flapping must alert exactly once");
        assert_eq!(clears, 1, "only the reported episode may CLEAR");
        assert_eq!(watcher.state(), GuardState::Alert);

        // A drop >=10 GB below the last alerted value re-alerts, even mid-episode.
        let t_drop = t0 + Duration::from_secs(55 * 60);
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 50, 40, 0), t_drop),
            None
        );
        let rep = watcher.check_at(&HostSample::new(10.0, 50, 35, 0), t_drop);
        assert!(
            rep.as_ref().is_some_and(GuardReport::is_alert),
            "a drop >=10 GB below the last alerted value must re-alert, got {rep:?}"
        );
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 50, 34, 0), t_drop),
            None
        );
    }

    #[test]
    fn guard_realerts_after_window_and_on_mem_drop() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let mut watcher = GuardWatcher::new(thresholds);
        let t0 = std::time::Instant::now();
        assert!(watcher
            .check_at(&HostSample::new(10.0, 18, 100, 0), t0)
            .is_some());
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 25, 100, 0), t0),
            Some(GuardReport::Clear)
        );
        // 3 points lower within the window: suppressed.
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 15, 100, 0), t0),
            None
        );
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 25, 100, 0), t0),
            None
        );
        // 10 points lower: re-alert.
        assert!(watcher
            .check_at(&HostSample::new(10.0, 8, 100, 0), t0)
            .is_some());
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 25, 100, 0), t0),
            Some(GuardReport::Clear)
        );
        // Same value but 24h later: re-alert.
        let later = t0 + GUARD_REALERT_WINDOW;
        assert!(watcher
            .check_at(&HostSample::new(10.0, 8, 100, 0), later)
            .is_some());
    }

    #[test]
    fn guard_alert_episode_longer_than_window_still_reports_a_worse_disk() {
        let thresholds = GuardThresholds {
            load_alert: 150.0,
            load_ok: 80.0,
            mem_free_min_pct: 20,
            disk_free_min_gb: 50,
            lanes_max: 6,
        };
        let mut watcher = GuardWatcher::new(thresholds);
        let t0 = std::time::Instant::now();
        assert!(watcher
            .check_at(&HostSample::new(10.0, 50, 40, 0), t0)
            .is_some());
        let later = t0 + GUARD_REALERT_WINDOW + Duration::from_secs(60);
        // Unchanged after a day of the same episode: time alone stays quiet.
        assert_eq!(
            watcher.check_at(&HostSample::new(10.0, 50, 40, 0), later),
            None
        );
        // Disk fell another 10+ GB: the lead hears about it even after 24h.
        assert!(watcher
            .check_at(&HostSample::new(10.0, 50, 28, 0), later)
            .is_some());
    }

    #[tokio::test]
    async fn zero_heavy_slots_is_clamped_instead_of_waiting_forever() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let guard = tokio::time::timeout(
            Duration::from_secs(10),
            acquire_heavy_slot(Some(temp_dir.path()), Some(0), Duration::from_millis(10)),
        )
        .await
        .expect("a zero slot count must not hang")
        .expect("acquire slot");
        assert_eq!(guard.slot_index(), 0);
    }

    #[tokio::test]
    async fn test_slot_takeover_when_holder_pid_is_dead() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let slot_path = temp_dir.path().join("slot.0");

        let dead_pid = 99_999_999;
        assert!(
            !is_pid_alive(dead_pid),
            "pid {} should not be alive",
            dead_pid
        );
        std::fs::write(&slot_path, format!("{}\n", dead_pid)).expect("write slot");

        let initial_pid = read_slot_pid(&slot_path);
        assert_eq!(initial_pid, Some(dead_pid));

        let acquired = try_acquire_heavy_slot(Some(temp_dir.path()), Some(1))
            .await
            .expect("acquire slot");
        assert!(acquired.is_some(), "expected to acquire slot");

        let guard = acquired.unwrap();
        assert_eq!(guard.slot_index(), 0);

        let current_pid = read_slot_pid(&slot_path);
        assert_eq!(current_pid, Some(std::process::id()));

        let second_acquire = try_acquire_heavy_slot(Some(temp_dir.path()), Some(1))
            .await
            .expect("second acquire");
        assert!(
            second_acquire.is_none(),
            "slot should be locked by first guard"
        );

        drop(guard);

        let third_acquire = try_acquire_heavy_slot(Some(temp_dir.path()), Some(1))
            .await
            .expect("third acquire");
        assert!(
            third_acquire.is_some(),
            "slot should be acquirable after release"
        );
    }

    #[test]
    fn test_parsers_loadavg_mem_df() {
        let loadavg_raw = "{ 84.73 58.23 31.64 }";
        let load = parse_sysctl_loadavg(loadavg_raw).expect("parse loadavg");
        assert!((load - 84.73).abs() < 1e-4);

        let loadavg_plain = "1.52 2.10 3.05";
        let load_plain = parse_sysctl_loadavg(loadavg_plain).expect("parse loadavg plain");
        assert!((load_plain - 1.52).abs() < 1e-4);

        let mem_raw = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\nSystem-wide memory free percentage: 48%\n";
        let mem = parse_memory_pressure(mem_raw).expect("parse memory");
        assert_eq!(mem, 48);

        let df_raw = "Filesystem     1024-blocks      Used Available Capacity iused     ifree %iused  Mounted on\n/dev/disk3s1s1   971350180  13340024  36236276    27%  484019 362362760    0%   /\n";
        let disk_gb = parse_df_k(df_raw).expect("parse df");
        assert_eq!(disk_gb, 36236276 / (1024 * 1024));
    }

    #[tokio::test]
    async fn test_fake_active_lane_counter() {
        let mut counter = FakeActiveLaneCounter::new(3);
        assert_eq!(counter.count_active_lanes().await.unwrap(), 3);
        counter.set(7);
        assert_eq!(counter.count_active_lanes().await.unwrap(), 7);
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_sample_printing_real_numbers() {
        let counter = FakeActiveLaneCounter::new(0);
        let sample = sample(&counter).await.expect("sample live metrics");
        println!(
            "LIVE SAMPLE: load_1m={:.2}, mem_free_pct={}% , disk_free_gb={}GB, active_lanes={}",
            sample.load_1m, sample.mem_free_pct, sample.disk_free_gb, sample.active_lanes
        );

        assert!(sample.load_1m >= 0.0);
        assert!(sample.mem_free_pct <= 100);
        assert!(sample.disk_free_gb > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_sample_disk_hold_failure() {
        let thresholds = GuardThresholds::from_env();
        assert_eq!(thresholds.disk_free_min_gb, 100_000);
        let counter = FakeActiveLaneCounter::new(0);
        let decision = can_launch_with_thresholds(&counter, &thresholds)
            .await
            .expect("can_launch");
        println!("DISK PROBE: {decision}");
        assert!(
            decision.is_hold(),
            "expected Hold due to unrealistic disk threshold 100,000 GB, got: {:?}",
            decision
        );
        assert!(
            decision.has_reason("disk"),
            "expected 'disk' in hold reason, got: {:?}",
            decision.reasons()
        );
    }

    #[test]
    fn test_parsers_malformed_metric_inputs() {
        // sysctl loadavg malformed tests
        assert!(parse_sysctl_loadavg("").is_err());
        assert!(parse_sysctl_loadavg("   ").is_err());
        assert!(parse_sysctl_loadavg("{}").is_err());
        assert!(parse_sysctl_loadavg("{   }").is_err());
        assert!(parse_sysctl_loadavg("not_a_number 1.0 2.0").is_err());

        // memory_pressure malformed tests
        assert!(parse_memory_pressure("").is_err());
        assert!(parse_memory_pressure("Random log without target line").is_err());
        assert!(parse_memory_pressure("System-wide memory free percentage:").is_err());
        assert!(
            parse_memory_pressure("System-wide memory free percentage: not_a_number%").is_err()
        );
        assert!(parse_memory_pressure("System-wide memory free percentage: -10%").is_err());

        // df -k malformed tests
        assert!(parse_df_k("").is_err());
        assert!(parse_df_k("Header line only\n").is_err());
        assert!(parse_df_k("Filesystem 1024-blocks Used\n/dev/disk1 100").is_err());
        assert!(
            parse_df_k("Filesystem 1024-blocks Used Available\n/dev/disk1 100 20 not_num").is_err()
        );
    }

    #[tokio::test]
    async fn test_heavy_slot_raii_release_on_cancellation() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let dir = temp_dir.path().to_path_buf();
        let (tx_acquired, rx_acquired) = tokio::sync::oneshot::channel();

        let dir_clone = dir.clone();
        let owner_task = tokio::spawn(async move {
            let guard = try_acquire_heavy_slot(Some(&dir_clone), Some(1))
                .await
                .expect("owner acquire slot")
                .expect("expected Some(HeavySlotGuard)");
            assert_eq!(guard.slot_index(), 0);

            // Signal AFTER flock is acquired
            let _ = tx_acquired.send(());

            // Hold guard indefinitely until task cancellation (abort)
            std::future::pending::<()>().await;
            drop(guard);
        });

        // Bounded await for owner task to acquire the slot
        tokio::time::timeout(Duration::from_secs(5), rx_acquired)
            .await
            .expect("owner acquisition timeout")
            .expect("owner task acquired slot");

        // Assert contender fails while owner holds slot
        let contender = try_acquire_heavy_slot(Some(&dir), Some(1))
            .await
            .expect("contender try_acquire");
        assert!(
            contender.is_none(),
            "contender must fail while owner task holds slot"
        );

        // Abort and await the owner task
        owner_task.abort();
        let _ = owner_task.await;

        // ONE direct acquisition succeeds (no sleeps/polling)
        let reacquired = try_acquire_heavy_slot(Some(&dir), Some(1))
            .await
            .expect("direct acquire after cancellation");
        assert!(
            reacquired.is_some(),
            "slot must be directly acquirable immediately after owner task is aborted"
        );
    }

    #[test]
    #[ignore]
    fn helper_hung_child_process() {
        if std::env::var_os("OMON_GUARD_TEST_HELPER").as_deref() != Some(std::ffi::OsStr::new("1"))
        {
            // If invoked in a broad `cargo test -- --ignored` without the child marker,
            // return immediately to avoid hanging the test suite.
            return;
        }

        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "READY");
        let _ = stdout.flush();
        // Wait on pending event: pause until terminating signal
        #[cfg(unix)]
        unsafe {
            libc::pause();
        }
        #[cfg(not(unix))]
        loop {
            std::thread::park();
        }
    }

    #[tokio::test]
    async fn test_metric_subprocess_timeout_probe() {
        use tokio::io::AsyncBufReadExt;

        let current_exe = std::env::current_exe().expect("current test exe");
        let mut cmd = tokio::process::Command::new(current_exe);
        cmd.arg("lanes::guard::tests::helper_hung_child_process")
            .arg("--ignored")
            .arg("--exact")
            .arg("--nocapture")
            .env("OMON_GUARD_TEST_HELPER", "1");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn().expect("spawn helper hung child");
        let child_pid = child.id().expect("child pid");

        // Separate readiness await from time behavior under test:
        // Subscribe to stdout and wait boundedly for READY line
        let stdout = child.stdout.take().expect("child stdout");
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        let ready_timeout = tokio::time::timeout(Duration::from_secs(5), async {
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                if line.trim() == "READY" {
                    return true;
                }
                line.clear();
            }
            false
        })
        .await;

        assert!(
            matches!(ready_timeout, Ok(true)),
            "child process must have started and signalled READY on stdout without file polling"
        );
        assert!(is_pid_alive(child_pid), "child must be alive and pending");

        // Restore child stdout for wait_child_with_timeout
        child.stdout = Some(reader.into_inner());

        // Apply timeout action on the coordinated hung child
        let res = wait_child_with_timeout(child, Duration::from_millis(50), "hung probe").await;
        assert!(
            matches!(res, Err(GuardError::Timeout(_))),
            "expected Timeout error, got {:?}",
            res
        );

        // Bounded cleanup verification: confirm the child process was killed and reaped
        assert!(
            !is_pid_alive(child_pid),
            "child process {child_pid} must be killed and reaped on timeout"
        );
    }
}

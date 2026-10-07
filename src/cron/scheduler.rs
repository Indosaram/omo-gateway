use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use tokio::sync::{broadcast, watch, Mutex, Notify};
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::guard::check_gateway_lifecycle;
use super::store::{get_cron_authority, update_cron_authority, CronAuthority};
use crate::{
    HermesJob, HermesStoreSynchronizer, OmonError, OutboundAction, OutboundDispatcher, Result,
    SessionKey,
};

pub const LEASE_DURATION: TimeDelta = TimeDelta::minutes(30);
pub const LEASE_REFRESH_INTERVAL: Duration = Duration::from_secs(60);
pub const STALE_LEASE_SAFETY_NET: TimeDelta = TimeDelta::minutes(120);

/// Decides whether a candidate expired cron run lease should be reclaimed.
///
/// A run is reclaimed if the lease is expired AND (the owner process is not alive,
/// the owner_pid is NULL, or the lease is older than `STALE_LEASE_SAFETY_NET`
/// as a safety net for wedged processes).
pub fn should_reclaim(owner_pid: Option<u32>, lease_expired: bool, lease_age: TimeDelta) -> bool {
    should_reclaim_with(
        owner_pid,
        lease_expired,
        lease_age,
        crate::ledger::is_process_alive,
    )
}

/// Decides whether a candidate expired cron run lease should be reclaimed using a custom liveness check.
pub fn should_reclaim_with<F>(
    owner_pid: Option<u32>,
    lease_expired: bool,
    lease_age: TimeDelta,
    is_alive: F,
) -> bool
where
    F: Fn(u32) -> bool,
{
    if !lease_expired {
        return false;
    }
    if lease_age >= STALE_LEASE_SAFETY_NET {
        return true;
    }
    match owner_pid {
        None => true,
        Some(pid) => pid == 0 || !is_alive(pid),
    }
}

pub const MAX_CONTEXT_CHARS: usize = 8000;

pub fn is_valid_context_job_id(job_id: &str) -> bool {
    !job_id.is_empty() && !job_id.contains("..") && !job_id.contains('/') && !job_id.contains('\\')
}

pub fn parse_context_from_ids(context_from: Option<&Value>) -> Vec<String> {
    let Some(value) = context_from else {
        return Vec::new();
    };
    match value {
        Value::String(s) => {
            let s = s.trim();
            if is_valid_context_job_id(s) {
                vec![s.to_string()]
            } else {
                Vec::new()
            }
        }
        Value::Array(arr) => arr
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|s| is_valid_context_job_id(s))
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

pub fn truncate_context_output(output: &str, max_chars: usize) -> String {
    let count = output.chars().count();
    if count > max_chars {
        let prefix: String = output.chars().take(max_chars).collect();
        format!("{prefix}\n\n[... output truncated ...]")
    } else {
        output.to_string()
    }
}

pub fn format_context_from_block(job_id: &str, output: &str) -> String {
    format!(
        "## Output from job '{job_id}'\n\
        The following is the most recent output from a preceding cron job. Use it as context for your analysis.\n\n\
        ```\n{output}\n```"
    )
}

pub async fn resolve_predecessor_output(
    pool: &SqlitePool,
    profile: &str,
    job_id: &str,
) -> Option<String> {
    if !is_valid_context_job_id(job_id) {
        return None;
    }

    // 1. Check persistent cron_outputs table by exact profile and job_id
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT output FROM cron_outputs \
         WHERE profile = ? AND job_id = ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(profile)
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((content,)) = row {
        if !content.trim().is_empty() {
            return Some(content);
        }
    }

    // 2. Fallback: Check messages table for backward compatibility with exact session keys
    let exact_session = if profile.is_empty() {
        format!("cron:{job_id}")
    } else {
        format!("discord:{profile}:cron:{job_id}")
    };

    let db_result: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM messages \
         WHERE (session_key = ? OR session_key LIKE ?) \
           AND role = 'assistant' AND TRIM(content) != '' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&exact_session)
    .bind(format!("%:{job_id}"))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((content,)) = db_result {
        if !content.trim().is_empty() {
            return Some(content);
        }
    }
    None
}

pub fn parse_wake_gate(stdout: &str) -> bool {
    let stripped_lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let Some(&last_line) = stripped_lines.last() else {
        return true;
    };

    // 1. Try parsing JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(last_line) {
        if let Some(obj) = value.as_object() {
            if let Some(wake) = obj.get("wakeAgent").or_else(|| obj.get("wake_agent")) {
                if let Some(b) = wake.as_bool() {
                    return b;
                }
            }
        }
    }

    // 2. Try parsing plain-text / YAML pattern (e.g. "wakeAgent: false")
    let lower = last_line.to_ascii_lowercase();
    if lower == "wakeagent: false"
        || lower == "wake_agent: false"
        || lower == "{\"wakeagent\": false}"
        || lower == "{\"wakeagent\":false}"
    {
        return false;
    }

    true
}

const CRON_SILENCE_TOKENS: &[&str] = &["[SILENT]", "SILENT", "NO_REPLY", "NO REPLY"];

fn is_silence_token_line(line: &str) -> bool {
    let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let upper = normalized.to_uppercase();
    CRON_SILENCE_TOKENS.iter().any(|&token| token == upper)
}

pub fn is_cron_silence_response(text: &str) -> bool {
    let stripped = text.trim();
    if stripped.is_empty() {
        return true;
    }

    if is_silence_token_line(stripped) {
        return true;
    }

    let lines: Vec<&str> = stripped
        .lines()
        .map(str::trim)
        .filter(|ln| !ln.is_empty())
        .collect();
    if let Some(&first) = lines.first() {
        if is_silence_token_line(first) {
            return true;
        }
    }
    if let Some(&last) = lines.last() {
        if is_silence_token_line(last) {
            return true;
        }
    }

    let upper = stripped.to_uppercase();
    if upper.starts_with("[SILENT]") {
        return true;
    }

    false
}

pub fn should_disable_after(completed: u64, times: Option<u64>) -> bool {
    match times {
        Some(limit) if limit > 0 => completed >= limit,
        _ => false,
    }
}

pub fn extract_repeat_info(payload: &Value) -> (Option<u64>, u64) {
    if let Some(repeat) = payload.get("repeat") {
        let times = repeat.get("times").and_then(Value::as_u64);
        let completed = repeat.get("completed").and_then(Value::as_u64).unwrap_or(0);
        return (times, completed);
    }
    let times = payload.get("times").and_then(Value::as_u64);
    let completed = payload
        .get("completed")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    (times, completed)
}

pub fn increment_repeat_completed(payload: &mut Value) -> (Option<u64>, u64) {
    if let Some(repeat) = payload.get_mut("repeat") {
        if let Some(obj) = repeat.as_object_mut() {
            let times = obj.get("times").and_then(Value::as_u64);
            let completed = obj.get("completed").and_then(Value::as_u64).unwrap_or(0) + 1;
            obj.insert("completed".to_string(), Value::from(completed));
            return (times, completed);
        }
    }
    let times = payload.get("times").and_then(Value::as_u64);
    let completed = payload
        .get("completed")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + 1;
    if let Some(obj) = payload.as_object_mut() {
        if times.is_some() {
            obj.insert("completed".to_string(), Value::from(completed));
        } else {
            let mut repeat_obj = serde_json::Map::new();
            repeat_obj.insert("completed".to_string(), Value::from(completed));
            obj.insert("repeat".to_string(), Value::Object(repeat_obj));
        }
    }
    (times, completed)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CronJobSpec {
    pub expression: String,
    #[serde(default)]
    pub payload: Value,
    pub session_key: Option<String>,
}

impl CronJobSpec {
    pub fn new(expression: impl Into<String>, payload: Value) -> Self {
        Self {
            expression: expression.into(),
            payload,
            session_key: None,
        }
    }
}

fn default_authority_string() -> String {
    "omon_owned".to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct CronJob {
    pub id: String,
    pub session_key: Option<String>,
    pub expression: String,
    #[sqlx(rename = "payload_json")]
    pub payload_json: String,
    pub enabled: bool,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    #[serde(default = "default_authority_string")]
    pub authority: String,
}

impl CronJob {
    pub fn authority(&self) -> CronAuthority {
        if self.authority.is_empty() {
            CronAuthority::OmonOwned
        } else {
            self.authority.parse().unwrap_or(CronAuthority::OmonOwned)
        }
    }

    pub fn is_hermes_mirror(&self) -> bool {
        self.authority() == CronAuthority::HermesMirror
    }

    pub fn is_cutover_pending(&self) -> bool {
        self.authority() == CronAuthority::CutoverPending
    }

    pub fn is_omon_owned(&self) -> bool {
        self.authority() == CronAuthority::OmonOwned
    }

    pub fn payload(&self) -> Result<Value> {
        serde_json::from_str(&self.payload_json)
            .map_err(|error| OmonError::Config(format!("invalid cron payload: {error}")))
    }

    pub fn last_status(&self) -> Option<String> {
        self.payload().ok().and_then(|p| {
            p.get("last_status")
                .and_then(Value::as_str)
                .map(String::from)
        })
    }

    pub fn last_run_at(&self) -> Option<String> {
        self.payload().ok().and_then(|p| {
            p.get("last_run_at")
                .and_then(Value::as_str)
                .map(String::from)
        })
    }

    pub fn profile(&self) -> String {
        if let Ok(payload) = self.payload() {
            if let Some(p) = payload.get("profile").and_then(Value::as_str) {
                if !p.is_empty() {
                    return p.to_string();
                }
            }
        }
        if let Some(key) = &self.session_key {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() >= 2 && !parts[1].is_empty() && parts[0] == "discord" {
                return parts[1].to_string();
            }
        }
        String::new()
    }

    pub fn last_error(&self) -> Option<String> {
        self.payload().ok().and_then(|p| {
            p.get("last_error")
                .and_then(Value::as_str)
                .map(String::from)
        })
    }

    pub fn last_delivery_error(&self) -> Option<String> {
        self.payload().ok().and_then(|p| {
            p.get("last_delivery_error")
                .and_then(Value::as_str)
                .map(String::from)
        })
    }

    pub fn is_missed(&self) -> bool {
        self.last_status().as_deref() == Some("missed")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CronNotification {
    pub job_id: String,
    pub channel_id: u64,
    pub content: String,
    pub triggered_at: DateTime<Utc>,
}

#[async_trait]
pub trait CronTaskExecutor: Send + Sync + 'static {
    /// Executes a job and optionally returns completion text. If no text is
    /// returned, the scheduler uses `notification` or `content` from payload.
    async fn execute(&self, job: &CronJob) -> Result<Option<String>>;
}

#[derive(Default)]
pub struct ShellAndPayloadTaskExecutor;

#[async_trait]
impl CronTaskExecutor for ShellAndPayloadTaskExecutor {
    async fn execute(&self, job: &CronJob) -> Result<Option<String>> {
        let payload = job.payload()?;

        if let Some(cmd) = payload
            .get("command")
            .or_else(|| payload.get("script"))
            .and_then(Value::as_str)
        {
            tracing::info!(job_id = %job.id, command = %cmd, "Executing cron shell command");
            let augmented_path = crate::tools::augmented_path_from_environment();
            let mut attempts = 0u64;
            let output = loop {
                attempts += 1;
                let mut command = tokio::process::Command::new("sh");
                command.arg("-c").arg(cmd);
                if !augmented_path.is_empty() {
                    command.env("PATH", &augmented_path);
                }
                let out = command.output().await.map_err(|e| {
                    OmonError::ToolExecution(format!("failed to execute cron command: {e}"))
                })?;
                if !out.status.success()
                    && crate::cron::executor::is_transient_process_init_error(out.status.code())
                    && attempts < 3
                {
                    tracing::warn!(
                        job_id = %job.id,
                        code = ?out.status.code(),
                        attempt = attempts,
                        "transient process initialization error detected in cron shell command, retrying..."
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(250 * attempts)).await;
                    continue;
                }
                break out;
            };

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                tracing::info!(job_id = %job.id, "Cron command completed successfully");
                return Ok(Some(if stdout.trim().is_empty() {
                    format!("Cron job `{}` completed successfully", job.id)
                } else {
                    format!("Cron job `{}` output:\n```\n{}\n```", job.id, stdout.trim())
                }));
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OmonError::ToolExecution(format!(
                "cron job `{}` failed with {:?}: {}",
                job.id,
                output.status.code(),
                stderr.trim()
            )));
        }

        Ok(payload
            .get("notification")
            .or_else(|| payload.get("content"))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }
}

#[derive(Default)]
pub struct PayloadTaskExecutor;

#[async_trait]
impl CronTaskExecutor for PayloadTaskExecutor {
    async fn execute(&self, job: &CronJob) -> Result<Option<String>> {
        ShellAndPayloadTaskExecutor.execute(job).await
    }
}

struct SchedulerState {
    shutdown: watch::Sender<bool>,
    task: Mutex<Option<JoinHandle<()>>>,
    executions: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct CronScheduler {
    pool: SqlitePool,
    executor: Arc<dyn CronTaskExecutor>,
    dispatcher: Option<Arc<dyn OutboundDispatcher>>,
    hermes_sync: Option<Arc<HermesStoreSynchronizer>>,
    notifications: broadcast::Sender<CronNotification>,
    wake: Arc<Notify>,
    poll_interval: Duration,
    state: Arc<SchedulerState>,
    clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    pre_retire_gate: Option<(Arc<Notify>, Arc<Notify>)>,
}

#[derive(Clone)]
struct CronClaim {
    run_id: String,
    claim_token: String,
    job: CronJob,
    advance_schedule: bool,
    occurrence: Option<DateTime<Utc>>,
}

pub async fn has_pending_cutover_receipt(pool: &SqlitePool) -> Result<bool> {
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='cron_cutover_receipts')",
    )
    .fetch_one(pool)
    .await?;
    if !table_exists {
        return Ok(false);
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM cron_cutover_receipts WHERE status = 'pending')",
    )
    .fetch_one(pool)
    .await?;
    Ok(pending)
}

impl CronScheduler {
    pub fn new(pool: SqlitePool, executor: Arc<dyn CronTaskExecutor>) -> Self {
        Self::with_options(pool, executor, None, Duration::from_secs(1))
    }

    pub async fn is_running(&self) -> bool {
        let task = self.state.task.lock().await;
        task.as_ref().is_some_and(|h| !h.is_finished())
    }

    pub async fn active_executions_count(&self) -> usize {
        let executions = self.state.executions.lock().await;
        executions.iter().filter(|h| !h.is_finished()).count()
    }

    pub async fn wait_idle(&self) {
        let executions = {
            let mut guard = self.state.executions.lock().await;
            std::mem::take(&mut *guard)
        };
        for execution in executions {
            let _ = execution.await;
        }
    }

    pub async fn acknowledge_incident(
        &self,
        job_id: &str,
        signature: Option<&str>,
    ) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let rows = if let Some(sig) = signature {
            sqlx::query(
                "UPDATE cron_incidents SET acknowledged = 1, updated_at = ? WHERE job_id = ? AND error_signature = ?",
            )
            .bind(&now)
            .bind(job_id)
            .bind(sig)
            .execute(&self.pool)
            .await
            .map_err(|e| OmonError::Database(e.to_string()))?
            .rows_affected()
        } else {
            sqlx::query(
                "UPDATE cron_incidents SET acknowledged = 1, updated_at = ? WHERE job_id = ?",
            )
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| OmonError::Database(e.to_string()))?
            .rows_affected()
        };
        Ok(rows > 0)
    }

    pub fn with_dispatcher(
        pool: SqlitePool,
        executor: Arc<dyn CronTaskExecutor>,
        dispatcher: Arc<dyn OutboundDispatcher>,
    ) -> Self {
        Self::with_options(pool, executor, Some(dispatcher), Duration::from_secs(1))
    }

    pub fn with_poll_interval(
        pool: SqlitePool,
        executor: Arc<dyn CronTaskExecutor>,
        poll_interval: Duration,
    ) -> Self {
        Self::with_options(pool, executor, None, poll_interval)
    }

    fn with_options(
        pool: SqlitePool,
        executor: Arc<dyn CronTaskExecutor>,
        dispatcher: Option<Arc<dyn OutboundDispatcher>>,
        poll_interval: Duration,
    ) -> Self {
        let (notifications, _) = broadcast::channel(128);
        let (shutdown, _) = watch::channel(false);
        Self {
            pool,
            executor,
            dispatcher,
            hermes_sync: None,
            notifications,
            wake: Arc::new(Notify::new()),
            poll_interval,
            clock: Arc::new(Utc::now),
            pre_retire_gate: None,
            state: Arc::new(SchedulerState {
                shutdown,
                task: Mutex::new(None),
                executions: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn with_hermes_sync(mut self, synchronizer: HermesStoreSynchronizer) -> Self {
        self.hermes_sync = Some(Arc::new(synchronizer));
        self
    }

    /// Supplies the clock used at scheduling, claim and completion boundaries.
    pub fn with_clock(mut self, clock: impl Fn() -> DateTime<Utc> + Send + Sync + 'static) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// Internal synchronization gate for testing deterministic races around retirement.
    pub fn with_pre_retire_gate(mut self, entered: Arc<Notify>, release: Arc<Notify>) -> Self {
        self.pre_retire_gate = Some((entered, release));
        self
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CronNotification> {
        self.notifications.subscribe()
    }

    pub async fn start(&self) {
        let mut task = self.state.task.lock().await;
        if task.as_ref().is_some_and(|task| !task.is_finished()) {
            return;
        }
        let scheduler = self.clone();
        let mut shutdown = self.state.shutdown.subscribe();
        *task = Some(tokio::spawn(async move {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                if let Some(synchronizer) = &scheduler.hermes_sync {
                    if let Err(error) = synchronizer.sync_at((scheduler.clock)()).await {
                        tracing::error!(%error, "Hermes cron store synchronization failed");
                    }
                }
                if let Err(error) = scheduler.run_due_jobs().await {
                    tracing::error!(%error, "cron scheduler poll failed");
                }
                tokio::select! {
                    _ = tokio::time::sleep(scheduler.poll_interval) => {},
                    _ = scheduler.wake.notified() => {},
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() { break; }
                    }
                }
            }
        }));
    }

    pub async fn shutdown(&self) {
        let _ = self.state.shutdown.send(true);
        self.wake.notify_waiters();
        if let Some(task) = self.state.task.lock().await.take() {
            let _ = task.await;
        }
        let executions = std::mem::take(&mut *self.state.executions.lock().await);
        for execution in executions {
            let _ = execution.await;
        }
    }

    pub fn validate_cron_payload_lifecycle(payload: &Value) -> Result<()> {
        if let Some(prompt) = payload.get("prompt").and_then(Value::as_str) {
            if let Err(err) = check_gateway_lifecycle(prompt) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in prompt: {err}"
                )));
            }
            let threats = crate::security::scan_cron_prompt(prompt);
            if !threats.is_empty() {
                return Err(OmonError::Config(format!(
                    "cron prompt injection detected: {}",
                    threats.join("; ")
                )));
            }
        }
        if let Some(script) = payload.get("script").and_then(Value::as_str) {
            if let Err(err) = check_gateway_lifecycle(script) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in script: {err}"
                )));
            }
        }
        if let Some(command) = payload.get("command").and_then(Value::as_str) {
            if let Err(err) = check_gateway_lifecycle(command) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in command: {err}"
                )));
            }
        }
        if let Some(ack) = payload.get("ack_command").and_then(Value::as_str) {
            if let Err(err) = check_gateway_lifecycle(ack) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in ack_command: {err}"
                )));
            }
        }
        if let Some(brief_file) = payload.get("brief_file").and_then(Value::as_str) {
            let path = std::path::Path::new(brief_file);
            if !path.exists() {
                return Err(OmonError::Config(format!(
                    "brief_file does not exist: {brief_file}"
                )));
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    return Err(OmonError::Config(format!(
                        "failed to read brief_file {brief_file}: {e}"
                    )));
                }
            };
            if let Err(err) = check_gateway_lifecycle(&content) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in brief_file: {err}"
                )));
            }
            let threats = crate::security::scan_cron_prompt(&content);
            if !threats.is_empty() {
                return Err(OmonError::Config(format!(
                    "cron prompt injection detected in brief_file: {}",
                    threats.join("; ")
                )));
            }
        }
        if let Some(tz) = payload
            .get("schedule")
            .and_then(|s| s.get("timezone"))
            .and_then(Value::as_str)
        {
            if tz.trim().parse::<chrono_tz::Tz>().is_err() {
                return Err(OmonError::Config(format!(
                    "uncomputable schedule: invalid timezone `{tz}`"
                )));
            }
        }
        Ok(())
    }

    pub async fn register_with_id(
        &self,
        id: impl Into<String>,
        spec: CronJobSpec,
    ) -> Result<CronJob> {
        Self::validate_cron_payload_lifecycle(&spec.payload)?;
        let id = id.into();
        let now = (self.clock)();
        let timezone = spec
            .payload
            .get("schedule")
            .and_then(|s| s.get("timezone"))
            .and_then(Value::as_str);
        let next_run_at = next_run_tz(&spec.expression, now, timezone)?;
        let payload_json = serde_json::to_string(&spec.payload)
            .map_err(|error| OmonError::Config(error.to_string()))?;
        sqlx::query(
            "INSERT INTO cron_jobs
             (id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, 1, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
             session_key = excluded.session_key,
             expression = excluded.expression,
             payload_json = excluded.payload_json,
             next_run_at = excluded.next_run_at,
             updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(&spec.session_key)
        .bind(&spec.expression)
        .bind(payload_json)
        .bind(next_run_at)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.wake.notify_one();
        self.get(&id)
            .await?
            .ok_or_else(|| OmonError::Database("registered cron job disappeared".into()))
    }

    pub async fn register(&self, spec: CronJobSpec) -> Result<CronJob> {
        Self::validate_cron_payload_lifecycle(&spec.payload)?;
        let now = (self.clock)();
        let timezone = spec
            .payload
            .get("schedule")
            .and_then(|s| s.get("timezone"))
            .and_then(Value::as_str);
        let next_run_at = next_run_tz(&spec.expression, now, timezone)?;
        let id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&spec.payload)
            .map_err(|error| OmonError::Config(error.to_string()))?;
        sqlx::query(
            "INSERT INTO cron_jobs
             (id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, 1, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&spec.session_key)
        .bind(&spec.expression)
        .bind(payload_json)
        .bind(next_run_at)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        self.wake.notify_one();
        self.get(&id)
            .await?
            .ok_or_else(|| OmonError::Database("registered cron job disappeared".into()))
    }

    pub async fn register_job(
        &self,
        expression: impl Into<String>,
        payload: Value,
    ) -> Result<CronJob> {
        self.register(CronJobSpec::new(expression, payload)).await
    }

    pub async fn get(&self, id: &str) -> Result<Option<CronJob>> {
        Ok(
            sqlx::query_as::<_, CronJob>("SELECT * FROM cron_jobs WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn list_active(&self) -> Result<Vec<CronJob>> {
        Ok(sqlx::query_as::<_, CronJob>(
            "SELECT * FROM cron_jobs WHERE enabled = 1 ORDER BY next_run_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn pause(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE cron_jobs SET enabled = 0, next_run_at = NULL, updated_at = ? WHERE id = ?",
        )
        .bind((self.clock)())
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.wake.notify_one();
        Ok(result.rows_affected() != 0)
    }

    pub async fn pause_job(&self, id: &str) -> Result<bool> {
        self.pause(id).await
    }

    pub async fn resume(&self, id: &str) -> Result<bool> {
        let Some(job) = self.get(id).await? else {
            return Ok(false);
        };
        let now = (self.clock)();
        let next = next_run(&job.expression, now)?;
        sqlx::query(
            "UPDATE cron_jobs SET enabled = 1, next_run_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(next)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        self.wake.notify_one();
        Ok(true)
    }

    pub async fn resume_job(&self, id: &str) -> Result<bool> {
        self.resume(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM cron_jobs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        self.wake.notify_one();
        Ok(result.rows_affected() != 0)
    }

    pub async fn delete_job(&self, id: &str) -> Result<bool> {
        self.delete(id).await
    }

    /// Claims a job through the same lease pipeline used by scheduled runs and
    /// executes it asynchronously. A manual run does not consume or advance the
    /// persisted schedule.
    pub async fn trigger(&self, id: &str) -> Result<bool> {
        if self.get(id).await?.is_none() {
            return Ok(false);
        }
        let Some(claim) = self.claim_job(id, false, false).await? else {
            return Ok(false);
        };
        self.spawn_claim(claim).await;
        Ok(true)
    }

    pub async fn trigger_job(&self, id: &str) -> Result<bool> {
        self.trigger(id).await
    }

    async fn retire_overdue_oneshot(
        &self,
        id: &str,
        expression: &str,
        observed_next_run_at: DateTime<Utc>,
        observed_payload_json: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let mut payload: Value = serde_json::from_str(observed_payload_json)
            .map_err(|error| OmonError::Config(format!("invalid cron payload: {error}")))?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "last_status".to_string(),
                Value::String("missed".to_string()),
            );
            obj.insert("last_run_at".to_string(), Value::String(now.to_rfc3339()));
            obj.insert(
                "last_error".to_string(),
                Value::String("missed: overdue unclaimed one-shot past grace".to_string()),
            );
            obj.insert("last_delivery_error".to_string(), Value::Null);
        }
        let updated_payload_json = serde_json::to_string(&payload).map_err(|error| {
            OmonError::Config(format!("failed to serialize updated payload: {error}"))
        })?;

        if let Some((entered, release)) = &self.pre_retire_gate {
            entered.notify_one();
            release.notified().await;
        }

        let result = sqlx::query(
            "UPDATE cron_jobs
             SET enabled = 0, next_run_at = NULL, payload_json = ?, updated_at = ?
             WHERE id = ?
               AND enabled = 1
               AND expression = ?
               AND next_run_at = ?
               AND payload_json = ?
               AND NOT EXISTS (
                   SELECT 1 FROM cron_runs
                   WHERE job_id = ? AND status = 'running'
               )",
        )
        .bind(&updated_payload_json)
        .bind(now)
        .bind(id)
        .bind(expression)
        .bind(observed_next_run_at)
        .bind(observed_payload_json)
        .bind(id)
        .execute(&self.pool)
        .await?;

        let retired = result.rows_affected() > 0;
        if retired {
            tracing::info!(job_id = %id, "retired overdue unclaimed one-shot past grace as missed");
            self.wake.notify_one();
        }
        Ok(retired)
    }

    pub async fn get_authority(&self, id: &str) -> Result<Option<CronAuthority>> {
        get_cron_authority(&self.pool, id).await
    }

    pub async fn set_authority(&self, id: &str, authority: CronAuthority) -> Result<bool> {
        update_cron_authority(&self.pool, id, authority).await
    }

    pub async fn has_pending_cutover_receipt(&self) -> Result<bool> {
        has_pending_cutover_receipt(&self.pool).await
    }

    /// Claims every due job and starts each execution in its own task. Claims
    /// are protected by durable lease rows, so concurrent scheduler instances
    /// cannot execute the same job while a live lease exists.
    pub async fn run_due_jobs(&self) -> Result<usize> {
        if has_pending_cutover_receipt(&self.pool).await? {
            return Ok(0);
        }
        let now = (self.clock)();
        let job_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM cron_jobs
             WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?
               AND authority != 'cutover_pending'
             ORDER BY next_run_at, id",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        let mut claimed = 0;
        for id in job_ids {
            if let Some(claim) = self.claim_job(&id, true, true).await? {
                self.spawn_claim(claim).await;
                claimed += 1;
            }
        }
        Ok(claimed)
    }

    async fn claim_job(
        &self,
        id: &str,
        require_due: bool,
        advance_schedule: bool,
    ) -> Result<Option<CronClaim>> {
        if has_pending_cutover_receipt(&self.pool).await? {
            return Ok(None);
        }
        let now = (self.clock)();
        let lease_expires_at = now + LEASE_DURATION;
        let run_id = Uuid::new_v4().to_string();
        let claim_token = Uuid::new_v4().to_string();
        let current_pid = std::process::id() as i64;

        let candidate_runs: Vec<(String, Option<i64>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT run_id, owner_pid, lease_expires_at FROM cron_runs
             WHERE job_id = ? AND status = 'running' AND lease_expires_at <= ?",
        )
        .bind(id)
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        for (candidate_run_id, owner_pid_i64, candidate_lease_expires_at) in candidate_runs {
            let lease_age = now.signed_duration_since(candidate_lease_expires_at);
            let owner_pid = owner_pid_i64.and_then(|p| if p > 0 { Some(p as u32) } else { None });
            if should_reclaim(owner_pid, true, lease_age) {
                sqlx::query(
                    "UPDATE cron_runs
                     SET status = 'failed', completed_at = ?, error = COALESCE(error, 'lease expired before completion')
                     WHERE run_id = ? AND status = 'running'",
                )
                .bind(now)
                .bind(&candidate_run_id)
                .execute(&self.pool)
                .await?;
            }
        }

        let job_row: Option<(String, Option<DateTime<Utc>>, String, bool)> = sqlx::query_as(
            "SELECT expression, next_run_at, payload_json, enabled FROM cron_jobs WHERE id = ? AND authority != 'cutover_pending'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((expression, next_run_opt, payload_json, enabled)) = job_row else {
            return Ok(None);
        };

        let parsed_payload: Value = serde_json::from_str(&payload_json).unwrap_or_default();
        let (times, completed) = extract_repeat_info(&parsed_payload);
        if should_disable_after(completed, times) {
            tracing::warn!(job_id = %id, completed, ?times, "cron job repeat limit reached; skipping claim");
            return Ok(None);
        }

        let mut target_occurrence: Option<DateTime<Utc>> = None;
        if require_due {
            if !enabled {
                return Ok(None);
            }
            if let Some(next_run) = next_run_opt {
                if expression.starts_with("once:") {
                    let overdue = now.signed_duration_since(next_run);
                    let grace = TimeDelta::from_std(ONESHOT_GRACE_DURATION)
                        .unwrap_or_else(|_| TimeDelta::seconds(120));
                    if overdue > grace {
                        self.retire_overdue_oneshot(id, &expression, next_run, &payload_json, now)
                            .await?;
                        return Ok(None);
                    }
                    target_occurrence = Some(next_run);
                } else if next_run <= now {
                    let catch_up_hours = parsed_payload
                        .get("catch_up_hours")
                        .and_then(Value::as_u64)
                        .unwrap_or(6);

                    let timezone = parsed_payload
                        .get("schedule")
                        .and_then(|s| s.get("timezone"))
                        .and_then(Value::as_str);

                    if catch_up_hours == 0 {
                        let next_future = next_run_tz(&expression, now, timezone)?;
                        sqlx::query(
                            "UPDATE cron_jobs SET next_run_at = ?, updated_at = ? WHERE id = ? AND next_run_at = ?",
                        )
                        .bind(next_future)
                        .bind(now)
                        .bind(id)
                        .bind(next_run)
                        .execute(&self.pool)
                        .await?;
                        return Ok(None);
                    }

                    let window_start = now - TimeDelta::hours(catch_up_hours as i64);
                    let newest = newest_missed_occurrence(
                        &expression,
                        next_run,
                        window_start,
                        now,
                        timezone,
                    )?;

                    match newest {
                        None => {
                            let next_future = next_run_tz(&expression, now, timezone)?;
                            sqlx::query(
                                "UPDATE cron_jobs SET next_run_at = ?, updated_at = ? WHERE id = ? AND next_run_at = ?",
                            )
                            .bind(next_future)
                            .bind(now)
                            .bind(id)
                            .bind(next_run)
                            .execute(&self.pool)
                            .await?;
                            return Ok(None);
                        }
                        Some(occ) => {
                            let occ_key = occ.to_rfc3339();
                            let last_occ = parsed_payload
                                .get("last_occurrence")
                                .and_then(Value::as_str);
                            if last_occ == Some(&occ_key) {
                                let next_future = next_run_tz(&expression, now, timezone)?;
                                sqlx::query(
                                    "UPDATE cron_jobs SET next_run_at = ?, updated_at = ? WHERE id = ? AND next_run_at = ?",
                                )
                                .bind(next_future)
                                .bind(now)
                                .bind(id)
                                .bind(next_run)
                                .execute(&self.pool)
                                .await?;
                                return Ok(None);
                            }
                            target_occurrence = Some(occ);
                        }
                    }
                }
            } else {
                return Ok(None);
            }
        }

        let due_clause = if require_due {
            "AND enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?"
        } else {
            ""
        };
        let sql = format!(
            "INSERT INTO cron_runs
             (run_id, job_id, claim_token, lease_expires_at, started_at, completed_at, status, attempt, error, owner_pid)
             SELECT ?, id, ?, ?, ?, NULL, 'running',
                    COALESCE((SELECT MAX(attempt) + 1 FROM cron_runs WHERE job_id = ?), 1), NULL, ?
             FROM cron_jobs
             WHERE id = ? {due_clause}
               AND authority != 'cutover_pending'
               AND NOT EXISTS (
                   SELECT 1 FROM cron_runs
                   WHERE job_id = ? AND status = 'running'
               )"
        );
        let mut query = sqlx::query(&sql)
            .bind(&run_id)
            .bind(&claim_token)
            .bind(lease_expires_at)
            .bind(now)
            .bind(id)
            .bind(current_pid)
            .bind(id);
        if require_due {
            query = query.bind(now);
        }
        let inserted = query.bind(id).execute(&self.pool).await?;
        if inserted.rows_affected() == 0 {
            return Ok(None);
        }

        let job = self
            .get(id)
            .await?
            .ok_or_else(|| OmonError::Database(format!("claimed cron job {id} disappeared")))?;
        Ok(Some(CronClaim {
            run_id,
            claim_token,
            job,
            advance_schedule,
            occurrence: target_occurrence,
        }))
    }

    async fn spawn_claim(&self, claim: CronClaim) {
        let scheduler = self.clone();
        let handle = tokio::spawn(async move {
            scheduler.execute_claim(claim).await;
        });
        let mut executions = self.state.executions.lock().await;
        executions.retain(|execution| !execution.is_finished());
        executions.push(handle);
    }

    async fn execute_claim(&self, claim: CronClaim) {
        let heartbeat_scheduler = self.clone();
        let heartbeat_token = claim.claim_token.clone();
        let (heartbeat_stop, mut heartbeat_stopped) = watch::channel(false);
        let heartbeat = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(LEASE_REFRESH_INTERVAL) => {
                        if let Err(error) = heartbeat_scheduler.refresh_lease(&heartbeat_token).await {
                            tracing::error!(%error, claim_token = %heartbeat_token, "cron lease refresh failed");
                        }
                    }
                    changed = heartbeat_stopped.changed() => {
                        if changed.is_err() || *heartbeat_stopped.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        let result = self.execute_job(&claim.job).await;
        let _ = heartbeat_stop.send(true);
        let _ = heartbeat.await;

        match result {
            Ok(output) => {
                if let Err(error) = self.complete_success(&claim, output.as_deref()).await {
                    tracing::error!(%error, job_id = %claim.job.id, run_id = %claim.run_id, "failed to commit successful cron run");
                }
            }
            Err(error) => {
                if let Err(record_error) = self.complete_failure(&claim, &error).await {
                    tracing::error!(%record_error, job_id = %claim.job.id, run_id = %claim.run_id, "failed to record cron failure");
                }
                tracing::error!(%error, job_id = %claim.job.id, run_id = %claim.run_id, "cron job execution failed");
            }
        }
    }

    async fn refresh_lease(&self, claim_token: &str) -> Result<()> {
        let lease_expires_at = (self.clock)() + LEASE_DURATION;
        sqlx::query(
            "UPDATE cron_runs SET lease_expires_at = ?
             WHERE claim_token = ? AND status = 'running'",
        )
        .bind(lease_expires_at)
        .bind(claim_token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn complete_success(&self, claim: &CronClaim, output: Option<&str>) -> Result<()> {
        let now = (self.clock)();
        let mut transaction = self.pool.begin().await?;
        let completed = sqlx::query(
            "UPDATE cron_runs
             SET status = 'succeeded', completed_at = ?, lease_expires_at = ?, error = NULL
             WHERE run_id = ? AND claim_token = ? AND status = 'running'",
        )
        .bind(now)
        .bind(now)
        .bind(&claim.run_id)
        .bind(&claim.claim_token)
        .execute(&mut *transaction)
        .await?;
        if completed.rows_affected() == 0 {
            transaction.rollback().await?;
            let current_status: Option<String> =
                sqlx::query_scalar("SELECT status FROM cron_runs WHERE run_id = ?")
                    .bind(&claim.run_id)
                    .fetch_optional(&self.pool)
                    .await?;

            match current_status.as_deref() {
                Some("succeeded" | "failed" | "abandoned") => {
                    let status = current_status.unwrap();
                    tracing::info!(
                        run_id = %claim.run_id,
                        claim_token = %claim.claim_token,
                        status = %status,
                        "cron run {} already finalized as {} by reclaim; skipping duplicate completion",
                        claim.run_id,
                        status
                    );
                    return Ok(());
                }
                Some(other) => {
                    return Err(OmonError::Database(format!(
                        "cron claim {} for run {} is no longer active (status: {})",
                        claim.claim_token, claim.run_id, other
                    )));
                }
                None => {
                    return Err(OmonError::Database(format!(
                        "cron claim {} for run {} not found",
                        claim.claim_token, claim.run_id
                    )));
                }
            }
        }

        let mut payload: Value =
            serde_json::from_str(&claim.job.payload_json).unwrap_or_else(|_| serde_json::json!({}));
        let (times, completed_count) = increment_repeat_completed(&mut payload);
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "last_status".to_string(),
                Value::String("succeeded".to_string()),
            );
            obj.insert("last_run_at".to_string(), Value::String(now.to_rfc3339()));
            if let Some(occ) = claim.occurrence {
                obj.insert(
                    "last_occurrence".to_string(),
                    Value::String(occ.to_rfc3339()),
                );
            }
            obj.insert("last_error".to_string(), Value::Null);
            obj.insert("last_delivery_error".to_string(), Value::Null);
        }
        let updated_payload_json =
            serde_json::to_string(&payload).unwrap_or_else(|_| claim.job.payload_json.clone());
        let limit_reached = should_disable_after(completed_count, times);

        if claim.advance_schedule {
            if claim.job.expression.starts_with("once:") || limit_reached {
                sqlx::query(
                    "UPDATE cron_jobs
                     SET enabled = 0, next_run_at = NULL, payload_json = ?, updated_at = ?
                     WHERE id = ? AND expression = ? AND payload_json = ?",
                )
                .bind(&updated_payload_json)
                .bind(now)
                .bind(&claim.job.id)
                .bind(&claim.job.expression)
                .bind(&claim.job.payload_json)
                .execute(&mut *transaction)
                .await?;
            } else {
                let timezone = payload
                    .get("schedule")
                    .and_then(|schedule| schedule.get("timezone"))
                    .and_then(Value::as_str);
                let next = next_run_tz(&claim.job.expression, now, timezone)?;
                sqlx::query(
                    "UPDATE cron_jobs
                     SET next_run_at = ?, payload_json = ?, updated_at = ?
                     WHERE id = ? AND enabled = 1 AND expression = ? AND payload_json = ?",
                )
                .bind(next)
                .bind(&updated_payload_json)
                .bind(now)
                .bind(&claim.job.id)
                .bind(&claim.job.expression)
                .bind(&claim.job.payload_json)
                .execute(&mut *transaction)
                .await?;
            }
        } else {
            sqlx::query(
                "UPDATE cron_jobs
                 SET payload_json = ?, updated_at = ?
                 WHERE id = ? AND expression = ? AND payload_json = ?",
            )
            .bind(&updated_payload_json)
            .bind(now)
            .bind(&claim.job.id)
            .bind(&claim.job.expression)
            .bind(&claim.job.payload_json)
            .execute(&mut *transaction)
            .await?;
        }

        if let Some(out) = output {
            let profile = claim.job.profile();
            let _ = sqlx::query(
                "INSERT INTO cron_outputs (profile, job_id, run_id, output, created_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(profile, job_id, run_id) DO UPDATE SET output = excluded.output, created_at = excluded.created_at",
            )
            .bind(&profile)
            .bind(&claim.job.id)
            .bind(&claim.run_id)
            .bind(out)
            .bind(now.to_rfc3339())
            .execute(&mut *transaction)
            .await;
        }

        transaction.commit().await?;
        self.wake.notify_one();
        Ok(())
    }

    async fn complete_failure(&self, claim: &CronClaim, error: &OmonError) -> Result<()> {
        let now = (self.clock)();
        let mut transaction = self.pool.begin().await?;
        let completed = sqlx::query(
            "UPDATE cron_runs
             SET status = 'failed', completed_at = ?, lease_expires_at = ?, error = ?
             WHERE run_id = ? AND claim_token = ? AND status = 'running'",
        )
        .bind(now)
        .bind(now)
        .bind(error.to_string())
        .bind(&claim.run_id)
        .bind(&claim.claim_token)
        .execute(&mut *transaction)
        .await?;
        if completed.rows_affected() == 0 {
            transaction.rollback().await?;
            let current_status: Option<String> =
                sqlx::query_scalar("SELECT status FROM cron_runs WHERE run_id = ?")
                    .bind(&claim.run_id)
                    .fetch_optional(&self.pool)
                    .await?;

            match current_status.as_deref() {
                Some("succeeded" | "failed" | "abandoned") => {
                    let status = current_status.unwrap();
                    tracing::info!(
                        run_id = %claim.run_id,
                        claim_token = %claim.claim_token,
                        status = %status,
                        "cron run {} already finalized as {} by reclaim; skipping duplicate completion",
                        claim.run_id,
                        status
                    );
                    return Ok(());
                }
                Some(other) => {
                    return Err(OmonError::Database(format!(
                        "cron claim {} for run {} is no longer active (status: {})",
                        claim.claim_token, claim.run_id, other
                    )));
                }
                None => {
                    return Err(OmonError::Database(format!(
                        "cron claim {} for run {} not found",
                        claim.claim_token, claim.run_id
                    )));
                }
            }
        }

        let mut payload: Value =
            serde_json::from_str(&claim.job.payload_json).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "last_status".to_string(),
                Value::String("failed".to_string()),
            );
            obj.insert("last_run_at".to_string(), Value::String(now.to_rfc3339()));
            if let Some(occ) = claim.occurrence {
                obj.insert(
                    "last_occurrence".to_string(),
                    Value::String(occ.to_rfc3339()),
                );
            }
            let err_str = error.to_string();
            obj.insert("last_error".to_string(), Value::String(err_str.clone()));
            if err_str.to_lowercase().contains("deliver")
                || err_str.to_lowercase().contains("discord")
            {
                obj.insert("last_delivery_error".to_string(), Value::String(err_str));
            } else {
                obj.insert("last_delivery_error".to_string(), Value::Null);
            }
        }
        let (times, completed_count) = increment_repeat_completed(&mut payload);
        let limit_reached = should_disable_after(completed_count, times);

        let updated_payload_json =
            serde_json::to_string(&payload).unwrap_or_else(|_| claim.job.payload_json.clone());

        if claim.advance_schedule {
            if claim.job.expression.starts_with("once:") || limit_reached {
                sqlx::query(
                    "UPDATE cron_jobs
                     SET enabled = 0, next_run_at = NULL, payload_json = ?, updated_at = ?
                     WHERE id = ? AND expression = ? AND payload_json = ?",
                )
                .bind(&updated_payload_json)
                .bind(now)
                .bind(&claim.job.id)
                .bind(&claim.job.expression)
                .bind(&claim.job.payload_json)
                .execute(&mut *transaction)
                .await?;
            } else {
                let recent_statuses: Vec<String> = sqlx::query_scalar(
                    "SELECT status FROM cron_runs
                     WHERE job_id = ?
                     ORDER BY started_at DESC, run_id DESC
                     LIMIT 50",
                )
                .bind(&claim.job.id)
                .fetch_all(&mut *transaction)
                .await?;

                let consecutive_failures = recent_statuses
                    .iter()
                    .take_while(|status| status.as_str() == "failed")
                    .count() as u32;

                let next_result =
                    next_run_after_failure(&claim.job.expression, now, consecutive_failures);
                match next_result {
                    Ok(Some(next)) => {
                        sqlx::query(
                            "UPDATE cron_jobs
                             SET next_run_at = ?, payload_json = ?, updated_at = ?
                             WHERE id = ? AND enabled = 1 AND expression = ? AND payload_json = ?",
                        )
                        .bind(next)
                        .bind(&updated_payload_json)
                        .bind(now)
                        .bind(&claim.job.id)
                        .bind(&claim.job.expression)
                        .bind(&claim.job.payload_json)
                        .execute(&mut *transaction)
                        .await?;
                    }
                    Ok(None) => {
                        sqlx::query(
                            "UPDATE cron_jobs
                             SET enabled = 0, next_run_at = NULL, payload_json = ?, updated_at = ?
                             WHERE id = ? AND expression = ? AND payload_json = ?",
                        )
                        .bind(&updated_payload_json)
                        .bind(now)
                        .bind(&claim.job.id)
                        .bind(&claim.job.expression)
                        .bind(&claim.job.payload_json)
                        .execute(&mut *transaction)
                        .await?;
                    }
                    Err(calc_err) => {
                        tracing::error!(
                            %calc_err,
                            job_id = %claim.job.id,
                            "failed to compute next run after failure, disabling cron job"
                        );
                        sqlx::query(
                            "UPDATE cron_jobs
                             SET enabled = 0, next_run_at = NULL, payload_json = ?, updated_at = ?
                             WHERE id = ? AND expression = ? AND payload_json = ?",
                        )
                        .bind(&updated_payload_json)
                        .bind(now)
                        .bind(&claim.job.id)
                        .bind(&claim.job.expression)
                        .bind(&claim.job.payload_json)
                        .execute(&mut *transaction)
                        .await?;
                    }
                }
            }
        } else {
            sqlx::query(
                "UPDATE cron_jobs
                 SET payload_json = ?, updated_at = ?
                 WHERE id = ? AND expression = ? AND payload_json = ?",
            )
            .bind(&updated_payload_json)
            .bind(now)
            .bind(&claim.job.id)
            .bind(&claim.job.expression)
            .bind(&claim.job.payload_json)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        self.wake.notify_one();
        Ok(())
    }

    pub async fn execute_job(&self, job: &CronJob) -> Result<Option<String>> {
        let payload = job.payload()?;
        let destinations = delivery_destination(&payload)?;

        if self.dispatcher.is_none() {
            let has_explicit_discord =
                if let Ok(hermes) = serde_json::from_value::<crate::HermesJob>(payload.clone()) {
                    hermes.has_explicit_discord_destination()
                } else {
                    false
                };

            if has_explicit_discord {
                return Err(OmonError::Config(format!(
                    "unconfigured delivery transport: discord for job {}",
                    job.id
                )));
            }
        }

        let execution = self.executor.execute(job).await;

        match execution {
            Ok(result_content) => {
                let content = result_content
                    .or_else(|| {
                        payload
                            .get("notification")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .or_else(|| {
                        payload
                            .get("content")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    });
                if let Some(content_str) = &content {
                    if !destinations.is_empty() {
                        if !content_str.trim().is_empty() && !is_cron_silence_response(content_str)
                        {
                            let mut delivery_errors = Vec::new();
                            for destination in &destinations {
                                if let Err(err) = self.deliver(job, destination, content_str).await
                                {
                                    delivery_errors.push(err);
                                }
                            }
                            if !delivery_errors.is_empty() {
                                return Err(OmonError::Config(format!(
                                    "delivery failed for {} of {} destinations: {}",
                                    delivery_errors.len(),
                                    destinations.len(),
                                    delivery_errors[0]
                                )));
                            }
                            let ack_command = job.payload().ok().and_then(|payload| {
                                payload
                                    .get("ack_command")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                            });
                            if let Some(ack_command) =
                                ack_command.filter(|command| !command.trim().is_empty())
                            {
                                super::ack::run_ack_logged(&ack_command).await;
                            }
                        } else {
                            tracing::info!(
                                job_id = %job.id,
                                "cron job returned empty or silence response, suppressing delivery"
                            );
                        }
                    }
                }
                let _ = sqlx::query("DELETE FROM cron_incidents WHERE job_id = ?")
                    .bind(&job.id)
                    .execute(&self.pool)
                    .await;
                Ok(content)
            }
            Err(error) => {
                let signature = format!("{error}");
                let is_acknowledged: bool = sqlx::query_scalar(
                    "SELECT acknowledged FROM cron_incidents WHERE job_id = ? AND error_signature = ?",
                )
                .bind(&job.id)
                .bind(&signature)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten()
                .map(|ack: i64| ack != 0)
                .unwrap_or(false);

                if !is_acknowledged {
                    let now = chrono::Utc::now().to_rfc3339();
                    let _ = sqlx::query(
                        "INSERT INTO cron_incidents (job_id, error_signature, acknowledged, created_at, updated_at)
                         VALUES (?, ?, 0, ?, ?)
                         ON CONFLICT(job_id, error_signature) DO UPDATE SET updated_at = excluded.updated_at",
                    )
                    .bind(&job.id)
                    .bind(&signature)
                    .bind(&now)
                    .bind(&now)
                    .execute(&self.pool)
                    .await;

                    let failure_destinations = match job.payload() {
                        Ok(payload) => {
                            let hermes: std::result::Result<crate::HermesJob, _> =
                                serde_json::from_value(payload);
                            if let Ok(h) = hermes {
                                h.failure_destinations().unwrap_or_default()
                            } else {
                                destinations
                            }
                        }
                        Err(_) => destinations,
                    };
                    if !failure_destinations.is_empty() {
                        let content = format!("Cron job {} failed: {error}", job.id);
                        for destination in &failure_destinations {
                            if let Err(delivery_error) =
                                self.deliver(job, destination, &content).await
                            {
                                tracing::error!(
                                    %delivery_error,
                                    job_id = %job.id,
                                    "failed to deliver cron failure notification"
                                );
                            }
                        }
                    }
                } else {
                    tracing::info!(
                        job_id = %job.id,
                        %signature,
                        "cron failure matches acknowledged incident signature, suppressing alert notification"
                    );
                }
                Err(error)
            }
        }
    }

    async fn deliver(
        &self,
        job: &CronJob,
        destination: &crate::HermesOrigin,
        content: &str,
    ) -> Result<()> {
        let channel_id = destination.chat_id.parse::<u64>().map_err(|_| {
            OmonError::Config(format!(
                "invalid Discord channel ID: {}",
                destination.chat_id
            ))
        })?;
        let notification = CronNotification {
            job_id: job.id.clone(),
            channel_id,
            content: content.to_string(),
            triggered_at: (self.clock)(),
        };
        let _ = self.notifications.send(notification);
        let profile = job.profile();
        let mut session = job
            .session_key
            .as_deref()
            .and_then(|key| SessionKey::from_storage_key(key).ok())
            .filter(|parsed| {
                parsed.channel_id == destination.chat_id
                    && parsed.thread_id.as_deref() == destination.thread_id.as_deref()
                    && (destination.bot_id.is_none() || parsed.bot_id == destination.bot_id)
            })
            .unwrap_or_else(|| {
                let session = SessionKey::new(
                    "discord",
                    None::<String>,
                    destination.chat_id.clone(),
                    destination.thread_id.clone(),
                    destination
                        .user_id
                        .clone()
                        .unwrap_or_else(|| format!("cron:{}", job.id)),
                );
                match &destination.bot_id {
                    Some(bot_id) => session.with_bot_id(bot_id.clone()),
                    None => session,
                }
            });
        if session.bot_id.is_none() && !profile.is_empty() {
            session = session.with_bot_id(profile);
        }

        let obl_id = format!("obl:cron:{}:{}", job.id, (self.clock)().timestamp_millis());
        let ledger = crate::DeliveryLedgerService::new(self.pool.clone());
        ledger.record_obligation(&obl_id, &session, content).await?;
        ledger.mark_obligation_attempting(&obl_id).await?;

        let dispatch_result = if let Some(dispatcher) = &self.dispatcher {
            dispatcher
                .dispatch(OutboundAction::SendMessage {
                    session: session.clone(),
                    content: content.to_string(),
                    reply_to: None,
                })
                .await
        } else {
            Ok(())
        };

        if dispatch_result.is_ok() {
            ledger.mark_obligation_delivered(&obl_id).await?;
        } else if let Err(ref e) = dispatch_result {
            ledger
                .mark_obligation_failed(&obl_id, &e.to_string())
                .await?;
        }

        dispatch_result?;

        let attach_to_session = job
            .payload()
            .ok()
            .and_then(|p| p.get("attach_to_session").and_then(Value::as_bool))
            .unwrap_or(true);

        if attach_to_session {
            if let Err(error) = mirror_cron_delivery_to_session(
                &self.pool,
                &job.id,
                job.session_key.as_deref(),
                destination,
                content,
            )
            .await
            {
                tracing::warn!(
                    job_id = %job.id,
                    %error,
                    "failed to mirror cron delivery into session transcript"
                );
            }
        }

        Ok(())
    }
}

pub async fn mirror_cron_delivery_to_session(
    pool: &SqlitePool,
    job_id: &str,
    job_session_key: Option<&str>,
    destination: &crate::HermesOrigin,
    content: &str,
) -> Result<bool> {
    let target_session_key = if let Some(key) = job_session_key {
        let key_matches = if let Ok(parsed) = key.parse::<crate::SessionKey>() {
            parsed.channel_id == destination.chat_id
                && parsed.thread_id.as_deref() == destination.thread_id.as_deref()
                && (destination.bot_id.is_none() || parsed.bot_id == destination.bot_id)
        } else {
            false
        };

        if key_matches {
            let exists: Option<(String,)> =
                sqlx::query_as("SELECT session_key FROM sessions WHERE session_key = ? LIMIT 1")
                    .bind(key)
                    .fetch_optional(pool)
                    .await?;
            exists.map(|(k,)| k)
        } else {
            None
        }
    } else {
        None
    };

    let target_session_key = match target_session_key {
        Some(k) => Some(k),
        None => {
            let fallback_user = destination
                .user_id
                .clone()
                .unwrap_or_else(|| format!("cron:{job_id}"));
            let found = crate::mirror::find_session_by_origin(
                pool,
                &destination.platform,
                &destination.chat_id,
                destination.thread_id.as_deref(),
                Some(&fallback_user),
                destination.bot_id.as_deref(),
            )
            .await?;
            if found.is_some() {
                found
            } else {
                let fallback = SessionKey::new(
                    &destination.platform,
                    None::<String>,
                    destination.chat_id.clone(),
                    destination.thread_id.clone(),
                    fallback_user,
                );
                let fallback = match &destination.bot_id {
                    Some(bid) => fallback.with_bot_id(bid.clone()),
                    None => fallback,
                };
                Some(fallback.storage_key())
            }
        }
    };

    let Some(key) = target_session_key else {
        return Ok(false);
    };

    let label = format!("cron:{job_id}");
    crate::mirror::mirror_to_session(pool, &key, "assistant", content, Some(&label)).await
}

pub fn delivery_destinations(payload: &Value) -> Result<Vec<crate::HermesOrigin>> {
    if payload.get("schedule").is_some() {
        let job: HermesJob = serde_json::from_value(payload.clone())
            .map_err(|error| OmonError::Config(format!("invalid Hermes cron payload: {error}")))?;
        return job.discord_destinations();
    }

    let mut destinations = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(chat_id) = payload.get("channel_id").and_then(|value| {
        value
            .as_u64()
            .map(|value| value.to_string())
            .or_else(|| value.as_str().map(str::to_owned))
    }) {
        let chat_id = chat_id.trim().to_string();
        if !chat_id.is_empty() {
            let key = ("discord".to_string(), chat_id.clone(), None);
            seen.insert(key);
            destinations.push(crate::HermesOrigin {
                platform: "discord".into(),
                chat_id,
                ..crate::HermesOrigin::default()
            });
        }
    }

    let deliver_str = if let Some(s) = payload.get("deliver").and_then(Value::as_str) {
        Some(s.to_string())
    } else if let Some(arr) = payload.get("deliver").and_then(Value::as_array) {
        let parts: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
        Some(parts.join(","))
    } else {
        None
    };

    if let Some(deliver) = deliver_str {
        for part in deliver.split(',') {
            let part = part.trim();
            if part.is_empty() || part == "local" {
                continue;
            }
            if part == "origin" || part == "all" || part == "discord" {
                if let Some(origin_val) = payload.get("origin") {
                    if let Ok(origin) =
                        serde_json::from_value::<crate::HermesOrigin>(origin_val.clone())
                    {
                        if origin.platform.eq_ignore_ascii_case("discord")
                            && !origin.chat_id.is_empty()
                        {
                            let key = (
                                origin.platform.to_lowercase(),
                                origin.chat_id.clone(),
                                origin.thread_id.clone(),
                            );
                            if seen.insert(key) {
                                destinations.push(origin);
                            }
                        }
                    }
                }
            } else if let Some(channel) = part.strip_prefix("discord:") {
                let chat_id = channel.trim_start_matches('#').trim().to_string();
                if !chat_id.is_empty() {
                    let key = ("discord".to_string(), chat_id.clone(), None);
                    if seen.insert(key) {
                        destinations.push(crate::HermesOrigin {
                            platform: "discord".into(),
                            chat_id,
                            ..crate::HermesOrigin::default()
                        });
                    }
                }
            } else if part.contains(':') {
                // Unknown platform prefix (e.g. telegram:123) - skip gracefully
                continue;
            } else {
                let chat_id = part.trim_start_matches('#').trim().to_string();
                if !chat_id.is_empty() {
                    let key = ("discord".to_string(), chat_id.clone(), None);
                    if seen.insert(key) {
                        destinations.push(crate::HermesOrigin {
                            platform: "discord".into(),
                            chat_id,
                            ..crate::HermesOrigin::default()
                        });
                    }
                }
            }
        }
    }

    Ok(destinations)
}

pub fn delivery_destination(payload: &Value) -> Result<Vec<crate::HermesOrigin>> {
    delivery_destinations(payload)
}

pub const ONESHOT_GRACE_DURATION: Duration = Duration::from_secs(120);

pub fn next_run(expression: &str, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
    if let Some(timestamp) = expression.strip_prefix("once:") {
        let ts = DateTime::parse_from_rfc3339(timestamp)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|error| {
                OmonError::Config(format!("invalid one-shot timestamp `{timestamp}`: {error}"))
            })?;
        if ts >= after {
            return Ok(ts);
        }
        let past = after - ts;
        if past <= TimeDelta::from_std(ONESHOT_GRACE_DURATION).unwrap_or(TimeDelta::seconds(120)) {
            return Ok(ts);
        }
        return Err(OmonError::Config(format!(
            "one-shot timestamp `{timestamp}` is more than 120s in the past and cannot be scheduled"
        )));
    }
    if let Some(interval) = parse_interval(expression)? {
        let delta = TimeDelta::from_std(interval)
            .map_err(|_| OmonError::Config("cron interval is too large".into()))?;
        return Ok(after + delta);
    }
    let normalized = normalize_cron_expression(expression);
    let schedule = Schedule::from_str(&normalized).map_err(|error| {
        OmonError::Config(format!("invalid cron expression `{expression}`: {error}"))
    })?;
    schedule
        .after(&after)
        .next()
        .ok_or_else(|| OmonError::Config(format!("cron expression `{expression}` has no next run")))
}

/// Computes the next run instant for `expression`, evaluating cron wall-clock
/// fields in `timezone` (an IANA name) when provided and converting the result
/// back to UTC for storage. Interval and one-shot schedules are absolute
/// instants, so the source timezone does not shift them.
pub fn next_run_tz(
    expression: &str,
    after: DateTime<Utc>,
    timezone: Option<&str>,
) -> Result<DateTime<Utc>> {
    let Some(tz_name) = timezone.map(str::trim).filter(|name| !name.is_empty()) else {
        return next_run(expression, after);
    };
    let tz: chrono_tz::Tz = tz_name
        .parse()
        .map_err(|_| OmonError::Config(format!("invalid timezone `{tz_name}`")))?;
    if expression.starts_with("once:") || parse_interval(expression)?.is_some() {
        return next_run(expression, after);
    }
    let normalized = normalize_cron_expression(expression);
    let schedule = Schedule::from_str(&normalized).map_err(|error| {
        OmonError::Config(format!("invalid cron expression `{expression}`: {error}"))
    })?;
    schedule
        .after(&after.with_timezone(&tz))
        .next()
        .map(|instant| instant.with_timezone(&Utc))
        .ok_or_else(|| OmonError::Config(format!("cron expression `{expression}` has no next run")))
}

pub fn newest_missed_occurrence(
    expression: &str,
    scheduled_from: DateTime<Utc>,
    window_start: DateTime<Utc>,
    now: DateTime<Utc>,
    timezone: Option<&str>,
) -> Result<Option<DateTime<Utc>>> {
    if window_start > now {
        return Ok(None);
    }
    let effective_start = std::cmp::max(scheduled_from, window_start);

    if let Some(timestamp) = expression.strip_prefix("once:") {
        let ts = DateTime::parse_from_rfc3339(timestamp)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|error| {
                OmonError::Config(format!("invalid one-shot timestamp `{timestamp}`: {error}"))
            })?;
        if ts >= window_start && ts <= now && ts >= scheduled_from {
            return Ok(Some(ts));
        }
        return Ok(None);
    }

    if let Some(interval) = parse_interval(expression)? {
        let delta = TimeDelta::from_std(interval)
            .map_err(|_| OmonError::Config("cron interval is too large".into()))?;
        if scheduled_from > now {
            return Ok(None);
        }
        let diff = now.signed_duration_since(scheduled_from);
        let step_ms = delta.num_milliseconds();
        if step_ms <= 0 {
            return Ok(None);
        }
        let count = diff.num_milliseconds() / step_ms;
        let latest = scheduled_from + delta * (count as i32);
        if latest >= window_start && latest <= now {
            return Ok(Some(latest));
        }
        return Ok(None);
    }

    let normalized = normalize_cron_expression(expression);
    let schedule = Schedule::from_str(&normalized).map_err(|error| {
        OmonError::Config(format!("invalid cron expression `{expression}`: {error}"))
    })?;

    let tz_opt: Option<chrono_tz::Tz> = match timezone.map(str::trim).filter(|tz| !tz.is_empty()) {
        Some(tz_name) => Some(
            tz_name
                .parse()
                .map_err(|_| OmonError::Config(format!("invalid timezone `{tz_name}`")))?,
        ),
        None => None,
    };

    let mut latest = None;
    if let Some(tz) = tz_opt {
        let search_start = (effective_start - TimeDelta::seconds(1)).with_timezone(&tz);
        let now_local = now.with_timezone(&tz);
        for inst in schedule.after(&search_start) {
            if inst <= now_local {
                let inst_utc = inst.with_timezone(&Utc);
                if inst_utc >= window_start && inst_utc >= scheduled_from {
                    latest = Some(inst_utc);
                }
            } else {
                break;
            }
        }
    } else {
        let search_start = effective_start - TimeDelta::seconds(1);
        for inst in schedule.after(&search_start) {
            if inst <= now {
                if inst >= window_start && inst >= scheduled_from {
                    latest = Some(inst);
                }
            } else {
                break;
            }
        }
    }

    Ok(latest)
}

fn normalize_cron_expression(expression: &str) -> String {
    let parts: Vec<&str> = expression.split_whitespace().collect();
    if parts.len() == 5 {
        format!("0 {expression}")
    } else {
        expression.to_string()
    }
}

fn parse_interval(expression: &str) -> Result<Option<Duration>> {
    let value = expression
        .strip_prefix("interval:")
        .or_else(|| expression.strip_prefix("@every "))
        .or_else(|| expression.strip_prefix("every "));
    let Some(value) = value.map(str::trim) else {
        return Ok(None);
    };
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let amount: u64 = number
        .parse()
        .map_err(|_| OmonError::Config(format!("invalid interval expression `{expression}")))?;
    if amount == 0 {
        return Err(OmonError::Config(
            "cron interval must be greater than zero".into(),
        ));
    }
    let duration = match unit.trim() {
        "ms" => Duration::from_millis(amount),
        "s" | "sec" | "secs" => Duration::from_secs(amount),
        "m" | "min" | "mins" => Duration::from_secs(amount.saturating_mul(60)),
        "h" | "hr" | "hrs" => Duration::from_secs(amount.saturating_mul(3_600)),
        "d" | "day" | "days" => Duration::from_secs(amount.saturating_mul(86_400)),
        _ => {
            return Err(OmonError::Config(format!(
                "invalid interval unit in `{expression}`"
            )));
        }
    };
    Ok(Some(duration))
}

pub const MIN_RETRY_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_RETRY_INTERVAL: Duration = Duration::from_secs(3600);

pub fn failure_backoff_duration(consecutive_failures: u32) -> Duration {
    if consecutive_failures == 0 {
        return Duration::ZERO;
    }
    let exponent = (consecutive_failures - 1).min(10);
    let multiplier = 1u64.checked_shl(exponent).unwrap_or(1024);
    let seconds = (10u64.saturating_mul(multiplier)).clamp(10, 3600);
    Duration::from_secs(seconds)
}

pub fn next_run_after_failure(
    expression: &str,
    now: DateTime<Utc>,
    consecutive_failures: u32,
) -> Result<Option<DateTime<Utc>>> {
    if expression.starts_with("once:") {
        return Ok(None);
    }
    let scheduled_next = next_run(expression, now)?;
    let backoff = failure_backoff_duration(consecutive_failures);
    let backoff_delta = TimeDelta::from_std(backoff)
        .map_err(|_| OmonError::Config("backoff duration out of range".into()))?;
    let backoff_next = now + backoff_delta;
    Ok(Some(std::cmp::max(scheduled_next, backoff_next)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_backoff_scales_exponentially_and_clamps() {
        assert_eq!(failure_backoff_duration(0), Duration::ZERO);
        assert_eq!(failure_backoff_duration(1), Duration::from_secs(10));
        assert_eq!(failure_backoff_duration(2), Duration::from_secs(20));
        assert_eq!(failure_backoff_duration(3), Duration::from_secs(40));
        assert_eq!(failure_backoff_duration(4), Duration::from_secs(80));
        assert_eq!(failure_backoff_duration(5), Duration::from_secs(160));
        assert_eq!(failure_backoff_duration(6), Duration::from_secs(320));
        assert_eq!(failure_backoff_duration(7), Duration::from_secs(640));
        assert_eq!(failure_backoff_duration(8), Duration::from_secs(1280));
        assert_eq!(failure_backoff_duration(9), Duration::from_secs(2560));
        assert_eq!(failure_backoff_duration(10), Duration::from_secs(3600));
        assert_eq!(failure_backoff_duration(100), Duration::from_secs(3600));
    }

    #[test]
    fn next_run_tz_evaluates_wall_clock_and_tracks_dst_offset() {
        // Asia/Seoul has no DST: 09:00 local is always 00:00 UTC.
        let after_seoul = DateTime::parse_from_rfc3339("2026-09-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            next_run_tz("0 9 * * *", after_seoul, Some("Asia/Seoul")).unwrap(),
            DateTime::parse_from_rfc3339("2026-09-06T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );

        // America/New_York observes DST: noon local resolves to a different UTC
        // instant across the standard-time/daylight-time boundary.
        let winter = DateTime::parse_from_rfc3339("2026-01-10T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            next_run_tz("0 12 * * *", winter, Some("America/New_York")).unwrap(),
            DateTime::parse_from_rfc3339("2026-01-10T17:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            "noon EST is 17:00 UTC"
        );
        let summer = DateTime::parse_from_rfc3339("2026-07-10T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            next_run_tz("0 12 * * *", summer, Some("America/New_York")).unwrap(),
            DateTime::parse_from_rfc3339("2026-07-10T16:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            "noon EDT is 16:00 UTC"
        );

        // Absent/blank timezone and non-cron schedules fall back to UTC.
        assert_eq!(
            next_run_tz("0 12 * * *", winter, None).unwrap(),
            next_run("0 12 * * *", winter).unwrap()
        );
        assert_eq!(
            next_run_tz("interval:1h", winter, Some("America/New_York")).unwrap(),
            next_run("interval:1h", winter).unwrap()
        );

        // Malformed cron expression is rejected regardless of the timezone.
        assert!(next_run_tz("garbage", winter, Some("Asia/Seoul")).is_err());
    }

    #[test]
    fn next_run_after_failure_advances_interval_and_applies_backoff() {
        let now = DateTime::parse_from_rfc3339("2026-08-16T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // interval:1s with 1 failure -> backoff of 10s wins over 1s schedule
        let next1 = next_run_after_failure("interval:1s", now, 1)
            .unwrap()
            .unwrap();
        assert_eq!(next1, now + TimeDelta::seconds(10));

        // interval:1s with 2 failures -> backoff of 20s wins over 1s schedule
        let next2 = next_run_after_failure("interval:1s", now, 2)
            .unwrap()
            .unwrap();
        assert_eq!(next2, now + TimeDelta::seconds(20));

        // interval:1h with 1 failure -> schedule (1h) wins over backoff (10s)
        let next_1h = next_run_after_failure("interval:1h", now, 1)
            .unwrap()
            .unwrap();
        assert_eq!(next_1h, now + TimeDelta::hours(1));

        // once: expression returns None on failure (job should finish)
        let next_once = next_run_after_failure("once:2026-08-16T12:00:00Z", now, 1).unwrap();
        assert!(next_once.is_none());
    }

    #[test]
    fn next_run_after_failure_cron_expression_respects_schedule_and_backoff() {
        let now = DateTime::parse_from_rfc3339("2026-08-16T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // Cron running every minute (next schedule = 12:01:00 = 60s)
        // Failure 1 backoff is 10s, so schedule (60s) wins
        let next1 = next_run_after_failure("* * * * *", now, 1)
            .unwrap()
            .unwrap();
        assert_eq!(next1, now + TimeDelta::seconds(60));

        // Failure 5 backoff is 160s, so backoff (160s) wins over schedule (60s)
        let next5 = next_run_after_failure("* * * * *", now, 5)
            .unwrap()
            .unwrap();
        assert_eq!(next5, now + TimeDelta::seconds(160));
    }

    #[test]
    fn test_is_cron_silence_response_matches_sentinels_and_variants() {
        assert!(is_cron_silence_response(""));
        assert!(is_cron_silence_response("   \n  \t "));
        assert!(is_cron_silence_response("[SILENT]"));
        assert!(is_cron_silence_response("[silent]"));
        assert!(is_cron_silence_response("SILENT"));
        assert!(is_cron_silence_response("silent"));
        assert!(is_cron_silence_response("NO_REPLY"));
        assert!(is_cron_silence_response("no_reply"));
        assert!(is_cron_silence_response("NO REPLY"));
        assert!(is_cron_silence_response("no reply"));
        assert!(is_cron_silence_response("[SILENT] No updates to report"));
        assert!(is_cron_silence_response("[silent] no new issues found"));
        assert!(is_cron_silence_response("Everything checked\n\n[SILENT]"));
        assert!(is_cron_silence_response(
            "[SILENT]\nChecked 5 repos, all green"
        ));
        assert!(is_cron_silence_response("NO_REPLY\nProcessed batch"));
    }

    #[test]
    fn test_is_cron_silence_response_preserves_mid_sentence_content() {
        assert!(!is_cron_silence_response(
            "I considered staying [SILENT] but here is the summary of issues."
        ));
        assert!(!is_cron_silence_response("Silent retry succeeded"));
        assert!(!is_cron_silence_response("Found 3 new vulnerabilities"));
    }

    struct SilentDispatcher;

    #[async_trait]
    impl OutboundDispatcher for SilentDispatcher {
        async fn dispatch(&self, _action: OutboundAction) -> Result<()> {
            Ok(())
        }
    }

    struct SilentExecutor(Option<String>);

    #[async_trait]
    impl CronTaskExecutor for SilentExecutor {
        async fn execute(&self, _job: &CronJob) -> Result<Option<String>> {
            Ok(self.0.clone())
        }
    }

    #[tokio::test]
    async fn test_execute_job_suppresses_delivery_for_silent_and_empty_output() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();

        // 1. Executor returning None suppresses delivery
        let scheduler = CronScheduler::with_dispatcher(
            database.pool().clone(),
            Arc::new(SilentExecutor(None)),
            Arc::new(SilentDispatcher),
        );
        let mut notifications = scheduler.subscribe();
        let job = scheduler
            .register_job(
                "interval:1m",
                serde_json::json!({
                    "channel_id": 123456,
                }),
            )
            .await
            .unwrap();

        scheduler.execute_job(&job).await.unwrap();
        // Notification channel should have no message
        assert!(notifications.try_recv().is_err());

        // 2. Executor returning [SILENT] suppresses delivery
        let silent_scheduler = CronScheduler::with_dispatcher(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("[SILENT]".into()))),
            Arc::new(SilentDispatcher),
        );
        let mut silent_notifications = silent_scheduler.subscribe();
        silent_scheduler.execute_job(&job).await.unwrap();
        assert!(silent_notifications.try_recv().is_err());

        // 3. Executor returning non-empty report delivers
        let active_scheduler = CronScheduler::with_dispatcher(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("Report content".into()))),
            Arc::new(SilentDispatcher),
        );
        let mut active_notifications = active_scheduler.subscribe();
        active_scheduler.execute_job(&job).await.unwrap();
        let note = active_notifications.try_recv().unwrap();
        assert_eq!(note.content, "Report content");
    }

    #[tokio::test]
    async fn test_deliver_runs_ack_command_after_delivery() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let scheduler = CronScheduler::with_dispatcher(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("Report content".into()))),
            Arc::new(SilentDispatcher),
        );
        let marker = tempfile::tempdir().expect("tempdir");
        let marker_path = marker.path().join("ack-ran");
        let job = scheduler
            .register_job(
                "interval:1m",
                serde_json::json!({
                    "channel_id": 123456,
                    "ack_command": format!("touch '{}'", marker_path.to_string_lossy().replace('\\', "/")),
                }),
            )
            .await
            .unwrap();

        scheduler.execute_job(&job).await.unwrap();
        assert!(
            marker_path.exists(),
            "ack command must run after successful delivery"
        );
    }

    #[test]
    fn test_should_disable_after() {
        assert!(!should_disable_after(0, Some(2)));
        assert!(!should_disable_after(1, Some(2)));
        assert!(should_disable_after(2, Some(2)));
        assert!(should_disable_after(3, Some(2)));
        assert!(!should_disable_after(5, None));
        assert!(!should_disable_after(5, Some(0)));
    }

    #[test]
    fn test_increment_repeat_completed() {
        let mut payload = serde_json::json!({
            "repeat": {
                "times": 3,
                "completed": 1
            }
        });
        let (times, completed) = increment_repeat_completed(&mut payload);
        assert_eq!(times, Some(3));
        assert_eq!(completed, 2);
        assert_eq!(payload["repeat"]["completed"], 2);

        let mut payload_flat = serde_json::json!({
            "times": 2
        });
        let (times, completed) = increment_repeat_completed(&mut payload_flat);
        assert_eq!(times, Some(2));
        assert_eq!(completed, 1);
        assert_eq!(payload_flat["completed"], 1);
    }

    #[tokio::test]
    async fn test_repeat_times_disables_job_after_limit_reached() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let scheduler = CronScheduler::new(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("Run ok".into()))),
        );

        let job = scheduler
            .register(CronJobSpec::new(
                "interval:1m",
                serde_json::json!({
                    "channel_id": "123",
                    "repeat": {
                        "times": 2,
                        "completed": 0
                    }
                }),
            ))
            .await
            .unwrap();

        // 1. Run 1: claim, complete_success
        let claim1 = scheduler
            .claim_job(&job.id, false, true)
            .await
            .unwrap()
            .unwrap();
        scheduler.complete_success(&claim1, None).await.unwrap();

        let job_after_1 = scheduler.get(&job.id).await.unwrap().unwrap();
        assert!(job_after_1.enabled);
        assert!(job_after_1.next_run_at.is_some());
        let (times1, comp1) = extract_repeat_info(&job_after_1.payload().unwrap());
        assert_eq!(times1, Some(2));
        assert_eq!(comp1, 1);

        // 2. Run 2 (reaches limit of 2): claim, complete_success
        let claim2 = scheduler
            .claim_job(&job.id, false, true)
            .await
            .unwrap()
            .unwrap();
        scheduler.complete_success(&claim2, None).await.unwrap();

        let job_after_2 = scheduler.get(&job.id).await.unwrap().unwrap();
        assert!(
            !job_after_2.enabled,
            "Job must be disabled after hitting repeat.times limit"
        );
        assert!(
            job_after_2.next_run_at.is_none(),
            "Disabled job must clear next_run_at"
        );
        let (times2, comp2) = extract_repeat_info(&job_after_2.payload().unwrap());
        assert_eq!(times2, Some(2));
        assert_eq!(comp2, 2);
    }

    #[test]
    fn test_parse_context_from_ids() {
        assert_eq!(parse_context_from_ids(None), Vec::<String>::new());
        assert_eq!(
            parse_context_from_ids(Some(&serde_json::json!("job1"))),
            vec!["job1".to_string()]
        );
        assert_eq!(
            parse_context_from_ids(Some(&serde_json::json!(["job1", "job2", "invalid/../id"]))),
            vec!["job1".to_string(), "job2".to_string()]
        );
        assert_eq!(
            parse_context_from_ids(Some(&serde_json::json!({"not": "a list"}))),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_truncate_context_output() {
        let short = "Hello world";
        assert_eq!(truncate_context_output(short, 8000), "Hello world");

        let long = "a".repeat(8005);
        let truncated = truncate_context_output(&long, 8000);
        assert_eq!(
            truncated.len(),
            8000 + "\n\n[... output truncated ...]".len()
        );
        assert!(truncated.starts_with(&"a".repeat(8000)));
        assert!(truncated.ends_with("\n\n[... output truncated ...]"));
    }

    #[test]
    fn test_format_context_from_block() {
        let formatted = format_context_from_block("weather_job", "Temperature: 20C");
        let expected = "## Output from job 'weather_job'\nThe following is the most recent output from a preceding cron job. Use it as context for your analysis.\n\n```\nTemperature: 20C\n```";
        assert_eq!(formatted, expected);
    }

    #[tokio::test]
    async fn test_resolve_predecessor_output_from_db_and_disk() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();

        // 1. Resolve from DB messages
        let now = chrono::Utc::now();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json, created_at, updated_at) \
             VALUES ('discord:123:cron:job_alpha', 'discord', '123', 'cron:job_alpha', '{}', ?, ?)",
        )
        .bind(now)
        .bind(now)
        .execute(database.pool())
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) \
             VALUES ('msg1', 'discord:123:cron:job_alpha', 'assistant', 'Output from alpha', '{}', ?)",
        )
        .bind(now)
        .execute(database.pool())
        .await
        .unwrap();

        let resolved_db = resolve_predecessor_output(database.pool(), "", "job_alpha").await;
        assert_eq!(resolved_db.as_deref(), Some("Output from alpha"));
    }

    #[test]
    fn test_parse_wake_gate() {
        assert!(parse_wake_gate(""));
        assert!(parse_wake_gate("All systems normal\nChecked 10 records"));
        assert!(parse_wake_gate("{\"wakeAgent\": true}"));
        assert!(parse_wake_gate("wakeAgent: true"));
        assert!(parse_wake_gate("Checked logs\n{\"wakeAgent\": true}"));

        assert!(!parse_wake_gate("{\"wakeAgent\": false}"));
        assert!(!parse_wake_gate("{\"wake_agent\": false}"));
        assert!(!parse_wake_gate("wakeAgent: false"));
        assert!(!parse_wake_gate("wake_agent: false"));
        assert!(!parse_wake_gate("{\"wakeAgent\":false}"));
        assert!(!parse_wake_gate(
            "Running daily health check...\nFound no pending alerts.\n{\"wakeAgent\": false}"
        ));
        assert!(!parse_wake_gate(
            "Running daily health check...\nFound no pending alerts.\nwakeAgent: false"
        ));
    }

    #[test]
    fn test_delivery_fan_out_parsing() {
        // 1. Comma-separated discord channels
        let payload = serde_json::json!({
            "deliver": "discord:123,discord:456"
        });
        let targets = delivery_destination(&payload).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].chat_id, "123");
        assert_eq!(targets[1].chat_id, "456");

        // 2. Comma-separated with unknown platforms (e.g. telegram)
        let payload = serde_json::json!({
            "deliver": "discord:123,telegram:456,discord:789"
        });
        let targets = delivery_destination(&payload).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].chat_id, "123");
        assert_eq!(targets[1].chat_id, "789");

        // 3. origin and all with discord origin
        let payload = serde_json::json!({
            "id": "job1",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "deliver": "origin,all",
            "origin": {"platform": "discord", "chat_id": "999", "thread_id": "888"}
        });
        let targets = delivery_destination(&payload).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].chat_id, "999");
        assert_eq!(targets[0].thread_id.as_deref(), Some("888"));

        // 4. all with non-discord origin -> gracefully skipped
        let payload = serde_json::json!({
            "id": "job2",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "deliver": "all",
            "origin": {"platform": "telegram", "chat_id": "999"}
        });
        let targets = delivery_destination(&payload).unwrap();
        assert!(targets.is_empty());

        // 5. local delivery -> empty
        let payload = serde_json::json!({
            "deliver": "local"
        });
        let targets = delivery_destination(&payload).unwrap();
        assert!(targets.is_empty());

        // 6. Deduplication of explicit channel and origin
        let payload = serde_json::json!({
            "deliver": "discord:123,discord:123,123"
        });
        let targets = delivery_destination(&payload).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].chat_id, "123");
    }

    #[test]
    fn test_oneshot_grace_window() {
        let now = Utc::now();

        // Future one-shot: fires at future timestamp
        let future_ts = now + chrono::TimeDelta::seconds(300);
        let future_expr = format!("once:{}", future_ts.to_rfc3339());
        let res = next_run(&future_expr, now).unwrap();
        assert_eq!(res, future_ts);

        // 60s past: within 120s grace window -> fires (Ok)
        let past_60s = now - chrono::TimeDelta::seconds(60);
        let past_60s_expr = format!("once:{}", past_60s.to_rfc3339());
        let res = next_run(&past_60s_expr, now);
        assert!(
            res.is_ok(),
            "60s past one-shot must fire within 120s grace window"
        );

        // Exactly 120s past: boundary -> fires (Ok)
        let past_120s = now - chrono::TimeDelta::seconds(120);
        let past_120s_expr = format!("once:{}", past_120s.to_rfc3339());
        let res = next_run(&past_120s_expr, now);
        assert!(res.is_ok(), "120s past one-shot boundary must fire");

        // 200s past: older than 120s grace window -> rejected (Err)
        let past_200s = now - chrono::TimeDelta::seconds(200);
        let past_200s_expr = format!("once:{}", past_200s.to_rfc3339());
        let res = next_run(&past_200s_expr, now);
        assert!(
            res.is_err(),
            "200s past one-shot must be rejected outside grace window"
        );
    }

    #[tokio::test]
    async fn test_mirror_cron_delivery_to_session() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let now = Utc::now();

        // 1. Create a session for discord channel "chan_999"
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json, created_at, updated_at) \
             VALUES ('discord:chan_999:user1', 'discord', 'chan_999', 'user1', '{}', ?, ?)",
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        let destination = crate::HermesOrigin {
            platform: "discord".into(),
            chat_id: "chan_999".into(),
            ..crate::HermesOrigin::default()
        };

        // 2. Mirror a cron delivery into that session
        let mirrored = mirror_cron_delivery_to_session(
            pool,
            "job_daily_report",
            None,
            &destination,
            "Daily system report: all systems green.",
        )
        .await
        .unwrap();
        assert!(mirrored, "Must successfully mirror delivery into session");

        // 3. Verify message is persisted in messages table as assistant role
        let row: (String, String, String, String) = sqlx::query_as(
            "SELECT session_key, role, content, metadata_json FROM messages WHERE session_key = 'discord:chan_999:user1'",
        )
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(row.0, "discord:chan_999:user1");
        assert_eq!(row.1, "assistant");
        assert_eq!(row.2, "Daily system report: all systems green.");
        assert!(row.3.contains("job_daily_report"));

        // 4. Delivery to non-existent session channel -> returns Ok(false)
        let unknown_dest = crate::HermesOrigin {
            platform: "discord".into(),
            chat_id: "chan_nonexistent".into(),
            ..crate::HermesOrigin::default()
        };
        let not_mirrored = mirror_cron_delivery_to_session(
            pool,
            "job_daily_report",
            None,
            &unknown_dest,
            "Some content",
        )
        .await
        .unwrap();
        assert!(!not_mirrored, "Must return false for unknown channel");
    }

    #[test]
    fn test_should_reclaim_logic() {
        let live_pid = std::process::id();
        let dead_pid = 4_194_304; // definitely non-existent PID

        // 1. Not expired -> never reclaim
        assert!(!should_reclaim(Some(live_pid), false, TimeDelta::zero()));
        assert!(!should_reclaim(Some(dead_pid), false, TimeDelta::zero()));
        assert!(!should_reclaim(None, false, TimeDelta::zero()));
        assert!(!should_reclaim(
            Some(live_pid),
            false,
            TimeDelta::minutes(200)
        ));

        // 2. Expired + Alive PID (recent) -> false
        assert!(!should_reclaim(Some(live_pid), true, TimeDelta::minutes(5)));
        assert!(!should_reclaim(
            Some(live_pid),
            true,
            TimeDelta::minutes(60)
        ));

        // 3. Expired + Dead PID -> true
        assert!(should_reclaim(Some(dead_pid), true, TimeDelta::minutes(5)));

        // 4. Expired + NULL / 0 owner_pid -> true (legacy rows)
        assert!(should_reclaim(None, true, TimeDelta::minutes(5)));
        assert!(should_reclaim(Some(0), true, TimeDelta::minutes(5)));

        // 5. Expired + Alive PID + Stale past safety net (>= 120m) -> true
        assert!(should_reclaim(Some(live_pid), true, STALE_LEASE_SAFETY_NET));
        assert!(should_reclaim(
            Some(live_pid),
            true,
            TimeDelta::minutes(150)
        ));

        // 6. Test with custom is_alive predicate
        assert!(!should_reclaim_with(
            Some(1234),
            true,
            TimeDelta::minutes(10),
            |_| true
        ));
        assert!(should_reclaim_with(
            Some(1234),
            true,
            TimeDelta::minutes(10),
            |_| false
        ));
        assert!(should_reclaim_with(
            Some(1234),
            true,
            STALE_LEASE_SAFETY_NET,
            |_| true
        ));
        assert!(should_reclaim_with(
            None,
            true,
            TimeDelta::minutes(10),
            |_| true
        ));
        assert!(!should_reclaim_with(
            Some(1234),
            false,
            TimeDelta::minutes(200),
            |_| false
        ));
    }

    #[tokio::test]
    async fn test_claim_job_stores_owner_pid_and_reclaim_skips_live_process() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let scheduler = CronScheduler::new(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("done".into()))),
        );

        let job = scheduler
            .register(CronJobSpec::new(
                "interval:1h",
                serde_json::json!({"channel_id": "test"}),
            ))
            .await
            .unwrap();

        // 1. Claim job: check that owner_pid is set to current process ID
        let claim = scheduler
            .claim_job(&job.id, false, false)
            .await
            .unwrap()
            .expect("should claim job");

        let (owner_pid, status): (Option<i64>, String) =
            sqlx::query_as("SELECT owner_pid, status FROM cron_runs WHERE run_id = ?")
                .bind(&claim.run_id)
                .fetch_one(database.pool())
                .await
                .unwrap();

        assert_eq!(owner_pid, Some(std::process::id() as i64));
        assert_eq!(status, "running");

        // 2. Expire the lease artificially (10 seconds ago)
        let expired_at = Utc::now() - TimeDelta::seconds(10);
        sqlx::query("UPDATE cron_runs SET lease_expires_at = ? WHERE run_id = ?")
            .bind(expired_at)
            .bind(&claim.run_id)
            .execute(database.pool())
            .await
            .unwrap();

        // 3. Attempting to claim again: owner process is still alive and lease_age < safety net,
        // so reclaim must SKIP this row (it stays 'running'), and cannot claim a duplicate.
        let second_claim = scheduler.claim_job(&job.id, false, false).await.unwrap();
        assert!(
            second_claim.is_none(),
            "Must not claim while live process is running"
        );

        let status_after: String =
            sqlx::query_scalar("SELECT status FROM cron_runs WHERE run_id = ?")
                .bind(&claim.run_id)
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(
            status_after, "running",
            "Live process run must not be reclaimed as failed"
        );

        // 4. Now simulate dead owner PID
        let dead_pid = 4_194_304i64;
        sqlx::query("UPDATE cron_runs SET owner_pid = ? WHERE run_id = ?")
            .bind(dead_pid)
            .bind(&claim.run_id)
            .execute(database.pool())
            .await
            .unwrap();

        // 5. Attempt claim again: dead PID -> reclaim TAKES the row (marks failed) and claims a new run
        let third_claim = scheduler
            .claim_job(&job.id, false, false)
            .await
            .unwrap()
            .expect("should reclaim dead owner run and claim new run");
        assert_ne!(third_claim.run_id, claim.run_id);

        let (old_status, old_error): (String, Option<String>) =
            sqlx::query_as("SELECT status, error FROM cron_runs WHERE run_id = ?")
                .bind(&claim.run_id)
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(old_status, "failed");
        assert_eq!(
            old_error.as_deref(),
            Some("lease expired before completion")
        );
    }

    #[tokio::test]
    async fn test_idempotent_completion_when_already_finalized() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let scheduler = CronScheduler::new(
            database.pool().clone(),
            Arc::new(SilentExecutor(Some("done".into()))),
        );

        let job = scheduler
            .register(CronJobSpec::new(
                "interval:1h",
                serde_json::json!({"channel_id": "test"}),
            ))
            .await
            .unwrap();

        let claim = scheduler
            .claim_job(&job.id, false, false)
            .await
            .unwrap()
            .expect("should claim job");

        // 1. Simulate that reclaimer marked this run as failed
        sqlx::query(
            "UPDATE cron_runs SET status = 'failed', completed_at = ?, error = 'lease expired before completion' WHERE run_id = ?",
        )
        .bind(Utc::now())
        .bind(&claim.run_id)
        .execute(database.pool())
        .await
        .unwrap();

        // 2. complete_failure on already finalized run returns Ok(())
        let res_fail = scheduler
            .complete_failure(&claim, &OmonError::ToolExecution("late error".into()))
            .await;
        assert!(
            res_fail.is_ok(),
            "complete_failure must be idempotent w.r.t reclaim"
        );

        // 3. complete_success on already finalized run also returns Ok(())
        let res_succ = scheduler.complete_success(&claim, None).await;
        assert!(
            res_succ.is_ok(),
            "complete_success must be idempotent w.r.t reclaim"
        );

        // 4. Genuine missing run returns Err
        let fake_claim = CronClaim {
            run_id: uuid::Uuid::new_v4().to_string(),
            claim_token: uuid::Uuid::new_v4().to_string(),
            job: claim.job.clone(),
            advance_schedule: false,
            occurrence: None,
        };
        let res_missing = scheduler.complete_success(&fake_claim, None).await;
        assert!(res_missing.is_err(), "missing run must return error");
    }

    struct RecordingDispatcher {
        sessions: Arc<Mutex<Vec<SessionKey>>>,
    }

    #[async_trait]
    impl OutboundDispatcher for RecordingDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<()> {
            if let OutboundAction::SendMessage { session, .. } = action {
                self.sessions.lock().await.push(session);
            }
            Ok(())
        }
    }

    async fn insert_session(pool: &SqlitePool, key: &SessionKey) {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, '{}', ?, ?)",
        )
        .bind(key.storage_key())
        .bind(&key.platform)
        .bind(&key.channel_id)
        .bind(&key.user_id)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_fan_out_dispatches_each_destination_to_its_own_channel() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let stored = SessionKey::new("discord", None::<String>, "111", None::<String>, "cron");
        insert_session(pool, &stored).await;

        let dispatched = Arc::new(Mutex::new(Vec::new()));
        let scheduler = CronScheduler::with_dispatcher(
            pool.clone(),
            Arc::new(SilentExecutor(Some("Report content".into()))),
            Arc::new(RecordingDispatcher {
                sessions: dispatched.clone(),
            }),
        );

        let mut spec = CronJobSpec::new(
            "interval:1m",
            serde_json::json!({"deliver": "discord:111,discord:222"}),
        );
        spec.session_key = Some(stored.storage_key());
        let job = scheduler.register(spec).await.unwrap();

        scheduler.execute_job(&job).await.unwrap();

        let channels: Vec<String> = dispatched
            .lock()
            .await
            .iter()
            .map(|session| session.channel_id.clone())
            .collect();
        assert_eq!(channels, vec!["111".to_string(), "222".to_string()]);
    }

    #[tokio::test]
    async fn test_register_with_id_updates_session_key_on_conflict() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();
        let old_session = SessionKey::new("discord", None::<String>, "111", None::<String>, "cron");
        let new_session = SessionKey::new("discord", None::<String>, "222", None::<String>, "cron");
        insert_session(pool, &old_session).await;
        insert_session(pool, &new_session).await;

        let scheduler =
            CronScheduler::new(pool.clone(), Arc::new(SilentExecutor(Some("done".into()))));

        let mut spec = CronJobSpec::new("interval:1m", serde_json::json!({"channel_id": "111"}));
        spec.session_key = Some(old_session.storage_key());
        scheduler
            .register_with_id("job_reroute", spec)
            .await
            .unwrap();

        let mut updated = CronJobSpec::new("interval:1m", serde_json::json!({"channel_id": "222"}));
        updated.session_key = Some(new_session.storage_key());
        let job = scheduler
            .register_with_id("job_reroute", updated)
            .await
            .unwrap();

        assert_eq!(
            job.session_key.as_deref(),
            Some(new_session.storage_key().as_str()),
            "re-registering a job with a new session_key must update the stored session_key"
        );
    }

    #[tokio::test]
    async fn test_brief_file_read_and_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let brief_path = temp.path().join("daily_brief.md");
        std::fs::write(
            &brief_path,
            "# Daily Brief\nCheck system status and report.",
        )
        .unwrap();

        let database = crate::Database::connect("sqlite::memory:").await.unwrap();

        struct MockBackend {
            prompts: Arc<std::sync::Mutex<Vec<String>>>,
        }

        #[async_trait]
        impl crate::agent::AgentBackend for MockBackend {
            async fn run(
                &self,
                _session: &mut crate::SessionContext,
                event: crate::InboundEvent,
            ) -> Result<()> {
                self.prompts.lock().unwrap().push(event.content);
                Ok(())
            }
        }

        let prompts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let backend = Arc::new(MockBackend {
            prompts: prompts.clone(),
        });

        let executor = crate::cron::AgentCronExecutor {
            backend: backend.clone(),
            workspace_root: temp.path().to_path_buf(),
            pool: database.pool().clone(),
            cron_script_timeout_secs: 30,
        };

        // 1. brief_file read at runtime
        let job_with_brief = CronJob {
            id: "brief_job".into(),
            session_key: None,
            expression: "0 9 * * *".into(),
            payload_json: serde_json::json!({
                "id": "brief_job",
                "brief_file": brief_path.to_str().unwrap(),
                "prompt": "fallback prompt that should be ignored",
                "schedule": {"kind": "cron", "expr": "0 9 * * *"},
                "deliver": "local"
            })
            .to_string(),
            enabled: true,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            authority: "omon_owned".into(),
        };

        executor.execute(&job_with_brief).await.unwrap();
        let last_prompt = prompts.lock().unwrap().pop().unwrap();
        assert!(
            last_prompt.contains("# Daily Brief\nCheck system status and report."),
            "executor must read brief_file at runtime as prompt, got: {last_prompt}"
        );
        assert!(
            !last_prompt.contains("fallback prompt that should be ignored"),
            "prompt must not contain fallback when brief_file is present"
        );

        // 2. Fallback to prompt when brief_file is absent
        let job_fallback = CronJob {
            id: "fallback_job".into(),
            session_key: None,
            expression: "0 9 * * *".into(),
            payload_json: serde_json::json!({
                "id": "fallback_job",
                "prompt": "direct prompt fallback text",
                "schedule": {"kind": "cron", "expr": "0 9 * * *"},
                "deliver": "local"
            })
            .to_string(),
            enabled: true,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            authority: "omon_owned".into(),
        };

        executor.execute(&job_fallback).await.unwrap();
        let fallback_prompt = prompts.lock().unwrap().pop().unwrap();
        assert!(
            fallback_prompt.contains("direct prompt fallback text"),
            "executor must fall back to prompt when brief_file is absent"
        );

        // 3. Load validation: nonexistent file error
        let non_existent = temp.path().join("missing.md");
        let hermes_missing: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "missing_job",
            "brief_file": non_existent.to_str().unwrap(),
            "schedule": {"kind": "cron", "expr": "0 9 * * *"}
        }))
        .unwrap();
        let err = hermes_missing.validate(None, Utc::now()).unwrap_err();
        assert!(
            err.contains("brief_file does not exist"),
            "missing brief_file must fail validation: {err}"
        );

        // 4. Load validation: lifecycle guard error in brief_file
        let evil_path = temp.path().join("evil.md");
        std::fs::write(&evil_path, "systemctl restart omon-gateway").unwrap();
        let hermes_evil: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "evil_job",
            "brief_file": evil_path.to_str().unwrap(),
            "schedule": {"kind": "cron", "expr": "0 9 * * *"}
        }))
        .unwrap();
        let evil_err = hermes_evil.validate(None, Utc::now()).unwrap_err();
        assert!(
            evil_err.contains("gateway lifecycle violation"),
            "lifecycle violation in brief_file must fail validation: {evil_err}"
        );
    }

    #[tokio::test]
    async fn test_catch_up_single_fire_after_simulated_downtime() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();

        struct CountingExecutor {
            count: Arc<std::sync::Mutex<usize>>,
        }
        #[async_trait]
        impl CronTaskExecutor for CountingExecutor {
            async fn execute(&self, _job: &CronJob) -> Result<Option<String>> {
                let mut guard = self.count.lock().unwrap();
                *guard += 1;
                Ok(Some("catch-up execution completed".into()))
            }
        }

        let exec_count = Arc::new(std::sync::Mutex::new(0));
        let executor = Arc::new(CountingExecutor {
            count: exec_count.clone(),
        });
        let scheduler =
            CronScheduler::with_poll_interval(pool.clone(), executor, Duration::from_secs(1));

        let now = Utc::now();
        // Register hourly job with 72h catch-up window
        let job_payload = serde_json::json!({
            "id": "catch_up_hourly",
            "name": "Catch-up Hourly",
            "prompt": "hourly task",
            "catch_up_hours": 72,
            "schedule": {
                "kind": "cron",
                "expr": "0 * * * *"
            }
        });
        let job = scheduler
            .register_with_id(
                "catch_up_hourly",
                CronJobSpec::new("0 * * * *", job_payload),
            )
            .await
            .unwrap();

        // Simulate 2 days of downtime (48 hours overdue)
        let two_days_ago = now - TimeDelta::days(2);
        sqlx::query("UPDATE cron_jobs SET next_run_at = ? WHERE id = ?")
            .bind(two_days_ago)
            .bind(&job.id)
            .execute(pool)
            .await
            .unwrap();

        // Poll 1: Exactly ONE catch-up run fires for the simulated 2-day-old job
        let claimed = scheduler.run_due_jobs().await.unwrap();
        assert_eq!(
            claimed, 1,
            "Exactly ONE catch-up run must fire for simulated 2-day-old job"
        );
        scheduler.wait_idle().await;

        assert_eq!(
            *exec_count.lock().unwrap(),
            1,
            "Executor must have executed exactly ONCE"
        );

        // Verify cron_runs table recorded exactly ONE run
        let run_count: i64 = sqlx::query_scalar("SELECT count(*) FROM cron_runs WHERE job_id = ?")
            .bind(&job.id)
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(run_count, 1, "cron_runs table must contain exactly ONE run");

        // Verify next_run_at advanced to the future and last_occurrence was recorded
        let updated = scheduler.get(&job.id).await.unwrap().unwrap();
        assert!(
            updated.next_run_at.unwrap() > now,
            "next_run_at must advance to future occurrence"
        );
        let payload = updated.payload().unwrap();
        assert!(
            payload.get("last_occurrence").is_some(),
            "payload must persist last_occurrence for occurrence-keyed dedup"
        );

        // Poll 2: Immediate subsequent poll must claim ZERO runs (occurrence-keyed dedup + future schedule)
        let second_claim = scheduler.run_due_jobs().await.unwrap();
        assert_eq!(
            second_claim, 0,
            "No duplicate run may fire on subsequent poll"
        );
        assert_eq!(
            *exec_count.lock().unwrap(),
            1,
            "Execution count must remain 1"
        );
    }

    #[tokio::test]
    async fn test_thread_destination_parse_and_deliver() {
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool();

        let payload = serde_json::json!({
            "id": "thread_dest_job",
            "prompt": "status update",
            "deliver": "origin",
            "origin": {
                "platform": "discord",
                "chat_id": "11223344",
                "thread_id": "99887766"
            },
            "schedule": {
                "kind": "cron",
                "expr": "0 * * * *"
            }
        });

        // 1. Verify parser resolves thread_id from origin
        let destinations = delivery_destinations(&payload).unwrap();
        assert_eq!(
            destinations.len(),
            1,
            "Must resolve exactly one destination"
        );
        assert_eq!(destinations[0].platform, "discord");
        assert_eq!(destinations[0].chat_id, "11223344");
        assert_eq!(destinations[0].thread_id.as_deref(), Some("99887766"));

        // 2. Verify delivery routes to the thread
        let dispatched = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let dispatcher = Arc::new(RecordingDispatcher {
            sessions: dispatched.clone(),
        });
        let executor = Arc::new(SilentExecutor(Some("thread delivery content".into())));
        let scheduler = CronScheduler::with_dispatcher(pool.clone(), executor, dispatcher.clone());

        let cron_job = CronJob {
            id: "thread_dest_job".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: payload.to_string(),
            enabled: true,
            next_run_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            authority: "omon_owned".into(),
        };

        let result = scheduler.execute_job(&cron_job).await;
        assert!(result.is_ok(), "execute_job must succeed: {result:?}");

        let sessions = dispatched.lock().await;
        assert_eq!(sessions.len(), 1, "Must dispatch exactly one message");
        assert_eq!(sessions[0].platform, "discord");
        assert_eq!(sessions[0].channel_id, "11223344");
        assert_eq!(
            sessions[0].thread_id.as_deref(),
            Some("99887766"),
            "session key must carry the resolved discord thread_id"
        );
    }

    #[test]
    fn test_timezone_validation_accepts_and_refuses() {
        let fixed_now = DateTime::parse_from_rfc3339("2026-10-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // 1. "Asia/Seoul" accepted
        let job_seoul: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "tz_seoul",
            "prompt": "run",
            "schedule": {
                "kind": "cron",
                "expr": "0 9 * * *",
                "timezone": "Asia/Seoul"
            }
        }))
        .unwrap();
        let validated_seoul = job_seoul.validate(None, fixed_now);
        assert!(
            validated_seoul.is_ok(),
            "Asia/Seoul must be accepted: {validated_seoul:?}"
        );
        let val_seoul = validated_seoul.unwrap();
        assert_eq!(val_seoul.effective_timezone.as_deref(), Some("Asia/Seoul"));
        assert!(val_seoul.computed_next.is_some());

        // 2. "UTC" accepted
        let job_utc: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "tz_utc",
            "prompt": "run",
            "schedule": {
                "kind": "cron",
                "expr": "0 9 * * *",
                "timezone": "UTC"
            }
        }))
        .unwrap();
        let validated_utc = job_utc.validate(None, fixed_now);
        assert!(
            validated_utc.is_ok(),
            "UTC must be accepted: {validated_utc:?}"
        );
        let val_utc = validated_utc.unwrap();
        assert_eq!(val_utc.effective_timezone.as_deref(), Some("UTC"));
        assert!(val_utc.computed_next.is_some());

        // 3. Uncomputable / invalid timezone refused at load time
        let job_bad_tz: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "tz_bad",
            "prompt": "run",
            "schedule": {
                "kind": "cron",
                "expr": "0 9 * * *",
                "timezone": "Mars/Olympus"
            }
        }))
        .unwrap();
        let err_tz = job_bad_tz.validate(None, fixed_now);
        assert!(
            err_tz.is_err(),
            "Invalid timezone must be refused as load-time error"
        );
        let msg = err_tz.unwrap_err();
        assert!(
            msg.contains("invalid timezone `Mars/Olympus`"),
            "Expected timezone error, got: {msg}"
        );

        // 4. Uncomputable cron expression refused at load time
        let job_bad_expr: HermesJob = serde_json::from_value(serde_json::json!({
            "id": "expr_bad",
            "prompt": "run",
            "schedule": {
                "kind": "cron",
                "expr": "not a valid cron expression",
                "timezone": "UTC"
            }
        }))
        .unwrap();
        let err_expr = job_bad_expr.validate(None, fixed_now);
        assert!(
            err_expr.is_err(),
            "Uncomputable cron expression must be refused at load time"
        );
        let msg_expr = err_expr.unwrap_err();
        assert!(
            msg_expr.contains("uncomputable schedule"),
            "Expected uncomputable error, got: {msg_expr}"
        );
    }
}

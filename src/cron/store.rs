use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::{OmonError, Result};

const SOURCE_KEY: &str = "_omon_hermes_source";
pub const DEFAULT_CRON_RUNS_RETENTION_DAYS: i64 = 14;
pub const DEFAULT_CRON_SCRIPT_TIMEOUT_SECS: u64 = 1800;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CronAuthority {
    HermesMirror,
    CutoverPending,
    OmonOwned,
}

impl CronAuthority {
    pub const HERMES_MIRROR: &'static str = "hermes_mirror";
    pub const CUTOVER_PENDING: &'static str = "cutover_pending";
    pub const OMON_OWNED: &'static str = "omon_owned";

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HermesMirror => Self::HERMES_MIRROR,
            Self::CutoverPending => Self::CUTOVER_PENDING,
            Self::OmonOwned => Self::OMON_OWNED,
        }
    }
}

impl std::fmt::Display for CronAuthority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CronAuthority {
    type Err = OmonError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "hermes_mirror" => Ok(Self::HermesMirror),
            "cutover_pending" => Ok(Self::CutoverPending),
            "omon_owned" => Ok(Self::OmonOwned),
            _ => Err(OmonError::Config(format!("unknown cron authority: {s}"))),
        }
    }
}

pub async fn update_cron_authority(
    pool: &SqlitePool,
    id: &str,
    authority: CronAuthority,
) -> Result<bool> {
    let result = sqlx::query("UPDATE cron_jobs SET authority = ?, updated_at = ? WHERE id = ?")
        .bind(authority.as_str())
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_cron_authority(pool: &SqlitePool, id: &str) -> Result<Option<CronAuthority>> {
    let authority: Option<String> =
        sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    authority.map(|a| a.parse()).transpose()
}

pub fn cron_script_timeout_secs_from(raw: Option<&str>) -> u64 {
    raw.and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(DEFAULT_CRON_SCRIPT_TIMEOUT_SECS)
}

pub fn resolve_cron_script_timeout(
    job_override: Option<u64>,
    global_default: u64,
) -> std::time::Duration {
    let secs = job_override
        .filter(|&secs| secs > 0)
        .unwrap_or(if global_default > 0 {
            global_default
        } else {
            DEFAULT_CRON_SCRIPT_TIMEOUT_SECS
        });
    std::time::Duration::from_secs(secs)
}

pub fn cron_runs_retention_days_from_environment() -> Result<i64> {
    let Some(value) = std::env::var_os("CRON_RUNS_RETENTION_DAYS") else {
        return Ok(DEFAULT_CRON_RUNS_RETENTION_DAYS);
    };
    let value = value.to_string_lossy();
    let days = value.parse::<i64>().map_err(|_| {
        OmonError::Config(format!(
            "CRON_RUNS_RETENTION_DAYS must be a non-negative integer, got `{value}`"
        ))
    })?;
    if days < 0 {
        return Err(OmonError::Config(format!(
            "CRON_RUNS_RETENTION_DAYS must be a non-negative integer, got `{value}`"
        )));
    }
    Ok(days)
}

pub async fn prune_terminal_cron_runs(
    pool: &SqlitePool,
    retention_days: i64,
    now: DateTime<Utc>,
) -> Result<u64> {
    if retention_days < 0 {
        return Err(OmonError::Config(
            "cron run retention days must be non-negative".into(),
        ));
    }
    let cutoff = now - chrono::TimeDelta::days(retention_days);
    let result = sqlx::query(
        "DELETE FROM cron_runs
         WHERE status IN ('succeeded', 'failed')
           AND COALESCE(completed_at, started_at) < ?",
    )
    .bind(cutoff)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HermesSchedule {
    pub kind: String,
    #[serde(default)]
    pub expr: Option<String>,
    #[serde(default)]
    pub minutes: Option<u64>,
    #[serde(default)]
    pub run_at: Option<String>,
    #[serde(default)]
    pub display: Option<String>,
    /// IANA source timezone the recurrence is anchored to. Persisted so the
    /// scheduler evaluates wall-clock cron fields there before converting the
    /// next instant to UTC for storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrNum {
        Str(String),
        Num(u64),
        SignedNum(i64),
    }

    match StrOrNum::deserialize(deserializer)? {
        StrOrNum::Str(s) => Ok(s),
        StrOrNum::Num(n) => Ok(n.to_string()),
        StrOrNum::SignedNum(n) => Ok(n.to_string()),
    }
}

fn deserialize_opt_string_or_number<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrNum {
        Str(String),
        Num(u64),
        SignedNum(i64),
    }

    match Option::<StrOrNum>::deserialize(deserializer)? {
        Some(StrOrNum::Str(s)) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        Some(StrOrNum::Num(n)) => Ok(Some(n.to_string())),
        Some(StrOrNum::SignedNum(n)) => Ok(Some(n.to_string())),
        None => Ok(None),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HermesOrigin {
    pub platform: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub chat_id: String,
    #[serde(default, deserialize_with = "deserialize_opt_string_or_number")]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub chat_name: Option<String>,
    #[serde(default)]
    pub bot_id: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HermesRepeat {
    #[serde(default)]
    pub times: Option<u64>,
    #[serde(default)]
    pub completed: u64,
}

fn deserialize_deliver<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DeliverInput {
        Single(String),
        Multiple(Vec<String>),
    }

    match Option::<DeliverInput>::deserialize(deserializer)? {
        Some(DeliverInput::Single(s)) => {
            let items: Vec<String> = s
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            if items.is_empty() {
                Ok(None)
            } else {
                Ok(Some(items))
            }
        }
        Some(DeliverInput::Multiple(list)) => {
            let mut items = Vec::new();
            for item in list {
                for part in item.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        items.push(trimmed.to_string());
                    }
                }
            }
            if items.is_empty() {
                Ok(None)
            } else {
                Ok(Some(items))
            }
        }
        None => Ok(None),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HermesJob {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub no_agent: bool,
    /// Shell command the gateway runs after this job's output was
    /// successfully delivered. Keeps checkpoint commit independent of the
    /// agent choosing to run an ack script.
    #[serde(default)]
    pub ack_command: Option<String>,
    #[serde(default)]
    pub context_from: Option<Value>,
    pub schedule: HermesSchedule,
    #[serde(default)]
    pub schedule_display: String,
    #[serde(default)]
    pub repeat: HermesRepeat,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_delivery_error: Option<String>,
    #[serde(default, deserialize_with = "deserialize_deliver")]
    pub deliver: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_deliver")]
    pub failure_deliver: Option<Vec<String>>,
    #[serde(default)]
    pub origin: Option<HermesOrigin>,
    #[serde(default)]
    pub enabled_toolsets: Option<Vec<String>>,
    #[serde(default)]
    pub workdir: Option<PathBuf>,
    #[serde(default)]
    pub attach_to_session: Option<bool>,
    #[serde(
        default,
        alias = "timeout",
        alias = "timeout_seconds",
        alias = "script_timeout",
        alias = "script_timeout_seconds"
    )]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub brief_file: Option<PathBuf>,
    #[serde(default = "default_catch_up_hours")]
    pub catch_up_hours: Option<u64>,
    #[serde(default)]
    pub monitor_script: Option<String>,
    #[serde(default)]
    pub monitor_url: Option<String>,
    #[serde(default)]
    pub monitor_state: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn default_catch_up_hours() -> Option<u64> {
    Some(6)
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHermesJob {
    pub expression: String,
    pub effective_timezone: Option<String>,
    pub computed_next: Option<DateTime<Utc>>,
}

impl HermesJob {
    pub fn catch_up_hours(&self) -> u64 {
        self.catch_up_hours.unwrap_or(6)
    }

    pub fn validate(
        &self,
        default_timezone: Option<&str>,
        now: DateTime<Utc>,
    ) -> std::result::Result<ValidatedHermesJob, String> {
        if self.id.trim().is_empty() {
            return Err("Hermes job has an empty id".into());
        }
        if let Err(err) = crate::cron::check_gateway_lifecycle(&self.prompt) {
            return Err(format!("gateway lifecycle violation: {err}"));
        }
        let prompt_threats = crate::security::scan_cron_prompt(&self.prompt);
        if !prompt_threats.is_empty() {
            return Err(format!(
                "cron prompt injection detected: {}",
                prompt_threats.join("; ")
            ));
        }
        if let Some(ref path) = self.brief_file {
            if !path.exists() {
                return Err(format!("brief_file does not exist: {}", path.display()));
            }
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => return Err(format!("failed to read brief_file {}: {e}", path.display())),
            };
            if let Err(err) = crate::cron::check_gateway_lifecycle(&content) {
                return Err(format!("gateway lifecycle violation in brief_file: {err}"));
            }
            let brief_threats = crate::security::scan_cron_prompt(&content);
            if !brief_threats.is_empty() {
                return Err(format!(
                    "cron prompt injection detected in brief_file: {}",
                    brief_threats.join("; ")
                ));
            }
        }
        if let Some(script) = self.script.as_deref() {
            if let Err(err) = crate::cron::check_gateway_lifecycle(script) {
                return Err(format!("gateway lifecycle violation in script: {err}"));
            }
            if let Some(home) = self.extra.get("_omon_hermes_home").and_then(Value::as_str) {
                let script_path = PathBuf::from(home).join("scripts").join(script);
                if let Ok(content) = std::fs::read_to_string(&script_path) {
                    if let Err(err) = crate::cron::check_gateway_lifecycle(&content) {
                        return Err(format!("gateway lifecycle violation in script body: {err}"));
                    }
                }
            }
        }
        if self.monitor_script.is_some() && self.monitor_url.is_some() {
            return Err(
                "conflicting monitor modes: both monitor_script and monitor_url are set"
                    .to_string(),
            );
        }
        if let Some(ref m_script) = self.monitor_script {
            if let Err(err) = crate::cron::check_gateway_lifecycle(m_script) {
                return Err(format!(
                    "gateway lifecycle violation in monitor_script: {err}"
                ));
            }
        }
        if let Some(ack) = self.ack_command.as_deref() {
            if let Err(err) = crate::cron::check_gateway_lifecycle(ack) {
                return Err(format!("gateway lifecycle violation in ack_command: {err}"));
            }
        }
        let expression = self.expression().map_err(|e| e.to_string())?;
        if let Some(timestamp) = expression.strip_prefix("once:") {
            if parse_timestamp(timestamp).is_err() {
                return Err(format!(
                    "malformed one-shot timestamp `{timestamp}` for job {}",
                    self.id
                ));
            }
        }
        let effective_timezone = self.schedule.timezone.as_deref().or(default_timezone);
        if let Some(tz_str) = effective_timezone {
            if tz_str.trim().parse::<chrono_tz::Tz>().is_err() {
                return Err(format!(
                    "uncomputable schedule for job {}: invalid timezone `{tz_str}`",
                    self.id
                ));
            }
        }
        let computed_next =
            match super::scheduler::next_run_tz(&expression, now, effective_timezone) {
                Ok(next) => Some(next),
                Err(_) if expression.starts_with("once:") => None,
                Err(error) => {
                    return Err(format!(
                        "uncomputable schedule `{expression}` for job {}: {error}",
                        self.id
                    ));
                }
            };

        Ok(ValidatedHermesJob {
            expression,
            effective_timezone: effective_timezone.map(str::to_owned),
            computed_next,
        })
    }

    pub fn expression(&self) -> Result<String> {
        match self.schedule.kind.as_str() {
            "cron" => self
                .schedule
                .expr
                .clone()
                .filter(|value| !value.trim().is_empty()),
            "interval" => self
                .schedule
                .minutes
                .map(|minutes| format!("interval:{}m", minutes)),
            "once" => self
                .schedule
                .run_at
                .clone()
                .map(|value| format!("once:{value}")),
            kind => {
                return Err(OmonError::Config(format!(
                    "unsupported Hermes schedule kind `{kind}` for job {}",
                    self.id
                )))
            }
        }
        .ok_or_else(|| {
            OmonError::Config(format!("Hermes job {} has an incomplete schedule", self.id))
        })
    }

    pub fn configured_discord_home(&self) -> Option<(String, Option<String>)> {
        let val_to_str = |v: &Value| -> Option<String> {
            if let Some(s) = v.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    Some(trimmed.to_string())
                } else {
                    None
                }
            } else {
                v.as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            }
        };

        let channel = self
            .extra
            .get("home_channel")
            .or_else(|| self.extra.get("discord_home_channel"))
            .or_else(|| self.extra.get("_omon_discord_home_channel"))
            .or_else(|| self.extra.get("home"))
            .or_else(|| self.extra.get("channel_id"))
            .and_then(val_to_str);

        let thread = self
            .extra
            .get("thread_id")
            .or_else(|| self.extra.get("discord_thread_id"))
            .or_else(|| self.extra.get("_omon_discord_home_thread"))
            .and_then(val_to_str);

        if let Some(ch) = channel {
            return Some((ch, thread));
        }

        if let Some(discord_obj) = self.extra.get("discord").and_then(Value::as_object) {
            let ch = discord_obj
                .get("channel")
                .or_else(|| discord_obj.get("home_channel"))
                .or_else(|| discord_obj.get("channel_id"))
                .and_then(val_to_str);
            let th = discord_obj.get("thread_id").and_then(val_to_str);
            if let Some(ch) = ch {
                return Some((ch, th));
            }
        }

        None
    }

    pub fn failure_destinations(&self) -> Result<Vec<HermesOrigin>> {
        if let Some(ref failure_deliver) = self.failure_deliver {
            let mut clone = self.clone();
            clone.deliver = Some(failure_deliver.clone());
            clone.discord_destinations()
        } else {
            self.discord_destinations()
        }
    }

    pub fn profile(&self) -> &str {
        if let Some(p) = self.extra.get("profile").and_then(Value::as_str) {
            if !p.is_empty() {
                return p;
            }
        }
        if let Some(ref origin) = self.origin {
            if let Some(ref bid) = origin.bot_id {
                if !bid.is_empty() {
                    return bid;
                }
            }
            if !origin.platform.is_empty() {
                return &origin.platform;
            }
        }
        "default"
    }

    pub fn has_explicit_discord_destination(&self) -> bool {
        let check_targets = |list: &Option<Vec<String>>| -> bool {
            if let Some(targets) = list {
                for t in targets {
                    let trimmed = t.trim();
                    if trimmed.len() >= 8 && trimmed[..8].eq_ignore_ascii_case("discord:") {
                        let rest = trimmed[8..].trim();
                        if !rest.is_empty() {
                            return true;
                        }
                    }
                }
            }
            false
        };

        check_targets(&self.deliver) || check_targets(&self.failure_deliver)
    }

    pub fn discord_destinations(&self) -> Result<Vec<HermesOrigin>> {
        let targets = match &self.deliver {
            Some(list) if !list.is_empty() => list.clone(),
            _ => vec!["origin".to_string()],
        };

        let configured_home = self.configured_discord_home();
        let bot_id = self
            .origin
            .as_ref()
            .and_then(|o| o.bot_id.clone())
            .or_else(|| {
                self.extra
                    .get("bot_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        let mut destinations = Vec::new();
        let mut seen = HashSet::new();

        for part in targets {
            let trimmed = part.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("local") {
                continue;
            }

            let is_discord_prefixed =
                trimmed.len() >= 8 && trimmed[..8].eq_ignore_ascii_case("discord:");

            if is_discord_prefixed {
                let rest = trimmed[8..].trim();
                if !rest.is_empty() {
                    let (chat_id, thread_id) = if let Some((c, t)) = rest.split_once(':') {
                        let c = c.trim().trim_start_matches('#').to_string();
                        let t = t.trim().trim_start_matches('#').to_string();
                        let thread = if t.is_empty() { None } else { Some(t) };
                        (c, thread)
                    } else {
                        (rest.trim_start_matches('#').to_string(), None)
                    };

                    if !chat_id.is_empty() {
                        let key = ("discord".to_string(), chat_id.clone(), thread_id.clone());
                        if seen.insert(key) {
                            destinations.push(HermesOrigin {
                                platform: "discord".into(),
                                chat_id,
                                thread_id,
                                user_id: None,
                                chat_name: None,
                                bot_id: bot_id.clone(),
                                extra: HashMap::new(),
                            });
                        }
                    }
                } else {
                    let mut resolved = false;
                    if let Some(ref origin) = self.origin {
                        if origin.platform.eq_ignore_ascii_case("discord")
                            && !origin.chat_id.is_empty()
                        {
                            let key = (
                                "discord".to_string(),
                                origin.chat_id.clone(),
                                origin.thread_id.clone(),
                            );
                            if seen.insert(key) {
                                destinations.push(origin.clone());
                            }
                            resolved = true;
                        }
                    }
                    if !resolved {
                        if let Some((ref home_channel, ref home_thread)) = configured_home {
                            let key = (
                                "discord".to_string(),
                                home_channel.clone(),
                                home_thread.clone(),
                            );
                            if seen.insert(key) {
                                destinations.push(HermesOrigin {
                                    platform: "discord".into(),
                                    chat_id: home_channel.clone(),
                                    thread_id: home_thread.clone(),
                                    user_id: None,
                                    chat_name: None,
                                    bot_id: bot_id.clone(),
                                    extra: HashMap::new(),
                                });
                            }
                        }
                    }
                }
            } else if trimmed.eq_ignore_ascii_case("origin")
                || trimmed.eq_ignore_ascii_case("all")
                || trimmed.eq_ignore_ascii_case("discord")
            {
                let mut resolved = false;
                if let Some(ref origin) = self.origin {
                    if origin.platform.eq_ignore_ascii_case("discord") && !origin.chat_id.is_empty()
                    {
                        let key = (
                            "discord".to_string(),
                            origin.chat_id.clone(),
                            origin.thread_id.clone(),
                        );
                        if seen.insert(key) {
                            destinations.push(origin.clone());
                        }
                        resolved = true;
                    }
                }
                if !resolved {
                    if let Some((ref home_channel, ref home_thread)) = configured_home {
                        let key = (
                            "discord".to_string(),
                            home_channel.clone(),
                            home_thread.clone(),
                        );
                        if seen.insert(key) {
                            destinations.push(HermesOrigin {
                                platform: "discord".into(),
                                chat_id: home_channel.clone(),
                                thread_id: home_thread.clone(),
                                user_id: None,
                                chat_name: None,
                                bot_id: bot_id.clone(),
                                extra: HashMap::new(),
                            });
                        }
                    }
                }
            } else if trimmed.contains(':') {
                if let Some((left, right)) = trimmed.split_once(':') {
                    let left_trimmed = left.trim().trim_start_matches('#');
                    let right_trimmed = right.trim().trim_start_matches('#');
                    if left_trimmed.chars().all(|c| c.is_ascii_digit())
                        && right_trimmed.chars().all(|c| c.is_ascii_digit())
                    {
                        let chat_id = left_trimmed.to_string();
                        let thread_id = if right_trimmed.is_empty() {
                            None
                        } else {
                            Some(right_trimmed.to_string())
                        };
                        let key = ("discord".to_string(), chat_id.clone(), thread_id.clone());
                        if seen.insert(key) {
                            destinations.push(HermesOrigin {
                                platform: "discord".into(),
                                chat_id,
                                thread_id,
                                user_id: None,
                                chat_name: None,
                                bot_id: bot_id.clone(),
                                extra: HashMap::new(),
                            });
                        }
                    }
                }
            } else {
                let chat_id = trimmed.trim_start_matches('#').trim().to_string();
                if !chat_id.is_empty() && chat_id.chars().all(|c| c.is_ascii_digit()) {
                    let key = ("discord".to_string(), chat_id.clone(), None);
                    if seen.insert(key) {
                        destinations.push(HermesOrigin {
                            platform: "discord".into(),
                            chat_id,
                            thread_id: None,
                            user_id: None,
                            chat_name: None,
                            bot_id: bot_id.clone(),
                            extra: HashMap::new(),
                        });
                    }
                }
            }
        }

        Ok(destinations)
    }

    pub fn discord_destination(&self) -> Result<Option<HermesOrigin>> {
        let targets = self.discord_destinations()?;
        Ok(targets.into_iter().next())
    }
}

#[derive(Clone, Debug)]
pub struct HermesStore {
    profile: String,
    home: PathBuf,
}

impl HermesStore {
    pub fn new(profile: impl Into<String>, home: impl Into<PathBuf>) -> Self {
        Self {
            profile: profile.into(),
            home: home.into(),
        }
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn jobs_path(&self) -> PathBuf {
        self.home.join("cron").join("jobs.json")
    }

    /// Reads the configured source timezone from `<home>/config.yaml`, if any.
    pub async fn timezone(&self) -> Option<String> {
        let path = self.home.join("config.yaml");
        let bytes = tokio::fs::read(&path).await.ok()?;
        #[derive(Deserialize)]
        struct ConfigTimezone {
            #[serde(default)]
            timezone: Option<String>,
        }
        serde_yaml::from_slice::<ConfigTimezone>(&bytes)
            .ok()?
            .timezone
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    pub async fn discord_home(&self) -> Option<(String, Option<String>)> {
        let path = self.home.join("config.yaml");
        let bytes = tokio::fs::read(&path).await.ok()?;
        #[derive(Deserialize)]
        struct RawDiscord {
            #[serde(default)]
            channel: Option<Value>,
            #[serde(default)]
            home_channel: Option<Value>,
            #[serde(default)]
            channel_id: Option<Value>,
            #[serde(default)]
            thread_id: Option<Value>,
        }
        #[derive(Deserialize)]
        struct RawConfig {
            #[serde(default)]
            discord: Option<RawDiscord>,
            #[serde(default)]
            home_channel: Option<Value>,
            #[serde(default)]
            thread_id: Option<Value>,
        }
        let parsed = serde_yaml::from_slice::<RawConfig>(&bytes).ok()?;
        let val_to_str = |v: &Value| -> Option<String> {
            if let Some(s) = v.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    Some(trimmed.to_string())
                } else {
                    None
                }
            } else {
                v.as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            }
        };

        let channel = parsed
            .discord
            .as_ref()
            .and_then(|d| {
                d.channel
                    .as_ref()
                    .or(d.home_channel.as_ref())
                    .or(d.channel_id.as_ref())
                    .and_then(val_to_str)
            })
            .or_else(|| parsed.home_channel.as_ref().and_then(val_to_str));

        let thread = parsed
            .discord
            .as_ref()
            .and_then(|d| d.thread_id.as_ref().and_then(val_to_str))
            .or_else(|| parsed.thread_id.as_ref().and_then(val_to_str));

        channel.map(|c| (c, thread))
    }

    pub async fn load(&self) -> Result<Vec<HermesJob>> {
        let path = self.jobs_path();
        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(OmonError::Config(format!(
                    "failed to read {}: {error}",
                    path.display()
                )))
            }
        };
        #[derive(Deserialize)]
        struct Document {
            #[serde(default)]
            jobs: Vec<HermesJob>,
        }
        let discord_home = self.discord_home().await;
        let mut jobs = serde_json::from_slice::<Document>(&bytes)
            .map(|document| document.jobs)
            .map_err(|error| {
                OmonError::Config(format!(
                    "invalid Hermes cron store {}: {error}",
                    path.display()
                ))
            })?;

        if let Some((home_channel, home_thread)) = discord_home {
            for job in &mut jobs {
                job.extra
                    .entry("_omon_discord_home_channel".into())
                    .or_insert_with(|| Value::String(home_channel.clone()));
                if let Some(ref thread) = home_thread {
                    job.extra
                        .entry("_omon_discord_home_thread".into())
                        .or_insert_with(|| Value::String(thread.clone()));
                }
            }
        }
        Ok(jobs)
    }
}

#[derive(Clone)]
pub struct HermesStoreSynchronizer {
    pool: SqlitePool,
    stores: Vec<HermesStore>,
}

impl HermesStoreSynchronizer {
    pub fn new(pool: SqlitePool, stores: Vec<HermesStore>) -> Self {
        Self { pool, stores }
    }

    pub fn selected_profiles() -> Option<Vec<String>> {
        std::env::var("OMON_HERMES_PROFILES").ok().map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
    }

    pub fn is_profile_selected(profile: &str, selected_profiles: Option<&[String]>) -> bool {
        match selected_profiles {
            Some(profiles) => profiles.iter().any(|p| p == profile),
            None => true,
        }
    }

    pub fn from_environment(pool: SqlitePool) -> Result<Self> {
        let root = std::env::var_os("HERMES_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".hermes")))
            .ok_or_else(|| {
                OmonError::Config(
                    "HOME or HERMES_HOME is required for Hermes cron synchronization".into(),
                )
            })?;
        let profiles = Self::selected_profiles().unwrap_or_else(|| {
            let mut profiles = vec!["default".to_owned()];
            if let Ok(entries) = std::fs::read_dir(root.join("profiles")) {
                profiles.extend(
                    entries
                        .flatten()
                        .filter(|entry| entry.path().is_dir())
                        .map(|entry| entry.file_name().to_string_lossy().into_owned()),
                );
            }
            profiles
        });
        let stores = profiles
            .into_iter()
            .map(|profile| {
                let home = if profile == "default" {
                    root.clone()
                } else {
                    root.join("profiles").join(&profile)
                };
                HermesStore::new(profile, home)
            })
            .collect();
        Ok(Self::new(pool, stores))
    }

    pub async fn sync(&self) -> Result<usize> {
        self.sync_at(Utc::now()).await
    }

    /// Imports using an explicit evaluation instant, shared by all source jobs.
    pub async fn sync_at(&self, now: DateTime<Utc>) -> Result<usize> {
        let mut imported = 0;
        for store in &self.stores {
            let jobs = match store.load().await {
                Ok(jobs) => jobs,
                Err(error) => {
                    tracing::warn!(profile = store.profile(), %error, "skipping malformed Hermes store");
                    continue;
                }
            };
            let source = store.jobs_path().to_string_lossy().into_owned();
            let timezone = store.timezone().await;
            let mut live = HashSet::new();
            for mut job in jobs {
                let validated = match job.validate(timezone.as_deref(), now) {
                    Ok(val) => val,
                    Err(reason) => {
                        tracing::warn!(job_id = %job.id, %reason, "skipping invalid Hermes job");
                        continue;
                    }
                };
                let expression = validated.expression;
                let computed_next = validated.computed_next;
                if job.schedule.timezone.is_none() {
                    job.schedule.timezone = timezone.clone();
                }
                let id = format!("hermes:{}:{}", store.profile(), job.id);
                live.insert(id.clone());
                let mut payload = serde_json::to_value(&job)
                    .map_err(|error| OmonError::Config(error.to_string()))?;
                payload[SOURCE_KEY] = Value::String(source.clone());
                payload["_omon_hermes_profile"] = Value::String(store.profile().to_owned());
                payload["_omon_hermes_home"] =
                    Value::String(store.home().to_string_lossy().into_owned());
                let payload_json = serde_json::to_string(&payload)
                    .map_err(|error| OmonError::Config(error.to_string()))?;
                let next_run_at = match job.next_run_at.as_deref().map(parse_timestamp).transpose()
                {
                    Ok(ts) => ts.or(computed_next),
                    Err(error) => {
                        tracing::warn!(job_id = %job.id, %error, "invalid next_run_at timestamp in Hermes job");
                        continue;
                    }
                };
                let created = match job.created_at.as_deref().map(parse_timestamp).transpose() {
                    Ok(Some(ts)) => ts,
                    _ => now,
                };
                let insert_result = sqlx::query(
                    "INSERT INTO cron_jobs (id, expression, payload_json, enabled, next_run_at, created_at, updated_at, authority)\n                     VALUES (?, ?, ?, ?, ?, ?, ?, 'hermes_mirror')\n                     ON CONFLICT(id) DO UPDATE SET\n                     expression=excluded.expression,\n                     payload_json=CASE\n                         WHEN json_extract(cron_jobs.payload_json, '$.last_status') IS NOT NULL\n                         THEN json_set(\n                             CASE\n                                 WHEN json_extract(cron_jobs.payload_json, '$.repeat.completed') IS NOT NULL\n                                      AND CAST(json_extract(cron_jobs.payload_json, '$.repeat.completed') AS INTEGER) > CAST(COALESCE(json_extract(excluded.payload_json, '$.repeat.completed'), 0) AS INTEGER)\n                                 THEN json_set(excluded.payload_json, '$.repeat.completed', CAST(json_extract(cron_jobs.payload_json, '$.repeat.completed') AS INTEGER))\n                                 ELSE excluded.payload_json\n                             END,\n                             '$.last_status', json_extract(cron_jobs.payload_json, '$.last_status'),\n                             '$.last_run_at', json_extract(cron_jobs.payload_json, '$.last_run_at'),\n                             '$.last_error', json_extract(cron_jobs.payload_json, '$.last_error'),\n                             '$.last_delivery_error', json_extract(cron_jobs.payload_json, '$.last_delivery_error')\n                         )\n                         WHEN json_extract(cron_jobs.payload_json, '$.repeat.completed') IS NOT NULL\n                              AND CAST(json_extract(cron_jobs.payload_json, '$.repeat.completed') AS INTEGER) > CAST(COALESCE(json_extract(excluded.payload_json, '$.repeat.completed'), 0) AS INTEGER)\n                         THEN json_set(excluded.payload_json, '$.repeat.completed', CAST(json_extract(cron_jobs.payload_json, '$.repeat.completed') AS INTEGER))\n                         ELSE excluded.payload_json\n                     END,\n                     enabled=CASE\n                         WHEN cron_jobs.expression LIKE 'once:%'\n                              AND cron_jobs.next_run_at IS NULL\n                              AND cron_jobs.expression = excluded.expression\n                         THEN cron_jobs.enabled\n                         WHEN json_extract(excluded.payload_json, '$.repeat.times') IS NOT NULL\n                              AND CAST(json_extract(excluded.payload_json, '$.repeat.times') AS INTEGER) > 0\n                              AND CAST(COALESCE(json_extract(cron_jobs.payload_json, '$.repeat.completed'), 0) AS INTEGER) >= CAST(json_extract(excluded.payload_json, '$.repeat.times') AS INTEGER)\n                              AND cron_jobs.next_run_at IS NULL\n                         THEN cron_jobs.enabled\n                         ELSE excluded.enabled\n                     END,\n                     next_run_at=CASE\n                         WHEN cron_jobs.expression LIKE 'once:%'\n                              AND cron_jobs.next_run_at IS NULL\n                              AND cron_jobs.expression = excluded.expression\n                         THEN NULL\n                         WHEN json_extract(excluded.payload_json, '$.repeat.times') IS NOT NULL\n                              AND CAST(json_extract(excluded.payload_json, '$.repeat.times') AS INTEGER) > 0\n                              AND CAST(COALESCE(json_extract(cron_jobs.payload_json, '$.repeat.completed'), 0) AS INTEGER) >= CAST(json_extract(excluded.payload_json, '$.repeat.times') AS INTEGER)\n                              AND cron_jobs.next_run_at IS NULL\n                         THEN NULL\n                         WHEN cron_jobs.expression <> excluded.expression\n                           OR json_remove(cron_jobs.payload_json, '$.repeat.completed', '$.last_status', '$.last_run_at', '$.last_error', '$.last_delivery_error', '$.last_occurrence') <> json_remove(excluded.payload_json, '$.repeat.completed', '$.last_status', '$.last_run_at', '$.last_error', '$.last_delivery_error', '$.last_occurrence')\n                           OR cron_jobs.enabled <> excluded.enabled\n                         THEN excluded.next_run_at\n                         ELSE cron_jobs.next_run_at\n                     END,\n                     updated_at=excluded.updated_at\n                     WHERE cron_jobs.authority = 'hermes_mirror'"
                )
                .bind(&id).bind(expression).bind(payload_json).bind(job.enabled)
                .bind(next_run_at).bind(created).bind(now).execute(&self.pool).await?;
                if insert_result.rows_affected() > 0 {
                    imported += 1;
                }
            }
            let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM cron_jobs WHERE json_extract(payload_json, '$._omon_hermes_source') = ? AND authority = 'hermes_mirror'")
                .bind(&source).fetch_all(&self.pool).await?;
            for (id,) in rows {
                if !live.contains(&id) {
                    sqlx::query(
                        "DELETE FROM cron_jobs WHERE id = ? AND authority = 'hermes_mirror'",
                    )
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }
        Ok(imported)
    }
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| OmonError::Config(format!("invalid Hermes timestamp `{value}`: {error}")))
}

pub const MAX_NOTEPAD_VALUE_BYTES: usize = 16 * 1024; // 16 KiB
pub const MAX_NOTEPAD_TOTAL_BYTES: usize = 64 * 1024; // 64 KiB

pub async fn set_cron_notepad(
    pool: &sqlx::SqlitePool,
    profile: &str,
    job_id: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    if value.len() > MAX_NOTEPAD_VALUE_BYTES {
        return Err(OmonError::Config(format!(
            "notepad value size {} bytes exceeds maximum allowed 16 KiB",
            value.len()
        )));
    }
    let current_entries = get_cron_notepads(pool, profile, job_id).await?;
    let mut total_size: usize = current_entries
        .iter()
        .filter(|(k, _)| k != key)
        .map(|(_, v)| v.len())
        .sum();
    total_size += value.len();
    if total_size > MAX_NOTEPAD_TOTAL_BYTES {
        return Err(OmonError::Config(format!(
            "total notepad size {} bytes exceeds maximum allowed 64 KiB for job {job_id}",
            total_size
        )));
    }

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO cron_notepads (profile, job_id, key, value, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(profile, job_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(profile)
    .bind(job_id)
    .bind(key)
    .bind(value)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| OmonError::Database(e.to_string()))?;

    Ok(())
}

pub async fn get_cron_notepads(
    pool: &sqlx::SqlitePool,
    profile: &str,
    job_id: &str,
) -> Result<Vec<(String, String)>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM cron_notepads WHERE profile = ? AND job_id = ? ORDER BY key ASC",
    )
    .bind(profile)
    .bind(job_id)
    .fetch_all(pool)
    .await
    .map_err(|e| OmonError::Database(e.to_string()))?;

    Ok(rows)
}

pub async fn delete_cron_notepad(
    pool: &sqlx::SqlitePool,
    profile: &str,
    job_id: &str,
    key: Option<&str>,
) -> Result<u64> {
    let rows = if let Some(k) = key {
        sqlx::query("DELETE FROM cron_notepads WHERE profile = ? AND job_id = ? AND key = ?")
            .bind(profile)
            .bind(job_id)
            .bind(k)
            .execute(pool)
            .await
            .map_err(|e| OmonError::Database(e.to_string()))?
            .rows_affected()
    } else {
        sqlx::query("DELETE FROM cron_notepads WHERE profile = ? AND job_id = ?")
            .bind(profile)
            .bind(job_id)
            .execute(pool)
            .await
            .map_err(|e| OmonError::Database(e.to_string()))?
            .rows_affected()
    };
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Database;

    fn job(value: Value) -> HermesJob {
        serde_json::from_value(value).expect("valid Hermes job")
    }

    #[tokio::test]
    async fn imports_timezone_and_rejects_invalid_schedule() {
        let root = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(root.path().join("cron"))
            .await
            .unwrap();
        tokio::fs::write(root.path().join("config.yaml"), "timezone: Asia/Seoul\n")
            .await
            .unwrap();
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let sync = HermesStoreSynchronizer::new(
            database.pool().clone(),
            vec![HermesStore::new("seoul", root.path())],
        );
        let now = parse_timestamp("2026-09-05T00:00:00Z").unwrap();
        for next in [Value::Null, json!("2026-09-06T00:00:00Z")] {
            let bytes = serde_json::to_vec(&json!({"jobs": [{
                "id": "bad", "schedule": {"kind": "cron", "expr": "garbage"},
                "next_run_at": next
            }]}))
            .unwrap();
            tokio::fs::write(sync.stores[0].jobs_path(), &bytes)
                .await
                .unwrap();
            let result = sync.sync_at(now).await;
            let rows: Vec<(bool, Option<DateTime<Utc>>)> =
                sqlx::query_as("SELECT enabled, next_run_at FROM cron_jobs")
                    .fetch_all(database.pool())
                    .await
                    .unwrap();
            println!("invalid expression with next={next}: result={result:?}, rows={rows:?}");
            assert_eq!(
                result.unwrap(),
                0,
                "malformed schedules must be isolated from import, even with a stored next run"
            );
            assert!(
                rows.is_empty(),
                "invalid schedule must never become an enabled NULL row"
            );
            assert_eq!(
                tokio::fs::read(sync.stores[0].jobs_path()).await.unwrap(),
                bytes
            );
        }
        tokio::fs::write(
            sync.stores[0].jobs_path(),
            serde_json::to_vec(&json!({"jobs": [{
                "id": "daily", "schedule": {"kind": "cron", "expr": "0 9 * * *"}
            }]}))
            .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(sync.sync_at(now).await.unwrap(), 1);
        let (payload, next): (String, DateTime<Utc>) =
            sqlx::query_as("SELECT payload_json, next_run_at FROM cron_jobs")
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(next, parse_timestamp("2026-09-06T00:00:00Z").unwrap());
        assert_eq!(
            serde_json::from_str::<Value>(&payload).unwrap()["schedule"]["timezone"],
            "Asia/Seoul"
        );
        println!("09:00 Asia/Seoul after {now}: next={next}");
        database.close().await;
        root.close().unwrap();
    }

    #[test]
    fn parses_full_job_and_resolves_origin_delivery() {
        let job = job(json!({
            "id": "brief",
            "prompt": "summarize",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "deliver": "origin",
            "origin": {"platform": "discord", "chat_id": "42", "thread_id": "43"},
            "enabled_toolsets": ["web", "file"]
        }));
        assert_eq!(job.expression().unwrap(), "0 9 * * *");
        let target = job.discord_destination().unwrap().unwrap();
        assert_eq!(target.chat_id, "42");
        assert_eq!(target.thread_id.as_deref(), Some("43"));
        assert_eq!(job.enabled_toolsets.unwrap(), vec!["web", "file"]);
        assert_eq!(job.timeout_secs, None);
    }

    #[test]
    fn parses_job_timeout_overrides() {
        let job_timeout = job(json!({
            "id": "t1",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "timeout": 600
        }));
        assert_eq!(job_timeout.timeout_secs, Some(600));

        let job_timeout_seconds = job(json!({
            "id": "t2",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "timeout_seconds": 2400
        }));
        assert_eq!(job_timeout_seconds.timeout_secs, Some(2400));

        let job_script_timeout = job(json!({
            "id": "t3",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "script_timeout": 1200
        }));
        assert_eq!(job_script_timeout.timeout_secs, Some(1200));

        let job_timeout_secs = job(json!({
            "id": "t4",
            "schedule": {"kind": "cron", "expr": "0 9 * * *"},
            "timeout_secs": 3600
        }));
        assert_eq!(job_timeout_secs.timeout_secs, Some(3600));
    }

    #[test]
    fn parses_cron_script_timeout_secs_from_env() {
        assert_eq!(cron_script_timeout_secs_from(Some("600")), 600);
        assert_eq!(cron_script_timeout_secs_from(Some(" 3600 ")), 3600);
        assert_eq!(cron_script_timeout_secs_from(None), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("   ")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("0")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("-10")), 1800);
        assert_eq!(cron_script_timeout_secs_from(Some("invalid")), 1800);
    }

    #[test]
    fn resolves_cron_script_timeout_with_overrides_and_fallbacks() {
        use std::time::Duration;

        // Job override takes precedence over global default
        assert_eq!(
            resolve_cron_script_timeout(Some(300), 1800),
            Duration::from_secs(300)
        );
        // None falls back to global default
        assert_eq!(
            resolve_cron_script_timeout(None, 2400),
            Duration::from_secs(2400)
        );
        // Zero override falls back to global default
        assert_eq!(
            resolve_cron_script_timeout(Some(0), 1800),
            Duration::from_secs(1800)
        );
        // None with zero global default falls back to DEFAULT_CRON_SCRIPT_TIMEOUT_SECS
        assert_eq!(
            resolve_cron_script_timeout(None, 0),
            Duration::from_secs(DEFAULT_CRON_SCRIPT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn supports_interval_and_one_shot_schedules() {
        let interval = job(json!({"id":"a", "schedule":{"kind":"interval", "minutes":5}}));
        let once = job(
            json!({"id":"b", "schedule":{"kind":"once", "run_at":"2026-08-15T09:00:00+09:00"}}),
        );
        assert_eq!(interval.expression().unwrap(), "interval:5m");
        assert_eq!(once.expression().unwrap(), "once:2026-08-15T09:00:00+09:00");
    }

    #[tokio::test]
    async fn prunes_only_old_terminal_cron_runs() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO cron_jobs
             (id, expression, payload_json, enabled, next_run_at, created_at, updated_at)
             VALUES ('retention-job', 'interval:1h', '{}', 1, NULL, ?, ?)",
        )
        .bind(now)
        .bind(now)
        .execute(database.pool())
        .await
        .unwrap();

        for (run_id, status, started_at, completed_at) in [
            (
                "old-succeeded",
                "succeeded",
                now - chrono::TimeDelta::days(31),
                Some(now - chrono::TimeDelta::days(30)),
            ),
            (
                "old-failed",
                "failed",
                now - chrono::TimeDelta::days(30),
                None,
            ),
            (
                "recent-succeeded",
                "succeeded",
                now - chrono::TimeDelta::days(2),
                Some(now - chrono::TimeDelta::days(1)),
            ),
            (
                "old-running",
                "running",
                now - chrono::TimeDelta::days(30),
                None,
            ),
        ] {
            sqlx::query(
                "INSERT INTO cron_runs
                 (run_id, job_id, claim_token, lease_expires_at, started_at, completed_at, status, attempt, error)
                 VALUES (?, 'retention-job', ?, ?, ?, ?, ?, 1, NULL)",
            )
            .bind(run_id)
            .bind(format!("token-{run_id}"))
            .bind(now + chrono::TimeDelta::hours(1))
            .bind(started_at)
            .bind(completed_at)
            .bind(status)
            .execute(database.pool())
            .await
            .unwrap();
        }

        let deleted = prune_terminal_cron_runs(database.pool(), 14, now)
            .await
            .unwrap();
        assert_eq!(deleted, 2);
        let remaining: HashSet<String> =
            sqlx::query_scalar("SELECT run_id FROM cron_runs ORDER BY run_id")
                .fetch_all(database.pool())
                .await
                .unwrap()
                .into_iter()
                .collect();
        assert_eq!(
            remaining,
            HashSet::from(["old-running".to_owned(), "recent-succeeded".to_owned()])
        );
    }

    #[tokio::test]
    async fn synchronization_does_not_rearm_completed_one_shot() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let root = std::env::temp_dir().join(format!("omon-hermes-once-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(root.join("cron")).await.unwrap();
        tokio::fs::write(
            root.join("cron/jobs.json"),
            serde_json::to_vec(&json!({"jobs": [{
                "id": "once", "prompt": "run", "enabled": true,
                "next_run_at": "2026-08-15T09:00:00+09:00",
                "schedule": {"kind": "once", "run_at": "2026-08-15T09:00:00+09:00"},
                "deliver": "local"
            }]}))
            .unwrap(),
        )
        .await
        .unwrap();
        let sync = HermesStoreSynchronizer::new(
            database.pool().clone(),
            vec![HermesStore::new("default", &root)],
        );
        sync.sync().await.unwrap();
        sqlx::query(
            "UPDATE cron_jobs SET enabled = 0, next_run_at = NULL WHERE id = 'hermes:default:once'",
        )
        .execute(database.pool())
        .await
        .unwrap();
        sync.sync().await.unwrap();
        let state: (bool, Option<DateTime<Utc>>) = sqlx::query_as(
            "SELECT enabled, next_run_at FROM cron_jobs WHERE id = 'hermes:default:once'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(state, (false, None));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn synchronization_preserves_scheduler_advanced_next_run() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let root = std::env::temp_dir().join(format!("omon-hermes-store-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(root.join("cron")).await.unwrap();
        tokio::fs::write(
            root.join("cron/jobs.json"),
            serde_json::to_vec(&json!({
                "jobs": [{
                    "id": "brief", "prompt": "summarize", "enabled": true,
                    "created_at": "2026-08-01T09:00:00+09:00",
                    "next_run_at": "2026-08-15T09:00:00+09:00",
                    "schedule": {"kind": "cron", "expr": "0 9 * * *"},
                    "deliver": "discord:42"
                }]
            }))
            .unwrap(),
        )
        .await
        .unwrap();
        let sync = HermesStoreSynchronizer::new(
            database.pool().clone(),
            vec![HermesStore::new("default", &root)],
        );
        sync.sync().await.unwrap();
        let advanced = Utc::now() + chrono::TimeDelta::days(30);
        sqlx::query("UPDATE cron_jobs SET next_run_at = ? WHERE id = 'hermes:default:brief'")
            .bind(advanced)
            .execute(database.pool())
            .await
            .unwrap();
        sync.sync().await.unwrap();
        let actual: DateTime<Utc> = sqlx::query_scalar(
            "SELECT next_run_at FROM cron_jobs WHERE id = 'hermes:default:brief'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert_eq!(actual, advanced);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn synchronization_does_not_rearm_completed_repeat_times() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let root =
            std::env::temp_dir().join(format!("omon-hermes-repeat-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(root.join("cron")).await.unwrap();
        tokio::fs::write(
            root.join("cron/jobs.json"),
            serde_json::to_vec(&json!({"jobs": [{
                "id": "repeat_job", "prompt": "run", "enabled": true,
                "schedule": {"kind": "interval", "minutes": 5},
                "repeat": {"times": 2, "completed": 0},
                "deliver": "local"
            }]}))
            .unwrap(),
        )
        .await
        .unwrap();
        let sync = HermesStoreSynchronizer::new(
            database.pool().clone(),
            vec![HermesStore::new("default", &root)],
        );
        sync.sync().await.unwrap();

        // Simulate scheduler completing 2 runs and disabling job
        sqlx::query(
            "UPDATE cron_jobs SET enabled = 0, next_run_at = NULL, payload_json = json_set(payload_json, '$.repeat.completed', 2) WHERE id = 'hermes:default:repeat_job'",
        )
        .execute(database.pool())
        .await
        .unwrap();

        // Sync again from jobs.json (which still has completed: 0)
        sync.sync().await.unwrap();

        let state: (bool, Option<DateTime<Utc>>, String) = sqlx::query_as(
            "SELECT enabled, next_run_at, payload_json FROM cron_jobs WHERE id = 'hermes:default:repeat_job'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();
        assert!(
            !state.0,
            "Job must remain disabled after repeat.times limit was reached"
        );
        assert_eq!(state.1, None, "Disabled job must not have next_run_at");
        let payload: Value = serde_json::from_str(&state.2).unwrap();
        assert_eq!(payload["repeat"]["completed"], 2);

        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn synchronization_skips_gateway_lifecycle_violators() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let root = std::env::temp_dir().join(format!("omon-hermes-guard-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(root.join("cron")).await.unwrap();
        tokio::fs::write(
            root.join("cron/jobs.json"),
            serde_json::to_vec(&json!({"jobs": [
                {
                    "id": "benign", "prompt": "check weather", "enabled": true,
                    "schedule": {"kind": "interval", "minutes": 5},
                    "deliver": "local"
                },
                {
                    "id": "evil_restart", "prompt": "launchctl kickstart gui/501/omon-gateway", "enabled": true,
                    "schedule": {"kind": "interval", "minutes": 5},
                    "deliver": "local"
                },
                {
                    "id": "evil_script", "prompt": "check status", "script": "systemctl restart hermes-gateway", "enabled": true,
                    "schedule": {"kind": "interval", "minutes": 5},
                    "deliver": "local"
                }
            ]}))
            .unwrap(),
        )
        .await
        .unwrap();

        let sync = HermesStoreSynchronizer::new(
            database.pool().clone(),
            vec![HermesStore::new("default", &root)],
        );
        let imported = sync.sync().await.unwrap();
        assert_eq!(imported, 1, "Only benign job should be imported");

        let jobs: Vec<String> = sqlx::query_scalar("SELECT id FROM cron_jobs ORDER BY id")
            .fetch_all(database.pool())
            .await
            .unwrap();
        assert_eq!(jobs, vec!["hermes:default:benign".to_string()]);

        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn discord_delivery_uses_profile_home() {
        let job_json = json!({
            "id": "profile_home_job",
            "prompt": "report",
            "deliver": ["discord", "all"],
            "schedule": {"kind": "cron", "expr": "0 * * * *"},
            "home_channel": "42",
            "thread_id": "43"
        });

        let job: HermesJob = serde_json::from_value(job_json).expect("HermesJob must deserialize");
        let destinations = job
            .discord_destinations()
            .expect("destinations must resolve");
        assert_eq!(
            destinations.len(),
            1,
            "Expected exactly 1 deduped destination"
        );
        assert_eq!(destinations[0].chat_id, "42");
        assert_eq!(destinations[0].thread_id.as_deref(), Some("43"));
    }
}

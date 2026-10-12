use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;

const MACHINE_TIMEOUT: Duration = Duration::from_secs(20);
const CACHE_TTL: Duration = Duration::from_secs(15);
const STALE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Serialize, Deserialize)]
struct CachedListing {
    saved_at: std::time::SystemTime,
    sessions: Vec<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaleListing {
    pub machine: String,
    pub stale: bool,
    pub age_seconds: u64,
}

pub fn probe_ssh_hosts(config: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    for line in config.lines() {
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("host"))
        {
            continue;
        }
        for host in words {
            if !host.contains(['*', '?', '!'])
                && host != "local"
                && !hosts.iter().any(|seen| seen == host)
            {
                hosts.push(host.to_owned());
            }
        }
    }
    hosts
}

pub async fn seed_hosts_file(
    path: &Path,
    config_path: &Path,
) -> Result<Vec<String>, RegistryError> {
    let config = tokio::fs::read_to_string(config_path)
        .await
        .map_err(|_| RegistryError::SshConfig(config_path.to_path_buf()))?;
    let candidates = probe_ssh_hosts(&config);
    let runner = ProcessCommandRunner;
    let checks = candidates.into_iter().map(|host| {
        let runner = &runner;
        async move {
            let output = tokio::time::timeout(
                Duration::from_secs(7),
                runner.run(&host, &["command", "-v", "omo"].map(str::to_owned)),
            )
            .await;
            match output {
                Ok(Ok(result)) if result.code == 0 && !result.stdout.is_empty() => Some(host),
                _ => None,
            }
        }
    });
    let hosts: Vec<String> = futures_util::future::join_all(checks)
        .await
        .into_iter()
        .flatten()
        .collect();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| RegistryError::SshConfig(path.to_path_buf()))?;
    }
    let bytes = serde_json::to_vec_pretty(&hosts)
        .map_err(|error| RegistryError::InvalidJson(error.to_string()))?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|_| RegistryError::SshConfig(path.to_path_buf()))?;
    Ok(hosts)
}

pub fn write_hosts_file(path: &Path, hosts: &[String]) -> Result<(), RegistryError> {
    if hosts
        .iter()
        .any(|host| host.is_empty() || host.chars().any(char::is_whitespace))
    {
        return Err(RegistryError::InvalidHosts(
            "host names must be non-empty and whitespace-free".into(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| RegistryError::SshConfig(path.to_path_buf()))?;
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(hosts).expect("serializing string list cannot fail"),
    )
    .map_err(|_| RegistryError::SshConfig(path.to_path_buf()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    #[serde(alias = "sessionId", alias = "id")]
    pub session_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, skip_serializing)]
    pub alive: Option<bool>,
    #[serde(default, skip_serializing)]
    pub surface: Option<String>,
    #[serde(
        default,
        alias = "lastActivityAt",
        alias = "lastActivity",
        alias = "updatedAt"
    )]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub controls: Vec<String>,
    #[serde(default)]
    pub machine: String,
    #[serde(default, skip_serializing)]
    pub last_activity: Option<String>,
    #[serde(default, skip_serializing)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostError {
    pub machine: String,
    pub message: String,
    pub code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionList {
    pub sessions: Vec<Session>,
    pub errors: Vec<HostError>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale: Vec<StaleListing>,
    #[serde(default, skip_serializing)]
    pub exit_code: i32,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("session list returned nothing answering (exit code 3)")]
    NothingAnswering,
    #[error("omo thread list failed with exit code {0}")]
    ExitCode(i32),
    #[error("invalid omo session JSON: {0}")]
    InvalidJson(String),
    #[error("invalid OMON_SESSION_HOSTS value: {0}")]
    InvalidHosts(String),
    #[error("failed to read SSH hosts from {0}")]
    SshConfig(PathBuf),
    #[error("failed to run command: {0}")]
    Command(String),
}

impl RegistryError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NothingAnswering => 3,
            Self::ExitCode(code @ 1..=5) => *code,
            Self::ExitCode(127) => 4,
            Self::ExitCode(255) => 3,
            Self::InvalidHosts(_) => 2,
            Self::ExitCode(_) | Self::InvalidJson(_) | Self::SshConfig(_) | Self::Command(_) => 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[async_trait]
pub trait CommandRunner: Send + Sync {
    async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String>;
}

#[derive(Default)]
pub struct ProcessCommandRunner;

#[async_trait]
impl CommandRunner for ProcessCommandRunner {
    async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
        let (program, args) = if machine == "local" {
            let (program, args) = argv.split_first().ok_or("empty command")?;
            (program.as_str(), args.to_vec())
        } else {
            let mut args = vec![
                "-o".to_owned(),
                "BatchMode=yes".to_owned(),
                "-o".to_owned(),
                "ConnectTimeout=5".to_owned(),
                machine.to_owned(),
            ];
            args.extend(argv.iter().cloned());
            ("ssh", args)
        };
        let mut cmd = Command::new(program);
        cmd.args(args).stdin(Stdio::null()).kill_on_drop(true);
        let output = cmd.output().await.map_err(|error| error.to_string())?;
        Ok(CommandOutput {
            code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

pub struct SessionRegistry<R = ProcessCommandRunner> {
    runner: Arc<R>,
    machines: Vec<String>,
    machine_timeout: Duration,
    listings: RwLock<HashMap<String, (Instant, Vec<Session>)>>,
    cache_dir: Option<PathBuf>,
}

impl SessionRegistry<ProcessCommandRunner> {
    pub fn from_environment() -> Result<Self, RegistryError> {
        let mut registry = Self::new(Arc::new(ProcessCommandRunner), configured_hosts()?);
        registry.cache_dir = Some(default_hosts_path().with_file_name("session-list-cache"));
        Ok(registry)
    }
}

impl<R: CommandRunner + 'static> SessionRegistry<R> {
    pub fn new(runner: Arc<R>, hosts: Vec<String>) -> Self {
        let mut machines = vec!["local".to_owned()];
        machines.extend(hosts.into_iter().filter(|host| host != "local"));
        Self {
            runner,
            machines,
            machine_timeout: MACHINE_TIMEOUT,
            listings: RwLock::new(HashMap::new()),
            cache_dir: (std::any::TypeId::of::<R>()
                == std::any::TypeId::of::<ProcessCommandRunner>()
                && std::env::args()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .any(|args| args == ["sessions", "list"]))
            .then(|| default_hosts_path().with_file_name("session-list-cache")),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.machine_timeout = timeout.min(MACHINE_TIMEOUT);
        self
    }

    pub async fn list_all(&self) -> SessionList {
        if let Some(dir) = &self.cache_dir {
            for machine in &self.machines {
                if self.listings.read().await.contains_key(machine) {
                    continue;
                }
                if let Ok(bytes) = tokio::fs::read(listing_path(dir, machine)).await {
                    if let Ok(saved) = serde_json::from_slice::<CachedListing>(&bytes) {
                        if let Ok(age) = saved.saved_at.elapsed() {
                            if age <= STALE_TTL {
                                self.listings.write().await.insert(
                                    machine.clone(),
                                    (Instant::now() - age, saved.sessions),
                                );
                            }
                        }
                    }
                }
            }
        }

        let tasks = self.machines.iter().map(|machine| {
            let machine = machine.clone();
            let runner = self.runner.clone();
            let machine_timeout = self.machine_timeout;
            async move {
                let cached = self.listings.read().await.get(&machine).cloned();
                if let Some((at, sessions)) = &cached {
                    if at.elapsed() < CACHE_TTL {
                        return (
                            machine,
                            Ok(Ok(CommandOutput {
                                code: 0,
                                stdout: serde_json::to_vec(sessions)
                                    .expect("session serialization"),
                                stderr: Vec::new(),
                            })),
                            true,
                        );
                    }
                }
                let argv = ["omo", "thread", "list", "--all-scope", "--json"]
                    .map(str::to_owned)
                    .to_vec();
                let result =
                    tokio::time::timeout(machine_timeout, runner.run(&machine, &argv)).await;
                (machine, result, false)
            }
        });
        let mut result = SessionList {
            sessions: Vec::new(),
            errors: Vec::new(),
            stale: Vec::new(),
            exit_code: 0,
        };
        for (machine, outcome, cached_hit) in futures_util::future::join_all(tasks).await {
            match outcome {
                Err(_) => {
                    if let Some((at, sessions)) = self.listings.read().await.get(&machine) {
                        if at.elapsed() <= STALE_TTL {
                            result.sessions.extend(sessions.clone());
                            result.stale.push(StaleListing {
                                machine,
                                stale: true,
                                age_seconds: at.elapsed().as_secs(),
                            });
                            continue;
                        }
                    }
                    result.errors.push(HostError {
                        machine,
                        message: "timed out after 20 seconds".into(),
                        code: 3,
                    });
                }
                Ok(Err(_)) => result.errors.push(HostError {
                    machine,
                    message: "failed to execute session listing".into(),
                    code: 5,
                }),
                Ok(Ok(output)) if output.code == 3 => result.errors.push(HostError {
                    machine,
                    message: RegistryError::NothingAnswering.to_string(),
                    code: 3,
                }),
                Ok(Ok(output)) if output.code != 0 => result.errors.push(HostError {
                    machine,
                    message: RegistryError::ExitCode(output.code).to_string(),
                    code: RegistryError::ExitCode(output.code).exit_code(),
                }),
                Ok(Ok(output)) => match parse_sessions(&output.stdout, &machine) {
                    Ok(mut sessions) => {
                        if !cached_hit {
                            self.listings
                                .write()
                                .await
                                .insert(machine.clone(), (Instant::now(), sessions.clone()));
                            if let Some(dir) = &self.cache_dir {
                                let saved = CachedListing {
                                    saved_at: std::time::SystemTime::now(),
                                    sessions: sessions.clone(),
                                };
                                if let Err(error) = save_listing(dir, &machine, &saved).await {
                                    tracing::warn!(%error, "session cache write failed");
                                }
                            }
                        }
                        result.sessions.append(&mut sessions);
                    }
                    Err(error) => result.errors.push(HostError {
                        machine,
                        message: "invalid omo session JSON".into(),
                        code: error.exit_code(),
                    }),
                },
            }
        }
        if result.errors.len() == self.machines.len() {
            // Prefer actionable failures over unavailable hosts, independently of host order.
            result.exit_code = [5, 2, 1, 4, 3]
                .into_iter()
                .find(|code| result.errors.iter().any(|error| error.code == *code))
                .unwrap_or(5);
        }
        result
    }
}

fn listing_path(dir: &Path, machine: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    dir.join(format!("{:x}.json", Sha256::digest(machine.as_bytes())))
}

async fn save_listing(dir: &Path, machine: &str, saved: &CachedListing) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::create_dir_all(dir).await?;
    tokio::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).await?;
    let path = listing_path(dir, machine);
    let temp = dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec(saved)?;
    tokio::fs::write(&temp, bytes).await?;
    tokio::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600)).await?;
    tokio::fs::rename(temp, path).await
}

pub fn parse_sessions(bytes: &[u8], machine: &str) -> Result<Vec<Session>, RegistryError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| RegistryError::InvalidJson(error.to_string()))?;
    let entries = value
        .as_array()
        .ok_or_else(|| RegistryError::InvalidJson("expected a JSON array".into()))?;
    entries
        .iter()
        .map(|entry| {
            let mut session: Session = serde_json::from_value(entry.clone())
                .map_err(|error| RegistryError::InvalidJson(error.to_string()))?;
            if session.session_id.trim().is_empty() {
                return Err(RegistryError::InvalidJson(
                    "sessionId must not be empty".into(),
                ));
            }
            if session.updated_at.is_none() {
                session.updated_at = session.last_activity.clone();
            }
            if session.last_activity.is_none() {
                session.last_activity = session.updated_at.clone();
            }
            session.machine = machine.to_owned();
            session.raw = entry.clone();
            Ok(session)
        })
        .collect()
}

pub fn configured_hosts() -> Result<Vec<String>, RegistryError> {
    let configured = std::env::var("OMON_SESSION_HOSTS").unwrap_or_default();
    if !configured.trim().is_empty() {
        return configured
            .split(',')
            .map(str::trim)
            .map(|host| {
                if host.is_empty() || host.chars().any(char::is_whitespace) {
                    Err(RegistryError::InvalidHosts(configured.clone()))
                } else {
                    Ok(host.to_owned())
                }
            })
            .collect();
    }
    read_seeded_hosts(&default_hosts_path())
}

fn default_hosts_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".omon/session-hosts.json")
}

fn read_seeded_hosts(path: &Path) -> Result<Vec<String>, RegistryError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|_| RegistryError::SshConfig(path.to_path_buf()))?;
    let hosts: Vec<String> = serde_json::from_slice(&bytes)
        .map_err(|error| RegistryError::InvalidHosts(error.to_string()))?;
    if hosts
        .iter()
        .any(|host| host.is_empty() || host.chars().any(char::is_whitespace))
    {
        return Err(RegistryError::InvalidHosts(
            "host names must be non-empty and whitespace-free".into(),
        ));
    }
    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TimeoutAfterSuccess(AtomicUsize);
    #[async_trait]
    impl CommandRunner for TimeoutAfterSuccess {
        async fn run(&self, _: &str, _: &[String]) -> Result<CommandOutput, String> {
            if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
                return std::future::pending().await;
            }
            Ok(CommandOutput {
                code: 0,
                stdout: br#"[{"sessionId":"cached"}]"#.to_vec(),
                stderr: Vec::new(),
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_serves_marked_stale_success_without_refreshing_age() {
        let registry =
            SessionRegistry::new(Arc::new(TimeoutAfterSuccess(AtomicUsize::new(0))), vec![]);
        assert_eq!(registry.list_all().await.sessions.len(), 1);
        tokio::time::advance(Duration::from_secs(16)).await;
        let result = registry.list_all().await;
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.sessions[0].session_id, "cached");
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["stale"][0]["machine"], "local");
        assert_eq!(json["stale"][0]["stale"], true);
        assert_eq!(json["stale"][0]["age_seconds"], 36);
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_without_stale_success_keeps_exit_three() {
        let registry =
            SessionRegistry::new(Arc::new(TimeoutAfterSuccess(AtomicUsize::new(1))), vec![]);
        let result = registry.list_all().await;
        assert_eq!(result.exit_code, 3);
        assert!(result.sessions.is_empty());
        assert!(result.stale.is_empty());
        assert_eq!(result.errors[0].code, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn expired_stale_success_keeps_exit_three() {
        let registry =
            SessionRegistry::new(Arc::new(TimeoutAfterSuccess(AtomicUsize::new(0))), vec![]);
        registry.list_all().await;
        tokio::time::advance(Duration::from_secs(301)).await;
        let result = registry.list_all().await;
        assert_eq!(result.exit_code, 3);
        assert!(result.sessions.is_empty());
    }

    struct FakeRunner {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl CommandRunner for FakeRunner {
        async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
            assert_eq!(argv, &["omo", "thread", "list", "--all-scope", "--json"]);
            self.calls.fetch_add(1, Ordering::SeqCst);
            if machine == "slow" {
                return Err("unexpected fake".into());
            }
            if machine == "empty" {
                return Ok(CommandOutput {
                    code: 3,
                    stdout: vec![],
                    stderr: vec![],
                });
            }
            let name = if machine == "local" {
                "local session"
            } else {
                "remote session"
            };
            Ok(CommandOutput {
                code: 0,
                stdout: serde_json::to_vec(&serde_json::json!([{
                    "sessionId": machine,
                    "name": name,
                    "cwd": "/tmp",
                    "status": "live",
                    "updated_at": "2026-10-11T12:00:00Z",
                    "controls": ["send", "read"]
                }]))
                .unwrap(),
                stderr: vec![],
            })
        }
    }

    #[test]
    fn strict_json_requires_array_and_session_id() {
        for input in [b"{}".as_slice(), b"[{\"cwd\":\"/tmp\"}]", b"not json"] {
            assert!(parse_sessions(input, "local").is_err());
        }
        assert_eq!(
            parse_sessions(b"[]", "local").unwrap(),
            Vec::<Session>::new()
        );
    }

    #[test]
    fn ssh_host_parser_skips_patterns_and_deduplicates() {
        assert_eq!(
            probe_ssh_hosts("Host *.example\nHost omarchy box\nHost box"),
            vec!["omarchy", "box"]
        );
    }

    #[tokio::test]
    async fn local_and_remote_sessions_merge_and_cache() {
        let runner = Arc::new(FakeRunner {
            calls: AtomicUsize::new(0),
        });
        let registry = SessionRegistry::new(runner.clone(), vec!["host".into()]);
        let first = registry.list_all().await;
        assert_eq!(first.sessions.len(), 2);
        assert_eq!(first.sessions[0].machine, "local");
        assert_eq!(first.sessions[0].session_id, "local");
        assert_eq!(
            first.sessions[0].updated_at.as_deref(),
            Some("2026-10-11T12:00:00Z")
        );
        assert_eq!(first.sessions[0].controls, vec!["send", "read"]);
        assert_eq!(first.sessions[1].machine, "host");
        assert_eq!(first.sessions[1].session_id, "host");
        assert!(first.errors.is_empty());
        let _ = registry.list_all().await;
        assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn exit_three_is_reported_as_nothing_answering() {
        let registry = SessionRegistry::new(
            Arc::new(FakeRunner {
                calls: AtomicUsize::new(0),
            }),
            vec!["empty".into()],
        );
        let listed = registry.list_all().await;
        assert!(listed
            .errors
            .iter()
            .any(|error| error.machine == "empty" && error.message.contains("nothing answering")));
        assert_eq!(listed.sessions.len(), 1);
    }

    #[test]
    fn parse_sessions_extracts_and_serializes_planned_fields() {
        let sample = serde_json::json!([{
            "sessionId": "s-123",
            "name": "worker",
            "cwd": "/repo",
            "status": "busy",
            "updated_at": "2026-10-11T11:22:33Z",
            "controls": ["send", "interrupt"]
        }]);
        let parsed = parse_sessions(&serde_json::to_vec(&sample).unwrap(), "local").unwrap();
        assert_eq!(parsed.len(), 1);
        let s = &parsed[0];
        assert_eq!(s.session_id, "s-123");
        assert_eq!(s.name.as_deref(), Some("worker"));
        assert_eq!(s.cwd.as_deref(), Some("/repo"));
        assert_eq!(s.status.as_deref(), Some("busy"));
        assert_eq!(s.updated_at.as_deref(), Some("2026-10-11T11:22:33Z"));
        assert_eq!(s.last_activity.as_deref(), Some("2026-10-11T11:22:33Z"));
        assert_eq!(s.controls, vec!["send", "interrupt"]);
        assert_eq!(s.machine, "local");

        let serialized = serde_json::to_value(s).unwrap();
        assert_eq!(serialized["session_id"], "s-123");
        assert_eq!(serialized["updated_at"], "2026-10-11T11:22:33Z");
        assert_eq!(
            serialized["controls"],
            serde_json::json!(["send", "interrupt"])
        );
    }

    struct DropGuard {
        tx: tokio::sync::mpsc::Sender<String>,
        machine: String,
    }

    impl Drop for DropGuard {
        fn drop(&mut self) {
            let _ = self.tx.try_send(self.machine.clone());
        }
    }

    struct DeadlineRunner {
        started_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        drop_tx: tokio::sync::mpsc::Sender<String>,
    }

    #[async_trait]
    impl CommandRunner for DeadlineRunner {
        async fn run(&self, machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            if machine == "stalled" {
                let _guard = DropGuard {
                    tx: self.drop_tx.clone(),
                    machine: machine.to_owned(),
                };
                if let Some(tx) = self.started_tx.lock().await.take() {
                    let _ = tx.send(());
                }
                std::future::pending::<()>().await;
                unreachable!()
            } else {
                let fixture = serde_json::json!([{
                    "sessionId": "s-local",
                    "name": "active session",
                    "cwd": "/Users/indo/code",
                    "status": "live",
                    "updated_at": "2026-10-11T10:00:00Z",
                    "controls": ["send", "read", "interrupt"]
                }]);
                Ok(CommandOutput {
                    code: 0,
                    stdout: serde_json::to_vec(&fixture).unwrap(),
                    stderr: vec![],
                })
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn hanging_host_times_out_without_blocking_other_hosts() {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (drop_tx, mut drop_rx) = tokio::sync::mpsc::channel(2);

        let runner = Arc::new(DeadlineRunner {
            started_tx: tokio::sync::Mutex::new(Some(started_tx)),
            drop_tx,
        });
        let mut registry = SessionRegistry::new(runner, vec!["stalled".into()]);
        registry.machine_timeout = Duration::from_secs(20);
        let registry = Arc::new(registry);

        let task = tokio::spawn(async move { registry.list_all().await });

        // Subscribe before trigger: ensure stalled runner is active and drop guard is registered
        tokio::time::timeout(Duration::from_secs(1), started_rx)
            .await
            .unwrap()
            .expect("stalled host must start running");

        // Advance paused time past 20s timeout
        tokio::time::advance(Duration::from_secs(21)).await;

        let listed = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(listed.sessions.len(), 1);
        assert_eq!(listed.sessions[0].machine, "local");
        assert_eq!(listed.sessions[0].session_id, "s-local");
        assert_eq!(listed.sessions[0].name.as_deref(), Some("active session"));
        assert_eq!(listed.sessions[0].cwd.as_deref(), Some("/Users/indo/code"));
        assert_eq!(listed.sessions[0].status.as_deref(), Some("live"));
        assert_eq!(
            listed.sessions[0].updated_at.as_deref(),
            Some("2026-10-11T10:00:00Z")
        );
        assert_eq!(
            listed.sessions[0].controls,
            vec![
                "send".to_string(),
                "read".to_string(),
                "interrupt".to_string()
            ]
        );
        assert_eq!(listed.errors.len(), 1);
        assert_eq!(listed.errors[0].machine, "stalled");

        // Verify future drop occurred upon timeout cancellation
        let dropped = drop_rx
            .try_recv()
            .expect("stalled future must drop on timeout");
        assert_eq!(dropped, "stalled");
    }

    #[test]
    fn hosts_file_round_trips_without_touching_home() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scratch/session-hosts.json");
        let hosts = vec!["omarchy".to_owned()];
        write_hosts_file(&path, &hosts).unwrap();
        assert_eq!(read_seeded_hosts(&path).unwrap(), hosts);
    }
}

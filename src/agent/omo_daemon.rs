//! Managed lifecycle for the local `omo app-server` daemon.
//!
//! The gateway is zero-config: when the app-server URL points at a local
//! address and nothing is listening there yet, the gateway spawns
//! `omo app-server --listen <url>` itself, waits for readiness, keeps it
//! alive (capped exponential backoff, never giving up), and kills it on
//! shutdown. Externally managed
//! daemons (already listening, or non-local URLs) are detected and left
//! alone.

use crate::agent::OmoBackendConfig;
use crate::{OmonError, Result};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout, Instant};

static SPAWN_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Daemon children this gateway owns, by app-server URL, so a backend that stops
/// answering can be restarted from the turn path without threading the supervisor
/// through every caller.
type ChildSlot = Arc<Mutex<Option<Child>>>;
type OwnedDaemons = std::collections::HashMap<String, std::sync::Weak<Mutex<Option<Child>>>>;
static OWNED_DAEMONS: LazyLock<std::sync::Mutex<OwnedDaemons>> = LazyLock::new(Default::default);
/// When each URL was last force-restarted; a slow model must not turn into a restart loop.
static LAST_FORCED_RESTART: LazyLock<std::sync::Mutex<std::collections::HashMap<String, Instant>>> =
    LazyLock::new(Default::default);
/// Minimum spacing between two forced restarts of the same daemon.
const FORCED_RESTART_MIN_INTERVAL: Duration = Duration::from_secs(120);

const READY_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const READY_WAIT_AFTER_SPAWN: Duration = Duration::from_secs(30);
/// First delay before a restart attempt; it doubles per consecutive failure.
const RESTART_BACKOFF: Duration = Duration::from_secs(2);
/// Upper bound of the restart delay: the supervisor keeps retrying at this pace.
const RESTART_BACKOFF_MAX: Duration = Duration::from_secs(60);
/// A daemon that stayed ready this long before exiting counts as healthy, so its
/// exit starts a fresh outage (backoff reset) instead of extending a crash loop.
const RESTART_WINDOW: Duration = Duration::from_secs(60);

/// Delay before the restart attempt that follows `failures` consecutive failures.
fn restart_backoff(failures: u32, base: Duration, cap: Duration) -> Duration {
    base.saturating_mul(1u32 << failures.min(16)).min(cap)
}

/// True when the daemon URL targets this machine (auto-spawn eligible).
pub fn is_local_url(url: &str) -> bool {
    let rest = url
        .trim()
        .strip_prefix("ws://")
        .unwrap_or_else(|| url.trim());
    let authority = rest.split('/').next().unwrap_or("");
    // `ws://localhost:pw@evil.example` connects to evil.example; userinfo is never local.
    if authority.contains('@') {
        return false;
    }
    let host = authority.split(':').next().unwrap_or("");
    host.eq_ignore_ascii_case("127.0.0.1") || host.eq_ignore_ascii_case("localhost")
}

/// GET `<http-url>/readyz` over a raw TCP connection; true on HTTP 200.
async fn probe_readyz(ws_url: &str, limit: Duration) -> bool {
    let Some(rest) = ws_url.trim().strip_prefix("ws://") else {
        return false; // wss:// daemons are externally managed; no local probe
    };
    let Some(authority) = rest.split('/').next() else {
        return false;
    };
    let request = format!("GET /readyz HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    timeout(limit, async move {
        let mut stream = tokio::net::TcpStream::connect(authority).await.ok()?;
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.ok()?;
        let head = String::from_utf8_lossy(&buf);
        Some(head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200"))
    })
    .await
    .unwrap_or(Some(false))
    .unwrap_or(false)
}

/// Resolve the daemon binary to something spawnable.
///
/// Under launchd the gateway inherits a minimal `PATH`
/// (`/usr/bin:/bin:/usr/sbin:/sbin`) which excludes user-level install
/// directories such as `~/.bun/bin`, so a bare `omo` fails with ENOENT even
/// though it resolves fine in an interactive shell. When the name carries no
/// path separator and cannot be found on `path_env`, fall back to well-known
/// absolute install locations. Explicit paths are returned untouched.
fn resolve_daemon_bin(bin: &str, path_env: &str) -> String {
    if bin.contains('/') || (cfg!(windows) && bin.contains('\\')) {
        return bin.to_string();
    }

    let sep = if cfg!(windows) { ';' } else { ':' };
    let exts: &[&str] = if cfg!(windows) {
        &[".cmd", ".exe", ".bat", ""]
    } else {
        &[""]
    };

    for dir in path_env.split(sep).filter(|dir| !dir.is_empty()) {
        for ext in exts {
            let candidate = std::path::Path::new(dir).join(format!("{bin}{ext}"));
            if candidate.try_exists().unwrap_or(false) {
                #[cfg(windows)]
                return candidate.to_string_lossy().into_owned();
                #[cfg(not(windows))]
                return bin.to_string();
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            for ext in [".cmd", ".exe", ".bat"] {
                let npm_cmd = format!("{appdata}\\npm\\{bin}{ext}");
                if std::path::Path::new(&npm_cmd).try_exists().unwrap_or(false) {
                    return npm_cmd;
                }
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            for candidate in [
                format!("{userprofile}\\AppData\\Roaming\\npm\\{bin}.cmd"),
                format!("{userprofile}\\.bun\\bin\\{bin}.exe"),
            ] {
                if std::path::Path::new(&candidate)
                    .try_exists()
                    .unwrap_or(false)
                {
                    return candidate;
                }
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        for candidate in [
            format!("{home}/.bun/bin/{bin}"),
            format!("{home}/.local/bin/{bin}"),
            format!("{home}/.npm-global/bin/{bin}"),
        ] {
            if std::path::Path::new(&candidate)
                .try_exists()
                .unwrap_or(false)
            {
                return candidate;
            }
        }
    }
    for candidate in [
        format!("/opt/homebrew/bin/{bin}"),
        format!("/usr/local/bin/{bin}"),
    ] {
        if std::path::Path::new(&candidate)
            .try_exists()
            .unwrap_or(false)
        {
            return candidate;
        }
    }

    bin.to_string()
}

fn listen_port(listen_url: &str) -> &str {
    listen_url
        .rsplit(':')
        .next()
        .and_then(|p| p.split('/').next())
        .unwrap_or("default")
}

/// Write the configured app-server token to an owner-only (0600) file that
/// `omo app-server --ws-auth <token-file>` reads.
fn write_token_file(
    dir: &std::path::Path,
    listen_url: &str,
    token: &str,
) -> std::io::Result<std::path::PathBuf> {
    create_private_dir(dir);
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("omo-appserver-{}.token", listen_port(listen_url)));
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let file = options.open(&path)?;
        // A pre-existing file keeps its old mode; tighten it explicitly.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        drop(file);
    }
    #[cfg(not(unix))]
    drop(options.open(&path)?);
    std::fs::write(&path, token)?;
    Ok(path)
}

/// Creates `dir` (and parents) owner-only on Unix; failures are left to the
/// caller's own open, which reports them.
fn create_private_dir(dir: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let _ = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir);
    }
    #[cfg(not(unix))]
    let _ = std::fs::create_dir_all(dir);
}

/// Daemon logs are rotated to `.1` once they pass this size.
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;

/// Opens the daemon log owner-only (it holds prompts and tool output), keeping a
/// single rotated generation so it cannot grow without bound.
fn open_private_log(path: &std::path::Path) -> Option<std::fs::File> {
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let _ = std::fs::rename(path, path.with_extension("log.1"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        options.mode(0o600);
        let file = options.open(path).ok()?;
        // A pre-existing log keeps its old mode; tighten it explicitly.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .ok()?;
        Some(file)
    }
    #[cfg(not(unix))]
    {
        options.open(path).ok()
    }
}

/// Token file for a token-authenticated daemon, or `None` when no token is
/// configured (the daemon then runs with `--ws-auth off`).
fn prepare_token_file(cfg: &OmoBackendConfig) -> Result<Option<std::path::PathBuf>> {
    let Some(token) = cfg.auth_token.as_deref() else {
        return Ok(None);
    };
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            OmonError::Config(
                "cannot autospawn a token-authenticated omo app-server: HOME is not set \
                 (set OMON_OMO_AUTOSPAWN=off and start the daemon yourself)"
                    .to_string(),
            )
        })?;
    let dir = std::path::PathBuf::from(home).join(".omon");
    write_token_file(&dir, &cfg.appserver_url, token)
        .map(Some)
        .map_err(|e| {
            OmonError::Config(format!(
                "cannot write omo app-server token file in {}: {e}",
                dir.display()
            ))
        })
}

/// Where the gateway records the pid of the daemon it spawned for `listen_url`, so the
/// next gateway can tell its own leftover daemon from an externally managed one.
fn daemon_pid_path(listen_url: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(
        std::path::PathBuf::from(home)
            .join(".omon")
            .join(format!("omo-appserver-{}.pid", listen_port(listen_url))),
    )
}

fn record_daemon_pid(listen_url: &str, pid: Option<u32>) {
    let (Some(path), Some(pid)) = (daemon_pid_path(listen_url), pid) else {
        return;
    };
    if let Some(dir) = path.parent() {
        create_private_dir(dir);
    }
    if let Err(error) = std::fs::write(&path, pid.to_string()) {
        tracing::warn!(path = %path.display(), %error, "failed to record omo app-server pid");
    }
}

/// The pid recorded by a previous gateway, if that process is still an
/// `app-server --listen <listen_url>` daemon. Anything else is left alone.
fn leftover_daemon_pid(listen_url: &str) -> Option<u32> {
    let path = daemon_pid_path(listen_url)?;
    let pid: u32 = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    let output = std::process::Command::new("ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let command = String::from_utf8_lossy(&output.stdout);
    let needle = format!("--listen {listen_url}");
    (command.contains("app-server") && command.contains(&needle)).then_some(pid)
}

fn process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Stops a daemon a previous gateway spawned (it outlives a gateway restart, still holds
/// the port, and would otherwise be mistaken for an external daemon nobody can restart).
async fn reclaim_leftover_daemon(listen_url: &str) {
    let Some(pid) = leftover_daemon_pid(listen_url) else {
        return;
    };
    tracing::warn!(%listen_url, pid, "stopping omo app-server left behind by a previous gateway");
    for signal in ["-TERM", "-KILL"] {
        let _ = std::process::Command::new("kill")
            .args([signal, &pid.to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && process_alive(pid) {
            sleep(Duration::from_millis(100)).await;
        }
        if !process_alive(pid) {
            break;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && probe_readyz(listen_url, READY_PROBE_TIMEOUT).await {
        sleep(Duration::from_millis(200)).await;
    }
}

/// Build the spawn command for a local daemon. Exposed for tests.
///
/// With a token file the daemon enforces the same bearer token the backend
/// sends; without one it runs unauthenticated (`--ws-auth off`).
fn daemon_command(bin: &str, listen_url: &str, token_file: Option<&std::path::Path>) -> Command {
    #[cfg(windows)]
    let mut cmd = if bin.ends_with(".cmd") || bin.ends_with(".bat") {
        let mut c = Command::new("cmd.exe");
        c.arg("/c").arg(bin);
        c
    } else {
        Command::new(bin)
    };
    #[cfg(not(windows))]
    let mut cmd = Command::new(bin);

    cmd.args(["app-server", "--listen", listen_url, "--ws-auth"]);
    match token_file {
        Some(path) => cmd.arg(path),
        None => cmd.arg("off"),
    };
    cmd.stdin(Stdio::null()).kill_on_drop(true);

    let log_file = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|home| {
            let port = listen_port(listen_url);
            let dir = std::path::PathBuf::from(home).join(".omon");
            create_private_dir(&dir);
            dir.join(format!("omo-appserver-{port}.log"))
        })
        .and_then(|path| open_private_log(&path));

    if let Some(file) = log_file {
        if let Ok(file_clone) = file.try_clone() {
            cmd.stdout(Stdio::from(file));
            cmd.stderr(Stdio::from(file_clone));
            return cmd;
        }
    }

    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    cmd
}

fn autospawn_enabled() -> bool {
    !matches!(
        std::env::var("OMON_OMO_AUTOSPAWN")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "off" | "false" | "0"
    )
}

/// Owns the spawned daemon child for the lifetime of the gateway and
/// restarts it, retrying with capped exponential backoff for as long as the
/// gateway runs, if it dies. Killing the supervisor kills the daemon.
pub struct OmoDaemonSupervisor {
    child: ChildSlot,
    shutdown: Arc<AtomicBool>,
}

impl OmoDaemonSupervisor {
    /// False once the supervisor has been shut down. A daemon that keeps failing
    /// is retried indefinitely rather than abandoned, so this never reports a
    /// give-up. Live backend readiness still requires probing the daemon's
    /// readyz endpoint.
    pub fn is_available(&self) -> bool {
        !self.shutdown.load(Ordering::Acquire)
    }

    /// Ensure a daemon is serving `cfg.appserver_url`, spawning a local one
    /// when necessary. Returns `Ok(None)` when an externally managed daemon
    /// is already serving (or autospawn is disabled / URL is non-local).
    pub async fn ensure(cfg: &OmoBackendConfig) -> Result<Option<Self>> {
        let _guard = SPAWN_LOCK.lock().await;
        let bin = resolve_daemon_bin(
            &std::env::var("OMON_OMO_BIN").unwrap_or_else(|_| "omo".to_string()),
            &std::env::var("PATH").unwrap_or_default(),
        );
        if !autospawn_enabled() || !is_local_url(&cfg.appserver_url) {
            return Ok(None);
        }
        reclaim_leftover_daemon(&cfg.appserver_url).await;
        if probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await {
            tracing::debug!(url = %cfg.appserver_url, "external omo app-server already serving");
            return Ok(None);
        }

        let token_file = prepare_token_file(cfg)?;
        tracing::info!(url = %cfg.appserver_url, bin = %bin, "spawning omo app-server daemon");
        let child = daemon_command(&bin, &cfg.appserver_url, token_file.as_deref())
            .spawn()
            .map_err(|e| {
                OmonError::Config(format!(
                    "failed to spawn '{bin} app-server' for {url}: {e} \
                     (is the omo CLI installed? set OMON_OMO_BIN or OMON_OMO_AUTOSPAWN=off to manage the daemon yourself)",
                    url = cfg.appserver_url
                ))
            })?;

        record_daemon_pid(&cfg.appserver_url, child.id());
        let supervisor = Self {
            child: Arc::new(Mutex::new(Some(child))),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        if let Ok(mut owned) = OWNED_DAEMONS.lock() {
            owned.insert(cfg.appserver_url.clone(), Arc::downgrade(&supervisor.child));
        }

        let deadline = Instant::now() + READY_WAIT_AFTER_SPAWN;
        while Instant::now() < deadline {
            if probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await {
                tracing::info!(url = %cfg.appserver_url, "omo app-server daemon ready");
                supervisor.spawn_watcher(cfg.appserver_url.clone(), bin, token_file);
                return Ok(Some(supervisor));
            }
            if supervisor.child_exited() {
                break;
            }
            sleep(Duration::from_millis(400)).await;
        }

        supervisor.kill();
        Err(OmonError::Config(format!(
            "spawned 'omo app-server' did not become ready at {} within {}s",
            cfg.appserver_url,
            READY_WAIT_AFTER_SPAWN.as_secs()
        )))
    }

    fn child_exited(&self) -> bool {
        self.child
            .try_lock()
            .ok()
            .and_then(|mut guard| {
                guard
                    .as_mut()
                    .and_then(|child| child.try_wait().ok().flatten())
                    .map(|_| true)
            })
            .unwrap_or(false)
    }

    fn spawn_watcher(&self, url: String, bin: String, token_file: Option<std::path::PathBuf>) {
        self.spawn_watcher_with_backoff(url, bin, token_file, RESTART_BACKOFF, RESTART_BACKOFF_MAX);
    }

    fn spawn_watcher_with_backoff(
        &self,
        url: String,
        bin: String,
        token_file: Option<std::path::PathBuf>,
        backoff_base: Duration,
        backoff_cap: Duration,
    ) {
        let child_slot = Arc::clone(&self.child);
        let shutdown = Arc::clone(&self.shutdown);
        tokio::spawn(async move {
            // Consecutive restart cycles that did not end in a daemon that stayed
            // healthy; drives the backoff. There is deliberately no give-up state:
            // the gateway keeps running after a failed restart, so abandoning the
            // daemon would leave every turn failing until a manual restart.
            let mut failures: u32 = 0;
            let mut healthy_since = Some(Instant::now());
            loop {
                if shutdown.load(Ordering::Acquire) {
                    return;
                }
                loop {
                    if shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    let exited = {
                        let mut guard = child_slot.lock().await;
                        match guard.as_mut() {
                            Some(child) => matches!(child.try_wait(), Ok(Some(_))),
                            None => true,
                        }
                    };
                    if exited {
                        break;
                    }
                    sleep(Duration::from_millis(500)).await;
                }
                if shutdown.load(Ordering::Acquire) {
                    return;
                }
                if healthy_since
                    .take()
                    .is_some_and(|since| since.elapsed() >= RESTART_WINDOW)
                {
                    failures = 0;
                }
                let delay = restart_backoff(failures, backoff_base, backoff_cap);
                failures = failures.saturating_add(1);
                tracing::warn!(
                    url = %url,
                    retry_in_secs = delay.as_secs_f32(),
                    "omo app-server daemon is down; restarting after backoff"
                );
                sleep(delay).await;
                if shutdown.load(Ordering::Acquire) {
                    return;
                }
                // A forked survivor or an external daemon may have taken over
                // the port; adopt it instead of spawning a child that would
                // die on bind conflict forever.
                if probe_readyz(&url, READY_PROBE_TIMEOUT).await {
                    tracing::info!(
                        url = %url,
                        "port already serving after daemon exit; deferring to the external daemon"
                    );
                    return;
                }
                match daemon_command(&bin, &url, token_file.as_deref()).spawn() {
                    Ok(child) => {
                        record_daemon_pid(&url, child.id());
                        child_slot.lock().await.replace(child);
                        let deadline = Instant::now() + READY_WAIT_AFTER_SPAWN;
                        let mut ready = false;
                        let mut exited = false;
                        while Instant::now() < deadline {
                            if shutdown.load(Ordering::Acquire) {
                                return;
                            }
                            let remaining = deadline.saturating_duration_since(Instant::now());
                            if probe_readyz(&url, READY_PROBE_TIMEOUT.min(remaining)).await {
                                tracing::info!(url = %url, "restarted omo app-server daemon ready");
                                ready = true;
                                break;
                            }
                            exited = child_slot
                                .lock()
                                .await
                                .as_mut()
                                .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))));
                            if exited {
                                break;
                            }
                            sleep(Duration::from_millis(200)).await;
                        }
                        if ready {
                            healthy_since = Some(Instant::now());
                        } else if !exited {
                            if let Some(mut child) = child_slot.lock().await.take() {
                                if let Err(error) = child.kill().await {
                                    tracing::error!(%error, "failed to kill/reap unready omo app-server");
                                }
                            }
                            tracing::error!(url = %url, "omo app-server replacement readiness deadline expired; will retry");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to restart omo app-server daemon; will retry");
                    }
                }
            }
        });
    }

    fn kill(&self) {
        self.shutdown.store(true, Ordering::Release);
        if let Ok(mut guard) = self.child.try_lock() {
            if let Some(child) = guard.as_mut() {
                let _ = child.start_kill();
            }
        }
    }
}

impl Drop for OmoDaemonSupervisor {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Kills the gateway-owned daemon serving `url` (SIGKILL: a wedged app-server can ignore
/// SIGTERM for minutes) and waits until its supervisor has a replacement answering
/// readyz. Returns false when the daemon is not owned by this gateway, a forced restart
/// happened too recently, or no replacement became ready in time.
pub async fn force_restart_owned_daemon(url: &str) -> bool {
    {
        let Ok(mut last) = LAST_FORCED_RESTART.lock() else {
            return false;
        };
        if last
            .get(url)
            .is_some_and(|at| at.elapsed() < FORCED_RESTART_MIN_INTERVAL)
        {
            tracing::warn!(%url, "omo app-server was force-restarted recently; not restarting again");
            return false;
        }
        last.insert(url.to_owned(), Instant::now());
    }
    let slot = OWNED_DAEMONS
        .lock()
        .ok()
        .and_then(|owned| owned.get(url).and_then(std::sync::Weak::upgrade));
    let Some(slot) = slot else {
        tracing::warn!(%url, "omo app-server is not owned by this gateway; cannot restart it");
        return false;
    };
    {
        let mut guard = slot.lock().await;
        let Some(child) = guard.as_mut() else {
            return false;
        };
        tracing::warn!(%url, pid = ?child.id(), "force-restarting unresponsive omo app-server");
        let _ = child.start_kill();
        // Reap it here so the supervisor sees the exit on its next poll.
        let _ = timeout(Duration::from_secs(5), child.wait()).await;
    }
    let deadline = Instant::now() + READY_WAIT_AFTER_SPAWN + RESTART_BACKOFF_MAX;
    while Instant::now() < deadline {
        let replaced = slot
            .lock()
            .await
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if replaced && probe_readyz(url, READY_PROBE_TIMEOUT).await {
            tracing::info!(%url, "omo app-server replacement ready after forced restart");
            return true;
        }
        sleep(Duration::from_millis(500)).await;
    }
    tracing::error!(%url, "omo app-server did not come back after forced restart");
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Re-exec only this test: daemon_command opens logs in the supervisor's
    // HOME, so changing just the spawned daemon's environment is insufficient.
    async fn isolated_home(test: &str) -> bool {
        let id = format!("agent::omo_daemon::tests::{test}");
        if std::env::var("U62_ISOLATED_TEST").as_deref() == Ok(id.as_str()) {
            return false;
        }
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".omon")).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([&id, "--exact", "--nocapture"])
            .env("U62_ISOLATED_TEST", &id)
            .env("HOME", home.path())
            .env("OMON_OMO_AUTOSPAWN", "on")
            .kill_on_drop(true);
        let output = timeout(Duration::from_secs(45), command.output())
            .await
            .unwrap()
            .unwrap();
        println!(
            "isolated {id}: {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            for entry in std::fs::read_dir(home.path().join(".omon")).unwrap() {
                let path = entry.unwrap().path();
                println!(
                    "fixture log {}:\n{}",
                    path.display(),
                    std::fs::read_to_string(&path).unwrap()
                );
            }
        }
        assert!(output.status.success());
        if matches!(
            test,
            "daemon_exiting_child_is_restarted_past_the_old_budget"
                | "daemon_unready_replacement_is_killed_and_retried"
                | "daemon_ensure_owned_local_surface"
                | "force_restart_replaces_the_owned_daemon_and_throttles"
                | "test_daemon_command_arguments"
        ) {
            assert!(
                std::fs::read_dir(home.path().join(".omon"))
                    .unwrap()
                    .next()
                    .is_some(),
                "production log opening must execute inside the isolated HOME"
            );
        }
        let home_path = home.path().to_owned();
        home.close().unwrap();
        assert!(!home_path.exists());
        println!("isolated {id}: subprocess reaped; HOME removed");
        true
    }

    /// More consecutive crashes than the old three-strike circuit tolerated.
    const RESTARTS_PAST_OLD_LIMIT: usize = 5;

    #[tokio::test]
    #[cfg(unix)]
    async fn daemon_exiting_child_is_restarted_past_the_old_budget() {
        if isolated_home("daemon_exiting_child_is_restarted_past_the_old_budget").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let events = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bin = dir.path().join("exit-daemon");
        std::fs::write(
            &bin,
            format!(
                r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import socket
s = socket.create_connection(('127.0.0.1', {}))
s.sendall(b'S')
s.recv(1)
"#,
                events.local_addr().unwrap().port()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        let supervisor = OmoDaemonSupervisor {
            child: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        supervisor.spawn_watcher_with_backoff(
            "invalid-url".into(),
            bin.to_string_lossy().into_owned(),
            None,
            Duration::from_millis(10),
            Duration::from_millis(40),
        );
        for _ in 0..RESTARTS_PAST_OLD_LIMIT {
            let (mut event, _) = timeout(Duration::from_secs(10), events.accept())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(event.read_u8().await.unwrap(), b'S');
            // Observe actual process exit before the watcher may restart it.
            let mut slot = supervisor.child.lock().await;
            event.write_all(b"X").await.unwrap();
            timeout(Duration::from_secs(5), slot.as_mut().unwrap().wait())
                .await
                .unwrap()
                .unwrap();
            drop(slot);
            assert!(
                supervisor.is_available(),
                "a crash-looping daemon must keep being retried, never abandoned"
            );
        }
        supervisor.kill();
    }

    #[test]
    fn restart_backoff_grows_exponentially_and_is_capped() {
        let base = Duration::from_secs(2);
        let cap = Duration::from_secs(60);
        let delays: Vec<u64> = (0..8)
            .map(|failures| restart_backoff(failures, base, cap).as_secs())
            .collect();
        assert_eq!(delays, vec![2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(restart_backoff(u32::MAX, base, cap), cap);
    }

    #[test]
    #[cfg(unix)]
    fn fixr5_daemon_log_is_owner_only_and_rotated() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("omo-appserver-1.log");
        std::fs::write(&log, b"old").unwrap();
        std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o644)).unwrap();
        drop(open_private_log(&log).expect("log opens"));
        assert_eq!(
            std::fs::metadata(&log).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let oversized = std::fs::OpenOptions::new().write(true).open(&log).unwrap();
        oversized.set_len(MAX_LOG_BYTES + 1).unwrap();
        drop(oversized);
        drop(open_private_log(&log).expect("log reopens after rotation"));
        assert_eq!(std::fs::metadata(&log).unwrap().len(), 0);
        assert_eq!(
            std::fs::metadata(log.with_extension("log.1"))
                .unwrap()
                .len(),
            MAX_LOG_BYTES + 1
        );
    }

    #[tokio::test]
    async fn daemon_external_takeover_is_untouched() {
        if isolated_home("daemon_external_takeover_is_untouched").await {
            return;
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let supervisor = OmoDaemonSupervisor {
            child: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = [0; 1024];
                let _ = socket.read(&mut buf).await.unwrap();
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await
                    .unwrap();
            }
        });
        supervisor.spawn_watcher(url.clone(), "/nonexistent/U62-daemon".into(), None);
        // The external server survives supervisor shutdown; no child is adopted.
        assert!(probe_readyz(&url, Duration::from_secs(5)).await);
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert!(supervisor.child.lock().await.is_none());
        assert!(supervisor.is_available());
        supervisor.kill();
    }

    #[tokio::test]
    async fn daemon_ensure_external_local_surface() {
        if isolated_home("daemon_ensure_external_local_surface").await {
            return;
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cfg = OmoBackendConfig::new(format!("ws://{}", listener.local_addr().unwrap()));
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .unwrap();
        });
        assert!(OmoDaemonSupervisor::ensure(&cfg).await.unwrap().is_none());
        timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn daemon_ensure_owned_local_surface() {
        if isolated_home("daemon_ensure_owned_local_surface").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let port = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cfg = OmoBackendConfig::new(format!("ws://{}", port.local_addr().unwrap()));
        drop(port);
        let bin = dir.path().join("ready-daemon");
        std::fs::write(
            &bin,
            r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', int(sys.argv[3].rsplit(':', 1)[1])))
s.listen()
while True:
    c, _ = s.accept()
    c.recv(4096)
    c.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n')
    c.close()
"#,
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        // This process re-executes exactly one test with its own HOME/env.
        std::env::set_var("OMON_OMO_BIN", &bin);
        let supervisor = timeout(Duration::from_secs(10), OmoDaemonSupervisor::ensure(&cfg))
            .await
            .unwrap()
            .unwrap()
            .expect("public ensure must own the newly spawned daemon");
        assert!(supervisor.is_available());
        assert!(probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await);
        supervisor.kill();
        let mut slot = supervisor.child.lock().await;
        let status = timeout(Duration::from_secs(5), slot.as_mut().unwrap().wait())
            .await
            .unwrap()
            .unwrap();
        assert!(!status.success());
        assert!(!supervisor.is_available());
        assert!(!probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await);
        println!(
            "public ensure: owned HTTP-200 child ready, shutdown child reaped, socket refused"
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn leftover_daemon_from_a_previous_gateway_is_reclaimed() {
        if isolated_home("leftover_daemon_from_a_previous_gateway_is_reclaimed").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let port = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cfg = OmoBackendConfig::new(format!("ws://{}", port.local_addr().unwrap()));
        drop(port);
        let bin = dir.path().join("ready-daemon");
        std::fs::write(
            &bin,
            r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', int(sys.argv[3].rsplit(':', 1)[1])))
s.listen()
while True:
    c, _ = s.accept()
    c.recv(4096)
    c.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n')
    c.close()
"#,
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::env::set_var("OMON_OMO_BIN", &bin);

        // Given: a daemon a previous gateway spawned, still serving, recorded in the pidfile.
        let mut leftover = std::process::Command::new(&bin)
            .args([
                "app-server",
                "--listen",
                &cfg.appserver_url,
                "--ws-auth",
                "off",
            ])
            .spawn()
            .unwrap();
        let leftover_pid = leftover.id();
        let ready_by = Instant::now() + Duration::from_secs(10);
        while !probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await {
            assert!(
                Instant::now() < ready_by,
                "fixture daemon never became ready"
            );
            sleep(Duration::from_millis(100)).await;
        }
        record_daemon_pid(&cfg.appserver_url, Some(leftover_pid));

        // When: a new gateway ensures its daemon.
        let supervisor = timeout(Duration::from_secs(30), OmoDaemonSupervisor::ensure(&cfg))
            .await
            .unwrap()
            .unwrap()
            .expect("the leftover is reclaimed, so this gateway owns the new daemon");

        // Then: the leftover is gone and the owned child serves the URL.
        let _ = leftover.wait();
        assert!(
            !process_alive(leftover_pid),
            "leftover daemon must be stopped"
        );
        let owned_pid = supervisor.child.lock().await.as_ref().and_then(Child::id);
        assert!(owned_pid.is_some_and(|pid| pid != leftover_pid));
        assert!(probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await);
        let recorded =
            std::fs::read_to_string(daemon_pid_path(&cfg.appserver_url).unwrap()).unwrap();
        assert_eq!(Some(recorded.trim().parse::<u32>().unwrap()), owned_pid);
        supervisor.kill();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn unrelated_pid_in_the_pidfile_is_never_killed() {
        if isolated_home("unrelated_pid_in_the_pidfile_is_never_killed").await {
            return;
        }
        let url = "ws://127.0.0.1:1";
        let mut bystander = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        record_daemon_pid(url, Some(bystander.id()));
        assert_eq!(leftover_daemon_pid(url), None);
        reclaim_leftover_daemon(url).await;
        assert!(
            process_alive(bystander.id()),
            "a non-daemon process must survive"
        );
        let _ = bystander.kill();
        let _ = bystander.wait();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn force_restart_replaces_the_owned_daemon_and_throttles() {
        if isolated_home("force_restart_replaces_the_owned_daemon_and_throttles").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let port = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cfg = OmoBackendConfig::new(format!("ws://{}", port.local_addr().unwrap()));
        drop(port);
        let bin = dir.path().join("ready-daemon");
        std::fs::write(
            &bin,
            r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', int(sys.argv[3].rsplit(':', 1)[1])))
s.listen()
while True:
    c, _ = s.accept()
    c.recv(4096)
    c.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n')
    c.close()
"#,
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::env::set_var("OMON_OMO_BIN", &bin);
        let supervisor = timeout(Duration::from_secs(10), OmoDaemonSupervisor::ensure(&cfg))
            .await
            .unwrap()
            .unwrap()
            .expect("ensure must own the spawned daemon");
        let first_pid = supervisor.child.lock().await.as_ref().and_then(Child::id);

        assert!(force_restart_owned_daemon(&cfg.appserver_url).await);
        let second_pid = supervisor.child.lock().await.as_ref().and_then(Child::id);
        assert!(second_pid.is_some());
        assert_ne!(
            first_pid, second_pid,
            "a new daemon process must be serving"
        );
        assert!(probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await);

        // A second stall right after must not trigger another restart.
        assert!(!force_restart_owned_daemon(&cfg.appserver_url).await);
        assert_eq!(
            supervisor.child.lock().await.as_ref().and_then(Child::id),
            second_pid
        );
        // A daemon this gateway does not own is never touched.
        assert!(!force_restart_owned_daemon("ws://127.0.0.1:1").await);
        supervisor.kill();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn daemon_unready_replacement_is_killed_and_retried() {
        if isolated_home("daemon_unready_replacement_is_killed_and_retried").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        // Given an exited owned daemon and a replacement that stays alive,
        // announces its PID over a real socket, and only serves HTTP 503.
        let dir = tempfile::tempdir().unwrap();
        let events = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", port.local_addr().unwrap());
        drop(port);
        let bin = dir.path().join("daemon");
        std::fs::write(
            &bin,
            format!(
                r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import os, socket, sys
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', int(sys.argv[3].rsplit(':', 1)[1])))
s.listen()
event = socket.create_connection(('127.0.0.1', {}))
event.sendall(str(os.getpid()).encode() + b'\n')
while True:
    c, _ = s.accept()
    c.recv(4096)
    c.sendall(b'HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\n\r\n')
    c.close()
"#,
                events.local_addr().unwrap().port()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut initial = Command::new("/usr/bin/true").spawn().unwrap();
        initial.wait().await.unwrap();
        let supervisor = OmoDaemonSupervisor {
            child: Arc::new(Mutex::new(Some(initial))),
            shutdown: Arc::new(AtomicBool::new(false)),
        };
        // Subscribe before starting the real production watcher.
        let started = events.accept();
        supervisor.spawn_watcher_with_backoff(
            url.clone(),
            bin.to_string_lossy().into_owned(),
            None,
            Duration::from_millis(10),
            Duration::from_millis(40),
        );
        let (mut event, _) = timeout(Duration::from_secs(10), started)
            .await
            .unwrap()
            .unwrap();
        let mut first_pid = Vec::new();
        loop {
            let byte = event.read_u8().await.unwrap();
            if byte == b'\n' {
                break;
            }
            first_pid.push(byte);
        }
        assert!(!probe_readyz(&url, READY_PROBE_TIMEOUT).await);
        // Subscribe to the retry before the readiness bound is crossed.
        let retried = events.accept();
        // Time is the behavior under test: advance beyond the readiness bound.
        tokio::time::pause();
        tokio::time::advance(READY_WAIT_AFTER_SPAWN + Duration::from_secs(3)).await;
        tokio::time::resume();
        // The unready replacement is killed (its event socket closes)...
        let eof = timeout(Duration::from_secs(5), event.read_u8()).await;
        // ...and the supervisor retries instead of giving up.
        let (mut second_event, _) = timeout(Duration::from_secs(10), retried)
            .await
            .expect("supervisor must start another daemon after the readiness deadline")
            .unwrap();
        let mut second_pid = Vec::new();
        loop {
            let byte = second_event.read_u8().await.unwrap();
            if byte == b'\n' {
                break;
            }
            second_pid.push(byte);
        }
        let available = supervisor.is_available();
        supervisor.kill();
        assert!(
            matches!(eof, Ok(Err(_))),
            "owned HTTP-503 replacement PID {} survived readiness deadline",
            String::from_utf8_lossy(&first_pid)
        );
        assert!(
            available,
            "an unready replacement must not permanently disable the supervisor"
        );
        assert_ne!(first_pid, second_pid, "retry must be a fresh process");
    }

    #[test]
    fn test_is_local_url() {
        assert!(is_local_url("ws://127.0.0.1:19742"));
        assert!(is_local_url("ws://localhost:19742"));
        assert!(is_local_url("ws://LOCALHOST:9"));
        assert!(!is_local_url("ws://10.1.2.3:19742"));
        assert!(!is_local_url("wss://relay.example.com"));
        assert!(!is_local_url("ws://localhost:pw@evil.example"));
        assert!(!is_local_url("ws://localhost@evil.example:80"));
    }

    #[tokio::test]
    async fn test_probe_readyz_detects_http_200_and_refusal() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let _ = sock.read(&mut buf).await;
            sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .await
                .unwrap();
        });
        assert!(probe_readyz(&format!("ws://{addr}"), Duration::from_secs(2)).await);
        server.await.unwrap();

        // Nothing listening on this ephemeral port range slot: probe fails.
        let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead_port = dead.local_addr().unwrap().port();
        drop(dead);
        assert!(
            !probe_readyz(
                &format!("ws://127.0.0.1:{dead_port}"),
                Duration::from_secs(2)
            )
            .await
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_resolve_daemon_bin_falls_back_to_known_install_paths() {
        // launchd hands the gateway a minimal PATH (/usr/bin:/bin:/usr/sbin:/sbin)
        // that excludes ~/.bun/bin, so a bare "omo" fails to spawn with ENOENT.
        // Resolution must fall back to well-known absolute install locations.
        let home = std::env::var("HOME").unwrap();
        let bun_omo = format!("{home}/.bun/bin/omo");
        if !std::path::Path::new(&bun_omo).exists() {
            return; // fallback target absent on this machine
        }

        let resolved = resolve_daemon_bin("omo", "/usr/bin:/bin:/usr/sbin:/sbin");
        assert_eq!(
            resolved, bun_omo,
            "bare 'omo' must resolve to an absolute path when PATH lacks the install dir"
        );

        // An explicit absolute override is always honoured verbatim.
        assert_eq!(
            resolve_daemon_bin("/custom/omo", "/usr/bin:/bin"),
            "/custom/omo"
        );
    }

    #[tokio::test]
    async fn test_daemon_command_arguments() {
        if isolated_home("test_daemon_command_arguments").await {
            return;
        }
        let cmd = daemon_command("omo", "ws://127.0.0.1:19742", None);
        // Command internals are not inspectable portably; assert via as_std.
        let std_cmd = cmd.as_std();
        let args: Vec<String> = std_cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            args,
            vec![
                "app-server",
                "--listen",
                "ws://127.0.0.1:19742",
                "--ws-auth",
                "off"
            ]
        );
        assert_eq!(std_cmd.get_program().to_string_lossy(), "omo");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn fixr1_daemon_command_uses_token_file_when_token_configured() {
        if isolated_home("fixr1_daemon_command_uses_token_file_when_token_configured").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let token_file =
            write_token_file(dir.path(), "ws://127.0.0.1:19742", "secret-token").unwrap();
        assert_eq!(
            std::fs::read_to_string(&token_file).unwrap(),
            "secret-token"
        );
        let mode = std::fs::metadata(&token_file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        let cmd = daemon_command("omo", "ws://127.0.0.1:19742", Some(&token_file));
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            args,
            vec![
                "app-server".to_string(),
                "--listen".to_string(),
                "ws://127.0.0.1:19742".to_string(),
                "--ws-auth".to_string(),
                token_file.to_string_lossy().to_string(),
            ]
        );
        assert!(!args.iter().any(|a| a == "off"));
    }
}

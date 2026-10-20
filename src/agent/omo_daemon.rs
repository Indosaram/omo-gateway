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
static LAST_FORCED_RESTART: LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
> = LazyLock::new(Default::default);
/// Minimum spacing between two forced restarts of the same daemon.
const FORCED_RESTART_MIN_INTERVAL: Duration = Duration::from_secs(120);

const READY_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const READY_WAIT_AFTER_SPAWN: Duration = Duration::from_secs(30);
/// Longest readiness wait for a spawned daemon. Under heavy host load a healthy app-server
/// can need well over READY_WAIT_AFTER_SPAWN; only a child that is still unready after this
/// is treated as hung and replaced.
const READY_WAIT_MAX: Duration = Duration::from_secs(180);

/// Readiness deadline for the attempt after `failures` consecutive unready restarts: it
/// grows so sustained load cannot turn every replacement into a kill-and-retry loop.
fn ready_wait(failures: u32) -> Duration {
    READY_WAIT_AFTER_SPAWN
        .saturating_mul(failures.saturating_add(1))
        .min(READY_WAIT_MAX)
}
/// First delay before a restart attempt; it doubles per consecutive failure.
const RESTART_BACKOFF: Duration = Duration::from_secs(2);
/// Upper bound of the restart delay: the supervisor keeps retrying at this pace.
const RESTART_BACKOFF_MAX: Duration = Duration::from_secs(60);
/// A daemon that stayed ready this long before exiting counts as healthy, so its
/// exit starts a fresh outage (backoff reset) instead of extending a crash loop.
const RESTART_WINDOW: Duration = Duration::from_secs(60);
/// How often the supervisor probes readyz of a running daemon that has been ready.
const LIVENESS_INTERVAL: Duration = Duration::from_secs(20);
/// Consecutive failed liveness probes after which a running daemon counts as wedged.
/// Incident 2026-10-16: one session's blocking file read (an iCloud dataless file)
/// froze the whole app-server process for 4h38m while it stayed alive, so an exit
/// check alone never restarted it.
const LIVENESS_FAILURES: u32 = 3;

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
    // The owner pid lets a later process tell a daemon whose gateway died (reclaim it) from
    // one a still-running gateway or dashboard is serving (leave it alone).
    let record = format!("{pid} {}", std::process::id());
    if let Err(error) = std::fs::write(&path, record) {
        tracing::warn!(path = %path.display(), %error, "failed to record omo app-server pid");
    }
}

/// The pidfile names a daemon spawned by another omon process that is still running.
fn daemon_owned_by_other_live_process(listen_url: &str) -> bool {
    let Some(record) =
        daemon_pid_path(listen_url).and_then(|path| std::fs::read_to_string(path).ok())
    else {
        return false;
    };
    record
        .split_whitespace()
        .nth(1)
        .and_then(|owner| owner.parse::<u32>().ok())
        .is_some_and(|owner| owner != std::process::id() && process_alive(owner))
}

/// The pid recorded by a previous gateway, if that process is still an
/// `app-server --listen <listen_url>` daemon and the process that spawned it is gone.
/// Anything else is left alone.
fn leftover_daemon_pid(listen_url: &str) -> Option<u32> {
    let path = daemon_pid_path(listen_url)?;
    let record = std::fs::read_to_string(path).ok()?;
    let mut fields = record.split_whitespace();
    let pid: u32 = fields.next()?.parse().ok()?;
    if let Some(owner) = fields.next().and_then(|owner| owner.parse::<u32>().ok()) {
        if owner != std::process::id() && process_alive(owner) {
            return None;
        }
    }
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

/// The app-server drops turn events after a thread is reloaded unless its TurnEngine
/// subscription is patched. omo 5.1.35 removed `omo app-server`, so the gateway launches a
/// preserved runtime through a wrapper script (OMON_OMO_BIN); the patch lives in that
/// runtime, never in the global senpi bundle. Returns the checked file and whether it is
/// patched, or None when `bin` is not such a wrapper (nothing to check).
fn app_server_patch_state(bin: &str) -> Option<(std::path::PathBuf, bool)> {
    let wrapper = std::fs::read_to_string(bin).ok()?;
    let cli_main = wrapper
        .split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
        .find(|word| word.ends_with("/dist/cli-main.js"))?;
    let turns = std::path::Path::new(cli_main)
        .parent()?
        .join("modes/app-server/threads/turns.js");
    let source = std::fs::read_to_string(&turns).ok()?;
    Some((turns, source.contains("_turnEngineSubscribed")))
}

fn warn_if_app_server_unpatched(bin: &str) {
    if let Some((path, false)) = app_server_patch_state(bin) {
        tracing::warn!(
            path = %path.display(),
            "app-server runtime lacks the TurnEngine subscription patch; stalled turns will be \
             recovered by restarting the app-server, but replies may lag"
        );
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

    let mut prepend_dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        prepend_dirs.push(std::path::PathBuf::from(home).join(".omon").join("bin"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            prepend_dirs.push(dir.to_path_buf());
        }
    }

    let inherited_path = std::env::var_os("PATH");
    let mut path_entries: Vec<std::path::PathBuf> = Vec::new();

    for dir in prepend_dirs {
        if !path_entries.contains(&dir) {
            path_entries.push(dir);
        }
    }

    if let Some(ref path_os) = inherited_path {
        for entry in std::env::split_paths(path_os) {
            if !path_entries.contains(&entry) {
                path_entries.push(entry);
            }
        }
    }

    if let Ok(new_path) = std::env::join_paths(path_entries) {
        cmd.env("PATH", new_path);
    }

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
        warn_if_app_server_unpatched(&bin);
        reclaim_leftover_daemon(&cfg.appserver_url).await;
        if probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await {
            // A daemon spawned by another live omon process (a standalone dashboard) dies
            // with that process; supervise the port so this one takes over afterwards.
            if daemon_owned_by_other_live_process(&cfg.appserver_url) {
                tracing::info!(url = %cfg.appserver_url, "omo app-server owned by another omon process; supervising for takeover");
                let token_file = prepare_token_file(cfg)?;
                let supervisor = Self {
                    child: Arc::new(Mutex::new(None)),
                    shutdown: Arc::new(AtomicBool::new(false)),
                };
                if let Ok(mut owned) = OWNED_DAEMONS.lock() {
                    owned.insert(cfg.appserver_url.clone(), Arc::downgrade(&supervisor.child));
                }
                supervisor.spawn_watcher(cfg.appserver_url.clone(), bin, token_file);
                return Ok(Some(supervisor));
            }
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
        let mut exited_early = false;
        while Instant::now() < deadline {
            if probe_readyz(&cfg.appserver_url, READY_PROBE_TIMEOUT).await {
                tracing::info!(url = %cfg.appserver_url, "omo app-server daemon ready");
                supervisor.spawn_watcher(cfg.appserver_url.clone(), bin, token_file);
                return Ok(Some(supervisor));
            }
            if supervisor.child_exited() {
                exited_early = true;
                break;
            }
            sleep(Duration::from_millis(400)).await;
        }

        // A slow start (heavy host load) must not take the whole gateway down: exiting here
        // made launchd restart it into the same timeout forever. Keep running and let the
        // watcher own the daemon: it respawns it once it exits, and kills a child that is
        // still unready after READY_WAIT_MAX so a hung daemon is replaced too.
        if exited_early {
            tracing::error!(
                url = %cfg.appserver_url,
                "omo app-server exited before becoming ready; supervising and restarting it"
            );
        } else {
            tracing::warn!(
                url = %cfg.appserver_url,
                wait_secs = READY_WAIT_AFTER_SPAWN.as_secs(),
                "omo app-server daemon not ready yet; continuing startup and supervising it"
            );
            let slot = Arc::clone(&supervisor.child);
            let shutdown = Arc::clone(&supervisor.shutdown);
            let url = cfg.appserver_url.clone();
            // Only this first child may be killed here; the watcher can swap in a replacement.
            let first_pid = supervisor
                .child
                .try_lock()
                .ok()
                .and_then(|g| g.as_ref().and_then(Child::id));
            tokio::spawn(async move {
                let deadline = Instant::now() + READY_WAIT_MAX - READY_WAIT_AFTER_SPAWN;
                while Instant::now() < deadline {
                    if shutdown.load(Ordering::Acquire) {
                        return;
                    }
                    if probe_readyz(&url, READY_PROBE_TIMEOUT).await {
                        tracing::info!(url = %url, "omo app-server daemon ready after slow start");
                        return;
                    }
                    let exited = slot.lock().await.as_mut().is_none_or(|child| {
                        child.id() != first_pid || matches!(child.try_wait(), Ok(Some(_)))
                    });
                    if exited {
                        return;
                    }
                    sleep(Duration::from_millis(500)).await;
                }
                // Still alive and never ready: kill it so the watcher sees an exit and respawns.
                if let Some(child) = slot.lock().await.as_mut() {
                    if child.id() == first_pid {
                        tracing::error!(url = %url, "omo app-server never became ready; replacing it");
                        let _ = child.start_kill();
                    }
                }
            });
        }
        supervisor.spawn_watcher(cfg.appserver_url.clone(), bin, token_file);
        Ok(Some(supervisor))
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
        self.spawn_watcher_with_backoff(
            url,
            bin,
            token_file,
            RESTART_BACKOFF,
            RESTART_BACKOFF_MAX,
            LIVENESS_INTERVAL,
        );
    }

    fn spawn_watcher_with_backoff(
        &self,
        url: String,
        bin: String,
        token_file: Option<std::path::PathBuf>,
        backoff_base: Duration,
        backoff_cap: Duration,
        liveness_interval: Duration,
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
                // A daemon that is alive but no longer answers readyz is wedged: kill it
                // so the restart path below replaces it. Probing starts only once this
                // daemon has answered at least once, so a slow start is never killed here.
                let mut answered = false;
                let mut missed: u32 = 0;
                let mut next_probe = Instant::now();
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
                    if Instant::now() >= next_probe {
                        next_probe = Instant::now() + liveness_interval;
                        if probe_readyz(&url, READY_PROBE_TIMEOUT).await {
                            answered = true;
                            missed = 0;
                        } else if answered {
                            missed += 1;
                            if missed >= LIVENESS_FAILURES {
                                if let Ok(mut last) = LAST_FORCED_RESTART.lock() {
                                    last.insert(url.clone(), std::time::Instant::now());
                                }
                                let mut guard = child_slot.lock().await;
                                if let Some(child) = guard.as_mut() {
                                    tracing::warn!(
                                        url = %url,
                                        pid = ?child.id(),
                                        missed,
                                        "omo app-server alive but not answering readyz; killing it"
                                    );
                                    let _ = child.start_kill();
                                    let _ = timeout(Duration::from_secs(5), child.wait()).await;
                                }
                                break;
                            }
                        }
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
                // the port; use it instead of spawning a child that would die on
                // bind conflict, but keep supervising: when that daemon goes away
                // (a standalone dashboard exiting), this gateway respawns its own.
                if probe_readyz(&url, READY_PROBE_TIMEOUT).await {
                    tracing::info!(
                        url = %url,
                        "port already serving after daemon exit; deferring to the external daemon"
                    );
                    while probe_readyz(&url, READY_PROBE_TIMEOUT).await {
                        if shutdown.load(Ordering::Acquire) {
                            return;
                        }
                        sleep(Duration::from_secs(5)).await;
                    }
                    failures = 0;
                    healthy_since = Some(Instant::now());
                    continue;
                }
                match daemon_command(&bin, &url, token_file.as_deref()).spawn() {
                    Ok(child) => {
                        record_daemon_pid(&url, child.id());
                        child_slot.lock().await.replace(child);
                        let deadline = Instant::now() + ready_wait(failures.saturating_sub(1));
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
        last.insert(url.to_owned(), std::time::Instant::now());
    }
    let slot = OWNED_DAEMONS
        .lock()
        .ok()
        .and_then(|owned| owned.get(url).and_then(std::sync::Weak::upgrade));
    // Nothing was killed on these paths: drop the stamp so `forced_restart_since` never tells a
    // concurrent turn its daemon died (and that resubmitting it is safe) when it did not.
    let unstamp = || {
        if let Ok(mut last) = LAST_FORCED_RESTART.lock() {
            last.remove(url);
        }
    };
    let Some(slot) = slot else {
        tracing::warn!(%url, "omo app-server is not owned by this gateway; cannot restart it");
        unstamp();
        return false;
    };
    {
        let mut guard = slot.lock().await;
        let Some(child) = guard.as_mut() else {
            unstamp();
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

/// True when the app-server at `url` answers readyz right now. A turn that produced no
/// events while the server stays responsive is a model-side stall, not a wedged daemon.
pub async fn owned_daemon_responsive(url: &str) -> bool {
    let owned = OWNED_DAEMONS
        .lock()
        .ok()
        .is_some_and(|owned| owned.get(url).and_then(std::sync::Weak::upgrade).is_some());
    owned && probe_readyz(url, READY_PROBE_TIMEOUT).await
}

/// Returns true if the daemon at `url` was force-restarted at or after `since`.
///
/// Incident 2026-10-16: concurrent turns in other sessions that fail with a transport
/// reset during a forced restart use this to determine that their connection died
/// because the app-server process was killed by the gateway, making a single retry safe.
pub fn forced_restart_since(url: &str, since: std::time::Instant) -> bool {
    let Ok(last) = LAST_FORCED_RESTART.lock() else {
        return false;
    };
    last.get(url).is_some_and(|&at| at >= since)
}

/// Polls `<url>/readyz` until the replacement daemon answers or `max` duration has elapsed.
///
/// Used after an owned daemon restart so concurrent turns lost to the restart can wait
/// for readiness before retrying (incident 2026-10-16).
pub async fn wait_owned_daemon_ready(url: &str, max: Duration) -> bool {
    let deadline = Instant::now() + max;
    while Instant::now() < deadline {
        if probe_readyz(url, READY_PROBE_TIMEOUT).await {
            return true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        sleep(Duration::from_millis(500).min(remaining)).await;
    }
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
            LIVENESS_INTERVAL,
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

    #[test]
    fn readiness_wait_grows_per_unready_restart_and_is_capped() {
        assert_eq!(ready_wait(0), READY_WAIT_AFTER_SPAWN);
        assert_eq!(ready_wait(1), READY_WAIT_AFTER_SPAWN * 2);
        assert_eq!(ready_wait(100), READY_WAIT_MAX);
    }

    #[test]
    fn forced_restart_since_reports_restarts_at_or_after_deadline() {
        let fake_url = "ws://127.0.0.1:59999/unit-test-forced-restart";
        let before = std::time::Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let restart_time = std::time::Instant::now();
        {
            let mut last = LAST_FORCED_RESTART.lock().unwrap();
            last.insert(fake_url.to_string(), restart_time);
        }
        std::thread::sleep(Duration::from_millis(5));
        let after = std::time::Instant::now();

        // `before` is strictly before the restart -> must be true
        assert!(forced_restart_since(fake_url, before));
        // `restart_time` is equal to the restart -> must be true (at >= since)
        assert!(forced_restart_since(fake_url, restart_time));
        // `after` is strictly after the restart -> must be false
        assert!(!forced_restart_since(fake_url, after));
        // Unrelated url -> must be false
        assert!(!forced_restart_since(
            "ws://127.0.0.1:59999/untracked-url",
            before
        ));
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

    #[test]
    fn patch_check_follows_the_runtime_wrapper_not_the_global_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let dist = dir.path().join("rt/dist");
        let threads = dist.join("modes/app-server/threads");
        std::fs::create_dir_all(&threads).unwrap();
        let wrapper = dir.path().join("omo-gateway-runtime");
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nexec bun {}/cli-main.js \"$@\" --extension x\n",
                dist.display()
            ),
        )
        .unwrap();
        let wrapper = wrapper.to_str().unwrap();

        // A plain `omo` binary or a wrapper whose runtime has no app-server: nothing to check.
        assert_eq!(app_server_patch_state("omo"), None);
        assert_eq!(app_server_patch_state(wrapper), None);

        let turns = threads.join("turns.js");
        std::fs::write(&turns, "this.subscribedThreadIds.has(t)").unwrap();
        assert_eq!(
            app_server_patch_state(wrapper),
            Some((turns.clone(), false))
        );
        std::fs::write(&turns, "e.session._turnEngineSubscribed||").unwrap();
        assert_eq!(app_server_patch_state(wrapper), Some((turns, true)));
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
        let mut fields = recorded.split_whitespace();
        assert_eq!(
            fields.next().and_then(|pid| pid.parse::<u32>().ok()),
            owned_pid
        );
        assert_eq!(
            fields.next().and_then(|owner| owner.parse::<u32>().ok()),
            Some(std::process::id())
        );
        supervisor.kill();
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn daemon_owned_by_a_live_process_is_not_reclaimed() {
        if isolated_home("daemon_owned_by_a_live_process_is_not_reclaimed").await {
            return;
        }
        let url = "ws://127.0.0.1:1";
        let mut owner = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let path = daemon_pid_path(url).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("{} {}", owner.id(), owner.id())).unwrap();
        assert_eq!(leftover_daemon_pid(url), None);
        let _ = owner.kill();
        let _ = owner.wait();
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
            LIVENESS_INTERVAL,
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
        tokio::time::advance(ready_wait(0) + Duration::from_secs(3)).await;
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

    /// Incident 2026-10-16: the app-server stayed alive but stopped answering for 4h38m
    /// (a blocking read of an iCloud dataless file froze its event loop). The supervisor
    /// must notice the silence, kill the wedged process, and start a fresh one.
    #[tokio::test]
    #[cfg(unix)]
    async fn daemon_alive_but_unresponsive_is_replaced() {
        if isolated_home("daemon_alive_but_unresponsive_is_replaced").await {
            return;
        }
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let events = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", port.local_addr().unwrap());
        drop(port);
        let bin = dir.path().join("wedging-daemon");
        // Answers readyz twice, then keeps accepting connections without replying,
        // like an app-server whose event loop is blocked.
        std::fs::write(
            &bin,
            format!(
                r#"#!/bin/sh
''''test -x /opt/homebrew/bin/python3 && exec /opt/homebrew/bin/python3 "$0" "$@"; exec /usr/bin/python3 "$0" "$@" # '''
import os, socket, sys
ev = socket.create_connection(('127.0.0.1', {}))
ev.sendall(str(os.getpid()).encode() + b'\n')
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', int(sys.argv[3].rsplit(':', 1)[1])))
s.listen(64)
held = []
answered = 0
while True:
    c, _ = s.accept()
    if answered < 2:
        c.recv(4096)
        c.sendall(b'HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n')
        c.close()
        answered += 1
    else:
        held.append(c)
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
        // Subscribe to the first daemon's start before starting the watcher.
        let started = events.accept();
        supervisor.spawn_watcher_with_backoff(
            url.clone(),
            bin.to_string_lossy().into_owned(),
            None,
            Duration::from_millis(10),
            Duration::from_millis(40),
            Duration::from_millis(100),
        );
        let (mut first, _) = timeout(Duration::from_secs(10), started)
            .await
            .unwrap()
            .unwrap();
        let mut first_pid = String::new();
        loop {
            let byte = first.read_u8().await.unwrap();
            if byte == b'\n' {
                break;
            }
            first_pid.push(byte as char);
        }
        // Subscribe to the replacement before the wedge is detected.
        let replaced = events.accept();
        // Detection needs LIVENESS_FAILURES probes that each wait READY_PROBE_TIMEOUT.
        let bound = READY_PROBE_TIMEOUT * (LIVENESS_FAILURES + 2) + Duration::from_secs(10);
        let eof = timeout(bound, first.read_u8()).await;
        let (mut second, _) = timeout(Duration::from_secs(15), replaced)
            .await
            .expect("supervisor must start a fresh daemon after the wedge")
            .unwrap();
        let mut second_pid = String::new();
        loop {
            let byte = second.read_u8().await.unwrap();
            if byte == b'\n' {
                break;
            }
            second_pid.push(byte as char);
        }
        supervisor.kill();
        assert!(
            eof.is_ok(),
            "wedged daemon PID {first_pid} was never killed"
        );
        assert!(
            !process_alive(first_pid.parse().unwrap()),
            "wedged daemon must be gone"
        );
        assert_ne!(first_pid, second_pid, "replacement must be a fresh process");
        assert!(
            forced_restart_since(&url, std::time::Instant::now() - bound * 2),
            "the liveness kill must be recorded like a forced restart"
        );
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

    #[tokio::test]
    async fn test_daemon_command_path_prepends_dirs() {
        if isolated_home("test_daemon_command_path_prepends_dirs").await {
            return;
        }
        let cmd = daemon_command("omo", "ws://127.0.0.1:19742", None);
        let envs: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> = cmd
            .as_std()
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|v| v.to_os_string())))
            .collect();
        let path_val = envs
            .iter()
            .find(|(k, _)| k == "PATH")
            .and_then(|(_, v)| v.as_ref())
            .expect("PATH env var must be set on daemon command");

        let entries: Vec<std::path::PathBuf> = std::env::split_paths(path_val).collect();
        assert!(
            entries.len() >= 2,
            "PATH must have at least the two prepended dirs"
        );

        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .expect("HOME or USERPROFILE must be present in test");
        let expected_omon_bin = std::path::PathBuf::from(home).join(".omon").join("bin");
        let expected_exe_dir = std::env::current_exe()
            .expect("current_exe must succeed in test")
            .parent()
            .expect("current_exe must have a parent dir")
            .to_path_buf();

        assert_eq!(entries[0], expected_omon_bin);
        assert_eq!(entries[1], expected_exe_dir);
    }
}

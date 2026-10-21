use crate::{OmonError, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchctlOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait MigrationLock: Send {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOperation {
    LockAcquired(PathBuf),
    LockReleased(PathBuf),
    PidAlive(i32),
    ProcessStartTime(i32),
    ProcessCommandLine(i32),
    Bootout(Vec<String>),
    Terminate(i32),
    Kill(i32),
    Sleep(Duration),
    RemoveFile(PathBuf),
    Write(PathBuf),
    Rename(PathBuf, PathBuf),
}

pub trait MigrationEnv: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn read(&self, path: &Path) -> Result<Vec<u8>>;
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()>;
    /// Create a private file exclusively, choosing a new suffix on collision.
    fn write_unique(&self, path: &Path, bytes: &[u8]) -> Result<PathBuf> {
        let _ = bytes;
        Err(OmonError::Config(format!(
            "exclusive migration writes unsupported for {}",
            path.display()
        )))
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let name = path.file_name().ok_or_else(|| {
            OmonError::Config(format!("invalid migration target: {}", path.display()))
        })?;
        let temporary = path.with_file_name(format!(
            ".{}.tmp-omon-migration-{}",
            name.to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        let temporary = self.write_unique(&temporary, bytes)?;
        if let Err(error) = self.rename(&temporary, path) {
            self.remove_file(&temporary).map_err(|cleanup| {
                OmonError::Config(format!("{error}; temporary cleanup failed: {cleanup}"))
            })?;
            return Err(error);
        }
        Ok(())
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<()>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let mut normalized = PathBuf::new();
        for comp in path.components() {
            match comp {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                c => normalized.push(c),
            }
        }
        Ok(normalized)
    }
    fn acquire_jobs_lock(&self, path: &Path) -> Result<Box<dyn MigrationLock>>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;

    fn current_uid(&self) -> u32;
    fn now(&self) -> DateTime<Utc>;

    fn pid_alive(&self, pid: i32) -> bool;
    /// Unknown identity never authorizes a signal.
    fn process_start_time(&self, _pid: i32) -> Result<Option<u64>> {
        Ok(None)
    }
    /// Live command-line probe; unknown identity never authorizes a signal.
    fn process_command_line(&self, _pid: i32) -> Result<Option<String>> {
        Ok(None)
    }
    fn terminate(&self, pid: i32) -> Result<()>;
    fn kill(&self, pid: i32) -> Result<()>;
    fn sleep(&self, duration: Duration);

    fn run_launchctl(&self, args: &[&str]) -> Result<LaunchctlOutput>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum WaitNotification {
    Exited,
    TimedOut,
}

#[cfg(target_os = "macos")]
struct KqueueFd(libc::c_int);

#[cfg(target_os = "macos")]
impl KqueueFd {
    fn new() -> Result<Self> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            Err(OmonError::Config(format!(
                "kqueue creation failed: {}",
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(Self(fd))
        }
    }

    fn raw(&self) -> libc::c_int {
        self.0
    }
}

#[cfg(target_os = "macos")]
impl Drop for KqueueFd {
    fn drop(&mut self) {
        if self.0 >= 0 {
            unsafe {
                libc::close(self.0);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn default_wait_exit_notification(child_pid: i32, timeout: Duration) -> Result<WaitNotification> {
    let kq = KqueueFd::new()?;

    let ke = libc::kevent {
        ident: child_pid as usize,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };

    let reg = unsafe { libc::kevent(kq.raw(), &ke, 1, std::ptr::null_mut(), 0, std::ptr::null()) };

    if reg < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(WaitNotification::Exited);
        }
        return Err(OmonError::Config(format!(
            "failed to register kqueue for pid {child_pid}: {err}"
        )));
    }

    let ts = libc::timespec {
        tv_sec: timeout.as_secs() as _,
        tv_nsec: timeout.subsec_nanos() as _,
    };
    let mut out_event = std::mem::MaybeUninit::<libc::kevent>::uninit();
    let n = unsafe {
        libc::kevent(
            kq.raw(),
            std::ptr::null(),
            0,
            out_event.as_mut_ptr(),
            1,
            &ts,
        )
    };
    if n < 0 {
        return Err(OmonError::Config(format!(
            "kevent wait failed for pid {child_pid}: {}",
            std::io::Error::last_os_error()
        )));
    }
    if n == 0 {
        return Ok(WaitNotification::TimedOut);
    }

    // SAFETY: n == 1, kevent populated out_event
    let ev = unsafe { out_event.assume_init() };
    if ev.flags & libc::EV_ERROR != 0 {
        return Err(OmonError::Config(format!(
            "kevent event error for pid {child_pid}: {}",
            std::io::Error::from_raw_os_error(ev.data as i32)
        )));
    }
    let ev_ident = ev.ident;
    let ev_filter = ev.filter;
    if ev_ident != child_pid as usize || ev_filter != libc::EVFILT_PROC {
        return Err(OmonError::Config(format!(
            "unexpected kevent for pid {child_pid}: ident {ev_ident} filter {ev_filter}"
        )));
    }
    Ok(WaitNotification::Exited)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_wait_exit_notification(child_pid: i32, timeout: Duration) -> Result<WaitNotification> {
    let start = std::time::Instant::now();
    loop {
        // kill(pid, 0) still succeeds for a zombie. The caller reaps with wait().
        if unsafe { libc::kill(child_pid, 0) != 0 } || process_is_zombie(child_pid) {
            return Ok(WaitNotification::Exited);
        }
        if start.elapsed() >= timeout {
            return Ok(WaitNotification::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Linux `/proc` state `Z` means the child has exited and is waiting to be reaped.
#[cfg(all(unix, not(target_os = "macos")))]
fn process_is_zombie(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().next().map(|state| state == "Z"))
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn default_wait_exit_notification(_child_pid: i32, _timeout: Duration) -> Result<WaitNotification> {
    Ok(WaitNotification::Exited)
}

fn run_command_with_timeout(cmd: Command, timeout: Duration) -> Result<std::process::Output> {
    run_command_with_timeout_impl(cmd, timeout, default_wait_exit_notification)
}

#[cfg(test)]
fn run_command_with_timeout_injected(
    cmd: Command,
    timeout: Duration,
    notifier: impl FnOnce(i32, Duration) -> Result<WaitNotification>,
) -> Result<std::process::Output> {
    run_command_with_timeout_impl(cmd, timeout, notifier)
}

fn run_command_with_timeout_impl(
    mut cmd: Command,
    timeout: Duration,
    notifier: impl FnOnce(i32, Duration) -> Result<WaitNotification>,
) -> Result<std::process::Output> {
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|error| OmonError::Config(format!("failed to spawn command: {error}")))?;

    let child_pid = child.id() as i32;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        if let Some(mut s) = stdout {
            std::io::Read::read_to_end(&mut s, &mut out)?;
        }
        Ok(out)
    });

    let stderr_reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut err = Vec::new();
        if let Some(mut s) = stderr {
            std::io::Read::read_to_end(&mut s, &mut err)?;
        }
        Ok(err)
    });

    let wait_notification = notifier(child_pid, timeout);

    let exit_status = match wait_notification {
        Ok(WaitNotification::Exited) => child.wait().map_err(|e| {
            OmonError::Config(format!("failed to wait on exited pid {child_pid}: {e}"))
        }),
        Ok(WaitNotification::TimedOut) => {
            let kill_res = child.kill();
            let wait_res = child.wait();
            let mut errors = vec![format!(
                "command pid {child_pid} timed out after {timeout:?}"
            )];
            if let Err(e) = kill_res {
                if e.kind() != std::io::ErrorKind::NotFound
                    && e.kind() != std::io::ErrorKind::InvalidInput
                {
                    errors.push(format!(
                        "failed to kill timed out command pid {child_pid}: {e}"
                    ));
                }
            }
            if let Err(e) = wait_res {
                errors.push(format!(
                    "failed to reap killed command pid {child_pid}: {e}"
                ));
            }
            Err(OmonError::Config(errors.join("; ")))
        }
        Err(notifier_err) => {
            let kill_res = child.kill();
            let wait_res = child.wait();
            let mut errors = vec![notifier_err.to_string()];
            if let Err(e) = kill_res {
                if e.kind() != std::io::ErrorKind::NotFound
                    && e.kind() != std::io::ErrorKind::InvalidInput
                {
                    errors.push(format!("failed to kill command pid {child_pid}: {e}"));
                }
            }
            if let Err(e) = wait_res {
                errors.push(format!("failed to reap command pid {child_pid}: {e}"));
            }
            Err(OmonError::Config(errors.join("; ")))
        }
    };

    let stdout_join = stdout_reader.join();
    let stderr_join = stderr_reader.join();

    let stdout_res = stdout_join
        .map_err(|_| OmonError::Config("stdout reader thread panicked".into()))
        .and_then(|r| r.map_err(|e| OmonError::Config(format!("failed to read stdout: {e}"))));
    let stderr_res = stderr_join
        .map_err(|_| OmonError::Config("stderr reader thread panicked".into()))
        .and_then(|r| r.map_err(|e| OmonError::Config(format!("failed to read stderr: {e}"))));

    let status = match exit_status {
        Ok(status) => status,
        Err(err) => {
            let mut msgs = vec![err.to_string()];
            if let Err(e) = stdout_res {
                msgs.push(e.to_string());
            }
            if let Err(e) = stderr_res {
                msgs.push(e.to_string());
            }
            return Err(OmonError::Config(msgs.join("; ")));
        }
    };

    match (stdout_res, stderr_res) {
        (Ok(stdout), Ok(stderr)) => Ok(std::process::Output {
            status,
            stdout,
            stderr,
        }),
        (Err(e1), Err(e2)) => Err(OmonError::Config(format!("{e1}; {e2}"))),
        (Err(e), Ok(_)) | (Ok(_), Err(e)) => Err(e),
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OsEnv;

struct OsMigrationLock(File);

impl MigrationLock for OsMigrationLock {}

impl Drop for OsMigrationLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

impl MigrationEnv for OsEnv {
    fn process_start_time(&self, pid: i32) -> Result<Option<u64>> {
        #[cfg(target_os = "macos")]
        {
            let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
            let size = i32::try_from(std::mem::size_of::<libc::proc_bsdinfo>())
                .map_err(|error| OmonError::Config(error.to_string()))?;
            // SAFETY: the kernel receives an aligned writable buffer of exactly size bytes.
            let read = unsafe {
                libc::proc_pidinfo(
                    pid,
                    libc::PROC_PIDTBSDINFO,
                    0,
                    info.as_mut_ptr().cast(),
                    size,
                )
            };
            if read != size {
                return Err(OmonError::Config(format!(
                    "cannot identify pid {pid}: {}",
                    std::io::Error::last_os_error()
                )));
            }
            // SAFETY: proc_pidinfo returned the full initialized proc_bsdinfo structure.
            let info = unsafe { info.assume_init() };
            // Hermes uses round(psutil.create_time() * 100), including ties-to-even.
            let epoch = info.pbi_start_tvsec as f64 + info.pbi_start_tvusec as f64 / 1_000_000.0;
            Ok(Some((epoch * 100.0).round_ties_even() as u64))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = pid;
            Err(OmonError::Config(
                "verified process retirement requires macOS".into(),
            ))
        }
    }

    fn process_command_line(&self, pid: i32) -> Result<Option<String>> {
        #[cfg(unix)]
        {
            let candidates = ["/bin/ps", "/usr/bin/ps"];
            let ps_bin = candidates
                .iter()
                .find(|p| Path::new(p).is_file())
                .copied()
                .ok_or_else(|| {
                    OmonError::Config(
                        "trusted ps binary not found at /bin/ps or /usr/bin/ps".into(),
                    )
                })?;
            let mut cmd = Command::new(ps_bin);
            cmd.args(["-p", &pid.to_string(), "-o", "command="]);
            match run_command_with_timeout(cmd, Duration::from_secs(5)) {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if text.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(text))
                    }
                }
                Ok(_) => Ok(None),
                Err(error) => Err(error),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            Ok(None)
        }
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(|error| fs_error("read", path, error))
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).map_err(|error| fs_error("read", path, error))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        self.write_atomic(path, bytes)
    }

    fn write_unique(&self, path: &Path, bytes: &[u8]) -> Result<PathBuf> {
        let mut candidate = path.to_path_buf();
        loop {
            let mut opts = File::options();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            opts.mode(0o600);
            let opened = opts.open(&candidate);
            match opened {
                Ok(mut file) => {
                    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                        self.remove_file(&candidate).map_err(|cleanup| {
                            OmonError::Config(format!(
                                "{error}; partial file cleanup failed: {cleanup}"
                            ))
                        })?;
                        return Err(fs_error("write", &candidate, error));
                    }
                    return Ok(candidate);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let mut name = path.as_os_str().to_os_string();
                    name.push(format!("-{}", uuid::Uuid::new_v4()));
                    candidate = PathBuf::from(name);
                }
                Err(error) => return Err(fs_error("create", &candidate, error)),
            }
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        std::fs::rename(from, to).map_err(|error| {
            OmonError::Config(format!(
                "failed to rename {} to {}: {error}",
                from.display(),
                to.display()
            ))
        })
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(fs_error("remove", path, error)),
        }
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        if path.exists() {
            std::fs::canonicalize(path).map_err(|error| fs_error("canonicalize", path, error))
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                let canonical_parent = std::fs::canonicalize(parent)
                    .map_err(|error| fs_error("canonicalize parent of", path, error))?;
                if let Some(file_name) = path.file_name() {
                    Ok(canonical_parent.join(file_name))
                } else {
                    Ok(canonical_parent)
                }
            } else {
                Ok(path.to_path_buf())
            }
        } else {
            Ok(path.to_path_buf())
        }
    }

    fn acquire_jobs_lock(&self, path: &Path) -> Result<Box<dyn MigrationLock>> {
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|error| fs_error("open lock file", path, error))?;
        file.lock_exclusive()
            .map_err(|error| fs_error("lock", path, error))?;
        Ok(Box::new(OsMigrationLock(file)))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries =
            std::fs::read_dir(path).map_err(|error| fs_error("read directory", path, error))?;
        entries
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| fs_error("read directory entry in", path, error))
            })
            .collect()
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path).map_err(|error| fs_error("create directory", path, error))
    }

    fn current_uid(&self) -> u32 {
        #[cfg(unix)]
        {
            // SAFETY: getuid has no preconditions and does not dereference pointers.
            unsafe { libc::getuid() }
        }
        #[cfg(not(unix))]
        {
            0
        }
    }

    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn pid_alive(&self, pid: i32) -> bool {
        #[cfg(unix)]
        {
            // SAFETY: signal 0 performs an existence/permission check and sends no signal.
            let result = unsafe { libc::kill(pid, 0) };
            result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
        #[cfg(not(unix))]
        {
            crate::ledger::is_process_alive(pid as u32)
        }
    }

    fn terminate(&self, pid: i32) -> Result<()> {
        #[cfg(unix)]
        {
            send_signal(pid, libc::SIGTERM, "SIGTERM")
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            Ok(())
        }
    }

    fn kill(&self, pid: i32) -> Result<()> {
        #[cfg(unix)]
        {
            send_signal(pid, libc::SIGKILL, "SIGKILL")
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            Ok(())
        }
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn run_launchctl(&self, args: &[&str]) -> Result<LaunchctlOutput> {
        let output = Command::new("launchctl")
            .args(args)
            .output()
            .map_err(|error| OmonError::Config(format!("failed to run launchctl: {error}")))?;
        Ok(LaunchctlOutput {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn fs_error(operation: &str, path: &Path, error: std::io::Error) -> OmonError {
    OmonError::Config(format!("failed to {operation} {}: {error}", path.display()))
}

#[cfg(unix)]
fn send_signal(pid: i32, signal: i32, name: &str) -> Result<()> {
    // SAFETY: libc::kill accepts a process id and signal number by value.
    if unsafe { libc::kill(pid, signal) } == 0 {
        Ok(())
    } else {
        Err(OmonError::Config(format!(
            "failed to send {name} to pid {pid}: {}",
            std::io::Error::last_os_error()
        )))
    }
}

/// Public test support for migration modules and integration tests.
///
/// This fake is compiled in all builds so `tests/` can inject it without a feature flag. It never
/// calls [`OsEnv`], the operating-system filesystem, process signals, or `launchctl`.
pub struct FakeMigrationEnv {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    directories: Mutex<HashSet<PathBuf>>,
    read_only_paths: Mutex<HashSet<PathBuf>>,
    now: Mutex<DateTime<Utc>>,
    current_uid: Mutex<u32>,
    alive_pids: Mutex<HashSet<i32>>,
    process_starts: Mutex<HashMap<i32, u64>>,
    process_cmdlines: Mutex<HashMap<i32, String>>,
    pid_death_after_sleeps: Mutex<HashMap<i32, usize>>,
    operations: Arc<Mutex<Vec<MigrationOperation>>>,
    terminate_calls: Mutex<Vec<i32>>,
    kill_calls: Mutex<Vec<i32>>,
    rename_calls: Mutex<Vec<(PathBuf, PathBuf)>>,
    write_calls: Mutex<Vec<(PathBuf, Vec<u8>)>>,
    launchctl_calls: Mutex<Vec<Vec<String>>>,
    launchctl_response: Mutex<LaunchctlOutput>,
    launchctl_error: Mutex<Option<String>>,
    aliases: Mutex<HashMap<PathBuf, PathBuf>>,
}

impl FakeMigrationEnv {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
            directories: Mutex::new(HashSet::new()),
            read_only_paths: Mutex::new(HashSet::new()),
            now: Mutex::new(now),
            current_uid: Mutex::new(0),
            alive_pids: Mutex::new(HashSet::new()),
            process_starts: Mutex::new(HashMap::new()),
            process_cmdlines: Mutex::new(HashMap::new()),
            pid_death_after_sleeps: Mutex::new(HashMap::new()),
            operations: Arc::new(Mutex::new(Vec::new())),
            terminate_calls: Mutex::new(Vec::new()),
            kill_calls: Mutex::new(Vec::new()),
            rename_calls: Mutex::new(Vec::new()),
            write_calls: Mutex::new(Vec::new()),
            launchctl_calls: Mutex::new(Vec::new()),
            launchctl_response: Mutex::new(LaunchctlOutput {
                status: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }),
            launchctl_error: Mutex::new(None),
            aliases: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_alias(&self, from: impl Into<PathBuf>, to: impl Into<PathBuf>) {
        self.aliases.lock().insert(from.into(), to.into());
    }

    pub fn set_now(&self, now: DateTime<Utc>) {
        *self.now.lock() = now;
    }

    pub fn set_current_uid(&self, uid: u32) {
        *self.current_uid.lock() = uid;
    }

    pub fn set_pid_alive(&self, pid: i32, alive: bool) {
        if alive {
            self.alive_pids.lock().insert(pid);
        } else {
            self.alive_pids.lock().remove(&pid);
        }
    }

    pub fn set_pid_death_after_sleeps(&self, pid: i32, sleeps: usize) {
        // Virtual-clock fixture: no wall-clock waiting.
        self.pid_death_after_sleeps.lock().insert(pid, sleeps);
    }

    pub fn set_process_start_time(&self, pid: i32, start_time: u64) {
        self.process_starts.lock().insert(pid, start_time);
    }

    pub fn set_process_command_line(&self, pid: i32, cmdline: impl Into<String>) {
        self.process_cmdlines.lock().insert(pid, cmdline.into());
    }

    pub fn operations(&self) -> Vec<MigrationOperation> {
        self.operations.lock().clone()
    }

    pub fn set_read_only(&self, path: impl Into<PathBuf>, read_only: bool) {
        let path = path.into();
        if read_only {
            self.read_only_paths.lock().insert(path);
        } else {
            self.read_only_paths.lock().remove(&path);
        }
    }

    pub fn set_launchctl_response(&self, response: LaunchctlOutput) {
        *self.launchctl_response.lock() = response;
        *self.launchctl_error.lock() = None;
    }

    pub fn set_launchctl_error(&self, message: impl Into<String>) {
        *self.launchctl_error.lock() = Some(message.into());
    }

    pub fn terminate_calls(&self) -> Vec<i32> {
        self.terminate_calls.lock().clone()
    }

    pub fn kill_calls(&self) -> Vec<i32> {
        self.kill_calls.lock().clone()
    }

    pub fn rename_calls(&self) -> Vec<(PathBuf, PathBuf)> {
        self.rename_calls.lock().clone()
    }

    pub fn write_calls(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.write_calls.lock().clone()
    }

    pub fn launchctl_calls(&self) -> Vec<Vec<String>> {
        self.launchctl_calls.lock().clone()
    }

    fn ensure_writable(&self, path: &Path) -> Result<()> {
        if self.read_only_paths.lock().contains(path) {
            Err(OmonError::Config(format!(
                "fake path is read-only: {}",
                path.display()
            )))
        } else {
            Ok(())
        }
    }

    fn record_parent_directories(&self, path: &Path) {
        let mut directories = self.directories.lock();
        let mut parent = path.parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
}

struct FakeMigrationLock {
    path: PathBuf,
    operations: Arc<Mutex<Vec<MigrationOperation>>>,
}

impl MigrationLock for FakeMigrationLock {}

impl Drop for FakeMigrationLock {
    fn drop(&mut self) {
        self.operations
            .lock()
            .push(MigrationOperation::LockReleased(self.path.clone()));
    }
}

impl MigrationEnv for FakeMigrationEnv {
    fn process_start_time(&self, pid: i32) -> Result<Option<u64>> {
        self.operations
            .lock()
            .push(MigrationOperation::ProcessStartTime(pid));
        Ok(self.process_starts.lock().get(&pid).copied())
    }

    fn process_command_line(&self, pid: i32) -> Result<Option<String>> {
        self.operations
            .lock()
            .push(MigrationOperation::ProcessCommandLine(pid));
        Ok(self.process_cmdlines.lock().get(&pid).cloned())
    }

    fn write_unique(&self, path: &Path, bytes: &[u8]) -> Result<PathBuf> {
        self.ensure_writable(path)?;
        let mut candidate = path.to_path_buf();
        let mut files = self.files.lock();
        while files.contains_key(&candidate) {
            let mut name = path.as_os_str().to_os_string();
            name.push(format!("-{}", uuid::Uuid::new_v4()));
            candidate = PathBuf::from(name);
        }
        files.insert(candidate.clone(), bytes.to_vec());
        drop(files);
        self.record_parent_directories(&candidate);
        self.write_calls
            .lock()
            .push((candidate.clone(), bytes.to_vec()));
        self.operations
            .lock()
            .push(MigrationOperation::Write(candidate.clone()));
        Ok(candidate)
    }

    fn read_to_string(&self, path: &Path) -> Result<String> {
        String::from_utf8(self.read(path)?).map_err(|error| {
            OmonError::Config(format!(
                "fake file {} is not UTF-8: {error}",
                path.display()
            ))
        })
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        self.files.lock().get(path).cloned().ok_or_else(|| {
            OmonError::Config(format!("fake file does not exist: {}", path.display()))
        })
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        self.ensure_writable(path)?;
        self.record_parent_directories(path);
        self.files.lock().insert(path.to_path_buf(), bytes.to_vec());
        self.write_calls
            .lock()
            .push((path.to_path_buf(), bytes.to_vec()));
        self.operations
            .lock()
            .push(MigrationOperation::Write(path.to_path_buf()));
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.ensure_writable(from)?;
        self.ensure_writable(to)?;
        let bytes = self.files.lock().remove(from).ok_or_else(|| {
            OmonError::Config(format!("fake file does not exist: {}", from.display()))
        })?;
        self.record_parent_directories(to);
        self.files.lock().insert(to.to_path_buf(), bytes);
        self.rename_calls
            .lock()
            .push((from.to_path_buf(), to.to_path_buf()));
        self.operations.lock().push(MigrationOperation::Rename(
            from.to_path_buf(),
            to.to_path_buf(),
        ));
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        self.ensure_writable(path)?;
        self.files.lock().remove(path);
        self.operations
            .lock()
            .push(MigrationOperation::RemoveFile(path.to_path_buf()));
        Ok(())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let mut normalized = PathBuf::new();
        for comp in path.components() {
            match comp {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                c => normalized.push(c),
            }
        }
        if let Some(target) = self.aliases.lock().get(&normalized) {
            Ok(target.clone())
        } else {
            Ok(normalized)
        }
    }

    fn acquire_jobs_lock(&self, path: &Path) -> Result<Box<dyn MigrationLock>> {
        self.operations
            .lock()
            .push(MigrationOperation::LockAcquired(path.to_path_buf()));
        Ok(Box::new(FakeMigrationLock {
            path: path.to_path_buf(),
            operations: Arc::clone(&self.operations),
        }))
    }

    fn exists(&self, path: &Path) -> bool {
        self.is_file(path) || self.is_dir(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.lock().contains_key(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.directories.lock().contains(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        if !self.is_dir(path) {
            return Err(OmonError::Config(format!(
                "fake directory does not exist: {}",
                path.display()
            )));
        }
        let files = self.files.lock();
        let directories = self.directories.lock();
        let mut entries: HashSet<PathBuf> = files
            .keys()
            .chain(directories.iter())
            .filter(|entry| entry.parent() == Some(path) && entry.as_path() != path)
            .cloned()
            .collect();
        let mut entries: Vec<_> = entries.drain().collect();
        entries.sort();
        Ok(entries)
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.ensure_writable(path)?;
        let mut directories = self.directories.lock();
        for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
            directories.insert(ancestor.to_path_buf());
        }
        Ok(())
    }

    fn current_uid(&self) -> u32 {
        *self.current_uid.lock()
    }

    fn now(&self) -> DateTime<Utc> {
        *self.now.lock()
    }

    fn pid_alive(&self, pid: i32) -> bool {
        self.operations
            .lock()
            .push(MigrationOperation::PidAlive(pid));
        self.alive_pids.lock().contains(&pid)
    }

    fn terminate(&self, pid: i32) -> Result<()> {
        self.terminate_calls.lock().push(pid);
        self.operations
            .lock()
            .push(MigrationOperation::Terminate(pid));
        Ok(())
    }

    fn kill(&self, pid: i32) -> Result<()> {
        self.kill_calls.lock().push(pid);
        self.operations.lock().push(MigrationOperation::Kill(pid));
        self.alive_pids.lock().remove(&pid);
        Ok(())
    }

    fn sleep(&self, duration: Duration) {
        self.operations
            .lock()
            .push(MigrationOperation::Sleep(duration));
        let mut schedules = self.pid_death_after_sleeps.lock();
        let mut dead = Vec::new();
        for (&pid, remaining) in schedules.iter_mut() {
            if *remaining <= 1 {
                dead.push(pid);
            } else {
                *remaining -= 1;
            }
        }
        for pid in dead {
            schedules.remove(&pid);
            self.alive_pids.lock().remove(&pid);
        }
    }

    fn run_launchctl(&self, args: &[&str]) -> Result<LaunchctlOutput> {
        self.launchctl_calls
            .lock()
            .push(args.iter().map(|arg| (*arg).to_string()).collect());
        self.operations.lock().push(MigrationOperation::Bootout(
            args.iter().map(|arg| (*arg).to_owned()).collect(),
        ));
        if let Some(message) = self.launchctl_error.lock().clone() {
            Err(OmonError::Config(message))
        } else {
            Ok(self.launchctl_response.lock().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        run_command_with_timeout, run_command_with_timeout_injected, FakeMigrationEnv,
        LaunchctlOutput, MigrationEnv, OsEnv,
    };
    use crate::OmonError;
    use chrono::{TimeZone, Utc};
    use std::any::{type_name, type_name_of_val};
    use std::path::Path;
    use std::process::Command;
    use std::time::Duration;

    #[test]
    fn fake_records_process_and_launchctl_calls_without_os_env() {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 12, 30, 0).unwrap();
        let env = FakeMigrationEnv::new(now);
        env.set_pid_alive(4242, true);
        env.set_launchctl_response(LaunchctlOutput {
            status: Some(0),
            stdout: "booted out".into(),
            stderr: String::new(),
        });
        assert_eq!(type_name_of_val(&env), type_name::<FakeMigrationEnv>());
        assert_ne!(type_name_of_val(&env), type_name::<OsEnv>());

        let output = env
            .run_launchctl(&["bootout", "gui/501/ai.hermes.gateway"])
            .unwrap();
        env.terminate(4242).unwrap();
        env.kill(4242).unwrap();

        assert_eq!(output.status, Some(0));
        assert_eq!(
            env.launchctl_calls(),
            vec![vec![
                "bootout".to_string(),
                "gui/501/ai.hermes.gateway".to_string()
            ]]
        );
        assert_eq!(env.terminate_calls(), vec![4242]);
        assert_eq!(env.kill_calls(), vec![4242]);
        assert!(!env.pid_alive(4242));
        println!("recorded launchctl={:?}", env.launchctl_calls());
        println!(
            "recorded terminate={:?} kill={:?}",
            env.terminate_calls(),
            env.kill_calls()
        );
    }

    #[test]
    fn fake_clock_is_injectable() {
        let first = Utc.with_ymd_and_hms(2026, 8, 15, 1, 2, 3).unwrap();
        let second = Utc.with_ymd_and_hms(2027, 1, 2, 3, 4, 5).unwrap();
        let env = FakeMigrationEnv::new(first);

        assert_eq!(env.now(), first);
        env.set_now(second);
        assert_eq!(env.now(), second);
        println!("injectable clock={}", env.now().to_rfc3339());
    }

    #[test]
    fn fake_filesystem_write_read_rename_and_exists_are_coherent() {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 0).unwrap();
        let env = FakeMigrationEnv::new(now);
        let source = Path::new("/migration/source.env");
        let destination = Path::new("/migration/destination.env");

        env.create_dir_all(Path::new("/migration")).unwrap();
        env.write(source, b"TOKEN=secret\n").unwrap();
        assert_eq!(env.read(source).unwrap(), b"TOKEN=secret\n");
        assert_eq!(env.read_to_string(source).unwrap(), "TOKEN=secret\n");
        assert!(env.exists(source));
        assert!(env.is_file(source));
        assert!(env.is_dir(Path::new("/migration")));

        env.rename(source, destination).unwrap();

        assert!(!env.exists(source));
        assert!(env.exists(destination));
        assert_eq!(env.read(destination).unwrap(), b"TOKEN=secret\n");
        assert_eq!(
            env.rename_calls(),
            vec![(source.into(), destination.into())]
        );
        assert_eq!(
            env.write_calls(),
            vec![(source.into(), b"TOKEN=secret\n".to_vec())]
        );
        assert_eq!(
            env.read_dir(Path::new("/migration")).unwrap(),
            vec![destination.to_path_buf()]
        );
        println!(
            "in-memory fs files={:?}",
            env.read_dir(Path::new("/migration")).unwrap()
        );
    }

    #[test]
    fn fake_read_only_path_rejects_writes() {
        let env = FakeMigrationEnv::new(Utc::now());
        let path = Path::new("/migration/read-only.env");
        env.set_read_only(path, true);

        let error = env.write(path, b"data").unwrap_err();

        assert!(error.to_string().contains("read-only"));
        assert!(env.write_calls().is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn subprocess_timeout_kills_and_reaps_owned_child() {
        let temp = tempfile::tempdir().unwrap();
        let signal_file = temp.path().join("started.txt");
        let signal_path = signal_file.display().to_string();
        let mut cmd = Command::new("/bin/sh");
        cmd.args([
            "-c",
            &format!("echo started > '{signal_path}' && exec sleep 5"),
        ]);
        let start = std::time::Instant::now();
        let result = run_command_with_timeout(cmd, Duration::from_millis(200));
        let elapsed = start.elapsed();
        assert!(result.is_err(), "expected timeout error, got {result:?}");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timed out"),
            "expected timeout message, got {err_msg}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "elapsed {elapsed:?} exceeded bounded timeout"
        );
        assert!(
            signal_file.exists(),
            "child should have signaled start before holding"
        );

        let pid_str = err_msg
            .split("command pid ")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .unwrap();
        let child_pid: i32 = pid_str.parse().unwrap();
        #[cfg(unix)]
        {
            let ret = unsafe { libc::kill(child_pid, 0) };
            assert_ne!(ret, 0, "child pid {child_pid} should have been reaped");
        }
    }

    #[test]
    #[cfg(unix)]
    fn subprocess_injected_notifier_failure_kills_and_reaps_child_without_hang() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "exec cat"]);
        cmd.stdin(std::process::Stdio::piped());

        let (pid_tx, pid_rx) = std::sync::mpsc::channel();
        let (res_tx, res_rx) = std::sync::mpsc::channel();
        let start = std::time::Instant::now();

        let test_thread = std::thread::spawn(move || {
            let res =
                run_command_with_timeout_injected(cmd, Duration::from_secs(5), |pid, _timeout| {
                    let _ = pid_tx.send(pid);
                    Err(OmonError::Config(format!(
                        "injected notifier failure for pid {pid}"
                    )))
                });
            let _ = res_tx.send(res);
        });

        let child_pid = pid_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("notifier should observe child pid immediately after spawn");

        let result = res_rx.recv_timeout(Duration::from_secs(2)).expect(
            "run_command_with_timeout_injected should complete without hanging on reader join",
        );
        let elapsed = start.elapsed();

        test_thread
            .join()
            .expect("test runner thread should join cleanly");

        assert!(
            result.is_err(),
            "expected error from injected notifier failure"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "elapsed {elapsed:?} should be bounded and not hang"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(&format!("injected notifier failure for pid {child_pid}")),
            "error should contain notifier message: {err_msg}"
        );
        #[cfg(unix)]
        {
            let ret = unsafe { libc::kill(child_pid, 0) };
            assert_ne!(
                ret, 0,
                "child pid {child_pid} should have been reaped, not leaked"
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn subprocess_simultaneous_large_stdout_stderr_does_not_deadlock() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "printf '%131072s' E >&2; printf '%131072s' O"]);
        let output = run_command_with_timeout(cmd, Duration::from_secs(2))
            .expect("should not deadlock on simultaneous large stderr/stdout");
        assert_eq!(output.stdout.len(), 131072);
        assert_eq!(output.stderr.len(), 131072);
        assert_eq!(output.stdout.last(), Some(&b'O'));
        assert_eq!(output.stderr.last(), Some(&b'E'));
        assert!(output.status.success());
    }

    #[test]
    #[cfg(unix)]
    fn os_env_reads_self_command_line() {
        let env = OsEnv;
        let self_pid = std::process::id() as i32;
        let cmd = env.process_command_line(self_pid).unwrap();
        assert!(cmd.is_some(), "expected to read self command line");
        let cmd = cmd.unwrap();
        assert!(!cmd.is_empty());
        assert!(
            !crate::migrate::gateway_down::looks_like_gateway_runtime_command_line(&cmd),
            "test runner should not look like gateway runtime: {cmd}"
        );
    }
}

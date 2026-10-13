use crate::migrate::sys::{LaunchctlOutput, MigrationEnv};
use crate::{OmonError, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

// 10s SIGTERM grace so an in-flight shutdown/cron run is not SIGKILLed mid-write.
const TERM_WAIT_POLLS: usize = 100;
const TERM_WAIT_INTERVAL: Duration = Duration::from_millis(100);

const PLIST_PREFIX: &str = "ai.hermes.gateway";
const PLIST_SUFFIX: &str = ".plist";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayDownSummary {
    pub pids_found: Vec<i32>,
    pub pids_terminated: Vec<i32>,
    pub pids_killed: Vec<i32>,
    pub plists_booted_out: Vec<String>,
    pub plists_disabled: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct GatewayLock {
    pid: i32,
    start_time: Option<u64>,
}

type DiscoveredLock = (PathBuf, Option<u64>);
type DiscoveredPidLocks = BTreeMap<i32, Vec<DiscoveredLock>>;

pub fn bring_gateway_down(
    env: &dyn MigrationEnv,
    hermes_root: &Path,
    launch_agents_dir: &Path,
    dry_run: bool,
) -> Result<GatewayDownSummary> {
    let pid_locks = discover_pid_locks(env, hermes_root)?;
    let pids_found = pid_locks.keys().copied().collect::<Vec<_>>();
    let mut pids_terminated = Vec::new();
    let mut pids_killed = Vec::new();

    let plists = discover_plists(env, launch_agents_dir)?;
    let mut plists_booted_out = Vec::with_capacity(plists.len());
    let mut plists_disabled = Vec::with_capacity(plists.len());
    let uid = env.current_uid();

    for (plist, disabled, label) in plists {
        plists_booted_out.push(label.clone());
        plists_disabled.push(disabled.clone());
        if dry_run {
            continue;
        }

        let target = format!("gui/{uid}/{label}");
        let output = env.run_launchctl(&["bootout", &target])?;
        if !launchctl_bootout_succeeded(&output) {
            return Err(OmonError::Config(format!(
                "failed to boot out Hermes LaunchAgent {label}: status {:?}, stdout: {}, stderr: {}",
                output.status,
                output.stdout.trim(),
                output.stderr.trim()
            )));
        }
        if env.exists(&disabled) {
            let backup = env.write_unique(&disabled, &env.read(&plist)?)?;
            env.remove_file(&plist)?;
            if let Some(destination) = plists_disabled.last_mut() {
                *destination = backup;
            }
        } else {
            env.rename(&plist, &disabled)?;
        }
    }

    for (&pid, locks) in &pid_locks {
        if dry_run {
            if verified_alive(env, pid, locks[0].1)? {
                pids_terminated.push(pid);
                pids_killed.push(pid);
            }
            continue;
        }

        if verified_alive(env, pid, locks[0].1)? {
            pids_terminated.push(pid);
            env.terminate(pid)?;
            if !wait_until_dead(env, pid, locks[0].1)? {
                env.kill(pid)?;
                pids_killed.push(pid);
                if !wait_until_dead(env, pid, locks[0].1)? {
                    return Err(OmonError::Config(format!(
                        "Hermes gateway pid {pid} remained alive after SIGKILL"
                    )));
                }
            }
        }

        for (lock, _) in locks {
            env.remove_file(lock)?;
        }
    }

    Ok(GatewayDownSummary {
        pids_found,
        pids_terminated,
        pids_killed,
        plists_booted_out,
        plists_disabled,
    })
}

pub fn looks_like_gateway_runtime_command_line(command: &str) -> bool {
    matches!(
        gateway_command_subcommand(command).as_deref(),
        Some("run" | "restart")
    )
}

fn split_command_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for ch in command.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ch if ch.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            ch => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn gateway_command_subcommand(command: &str) -> Option<String> {
    let raw_tokens = split_command_tokens(command);
    let tokens: Vec<String> = raw_tokens
        .into_iter()
        .map(|t| {
            t.trim_matches(|c| c == '\'' || c == '"')
                .replace('\\', "/")
                .to_ascii_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();

    if tokens.is_empty() {
        return None;
    }

    // Dedicated entrypoints carry no subcommand to inspect.
    for token in &tokens {
        if token == "gateway/run.py" || token.ends_with("/gateway/run.py") {
            return Some("run".into());
        }
        let basename = token.rsplit('/').next().unwrap_or(token.as_str());
        if basename == "hermes-gateway" || basename == "hermes-gateway.exe" {
            return Some("run".into());
        }
    }

    let joined = tokens.join(" ");
    let has_gateway_entry = joined.contains("hermes_cli.main")
        || joined.contains("hermes_cli/main.py")
        || tokens.iter().any(|t| {
            let b = t.rsplit('/').next().unwrap_or(t.as_str());
            b == "hermes" || b == "hermes.exe"
        });

    if !has_gateway_entry {
        return None;
    }

    // Drop profile selectors anywhere: --profile X / -p X / --profile=X / -p=X.
    let mut filtered = Vec::new();
    let mut skip_next = false;
    for token in &tokens {
        if skip_next {
            skip_next = false;
            continue;
        }
        if token == "--profile" || token == "-p" {
            skip_next = true;
            continue;
        }
        if token.starts_with("--profile=") || token.starts_with("-p=") {
            continue;
        }
        filtered.push(token.as_str());
    }

    for (i, token) in filtered.iter().enumerate() {
        if *token != "gateway" {
            continue;
        }
        if i + 1 >= filtered.len() {
            return Some("run".into());
        }
        return Some(filtered[i + 1].to_string());
    }

    None
}

fn verified_alive(env: &dyn MigrationEnv, pid: i32, start: Option<u64>) -> Result<bool> {
    if !env.pid_alive(pid) {
        return Ok(false);
    }
    match (start, env.process_start_time(pid)?) {
        (Some(expected), Some(actual)) if expected == actual => {
            let Some(cmdline) = env.process_command_line(pid)? else {
                return Err(OmonError::Config(format!(
                    "cannot verify live Hermes gateway pid {pid}"
                )));
            };
            Ok(looks_like_gateway_runtime_command_line(&cmdline))
        }
        (Some(_), Some(_)) => Ok(false),
        _ => Err(OmonError::Config(format!(
            "cannot verify live Hermes gateway pid {pid}"
        ))),
    }
}

fn wait_until_dead(env: &dyn MigrationEnv, pid: i32, start: Option<u64>) -> Result<bool> {
    for _ in 0..TERM_WAIT_POLLS {
        env.sleep(TERM_WAIT_INTERVAL);
        if !verified_alive(env, pid, start)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn discover_pid_locks(env: &dyn MigrationEnv, hermes_root: &Path) -> Result<DiscoveredPidLocks> {
    let mut locks = vec![hermes_root.join("gateway.lock")];
    let profiles_root = hermes_root.join("profiles");
    if env.is_dir(&profiles_root) {
        let mut profiles = env
            .read_dir(&profiles_root)?
            .into_iter()
            .filter(|path| env.is_dir(path))
            .collect::<Vec<_>>();
        profiles.sort();
        locks.extend(profiles.into_iter().map(|path| path.join("gateway.lock")));
    }

    let mut pid_locks = DiscoveredPidLocks::new();
    for lock in locks {
        if !env.is_file(&lock) {
            continue;
        }
        let contents = env.read_to_string(&lock)?;
        let parsed = serde_json::from_str::<GatewayLock>(&contents).map_err(|error| {
            OmonError::Config(format!("invalid gateway lock {}: {error}", lock.display()))
        })?;
        if parsed.pid <= 0 {
            return Err(OmonError::Config(format!(
                "invalid gateway pid in {}",
                lock.display()
            )));
        }
        let entries = pid_locks.entry(parsed.pid).or_default();
        if entries.iter().any(|(_, start)| *start != parsed.start_time) {
            return Err(OmonError::Config(format!(
                "conflicting identity for pid {}",
                parsed.pid
            )));
        }
        entries.push((lock, parsed.start_time));
    }
    Ok(pid_locks)
}

fn discover_plists(
    env: &dyn MigrationEnv,
    launch_agents_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf, String)>> {
    if !env.is_dir(launch_agents_dir) {
        return Ok(Vec::new());
    }

    let mut plists = Vec::new();
    for path in env.read_dir(launch_agents_dir)? {
        if !env.is_file(&path) {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(PLIST_PREFIX) || !file_name.ends_with(PLIST_SUFFIX) {
            continue;
        }
        let label = file_name.trim_end_matches(PLIST_SUFFIX).to_owned();
        let disabled = path.with_file_name(format!("{file_name}.disabled"));
        plists.push((path, disabled, label));
    }
    plists.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(plists)
}

fn launchctl_bootout_succeeded(output: &LaunchctlOutput) -> bool {
    if output.status == Some(0) {
        return true;
    }
    let message = format!("{}\n{}", output.stdout, output.stderr).to_ascii_lowercase();
    [
        "not loaded",
        "no such process",
        "could not find specified service",
        "service not found",
    ]
    .iter()
    .any(|expected| message.contains(expected))
}

#[cfg(test)]
mod tests {
    use super::{bring_gateway_down, looks_like_gateway_runtime_command_line};
    use crate::migrate::sys::{
        FakeMigrationEnv, LaunchctlOutput, MigrationEnv, MigrationOperation,
    };
    use crate::Result;
    use chrono::{DateTime, TimeZone, Utc};
    use parking_lot::Mutex;
    use std::collections::{HashMap, VecDeque};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    const ROOT: &str = "/fixtures/.hermes";
    const AGENTS: &str = "/fixtures/Library/LaunchAgents";

    fn fixture() -> FakeMigrationEnv {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 12, 0, 0).unwrap();
        let env = FakeMigrationEnv::new(now);
        env.set_current_uid(501);
        env
    }

    fn write(env: &FakeMigrationEnv, path: &str, contents: &str) {
        env.write(Path::new(path), contents.as_bytes()).unwrap();
    }

    struct ScriptedPidEnv {
        inner: FakeMigrationEnv,
        filesystem: Option<crate::migrate::sys::OsEnv>,
        alive: Mutex<HashMap<i32, VecDeque<bool>>>,
        starts: Mutex<VecDeque<u64>>,
        cmdlines: Mutex<HashMap<i32, String>>,
        events: Mutex<Vec<MigrationOperation>>,
        alive_calls: Mutex<HashMap<i32, usize>>,
        signal_order: Mutex<Vec<(&'static str, i32)>>,
        sleep_calls: Mutex<usize>,
    }

    impl ScriptedPidEnv {
        fn new(inner: FakeMigrationEnv) -> Self {
            Self {
                inner,
                filesystem: None,
                alive: Mutex::new(HashMap::new()),
                starts: Mutex::new(VecDeque::from([1])),
                cmdlines: Mutex::new(HashMap::new()),
                events: Mutex::new(Vec::new()),
                alive_calls: Mutex::new(HashMap::new()),
                signal_order: Mutex::new(Vec::new()),
                sleep_calls: Mutex::new(0),
            }
        }

        fn script_pid(&self, pid: i32, responses: impl IntoIterator<Item = bool>) {
            self.alive
                .lock()
                .insert(pid, responses.into_iter().collect());
        }

        fn script_cmdline(&self, pid: i32, cmdline: impl Into<String>) {
            self.cmdlines.lock().insert(pid, cmdline.into());
        }

        fn alive_calls(&self, pid: i32) -> usize {
            self.alive_calls.lock().get(&pid).copied().unwrap_or(0)
        }

        fn signal_order(&self) -> Vec<(&'static str, i32)> {
            self.signal_order.lock().clone()
        }

        fn fs(&self) -> &dyn MigrationEnv {
            match &self.filesystem {
                Some(env) => env,
                None => &self.inner,
            }
        }
    }

    impl MigrationEnv for ScriptedPidEnv {
        fn read_to_string(&self, path: &Path) -> Result<String> {
            self.fs().read_to_string(path)
        }

        fn read(&self, path: &Path) -> Result<Vec<u8>> {
            self.fs().read(path)
        }

        fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
            self.fs().write(path, bytes)
        }

        fn write_unique(&self, path: &Path, bytes: &[u8]) -> Result<PathBuf> {
            self.fs().write_unique(path, bytes)
        }

        fn rename(&self, from: &Path, to: &Path) -> Result<()> {
            self.fs().rename(from, to)
        }

        fn remove_file(&self, path: &Path) -> Result<()> {
            self.events
                .lock()
                .push(MigrationOperation::RemoveFile(path.into()));
            self.fs().remove_file(path)
        }

        fn acquire_jobs_lock(
            &self,
            path: &Path,
        ) -> Result<Box<dyn crate::migrate::sys::MigrationLock>> {
            self.fs().acquire_jobs_lock(path)
        }

        fn exists(&self, path: &Path) -> bool {
            self.fs().exists(path)
        }

        fn is_file(&self, path: &Path) -> bool {
            self.fs().is_file(path)
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.fs().is_dir(path)
        }

        fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
            self.fs().read_dir(path)
        }

        fn create_dir_all(&self, path: &Path) -> Result<()> {
            self.fs().create_dir_all(path)
        }

        fn current_uid(&self) -> u32 {
            self.inner.current_uid()
        }

        fn now(&self) -> DateTime<Utc> {
            self.inner.now()
        }

        fn pid_alive(&self, pid: i32) -> bool {
            self.events.lock().push(MigrationOperation::PidAlive(pid));
            *self.alive_calls.lock().entry(pid).or_default() += 1;
            let mut alive = self.alive.lock();
            let responses = alive.entry(pid).or_default();
            match responses.len() {
                0 => false,
                1 => responses[0],
                _ => responses.pop_front().unwrap_or(false),
            }
        }

        fn process_start_time(&self, pid: i32) -> Result<Option<u64>> {
            self.events
                .lock()
                .push(MigrationOperation::ProcessStartTime(pid));
            let mut starts = self.starts.lock();
            Ok(if starts.len() > 1 {
                starts.pop_front()
            } else {
                starts.front().copied()
            })
        }

        fn process_command_line(&self, pid: i32) -> Result<Option<String>> {
            self.events
                .lock()
                .push(MigrationOperation::ProcessCommandLine(pid));
            if let Some(cmd) = self.cmdlines.lock().get(&pid).cloned() {
                return Ok(Some(cmd));
            }
            if let Ok(Some(cmd)) = self.inner.process_command_line(pid) {
                return Ok(Some(cmd));
            }
            Ok(Some("python3 -m hermes_cli.main gateway run".into()))
        }

        fn terminate(&self, pid: i32) -> Result<()> {
            self.events.lock().push(MigrationOperation::Terminate(pid));
            self.signal_order.lock().push(("TERM", pid));
            self.inner.terminate(pid)
        }

        fn kill(&self, pid: i32) -> Result<()> {
            self.events.lock().push(MigrationOperation::Kill(pid));
            self.signal_order.lock().push(("KILL", pid));
            self.inner.kill(pid)
        }

        fn sleep(&self, duration: Duration) {
            // Advance the scripted lifecycle only; bounded exit time is under test.
            self.events.lock().push(MigrationOperation::Sleep(duration));
            *self.sleep_calls.lock() += 1;
        }

        fn run_launchctl(&self, args: &[&str]) -> Result<LaunchctlOutput> {
            self.events.lock().push(MigrationOperation::Bootout(
                args.iter().map(|arg| (*arg).into()).collect(),
            ));
            self.inner.run_launchctl(args)
        }
    }

    #[test]
    fn valid_and_invalid_gateway_command_lines() {
        for valid in [
            "python3 -m hermes_cli.main gateway run",
            "python3 /path/to/hermes_cli/main.py gateway run",
            "/usr/local/bin/hermes gateway run",
            "/usr/local/bin/hermes --profile work gateway run",
            "/usr/local/bin/hermes -p=work gateway run",
            "/usr/local/bin/hermes -p work gateway run",
            "hermes gateway restart",
            "python3 gateway/run.py",
            "python3 /opt/hermes/gateway/run.py",
            "hermes-gateway",
            "/opt/hermes/bin/hermes-gateway",
            "\"/Applications/Hermes Gateway.app/hermes-gateway\"",
        ] {
            assert!(
                looks_like_gateway_runtime_command_line(valid),
                "expected valid: {valid}"
            );
        }

        for invalid in [
            "hermes gateway status",
            "python3 -m hermes_cli.main gateway status",
            "python3 -m tui_gateway",
            "python3 /var/services/worker.py",
            "python3 -m hermes_cli.main status",
            "python3 -m hermes_cli.main --profile work status",
            "cat /tmp/hermes",
            "sh -c hermes",
            "",
        ] {
            assert!(
                !looks_like_gateway_runtime_command_line(invalid),
                "expected invalid: {invalid}"
            );
        }
    }

    #[test]
    fn matching_start_wrong_live_command_never_signaled() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4242,"start_time":1}"#,
        );
        env.set_pid_alive(4242, true);
        env.set_process_start_time(4242, 1);
        env.set_process_command_line(4242, "python3 /var/services/unrelated_worker.py");
        write(&env, &format!("{AGENTS}/ai.hermes.gateway.plist"), "active");

        let result = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false);
        println!(
            "wrong-command signals={:?}/{:?} result={result:?}",
            env.terminate_calls(),
            env.kill_calls()
        );
        assert!(env.terminate_calls().is_empty() && env.kill_calls().is_empty());
        assert!(result.is_ok());

        // ScriptedPidEnv control with matching start and wrong command line
        let inner = fixture();
        write(
            &inner,
            &format!("{ROOT}/gateway.lock"),
            r#"{"pid":4242,"start_time":1}"#,
        );
        let scripted = ScriptedPidEnv::new(inner);
        scripted.script_pid(4242, [true]);
        scripted.script_cmdline(4242, "hermes gateway status");
        let result = bring_gateway_down(&scripted, Path::new(ROOT), Path::new(AGENTS), false);
        assert!(result.is_ok());
        assert!(scripted.signal_order().is_empty());
    }

    #[test]
    fn stale_pid_never_signaled() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4242,"start_time":1}"#,
        );
        env.set_pid_alive(4242, true);
        env.set_process_start_time(4242, 2);
        write(&env, &format!("{AGENTS}/ai.hermes.gateway.plist"), "active");
        write(
            &env,
            &format!("{AGENTS}/ai.hermes.gateway.plist.disabled"),
            "backup",
        );

        let result = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false);
        println!(
            "C06 signals={:?}/{:?} bootout={:?} result={result:?}",
            env.terminate_calls(),
            env.kill_calls(),
            env.launchctl_calls()
        );
        assert!(env.terminate_calls().is_empty() && env.kill_calls().is_empty());
        assert_eq!(env.launchctl_calls().len(), 1);
        assert!(result.is_ok());
        assert_eq!(
            env.read_to_string(Path::new(&format!(
                "{AGENTS}/ai.hermes.gateway.plist.disabled"
            )))
            .unwrap(),
            "backup"
        );
        assert!(!env.exists(Path::new(&format!("{AGENTS}/ai.hermes.gateway.plist"))));
    }

    #[test]
    fn alive_pids_terminate_then_escalate_only_when_still_alive() {
        let inner = fixture();
        write(
            &inner,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4101,"kind":"hermes-gateway","start_time":1}"#,
        );
        write(
            &inner,
            "/fixtures/.hermes/profiles/work/gateway.lock",
            r#"{"pid":4102,"kind":"hermes-gateway","start_time":1}"#,
        );
        let env = ScriptedPidEnv::new(inner);
        env.script_pid(
            4101,
            std::iter::repeat_n(true, super::TERM_WAIT_POLLS + 1).chain([false]),
        );
        env.script_pid(4102, [true, false]);

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_found, [4101, 4102]);
        assert_eq!(summary.pids_terminated, [4101, 4102]);
        assert_eq!(summary.pids_killed, [4101]);
        assert_eq!(env.inner.terminate_calls(), [4101, 4102]);
        assert_eq!(env.inner.kill_calls(), [4101]);
        assert_eq!(
            env.signal_order(),
            [("TERM", 4101), ("KILL", 4101), ("TERM", 4102)]
        );
        assert_eq!(env.alive_calls(4101), super::TERM_WAIT_POLLS + 2);
        assert_eq!(env.alive_calls(4102), 2);
        assert_eq!(*env.sleep_calls.lock(), super::TERM_WAIT_POLLS + 2);
        assert!(!env
            .inner
            .exists(Path::new("/fixtures/.hermes/gateway.lock")));
        assert!(!env
            .inner
            .exists(Path::new("/fixtures/.hermes/profiles/work/gateway.lock")));
        println!(
            "signal-order={:?} terminated={:?} killed={:?}",
            env.signal_order(),
            summary.pids_terminated,
            summary.pids_killed
        );
    }

    #[test]
    fn pid_that_dies_during_bounded_wait_is_not_killed() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4151,"kind":"hermes-gateway","start_time":1}"#,
        );
        env.set_pid_alive(4151, true);
        env.set_process_start_time(4151, 1);
        env.set_process_command_line(4151, "python3 -m hermes_cli.main gateway run");
        env.set_pid_death_after_sleeps(4151, 3);

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_terminated, [4151]);
        assert!(summary.pids_killed.is_empty());
        assert_eq!(env.terminate_calls(), [4151]);
        assert!(env.kill_calls().is_empty());
        assert_eq!(
            env.operations()
                .iter()
                .filter(|operation| matches!(operation, MigrationOperation::Sleep(_)))
                .count(),
            3
        );
        assert!(!env.exists(Path::new("/fixtures/.hermes/gateway.lock")));
    }

    #[test]
    fn pid_alive_after_bounded_wait_is_killed() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4152,"kind":"hermes-gateway","start_time":1}"#,
        );
        env.set_pid_alive(4152, true);
        env.set_process_start_time(4152, 1);
        env.set_process_command_line(4152, "python3 -m hermes_cli.main gateway run");

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_terminated, [4152]);
        assert_eq!(summary.pids_killed, [4152]);
        assert_eq!(env.terminate_calls(), [4152]);
        assert_eq!(env.kill_calls(), [4152]);
        assert_eq!(
            env.operations()
                .iter()
                .filter(|operation| matches!(operation, MigrationOperation::Sleep(_)))
                .count(),
            super::TERM_WAIT_POLLS + 1
        );
        assert!(!env.exists(Path::new("/fixtures/.hermes/gateway.lock")));
    }

    #[test]
    fn every_matching_plist_is_booted_out_then_renamed_disabled() {
        let env = fixture();
        for label in [
            "ai.hermes.gateway",
            "ai.hermes.gateway-advisor",
            "ai.hermes.gateway-marketer",
        ] {
            write(&env, &format!("{AGENTS}/{label}.plist"), "<plist/>");
        }
        write(
            &env,
            &format!("{AGENTS}/com.example.gateway.plist"),
            "<plist/>",
        );

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(
            env.launchctl_calls(),
            [
                vec![
                    String::from("bootout"),
                    String::from("gui/501/ai.hermes.gateway-advisor"),
                ],
                vec![
                    String::from("bootout"),
                    String::from("gui/501/ai.hermes.gateway-marketer"),
                ],
                vec![
                    String::from("bootout"),
                    String::from("gui/501/ai.hermes.gateway"),
                ],
            ]
        );
        assert_eq!(env.rename_calls().len(), 3);
        assert!(env
            .rename_calls()
            .iter()
            .all(|(from, to)| to == &PathBuf::from(format!("{}.disabled", from.display()))));
        assert_eq!(summary.plists_booted_out.len(), 3);
        assert_eq!(summary.plists_disabled.len(), 3);
        println!(
            "launchctl={:?} renames={:?}",
            env.launchctl_calls(),
            env.rename_calls()
        );
    }

    #[test]
    fn stale_pid_lock_is_removed_and_disabled_plists_remain_a_no_op() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4201,"kind":"hermes-gateway","start_time":1}"#,
        );
        write(
            &env,
            &format!("{AGENTS}/ai.hermes.gateway.plist.disabled"),
            "<plist/>",
        );

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_found, [4201]);
        assert!(summary.pids_terminated.is_empty());
        assert!(summary.pids_killed.is_empty());
        assert!(summary.plists_booted_out.is_empty());
        assert!(summary.plists_disabled.is_empty());
        assert!(env.terminate_calls().is_empty());
        assert!(env.kill_calls().is_empty());
        assert!(env.launchctl_calls().is_empty());
        assert!(env.rename_calls().is_empty());
        assert!(!env.exists(Path::new("/fixtures/.hermes/gateway.lock")));
        println!("stale-lock-cleanup summary={summary:?}");
    }

    #[test]
    fn not_loaded_bootout_is_success_and_plist_is_still_disabled() {
        let env = fixture();
        let plist = format!("{AGENTS}/ai.hermes.gateway.plist");
        write(&env, &plist, "<plist/>");
        env.set_launchctl_response(LaunchctlOutput {
            status: Some(3),
            stdout: String::new(),
            stderr: "Boot-out failed: 3: No such process".into(),
        });

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(env.launchctl_calls().len(), 1);
        assert_eq!(env.rename_calls().len(), 1);
        assert_eq!(summary.plists_booted_out, ["ai.hermes.gateway"]);
        assert_eq!(summary.plists_disabled.len(), 1);
        println!(
            "not-loaded tolerated launchctl={:?} rename={:?}",
            env.launchctl_calls(),
            env.rename_calls()
        );
    }

    #[test]
    fn dry_run_reports_intended_actions_with_zero_side_effects() {
        let env = fixture();
        write(
            &env,
            "/fixtures/.hermes/gateway.lock",
            r#"{"pid":4301,"kind":"hermes-gateway","start_time":1}"#,
        );
        env.set_pid_alive(4301, true);
        env.set_process_start_time(4301, 1);
        env.set_process_command_line(4301, "python3 -m hermes_cli.main gateway run");
        write(
            &env,
            &format!("{AGENTS}/ai.hermes.gateway.plist"),
            "<plist/>",
        );

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), true).unwrap();

        assert_eq!(summary.pids_found, [4301]);
        assert_eq!(summary.pids_terminated, [4301]);
        assert_eq!(summary.pids_killed, [4301]);
        assert_eq!(summary.plists_booted_out, ["ai.hermes.gateway"]);
        assert_eq!(summary.plists_disabled.len(), 1);
        assert!(env.terminate_calls().is_empty());
        assert!(env.kill_calls().is_empty());
        assert!(env.launchctl_calls().is_empty());
        assert!(env.rename_calls().is_empty());
        println!("dry-run intended summary={summary:?}");
    }

    #[test]
    fn service_unloads_before_verified_signals_and_delayed_kill_exit() {
        let inner = fixture();
        write(
            &inner,
            &format!("{ROOT}/gateway.lock"),
            r#"{"pid":4242,"start_time":1}"#,
        );
        write(
            &inner,
            &format!("{AGENTS}/ai.hermes.gateway.plist"),
            "active",
        );
        let env = ScriptedPidEnv::new(inner);
        // Initial live observation, ten TERM observations, two still-live KILL
        // observations, then the exact scripted exit. No wall-clock waits.
        env.script_pid(
            4242,
            std::iter::repeat_n(true, super::TERM_WAIT_POLLS + 3).chain([false]),
        );

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_killed, [4242]);
        let events = env.events.lock();
        let bootout = events
            .iter()
            .position(|event| matches!(event, MigrationOperation::Bootout(_)))
            .unwrap();
        for signal in [
            MigrationOperation::Terminate(4242),
            MigrationOperation::Kill(4242),
        ] {
            let index = events.iter().position(|event| *event == signal).unwrap();
            assert!(bootout < index);
            assert_eq!(
                events[index - 2],
                MigrationOperation::ProcessStartTime(4242)
            );
            assert_eq!(
                events[index - 1],
                MigrationOperation::ProcessCommandLine(4242)
            );
        }
        let kill = events
            .iter()
            .position(|event| *event == MigrationOperation::Kill(4242))
            .unwrap();
        assert_eq!(
            events[kill + 1..]
                .iter()
                .filter(|event| matches!(event, MigrationOperation::Sleep(_)))
                .count(),
            3
        );
        assert_eq!(
            events.last(),
            Some(&MigrationOperation::RemoveFile(PathBuf::from(format!(
                "{ROOT}/gateway.lock"
            ))))
        );
        println!("C06 delayed-exit events={events:?}");
    }

    #[test]
    fn reused_pid_during_term_wait_never_receives_kill() {
        let inner = fixture();
        write(
            &inner,
            &format!("{ROOT}/gateway.lock"),
            r#"{"pid":4242,"start_time":1}"#,
        );
        let env = ScriptedPidEnv::new(inner);
        env.script_pid(4242, [true]);
        *env.starts.lock() = VecDeque::from([1, 2]);

        let summary = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).unwrap();

        assert_eq!(summary.pids_terminated, [4242]);
        assert!(summary.pids_killed.is_empty());
        assert_eq!(env.signal_order(), [("TERM", 4242)]);
        println!("C06 reuse-after-TERM events={:?}", env.events.lock());
    }

    #[test]
    fn kill_timeout_preserves_lock_and_is_bounded() {
        let inner = fixture();
        write(
            &inner,
            &format!("{ROOT}/gateway.lock"),
            r#"{"pid":4242,"start_time":1}"#,
        );
        let env = ScriptedPidEnv::new(inner);
        env.script_pid(4242, [true]);

        let result = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false);

        assert!(result.is_err());
        assert_eq!(env.signal_order(), [("TERM", 4242), ("KILL", 4242)]);
        assert_eq!(*env.sleep_calls.lock(), 2 * super::TERM_WAIT_POLLS);
        assert!(env.exists(Path::new(&format!("{ROOT}/gateway.lock"))));
        println!(
            "C06 timeout result={result:?} virtual_waits={}",
            env.sleep_calls.lock()
        );
    }

    #[test]
    fn failed_bootout_preserves_files_and_never_signals() {
        for transport_error in [false, true] {
            let env = fixture();
            let lock = format!("{ROOT}/gateway.lock");
            let plist = format!("{AGENTS}/ai.hermes.gateway.plist");
            write(&env, &lock, r#"{"pid":4242,"start_time":1}"#);
            write(&env, &plist, "active");
            env.set_pid_alive(4242, true);
            env.set_process_start_time(4242, 1);
            env.set_process_command_line(4242, "python3 -m hermes_cli.main gateway run");
            if transport_error {
                env.set_launchctl_error("fixture launchctl unavailable");
            } else {
                env.set_launchctl_response(LaunchctlOutput {
                    status: Some(5),
                    stdout: String::new(),
                    stderr: "permission denied".into(),
                });
            }

            let result = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false);

            assert!(result.is_err());
            assert!(env.terminate_calls().is_empty() && env.kill_calls().is_empty());
            assert_eq!(env.read_to_string(Path::new(&plist)).unwrap(), "active");
            assert!(env.exists(Path::new(&lock)));
            println!("C06 bootout-refusal transport_error={transport_error} result={result:?}");
        }
    }

    #[test]
    fn unknown_or_conflicting_identity_never_authorizes_signals() {
        for payload in [
            r#"{"pid":4242}"#,
            r#"{"pid":4242,"start_time":1}"#,
            r#"{"pid":0,"start_time":1}"#,
        ] {
            let env = fixture();
            write(&env, &format!("{ROOT}/gateway.lock"), payload);
            env.set_pid_alive(4242, true);
            // Unknown OS start time must not be treated as a match.
            let result = bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false);
            assert!(result.is_err());
            assert!(env.terminate_calls().is_empty() && env.kill_calls().is_empty());
            assert!(env.exists(Path::new(&format!("{ROOT}/gateway.lock"))));
            println!("C06 identity-refusal payload={payload} result={result:?}");
        }
        let env = fixture();
        write(
            &env,
            &format!("{ROOT}/gateway.lock"),
            r#"{"pid":4242,"start_time":1}"#,
        );
        write(
            &env,
            &format!("{ROOT}/profiles/work/gateway.lock"),
            r#"{"pid":4242,"start_time":2}"#,
        );
        assert!(bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).is_err());
        assert!(!env.operations().iter().any(|event| matches!(
            event,
            MigrationOperation::Terminate(_)
                | MigrationOperation::Kill(_)
                | MigrationOperation::Bootout(_)
                | MigrationOperation::RemoveFile(_)
        )));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn migration_entry_retires_service_with_real_files_and_database() {
        use crate::migrate::{run_migrate_with, MigrateArgs, MigrationPaths};
        use std::os::unix::fs::PermissionsExt;

        for current_start in [1, 2] {
            let temp = tempfile::tempdir().unwrap();
            let home = temp.path().join("home");
            let root = home.join("hermes");
            let agents = home.join("Library/LaunchAgents");
            let lock = root.join("gateway.lock");
            let plist = agents.join("ai.hermes.gateway.plist");
            let disabled = agents.join("ai.hermes.gateway.plist.disabled");
            let mut env = ScriptedPidEnv::new(fixture());
            env.filesystem = Some(crate::migrate::sys::OsEnv);
            env.create_dir_all(&root).unwrap();
            env.create_dir_all(&agents).unwrap();
            env.write(
                &root.join("config.yaml"),
                b"model:\n  default: fixture-model\n",
            )
            .unwrap();
            env.write(&lock, br#"{"pid":4242,"start_time":1}"#).unwrap();
            env.write(&plist, b"active").unwrap();
            env.write(&disabled, b"backup").unwrap();
            env.script_pid(4242, [true, false]);
            *env.starts.lock() = VecDeque::from([current_start]);
            let database = crate::Database::connect(&format!(
                "sqlite://{}",
                temp.path().join("gateway.db").display()
            ))
            .await
            .unwrap();

            let summary = run_migrate_with(
                MigrateArgs {
                    dry_run: false,
                    no_cutover: false,
                },
                &env,
                MigrationPaths {
                    hermes_root: root,
                    target_env: home.join("gateway.env"),
                    launch_agents_dir: agents,
                },
                Some(database.pool().clone()),
            )
            .await
            .unwrap();

            let expected_pids = if current_start == 1 {
                vec![4242]
            } else {
                vec![]
            };
            assert_eq!(summary.pids_stopped, expected_pids);
            assert!(env.inner.kill_calls().is_empty());
            assert_eq!(
                env.inner.launchctl_calls(),
                [vec![
                    "bootout".to_string(),
                    "gui/501/ai.hermes.gateway".to_string()
                ]]
            );
            assert!(!plist.exists() && !lock.exists());
            assert_eq!(env.read(&disabled).unwrap(), b"backup");
            assert_eq!(summary.plists_disabled.len(), 1);
            let backup = &summary.plists_disabled[0];
            assert_ne!(backup, &disabled);
            assert_eq!(env.read(backup).unwrap(), b"active");
            assert_eq!(
                std::fs::metadata(backup).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let cron_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs")
                .fetch_one(database.pool())
                .await
                .unwrap();
            assert_eq!(cron_count, 0);
            println!("C06 production-entry current_start={current_start} stopped={:?} backup_mode=600 original_backup_intact=true events={:?}", summary.pids_stopped, env.events.lock());
            database.pool().close().await;
            let path = temp.path().to_path_buf();
            temp.close().unwrap();
            assert!(!path.exists());
            println!("C06 production-entry cleanup=true database_closed=true");
        }
    }

    #[test]
    fn malformed_locks_refuse_before_side_effects() {
        let inner = fixture();
        write(&inner, "/fixtures/.hermes/gateway.lock", "{not-json");
        write(
            &inner,
            "/fixtures/.hermes/profiles/work/gateway.lock",
            r#"{"pid":4401}"#,
        );
        let env = ScriptedPidEnv::new(inner);
        env.script_pid(
            4401,
            std::iter::repeat_n(true, super::TERM_WAIT_POLLS + 1).chain([false]),
        );

        assert!(bring_gateway_down(&env, Path::new(ROOT), Path::new(AGENTS), false).is_err());
        assert!(env.inner.terminate_calls().is_empty());
        assert!(env.inner.kill_calls().is_empty());
        assert!(env
            .inner
            .exists(Path::new("/fixtures/.hermes/gateway.lock")));
    }
}

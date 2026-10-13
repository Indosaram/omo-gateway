use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use uuid::Uuid;

use crate::error::{OmonError, Result};

pub const DRAIN_REQUEST_FILENAME: &str = ".drain_request.json";
const DRAIN_REQUEST_MAX_AGE: chrono::TimeDelta = chrono::TimeDelta::seconds(3600);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrainRequest {
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub requested_at: Option<String>,
    #[serde(default)]
    pub principal: Option<String>,
    #[serde(default)]
    pub epoch: Option<String>,
    #[serde(default)]
    pub suppress_notification: bool,
}

fn default_action() -> String {
    "drain".to_string()
}

impl Default for DrainRequest {
    fn default() -> Self {
        Self {
            action: default_action(),
            requested_at: Some(Utc::now().to_rfc3339()),
            principal: Some("drain-control".to_string()),
            epoch: Some(current_instantiation_epoch().to_string()),
            suppress_notification: false,
        }
    }
}

static PROCESS_EPOCH: OnceLock<String> = OnceLock::new();

/// Computes the instantiation epoch for this container/VM/process.
///
/// On Linux: reads boot_id from /proc and starttime of PID 1.
/// Without either identity source: returns an empty epoch, accepting external markers.
pub fn current_instantiation_epoch() -> &'static str {
    PROCESS_EPOCH.get_or_init(|| {
        let mut boot_id = String::new();
        if let Ok(content) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
            boot_id = content.trim().to_string();
        }

        let mut pid1_start = String::new();
        if let Ok(stat) = fs::read_to_string("/proc/1/stat") {
            if let Some((_, tail)) = stat.rsplit_once(')') {
                let fields: Vec<&str> = tail.split_whitespace().collect();
                if fields.len() >= 20 {
                    pid1_start = fields[19].to_string();
                }
            }
        }

        if !boot_id.is_empty() || !pid1_start.is_empty() {
            format!("{boot_id}:{pid1_start}")
        } else {
            // Unknown boot identity must not make other processes' markers stale.
            String::new()
        }
    })
}

/// Checks if a marker's epoch is a definite mismatch with the current process boot epoch.
///
/// Lenient by design: returns false (not stale) if either current epoch or marker epoch is empty.
pub fn is_marker_epoch_stale(marker_epoch: Option<&str>, current_epoch: &str) -> bool {
    let current_trimmed = current_epoch.trim();
    if current_trimmed.is_empty() {
        return false;
    }
    match marker_epoch.map(str::trim).filter(|s| !s.is_empty()) {
        None => false, // legacy or contentless marker -> not stale (fail-safe)
        Some(m_epoch) => m_epoch != current_trimmed,
    }
}

/// Validates the raw JSON marker content against the current epoch and time.
///
/// Returns `Some(DrainRequest)` if a valid (non-stale) drain request is present,
/// or `None` if the marker is stale or explicitly requests a non-drain action.
pub fn validate_marker(content: &str, current_epoch: &str) -> Option<DrainRequest> {
    validate_marker_at(content, current_epoch, Utc::now())
}

fn validate_marker_at(
    content: &str,
    current_epoch: &str,
    now: DateTime<Utc>,
) -> Option<DrainRequest> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Some(DrainRequest::default());
    }

    match serde_json::from_str::<DrainRequest>(trimmed) {
        Ok(req) => {
            if req.action != "drain" {
                return None;
            }
            if is_marker_epoch_stale(req.epoch.as_deref(), current_epoch) {
                return None;
            }
            // Missing, malformed and future timestamps remain fail-safe active.
            if let Some(requested_at) = req.requested_at.as_deref().and_then(parse_requested_at) {
                if now.signed_duration_since(requested_at) > DRAIN_REQUEST_MAX_AGE {
                    return None;
                }
            }
            Some(req)
        }
        Err(_) => {
            // Corrupt / unparseable file is treated as a valid contentless drain request (fail-safe toward quiescing)
            Some(DrainRequest {
                action: "drain".to_string(),
                requested_at: None,
                principal: Some("malformed-marker".to_string()),
                epoch: None,
                suppress_notification: false,
            })
        }
    }
}

// Like upstream's ISO parser, offset-free timestamps are UTC, not local time.
fn parse_requested_at(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|date| date.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"]
                .iter()
                .find_map(|format| NaiveDateTime::parse_from_str(raw, format).ok())
                .map(|date| date.and_utc())
        })
        .or_else(|| {
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .ok()?
                .and_hms_opt(0, 0, 0)
                .map(|date| date.and_utc())
        })
}

pub fn drain_request_path(dir: &Path) -> PathBuf {
    dir.join(DRAIN_REQUEST_FILENAME)
}

/// Writes a fresh `.drain_request.json` marker file atomically.
pub fn write_drain_request(
    dir: &Path,
    principal: Option<&str>,
    suppress_notification: bool,
) -> Result<DrainRequest> {
    let req = DrainRequest {
        action: "drain".to_string(),
        requested_at: Some(Utc::now().to_rfc3339()),
        principal: principal.map(str::to_string),
        epoch: Some(current_instantiation_epoch().to_string()),
        suppress_notification,
    };
    let path = drain_request_path(dir);
    let tmp_path = dir.join(format!(".drain_request.{}.tmp", Uuid::new_v4()));
    let payload = serde_json::to_string_pretty(&req)
        .map_err(|e| OmonError::Config(format!("failed to serialize drain request: {e}")))?;

    fs::write(&tmp_path, payload)
        .map_err(|e| OmonError::Config(format!("failed to write drain request temp file: {e}")))?;
    if let Err(error) = fs::rename(&tmp_path, &path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(OmonError::Config(format!(
            "failed to commit drain request marker: {error}"
        )));
    }

    Ok(req)
}

/// Removes the `.drain_request.json` marker file (cancel drain). Returns true if one was present.
pub fn clear_drain_request(dir: &Path) -> Result<bool> {
    let path = drain_request_path(dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        // Already gone (never written, or removed by another process).
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(OmonError::Config(format!(
            "failed to remove drain request marker: {error}"
        ))),
    }
}

/// Checks the state dir for an active, valid drain request marker.
pub fn check_drain_requested(dir: &Path, current_epoch: &str) -> Option<DrainRequest> {
    check_drain_requested_at(dir, current_epoch, Utc::now())
}

fn check_drain_requested_at(
    dir: &Path,
    current_epoch: &str,
    now: DateTime<Utc>,
) -> Option<DrainRequest> {
    let path = drain_request_path(dir);
    match fs::read_to_string(&path) {
        Ok(content) => validate_marker_at(&content, current_epoch, now),
        // Absent, including a marker removed by a cancel between two filesystem calls.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            // The marker exists but cannot be read (permissions, transient IO,
            // wrong file type). Absent means "not draining"; unreadable must
            // never be mistaken for absent, so fail CLOSED.
            tracing::warn!(
                path = %path.display(),
                %error,
                "drain marker exists but is unreadable; failing closed and treating the gateway as draining"
            );
            Some(DrainRequest {
                action: "drain".to_string(),
                requested_at: None,
                principal: Some("unreadable-marker".to_string()),
                epoch: None,
                suppress_notification: false,
            })
        }
    }
}

/// Background watcher for the `.drain_request.json` file.
pub struct DrainWatcher {
    state_dir: PathBuf,
    interval: Duration,
    drain_tx: watch::Sender<bool>,
    drain_rx: watch::Receiver<bool>,
}

impl DrainWatcher {
    pub fn new(state_dir: PathBuf, interval: Duration) -> Self {
        let (drain_tx, drain_rx) = watch::channel(false);
        Self {
            state_dir,
            interval,
            drain_tx,
            drain_rx,
        }
    }

    pub fn receiver(&self) -> watch::Receiver<bool> {
        self.drain_rx.clone()
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(self.serve(tokio_util::sync::CancellationToken::new()))
    }

    pub async fn serve(self, shutdown: tokio_util::sync::CancellationToken) {
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => return,
                _ = ticker.tick() => { self.scan_at(current_instantiation_epoch(), Utc::now()); }
            }
        }
    }

    /// One completed file scan; the caller owns the existing watcher lifecycle.
    pub fn scan_at(&self, epoch: &str, now: DateTime<Utc>) -> bool {
        if let Some(req) = check_drain_requested_at(&self.state_dir, epoch, now) {
            tracing::warn!(
                principal = ?req.principal,
                epoch = ?req.epoch,
                "valid drain request detected by drain watcher"
            );
            self.drain_tx.send_if_modified(|val| {
                if !*val {
                    *val = true;
                    true
                } else {
                    false
                }
            });
            true
        } else {
            self.drain_tx.send_if_modified(|val| {
                if *val {
                    *val = false;
                    true
                } else {
                    false
                }
            });
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_marker_reads_as_not_draining() {
        let dir = tempfile::tempdir().unwrap();
        assert!(super::check_drain_requested_at(dir.path(), "", chrono::Utc::now()).is_none());
    }

    use super::*;

    #[test]
    fn drain_marker_expiry_is_lenient_and_bounded() {
        // Given a fixed scan time, independent of the date this test runs.
        let now = "2026-09-05T12:00:00Z".parse().unwrap();
        for (id, timestamp, epoch, current, active) in [
            (
                "expired_same_epoch",
                Some("2026-09-05T10:59:59Z"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
            (
                "exact_boundary",
                Some("2026-09-05T11:00:00Z"),
                Some("boot:1"),
                "boot:1",
                true,
            ),
            (
                "fraction_past_boundary",
                Some("2026-09-05T10:59:59.999Z"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
            (
                "fresh",
                Some("2026-09-05T12:00:00Z"),
                Some("boot:1"),
                "boot:1",
                true,
            ),
            ("missing", None, Some("boot:1"), "boot:1", true),
            (
                "malformed",
                Some("not-a-time"),
                Some("boot:1"),
                "boot:1",
                true,
            ),
            (
                "future",
                Some("2026-09-05T12:00:01Z"),
                Some("boot:1"),
                "boot:1",
                true,
            ),
            ("wrong_epoch", None, Some("old:1"), "boot:1", false),
            (
                "legacy_expired",
                Some("2026-09-05T10:59:59Z"),
                None,
                "boot:1",
                false,
            ),
            (
                "unknown_epoch_expired",
                Some("2026-09-05T10:59:59Z"),
                Some("old:1"),
                "",
                false,
            ),
            (
                "unknown_epoch_fresh",
                Some("2026-09-05T12:00:00Z"),
                Some("old:1"),
                "",
                true,
            ),
            (
                "offset_expired",
                Some("2026-09-05T12:59:59+02:00"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
            (
                "naive_utc_expired",
                Some("2026-09-05T10:59:59"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
            (
                "space_utc_expired",
                Some("2026-09-05 10:59:59"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
            (
                "date_utc_expired",
                Some("2026-09-05"),
                Some("boot:1"),
                "boot:1",
                false,
            ),
        ] {
            let payload = serde_json::json!({
                "action": "drain", "requested_at": timestamp, "epoch": epoch,
                "principal": "u75-operator", "suppress_notification": true
            })
            .to_string();
            // When the marker is validated at that exact time.
            let actual = validate_marker_at(&payload, current, now);
            println!("case={id} T={now} current={current:?} payload={payload} expected_active={active} actual={actual:?}");
            // Then only definite expiry or epoch mismatch makes it inactive.
            assert_eq!(actual.is_some(), active, "{id}");
            if let Some(req) = actual {
                assert_eq!(req.principal.as_deref(), Some("u75-operator"));
                assert!(req.suppress_notification);
            }
        }
        for payload in ["", "{malformed", "{}", r#"{"requested_at":42}"#] {
            assert!(validate_marker_at(payload, "boot:1", now).is_some());
            println!("fail_safe payload={payload:?} active=true");
        }
        assert!(validate_marker_at(r#"{"action":"restart"}"#, "boot:1", now).is_none());
    }

    #[tokio::test]
    async fn drain_marker_expiry_local_surface() {
        // Given a real marker and a receiver subscribed before either scan.
        let temp = tempfile::tempdir().unwrap();
        let now = "2026-09-05T12:00:00Z".parse().unwrap();
        let epoch = current_instantiation_epoch();
        let watcher = DrainWatcher::new(temp.path().to_path_buf(), Duration::from_secs(3));
        let mut rx = watcher.receiver();
        let path = drain_request_path(temp.path());
        let mut req = DrainRequest {
            requested_at: Some("2026-09-05T10:59:59Z".to_string()),
            principal: Some("u75-surface".to_string()),
            suppress_notification: true,
            ..DrainRequest::default()
        };
        let expired = serde_json::to_string(&req).unwrap();
        fs::write(&path, &expired).unwrap();
        // When the exact production watcher scan completes with injected time.
        let active = watcher.scan_at(epoch, now);
        println!(
            "scan_completed=true payload={expired} T={now} active={active} receiver={}",
            *rx.borrow()
        );
        // Then the completed scan (not an event timeout) proves inactivity.
        assert!(!active);
        assert!(!*rx.borrow());
        assert!(!rx.has_changed().unwrap());
        assert!(check_drain_requested_at(temp.path(), epoch, now).is_none());

        // Given the same subscribed receiver, refresh only the timestamp to T.
        req.requested_at = Some("2026-09-05T12:00:00Z".to_string());
        let fresh = serde_json::to_string(&req).unwrap();
        fs::write(&path, &fresh).unwrap();
        // When scanned again, then the exact subscribed state-change fires.
        assert!(watcher.scan_at(epoch, now));
        tokio::time::timeout(Duration::from_secs(5), rx.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(*rx.borrow());
        assert_eq!(check_drain_requested_at(temp.path(), epoch, now), Some(req));
        println!("scan_completed=true payload={fresh} T={now} active=true state_change=true");

        // Real public entry used by main: fresh writer -> check -> spawn -> signal.
        let written = write_drain_request(temp.path(), Some("u75-public-entry"), true).unwrap();
        assert_eq!(
            check_drain_requested(temp.path(), epoch),
            Some(written.clone())
        );
        let running = DrainWatcher::new(temp.path().to_path_buf(), Duration::from_secs(3));
        let mut running_rx = running.receiver();
        use futures_util::FutureExt;

        let handle = running.spawn();
        // Catch assertion panics so the owned watcher is reaped before unwinding.
        let outcome = std::panic::AssertUnwindSafe(async {
            tokio::time::timeout(Duration::from_secs(5), running_rx.changed())
                .await
                .expect("watcher must signal the fresh marker before the deadline")
                .expect("watcher state channel must remain open");
            assert!(*running_rx.borrow());
        })
        .catch_unwind()
        .await;
        handle.abort();
        let cancelled = handle.await.expect_err("watcher must exit by cancellation");
        assert!(cancelled.is_cancelled());
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
        println!(
            "public_entry payload={} watcher_active=true task_joined=true",
            serde_json::to_string(&written).unwrap()
        );
        assert!(clear_drain_request(temp.path()).unwrap());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        temp.close().unwrap();
        println!("cleanup=closed empty_marker_directory=true");
    }

    #[tokio::test]
    async fn drain_marker_process_helper() {
        let Ok(dir) = std::env::var("U63_DRAIN_DIR") else {
            return;
        };
        let dir = Path::new(&dir);
        match std::env::var("U63_DRAIN_ROLE").unwrap().as_str() {
            "write" => {
                let req = write_drain_request(dir, Some("u63-external-writer"), true).unwrap();
                println!("payload={}", serde_json::to_string(&req).unwrap());
            }
            "read" => {
                let detected = check_drain_requested(dir, current_instantiation_epoch());
                println!("detected={detected:?}");
                let req = detected.expect("same-boot external marker must be active");
                assert_eq!(req.principal.as_deref(), Some("u63-external-writer"));
                assert!(req.suppress_notification);
                let watcher = DrainWatcher::new(dir.to_path_buf(), Duration::from_secs(3));
                let mut rx = watcher.receiver();
                let handle = watcher.spawn();
                let changed = tokio::time::timeout(Duration::from_secs(5), rx.changed()).await;
                handle.abort();
                let joined = handle.await;
                changed
                    .expect("watcher must detect external marker")
                    .unwrap();
                assert!(*rx.borrow());
                assert!(joined.is_ok() || joined.unwrap_err().is_cancelled());
                println!("watcher_active=true");
            }
            role => panic!("unknown helper role: {role}"),
        }
    }

    #[tokio::test]
    async fn drain_marker_cross_process_epoch() {
        let temp = tempfile::tempdir().unwrap();
        for role in ["write", "read"] {
            let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "drain_control::tests::drain_marker_process_helper",
                    "--exact",
                    "--nocapture",
                ])
                .env("U63_DRAIN_DIR", temp.path())
                .env("U63_DRAIN_ROLE", role)
                .kill_on_drop(true);
            let output = tokio::time::timeout(Duration::from_secs(30), command.output())
                .await
                .expect("helper must exit")
                .unwrap();
            println!("{role}: {}", String::from_utf8_lossy(&output.stdout));
            assert!(
                output.status.success(),
                "{role} helper failed: {}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // A known boot mismatch is still rejected; an unknown boot is lenient.
        let path = drain_request_path(temp.path());
        fs::write(&path, r#"{"action":"drain","epoch":"old-boot:1"}"#).unwrap();
        assert!(check_drain_requested(temp.path(), "current-boot:2").is_none());
        assert!(check_drain_requested(temp.path(), "").is_some());
        fs::write(&path, r#"{"action":"restart"}"#).unwrap();
        assert!(check_drain_requested(temp.path(), "").is_none());
        fs::write(&path, "").unwrap();
        assert!(check_drain_requested(temp.path(), "current-boot:2").is_some());
        fs::write(&path, "{malformed").unwrap();
        assert!(check_drain_requested(temp.path(), "current-boot:2").is_some());
        assert!(clear_drain_request(temp.path()).unwrap());
        assert!(!clear_drain_request(temp.path()).unwrap());
        assert!(check_drain_requested(temp.path(), "").is_none());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        temp.close().unwrap();
    }

    #[test]
    fn test_marker_epoch_validation_fresh_vs_stale() {
        let current = "boot-123:pid-456";

        // 1. Same epoch -> valid, not stale
        assert!(!is_marker_epoch_stale(Some("boot-123:pid-456"), current));

        // 2. Different epoch -> stale
        assert!(is_marker_epoch_stale(Some("boot-999:pid-000"), current));

        // 3. None or empty epoch in marker -> not stale (lenient / backward-compatible)
        assert!(!is_marker_epoch_stale(None, current));
        assert!(!is_marker_epoch_stale(Some(""), current));
        assert!(!is_marker_epoch_stale(Some("   "), current));

        // 4. Empty current epoch -> not stale
        assert!(!is_marker_epoch_stale(Some("boot-999:pid-000"), ""));
    }

    #[test]
    fn test_validate_marker_content() {
        let current = "epoch-abc";
        let now = "2026-08-16T12:00:00Z".parse().unwrap();

        // Valid payload with matching epoch
        let json_matching = r#"{"action":"drain","requested_at":"2026-08-16T12:00:00Z","principal":"admin","epoch":"epoch-abc"}"#;
        let validated = validate_marker_at(json_matching, current, now);
        assert!(validated.is_some());
        let req = validated.unwrap();
        assert_eq!(req.action, "drain");
        assert_eq!(req.principal.as_deref(), Some("admin"));
        assert_eq!(req.epoch.as_deref(), Some("epoch-abc"));

        // Payload with mismatched epoch -> rejected as stale (None)
        let json_stale = r#"{"action":"drain","requested_at":"2026-08-16T12:00:00Z","principal":"admin","epoch":"old-epoch-xyz"}"#;
        assert!(validate_marker_at(json_stale, current, now).is_none());

        // Payload with legacy/missing epoch -> accepted
        let json_no_epoch = r#"{"action":"drain","requested_at":"2026-08-16T12:00:00Z"}"#;
        assert!(validate_marker_at(json_no_epoch, current, now).is_some());

        // Action not drain -> rejected
        let json_other_action = r#"{"action":"restart","epoch":"epoch-abc"}"#;
        assert!(validate_marker_at(json_other_action, current, now).is_none());

        // Corrupt JSON -> accepted as fail-safe drain
        let json_corrupt = "{malformed json";
        let fallback = validate_marker_at(json_corrupt, current, now);
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().action, "drain");
    }

    #[test]
    fn test_write_and_clear_drain_request_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();

        assert!(check_drain_requested(dir, current_instantiation_epoch()).is_none());

        let req = write_drain_request(dir, Some("test-op"), true).unwrap();
        assert_eq!(req.action, "drain");
        assert_eq!(req.principal.as_deref(), Some("test-op"));
        assert!(req.suppress_notification);

        let detected = check_drain_requested(dir, current_instantiation_epoch());
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().principal.as_deref(), Some("test-op"));

        let cleared = clear_drain_request(dir).unwrap();
        assert!(cleared);
        assert!(check_drain_requested(dir, current_instantiation_epoch()).is_none());
    }

    #[test]
    fn unreadable_drain_marker_is_treated_as_draining() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        let epoch = current_instantiation_epoch();
        let now = Utc::now();

        // Absent marker stays "not draining".
        assert!(check_drain_requested_at(dir, epoch, now).is_none());

        // Present but unreadable: a directory at the marker path makes every
        // read fail the same way a permissions or transient IO failure does.
        let path = drain_request_path(dir);
        fs::create_dir(&path).unwrap();
        assert!(fs::read_to_string(&path).is_err());

        let detected = check_drain_requested_at(dir, epoch, now);
        println!("unreadable_marker detected={detected:?}");
        let req = detected.expect("unreadable marker must fail CLOSED and read as draining");
        assert_eq!(req.action, "drain");
        assert_eq!(req.principal.as_deref(), Some("unreadable-marker"));

        // The watcher built on the same predicate must publish the drain state.
        let watcher = DrainWatcher::new(dir.to_path_buf(), Duration::from_secs(3));
        let rx = watcher.receiver();
        assert!(watcher.scan_at(epoch, now));
        assert!(*rx.borrow());

        fs::remove_dir(&path).unwrap();
        assert!(check_drain_requested_at(dir, epoch, now).is_none());

        // A real permission denial behaves identically (skipped when running as root).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if unsafe { libc::geteuid() } != 0 {
                fs::write(&path, r#"{"action":"drain"}"#).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
                let denied = check_drain_requested_at(dir, epoch, now);
                println!("permission_denied_marker detected={denied:?}");
                assert!(
                    denied.is_some(),
                    "a marker we cannot read must never report not-draining"
                );
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
    }

    #[test]
    fn failed_marker_commit_leaves_no_temp_file() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();
        // A directory at the marker path makes the final rename fail.
        fs::create_dir(drain_request_path(dir)).unwrap();

        assert!(write_drain_request(dir, Some("op"), false).is_err());

        let leftovers: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files leaked: {leftovers:?}");
    }

    #[test]
    fn clearing_an_absent_marker_is_not_an_error() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!clear_drain_request(temp.path()).unwrap());
        write_drain_request(temp.path(), None, false).unwrap();
        assert!(clear_drain_request(temp.path()).unwrap());
        assert!(!clear_drain_request(temp.path()).unwrap());
    }
}

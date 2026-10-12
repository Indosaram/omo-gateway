use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::storage::{list_lanes, Lane};
use crate::work_heartbeat::Clock;
use crate::{OmonError, Result};

use super::signals::{parse_signals, SignalRunner};

pub const SETTLE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Settler {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
    signals: Arc<dyn SignalRunner>,
    judge: Arc<dyn crate::judge::Judge>,
    serial: Arc<tokio::sync::Mutex<()>>,
}

impl Settler {
    pub fn new(pool: SqlitePool, clock: Arc<dyn Clock>, signals: Arc<dyn SignalRunner>) -> Self {
        Self {
            judge: Arc::new(crate::judge::JevCli::new(
                None,
                Some(pool.clone()),
                None,
                None,
            )),
            pool,
            clock,
            signals,
            serial: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn with_judge(mut self, judge: Arc<dyn crate::judge::Judge>) -> Self {
        self.judge = judge;
        self
    }

    /// Returns newly verified lane keys: todo 15 owns their close lifecycle.
    pub async fn settle_due(&self) -> Result<Vec<String>> {
        let _serial = self.serial.lock().await;
        let now = self.clock.now();
        let mut verified = Vec::new();
        for listed in list_lanes(&self.pool, &["done_claimed"]).await? {
            let mut tx = self.pool.begin().await?;
            // Read the authoritative attempt only after acquiring SQLite's write lock.
            sqlx::query("UPDATE lanes SET updated_at = updated_at WHERE key = ?")
                .bind(&listed.key)
                .execute(&mut *tx)
                .await?;
            let Some(lane) = sqlx::query_as::<_, Lane>(
                "SELECT * FROM lanes WHERE key = ? AND status = 'done_claimed'",
            )
            .bind(&listed.key)
            .fetch_optional(&mut *tx)
            .await?
            else {
                tx.rollback().await?;
                continue;
            };
            let Some(claim_at) = lane.claim_at else {
                tx.rollback().await?;
                continue;
            };
            if claim_at > now - chrono::Duration::minutes(5) {
                tx.rollback().await?;
                continue;
            }
            let generation: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'")
                .bind(&lane.key).fetch_one(&mut *tx).await?;
            let claim_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND to_status = 'done_claimed' AND from_status != 'done_claimed'")
                .bind(&lane.key).fetch_one(&mut *tx).await?;
            let started: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_history WHERE lane_key = ? AND at = ? AND reason = 'settlement_started' AND id > ?")
                .bind(&lane.key).bind(claim_at).bind(generation.max(claim_id)).fetch_one(&mut *tx).await?;
            if started == 0 {
                sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'done_claimed', 'done_claimed', 'settlement_started')")
                    .bind(&lane.key).bind(claim_at).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            // A crash after reservation cannot safely replay arbitrary command signals.
            let mut reasons = Vec::new();
            let mut diff_detail = None;
            if started != 0 {
                reasons.push("settlement interrupted; claim again to recheck".into());
            } else if let Some(remote_reasons) =
                crate::sessions::external::remote_checks(&self.pool, &lane).await?
            {
                reasons.extend(remote_reasons);
            } else {
                match parse_signals(&lane.done_signals) {
                    Ok(signals) => {
                        let results = match crate::cron::remediate::settle_lane(&lane.key).await? {
                            Some(results) => results,
                            None => self.signals.check_all(&signals, Path::new(&lane.cwd)).await,
                        };
                        for result in results {
                            if !result.ok {
                                reasons.push(format!("{}: {}", result.signal, result.detail));
                            }
                        }
                    }
                    Err(error) => reasons.push(error.to_string()),
                }
                match worktree_reasons(&lane).await {
                    Ok(checks) => reasons.extend(checks),
                    Err(error) => reasons.push(error.to_string()),
                }
                if reasons.is_empty() {
                    let (detail, rejected) =
                        done_diff(&self.pool, &lane, self.judge.as_ref()).await?;
                    if rejected {
                        reasons.push(detail.clone());
                    }
                    diff_detail = Some(detail);
                }
            }
            let target = if reasons.is_empty() {
                "verified_done"
            } else {
                "not_done"
            };
            let detail = reasons.join("; ");
            let mut tx = self.pool.begin().await?;
            let changed = sqlx::query("UPDATE lanes SET status = ?, objection = ?, last_turn_outcome = NULL, updated_at = ? WHERE key = ? AND status = 'done_claimed' AND claim_at = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND to_status = 'done_claimed' AND from_status != 'done_claimed') = ?")
                .bind(target).bind(if reasons.is_empty() { None } else { Some(&detail) }).bind(now)
                .bind(&lane.key).bind(claim_at).bind(&lane.key).bind(generation).bind(&lane.key).bind(claim_id).execute(&mut *tx).await?;
            if changed.rows_affected() == 0 {
                tx.rollback().await?;
                continue;
            }
            if let Some(diff_detail) = diff_detail {
                sqlx::query("INSERT INTO lane_history(lane_key,at,from_status,to_status,reason) VALUES(?,?,'done_claimed','done_claimed',?)")
                    .bind(&lane.key).bind(now).bind(diff_detail).execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'done_claimed', ?, ?)")
                .bind(&lane.key).bind(now).bind(target).bind(&detail).execute(&mut *tx).await?;
            if !reasons.is_empty() {
                let body = super::scrub::scrub(&detail);
                sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, 'not_done', ?, ?)")
                    .bind(&lane.key).bind(&lane.lead_session_key).bind(&body).bind(now).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO lane_inbox(lane_key, text, at) VALUES (?, ?, ?)")
                    .bind(&lane.key)
                    .bind(format!(
                        "[LEAD] not done: {detail}; keep working, claim again when fixed"
                    ))
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            if reasons.is_empty() {
                verified.push(lane.key);
            }
        }
        Ok(verified)
    }
}

async fn done_diff(
    pool: &SqlitePool,
    lane: &Lane,
    judge: &dyn crate::judge::Judge,
) -> Result<(String, bool)> {
    let revision: Option<String> =
        sqlx::query_scalar("SELECT start_commit FROM lane_usage_context WHERE lane_key=?")
            .bind(&lane.key)
            .fetch_optional(pool)
            .await?
            .flatten();
    let Some(revision) = revision else {
        tracing::info!(lane = %lane.key, "done_diff skipped: no start commit");
        return Ok(("done_diff skipped: no start commit".into(), false));
    };
    let brief = match tokio::fs::read_to_string(&lane.brief_path).await {
        Ok(brief) => brief,
        Err(error) => {
            tracing::warn!(%error, lane = %lane.key, "done_diff skipped");
            return Ok(("done_diff skipped: brief unavailable".into(), false));
        }
    };
    let prompt = format!("{}\n{}", lane.title, deliverable(&brief));
    let input = match lane_diff_input(Path::new(&lane.cwd), &revision).await {
        Ok(input) => input,
        Err(error) => {
            tracing::warn!(%error, lane = %lane.key, "done_diff skipped");
            return Ok(("done_diff skipped: diff unavailable".into(), false));
        }
    };
    match judge
        .diff(
            crate::judge::Purpose::DoneDiff,
            &prompt,
            &input.0,
            Some(input.1.as_str()),
        )
        .await
    {
        Ok(verdict) => Ok((
            format!(
                "done_diff fulfills={:.3} clean={:.3} risk={:.3}",
                verdict.fulfills, verdict.clean, verdict.risk
            ),
            verdict.fulfills < 0.65 || verdict.risk > 1.5,
        )),
        Err(error) => {
            tracing::warn!(%error, lane = %lane.key, "done_diff skipped");
            Ok(("done_diff skipped: jev unavailable".into(), false))
        }
    }
}

struct DiffInput(PathBuf, String);
impl Drop for DiffInput {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            tracing::warn!(%error, "done_diff input cleanup failed");
        }
    }
}

async fn lane_diff_input(cwd: &Path, revision: &str) -> Result<DiffInput> {
    const MAX_DIFF: usize = 256 * 1024;
    let mut input = DiffInput(
        std::env::temp_dir().join(format!("omon-done-diff-{}", uuid::Uuid::new_v4())),
        String::new(),
    );
    tokio::fs::create_dir(&input.0)
        .await
        .map_err(super::runner::io_error)?;
    let index = input.0.join("lane.index");
    for args in [vec!["read-tree", revision], vec!["add", "-A", "--"]] {
        let output = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::process::Command::new("git")
                .arg("-C")
                .arg(cwd)
                .env("GIT_INDEX_FILE", &index)
                .args(args)
                .kill_on_drop(true)
                .output(),
        );
        let output = output
            .await
            .map_err(|_| OmonError::Config("done_diff index timed out".into()))?
            .map_err(super::runner::io_error)?;
        if !output.status.success() {
            return Err(OmonError::Config("done_diff index failed".into()));
        }
    }
    // Rebuild the lane's changed files at their real paths so the judge sees
    // "done.txt added" rather than an opaque patch file: the base commit holds
    // the start-of-lane contents and the index holds the current contents.
    let listed = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("git")
            .arg("-C")
            .arg(cwd)
            .env("GIT_INDEX_FILE", &index)
            .args(["diff", "--cached", "--name-only", "-z", revision, "--"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| OmonError::Config("done_diff listing timed out".into()))?
    .map_err(super::runner::io_error)?;
    if !listed.status.success() {
        return Err(OmonError::Config("done_diff listing failed".into()));
    }
    let mut budget = MAX_DIFF;
    let mut changes = Vec::new();
    for raw in listed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|p| !p.is_empty())
    {
        let path = PathBuf::from(String::from_utf8_lossy(raw).into_owned());
        if path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            continue;
        }
        let shown = tokio::process::Command::new("git")
            .arg("-C")
            .arg(cwd)
            .arg("show")
            .arg(format!("{revision}:{}", path.display()))
            .kill_on_drop(true)
            .output()
            .await
            .map_err(super::runner::io_error)?;
        let old = shown.status.success().then_some(shown.stdout);
        let new = match tokio::fs::read(cwd.join(&path)).await {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(super::runner::io_error(error)),
        };
        let size = old.as_ref().map_or(0, Vec::len) + new.as_ref().map_or(0, Vec::len);
        if size > budget {
            break;
        }
        budget -= size;
        changes.push((path, old, new));
    }
    git(&input.0, &["init", "--quiet"]).await?;
    for (path, old, _) in &changes {
        write_side(&input.0, path, old.as_deref()).await?;
    }
    git(&input.0, &["add", "-A", "--"]).await?;
    git(
        &input.0,
        &[
            "-c",
            "user.name=omon",
            "-c",
            "user.email=omon@localhost",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "lane start",
        ],
    )
    .await?;
    input.1 = git(&input.0, &["rev-parse", "HEAD"])
        .await?
        .trim()
        .to_owned();
    for (path, _, new) in &changes {
        write_side(&input.0, path, new.as_deref()).await?;
    }
    git(&input.0, &["add", "-A", "--"]).await?;
    Ok(input)
}

/// Keep only what the diff can show. Lane briefs mix the deliverable with
/// operating rules (where to work, no network, commit, report, emit markers);
/// jev scores those unobservable rules as unmet and rejects correct work
/// (measured: same diff 0.39-0.51 with the whole brief, 0.92-0.98 without).
pub fn deliverable(brief: &str) -> String {
    const PROCESS: &[&str] = &[
        "work only",
        "only inside",
        "no network",
        "delegat",
        "service",
        "production",
        "model",
        "commit",
        "emit ",
        "report",
        "claim",
        "do not ",
        "don't ",
        "never ",
        "then end",
        "use supplied",
        "database_url",
        "on path",
        "owned local qa",
        "omo-gateway",
    ];
    // Sentence ends are ". " or a line break; a bare "." inside "done.txt" is not.
    let kept: Vec<&str> = brief
        .lines()
        .flat_map(|line| line.split(". "))
        .map(|sentence| sentence.trim().trim_end_matches('.'))
        .filter(|sentence| !sentence.is_empty())
        .filter(|sentence| {
            let lower = sentence.to_lowercase();
            !PROCESS.iter().any(|word| lower.contains(word))
        })
        .collect();
    if kept.is_empty() {
        brief.trim().to_owned()
    } else {
        kept.join(". ") + "."
    }
}

async fn write_side(root: &Path, path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    let target = root.join(path);
    match bytes {
        Some(bytes) => {
            if let Some(parent) = target.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(super::runner::io_error)?;
            }
            tokio::fs::write(&target, bytes)
                .await
                .map_err(super::runner::io_error)
        }
        None => match tokio::fs::remove_file(&target).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(super::runner::io_error(error)),
        },
    }
}

async fn git(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| OmonError::Config(format!("git check timed out: {}", cwd.display())))?
    .map_err(super::runner::io_error)?;
    if !output.status.success() {
        return Err(OmonError::Config(format!(
            "git check {}: {}",
            cwd.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn worktree_reasons(lane: &Lane) -> Result<Vec<String>> {
    let cwd = tokio::fs::canonicalize(&lane.cwd)
        .await
        .map_err(super::runner::io_error)?;
    // Non-repository lanes have no git worktrees to inspect.
    let probe = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("git")
            .arg("-C")
            .arg(&cwd)
            .args(["rev-parse", "--is-inside-work-tree"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| OmonError::Config(format!("git probe timed out: {}", cwd.display())))?
    .map_err(super::runner::io_error)?;
    if !probe.status.success() {
        if String::from_utf8_lossy(&probe.stderr).contains("not a git repository") {
            return Ok(Vec::new());
        }
        return Err(OmonError::Config(
            String::from_utf8_lossy(&probe.stderr).into_owned(),
        ));
    }
    let mut paths = vec![cwd.clone()];
    for line in git(&cwd, &["worktree", "list", "--porcelain"])
        .await?
        .lines()
    {
        let Some(path) = line.strip_prefix("worktree ") else {
            continue;
        };
        let path = PathBuf::from(path);
        if include_worktree(&path, &cwd, lane.created_at, external_creation_time(&path)).await?
            && !paths.contains(&path)
        {
            paths.push(path);
        }
    }
    let mut reasons = Vec::new();
    for path in paths {
        if !git(&path, &["status", "--porcelain", "--untracked-files=no"])
            .await?
            .trim()
            .is_empty()
        {
            reasons.push(format!("uncommitted {}", path.display()));
        }
        if !git(&path, &["remote"]).await?.trim().is_empty()
            && !git(
                &path,
                &["log", "--branches", "--not", "--remotes", "--oneline"],
            )
            .await?
            .trim()
            .is_empty()
        {
            reasons.push(format!("unpushed {}", path.display()));
        }
    }
    Ok(reasons)
}

async fn include_worktree(
    path: &Path,
    cwd: &Path,
    start: Option<DateTime<Utc>>,
    creation: impl std::future::Future<Output = Result<DateTime<Utc>>>,
) -> Result<bool> {
    if path.starts_with(cwd) {
        return Ok(true);
    }
    let start = start.ok_or_else(|| {
        OmonError::Config(format!(
            "unsupported worktree creation-time check {}: lane start is missing",
            path.display()
        ))
    })?;
    Ok(creation.await? >= start)
}

async fn external_creation_time(path: &Path) -> Result<DateTime<Utc>> {
    let metadata = tokio::fs::metadata(path).await.map_err(|error| {
        OmonError::Config(format!("worktree metadata {}: {error}", path.display()))
    })?;
    match metadata.created() {
        Ok(created) => return Ok(DateTime::<Utc>::from(created)),
        Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {}
        Err(error) => return Err(super::runner::io_error(error)),
    }
    let registration = PathBuf::from(
        git(path, &["rev-parse", "--absolute-git-dir"])
            .await?
            .trim(),
    );
    let back_pointer = registration.join("gitdir");
    match tokio::fs::read_to_string(&back_pointer).await {
        Ok(pointer) => {
            let registered_path = tokio::fs::canonicalize(pointer.trim()).await
                .map_err(super::runner::io_error)?;
            let expected_path = tokio::fs::canonicalize(path.join(".git")).await
                .map_err(super::runner::io_error)?;
            if registered_path != expected_path {
                return Err(OmonError::Config(format!("unsupported worktree creation-time check {}: git registration {} points to {}", path.display(), registration.display(), registered_path.display())));
            }
        }
        Err(error) => return Err(OmonError::Config(format!("unsupported worktree creation-time check {}: no linked-worktree registration creation evidence at {} ({error})", path.display(), back_pointer.display()))),
    }
    let registration_metadata = tokio::fs::metadata(&registration).await.map_err(|error| {
        OmonError::Config(format!(
            "worktree registration metadata {}: {error}",
            registration.display()
        ))
    })?;
    // Git registration records have no creation timestamp; only their birth time is evidence.
    registration_metadata.created().map(DateTime::<Utc>::from).map_err(|error| {
        OmonError::Config(format!(
            "unsupported worktree creation-time check {}: path birth time unavailable; git registration {} provides no creation timestamp ({error})",
            path.display(), registration.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn deliverable_drops_operating_rules_and_keeps_the_change() {
        let brief = "Owned local QA. Work only inside /tmp/x. No network, delegation, source messaging, services, or production files. Use supplied DATABASE_URL and the omo-gateway on PATH. Do not change models. Create done.txt containing QA15_READY on your lane branch. Commit exactly done.txt with a short message. Write no other files. Then emit QA15_READY and report done. Do not remove worktrees or delete branches.";
        assert_eq!(
            super::deliverable(brief),
            "Create done.txt containing QA15_READY on your lane branch. Write no other files."
        );
        assert_eq!(
            super::deliverable("Fix the bug in src/a.rs.\nAdd a test in tests/a.rs."),
            "Fix the bug in src/a.rs. Add a test in tests/a.rs."
        );
        assert_eq!(super::deliverable("Commit it."), "Commit it.");
    }

    use super::*;

    #[tokio::test]
    async fn cwd_and_descendants_do_not_query_creation_metadata() {
        for path in [Path::new("/repo"), Path::new("/repo/child")] {
            assert!(include_worktree(path, Path::new("/repo"), None, async {
                panic!("creation metadata must not be queried for included paths")
            })
            .await
            .unwrap());
        }
    }

    #[tokio::test]
    async fn external_selection_uses_creation_time_not_modification_time() {
        let start = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        for (created, included) in [
            (start - chrono::Duration::seconds(1), false),
            (start, true),
            (start + chrono::Duration::seconds(1), true),
        ] {
            assert_eq!(
                include_worktree(
                    Path::new("/external"),
                    Path::new("/repo"),
                    Some(start),
                    async { Ok(created) }
                )
                .await
                .unwrap(),
                included
            );
        }
        assert!(include_worktree(
            Path::new("/external"),
            Path::new("/repo"),
            Some(start),
            async { Err(OmonError::Config("unsupported creation-time check".into())) }
        )
        .await
        .is_err());
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::net::UnixDatagram;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::agent::{AgentBackend, OmoBackend, OmoBackendConfig};
use crate::judge::{JevCli, Judge};
use crate::multiplexer::OutboundDispatcher;
use crate::storage::{get_lane, list_lanes, read_transcript, Lane};
use crate::work_heartbeat::SystemClock;
use crate::{InboundEvent, OmonError, OutboundAction, Result, SessionContext, SessionKey};

use super::activity::{LaneActivity, StreamCapture};
use super::posts::{self, PostSink};

tokio::task_local! {
    static CAPTURE_GENERATION: i64;
}

pub const DEFAULT_MAX_TURNS: i64 = 24;

/// A working lane quiet this long gets one progress nudge per silence.
pub const NUDGE_AFTER: chrono::Duration = chrono::Duration::minutes(15);
pub const NUDGE_TEXT: &str = "[LEAD] 진행 상황을 [REPORT] 한 줄로 보고하고 계속하세요";

/// OMON_LANE_CONTROL_ENABLED (default true), read once per process.
pub fn lane_control_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("OMON_LANE_CONTROL_ENABLED").map_or(true, |value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
    })
}

/// OMON_LANE_AUTO_WORKTREE (default true), read once per process.
pub fn auto_worktree_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("OMON_LANE_AUTO_WORKTREE").map_or(true, |value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
    })
}

async fn paused(pool: &SqlitePool, key: &str) -> Result<bool> {
    if !lane_control_enabled() {
        return Ok(false);
    }
    crate::storage::lanes::is_lane_paused(pool, key).await
}

async fn reopen_generation(pool: &SqlitePool, key: &str) -> Result<i64> {
    let (generation,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'",
    )
    .bind(key)
    .fetch_one(pool)
    .await?;
    Ok(generation)
}

async fn reopen_marker(pool: &SqlitePool, key: &str) -> Result<Option<(i64, String)>> {
    Ok(sqlx::query_as("SELECT id, at FROM lane_history WHERE lane_key = ? AND reason = 'reopened' ORDER BY id DESC LIMIT 1")
        .bind(key).fetch_optional(pool).await?)
}

pub async fn reopen(pool: &SqlitePool, key: &str, text: Option<&str>) -> Result<()> {
    let lane = get_lane(pool, key)
        .await?
        .ok_or_else(|| OmonError::Config("lane not found".into()))?;
    if !matches!(lane.status.as_str(), "closed" | "failed") {
        return Err(OmonError::Config(format!(
            "IllegalTransition: {} -> reopened",
            lane.status
        )));
    }
    restore_cwd(pool, &lane).await?;
    if let Some(thread) = &lane.thread_channel_id {
        let transport = transport_for(thread)?;
        thread_effect(pool, &lane, Some(&transport), false, LIFECYCLE_TIMEOUT).await?;
    }
    apply_reopen(pool, key, text, None).await
}

async fn apply_reopen(
    pool: &SqlitePool,
    key: &str,
    text: Option<&str>,
    receipt: Option<&str>,
) -> Result<()> {
    // Read-then-write across processes: take the write lock up front so a concurrent
    // commit cannot turn the first UPDATE into an immediate SQLITE_BUSY_SNAPSHOT.
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let (from,): (String,) = sqlx::query_as("SELECT status FROM lanes WHERE key = ?")
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OmonError::Config(format!("lane not found: {key}")))?;
    if !matches!(from.as_str(), "closed" | "failed") {
        return Err(OmonError::Config(format!(
            "IllegalTransition: {from} -> reopened"
        )));
    }
    let now = Utc::now();
    let changed = sqlx::query("UPDATE lanes SET status = 'working', turns_run = 0, last_turn_outcome = NULL, claim_evidence = NULL, claim_at = NULL, objection = NULL, updated_at = ? WHERE key = ? AND status = ?")
        .bind(now)
        .bind(key)
        .bind(&from)
        .execute(&mut *tx)
        .await?;
    if changed.rows_affected() != 1 {
        return Err(OmonError::Config("concurrent reopen".into()));
    }
    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, ?, 'working', 'reopened')")
        .bind(key).bind(now).bind(from).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM lane_inbox WHERE lane_key = ? AND consumed_at IS NULL")
        .bind(key)
        .execute(&mut *tx)
        .await?;
    // A pause is a throttle for the run it was set in, not a property of the lane: close
    // does not clear it, so a reopen must, or the reopened lane is skipped forever.
    sqlx::query("UPDATE lane_pauses SET resumed_at = ? WHERE lane_key = ? AND resumed_at IS NULL")
        .bind(now)
        .bind(key)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO lane_inbox(lane_key, text, at) VALUES (?, ?, ?)")
        .bind(key)
        .bind(text.unwrap_or("The lane was reopened; re-check state and continue"))
        .bind(now)
        .execute(&mut *tx)
        .await?;
    if let Some(receipt) = receipt {
        let applied = sqlx::query("UPDATE lane_owner_replies SET state='applied',error=NULL,reopen_history_id=(SELECT MAX(id) FROM lane_history WHERE lane_key=? AND reason='reopened') WHERE message_id=? AND state='pending' AND generation=(SELECT COALESCE(MAX(id),0) FROM lane_history WHERE lane_key=? AND reason='reopened' AND id<(SELECT MAX(id) FROM lane_history WHERE lane_key=? AND reason='reopened'))")
            .bind(key).bind(receipt).bind(key).bind(key).execute(&mut *tx).await?;
        if applied.rows_affected() != 1 {
            return Err(OmonError::Config("stale owner reply".into()));
        }
    }
    tx.commit().await?;
    Ok(())
}

/// One transport for the platform of `thread` (a Slack ref or a Discord channel id).
fn transport_for(thread: &str) -> Result<super::slack_threads::LaneTransports> {
    let mut transports = super::slack_threads::LaneTransports::default();
    if super::slack_threads::is_slack_ref(thread) {
        transports.slack = Some(Arc::new(
            super::slack_threads::SlackLaneThreads::from_env()
                .ok_or_else(|| OmonError::Config("Slack is not configured".into()))?,
        ));
    } else {
        transports.discord = Some(Arc::new(
            super::discord_threads::DiscordThreads::from_env()
                .map_err(|error| OmonError::Config(error.to_string()))?,
        ));
    }
    Ok(transports)
}

#[async_trait]
pub trait LaneLifecycle: Send + Sync {
    async fn bot_id(
        &self,
    ) -> std::result::Result<String, super::discord_threads::DiscordThreadError>;
    /// Bot identity on the platform that owns `thread`.
    async fn bot_id_for(
        &self,
        _thread: &str,
    ) -> std::result::Result<String, super::discord_threads::DiscordThreadError> {
        self.bot_id().await
    }
    async fn status(
        &self,
        thread: &str,
        status: super::discord_threads::ThreadStatus,
    ) -> std::result::Result<(), super::discord_threads::DiscordThreadError>;
    async fn archived(
        &self,
        thread: &str,
        archived: bool,
    ) -> std::result::Result<(), super::discord_threads::DiscordThreadError>;
}

fn discord_thread(
    thread: &str,
) -> std::result::Result<serenity::all::ChannelId, super::discord_threads::DiscordThreadError> {
    thread
        .parse::<u64>()
        .ok()
        .filter(|id| *id != 0)
        .map(serenity::all::ChannelId::new)
        .ok_or_else(|| OmonError::Config("invalid lane thread".into()).into())
}

#[async_trait]
impl LaneLifecycle for super::discord_threads::DiscordThreads {
    async fn bot_id(
        &self,
    ) -> std::result::Result<String, super::discord_threads::DiscordThreadError> {
        Ok(self.http().get_current_user().await?.id.to_string())
    }
    async fn status(
        &self,
        thread: &str,
        status: super::discord_threads::ThreadStatus,
    ) -> std::result::Result<(), super::discord_threads::DiscordThreadError> {
        self.set_status(discord_thread(thread)?, status)
            .await
            .map(|_| ())
    }
    async fn archived(
        &self,
        thread: &str,
        archived: bool,
    ) -> std::result::Result<(), super::discord_threads::DiscordThreadError> {
        let thread = discord_thread(thread)?;
        if archived {
            self.archive(thread).await
        } else {
            self.unarchive(thread).await
        }
        .map(|_| ())
    }
}

async fn thread_effect(
    pool: &SqlitePool,
    lane: &Lane,
    transport: Option<&dyn LaneLifecycle>,
    close: bool,
    timeout: Duration,
) -> Result<()> {
    let Some(thread) = &lane.thread_channel_id else {
        return Ok(());
    };
    if let Ok(id) = thread.parse::<u64>() {
        if let Some(bot) = crate::storage::Database::get_thread_owner(pool, id).await? {
            crate::storage::lanes::bind_lane_thread(pool, &lane.key, &bot.to_string()).await?;
        }
    }
    let bound: Option<String> =
        sqlx::query_scalar("SELECT bot_id FROM lane_thread_bindings WHERE lane_key=?")
            .bind(&lane.key)
            .fetch_optional(pool)
            .await?;
    let bound =
        bound.ok_or_else(|| OmonError::Config("lane has no evidenced bot binding".into()))?;
    if !super::slack_threads::valid_thread_ref(thread) {
        return Err(OmonError::Config("invalid lane thread".into()));
    }
    let transport = transport
        .ok_or_else(|| OmonError::Config("lane lifecycle transport unavailable".into()))?;
    if tokio::time::timeout(timeout, transport.bot_id_for(thread))
        .await
        .map_err(|_| OmonError::Config("lane lifecycle bot lookup timed out".into()))?
        .map_err(|error| OmonError::Config(error.to_string()))?
        != bound
    {
        return Err(OmonError::Config("lane lifecycle bot mismatch".into()));
    }
    let status = if close {
        super::discord_threads::ThreadStatus::Done
    } else {
        super::discord_threads::ThreadStatus::Working
    };
    // Discord allows two thread renames per ten minutes and holds the request
    // until the window resets. The title marker is cosmetic, so a held rename is
    // skipped instead of stalling close/reopen and the lane loop behind it.
    match tokio::time::timeout(timeout, transport.status(thread, status)).await {
        Ok(Ok(())) => {}
        // On reopen the thread may still be archived, where Discord can refuse the
        // rename; the title marker is cosmetic and the status refresh repairs it.
        Ok(Err(error)) if !close => {
            tracing::warn!(lane = %lane.key, %error, "lane thread rename failed on reopen; skipped")
        }
        Ok(Err(error)) => return Err(OmonError::Config(error.to_string())),
        Err(_) => tracing::warn!(lane = %lane.key, "lane thread rename held by Discord; skipped"),
    }
    tokio::time::timeout(timeout, transport.archived(thread, close))
        .await
        .map_err(|_| OmonError::Config("lane thread archive timed out".into()))?
        .map_err(|error| OmonError::Config(error.to_string()))
}

async fn git(repo: &str, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(io_error)?;
    if !output.status.success() {
        return Err(OmonError::Config(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

async fn worktree_proof(repo: &str, path: &str) -> Result<(String, String, String, String)> {
    use std::os::unix::fs::MetadataExt;
    let list = git(repo, &["worktree", "list", "--porcelain"]).await?;
    let records: Vec<&str> = list.split("\n\n").collect();
    let record = records
        .iter()
        .skip(1)
        .find(|record| record.lines().next() == Some(format!("worktree {path}").as_str()))
        .ok_or_else(|| OmonError::Config("path is not a registered linked worktree".into()))?;
    let branch = record
        .lines()
        .find_map(|line| line.strip_prefix("branch refs/heads/"))
        .ok_or_else(|| {
            OmonError::Config("linked worktree requires an existing local branch".into())
        })?
        .to_owned();
    let common = git(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    let control_common = git(
        repo,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    let common = tokio::fs::canonicalize(common).await.map_err(io_error)?;
    if common
        != tokio::fs::canonicalize(control_common)
            .await
            .map_err(io_error)?
    {
        return Err(OmonError::Config(
            "worktree common repository mismatch".into(),
        ));
    }
    let git_dir = tokio::fs::canonicalize(git(path, &["rev-parse", "--absolute-git-dir"]).await?)
        .await
        .map_err(io_error)?;
    let meta = tokio::fs::metadata(&git_dir).await.map_err(io_error)?;
    let marker = tokio::fs::read(path.to_owned() + "/.git")
        .await
        .map_err(io_error)?;
    let identity = format!("{}:{}:{:x}", meta.dev(), meta.ino(), Sha256::digest(marker));
    if git(path, &["symbolic-ref", "--short", "HEAD"]).await? != branch {
        return Err(OmonError::Config("worktree branch mismatch".into()));
    }
    Ok((
        common.to_string_lossy().into(),
        git_dir.to_string_lossy().into(),
        identity,
        branch,
    ))
}

pub async fn register_owned_worktree(
    pool: &SqlitePool,
    key: &str,
    repo: &std::path::Path,
    path: &std::path::Path,
) -> Result<()> {
    let lane = get_lane(pool, key)
        .await?
        .ok_or_else(|| OmonError::Config("lane not found".into()))?;
    let repo = tokio::fs::canonicalize(repo).await.map_err(io_error)?;
    let path = tokio::fs::canonicalize(path).await.map_err(io_error)?;
    if repo.starts_with(&path) {
        return Err(OmonError::Config(
            "control repository must survive removal".into(),
        ));
    }
    let repo = repo.to_string_lossy().to_string();
    let path = path.to_string_lossy().to_string();
    let (common, dir, identity, branch) = worktree_proof(&repo, &path).await?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let targets: Vec<(String, String)> =
        sqlx::query_as("SELECT path,repo FROM lane_owned_worktrees")
            .fetch_all(&mut *tx)
            .await?;
    if targets.iter().any(|(target, control)| {
        std::path::Path::new(&repo).starts_with(target)
            || std::path::Path::new(control).starts_with(&path)
    }) {
        return Err(OmonError::Config(
            "control repository intersects registered removal targets".into(),
        ));
    }
    let pending: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_close_intents WHERE lane_key=? AND state IN ('pending','running'))").bind(key).fetch_one(&mut *tx).await?;
    if pending || lane.status == "closed" {
        return Err(OmonError::Config(
            "cannot register during close or while closed".into(),
        ));
    }
    sqlx::query("INSERT INTO lane_owned_worktrees(lane_key,path,role,repo,common_dir,git_dir,identity,branch,registrar) VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(key).bind(&path).bind(if lane.cwd==path {"cwd"} else {"auxiliary"}).bind(repo).bind(common).bind(dir).bind(identity).bind(branch)
        .bind(format!("local-cli:{}",std::env::var("USER").unwrap_or_default())).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// A worktree `lane start` created for one lane.
#[derive(Debug, Clone)]
pub struct AutoWorktree {
    pub repo: PathBuf,
    pub path: PathBuf,
    pub branch: String,
}

/// Creates `<repo-parent>/<repo-name>-lanes/<key>` on a new `lane/<key>` branch from HEAD
/// when `cwd` (canonical) is the root of a main git worktree, so parallel lanes on one
/// repository never share a tree. Returns None, and logs why, for any other cwd.
pub async fn create_auto_worktree(
    cwd: &std::path::Path,
    key: &str,
) -> Result<Option<AutoWorktree>> {
    if !auto_worktree_enabled() {
        return Ok(None);
    }
    let probe = tokio::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--absolute-git-dir",
            "--git-common-dir",
        ])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(io_error)?;
    if !probe.status.success() {
        let stderr = String::from_utf8_lossy(&probe.stderr);
        if stderr.contains("not a git repository") {
            tracing::info!(key, cwd = %cwd.display(), "auto worktree skipped: not a git repository");
            return Ok(None);
        }
        return Err(OmonError::Config(format!(
            "auto worktree probe {}: {}",
            cwd.display(),
            stderr.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&probe.stdout).into_owned();
    let mut lines = stdout.lines();
    let (Some(top), Some(git_dir), Some(common)) = (lines.next(), lines.next(), lines.next())
    else {
        return Err(OmonError::Config(
            "auto worktree probe: unexpected output".into(),
        ));
    };
    let top = tokio::fs::canonicalize(top).await.map_err(io_error)?;
    let git_dir = tokio::fs::canonicalize(git_dir).await.map_err(io_error)?;
    let common = tokio::fs::canonicalize(common).await.map_err(io_error)?;
    if git_dir != common {
        tracing::info!(key, cwd = %cwd.display(), "auto worktree skipped: cwd is already a linked worktree");
        return Ok(None);
    }
    if top != cwd {
        tracing::info!(key, cwd = %cwd.display(), "auto worktree skipped: cwd is not the repository root");
        return Ok(None);
    }
    let repo = top.to_string_lossy().into_owned();
    if git(&repo, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .await
        .is_err()
    {
        tracing::info!(key, cwd = %cwd.display(), "auto worktree skipped: repository has no commits");
        return Ok(None);
    }
    let (Some(name), Some(parent)) = (top.file_name(), top.parent()) else {
        return Err(OmonError::Config(
            "auto worktree: repository root has no parent".into(),
        ));
    };
    let lanes_dir = parent.join(format!("{}-lanes", name.to_string_lossy()));
    tokio::fs::create_dir_all(&lanes_dir)
        .await
        .map_err(io_error)?;
    let path = lanes_dir.join(key);
    let branch = format!("lane/{key}");
    let target = path.to_string_lossy().into_owned();
    git(&repo, &["worktree", "add", "-b", &branch, &target, "HEAD"]).await?;
    let path = tokio::fs::canonicalize(&path).await.map_err(io_error)?;
    tracing::info!(key, path = %path.display(), "auto worktree created");
    Ok(Some(AutoWorktree {
        repo: top,
        path,
        branch,
    }))
}

/// Undoes `create_auto_worktree` when the lane row was never stored. The tree is fresh
/// and clean and its branch sits at HEAD, so neither command needs a force flag. A failure
/// is returned rather than logged away: a tree that survives this is one nothing else
/// will ever clean, so the caller has to be able to say what was left behind.
pub async fn discard_auto_worktree(tree: &AutoWorktree) -> Result<()> {
    let repo = tree.repo.to_string_lossy();
    git(&repo, &["worktree", "remove", &tree.path.to_string_lossy()]).await?;
    git(&repo, &["branch", "-d", &tree.branch]).await?;
    Ok(())
}

/// The lane branch still holds commits the lane's own base lacks. Close removes the
/// worktree and keeps the branch, so the owner must be told where the work went; nothing
/// here merges it. The base is the commit the lane started from, not the control
/// repository's moving HEAD, so commits the owner made in the meantime are never reported
/// as lane work and a diverged HEAD cannot inflate the count.
pub(crate) async fn unmerged_lane_branch(
    pool: &SqlitePool,
    lane_key: &str,
) -> Result<Option<String>> {
    let trees: Vec<(String, String)> = sqlx::query_as(
        "SELECT repo, branch FROM lane_owned_worktrees WHERE lane_key=? ORDER BY id",
    )
    .bind(lane_key)
    .fetch_all(pool)
    .await?;
    let start: Option<String> =
        sqlx::query_scalar("SELECT start_commit FROM lane_usage_context WHERE lane_key=?")
            .bind(lane_key)
            .fetch_optional(pool)
            .await?
            .flatten();
    for (repo, branch) in trees {
        let base = match &start {
            // The recorded base is only usable while the branch still descends from it.
            Some(commit)
                if git(&repo, &["merge-base", "--is-ancestor", commit, &branch])
                    .await
                    .is_ok() =>
            {
                Some(commit.clone())
            }
            _ => None,
        };
        let reference = base.as_deref().unwrap_or("HEAD");
        let ahead = match git(
            &repo,
            &["rev-list", "--count", &format!("{reference}..{branch}")],
        )
        .await
        {
            Ok(ahead) => ahead,
            Err(error) => {
                tracing::info!(lane = lane_key, %branch, %error, "lane branch comparison skipped");
                continue;
            }
        };
        if ahead != "0" {
            let against = match &base {
                Some(commit) => format!("base {}", commit.chars().take(12).collect::<String>()),
                None => "the repository HEAD".to_string(),
            };
            return Ok(Some(format!("{branch} ({ahead} commits not in {against})")));
        }
    }
    Ok(None)
}

/// Why a worktree must survive the sweep: local changes, or commits no remote has.
async fn keep_reason(path: &str) -> Result<Option<&'static str>> {
    if !git(path, &["status", "--porcelain"]).await?.is_empty() {
        return Ok(Some("dirty"));
    }
    if !git(path, &["remote"]).await?.is_empty()
        && !git(path, &["log", "HEAD", "--not", "--remotes", "--oneline"])
            .await?
            .is_empty()
    {
        return Ok(Some("unpushed"));
    }
    Ok(None)
}

async fn validate_owned(worktree: &crate::storage::lanes::OwnedWorktree) -> Result<()> {
    let proof = worktree_proof(&worktree.repo, &worktree.path).await?;
    if proof
        != (
            worktree.common_dir.clone(),
            worktree.git_dir.clone(),
            worktree.identity.clone(),
            worktree.branch.clone(),
        )
    {
        return Err(OmonError::Config(
            "registered worktree identity changed".into(),
        ));
    }
    Ok(())
}

async fn remove_owned(
    pool: &SqlitePool,
    intent: &crate::storage::lanes::CloseIntent,
) -> Result<()> {
    let trees: Vec<crate::storage::lanes::OwnedWorktree> =
        sqlx::query_as("SELECT * FROM lane_owned_worktrees WHERE lane_key=? ORDER BY id")
            .bind(&intent.lane_key)
            .fetch_all(pool)
            .await?;
    if trees.is_empty() {
        sqlx::query(
            "UPDATE lane_close_intents SET removal='skipped: unowned external cwd' WHERE id=?",
        )
        .bind(intent.id)
        .execute(pool)
        .await?;
    }
    for tree in trees {
        if tree.state == "removed" {
            continue;
        }
        let common = git(
            &tree.repo,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .await?;
        if tokio::fs::canonicalize(common)
            .await
            .map_err(io_error)?
            .to_string_lossy()
            != tree.common_dir
        {
            return Err(OmonError::Config(
                "removal control repository identity changed".into(),
            ));
        }
        let exists = tokio::fs::try_exists(&tree.path).await.map_err(io_error)?;
        let list = git(&tree.repo, &["worktree", "list", "--porcelain"]).await?;
        let registered = list
            .lines()
            .any(|line| line == format!("worktree {}", tree.path));
        if !exists
            && !registered
            && tree.state == "removing"
            && tree.removal_intent == Some(intent.id)
        {
            if tokio::fs::symlink_metadata(&tree.path).await.is_ok()
                || tokio::fs::try_exists(&tree.git_dir)
                    .await
                    .map_err(io_error)?
            {
                return Err(OmonError::Config(
                    "ambiguous missing worktree registration".into(),
                ));
            }
            git(
                &tree.repo,
                &[
                    "show-ref",
                    "--verify",
                    &format!("refs/heads/{}", tree.branch),
                ],
            )
            .await?;
        } else {
            validate_owned(&tree).await?;
            sqlx::query("UPDATE lane_owned_worktrees SET state='removing',removal_intent=?,removal_identity=identity WHERE id=? AND state IN ('registered','removing')")
                .bind(intent.id).bind(tree.id).execute(pool).await?;
            git(&tree.repo, &["worktree", "remove", &tree.path]).await?;
            git(
                &tree.repo,
                &[
                    "show-ref",
                    "--verify",
                    &format!("refs/heads/{}", tree.branch),
                ],
            )
            .await?;
            if tokio::fs::try_exists(&tree.path).await.map_err(io_error)?
                || git(&tree.repo, &["worktree", "list", "--porcelain"])
                    .await?
                    .lines()
                    .any(|line| line == format!("worktree {}", tree.path))
            {
                return Err(OmonError::Config("worktree removal not observed".into()));
            }
        }
        sqlx::query("UPDATE lane_owned_worktrees SET state='removed',removal_receipt=? WHERE id=? AND state='removing' AND removal_intent=?")
            .bind(json!({"intent":intent.id,"identity":tree.identity,"path":tree.path,"branch":tree.branch,"verified":true}).to_string()).bind(tree.id).bind(intent.id).execute(pool).await?;
    }
    Ok(())
}

async fn restore_cwd(pool: &SqlitePool, lane: &Lane) -> Result<()> {
    let tree: Option<crate::storage::lanes::OwnedWorktree> =
        sqlx::query_as("SELECT * FROM lane_owned_worktrees WHERE lane_key=? AND role='cwd'")
            .bind(&lane.key)
            .fetch_optional(pool)
            .await?;
    let Some(tree) = tree else {
        if !tokio::fs::metadata(&lane.cwd)
            .await
            .map_err(io_error)?
            .is_dir()
        {
            return Err(OmonError::Config("lane cwd is not a directory".into()));
        }
        return Ok(());
    };
    if tree.path != lane.cwd {
        return Err(OmonError::Config("previous cwd capability mismatch".into()));
    }
    let mut tree = tree;
    if tree.state == "registered" {
        return validate_owned(&tree).await;
    }
    if tree.state == "restoring" {
        // A crash between 'restoring' and 'registered' must not strand the lane.
        if finish_interrupted_restore(pool, &tree).await? {
            return Ok(());
        }
        tree.state = "removed".into();
    }
    let completed: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_close_intents WHERE id=? AND state='completed' AND previous_cwd=?)").bind(tree.removal_intent).bind(&tree.path).fetch_one(pool).await?;
    let receipt = tree
        .removal_receipt
        .as_deref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
    if !completed
        || tree.state != "removed"
        || !receipt.is_some_and(|value| {
            value["verified"] == true
                && value["identity"] == tree.identity
                && value["branch"] == tree.branch
                && value["path"] == tree.path
                && value["intent"].as_i64() == tree.removal_intent
        })
    {
        return Err(OmonError::Config("cwd restoration requires verified removal receipt; ambiguous restoration needs explicit reconciliation".into()));
    }
    if tokio::fs::symlink_metadata(&tree.path).await.is_ok() {
        return Err(OmonError::Config("restoration path is occupied".into()));
    }
    let common = git(
        &tree.repo,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    if tokio::fs::canonicalize(common)
        .await
        .map_err(io_error)?
        .to_string_lossy()
        != tree.common_dir
    {
        return Err(OmonError::Config(
            "restoration control repository identity changed".into(),
        ));
    }
    git(
        &tree.repo,
        &[
            "show-ref",
            "--verify",
            &format!("refs/heads/{}", tree.branch),
        ],
    )
    .await?;
    let changed = sqlx::query(
        "UPDATE lane_owned_worktrees SET state='restoring' WHERE id=? AND state='removed'",
    )
    .bind(tree.id)
    .execute(pool)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(OmonError::Config("concurrent cwd restoration".into()));
    }
    if let Err(error) = git(&tree.repo, &["worktree", "add", &tree.path, &tree.branch]).await {
        sqlx::query(
            "UPDATE lane_owned_worktrees SET state='removed' WHERE id=? AND state='restoring'",
        )
        .bind(tree.id)
        .execute(pool)
        .await?;
        return Err(error);
    }
    let (common, dir, identity, branch) = worktree_proof(&tree.repo, &tree.path).await?;
    if common != tree.common_dir || branch != tree.branch {
        return Err(OmonError::Config("restored worktree mismatch".into()));
    }
    sqlx::query("UPDATE lane_owned_worktrees SET state='registered',git_dir=?,identity=?,restoration_receipt=? WHERE id=? AND state='restoring'")
        .bind(dir).bind(&identity).bind(json!({"identity":identity,"removal":tree.removal_receipt,"path":tree.path,"branch":branch}).to_string()).bind(tree.id).execute(pool).await?;
    Ok(())
}

/// True when the interrupted restoration had in fact produced the owned worktree
/// and is now registered; false when nothing was restored and the state was rolled
/// back to 'removed' so the restoration can run again.
async fn finish_interrupted_restore(
    pool: &SqlitePool,
    tree: &crate::storage::lanes::OwnedWorktree,
) -> Result<bool> {
    match worktree_proof(&tree.repo, &tree.path).await {
        Ok((common, dir, identity, branch))
            if common == tree.common_dir && branch == tree.branch =>
        {
            sqlx::query("UPDATE lane_owned_worktrees SET state='registered',git_dir=?,identity=?,restoration_receipt=? WHERE id=? AND state='restoring'")
                .bind(dir).bind(&identity).bind(json!({"identity":identity,"removal":tree.removal_receipt,"path":tree.path,"branch":branch,"recovered":true}).to_string()).bind(tree.id).execute(pool).await?;
            Ok(true)
        }
        _ => {
            if tokio::fs::symlink_metadata(&tree.path).await.is_ok() {
                return Err(OmonError::Config(
                    "interrupted restoration left an unverifiable path; needs explicit reconciliation"
                        .into(),
                ));
            }
            sqlx::query(
                "UPDATE lane_owned_worktrees SET state='removed' WHERE id=? AND state='restoring'",
            )
            .bind(tree.id)
            .execute(pool)
            .await?;
            Ok(false)
        }
    }
}

pub(super) fn io_error(error: std::io::Error) -> OmonError {
    OmonError::Config(format!("lane I/O: {error}"))
}

pub fn lane_key(key: &str) -> SessionKey {
    SessionKey::new(
        "lane",
        None::<String>,
        key,
        None::<String>,
        format!("lane:{key}"),
    )
}

/// Captures lane output only; this dispatcher has no Discord transport.
pub struct CapturingDispatcher {
    pool: SqlitePool,
    streams: tokio::sync::Mutex<HashMap<(String, uuid::Uuid), StreamCapture>>,
    posts_ready: Arc<Notify>,
    approval_dispatcher: Option<Arc<dyn OutboundDispatcher>>,
}

impl CapturingDispatcher {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            streams: tokio::sync::Mutex::new(HashMap::new()),
            posts_ready: Arc::new(Notify::new()),
            approval_dispatcher: None,
        }
    }

    pub fn with_approval_dispatcher(mut self, dispatcher: Arc<dyn OutboundDispatcher>) -> Self {
        self.approval_dispatcher = Some(dispatcher);
        self
    }
}

#[async_trait]
impl OutboundDispatcher for CapturingDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        let message = match action {
            OutboundAction::SendMessage {
                session, content, ..
            } => Some((session, content, uuid::Uuid::new_v4().to_string())),
            OutboundAction::EditMessage {
                session,
                content,
                platform_message_id,
            } => Some((session, content, platform_message_id)),
            OutboundAction::Stream { session, chunk } => {
                if session.platform != "lane" {
                    return Err(OmonError::Config("non-lane stream sent to capture".into()));
                }
                let generation = match CAPTURE_GENERATION.try_with(|generation| *generation) {
                    Ok(generation) => generation,
                    Err(_) => reopen_generation(&self.pool, &session.channel_id).await?,
                };
                LaneActivity::new(self.pool.clone(), Arc::new(SystemClock))
                    .captured_generation(&session.channel_id, "", Some(generation))
                    .await?;
                let mut streams = self.streams.lock().await;
                let stream_key = (session.channel_id.clone(), chunk.stream_id);
                let content = streams
                    .entry(stream_key.clone())
                    .or_default()
                    .accept(&chunk)
                    .map(str::to_owned);
                if chunk.is_final {
                    streams.remove(&stream_key);
                }
                content.map(|content| (session, content, format!("stream:{}", chunk.stream_id)))
            }
            OutboundAction::UploadFile { session, path } => Some((
                session,
                format!("[file] {}", path.display()),
                uuid::Uuid::new_v4().to_string(),
            )),
            OutboundAction::ApprovalRequest {
                session,
                request_id,
                command,
                reason,
            } => {
                let lead_key = if session.platform == "lane" {
                    sqlx::query_scalar::<_, String>(
                        "SELECT lead_session_key FROM lanes WHERE key = ?",
                    )
                    .bind(&session.channel_id)
                    .fetch_optional(&self.pool)
                    .await?
                    .unwrap_or_else(|| super::kibitzer::DEFAULT_LEAD.to_string())
                } else {
                    session.storage_key()
                };
                let owner_session =
                    SessionKey::from_storage_key(&lead_key).unwrap_or_else(|_| session.clone());
                if let Some(dispatcher) = &self.approval_dispatcher {
                    dispatcher
                        .dispatch(OutboundAction::ApprovalRequest {
                            session: owner_session,
                            request_id,
                            command,
                            reason,
                        })
                        .await?;
                    return Ok(());
                }
                let body = format!(
                    "[APPROVAL_REQUEST] lane={} request_id={} command={} reason={}",
                    session.channel_id, request_id, command, reason
                );
                sqlx::query(
                    "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at)
                     VALUES (?, ?, 'action_loop', ?, ?)",
                )
                .bind(if session.platform == "lane" {
                    Some(session.channel_id.as_str())
                } else {
                    None
                })
                .bind(&lead_key)
                .bind(&body)
                .bind(Utc::now())
                .execute(&self.pool)
                .await?;
                return Ok(());
            }
            OutboundAction::DeleteMessage { .. }
            | OutboundAction::Typing { .. }
            | OutboundAction::React { .. }
            | OutboundAction::ExpireApproval { .. } => None,
        };
        if let Some((session, content, identity)) = message {
            if session.platform != "lane" {
                return Err(OmonError::Config(
                    "non-lane output sent to lane capture".into(),
                ));
            }
            let generation = match CAPTURE_GENERATION.try_with(|generation| *generation) {
                Ok(generation) => generation,
                Err(_) => reopen_generation(&self.pool, &session.channel_id).await?,
            };
            let activity = LaneActivity::new(self.pool.clone(), Arc::new(SystemClock));
            let decision = activity
                .captured_item(&session.channel_id, &content, generation, &identity)
                .await?;
            // A replayed item was already recorded; a second row would be re-read as new output.
            if !decision.replayed {
                sqlx::query("INSERT INTO lane_transcript(lane_key, at, role, text) SELECT ?, ?, 'agent', ? WHERE (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                    .bind(&session.channel_id).bind(Utc::now()).bind(&content).bind(&session.channel_id).bind(generation).execute(&self.pool).await?;
            }
            self.posts_ready.notify_one();
        }
        Ok(())
    }
}

/// The socket is only a wake hint. The database remains authoritative across restarts.
async fn socket_path(pool: &SqlitePool) -> Result<PathBuf> {
    let databases: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list")
        .fetch_all(pool)
        .await?;
    let path = databases
        .into_iter()
        .find(|(_, name, _)| name == "main")
        .map(|(_, _, path)| path)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| OmonError::Config("lane service requires a file-backed database".into()))?;
    let path = tokio::fs::canonicalize(path).await.map_err(io_error)?;
    let digest = format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()));
    Ok(PathBuf::from("/tmp").join(format!("omon-lanes-{}.sock", &digest[..24])))
}

pub async fn wake(pool: &SqlitePool) -> Result<()> {
    let path = socket_path(pool).await?;
    let socket = UnixDatagram::unbound().map_err(io_error)?;
    match socket.send_to(b"wake", path).await {
        Ok(_) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(io_error(error)),
    }
}

struct WakeSocket {
    socket: UnixDatagram,
    path: PathBuf,
    _lock: std::fs::File,
}

impl WakeSocket {
    async fn bind(pool: &SqlitePool) -> Result<Self> {
        let path = socket_path(pool).await?;
        let lock_path = path.with_extension("lock");
        let lock = tokio::task::spawn_blocking(move || -> Result<std::fs::File> {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lock_path)
                .map_err(io_error)?;
            fs2::FileExt::try_lock_exclusive(&file).map_err(|error| {
                OmonError::Config(format!("lane runner already owns this database: {error}"))
            })?;
            Ok(file)
        })
        .await
        .map_err(|error| OmonError::Config(error.to_string()))??;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
        let socket = UnixDatagram::bind(&path).map_err(io_error)?;
        Ok(Self {
            socket,
            path,
            _lock: lock,
        })
    }
}

impl Drop for WakeSocket {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            tracing::warn!(%error, "remove lane wake socket");
        }
    }
}

#[derive(Clone)]
pub struct LaneRunner {
    pool: SqlitePool,
    backend: Arc<dyn AgentBackend>,
    max_turns: i64,
    keywords: String,
    judge: Arc<dyn Judge>,
    post_sink: Option<Arc<dyn PostSink>>,
    posts_ready: Arc<Notify>,
    capture: Option<Arc<CapturingDispatcher>>,
    settler: super::settle::Settler,
    kibitzer_hint: Option<Arc<Notify>>,
    lifecycle: Option<Arc<dyn LaneLifecycle>>,
    lifecycle_timeout: Duration,
}

/// Bound on each Discord lane-thread call (bot lookup, rename, archive).
pub const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(15);

impl LaneRunner {
    pub fn new(pool: SqlitePool, backend: Arc<dyn AgentBackend>) -> Self {
        Self {
            judge: Arc::new(JevCli::new(None, Some(pool.clone()), None, None)),
            settler: super::settle::Settler::new(
                pool.clone(),
                Arc::new(SystemClock),
                Arc::new(super::signals::SystemSignalRunner::new()),
            ),
            pool,
            backend,
            max_turns: DEFAULT_MAX_TURNS,
            keywords: "ulw set goal and work".into(),
            post_sink: None,
            posts_ready: Arc::new(Notify::new()),
            capture: None,
            kibitzer_hint: None,
            lifecycle: None,
            lifecycle_timeout: LIFECYCLE_TIMEOUT,
        }
    }

    pub fn with_kibitzer_hint(mut self, hint: Arc<Notify>) -> Self {
        self.kibitzer_hint = Some(hint);
        self
    }

    pub fn with_lifecycle(mut self, lifecycle: Arc<dyn LaneLifecycle>) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    pub fn with_lifecycle_timeout(mut self, timeout: Duration) -> Self {
        self.lifecycle_timeout = timeout;
        self
    }

    pub async fn close_intent(&self, id: i64) -> Result<()> {
        let claimed = sqlx::query(
            "UPDATE lane_close_intents SET state='running' WHERE id=? AND state='pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() != 1 {
            return Ok(());
        }
        let intent: crate::storage::lanes::CloseIntent =
            sqlx::query_as("SELECT * FROM lane_close_intents WHERE id=?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        let result: Result<()>=async {
            let lane=self.lane(&intent.lane_key).await?;
            if reopen_generation(&self.pool,&lane.key).await?!=intent.generation || lane.status!=intent.original_status || (!intent.force && lane.status!="verified_done") {
                return Err(OmonError::Config("stale close intent".into()));
            }
            if intent.phase==0 {
                thread_effect(&self.pool,&lane,self.lifecycle.as_deref(),true,self.lifecycle_timeout).await?;
                sqlx::query("UPDATE lane_close_intents SET phase=1 WHERE id=? AND state='running'").bind(id).execute(&self.pool).await?;
            }
            remove_owned(&self.pool,&intent).await?;
            // The worktree is gone and its branch stays behind; compute the gap before the
            // write lock so the close transaction never waits on git.
            let unmerged = unmerged_lane_branch(&self.pool, &lane.key).await?;
            let mut tx=self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let changed=sqlx::query("UPDATE lanes SET status='closed',updated_at=? WHERE key=? AND status=? AND (SELECT COALESCE(MAX(id),0) FROM lane_history WHERE lane_key=? AND reason='reopened')=?")
                .bind(Utc::now()).bind(&lane.key).bind(&intent.original_status).bind(&lane.key).bind(intent.generation).execute(&mut *tx).await?;
            if changed.rows_affected()!=1 { return Err(OmonError::Config("stale close completion".into())); }
            let mut status=intent.original_status.parse::<super::state::LaneStatus>().map_err(|error| OmonError::Config(error.to_string()))?;
            loop {
                let to=match status {
                    super::state::LaneStatus::DoneClaimed=>super::state::LaneStatus::NotDone,
                    super::state::LaneStatus::NotDone|super::state::LaneStatus::Blocked|super::state::LaneStatus::Failed|super::state::LaneStatus::Queued=>super::state::LaneStatus::Working,
                    super::state::LaneStatus::Working|super::state::LaneStatus::VerifiedDone=>super::state::LaneStatus::Closed,
                    super::state::LaneStatus::Closed=>break,
                };
                if !status.can_transition_to(to,intent.note.as_deref()) { return Err(OmonError::Config("illegal close transition".into())); }
                sqlx::query("INSERT INTO lane_history(lane_key,at,from_status,to_status,reason) VALUES (?,?,?,?,?)")
                    .bind(&lane.key).bind(Utc::now()).bind(status.as_str()).bind(to.as_str()).bind(intent.note.as_deref().map(super::state::safe_reason)).execute(&mut *tx).await?;
                status=to;
            }
            if let Some(note) = unmerged {
                // Surfaced, never merged: the owner decides whether to merge or push.
                let body = super::scrub::scrub(&format!(
                    "lane work is on unmerged branch {note}; nothing was merged into the repository HEAD"
                ));
                let reported: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_events WHERE lane_key=? AND kind='unmerged_branch' AND body=?)")
                    .bind(&lane.key).bind(&body).fetch_one(&mut *tx).await?;
                if !reported {
                    crate::storage::lanes::enqueue_lane_event_with_executor(
                        &mut *tx,
                        Some(&lane.key),
                        &lane.lead_session_key,
                        "unmerged_branch",
                        &body,
                    )
                    .await?;
                }
            }
            sqlx::query("UPDATE lane_close_intents SET state='completed',phase=2,error=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=? AND state='running'").bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
            Ok(())
        }.await;
        if let Err(error) = &result {
            let lane = self.lane(&intent.lane_key).await?;
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let changed=sqlx::query("UPDATE lane_close_intents SET state='failed',error=?,updated_at=CURRENT_TIMESTAMP WHERE id=? AND state='running'").bind(error.to_string()).bind(id).execute(&mut *tx).await?;
            if changed.rows_affected() == 1 {
                crate::storage::lanes::enqueue_lane_event_with_executor(
                    &mut *tx,
                    Some(&lane.key),
                    &lane.lead_session_key,
                    "close_failed",
                    &super::scrub::scrub(&format!("attempt {}: {error}", intent.attempt)),
                )
                .await?;
            }
            tx.commit().await?;
        }
        result
    }

    /// Startup sweep: an owned worktree still registered to a closed or failed lane is
    /// removed through the close path (a failed lane takes a forced close, a closed one
    /// replays its completed close intent). A dirty or unpushed worktree is kept and
    /// reported once as a `worktree_kept` lane event.
    pub async fn sweep_orphan_worktrees(&self) -> Result<()> {
        if !auto_worktree_enabled() {
            return Ok(());
        }
        let lanes: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT DISTINCT l.key, l.status, l.lead_session_key FROM lanes l
             JOIN lane_owned_worktrees w ON w.lane_key = l.key
             WHERE w.state = 'registered' AND l.status IN ('closed','failed') ORDER BY l.key",
        )
        .fetch_all(&self.pool)
        .await?;
        for (key, status, lead) in lanes {
            let result: Result<()> = async {
                let paths: Vec<String> = sqlx::query_scalar(
                    "SELECT path FROM lane_owned_worktrees WHERE lane_key=? AND state='registered' ORDER BY id",
                )
                .bind(&key)
                .fetch_all(&self.pool)
                .await?;
                let mut kept = Vec::new();
                for path in &paths {
                    if let Some(reason) = keep_reason(path).await? {
                        kept.push(format!("{reason} {path}"));
                    }
                }
                if !kept.is_empty() {
                    let body = super::scrub::scrub(&format!(
                        "worktree kept for {status} lane {key}: {}",
                        kept.join("; ")
                    ));
                    let reported: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM lane_events WHERE lane_key=? AND kind='worktree_kept' AND body=?)",
                    )
                    .bind(&key)
                    .bind(&body)
                    .fetch_one(&self.pool)
                    .await?;
                    if !reported {
                        crate::storage::lanes::enqueue_lane_event(
                            &self.pool,
                            Some(&key),
                            &lead,
                            "worktree_kept",
                            &body,
                        )
                        .await?;
                    }
                    return Ok(());
                }
                if status == "failed" {
                    let id = crate::storage::lanes::request_close(
                        &self.pool,
                        &key,
                        true,
                        Some("forced: startup sweep of orphan lane worktree"),
                    )
                    .await?;
                    return self.close_intent(id).await;
                }
                let intent: Option<crate::storage::lanes::CloseIntent> = sqlx::query_as(
                    "SELECT * FROM lane_close_intents WHERE lane_key=? AND state='completed' ORDER BY id DESC LIMIT 1",
                )
                .bind(&key)
                .fetch_optional(&self.pool)
                .await?;
                match intent {
                    Some(intent) => remove_owned(&self.pool, &intent).await,
                    None => Err(OmonError::Config(
                        "closed lane has no completed close intent".into(),
                    )),
                }
            }
            .await;
            if let Err(error) = result {
                tracing::warn!(%key, %error, "orphan lane worktree sweep failed");
            }
        }
        Ok(())
    }

    pub async fn apply_owner_replies(&self) -> Result<()> {
        let replies: Vec<(String,String,Option<String>,i64)>=sqlx::query_as("SELECT message_id,lane_key,text,generation FROM lane_owner_replies WHERE state='pending' AND error IS NULL ORDER BY rowid").fetch_all(&self.pool).await?;
        for (id, key, text, generation) in replies {
            let lane = self.lane(&key).await?;
            let current = reopen_generation(&self.pool, &key).await?;
            let result: Result<()>=async {
                if lane.status=="closed" || lane.status=="failed" {
                    if current!=generation { return Err(OmonError::Config("stale reopen reply".into())); }
                    restore_cwd(&self.pool,&lane).await?;
                    thread_effect(&self.pool,&lane,self.lifecycle.as_deref(),false,self.lifecycle_timeout).await?;
                    apply_reopen(&self.pool,&key,text.as_deref(),Some(&id)).await?;
                } else if lane.status=="working" && current!=generation {
                    let mut tx=self.pool.begin_with("BEGIN IMMEDIATE").await?;
                    let previous: Option<i64>=sqlx::query_scalar("SELECT reopen_history_id FROM lane_owner_replies WHERE lane_key=? AND generation=? AND state='applied' AND reopen_history_id=? LIMIT 1").bind(&key).bind(generation).bind(current).fetch_optional(&mut *tx).await?;
                    if previous.is_none() { return Err(OmonError::Config("reply generation mismatch".into())); }
                    let applied=sqlx::query("UPDATE lane_owner_replies SET state='applied',error=NULL,reopen_history_id=? WHERE message_id=? AND state='pending'").bind(current).bind(&id).execute(&mut *tx).await?;
                    if applied.rows_affected()==1 {
                        sqlx::query("INSERT INTO lane_inbox(lane_key,text,at) VALUES (?,?,?)").bind(&key).bind(text.as_deref().unwrap_or("The lane was reopened; re-check state and continue")).bind(Utc::now()).execute(&mut *tx).await?;
                    }
                    tx.commit().await?;
                }
                Ok(())
            }.await;
            if let Err(error) = result {
                let parked = sqlx::query(
                    "UPDATE lane_owner_replies SET error=? WHERE message_id=? AND state='pending'",
                )
                .bind(error.to_string())
                .bind(&id)
                .execute(&self.pool)
                .await?;
                tracing::warn!(%key,%error,"owner reply remains pending");
                // A parked reply is retried only by a new owner reply: tell the lead once.
                if parked.rows_affected() == 1 {
                    crate::storage::lanes::enqueue_lane_event_with_executor(
                        &self.pool,
                        Some(&key),
                        &lane.lead_session_key,
                        "owner_reply_failed",
                        &super::scrub::scrub(&format!(
                            "owner reply to {key} could not be applied: {error}"
                        ))
                        .chars()
                        .take(2000)
                        .collect::<String>(),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    pub fn with_judge(mut self, judge: Arc<dyn Judge>) -> Self {
        self.judge = judge;
        self
    }

    pub fn with_settlement(
        mut self,
        clock: Arc<dyn crate::work_heartbeat::Clock>,
        signals: Arc<dyn super::signals::SignalRunner>,
    ) -> Self {
        self.settler = super::settle::Settler::new(self.pool.clone(), clock, signals);
        self
    }

    pub async fn settle_due(&self) -> Result<Vec<String>> {
        self.settler.settle_due().await
    }

    pub fn with_post_sink(mut self, sink: Arc<dyn PostSink>) -> Self {
        self.post_sink = Some(sink);
        self
    }

    pub fn from_env(pool: SqlitePool) -> Result<Self> {
        Self::from_env_with_requester(pool, None)
    }

    /// Lane turns raise approval cards in the lead's conversation through the shared requester,
    /// so a lane action that needs the owner is asked instead of silently declined.
    pub fn from_env_with_requester(
        pool: SqlitePool,
        requester: Option<Arc<crate::DiscordApprovalRequester>>,
    ) -> Result<Self> {
        let config = OmoBackendConfig::lane_from_env()?;
        if config.total_timeout.is_zero() {
            return Err(OmonError::Config(
                "OMON_LANE_TURN_SECS must be positive".into(),
            ));
        }
        let capture = Arc::new(CapturingDispatcher::new(pool.clone()));
        let mut backend = OmoBackend::new(config, capture.clone()).with_pool(pool.clone());
        if let Some(requester) = requester {
            backend = backend.with_approval_requester(requester);
        }
        let mut runner = Self::new(pool, Arc::new(backend));
        runner.posts_ready = capture.posts_ready.clone();
        runner.capture = Some(capture);
        let discord_enabled = std::env::var("OMON_DISCORD_ENABLED")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(true);
        let mut transports = super::slack_threads::LaneTransports {
            discord: None,
            slack: super::slack_threads::SlackLaneThreads::from_env().map(Arc::new),
        };
        if let Some(transport) = super::discord_threads::DiscordThreads::loopback_from_env()? {
            transports.discord = Some(Arc::new(transport));
        } else if discord_enabled {
            match super::discord_threads::DiscordThreads::from_env() {
                Ok(transport) => transports.discord = Some(Arc::new(transport)),
                // A Slack-only gateway has no Discord token; lanes then live on Slack.
                Err(super::discord_threads::DiscordThreadError::MissingToken)
                    if transports.slack.is_some() =>
                {
                    tracing::info!("no Discord token; lane threads use Slack only");
                }
                Err(error) => return Err(OmonError::Config(error.to_string())),
            }
        }
        if transports.discord.is_some() || transports.slack.is_some() {
            let transports = Arc::new(transports);
            runner.post_sink = Some(transports.clone());
            runner.lifecycle = Some(transports);
        }
        if let Ok(value) = std::env::var("OMON_LANE_MAX_TURNS") {
            runner.max_turns = value
                .parse()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| OmonError::Config("OMON_LANE_MAX_TURNS must be positive".into()))?;
        }
        if let Ok(keywords) = std::env::var("OMON_LANE_KEYWORDS") {
            runner.keywords = keywords;
        }
        Ok(runner)
    }

    pub async fn serve(self, shutdown: CancellationToken) -> Result<()> {
        let socket = WakeSocket::bind(&self.pool).await?;
        sqlx::query("UPDATE lane_close_intents SET state='pending' WHERE state='running'")
            .execute(&self.pool)
            .await?;
        if let Err(error) = self.sweep_orphan_worktrees().await {
            tracing::warn!(%error, "orphan lane worktree sweep failed");
        }
        posts::recover(&self.pool).await?;
        if let Some(sink) = &self.post_sink {
            posts::drain_with_status(&self.pool, sink.as_ref(), Utc::now()).await?;
            posts::refresh_statuses(&self.pool, sink.as_ref(), Utc::now()).await?;
        }
        tracing::info!(socket = %socket.path.display(), "gateway lane service ready");
        let mut tasks = JoinSet::new();
        let mut settlements = JoinSet::new();
        let mut active: HashMap<String, (Arc<Notify>, CancellationToken)> = HashMap::new();
        let mut bytes = [0; 64];
        let mut settle_tick = tokio::time::interval(super::settle::SETTLE_INTERVAL);
        settle_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut guard_tick = tokio::time::interval(super::guard::GUARD_INTERVAL);
        guard_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut guard_watcher = super::guard::GuardWatcher::default();
        let guard_lead = Some(crate::runtime_config::default_lead());
        let service_result: Result<()> = async {
        loop {
            if shutdown.is_cancelled() {
                break;
            }
            for lane in list_lanes(&self.pool,&["verified_done"]).await? {
                let exists: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_close_intents WHERE lane_key=? AND generation=?)")
                    .bind(&lane.key).bind(reopen_generation(&self.pool,&lane.key).await?).fetch_one(&self.pool).await?;
                if !exists { crate::storage::lanes::request_close(&self.pool,&lane.key,false,None).await?; }
            }
            let pending: Vec<(i64,String)>=sqlx::query_as("SELECT id,lane_key FROM lane_close_intents WHERE state='pending' ORDER BY id").fetch_all(&self.pool).await?;
            for (id,key) in pending {
                if let Some((notify,cancel))=active.get(&key) { cancel.cancel(); notify.notify_one(); }
                else if let Err(error)=self.close_intent(id).await { tracing::warn!(%key,%error,"lane close failed"); }
            }
            self.apply_owner_replies().await?;
            if let Some(sink) = &self.post_sink {
                posts::refresh_statuses(&self.pool, sink.as_ref(), Utc::now()).await?;
            }
            for lane in list_lanes(&self.pool, &["working", "not_done"]).await? {
                let released: bool = sqlx::query_scalar("SELECT kind='external' AND external_target IS NULL FROM lanes WHERE key=?")
                    .bind(&lane.key).fetch_one(&self.pool).await?;
                if released { continue; }
                let closing: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_close_intents WHERE lane_key=? AND state IN ('pending','running'))").bind(&lane.key).fetch_one(&self.pool).await?;
                if closing { continue; }
                if paused(&self.pool, &lane.key).await? {
                    if let Some((notify, _)) = active.get(&lane.key) { notify.notify_one(); }
                    continue;
                }
                if !active.contains_key(&lane.key) {
                    let notify = Arc::new(Notify::new());
                    let runner = self.clone();
                    let cancellation = shutdown.child_token();
                    active.insert(lane.key.clone(), (notify.clone(), cancellation.clone()));
                    let generation = reopen_generation(&self.pool, &lane.key).await?;
                    tasks.spawn(async move {
                        let result = runner.run_lane(&lane.key, notify, cancellation).await;
                        (lane.key, generation, result)
                    });
                }
            }
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = guard_tick.tick() => {
                    match super::guard::sample(&self.pool).await {
                        Ok(host) => {
                            if let Some(lead) = &guard_lead {
                                if let Err(error) = guard_watcher.enqueue_transition(&self.pool, lead, &host).await {
                                    tracing::warn!(%error, "guard transition remains pending");
                                }
                            }
                            if let Err(error) = super::guard::retry_queued(&self.pool, &host, guard_watcher.thresholds()).await {
                                tracing::warn!(%error, "queued lane retry failed");
                            }
                        }
                        Err(error) => tracing::warn!(%error, "host guard sample failed"),
                    }
                }
                _ = settle_tick.tick(), if settlements.is_empty() => {
                    let settler = self.settler.clone();
                    settlements.spawn(async move { settler.settle_due().await });
                }
                Some(result) = settlements.join_next(), if !settlements.is_empty() => {
                    match result {
                        Ok(Ok(verified)) => {
                            self.verified_done_ready(&verified);
                            for (notify,_) in active.values() { notify.notify_one(); }
                        }
                        Ok(Err(error)) => {
                            return Err(error);
                        }
                        Err(error) => {
                            return Err(OmonError::Multiplexer(error.to_string()));
                        }
                    }
                }
                _ = self.posts_ready.notified() => {
                    if let Some(sink) = &self.post_sink { posts::drain_with_status(&self.pool, sink.as_ref(), Utc::now()).await?; }
                }
                result = socket.socket.recv(&mut bytes) => {
                    result.map_err(io_error)?;
                    for (notify,_) in active.values() { notify.notify_one(); }
                    if let Some(hint) = &self.kibitzer_hint { hint.notify_one(); }
                }
                Some(result) = tasks.join_next(), if !tasks.is_empty() => {
                    let (key, generation, outcome) = result.map_err(|error| OmonError::Multiplexer(error.to_string()))?;
                    active.remove(&key);
                    if let Err(error) = outcome {
                        tracing::error!(%key, %error, "lane task failed");
                        self.fail(&key, "dead", &error.to_string(), generation).await?;
                    }
                }
            }
        }
        Ok(())
        }.await;
        let mut first_error = service_result.err();
        shutdown.cancel();
        settlements.abort_all();
        while let Some(result) = settlements.join_next().await {
            match result {
                Ok(Err(error)) => {
                    first_error.get_or_insert(error);
                }
                Err(error) if !error.is_cancelled() => {
                    first_error.get_or_insert_with(|| OmonError::Multiplexer(error.to_string()));
                }
                _ => {}
            }
        }
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((key, _, Err(error))) => tracing::warn!(%key, %error, "lane shutdown outcome"),
                Err(error) => {
                    first_error.get_or_insert_with(|| OmonError::Multiplexer(error.to_string()));
                }
                _ => {}
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Todo 15 attaches close scheduling here; verification never closes a lane itself.
    fn verified_done_ready(&self, keys: &[String]) {
        for key in keys {
            tracing::info!(%key, "verified lane ready for close lifecycle");
        }
    }

    pub async fn run_lane(
        &self,
        key: &str,
        notify: Arc<Notify>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let initial = self.lane(key).await?;
        if paused(&self.pool, key).await? {
            return Ok(());
        }
        if crate::sessions::external::ExternalDriver::<
            crate::sessions::registry::ProcessCommandRunner,
        >::is_external(&self.pool, key)
        .await?
        {
            return crate::sessions::external::ExternalDriver::from_environment(self.pool.clone())?
                .drive(key, notify, shutdown)
                .await;
        }
        let mut session = SessionContext::new(lane_key(key));
        session
            .state
            .metadata
            .insert("lane_cwd".into(), json!(initial.cwd));
        let stored: Option<(String,)> =
            sqlx::query_as("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(session.key.storage_key())
                .fetch_optional(&self.pool)
                .await?;
        let stored_id = stored
            .map(|(state,)| serde_json::from_str::<serde_json::Value>(&state))
            .transpose()
            .map_err(|error| OmonError::Config(error.to_string()))?
            .and_then(|state| {
                state["metadata"]["omo_thread_id"]
                    .as_str()
                    .map(str::to_owned)
            });
        if let Some(id) = stored_id.or(initial.omo_thread_id) {
            session
                .state
                .metadata
                .insert("omo_thread_id".into(), json!(id));
        }
        let mut generation = reopen_generation(&self.pool, key).await?;
        let mut was_reopened = generation != 0;
        let mut reopen_at = reopen_marker(&self.pool, key).await?.map(|(_, at)| at);
        let mut resume_fallback = false;
        let mut errors = 0;
        let activity = LaneActivity::new(self.pool.clone(), Arc::new(SystemClock));
        loop {
            let current_generation = reopen_generation(&self.pool, key).await?;
            if current_generation != generation {
                generation = current_generation;
                was_reopened = generation != 0;
                reopen_at = reopen_marker(&self.pool, key).await?.map(|(_, at)| at);
                resume_fallback = false;
                errors = 0;
            }
            let lane = self.lane(key).await?;
            let closing: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_close_intents WHERE lane_key=? AND state IN ('pending','running'))").bind(key).fetch_one(&self.pool).await?;
            // A paused lane ends its task; the service respawns it after `lane resume` wakes it.
            if shutdown.is_cancelled()
                || closing
                || !matches!(lane.status.as_str(), "working" | "not_done")
                || paused(&self.pool, key).await?
            {
                return Ok(());
            }
            if lane.status == "not_done" {
                let mut tx = self.pool.begin().await?;
                let changed = sqlx::query("UPDATE lanes SET status = 'working', updated_at = ? WHERE key = ? AND status = 'not_done' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                    .bind(Utc::now()).bind(key).bind(key).bind(generation).execute(&mut *tx).await?;
                if changed.rows_affected() == 1 {
                    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'not_done', 'working', 'resuming work')")
                        .bind(key).bind(Utc::now()).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            if lane.turns_run >= self.max_turns && !(was_reopened && lane.turns_run == 0) {
                return self
                    .fail(key, "turn_cap", "lane turn cap reached", generation)
                    .await;
            }
            if errors >= 3 {
                return self
                    .fail(key, "dead", "three consecutive turn errors", generation)
                    .await;
            }
            if lane.model.as_deref()
                == Some(&crate::agent::model_for_purpose(
                    crate::agent::ModelPurpose::LaneSmall,
                ))
            {
                let (failed_checks,): (i64,) = sqlx::query_as("SELECT count(*) FROM lane_history WHERE lane_key = ? AND to_status = 'not_done' AND COALESCE(reason, '') NOT IN ('paused', 'resumed')")
                    .bind(key).fetch_one(&self.pool).await?;
                if failed_checks >= 2 {
                    let mut tx = self.pool.begin().await?;
                    let changed = sqlx::query("UPDATE lanes SET model = ? WHERE key = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                        .bind(crate::agent::model_for_purpose(
                            crate::agent::ModelPurpose::LaneLarge,
                        ))
                        .bind(key)
                        .bind(key).bind(generation)
                        .execute(&mut *tx)
                        .await?;
                    if changed.rows_affected() == 0 {
                        tx.rollback().await?;
                        continue;
                    }
                    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'working', 'working', 'escalated')")
                        .bind(key).bind(Utc::now()).execute(&mut *tx).await?;
                    tx.commit().await?;
                }
            }
            session.state.active_model = self.lane(key).await?.model;
            if lane.model.as_deref() != session.state.active_model.as_deref() {
                sqlx::query("UPDATE lane_usage_context SET purpose='lane_large' WHERE lane_key=?")
                    .bind(key)
                    .execute(&self.pool)
                    .await?;
            }
            let inbox = self.pending(key).await?;
            if inbox.is_empty() && lane.last_turn_outcome.as_deref() == Some("idle_without_claim") {
                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = notify.notified() => continue,
                }
            }
            let prompt = if lane.turns_run == 0 && was_reopened && !resume_fallback {
                inbox
                    .iter()
                    .map(|(_, text)| text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else if lane.turns_run == 0 || resume_fallback {
                let brief = tokio::fs::read_to_string(&lane.brief_path)
                    .await
                    .map_err(io_error)?;
                let databases: Vec<(i64, String, String)> = sqlx::query_as("PRAGMA database_list")
                    .fetch_all(&self.pool)
                    .await?;
                let database = databases
                    .into_iter()
                    .find(|(_, name, _)| name == "main")
                    .map(|(_, _, path)| path)
                    .unwrap_or_default();
                let executable = std::env::current_exe().map_err(io_error)?;
                format!("{}\n\n{}{}{}\n\n{}\nControl executable (one argv element): {}\nControl DATABASE_URL: {}\nUse this DATABASE_URL explicitly for every lane command; do not use another gateway database.\n{}", self.keywords, brief, if was_reopened { "\n\nReopened lane; continue with prior context." } else { "" }, if resume_fallback { "\n\nResumed lane; previous thread unavailable" } else { "" }, brief_footer(&lane), json!(executable), json!(format!("sqlite://{database}")), inbox.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>().join("\n"))
            } else if !inbox.is_empty() {
                inbox
                    .iter()
                    .map(|(_, text)| text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                "Continue the lane. Report progress with [REPORT] lines; when every Done signal holds, run the claim command.".into()
            };
            sqlx::query("INSERT INTO lane_transcript(lane_key, at, role, text) SELECT ?, ?, 'user', ? WHERE (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                .bind(key).bind(Utc::now()).bind(&prompt).bind(key).bind(generation).execute(&self.pool).await?;
            sqlx::query("UPDATE lanes SET last_turn_outcome = NULL WHERE key = ? AND last_turn_outcome = 'idle_without_claim' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                .bind(key).bind(key).bind(generation).execute(&self.pool).await?;
            let event = InboundEvent::message(
                session.key.clone(),
                uuid::Uuid::new_v4().to_string(),
                prompt,
            );
            let steering_session = session.clone();
            let turn_cancel = shutdown.child_token();
            let (transcript_start,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(MAX(id), 0) FROM lane_transcript WHERE lane_key = ?",
            )
            .bind(key)
            .fetch_one(&self.pool)
            .await?;
            // The 20-minute quiet window counts from this turn's start, not from the end of a
            // previous turn that may have finished long ago (blocked, idle, or gateway down).
            sqlx::query(
                "UPDATE lanes SET last_activity_at = ?, stale_posted_at = NULL WHERE key = ? AND status = 'working'",
            )
            .bind(chrono::Utc::now())
            .bind(key)
            .execute(&self.pool)
            .await?;
            // tokio intervals tick immediately; the first quiet check waits one period.
            let mut quiet_check = tokio::time::interval_at(
                tokio::time::Instant::now() + std::time::Duration::from_secs(60),
                std::time::Duration::from_secs(60),
            );
            quiet_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut turn_tasks = JoinSet::new();
            let backend = self.backend.clone();
            let cancellation = turn_cancel.clone();
            let _cancel_on_exit = turn_cancel.clone().drop_guard();
            let turn_task = turn_tasks.spawn(async move {
                CAPTURE_GENERATION
                    .scope(generation, async move {
                        let outcome = backend
                            .run_cancelable(&mut session, event, cancellation)
                            .await;
                        (session, outcome)
                    })
                    .await
            });
            let outcome = {
                loop {
                    tokio::select! {
                        biased;
                        _ = shutdown.cancelled(), if !turn_cancel.is_cancelled() => {
                            turn_cancel.cancel();
                        }
                        result = turn_tasks.join_next() => {
                            let (returned_session, outcome) = result
                                .ok_or_else(|| OmonError::Multiplexer("lane turn task missing".into()))?
                                .map_err(|error| OmonError::Multiplexer(format!("lane turn task: {error}")))?;
                            session = returned_session;
                            break outcome;
                        },
                        _ = quiet_check.tick() => {
                            if reopen_generation(&self.pool, key).await? != generation {
                                turn_cancel.cancel();
                                continue;
                            }
                            activity.quiet_generation(key, true, Some(generation)).await?;
                            if reopen_generation(&self.pool, key).await? != generation {
                                turn_cancel.cancel();
                                continue;
                            }
                            self.posts_ready.notify_one();
                            if !turn_task.is_finished() {
                                self.nudge_quiet(key, &steering_session, generation).await?;
                            }
                        }
                        _ = notify.notified(), if !turn_cancel.is_cancelled() => {
                            if paused(&self.pool, key).await? {
                                turn_cancel.cancel();
                                continue;
                            }
                            let current = self.lane(key).await?;
                            let current_generation = reopen_generation(&self.pool, key).await?;
                            if current_generation != generation || !matches!(current.status.as_str(), "working" | "not_done" | "done_claimed") {
                                turn_cancel.cancel();
                                continue;
                            }
                            for (id, text) in self.pending(key).await? {
                                if inbox.iter().any(|(sent, _)| *sent == id) { continue; }
                                if reopen_generation(&self.pool, key).await? != generation {
                                    turn_cancel.cancel();
                                    break;
                                }
                                if turn_task.is_finished() || turn_cancel.is_cancelled() { break; }
                                match self.backend.steer(&steering_session, &text).await {
                                    Ok(true) if reopen_generation(&self.pool, key).await? == generation => {
                                        sqlx::query("UPDATE lane_inbox SET consumed_at = ? WHERE id = ? AND consumed_at IS NULL AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                                            .bind(Utc::now()).bind(id).bind(key).bind(generation).execute(&self.pool).await?;
                                    },
                                    Ok(true) => break,
                                    Ok(false) => {},
                                    Err(error) => tracing::warn!(%key, %error, "steer deferred to next turn"),
                                }
                            }
                        }
                    }
                }
            };
            let current_generation = reopen_generation(&self.pool, key).await?;
            if let Some(capture) = &self.capture {
                capture
                    .streams
                    .lock()
                    .await
                    .retain(|(lane, _), _| lane != key);
            }
            if current_generation != generation {
                generation = current_generation;
                was_reopened = generation != 0;
                reopen_at = reopen_marker(&self.pool, key).await?.map(|(_, at)| at);
                resume_fallback = false;
                errors = 0;
                continue;
            }
            if reopen_at.as_deref()
                != reopen_marker(&self.pool, key)
                    .await?
                    .as_ref()
                    .map(|(_, at)| at.as_str())
            {
                generation = reopen_generation(&self.pool, key).await?;
                was_reopened = true;
                reopen_at = reopen_marker(&self.pool, key).await?.map(|(_, at)| at);
                resume_fallback = false;
                errors = 0;
                continue;
            }
            let (resume_generation,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'")
                .bind(key).fetch_one(&self.pool).await?;
            if !resume_fallback
                && was_reopened
                && resume_generation == generation
                && outcome
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.to_string().contains("continuity_error"))
            {
                if self.clear_failed_resume(key, generation).await? {
                    session.state.metadata.remove("omo_thread_id");
                    resume_fallback = true;
                }
                continue;
            }
            let id = session
                .state
                .metadata
                .get("omo_thread_id")
                .and_then(|value| value.as_str());
            let paused_now = paused(&self.pool, key).await?;
            let outcome_text = match &outcome {
                Ok(()) => "completed".into(),
                Err(_) if paused_now => "paused".into(),
                Err(error) => format!("error: {error}"),
            };
            let mut tx = self.pool.begin().await?;
            sqlx::query("UPDATE lanes SET updated_at = updated_at WHERE key = ?")
                .bind(key)
                .execute(&mut *tx)
                .await?;
            let (committed_generation,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'",
        )
        .bind(key)
        .fetch_one(&mut *tx)
        .await?;
            let (committed_status,): (String,) =
                sqlx::query_as("SELECT status FROM lanes WHERE key = ?")
                    .bind(key)
                    .fetch_one(&mut *tx)
                    .await?;
            if committed_generation != generation
                || !matches!(
                    committed_status.as_str(),
                    "working" | "not_done" | "blocked" | "done_claimed"
                )
            {
                tx.rollback().await?;
                generation = committed_generation;
                was_reopened = generation != 0;
                reopen_at = reopen_marker(&self.pool, key).await?.map(|(_, at)| at);
                resume_fallback = false;
                errors = 0;
                continue;
            }
            sqlx::query(
                "INSERT INTO lane_transcript(lane_key, at, role, text) VALUES (?, ?, 'outcome', ?)",
            )
            .bind(key)
            .bind(Utc::now())
            .bind(if shutdown.is_cancelled() {
                "interrupted"
            } else {
                &outcome_text
            })
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE lanes SET turns_run = turns_run + 1, omo_thread_id = ?, last_turn_outcome = CASE WHEN last_turn_outcome = 'idle_without_claim' THEN last_turn_outcome ELSE ? END, updated_at = ? WHERE key = ?")
                .bind(id).bind(&outcome_text).bind(Utc::now()).bind(key).execute(&mut *tx).await?;
            tx.commit().await?;
            self.posts_ready.notify_one();
            if outcome.is_ok() {
                for (id, _) in inbox {
                    sqlx::query("UPDATE lane_inbox SET consumed_at = ? WHERE id = ? AND consumed_at IS NULL AND EXISTS (SELECT 1 FROM lanes WHERE key = ? AND status IN ('working', 'not_done')) AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                        .bind(Utc::now()).bind(id).bind(key).bind(key).bind(generation).execute(&self.pool).await?;
                }
            }
            if shutdown.is_cancelled() || paused_now {
                return Ok(());
            }
            errors = if outcome.is_err() { errors + 1 } else { 0 };
            if errors >= 3 {
                return self.fail(key, "dead", &outcome_text, generation).await;
            }
            if outcome.is_ok() && self.lane(key).await?.status == "working" {
                if reopen_generation(&self.pool, key).await? != generation {
                    generation = reopen_generation(&self.pool, key).await?;
                    was_reopened = true;
                    resume_fallback = false;
                    errors = 0;
                    continue;
                }
                let transcript = read_transcript(&self.pool, key, None).await?;
                let messages: Vec<_> = transcript
                    .iter()
                    .filter(|entry| entry.id > transcript_start && entry.role == "agent")
                    .collect();
                let reports: Vec<_> = messages
                    .iter()
                    .flat_map(|entry| super::activity::reports(key, &entry.text))
                    .collect();
                let pause = if reports.is_empty() {
                    activity
                        .unreported_turn_generation(
                            key,
                            messages.last().map_or("", |entry| entry.text.as_str()),
                            self.judge.as_ref(),
                            generation,
                        )
                        .await?
                        .pause
                } else {
                    reports
                        .iter()
                        .any(|report| report.kind == super::activity::ReportKind::Done)
                };
                if pause
                    && reopen_generation(&self.pool, key).await? == generation
                    && self.lane(key).await?.status == "working"
                {
                    sqlx::query("UPDATE lanes SET last_turn_outcome = 'idle_without_claim' WHERE key = ? AND status = 'working' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
                        .bind(key).bind(key).bind(generation).execute(&self.pool).await?;
                }
            }
        }
    }

    pub async fn nudge_quiet(
        &self,
        key: &str,
        session: &SessionContext,
        generation: i64,
    ) -> Result<bool> {
        self.nudge_quiet_at(key, session, generation, Utc::now())
            .await
    }

    /// Steers a working, unpaused lane that has been quiet for [`NUDGE_AFTER`] once per
    /// silence, and only when the backend reports no tool item in flight. A silence ends
    /// when captured output moves `last_activity_at` past the previous nudge.
    pub async fn nudge_quiet_at(
        &self,
        key: &str,
        session: &SessionContext,
        generation: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<bool> {
        if !lane_control_enabled() || paused(&self.pool, key).await? {
            return Ok(false);
        }
        let lane = self.lane(key).await?;
        if lane.status != "working"
            || lane.last_activity_at > now - NUDGE_AFTER
            || reopen_generation(&self.pool, key).await? != generation
        {
            return Ok(false);
        }
        let nudged: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_events WHERE lane_key = ? AND kind = 'nudged' AND created_at >= ?)")
            .bind(key).bind(lane.last_activity_at).fetch_one(&self.pool).await?;
        if nudged || self.backend.tool_in_flight(session).await != Some(false) {
            return Ok(false);
        }
        match self.backend.steer(session, NUDGE_TEXT).await {
            Ok(true) => {}
            Ok(false) => return Ok(false),
            Err(error) => {
                tracing::warn!(%key, %error, "lane nudge deferred");
                return Ok(false);
            }
        }
        sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, 'nudged', ?, ?)")
            .bind(key).bind(&lane.lead_session_key).bind(NUDGE_TEXT).bind(now).execute(&self.pool).await?;
        Ok(true)
    }

    async fn lane(&self, key: &str) -> Result<Lane> {
        get_lane(&self.pool, key)
            .await?
            .ok_or_else(|| OmonError::Config(format!("lane not found: {key}")))
    }

    async fn clear_failed_resume(&self, key: &str, generation: i64) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query("UPDATE lanes SET omo_thread_id = NULL, last_turn_outcome = 'thread_resume_failed', updated_at = ? WHERE key = ? AND status = 'working' AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?")
            .bind(Utc::now()).bind(key).bind(key).bind(generation).execute(&mut *tx).await?;
        if changed.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }
        sqlx::query("UPDATE sessions SET state_json = json_remove(state_json, '$.metadata.omo_thread_id') WHERE session_key = ?")
            .bind(lane_key(key).storage_key()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, 'working', 'working', 'Resumed lane; previous thread unavailable')")
            .bind(key).bind(Utc::now()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn pending(&self, key: &str) -> Result<Vec<(i64, String)>> {
        Ok(sqlx::query_as("SELECT id, text FROM lane_inbox WHERE lane_key = ? AND consumed_at IS NULL ORDER BY id")
            .bind(key).fetch_all(&self.pool).await?)
    }

    async fn fail(&self, key: &str, kind: &str, reason: &str, generation: i64) -> Result<()> {
        let body: String = super::scrub::scrub(reason).chars().take(2000).collect();
        let mut tx = self.pool.begin().await?;
        let lane: Option<Lane> = sqlx::query_as("UPDATE lanes SET updated_at = updated_at WHERE key = ? AND status IN ('working', 'blocked', 'queued') AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ? RETURNING *")
            .bind(key).bind(key).bind(generation).fetch_optional(&mut *tx).await?;
        if let Some(lane) = lane {
            sqlx::query("UPDATE lanes SET status = 'failed', updated_at = ? WHERE key = ?")
                .bind(Utc::now())
                .bind(key)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, ?, 'failed', ?)")
                .bind(key).bind(Utc::now()).bind(lane.status).bind(reason).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
                .bind(key).bind(lane.lead_session_key).bind(kind).bind(body).bind(Utc::now()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

pub fn brief_footer(lane: &Lane) -> String {
    format!("Lane key: {}\nDone = {}\n[REPORT] {} | milestone|blocker|question|done | <text>\nClaim command: omo-gateway lane claim-done {} --evidence \"<evidence>\"\n\nLane done protocol\nWhen every Done signal is live and every change is pushed, run the claim command from your own session. For local or QA-only work the signals can be a command, file or URL. Run them yourself first: the watcher re-runs every signal and never takes your word for it. The lane key above is required.\nAfter claiming, stay idle and start no new work. The lead receives a done_claimed event. A claim starts a five-minute objection window. Before closing, the watcher reads every signal live and refuses uncommitted tracked changes or commits on no remote. Closing marks the thread done and removes lane-owned worktrees.\nIf the lead objects or a check fails, you receive [LEAD] not done: <reasons>; keep working, claim again when fixed. Fix every reason, then claim again for a fresh five-minute window.", lane.key, lane.done_signals, lane.key, lane.key)
}

#[cfg(test)]
mod activity_generation_tests {
    use super::*;

    #[tokio::test]
    async fn interrupted_restoration_without_a_worktree_rolls_back_to_removed() {
        let dir = tempfile::tempdir().unwrap();
        let repo = tokio::fs::canonicalize(dir.path())
            .await
            .unwrap()
            .join("repo");
        tokio::fs::create_dir(&repo).await.unwrap();
        git(repo.to_str().unwrap(), &["init", "--quiet"])
            .await
            .unwrap();
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let missing = dir.path().join("gone-lane");
        let mut lane = Lane::new(
            "restoring",
            "Restoring",
            "lead",
            missing.to_str().unwrap(),
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "closed".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        sqlx::query("INSERT INTO lane_owned_worktrees(lane_key,path,role,repo,common_dir,git_dir,identity,branch,registrar,state) VALUES ('restoring',?,'cwd',?,'c','g','i','b','test','restoring')")
            .bind(missing.to_str().unwrap())
            .bind(repo.to_str().unwrap())
            .execute(db.pool())
            .await
            .unwrap();
        // The receipt checks that follow still refuse this lane, but the stranded
        // 'restoring' state itself must be cleared first.
        assert!(restore_cwd(db.pool(), &lane).await.is_err());
        let state: String =
            sqlx::query_scalar("SELECT state FROM lane_owned_worktrees WHERE lane_key='restoring'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(state, "removed");
    }

    #[tokio::test]
    async fn parked_owner_reply_notifies_the_lead_once() {
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut lane = Lane::new(
            "parked",
            "Parked",
            "lead-key",
            dir.path().join("missing-cwd").to_str().unwrap(),
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "closed".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        sqlx::query("INSERT INTO lane_owner_replies(message_id,lane_key,author_id,text,generation) VALUES ('m1','parked','owner','again',0)")
            .execute(db.pool())
            .await
            .unwrap();
        let runner = LaneRunner::new(db.pool().clone(), Arc::new(UnusedBackend));
        runner.apply_owner_replies().await.unwrap();
        runner.apply_owner_replies().await.unwrap();
        let events: Vec<(String, String)> = sqlx::query_as(
            "SELECT kind, lead_session_key FROM lane_events WHERE lane_key = 'parked'",
        )
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(
            events,
            vec![("owner_reply_failed".into(), "lead-key".into())]
        );
    }

    struct UnusedBackend;

    #[async_trait]
    impl AgentBackend for UnusedBackend {
        async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
            panic!("failure fencing does not run a backend")
        }
    }

    #[tokio::test]
    async fn old_third_error_cannot_fail_reopened_lane() {
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let mut lane = Lane::new(
            "fence",
            "Fence",
            "lead",
            "/tmp",
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "closed".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        reopen(db.pool(), "fence", Some("new work")).await.unwrap();
        let runner = LaneRunner::new(db.pool().clone(), Arc::new(UnusedBackend));
        runner
            .fail("fence", "dead", "third old error", 0)
            .await
            .unwrap();
        assert_eq!(
            get_lane(db.pool(), "fence").await.unwrap().unwrap().status,
            "working"
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_events")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn stale_fallback_preserves_new_durable_binding() {
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let mut lane = Lane::new(
            "fence",
            "Fence",
            "lead",
            "/tmp",
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "closed".into();
        lane.omo_thread_id = Some("new-binding".into());
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        let key = lane_key("fence");
        sqlx::query("INSERT INTO sessions(session_key, platform, channel_id, user_id, state_json, created_at, updated_at) VALUES (?, 'lane', 'fence', 'lane:fence', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(key.storage_key()).bind(json!({"metadata":{"omo_thread_id":"new-binding"}}).to_string()).execute(db.pool()).await.unwrap();
        reopen(db.pool(), "fence", None).await.unwrap();
        let runner = LaneRunner::new(db.pool().clone(), Arc::new(UnusedBackend));
        assert!(!runner.clear_failed_resume("fence", 0).await.unwrap());
        let state: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(key.storage_key())
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&state).unwrap()["metadata"]["omo_thread_id"],
            "new-binding"
        );
        assert_eq!(
            get_lane(db.pool(), "fence")
                .await
                .unwrap()
                .unwrap()
                .omo_thread_id
                .as_deref(),
            Some("new-binding")
        );
    }

    #[tokio::test]
    async fn reopen_clears_a_pause_that_outlived_close() {
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let mut lane = Lane::new(
            "paused",
            "Paused",
            "lead",
            "/tmp",
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "closed".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        // Close never clears a pause, so the row is still open when the lane reopens.
        sqlx::query("INSERT INTO lane_pauses(lane_key, paused_at, paused_by, resumed_at) VALUES ('paused', ?, 'owner', NULL)")
            .bind(Utc::now())
            .execute(db.pool())
            .await
            .unwrap();
        reopen(db.pool(), "paused", None).await.unwrap();
        assert!(!crate::storage::lanes::is_lane_paused(db.pool(), "paused")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn close_surfaces_the_unmerged_lane_branch() {
        let dir = tempfile::tempdir().unwrap();
        let root = tokio::fs::canonicalize(dir.path()).await.unwrap();
        let repo_path = root.join("repo");
        tokio::fs::create_dir(&repo_path).await.unwrap();
        let repo = repo_path.to_string_lossy().into_owned();
        git(&repo, &["init", "--quiet"]).await.unwrap();
        git(&repo, &["config", "user.email", "lane@example.com"])
            .await
            .unwrap();
        git(&repo, &["config", "user.name", "lane"]).await.unwrap();
        tokio::fs::write(repo_path.join("seed.txt"), "seed")
            .await
            .unwrap();
        git(&repo, &["add", "seed.txt"]).await.unwrap();
        git(&repo, &["commit", "--quiet", "-m", "seed"])
            .await
            .unwrap();
        let tree_path = root.join("repo-lanes").join("b15");
        tokio::fs::create_dir_all(root.join("repo-lanes"))
            .await
            .unwrap();
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "lane/b15",
                tree_path.to_str().unwrap(),
                "HEAD",
            ],
        )
        .await
        .unwrap();
        let tree = tokio::fs::canonicalize(&tree_path).await.unwrap();
        tokio::fs::write(tree.join("lane.txt"), "lane work")
            .await
            .unwrap();
        git(tree.to_str().unwrap(), &["add", "lane.txt"])
            .await
            .unwrap();
        git(tree.to_str().unwrap(), &["commit", "--quiet", "-m", "work"])
            .await
            .unwrap();
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let mut lane = Lane::new(
            "b15",
            "B15",
            "lead",
            tree.to_str().unwrap(),
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "verified_done".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        register_owned_worktree(db.pool(), "b15", &repo_path, &tree)
            .await
            .unwrap();
        let runner = LaneRunner::new(db.pool().clone(), Arc::new(UnusedBackend));
        let id = crate::storage::lanes::request_close(db.pool(), "b15", false, None)
            .await
            .unwrap();
        runner.close_intent(id).await.unwrap();
        let body: String = sqlx::query_scalar(
            "SELECT body FROM lane_events WHERE lane_key='b15' AND kind='unmerged_branch'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert!(body.contains("lane/b15"), "{body}");
        assert_eq!(
            get_lane(db.pool(), "b15").await.unwrap().unwrap().status,
            "closed"
        );
    }

    #[tokio::test]
    async fn unmerged_branch_counts_only_work_beyond_the_recorded_base() {
        let dir = tempfile::tempdir().unwrap();
        let root = tokio::fs::canonicalize(dir.path()).await.unwrap();
        let repo_path = root.join("repo");
        tokio::fs::create_dir(&repo_path).await.unwrap();
        let repo = repo_path.to_string_lossy().into_owned();
        git(&repo, &["init", "--quiet"]).await.unwrap();
        git(&repo, &["config", "user.email", "lane@example.com"])
            .await
            .unwrap();
        git(&repo, &["config", "user.name", "lane"]).await.unwrap();
        tokio::fs::write(repo_path.join("seed.txt"), "seed")
            .await
            .unwrap();
        git(&repo, &["add", "seed.txt"]).await.unwrap();
        git(&repo, &["commit", "--quiet", "-m", "seed"])
            .await
            .unwrap();
        let base = git(&repo, &["rev-parse", "HEAD"]).await.unwrap();
        let tree_path = root.join("repo-lanes").join("b15b");
        tokio::fs::create_dir_all(root.join("repo-lanes"))
            .await
            .unwrap();
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "lane/b15b",
                tree_path.to_str().unwrap(),
                "HEAD",
            ],
        )
        .await
        .unwrap();
        let tree = tokio::fs::canonicalize(&tree_path).await.unwrap();
        tokio::fs::write(tree.join("lane.txt"), "lane work")
            .await
            .unwrap();
        git(tree.to_str().unwrap(), &["add", "lane.txt"])
            .await
            .unwrap();
        git(tree.to_str().unwrap(), &["commit", "--quiet", "-m", "work"])
            .await
            .unwrap();
        // The control repository moves onto an unrelated history, so its HEAD no longer
        // contains the commit the lane was started from.
        git(&repo, &["checkout", "--orphan", "unrelated"])
            .await
            .unwrap();
        git(
            &repo,
            &["commit", "--quiet", "--allow-empty", "-m", "unrelated"],
        )
        .await
        .unwrap();
        let db = crate::storage::Database::connect("sqlite::memory:")
            .await
            .unwrap();
        let mut lane = Lane::new(
            "b15b",
            "B15b",
            "lead",
            tree.to_str().unwrap(),
            "/tmp/brief",
            "lane",
            "file done",
        );
        lane.status = "working".into();
        crate::storage::insert_lane(db.pool(), &lane).await.unwrap();
        register_owned_worktree(db.pool(), "b15b", &repo_path, &tree)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO lane_usage_context(lane_key,start_commit,purpose) VALUES('b15b',?,'lane_small')",
        )
        .bind(&base)
        .execute(db.pool())
        .await
        .unwrap();
        let note = unmerged_lane_branch(db.pool(), "b15b")
            .await
            .unwrap()
            .unwrap();
        assert!(note.contains("lane/b15b"), "{note}");
        assert!(note.contains("1 commits"), "{note}");
        assert!(
            note.contains(&base.chars().take(12).collect::<String>()),
            "{note}"
        );
        // Only without a recorded base does the moving HEAD become the reference, and it
        // then counts the abandoned base commit as lane work.
        sqlx::query("DELETE FROM lane_usage_context WHERE lane_key='b15b'")
            .execute(db.pool())
            .await
            .unwrap();
        let fallback = unmerged_lane_branch(db.pool(), "b15b")
            .await
            .unwrap()
            .unwrap();
        assert!(fallback.contains("2 commits"), "{fallback}");
    }
}

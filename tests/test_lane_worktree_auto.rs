//! Todo 5: every repo lane gets its own worktree automatically.
//! All git commands run against throwaway repositories under a temp dir.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::cli::change_claim;
use omon_gateway::lanes::runner::{
    create_auto_worktree, lane_key, register_owned_worktree, LaneRunner,
};
use omon_gateway::lanes::settle::Settler;
use omon_gateway::lanes::signals::{DoneSignal, SignalResult, SignalRunner};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::MockClock;
use omon_gateway::{InboundEvent, Result, SessionContext};

struct NoModel;
#[async_trait]
impl AgentBackend for NoModel {
    async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
        panic!("worktree tests must not run a model")
    }
}

struct Pass;
#[async_trait]
impl SignalRunner for Pass {
    async fn check(&self, signal: &DoneSignal, _: &Path) -> SignalResult {
        SignalResult::pass(signal.to_string(), "verified")
    }
}

async fn git(cwd: &Path, args: &[&str]) -> String {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["-c", "user.name=t", "-c", "user.email=t@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

async fn temp_repo() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = tokio::fs::canonicalize(temp.path()).await.unwrap();
    let repo = root.join("repo");
    tokio::fs::create_dir(&repo).await.unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]).await;
    tokio::fs::write(repo.join("README"), "base\n")
        .await
        .unwrap();
    git(&repo, &["add", "README"]).await;
    git(&repo, &["commit", "--quiet", "-m", "base"]).await;
    let remote = root.join("remote.git");
    tokio::fs::create_dir(&remote).await.unwrap();
    git(&remote, &["init", "--bare", "--quiet"]).await;
    git(
        &repo,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    )
    .await;
    git(&repo, &["push", "-q", "origin", "main"]).await;
    (temp, repo)
}

async fn cli_start(url: &str, cwd: &Path, brief: &Path, key: &str) -> Lane {
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(cwd)
        .env("DATABASE_URL", url)
        .env_remove("OMON_LANE_AUTO_WORKTREE")
        .env_remove("PI_SESSION_ID")
        .args(["lane", "start", key, "--title", "QA", "--brief"])
        .arg(brief)
        .args([
            "--done",
            "file done.txt",
            "--lead",
            &lane_key("lead").storage_key(),
            "--model",
            "small",
        ])
        .output()
        .await
        .unwrap();
    // 4 is a host-guard hold (queued); the lane and its worktree exist either way.
    assert!(
        matches!(output.status.code(), Some(0 | 4)),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let database = Database::connect(url).await.unwrap();
    let lane = get_lane(database.pool(), key).await.unwrap().unwrap();
    database.close().await;
    lane
}

async fn owned_rows(pool: &sqlx::SqlitePool, key: &str) -> Vec<(String, String, String)> {
    sqlx::query_as("SELECT path, role, state FROM lane_owned_worktrees WHERE lane_key = ?")
        .bind(key)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn two_lanes_get_distinct_worktree_paths() {
    let (temp, repo) = temp_repo().await;
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "create done.txt").await.unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let a = cli_start(&url, &repo, &brief, "wt-a").await;
    let b = cli_start(&url, &repo, &brief, "wt-b").await;

    let lanes_dir = repo.parent().unwrap().join("repo-lanes");
    assert_eq!(PathBuf::from(&a.cwd), lanes_dir.join("wt-a"));
    assert_eq!(PathBuf::from(&b.cwd), lanes_dir.join("wt-b"));
    assert_ne!(a.cwd, b.cwd);

    let listed = git(&repo, &["worktree", "list", "--porcelain"]).await;
    for (lane, branch) in [(&a, "lane/wt-a"), (&b, "lane/wt-b")] {
        assert!(
            listed.contains(&format!("worktree {}", lane.cwd)),
            "{listed}"
        );
        assert!(
            listed.contains(&format!("branch refs/heads/{branch}")),
            "{listed}"
        );
    }
    let database = Database::connect(&url).await.unwrap();
    for lane in [&a, &b] {
        assert_eq!(
            owned_rows(database.pool(), &lane.key).await,
            vec![(lane.cwd.clone(), "cwd".into(), "registered".into())]
        );
    }
}

#[tokio::test]
async fn dirty_lane_does_not_block_another_lanes_claim() {
    let (temp, repo) = temp_repo().await;
    let brief = temp.path().join("brief.md");
    tokio::fs::write(&brief, "create done.txt").await.unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    // B starts first, so A's worktree is created after B's start and would count
    // against B unless worktrees owned by other lanes are excluded.
    let b = cli_start(&url, &repo, &brief, "claim-b").await;
    let a = cli_start(&url, &repo, &brief, "dirty-a").await;
    tokio::fs::write(Path::new(&a.cwd).join("README"), "lane a edit\n")
        .await
        .unwrap();

    let database = Database::connect(&url).await.unwrap();
    let pool = database.pool();
    for key in [&a.key, &b.key] {
        sqlx::query("UPDATE lanes SET status = 'working' WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await
            .unwrap();
        change_claim(pool, key, true, "file evidence")
            .await
            .unwrap();
    }
    // No start commit: done_diff skips instead of calling the real judge.
    sqlx::query("DELETE FROM lane_usage_context")
        .execute(pool)
        .await
        .unwrap();
    let settler = Settler::new(
        pool.clone(),
        Arc::new(MockClock::new(
            chrono::Utc::now() + chrono::Duration::minutes(10),
        )),
        Arc::new(Pass),
    );
    let verified = settler.settle_due().await.unwrap();
    assert_eq!(verified, vec![b.key.clone()]);
    assert_eq!(
        get_lane(pool, &b.key).await.unwrap().unwrap().status,
        "verified_done"
    );
    let a_row = get_lane(pool, &a.key).await.unwrap().unwrap();
    assert_eq!(a_row.status, "not_done");
    assert!(
        a_row
            .objection
            .as_deref()
            .unwrap_or("")
            .contains("uncommitted"),
        "{:?}",
        a_row.objection
    );
}

#[tokio::test]
async fn sweep_removes_clean_orphan_and_keeps_dirty_one() {
    let (temp, repo) = temp_repo().await;
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let database = Database::connect(&url).await.unwrap();
    let pool = database.pool();
    let mut paths = Vec::new();
    for key in ["orphan-clean", "orphan-dirty"] {
        let tree = create_auto_worktree(&repo, key).await.unwrap().unwrap();
        let mut lane = Lane::new(
            key,
            "QA",
            lane_key("lead").storage_key(),
            tree.path.to_str().unwrap(),
            temp.path().join("brief.md").to_str().unwrap(),
            lane_key(key).storage_key(),
            "file done.txt",
        );
        lane.status = "working".into();
        insert_lane(pool, &lane).await.unwrap();
        register_owned_worktree(pool, key, &repo, &tree.path)
            .await
            .unwrap();
        sqlx::query("UPDATE lanes SET status = 'failed' WHERE key = ?")
            .bind(key)
            .execute(pool)
            .await
            .unwrap();
        paths.push(tree.path);
    }
    tokio::fs::write(paths[1].join("scratch.txt"), "unsaved\n")
        .await
        .unwrap();

    let runner = LaneRunner::new(pool.clone(), Arc::new(NoModel));
    runner.sweep_orphan_worktrees().await.unwrap();
    // A second sweep must not report the kept worktree again.
    runner.sweep_orphan_worktrees().await.unwrap();

    assert!(!tokio::fs::try_exists(&paths[0]).await.unwrap());
    assert_eq!(owned_rows(pool, "orphan-clean").await[0].2, "removed");
    assert_eq!(
        get_lane(pool, "orphan-clean")
            .await
            .unwrap()
            .unwrap()
            .status,
        "closed"
    );
    // The branch survives removal: no work is ever deleted.
    git(
        &repo,
        &["show-ref", "--verify", "refs/heads/lane/orphan-clean"],
    )
    .await;

    assert!(tokio::fs::try_exists(paths[1].join("scratch.txt"))
        .await
        .unwrap());
    assert_eq!(owned_rows(pool, "orphan-dirty").await[0].2, "registered");
    assert_eq!(
        get_lane(pool, "orphan-dirty")
            .await
            .unwrap()
            .unwrap()
            .status,
        "failed"
    );
    let kept: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM lane_events WHERE lane_key = 'orphan-dirty' AND kind = 'worktree_kept'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(kept, 1);
}

#[tokio::test]
async fn non_git_cwd_creates_no_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let root = tokio::fs::canonicalize(temp.path()).await.unwrap();
    let plain = root.join("plain");
    tokio::fs::create_dir(&plain).await.unwrap();
    let brief = root.join("brief.md");
    tokio::fs::write(&brief, "create done.txt").await.unwrap();
    let url = format!("sqlite://{}", root.join("qa.db").display());
    let lane = cli_start(&url, &plain, &brief, "plain-lane").await;

    assert_eq!(PathBuf::from(&lane.cwd), plain);
    assert!(!tokio::fs::try_exists(root.join("plain-lanes"))
        .await
        .unwrap());
    let database = Database::connect(&url).await.unwrap();
    assert!(owned_rows(database.pool(), "plain-lane").await.is_empty());
}

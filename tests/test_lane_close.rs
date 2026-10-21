use async_trait::async_trait;
use omon_gateway::agent::AgentBackend;
use omon_gateway::discord::adapter::{intercept_lane_owner_reply, InboundFilterConfig};
use omon_gateway::lanes::discord_threads::{DiscordThreadError, ThreadStatus};
use omon_gateway::lanes::runner::{lane_key, register_owned_worktree, LaneLifecycle, LaneRunner};
use omon_gateway::storage::lanes::{bind_lane_thread, request_close};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::{InboundEvent, Result, SessionContext};
use serenity::all::{ChannelType, Message, UserId};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Notify};
use tokio_util::sync::CancellationToken;

struct Backend;
#[async_trait]
impl AgentBackend for Backend {
    async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
        panic!("close must not run a model")
    }
}

#[derive(Default)]
struct Transport {
    calls: Mutex<Vec<String>>,
    fail: bool,
    hang_status: bool,
}
#[async_trait]
impl LaneLifecycle for Transport {
    async fn bot_id(&self) -> std::result::Result<String, DiscordThreadError> {
        Ok("99".into())
    }
    async fn status(
        &self,
        _: &str,
        status: ThreadStatus,
    ) -> std::result::Result<(), DiscordThreadError> {
        self.calls.lock().unwrap().push(format!("{status:?}"));
        if self.hang_status {
            std::future::pending::<()>().await;
        }
        if self.fail {
            return Err(DiscordThreadError::MissingToken);
        }
        Ok(())
    }
    async fn archived(
        &self,
        _: &str,
        archived: bool,
    ) -> std::result::Result<(), DiscordThreadError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("archived:{archived}"));
        Ok(())
    }
}

async fn git(repo: &Path, args: &[&str]) -> String {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().into()
}

struct Fixture {
    dir: tempfile::TempDir,
    db: Database,
    transport: Arc<Transport>,
}
impl Fixture {
    async fn new(status: &str, owned: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        tokio::fs::create_dir(&repo).await.unwrap();
        git(&repo, &["init", "-b", "main"]).await;
        git(
            &repo,
            &[
                "-c",
                "user.name=QA",
                "-c",
                "user.email=qa@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        )
        .await;
        let remote = dir.path().join("remote.git");
        tokio::fs::create_dir(&remote).await.unwrap();
        git(&remote, &["init", "--bare"]).await;
        git(
            &repo,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        )
        .await;
        git(&repo, &["push", "-q", "origin", "main"]).await;
        let cwd = dir.path().join("lane");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "lane-branch",
                cwd.to_str().unwrap(),
            ],
        )
        .await;
        let cwd = tokio::fs::canonicalize(cwd).await.unwrap();
        let db = Database::connect(&format!(
            "sqlite://{}",
            dir.path().join("db.sqlite").display()
        ))
        .await
        .unwrap();
        let mut lane = Lane::new(
            "close-qa",
            "QA",
            lane_key("lead").storage_key(),
            cwd.to_str().unwrap(),
            dir.path().join("brief.md").to_str().unwrap(),
            lane_key("close-qa").storage_key(),
            "file result",
        );
        lane.status = status.into();
        lane.thread_channel_id = Some("88".into());
        lane.omo_thread_id = Some("continuity".into());
        tokio::fs::write(&lane.brief_path, "fixture").await.unwrap();
        insert_lane(db.pool(), &lane).await.unwrap();
        bind_lane_thread(db.pool(), &lane.key, "99").await.unwrap();
        if owned {
            register_owned_worktree(db.pool(), &lane.key, &repo, &cwd)
                .await
                .unwrap();
        }
        Self {
            dir,
            db,
            transport: Arc::new(Transport::default()),
        }
    }
    fn runner(&self) -> LaneRunner {
        LaneRunner::new(self.db.pool().clone(), Arc::new(Backend))
            .with_lifecycle(self.transport.clone())
    }
    async fn close(&self) -> Result<()> {
        let id = request_close(self.db.pool(), "close-qa", false, None).await?;
        self.runner().close_intent(id).await
    }
    async fn status(&self) -> String {
        get_lane(self.db.pool(), "close-qa")
            .await
            .unwrap()
            .unwrap()
            .status
    }
    fn cwd(&self) -> std::path::PathBuf {
        std::fs::canonicalize(self.dir.path()).unwrap().join("lane")
    }
    fn repo(&self) -> std::path::PathBuf {
        self.dir.path().join("repo")
    }
}

fn message(id: &str, author: &str, content: &str) -> Message {
    serde_json::from_value(serde_json::json!({"id":id,"channel_id":"88","guild_id":"77","author":{"id":author,"username":"owner","discriminator":"0001","avatar":null,"bot":false},"content":content,"timestamp":"2026-10-11T00:00:00Z","edited_timestamp":null,"tts":false,"mention_everyone":false,"mentions":[],"mention_roles":[],"attachments":[],"embeds":[],"pinned":false,"type":0})).unwrap()
}

#[tokio::test]
async fn registered_cwd_removed_and_owner_reply_restores_same_branch_once() {
    let f = Fixture::new("verified_done", true).await;
    f.close().await.unwrap();
    assert_eq!(f.status().await, "closed");
    assert!(!f.cwd().exists());
    assert!(!git(&f.repo(), &["worktree", "list", "--porcelain"])
        .await
        .contains(f.cwd().to_str().unwrap()));
    git(
        &f.repo(),
        &["show-ref", "--verify", "refs/heads/lane-branch"],
    )
    .await;
    let config = InboundFilterConfig {
        allowed_users: &[414011632306618368],
        thread_require_mention: true,
        ..Default::default()
    };
    let reply = message(
        "1001",
        "414011632306618368",
        "  exact owner text\nnext line  ",
    );
    assert!(intercept_lane_owner_reply(
        f.db.pool(),
        &reply,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    assert!(intercept_lane_owner_reply(
        f.db.pool(),
        &reply,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    f.runner().apply_owner_replies().await.unwrap();
    assert!(intercept_lane_owner_reply(
        f.db.pool(),
        &reply,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    f.runner().apply_owner_replies().await.unwrap();
    assert_eq!(f.status().await, "working");
    assert_eq!(
        git(&f.cwd(), &["symbolic-ref", "--short", "HEAD"]).await,
        "lane-branch"
    );
    let text: Vec<String> = sqlx::query_scalar("SELECT text FROM lane_inbox")
        .fetch_all(f.db.pool())
        .await
        .unwrap();
    assert_eq!(text, vec![reply.content]);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_history WHERE reason='reopened'")
            .fetch_one(f.db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        get_lane(f.db.pool(), "close-qa")
            .await
            .unwrap()
            .unwrap()
            .omo_thread_id
            .as_deref(),
        Some("continuity")
    );
    assert_eq!(
        *f.transport.calls.lock().unwrap(),
        vec!["Done", "archived:true", "Working", "archived:false"]
    );
}

#[tokio::test]
async fn dirty_untracked_removal_fails_once_and_preserves_verified_done() {
    let f = Fixture::new("verified_done", true).await;
    tokio::fs::write(f.cwd().join("dirty"), "owner data")
        .await
        .unwrap();
    let error = f.close().await.unwrap_err();
    assert!(error.to_string().contains("git worktree remove"), "{error}");
    assert_eq!(f.status().await, "verified_done");
    assert!(f.cwd().join("dirty").exists());
    let id: i64 = sqlx::query_scalar("SELECT id FROM lane_close_intents")
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    f.runner().close_intent(id).await.unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_events WHERE kind='close_failed'")
            .fetch_one(f.db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
    git(
        &f.repo(),
        &["show-ref", "--verify", "refs/heads/lane-branch"],
    )
    .await;
}

#[tokio::test]
async fn forced_failure_preserves_original_status_and_normal_close_refuses() {
    let f = Fixture::new("working", true).await;
    assert!(request_close(f.db.pool(), "close-qa", false, None)
        .await
        .is_err());
    assert!(request_close(f.db.pool(), "close-qa", true, None)
        .await
        .is_err());
    assert!(
        request_close(f.db.pool(), "close-qa", false, Some("forced: QA"))
            .await
            .is_err()
    );
    tokio::fs::write(f.cwd().join("dirty"), "owner data")
        .await
        .unwrap();
    let id = request_close(f.db.pool(), "close-qa", true, Some("forced: QA"))
        .await
        .unwrap();
    assert!(f.runner().close_intent(id).await.is_err());
    assert_eq!(f.status().await, "working");
}

#[tokio::test]
async fn external_cwd_skipped_and_main_worktree_registration_refused() {
    let f = Fixture::new("verified_done", false).await;
    assert!(
        register_owned_worktree(f.db.pool(), "close-qa", &f.repo(), &f.repo())
            .await
            .is_err()
    );
    f.close().await.unwrap();
    assert!(f.cwd().exists());
    let receipt: String = sqlx::query_scalar("SELECT removal FROM lane_close_intents")
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    assert_eq!(receipt, "skipped: unowned external cwd");
}

#[tokio::test]
async fn occupied_restore_path_keeps_lane_closed_and_reply_pending() {
    let f = Fixture::new("verified_done", true).await;
    f.close().await.unwrap();
    tokio::fs::create_dir(f.cwd()).await.unwrap();
    tokio::fs::write(f.cwd().join("owner"), "replacement")
        .await
        .unwrap();
    omon_gateway::storage::lanes::record_owner_reply(
        f.db.pool(),
        "88",
        "99",
        "owner",
        "owner",
        "1001",
        "continue",
    )
    .await
    .unwrap();
    f.runner().apply_owner_replies().await.unwrap();
    assert_eq!(f.status().await, "closed");
    let state: String = sqlx::query_scalar("SELECT state FROM lane_owner_replies")
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    assert_eq!(state, "pending");
    assert!(f.cwd().join("owner").exists());
}

#[tokio::test]
async fn missing_removal_proof_and_branch_deny_recreation() {
    let f = Fixture::new("verified_done", true).await;
    f.close().await.unwrap();
    sqlx::query("UPDATE lane_owned_worktrees SET removal_receipt=NULL")
        .execute(f.db.pool())
        .await
        .unwrap();
    assert!(
        omon_gateway::lanes::runner::reopen(f.db.pool(), "close-qa", Some("continue"))
            .await
            .is_err()
    );
    assert!(!f.cwd().exists());
    assert_eq!(f.status().await, "closed");
}

#[tokio::test]
async fn tracked_dirty_worktree_retains_verified_done_and_branch() {
    let f = Fixture::new("verified_done", true).await;
    tokio::fs::write(f.cwd().join("tracked"), "base")
        .await
        .unwrap();
    git(&f.cwd(), &["add", "tracked"]).await;
    git(
        &f.cwd(),
        &[
            "-c",
            "user.name=QA",
            "-c",
            "user.email=qa@example.invalid",
            "commit",
            "-m",
            "tracked fixture",
        ],
    )
    .await;
    tokio::fs::write(f.cwd().join("tracked"), "owner changes")
        .await
        .unwrap();
    assert!(f.close().await.is_err());
    assert_eq!(f.status().await, "verified_done");
    assert_eq!(
        tokio::fs::read_to_string(f.cwd().join("tracked"))
            .await
            .unwrap(),
        "owner changes"
    );
}

#[tokio::test]
async fn missing_preserved_branch_denies_restoration_without_creating_branch() {
    let f = Fixture::new("verified_done", true).await;
    f.close().await.unwrap();
    git(
        &f.repo(),
        &["branch", "-m", "lane-branch", "renamed-by-owner"],
    )
    .await;
    assert!(
        omon_gateway::lanes::runner::reopen(f.db.pool(), "close-qa", Some("continue"))
            .await
            .is_err()
    );
    assert!(!f.cwd().exists());
    assert!(!git(&f.repo(), &["branch", "--list", "lane-branch"])
        .await
        .contains("lane-branch"));
}

#[tokio::test]
async fn interrupted_removal_reconciles_only_authorized_absence() {
    let f = Fixture::new("verified_done", true).await;
    let id = request_close(f.db.pool(), "close-qa", false, None)
        .await
        .unwrap();
    sqlx::query("UPDATE lane_owned_worktrees SET state='removing',removal_intent=?,removal_identity=identity").bind(id).execute(f.db.pool()).await.unwrap();
    git(
        &f.repo(),
        &["worktree", "remove", f.cwd().to_str().unwrap()],
    )
    .await;
    f.runner().close_intent(id).await.unwrap();
    assert_eq!(f.status().await, "closed");
    let receipt: Option<String> =
        sqlx::query_scalar("SELECT removal_receipt FROM lane_owned_worktrees")
            .fetch_one(f.db.pool())
            .await
            .unwrap();
    assert!(receipt.is_some());
    f.runner().close_intent(id).await.unwrap();
    assert_eq!(f.transport.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn replacement_registration_and_ambiguous_restoration_are_not_adopted() {
    let f = Fixture::new("verified_done", true).await;
    let id = request_close(f.db.pool(), "close-qa", false, None)
        .await
        .unwrap();
    sqlx::query("UPDATE lane_owned_worktrees SET identity='replacement'")
        .execute(f.db.pool())
        .await
        .unwrap();
    assert!(f.runner().close_intent(id).await.is_err());
    assert!(f.cwd().exists());
    let f = Fixture::new("verified_done", true).await;
    f.close().await.unwrap();
    sqlx::query("UPDATE lane_owned_worktrees SET state='restoring'")
        .execute(f.db.pool())
        .await
        .unwrap();
    git(
        &f.repo(),
        &["worktree", "add", f.cwd().to_str().unwrap(), "lane-branch"],
    )
    .await;
    assert!(
        omon_gateway::lanes::runner::reopen(f.db.pool(), "close-qa", Some("continue"))
            .await
            .is_err()
    );
    assert_eq!(f.status().await, "closed");
}

#[tokio::test]
async fn duplicate_thread_identity_is_rejected_by_migration() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE lanes(key TEXT PRIMARY KEY,thread_channel_id TEXT); INSERT INTO lanes VALUES ('first','88'),('second','88');").execute(&pool).await.unwrap();
    assert!(
        sqlx::raw_sql(include_str!("../migrations/0038_lane_lifecycle.sql"))
            .execute(&pool)
            .await
            .is_err()
    );
    let keys: Vec<String> = sqlx::query_scalar("SELECT key FROM lanes ORDER BY key")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(keys, vec!["first", "second"]);
}

#[tokio::test]
async fn distinct_original_generation_replies_enter_inbox_once() {
    let f = Fixture::new("closed", false).await;
    for (id, text) in [("1001", "first"), ("1002", "second")] {
        assert!(omon_gateway::storage::lanes::record_owner_reply(
            f.db.pool(),
            "88",
            "99",
            "owner",
            "owner",
            id,
            text
        )
        .await
        .unwrap());
    }
    f.runner().apply_owner_replies().await.unwrap();
    f.runner().apply_owner_replies().await.unwrap();
    let texts: Vec<String> = sqlx::query_scalar("SELECT text FROM lane_inbox ORDER BY id")
        .fetch_all(f.db.pool())
        .await
        .unwrap();
    assert_eq!(texts, vec!["first", "second"]);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_history WHERE reason='reopened'")
            .fetch_one(f.db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn verified_reply_closes_before_reopening_without_direct_transition() {
    let f = Fixture::new("verified_done", true).await;
    assert!(omon_gateway::storage::lanes::record_owner_reply(
        f.db.pool(),
        "88",
        "99",
        "owner",
        "owner",
        "1001",
        "continue"
    )
    .await
    .unwrap());
    f.runner().apply_owner_replies().await.unwrap();
    assert_eq!(f.status().await, "verified_done");
    let id: i64 = sqlx::query_scalar("SELECT id FROM lane_close_intents")
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    f.runner().close_intent(id).await.unwrap();
    f.runner().apply_owner_replies().await.unwrap();
    assert_eq!(f.status().await, "working");
    assert!(f.cwd().exists());
}

mod event_restart {
    include!("test_lane_dispatch.rs");

    #[tokio::test]
    async fn reconstructed_dispatcher_redelivers_persisted_undelivered_event_once() {
        let mut f = Fixture::new(1.0).await;
        f.enqueue("close-qa", "close_failed", "owned fixture failure", 0)
            .await;
        f.dispatcher = EventDispatcher::new(f.db.pool().clone(), f.mux.clone(), f.judge.clone());
        f.tick(30).await;
        let (event, release) = f.turn().await;
        assert_eq!(event.origin, EventOrigin::System);
        assert_eq!(f.pending().await, 0);
        assert_eq!(f.judge.calls.load(Ordering::SeqCst), 1);
        f.finish(release).await;
        f.dispatcher = EventDispatcher::new(f.db.pool().clone(), f.mux.clone(), f.judge.clone());
        f.tick(61).await;
        assert!(f.started.try_recv().is_err());
    }
}

#[tokio::test]
async fn transport_failure_never_commits_closed_or_removes_cwd() {
    let f = Fixture::new("verified_done", true).await;
    let runner = LaneRunner::new(f.db.pool().clone(), Arc::new(Backend)).with_lifecycle(Arc::new(
        Transport {
            calls: Mutex::new(Vec::new()),
            fail: true,
            hang_status: false,
        },
    ));
    let id = request_close(f.db.pool(), "close-qa", false, None)
        .await
        .unwrap();
    assert!(runner.close_intent(id).await.is_err());
    assert_eq!(f.status().await, "verified_done");
    assert!(f.cwd().exists());
}

#[tokio::test]
async fn adapter_preserves_admission_owner_bot_and_stop_boundaries() {
    let f = Fixture::new("closed", false).await;
    let owner = message("1001", "414011632306618368", "continue");
    let paired_other = message("1002", "123", "continue");
    let denied = InboundFilterConfig {
        allowed_users: &[123],
        ..Default::default()
    };
    assert!(!intercept_lane_owner_reply(
        f.db.pool(),
        &owner,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &denied
    )
    .await
    .unwrap());
    let config = InboundFilterConfig {
        allow_all_users: true,
        paired_users: &[123],
        ..Default::default()
    };
    assert!(!intercept_lane_owner_reply(
        f.db.pool(),
        &paired_other,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    assert!(!intercept_lane_owner_reply(
        f.db.pool(),
        &owner,
        UserId::new(98),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    assert!(!intercept_lane_owner_reply(
        f.db.pool(),
        &message("1003", "414011632306618368", "/stop"),
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    let mut bot = owner.clone();
    bot.author.bot = true;
    assert!(!intercept_lane_owner_reply(
        f.db.pool(),
        &bot,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
    assert!(intercept_lane_owner_reply(
        f.db.pool(),
        &owner,
        UserId::new(99),
        Some(ChannelType::PublicThread),
        &config
    )
    .await
    .unwrap());
}

struct CancelBackend {
    started: mpsc::Sender<()>,
    cancelled: mpsc::Sender<()>,
    finished: mpsc::Sender<()>,
    release: Arc<Notify>,
}
#[async_trait]
impl AgentBackend for CancelBackend {
    async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
        unreachable!()
    }
    async fn run_cancelable(
        &self,
        _: &mut SessionContext,
        _: InboundEvent,
        cancel: CancellationToken,
    ) -> Result<()> {
        self.started.send(()).await.unwrap();
        cancel.cancelled().await;
        self.cancelled.send(()).await.unwrap();
        self.release.notified().await;
        self.finished.send(()).await.unwrap();
        Ok(())
    }
}

#[tokio::test]
async fn service_cancels_and_joins_before_thread_and_git_effects() {
    let f = Fixture::new("working", true).await;
    let (started, mut start) = mpsc::channel(1);
    let (cancelled, mut cancellation) = mpsc::channel(1);
    let (finished, mut finish) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let runner = LaneRunner::new(
        f.db.pool().clone(),
        Arc::new(CancelBackend {
            started,
            cancelled,
            finished,
            release: release.clone(),
        }),
    )
    .with_lifecycle(f.transport.clone());
    let shutdown = CancellationToken::new();
    let task = tokio::spawn(runner.serve(shutdown.clone()));
    tokio::time::timeout(std::time::Duration::from_secs(10), start.recv())
        .await
        .unwrap()
        .unwrap();
    let id = request_close(f.db.pool(), "close-qa", true, Some("forced: QA"))
        .await
        .unwrap();
    omon_gateway::lanes::runner::wake(f.db.pool())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), cancellation.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(f.transport.calls.lock().unwrap().is_empty());
    assert!(f.cwd().exists());
    release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(10), finish.recv())
        .await
        .unwrap()
        .unwrap();
    shutdown.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM lane_close_intents WHERE id=?")
        .bind(id)
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    if state == "pending" {
        f.runner().close_intent(id).await.unwrap();
    }
    assert_eq!(f.status().await, "closed");
}

#[tokio::test]
async fn held_thread_rename_does_not_block_close() {
    // Discord holds a third thread rename within ten minutes; close must still finish.
    let f = Fixture::new("verified_done", true).await;
    let transport = Arc::new(Transport {
        calls: Mutex::new(Vec::new()),
        fail: false,
        hang_status: true,
    });
    let runner = LaneRunner::new(f.db.pool().clone(), Arc::new(Backend))
        .with_lifecycle(transport.clone())
        .with_lifecycle_timeout(std::time::Duration::from_millis(200));
    let id = request_close(f.db.pool(), "close-qa", false, None)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), runner.close_intent(id))
        .await
        .expect("close must not wait on a held rename")
        .unwrap();
    assert_eq!(f.status().await, "closed");
    assert_eq!(
        *transport.calls.lock().unwrap(),
        vec!["Done", "archived:true"]
    );
}

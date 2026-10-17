//! Regression tests for the lanes review fixes (rename failure on reopen).
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use omon_gateway::agent::AgentBackend;
use omon_gateway::lanes::discord_threads::{DiscordThreadError, ThreadStatus};
use omon_gateway::lanes::runner::{LaneLifecycle, LaneRunner};
use omon_gateway::storage::lanes::bind_lane_thread;
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::{InboundEvent, Result, SessionContext};

struct NoBackend;
#[async_trait]
impl AgentBackend for NoBackend {
    async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
        panic!("reopen must not run a model")
    }
}

/// Discord refuses the rename (the thread is still archived) but unarchive works.
#[derive(Default)]
struct RenameRefused {
    calls: Mutex<Vec<String>>,
}

#[async_trait]
impl LaneLifecycle for RenameRefused {
    async fn bot_id(&self) -> std::result::Result<String, DiscordThreadError> {
        Ok("99".into())
    }

    async fn status(
        &self,
        _: &str,
        status: ThreadStatus,
    ) -> std::result::Result<(), DiscordThreadError> {
        self.calls.lock().unwrap().push(format!("{status:?}"));
        Err(DiscordThreadError::MissingToken)
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

#[tokio::test]
async fn refused_rename_does_not_block_an_owner_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let mut lane = Lane::new(
        "rename-refused",
        "Rename refused",
        "lead",
        dir.path().to_str().unwrap(),
        "/tmp/brief",
        "lane:rename-refused",
        "file done",
    );
    lane.status = "closed".into();
    lane.thread_channel_id = Some("88".into());
    insert_lane(db.pool(), &lane).await.unwrap();
    bind_lane_thread(db.pool(), &lane.key, "99").await.unwrap();
    sqlx::query("INSERT INTO lane_owner_replies(message_id,lane_key,author_id,text,generation) VALUES ('m1','rename-refused','owner','continue',0)")
        .execute(db.pool())
        .await
        .unwrap();
    let transport = Arc::new(RenameRefused::default());
    let runner =
        LaneRunner::new(db.pool().clone(), Arc::new(NoBackend)).with_lifecycle(transport.clone());
    runner.apply_owner_replies().await.unwrap();
    assert_eq!(
        get_lane(db.pool(), "rename-refused")
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
    assert_eq!(
        *transport.calls.lock().unwrap(),
        vec!["Working", "archived:false"]
    );
}

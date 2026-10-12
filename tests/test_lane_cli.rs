use std::process::Command;

use omon_gateway::lanes::runner::lane_key;
use omon_gateway::storage::Database;
use std::sync::Arc;

struct GuidedBackend(tokio::sync::mpsc::UnboundedSender<String>, sqlx::SqlitePool);

#[async_trait::async_trait]
impl omon_gateway::agent::AgentBackend for GuidedBackend {
    async fn run(
        &self,
        session: &mut omon_gateway::SessionContext,
        event: omon_gateway::InboundEvent,
    ) -> omon_gateway::Result<()> {
        self.0.send(event.content).unwrap();
        omon_gateway::lanes::cli::change_claim(
            &self.1,
            &session.key.channel_id,
            true,
            "test receipt",
        )
        .await
    }
}

#[tokio::test]
async fn object_resumes_captured_question_with_owner_text_first() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let database = Database::connect(&url).await.unwrap();
    let mut lane = omon_gateway::storage::Lane::new(
        "question",
        "QA",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        "unused",
        lane_key("question").storage_key(),
        "file done.txt",
    );
    lane.status = "working".into();
    lane.turns_run = 1;
    lane.last_turn_outcome = Some("idle_without_claim".into());
    omon_gateway::storage::insert_lane(database.pool(), &lane)
        .await
        .unwrap();
    let activity = omon_gateway::lanes::activity::LaneActivity::new(
        database.pool().clone(),
        Arc::new(omon_gateway::work_heartbeat::SystemClock),
    );
    activity
        .captured("question", "[REPORT] question | question | A or B")
        .await
        .unwrap();
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args(["lane", "object", "question", "--reason", "use B"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "working");
    let row = omon_gateway::storage::get_lane(database.pool(), "question")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.last_turn_outcome, None);
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let runner = omon_gateway::lanes::runner::LaneRunner::new(
        database.pool().clone(),
        Arc::new(GuidedBackend(sent, database.pool().clone())),
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        runner.run_lane(
            "question",
            Arc::new(tokio::sync::Notify::new()),
            tokio_util::sync::CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(received.try_recv().unwrap(), "use B");
    assert!(received.try_recv().is_err());
    assert_eq!(
        omon_gateway::storage::get_lane(database.pool(), "question")
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
}

#[tokio::test]
async fn object_preserves_claimed_objection_and_rejects_other_states() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let database = Database::connect(&url).await.unwrap();
    for (index, status) in [
        "queued",
        "working",
        "not_done",
        "verified_done",
        "closed",
        "failed",
        "done_claimed",
    ]
    .iter()
    .enumerate()
    {
        let key = format!("object-{index}");
        let mut lane = omon_gateway::storage::Lane::new(
            &key,
            "QA",
            "lead",
            "/tmp",
            "unused",
            "lane",
            "file done.txt",
        );
        lane.status = (*status).into();
        omon_gateway::storage::insert_lane(database.pool(), &lane)
            .await
            .unwrap();
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
            .current_dir(temp.path())
            .env("DATABASE_URL", &url)
            .args(["lane", "object", &key, "--reason", "missing result"])
            .output()
            .await
            .unwrap();
        let claimed = *status == "done_claimed";
        assert_eq!(output.status.code(), Some(if claimed { 0 } else { 2 }));
        let row = omon_gateway::storage::get_lane(database.pool(), &key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, if claimed { "not_done" } else { status });
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_inbox WHERE lane_key = ?")
            .bind(&key)
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(count, if claimed { 1 } else { 0 });
    }
}

#[tokio::test]
async fn invalid_done_exits_two_without_a_lane() {
    let temp = tempfile::tempdir().unwrap();
    let brief = temp.path().join("brief.md");
    std::fs::write(&brief, "create done.txt").unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args(["lane", "start", "bad-done", "--title", "QA", "--brief"])
        .arg(&brief)
        .args([
            "--done",
            "command [\"true\"]",
            "--lead",
            &lane_key("lead").storage_key(),
            "--model",
            "small",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false);
    let database = Database::connect(&url).await.unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM lanes")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn hold_exits_four_and_persists_queued_small_lane() {
    let temp = tempfile::tempdir().unwrap();
    let brief = temp.path().join("brief.md");
    std::fs::write(&brief, "create done.txt").unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .env("OMON_GUARD_DISK_FREE_MIN_GB", "100000000")
        .env_remove("OMON_MODEL_SMALL")
        .args(["lane", "start", "held-lane", "--title", "QA", "--brief"])
        .arg(&brief)
        .args([
            "--done",
            "file done.txt",
            "--lead",
            &lane_key("lead").storage_key(),
            "--model",
            "small",
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["lane"]["status"], "queued");
    assert_eq!(value["lane"]["model"], "inferhub/cb/deepseek-v4.1-flash");
    let database = Database::connect(&url).await.unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM lanes WHERE status = 'queued'")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn forced_close_claimed_lane_requires_explicit_force_and_note() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let database = Database::connect(&url).await.unwrap();
    let mut lane = omon_gateway::storage::Lane::new(
        "claimed",
        "Claimed",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        "unused",
        lane_key("claimed").storage_key(),
        "file done.txt",
    );
    lane.status = "done_claimed".into();
    omon_gateway::storage::insert_lane(database.pool(), &lane)
        .await
        .unwrap();
    for args in [
        vec!["lane", "close", "claimed"],
        vec!["lane", "close", "claimed", "--note", "forced: qa"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
            .current_dir(temp.path())
            .env("DATABASE_URL", &url)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args([
            "lane",
            "close",
            "claimed",
            "--force",
            "--note",
            "forced: qa",
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        omon_gateway::storage::get_lane(database.pool(), "claimed")
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "close_pending");
    let state: String =
        sqlx::query_scalar("SELECT state FROM lane_close_intents WHERE lane_key='claimed'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(state, "pending");
}

#[tokio::test]
async fn reopen_command_requires_lane_and_illegal_state_exits_two() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args(["lane", "reopen", "nope"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let database = Database::connect(&url).await.unwrap();
    let mut lane = omon_gateway::storage::Lane::new(
        "working-lane",
        "QA",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        "unused",
        lane_key("working-lane").storage_key(),
        "file done.txt",
    );
    lane.status = "working".into();
    lane.turns_run = 8;
    lane.omo_thread_id = Some("kept-thread".into());
    omon_gateway::storage::insert_lane(database.pool(), &lane)
        .await
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args(["lane", "reopen", "working-lane"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let row = omon_gateway::storage::get_lane(database.pool(), "working-lane")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "working");
    assert_eq!(row.turns_run, 8);
    assert_eq!(row.omo_thread_id.as_deref(), Some("kept-thread"));
}

#[tokio::test]
async fn reopen_command_queues_exact_owner_text_without_premature_reset() {
    let temp = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", temp.path().join("qa.db").display());
    let database = Database::connect(&url).await.unwrap();
    let mut lane = omon_gateway::storage::Lane::new(
        "closed-lane",
        "QA",
        lane_key("lead").storage_key(),
        temp.path().to_str().unwrap(),
        "unused",
        lane_key("closed-lane").storage_key(),
        "file done.txt",
    );
    lane.status = "closed".into();
    lane.turns_run = 12;
    lane.omo_thread_id = Some("kept-thread".into());
    omon_gateway::storage::insert_lane(database.pool(), &lane)
        .await
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
        .current_dir(temp.path())
        .env("DATABASE_URL", &url)
        .args([
            "lane",
            "reopen",
            "closed-lane",
            "--text",
            "add a line to done.txt",
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let row = omon_gateway::storage::get_lane(database.pool(), "closed-lane")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "closed");
    assert_eq!(row.turns_run, 12);
    assert_eq!(row.omo_thread_id.as_deref(), Some("kept-thread"));
    let (text,): (String,) = sqlx::query_as(
        "SELECT text FROM lane_owner_replies WHERE lane_key = 'closed-lane' AND state='pending'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(text, "add a line to done.txt");
    let (count,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM lane_history WHERE lane_key = 'closed-lane' AND reason='reopened'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(count, 0);
}

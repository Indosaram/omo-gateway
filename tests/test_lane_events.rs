use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, TimeZone, Utc};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::lanes::activity::{reports, LaneActivity, ReportKind};
use omon_gateway::lanes::posts::{self, Post, PostSink, PostState};
use omon_gateway::storage::{get_lane, insert_lane, Database, Lane};
use omon_gateway::work_heartbeat::{Clock, MockClock};
use sqlx::SqlitePool;
use tokio::sync::mpsc;

struct Sink {
    sent: mpsc::UnboundedSender<Post>,
    fail: bool,
}

#[async_trait]
impl PostSink for Sink {
    async fn send(&self, post: &Post) -> omon_gateway::Result<String> {
        self.sent.send(post.clone()).unwrap();
        if self.fail {
            Err(omon_gateway::OmonError::Config(
                "ambiguous transport failure".into(),
            ))
        } else {
            Ok("receipt-123".into())
        }
    }
}

async fn queued_post(database: &Database) -> Post {
    let text: String = sqlx::query_scalar(
        "SELECT text FROM lane_transcript WHERE role = 'thread_post' ORDER BY id LIMIT 1",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence() {
    let (database, _, _) = fixture().await;
    let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    let evidence = format!("완료 {token} {}", "가".repeat(2100));
    omon_gateway::lanes::cli::change_claim(database.pool(), "qa-1", true, &evidence)
        .await
        .unwrap();
    let body: String = sqlx::query_scalar(
        "SELECT body FROM lane_events WHERE lane_key = 'qa-1' AND kind = 'done_claimed'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert!(!body.contains(token));
    assert!(body.starts_with("완료 <redacted> "));
    assert_eq!(body.chars().count(), 2000);
    let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
    assert_eq!(lane.claim_evidence.as_deref(), Some(evidence.as_str()));
    let history: String = sqlx::query_scalar(
        "SELECT reason FROM lane_history WHERE lane_key = 'qa-1' AND to_status = 'done_claimed'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(history, evidence);
}

#[tokio::test]
async fn agent_post_and_event_scrub_token_but_owner_answer_stays_exact() {
    let (database, _, activity) = fixture().await;
    let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    let text = format!("작업 완료 {token}");
    sqlx::query("UPDATE lanes SET thread_channel_id = '123'")
        .execute(database.pool())
        .await
        .unwrap();
    activity
        .captured_item(
            "qa-1",
            &format!("[REPORT] qa-1 | milestone | {text}"),
            0,
            "scrub-post",
        )
        .await
        .unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    assert_eq!(
        posts::drain(database.pool(), &Sink { sent, fail: false })
            .await
            .unwrap(),
        1
    );
    let delivered = received.try_recv().unwrap();
    assert_eq!(delivered.body, "작업 완료 <redacted>");
    let stored = queued_post(&database).await;
    assert_eq!(stored.body, delivered.body);
    assert_eq!(stored.state, PostState::Sent);
    activity
        .captured_item(
            "qa-1",
            &format!("[REPORT] qa-1 | question | {text}"),
            0,
            "scrub-question",
        )
        .await
        .unwrap();
    let body: String = sqlx::query_scalar("SELECT body FROM lane_events WHERE kind = 'question'")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(body, "작업 완료 <redacted>");
    omon_gateway::lanes::cli::change_claim(database.pool(), "qa-1", false, &text)
        .await
        .unwrap();
    let guidance: String = sqlx::query_scalar(
        "SELECT text FROM lane_inbox WHERE lane_key = 'qa-1' AND consumed_at IS NULL",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(guidance, text);
}

#[tokio::test]
async fn configured_hosts_scrub_stored_and_delivered_values_in_isolation() {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "configured_host_subprocess_helper",
                "--ignored",
                "--nocapture",
            ])
            .env("OMON_SCRUB_HOSTS", " private.example , SECOND.EXAMPLE , ")
            .env("HOME", "/Users/qa-owner")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("configured-host subprocess must finish")
    .expect("configured-host subprocess must launch");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[tokio::test]
#[ignore = "invoked by configured_hosts_scrub_stored_and_delivered_values_in_isolation"]
async fn configured_host_subprocess_helper() {
    let (database, _, activity) = fixture().await;
    let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
    let text =
        format!("작업 /Users/qa-owner/repo {token} PRIVATE.EXAMPLE second.example public.example");
    let published = "작업 ~/repo <redacted> <redacted> <redacted> public.example";
    sqlx::query("UPDATE lanes SET title = ?, thread_channel_id = '123' WHERE key = 'qa-1'")
        .bind(&text)
        .execute(database.pool())
        .await
        .unwrap();
    activity
        .captured_item(
            "qa-1",
            &format!("[REPORT] qa-1 | milestone | {text}"),
            0,
            "configured-host-post",
        )
        .await
        .unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    assert_eq!(
        posts::drain(database.pool(), &Sink { sent, fail: false })
            .await
            .unwrap(),
        1
    );
    let delivered = received.try_recv().unwrap();
    let stored = queued_post(&database).await;
    assert_eq!(delivered.body, published);
    assert_eq!(stored.body, delivered.body);
    assert_eq!(stored.state, PostState::Sent);
    assert_eq!(stored.message_id.as_deref(), Some("receipt-123"));
    for body in [&stored.body, &delivered.body] {
        assert!(!body.contains(token));
        assert!(!body.contains("/Users/qa-owner"));
        assert!(!body.to_lowercase().contains("private.example"));
        assert!(!body.to_lowercase().contains("second.example"));
    }
    activity
        .captured_item(
            "qa-1",
            &format!("[REPORT] qa-1 | question | {text}"),
            0,
            "configured-host-question",
        )
        .await
        .unwrap();
    let body: String = sqlx::query_scalar(
        "SELECT body FROM lane_events WHERE lane_key = 'qa-1' AND kind = 'question'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(body, published);
    omon_gateway::lanes::cli::change_claim(database.pool(), "qa-1", false, &text)
        .await
        .unwrap();
    let guidance: String = sqlx::query_scalar(
        "SELECT text FROM lane_inbox WHERE lane_key = 'qa-1' AND consumed_at IS NULL",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(guidance, text);
    let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
    assert_eq!(lane.title, text);
}

#[tokio::test]
async fn acknowledged_post_is_not_sent_again_after_recovery() {
    let (database, _, activity) = fixture().await;
    sqlx::query("UPDATE lanes SET thread_channel_id = '123'")
        .execute(database.pool())
        .await
        .unwrap();
    activity
        .captured_item(
            "qa-1",
            "[REPORT] qa-1 | milestone | /Users/private/file",
            0,
            "item-1",
        )
        .await
        .unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    let sink = Sink { sent, fail: false };
    assert_eq!(posts::drain(database.pool(), &sink).await.unwrap(), 1);
    assert!(!received.try_recv().unwrap().body.contains("/Users/private"));
    posts::recover(database.pool()).await.unwrap();
    assert_eq!(posts::drain(database.pool(), &sink).await.unwrap(), 0);
    let post = queued_post(&database).await;
    assert_eq!(post.state, PostState::Sent);
    assert_eq!(post.message_id.as_deref(), Some("receipt-123"));
    assert!(received.try_recv().is_err());
}

#[tokio::test]
async fn ambiguous_post_stays_uncertain_without_retry() {
    let (database, _, activity) = fixture().await;
    sqlx::query("UPDATE lanes SET thread_channel_id = '123'")
        .execute(database.pool())
        .await
        .unwrap();
    activity
        .captured("qa-1", "[REPORT] qa-1 | milestone | test")
        .await
        .unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    let sink = Sink { sent, fail: true };
    assert_eq!(posts::drain(database.pool(), &sink).await.unwrap(), 0);
    received.try_recv().unwrap();
    assert_eq!(posts::drain(database.pool(), &sink).await.unwrap(), 0);
    let post = queued_post(&database).await;
    assert_eq!(post.state, PostState::Uncertain);
    assert!(post.error.is_some());
    assert!(post.message_id.is_none());
    assert!(received.try_recv().is_err());
}

#[tokio::test]
async fn interrupted_send_becomes_uncertain_on_restart() {
    let (database, _, activity) = fixture().await;
    activity
        .captured("qa-1", "[REPORT] qa-1 | milestone | test")
        .await
        .unwrap();
    sqlx::query("UPDATE lane_transcript SET text = json_set(text, '$.state', 'sending') WHERE role = 'thread_post'")
        .execute(database.pool()).await.unwrap();
    posts::recover(database.pool()).await.unwrap();
    assert_eq!(queued_post(&database).await.state, PostState::Uncertain);
}

#[tokio::test]
async fn stable_item_replay_does_not_duplicate_post() {
    let (database, _, activity) = fixture().await;
    for item in ["one", "one", "two"] {
        activity
            .captured_item("qa-1", "[REPORT] qa-1 | milestone | same text", 0, item)
            .await
            .unwrap();
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM lane_transcript WHERE role = 'thread_post'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn failed_post_enqueue_rolls_back_stale_event_and_latch() {
    let (database, clock, activity) = fixture().await;
    sqlx::query("CREATE TRIGGER reject_post BEFORE INSERT ON lane_transcript WHEN NEW.role = 'thread_post' BEGIN SELECT RAISE(ABORT, 'rejected'); END")
        .execute(database.pool()).await.unwrap();
    clock.advance(Duration::minutes(20));
    assert!(activity.quiet("qa-1", true).await.is_err());
    assert!(events(database.pool()).await.is_empty());
    assert!(get_lane(database.pool(), "qa-1")
        .await
        .unwrap()
        .unwrap()
        .stale_posted_at
        .is_none());
}

#[test]
fn cumulative_snapshots_replace_content_and_finalize_once() {
    let mut capture = omon_gateway::lanes::activity::StreamCapture::default();
    let mut chunk = omon_gateway::StreamChunk {
        stream_id: uuid::Uuid::new_v4(),
        sequence: 0,
        content: "[REPORT] qa-1 | mile".into(),
        is_final: false,
        reply_to: None,
    };
    assert!(capture.accept(&chunk).is_none());
    chunk.sequence = 1;
    chunk.content = "[REPORT] qa-1 | milestone | complete".into();
    chunk.is_final = true;
    assert_eq!(capture.accept(&chunk), Some(chunk.content.as_str()));
    assert!(capture.accept(&chunk).is_none());
    assert_eq!(capture.content, chunk.content);
}

#[tokio::test]
async fn missing_destination_never_calls_sink() {
    let (database, _, activity) = fixture().await;
    activity
        .captured("qa-1", "[REPORT] qa-1 | milestone | test")
        .await
        .unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    assert_eq!(
        posts::drain(database.pool(), &Sink { sent, fail: false })
            .await
            .unwrap(),
        0
    );
    let post = queued_post(&database).await;
    assert_eq!(post.state, PostState::Pending);
    assert!(post.error.is_some());
    assert!(received.try_recv().is_err());
}

#[tokio::test]
async fn old_generation_item_cannot_enqueue_post() {
    let (database, clock, activity) = fixture().await;
    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES ('qa-1', ?, 'closed', 'working', 'reopened')")
        .bind(clock.now()).execute(database.pool()).await.unwrap();
    assert!(
        activity
            .captured_item(
                "qa-1",
                "[REPORT] qa-1 | milestone | obsolete",
                0,
                "old-item"
            )
            .await
            .unwrap()
            .pause
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_transcript")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

struct CaptureSink(mpsc::UnboundedSender<omon_gateway::OutboundAction>);

struct QuestionBackend(sqlx::SqlitePool);

struct TransactionBackend {
    pool: SqlitePool,
    held: mpsc::UnboundedSender<()>,
    release: Arc<tokio::sync::Semaphore>,
}

#[async_trait]
impl omon_gateway::agent::AgentBackend for TransactionBackend {
    async fn run(
        &self,
        session: &mut omon_gateway::SessionContext,
        _: omon_gateway::InboundEvent,
    ) -> omon_gateway::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE lanes SET updated_at = updated_at WHERE key = 'qa-1'")
            .execute(&mut *tx)
            .await?;
        self.held.send(()).unwrap();
        self.release.acquire().await.unwrap().forget();
        tx.commit().await?;
        session.state.metadata.insert(
            "omo_thread_id".into(),
            serde_json::json!("held-turn-thread"),
        );
        omon_gateway::lanes::cli::change_claim(&self.pool, "qa-1", true, "transaction completed")
            .await
    }
}

#[tokio::test]
async fn notification_does_not_suspend_backend_holding_sqlite_connection() {
    let (database, _, _) = fixture().await;
    sqlx::query("UPDATE lanes SET turns_run = 1 WHERE key = 'qa-1'")
        .execute(database.pool())
        .await
        .unwrap();
    let (held, mut observed) = mpsc::unbounded_channel();
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let runner = omon_gateway::lanes::runner::LaneRunner::new(
        database.pool().clone(),
        Arc::new(TransactionBackend {
            pool: database.pool().clone(),
            held,
            release: release.clone(),
        }),
    );
    let notify = Arc::new(tokio::sync::Notify::new());
    let runner_notify = notify.clone();
    let task = tokio::spawn(async move {
        runner
            .run_lane(
                "qa-1",
                runner_notify,
                tokio_util::sync::CancellationToken::new(),
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.recv())
        .await
        .unwrap()
        .unwrap();
    notify.notify_one();
    tokio::task::yield_now().await;
    release.add_permits(1);
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
    assert_eq!(lane.status, "done_claimed");
    assert_eq!(lane.turns_run, 2);
    assert_eq!(lane.omo_thread_id.as_deref(), Some("held-turn-thread"));
}

#[async_trait]
impl omon_gateway::agent::AgentBackend for QuestionBackend {
    async fn run(
        &self,
        session: &mut omon_gateway::SessionContext,
        _: omon_gateway::InboundEvent,
    ) -> omon_gateway::Result<()> {
        use omon_gateway::multiplexer::OutboundDispatcher;
        omon_gateway::lanes::runner::CapturingDispatcher::new(self.0.clone())
            .dispatch(omon_gateway::OutboundAction::SendMessage {
                session: session.key.clone(),
                content: "[REPORT] qa-1 | question | choose A or B".into(),
                reply_to: None,
            })
            .await
    }
}

#[tokio::test]
async fn runner_question_pauses_after_one_turn_without_outcome_lock_deadlock() {
    let (database, _, _) = fixture().await;
    sqlx::query("UPDATE lanes SET turns_run = 1 WHERE key = 'qa-1'")
        .execute(database.pool())
        .await
        .unwrap();
    let runner = omon_gateway::lanes::runner::LaneRunner::new(
        database.pool().clone(),
        Arc::new(QuestionBackend(database.pool().clone())),
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        runner.run_lane(
            "qa-1",
            Arc::new(tokio::sync::Notify::new()),
            tokio_util::sync::CancellationToken::new(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
    assert_eq!(lane.status, "blocked");
    assert_eq!(lane.turns_run, 2);
    assert_eq!(events(database.pool()).await, ["question"]);
}

#[async_trait]
impl omon_gateway::multiplexer::OutboundDispatcher for CaptureSink {
    async fn dispatch(&self, action: omon_gateway::OutboundAction) -> omon_gateway::Result<()> {
        self.0.send(action).unwrap();
        Ok(())
    }
}

#[tokio::test]
async fn backend_captures_completed_lane_items_with_stable_identity() {
    use futures_util::{SinkExt, StreamExt};
    use omon_gateway::agent::{AgentBackend, OmoBackend, OmoBackendConfig};
    use serde_json::{json, Value};
    use tokio_tungstenite::tungstenite::Message;

    let directory = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, mut received) = mpsc::unbounded_channel();
    let peer = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        for response in [
            json!({}),
            json!({"thread":{"id":"thread-12"}}),
            json!({"turn":{"id":"turn-12","status":"inProgress"}}),
        ] {
            let request = socket.next().await.unwrap().unwrap();
            let request: Value = serde_json::from_str(request.to_text().unwrap()).unwrap();
            socket
                .send(Message::text(
                    json!({"jsonrpc":"2.0","id":request["id"],"result":response}).to_string(),
                ))
                .await
                .unwrap();
        }
        for identity in ["item-a", "item-b"] {
            socket.send(Message::text(json!({"method":"item/completed","params":{"threadId":"thread-12","turnId":"turn-12","item":{"id":identity,"type":"agentMessage","text":"[REPORT] qa-1 | milestone | same"}}}).to_string())).await.unwrap();
        }
        socket.send(Message::text(json!({"method":"turn/completed","params":{"threadId":"thread-12","turn":{"id":"turn-12","status":"completed"}}}).to_string())).await.unwrap();
    });
    let backend = OmoBackend::new(
        OmoBackendConfig::new(format!("ws://{address}")),
        Arc::new(CaptureSink(sent)),
    );
    let key =
        omon_gateway::SessionKey::new("lane", None::<String>, "qa-1", None::<String>, "lane:qa-1");
    let mut session = omon_gateway::SessionContext::new(key.clone());
    session
        .state
        .metadata
        .insert("lane_cwd".into(), json!(directory.path()));
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        backend.run(
            &mut session,
            omon_gateway::InboundEvent::message(key, "request", "work"),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    peer.await.unwrap();
    let mut ids = Vec::new();
    while let Ok(action) = received.try_recv() {
        match action {
            omon_gateway::OutboundAction::EditMessage {
                platform_message_id,
                ..
            } => ids.push(platform_message_id),
            omon_gateway::OutboundAction::Stream { chunk, .. } => assert!(!chunk.is_final),
            _ => panic!("unexpected lane action"),
        }
    }
    assert_eq!(
        ids,
        ["lane-item:turn-12:item-a", "lane-item:turn-12:item-b"]
    );
}

async fn fixture() -> (Database, MockClock, LaneActivity) {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let clock = MockClock::new(Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap());
    let mut lane = Lane::new(
        "qa-1",
        "QA",
        "lead",
        "/tmp",
        "/tmp/brief",
        "lane",
        "file done.txt",
    );
    lane.status = "working".into();
    lane.last_activity_at = clock.now();
    insert_lane(database.pool(), &lane).await.unwrap();
    let activity = LaneActivity::new(database.pool().clone(), Arc::new(clock.clone()));
    (database, clock, activity)
}

async fn events(pool: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar("SELECT kind FROM lane_events ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

struct IntentJudge {
    selected: &'static str,
    observed: mpsc::UnboundedSender<String>,
    claim_pool: Option<SqlitePool>,
}

#[async_trait]
impl Judge for IntentJudge {
    async fn choice(
        &self,
        purpose: Purpose,
        state: &str,
        _: &str,
        options: &[&str],
    ) -> Result<Choice, JudgeError> {
        assert_eq!(purpose, Purpose::ReportIntent);
        assert_eq!(options, ["finished", "progress", "blocked_or_question"]);
        self.observed.send(state.into()).unwrap();
        if let Some(pool) = &self.claim_pool {
            sqlx::query("UPDATE lanes SET status = 'done_claimed' WHERE key = 'qa-1'")
                .execute(pool)
                .await
                .unwrap();
        }
        Ok(Choice::new(self.selected))
    }

    async fn noul(&self, _: Purpose, _: &str, _: &str) -> Result<f64, JudgeError> {
        panic!("unexpected noul")
    }

    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> Result<DiffVerdict, JudgeError> {
        panic!("unexpected diff")
    }
}

#[test]
fn parser_rejects_foreign_malformed_and_empty_reports() {
    let input = "[REPORT] other | question | wrong\n[REPORT] qa-1 | unknown | no\n[REPORT] qa-1 | question | \n[REPORT] qa-1 | done\n[REPORT] qa-1 | milestone | A | B";
    let parsed: Vec<_> = reports("qa-1", input).collect();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].kind, ReportKind::Milestone);
    assert_eq!(parsed[0].text, "A | B");
}

#[tokio::test]
async fn question_and_blocker_pause_with_one_persistent_event() {
    for (report, kind) in [("question", "question"), ("blocker", "blocked")] {
        let (database, clock, activity) = fixture().await;
        let text = format!("[REPORT] qa-1 | {report} | pick A or B");
        let decision = activity.captured("qa-1", &text).await.unwrap();
        activity.captured("qa-1", &text).await.unwrap();
        let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
        assert!(decision.pause);
        assert_eq!(lane.status, "blocked");
        assert_eq!(lane.last_report.as_deref(), Some(text.as_str()));
        assert_eq!(lane.last_activity_at, clock.now());
        assert_eq!(events(database.pool()).await, [kind]);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_history")
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn milestone_posts_without_waking_lead() {
    let (database, _, activity) = fixture().await;
    let decision = activity
        .captured("qa-1", "[REPORT] qa-1 | milestone | file written")
        .await
        .unwrap();
    assert_eq!(decision.thread_posts, ["file written"]);
    assert_eq!(decision.report_count, 1);
    assert!(!decision.pause);
    assert!(events(database.pool()).await.is_empty());
}

#[tokio::test]
async fn done_report_requires_claim_and_never_verifies_lane() {
    let (database, _, activity) = fixture().await;
    let decision = activity
        .captured("qa-1", "[REPORT] qa-1 | done | complete")
        .await
        .unwrap();
    assert!(decision.pause);
    assert_eq!(events(database.pool()).await, ["idle_without_claim"]);
    assert_eq!(
        get_lane(database.pool(), "qa-1")
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
}

#[tokio::test]
async fn claimed_lane_suppresses_done_and_question_events() {
    let (database, _, activity) = fixture().await;
    sqlx::query("UPDATE lanes SET status = 'done_claimed' WHERE key = 'qa-1'")
        .execute(database.pool())
        .await
        .unwrap();
    let decision = activity
        .captured(
            "qa-1",
            "[REPORT] qa-1 | done | complete\n[REPORT] qa-1 | question | late question",
        )
        .await
        .unwrap();
    assert!(decision.pause);
    assert!(events(database.pool()).await.is_empty());
}

#[tokio::test]
async fn quiet_threshold_and_restart_are_idempotent() {
    let (database, clock, activity) = fixture().await;
    activity
        .captured("qa-1", "[REPORT] qa-1 | milestone | checkpoint")
        .await
        .unwrap();
    clock.advance(Duration::minutes(20) - Duration::milliseconds(1));
    assert!(activity
        .quiet("qa-1", true)
        .await
        .unwrap()
        .thread_posts
        .is_empty());
    clock.advance(Duration::milliseconds(1));
    let decision = activity.quiet("qa-1", true).await.unwrap();
    let restarted = LaneActivity::new(database.pool().clone(), Arc::new(clock.clone()));
    assert!(restarted
        .quiet("qa-1", true)
        .await
        .unwrap()
        .thread_posts
        .is_empty());
    assert_eq!(
        decision.thread_posts,
        ["[REPORT] qa-1 | milestone | checkpoint"]
    );
    assert_eq!(events(database.pool()).await, ["stale"]);
    assert_eq!(
        get_lane(database.pool(), "qa-1")
            .await
            .unwrap()
            .unwrap()
            .stale_posted_at,
        Some(clock.now())
    );
}

#[tokio::test]
async fn ordinary_activity_rearms_stale_but_idle_turns_never_fire() {
    let (database, clock, activity) = fixture().await;
    clock.advance(Duration::minutes(20));
    activity.quiet("qa-1", true).await.unwrap();
    activity
        .captured("qa-1", "ordinary activity")
        .await
        .unwrap();
    assert_eq!(
        get_lane(database.pool(), "qa-1")
            .await
            .unwrap()
            .unwrap()
            .stale_posted_at,
        None
    );
    clock.advance(Duration::minutes(20));
    assert!(activity
        .quiet("qa-1", false)
        .await
        .unwrap()
        .thread_posts
        .is_empty());
    assert_eq!(
        activity
            .quiet("qa-1", true)
            .await
            .unwrap()
            .thread_posts
            .len(),
        1
    );
    assert_eq!(events(database.pool()).await, ["stale", "stale"]);
}

#[tokio::test]
async fn unreported_intents_are_bounded_scrubbed_and_typed() {
    for (selected, expected) in [
        ("progress", None),
        ("finished", Some("idle_without_claim")),
        ("blocked_or_question", Some("blocked")),
    ] {
        let (database, _, activity) = fixture().await;
        let (observed, mut receive) = mpsc::unbounded_channel();
        let judge = IntentJudge {
            selected,
            observed,
            claim_pool: None,
        };
        let message = format!("/Users/private/project {}", "한".repeat(3000));
        let decision = activity
            .unreported_turn("qa-1", &message, &judge)
            .await
            .unwrap();
        let excerpt = receive.try_recv().unwrap();
        assert_eq!(excerpt.chars().count(), 2000);
        assert!(!excerpt.contains("/Users/private"));
        assert_eq!(decision.pause, expected.is_some());
        assert_eq!(
            events(database.pool()).await,
            expected.into_iter().collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn concurrent_claim_wins_over_judge_response() {
    let (database, _, activity) = fixture().await;
    let (observed, _receive) = mpsc::unbounded_channel();
    let judge = IntentJudge {
        selected: "blocked_or_question",
        observed,
        claim_pool: Some(database.pool().clone()),
    };
    let decision = activity
        .unreported_turn("qa-1", "question", &judge)
        .await
        .unwrap();
    assert!(decision.pause);
    assert!(events(database.pool()).await.is_empty());
    assert_eq!(
        get_lane(database.pool(), "qa-1")
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
}

#[tokio::test]
async fn invalid_judge_choice_leaves_registry_unchanged() {
    let (database, _, activity) = fixture().await;
    let (observed, _receive) = mpsc::unbounded_channel();
    let judge = IntentJudge {
        selected: "unexpected",
        observed,
        claim_pool: None,
    };
    assert!(activity
        .unreported_turn("qa-1", "text", &judge)
        .await
        .is_err());
    assert!(events(database.pool()).await.is_empty());
    assert_eq!(
        get_lane(database.pool(), "qa-1")
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
}

#[tokio::test]
async fn event_insert_failure_rolls_back_block_and_report() {
    let (database, _, activity) = fixture().await;
    sqlx::query("CREATE TRIGGER reject_event BEFORE INSERT ON lane_events BEGIN SELECT RAISE(ABORT, 'test rejection'); END")
        .execute(database.pool()).await.unwrap();
    assert!(activity
        .captured("qa-1", "[REPORT] qa-1 | question | choose")
        .await
        .is_err());
    let lane = get_lane(database.pool(), "qa-1").await.unwrap().unwrap();
    assert_eq!(lane.status, "working");
    assert_eq!(lane.last_report, None);
    assert!(events(database.pool()).await.is_empty());
}

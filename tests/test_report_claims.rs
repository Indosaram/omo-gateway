use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Json, Router};
use chrono::Utc;
use omon_gateway::report::claims::{
    record_and_schedule_claims, recover_stale_report_claims, strip_and_extract_claims,
    ExtractedClaim, ReportClaimRow, StreamClaimFilter,
};
use omon_gateway::slack::{SlackApi, SlackEgress};
use omon_gateway::storage::init_pool;
use omon_gateway::{OutboundAction, OutboundDispatcher, Result, SessionKey, StreamChunk};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

type SentMessage = (SessionKey, String, Option<String>);

#[derive(Default, Clone)]
struct RecordingDispatcher {
    sent: Arc<Mutex<Vec<SentMessage>>>,
}

#[async_trait]
impl OutboundDispatcher for RecordingDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        if let OutboundAction::SendMessage {
            session,
            content,
            reply_to,
        } = action
        {
            self.sent.lock().push((session, content, reply_to));
        }
        Ok(())
    }
}

#[derive(Default)]
struct FakeSlackState {
    posts: Mutex<Vec<Value>>,
    updates: Mutex<Vec<Value>>,
}

async fn fake_slack_post(
    State(state): State<Arc<FakeSlackState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.posts.lock().push(body);
    let idx = state.posts.lock().len();
    Json(json!({ "ok": true, "ts": format!("1700000000.000{idx}") }))
}

async fn fake_slack_update(
    State(state): State<Arc<FakeSlackState>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    state.updates.lock().push(body);
    Json(json!({ "ok": true, "ts": "1700000000.0001" }))
}

#[tokio::test]
async fn a_marker_split_across_two_stream_chunks_is_stripped_and_one_row_is_stored() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let state = Arc::new(FakeSlackState::default());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let st_clone = state.clone();
    tokio::spawn(async move {
        let app = Router::new()
            .route("/chat.postMessage", axum::routing::post(fake_slack_post))
            .route("/chat.update", axum::routing::post(fake_slack_update))
            .with_state(st_clone);
        axum::serve(listener, app).await.unwrap();
    });

    let api = SlackApi::with_base_url("xoxb-test".into(), format!("http://{addr}"));
    let egress = SlackEgress::new(api)
        .with_pool(pool.clone())
        .with_claim_delay(Duration::from_secs(60));

    let session = SessionKey::new("slack", Some("T1"), "C1", Some("1700.01"), "U1");
    let stream_id = Uuid::new_v4();

    let chunk1 = StreamChunk {
        stream_id,
        sequence: 1,
        content: "다운로드 수는 42건입니다 [[measure [\"echo\",".into(),
        is_final: false,
        reply_to: None,
    };
    let chunk2 = StreamChunk {
        stream_id,
        sequence: 2,
        content: "다운로드 수는 42건입니다 [[measure [\"echo\",\"42\"] expect~\"^42$\"]]".into(),
        is_final: true,
        reply_to: None,
    };

    egress
        .dispatch(OutboundAction::Stream {
            session: session.clone(),
            chunk: chunk1,
        })
        .await
        .unwrap();
    egress
        .dispatch(OutboundAction::Stream {
            session: session.clone(),
            chunk: chunk2,
        })
        .await
        .unwrap();

    for post in state.posts.lock().iter() {
        let text = post.get("text").and_then(Value::as_str).unwrap_or("");
        assert!(!text.contains("[[measure"), "leaked marker in post: {text}");
    }
    for upd in state.updates.lock().iter() {
        let text = upd.get("text").and_then(Value::as_str).unwrap_or("");
        assert!(
            !text.contains("[[measure"),
            "leaked marker in update: {text}"
        );
    }

    let rows: Vec<ReportClaimRow> = sqlx::query_as("SELECT * FROM report_claims")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sentence, "다운로드 수는 42건입니다");
    assert_eq!(rows[0].expect_regex, "^42$");

    let filter = StreamClaimFilter::new();
    let (s1, c1) = filter.filter_chunk("s", stream_id, "줄 수는 10줄 [[measure [\"wc\"", false);
    assert!(!s1.contains("[[measure"));
    assert!(c1.is_empty());
    let (s2, c2) = filter.filter_chunk(
        "s",
        stream_id,
        "줄 수는 10줄 [[measure [\"wc\",\"-l\"] expect~\"10\"]]",
        true,
    );
    assert_eq!(s2, "줄 수는 10줄");
    assert_eq!(c2.len(), 1);
}

#[tokio::test]
async fn b_mismatch_posts_exactly_one_correction_in_thread() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let disp = RecordingDispatcher::default();
    let session = SessionKey::new("slack", Some("T1"), "C1", Some("1700.01"), "U1");

    let claims = vec![ExtractedClaim {
        sentence: "현재 줄 수는 154줄입니다".into(),
        argv: vec!["echo".into(), "1027".into()],
        expect_regex: "^154$".into(),
    }];

    let ids = record_and_schedule_claims(
        pool.clone(),
        Arc::new(disp.clone()),
        session.clone(),
        "1700000000.0001".into(),
        claims,
        PathBuf::from("."),
        Duration::ZERO,
    )
    .await
    .unwrap();
    assert_eq!(ids.len(), 1);

    for _ in 0..50 {
        let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
            .bind(ids[0])
            .fetch_one(&pool)
            .await
            .unwrap();
        if row.state != "pending" {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
        .bind(ids[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.state, "corrected");
    assert_eq!(row.measured_output.as_deref(), Some("1027"));

    let sent = disp.sent.lock();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].1,
        "정정: 현재 줄 수는 154줄입니다 -> 실제 측정값: 1027"
    );
    assert_eq!(sent[0].2.as_deref(), Some("1700000000.0001"));
}

#[tokio::test]
async fn c_match_posts_nothing() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let disp = RecordingDispatcher::default();
    let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");

    let claims = vec![ExtractedClaim {
        sentence: "상태는 정상(42)입니다".into(),
        argv: vec!["echo".into(), "42".into()],
        expect_regex: "^42$".into(),
    }];

    let ids = record_and_schedule_claims(
        pool.clone(),
        Arc::new(disp.clone()),
        session,
        "1700000000.0002".into(),
        claims,
        PathBuf::from("."),
        Duration::ZERO,
    )
    .await
    .unwrap();

    for _ in 0..50 {
        let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
            .bind(ids[0])
            .fetch_one(&pool)
            .await
            .unwrap();
        if row.state != "pending" {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
        .bind(ids[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.state, "confirmed");
    assert!(disp.sent.lock().is_empty());
}

#[tokio::test]
async fn d_git_push_argv_is_refused_and_not_run() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let disp = RecordingDispatcher::default();
    let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");

    let claims = vec![ExtractedClaim {
        sentence: "브랜치를 푸시했습니다".into(),
        argv: vec![
            "git".into(),
            "push".into(),
            "origin".into(),
            "lane/x".into(),
        ],
        expect_regex: ".*".into(),
    }];

    let ids = record_and_schedule_claims(
        pool.clone(),
        Arc::new(disp.clone()),
        session,
        "1700000000.0003".into(),
        claims,
        PathBuf::from("."),
        Duration::ZERO,
    )
    .await
    .unwrap();

    let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
        .bind(ids[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.state, "refused");
    assert!(disp.sent.lock().is_empty());
}

#[tokio::test]
async fn e_spawn_failure_gives_unmeasurable() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let disp = RecordingDispatcher::default();
    let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");

    let claims = vec![ExtractedClaim {
        sentence: "파일을 읽었습니다".into(),
        // Only bare allowlisted program names run, so a failing run stands in for a spawn failure.
        argv: vec!["cat".into(), "/nonexistent-dir-99999/file".into()],
        expect_regex: ".*".into(),
    }];

    let ids = record_and_schedule_claims(
        pool.clone(),
        Arc::new(disp.clone()),
        session,
        "1700000000.0004".into(),
        claims,
        PathBuf::from("."),
        Duration::ZERO,
    )
    .await
    .unwrap();

    for _ in 0..50 {
        let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
            .bind(ids[0])
            .fetch_one(&pool)
            .await
            .unwrap();
        if row.state != "pending" {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
        .bind(ids[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.state, "unmeasurable");
    assert!(disp.sent.lock().is_empty());
}

#[tokio::test]
async fn f_restart_with_stale_pending_row_gives_unmeasurable() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let stale_time = Utc::now() - chrono::Duration::minutes(15);

    sqlx::query(
        "INSERT INTO report_claims
            (session_key, platform_message_id, sentence, argv_json, expect_regex, state, created_at)
         VALUES ('slack:T1:C1::U1', '1700.05', '오래된 수치', '[\"ls\"]', '.*', 'pending', ?)",
    )
    .bind(stale_time)
    .execute(&pool)
    .await
    .unwrap();

    let updated = recover_stale_report_claims(&pool, Utc::now())
        .await
        .unwrap();
    assert_eq!(updated, 1);

    let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.state, "unmeasurable");

    let (stripped, parsed) =
        strip_and_extract_claims("테스트 1건 통과 [[measure [\"ls\"] expect~\".*\"]] 완료");
    assert_eq!(stripped, "테스트 1건 통과  완료");
    assert_eq!(parsed.len(), 1);
}

#[tokio::test]
async fn g_only_allowlisted_read_only_programs_are_remeasured() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let disp = RecordingDispatcher::default();
    let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "U1");
    let refused: Vec<Vec<&str>> = vec![
        vec!["cat", "/Users/x/.ssh/id_rsa"],
        vec!["head", ".env"],
        vec!["tail", "config/api_token.txt"],
        vec!["rg", "--pre", "sh", "x"],
        vec!["git", "-c", "core.fsmonitor=rm -rf ~", "status"],
        vec!["git", "status", "--config-env=x=y"],
        vec!["git", "branch", "-D", "main"],
        vec!["git", "push", "origin", "main"],
        vec!["cargo", "test"],
        vec!["/tmp/evil/cat", "file"],
    ];
    let claims: Vec<ExtractedClaim> = refused
        .iter()
        .map(|argv| ExtractedClaim {
            sentence: "수치".into(),
            argv: argv.iter().map(|s| s.to_string()).collect(),
            expect_regex: ".*".into(),
        })
        .collect();
    let ids = record_and_schedule_claims(
        pool.clone(),
        Arc::new(disp.clone()),
        session,
        "1700000000.0007".into(),
        claims,
        PathBuf::from("."),
        Duration::ZERO,
    )
    .await
    .unwrap();
    assert_eq!(ids.len(), refused.len());
    for (id, argv) in ids.iter().zip(&refused) {
        let row: ReportClaimRow = sqlx::query_as("SELECT * FROM report_claims WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.state, "refused", "argv {argv:?} must be refused");
    }
    assert!(disp.sent.lock().is_empty());
}

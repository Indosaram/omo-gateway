use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use omon_gateway::cron::remediate::{
    Detail, LaneRequest, Problem, Remediator, RunReceipt, Runtime,
};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::{CronJobSpec, CronScheduler, Database, Result, ShellAndPayloadTaskExecutor};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SECRET: &str = "Authorization: Bearer secret-auth-token-123";

struct FakeJudge(&'static str);
#[async_trait]
impl Judge for FakeJudge {
    async fn choice(
        &self,
        _: Purpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        Ok(Choice::new(self.0))
    }
    async fn noul(&self, _: Purpose, _: &str, _: &str) -> std::result::Result<f64, JudgeError> {
        Ok(0.0)
    }
    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        Ok(DiffVerdict::new(1.0, 1.0, 0.0))
    }
}

#[derive(Default)]
struct RecordingRuntime {
    lanes: Mutex<Vec<LaneRequest>>,
    reruns: Mutex<usize>,
    outcomes: Mutex<Vec<String>>,
    failed_receipt: Mutex<bool>,
}
#[async_trait]
impl Runtime for RecordingRuntime {
    async fn open_lane(&self, request: LaneRequest, _: &dyn Judge) -> Result<()> {
        self.lanes.lock().unwrap().push(request);
        Ok(())
    }
    async fn rerun(&self, job: &str) -> Result<RunReceipt> {
        *self.reruns.lock().unwrap() += 1;
        let failed = *self.failed_receipt.lock().unwrap();
        Ok(RunReceipt {
            job_id: job.into(),
            run_id: "rerun".into(),
            status: if failed { "failed" } else { "succeeded" }.into(),
            output: Some("ok".into()),
            last_delivery_error: None,
            error: failed.then(|| "still broken".into()),
        })
    }
    async fn quota_check(&self) -> Result<Value> {
        Ok(json!({}))
    }
    async fn log_excerpt(&self, _: &str) -> Result<String> {
        Ok(SECRET.into())
    }
    async fn outcome(&self, _: &Problem, message: &str) -> Result<()> {
        self.outcomes.lock().unwrap().push(message.to_owned());
        Ok(())
    }
}

async fn fixture(
    payload: Value,
    choice: &'static str,
    cause: &str,
) -> (Remediator, Arc<RecordingRuntime>, i64) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
    let pool = db.pool().clone();
    let scheduler = CronScheduler::new(pool.clone(), Arc::new(ShellAndPayloadTaskExecutor));
    scheduler
        .register_with_id("fixture", CronJobSpec::new("interval:1h", payload))
        .await
        .unwrap();
    let id = sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,created_at,updated_at) VALUES('fixture','failed','fixture',?,'open',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)")
        .bind(cause)
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid();
    let runtime = Arc::new(RecordingRuntime::default());
    (
        Remediator {
            pool,
            judge: Arc::new(FakeJudge(choice)),
            runtime: runtime.clone(),
        },
        runtime,
        id,
    )
}

fn token_from(runtime: &RecordingRuntime) -> String {
    let outcomes = runtime.outcomes.lock().unwrap();
    let message = outcomes.last().expect("owner message recorded");
    message
        .split("--owner-token ")
        .nth(1)
        .expect("owner message carries the approval token")
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

async fn status_of(remediator: &Remediator, id: i64) -> String {
    sqlx::query_scalar("SELECT status FROM cron_problems WHERE id=?")
        .bind(id)
        .fetch_one(&remediator.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn approval_requires_the_one_time_owner_token() {
    let (remediator, runtime, id) = fixture(json!({}), "needs_owner_login_or_quota", "login").await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(status_of(&remediator, id).await, "needs_owner");
    let token = token_from(&runtime);

    for wrong in ["", "bogus", " "] {
        let error = remediator
            .approve_rerun_with_token(id, wrong)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("token rejected"), "{error}");
    }
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
    assert_eq!(status_of(&remediator, id).await, "needs_owner");

    let receipt = remediator
        .approve_rerun_with_token(id, &token)
        .await
        .unwrap()
        .expect("valid token reruns");
    assert_eq!(receipt.status, "succeeded");
    assert_eq!(*runtime.reruns.lock().unwrap(), 1);
    assert!(remediator
        .approve_rerun_with_token(id, &token)
        .await
        .is_err());
}

#[tokio::test]
async fn approval_after_exhausted_budget_grants_one_more_attempt() {
    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix", "boom").await;
    let detail = Detail {
        cause: "boom".into(),
        reruns: 3,
        ..Detail::default()
    };
    sqlx::query("UPDATE cron_problems SET detail=?,status='remediating' WHERE id=?")
        .bind(serde_json::to_string(&detail).unwrap())
        .bind(id)
        .execute(&remediator.pool)
        .await
        .unwrap();
    assert!(remediator.verify_once(id).await.unwrap().is_none());
    assert_eq!(status_of(&remediator, id).await, "needs_owner");
    let token = token_from(&runtime);
    let receipt = remediator.approve_rerun_with_token(id, &token).await;
    assert!(receipt.unwrap().is_some(), "approval must not be a no-op");
    assert_eq!(*runtime.reruns.lock().unwrap(), 1);
    assert_eq!(status_of(&remediator, id).await, "fixed");
}

#[tokio::test]
async fn stale_remediating_rows_without_a_live_lane_are_escalated() {
    let (remediator, runtime, stale_id) = fixture(json!({}), "script_bug_needs_fix", "boom").await;
    let old = chrono::Utc::now() - chrono::TimeDelta::hours(3);
    sqlx::query("UPDATE cron_problems SET status='remediating',updated_at=? WHERE id=?")
        .bind(old)
        .bind(stale_id)
        .execute(&remediator.pool)
        .await
        .unwrap();
    let fresh_id = sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,created_at,updated_at) VALUES('fixture','stuck','fresh','x','remediating',?,?)")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(&remediator.pool)
        .await
        .unwrap()
        .last_insert_rowid();
    let mut lane = omon_gateway::storage::Lane::new(
        "cron-fix-fixture-live",
        "fix",
        "discord:dm:owner",
        "/tmp",
        "/tmp/brief",
        "lane:fixture",
        "file /tmp/ready",
    );
    lane.status = "working".into();
    omon_gateway::storage::insert_lane(&remediator.pool, &lane)
        .await
        .unwrap();
    let laned_id = sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,lane_key,created_at,updated_at) VALUES('fixture','stuck','laned','x','remediating',?,?,?)")
        .bind(&lane.key)
        .bind(old)
        .bind(old)
        .execute(&remediator.pool)
        .await
        .unwrap()
        .last_insert_rowid();

    assert_eq!(remediator.sweep_stale_remediating().await.unwrap(), 1);
    assert_eq!(status_of(&remediator, stale_id).await, "needs_owner");
    assert_eq!(status_of(&remediator, fresh_id).await, "remediating");
    assert_eq!(status_of(&remediator, laned_id).await, "remediating");
    assert_eq!(runtime.outcomes.lock().unwrap().len(), 1);
    assert_eq!(remediator.sweep_stale_remediating().await.unwrap(), 0);
}

#[tokio::test]
async fn failed_transient_rerun_escalates_exactly_once() {
    let (remediator, runtime, id) = fixture(json!({}), "transient_retry_once", "flaky").await;
    *runtime.failed_receipt.lock().unwrap() = true;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(status_of(&remediator, id).await, "needs_owner");
    let outcomes = runtime.outcomes.lock().unwrap();
    assert_eq!(outcomes.len(), 1, "{outcomes:?}");
    assert!(outcomes[0].contains("Verification failed"));
}

#[tokio::test]
async fn error_text_is_scrubbed_in_inbox_brief_and_digest() {
    let (remediator, runtime, id) = fixture(json!({}), "needs_owner_login_or_quota", SECRET).await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let snippet: String = sqlx::query_scalar("SELECT snippet FROM inbox_items WHERE id=?")
        .bind(format!("cron:{id}"))
        .fetch_one(&remediator.pool)
        .await
        .unwrap();
    assert!(!snippet.contains("secret-auth-token-123"), "{snippet}");
    assert!(!runtime
        .outcomes
        .lock()
        .unwrap()
        .iter()
        .any(|message| message.contains("secret-auth-token-123")));

    let digest = omon_gateway::cron::report::digest_section(
        &remediator.pool,
        chrono::Utc::now() + chrono::TimeDelta::minutes(1),
    )
    .await
    .unwrap()
    .expect("recent problem appears in the digest");
    assert!(!digest.contains("secret-auth-token-123"), "{digest}");

    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix", SECRET).await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let lanes = runtime.lanes.lock().unwrap();
    assert!(!lanes[0].brief.contains("secret-auth-token-123"));
}

#[tokio::test]
async fn health_output_never_exposes_the_approval_digest() {
    let (remediator, _runtime, id) =
        fixture(json!({}), "needs_owner_login_or_quota", "login").await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let stored: String = sqlx::query_scalar("SELECT detail FROM cron_problems WHERE id=?")
        .bind(id)
        .fetch_one(&remediator.pool)
        .await
        .unwrap();
    assert!(stored.contains("approval_hash"));
    let report = omon_gateway::cron::report::health(&remediator.pool)
        .await
        .unwrap();
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("approval_hash"), "{json}");
}

async fn insert_failed_run(pool: &sqlx::SqlitePool, run: &str, error: &str) {
    sqlx::query("INSERT INTO cron_runs(run_id,job_id,claim_token,lease_expires_at,attempt,status,started_at,completed_at,error) VALUES(?,'fixture',?,'2026-10-11T00:05:00Z',1,'failed','2026-10-11T00:00:00Z','2026-10-11T00:01:00Z',?)")
        .bind(run)
        .bind(format!("claim-{run}"))
        .bind(error)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn varying_error_noise_maps_to_one_problem_and_stale_remediating_reopens() {
    let (remediator, _runtime, id) = fixture(json!({}), "script_bug_needs_fix", "seed").await;
    let pool = &remediator.pool;
    sqlx::query("DELETE FROM cron_problems WHERE id=?")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    insert_failed_run(
        pool,
        "r1",
        "cron script failed with Some(1): 2026-10-13T01:02:03Z pid 4242 connect refused",
    )
    .await;
    omon_gateway::cron::health::on_run_finished(pool, "fixture", "r1", false)
        .await
        .unwrap();
    insert_failed_run(
        pool,
        "r2",
        "cron script failed with Some(1): 2026-10-13T05:06:07Z pid 9 connect refused",
    )
    .await;
    omon_gateway::cron::health::on_run_finished(pool, "fixture", "r2", false)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_problems")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "noise in the error text must not fan out problems"
    );
    sqlx::query("UPDATE cron_problems SET status='remediating',updated_at=?")
        .bind(chrono::Utc::now())
        .execute(pool)
        .await
        .unwrap();
    insert_failed_run(
        pool,
        "r3",
        "cron script failed with Some(1): 2026-10-14T05:06:07Z pid 77 connect refused",
    )
    .await;
    omon_gateway::cron::health::on_run_finished(pool, "fixture", "r3", false)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM cron_problems")
            .fetch_one(pool)
            .await
            .unwrap(),
        "remediating"
    );
    sqlx::query("UPDATE cron_problems SET updated_at=?")
        .bind(chrono::Utc::now() - chrono::TimeDelta::hours(3))
        .execute(pool)
        .await
        .unwrap();
    insert_failed_run(
        pool,
        "r4",
        "cron script failed with Some(1): 2026-10-15T05:06:07Z pid 78 connect refused",
    )
    .await;
    omon_gateway::cron::health::on_run_finished(pool, "fixture", "r4", false)
        .await
        .unwrap();
    let (status, run): (String, String) = sqlx::query_as("SELECT status,run_id FROM cron_problems")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!((status.as_str(), run.as_str()), ("open", "r4"));
}

#[tokio::test]
async fn native_socket_rejects_tokenless_approval_and_bounds_request_size() {
    let (remediator, _runtime, id) =
        fixture(json!({}), "needs_owner_login_or_quota", "login").await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let pool = remediator.pool.clone();
    omon_gateway::cron::remediate::install(Arc::new(remediator)).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cron.sock");
    let scheduler = CronScheduler::new(pool.clone(), Arc::new(ShellAndPayloadTaskExecutor));
    let server = omon_gateway::cron::report::NativeServer::bind(path.clone())
        .await
        .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    let shutdown = tokio_util::sync::CancellationToken::new();
    let serving = tokio::spawn(server.serve(pool.clone(), scheduler, shutdown.clone()));

    async fn ask(path: &Path, payload: Vec<u8>) -> Option<Value> {
        let mut stream = tokio::net::UnixStream::connect(path).await.unwrap();
        let _ = stream.write_all(&payload).await;
        let mut response = String::new();
        let read = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            BufReader::new(stream).read_line(&mut response),
        )
        .await
        .expect("native socket answers or closes");
        match read {
            Ok(n) if n > 0 => Some(serde_json::from_str(response.trim()).unwrap()),
            _ => None,
        }
    }

    let reply = ask(
        &path,
        format!("{}\n", json!({"approve_problem": id})).into_bytes(),
    )
    .await
    .expect("tokenless approval gets an explicit rejection");
    assert_eq!(reply["status"], "failed");
    assert!(reply["error"].as_str().unwrap().contains("token rejected"));

    if let Some(reply) = ask(&path, vec![b'x'; 10_000]).await {
        assert_eq!(reply["status"], "failed");
    }
    let reply = ask(
        &path,
        format!(
            "{}\n",
            json!({"approve_problem": id, "owner_token": "nope"})
        )
        .into_bytes(),
    )
    .await
    .expect("server keeps serving after an oversized request");
    assert_eq!(reply["status"], "failed");

    shutdown.cancel();
    tokio::time::timeout(std::time::Duration::from_secs(20), serving)
        .await
        .expect("shutdown completes")
        .unwrap()
        .unwrap();
}

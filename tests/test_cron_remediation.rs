use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use omon_gateway::cron::remediate::{
    Detail, LaneRequest, Problem, Remediator, RunReceipt, Runtime,
};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::{CronJobSpec, CronScheduler, Database, Result, ShellAndPayloadTaskExecutor};
use serde_json::{json, Value};

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
struct FakeRuntime {
    lanes: Mutex<Vec<LaneRequest>>,
    reruns: Mutex<usize>,
    quota: Mutex<usize>,
    log_error: Mutex<bool>,
    rerun_error: Mutex<bool>,
    lane_error: Mutex<bool>,
    logs: Mutex<Option<String>>,
    receipt: Mutex<Option<RunReceipt>>,
}
#[async_trait]
impl Runtime for FakeRuntime {
    async fn open_lane(&self, request: LaneRequest, _: &dyn Judge) -> Result<()> {
        if *self.lane_error.lock().unwrap() {
            return Err(omon_gateway::OmonError::Config("lane fixture error".into()));
        }
        self.lanes.lock().unwrap().push(request);
        Ok(())
    }
    async fn rerun(&self, job: &str) -> Result<RunReceipt> {
        *self.reruns.lock().unwrap() += 1;
        if *self.rerun_error.lock().unwrap() {
            return Err(omon_gateway::OmonError::Config(
                "rerun fixture error".into(),
            ));
        }
        if let Some(receipt) = self.receipt.lock().unwrap().clone() {
            return Ok(receipt);
        }
        Ok(RunReceipt {
            job_id: job.into(),
            run_id: "verified-run".into(),
            status: "succeeded".into(),
            output: Some("count=7".into()),
            last_delivery_error: None,
            error: None,
        })
    }
    async fn quota_check(&self) -> Result<Value> {
        *self.quota.lock().unwrap() += 1;
        Ok(json!({"remaining":0}))
    }
    async fn log_excerpt(&self, _: &str) -> Result<String> {
        if *self.log_error.lock().unwrap() {
            return Err(omon_gateway::OmonError::Config("log fixture error".into()));
        }
        Ok(self
            .logs
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "run-1 traceback".into()))
    }
    async fn outcome(&self, _: &Problem, _: &str) -> Result<()> {
        Ok(())
    }
}

async fn fixture(payload: Value, choice: &'static str) -> (Remediator, Arc<FakeRuntime>, i64) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(db.pool()).await.unwrap();
    let pool = db.pool().clone();
    let scheduler = CronScheduler::new(pool.clone(), Arc::new(ShellAndPayloadTaskExecutor));
    scheduler
        .register_with_id("fixture", CronJobSpec::new("interval:1h", payload))
        .await
        .unwrap();
    let id = sqlx::query("INSERT INTO cron_problems(job_id,kind,signature,detail,status,created_at,updated_at) VALUES('fixture','failed','fixture','traceback','open',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)")
        .execute(&pool).await.unwrap().last_insert_rowid();
    let runtime = Arc::new(FakeRuntime::default());
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

#[tokio::test]
async fn script_failure_opens_one_lane_with_native_done_signal() {
    let (remediator, runtime, id) =
        fixture(json!({"script":"/tmp/fix.py"}), "script_bug_needs_fix").await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let lanes = runtime.lanes.lock().unwrap();
    assert_eq!(lanes.len(), 1);
    assert_eq!(lanes[0].cwd, Path::new("/tmp"));
    assert!(lanes[0].brief.contains("traceback"));
    assert!(lanes[0].brief.contains("run-1 traceback"));
    let signals = omon_gateway::lanes::signals::parse_signals(&lanes[0].done).unwrap();
    assert!(
        matches!(&signals[0], omon_gateway::lanes::signals::DoneSignal::Command { argv, .. } if argv == &["omo-gateway", "cron", "run-now", "fixture", "--wait", "--json"])
    );
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
}

#[tokio::test]
async fn lane_cwd_uses_job_cwd_and_script_command_line() {
    // Live QA: a job with cwd + "python3 /dir/job.py" produced cwd "/usr/bin" (lane I/O error).
    let (remediator, runtime, id) = fixture(
        json!({"cwd":"/tmp/qa-cron-fixture","script":"/usr/bin/python3 /tmp/qa-cron-fixture/job.py"}),
        "script_bug_needs_fix",
    )
    .await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(
        runtime.lanes.lock().unwrap()[0].cwd,
        Path::new("/tmp/qa-cron-fixture")
    );

    let (remediator, runtime, id) = fixture(
        json!({"script":"/usr/bin/python3 /tmp/qa-cron-fixture/job.py"}),
        "script_bug_needs_fix",
    )
    .await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(
        runtime.lanes.lock().unwrap()[0].cwd,
        Path::new("/tmp/qa-cron-fixture")
    );
}

#[tokio::test]
async fn publishing_prepares_file_and_owner_task_without_rerun() {
    let (remediator, runtime, id) =
        fixture(json!({"external_effect":true}), "script_bug_needs_fix").await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
    {
        let lanes = runtime.lanes.lock().unwrap();
        let signals = omon_gateway::lanes::signals::parse_signals(&lanes[0].done).unwrap();
        assert!(
            matches!(&signals[0], omon_gateway::lanes::signals::DoneSignal::File { path, .. } if path.extension().is_some_and(|extension| extension == "ready"))
        );
    }
    let class: String = sqlx::query_scalar("SELECT d.class FROM inbox_decisions d JOIN inbox_items i ON i.id=d.item_id WHERE i.source='cron'").fetch_one(&remediator.pool).await.unwrap();
    assert_eq!(class, "owner");
}

#[tokio::test]
async fn fourth_attempt_needs_owner_without_execution() {
    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix").await;
    let detail = Detail {
        cause: "error".into(),
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
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
    let status: String = sqlx::query_scalar("SELECT status FROM cron_problems WHERE id=?")
        .bind(id)
        .fetch_one(&remediator.pool)
        .await
        .unwrap();
    assert_eq!(status, "needs_owner");
}

#[tokio::test]
async fn natural_success_verifies_lane_without_execution() {
    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix").await;
    let mut lane = omon_gateway::storage::Lane::new(
        "cron-fix-fixture-abcd",
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
    sqlx::query("UPDATE cron_problems SET status='fixed',lane_key=? WHERE id=?")
        .bind(&lane.key)
        .bind(id)
        .execute(&remediator.pool)
        .await
        .unwrap();
    omon_gateway::cron::remediate::reconcile_fixed(&remediator.pool, "fixture")
        .await
        .unwrap();
    assert_eq!(
        omon_gateway::storage::get_lane(&remediator.pool, &lane.key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "verified_done"
    );
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
}

#[tokio::test]
async fn transient_uses_one_machine_verified_rerun_and_no_lane() {
    let (remediator, runtime, id) = fixture(
        json!({"expect":{"output_regex":"count=7"}}),
        "transient_retry_once",
    )
    .await;
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(*runtime.reruns.lock().unwrap(), 1);
    assert!(runtime.lanes.lock().unwrap().is_empty());
    let status: String = sqlx::query_scalar("SELECT status FROM cron_problems WHERE id=?")
        .bind(id)
        .fetch_one(&remediator.pool)
        .await
        .unwrap();
    assert_eq!(status, "fixed");
}

#[tokio::test]
async fn quota_diagnosis_checks_mahoquot_before_owner_task() {
    let (remediator, runtime, id) = fixture(json!({}), "needs_owner_login_or_quota").await;
    sqlx::query("UPDATE cron_problems SET detail='429 quota' WHERE id=?")
        .bind(id)
        .execute(&remediator.pool)
        .await
        .unwrap();
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    assert_eq!(*runtime.quota.lock().unwrap(), 1);
    assert!(runtime.lanes.lock().unwrap().is_empty());
    assert_eq!(*runtime.reruns.lock().unwrap(), 0);
}

#[tokio::test]
async fn diagnostic_errors_end_with_durable_failure_and_no_phantom_lane() {
    for stage in ["log", "lane", "runs", "choice"] {
        let (remediator, runtime, id) = fixture(
            json!({}),
            if stage == "choice" {
                "invalid"
            } else {
                "script_bug_needs_fix"
            },
        )
        .await;
        *runtime.log_error.lock().unwrap() = stage == "log";
        *runtime.lane_error.lock().unwrap() = stage == "lane";
        if stage == "runs" {
            sqlx::query("DROP TABLE cron_runs")
                .execute(&remediator.pool)
                .await
                .unwrap();
        }
        let error = remediator
            .on_problem(id, "discord:dm:owner")
            .await
            .unwrap_err();
        let (status, text, lane): (String, String, Option<String>) =
            sqlx::query_as("SELECT status,detail,lane_key FROM cron_problems WHERE id=?")
                .bind(id)
                .fetch_one(&remediator.pool)
                .await
                .unwrap();
        assert_eq!(status, "needs_owner");
        assert!(lane.is_none());
        let evidence: Value =
            serde_json::from_str(&Detail::parse(&text).evidence.unwrap()).unwrap();
        assert_eq!(evidence["status"], "failed");
        assert_eq!(evidence["error"], error.to_string());
        let (class, action): (String, String) =
            sqlx::query_as("SELECT class,proposed_action FROM inbox_decisions WHERE item_id=?")
                .bind(format!("cron:{id}"))
                .fetch_one(&remediator.pool)
                .await
                .unwrap();
        assert_eq!(class, "owner");
        assert!(action.contains(&error.to_string()));
        remediator.on_problem(id, "discord:dm:owner").await.unwrap();
        assert!(runtime.lanes.lock().unwrap().is_empty());
        assert_eq!(*runtime.reruns.lock().unwrap(), 0);
    }
}

#[tokio::test]
async fn rerun_errors_preserve_reserved_attempt_and_require_owner_without_retry() {
    for entry in ["transient", "settle", "approval"] {
        let (remediator, runtime, id) = fixture(json!({}), "transient_retry_once").await;
        *runtime.rerun_error.lock().unwrap() = true;
        let error = match entry {
            "transient" => remediator
                .on_problem(id, "discord:dm:owner")
                .await
                .unwrap_err(),
            "approval" => {
                sqlx::query("UPDATE cron_problems SET status='needs_owner' WHERE id=?")
                    .bind(id)
                    .execute(&remediator.pool)
                    .await
                    .unwrap();
                remediator.approve_rerun(id).await.unwrap_err()
            }
            _ => {
                sqlx::query("UPDATE cron_problems SET status='remediating' WHERE id=?")
                    .bind(id)
                    .execute(&remediator.pool)
                    .await
                    .unwrap();
                remediator.verify_once(id).await.unwrap_err()
            }
        };
        let (status, text): (String, String) =
            sqlx::query_as("SELECT status,detail FROM cron_problems WHERE id=?")
                .bind(id)
                .fetch_one(&remediator.pool)
                .await
                .unwrap();
        assert_eq!(status, "needs_owner");
        let detail = Detail::parse(&text);
        assert_eq!(detail.reruns, 1);
        let evidence: Value = serde_json::from_str(&detail.evidence.unwrap()).unwrap();
        assert_eq!(evidence["status"], "failed");
        assert_eq!(evidence["stage"], "verification");
        assert_eq!(evidence["error"], error.to_string());
        assert!(remediator.verify_once(id).await.unwrap().is_none());
        remediator.on_problem(id, "discord:dm:owner").await.unwrap();
        assert_eq!(*runtime.reruns.lock().unwrap(), 1);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inbox_decisions WHERE item_id=?")
            .bind(format!("cron:{id}"))
            .fetch_one(&remediator.pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn invalid_verification_regex_ends_with_failed_evidence() {
    let (remediator, runtime, id) = fixture(
        json!({"expect":{"output_regex":"["}}),
        "transient_retry_once",
    )
    .await;
    assert!(remediator.on_problem(id, "discord:dm:owner").await.is_err());
    let (status, text): (String, String) =
        sqlx::query_as("SELECT status,detail FROM cron_problems WHERE id=?")
            .bind(id)
            .fetch_one(&remediator.pool)
            .await
            .unwrap();
    assert_eq!(status, "needs_owner");
    let detail = Detail::parse(&text);
    assert_eq!(detail.reruns, 1);
    let evidence: Value = serde_json::from_str(&detail.evidence.unwrap()).unwrap();
    assert_eq!(evidence["status"], "failed");
    assert!(remediator.verify_once(id).await.unwrap().is_none());
    assert_eq!(*runtime.reruns.lock().unwrap(), 1);
}

#[tokio::test]
async fn failed_machine_receipt_requires_owner_instead_of_stalling() {
    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix").await;
    *runtime.receipt.lock().unwrap() = Some(RunReceipt {
        job_id: "fixture".into(),
        run_id: "failed-run".into(),
        status: "failed".into(),
        output: Some("agent claims success".into()),
        last_delivery_error: None,
        error: Some("machine failure".into()),
    });
    let receipt = remediator.verify_once(id).await.unwrap().unwrap();
    assert_eq!(receipt.status, "failed");
    let (status, text): (String, String) =
        sqlx::query_as("SELECT status,detail FROM cron_problems WHERE id=?")
            .bind(id)
            .fetch_one(&remediator.pool)
            .await
            .unwrap();
    assert_eq!(status, "needs_owner");
    let detail = Detail::parse(&text);
    assert_eq!(detail.reruns, 1);
    let evidence: RunReceipt = serde_json::from_str(&detail.evidence.unwrap()).unwrap();
    assert_eq!(evidence.error.as_deref(), Some("machine failure"));
    assert!(remediator.verify_once(id).await.unwrap().is_none());
    assert_eq!(*runtime.reruns.lock().unwrap(), 1);
}

#[tokio::test]
async fn brief_includes_diagnostic_data_with_bounded_unicode_logs() {
    let (remediator, runtime, id) = fixture(json!({}), "script_bug_needs_fix").await;
    let error = "fixture-error-123";
    let excerpt = "fixture-log-456";
    sqlx::query("UPDATE cron_problems SET detail=? WHERE id=?")
        .bind(error)
        .bind(id)
        .execute(&remediator.pool)
        .await
        .unwrap();
    *runtime.logs.lock().unwrap() = Some(format!("{excerpt}\n{}\nEND-OF-LOG", "한".repeat(6000)));
    remediator.on_problem(id, "discord:dm:owner").await.unwrap();
    let lanes = runtime.lanes.lock().unwrap();
    assert!(lanes[0].brief.contains(error));
    assert!(lanes[0].brief.contains(excerpt));
    assert!(!lanes[0].brief.contains("END-OF-LOG"));
    assert_eq!(
        lanes[0].brief.matches('한').count(),
        4000 - excerpt.chars().count() - 1
    );
}

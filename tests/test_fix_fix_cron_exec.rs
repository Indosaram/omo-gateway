use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::cron::remediate::{LaneRequest, Problem, Remediator, RunReceipt, Runtime};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::{
    AgentBackend, AgentCronExecutor, CronJob, CronTaskExecutor, Database, HermesJob, HermesStore,
    HermesStoreSynchronizer, InboundEvent, OmonError, OutboundAction, OutboundDispatcher, Result,
    SessionContext,
};
use serde_json::{json, Value};

fn cron_job(id: &str, payload: &Value) -> CronJob {
    CronJob {
        id: id.into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(payload).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    }
}

async fn pool() -> sqlx::SqlitePool {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db.pool().clone()
}

// Defect 1: a non-ASCII deliver target must not panic on a char boundary.
#[test]
fn non_ascii_deliver_target_does_not_panic() {
    let job: HermesJob = serde_json::from_value(json!({
        "id": "korean",
        "schedule": {"kind": "cron", "expr": "0 9 * * *"},
        "deliver": ["디스코드:123"],
        "failure_deliver": ["디스코드:456"]
    }))
    .unwrap();
    assert!(!job.has_explicit_discord_destination());
    let destinations = job.discord_destinations().unwrap();
    assert!(destinations.iter().all(|d| d.chat_id != "123"));
}

struct FakeJudge;
#[async_trait]
impl Judge for FakeJudge {
    async fn choice(
        &self,
        _: Purpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        Ok(Choice::new("needs_owner_login_or_quota"))
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

struct NoopRuntime;
#[async_trait]
impl Runtime for NoopRuntime {
    async fn open_lane(&self, _: LaneRequest, _: &dyn Judge) -> Result<()> {
        Ok(())
    }
    async fn rerun(&self, _: &str) -> Result<RunReceipt> {
        Err(OmonError::Config("unused".into()))
    }
    async fn quota_check(&self) -> Result<Value> {
        Ok(json!({}))
    }
    async fn log_excerpt(&self, _: &str) -> Result<String> {
        Ok(String::new())
    }
    async fn outcome(&self, _: &Problem, _: &str) -> Result<()> {
        Ok(())
    }
}

// Defect 2: a re-opened problem must re-open the owner's inbox decision.
#[tokio::test]
async fn needs_owner_reopens_closed_inbox_decision() {
    let pool = pool().await;
    let remediator = Remediator {
        pool: pool.clone(),
        judge: Arc::new(FakeJudge),
        runtime: Arc::new(NoopRuntime),
    };
    let problem = Problem {
        id: 7,
        job_id: "job".into(),
        kind: "failed".into(),
        detail: "boom".into(),
        status: "open".into(),
        lane_key: None,
    };
    sqlx::query("INSERT INTO cron_problems(id,job_id,kind,signature,detail,status,created_at,updated_at) VALUES(7,'job','failed','job','boom','open',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)")
        .execute(&pool)
        .await
        .unwrap();
    remediator.needs_owner(&problem, "first").await.unwrap();
    for closed in ["done", "dismissed"] {
        sqlx::query("UPDATE inbox_decisions SET status=? WHERE item_id='cron:7'")
            .bind(closed)
            .execute(&pool)
            .await
            .unwrap();
        remediator.needs_owner(&problem, "again").await.unwrap();
        let (status, action): (String, String) = sqlx::query_as(
            "SELECT status,proposed_action FROM inbox_decisions WHERE item_id='cron:7'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, "open", "re-opened problem after {closed}");
        assert_eq!(action, "again");
    }
}

/// Mirrors OmoBackend's cron path: it runs any `cron_ack_command` found in
/// session metadata as soon as it has produced output, before the scheduler
/// delivers anything.
struct AckingBackend {
    acked_in_backend: Arc<Mutex<usize>>,
}
#[async_trait]
impl AgentBackend for AckingBackend {
    async fn run(&self, session: &mut SessionContext, _: InboundEvent) -> Result<()> {
        session
            .state
            .metadata
            .insert("cron_agent_output".into(), json!("report"));
        if let Some(ack) = session
            .state
            .metadata
            .get("cron_ack_command")
            .and_then(Value::as_str)
        {
            *self.acked_in_backend.lock().unwrap() += 1;
            omon_gateway::cron::ack::run_ack_logged(ack).await;
        }
        Ok(())
    }
}

#[derive(Default)]
struct FlakyDispatcher {
    fail: Mutex<bool>,
}
#[async_trait]
impl OutboundDispatcher for FlakyDispatcher {
    async fn dispatch(&self, _: OutboundAction) -> Result<()> {
        if *self.fail.lock().unwrap() {
            return Err(OmonError::Config("discord down".into()));
        }
        Ok(())
    }
}

// Defect 3: the ack must run only after scheduler delivery succeeded, once.
#[tokio::test]
async fn ack_runs_only_after_successful_scheduler_delivery() {
    let pool = pool().await;
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("scripts")).unwrap();
    let ack_file = temp.path().join("ack.log");
    let acked_in_backend = Arc::new(Mutex::new(0));
    let executor = Arc::new(AgentCronExecutor {
        backend: Arc::new(AckingBackend {
            acked_in_backend: acked_in_backend.clone(),
        }),
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    });
    let dispatcher = Arc::new(FlakyDispatcher::default());
    let scheduler =
        omon_gateway::CronScheduler::with_dispatcher(pool.clone(), executor, dispatcher.clone());
    let payload = json!({
        "id": "ack_job",
        "prompt": "report",
        "deliver": "discord:42",
        "ack_command": format!("printf 'ack\\n' >> '{}'", ack_file.display()),
        "schedule": {"kind": "every", "every": "1h"},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });
    let job = cron_job("ack_job", &payload);

    *dispatcher.fail.lock().unwrap() = true;
    assert!(scheduler.execute_job(&job).await.is_err());
    let acks = std::fs::read_to_string(&ack_file).unwrap_or_default();
    assert_eq!(acks.lines().count(), 0, "failed delivery must not ack");

    *dispatcher.fail.lock().unwrap() = false;
    scheduler.execute_job(&job).await.unwrap();
    let acks = std::fs::read_to_string(&ack_file).unwrap_or_default();
    assert_eq!(
        acks.lines().count(),
        1,
        "successful delivery acks exactly once"
    );
    assert_eq!(*acked_in_backend.lock().unwrap(), 0);

    // Native (non-Hermes) cron payloads take the same scheduler-owned ack path.
    let native = json!({"prompt": "report", "ack_command": "true"});
    let backend: Arc<dyn AgentBackend> = Arc::new(AckingBackend {
        acked_in_backend: acked_in_backend.clone(),
    });
    omon_gateway::cron::execute_native_cron(
        &backend,
        temp.path(),
        &cron_job("native", &native),
        &native,
        30,
    )
    .await
    .unwrap();
    assert_eq!(*acked_in_backend.lock().unwrap(), 0);
}

struct FailOnceBackend {
    calls: Arc<Mutex<usize>>,
}
#[async_trait]
impl AgentBackend for FailOnceBackend {
    async fn run(&self, session: &mut SessionContext, _: InboundEvent) -> Result<()> {
        let first = {
            let mut calls = self.calls.lock().unwrap();
            *calls += 1;
            *calls == 1
        };
        if first {
            return Err(OmonError::Llm("upstream failure".into()));
        }
        session
            .state
            .metadata
            .insert("cron_agent_output".into(), json!("changed"));
        Ok(())
    }
}

// Defect 4: a failed agent run must not consume the monitor change.
#[tokio::test]
async fn monitor_snapshot_is_committed_only_after_successful_run() {
    let pool = pool().await;
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("scripts")).unwrap();
    std::fs::write(temp.path().join("scripts/mon.sh"), "echo state-1\n").unwrap();
    let calls = Arc::new(Mutex::new(0));
    let executor = AgentCronExecutor {
        backend: Arc::new(FailOnceBackend {
            calls: calls.clone(),
        }),
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    };
    let payload = json!({
        "id": "mon_job",
        "prompt": "summarize change",
        "monitor_script": "mon.sh",
        "schedule": {"kind": "every", "every": "1h"},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });
    let job = cron_job("mon_job", &payload);

    assert!(executor.execute(&job).await.is_err());
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_monitor_states")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0, "failed run must not record the snapshot");

    assert_eq!(
        executor.execute(&job).await.unwrap().as_deref(),
        Some("changed")
    );
    assert_eq!(
        *calls.lock().unwrap(),
        2,
        "unreported state reruns the agent"
    );

    assert_eq!(executor.execute(&job).await.unwrap(), None);
    assert_eq!(*calls.lock().unwrap(), 2, "reported state is skipped");
}

struct PromptBackend {
    prompts: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl AgentBackend for PromptBackend {
    async fn run(&self, _: &mut SessionContext, event: InboundEvent) -> Result<()> {
        self.prompts.lock().unwrap().push(event.content);
        Ok(())
    }
}

// P2: context_from must read predecessor output under the key the scheduler writes.
#[tokio::test]
async fn context_from_reads_scheduler_written_output_key() {
    let pool = pool().await;
    let temp = tempfile::tempdir().unwrap();
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let executor = AgentCronExecutor {
        backend: Arc::new(PromptBackend {
            prompts: prompts.clone(),
        }),
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    };
    // The scheduler stores outputs under CronJob::profile(), which is ""
    // for a job without an explicit profile or a discord session key.
    let source = cron_job("src_job", &json!({"prompt": "x"}));
    sqlx::query(
        "INSERT INTO cron_outputs (profile, job_id, run_id, output, created_at) VALUES (?, 'src_job', 'r1', 'PREDECESSOR_OUTPUT', ?)",
    )
    .bind(source.profile())
    .bind(Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();
    let payload = json!({
        "id": "consumer",
        "prompt": "use context",
        "context_from": ["src_job"],
        "origin": {"platform": "discord", "chat_id": "42"},
        "schedule": {"kind": "every", "every": "1h"},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });
    executor
        .execute(&cron_job("consumer", &payload))
        .await
        .unwrap();
    let prompts = prompts.lock().unwrap();
    assert!(prompts[0].contains("PREDECESSOR_OUTPUT"), "{}", prompts[0]);
}

// P2: sync must not delete mirrored jobs on a transient invalid entry or a missing file.
#[tokio::test]
async fn sync_keeps_mirror_jobs_when_entry_invalid_or_store_missing() {
    let pool = pool().await;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("cron")).unwrap();
    let sync =
        HermesStoreSynchronizer::new(pool.clone(), vec![HermesStore::new("default", root.path())]);
    let jobs_path = root.path().join("cron/jobs.json");
    let now = Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap();
    let count = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cron_jobs WHERE id='hermes:default:daily'",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    std::fs::write(
        &jobs_path,
        r#"{"jobs":[{"id":"daily","schedule":{"kind":"cron","expr":"0 9 * * *"}}]}"#,
    )
    .unwrap();
    sync.sync_at(now).await.unwrap();
    assert_eq!(count().await, 1);

    std::fs::write(
        &jobs_path,
        r#"{"jobs":[{"id":"daily","schedule":{"kind":"cron","expr":"garbage"}}]}"#,
    )
    .unwrap();
    sync.sync_at(now).await.unwrap();
    assert_eq!(count().await, 1, "invalid entry must not delete its mirror");

    std::fs::remove_file(&jobs_path).unwrap();
    sync.sync_at(now).await.unwrap();
    assert_eq!(count().await, 1, "missing store must not delete mirrors");
}

// P2: log excerpt reads a bounded tail, not the whole log.
#[tokio::test]
async fn log_tail_is_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("gateway.log");
    let mut log = "old line\n".repeat(1000);
    log.push_str("newest line\n");
    std::fs::write(&path, &log).unwrap();
    let tail = omon_gateway::cron::remediate::read_log_tail(&path, 64)
        .await
        .unwrap();
    assert!(tail.len() <= 64, "tail len {}", tail.len());
    assert!(tail.ends_with("newest line\n"));
    let whole = omon_gateway::cron::remediate::read_log_tail(&path, 1 << 20)
        .await
        .unwrap();
    assert_eq!(whole, log);
}

// P2: lane cwd fallback must not be a developer checkout path.
#[test]
fn lane_cwd_fallback_is_not_a_developer_path() {
    let job = cron_job("plain", &json!({"prompt": "x"}));
    let problem = Problem {
        id: 1,
        job_id: "plain".into(),
        kind: "failed".into(),
        detail: "boom".into(),
        status: "open".into(),
        lane_key: None,
    };
    let lane = omon_gateway::cron::remediate::make_lane(&job, &problem, "k", "lead", &[], None, "")
        .unwrap();
    let expected = std::env::var_os("OMON_WORKSPACE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    assert_eq!(lane.cwd, expected);
}

// Incident 2026-10-16: A timed-out cron script must kill its whole process tree,
// including descendants that moved into their own process group via `set -m`.
#[cfg(unix)]
#[tokio::test]
async fn timed_out_cron_script_kills_whole_process_tree_including_new_process_group() {
    let temp = tempfile::tempdir().unwrap();
    let pid_file = temp.path().join("grandchild.pid");
    let script = format!(
        "(set -m; sh -c 'sleep 300' & echo $! > '{}'; wait)",
        pid_file.display()
    );
    let payload = json!({
        "script": script,
        "timeout_secs": 2,
    });
    let job = cron_job("timeout_tree_job", &payload);
    let acked = Arc::new(Mutex::new(0));
    let backend: Arc<dyn AgentBackend> = Arc::new(AckingBackend {
        acked_in_backend: acked,
    });

    let res =
        omon_gateway::cron::execute_native_cron(&backend, temp.path(), &job, &payload, 30).await;

    let err = res.expect_err("cron script with sleep 300 must time out after 2s");
    assert!(
        err.to_string().contains("timed out"),
        "error should indicate timeout, got: {err}"
    );

    let pid_str = std::fs::read_to_string(&pid_file).expect("grandchild PID file must be written");
    let grandchild_pid: i32 = pid_str
        .trim()
        .parse()
        .expect("grandchild PID must be an integer");
    assert!(grandchild_pid > 1, "grandchild PID must be valid (> 1)");

    // Bounded poll for process reaping up to 5s.
    let start = std::time::Instant::now();
    let mut process_alive = true;
    while start.elapsed() < std::time::Duration::from_secs(5) {
        // SAFETY: libc::kill with signal 0 checks process existence without sending a signal.
        let ret = unsafe { libc::kill(grandchild_pid, 0) };
        if ret != 0 {
            process_alive = false;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    if process_alive {
        // SAFETY: Cleanup in case of test failure so orphan processes do not linger.
        unsafe {
            libc::kill(-grandchild_pid, libc::SIGKILL);
            libc::kill(grandchild_pid, libc::SIGKILL);
        }
    }

    assert!(
        !process_alive,
        "grandchild process {grandchild_pid} survived the cron script timeout"
    );
}

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use omon_gateway::{
    storage::init_pool, AgentBackend, AgentCronExecutor, CronJob, CronTaskExecutor, InboundEvent,
    Result, SessionContext,
};
use serde_json::json;

#[derive(Clone, Default)]
struct MockAgentBackend {
    runs: Arc<Mutex<Vec<(SessionContext, InboundEvent)>>>,
}

#[async_trait]
impl AgentBackend for MockAgentBackend {
    async fn run(&self, session: &mut SessionContext, event: InboundEvent) -> Result<()> {
        self.runs.lock().unwrap().push((session.clone(), event));
        Ok(())
    }
}

#[tokio::test]
async fn skill_only_bundle_runs() {
    let temp = tempfile::tempdir().unwrap();
    let hermes_home = temp.path();

    let bundles_dir = hermes_home.join("skill-bundles");
    std::fs::create_dir_all(&bundles_dir).unwrap();
    std::fs::write(bundles_dir.join("bundle.json"), r#"{"skills": ["member"]}"#).unwrap();

    let member_dir = hermes_home.join("skills").join("member");
    std::fs::create_dir_all(&member_dir).unwrap();
    std::fs::write(
        member_dir.join("SKILL.md"),
        "# Member Skill\nExecute member instructions.",
    )
    .unwrap();

    let pool = init_pool("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let mock_backend = Arc::new(MockAgentBackend::default());
    let executor = AgentCronExecutor {
        backend: mock_backend.clone(),
        workspace_root: hermes_home.to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    };

    let job_json = json!({
        "id": "job_bundle_test",
        "name": "Bundle Test Job",
        "prompt": "",
        "skills": ["/bundle"],
        "schedule": {
            "kind": "every",
            "every": "1h"
        },
        "repeat": {},
        "_omon_hermes_home": hermes_home.to_str().unwrap()
    });

    let cron_job = CronJob {
        id: "job_bundle_test".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&job_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };

    executor
        .execute(&cron_job)
        .await
        .expect("skill-only job with /bundle should execute successfully");

    {
        let runs = mock_backend.runs.lock().unwrap();
        assert_eq!(runs.len(), 1, "Expected exactly one backend turn");
        let (_, ref event) = runs[0];
        let content = &event.content;
        assert!(
            content.contains("[Skill: member]"),
            "Backend turn must contain resolved member skill: {content}"
        );
        assert!(
            content.contains("Execute member instructions."),
            "Backend turn must contain member skill content: {content}"
        );
    }

    let job_missing_skill_json = json!({
        "id": "job_missing_skill_test",
        "name": "Missing Skill Job",
        "prompt": "Continue with normal task",
        "skills": ["non_existent_skill"],
        "schedule": {
            "kind": "every",
            "every": "1h"
        },
        "repeat": {},
        "_omon_hermes_home": hermes_home.to_str().unwrap()
    });

    let cron_job_missing = CronJob {
        id: "job_missing_skill_test".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&job_missing_skill_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };

    executor
        .execute(&cron_job_missing)
        .await
        .expect("job with normal prompt and missing skill should continue");

    {
        let runs = mock_backend.runs.lock().unwrap();
        assert_eq!(runs.len(), 2, "Expected 2 backend turns total");
        let (_, ref event) = runs[1];
        let content = &event.content;
        assert!(
            content.contains("Continue with normal task"),
            "Prompt must be preserved: {content}"
        );
        assert!(
            content.contains("⚠️ Skill(s) not found and skipped: non_existent_skill"),
            "Warning for skipped skill must be present: {content}"
        );
    }
}

#[tokio::test]
async fn overrides_are_honored_or_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let pool = init_pool("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let mock_backend = Arc::new(MockAgentBackend::default());
    let executor = AgentCronExecutor {
        backend: mock_backend.clone(),
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    };

    // 1. Unsupported provider without base_url must be rejected with config error
    let invalid_job_json = json!({
        "id": "unsupported_provider_job",
        "name": "Unsupported Provider Job",
        "prompt": "Test unsupported provider",
        "provider": "unsupported_nonexistent_provider",
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {}
    });
    let cron_job = CronJob {
        id: "unsupported_provider_job".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&invalid_job_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };

    let result = executor.execute(&cron_job).await;
    assert!(
        result.is_err(),
        "unsupported provider without base_url must be rejected"
    );

    // 2. Supported / configured base_url override is honored
    let valid_job_json = json!({
        "id": "valid_provider_job",
        "name": "Valid Provider Job",
        "prompt": "Test valid provider",
        "model": "custom-model",
        "provider": "custom",
        "base_url": "http://127.0.0.1:18801/v1",
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {}
    });
    let cron_job_valid = CronJob {
        id: "valid_provider_job".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&valid_job_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };
    executor
        .execute(&cron_job_valid)
        .await
        .expect("valid override must execute");
    let runs = mock_backend.runs.lock().unwrap();
    let (ref session, _) = runs.last().unwrap();
    assert_eq!(session.state.active_model.as_deref(), Some("custom-model"));
    assert_eq!(
        session
            .state
            .metadata
            .get("cron_job_base_url")
            .and_then(serde_json::Value::as_str),
        Some("http://127.0.0.1:18801/v1")
    );
}

#[derive(Clone, Default)]
struct RecordingDispatcher {
    dispatched: Arc<Mutex<Vec<(omon_gateway::SessionKey, String)>>>,
}

#[async_trait]
impl omon_gateway::OutboundDispatcher for RecordingDispatcher {
    async fn dispatch(&self, action: omon_gateway::OutboundAction) -> Result<()> {
        match action {
            omon_gateway::OutboundAction::SendMessage {
                session, content, ..
            } => {
                self.dispatched.lock().unwrap().push((session, content));
            }
            omon_gateway::OutboundAction::Stream { session, chunk } => {
                self.dispatched
                    .lock()
                    .unwrap()
                    .push((session, chunk.content));
            }
            _ => {}
        }
        Ok(())
    }
}

struct RespondingBackend {
    response: Arc<Mutex<String>>,
}

#[async_trait]
impl AgentBackend for RespondingBackend {
    async fn run(&self, session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
        let resp = self.response.lock().unwrap().clone();
        session
            .state
            .metadata
            .insert("cron_agent_output".into(), json!(resp));
        Ok(())
    }
}

#[tokio::test]
async fn agent_result_uses_scheduler_delivery() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("scripts")).unwrap();
    let resp_holder = Arc::new(Mutex::new("[SILENT]".to_string()));
    let backend = Arc::new(RespondingBackend {
        response: resp_holder.clone(),
    });

    let executor = Arc::new(AgentCronExecutor {
        backend,
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    });

    let dispatcher = Arc::new(RecordingDispatcher::default());
    let scheduler = omon_gateway::CronScheduler::with_dispatcher(
        pool.clone(),
        executor.clone(),
        dispatcher.clone(),
    );

    let job_json = json!({
        "id": "job_multi_dest",
        "name": "Multi Destination Job",
        "prompt": "Produce report",
        "deliver": "discord:42,discord:43",
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });

    let job = scheduler
        .register_with_id(
            "job_multi_dest".to_string(),
            omon_gateway::CronJobSpec::new("every 1h", job_json.clone()),
        )
        .await
        .unwrap();

    // Case 1: [SILENT] response must be suppressed by scheduler -> 0 calls to dispatcher
    sqlx::query("UPDATE cron_jobs SET next_run_at = datetime('now', '-1 minute') WHERE id = ?")
        .bind(&job.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 1);
    scheduler.wait_idle().await;
    assert_eq!(
        dispatcher.dispatched.lock().unwrap().len(),
        0,
        "Silent output must result in 0 dispatcher calls"
    );

    // Case 2: Ordinary report must fan out to BOTH destinations (discord:42 and discord:43)
    *resp_holder.lock().unwrap() = "Report content 123".to_string();
    // A distinct occurrence (second-resolution `datetime('now')` would repeat Case 1's
    // occurrence within the same second and be deduped by `last_occurrence`).
    sqlx::query("UPDATE cron_jobs SET next_run_at = datetime('now', '-2 minutes') WHERE id = ?")
        .bind(&job.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(scheduler.run_due_jobs().await.unwrap(), 1);
    scheduler.wait_idle().await;

    let dispatched = dispatcher.dispatched.lock().unwrap().clone();
    assert_eq!(
        dispatched.len(),
        2,
        "Report must fan out to exactly 2 destinations"
    );
    let channels: Vec<String> = dispatched
        .iter()
        .map(|(s, _)| s.channel_id.clone())
        .collect();
    assert!(
        channels.contains(&"42".to_string()),
        "Must deliver to channel 42"
    );
    assert!(
        channels.contains(&"43".to_string()),
        "Must deliver to channel 43"
    );
}

#[tokio::test]
async fn script_gate_is_silent_in_both_modes() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let temp = tempfile::tempdir().unwrap();
    let scripts_dir = temp.path().join("scripts");
    std::fs::create_dir_all(&scripts_dir).unwrap();

    let script_gate_path = scripts_dir.join("gate.sh");
    std::fs::write(&script_gate_path, "printf '{\"wakeAgent\":false}'\n").unwrap();

    let script_empty_path = scripts_dir.join("empty.sh");
    std::fs::write(&script_empty_path, "printf ''\n").unwrap();

    let mock_backend = Arc::new(MockAgentBackend::default());
    let executor = AgentCronExecutor {
        backend: mock_backend.clone(),
        workspace_root: temp.path().to_path_buf(),
        pool: pool.clone(),
        cron_script_timeout_secs: 30,
    };

    // Case 1: no_agent: true without script must be rejected with Config error
    let invalid_job_json = json!({
        "id": "job_no_agent_no_script",
        "name": "Invalid Job",
        "no_agent": true,
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {}
    });
    let cron_job_invalid = CronJob {
        id: "job_no_agent_no_script".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&invalid_job_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };
    let err_res = executor.execute(&cron_job_invalid).await;
    assert!(
        err_res.is_err(),
        "no_agent: true without script must be rejected"
    );

    // Case 2: no_agent: true with script output {"wakeAgent":false} must return Ok(None)
    let gate_false_json = json!({
        "id": "job_gate_false",
        "name": "Gate False Job",
        "no_agent": true,
        "script": "gate.sh",
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });
    let cron_job_gate_false = CronJob {
        id: "job_gate_false".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&gate_false_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };
    let gate_res = executor
        .execute(&cron_job_gate_false)
        .await
        .expect("execute succeeds");
    assert_eq!(
        gate_res, None,
        "wakeAgent:false must suppress output and return Ok(None)"
    );

    // Case 3: Empty script stdout with prompt must skip agent execution
    let empty_stdout_json = json!({
        "id": "job_empty_stdout",
        "name": "Empty Stdout Job",
        "script": "empty.sh",
        "prompt": "Analyze something",
        "schedule": { "kind": "every", "every": "1h" },
        "repeat": {},
        "_omon_hermes_home": temp.path().to_str().unwrap()
    });
    let cron_job_empty_stdout = CronJob {
        id: "job_empty_stdout".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&empty_stdout_json).unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };
    let empty_res = executor
        .execute(&cron_job_empty_stdout)
        .await
        .expect("execute succeeds");
    assert_eq!(
        empty_res, None,
        "Empty script stdout with prompt must skip agent run and return Ok(None)"
    );
    assert_eq!(
        mock_backend.runs.lock().unwrap().len(),
        0,
        "Mock backend must NOT be invoked when script output is empty"
    );
}

#[tokio::test]
async fn native_cron_suppresses_direct_emission_and_returns_output() {
    use omon_gateway::cron::execute_native_cron;

    struct CronOutputBackend;
    #[async_trait]
    impl AgentBackend for CronOutputBackend {
        async fn run(&self, session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
            assert_eq!(
                session
                    .state
                    .metadata
                    .get("cron_suppress_direct_emission")
                    .and_then(serde_json::Value::as_bool),
                Some(true),
                "native cron must set cron_suppress_direct_emission in metadata"
            );
            session
                .state
                .metadata
                .insert("cron_agent_output".into(), json!("native report text"));
            Ok(())
        }
    }

    let temp = tempfile::tempdir().unwrap();
    let backend: Arc<dyn AgentBackend> = Arc::new(CronOutputBackend);

    let cron_job = CronJob {
        id: "native_cron_output_test".into(),
        session_key: None,
        expression: "every 1h".into(),
        payload_json: serde_json::to_string(&json!({
            "id": "native_cron_output_test",
            "prompt": "Do some native work",
            "schedule": { "kind": "every", "every": "1h" },
            "repeat": {}
        }))
        .unwrap(),
        enabled: true,
        next_run_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        authority: "omon_owned".into(),
    };

    let payload = serde_json::from_str(&cron_job.payload_json).unwrap();
    let output = execute_native_cron(&backend, temp.path(), &cron_job, &payload, 30)
        .await
        .expect("execute_native_cron must succeed");

    assert_eq!(
        output.as_deref(),
        Some("native report text"),
        "execute_native_cron must return agent output to scheduler"
    );
}

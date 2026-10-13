use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::lanes::{
    settle::Settler,
    signals::{DoneSignal, SignalResult, SignalRunner},
};
use omon_gateway::sessions::{
    control::{ControlError, SendMode, SessionControl, Target},
    external::{ExternalDriver, MIGRATION},
    registry::{CommandOutput, CommandRunner},
};
use omon_gateway::storage::{get_lane, Database};
use omon_gateway::work_heartbeat::{Clock, MockClock};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct FakeRunner {
    calls: Mutex<Vec<(String, Vec<String>)>>,
    reads: Mutex<VecDeque<Value>>,
    mutation_code: Mutex<i32>,
    controls: Mutex<Option<Vec<String>>>,
    cwd: Mutex<String>,
}

#[async_trait]
impl CommandRunner for FakeRunner {
    async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
        self.calls
            .lock()
            .unwrap()
            .push((machine.into(), argv.to_vec()));
        let list = argv.iter().any(|word| word == "list");
        let read = argv.iter().any(|word| word == "read");
        let (code, value) = if list {
            let controls = self
                .controls
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| vec!["send".into(), "interrupt".into()]);
            (
                0,
                json!([{"session_id":"session-1","cwd":*self.cwd.lock().unwrap(),"status":"running","controls":controls}]),
            )
        } else if read {
            (
                0,
                self.reads
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("scripted read"),
            )
        } else {
            (*self.mutation_code.lock().unwrap(), json!({"ok":true}))
        };
        Ok(CommandOutput {
            code,
            stdout: serde_json::to_vec(&value).unwrap(),
            stderr: vec![],
        })
    }
}

#[tokio::test]
async fn local_send_has_exact_argv_and_idempotency_key() {
    let runner = Arc::new(FakeRunner::default());
    let control = SessionControl::new(runner.clone(), vec![]);
    let target: Target = "local:session-1".parse().unwrap();
    let key = uuid::Uuid::new_v4();
    control
        .send(&target, "two words 'quoted'", SendMode::FollowUp, key)
        .await
        .unwrap();
    let calls = runner.calls.lock().unwrap();
    assert_eq!(
        calls.last().unwrap(),
        &(
            "local".into(),
            vec![
                "omo".into(),
                "thread".into(),
                "send".into(),
                "session-1".into(),
                "two words 'quoted'".into(),
                "--mode".into(),
                "follow_up".into(),
                "--idempotency-key".into(),
                key.to_string(),
                "--json".into()
            ]
        )
    );
}

#[tokio::test]
async fn remote_send_preserves_text_and_exact_ssh_words() {
    let runner = Arc::new(FakeRunner::default());
    let control = SessionControl::new(runner.clone(), vec!["omarchy".into()]);
    let target: Target = "omarchy:session-1".parse().unwrap();
    let key = uuid::Uuid::new_v4();
    control
        .send(
            &target,
            "two words 'quoted' $(touch nope)",
            SendMode::Steer,
            key,
        )
        .await
        .unwrap();
    let calls = runner.calls.lock().unwrap();
    assert_eq!(
        calls.last().unwrap().1,
        vec![
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "--",
            "omarchy",
            "'omo'",
            "'thread'",
            "'send'",
            "'session-1'",
            "'two words '\\''quoted'\\'' $(touch nope)'",
            "'--mode'",
            "'steer'",
            "'--idempotency-key'",
            &format!("'{key}'"),
            "'--json'"
        ]
    );
}

#[tokio::test]
async fn exit_three_retains_not_running_message() {
    let runner = Arc::new(FakeRunner::default());
    *runner.mutation_code.lock().unwrap() = 3;
    let control = SessionControl::new(runner, vec![]);
    let error = control
        .send(
            &"local:session-1".parse().unwrap(),
            "hi",
            SendMode::Auto,
            uuid::Uuid::new_v4(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ControlError::NotRunning));
    assert_eq!(error.exit_code(), 3);
}

#[tokio::test]
async fn missing_send_control_does_not_send() {
    let runner = Arc::new(FakeRunner::default());
    *runner.controls.lock().unwrap() = Some(vec!["read".into()]);
    let control = SessionControl::new(runner.clone(), vec![]);
    assert!(matches!(
        control
            .send(
                &"local:session-1".parse().unwrap(),
                "hi",
                SendMode::Auto,
                uuid::Uuid::new_v4()
            )
            .await,
        Err(ControlError::Unsupported("send"))
    ));
    assert!(!runner
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(_, args)| args.iter().any(|s| s == "send")));
}

#[tokio::test]
async fn remote_multiline_is_refused_without_any_command() {
    let runner = Arc::new(FakeRunner::default());
    let control = SessionControl::new(runner.clone(), vec![]);
    assert!(matches!(
        control
            .send(
                &"omarchy:session-1".parse().unwrap(),
                "one\ntwo",
                SendMode::Auto,
                uuid::Uuid::new_v4()
            )
            .await,
        Err(ControlError::RemoteMultiline)
    ));
    assert!(runner.calls.lock().unwrap().is_empty());
}

struct PassingChecks;
#[async_trait]
impl SignalRunner for PassingChecks {
    async fn check(&self, signal: &DoneSignal, _: &Path) -> SignalResult {
        SignalResult::pass(signal.to_string(), "fake live proof")
    }
}

#[tokio::test]
async fn adopted_lane_reads_claim_then_settles_after_objection_window() {
    let temp = tempfile::tempdir().unwrap();
    let database = Database::connect(&format!(
        "sqlite://{}?mode=rwc",
        temp.path().join("test.db").display()
    ))
    .await
    .unwrap();
    let migrated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('lanes') WHERE name='external_target')",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    if !migrated {
        sqlx::raw_sql(MIGRATION)
            .execute(database.pool())
            .await
            .unwrap();
    }
    let runner = Arc::new(FakeRunner::default());
    *runner.cwd.lock().unwrap() = temp.path().to_str().unwrap().into();
    runner
        .reads
        .lock()
        .unwrap()
        .push_back(json!({"messages":[],"nextCursor":"baseline"}));
    let clock = Arc::new(MockClock::new(
        Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap(),
    ));
    let driver = ExternalDriver::new(
        database.pool().clone(),
        SessionControl::new(runner.clone(), vec![]),
        clock.clone(),
    );
    let target: Target = "local:session-1".parse().unwrap();
    std::fs::write(temp.path().join("proof.txt"), "proof ready\n").unwrap();
    let key = driver
        .adopt(
            &target,
            "finish proof",
            "file proof.txt",
            omon_gateway::lanes::kibitzer::DEFAULT_LEAD,
        )
        .await
        .unwrap();
    assert_eq!(
        get_lane(database.pool(), &key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
    driver.send_goal(&key).await.unwrap();
    driver.send_goal(&key).await.unwrap();
    assert_eq!(
        runner
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, words)| words.iter().any(|w| w == "send"))
            .count(),
        1
    );
    runner.reads.lock().unwrap().push_back(json!({"messages":[{"id":"report","role":"assistant","text":format!("[REPORT] {key} | done | proof is ready")}],"nextCursor":"report"}));
    driver.read_new(&key).await.unwrap();
    assert_eq!(
        get_lane(database.pool(), &key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
    runner.reads.lock().unwrap().push_back(json!({"messages":[{"id":"claim","role":"assistant","text":format!("[CLAIM_DONE] {key} | proof checked")}],"nextCursor":"claim"}));
    driver.read_new(&key).await.unwrap();
    assert_eq!(
        get_lane(database.pool(), &key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "done_claimed"
    );
    let settler = Settler::new(
        database.pool().clone(),
        clock.clone(),
        Arc::new(PassingChecks),
    );
    assert!(settler.settle_due().await.unwrap().is_empty());
    clock.set(clock.now() + chrono::Duration::minutes(5));
    assert_eq!(settler.settle_due().await.unwrap(), vec![key.clone()]);
    assert_eq!(
        get_lane(database.pool(), &key)
            .await
            .unwrap()
            .unwrap()
            .status,
        "verified_done"
    );
    driver.release(&target).await.unwrap();
    let before = runner.calls.lock().unwrap().len();
    assert_eq!(driver.read_new(&key).await.unwrap(), 0);
    assert_eq!(runner.calls.lock().unwrap().len(), before);
    database.close().await;
}

use async_trait::async_trait;
use omon_gateway::sessions::registry::{
    parse_sessions, CommandOutput, CommandRunner, SessionRegistry,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

#[tokio::test]
async fn list_all_merges_local_and_remote_and_reports_exit_three() {
    struct Runner;
    #[async_trait]
    impl CommandRunner for Runner {
        async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
            assert_eq!(argv, &["omo", "thread", "list", "--all-scope", "--json"]);
            if machine == "empty" {
                return Ok(CommandOutput {
                    code: 3,
                    stdout: vec![],
                    stderr: vec![],
                });
            }
            let output = serde_json::json!([{
                "sessionId": "s1",
                "name": "QA",
                "cwd": "/tmp",
                "status": "live",
                "updated_at": "2026-10-11T09:39:14.800Z",
                "controls": ["send", "read", "interrupt"]
            }]);
            Ok(CommandOutput {
                code: 0,
                stdout: serde_json::to_vec(&output).unwrap(),
                stderr: vec![],
            })
        }
    }

    let registry = SessionRegistry::new(Arc::new(Runner), vec!["omarchy".into(), "empty".into()]);
    let listed = registry.list_all().await;
    assert_eq!(listed.sessions.len(), 2);
    assert_eq!(listed.exit_code, 0);
    assert_eq!(listed.sessions[0].machine, "local");
    assert_eq!(listed.sessions[0].session_id, "s1");
    assert_eq!(
        listed.sessions[0].updated_at.as_deref(),
        Some("2026-10-11T09:39:14.800Z")
    );
    assert_eq!(
        listed.sessions[0].controls,
        vec!["send", "read", "interrupt"]
    );
    assert_eq!(listed.sessions[1].machine, "omarchy");
    assert_eq!(listed.sessions[1].session_id, "s1");
    assert!(listed
        .errors
        .iter()
        .any(|error| error.machine == "empty" && error.message.contains("nothing answering")));
}

#[test]
fn parser_rejects_non_array_and_missing_ids() {
    assert!(parse_sessions(b"{}", "local").is_err());
    assert!(parse_sessions(b"[{}]", "local").is_err());
}

#[test]
fn parser_and_serializer_preserve_exact_planned_fields() {
    let raw = serde_json::json!([
        {
            "sessionId": "01a12a2d-3871-703d-9551-5b79f7bf95bd",
            "name": "Investigate Noveling Sign In Failure",
            "cwd": "/Volumes/T9-Mac/Noveling/noveling",
            "status": "live",
            "private_metadata": {"token": "fixture-only", "message": "not output"},
            "updated_at": "2026-10-11T09:39:14.800Z",
            "controls": ["send", "read", "rename", "set_model", "set_reasoning", "interrupt"]
        }
    ]);
    let sessions = parse_sessions(&serde_json::to_vec(&raw).unwrap(), "local").unwrap();
    assert_eq!(sessions.len(), 1);
    let s = &sessions[0];
    assert_eq!(s.session_id, "01a12a2d-3871-703d-9551-5b79f7bf95bd");
    assert_eq!(
        s.name.as_deref(),
        Some("Investigate Noveling Sign In Failure")
    );
    assert_eq!(s.cwd.as_deref(), Some("/Volumes/T9-Mac/Noveling/noveling"));
    assert_eq!(s.status.as_deref(), Some("live"));
    assert_eq!(s.updated_at.as_deref(), Some("2026-10-11T09:39:14.800Z"));
    assert_eq!(s.last_activity.as_deref(), Some("2026-10-11T09:39:14.800Z"));
    assert_eq!(
        s.controls,
        vec![
            "send",
            "read",
            "rename",
            "set_model",
            "set_reasoning",
            "interrupt"
        ]
    );
    assert_eq!(s.machine, "local");

    // Exact serialized fields check per plan line 457
    let serialized = serde_json::to_value(s).unwrap();
    assert_eq!(
        serialized["session_id"],
        "01a12a2d-3871-703d-9551-5b79f7bf95bd"
    );
    assert_eq!(serialized["updated_at"], "2026-10-11T09:39:14.800Z");
    assert_eq!(
        serialized["controls"],
        serde_json::json!([
            "send",
            "read",
            "rename",
            "set_model",
            "set_reasoning",
            "interrupt"
        ])
    );
    assert_eq!(serialized["machine"], "local");
    assert_eq!(
        serialized
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "controls",
            "cwd",
            "machine",
            "name",
            "session_id",
            "status",
            "updated_at"
        ]
    );
    assert_eq!(s.raw["private_metadata"]["token"], "fixture-only");
}

#[tokio::test]
async fn all_host_failures_keep_taxonomy_and_partial_empty_success_is_usable() {
    struct Runner {
        local: i32,
        remote: i32,
    }
    #[async_trait]
    impl CommandRunner for Runner {
        async fn run(&self, machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            let code = if machine == "local" {
                self.local
            } else {
                self.remote
            };
            Ok(CommandOutput {
                code,
                stdout: b"[]".to_vec(),
                stderr: b"fixture-only private diagnostic".to_vec(),
            })
        }
    }
    for (local, remote, expected) in [
        (3, 3, 3),
        (255, 3, 3),
        (1, 3, 1),
        (2, 3, 2),
        (4, 3, 4),
        (127, 3, 4),
        (5, 3, 5),
        (42, 3, 5),
        (3, 5, 5),
        (0, 3, 0),
        (3, 0, 0),
        (0, 0, 0),
    ] {
        let registry =
            SessionRegistry::new(Arc::new(Runner { local, remote }), vec!["host".into()]);
        let listed = registry.list_all().await;
        assert!(listed.sessions.is_empty());
        assert_eq!(listed.exit_code, expected, "local={local}, remote={remote}");
        assert_eq!(
            listed.errors.len(),
            usize::from(local != 0) + usize::from(remote != 0)
        );
        assert!(!serde_json::to_string(&listed)
            .unwrap()
            .contains("private diagnostic"));
    }
}

#[tokio::test]
async fn malformed_host_and_runner_error_do_not_leak_diagnostics() {
    struct Runner;
    #[async_trait]
    impl CommandRunner for Runner {
        async fn run(&self, machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            if machine == "local" {
                return Err("fixture-only private runner error".into());
            }
            Ok(CommandOutput {
                code: 0,
                stdout: br#"[{"sessionId":"s1","controls":"private parser value"}]"#.to_vec(),
                stderr: vec![],
            })
        }
    }
    let registry = SessionRegistry::new(Arc::new(Runner), vec!["host".into()]);
    let listed = registry.list_all().await;
    assert_eq!(listed.exit_code, 5);
    assert_eq!(listed.errors.len(), 2);
    assert!(listed.errors.iter().all(|error| error.code == 5));
    assert!(!serde_json::to_string(&listed).unwrap().contains("private"));
}

#[tokio::test(start_paused = true)]
async fn cache_reuses_instance_until_exactly_fifteen_seconds() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Runner(AtomicUsize);
    #[async_trait]
    impl CommandRunner for Runner {
        async fn run(&self, _machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CommandOutput {
                code: 0,
                stdout: b"[]".to_vec(),
                stderr: vec![],
            })
        }
    }
    let runner = Arc::new(Runner(AtomicUsize::new(0)));
    let registry = SessionRegistry::new(runner.clone(), vec![]);
    assert_eq!(registry.list_all().await.exit_code, 0);
    tokio::time::advance(Duration::from_secs(14)).await;
    assert_eq!(registry.list_all().await.exit_code, 0);
    assert_eq!(runner.0.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(registry.list_all().await.exit_code, 0);
    assert_eq!(runner.0.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn timeout_drops_child_future_deterministically_without_sleeps() {
    struct DropGuard {
        tx: mpsc::Sender<()>,
    }
    impl Drop for DropGuard {
        fn drop(&mut self) {
            let _ = self.tx.try_send(());
        }
    }

    struct StalledRunner {
        started_tx: Mutex<Option<oneshot::Sender<()>>>,
        dropped_tx: mpsc::Sender<()>,
    }

    #[async_trait]
    impl CommandRunner for StalledRunner {
        async fn run(&self, machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            if machine == "stalled" {
                let _guard = DropGuard {
                    tx: self.dropped_tx.clone(),
                };
                if let Some(tx) = self.started_tx.lock().await.take() {
                    let _ = tx.send(());
                }
                std::future::pending::<()>().await;
                unreachable!()
            } else {
                let fixture = serde_json::json!([
                    {
                        "sessionId": "local-1",
                        "name": "Local Session",
                        "cwd": "/tmp",
                        "status": "idle",
                        "updated_at": "2026-10-11T00:00:00Z",
                        "controls": ["send"]
                    }
                ]);
                Ok(CommandOutput {
                    code: 0,
                    stdout: serde_json::to_vec(&fixture).unwrap(),
                    stderr: vec![],
                })
            }
        }
    }

    let (started_tx, started_rx) = oneshot::channel();
    let (dropped_tx, mut dropped_rx) = mpsc::channel(1);

    let runner = Arc::new(StalledRunner {
        started_tx: Mutex::new(Some(started_tx)),
        dropped_tx,
    });
    let registry =
        SessionRegistry::new(runner, vec!["stalled".into()]).with_timeout(Duration::from_secs(20));
    let registry = Arc::new(registry);

    let task = tokio::spawn(async move { registry.list_all().await });

    // Subscribe before trigger: wait until stalled host has entered run and held drop guard
    tokio::time::timeout(Duration::from_secs(1), started_rx)
        .await
        .unwrap()
        .expect("stalled runner must signal start");

    // Advance paused time past 20s timeout
    tokio::time::advance(Duration::from_secs(21)).await;

    let listed = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(listed.sessions[0].session_id, "local-1");
    assert_eq!(listed.sessions[0].machine, "local");
    assert_eq!(listed.errors.len(), 1);
    assert_eq!(listed.errors[0].machine, "stalled");

    // Future was dropped deterministically
    dropped_rx
        .try_recv()
        .expect("stalled future must drop on timeout");
}

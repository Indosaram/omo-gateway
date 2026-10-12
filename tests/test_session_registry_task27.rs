use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use omon_gateway::sessions::registry::{
    parse_sessions, CommandOutput, CommandRunner, SessionRegistry,
};
use tokio::sync::{mpsc, oneshot, Mutex};

struct Runner {
    started: mpsc::Sender<String>,
    release: Mutex<Option<oneshot::Receiver<()>>>,
}

#[async_trait]
impl CommandRunner for Runner {
    async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
        assert_eq!(argv, &["omo", "thread", "list", "--all-scope", "--json"]);
        self.started.send(machine.to_owned()).await.unwrap();
        match machine {
            "local" => {
                self.release.lock().await.take().unwrap().await.unwrap();
            }
            "stalled" => return std::future::pending().await,
            "empty" => {
                return Ok(CommandOutput {
                    code: 3,
                    stdout: vec![],
                    stderr: vec![],
                });
            }
            "remote" => {}
            _ => panic!("unexpected fixture machine"),
        }
        Ok(CommandOutput {
            code: 0,
            stdout: serde_json::to_vec(&serde_json::json!([{
                "sessionId": machine,
                "name": "fixture",
                "cwd": "/fixture",
                "status": "live",
                "alive": true,
                "surface": "tui",
                "controls": ["read"]
            }]))
            .unwrap(),
            stderr: vec![],
        })
    }
}

#[tokio::test(start_paused = true)]
async fn parallel_listing_retains_successes_when_one_host_times_out() {
    // Given: all subscriptions exist before listing, with a gated local host.
    let (started, mut starts) = mpsc::channel(4);
    let (release, released) = oneshot::channel();
    let registry = SessionRegistry::new(
        Arc::new(Runner {
            started,
            release: Mutex::new(Some(released)),
        }),
        vec!["remote".into(), "stalled".into(), "empty".into()],
    );

    // When: every host starts before the local host is released.
    let listing = tokio::spawn(async move { registry.list_all().await });
    let mut machines = Vec::new();
    for _ in 0..4 {
        machines.push(
            tokio::time::timeout(Duration::from_secs(1), starts.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    release.send(()).unwrap();
    tokio::time::advance(Duration::from_secs(20)).await;
    let result = tokio::time::timeout(Duration::from_secs(1), listing)
        .await
        .unwrap()
        .unwrap();

    // Then: both usable machines survive, and each failure has its own identity.
    machines.sort();
    assert_eq!(machines, ["empty", "local", "remote", "stalled"]);
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.sessions.len(), 2);
    assert_eq!(result.sessions[0].machine, "local");
    assert_eq!(result.sessions[1].machine, "remote");
    assert_eq!(result.errors.len(), 2);
    assert_eq!(result.errors[0].machine, "stalled");
    assert_eq!(result.errors[0].code, 3);
    assert_eq!(result.errors[1].machine, "empty");
    assert!(result.errors[1].message.contains("nothing answering"));
}

#[test]
fn upstream_liveness_fields_are_typed_without_expanding_public_output() {
    // Given: upstream liveness and surface fields plus private metadata.
    let input =
        br#"[{"sessionId":"fixture","alive":false,"surface":"rpc_host","private":"hidden"}]"#;
    // When: the upstream boundary parses a record.
    let sessions = parse_sessions(input, "remote").unwrap();
    // Then: liveness is available internally and only planned fields serialize.
    assert_eq!(sessions[0].alive, Some(false));
    assert_eq!(sessions[0].surface.as_deref(), Some("rpc_host"));
    let output = serde_json::to_value(&sessions[0]).unwrap();
    assert_eq!(output.as_object().unwrap().len(), 7);
    assert!(output.get("alive").is_none());
    assert!(output.get("surface").is_none());
    assert!(output.get("private").is_none());
    assert!(parse_sessions(br#"[{"sessionId":"fixture","alive":"false"}]"#, "local").is_err());
}

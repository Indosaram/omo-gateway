use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::control::{self, ControlError, Target};
use super::registry::{CommandRunner, ProcessCommandRunner};

pub const PROBE_PY: &str = include_str!("probe.py");
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionItem {
    pub id: String,
    pub header: Option<String>,
    pub question: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingQuestion {
    pub request_id: String,
    pub questions: Vec<QuestionItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveSession {
    #[serde(default)]
    pub machine: String,
    pub session_id: String,
    pub routing_id: String,
    pub endpoint_kind: String,
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub streaming: bool,
    pub compacting: bool,
    pub blocking_question: bool,
    pub pending_messages: u64,
    pub message_count: u64,
    pub last_user: String,
    pub last_assistant: String,
    pub questions: Vec<PendingQuestion>,
    #[serde(default)]
    pub messages_error: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotError {
    pub endpoint: String,
    pub error: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineSnapshot {
    #[serde(default)]
    pub machine: String,
    pub sessions: Vec<LiveSession>,
    pub errors: Vec<SnapshotError>,
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("probe timed out")]
    Timeout,
    #[error("probe transport failed: {0}")]
    Transport(String),
    #[error("session not found on this machine")]
    NotFound,
    #[error("session refused: {0}")]
    Refused(String),
    #[error("invalid probe output: {0}")]
    InvalidJson(String),
}

pub struct Probe<R = ProcessCommandRunner> {
    runner: Arc<R>,
}

impl Probe<ProcessCommandRunner> {
    pub fn new_process() -> Self {
        Self {
            runner: Arc::new(ProcessCommandRunner),
        }
    }
}

impl<R: CommandRunner + 'static> Probe<R> {
    pub fn new(runner: Arc<R>) -> Self {
        Self { runner }
    }

    /// Local argv only when the machine *is* `local`. A remote host whose name does not
    /// parse as a `Target` is an error: running the probe locally would report this
    /// machine's sessions under the remote label.
    pub fn argv(machine: &str, args: &[String]) -> Result<Vec<String>, ProbeError> {
        let b64_code = base64::engine::general_purpose::STANDARD.encode(PROBE_PY.as_bytes());
        let mut inner_words = vec![
            "python3".to_owned(),
            "-c".to_owned(),
            "import base64,sys;exec(base64.b64decode(sys.argv.pop(1)))".to_owned(),
            b64_code,
        ];
        inner_words.extend(args.iter().cloned());

        if machine == "local" {
            return Ok(inner_words);
        }
        let target = format!("{machine}:session")
            .parse::<Target>()
            .map_err(|_| ProbeError::Transport(format!("invalid machine name {machine:?}")))?;
        Ok(control::transport_argv(&target, &inner_words))
    }

    pub async fn snapshot(
        &self,
        machine: &str,
        text_limit: usize,
    ) -> Result<MachineSnapshot, ProbeError> {
        let args = vec!["snapshot".to_owned(), text_limit.to_string()];
        let argv = Self::argv(machine, &args)?;

        let output = tokio::time::timeout(PROBE_TIMEOUT, self.runner.run("local", &argv))
            .await
            .map_err(|_| ProbeError::Timeout)?
            .map_err(ProbeError::Transport)?;

        match output.code {
            0 => {
                let stdout_str = std::str::from_utf8(&output.stdout)
                    .map_err(|e| ProbeError::InvalidJson(e.to_string()))?;
                let mut snapshot: MachineSnapshot = serde_json::from_str(stdout_str)
                    .map_err(|e| ProbeError::InvalidJson(format!("{}: {}", e, stdout_str)))?;
                snapshot.machine = machine.to_owned();
                for session in &mut snapshot.sessions {
                    session.machine = machine.to_owned();
                }
                Ok(snapshot)
            }
            3 => Err(ProbeError::NotFound),
            4 => {
                let err_msg = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                Err(ProbeError::Refused(err_msg))
            }
            _ => {
                let err_msg = if !output.stderr.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_owned()
                } else {
                    String::from_utf8_lossy(&output.stdout).trim().to_owned()
                };
                Err(ProbeError::Transport(format!(
                    "exit {}: {}",
                    output.code, err_msg
                )))
            }
        }
    }

    pub async fn snapshot_all(
        &self,
        machines: &[String],
        text_limit: usize,
    ) -> Vec<(String, Result<MachineSnapshot, ProbeError>)> {
        let futures = machines.iter().map(|machine| {
            let machine_owned = machine.clone();
            async move {
                let res = self.snapshot(&machine_owned, text_limit).await;
                (machine_owned, res)
            }
        });
        futures_util::future::join_all(futures).await
    }

    pub async fn interrupt(&self, machine: &str, session: &str) -> Result<(), ProbeError> {
        let args = vec!["interrupt".to_owned(), session.to_owned()];
        let argv = Self::argv(machine, &args)?;

        let output = tokio::time::timeout(PROBE_TIMEOUT, self.runner.run("local", &argv))
            .await
            .map_err(|_| ProbeError::Timeout)?
            .map_err(ProbeError::Transport)?;

        match output.code {
            0 => Ok(()),
            3 => Err(ProbeError::NotFound),
            4 => {
                let err_msg = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                Err(ProbeError::Refused(err_msg))
            }
            _ => {
                let err_msg = if !output.stderr.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_owned()
                } else {
                    String::from_utf8_lossy(&output.stdout).trim().to_owned()
                };
                Err(ProbeError::Transport(format!(
                    "exit {}: {}",
                    output.code, err_msg
                )))
            }
        }
    }

    pub async fn answer(
        &self,
        machine: &str,
        session: &str,
        request_id: &str,
        answers: &serde_json::Value,
        comment: Option<&str>,
    ) -> Result<(), ProbeError> {
        let mut args = vec![
            "answer".to_owned(),
            session.to_owned(),
            request_id.to_owned(),
            answers.to_string(),
        ];
        if let Some(c) = comment {
            args.push(c.to_owned());
        }
        let argv = Self::argv(machine, &args)?;

        let output = tokio::time::timeout(PROBE_TIMEOUT, self.runner.run("local", &argv))
            .await
            .map_err(|_| ProbeError::Timeout)?
            .map_err(ProbeError::Transport)?;

        match output.code {
            0 => Ok(()),
            3 => Err(ProbeError::NotFound),
            4 => {
                let err_msg = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                Err(ProbeError::Refused(err_msg))
            }
            _ => {
                let err_msg = if !output.stderr.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_owned()
                } else {
                    String::from_utf8_lossy(&output.stdout).trim().to_owned()
                };
                Err(ProbeError::Transport(format!(
                    "exit {}: {}",
                    output.code, err_msg
                )))
            }
        }
    }
}

/// One text answers one question. A request that asks several questions at once cannot be
/// answered from a single reply: the text would land as free-form input on every question
/// whose options do not list it, which is not what the owner typed.
pub fn answers_for(
    question: &PendingQuestion,
    text: &str,
) -> Result<serde_json::Value, ControlError> {
    let trimmed = text.trim();
    if question.questions.len() > 1 {
        let ids = question
            .questions
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ControlError::Ambiguous(format!(
            "request {} asks {} questions ({ids}); answer it at the terminal",
            question.request_id,
            question.questions.len()
        )));
    }
    let mut map = serde_json::Map::new();

    for item in &question.questions {
        let is_match = item.options.iter().any(|opt| opt == trimmed);
        let val = if is_match {
            serde_json::json!({
                "selected": [trimmed]
            })
        } else {
            serde_json::json!({
                "selected": [],
                "text": trimmed
            })
        };
        map.insert(item.id.clone(), val);
    }

    Ok(serde_json::Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::registry::CommandOutput;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeRunner {
        outputs: Mutex<std::collections::VecDeque<Result<CommandOutput, String>>>,
        recorded: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Result<CommandOutput, String>>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into()),
                recorded: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl CommandRunner for FakeRunner {
        async fn run(&self, machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
            self.recorded
                .lock()
                .unwrap()
                .push((machine.to_owned(), argv.to_vec()));
            self.outputs
                .lock()
                .unwrap()
                .pop_front()
                .expect("runner has queued output")
        }
    }

    #[test]
    fn test_argv_local_and_remote() {
        let local_argv =
            Probe::<ProcessCommandRunner>::argv("local", &["snapshot".into(), "300".into()])
                .expect("local argv");
        assert_eq!(local_argv[0], "python3");
        assert_eq!(local_argv[1], "-c");
        assert_eq!(
            local_argv[2],
            "import base64,sys;exec(base64.b64decode(sys.argv.pop(1)))"
        );
        assert!(!local_argv[3].is_empty());
        assert_eq!(local_argv[4], "snapshot");
        assert_eq!(local_argv[5], "300");

        let remote_argv =
            Probe::<ProcessCommandRunner>::argv("omarchy", &["snapshot".into(), "300".into()])
                .expect("remote argv");
        assert_eq!(remote_argv[0], "ssh");
        assert_eq!(remote_argv[1], "-o");
        assert_eq!(remote_argv[2], "BatchMode=yes");
        assert_eq!(remote_argv[3], "-o");
        assert_eq!(remote_argv[4], "ConnectTimeout=5");
        assert_eq!(remote_argv[5], "--");
        assert_eq!(remote_argv[6], "omarchy");
        assert!(remote_argv[7].starts_with("'python3'"));
        assert_eq!(remote_argv[8], "'-c'");
    }

    #[tokio::test]
    async fn test_snapshot_parses_json_and_fills_machine() {
        let json_body = r#"{
            "sessions": [{
                "session_id": "sess-123",
                "routing_id": "rpc-1",
                "endpoint_kind": "tui",
                "name": "my-session",
                "cwd": "/home/user",
                "streaming": true,
                "compacting": false,
                "blocking_question": false,
                "pending_messages": 0,
                "message_count": 5,
                "last_user": "hello",
                "last_assistant": "hi there",
                "questions": []
            }],
            "errors": []
        }"#;

        let fake = Arc::new(FakeRunner::new(vec![Ok(CommandOutput {
            code: 0,
            stdout: json_body.as_bytes().to_vec(),
            stderr: vec![],
        })]));

        let probe = Probe::new(fake);
        let snapshot = probe.snapshot("local", 300).await.expect("snapshot parses");
        assert_eq!(snapshot.machine, "local");
        assert_eq!(snapshot.sessions.len(), 1);
        let sess = &snapshot.sessions[0];
        assert_eq!(sess.machine, "local");
        assert_eq!(sess.session_id, "sess-123");
        assert_eq!(sess.routing_id, "rpc-1");
        assert_eq!(sess.endpoint_kind, "tui");
        assert_eq!(sess.name.as_deref(), Some("my-session"));
        assert!(sess.streaming);
        assert!(!sess.compacting);
        assert!(!sess.blocking_question);
        assert_eq!(sess.message_count, 5);
        assert_eq!(sess.last_user, "hello");
        assert_eq!(sess.last_assistant, "hi there");
    }

    #[tokio::test]
    async fn test_exit_codes_and_timeout() {
        let fake_exit_3 = Arc::new(FakeRunner::new(vec![Ok(CommandOutput {
            code: 3,
            stdout: b"{\"ok\":false,\"error\":\"not found\"}".to_vec(),
            stderr: vec![],
        })]));
        let probe = Probe::new(fake_exit_3);
        let err = probe.interrupt("local", "nonexistent").await.unwrap_err();
        match err {
            ProbeError::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }

        struct HangingRunner;
        #[async_trait]
        impl CommandRunner for HangingRunner {
            async fn run(&self, _machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
                tokio::time::sleep(PROBE_TIMEOUT * 2).await;
                Ok(CommandOutput {
                    code: 0,
                    stdout: vec![],
                    stderr: vec![],
                })
            }
        }
        tokio::time::pause();
        let err = Probe::new(Arc::new(HangingRunner))
            .snapshot("omarchy", 10)
            .await
            .unwrap_err();
        assert!(matches!(err, ProbeError::Timeout), "{err:?}");
    }

    #[test]
    fn unparsable_machine_is_an_error_not_a_local_run() {
        // `valid_host` accepts names a `Target` rejects; those must never run the probe
        // on this machine and report local sessions under the remote label.
        let argv = Probe::<ProcessCommandRunner>::argv("box+1", &["snapshot".into()]);
        match argv {
            Err(ProbeError::Transport(message)) => {
                assert!(message.contains("box+1"), "{message}");
            }
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn remote_probe_runs_ssh_argv_locally_once() {
        let runner = Arc::new(FakeRunner::new(vec![Ok(CommandOutput {
            code: 0,
            stdout: br#"{"sessions":[],"errors":[]}"#.to_vec(),
            stderr: vec![],
        })]));
        Probe::new(runner.clone())
            .snapshot("omarchy", 10)
            .await
            .unwrap();
        let recorded = runner.recorded.lock().unwrap();
        assert_eq!(recorded[0].0, "local");
        assert_eq!(recorded[0].1[0], "ssh");
        assert_eq!(recorded[0].1.iter().filter(|w| *w == "ssh").count(), 1);
    }

    #[test]
    fn test_answers_for_mapping() {
        let pq = PendingQuestion {
            request_id: "req-1".into(),
            questions: vec![QuestionItem {
                id: "q1".into(),
                header: Some("Approval".into()),
                question: Some("Proceed?".into()),
                options: vec!["Yes".into(), "No".into()],
            }],
        };

        let ans_match = answers_for(&pq, "Yes").expect("single question answers");
        let obj = ans_match.as_object().expect("object");
        assert_eq!(
            obj.get("q1").unwrap(),
            &serde_json::json!({ "selected": ["Yes"] })
        );

        let ans_custom = answers_for(&pq, "Maybe later").expect("single question answers");
        let obj2 = ans_custom.as_object().expect("object");
        assert_eq!(
            obj2.get("q1").unwrap(),
            &serde_json::json!({ "selected": [], "text": "Maybe later" })
        );
    }

    #[test]
    fn multi_question_request_is_rejected_instead_of_repeating_one_answer() {
        let pq = PendingQuestion {
            request_id: "req-2".into(),
            questions: vec![
                QuestionItem {
                    id: "q1".into(),
                    header: Some("Approval".into()),
                    question: Some("Proceed?".into()),
                    options: vec!["Yes".into(), "No".into()],
                },
                QuestionItem {
                    id: "q2".into(),
                    header: None,
                    question: Some("Notes?".into()),
                    options: vec![],
                },
            ],
        };

        match answers_for(&pq, "Yes") {
            Err(ControlError::Ambiguous(message)) => {
                assert!(message.contains("req-2"), "{message}");
                assert!(message.contains("q1, q2"), "{message}");
            }
            other => panic!("expected an ambiguous error, got {other:?}"),
        }
    }
}

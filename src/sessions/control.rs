use std::{fmt, str::FromStr, sync::Arc, time::Duration};

use clap::ValueEnum;
use serde_json::{json, Value};
use thiserror::Error;

use super::probe::{answers_for, PendingQuestion, Probe, ProbeError};
use super::registry::{
    CommandOutput, CommandRunner, ProcessCommandRunner, RegistryError, Session, SessionRegistry,
};

/// Budget for list/send/read calls.
const CONTROL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    machine: String,
    session: String,
}

impl Target {
    pub fn machine(&self) -> &str {
        &self.machine
    }
    pub fn session(&self) -> &str {
        &self.session
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.machine, self.session)
    }
}

impl FromStr for Target {
    type Err = ControlError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (machine, session) = value.split_once(':').ok_or(ControlError::InvalidTarget)?;
        if machine.is_empty()
            || session.is_empty()
            || machine.starts_with('-')
            || session.starts_with('-')
            || !machine
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-@".contains(c))
            || !session
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        {
            return Err(ControlError::InvalidTarget);
        }
        Ok(Self {
            machine: machine.into(),
            session: session.into(),
        })
    }
}

/// A durable omo session id is a full UUID, which `resolve` would hand back unchanged; any
/// other spelling (a short prefix, a host-local `rpc-N`) needs the listing to canonicalize.
fn is_canonical_session_id(session: &str) -> bool {
    let lengths: Vec<usize> = session.split('-').map(str::len).collect();
    lengths == [8, 4, 4, 4, 12] && session.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum SendMode {
    #[default]
    Auto,
    Steer,
    #[value(name = "follow_up")]
    FollowUp,
}

impl SendMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Steer => "steer",
            Self::FollowUp => "follow_up",
        }
    }
}

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("target must be <machine>:<session> without options or shell syntax")]
    InvalidTarget,
    #[error("not running - kept 24 h")]
    NotRunning,
    #[error("session does not advertise {0} control")]
    Unsupported(&'static str),
    #[error("target matches several sessions: {0}")]
    Ambiguous(String),
    #[error("session control failed: {0}")]
    Probe(String),
    #[error("text must not be empty or contain NUL")]
    InvalidText,
    #[error("session command timed out")]
    Timeout,
    #[error("session command failed ({code}): {message}")]
    Command { code: i32, message: String },
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error("invalid session response: {0}")]
    Json(#[from] serde_json::Error),
}

impl ControlError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotRunning => 3,
            Self::InvalidTarget | Self::Unsupported(_) | Self::Ambiguous(_) | Self::InvalidText => {
                2
            }
            Self::Registry(error) => error.exit_code(),
            Self::Command { code, .. } if (1..=5).contains(code) => *code,
            Self::Probe(_) | Self::Command { .. } | Self::Timeout | Self::Json(_) => 5,
        }
    }
}

/// OpenSSH joins remote argv using spaces; quote each remote word, never a whole shell command.
pub fn transport_argv(target: &Target, words: &[String]) -> Vec<String> {
    if target.machine == "local" {
        return words.to_vec();
    }
    let mut argv = [
        "ssh",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=5",
        "--",
        target.machine(),
    ]
    .map(str::to_owned)
    .to_vec();
    argv.extend(
        words
            .iter()
            .map(|word| format!("'{}'", word.replace('\'', "'\\''"))),
    );
    argv
}

pub struct SessionControl<R = ProcessCommandRunner> {
    runner: Arc<R>,
    registry: SessionRegistry<R>,
}

impl SessionControl<ProcessCommandRunner> {
    pub fn from_environment() -> Result<Self, ControlError> {
        Ok(Self::new(
            Arc::new(ProcessCommandRunner),
            super::registry::configured_hosts()?,
        ))
    }
}

impl<R: CommandRunner + 'static> SessionControl<R> {
    pub fn new(runner: Arc<R>, hosts: Vec<String>) -> Self {
        Self {
            registry: SessionRegistry::new(runner.clone(), hosts),
            runner,
        }
    }

    pub fn registry(&self) -> &SessionRegistry<R> {
        &self.registry
    }

    pub fn runner(&self) -> Arc<R> {
        self.runner.clone()
    }

    pub async fn resolve(&self, target: &Target) -> Result<(Target, Session), ControlError> {
        let list = SessionRegistry::new(self.runner.clone(), vec![target.machine.clone()])
            .list_all()
            .await;
        let machine_error = list
            .errors
            .into_iter()
            .find(|error| error.machine == target.machine);

        // Daemon-hosted sessions list a host-local id ("rpc-2") that `omo thread` rejects;
        // the durable id is the only address every subcommand accepts, so it is canonical.
        let sessions: Vec<Session> = list
            .sessions
            .into_iter()
            .filter(|s| s.machine == target.machine)
            .map(|mut s| {
                if let Some(durable) = s.raw.get("durableSessionId").and_then(Value::as_str) {
                    if !durable.is_empty() && durable != s.session_id {
                        s.session_id = durable.to_owned();
                    }
                }
                s
            })
            .collect();

        // 1. Exact id (canonical durable id or the host-local id the listing reported)
        let exact_matches: Vec<Session> = sessions
            .iter()
            .filter(|s| {
                s.session_id == target.session
                    || s.raw.get("sessionId").and_then(Value::as_str)
                        == Some(target.session.as_str())
            })
            .cloned()
            .collect();
        if exact_matches.len() == 1 {
            let session = exact_matches.into_iter().next().unwrap();
            let resolved_target = Target {
                machine: target.machine.clone(),
                session: session.session_id.clone(),
            };
            return Ok((resolved_target, session));
        }

        // 2. Exact durableSessionId
        let durable_matches: Vec<Session> = sessions
            .iter()
            .filter(|s| {
                s.raw
                    .get("durableSessionId")
                    .and_then(Value::as_str)
                    .map(|d| d == target.session)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if durable_matches.len() == 1 {
            let session = durable_matches.into_iter().next().unwrap();
            let resolved_target = Target {
                machine: target.machine.clone(),
                session: session.session_id.clone(),
            };
            return Ok((resolved_target, session));
        }

        // 3. Unique prefix of either session_id or durableSessionId
        let prefix_matches: Vec<Session> = sessions
            .iter()
            .filter(|s| {
                let id_match = s.session_id.starts_with(&target.session);
                let durable_match = s
                    .raw
                    .get("durableSessionId")
                    .and_then(Value::as_str)
                    .map(|d| d.starts_with(&target.session))
                    .unwrap_or(false);
                id_match || durable_match
            })
            .cloned()
            .collect();
        if prefix_matches.len() == 1 {
            let session = prefix_matches.into_iter().next().unwrap();
            let resolved_target = Target {
                machine: target.machine.clone(),
                session: session.session_id.clone(),
            };
            return Ok((resolved_target, session));
        }
        if prefix_matches.len() > 1 {
            let ids = prefix_matches
                .iter()
                .map(|s| s.session_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ControlError::Ambiguous(ids));
        }

        // 4. Exact name
        let name_matches: Vec<Session> = sessions
            .iter()
            .filter(|s| {
                s.raw
                    .get("name")
                    .and_then(Value::as_str)
                    .map(|n| n == target.session)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        if name_matches.len() == 1 {
            let session = name_matches.into_iter().next().unwrap();
            let resolved_target = Target {
                machine: target.machine.clone(),
                session: session.session_id.clone(),
            };
            return Ok((resolved_target, session));
        }
        if name_matches.len() > 1 {
            let ids = name_matches
                .iter()
                .map(|s| s.session_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ControlError::Ambiguous(ids));
        }

        match machine_error {
            Some(error) => Err(ControlError::Command {
                code: error.code,
                message: format!(
                    "session listing failed on {}: {}",
                    error.machine, error.message
                ),
            }),
            None => Err(ControlError::NotRunning),
        }
    }

    pub async fn session(
        &self,
        target: &Target,
        capability: &'static str,
    ) -> Result<Session, ControlError> {
        // Do not use a cached listing to authorize a mutation.
        let list = SessionRegistry::new(self.runner.clone(), vec![target.machine.clone()])
            .list_all()
            .await;
        let machine_error = list
            .errors
            .into_iter()
            .find(|error| error.machine == target.machine);
        let session = list
            .sessions
            .into_iter()
            .find(|s| {
                s.machine == target.machine
                    && (s.session_id == target.session
                        || s.raw.get("durableSessionId").and_then(Value::as_str)
                            == Some(target.session.as_str()))
            })
            .ok_or_else(|| match machine_error {
                // A failed listing says nothing about the session; only a successful
                // listing that lacks it proves it is not running.
                Some(error) => ControlError::Command {
                    code: error.code,
                    message: format!(
                        "session listing failed on {}: {}",
                        error.machine, error.message
                    ),
                },
                None => ControlError::NotRunning,
            })?;
        if !session.controls.iter().any(|c| c == capability) {
            return Err(ControlError::Unsupported(capability));
        }
        Ok(session)
    }

    pub async fn command(&self, target: &Target, words: &[String]) -> Result<Value, ControlError> {
        let output = self.output(target, words).await?;
        match output.code {
            0 => Ok(serde_json::from_slice(&output.stdout)?),
            3 => Err(ControlError::NotRunning),
            code => Err(ControlError::Command {
                code,
                message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            }),
        }
    }

    pub async fn output(
        &self,
        target: &Target,
        words: &[String],
    ) -> Result<CommandOutput, ControlError> {
        self.output_with_timeout(target, words, CONTROL_TIMEOUT)
            .await
    }

    /// Like `output`, with a caller-chosen budget for long-running remote commands.
    pub async fn output_with_timeout(
        &self,
        target: &Target,
        words: &[String],
        timeout: Duration,
    ) -> Result<CommandOutput, ControlError> {
        let argv = transport_argv(target, words);
        let output = tokio::time::timeout(timeout, self.runner.run("local", &argv))
            .await
            .map_err(|_| ControlError::Timeout)?
            .map_err(|message| ControlError::Command { code: 5, message })?;
        Ok(output)
    }

    pub async fn read(
        &self,
        target: &Target,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<Value, ControlError> {
        // A canonical id needs no listing: `resolve` would spend a full `omo thread list`
        // (ssh for a remote host) handing the id back unchanged, and a read is not a
        // mutation, so nothing else depends on that listing.
        let resolved_target = if is_canonical_session_id(target.session()) {
            target.clone()
        } else {
            self.resolve(target).await?.0
        };
        let mut words = vec![
            "omo".into(),
            "thread".into(),
            "read".into(),
            resolved_target.session.clone(),
            "--limit".into(),
            limit.to_string(),
        ];
        if let Some(cursor) = cursor {
            words.extend(["--cursor".into(), cursor.into()]);
        }
        words.extend(["--all-scope".into(), "--json".into()]);
        self.command(&resolved_target, &words).await
    }

    pub async fn send(
        &self,
        target: &Target,
        text: &str,
        mode: SendMode,
        key: uuid::Uuid,
    ) -> Result<Value, ControlError> {
        if text.trim().is_empty() || text.contains('\0') {
            return Err(ControlError::InvalidText);
        }
        // A canonical id needs no listing here either: callers that already resolved (the
        // Discord/Slack commands) and the external lane's stored targets all pass one, and
        // `resolve` would spend a second `omo thread list` (ssh for a remote host) only to
        // hand the id back. The capability pre-check needs that listing, so it stays on the
        // path that has to resolve anyway; `omo thread send` itself rejects a session that
        // cannot take a send.
        let resolved_target = if is_canonical_session_id(target.session()) {
            target.clone()
        } else {
            let (resolved, session) = self.resolve(target).await?;
            if !session.controls.iter().any(|c| c == "send") {
                return Err(ControlError::Unsupported("send"));
            }
            resolved
        };
        self.command(
            &resolved_target,
            &[
                "omo".into(),
                "thread".into(),
                "send".into(),
                resolved_target.session.clone(),
                text.into(),
                "--mode".into(),
                mode.as_str().into(),
                "--idempotency-key".into(),
                key.to_string(),
                "--all-scope".into(),
                "--json".into(),
            ],
        )
        .await
    }

    pub async fn interrupt(&self, target: &Target) -> Result<Value, ControlError> {
        let (resolved_target, session) = self.resolve(target).await?;
        if !session.controls.iter().any(|c| c == "interrupt") {
            return Err(ControlError::Unsupported("interrupt"));
        }
        let probe = Probe::new(self.runner.clone());
        probe
            .interrupt(resolved_target.machine(), &session.session_id)
            .await
            .map_err(|e| match e {
                ProbeError::NotFound => ControlError::NotRunning,
                ProbeError::Timeout => ControlError::Timeout,
                ProbeError::Refused(msg) => ControlError::Command {
                    code: 4,
                    message: msg,
                },
                other => ControlError::Probe(other.to_string()),
            })?;
        Ok(json!({"ok": true, "target": resolved_target.to_string()}))
    }

    pub async fn answer(
        &self,
        target: &Target,
        text: &str,
        request_id: Option<&str>,
    ) -> Result<Value, ControlError> {
        if text.trim().is_empty() || text.contains('\0') {
            return Err(ControlError::InvalidText);
        }
        let (resolved_target, session) = self.resolve(target).await?;
        let probe = Probe::new(self.runner.clone());
        let snapshot = probe
            .snapshot(resolved_target.machine(), 100)
            .await
            .map_err(|e| match e {
                ProbeError::NotFound => ControlError::NotRunning,
                ProbeError::Timeout => ControlError::Timeout,
                ProbeError::Refused(msg) => ControlError::Command {
                    code: 4,
                    message: msg,
                },
                other => ControlError::Probe(other.to_string()),
            })?;

        let live_session = snapshot
            .sessions
            .into_iter()
            .find(|s| s.session_id == session.session_id || s.routing_id == session.session_id)
            .ok_or(ControlError::NotRunning)?;

        let question: PendingQuestion = if let Some(req_id) = request_id {
            live_session
                .questions
                .into_iter()
                .find(|q| q.request_id == req_id)
                .ok_or_else(|| {
                    ControlError::Probe(format!("no pending question matching request_id {req_id}"))
                })?
        } else {
            match live_session.questions.len() {
                0 => return Err(ControlError::Probe("no pending question".into())),
                1 => live_session.questions.into_iter().next().unwrap(),
                n => {
                    return Err(ControlError::Ambiguous(format!(
                        "session has {n} pending questions, specify request_id"
                    )))
                }
            }
        };

        let answers = answers_for(&question, text)?;
        probe
            .answer(
                resolved_target.machine(),
                &session.session_id,
                &question.request_id,
                &answers,
                None,
            )
            .await
            .map_err(|e| match e {
                ProbeError::NotFound => ControlError::NotRunning,
                ProbeError::Timeout => ControlError::Timeout,
                ProbeError::Refused(msg) => ControlError::Command {
                    code: 4,
                    message: msg,
                },
                other => ControlError::Probe(other.to_string()),
            })?;

        Ok(json!({
            "ok": true,
            "target": resolved_target.to_string(),
            "request_id": question.request_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    /// Fails every listing, as an ssh timeout or VPN blip would.
    struct ListingDown;

    #[async_trait]
    impl CommandRunner for ListingDown {
        async fn run(&self, _machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            Err("ssh: connect timed out".into())
        }
    }

    /// Answers every command with success after a fixed amount of (paused) time.
    struct SlowRunner(Duration);

    #[async_trait]
    impl CommandRunner for SlowRunner {
        async fn run(&self, _machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
            tokio::time::sleep(self.0).await;
            Ok(CommandOutput {
                code: 0,
                stdout: b"{}".to_vec(),
                stderr: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn failed_listing_is_not_reported_as_not_running() {
        let control = SessionControl::new(Arc::new(ListingDown), vec!["omarchy".into()]);
        let target: Target = "omarchy:session-1".parse().unwrap();
        let error = control.session(&target, "send").await.unwrap_err();
        assert!(
            !matches!(error, ControlError::NotRunning),
            "a failed listing must stay a transient error, got {error:?}"
        );
        assert!(matches!(error, ControlError::Command { .. }), "{error:?}");
    }

    #[tokio::test]
    async fn successful_listing_without_the_session_is_not_running() {
        struct EmptyListing;
        #[async_trait]
        impl CommandRunner for EmptyListing {
            async fn run(&self, _: &str, _: &[String]) -> Result<CommandOutput, String> {
                Ok(CommandOutput {
                    code: 0,
                    stdout: b"[]".to_vec(),
                    stderr: Vec::new(),
                })
            }
        }
        let control = SessionControl::new(Arc::new(EmptyListing), vec!["omarchy".into()]);
        let target: Target = "omarchy:session-1".parse().unwrap();
        assert!(matches!(
            control.session(&target, "send").await,
            Err(ControlError::NotRunning)
        ));
    }

    /// Answers a listing with one session and every other command with an empty page,
    /// recording the argv of every call so a test can see which commands ran.
    #[derive(Default)]
    struct Recording {
        calls: std::sync::Mutex<Vec<Vec<String>>>,
    }

    #[async_trait]
    impl CommandRunner for Recording {
        async fn run(&self, _machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
            self.calls.lock().unwrap().push(argv.to_vec());
            let listing = argv.iter().any(|word| word.contains("list"));
            Ok(CommandOutput {
                code: 0,
                stdout: if listing {
                    br#"[{"sessionId":"01a1429e-8585-7e8a-b89b-2ea8458e4312","name":"qa"}]"#
                        .to_vec()
                } else {
                    br#"{"items":[]}"#.to_vec()
                },
                stderr: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn canonical_read_skips_the_session_listing() {
        let runner = Arc::new(Recording::default());
        let control = SessionControl::new(runner.clone(), vec![]);
        let target: Target = "local:01a1429e-8585-7e8a-b89b-2ea8458e4312"
            .parse()
            .unwrap();
        control
            .read(&target, 5, None)
            .await
            .expect("canonical read");

        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "one read, no listing: {calls:?}");
        assert!(
            !calls[0].iter().any(|word| word.contains("list")),
            "a canonical id must not be resolved through a listing: {:?}",
            calls[0]
        );
        assert!(
            calls[0].contains(&"01a1429e-8585-7e8a-b89b-2ea8458e4312".to_owned()),
            "read must address the canonical id: {:?}",
            calls[0]
        );
    }

    #[tokio::test]
    async fn prefix_read_still_resolves_through_the_listing() {
        let runner = Arc::new(Recording::default());
        let control = SessionControl::new(runner.clone(), vec![]);
        let target: Target = "local:01a1429e".parse().unwrap();
        control.read(&target, 5, None).await.expect("prefix read");

        let calls = runner.calls.lock().unwrap();
        assert!(
            calls
                .iter()
                .any(|argv| argv.iter().any(|word| word.contains("list"))),
            "a short prefix still needs the listing: {calls:?}"
        );
        let last = calls.last().expect("read ran");
        assert!(
            last.contains(&"01a1429e-8585-7e8a-b89b-2ea8458e4312".to_owned()),
            "read must address the resolved id: {last:?}"
        );
    }

    #[tokio::test]
    async fn canonical_send_skips_the_session_listing() {
        let runner = Arc::new(Recording::default());
        let control = SessionControl::new(runner.clone(), vec![]);
        let target: Target = "local:01a1429e-8585-7e8a-b89b-2ea8458e4312"
            .parse()
            .unwrap();
        control
            .send(&target, "hi", SendMode::Auto, uuid::Uuid::new_v4())
            .await
            .expect("canonical send");

        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "one send, no listing: {calls:?}");
        assert!(
            calls[0].iter().any(|word| word == "send"),
            "the send itself must run: {:?}",
            calls[0]
        );
        assert!(
            !calls[0].iter().any(|word| word.contains("list")),
            "a canonical id must not be resolved through a listing: {:?}",
            calls[0]
        );
    }

    #[tokio::test]
    async fn prefix_send_still_checks_the_send_capability() {
        // The pre-check needs the listing, so it survives on the path that must resolve.
        struct NoSendControl;
        #[async_trait]
        impl CommandRunner for NoSendControl {
            async fn run(&self, _machine: &str, argv: &[String]) -> Result<CommandOutput, String> {
                assert!(
                    argv.iter().any(|word| word == "list"),
                    "only the listing may run: {argv:?}"
                );
                Ok(CommandOutput {
                    code: 0,
                    stdout: br#"[{"sessionId":"01a1429e-8585-7e8a-b89b-2ea8458e4312","controls":["read"]}]"#
                        .to_vec(),
                    stderr: Vec::new(),
                })
            }
        }
        let control = SessionControl::new(Arc::new(NoSendControl), vec![]);
        let target: Target = "local:01a1429e".parse().unwrap();
        assert!(matches!(
            control
                .send(&target, "hi", SendMode::Auto, uuid::Uuid::new_v4())
                .await,
            Err(ControlError::Unsupported("send"))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn output_budget_is_per_call() {
        let control =
            SessionControl::new(Arc::new(SlowRunner(Duration::from_secs(30))), Vec::new());
        let target: Target = "local:session-1".parse().unwrap();
        let words = vec!["true".to_owned()];
        assert!(matches!(
            control.output(&target, &words).await,
            Err(ControlError::Timeout)
        ));
        let output = control
            .output_with_timeout(&target, &words, Duration::from_secs(120))
            .await
            .expect("a longer budget lets a slow command finish");
        assert_eq!(output.code, 0);
    }
}

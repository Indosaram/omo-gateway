use std::{fmt, str::FromStr, sync::Arc, time::Duration};

use clap::ValueEnum;
use serde_json::Value;
use thiserror::Error;

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
    #[error("remote multiline text is unsupported; stdin/text-file support has not been verified")]
    RemoteMultiline,
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
            Self::InvalidTarget
            | Self::Unsupported(_)
            | Self::RemoteMultiline
            | Self::InvalidText => 2,
            Self::Registry(error) => error.exit_code(),
            Self::Command { code, .. } if (1..=5).contains(code) => *code,
            Self::Command { .. } | Self::Timeout | Self::Json(_) => 5,
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
            .find(|s| s.machine == target.machine && s.session_id == target.session)
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
        let mut words = vec![
            "omo".into(),
            "thread".into(),
            "read".into(),
            target.session.clone(),
            "--limit".into(),
            limit.to_string(),
            "--json".into(),
        ];
        if let Some(cursor) = cursor {
            words.extend(["--cursor".into(), cursor.into()]);
        }
        self.command(target, &words).await
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
        if target.machine != "local" && text.contains(['\n', '\r']) {
            return Err(ControlError::RemoteMultiline);
        }
        self.session(target, "send").await?;
        self.command(
            target,
            &[
                "omo".into(),
                "thread".into(),
                "send".into(),
                target.session.clone(),
                text.into(),
                "--mode".into(),
                mode.as_str().into(),
                "--idempotency-key".into(),
                key.to_string(),
                "--json".into(),
            ],
        )
        .await
    }

    pub async fn interrupt(&self, target: &Target) -> Result<Value, ControlError> {
        self.session(target, "interrupt").await?;
        self.command(
            target,
            &[
                "omo".into(),
                "thread".into(),
                "interrupt".into(),
                target.session.clone(),
                "--json".into(),
            ],
        )
        .await
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

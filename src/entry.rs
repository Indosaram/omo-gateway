use clap::{Parser, Subcommand};
use omon_gateway::migrate::MigrateArgs;
use omon_gateway::profiles::{apply_block, export_json, export_profile};
use omon_gateway::{OmonError, Result};
use tokio_util::sync::CancellationToken;

#[allow(dead_code, private_interfaces)]
mod legacy {
    include!("main.rs");

    pub async fn run_gateway_public() -> Result<()> {
        run_gateway().await
    }

    #[allow(unused_imports)]
    pub mod dashboard {
        use tracing_subscriber::util::SubscriberInitExt;

        trait OptionStringTrim {
            fn trim(&self) -> &str;
        }

        impl OptionStringTrim for Option<String> {
            fn trim(&self) -> &str {
                self.as_deref().unwrap_or_default().trim()
            }
        }

        include!("dashboard.rs");
    }

    pub mod dashboard_runtime {
        include!("dashboard_runtime.rs");
    }
}

#[derive(Debug, Parser)]
#[command(name = "omo-gateway")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the Discord gateway and, when enabled by environment, the dashboard.
    Run,
    /// Run only the Web Dashboard HTTP/WebSocket server.
    Dashboard(legacy::dashboard::DashboardArgs),
    /// Alias for `dashboard`.
    Serve(legacy::dashboard::DashboardArgs),
    /// Run database migration utilities.
    Migrate(MigrateArgs),
    /// Control persistent worker lanes.
    Lane(omon_gateway::lanes::cli::LaneArgs),
    Inbox(omon_gateway::inbox::cli::InboxArgs),
    InboxPropose(omon_gateway::inbox::proposals::ProposalArgs),
    Profile(ProfileArgs),
    Sessions(omon_gateway::sessions::cli::SessionsArgs),
    Usage(omon_gateway::usage::UsageArgs),
    Cron(omon_gateway::cron::report::CronArgs),
}

#[derive(Debug, clap::Args)]
struct ProfileArgs {
    #[command(subcommand)]
    command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    Export {
        bot_id: String,
    },
    ApplyBlock {
        #[arg(long)]
        bot_id: String,
        #[arg(long)]
        block_file: std::path::PathBuf,
    },
}

impl Cli {
    fn into_command(self) -> Command {
        self.command.unwrap_or(Command::Run)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let command = Cli::parse().into_command();
    if let Command::Cron(args) = command {
        std::process::exit(omon_gateway::cron::report::run(args).await);
    }
    if let Command::InboxPropose(args) = command {
        std::process::exit(omon_gateway::inbox::proposals::run_proposal(args).await);
    }
    if let Command::Usage(args) = command {
        std::process::exit(omon_gateway::usage::run(args).await);
    }
    if let Command::Lane(args) = command {
        std::process::exit(omon_gateway::lanes::cli::run(args).await);
    }
    if let Command::Inbox(args) = command {
        std::process::exit(omon_gateway::inbox::cli::run(args).await);
    }
    if let Command::Profile(args) = command {
        std::process::exit(run_profile(args).await);
    }
    if let Command::Sessions(args) = command {
        std::process::exit(omon_gateway::sessions::cli::run(args).await);
    }
    legacy::dashboard::init_tracing();

    match command {
        Command::Run => run_gateway_with_optional_dashboard().await,
        Command::Dashboard(args) | Command::Serve(args) => {
            legacy::dashboard_runtime::run_standalone_cli(
                legacy::dashboard::DashboardSettings::from_args(args),
            )
            .await
        }
        Command::Migrate(args) => omon_gateway::migrate::run_migrate(args).await,
        Command::Lane(args) => std::process::exit(omon_gateway::lanes::cli::run(args).await),
        Command::Inbox(args) => std::process::exit(omon_gateway::inbox::cli::run(args).await),
        Command::InboxPropose(args) => {
            std::process::exit(omon_gateway::inbox::proposals::run_proposal(args).await)
        }
        Command::Profile(args) => std::process::exit(run_profile(args).await),
        Command::Sessions(args) => std::process::exit(omon_gateway::sessions::cli::run(args).await),
        Command::Usage(args) => std::process::exit(omon_gateway::usage::run(args).await),
        Command::Cron(args) => std::process::exit(omon_gateway::cron::report::run(args).await),
    }
}

async fn connect_profile_database() -> std::result::Result<omon_gateway::Database, String> {
    let url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".to_owned());
    omon_gateway::Database::connect(&url)
        .await
        .map_err(|error| error.to_string())
}

async fn run_profile(args: ProfileArgs) -> i32 {
    let result = async {
        let database = connect_profile_database().await?;
        let result = match args.command {
            ProfileCommand::Export { bot_id } => {
                let profile = export_profile(database.pool(), &bot_id)
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("unknown bot id: {bot_id}"))?;
                export_json(&profile).map_err(|error| error.to_string())
            }
            ProfileCommand::ApplyBlock { bot_id, block_file } => {
                let block = tokio::fs::read_to_string(block_file)
                    .await
                    .map_err(|error| error.to_string())?;
                let applied = apply_block(database.pool(), &bot_id, &block)
                    .await
                    .map_err(|error| error.to_string())?;
                serde_json::to_string_pretty(&serde_json::json!({
                    "bot_id": bot_id,
                    "before": {"characters": applied.before.length, "sha256": applied.before.sha256},
                    "after": {"characters": applied.after.length, "sha256": applied.after.sha256}
                }))
                .map_err(|error| error.to_string())
            }
        };
        database.close().await;
        result
    }
    .await;
    match result {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(error) => {
            let code = if error.starts_with("unknown bot id:") {
                2
            } else {
                1
            };
            eprintln!("{}", serde_json::json!({"ok": false, "error": error}));
            code
        }
    }
}

async fn run_gateway_with_optional_dashboard() -> Result<()> {
    let settings = legacy::dashboard::DashboardSettings::from_env();
    if !settings.enabled {
        return legacy::run_gateway_public().await;
    }
    settings.validate()?;

    let shutdown = CancellationToken::new();
    let dashboard_shutdown = shutdown.clone();
    let dashboard = legacy::dashboard_runtime::run_standalone(settings, dashboard_shutdown, false);
    let gateway = legacy::run_gateway_public();
    tokio::pin!(dashboard);
    tokio::pin!(gateway);

    tokio::select! {
        result = &mut gateway => {
            shutdown.cancel();
            dashboard.await?;
            result
        }
        result = &mut dashboard => {
            match result {
                Ok(()) => Err(OmonError::Config("dashboard server stopped unexpectedly while the Discord gateway was running".into())),
                Err(error) => Err(error),
            }
        }
    }
}

#[derive(Debug)]
pub struct RuntimeOwnershipLock {
    _file: std::fs::File,
    pub path: std::path::PathBuf,
}

impl RuntimeOwnershipLock {
    pub fn try_acquire(name: &str) -> Result<Self, OmonError> {
        let lock_dir = std::env::var("OMON_RUNTIME_LOCK_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let lock_path = lock_dir.join(format!("omon-gateway-{name}.lock"));

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| {
                OmonError::Config(format!(
                    "failed to open runtime lock {}: {e}",
                    lock_path.display()
                ))
            })?;

        use fs2::FileExt;
        file.try_lock_exclusive().map_err(|e| {
            OmonError::Config(format!(
                "another gateway instance owns runtime lock {} (error: {e})",
                lock_path.display()
            ))
        })?;

        Ok(Self {
            _file: file,
            path: lock_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command, RuntimeOwnershipLock};

    #[tokio::test]
    async fn session_cli_output_preserves_aggregate_exit_and_projection() {
        use omon_gateway::sessions::registry::{CommandOutput, CommandRunner, SessionRegistry};
        struct Runner(i32);
        #[async_trait::async_trait]
        impl CommandRunner for Runner {
            async fn run(&self, _machine: &str, _argv: &[String]) -> Result<CommandOutput, String> {
                Ok(CommandOutput {
                    code: self.0,
                    stdout: br#"[{"sessionId":"fixture","name":"QA","cwd":"/tmp","private":"not output"}]"#.to_vec(),
                    stderr: vec![],
                })
            }
        }
        for expected in [0, 1, 2, 3, 4, 5] {
            let registry = SessionRegistry::new(std::sync::Arc::new(Runner(expected)), vec![]);
            let listed = registry.list_all().await;
            let code = listed.exit_code;
            let output = serde_json::to_string(&listed).unwrap();
            assert_eq!(code, expected);
            assert!(!output.contains("not output"));
            let value: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert!(value.get("sessions").is_some());
            assert!(value.get("errors").is_some());
        }
        assert!(Cli::try_parse_from(["omo-gateway", "sessions", "list", "--json"]).is_ok());
    }

    #[test]
    fn duplicate_gateway_start_is_rejected() {
        let lock_name = format!("test-lock-{}", uuid::Uuid::new_v4());
        let lock1 = RuntimeOwnershipLock::try_acquire(&lock_name).unwrap();

        // Second acquisition must fail explicitly
        let err = RuntimeOwnershipLock::try_acquire(&lock_name).unwrap_err();
        assert!(
            err.to_string()
                .contains("another gateway instance owns runtime lock"),
            "Second start must be rejected: {err}"
        );

        // After releasing, a third can acquire
        drop(lock1);
        let lock3 = RuntimeOwnershipLock::try_acquire(&lock_name).unwrap();
        assert!(lock3.path.exists());
    }

    #[test]
    fn cli_defaults_to_gateway_run() {
        assert!(matches!(
            Cli::try_parse_from(["omo-gateway"]).unwrap().into_command(),
            Command::Run
        ));
    }

    #[test]
    fn cli_accepts_dashboard_and_serve_alias() {
        let dashboard = Cli::try_parse_from([
            "omo-gateway",
            "dashboard",
            "--host",
            "127.0.0.1",
            "--port",
            "9120",
        ])
        .unwrap();
        match dashboard.into_command() {
            Command::Dashboard(args) => {
                assert_eq!(args.host, "127.0.0.1");
                assert_eq!(args.port, 9120);
            }
            other => panic!("expected dashboard command, got {other:?}"),
        }

        assert!(matches!(
            Cli::try_parse_from(["omo-gateway", "serve", "--insecure"])
                .unwrap()
                .into_command(),
            Command::Serve(_)
        ));
    }

    #[tokio::test]
    async fn attached_dashboard_controls_live_runtime() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use omon_gateway::{
            AgentRunner, CronJob, CronScheduler, CronTaskExecutor, InboundEvent, MultiplexerConfig,
            OmonError, OutboundAction, OutboundDispatcher, SessionContext, SessionKey,
            SessionMultiplexer, SmartApprovalGuard,
        };
        use std::sync::Arc;
        use tower::ServiceExt;

        let pool = omon_gateway::storage::init_pool("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let workspace_root =
            std::env::temp_dir().join(format!("omon-test-attached-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace_root).unwrap();

        #[derive(Default)]
        struct RecDispatcher(Arc<tokio::sync::Mutex<Vec<(String, String)>>>);
        #[async_trait::async_trait]
        impl OutboundDispatcher for RecDispatcher {
            async fn dispatch(&self, action: OutboundAction) -> Result<(), OmonError> {
                if let OutboundAction::SendMessage {
                    session, content, ..
                } = action
                {
                    self.0.lock().await.push((session.channel_id, content));
                }
                Ok(())
            }
        }
        let rec_messages = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let dispatcher = Arc::new(RecDispatcher(rec_messages.clone()));

        struct InterruptRunner;
        #[async_trait::async_trait]
        impl AgentRunner for InterruptRunner {
            async fn run(
                &self,
                _session: &mut SessionContext,
                _event: InboundEvent,
            ) -> Result<(), OmonError> {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                Ok(())
            }
        }

        let mux = SessionMultiplexer::with_dispatcher(
            pool.clone(),
            Arc::new(InterruptRunner),
            Some(dispatcher.clone()),
            MultiplexerConfig::default(),
        );

        let approvals = SmartApprovalGuard::new();

        struct RecCron;
        #[async_trait::async_trait]
        impl CronTaskExecutor for RecCron {
            async fn execute(&self, job: &CronJob) -> Result<Option<String>, OmonError> {
                Ok(Some(format!("cron-ran:{}", job.id)))
            }
        }
        let scheduler =
            CronScheduler::with_dispatcher(pool.clone(), Arc::new(RecCron), dispatcher.clone());

        let state = crate::legacy::dashboard::DashboardState::new(
            pool.clone(),
            Some(mux.clone()),
            scheduler.clone(),
            omon_gateway::ToolRegistry::new(),
            approvals.clone(),
            crate::legacy::dashboard::WebDashboardDispatcher::new(),
            serde_json::json!({}),
            workspace_root.clone(),
            vec![],
            1,
            workspace_root.clone(),
        );

        let app = crate::legacy::dashboard::router(state);

        // 1. Live Turn Interruption via Dashboard
        let session = SessionKey::new("discord", Some("bot1"), "123", None::<String>, "42");
        sqlx::query("INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json) VALUES (?, 'discord', '123', '42', '{}')")
            .bind(session.storage_key())
            .execute(&pool)
            .await
            .unwrap();

        let event = InboundEvent::message(session.clone(), "msg1", "held turn");
        mux.route(event).await.unwrap();

        let stop_req = Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{}/stop", session.storage_key()))
            .header("Host", "127.0.0.1:19744")
            .body(Body::empty())
            .unwrap();
        let stop_resp = app.clone().oneshot(stop_req).await.unwrap();
        assert_eq!(stop_resp.status(), StatusCode::OK);
        let stop_bytes = axum::body::to_bytes(stop_resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let stop_json: serde_json::Value = serde_json::from_slice(&stop_bytes).unwrap();
        assert_eq!(
            stop_json["stopped"], true,
            "Active live turn must be stopped"
        );

        // 2. Live Approval Resolution via Dashboard
        let prompt = approvals.request().await;
        let approval_id = prompt.request_id;

        let resolve_req = Request::builder()
            .method("POST")
            .uri(format!("/api/approvals/{}/resolve", approval_id))
            .header("Host", "127.0.0.1:19744")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"decision": "once"}"#))
            .unwrap();
        let resolve_resp = app.clone().oneshot(resolve_req).await.unwrap();
        assert_eq!(resolve_resp.status(), StatusCode::OK);

        let decision = prompt.wait(std::time::Duration::from_millis(100)).await;
        assert_eq!(decision, Ok(omon_gateway::ApprovalDecision::Once));

        // 3. Manual Cron Trigger reaches primary egress
        let cron_json = serde_json::json!({
            "id": "dash_cron_1",
            "name": "Dash Cron",
            "prompt": "run task",
            "deliver": "discord:999",
            "schedule": {"kind": "cron", "expr": "0 * * * *"}
        });
        let cron_job = CronJob {
            id: "dash_cron_1".into(),
            session_key: None,
            expression: "0 * * * *".into(),
            payload_json: serde_json::to_string(&cron_json).unwrap(),
            enabled: true,
            next_run_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            authority: "omon_owned".into(),
        };
        sqlx::query("INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority, created_at, updated_at) VALUES (?, ?, ?, 1, 'omon_owned', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(&cron_job.id)
            .bind(&cron_job.expression)
            .bind(&cron_job.payload_json)
            .execute(&pool)
            .await
            .unwrap();

        let trigger_req = Request::builder()
            .method("POST")
            .uri("/api/cron/jobs/dash_cron_1/trigger")
            .header("Host", "127.0.0.1:19744")
            .body(Body::empty())
            .unwrap();
        let trigger_resp = app.oneshot(trigger_req).await.unwrap();
        assert_eq!(trigger_resp.status(), StatusCode::OK);

        // Execute scheduled work through scheduler to verify delivery reaches primary egress
        scheduler.execute_job(&cron_job).await.unwrap();
        let msgs = rec_messages.lock().await.clone();
        assert!(
            msgs.iter()
                .any(|(ch, c)| ch == "999" && c.contains("cron-ran:dash_cron_1")),
            "Must reach primary egress"
        );

        let _ = std::fs::remove_dir_all(workspace_root);
    }
}

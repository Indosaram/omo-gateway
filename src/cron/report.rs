use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

use super::remediate::{Detail, RunReceipt};
use crate::{CronScheduler, OmonError, Result};

pub const PERSONA_COMMAND: &str = "omo-gateway cron health";

/// Upper bound for one native-socket request line.
const MAX_NATIVE_REQUEST_BYTES: u64 = 4096;
/// How long shutdown waits for in-flight native requests before aborting them.
const NATIVE_SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Debug, Args)]
pub struct CronArgs {
    #[command(subcommand)]
    pub command: CronCommand,
}

#[derive(Debug, Subcommand)]
pub enum CronCommand {
    Health {
        #[arg(long)]
        json: bool,
    },
    RunNow {
        job_id: String,
        #[arg(long, required = true)]
        wait: bool,
        #[arg(long, required = true)]
        json: bool,
    },
    ApproveRerun {
        problem_id: i64,
        #[arg(long, required = true)]
        owner_approved: bool,
        /// One-time token delivered to the owner with the escalation message.
        #[arg(long, required = true)]
        owner_token: String,
    },
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProblemReport {
    pub id: i64,
    pub job_id: String,
    pub kind: String,
    pub status: String,
    pub lane_key: Option<String>,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct JobReport {
    pub job_id: String,
    pub last_status: Option<String>,
    pub last_success_kst: Option<String>,
    pub problems: Vec<ProblemReport>,
}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub ok_jobs: usize,
    pub jobs: Vec<JobReport>,
}

pub async fn health(pool: &SqlitePool) -> Result<HealthReport> {
    let jobs: Vec<(String, String)> =
        sqlx::query_as("SELECT id,payload_json FROM cron_jobs WHERE enabled=1 ORDER BY id")
            .fetch_all(pool)
            .await?;
    let mut report = HealthReport {
        ok_jobs: 0,
        jobs: Vec::new(),
    };
    for (job_id, payload) in jobs {
        let payload: Value =
            serde_json::from_str(&payload).map_err(|e| OmonError::Config(e.to_string()))?;
        let last_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM cron_runs WHERE job_id=? ORDER BY started_at DESC LIMIT 1",
        )
        .bind(&job_id)
        .fetch_optional(pool)
        .await?;
        let last_success: Option<DateTime<Utc>> = sqlx::query_scalar("SELECT completed_at FROM cron_runs WHERE job_id=? AND status='succeeded' ORDER BY completed_at DESC LIMIT 1").bind(&job_id).fetch_optional(pool).await?.flatten();
        let problems = sqlx::query_as::<_, ProblemReport>("SELECT id,job_id,kind,status,lane_key,detail FROM cron_problems WHERE job_id=? AND status IN ('open','remediating','needs_owner') ORDER BY id")
            .bind(&job_id).fetch_all(pool).await?;
        let problems = problems
            .into_iter()
            .map(|mut problem| {
                problem.detail = without_approval_hash(&problem.detail);
                problem
            })
            .collect::<Vec<_>>();
        if problems.is_empty()
            && last_status.as_deref() == Some("succeeded")
            && payload
                .get("last_delivery_error")
                .is_none_or(Value::is_null)
        {
            report.ok_jobs = report.ok_jobs.saturating_add(1);
        }
        report.jobs.push(JobReport {
            job_id,
            last_status: last_status.or_else(|| {
                payload
                    .get("last_status")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }),
            last_success_kst: last_success.map(|date| {
                date.with_timezone(&chrono_tz::Asia::Seoul)
                    .format("%Y-%m-%d %H:%M:%S KST")
                    .to_string()
            }),
            problems,
        });
    }
    Ok(report)
}

/// Health output is agent-reachable; never echo the stored approval-token digest.
fn without_approval_hash(detail: &str) -> String {
    match serde_json::from_str::<Value>(detail) {
        Ok(mut value) => {
            let removed = value
                .as_object_mut()
                .is_some_and(|object| object.remove("approval_hash").is_some());
            if removed {
                value.to_string()
            } else {
                detail.to_owned()
            }
        }
        Err(_) => detail.to_owned(),
    }
}

pub fn render(report: &HealthReport) -> String {
    let mut lines = vec![format!("OK jobs: {}", report.ok_jobs)];
    for job in &report.jobs {
        if job.problems.is_empty() {
            if job.last_status.as_deref() != Some("succeeded") {
                lines.push(format!(
                    "- {}: {} (last success: {})",
                    job.job_id,
                    job.last_status.as_deref().unwrap_or("never run"),
                    job.last_success_kst.as_deref().unwrap_or("none")
                ));
            }
        } else {
            for problem in &job.problems {
                lines.push(format!(
                    "- {}: {} / {} (lane: {}; last success: {}): {}",
                    job.job_id,
                    problem.kind,
                    problem.status,
                    problem.lane_key.as_deref().unwrap_or("none"),
                    job.last_success_kst.as_deref().unwrap_or("none"),
                    Detail::parse(&problem.detail).cause
                ));
            }
        }
    }
    lines.join("\n")
}

pub fn outcome_message(job: &str, cause: &str, fix: &str, receipt: &RunReceipt) -> Result<String> {
    if receipt.status != "succeeded" || receipt.last_delivery_error.is_some() {
        return Err(OmonError::Config(
            "cannot report an unsuccessful or undelivered rerun as fixed".into(),
        ));
    }
    Ok(format!(
        "{job}: {cause} -> {fix} (re-run {} succeeded, evidence: {})",
        receipt.run_id,
        receipt
            .output
            .as_deref()
            .unwrap_or("native run committed; last_delivery_error=null")
    ))
}

pub fn owner_message(job: &str, action: &str) -> String {
    format!("{job}: needs you - {action}")
}

pub async fn queue_outcome(
    pool: &SqlitePool,
    _scheduler: &CronScheduler,
    problem: i64,
    job: &str,
    message: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO cron_remediation_outcomes(problem_id,job_id,message,created_at) VALUES(?,?,?,?) ON CONFLICT(problem_id,message) DO NOTHING")
        .bind(problem).bind(job).bind(crate::lanes::scrub::scrub(message)).bind(Utc::now()).execute(pool).await?;
    Ok(())
}

pub async fn digest_section(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Option<String>> {
    let since = now - chrono::Duration::hours(24);
    let problems: Vec<ProblemReport> = sqlx::query_as("SELECT id,job_id,kind,status,lane_key,detail FROM cron_problems WHERE julianday(updated_at)>=julianday(?) AND julianday(updated_at)<=julianday(?) AND status!='ignored' ORDER BY job_id,id")
        .bind(since).bind(now).fetch_all(pool).await?;
    let outcomes: Vec<String> = sqlx::query_scalar("SELECT message FROM cron_remediation_outcomes WHERE delivered_at IS NULL AND julianday(created_at)<=julianday(?) ORDER BY id")
        .bind(now).fetch_all(pool).await?;
    if problems.is_empty() && outcomes.is_empty() {
        return Ok(None);
    }
    let mut lines = vec!["## Cron".to_owned()];
    for problem in problems {
        lines.push(format!(
            "- {}: {} / {} (lane: {}): {}",
            problem.job_id,
            problem.kind,
            problem.status,
            problem.lane_key.as_deref().unwrap_or("none"),
            crate::lanes::scrub::scrub(&Detail::parse(&problem.detail).cause)
        ));
    }
    lines.extend(outcomes.into_iter().map(|message| format!("- {message}")));
    Ok(Some(lines.join("\n")))
}

pub async fn acknowledge_digest(pool: &SqlitePool, run_id: &str) -> Result<()> {
    sqlx::query("UPDATE cron_remediation_outcomes SET delivered_at=? WHERE delivered_at IS NULL AND julianday(created_at)<=(SELECT julianday(started_at) FROM cron_runs WHERE run_id=? AND job_id=? AND status='succeeded')")
        .bind(Utc::now()).bind(run_id).bind(crate::inbox::digest::JOB_ID).execute(pool).await?;
    // A delivered digest no longer needs the one-time approval token in the clear.
    sqlx::query("UPDATE OR IGNORE cron_remediation_outcomes SET message = substr(message, 1, instr(message, '--owner-token ') + 13) || '<redacted>' WHERE delivered_at IS NOT NULL AND instr(message, '--owner-token ') > 0 AND message NOT LIKE '%--owner-token <redacted>'")
        .execute(pool)
        .await?;
    Ok(())
}

fn socket_path() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var("HOME").map_err(|e| OmonError::Config(e.to_string()))?)
            .join(".omon/cron-run-now.sock"),
    )
}

pub async fn serve_native(
    pool: SqlitePool,
    scheduler: CronScheduler,
    shutdown: CancellationToken,
) -> Result<()> {
    bind_native().await?.serve(pool, scheduler, shutdown).await
}

pub struct NativeServer {
    path: PathBuf,
    listener: tokio::net::UnixListener,
}

pub async fn bind_native() -> Result<NativeServer> {
    let path = socket_path()?;
    NativeServer::bind(path).await
}

impl NativeServer {
    pub async fn bind(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| OmonError::Config(e.to_string()))?;
        }
        if path.exists() {
            if tokio::net::UnixStream::connect(&path).await.is_ok() {
                return Err(OmonError::Config(
                    "native cron socket is already owned by a gateway".into(),
                ));
            }
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| OmonError::Config(e.to_string()))?;
        }
        let listener = {
            // Create the socket 0600 from the start instead of chmod-ing after bind.
            // SAFETY: umask only swaps the process file-creation mask; the bind below is synchronous.
            let previous = unsafe { libc::umask(0o177) };
            let bound = tokio::net::UnixListener::bind(&path);
            unsafe { libc::umask(previous) };
            bound.map_err(|e| OmonError::Config(e.to_string()))?
        };
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|e| OmonError::Config(e.to_string()))?;
        Ok(Self { path, listener })
    }

    pub async fn serve(
        self,
        pool: SqlitePool,
        scheduler: CronScheduler,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let Self { path, listener } = self;
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => break,
                accepted = listener.accept() => {
                    let stream = match accepted {
                        Ok((stream, _)) => stream,
                        Err(error) => {
                            // A transient accept error (e.g. fd exhaustion) must not stop the gateway.
                            tracing::warn!(%error, "native cron accept failed");
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                    };
                    let pool = pool.clone();
                    let scheduler = scheduler.clone();
                    connections.spawn(async move {
                        let mut stream = BufReader::new(stream);
                        let mut line = String::new();
                        let result: Result<RunReceipt> = async {
                            tokio::time::timeout(std::time::Duration::from_secs(10), (&mut stream).take(MAX_NATIVE_REQUEST_BYTES).read_line(&mut line)).await
                                .map_err(|_| OmonError::Config("native cron request timed out".into()))?
                                .map_err(|e| OmonError::Config(e.to_string()))?;
                            let request: Value = serde_json::from_str(line.trim()).map_err(|e| OmonError::Config(e.to_string()))?;
                            if let Some(id) = request.get("approve_problem").and_then(Value::as_i64) {
                                let token = request.get("owner_token").and_then(Value::as_str).unwrap_or_default();
                                return super::remediate::approve_rerun(id, token).await;
                            }
                            let job = request.as_str().ok_or_else(|| OmonError::Config("expected native job id".into()))?;
                            let payload: String = sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id=?").bind(job).fetch_one(&pool).await?;
                            if super::remediate::publishing(job, &serde_json::from_str(&payload).map_err(|e| OmonError::Config(e.to_string()))?) {
                                return Err(OmonError::Config("publishing rerun requires owner approval through remediation".into()));
                            }
                            scheduler.trigger_wait(job).await
                        }.await;
                        let response = match result {
                            Ok(receipt) => serde_json::to_string(&receipt),
                            Err(error) => serde_json::to_string(&serde_json::json!({"status":"failed","error":error.to_string()})),
                        };
                        match response {
                            Ok(response) => {
                                if let Err(error) = stream.get_mut().write_all(format!("{response}\n").as_bytes()).await {
                                    tracing::warn!(%error, "native cron receipt delivery failed");
                                }
                            }
                            Err(error) => tracing::error!(%error, "native cron receipt serialization failed"),
                        }
                    });
                }
                Some(result) = connections.join_next(), if !connections.is_empty() => {
                    if let Err(error) = result { tracing::error!(%error, "native cron request task failed"); }
                }
            }
        }
        // In-flight `trigger_wait` runs can last as long as the job timeout; give them a
        // short grace period, then abort so shutdown (and socket cleanup) cannot hang.
        let drain = async {
            while let Some(result) = connections.join_next().await {
                if let Err(error) = result {
                    tracing::error!(%error, "native cron request task failed on shutdown");
                }
            }
        };
        if tokio::time::timeout(NATIVE_SHUTDOWN_GRACE, drain)
            .await
            .is_err()
        {
            connections.abort_all();
            while connections.join_next().await.is_some() {}
        }
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| OmonError::Config(e.to_string()))?;
        Ok(())
    }
}

pub async fn run(args: CronArgs) -> i32 {
    let result: Result<(i32, String)> = async {
        match args.command {
            CronCommand::Health { json } => {
                let url = std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "sqlite://omon_gateway.db".into());
                let database = crate::Database::connect(&url).await?;
                let report = health(database.pool()).await?;
                let output = if json {
                    serde_json::to_string(&report).map_err(|e| OmonError::Config(e.to_string()))?
                } else {
                    render(&report)
                };
                database.close().await;
                Ok((0, output))
            }
            CronCommand::RunNow {
                job_id,
                wait: _,
                json: _,
            } => {
                let mut stream = tokio::net::UnixStream::connect(socket_path()?)
                    .await
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                stream
                    .write_all(
                        format!(
                            "{}\n",
                            serde_json::to_string(&job_id)
                                .map_err(|e| OmonError::Config(e.to_string()))?
                        )
                        .as_bytes(),
                    )
                    .await
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                let mut response = String::new();
                tokio::time::timeout(
                    std::time::Duration::from_secs(1800),
                    BufReader::new(stream).read_line(&mut response),
                )
                .await
                .map_err(|_| OmonError::Config("native cron run exceeded 1800 seconds".into()))?
                .map_err(|e| OmonError::Config(e.to_string()))?;
                let receipt: Value = serde_json::from_str(response.trim())
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                Ok((
                    if receipt.get("status").and_then(Value::as_str) == Some("succeeded") {
                        0
                    } else {
                        1
                    },
                    response.trim().into(),
                ))
            }
            CronCommand::ApproveRerun {
                problem_id,
                owner_approved: _,
                owner_token,
            } => {
                let mut stream = tokio::net::UnixStream::connect(socket_path()?)
                    .await
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                let request =
                    serde_json::json!({"approve_problem":problem_id,"owner_token":owner_token})
                        .to_string();
                stream
                    .write_all(format!("{request}\n").as_bytes())
                    .await
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                let mut response = String::new();
                tokio::time::timeout(
                    std::time::Duration::from_secs(1800),
                    BufReader::new(stream).read_line(&mut response),
                )
                .await
                .map_err(|_| OmonError::Config("approved cron run exceeded 1800 seconds".into()))?
                .map_err(|e| OmonError::Config(e.to_string()))?;
                let receipt: Value = serde_json::from_str(response.trim())
                    .map_err(|e| OmonError::Config(e.to_string()))?;
                Ok((
                    if receipt.get("status").and_then(Value::as_str) == Some("succeeded") {
                        0
                    } else {
                        1
                    },
                    response.trim().into(),
                ))
            }
        }
    }
    .await;
    match result {
        Ok((code, output)) => {
            println!("{output}");
            code
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"ok":false,"error":error.to_string()})
            );
            2
        }
    }
}

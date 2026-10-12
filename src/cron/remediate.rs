use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::judge::{Judge, Purpose};
use crate::lanes::signals::{DoneSignal, SignalResult};
use crate::{CronJob, CronScheduler, OmonError, Result};

pub const MAX_RERUNS: u64 = 3;
pub const PUBLISHING_JOBS: [&str; 3] = [
    "omon-playstore-daily-publish",
    "omon-vpn-mall-daily-publish-v2",
    "omon-content-intel-publish-ready",
];

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Problem {
    pub id: i64,
    pub job_id: String,
    pub kind: String,
    pub detail: String,
    pub status: String,
    pub lane_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Detail {
    pub cause: String,
    #[serde(default)]
    pub reruns: u64,
    #[serde(default)]
    pub owner_approved: bool,
    #[serde(default)]
    pub last_run_id: Option<String>,
    #[serde(default)]
    pub evidence: Option<String>,
}

impl Detail {
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_else(|_| Self {
            cause: text.into(),
            ..Self::default()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReceipt {
    pub job_id: String,
    pub run_id: String,
    pub status: String,
    pub output: Option<String>,
    pub last_delivery_error: Option<String>,
    pub error: Option<String>,
}

impl RunReceipt {
    pub fn verified(&self, payload: &Value) -> Result<bool> {
        if self.status != "succeeded" || self.last_delivery_error.is_some() {
            return Ok(false);
        }
        if let Some(pattern) = payload
            .pointer("/expect/output_regex")
            .and_then(Value::as_str)
        {
            let regex = regex::Regex::new(pattern).map_err(|e| OmonError::Config(e.to_string()))?;
            return Ok(self
                .output
                .as_deref()
                .is_some_and(|output| regex.is_match(output)));
        }
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct LaneRequest {
    pub key: String,
    pub cwd: PathBuf,
    pub brief: String,
    pub done: String,
    pub lead: String,
}

#[async_trait]
pub trait Runtime: Send + Sync {
    async fn open_lane(&self, request: LaneRequest, judge: &dyn Judge) -> Result<()>;
    async fn rerun(&self, job: &str) -> Result<RunReceipt>;
    async fn quota_check(&self) -> Result<Value>;
    async fn log_excerpt(&self, job: &str) -> Result<String>;
    async fn outcome(&self, problem: &Problem, message: &str) -> Result<()>;
}

pub struct Remediator {
    pub pool: SqlitePool,
    pub judge: Arc<dyn Judge>,
    pub runtime: Arc<dyn Runtime>,
}

static REMEDIATOR: OnceLock<Arc<Remediator>> = OnceLock::new();

pub fn install(remediator: Arc<Remediator>) -> Result<()> {
    REMEDIATOR
        .set(remediator)
        .map_err(|_| OmonError::Config("cron remediator already installed".into()))
}

pub async fn handle_event(body: &str, lead: &str) -> Result<bool> {
    let Some(remediator) = REMEDIATOR.get() else {
        return Ok(false);
    };
    let event: Value = serde_json::from_str(body).map_err(|e| OmonError::Config(e.to_string()))?;
    let id = event
        .get("problem_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| OmonError::Config("cron_problem lacks problem_id".into()))?;
    remediator.on_problem(id, lead).await?;
    Ok(true)
}

pub async fn approve_rerun(id: i64) -> Result<RunReceipt> {
    let remediator = REMEDIATOR
        .get()
        .ok_or_else(|| OmonError::Config("cron remediator not installed".into()))?;
    remediator
        .approve_rerun(id)
        .await?
        .ok_or_else(|| OmonError::Config("cron rerun budget exhausted or already fixed".into()))
}

pub async fn settle_lane(key: &str) -> Result<Option<Vec<SignalResult>>> {
    let Some(remediator) = REMEDIATOR.get() else {
        return Ok(None);
    };
    let Some(problem) = sqlx::query_as::<_, Problem>(
        "SELECT id,job_id,kind,detail,status,lane_key FROM cron_problems WHERE lane_key=?",
    )
    .bind(key)
    .fetch_optional(&remediator.pool)
    .await?
    else {
        return Ok(None);
    };
    if problem.status == "fixed" {
        return Ok(Some(vec![SignalResult::pass(
            "cron natural success",
            "committed clean success",
        )]));
    }
    let job = remediator.job(&problem.job_id).await?;
    if publishing(&job.id, &job.payload()?) && !Detail::parse(&problem.detail).owner_approved {
        return Ok(None);
    }
    let receipt = remediator.verify_once(problem.id).await?;
    Ok(Some(vec![match receipt {
        Some(receipt) if receipt.verified(&job.payload()?)? => SignalResult::pass(
            "cron rerun",
            serde_json::to_string(&receipt).map_err(|e| OmonError::Config(e.to_string()))?,
        ),
        Some(receipt) => SignalResult::fail(
            "cron rerun",
            serde_json::to_string(&receipt).map_err(|e| OmonError::Config(e.to_string()))?,
        ),
        None => SignalResult::fail("cron rerun", "owner action required"),
    }]))
}

pub fn publishing(id: &str, payload: &Value) -> bool {
    PUBLISHING_JOBS.contains(&id)
        || payload.get("external_effect").and_then(Value::as_bool) == Some(true)
}

pub fn lane_key(job: &str, _id: i64) -> String {
    let slug: String = job
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(29)
        .collect();
    let suffix: String = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(4)
        .collect();
    format!("cron-fix-{slug}-{suffix}")
}

impl Remediator {
    async fn job(&self, id: &str) -> Result<CronJob> {
        sqlx::query_as("SELECT * FROM cron_jobs WHERE id=?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    pub async fn on_problem(&self, id: i64, lead: &str) -> Result<()> {
        let result = self.on_problem_inner(id, lead).await;
        if let Err(error) = &result {
            self.record_failure(id, "diagnosis", error).await?;
        }
        result
    }

    async fn record_failure(&self, id: i64, stage: &str, error: &OmonError) -> Result<()> {
        let mut problem: Problem = sqlx::query_as(
            "SELECT id,job_id,kind,detail,status,lane_key FROM cron_problems WHERE id=?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        if matches!(problem.status.as_str(), "fixed" | "needs_owner") {
            return Ok(());
        }
        let mut detail = Detail::parse(&problem.detail);
        detail.evidence =
            Some(json!({"status":"failed", "stage":stage, "error":error.to_string()}).to_string());
        problem.detail =
            serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE cron_problems SET detail=?,status='needs_owner',lane_key=CASE WHEN EXISTS(SELECT 1 FROM lanes WHERE key=cron_problems.lane_key) THEN lane_key ELSE NULL END,updated_at=? WHERE id=? AND status!='fixed'")
            .bind(&problem.detail).bind(chrono::Utc::now()).bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        problem.lane_key = sqlx::query_scalar("SELECT lane_key FROM cron_problems WHERE id=?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        self.needs_owner(&problem, &format!("Remediation failed during {stage}: {error}; inspect recorded evidence before approving another attempt")).await
    }

    async fn on_problem_inner(&self, id: i64, lead: &str) -> Result<()> {
        let Some(mut problem) = sqlx::query_as::<_, Problem>("SELECT id,job_id,kind,detail,status,lane_key FROM cron_problems WHERE id=? AND status='open'")
            .bind(id).fetch_optional(&self.pool).await? else { return Ok(()) };
        let job = self.job(&problem.job_id).await?;
        let payload = job.payload()?;
        let output: Option<String> = sqlx::query_scalar(
            "SELECT output FROM cron_outputs WHERE job_id=? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&job.id)
        .fetch_optional(&self.pool)
        .await?;
        let detail = Detail::parse(&problem.detail);
        let state = format!("{}\n{}", detail.cause, output.as_deref().unwrap_or(""));
        let choice = self.judge.choice(Purpose::CronCause, &crate::lanes::scrub::scrub(&state),
            "Classify the root cause; OAuth/login/payment/quota needs owner; traceback needs script fix; transient may retry once",
            &["transient_retry_once", "script_bug_needs_fix", "needs_owner_login_or_quota"])
            .await.map_err(|e| OmonError::Config(e.to_string()))?;
        let key = lane_key(&job.id, id);
        let claimed = sqlx::query("UPDATE cron_problems SET status='remediating',detail=?,lane_key=?,updated_at=? WHERE id=? AND status='open'")
            .bind(serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?).bind(if choice.selected == "script_bug_needs_fix" || publishing(&job.id, &payload) { Some(&key) } else { None })
            .bind(chrono::Utc::now()).bind(id).execute(&self.pool).await?;
        if claimed.rows_affected() == 0 {
            return Ok(());
        }
        problem.detail =
            serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?;
        match choice.selected.as_str() {
            "needs_owner_login_or_quota" => {
                let action = if state.to_ascii_lowercase().contains("quota")
                    || state.contains("429")
                {
                    match self.runtime.quota_check().await {
                        Ok(quota) => format!("Inspect provider quota/payment or credentials; mahoquot direct result: {quota}; error: {}", detail.cause),
                        Err(error) => format!("Restore access to mahoquot 127.0.0.1:18801 before diagnosing quota; direct check failed: {error}"),
                    }
                } else {
                    format!(
                        "Restore the job's login/credentials/payment, then approve a rerun: {}",
                        detail.cause
                    )
                };
                self.needs_owner(&problem, &action).await?;
            }
            "transient_retry_once" if !publishing(&job.id, &payload) => {
                if let Some(receipt) = self.verify_once(id).await? {
                    if !receipt.verified(&payload)? {
                        self.needs_owner(&problem, "The single transient rerun failed; inspect the recorded error and approve further remediation").await?;
                    }
                }
            }
            "script_bug_needs_fix" | "transient_retry_once" => {
                let runs: Vec<(String, Option<String>)> = sqlx::query_as("SELECT status,error FROM cron_runs WHERE job_id=? ORDER BY started_at DESC LIMIT 3")
                    .bind(&job.id).fetch_all(&self.pool).await?;
                let logs = self.runtime.log_excerpt(&job.id).await?;
                let request =
                    make_lane(&job, &problem, &key, lead, &runs, output.as_deref(), &logs)?;
                self.runtime.open_lane(request, self.judge.as_ref()).await?;
                if publishing(&job.id, &payload) {
                    problem.lane_key = Some(key);
                    self.needs_owner(
                        &problem,
                        "Review the prepared fix and explicitly approve the publishing rerun",
                    )
                    .await?;
                }
            }
            other => return Err(OmonError::Config(format!("invalid cron cause: {other}"))),
        }
        Ok(())
    }

    pub async fn needs_owner(&self, problem: &Problem, action: &str) -> Result<()> {
        let now = chrono::Utc::now();
        let item = format!("cron:{}", problem.id);
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE cron_problems SET status='needs_owner',updated_at=? WHERE id=? AND status!='fixed'")
            .bind(now).bind(problem.id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO inbox_items(id,source,source_ref,title,snippet,received_at,raw_hash,collected_at) VALUES(?,'cron',?,?,?,?,'cron',?) ON CONFLICT(id) DO NOTHING")
            .bind(&item).bind(problem.id.to_string()).bind(&problem.job_id).bind(action).bind(now).bind(now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO inbox_decisions(item_id,class,reason,proposed_action,lane_key,status,decided_at) VALUES(?,'owner',?,?,?,'open',?) ON CONFLICT(item_id) DO UPDATE SET proposed_action=excluded.proposed_action,lane_key=excluded.lane_key")
            .bind(&item).bind(&problem.kind).bind(action).bind(&problem.lane_key).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
        self.runtime
            .outcome(
                problem,
                &super::report::owner_message(&problem.job_id, action),
            )
            .await
    }

    pub async fn verify_once(&self, id: i64) -> Result<Option<RunReceipt>> {
        let result = self.verify_once_inner(id).await;
        if let Err(error) = &result {
            self.record_failure(id, "verification", error).await?;
        }
        result
    }

    async fn verify_once_inner(&self, id: i64) -> Result<Option<RunReceipt>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE cron_problems SET updated_at=updated_at WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let problem: Problem = sqlx::query_as(
            "SELECT id,job_id,kind,detail,status,lane_key FROM cron_problems WHERE id=?",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let mut detail = Detail::parse(&problem.detail);
        let job: CronJob = sqlx::query_as("SELECT * FROM cron_jobs WHERE id=?")
            .bind(&problem.job_id)
            .fetch_one(&mut *tx)
            .await?;
        let payload = job.payload()?;
        if matches!(problem.status.as_str(), "fixed" | "needs_owner" | "ignored") {
            tx.rollback().await?;
            return Ok(None);
        }
        if detail.reruns >= MAX_RERUNS || (publishing(&job.id, &payload) && !detail.owner_approved)
        {
            tx.rollback().await?;
            self.needs_owner(&problem, "Approve further remediation; rerun budget exhausted or publishing approval required").await?;
            return Ok(None);
        }
        detail.reruns = detail.reruns.saturating_add(1);
        sqlx::query("UPDATE cron_problems SET detail=?,updated_at=? WHERE id=?")
            .bind(serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?)
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let receipt = self.runtime.rerun(&job.id).await?;
        detail.last_run_id = Some(receipt.run_id.clone());
        detail.evidence =
            Some(serde_json::to_string(&receipt).map_err(|e| OmonError::Config(e.to_string()))?);
        let verified = receipt.verified(&payload)?;
        sqlx::query("UPDATE cron_problems SET detail=?,status=?,updated_at=? WHERE id=?")
            .bind(serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?)
            .bind(if verified { "fixed" } else { "needs_owner" })
            .bind(chrono::Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await?;
        if verified {
            self.runtime
                .outcome(
                    &problem,
                    &super::report::outcome_message(
                        &job.id,
                        &detail.cause,
                        "verified native rerun",
                        &receipt,
                    )?,
                )
                .await?;
        } else {
            self.needs_owner(
                &problem,
                &format!("Verification failed; inspect recorded machine results before approving another attempt: {}", detail.evidence.as_deref().unwrap_or("")),
            )
            .await?;
        }
        Ok(Some(receipt))
    }

    pub async fn approve_rerun(&self, id: i64) -> Result<Option<RunReceipt>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE cron_problems SET updated_at=updated_at WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let text: String = sqlx::query_scalar(
            "SELECT detail FROM cron_problems WHERE id=? AND status='needs_owner'",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let mut detail = Detail::parse(&text);
        detail.owner_approved = true;
        sqlx::query(
            "UPDATE cron_problems SET detail=?,status='remediating',updated_at=? WHERE id=?",
        )
        .bind(serde_json::to_string(&detail).map_err(|e| OmonError::Config(e.to_string()))?)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.verify_once(id).await
    }
}

pub fn make_lane(
    job: &CronJob,
    problem: &Problem,
    key: &str,
    lead: &str,
    runs: &[(String, Option<String>)],
    output: Option<&str>,
    logs: &str,
) -> Result<LaneRequest> {
    let payload = job.payload()?;
    // Prefer the job's explicit working directory. `script` is a command line
    // (e.g. "/usr/bin/python3 /path/job.py"), so only its last absolute
    // argument that is an existing file can name a directory.
    let cwd = ["workdir", "cwd"]
        .iter()
        .find_map(|key| payload.get(*key).and_then(Value::as_str))
        .map(PathBuf::from)
        .or_else(|| {
            payload
                .get("script")
                .and_then(Value::as_str)
                .and_then(|script| {
                    let args: Vec<&Path> = script.split_whitespace().map(Path::new).collect();
                    // A bare path keeps its parent; a command line uses its
                    // last absolute argument (e.g. the .py after python3).
                    if args.len() == 1 {
                        args[0].parent()
                    } else {
                        args.into_iter()
                            .rev()
                            .find(|arg| arg.is_absolute())
                            .and_then(Path::parent)
                    }
                })
                .filter(|p| !p.as_os_str().is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from("/Users/indo/code/project/omon-gateway"));
    let done = if publishing(&job.id, &payload) {
        DoneSignal::File {
            path: PathBuf::from(format!(".omo/cron-fix/{key}.ready")),
            sha256: None,
            json_path: None,
            json_value: None,
        }
        .to_string()
    } else {
        DoneSignal::Command {
            argv: vec![
                "omo-gateway".into(),
                "cron".into(),
                "run-now".into(),
                job.id.clone(),
                "--wait".into(),
                "--json".into(),
            ],
            stdout_regex: Some("\"status\":\"succeeded\"".into()),
        }
        .to_string()
    };
    let brief = format!("Job: {} / {}\nExpression: {}\nProblem: {} / {}\nLast 3 runs: {}\nLast output: {}\nScript or prompt: {}\nGateway log (UTC):\n{}\nRules: no .env edits; no model changes; no job disable/delete; quota claims require direct mahoquot 127.0.0.1:18801 check; katok forbidden. Diagnose, fix the root cause in the script/config/repo, then claim done. Do not execute the Done command yourself: gateway settlement owns the single verification rerun and validates output regex and delivery receipt.\nDone = {}\n",
        job.id, payload.get("name").and_then(Value::as_str).unwrap_or(&job.id), job.expression, problem.kind, Detail::parse(&problem.detail).cause, serde_json::to_string(runs).map_err(|e| OmonError::Config(e.to_string()))?, output.unwrap_or("").chars().take(4000).collect::<String>(), payload.get("script").or_else(|| payload.get("prompt")).unwrap_or(&Value::Null), logs.lines().take(80).collect::<Vec<_>>().join("\n").chars().take(4000).collect::<String>(), done);
    Ok(LaneRequest {
        key: key.into(),
        cwd,
        brief,
        done,
        lead: lead.into(),
    })
}

pub struct GatewayRuntime {
    pub pool: SqlitePool,
    pub scheduler: CronScheduler,
    pub briefs: PathBuf,
}

#[async_trait]
impl Runtime for GatewayRuntime {
    async fn open_lane(&self, request: LaneRequest, judge: &dyn Judge) -> Result<()> {
        tokio::fs::create_dir_all(&self.briefs)
            .await
            .map_err(|e| OmonError::Config(e.to_string()))?;
        let brief = self.briefs.join(format!("{}.md", request.key));
        tokio::fs::write(&brief, request.brief)
            .await
            .map_err(|e| OmonError::Config(e.to_string()))?;
        crate::lanes::cli::execute(
            &self.pool,
            crate::lanes::cli::LaneCommand::Start(crate::lanes::cli::StartArgs {
                key: request.key,
                title: "Cron root-cause remediation".into(),
                brief,
                done: request.done,
                thread: None,
                cwd: Some(request.cwd),
                lead: Some(request.lead),
                model: crate::lanes::cli::LaneModel::Auto,
            }),
            judge,
        )
        .await?;
        Ok(())
    }
    async fn rerun(&self, job: &str) -> Result<RunReceipt> {
        Box::pin(self.scheduler.trigger_wait(job)).await
    }
    async fn quota_check(&self) -> Result<Value> {
        let response = reqwest::Client::new()
            .get("http://127.0.0.1:18801")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| OmonError::Config(format!("mahoquot direct check failed: {e}")))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| OmonError::Config(e.to_string()))?;
        Ok(json!({"http_status":status,"body":body}))
    }
    async fn log_excerpt(&self, job: &str) -> Result<String> {
        let home = std::env::var("HOME").map_err(|e| OmonError::Config(e.to_string()))?;
        let run_ids: Vec<String> = sqlx::query_scalar(
            "SELECT run_id FROM cron_runs WHERE job_id=? ORDER BY started_at DESC LIMIT 3",
        )
        .bind(job)
        .fetch_all(&self.pool)
        .await?;
        // A missing log (fresh install, rotation) must not abort remediation:
        // the brief still carries the recorded error and run ids.
        let log =
            match tokio::fs::read_to_string(PathBuf::from(home).join(".omon/gateway.log")).await {
                Ok(log) => log,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok("(gateway log not found; no excerpt)".to_owned());
                }
                Err(error) => return Err(OmonError::Config(error.to_string())),
            };
        let mut lines: Vec<&str> = log
            .lines()
            .rev()
            .filter(|line| line.contains(job) || run_ids.iter().any(|id| line.contains(id)))
            .take(80)
            .collect();
        lines.reverse();
        Ok(lines.join("\n"))
    }
    async fn outcome(&self, problem: &Problem, message: &str) -> Result<()> {
        super::report::queue_outcome(
            &self.pool,
            &self.scheduler,
            problem.id,
            &problem.job_id,
            message,
        )
        .await
    }
}

pub async fn reconcile_fixed(pool: &SqlitePool, job: &str) -> Result<()> {
    let lanes: Vec<String> = sqlx::query_scalar("SELECT lane_key FROM cron_problems WHERE job_id=? AND status='fixed' AND lane_key IS NOT NULL")
        .bind(job).fetch_all(pool).await?;
    for key in lanes {
        let mut tx = pool.begin().await?;
        let from: Option<String> = sqlx::query_scalar(
            "SELECT status FROM lanes WHERE key=? AND status NOT IN ('verified_done','closed')",
        )
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(from) = from {
            sqlx::query(
                "UPDATE lanes SET status='verified_done',updated_at=? WHERE key=? AND status=?",
            )
            .bind(chrono::Utc::now())
            .bind(&key)
            .bind(&from)
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO lane_history(lane_key,at,from_status,to_status,reason) VALUES(?,?,?,'verified_done','cron natural clean success')").bind(&key).bind(chrono::Utc::now()).bind(from).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

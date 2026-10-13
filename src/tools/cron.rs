use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, SqlitePool};

use super::terminal::ApprovalPolicy;
use super::Tool;
use crate::{
    check_gateway_lifecycle, next_run, scan_cron_prompt, CronJobSpec, CronScheduler, OmonError,
};

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct DbCronJob {
    pub id: String,
    pub session_key: Option<String>,
    pub expression: String,
    pub payload_json: String,
    pub enabled: bool,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
pub struct DbCronRun {
    pub run_id: String,
    pub job_id: String,
    pub lease_expires_at: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub attempt: i64,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct CronTool {
    pool: SqlitePool,
    scheduler: Arc<parking_lot::RwLock<Option<Arc<CronScheduler>>>>,
    deny_globs: Vec<String>,
    approval_policy: Option<ApprovalPolicy>,
}

async fn check_imported_read_only(pool: &SqlitePool, id: &str) -> Result<(), OmonError> {
    if id.starts_with("hermes:") {
        return Err(OmonError::ToolExecution(format!(
            "cannot modify imported job '{id}' (read-only)"
        )));
    }
    let authority: Option<(String,)> =
        sqlx::query_as("SELECT authority FROM cron_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| OmonError::Database(e.to_string()))?;
    if let Some((auth,)) = authority {
        if auth == "hermes_mirror" || auth == "hermes_synced" {
            return Err(OmonError::ToolExecution(format!(
                "cannot modify imported job '{id}' (read-only)"
            )));
        }
    }
    Ok(())
}

/// Agent-authored cron scripts run unattended through a shell, so they get the
/// terminal tool's policy: hardline and dangerous commands and user deny globs
/// (`APPROVALS_DENY`) are refused outright; everything else needs the same
/// interactive approval gate as the terminal (see `CronTool::requires_approval`).
fn check_cron_script(script: &str, deny_globs: &[String]) -> Result<(), OmonError> {
    check_gateway_lifecycle(script).map_err(OmonError::ToolExecution)?;
    if let Some(reason) = crate::security::detect_hardline_command(script) {
        return Err(OmonError::ToolExecution(format!(
            "cron script rejected (hardline): {reason}"
        )));
    }
    if let Some(pattern) = crate::security::match_user_deny_rule(script, deny_globs) {
        return Err(OmonError::ToolExecution(format!(
            "cron script rejected: it matches the user-defined deny rule '{pattern}' (APPROVALS_DENY)"
        )));
    }
    if let Some(finding) = crate::security::detect_dangerous_command(script) {
        return Err(OmonError::ToolExecution(format!(
            "cron script rejected (dangerous command): {}",
            finding.description
        )));
    }
    Ok(())
}

/// Deny globs configured through the environment, as the runtime reads them.
fn env_deny_globs() -> Vec<String> {
    std::env::var("APPROVALS_DENY")
        .or_else(|_| std::env::var("OMON_APPROVALS_DENY"))
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

impl CronTool {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            scheduler: Arc::new(parking_lot::RwLock::new(None)),
            deny_globs: Vec::new(),
            approval_policy: None,
        }
    }

    pub fn with_scheduler(pool: SqlitePool, scheduler: Arc<CronScheduler>) -> Self {
        Self {
            pool,
            scheduler: Arc::new(parking_lot::RwLock::new(Some(scheduler))),
            deny_globs: Vec::new(),
            approval_policy: None,
        }
    }

    /// Extra deny globs, applied on top of the `APPROVALS_DENY` environment value.
    pub fn with_deny_globs(mut self, globs: Vec<String>) -> Self {
        self.deny_globs = globs;
        self
    }

    /// Overrides the policy otherwise read from `APPROVAL_MODE`.
    pub fn with_approval_policy(mut self, policy: ApprovalPolicy) -> Self {
        self.approval_policy = Some(policy);
        self
    }

    fn effective_deny_globs(&self) -> Vec<String> {
        let mut globs = env_deny_globs();
        for glob in &self.deny_globs {
            if !globs.contains(glob) {
                globs.push(glob.clone());
            }
        }
        globs
    }

    fn effective_approval_policy(&self) -> ApprovalPolicy {
        self.approval_policy.unwrap_or_else(|| {
            ApprovalPolicy::parse(std::env::var("APPROVAL_MODE").ok().as_deref())
        })
    }

    pub fn bind_scheduler(&self, scheduler: Arc<CronScheduler>) {
        *self.scheduler.write() = Some(scheduler);
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Manage Omon-native scheduled workflows. Jobs may run an agent prompt, a script, or both. \
        Hermes-owned jobs are synchronized read-only from their profile stores."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "list", "get", "add", "create", "delete", "remove", "pause", "resume",
                        "trigger", "run", "run_now", "update", "runs", "status", "ack", "acknowledge", "ack_incident",
                        "notepad", "notepad_set", "notepad_get", "notepad_list", "notepad_delete"
                    ],
                    "description": "The cron operation to perform (list, get, add, delete, pause, resume, trigger, update, runs, status, ack, notepad)."
                },
                "id": {
                    "type": "string",
                    "description": "Job ID (required for get/add/delete/pause/resume/trigger/update)."
                },
                "job_id": {
                    "type": "string",
                    "description": "Alternative alias for id."
                },
                "expression": {
                    "type": "string",
                    "description": "Cron expression (e.g. '0 */2 * * *'), interval ('@every 5m'), or one-shot expression ('once:2026-08-26T13:16:00Z'). Required unless delay/once_at is supplied."
                },
                "once_at": {
                    "type": "string",
                    "description": "One-shot execution time as RFC3339, e.g. 2026-08-26T13:16:00Z. The job disables itself after execution."
                },
                "delay": {
                    "type": "string",
                    "description": "Relative one-shot delay such as 1h, 30m, or 90s. The job disables itself after execution."
                },
                "schedule": {
                    "type": "string",
                    "description": "Alternative alias for expression."
                },
                "prompt": {"type": "string", "description": "Self-contained agent task."},
                "script": {"type": "string", "description": "Script or shell command to execute."},
                "deliver": {"type": "string", "description": "Delivery target such as discord:123."},
                "enabled": {"type": "boolean", "description": "Whether the cron job is active."},
                "enabled_toolsets": {"type": "array", "items": {"type": "string"}},
                "description": {
                    "type": "string",
                    "description": "Human-readable description of the cron job."
                },
                "name": {
                    "type": "string",
                    "description": "Alternative alias for description."
                },
                "signature": {
                    "type": "string",
                    "description": "Error signature of incident to acknowledge (optional)."
                },
                "key": {
                    "type": "string",
                    "description": "Notepad key."
                },
                "value": {
                    "type": "string",
                    "description": "Notepad value (max 16 KiB)."
                },
                "op": {
                    "type": "string",
                    "description": "Notepad sub-operation (set, get, list, delete)."
                },
                "profile": {
                    "type": "string",
                    "description": "Target profile for profile-scoped operations (default: 'default')."
                }
            },
            "required": ["action"]
        })
    }

    fn requires_approval(&self, args: &Value) -> Option<String> {
        let action = args.get("action").and_then(Value::as_str)?;
        if !matches!(action, "add" | "create" | "update") {
            return None;
        }
        if self.effective_approval_policy() == ApprovalPolicy::Never {
            return None;
        }
        // Every stored shell field runs unattended, and the approval key is derived from this
        // text, so it must name all of them in full: a preview would let one grant cover a
        // different command.
        let mut parts = Vec::new();
        for key in ["script", "ack_command"] {
            if let Some(command) = args
                .get(key)
                .and_then(Value::as_str)
                .filter(|command| !command.trim().is_empty())
            {
                parts.push(format!("{key}={command}"));
            }
        }
        if parts.is_empty() {
            return None;
        }
        Some(format!(
            "schedule an unattended shell command via cron: {}",
            parts.join("; ")
        ))
    }

    async fn execute_with_context(
        &self,
        mut args: Value,
        session: Option<&crate::SessionKey>,
    ) -> Result<Value, OmonError> {
        // A Discord-created reminder must have an actual delivery target.
        // Previously the tool defaulted to `local`, so the scheduler completed
        // the job but had nowhere to send the notification.
        let is_create = matches!(
            args.get("action").and_then(Value::as_str),
            Some("add" | "create")
        );
        if is_create {
            if let Some(session) = session {
                if session.platform.eq_ignore_ascii_case("discord") {
                    if args.get("deliver").is_none()
                        || args.get("deliver").and_then(Value::as_str) == Some("local")
                    {
                        if let Some(ref tid) = session.thread_id {
                            // Native payloads parse `discord:<a>:<b>` as one chat id, so a
                            // thread target is expressed through `origin` instead.
                            args["deliver"] = Value::String("origin".into());
                            args["origin"] = json!({
                                "platform": "discord",
                                "chat_id": session.channel_id,
                                "thread_id": tid,
                            });
                        } else {
                            args["deliver"] =
                                Value::String(format!("discord:{}", session.channel_id));
                        }
                    }
                    if let Some(ref bid) = session.bot_id {
                        args["bot_id"] = Value::String(bid.clone());
                    }
                    args["_session_key"] = Value::String(session.storage_key());
                }
            }
        }
        self.execute(args).await
    }

    async fn execute(&self, args: Value) -> Result<Value, OmonError> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| OmonError::ToolExecution("missing 'action'".into()))?;

        let id_param = args
            .get("id")
            .or_else(|| args.get("job_id"))
            .and_then(Value::as_str);

        match action {
            "list" => {
                let jobs: Vec<DbCronJob> = sqlx::query_as(
                    "SELECT id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at FROM cron_jobs ORDER BY id",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| OmonError::Database(e.to_string()))?;

                Ok(json!({
                    "count": jobs.len(),
                    "cron_jobs": jobs
                }))
            }
            "get" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;

                let job: Option<DbCronJob> = sqlx::query_as(
                    "SELECT id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at FROM cron_jobs WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OmonError::Database(e.to_string()))?;

                match job {
                    Some(j) => Ok(json!(j)),
                    None => Err(OmonError::ToolExecution(format!(
                        "cron job not found: {id}"
                    ))),
                }
            }
            "delete" | "remove" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                check_imported_read_only(&self.pool, id).await?;

                let res = sqlx::query("DELETE FROM cron_jobs WHERE id = ?")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

                Ok(json!({
                    "deleted": res.rows_affected() > 0,
                    "id": id
                }))
            }
            "pause" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                check_imported_read_only(&self.pool, id).await?;

                let scheduler_opt = self.scheduler.read().clone();
                if let Some(scheduler) = &scheduler_opt {
                    let paused = scheduler.pause(id).await?;
                    if !paused {
                        return Err(OmonError::ToolExecution(format!(
                            "cron job not found: {id}"
                        )));
                    }
                } else {
                    let res = sqlx::query(
                        "UPDATE cron_jobs SET enabled = 0, next_run_at = NULL, updated_at = ? WHERE id = ?",
                    )
                    .bind(chrono::Utc::now())
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

                    if res.rows_affected() == 0 {
                        return Err(OmonError::ToolExecution(format!(
                            "cron job not found: {id}"
                        )));
                    }
                }

                Ok(json!({
                    "status": "paused",
                    "id": id,
                    "enabled": false
                }))
            }
            "resume" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                check_imported_read_only(&self.pool, id).await?;

                let scheduler_opt = self.scheduler.read().clone();
                let next_run_str = if let Some(scheduler) = &scheduler_opt {
                    let resumed = scheduler.resume(id).await?;
                    if !resumed {
                        return Err(OmonError::ToolExecution(format!(
                            "cron job not found: {id}"
                        )));
                    }
                    let job = scheduler.get(id).await?.ok_or_else(|| {
                        OmonError::ToolExecution(format!("cron job not found: {id}"))
                    })?;
                    job.next_run_at.map(|t| t.to_rfc3339())
                } else {
                    let job: Option<DbCronJob> = sqlx::query_as(
                        "SELECT id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at FROM cron_jobs WHERE id = ?",
                    )
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

                    let job = job.ok_or_else(|| {
                        OmonError::ToolExecution(format!("cron job not found: {id}"))
                    })?;

                    let now = chrono::Utc::now();
                    let next = next_run(&job.expression, now)?;
                    sqlx::query(
                        "UPDATE cron_jobs SET enabled = 1, next_run_at = ?, updated_at = ? WHERE id = ?",
                    )
                    .bind(next)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

                    Some(next.to_rfc3339())
                };

                Ok(json!({
                    "status": "resumed",
                    "id": id,
                    "enabled": true,
                    "next_run_at": next_run_str
                }))
            }
            "trigger" | "run" | "run_now" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;

                let exists: Option<(String,)> =
                    sqlx::query_as("SELECT id FROM cron_jobs WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(|e| OmonError::Database(e.to_string()))?;

                if exists.is_none() {
                    return Err(OmonError::ToolExecution(format!(
                        "cron job not found: {id}"
                    )));
                }

                let scheduler_opt = self.scheduler.read().clone();
                if let Some(scheduler) = &scheduler_opt {
                    let triggered = scheduler.trigger(id).await?;
                    if !triggered {
                        return Err(OmonError::ToolExecution(format!(
                            "failed to trigger cron job: {id}"
                        )));
                    }
                } else {
                    let now = chrono::Utc::now();
                    sqlx::query(
                        "UPDATE cron_jobs SET next_run_at = ?, updated_at = ? WHERE id = ?",
                    )
                    .bind(now)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;
                }

                Ok(json!({
                    "status": "triggered",
                    "id": id
                }))
            }
            "update" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                check_imported_read_only(&self.pool, id).await?;

                let job: Option<DbCronJob> = sqlx::query_as(
                    "SELECT id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at FROM cron_jobs WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| OmonError::Database(e.to_string()))?;

                let job = job
                    .ok_or_else(|| OmonError::ToolExecution(format!("cron job not found: {id}")))?;

                let mut payload: Value =
                    serde_json::from_str(&job.payload_json).unwrap_or_else(|_| json!({}));

                if let Some(p) = args.get("prompt").and_then(Value::as_str) {
                    check_gateway_lifecycle(p).map_err(OmonError::ToolExecution)?;
                    let threats = scan_cron_prompt(p);
                    if !threats.is_empty() {
                        return Err(OmonError::ToolExecution(format!(
                            "cron prompt rejected due to security threats: {}",
                            threats.join(", ")
                        )));
                    }
                    payload["prompt"] = Value::String(p.to_string());
                }

                let deny_globs = self.effective_deny_globs();
                if let Some(s) = args.get("script").and_then(Value::as_str) {
                    check_cron_script(s, &deny_globs)?;
                    payload["script"] = Value::String(s.to_string());
                }

                if let Some(ack) = args.get("ack_command") {
                    // Same unattended `sh -c` as a script: screen it identically.
                    if let Some(command) = ack.as_str() {
                        check_cron_script(command, &deny_globs)?;
                    }
                    payload["ack_command"] = ack.clone();
                }

                if let Some(desc) = args
                    .get("description")
                    .or_else(|| args.get("name"))
                    .and_then(Value::as_str)
                {
                    payload["name"] = Value::String(desc.to_string());
                }

                if let Some(deliver) = args.get("deliver").and_then(Value::as_str) {
                    payload["deliver"] = Value::String(deliver.to_string());
                }

                if let Some(toolsets) = args.get("enabled_toolsets") {
                    payload["enabled_toolsets"] = toolsets.clone();
                }

                let expression = args
                    .get("expression")
                    .or_else(|| args.get("schedule"))
                    .and_then(Value::as_str)
                    .unwrap_or(&job.expression);

                let enabled = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(job.enabled);

                let expression_changed = args
                    .get("expression")
                    .or_else(|| args.get("schedule"))
                    .and_then(Value::as_str)
                    .is_some_and(|new_expr| new_expr != job.expression);

                let enabled_changed = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .is_some_and(|new_enabled| new_enabled != job.enabled);

                let now = chrono::Utc::now();
                if expression_changed {
                    // Reject an invalid new expression even when the job is disabled.
                    next_run(expression, now)?;
                }
                let next_run_at: Option<String> = if !enabled {
                    None
                } else if expression_changed || (enabled_changed && job.next_run_at.is_none()) {
                    Some(next_run(expression, now)?.to_rfc3339())
                } else {
                    job.next_run_at
                };

                let payload_json = serde_json::to_string(&payload)
                    .map_err(|error| OmonError::ToolExecution(error.to_string()))?;

                sqlx::query(
                    "UPDATE cron_jobs \
                     SET expression = ?, payload_json = ?, enabled = ?, next_run_at = ?, updated_at = ? \
                     WHERE id = ?",
                )
                .bind(expression)
                .bind(&payload_json)
                .bind(enabled)
                .bind(&next_run_at)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| OmonError::Database(e.to_string()))?;

                Ok(json!({
                    "status": "updated",
                    "id": id,
                    "expression": expression,
                    "enabled": enabled,
                    "next_run_at": next_run_at
                }))
            }
            "add" | "create" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                check_imported_read_only(&self.pool, id).await?;
                let existing: Option<(String,)> =
                    sqlx::query_as("SELECT id FROM cron_jobs WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(|e| OmonError::Database(e.to_string()))?;
                if existing.is_some() {
                    return Err(OmonError::ToolExecution(format!(
                        "cron job '{id}' already exists; use 'update' to change it"
                    )));
                }
                let now = chrono::Utc::now();
                let expression = if let Some(once_at) = args.get("once_at").and_then(Value::as_str)
                {
                    // Validate the timestamp immediately and store the canonical one-shot form.
                    let next = next_run(&format!("once:{once_at}"), now)?;
                    format!("once:{}", next.to_rfc3339())
                } else if let Some(delay) = args.get("delay").and_then(Value::as_str) {
                    // Relative delays are converted to an absolute one-shot timestamp at creation time.
                    let next = next_run(&format!("interval:{delay}"), now)?;
                    format!("once:{}", next.to_rfc3339())
                } else {
                    args.get("expression")
                        .or_else(|| args.get("schedule"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            OmonError::ToolExecution(
                                "missing 'expression' (or once_at/delay)".into(),
                            )
                        })?
                        .to_string()
                };
                let prompt = args.get("prompt").and_then(Value::as_str);
                let script = args.get("script").and_then(Value::as_str);
                if prompt.is_none() && script.is_none() {
                    return Err(OmonError::ToolExecution(
                        "add requires at least one of 'prompt' or 'script'".into(),
                    ));
                }
                if let Some(p) = prompt {
                    check_gateway_lifecycle(p).map_err(OmonError::ToolExecution)?;
                    let threats = scan_cron_prompt(p);
                    if !threats.is_empty() {
                        return Err(OmonError::ToolExecution(format!(
                            "cron prompt rejected due to security threats: {}",
                            threats.join(", ")
                        )));
                    }
                }
                let deny_globs = self.effective_deny_globs();
                if let Some(s) = script {
                    check_cron_script(s, &deny_globs)?;
                }
                let desc = args
                    .get("description")
                    .or_else(|| args.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let mut payload = json!({
                    "name": desc,
                    "prompt": prompt.unwrap_or_default(),
                    "script": script,
                    "deliver": args.get("deliver").and_then(Value::as_str).unwrap_or("local"),
                    "bot_id": args.get("bot_id").and_then(Value::as_str),
                    "enabled_toolsets": args.get("enabled_toolsets").cloned().unwrap_or(Value::Null)
                });
                if let Some(origin) = args.get("origin") {
                    payload["origin"] = origin.clone();
                }
                if let Some(repeat) = args.get("repeat") {
                    payload["repeat"] = repeat.clone();
                }
                if let Some(context_from) = args.get("context_from") {
                    payload["context_from"] = context_from.clone();
                }
                if let Some(skills) = args.get("skills") {
                    payload["skills"] = skills.clone();
                }
                if let Some(skill) = args.get("skill") {
                    payload["skill"] = skill.clone();
                }
                if let Some(no_agent) = args.get("no_agent") {
                    payload["no_agent"] = no_agent.clone();
                }
                if let Some(ack) = args.get("ack_command") {
                    // The ack runs unattended through `sh -c` after delivery, so it gets the
                    // same hardline/dangerous screening as a cron script.
                    if let Some(command) = ack.as_str() {
                        check_cron_script(command, &deny_globs)?;
                    }
                    payload["ack_command"] = ack.clone();
                }
                if let Some(timeout) = args.get("timeout").or_else(|| args.get("timeout_secs")) {
                    payload["timeout_secs"] = timeout.clone();
                }
                if let Some(model) = args.get("model") {
                    payload["model"] = model.clone();
                }
                let enabled = args.get("enabled").and_then(Value::as_bool).unwrap_or(true);
                payload["enabled"] = Value::Bool(enabled);

                let session_key = args
                    .get("_session_key")
                    .and_then(Value::as_str)
                    .map(str::to_owned);

                let scheduler_opt = self.scheduler.read().clone();
                if let Some(scheduler) = &scheduler_opt {
                    let mut spec = CronJobSpec::new(&expression, payload.clone());
                    spec.session_key = session_key.clone();
                    let mut job = scheduler.register_with_id(id, spec).await?;
                    if !enabled {
                        scheduler.pause(id).await?;
                        job.enabled = false;
                    }
                    return Ok(json!({
                        "status": "registered",
                        "id": id,
                        "expression": expression,
                        "enabled": job.enabled,
                        "next_run_at": job.next_run_at
                    }));
                }

                let payload_json = serde_json::to_string(&payload)
                    .map_err(|error| OmonError::ToolExecution(error.to_string()))?;
                let next_run_at = if enabled {
                    Some(next_run(&expression, now)?)
                } else {
                    None
                };

                sqlx::query(
                    "INSERT INTO cron_jobs (id, session_key, expression, payload_json, enabled, next_run_at, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(id) DO UPDATE SET session_key=excluded.session_key, expression=excluded.expression, payload_json=excluded.payload_json,
                     enabled=excluded.enabled, next_run_at=excluded.next_run_at, updated_at=excluded.updated_at",
                )
                .bind(id)
                .bind(session_key)
                .bind(&expression)
                .bind(payload_json)
                .bind(enabled)
                .bind(next_run_at)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| OmonError::Database(e.to_string()))?;

                Ok(json!({
                    "status": "registered",
                    "id": id,
                    "expression": expression,
                    "one_shot": expression.starts_with("once:"),
                    "prompt": prompt,
                    "script": script
                }))
            }
            "runs" => {
                let limit = args
                    .get("limit")
                    .and_then(Value::as_i64)
                    .unwrap_or(20)
                    .clamp(1, 100);

                let rows: Vec<DbCronRun> = if let Some(job_id) = id_param {
                    sqlx::query_as(
                        "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error
                         FROM cron_runs WHERE job_id = ? ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(job_id)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?
                } else {
                    sqlx::query_as(
                        "SELECT run_id, job_id, lease_expires_at, started_at, completed_at, status, attempt, error
                         FROM cron_runs ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?
                };

                let runs: Vec<Value> = rows
                    .into_iter()
                    .map(|r| {
                        json!({
                            "run_id": r.run_id,
                            "job_id": r.job_id,
                            "lease_expires_at": r.lease_expires_at,
                            "started_at": r.started_at,
                            "completed_at": r.completed_at,
                            "status": r.status,
                            "attempt": r.attempt,
                            "error": r.error
                        })
                    })
                    .collect();

                Ok(json!({ "runs": runs }))
            }
            "status" => {
                let total_jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

                let enabled_jobs: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs WHERE enabled = 1")
                        .fetch_one(&self.pool)
                        .await
                        .map_err(|e| OmonError::Database(e.to_string()))?;

                let paused_jobs = total_jobs - enabled_jobs;

                let scheduler_opt = self.scheduler.read().clone();
                let (running, active_claims, ticker_health) = if let Some(scheduler) = scheduler_opt
                {
                    let running = scheduler.is_running().await;
                    let active = scheduler.active_executions_count().await;
                    let health = if running { "healthy" } else { "stopped" };
                    (running, active, health)
                } else {
                    (true, 0, "unbound")
                };

                Ok(json!({
                    "status": "ok",
                    "running": running,
                    "total_jobs": total_jobs,
                    "enabled_jobs": enabled_jobs,
                    "paused_jobs": paused_jobs,
                    "active_executions": active_claims,
                    "ticker": {
                        "health": ticker_health,
                        "running": running
                    }
                }))
            }
            "ack" | "acknowledge" | "ack_incident" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                let signature = args.get("signature").and_then(Value::as_str);

                let scheduler_opt = self.scheduler.read().clone();
                let acknowledged = if let Some(scheduler) = &scheduler_opt {
                    scheduler.acknowledge_incident(id, signature).await?
                } else {
                    let now = chrono::Utc::now().to_rfc3339();
                    let rows = if let Some(sig) = signature {
                        sqlx::query(
                            "UPDATE cron_incidents SET acknowledged = 1, updated_at = ? WHERE job_id = ? AND error_signature = ?",
                        )
                        .bind(&now)
                        .bind(id)
                        .bind(sig)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| OmonError::Database(e.to_string()))?
                        .rows_affected()
                    } else {
                        sqlx::query(
                            "UPDATE cron_incidents SET acknowledged = 1, updated_at = ? WHERE job_id = ?",
                        )
                        .bind(&now)
                        .bind(id)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| OmonError::Database(e.to_string()))?
                        .rows_affected()
                    };
                    rows > 0
                };

                Ok(json!({
                    "status": "acknowledged",
                    "id": id,
                    "acknowledged": acknowledged
                }))
            }
            "notepad" | "notepad_set" | "notepad_get" | "notepad_list" | "notepad_delete" => {
                let id = id_param.ok_or_else(|| OmonError::ToolExecution("missing 'id'".into()))?;
                let profile = args
                    .get("profile")
                    .and_then(Value::as_str)
                    .unwrap_or("default");

                let op = if action == "notepad" {
                    args.get("op").and_then(Value::as_str).unwrap_or("list")
                } else {
                    action.trim_start_matches("notepad_")
                };

                match op {
                    "set" => {
                        let key = args
                            .get("key")
                            .and_then(Value::as_str)
                            .ok_or_else(|| OmonError::ToolExecution("missing 'key'".into()))?;
                        let value = args
                            .get("value")
                            .and_then(Value::as_str)
                            .ok_or_else(|| OmonError::ToolExecution("missing 'value'".into()))?;

                        crate::cron::set_cron_notepad(&self.pool, profile, id, key, value).await?;
                        Ok(json!({
                            "status": "ok",
                            "profile": profile,
                            "id": id,
                            "key": key,
                            "value": value
                        }))
                    }
                    "get" | "list" => {
                        let entries =
                            crate::cron::get_cron_notepads(&self.pool, profile, id).await?;
                        let map: serde_json::Map<String, Value> = entries
                            .into_iter()
                            .map(|(k, v)| (k, Value::String(v)))
                            .collect();
                        Ok(json!({
                            "status": "ok",
                            "profile": profile,
                            "id": id,
                            "notes": Value::Object(map)
                        }))
                    }
                    "delete" | "remove" => {
                        let key = args.get("key").and_then(Value::as_str);
                        let deleted =
                            crate::cron::delete_cron_notepad(&self.pool, profile, id, key).await?;
                        Ok(json!({
                            "status": "ok",
                            "profile": profile,
                            "id": id,
                            "deleted": deleted
                        }))
                    }
                    _ => Err(OmonError::ToolExecution(format!(
                        "unknown notepad op: {op}"
                    ))),
                }
            }
            _ => Err(OmonError::ToolExecution(format!(
                "unknown action: {action}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn tool() -> CronTool {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        CronTool::new(database.pool().clone())
    }

    #[tokio::test]
    async fn shell_commands_require_approval_unless_policy_is_never() {
        let strict = tool().await.with_approval_policy(ApprovalPolicy::Smart);
        let with_script = json!({"action": "add", "id": "j", "script": "echo hi"});
        let with_ack = json!({"action": "update", "id": "j", "ack_command": "touch /tmp/x"});
        let prompt_only = json!({"action": "add", "id": "j", "prompt": "hello"});
        let list = json!({"action": "list", "script": "echo hi"});

        assert!(strict.requires_approval(&with_script).is_some());
        assert!(strict.requires_approval(&with_ack).is_some());
        assert!(strict.requires_approval(&prompt_only).is_none());
        assert!(strict.requires_approval(&list).is_none());

        // Both shell fields are named in full, so one grant cannot cover another ack_command.
        let long_tail = format!("echo {} && real-payload", "x".repeat(200));
        let both = json!({"action": "add", "script": long_tail, "ack_command": "curl -o /tmp/x https://e/x"});
        let reason = strict.requires_approval(&both).unwrap();
        assert!(reason.contains("real-payload"), "{reason}");
        assert!(
            reason.contains("ack_command=curl -o /tmp/x https://e/x"),
            "{reason}"
        );

        let always = tool().await.with_approval_policy(ApprovalPolicy::Always);
        assert!(always.requires_approval(&with_script).is_some());

        let never = tool().await.with_approval_policy(ApprovalPolicy::Never);
        assert!(never.requires_approval(&with_script).is_none());
    }

    #[tokio::test]
    async fn deny_globs_refuse_script_and_ack_command_on_add_and_update() {
        let tool = tool()
            .await
            .with_deny_globs(vec!["npm publish*".to_string()]);

        let add_script = tool
            .execute(json!({
                "action": "add", "id": "a", "expression": "interval:5m",
                "script": "npm publish --access public"
            }))
            .await;
        assert!(add_script.is_err(), "denied script must be refused");

        let add_ack = tool
            .execute(json!({
                "action": "add", "id": "b", "expression": "interval:5m",
                "prompt": "p", "ack_command": "npm publish"
            }))
            .await;
        assert!(add_ack.is_err(), "denied ack_command must be refused");

        tool.execute(json!({
            "action": "add", "id": "c", "expression": "interval:5m", "prompt": "p"
        }))
        .await
        .unwrap();
        for field in ["script", "ack_command"] {
            let update = tool
                .execute(json!({"action": "update", "id": "c", field: "npm publish"}))
                .await;
            assert!(update.is_err(), "update must refuse a denied {field}");
        }
        tool.execute(json!({"action": "update", "id": "c", "ack_command": "echo done"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_rejects_dangerous_ack_command_and_invalid_expression() {
        let tool = tool().await;
        tool.execute(json!({
            "action": "add", "id": "d", "expression": "interval:5m", "prompt": "p"
        }))
        .await
        .unwrap();

        assert!(tool
            .execute(json!({"action": "update", "id": "d", "ack_command": "rm -rf /"}))
            .await
            .is_err());
        assert!(tool
            .execute(json!({"action": "update", "id": "d", "expression": "not a schedule"}))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn runs_output_omits_claim_token() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let pool = database.pool().clone();
        let tool = CronTool::new(pool.clone());
        sqlx::query(
            "INSERT INTO cron_jobs (id, expression, payload_json, enabled, created_at, updated_at)
             VALUES ('j', 'interval:5m', '{}', 1, datetime('now'), datetime('now'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO cron_runs (run_id, job_id, claim_token, lease_expires_at, started_at, status, attempt)
             VALUES ('r1', 'j', 'secret-claim', datetime('now'), datetime('now'), 'running', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let runs = tool
            .execute(json!({"action": "runs", "id": "j"}))
            .await
            .unwrap();

        assert_eq!(runs["runs"][0]["run_id"], "r1");
        assert!(runs["runs"][0].get("claim_token").is_none(), "{runs}");
    }

    #[tokio::test]
    async fn test_cron_tool_lifecycle_actions() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        let tool = CronTool::new(database.pool().clone());

        // 1. Add job
        let add_res = tool
            .execute(json!({
                "action": "add",
                "id": "my_job",
                "expression": "interval:5m",
                "prompt": "Say hello",
                "description": "Daily greeting"
            }))
            .await
            .unwrap();
        assert_eq!(add_res["status"], "registered");
        assert_eq!(add_res["id"], "my_job");

        // 2. Get job
        let get_res = tool
            .execute(json!({
                "action": "get",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(get_res["id"], "my_job");
        assert_eq!(get_res["enabled"], true);

        // 3. List jobs
        let list_res = tool
            .execute(json!({
                "action": "list"
            }))
            .await
            .unwrap();
        assert_eq!(list_res["count"], 1);

        // 4. Pause job
        let pause_res = tool
            .execute(json!({
                "action": "pause",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(pause_res["status"], "paused");
        assert_eq!(pause_res["enabled"], false);

        let get_paused = tool
            .execute(json!({
                "action": "get",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(get_paused["enabled"], false);
        assert!(get_paused["next_run_at"].is_null());

        // 5. Resume job
        let resume_res = tool
            .execute(json!({
                "action": "resume",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(resume_res["status"], "resumed");
        assert_eq!(resume_res["enabled"], true);
        assert!(resume_res["next_run_at"].is_string());

        // 6. Update job
        let update_res = tool
            .execute(json!({
                "action": "update",
                "id": "my_job",
                "expression": "interval:10m",
                "prompt": "Updated greeting"
            }))
            .await
            .unwrap();
        assert_eq!(update_res["status"], "updated");
        assert_eq!(update_res["expression"], "interval:10m");

        let get_updated = tool
            .execute(json!({
                "action": "get",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(get_updated["expression"], "interval:10m");
        let payload: Value =
            serde_json::from_str(get_updated["payload_json"].as_str().unwrap()).unwrap();
        assert_eq!(payload["prompt"], "Updated greeting");

        // 7. Trigger job
        let trigger_res = tool
            .execute(json!({
                "action": "trigger",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(trigger_res["status"], "triggered");

        // 8. Delete job
        let delete_res = tool
            .execute(json!({
                "action": "delete",
                "id": "my_job"
            }))
            .await
            .unwrap();
        assert_eq!(delete_res["deleted"], true);

        // Verify gone
        assert!(tool
            .execute(json!({
                "action": "get",
                "id": "my_job"
            }))
            .await
            .is_err());
    }
}

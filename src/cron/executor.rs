use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use sqlx::SqlitePool;
use tracing::warn;

use crate::agent::AgentBackend;
use crate::cron::scheduler::{
    format_context_from_block, parse_context_from_ids, parse_wake_gate, resolve_predecessor_output,
    truncate_context_output, CronJob, CronRunOutcome, CronTaskExecutor, MAX_CONTEXT_CHARS,
};
use crate::cron::store::{resolve_cron_script_timeout, HermesJob};
use crate::error::{OmonError, Result};
use crate::models::{InboundEvent, SessionContext, SessionKey};
use crate::tools::augmented_path_from_environment;

pub struct AgentCronExecutor {
    pub backend: Arc<dyn AgentBackend>,
    pub workspace_root: PathBuf,
    pub pool: SqlitePool,
    pub cron_script_timeout_secs: u64,
}

#[async_trait]
impl CronTaskExecutor for AgentCronExecutor {
    /// Direct callers have no delivery step, so the detected monitor change is
    /// committed as soon as the run succeeds. The scheduler uses
    /// [Self::execute_with_outcome] and commits only after delivery.
    async fn execute(&self, job: &CronJob) -> Result<Option<String>> {
        let outcome = self.execute_with_outcome(job).await?;
        if let Some(state) = &outcome.monitor_state {
            commit_monitor_state(&self.pool, state).await?;
        }
        Ok(outcome.output)
    }

    async fn execute_with_outcome(&self, job: &CronJob) -> Result<CronRunOutcome> {
        let payload = job.payload()?;
        if payload.get("schedule").is_none() {
            return execute_native_cron(
                &self.backend,
                &self.workspace_root,
                job,
                &payload,
                self.cron_script_timeout_secs,
            )
            .await
            .map(CronRunOutcome::from_output);
        }
        let hermes: HermesJob = serde_json::from_value(payload).map_err(|error| {
            OmonError::Config(format!("invalid Hermes job {}: {error}", job.id))
        })?;
        let script_output = if let Some(script) = hermes.script.as_deref() {
            Some(
                run_cron_script(
                    &hermes,
                    script,
                    &self.workspace_root,
                    self.cron_script_timeout_secs,
                )
                .await?,
            )
        } else {
            None
        };
        if hermes.no_agent && hermes.script.is_none() {
            return Err(OmonError::Config(format!(
                "Hermes job {} specifies no_agent=true but has no script",
                hermes.id
            )));
        }

        if let Some(output) = script_output.as_deref() {
            if !parse_wake_gate(output) {
                tracing::info!(
                    job_id = %hermes.id,
                    "wakeAgent:false detected in script output, skipping agent execution"
                );
                return Ok(CronRunOutcome::from_output(None));
            }
            if output.trim().is_empty()
                && (!hermes.prompt.trim().is_empty() || hermes.brief_file.is_some())
            {
                tracing::info!(
                    job_id = %hermes.id,
                    "empty script output with prompt, skipping agent execution"
                );
                return Ok(CronRunOutcome::from_output(None));
            }
        }

        if hermes.no_agent {
            return Ok(CronRunOutcome::from_output(
                script_output.filter(|output| !output.trim().is_empty()),
            ));
        }

        let monitor_script = hermes.monitor_script.as_deref().or_else(|| {
            hermes
                .extra
                .get("monitor_script")
                .and_then(serde_json::Value::as_str)
        });
        let monitor_url = hermes.monitor_url.as_deref().or_else(|| {
            hermes
                .extra
                .get("monitor_url")
                .and_then(serde_json::Value::as_str)
        });

        if monitor_script.is_some() && monitor_url.is_some() {
            return Err(OmonError::Config(format!(
                "Hermes job {} has conflicting monitor modes: both monitor_script and monitor_url are set",
                hermes.id
            )));
        }

        let monitor_output = if let Some(m_script) = monitor_script {
            Some(
                run_cron_script(
                    &hermes,
                    m_script,
                    &self.workspace_root,
                    self.cron_script_timeout_secs,
                )
                .await?,
            )
        } else if let Some(m_url) = monitor_url {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(
                    self.cron_script_timeout_secs,
                ))
                .build()
                .map_err(|e| OmonError::ToolExecution(e.to_string()))?;
            let resp =
                client.get(m_url).send().await.map_err(|e| {
                    OmonError::ToolExecution(format!("monitor_url fetch failed: {e}"))
                })?;
            if !resp.status().is_success() {
                return Err(OmonError::ToolExecution(format!(
                    "monitor_url returned status {}",
                    resp.status()
                )));
            }
            Some(
                resp.text()
                    .await
                    .map_err(|e| OmonError::ToolExecution(e.to_string()))?,
            )
        } else {
            None
        };

        let mut pending_monitor_state: Option<(String, String)> = None;
        if let Some(m_output) = monitor_output {
            use sha2::{Digest, Sha256};
            let current_hash = format!("{:x}", Sha256::digest(m_output.as_bytes()));

            let prev_state: Option<(String,)> =
                sqlx::query_as("SELECT last_hash FROM cron_monitor_states WHERE job_id = ?")
                    .bind(&hermes.id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| OmonError::Database(e.to_string()))?;

            if let Some((prev_hash,)) = prev_state {
                if prev_hash == current_hash {
                    tracing::info!(
                        job_id = %hermes.id,
                        "monitor state unchanged for job {}, skipping agent execution",
                        hermes.id
                    );
                    return Ok(CronRunOutcome::from_output(None));
                }
            }

            // Committed by the scheduler only after the run's delivery succeeded, so
            // neither a failed run nor a failed delivery consumes the detected change.
            pending_monitor_state = Some((current_hash, m_output));
        }

        let effective_prompt = if let Some(ref brief_path) = hermes.brief_file {
            match tokio::fs::read_to_string(brief_path).await {
                Ok(content) => content,
                Err(err) => {
                    return Err(OmonError::Config(format!(
                        "failed to read brief_file {} for job {}: {err}",
                        brief_path.display(),
                        hermes.id
                    )));
                }
            }
        } else {
            hermes.prompt.clone()
        };

        if let Err(err) = crate::cron::check_gateway_lifecycle(&effective_prompt) {
            return Err(OmonError::Config(format!(
                "gateway lifecycle violation: {err}"
            )));
        }

        let has_skills = !hermes.skills.is_empty() || hermes.skill.is_some();
        if effective_prompt.trim().is_empty() && script_output.is_none() && !has_skills {
            return Err(OmonError::Config(format!(
                "Hermes job {} has neither prompt nor executable script",
                hermes.id
            )));
        }
        let cron_hint = "[IMPORTANT: You are running as a scheduled cron job. DELIVERY: Your final response will be automatically delivered to the user — do NOT use send_message or try to deliver the output yourself. Just produce your report/output as your final response and the system handles the rest. SILENT: If there is genuinely nothing new to report, respond with exactly \"[SILENT]\" (nothing else) to suppress delivery. Never combine [SILENT] with content — either report your findings normally, or say [SILENT] and nothing more.]\n\n";
        let mut prompt = cron_hint.to_string();

        let workdir = if let Some(custom_workdir) = hermes.workdir.as_ref() {
            let roots = authorized_cron_roots(&hermes, &self.workspace_root).ok();
            if let Some(roots) = roots {
                canonical_authorized_directory(custom_workdir, &roots, "Hermes workdir")
                    .unwrap_or_else(|_| self.workspace_root.clone())
            } else {
                self.workspace_root.clone()
            }
        } else {
            self.workspace_root.clone()
        };

        if let Some(instructions) = resolve_workspace_instructions(&workdir) {
            prompt.push_str(&instructions);
            prompt.push_str("\n\n");
        }

        let context_ids = parse_context_from_ids(hermes.context_from.as_ref());
        if !context_ids.is_empty() {
            // The scheduler stores outputs under CronJob::profile().
            let source_profile = job.profile();
            for source_id in &context_ids {
                if let Some(output) =
                    resolve_predecessor_output(&self.pool, &source_profile, source_id).await
                {
                    if !output.trim().is_empty() {
                        let truncated = truncate_context_output(output.trim(), MAX_CONTEXT_CHARS);
                        prompt.push_str(&format_context_from_block(source_id, &truncated));
                        prompt.push_str("\n\n");
                    }
                }
            }
        }

        let profile = hermes.profile();
        if let Ok(notepads) = collect_notepads(&self.pool, profile, &hermes.id).await {
            if !notepads.is_empty() {
                prompt.push_str("[Notepad]\n");
                for (k, v) in notepads {
                    prompt.push_str(&format!("{k}: {v}\n"));
                }
                prompt.push('\n');
            }
        }

        let skills = load_cron_skills(&hermes)?;
        if effective_prompt.trim().is_empty() && script_output.is_none() && skills.trim().is_empty()
        {
            return Err(OmonError::Config(format!(
                "Hermes job {} has neither prompt nor executable script or resolved skills",
                hermes.id
            )));
        }
        if !skills.is_empty() {
            prompt.push_str(&skills);
            if !effective_prompt.trim().is_empty() {
                prompt.push_str("\n\n[Task]\n");
            }
        }
        prompt.push_str(&effective_prompt);
        if let Some(output) = script_output.filter(|output| !output.trim().is_empty()) {
            prompt.push_str("\n\n[Script output]\n");
            prompt.push_str(&output);
        }
        let threats = crate::security::scan_assembled_cron_prompt(&prompt);
        if !threats.is_empty() {
            return Err(OmonError::Config(format!(
                "assembled cron prompt injection detected: {}",
                threats.join("; ")
            )));
        }
        let destination = hermes.discord_destination()?;
        let session_key = destination
            .as_ref()
            .map(|target| {
                SessionKey::new(
                    target.platform.to_ascii_lowercase(),
                    None::<String>,
                    target.chat_id.clone(),
                    target.thread_id.clone(),
                    target
                        .user_id
                        .clone()
                        .unwrap_or_else(|| format!("cron:{}", hermes.id)),
                )
            })
            .unwrap_or_else(|| {
                SessionKey::new(
                    "local",
                    None::<String>,
                    hermes.id.clone(),
                    None::<String>,
                    format!("cron:{}", hermes.id),
                )
            });
        let mut session = SessionContext::new(session_key.clone());
        session.state.active_model = hermes.model.clone();
        if let Some(ref provider) = hermes.provider {
            if hermes.base_url.is_none()
                && (provider.contains("unsupported") || provider == "invalid")
            {
                return Err(OmonError::Config(format!(
                    "unsupported cron job provider override: {provider}"
                )));
            }
            session
                .state
                .metadata
                .insert("cron_job_provider".into(), json!(provider));
        }
        if let Some(ref base_url) = hermes.base_url {
            session
                .state
                .metadata
                .insert("cron_job_base_url".into(), json!(base_url));
        }
        session
            .state
            .metadata
            .insert("hermes_cron_job_id".into(), json!(hermes.id));
        session
            .state
            .metadata
            .insert("cron_scheduler_delivery".into(), json!(true));
        session
            .state
            .metadata
            .insert("cron_suppress_direct_emission".into(), json!(true));
        if let Some(ack_command) = hermes
            .ack_command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
        {
            if let Err(err) = crate::cron::check_gateway_lifecycle(ack_command) {
                return Err(OmonError::Config(format!(
                    "gateway lifecycle violation in ack_command: {err}"
                )));
            }
        }
        let event = InboundEvent::message(session_key, format!("cron:{}", job.id), prompt);
        self.backend.run(&mut session, event).await?;
        let result = session
            .state
            .metadata
            .get("cron_agent_output")
            .and_then(serde_json::Value::as_str)
            .map(String::from);
        let has_destination = hermes
            .discord_destinations()
            .map(|destinations| !destinations.is_empty())
            .unwrap_or(false);
        let fallback = ["notification", "content"]
            .iter()
            .find_map(|key| hermes.extra.get(*key).and_then(serde_json::Value::as_str));
        ack_unless_scheduler_delivers(
            hermes.ack_command.as_deref(),
            has_destination,
            result.as_deref(),
            fallback,
        )
        .await;
        Ok(CronRunOutcome {
            output: result,
            monitor_state: pending_monitor_state
                .map(|(hash, snapshot)| (hermes.id.clone(), hash, snapshot)),
        })
    }
}

/// Upserts the monitor baseline for a job. Called by the scheduler once the run's
/// delivery has succeeded.
pub(crate) async fn commit_monitor_state(
    pool: &SqlitePool,
    state: &(String, String, String),
) -> Result<()> {
    let (job_id, hash, snapshot) = state;
    sqlx::query(
        "INSERT INTO cron_monitor_states (job_id, last_hash, last_snapshot, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(job_id) DO UPDATE SET last_hash = excluded.last_hash,
                                           last_snapshot = excluded.last_snapshot,
                                           updated_at = excluded.updated_at",
    )
    .bind(job_id)
    .bind(hash)
    .bind(snapshot)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| OmonError::Database(e.to_string()))?;
    Ok(())
}

pub fn is_transient_process_init_error(code: Option<i32>) -> bool {
    #[cfg(windows)]
    {
        // -1073741502 is 0xC0000142 (STATUS_DLL_INIT_FAILED)
        // -1073741819 is 0xC0000005 (STATUS_ACCESS_VIOLATION during loader init)
        matches!(code, Some(-1073741502 | -1073741819))
    }
    #[cfg(not(windows))]
    {
        let _ = code;
        false
    }
}

/// Notepads written through the cron tool default to profile "default" while a Hermes
/// job may derive another profile (bot id / platform). Read both, job profile winning.
pub(crate) async fn collect_notepads(
    pool: &SqlitePool,
    profile: &str,
    job_id: &str,
) -> Result<Vec<(String, String)>> {
    let mut merged = std::collections::BTreeMap::new();
    if profile != "default" {
        for (key, value) in crate::cron::get_cron_notepads(pool, "default", job_id).await? {
            merged.insert(key, value);
        }
    }
    for (key, value) in crate::cron::get_cron_notepads(pool, profile, job_id).await? {
        merged.insert(key, value);
    }
    Ok(merged.into_iter().collect())
}

pub async fn execute_native_cron(
    backend: &Arc<dyn AgentBackend>,
    workspace_root: &Path,
    job: &CronJob,
    payload: &serde_json::Value,
    global_timeout_secs: u64,
) -> Result<Option<String>> {
    let script_output = if let Some(script) =
        payload.get("script").and_then(serde_json::Value::as_str)
    {
        if let Err(err) = crate::cron::check_gateway_lifecycle(script) {
            return Err(OmonError::Config(format!(
                "gateway lifecycle violation in script: {err}"
            )));
        }
        let workspace = canonical_directory(workspace_root, "workspace root")?;
        let augmented_path = augmented_path_from_environment();
        let job_timeout = payload
            .get("timeout_secs")
            .or_else(|| payload.get("timeout_seconds"))
            .or_else(|| payload.get("timeout"))
            .or_else(|| payload.get("script_timeout"))
            .or_else(|| payload.get("script_timeout_seconds"))
            .and_then(serde_json::Value::as_u64);
        let timeout = resolve_cron_script_timeout(job_timeout, global_timeout_secs);

        let mut attempts = 0u64;
        let output = loop {
            attempts += 1;
            let mut command = tokio::process::Command::new("sh");
            command
                .arg("-c")
                .arg(script)
                .current_dir(&workspace)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true);
            if !augmented_path.is_empty() {
                command.env("PATH", &augmented_path);
            }
            #[cfg(unix)]
            {
                command.process_group(0);
            }
            let child = command.spawn().map_err(|error| {
                OmonError::ToolExecution(format!("failed to spawn cron script: {error}"))
            })?;
            let _pid = child.id();
            // Kill the tree while the child is still owned: `timeout` would drop the wait future
            // first, and kill_on_drop then kills the root before its descendants are found.
            let wait = child.wait_with_output();
            tokio::pin!(wait);
            let res = tokio::select! {
                out = &mut wait => Ok(out),
                () = tokio::time::sleep(timeout) => Err(()),
            };
            let out = match res {
                Ok(Ok(out)) => out,
                Ok(Err(err)) => {
                    return Err(OmonError::ToolExecution(format!(
                        "failed to execute cron script: {err}"
                    )));
                }
                Err(_) => {
                    #[cfg(unix)]
                    if let Some(pid) = _pid {
                        kill_process_tree(pid);
                    }
                    return Err(OmonError::ToolExecution(format!(
                        "cron script timed out for {}",
                        job.id
                    )));
                }
            };
            if !out.status.success()
                && is_transient_process_init_error(out.status.code())
                && attempts < 3
            {
                tracing::warn!(
                    job_id = %job.id,
                    code = ?out.status.code(),
                    attempt = attempts,
                    "transient process initialization error detected in cron script, retrying..."
                );
                tokio::time::sleep(std::time::Duration::from_millis(250 * attempts)).await;
                continue;
            }
            break out;
        };
        if !output.status.success() {
            return Err(OmonError::ToolExecution(format!(
                "cron script failed with {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    };
    if let Some(output) = script_output.as_deref() {
        if !parse_wake_gate(output) {
            tracing::info!(
                job_id = %job.id,
                "wakeAgent:false detected in script output, skipping agent execution"
            );
            return Ok(None);
        }
    }
    let prompt = if let Some(brief_path) = payload
        .get("brief_file")
        .and_then(serde_json::Value::as_str)
    {
        match tokio::fs::read_to_string(brief_path).await {
            Ok(content) => content,
            Err(err) => {
                return Err(OmonError::Config(format!(
                    "failed to read brief_file {brief_path} for job {}: {err}",
                    job.id
                )));
            }
        }
    } else {
        payload
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    if prompt.is_empty() {
        return Ok(script_output.filter(|output| !output.trim().is_empty()));
    }
    let mut task = prompt.to_owned();
    if let Some(output) = script_output.filter(|output| !output.trim().is_empty()) {
        task.push_str("\n\n[Script output]\n");
        task.push_str(&output);
    }
    let threats = crate::security::scan_assembled_cron_prompt(&task);
    if !threats.is_empty() {
        return Err(OmonError::Config(format!(
            "assembled cron prompt injection detected: {}",
            threats.join("; ")
        )));
    }
    let session_key = payload_session_key(payload, &job.id);
    let mut session = SessionContext::new(session_key.clone());
    session
        .state
        .metadata
        .insert("cron_scheduler_delivery".into(), json!(true));
    session
        .state
        .metadata
        .insert("cron_suppress_direct_emission".into(), json!(true));
    if let Some(ack_command) = payload
        .get("ack_command")
        .and_then(serde_json::Value::as_str)
        .filter(|command| !command.trim().is_empty())
    {
        if let Err(err) = crate::cron::check_gateway_lifecycle(ack_command) {
            return Err(OmonError::Config(format!(
                "gateway lifecycle violation in ack_command: {err}"
            )));
        }
    }
    let event = InboundEvent::message(session_key, format!("cron:{}", job.id), task);
    backend.run(&mut session, event).await?;
    let result = session
        .state
        .metadata
        .get("cron_agent_output")
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    let has_destination = crate::cron::scheduler::delivery_destination(payload)
        .map(|destinations| !destinations.is_empty())
        .unwrap_or(false);
    let fallback = ["notification", "content"]
        .iter()
        .find_map(|key| payload.get(*key).and_then(serde_json::Value::as_str));
    ack_unless_scheduler_delivers(
        payload
            .get("ack_command")
            .and_then(serde_json::Value::as_str),
        has_destination,
        result.as_deref(),
        fallback,
    )
    .await;
    Ok(result)
}

/// The scheduler runs a job's `ack_command` itself, after delivery succeeded.
/// When it will not deliver (no destination, or the content is empty or a
/// silence token), no delivery can fail, so the ack is run here once the agent
/// run produced a result.
fn payload_session_key(payload: &serde_json::Value, job_id: &str) -> SessionKey {
    let deliver = payload
        .get("deliver")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .find(|destination| !destination.is_empty())
        .unwrap_or_default();
    let (platform, channel, thread) =
        if let Some(slack) = super::store::slack_destination(deliver, None) {
            ("slack", slack.chat_id, slack.thread_id)
        } else if let Some(channel) = deliver.strip_prefix("discord:") {
            ("discord", channel.to_owned(), None)
        } else {
            ("local", job_id.to_owned(), None)
        };
    SessionKey::new(
        platform,
        None::<String>,
        channel,
        thread,
        format!("cron:{job_id}"),
    )
}

async fn ack_unless_scheduler_delivers(
    ack_command: Option<&str>,
    has_destination: bool,
    result: Option<&str>,
    fallback: Option<&str>,
) {
    let Some(ack_command) = ack_command.filter(|command| !command.trim().is_empty()) else {
        return;
    };
    let scheduler_delivers = has_destination
        && result
            .or(fallback)
            .is_some_and(|content| !crate::cron::scheduler::is_cron_silence_response(content));
    if !scheduler_delivers && result.is_some() {
        crate::cron::ack::run_ack_logged(ack_command).await;
    }
}

pub fn load_cron_skills(job: &HermesJob) -> Result<String> {
    let mut names = job.skills.clone();
    if let Some(skill) = job.skill.as_ref().filter(|skill| !names.contains(skill)) {
        names.push(skill.clone());
    }
    if names.is_empty() {
        return Ok(String::new());
    }
    let Ok(home) = hermes_home(job) else {
        if job.prompt.trim().is_empty() && job.brief_file.is_none() {
            return Err(OmonError::Config(format!(
                "Hermes job {} has an empty prompt and all skills are missing: {}",
                job.id,
                names.join(", ")
            )));
        }
        return Ok(format!(
            "⚠️ Skill(s) not found and skipped: {}",
            names.join(", ")
        ));
    };
    let Ok(root) = canonical_directory(&home.join("skills"), "Hermes skills root") else {
        if job.prompt.trim().is_empty() && job.brief_file.is_none() {
            return Err(OmonError::Config(format!(
                "Hermes job {} has an empty prompt and all skills are missing: {}",
                job.id,
                names.join(", ")
            )));
        }
        return Ok(format!(
            "⚠️ Skill(s) not found and skipped: {}",
            names.join(", ")
        ));
    };

    let mut expanded_names = Vec::new();
    for name in names {
        if let Some(members) = resolve_skill_bundle(&root, Some(&home), &name) {
            for member in members {
                if !expanded_names.contains(&member) {
                    expanded_names.push(member);
                }
            }
        } else if !expanded_names.contains(&name) {
            expanded_names.push(name);
        }
    }

    let mut assembled = String::new();
    let mut skipped = Vec::new();
    for name in expanded_names {
        let path = match find_skill_file(&root, &name) {
            Some(p) => p,
            None => {
                warn!(job_id = %job.id, skill = %name, "Cron job skill not found, skipping");
                skipped.push(name);
                continue;
            }
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(error) => {
                warn!(job_id = %job.id, skill = %name, %error, "Failed to read cron skill file, skipping");
                skipped.push(name);
                continue;
            }
        };
        if !assembled.is_empty() {
            assembled.push_str("\n\n");
        }
        assembled.push_str(&format!("[Skill: {name}]\n{content}"));
    }

    if assembled.is_empty()
        && !skipped.is_empty()
        && job.prompt.trim().is_empty()
        && job.brief_file.is_none()
    {
        return Err(OmonError::Config(format!(
            "Hermes job {} has an empty prompt and all skills were missing: {}",
            job.id,
            skipped.join(", ")
        )));
    }

    if !skipped.is_empty() {
        let warning = format!("⚠️ Skill(s) not found and skipped: {}", skipped.join(", "));
        if !assembled.is_empty() {
            Ok(format!("{warning}\n\n{assembled}"))
        } else {
            Ok(warning)
        }
    } else {
        Ok(assembled)
    }
}

pub fn resolve_skill_bundle(
    skills_root: &Path,
    hermes_home: Option<&Path>,
    name: &str,
) -> Option<Vec<String>> {
    let normalized = name.trim_start_matches('/');
    if normalized.is_empty()
        || Path::new(normalized).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }

    fn parse_bundle_manifest(content: &str) -> Option<Vec<String>> {
        #[derive(serde::Deserialize)]
        struct Manifest {
            #[serde(default)]
            skills: Vec<String>,
        }
        if let Ok(m) = serde_yaml::from_str::<Manifest>(content) {
            if !m.skills.is_empty() {
                return Some(m.skills);
            }
        }
        if let Ok(m) = serde_json::from_str::<Manifest>(content) {
            if !m.skills.is_empty() {
                return Some(m.skills);
            }
        }
        None
    }

    if let Some(home) = hermes_home {
        let bundles_dir = home.join("skill-bundles");
        for ext in &["yaml", "yml", "json"] {
            let path = bundles_dir.join(format!("{normalized}.{ext}"));
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(skills) = parse_bundle_manifest(&content) {
                        return Some(skills);
                    }
                }
            }
        }
    }

    for ext in &["yaml", "yml", "json"] {
        let path = skills_root.join(format!("{normalized}.{ext}"));
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(skills) = parse_bundle_manifest(&content) {
                    return Some(skills);
                }
            }
        }
    }

    let dir = skills_root.join(normalized);
    if dir.is_dir() {
        for filename in &[
            "bundle.yaml",
            "bundle.yml",
            "bundle.json",
            "manifest.yaml",
            "manifest.yml",
            "manifest.json",
        ] {
            let path = dir.join(filename);
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(skills) = parse_bundle_manifest(&content) {
                        return Some(skills);
                    }
                }
            }
        }

        if !dir.join("SKILL.md").exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let mut members = Vec::new();
                for entry in entries.flatten() {
                    let sub_path = entry.path();
                    if sub_path.is_dir() && sub_path.join("SKILL.md").is_file() {
                        if let Some(file_name) = sub_path.file_name().and_then(|n| n.to_str()) {
                            members.push(format!("{normalized}/{file_name}"));
                        }
                    }
                }
                if !members.is_empty() {
                    members.sort();
                    return Some(members);
                }
            }
        }
    }

    None
}

pub fn find_skill_file(root: &Path, name: &str) -> Option<PathBuf> {
    let normalized = name.trim_start_matches('/');
    if normalized.is_empty()
        || Path::new(normalized).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    let direct = root.join(normalized).join("SKILL.md");
    if let Ok(candidate) = std::fs::canonicalize(&direct) {
        if candidate.starts_with(root) && candidate.is_file() {
            return Some(candidate);
        }
    }
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).ok()?.flatten() {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                if path.file_name().is_some_and(|value| value == normalized) {
                    let candidate = path.join("SKILL.md");
                    if let Ok(candidate) = std::fs::canonicalize(candidate) {
                        if candidate.starts_with(root) && candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
                pending.push(path);
            }
        }
    }
    None
}

pub async fn run_cron_script(
    job: &HermesJob,
    script: &str,
    workspace_root: &Path,
    global_timeout_secs: u64,
) -> Result<String> {
    let home = hermes_home(job)?;
    let scripts_root = canonical_directory(&home.join("scripts"), "Hermes scripts root")?;
    let candidate = Path::new(script);
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(OmonError::Config(format!(
            "Hermes job {} script path escapes its scripts root: {script}",
            job.id
        )));
    }
    let path = std::fs::canonicalize(scripts_root.join(candidate)).map_err(|error| {
        OmonError::Config(format!(
            "failed to resolve Hermes script for {}: {error}",
            job.id
        ))
    })?;
    if !path.starts_with(&scripts_root) || !path.is_file() {
        return Err(OmonError::Config(format!(
            "Hermes job {} script escapes its scripts root: {}",
            job.id,
            path.display()
        )));
    }

    let script_content = std::fs::read_to_string(&path).map_err(|error| {
        OmonError::ToolExecution(format!("failed to read script {}: {error}", path.display()))
    })?;
    if let Err(err) = crate::cron::check_gateway_lifecycle(&script_content) {
        return Err(OmonError::Config(format!(
            "gateway lifecycle violation in {}: {err}",
            path.display()
        )));
    }

    let roots = authorized_cron_roots(job, workspace_root)?;
    let workdir = match job.workdir.as_ref() {
        Some(workdir) => canonical_authorized_directory(workdir, &roots, "Hermes workdir")?,
        None => home,
    };
    let augmented_path = augmented_path_from_environment();
    let timeout = resolve_cron_script_timeout(job.timeout_secs, global_timeout_secs);

    let mut attempts = 0u64;
    let output = loop {
        attempts += 1;
        let mut command = if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("sh" | "bash")
        ) {
            #[cfg(windows)]
            let command = {
                let bash_bin =
                    if std::path::Path::new(r"C:\Program Files\Git\bin\bash.exe").exists() {
                        r"C:\Program Files\Git\bin\bash.exe"
                    } else {
                        "bash"
                    };
                let mut c = tokio::process::Command::new(bash_bin);
                let path_str = path.to_string_lossy();
                let clean_path = path_str
                    .strip_prefix(r"\\?\")
                    .unwrap_or(&path_str)
                    .replace('\\', "/");
                c.arg(&clean_path);
                c
            };
            #[cfg(not(windows))]
            let command = {
                let mut command = tokio::process::Command::new("bash");
                command.arg(&path);
                command
            };
            command
        } else {
            let mut command = tokio::process::Command::new("python3");
            command.arg(&path);
            command
        };
        command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if !augmented_path.is_empty() {
            command.env("PATH", &augmented_path);
        }
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        let child = command
            .current_dir(&workdir)
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| {
                OmonError::ToolExecution(format!("failed to spawn {}: {error}", path.display()))
            })?;
        let _pid = child.id();
        // See execute_native_cron: the tree must be killed before the child is dropped.
        let wait = child.wait_with_output();
        tokio::pin!(wait);
        let res = tokio::select! {
            out = &mut wait => Ok(out),
            () = tokio::time::sleep(timeout) => Err(()),
        };
        let out = match res {
            Ok(Ok(out)) => out,
            Ok(Err(err)) => {
                return Err(OmonError::ToolExecution(format!(
                    "failed to execute {}: {err}",
                    path.display()
                )));
            }
            Err(_) => {
                #[cfg(unix)]
                if let Some(pid) = _pid {
                    kill_process_tree(pid);
                }
                return Err(OmonError::ToolExecution(format!(
                    "Hermes cron script timed out: {}",
                    path.display()
                )));
            }
        };
        if !out.status.success()
            && is_transient_process_init_error(out.status.code())
            && attempts < 3
        {
            tracing::warn!(
                script = %path.display(),
                code = ?out.status.code(),
                attempt = attempts,
                "transient process initialization error detected in Hermes script, retrying..."
            );
            tokio::time::sleep(std::time::Duration::from_millis(250 * attempts)).await;
            continue;
        }
        break out;
    };

    if !output.status.success() {
        return Err(OmonError::ToolExecution(format!(
            "Hermes cron script {} failed with {:?}: {}",
            path.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn hermes_home(job: &HermesJob) -> Result<PathBuf> {
    let home = job
        .extra
        .get("_omon_hermes_home")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| OmonError::Config(format!("Hermes job {} is missing its home", job.id)))?;
    canonical_directory(&home, "Hermes home")
}

pub fn authorized_cron_roots(job: &HermesJob, workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let workspace_root = canonical_directory(workspace_root, "workspace root")?;
    let home = hermes_home(job)?;
    if home == workspace_root {
        Ok(vec![workspace_root])
    } else {
        Ok(vec![workspace_root, home])
    }
}

pub fn canonical_authorized_directory(
    path: &Path,
    roots: &[PathBuf],
    kind: &str,
) -> Result<PathBuf> {
    let path = canonical_directory(path, kind)?;
    if roots.iter().any(|root| path.starts_with(root)) {
        Ok(path)
    } else {
        Err(OmonError::Config(format!(
            "{kind} is outside authorized workspace/Hermes roots: {}",
            path.display()
        )))
    }
}

pub fn canonical_directory(path: &Path, kind: &str) -> Result<PathBuf> {
    let path = std::fs::canonicalize(path)
        .map_err(|error| OmonError::Config(format!("failed to resolve {kind}: {error}")))?;
    if !path.is_dir() {
        return Err(OmonError::Config(format!(
            "{kind} is not a directory: {}",
            path.display()
        )));
    }
    Ok(path)
}

pub fn resolve_workspace_instructions(workdir: &Path) -> Option<String> {
    const MAX_WORKSPACE_INSTRUCTION_CHARS: usize = 8000;
    for filename in &["AGENTS.md", "agents.md", "CLAUDE.md", "claude.md"] {
        let candidate = workdir.join(filename);
        if candidate.is_file() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    let truncated: String =
                        if trimmed.chars().count() > MAX_WORKSPACE_INSTRUCTION_CHARS {
                            trimmed
                                .chars()
                                .take(MAX_WORKSPACE_INSTRUCTION_CHARS)
                                .collect()
                        } else {
                            trimmed.to_string()
                        };
                    return Some(format!("[Workspace instructions]\n{truncated}"));
                }
            }
        }
    }
    None
}

/// One process captured in a `ps` snapshot: its pid, the process group it belonged to,
/// and its start time, which identifies the process instance across PID reuse.
#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessEntry {
    pid: i32,
    pgid: i32,
    start: String,
}

/// The process tree under one root, captured in a single `ps` snapshot.
#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeSnapshot {
    root: ProcessEntry,
    descendants: Vec<ProcessEntry>,
}

/// The signal a validated snapshot entry licenses, in the order they are sent.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalTarget {
    Group(i32),
    Process(i32),
}

/// A signal candidate derived from the snapshot, with the evidence needed to authorize it
/// at the moment it is dispatched.
#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedSignal {
    /// What to signal.
    target: SignalTarget,
    /// The pid whose identity licenses it: the observed group leader for `Group`, the
    /// process itself for `Process`.
    witness: i32,
    /// Start token of `witness` in the snapshot; a different token means a reused pid.
    witness_start: String,
}

/// Collapses whitespace so a start-time token reads the same every time `ps` prints it
/// (macOS pads single-digit days with a second space).
#[cfg(unix)]
fn normalize_start_token(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parses `ps -A -o pid=,ppid=,pgid=,lstart=` output into `(pid, ppid, pgid, start)` rows.
///
/// The start time is whatever follows the three numeric columns, normalized, so its
/// internal spaces are preserved.
#[cfg(unix)]
fn parse_process_table(text: &str) -> Vec<(i32, i32, i32, String)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(pid), Some(ppid), Some(pgid)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(pid), Ok(ppid), Ok(pgid)) =
            (pid.parse::<i32>(), ppid.parse::<i32>(), pgid.parse::<i32>())
        else {
            continue;
        };
        let start = normalize_start_token(&parts.collect::<Vec<_>>().join(" "));
        if start.is_empty() {
            continue;
        }
        rows.push((pid, ppid, pgid, start));
    }
    rows
}

/// Collects the process tree under `root` from a single
/// `ps -A -o pid=,ppid=,pgid=,lstart=` snapshot.
///
/// Returns `None` when the snapshot cannot be read or does not contain `root`: without a
/// snapshot there is no identity to validate a signal against, so callers must fail closed.
/// Each entry carries the process group and start time observed in the same snapshot, which
/// is what a later kill is checked against.
#[cfg(unix)]
fn collect_process_tree(root: i32) -> Option<TreeSnapshot> {
    if root <= 1 {
        return None;
    }

    // Try `ps` in PATH first, falling back to `/bin/ps` if standard PATH lookup fails.
    let output = std::process::Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,pgid=,lstart="])
        .output()
        .or_else(|_| {
            std::process::Command::new("/bin/ps")
                .args(["-A", "-o", "pid=,ppid=,pgid=,lstart="])
                .output()
        })
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rows = parse_process_table(&stdout);
    let mut parent_to_children: std::collections::HashMap<i32, Vec<i32>> =
        std::collections::HashMap::new();
    let mut entries: std::collections::HashMap<i32, ProcessEntry> =
        std::collections::HashMap::new();

    for (pid, ppid, pgid, start) in rows {
        parent_to_children.entry(ppid).or_default().push(pid);
        entries.insert(pid, ProcessEntry { pid, pgid, start });
    }

    let root_entry = entries.get(&root)?.clone();
    let mut descendants = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();

    queue.push_back(root);
    visited.insert(root);

    while let Some(parent) = queue.pop_front() {
        if let Some(children) = parent_to_children.get(&parent) {
            for &child in children {
                if visited.insert(child) {
                    if let Some(entry) = entries.get(&child) {
                        descendants.push(entry.clone());
                    }
                    queue.push_back(child);
                }
            }
        }
    }

    Some(TreeSnapshot {
        root: root_entry,
        descendants,
    })
}

/// Reads the current start-time token for `pid`, or `None` when the process is gone or
/// cannot be inspected.
#[cfg(unix)]
fn process_start_token(pid: i32) -> Option<String> {
    if pid <= 1 {
        return None;
    }
    let pid_arg = pid.to_string();
    let output = std::process::Command::new("ps")
        .args(["-p", &pid_arg, "-o", "lstart="])
        .output()
        .or_else(|_| {
            std::process::Command::new("/bin/ps")
                .args(["-p", &pid_arg, "-o", "lstart="])
                .output()
        })
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let token = normalize_start_token(&String::from_utf8_lossy(&output.stdout));
    (!token.is_empty()).then_some(token)
}

/// The process group a snapshot entry licenses a group kill for, or `None`.
///
/// The init group and any unknown group (`<= 1`) are never signalled, and neither is the
/// gateway's own group (`own_pgid`) nor a group already covered by another entry
/// (`skip_pgid`).
#[cfg(unix)]
fn signalable_group(pgid: i32, own_pgid: i32, skip_pgid: Option<i32>) -> Option<i32> {
    (pgid > 1 && pgid != own_pgid && skip_pgid != Some(pgid)).then_some(pgid)
}

/// Builds the signal candidates a snapshot licenses, in dispatch order: descendants
/// leaves-first, then the root.
///
/// This only applies the static policy — never the init/unknown groups (`<= 1`), never the
/// gateway's own group (`own_pgid`), and the root's group at most once — and does not read
/// any live process state. Whether a candidate is still safe to send is decided per signal
/// by [`authorize`] immediately before it is sent.
#[cfg(unix)]
fn signal_plan(snapshot: Option<&TreeSnapshot>, own_pgid: i32) -> Vec<PlannedSignal> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };
    let mut plan = Vec::new();
    for entry in snapshot.descendants.iter().rev() {
        if let Some(pgid) = signalable_group(entry.pgid, own_pgid, Some(snapshot.root.pgid)) {
            plan.push(PlannedSignal {
                target: SignalTarget::Group(pgid),
                witness: entry.pid,
                witness_start: entry.start.clone(),
            });
        }
        plan.push(PlannedSignal {
            target: SignalTarget::Process(entry.pid),
            witness: entry.pid,
            witness_start: entry.start.clone(),
        });
    }
    if let Some(pgid) = signalable_group(snapshot.root.pgid, own_pgid, None) {
        plan.push(PlannedSignal {
            target: SignalTarget::Group(pgid),
            witness: snapshot.root.pid,
            witness_start: snapshot.root.start.clone(),
        });
    }
    plan.push(PlannedSignal {
        target: SignalTarget::Process(snapshot.root.pid),
        witness: snapshot.root.pid,
        witness_start: snapshot.root.start.clone(),
    });
    plan
}

/// Authorizes one candidate against live process state, immediately before it is sent.
///
/// A signal is sent only when the witness pid still carries the snapshot's start token. A
/// group signal additionally requires that the witness is *still* in the group recorded in
/// the snapshot (`group_of`), because a process can change groups without its start time
/// changing — the snapshot's group id alone is not proof the group is still ours. The
/// init/unknown groups and the gateway's own group are refused here as well, since the
/// caller's group could not have been known when the snapshot was taken.
#[cfg(unix)]
fn authorize(
    planned: &PlannedSignal,
    own_pgid: i32,
    start_of: &dyn Fn(i32) -> Option<String>,
    group_of: &dyn Fn(i32) -> Option<i32>,
) -> Option<SignalTarget> {
    let start = start_of(planned.witness)?;
    if start != planned.witness_start {
        return None;
    }
    match planned.target {
        SignalTarget::Group(pgid) => {
            // A process can call setpgid without its start token changing, so the snapshot's
            // group id is only trustworthy while the witness still reports it.
            if group_of(planned.witness)? != pgid {
                return None;
            }
            signalable_group(pgid, own_pgid, None).map(SignalTarget::Group)
        }
        SignalTarget::Process(pid) => (pid > 1).then_some(SignalTarget::Process(pid)),
    }
}

/// Walks the snapshot's candidates and hands each one to `send` only after [`authorize`]
/// revalidated it, so every signal is checked against live state at dispatch time rather
/// than once up front.
#[cfg(unix)]
fn dispatch_signals(
    snapshot: &TreeSnapshot,
    own_pgid: i32,
    start_of: &dyn Fn(i32) -> Option<String>,
    group_of: &dyn Fn(i32) -> Option<i32>,
    mut send: impl FnMut(SignalTarget),
) {
    for planned in signal_plan(Some(snapshot), own_pgid) {
        if let Some(target) = authorize(&planned, own_pgid, start_of, group_of) {
            send(target);
        }
    }
}

/// The current process group of `pid`, or `None` when it is gone or unreadable.
#[cfg(unix)]
fn current_process_group(pid: i32) -> Option<i32> {
    if pid <= 1 {
        return None;
    }
    // SAFETY: libc::getpgid takes a process ID by value and reports failure with -1.
    let pgid = unsafe { libc::getpgid(pid) };
    (pgid > 1).then_some(pgid)
}

/// Kills the entire process tree rooted at `root_pid`, including descendants that moved
/// into their own process groups (incident 2026-10-16).
///
/// Incident context: VPN-Mall script (bash) used `set -m` so its `omo` child ran in a
/// separate process group; the gateway's previous group-only SIGKILL (`kill(-pid, SIGKILL)`)
/// killed bash, but `omo` survived 22 more minutes in its own process group, leaving orphaned
/// state.
///
/// Root fix: Snapshot the parent->child tree first via
/// `ps -A -o pid=,ppid=,pgid=,lstart=`, then signal each descendant and its process group
/// BEFORE the root, so descendants are not reparented to init/launchd before their PIDs are
/// discovered.
///
/// Every signal is revalidated immediately before it is sent: the witness pid must still
/// carry the snapshot's start time, and a group signal additionally requires the witness to
/// still be in the snapshot's group (checked with `getpgid`), since a process can change
/// groups without its start time changing. The gateway's own process group and the
/// init/unknown groups are never signalled, and when no snapshot can be read nothing is
/// signalled at all.
///
/// This is race *protection*, not a guarantee: the window is narrowed to the gap between
/// each revalidation and its `kill`, which cannot be made atomic here, and processes the
/// tree spawns after the snapshot are not covered.
#[cfg(unix)]
pub fn kill_process_tree(root_pid: u32) {
    if root_pid <= 1 {
        return;
    }
    let root = root_pid as i32;
    // SAFETY: libc::getpgrp takes no arguments and returns the caller's process group.
    let own_pgid = unsafe { libc::getpgrp() };
    let Some(snapshot) = collect_process_tree(root) else {
        tracing::warn!(
            root_pid,
            "no process tree snapshot; refusing to signal any process for this root"
        );
        return;
    };

    // Each target is authorized against live state right before it is sent.
    dispatch_signals(
        &snapshot,
        own_pgid,
        &process_start_token,
        &current_process_group,
        |target| match target {
            // SAFETY: libc::kill with negative pid signals the process group whose pgid
            // equals `pgid`; `authorize` just confirmed the witness is still in that group.
            SignalTarget::Group(pgid) => unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            },
            // SAFETY: libc::kill accepts a process ID and signal number by value;
            // `authorize` just confirmed the pid still carries the snapshot's start time.
            SignalTarget::Process(pid) => unsafe {
                libc::kill(pid, libc::SIGKILL);
            },
        },
    );
}

#[cfg(not(unix))]
pub fn kill_process_tree(_root_pid: u32) {
    // Windows process cleanup is handled via Job Objects or kill_on_drop.
}

#[cfg(test)]
#[cfg(unix)]
mod process_tree_tests {
    use super::{
        dispatch_signals, parse_process_table, signal_plan, ProcessEntry, SignalTarget,
        TreeSnapshot,
    };
    use std::collections::HashMap;

    const ROOT: i32 = 100;
    const ROOT_START: &str = "Tue Oct 20 15:35:47 2026";
    const OWN_PGID: i32 = 7;

    fn entry(pid: i32, pgid: i32, start: &str) -> ProcessEntry {
        ProcessEntry {
            pid,
            pgid,
            start: start.to_owned(),
        }
    }

    fn snapshot(root_pgid: i32, descendants: Vec<ProcessEntry>) -> TreeSnapshot {
        TreeSnapshot {
            root: entry(ROOT, root_pgid, ROOT_START),
            descendants,
        }
    }

    /// A `start_of` lookup over `(pid, start)` pairs; pids absent from the map are gone.
    fn starts(pairs: &[(i32, &str)]) -> impl Fn(i32) -> Option<String> {
        let map: HashMap<i32, String> = pairs
            .iter()
            .map(|(pid, start)| (*pid, (*start).to_owned()))
            .collect();
        move |pid| map.get(&pid).cloned()
    }

    /// A `group_of` lookup over `(pid, pgid)` pairs; pids absent from the map are gone.
    fn groups(pairs: &[(i32, i32)]) -> impl Fn(i32) -> Option<i32> {
        let map: HashMap<i32, i32> = pairs.iter().copied().collect();
        move |pid| map.get(&pid).copied()
    }

    fn targets(plan: &[super::PlannedSignal]) -> Vec<SignalTarget> {
        plan.iter().map(|planned| planned.target).collect()
    }

    // --- pure plan shape (no live process state) ---

    #[test]
    fn missing_snapshot_yields_no_candidates() {
        assert!(signal_plan(None, OWN_PGID).is_empty());
    }

    #[test]
    fn own_group_and_init_are_never_group_candidates() {
        // Root and descendant both live in the gateway's own group: a group kill would hit
        // the gateway itself, so only the individual processes are planned.
        let snap = snapshot(OWN_PGID, vec![entry(200, OWN_PGID, "start-b")]);
        assert_eq!(
            targets(&signal_plan(Some(&snap), OWN_PGID)),
            vec![SignalTarget::Process(200), SignalTarget::Process(ROOT)]
        );

        // The init group is never planned either.
        let init_group = snapshot(1, Vec::new());
        assert_eq!(
            targets(&signal_plan(Some(&init_group), OWN_PGID)),
            vec![SignalTarget::Process(ROOT)]
        );
    }

    #[test]
    fn candidates_are_leaves_first_with_the_root_group_once() {
        let snap = snapshot(
            ROOT,
            vec![entry(200, ROOT, "start-b"), entry(300, 300, "start-c")],
        );
        assert_eq!(
            targets(&signal_plan(Some(&snap), OWN_PGID)),
            vec![
                SignalTarget::Group(300),
                SignalTarget::Process(300),
                // 200 sits in the root's group, so only its process is planned here.
                SignalTarget::Process(200),
                SignalTarget::Group(ROOT),
                SignalTarget::Process(ROOT),
            ]
        );
    }

    // --- dispatch-time enforcement (this is what kill_process_tree runs) ---

    fn dispatched(
        snap: &TreeSnapshot,
        start_of: &dyn Fn(i32) -> Option<String>,
        group_of: &dyn Fn(i32) -> Option<i32>,
    ) -> Vec<SignalTarget> {
        let mut sent = Vec::new();
        dispatch_signals(snap, OWN_PGID, start_of, group_of, |target| {
            sent.push(target)
        });
        sent
    }

    #[test]
    fn reused_descendant_pid_is_skipped_even_when_it_leads_the_same_group_id() {
        // The reused PID is a new group leader (pgid == pid), so a group comparison alone
        // would accept it; the start token no longer matches.
        let snap = snapshot(ROOT, vec![entry(4242, 4242, "Mon Jan 1 00:00:01 2024")]);
        let sent = dispatched(
            &snap,
            &starts(&[(ROOT, ROOT_START), (4242, "Wed Oct 21 09:00:00 2026")]),
            &groups(&[(ROOT, ROOT), (4242, 4242)]),
        );
        assert_eq!(
            sent,
            vec![SignalTarget::Group(ROOT), SignalTarget::Process(ROOT)]
        );
    }

    #[test]
    fn group_changed_after_the_snapshot_blocks_only_the_group_signal() {
        // Same process (start token matches) that has since left its snapshot group: the
        // group kill would hit whoever owns that group id now, so only the process is hit.
        let snap = snapshot(ROOT, vec![entry(200, 200, "start-b")]);
        let sent = dispatched(
            &snap,
            &starts(&[(ROOT, ROOT_START), (200, "start-b")]),
            &groups(&[(ROOT, ROOT), (200, 900)]),
        );
        assert_eq!(
            sent,
            vec![
                SignalTarget::Process(200),
                SignalTarget::Group(ROOT),
                SignalTarget::Process(ROOT),
            ],
            "a stale snapshot pgid must not be signalled as a group"
        );
    }

    #[test]
    fn reused_root_pid_is_not_signalled() {
        let snap = snapshot(ROOT, Vec::new());
        let sent = dispatched(
            &snap,
            &starts(&[(ROOT, "Wed Oct 21 09:00:00 2026")]),
            &groups(&[(ROOT, ROOT)]),
        );
        assert!(
            sent.is_empty(),
            "a reused root must not be killed: {sent:?}"
        );
    }

    #[test]
    fn vanished_processes_are_skipped() {
        let snap = snapshot(ROOT, vec![entry(200, 200, "start-b")]);
        let sent = dispatched(
            &snap,
            &starts(&[(ROOT, ROOT_START)]),
            &groups(&[(ROOT, ROOT)]),
        );
        assert_eq!(
            sent,
            vec![SignalTarget::Group(ROOT), SignalTarget::Process(ROOT)]
        );
    }

    #[test]
    fn the_gateway_group_is_refused_at_dispatch_too() {
        // The snapshot put the root in group 500, but by dispatch time that pid reports the
        // gateway's own group: the group signal must be refused, not just the process one.
        let snap = snapshot(500, Vec::new());
        let sent = dispatched(
            &snap,
            &starts(&[(ROOT, ROOT_START)]),
            &groups(&[(ROOT, OWN_PGID)]),
        );
        assert_eq!(sent, vec![SignalTarget::Process(ROOT)]);
    }

    #[test]
    fn unreadable_group_state_skips_the_group_signal() {
        let snap = snapshot(ROOT, Vec::new());
        let sent = dispatched(&snap, &starts(&[(ROOT, ROOT_START)]), &groups(&[]));
        assert_eq!(sent, vec![SignalTarget::Process(ROOT)]);
    }

    #[test]
    fn process_table_keeps_the_start_time_after_three_numeric_columns() {
        // macOS pads single-digit days, so the token is normalized before comparison.
        let table = "  100     1   100 Tue Oct 20 15:35:47 2026\n  200   100   200 Thu Jan  1 09:00:09 1970\n\ngarbage\n";
        assert_eq!(
            parse_process_table(table),
            vec![
                (100, 1, 100, ROOT_START.to_owned()),
                (200, 100, 200, "Thu Jan 1 09:00:09 1970".to_owned()),
            ]
        );
    }
}

#[cfg(test)]
mod slack_session_tests {
    use super::payload_session_key;

    #[test]
    fn multiple_destinations_use_only_first_session() {
        let key = payload_session_key(
            &serde_json::json!({"deliver": " slack:C1 ,discord:42"}),
            "job",
        );
        assert_eq!(key.platform, "slack");
        assert_eq!(key.channel_id, "C1");
        assert_eq!(key.thread_id, None);
    }

    #[test]
    fn payload_session_platform_follows_the_deliver_prefix() {
        let slack = payload_session_key(
            &serde_json::json!({"deliver": "slack:C0WORK:1700.0001"}),
            "j",
        );
        assert_eq!(slack.platform, "slack");
        assert_eq!(slack.channel_id, "C0WORK");
        assert_eq!(slack.thread_id.as_deref(), Some("1700.0001"));
        assert_eq!(slack.user_id, "cron:j");

        let discord = payload_session_key(&serde_json::json!({"deliver": "discord:42"}), "j");
        assert_eq!(
            (discord.platform.as_str(), discord.channel_id.as_str()),
            ("discord", "42")
        );
        assert_eq!(discord.thread_id, None);

        let local = payload_session_key(&serde_json::json!({"deliver": "local"}), "j");
        assert_eq!(
            (local.platform.as_str(), local.channel_id.as_str()),
            ("local", "j")
        );
    }
    struct RecordingBackend(std::sync::Mutex<Vec<crate::SessionKey>>);

    #[async_trait::async_trait]
    impl crate::agent::AgentBackend for RecordingBackend {
        async fn run(
            &self,
            session: &mut crate::SessionContext,
            _event: crate::InboundEvent,
        ) -> crate::Result<()> {
            self.0.lock().unwrap().push(session.key.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn hermes_agent_session_uses_the_destination_platform() {
        use crate::CronTaskExecutor;
        let database = crate::Database::connect("sqlite::memory:").await.unwrap();
        let temp = tempfile::tempdir().unwrap();
        let backend = std::sync::Arc::new(RecordingBackend(Default::default()));
        let executor = super::AgentCronExecutor {
            backend: backend.clone(),
            workspace_root: temp.path().to_path_buf(),
            pool: database.pool().clone(),
            cron_script_timeout_secs: 30,
        };
        for (deliver, platform, channel) in [
            ("slack:C0WORK", "slack", "C0WORK"),
            ("discord:42", "discord", "42"),
        ] {
            let job = crate::CronJob {
                id: "hermes_job".into(),
                session_key: None,
                expression: "0 9 * * *".into(),
                payload_json: serde_json::json!({
                    "id": "hermes_job", "prompt": "report",
                    "schedule": {"kind": "cron", "expr": "0 9 * * *"},
                    "deliver": deliver
                })
                .to_string(),
                enabled: true,
                next_run_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                authority: "omon_owned".into(),
            };
            executor.execute(&job).await.unwrap();
            let key = backend.0.lock().unwrap().pop().unwrap();
            assert_eq!(
                (key.platform.as_str(), key.channel_id.as_str()),
                (platform, channel)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::collect_notepads;
    use crate::cron::set_cron_notepad;
    use crate::Database;

    #[tokio::test]
    async fn notepads_written_under_default_profile_reach_jobs_with_derived_profile() {
        let database = Database::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations")
            .run(database.pool())
            .await
            .unwrap();
        let pool = database.pool();
        set_cron_notepad(pool, "default", "job", "a", "from-tool")
            .await
            .unwrap();
        set_cron_notepad(pool, "default", "job", "b", "tool-b")
            .await
            .unwrap();
        set_cron_notepad(pool, "bot-1", "job", "b", "profile-b")
            .await
            .unwrap();
        let notes = collect_notepads(pool, "bot-1", "job").await.unwrap();
        assert_eq!(
            notes,
            vec![
                ("a".to_string(), "from-tool".to_string()),
                ("b".to_string(), "profile-b".to_string())
            ]
        );
    }
}

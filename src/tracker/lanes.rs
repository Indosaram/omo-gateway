use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{OmonError, Result};
use crate::inbox::triage::production_judge;
use crate::lanes::cli::{execute, LaneCommand, LaneModel, StartArgs};
use crate::tracker::model::{
    DelegateInput, LaneActionInput, LaneDetail, LaneEventEntry, LaneHistoryEntry, LaneSummary,
    LaneTranscriptEntry,
};
use crate::tracker::seed::now_utc;

fn invalid(message: impl Into<String>) -> OmonError {
    OmonError::Config(message.into())
}

pub fn model_short_name(model: &str) -> String {
    model.rsplit('/').next().unwrap_or(model).to_string()
}

pub fn deterministic_avatar_color(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();
    let r = 40 + (hash[0] % 180);
    let g = 40 + (hash[1] % 180);
    let b = 40 + (hash[2] % 180);
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub async fn ensure_agent_user(pool: &SqlitePool, model: Option<&str>) -> Result<String> {
    let full_model = model.unwrap_or("agent");
    let short_name = model_short_name(full_model);

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_users WHERE kind = 'agent' AND (model = ? OR (model IS NULL AND name = ?)) LIMIT 1",
    )
    .bind(full_model)
    .bind(&short_name)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    let color = deterministic_avatar_color(&short_name);
    let now = now_utc();

    sqlx::query(
        "INSERT INTO tracker_users (id, name, display_name, email, avatar_color, kind, model, is_me, created_at)
         VALUES (?, ?, ?, NULL, ?, 'agent', ?, 0, ?)
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(&id)
    .bind(&short_name)
    .bind(&short_name)
    .bind(&color)
    .bind(full_model)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(id)
}

pub fn map_lane_status_to_type(status: &str) -> (&'static str, &'static str) {
    match status {
        "queued" => ("unstarted", "Queued"),
        "working" => ("started", "Working"),
        "blocked" => ("started", "Blocked"),
        "done_claimed" => ("started", "In Review"),
        "not_done" => ("started", "Changes Requested"),
        "verified_done" => ("completed", "Verified"),
        "closed" => ("completed", "Done"),
        "failed" => ("canceled", "Failed"),
        _ => ("started", "Working"),
    }
}

pub async fn read_brief_content(brief_path: &str) -> String {
    if brief_path.is_empty() {
        return String::new();
    }
    match tokio::fs::read(brief_path).await {
        Ok(bytes) => {
            let limit = bytes.len().min(20 * 1024);
            String::from_utf8_lossy(&bytes[..limit]).into_owned()
        }
        Err(_) => String::new(),
    }
}

static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn sync_inner(pool: &SqlitePool) -> Result<()> {
    let lanes = sqlx::query(
        "SELECT key, title, cwd, brief_path, model, status FROM lanes ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    let agt_team: Option<(String, i64)> =
        sqlx::query_as("SELECT id, issue_seq FROM tracker_teams WHERE key = 'AGT'")
            .fetch_optional(pool)
            .await?;

    let Some((agt_team_id, mut agt_seq)) = agt_team else {
        return Ok(());
    };

    let agt_states = sqlx::query(
        "SELECT id, type, lane_status FROM tracker_states WHERE team_id = ? AND lane_status IS NOT NULL",
    )
    .bind(&agt_team_id)
    .fetch_all(pool)
    .await?;

    let mut state_by_lane_status = std::collections::HashMap::<String, (String, String)>::new();
    for row in agt_states {
        let id: String = row.get("id");
        let state_type: String = row.get("type");
        let lane_status: String = row.get("lane_status");
        state_by_lane_status.insert(lane_status, (id, state_type));
    }

    let me_user_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_users WHERE is_me = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;

    for lane_row in lanes {
        let lane_key: String = lane_row.get("key");
        let lane_title: String = lane_row.get("title");
        let brief_path: String = lane_row.get("brief_path");
        let lane_model: Option<String> = lane_row.get("model");
        let lane_status: String = lane_row.get("status");

        let existing_issue = sqlx::query(
            "SELECT id, team_id, state_id, started_at
             FROM tracker_issues WHERE lane_key = ?",
        )
        .bind(&lane_key)
        .fetch_optional(pool)
        .await?;

        let now = now_utc();

        if let Some(issue) = existing_issue {
            let issue_id: String = issue.get("id");
            let issue_team_id: String = issue.get("team_id");
            let prev_state_id: String = issue.get("state_id");
            let mut started_at: Option<String> = issue.get("started_at");
            let completed_at: Option<String>;
            let canceled_at: Option<String>;

            // A human move the lane accepted stays visible while its close/reopen request is
            // pending; otherwise the issue always converges back to the lane's real status.
            let last_lane_status: Option<String> = sqlx::query_scalar(
                "SELECT to_value FROM tracker_activity WHERE issue_id = ? AND kind = 'lane_status' ORDER BY id DESC LIMIT 1",
            )
            .bind(&issue_id)
            .fetch_optional(pool)
            .await?;
            let status_changed = last_lane_status.as_deref() != Some(lane_status.as_str());
            if !status_changed {
                let pending: i64 = sqlx::query_scalar(
                    "SELECT (SELECT COUNT(*) FROM lane_close_intents WHERE lane_key = ?1 AND state IN ('pending','running'))
                          + (SELECT COUNT(*) FROM lane_owner_replies WHERE lane_key = ?1 AND state = 'pending')",
                )
                .bind(&lane_key)
                .fetch_one(pool)
                .await?;
                if pending > 0 {
                    continue;
                }
            }

            let target_state_id = if issue_team_id == agt_team_id {
                state_by_lane_status
                    .get(&lane_status)
                    .map(|(id, _)| id.clone())
            } else {
                let (mapped_type, _) = map_lane_status_to_type(&lane_status);
                sqlx::query_scalar::<_, String>(
                    "SELECT id FROM tracker_states WHERE team_id = ? AND type = ? ORDER BY position ASC LIMIT 1",
                )
                .bind(&issue_team_id)
                .bind(mapped_type)
                .fetch_optional(pool)
                .await?
            };

            if let Some(new_state_id) = target_state_id {
                let new_state_type: String =
                    sqlx::query_scalar("SELECT type FROM tracker_states WHERE id = ?")
                        .bind(&new_state_id)
                        .fetch_one(pool)
                        .await?;

                if new_state_id != prev_state_id {
                    match new_state_type.as_str() {
                        "started" => {
                            if started_at.is_none() {
                                started_at = Some(now.clone());
                            }
                            completed_at = None;
                            canceled_at = None;
                        }
                        "completed" => {
                            completed_at = Some(now.clone());
                            canceled_at = None;
                        }
                        "canceled" => {
                            canceled_at = Some(now.clone());
                            completed_at = None;
                        }
                        _ => {
                            completed_at = None;
                            canceled_at = None;
                        }
                    }

                    sqlx::query(
                        "UPDATE tracker_issues
                         SET state_id = ?, started_at = ?, completed_at = ?, canceled_at = ?, updated_at = ?
                         WHERE id = ?",
                    )
                    .bind(&new_state_id)
                    .bind(&started_at)
                    .bind(&completed_at)
                    .bind(&canceled_at)
                    .bind(&now)
                    .bind(&issue_id)
                    .execute(pool)
                    .await?;
                }
            }

            if !status_changed {
                continue;
            }

            sqlx::query(
                "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
                 VALUES (?, NULL, 'lane_status', ?, ?, ?)",
            )
            .bind(&issue_id)
            .bind(last_lane_status.as_deref())
            .bind(&lane_status)
            .bind(&now)
            .execute(pool)
            .await?;

            if matches!(
                lane_status.as_str(),
                "blocked" | "done_claimed" | "verified_done" | "failed"
            ) {
                if let Some(ref me_id) = me_user_id {
                    let notif_id = Uuid::new_v4().to_string();
                    let kind = format!("lane_{lane_status}");
                    let body = format!("Lane {lane_key} entered status {lane_status}");
                    sqlx::query(
                        "INSERT INTO tracker_notifications (id, user_id, issue_id, project_id, kind, actor_id, body, created_at)
                         VALUES (?, ?, ?, NULL, ?, NULL, ?, ?)",
                    )
                    .bind(notif_id)
                    .bind(me_id)
                    .bind(&issue_id)
                    .bind(kind)
                    .bind(body)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        } else {
            let agent_user_id = ensure_agent_user(pool, lane_model.as_deref()).await?;
            let brief_text = read_brief_content(&brief_path).await;

            let (target_state_id, state_type) = if let Some((id, t)) =
                state_by_lane_status.get(&lane_status)
            {
                (id.clone(), t.clone())
            } else {
                let default_state: (String, String) = sqlx::query_as(
                    "SELECT id, type FROM tracker_states WHERE team_id = ? ORDER BY position ASC LIMIT 1",
                )
                .bind(&agt_team_id)
                .fetch_one(pool)
                .await?;
                default_state
            };

            agt_seq += 1;
            let identifier = format!("AGT-{agt_seq}");
            let issue_id = Uuid::new_v4().to_string();

            let (started_at, completed_at, canceled_at) = match state_type.as_str() {
                "started" => (Some(now.clone()), None, None),
                "completed" => (None, Some(now.clone()), None),
                "canceled" => (None, None, Some(now.clone())),
                _ => (None, None, None),
            };

            sqlx::query("UPDATE tracker_teams SET issue_seq = ? WHERE id = ?")
                .bind(agt_seq)
                .bind(&agt_team_id)
                .execute(pool)
                .await?;

            sqlx::query(
                "INSERT INTO tracker_issues (
                    id, team_id, number, identifier, title, description, state_id,
                    priority, estimate, assignee_id, creator_id, project_id, milestone_id,
                    cycle_id, parent_id, due_date, sort_order, lane_key, created_at, updated_at,
                    started_at, completed_at, canceled_at, archived_at, snoozed_until
                 ) VALUES (
                    ?, ?, ?, ?, ?, ?, ?,
                    0, NULL, ?, ?, NULL, NULL,
                    NULL, NULL, NULL, 0.0, ?, ?, ?,
                    ?, ?, ?, NULL, NULL
                 )",
            )
            .bind(&issue_id)
            .bind(&agt_team_id)
            .bind(agt_seq)
            .bind(&identifier)
            .bind(&lane_title)
            .bind(&brief_text)
            .bind(&target_state_id)
            .bind(&agent_user_id)
            .bind(&agent_user_id)
            .bind(&lane_key)
            .bind(&now)
            .bind(&now)
            .bind(&started_at)
            .bind(&completed_at)
            .bind(&canceled_at)
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT INTO tracker_subscribers (issue_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
            )
            .bind(&issue_id)
            .bind(&agent_user_id)
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
                 VALUES (?, ?, 'created', NULL, NULL, ?)",
            )
            .bind(&issue_id)
            .bind(&agent_user_id)
            .bind(&now)
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
                 VALUES (?, NULL, 'lane_status', NULL, ?, ?)",
            )
            .bind(&issue_id)
            .bind(&lane_status)
            .bind(&now)
            .execute(pool)
            .await?;

            if matches!(
                lane_status.as_str(),
                "blocked" | "done_claimed" | "verified_done" | "failed"
            ) {
                if let Some(ref me_id) = me_user_id {
                    let notif_id = Uuid::new_v4().to_string();
                    let kind = format!("lane_{lane_status}");
                    let body = format!("Lane {lane_key} entered status {lane_status}");
                    sqlx::query(
                        "INSERT INTO tracker_notifications (id, user_id, issue_id, project_id, kind, actor_id, body, created_at)
                         VALUES (?, ?, ?, NULL, ?, NULL, ?, ?)",
                    )
                    .bind(notif_id)
                    .bind(me_id)
                    .bind(&issue_id)
                    .bind(kind)
                    .bind(body)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

pub async fn sync(pool: &SqlitePool) -> Result<()> {
    let _guard = SYNC_LOCK.lock().await;
    sync_inner(pool).await
}

pub enum LaneTransitionError {
    LaneNotVerified,
    IllegalLaneTransition(String),
    CommandFailed(String),
}

pub async fn handle_lane_state_transition(
    pool: &SqlitePool,
    lane_key: &str,
    target_state_type: &str,
    force_note: Option<&str>,
) -> std::result::Result<(), LaneTransitionError> {
    let lane_status: Option<String> = sqlx::query_scalar("SELECT status FROM lanes WHERE key = ?")
        .bind(lane_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| LaneTransitionError::CommandFailed(e.to_string()))?;

    let Some(current_status) = lane_status else {
        return Err(LaneTransitionError::IllegalLaneTransition(format!(
            "lane {lane_key} not found"
        )));
    };

    let command = match target_state_type {
        "completed" => {
            if current_status == "verified_done" {
                LaneCommand::Close {
                    key: lane_key.to_string(),
                    force: false,
                    note: None,
                }
            } else if let Some(note) = force_note.filter(|n| !n.trim().is_empty()) {
                LaneCommand::Close {
                    key: lane_key.to_string(),
                    force: true,
                    note: Some(format!("forced: {note}")),
                }
            } else {
                return Err(LaneTransitionError::LaneNotVerified);
            }
        }
        "canceled" => {
            if matches!(
                current_status.as_str(),
                "working" | "blocked" | "verified_done"
            ) {
                LaneCommand::Close {
                    key: lane_key.to_string(),
                    force: true,
                    note: Some("forced: canceled from tracker".to_string()),
                }
            } else {
                return Err(LaneTransitionError::IllegalLaneTransition(format!(
                    "lane cannot be canceled from status {current_status}"
                )));
            }
        }
        "started" | "unstarted" | "backlog" => {
            if matches!(current_status.as_str(), "closed" | "failed") {
                LaneCommand::Reopen {
                    key: lane_key.to_string(),
                    text: None,
                }
            } else {
                return Err(LaneTransitionError::IllegalLaneTransition(format!(
                    "lane is already in active status {current_status}"
                )));
            }
        }
        _ => {
            return Err(LaneTransitionError::IllegalLaneTransition(format!(
                "unsupported target state type {target_state_type}"
            )));
        }
    };

    let judge = production_judge(pool.clone());
    let (code, output) = execute(pool, command, &judge)
        .await
        .map_err(|e| LaneTransitionError::CommandFailed(e.to_string()))?;

    if code == 0 || code == 4 {
        sync(pool)
            .await
            .map_err(|e| LaneTransitionError::CommandFailed(e.to_string()))?;
        Ok(())
    } else {
        Err(LaneTransitionError::CommandFailed(output.to_string()))
    }
}

pub async fn delegate_issue(pool: &SqlitePool, issue_id: &str, input: DelegateInput) -> Result<()> {
    let issue_row = sqlx::query(
        "SELECT identifier, title, description, lane_key FROM tracker_issues WHERE id = ?",
    )
    .bind(issue_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| invalid("issue not found"))?;

    let current_lane_key: Option<String> = issue_row.get("lane_key");
    if current_lane_key.is_some() {
        return Err(invalid("issue already delegated to a lane"));
    }

    let identifier: String = issue_row.get("identifier");
    let title: String = issue_row.get("title");
    let description: String = issue_row.get("description");

    let hex_suffix = format!("{:04x}", rand_u16());
    let raw_key = format!("{}-{hex_suffix}", identifier.to_lowercase());
    let lane_key = sanitize_lane_key(&raw_key);

    let brief_dir = std::env::var("OMON_TRACKER_BRIEF_DIR")
        .unwrap_or_else(|_| ".omo/tracker-briefs".to_string());
    let brief_dir_path = PathBuf::from(brief_dir);
    tokio::fs::create_dir_all(&brief_dir_path)
        .await
        .map_err(|e| invalid(e.to_string()))?;

    let brief_path = brief_dir_path.join(format!("{lane_key}.md"));
    let brief_content = format!("# {identifier}: {title}\n\n{description}");
    tokio::fs::write(&brief_path, brief_content)
        .await
        .map_err(|e| invalid(e.to_string()))?;

    let model = match input.model.as_deref() {
        Some("small") => LaneModel::Small,
        Some("large") => LaneModel::Large,
        _ => LaneModel::Auto,
    };

    let thread = input.thread;

    let start_args = StartArgs {
        key: lane_key.clone(),
        title,
        brief: brief_path,
        done: input.done,
        thread,
        cwd: Some(PathBuf::from(input.cwd)),
        lead: None,
        model,
    };

    let judge = production_judge(pool.clone());
    let guard = SYNC_LOCK.lock().await;
    let (code, output) = execute(pool, LaneCommand::Start(start_args), &judge).await?;

    if code == 0 || code == 4 {
        let now = now_utc();
        sqlx::query("UPDATE tracker_issues SET lane_key = ?, updated_at = ? WHERE id = ?")
            .bind(&lane_key)
            .bind(&now)
            .bind(issue_id)
            .execute(pool)
            .await?;

        let me_id: Option<String> =
            sqlx::query_scalar("SELECT id FROM tracker_users WHERE is_me = 1 LIMIT 1")
                .fetch_optional(pool)
                .await?;

        sqlx::query(
            "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
             VALUES (?, ?, 'delegated', NULL, ?, ?)",
        )
        .bind(issue_id)
        .bind(me_id)
        .bind(&lane_key)
        .bind(&now)
        .execute(pool)
        .await?;

        drop(guard);

        sync(pool).await?;
        Ok(())
    } else {
        drop(guard);
        Err(invalid(format!("lane start failed: {output}")))
    }
}

pub async fn execute_lane_action(
    pool: &SqlitePool,
    issue_id: &str,
    input: LaneActionInput,
) -> Result<()> {
    let lane_key: Option<String> =
        sqlx::query_scalar("SELECT lane_key FROM tracker_issues WHERE id = ?")
            .bind(issue_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| invalid("issue not found"))?;

    let Some(lane_key) = lane_key else {
        return Err(invalid("issue is not linked to a lane"));
    };

    let command = match input.action.as_str() {
        "send" => LaneCommand::Send {
            key: lane_key.clone(),
            text: input.text.unwrap_or_default(),
        },
        "object" => LaneCommand::Object {
            key: lane_key.clone(),
            reason: input.text.unwrap_or_default(),
        },
        "close" => {
            let force = input.force.unwrap_or(false);
            let note = if force {
                let text = input.text.unwrap_or_default();
                let text = text.trim();
                Some(if text.starts_with("forced:") {
                    text.to_string()
                } else if text.is_empty() {
                    "forced: closed from tracker".to_string()
                } else {
                    format!("forced: {text}")
                })
            } else {
                input.text.filter(|text| !text.trim().is_empty())
            };
            LaneCommand::Close {
                key: lane_key.clone(),
                force,
                note,
            }
        }
        "reopen" => LaneCommand::Reopen {
            key: lane_key.clone(),
            text: input.text,
        },
        _ => return Err(invalid(format!("unknown lane action: {}", input.action))),
    };

    let judge = production_judge(pool.clone());
    let (code, output) = execute(pool, command, &judge).await?;

    if code == 0 || code == 4 {
        sync(pool).await?;
        Ok(())
    } else {
        Err(invalid(format!("lane action failed: {output}")))
    }
}

fn rand_u16() -> u16 {
    let nanos = Utc::now().timestamp_subsec_nanos();
    ((nanos ^ (nanos >> 16)) & 0xFFFF) as u16
}

fn sanitize_lane_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for ch in key.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
        } else if ch.is_ascii_uppercase() {
            out.push(ch.to_ascii_lowercase());
        }
    }
    if out.is_empty() {
        out.push_str("lane-1");
    }
    if out.len() > 48 {
        out.truncate(48);
    }
    out
}

pub async fn load_lane_summary(pool: &SqlitePool, lane_key: &str) -> Result<Option<LaneSummary>> {
    let row = sqlx::query(
        "SELECT key, status, model, turns_run, last_activity_at, thread_channel_id
         FROM lanes WHERE key = ?",
    )
    .bind(lane_key)
    .fetch_optional(pool)
    .await?;

    let Some(r) = row else {
        return Ok(None);
    };

    let last_activity: chrono::DateTime<Utc> = r.get("last_activity_at");
    let last_activity_at = last_activity.to_rfc3339_opts(SecondsFormat::Millis, true);

    Ok(Some(LaneSummary {
        key: r.get("key"),
        status: r.get("status"),
        model: r.get("model"),
        turns_run: r.get("turns_run"),
        last_activity_at,
        thread_channel_id: r.get("thread_channel_id"),
    }))
}

pub async fn load_lane_detail(pool: &SqlitePool, lane_key: &str) -> Result<Option<LaneDetail>> {
    let lane_row = sqlx::query(
        "SELECT key, title, status, cwd, brief_path, model, done_signals, claim_evidence,
                objection, turns_run, last_turn_outcome, last_report, last_activity_at,
                thread_channel_id, created_at
         FROM lanes WHERE key = ?",
    )
    .bind(lane_key)
    .fetch_optional(pool)
    .await?;

    let Some(lr) = lane_row else {
        return Ok(None);
    };

    let last_activity: chrono::DateTime<Utc> = lr.get("last_activity_at");
    let last_activity_at = last_activity.to_rfc3339_opts(SecondsFormat::Millis, true);

    let created_at: Option<chrono::DateTime<Utc>> = lr.try_get("created_at").ok();
    let created_at_str = created_at.map(|c| c.to_rfc3339_opts(SecondsFormat::Millis, true));

    let lane_json = serde_json::json!({
        "key": lr.get::<String, _>("key"),
        "title": lr.get::<String, _>("title"),
        "status": lr.get::<String, _>("status"),
        "cwd": lr.get::<String, _>("cwd"),
        "brief_path": lr.get::<String, _>("brief_path"),
        "model": lr.get::<Option<String>, _>("model"),
        "done_signals": lr.get::<String, _>("done_signals"),
        "claim_evidence": lr.get::<Option<String>, _>("claim_evidence"),
        "objection": lr.get::<Option<String>, _>("objection"),
        "turns_run": lr.get::<i64, _>("turns_run"),
        "last_turn_outcome": lr.get::<Option<String>, _>("last_turn_outcome"),
        "last_report": lr.get::<Option<String>, _>("last_report"),
        "last_activity_at": last_activity_at,
        "thread_channel_id": lr.get::<Option<String>, _>("thread_channel_id"),
        "created_at": created_at_str,
    });

    let history_rows = sqlx::query(
        "SELECT id, at, from_status, to_status, reason FROM lane_history WHERE lane_key = ? ORDER BY id ASC",
    )
    .bind(lane_key)
    .fetch_all(pool)
    .await?;

    let mut history = Vec::with_capacity(history_rows.len());
    for hr in history_rows {
        let at_dt: chrono::DateTime<Utc> = hr.get("at");
        history.push(LaneHistoryEntry {
            id: hr.get("id"),
            at: at_dt.to_rfc3339_opts(SecondsFormat::Millis, true),
            from_status: hr.get("from_status"),
            to_status: hr.get("to_status"),
            reason: hr.get("reason"),
        });
    }

    let transcript_rows = sqlx::query(
        "SELECT id, at, role, text FROM lane_transcript WHERE lane_key = ? ORDER BY id DESC LIMIT 200",
    )
    .bind(lane_key)
    .fetch_all(pool)
    .await?;

    let mut transcript = Vec::with_capacity(transcript_rows.len());
    for tr in transcript_rows.into_iter().rev() {
        let at_dt: chrono::DateTime<Utc> = tr.get("at");
        transcript.push(LaneTranscriptEntry {
            id: tr.get("id"),
            at: at_dt.to_rfc3339_opts(SecondsFormat::Millis, true),
            role: tr.get("role"),
            text: tr.get("text"),
        });
    }

    let event_rows = sqlx::query(
        "SELECT id, kind, body, created_at FROM lane_events WHERE lane_key = ? ORDER BY id DESC LIMIT 100",
    )
    .bind(lane_key)
    .fetch_all(pool)
    .await?;

    let mut events = Vec::with_capacity(event_rows.len());
    for er in event_rows.into_iter().rev() {
        let created_dt: chrono::DateTime<Utc> = er.get("created_at");
        events.push(LaneEventEntry {
            id: er.get("id"),
            kind: er.get("kind"),
            body: er.get("body"),
            created_at: created_dt.to_rfc3339_opts(SecondsFormat::Millis, true),
        });
    }

    Ok(Some(LaneDetail {
        lane: lane_json,
        history,
        transcript,
        events,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{insert_lane, Database, Lane};
    use crate::tracker::seed::ensure_seeded;

    #[tokio::test]
    async fn concurrent_sync_after_lane_status_change_creates_single_notification() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool();
        ensure_seeded(pool).await.unwrap();

        let mut lane = Lane::new(
            "test-lane-sync-race",
            "Test Lane Sync Race",
            "lead",
            "/tmp",
            "/tmp/brief.md",
            "session-1",
            "file done.txt",
        );
        lane.status = "working".to_string();
        insert_lane(pool, &lane).await.unwrap();

        // Initial sync creates the AGT issue while lane is in "working" status
        sync(pool).await.unwrap();

        let initial_notifs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracker_notifications WHERE kind LIKE 'lane_%'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(initial_notifs, 0);

        // Update lane status to "blocked", which triggers a notification on next sync
        sqlx::query("UPDATE lanes SET status = 'blocked' WHERE key = 'test-lane-sync-race'")
            .execute(pool)
            .await
            .unwrap();

        // Run two sync calls concurrently
        let (res1, res2) = tokio::join!(sync(pool), sync(pool));
        res1.unwrap();
        res2.unwrap();

        // Exactly one new lane_status notification row must be inserted
        let notif_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracker_notifications WHERE kind = 'lane_blocked'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            notif_count, 1,
            "expected exactly one lane_status notification row"
        );

        let total_lane_notifs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracker_notifications WHERE kind LIKE 'lane_%'",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(
            total_lane_notifs, 1,
            "expected exactly one total lane notification row"
        );
    }
}

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use sqlx::{Row, SqlitePool};

use crate::tracker::api::{
    me_id, new_id, non_empty, prepare, valid_priority, ApiResult, TrackerError,
};
use crate::tracker::lanes::{self, LaneTransitionError};
use crate::tracker::model::{
    Activity, BatchIssuePatch, Comment, Issue, IssueCreate, IssueDetail, IssuePatch, Link,
    Reaction, Relation, RelationDetail,
};
use crate::tracker::seed::now_utc;
use crate::tracker::sessions;

const ISSUE_SELECT: &str = "SELECT i.*,
    (SELECT s.session_state FROM tracker_states s WHERE s.id = i.state_id) AS session_state,
    (SELECT COUNT(*) FROM tracker_issues c WHERE c.parent_id = i.id AND c.archived_at IS NULL) AS sub_issue_count,
    (SELECT COUNT(*) FROM tracker_issues c JOIN tracker_states s ON s.id = c.state_id
       WHERE c.parent_id = i.id AND c.archived_at IS NULL AND s.type IN ('completed','canceled')) AS sub_issue_completed,
    (SELECT COUNT(*) FROM tracker_comments m WHERE m.issue_id = i.id) AS comment_count
  FROM tracker_issues i";

fn issue_from_row(row: &sqlx::sqlite::SqliteRow) -> Issue {
    Issue {
        id: row.get("id"),
        team_id: row.get("team_id"),
        number: row.get("number"),
        identifier: row.get("identifier"),
        title: row.get("title"),
        description: row.get("description"),
        state_id: row.get("state_id"),
        priority: row.get("priority"),
        estimate: row.get("estimate"),
        assignee_id: row.get("assignee_id"),
        creator_id: row.get("creator_id"),
        project_id: row.get("project_id"),
        milestone_id: row.get("milestone_id"),
        cycle_id: row.get("cycle_id"),
        parent_id: row.get("parent_id"),
        due_date: row.get("due_date"),
        sort_order: row.get("sort_order"),
        lane_key: row.get("lane_key"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        canceled_at: row.get("canceled_at"),
        archived_at: row.get("archived_at"),
        snoozed_until: row.get("snoozed_until"),
        label_ids: Vec::new(),
        sub_issue_count: row.get("sub_issue_count"),
        sub_issue_completed: row.get("sub_issue_completed"),
        comment_count: row.get("comment_count"),
        lane: None,
        session_ref: row.get("session_ref"),
        session: row
            .get::<Option<String>, _>("session_ref")
            .and_then(|reference| {
                sessions::session_summary(
                    &reference,
                    row.get::<Option<String>, _>("session_state").as_deref(),
                )
            }),
    }
}

async fn hydrate(pool: &SqlitePool, mut issues: Vec<Issue>) -> ApiResult<Vec<Issue>> {
    if issues.is_empty() {
        return Ok(issues);
    }
    let mut labels: HashMap<String, Vec<String>> = HashMap::new();
    for row in sqlx::query("SELECT issue_id, label_id FROM tracker_issue_labels ORDER BY label_id")
        .fetch_all(pool)
        .await?
    {
        labels
            .entry(row.get("issue_id"))
            .or_default()
            .push(row.get("label_id"));
    }
    for issue in &mut issues {
        issue.label_ids = labels.remove(&issue.id).unwrap_or_default();
        if let Some(key) = issue.lane_key.as_deref() {
            issue.lane = lanes::load_lane_summary(pool, key).await?;
        }
    }
    Ok(issues)
}

pub async fn load_issues(pool: &SqlitePool, filter: &str, binds: &[&str]) -> ApiResult<Vec<Issue>> {
    let sql = format!("{ISSUE_SELECT} {filter}");
    let mut query = sqlx::query(&sql);
    for bind in binds {
        query = query.bind(*bind);
    }
    let rows = query.fetch_all(pool).await?;
    hydrate(pool, rows.iter().map(issue_from_row).collect()).await
}

pub async fn load_issue(pool: &SqlitePool, id: &str) -> ApiResult<Issue> {
    load_issues(pool, "WHERE i.id = ?", &[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| TrackerError::not_found("issue"))
}

pub async fn resolve_issue_id(pool: &SqlitePool, id_or_identifier: &str) -> ApiResult<String> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_issues WHERE id = ? OR identifier = ? COLLATE NOCASE",
    )
    .bind(id_or_identifier)
    .bind(id_or_identifier)
    .fetch_optional(pool)
    .await?;
    id.ok_or_else(|| TrackerError::not_found("issue"))
}

pub async fn record_activity(
    pool: &SqlitePool,
    issue_id: &str,
    actor: Option<&str>,
    kind: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(issue_id)
    .bind(actor)
    .bind(kind)
    .bind(from)
    .bind(to)
    .bind(now_utc())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn subscribe_user(pool: &SqlitePool, issue_id: &str, user_id: &str) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO tracker_subscribers (issue_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
    )
    .bind(issue_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn notify(
    pool: &SqlitePool,
    user_id: &str,
    issue_id: &str,
    kind: &str,
    actor: &str,
    body: &str,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO tracker_notifications (id, user_id, issue_id, project_id, kind, actor_id, body, created_at)
         VALUES (?, ?, ?, NULL, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(user_id)
    .bind(issue_id)
    .bind(kind)
    .bind(actor)
    .bind(body)
    .bind(now_utc())
    .execute(pool)
    .await?;
    Ok(())
}

async fn notify_assignee(
    pool: &SqlitePool,
    issue_id: &str,
    assignee: &str,
    actor: &str,
) -> ApiResult<()> {
    let kind: Option<(String, bool)> =
        sqlx::query_as("SELECT kind, is_me FROM tracker_users WHERE id = ?")
            .bind(assignee)
            .fetch_optional(pool)
            .await?;
    if let Some((kind, is_me)) = kind {
        if kind == "human" && (assignee != actor || is_me) {
            let identifier: String =
                sqlx::query_scalar("SELECT identifier FROM tracker_issues WHERE id = ?")
                    .bind(issue_id)
                    .fetch_one(pool)
                    .await?;
            notify(
                pool,
                assignee,
                issue_id,
                "assigned",
                actor,
                &format!("{identifier} was assigned to you"),
            )
            .await?;
        }
    }
    Ok(())
}

async fn state_info(pool: &SqlitePool, state_id: &str) -> ApiResult<(String, String)> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT team_id, type FROM tracker_states WHERE id = ?")
            .bind(state_id)
            .fetch_optional(pool)
            .await?;
    row.ok_or_else(|| TrackerError::bad_request("unknown state_id"))
}

async fn default_state(pool: &SqlitePool, team_id: &str) -> ApiResult<String> {
    let state: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_states WHERE team_id = ?
         ORDER BY CASE type WHEN 'unstarted' THEN 0 WHEN 'backlog' THEN 1 WHEN 'triage' THEN 2 ELSE 3 END, position
         LIMIT 1",
    )
    .bind(team_id)
    .fetch_optional(pool)
    .await?;
    state.ok_or_else(|| TrackerError::bad_request("team has no workflow states"))
}

fn timestamps_for(
    state_type: &str,
    started_at: Option<String>,
    now: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    match state_type {
        "started" => (
            Some(started_at.unwrap_or_else(|| now.to_string())),
            None,
            None,
        ),
        "completed" => (
            Some(started_at.unwrap_or_else(|| now.to_string())),
            Some(now.to_string()),
            None,
        ),
        "canceled" => (started_at, None, Some(now.to_string())),
        _ => (None, None, None),
    }
}

async fn set_labels(pool: &SqlitePool, issue_id: &str, label_ids: &[String]) -> ApiResult<()> {
    sqlx::query("DELETE FROM tracker_issue_labels WHERE issue_id = ?")
        .bind(issue_id)
        .execute(pool)
        .await?;
    for label in label_ids {
        sqlx::query("INSERT INTO tracker_issue_labels (issue_id, label_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .bind(issue_id)
            .bind(label)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    team_id: Option<String>,
    include_archived: Option<String>,
}

pub async fn list(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    prepare(&pool).await?;
    let include_archived = matches!(query.include_archived.as_deref(), Some("1" | "true"));
    let mut clauses = Vec::new();
    let mut binds = Vec::new();
    if !include_archived {
        clauses.push("i.archived_at IS NULL");
    }
    if let Some(team) = query.team_id.as_deref() {
        clauses.push("i.team_id = ?");
        binds.push(team);
    }
    let filter = if clauses.is_empty() {
        "ORDER BY i.sort_order, i.created_at".to_string()
    } else {
        format!(
            "WHERE {} ORDER BY i.sort_order, i.created_at",
            clauses.join(" AND ")
        )
    };
    let items = load_issues(&pool, &filter, &binds).await?;
    Ok(Json(serde_json::json!({ "items": items })))
}

pub async fn create(
    State(pool): State<SqlitePool>,
    Json(input): Json<IssueCreate>,
) -> ApiResult<(StatusCode, Json<Issue>)> {
    let me = me_id(&pool).await?;
    let title = non_empty(&input.title, "title")?;
    let team: Option<(String, bool)> =
        sqlx::query_as("SELECT key, system FROM tracker_teams WHERE id = ?")
            .bind(&input.team_id)
            .fetch_optional(&pool)
            .await?;
    let (team_key, system) = team.ok_or_else(|| TrackerError::bad_request("unknown team_id"))?;
    if system {
        return Err(TrackerError::conflict(
            "issues in the agent team are created by lanes; delegate an issue instead",
            "system_team",
        ));
    }
    let state_id = match input.state_id {
        Some(state) => {
            let (state_team, _) = state_info(&pool, &state).await?;
            if state_team != input.team_id {
                return Err(TrackerError::bad_request("state belongs to another team"));
            }
            state
        }
        None => default_state(&pool, &input.team_id).await?,
    };
    let (_, state_type) = state_info(&pool, &state_id).await?;
    let priority = valid_priority(input.priority.unwrap_or(0))?;
    let now = now_utc();
    let (started_at, completed_at, canceled_at) = timestamps_for(&state_type, None, &now);

    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let number: i64 = sqlx::query_scalar(
        "UPDATE tracker_teams SET issue_seq = issue_seq + 1 WHERE id = ? RETURNING issue_seq",
    )
    .bind(&input.team_id)
    .fetch_one(&mut *tx)
    .await?;
    let sort_order: f64 = sqlx::query_scalar(
        "SELECT COALESCE(MIN(sort_order), 0.0) - 1.0 FROM tracker_issues WHERE team_id = ?",
    )
    .bind(&input.team_id)
    .fetch_one(&mut *tx)
    .await?;
    let id = new_id();
    let identifier = format!("{team_key}-{number}");
    sqlx::query(
        "INSERT INTO tracker_issues (id, team_id, number, identifier, title, description, state_id, priority,
            estimate, assignee_id, creator_id, project_id, milestone_id, cycle_id, parent_id, due_date,
            sort_order, lane_key, created_at, updated_at, started_at, completed_at, canceled_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.team_id)
    .bind(number)
    .bind(&identifier)
    .bind(&title)
    .bind(input.description.unwrap_or_default())
    .bind(&state_id)
    .bind(priority)
    .bind(input.estimate)
    .bind(&input.assignee_id)
    .bind(&me)
    .bind(&input.project_id)
    .bind(&input.milestone_id)
    .bind(&input.cycle_id)
    .bind(&input.parent_id)
    .bind(&input.due_date)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .bind(&started_at)
    .bind(&completed_at)
    .bind(&canceled_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    if let Some(labels) = input.label_ids.as_deref() {
        set_labels(&pool, &id, labels).await?;
    }
    record_activity(&pool, &id, Some(&me), "created", None, None).await?;
    subscribe_user(&pool, &id, &me).await?;
    if let Some(assignee) = input.assignee_id.as_deref() {
        subscribe_user(&pool, &id, assignee).await?;
        if assignee != me {
            notify_assignee(&pool, &id, assignee, &me).await?;
        }
    }
    Ok((StatusCode::CREATED, Json(load_issue(&pool, &id).await?)))
}

fn lane_error(error: LaneTransitionError) -> TrackerError {
    match error {
        LaneTransitionError::LaneNotVerified => TrackerError::conflict(
            "lane must be verified before closing; use force close",
            "lane_not_verified",
        ),
        LaneTransitionError::IllegalLaneTransition(message) => TrackerError::conflict(
            format!("IllegalTransition: {message}"),
            "illegal_lane_transition",
        ),
        LaneTransitionError::CommandFailed(message) => {
            TrackerError::conflict(message, "lane_command_failed")
        }
    }
}

async fn change<T: ToString>(
    pool: &SqlitePool,
    issue_id: &str,
    me: &str,
    column: &'static str,
    kind: &str,
    old: Option<T>,
    new: Option<T>,
) -> ApiResult<bool> {
    let old = old.map(|value| value.to_string());
    let new = new.map(|value| value.to_string());
    if old == new {
        return Ok(false);
    }
    sqlx::query(&format!(
        "UPDATE tracker_issues SET {column} = ? WHERE id = ?"
    ))
    .bind(&new)
    .bind(issue_id)
    .execute(pool)
    .await?;
    record_activity(
        pool,
        issue_id,
        Some(me),
        kind,
        old.as_deref(),
        new.as_deref(),
    )
    .await?;
    Ok(true)
}

pub async fn apply_patch(
    pool: &SqlitePool,
    issue_id: &str,
    me: &str,
    patch: IssuePatch,
) -> ApiResult<Issue> {
    let current = load_issue(pool, issue_id).await?;
    let mut touched = false;

    if current.session_ref.is_some() && (patch.title.is_some() || patch.description.is_some()) {
        return Err(TrackerError::conflict(
            "session issues mirror the live session title and description",
            "session_state_managed",
        ));
    }

    if let Some(team_id) = patch
        .team_id
        .as_deref()
        .filter(|team| *team != current.team_id)
    {
        if current.lane_key.is_some() {
            return Err(TrackerError::conflict(
                "a lane-linked issue cannot move to another team",
                "illegal_lane_transition",
            ));
        }
        if current.session_ref.is_some() {
            return Err(TrackerError::conflict(
                "a session issue cannot move to another team",
                "system_team",
            ));
        }
        let team: Option<(String, bool)> =
            sqlx::query_as("SELECT key, system FROM tracker_teams WHERE id = ?")
                .bind(team_id)
                .fetch_optional(pool)
                .await?;
        let (key, system) = team.ok_or_else(|| TrackerError::bad_request("unknown team_id"))?;
        if system {
            return Err(TrackerError::conflict(
                "cannot move issues into the agent team",
                "system_team",
            ));
        }
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let number: i64 = sqlx::query_scalar(
            "UPDATE tracker_teams SET issue_seq = issue_seq + 1 WHERE id = ? RETURNING issue_seq",
        )
        .bind(team_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let state = match patch.state_id.as_deref() {
            Some(state) => state.to_string(),
            None => default_state(pool, team_id).await?,
        };
        sqlx::query("UPDATE tracker_issues SET team_id = ?, number = ?, identifier = ?, state_id = ?, cycle_id = NULL WHERE id = ?")
            .bind(team_id)
            .bind(number)
            .bind(format!("{key}-{number}"))
            .bind(&state)
            .bind(issue_id)
            .execute(pool)
            .await?;
        record_activity(
            pool,
            issue_id,
            Some(me),
            "team",
            Some(&current.team_id),
            Some(team_id),
        )
        .await?;
        touched = true;
    }

    if let Some(state_id) = patch
        .state_id
        .as_deref()
        .filter(|state| *state != current.state_id)
    {
        let (state_team, state_type) = state_info(pool, state_id).await?;
        let team_now: String =
            sqlx::query_scalar("SELECT team_id FROM tracker_issues WHERE id = ?")
                .bind(issue_id)
                .fetch_one(pool)
                .await?;
        if state_team != team_now {
            return Err(TrackerError::bad_request("state belongs to another team"));
        }
        if let Some(reference) = current.session_ref.as_deref() {
            // Sync owns SES states; the one allowed move (Ended on a streaming session)
            // interrupts it and the re-sync records whatever state the session lands in.
            sessions::manual_state_move(pool, reference, state_id).await?;
        } else {
            if let Some(key) = current.lane_key.as_deref() {
                lanes::handle_lane_state_transition(
                    pool,
                    key,
                    &state_type,
                    patch.force_note.as_deref(),
                )
                .await
                .map_err(lane_error)?;
            }
            let now = now_utc();
            let (started, completed, canceled) =
                timestamps_for(&state_type, current.started_at.clone(), &now);
            sqlx::query("UPDATE tracker_issues SET state_id = ?, started_at = ?, completed_at = ?, canceled_at = ? WHERE id = ?")
                .bind(state_id)
                .bind(&started)
                .bind(&completed)
                .bind(&canceled)
                .bind(issue_id)
                .execute(pool)
                .await?;
            record_activity(
                pool,
                issue_id,
                Some(me),
                "state",
                Some(&current.state_id),
                Some(state_id),
            )
            .await?;
            touched = true;
        }
    }

    if let Some(title) = patch.title.as_deref() {
        let title = non_empty(title, "title")?;
        touched |= change(
            pool,
            issue_id,
            me,
            "title",
            "title",
            Some(current.title.clone()),
            Some(title),
        )
        .await?;
    }
    if let Some(description) = patch.description {
        if description != current.description {
            sqlx::query("UPDATE tracker_issues SET description = ? WHERE id = ?")
                .bind(&description)
                .bind(issue_id)
                .execute(pool)
                .await?;
            record_activity(pool, issue_id, Some(me), "description", None, None).await?;
            touched = true;
        }
    }
    if let Some(priority) = patch.priority {
        let priority = valid_priority(priority)?;
        touched |= change(
            pool,
            issue_id,
            me,
            "priority",
            "priority",
            Some(current.priority),
            Some(priority),
        )
        .await?;
    }
    if let Some(estimate) = patch.estimate {
        touched |= change(
            pool,
            issue_id,
            me,
            "estimate",
            "estimate",
            current.estimate,
            estimate,
        )
        .await?;
    }
    if let Some(assignee) = patch.assignee_id {
        if change(
            pool,
            issue_id,
            me,
            "assignee_id",
            "assignee",
            current.assignee_id.clone(),
            assignee.clone(),
        )
        .await?
        {
            touched = true;
            if let Some(user) = assignee.as_deref() {
                subscribe_user(pool, issue_id, user).await?;
                if user != me {
                    notify_assignee(pool, issue_id, user, me).await?;
                }
            }
        }
    }
    if let Some(project) = patch.project_id {
        if change(
            pool,
            issue_id,
            me,
            "project_id",
            "project",
            current.project_id.clone(),
            project.clone(),
        )
        .await?
        {
            touched = true;
            if patch.milestone_id.is_none() && current.milestone_id.is_some() {
                sqlx::query("UPDATE tracker_issues SET milestone_id = NULL WHERE id = ?")
                    .bind(issue_id)
                    .execute(pool)
                    .await?;
            }
        }
    }
    if let Some(milestone) = patch.milestone_id {
        touched |= change(
            pool,
            issue_id,
            me,
            "milestone_id",
            "milestone",
            current.milestone_id.clone(),
            milestone,
        )
        .await?;
    }
    if let Some(cycle) = patch.cycle_id {
        touched |= change(
            pool,
            issue_id,
            me,
            "cycle_id",
            "cycle",
            current.cycle_id.clone(),
            cycle,
        )
        .await?;
    }
    if let Some(parent) = patch.parent_id {
        if parent.as_deref() == Some(issue_id) {
            return Err(TrackerError::bad_request(
                "an issue cannot be its own parent",
            ));
        }
        touched |= change(
            pool,
            issue_id,
            me,
            "parent_id",
            "parent",
            current.parent_id.clone(),
            parent,
        )
        .await?;
    }
    if let Some(due) = patch.due_date {
        touched |= change(
            pool,
            issue_id,
            me,
            "due_date",
            "due_date",
            current.due_date.clone(),
            due,
        )
        .await?;
    }
    if let Some(sort_order) = patch.sort_order {
        sqlx::query("UPDATE tracker_issues SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(issue_id)
            .execute(pool)
            .await?;
        touched = true;
    }
    if let Some(snoozed) = patch.snoozed_until {
        sqlx::query("UPDATE tracker_issues SET snoozed_until = ? WHERE id = ?")
            .bind(&snoozed)
            .bind(issue_id)
            .execute(pool)
            .await?;
        touched = true;
    }
    if let Some(archived) = patch.archived {
        if archived != current.archived_at.is_some() {
            sqlx::query("UPDATE tracker_issues SET archived_at = ? WHERE id = ?")
                .bind(archived.then(now_utc))
                .bind(issue_id)
                .execute(pool)
                .await?;
            let kind = if archived { "archived" } else { "unarchived" };
            record_activity(pool, issue_id, Some(me), kind, None, None).await?;
            touched = true;
        }
    }

    let mut labels: Vec<String> = patch.label_ids.unwrap_or_else(|| current.label_ids.clone());
    for add in patch.add_label_ids.unwrap_or_default() {
        if !labels.contains(&add) {
            labels.push(add);
        }
    }
    let removed = patch.remove_label_ids.unwrap_or_default();
    labels.retain(|label| !removed.contains(label));
    if labels != current.label_ids {
        for added in labels
            .iter()
            .filter(|label| !current.label_ids.contains(label))
        {
            record_activity(pool, issue_id, Some(me), "label_added", None, Some(added)).await?;
        }
        for gone in current
            .label_ids
            .iter()
            .filter(|label| !labels.contains(label))
        {
            record_activity(pool, issue_id, Some(me), "label_removed", Some(gone), None).await?;
        }
        set_labels(pool, issue_id, &labels).await?;
        touched = true;
    }

    if touched {
        sqlx::query("UPDATE tracker_issues SET updated_at = ? WHERE id = ?")
            .bind(now_utc())
            .bind(issue_id)
            .execute(pool)
            .await?;
    }
    load_issue(pool, issue_id).await
}

pub async fn update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<IssuePatch>,
) -> ApiResult<Json<Issue>> {
    let me = me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    Ok(Json(apply_patch(&pool, &issue_id, &me, patch).await?))
}

pub async fn batch(
    State(pool): State<SqlitePool>,
    Json(input): Json<BatchIssuePatch>,
) -> ApiResult<Json<serde_json::Value>> {
    let me = me_id(&pool).await?;
    let mut items = Vec::with_capacity(input.ids.len());
    for id in &input.ids {
        let issue_id = resolve_issue_id(&pool, id).await?;
        items.push(apply_patch(&pool, &issue_id, &me, input.patch.clone()).await?);
    }
    Ok(Json(serde_json::json!({ "items": items })))
}

pub async fn remove(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let issue_id = resolve_issue_id(&pool, &id).await?;
    let lane: Option<String> =
        sqlx::query_scalar("SELECT lane_key FROM tracker_issues WHERE id = ?")
            .bind(&issue_id)
            .fetch_one(&pool)
            .await?;
    if lane.is_some() {
        return Err(TrackerError::conflict(
            "a lane-linked issue cannot be deleted; close the lane instead",
            "lane_linked",
        ));
    }
    sqlx::query("UPDATE tracker_issues SET parent_id = NULL WHERE parent_id = ?")
        .bind(&issue_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM tracker_notifications WHERE issue_id = ?")
        .bind(&issue_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM tracker_issues WHERE id = ?")
        .bind(&issue_id)
        .execute(&pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_comments(pool: &SqlitePool, filter: &str, bind: &str) -> ApiResult<Vec<Comment>> {
    let rows = sqlx::query(&format!(
        "SELECT id, issue_id, user_id, body, parent_id, created_at, updated_at, edited, resolved_at
         FROM tracker_comments {filter} ORDER BY created_at"
    ))
    .bind(bind)
    .fetch_all(pool)
    .await?;
    let mut comments = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.get("id");
        let mut reactions: Vec<Reaction> = Vec::new();
        for reaction in sqlx::query(
            "SELECT emoji, user_id FROM tracker_reactions WHERE comment_id = ? ORDER BY rowid",
        )
        .bind(&id)
        .fetch_all(pool)
        .await?
        {
            let emoji: String = reaction.get("emoji");
            let user: String = reaction.get("user_id");
            match reactions.iter_mut().find(|r| r.emoji == emoji) {
                Some(existing) => existing.user_ids.push(user),
                None => reactions.push(Reaction {
                    emoji,
                    user_ids: vec![user],
                }),
            }
        }
        comments.push(Comment {
            id,
            issue_id: row.get("issue_id"),
            user_id: row.get("user_id"),
            body: row.get("body"),
            parent_id: row.get("parent_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            edited: row.get("edited"),
            resolved_at: row.get("resolved_at"),
            reactions,
        });
    }
    Ok(comments)
}

pub fn relation_direction(relation: &Relation, from_issue: &str) -> &'static str {
    let forward = relation.issue_id == from_issue;
    match (relation.relation_type.as_str(), forward) {
        ("blocks", true) => "blocks",
        ("blocks", false) => "blocked_by",
        ("duplicate", true) => "duplicate",
        ("duplicate", false) => "duplicated_by",
        _ => "related",
    }
}

pub async fn fetch_issue_detail(pool: &SqlitePool, issue_id: &str) -> ApiResult<IssueDetail> {
    let issue = load_issue(pool, issue_id).await?;
    let children = load_issues(
        pool,
        "WHERE i.parent_id = ? ORDER BY i.sort_order, i.created_at",
        &[issue_id],
    )
    .await?;
    let parent = match issue.parent_id.as_deref() {
        Some(parent) => load_issues(pool, "WHERE i.id = ?", &[parent])
            .await?
            .into_iter()
            .next(),
        None => None,
    };
    let relation_rows: Vec<Relation> = sqlx::query_as(
        "SELECT id, issue_id, related_issue_id, type, created_at FROM tracker_relations
         WHERE issue_id = ? OR related_issue_id = ? ORDER BY created_at",
    )
    .bind(issue_id)
    .bind(issue_id)
    .fetch_all(pool)
    .await?;
    let mut relations = Vec::with_capacity(relation_rows.len());
    for relation in relation_rows {
        let other = if relation.issue_id == issue_id {
            &relation.related_issue_id
        } else {
            &relation.issue_id
        };
        let other_issue = load_issue(pool, other).await?;
        let direction = relation_direction(&relation, issue_id).to_string();
        relations.push(RelationDetail {
            id: relation.id,
            issue_id: relation.issue_id,
            related_issue_id: relation.related_issue_id,
            relation_type: relation.relation_type,
            created_at: relation.created_at,
            issue: other_issue,
            direction,
        });
    }
    let comments = load_comments(pool, "WHERE issue_id = ?", issue_id).await?;
    let activity: Vec<Activity> = sqlx::query_as(
        "SELECT id, issue_id, actor_id, kind, from_value, to_value, created_at FROM tracker_activity
         WHERE issue_id = ? ORDER BY id",
    )
    .bind(issue_id)
    .fetch_all(pool)
    .await?;
    let links: Vec<Link> = sqlx::query_as(
        "SELECT id, issue_id, url, title, created_at FROM tracker_links WHERE issue_id = ? ORDER BY created_at",
    )
    .bind(issue_id)
    .fetch_all(pool)
    .await?;
    let subscriber_ids: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM tracker_subscribers WHERE issue_id = ? ORDER BY user_id",
    )
    .bind(issue_id)
    .fetch_all(pool)
    .await?;
    let lane_detail = match issue.lane_key.as_deref() {
        Some(key) => lanes::load_lane_detail(pool, key).await?,
        None => None,
    };
    let session_detail = issue.session.as_ref().map(sessions::session_detail);
    Ok(IssueDetail {
        issue,
        children,
        parent,
        relations,
        comments,
        activity,
        links,
        subscriber_ids,
        lane_detail,
        session_detail,
    })
}

pub async fn detail(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<Json<IssueDetail>> {
    prepare(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    Ok(Json(fetch_issue_detail(&pool, &issue_id).await?))
}

pub fn row_count_error(rows: u64, what: &str) -> ApiResult<()> {
    if rows == 0 {
        Err(TrackerError::not_found(what))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use crate::tracker::seed::{ensure_seeded, ensure_sessions_team};
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn patch_title_on_session_issue_returns_conflict_and_leaves_db_unchanged() {
        std::env::set_var(sessions::SYNC_ENV, "0");
        let db = Database::connect("sqlite::memory:").await.unwrap();
        ensure_seeded(db.pool()).await.unwrap();
        let ses_team_id = ensure_sessions_team(db.pool()).await.unwrap();

        let (state_id,): (String,) = sqlx::query_as(
            "SELECT id FROM tracker_states WHERE team_id = ? AND session_state = 'working'",
        )
        .bind(&ses_team_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

        let issue_id = new_id();
        let now = now_utc();
        let original_title = "Original Session Title";
        let original_description = "Original Session Description";
        let session_reference = "local:session-test";

        // Insert using the same SQL the sessions sync uses (src/tracker/sessions.rs upsert insert)
        sqlx::query(
            "INSERT INTO tracker_issues (id, team_id, number, identifier, title, description, state_id, priority,
                estimate, assignee_id, creator_id, project_id, milestone_id, cycle_id, parent_id, due_date,
                sort_order, lane_key, session_ref, created_at, updated_at, started_at, completed_at, canceled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?, NULL, NULL, NULL, NULL, NULL, 0.0, NULL, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(&issue_id)
        .bind(&ses_team_id)
        .bind(1i64)
        .bind("SES-1")
        .bind(original_title)
        .bind(original_description)
        .bind(&state_id)
        .bind("me")
        .bind("me")
        .bind(session_reference)
        .bind(&now)
        .bind(&now)
        .bind(Some(&now))
        .bind(Option::<String>::None)
        .execute(db.pool())
        .await
        .unwrap();

        let patch = IssuePatch {
            title: Some("User-edited title".into()),
            ..Default::default()
        };

        let result = apply_patch(db.pool(), &issue_id, "me", patch).await;
        assert!(
            result.is_err(),
            "patching title on a session issue must fail"
        );

        let err = result.unwrap_err();
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "session_state_managed");
        assert_eq!(
            body["error"],
            "session issues mirror the live session title and description"
        );

        let (title_in_db, description_in_db): (String, String) =
            sqlx::query_as("SELECT title, description FROM tracker_issues WHERE id = ?")
                .bind(&issue_id)
                .fetch_one(db.pool())
                .await
                .unwrap();

        assert_eq!(title_in_db, original_title);
        assert_eq!(description_in_db, original_description);
    }
}

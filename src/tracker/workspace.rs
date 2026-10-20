use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::tracker::api::{
    me_id, new_id, next_sort, non_empty, one_of, prepare, require, valid_priority, ApiResult,
    TrackerError,
};
use crate::tracker::issues::{load_issues, row_count_error};
use crate::tracker::lanes::deterministic_avatar_color;
use crate::tracker::model::{
    Bootstrap, Cycle, CycleCreate, CyclePatch, Favorite, FavoriteToggle, Label, LabelCreate,
    LabelPatch, Milestone, MilestoneCreate, MilestonePatch, Notification, NotificationPatch,
    Project, ProjectDetail, ProjectInput, ProjectPatch, ProjectUpdate, ProjectUpdateCreate,
    ProjectUpdatePatch, StateCreate, StatePatch, Team, TeamCreate, TeamPatch, User, UserCreate,
    UserPatch, View, ViewCreate, ViewPatch, WorkflowState,
};
use crate::tracker::seed::{now_utc, seed_team_states};

const PROJECT_STATUSES: &[&str] = &[
    "backlog",
    "planned",
    "started",
    "paused",
    "completed",
    "canceled",
];
const HEALTHS: &[&str] = &["on_track", "at_risk", "off_track"];
const STATE_TYPES: &[&str] = &[
    "triage",
    "backlog",
    "unstarted",
    "started",
    "completed",
    "canceled",
];
const FAVORITE_KINDS: &[&str] = &["issue", "project", "view", "cycle", "team", "label"];
const PALETTE: &[&str] = &[
    "#5e6ad2", "#26b5ce", "#4cb782", "#f2c94c", "#f2994a", "#eb5757", "#bb87fc",
];

fn created<T>(value: T) -> (StatusCode, Json<T>) {
    (StatusCode::CREATED, Json(value))
}

async fn load_users(pool: &SqlitePool) -> ApiResult<Vec<User>> {
    Ok(sqlx::query_as(
        "SELECT id, name, display_name, email, avatar_color, kind, model, is_me, created_at
         FROM tracker_users ORDER BY is_me DESC, kind, name",
    )
    .fetch_all(pool)
    .await?)
}

async fn load_user(pool: &SqlitePool, id: &str) -> ApiResult<User> {
    sqlx::query_as(
        "SELECT id, name, display_name, email, avatar_color, kind, model, is_me, created_at
         FROM tracker_users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("user"))
}

const TEAM_COLUMNS: &str = "id, key, name, icon, color, issue_seq, system, cycles_enabled, cycle_weeks, triage_enabled, sort_order, created_at";

async fn load_team(pool: &SqlitePool, id: &str) -> ApiResult<Team> {
    sqlx::query_as(&format!(
        "SELECT {TEAM_COLUMNS} FROM tracker_teams WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("team"))
}

const STATE_COLUMNS: &str =
    "id, team_id, name, type, color, position, lane_status, description, session_state";

async fn load_state(pool: &SqlitePool, id: &str) -> ApiResult<WorkflowState> {
    sqlx::query_as(&format!(
        "SELECT {STATE_COLUMNS} FROM tracker_states WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("state"))
}

const LABEL_COLUMNS: &str = "id, team_id, name, color, parent_id, description, created_at";

async fn load_label(pool: &SqlitePool, id: &str) -> ApiResult<Label> {
    sqlx::query_as(&format!(
        "SELECT {LABEL_COLUMNS} FROM tracker_labels WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("label"))
}

const CYCLE_COLUMNS: &str =
    "id, team_id, number, name, description, starts_at, ends_at, completed_at";

async fn load_cycle(pool: &SqlitePool, id: &str) -> ApiResult<Cycle> {
    sqlx::query_as(&format!(
        "SELECT {CYCLE_COLUMNS} FROM tracker_cycles WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("cycle"))
}

const MILESTONE_COLUMNS: &str = "id, project_id, name, target_date, sort_order";
const UPDATE_COLUMNS: &str = "id, project_id, user_id, health, body, created_at, updated_at";

async fn load_projects(
    pool: &SqlitePool,
    filter: &str,
    bind: Option<&str>,
) -> ApiResult<Vec<Project>> {
    let sql = format!(
        "SELECT p.*,
            (SELECT COUNT(*) FROM tracker_issues i WHERE i.project_id = p.id AND i.archived_at IS NULL) AS issue_count,
            (SELECT COUNT(*) FROM tracker_issues i JOIN tracker_states s ON s.id = i.state_id
               WHERE i.project_id = p.id AND i.archived_at IS NULL AND s.type = 'completed') AS completed_issue_count,
            (SELECT u.health FROM tracker_project_updates u WHERE u.project_id = p.id ORDER BY u.created_at DESC LIMIT 1) AS latest_health
         FROM tracker_projects p {filter} ORDER BY p.sort_order, p.created_at"
    );
    let mut query = sqlx::query(&sql);
    if let Some(bind) = bind {
        query = query.bind(bind);
    }
    let rows = query.fetch_all(pool).await?;
    let mut teams: HashMap<String, Vec<String>> = HashMap::new();
    for row in sqlx::query("SELECT project_id, team_id FROM tracker_project_teams ORDER BY team_id")
        .fetch_all(pool)
        .await?
    {
        teams
            .entry(row.get("project_id"))
            .or_default()
            .push(row.get("team_id"));
    }
    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    for row in
        sqlx::query("SELECT project_id, user_id FROM tracker_project_members ORDER BY user_id")
            .fetch_all(pool)
            .await?
    {
        members
            .entry(row.get("project_id"))
            .or_default()
            .push(row.get("user_id"));
    }
    Ok(rows
        .iter()
        .map(|row| {
            let id: String = row.get("id");
            Project {
                team_ids: teams.remove(&id).unwrap_or_default(),
                member_ids: members.remove(&id).unwrap_or_default(),
                id,
                name: row.get("name"),
                description: row.get("description"),
                summary: row.get("summary"),
                icon: row.get("icon"),
                color: row.get("color"),
                status: row.get("status"),
                priority: row.get("priority"),
                lead_id: row.get("lead_id"),
                start_date: row.get("start_date"),
                target_date: row.get("target_date"),
                sort_order: row.get("sort_order"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                archived_at: row.get("archived_at"),
                issue_count: row.get("issue_count"),
                completed_issue_count: row.get("completed_issue_count"),
                latest_health: row.get("latest_health"),
            }
        })
        .collect())
}

async fn load_project(pool: &SqlitePool, id: &str) -> ApiResult<Project> {
    load_projects(pool, "WHERE p.id = ?", Some(id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| TrackerError::not_found("project"))
}

fn view_from_row(row: &sqlx::sqlite::SqliteRow) -> View {
    let parse = |column: &str| -> Value {
        let raw: String = row.get(column);
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))
    };
    View {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        icon: row.get("icon"),
        color: row.get("color"),
        owner_id: row.get("owner_id"),
        team_id: row.get("team_id"),
        filters: parse("filters"),
        display: parse("display"),
        shared: row.get("shared"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

async fn load_view(pool: &SqlitePool, id: &str) -> ApiResult<View> {
    let row = sqlx::query("SELECT * FROM tracker_views WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| TrackerError::not_found("view"))?;
    Ok(view_from_row(&row))
}

const NOTIFICATION_COLUMNS: &str = "id, user_id, issue_id, project_id, kind, actor_id, body, created_at, read_at, snoozed_until, archived_at";

async fn unread_count(pool: &SqlitePool, me: &str) -> ApiResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM tracker_notifications WHERE user_id = ? AND read_at IS NULL AND archived_at IS NULL
           AND (snoozed_until IS NULL OR snoozed_until <= ?)",
    )
    .bind(me)
    .bind(now_utc())
    .fetch_one(pool)
    .await?)
}

pub async fn bootstrap(State(pool): State<SqlitePool>) -> ApiResult<Json<Bootstrap>> {
    let me = prepare(&pool).await?;
    let users = load_users(&pool).await?;
    let me_user = users
        .iter()
        .find(|user| user.id == me)
        .cloned()
        .ok_or_else(|| TrackerError::not_found("current user"))?;
    let teams: Vec<Team> = sqlx::query_as(&format!(
        "SELECT {TEAM_COLUMNS} FROM tracker_teams ORDER BY system, sort_order, created_at"
    ))
    .fetch_all(&pool)
    .await?;
    let states: Vec<WorkflowState> = sqlx::query_as(&format!(
        "SELECT {STATE_COLUMNS} FROM tracker_states ORDER BY team_id, position"
    ))
    .fetch_all(&pool)
    .await?;
    let labels: Vec<Label> = sqlx::query_as(&format!(
        "SELECT {LABEL_COLUMNS} FROM tracker_labels ORDER BY name COLLATE NOCASE"
    ))
    .fetch_all(&pool)
    .await?;
    let projects = load_projects(&pool, "WHERE p.archived_at IS NULL", None).await?;
    let cycles: Vec<Cycle> = sqlx::query_as(&format!(
        "SELECT {CYCLE_COLUMNS} FROM tracker_cycles ORDER BY team_id, number"
    ))
    .fetch_all(&pool)
    .await?;
    let milestones: Vec<Milestone> = sqlx::query_as(&format!(
        "SELECT {MILESTONE_COLUMNS} FROM tracker_milestones ORDER BY project_id, sort_order"
    ))
    .fetch_all(&pool)
    .await?;
    let views = sqlx::query(
        "SELECT * FROM tracker_views WHERE shared = 1 OR owner_id = ? ORDER BY created_at",
    )
    .bind(&me)
    .fetch_all(&pool)
    .await?
    .iter()
    .map(view_from_row)
    .collect();
    let favorites: Vec<Favorite> = sqlx::query_as(
        "SELECT id, user_id, kind, target_id, sort_order FROM tracker_favorites WHERE user_id = ? ORDER BY sort_order",
    )
    .bind(&me)
    .fetch_all(&pool)
    .await?;
    let unread_count = unread_count(&pool, &me).await?;
    Ok(Json(Bootstrap {
        me: me_user,
        users,
        teams,
        states,
        labels,
        projects,
        cycles,
        milestones,
        views,
        favorites,
        unread_count,
    }))
}

async fn set_project_links(
    conn: &mut sqlx::SqliteConnection,
    project_id: &str,
    team_ids: Option<&[String]>,
    member_ids: Option<&[String]>,
) -> ApiResult<()> {
    if let Some(team_ids) = team_ids {
        sqlx::query("DELETE FROM tracker_project_teams WHERE project_id = ?")
            .bind(project_id)
            .execute(&mut *conn)
            .await?;
        for team in team_ids {
            sqlx::query("INSERT INTO tracker_project_teams (project_id, team_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(project_id)
                .bind(team)
                .execute(&mut *conn)
                .await?;
        }
    }
    if let Some(member_ids) = member_ids {
        sqlx::query("DELETE FROM tracker_project_members WHERE project_id = ?")
            .bind(project_id)
            .execute(&mut *conn)
            .await?;
        for member in member_ids {
            sqlx::query("INSERT INTO tracker_project_members (project_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING")
                .bind(project_id)
                .bind(member)
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(())
}

pub async fn list_projects(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    me_id(&pool).await?;
    let items = load_projects(&pool, "", None).await?;
    Ok(Json(json!({ "items": items })))
}

pub async fn create_project(
    State(pool): State<SqlitePool>,
    Json(input): Json<ProjectInput>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    let me = me_id(&pool).await?;
    let name = non_empty(&input.name, "name")?;
    let status = input.status.unwrap_or_else(|| "planned".to_string());
    one_of(&status, PROJECT_STATUSES, "status")?;
    let priority = valid_priority(input.priority.unwrap_or(0))?;
    let id = new_id();
    let now = now_utc();
    let sort_order: f64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0.0) + 1.0 FROM tracker_projects")
            .fetch_one(&pool)
            .await?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query(
        "INSERT INTO tracker_projects (id, name, description, summary, icon, color, status, priority, lead_id,
            start_date, target_date, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(input.description.unwrap_or_default())
    .bind(input.summary.unwrap_or_default())
    .bind(input.icon.unwrap_or_else(|| "box".to_string()))
    .bind(input.color.unwrap_or_else(|| PALETTE[0].to_string()))
    .bind(&status)
    .bind(priority)
    .bind(input.lead_id.unwrap_or_else(|| Some(me.clone())))
    .bind(input.start_date.flatten())
    .bind(input.target_date.flatten())
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    let members = input.member_ids.unwrap_or_else(|| vec![me.clone()]);
    set_project_links(&mut tx, &id, input.team_ids.as_deref(), Some(&members)).await?;
    tx.commit().await?;
    Ok(created(load_project(&pool, &id).await?))
}

pub async fn get_project(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProjectDetail>> {
    me_id(&pool).await?;
    let project = load_project(&pool, &id).await?;
    let updates: Vec<ProjectUpdate> = sqlx::query_as(&format!(
        "SELECT {UPDATE_COLUMNS} FROM tracker_project_updates WHERE project_id = ? ORDER BY created_at DESC"
    ))
    .bind(&id)
    .fetch_all(&pool)
    .await?;
    let milestones: Vec<Milestone> = sqlx::query_as(&format!(
        "SELECT {MILESTONE_COLUMNS} FROM tracker_milestones WHERE project_id = ? ORDER BY sort_order"
    ))
    .bind(&id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(ProjectDetail {
        member_ids: project.member_ids.clone(),
        team_ids: project.team_ids.clone(),
        project,
        updates,
        milestones,
    }))
}

pub async fn update_project(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<ProjectPatch>,
) -> ApiResult<Json<Project>> {
    let current = load_project(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    let status = patch.status.unwrap_or(current.status);
    one_of(&status, PROJECT_STATUSES, "status")?;
    let priority = valid_priority(patch.priority.unwrap_or(current.priority))?;
    let archived_at = match patch.archived {
        Some(true) => current.archived_at.or_else(|| Some(now_utc())),
        Some(false) => None,
        None => current.archived_at,
    };
    sqlx::query(
        "UPDATE tracker_projects SET name = ?, description = ?, summary = ?, icon = ?, color = ?, status = ?,
            priority = ?, lead_id = ?, start_date = ?, target_date = ?, sort_order = ?, archived_at = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&name)
    .bind(patch.description.unwrap_or(current.description))
    .bind(patch.summary.unwrap_or(current.summary))
    .bind(patch.icon.unwrap_or(current.icon))
    .bind(patch.color.unwrap_or(current.color))
    .bind(&status)
    .bind(priority)
    .bind(patch.lead_id.unwrap_or(current.lead_id))
    .bind(patch.start_date.unwrap_or(current.start_date))
    .bind(patch.target_date.unwrap_or(current.target_date))
    .bind(patch.sort_order.unwrap_or(current.sort_order))
    .bind(&archived_at)
    .bind(now_utc())
    .bind(&id)
    .execute(&pool)
    .await?;
    // The pool has one connection: release it before load_project borrows the pool again.
    {
        let mut conn = pool.acquire().await?;
        set_project_links(
            &mut conn,
            &id,
            patch.team_ids.as_deref(),
            patch.member_ids.as_deref(),
        )
        .await?;
    }
    Ok(Json(load_project(&pool, &id).await?))
}

pub async fn delete_project(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    sqlx::query(
        "UPDATE tracker_issues SET project_id = NULL, milestone_id = NULL WHERE project_id = ?",
    )
    .bind(&id)
    .execute(&pool)
    .await?;
    sqlx::query("DELETE FROM tracker_favorites WHERE kind = 'project' AND target_id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    let result = sqlx::query("DELETE FROM tracker_projects WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "project")?;
    Ok(StatusCode::NO_CONTENT)
}

async fn load_project_update(pool: &SqlitePool, id: &str) -> ApiResult<ProjectUpdate> {
    sqlx::query_as(&format!(
        "SELECT {UPDATE_COLUMNS} FROM tracker_project_updates WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("project update"))
}

pub async fn add_project_update(
    State(pool): State<SqlitePool>,
    Path(project_id): Path<String>,
    Json(input): Json<ProjectUpdateCreate>,
) -> ApiResult<(StatusCode, Json<ProjectUpdate>)> {
    let me = me_id(&pool).await?;
    require(&pool, "tracker_projects", &project_id, "project").await?;
    one_of(&input.health, HEALTHS, "health")?;
    let body = non_empty(&input.body, "body")?;
    let id = new_id();
    let now = now_utc();
    sqlx::query(
        "INSERT INTO tracker_project_updates (id, project_id, user_id, health, body, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&me)
    .bind(&input.health)
    .bind(&body)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await?;
    let name: String = sqlx::query_scalar("SELECT name FROM tracker_projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await?;
    let members: Vec<String> = sqlx::query_scalar(
        "SELECT m.user_id FROM tracker_project_members m JOIN tracker_users u ON u.id = m.user_id
         WHERE m.project_id = ? AND m.user_id != ? AND u.kind = 'human'",
    )
    .bind(&project_id)
    .bind(&me)
    .fetch_all(&pool)
    .await?;
    for member in members {
        sqlx::query(
            "INSERT INTO tracker_notifications (id, user_id, issue_id, project_id, kind, actor_id, body, created_at)
             VALUES (?, ?, NULL, ?, 'project_update', ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&member)
        .bind(&project_id)
        .bind(&me)
        .bind(format!("New update on {name}"))
        .bind(&now)
        .execute(&pool)
        .await?;
    }
    Ok(created(load_project_update(&pool, &id).await?))
}

pub async fn update_project_update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<ProjectUpdatePatch>,
) -> ApiResult<Json<ProjectUpdate>> {
    let current = load_project_update(&pool, &id).await?;
    let health = patch.health.unwrap_or(current.health);
    one_of(&health, HEALTHS, "health")?;
    let body = match patch.body.as_deref() {
        Some(body) => non_empty(body, "body")?,
        None => current.body,
    };
    sqlx::query(
        "UPDATE tracker_project_updates SET health = ?, body = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&health)
    .bind(&body)
    .bind(now_utc())
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_project_update(&pool, &id).await?))
}

pub async fn delete_project_update(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM tracker_project_updates WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "project update")?;
    Ok(StatusCode::NO_CONTENT)
}

async fn load_milestone(pool: &SqlitePool, id: &str) -> ApiResult<Milestone> {
    sqlx::query_as(&format!(
        "SELECT {MILESTONE_COLUMNS} FROM tracker_milestones WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("milestone"))
}

pub async fn add_milestone(
    State(pool): State<SqlitePool>,
    Path(project_id): Path<String>,
    Json(input): Json<MilestoneCreate>,
) -> ApiResult<(StatusCode, Json<Milestone>)> {
    require(&pool, "tracker_projects", &project_id, "project").await?;
    let name = non_empty(&input.name, "name")?;
    let id = new_id();
    let sort_order = next_sort(&pool, "tracker_milestones", "project_id", &project_id).await?;
    sqlx::query("INSERT INTO tracker_milestones (id, project_id, name, target_date, sort_order) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&project_id)
        .bind(&name)
        .bind(&input.target_date)
        .bind(sort_order)
        .execute(&pool)
        .await?;
    Ok(created(load_milestone(&pool, &id).await?))
}

pub async fn update_milestone(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<MilestonePatch>,
) -> ApiResult<Json<Milestone>> {
    let current = load_milestone(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    sqlx::query(
        "UPDATE tracker_milestones SET name = ?, target_date = ?, sort_order = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(patch.target_date.unwrap_or(current.target_date))
    .bind(patch.sort_order.unwrap_or(current.sort_order))
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_milestone(&pool, &id).await?))
}

pub async fn delete_milestone(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM tracker_milestones WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "milestone")?;
    Ok(StatusCode::NO_CONTENT)
}

fn valid_range(starts_at: &str, ends_at: &str) -> ApiResult<()> {
    let parse = |value: &str| {
        chrono::DateTime::parse_from_rfc3339(value)
            .map_err(|_| TrackerError::bad_request(format!("invalid timestamp: {value}")))
    };
    if parse(ends_at)? <= parse(starts_at)? {
        return Err(TrackerError::bad_request("ends_at must be after starts_at"));
    }
    Ok(())
}

pub async fn create_cycle(
    State(pool): State<SqlitePool>,
    Json(input): Json<CycleCreate>,
) -> ApiResult<(StatusCode, Json<Cycle>)> {
    require(&pool, "tracker_teams", &input.team_id, "team").await?;
    valid_range(&input.starts_at, &input.ends_at)?;
    let number: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(number), 0) + 1 FROM tracker_cycles WHERE team_id = ?",
    )
    .bind(&input.team_id)
    .fetch_one(&pool)
    .await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO tracker_cycles (id, team_id, number, name, description, starts_at, ends_at, completed_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(&id)
    .bind(&input.team_id)
    .bind(number)
    .bind(input.name.filter(|name| !name.trim().is_empty()))
    .bind(&input.description)
    .bind(&input.starts_at)
    .bind(&input.ends_at)
    .execute(&pool)
    .await?;
    sqlx::query("UPDATE tracker_teams SET cycles_enabled = 1 WHERE id = ?")
        .bind(&input.team_id)
        .execute(&pool)
        .await?;
    Ok(created(load_cycle(&pool, &id).await?))
}

pub async fn update_cycle(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<CyclePatch>,
) -> ApiResult<Json<Cycle>> {
    let current = load_cycle(&pool, &id).await?;
    let starts_at = patch.starts_at.unwrap_or(current.starts_at);
    let ends_at = patch.ends_at.unwrap_or(current.ends_at);
    valid_range(&starts_at, &ends_at)?;
    sqlx::query(
        "UPDATE tracker_cycles SET name = ?, description = ?, starts_at = ?, ends_at = ?, completed_at = ? WHERE id = ?",
    )
    .bind(patch.name.unwrap_or(current.name))
    .bind(patch.description.unwrap_or(current.description))
    .bind(&starts_at)
    .bind(&ends_at)
    .bind(patch.completed_at.unwrap_or(current.completed_at))
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_cycle(&pool, &id).await?))
}

pub async fn delete_cycle(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE tracker_issues SET cycle_id = NULL WHERE cycle_id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    let result = sqlx::query("DELETE FROM tracker_cycles WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "cycle")?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_label(
    State(pool): State<SqlitePool>,
    Json(input): Json<LabelCreate>,
) -> ApiResult<(StatusCode, Json<Label>)> {
    let name = non_empty(&input.name, "name")?;
    let color = non_empty(&input.color, "color")?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO tracker_labels (id, team_id, name, color, parent_id, description, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.team_id)
    .bind(&name)
    .bind(&color)
    .bind(&input.parent_id)
    .bind(&input.description)
    .bind(now_utc())
    .execute(&pool)
    .await?;
    Ok(created(load_label(&pool, &id).await?))
}

pub async fn update_label(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<LabelPatch>,
) -> ApiResult<Json<Label>> {
    let current = load_label(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    let parent_id = patch.parent_id.unwrap_or(current.parent_id);
    if parent_id.as_deref() == Some(id.as_str()) {
        return Err(TrackerError::bad_request(
            "a label cannot be its own parent",
        ));
    }
    // The ancestor walk and the write share one BEGIN IMMEDIATE transaction, so two concurrent
    // cross-parent patches serialize: the second one sees the first edge and is rejected instead
    // of both passing their check and closing a cycle.
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(parent) = parent_id.as_deref() {
        // Walk up from the new parent; reaching this label means the change closes a cycle.
        let cycle: Option<i64> = sqlx::query_scalar(
            "WITH RECURSIVE up(id) AS (
                SELECT ? UNION SELECT l.parent_id FROM tracker_labels l JOIN up ON l.id = up.id
                WHERE l.parent_id IS NOT NULL
             )
             SELECT 1 FROM up WHERE id = ? LIMIT 1",
        )
        .bind(parent)
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?;
        if cycle.is_some() {
            return Err(TrackerError::bad_request(
                "a label cannot be nested under its own descendant",
            ));
        }
    }
    sqlx::query("UPDATE tracker_labels SET name = ?, color = ?, parent_id = ?, description = ? WHERE id = ?")
        .bind(&name)
        .bind(patch.color.unwrap_or(current.color))
        .bind(&parent_id)
        .bind(patch.description.unwrap_or(current.description))
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(load_label(&pool, &id).await?))
}

pub async fn delete_label(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM tracker_labels WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "label")?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_team(
    State(pool): State<SqlitePool>,
    Json(input): Json<TeamCreate>,
) -> ApiResult<(StatusCode, Json<Team>)> {
    me_id(&pool).await?;
    let key = input.key.trim().to_ascii_uppercase();
    if key.is_empty() || key.len() > 5 || !key.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(TrackerError::bad_request("key must be 1-5 letters"));
    }
    let name = non_empty(&input.name, "name")?;
    let id = new_id();
    let sort_order: f64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), 0.0) + 1.0 FROM tracker_teams WHERE system = 0",
    )
    .fetch_one(&pool)
    .await?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query(
        "INSERT INTO tracker_teams (id, key, name, icon, color, issue_seq, system, cycles_enabled, cycle_weeks,
            triage_enabled, created_at, sort_order)
         VALUES (?, ?, ?, ?, ?, 0, 0, ?, 2, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&key)
    .bind(&name)
    .bind(input.icon.unwrap_or_else(|| "box".to_string()))
    .bind(input.color.unwrap_or_else(|| PALETTE[1].to_string()))
    .bind(input.cycles_enabled.unwrap_or(false))
    .bind(input.triage_enabled.unwrap_or(false))
    .bind(now_utc())
    .bind(sort_order)
    .execute(&mut *tx)
    .await?;
    seed_team_states(&mut tx, &id).await?;
    tx.commit().await?;
    Ok(created(load_team(&pool, &id).await?))
}

pub async fn update_team(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<TeamPatch>,
) -> ApiResult<Json<Team>> {
    let current = load_team(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    let cycle_weeks = patch.cycle_weeks.unwrap_or(current.cycle_weeks);
    if !(1..=8).contains(&cycle_weeks) {
        return Err(TrackerError::bad_request(
            "cycle_weeks must be between 1 and 8",
        ));
    }
    sqlx::query(
        "UPDATE tracker_teams SET name = ?, icon = ?, color = ?, cycles_enabled = ?, cycle_weeks = ?,
            triage_enabled = ?, sort_order = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(patch.icon.unwrap_or(current.icon))
    .bind(patch.color.unwrap_or(current.color))
    .bind(patch.cycles_enabled.unwrap_or(current.cycles_enabled))
    .bind(cycle_weeks)
    .bind(patch.triage_enabled.unwrap_or(current.triage_enabled))
    .bind(patch.sort_order.unwrap_or(current.sort_order))
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_team(&pool, &id).await?))
}

async fn reject_system_team(pool: &SqlitePool, team_id: &str) -> ApiResult<()> {
    let system: Option<bool> = sqlx::query_scalar("SELECT system FROM tracker_teams WHERE id = ?")
        .bind(team_id)
        .fetch_optional(pool)
        .await?;
    match system {
        None => Err(TrackerError::not_found("team")),
        Some(true) => Err(TrackerError::conflict(
            "system team workflows are managed automatically and cannot be edited",
            "system_team",
        )),
        Some(false) => Ok(()),
    }
}

pub async fn create_state(
    State(pool): State<SqlitePool>,
    Json(input): Json<StateCreate>,
) -> ApiResult<(StatusCode, Json<WorkflowState>)> {
    reject_system_team(&pool, &input.team_id).await?;
    let name = non_empty(&input.name, "name")?;
    one_of(&input.state_type, STATE_TYPES, "type")?;
    let position =
        match input.position {
            Some(position) => position,
            None => sqlx::query_scalar(
                "SELECT COALESCE(MAX(position), 0.0) + 1.0 FROM tracker_states WHERE team_id = ?",
            )
            .bind(&input.team_id)
            .fetch_one(&pool)
            .await?,
        };
    let id = new_id();
    sqlx::query(
        "INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description)
         VALUES (?, ?, ?, ?, ?, ?, NULL, ?)",
    )
    .bind(&id)
    .bind(&input.team_id)
    .bind(&name)
    .bind(&input.state_type)
    .bind(&input.color)
    .bind(position)
    .bind(&input.description)
    .execute(&pool)
    .await?;
    Ok(created(load_state(&pool, &id).await?))
}

pub async fn update_state(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<StatePatch>,
) -> ApiResult<Json<WorkflowState>> {
    let current = load_state(&pool, &id).await?;
    reject_system_team(&pool, &current.team_id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    sqlx::query(
        "UPDATE tracker_states SET name = ?, color = ?, position = ?, description = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(patch.color.unwrap_or(current.color))
    .bind(patch.position.unwrap_or(current.position))
    .bind(patch.description.unwrap_or(current.description))
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_state(&pool, &id).await?))
}

pub async fn delete_state(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let current = load_state(&pool, &id).await?;
    reject_system_team(&pool, &current.team_id).await?;
    let in_use: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracker_issues WHERE state_id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await?;
    if in_use > 0 {
        return Err(TrackerError::conflict(
            format!("{in_use} issues use this state; move them first"),
            "state_in_use",
        ));
    }
    let same_type: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tracker_states WHERE team_id = ? AND type = ?")
            .bind(&current.team_id)
            .bind(&current.state_type)
            .fetch_one(&pool)
            .await?;
    if same_type <= 1 {
        return Err(TrackerError::conflict(
            "a team needs at least one state of each category",
            "last_state_of_type",
        ));
    }
    sqlx::query("DELETE FROM tracker_states WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_user(
    State(pool): State<SqlitePool>,
    Json(input): Json<UserCreate>,
) -> ApiResult<(StatusCode, Json<User>)> {
    me_id(&pool).await?;
    let name = non_empty(&input.name, "name")?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO tracker_users (id, name, display_name, email, avatar_color, kind, model, is_me, created_at)
         VALUES (?, ?, ?, ?, ?, 'human', NULL, 0, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(input.display_name.unwrap_or_else(|| name.clone()))
    .bind(input.email.filter(|email| !email.trim().is_empty()))
    .bind(input.avatar_color.unwrap_or_else(|| deterministic_avatar_color(&name)))
    .bind(now_utc())
    .execute(&pool)
    .await?;
    Ok(created(load_user(&pool, &id).await?))
}

pub async fn update_user(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<UserPatch>,
) -> ApiResult<Json<User>> {
    let current = load_user(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    sqlx::query("UPDATE tracker_users SET name = ?, display_name = ?, email = ?, avatar_color = ? WHERE id = ?")
        .bind(&name)
        .bind(patch.display_name.unwrap_or(current.display_name))
        .bind(patch.email.unwrap_or(current.email))
        .bind(patch.avatar_color.unwrap_or(current.avatar_color))
        .bind(&id)
        .execute(&pool)
        .await?;
    Ok(Json(load_user(&pool, &id).await?))
}

fn require_object(value: &Value, field: &str) -> ApiResult<String> {
    if value.is_object() {
        Ok(value.to_string())
    } else {
        Err(TrackerError::bad_request(format!(
            "{field} must be an object"
        )))
    }
}

pub async fn create_view(
    State(pool): State<SqlitePool>,
    Json(input): Json<ViewCreate>,
) -> ApiResult<(StatusCode, Json<View>)> {
    let me = me_id(&pool).await?;
    let name = non_empty(&input.name, "name")?;
    let filters = require_object(&input.filters, "filters")?;
    let display = require_object(&input.display, "display")?;
    let id = new_id();
    let now = now_utc();
    sqlx::query(
        "INSERT INTO tracker_views (id, name, description, icon, color, owner_id, team_id, filters, display, shared,
            created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&input.description)
    .bind(input.icon.unwrap_or_else(|| "layers".to_string()))
    .bind(&input.color)
    .bind(&me)
    .bind(&input.team_id)
    .bind(&filters)
    .bind(&display)
    .bind(input.shared.unwrap_or(true))
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await?;
    Ok(created(load_view(&pool, &id).await?))
}

pub async fn update_view(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<ViewPatch>,
) -> ApiResult<Json<View>> {
    let current = load_view(&pool, &id).await?;
    let name = match patch.name.as_deref() {
        Some(name) => non_empty(name, "name")?,
        None => current.name,
    };
    let filters = require_object(
        patch.filters.as_ref().unwrap_or(&current.filters),
        "filters",
    )?;
    let display = require_object(
        patch.display.as_ref().unwrap_or(&current.display),
        "display",
    )?;
    sqlx::query(
        "UPDATE tracker_views SET name = ?, description = ?, icon = ?, color = ?, team_id = ?, filters = ?, display = ?,
            shared = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(patch.description.unwrap_or(current.description))
    .bind(patch.icon.unwrap_or(current.icon))
    .bind(patch.color.unwrap_or(current.color))
    .bind(patch.team_id.unwrap_or(current.team_id))
    .bind(&filters)
    .bind(&display)
    .bind(patch.shared.unwrap_or(current.shared))
    .bind(now_utc())
    .bind(&id)
    .execute(&pool)
    .await?;
    Ok(Json(load_view(&pool, &id).await?))
}

pub async fn delete_view(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM tracker_favorites WHERE kind = 'view' AND target_id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    let result = sqlx::query("DELETE FROM tracker_views WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "view")?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn toggle_favorite(
    State(pool): State<SqlitePool>,
    Json(input): Json<FavoriteToggle>,
) -> ApiResult<Json<Value>> {
    let me = me_id(&pool).await?;
    one_of(&input.kind, FAVORITE_KINDS, "kind")?;
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_favorites WHERE user_id = ? AND kind = ? AND target_id = ?",
    )
    .bind(&me)
    .bind(&input.kind)
    .bind(&input.target_id)
    .fetch_optional(&pool)
    .await?;
    if let Some(id) = existing {
        sqlx::query("DELETE FROM tracker_favorites WHERE id = ?")
            .bind(&id)
            .execute(&pool)
            .await?;
        return Ok(Json(json!({ "favorited": false, "favorite": null })));
    }
    let favorite = Favorite {
        id: new_id(),
        user_id: me.clone(),
        kind: input.kind,
        target_id: input.target_id,
        sort_order: next_sort(&pool, "tracker_favorites", "user_id", &me).await?,
    };
    sqlx::query("INSERT INTO tracker_favorites (id, user_id, kind, target_id, sort_order) VALUES (?, ?, ?, ?, ?)")
        .bind(&favorite.id)
        .bind(&favorite.user_id)
        .bind(&favorite.kind)
        .bind(&favorite.target_id)
        .bind(favorite.sort_order)
        .execute(&pool)
        .await?;
    Ok(Json(json!({ "favorited": true, "favorite": favorite })))
}

pub async fn list_notifications(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let me = prepare(&pool).await?;
    let items: Vec<Notification> = sqlx::query_as(&format!(
        "SELECT {NOTIFICATION_COLUMNS} FROM tracker_notifications WHERE user_id = ? AND archived_at IS NULL
         ORDER BY created_at DESC LIMIT 500"
    ))
    .bind(&me)
    .fetch_all(&pool)
    .await?;
    Ok(Json(json!({ "items": items })))
}

pub async fn update_notification(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(patch): Json<NotificationPatch>,
) -> ApiResult<Json<Notification>> {
    let current: Notification = sqlx::query_as(&format!(
        "SELECT {NOTIFICATION_COLUMNS} FROM tracker_notifications WHERE id = ?"
    ))
    .bind(&id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| TrackerError::not_found("notification"))?;
    let now = now_utc();
    let read_at = match patch.read {
        Some(true) => current.read_at.or_else(|| Some(now.clone())),
        Some(false) => None,
        None => current.read_at,
    };
    let archived_at = match patch.archived {
        Some(true) => current.archived_at.or_else(|| Some(now.clone())),
        Some(false) => None,
        None => current.archived_at,
    };
    sqlx::query("UPDATE tracker_notifications SET read_at = ?, snoozed_until = ?, archived_at = ? WHERE id = ?")
        .bind(&read_at)
        .bind(patch.snoozed_until.unwrap_or(current.snoozed_until))
        .bind(&archived_at)
        .bind(&id)
        .execute(&pool)
        .await?;
    let updated: Notification = sqlx::query_as(&format!(
        "SELECT {NOTIFICATION_COLUMNS} FROM tracker_notifications WHERE id = ?"
    ))
    .bind(&id)
    .fetch_one(&pool)
    .await?;
    Ok(Json(updated))
}

pub async fn read_all_notifications(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let me = me_id(&pool).await?;
    let result = sqlx::query(
        "UPDATE tracker_notifications SET read_at = ? WHERE user_id = ? AND read_at IS NULL AND archived_at IS NULL",
    )
    .bind(now_utc())
    .bind(&me)
    .execute(&pool)
    .await?;
    Ok(Json(json!({ "updated": result.rows_affected() })))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
}

pub async fn search(
    State(pool): State<SqlitePool>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Value>> {
    prepare(&pool).await?;
    let q = query.q.unwrap_or_default();
    let q = q.trim();
    if q.is_empty() {
        return Ok(Json(json!({ "issues": [], "projects": [] })));
    }
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let issues = load_issues(
        &pool,
        "WHERE i.title LIKE ?1 ESCAPE '\\' OR i.identifier LIKE ?1 ESCAPE '\\' OR i.description LIKE ?1 ESCAPE '\\'
         ORDER BY CASE WHEN i.identifier LIKE ?1 ESCAPE '\\' THEN 0 WHEN i.title LIKE ?1 ESCAPE '\\' THEN 1 ELSE 2 END,
                  i.updated_at DESC
         LIMIT 50",
        &[&pattern],
    )
    .await?;
    let projects = load_projects(&pool, "WHERE p.name LIKE ? ESCAPE '\\'", Some(&pattern)).await?;
    Ok(Json(json!({ "issues": issues, "projects": projects })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use crate::tracker::seed::ensure_seeded;
    use axum::response::IntoResponse;

    async fn insert_label(pool: &SqlitePool, id: &str, parent: Option<&str>) {
        sqlx::query(
            "INSERT INTO tracker_labels (id, team_id, name, color, parent_id, description, created_at)
             VALUES (?, NULL, ?, '#ffffff', ?, NULL, ?)",
        )
        .bind(id)
        .bind(id)
        .bind(parent)
        .bind(now_utc())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_project_with_unknown_team_leaves_no_project_behind() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        ensure_seeded(&pool).await.unwrap();
        let input = ProjectInput {
            name: "Orphan".into(),
            description: None,
            summary: None,
            icon: None,
            color: None,
            status: None,
            priority: None,
            lead_id: None,
            start_date: None,
            target_date: None,
            team_ids: Some(vec!["missing-team".into()]),
            member_ids: None,
        };
        let result = create_project(State(pool.clone()), Json(input)).await;
        assert!(result.is_err(), "an unknown team id must fail the create");
        let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracker_projects")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(projects, 0);
    }

    #[tokio::test]
    async fn update_label_rejects_indirect_parent_cycle() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        ensure_seeded(&pool).await.unwrap();
        insert_label(&pool, "label-a", None).await;
        insert_label(&pool, "label-b", Some("label-a")).await;
        insert_label(&pool, "label-c", Some("label-b")).await;

        let patch = LabelPatch {
            parent_id: Some(Some("label-c".into())),
            ..Default::default()
        };
        let result = update_label(State(pool.clone()), Path("label-a".into()), Json(patch)).await;
        assert!(
            result.is_err(),
            "A under C closes the cycle A <- B <- C <- A"
        );
        let parent: Option<String> =
            sqlx::query_scalar("SELECT parent_id FROM tracker_labels WHERE id = 'label-a'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(parent, None);

        let ok = LabelPatch {
            parent_id: Some(Some("label-c".into())),
            ..Default::default()
        };
        insert_label(&pool, "label-d", None).await;
        let updated = update_label(State(pool.clone()), Path("label-d".into()), Json(ok))
            .await
            .unwrap();
        assert_eq!(updated.parent_id.as_deref(), Some("label-c"));
    }

    /// Two cross-parent label patches at once: A under B and B under A. The ancestor walk and the
    /// write share one write transaction, so exactly one wins and the hierarchy stays acyclic.
    #[tokio::test]
    async fn concurrent_cross_parent_label_patches_leave_no_cycle() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        ensure_seeded(&pool).await.unwrap();
        insert_label(&pool, "label-a", None).await;
        insert_label(&pool, "label-b", None).await;

        // An open write transaction excludes a second writer; that is what serializes the
        // ancestor walk with the parent write.
        let held = pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let mut blocked = Box::pin(pool.begin_with("BEGIN IMMEDIATE"));
        assert!(
            futures_util::poll!(blocked.as_mut()).is_pending(),
            "a second writer must wait for the open transaction"
        );
        held.commit().await.unwrap();
        blocked.await.unwrap().commit().await.unwrap();

        let under_b = LabelPatch {
            parent_id: Some(Some("label-b".into())),
            ..Default::default()
        };
        let under_a = LabelPatch {
            parent_id: Some(Some("label-a".into())),
            ..Default::default()
        };
        let outcomes = tokio::join!(
            update_label(State(pool.clone()), Path("label-a".into()), Json(under_b)),
            update_label(State(pool.clone()), Path("label-b".into()), Json(under_a)),
        );
        let results = [outcomes.0, outcomes.1];
        assert_eq!(
            results.iter().filter(|result| result.is_ok()).count(),
            1,
            "exactly one cross-parent label patch may win"
        );
        let rejected = results
            .into_iter()
            .find_map(|result| result.err())
            .expect("the losing patch is rejected");
        assert_eq!(rejected.into_response().status(), StatusCode::BAD_REQUEST);

        for start in ["label-a", "label-b"] {
            let mut current = Some(start.to_string());
            for _ in 0..8 {
                let Some(id) = current else { break };
                let parent: Option<String> =
                    sqlx::query_scalar("SELECT parent_id FROM tracker_labels WHERE id = ?")
                        .bind(&id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                assert_ne!(parent.as_deref(), Some(start), "cycle through {start}");
                current = parent;
            }
        }
    }
}

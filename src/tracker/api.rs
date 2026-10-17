use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::OmonError;
use crate::tracker::{issues, lanes, seed, threads, workspace};

#[derive(Debug)]
pub struct TrackerError {
    status: StatusCode,
    message: String,
    code: Option<&'static str>,
}

impl TrackerError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
            code: None,
        }
    }

    pub fn not_found(resource: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: format!("{resource} not found"),
            code: Some("not_found"),
        }
    }

    pub fn conflict(message: impl Into<String>, code: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
            code: Some(code),
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
            code: None,
        }
    }
}

impl From<OmonError> for TrackerError {
    fn from(error: OmonError) -> Self {
        match error {
            OmonError::Config(message) => Self::bad_request(message),
            other => Self::internal(other.to_string()),
        }
    }
}

impl From<sqlx::Error> for TrackerError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &error {
            if db.is_unique_violation() {
                return Self::conflict(db.message().to_string(), "conflict");
            }
            if db.is_foreign_key_violation() || db.is_check_violation() {
                return Self::bad_request(db.message().to_string());
            }
        }
        Self::internal(error.to_string())
    }
}

impl IntoResponse for TrackerError {
    fn into_response(self) -> Response {
        let body = match self.code {
            Some(code) => json!({"error": self.message, "code": code}),
            None => json!({"error": self.message}),
        };
        (self.status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = std::result::Result<T, TrackerError>;

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn non_empty(value: &str, field: &str) -> ApiResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(TrackerError::bad_request(format!(
            "{field} must not be empty"
        )))
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn one_of(value: &str, allowed: &[&str], field: &str) -> ApiResult<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(TrackerError::bad_request(format!(
            "{field} must be one of {}",
            allowed.join(", ")
        )))
    }
}

pub fn valid_priority(priority: i64) -> ApiResult<i64> {
    if (0..=4).contains(&priority) {
        Ok(priority)
    } else {
        Err(TrackerError::bad_request(
            "priority must be between 0 and 4",
        ))
    }
}

/// `table` is always a compile-time constant, never user input.
pub async fn require(
    pool: &SqlitePool,
    table: &'static str,
    id: &str,
    what: &str,
) -> ApiResult<()> {
    let found: Option<i64> = sqlx::query_scalar(&format!("SELECT 1 FROM {table} WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    match found {
        Some(_) => Ok(()),
        None => Err(TrackerError::not_found(what)),
    }
}

/// `table` and `column` are compile-time constants, never user input.
pub async fn next_sort(
    pool: &SqlitePool,
    table: &'static str,
    column: &'static str,
    value: &str,
) -> ApiResult<f64> {
    let next: f64 = sqlx::query_scalar(&format!(
        "SELECT COALESCE(MAX(sort_order), 0.0) + 1.0 FROM {table} WHERE {column} = ?"
    ))
    .bind(value)
    .fetch_one(pool)
    .await?;
    Ok(next)
}

pub async fn me_id(pool: &SqlitePool) -> ApiResult<String> {
    seed::ensure_seeded(pool).await?;
    let id: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_users WHERE is_me = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;
    id.ok_or_else(|| TrackerError::not_found("current user"))
}

pub async fn prepare(pool: &SqlitePool) -> ApiResult<String> {
    let me = me_id(pool).await?;
    lanes::sync(pool).await?;
    Ok(me)
}

pub fn router<S>(pool: SqlitePool) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/bootstrap", get(workspace::bootstrap))
        .route("/issues", get(issues::list).post(issues::create))
        .route("/issues/batch", post(issues::batch))
        .route(
            "/issues/{id}",
            get(issues::detail)
                .patch(issues::update)
                .delete(issues::remove),
        )
        .route("/issues/{id}/comments", post(threads::add_comment))
        .route(
            "/comments/{id}",
            patch(threads::update_comment).delete(threads::delete_comment),
        )
        .route("/comments/{id}/reactions", post(threads::toggle_reaction))
        .route("/issues/{id}/relations", post(threads::add_relation))
        .route("/relations/{id}", delete(threads::delete_relation))
        .route("/issues/{id}/links", post(threads::add_link))
        .route("/links/{id}", delete(threads::delete_link))
        .route("/issues/{id}/subscribe", post(threads::subscribe))
        .route("/issues/{id}/delegate", post(threads::delegate))
        .route("/issues/{id}/lane-action", post(threads::lane_action))
        .route(
            "/projects",
            get(workspace::list_projects).post(workspace::create_project),
        )
        .route(
            "/projects/{id}",
            get(workspace::get_project)
                .patch(workspace::update_project)
                .delete(workspace::delete_project),
        )
        .route(
            "/projects/{id}/updates",
            post(workspace::add_project_update),
        )
        .route(
            "/project-updates/{id}",
            patch(workspace::update_project_update).delete(workspace::delete_project_update),
        )
        .route("/projects/{id}/milestones", post(workspace::add_milestone))
        .route(
            "/milestones/{id}",
            patch(workspace::update_milestone).delete(workspace::delete_milestone),
        )
        .route("/cycles", post(workspace::create_cycle))
        .route(
            "/cycles/{id}",
            patch(workspace::update_cycle).delete(workspace::delete_cycle),
        )
        .route("/labels", post(workspace::create_label))
        .route(
            "/labels/{id}",
            patch(workspace::update_label).delete(workspace::delete_label),
        )
        .route("/teams", post(workspace::create_team))
        .route("/teams/{id}", patch(workspace::update_team))
        .route("/states", post(workspace::create_state))
        .route(
            "/states/{id}",
            patch(workspace::update_state).delete(workspace::delete_state),
        )
        .route("/users", post(workspace::create_user))
        .route("/users/{id}", patch(workspace::update_user))
        .route("/views", post(workspace::create_view))
        .route(
            "/views/{id}",
            patch(workspace::update_view).delete(workspace::delete_view),
        )
        .route("/favorites", post(workspace::toggle_favorite))
        .route("/notifications", get(workspace::list_notifications))
        .route(
            "/notifications/read-all",
            post(workspace::read_all_notifications),
        )
        .route("/notifications/{id}", patch(workspace::update_notification))
        .route("/search", get(workspace::search))
        .with_state(pool)
}

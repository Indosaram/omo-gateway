use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::SqlitePool;

use crate::error::OmonError;

use crate::tracker::api::{me_id, new_id, non_empty, one_of, ApiResult, TrackerError};
use crate::tracker::issues::{
    fetch_issue_detail, load_comments, load_issue, notify, record_activity, relation_direction,
    resolve_issue_id, row_count_error, subscribe_user,
};
use crate::tracker::lanes;
use crate::tracker::model::{
    Comment, CommentCreate, CommentPatch, DelegateInput, IssueDetail, LaneActionInput, Link,
    LinkCreate, ReactionToggle, Relation, RelationCreate, RelationDetail, SubscribeInput,
};
use crate::tracker::seed::now_utc;

async fn load_comment(pool: &SqlitePool, id: &str) -> ApiResult<Comment> {
    load_comments(pool, "WHERE id = ?", id)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| TrackerError::not_found("comment"))
}

async fn touch_issue(pool: &SqlitePool, issue_id: &str) -> ApiResult<()> {
    sqlx::query("UPDATE tracker_issues SET updated_at = ? WHERE id = ?")
        .bind(now_utc())
        .bind(issue_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_comment(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<CommentCreate>,
) -> ApiResult<(StatusCode, Json<Comment>)> {
    let me = me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    let body = non_empty(&input.body, "body")?;
    if let Some(parent) = input.parent_id.as_deref() {
        let parent_issue: Option<String> =
            sqlx::query_scalar("SELECT issue_id FROM tracker_comments WHERE id = ?")
                .bind(parent)
                .fetch_optional(&pool)
                .await?;
        if parent_issue.as_deref() != Some(issue_id.as_str()) {
            return Err(TrackerError::bad_request(
                "parent comment belongs to another issue",
            ));
        }
    }
    let comment_id = new_id();
    let now = now_utc();
    sqlx::query(
        "INSERT INTO tracker_comments (id, issue_id, user_id, body, parent_id, created_at, updated_at, edited, resolved_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL)",
    )
    .bind(&comment_id)
    .bind(&issue_id)
    .bind(&me)
    .bind(&body)
    .bind(&input.parent_id)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await?;
    subscribe_user(&pool, &issue_id, &me).await?;
    touch_issue(&pool, &issue_id).await?;

    let identifier: String =
        sqlx::query_scalar("SELECT identifier FROM tracker_issues WHERE id = ?")
            .bind(&issue_id)
            .fetch_one(&pool)
            .await?;
    let subscribers: Vec<String> = sqlx::query_scalar(
        "SELECT s.user_id FROM tracker_subscribers s JOIN tracker_users u ON u.id = s.user_id
         WHERE s.issue_id = ? AND s.user_id != ? AND u.kind = 'human'",
    )
    .bind(&issue_id)
    .bind(&me)
    .fetch_all(&pool)
    .await?;
    let excerpt: String = body.chars().take(140).collect();
    for user in subscribers {
        notify(
            &pool,
            &user,
            &issue_id,
            "comment",
            &me,
            &format!("New comment on {identifier}: {excerpt}"),
        )
        .await?;
    }
    Ok((
        StatusCode::CREATED,
        Json(load_comment(&pool, &comment_id).await?),
    ))
}

pub async fn update_comment(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<CommentPatch>,
) -> ApiResult<Json<Comment>> {
    let current = load_comment(&pool, &id).await?;
    let now = now_utc();
    if let Some(body) = input.body.as_deref() {
        let body = non_empty(body, "body")?;
        if body != current.body {
            sqlx::query(
                "UPDATE tracker_comments SET body = ?, edited = 1, updated_at = ? WHERE id = ?",
            )
            .bind(&body)
            .bind(&now)
            .bind(&id)
            .execute(&pool)
            .await?;
        }
    }
    if let Some(resolved) = input.resolved {
        sqlx::query("UPDATE tracker_comments SET resolved_at = ? WHERE id = ?")
            .bind(resolved.then_some(now.as_str()))
            .bind(&id)
            .execute(&pool)
            .await?;
    }
    Ok(Json(load_comment(&pool, &id).await?))
}

pub async fn delete_comment(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM tracker_comments WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "comment")?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn toggle_reaction(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<ReactionToggle>,
) -> ApiResult<Json<Comment>> {
    let me = me_id(&pool).await?;
    load_comment(&pool, &id).await?;
    let emoji = non_empty(&input.emoji, "emoji")?;
    let removed = sqlx::query(
        "DELETE FROM tracker_reactions WHERE comment_id = ? AND user_id = ? AND emoji = ?",
    )
    .bind(&id)
    .bind(&me)
    .bind(&emoji)
    .execute(&pool)
    .await?;
    if removed.rows_affected() == 0 {
        sqlx::query("INSERT INTO tracker_reactions (comment_id, user_id, emoji) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(&me)
            .bind(&emoji)
            .execute(&pool)
            .await?;
    }
    Ok(Json(load_comment(&pool, &id).await?))
}

pub async fn add_relation(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<RelationCreate>,
) -> ApiResult<(StatusCode, Json<RelationDetail>)> {
    let me = me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    let related = resolve_issue_id(&pool, &input.related_issue_id).await?;
    one_of(
        &input.relation_type,
        &["blocks", "related", "duplicate"],
        "type",
    )?;
    if related == issue_id {
        return Err(TrackerError::bad_request(
            "an issue cannot relate to itself",
        ));
    }
    let relation = Relation {
        id: new_id(),
        issue_id: issue_id.clone(),
        related_issue_id: related.clone(),
        relation_type: input.relation_type,
        created_at: now_utc(),
    };
    sqlx::query(
        "INSERT INTO tracker_relations (id, issue_id, related_issue_id, type, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&relation.id)
    .bind(&relation.issue_id)
    .bind(&relation.related_issue_id)
    .bind(&relation.relation_type)
    .bind(&relation.created_at)
    .execute(&pool)
    .await?;
    record_activity(
        &pool,
        &issue_id,
        Some(&me),
        "relation_added",
        None,
        Some(&related),
    )
    .await?;
    touch_issue(&pool, &issue_id).await?;
    let direction = relation_direction(&relation, &issue_id).to_string();
    let other = load_issue(&pool, &related).await?;
    Ok((
        StatusCode::CREATED,
        Json(RelationDetail {
            id: relation.id,
            issue_id: relation.issue_id,
            related_issue_id: relation.related_issue_id,
            relation_type: relation.relation_type,
            created_at: relation.created_at,
            issue: other,
            direction,
        }),
    ))
}

pub async fn delete_relation(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let me = me_id(&pool).await?;
    let relation: Option<(String, String)> =
        sqlx::query_as("SELECT issue_id, related_issue_id FROM tracker_relations WHERE id = ?")
            .bind(&id)
            .fetch_optional(&pool)
            .await?;
    let (issue_id, related) = relation.ok_or_else(|| TrackerError::not_found("relation"))?;
    sqlx::query("DELETE FROM tracker_relations WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    record_activity(
        &pool,
        &issue_id,
        Some(&me),
        "relation_removed",
        Some(&related),
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_link(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<LinkCreate>,
) -> ApiResult<(StatusCode, Json<Link>)> {
    let issue_id = resolve_issue_id(&pool, &id).await?;
    let url = non_empty(&input.url, "url")?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(TrackerError::bad_request(
            "url must start with http:// or https://",
        ));
    }
    let title = input
        .title
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| url.clone());
    let link = Link {
        id: new_id(),
        issue_id,
        url,
        title,
        created_at: now_utc(),
    };
    sqlx::query(
        "INSERT INTO tracker_links (id, issue_id, url, title, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&link.id)
    .bind(&link.issue_id)
    .bind(&link.url)
    .bind(&link.title)
    .bind(&link.created_at)
    .execute(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(link)))
}

pub async fn delete_link(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM tracker_links WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await?;
    row_count_error(result.rows_affected(), "link")?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn subscribe(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<SubscribeInput>,
) -> ApiResult<Json<serde_json::Value>> {
    let me = me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    if input.subscribed {
        subscribe_user(&pool, &issue_id, &me).await?;
    } else {
        sqlx::query("DELETE FROM tracker_subscribers WHERE issue_id = ? AND user_id = ?")
            .bind(&issue_id)
            .bind(&me)
            .execute(&pool)
            .await?;
    }
    Ok(Json(serde_json::json!({ "subscribed": input.subscribed })))
}

pub async fn delegate(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<DelegateInput>,
) -> ApiResult<Json<IssueDetail>> {
    me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    non_empty(&input.cwd, "cwd")?;
    non_empty(&input.done, "done")?;
    if let Some(model) = input.model.as_deref() {
        one_of(model, &["auto", "small", "large"], "model")?;
    }
    lanes::delegate_issue(&pool, &issue_id, input).await?;
    Ok(Json(fetch_issue_detail(&pool, &issue_id).await?))
}

pub async fn lane_action(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(input): Json<LaneActionInput>,
) -> ApiResult<Json<IssueDetail>> {
    me_id(&pool).await?;
    let issue_id = resolve_issue_id(&pool, &id).await?;
    one_of(
        &input.action,
        &["send", "object", "close", "reopen"],
        "action",
    )?;
    lanes::execute_lane_action(&pool, &issue_id, input)
        .await
        .map_err(lane_action_error)?;
    Ok(Json(fetch_issue_detail(&pool, &issue_id).await?))
}

/// Missing issue -> 404; a rejected lane command or illegal transition -> 409; other
/// validation errors -> 400; anything else keeps the default (500) mapping.
fn lane_action_error(error: OmonError) -> TrackerError {
    match error {
        OmonError::Config(message) => {
            if message == "issue not found" {
                TrackerError::not_found("issue")
            } else if message.starts_with("lane action failed")
                || message.contains("IllegalTransition")
                || message.starts_with("illegal lane transition")
                || message.starts_with("lane must be")
            {
                TrackerError::conflict(message, "lane_command_failed")
            } else {
                TrackerError::bad_request(message)
            }
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn status(error: OmonError) -> StatusCode {
        lane_action_error(error).into_response().status()
    }

    #[test]
    fn lane_action_errors_map_to_404_400_and_409() {
        assert_eq!(
            status(OmonError::Config("issue not found".into())),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status(OmonError::Config("issue is not linked to a lane".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(OmonError::Config("text must not be empty".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(OmonError::Config("lane action failed: {}".into())),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(OmonError::Config(
                "IllegalTransition: working -> closed".into()
            )),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(OmonError::Database("disk".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}

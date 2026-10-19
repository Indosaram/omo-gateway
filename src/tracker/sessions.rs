use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::error::OmonError;
use crate::sessions::control::{ControlError, SendMode, SessionControl, Target};
use crate::sessions::overview::{self, state_of, Overview, SessionState};
use crate::sessions::registry::configured_hosts;
use crate::tracker::api::{non_empty, ApiResult, TrackerError};

const CACHE_TTL: Duration = Duration::from_secs(15);
const TEXT_LIMIT: usize = 400;
pub const PREVIEW_CHARS: usize = 400;

type Cached = Mutex<Option<(Instant, Value)>>;

fn cache() -> &'static Cached {
    static CACHE: OnceLock<Cached> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

async fn invalidate() {
    *cache().lock().await = None;
}

fn state_name(state: SessionState) -> &'static str {
    match state {
        SessionState::Working => "working",
        SessionState::Question => "question",
        SessionState::Blocked => "blocked",
        SessionState::Idle => "idle",
    }
}

fn state_rank(state: SessionState) -> u8 {
    match state {
        SessionState::Question => 0,
        SessionState::Blocked => 1,
        SessionState::Working => 2,
        SessionState::Idle => 3,
    }
}

fn preview(text: &str) -> String {
    text.chars().take(PREVIEW_CHARS).collect()
}

fn parse_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|at| at.with_timezone(&Utc))
}

/// Shapes a collected overview into the board JSON; pure so it can be tested without ssh.
pub fn shape(overview: &Overview, now: DateTime<Utc>) -> Value {
    let mut machines = Vec::new();
    let mut rows = Vec::new();

    for (machine, snapshot) in &overview.snapshots {
        match snapshot {
            Err(error) => machines.push(json!({
                "machine": machine,
                "ok": false,
                "error": error,
                "session_count": 0,
            })),
            Ok(snapshot) => {
                machines.push(json!({
                    "machine": machine,
                    "ok": true,
                    "error": null,
                    "session_count": snapshot.sessions.len(),
                }));
                for live in &snapshot.sessions {
                    let listing = overview.list.sessions.iter().find(|s| {
                        s.machine == *machine
                            && (s.session_id == live.session_id
                                || s.raw.get("durableSessionId").and_then(Value::as_str)
                                    == Some(live.session_id.as_str()))
                    });
                    let name = live
                        .name
                        .clone()
                        .filter(|name| !name.trim().is_empty())
                        .or_else(|| listing.and_then(|s| s.name.clone()));
                    let cwd = live
                        .cwd
                        .clone()
                        .or_else(|| listing.and_then(|s| s.cwd.clone()));
                    let updated_at = live.updated_at.clone().or_else(|| {
                        listing.and_then(|s| s.updated_at.clone().or(s.last_activity.clone()))
                    });
                    let state = state_of(live);
                    let sort_key = (state_rank(state), parse_time(updated_at.as_deref()));
                    rows.push((
                        sort_key,
                        json!({
                            "machine": machine,
                            "session_id": live.session_id,
                            "name": name,
                            "cwd": cwd,
                            "state": state_name(state),
                            "streaming": live.streaming,
                            "compacting": live.compacting,
                            "message_count": live.message_count,
                            "pending_messages": live.pending_messages,
                            "last_user": preview(&live.last_user),
                            "last_assistant": preview(&live.last_assistant),
                            "questions": live.questions,
                            "updated_at": updated_at,
                        }),
                    ));
                }
            }
        }
    }

    for error in &overview.list.errors {
        if !overview.snapshots.iter().any(|(m, _)| *m == error.machine) {
            machines.push(json!({
                "machine": error.machine,
                "ok": false,
                "error": error.message,
                "session_count": 0,
            }));
        }
    }

    // State order first, then most recent activity first; unknown times sink.
    rows.sort_by(|(a, _), (b, _)| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

    let count = |name: &str| rows.iter().filter(|(_, row)| row["state"] == name).count();
    let counts = json!({
        "working": count("working"),
        "question": count("question"),
        "blocked": count("blocked"),
        "idle": count("idle"),
        "total": rows.len(),
    });

    json!({
        "collected_at": now.to_rfc3339(),
        "machines": machines,
        "sessions": rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>(),
        "counts": counts,
    })
}

fn control_error(error: ControlError) -> TrackerError {
    match error {
        ControlError::InvalidTarget
        | ControlError::InvalidText
        | ControlError::Ambiguous(_)
        | ControlError::Unsupported(_) => TrackerError::bad_request(error.to_string()),
        other => OmonError::ToolExecution(other.to_string()).into(),
    }
}

fn control() -> ApiResult<SessionControl> {
    SessionControl::from_environment().map_err(control_error)
}

#[derive(Debug, Default, Deserialize)]
pub struct BoardQuery {
    #[serde(default)]
    refresh: Option<String>,
}

pub async fn list(Query(query): Query<BoardQuery>) -> ApiResult<Json<Value>> {
    let refresh = matches!(query.refresh.as_deref(), Some("1" | "true"));
    let mut cached = cache().lock().await;
    if !refresh {
        if let Some((at, value)) = cached.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return Ok(Json(value.clone()));
            }
        }
    }
    let control = control()?;
    let collected = overview::collect(&control, TEXT_LIMIT).await;
    let value = shape(&collected, Utc::now());
    *cached = Some((Instant::now(), value.clone()));
    Ok(Json(value))
}

fn target(machine: &str, session_id: &str) -> ApiResult<Target> {
    if machine != "local" {
        let hosts =
            configured_hosts().map_err(|error| control_error(ControlError::Registry(error)))?;
        if !hosts.iter().any(|host| host == machine) {
            return Err(TrackerError::not_found("machine"));
        }
    }
    format!("{machine}:{session_id}")
        .parse()
        .map_err(control_error)
}

#[derive(Debug, Deserialize)]
pub struct SendBody {
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct AnswerBody {
    text: String,
    #[serde(default)]
    request: Option<String>,
}

pub async fn send(
    Path((machine, session_id)): Path<(String, String)>,
    Json(body): Json<SendBody>,
) -> ApiResult<Json<Value>> {
    let target = target(&machine, &session_id)?;
    let text = non_empty(&body.text, "text")?;
    let result = control()?
        .send(&target, &text, SendMode::Auto, uuid::Uuid::new_v4())
        .await;
    invalidate().await;
    Ok(Json(
        json!({"ok": true, "result": result.map_err(control_error)?}),
    ))
}

pub async fn interrupt(
    Path((machine, session_id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let target = target(&machine, &session_id)?;
    let result = control()?.interrupt(&target).await;
    invalidate().await;
    Ok(Json(
        json!({"ok": true, "result": result.map_err(control_error)?}),
    ))
}

pub async fn answer(
    Path((machine, session_id)): Path<(String, String)>,
    Json(body): Json<AnswerBody>,
) -> ApiResult<Json<Value>> {
    let target = target(&machine, &session_id)?;
    let text = non_empty(&body.text, "text")?;
    let request = body.request.as_deref().filter(|r| !r.trim().is_empty());
    let result = control()?.answer(&target, &text, request).await;
    invalidate().await;
    Ok(Json(
        json!({"ok": true, "result": result.map_err(control_error)?}),
    ))
}

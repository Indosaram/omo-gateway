use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::{OmonError, Result};
use crate::sessions::control::{ControlError, SendMode, SessionControl, Target};
use crate::sessions::overview::{self, home_relative, short_id, state_of, Overview, SessionState};
use crate::sessions::probe::{LiveSession, MachineSnapshot};
use crate::sessions::registry::configured_hosts;
use crate::tracker::api::{non_empty, ApiResult, TrackerError};
use crate::tracker::lanes::deterministic_avatar_color;
use crate::tracker::model::{SessionDetail, SessionSummary};
use crate::tracker::seed;

const CACHE_TTL: Duration = Duration::from_secs(15);
/// Probe text limit; the issue detail shows last messages up to this many characters.
pub const TEXT_LIMIT: usize = 4000;
pub const PREVIEW_CHARS: usize = 400;
/// Set to "0" to skip mirroring live sessions into tracker issues (collection still serves the board).
pub const SYNC_ENV: &str = "OMON_TRACKER_SESSION_SYNC";

struct Collected {
    /// When the collection that produced this entry started; later starts win a tie.
    started: Instant,
    at: Instant,
    overview: Arc<Overview>,
    board: Value,
}

#[derive(Default)]
struct CacheSlot {
    collected: Option<Collected>,
    /// Bumped by `invalidate`; a collection that started under an older generation is
    /// returned to its caller but never stored, so it cannot undo the invalidation.
    generation: u64,
}

type Cached = Mutex<CacheSlot>;

fn cache() -> &'static Cached {
    static CACHE: OnceLock<Cached> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(CacheSlot::default()))
}

/// Latest known live sessions keyed by session_ref; machines that failed keep their previous entries.
fn live_map() -> &'static RwLock<HashMap<String, LiveSession>> {
    static LIVE: OnceLock<RwLock<HashMap<String, LiveSession>>> = OnceLock::new();
    LIVE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn sync_lock() -> &'static Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(|| Arc::new(Mutex::new(())))
}

pub fn session_ref(machine: &str, session_id: &str) -> String {
    format!("{machine}:{session_id}")
}

fn answered(overview: &Overview) -> impl Iterator<Item = (&String, &MachineSnapshot)> {
    overview
        .snapshots
        .iter()
        .filter_map(|(machine, snapshot)| snapshot.as_ref().ok().map(|s| (machine, s)))
}

fn remember(overview: &Overview) {
    let ok: HashSet<&str> = answered(overview).map(|(m, _)| m.as_str()).collect();
    let mut map = live_map()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    map.retain(|reference, _| {
        reference
            .split_once(':')
            .is_some_and(|(machine, _)| !ok.contains(machine))
    });
    for (machine, snapshot) in answered(overview) {
        for live in &snapshot.sessions {
            map.insert(session_ref(machine, &live.session_id), live.clone());
        }
    }
}

fn live_session(reference: &str) -> Option<LiveSession> {
    live_map()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(reference)
        .cloned()
}

async fn current(refresh: bool) -> ApiResult<(Arc<Overview>, Value)> {
    let (generation, started) = {
        let slot = cache().lock().await;
        if !refresh {
            if let Some(collected) = slot.collected.as_ref() {
                if collected.at.elapsed() < CACHE_TTL {
                    return Ok((collected.overview.clone(), collected.board.clone()));
                }
            }
        }
        (slot.generation, Instant::now())
    };
    // The ssh collection runs without the cache lock so send/answer/interrupt never wait on it.
    let control = control()?;
    let overview = Arc::new(overview::collect(&control, TEXT_LIMIT).await);
    let board = shape(&overview, Utc::now());
    store(generation, started, overview.clone(), board.clone()).await;
    Ok((overview, board))
}

/// Publishes a collection unless `invalidate` ran after it started or a collection that began
/// later already published. The cache entry and the live session map are written under the one
/// cache lock, so an accepted-but-slow collection can never rewrite `live_map` after a newer one
/// published. Lock order is always cache -> live_map; nothing takes them the other way round.
async fn store(generation: u64, started: Instant, overview: Arc<Overview>, board: Value) {
    let mut slot = cache().lock().await;
    if slot.generation != generation {
        return;
    }
    if slot
        .collected
        .as_ref()
        .is_some_and(|existing| existing.started > started)
    {
        return;
    }
    slot.collected = Some(Collected {
        started,
        at: Instant::now(),
        overview: overview.clone(),
        board,
    });
    // Synchronous and bounded (an in-memory map swap, no ssh); still under the lock so both
    // publications commit or neither does.
    remember(&overview);
}

async fn invalidate() {
    let mut slot = cache().lock().await;
    slot.collected = None;
    slot.generation = slot.generation.wrapping_add(1);
}

const TOMBSTONE_PREFIX: &str = "session_tombstone:";

/// Marks a session issue as deleted so sync does not re-create it while the session lives.
/// Takes a connection so the tombstone can share the delete's transaction.
pub async fn tombstone(conn: &mut sqlx::SqliteConnection, reference: &str) -> Result<()> {
    sqlx::query("INSERT INTO tracker_meta (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(format!("{TOMBSTONE_PREFIX}{reference}"))
        .bind(seed::now_utc())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn is_tombstoned(conn: &mut sqlx::SqliteConnection, reference: &str) -> Result<bool> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM tracker_meta WHERE key = ?")
        .bind(format!("{TOMBSTONE_PREFIX}{reference}"))
        .fetch_optional(&mut *conn)
        .await?;
    Ok(found.is_some())
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

struct Resolved {
    name: Option<String>,
    cwd: Option<String>,
    updated_at: Option<String>,
}

/// Fills gaps in a probed session from the registry listing of the same machine.
fn resolve(overview: &Overview, machine: &str, live: &LiveSession) -> Resolved {
    let listing = overview.list.sessions.iter().find(|s| {
        s.machine == machine
            && (s.session_id == live.session_id
                || s.raw.get("durableSessionId").and_then(Value::as_str)
                    == Some(live.session_id.as_str()))
    });
    Resolved {
        name: live
            .name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| listing.and_then(|s| s.name.clone())),
        cwd: live
            .cwd
            .clone()
            .or_else(|| listing.and_then(|s| s.cwd.clone())),
        updated_at: live
            .updated_at
            .clone()
            .or_else(|| listing.and_then(|s| s.updated_at.clone().or(s.last_activity.clone()))),
    }
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
                    let Resolved {
                        name,
                        cwd,
                        updated_at,
                    } = resolve(overview, machine, live);
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
    let (_, board) = current(refresh).await?;
    Ok(Json(board))
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

/// Issue title: the session name, unless it is empty or a transcript path, then the cwd
/// basename, then the short session id.
pub fn session_title(name: Option<&str>, cwd: Option<&str>, session_id: &str) -> String {
    let name = name.map(str::trim).unwrap_or_default();
    if !name.is_empty() && !name.starts_with('/') {
        return name.to_string();
    }
    cwd.and_then(|cwd| cwd.trim_end_matches('/').rsplit('/').next())
        .filter(|base| !base.is_empty())
        .map_or_else(|| short_id(session_id).to_string(), str::to_string)
}

fn session_description(
    machine: &str,
    cwd: Option<&str>,
    session_id: &str,
    last_user: &str,
    last_assistant: &str,
) -> String {
    let mut header = format!("**Machine** {machine}");
    if let Some(cwd) = cwd {
        header.push_str(&format!(" · **Directory** `{}`", home_relative(cwd)));
    }
    header.push_str(&format!(" · **Session** `{session_id}`"));
    let or_none = |text: &str| {
        if text.trim().is_empty() {
            "_None_".to_string()
        } else {
            text.to_string()
        }
    };
    format!(
        "{header}\n\n### Last user message\n{}\n\n### Last assistant message\n{}",
        or_none(last_user),
        or_none(last_assistant)
    )
}

fn build_summary(
    reference: &str,
    state: Option<&str>,
    live: Option<&LiveSession>,
) -> Option<SessionSummary> {
    let (machine, session_id) = reference.split_once(':')?;
    let state = state
        .map(str::to_string)
        .or_else(|| live.map(|l| state_name(state_of(l)).to_string()))
        .unwrap_or_else(|| "ended".to_string());
    Some(SessionSummary {
        machine: machine.to_string(),
        session_id: session_id.to_string(),
        state,
        streaming: live.is_some_and(|l| l.streaming),
        message_count: live.map_or(0, |l| l.message_count),
        pending_messages: live.map_or(0, |l| l.pending_messages),
        cwd: live.and_then(|l| l.cwd.clone()),
        updated_at: live.and_then(|l| l.updated_at.clone()),
        questions: live.map(|l| l.questions.clone()).unwrap_or_default(),
    })
}

/// Summary for an SES issue; `state` is the session_state of the issue's workflow state.
pub fn session_summary(reference: &str, state: Option<&str>) -> Option<SessionSummary> {
    build_summary(reference, state, live_session(reference).as_ref())
}

pub fn session_detail(summary: &SessionSummary) -> SessionDetail {
    let live = live_session(&session_ref(&summary.machine, &summary.session_id));
    SessionDetail {
        summary: summary.clone(),
        last_user: live
            .as_ref()
            .map(|l| l.last_user.clone())
            .unwrap_or_default(),
        last_assistant: live.map(|l| l.last_assistant).unwrap_or_default(),
    }
}

/// How long a tracker request waits for a stale collection before answering from the DB;
/// the collection keeps running in the background and the next request sees its result.
const SYNC_WAIT: Duration = Duration::from_secs(3);

/// Mirrors live sessions into SES issues. Never fails the caller: collection or sync errors
/// are logged and skipped, and a slow probe never holds the request longer than SYNC_WAIT.
pub async fn sync_issues(pool: &SqlitePool) {
    if std::env::var(SYNC_ENV).as_deref() == Ok("0") {
        return;
    }
    // A sync already in flight will publish its result; later requests never queue behind it.
    let Ok(guard) = sync_lock().clone().try_lock_owned() else {
        return;
    };
    let pool = pool.clone();
    let task = tokio::spawn(async move {
        let _guard = guard;
        sync_locked(&pool).await;
    });
    match tokio::time::timeout(SYNC_WAIT, task).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(%error, "tracker session sync task failed"),
        Err(_) => tracing::debug!("tracker session sync still running; serving current issues"),
    }
}

async fn sync_now(pool: &SqlitePool) {
    let _guard = sync_lock().lock().await;
    sync_locked(pool).await;
}

async fn sync_locked(pool: &SqlitePool) {
    let overview = match current(false).await {
        Ok((overview, _)) => overview,
        Err(error) => {
            tracing::warn!(
                ?error,
                "session collection failed; tracker session sync skipped"
            );
            return;
        }
    };
    if let Err(error) = sync_from_overview(pool, &overview, Utc::now()).await {
        tracing::warn!(%error, "tracker session sync failed");
    }
}

struct SyncContext {
    team_id: String,
    /// Identifier prefix: the Sessions team's key (normally "SES").
    team_key: String,
    /// session_state -> (state id, state type)
    states: HashMap<String, (String, String)>,
    /// state id -> session_state
    by_state_id: HashMap<String, String>,
    me: Option<String>,
}

struct Existing {
    id: String,
    identifier: String,
    title: String,
    description: String,
    state_id: String,
    assignee_id: Option<String>,
    started_at: Option<String>,
}

async fn ensure_machine_user(pool: &SqlitePool, machine: &str) -> Result<String> {
    let model = format!("omo@{machine}");
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_users WHERE kind = 'agent' AND model = ? LIMIT 1",
    )
    .bind(&model)
    .fetch_optional(pool)
    .await?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tracker_users (id, name, display_name, email, avatar_color, kind, model, is_me, created_at)
         VALUES (?, ?, ?, NULL, ?, 'agent', ?, 0, ?)",
    )
    .bind(&id)
    .bind(machine)
    .bind(machine)
    .bind(deterministic_avatar_color(&model))
    .bind(&model)
    .bind(seed::now_utc())
    .execute(pool)
    .await?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
async fn record_state(
    pool: &SqlitePool,
    ctx: &SyncContext,
    issue_id: &str,
    identifier: &str,
    title: &str,
    actor: Option<&str>,
    from: Option<&str>,
    to: &str,
    now: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
         VALUES (?, NULL, 'session_state', ?, ?, ?)",
    )
    .bind(issue_id)
    .bind(from)
    .bind(to)
    .bind(now)
    .execute(pool)
    .await?;
    let body = match to {
        "question" => format!("{identifier} needs an answer: {title}"),
        "blocked" => format!("{identifier} is blocked: {title}"),
        _ => return Ok(()),
    };
    if let Some(me) = ctx.me.as_deref() {
        sqlx::query(
            "INSERT INTO tracker_notifications (id, user_id, issue_id, project_id, kind, actor_id, body, created_at)
             VALUES (?, ?, ?, NULL, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(me)
        .bind(issue_id)
        .bind(format!("session_{to}"))
        .bind(actor)
        .bind(body)
        .bind(now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Moves an existing issue to the state mirroring `target`; returns whether it moved.
async fn move_to(
    pool: &SqlitePool,
    ctx: &SyncContext,
    issue: &Existing,
    target: &str,
    now: &str,
) -> Result<bool> {
    let Some((state_id, state_type)) = ctx.states.get(target) else {
        return Ok(false);
    };
    if *state_id == issue.state_id {
        return Ok(false);
    }
    let (started_at, completed_at) = match state_type.as_str() {
        "started" => (
            issue.started_at.clone().or_else(|| Some(now.to_string())),
            None,
        ),
        "completed" => (issue.started_at.clone(), Some(now.to_string())),
        _ => (issue.started_at.clone(), None),
    };
    sqlx::query(
        "UPDATE tracker_issues
         SET state_id = ?, started_at = ?, completed_at = ?, canceled_at = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(state_id)
    .bind(&started_at)
    .bind(&completed_at)
    .bind(now)
    .bind(&issue.id)
    .execute(pool)
    .await?;
    record_state(
        pool,
        ctx,
        &issue.id,
        &issue.identifier,
        &issue.title,
        issue.assignee_id.as_deref(),
        ctx.by_state_id.get(&issue.state_id).map(String::as_str),
        target,
        now,
    )
    .await?;
    Ok(true)
}

async fn load_existing(pool: &SqlitePool, reference: &str) -> Result<Option<Existing>> {
    let row = sqlx::query(
        "SELECT id, identifier, title, description, state_id, assignee_id, started_at
         FROM tracker_issues WHERE session_ref = ?",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| Existing {
        id: row.get("id"),
        identifier: row.get("identifier"),
        title: row.get("title"),
        description: row.get("description"),
        state_id: row.get("state_id"),
        assignee_id: row.get("assignee_id"),
        started_at: row.get("started_at"),
    }))
}

#[allow(clippy::too_many_arguments)]
async fn upsert(
    pool: &SqlitePool,
    ctx: &SyncContext,
    reference: &str,
    machine: &str,
    title: &str,
    description: &str,
    target: &str,
    now: &str,
) -> Result<()> {
    if let Some(issue) = load_existing(pool, reference).await? {
        let mut changed = move_to(pool, ctx, &issue, target, now).await?;
        if issue.title != title || issue.description != description {
            sqlx::query("UPDATE tracker_issues SET title = ?, description = ? WHERE id = ?")
                .bind(title)
                .bind(description)
                .bind(&issue.id)
                .execute(pool)
                .await?;
            changed = true;
        }
        if changed {
            sqlx::query("UPDATE tracker_issues SET updated_at = ? WHERE id = ?")
                .bind(now)
                .bind(&issue.id)
                .execute(pool)
                .await?;
        }
        return Ok(());
    }

    let Some((state_id, state_type)) = ctx.states.get(target) else {
        return Ok(());
    };
    let agent = ensure_machine_user(pool, machine).await?;
    let id = Uuid::new_v4().to_string();
    let started_at =
        matches!(state_type.as_str(), "started" | "completed").then(|| now.to_string());
    let completed_at = (state_type == "completed").then(|| now.to_string());
    // The tombstone check and the insert share one write transaction, so a delete that runs
    // concurrently either lands before (this insert sees the tombstone) or after (the delete
    // removes the row by session_ref).
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    if is_tombstoned(&mut tx, reference).await? {
        return Ok(());
    }
    let number: i64 = sqlx::query_scalar(
        "UPDATE tracker_teams SET issue_seq = issue_seq + 1 WHERE id = ? RETURNING issue_seq",
    )
    .bind(&ctx.team_id)
    .fetch_one(&mut *tx)
    .await?;
    let identifier = format!("{}-{number}", ctx.team_key);
    sqlx::query(
        "INSERT INTO tracker_issues (id, team_id, number, identifier, title, description, state_id, priority,
            estimate, assignee_id, creator_id, project_id, milestone_id, cycle_id, parent_id, due_date,
            sort_order, lane_key, session_ref, created_at, updated_at, started_at, completed_at, canceled_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?, NULL, NULL, NULL, NULL, NULL, 0.0, NULL, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(&id)
    .bind(&ctx.team_id)
    .bind(number)
    .bind(&identifier)
    .bind(title)
    .bind(description)
    .bind(state_id)
    .bind(&agent)
    .bind(&agent)
    .bind(reference)
    .bind(now)
    .bind(now)
    .bind(&started_at)
    .bind(&completed_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    for user in std::iter::once(agent.as_str()).chain(ctx.me.as_deref()) {
        sqlx::query(
            "INSERT INTO tracker_subscribers (issue_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(&id)
        .bind(user)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
         VALUES (?, ?, 'created', NULL, NULL, ?)",
    )
    .bind(&id)
    .bind(&agent)
    .bind(now)
    .execute(pool)
    .await?;
    record_state(
        pool,
        ctx,
        &id,
        &identifier,
        title,
        Some(agent.as_str()),
        None,
        target,
        now,
    )
    .await
}

/// Mirrors one collected overview into SES issues. Machines whose snapshot failed (or answered
/// only partially) keep their issues untouched; sessions gone from a machine that answered
/// move to Ended; issues are never deleted.
pub async fn sync_from_overview(
    pool: &SqlitePool,
    overview: &Overview,
    now: DateTime<Utc>,
) -> Result<()> {
    remember(overview);
    let team_id = seed::ensure_sessions_team(pool).await?;
    let now = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut states = HashMap::new();
    let mut by_state_id = HashMap::new();
    for row in sqlx::query(
        "SELECT id, type, session_state FROM tracker_states WHERE team_id = ? AND session_state IS NOT NULL",
    )
    .bind(&team_id)
    .fetch_all(pool)
    .await?
    {
        let id: String = row.get("id");
        let session_state: String = row.get("session_state");
        by_state_id.insert(id.clone(), session_state.clone());
        states.insert(session_state, (id, row.get::<String, _>("type")));
    }
    let me: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_users WHERE is_me = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;
    let team_key: String = sqlx::query_scalar("SELECT key FROM tracker_teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(pool)
        .await?;
    let ctx = SyncContext {
        team_id,
        team_key,
        states,
        by_state_id,
        me,
    };

    for (machine, snapshot) in answered(overview) {
        let mut present = HashSet::new();
        for live in &snapshot.sessions {
            let reference = session_ref(machine, &live.session_id);
            let resolved = resolve(overview, machine, live);
            let title = session_title(
                resolved.name.as_deref(),
                resolved.cwd.as_deref(),
                &live.session_id,
            );
            let description = session_description(
                machine,
                resolved.cwd.as_deref(),
                &live.session_id,
                &live.last_user,
                &live.last_assistant,
            );
            let target = state_name(state_of(live));
            upsert(
                pool,
                &ctx,
                &reference,
                machine,
                &title,
                &description,
                target,
                &now,
            )
            .await?;
            present.insert(reference);
        }

        // A partial answer cannot tell a finished session from an unreachable endpoint.
        if !snapshot.errors.is_empty() {
            continue;
        }
        // Tombstones only need to outlive their session.
        let machine_prefix = format!("{TOMBSTONE_PREFIX}{machine}:");
        let tombstones: Vec<String> =
            sqlx::query_scalar("SELECT key FROM tracker_meta WHERE substr(key, 1, ?) = ?")
                .bind(machine_prefix.chars().count() as i64)
                .bind(&machine_prefix)
                .fetch_all(pool)
                .await?;
        for key in tombstones {
            let reference = &key[TOMBSTONE_PREFIX.len()..];
            if !present.contains(reference) {
                sqlx::query("DELETE FROM tracker_meta WHERE key = ?")
                    .bind(&key)
                    .execute(pool)
                    .await?;
            }
        }
        let tracked: Vec<String> = sqlx::query_scalar(
            "SELECT session_ref FROM tracker_issues WHERE session_ref IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;
        for reference in tracked {
            let same_machine = reference
                .split_once(':')
                .is_some_and(|(m, _)| m == machine.as_str());
            if !same_machine || present.contains(&reference) {
                continue;
            }
            if let Some(issue) = load_existing(pool, &reference).await? {
                move_to(pool, &ctx, &issue, "ended", &now).await?;
            }
        }
    }
    Ok(())
}

/// Handles a human state move on an SES issue: moving to Ended while the session streams
/// interrupts it; every other move is rejected because sync owns the state.
pub async fn manual_state_move(
    pool: &SqlitePool,
    reference: &str,
    target_state_id: &str,
) -> ApiResult<()> {
    let target: Option<Option<String>> =
        sqlx::query_scalar("SELECT session_state FROM tracker_states WHERE id = ?")
            .bind(target_state_id)
            .fetch_optional(pool)
            .await?;
    let streaming = live_session(reference).is_some_and(|live| live.streaming);
    if target.flatten().as_deref() == Some("ended") && streaming {
        let (machine, session_id) = reference
            .split_once(':')
            .ok_or_else(|| TrackerError::bad_request("invalid session reference"))?;
        let target = self::target(machine, session_id)?;
        control()?.interrupt(&target).await.map_err(control_error)?;
        invalidate().await;
        sync_now(pool).await;
        return Ok(());
    }
    Err(TrackerError::conflict(
        "session issues follow the live session state; only Ended on a streaming session (interrupt) is allowed",
        "session_state_managed",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::registry::SessionList;

    fn overview_with_session(machine: &str, session_id: &str) -> Arc<Overview> {
        Arc::new(Overview {
            list: SessionList {
                sessions: Vec::new(),
                errors: Vec::new(),
                stale: Vec::new(),
                exit_code: 0,
            },
            snapshots: vec![(
                machine.to_string(),
                Ok(MachineSnapshot {
                    machine: machine.to_string(),
                    sessions: vec![LiveSession {
                        machine: machine.to_string(),
                        session_id: session_id.to_string(),
                        routing_id: session_id.to_string(),
                        ..LiveSession::default()
                    }],
                    errors: Vec::new(),
                }),
            )],
        })
    }

    /// The cache lock is released before `collect` runs, so a collection can finish after
    /// `invalidate`, and two collections of one generation can finish out of order. Only an
    /// accepted collection may reach the cache or the live session map.
    ///
    /// Both phases share the process-wide cache and live map, so they run in one test.
    #[tokio::test]
    async fn stale_and_out_of_order_collections_never_publish() {
        // Phase 1: a collection that started before invalidate.
        let stale_generation = cache().lock().await.generation;
        let stale_start = Instant::now();
        invalidate().await;
        store(
            stale_generation,
            stale_start,
            overview_with_session("stale-machine", "s1"),
            json!({"which": "stale"}),
        )
        .await;
        assert!(
            cache().lock().await.collected.is_none(),
            "a collection started before invalidate must not be stored"
        );
        assert!(
            live_session("stale-machine:s1").is_none(),
            "a rejected collection must not publish live session state either"
        );

        // Phase 2: same generation, the older collection finishes last.
        let generation = cache().lock().await.generation;
        let older_start = Instant::now();
        let newer_start = older_start + Duration::from_millis(10);
        store(
            generation,
            newer_start,
            overview_with_session("newer-machine", "s1"),
            json!({"which": "newer"}),
        )
        .await;
        store(
            generation,
            older_start,
            overview_with_session("older-machine", "s1"),
            json!({"which": "older"}),
        )
        .await;

        let board = cache()
            .lock()
            .await
            .collected
            .as_ref()
            .map(|collected| collected.board.clone())
            .expect("the newer collection stays cached");
        assert_eq!(board, json!({"which": "newer"}));
        assert!(live_session("newer-machine:s1").is_some());
        assert!(
            live_session("older-machine:s1").is_none(),
            "a superseded collection must not republish live session state"
        );
        invalidate().await;
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::Result;
use crate::OmonError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Lane {
    pub key: String,
    pub title: String,
    pub lead_session_key: String,
    pub thread_channel_id: Option<String>,
    pub parent_channel_id: Option<String>,
    pub status_message_id: Option<String>,
    pub cwd: String,
    pub brief_path: String,
    pub session_key: String,
    pub omo_thread_id: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub done_signals: String,
    pub claim_evidence: Option<String>,
    pub claim_at: Option<DateTime<Utc>>,
    pub objection: Option<String>,
    pub turns_run: i64,
    pub last_turn_outcome: Option<String>,
    pub last_report: Option<String>,
    pub last_activity_at: DateTime<Utc>,
    pub stale_posted_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Lane {
    pub fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        lead_session_key: impl Into<String>,
        cwd: impl Into<String>,
        brief_path: impl Into<String>,
        session_key: impl Into<String>,
        done_signals: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            title: title.into(),
            lead_session_key: lead_session_key.into(),
            thread_channel_id: None,
            parent_channel_id: None,
            status_message_id: None,
            cwd: cwd.into(),
            brief_path: brief_path.into(),
            session_key: session_key.into(),
            omo_thread_id: None,
            model: None,
            status: "queued".to_string(),
            done_signals: done_signals.into(),
            claim_evidence: None,
            claim_at: None,
            objection: None,
            turns_run: 0,
            last_turn_outcome: None,
            last_report: None,
            last_activity_at: now,
            stale_posted_at: None,
            created_at: Some(now),
            updated_at: Some(now),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct LaneHistoryEntry {
    pub id: i64,
    pub lane_key: String,
    pub at: DateTime<Utc>,
    pub from_status: Option<String>,
    pub to_status: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct LaneEvent {
    pub id: i64,
    pub lane_key: Option<String>,
    pub lead_session_key: String,
    pub kind: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct LaneTranscriptEntry {
    pub id: i64,
    pub lane_key: String,
    pub at: DateTime<Utc>,
    pub role: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneFieldsUpdate {
    pub title: Option<String>,
    pub lead_session_key: Option<String>,
    pub thread_channel_id: Option<String>,
    pub parent_channel_id: Option<String>,
    pub status_message_id: Option<String>,
    pub cwd: Option<String>,
    pub brief_path: Option<String>,
    pub session_key: Option<String>,
    pub omo_thread_id: Option<String>,
    pub model: Option<String>,
    pub status: Option<String>,
    pub done_signals: Option<String>,
    pub claim_evidence: Option<String>,
    pub claim_at: Option<DateTime<Utc>>,
    pub objection: Option<String>,
    pub turns_run: Option<i64>,
    pub last_turn_outcome: Option<String>,
    pub last_report: Option<String>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub stale_posted_at: Option<DateTime<Utc>>,

    pub clear_thread_channel_id: bool,
    pub clear_parent_channel_id: bool,
    pub clear_status_message_id: bool,
    pub clear_omo_thread_id: bool,
    pub clear_model: bool,
    pub clear_claim_evidence: bool,
    pub clear_claim_at: bool,
    pub clear_objection: bool,
    pub clear_last_turn_outcome: bool,
    pub clear_last_report: bool,
    pub clear_stale_posted_at: bool,
}

pub type LaneUpdate = LaneFieldsUpdate;

#[derive(Debug, sqlx::FromRow)]
pub struct CloseIntent {
    pub id: i64,
    pub lane_key: String,
    pub generation: i64,
    pub original_status: String,
    pub force: bool,
    pub note: Option<String>,
    pub state: String,
    pub phase: i64,
    pub attempt: i64,
    pub previous_cwd: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OwnedWorktree {
    pub id: i64,
    pub lane_key: String,
    pub path: String,
    pub role: String,
    pub repo: String,
    pub common_dir: String,
    pub git_dir: String,
    pub identity: String,
    pub branch: String,
    pub state: String,
    pub removal_intent: Option<i64>,
    pub removal_receipt: Option<String>,
}

pub async fn request_close(
    pool: &SqlitePool,
    key: &str,
    force: bool,
    note: Option<&str>,
) -> Result<i64> {
    if force != note.is_some_and(|note| note.starts_with("forced:")) {
        return Err(OmonError::Config(
            "--force requires --note beginning with forced:; forced note requires --force".into(),
        ));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let lane: Lane = sqlx::query_as("SELECT * FROM lanes WHERE key = ?")
        .bind(key)
        .fetch_one(&mut *tx)
        .await?;
    if lane.status == "closed" || (!force && lane.status != "verified_done") {
        return Err(OmonError::Config(format!(
            "IllegalTransition: {} -> closed",
            lane.status
        )));
    }
    let generation: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(id),0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'",
    )
    .bind(key)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO lane_close_intents(lane_key,generation,original_status,force,note,state,previous_cwd) VALUES (?,?,?,?,?,'pending',?) ON CONFLICT(lane_key,generation) DO UPDATE SET state='pending', attempt=attempt+1, error=NULL, updated_at=CURRENT_TIMESTAMP WHERE lane_close_intents.state='failed' AND lane_close_intents.original_status=excluded.original_status AND lane_close_intents.force=excluded.force")
        .bind(key).bind(generation).bind(&lane.status).bind(force).bind(note).bind(&lane.cwd).execute(&mut *tx).await?;
    let id =
        sqlx::query_scalar("SELECT id FROM lane_close_intents WHERE lane_key=? AND generation=?")
            .bind(key)
            .bind(generation)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn bind_lane_thread(pool: &SqlitePool, key: &str, bot: &str) -> Result<()> {
    let changed = sqlx::query("INSERT INTO lane_thread_bindings(lane_key,bot_id) VALUES (?,?) ON CONFLICT(lane_key) DO UPDATE SET bot_id=excluded.bot_id WHERE lane_thread_bindings.bot_id=excluded.bot_id")
        .bind(key).bind(bot).execute(pool).await?;
    if changed.rows_affected() != 1 {
        return Err(OmonError::Config("lane bot binding mismatch".into()));
    }
    Ok(())
}

pub async fn record_owner_reply(
    pool: &SqlitePool,
    thread: &str,
    bot: &str,
    author: &str,
    owner: &str,
    message: &str,
    text: &str,
) -> Result<bool> {
    if author != owner || text.trim().is_empty() || text.trim().eq_ignore_ascii_case("/stop") {
        return Ok(false);
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let duplicate: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lane_owner_replies WHERE message_id=?)")
            .bind(message)
            .fetch_one(&mut *tx)
            .await?;
    if duplicate {
        return Ok(true);
    }
    let key: Option<String> = sqlx::query_scalar("SELECT l.key FROM lanes l JOIN lane_thread_bindings b ON b.lane_key=l.key WHERE l.thread_channel_id=? AND b.bot_id=? AND (l.status IN ('closed','verified_done') OR EXISTS(SELECT 1 FROM lane_owner_replies r WHERE r.lane_key=l.key AND r.state='pending'))")
        .bind(thread).bind(bot).fetch_optional(&mut *tx).await?;
    let Some(key) = key else {
        return Ok(false);
    };
    sqlx::query("UPDATE lane_owner_replies SET error=NULL WHERE lane_key=? AND state='pending'")
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO lane_owner_replies(message_id,lane_key,bot_id,author_id,text,generation) SELECT ?,?,?,?,?,COALESCE((SELECT MIN(generation) FROM lane_owner_replies WHERE lane_key=? AND state='pending'),COALESCE(MAX(id),0)) FROM lane_history WHERE lane_key=? AND reason='reopened'")
        .bind(message).bind(&key).bind(bot).bind(author).bind(text).bind(&key).bind(&key).execute(&mut *tx).await?;
    tx.commit().await?;
    if get_lane(pool, &key)
        .await?
        .is_some_and(|lane| lane.status == "verified_done")
    {
        request_close(pool, &key, false, None).await?;
    }
    Ok(true)
}

pub trait IntoReason<'a> {
    fn into_reason(self) -> Option<&'a str>;
}

impl<'a> IntoReason<'a> for Option<&'a str> {
    fn into_reason(self) -> Option<&'a str> {
        self
    }
}

impl<'a> IntoReason<'a> for &'a str {
    fn into_reason(self) -> Option<&'a str> {
        Some(self)
    }
}

impl<'a> IntoReason<'a> for &'a String {
    fn into_reason(self) -> Option<&'a str> {
        Some(self.as_str())
    }
}

impl<'a> IntoReason<'a> for () {
    fn into_reason(self) -> Option<&'a str> {
        None
    }
}

pub async fn insert_lane(pool: &SqlitePool, lane: &Lane) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO lanes (
            key, title, lead_session_key, thread_channel_id, parent_channel_id,
            status_message_id, cwd, brief_path, session_key, omo_thread_id,
            model, status, done_signals, claim_evidence, claim_at,
            objection, turns_run, last_turn_outcome, last_report,
            last_activity_at, stale_posted_at, created_at, updated_at
        ) VALUES (
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?,
            COALESCE(?, CURRENT_TIMESTAMP),
            COALESCE(?, CURRENT_TIMESTAMP)
        )
        "#,
    )
    .bind(&lane.key)
    .bind(&lane.title)
    .bind(&lane.lead_session_key)
    .bind(&lane.thread_channel_id)
    .bind(&lane.parent_channel_id)
    .bind(&lane.status_message_id)
    .bind(&lane.cwd)
    .bind(&lane.brief_path)
    .bind(&lane.session_key)
    .bind(&lane.omo_thread_id)
    .bind(&lane.model)
    .bind(&lane.status)
    .bind(&lane.done_signals)
    .bind(&lane.claim_evidence)
    .bind(lane.claim_at)
    .bind(&lane.objection)
    .bind(lane.turns_run)
    .bind(&lane.last_turn_outcome)
    .bind(&lane.last_report)
    .bind(lane.last_activity_at)
    .bind(lane.stale_posted_at)
    .bind(lane.created_at)
    .bind(lane.updated_at)
    .execute(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(())
}

pub async fn get_lane(pool: &SqlitePool, key: &str) -> Result<Option<Lane>> {
    let lane = sqlx::query_as::<_, Lane>(
        r#"
        SELECT
            key, title, lead_session_key, thread_channel_id, parent_channel_id,
            status_message_id, cwd, brief_path, session_key, omo_thread_id,
            model, status, done_signals, claim_evidence, claim_at,
            objection, turns_run, last_turn_outcome, last_report,
            last_activity_at, stale_posted_at, created_at, updated_at
        FROM lanes
        WHERE key = ?
        "#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(lane)
}

pub async fn get_lane_by_thread(
    pool: &SqlitePool,
    thread_channel_id: &str,
) -> Result<Option<Lane>> {
    let lane = sqlx::query_as::<_, Lane>(
        r#"
        SELECT
            key, title, lead_session_key, thread_channel_id, parent_channel_id,
            status_message_id, cwd, brief_path, session_key, omo_thread_id,
            model, status, done_signals, claim_evidence, claim_at,
            objection, turns_run, last_turn_outcome, last_report,
            last_activity_at, stale_posted_at, created_at, updated_at
        FROM lanes
        WHERE thread_channel_id = ?
        "#,
    )
    .bind(thread_channel_id)
    .fetch_optional(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(lane)
}

pub async fn list_lanes<S: AsRef<str>>(pool: &SqlitePool, statuses: &[S]) -> Result<Vec<Lane>> {
    if statuses.is_empty() {
        let rows = sqlx::query_as::<_, Lane>(
            r#"
            SELECT
                key, title, lead_session_key, thread_channel_id, parent_channel_id,
                status_message_id, cwd, brief_path, session_key, omo_thread_id,
                model, status, done_signals, claim_evidence, claim_at,
                objection, turns_run, last_turn_outcome, last_report,
                last_activity_at, stale_posted_at, created_at, updated_at
            FROM lanes
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(OmonError::from)?;
        Ok(rows)
    } else {
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            r#"
            SELECT
                key, title, lead_session_key, thread_channel_id, parent_channel_id,
                status_message_id, cwd, brief_path, session_key, omo_thread_id,
                model, status, done_signals, claim_evidence, claim_at,
                objection, turns_run, last_turn_outcome, last_report,
                last_activity_at, stale_posted_at, created_at, updated_at
            FROM lanes
            WHERE status IN ({placeholders})
            ORDER BY created_at DESC
            "#
        );
        let mut q = sqlx::query_as::<_, Lane>(&sql);
        for s in statuses {
            q = q.bind(s.as_ref());
        }
        let rows = q.fetch_all(pool).await.map_err(OmonError::from)?;
        Ok(rows)
    }
}

pub async fn transition_lane<'a, S: AsRef<str>, R: IntoReason<'a>>(
    pool: &SqlitePool,
    key: &str,
    from_set: &[S],
    to: &str,
    reason: R,
) -> Result<bool> {
    if from_set.is_empty() {
        return Ok(false);
    }

    let reason = reason.into_reason();
    let mut tx = pool.begin().await?;

    let row: Option<(String,)> = sqlx::query_as("SELECT status FROM lanes WHERE key = ?")
        .bind(key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(OmonError::from)?;

    let from_status = match row {
        Some((s,)) => s,
        None => return Ok(false),
    };

    let matches = from_set.iter().any(|s| s.as_ref() == from_status);
    if !matches {
        return Ok(false);
    }

    let placeholders = vec!["?"; from_set.len()].join(", ");
    let sql = format!(
        "UPDATE lanes SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE key = ? AND status IN ({placeholders})"
    );
    let mut q = sqlx::query(&sql).bind(to).bind(key);
    for s in from_set {
        q = q.bind(s.as_ref());
    }

    let res = q.execute(&mut *tx).await.map_err(OmonError::from)?;
    if res.rows_affected() == 0 {
        return Ok(false);
    }

    let now = Utc::now();
    sqlx::query(
        "INSERT INTO lane_history (lane_key, at, from_status, to_status, reason) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(key)
    .bind(now)
    .bind(&from_status)
    .bind(to)
    .bind(reason)
    .execute(&mut *tx)
    .await
    .map_err(OmonError::from)?;

    tx.commit().await.map_err(OmonError::from)?;
    Ok(true)
}

pub async fn get_lane_history(pool: &SqlitePool, key: &str) -> Result<Vec<LaneHistoryEntry>> {
    let rows = sqlx::query_as::<_, LaneHistoryEntry>(
        r#"
        SELECT id, lane_key, at, from_status, to_status, reason
        FROM lane_history
        WHERE lane_key = ?
        ORDER BY id ASC
        "#,
    )
    .bind(key)
    .fetch_all(pool)
    .await
    .map_err(OmonError::from)?;

    Ok(rows)
}

pub async fn set_lane_fields(
    pool: &SqlitePool,
    key: &str,
    fields: &LaneFieldsUpdate,
) -> Result<bool> {
    let mut sets = Vec::new();

    if fields.title.is_some() {
        sets.push("title = ?");
    }
    if fields.lead_session_key.is_some() {
        sets.push("lead_session_key = ?");
    }
    if fields.thread_channel_id.is_some() {
        sets.push("thread_channel_id = ?");
    } else if fields.clear_thread_channel_id {
        sets.push("thread_channel_id = NULL");
    }
    if fields.parent_channel_id.is_some() {
        sets.push("parent_channel_id = ?");
    } else if fields.clear_parent_channel_id {
        sets.push("parent_channel_id = NULL");
    }
    if fields.status_message_id.is_some() {
        sets.push("status_message_id = ?");
    } else if fields.clear_status_message_id {
        sets.push("status_message_id = NULL");
    }
    if fields.cwd.is_some() {
        sets.push("cwd = ?");
    }
    if fields.brief_path.is_some() {
        sets.push("brief_path = ?");
    }
    if fields.session_key.is_some() {
        sets.push("session_key = ?");
    }
    if fields.omo_thread_id.is_some() {
        sets.push("omo_thread_id = ?");
    } else if fields.clear_omo_thread_id {
        sets.push("omo_thread_id = NULL");
    }
    if fields.model.is_some() {
        sets.push("model = ?");
    } else if fields.clear_model {
        sets.push("model = NULL");
    }
    if fields.status.is_some() {
        sets.push("status = ?");
    }
    if fields.done_signals.is_some() {
        sets.push("done_signals = ?");
    }
    if fields.claim_evidence.is_some() {
        sets.push("claim_evidence = ?");
    } else if fields.clear_claim_evidence {
        sets.push("claim_evidence = NULL");
    }
    if fields.claim_at.is_some() {
        sets.push("claim_at = ?");
    } else if fields.clear_claim_at {
        sets.push("claim_at = NULL");
    }
    if fields.objection.is_some() {
        sets.push("objection = ?");
    } else if fields.clear_objection {
        sets.push("objection = NULL");
    }
    if fields.turns_run.is_some() {
        sets.push("turns_run = ?");
    }
    if fields.last_turn_outcome.is_some() {
        sets.push("last_turn_outcome = ?");
    } else if fields.clear_last_turn_outcome {
        sets.push("last_turn_outcome = NULL");
    }
    if fields.last_report.is_some() {
        sets.push("last_report = ?");
    } else if fields.clear_last_report {
        sets.push("last_report = NULL");
    }
    if fields.last_activity_at.is_some() {
        sets.push("last_activity_at = ?");
    }
    if fields.stale_posted_at.is_some() {
        sets.push("stale_posted_at = ?");
    } else if fields.clear_stale_posted_at {
        sets.push("stale_posted_at = NULL");
    }

    sets.push("updated_at = CURRENT_TIMESTAMP");

    let sql = format!("UPDATE lanes SET {} WHERE key = ?", sets.join(", "));
    let mut query = sqlx::query(&sql);

    if let Some(val) = &fields.title {
        query = query.bind(val);
    }
    if let Some(val) = &fields.lead_session_key {
        query = query.bind(val);
    }
    if let Some(val) = &fields.thread_channel_id {
        query = query.bind(val);
    }
    if let Some(val) = &fields.parent_channel_id {
        query = query.bind(val);
    }
    if let Some(val) = &fields.status_message_id {
        query = query.bind(val);
    }
    if let Some(val) = &fields.cwd {
        query = query.bind(val);
    }
    if let Some(val) = &fields.brief_path {
        query = query.bind(val);
    }
    if let Some(val) = &fields.session_key {
        query = query.bind(val);
    }
    if let Some(val) = &fields.omo_thread_id {
        query = query.bind(val);
    }
    if let Some(val) = &fields.model {
        query = query.bind(val);
    }
    if let Some(val) = &fields.status {
        query = query.bind(val);
    }
    if let Some(val) = &fields.done_signals {
        query = query.bind(val);
    }
    if let Some(val) = &fields.claim_evidence {
        query = query.bind(val);
    }
    if let Some(val) = fields.claim_at {
        query = query.bind(val);
    }
    if let Some(val) = &fields.objection {
        query = query.bind(val);
    }
    if let Some(val) = fields.turns_run {
        query = query.bind(val);
    }
    if let Some(val) = &fields.last_turn_outcome {
        query = query.bind(val);
    }
    if let Some(val) = &fields.last_report {
        query = query.bind(val);
    }
    if let Some(val) = fields.last_activity_at {
        query = query.bind(val);
    }
    if let Some(val) = fields.stale_posted_at {
        query = query.bind(val);
    }

    query = query.bind(key);

    let res = query.execute(pool).await.map_err(OmonError::from)?;
    Ok(res.rows_affected() > 0)
}

pub async fn append_transcript(
    pool: &SqlitePool,
    lane_key: &str,
    role: &str,
    text: &str,
) -> Result<i64> {
    let now = Utc::now();
    let res =
        sqlx::query("INSERT INTO lane_transcript (lane_key, at, role, text) VALUES (?, ?, ?, ?)")
            .bind(lane_key)
            .bind(now)
            .bind(role)
            .bind(text)
            .execute(pool)
            .await
            .map_err(OmonError::from)?;

    Ok(res.last_insert_rowid())
}

pub async fn read_transcript(
    pool: &SqlitePool,
    key: &str,
    last_n: Option<usize>,
) -> Result<Vec<LaneTranscriptEntry>> {
    let rows = match last_n {
        Some(limit) => sqlx::query_as::<_, LaneTranscriptEntry>(
            r#"
                SELECT id, lane_key, at, role, text
                FROM (
                    SELECT id, lane_key, at, role, text
                    FROM lane_transcript
                    WHERE lane_key = ?
                    ORDER BY id DESC
                    LIMIT ?
                )
                ORDER BY id ASC
                "#,
        )
        .bind(key)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .map_err(OmonError::from)?,
        None => sqlx::query_as::<_, LaneTranscriptEntry>(
            r#"
                SELECT id, lane_key, at, role, text
                FROM lane_transcript
                WHERE lane_key = ?
                ORDER BY id ASC
                "#,
        )
        .bind(key)
        .fetch_all(pool)
        .await
        .map_err(OmonError::from)?,
    };

    Ok(rows)
}

pub async fn enqueue_lane_event(
    pool: &SqlitePool,
    lane_key: Option<&str>,
    lead_session_key: &str,
    kind: &str,
    body: &str,
) -> Result<i64> {
    enqueue_lane_event_with_executor(pool, lane_key, lead_session_key, kind, body).await
}

pub async fn enqueue_lane_event_with_executor<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Sqlite>,
    lane_key: Option<&str>,
    lead_session_key: &str,
    kind: &str,
    body: &str,
) -> Result<i64> {
    let now = Utc::now();
    let res = sqlx::query(
        r#"
        INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at, delivered_at)
        VALUES (?, ?, ?, ?, ?, NULL)
        "#,
    )
    .bind(lane_key)
    .bind(lead_session_key)
    .bind(kind)
    .bind(body)
    .bind(now)
    .execute(executor)
    .await
    .map_err(OmonError::from)?;

    Ok(res.last_insert_rowid())
}

pub async fn take_undelivered_events(
    pool: &SqlitePool,
    limit: Option<usize>,
) -> Result<Vec<LaneEvent>> {
    let rows = match limit {
        Some(lim) => sqlx::query_as::<_, LaneEvent>(
            r#"
                SELECT id, lane_key, lead_session_key, kind, body, created_at, delivered_at
                FROM lane_events
                WHERE delivered_at IS NULL
                ORDER BY id ASC
                LIMIT ?
                "#,
        )
        .bind(lim as i64)
        .fetch_all(pool)
        .await
        .map_err(OmonError::from)?,
        None => sqlx::query_as::<_, LaneEvent>(
            r#"
                SELECT id, lane_key, lead_session_key, kind, body, created_at, delivered_at
                FROM lane_events
                WHERE delivered_at IS NULL
                ORDER BY id ASC
                "#,
        )
        .fetch_all(pool)
        .await
        .map_err(OmonError::from)?,
    };

    Ok(rows)
}

pub async fn mark_events_delivered(pool: &SqlitePool, ids: &[i64]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!("UPDATE lane_events SET delivered_at = ? WHERE id IN ({placeholders})");
    let now = Utc::now();
    let mut q = sqlx::query(&sql).bind(now);
    for id in ids {
        q = q.bind(id);
    }
    let res = q.execute(pool).await.map_err(OmonError::from)?;
    Ok(res.rows_affected())
}

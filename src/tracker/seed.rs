use chrono::{SecondsFormat, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;

pub fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub async fn seed_team_states(
    executor: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    team_id: &str,
) -> Result<()> {
    let default_states = [
        ("Backlog", "backlog", "#bec2c8", 1.0),
        ("Todo", "unstarted", "#e2e2e2", 2.0),
        ("In Progress", "started", "#f2c94c", 3.0),
        ("In Review", "started", "#4cb782", 4.0),
        ("Done", "completed", "#5e6ad2", 5.0),
        ("Canceled", "canceled", "#95a2b3", 6.0),
        ("Duplicate", "canceled", "#95a2b3", 7.0),
    ];

    for (name, state_type, color, position) in default_states {
        let state_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description)
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL)",
        )
        .bind(state_id)
        .bind(team_id)
        .bind(name)
        .bind(state_type)
        .bind(color)
        .bind(position)
        .execute(&mut **executor)
        .await?;
    }

    Ok(())
}

pub async fn seed_agent_states(
    executor: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    team_id: &str,
) -> Result<()> {
    let agent_states = [
        ("Queued", "unstarted", "#e2e2e2", 1.0, "queued"),
        ("Working", "started", "#f2c94c", 2.0, "working"),
        ("Blocked", "started", "#eb5757", 3.0, "blocked"),
        ("In Review", "started", "#4cb782", 4.0, "done_claimed"),
        ("Changes Requested", "started", "#fc7840", 5.0, "not_done"),
        ("Verified", "completed", "#5e6ad2", 6.0, "verified_done"),
        ("Done", "completed", "#5e6ad2", 7.0, "closed"),
        ("Failed", "canceled", "#95a2b3", 8.0, "failed"),
    ];

    for (name, state_type, color, position, lane_status) in agent_states {
        let state_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description)
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(state_id)
        .bind(team_id)
        .bind(name)
        .bind(state_type)
        .bind(color)
        .bind(position)
        .bind(lane_status)
        .execute(&mut **executor)
        .await?;
    }

    Ok(())
}

/// (name, type, color, position, session_state) for the system team that mirrors omo sessions.
pub const SESSION_STATES: [(&str, &str, &str, f64, &str); 5] = [
    ("Needs answer", "started", "#eb5757", 1.0, "question"),
    ("Blocked", "started", "#fc7840", 2.0, "blocked"),
    ("Working", "started", "#f2c94c", 3.0, "working"),
    ("Idle", "unstarted", "#e2e2e2", 4.0, "idle"),
    ("Ended", "completed", "#95a2b3", 5.0, "ended"),
];

/// Creates the SES system team and any missing session states; safe to call on every request
/// and independent of the one-time `seeded` marker so existing workspaces gain the team.
pub async fn ensure_sessions_team(pool: &SqlitePool) -> Result<String> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_teams WHERE key = 'SES'")
            .fetch_optional(pool)
            .await?;
    if let Some(id) = existing {
        let seeded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT session_state) FROM tracker_states WHERE team_id = ? AND session_state IS NOT NULL",
        )
        .bind(&id)
        .fetch_one(pool)
        .await?;
        if seeded >= SESSION_STATES.len() as i64 {
            return Ok(id);
        }
    }

    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query(
        "INSERT INTO tracker_teams (id, key, name, icon, color, issue_seq, system, cycles_enabled, cycle_weeks, triage_enabled, created_at, sort_order)
         VALUES (?, 'SES', 'Sessions', 'terminal', '#5e6ad2', 0, 1, 0, 2, 0, ?, 0.5)
         ON CONFLICT(key) DO NOTHING",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(now_utc())
    .execute(&mut *tx)
    .await?;
    let team_id: String = sqlx::query_scalar("SELECT id FROM tracker_teams WHERE key = 'SES'")
        .fetch_one(&mut *tx)
        .await?;
    for (name, state_type, color, position, session_state) in SESSION_STATES {
        let present: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM tracker_states WHERE team_id = ? AND session_state = ? LIMIT 1",
        )
        .bind(&team_id)
        .bind(session_state)
        .fetch_optional(&mut *tx)
        .await?;
        if present.is_some() {
            continue;
        }
        sqlx::query(
            "INSERT INTO tracker_states (id, team_id, name, type, color, position, lane_status, description, session_state)
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&team_id)
        .bind(name)
        .bind(state_type)
        .bind(color)
        .bind(position)
        .bind(session_state)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(team_id)
}

pub async fn ensure_seeded(pool: &SqlitePool) -> Result<()> {
    let seeded: Option<String> =
        sqlx::query_scalar("SELECT value FROM tracker_meta WHERE key = 'seeded'")
            .fetch_optional(pool)
            .await?;
    if seeded.as_deref() == Some("1") {
        return Ok(());
    }

    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    let existing: Option<String> =
        sqlx::query_scalar("SELECT value FROM tracker_meta WHERE key = 'seeded'")
            .fetch_optional(&mut *tx)
            .await?;

    if existing.as_deref() == Some("1") {
        tx.commit().await?;
        return Ok(());
    }

    let now = now_utc();

    sqlx::query(
        "INSERT INTO tracker_users (id, name, display_name, email, avatar_color, kind, model, is_me, created_at)
         VALUES (?, ?, ?, NULL, ?, 'human', NULL, 1, ?)
         ON CONFLICT(id) DO NOTHING",
    )
    .bind("me")
    .bind("Me")
    .bind("Me")
    .bind("#5e6ad2")
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let omo_team_id = Uuid::new_v4().to_string();
    let inserted_omo = sqlx::query(
        "INSERT INTO tracker_teams (id, key, name, icon, color, issue_seq, system, cycles_enabled, cycle_weeks, triage_enabled, created_at, sort_order)
         VALUES (?, 'OMO', 'Omon', 'box', '#5e6ad2', 0, 0, 0, 2, 0, ?, 1.0)
         ON CONFLICT(key) DO NOTHING",
    )
    .bind(&omo_team_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    if inserted_omo.rows_affected() > 0 {
        seed_team_states(&mut tx, &omo_team_id).await?;
    }

    let agt_team_id = Uuid::new_v4().to_string();
    let inserted_agt = sqlx::query(
        "INSERT INTO tracker_teams (id, key, name, icon, color, issue_seq, system, cycles_enabled, cycle_weeks, triage_enabled, created_at, sort_order)
         VALUES (?, 'AGT', 'Agents', 'bot', '#5e6ad2', 0, 1, 0, 2, 0, ?, 2.0)
         ON CONFLICT(key) DO NOTHING",
    )
    .bind(&agt_team_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    if inserted_agt.rows_affected() > 0 {
        seed_agent_states(&mut tx, &agt_team_id).await?;
    }

    let labels = [
        ("Bug", "#eb5757"),
        ("Feature", "#bb87fc"),
        ("Improvement", "#4ea7fc"),
    ];

    for (name, color) in labels {
        let label_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO tracker_labels (id, team_id, name, color, parent_id, description, created_at)
             VALUES (?, NULL, ?, ?, NULL, NULL, ?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(label_id)
        .bind(name)
        .bind(color)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO tracker_meta (key, value)
         VALUES ('seeded', '1')
         ON CONFLICT(key) DO UPDATE SET value = '1'",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

//! Mirrors agent work requested by configured people (by default the owner's wife, "여신님") from
//! the inbox into Omon team issues carrying a per-person label, plus a shared view filtered on it.
//!
//! Only items the triage classed `bot` (work an agent can do) are mirrored; items that need the
//! owner himself (`owner`), and `fyi`/`ignore` chatter, stay out of the tracker.
//! Each mirrored item is recorded in `tracker_meta` (`request_item:<item id>`), so an issue the
//! owner deletes is not recreated and a state the owner sets is kept until the inbox decision
//! itself changes.

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::Result;
use crate::tracker::seed::now_utc;

const REQUESTERS_ENV: &str = "TRACKER_REQUESTERS";
const DEFAULT_REQUESTERS: &str = "여신님";
const META_PREFIX: &str = "request_item:";
const LABEL_COLOR: &str = "#f27ec2";

static SYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Requester names from `TRACKER_REQUESTERS` (comma separated); each matches inbox authors
/// containing it.
fn requesters() -> Vec<String> {
    std::env::var(REQUESTERS_ENV)
        .unwrap_or_else(|_| DEFAULT_REQUESTERS.to_string())
        .split(',')
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// (state type, Omon state name) for an inbox decision status.
fn state_for(status: &str) -> (&'static str, &'static str) {
    match status {
        "done" => ("completed", "Done"),
        "dismissed" => ("canceled", "Canceled"),
        _ => ("unstarted", "Todo"),
    }
}

fn title_for(snippet: &str) -> String {
    let line = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        title.push('…');
    }
    if title.is_empty() {
        title = "(빈 메시지)".to_string();
    }
    title
}

/// Calendar date in KST (the owner's zone) of an RFC 3339 timestamp.
fn kst_date(timestamp: &str) -> Option<String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
    let kst = chrono::FixedOffset::east_opt(9 * 3600)?;
    Some(parsed.with_timezone(&kst).format("%Y-%m-%d").to_string())
}

fn kst_time(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .zip(chrono::FixedOffset::east_opt(9 * 3600))
        .map(|(t, kst)| {
            t.with_timezone(&kst)
                .format("%Y-%m-%d %H:%M KST")
                .to_string()
        })
        .unwrap_or_else(|| timestamp.to_string())
}

async fn ensure_label(pool: &SqlitePool, name: &str) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM tracker_labels WHERE team_id IS NULL AND name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tracker_labels (id, team_id, name, color, parent_id, description, created_at)
         VALUES (?, NULL, ?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(LABEL_COLOR)
    .bind(format!(
        "{name}이(가) 요청한 에이전트 작업 (인박스에서 자동 수집)"
    ))
    .bind(now_utc())
    .execute(pool)
    .await?;
    Ok(id)
}

async fn ensure_view(pool: &SqlitePool, name: &str, label_id: &str, me: &str) -> Result<()> {
    let key = format!("request_view:{name}");
    let known: Option<String> = sqlx::query_scalar("SELECT value FROM tracker_meta WHERE key = ?")
        .bind(&key)
        .fetch_optional(pool)
        .await?;
    if known.is_some() {
        return Ok(());
    }
    let id = Uuid::new_v4().to_string();
    let now = now_utc();
    let filters = serde_json::json!({ "label_ids": [label_id] }).to_string();
    let display = serde_json::json!({
        "layout": "list",
        "grouping": "status",
        "sub_grouping": "none",
        "ordering": "created",
        "show_completed": "all",
        "show_sub_issues": true,
        "show_empty_groups": false,
        "properties": ["id", "status", "labels", "due", "created"]
    })
    .to_string();
    sqlx::query(
        "INSERT INTO tracker_views (id, name, description, icon, color, owner_id, team_id, filters, display, shared, created_at, updated_at)
         VALUES (?, ?, ?, 'layers', ?, ?, NULL, ?, ?, 1, ?, ?)",
    )
    .bind(&id)
    .bind(format!("{name} 요청"))
    .bind(format!("{name}이(가) 지시한 작업"))
    .bind(LABEL_COLOR)
    .bind(me)
    .bind(&filters)
    .bind(&display)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO tracker_meta (key, value) VALUES (?, ?)")
        .bind(&key)
        .bind(&id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mirrors request-like inbox items from every configured requester into labelled Omon issues.
pub async fn sync(pool: &SqlitePool) -> Result<()> {
    let _guard = SYNC_LOCK.lock().await;
    let team: Option<(String, String)> =
        sqlx::query_as("SELECT id, key FROM tracker_teams WHERE key = 'OMO'")
            .fetch_optional(pool)
            .await?;
    let Some((team_id, team_key)) = team else {
        return Ok(());
    };
    let me: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracker_users WHERE is_me = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;
    let Some(me) = me else {
        return Ok(());
    };
    for name in requesters() {
        sync_requester(pool, &name, &team_id, &team_key, &me).await?;
    }
    Ok(())
}

async fn state_id(
    pool: &SqlitePool,
    team_id: &str,
    status: &str,
) -> Result<Option<(String, String)>> {
    let (state_type, state_name) = state_for(status);
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT id, type FROM tracker_states WHERE team_id = ? AND type = ?
         ORDER BY (name = ?) DESC, position LIMIT 1",
    )
    .bind(team_id)
    .bind(state_type)
    .bind(state_name)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

fn lifecycle(state_type: &str, at: &str) -> (Option<String>, Option<String>) {
    match state_type {
        "completed" => (Some(at.to_string()), None),
        "canceled" => (None, Some(at.to_string())),
        _ => (None, None),
    }
}

async fn sync_requester(
    pool: &SqlitePool,
    name: &str,
    team_id: &str,
    team_key: &str,
    me: &str,
) -> Result<()> {
    let rows = sqlx::query(
        "SELECT i.id, i.source, i.author, i.snippet, i.received_at, d.class, d.status, d.reason,
                d.due_at, d.proposed_action, m.value AS mirrored
           FROM inbox_items i
           JOIN inbox_decisions d ON d.item_id = i.id
           LEFT JOIN tracker_meta m ON m.key = ? || i.id
          WHERE instr(coalesce(i.author, ''), ?) > 0 AND d.class = 'bot'
          ORDER BY i.received_at",
    )
    .bind(META_PREFIX)
    .bind(name)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }
    let label_id = ensure_label(pool, name).await?;
    ensure_view(pool, name, &label_id, me).await?;

    for row in &rows {
        let item_id: String = row.get("id");
        let status: String = row.get("status");
        let mirrored: Option<String> = row.get("mirrored");
        let meta_key = format!("{META_PREFIX}{item_id}");

        if let Some(value) = mirrored {
            let (issue_id, last_status) = value.split_once('|').unwrap_or((value.as_str(), ""));
            if last_status == status {
                continue;
            }
            // The inbox decision moved (e.g. the owner marked it done); follow it.
            if let Some((state, state_type)) = state_id(pool, team_id, &status).await? {
                let now = now_utc();
                let (completed_at, canceled_at) = lifecycle(&state_type, &now);
                sqlx::query(
                    "UPDATE tracker_issues SET state_id = ?, completed_at = ?, canceled_at = ?, updated_at = ?
                     WHERE id = ?",
                )
                .bind(&state)
                .bind(&completed_at)
                .bind(&canceled_at)
                .bind(&now)
                .bind(issue_id)
                .execute(pool)
                .await?;
            }
            sqlx::query("UPDATE tracker_meta SET value = ? WHERE key = ?")
                .bind(format!("{issue_id}|{status}"))
                .bind(&meta_key)
                .execute(pool)
                .await?;
            continue;
        }

        let Some((state, state_type)) = state_id(pool, team_id, &status).await? else {
            continue;
        };
        let snippet: String = row.get("snippet");
        let source: String = row.get("source");
        let author: Option<String> = row.get("author");
        let received_at: String = row.get("received_at");
        let class: String = row.get("class");
        let reason: String = row.get("reason");
        let due_at: Option<String> = row.get("due_at");
        let proposed: Option<String> = row.get("proposed_action");

        let mut description = format!(
            "> {}\n\n- 요청자: {} ({source})\n- 받은 시각: {}\n- 인박스 분류: {class} / {status}\n- 분류 사유: {reason}\n",
            snippet.trim().replace('\n', "\n> "),
            author.as_deref().unwrap_or(name),
            kst_time(&received_at),
        );
        if let Some(action) = proposed.as_deref().filter(|a| !a.trim().is_empty()) {
            description.push_str(&format!("- 제안된 처리: {action}\n"));
        }
        description.push_str(&format!("- inbox item: `{item_id}`\n"));

        let created_at = chrono::DateTime::parse_from_rfc3339(&received_at)
            .map(|t| {
                t.with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_else(|_| now_utc());
        let (completed_at, canceled_at) = lifecycle(&state_type, &now_utc());
        let due_date = due_at.as_deref().and_then(kst_date);
        let issue_id = Uuid::new_v4().to_string();

        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let number: i64 = sqlx::query_scalar(
            "UPDATE tracker_teams SET issue_seq = issue_seq + 1 WHERE id = ? RETURNING issue_seq",
        )
        .bind(team_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO tracker_issues (id, team_id, number, identifier, title, description, state_id, priority,
                estimate, assignee_id, creator_id, project_id, milestone_id, cycle_id, parent_id, due_date,
                sort_order, lane_key, created_at, updated_at, started_at, completed_at, canceled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?, NULL, NULL, NULL, NULL, ?, ?, NULL, ?, ?, NULL, ?, ?)",
        )
        .bind(&issue_id)
        .bind(team_id)
        .bind(number)
        .bind(format!("{team_key}-{number}"))
        .bind(title_for(&snippet))
        .bind(&description)
        .bind(&state)
        .bind(me)
        .bind(me)
        .bind(&due_date)
        .bind(-(number as f64))
        .bind(&created_at)
        .bind(&created_at)
        .bind(&completed_at)
        .bind(&canceled_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO tracker_issue_labels (issue_id, label_id) VALUES (?, ?)")
            .bind(&issue_id)
            .bind(&label_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO tracker_subscribers (issue_id, user_id) VALUES (?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(&issue_id)
        .bind(me)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO tracker_activity (issue_id, actor_id, kind, from_value, to_value, created_at)
             VALUES (?, NULL, 'created', NULL, NULL, ?)",
        )
        .bind(&issue_id)
        .bind(&created_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO tracker_meta (key, value) VALUES (?, ?)")
            .bind(&meta_key)
            .bind(format!("{issue_id}|{status}"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use crate::tracker::seed::ensure_seeded;

    async fn insert_item(
        pool: &SqlitePool,
        id: &str,
        author: &str,
        snippet: &str,
        class: &str,
        status: &str,
    ) {
        sqlx::query(
            "INSERT INTO inbox_items (id, source, source_ref, thread_ref, author, title, snippet, received_at, link, raw_hash, collected_at)
             VALUES (?, 'kakao', ?, 't', ?, ?, ?, '2026-10-21T10:00:00+00:00', NULL, 'h', '2026-10-21T10:00:00+00:00')",
        )
        .bind(id)
        .bind(id)
        .bind(author)
        .bind(author)
        .bind(snippet)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO inbox_decisions (item_id, class, reason, status, decided_at) VALUES (?, ?, 'r', ?, '2026-10-21T10:00:00+00:00')",
        )
        .bind(id)
        .bind(class)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn labelled(pool: &SqlitePool) -> Vec<(String, String)> {
        sqlx::query_as(
            "SELECT i.title, s.name FROM tracker_issues i
               JOIN tracker_issue_labels il ON il.issue_id = i.id
               JOIN tracker_labels l ON l.id = il.label_id AND l.name = '여신님'
               JOIN tracker_states s ON s.id = i.state_id
              ORDER BY i.title",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn mirrors_requests_once_and_follows_inbox_status() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool();
        ensure_seeded(pool).await.unwrap();
        insert_item(pool, "a", "여신님😍", "인강 체크표 만들어줘", "bot", "open").await;
        insert_item(pool, "b", "여신님😍", "티거 교재 만들어줘", "bot", "open").await;
        insert_item(pool, "c", "여신님😍", "방석 챙겨줘", "owner", "open").await;
        insert_item(pool, "e", "여신님😍", "쿠팡 회수 해줘요", "fyi", "open").await;
        insert_item(pool, "d", "다른사람", "이거 만들어줘", "bot", "open").await;

        sync(pool).await.unwrap();
        sync(pool).await.unwrap();
        assert_eq!(
            labelled(pool).await,
            vec![
                ("인강 체크표 만들어줘".to_string(), "Todo".to_string()),
                ("티거 교재 만들어줘".to_string(), "Todo".to_string()),
            ]
        );
        let views: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tracker_views WHERE name = '여신님 요청'")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(views, 1);

        sqlx::query("UPDATE inbox_decisions SET status = 'done' WHERE item_id = 'a'")
            .execute(pool)
            .await
            .unwrap();
        sync(pool).await.unwrap();
        assert_eq!(labelled(pool).await[0].1, "Done");

        // A deleted issue stays deleted.
        sqlx::query("DELETE FROM tracker_issues WHERE title LIKE '티거%'")
            .execute(pool)
            .await
            .unwrap();
        sync(pool).await.unwrap();
        assert_eq!(labelled(pool).await.len(), 1);
    }
}

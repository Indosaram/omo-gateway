use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;

/// Appends an out-of-band delivery record to the target session transcript with role preservation.
///
/// `role` should typically be "assistant" for agent out-of-band responses,
/// or "user" for system/external notifications (to maintain LLM provider alternation).
pub async fn mirror_to_session(
    pool: &SqlitePool,
    session_key: &str,
    role: &str,
    content: &str,
    source_label: Option<&str>,
) -> Result<bool> {
    let text = content.trim();
    if text.is_empty() {
        return Ok(false);
    }

    let exists: Option<(String,)> =
        sqlx::query_as("SELECT session_key FROM sessions WHERE session_key = ? LIMIT 1")
            .bind(session_key)
            .fetch_optional(pool)
            .await?;

    let Some((target_session_key,)) = exists else {
        return Ok(false);
    };

    let message_id = Uuid::new_v4().to_string();
    let metadata = json!({
        "mirror": true,
        "mirror_source": source_label.unwrap_or("out_of_band"),
    });

    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at)
         VALUES (?, ?, ?, ?, ?, (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')))",
    )
    .bind(&message_id)
    .bind(&target_session_key)
    .bind(role)
    .bind(text)
    .bind(metadata.to_string())
    .execute(pool)
    .await?;

    let _ = sqlx::query(
        "UPDATE sessions SET updated_at = (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE session_key = ?",
    )
    .bind(&target_session_key)
    .execute(pool)
    .await;

    Ok(true)
}

/// Finds the most relevant active session key for a platform and origin coordinates.
fn extract_bot_id_from_session_key(key: &str) -> Option<String> {
    if let Ok(k) = crate::SessionKey::from_storage_key(key) {
        return k.bot_id;
    }
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() >= 4 {
        Some(parts[1].to_string())
    } else {
        None
    }
}

pub async fn find_session_by_origin(
    pool: &SqlitePool,
    platform: &str,
    chat_id: &str,
    thread_id: Option<&str>,
    user_id: Option<&str>,
    bot_id: Option<&str>,
) -> Result<Option<String>> {
    let platform = platform.to_ascii_lowercase();

    if let Some(tid) = thread_id {
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT session_key, user_id FROM sessions
             WHERE lower(platform) = ? AND channel_id = ? AND thread_id = ?
             ORDER BY updated_at DESC",
        )
        .bind(&platform)
        .bind(chat_id)
        .bind(tid)
        .fetch_all(pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let matching_bot_rows: Vec<&(String, Option<String>)> = rows
            .iter()
            .filter(|r| {
                if let Some(bid) = bot_id {
                    extract_bot_id_from_session_key(&r.0).as_deref() == Some(bid)
                } else {
                    true
                }
            })
            .collect();

        if matching_bot_rows.is_empty() {
            return Ok(None);
        }

        if let Some(uid) = user_id {
            if let Some(matching) = matching_bot_rows
                .iter()
                .find(|r| r.1.as_deref() == Some(uid))
            {
                return Ok(Some(matching.0.clone()));
            }
        }

        if bot_id.is_none() {
            let distinct_bots: std::collections::HashSet<_> = matching_bot_rows
                .iter()
                .map(|r| extract_bot_id_from_session_key(&r.0))
                .collect();
            if distinct_bots.len() > 1 {
                return Ok(None);
            }
        }

        return Ok(Some(matching_bot_rows[0].0.clone()));
    }

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT session_key, user_id FROM sessions
         WHERE lower(platform) = ? AND channel_id = ? AND (thread_id IS NULL OR thread_id = '')
         ORDER BY updated_at DESC",
    )
    .bind(&platform)
    .bind(chat_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let matching_bot_rows: Vec<&(String, Option<String>)> = rows
        .iter()
        .filter(|r| {
            if let Some(bid) = bot_id {
                extract_bot_id_from_session_key(&r.0).as_deref() == Some(bid)
            } else {
                true
            }
        })
        .collect();

    if matching_bot_rows.is_empty() {
        return Ok(None);
    }

    if let Some(uid) = user_id {
        if let Some(matching) = matching_bot_rows
            .iter()
            .find(|r| r.1.as_deref() == Some(uid))
        {
            return Ok(Some(matching.0.clone()));
        }
    }

    if bot_id.is_none() {
        let distinct_bots: std::collections::HashSet<_> = matching_bot_rows
            .iter()
            .map(|r| extract_bot_id_from_session_key(&r.0))
            .collect();
        if distinct_bots.len() > 1 {
            return Ok(None);
        }
    }

    Ok(Some(matching_bot_rows[0].0.clone()))
}

/// Upper bound on briefing text carried into one turn, so a backlog of cron
/// output cannot crowd out the user's own message.
const MAX_BRIEFING_CHARS: usize = 6000;

/// Cron deliveries selected for the next agent turn, plus the transcript rows
/// that were actually rendered into [text] (and so may be marked consumed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CronBriefings {
    pub text: String,
    pub message_ids: Vec<String>,
}

/// Selects cron deliveries mirrored into this session that were not yet handed
/// to the agent and renders them as context, WITHOUT marking them consumed.
///
/// Cron output is posted to the channel by the scheduler, not by the agent's own
/// thread, so without this the agent cannot see what it "said" there and cannot
/// answer follow-up questions about it. Callers mark [CronBriefings::message_ids]
/// with [mark_cron_briefings_injected] once the turn that carries the text has
/// been accepted, so a failed turn does not lose the context.
///
/// When the backlog exceeds [MAX_BRIEFING_CHARS] the newest deliveries are kept
/// and the older ones stay pending for a later turn instead of being dropped.
pub async fn peek_pending_cron_briefings(
    pool: &SqlitePool,
    session_key: &str,
) -> Result<Option<CronBriefings>> {
    // json_valid guards every json_extract: one non-JSON metadata row in the
    // session must not make the whole query fail.
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, content, created_at,
                COALESCE(CASE WHEN json_valid(metadata_json)
                         THEN json_extract(metadata_json, '$.mirror_source') END, '')
         FROM messages
         WHERE session_key = ?
           AND CASE WHEN json_valid(metadata_json)
               THEN json_extract(metadata_json, '$.mirror') END = 1
           AND CASE WHEN json_valid(metadata_json)
               THEN json_extract(metadata_json, '$.mirror_source') END LIKE 'cron:%'
           AND CASE WHEN json_valid(metadata_json)
               THEN COALESCE(json_extract(metadata_json, '$.mirror_injected'), 0) END = 0
           AND CASE WHEN json_valid(metadata_json)
               THEN json_extract(metadata_json, '$.mirror_source') END <> ?
         ORDER BY created_at ASC, sequence ASC",
    )
    .bind(session_key)
    // The inbox digest can carry a one-time owner approval token; it is delivered to
    // Discord only and must never be injected into an agent prompt.
    .bind(format!("cron:{}", crate::inbox::digest::JOB_ID))
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }

    let header = "[Context: messages you already posted to this conversation as scheduled (cron) deliveries. \
         The user may refer to them.]\n";
    let mut used = header.chars().count();
    let mut selected: Vec<(String, String)> = Vec::new();
    let mut truncated = false;
    for (id, content, created_at, source) in rows.iter().rev() {
        let job = source.strip_prefix("cron:").unwrap_or(source);
        let mut entry = format!("--- {created_at} ({job})\n{}\n", content.trim());
        let mut len = entry.chars().count();
        if selected.is_empty() && used + len > MAX_BRIEFING_CHARS {
            // A single oversized delivery is cut (keeping its tail) so it is still
            // delivered once instead of blocking the queue forever.
            let keep = MAX_BRIEFING_CHARS.saturating_sub(used);
            entry = entry.chars().skip(len - keep.min(len)).collect();
            len = entry.chars().count();
            truncated = true;
        } else if used + len > MAX_BRIEFING_CHARS {
            truncated = true;
            break;
        }
        used += len;
        selected.push((id.clone(), entry));
    }
    selected.reverse();

    let mut text = String::new();
    if truncated {
        text.push_str("[Context: earlier scheduled deliveries truncated]\n");
    }
    text.push_str(header);
    let mut message_ids = Vec::with_capacity(selected.len());
    for (id, entry) in selected {
        text.push_str(&entry);
        message_ids.push(id);
    }
    Ok(Some(CronBriefings { text, message_ids }))
}

/// Marks the given mirrored deliveries as handed to the agent.
pub async fn mark_cron_briefings_injected(pool: &SqlitePool, message_ids: &[String]) -> Result<()> {
    if message_ids.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for id in message_ids {
        sqlx::query(
            "UPDATE messages SET metadata_json = json_set(metadata_json, '$.mirror_injected', json('true'))
             WHERE id = ? AND json_valid(metadata_json)",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Selects pending cron briefings and immediately marks exactly the rendered rows
/// consumed. Prefer [peek_pending_cron_briefings] plus [mark_cron_briefings_injected]
/// when the caller can confirm the turn was accepted before consuming them.
pub async fn take_pending_cron_briefings(
    pool: &SqlitePool,
    session_key: &str,
) -> Result<Option<String>> {
    let Some(briefings) = peek_pending_cron_briefings(pool, session_key).await? else {
        return Ok(None);
    };
    mark_cron_briefings_injected(pool, &briefings.message_ids).await?;
    Ok(Some(briefings.text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cron_briefings_are_handed_to_the_agent_exactly_once() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let session_key = "briefing-session";
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(&pool)
        .await
        .unwrap();

        assert!(take_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .is_none());

        mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "D-3 Plug in : Tokyo #15 call for startups",
            Some("cron:gov-grants"),
        )
        .await
        .unwrap();
        mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "not a cron mirror",
            Some("system"),
        )
        .await
        .unwrap();

        let first = take_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .expect("pending cron briefing must be returned");
        assert!(first.contains("Plug in : Tokyo #15"), "{first}");
        assert!(first.contains("gov-grants"), "{first}");
        assert!(!first.contains("not a cron mirror"), "{first}");

        assert!(
            take_pending_cron_briefings(&pool, session_key)
                .await
                .unwrap()
                .is_none(),
            "a briefing must be injected only once"
        );
    }

    #[tokio::test]
    async fn inbox_digest_mirror_rows_are_never_briefed_to_the_agent() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let session_key = "digest-session";
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(&pool)
        .await
        .unwrap();
        let digest_label = format!("cron:{}", crate::inbox::digest::JOB_ID);
        mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "digest with --owner-token deadbeef",
            Some(digest_label.as_str()),
        )
        .await
        .unwrap();
        mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "ordinary cron report",
            Some("cron:gov-grants"),
        )
        .await
        .unwrap();

        let briefings = peek_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .expect("the ordinary cron report must still be briefed");
        assert_eq!(briefings.message_ids.len(), 1);
        assert!(briefings.text.contains("ordinary cron report"));
        assert!(!briefings.text.contains("deadbeef"));
    }

    #[tokio::test]
    async fn test_mirror_to_session_role_preservation() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();

        let session_key = "test-session-key";
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(&pool)
        .await
        .unwrap();

        // 1. Mirror assistant role
        let mirrored = mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "Job completed: ok",
            Some("cron"),
        )
        .await
        .unwrap();
        assert!(mirrored);

        // 2. Mirror user role
        let mirrored_user = mirror_to_session(
            &pool,
            session_key,
            "user",
            "System alert: high memory",
            Some("system"),
        )
        .await
        .unwrap();
        assert!(mirrored_user);

        // 3. Empty content should return false
        let mirrored_empty = mirror_to_session(&pool, session_key, "assistant", "   ", None)
            .await
            .unwrap();
        assert!(!mirrored_empty);

        // 4. Non-existent session should return false
        let mirrored_missing =
            mirror_to_session(&pool, "non-existent-key", "assistant", "content", None)
                .await
                .unwrap();
        assert!(!mirrored_missing);

        // Verify rows in messages table
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT role, content, metadata_json FROM messages WHERE session_key = ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "assistant");
        assert_eq!(rows[0].1, "Job completed: ok");
        assert!(rows[0].2.contains("\"mirror\":true"));
        assert!(rows[0].2.contains("\"mirror_source\":\"cron\""));

        assert_eq!(rows[1].0, "user");
        assert_eq!(rows[1].1, "System alert: high memory");
        assert!(rows[1].2.contains("\"mirror_source\":\"system\""));
    }

    #[tokio::test]
    async fn test_find_session_by_origin_precedence() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, thread_id, user_id, state_json)
             VALUES ('sess-chan', 'discord', 'c1', NULL, 'u1', '{}'),
                    ('sess-thread', 'discord', 'c1', 't1', 'u1', '{}'),
                    ('sess-user2', 'discord', 'c1', NULL, 'u2', '{}')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let found = find_session_by_origin(&pool, "discord", "c1", Some("t1"), None, None)
            .await
            .unwrap();
        assert_eq!(found, Some("sess-thread".to_string()));

        let found_user = find_session_by_origin(&pool, "discord", "c1", None, Some("u2"), None)
            .await
            .unwrap();
        assert_eq!(found_user, Some("sess-user2".to_string()));

        let found_chan = find_session_by_origin(&pool, "discord", "c1", None, None, None)
            .await
            .unwrap();
        assert!(found_chan.is_some());
    }

    #[tokio::test]
    async fn channel_lookup_does_not_fall_back_to_thread_session() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, thread_id, user_id, state_json)
             VALUES ('discord:bot-a:c1:t1:u1', 'discord', 'c1', 't1', 'u1', '{}')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let found = find_session_by_origin(&pool, "discord", "c1", None, None, None)
            .await
            .unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn mirror_refuses_wrong_thread_and_ambiguous_bot() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, thread_id, user_id, state_json)
             VALUES ('discord:bot-a:c1:t1:u1', 'discord', 'c1', 't1', 'u1', '{}'),
                    ('discord:bot-b:c1:t2:u2', 'discord', 'c1', 't2', 'u2', '{}')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let found_t3 = find_session_by_origin(&pool, "discord", "c1", Some("t3"), None, None)
            .await
            .unwrap();
        assert_eq!(found_t3, None);

        let found_c1 = find_session_by_origin(&pool, "discord", "c1", None, None, None)
            .await
            .unwrap();
        assert_eq!(found_c1, None);

        let found_bota =
            find_session_by_origin(&pool, "discord", "c1", Some("t1"), None, Some("bot-a"))
                .await
                .unwrap();
        assert_eq!(found_bota, Some("discord:bot-a:c1:t1:u1".to_string()));
    }
    #[tokio::test]
    async fn cron_briefings_survive_malformed_metadata_and_oversize_backlog() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let session_key = "briefing-bounds";
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, channel_id, user_id, state_json)
             VALUES (?, 'discord', 'c1', 'u1', '{}')",
        )
        .bind(session_key)
        .execute(&pool)
        .await
        .unwrap();
        // A non-JSON metadata row must not fail the lookup.
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json)
             VALUES ('legacy-row', ?, 'user', 'legacy', 'not json')",
        )
        .bind(session_key)
        .execute(&pool)
        .await
        .unwrap();

        let big = "x".repeat(MAX_BRIEFING_CHARS - 100);
        for (label, text) in [("cron:big", big.as_str()), ("cron:mid", "MIDDLE-DELIVERY")] {
            mirror_to_session(&pool, session_key, "assistant", text, Some(label))
                .await
                .unwrap();
        }
        mirror_to_session(
            &pool,
            session_key,
            "assistant",
            "NEWEST-DELIVERY",
            Some("cron:new"),
        )
        .await
        .unwrap();

        let peeked = peek_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .expect("pending briefings");
        assert!(peeked.text.contains("NEWEST-DELIVERY"), "{}", peeked.text);
        assert!(peeked.text.contains("truncated"), "{}", peeked.text);
        assert!(peeked.text.contains("MIDDLE-DELIVERY"), "{}", peeked.text);
        assert!(!peeked.text.contains("xxxxxxxxxx"), "{}", peeked.text);
        assert!(peeked.text.chars().count() <= MAX_BRIEFING_CHARS + 100);
        // Peeking must not consume anything.
        let again = peek_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(again, peeked);

        mark_cron_briefings_injected(&pool, &peeked.message_ids)
            .await
            .unwrap();
        // The dropped (older, oversized) delivery is still pending, not silently consumed.
        let rest = peek_pending_cron_briefings(&pool, session_key)
            .await
            .unwrap()
            .expect("rows that were not rendered must stay pending");
        assert!(rest.text.contains("xxxxxxxxxx"), "{}", rest.text);
        assert!(!rest.text.contains("NEWEST-DELIVERY"), "{}", rest.text);
        assert!(rest.text.chars().count() <= MAX_BRIEFING_CHARS + 100);
    }
}

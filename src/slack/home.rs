//! Owner home on Slack: the owner DM that leads lanes, digests and alerts when Slack is home.

use sqlx::SqlitePool;

use super::{SlackApi, SlackConfig, PLATFORM};
use crate::{Result, SessionKey};

/// Discord bot whose persona (`bot_profiles` row) the Slack bot inherits by default (wawabot).
pub const DEFAULT_PERSONA_SOURCE: &str = "1465631383862120451";

/// Opens (or reuses) the owner's DM with the bot and persists its session row, so lead
/// validation, the digest and the kibitzer all find the same key.
pub async fn owner_dm_key(
    api: &SlackApi,
    config: &SlackConfig,
    bot_user_id: &str,
    pool: &SqlitePool,
) -> Result<SessionKey> {
    let owner = config.owner_id();
    let channel = api.open_im(owner).await?;
    let key = dm_key(&channel, owner, bot_user_id);
    persist_session(pool, &key).await?;
    Ok(key)
}

fn dm_key(channel: &str, owner: &str, bot_user_id: &str) -> SessionKey {
    SessionKey::new(PLATFORM, None::<String>, channel, None::<String>, owner)
        .with_bot_id(bot_user_id)
}

async fn persist_session(pool: &SqlitePool, key: &SessionKey) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json)
         VALUES (?, ?, NULL, ?, NULL, ?, '{}') ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(key.storage_key())
    .bind(&key.platform)
    .bind(&key.channel_id)
    .bind(&key.user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// True when owner-facing traffic (lead, digest, alerts) belongs on Slack:
/// `OMON_HOME_PLATFORM=slack`, or no Discord transport is configured.
pub fn is_home(home_platform: Option<&str>, discord_configured: bool) -> bool {
    match home_platform.map(|value| value.trim().to_ascii_lowercase()) {
        Some(platform) if platform == PLATFORM => true,
        Some(platform) if platform == "discord" => !discord_configured,
        _ => !discord_configured,
    }
}

/// Copies the source bot's persona to the Slack bot unless the Slack bot already has one.
/// Returns whether a row was written.
pub async fn copy_persona(
    pool: &SqlitePool,
    source_bot_id: &str,
    slack_bot_id: &str,
) -> Result<bool> {
    if source_bot_id == slack_bot_id {
        return Ok(false);
    }
    let result = sqlx::query(
        "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets, custom_settings_json)
         SELECT ?, name, model, system_prompt, enabled_toolsets, custom_settings_json
         FROM bot_profiles WHERE bot_id = ?
         AND NOT EXISTS (SELECT 1 FROM bot_profiles WHERE bot_id = ?)",
    )
    .bind(slack_bot_id)
    .bind(source_bot_id)
    .bind(slack_bot_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Persona source from `OMON_SLACK_PERSONA_FROM`, else [`DEFAULT_PERSONA_SOURCE`].
pub fn persona_source(raw: Option<&str>) -> String {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PERSONA_SOURCE)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::Database;

    async fn pool() -> SqlitePool {
        Database::connect("sqlite::memory:")
            .await
            .unwrap()
            .pool()
            .clone()
    }

    async fn fake_open_im() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new().fallback(|req: axum::extract::Request| async move {
            let path = req.uri().path().to_owned();
            let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
                .await
                .unwrap();
            let body: Value = crate::slack::api::decode_test_body(&bytes);
            axum::Json(match path.as_str() {
                "/api/conversations.open" if body["users"] == "UOWNER" => {
                    json!({ "ok": true, "channel": { "id": "DOWNER" } })
                }
                _ => json!({ "ok": false, "error": "unexpected" }),
            })
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("{base}/api")
    }

    #[tokio::test]
    async fn owner_dm_key_is_the_persisted_owner_dm() {
        let pool = pool().await;
        let api = SlackApi::with_base_url("xoxb-test".into(), fake_open_im().await);
        let config = SlackConfig {
            allowed_users: vec!["UOWNER".into()],
            ..SlackConfig::test_default()
        };
        let key = owner_dm_key(&api, &config, "UBOT", &pool).await.unwrap();
        assert_eq!(key.platform, PLATFORM);
        assert_eq!(key.guild_id, None);
        assert_eq!(key.thread_id, None);
        assert_eq!(key.channel_id, "DOWNER");
        assert_eq!(key.user_id, "UOWNER");
        assert_eq!(key.bot_id.as_deref(), Some("UBOT"));
        let stored: (String, String) = sqlx::query_as(
            "SELECT platform, user_id FROM sessions WHERE session_key = ? AND guild_id IS NULL AND thread_id IS NULL",
        )
        .bind(key.storage_key())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, ("slack".into(), "UOWNER".into()));
        let again = owner_dm_key(&api, &config, "UBOT", &pool).await.unwrap();
        assert_eq!(again.storage_key(), key.storage_key());
        assert_eq!(
            key.storage_key()
                .parse::<SessionKey>()
                .unwrap()
                .storage_key(),
            key.storage_key()
        );
    }

    #[test]
    fn home_follows_explicit_platform_then_discord_presence() {
        assert!(is_home(Some("slack"), true));
        assert!(is_home(Some(" Slack "), true));
        assert!(!is_home(Some("discord"), true));
        assert!(is_home(Some("discord"), false));
        assert!(!is_home(None, true));
        assert!(is_home(None, false));
    }

    #[tokio::test]
    async fn persona_copies_once_and_never_overwrites() {
        let pool = pool().await;
        sqlx::query(
            "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets) VALUES ('SRC', 'wawabot', 'm', 'persona', 'web')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(copy_persona(&pool, "SRC", "UBOT").await.unwrap());
        let copied: (String, String) =
            sqlx::query_as("SELECT name, system_prompt FROM bot_profiles WHERE bot_id = 'UBOT'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(copied, ("wawabot".into(), "persona".into()));
        sqlx::query("UPDATE bot_profiles SET system_prompt = 'edited' WHERE bot_id = 'UBOT'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(!copy_persona(&pool, "SRC", "UBOT").await.unwrap());
        let kept: String =
            sqlx::query_scalar("SELECT system_prompt FROM bot_profiles WHERE bot_id = 'UBOT'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kept, "edited");
        assert!(!copy_persona(&pool, "MISSING", "UOTHER").await.unwrap());
    }

    #[test]
    fn persona_source_defaults_to_wawabot() {
        assert_eq!(persona_source(None), DEFAULT_PERSONA_SOURCE);
        assert_eq!(persona_source(Some("  ")), DEFAULT_PERSONA_SOURCE);
        assert_eq!(persona_source(Some(" X ")), "X");
    }
}

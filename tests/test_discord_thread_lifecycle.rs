use omon_gateway::discord::adapter::{handle_thread_delete, handle_thread_update};
use omon_gateway::discord::commands::PoiseData;
use omon_gateway::{
    AgentRunner, InboundEvent, MultiplexerConfig, OmonError, SessionContext, SessionKey,
    SessionMultiplexer,
};
use poise::serenity_prelude as serenity;
use serenity::all::{GuildChannel, PartialGuildChannel};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

struct NoopRunner;

#[async_trait::async_trait]
impl AgentRunner for NoopRunner {
    async fn run(
        &self,
        _session: &mut SessionContext,
        _event: InboundEvent,
    ) -> Result<(), OmonError> {
        Ok(())
    }
}

async fn setup_test_db() -> SqlitePool {
    let pool = omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .expect("in-memory db init");

    let _ = sqlx::query("ALTER TABLE sessions ADD COLUMN metadata TEXT DEFAULT '{}'")
        .execute(&pool)
        .await;

    pool
}

fn make_session_key(thread_id: &str) -> SessionKey {
    SessionKey::new("discord", None::<String>, thread_id, None::<String>, "")
}

#[tokio::test]
async fn test_thread_archive_updates_metadata_and_stops_multiplexer() {
    let pool = setup_test_db().await;
    let runner = Arc::new(NoopRunner);
    let multiplexer = SessionMultiplexer::new(pool.clone(), runner, MultiplexerConfig::default());
    let data = PoiseData::new(multiplexer.clone(), pool.clone());

    let thread_id = "1234567890";
    let session_key = make_session_key(thread_id);

    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, user_id, metadata, state_json)
         VALUES (?, 'discord', ?, '', '{\"thread_archived\": false}', '{}')",
    )
    .bind(session_key.storage_key())
    .bind(thread_id)
    .execute(&pool)
    .await
    .unwrap();

    let thread_channel: GuildChannel = serde_json::from_value(serde_json::json!({
        "id": thread_id,
        "name": "test-thread",
        "type": 11,
        "guild_id": "111",
        "position": 0,
        "permission_overwrites": [],
        "thread_metadata": {
            "archived": true,
            "auto_archive_duration": 60,
            "archive_timestamp": "2026-10-07T00:00:00Z",
            "locked": false
        }
    }))
    .unwrap();

    handle_thread_update(&data, &thread_channel).await;

    let row = sqlx::query("SELECT metadata FROM sessions WHERE session_key = ?")
        .bind(session_key.storage_key())
        .fetch_one(&pool)
        .await
        .unwrap();

    let metadata_str: String = row.get("metadata");
    let metadata_val: serde_json::Value = serde_json::from_str(&metadata_str).unwrap();
    // SQLite json_set stores booleans as 1/0 or booleans depending on JSON representation.
    assert!(metadata_val["thread_archived"] == true || metadata_val["thread_archived"] == 1);
}

#[tokio::test]
async fn test_thread_unarchive_updates_metadata() {
    let pool = setup_test_db().await;
    let runner = Arc::new(NoopRunner);
    let multiplexer = SessionMultiplexer::new(pool.clone(), runner, MultiplexerConfig::default());
    let data = PoiseData::new(multiplexer.clone(), pool.clone());

    let thread_id = "2345678901";
    let session_key = make_session_key(thread_id);

    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, user_id, metadata, state_json)
         VALUES (?, 'discord', ?, '', '{\"thread_archived\": true}', '{}')",
    )
    .bind(session_key.storage_key())
    .bind(thread_id)
    .execute(&pool)
    .await
    .unwrap();

    let thread_channel: GuildChannel = serde_json::from_value(serde_json::json!({
        "id": thread_id,
        "name": "test-thread",
        "type": 11,
        "guild_id": "111",
        "position": 0,
        "permission_overwrites": [],
        "thread_metadata": {
            "archived": false,
            "auto_archive_duration": 60,
            "archive_timestamp": "2026-10-07T00:00:00Z",
            "locked": false
        }
    }))
    .unwrap();

    handle_thread_update(&data, &thread_channel).await;

    let row = sqlx::query("SELECT metadata FROM sessions WHERE session_key = ?")
        .bind(session_key.storage_key())
        .fetch_one(&pool)
        .await
        .unwrap();

    let metadata_str: String = row.get("metadata");
    let metadata_val: serde_json::Value = serde_json::from_str(&metadata_str).unwrap();
    assert!(metadata_val["thread_archived"] == false || metadata_val["thread_archived"] == 0);
}

#[tokio::test]
async fn test_thread_delete_cleans_up_omo_thread_id_preserving_session() {
    let pool = setup_test_db().await;
    let runner = Arc::new(NoopRunner);
    let multiplexer = SessionMultiplexer::new(pool.clone(), runner, MultiplexerConfig::default());
    let data = PoiseData::new(multiplexer.clone(), pool.clone());

    let thread_id = "3456789012";
    let session_key = make_session_key(thread_id);

    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, user_id, metadata, state_json)
         VALUES (?, 'discord', ?, '', '{\"omo_thread_id\": \"omo-thread-xyz\", \"custom_keep\": \"keep_val\"}', '{}')",
    )
    .bind(session_key.storage_key())
    .bind(thread_id)
    .execute(&pool)
    .await
    .unwrap();

    let partial_channel: PartialGuildChannel = serde_json::from_value(serde_json::json!({
        "id": thread_id,
        "name": "deleted-thread",
        "type": 11,
        "guild_id": "111",
        "parent_id": null
    }))
    .unwrap();

    handle_thread_delete(&data, &partial_channel).await;

    let row = sqlx::query("SELECT session_key, metadata FROM sessions WHERE session_key = ?")
        .bind(session_key.storage_key())
        .fetch_one(&pool)
        .await
        .unwrap();

    let stored_key: String = row.get("session_key");
    assert_eq!(stored_key, session_key.storage_key());

    let metadata_str: String = row.get("metadata");
    let metadata_val: serde_json::Value = serde_json::from_str(&metadata_str).unwrap();
    assert!(metadata_val.get("omo_thread_id").is_none());
    assert_eq!(metadata_val["custom_keep"], "keep_val");
}

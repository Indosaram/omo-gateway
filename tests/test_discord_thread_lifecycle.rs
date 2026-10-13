use omon_gateway::discord::adapter::{handle_thread_delete, handle_thread_update};
use omon_gateway::discord::commands::PoiseData;
use omon_gateway::{
    AgentRunner, InboundEvent, MultiplexerConfig, OmonError, SessionContext, SessionKey,
    SessionMultiplexer,
};
use poise::serenity_prelude as serenity;
use serenity::all::{GuildChannel, PartialGuildChannel};
use sqlx::SqlitePool;
use std::sync::Arc;

const BOT_ID: u64 = 999;
const GUILD_ID: &str = "111";

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
    omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .expect("in-memory db init")
}

fn thread_key(thread_id: &str, parent_id: Option<&str>) -> SessionKey {
    SessionKey::new(
        "discord",
        Some(GUILD_ID),
        parent_id.unwrap_or(thread_id),
        Some(thread_id),
        "",
    )
    .with_bot_id(BOT_ID.to_string())
}

async fn insert_session(pool: &SqlitePool, key: &SessionKey, state_json: &str) {
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json)
         VALUES (?, 'discord', ?, ?, ?, '', ?)",
    )
    .bind(key.storage_key())
    .bind(&key.guild_id)
    .bind(&key.channel_id)
    .bind(&key.thread_id)
    .bind(state_json)
    .execute(pool)
    .await
    .unwrap();
}

async fn session_state(pool: &SqlitePool, key: &SessionKey) -> serde_json::Value {
    let state: String = sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
        .bind(key.storage_key())
        .fetch_one(pool)
        .await
        .unwrap();
    serde_json::from_str(&state).unwrap()
}

fn thread_channel(thread_id: &str, archived: bool) -> GuildChannel {
    serde_json::from_value(serde_json::json!({
        "id": thread_id,
        "name": "test-thread",
        "type": 11,
        "guild_id": GUILD_ID,
        "position": 0,
        "permission_overwrites": [],
        "thread_metadata": {
            "archived": archived,
            "auto_archive_duration": 60,
            "archive_timestamp": "2026-10-07T00:00:00Z",
            "locked": false
        }
    }))
    .unwrap()
}

fn test_data(pool: &SqlitePool) -> PoiseData {
    let multiplexer = SessionMultiplexer::new(
        pool.clone(),
        Arc::new(NoopRunner),
        MultiplexerConfig::default(),
    );
    PoiseData::new(multiplexer, pool.clone())
}

#[tokio::test]
async fn test_thread_archive_updates_metadata_and_stops_multiplexer() {
    let pool = setup_test_db().await;
    let data = test_data(&pool);
    let key = thread_key("1234567890", None);
    insert_session(&pool, &key, r#"{"metadata":{"thread_archived":false}}"#).await;

    handle_thread_update(&data, &thread_channel("1234567890", true), BOT_ID).await;

    let state = session_state(&pool, &key).await;
    assert_eq!(
        state["metadata"]["thread_archived"],
        serde_json::json!(true)
    );
}

#[tokio::test]
async fn test_thread_unarchive_updates_metadata() {
    let pool = setup_test_db().await;
    let data = test_data(&pool);
    let key = thread_key("2345678901", None);
    insert_session(&pool, &key, r#"{"metadata":{"thread_archived":true}}"#).await;

    handle_thread_update(&data, &thread_channel("2345678901", false), BOT_ID).await;

    let state = session_state(&pool, &key).await;
    assert_eq!(
        state["metadata"]["thread_archived"],
        serde_json::json!(false)
    );
}

#[tokio::test]
async fn test_thread_delete_cleans_up_omo_thread_id_preserving_session() {
    let pool = setup_test_db().await;
    let data = test_data(&pool);
    let key = thread_key("3456789012", Some("111000"));
    insert_session(
        &pool,
        &key,
        r#"{"metadata":{"omo_thread_id":"omo-thread-xyz","custom_keep":"keep_val"}}"#,
    )
    .await;

    let partial_channel: PartialGuildChannel = serde_json::from_value(serde_json::json!({
        "id": "3456789012",
        "name": "deleted-thread",
        "type": 11,
        "guild_id": GUILD_ID,
        "parent_id": "111000"
    }))
    .unwrap();

    handle_thread_delete(&data, &partial_channel, BOT_ID).await;

    let state = session_state(&pool, &key).await;
    assert!(state["metadata"].get("omo_thread_id").is_none());
    assert_eq!(state["metadata"]["custom_keep"], "keep_val");
}

//! Slack conversation lifecycle: archiving or deleting a channel, or deleting a thread root,
//! stops the affected turns and updates session state with the same SQL as Discord's
//! `handle_thread_update` / `handle_thread_delete`.

use serde_json::Value;
use sqlx::SqlitePool;

use super::PLATFORM;
use crate::{Result, SessionKey, SessionMultiplexer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Archived { channel: String, archived: bool },
    ChannelDeleted { channel: String },
    ThreadRootDeleted { channel: String, thread_ts: String },
}

pub fn classify(event: &Value) -> Option<Lifecycle> {
    let field = |name: &str| event.get(name).and_then(Value::as_str).map(str::to_owned);
    match field("type")?.as_str() {
        "channel_archive" | "group_archive" => Some(Lifecycle::Archived {
            channel: field("channel")?,
            archived: true,
        }),
        "channel_unarchive" | "group_unarchive" => Some(Lifecycle::Archived {
            channel: field("channel")?,
            archived: false,
        }),
        "channel_deleted" | "group_deleted" => Some(Lifecycle::ChannelDeleted {
            channel: field("channel")?,
        }),
        "message" if field("subtype").as_deref() == Some("message_deleted") => {
            Some(Lifecycle::ThreadRootDeleted {
                channel: field("channel")?,
                thread_ts: field("deleted_ts")?,
            })
        }
        _ => None,
    }
}

/// `Ok(true)` when the event was a lifecycle event (handled or not relevant to any session);
/// such events never reach message routing.
pub async fn handle(
    multiplexer: &SessionMultiplexer,
    pool: &SqlitePool,
    bot_user_id: &str,
    event: &Value,
) -> Result<bool> {
    let Some(lifecycle) = classify(event) else {
        return Ok(false);
    };
    match lifecycle {
        Lifecycle::Archived { channel, archived } => {
            for key in sessions(pool, bot_user_id, &channel, None).await? {
                if archived {
                    stop(multiplexer, &key, "archived").await;
                }
                if let Err(error) = sqlx::query(
                    "UPDATE sessions SET state_json = json_set(COALESCE(state_json, '{}'), '$.metadata.thread_archived', json(?)) WHERE session_key = ?",
                )
                .bind(if archived { "true" } else { "false" })
                .bind(key.storage_key())
                .execute(pool)
                .await
                {
                    tracing::warn!(session = %key, %error, "failed to persist Slack archived state");
                }
            }
        }
        Lifecycle::ChannelDeleted { channel } => {
            for key in sessions(pool, bot_user_id, &channel, None).await? {
                clear_binding(multiplexer, pool, &key).await;
            }
        }
        Lifecycle::ThreadRootDeleted { channel, thread_ts } => {
            for key in sessions(pool, bot_user_id, &channel, Some(&thread_ts)).await? {
                clear_binding(multiplexer, pool, &key).await;
            }
        }
    }
    Ok(true)
}

async fn stop(multiplexer: &SessionMultiplexer, key: &SessionKey, why: &str) {
    if let Err(error) = multiplexer.stop(key).await {
        tracing::warn!(session = %key, %error, why, "failed to stop Slack turn");
    }
}

async fn clear_binding(multiplexer: &SessionMultiplexer, pool: &SqlitePool, key: &SessionKey) {
    stop(multiplexer, key, "deleted").await;
    if let Err(error) = sqlx::query(
        "UPDATE sessions SET state_json = json_remove(COALESCE(state_json, '{}'), '$.metadata.omo_thread_id') WHERE session_key = ?",
    )
    .bind(key.storage_key())
    .execute(pool)
    .await
    {
        tracing::warn!(session = %key, %error, "failed to clear deleted Slack thread binding");
    }
}

/// Sessions of this bot in `channel`; only the thread `thread_ts` when given.
async fn sessions(
    pool: &SqlitePool,
    bot_user_id: &str,
    channel: &str,
    thread_ts: Option<&str>,
) -> Result<Vec<SessionKey>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT session_key FROM sessions WHERE platform = ? AND channel_id = ? AND (? IS NULL OR thread_id = ?)",
    )
    .bind(PLATFORM)
    .bind(channel)
    .bind(thread_ts)
    .bind(thread_ts)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .filter_map(|raw| SessionKey::from_storage_key(raw).ok())
        .filter(|key| key.bot_id.as_deref() == Some(bot_user_id))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use super::*;
    use crate::{Database, InboundEvent, MultiplexerConfig};

    struct Idle;

    #[async_trait]
    impl crate::AgentRunner for Idle {
        async fn run(&self, _: &mut crate::SessionContext, _: InboundEvent) -> Result<()> {
            Ok(())
        }
    }

    async fn insert(pool: &SqlitePool, key: &SessionKey, state: &str) {
        sqlx::query("INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(key.storage_key())
            .bind(&key.platform)
            .bind(&key.guild_id)
            .bind(&key.channel_id)
            .bind(&key.thread_id)
            .bind(&key.user_id)
            .bind(state)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn state(pool: &SqlitePool, key: &SessionKey) -> Value {
        let raw: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(key.storage_key())
                .fetch_one(pool)
                .await
                .unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    fn thread(channel: &str, ts: &str, bot: &str) -> SessionKey {
        SessionKey::new("slack", Some("T1"), channel, Some(ts), "").with_bot_id(bot)
    }

    #[test]
    fn classify_maps_lifecycle_events_only() {
        assert_eq!(
            classify(&json!({"type": "channel_archive", "channel": "C1"})),
            Some(Lifecycle::Archived {
                channel: "C1".into(),
                archived: true
            })
        );
        assert_eq!(
            classify(&json!({"type": "group_archive", "channel": "G1"})),
            Some(Lifecycle::Archived {
                channel: "G1".into(),
                archived: true
            })
        );
        assert_eq!(
            classify(&json!({"type": "channel_unarchive", "channel": "C1"})),
            Some(Lifecycle::Archived {
                channel: "C1".into(),
                archived: false
            })
        );
        assert_eq!(
            classify(&json!({"type": "channel_deleted", "channel": "C1"})),
            Some(Lifecycle::ChannelDeleted {
                channel: "C1".into()
            })
        );
        assert_eq!(
            classify(
                &json!({"type": "message", "subtype": "message_deleted", "channel": "C1", "deleted_ts": "1.0"})
            ),
            Some(Lifecycle::ThreadRootDeleted {
                channel: "C1".into(),
                thread_ts: "1.0".into()
            })
        );
        assert_eq!(
            classify(&json!({"type": "message", "channel": "C1", "ts": "1.0"})),
            None
        );
        assert_eq!(
            classify(&json!({"type": "app_mention", "channel": "C1"})),
            None
        );
    }

    #[tokio::test]
    async fn archive_marks_bot_sessions_and_delete_clears_thread_binding() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let multiplexer =
            SessionMultiplexer::new(pool.clone(), Arc::new(Idle), MultiplexerConfig::default());
        let ours = thread("C1", "1.0", "UBOT");
        let other_thread = thread("C1", "2.0", "UBOT");
        let other_bot = thread("C1", "1.0", "UOTHER");
        let elsewhere = thread("C2", "1.0", "UBOT");
        let bound = r#"{"metadata":{"omo_thread_id":"thr-1"}}"#;
        for key in [&ours, &other_thread, &other_bot, &elsewhere] {
            insert(&pool, key, bound).await;
        }

        let archive = json!({"type": "channel_archive", "channel": "C1"});
        assert!(handle(&multiplexer, &pool, "UBOT", &archive).await.unwrap());
        assert_eq!(
            state(&pool, &ours).await["metadata"]["thread_archived"],
            json!(true)
        );
        assert_eq!(
            state(&pool, &other_thread).await["metadata"]["thread_archived"],
            json!(true)
        );
        assert!(state(&pool, &other_bot).await["metadata"]
            .get("thread_archived")
            .is_none());
        assert!(state(&pool, &elsewhere).await["metadata"]
            .get("thread_archived")
            .is_none());

        let unarchive = json!({"type": "channel_unarchive", "channel": "C1"});
        assert!(handle(&multiplexer, &pool, "UBOT", &unarchive)
            .await
            .unwrap());
        assert_eq!(
            state(&pool, &ours).await["metadata"]["thread_archived"],
            json!(false)
        );

        let root_deleted = json!({"type": "message", "subtype": "message_deleted", "channel": "C1", "deleted_ts": "1.0"});
        assert!(handle(&multiplexer, &pool, "UBOT", &root_deleted)
            .await
            .unwrap());
        assert!(state(&pool, &ours).await["metadata"]
            .get("omo_thread_id")
            .is_none());
        assert_eq!(
            state(&pool, &other_thread).await["metadata"]["omo_thread_id"],
            "thr-1"
        );
        assert_eq!(
            state(&pool, &other_bot).await["metadata"]["omo_thread_id"],
            "thr-1"
        );

        let deleted = json!({"type": "channel_deleted", "channel": "C1"});
        assert!(handle(&multiplexer, &pool, "UBOT", &deleted).await.unwrap());
        assert!(state(&pool, &other_thread).await["metadata"]
            .get("omo_thread_id")
            .is_none());
        assert_eq!(
            state(&pool, &elsewhere).await["metadata"]["omo_thread_id"],
            "thr-1"
        );

        let plain = json!({"type": "message", "channel": "C1", "ts": "3.0", "text": "hi"});
        assert!(!handle(&multiplexer, &pool, "UBOT", &plain).await.unwrap());
    }
}

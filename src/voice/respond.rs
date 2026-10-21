//! Routes a voice transcript through the existing session multiplexer.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::pipeline::VoiceLanguageModel;
use crate::{InboundEvent, OmonError, Result, SessionKey, SessionMultiplexer};

/// Asks the session actor that already serves this channel, then reads the
/// assistant text that turn stored. There is no second model client.
pub struct MultiplexerVoiceModel {
    multiplexer: SessionMultiplexer,
    pool: SqlitePool,
    /// Discord voice channel id -> text session the reply should continue.
    sessions: Arc<parking_lot::Mutex<std::collections::HashMap<u64, SessionKey>>>,
}

impl MultiplexerVoiceModel {
    pub fn new(
        multiplexer: SessionMultiplexer,
        pool: SqlitePool,
        sessions: Arc<parking_lot::Mutex<std::collections::HashMap<u64, SessionKey>>>,
    ) -> Self {
        Self {
            multiplexer,
            pool,
            sessions,
        }
    }

    pub async fn respond_with_speaker(
        &self,
        channel_id: u64,
        speaker: Option<u64>,
        transcript: &str,
    ) -> Result<String> {
        let session = self
            .sessions
            .lock()
            .get(&channel_id)
            .cloned()
            .ok_or_else(|| {
                OmonError::Config(format!(
                    "no text session is bound to voice channel {channel_id}"
                ))
            })?;
        let event = voice_turn_event(session.clone(), speaker, transcript);
        let inbound_id = event.id;
        self.multiplexer.route_awaiting_turn(event).await?;
        assistant_reply_after(&self.pool, &session, inbound_id).await
    }
}

#[async_trait]
impl VoiceLanguageModel for MultiplexerVoiceModel {
    async fn respond(&self, channel_id: u64, transcript: &str) -> Result<String> {
        self.respond_with_speaker(channel_id, None, transcript)
            .await
    }
}

pub fn voice_turn_event(
    session: SessionKey,
    speaker: Option<u64>,
    transcript: &str,
) -> InboundEvent {
    let mut event = InboundEvent::message(session, format!("voice:{}", Uuid::new_v4()), transcript);
    if let Some(speaker) = speaker {
        event = event.with_author_id(speaker.to_string());
    }
    event
}

/// Assistant rows written after the voice turn's own user row, oldest first.
pub async fn assistant_reply_after(
    pool: &SqlitePool,
    session: &SessionKey,
    inbound_id: Uuid,
) -> Result<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT content FROM messages
         WHERE session_key = ? AND role = 'assistant'
           AND sequence > COALESCE((SELECT sequence FROM messages WHERE id = ?), 0)
         ORDER BY sequence ASC",
    )
    .bind(session.storage_key())
    .bind(inbound_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.0)
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[test]
    fn voice_events_carry_the_speaker_and_a_unique_platform_id() {
        let session = SessionKey::new("discord", Some("1"), "2", None::<String>, "");
        let event = voice_turn_event(session, Some(99), "[voice] hi");
        assert_eq!(event.author_id.as_deref(), Some("99"));
        assert!(event.platform_message_id.starts_with("voice:"));
        assert_eq!(event.content, "[voice] hi");
    }

    #[tokio::test]
    async fn assistant_rows_after_the_voice_turn_are_joined_in_order() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let session = SessionKey::new("discord", Some("g"), "c", None::<String>, "");
        let key = session.storage_key();
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id, state_json)
             VALUES (?, 'discord', 'g', 'c', NULL, '', '{}')",
        )
        .bind(&key)
        .execute(db.pool())
        .await
        .unwrap();
        let prior = Uuid::new_v4();
        let user = Uuid::new_v4();
        for (id, role, content) in [(prior, "assistant", "before"), (user, "user", "heard")] {
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, ?, ?)",
            )
            .bind(id.to_string())
            .bind(&key)
            .bind(role)
            .bind(content)
            .execute(db.pool())
            .await
            .unwrap();
        }
        for content in ["first", "second"] {
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content) VALUES (?, ?, 'assistant', ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&key)
            .bind(content)
            .execute(db.pool())
            .await
            .unwrap();
        }
        let reply = assistant_reply_after(db.pool(), &session, user)
            .await
            .unwrap();
        assert_eq!(reply, "first\n\nsecond");
    }
}

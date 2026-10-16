use std::time::Duration;

use chrono::Utc;

use super::control::SessionControl;
use super::overview::{collect, format_overview, DISCORD_LIMIT};
use crate::{InboundEvent, SessionKey};

pub const REPLY_TIMEOUT: Duration = Duration::from_secs(25);

pub fn matches_intent(message: &str) -> bool {
    if message.chars().count() > 80 || message.contains(['\n', '\r']) {
        return false;
    }
    let normalized = message
        .trim()
        .trim_end_matches(['?', '!', '.', '？'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    matches!(
        normalized.as_str(),
        "세션들 상태"
            | "지금 돌아가는 세션들 상태 알려줘"
            | "뭐 돌아가"
            | "세션 상태"
            | "세션 상태 알려줘"
            | "세션들 상태 알려줘"
            | "session status"
            | "show session status"
            | "session status please"
    )
}

pub fn owner_id() -> String {
    std::env::var("DISCORD_ALLOWED_USERS")
        .ok()
        .and_then(|value| {
            value
                .split(',')
                .map(str::trim)
                .find(|id| !id.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| crate::inbox::proposals::OWNER_ID.to_owned())
}

pub fn owner_dm(key: &SessionKey, author: &str, owner: &str) -> bool {
    author == owner
        && key.guild_id.is_none()
        && ((key.platform == "discord" && key.user_id == owner)
            || (key.platform == "web" && key.user_id == "dashboard"))
}

pub fn eligible(event: &InboundEvent, private_owner_conversation: bool) -> bool {
    private_owner_conversation
        && event.origin == crate::EventOrigin::User
        && event.attachments.is_empty()
        && matches_intent(&event.content)
}

pub fn owner_thread(key: &SessionKey, author: &str, owner: &str) -> bool {
    author == owner
        && key.platform == "discord"
        && key.guild_id.is_some()
        && key.thread_id.as_deref().is_some_and(|thread| {
            std::env::var("DISCORD_HOME_CHANNEL_THREAD_ID")
                .is_ok_and(|configured| configured.split(',').map(str::trim).any(|id| id == thread))
        })
        && crate::multiplexer::SessionActor::is_owner_lead_session(key, None)
}

fn compact(value: &str, max: usize) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}

pub async fn control_reply<R: super::registry::CommandRunner + 'static>(
    control: &SessionControl<R>,
) -> String {
    bounded_reply(async {
        let overview = collect(control, 120).await;
        format_overview(&overview, Utc::now(), DISCORD_LIMIT)
    })
    .await
}

pub async fn bounded_reply(fetch: impl std::future::Future<Output = String>) -> String {
    match tokio::time::timeout(REPLY_TIMEOUT, fetch).await {
        Ok(reply) => reply,
        Err(_) => "세션 상태 조회 오류: 25초 제한 시간 초과".into(),
    }
}

pub async fn reply() -> String {
    match SessionControl::from_environment() {
        Ok(control) => control_reply(&control).await,
        Err(error) => format!("세션 상태 조회 오류: {}", compact(&error.to_string(), 120)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::{CommandOutput, CommandRunner};
    use super::*;
    use crate::{
        multiplexer::{AgentRunner, MultiplexerConfig, OutboundDispatcher, SessionMultiplexer},
        Database, OutboundAction, SessionContext,
    };
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn short_whole_message_intents_match() {
        for text in [
            "세션들 상태",
            "지금 돌아가는 세션들 상태 알려줘",
            "뭐 돌아가?",
            "세션 상태",
            "session status",
            " SESSION  STATUS! ",
        ] {
            assert!(matches_intent(text), "{text}");
        }
    }

    #[test]
    fn other_requests_do_not_match() {
        for text in [
            "세션 상태 보고 고쳐줘",
            "session status and stop it",
            "뭐 돌아가는데 문제가 있어",
            "세션 상태\n코드 수정해",
            "session status is broken",
            "",
            "hello",
        ] {
            assert!(!matches_intent(text), "{text}");
        }
    }

    struct FakeRegistry;
    #[async_trait]
    impl CommandRunner for FakeRegistry {
        async fn run(&self, _: &str, _: &[String]) -> std::result::Result<CommandOutput, String> {
            Ok(CommandOutput {
                code: 0,
                stdout: b"[]".to_vec(),
                stderr: vec![],
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_listing_returns_one_line_within_timeout() {
        let started = tokio::time::Instant::now();
        let reply = bounded_reply(std::future::pending()).await;
        assert_eq!(tokio::time::Instant::now() - started, REPLY_TIMEOUT);
        assert_eq!(reply.lines().count(), 1);
        assert!(reply.contains("25초"));
    }

    #[test]
    fn owner_identity_and_attachments_gate_interception() {
        let key = SessionKey::new("discord", None::<String>, "dm", None::<String>, "other");
        assert!(!owner_dm(&key, "other", "owner"));
        let shared = SessionKey::new("discord", Some("guild"), "shared", Some("thread"), "owner");
        assert!(!owner_dm(&shared, "owner", "owner"));
        let event = InboundEvent::message(key, "1", "session status");
        assert!(!eligible(&event, false));
    }

    struct CountingBackend(AtomicUsize);
    #[async_trait]
    impl AgentRunner for CountingBackend {
        async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> crate::Result<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct Replies(AtomicUsize);
    #[async_trait]
    impl OutboundDispatcher for Replies {
        async fn dispatch(&self, action: OutboundAction) -> crate::Result<()> {
            if matches!(action, OutboundAction::SendMessage { .. }) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn owner_dm_direct_reply_records_pair_without_backend_turn() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = Arc::new(CountingBackend(AtomicUsize::new(0)));
        let replies = Arc::new(Replies(AtomicUsize::new(0)));
        let mux = SessionMultiplexer::with_dispatcher(
            db.pool().clone(),
            backend.clone(),
            Some(replies.clone()),
            MultiplexerConfig::default(),
        );
        let key = SessionKey::new("discord", None::<String>, "dm", None::<String>, "owner");
        let event = InboundEvent::message(key.clone(), "1", "세션 상태");
        let control = SessionControl::new(Arc::new(FakeRegistry), vec![]);
        assert!(tokio::time::timeout(
            REPLY_TIMEOUT,
            mux.intercept_session_status_with(
                &event,
                owner_dm(&key, "owner", "owner"),
                control_reply(&control)
            )
        )
        .await
        .unwrap()
        .unwrap());
        assert_eq!(backend.0.load(Ordering::SeqCst), 0);
        assert_eq!(replies.0.load(Ordering::SeqCst), 1);
        let roles: Vec<String> =
            sqlx::query_scalar("SELECT role FROM messages ORDER BY created_at")
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert_eq!(roles, ["user", "assistant"]);
        let state: String = sqlx::query_scalar("SELECT state FROM delivery_obligations")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(state, "delivered");
        db.close().await;
    }

    #[tokio::test]
    async fn shared_channel_and_normal_chat_fall_through() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = Arc::new(CountingBackend(AtomicUsize::new(0)));
        let mux = SessionMultiplexer::new(
            db.pool().clone(),
            backend.clone(),
            MultiplexerConfig::default(),
        );
        for (key, content, private) in [
            (
                SessionKey::new("discord", Some("guild"), "shared", None::<String>, "owner"),
                "session status",
                false,
            ),
            (
                SessionKey::new("discord", None::<String>, "dm", None::<String>, "owner"),
                "fix this bug",
                true,
            ),
        ] {
            let event = InboundEvent::message(key, "", content);
            assert!(!mux
                .intercept_session_status_with(&event, private, async {
                    panic!("registry must not run")
                })
                .await
                .unwrap());
            mux.route_awaiting_turn(event).await.unwrap();
        }
        assert_eq!(backend.0.load(Ordering::SeqCst), 2);
        db.close().await;
    }
}

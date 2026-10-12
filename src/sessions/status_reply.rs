use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::registry::{CommandRunner, SessionList, SessionRegistry};
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

/// The local dashboard is an owner-only transport. Discord guild conversations
/// require the caller to establish thread privacy separately, not just bot ownership.
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

fn home_relative(folder: &str) -> String {
    for prefix in ["/Users/", "/home/"] {
        if let Some(rest) = folder.strip_prefix(prefix) {
            return match rest.split_once('/') {
                Some((_, suffix)) => format!("~/{suffix}"),
                None => "~".into(),
            };
        }
    }
    if folder == "/root" {
        return "~".into();
    }
    folder
        .strip_prefix("/root/")
        .map_or_else(|| folder.to_owned(), |rest| format!("~/{rest}"))
}

fn activity(value: Option<&str>, now: DateTime<Utc>) -> String {
    let Some(at) = value.and_then(|value| DateTime::parse_from_rfc3339(value).ok()) else {
        return "활동 미상 (KST)".into();
    };
    let seconds = now.signed_duration_since(at).num_seconds().max(0);
    let relative = if seconds < 60 {
        format!("{seconds}초 전")
    } else if seconds < 3600 {
        format!("{}분 전", seconds / 60)
    } else if seconds < 86400 {
        format!("{}시간 전", seconds / 3600)
    } else {
        format!("{}일 전", seconds / 86400)
    };
    format!("{relative} (KST)")
}

pub fn format_reply(list: &SessionList, now: DateTime<Utc>) -> String {
    let mut groups = BTreeMap::<&str, Vec<_>>::new();
    for session in &list.sessions {
        groups.entry(&session.machine).or_default().push(session);
    }
    for stale in &list.stale {
        groups.entry(&stale.machine).or_default();
    }
    let mut lines = Vec::new();
    for (machine, sessions) in groups {
        let stale = list
            .stale
            .iter()
            .find(|item| item.machine == machine && item.stale)
            .map(|item| format!(" · stale {}초 전", item.age_seconds))
            .unwrap_or_default();
        lines.push(format!("{}{}", compact(machine, 32), stale));
        let mut counts = BTreeMap::<String, usize>::new();
        for session in sessions {
            let status = session.status.as_deref().unwrap_or("unknown");
            if status.eq_ignore_ascii_case("busy") || status.eq_ignore_ascii_case("working") {
                lines.push(format!(
                    "- {} · {} · {} · {}",
                    compact(session.name.as_deref().unwrap_or(&session.session_id), 24),
                    compact(&home_relative(session.cwd.as_deref().unwrap_or("?")), 40),
                    compact(status, 16),
                    activity(
                        session
                            .updated_at
                            .as_deref()
                            .or(session.last_activity.as_deref()),
                        now
                    )
                ));
            } else {
                *counts.entry(compact(status, 16)).or_default() += 1;
            }
        }
        if !counts.is_empty() {
            lines.push(format!(
                "- {}",
                compact(
                    &counts
                        .into_iter()
                        .map(|(status, count)| format!("{status} {count}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    110
                )
            ));
        }
    }
    for error in &list.errors {
        lines.push(format!(
            "{}: 오류 {}",
            compact(&error.machine, 32),
            compact(&error.message, 70)
        ));
    }
    if lines.is_empty() {
        lines.push("실행 중인 세션 없음".into());
    }
    if lines.len() > 15 {
        let omitted = lines.len() - 14;
        lines.truncate(14);
        lines.push(format!("... {omitted}줄 생략"));
    }
    lines.join("\n")
}

pub async fn registry_reply<R: CommandRunner + 'static>(registry: &SessionRegistry<R>) -> String {
    bounded_reply(registry.list_all()).await
}

async fn bounded_reply(listing: impl std::future::Future<Output = SessionList>) -> String {
    match tokio::time::timeout(REPLY_TIMEOUT, listing).await {
        Ok(list) => format_reply(&list, Utc::now()),
        Err(_) => "세션 상태 조회 오류: 25초 제한 시간 초과".into(),
    }
}

pub async fn reply() -> String {
    match SessionRegistry::from_environment() {
        Ok(registry) => registry_reply(&registry).await,
        Err(error) => format!("세션 상태 조회 오류: {}", compact(&error.to_string(), 120)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::{parse_sessions, CommandOutput, HostError, StaleListing};
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

    #[test]
    fn formatter_groups_counts_stale_errors_and_activity() {
        let now = DateTime::parse_from_rfc3339("2026-10-12T09:00:00+09:00")
            .unwrap()
            .with_timezone(&Utc);
        let list = SessionList {
            sessions: parse_sessions(br#"[{"id":"1","name":"build","cwd":"/home/indo/project","status":"busy","updatedAt":"2026-10-12T08:58:00+09:00"},{"id":"2","status":"idle"},{"id":"3","status":"idle"}]"#, "local").unwrap(),
            errors: vec![HostError { machine: "remote".into(), message: "unreachable\nhost".into(), code: 3 }],
            stale: vec![StaleListing { machine: "local".into(), stale: true, age_seconds: 36 }], exit_code: 0,
        };
        let reply = format_reply(&list, now);
        assert!(reply.contains("~/project"));
        assert!(reply.contains("busy"));
        assert!(reply.contains("2분 전 (KST)"));
        assert!(reply.contains("idle 2"));
        assert!(reply.contains("stale 36"));
        assert!(reply.contains("remote: 오류 unreachable host"));
        assert_eq!(reply.lines().count(), 4);
    }

    #[test]
    fn formatter_caps_lines_and_discord_size() {
        let mut list = SessionList {
            sessions: vec![],
            errors: vec![],
            stale: vec![],
            exit_code: 0,
        };
        for i in 0..40 {
            list.sessions.extend(
                parse_sessions(
                    format!(
                        r#"[{{"id":"{i}","status":"working","name":"{}","cwd":"/Users/indo/{}"}}]"#,
                        "n".repeat(300),
                        "x".repeat(300)
                    )
                    .as_bytes(),
                    "local",
                )
                .unwrap(),
            );
        }
        let reply = format_reply(&list, Utc::now());
        assert_eq!(reply.lines().count(), 15);
        assert!(reply.chars().count() < 2000);
        assert!(reply.contains("생략"));
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
        let registry = SessionRegistry::new(Arc::new(FakeRegistry), vec![]);
        assert!(tokio::time::timeout(
            REPLY_TIMEOUT,
            mux.intercept_session_status_with(
                &event,
                owner_dm(&key, "owner", "owner"),
                registry_reply(&registry)
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

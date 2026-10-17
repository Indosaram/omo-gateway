use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Timelike, Utc};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use super::control::SessionControl;
use super::overview::{collect, compact, short_id};
use super::probe::LiveSession;
use super::registry::CommandRunner;
use crate::{OutboundAction, OutboundDispatcher, Result, SessionKey};

pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
/// A question or blocked session is only reported after it has gone unanswered this long,
/// so the owner is not pinged for questions they are about to answer at the terminal.
pub const DEFAULT_NOTIFY_AFTER: Duration = Duration::from_secs(30 * 60);
/// Owner sleep window in KST (start hour inclusive, end hour exclusive). Nothing is sent
/// inside it; held notices go out after it ends if still unanswered.
pub const DEFAULT_QUIET_HOURS: (u32, u32) = (0, 8);
const KST: TimeDelta = TimeDelta::hours(9);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotifyPolicy {
    pub notify_after: Duration,
    pub quiet_hours: Option<(u32, u32)>,
}

impl Default for NotifyPolicy {
    fn default() -> Self {
        Self {
            notify_after: DEFAULT_NOTIFY_AFTER,
            quiet_hours: Some(DEFAULT_QUIET_HOURS),
        }
    }
}

impl NotifyPolicy {
    /// `OMON_SESSION_NOTIFY_AFTER_SECS` (default 1800) and `OMON_SESSION_QUIET_HOURS`
    /// (`"0-8"` KST by default, `"off"` disables).
    pub fn from_env() -> Self {
        let mut policy = Self::default();
        if let Some(secs) = std::env::var("OMON_SESSION_NOTIFY_AFTER_SECS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
        {
            policy.notify_after = Duration::from_secs(secs);
        }
        if let Ok(value) = std::env::var("OMON_SESSION_QUIET_HOURS") {
            policy.quiet_hours = parse_quiet_hours(&value);
        }
        policy
    }

    fn is_quiet(&self, now: DateTime<Utc>) -> bool {
        let Some((start, end)) = self.quiet_hours else {
            return false;
        };
        let hour = (now + KST).hour();
        if start <= end {
            (start..end).contains(&hour)
        } else {
            hour >= start || hour < end
        }
    }
}

fn parse_quiet_hours(value: &str) -> Option<(u32, u32)> {
    let (start, end) = value.trim().split_once('-')?;
    let start = start.trim().parse::<u32>().ok().filter(|h| *h < 24)?;
    let end = end.trim().parse::<u32>().ok().filter(|h| *h <= 24)?;
    (start != end).then_some((start, end % 24))
}

pub async fn watch(pool: &SqlitePool, target: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO session_watches(target, added_at) VALUES(?, ?)")
        .bind(target)
        .bind(Utc::now())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn unwatch(pool: &SqlitePool, target: &str) -> Result<bool> {
    let removed = sqlx::query("DELETE FROM session_watches WHERE target = ?")
        .bind(target)
        .execute(pool)
        .await?;
    Ok(removed.rows_affected() > 0)
}

pub async fn watched(pool: &SqlitePool) -> Result<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT target FROM session_watches ORDER BY added_at")
            .fetch_all(pool)
            .await?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub dedupe_key: String,
    pub target: String,
    pub kind: String,
    pub text: String,
}

pub struct Watcher<R> {
    pool: SqlitePool,
    control: SessionControl<R>,
    interval: Duration,
    streaming: Option<HashMap<String, bool>>,
    policy: NotifyPolicy,
    first_seen: HashMap<String, DateTime<Utc>>,
    slack_hints: bool,
}

fn display_name(session: &LiveSession) -> &str {
    session
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&session.session_id)
}

fn session_notices(
    session: &LiveSession,
    target: &str,
    watched: bool,
    was_streaming: Option<bool>,
    slack_hints: bool,
) -> Vec<Notice> {
    let machine = &session.machine;
    let name = compact(display_name(session), 40);
    let shown = format!("{machine}:{}", short_id(&session.session_id));
    let mut notices = Vec::new();
    if session.blocking_question && session.questions.is_empty() {
        notices.push(Notice {
            dedupe_key: format!("b:{target}:{}", session.message_count),
            target: target.to_owned(),
            kind: "blocked".into(),
            text: format!(
                "⛔ {machine} · {name}: 터미널에서 답해야 하는 질문을 기다리고 있어요 (`{shown}`)"
            ),
        });
    }
    for pending in &session.questions {
        let first = pending.questions.first();
        let question = first
            .and_then(|item| item.question.as_deref().or(item.header.as_deref()))
            .unwrap_or("질문");
        let options = first
            .map(|item| item.options.join(" / "))
            .filter(|options| !options.is_empty())
            .map(|options| format!("\n선택지: {options}"))
            .unwrap_or_default();
        let question = compact(&question.replace(['\n', '\r'], " "), 160);
        // Keyed by question text, not request id: an agent that re-asks the same question
        // (new request id) must not re-notify.
        let hint = if session.blocking_question {
            "\n→ 터미널에서 답해 주세요".to_owned()
        } else if slack_hints {
            format!("\n→ `/omo-sessions answer {shown} …`")
        } else {
            format!("\n→ `/sessions answer target:{shown} text:…`")
        };
        notices.push(Notice {
            dedupe_key: format!("q:{target}:{}", short_hash(&question)),
            target: target.to_owned(),
            kind: "question".into(),
            text: format!("❓ {machine} · {name}: {question}{options}{hint}"),
        });
    }
    if watched && was_streaming == Some(true) && !session.streaming {
        notices.push(Notice {
            dedupe_key: format!("c:{target}:{}", session.message_count),
            target: target.to_owned(),
            kind: "completion".into(),
            text: format!(
                "✅ {machine} · {name} 턴 완료 (`{shown}`)\n↳ {}",
                compact(&session.last_assistant.replace(['\n', '\r'], " "), 300)
            ),
        });
    }
    notices
}

fn short_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(text.as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

impl<R: CommandRunner + 'static> Watcher<R> {
    pub fn new(pool: SqlitePool, control: SessionControl<R>, interval: Duration) -> Self {
        Self::with_policy(pool, control, interval, NotifyPolicy::default())
    }

    pub fn with_policy(
        pool: SqlitePool,
        control: SessionControl<R>,
        interval: Duration,
        policy: NotifyPolicy,
    ) -> Self {
        Self {
            pool,
            control,
            interval,
            streaming: None,
            policy,
            first_seen: HashMap::new(),
            slack_hints: false,
        }
    }

    /// Notices not yet delivered. Completion needs a streaming -> idle edge, so the first
    /// tick of a process only records baselines; question notices are deduplicated in SQLite
    /// and therefore never repeat across restarts.
    pub async fn tick(&mut self) -> Result<Vec<Notice>> {
        self.tick_at(Utc::now()).await
    }

    /// Questions and blocked sessions are held until they stay unanswered for
    /// `notify_after`; nothing is released during quiet hours.
    pub async fn tick_at(&mut self, now: DateTime<Utc>) -> Result<Vec<Notice>> {
        let overview = collect(&self.control, 300).await;
        let watched = watched(&self.pool).await?;
        let previous = self.streaming.take();
        let mut current = HashMap::new();
        let mut notices = Vec::new();
        for (machine, snapshot) in &overview.snapshots {
            let Ok(snapshot) = snapshot else {
                if let Some(previous) = &previous {
                    let prefix = format!("{machine}:");
                    current.extend(
                        previous
                            .iter()
                            .filter(|(target, _)| target.starts_with(&prefix))
                            .map(|(target, streaming)| (target.clone(), *streaming)),
                    );
                }
                continue;
            };
            for session in &snapshot.sessions {
                let target = format!("{machine}:{}", session.session_id);
                let routed = format!("{machine}:{}", session.routing_id);
                let is_watched = watched.iter().any(|w| *w == target || *w == routed);
                let was = previous.as_ref().and_then(|map| map.get(&target).copied());
                current.insert(target.clone(), session.streaming);
                notices.extend(session_notices(
                    session,
                    &target,
                    is_watched,
                    was,
                    self.slack_hints,
                ));
            }
        }
        self.streaming = Some(current);
        let present: std::collections::HashSet<&str> =
            notices.iter().map(|n| n.dedupe_key.as_str()).collect();
        self.first_seen
            .retain(|key, _| present.contains(key.as_str()));
        let quiet = self.policy.is_quiet(now);
        let wait = TimeDelta::from_std(self.policy.notify_after).unwrap_or(TimeDelta::zero());
        let mut fresh = Vec::new();
        for notice in notices {
            if notice.kind != "completion" {
                let seen = *self
                    .first_seen
                    .entry(notice.dedupe_key.clone())
                    .or_insert(now);
                if now - seen < wait {
                    continue;
                }
            }
            if quiet {
                continue;
            }
            let sent: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM session_watch_notices WHERE dedupe_key = ?)",
            )
            .bind(&notice.dedupe_key)
            .fetch_one(&self.pool)
            .await?;
            if !sent {
                fresh.push(notice);
            }
        }
        Ok(fresh)
    }

    pub async fn mark_sent(&self, notice: &Notice) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO session_watch_notices(dedupe_key, target, kind, sent_at) VALUES(?, ?, ?, ?)",
        )
        .bind(&notice.dedupe_key)
        .bind(&notice.target)
        .bind(&notice.kind)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn serve(
        mut self,
        dispatcher: Arc<dyn OutboundDispatcher>,
        owner_dm: SessionKey,
        shutdown: CancellationToken,
    ) -> Result<()> {
        self.slack_hints = owner_dm.platform == crate::slack::PLATFORM;
        tracing::info!(
            interval_secs = self.interval.as_secs(),
            "session watcher started"
        );
        let mut ticker = tokio::time::interval(self.interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = ticker.tick() => {}
            }
            let notices = match self.tick().await {
                Ok(notices) => notices,
                Err(error) => {
                    tracing::warn!(%error, "session watcher tick failed; will retry");
                    continue;
                }
            };
            for notice in notices {
                let action = OutboundAction::SendMessage {
                    session: owner_dm.clone(),
                    content: notice.text.clone(),
                    reply_to: None,
                };
                match dispatcher.dispatch(action).await {
                    Ok(()) => {
                        if let Err(error) = self.mark_sent(&notice).await {
                            tracing::warn!(%error, key = %notice.dedupe_key, "session notice sent but not recorded");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, key = %notice.dedupe_key, "session notice delivery failed; will retry next tick");
                    }
                }
            }
        }
    }
}

pub fn interval_from_env() -> Duration {
    std::env::var("OMON_SESSION_WATCH_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| *secs >= 10)
        .map_or(DEFAULT_INTERVAL, Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::super::probe::{PendingQuestion, QuestionItem};
    use super::super::registry::CommandOutput;
    use super::*;
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use serde_json::json;

    struct Scripted {
        snapshot: Mutex<serde_json::Value>,
        remote_down: bool,
    }

    #[async_trait]
    impl CommandRunner for Scripted {
        async fn run(
            &self,
            _machine: &str,
            argv: &[String],
        ) -> std::result::Result<CommandOutput, String> {
            let remote = argv.first().is_some_and(|word| word == "ssh");
            if argv.iter().any(|word| word.contains("list"))
                && argv.iter().any(|word| word.contains("thread"))
            {
                return Ok(CommandOutput {
                    code: 0,
                    stdout: b"[]".to_vec(),
                    stderr: Vec::new(),
                });
            }
            if remote && self.remote_down {
                return Err("ssh: connect timed out".into());
            }
            let body = if remote {
                json!({"sessions": [], "errors": []})
            } else {
                self.snapshot.lock().clone()
            };
            Ok(CommandOutput {
                code: 0,
                stdout: body.to_string().into_bytes(),
                stderr: Vec::new(),
            })
        }
    }

    fn live(streaming: bool, questions: bool, messages: u64) -> serde_json::Value {
        let pending = if questions {
            json!([{"request_id": "toolu_1", "questions": [{"id": "q1", "header": "QA", "question": "Pick one", "options": ["Alpha", "Beta"]}]}])
        } else {
            json!([])
        };
        json!({"sessions": [{
            "session_id": "01a1429e-8585-7e8a-b89b-2ea8458e4312", "routing_id": "rpc-2", "endpoint_kind": "rpc_host",
            "name": "qa", "cwd": "/tmp/qa", "streaming": streaming, "compacting": false,
            "blocking_question": false, "pending_messages": 0, "message_count": messages,
            "last_user": "", "last_assistant": "finished the task", "questions": pending
        }], "errors": []})
    }

    async fn pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn watcher(pool: &SqlitePool, runner: Arc<Scripted>) -> Watcher<Scripted> {
        policy_watcher(
            pool,
            runner,
            NotifyPolicy {
                notify_after: Duration::ZERO,
                quiet_hours: None,
            },
        )
    }

    fn policy_watcher(
        pool: &SqlitePool,
        runner: Arc<Scripted>,
        policy: NotifyPolicy,
    ) -> Watcher<Scripted> {
        Watcher::with_policy(
            pool.clone(),
            SessionControl::new(runner, vec!["omarchy".into()]),
            DEFAULT_INTERVAL,
            policy,
        )
    }

    fn kst(hour: u32, minute: u32) -> DateTime<Utc> {
        use chrono::TimeZone;
        Utc.with_ymd_and_hms(2026, 10, 16, hour, minute, 0).unwrap() - KST
    }

    #[tokio::test]
    async fn question_is_held_until_unanswered_for_notify_after() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(false, true, 3)),
            remote_down: false,
        });
        let mut watcher = policy_watcher(&pool, runner.clone(), NotifyPolicy::default());
        assert!(watcher.tick_at(kst(14, 0)).await.unwrap().is_empty());
        assert!(watcher.tick_at(kst(14, 29)).await.unwrap().is_empty());
        let notices = watcher.tick_at(kst(14, 30)).await.unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, "question");

        // Answered before the wait ran out: never sent.
        let mut answered = policy_watcher(&pool, runner.clone(), NotifyPolicy::default());
        *runner.snapshot.lock() = live(false, true, 3);
        answered.tick_at(kst(15, 0)).await.unwrap();
        *runner.snapshot.lock() = live(false, false, 4);
        assert!(answered.tick_at(kst(15, 40)).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn quiet_hours_hold_notices_until_morning() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(false, true, 3)),
            remote_down: false,
        });
        let mut watcher = policy_watcher(&pool, runner, NotifyPolicy::default());
        assert!(watcher.tick_at(kst(1, 0)).await.unwrap().is_empty());
        assert!(watcher.tick_at(kst(3, 0)).await.unwrap().is_empty());
        assert!(watcher.tick_at(kst(7, 59)).await.unwrap().is_empty());
        assert_eq!(watcher.tick_at(kst(8, 0)).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn reasking_the_same_question_does_not_notify_again() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(false, true, 3)),
            remote_down: false,
        });
        let mut watcher = watcher(&pool, runner.clone());
        let notices = watcher.tick().await.unwrap();
        assert_eq!(notices.len(), 1);
        watcher.mark_sent(&notices[0]).await.unwrap();
        let mut reasked = live(false, true, 9);
        reasked["sessions"][0]["questions"][0]["request_id"] = json!("toolu_2");
        *runner.snapshot.lock() = reasked;
        assert!(watcher.tick().await.unwrap().is_empty());
    }

    #[test]
    fn quiet_hours_parse_and_wrap_midnight() {
        assert_eq!(parse_quiet_hours("0-8"), Some((0, 8)));
        assert_eq!(parse_quiet_hours("23-7"), Some((23, 7)));
        assert_eq!(parse_quiet_hours("off"), None);
        let policy = NotifyPolicy {
            notify_after: Duration::ZERO,
            quiet_hours: Some((23, 7)),
        };
        assert!(policy.is_quiet(kst(23, 30)));
        assert!(policy.is_quiet(kst(6, 59)));
        assert!(!policy.is_quiet(kst(7, 0)));
    }

    #[tokio::test]
    async fn question_notifies_once_and_survives_restart() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(false, true, 3)),
            remote_down: false,
        });
        let mut first = watcher(&pool, runner.clone());
        let notices = first.tick().await.unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, "question");
        assert!(notices[0].text.contains("Alpha / Beta"));
        first.mark_sent(&notices[0]).await.unwrap();
        assert!(first.tick().await.unwrap().is_empty());
        let mut restarted = watcher(&pool, runner);
        assert!(restarted.tick().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn completion_only_for_watched_sessions_after_a_baseline() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(true, false, 5)),
            remote_down: false,
        });
        let mut unwatched = watcher(&pool, runner.clone());
        assert!(unwatched.tick().await.unwrap().is_empty());
        *runner.snapshot.lock() = live(false, false, 7);
        assert!(unwatched.tick().await.unwrap().is_empty());

        watch(&pool, "local:01a1429e-8585-7e8a-b89b-2ea8458e4312")
            .await
            .unwrap();
        *runner.snapshot.lock() = live(false, false, 7);
        let mut fresh = watcher(&pool, runner.clone());
        assert!(
            fresh.tick().await.unwrap().is_empty(),
            "first tick only records baselines"
        );
        *runner.snapshot.lock() = live(true, false, 8);
        assert!(fresh.tick().await.unwrap().is_empty());
        *runner.snapshot.lock() = live(false, false, 9);
        let notices = fresh.tick().await.unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].kind, "completion");
        assert!(notices[0].text.contains("finished the task"));
        assert!(unwatch(&pool, "local:01a1429e-8585-7e8a-b89b-2ea8458e4312")
            .await
            .unwrap());
        assert!(watched(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn unreachable_host_does_not_hide_local_notices() {
        let pool = pool().await;
        let runner = Arc::new(Scripted {
            snapshot: Mutex::new(live(false, true, 3)),
            remote_down: true,
        });
        let notices = watcher(&pool, runner).tick().await.unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(
            notices[0].target,
            "local:01a1429e-8585-7e8a-b89b-2ea8458e4312"
        );
    }

    #[test]
    fn blocked_question_tells_owner_to_use_the_terminal() {
        let session = LiveSession {
            machine: "omarchy".into(),
            session_id: "01a13dca-860b-799b".into(),
            blocking_question: true,
            message_count: 4,
            questions: vec![PendingQuestion {
                request_id: "r".into(),
                questions: vec![QuestionItem::default()],
            }],
            ..LiveSession::default()
        };
        let notices = session_notices(&session, "omarchy:01a13dca-860b-799b", false, None, false);
        assert_eq!(
            notices.iter().map(|n| n.kind.as_str()).collect::<Vec<_>>(),
            ["question"],
            "one message per question, not a second blocked ping"
        );
        assert!(notices[0].text.contains("터미널"));
        assert!(!notices[0].text.contains("/sessions answer"));
    }

    #[test]
    fn question_hint_uses_slack_command_syntax_for_slack_owner_dm() {
        let session = LiveSession {
            machine: "local".into(),
            session_id: "01a13dca-860b-799b".into(),
            questions: vec![PendingQuestion {
                request_id: "r".into(),
                questions: vec![QuestionItem::default()],
            }],
            ..LiveSession::default()
        };
        let target = "local:01a13dca-860b-799b";
        let slack = session_notices(&session, target, false, None, true);
        assert!(
            slack[0].text.contains("/omo-sessions answer local:"),
            "{}",
            slack[0].text
        );
        assert!(!slack[0].text.contains("target:"));
        let discord = session_notices(&session, target, false, None, false);
        assert!(
            discord[0].text.contains("/sessions answer target:local:"),
            "{}",
            discord[0].text
        );
        assert_eq!(slack[0].dedupe_key, discord[0].dedupe_key);
    }
}

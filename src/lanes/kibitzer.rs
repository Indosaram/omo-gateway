//! Event-armed owner-intent evaluation. Producers arm in their commit transaction,
//! then hint the existing lane socket; startup owns this worker and its Notify.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::agent::{model_for_purpose, AgentBackend, ModelPurpose, OmoBackend, OmoBackendConfig};
use crate::judge::{scrub_for_external, JevCli, Judge, Purpose};
use crate::memory::MemoryStore;
use crate::multiplexer::OutboundDispatcher;
use crate::work_heartbeat::{Clock, SystemClock};
use crate::{InboundEvent, OmonError, OutboundAction, Result, SessionContext, SessionKey};

pub const DEFAULT_LEAD: &str =
    "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451";
const SPACING: TimeDelta = TimeDelta::minutes(30);
const KST: TimeDelta = TimeDelta::hours(9);
const TURN_TIMEOUT: Duration = Duration::from_secs(60);
// The judge truncates its input to 4000 chars (judge::scrub_for_external).
const DOC_CAP: usize = 4000;
const DM_BUDGET: usize = 1600;
// SQL triggers can arm from another process, which cannot signal our Notify.
// This is a local durable-state check, never an idle judge/model invocation.
const ARM_SCAN_INTERVAL: Duration = Duration::from_secs(5);

// A newtype forces the nullable field to be present (Option alone permits omission).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct TargetLane(pub Option<String>);

impl<'de> Deserialize<'de> for TargetLane {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        serde_json::from_value(value)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub wake: bool,
    pub reason: String,
    pub suggested: String,
    pub target_lane: TargetLane,
}

pub fn parse_decision(raw: &str) -> std::result::Result<Decision, serde_json::Error> {
    serde_json::from_str(strip_code_fence(raw.trim()))
}

/// A model often wraps its JSON in a Markdown fence; only a whole-reply fence is removed.
fn strip_code_fence(text: &str) -> &str {
    let Some(inner) = text.strip_prefix("```").and_then(|t| t.strip_suffix("```")) else {
        return text;
    };
    let inner = inner.strip_prefix("json").unwrap_or(inner);
    inner.trim()
}

/// Task-local sink: completed stream snapshots only, never a Discord transport.
#[derive(Default)]
pub struct Capture {
    text: parking_lot::Mutex<String>,
    streams: parking_lot::Mutex<HashMap<uuid::Uuid, super::activity::StreamCapture>>,
}

#[async_trait]
impl OutboundDispatcher for Capture {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action {
            OutboundAction::SendMessage { content, .. }
            | OutboundAction::EditMessage { content, .. } => *self.text.lock() = content,
            OutboundAction::Stream { chunk, .. } => {
                let mut streams = self.streams.lock();
                if let Some(text) = streams.entry(chunk.stream_id).or_default().accept(&chunk) {
                    *self.text.lock() = text.to_owned();
                }
            }
            OutboundAction::Typing { .. }
            | OutboundAction::React { .. }
            | OutboundAction::DeleteMessage { .. }
            | OutboundAction::ExpireApproval { .. } => {}
            OutboundAction::UploadFile { .. } | OutboundAction::ApprovalRequest { .. } => {
                return Err(OmonError::Config(
                    "kibitzer cannot request external actions".into(),
                ));
            }
        }
        Ok(())
    }
}

/// The owner's home DM: the verified Discord DM, or a Slack DM with the Slack owner.
fn is_owner_home_dm(lead: &SessionKey, slack_owner: &str) -> bool {
    if lead.guild_id.is_some() || lead.thread_id.is_some() {
        return false;
    }
    match lead.platform.as_str() {
        "discord" => {
            lead.user_id == "414011632306618368"
                && lead.bot_id.as_deref() == Some("1465631383862120451")
                && lead.channel_id == "1474231559895646240"
        }
        crate::slack::PLATFORM => {
            lead.user_id == slack_owner
                && lead.bot_id.as_deref().is_some_and(|bot| !bot.is_empty())
                && !lead.channel_id.is_empty()
        }
        _ => false,
    }
}

pub struct Kibitzer {
    pool: SqlitePool,
    lead: SessionKey,
    judge: Arc<dyn Judge>,
    backend: Arc<dyn AgentBackend>,
    capture: Arc<Capture>,
    clock: Arc<dyn Clock>,
    enabled: bool,
}

impl Kibitzer {
    pub fn new(
        pool: SqlitePool,
        lead: SessionKey,
        judge: Arc<dyn Judge>,
        backend: Arc<dyn AgentBackend>,
        capture: Arc<Capture>,
        clock: Arc<dyn Clock>,
        enabled: bool,
    ) -> Result<Self> {
        if !is_owner_home_dm(&lead, &crate::slack::owner_id()) {
            return Err(OmonError::Config(
                "kibitzer lead must be the verified owner home DM".into(),
            ));
        }
        Ok(Self {
            pool,
            lead,
            judge,
            backend,
            capture,
            clock,
            enabled,
        })
    }

    pub fn from_env(pool: SqlitePool) -> Result<Self> {
        let enabled = std::env::var("OMON_KIBITZER_ENABLED")
            .map(|value| value != "false")
            .unwrap_or(true);
        tracing::info!(
            enabled,
            judge_api_key_present = std::env::var_os("TYPESAFE_API_KEY").is_some(),
            judge_url_present = std::env::var_os("TYPESAFE_URL").is_some(),
            judge_model_present = std::env::var_os("TYPESAFE_MODEL").is_some(),
            "kibitzer configured"
        );
        let lead = SessionKey::from_storage_key(
            &std::env::var("OMON_LANE_DEFAULT_LEAD").unwrap_or_else(|_| DEFAULT_LEAD.into()),
        )?;
        let capture = Arc::new(Capture::default());
        let mut config = OmoBackendConfig::from_env()?;
        let model = model_for_purpose(ModelPurpose::Kibitzer);
        config.default_model = Some(model.clone());
        config.fallback_model = model;
        config.total_timeout = TURN_TIMEOUT;
        let backend = Arc::new(OmoBackend::new(config, capture.clone()).with_pool(pool.clone()));
        Self::new(
            pool.clone(),
            lead,
            Arc::new(JevCli::new(None, Some(pool), None, None)),
            backend,
            capture,
            Arc::new(SystemClock),
            enabled,
        )
    }

    /// Idempotent task-owned schema setup; run before producers, not per evaluation.
    /// Scheduling survives restarts without changing the shared migration registry.
    pub async fn initialize(&self) -> Result<()> {
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS kibitzer_state (
                lead TEXT PRIMARY KEY, generation INTEGER NOT NULL DEFAULT 0,
                consumed INTEGER NOT NULL DEFAULT 0, admitted_ms INTEGER);
             CREATE TRIGGER IF NOT EXISTS kibitzer_inbox_insert AFTER INSERT ON inbox_items
             BEGIN UPDATE kibitzer_state SET generation = generation + 1; END;
             CREATE TRIGGER IF NOT EXISTS kibitzer_lane_insert AFTER INSERT ON lane_events
             WHEN NEW.kind IN ('blocked','question','stale','not_done','cron_problem')
             BEGIN UPDATE kibitzer_state SET generation = generation + 1
               WHERE lead = NEW.lead_session_key OR NEW.kind = 'cron_problem'; END;",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("INSERT INTO kibitzer_state(lead) VALUES (?) ON CONFLICT DO NOTHING")
            .bind(self.lead.storage_key())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn wake_count(&self, now: DateTime<Utc>) -> Result<i64> {
        let start = day_start(now);
        Ok(sqlx::query_scalar("SELECT count(*) FROM lane_events WHERE kind = 'kibitzer' AND lead_session_key = ? AND julianday(created_at) >= julianday(?) AND julianday(created_at) < julianday(?)")
            .bind(self.lead.storage_key()).bind(start).bind(start + TimeDelta::days(1))
            .fetch_one(&self.pool).await?)
    }

    /// None means unarmed: no timer may be installed in that state.
    pub async fn next_due(&self) -> Result<Option<DateTime<Utc>>> {
        self.next_due_at(self.clock.now()).await
    }

    /// Computes eligibility against a caller-supplied instant so a check and its
    /// comparison use the same clock reading (a later reading would always be "due > now").
    async fn next_due_at(&self, now: DateTime<Utc>) -> Result<Option<DateTime<Utc>>> {
        if !self.enabled {
            return Ok(None);
        }
        let (generation, consumed, admitted): (i64, i64, Option<i64>) = sqlx::query_as(
            "SELECT generation, consumed, admitted_ms FROM kibitzer_state WHERE lead = ?",
        )
        .bind(self.lead.storage_key())
        .fetch_one(&self.pool)
        .await?;
        if generation == consumed {
            return Ok(None);
        }
        let mut due = now;
        if let Some(last) = admitted.and_then(DateTime::from_timestamp_millis) {
            due = due.max(last + SPACING);
        }
        if self.wake_count(now).await? >= 6 {
            due = due.max(day_start(now) + TimeDelta::days(1));
        }
        if (due + KST).hour() < 8 {
            due = day_start(due) + TimeDelta::hours(8);
        }
        Ok(Some(due))
    }

    /// Consumes only the admitted generation. Input arriving during a turn remains armed.
    pub async fn evaluate(&self) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }
        let now = self.clock.now();
        if self.next_due_at(now).await?.is_none_or(|due| due > now) {
            return Ok(false);
        }
        let reserved = sqlx::query("UPDATE kibitzer_state SET consumed = generation, admitted_ms = ? WHERE lead = ? AND generation > consumed AND (admitted_ms IS NULL OR admitted_ms <= ?)")
            .bind(now.timestamp_millis()).bind(self.lead.storage_key())
            .bind((now - SPACING).timestamp_millis()).execute(&self.pool).await?;
        if reserved.rows_affected() != 1 {
            return Ok(false);
        }
        tracing::info!(lead = %self.lead.storage_key(), admitted_ms = now.timestamp_millis(), "kibitzer armed generation admitted");
        // The generation is already consumed: a failure here must re-arm it.
        let state = match self.state_document().await {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(%error, "kibitzer state unavailable");
                self.restore_admitted(now).await?;
                return Ok(false);
            }
        };
        let score = match self
            .judge
            .noul(
                Purpose::KibitzerWake,
                &scrub_for_external(&state),
                "Would the owner want the agent to proactively do or ask something right now?",
            )
            .await
        {
            Ok(score) => score,
            Err(error) => {
                tracing::warn!(%error, "kibitzer judge failed");
                self.restore_admitted(now).await?;
                return Ok(false);
            }
        };
        if !score.is_finite() || score < 0.7 {
            return Ok(false);
        }
        self.capture.text.lock().clear();
        self.capture.streams.lock().clear();
        let key = SessionKey::new(
            "cron",
            None::<String>,
            "cron:kibitzer",
            None::<String>,
            "cron:kibitzer",
        );
        let mut session = SessionContext::new(key.clone());
        session
            .state
            .metadata
            .insert("usage_purpose".into(), serde_json::json!("kibitzer"));
        session.state.active_model = Some(model_for_purpose(ModelPurpose::Kibitzer));
        session.state.enabled_toolsets = Some(Vec::new());
        session.state.system_prompt = Some("Return only JSON with exactly wake (boolean), reason (string), suggested (string), target_lane (string or null). Decide whether the owner needs proactive action. Do not perform actions.".into());
        let event =
            InboundEvent::message(key, uuid::Uuid::new_v4().to_string(), super::scrub(&state));
        let cancellation = CancellationToken::new();
        match tokio::time::timeout(
            TURN_TIMEOUT,
            self.backend
                .run_cancelable(&mut session, event, cancellation.clone()),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::warn!(%error, "kibitzer turn failed");
                self.restore_admitted(now).await?;
                return Ok(false);
            }
            Err(_) => {
                cancellation.cancel();
                if let Err(error) = self.backend.cancel(&session).await {
                    tracing::warn!(%error, "kibitzer cancellation failed");
                }
                tracing::warn!("kibitzer turn timed out");
                self.restore_admitted(now).await?;
                return Ok(false);
            }
        }
        // Bind first so the capture lock is released before any await below.
        let parsed = parse_decision(&self.capture.text.lock());
        let decision = match parsed {
            Ok(decision) => decision,
            Err(_) => {
                tracing::warn!("kibitzer reply is not strict decision JSON");
                self.restore_admitted(now).await?;
                return Ok(false);
            }
        };
        if !decision.wake {
            return Ok(false);
        }
        let decision = Decision {
            reason: super::scrub(&decision.reason).chars().take(1500).collect(),
            suggested: super::scrub(&decision.suggested)
                .chars()
                .take(1500)
                .collect(),
            target_lane: TargetLane(
                decision
                    .target_lane
                    .0
                    .map(|value| super::scrub(&value).chars().take(48).collect()),
            ),
            ..decision
        };
        let body = serde_json::to_string(&decision)
            .map_err(|error| OmonError::Config(error.to_string()))?;
        let now = self.clock.now();
        let start = day_start(now);
        // One atomic statement enforces the cap even with simultaneous workers.
        let inserted = sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) SELECT NULL, ?, 'kibitzer', ?, ? WHERE (SELECT count(*) FROM lane_events WHERE kind = 'kibitzer' AND lead_session_key = ? AND julianday(created_at) >= julianday(?) AND julianday(created_at) < julianday(?)) < 6")
            .bind(self.lead.storage_key()).bind(body).bind(now).bind(self.lead.storage_key())
            .bind(start).bind(start + TimeDelta::days(1)).execute(&self.pool).await?;
        Ok(inserted.rows_affected() == 1)
    }

    /// Re-arms a generation whose evaluation failed (not a decision); SPACING still applies.
    async fn restore_admitted(&self, admitted: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE kibitzer_state SET consumed = consumed - 1 WHERE lead = ? AND admitted_ms = ? AND consumed = generation")
            .bind(self.lead.storage_key())
            .bind(admitted.timestamp_millis())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn state_document(&self) -> Result<String> {
        let lead = self.lead.storage_key();
        let lanes: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT key, status, last_report FROM lanes WHERE lead_session_key = ? AND status IN ('queued','working','blocked','done_claimed','not_done','verified_done') ORDER BY created_at, key")
            .bind(&lead).fetch_all(&self.pool).await?;
        let events: Vec<(String, String)> = sqlx::query_as("SELECT kind, body FROM lane_events WHERE lead_session_key = ? OR kind = 'cron_problem' ORDER BY created_at DESC, id DESC LIMIT 10")
            .bind(&lead).fetch_all(&self.pool).await?;
        let mut messages: Vec<(String, String)> = sqlx::query_as("SELECT role, content FROM messages WHERE session_key = ? ORDER BY created_at DESC, id DESC LIMIT 30")
            .bind(&lead).fetch_all(&self.pool).await?;
        messages.reverse();
        let memories = MemoryStore::new(self.pool.clone())
            .search(&self.lead, "owner preferences", 10)
            .await?;
        let mut other = String::from("Open lanes:\n");
        for (key, status, report) in lanes {
            other.push_str(&format!("{key} {status}: {}\n", report.unwrap_or_default()));
        }
        other.push_str("Recent events:\n");
        for (kind, body) in events.into_iter().rev() {
            other.push_str(&format!("{kind}: {body}\n"));
        }
        other.push_str("Owner preferences:\n");
        for memory in memories {
            other.push_str(&memory.content);
            other.push('\n');
        }
        // The judge input is capped at DOC_CAP chars from the front, so the owner's
        // recent DMs come first and the lanes/events/memories take the remainder.
        let header = "Home DM messages:\n";
        let dm_budget = DM_BUDGET - header.chars().count();
        let mut entries: Vec<String> = messages
            .into_iter()
            .map(|(role, content)| {
                format!("{role}: {content}\n")
                    .chars()
                    .take(dm_budget)
                    .collect()
            })
            .collect();
        while entries
            .iter()
            .map(|entry: &String| entry.chars().count())
            .sum::<usize>()
            > dm_budget
        {
            entries.remove(0);
        }
        let dm = format!("{header}{}", entries.concat());
        let rest: String = other
            .chars()
            .take(DOC_CAP.saturating_sub(dm.chars().count()))
            .collect();
        Ok(format!("{dm}{rest}"))
    }

    pub async fn serve(&self, hint: Arc<Notify>, shutdown: CancellationToken) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        tracing::info!(lead = %self.lead.storage_key(), arm_scan_secs = ARM_SCAN_INTERVAL.as_secs(), "kibitzer serve started");
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
            // notify_one retains a permit between the durable scan and this wait.
            // Bound the wait as external SQLite writers have no in-process hint.
            // A transient DB error must not take the whole lane service down.
            if let Err(error) = self.evaluate().await {
                tracing::warn!(%error, "kibitzer evaluation failed; will retry");
            }
            let due = self.next_due().await.unwrap_or_else(|error| {
                tracing::warn!(%error, "kibitzer due check failed; will retry");
                None
            });
            match due {
                Some(due) => {
                    let delay = (due - self.clock.now())
                        .to_std()
                        .unwrap_or(Duration::ZERO)
                        .min(ARM_SCAN_INTERVAL);
                    tokio::select! {
                        _ = shutdown.cancelled() => return Ok(()),
                        _ = hint.notified() => {},
                        _ = tokio::time::sleep(delay) => {},
                    }
                }
                None => tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = hint.notified() => {},
                    _ = tokio::time::sleep(ARM_SCAN_INTERVAL) => {},
                },
            }
        }
    }
}

fn day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let local = now + KST;
    local
        - TimeDelta::seconds(i64::from(local.num_seconds_from_midnight()))
        - TimeDelta::nanoseconds(i64::from(local.nanosecond()))
        - KST
}

/// Call only for a genuinely new, identity-qualified owner/User insert, inside
/// its transcript transaction; after commit call runner::wake, even if indexing fails.
pub async fn arm_owner(tx: &mut Transaction<'_, Sqlite>, lead: &SessionKey) -> Result<()> {
    // The table exists only after `initialize`, which a disabled kibitzer skips; an
    // owner's transcript transaction must not fail because of it.
    let present: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'kibitzer_state')",
    )
    .fetch_one(&mut **tx)
    .await?;
    if !present {
        return Ok(());
    }
    sqlx::query("UPDATE kibitzer_state SET generation = generation + 1 WHERE lead = ?")
        .bind(lead.storage_key())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::{Choice, DiffVerdict, JudgeError};
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::oneshot;

    struct ScanClock {
        scans: AtomicUsize,
        idle: parking_lot::Mutex<Option<oneshot::Sender<()>>>,
    }

    impl Clock for ScanClock {
        fn now(&self) -> DateTime<Utc> {
            if self.scans.fetch_add(1, Ordering::SeqCst) == 1 {
                self.idle.lock().take().unwrap().send(()).unwrap();
            }
            "2026-10-12T00:17:00Z".parse().unwrap()
        }
    }

    struct DecisionGate {
        calls: AtomicUsize,
        decided: parking_lot::Mutex<Option<oneshot::Sender<()>>>,
    }

    #[async_trait]
    impl Judge for DecisionGate {
        async fn noul(
            &self,
            purpose: Purpose,
            _: &str,
            _: &str,
        ) -> std::result::Result<f64, JudgeError> {
            assert_eq!(purpose, Purpose::KibitzerWake);
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.decided.lock().take().unwrap().send(()).unwrap();
            Ok(0.1)
        }

        async fn choice(
            &self,
            _: Purpose,
            _: &str,
            _: &str,
            _: &[&str],
        ) -> std::result::Result<Choice, JudgeError> {
            panic!("unexpected choice")
        }

        async fn diff(
            &self,
            _: Purpose,
            _: &str,
            _: &Path,
            _: Option<&str>,
        ) -> std::result::Result<DiffVerdict, JudgeError> {
            panic!("unexpected diff")
        }
    }

    #[tokio::test]
    async fn arm_owner_is_a_no_op_before_the_state_table_exists() {
        let db = crate::Database::connect("sqlite::memory:").await.unwrap();
        let mut tx = db.pool().begin().await.unwrap();
        arm_owner(
            &mut tx,
            &SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    #[test]
    fn lead_must_be_an_owner_home_dm_on_discord_or_slack() {
        let slack =
            |user: &str, bot: Option<&str>, guild: Option<&str>, thread: Option<&str>| SessionKey {
                platform: "slack".into(),
                guild_id: guild.map(str::to_owned),
                channel_id: "D0C9HNVUK5M".into(),
                thread_id: thread.map(str::to_owned),
                user_id: user.into(),
                bot_id: bot.map(str::to_owned),
            };
        assert!(is_owner_home_dm(
            &SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
            "UOWNER"
        ));
        assert!(is_owner_home_dm(
            &slack("UOWNER", Some("UBOT"), None, None),
            "UOWNER"
        ));
        assert!(!is_owner_home_dm(
            &slack("UOTHER", Some("UBOT"), None, None),
            "UOWNER"
        ));
        assert!(!is_owner_home_dm(
            &slack("UOWNER", None, None, None),
            "UOWNER"
        ));
        assert!(!is_owner_home_dm(
            &slack("UOWNER", Some("UBOT"), Some("T1"), None),
            "UOWNER"
        ));
        assert!(!is_owner_home_dm(
            &slack("UOWNER", Some("UBOT"), None, Some("1.2")),
            "UOWNER"
        ));
        let mut discord = SessionKey::from_storage_key(DEFAULT_LEAD).unwrap();
        discord.user_id = "UOWNER".into();
        assert!(!is_owner_home_dm(&discord, "UOWNER"));
    }

    #[test]
    fn decision_json_in_a_whole_reply_fence_is_accepted() {
        let raw = "```json\n{\"wake\":true,\"reason\":\"r\",\"suggested\":\"s\",\"target_lane\":null}\n```";
        assert!(parse_decision(raw).unwrap().wake);
        assert!(parse_decision("```json\n{} trailing\n```").is_err());
        assert!(parse_decision("prose ```{}```").is_err());
    }

    struct NoTurn;

    #[async_trait]
    impl AgentBackend for NoTurn {
        async fn run(&self, _: &mut SessionContext, _: InboundEvent) -> Result<()> {
            panic!("low judge score must not run a model")
        }
    }

    /// Every reading is 1 ms later than the previous one, like a real wall clock.
    struct AdvancingClock(AtomicUsize);

    impl Clock for AdvancingClock {
        fn now(&self) -> DateTime<Utc> {
            let tick = self.0.fetch_add(1, Ordering::SeqCst) as i64;
            "2026-10-12T00:17:00Z".parse::<DateTime<Utc>>().unwrap() + TimeDelta::milliseconds(tick)
        }
    }

    #[tokio::test]
    async fn armed_generation_is_admitted_under_an_advancing_clock() {
        let scratch = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", scratch.path().join("qa.db").display());
        let db = crate::Database::connect(&url).await.unwrap();
        let (decision_tx, decision_rx) = oneshot::channel();
        let gate = Arc::new(DecisionGate {
            calls: AtomicUsize::new(0),
            decided: parking_lot::Mutex::new(Some(decision_tx)),
        });
        let worker = Kibitzer::new(
            db.pool().clone(),
            SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
            gate.clone(),
            Arc::new(NoTurn),
            Arc::new(Capture::default()),
            Arc::new(AdvancingClock(AtomicUsize::new(0))),
            true,
        )
        .unwrap();
        worker.initialize().await.unwrap();
        sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,raw_hash,collected_at) VALUES ('armed','mail','armed','due',CURRENT_TIMESTAMP,'hash',CURRENT_TIMESTAMP)")
            .execute(db.pool()).await.unwrap();
        assert!(worker.evaluate().await.unwrap() || gate.calls.load(Ordering::SeqCst) == 1);
        tokio::time::timeout(Duration::from_secs(5), decision_rx)
            .await
            .expect("armed generation must reach the judge on the first evaluation")
            .unwrap();
        let (generation, consumed): (i64, i64) =
            sqlx::query_as("SELECT generation, consumed FROM kibitzer_state")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!((generation, consumed), (1, 1));
        assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    }

    struct FailingJudge;

    #[async_trait]
    impl Judge for FailingJudge {
        async fn noul(&self, _: Purpose, _: &str, _: &str) -> std::result::Result<f64, JudgeError> {
            Err(JudgeError::Timeout { timeout_secs: 1 })
        }

        async fn choice(
            &self,
            _: Purpose,
            _: &str,
            _: &str,
            _: &[&str],
        ) -> std::result::Result<Choice, JudgeError> {
            panic!("unexpected choice")
        }

        async fn diff(
            &self,
            _: Purpose,
            _: &str,
            _: &Path,
            _: Option<&str>,
        ) -> std::result::Result<DiffVerdict, JudgeError> {
            panic!("unexpected diff")
        }
    }

    #[tokio::test]
    async fn state_document_keeps_owner_dms_inside_the_judge_cap() {
        let scratch = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", scratch.path().join("qa.db").display());
        let db = crate::Database::connect(&url).await.unwrap();
        let worker = Kibitzer::new(
            db.pool().clone(),
            SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
            Arc::new(FailingJudge),
            Arc::new(NoTurn),
            Arc::new(Capture::default()),
            Arc::new(AdvancingClock(AtomicUsize::new(0))),
            true,
        )
        .unwrap();
        worker.initialize().await.unwrap();
        for index in 0..10 {
            sqlx::query("INSERT INTO lane_events(lead_session_key,kind,body,created_at) VALUES (?, 'question', ?, CURRENT_TIMESTAMP)")
                .bind(DEFAULT_LEAD)
                .bind(format!("{index}{}", "x".repeat(1900)))
                .execute(db.pool())
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO sessions(session_key, platform, channel_id, user_id) VALUES (?, 'discord', 'c', 'u')")
            .bind(DEFAULT_LEAD)
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO messages(id, session_key, role, content) VALUES ('m1', ?, 'user', 'OWNER_DM_MARKER please call me')")
            .bind(DEFAULT_LEAD)
            .execute(db.pool())
            .await
            .unwrap();
        let document = worker.state_document().await.unwrap();
        assert!(document.chars().count() <= 4000);
        let capped = crate::judge::scrub_for_external(&document);
        assert!(capped.contains("OWNER_DM_MARKER"));
    }

    #[tokio::test]
    async fn judge_failure_keeps_generation_armed() {
        let scratch = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", scratch.path().join("qa.db").display());
        let db = crate::Database::connect(&url).await.unwrap();
        let worker = Kibitzer::new(
            db.pool().clone(),
            SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
            Arc::new(FailingJudge),
            Arc::new(NoTurn),
            Arc::new(Capture::default()),
            Arc::new(AdvancingClock(AtomicUsize::new(0))),
            true,
        )
        .unwrap();
        worker.initialize().await.unwrap();
        sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,raw_hash,collected_at) VALUES ('armed','mail','armed','due',CURRENT_TIMESTAMP,'hash',CURRENT_TIMESTAMP)")
            .execute(db.pool()).await.unwrap();
        assert!(!worker.evaluate().await.unwrap());
        let (generation, consumed): (i64, i64) =
            sqlx::query_as("SELECT generation, consumed FROM kibitzer_state")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!((generation, consumed), (1, 0));
    }

    #[tokio::test]
    async fn qa_gaps_kibitzer_serves_external_insert_without_notify() {
        let scratch = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", scratch.path().join("qa.db").display());
        let db = crate::Database::connect(&url).await.unwrap();
        let writer = sqlx::SqlitePool::connect(&url).await.unwrap();
        let (idle_tx, idle_rx) = oneshot::channel();
        let (decision_tx, decision_rx) = oneshot::channel();
        let gate = Arc::new(DecisionGate {
            calls: AtomicUsize::new(0),
            decided: parking_lot::Mutex::new(Some(decision_tx)),
        });
        let worker = Kibitzer::new(
            db.pool().clone(),
            SessionKey::from_storage_key(DEFAULT_LEAD).unwrap(),
            gate.clone(),
            Arc::new(NoTurn),
            Arc::new(Capture::default()),
            Arc::new(ScanClock {
                scans: AtomicUsize::new(0),
                idle: parking_lot::Mutex::new(Some(idle_tx)),
            }),
            true,
        )
        .unwrap();
        worker.initialize().await.unwrap();
        let shutdown = CancellationToken::new();
        let task_shutdown = shutdown.clone();
        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(async move { worker.serve(Arc::new(Notify::new()), task_shutdown).await });
        tokio::time::timeout(Duration::from_secs(10), idle_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(gate.calls.load(Ordering::SeqCst), 0);

        sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,raw_hash,collected_at) VALUES ('external','mail','external','due',CURRENT_TIMESTAMP,'hash',CURRENT_TIMESTAMP)")
            .execute(&writer).await.unwrap();
        let decision = tokio::time::timeout(Duration::from_secs(15), decision_rx).await;
        shutdown.cancel();
        tasks.join_next().await.unwrap().unwrap().unwrap();
        decision.unwrap().unwrap();
        assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
        let state: (i64, i64, Option<i64>) =
            sqlx::query_as("SELECT generation, consumed, admitted_ms FROM kibitzer_state")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(state, (1, 1, Some(1791764220000)));
    }
}

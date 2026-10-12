use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use omon_gateway::agent::{model_for_purpose, AgentBackend, ModelPurpose};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::lanes::kibitzer::{arm_owner, parse_decision, Capture, Kibitzer, DEFAULT_LEAD};
use omon_gateway::work_heartbeat::MockClock;
use omon_gateway::{
    Database, InboundEvent, OutboundAction, OutboundDispatcher, Result, SessionContext, SessionKey,
};

struct Gate {
    score: f64,
    calls: AtomicUsize,
}
#[async_trait]
impl Judge for Gate {
    async fn noul(
        &self,
        purpose: Purpose,
        state: &str,
        _: &str,
    ) -> std::result::Result<f64, JudgeError> {
        assert_eq!(purpose, Purpose::KibitzerWake);
        assert!(state.chars().count() <= 4000);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.score)
    }
    async fn choice(
        &self,
        _: Purpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        panic!("noul only")
    }
    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        panic!("noul only")
    }
}

struct Runner {
    capture: Arc<Capture>,
    reply: String,
    calls: AtomicUsize,
}
#[async_trait]
impl AgentBackend for Runner {
    async fn run(&self, session: &mut SessionContext, _: InboundEvent) -> Result<()> {
        assert_eq!(session.key.user_id, "cron:kibitzer");
        assert_eq!(
            session.state.active_model,
            Some(model_for_purpose(ModelPurpose::Kibitzer))
        );
        assert_eq!(session.state.enabled_toolsets, Some(vec![]));
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.capture
            .dispatch(OutboundAction::SendMessage {
                session: session.key.clone(),
                content: self.reply.clone(),
                reply_to: None,
            })
            .await
    }
}

const YES: &str = r#"{"wake":true,"reason":"due","suggested":"review","target_lane":null}"#;
struct Fixture {
    db: Database,
    worker: Kibitzer,
    gate: Arc<Gate>,
    runner: Arc<Runner>,
    clock: Arc<MockClock>,
    lead: SessionKey,
}
impl Fixture {
    async fn new(score: f64, reply: &str, enabled: bool) -> Self {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let lead = SessionKey::from_storage_key(DEFAULT_LEAD).unwrap();
        let clock = Arc::new(MockClock::new("2026-10-11T00:00:00Z".parse().unwrap()));
        let capture = Arc::new(Capture::default());
        let gate = Arc::new(Gate {
            score,
            calls: AtomicUsize::new(0),
        });
        let runner = Arc::new(Runner {
            capture: capture.clone(),
            reply: reply.into(),
            calls: AtomicUsize::new(0),
        });
        let worker = Kibitzer::new(
            db.pool().clone(),
            lead.clone(),
            gate.clone(),
            runner.clone(),
            capture,
            clock.clone(),
            enabled,
        )
        .unwrap();
        worker.initialize().await.unwrap();
        Self {
            db,
            worker,
            gate,
            runner,
            clock,
            lead,
        }
    }
    async fn inbox(&self, id: &str) {
        sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,raw_hash,collected_at) VALUES (?,'mail',?,'due',?,'hash',?) ON CONFLICT DO NOTHING")
            .bind(id).bind(id).bind(Utc::now()).bind(Utc::now()).execute(self.db.pool()).await.unwrap();
    }
    async fn count(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM lane_events WHERE kind='kibitzer'")
            .fetch_one(self.db.pool())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn no_turns_when_unarmed_for_a_day_or_disabled() {
    for enabled in [true, false] {
        let f = Fixture::new(0.7, YES, enabled).await;
        if !enabled {
            f.inbox("new").await;
        }
        f.clock.advance(TimeDelta::days(1));
        assert!(!f.worker.evaluate().await.unwrap());
        assert_eq!(f.gate.calls.load(Ordering::SeqCst), 0);
        assert_eq!(f.runner.calls.load(Ordering::SeqCst), 0);
        assert!(f.worker.next_due().await.unwrap().is_none());
    }
}

#[tokio::test]
async fn one_turn_and_event_when_new_inbox_passes_gate() {
    let f = Fixture::new(0.7, YES, true).await;
    f.inbox("new").await;
    assert!(f.worker.evaluate().await.unwrap());
    f.inbox("new").await;
    f.clock.advance(TimeDelta::hours(1));
    assert!(!f.worker.evaluate().await.unwrap());
    assert_eq!(f.runner.calls.load(Ordering::SeqCst), 1);
    assert_eq!(f.count().await, 1);
}

#[tokio::test]
async fn no_turn_when_gate_is_low_or_nonfinite() {
    for score in [0.69, f64::NAN, f64::INFINITY] {
        let f = Fixture::new(score, YES, true).await;
        f.inbox("new").await;
        assert!(!f.worker.evaluate().await.unwrap());
        assert_eq!(f.runner.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn no_event_when_reply_is_invalid_or_false() {
    for reply in [
        "not JSON",
        r#"{"wake":false,"reason":"idle","suggested":"","target_lane":null}"#,
    ] {
        let f = Fixture::new(0.8, reply, true).await;
        f.inbox("new").await;
        assert!(!f.worker.evaluate().await.unwrap());
        assert_eq!(f.count().await, 0);
    }
}

#[tokio::test]
async fn seventh_wake_is_suppressed_and_kst_day_resets() {
    let f = Fixture::new(0.8, YES, true).await;
    for n in 0..7 {
        f.inbox(&n.to_string()).await;
        assert_eq!(f.worker.evaluate().await.unwrap(), n < 6);
        f.clock.advance(TimeDelta::minutes(30));
    }
    assert_eq!(f.count().await, 6);
    assert_eq!(f.runner.calls.load(Ordering::SeqCst), 6);
    f.clock.set("2026-10-12T00:00:00Z".parse().unwrap());
    assert!(f.worker.evaluate().await.unwrap());
}

#[tokio::test]
async fn admission_spacing_survives_reinitialization_and_hours_are_kst() {
    let f = Fixture::new(0.1, YES, true).await;
    f.clock.set("2026-10-10T22:59:00Z".parse().unwrap());
    f.inbox("a").await;
    assert!(!f.worker.evaluate().await.unwrap());
    f.clock.advance(TimeDelta::minutes(1));
    assert!(!f.worker.evaluate().await.unwrap());
    assert_eq!(f.gate.calls.load(Ordering::SeqCst), 1);
    f.inbox("b").await;
    f.worker.initialize().await.unwrap();
    f.clock.advance(TimeDelta::minutes(29));
    assert!(!f.worker.evaluate().await.unwrap());
    assert_eq!(f.gate.calls.load(Ordering::SeqCst), 1);
    f.clock.advance(TimeDelta::minutes(1));
    assert!(!f.worker.evaluate().await.unwrap());
    assert_eq!(f.gate.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn lane_filter_and_owner_transaction_arm_only_committed_input() {
    let f = Fixture::new(0.8, YES, true).await;
    for kind in ["milestone", "dead", "kibitzer"] {
        sqlx::query(
            "INSERT INTO lane_events(lead_session_key,kind,body,created_at) VALUES (?,?, '',?)",
        )
        .bind(DEFAULT_LEAD)
        .bind(kind)
        .bind(Utc::now())
        .execute(f.db.pool())
        .await
        .unwrap();
    }
    assert!(f.worker.next_due().await.unwrap().is_none());
    let mut tx = f.db.pool().begin().await.unwrap();
    arm_owner(&mut tx, &f.lead).await.unwrap();
    tx.rollback().await.unwrap();
    assert!(f.worker.next_due().await.unwrap().is_none());
    sqlx::query("INSERT INTO lane_events(lead_session_key,kind,body,created_at) VALUES ('cron:job','cron_problem','failure',?)")
        .bind(Utc::now()).execute(f.db.pool()).await.unwrap();
    assert!(f.worker.evaluate().await.unwrap());
}

#[tokio::test]
async fn oversized_unicode_document_is_bounded() {
    let f = Fixture::new(0.8, YES, true).await;
    sqlx::query(
        "INSERT INTO lane_events(lead_session_key,kind,body,created_at) VALUES (?,'question',?,?)",
    )
    .bind(DEFAULT_LEAD)
    .bind("한".repeat(9000))
    .bind(Utc::now())
    .execute(f.db.pool())
    .await
    .unwrap();
    assert!(f.worker.state_document().await.unwrap().chars().count() <= 6000);
    assert!(f.worker.evaluate().await.unwrap());
}

#[test]
fn strict_parser_rejects_missing_duplicate_wrapped_and_wrong_fields() {
    assert!(parse_decision(YES).unwrap().target_lane.0.is_none());
    for raw in [
        r#"{"wake":true,"reason":"x","suggested":"y"}"#,
        r#"{"wake":true,"wake":false,"reason":"x","suggested":"y","target_lane":null}"#,
        r#"{"wake":true,"reason":"x","suggested":"y","target_lane":null,"extra":0}"#,
        r#"{"wake":"true","reason":"x","suggested":"y","target_lane":null}"#,
        "```json\n{}\n```",
        "{} trailing",
        "prose {}",
    ] {
        assert!(parse_decision(raw).is_err(), "{raw}");
    }
}

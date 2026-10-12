use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use omon_gateway::judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose};
use omon_gateway::lanes::dispatch::EventDispatcher;
use omon_gateway::{
    AgentRunner, Database, EventOrigin, InboundEvent, MultiplexerConfig, OutboundAction,
    OutboundDispatcher, Result, SessionContext, SessionKey, SessionMultiplexer,
};
use tokio::sync::{mpsc, oneshot, watch};

struct FakeJudge {
    score: f64,
    calls: AtomicUsize,
}

#[async_trait]
impl Judge for FakeJudge {
    async fn choice(
        &self,
        _: Purpose,
        _: &str,
        _: &str,
        _: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        panic!("dispatcher must use noul")
    }
    async fn noul(
        &self,
        purpose: Purpose,
        _: &str,
        _: &str,
    ) -> std::result::Result<f64, JudgeError> {
        assert_eq!(purpose, Purpose::EventWake);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.score)
    }
    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        panic!("dispatcher must use noul")
    }
}

struct FakeRunner {
    started: mpsc::Sender<(InboundEvent, oneshot::Sender<()>)>,
}

#[async_trait]
impl AgentRunner for FakeRunner {
    async fn run(&self, _: &mut SessionContext, event: InboundEvent) -> Result<()> {
        let (release, released) = oneshot::channel();
        self.started
            .send((event, release))
            .await
            .expect("test observer open");
        released.await.expect("test releases turn");
        Ok(())
    }
}

struct FakeOutbound {
    reactions: AtomicUsize,
    ended: mpsc::Sender<()>,
}

#[async_trait]
impl OutboundDispatcher for FakeOutbound {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        match action {
            OutboundAction::React { .. } => {
                self.reactions.fetch_add(1, Ordering::SeqCst);
            }
            OutboundAction::Typing { active: false, .. } => {
                self.ended.send(()).await.expect("completion observer open");
            }
            _ => {}
        }
        Ok(())
    }
}

struct Fixture {
    db: Database,
    mux: SessionMultiplexer,
    dispatcher: EventDispatcher,
    lead: SessionKey,
    started: mpsc::Receiver<(InboundEvent, oneshot::Sender<()>)>,
    ended: mpsc::Receiver<()>,
    outbound: Arc<FakeOutbound>,
    judge: Arc<FakeJudge>,
    drain: watch::Sender<bool>,
    now: DateTime<Utc>,
}

impl Fixture {
    async fn new(score: f64) -> Self {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (started_tx, started) = mpsc::channel(8);
        let (ended_tx, ended) = mpsc::channel(8);
        let (drain, drain_rx) = watch::channel(false);
        let outbound = Arc::new(FakeOutbound {
            reactions: AtomicUsize::new(0),
            ended: ended_tx,
        });
        let mux = SessionMultiplexer::with_dispatcher(
            db.pool().clone(),
            Arc::new(FakeRunner {
                started: started_tx,
            }),
            Some(outbound.clone()),
            MultiplexerConfig::default(),
        )
        .with_drain_receiver(drain_rx);
        let judge = Arc::new(FakeJudge {
            score,
            calls: AtomicUsize::new(0),
        });
        let dispatcher = EventDispatcher::new(db.pool().clone(), mux.clone(), judge.clone());
        Self {
            db,
            mux,
            dispatcher,
            lead: SessionKey::new(
                "discord",
                None::<String>,
                "dispatch-test",
                None::<String>,
                "414011632306618368",
            ),
            started,
            ended,
            outbound,
            judge,
            drain,
            now: DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
        }
    }

    fn at(&self, seconds: i64) -> DateTime<Utc> {
        self.now + TimeDelta::seconds(seconds)
    }

    async fn enqueue(&self, lane: &str, kind: &str, body: &str, seconds: i64) -> i64 {
        sqlx::query("INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(lane).bind(self.lead.storage_key()).bind(kind).bind(body).bind(self.at(seconds))
            .execute(self.db.pool()).await.unwrap().last_insert_rowid()
    }

    async fn tick(&self, seconds: i64) {
        self.dispatcher.tick(self.at(seconds)).await.unwrap();
    }

    async fn turn(&mut self) -> (InboundEvent, oneshot::Sender<()>) {
        tokio::time::timeout(Duration::from_secs(2), self.started.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn finish(&mut self, release: oneshot::Sender<()>) {
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), self.ended.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(self.mux.session_is_idle(&self.lead).await.unwrap());
    }

    async fn pending(&self) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE delivered_at IS NULL")
            .fetch_one(self.db.pool())
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn three_events_within_ten_seconds_coalesce_into_one_turn() {
    let mut f = Fixture::new(0.9).await;
    for seconds in [0, 5, 10] {
        f.enqueue(&format!("lane-{seconds}"), "question", "pick", seconds)
            .await;
    }
    f.tick(10).await;
    assert!(f.started.try_recv().is_err());
    f.tick(30).await;
    let (event, release) = f.turn().await;
    assert_eq!(event.content.lines().count(), 4);
    assert!(event.content.starts_with("[LANE_EVENT]\n"));
    assert_eq!(f.pending().await, 0);
    f.finish(release).await;
    assert!(f.started.try_recv().is_err());
}

#[tokio::test]
async fn done_claimed_delivers_immediately_after_idle_delivery_slot() {
    let mut f = Fixture::new(0.0).await;
    f.enqueue("lane", "done_claimed", "first", 0).await;
    f.tick(0).await;
    let (_, release) = f.turn().await;
    f.finish(release).await;
    f.enqueue("lane", "done_claimed", "second", 31).await;
    f.tick(31).await;
    let (event, release) = f.turn().await;
    assert_eq!(event.received_at, f.at(31));
    assert_eq!(f.judge.calls.load(Ordering::SeqCst), 0);
    f.finish(release).await;
}

#[tokio::test]
async fn two_done_claims_five_seconds_apart_wait_thirty_seconds() {
    let mut f = Fixture::new(1.0).await;
    f.enqueue("lane", "done_claimed", "first", 0).await;
    f.tick(0).await;
    let (first, release) = f.turn().await;
    f.finish(release).await;
    f.enqueue("lane", "done_claimed", "second", 5).await;
    f.tick(5).await;
    f.tick(29).await;
    assert_eq!(f.pending().await, 1);
    assert!(f.started.try_recv().is_err());
    f.tick(30).await;
    let (second, release) = f.turn().await;
    assert!(
        second
            .received_at
            .signed_duration_since(first.received_at)
            .num_seconds()
            >= 30
    );
    f.finish(release).await;
}

#[tokio::test]
async fn event_during_in_flight_turn_waits_until_turn_ends() {
    let mut f = Fixture::new(1.0).await;
    f.mux
        .route(InboundEvent::message(
            f.lead.clone(),
            "user-message",
            "blocking",
        ))
        .await
        .unwrap();
    let (_, user_release) = f.turn().await;
    f.enqueue("lane", "done_claimed", "ready", 0).await;
    f.tick(30).await;
    assert_eq!(f.pending().await, 1);
    assert!(f.started.try_recv().is_err());
    f.finish(user_release).await;
    f.tick(30).await;
    let (_, release) = f.turn().await;
    assert_eq!(f.pending().await, 0);
    f.finish(release).await;
}

#[tokio::test]
async fn duplicate_lane_kind_pairs_collapse_to_latest_body() {
    let mut f = Fixture::new(1.0).await;
    f.enqueue("lane", "question", "old", 0).await;
    f.enqueue("lane", "question", "latest", 5).await;
    f.tick(30).await;
    let (event, release) = f.turn().await;
    assert_eq!(event.content.lines().count(), 2);
    let line = event.content.lines().nth(1).unwrap();
    let (_, body) = line.split_once(": ").unwrap();
    assert_eq!(body, "latest");
    assert_eq!(f.pending().await, 0);
    assert_eq!(f.judge.calls.load(Ordering::SeqCst), 1);
    f.finish(release).await;
}

#[tokio::test]
async fn route_error_keeps_events_undelivered_for_retry() {
    let mut f = Fixture::new(1.0).await;
    f.enqueue("lane", "done_claimed", "ready", 0).await;
    f.drain.send(true).unwrap();
    let mut rejected = InboundEvent::message(f.lead.clone(), "synthetic", "drain probe");
    rejected.origin = EventOrigin::System;
    assert!(f.mux.route(rejected).await.is_err());
    f.tick(0).await;
    assert_eq!(f.pending().await, 1);
    assert!(f.started.try_recv().is_err());
    f.drain.send(false).unwrap();
    f.tick(5).await;
    let (_, release) = f.turn().await;
    assert_eq!(f.pending().await, 0);
    f.finish(release).await;
}

#[tokio::test]
async fn system_event_triggers_no_outbound_reaction() {
    let mut f = Fixture::new(1.0).await;
    f.enqueue("lane", "done_claimed", "ready", 0).await;
    f.tick(0).await;
    let (event, release) = f.turn().await;
    assert_eq!(event.origin, EventOrigin::System);
    f.finish(release).await;
    assert_eq!(f.outbound.reactions.load(Ordering::SeqCst), 0);
    let content: String = sqlx::query_scalar("SELECT content FROM messages WHERE id = ?")
        .bind(event.id.to_string())
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    assert!(content.starts_with("[system] "));
}

#[tokio::test]
async fn jev_below_threshold_marks_digest_only_without_turn() {
    let mut f = Fixture::new(0.49).await;
    let id = f.enqueue("lane", "question", "routine", 0).await;
    f.tick(30).await;
    assert_eq!(f.pending().await, 0);
    let reason: String = sqlx::query_scalar("SELECT delivery_reason FROM lane_events WHERE id = ?")
        .bind(id)
        .fetch_one(f.db.pool())
        .await
        .unwrap();
    assert_eq!(reason, "jev_skip");
    assert_eq!(f.judge.calls.load(Ordering::SeqCst), 1);
    assert!(f.started.try_recv().is_err());
}

#[tokio::test]
async fn user_admitted_after_idle_probe_refuses_system_turn() {
    let mut f = Fixture::new(1.0).await;
    assert!(f.mux.session_is_idle(&f.lead).await.unwrap());
    f.mux
        .route(InboundEvent::message(
            f.lead.clone(),
            "race-user",
            "blocking",
        ))
        .await
        .unwrap();
    let (_, release) = f.turn().await;
    let mut event = InboundEvent::message(f.lead.clone(), "race-system", "system");
    event.origin = EventOrigin::System;
    assert!(f.mux.route(event).await.is_err());
    assert!(f.started.try_recv().is_err());
    f.finish(release).await;
    f.mux.shutdown().await.unwrap();
}

#[test]
fn origin_defaults_for_legacy_events_and_roundtrips_system() {
    let event = InboundEvent::message(
        SessionKey::new("local", None::<String>, "origin14", None::<String>, "owner"),
        "origin14",
        "input",
    );
    let mut legacy = serde_json::to_value(&event).unwrap();
    legacy.as_object_mut().unwrap().remove("origin");
    assert_eq!(
        serde_json::from_value::<InboundEvent>(legacy)
            .unwrap()
            .origin,
        EventOrigin::User
    );
    let mut system = event;
    system.origin = EventOrigin::System;
    let decoded: InboundEvent =
        serde_json::from_str(&serde_json::to_string(&system).unwrap()).unwrap();
    assert_eq!(decoded.origin, EventOrigin::System);
}

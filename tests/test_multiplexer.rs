use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use omon_gateway::discord::adapter::{message_to_inbound_with_config, InboundFilterConfig};
use omon_gateway::{
    AgentRunner, Database, DeliveryLedgerService, InboundEvent, MultiplexerConfig, OmonError,
    OutboundAction, OutboundDispatcher, ProfileRoute, ProfileRouter, SessionActor, SessionContext,
    SessionKey, SessionMultiplexer,
};
use serenity::model::channel::{ChannelType, Message};
use serenity::model::id::UserId;
use tokio::sync::{mpsc, oneshot, Barrier, Mutex};

fn session(user: &str) -> SessionKey {
    SessionKey::new(
        "discord",
        Some("guild"),
        format!("channel-{user}"),
        None::<String>,
        user,
    )
}

#[tokio::test]
async fn routes_multiple_sessions_in_parallel() {
    struct ParallelRunner {
        barrier: Barrier,
        completed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for ParallelRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.barrier.wait().await;
            self.completed
                .send(event.content)
                .expect("test receiver should remain open");
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let runner = Arc::new(ParallelRunner {
        barrier: Barrier::new(8),
        completed: completed_tx,
    });
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        runner,
        MultiplexerConfig::default(),
    );

    let mut routes = Vec::new();
    for index in 0..8 {
        let multiplexer = multiplexer.clone();
        routes.push(tokio::spawn(async move {
            multiplexer
                .route(InboundEvent::message(
                    session(&format!("user-{index}")),
                    format!("message-{index}"),
                    format!("event-{index}"),
                ))
                .await
                .unwrap();
        }));
    }
    for route in routes {
        route.await.unwrap();
    }

    let mut received = Vec::new();
    while received.len() < 8 {
        received.push(
            tokio::time::timeout(Duration::from_secs(2), completed_rx.recv())
                .await
                .expect("parallel actors should all complete")
                .unwrap(),
        );
    }
    assert_eq!(multiplexer.active_sessions(), 8);
}

#[tokio::test]
async fn handles_events_sequentially_within_one_session() {
    struct SequentialRunner {
        active: Mutex<HashMap<String, usize>>,
        completed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for SequentialRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            let mut active = self.active.lock().await;
            let count = active.entry(session.key.storage_key()).or_default();
            *count += 1;
            assert_eq!(*count, 1, "same-session executions overlapped");
            drop(active);
            tokio::task::yield_now().await;
            let mut active = self.active.lock().await;
            *active.get_mut(&session.key.storage_key()).unwrap() -= 1;
            drop(active);
            self.completed
                .send(event.content)
                .expect("test receiver should remain open");
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(SequentialRunner {
            active: Mutex::new(HashMap::new()),
            completed: completed_tx,
        }),
        MultiplexerConfig::default(),
    );
    let key = session("one-user");

    let mut observed = Vec::new();
    for index in 0..20 {
        multiplexer
            .route(InboundEvent::message(
                key.clone(),
                format!("message-{index}"),
                index.to_string(),
            ))
            .await
            .unwrap();
        observed.push(
            tokio::time::timeout(Duration::from_secs(2), completed_rx.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    assert_eq!(
        observed,
        (0..20).map(|index| index.to_string()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn events_arriving_during_running_turn_are_queued_and_processed_in_order() {
    struct QueuedRunner {
        started: mpsc::UnboundedSender<String>,
        first_release: Mutex<Option<oneshot::Receiver<()>>>,
        completed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for QueuedRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.started.send(event.content.clone()).unwrap();
            if event.content == "first" {
                let release = self.first_release.lock().await.take().unwrap();
                let _ = release.await;
                session.state.metadata.insert("turn_1".into(), true.into());
            } else if event.content == "second" {
                session.state.metadata.insert("turn_2".into(), true.into());
            } else if event.content == "third" {
                session.state.metadata.insert("turn_3".into(), true.into());
            }
            self.completed.send(event.content).unwrap();
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("queue-user");
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let (release_tx, release_rx) = oneshot::channel();
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(QueuedRunner {
            started: started_tx,
            first_release: Mutex::new(Some(release_rx)),
            completed: completed_tx,
        }),
        MultiplexerConfig::default(),
    );

    multiplexer
        .route(InboundEvent::message(key.clone(), "queue-1", "first"))
        .await
        .unwrap();
    assert_eq!(started_rx.recv().await.as_deref(), Some("first"));

    // Route second and third events while the first turn is still in flight
    multiplexer
        .route(InboundEvent::message(key.clone(), "queue-2", "second"))
        .await
        .unwrap();
    multiplexer
        .route(InboundEvent::message(key.clone(), "queue-3", "third"))
        .await
        .unwrap();

    // Release turn 1
    release_tx.send(()).unwrap();

    assert_eq!(completed_rx.recv().await.as_deref(), Some("first"));
    assert_eq!(started_rx.recv().await.as_deref(), Some("second"));
    assert_eq!(completed_rx.recv().await.as_deref(), Some("second"));
    assert_eq!(started_rx.recv().await.as_deref(), Some("third"));
    assert_eq!(completed_rx.recv().await.as_deref(), Some("third"));

    let messages: Vec<String> =
        sqlx::query_scalar("SELECT content FROM messages WHERE session_key = ? ORDER BY sequence")
            .bind(key.storage_key())
            .fetch_all(database.pool())
            .await
            .unwrap();
    assert_eq!(messages, vec!["first", "second", "third"]);
}

#[tokio::test]
async fn stop_immediately_cancels_the_active_turn() {
    struct StoppableRunner {
        started: mpsc::UnboundedSender<()>,
        release: Mutex<Option<oneshot::Receiver<()>>>,
        dropped: Arc<AtomicBool>,
        completed: Arc<AtomicBool>,
    }

    struct DropSignal(Arc<AtomicBool>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl AgentRunner for StoppableRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            let _drop_signal = DropSignal(self.dropped.clone());
            self.started.send(()).unwrap();
            let release = self.release.lock().await.take().unwrap();
            let _ = release.await;
            self.completed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("stop-user");
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (_release_tx, release_rx) = oneshot::channel();
    let dropped = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(StoppableRunner {
            started: started_tx,
            release: Mutex::new(Some(release_rx)),
            dropped: dropped.clone(),
            completed: completed.clone(),
        }),
        MultiplexerConfig::default(),
    );

    multiplexer
        .route(InboundEvent::message(key.clone(), "stop-1", "long turn"))
        .await
        .unwrap();
    started_rx.recv().await.unwrap();

    assert!(multiplexer.stop(&key).await.unwrap());
    assert!(dropped.load(Ordering::SeqCst));
    assert!(!completed.load(Ordering::SeqCst));
    assert!(!multiplexer.stop(&key).await.unwrap());
}

#[tokio::test]
async fn stop_cancels_active_turn_and_clears_queued_events() {
    struct QueuedStoppableRunner {
        started: mpsc::UnboundedSender<String>,
        first_release: Mutex<Option<oneshot::Receiver<()>>>,
        completed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for QueuedStoppableRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.started.send(event.content.clone()).unwrap();
            if event.content == "turn-1" {
                let release = self.first_release.lock().await.take().unwrap();
                let _ = release.await;
            }
            self.completed.send(event.content).unwrap();
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("stop-queue-user");
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let (_release_tx, release_rx) = oneshot::channel();
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(QueuedStoppableRunner {
            started: started_tx,
            first_release: Mutex::new(Some(release_rx)),
            completed: completed_tx,
        }),
        MultiplexerConfig::default(),
    );

    multiplexer
        .route(InboundEvent::message(key.clone(), "turn-1", "turn-1"))
        .await
        .unwrap();
    assert_eq!(started_rx.recv().await.as_deref(), Some("turn-1"));

    // Queue turn 2 and turn 3 behind turn 1
    multiplexer
        .route(InboundEvent::message(key.clone(), "turn-2", "turn-2"))
        .await
        .unwrap();
    multiplexer
        .route(InboundEvent::message(key.clone(), "turn-3", "turn-3"))
        .await
        .unwrap();

    // Stop session: cancels active turn 1 and drains queued turns
    assert!(multiplexer.stop(&key).await.unwrap());

    // Neither turn-1, turn-2, nor turn-3 should have completed successfully
    assert!(completed_rx.try_recv().is_err());
    // Turn 2 and 3 should not have started
    assert!(started_rx.try_recv().is_err());
}

#[tokio::test]
async fn scale_to_zero_evicts_and_flushes_idle_sessions() {
    struct StatefulRunner {
        completed: mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl AgentRunner for StatefulRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            session.state.metadata.insert("flushed".into(), true.into());
            self.completed.send(()).unwrap();
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let key = session("idle-user");
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(StatefulRunner {
            completed: completed_tx,
        }),
        MultiplexerConfig {
            idle_timeout: Duration::ZERO,
            gc_interval: Duration::from_secs(60),
        },
    );
    multiplexer
        .route(InboundEvent::message(key.clone(), "idle-message", "hello"))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), completed_rx.recv())
        .await
        .unwrap()
        .unwrap();

    // With a zero idle timeout the session becomes collectable as soon as the
    // actor finishes the turn and returns to idle. Poll deterministically until
    // it is evicted (bounded) instead of relying on a paused-clock/real-clock
    // interplay, which raced under full-suite load.
    let mut evicted = 0;
    for _ in 0..200 {
        evicted = multiplexer.collect_garbage().await.unwrap();
        if evicted == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(evicted, 1, "idle session should be garbage-collected");
    assert!(!multiplexer.contains_session(&key));
    let state: String = sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
        .bind(key.storage_key())
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&state).unwrap()["metadata"]["flushed"],
        true
    );
}

#[tokio::test]
async fn route_reports_actor_startup_failure_instead_of_acknowledging_a_lost_event() {
    struct NeverRunner(AtomicBool);

    #[async_trait]
    impl AgentRunner for NeverRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("corrupt-state-user");
    sqlx::query(
        "INSERT INTO sessions (
            session_key, platform, guild_id, channel_id, thread_id, user_id,
            state_json, created_at, updated_at
         ) VALUES (?, 'discord', 'guild', 'channel', NULL, 'corrupt-state-user',
                   '{not-json', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(key.storage_key())
    .execute(database.pool())
    .await
    .unwrap();

    let runner = Arc::new(NeverRunner(AtomicBool::new(false)));
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        runner.clone(),
        MultiplexerConfig::default(),
    );
    let result = multiplexer
        .route(InboundEvent::message(
            key,
            "message-corrupt",
            "must not vanish",
        ))
        .await;

    assert!(result.is_err(), "route must surface actor load failure");
    assert!(!runner.0.load(Ordering::SeqCst));
    let persisted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(
        persisted, 0,
        "failed startup must not falsely persist/execute"
    );
}

#[tokio::test]
async fn dropping_multiplexer_releases_actor_cycle_and_flushes_dirty_state() {
    struct ReclaimRunner {
        completed: mpsc::UnboundedSender<()>,
        dropped: Arc<AtomicBool>,
    }

    impl Drop for ReclaimRunner {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl AgentRunner for ReclaimRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            session
                .state
                .metadata
                .insert("drop_flushed".into(), true.into());
            self.completed.send(()).unwrap();
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("drop-user");
    let dropped = Arc::new(AtomicBool::new(false));
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let runner = Arc::new(ReclaimRunner {
        completed: completed_tx,
        dropped: dropped.clone(),
    });
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        runner.clone(),
        MultiplexerConfig::default(),
    );

    multiplexer
        .route(InboundEvent::message(key.clone(), "drop-message", "hello"))
        .await
        .unwrap();
    completed_rx.recv().await.unwrap();
    drop(runner);
    drop(multiplexer);

    tokio::time::timeout(Duration::from_secs(2), async {
        while !dropped.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("actor-owned runner Arc should be released when multiplexer drops");

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let state: String =
                sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                    .bind(key.storage_key())
                    .fetch_one(database.pool())
                    .await
                    .unwrap();
            if serde_json::from_str::<serde_json::Value>(&state).unwrap()["metadata"]
                ["drop_flushed"]
                == true
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("dirty session state should flush during graceful actor shutdown");
}

#[tokio::test]
async fn gc_does_not_evict_an_actor_with_an_active_turn() {
    struct BlockingRunner {
        entered: mpsc::UnboundedSender<()>,
        release: Mutex<Option<oneshot::Receiver<()>>>,
        completed: mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl AgentRunner for BlockingRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.entered.send(()).unwrap();
            let release = self.release.lock().await.take().unwrap();
            let _ = release.await;
            self.completed.send(()).unwrap();
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let key = session("gc-race-user");
    let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
    let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
    let (release_tx, release_rx) = oneshot::channel();
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        Arc::new(BlockingRunner {
            entered: entered_tx,
            release: Mutex::new(Some(release_rx)),
            completed: completed_tx,
        }),
        MultiplexerConfig {
            idle_timeout: Duration::ZERO,
            gc_interval: Duration::from_secs(60),
        },
    );

    multiplexer
        .route(InboundEvent::message(key.clone(), "race-1", "first"))
        .await
        .unwrap();
    entered_rx.recv().await.unwrap();

    assert_eq!(multiplexer.collect_garbage().await.unwrap(), 0);
    assert!(multiplexer.contains_session(&key));

    release_tx.send(()).unwrap();
    completed_rx.recv().await.unwrap();
}

#[tokio::test]
async fn delivery_ledger_deduplicates_concurrent_claims_and_records_latency() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let ledger = DeliveryLedgerService::new(database.pool().clone());
    let event = InboundEvent::message(session("ledger-user"), "platform-message-1", "hello");

    let (first, second) = tokio::join!(
        ledger.record_incoming(&event),
        ledger.record_incoming(&event)
    );
    assert_ne!(first.unwrap(), second.unwrap());
    assert!(ledger.is_duplicate("platform-message-1").await.unwrap());

    ledger.mark_delivered("platform-message-1").await.unwrap();
    let entry = ledger.get("platform-message-1").await.unwrap().unwrap();
    assert_eq!(entry.status, "delivered");
    assert!(entry.completed_at.is_some());
    assert!(entry.processing_latency_ms.unwrap() >= 0);
}

#[tokio::test]
async fn transcript_level_inbound_dedup_skips_duplicate_platform_message_id() {
    struct CountingRunner {
        runs: std::sync::atomic::AtomicUsize,
        ran_tx: tokio::sync::mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl AgentRunner for CountingRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            let _ = self.ran_tx.send(());
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let (ran_tx, mut ran_rx) = tokio::sync::mpsc::unbounded_channel();
    let runner = Arc::new(CountingRunner {
        runs: std::sync::atomic::AtomicUsize::new(0),
        ran_tx,
    });
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        runner.clone(),
        MultiplexerConfig::default(),
    );

    let key = session("dedup-user");
    let event1 = InboundEvent::message(key.clone(), "plat-msg-unique-1", "first prompt");
    multiplexer.route(event1).await.unwrap();

    // Wait for the first turn to actually run (bounded), instead of a fixed sleep.
    tokio::time::timeout(Duration::from_secs(5), ran_rx.recv())
        .await
        .expect("first event should run within timeout")
        .expect("runner channel closed");
    assert_eq!(runner.runs.load(Ordering::SeqCst), 1);

    // Send another event with the exact same platform_message_id (simulating a
    // replayed webhook/gateway event after ledger eviction).
    let event2 = InboundEvent::message(key.clone(), "plat-msg-unique-1", "duplicate prompt");
    multiplexer.route(event2).await.unwrap();

    // The duplicate must NOT run: assert no run signal arrives within a bounded window.
    assert!(
        tokio::time::timeout(Duration::from_millis(300), ran_rx.recv())
            .await
            .is_err(),
        "duplicate platform_message_id must not trigger a second run"
    );
    assert_eq!(runner.runs.load(Ordering::SeqCst), 1);

    // Send a new event with a different platform_message_id.
    let event3 = InboundEvent::message(key.clone(), "plat-msg-unique-2", "second prompt");
    multiplexer.route(event3).await.unwrap();

    tokio::time::timeout(Duration::from_secs(5), ran_rx.recv())
        .await
        .expect("new event should run within timeout")
        .expect("runner channel closed");
    assert_eq!(runner.runs.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn flush_failure_replays_completed_turn_on_restart() {
    struct ReplayRunner {
        runs: Arc<std::sync::atomic::AtomicUsize>,
        ran_tx: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for ReplayRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            session
                .state
                .metadata
                .insert("turn_executed".into(), serde_json::json!(true));
            let _ = self.ran_tx.send(event.content);
            Ok(())
        }
    }

    struct TestDispatcher {
        typing_off: mpsc::UnboundedSender<()>,
        sent_messages: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl OutboundDispatcher for TestDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<(), OmonError> {
            match action {
                OutboundAction::Typing { active: false, .. } => {
                    let _ = self.typing_off.send(());
                }
                OutboundAction::SendMessage { content, .. } => {
                    self.sent_messages.lock().await.push(content);
                }
                _ => {}
            }
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let key = session("flush-replay-user");

    let delivery_id = "claim-flush-replay-1";
    let ledger = DeliveryLedgerService::new(pool.clone());
    let event = InboundEvent::message(key.clone(), "plat-msg-flush-1", "durable replay prompt")
        .with_delivery_id(delivery_id);
    ledger
        .record_incoming_as(&event, delivery_id)
        .await
        .unwrap();

    // Install trigger that fails flush (update of state_json on sessions)
    sqlx::query(
        "CREATE TRIGGER fail_sessions_state_flush BEFORE UPDATE OF state_json ON sessions
         BEGIN
             SELECT RAISE(ABORT, 'forced flush failure');
         END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    let runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (ran_tx, mut ran_rx) = mpsc::unbounded_channel();
    let (typing_off_tx, mut typing_off_rx) = mpsc::unbounded_channel();
    let sent_messages = Arc::new(Mutex::new(Vec::new()));

    let runner = Arc::new(ReplayRunner {
        runs: runs.clone(),
        ran_tx: ran_tx.clone(),
    });

    let dispatcher = Arc::new(TestDispatcher {
        typing_off: typing_off_tx.clone(),
        sent_messages: sent_messages.clone(),
    });

    let multiplexer = SessionMultiplexer::with_dispatcher(
        pool.clone(),
        runner.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
    );

    multiplexer.route(event).await.unwrap();

    // Turn 1 executes in-memory
    let content = tokio::time::timeout(Duration::from_secs(5), ran_rx.recv())
        .await
        .expect("turn 1 should run within timeout")
        .expect("channel open");
    assert_eq!(content, "durable replay prompt");
    assert_eq!(runs.load(Ordering::SeqCst), 1);

    // Bounded wait for the actor's terminal completion path (release typing).
    tokio::time::timeout(Duration::from_secs(5), typing_off_rx.recv())
        .await
        .expect("turn 1 terminal completion should release typing")
        .expect("channel open");

    // Simulate process crash / restart:
    // Drop old multiplexer and remove the failure trigger so subsequent flushes succeed.
    drop(multiplexer);
    sqlx::query("DROP TRIGGER fail_sessions_state_flush")
        .execute(&pool)
        .await
        .unwrap();

    let new_multiplexer = SessionMultiplexer::with_dispatcher(
        pool.clone(),
        runner.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
    );

    let recovered = SessionActor::recover_resume_pending_sessions(&pool, &new_multiplexer)
        .await
        .unwrap();
    assert_eq!(
        recovered, 1,
        "session with failed flush must remain marked resume_pending and be recovered on restart"
    );

    // Turn 2 executes via replay
    let replayed_content = tokio::time::timeout(Duration::from_secs(5), ran_rx.recv())
        .await
        .expect("replayed turn should run within timeout")
        .expect("channel open");
    assert_eq!(replayed_content, "durable replay prompt");
    assert_eq!(
        runs.load(Ordering::SeqCst),
        2,
        "turn must have executed twice (initial + replay)"
    );

    tokio::time::timeout(Duration::from_secs(5), typing_off_rx.recv())
        .await
        .expect("replayed turn terminal completion should release typing")
        .expect("channel open");

    // After replayed turn succeeds and flushes cleanly, resume_pending must be 0
    let remaining = omon_gateway::storage::count_resume_pending_sessions(&pool)
        .await
        .unwrap();
    assert_eq!(
        remaining, 0,
        "resume_pending must be cleared after clean flush"
    );

    // The session state must now be persisted in SQLite
    let state_json: String =
        sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(key.storage_key())
            .fetch_one(&pool)
            .await
            .unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();
    assert_eq!(
        state["metadata"]["turn_executed"], true,
        "flushed state must persist across recovery"
    );

    let entry = ledger.get(delivery_id).await.unwrap().unwrap();
    assert_eq!(
        entry.status, "delivered",
        "turn delivery claim must be marked delivered after replay"
    );
}

#[tokio::test]
async fn guild_lane_is_shared_per_bot_not_author() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    // Alice and Bob send unmentioned followups in thread 8 (guild 9, bot 84)
    let session_alice =
        SessionKey::new("discord", Some("9"), "8", Some("8"), "100").with_bot_id("84");
    let session_bob =
        SessionKey::new("discord", Some("9"), "8", Some("8"), "200").with_bot_id("84");

    // Invariant: Storage keys and SessionKeys must be canonical and independent of sender
    assert_eq!(
        session_alice.storage_key(),
        session_bob.storage_key(),
        "Alice and Bob in the same guild lane must produce the exact same storage key"
    );
    assert_eq!(
        session_alice, session_bob,
        "Alice and Bob SessionKeys must be identical"
    );

    struct RecordingRunner {
        executed: Mutex<Vec<(String, String)>>,
    }

    #[async_trait]
    impl AgentRunner for RecordingRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            session
                .state
                .metadata
                .insert(format!("turn_{}", event.content), true.into());
            self.executed
                .lock()
                .await
                .push((session.key.storage_key(), event.content.clone()));
            Ok(())
        }
    }

    let runner = Arc::new(RecordingRunner {
        executed: Mutex::new(Vec::new()),
    });
    struct TurnDispatcher {
        typing_off: mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl OutboundDispatcher for TurnDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<(), OmonError> {
            if let OutboundAction::Typing { active: false, .. } = action {
                let _ = self.typing_off.send(());
            }
            Ok(())
        }
    }

    let (typing_off_tx, mut typing_off_rx) = mpsc::unbounded_channel();
    let dispatcher = Arc::new(TurnDispatcher {
        typing_off: typing_off_tx,
    });
    let multiplexer = SessionMultiplexer::with_dispatcher(
        pool.clone(),
        runner.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
    );

    let event_alice = InboundEvent::message(session_alice.clone(), "msg-1", "alice_turn");
    multiplexer.route(event_alice).await.unwrap();

    let event_bob = InboundEvent::message(session_bob.clone(), "msg-2", "bob_turn");
    multiplexer.route(event_bob).await.unwrap();

    tokio::time::timeout(Duration::from_secs(2), typing_off_rx.recv())
        .await
        .expect("turn 1 terminal completion should release typing")
        .expect("channel open");
    tokio::time::timeout(Duration::from_secs(2), typing_off_rx.recv())
        .await
        .expect("turn 2 terminal completion should release typing")
        .expect("channel open");

    // Alice and Bob unmentioned followups must route to the SAME permanent lane key (not per-user)
    assert_eq!(
        multiplexer.active_sessions(),
        1,
        "Alice and Bob must share 1 active session lane in multiplexer"
    );

    let executed = runner.executed.lock().await.clone();
    assert_eq!(executed.len(), 2);
    assert_eq!(executed[0].0, session_alice.storage_key());
    assert_eq!(executed[1].0, session_bob.storage_key());

    // Bot 42 is connected in the same guild, but has its own lane
    let session_bot42 =
        SessionKey::new("discord", Some("9"), "8", Some("8"), "100").with_bot_id("42");
    assert_ne!(
        session_alice.storage_key(),
        session_bot42.storage_key(),
        "Bot 42 must not share lane key with Bot 84"
    );

    // Simulate restart / eviction:
    // Dropping multiplexer flushes dirty actor state to SQLite
    drop(multiplexer);

    // Verify durable state across simulated restart:
    // Session state stored in SQLite under session_alice.storage_key() must be accessible
    let state_json: String =
        sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
            .bind(session_alice.storage_key())
            .fetch_one(&pool)
            .await
            .unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();
    assert_eq!(state["metadata"]["turn_alice_turn"], true);
    assert_eq!(state["metadata"]["turn_bob_turn"], true);

    // Recreate multiplexer after restart and continue conversation
    let (typing_off_tx3, mut typing_off_rx3) = mpsc::unbounded_channel();
    let dispatcher3 = Arc::new(TurnDispatcher {
        typing_off: typing_off_tx3,
    });
    let restart_multiplexer = SessionMultiplexer::with_dispatcher(
        pool.clone(),
        runner.clone(),
        Some(dispatcher3),
        MultiplexerConfig::default(),
    );
    let event_bob_continuation = InboundEvent::message(session_bob.clone(), "msg-3", "bob_turn_2");
    restart_multiplexer
        .route(event_bob_continuation)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), typing_off_rx3.recv())
        .await
        .expect("turn 3 should complete")
        .expect("channel open");
    assert_eq!(restart_multiplexer.active_sessions(), 1);
}

#[tokio::test]
async fn thread_inherits_parent_profile() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    // Route configured on parent channel 200 selecting "model-x" in guild 100
    let parent_route = ProfileRoute {
        name: Some("parent-channel-route".into()),
        guild: Some(100),
        channel: Some(200),
        thread: None,
        enabled: true,
        model: Some("model-x".into()),
        system_prompt: None,
        enabled_toolsets: None,
        ..Default::default()
    };
    let profile_router = ProfileRouter::new(vec![parent_route]);

    let captured_model = Arc::new(Mutex::new(None));
    let sent_actions = Arc::new(Mutex::new(Vec::new()));
    let (done_tx, mut done_rx) = mpsc::unbounded_channel();

    struct ProfileRunner {
        captured_model: Arc<Mutex<Option<String>>>,
        dispatcher: Arc<ProfileDispatcher>,
    }

    #[async_trait]
    impl AgentRunner for ProfileRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            event: InboundEvent,
        ) -> Result<(), OmonError> {
            *self.captured_model.lock().await = session.state.active_model.clone();
            self.dispatcher
                .dispatch(OutboundAction::SendMessage {
                    session: session.key.clone(),
                    content: format!("reply to {}", event.content),
                    reply_to: Some(event.platform_message_id),
                })
                .await?;
            Ok(())
        }
    }

    struct ProfileDispatcher {
        sent_actions: Arc<Mutex<Vec<OutboundAction>>>,
        done_tx: mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl OutboundDispatcher for ProfileDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<(), OmonError> {
            match action {
                OutboundAction::SendMessage { .. } => {
                    self.sent_actions.lock().await.push(action);
                    let _ = self.done_tx.send(());
                }
                OutboundAction::Typing { active: false, .. } => {
                    let _ = self.done_tx.send(());
                }
                _ => {}
            }
            Ok(())
        }
    }

    let dispatcher = Arc::new(ProfileDispatcher {
        sent_actions: sent_actions.clone(),
        done_tx: done_tx.clone(),
    });
    let runner = Arc::new(ProfileRunner {
        captured_model: captured_model.clone(),
        dispatcher: dispatcher.clone(),
    });

    let multiplexer = SessionMultiplexer::with_profile_router(
        pool.clone(),
        runner.clone(),
        Some(dispatcher.clone()),
        MultiplexerConfig::default(),
        profile_router,
    );

    // Incoming message in thread 300 (parent 200, guild 100)
    let raw = serde_json::json!({
        "id": "1001",
        "channel_id": "300",
        "guild_id": "100",
        "author": {
            "id": "500",
            "username": "alice",
            "discriminator": "0001",
            "avatar": null,
            "bot": false
        },
        "content": "hello from thread 300",
        "timestamp": "2026-09-07T00:00:00Z",
        "tts": false,
        "mention_everyone": false,
        "mentions": [],
        "mention_roles": [],
        "attachments": [],
        "embeds": [],
        "pinned": false,
        "type": 0
    });
    let message: Message = serde_json::from_value(raw).unwrap();
    let config = InboundFilterConfig {
        allowed_users: &[500],
        active_threads: &[300],
        parent_channel_id: Some(200),
        primary_bot_id: Some(42),
        ..Default::default()
    };
    let event = message_to_inbound_with_config(
        &message,
        UserId::new(42),
        Some(ChannelType::PublicThread),
        &config,
    )
    .expect("message in thread 300 should convert to inbound event");

    multiplexer.route(event).await.unwrap();

    // Bounded wait for turn completion
    tokio::time::timeout(Duration::from_secs(2), done_rx.recv())
        .await
        .expect("turn should complete")
        .expect("channel open");

    // Check egress destination: reply must go to thread 300, not parent 200
    let sent = sent_actions.lock().await.clone();
    assert_eq!(sent.len(), 1, "exactly one reply message must be sent");
    if let OutboundAction::SendMessage { session, .. } = &sent[0] {
        let egress_target = session.thread_id.as_deref().unwrap_or(&session.channel_id);
        assert_eq!(
            egress_target, "300",
            "egress reply destination must remain thread 300"
        );
    } else {
        panic!("expected SendMessage action");
    }

    // Check model: thread 300 must inherit "model-x" configured on parent channel 200
    let active_model = captured_model.lock().await.clone();
    assert_eq!(
        active_model.as_deref(),
        Some("model-x"),
        "thread 300 must inherit profile route model from parent channel 200"
    );
}

#[tokio::test]
async fn terminal_outcomes_release_typing() {
    struct CapturingDispatcher {
        actions: Arc<Mutex<Vec<OutboundAction>>>,
        dispatched: Arc<tokio::sync::Notify>,
    }
    #[async_trait]
    impl OutboundDispatcher for CapturingDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<(), OmonError> {
            self.actions.lock().await.push(action);
            // notify_one keeps a permit, so a waiter that arrives late still wakes.
            self.dispatched.notify_one();
            Ok(())
        }
    }

    struct DynamicRunner {
        error_channel: Option<String>,
        blocking_channel: Option<String>,
        barrier: Arc<tokio::sync::Barrier>,
        done_tx: mpsc::UnboundedSender<String>,
    }
    #[async_trait]
    impl AgentRunner for DynamicRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            let channel = session.key.channel_id.clone();
            if Some(&channel) == self.error_channel.as_ref() {
                let _ = self.done_tx.send(channel);
                return Err(OmonError::Multiplexer("simulated runner failure".into()));
            }
            if Some(&channel) == self.blocking_channel.as_ref() {
                self.barrier.wait().await;
            }
            let _ = self.done_tx.send(channel);
            Ok(())
        }
    }

    let actions = Arc::new(Mutex::new(Vec::new()));
    let dispatched = Arc::new(tokio::sync::Notify::new());
    let dispatcher = Arc::new(CapturingDispatcher {
        actions: actions.clone(),
        dispatched: dispatched.clone(),
    });

    let (done_tx, mut done_rx) = mpsc::unbounded_channel();
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let runner = Arc::new(DynamicRunner {
        error_channel: Some("chan-err".into()),
        blocking_channel: Some("chan-block".into()),
        barrier: barrier.clone(),
        done_tx,
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let multiplexer = SessionMultiplexer::with_dispatcher(
        db.pool().clone(),
        runner.clone(),
        Some(dispatcher),
        MultiplexerConfig::default(),
    );

    // 1. Success outcome releases typing: [true, false]
    let session_ok = SessionKey::new("discord", Some("g1"), "chan-ok", None::<String>, "");
    let ev_ok = InboundEvent::message(session_ok.clone(), "1", "hello");
    multiplexer.route(ev_ok).await.unwrap();
    done_rx.recv().await.unwrap();

    // Bounded wait for actor to process turn completion and release typing
    for _ in 0..100 {
        let acts = actions.lock().await;
        let ok_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_ok => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        if ok_typings == vec![true, false] {
            break;
        }
        drop(acts);
        // Wait for the next dispatched action instead of a scheduler-dependent yield.
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), dispatched.notified()).await;
    }
    {
        let acts = actions.lock().await;
        let ok_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_ok => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            ok_typings,
            vec![true, false],
            "success turn must release typing"
        );
    }

    // 2. Error outcome releases typing: [true, false]
    let session_err = SessionKey::new("discord", Some("g1"), "chan-err", None::<String>, "");
    let ev_err = InboundEvent::message(session_err.clone(), "2", "fail");
    multiplexer.route(ev_err).await.unwrap();
    done_rx.recv().await.unwrap();

    for _ in 0..100 {
        let acts = actions.lock().await;
        let err_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_err => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        if err_typings == vec![true, false] {
            break;
        }
        drop(acts);
        // Wait for the next dispatched action instead of a scheduler-dependent yield.
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), dispatched.notified()).await;
    }
    {
        let acts = actions.lock().await;
        let err_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_err => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            err_typings,
            vec![true, false],
            "error turn must release typing"
        );
    }

    // 3. Multi-session isolation: session A (blocking) and session B (running)
    // While session A is blocked, session B completes and releases its typing,
    // while session A remains active (active: true)
    let session_block = SessionKey::new("discord", Some("g1"), "chan-block", None::<String>, "");
    let ev_block = InboundEvent::message(session_block.clone(), "3", "blocking");
    multiplexer.route(ev_block).await.unwrap();

    let session_fast = SessionKey::new("discord", Some("g1"), "chan-fast", None::<String>, "");
    let ev_fast = InboundEvent::message(session_fast.clone(), "4", "fast");
    multiplexer.route(ev_fast).await.unwrap();
    done_rx.recv().await.unwrap();

    // Fast session completes and releases typing
    for _ in 0..100 {
        let acts = actions.lock().await;
        let fast_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_fast => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        if fast_typings == vec![true, false] {
            break;
        }
        drop(acts);
        // Wait for the next dispatched action instead of a scheduler-dependent yield.
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), dispatched.notified()).await;
    }

    // Verify session_block is still active (active: true) and has NOT released typing yet
    {
        let acts = actions.lock().await;
        let block_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_block => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            block_typings,
            vec![true],
            "blocking session must remain typing: true while active"
        );
    }

    // Unblock session_block
    barrier.wait().await;
    done_rx.recv().await.unwrap();

    for _ in 0..100 {
        let acts = actions.lock().await;
        let block_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_block => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        if block_typings == vec![true, false] {
            break;
        }
        drop(acts);
        // Wait for the next dispatched action instead of a scheduler-dependent yield.
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), dispatched.notified()).await;
    }
    {
        let acts = actions.lock().await;
        let block_typings: Vec<bool> = acts
            .iter()
            .filter_map(|a| match a {
                OutboundAction::Typing { session, active } if session == &session_block => {
                    Some(*active)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            block_typings,
            vec![true, false],
            "unblocked session must release typing upon completion"
        );
    }
}

async fn run_user_insert_failure_scenario() {
    struct TrackingRunner {
        ran: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AgentRunner for TrackingRunner {
        async fn run(
            &self,
            _session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.ran.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let key = session("user-insert-fail");

    let delivery_id = "claim-user-insert-fail-1";
    let ledger = DeliveryLedgerService::new(pool.clone());
    let event = InboundEvent::message(
        key.clone(),
        "plat-msg-insert-fail-1",
        "user write blocked prompt",
    )
    .with_delivery_id(delivery_id);
    ledger
        .record_incoming_as(&event, delivery_id)
        .await
        .unwrap();

    // SQLite trigger RAISE(ABORT, 'write blocked') on user insert
    sqlx::query(
        "CREATE TRIGGER block_user_insert BEFORE INSERT ON messages
         BEGIN
             SELECT RAISE(ABORT, 'write blocked');
         END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    let ran = Arc::new(AtomicBool::new(false));
    let runner = Arc::new(TrackingRunner { ran: ran.clone() });

    let multiplexer = SessionMultiplexer::new(pool.clone(), runner, MultiplexerConfig::default());

    let turn_res = multiplexer.route_awaiting_turn(event).await;

    // Constituent 1 assertions:
    // 1. Engine runner must never be called when user insert persistence fails
    assert!(
        !ran.load(Ordering::SeqCst),
        "engine runner must not be called when user insert persistence fails"
    );

    // 2. Actor must surface the durable failure, not swallow it
    assert!(
        turn_res.is_err(),
        "actor must surface durable failure rather than reporting success"
    );

    // 3. Turn delivery claim must not be acknowledged as delivered
    let entry = ledger
        .get(delivery_id)
        .await
        .unwrap()
        .expect("delivery ledger entry exists");
    assert_ne!(
        entry.status, "delivered",
        "turn must never be acknowledged as delivered when user insert fails"
    );
    assert_eq!(
        entry.status, "failed",
        "turn delivery claim must be marked failed when user insert fails"
    );
}

async fn run_session_update_failure_scenario() {
    struct SuccessRunner {
        runs: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl AgentRunner for SuccessRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            session
                .state
                .metadata
                .insert("turn_executed".into(), serde_json::json!(true));
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let key = session("session-update-fail");

    let delivery_id = "claim-session-update-fail-1";
    let ledger = DeliveryLedgerService::new(pool.clone());
    let event = InboundEvent::message(
        key.clone(),
        "plat-msg-session-update-fail-1",
        "session write blocked prompt",
    )
    .with_delivery_id(delivery_id);
    ledger
        .record_incoming_as(&event, delivery_id)
        .await
        .unwrap();

    // Second variant: SQLite trigger RAISE(ABORT, 'write blocked') on session update
    sqlx::query(
        "CREATE TRIGGER block_session_update BEFORE UPDATE OF state_json ON sessions
         BEGIN
             SELECT RAISE(ABORT, 'write blocked');
         END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    let runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let runner = Arc::new(SuccessRunner { runs: runs.clone() });

    let multiplexer = SessionMultiplexer::new(pool.clone(), runner, MultiplexerConfig::default());

    let turn_res = multiplexer.route_awaiting_turn(event).await;

    // Runner was called because user insert succeeded
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "engine runner should execute for the turn"
    );

    // Constituent 2 assertions:
    // 1. Actor must surface the durable failure on completion flush failure, not swallow it
    assert!(
        turn_res.is_err(),
        "actor must surface durable flush failure rather than reporting success"
    );

    // 2. Turn delivery claim must not be acknowledged as delivered
    let entry = ledger
        .get(delivery_id)
        .await
        .unwrap()
        .expect("delivery ledger entry exists");
    assert_ne!(
        entry.status, "delivered",
        "turn must never be acknowledged as delivered when completion flush fails"
    );
    assert_eq!(
        entry.status, "failed",
        "turn delivery claim must be marked failed when completion flush fails"
    );
}

#[tokio::test]
async fn persistence_failure_never_acknowledges_turn_on_user_insert() {
    run_user_insert_failure_scenario().await;
}

#[tokio::test]
async fn persistence_failure_never_acknowledges_turn_on_session_update() {
    run_session_update_failure_scenario().await;
}

#[tokio::test]
async fn persistence_failure_never_acknowledges_turn() {
    run_user_insert_failure_scenario().await;
    run_session_update_failure_scenario().await;
}

#[tokio::test]
async fn live_controls_change_authoritative_session() {
    struct InspectRunner {
        models: Arc<tokio::sync::Mutex<Vec<Option<String>>>>,
    }

    #[async_trait]
    impl AgentRunner for InspectRunner {
        async fn run(
            &self,
            session: &mut SessionContext,
            _event: InboundEvent,
        ) -> Result<(), OmonError> {
            self.models
                .lock()
                .await
                .push(session.state.active_model.clone());
            session
                .state
                .metadata
                .insert("omo_thread_id".into(), serde_json::json!("remote-thread-1"));
            Ok(())
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let models = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let runner = Arc::new(InspectRunner {
        models: models.clone(),
    });
    let multiplexer = SessionMultiplexer::new(
        database.pool().clone(),
        runner,
        MultiplexerConfig::default(),
    );

    let key = session("user-control-test");
    multiplexer
        .route_awaiting_turn(InboundEvent::message(key.clone(), "msg-1", "hello"))
        .await
        .unwrap();

    multiplexer
        .set_model(&key, "model-beta".into())
        .await
        .unwrap();

    multiplexer
        .route_awaiting_turn(InboundEvent::message(
            key.clone(),
            "msg-2",
            "after model switch",
        ))
        .await
        .unwrap();
    {
        let captured = models.lock().await;
        assert_eq!(captured[1].as_deref(), Some("model-beta"));
    }

    multiplexer.reset(&key).await.unwrap();

    let ctx = multiplexer
        .session_context(&key)
        .await
        .unwrap()
        .expect("session context must exist");
    assert!(
        !ctx.state.metadata.contains_key("omo_thread_id"),
        "remote thread must be cleared"
    );
}

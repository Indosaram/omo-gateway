use omon_gateway::tools::Tool;
use omon_gateway::{
    Database, DeadTargetRegistry, DiscordEgress, OutboundAction, OutboundDispatcher, SessionKey,
};
use std::sync::Arc;

#[tokio::test]
async fn dead_targets_are_scoped_and_recoverable() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    let dead_targets = Arc::new(DeadTargetRegistry::new().with_pool(db.pool().clone()));
    let client_a = Arc::new(serenity::all::Http::new("token-a"));
    let client_b = Arc::new(serenity::all::Http::new("token-b"));
    let mut clients = std::collections::HashMap::new();
    clients.insert("bot-a".to_string(), client_a);
    clients.insert("bot-b".to_string(), client_b);

    let _egress = DiscordEgress::with_bot_clients("bot-a", clients)
        .unwrap()
        .with_dead_targets(dead_targets.clone());

    dead_targets.mark_dead_for_bot("bot-a", 42, 403, "HTTP 403: Missing Permissions");

    assert!(
        dead_targets.is_dead_for_bot("bot-a", 42),
        "bot-a must be dead on channel 42"
    );

    assert!(
        !dead_targets.is_dead_for_bot("bot-b", 42),
        "bot-b must NOT be dead on channel 42 when only bot-a encountered error"
    );

    dead_targets
        .mark_dead_and_persist("bot-a", 10003, 404, "HTTP 404: Unknown Channel")
        .await
        .unwrap();

    let recreated_registry = Arc::new(DeadTargetRegistry::new().with_pool(db.pool().clone()));
    recreated_registry.load_from_db(db.pool()).await.unwrap();

    assert!(
        recreated_registry.is_dead_for_bot("bot-a", 10003),
        "unknown-channel 10003 must persist across egress recreation"
    );

    let egress2 = DiscordEgress::new(Arc::new(serenity::all::Http::new("token-a")))
        .with_dead_targets(recreated_registry.clone());
    let session_dead = SessionKey::new("discord", None::<String>, "10003", None::<String>, "cron");
    let result = egress2
        .dispatch(OutboundAction::SendMessage {
            session: session_dead,
            content: "cron alert".into(),
            reply_to: None,
        })
        .await;

    assert!(
        result.is_err(),
        "subsequent send to dead target must be skipped with Err without claiming delivered"
    );

    recreated_registry
        .clear_and_persist("bot-a", 10003)
        .await
        .unwrap();
    assert!(
        !recreated_registry.is_dead_for_bot("bot-a", 10003),
        "target must be recovered after clear"
    );
}

struct DummyCronExecutor;

#[async_trait::async_trait]
impl omon_gateway::CronTaskExecutor for DummyCronExecutor {
    async fn execute(&self, _job: &omon_gateway::CronJob) -> omon_gateway::Result<Option<String>> {
        Ok(None)
    }
}

#[tokio::test]
async fn reminder_retains_bot_and_thread() {
    let pool = omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let bot_b = "bot-b";
    let session = omon_gateway::SessionKey::new("discord", None::<String>, "42", Some("43"), "u1")
        .with_bot_id(bot_b);
    let session_key = session.storage_key();

    sqlx::query(
        "INSERT INTO sessions (session_key, platform, channel_id, thread_id, user_id, state_json)
         VALUES (?, 'discord', '42', '43', 'u1', '{}')",
    )
    .bind(&session_key)
    .execute(&pool)
    .await
    .unwrap();

    let executor = Arc::new(DummyCronExecutor);
    let scheduler = omon_gateway::CronScheduler::new(pool.clone(), executor);
    let tool = omon_gateway::CronTool::with_scheduler(pool.clone(), Arc::new(scheduler.clone()));

    let add_res = tool
        .execute_with_context(
            serde_json::json!({
                "action": "add",
                "id": "reminder_1",
                "prompt": "meeting reminder",
                "expression": "every 1h"
            }),
            Some(&session),
        )
        .await
        .expect("CronTool add must succeed");

    let job_id = add_res
        .get("id")
        .or_else(|| add_res.get("job_id"))
        .and_then(serde_json::Value::as_str)
        .expect("job_id must be returned");

    let job = scheduler
        .get(job_id)
        .await
        .unwrap()
        .expect("job must exist");
    let payload = job.payload().unwrap();

    let destinations = omon_gateway::delivery_destinations(&payload).unwrap();
    assert_eq!(
        destinations.len(),
        1,
        "Reminder must resolve to exactly one destination"
    );
    assert_eq!(
        destinations[0].chat_id, "42",
        "Reminder must retain the channel delivery coordinate"
    );
    assert_eq!(
        destinations[0].thread_id.as_deref(),
        Some("43"),
        "Reminder must retain the thread delivery coordinate"
    );
    let job_bot_id = payload.get("bot_id").and_then(serde_json::Value::as_str);
    assert_eq!(
        job_bot_id,
        Some(bot_b),
        "Reminder must retain bot_id (bot-b)"
    );

    let dest_1 = omon_gateway::HermesOrigin {
        platform: "discord".into(),
        chat_id: "42".into(),
        thread_id: Some("43".into()),
        bot_id: Some(bot_b.into()),
        user_id: Some("u1".into()),
        chat_name: None,
        extra: std::collections::HashMap::new(),
    };
    let dest_2_fanout = omon_gateway::HermesOrigin {
        platform: "discord".into(),
        chat_id: "99".into(),
        thread_id: None,
        bot_id: Some(bot_b.into()),
        user_id: None,
        chat_name: None,
        extra: std::collections::HashMap::new(),
    };

    let m1 = omon_gateway::mirror_cron_delivery_to_session(
        &pool,
        job_id,
        Some(&session_key),
        &dest_1,
        "Reminder message 1",
    )
    .await
    .unwrap();
    assert!(
        m1,
        "Mirroring destination 1 to matching session must succeed"
    );

    let m2 = omon_gateway::mirror_cron_delivery_to_session(
        &pool,
        job_id,
        Some(&session_key),
        &dest_2_fanout,
        "Fanout message 2",
    )
    .await
    .unwrap();
    assert!(
        !m2,
        "Fanout destination 2 (channel 99) must NOT be mirrored into source session (channel 42)"
    );

    let msgs: Vec<(String,)> = sqlx::query_as("SELECT content FROM messages WHERE session_key = ?")
        .bind(&session_key)
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
        msgs.len(),
        1,
        "Source session must contain exactly 1 mirrored message"
    );
    assert_eq!(msgs[0].0, "Reminder message 1");
}

#[derive(Clone, Default)]
struct RecordingOutboundDispatcher {
    actions: Arc<std::sync::Mutex<Vec<OutboundAction>>>,
}

impl RecordingOutboundDispatcher {
    fn new() -> Self {
        Self::default()
    }

    fn actions(&self) -> Vec<OutboundAction> {
        self.actions.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl OutboundDispatcher for RecordingOutboundDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
        self.actions.lock().unwrap().push(action);
        Ok(())
    }
}

#[tokio::test]
async fn cron_final_obligation_recovers() {
    let pool = omon_gateway::storage::init_pool("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let bot_id = "bot-cron-test";
    let session =
        omon_gateway::SessionKey::new("discord", None::<String>, "100", None::<String>, "cron")
            .with_bot_id(bot_id);

    let ledger = omon_gateway::DeliveryLedgerService::new(pool.clone());

    // 1. Record an undelivered obligation (e.g. left by an interrupted cron execution or failed dispatch)
    let obl_id = "obl:cron:test-job-1:1000";
    ledger
        .record_obligation(obl_id, &session, "Durable alert payload")
        .await
        .expect("record_obligation must succeed");
    ledger
        .mark_obligation_attempting(obl_id)
        .await
        .expect("mark_obligation_attempting must succeed");
    // Simulate recycled PID with previous instance start stamp
    sqlx::query(
        "UPDATE delivery_obligations SET owner_started_at = '2026-01-01T00:00:00Z' WHERE id = ?",
    )
    .bind(obl_id)
    .execute(&pool)
    .await
    .unwrap();

    // Also insert an expired/stale obligation that should be pruned/abandoned
    let stale_obl_id = "obl:cron:stale-job:500";
    ledger
        .record_obligation(stale_obl_id, &session, "Stale payload")
        .await
        .expect("record_obligation must succeed");
    // Update created_at and owner_started_at of stale obligation to 2 days ago
    let two_days_ago = chrono::Utc::now() - chrono::Duration::days(2);
    sqlx::query("UPDATE delivery_obligations SET created_at = ?, owner_started_at = '2026-01-01T00:00:00Z' WHERE id = ?")
        .bind(two_days_ago)
        .bind(stale_obl_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify recoverable sweep finds the live obligation and abandons the stale one
    let mock_dispatcher = Arc::new(RecordingOutboundDispatcher::new());
    let recovered_count =
        omon_gateway::recover_pending_delivery_obligations(&pool, mock_dispatcher.clone())
            .await
            .expect("recover_pending_delivery_obligations must succeed");

    assert_eq!(
        recovered_count, 1,
        "Exactly 1 live obligation should be recovered"
    );

    // Verify mock dispatcher received the recovered delivery with bot_id preserved
    let dispatched = mock_dispatcher.actions();
    assert_eq!(
        dispatched.len(),
        1,
        "One delivery action must be dispatched"
    );
    match &dispatched[0] {
        OutboundAction::Stream { session: s, chunk } => {
            assert_eq!(
                s.bot_id.as_deref(),
                Some(bot_id),
                "bot_id must be preserved in recovery"
            );
            assert!(
                chunk.content.contains("Durable alert payload"),
                "Payload content must match"
            );
        }
        OutboundAction::SendMessage {
            session: s,
            content,
            ..
        } => {
            assert_eq!(
                s.bot_id.as_deref(),
                Some(bot_id),
                "bot_id must be preserved in recovery"
            );
            assert!(
                content.contains("Durable alert payload"),
                "Payload content must match"
            );
        }
        _ => panic!("Unexpected action type"),
    }

    // Verify obligation state in DB is now 'delivered'
    let obl = ledger
        .get_obligation(obl_id)
        .await
        .unwrap()
        .expect("Obligation must exist");
    assert_eq!(
        obl.state, "delivered",
        "Recovered obligation must be marked delivered"
    );

    // Verify stale obligation was marked abandoned
    let stale_obl = ledger
        .get_obligation(stale_obl_id)
        .await
        .unwrap()
        .expect("Stale obligation must exist");
    assert_eq!(
        stale_obl.state, "abandoned",
        "Expired obligation must be marked abandoned"
    );
}

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use omon_gateway::agent::{model_for_purpose, ModelPurpose, OmoBackend, OmoBackendConfig};
use omon_gateway::multiplexer::{
    MultiplexerConfig, OutboundDispatcher, SessionActor, SessionMultiplexer,
};
use omon_gateway::{
    AgentBackend, Database, InboundEvent, OutboundAction, SessionContext, SessionKey, SessionState,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _lock: parking_lot::MutexGuard<'static, ()>,
    original: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn new(vars: &[&'static str]) -> Self {
        let lock = ENV_LOCK.lock();
        let original = vars.iter().map(|&k| (k, std::env::var(k).ok())).collect();
        Self {
            _lock: lock,
            original,
        }
    }

    fn set(&self, key: &str, val: &str) {
        std::env::set_var(key, val);
    }

    fn remove(&self, key: &str) {
        std::env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.original {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

struct NoopDispatcher;

#[async_trait::async_trait]
impl OutboundDispatcher for NoopDispatcher {
    async fn dispatch(&self, _action: OutboundAction) -> omon_gateway::Result<()> {
        Ok(())
    }
}

#[test]
fn test_purpose_defaults_isolated() {
    let guard = EnvGuard::new(&["OMON_MODEL_LEAD", "OMON_MODEL_LARGE", "OMON_MODEL_SMALL"]);
    guard.remove("OMON_MODEL_LEAD");
    guard.remove("OMON_MODEL_LARGE");
    guard.remove("OMON_MODEL_SMALL");

    assert_eq!(
        model_for_purpose(ModelPurpose::Lead),
        "mahoquot/claude-opus-5-5"
    );
    assert_eq!(
        model_for_purpose(ModelPurpose::Large),
        "mahoquot/claude-opus-5-5"
    );
    assert_eq!(
        model_for_purpose(ModelPurpose::LaneLarge),
        "mahoquot/claude-opus-5-5"
    );

    for purpose in [
        ModelPurpose::Small,
        ModelPurpose::LaneSmall,
        ModelPurpose::Kibitzer,
        ModelPurpose::InboxWrite,
        ModelPurpose::Digest,
        ModelPurpose::JudgeFallback,
        ModelPurpose::CronFix,
    ] {
        assert_eq!(
            model_for_purpose(purpose),
            "inferhub/cb/deepseek-v4.1-flash"
        );
    }
}

#[test]
fn test_purpose_env_overrides_isolated() {
    let guard = EnvGuard::new(&["OMON_MODEL_LEAD", "OMON_MODEL_LARGE", "OMON_MODEL_SMALL"]);

    guard.set("OMON_MODEL_LEAD", "custom/test-lead");
    guard.set("OMON_MODEL_LARGE", "custom/test-large");
    guard.set("OMON_MODEL_SMALL", "custom/test-small");

    assert_eq!(model_for_purpose(ModelPurpose::Lead), "custom/test-lead");
    assert_eq!(model_for_purpose(ModelPurpose::Large), "custom/test-large");
    assert_eq!(
        model_for_purpose(ModelPurpose::LaneLarge),
        "custom/test-large"
    );

    for purpose in [
        ModelPurpose::Small,
        ModelPurpose::LaneSmall,
        ModelPurpose::Kibitzer,
        ModelPurpose::InboxWrite,
        ModelPurpose::Digest,
        ModelPurpose::JudgeFallback,
        ModelPurpose::CronFix,
    ] {
        assert_eq!(model_for_purpose(purpose), "custom/test-small");
    }

    guard.set("OMON_MODEL_LEAD", "   ");
    guard.set("OMON_MODEL_LARGE", "");
    guard.set("OMON_MODEL_SMALL", " \t \n ");

    assert_eq!(
        model_for_purpose(ModelPurpose::Lead),
        "mahoquot/claude-opus-5-5"
    );
    assert_eq!(
        model_for_purpose(ModelPurpose::Large),
        "mahoquot/claude-opus-5-5"
    );
    assert_eq!(
        model_for_purpose(ModelPurpose::Small),
        "inferhub/cb/deepseek-v4.1-flash"
    );
}

#[test]
fn test_actor_is_owner_lead_session_matrix() {
    let guard = EnvGuard::new(&[
        "DISCORD_ALLOWED_USERS",
        "DISCORD_HOME_CHANNEL",
        "DISCORD_HOME_CHANNEL_THREAD_ID",
        "OMON_OMOMEOW_BOT_ID",
    ]);
    guard.set("DISCORD_ALLOWED_USERS", "414011632306618368,111222333");
    guard.set("DISCORD_HOME_CHANNEL", "1234567890,9999999999");
    guard.set("DISCORD_HOME_CHANNEL_THREAD_ID", "9876543210");
    guard.set("OMON_OMOMEOW_BOT_ID", "1465631383862120451");

    let owner_dm = SessionKey::new(
        "discord",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    assert!(SessionActor::is_owner_lead_session(&owner_dm, None));
    assert!(SessionActor::is_owner_lead_session(
        &owner_dm,
        Some("omomeow")
    ));
    assert!(SessionActor::is_owner_lead_session(
        &owner_dm,
        Some("wawabot")
    ));

    let non_owner_dm = SessionKey::new(
        "discord",
        None::<String>,
        "888888888888888888",
        None::<String>,
        "888888888888888888",
    );
    assert!(!SessionActor::is_owner_lead_session(&non_owner_dm, None));

    let home_channel = SessionKey::new(
        "discord",
        Some("guild-1"),
        "1234567890",
        None::<String>,
        "other-user",
    );
    assert!(SessionActor::is_owner_lead_session(&home_channel, None));

    let home_thread = SessionKey::new(
        "discord",
        Some("guild-1"),
        "random-channel",
        Some("9876543210"),
        "other-user",
    );
    assert!(SessionActor::is_owner_lead_session(&home_thread, None));

    let non_home = SessionKey::new(
        "discord",
        Some("guild-1"),
        "random-channel-123",
        None::<String>,
        "other-user",
    );
    assert!(!SessionActor::is_owner_lead_session(&non_home, None));

    let mut other_bot = owner_dm.clone();
    other_bot.bot_id = Some("555555555555555555".into());
    assert!(!SessionActor::is_owner_lead_session(
        &other_bot,
        Some("other-bot")
    ));

    let non_discord = SessionKey::new(
        "slack",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    assert!(!SessionActor::is_owner_lead_session(&non_discord, None));
}

#[tokio::test]
async fn test_actor_load_context_model_resolution_matrix() {
    let guard = EnvGuard::new(&[
        "DISCORD_ALLOWED_USERS",
        "DISCORD_HOME_CHANNEL",
        "DISCORD_HOME_CHANNEL_THREAD_ID",
        "OMON_MODEL_LEAD",
    ]);
    guard.set("DISCORD_ALLOWED_USERS", "414011632306618368");
    guard.set("DISCORD_HOME_CHANNEL", "1234567890");
    guard.set("DISCORD_HOME_CHANNEL_THREAD_ID", "9876543210");
    guard.remove("OMON_MODEL_LEAD");

    let db = Database::connect("sqlite::memory:").await.unwrap();

    let owner_dm_key = SessionKey::new(
        "discord",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    let ctx = SessionActor::load_context(db.pool(), owner_dm_key.clone(), None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("mahoquot/claude-opus-5-5")
    );

    let persisted_no_model = SessionKey::new(
        "discord",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    let empty_state = serde_json::to_string(&SessionState::default()).unwrap();
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'discord', NULL, '414011632306618368', '414011632306618368', ?)"
    )
    .bind(persisted_no_model.storage_key())
    .bind(&empty_state)
    .execute(db.pool())
    .await
    .unwrap();

    let ctx = SessionActor::load_context(db.pool(), persisted_no_model.clone(), None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("mahoquot/claude-opus-5-5")
    );

    let persisted_explicit = SessionKey::new(
        "discord",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    let explicit_state = serde_json::to_string(&SessionState {
        active_model: Some("custom/explicit-override-model".into()),
        ..Default::default()
    })
    .unwrap();
    sqlx::query("UPDATE sessions SET state_json = ? WHERE session_key = ?")
        .bind(&explicit_state)
        .bind(persisted_explicit.storage_key())
        .execute(db.pool())
        .await
        .unwrap();

    let ctx = SessionActor::load_context(db.pool(), persisted_explicit.clone(), None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("custom/explicit-override-model")
    );

    let home_channel_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "1234567890",
        None::<String>,
        "other-user",
    );
    let ctx = SessionActor::load_context(db.pool(), home_channel_key, None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("mahoquot/claude-opus-5-5")
    );

    let home_thread_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "chan-1",
        Some("9876543210"),
        "other-user",
    );
    let ctx = SessionActor::load_context(db.pool(), home_thread_key, None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("mahoquot/claude-opus-5-5")
    );

    let non_owner_key = SessionKey::new(
        "discord",
        None::<String>,
        "777777777777777777",
        None::<String>,
        "777777777777777777",
    );
    let ctx = SessionActor::load_context(db.pool(), non_owner_key, None)
        .await
        .unwrap();
    assert_eq!(ctx.state.active_model, None);

    let shared_channel_key = SessionKey::new(
        "discord",
        Some("guild-1"),
        "random-channel",
        None::<String>,
        "other-user",
    );
    let ctx = SessionActor::load_context(db.pool(), shared_channel_key, None)
        .await
        .unwrap();
    assert_eq!(ctx.state.active_model, None);

    let cron_key = SessionKey::new(
        "cron",
        None::<String>,
        "job-nightly",
        None::<String>,
        "cron",
    );
    let cron_state = serde_json::to_string(&SessionState {
        active_model: Some("inferhub/cb/deepseek-v4.1-flash".into()),
        ..Default::default()
    })
    .unwrap();
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'cron', NULL, 'job-nightly', 'cron', ?)"
    )
    .bind(cron_key.storage_key())
    .bind(&cron_state)
    .execute(db.pool())
    .await
    .unwrap();

    let ctx = SessionActor::load_context(db.pool(), cron_key, None)
        .await
        .unwrap();
    assert_eq!(
        ctx.state.active_model.as_deref(),
        Some("inferhub/cb/deepseek-v4.1-flash")
    );
}

#[tokio::test]
async fn test_owner_dm_turn_start_routes_to_opus_mock_appserver() {
    let guard = EnvGuard::new(&[
        "DISCORD_ALLOWED_USERS",
        "DISCORD_HOME_CHANNEL",
        "DISCORD_HOME_CHANNEL_THREAD_ID",
        "OMON_MODEL_LEAD",
    ]);
    guard.set("DISCORD_ALLOWED_USERS", "414011632306618368");
    guard.remove("OMON_MODEL_LEAD");

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (captured_model_tx, captured_model_rx) = tokio::sync::oneshot::channel::<String>();

    let server_task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(req["method"], "initialize");
        ws.send(Message::text(
            json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "result": { "userAgent": "mock-appserver/1.0" }
            })
            .to_string(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert!(req["method"] == "thread/start" || req["method"] == "thread/resume");
        ws.send(Message::text(
            json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "result": { "thread": { "id": "th-owner-100" } }
            })
            .to_string(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(req["method"], "thread/settings/update");

        let settings_model = req
            .pointer("/params/model")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let _ = captured_model_tx.send(settings_model);
        ws.send(Message::text(
            json!({"jsonrpc": "2.0", "id": req["id"], "result": {}}).to_string(),
        ))
        .await
        .unwrap();

        let msg = ws.next().await.unwrap().unwrap();
        let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(req["method"], "turn/start");

        let turn_id = "turn-owner-100";
        ws.send(Message::text(
            json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "result": { "turn": { "id": turn_id, "status": "inProgress" } }
            })
            .to_string(),
        ))
        .await
        .unwrap();

        ws.send(Message::text(
            json!({
                "jsonrpc": "2.0",
                "method": "item/agentMessage/delta",
                "params": {
                    "threadId": "th-owner-100",
                    "turnId": turn_id,
                    "delta": "Greetings from Lead Opus."
                }
            })
            .to_string(),
        ))
        .await
        .unwrap();

        ws.send(Message::text(
            json!({
                "jsonrpc": "2.0",
                "method": "turn/completed",
                "params": {
                    "threadId": "th-owner-100",
                    "turn": { "id": turn_id, "status": "completed" }
                }
            })
            .to_string(),
        ))
        .await
        .unwrap();
    });

    let db = Database::connect("sqlite::memory:").await.unwrap();
    let mut config = OmoBackendConfig::new(format!("ws://{addr}"));
    config.connect_timeout = Duration::from_secs(5);
    config.request_timeout = Duration::from_secs(5);
    config.total_timeout = Duration::from_secs(5);

    let dispatcher = Arc::new(NoopDispatcher);
    let backend = Arc::new(OmoBackend::new(config, dispatcher));

    let mux_config = MultiplexerConfig::default();
    let multiplexer = SessionMultiplexer::new(db.pool().clone(), backend, mux_config);

    let owner_dm_key = SessionKey::new(
        "discord",
        None::<String>,
        "414011632306618368",
        None::<String>,
        "414011632306618368",
    );
    let event = InboundEvent::message(owner_dm_key, "msg-owner-1", "Plan our roadmap");

    multiplexer
        .route_awaiting_turn(event)
        .await
        .expect("owner DM turn routes and finishes");

    let received_model = captured_model_rx
        .await
        .expect("captured turn/start model from wire");
    assert_eq!(received_model, "mahoquot/claude-opus-5-5");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_unknown_small_model_fallback_to_deepseek_never_opus_mock_appserver() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (attempt1_model_tx, attempt1_model_rx) = tokio::sync::oneshot::channel::<String>();
    let (attempt2_model_tx, attempt2_model_rx) = tokio::sync::oneshot::channel::<String>();

    let server_task = tokio::spawn(async move {
        let mut attempt1_model_tx = Some(attempt1_model_tx);
        let mut attempt2_model_tx = Some(attempt2_model_tx);

        let (socket1, _) = listener.accept().await.unwrap();
        let mut ws1 = tokio_tungstenite::accept_async(socket1).await.unwrap();

        let mut requested_model = String::new();
        for method in [
            "initialize",
            "thread/resume",
            "thread/settings/update",
            "turn/start",
        ] {
            let msg = ws1.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);

            if method == "thread/settings/update" {
                let model = req["params"]["model"].as_str().unwrap().to_string();
                if let Some(tx) = attempt1_model_tx.take() {
                    let _ = tx.send(model.clone());
                }
                requested_model = model;
                ws1.send(Message::text(
                    json!({"jsonrpc": "2.0", "id": req["id"], "result": {}}).to_string(),
                ))
                .await
                .unwrap();
            } else if method == "turn/start" {
                ws1.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "error": { "code": -32603, "message": format!("model not found: {requested_model}") }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            } else {
                ws1.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "th-fallback-1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }

        let (socket2, _) = listener.accept().await.unwrap();
        let mut ws2 = tokio_tungstenite::accept_async(socket2).await.unwrap();

        for method in [
            "initialize",
            "thread/resume",
            "thread/settings/update",
            "turn/start",
        ] {
            let msg = ws2.next().await.unwrap().unwrap();
            let req: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
            assert_eq!(req["method"], method);

            if method == "thread/settings/update" {
                let model = req["params"]["model"].as_str().unwrap().to_string();
                assert_eq!(model, "inferhub/cb/deepseek-v4.1-flash");
                assert_ne!(model, "mahoquot/claude-opus-5-5");
                if let Some(tx) = attempt2_model_tx.take() {
                    let _ = tx.send(model);
                }
                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": {}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            } else if method == "turn/start" {
                let turn_id = "turn-fb-1";
                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "turn": { "id": turn_id, "status": "inProgress" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "method": "item/agentMessage/delta",
                        "params": {
                            "threadId": "th-fallback-1",
                            "turnId": turn_id,
                            "delta": "Small task fallback completed successfully."
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();

                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "method": "turn/completed",
                        "params": {
                            "threadId": "th-fallback-1",
                            "turn": { "id": turn_id, "status": "completed" }
                        }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            } else {
                ws2.send(Message::text(
                    json!({
                        "jsonrpc": "2.0",
                        "id": req["id"],
                        "result": { "thread": { "id": "th-fallback-1" } }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            }
        }
    });

    let mut config = OmoBackendConfig::new(format!("ws://{addr}"));
    config.connect_timeout = Duration::from_secs(5);
    config.request_timeout = Duration::from_secs(5);
    config.total_timeout = Duration::from_secs(10);
    config.fallback_model = "inferhub/cb/deepseek-v4.1-flash".to_string();

    let dispatcher = Arc::new(NoopDispatcher);
    let backend = OmoBackend::new(config, dispatcher);

    let key = SessionKey::new(
        "discord",
        Some("guild-small"),
        "channel-small",
        None::<String>,
        "user-1",
    );
    let mut session = SessionContext::new(key.clone());
    session.state.active_model = Some("unknown/unsupported-small-model".to_string());
    session
        .state
        .metadata
        .insert("omo_thread_id".into(), json!("th-fallback-1"));

    let event = InboundEvent::message(key, "msg-fb-1", "run small task");
    backend
        .run(&mut session, event)
        .await
        .expect("turn succeeds via fallback");

    let model1 = attempt1_model_rx.await.unwrap();
    let model2 = attempt2_model_rx.await.unwrap();
    assert_eq!(model1, "unknown/unsupported-small-model");
    assert_eq!(model2, "inferhub/cb/deepseek-v4.1-flash");
    assert_ne!(model2, "mahoquot/claude-opus-5-5");

    server_task.await.unwrap();
}

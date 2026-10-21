use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::api::SlackApi;
use super::commands::SlackCommands;
use super::egress::SlackEgress;
use super::ingress::SlackIngress;
use super::socket::{SocketClient, SocketEvent};
use super::SlackConfig;
use crate::Result;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(120);
/// Live ingress debounces split messages for a few seconds before routing them.
const RECONCILE_SETTLE: Duration = Duration::from_secs(60);

pub struct SlackRuntime {
    pub config: Arc<SlackConfig>,
    pub api: SlackApi,
    pub egress: Arc<SlackEgress>,
    pub ingress: Arc<SlackIngress>,
    pub commands: Arc<SlackCommands>,
    pub approvals: crate::SmartApprovalGuard,
    pub bot_user_id: String,
    pub team_id: String,
    /// Serializes startup backfill against the reconcile passes. Both read the same cursors and
    /// claim the same delivery rows, so running them at once lets a pass that starts mid-turn
    /// read a stale cursor and step past a message whose turn later failed.
    backfill_lock: tokio::sync::Mutex<()>,
}

pub struct SlackDeps {
    pub config: Arc<SlackConfig>,
    pub egress: Arc<SlackEgress>,
    pub commands: super::commands::SlackCommandDeps,
    pub inbox_actions: Option<Arc<dyn crate::inbox::proposals::Actions>>,
    /// Audio messages whose final reply should be uploaded as speech.
    pub voice_replies: Option<Arc<crate::SlackVoiceReplies>>,
    /// Browser huddle client. Absent when the gateway was built without the command wired.
    pub huddle: Option<Arc<dyn crate::HuddleControl>>,
}

/// Resolves the bot identity with `auth.test` before anything listens, so a bad token fails
/// startup of the Slack side instead of silently dropping every event.
pub async fn build(deps: SlackDeps) -> Result<SlackRuntime> {
    let api = SlackApi::new(deps.config.bot_token.clone());
    let auth = api.auth_test().await?;
    let approvals = deps.commands.approvals.clone();
    let mut ingress = SlackIngress::new(
        deps.config.clone(),
        api.clone(),
        deps.commands.multiplexer.clone(),
        deps.commands.pool.clone(),
        auth.user_id.clone(),
        auth.team_id.clone(),
    )
    .with_approvals(approvals.clone())
    .with_pairing(deps.commands.pairing.clone());
    if let Some(actions) = deps.inbox_actions {
        ingress = ingress.with_inbox_actions(actions);
    }
    if let Some(replies) = deps.voice_replies {
        ingress = ingress.with_voice_replies(replies);
    }
    let ingress = Arc::new(ingress);
    let mut commands = SlackCommands::new(
        deps.config.clone(),
        api.clone(),
        auth.user_id.clone(),
        deps.commands,
    );
    if let Some(huddle) = deps.huddle {
        commands = commands.with_huddle(huddle);
    }
    let commands = Arc::new(commands);
    Ok(SlackRuntime {
        config: deps.config,
        api,
        egress: deps.egress,
        ingress,
        commands,
        approvals,
        bot_user_id: auth.user_id,
        team_id: auth.team_id,
        backfill_lock: tokio::sync::Mutex::new(()),
    })
}

impl SlackRuntime {
    /// Routes messages missed while the gateway was down. Runs once, before `serve`.
    pub async fn backfill(&self) {
        if !self.config.missed_backfill {
            return;
        }
        let _pass = self.backfill_lock.lock().await;
        match super::backfill::run(&self.ingress).await {
            Ok(0) => {}
            Ok(routed) => tracing::info!(routed, "slack missed-message backfill routed messages"),
            Err(error) => tracing::warn!(%error, "slack missed-message backfill failed"),
        }
    }

    /// One settled reconcile pass, serialized with [`Self::backfill`].
    async fn reconcile_once(&self) {
        let _pass = self.backfill_lock.lock().await;
        match super::backfill::run_settled(&self.ingress, Some(RECONCILE_SETTLE)).await {
            Ok(0) => {}
            Ok(routed) => tracing::warn!(
                routed,
                "slack reconcile routed messages the socket never delivered"
            ),
            Err(error) => tracing::warn!(%error, "slack reconcile failed"),
        }
    }

    /// Re-runs the backfill every `RECONCILE_INTERVAL` while the gateway is up. Socket Mode can
    /// keep an open, pinging connection that stops delivering events (incident 2026-10-19: two
    /// owner replies at 11:50 never arrived and were only picked up by a restart), so the cursor
    /// sweep is what guarantees every message is eventually answered.
    pub async fn reconcile(self: Arc<Self>, shutdown: CancellationToken) {
        if !self.config.missed_backfill {
            return;
        }
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return,
                _ = tokio::time::sleep(RECONCILE_INTERVAL) => {}
            }
            self.reconcile_once().await;
        }
    }

    /// Hands debounced split messages to the multiplexer; called on every shutdown path.
    pub async fn flush(&self) {
        self.ingress.flush().await;
    }

    /// Runs Socket Mode until `shutdown`, reconnecting on its own. Each envelope is handled on
    /// its own task so a slow turn admission never delays the next ack.
    pub async fn serve(self: Arc<Self>, shutdown: CancellationToken) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SocketEvent>(256);
        let socket = SocketClient::new(self.api.clone(), self.config.app_token.clone());
        let socket_shutdown = shutdown.clone();
        let socket_task = tokio::spawn(async move { socket.run(tx, socket_shutdown).await });
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                event = rx.recv() => {
                    let Some(event) = event else { break };
                    let runtime = self.clone();
                    tokio::spawn(async move { runtime.handle(event).await });
                }
            }
        }
        socket_task.abort();
        Ok(())
    }

    async fn handle(&self, event: SocketEvent) {
        let outcome = match event {
            SocketEvent::EventsApi(event, _) => self.ingress.handle(event).await,
            SocketEvent::SlashCommand(payload) => self.commands.handle(&payload).await,
            SocketEvent::Interactive(payload) => {
                super::interactive::handle_block_actions(
                    &payload,
                    &self.config,
                    &self.approvals,
                    &self.egress,
                    &self.api,
                )
                .await
            }
        };
        if let Err(error) = outcome {
            tracing::warn!(%error, "slack event handling failed");
        }
    }
}

/// Restarts `serve` with backoff so a Slack outage never ends the gateway.
pub async fn supervise(runtime: Arc<SlackRuntime>, shutdown: CancellationToken) -> Result<()> {
    let mut backoff = Duration::from_secs(1);
    while !shutdown.is_cancelled() {
        match runtime.clone().serve(shutdown.clone()).await {
            Ok(()) if shutdown.is_cancelled() => break,
            Ok(()) => tracing::warn!("slack runtime stopped; restarting"),
            Err(error) => tracing::warn!(%error, "slack runtime failed; restarting"),
        }
        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = tokio::time::sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::response::Response;
    use serde_json::{json, Value};

    use super::*;
    use crate::slack::ingress::write_cursor;
    use crate::{Database, InboundEvent, MultiplexerConfig, SessionMultiplexer};

    const BOT: &str = "UBOT";
    const TEAM: &str = "T1";
    const OWNER: &str = "UOWNER";
    const STEP_TIMEOUT: Duration = Duration::from_secs(10);

    /// Holds the "hold" turn open until the test releases it, so a backfill pass can be observed
    /// mid-turn.
    struct BlockingRunner {
        entered: tokio::sync::mpsc::UnboundedSender<()>,
        release: Arc<tokio::sync::Semaphore>,
    }

    #[async_trait]
    impl crate::AgentRunner for BlockingRunner {
        async fn run(&self, _: &mut crate::SessionContext, event: InboundEvent) -> Result<()> {
            if event.content == "hold" {
                let _ = self.entered.send(());
                if let Ok(permit) = self.release.acquire().await {
                    permit.forget();
                }
            }
            Ok(())
        }
    }

    /// Reports every `conversations.history` request for `CFREE` on a channel.
    async fn fake() -> (String, tokio::sync::mpsc::UnboundedReceiver<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let app = axum::Router::new().fallback(move |req: Request| {
            let tx = tx.clone();
            async move {
                let method = req.uri().path().trim_start_matches("/api/").to_owned();
                let body = axum::body::to_bytes(req.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let body: Value = crate::slack::api::decode_test_body(&body);
                let channel = body["channel"].as_str().unwrap_or_default().to_owned();
                let reply = match (method.as_str(), channel.as_str()) {
                    ("conversations.open", _) => json!({ "ok": true, "channel": { "id": "D1" } }),
                    ("conversations.history", "CFREE") => {
                        let _ = tx.send(channel);
                        json!({ "ok": true, "messages": [
                            { "type": "message", "user": OWNER, "text": "hold", "ts": "101.000000" }
                        ]})
                    }
                    _ => json!({ "ok": true, "messages": [] }),
                };
                Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::from(reply.to_string()))
                    .unwrap()
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await });
        (base, rx)
    }

    struct Fixture {
        _db: Database,
        runtime: Arc<SlackRuntime>,
        entered: tokio::sync::mpsc::UnboundedReceiver<()>,
        hits: tokio::sync::mpsc::UnboundedReceiver<String>,
        release: Arc<tokio::sync::Semaphore>,
    }

    async fn fixture() -> Fixture {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let (entered_tx, entered) = tokio::sync::mpsc::unbounded_channel();
        let release = Arc::new(tokio::sync::Semaphore::new(0));
        let multiplexer = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(BlockingRunner {
                entered: entered_tx,
                release: release.clone(),
            }),
            MultiplexerConfig::default(),
        );
        let (base, hits) = fake().await;
        let config = Arc::new(SlackConfig {
            allowed_users: vec![OWNER.into()],
            free_response_channels: vec!["CFREE".into()],
            ..SlackConfig::test_default()
        });
        let api = SlackApi::with_base_url("xoxb-test".into(), format!("{base}/api"));
        let ingress = Arc::new(SlackIngress::new(
            config.clone(),
            api.clone(),
            multiplexer.clone(),
            pool.clone(),
            BOT.into(),
            TEAM.into(),
        ));
        write_cursor(&pool, BOT, "CFREE", "", "100.000000")
            .await
            .unwrap();
        let commands = Arc::new(SlackCommands::new(
            config.clone(),
            api.clone(),
            BOT,
            crate::slack::commands::SlackCommandDeps::new(multiplexer, pool),
        ));
        let runtime = Arc::new(SlackRuntime {
            config,
            egress: Arc::new(SlackEgress::new(api.clone())),
            api,
            ingress,
            commands,
            approvals: crate::SmartApprovalGuard::new(),
            bot_user_id: BOT.into(),
            team_id: TEAM.into(),
            backfill_lock: tokio::sync::Mutex::new(()),
        });
        Fixture {
            _db: db,
            runtime,
            entered,
            hits,
            release,
        }
    }

    #[tokio::test]
    async fn reconcile_pass_waits_for_the_startup_backfill() {
        let mut f = fixture().await;
        let backfill = tokio::spawn({
            let runtime = f.runtime.clone();
            async move { runtime.backfill().await }
        });
        tokio::time::timeout(STEP_TIMEOUT, f.entered.recv())
            .await
            .expect("the startup backfill should route the missed message")
            .expect("runner channel open");
        // The backfill pass already read the channel; clear that hit.
        while f.hits.try_recv().is_ok() {}
        assert!(
            f.runtime.backfill_lock.try_lock().is_err(),
            "the startup pass must hold the pass lock for the whole turn"
        );

        let runtime = f.runtime.clone();
        let mut reconcile = Box::pin(async move { runtime.reconcile_once().await });
        assert!(
            futures_util::poll!(reconcile.as_mut()).is_pending(),
            "a reconcile pass must not read a target while the startup backfill is mid-turn"
        );

        f.release.add_permits(1);
        tokio::time::timeout(STEP_TIMEOUT, backfill)
            .await
            .expect("the startup pass finishes once its turn is released")
            .unwrap();
        tokio::time::timeout(STEP_TIMEOUT, reconcile)
            .await
            .expect("the waiting pass runs once the startup pass releases the lock");
        assert!(
            f.hits.try_recv().is_ok(),
            "the waiting pass reads the target once the backfill finishes"
        );
    }
}

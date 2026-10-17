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

pub struct SlackRuntime {
    pub config: Arc<SlackConfig>,
    pub api: SlackApi,
    pub egress: Arc<SlackEgress>,
    pub ingress: Arc<SlackIngress>,
    pub commands: Arc<SlackCommands>,
    pub approvals: crate::SmartApprovalGuard,
    pub bot_user_id: String,
    pub team_id: String,
}

pub struct SlackDeps {
    pub config: Arc<SlackConfig>,
    pub egress: Arc<SlackEgress>,
    pub commands: super::commands::SlackCommandDeps,
    pub inbox_actions: Option<Arc<dyn crate::inbox::proposals::Actions>>,
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
    let ingress = Arc::new(ingress);
    let commands = Arc::new(SlackCommands::new(
        deps.config.clone(),
        api.clone(),
        auth.user_id.clone(),
        deps.commands,
    ));
    Ok(SlackRuntime {
        config: deps.config,
        api,
        egress: deps.egress,
        ingress,
        commands,
        approvals,
        bot_user_id: auth.user_id,
        team_id: auth.team_id,
    })
}

impl SlackRuntime {
    /// Routes messages missed while the gateway was down. Runs once, before `serve`.
    pub async fn backfill(&self) {
        if !self.config.missed_backfill {
            return;
        }
        match super::backfill::run(&self.ingress).await {
            Ok(0) => {}
            Ok(routed) => tracing::info!(routed, "slack missed-message backfill routed messages"),
            Err(error) => tracing::warn!(%error, "slack missed-message backfill failed"),
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

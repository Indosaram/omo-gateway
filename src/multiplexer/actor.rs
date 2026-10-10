use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::router::SessionMultiplexer;
use crate::{
    render_user_prompt, strip_leading_message_timestamps, DeliveryLedgerService, InboundEvent,
    OmonError, OutboundAction, ProfileRouter, Result, SessionContext, SessionKey, SessionState,
};

/// Sensible maximum number of pending events queued per session actor.
/// If the queue overflows, oldest events are preserved and new incoming events
/// are rejected to avoid unbounded memory growth.
const MAX_PENDING_EVENTS: usize = 64;

pub use crate::agent::AgentBackend;
pub use crate::agent::AgentBackend as AgentRunner;

#[async_trait]
pub trait OutboundDispatcher: Send + Sync + 'static {
    async fn dispatch(&self, action: OutboundAction) -> Result<()>;
}

pub(crate) enum ActorCommand {
    Event(Box<InboundEvent>),
    /// An event whose turn outcome is reported back once the turn reaches a terminal state.
    /// Startup backfill uses this to advance its durability cursor only after real success.
    EventWithAck {
        event: Box<InboundEvent>,
        ack: oneshot::Sender<Result<()>>,
    },
    Stop {
        reply: oneshot::Sender<Result<bool>>,
    },
    Steer {
        guidance: String,
        reply: oneshot::Sender<Result<bool>>,
    },
    EvictIfIdle {
        idle_timeout: Duration,
        reply: oneshot::Sender<Result<bool>>,
    },
    TouchActivity,
    SetModel {
        model: String,
        reply: oneshot::Sender<Result<()>>,
    },
    Reset {
        reply: oneshot::Sender<Result<()>>,
    },
    GetContext {
        reply: oneshot::Sender<SessionContext>,
    },
}

enum TurnOutcome {
    Completed(Result<()>),
    Stopped(oneshot::Sender<Result<bool>>),
    Shutdown,
}

pub struct SessionActor {
    context: SessionContext,
    receiver: mpsc::Receiver<ActorCommand>,
    runner: Arc<dyn AgentRunner>,
    dispatcher: Option<Arc<dyn OutboundDispatcher>>,
    pool: SqlitePool,
    last_active_at: tokio::time::Instant,
    dirty: bool,
    /// Set when a reset cleared the remote conversation binding, so the next flush
    /// must delete it instead of preserving the stored one for session stickiness.
    binding_cleared: bool,
    /// Outcome channel for the turn currently being executed, when the sender asked for one.
    pending_ack: Option<oneshot::Sender<Result<()>>>,
}

impl SessionActor {
    /// Recovers sessions marked `resume_pending` from previous runs or failed flushes,
    /// re-dispatching unfinished user turns or marking completed deliveries.
    pub async fn recover_resume_pending_sessions(
        pool: &SqlitePool,
        multiplexer: &SessionMultiplexer,
    ) -> Result<usize> {
        let pending_keys = crate::storage::fetch_resume_pending_session_keys(pool).await?;
        let mut resumed_count = 0;
        for session_key in pending_keys {
            let storage_key = session_key.storage_key();
            let is_suspended = crate::storage::is_session_suspended(pool, &storage_key).await?;
            let cleared = crate::storage::clear_session_resume_pending(pool, &storage_key).await?;
            if !cleared {
                continue;
            }
            if is_suspended {
                tracing::info!(
                    session = %session_key,
                    "skipping restart recovery for suspended session"
                );
                continue;
            }

            if let Some(unfinished) =
                crate::storage::find_last_unfinished_user_turn(pool, &storage_key).await?
            {
                let delivery_id: Option<String> = if let Some(ref pid) =
                    unfinished.platform_message_id
                {
                    sqlx::query_scalar(
                        "SELECT message_id FROM delivery_ledger WHERE session_key = ? AND (platform_message_id = ? OR message_id = ?) ORDER BY created_at DESC LIMIT 1",
                    )
                    .bind(&storage_key)
                    .bind(pid)
                    .bind(pid)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
                } else {
                    sqlx::query_scalar(
                        "SELECT message_id FROM delivery_ledger WHERE session_key = ? ORDER BY created_at DESC LIMIT 1",
                    )
                    .bind(&storage_key)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
                };

                let attachments: Vec<crate::MessageAttachment> =
                    serde_json::from_str(&unfinished.metadata_json).unwrap_or_default();
                let event = InboundEvent {
                    id: uuid::Uuid::parse_str(&unfinished.message_id)
                        .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                    session: session_key.clone(),
                    platform_message_id: String::new(),
                    delivery_id,
                    content: unfinished.content,
                    attachments,
                    received_at: unfinished.created_at,
                };
                tracing::info!(
                    session = %session_key,
                    "re-dispatching unfinished user turn on restart recovery"
                );
                if let Err(error) = multiplexer.route(event).await {
                    tracing::error!(
                        session = %session_key,
                        %error,
                        "failed to route resumed session event"
                    );
                } else {
                    resumed_count += 1;
                }
            } else {
                let delivery_id: Option<String> = sqlx::query_scalar(
                    "SELECT message_id FROM delivery_ledger WHERE session_key = ? ORDER BY created_at DESC LIMIT 1",
                )
                .bind(&storage_key)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some(del_id) = delivery_id {
                    let ledger = DeliveryLedgerService::new(pool.clone());
                    let _ = ledger.mark_delivered(&del_id).await;
                }
                resumed_count += 1;
            }
        }
        Ok(resumed_count)
    }

    pub(crate) async fn load(
        key: SessionKey,
        receiver: mpsc::Receiver<ActorCommand>,
        runner: Arc<dyn AgentRunner>,
        dispatcher: Option<Arc<dyn OutboundDispatcher>>,
        pool: SqlitePool,
        profile_router: Option<Arc<ProfileRouter>>,
    ) -> Result<Self> {
        let context = load_context(&pool, key, profile_router.as_deref()).await?;
        Ok(Self {
            context,
            receiver,
            runner,
            dispatcher,
            pool,
            last_active_at: tokio::time::Instant::now(),
            dirty: false,
            binding_cleared: false,
            pending_ack: None,
        })
    }

    pub(crate) async fn run(mut self) {
        let mut pending_events: VecDeque<ActorCommand> = VecDeque::new();
        loop {
            let command = if let Some(cmd) = pending_events.pop_front() {
                cmd
            } else {
                match self.receiver.recv().await {
                    Some(command) => command,
                    None => break,
                }
            };

            // An acked event is handled exactly like a plain event; the ack rides along on the
            // actor so every terminal outcome below can report the turn result to the sender.
            let command = match command {
                ActorCommand::EventWithAck { event, ack } => {
                    self.resolve_pending_ack(Err(OmonError::Multiplexer(
                        "superseded by a newer acked turn".into(),
                    )));
                    self.pending_ack = Some(ack);
                    ActorCommand::Event(event)
                }
                other => other,
            };

            match command {
                ActorCommand::Event(event) => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.context.updated_at = Utc::now();
                    self.context.state.suspended = false;
                    self.dirty = true;

                    if !event.platform_message_id.is_empty() {
                        match crate::storage::has_platform_message_id(
                            &self.pool,
                            &self.context.key.storage_key(),
                            &event.platform_message_id,
                        )
                        .await
                        {
                            Ok(true) => {
                                tracing::info!(
                                    session = %self.context.key,
                                    platform_message_id = %event.platform_message_id,
                                    "skipping replayed inbound turn: platform_message_id already exists in transcript"
                                );
                                self.complete_delivery(event.delivery_id.as_deref(), &Ok(()))
                                    .await;
                                self.resolve_pending_ack(Ok(()));
                                continue;
                            }
                            Ok(false) => {}
                            Err(error) => {
                                tracing::warn!(
                                    session = %self.context.key,
                                    platform_message_id = %event.platform_message_id,
                                    %error,
                                    "failed to check transcript dedup index"
                                );
                            }
                        }
                    }

                    if let Err(error) = self.persist_inbound(&event).await {
                        tracing::error!(
                            session = %self.context.key,
                            %error,
                            "failed to persist inbound event; aborting turn before side effects"
                        );
                        self.dirty = false;
                        self.complete_delivery(
                            event.delivery_id.as_deref(),
                            &Err(OmonError::Database(format!(
                                "failed to persist inbound event: {error}"
                            ))),
                        )
                        .await;
                        self.resolve_pending_ack(Err(OmonError::Database(format!(
                            "failed to persist inbound event: {error}"
                        ))));
                        continue;
                    }

                    let event_id = event.id;
                    let reply_to = event.platform_message_id.clone();
                    let platform_message_id = reply_to.clone();
                    let delivery_id = event.delivery_id.clone();
                    let cancellation = CancellationToken::new();
                    let mut turn_context = self.context.clone();

                    // Immediately broadcast typing indicator when turn execution begins
                    if !turn_context.key.user_id.starts_with("cron:") {
                        if let Some(dispatcher) = &self.dispatcher {
                            let _ = dispatcher
                                .dispatch(crate::OutboundAction::Typing {
                                    session: turn_context.key.clone(),
                                    active: true,
                                })
                                .await;
                        }
                    }

                    let runner = self.runner.clone();
                    let mut run = Box::pin(runner.run_cancelable(
                        &mut turn_context,
                        *event,
                        cancellation.clone(),
                    ));
                    let outcome = loop {
                        tokio::select! {
                            biased;
                            command = self.receiver.recv() => {
                                match command {
                                    Some(ActorCommand::Event(next)) => {
                                        if pending_events.len() < MAX_PENDING_EVENTS {
                                            pending_events.push_back(ActorCommand::Event(next));
                                        } else {
                                            tracing::warn!(
                                                session = %self.context.key,
                                                "pending turn queue full (max {}); dropping new event",
                                                MAX_PENDING_EVENTS
                                            );
                                            self.complete_delivery(
                                                next.delivery_id.as_deref(),
                                                &Err(OmonError::Multiplexer("pending turn queue full".into())),
                                            )
                                            .await;
                                        }
                                    }
                                    Some(ActorCommand::EventWithAck { event: next, ack }) => {
                                        if pending_events.len() < MAX_PENDING_EVENTS {
                                            pending_events.push_back(ActorCommand::EventWithAck { event: next, ack });
                                        } else {
                                            tracing::warn!(
                                                session = %self.context.key,
                                                "pending turn queue full (max {}); dropping new event",
                                                MAX_PENDING_EVENTS
                                            );
                                            self.complete_delivery(
                                                next.delivery_id.as_deref(),
                                                &Err(OmonError::Multiplexer("pending turn queue full".into())),
                                            )
                                            .await;
                                            let _ = ack.send(Err(OmonError::Multiplexer("pending turn queue full".into())));
                                        }
                                    }
                                    Some(ActorCommand::Stop { reply }) => {
                                        cancellation.cancel();
                                        while let Some(pending) = pending_events.pop_front() {
                                            match pending {
                                                ActorCommand::Event(p) => {
                                                    self.complete_delivery(
                                                        p.delivery_id.as_deref(),
                                                        &Err(OmonError::Multiplexer("stopped by user".into())),
                                                    )
                                                    .await;
                                                }
                                                ActorCommand::EventWithAck { event: p, ack } => {
                                                    self.complete_delivery(
                                                        p.delivery_id.as_deref(),
                                                        &Err(OmonError::Multiplexer("stopped by user".into())),
                                                    )
                                                    .await;
                                                    let _ = ack.send(Err(OmonError::Multiplexer("stopped by user".into())));
                                                }
                                                _ => {}
                                            }
                                        }
                                        break TurnOutcome::Stopped(reply);
                                    }
                                    Some(ActorCommand::EvictIfIdle { reply, .. }) => {
                                        let _ = reply.send(Ok(false));
                                    }
                                    Some(ActorCommand::Steer { guidance, reply }) => {
                                        // Steer off the actor loop so the running turn's stream
                                        // keeps draining while the RPC round trip is in flight.
                                        let runner = self.runner.clone();
                                        let context = self.context.clone();
                                        tokio::spawn(async move {
                                            let res = runner.steer(&context, &guidance).await;
                                            let _ = reply.send(res);
                                        });
                                    }
                                    Some(ActorCommand::TouchActivity) => {
                                        self.last_active_at = tokio::time::Instant::now();
                                    }
                                    Some(ActorCommand::SetModel { model, reply }) => {
                                        self.context.state.active_model = Some(model);
                                        self.dirty = true;
                                        let _ = reply.send(Ok(()));
                                    }
                                    Some(ActorCommand::Reset { reply }) => {
                                        cancellation.cancel();
                                        self.context.state.metadata.remove("omo_thread_id");
                                        self.binding_cleared = true;
                                        self.context.state = crate::SessionState::default();
                                        self.dirty = true;
                                        let _ = reply.send(Ok(()));
                                        break TurnOutcome::Shutdown;
                                    }
                                    Some(ActorCommand::GetContext { reply }) => {
                                        let _ = reply.send(self.context.clone());
                                    }
                                    None => {
                                        cancellation.cancel();
                                        while let Some(pending) = pending_events.pop_front() {
                                            match pending {
                                                ActorCommand::Event(p) => {
                                                    self.complete_delivery(
                                                        p.delivery_id.as_deref(),
                                                        &Err(OmonError::Multiplexer("session actor shutting down".into())),
                                                    )
                                                    .await;
                                                }
                                                ActorCommand::EventWithAck { event: p, ack } => {
                                                    self.complete_delivery(
                                                        p.delivery_id.as_deref(),
                                                        &Err(OmonError::Multiplexer("session actor shutting down".into())),
                                                    )
                                                    .await;
                                                    let _ = ack.send(Err(OmonError::Multiplexer("session actor shutting down".into())));
                                                }
                                                _ => {}
                                            }
                                        }
                                        break TurnOutcome::Shutdown;
                                    }
                                }
                            }
                            result = &mut run => break TurnOutcome::Completed(result),
                        }
                    };
                    drop(run);

                    // Durable remote binding must never be lost on failure or overwritten by a stale actor copy.
                    if let Some(thread_id) = turn_context.state.metadata.get("omo_thread_id") {
                        self.context
                            .state
                            .metadata
                            .insert("omo_thread_id".into(), thread_id.clone());
                    }

                    match outcome {
                        TurnOutcome::Completed(mut result) => {
                            if result.is_ok() {
                                self.context = turn_context;
                                if let Err(error) = self.flush_if_dirty().await {
                                    tracing::error!(
                                        session = %self.context.key,
                                        %error,
                                        "failed to flush session actor on turn completion"
                                    );
                                    let _ = crate::storage::mark_session_resume_pending(
                                        &self.pool,
                                        &self.context.key.storage_key(),
                                    )
                                    .await;
                                    result = Err(OmonError::Database(format!(
                                        "failed to flush session actor on turn completion: {error}"
                                    )));
                                } else {
                                    let _ = crate::storage::clear_session_resume_pending(
                                        &self.pool,
                                        &self.context.key.storage_key(),
                                    )
                                    .await;
                                }
                            } else {
                                self.dirty = false;
                            }
                            self.complete_delivery(delivery_id.as_deref(), &result)
                                .await;
                            self.resolve_pending_ack(match &result {
                                Ok(()) => Ok(()),
                                Err(error) => {
                                    Err(OmonError::Multiplexer(format!("turn failed: {error}")))
                                }
                            });
                            if let Some(dispatcher) = &self.dispatcher {
                                let reaction_msg_id = (!platform_message_id.is_empty())
                                    .then(|| platform_message_id.clone());
                                if let Some(msg_id) = reaction_msg_id {
                                    let emoji = match &result {
                                        Ok(()) => crate::models::PROCESSING_SUCCESS_EMOJI,
                                        Err(_) => crate::models::PROCESSING_FAILURE_EMOJI,
                                    };
                                    let _ = dispatcher
                                        .dispatch(OutboundAction::React {
                                            session: self.context.key.clone(),
                                            message_id: msg_id,
                                            emoji: emoji.to_string(),
                                            remove_others: true,
                                        })
                                        .await;
                                }
                            }
                            if let Err(error) = &result {
                                tracing::error!(session = %self.context.key, %error, "agent runner failed");
                                if let Some(dispatcher) = &self.dispatcher {
                                    let _ = dispatcher
                                        .dispatch(OutboundAction::SendMessage {
                                            session: self.context.key.clone(),
                                            content: error.to_string(),
                                            reply_to: Some(reply_to),
                                        })
                                        .await;
                                }
                            }
                            self.release_typing().await;
                        }
                        TurnOutcome::Stopped(reply) => {
                            self.context.state.suspended = true;
                            self.dirty = true;
                            let cancel_res = self
                                .interrupt_turn(event_id, delivery_id.as_deref(), "stopped by user")
                                .await;
                            self.release_typing().await;
                            let _ = self.flush_if_dirty().await;
                            if let Some(dispatcher) = &self.dispatcher {
                                let reaction_msg_id = (!platform_message_id.is_empty())
                                    .then(|| platform_message_id.clone());
                                if let Some(msg_id) = reaction_msg_id {
                                    let _ = dispatcher
                                        .dispatch(OutboundAction::React {
                                            session: self.context.key.clone(),
                                            message_id: msg_id,
                                            emoji: crate::models::PROCESSING_FAILURE_EMOJI
                                                .to_string(),
                                            remove_others: true,
                                        })
                                        .await;
                                }
                            }
                            self.resolve_pending_ack(Err(OmonError::Multiplexer(
                                "stopped by user".into(),
                            )));
                            let _ = reply.send(cancel_res.map(|_| true));
                        }
                        TurnOutcome::Shutdown => {
                            let _ = self
                                .interrupt_turn(
                                    event_id,
                                    delivery_id.as_deref(),
                                    "session actor shutting down",
                                )
                                .await;
                            self.release_typing().await;
                            if let Err(error) = crate::storage::mark_session_resume_pending(
                                &self.pool,
                                &self.context.key.storage_key(),
                            )
                            .await
                            {
                                tracing::error!(session = %self.context.key, %error, "failed to mark resume_pending on shutdown");
                            }
                            self.resolve_pending_ack(Err(OmonError::Multiplexer(
                                "session actor shutting down".into(),
                            )));
                        }
                    }
                    self.context.updated_at = Utc::now();
                }
                ActorCommand::Stop { reply } => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.context.state.suspended = true;
                    self.dirty = true;
                    while let Some(pending) = pending_events.pop_front() {
                        match pending {
                            ActorCommand::Event(p) => {
                                self.complete_delivery(
                                    p.delivery_id.as_deref(),
                                    &Err(OmonError::Multiplexer("stopped by user".into())),
                                )
                                .await;
                            }
                            ActorCommand::EventWithAck { event: p, ack } => {
                                self.complete_delivery(
                                    p.delivery_id.as_deref(),
                                    &Err(OmonError::Multiplexer("stopped by user".into())),
                                )
                                .await;
                                let _ =
                                    ack.send(Err(OmonError::Multiplexer("stopped by user".into())));
                            }
                            _ => {}
                        }
                    }
                    self.release_typing().await;
                    let _ = self.flush_if_dirty().await;
                    let _ = reply.send(Ok(false));
                }
                ActorCommand::TouchActivity => {
                    self.last_active_at = tokio::time::Instant::now();
                }
                ActorCommand::Steer { reply, .. } => {
                    // Turn is not currently running in this branch
                    let _ = reply.send(Ok(false));
                }
                ActorCommand::SetModel { model, reply } => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.context.state.active_model = Some(model);
                    self.dirty = true;
                    let flush_res = self.flush_if_dirty().await;
                    let _ = reply.send(flush_res);
                }
                ActorCommand::Reset { reply } => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.context.state.metadata.remove("omo_thread_id");
                    self.binding_cleared = true;
                    self.context.state = crate::SessionState::default();
                    self.dirty = true;
                    let flush_res = self.flush_if_dirty().await;
                    let _ = reply.send(flush_res);
                }
                ActorCommand::GetContext { reply } => {
                    let _ = reply.send(self.context.clone());
                }
                ActorCommand::EvictIfIdle {
                    idle_timeout,
                    reply,
                } => {
                    let idle = self.last_active_at.elapsed() > idle_timeout
                        && self.receiver.is_empty()
                        && pending_events.is_empty();
                    if idle {
                        let result = self.flush_if_dirty().await.map(|_| true);
                        let should_stop = result.is_ok();
                        let _ = reply.send(result);
                        if should_stop {
                            break;
                        }
                    } else {
                        let _ = reply.send(Ok(false));
                    }
                }
                ActorCommand::EventWithAck { event, ack } => {
                    // The loop head rewrites acked events into plain events, so reaching this
                    // arm means the invariant changed. Report it instead of panicking the lane.
                    tracing::error!(
                        session = %self.context.key,
                        platform_message_id = %event.platform_message_id,
                        "acked event reached the terminal command match; reporting failure without executing"
                    );
                    let _ = ack.send(Err(OmonError::Multiplexer(
                        "acked event was not dispatched".into(),
                    )));
                }
            }
        }

        // Dropping the multiplexer closes all strong senders. Flush dirty state
        // on that graceful channel shutdown so actor memory can be reclaimed
        // without silently discarding the last in-memory session mutation.
        if !pending_events.is_empty() {
            if let Err(error) = crate::storage::mark_session_resume_pending(
                &self.pool,
                &self.context.key.storage_key(),
            )
            .await
            {
                tracing::error!(session = %self.context.key, %error, "failed to mark resume_pending for remaining pending events");
            }
        }
        if let Err(error) = self.flush_if_dirty().await {
            tracing::error!(session = %self.context.key, %error, "failed to flush session actor during shutdown");
            let _ = crate::storage::mark_session_resume_pending(
                &self.pool,
                &self.context.key.storage_key(),
            )
            .await;
        }
    }

    async fn interrupt_turn(
        &self,
        event_id: uuid::Uuid,
        delivery_id: Option<&str>,
        reason: &str,
    ) -> Result<()> {
        let cancel_res = self.runner.cancel(&self.context).await;
        if let Err(ref error) = cancel_res {
            tracing::warn!(session = %self.context.key, %error, "runner cancellation cleanup failed");
        }
        if let Err(error) = self.rollback_partial_history(event_id).await {
            tracing::error!(session = %self.context.key, %error, "failed to roll back interrupted turn history");
        }
        if let Some(delivery_id) = delivery_id {
            let ledger = DeliveryLedgerService::new(self.pool.clone());
            if let Err(error) = ledger.mark_failed(delivery_id, reason).await {
                tracing::error!(%delivery_id, %error, "failed to mark interrupted delivery claim failed");
            }
        }
        tracing::info!(session = %self.context.key, %reason, "agent turn interrupted");
        cancel_res
    }

    async fn release_typing(&self) {
        if !self.context.key.user_id.starts_with("cron:") {
            if let Some(dispatcher) = &self.dispatcher {
                let _ = dispatcher
                    .dispatch(crate::OutboundAction::Typing {
                        session: self.context.key.clone(),
                        active: false,
                    })
                    .await;
            }
        }
    }

    async fn rollback_partial_history(&self, event_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            "DELETE FROM messages
             WHERE session_key = ? AND sequence > (
                 SELECT sequence FROM messages WHERE id = ? AND session_key = ?
             )",
        )
        .bind(self.context.key.storage_key())
        .bind(event_id.to_string())
        .bind(self.context.key.storage_key())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Reports the turn outcome to a waiting sender, if one asked for an ack.
    fn resolve_pending_ack(&mut self, outcome: Result<()>) {
        if let Some(ack) = self.pending_ack.take() {
            let _ = ack.send(outcome);
        }
    }

    async fn complete_delivery(&self, delivery_id: Option<&str>, result: &Result<()>) {
        let Some(delivery_id) = delivery_id else {
            return;
        };
        let ledger = DeliveryLedgerService::new(self.pool.clone());
        let completion = match result {
            Ok(()) => ledger.mark_delivered(delivery_id).await,
            Err(error) => ledger.mark_failed(delivery_id, error.to_string()).await,
        };
        if let Err(error) = completion {
            tracing::error!(%delivery_id, %error, "failed to complete delivery claim");
        }
    }

    async fn persist_inbound(&self, event: &InboundEvent) -> Result<()> {
        ensure_session(&self.pool, &self.context).await?;
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at, platform_message_id)
             VALUES (?, ?, 'user', ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(event.id.to_string())
        .bind(self.context.key.storage_key())
        .bind(strip_leading_message_timestamps(&render_user_prompt(event)))
        .bind(serde_json::to_string(&event.attachments).map_err(serialization_error)?)
        .bind(event.received_at)
        .bind(if event.platform_message_id.is_empty() {
            None
        } else {
            Some(&event.platform_message_id)
        })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn flush_if_dirty(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.flush().await?;
        self.dirty = false;
        self.binding_cleared = false;
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        ensure_session(&self.pool, &self.context).await?;
        let state = serde_json::to_string(&self.context.state).map_err(serialization_error)?;
        // An ordinary flush preserves an existing remote binding (session stickiness is a
        // product requirement). A flush following a reset must NOT: restoring the binding
        // there would resurrect pre-reset conversation context the user was told was cleared.
        let preserve_binding = if self.binding_cleared { 0i64 } else { 1i64 };
        sqlx::query(
            "UPDATE sessions
             SET state_json = CASE
                 WHEN ? = 1
                      AND json_extract(state_json, '$.metadata.omo_thread_id') IS NOT NULL
                      AND json_extract(?, '$.metadata.omo_thread_id') IS NULL
                 THEN json_set(?, '$.metadata.omo_thread_id', json_extract(state_json, '$.metadata.omo_thread_id'))
                 ELSE ?
             END,
             updated_at = ?
             WHERE session_key = ?",
        )
        .bind(preserve_binding)
        .bind(&state)
        .bind(&state)
        .bind(&state)
        .bind(self.context.updated_at)
        .bind(self.context.key.storage_key())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

async fn load_context(
    pool: &SqlitePool,
    key: SessionKey,
    profile_router: Option<&ProfileRouter>,
) -> Result<SessionContext> {
    let row: Option<(String, chrono::DateTime<Utc>, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT state_json, created_at, updated_at FROM sessions WHERE session_key = ?",
    )
    .bind(key.storage_key())
    .fetch_optional(pool)
    .await?;
    match row {
        Some((state_json, created_at, updated_at)) => {
            let mut state: SessionState =
                serde_json::from_str(&state_json).map_err(serialization_error)?;

            // Check for bot-specific profile override first if bot_id is present
            if let Some(bot_id) = key.bot_id.as_deref() {
                if let Ok(Some((model, prompt, toolsets))) = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
                    "SELECT model, system_prompt, enabled_toolsets FROM bot_profiles WHERE bot_id = ?"
                )
                .bind(bot_id)
                .fetch_optional(pool)
                .await {
                    if state.active_model.is_none() && model.is_some() {
                        state.active_model = model;
                    }
                    if state.system_prompt.is_none() && prompt.is_some() {
                        state.system_prompt = prompt;
                    }
                    if state.enabled_toolsets.is_none() && toolsets.is_some() {
                        state.enabled_toolsets = toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
                    }
                }
            }

            if let Some(router) = profile_router {
                if let Some(route) = router.match_session(&key) {
                    if state.active_model.is_none() {
                        state.active_model = route.model.clone();
                    }
                    if state.system_prompt.is_none() {
                        state.system_prompt = route.system_prompt.clone();
                    }
                    if state.enabled_toolsets.is_none() {
                        state.enabled_toolsets = route.enabled_toolsets.clone();
                    }
                    if let Some(toolsets) = &route.enabled_toolsets {
                        state
                            .metadata
                            .entry("enabled_toolsets".into())
                            .or_insert_with(|| serde_json::json!(toolsets));
                    }
                }
            }
            Ok(SessionContext {
                key,
                state,
                created_at,
                updated_at,
            })
        }
        None => {
            let mut context = SessionContext::new(key);
            if let Some(bot_id) = context.key.bot_id.as_deref() {
                if let Ok(Some((model, prompt, toolsets))) = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
                    "SELECT model, system_prompt, enabled_toolsets FROM bot_profiles WHERE bot_id = ?"
                )
                .bind(bot_id)
                .fetch_optional(pool)
                .await {
                    if model.is_some() {
                        context.state.active_model = model;
                    }
                    if prompt.is_some() {
                        context.state.system_prompt = prompt;
                    }
                    if toolsets.is_some() {
                        context.state.enabled_toolsets = toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
                    }
                }
            }
            if let Some(router) = profile_router {
                router.apply_to_session(&mut context);
            }
            Ok(context)
        }
    }
}

async fn ensure_session(pool: &SqlitePool, context: &SessionContext) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (
            session_key, platform, guild_id, channel_id, thread_id, user_id,
            state_json, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(context.key.storage_key())
    .bind(&context.key.platform)
    .bind(&context.key.guild_id)
    .bind(&context.key.channel_id)
    .bind(&context.key.thread_id)
    .bind(&context.key.user_id)
    .bind(serde_json::to_string(&context.state).map_err(serialization_error)?)
    .bind(context.created_at)
    .bind(context.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mirrors an inbound user message into the session transcript without running a turn.
///
/// The instant-steer path injects a message into a turn that is already executing, so no
/// actor turn ever persists it. Without this the message would be absent from history for
/// `/undo`, `/retry`, and transcript search even though the agent did see it.
pub async fn record_inbound_transcript(pool: &SqlitePool, event: &InboundEvent) -> Result<()> {
    sqlx::query(
        "INSERT INTO sessions (
            session_key, platform, guild_id, channel_id, thread_id, user_id,
            state_json, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, '{}', ?, ?)
         ON CONFLICT(session_key) DO NOTHING",
    )
    .bind(event.session.storage_key())
    .bind(&event.session.platform)
    .bind(&event.session.guild_id)
    .bind(&event.session.channel_id)
    .bind(&event.session.thread_id)
    .bind(&event.session.user_id)
    .bind(event.received_at)
    .bind(event.received_at)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at, platform_message_id)
         VALUES (?, ?, 'user', ?, ?, ?, ?)
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(event.id.to_string())
    .bind(event.session.storage_key())
    .bind(strip_leading_message_timestamps(&render_user_prompt(event)))
    .bind(serde_json::to_string(&event.attachments).map_err(serialization_error)?)
    .bind(event.received_at)
    .bind(if event.platform_message_id.is_empty() {
        None
    } else {
        Some(&event.platform_message_id)
    })
    .execute(pool)
    .await?;
    Ok(())
}

fn serialization_error(error: serde_json::Error) -> OmonError {
    OmonError::Multiplexer(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use tokio::sync::Barrier;

    fn test_session(user: &str) -> SessionKey {
        SessionKey::new("discord", Some("guild"), "channel", None::<String>, user)
    }

    struct TurnRecordingRunner {
        started: mpsc::UnboundedSender<String>,
        barrier: Arc<Barrier>,
        completed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for TurnRecordingRunner {
        async fn run(&self, _session: &mut SessionContext, event: InboundEvent) -> Result<()> {
            let _ = self.started.send(event.content.clone());
            if event.content == "blocking" {
                self.barrier.wait().await;
            }
            let _ = self.completed.send(event.content);
            Ok(())
        }
    }

    struct SteerRecordingRunner {
        started: mpsc::UnboundedSender<String>,
        barrier: Arc<Barrier>,
        steered: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl AgentRunner for SteerRecordingRunner {
        async fn run(&self, _session: &mut SessionContext, event: InboundEvent) -> Result<()> {
            let _ = self.started.send(event.content.clone());
            if event.content == "blocking" {
                self.barrier.wait().await;
            }
            Ok(())
        }

        async fn steer(&self, _session: &SessionContext, guidance: &str) -> Result<bool> {
            let _ = self.steered.send(guidance.to_string());
            Ok(true)
        }
    }

    #[tokio::test]
    async fn actor_steers_active_turn_and_reports_idle_as_not_steered() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-steer-test");
        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let (steered_tx, mut steered_rx) = mpsc::unbounded_channel();
        let barrier = Arc::new(Barrier::new(2));

        let runner = Arc::new(SteerRecordingRunner {
            started: started_tx,
            barrier: barrier.clone(),
            steered: steered_tx,
        });

        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(key.clone(), cmd_rx, runner, None, db.pool().clone(), None)
            .await
            .unwrap();
        let handle = tokio::spawn(actor.run());

        // With no turn executing the actor reports "not steered" so the caller
        // can fall back to the normal queued path instead of dropping the message.
        let (reply_tx, reply_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::Steer {
                guidance: "idle-guidance".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        assert!(!reply_rx.await.unwrap().unwrap());
        assert!(steered_rx.try_recv().is_err());

        // While a turn is blocked mid-flight the same command must reach the
        // runner's steer hook without disturbing the running turn.
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-1",
                "blocking",
            ))))
            .await
            .unwrap();
        assert_eq!(started_rx.recv().await.as_deref(), Some("blocking"));

        let (reply_tx, reply_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::Steer {
                guidance: "focus on tests".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        assert!(reply_rx.await.unwrap().unwrap());
        assert_eq!(steered_rx.recv().await.as_deref(), Some("focus on tests"));

        barrier.wait().await;
        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn actor_queues_turns_in_order_without_cancellation() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-queue-test");
        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
        let barrier = Arc::new(Barrier::new(2));

        let runner = Arc::new(TurnRecordingRunner {
            started: started_tx,
            barrier: barrier.clone(),
            completed: completed_tx,
        });

        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(key.clone(), cmd_rx, runner, None, db.pool().clone(), None)
            .await
            .unwrap();
        let handle = tokio::spawn(actor.run());

        // Send first blocking turn
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-1",
                "blocking",
            ))))
            .await
            .unwrap();

        assert_eq!(started_rx.recv().await.as_deref(), Some("blocking"));

        // Send two more events while first is blocking
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-2",
                "follow-up-1",
            ))))
            .await
            .unwrap();
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-3",
                "follow-up-2",
            ))))
            .await
            .unwrap();

        // Release the first turn
        barrier.wait().await;

        assert_eq!(completed_rx.recv().await.as_deref(), Some("blocking"));
        assert_eq!(started_rx.recv().await.as_deref(), Some("follow-up-1"));
        assert_eq!(completed_rx.recv().await.as_deref(), Some("follow-up-1"));
        assert_eq!(started_rx.recv().await.as_deref(), Some("follow-up-2"));
        assert_eq!(completed_rx.recv().await.as_deref(), Some("follow-up-2"));

        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn actor_stop_cancels_active_turn_immediately() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-stop-test");
        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
        let barrier = Arc::new(Barrier::new(2));

        let runner = Arc::new(TurnRecordingRunner {
            started: started_tx,
            barrier: barrier.clone(),
            completed: completed_tx,
        });

        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(key.clone(), cmd_rx, runner, None, db.pool().clone(), None)
            .await
            .unwrap();
        let handle = tokio::spawn(actor.run());

        // Send blocking turn
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-1",
                "blocking",
            ))))
            .await
            .unwrap();

        assert_eq!(started_rx.recv().await.as_deref(), Some("blocking"));

        // Send Stop command
        let (reply_tx, reply_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::Stop { reply: reply_tx })
            .await
            .unwrap();

        let stop_result = reply_rx.await.unwrap();
        assert!(stop_result.unwrap());

        // Turn did not complete successfully
        assert!(completed_rx.try_recv().is_err());

        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn actor_load_applies_profile_routes_to_fresh_and_preserves_explicit_model() {
        use crate::ProfileRoute;

        let db = Database::connect("sqlite::memory:").await.unwrap();
        let route = ProfileRoute {
            name: Some("profile-1".into()),
            guild: Some(123),
            channel: Some(456),
            thread: None,
            enabled: true,
            model: Some("profile-model".into()),
            system_prompt: Some("profile-prompt".into()),
            enabled_toolsets: Some(vec!["terminal".into(), "web".into()]),
            ..Default::default()
        };
        let router = Arc::new(ProfileRouter::new(vec![route]));

        // 1. Fresh session (not in DB)
        let key = SessionKey::new(
            "discord",
            Some("123"),
            "456",
            None::<String>,
            "user-profile-test",
        );
        let (_tx1, rx1) = mpsc::channel(32);
        let runner = Arc::new(TurnRecordingRunner {
            started: mpsc::unbounded_channel().0,
            barrier: Arc::new(Barrier::new(1)),
            completed: mpsc::unbounded_channel().0,
        });
        let actor = SessionActor::load(
            key.clone(),
            rx1,
            runner.clone(),
            None,
            db.pool().clone(),
            Some(router.clone()),
        )
        .await
        .unwrap();

        assert_eq!(
            actor.context.state.active_model.as_deref(),
            Some("profile-model")
        );
        assert_eq!(
            actor.context.state.system_prompt.as_deref(),
            Some("profile-prompt")
        );
        assert_eq!(
            actor.context.state.enabled_toolsets.as_deref(),
            Some(&["terminal".to_string(), "web".to_string()][..])
        );

        // 2. Existing session in DB with explicit model set via `/model`
        let key2 = SessionKey::new(
            "discord",
            Some("123"),
            "456",
            None::<String>,
            "user-explicit-model",
        );
        let explicit_state = SessionState {
            active_model: Some("explicit-user-model".into()),
            ..Default::default()
        };
        let explicit_json = serde_json::to_string(&explicit_state).unwrap();

        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'discord', '123', '456', 'user-explicit-model', ?)"
        )
        .bind(key2.storage_key())
        .bind(&explicit_json)
        .execute(db.pool())
        .await
        .unwrap();

        let (_tx2, rx2) = mpsc::channel(32);
        let actor2 = SessionActor::load(
            key2.clone(),
            rx2,
            runner.clone(),
            None,
            db.pool().clone(),
            Some(router.clone()),
        )
        .await
        .unwrap();

        // Explicit model must NOT be clobbered by profile
        assert_eq!(
            actor2.context.state.active_model.as_deref(),
            Some("explicit-user-model")
        );
        // But unset prompt and toolsets are populated from profile defaults
        assert_eq!(
            actor2.context.state.system_prompt.as_deref(),
            Some("profile-prompt")
        );
        assert_eq!(
            actor2.context.state.enabled_toolsets.as_deref(),
            Some(&["terminal".to_string(), "web".to_string()][..])
        );
    }

    #[derive(Default)]
    struct CapturingDispatcher {
        actions: tokio::sync::Mutex<Vec<OutboundAction>>,
    }

    #[async_trait]
    impl OutboundDispatcher for CapturingDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> Result<()> {
            self.actions.lock().await.push(action);
            Ok(())
        }
    }

    struct ScriptedFakeBackend {
        pool: SqlitePool,
        dispatcher: Arc<CapturingDispatcher>,
        chunks_to_emit: Vec<String>,
        final_assistant_message: String,
        completed: mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl AgentBackend for ScriptedFakeBackend {
        async fn run(&self, session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
            let stream_id = uuid::Uuid::new_v4();
            for (seq, chunk_text) in self.chunks_to_emit.iter().enumerate() {
                let is_final = seq + 1 == self.chunks_to_emit.len();
                self.dispatcher
                    .dispatch(OutboundAction::Stream {
                        session: session.key.clone(),
                        chunk: crate::StreamChunk {
                            stream_id,
                            sequence: seq as u64,
                            content: chunk_text.clone(),
                            is_final,
                            reply_to: None,
                        },
                    })
                    .await?;
            }

            let now = Utc::now();
            sqlx::query(
                "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) VALUES (?, ?, 'assistant', ?, '{}', ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(session.key.storage_key())
            .bind(&self.final_assistant_message)
            .bind(now)
            .execute(&self.pool)
            .await?;

            let _ = self.completed.send(());
            Ok(())
        }
    }

    #[tokio::test]
    async fn actor_turn_loop_with_scripted_fake_backend() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-fake-backend-test");
        let pool = db.pool().clone();

        let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
        let dispatcher = Arc::new(CapturingDispatcher::default());
        let backend = Arc::new(ScriptedFakeBackend {
            pool: pool.clone(),
            dispatcher: dispatcher.clone(),
            chunks_to_emit: vec!["Hello ".to_string(), "world!".to_string()],
            final_assistant_message: "Hello world!".to_string(),
            completed: completed_tx,
        });

        let delivery_id = "delivery_claim_test_1";
        let ledger = DeliveryLedgerService::new(pool.clone());
        let mut event = InboundEvent::message(key.clone(), "msg-platform-123", "Hello agent");
        event.delivery_id = Some(delivery_id.to_string());
        ledger
            .record_incoming_as(&event, delivery_id)
            .await
            .unwrap();

        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            backend,
            Some(dispatcher.clone()),
            pool.clone(),
            None,
        )
        .await
        .unwrap();

        let handle = tokio::spawn(actor.run());

        cmd_tx
            .send(ActorCommand::Event(Box::new(event)))
            .await
            .unwrap();

        // Wait until the backend turn has executed completely
        assert_eq!(completed_rx.recv().await, Some(()));

        // Allow actor loop to complete turn finalization and delivery ledger update
        drop(cmd_tx);
        handle.await.unwrap();

        // 1. Assert user message was persisted
        let user_msgs: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT role, content, platform_message_id FROM messages WHERE session_key = ? AND role = 'user'",
        )
        .bind(key.storage_key())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(user_msgs.len(), 1, "Expected 1 persisted user message");
        assert_eq!(user_msgs[0].0, "user");
        assert_eq!(user_msgs[0].1, "Hello agent");
        assert_eq!(user_msgs[0].2.as_deref(), Some("msg-platform-123"));

        // 2. Assert assistant chunks delivered via dispatcher
        let actions = dispatcher.actions.lock().await;
        let stream_chunks: Vec<&crate::StreamChunk> = actions
            .iter()
            .filter_map(|action| match action {
                OutboundAction::Stream { chunk, .. } => Some(chunk),
                _ => None,
            })
            .collect();
        assert_eq!(stream_chunks.len(), 2, "Expected 2 stream chunks delivered");
        assert_eq!(stream_chunks[0].sequence, 0);
        assert_eq!(stream_chunks[0].content, "Hello ");
        assert!(!stream_chunks[0].is_final);
        assert_eq!(stream_chunks[1].sequence, 1);
        assert_eq!(stream_chunks[1].content, "world!");
        assert!(stream_chunks[1].is_final);

        // 3. Assert assistant message persisted
        let assistant_msgs: Vec<(String, String)> = sqlx::query_as(
            "SELECT role, content FROM messages WHERE session_key = ? AND role = 'assistant'",
        )
        .bind(key.storage_key())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            assistant_msgs.len(),
            1,
            "Expected 1 persisted assistant message"
        );
        assert_eq!(assistant_msgs[0].0, "assistant");
        assert_eq!(assistant_msgs[0].1, "Hello world!");

        // 4. Assert delivery ledger completed
        let entry = ledger
            .get(delivery_id)
            .await
            .unwrap()
            .expect("Ledger entry must exist");
        assert_eq!(
            entry.status, "delivered",
            "Delivery ledger claim must be marked delivered"
        );
    }

    #[tokio::test]
    async fn turn_terminal_paths_release_typing() {
        let db = Database::connect("sqlite::memory:").await.unwrap();

        // 1. Success path: normal completion releases typing
        {
            let key = test_session("typing-success");
            let dispatcher = Arc::new(CapturingDispatcher::default());
            let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
            let backend = Arc::new(ScriptedFakeBackend {
                pool: db.pool().clone(),
                dispatcher: dispatcher.clone(),
                chunks_to_emit: vec!["done".to_string()],
                final_assistant_message: "done".to_string(),
                completed: completed_tx,
            });

            let (cmd_tx, cmd_rx) = mpsc::channel(32);
            let actor = SessionActor::load(
                key.clone(),
                cmd_rx,
                backend,
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let handle = tokio::spawn(actor.run());
            cmd_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key.clone(),
                    "msg-success",
                    "hello",
                ))))
                .await
                .unwrap();

            completed_rx.recv().await.unwrap();
            drop(cmd_tx);
            handle.await.unwrap();

            let actions = dispatcher.actions.lock().await;
            let typing_events: Vec<bool> = actions
                .iter()
                .filter_map(|action| match action {
                    OutboundAction::Typing { session, active } if session == &key => Some(*active),
                    _ => None,
                })
                .collect();
            assert_eq!(
                typing_events,
                vec![true, false],
                "success path must start and release typing"
            );
        }

        // 2. Error path: runner error releases typing
        {
            struct FailingRunner;
            #[async_trait]
            impl AgentRunner for FailingRunner {
                async fn run(
                    &self,
                    _session: &mut SessionContext,
                    _event: InboundEvent,
                ) -> Result<()> {
                    Err(OmonError::Llm("runner exploded".into()))
                }
            }

            let key = test_session("typing-error");
            let dispatcher = Arc::new(CapturingDispatcher::default());
            let (cmd_tx, cmd_rx) = mpsc::channel(32);
            let actor = SessionActor::load(
                key.clone(),
                cmd_rx,
                Arc::new(FailingRunner),
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let handle = tokio::spawn(actor.run());
            cmd_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key.clone(),
                    "msg-error",
                    "explode",
                ))))
                .await
                .unwrap();

            let (reply_tx, reply_rx) = oneshot::channel();
            cmd_tx
                .send(ActorCommand::Stop { reply: reply_tx })
                .await
                .unwrap();
            let _ = reply_rx.await.unwrap();
            drop(cmd_tx);
            handle.await.unwrap();

            let actions = dispatcher.actions.lock().await;
            let typing_events: Vec<bool> = actions
                .iter()
                .filter_map(|action| match action {
                    OutboundAction::Typing { session, active } if session == &key => Some(*active),
                    _ => None,
                })
                .collect();
            assert_eq!(
                typing_events,
                vec![true, false],
                "error path must start and release typing"
            );
        }

        // 3. Stop path: user stop cancels turn and releases typing
        {
            let key = test_session("typing-stop");
            let dispatcher = Arc::new(CapturingDispatcher::default());
            let (started_tx, mut started_rx) = mpsc::unbounded_channel();
            let barrier = Arc::new(Barrier::new(2));
            let runner = Arc::new(TurnRecordingRunner {
                started: started_tx,
                barrier: barrier.clone(),
                completed: mpsc::unbounded_channel().0,
            });

            let (cmd_tx, cmd_rx) = mpsc::channel(32);
            let actor = SessionActor::load(
                key.clone(),
                cmd_rx,
                runner,
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let handle = tokio::spawn(actor.run());
            cmd_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key.clone(),
                    "msg-stop",
                    "blocking",
                ))))
                .await
                .unwrap();

            started_rx.recv().await.unwrap();

            let (reply_tx, reply_rx) = oneshot::channel();
            cmd_tx
                .send(ActorCommand::Stop { reply: reply_tx })
                .await
                .unwrap();
            assert!(reply_rx.await.unwrap().unwrap());

            drop(cmd_tx);
            handle.await.unwrap();

            let actions = dispatcher.actions.lock().await;
            let typing_events: Vec<bool> = actions
                .iter()
                .filter_map(|action| match action {
                    OutboundAction::Typing { session, active } if session == &key => Some(*active),
                    _ => None,
                })
                .collect();
            assert_eq!(
                typing_events,
                vec![true, false],
                "stop path must start and release typing"
            );
        }

        // 4. Shutdown path: dropping cmd channel while turn is running releases typing
        {
            let key = test_session("typing-shutdown");
            let dispatcher = Arc::new(CapturingDispatcher::default());
            let (started_tx, mut started_rx) = mpsc::unbounded_channel();
            let barrier = Arc::new(Barrier::new(2));
            let runner = Arc::new(TurnRecordingRunner {
                started: started_tx,
                barrier: barrier.clone(),
                completed: mpsc::unbounded_channel().0,
            });

            let (cmd_tx, cmd_rx) = mpsc::channel(32);
            let actor = SessionActor::load(
                key.clone(),
                cmd_rx,
                runner,
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let handle = tokio::spawn(actor.run());
            cmd_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key.clone(),
                    "msg-shutdown",
                    "blocking",
                ))))
                .await
                .unwrap();

            started_rx.recv().await.unwrap();
            drop(cmd_tx);
            handle.await.unwrap();

            let actions = dispatcher.actions.lock().await;
            let typing_events: Vec<bool> = actions
                .iter()
                .filter_map(|action| match action {
                    OutboundAction::Typing { session, active } if session == &key => Some(*active),
                    _ => None,
                })
                .collect();
            assert_eq!(
                typing_events,
                vec![true, false],
                "shutdown path must start and release typing"
            );
        }

        // 5. Lane isolation: stopping lane A does not release typing for active lane B
        {
            let key_a = SessionKey::new(
                "discord",
                Some("guild"),
                "channel-a",
                None::<String>,
                "user-a",
            );
            let key_b = SessionKey::new(
                "discord",
                Some("guild"),
                "channel-b",
                None::<String>,
                "user-b",
            );
            let dispatcher = Arc::new(CapturingDispatcher::default());

            let (started_a_tx, mut started_a_rx) = mpsc::unbounded_channel();
            let (started_b_tx, mut started_b_rx) = mpsc::unbounded_channel();
            let barrier_a = Arc::new(Barrier::new(2));
            let barrier_b = Arc::new(Barrier::new(2));

            let runner_a = Arc::new(TurnRecordingRunner {
                started: started_a_tx,
                barrier: barrier_a.clone(),
                completed: mpsc::unbounded_channel().0,
            });
            let runner_b = Arc::new(TurnRecordingRunner {
                started: started_b_tx,
                barrier: barrier_b.clone(),
                completed: mpsc::unbounded_channel().0,
            });

            let (cmd_a_tx, cmd_a_rx) = mpsc::channel(32);
            let (cmd_b_tx, cmd_b_rx) = mpsc::channel(32);

            let actor_a = SessionActor::load(
                key_a.clone(),
                cmd_a_rx,
                runner_a,
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let actor_b = SessionActor::load(
                key_b.clone(),
                cmd_b_rx,
                runner_b,
                Some(dispatcher.clone()),
                db.pool().clone(),
                None,
            )
            .await
            .unwrap();

            let handle_a = tokio::spawn(actor_a.run());
            let handle_b = tokio::spawn(actor_b.run());

            cmd_a_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key_a.clone(),
                    "msg-a",
                    "blocking",
                ))))
                .await
                .unwrap();
            cmd_b_tx
                .send(ActorCommand::Event(Box::new(InboundEvent::message(
                    key_b.clone(),
                    "msg-b",
                    "blocking",
                ))))
                .await
                .unwrap();

            started_a_rx.recv().await.unwrap();
            started_b_rx.recv().await.unwrap();

            let (reply_a_tx, reply_a_rx) = oneshot::channel();
            cmd_a_tx
                .send(ActorCommand::Stop { reply: reply_a_tx })
                .await
                .unwrap();
            assert!(reply_a_rx.await.unwrap().unwrap());

            {
                let actions = dispatcher.actions.lock().await;
                let typing_a: Vec<bool> = actions
                    .iter()
                    .filter_map(|action| match action {
                        OutboundAction::Typing { session, active } if session == &key_a => {
                            Some(*active)
                        }
                        _ => None,
                    })
                    .collect();
                let typing_b: Vec<bool> = actions
                    .iter()
                    .filter_map(|action| match action {
                        OutboundAction::Typing { session, active } if session == &key_b => {
                            Some(*active)
                        }
                        _ => None,
                    })
                    .collect();

                assert_eq!(
                    typing_a,
                    vec![true, false],
                    "lane A stopped and released typing"
                );
                assert_eq!(
                    typing_b,
                    vec![true],
                    "lane B typing must remain active while lane A stopped"
                );
            }

            barrier_b.wait().await;
            drop(cmd_a_tx);
            drop(cmd_b_tx);
            handle_a.await.unwrap();
            handle_b.await.unwrap();

            let actions = dispatcher.actions.lock().await;
            let typing_b: Vec<bool> = actions
                .iter()
                .filter_map(|action| match action {
                    OutboundAction::Typing { session, active } if session == &key_b => {
                        Some(*active)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                typing_b,
                vec![true, false],
                "lane B completed and released typing"
            );
        }
    }
}

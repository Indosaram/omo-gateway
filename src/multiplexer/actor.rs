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

/// How many recently completed inbound `platform_message_id`s an actor remembers in memory.
/// Completion is otherwise inferred from the transcript (an assistant reply after the user row).
const MAX_REMEMBERED_COMPLETED_INBOUND: usize = 256;

/// A queued Discord message dropped at shutdown has no transcript row, and the live path
/// already advanced the backfill cursor past it, so neither restart recovery nor backfill
/// would ever see it again. Moving the cursor back to just before it lets the next
/// startup backfill fetch and route it.
async fn rewind_backfill_cursor(pool: &SqlitePool, event: &InboundEvent) {
    if event.session.platform == crate::slack::PLATFORM {
        rewind_slack_backfill_cursor(pool, event).await;
        return;
    }
    if event.session.platform != "discord" {
        return;
    }
    let (Some(bot_id), Ok(own_id)) = (
        event.session.bot_id.as_deref(),
        event.platform_message_id.trim().parse::<u64>(),
    ) else {
        return;
    };
    // A debounced batch carries only its last message id; its earlier messages are
    // constituents of the same delivery and must be refetched too.
    let earliest_constituent: Option<i64> = match event.delivery_id.as_deref() {
        Some(delivery_id) => sqlx::query_scalar(
            "SELECT MIN(CAST(substr(constituent_id, 9) AS INTEGER))
             FROM delivery_ledger_constituents
             WHERE parent_delivery_id = ? AND constituent_id LIKE 'discord:%'",
        )
        .bind(delivery_id)
        .fetch_one(pool)
        .await
        .ok()
        .flatten(),
        None => None,
    };
    let message_id = earliest_constituent
        .and_then(|id| u64::try_from(id).ok())
        .filter(|id| *id > 0)
        .map_or(own_id, |id| id.min(own_id));
    let channel_id = event
        .session
        .thread_id
        .as_deref()
        .unwrap_or(&event.session.channel_id);
    let before = message_id.saturating_sub(1).to_string();
    // Cursor ids are numeric strings; compare as integers so "9" < "10".
    let result = sqlx::query(
        "UPDATE discord_bot_cursors SET last_message_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE bot_id = ? AND channel_id = ? AND CAST(last_message_id AS INTEGER) >= ?",
    )
    .bind(&before)
    .bind(bot_id)
    .bind(channel_id)
    .bind(message_id as i64)
    .execute(pool)
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, channel_id, "failed to rewind backfill cursor for a dropped queued message");
    }
}

/// Slack counterpart of `rewind_backfill_cursor` over `slack_cursors`, keyed by
/// (bot, channel, thread_ts or ''). Slack ts values are fixed-width `secs.micros`, so they
/// compare as strings once the earliest dropped message is known.
async fn rewind_slack_backfill_cursor(pool: &SqlitePool, event: &InboundEvent) {
    let Some(bot_id) = event.session.bot_id.as_deref() else {
        return;
    };
    let own_ts = event.platform_message_id.trim();
    if parse_slack_ts(own_ts).is_none() {
        return;
    }
    let constituents: Vec<String> = match event.delivery_id.as_deref() {
        Some(delivery_id) => sqlx::query_scalar(
            "SELECT constituent_id FROM delivery_ledger_constituents
             WHERE parent_delivery_id = ? AND constituent_id LIKE 'slack:%'",
        )
        .bind(delivery_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default(),
        None => Vec::new(),
    };
    let earliest = constituents
        .iter()
        .filter_map(|id| id.rsplit(':').next())
        .chain(std::iter::once(own_ts))
        .filter_map(|ts| parse_slack_ts(ts).map(|parsed| (parsed, ts)))
        .min()
        .map(|(parsed, _)| parsed);
    let Some(before) = earliest.and_then(slack_ts_before) else {
        return;
    };
    let thread_ts = event.session.thread_id.as_deref().unwrap_or("");
    let result = sqlx::query(
        "UPDATE slack_cursors SET last_ts = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE bot_id = ? AND channel_id = ? AND thread_ts = ? AND last_ts >= ?",
    )
    .bind(&before)
    .bind(bot_id)
    .bind(&event.session.channel_id)
    .bind(thread_ts)
    .bind(&before)
    .execute(pool)
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, channel_id = %event.session.channel_id, "failed to rewind Slack backfill cursor for a dropped queued message");
    }
}

fn parse_slack_ts(ts: &str) -> Option<(u64, u32)> {
    let (secs, micros) = ts.split_once('.')?;
    if micros.len() != 6 {
        return None;
    }
    Some((secs.parse().ok()?, micros.parse().ok()?))
}

fn slack_ts_before((secs, micros): (u64, u32)) -> Option<String> {
    match micros {
        0 => secs.checked_sub(1).map(|secs| format!("{secs}.999999")),
        micros => Some(format!("{secs}.{:06}", micros - 1)),
    }
}

pub use crate::agent::AgentBackend;
pub use crate::agent::AgentBackend as AgentRunner;

#[async_trait]
pub trait OutboundDispatcher: Send + Sync + 'static {
    async fn dispatch(&self, action: OutboundAction) -> Result<()>;

    async fn owner_thread_is_private(&self, _session: &SessionKey) -> bool {
        false
    }

    async fn dispatch_direct_reply(&self, session: SessionKey, content: String) -> Result<()> {
        self.dispatch(OutboundAction::SendMessage {
            session,
            content,
            reply_to: None,
        })
        .await
    }

    /// Posts one message and returns its platform id, so it can be edited in place later.
    /// `None` means this transport cannot report message ids.
    async fn post_status(
        &self,
        _session: &SessionKey,
        _content: String,
        _reply_to: Option<String>,
    ) -> Result<Option<String>> {
        Ok(None)
    }
}

pub(crate) enum ActorCommand {
    Event(Box<InboundEvent>),
    EventIfIdle {
        event: Box<InboundEvent>,
        reply: oneshot::Sender<Result<()>>,
    },
    IsIdle {
        reply: oneshot::Sender<bool>,
    },
    WaitIdle {
        reply: oneshot::Sender<()>,
    },
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
    SetYolo {
        yolo: bool,
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
    /// A mid-turn `/reset`: the turn is abandoned but the actor keeps serving the session.
    Reset(oneshot::Sender<Result<()>>),
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
    inbox_event_tx: Option<crate::inbox::watchers::InboxEventSender>,
    /// Kept so a reset can rebuild the profile-derived defaults (toolsets, prompt, model).
    profile_router: Option<Arc<ProfileRouter>>,
    /// Inbound `platform_message_id`s whose turn finished successfully in this actor. A transcript
    /// row alone only proves the message was persisted, not that its turn completed.
    completed_inbound_ids: VecDeque<String>,
}

impl SessionActor {
    /// Returns a reference to the actor's session context.
    pub fn context(&self) -> &SessionContext {
        &self.context
    }

    /// Evaluates whether the given session qualifies as the owner-facing lead session.
    pub fn is_owner_lead_session(key: &SessionKey, bot_name: Option<&str>) -> bool {
        is_owner_lead_session(key, bot_name)
    }

    /// Loads context for a session, determining active model and profile attributes.
    pub async fn load_context(
        pool: &SqlitePool,
        key: SessionKey,
        profile_router: Option<&ProfileRouter>,
    ) -> Result<SessionContext> {
        load_context(pool, key, profile_router).await
    }

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
                    referenced_message_id: None,
                    author_id: None,
                    origin: crate::EventOrigin::User,
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
                    // Keep the marker so the next boot retries instead of losing the turn.
                    if let Err(mark_error) =
                        crate::storage::mark_session_resume_pending(pool, &storage_key).await
                    {
                        tracing::error!(
                            session = %session_key,
                            error = %mark_error,
                            "failed to restore resume_pending marker after routing failure"
                        );
                    }
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
        inbox_event_tx: Option<crate::inbox::watchers::InboxEventSender>,
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
            inbox_event_tx,
            profile_router,
            completed_inbound_ids: VecDeque::new(),
        })
    }

    #[cfg(test)]
    pub(crate) async fn run(self) {
        if let Err(error) = self.run_with_shutdown(CancellationToken::new()).await {
            tracing::error!(%error, "session actor shutdown failed");
        }
    }

    pub(crate) async fn run_with_shutdown(
        mut self,
        service_shutdown: CancellationToken,
    ) -> Result<()> {
        let mut pending_events: VecDeque<ActorCommand> = VecDeque::new();
        loop {
            if service_shutdown.is_cancelled() {
                break;
            }
            let command = if let Some(cmd) = pending_events.pop_front() {
                cmd
            } else {
                match tokio::select! {
                    biased;
                    _ = service_shutdown.cancelled() => None,
                    command = self.receiver.recv() => command,
                } {
                    Some(command) => command,
                    None => break,
                }
            };

            // An acked event is handled exactly like a plain event; the ack rides along on the
            // actor so every terminal outcome below can report the turn result to the sender.
            let command = match command {
                ActorCommand::EventIfIdle { event, reply } => {
                    if !pending_events.is_empty() {
                        let _ =
                            reply.send(Err(OmonError::Multiplexer("lead session is busy".into())));
                        continue;
                    }
                    let _ = reply.send(Ok(()));
                    ActorCommand::Event(event)
                }
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
                        let storage_key = self.context.key.storage_key();
                        match self
                            .inbound_turn_completed(&storage_key, &event.platform_message_id)
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
                            Ok(false) => {
                                // A stale row from a failed or interrupted attempt must not
                                // survive next to the fresh attempt's transcript row.
                                if let Err(error) = discard_unfinished_inbound(
                                    &self.pool,
                                    &storage_key,
                                    &event.platform_message_id,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        session = %self.context.key,
                                        platform_message_id = %event.platform_message_id,
                                        %error,
                                        "failed to discard stale unfinished inbound row"
                                    );
                                }
                            }
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
                    let platform_message_id = if event.origin == crate::EventOrigin::System {
                        String::new()
                    } else {
                        reply_to.clone()
                    };
                    let delivery_id = event.delivery_id.clone();
                    let cancellation = CancellationToken::new();
                    let mut turn_context = self.context.clone();
                    let model_at_turn_start = self.context.state.active_model.clone();
                    let yolo_at_turn_start = self.context.state.yolo;

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

                    let mut status_driver = match &self.dispatcher {
                        Some(dispatcher)
                            if crate::report::status::status_message_enabled()
                                && event.origin == crate::EventOrigin::User
                                && !turn_context.key.user_id.starts_with("cron:")
                                && is_owner_lead_session(&turn_context.key, None) =>
                        {
                            Some(
                                crate::report::status::StatusDriver::start(
                                    self.pool.clone(),
                                    dispatcher.clone(),
                                    crate::report::status::TurnStart {
                                        session: turn_context.key.clone(),
                                        event_id: event_id.to_string(),
                                        owner_text: event.content.clone(),
                                        reply_to: Some(reply_to.clone()),
                                        owner_message_at: Some(event.received_at),
                                        started_at: Utc::now(),
                                    },
                                )
                                .await,
                            )
                        }
                        _ => None,
                    };

                    // One "queued" notice per running turn; a fresh turn starts with it unsent.
                    let mut queued_notice_sent = false;
                    let runner = self.runner.clone();
                    let mut run = Box::pin(runner.run_cancelable(
                        &mut turn_context,
                        *event,
                        cancellation.clone(),
                    ));
                    let outcome = loop {
                        let status_due = status_driver.as_ref().map(|driver| {
                            tokio::time::Instant::now()
                                + (driver.next_due() - Utc::now())
                                    .to_std()
                                    .unwrap_or(Duration::ZERO)
                        });
                        let status_sleep = async move {
                            match status_due {
                                Some(due) => tokio::time::sleep_until(due).await,
                                None => std::future::pending::<()>().await,
                            }
                        };
                        tokio::select! {
                            biased;
                            _ = service_shutdown.cancelled() => {
                                cancellation.cancel();
                                match tokio::time::timeout(std::time::Duration::from_secs(20), &mut run).await {
                                    Ok(result) => {
                                        if let Err(error) = result {
                                            tracing::debug!(session = %self.context.key, %error, "active turn canceled for service shutdown");
                                        }
                                    }
                                    Err(error) => tracing::error!(session = %self.context.key, %error, "backend cancellation did not finish before shutdown deadline"),
                                }
                                break TurnOutcome::Shutdown;
                            }
                            command = self.receiver.recv() => {
                                match command {
                                    Some(ActorCommand::EventIfIdle { reply, .. }) => {
                                        let _ = reply.send(Err(OmonError::Multiplexer("lead session is busy".into())));
                                    }
                                    Some(ActorCommand::IsIdle { reply }) => {
                                        let _ = reply.send(false);
                                    }
                                    Some(command @ ActorCommand::WaitIdle { .. }) => {
                                        pending_events.push_back(command);
                                    }
                                    Some(ActorCommand::Event(next)) => {
                                        // A short reply answers a pending approval only when the
                                        // platform author is the owner; run it off the loop.
                                        let owner = crate::context::owner_author_for(&self.context.key);
                                        if next.origin == crate::EventOrigin::User
                                            && owner.is_some()
                                            && next.author_id.as_deref() == owner.as_deref()
                                            && is_owner_lead_session(&self.context.key, None)
                                            && crate::autonomy::parse_short_owner_reply(
                                                &next.content,
                                            )
                                            .is_some()
                                        {
                                            let resolved = tokio::time::timeout(
                                                Duration::from_secs(5),
                                                self.runner.resolve_owner_reply(
                                                    &self.context.key,
                                                    &next.content,
                                                ),
                                            )
                                            .await;
                                            if let Ok(Ok(true)) = resolved {
                                                self.complete_delivery(
                                                    next.delivery_id.as_deref(),
                                                    &Ok(()),
                                                )
                                                .await;
                                                continue;
                                            }
                                        }
                                        if pending_events.len() < MAX_PENDING_EVENTS {
                                            let notify_reply_to = (!queued_notice_sent
                                                && next.origin == crate::EventOrigin::User
                                                && owner.is_some()
                                                && next.author_id.as_deref() == owner.as_deref()
                                                && !next.platform_message_id.is_empty()
                                                && !self.context.key.user_id.starts_with("cron:"))
                                            .then(|| next.platform_message_id.clone());
                                            pending_events.push_back(ActorCommand::Event(next));
                                            if let (Some(reply_to), Some(dispatcher)) =
                                                (notify_reply_to, &self.dispatcher)
                                            {
                                                queued_notice_sent = true;
                                                let _ = dispatcher
                                                    .dispatch(OutboundAction::SendMessage {
                                                        session: self.context.key.clone(),
                                                        content: QUEUED_NOTICE_TEXT.to_string(),
                                                        reply_to: Some(reply_to),
                                                    })
                                                    .await;
                                            }
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
                                    Some(ActorCommand::SetYolo { yolo, reply }) => {
                                        self.context.state.yolo = yolo;
                                        self.dirty = true;
                                        let _ = reply.send(Ok(()));
                                    }
                                    Some(ActorCommand::Reset { reply }) => {
                                        cancellation.cancel();
                                        self.binding_cleared = true;
                                        break TurnOutcome::Reset(reply);
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
                            _ = status_sleep => {
                                if let Some(driver) = status_driver.as_mut() {
                                    driver.tick(Utc::now()).await;
                                }
                            }
                        }
                    };
                    drop(run);

                    // Tool activity is per turn: clear it even when no status driver ran (status
                    // messages disabled, a non-owner session, or a system-origin turn), so a
                    // stale tool name can never be shown by a later turn's status edit.
                    crate::report::status::clear_tool_activity(&turn_context.key);

                    // The status message shows only the owner-facing reason, never the raw error
                    // (which the reply next to it promises stays in the logs).
                    if let Some(driver) = status_driver.take() {
                        match &outcome {
                            TurnOutcome::Completed(Ok(())) => {
                                driver.finish(Utc::now(), Ok(())).await
                            }
                            TurnOutcome::Completed(Err(error)) => {
                                driver
                                    .finish(
                                        Utc::now(),
                                        Err(owner_failure_reason(&error.to_string()).to_string()),
                                    )
                                    .await
                            }
                            TurnOutcome::Stopped(_) => {
                                driver
                                    .finish(Utc::now(), Err("stopped by user".into()))
                                    .await
                            }
                            TurnOutcome::Reset(_) => {
                                driver.finish(Utc::now(), Err("session reset".into())).await
                            }
                            // The restarted turn re-adopts the stored message by event id.
                            TurnOutcome::Shutdown => driver.abandon(),
                        }
                    }

                    // Durable remote binding must never be lost on failure or overwritten by a stale actor copy,
                    // unless a mid-turn reset deliberately cleared it.
                    if !self.binding_cleared {
                        if let Some(thread_id) = turn_context.state.metadata.get("omo_thread_id") {
                            self.context
                                .state
                                .metadata
                                .insert("omo_thread_id".into(), thread_id.clone());
                        }
                    }

                    match outcome {
                        TurnOutcome::Completed(mut result) => {
                            if result.is_ok() {
                                // A mid-turn /model switch must survive the turn-start snapshot.
                                let mid_turn_model = (self.context.state.active_model
                                    != model_at_turn_start)
                                    .then(|| self.context.state.active_model.take());
                                // Likewise a mid-turn /yolo toggle.
                                let mid_turn_yolo = (self.context.state.yolo != yolo_at_turn_start)
                                    .then_some(self.context.state.yolo);
                                self.context = turn_context;
                                if let Some(model) = mid_turn_model {
                                    self.context.state.active_model = model;
                                }
                                if let Some(yolo) = mid_turn_yolo {
                                    self.context.state.yolo = yolo;
                                }
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
                            if result.is_ok() && !platform_message_id.is_empty() {
                                self.remember_completed_inbound(&platform_message_id);
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
                                            content: owner_failure_text(&error.to_string()),
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
                        TurnOutcome::Reset(reply) => {
                            let _ = self
                                .interrupt_turn(event_id, delivery_id.as_deref(), "session reset")
                                .await;
                            self.release_typing().await;
                            self.resolve_pending_ack(Err(OmonError::Multiplexer(
                                "session reset".into(),
                            )));
                            self.reset_state().await;
                            let _ = reply.send(self.flush_if_dirty().await);
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
                            // A mid-turn reset intentionally abandons the turn; do not resume it.
                            if !self.binding_cleared {
                                if let Err(error) = crate::storage::mark_session_resume_pending(
                                    &self.pool,
                                    &self.context.key.storage_key(),
                                )
                                .await
                                {
                                    tracing::error!(session = %self.context.key, %error, "failed to mark resume_pending on shutdown");
                                }
                            }
                            self.resolve_pending_ack(Err(OmonError::Multiplexer(
                                "session actor shutting down".into(),
                            )));
                            break;
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
                ActorCommand::SetYolo { yolo, reply } => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.context.state.yolo = yolo;
                    self.dirty = true;
                    let flush_res = self.flush_if_dirty().await;
                    let _ = reply.send(flush_res);
                }
                ActorCommand::Reset { reply } => {
                    self.last_active_at = tokio::time::Instant::now();
                    self.reset_state().await;
                    let flush_res = self.flush_if_dirty().await;
                    let _ = reply.send(flush_res);
                }
                ActorCommand::GetContext { reply } => {
                    let _ = reply.send(self.context.clone());
                }
                ActorCommand::IsIdle { reply } => {
                    let _ = reply.send(pending_events.is_empty() && self.receiver.is_empty());
                }
                ActorCommand::WaitIdle { reply } => {
                    let _ = reply.send(());
                }
                ActorCommand::EventIfIdle { reply, .. } => {
                    let _ = reply.send(Err(OmonError::Multiplexer(
                        "idle event was not dispatched".into(),
                    )));
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
        self.receiver.close();
        while let Ok(command) = self.receiver.try_recv() {
            pending_events.push_back(command);
        }
        // Only queued turns need a resume; a stray activity/idle probe does not.
        let has_pending_turn = pending_events.iter().any(|command| {
            matches!(
                command,
                ActorCommand::Event(_)
                    | ActorCommand::EventWithAck { .. }
                    | ActorCommand::EventIfIdle { .. }
            )
        });
        if has_pending_turn {
            if let Err(error) = crate::storage::mark_session_resume_pending(
                &self.pool,
                &self.context.key.storage_key(),
            )
            .await
            {
                tracing::error!(session = %self.context.key, %error, "failed to mark resume_pending for remaining pending events");
            }
        }
        for command in pending_events {
            match command {
                ActorCommand::Event(event) => {
                    self.complete_delivery(
                        event.delivery_id.as_deref(),
                        &Err(OmonError::Multiplexer("session actor shutting down".into())),
                    )
                    .await;
                    rewind_backfill_cursor(&self.pool, &event).await;
                }
                ActorCommand::EventWithAck { ack, .. } => {
                    let _ = ack.send(Err(OmonError::Multiplexer(
                        "session actor shutting down".into(),
                    )));
                }
                ActorCommand::EventIfIdle { reply, .. } => {
                    let _ = reply.send(Err(OmonError::Multiplexer(
                        "session actor shutting down".into(),
                    )));
                }
                _ => {}
            }
        }
        if let Err(error) = self.flush_if_dirty().await {
            tracing::error!(session = %self.context.key, %error, "failed to flush session actor during shutdown");
            let _ = crate::storage::mark_session_resume_pending(
                &self.pool,
                &self.context.key.storage_key(),
            )
            .await;
            return Err(error);
        }
        Ok(())
    }

    /// Clears the session to the state a brand-new session would get, including the
    /// profile-derived defaults (toolset restrictions, system prompt, model), and drops the
    /// remote conversation binding.
    async fn reset_state(&mut self) {
        let fresh = match fresh_context(
            &self.pool,
            self.context.key.clone(),
            self.profile_router.as_deref(),
        )
        .await
        {
            Ok(context) => context.state,
            Err(error) => {
                tracing::error!(session = %self.context.key, %error, "failed to rebuild profile defaults on reset");
                crate::SessionState::default()
            }
        };
        self.context.state = fresh;
        self.context.state.metadata.remove("omo_thread_id");
        self.binding_cleared = true;
        self.dirty = true;
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

    /// Whether the turn for an already persisted inbound `platform_message_id` finished, so a
    /// redelivery can be skipped. A bare transcript row is not proof: `persist_inbound` writes it
    /// before the turn runs, so a failed or interrupted turn leaves the same row behind.
    async fn inbound_turn_completed(
        &self,
        storage_key: &str,
        platform_message_id: &str,
    ) -> Result<bool> {
        if !crate::storage::has_platform_message_id(&self.pool, storage_key, platform_message_id)
            .await?
        {
            return Ok(false);
        }
        if self
            .completed_inbound_ids
            .iter()
            .any(|id| id == platform_message_id)
        {
            return Ok(true);
        }
        // The delivery ledger is authoritative when it knows this message: a failed or reclaimed
        // (in progress) claim means the earlier attempt did not finish, even if it left an
        // assistant row behind. It also survives actor eviction and covers silent turns.
        let ledger_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM delivery_ledger
             WHERE session_key = ? AND platform_message_id = ?
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(storage_key)
        .bind(platform_message_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(status) = ledger_status {
            return Ok(status == "delivered");
        }
        // Answered = an assistant row follows the user row before any later user row.
        let answered: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                 SELECT 1 FROM messages u
                 WHERE u.session_key = ? AND u.platform_message_id = ? AND u.role = 'user'
                   AND EXISTS (
                       SELECT 1 FROM messages a
                       WHERE a.session_key = u.session_key AND a.role = 'assistant'
                         AND a.sequence > u.sequence
                         AND NOT EXISTS (
                             SELECT 1 FROM messages n
                             WHERE n.session_key = u.session_key AND n.role = 'user'
                               AND n.sequence > u.sequence AND n.sequence < a.sequence
                         )
                   )
             )",
        )
        .bind(storage_key)
        .bind(platform_message_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(answered)
    }

    fn remember_completed_inbound(&mut self, platform_message_id: &str) {
        if self
            .completed_inbound_ids
            .iter()
            .any(|id| id == platform_message_id)
        {
            return;
        }
        if self.completed_inbound_ids.len() >= MAX_REMEMBERED_COMPLETED_INBOUND {
            self.completed_inbound_ids.pop_front();
        }
        self.completed_inbound_ids
            .push_back(platform_message_id.to_string());
    }

    async fn rollback_partial_history(&self, event_id: uuid::Uuid) -> Result<()> {
        // Only assistant rows are partial output of the interrupted turn. User rows after the
        // turn's own row are steered messages that the user really sent and must be kept.
        sqlx::query(
            "DELETE FROM messages
             WHERE session_key = ? AND role = 'assistant'
               AND COALESCE(json_extract(metadata_json, '$.mirror'), 0) <> 1
               AND sequence > (
                 SELECT sequence FROM messages WHERE id = ? AND session_key = ?
             ) AND NOT EXISTS (
                 SELECT 1 FROM delivery_obligations
                 WHERE id = json_extract(messages.metadata_json, '$.delivery_obligation_id')
                   AND state IN ('attempting', 'delivered')
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
        let inserted = sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at, platform_message_id)
             VALUES (?, ?, 'user', ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(event.id.to_string())
        .bind(self.context.key.storage_key())
        .bind(inbound_transcript_content(event))
        .bind(serde_json::to_string(&event.attachments).map_err(serialization_error)?)
        .bind(event.received_at)
        .bind(if event.platform_message_id.is_empty() {
            None
        } else {
            Some(&event.platform_message_id)
        })
        .execute(&self.pool)
        .await?;
        // An owner message in the lead's home DM is what the kibitzer exists to react to.
        // A replayed event (row already present) is not a new owner message.
        if inserted.rows_affected() == 1
            && event.origin == crate::EventOrigin::User
            && self.is_kibitzer_lead()
        {
            if let Err(error) = self.arm_kibitzer().await {
                tracing::warn!(session = %self.context.key, %error, "failed to arm kibitzer for owner message");
            }
        }
        if event.origin == crate::EventOrigin::User
            && (event.session.platform == "discord"
                || event.session.platform == crate::slack::PLATFORM)
            && !event.platform_message_id.is_empty()
        {
            let index = crate::MessageSearchIndex::new(self.pool.clone());
            index.index_inbound(&event.session, event).await?;
            if let Some(sender) = &self.inbox_event_tx {
                if let Err(error) = crate::inbox::watchers::on_gateway_message(sender).await {
                    tracing::warn!(%error, "Discord inbox event receiver is closed after message index commit");
                }
            }
        }
        Ok(())
    }

    fn is_kibitzer_lead(&self) -> bool {
        let lead = crate::runtime_config::default_lead();
        SessionKey::from_storage_key(lead.trim())
            .is_ok_and(|lead| lead.storage_key() == self.context.key.storage_key())
    }

    async fn arm_kibitzer(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        crate::lanes::kibitzer::arm_owner(&mut tx, &self.context.key).await?;
        tx.commit().await?;
        crate::lanes::runner::wake(&self.pool).await
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

/// Removes the user row left by a failed or interrupted attempt at the same platform message so
/// the redelivered attempt does not leave a duplicate row in the transcript.
async fn discard_unfinished_inbound(
    pool: &SqlitePool,
    storage_key: &str,
    platform_message_id: &str,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM messages WHERE session_key = ? AND platform_message_id = ? AND role = 'user'",
    )
    .bind(storage_key)
    .bind(platform_message_id)
    .execute(pool)
    .await?;
    Ok(())
}

const QUEUED_NOTICE_TEXT: &str = "앞 작업이 끝나면 바로 이어서 할게요.";

/// Owner-facing Korean line for a failed turn; the raw error stays in the logs.
fn owner_failure_text(error: &str) -> String {
    format!(
        "⚠️ 응답을 만들지 못했어요 ({}). 같은 메시지를 다시 보내 주시면 이어서 할게요.",
        owner_failure_reason(error)
    )
}

/// Short owner-facing reason for a failed turn. Shared with the status message, which must not
/// leak the raw error text that `owner_failure_text` deliberately hides.
fn owner_failure_reason(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if [
        "no output within",
        "turn interrupted",
        "timeout waiting for turn/start",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        "모델이 응답하지 않았어요"
    } else if ["quota", "429", "rate limit"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        "모델 사용량 한도에 걸렸어요"
    } else {
        "처리 중 오류가 났어요"
    }
}

fn is_owner_lead_session(key: &SessionKey, bot_name: Option<&str>) -> bool {
    if key.platform == crate::slack::PLATFORM {
        return is_slack_owner_lead_session(
            key,
            &crate::slack::owner_id(),
            &crate::slack::home_channels(),
        );
    }
    if key.platform != "discord" {
        return false;
    }

    let is_omomeow = match key.bot_id.as_deref() {
        None => true,
        Some(id) => {
            if id == "1465631383862120451"
                || id.eq_ignore_ascii_case("omomeow")
                || id.eq_ignore_ascii_case("wawabot")
            {
                true
            } else if let Ok(configured) = std::env::var("OMON_OMOMEOW_BOT_ID") {
                id == configured.trim()
            } else if let Some(name) = bot_name {
                name.eq_ignore_ascii_case("omomeow") || name.eq_ignore_ascii_case("wawabot")
            } else {
                false
            }
        }
    };

    if !is_omomeow {
        return false;
    }

    let owner_id = std::env::var("DISCORD_ALLOWED_USERS")
        .ok()
        .and_then(|val| {
            val.split(',')
                .map(str::trim)
                .find(|s| !s.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "414011632306618368".to_string());

    let is_owner_dm = key.guild_id.is_none() && key.user_id == owner_id;
    if is_owner_dm {
        return true;
    }

    if let Ok(home_val) = std::env::var("DISCORD_HOME_CHANNEL") {
        let home_channels: Vec<&str> = home_val
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if home_channels.iter().any(|&h| h == key.channel_id) {
            return true;
        }
        if let Some(tid) = key.thread_id.as_deref() {
            if home_channels.contains(&tid) {
                return true;
            }
        }
    }
    if let Ok(thread_val) = std::env::var("DISCORD_HOME_CHANNEL_THREAD_ID") {
        let home_threads: Vec<&str> = thread_val
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(tid) = key.thread_id.as_deref() {
            if home_threads.contains(&tid) {
                return true;
            }
        }
        if home_threads.iter().any(|&h| h == key.channel_id) {
            return true;
        }
    }

    false
}

/// The Slack owner DM, or any conversation in a `SLACK_HOME_CHANNEL` (threads included).
fn is_slack_owner_lead_session(key: &SessionKey, owner: &str, home_channels: &[String]) -> bool {
    if key.guild_id.is_none() && key.user_id == owner {
        return true;
    }
    home_channels.iter().any(|home| home == &key.channel_id)
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
            let mut bot_profile_model: Option<String> = None;
            let mut bot_name: Option<String> = None;
            if let Some(bot_id) = key.bot_id.as_deref() {
                if let Ok(Some((name, model, prompt, toolsets))) = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
                    "SELECT name, model, system_prompt, enabled_toolsets FROM bot_profiles WHERE bot_id = ?"
                )
                .bind(bot_id)
                .fetch_optional(pool)
                .await {
                    bot_name = name;
                    bot_profile_model = model;
                    if state.system_prompt.is_none() && prompt.is_some() {
                        state.system_prompt = prompt;
                    }
                    if state.enabled_toolsets.is_none() && toolsets.is_some() {
                        state.enabled_toolsets = toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
                    }
                }
            }

            if state.active_model.is_none() {
                if is_owner_lead_session(&key, bot_name.as_deref()) {
                    state.active_model = Some(crate::agent::model_for_purpose(
                        crate::agent::ModelPurpose::Lead,
                    ));
                } else if let Some(model) = bot_profile_model {
                    state.active_model = Some(model);
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
        None => fresh_context(pool, key, profile_router).await,
    }
}

/// Builds the context a session gets when it has no stored state: the bot profile and
/// profile-router defaults applied over an empty state.
async fn fresh_context(
    pool: &SqlitePool,
    key: SessionKey,
    profile_router: Option<&ProfileRouter>,
) -> Result<SessionContext> {
    let mut context = SessionContext::new(key);
    let mut bot_profile_model: Option<String> = None;
    let mut bot_name: Option<String> = None;
    if let Some(bot_id) = context.key.bot_id.as_deref() {
        if let Ok(Some((name, model, prompt, toolsets))) = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT name, model, system_prompt, enabled_toolsets FROM bot_profiles WHERE bot_id = ?"
        )
        .bind(bot_id)
        .fetch_optional(pool)
        .await {
            bot_name = name;
            bot_profile_model = model;
            if prompt.is_some() {
                context.state.system_prompt = prompt;
            }
            if toolsets.is_some() {
                context.state.enabled_toolsets = toolsets.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
            }
        }
    }

    if context.state.active_model.is_none() {
        if is_owner_lead_session(&context.key, bot_name.as_deref()) {
            context.state.active_model = Some(crate::agent::model_for_purpose(
                crate::agent::ModelPurpose::Lead,
            ));
        } else if let Some(model) = bot_profile_model {
            context.state.active_model = Some(model);
        }
    }

    if let Some(router) = profile_router {
        router.apply_to_session(&mut context);
    }
    Ok(context)
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
    .bind(inbound_transcript_content(event))
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

fn inbound_transcript_content(event: &InboundEvent) -> String {
    let content = strip_leading_message_timestamps(&render_user_prompt(event));
    match event.origin {
        crate::EventOrigin::User => content,
        crate::EventOrigin::System => format!("[system] {content}"),
    }
}

fn serialization_error(error: serde_json::Error) -> OmonError {
    OmonError::Multiplexer(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use tokio::sync::Barrier;

    #[test]
    fn slack_owner_lead_covers_owner_dm_and_home_channel() {
        let home = vec!["CHOME".to_owned()];
        let dm = SessionKey::new("slack", None::<String>, "DOWNER", None::<String>, "UOWNER");
        assert!(is_slack_owner_lead_session(&dm, "UOWNER", &home));
        let other_dm = SessionKey::new("slack", None::<String>, "DX", None::<String>, "UX");
        assert!(!is_slack_owner_lead_session(&other_dm, "UOWNER", &home));
        let home_thread = SessionKey::new("slack", Some("T1"), "CHOME", Some("1.000001"), "UOWNER");
        assert!(is_slack_owner_lead_session(&home_thread, "UOWNER", &home));
        let elsewhere = SessionKey::new("slack", Some("T1"), "COTHER", None::<String>, "UOWNER");
        assert!(!is_slack_owner_lead_session(&elsewhere, "UOWNER", &home));
        assert!(!is_slack_owner_lead_session(&home_thread, "UOWNER", &[]));
    }

    #[test]
    fn status_failure_reason_never_leaks_the_raw_error() {
        let raw = "LLM error: websocket closed: quota exhausted for account sk-live-abc";
        let reason = owner_failure_reason(raw);
        assert_eq!(reason, "모델 사용량 한도에 걸렸어요");
        assert!(!reason.contains("sk-live-abc"));
        // The status line the owner sees carries the reason only.
        let status = format!("실패 · {reason}");
        assert!(!status.contains("websocket closed"), "{status}");
        assert_eq!(
            owner_failure_reason("database is locked"),
            "처리 중 오류가 났어요"
        );
        assert_eq!(
            owner_failure_reason("agent produced no output within 45s"),
            "모델이 응답하지 않았어요"
        );
    }

    #[test]
    fn owner_failure_text_maps_known_causes_to_korean_reasons() {
        let no_response = "⚠️ 응답을 만들지 못했어요 (모델이 응답하지 않았어요). 같은 메시지를 다시 보내 주시면 이어서 할게요.";
        for error in [
            "LLM error: turn interrupted: agent produced no output within 45s of starting the prompt",
            "agent produced no output within 45s",
            "Turn Interrupted",
            "timeout waiting for turn/start",
        ] {
            assert_eq!(owner_failure_text(error), no_response, "{error}");
        }
        let quota = "⚠️ 응답을 만들지 못했어요 (모델 사용량 한도에 걸렸어요). 같은 메시지를 다시 보내 주시면 이어서 할게요.";
        for error in [
            "quota exhausted",
            "HTTP 429 Too Many Requests",
            "Rate limit reached",
        ] {
            assert_eq!(owner_failure_text(error), quota, "{error}");
        }
        assert_eq!(
            owner_failure_text("database is locked"),
            "⚠️ 응답을 만들지 못했어요 (처리 중 오류가 났어요). 같은 메시지를 다시 보내 주시면 이어서 할게요."
        );
    }

    #[test]
    fn slack_ts_predecessor_is_fixed_width() {
        assert_eq!(
            parse_slack_ts("1700000000.000100"),
            Some((1_700_000_000, 100))
        );
        assert_eq!(parse_slack_ts("1700000000.1"), None);
        assert_eq!(parse_slack_ts("abc"), None);
        assert_eq!(
            slack_ts_before((1_700_000_000, 100)).as_deref(),
            Some("1700000000.000099")
        );
        assert_eq!(
            slack_ts_before((1_700_000_000, 0)).as_deref(),
            Some("1699999999.999999")
        );
        assert_eq!(slack_ts_before((0, 0)), None);
    }

    #[tokio::test]
    async fn dropped_slack_message_rewinds_its_cursor_to_the_earliest_constituent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        sqlx::query(
            "INSERT INTO slack_cursors (bot_id, channel_id, thread_ts, last_ts, updated_at) VALUES ('UBOT', 'C1', '', '1700000000.000300', CURRENT_TIMESTAMP), ('UBOT', 'C2', '', '1700000000.000300', CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO delivery_ledger_constituents (parent_delivery_id, constituent_id) VALUES ('slack:C1:1700000000.000300', 'slack:C1:1700000000.000200')")
            .execute(&pool)
            .await
            .unwrap();
        let session = SessionKey::new("slack", Some("T1"), "C1", None::<String>, "UOWNER")
            .with_bot_id("UBOT");
        let event = InboundEvent::message(session, "1700000000.000300", "hi")
            .with_delivery_id("slack:C1:1700000000.000300");
        rewind_backfill_cursor(&pool, &event).await;
        let cursors: Vec<(String, String)> =
            sqlx::query_as("SELECT channel_id, last_ts FROM slack_cursors ORDER BY channel_id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            cursors,
            [
                ("C1".to_owned(), "1700000000.000199".to_owned()),
                ("C2".to_owned(), "1700000000.000300".to_owned()),
            ]
        );
    }

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
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            runner,
            None,
            db.pool().clone(),
            None,
            None,
        )
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

    async fn spawn_blocking_actor(
        db: &Database,
        key: &SessionKey,
    ) -> (
        mpsc::Sender<ActorCommand>,
        mpsc::UnboundedReceiver<String>,
        Arc<Barrier>,
        tokio::task::JoinHandle<()>,
    ) {
        let (started_tx, started_rx) = mpsc::unbounded_channel();
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
            None,
            db.pool().clone(),
            None,
            None,
        )
        .await
        .unwrap();
        (cmd_tx, started_rx, barrier, tokio::spawn(actor.run()))
    }

    #[tokio::test]
    async fn mid_turn_set_model_survives_turn_completion() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-mid-turn-model");
        let (cmd_tx, mut started_rx, barrier, handle) = spawn_blocking_actor(&db, &key).await;

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
            .send(ActorCommand::SetModel {
                model: "mid-turn-model".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // WaitIdle sent mid-turn is queued and only answered after the turn finalizes.
        let (idle_tx, idle_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::WaitIdle { reply: idle_tx })
            .await
            .unwrap();
        barrier.wait().await;
        idle_rx.await.unwrap();
        let (ctx_tx, ctx_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::GetContext { reply: ctx_tx })
            .await
            .unwrap();
        let context = ctx_rx.await.unwrap();
        assert_eq!(
            context.state.active_model.as_deref(),
            Some("mid-turn-model"),
            "a /model switch during a turn must not be reverted when the turn completes"
        );

        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn mid_turn_set_yolo_survives_turn_completion() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-mid-turn-yolo");
        let (cmd_tx, mut started_rx, barrier, handle) = spawn_blocking_actor(&db, &key).await;

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
            .send(ActorCommand::SetYolo {
                yolo: true,
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        let (idle_tx, idle_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::WaitIdle { reply: idle_tx })
            .await
            .unwrap();
        barrier.wait().await;
        idle_rx.await.unwrap();
        let (ctx_tx, ctx_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::GetContext { reply: ctx_tx })
            .await
            .unwrap();
        assert!(
            ctx_rx.await.unwrap().state.yolo,
            "a /yolo toggle during a turn must not be reverted when the turn completes"
        );
        drop(cmd_tx);
        handle.await.unwrap();

        let persisted: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(key.storage_key())
                .fetch_one(db.pool())
                .await
                .unwrap();
        let state: SessionState = serde_json::from_str(&persisted).unwrap();
        assert!(state.yolo, "flush after the turn must persist yolo=true");
    }

    #[tokio::test]
    async fn mid_turn_reset_does_not_restore_thread_binding_or_mark_resume_pending() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-mid-turn-reset");
        let mut state = SessionState::default();
        state
            .metadata
            .insert("omo_thread_id".into(), serde_json::json!("old-thread"));
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json) VALUES (?, 'discord', 'guild', 'channel', 'actor-mid-turn-reset', ?)",
        )
        .bind(key.storage_key())
        .bind(serde_json::to_string(&state).unwrap())
        .execute(db.pool())
        .await
        .unwrap();

        let (cmd_tx, mut started_rx, _barrier, handle) = spawn_blocking_actor(&db, &key).await;
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
            .send(ActorCommand::Reset { reply: reply_tx })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();
        drop(cmd_tx);
        handle.await.unwrap();

        let (thread_id, resume_pending): (Option<String>, i64) = sqlx::query_as(
            "SELECT json_extract(state_json, '$.metadata.omo_thread_id'), resume_pending FROM sessions WHERE session_key = ?",
        )
        .bind(key.storage_key())
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(thread_id, None, "reset must not resurrect the old binding");
        assert_eq!(resume_pending, 0, "reset must not schedule a resume");
    }

    #[tokio::test]
    async fn shutdown_drain_fails_queued_plain_event_delivery() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-drain-plain-event");
        let ledger = DeliveryLedgerService::new(db.pool().clone());
        let (cmd_tx, mut started_rx, _barrier, handle) = spawn_blocking_actor(&db, &key).await;

        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-1",
                "blocking",
            ))))
            .await
            .unwrap();
        assert_eq!(started_rx.recv().await.as_deref(), Some("blocking"));

        let delivery_id = "delivery_queued_plain";
        let mut queued = InboundEvent::message(key.clone(), "msg-2", "queued");
        queued.delivery_id = Some(delivery_id.to_string());
        ledger
            .record_incoming_as(&queued, delivery_id)
            .await
            .unwrap();
        cmd_tx
            .send(ActorCommand::Event(Box::new(queued)))
            .await
            .unwrap();

        // Closing the command channel ends the actor while the plain event is still queued.
        drop(cmd_tx);
        handle.await.unwrap();

        let entry = ledger.get(delivery_id).await.unwrap().unwrap();
        assert_eq!(
            entry.status, "failed",
            "dropped queued event must be released as failed, not left in_progress"
        );
    }

    #[tokio::test]
    async fn idle_reset_keeps_profile_toolset_restriction() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let mut key = test_session("actor-reset-profile");
        key.bot_id = Some("bot-restricted".into());
        sqlx::query(
            "INSERT INTO bot_profiles (bot_id, name, system_prompt, enabled_toolsets) VALUES ('bot-restricted', 'restricted', 'profile prompt', 'web')",
        )
        .execute(db.pool())
        .await
        .unwrap();
        let (cmd_tx, _started_rx, _barrier, handle) = spawn_blocking_actor(&db, &key).await;

        let (reply_tx, reply_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::Reset { reply: reply_tx })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        let (ctx_tx, ctx_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::GetContext { reply: ctx_tx })
            .await
            .unwrap();
        let state = ctx_rx.await.unwrap().state;
        assert_eq!(
            state.enabled_toolsets,
            Some(vec!["web".to_string()]),
            "reset must keep the profile's toolset restriction"
        );
        assert_eq!(state.system_prompt.as_deref(), Some("profile prompt"));
        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn mid_turn_reset_keeps_actor_serving_the_session() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-mid-turn-reset-alive");
        let (started_tx, mut started_rx) = mpsc::unbounded_channel();
        let (completed_tx, mut completed_rx) = mpsc::unbounded_channel();
        let barrier = Arc::new(Barrier::new(2));
        let runner = Arc::new(TurnRecordingRunner {
            started: started_tx,
            barrier,
            completed: completed_tx,
        });
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            runner,
            None,
            db.pool().clone(),
            None,
            None,
        )
        .await
        .unwrap();
        let handle = tokio::spawn(actor.run());

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
            .send(ActorCommand::Reset { reply: reply_tx })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // A message sent right after the reset must still run on the same actor.
        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "msg-2",
                "after-reset",
            ))))
            .await
            .unwrap();
        assert_eq!(started_rx.recv().await.as_deref(), Some("after-reset"));
        assert_eq!(completed_rx.recv().await.as_deref(), Some("after-reset"));
        assert!(!handle.is_finished(), "actor must survive a mid-turn reset");
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
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            runner,
            None,
            db.pool().clone(),
            None,
            None,
        )
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
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            runner,
            None,
            db.pool().clone(),
            None,
            None,
        )
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
            None,
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
            None,
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

    struct FlakyRunner {
        runs: std::sync::atomic::AtomicUsize,
        fail_first: bool,
    }

    #[async_trait]
    impl AgentRunner for FlakyRunner {
        async fn run(&self, _session: &mut SessionContext, _event: InboundEvent) -> Result<()> {
            let previous = self.runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail_first && previous == 0 {
                return Err(OmonError::Llm("boom".into()));
            }
            Ok(())
        }
    }

    async fn run_event_until_idle(cmd_tx: &mpsc::Sender<ActorCommand>, event: InboundEvent) {
        cmd_tx
            .send(ActorCommand::Event(Box::new(event)))
            .await
            .unwrap();
        let (idle_tx, idle_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::WaitIdle { reply: idle_tx })
            .await
            .unwrap();
        idle_rx.await.unwrap();
    }

    async fn spawn_flaky_actor(
        db: &Database,
        key: &SessionKey,
        runner: Arc<FlakyRunner>,
    ) -> (mpsc::Sender<ActorCommand>, tokio::task::JoinHandle<()>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let actor = SessionActor::load(
            key.clone(),
            cmd_rx,
            runner,
            None,
            db.pool().clone(),
            None,
            None,
        )
        .await
        .unwrap();
        (cmd_tx, tokio::spawn(actor.run()))
    }

    #[tokio::test]
    async fn redelivery_of_a_failed_turn_runs_again_instead_of_being_skipped() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let key = test_session("actor-failed-redelivery");
        let runner = Arc::new(FlakyRunner {
            runs: std::sync::atomic::AtomicUsize::new(0),
            fail_first: true,
        });
        let (cmd_tx, handle) = spawn_flaky_actor(&db, &key, runner.clone()).await;

        let delivery_id = "claim-failed-redelivery";
        let ledger = DeliveryLedgerService::new(pool.clone());
        let mut first = InboundEvent::message(key.clone(), "plat-failed-1", "please run");
        first.delivery_id = Some(delivery_id.to_string());
        assert!(ledger
            .record_incoming_as(&first, delivery_id)
            .await
            .unwrap());
        run_event_until_idle(&cmd_tx, first).await;
        assert_eq!(runner.runs.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(
            ledger.get(delivery_id).await.unwrap().unwrap().status,
            "failed"
        );

        // The failed delivery is reclaimed and redelivered with the same platform message id.
        let mut second = InboundEvent::message(key.clone(), "plat-failed-1", "please run");
        second.delivery_id = Some(delivery_id.to_string());
        assert!(ledger
            .record_incoming_as(&second, delivery_id)
            .await
            .unwrap());
        run_event_until_idle(&cmd_tx, second).await;

        assert_eq!(
            runner.runs.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "a failed turn must not be skipped as an already processed replay"
        );
        assert_eq!(
            ledger.get(delivery_id).await.unwrap().unwrap().status,
            "delivered"
        );
        let user_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE session_key = ? AND role = 'user' AND platform_message_id = 'plat-failed-1'",
        )
        .bind(key.storage_key())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            user_rows, 1,
            "the stale row of the failed attempt must be replaced"
        );

        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn redelivery_of_a_completed_turn_is_still_skipped() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-completed-redelivery");
        let runner = Arc::new(FlakyRunner {
            runs: std::sync::atomic::AtomicUsize::new(0),
            fail_first: false,
        });
        let (cmd_tx, handle) = spawn_flaky_actor(&db, &key, runner.clone()).await;

        run_event_until_idle(
            &cmd_tx,
            InboundEvent::message(key.clone(), "plat-done-1", "once"),
        )
        .await;
        run_event_until_idle(
            &cmd_tx,
            InboundEvent::message(key.clone(), "plat-done-1", "once"),
        )
        .await;
        assert_eq!(runner.runs.load(std::sync::atomic::Ordering::SeqCst), 1);

        drop(cmd_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn interrupt_rollback_keeps_steered_user_rows() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let key = test_session("actor-rollback-steered");
        let (cmd_tx, mut started_rx, _barrier, handle) = spawn_blocking_actor(&db, &key).await;

        cmd_tx
            .send(ActorCommand::Event(Box::new(InboundEvent::message(
                key.clone(),
                "plat-turn-1",
                "blocking",
            ))))
            .await
            .unwrap();
        assert_eq!(started_rx.recv().await.as_deref(), Some("blocking"));

        // A steered message lands in the transcript mid-turn, followed by partial assistant output.
        record_inbound_transcript(
            db.pool(),
            &InboundEvent::message(key.clone(), "plat-steer-1", "steered guidance"),
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) VALUES (?, ?, 'assistant', 'partial', '{}', ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(key.storage_key())
        .bind(Utc::now())
        .execute(db.pool())
        .await
        .unwrap();

        let (reply_tx, reply_rx) = oneshot::channel();
        cmd_tx
            .send(ActorCommand::Stop { reply: reply_tx })
            .await
            .unwrap();
        assert!(reply_rx.await.unwrap().unwrap());
        drop(cmd_tx);
        handle.await.unwrap();

        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT role, platform_message_id FROM messages WHERE session_key = ? ORDER BY sequence",
        )
        .bind(key.storage_key())
        .fetch_all(db.pool())
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![
                ("user".to_string(), Some("plat-turn-1".to_string())),
                ("user".to_string(), Some("plat-steer-1".to_string())),
            ],
            "rollback must drop partial assistant output but keep the steered user message"
        );
    }

    #[tokio::test]
    async fn failed_recovery_route_keeps_resume_pending_marker() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let pool = db.pool().clone();
        let key = test_session("actor-recovery-marker");
        record_inbound_transcript(
            &pool,
            &InboundEvent::message(key.clone(), "plat-recover-1", "resume me"),
        )
        .await
        .unwrap();
        crate::storage::mark_session_resume_pending(&pool, &key.storage_key())
            .await
            .unwrap();

        // A draining multiplexer refuses the re-dispatch, so recovery cannot make progress.
        let (_drain_tx, drain_rx) = tokio::sync::watch::channel(true);
        let multiplexer = SessionMultiplexer::new(
            pool.clone(),
            Arc::new(FlakyRunner {
                runs: std::sync::atomic::AtomicUsize::new(0),
                fail_first: false,
            }),
            crate::multiplexer::MultiplexerConfig::default(),
        )
        .with_drain_receiver(drain_rx);

        let recovered = SessionActor::recover_resume_pending_sessions(&pool, &multiplexer)
            .await
            .unwrap();
        assert_eq!(recovered, 0);
        assert_eq!(
            crate::storage::count_resume_pending_sessions(&pool)
                .await
                .unwrap(),
            1,
            "the marker must survive a failed re-dispatch so the next boot retries"
        );
    }
}

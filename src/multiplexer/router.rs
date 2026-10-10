use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::{mpsc, watch, Notify};

use crate::{InboundEvent, OmonError, ProfileRouter, Result, SessionContext, SessionKey};

use super::actor::{ActorCommand, AgentRunner, OutboundDispatcher, SessionActor};

const SESSION_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Debug)]
pub struct MultiplexerConfig {
    pub idle_timeout: Duration,
    pub gc_interval: Duration,
}

impl Default for MultiplexerConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(10 * 60),
            gc_interval: Duration::from_secs(60),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ActorStartup {
    Starting,
    Ready,
    Failed(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SendOutcome {
    Sent,
    Retiring,
    Closed,
}

pub(crate) struct SessionHandle {
    pub(crate) sender: mpsc::Sender<ActorCommand>,
    accepting: AtomicBool,
    finished: AtomicBool,
    in_flight: AtomicUsize,
    in_flight_drained: Notify,
    state_changed: Notify,
    startup: watch::Receiver<ActorStartup>,
}

impl SessionHandle {
    fn new(sender: mpsc::Sender<ActorCommand>, startup: watch::Receiver<ActorStartup>) -> Self {
        Self {
            sender,
            accepting: AtomicBool::new(true),
            finished: AtomicBool::new(false),
            in_flight: AtomicUsize::new(0),
            in_flight_drained: Notify::new(),
            state_changed: Notify::new(),
            startup,
        }
    }

    async fn wait_started(&self) -> Result<()> {
        let mut startup = self.startup.clone();
        loop {
            let state = startup.borrow().clone();
            match state {
                ActorStartup::Starting => {}
                ActorStartup::Ready => return Ok(()),
                ActorStartup::Failed(error) => {
                    return Err(OmonError::Multiplexer(format!(
                        "session actor failed to start: {error}"
                    )))
                }
            }
            startup.changed().await.map_err(|_| {
                OmonError::Multiplexer("session actor stopped during startup".into())
            })?;
        }
    }

    pub(crate) async fn send_event(&self, event: InboundEvent) -> Result<SendOutcome> {
        self.send_command(ActorCommand::Event(Box::new(event)))
            .await
    }

    pub(crate) async fn send_event_with_ack(
        &self,
        event: InboundEvent,
        ack: tokio::sync::oneshot::Sender<Result<()>>,
    ) -> Result<SendOutcome> {
        self.send_command(ActorCommand::EventWithAck {
            event: Box::new(event),
            ack,
        })
        .await
    }

    pub(crate) async fn stop(&self) -> Result<(SendOutcome, Option<bool>)> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::Stop { reply: reply_tx })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok((outcome, None));
        }
        match reply_rx.await {
            Ok(result) => Ok((SendOutcome::Sent, Some(result?))),
            Err(_) => Ok((SendOutcome::Closed, None)),
        }
    }

    pub(crate) async fn steer(&self, guidance: String) -> Result<(SendOutcome, Option<bool>)> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::Steer {
                guidance,
                reply: reply_tx,
            })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok((outcome, None));
        }
        match reply_rx.await {
            Ok(result) => Ok((SendOutcome::Sent, Some(result?))),
            Err(_) => Ok((SendOutcome::Closed, None)),
        }
    }

    pub(crate) async fn set_model(&self, model: String) -> Result<()> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::SetModel {
                model,
                reply: reply_tx,
            })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok(());
        }
        reply_rx
            .await
            .map_err(|_| OmonError::Multiplexer("actor closed before set_model reply".into()))?
    }

    pub(crate) async fn reset(&self) -> Result<()> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::Reset { reply: reply_tx })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok(());
        }
        reply_rx
            .await
            .map_err(|_| OmonError::Multiplexer("actor closed before reset reply".into()))?
    }

    pub(crate) async fn get_context(&self) -> Result<SessionContext> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::GetContext { reply: reply_tx })
            .await?;
        if outcome != SendOutcome::Sent {
            return Err(OmonError::Multiplexer("actor not accepting".into()));
        }
        reply_rx
            .await
            .map_err(|_| OmonError::Multiplexer("actor closed before get_context reply".into()))
    }

    async fn send_command(&self, command: ActorCommand) -> Result<SendOutcome> {
        self.wait_started().await?;
        if self.finished.load(Ordering::Acquire) {
            return Ok(SendOutcome::Closed);
        }
        if !self.accepting.load(Ordering::Acquire) {
            return Ok(SendOutcome::Retiring);
        }

        self.in_flight.fetch_add(1, Ordering::AcqRel);
        if !self.accepting.load(Ordering::Acquire) || self.finished.load(Ordering::Acquire) {
            self.finish_send();
            return Ok(if self.finished.load(Ordering::Acquire) {
                SendOutcome::Closed
            } else {
                SendOutcome::Retiring
            });
        }

        let result = self.sender.send(command).await;
        self.finish_send();
        Ok(if result.is_ok() {
            SendOutcome::Sent
        } else {
            SendOutcome::Closed
        })
    }

    fn finish_send(&self) {
        if self.in_flight.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.in_flight_drained.notify_waiters();
        }
    }

    pub(crate) fn try_retire(&self) -> bool {
        if self.finished.load(Ordering::Acquire) {
            return false;
        }
        self.accepting
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub(crate) async fn wait_for_in_flight(&self) {
        loop {
            let notified = self.in_flight_drained.notified();
            if self.in_flight.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }

    pub(crate) fn resume(&self) {
        if !self.finished.load(Ordering::Acquire) {
            self.accepting.store(true, Ordering::Release);
        }
        self.state_changed.notify_waiters();
    }

    pub(crate) fn mark_finished(&self) {
        self.accepting.store(false, Ordering::Release);
        self.finished.store(true, Ordering::Release);
        self.state_changed.notify_waiters();
        self.in_flight_drained.notify_waiters();
    }

    pub(crate) async fn wait_until_reusable(&self) {
        loop {
            let notified = self.state_changed.notified();
            if self.accepting.load(Ordering::Acquire) || self.finished.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Clone)]
pub struct SessionMultiplexer {
    pub(crate) sessions: Arc<DashMap<SessionKey, Arc<SessionHandle>>>,
    runner: Arc<dyn AgentRunner>,
    dispatcher: Option<Arc<dyn OutboundDispatcher>>,
    pool: SqlitePool,
    config: MultiplexerConfig,
    profile_router: Arc<ProfileRouter>,
    drain_rx: Option<tokio::sync::watch::Receiver<bool>>,
}

impl SessionMultiplexer {
    pub fn new(pool: SqlitePool, runner: Arc<dyn AgentRunner>, config: MultiplexerConfig) -> Self {
        Self::with_dispatcher(pool, runner, None, config)
    }

    pub fn with_dispatcher(
        pool: SqlitePool,
        runner: Arc<dyn AgentRunner>,
        dispatcher: Option<Arc<dyn OutboundDispatcher>>,
        config: MultiplexerConfig,
    ) -> Self {
        Self::with_profile_router(pool, runner, dispatcher, config, ProfileRouter::default())
    }

    pub fn with_profile_router(
        pool: SqlitePool,
        runner: Arc<dyn AgentRunner>,
        dispatcher: Option<Arc<dyn OutboundDispatcher>>,
        config: MultiplexerConfig,
        profile_router: ProfileRouter,
    ) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            runner,
            dispatcher,
            pool,
            config,
            profile_router: Arc::new(profile_router),
            drain_rx: None,
        }
    }

    pub fn with_drain_receiver(mut self, rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.drain_rx = Some(rx);
        self
    }

    pub fn with_router(mut self, profile_router: ProfileRouter) -> Self {
        self.profile_router = Arc::new(profile_router);
        self
    }

    pub fn profile_router(&self) -> &ProfileRouter {
        &self.profile_router
    }

    pub async fn route(&self, event: InboundEvent) -> Result<()> {
        if let Some(ref rx) = self.drain_rx {
            if *rx.borrow() {
                return Err(OmonError::Config(
                    "gateway is draining; new turns are refused".into(),
                ));
            }
        }
        let key = event.session.clone();
        loop {
            let handle = self.handle_for(&key);
            match handle.send_event(event.clone()).await? {
                SendOutcome::Sent => return Ok(()),
                SendOutcome::Retiring => {
                    handle.wait_until_reusable().await;
                }
                SendOutcome::Closed => {
                    self.remove_handle(&key, &handle);
                    handle.mark_finished();
                }
            }
        }
    }

    /// Cancels the active turn for an existing session. Returns `false` when
    /// the session has no active execution (or no actor at all).
    pub async fn stop(&self, key: &SessionKey) -> Result<bool> {
        if let Some(dispatcher) = &self.dispatcher {
            let _ = dispatcher
                .dispatch(crate::OutboundAction::Typing {
                    session: key.clone(),
                    active: false,
                })
                .await;
        }
        loop {
            let Some(handle) = self.sessions.get(key).map(|entry| entry.clone()) else {
                crate::storage::mark_session_suspended(&self.pool, &key.storage_key(), true)
                    .await?;
                return Ok(false);
            };
            match handle.stop().await? {
                (SendOutcome::Sent, Some(interrupted)) => return Ok(interrupted),
                (SendOutcome::Retiring, _) => handle.wait_until_reusable().await,
                (SendOutcome::Closed, _) | (SendOutcome::Sent, None) => {
                    self.remove_handle(key, &handle);
                    handle.mark_finished();
                }
            }
        }
    }

    /// Injects steering guidance immediately into an ongoing turn for the specified session.
    /// Returns `Ok(true)` if guidance was successfully steered into an active turn,
    /// `Ok(false)` if no turn was currently active or steer was unsupported.
    pub async fn steer(&self, key: &SessionKey, guidance: &str) -> Result<bool> {
        let Some(handle) = self.sessions.get(key).map(|entry| entry.clone()) else {
            return Ok(false);
        };
        match handle.steer(guidance.to_string()).await? {
            (SendOutcome::Sent, Some(steered)) => Ok(steered),
            _ => Ok(false),
        }
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }

    pub fn contains_session(&self, key: &SessionKey) -> bool {
        self.sessions.contains_key(key)
    }

    /// Routes an event and resolves once its turn reaches a terminal outcome.
    ///
    /// Startup backfill uses this so a durability cursor only advances past a message
    /// whose replayed turn actually succeeded. The wait is signal-driven, never polled.
    pub async fn route_awaiting_turn(&self, event: InboundEvent) -> Result<()> {
        let key = event.session.clone();
        loop {
            let handle = self.handle_for(&key);
            let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
            match handle.send_event_with_ack(event.clone(), ack_tx).await? {
                SendOutcome::Sent => {
                    return match ack_rx.await {
                        Ok(outcome) => outcome,
                        Err(_) => Err(OmonError::Multiplexer(
                            "session actor dropped the turn before reporting an outcome".into(),
                        )),
                    };
                }
                SendOutcome::Retiring => {
                    handle.wait_until_reusable().await;
                }
                SendOutcome::Closed => {
                    self.remove_handle(&key, &handle);
                    handle.mark_finished();
                }
            }
        }
    }

    /// Touches the session's activity timestamp to prevent premature idle GC eviction.
    pub fn touch_activity(&self, key: &SessionKey) {
        if let Some(handle) = self.sessions.get(key) {
            let _ = handle.sender.try_send(ActorCommand::TouchActivity);
        }
    }

    pub async fn set_model(&self, key: &SessionKey, model: String) -> Result<()> {
        let storage_key = key.storage_key();
        let state_json: Option<String> =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(&storage_key)
                .fetch_optional(&self.pool)
                .await?;
        let mut state: crate::SessionState = state_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        state.active_model = Some(model.clone());
        let state_str =
            serde_json::to_string(&state).map_err(|e| OmonError::Database(e.to_string()))?;
        sqlx::query(
            "UPDATE sessions SET state_json = ?, updated_at = CURRENT_TIMESTAMP WHERE session_key = ?",
        )
        .bind(state_str)
        .bind(&storage_key)
        .execute(&self.pool)
        .await?;

        if let Some(handle) = self.sessions.get(key) {
            let _ = handle.set_model(model).await;
        }
        Ok(())
    }

    pub async fn reset(&self, key: &SessionKey) -> Result<()> {
        if let Some(handle) = self.sessions.get(key) {
            let _ = handle.reset().await;
        }
        Ok(())
    }

    pub async fn session_context(&self, key: &SessionKey) -> Result<Option<SessionContext>> {
        if let Some(handle) = self.sessions.get(key) {
            if let Ok(ctx) = handle.get_context().await {
                return Ok(Some(ctx));
            }
        }
        let storage_key = key.storage_key();
        let state_json: Option<String> =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(&storage_key)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(json_str) = state_json {
            let state: crate::SessionState = serde_json::from_str(&json_str).unwrap_or_default();
            let mut ctx = SessionContext::new(key.clone());
            ctx.state = state;
            Ok(Some(ctx))
        } else {
            Ok(None)
        }
    }

    /// Marks all currently tracked in-flight sessions as `resume_pending` in SQLite.
    pub async fn mark_in_flight_resume_pending(&self) -> Result<usize> {
        let mut count = 0;
        for entry in self.sessions.iter() {
            let key = entry.key();
            crate::storage::mark_session_resume_pending(&self.pool, &key.storage_key()).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Returns an activity heartbeat callback suitable for injecting into approval requesters.
    pub fn activity_heartbeat(&self) -> Arc<dyn Fn(&SessionKey) + Send + Sync> {
        let multiplexer = self.clone();
        Arc::new(move |key: &SessionKey| {
            multiplexer.touch_activity(key);
        })
    }

    pub async fn collect_garbage(&self) -> Result<usize> {
        super::gc::collect(self).await
    }

    fn handle_for(&self, key: &SessionKey) -> Arc<SessionHandle> {
        if let Some(handle) = self.sessions.get(key) {
            return handle.clone();
        }

        let (sender, receiver) = mpsc::channel(SESSION_CHANNEL_CAPACITY);
        let (startup_tx, startup_rx) = watch::channel(ActorStartup::Starting);
        let handle = Arc::new(SessionHandle::new(sender, startup_rx));
        match self.sessions.entry(key.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                entry.insert(handle.clone());
                self.spawn_actor(key.clone(), receiver, Arc::downgrade(&handle), startup_tx);
                handle
            }
        }
    }

    fn spawn_actor(
        &self,
        key: SessionKey,
        receiver: mpsc::Receiver<ActorCommand>,
        handle: Weak<SessionHandle>,
        startup: watch::Sender<ActorStartup>,
    ) {
        let sessions = Arc::downgrade(&self.sessions);
        let runner = self.runner.clone();
        let dispatcher = self.dispatcher.clone();
        let pool = self.pool.clone();
        let profile_router = self.profile_router.clone();
        tokio::spawn(async move {
            match SessionActor::load(
                key.clone(),
                receiver,
                runner,
                dispatcher,
                pool,
                Some(profile_router),
            )
            .await
            {
                Ok(actor) => {
                    let _ = startup.send(ActorStartup::Ready);
                    actor.run().await;
                }
                Err(error) => {
                    let _ = startup.send(ActorStartup::Failed(error.to_string()));
                    tracing::error!(session = %key, %error, "failed to start session actor");
                }
            }
            if let Some(handle) = handle.upgrade() {
                if let Some(sessions) = sessions.upgrade() {
                    sessions.remove_if(&key, |_, current| Arc::ptr_eq(current, &handle));
                }
                handle.mark_finished();
            }
        });
    }

    pub(crate) fn remove_handle(&self, key: &SessionKey, handle: &Arc<SessionHandle>) -> bool {
        self.sessions
            .remove_if(key, |_, current| Arc::ptr_eq(current, handle))
            .is_some()
    }

    pub(crate) fn idle_timeout(&self) -> Duration {
        self.config.idle_timeout
    }

    pub(crate) fn gc_interval(&self) -> Duration {
        self.config.gc_interval
    }
}

impl From<mpsc::error::SendError<ActorCommand>> for OmonError {
    fn from(_: mpsc::error::SendError<ActorCommand>) -> Self {
        OmonError::Multiplexer("session actor stopped before accepting an event".into())
    }
}

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
    shutdown_error: parking_lot::Mutex<Option<String>>,
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
            shutdown_error: parking_lot::Mutex::new(None),
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

    /// Reports `Sent` only once the live actor applied the change. `Retiring` and `Closed`
    /// mean it was NOT delivered, so the caller must wait and retry or drop the dead handle.
    pub(crate) async fn set_model(&self, model: String) -> Result<SendOutcome> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::SetModel {
                model,
                reply: reply_tx,
            })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok(outcome);
        }
        match reply_rx.await {
            Ok(result) => result.map(|()| SendOutcome::Sent),
            Err(_) => Ok(SendOutcome::Closed),
        }
    }

    pub(crate) async fn set_yolo(&self, yolo: bool) -> Result<SendOutcome> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::SetYolo {
                yolo,
                reply: reply_tx,
            })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok(outcome);
        }
        match reply_rx.await {
            Ok(result) => result.map(|()| SendOutcome::Sent),
            Err(_) => Ok(SendOutcome::Closed),
        }
    }

    pub(crate) async fn reset(&self) -> Result<SendOutcome> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let outcome = self
            .send_command(ActorCommand::Reset { reply: reply_tx })
            .await?;
        if outcome != SendOutcome::Sent {
            return Ok(outcome);
        }
        match reply_rx.await {
            Ok(result) => result.map(|()| SendOutcome::Sent),
            Err(_) => Ok(SendOutcome::Closed),
        }
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
    inbox_event_tx: Option<crate::inbox::watchers::InboxEventSender>,
    service_shutdown: tokio_util::sync::CancellationToken,
    admission: Arc<tokio::sync::RwLock<()>>,
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
            inbox_event_tx: None,
            service_shutdown: tokio_util::sync::CancellationToken::new(),
            admission: Arc::new(tokio::sync::RwLock::new(())),
        }
    }

    pub fn with_drain_receiver(mut self, rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.drain_rx = Some(rx);
        self
    }

    pub fn with_inbox_event_sender(
        mut self,
        sender: crate::inbox::watchers::InboxEventSender,
    ) -> Self {
        self.inbox_event_tx = Some(sender);
        self
    }

    pub(crate) fn inbox_event_sender(&self) -> Option<crate::inbox::watchers::InboxEventSender> {
        self.inbox_event_tx.clone()
    }

    pub fn with_router(mut self, profile_router: ProfileRouter) -> Self {
        self.profile_router = Arc::new(profile_router);
        self
    }

    pub fn profile_router(&self) -> &ProfileRouter {
        &self.profile_router
    }

    pub fn is_draining(&self) -> bool {
        self.service_shutdown.is_cancelled()
            || self.drain_rx.as_ref().is_some_and(|rx| *rx.borrow())
    }

    pub async fn route(&self, event: InboundEvent) -> Result<()> {
        let _admission = self.admission.read().await;
        if self.is_draining() {
            return Err(OmonError::Config(
                "gateway is draining; new turns are refused".into(),
            ));
        }
        let key = event.session.clone();
        loop {
            if self.is_draining() {
                return Err(OmonError::Multiplexer("gateway admission closed".into()));
            }
            let handle = self.handle_for(&key);
            if event.origin == crate::EventOrigin::System {
                let (reply, result) = tokio::sync::oneshot::channel();
                match handle
                    .send_command(ActorCommand::EventIfIdle {
                        event: Box::new(event.clone()),
                        reply,
                    })
                    .await?
                {
                    SendOutcome::Sent => {
                        return result.await.map_err(|_| {
                            OmonError::Multiplexer("actor closed during system admission".into())
                        })?
                    }
                    SendOutcome::Retiring => handle.wait_until_reusable().await,
                    SendOutcome::Closed => {
                        self.remove_handle(&key, &handle);
                        handle.mark_finished();
                    }
                }
                continue;
            }
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

    pub async fn direct_reply(&self, event: &InboundEvent, content: &str) -> Result<()> {
        let dispatcher = self
            .dispatcher
            .as_ref()
            .ok_or_else(|| OmonError::Config("direct reply delivery is not configured".into()))?;
        super::record_inbound_transcript(&self.pool, event).await?;
        let ledger = crate::DeliveryLedgerService::new(self.pool.clone());
        let id = uuid::Uuid::new_v4().to_string();
        ledger
            .record_obligation(&id, &event.session, content)
            .await?;
        sqlx::query("INSERT INTO messages (id, session_key, role, content, metadata_json, created_at) VALUES (?, ?, 'assistant', ?, ?, ?)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(event.session.storage_key())
            .bind(content)
            .bind(serde_json::json!({"delivery_obligation_id": id, "direct_reply": true}).to_string())
            .bind(chrono::Utc::now())
            .execute(&self.pool).await?;
        ledger.mark_obligation_attempting(&id).await?;
        match dispatcher
            .dispatch_direct_reply(event.session.clone(), content.to_owned())
            .await
        {
            Ok(()) => ledger.mark_obligation_delivered(&id).await,
            Err(error) => {
                ledger
                    .mark_obligation_failed(&id, &error.to_string())
                    .await?;
                Err(error)
            }
        }
    }

    pub async fn owner_thread_is_private(&self, session: &SessionKey) -> bool {
        match &self.dispatcher {
            Some(dispatcher) => dispatcher.owner_thread_is_private(session).await,
            None => false,
        }
    }

    pub async fn intercept_session_status(
        &self,
        event: &InboundEvent,
        private_owner: bool,
    ) -> Result<bool> {
        self.intercept_session_status_with(
            event,
            private_owner,
            crate::sessions::status_reply::reply(),
        )
        .await
    }

    pub async fn intercept_session_status_with(
        &self,
        event: &InboundEvent,
        private_owner: bool,
        reply: impl std::future::Future<Output = String>,
    ) -> Result<bool> {
        if !crate::sessions::status_reply::eligible(event, private_owner) {
            return Ok(false);
        }
        let content = reply.await;
        self.direct_reply(event, &content).await?;
        Ok(true)
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

    pub async fn session_is_idle(&self, key: &SessionKey) -> Result<bool> {
        let Some(handle) = self.sessions.get(key).map(|entry| entry.clone()) else {
            return Ok(true);
        };
        let (reply, result) = tokio::sync::oneshot::channel();
        match handle.send_command(ActorCommand::IsIdle { reply }).await? {
            SendOutcome::Sent => result
                .await
                .map_err(|_| OmonError::Multiplexer("actor closed during idle probe".into())),
            SendOutcome::Retiring => Ok(false),
            SendOutcome::Closed => Ok(true),
        }
    }

    pub async fn wait_for_session_idle(&self, key: &SessionKey) -> Result<()> {
        let Some(handle) = self.sessions.get(key).map(|entry| entry.clone()) else {
            return Ok(());
        };
        let (reply, result) = tokio::sync::oneshot::channel();
        match handle
            .send_command(ActorCommand::WaitIdle { reply })
            .await?
        {
            SendOutcome::Sent => result
                .await
                .map_err(|_| OmonError::Multiplexer("actor closed before idle barrier".into())),
            SendOutcome::Retiring | SendOutcome::Closed => Err(OmonError::Multiplexer(
                "actor unavailable for idle barrier".into(),
            )),
        }
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
            let admission = self.admission.read().await;
            if self.is_draining() {
                return Err(OmonError::Multiplexer("gateway admission closed".into()));
            }
            let handle = self.handle_for(&key);
            let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
            match handle.send_event_with_ack(event.clone(), ack_tx).await? {
                SendOutcome::Sent => {
                    drop(admission);
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
        // Patch only `active_model` in place: a corrupt `state_json` makes `json_set` fail
        // instead of being silently replaced by a default state, and the write stays atomic.
        sqlx::query(
            "UPDATE sessions SET state_json = json_set(state_json, '$.active_model', ?), updated_at = CURRENT_TIMESTAMP WHERE session_key = ?",
        )
        .bind(&model)
        .bind(key.storage_key())
        .execute(&self.pool)
        .await?;

        self.apply_to_live_actor(key, |handle| {
            let model = model.clone();
            async move { handle.set_model(model).await }
        })
        .await
    }

    /// Pushes a persisted yolo change into the live actor (no-op without one).
    pub async fn set_yolo(&self, key: &SessionKey, yolo: bool) -> Result<()> {
        self.apply_to_live_actor(key, |handle| async move { handle.set_yolo(yolo).await })
            .await
    }

    pub async fn reset(&self, key: &SessionKey) -> Result<()> {
        self.apply_to_live_actor(key, |handle| async move { handle.reset().await })
            .await
    }

    /// Delivers a state-mutating command to the session's live actor. A retiring actor (GC is
    /// probing it) is waited for and retried, and a closed one is dropped, so the command is
    /// never reported as applied while the live actor keeps stale state. No actor is a no-op.
    async fn apply_to_live_actor<F, Fut>(&self, key: &SessionKey, command: F) -> Result<()>
    where
        F: Fn(Arc<SessionHandle>) -> Fut,
        Fut: std::future::Future<Output = Result<SendOutcome>>,
    {
        loop {
            let Some(handle) = self.sessions.get(key).map(|entry| entry.clone()) else {
                return Ok(());
            };
            match command(handle.clone()).await? {
                SendOutcome::Sent => return Ok(()),
                SendOutcome::Retiring => handle.wait_until_reusable().await,
                SendOutcome::Closed => {
                    self.remove_handle(key, &handle);
                    handle.mark_finished();
                }
            }
        }
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

    /// Marks sessions that have an active turn or queued events as `resume_pending` in SQLite.
    /// Resident but idle sessions are skipped: their last turn already finished (or failed
    /// visibly), and re-running it on restart would repeat side effects.
    pub async fn mark_in_flight_resume_pending(&self) -> Result<usize> {
        let sessions: Vec<_> = self
            .sessions
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        let mut count = 0;
        for (key, handle) in sessions {
            if !Self::has_unfinished_work(&handle).await {
                continue;
            }
            crate::storage::mark_session_resume_pending(&self.pool, &key.storage_key()).await?;
            count += 1;
        }
        Ok(count)
    }

    /// True when the actor is running a turn or holds queued events. A full mailbox or an actor
    /// that does not answer within the probe window is assumed busy so shutdown never loses
    /// in-flight work. The probe uses `try_send` (no in-flight accounting) so a timeout can never
    /// leak the send counter that `shutdown` waits on.
    async fn has_unfinished_work(handle: &Arc<SessionHandle>) -> bool {
        const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
        if handle.finished.load(Ordering::Acquire) {
            return false;
        }
        let (reply, result) = tokio::sync::oneshot::channel();
        match handle.sender.try_send(ActorCommand::IsIdle { reply }) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => return true,
            Err(mpsc::error::TrySendError::Closed(_)) => return false,
        }
        match tokio::time::timeout(PROBE_TIMEOUT, result).await {
            Ok(Ok(idle)) => !idle,
            Ok(Err(_)) => false,
            Err(_) => true,
        }
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

    pub async fn shutdown(&self) -> Result<()> {
        self.service_shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(30), async {
            let _admission = self.admission.write().await;
            let handles: Vec<_> = self
                .sessions
                .iter()
                .map(|entry| entry.value().clone())
                .collect();
            for handle in handles {
                handle.try_retire();
                handle.wait_for_in_flight().await;
                loop {
                    let finished = handle.state_changed.notified();
                    if handle.finished.load(Ordering::Acquire) {
                        break;
                    }
                    finished.await;
                }
                if let Some(error) = handle.shutdown_error.lock().as_ref() {
                    return Err(OmonError::Multiplexer(error.clone()));
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| OmonError::Multiplexer("actor shutdown exceeded 30 seconds".into()))?
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
        let inbox_event_tx = self.inbox_event_tx.clone();
        let service_shutdown = self.service_shutdown.clone();
        tokio::spawn(async move {
            match SessionActor::load(
                key.clone(),
                receiver,
                runner,
                dispatcher,
                pool,
                Some(profile_router),
                inbox_event_tx,
            )
            .await
            {
                Ok(actor) => {
                    let _ = startup.send(ActorStartup::Ready);
                    if let Err(error) = actor.run_with_shutdown(service_shutdown).await {
                        if let Some(handle) = handle.upgrade() {
                            *handle.shutdown_error.lock() = Some(error.to_string());
                        }
                        tracing::error!(session = %key, %error, "session actor final flush failed");
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use async_trait::async_trait;

    struct GateRunner {
        started: mpsc::UnboundedSender<String>,
        release: Arc<Notify>,
    }

    #[async_trait]
    impl AgentRunner for GateRunner {
        async fn run(&self, _session: &mut SessionContext, event: InboundEvent) -> Result<()> {
            let _ = self.started.send(event.content.clone());
            if event.content == "blocking" {
                self.release.notified().await;
            }
            Ok(())
        }
    }

    fn gate_multiplexer(
        db: &Database,
        config: MultiplexerConfig,
    ) -> (
        SessionMultiplexer,
        mpsc::UnboundedReceiver<String>,
        Arc<Notify>,
    ) {
        let (started_tx, started_rx) = mpsc::unbounded_channel();
        let release = Arc::new(Notify::new());
        let multiplexer = SessionMultiplexer::new(
            db.pool().clone(),
            Arc::new(GateRunner {
                started: started_tx,
                release: release.clone(),
            }),
            config,
        );
        (multiplexer, started_rx, release)
    }

    fn key(channel: &str) -> SessionKey {
        SessionKey::new("discord", Some("guild"), channel, None::<String>, "u")
    }

    async fn resume_pending(db: &Database, key: &SessionKey) -> i64 {
        sqlx::query_scalar("SELECT resume_pending FROM sessions WHERE session_key = ?")
            .bind(key.storage_key())
            .fetch_one(db.pool())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn mark_in_flight_resume_pending_skips_idle_sessions() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (multiplexer, mut started_rx, release) =
            gate_multiplexer(&db, MultiplexerConfig::default());
        let busy = key("channel-busy");
        let idle = key("channel-idle");

        multiplexer
            .route_awaiting_turn(InboundEvent::message(idle.clone(), "m-idle", "quick"))
            .await
            .unwrap();
        multiplexer
            .route(InboundEvent::message(busy.clone(), "m-busy", "blocking"))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(content) = started_rx.recv().await {
                if content == "blocking" {
                    break;
                }
            }
        })
        .await
        .expect("busy turn should start");

        assert_eq!(
            multiplexer.mark_in_flight_resume_pending().await.unwrap(),
            1
        );
        assert_eq!(resume_pending(&db, &busy).await, 1);
        assert_eq!(
            resume_pending(&db, &idle).await,
            0,
            "a resident idle session must not be re-run on restart"
        );

        release.notify_one();
        multiplexer.wait_for_session_idle(&busy).await.unwrap();
    }

    #[tokio::test]
    async fn set_yolo_waits_out_a_retiring_actor_instead_of_dropping_the_change() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (multiplexer, _started_rx, _release) =
            gate_multiplexer(&db, MultiplexerConfig::default());
        let key = key("channel-retire");
        multiplexer
            .route_awaiting_turn(InboundEvent::message(key.clone(), "m-1", "quick"))
            .await
            .unwrap();

        // GC probing an actor: it stops accepting commands until the probe is answered.
        let handle = multiplexer
            .sessions
            .get(&key)
            .map(|entry| entry.clone())
            .unwrap();
        assert!(handle.try_retire());

        let task = tokio::spawn({
            let multiplexer = multiplexer.clone();
            let key = key.clone();
            async move { multiplexer.set_yolo(&key, true).await }
        });
        // The test runtime is single-threaded: one yield runs the task to its first pending
        // point, which is the wait for the actor to accept commands again.
        tokio::task::yield_now().await;
        assert!(
            !task.is_finished(),
            "set_yolo must wait while the actor is retiring"
        );
        handle.resume();
        task.await.unwrap().unwrap();

        let context = multiplexer.session_context(&key).await.unwrap().unwrap();
        assert!(context.state.yolo, "the live actor must observe the change");
    }

    async fn insert_session_state(db: &Database, key: &SessionKey, state_json: &str) {
        sqlx::query(
            "INSERT INTO sessions (
                session_key, platform, guild_id, channel_id, thread_id, user_id,
                state_json, created_at, updated_at
             ) VALUES (?, 'discord', 'guild', 'channel-sm', NULL, 'u', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(key.storage_key())
        .bind(state_json)
        .execute(db.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn set_model_patches_only_the_model_and_refuses_corrupt_state() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (multiplexer, _started_rx, _release) =
            gate_multiplexer(&db, MultiplexerConfig::default());

        let intact = key("channel-sm-intact");
        insert_session_state(
            &db,
            &intact,
            r#"{"yolo":true,"system_prompt":"keep","metadata":{"omo_thread_id":"t1"}}"#,
        )
        .await;
        multiplexer
            .set_model(&intact, "model-b".into())
            .await
            .unwrap();
        let stored: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(intact.storage_key())
                .fetch_one(db.pool())
                .await
                .unwrap();
        let state: crate::SessionState = serde_json::from_str(&stored).unwrap();
        assert_eq!(state.active_model.as_deref(), Some("model-b"));
        assert!(state.yolo);
        assert_eq!(state.system_prompt.as_deref(), Some("keep"));
        assert_eq!(
            state.metadata.get("omo_thread_id"),
            Some(&serde_json::json!("t1"))
        );

        let corrupt = key("channel-sm-corrupt");
        insert_session_state(&db, &corrupt, "{not-json").await;
        assert!(multiplexer
            .set_model(&corrupt, "model-b".into())
            .await
            .is_err());
        let raw: String =
            sqlx::query_scalar("SELECT state_json FROM sessions WHERE session_key = ?")
                .bind(corrupt.storage_key())
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(raw, "{not-json", "corrupt state must not be overwritten");
    }

    #[tokio::test]
    async fn gc_keeps_evicting_after_one_session_fails_to_flush() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let (multiplexer, _started_rx, _release) = gate_multiplexer(
            &db,
            MultiplexerConfig {
                idle_timeout: Duration::ZERO,
                gc_interval: Duration::from_secs(60),
            },
        );
        let failing = key("channel-gc-failing");
        let healthy = key("channel-gc-healthy");
        for (session, id) in [(&failing, "m-f"), (&healthy, "m-h")] {
            multiplexer
                .route_awaiting_turn(InboundEvent::message(session.clone(), id, "quick"))
                .await
                .unwrap();
        }

        // Leave the failing actor dirty with a flush that cannot succeed.
        sqlx::query(&format!(
            "CREATE TRIGGER fail_gc_flush BEFORE UPDATE OF state_json ON sessions
             WHEN old.session_key = '{}'
             BEGIN SELECT RAISE(ABORT, 'forced flush failure'); END;",
            failing.storage_key()
        ))
        .execute(db.pool())
        .await
        .unwrap();
        assert!(multiplexer.set_yolo(&failing, true).await.is_err());

        // A zero idle timeout needs the actor clock to tick past its last activity, so retry
        // (bounded, no sleeping) until the healthy session is evicted.
        let mut evicted = 0;
        for _ in 0..1000 {
            evicted += multiplexer.collect_garbage().await.unwrap();
            if !multiplexer.contains_session(&healthy) {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(evicted, 1, "the healthy idle session must still be evicted");
        assert!(!multiplexer.contains_session(&healthy));
        assert!(multiplexer.contains_session(&failing));
    }
}

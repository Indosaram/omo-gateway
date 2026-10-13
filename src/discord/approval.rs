use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serenity::all::{ButtonStyle, CreateActionRow, CreateButton};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::sync::{oneshot, RwLock};
use uuid::Uuid;

use crate::{OutboundAction, OutboundDispatcher, SessionKey};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    #[serde(alias = "approved")]
    Once,
    Session,
    Always,
    #[serde(alias = "rejected")]
    Deny {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

impl ApprovalDecision {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Once | Self::Session | Self::Always)
    }

    pub fn deny(reason: Option<String>) -> Self {
        Self::Deny { reason }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ApprovalError {
    #[error("approval request timed out")]
    Timeout,
    #[error("approval request was cancelled")]
    Cancelled,
    #[error("failed to persist always-allow approval: {0}")]
    Persistence(String),
}

pub const DEFAULT_APPROVAL_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
pub type ActivityHeartbeat = Arc<dyn Fn(&SessionKey) + Send + Sync>;

#[derive(Debug)]
pub struct ApprovalPrompt {
    pub request_id: Uuid,
    pub components: Vec<CreateActionRow>,
    receiver: oneshot::Receiver<ApprovalDecision>,
    lease: PendingLease,
}

#[derive(Debug)]
struct PendingLease {
    request_id: Uuid,
    pending: Arc<Mutex<HashMap<Uuid, PendingApprovalEntry>>>,
}

impl Drop for PendingLease {
    fn drop(&mut self) {
        self.pending.lock().remove(&self.request_id);
    }
}

impl ApprovalPrompt {
    pub async fn wait(self, timeout: Duration) -> Result<ApprovalDecision, ApprovalError> {
        let _lease = self.lease;
        match tokio::time::timeout(timeout, self.receiver).await {
            Ok(Ok(decision)) => Ok(decision),
            Ok(Err(_)) => Err(ApprovalError::Cancelled),
            Err(_) => Err(ApprovalError::Timeout),
        }
    }

    pub async fn wait_with_heartbeat<F>(
        self,
        timeout: Duration,
        interval: Duration,
        mut heartbeat_fn: F,
    ) -> Result<ApprovalDecision, ApprovalError>
    where
        F: FnMut() + Send,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        let _lease = self.lease;
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        ticker.tick().await;

        let mut receiver = self.receiver;
        loop {
            tokio::select! {
                biased;
                res = &mut receiver => {
                    return match res {
                        Ok(decision) => Ok(decision),
                        Err(_) => Err(ApprovalError::Cancelled),
                    };
                }
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(ApprovalError::Timeout);
                }
                _ = ticker.tick() => {
                    heartbeat_fn();
                }
            }
        }
    }
}

#[derive(Debug)]
struct PendingApprovalEntry {
    session: Option<SessionKey>,
    sender: oneshot::Sender<ApprovalDecision>,
    created_at: std::time::Instant,
}

/// Tracks pending approval requests, resolves Discord button interactions
/// through a one-shot channel, and maintains per-session and global approval caches.
#[derive(Clone, Default)]
pub struct SmartApprovalGuard {
    pending: Arc<Mutex<HashMap<Uuid, PendingApprovalEntry>>>,
    session_cache: Arc<RwLock<HashMap<SessionKey, HashSet<String>>>>,
    yolo_sessions: Arc<RwLock<HashSet<String>>>,
    always_cache: Arc<RwLock<HashSet<String>>>,
    pool: Arc<RwLock<Option<SqlitePool>>>,
}

#[async_trait]
pub trait ApprovalRequester: Send + Sync {
    async fn request_approval(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
    ) -> Result<ApprovalDecision, ApprovalError>;

    /// Request a tool rule independently of its human-readable display target.
    /// Requesters without remembered grants can retain the original interface.
    async fn request_approval_scoped(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
        _pattern_key: &str,
    ) -> Result<ApprovalDecision, ApprovalError> {
        self.request_approval(session, command, reason).await
    }

    /// Requesters that remember grants must override this and cap before caching.
    async fn request_approval_with_max_scope(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
        max_scope: ApprovalScope,
    ) -> Result<ApprovalDecision, ApprovalError> {
        self.request_approval(session, command, reason)
            .await
            .map(|decision| max_scope.cap(decision))
    }

    async fn is_yolo(&self, _session: &SessionKey) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalScope {
    Once,
    Session,
    Always,
}

impl ApprovalScope {
    fn cap(self, decision: ApprovalDecision) -> ApprovalDecision {
        match (self, decision) {
            (Self::Once, ApprovalDecision::Session | ApprovalDecision::Always) => {
                ApprovalDecision::Once
            }
            (Self::Session, ApprovalDecision::Always) => ApprovalDecision::Session,
            (_, decision) => decision,
        }
    }
}

#[derive(Clone)]
pub struct DiscordApprovalRequester {
    guard: SmartApprovalGuard,
    dispatcher: Arc<RwLock<Option<Arc<dyn OutboundDispatcher>>>>,
    heartbeat: Arc<RwLock<Option<ActivityHeartbeat>>>,
    heartbeat_interval: Duration,
    timeout: Duration,
}

impl DiscordApprovalRequester {
    pub fn new(guard: SmartApprovalGuard, timeout: Duration) -> Self {
        Self {
            guard,
            dispatcher: Arc::new(RwLock::new(None)),
            heartbeat: Arc::new(RwLock::new(None)),
            heartbeat_interval: DEFAULT_APPROVAL_HEARTBEAT_INTERVAL,
            timeout,
        }
    }

    pub fn with_heartbeat(self, heartbeat: ActivityHeartbeat) -> Self {
        *self.heartbeat.try_write().expect("uncontended lock") = Some(heartbeat);
        self
    }

    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    pub async fn set_heartbeat(&self, heartbeat: ActivityHeartbeat) {
        *self.heartbeat.write().await = Some(heartbeat);
    }

    pub async fn set_dispatcher(&self, dispatcher: Arc<dyn OutboundDispatcher>) {
        *self.dispatcher.write().await = Some(dispatcher);
    }

    pub fn guard(&self) -> &SmartApprovalGuard {
        &self.guard
    }
}

#[async_trait]
impl ApprovalRequester for DiscordApprovalRequester {
    async fn is_yolo(&self, session: &SessionKey) -> bool {
        self.guard.is_yolo(session).await
    }

    async fn request_approval(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
    ) -> Result<ApprovalDecision, ApprovalError> {
        let pattern_key = crate::security::derive_pattern_key(command);
        self.request_approval_scoped(session, command, reason, &pattern_key)
            .await
    }

    async fn request_approval_scoped(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
        pattern_key: &str,
    ) -> Result<ApprovalDecision, ApprovalError> {
        self.request_with_scope(session, command, reason, pattern_key, ApprovalScope::Always)
            .await
    }

    async fn request_approval_with_max_scope(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
        max_scope: ApprovalScope,
    ) -> Result<ApprovalDecision, ApprovalError> {
        if max_scope == ApprovalScope::Always {
            return self.request_approval(session, command, reason).await;
        }
        // Scanner grants are distinct from broad terminal category grants and
        // from findings on another command. Nothing capped is persisted globally.
        let key = format!(
            "tirith:{max_scope:?}:{}",
            serde_json::json!([command, reason])
        );
        self.request_with_scope(session, command, reason, &key, max_scope)
            .await
    }
}

impl DiscordApprovalRequester {
    async fn request_with_scope(
        &self,
        session: &SessionKey,
        command: &str,
        reason: &str,
        pattern_key: &str,
        max_scope: ApprovalScope,
    ) -> Result<ApprovalDecision, ApprovalError> {
        if self.guard.is_yolo(session).await {
            return Ok(ApprovalDecision::Once);
        }

        let remembered = match max_scope {
            ApprovalScope::Once => false,
            ApprovalScope::Session => self
                .guard
                .session_cache
                .read()
                .await
                .get(session)
                .is_some_and(|keys| keys.contains(pattern_key)),
            ApprovalScope::Always => self.guard.is_approved(session, pattern_key).await,
        };
        if remembered {
            return Ok(ApprovalDecision::Session);
        }

        let dispatcher = self
            .dispatcher
            .read()
            .await
            .clone()
            .ok_or(ApprovalError::Cancelled)?;
        let (display_command, display_reason) = match (
            crate::security::redact_approval_display(command),
            crate::security::redact_approval_display(reason),
        ) {
            (Ok(command), Ok(reason)) => (command, reason),
            (Err(error), _) | (_, Err(error)) => {
                tracing::warn!(%error, "approval display preparation failed");
                return Err(ApprovalError::Cancelled);
            }
        };
        let prompt = self.guard.request_with_session(Some(session.clone())).await;
        let request_id = prompt.request_id;
        // The delivery owner outlives a dropped caller: it finishes dispatch before
        // consuming the terminal signal, so expiry cannot overtake a late send.
        let (terminal, ended) = oneshot::channel::<()>();
        let (sent, delivered) = oneshot::channel();
        let action = OutboundAction::ApprovalRequest {
            session: session.clone(),
            request_id,
            command: display_command,
            reason: display_reason,
        };
        let delivery = tokio::spawn(async move {
            let result =
                tokio::time::timeout(Duration::from_secs(10), dispatcher.dispatch(action)).await;
            let success = matches!(result, Ok(Ok(())));
            if let Ok(Err(error)) = &result {
                tracing::warn!(%error, %request_id, "approval delivery failed");
            }
            let _ = sent.send(success);
            let _ = ended.await;
            match tokio::time::timeout(
                Duration::from_secs(10),
                dispatcher.dispatch(OutboundAction::ExpireApproval { request_id }),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%error, %request_id, "approval expiry delivery failed")
                }
                Err(_) => tracing::warn!(%request_id, "approval expiry delivery timed out"),
            }
        });
        if delivered.await != Ok(true) {
            drop(prompt);
            drop(terminal);
            if let Err(error) = delivery.await {
                tracing::warn!(%error, "approval delivery task failed");
            }
            return Err(ApprovalError::Cancelled);
        }
        let heartbeat = self.heartbeat.read().await.clone();
        let heartbeat_interval = self.heartbeat_interval;
        let result = match heartbeat {
            Some(hb) => {
                let session_clone = session.clone();
                prompt
                    .wait_with_heartbeat(self.timeout, heartbeat_interval, move || {
                        hb(&session_clone);
                    })
                    .await
            }
            None => prompt.wait(self.timeout).await,
        };
        let result = result.map(|decision| max_scope.cap(decision));
        if let Ok(decision) = &result {
            match decision {
                ApprovalDecision::Session => {
                    self.guard.approve_session(session, pattern_key).await;
                }
                ApprovalDecision::Always => {
                    // A failed persistence must surface to the requester:
                    // silently downgrading to a non-durable grant would be
                    // the exact "reports success when persistence fails"
                    // defect this lane fixes.
                    self.guard
                        .approve_always(pattern_key)
                        .await
                        .map_err(|error| ApprovalError::Persistence(error.to_string()))?;
                }
                _ => {}
            }
        }
        drop(terminal);
        // The expiry edit is cosmetic and the task finishes on its own once
        // `terminal` is dropped; bound how long the decision waits for it so a
        // slow Discord API cannot delay an already-made decision by 10s.
        if let Ok(Err(error)) = tokio::time::timeout(Duration::from_secs(2), delivery).await {
            tracing::warn!(%error, "approval delivery task failed");
        }
        result
    }
}
impl SmartApprovalGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pool(self, pool: SqlitePool) -> Self {
        *self.pool.try_write().expect("uncontended lock") = Some(pool);
        self
    }

    pub async fn set_pool(&self, pool: SqlitePool) {
        *self.pool.write().await = Some(pool);
    }

    pub async fn load_persisted_allowlist(&self) -> Result<usize, sqlx::Error> {
        let pool = self.pool.read().await.clone();
        let Some(pool) = pool else {
            return Ok(0);
        };
        let rows: Vec<(String,)> = sqlx::query_as("SELECT pattern_key FROM approval_allowlist")
            .fetch_all(&pool)
            .await?;
        let mut always = self.always_cache.write().await;
        let count = rows.len();
        for (pattern,) in rows {
            always.insert(pattern);
        }
        Ok(count)
    }

    pub async fn load_persisted_yolo(&self) -> Result<usize, sqlx::Error> {
        let pool = self.pool.read().await.clone();
        let Some(pool) = pool else {
            return Ok(0);
        };
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT session_key, state_json FROM sessions")
                .fetch_all(&pool)
                .await?;
        let mut yolo = self.yolo_sessions.write().await;
        let mut count = 0;
        for (session_key, state_json) in rows {
            let state: crate::SessionState = serde_json::from_str(&state_json)
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
            if state.yolo {
                yolo.insert(session_key);
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn is_approved(&self, session: &SessionKey, pattern_key: &str) -> bool {
        if self.always_cache.read().await.contains(pattern_key) {
            return true;
        }
        if let Some(session_patterns) = self.session_cache.read().await.get(session) {
            return session_patterns.contains(pattern_key);
        }
        false
    }

    /// Upper bounds for the in-memory approval caches. A long-running
    /// gateway must not grow these without limit: keys embed the full
    /// command + reason text, and every distinct (session, command) pair
    /// from an agent run adds an entry.
    const MAX_SESSIONS: usize = 512;
    const MAX_PATTERNS_PER_SESSION: usize = 128;

    pub async fn approve_session(&self, session: &SessionKey, pattern_key: &str) {
        let mut cache = self.session_cache.write().await;
        let patterns = cache.entry(session.clone()).or_default();
        patterns.insert(pattern_key.to_string());
        // Bound the per-session set; evicting an arbitrary entry is safe for
        // a cache (worst case: one command re-asks for approval).
        while patterns.len() > Self::MAX_PATTERNS_PER_SESSION {
            let victim = patterns.iter().next().cloned();
            match victim {
                Some(v) => {
                    patterns.remove(&v);
                }
                None => break,
            }
        }
        // Bound the number of tracked sessions; drop an arbitrary one.
        while cache.len() > Self::MAX_SESSIONS {
            let victim = cache.keys().next().cloned();
            match victim {
                Some(v) => {
                    cache.remove(&v);
                }
                None => break,
            }
        }
    }

    /// Records a permanent allow. The durable row is written BEFORE the
    /// in-memory grant is published: persisting after would let a failed
    /// INSERT leave an active grant the user believes is permanent but that
    /// vanishes on restart. Returns Err only when persistence failed and no
    /// grant was published.
    pub async fn approve_always(&self, pattern_key: &str) -> Result<(), sqlx::Error> {
        let pool = self.pool.read().await.clone();
        if let Some(pool) = pool {
            let pattern = pattern_key.to_string();
            sqlx::query(
                "INSERT INTO approval_allowlist (pattern_key) VALUES (?) ON CONFLICT(pattern_key) DO NOTHING",
            )
            .bind(&pattern)
            .execute(&pool)
            .await?;
        }
        self.always_cache
            .write()
            .await
            .insert(pattern_key.to_string());
        Ok(())
    }

    pub async fn approve_permanent(&self, pattern_key: &str) -> Result<(), sqlx::Error> {
        self.approve_always(pattern_key).await
    }

    pub async fn load_permanent(&self, patterns: impl IntoIterator<Item = String>) {
        let mut always = self.always_cache.write().await;
        for pat in patterns {
            always.insert(pat);
        }
    }

    pub async fn is_yolo(&self, session: &SessionKey) -> bool {
        self.yolo_sessions
            .read()
            .await
            .contains(&session.storage_key())
    }

    pub async fn set_yolo(&self, session: &SessionKey, enabled: bool) {
        let mut yolo = self.yolo_sessions.write().await;
        let storage_key = session.storage_key();
        if enabled {
            yolo.insert(storage_key);
        } else {
            yolo.remove(&storage_key);
        }
    }

    pub async fn clear_session(&self, session: &SessionKey) {
        self.pending
            .lock()
            .retain(|_, entry| entry.session.as_ref() != Some(session));
        self.session_cache.write().await.remove(session);
        self.yolo_sessions
            .write()
            .await
            .remove(&session.storage_key());
    }

    pub async fn request(&self) -> ApprovalPrompt {
        self.request_with_session(None).await
    }

    pub async fn request_with_session(&self, session: Option<SessionKey>) -> ApprovalPrompt {
        let request_id = Uuid::new_v4();
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().insert(
            request_id,
            PendingApprovalEntry {
                session,
                sender,
                created_at: std::time::Instant::now(),
            },
        );
        ApprovalPrompt {
            request_id,
            components: approval_buttons(request_id),
            receiver,
            lease: PendingLease {
                request_id,
                pending: self.pending.clone(),
            },
        }
    }

    pub async fn resolve_custom_id(&self, custom_id: &str) -> bool {
        let Some((request_id, decision)) = parse_custom_id(custom_id) else {
            return false;
        };
        let Some(entry) = self.pending.lock().remove(&request_id) else {
            return false;
        };
        entry.sender.send(decision).is_ok()
    }

    pub async fn resolve_session_deny(&self, session: &SessionKey, reason: Option<String>) -> bool {
        let mut lock = self.pending.lock();
        let target = lock
            .iter()
            .filter(|(_, entry)| entry.session.as_ref() == Some(session))
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(id, _)| *id);

        if let Some(request_id) = target {
            if let Some(entry) = lock.remove(&request_id) {
                return entry.sender.send(ApprovalDecision::Deny { reason }).is_ok();
            }
        }
        false
    }

    pub async fn cancel(&self, request_id: Uuid) {
        self.pending.lock().remove(&request_id);
    }

    pub async fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }
}

pub fn approval_buttons(request_id: Uuid) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("omon:approval:{request_id}:once"))
            .label("Allow Once")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("omon:approval:{request_id}:session"))
            .label("Allow Session")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("omon:approval:{request_id}:always"))
            .label("Always Allow")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("omon:approval:{request_id}:deny"))
            .label("Deny")
            .style(ButtonStyle::Danger),
    ])]
}

pub fn is_approval_custom_id(custom_id: &str) -> bool {
    parse_custom_id(custom_id).is_some()
}

pub fn parse_custom_id(custom_id: &str) -> Option<(Uuid, ApprovalDecision)> {
    let mut parts = custom_id.split(':');
    if parts.next()? != "omon" || parts.next()? != "approval" {
        return None;
    }
    let request_id = Uuid::parse_str(parts.next()?).ok()?;
    let decision = match parts.next()? {
        "once" | "approve" => ApprovalDecision::Once,
        "session" => ApprovalDecision::Session,
        "always" => ApprovalDecision::Always,
        "deny" | "reject" => ApprovalDecision::Deny { reason: None },
        _ => return None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((request_id, decision))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;

    #[tokio::test]
    async fn approval_lifecycle_cleans_all_surfaces_and_isolates_bots() {
        // Given: two bot lanes and two FIFO waiters in lane A.
        let guard = SmartApprovalGuard::new();
        let a =
            SessionKey::new("discord", Some("guild"), "42", Some("43"), "user").with_bot_id("A");
        let b = a.clone().with_bot_id("B");
        let first = guard.request_with_session(Some(a.clone())).await;
        let second = guard.request_with_session(Some(a.clone())).await;
        let other = guard.request_with_session(Some(b.clone())).await;
        let ids = [first.request_id, second.request_id, other.request_id];
        // When: deny only A.
        assert!(guard.resolve_session_deny(&a, None).await);
        let remaining = {
            let pending = guard.pending.lock();
            ids.map(|id| pending.contains_key(&id))
        };
        for id in ids {
            guard.cancel(id).await;
        }
        // Then: oldest A alone was consumed; no fallback to newest B.
        println!("AP08 FIFO/bot remaining={remaining:?}");
        assert_eq!(remaining, [false, true, true]);
    }

    struct LifecycleDispatcher(tokio::sync::mpsc::UnboundedSender<OutboundAction>);

    #[async_trait]
    impl OutboundDispatcher for LifecycleDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> crate::Result<()> {
            self.0.send(action).unwrap();
            Ok(())
        }
    }

    struct HangingExpiryDispatcher(tokio::sync::mpsc::UnboundedSender<OutboundAction>);

    #[async_trait]
    impl OutboundDispatcher for HangingExpiryDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> crate::Result<()> {
            if matches!(action, OutboundAction::ExpireApproval { .. }) {
                std::future::pending::<()>().await;
            }
            self.0.send(action).unwrap();
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn decision_is_not_delayed_by_a_hanging_expiry_edit() {
        let guard = SmartApprovalGuard::new();
        let requester = DiscordApprovalRequester::new(guard.clone(), Duration::from_secs(60));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        requester
            .set_dispatcher(Arc::new(HangingExpiryDispatcher(tx)))
            .await;
        let session = SessionKey::new("discord", None::<String>, "42", None::<String>, "7");
        let request = tokio::spawn({
            let requester = requester.clone();
            async move {
                requester
                    .request_approval(&session, "rm -rf fixture", "test")
                    .await
            }
        });
        let Some(OutboundAction::ApprovalRequest { request_id, .. }) = rx.recv().await else {
            panic!("approval request expected");
        };
        assert!(
            guard
                .resolve_custom_id(&format!("omon:approval:{request_id}:once"))
                .await
        );
        // The expiry dispatch hangs for its full 10s timeout; the decision must come back sooner.
        let decision = tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .expect("decision must not wait for the expiry edit")
            .unwrap();
        assert_eq!(decision, Ok(ApprovalDecision::Once));
    }

    #[tokio::test]
    async fn approval_lifecycle_drop_cleans_pending() {
        // Given: observer subscribed before requesting.
        let guard = SmartApprovalGuard::new();
        let requester = DiscordApprovalRequester::new(guard.clone(), Duration::from_secs(60));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        requester
            .set_dispatcher(Arc::new(LifecycleDispatcher(tx)))
            .await;
        let session = SessionKey::new("discord", None::<String>, "42", None::<String>, "7");
        let mut request = Box::pin(requester.request_approval(&session, "rm -rf fixture", "test"));
        let action = tokio::select! {
            result = &mut request => panic!("unexpected completion {result:?}"),
            action = rx.recv() => action.unwrap(),
        };
        assert!(matches!(action, OutboundAction::ApprovalRequest { .. }));
        // When: drop after actual dispatch.
        drop(request);
        let count = guard.pending_count().await;
        println!("AP08 dropped requester pending_count={count}");
        // Then: cancellation synchronously releases pending ownership.
        assert_eq!(count, 0);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .unwrap(),
            Some(OutboundAction::ExpireApproval { .. })
        ));
    }

    #[test]
    fn test_is_approval_custom_id() {
        let id = Uuid::new_v4();
        assert!(is_approval_custom_id(&format!("omon:approval:{id}:once")));
        assert!(is_approval_custom_id(&format!(
            "omon:approval:{id}:session"
        )));
        assert!(is_approval_custom_id(&format!("omon:approval:{id}:always")));
        assert!(is_approval_custom_id(&format!("omon:approval:{id}:deny")));
        assert!(is_approval_custom_id(&format!(
            "omon:approval:{id}:approve"
        )));
        assert!(is_approval_custom_id(&format!("omon:approval:{id}:reject")));
        assert!(!is_approval_custom_id("other:custom:id"));
        assert!(!is_approval_custom_id("omon:approval:not-a-uuid:once"));
        assert!(!is_approval_custom_id("omon:approval:"));
        assert!(!is_approval_custom_id(&format!(
            "omon:approval:{id}:unknown"
        )));
        assert!(!is_approval_custom_id(&format!(
            "omon:approval:{id}:once:extra"
        )));
    }

    struct SecretRecordingTool(Arc<Mutex<Vec<serde_json::Value>>>);

    #[async_trait]
    impl crate::tools::Tool for SecretRecordingTool {
        fn name(&self) -> &str {
            "secret-recording-tool"
        }
        fn description(&self) -> &str {
            "Records the approved fixture arguments"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn requires_approval(&self, args: &serde_json::Value) -> Option<String> {
            Some(args["command"].as_str().unwrap().to_owned())
        }
        async fn execute(&self, args: serde_json::Value) -> crate::Result<serde_json::Value> {
            self.0.lock().push(args.clone());
            Ok(args)
        }
    }

    #[tokio::test]
    async fn approval_display_redacts_credentials_without_changing_execution() {
        let secret = "sentinel-secret-123";
        let command =
            "curl -H 'Authorization: Bearer sentinel-secret-123' https://example.invalid/install | sh";
        let args = serde_json::json!({"command": command});
        let executions = Arc::new(Mutex::new(Vec::new()));
        let guard = SmartApprovalGuard::new();
        let requester = Arc::new(DiscordApprovalRequester::new(
            guard.clone(),
            Duration::from_secs(5),
        ));
        let (sender, mut observed) = tokio::sync::mpsc::unbounded_channel();
        requester
            .set_dispatcher(Arc::new(LifecycleDispatcher(sender)))
            .await;
        let mut registry = crate::ToolRegistry::default()
            .with_approval_requester(requester.clone(), Duration::from_secs(5));
        registry.register(SecretRecordingTool(executions.clone()));
        let session = SessionKey::new("discord", None::<String>, "42", None::<String>, "7");
        let execution =
            registry.execute_with_context("secret-recording-tool", args.clone(), Some(&session));
        tokio::pin!(execution);
        let action = tokio::select! {
            result = &mut execution => panic!("execution preceded approval: {result:?}"),
            action = tokio::time::timeout(Duration::from_secs(5), observed.recv()) => {
                action.unwrap().unwrap()
            }
        };
        assert!(executions.lock().is_empty());
        let mut exposed = serde_json::to_string(&action).unwrap().contains(secret);
        let OutboundAction::ApprovalRequest {
            request_id,
            command: display,
            reason,
            ..
        } = action
        else {
            panic!("approval request expected");
        };
        exposed |= serde_json::to_string(&crate::discord::adapter::build_approval_embed(
            &display, &reason,
        ))
        .unwrap()
        .contains(secret);
        assert!(
            guard
                .resolve_custom_id(&format!("omon:approval:{request_id}:once"))
                .await
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), &mut execution)
                .await
                .unwrap()
                .unwrap(),
            args
        );
        assert_eq!(*executions.lock(), vec![args]);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(5), observed.recv())
                .await
                .unwrap(),
            Some(OutboundAction::ExpireApproval { .. })
        ));

        let direct = requester.request_approval(&session, command, command);
        tokio::pin!(direct);
        let action = tokio::select! {
            result = &mut direct => panic!("approval unexpectedly completed: {result:?}"),
            action = tokio::time::timeout(Duration::from_secs(5), observed.recv()) => {
                action.unwrap().unwrap()
            }
        };
        exposed |= serde_json::to_string(&action).unwrap().contains(secret);
        let OutboundAction::ApprovalRequest { request_id, .. } = action else {
            panic!("approval request expected");
        };
        assert!(
            guard
                .resolve_custom_id(&format!("omon:approval:{request_id}:once"))
                .await
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), &mut direct)
                .await
                .unwrap(),
            Ok(ApprovalDecision::Once)
        );
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(5), observed.recv())
                .await
                .unwrap(),
            Some(OutboundAction::ExpireApproval { .. })
        ));
        assert_eq!(guard.pending_count().await, 0);
        println!("U04 secret_exposed={exposed} raw_args_equal=true executions=1 pending=0");
        assert!(!exposed);
    }

    #[test]
    fn test_parse_custom_id_round_trip() {
        let id = Uuid::new_v4();
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:once")),
            Some((id, ApprovalDecision::Once))
        );
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:session")),
            Some((id, ApprovalDecision::Session))
        );
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:always")),
            Some((id, ApprovalDecision::Always))
        );
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:deny")),
            Some((id, ApprovalDecision::Deny { reason: None }))
        );
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:approve")),
            Some((id, ApprovalDecision::Once))
        );
        assert_eq!(
            parse_custom_id(&format!("omon:approval:{id}:reject")),
            Some((id, ApprovalDecision::Deny { reason: None }))
        );
    }

    #[tokio::test]
    async fn test_resolve_custom_id_unknown_uuid() {
        let guard = SmartApprovalGuard::new();
        let unknown_id = Uuid::new_v4();
        let custom_id = format!("omon:approval:{unknown_id}:once");
        assert!(!guard.resolve_custom_id(&custom_id).await);
    }

    struct MockDispatcher;

    #[async_trait]
    impl OutboundDispatcher for MockDispatcher {
        async fn dispatch(&self, _action: OutboundAction) -> crate::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_session_approval_cache_suppresses_repeat_prompts() {
        let guard = SmartApprovalGuard::new();
        let requester = DiscordApprovalRequester::new(guard.clone(), Duration::from_millis(50));
        requester.set_dispatcher(Arc::new(MockDispatcher)).await;

        let session_a =
            SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");
        let session_b =
            SessionKey::new("discord", None::<String>, "chan2", None::<String>, "user2");

        let cmd = "rm -rf /tmp/build";
        let pattern = crate::security::derive_pattern_key(cmd);
        assert!(!guard.is_approved(&session_a, &pattern).await);
        assert!(!guard.is_approved(&session_b, &pattern).await);

        // Approve session A
        guard.approve_session(&session_a, &pattern).await;

        assert!(guard.is_approved(&session_a, &pattern).await);
        assert!(!guard.is_approved(&session_b, &pattern).await);

        // requester immediately auto-approves session A without prompting
        let decision = requester
            .request_approval(&session_a, cmd, "recursive delete")
            .await
            .unwrap();
        assert_eq!(decision, ApprovalDecision::Session);

        // session B is not cached and times out
        let err = requester
            .request_approval(&session_b, cmd, "recursive delete")
            .await
            .unwrap_err();
        assert_eq!(err, ApprovalError::Timeout);
    }

    #[tokio::test]
    async fn test_always_approval_cache_applies_globally() {
        let guard = SmartApprovalGuard::new();
        let session_a =
            SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");
        let session_b =
            SessionKey::new("discord", None::<String>, "chan2", None::<String>, "user2");

        let pattern = "disk copy";
        assert!(!guard.is_approved(&session_a, pattern).await);
        assert!(!guard.is_approved(&session_b, pattern).await);

        guard.approve_always(pattern).await.unwrap();

        assert!(guard.is_approved(&session_a, pattern).await);
        assert!(guard.is_approved(&session_b, pattern).await);
    }

    #[tokio::test]
    async fn test_resolve_session_deny_with_reason() {
        let guard = SmartApprovalGuard::new();
        let session = SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");

        let prompt = guard.request_with_session(Some(session.clone())).await;

        let reason = Some("unsafe directory operation".to_string());
        assert!(guard.resolve_session_deny(&session, reason.clone()).await);

        let decision = prompt.wait(Duration::from_millis(50)).await.unwrap();
        assert_eq!(decision, ApprovalDecision::Deny { reason });

        // Subsequent resolve fails
        assert!(!guard.resolve_session_deny(&session, None).await);
    }

    #[tokio::test]
    async fn test_yolo_toggle_and_auto_approval() {
        let guard = SmartApprovalGuard::new();
        let requester = DiscordApprovalRequester::new(guard.clone(), Duration::from_millis(50));
        requester.set_dispatcher(Arc::new(MockDispatcher)).await;

        let session_a =
            SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");
        let session_b =
            SessionKey::new("discord", None::<String>, "chan2", None::<String>, "user2");

        assert!(!guard.is_yolo(&session_a).await);
        assert!(!guard.is_yolo(&session_b).await);
        assert!(!requester.is_yolo(&session_a).await);

        guard.set_yolo(&session_a, true).await;
        assert!(guard.is_yolo(&session_a).await);
        assert!(requester.is_yolo(&session_a).await);
        assert!(!guard.is_yolo(&session_b).await);

        // Session A auto-approves via request_approval
        let decision = requester
            .request_approval(&session_a, "rm -rf /tmp/scratch", "recursive delete")
            .await
            .unwrap();
        assert!(decision.is_approved());

        // Session B still times out
        let err = requester
            .request_approval(&session_b, "rm -rf /tmp/scratch", "recursive delete")
            .await
            .unwrap_err();
        assert_eq!(err, ApprovalError::Timeout);

        // Disable YOLO
        guard.set_yolo(&session_a, false).await;
        assert!(!guard.is_yolo(&session_a).await);

        // Clear session clears YOLO
        guard.set_yolo(&session_a, true).await;
        assert!(guard.is_yolo(&session_a).await);
        guard.clear_session(&session_a).await;
        assert!(!guard.is_yolo(&session_a).await);
    }

    #[tokio::test]
    async fn test_wait_with_heartbeat_fires_periodically_and_stops_on_resolve() {
        let guard = SmartApprovalGuard::new();
        let prompt = guard.request().await;
        let request_id = prompt.request_id;

        let heartbeat_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let hb = heartbeat_count.clone();
        let (beat_tx, mut beat_rx) = tokio::sync::mpsc::unbounded_channel();

        let wait_handle = tokio::spawn(async move {
            prompt
                .wait_with_heartbeat(
                    Duration::from_millis(200),
                    Duration::from_millis(10),
                    move || {
                        hb.fetch_add(1, Ordering::SeqCst);
                        let _ = beat_tx.send(());
                    },
                )
                .await
        });

        // Causally wait for exactly 2 heartbeats to fire without wall-clock sleep
        beat_rx.recv().await.expect("first heartbeat");
        beat_rx.recv().await.expect("second heartbeat");

        let custom_id = format!("omon:approval:{request_id}:once");
        assert!(guard.resolve_custom_id(&custom_id).await);

        let result = wait_handle.await.unwrap();
        assert_eq!(result, Ok(ApprovalDecision::Once));

        let count_after_resolve = heartbeat_count.load(Ordering::SeqCst);
        assert!(count_after_resolve >= 2);

        // Verify heartbeat stopped: since wait_handle resolved and joined, no further beats arrive
        assert!(
            beat_rx.try_recv().is_err(),
            "heartbeat must stop firing after resolve"
        );
    }

    #[tokio::test]
    async fn test_requester_heartbeat_invoked_during_approval_wait() {
        let guard = SmartApprovalGuard::new();
        let heartbeat_sessions = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let hb_sessions = heartbeat_sessions.clone();

        let requester = DiscordApprovalRequester::new(guard.clone(), Duration::from_millis(300))
            .with_heartbeat_interval(Duration::from_millis(20))
            .with_heartbeat(Arc::new(move |session: &SessionKey| {
                hb_sessions.lock().push(session.clone());
            }));
        requester.set_dispatcher(Arc::new(MockDispatcher)).await;

        let session = SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");

        let err = requester
            .request_approval(&session, "rm -rf /tmp/danger", "danger")
            .await
            .unwrap_err();
        assert_eq!(err, ApprovalError::Timeout);

        let sessions = heartbeat_sessions.lock().clone();
        assert!(
            !sessions.is_empty(),
            "expected heartbeat callback to be called"
        );
        assert_eq!(sessions[0], session);
    }

    #[tokio::test]
    async fn test_permanent_allowlist_persistence_roundtrip_and_auto_approval() {
        let db = crate::Database::connect("sqlite::memory:").await.unwrap();

        // 1. Initial guard with DB pool - approve a pattern as Always
        let guard_1 = SmartApprovalGuard::new().with_pool(db.pool().clone());
        let cmd = "rm -rf /tmp/build_cache";
        let pattern_key = crate::security::derive_pattern_key(cmd);

        let session = SessionKey::new("discord", None::<String>, "chan1", None::<String>, "user1");

        assert!(!guard_1.is_approved(&session, &pattern_key).await);

        guard_1.approve_always(&pattern_key).await.unwrap();
        assert!(guard_1.is_approved(&session, &pattern_key).await);

        // 2. Verify row exists in DB
        let db_rows: Vec<(String,)> = sqlx::query_as("SELECT pattern_key FROM approval_allowlist")
            .fetch_all(db.pool())
            .await
            .unwrap();
        assert_eq!(db_rows.len(), 1);
        assert_eq!(db_rows[0].0, pattern_key);

        // 3. Simulate process restart: create fresh guard with the same pool and load allowlist
        let guard_2 = SmartApprovalGuard::new().with_pool(db.pool().clone());
        assert!(!guard_2.is_approved(&session, &pattern_key).await);

        let loaded = guard_2.load_persisted_allowlist().await.unwrap();
        assert_eq!(loaded, 1);
        assert!(guard_2.is_approved(&session, &pattern_key).await);

        // 4. Requester using restarted guard immediately auto-approves without prompting
        let requester = DiscordApprovalRequester::new(guard_2, Duration::from_millis(50));
        requester.set_dispatcher(Arc::new(MockDispatcher)).await;

        let decision = requester
            .request_approval(&session, cmd, "recursive delete")
            .await
            .unwrap();
        assert_eq!(decision, ApprovalDecision::Session);
    }

    #[tokio::test]
    async fn test_yolo_persistence_roundtrip_and_restart() {
        let db = crate::Database::connect("sqlite::memory:").await.unwrap();
        let session = SessionKey::new("discord", Some("guild1"), "chan1", None::<String>, "user1");

        let state = crate::SessionState {
            yolo: true,
            ..Default::default()
        };
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(session.storage_key())
        .bind(&session.platform)
        .bind(&session.guild_id)
        .bind(&session.channel_id)
        .bind(&session.user_id)
        .bind(serde_json::to_string(&state).unwrap())
        .execute(db.pool())
        .await
        .unwrap();

        let guard = SmartApprovalGuard::new().with_pool(db.pool().clone());
        assert!(!guard.is_yolo(&session).await);

        let loaded = guard.load_persisted_yolo().await.unwrap();
        assert_eq!(loaded, 1);
        assert!(guard.is_yolo(&session).await);

        let requester = DiscordApprovalRequester::new(guard, Duration::from_millis(50));
        let decision = requester
            .request_approval(&session, "echo hello", "harmless")
            .await
            .unwrap();
        assert_eq!(decision, ApprovalDecision::Once);
        assert!(decision.is_approved());
    }

    #[tokio::test]
    async fn test_twin_bot_same_dm_yolo_restoration() {
        let db = crate::Database::connect("sqlite::memory:").await.unwrap();
        let bot84 = SessionKey::new("discord", None::<String>, "dm1", None::<String>, "user1")
            .with_bot_id("84");
        let bot42 = SessionKey::new("discord", None::<String>, "dm1", None::<String>, "user1")
            .with_bot_id("42");
        let botless = SessionKey::new("discord", None::<String>, "dm1", None::<String>, "user1");

        let state = crate::SessionState {
            yolo: true,
            ..Default::default()
        };
        sqlx::query(
            "INSERT INTO sessions (session_key, platform, guild_id, channel_id, user_id, state_json)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(bot84.storage_key())
        .bind(&bot84.platform)
        .bind(&bot84.guild_id)
        .bind(&bot84.channel_id)
        .bind(&bot84.user_id)
        .bind(serde_json::to_string(&state).unwrap())
        .execute(db.pool())
        .await
        .unwrap();

        let guard = SmartApprovalGuard::new().with_pool(db.pool().clone());
        let loaded = guard.load_persisted_yolo().await.unwrap();
        assert_eq!(loaded, 1);
        assert!(guard.is_yolo(&bot84).await, "bot84 must be yolo");
        assert!(!guard.is_yolo(&bot42).await, "bot42 must not be yolo");
        assert!(
            !guard.is_yolo(&botless).await,
            "botless alias must not be yolo"
        );
    }

    #[tokio::test]
    async fn approve_always_persists_before_publishing_and_fails_closed() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = SmartApprovalGuard::new().with_pool(pool.clone());
        let pattern = "terminal:cargo:*";

        // Success path: durable row exists AND cache published.
        store.approve_always(pattern).await.unwrap();
        assert!(store.always_cache.read().await.contains(pattern));
        let persisted: Option<(String,)> =
            sqlx::query_as("SELECT pattern_key FROM approval_allowlist WHERE pattern_key = ?")
                .bind(pattern)
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert_eq!(persisted.as_ref().map(|(p,)| p.as_str()), Some(pattern));

        // Failure path: a closed pool makes the INSERT fail; the grant must
        // NOT be published and the error must surface instead of a warn.
        let failing = SmartApprovalGuard::new().with_pool(pool.clone());
        pool.close().await;
        let err = failing.approve_always("terminal:rm:*").await;
        assert!(err.is_err(), "closed pool must surface a persistence error");
        assert!(
            !failing.always_cache.read().await.contains("terminal:rm:*"),
            "a failed persistence attempt must not leave an active grant"
        );
    }

    #[tokio::test]
    async fn approval_caches_are_bounded() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let store = SmartApprovalGuard::new().with_pool(pool);
        let session = SessionKey::new("discord", None::<String>, "42", None::<String>, "7");

        for i in 0..(SmartApprovalGuard::MAX_PATTERNS_PER_SESSION + 20) {
            store
                .approve_session(&session, &format!("cmd:pattern-{i}"))
                .await;
        }
        {
            let cache = store.session_cache.read().await;
            let patterns = cache.get(&session).unwrap();
            assert!(
                patterns.len() <= SmartApprovalGuard::MAX_PATTERNS_PER_SESSION,
                "per-session pattern cache must be bounded, got {}",
                patterns.len()
            );
        }

        for i in 0..(SmartApprovalGuard::MAX_SESSIONS + 20) {
            let s = SessionKey::new(
                "discord",
                None::<String>,
                format!("chan-{i}"),
                None::<String>,
                "user",
            );
            store.approve_session(&s, "cmd:x").await;
        }
        {
            let cache = store.session_cache.read().await;
            assert!(
                cache.len() <= SmartApprovalGuard::MAX_SESSIONS,
                "session cache must be bounded, got {}",
                cache.len()
            );
        }
    }
}

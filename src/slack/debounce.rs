//! Split-message debouncer for Slack ingress.
//!
//! Slack clients split long pastes and quick follow-ups into several `message` events, and the
//! runtime handles every envelope on its own task, so a burst can reach ingress out of order.
//! Events for one session are buffered for a short window, sorted by Slack `ts`, coalesced into
//! one turn and handed to a [`FlushSink`] together with every constituent delivery id so the
//! ledger can claim them all (`record_incoming_with_constituents`). Flushes for one session are
//! serialized, so a later batch never overtakes an earlier one that is still routing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::ingress::ts_key;
use crate::{InboundEvent, Result, SessionKey};

/// Default window for coalescing client-split messages, same as Discord.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(600);

/// Receives one coalesced event plus the delivery ids of every message folded into it.
#[async_trait]
pub trait FlushSink: Send + Sync {
    async fn flush(&self, event: InboundEvent, constituents: Vec<String>) -> Result<()>;
}

struct Batch {
    events: Vec<InboundEvent>,
    token: u64,
}

#[derive(Clone)]
pub struct SlackDebouncer {
    duration: Duration,
    next_token: Arc<AtomicU64>,
    buffer: Arc<Mutex<HashMap<SessionKey, Batch>>>,
    /// Per-session flush locks. Tokio mutexes hand the lock out in request order, which keeps
    /// batches of one session in arrival order.
    lanes: Arc<Mutex<HashMap<SessionKey, Arc<Mutex<()>>>>>,
    sink: Arc<dyn FlushSink>,
}

impl SlackDebouncer {
    pub fn new(duration: Duration, sink: Arc<dyn FlushSink>) -> Self {
        Self {
            duration,
            next_token: Arc::new(AtomicU64::new(1)),
            buffer: Arc::new(Mutex::new(HashMap::new())),
            lanes: Arc::new(Mutex::new(HashMap::new())),
            sink,
        }
    }

    /// Buffers the event and (re)starts the session's window. The batch flushes once no new
    /// event arrived for `duration`.
    pub async fn enqueue(&self, event: InboundEvent) {
        let session = event.session.clone();
        let token = self.next_token.fetch_add(1, Ordering::SeqCst);
        {
            let mut buffer = self.buffer.lock().await;
            let batch = buffer.entry(session.clone()).or_insert_with(|| Batch {
                events: Vec::new(),
                token,
            });
            batch.events.push(event);
            batch.token = token;
        }
        let this = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(this.duration).await;
            let lane = this.lane(&session).await;
            let ordered = lane.lock().await;
            let events = {
                let mut buffer = this.buffer.lock().await;
                let current = buffer
                    .get(&session)
                    .is_some_and(|batch| batch.token == token);
                if current {
                    buffer.remove(&session).map(|batch| batch.events)
                } else {
                    None
                }
            };
            if let Some(events) = events {
                this.flush_events(events).await;
            }
            drop(ordered);
            this.release_lane(&session, lane).await;
        });
    }

    /// Drops the session's pending batch (text `/stop`). Returns what was dropped.
    pub async fn cancel(&self, session: &SessionKey) -> Option<Vec<InboundEvent>> {
        self.buffer
            .lock()
            .await
            .remove(session)
            .map(|batch| batch.events)
    }

    /// Routes every pending batch now, for shutdown and drain, so nothing buffered is lost.
    pub async fn flush_all(&self) {
        let batches: Vec<(SessionKey, Vec<InboundEvent>)> = {
            let mut buffer = self.buffer.lock().await;
            buffer
                .drain()
                .map(|(key, batch)| (key, batch.events))
                .collect()
        };
        for (session, events) in batches {
            let lane = self.lane(&session).await;
            let ordered = lane.lock().await;
            self.flush_events(events).await;
            drop(ordered);
            self.release_lane(&session, lane).await;
        }
    }

    pub async fn is_empty(&self) -> bool {
        self.buffer.lock().await.is_empty()
    }

    /// True while a buffered, not yet routed event carries this delivery id.
    pub async fn is_pending(&self, delivery_id: &str) -> bool {
        self.buffer.lock().await.values().any(|batch| {
            batch
                .events
                .iter()
                .any(|event| event.delivery_id.as_deref() == Some(delivery_id))
        })
    }

    async fn lane(&self, session: &SessionKey) -> Arc<Mutex<()>> {
        self.lanes
            .lock()
            .await
            .entry(session.clone())
            .or_default()
            .clone()
    }

    /// Drops the session's flush lock from the map once no other task holds or waits on it, so
    /// the map does not grow with every session ever seen. Holding the map lock while checking
    /// the count means nobody can clone the entry in between.
    async fn release_lane(&self, session: &SessionKey, lane: Arc<Mutex<()>>) {
        let mut lanes = self.lanes.lock().await;
        // One reference in the map plus the caller's `lane`.
        if Arc::strong_count(&lane) == 2 {
            lanes.remove(session);
        }
    }

    async fn flush_events(&self, events: Vec<InboundEvent>) {
        let Some((event, constituents)) = coalesce(events) else {
            return;
        };
        let session = event.session.clone();
        tracing::info!(
            session = %session,
            delivery_id = ?event.delivery_id,
            constituents = constituents.len(),
            "flushing debounced Slack message"
        );
        if let Err(error) = self.sink.flush(event, constituents).await {
            tracing::error!(session = %session, %error, "failed to route debounced Slack event");
        }
    }
}

/// Sorts a batch by Slack `ts`, collects unique constituent delivery ids in that order and
/// merges the events (content joined by newlines, attachments unioned, last id wins).
pub fn coalesce(mut events: Vec<InboundEvent>) -> Option<(InboundEvent, Vec<String>)> {
    events.sort_by_key(|event| ts_key(&event.platform_message_id));
    let mut constituents = Vec::new();
    for event in &events {
        let id = event.delivery_id.clone().unwrap_or_else(|| {
            format!(
                "slack:{}:{}",
                event.session.channel_id, event.platform_message_id
            )
        });
        if !constituents.contains(&id) {
            constituents.push(id);
        }
    }
    let event = crate::discord::adapter::coalesce_inbound_events(events)?;
    Some((event, constituents))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Collect(tokio::sync::mpsc::UnboundedSender<(InboundEvent, Vec<String>)>);

    #[async_trait]
    impl FlushSink for Collect {
        async fn flush(&self, event: InboundEvent, constituents: Vec<String>) -> Result<()> {
            let _ = self.0.send((event, constituents));
            Ok(())
        }
    }

    fn key(channel: &str) -> SessionKey {
        SessionKey::new("slack", None::<String>, channel, None::<String>, "U1").with_bot_id("UBOT")
    }

    fn event(channel: &str, ts: &str, text: &str) -> InboundEvent {
        InboundEvent::message(key(channel), ts, text)
            .with_delivery_id(format!("slack:{channel}:{ts}"))
            .with_author_id("U1")
    }

    fn debouncer(
        duration: Duration,
    ) -> (
        SlackDebouncer,
        tokio::sync::mpsc::UnboundedReceiver<(InboundEvent, Vec<String>)>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (SlackDebouncer::new(duration, Arc::new(Collect(tx))), rx)
    }

    #[tokio::test(start_paused = true)]
    async fn burst_is_coalesced_in_ts_order_with_all_constituents() {
        let (debouncer, mut rx) = debouncer(Duration::from_millis(50));
        debouncer
            .enqueue(event("D1", "1700000000.000300", "third"))
            .await;
        debouncer
            .enqueue(event("D1", "1700000000.000100", "first"))
            .await;
        debouncer
            .enqueue(event("D1", "1700000000.000200", "second"))
            .await;
        debouncer
            .enqueue(event("D1", "1700000000.000200", "second"))
            .await;

        let (merged, constituents) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(merged.content, "first\nsecond\nthird");
        assert_eq!(merged.platform_message_id, "1700000000.000300");
        assert_eq!(
            merged.delivery_id.as_deref(),
            Some("slack:D1:1700000000.000300")
        );
        assert_eq!(merged.author_id.as_deref(), Some("U1"));
        assert_eq!(
            constituents,
            vec![
                "slack:D1:1700000000.000100".to_owned(),
                "slack:D1:1700000000.000200".to_owned(),
                "slack:D1:1700000000.000300".to_owned(),
            ]
        );
        assert!(debouncer.is_empty().await);
    }

    #[tokio::test(start_paused = true)]
    async fn sessions_flush_independently() {
        let (debouncer, mut rx) = debouncer(Duration::from_millis(20));
        debouncer.enqueue(event("D1", "1.000001", "a")).await;
        debouncer.enqueue(event("D2", "1.000002", "b")).await;
        let mut got = Vec::new();
        for _ in 0..2 {
            let (merged, _) = tokio::time::timeout(Duration::from_secs(5), rx.recv())
                .await
                .unwrap()
                .unwrap();
            got.push((merged.session.channel_id, merged.content));
        }
        got.sort();
        assert_eq!(
            got,
            vec![("D1".into(), "a".into()), ("D2".into(), "b".into())]
        );
    }

    #[tokio::test]
    async fn cancel_drops_pending_batch_and_flush_all_routes_the_rest() {
        let (debouncer, mut rx) = debouncer(Duration::from_secs(3600));
        debouncer.enqueue(event("D1", "1.000001", "drop me")).await;
        debouncer.enqueue(event("D2", "1.000002", "keep me")).await;
        assert!(debouncer.is_pending("slack:D1:1.000001").await);
        let dropped = debouncer.cancel(&key("D1")).await.unwrap();
        assert!(!debouncer.is_pending("slack:D1:1.000001").await);
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].content, "drop me");

        debouncer.flush_all().await;
        let (merged, constituents) = rx.try_recv().expect("flush_all routes synchronously");
        assert_eq!(merged.content, "keep me");
        assert_eq!(constituents, vec!["slack:D2:1.000002".to_owned()]);
        assert!(rx.try_recv().is_err(), "cancelled batch must not be routed");
        assert!(debouncer.is_empty().await);
        assert!(debouncer.lanes.lock().await.is_empty());
    }
}

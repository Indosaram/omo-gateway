use std::future::Future;
use std::time::Duration;

use sqlx::SqlitePool;

use super::{collect_all, prefilter::PrefilterRules, InboxSource, InboxSourceKind};
use crate::Result;

pub const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InboxWatcherEvent {
    Startup,
    GatewayMessage,
    IMessage { rowid: Option<i64> },
    Mail,
    Periodic,
}

pub type InboxEventSender = tokio::sync::mpsc::Sender<InboxWatcherEvent>;
pub type InboxEventReceiver = tokio::sync::mpsc::Receiver<InboxWatcherEvent>;

pub fn event_channel(capacity: usize) -> (InboxEventSender, InboxEventReceiver) {
    tokio::sync::mpsc::channel(capacity)
}

pub async fn on_gateway_message(
    sender: &InboxEventSender,
) -> std::result::Result<(), tokio::sync::mpsc::error::SendError<InboxWatcherEvent>> {
    sender.send(InboxWatcherEvent::GatewayMessage).await
}

pub fn try_on_gateway_message(
    sender: &InboxEventSender,
) -> std::result::Result<(), tokio::sync::mpsc::error::TrySendError<InboxWatcherEvent>> {
    sender.try_send(InboxWatcherEvent::GatewayMessage)
}

pub async fn on_imessage_event(
    sender: &InboxEventSender,
    rowid: Option<i64>,
) -> std::result::Result<(), tokio::sync::mpsc::error::SendError<InboxWatcherEvent>> {
    sender.send(InboxWatcherEvent::IMessage { rowid }).await
}

pub async fn on_mail_event(
    sender: &InboxEventSender,
) -> std::result::Result<(), tokio::sync::mpsc::error::SendError<InboxWatcherEvent>> {
    sender.send(InboxWatcherEvent::Mail).await
}

pub async fn on_periodic(
    sender: &InboxEventSender,
) -> std::result::Result<(), tokio::sync::mpsc::error::SendError<InboxWatcherEvent>> {
    sender.send(InboxWatcherEvent::Periodic).await
}

pub fn filter_sources_for_event<'a>(
    sources: &[&'a dyn InboxSource],
    event: &InboxWatcherEvent,
) -> Vec<&'a dyn InboxSource> {
    let matched: Vec<&'a dyn InboxSource> = match event {
        InboxWatcherEvent::Startup => sources.to_vec(),
        InboxWatcherEvent::GatewayMessage => sources
            .iter()
            .copied()
            .filter(|s| s.name() == InboxSourceKind::Discord)
            .collect(),
        InboxWatcherEvent::IMessage { .. } => sources
            .iter()
            .copied()
            .filter(|s| s.name() == InboxSourceKind::Imessage)
            .collect(),
        InboxWatcherEvent::Mail => sources
            .iter()
            .copied()
            .filter(|s| s.name() == InboxSourceKind::Mail)
            .collect(),
        InboxWatcherEvent::Periodic => sources
            .iter()
            .copied()
            .filter(|s| s.name() == InboxSourceKind::Kakao)
            .collect(),
    };
    if matched.is_empty() {
        sources.to_vec()
    } else {
        matched
    }
}

pub async fn process_event(
    pool: &SqlitePool,
    sources: &[&dyn InboxSource],
    rules: &PrefilterRules,
    event: &InboxWatcherEvent,
) -> Vec<super::SourceCollectResult> {
    let target_sources = filter_sources_for_event(sources, event);
    collect_all(pool, &target_sources, rules).await
}

pub async fn run_event_watch(
    pool: SqlitePool,
    sources: Vec<Box<dyn InboxSource>>,
    rules: PrefilterRules,
    event_rx: InboxEventReceiver,
    progress_tx: Option<tokio::sync::mpsc::Sender<Vec<super::SourceCollectResult>>>,
) {
    run_event_watch_with_shutdown(pool, sources, rules, event_rx, progress_tx, None).await;
}

pub async fn run_event_watch_with_shutdown(
    pool: SqlitePool,
    sources: Vec<Box<dyn InboxSource>>,
    rules: PrefilterRules,
    mut event_rx: InboxEventReceiver,
    progress_tx: Option<tokio::sync::mpsc::Sender<Vec<super::SourceCollectResult>>>,
    mut shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    let borrowed: Vec<&dyn InboxSource> = sources.iter().map(Box::as_ref).collect();
    loop {
        tokio::select! {
            biased;
            Some(true) = async {
                match shutdown_rx.as_mut() {
                    Some(rx) => {
                        if *rx.borrow() {
                            Some(true)
                        } else {
                            rx.changed().await.ok().map(|_| *rx.borrow())
                        }
                    }
                    None => std::future::pending::<Option<bool>>().await,
                }
            } => {
                break;
            }
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        let results = process_event(&pool, &borrowed, &rules, &event).await;
                        if let Some(ref tx) = progress_tx {
                            let _ = tx.send(results).await;
                        }
                    }
                    None => break,
                }
            }
        }
    }
}

/// Strongly typed cursor metadata for iMessage collection vs watcher state.
///
/// `IMessageSource` uses an RFC3339 timestamp cursor in `inbox_cursors.cursor`.
/// The `imsg watch` child process, by contrast, uses a numeric rowid cursor (`--since-rowid <rowid>`).
/// These two cursors represent different data types and MUST NOT be silently mixed or overwritten.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IMessageCursorState {
    /// RFC3339 timestamp used by `IMessageSource::fetch_since`.
    pub source_timestamp: Option<String>,
    /// Numeric rowid used by `imsg watch --since-rowid`.
    pub watcher_rowid: Option<i64>,
}

pub async fn read_imessage_source_cursor(pool: &SqlitePool) -> Result<Option<String>> {
    let cursor = sqlx::query_scalar::<_, String>(
        "SELECT cursor FROM inbox_cursors WHERE source = 'imessage'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(cursor)
}

pub async fn latest_imessage_rowid(pool: &SqlitePool) -> Result<Option<i64>> {
    let rowids = sqlx::query_scalar::<_, String>(
        "SELECT source_ref FROM inbox_items WHERE source = 'imessage' AND source_ref GLOB '[0-9]*'",
    )
    .fetch_all(pool)
    .await?;
    let max_rowid = rowids.iter().filter_map(|s| s.parse::<i64>().ok()).max();
    Ok(max_rowid)
}

pub fn imsg_watch_args(since_rowid: Option<i64>) -> Vec<String> {
    let mut args = vec!["watch".into(), "--json".into()];
    if let Some(rowid) = since_rowid {
        args.push("--since-rowid".into());
        args.push(rowid.to_string());
    }
    args
}

pub fn parse_imsg_watch_line(line: &str) -> Option<i64> {
    let parsed: serde_json::Value = serde_json::from_str(line).ok()?;
    parsed
        .get("id")
        .or_else(|| parsed.get("rowid"))
        .and_then(|v| {
            if let Some(n) = v.as_i64() {
                Some(n)
            } else if let Some(s) = v.as_str() {
                s.parse::<i64>().ok()
            } else {
                None
            }
        })
}

pub async fn collect_once(
    pool: &SqlitePool,
    sources: &[&dyn InboxSource],
    rules: &PrefilterRules,
) -> Vec<super::SourceCollectResult> {
    collect_all(pool, sources, rules).await
}

pub async fn run_cursor_watch<F, Fut>(
    pool: SqlitePool,
    sources: Vec<Box<dyn InboxSource>>,
    rules: PrefilterRules,
    interval: Duration,
    mut wait_for_change: F,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    let borrowed: Vec<&dyn InboxSource> = sources.iter().map(Box::as_ref).collect();
    loop {
        wait_for_change().await;
        collect_all(&pool, &borrowed, &rules).await;
        tokio::time::sleep(interval).await;
    }
}

pub async fn run_imessage_watch<F, Fut>(
    pool: SqlitePool,
    source: Box<dyn InboxSource>,
    rules: PrefilterRules,
    mut next_event: F,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    let sources: [&dyn InboxSource; 1] = [source.as_ref()];
    loop {
        next_event().await;
        collect_all(&pool, &sources, &rules).await;
    }
}

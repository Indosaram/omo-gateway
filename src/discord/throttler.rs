use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serenity::all::{ChannelId, CreateMessage, EditMessage, Http, MessageId};
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::adapter::safe_allowed_mentions;
use crate::Result;

pub const DISCORD_MESSAGE_LIMIT: usize = 2_000;
pub const MAX_SPLIT_MESSAGES: usize = 8;
pub const TRUNCATION_NOTICE: &str =
    "\n\n… [Response truncated: output exceeded Discord 8-message limit]";
const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(800);

#[async_trait]
pub trait DiscordMessageTransport: Send + Sync + 'static {
    async fn start_typing(&self, channel_id: ChannelId) -> Result<()>;
    async fn edit_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        content: String,
    ) -> Result<()>;
    async fn send_message(&self, channel_id: ChannelId, content: String) -> Result<MessageId>;
    async fn send_message_with_reference(
        &self,
        channel_id: ChannelId,
        content: String,
        reference: Option<MessageId>,
    ) -> Result<MessageId> {
        let _ = reference;
        self.send_message(channel_id, content).await
    }
    async fn delete_message(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()>;
}

#[derive(Clone)]
pub struct SerenityMessageTransport {
    http: Arc<Http>,
}

impl SerenityMessageTransport {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

#[async_trait]
impl DiscordMessageTransport for SerenityMessageTransport {
    async fn start_typing(&self, channel_id: ChannelId) -> Result<()> {
        self.http.broadcast_typing(channel_id).await?;
        Ok(())
    }

    async fn edit_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        content: String,
    ) -> Result<()> {
        channel_id
            .edit_message(
                &self.http,
                message_id,
                EditMessage::new()
                    .content(content)
                    .allowed_mentions(safe_allowed_mentions()),
            )
            .await?;
        Ok(())
    }

    async fn send_message(&self, channel_id: ChannelId, content: String) -> Result<MessageId> {
        self.send_message_with_reference(channel_id, content, None)
            .await
    }

    async fn send_message_with_reference(
        &self,
        channel_id: ChannelId,
        content: String,
        reference: Option<MessageId>,
    ) -> Result<MessageId> {
        if let Some(target_msg_id) = reference {
            let builder = CreateMessage::new()
                .content(content.clone())
                .reference_message((channel_id, target_msg_id))
                .allowed_mentions(safe_allowed_mentions());
            match channel_id.send_message(&self.http, builder).await {
                Ok(msg) => return Ok(msg.id),
                Err(error) => {
                    tracing::warn!(
                        %error,
                        target_msg_id = %target_msg_id,
                        "Failed to send stream message with reference; retrying without reference"
                    );
                }
            }
        }
        let message = channel_id
            .send_message(
                &self.http,
                CreateMessage::new()
                    .content(content)
                    .allowed_mentions(safe_allowed_mentions()),
            )
            .await?;
        Ok(message.id)
    }

    async fn delete_message(&self, channel_id: ChannelId, message_id: MessageId) -> Result<()> {
        channel_id.delete_message(&self.http, message_id).await?;
        Ok(())
    }
}

struct LiveEditState {
    last_edit: Option<Instant>,
    message_ids: Vec<MessageId>,
}

/// Serializes and rate-limits updates for one streaming Discord response.
///
/// Non-final updates are coalesced: when a newer update arrives during the
/// debounce window, the stale update returns without touching Discord. The
/// debounce sleep happens outside the state mutex, so a final update does not
/// queue behind a sleeping intermediate update. Network mutations remain
/// serialized to preserve message ordering and the chunk/message-id mapping.
pub struct LiveEditThrottler<T: DiscordMessageTransport + ?Sized> {
    transport: Arc<T>,
    channel_id: ChannelId,
    debounce: Duration,
    revision: AtomicU64,
    state: Mutex<LiveEditState>,
}

impl<T: DiscordMessageTransport + ?Sized> LiveEditThrottler<T> {
    pub fn new(transport: Arc<T>, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self::with_debounce(transport, channel_id, message_id, DEFAULT_DEBOUNCE)
    }

    pub fn with_debounce(
        transport: Arc<T>,
        channel_id: ChannelId,
        message_id: MessageId,
        debounce: Duration,
    ) -> Self {
        Self {
            transport,
            channel_id,
            debounce,
            revision: AtomicU64::new(0),
            state: Mutex::new(LiveEditState {
                last_edit: None,
                message_ids: vec![message_id],
            }),
        }
    }

    pub fn debounce(&self) -> Duration {
        self.debounce
    }

    pub async fn update(&self, content: &str, is_final: bool) -> Result<()> {
        let revision = self.revision.fetch_add(1, Ordering::AcqRel) + 1;

        if !is_final && !self.wait_for_debounce(revision).await {
            return Ok(());
        }

        let mut state = self.state.lock().await;
        if !is_final && self.revision.load(Ordering::Acquire) != revision {
            return Ok(());
        }

        // A previous edit may have finished while this update was waiting for
        // the wire-serialization mutex. Re-check the deadline without holding
        // the mutex across the sleep.
        if !is_final {
            if let Some(last_edit) = state.last_edit {
                let deadline = last_edit + self.debounce;
                if deadline > Instant::now() {
                    drop(state);
                    tokio::time::sleep_until(deadline).await;
                    if self.revision.load(Ordering::Acquire) != revision {
                        return Ok(());
                    }
                    state = self.state.lock().await;
                    if self.revision.load(Ordering::Acquire) != revision {
                        return Ok(());
                    }
                }
            }
        }

        // The typing indicator is cosmetic: its failure must never abort an
        // update (especially the final one that carries the answer).
        if let Err(error) = self.transport.start_typing(self.channel_id).await {
            tracing::debug!(%error, "typing indicator failed; continuing live edit");
        }

        if !is_final {
            let preview = truncate_live_preview(content, DISCORD_MESSAGE_LIMIT);
            if let Some(&first_id) = state.message_ids.first() {
                self.transport
                    .edit_message(self.channel_id, first_id, preview)
                    .await?;
            }
        } else {
            let chunks = bound_split_messages(
                chunk_markdown(content, DISCORD_MESSAGE_LIMIT),
                MAX_SPLIT_MESSAGES,
            );
            for (index, chunk) in chunks.iter().enumerate() {
                if index < state.message_ids.len() {
                    self.transport
                        .edit_message(self.channel_id, state.message_ids[index], chunk.clone())
                        .await?;
                } else {
                    let message_id = self
                        .transport
                        .send_message(self.channel_id, chunk.clone())
                        .await?;
                    state.message_ids.push(message_id);
                }
            }

            let stale: Vec<_> = state.message_ids.drain(chunks.len()..).collect();
            for message_id in stale {
                self.transport
                    .delete_message(self.channel_id, message_id)
                    .await?;
            }
        }

        state.last_edit = Some(Instant::now());
        Ok(())
    }

    async fn wait_for_debounce(&self, revision: u64) -> bool {
        loop {
            if self.revision.load(Ordering::Acquire) != revision {
                return false;
            }
            let deadline = {
                let state = self.state.lock().await;
                state.last_edit.map(|last_edit| last_edit + self.debounce)
            };
            let Some(deadline) = deadline else {
                return self.revision.load(Ordering::Acquire) == revision;
            };
            if deadline <= Instant::now() {
                return self.revision.load(Ordering::Acquire) == revision;
            }
            tokio::time::sleep_until(deadline).await;
        }
    }
}

/// Truncates streamed live content for a single preview message up to `limit` characters,
/// appending a trailing indicator (` …`) when content exceeds the limit.
pub fn truncate_live_preview(content: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    if content.is_empty() {
        return "\u{200b}".into();
    }
    if content.chars().count() <= limit {
        return content.to_string();
    }
    let max_chars = limit.saturating_sub(2);
    let (prefix, _) = take_chars(content, max_chars);
    format!("{prefix} …")
}

pub fn is_chunk_pagination_enabled() -> bool {
    std::env::var("DISCORD_CHUNK_PAGINATION")
        .map(|val| {
            let lower = val.to_ascii_lowercase();
            lower != "false" && lower != "0" && lower != "no" && lower != "off"
        })
        .unwrap_or(true)
}

/// Splits Markdown by Unicode characters while keeping fenced code valid in
/// every Discord message. When `paginate` is enabled and the message splits into
/// multiple chunks, `(i/N)` headers are prepended to each chunk while preserving
/// code fence boundaries.
pub fn chunk_markdown_paginated(content: &str, limit: usize, paginate: bool) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    if content.is_empty() {
        return vec!["\u{200b}".into()];
    }

    // Single chunk check
    let unpaginated = chunk_markdown_raw(content, limit, None);
    if unpaginated.len() <= 1 || !paginate {
        return unpaginated;
    }

    // Multi-chunk pagination: two-pass convergence to find exact total chunk count
    let mut total = unpaginated.len();
    let mut chunks = chunk_markdown_raw(content, limit, Some(total));
    if chunks.len() != total {
        total = chunks.len();
        chunks = chunk_markdown_raw(content, limit, Some(total));
    }
    chunks
}

/// Splits Markdown by Unicode characters while keeping fenced code valid in
/// every Discord message. A fence crossing a boundary is closed in the old
/// chunk and reopened with its original language tag in the next chunk.
///
/// If `DISCORD_CHUNK_PAGINATION` is enabled (default true), multi-chunk messages
/// include `(i/N)` headers.
pub fn chunk_markdown(content: &str, limit: usize) -> Vec<String> {
    chunk_markdown_paginated(content, limit, is_chunk_pagination_enabled())
}

/// Bounds the number of split message chunks to `max_messages`. If the total chunk count
/// exceeds `max_messages`, only the first `max_messages` are kept and an explicit truncation
/// notice is appended to the final kept chunk.
pub fn bound_split_messages(mut chunks: Vec<String>, max_messages: usize) -> Vec<String> {
    if chunks.len() <= max_messages || max_messages == 0 {
        return chunks;
    }

    chunks.truncate(max_messages);
    if let Some(last) = chunks.last_mut() {
        let budget = DISCORD_MESSAGE_LIMIT.saturating_sub(TRUNCATION_NOTICE.len());
        if last.len() > budget {
            let mut end = budget;
            while end > 0 && !last.is_char_boundary(end) {
                end -= 1;
            }
            last.truncate(end);
        }
        last.push_str(TRUNCATION_NOTICE);
    }
    chunks
}

fn chunk_markdown_raw(content: &str, limit: usize, total_chunks: Option<usize>) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = content;
    let mut reopen: Option<String> = None;
    let mut current_index = 1;

    while !remaining.is_empty() {
        let (header, fence_open) = if let Some(total) = total_chunks {
            let hdr = format!("({current_index}/{total})\n");
            let f_open = reopen
                .as_ref()
                .map(|language| format!("```{language}\n"))
                .unwrap_or_default();
            (hdr, f_open)
        } else {
            let f_open = reopen
                .as_ref()
                .map(|language| format!("```{language}\n"))
                .unwrap_or_default();
            (String::new(), f_open)
        };

        let prefix = format!("{header}{fence_open}");
        let potential_closing_len = 4; // reserve space for potential "\n```" suffix
        let budget = limit.saturating_sub(prefix.chars().count() + potential_closing_len);
        if budget == 0 {
            chunks.push(take_chars(remaining, limit).0.to_owned());
            remaining = take_chars(remaining, limit).1;
            reopen = None;
            current_index += 1;
            continue;
        }

        let (candidate, rest) = take_chars(remaining, budget);
        let split_at = if rest.is_empty() {
            candidate.len()
        } else {
            preferred_boundary(candidate).unwrap_or(candidate.len())
        };
        let piece = &remaining[..split_at];
        let mut fence = reopen.clone();
        scan_fences(piece, &mut fence);

        let suffix = if !rest.is_empty() && fence.is_some() {
            "\n```"
        } else {
            ""
        };
        chunks.push(format!("{prefix}{piece}{suffix}"));
        remaining = &remaining[split_at..];
        reopen = fence;
        current_index += 1;
    }

    chunks
}

fn take_chars(value: &str, count: usize) -> (&str, &str) {
    let split = value
        .char_indices()
        .nth(count)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    value.split_at(split)
}

fn preferred_boundary(candidate: &str) -> Option<usize> {
    candidate
        .rmatch_indices('\n')
        .next()
        .filter(|(index, _)| *index > candidate.len() / 2)
        .map(|(index, _)| index + 1)
        .or_else(|| {
            candidate
                .rmatch_indices(char::is_whitespace)
                .next()
                .filter(|(index, _)| *index > candidate.len() / 2)
                .map(|(index, character)| index + character.len())
        })
}

fn scan_fences(text: &str, open: &mut Option<String>) {
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(after_fence) = trimmed.strip_prefix("```") {
            if open.is_some() {
                *open = None;
            } else {
                *open = Some(after_fence.trim().to_owned());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingTypingTransport {
        typing_calls: AtomicU64,
        edits: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait]
    impl DiscordMessageTransport for FailingTypingTransport {
        async fn start_typing(&self, _channel_id: ChannelId) -> Result<()> {
            self.typing_calls.fetch_add(1, Ordering::SeqCst);
            Err(crate::error::OmonError::Config("typing failed".into()))
        }

        async fn edit_message(
            &self,
            _channel_id: ChannelId,
            _message_id: MessageId,
            content: String,
        ) -> Result<()> {
            self.edits.lock().unwrap().push(content);
            Ok(())
        }

        async fn send_message(
            &self,
            _channel_id: ChannelId,
            _content: String,
        ) -> Result<MessageId> {
            Ok(MessageId::new(2))
        }

        async fn delete_message(
            &self,
            _channel_id: ChannelId,
            _message_id: MessageId,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn final_update_is_delivered_even_when_typing_fails() {
        let transport = Arc::new(FailingTypingTransport {
            typing_calls: AtomicU64::new(0),
            edits: std::sync::Mutex::new(Vec::new()),
        });
        let throttler = LiveEditThrottler::with_debounce(
            transport.clone(),
            ChannelId::new(1),
            MessageId::new(1),
            Duration::ZERO,
        );
        throttler.update("partial", false).await.unwrap();
        throttler.update("final answer", true).await.unwrap();
        assert_eq!(
            transport.edits.lock().unwrap().last().map(String::as_str),
            Some("final answer")
        );
        assert_eq!(transport.typing_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_truncate_live_preview_empty_and_short() {
        assert_eq!(truncate_live_preview("", 2000), "\u{200b}");
        assert_eq!(truncate_live_preview("hello world", 2000), "hello world");
        assert_eq!(truncate_live_preview("hello", 0), "");
    }

    #[test]
    fn test_chunk_markdown_pagination_single_chunk_no_header() {
        let content = "This is a single chunk message that should not receive a header.";
        let chunks = chunk_markdown_paginated(content, 2000, true);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], content);
        assert!(!chunks[0].contains("(1/1)"));
    }

    #[test]
    fn test_chunk_markdown_pagination_multi_chunk_headers() {
        let content = "Paragraph one.\n\n".repeat(50); // ~800 chars
        let chunks = chunk_markdown_paginated(&content, 300, true);
        assert!(chunks.len() > 1);
        let total = chunks.len();
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(chunk.starts_with(&format!("({}/{})\n", i + 1, total)));
            assert!(chunk.chars().count() <= 300);
        }
    }

    #[test]
    fn test_chunk_markdown_pagination_preserves_code_fences() {
        let content = format!("intro\n```rust\n{}\n```\noutro", "let x = 42;\n".repeat(40));
        let chunks = chunk_markdown_paginated(&content, 200, true);
        assert!(chunks.len() > 1);
        let total = chunks.len();
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(chunk.starts_with(&format!("({}/{})\n", i + 1, total)));
            assert!(chunk.chars().count() <= 200);
            assert_eq!(
                chunk.matches("```").count() % 2,
                0,
                "code fence unbalanced in chunk {i}: {chunk}"
            );
        }
        // Second chunk should reopen the rust fence cleanly after header
        assert!(chunks[1].contains("```rust\n"));
    }

    #[test]
    fn test_chunk_markdown_unpaginated_skips_headers() {
        let content = "Some long text without headers.\n\n".repeat(40);
        let chunks = chunk_markdown_paginated(&content, 300, false);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(!chunk.starts_with("(1/") && !chunk.starts_with("(2/"));
            assert!(chunk.chars().count() <= 300);
        }
    }
}

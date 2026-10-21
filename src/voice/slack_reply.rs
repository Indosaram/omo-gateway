//! Remembers which Slack audio messages should get a spoken file back.

use std::collections::HashSet;
use std::sync::Arc;

/// Audio message timestamps (`channel:ts`) whose agent reply should be synthesized.
#[derive(Clone, Default)]
pub struct SlackVoiceReplies {
    pending: Arc<parking_lot::Mutex<HashSet<String>>>,
}

impl SlackVoiceReplies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn note(&self, channel: &str, ts: &str) {
        if channel.is_empty() || ts.is_empty() {
            return;
        }
        self.pending.lock().insert(key(channel, ts));
    }

    /// Claims one pending audio message. A later duplicate reply does not upload again.
    pub fn take(&self, channel: &str, reply_to: Option<&str>) -> bool {
        let Some(ts) = reply_to.filter(|ts| !ts.is_empty()) else {
            return false;
        };
        self.pending.lock().remove(&key(channel, ts))
    }
}

fn key(channel: &str, ts: &str) -> String {
    format!("{channel}:{ts}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_audio_message_is_claimed_once_for_its_reply() {
        let replies = SlackVoiceReplies::new();
        replies.note("C1", "1700.0001");
        assert!(replies.take("C1", Some("1700.0001")));
        assert!(!replies.take("C1", Some("1700.0001")));
        assert!(!replies.take("C1", None));
        replies.note("", "1");
        assert!(!replies.take("", Some("1")));
    }
}

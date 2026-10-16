//! Slack channel: Socket Mode ingress, Web API egress, slash commands and approval buttons.
//!
//! Shared contract for every submodule. Session keys use `platform = "slack"`,
//! `guild_id = Some(team_id)` in channels (`None` in DMs), `channel_id` = Slack channel id,
//! `thread_id = Some(thread_ts)` for threaded conversations, `user_id` = Slack user id in DMs
//! and `bot_id = Some(bot user id)`. Platform message ids are Slack `ts` strings.

pub mod api;
pub mod commands;
pub mod egress;
pub mod ingress;
pub mod interactive;
pub mod mrkdwn;
pub mod runtime;
pub mod socket;

use std::path::PathBuf;

pub use api::SlackApi;
pub use egress::SlackEgress;
pub use runtime::SlackRuntime;

pub const PLATFORM: &str = "slack";
/// Owner Slack user id used when `SLACK_ALLOWED_USERS` is unset.
pub const DEFAULT_OWNER_ID: &str = "U04M1JNJM8T";
/// Slack rejects `text` longer than 40k; keep chunks well under the 4k display fold.
pub const MESSAGE_LIMIT: usize = 3900;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: String,
    /// Users allowed to talk to the bot and press approval buttons. First entry is the owner.
    pub allowed_users: Vec<String>,
    /// Channels where every message (not only @mentions) starts or continues a turn.
    pub free_response_channels: Vec<String>,
    /// Answer a top-level channel @mention in a thread rooted at the mention.
    pub auto_thread: bool,
    pub attachment_dir: PathBuf,
}

impl SlackConfig {
    /// `None` when Slack is disabled (`OMON_SLACK_ENABLED=false`) or either token is missing.
    ///
    /// Tokens: `SLACK_BOT_TOKEN` / `SLACK_APP_TOKEN`, else the files named by
    /// `SLACK_BOT_TOKEN_FILE` / `SLACK_APP_TOKEN_FILE`, else
    /// `~/.omon/secrets/slack-bot-token` / `~/.omon/secrets/slack-app-token`.
    pub fn from_env(workspace_root: &std::path::Path) -> Option<Self> {
        if std::env::var("OMON_SLACK_ENABLED")
            .is_ok_and(|value| matches!(value.trim(), "0" | "false" | "no" | "off"))
        {
            return None;
        }
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        let secrets = home.join(".omon").join("secrets");
        let bot_token = token("SLACK_BOT_TOKEN", "SLACK_BOT_TOKEN_FILE", &secrets.join("slack-bot-token"), "xoxb-")?;
        let app_token = token("SLACK_APP_TOKEN", "SLACK_APP_TOKEN_FILE", &secrets.join("slack-app-token"), "xapp-")?;
        let mut allowed_users = list("SLACK_ALLOWED_USERS");
        if allowed_users.is_empty() {
            allowed_users.push(DEFAULT_OWNER_ID.to_owned());
        }
        Some(Self {
            bot_token,
            app_token,
            allowed_users,
            free_response_channels: list("SLACK_FREE_RESPONSE_CHANNELS"),
            auto_thread: !std::env::var("SLACK_AUTO_THREAD")
                .is_ok_and(|value| matches!(value.trim(), "0" | "false" | "no" | "off")),
            attachment_dir: workspace_root.join("attachments").join("slack"),
        })
    }

    pub fn owner_id(&self) -> &str {
        self.allowed_users
            .first()
            .map_or(DEFAULT_OWNER_ID, String::as_str)
    }

    pub fn is_allowed(&self, user: &str) -> bool {
        self.allowed_users.iter().any(|id| id == user)
    }
}

/// Owner Slack user id from the environment, for code that has no `SlackConfig` at hand.
pub fn owner_id() -> String {
    list("SLACK_ALLOWED_USERS")
        .into_iter()
        .next()
        .unwrap_or_else(|| DEFAULT_OWNER_ID.to_owned())
}

fn list(name: &str) -> Vec<String> {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn token(var: &str, file_var: &str, default_file: &std::path::Path, prefix: &str) -> Option<String> {
    let raw = std::env::var(var).ok().or_else(|| {
        let path = std::env::var_os(file_var)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_file.to_path_buf());
        std::fs::read_to_string(path).ok()
    })?;
    let value = raw.trim().trim_matches('"').to_owned();
    value.starts_with(prefix).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_rejects_wrong_prefix_and_trims() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tok");
        std::fs::write(&path, "  xoxb-abc\n").unwrap();
        assert_eq!(
            token("OMON_TEST_UNSET_SLACK_VAR", "OMON_TEST_UNSET_SLACK_FILE", &path, "xoxb-"),
            Some("xoxb-abc".into())
        );
        assert_eq!(
            token("OMON_TEST_UNSET_SLACK_VAR", "OMON_TEST_UNSET_SLACK_FILE", &path, "xapp-"),
            None
        );
    }
}

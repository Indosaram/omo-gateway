//! Slack channel: Socket Mode ingress, Web API egress, slash commands and approval buttons.
//!
//! Shared contract for every submodule. Session keys use `platform = "slack"`,
//! `guild_id = Some(team_id)` in channels (`None` in DMs), `channel_id` = Slack channel id,
//! `thread_id = Some(thread_ts)` for threaded conversations, `user_id` = Slack user id in DMs
//! and `bot_id = Some(bot user id)`. Platform message ids are Slack `ts` strings.

pub mod api;
pub mod backfill;
pub mod buttons;
pub mod commands;
pub mod debounce;
pub mod egress;
pub mod home;
pub mod ingress;
pub mod interactive;
pub mod lifecycle;
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
pub const DEFAULT_CHANNEL_CONTEXT_LIMIT: usize = 10;
pub const MAX_CHANNEL_CONTEXT_LIMIT: usize = 25;

/// Which bot-authored messages may start or continue a turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AllowBots {
    #[default]
    None,
    /// Only bot messages that @mention this bot.
    Mentions,
    All,
}

impl AllowBots {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("all") => Self::All,
            Some("mentions" | "mention") => Self::Mentions,
            _ => Self::None,
        }
    }
}

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
    /// Owner home channel (`SLACK_HOME_CHANNEL`); always a free-response channel too.
    pub home_channel: Option<String>,
    pub approval_mentions: bool,
    pub allowed_channels: Vec<String>,
    pub ignored_channels: Vec<String>,
    pub allow_bots: AllowBots,
    pub thread_require_mention: bool,
    pub channel_context: bool,
    pub channel_context_limit: usize,
    pub channel_topic_context: bool,
    pub processing_reactions: bool,
    pub runtime_footer: bool,
    pub missed_backfill: bool,
    pub destructive_slash_confirm: bool,
    pub allow_all_users: bool,
}

impl SlackConfig {
    /// `None` when Slack is disabled (`OMON_SLACK_ENABLED=false`) or either token is missing.
    ///
    /// Tokens: `SLACK_BOT_TOKEN` / `SLACK_APP_TOKEN`, else the files named by
    /// `SLACK_BOT_TOKEN_FILE` / `SLACK_APP_TOKEN_FILE`, else
    /// `~/.omon/secrets/slack-bot-token` / `~/.omon/secrets/slack-app-token`.
    ///
    /// Behavior flags read `SLACK_<NAME>` and fall back to `DISCORD_<NAME>`. Id-valued settings
    /// (users, channels, home channel) are Slack-only: a Discord snowflake never names a Slack
    /// conversation, and an inherited channel allowlist would silence every Slack channel.
    pub fn from_env(workspace_root: &std::path::Path) -> Option<Self> {
        if std::env::var("OMON_SLACK_ENABLED")
            .is_ok_and(|value| matches!(value.trim(), "0" | "false" | "no" | "off"))
        {
            return None;
        }
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        let secrets = home.join(".omon").join("secrets");
        let bot_token = token(
            "SLACK_BOT_TOKEN",
            "SLACK_BOT_TOKEN_FILE",
            &secrets.join("slack-bot-token"),
            "xoxb-",
        )?;
        let app_token = token(
            "SLACK_APP_TOKEN",
            "SLACK_APP_TOKEN_FILE",
            &secrets.join("slack-app-token"),
            "xapp-",
        )?;
        Some(Self::from_lookup(
            bot_token,
            app_token,
            workspace_root.join("attachments").join("slack"),
            &|name| std::env::var(name).ok(),
        ))
    }

    fn from_lookup(
        bot_token: String,
        app_token: String,
        attachment_dir: PathBuf,
        lookup: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        let mut allowed_users = split_list(lookup("SLACK_ALLOWED_USERS"));
        if allowed_users.is_empty() {
            allowed_users.push(DEFAULT_OWNER_ID.to_owned());
        }
        let home_channels = split_list(lookup("SLACK_HOME_CHANNEL"));
        let mut free_response_channels = split_list(lookup("SLACK_FREE_RESPONSE_CHANNELS"));
        for channel in &home_channels {
            if !free_response_channels.contains(channel) {
                free_response_channels.push(channel.clone());
            }
        }
        let destructive_slash_confirm = setting(lookup, "DESTRUCTIVE_SLASH_CONFIRM")
            .or_else(|| non_empty(lookup("APPROVALS_DESTRUCTIVE_SLASH_CONFIRM")));
        Self {
            bot_token,
            app_token,
            allowed_users,
            free_response_channels,
            auto_thread: parse_flag(non_empty(lookup("SLACK_AUTO_THREAD")).as_deref(), true),
            attachment_dir,
            home_channel: home_channels.into_iter().next(),
            approval_mentions: flag(lookup, "APPROVAL_MENTIONS", true),
            allowed_channels: split_list(lookup("SLACK_ALLOWED_CHANNELS")),
            ignored_channels: split_list(lookup("SLACK_IGNORED_CHANNELS")),
            allow_bots: AllowBots::parse(setting(lookup, "ALLOW_BOTS").as_deref()),
            thread_require_mention: flag(lookup, "THREAD_REQUIRE_MENTION", false),
            channel_context: flag(lookup, "CHANNEL_CONTEXT", true),
            channel_context_limit: setting(lookup, "CHANNEL_CONTEXT_LIMIT")
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(DEFAULT_CHANNEL_CONTEXT_LIMIT)
                .min(MAX_CHANNEL_CONTEXT_LIMIT),
            channel_topic_context: flag(lookup, "CHANNEL_TOPIC_CONTEXT", true),
            processing_reactions: flag(lookup, "PROCESSING_REACTIONS", true),
            runtime_footer: flag(lookup, "RUNTIME_FOOTER", false),
            missed_backfill: flag(lookup, "MISSED_BACKFILL", true),
            destructive_slash_confirm: parse_flag(destructive_slash_confirm.as_deref(), true),
            allow_all_users: flag(lookup, "ALLOW_ALL_USERS", false),
        }
    }

    /// Fixed tokens and contract defaults, for unit tests in every lane.
    pub fn test_default() -> Self {
        Self::from_lookup(
            "xoxb-test".into(),
            "xapp-test".into(),
            PathBuf::from("/tmp/omon-slack-test"),
            &|_| None,
        )
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

/// Slack home channels from `SLACK_HOME_CHANNEL`, for code that has no `SlackConfig` at hand.
pub fn home_channels() -> Vec<String> {
    list("SLACK_HOME_CHANNEL")
}

fn list(name: &str) -> Vec<String> {
    split_list(std::env::var(name).ok())
}

fn split_list(value: Option<String>) -> Vec<String> {
    value
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

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn setting(lookup: &dyn Fn(&str) -> Option<String>, name: &str) -> Option<String> {
    non_empty(lookup(&format!("SLACK_{name}")))
        .or_else(|| non_empty(lookup(&format!("DISCORD_{name}"))))
}

fn flag(lookup: &dyn Fn(&str) -> Option<String>, name: &str, default: bool) -> bool {
    parse_flag(setting(lookup, name).as_deref(), default)
}

fn parse_flag(raw: Option<&str>, default: bool) -> bool {
    match raw
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("1" | "true" | "yes" | "on") => true,
        Some("0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

fn token(
    var: &str,
    file_var: &str,
    default_file: &std::path::Path,
    prefix: &str,
) -> Option<String> {
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
    use std::collections::HashMap;

    use super::*;

    fn config_from(pairs: &[(&str, &str)]) -> SlackConfig {
        let env: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        SlackConfig::from_lookup(
            "xoxb-t".into(),
            "xapp-t".into(),
            PathBuf::from("/tmp"),
            &move |name| env.get(name).cloned(),
        )
    }

    #[test]
    fn defaults_match_the_contract() {
        let config = SlackConfig::test_default();
        assert_eq!(config.allowed_users, [DEFAULT_OWNER_ID]);
        assert_eq!(config.home_channel, None);
        assert!(config.approval_mentions);
        assert!(config.allowed_channels.is_empty() && config.ignored_channels.is_empty());
        assert_eq!(config.allow_bots, AllowBots::None);
        assert!(!config.thread_require_mention);
        assert!(config.channel_context);
        assert_eq!(config.channel_context_limit, 10);
        assert!(config.channel_topic_context);
        assert!(config.processing_reactions);
        assert!(!config.runtime_footer);
        assert!(config.missed_backfill);
        assert!(config.destructive_slash_confirm);
        assert!(!config.allow_all_users);
        assert!(config.auto_thread);
    }

    #[test]
    fn slack_settings_win_and_discord_flags_fill_gaps() {
        let config = config_from(&[
            ("SLACK_RUNTIME_FOOTER", "true"),
            ("DISCORD_RUNTIME_FOOTER", "false"),
            ("DISCORD_CHANNEL_CONTEXT", "off"),
            ("DISCORD_ALLOW_BOTS", "mentions"),
            ("SLACK_CHANNEL_CONTEXT_LIMIT", "99"),
            ("DISCORD_MISSED_BACKFILL", "0"),
            ("APPROVALS_DESTRUCTIVE_SLASH_CONFIRM", "false"),
        ]);
        assert!(config.runtime_footer);
        assert!(!config.channel_context);
        assert_eq!(config.allow_bots, AllowBots::Mentions);
        assert_eq!(config.channel_context_limit, MAX_CHANNEL_CONTEXT_LIMIT);
        assert!(!config.missed_backfill);
        assert!(!config.destructive_slash_confirm);
    }

    #[test]
    fn id_settings_never_inherit_discord_values() {
        let config = config_from(&[
            ("DISCORD_ALLOWED_CHANNELS", "1529738911377657967"),
            ("DISCORD_IGNORED_CHANNELS", "1"),
            ("DISCORD_HOME_CHANNEL", "2"),
            ("DISCORD_ALLOWED_USERS", "414011632306618368"),
        ]);
        assert!(config.allowed_channels.is_empty());
        assert!(config.ignored_channels.is_empty());
        assert_eq!(config.home_channel, None);
        assert_eq!(config.allowed_users, [DEFAULT_OWNER_ID]);
    }

    #[test]
    fn home_channel_is_also_free_response() {
        let config = config_from(&[
            ("SLACK_HOME_CHANNEL", " CHOME "),
            ("SLACK_FREE_RESPONSE_CHANNELS", "CFREE,CHOME"),
        ]);
        assert_eq!(config.home_channel.as_deref(), Some("CHOME"));
        assert_eq!(config.free_response_channels, ["CFREE", "CHOME"]);
    }

    #[test]
    fn allow_bots_parses_known_modes_only() {
        assert_eq!(AllowBots::parse(Some(" ALL ")), AllowBots::All);
        assert_eq!(AllowBots::parse(Some("mention")), AllowBots::Mentions);
        assert_eq!(AllowBots::parse(Some("everyone")), AllowBots::None);
        assert_eq!(AllowBots::parse(None), AllowBots::None);
    }

    #[test]
    fn token_rejects_wrong_prefix_and_trims() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tok");
        std::fs::write(&path, "  xoxb-abc\n").unwrap();
        assert_eq!(
            token(
                "OMON_TEST_UNSET_SLACK_VAR",
                "OMON_TEST_UNSET_SLACK_FILE",
                &path,
                "xoxb-"
            ),
            Some("xoxb-abc".into())
        );
        assert_eq!(
            token(
                "OMON_TEST_UNSET_SLACK_VAR",
                "OMON_TEST_UNSET_SLACK_FILE",
                &path,
                "xapp-"
            ),
            None
        );
    }
}

use std::sync::Arc;

use serenity::all::{
    Channel, ChannelId, ChannelType, CreateMessage, CreateThread, EditMessage, EditThread,
    GuildChannel, Http, Message, MessageId,
};

pub use crate::discord::adapter::safe_allowed_mentions;

/// Lane thread messages never ping anyone: no users, roles, everyone or reply author.
pub fn lane_allowed_mentions() -> serenity::all::CreateAllowedMentions {
    safe_allowed_mentions()
        .all_users(false)
        .empty_users()
        .replied_user(false)
}

pub const DISCORD_THREAD_NAME_MAX_CHARS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Working,
    Waiting,
    Done,
}

impl ThreadStatus {
    pub const ALL: [ThreadStatus; 3] = [
        ThreadStatus::Working,
        ThreadStatus::Waiting,
        ThreadStatus::Done,
    ];

    #[inline]
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Working => "🔄",
            Self::Waiting => "⏸",
            Self::Done => "✅",
        }
    }

    #[inline]
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Working => "🔄 ",
            Self::Waiting => "⏸ ",
            Self::Done => "✅ ",
        }
    }
}

impl From<crate::lanes::state::LaneStatus> for ThreadStatus {
    fn from(status: crate::lanes::state::LaneStatus) -> Self {
        match status {
            crate::lanes::state::LaneStatus::Working => ThreadStatus::Working,
            crate::lanes::state::LaneStatus::Queued
            | crate::lanes::state::LaneStatus::Blocked
            | crate::lanes::state::LaneStatus::NotDone => ThreadStatus::Waiting,
            crate::lanes::state::LaneStatus::DoneClaimed
            | crate::lanes::state::LaneStatus::VerifiedDone
            | crate::lanes::state::LaneStatus::Closed
            | crate::lanes::state::LaneStatus::Failed => ThreadStatus::Done,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreatedLaneThread {
    pub parent_channel_id: ChannelId,
    pub starter_message_id: MessageId,
    pub thread_channel_id: ChannelId,
    pub thread_name: String,
}

pub fn strip_status_prefix(title: &str) -> &str {
    let trimmed = title.trim();
    for prefix in &["🔄", "⏸", "✅"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.strip_prefix(' ').unwrap_or(rest).trim();
        }
    }
    trimmed
}

pub fn format_thread_name(status: ThreadStatus, raw_title: &str) -> String {
    let clean = strip_status_prefix(raw_title);
    let base = if clean.is_empty() { "lane" } else { clean };
    let full = format!("{}{}", status.prefix(), base);
    if full.chars().count() > DISCORD_THREAD_NAME_MAX_CHARS {
        full.chars().take(DISCORD_THREAD_NAME_MAX_CHARS).collect()
    } else {
        full
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscordThreadError {
    #[error("missing required environment variable DISCORD_BOT_TOKEN")]
    MissingToken,

    #[error("channel {channel_id} is not a guild thread (kind: {kind:?})")]
    NotAThread {
        channel_id: ChannelId,
        kind: Option<ChannelType>,
    },

    #[error("channel {0} is not a guild channel")]
    NotAGuildChannel(ChannelId),

    #[error("discord api error (status: {status_code}, code: {discord_code}): {message}")]
    DiscordApi {
        status_code: reqwest::StatusCode,
        discord_code: isize,
        message: String,
        #[source]
        source: Box<serenity::Error>,
    },

    #[error("serenity error: {0}")]
    Serenity(Box<serenity::Error>),

    #[error("omon error: {0}")]
    Omon(Box<crate::OmonError>),
}

impl DiscordThreadError {
    pub fn discord_code(&self) -> Option<isize> {
        match self {
            Self::DiscordApi { discord_code, .. } => Some(*discord_code),
            _ => None,
        }
    }

    pub fn status_code(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::DiscordApi { status_code, .. } => Some(*status_code),
            _ => None,
        }
    }
}

impl From<serenity::Error> for DiscordThreadError {
    fn from(err: serenity::Error) -> Self {
        if let serenity::Error::Http(serenity::all::HttpError::UnsuccessfulRequest(ref resp)) = err
        {
            return DiscordThreadError::DiscordApi {
                status_code: resp.status_code,
                discord_code: resp.error.code,
                message: resp.error.message.clone(),
                source: Box::new(err),
            };
        }
        DiscordThreadError::Serenity(Box::new(err))
    }
}

impl From<crate::OmonError> for DiscordThreadError {
    fn from(err: crate::OmonError) -> Self {
        DiscordThreadError::Omon(Box::new(err))
    }
}

pub fn parse_bot_tokens() -> Vec<String> {
    let mut tokens = Vec::new();
    if let Ok(tok) = std::env::var("DISCORD_BOT_TOKEN") {
        for t in tok.split(',') {
            let trimmed = t.trim().trim_matches('"').trim_matches('\'');
            if !trimmed.is_empty() {
                tokens.push(trimmed.to_string());
            }
        }
    }
    if let Ok(toks) = std::env::var("DISCORD_BOT_TOKENS") {
        for t in toks.split(',') {
            let trimmed = t.trim().trim_matches('"').trim_matches('\'');
            if !trimmed.is_empty() && !tokens.contains(&trimmed.to_string()) {
                tokens.push(trimmed.to_string());
            }
        }
    }
    tokens
}

pub fn first_bot_token() -> Result<String, DiscordThreadError> {
    parse_bot_tokens()
        .into_iter()
        .next()
        .ok_or(DiscordThreadError::MissingToken)
}

fn creation_metadata(key: &str, title: &str) -> (String, String) {
    let title = super::scrub::scrub(title);
    let key = super::scrub::scrub(key);
    let starter_text = if key.trim().is_empty() {
        format!("lane: {}", title.trim())
    } else {
        format!("lane {}: {}", key.trim(), title.trim())
    };
    let thread_name = format_thread_name(ThreadStatus::Working, &title);
    (starter_text, thread_name)
}

pub async fn create_thread(
    http: &Http,
    parent_channel: ChannelId,
    key: &str,
    title: &str,
) -> Result<CreatedLaneThread, DiscordThreadError> {
    let (starter_text, thread_name) = creation_metadata(key, title);

    let starter_msg = parent_channel
        .send_message(
            http,
            CreateMessage::new()
                .content(starter_text)
                .allowed_mentions(lane_allowed_mentions()),
        )
        .await?;

    let builder = CreateThread::new(thread_name.clone());
    let thread_channel = parent_channel
        .create_thread_from_message(http, starter_msg.id, builder)
        .await?;

    Ok(CreatedLaneThread {
        parent_channel_id: parent_channel,
        starter_message_id: starter_msg.id,
        thread_channel_id: thread_channel.id,
        thread_name,
    })
}

pub async fn create_thread_with_title(
    http: &Http,
    parent_channel: ChannelId,
    title: &str,
) -> Result<CreatedLaneThread, DiscordThreadError> {
    create_thread(http, parent_channel, "", title).await
}

pub async fn set_status(
    http: &Http,
    thread: ChannelId,
    status: ThreadStatus,
) -> Result<String, DiscordThreadError> {
    let channel = thread.to_channel(http).await?;
    let (current_name, kind) = match channel {
        Channel::Guild(gc) => (gc.name, gc.kind),
        _ => {
            return Err(DiscordThreadError::NotAThread {
                channel_id: thread,
                kind: None,
            });
        }
    };

    if !matches!(
        kind,
        ChannelType::NewsThread | ChannelType::PublicThread | ChannelType::PrivateThread
    ) {
        return Err(DiscordThreadError::NotAThread {
            channel_id: thread,
            kind: Some(kind),
        });
    }

    let new_name = format_thread_name(status, &current_name);
    let builder = EditThread::new().name(new_name.clone());
    thread.edit_thread(http, builder).await?;
    Ok(new_name)
}

pub async fn set_status_with_title(
    http: &Http,
    thread: ChannelId,
    status: ThreadStatus,
    title: &str,
) -> Result<String, DiscordThreadError> {
    let new_name = format_thread_name(status, title);
    let builder = EditThread::new().name(new_name.clone());
    thread.edit_thread(http, builder).await?;
    Ok(new_name)
}

pub async fn archive(http: &Http, thread: ChannelId) -> Result<GuildChannel, DiscordThreadError> {
    let builder = EditThread::new().archived(true);
    let channel = thread.edit_thread(http, builder).await?;
    Ok(channel)
}

pub async fn unarchive(http: &Http, thread: ChannelId) -> Result<GuildChannel, DiscordThreadError> {
    let builder = EditThread::new().archived(false);
    let channel = thread.edit_thread(http, builder).await?;
    Ok(channel)
}

pub async fn get_thread(
    http: &Http,
    thread: ChannelId,
) -> Result<GuildChannel, DiscordThreadError> {
    match thread.to_channel(http).await? {
        Channel::Guild(gc) => Ok(gc),
        Channel::Private(_) => Err(DiscordThreadError::NotAGuildChannel(thread)),
        _ => Err(DiscordThreadError::NotAGuildChannel(thread)),
    }
}

pub async fn is_archived(http: &Http, thread: ChannelId) -> Result<bool, DiscordThreadError> {
    let gc = get_thread(http, thread).await?;
    Ok(gc
        .thread_metadata
        .as_ref()
        .map(|m| m.archived)
        .unwrap_or(false))
}

pub async fn post(
    http: &Http,
    thread: ChannelId,
    text: &str,
) -> Result<Message, DiscordThreadError> {
    let builder = CreateMessage::new()
        .content(text)
        .allowed_mentions(lane_allowed_mentions());
    let message = thread.send_message(http, builder).await?;
    Ok(message)
}

pub async fn edit(
    http: &Http,
    thread: ChannelId,
    msg_id: MessageId,
    text: &str,
) -> Result<Message, DiscordThreadError> {
    let builder = EditMessage::new()
        .content(text)
        .allowed_mentions(lane_allowed_mentions());
    let message = thread.edit_message(http, msg_id, builder).await?;
    Ok(message)
}

#[derive(Clone)]
pub struct DiscordThreads {
    http: Arc<Http>,
}

/// QA REST origin only: no credentials, paths, query, or fragment are admitted.
pub fn parse_loopback_api_base(raw: &str) -> crate::Result<reqwest::Url> {
    let invalid = || {
        crate::OmonError::Config("OMON_DISCORD_API_BASE requires a loopback HTTP(S) origin".into())
    };
    let url = reqwest::Url::parse(raw).map_err(|_| invalid())?;
    let loopback = match url.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback()),
        None => false,
    };
    if !loopback
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(url)
}

impl DiscordThreads {
    /// Construct only the opted-in QA REST sink; never reads real bot credentials.
    pub fn loopback_from_env() -> crate::Result<Option<Self>> {
        match std::env::var("OMON_DISCORD_API_BASE") {
            Ok(raw) => Self::with_loopback_api_base(&raw).map(Some),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(crate::OmonError::Config(error.to_string())),
        }
    }

    pub fn with_loopback_api_base(raw: &str) -> crate::Result<Self> {
        let base = parse_loopback_api_base(raw)?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve("localhost", std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| crate::OmonError::Config(error.to_string()))?;
        Ok(Self::with_http(Arc::new(
            serenity::all::HttpBuilder::new("local-fixture-only")
                .client(client)
                .proxy(base.as_str().trim_end_matches('/'))
                .ratelimiter_disabled(true)
                .build(),
        )))
    }

    pub fn from_env() -> Result<Self, DiscordThreadError> {
        if let Some(sink) = Self::loopback_from_env()? {
            return Ok(sink);
        }
        let token = first_bot_token()?;
        Ok(Self::new(token))
    }

    pub fn new(token: impl AsRef<str>) -> Self {
        let http = Arc::new(Http::new(token.as_ref()));
        Self { http }
    }

    pub fn with_http(http: Arc<Http>) -> Self {
        Self { http }
    }

    pub fn http(&self) -> &Arc<Http> {
        &self.http
    }

    pub async fn create_thread(
        &self,
        parent_channel: ChannelId,
        key: &str,
        title: &str,
    ) -> Result<CreatedLaneThread, DiscordThreadError> {
        create_thread(&self.http, parent_channel, key, title).await
    }

    pub async fn create_thread_with_title(
        &self,
        parent_channel: ChannelId,
        title: &str,
    ) -> Result<CreatedLaneThread, DiscordThreadError> {
        create_thread_with_title(&self.http, parent_channel, title).await
    }

    pub async fn set_status(
        &self,
        thread: ChannelId,
        status: ThreadStatus,
    ) -> Result<String, DiscordThreadError> {
        set_status(&self.http, thread, status).await
    }

    pub async fn set_status_with_title(
        &self,
        thread: ChannelId,
        status: ThreadStatus,
        title: &str,
    ) -> Result<String, DiscordThreadError> {
        set_status_with_title(&self.http, thread, status, title).await
    }

    pub async fn archive(&self, thread: ChannelId) -> Result<GuildChannel, DiscordThreadError> {
        archive(&self.http, thread).await
    }

    pub async fn unarchive(&self, thread: ChannelId) -> Result<GuildChannel, DiscordThreadError> {
        unarchive(&self.http, thread).await
    }

    pub async fn get_thread(&self, thread: ChannelId) -> Result<GuildChannel, DiscordThreadError> {
        get_thread(&self.http, thread).await
    }

    pub async fn is_archived(&self, thread: ChannelId) -> Result<bool, DiscordThreadError> {
        is_archived(&self.http, thread).await
    }

    pub async fn post(&self, thread: ChannelId, text: &str) -> Result<Message, DiscordThreadError> {
        post(&self.http, thread, text).await
    }

    pub async fn edit(
        &self,
        thread: ChannelId,
        msg_id: MessageId,
        text: &str,
    ) -> Result<Message, DiscordThreadError> {
        edit(&self.http, thread, msg_id, text).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qa_gaps_loopback_base_accepts_local_origins() {
        for origin in [
            "http://127.0.0.1:18080",
            "http://[::1]:18080",
            "http://localhost:18080",
            "https://127.0.0.1",
        ] {
            let sink = DiscordThreads::with_loopback_api_base(origin).unwrap();
            assert_eq!(sink.http().proxy.as_deref(), Some(origin));
            assert_eq!(sink.http().token(), "Bot local-fixture-only");
        }
    }

    #[test]
    fn qa_gaps_loopback_base_rejects_nonlocal_or_ambiguous_origins() {
        for origin in [
            "http://example.com",
            "http://192.168.1.1",
            "http://0.0.0.0",
            "http://[::]",
            "http://[::ffff:127.0.0.1]",
            "http://localhost.example.com",
            "http://localhost@evil.example",
            "http://user@localhost",
            "http://localhost/api",
            "http://localhost?x=1",
            "http://localhost#fragment",
            "ftp://localhost",
            "",
            "not a url",
        ] {
            assert!(parse_loopback_api_base(origin).is_err(), "{origin}");
        }
    }

    #[tokio::test]
    async fn creation_metadata_scrubs_configured_hosts_in_isolation() {
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "lanes::discord_threads::tests::creation_metadata_subprocess_helper",
                    "--ignored",
                    "--nocapture",
                ])
                .env("OMON_SCRUB_HOSTS", " private.example , SECOND.EXAMPLE , ")
                .env("HOME", "/Users/qa-owner")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .expect("metadata subprocess must finish")
        .expect("metadata subprocess must launch");
        assert!(
            output.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    #[ignore = "invoked by creation_metadata_scrubs_configured_hosts_in_isolation"]
    fn creation_metadata_subprocess_helper() {
        let token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
        let title = format!(
            "작업 /Users/qa-owner/repo {token} PRIVATE.EXAMPLE second.example public.example"
        );
        let published = "작업 ~/repo <redacted> <redacted> <redacted> public.example";
        let (starter, name) = creation_metadata("qa-1", &title);
        assert_eq!(starter, format!("lane qa-1: {published}"));
        assert_eq!(name, format!("🔄 {published}"));
        for value in [&starter, &name] {
            assert!(!value.contains(token));
            assert!(!value.contains("/Users/qa-owner"));
            assert!(!value.to_lowercase().contains("private.example"));
            assert!(!value.to_lowercase().contains("second.example"));
        }
        let waiting = format_thread_name(ThreadStatus::Waiting, &name);
        assert_eq!(waiting, format!("⏸ {published}"));
        assert_eq!(
            format_thread_name(ThreadStatus::Done, &waiting),
            format!("✅ {published}"),
        );
        assert_eq!(
            creation_metadata("qa-1", "Fix login"),
            ("lane qa-1: Fix login".into(), "🔄 Fix login".into()),
        );
        assert_eq!(
            creation_metadata("", "Fix login"),
            ("lane: Fix login".into(), "🔄 Fix login".into()),
        );
        let (starter, _) = creation_metadata("private.example", "Fix login");
        assert_eq!(starter, "lane <redacted>: Fix login");
    }

    #[test]
    fn test_prefix_replace_all_statuses() {
        let raw = "fix login bug";
        let working = format_thread_name(ThreadStatus::Working, raw);
        assert_eq!(working, "🔄 fix login bug");

        let waiting = format_thread_name(ThreadStatus::Waiting, &working);
        assert_eq!(waiting, "⏸ fix login bug");

        let done = format_thread_name(ThreadStatus::Done, &waiting);
        assert_eq!(done, "✅ fix login bug");

        let back_to_working = format_thread_name(ThreadStatus::Working, &done);
        assert_eq!(back_to_working, "🔄 fix login bug");
    }

    #[test]
    fn test_prefix_variations_and_whitespace() {
        assert_eq!(
            format_thread_name(ThreadStatus::Waiting, "🔄fix bug"),
            "⏸ fix bug"
        );
        assert_eq!(
            format_thread_name(ThreadStatus::Done, "⏸   padded title"),
            "✅ padded title"
        );
        assert_eq!(
            format_thread_name(ThreadStatus::Working, "🚀 deploy pipeline"),
            "🔄 🚀 deploy pipeline"
        );
        assert_eq!(format_thread_name(ThreadStatus::Working, ""), "🔄 lane");
        assert_eq!(format_thread_name(ThreadStatus::Done, "   "), "✅ lane");
        assert_eq!(format_thread_name(ThreadStatus::Waiting, "🔄"), "⏸ lane");
        assert_eq!(format_thread_name(ThreadStatus::Working, "⏸   "), "🔄 lane");
    }

    #[test]
    fn test_ascii_100_char_cap() {
        let long_ascii = "a".repeat(150);
        let formatted = format_thread_name(ThreadStatus::Working, &long_ascii);
        assert_eq!(formatted.chars().count(), 100);
        assert!(formatted.starts_with("🔄 "));
        let suffix = &formatted["🔄 ".len()..];
        assert_eq!(suffix.chars().count(), 98);
        assert!(suffix.chars().all(|c| c == 'a'));
    }

    #[test]
    fn test_korean_unicode_char_boundary_cap() {
        let long_korean = "가".repeat(150);
        let formatted = format_thread_name(ThreadStatus::Working, &long_korean);
        assert_eq!(formatted.chars().count(), 100);
        assert!(formatted.starts_with("🔄 "));
        let suffix = &formatted["🔄 ".len()..];
        assert_eq!(suffix.chars().count(), 98);
        assert!(suffix.chars().all(|c| c == '가'));

        let mixed = "오모게이트웨이_레인_작업_진행_".repeat(10);
        let formatted_mixed = format_thread_name(ThreadStatus::Done, &mixed);
        assert_eq!(formatted_mixed.chars().count(), 100);
        assert!(formatted_mixed.starts_with("✅ "));
    }

    #[test]
    fn test_thread_status_conversion_from_lane_status() {
        use crate::lanes::state::LaneStatus;

        assert_eq!(
            ThreadStatus::from(LaneStatus::Working),
            ThreadStatus::Working
        );
        assert_eq!(
            ThreadStatus::from(LaneStatus::Queued),
            ThreadStatus::Waiting
        );
        assert_eq!(
            ThreadStatus::from(LaneStatus::Blocked),
            ThreadStatus::Waiting
        );
        assert_eq!(
            ThreadStatus::from(LaneStatus::NotDone),
            ThreadStatus::Waiting
        );
        assert_eq!(
            ThreadStatus::from(LaneStatus::DoneClaimed),
            ThreadStatus::Done
        );
        assert_eq!(
            ThreadStatus::from(LaneStatus::VerifiedDone),
            ThreadStatus::Done
        );
        assert_eq!(ThreadStatus::from(LaneStatus::Closed), ThreadStatus::Done);
        assert_eq!(ThreadStatus::from(LaneStatus::Failed), ThreadStatus::Done);
    }

    #[test]
    fn test_strip_status_prefix() {
        assert_eq!(strip_status_prefix("🔄 my-lane"), "my-lane");
        assert_eq!(strip_status_prefix("⏸ my-lane"), "my-lane");
        assert_eq!(strip_status_prefix("✅ my-lane"), "my-lane");
        assert_eq!(strip_status_prefix("🔄   spaced"), "spaced");
        assert_eq!(strip_status_prefix("no-prefix"), "no-prefix");
        assert_eq!(strip_status_prefix("   "), "");
    }

    #[test]
    fn test_token_parsing_logic() {
        let original_token = std::env::var("DISCORD_BOT_TOKEN").ok();
        let original_tokens = std::env::var("DISCORD_BOT_TOKENS").ok();

        std::env::set_var("DISCORD_BOT_TOKEN", "\"tok_1\", 'tok_2' ");
        std::env::set_var("DISCORD_BOT_TOKENS", "tok_2, tok_3");

        let parsed = parse_bot_tokens();
        assert_eq!(parsed, vec!["tok_1", "tok_2", "tok_3"]);
        assert_eq!(first_bot_token().unwrap(), "tok_1");

        if let Some(t) = original_token {
            std::env::set_var("DISCORD_BOT_TOKEN", t);
        } else {
            std::env::remove_var("DISCORD_BOT_TOKEN");
        }
        if let Some(t) = original_tokens {
            std::env::set_var("DISCORD_BOT_TOKENS", t);
        } else {
            std::env::remove_var("DISCORD_BOT_TOKENS");
        }
    }

    #[tokio::test]
    #[ignore = "requires live Discord bot token and DISCORD_HOME_CHANNEL"]
    async fn test_live_discord_thread_lifecycle_qa_scenario() {
        let token = match first_bot_token() {
            Ok(tok) => tok,
            Err(e) => {
                eprintln!("[BLOCKER] Live QA skipped: missing Discord bot token: {e}");
                return;
            }
        };

        let home_channel_raw = match std::env::var("DISCORD_HOME_CHANNEL") {
            Ok(val) if !val.trim().is_empty() => val,
            _ => {
                eprintln!("[BLOCKER] Live QA skipped: DISCORD_HOME_CHANNEL not set or empty");
                return;
            }
        };

        let home_channel_id = match home_channel_raw.trim().parse::<u64>() {
            Ok(id) => ChannelId::new(id),
            Err(e) => {
                eprintln!("[BLOCKER] Live QA skipped: invalid DISCORD_HOME_CHANNEL '{home_channel_raw}': {e}");
                return;
            }
        };

        let helper = DiscordThreads::new(token);

        println!("=== STEP 1: Creating starter message and QA thread ===");
        let created = helper
            .create_thread(home_channel_id, "qa-10", "qa-lane-thread")
            .await
            .expect("create_thread must succeed in home channel");

        println!("Parent Channel ID: {}", created.parent_channel_id.get());
        println!("Starter Message ID: {}", created.starter_message_id.get());
        println!("Thread Channel ID: {}", created.thread_channel_id.get());
        println!("Initial Thread Name: {}", created.thread_name);
        assert_eq!(created.thread_name, "🔄 qa-lane-thread");

        println!("=== STEP 2: Renaming thread status to Done (✅) ===");
        let renamed_title = helper
            .set_status(created.thread_channel_id, ThreadStatus::Done)
            .await
            .expect("set_status to Done must succeed");
        println!("Renamed Thread Title: {renamed_title}");
        assert_eq!(renamed_title, "✅ qa-lane-thread");

        println!("=== STEP 3: Archiving own QA thread ===");
        let archived_channel = helper
            .archive(created.thread_channel_id)
            .await
            .expect("archive must succeed");
        println!(
            "Archived Channel ID: {}, is_archived: {:?}",
            archived_channel.id.get(),
            archived_channel
                .thread_metadata
                .as_ref()
                .map(|m| m.archived)
        );

        println!("=== STEP 4: Verifying archived flag via GET channel ===");
        let is_arch = helper
            .is_archived(created.thread_channel_id)
            .await
            .expect("is_archived via GET channel must succeed");
        println!("GET channel archived flag: {is_arch}");
        assert!(
            is_arch,
            "Thread must be confirmed archived in GET channel response"
        );

        println!("=== STEP 5: Verifying invalid channel error code handling ===");
        let invalid_channel = ChannelId::new(999_999_999_999_999_999);
        let err_res = helper
            .post(invalid_channel, "test message to invalid channel")
            .await;
        assert!(err_res.is_err(), "Post to invalid channel must return Err");
        let err = err_res.unwrap_err();
        println!("Invalid channel error received: {err:?}");
        if let Some(code) = err.discord_code() {
            println!("Logged Discord Error Code: {code}");
        }
        if let Some(status) = err.status_code() {
            println!("Logged HTTP Status Code: {status}");
        }

        println!("=== QA RECEIPT ===");
        println!(
            "Receipt: Thread {} created, renamed to {}, and safely archived.",
            created.thread_channel_id, renamed_title
        );
    }
}

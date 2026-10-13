use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{InboundEvent, SessionContext, SessionKey};

/// Flexible deserializer for optional u64 supporting numbers (123), strings ("123"), and null.
fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumericOrString {
        U64(u64),
        I64(i64),
        String(String),
    }

    let opt: Option<NumericOrString> = Option::deserialize(deserializer)?;
    match opt {
        Some(NumericOrString::U64(v)) => Ok(Some(v)),
        Some(NumericOrString::I64(v)) => {
            // A negative id is a config typo; mapping it to `None` would widen the route to
            // "match any" instead of rejecting it.
            u64::try_from(v).map(Some).map_err(|_| {
                serde::de::Error::custom(format!("negative id {v} is not a valid Discord id"))
            })
        }
        Some(NumericOrString::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<u64>()
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
        None => Ok(None),
    }
}

fn default_true() -> bool {
    true
}

/// A routing rule that maps a Discord hierarchy (guild, channel, thread)
/// to a profile with custom model, system prompt, and enabled toolsets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileRoute {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(
        default,
        alias = "guild_id",
        deserialize_with = "deserialize_optional_u64"
    )]
    pub guild: Option<u64>,
    #[serde(
        default,
        alias = "channel_id",
        alias = "chat_id",
        deserialize_with = "deserialize_optional_u64"
    )]
    pub channel: Option<u64>,
    #[serde(
        default,
        alias = "thread_id",
        deserialize_with = "deserialize_optional_u64"
    )]
    pub thread: Option<u64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default, alias = "toolsets")]
    pub enabled_toolsets: Option<Vec<String>>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub allowed_users: Option<Vec<u64>>,
}

impl ProfileRoute {
    pub fn new() -> Self {
        Self {
            name: None,
            guild: None,
            channel: None,
            thread: None,
            enabled: true,
            model: None,
            system_prompt: None,
            enabled_toolsets: None,
            profile: None,
            allowed_users: None,
        }
    }

    /// Specificity score for hierarchical matching precedence.
    ///
    /// Precedence matches Hermes profile routing:
    /// - `thread` specified: +8 (highest)
    /// - `channel` specified: +4 (middle)
    /// - `guild` specified: +2 (lowest)
    ///
    /// Thus: thread matches (8..14) > channel matches (4..6) > guild matches (2).
    pub fn specificity(&self) -> u32 {
        let mut score = 0;
        if self.guild.is_some() {
            score += 2;
        }
        if self.channel.is_some() {
            score += 4;
        }
        if self.thread.is_some() {
            score += 8;
        }
        score
    }

    /// Checks if this route matches the given target context conjunctively.
    pub fn matches(&self, guild_id: Option<u64>, channel_id: u64, thread_id: Option<u64>) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(req_thread) = self.thread {
            if thread_id != Some(req_thread) {
                return false;
            }
        }
        if let Some(req_channel) = self.channel {
            if channel_id != req_channel {
                return false;
            }
        }
        if let Some(req_guild) = self.guild {
            if guild_id != Some(req_guild) {
                return false;
            }
        }
        true
    }

    /// Applies profile defaults to a session without overwriting explicitly set fields.
    pub fn apply_to_session(&self, session: &mut SessionContext) {
        if session.state.active_model.is_none() {
            if let Some(model) = &self.model {
                session.state.active_model = Some(model.clone());
            }
        }
        if session.state.system_prompt.is_none() {
            if let Some(system_prompt) = &self.system_prompt {
                session.state.system_prompt = Some(system_prompt.clone());
            }
        }
        if let Some(toolsets) = &self.enabled_toolsets {
            if session.state.enabled_toolsets.is_none() {
                session.state.enabled_toolsets = Some(toolsets.clone());
            }
            session
                .state
                .metadata
                .entry("enabled_toolsets".into())
                .or_insert_with(|| serde_json::json!(toolsets));
        }
        if let Some(profile) = &self.profile {
            session
                .state
                .metadata
                .insert("profile".into(), serde_json::Value::String(profile.clone()));
        }
    }
}

impl Default for ProfileRoute {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProfileRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProfileRoute(name={:?}, guild={:?}, channel={:?}, thread={:?}, model={:?})",
            self.name, self.guild, self.channel, self.thread, self.model
        )
    }
}

/// Hierarchical router that matches Discord contexts to profile configurations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileRouter {
    routes: Vec<ProfileRoute>,
}

impl ProfileRouter {
    /// Creates a new `ProfileRouter` with routes sorted by specificity descending.
    pub fn new(routes: Vec<ProfileRoute>) -> Self {
        let mut sorted = routes;
        // Stable sort descending by specificity ensures highest specificity matches first.
        sorted.sort_by_key(|b| std::cmp::Reverse(b.specificity()));
        Self { routes: sorted }
    }

    /// Parses routes from a JSON string.
    pub fn from_json(json_str: &str) -> Self {
        Self::new(parse_profile_routes(json_str))
    }

    pub fn routes(&self) -> &[ProfileRoute] {
        &self.routes
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_user_allowed_for_bot_or_route(
        &self,
        bot_id: Option<&str>,
        channel_id: u64,
        user_id: u64,
        global_allowed_users: &[u64],
    ) -> bool {
        self.is_user_allowed_in_context(
            bot_id,
            None,
            channel_id,
            None,
            None,
            user_id,
            global_allowed_users,
        )
    }

    /// Like [`Self::is_user_allowed_for_bot_or_route`] but resolves the route with the full
    /// Discord context, so a per-route `allowed_users` list on a guild-, thread- or
    /// parent-scoped route is enforced instead of silently skipped.
    #[allow(clippy::too_many_arguments)]
    pub fn is_user_allowed_in_context(
        &self,
        bot_id: Option<&str>,
        guild_id: Option<u64>,
        channel_id: u64,
        thread_id: Option<u64>,
        parent_chat_id: Option<u64>,
        user_id: u64,
        global_allowed_users: &[u64],
    ) -> bool {
        if let Some(route) =
            self.match_route_with_parent(guild_id, channel_id, thread_id, parent_chat_id)
        {
            if let Some(ref allowed) = route.allowed_users {
                return allowed.contains(&user_id);
            }
        }
        if let Some(bot) = bot_id {
            for route in &self.routes {
                if route.name.as_deref() == Some(bot) || route.profile.as_deref() == Some(bot) {
                    if let Some(ref allowed) = route.allowed_users {
                        return allowed.contains(&user_id);
                    }
                }
            }
        }
        if global_allowed_users.is_empty() {
            true
        } else {
            global_allowed_users.contains(&user_id)
        }
    }

    /// Finds the highest-specificity matching route for the given Discord context.
    pub fn match_route(
        &self,
        guild_id: Option<u64>,
        channel_id: u64,
        thread_id: Option<u64>,
    ) -> Option<&ProfileRoute> {
        self.match_route_with_parent(guild_id, channel_id, thread_id, None)
    }

    /// Finds matching route with explicit parent channel awareness.
    /// Follows precedence: thread-specific route > direct channel route > parent-channel route > guild route.
    pub fn match_route_with_parent(
        &self,
        guild_id: Option<u64>,
        channel_id: u64,
        thread_id: Option<u64>,
        parent_chat_id: Option<u64>,
    ) -> Option<&ProfileRoute> {
        // 1. Thread-specific route: route explicitly matching this thread ID
        if let Some(tid) = thread_id {
            if let Some(route) = self.routes.iter().find(|r| {
                r.thread == Some(tid)
                    && (r.channel.is_none()
                        || r.channel == Some(channel_id)
                        || parent_chat_id.is_some_and(|pid| r.channel == Some(pid)))
                    && (r.guild.is_none() || r.guild == guild_id)
                    && r.enabled
            }) {
                return Some(route);
            }
            // A route targeting the thread ID directly as channel (e.g. {"channel": 300})
            if let Some(route) = self.routes.iter().find(|r| {
                r.channel == Some(tid)
                    && r.thread.is_none()
                    && (r.guild.is_none() || r.guild == guild_id)
                    && r.enabled
            }) {
                return Some(route);
            }
        }

        // 2. Direct channel route: route on the given channel_id
        if let Some(route) = self
            .routes
            .iter()
            .find(|r| r.matches(guild_id, channel_id, thread_id))
        {
            return Some(route);
        }
        if thread_id.is_some() {
            if let Some(route) = self.routes.iter().find(|r| {
                r.channel == Some(channel_id)
                    && r.thread.is_none()
                    && (r.guild.is_none() || r.guild == guild_id)
                    && r.enabled
            }) {
                return Some(route);
            }
        }

        // 3. Parent channel route when direct channel route misses
        if let Some(parent_id) = parent_chat_id {
            if parent_id != channel_id {
                if let Some(route) = self.routes.iter().find(|r| {
                    r.channel == Some(parent_id)
                        && r.thread.is_none()
                        && (r.guild.is_none() || r.guild == guild_id)
                        && r.enabled
                }) {
                    return Some(route);
                }
            }
        }

        // 4. Guild-level fallback
        if let Some(gid) = guild_id {
            if let Some(route) = self.routes.iter().find(|r| {
                r.guild == Some(gid) && r.channel.is_none() && r.thread.is_none() && r.enabled
            }) {
                return Some(route);
            }
        }

        None
    }

    /// Alias for `match_route` to match profile-returning API style.
    pub fn match_profile(
        &self,
        guild_id: Option<u64>,
        channel_id: u64,
        thread_id: Option<u64>,
    ) -> Option<&ProfileRoute> {
        self.match_route(guild_id, channel_id, thread_id)
    }

    /// Matches a route against a `SessionKey`.
    pub fn match_session(&self, session_key: &SessionKey) -> Option<&ProfileRoute> {
        let guild_id = session_key
            .guild_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok());
        let channel_id = session_key.channel_id.parse::<u64>().ok()?;
        let thread_id = session_key
            .thread_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok());
        self.match_route(guild_id, channel_id, thread_id)
    }

    /// Matches a route against an `InboundEvent`, taking parent channel into account if present.
    pub fn match_event(&self, event: &InboundEvent) -> Option<&ProfileRoute> {
        let guild_id = event
            .session
            .guild_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok());
        let channel_id = event.session.channel_id.parse::<u64>().ok()?;
        let thread_id = event
            .session
            .thread_id
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok());
        let parent_id = event.parent_chat_id().and_then(|s| s.parse::<u64>().ok());
        self.match_route_with_parent(guild_id, channel_id, thread_id, parent_id)
    }

    /// Applies matching profile overrides to a session context if a route matches.
    pub fn apply_to_session(&self, session: &mut SessionContext) {
        if let Some(route) = self.match_session(&session.key) {
            route.apply_to_session(session);
        }
    }
}

/// Parses profile routes from a JSON string. Returns an empty Vec on missing or invalid input.
pub fn parse_profile_routes(json_str: &str) -> Vec<ProfileRoute> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        Ok(entries) => entries
            .into_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                match serde_json::from_value::<ProfileRoute>(entry) {
                    Ok(route) => Some(route),
                    Err(err) => {
                        // One bad entry must not drop every other route's restrictions.
                        tracing::error!(%err, index, "invalid profile route entry skipped");
                        None
                    }
                }
            })
            .collect(),
        Err(err) => {
            tracing::warn!(%err, raw = %trimmed, "failed to parse profile routes JSON; falling back to empty routes");
            Vec::new()
        }
    }
}

/// Strict variant of [`parse_profile_routes`] for startup validation: empty input is an empty
/// list, but any malformed entry (including negative ids) is an error instead of a silent drop.
pub fn try_parse_profile_routes(json_str: &str) -> Result<Vec<ProfileRoute>, serde_json::Error> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(trimmed)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPromptConfig {
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default, alias = "toolsets")]
    pub skills: Option<Vec<String>>,
}

/// Parses per-channel prompts and skill mappings from a JSON mapping string:
/// `{"channel_id": {"system_prompt": "...", "skills": ["..."]}}` or `{"channel_id": "prompt"}`.
pub fn parse_channel_prompts(json_str: &str) -> Vec<ProfileRoute> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PromptOrConfig {
        Prompt(String),
        Config(ChannelPromptConfig),
    }

    let map: std::result::Result<std::collections::HashMap<String, PromptOrConfig>, _> =
        serde_json::from_str(trimmed);
    match map {
        Ok(entries) => {
            let mut routes = Vec::new();
            for (chan_key, val) in entries {
                let Ok(channel_id) = chan_key.trim().parse::<u64>() else {
                    tracing::warn!(
                        chan_key,
                        "invalid channel ID in DISCORD_CHANNEL_PROMPTS JSON key"
                    );
                    continue;
                };
                let (system_prompt, skills) = match val {
                    PromptOrConfig::Prompt(p) => (Some(p), None),
                    PromptOrConfig::Config(c) => (c.system_prompt, c.skills),
                };
                routes.push(ProfileRoute {
                    name: Some(format!("channel-prompt-{channel_id}")),
                    guild: None,
                    channel: Some(channel_id),
                    thread: None,
                    enabled: true,
                    model: None,
                    system_prompt,
                    enabled_toolsets: skills,
                    profile: None,
                    allowed_users: None,
                });
            }
            routes.sort_by_key(|r| r.channel);
            routes
        }
        Err(err) => {
            tracing::warn!(%err, raw = %trimmed, "failed to parse DISCORD_CHANNEL_PROMPTS JSON; falling back to empty");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_thread_beats_channel_beats_guild() {
        let guild_route = ProfileRoute {
            name: Some("guild-route".into()),
            guild: Some(100),
            channel: None,
            thread: None,
            enabled: true,
            model: Some("guild-model".into()),
            system_prompt: Some("guild-prompt".into()),
            enabled_toolsets: None,
            ..Default::default()
        };
        let channel_route = ProfileRoute {
            name: Some("channel-route".into()),
            guild: Some(100),
            channel: Some(200),
            thread: None,
            enabled: true,
            model: Some("channel-model".into()),
            system_prompt: Some("channel-prompt".into()),
            enabled_toolsets: None,
            ..Default::default()
        };
        let thread_route = ProfileRoute {
            name: Some("thread-route".into()),
            guild: Some(100),
            channel: Some(200),
            thread: Some(300),
            enabled: true,
            model: Some("thread-model".into()),
            system_prompt: Some("thread-prompt".into()),
            enabled_toolsets: None,
            ..Default::default()
        };

        // Insert in arbitrary order
        let router = ProfileRouter::new(vec![
            guild_route.clone(),
            thread_route.clone(),
            channel_route.clone(),
        ]);

        // 1. Thread context: Thread route wins
        let matched = router.match_route(Some(100), 200, Some(300));
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().model.as_deref(), Some("thread-model"));

        // 2. Different thread in channel 200: Channel route wins
        let matched = router.match_route(Some(100), 200, Some(999));
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().model.as_deref(), Some("channel-model"));

        // 3. Direct channel message in channel 200 (no thread): Channel route wins
        let matched = router.match_route(Some(100), 200, None);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().model.as_deref(), Some("channel-model"));

        // 4. Other channel in guild 100: Guild route wins
        let matched = router.match_route(Some(100), 555, None);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().model.as_deref(), Some("guild-model"));

        // 5. Unrelated guild: returns None
        let matched = router.match_route(Some(999), 555, None);
        assert!(matched.is_none());
    }

    #[test]
    fn test_parent_channel_fallback_when_thread_route_misses() {
        let router = ProfileRouter::new(vec![
            ProfileRoute {
                name: Some("parent-channel".into()),
                guild: Some(100),
                channel: Some(200),
                thread: None,
                enabled: true,
                model: Some("model-x".into()),
                system_prompt: Some("parent prompt".into()),
                enabled_toolsets: None,
                ..Default::default()
            },
            ProfileRoute {
                name: Some("specific-thread".into()),
                guild: Some(100),
                channel: Some(200),
                thread: Some(301),
                enabled: true,
                model: Some("specific-model".into()),
                system_prompt: None,
                enabled_toolsets: None,
                ..Default::default()
            },
        ]);

        // Thread 301 gets its specific model
        let matched = router.match_route(Some(100), 200, Some(301)).unwrap();
        assert_eq!(matched.model.as_deref(), Some("specific-model"));

        // Thread 300 (no specific thread route) inherits parent channel 200 route
        let matched = router.match_route(Some(100), 200, Some(300)).unwrap();
        assert_eq!(matched.model.as_deref(), Some("model-x"));
        assert_eq!(matched.system_prompt.as_deref(), Some("parent prompt"));

        // When thread ID 300 is supplied with explicit parent 200
        let matched = router
            .match_route_with_parent(Some(100), 300, Some(300), Some(200))
            .unwrap();
        assert_eq!(matched.model.as_deref(), Some("model-x"));
    }

    #[test]
    fn test_no_match_returns_none() {
        let router = ProfileRouter::new(vec![ProfileRoute {
            name: Some("specific".into()),
            guild: Some(111),
            channel: Some(222),
            thread: Some(333),
            enabled: true,
            model: Some("custom".into()),
            system_prompt: None,
            enabled_toolsets: None,
            ..Default::default()
        }]);

        assert!(router.match_route(Some(999), 888, Some(777)).is_none());
        assert!(router.match_route(None, 888, None).is_none());
    }

    #[test]
    fn test_disabled_route_is_ignored() {
        let router = ProfileRouter::new(vec![ProfileRoute {
            name: Some("disabled".into()),
            guild: Some(100),
            channel: Some(200),
            thread: None,
            enabled: false,
            model: Some("disabled-model".into()),
            system_prompt: None,
            enabled_toolsets: None,
            ..Default::default()
        }]);

        assert!(router.match_route(Some(100), 200, None).is_none());
    }

    #[test]
    fn test_parse_channel_prompts_json() {
        let json = r#"{
            "123456": {
                "system_prompt": "You are a Rust compiler specialist",
                "skills": ["terminal", "file"]
            },
            "789012": "Short system prompt only"
        }"#;

        let routes = parse_channel_prompts(json);
        assert_eq!(routes.len(), 2);

        let r1 = routes.iter().find(|r| r.channel == Some(123456)).unwrap();
        assert_eq!(
            r1.system_prompt.as_deref(),
            Some("You are a Rust compiler specialist")
        );
        assert_eq!(
            r1.enabled_toolsets.as_deref(),
            Some(&["terminal".to_string(), "file".to_string()][..])
        );

        let r2 = routes.iter().find(|r| r.channel == Some(789012)).unwrap();
        assert_eq!(
            r2.system_prompt.as_deref(),
            Some("Short system prompt only")
        );
        assert_eq!(r2.enabled_toolsets, None);

        // Empty string and invalid JSON
        assert!(parse_channel_prompts("").is_empty());
        assert!(parse_channel_prompts("not valid json").is_empty());
    }

    #[test]
    fn test_json_parsing_valid_empty_and_malformed() {
        // Valid JSON with standard keys
        let valid_json = r#"[
            {
                "guild": 123,
                "channel": 456,
                "thread": null,
                "model": "gpt-x",
                "system_prompt": "You are a specialist",
                "toolsets": ["terminal", "web"]
            },
            {
                "guild_id": "789",
                "channel_id": "101",
                "thread_id": "202",
                "model": "claude-3-5-sonnet",
                "enabled_toolsets": ["file", "cron"]
            }
        ]"#;
        let routes = parse_profile_routes(valid_json);
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].guild, Some(123));
        assert_eq!(routes[0].channel, Some(456));
        assert_eq!(routes[0].thread, None);
        assert_eq!(routes[0].model.as_deref(), Some("gpt-x"));
        assert_eq!(
            routes[0].system_prompt.as_deref(),
            Some("You are a specialist")
        );
        assert_eq!(
            routes[0].enabled_toolsets.as_deref(),
            Some(&["terminal".to_string(), "web".to_string()][..])
        );

        assert_eq!(routes[1].guild, Some(789));
        assert_eq!(routes[1].channel, Some(101));
        assert_eq!(routes[1].thread, Some(202));
        assert_eq!(routes[1].model.as_deref(), Some("claude-3-5-sonnet"));
        assert_eq!(
            routes[1].enabled_toolsets.as_deref(),
            Some(&["file".to_string(), "cron".to_string()][..])
        );

        // Empty / whitespace strings return empty list
        assert!(parse_profile_routes("").is_empty());
        assert!(parse_profile_routes("   ").is_empty());
        assert!(parse_profile_routes("[]").is_empty());

        // Malformed JSON logs warning and returns empty list without panicking
        assert!(parse_profile_routes("{not-valid-json").is_empty());
        assert!(parse_profile_routes(r#"{"not_an_array": true}"#).is_empty());
    }

    #[test]
    fn test_apply_to_session_populates_fresh_session() {
        let route = ProfileRoute {
            name: Some("test-profile".into()),
            guild: Some(10),
            channel: Some(20),
            thread: None,
            enabled: true,
            model: Some("gpt-routed".into()),
            system_prompt: Some("Custom system prompt".into()),
            enabled_toolsets: Some(vec!["terminal".into(), "web".into()]),
            ..Default::default()
        };

        let key = SessionKey::new("discord", Some("10"), "20", None::<String>, "user-1");
        let mut session = SessionContext::new(key);
        assert_eq!(session.state.active_model, None);
        assert_eq!(session.state.system_prompt, None);
        assert_eq!(session.state.enabled_toolsets, None);

        route.apply_to_session(&mut session);

        assert_eq!(session.state.active_model.as_deref(), Some("gpt-routed"));
        assert_eq!(
            session.state.system_prompt.as_deref(),
            Some("Custom system prompt")
        );
        assert_eq!(
            session.state.enabled_toolsets.as_deref(),
            Some(&["terminal".to_string(), "web".to_string()][..])
        );
        assert_eq!(
            session.state.metadata.get("enabled_toolsets"),
            Some(&serde_json::json!(["terminal", "web"]))
        );
    }

    #[test]
    fn test_apply_to_session_preserves_explicitly_set_model() {
        let route = ProfileRoute {
            name: Some("test-profile".into()),
            guild: Some(10),
            channel: Some(20),
            thread: None,
            enabled: true,
            model: Some("gpt-routed".into()),
            system_prompt: Some("Custom system prompt".into()),
            enabled_toolsets: Some(vec!["terminal".into()]),
            ..Default::default()
        };

        let key = SessionKey::new("discord", Some("10"), "20", None::<String>, "user-1");
        let mut session = SessionContext::new(key);
        // Explicitly set active_model (e.g. from `/model claude-3-opus`)
        session.state.active_model = Some("claude-3-opus".into());

        route.apply_to_session(&mut session);

        // Explicit model must NOT be clobbered by the profile route
        assert_eq!(session.state.active_model.as_deref(), Some("claude-3-opus"));
        // But unset system prompt and toolsets should still be populated
        assert_eq!(
            session.state.system_prompt.as_deref(),
            Some("Custom system prompt")
        );
        assert_eq!(
            session.state.enabled_toolsets.as_deref(),
            Some(&["terminal".to_string()][..])
        );
    }

    #[test]
    fn test_match_session_key_resolution() {
        let router = ProfileRouter::new(vec![
            ProfileRoute {
                name: Some("channel-profile".into()),
                guild: Some(1234),
                channel: Some(5678),
                thread: None,
                enabled: true,
                model: Some("model-a".into()),
                system_prompt: None,
                enabled_toolsets: None,
                ..Default::default()
            },
            ProfileRoute {
                name: Some("thread-profile".into()),
                guild: Some(1234),
                channel: Some(5678),
                thread: Some(9999),
                enabled: true,
                model: Some("model-b".into()),
                system_prompt: None,
                enabled_toolsets: None,
                ..Default::default()
            },
        ]);

        let key_thread = SessionKey::new("discord", Some("1234"), "5678", Some("9999"), "user-1");
        let matched = router.match_session(&key_thread);
        assert_eq!(matched.unwrap().model.as_deref(), Some("model-b"));

        let key_channel =
            SessionKey::new("discord", Some("1234"), "5678", None::<String>, "user-1");
        let matched = router.match_session(&key_channel);
        assert_eq!(matched.unwrap().model.as_deref(), Some("model-a"));

        let key_other =
            SessionKey::new("discord", Some("1234"), "999999", None::<String>, "user-1");
        assert!(router.match_session(&key_other).is_none());
    }

    #[test]
    fn negative_ids_are_rejected_instead_of_widening_the_route() {
        assert!(serde_json::from_str::<ProfileRoute>(r#"{"channel": -5}"#).is_err());
        assert!(serde_json::from_str::<ProfileRoute>(r#"{"guild": -1}"#).is_err());
        assert!(try_parse_profile_routes(r#"[{"thread": -2}]"#).is_err());
        assert_eq!(
            try_parse_profile_routes(r#"[{"channel": 5}]"#).unwrap()[0].channel,
            Some(5)
        );
        assert!(try_parse_profile_routes("  ").unwrap().is_empty());
    }

    #[test]
    fn one_bad_route_entry_does_not_drop_the_other_routes() {
        let routes = parse_profile_routes(
            r#"[
                {"channel": -5, "enabled_toolsets": ["web"]},
                {"channel": 6, "allowed_users": [1]}
            ]"#,
        );
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].channel, Some(6));
        assert_eq!(routes[0].allowed_users, Some(vec![1]));
    }

    #[test]
    fn allowlist_of_guild_scoped_route_is_enforced_with_full_context() {
        let router =
            ProfileRouter::from_json(r#"[{"guild": 10, "channel": 20, "allowed_users": [2]}]"#);
        let global = vec![1];
        // The legacy guild-less lookup cannot see a guild-scoped route.
        assert!(router.is_user_allowed_for_bot_or_route(None, 20, 1, &global));
        // With the guild known, the per-route allowlist applies and denies user 1.
        assert!(!router.is_user_allowed_in_context(None, Some(10), 20, None, None, 1, &global));
        assert!(router.is_user_allowed_in_context(None, Some(10), 20, None, None, 2, &global));
    }
}

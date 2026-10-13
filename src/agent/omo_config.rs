use crate::{OmonError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

pub fn validate_agent_backend_value(value: Option<&str>) -> Result<()> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(()),
        Some(v)
            if v.eq_ignore_ascii_case("omo")
                || v.eq_ignore_ascii_case("omo-appserver")
                || v.eq_ignore_ascii_case("appserver") =>
        {
            Ok(())
        }
        Some(v)
            if v.eq_ignore_ascii_case("llm")
                || v.eq_ignore_ascii_case("hermes")
                || v.eq_ignore_ascii_case("direct") =>
        {
            Err(OmonError::Config(
                "direct LLM backend has been removed; only 'omo' (app-server) backend is supported"
                    .to_string(),
            ))
        }
        Some(invalid) => Err(OmonError::Config(format!(
            "invalid OMON_AGENT_BACKEND '{invalid}': direct LLM backend has been removed and only 'omo' is supported"
        ))),
    }
}

pub fn validate_agent_backend_env() -> Result<()> {
    let val = std::env::var("OMON_AGENT_BACKEND").ok();
    validate_agent_backend_value(val.as_deref())
}

/// Configuration for the OMO app-server WebSocket daemon backend.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmoBackendConfig {
    pub appserver_url: String,
    /// Bearer token for the app-server (grants tool execution): never serialized.
    #[serde(default, skip_serializing)]
    pub auth_token: Option<String>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub total_timeout: Duration,
    /// Grace window during which a content-less terminal frame is treated as
    /// the premature-completion race and ignored. Past it, an empty terminal
    /// fails the turn (an upstream LLM failure recorded as an empty success).
    pub no_content_grace: Duration,
    /// Interactive turns are interrupted when the agent produces no output at all
    /// within this window of the prompt starting (turn/start acknowledgement).
    pub initial_response_timeout: Duration,
    pub default_model: Option<String>,
    pub fallback_model: String,
    pub per_agent_workspace: bool,
    pub workspace_root: Option<PathBuf>,
}

impl std::fmt::Debug for OmoBackendConfig {
    /// Redacts `auth_token` so tracing output and panics never carry the secret.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OmoBackendConfig")
            .field("appserver_url", &self.appserver_url)
            .field("auth_token", &self.auth_token.as_ref().map(|_| "***"))
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("total_timeout", &self.total_timeout)
            .field("no_content_grace", &self.no_content_grace)
            .field("initial_response_timeout", &self.initial_response_timeout)
            .field("default_model", &self.default_model)
            .field("fallback_model", &self.fallback_model)
            .field("per_agent_workspace", &self.per_agent_workspace)
            .field("workspace_root", &self.workspace_root)
            .finish()
    }
}

/// Default local daemon URL when neither lane configures an endpoint.
pub const CRON_APPSERVER_URL_DEFAULT: &str = "ws://127.0.0.1:19742";

/// Default turn budget for lane sessions when OMON_LANE_TURN_SECS is unset.
pub const DEFAULT_LANE_TURN_SECS: u64 = 3600;

/// Default fallback model when primary model fails due to quota or daemon model rejection.
pub const DEFAULT_FALLBACK_MODEL: &str = "inferhub/cb/deepseek-v4.1-flash";

/// Fail-fast window for an interactive turn whose agent produces no output.
pub const DEFAULT_INITIAL_RESPONSE_TIMEOUT: Duration = Duration::from_secs(45);

/// Default primary model when neither OMON_DEFAULT_MODEL nor DEFAULT_MODEL is set.
pub const DEFAULT_PRIMARY_MODEL: &str = "mahoquot/nekos-claude-sonnet-5-5";

/// Default model for the owner-facing lead session.
pub const OMON_MODEL_LEAD: &str = "mahoquot/claude-opus-5-5";

/// Default model for large work and large lanes.
pub const OMON_MODEL_LARGE: &str = "mahoquot/claude-opus-5-5";

/// Default model for small work, small lanes, kibitzer, triage writing, digests, judge fallback.
pub const OMON_MODEL_SMALL: &str = "inferhub/cb/deepseek-v4.1-flash";

pub const OMON_MODEL_LEAD_DEFAULT: &str = OMON_MODEL_LEAD;
pub const OMON_MODEL_LARGE_DEFAULT: &str = OMON_MODEL_LARGE;
pub const OMON_MODEL_SMALL_DEFAULT: &str = OMON_MODEL_SMALL;

/// Purpose for which a model is requested in the gateway architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
    Lead,
    Large,
    Small,
    LaneLarge,
    LaneSmall,
    Kibitzer,
    InboxWrite,
    Digest,
    JudgeFallback,
    CronFix,
}

/// Resolves the model identifier for a given purpose, honoring environment overrides
/// (OMON_MODEL_LEAD, OMON_MODEL_LARGE, OMON_MODEL_SMALL) or falling back to compiled-in defaults.
pub fn model_for_purpose(purpose: ModelPurpose) -> String {
    match purpose {
        ModelPurpose::Lead => std::env::var("OMON_MODEL_LEAD")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| OMON_MODEL_LEAD.to_string()),
        ModelPurpose::Large | ModelPurpose::LaneLarge => std::env::var("OMON_MODEL_LARGE")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| OMON_MODEL_LARGE.to_string()),
        ModelPurpose::Small
        | ModelPurpose::LaneSmall
        | ModelPurpose::Kibitzer
        | ModelPurpose::InboxWrite
        | ModelPurpose::Digest
        | ModelPurpose::JudgeFallback
        | ModelPurpose::CronFix => std::env::var("OMON_MODEL_SMALL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| OMON_MODEL_SMALL.to_string()),
    }
}

/// `OMON_WORKSPACE_ROOT` when set and non-empty, else `$HOME/.omon/workspace`;
/// `None` only when neither is available.
fn default_workspace_root(
    configured: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    configured
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|home| !home.is_empty())
                .map(|home| PathBuf::from(home).join(".omon").join("workspace"))
        })
}

impl Default for OmoBackendConfig {
    fn default() -> Self {
        Self {
            appserver_url: CRON_APPSERVER_URL_DEFAULT.to_string(),
            auth_token: None,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(1800),
            total_timeout: Duration::from_secs(1800),
            no_content_grace: Duration::from_secs(15),
            initial_response_timeout: DEFAULT_INITIAL_RESPONSE_TIMEOUT,
            default_model: None,
            fallback_model: DEFAULT_FALLBACK_MODEL.to_string(),
            per_agent_workspace: true,
            workspace_root: None,
        }
    }
}

impl OmoBackendConfig {
    pub fn new(appserver_url: impl Into<String>) -> Self {
        Self {
            appserver_url: appserver_url.into(),
            ..Default::default()
        }
    }

    pub fn with_auth_token(mut self, auth_token: Option<impl Into<String>>) -> Self {
        self.auth_token = auth_token.map(Into::into);
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn with_total_timeout(mut self, timeout: Duration) -> Self {
        self.total_timeout = timeout;
        self
    }

    pub fn with_no_content_grace(mut self, grace: Duration) -> Self {
        self.no_content_grace = grace;
        self
    }

    pub fn with_initial_response_timeout(mut self, timeout: Duration) -> Self {
        self.initial_response_timeout = timeout;
        self
    }

    pub fn with_default_model(mut self, default_model: Option<impl Into<String>>) -> Self {
        self.default_model = default_model.map(Into::into);
        self
    }

    pub fn with_fallback_model(mut self, fallback_model: impl Into<String>) -> Self {
        self.fallback_model = fallback_model.into();
        self
    }

    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace_root.into());
        self
    }

    pub fn with_per_agent_workspace(mut self, per_agent_workspace: bool) -> Self {
        self.per_agent_workspace = per_agent_workspace;
        self
    }

    pub fn validate_url(url: &str) -> Result<()> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(OmonError::Config(
                "OMON_OMO_APPSERVER_URL is required when OMON_AGENT_BACKEND is 'omo'".to_string(),
            ));
        }
        if !trimmed.starts_with("ws://") && !trimmed.starts_with("wss://") {
            return Err(OmonError::Config(format!(
                "invalid OMON_OMO_APPSERVER_URL '{trimmed}': must start with ws:// or wss://"
            )));
        }
        Ok(())
    }

    pub fn from_env() -> Result<Self> {
        // Zero-config default: when the env var is absent, target the local
        // daemon URL that the supervisor (see omo_daemon) auto-spawns.
        let appserver_url = std::env::var("OMON_OMO_APPSERVER_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "ws://127.0.0.1:19742".to_string());

        Self::validate_url(&appserver_url)?;

        // Idle-gap tolerance between streamed daemon events; long agent turns
        // (digests, refactors) need minutes, not seconds. 기본 1800s(30분)로
        // 총 상한과 맞춰, 조용한 구간 때문에 총 상한보다 먼저 끊기는 일이 없게 한다.
        let request_timeout = match std::env::var("OMON_OMO_TURN_TIMEOUT_SECS") {
            Ok(v) if !v.trim().is_empty() => Duration::from_secs(
                v.trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|secs| *secs > 0)
                    .ok_or_else(|| {
                        OmonError::Config(format!(
                        "invalid OMON_OMO_TURN_TIMEOUT_SECS: '{v}', expected a positive integer"
                    ))
                    })?,
            ),
            _ => Duration::from_secs(1800),
        };

        // Hard ceiling for a whole turn: a looping agent keeps emitting
        // events, so the per-event gap timeout never fires. On deadline the
        // backend sends turn/interrupt so the daemon thread is freed.
        // 기본 1800s(30분): 대화형 턴은 INTERACTIVE_TURN_CAP(1800s)까지 허용한다.
        let total_timeout = match std::env::var("OMON_OMO_TURN_TOTAL_TIMEOUT_SECS") {
            Ok(v) if !v.trim().is_empty() => Duration::from_secs(
                v.trim().parse::<u64>().ok().filter(|secs| *secs > 0).ok_or_else(|| {
                    OmonError::Config(format!(
                        "invalid OMON_OMO_TURN_TOTAL_TIMEOUT_SECS: '{v}', expected a positive integer"
                    ))
                })?,
            ),
            _ => Duration::from_secs(1800),
        };

        let auth_token = std::env::var("OMON_OMO_APPSERVER_AUTH_TOKEN")
            .or_else(|_| std::env::var("OMON_OMO_AUTH_TOKEN"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let default_model = std::env::var("OMON_DEFAULT_MODEL")
            .or_else(|_| std::env::var("DEFAULT_MODEL"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| Some(DEFAULT_PRIMARY_MODEL.to_string()));

        let fallback_model = std::env::var("OMON_OMO_FALLBACK_MODEL")
            .or_else(|_| std::env::var("FALLBACK_MODEL"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_FALLBACK_MODEL.to_string());

        let per_agent_workspace = match std::env::var("OMON_PER_AGENT_WORKSPACE") {
            Ok(v) => {
                let trimmed = v.trim().to_ascii_lowercase();
                if trimmed.is_empty() {
                    true
                } else {
                    !matches!(trimmed.as_str(), "false" | "0" | "off" | "no")
                }
            }
            Err(_) => true,
        };

        // Same zero-config default as the main runtime (`~/.omon/workspace`), so the
        // lane, kibitzer, triage and judge-fallback backends that build their config
        // straight from the environment get per-agent workspaces too.
        let workspace_root = default_workspace_root(
            std::env::var_os("OMON_WORKSPACE_ROOT"),
            std::env::var_os("HOME"),
        );

        Ok(Self {
            appserver_url,
            auth_token,
            connect_timeout: Duration::from_secs(15),
            request_timeout,
            total_timeout,
            no_content_grace: Duration::from_secs(15),
            initial_response_timeout: DEFAULT_INITIAL_RESPONSE_TIMEOUT,
            default_model,
            fallback_model,
            per_agent_workspace,
            workspace_root,
        })
    }

    /// Configuration for the **cron lane**.
    ///
    /// Cron shares the configured interactive daemon through distinct agent
    /// threads unless an explicit cron endpoint is supplied. It inherits the
    /// interactive credentials and model, with a tighter turn ceiling.
    pub fn cron_from_env() -> Result<Self> {
        let mut config = Self::from_env()?;

        if let Some(appserver_url) = std::env::var("OMON_OMO_CRON_APPSERVER_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            config.appserver_url = appserver_url;
        }
        Self::validate_url(&config.appserver_url)?;

        config.total_timeout = match std::env::var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS") {
            Ok(v) if !v.trim().is_empty() => Duration::from_secs(v.trim().parse::<u64>().ok().filter(|secs| *secs > 0).ok_or_else(|| {
                OmonError::Config(format!(
                    "invalid OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS: '{v}', expected a positive integer"
                ))
            })?),
            _ => Duration::from_secs(600),
        };

        Ok(config)
    }

    /// Configuration for the **lane runner**.
    ///
    /// Lane turns inherit the interactive daemon credentials and model, with
    /// a dedicated total timeout parsed from `OMON_LANE_TURN_SECS` (default 3600s).
    pub fn lane_from_env() -> Result<Self> {
        let mut config = Self::from_env()?;
        config.total_timeout = lane_turn_timeout_from_env()?;
        Ok(config)
    }
}

/// Parse `OMON_LANE_TURN_SECS` with zero-config default of 3600s (1 hour).
pub fn lane_turn_timeout_from_env() -> Result<Duration> {
    match std::env::var("OMON_LANE_TURN_SECS") {
        Ok(v) if !v.trim().is_empty() => {
            let secs = v
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|secs| *secs > 0)
                .ok_or_else(|| {
                    OmonError::Config(format!(
                        "invalid OMON_LANE_TURN_SECS: '{v}', expected a positive integer"
                    ))
                })?;
            Ok(Duration::from_secs(secs))
        }
        _ => Ok(Duration::from_secs(DEFAULT_LANE_TURN_SECS)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_turn_stream_timeout_env_contract() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Default: the gap tolerance matches the whole-turn ceiling (30 min)
        std::env::remove_var("OMON_OMO_TURN_TIMEOUT_SECS");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert_eq!(cfg.request_timeout, Duration::from_secs(1800));

        // Explicit override
        std::env::set_var("OMON_OMO_TURN_TIMEOUT_SECS", "300");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert_eq!(cfg.request_timeout, Duration::from_secs(300));

        // Invalid value fails boot (fail-fast convention)
        std::env::set_var("OMON_OMO_TURN_TIMEOUT_SECS", "soon");
        assert!(OmoBackendConfig::from_env().is_err());
        std::env::remove_var("OMON_OMO_TURN_TIMEOUT_SECS");
    }

    #[test]
    fn test_fallback_model_env_contract() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OMON_OMO_FALLBACK_MODEL");
        std::env::remove_var("FALLBACK_MODEL");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert_eq!(cfg.fallback_model, "inferhub/cb/deepseek-v4.1-flash");

        std::env::set_var("OMON_OMO_FALLBACK_MODEL", "custom/fallback-model");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert_eq!(cfg.fallback_model, "custom/fallback-model");

        std::env::remove_var("OMON_OMO_FALLBACK_MODEL");
        std::env::remove_var("FALLBACK_MODEL");
    }

    #[test]
    fn test_validate_agent_backend_env_contract() {
        // Default (absent or empty) succeeds as Omo backend
        assert!(validate_agent_backend_value(None).is_ok());
        assert!(validate_agent_backend_value(Some("")).is_ok());
        assert!(validate_agent_backend_value(Some("   ")).is_ok());

        // Supported OMO aliases succeed
        assert!(validate_agent_backend_value(Some("omo")).is_ok());
        assert!(validate_agent_backend_value(Some("omo-appserver")).is_ok());
        assert!(validate_agent_backend_value(Some("appserver")).is_ok());
        assert!(validate_agent_backend_value(Some("OMO")).is_ok());

        // Legacy direct-LLM aliases return Config error stating removal
        let err_llm = validate_agent_backend_value(Some("llm")).unwrap_err();
        assert!(
            matches!(err_llm, OmonError::Config(ref msg) if msg.contains("direct LLM backend has been removed") && msg.contains("omo")),
            "Expected Config error for 'llm', got {err_llm:?}"
        );
        let err_hermes = validate_agent_backend_value(Some("hermes")).unwrap_err();
        assert!(
            matches!(err_hermes, OmonError::Config(ref msg) if msg.contains("direct LLM backend has been removed") && msg.contains("omo")),
            "Expected Config error for 'hermes', got {err_hermes:?}"
        );
        let err_direct = validate_agent_backend_value(Some("direct")).unwrap_err();
        assert!(
            matches!(err_direct, OmonError::Config(ref msg) if msg.contains("direct LLM backend has been removed") && msg.contains("omo")),
            "Expected Config error for 'direct', got {err_direct:?}"
        );

        // Unknown backend returns Config error
        let err_unknown = validate_agent_backend_value(Some("unknown_backend")).unwrap_err();
        assert!(
            matches!(err_unknown, OmonError::Config(ref msg) if msg.contains("invalid OMON_AGENT_BACKEND")),
            "Expected Config error for unknown backend, got {err_unknown:?}"
        );
    }

    #[test]
    fn test_omo_backend_config_validates_websocket_scheme() {
        let err = OmoBackendConfig::validate_url("http://127.0.0.1:19742").unwrap_err();
        assert!(
            matches!(err, OmonError::Config(ref msg) if msg.contains("must start with ws:// or wss://")),
            "Expected Config error for invalid scheme, got {err:?}"
        );

        let err_empty = OmoBackendConfig::validate_url("   ").unwrap_err();
        assert!(
            matches!(err_empty, OmonError::Config(ref msg) if msg.contains("OMON_OMO_APPSERVER_URL is required")),
            "Expected Config error for empty url, got {err_empty:?}"
        );

        assert!(OmoBackendConfig::validate_url("ws://127.0.0.1:19742").is_ok());
        assert!(OmoBackendConfig::validate_url("wss://example.com/ws").is_ok());
    }

    #[test]
    fn test_cron_from_env_shares_multiplexed_daemon() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OMON_OMO_APPSERVER_URL");
        std::env::remove_var("OMON_OMO_CRON_APPSERVER_URL");
        std::env::remove_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS");

        let interactive = OmoBackendConfig::from_env().unwrap();
        let cron = OmoBackendConfig::cron_from_env().unwrap();

        // Cron and interactive share one multiplexed daemon instance by default.
        assert_eq!(interactive.appserver_url, "ws://127.0.0.1:19742");
        assert_eq!(cron.appserver_url, "ws://127.0.0.1:19742");
        assert_eq!(cron.appserver_url, interactive.appserver_url);

        // Cron turns get a tighter ceiling than interactive turns.
        assert_eq!(cron.total_timeout, Duration::from_secs(600));
        assert!(cron.total_timeout < interactive.total_timeout);
    }

    #[test]
    fn test_cron_from_env_honours_overrides_and_inherits_credentials() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OMON_OMO_CRON_APPSERVER_URL", "ws://127.0.0.1:29999");
        std::env::set_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS", "120");
        std::env::set_var("OMON_OMO_APPSERVER_AUTH_TOKEN", "shared-token");
        std::env::set_var("OMON_DEFAULT_MODEL", "glm-5.3-flash");

        let cron = OmoBackendConfig::cron_from_env().unwrap();
        assert_eq!(cron.appserver_url, "ws://127.0.0.1:29999");
        assert_eq!(cron.total_timeout, Duration::from_secs(120));
        assert_eq!(cron.auth_token.as_deref(), Some("shared-token"));
        assert_eq!(cron.default_model.as_deref(), Some("glm-5.3-flash"));

        // Invalid override fails boot, consistent with the other timeouts.
        std::env::set_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS", "later");
        assert!(OmoBackendConfig::cron_from_env().is_err());

        std::env::remove_var("OMON_OMO_CRON_APPSERVER_URL");
        std::env::remove_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS");
        std::env::remove_var("OMON_OMO_APPSERVER_AUTH_TOKEN");
        std::env::remove_var("OMON_DEFAULT_MODEL");
    }

    #[test]
    fn test_from_env_defaults_to_local_daemon_when_env_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OMON_OMO_APPSERVER_URL");

        // The gateway must be zero-config: with no env var, the backend targets
        // the local default daemon URL that the supervisor auto-spawns.
        let config = OmoBackendConfig::from_env()
            .expect("from_env must succeed with default local daemon URL");
        assert_eq!(config.appserver_url, "ws://127.0.0.1:19742");
    }

    #[test]
    fn test_omo_backend_config_from_env_parses_url_and_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OMON_OMO_APPSERVER_URL", "ws://127.0.0.1:19742");
        std::env::set_var("OMON_OMO_APPSERVER_AUTH_TOKEN", "secret-token-123");
        std::env::set_var("OMON_DEFAULT_MODEL", "claude-3-5-sonnet");

        let config = OmoBackendConfig::from_env().unwrap();
        assert_eq!(config.appserver_url, "ws://127.0.0.1:19742");
        assert_eq!(config.auth_token.as_deref(), Some("secret-token-123"));
        assert_eq!(config.default_model.as_deref(), Some("claude-3-5-sonnet"));

        std::env::remove_var("OMON_OMO_APPSERVER_URL");
        std::env::remove_var("OMON_OMO_APPSERVER_AUTH_TOKEN");
        std::env::remove_var("OMON_DEFAULT_MODEL");
    }

    #[test]
    fn test_per_agent_workspace_and_workspace_root_env_parsing() {
        let _guard = ENV_LOCK.lock().unwrap();

        // Default when unset
        std::env::remove_var("OMON_PER_AGENT_WORKSPACE");
        std::env::remove_var("OMON_WORKSPACE_ROOT");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert!(cfg.per_agent_workspace);
        assert_eq!(
            cfg.workspace_root,
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".omon").join("workspace")),
            "unset OMON_WORKSPACE_ROOT must fall back to ~/.omon/workspace"
        );

        // Workspace root parsed
        std::env::set_var("OMON_WORKSPACE_ROOT", "/tmp/omon-test-ws");
        let cfg = OmoBackendConfig::from_env().unwrap();
        assert_eq!(cfg.workspace_root, Some(PathBuf::from("/tmp/omon-test-ws")));

        // Kill-switch: false / 0 / off / no (case-insensitive)
        for falsy in ["false", "False", "0", "off", "OFF", "no", "NO"] {
            std::env::set_var("OMON_PER_AGENT_WORKSPACE", falsy);
            let cfg = OmoBackendConfig::from_env().unwrap();
            assert!(
                !cfg.per_agent_workspace,
                "Expected per_agent_workspace=false for '{falsy}'"
            );
        }

        // Truthy values
        for truthy in ["true", "True", "1", "on", "yes", "YES", ""] {
            std::env::set_var("OMON_PER_AGENT_WORKSPACE", truthy);
            let cfg = OmoBackendConfig::from_env().unwrap();
            assert!(
                cfg.per_agent_workspace,
                "Expected per_agent_workspace=true for '{truthy}'"
            );
        }

        std::env::remove_var("OMON_PER_AGENT_WORKSPACE");
        std::env::remove_var("OMON_WORKSPACE_ROOT");
    }

    #[test]
    fn fixr5_default_workspace_root_precedence_and_empty_rejection() {
        use std::ffi::OsString;
        let home = Some(OsString::from("/home/u"));
        assert_eq!(
            default_workspace_root(Some("/srv/ws".into()), home.clone()),
            Some(PathBuf::from("/srv/ws"))
        );
        // An empty override would resolve to the cwd; it must fall back instead.
        assert_eq!(
            default_workspace_root(Some("".into()), home.clone()),
            Some(PathBuf::from("/home/u/.omon/workspace"))
        );
        assert_eq!(
            default_workspace_root(None, home),
            Some(PathBuf::from("/home/u/.omon/workspace"))
        );
        assert_eq!(default_workspace_root(None, None), None);
    }

    #[test]
    fn fixr5_backend_config_debug_and_serialize_hide_auth_token() {
        let config =
            OmoBackendConfig::new("ws://127.0.0.1:1").with_auth_token(Some("super-secret-token"));
        assert!(!format!("{config:?}").contains("super-secret-token"));
        assert!(format!("{config:?}").contains("ws://127.0.0.1:1"));
        assert!(!serde_json::to_string(&config)
            .unwrap()
            .contains("super-secret-token"));
    }

    #[test]
    fn test_lane_turn_timeout_env_contract() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OMON_LANE_TURN_SECS");
        assert_eq!(
            lane_turn_timeout_from_env().unwrap(),
            Duration::from_secs(3600)
        );

        std::env::set_var("OMON_LANE_TURN_SECS", "7200");
        assert_eq!(
            lane_turn_timeout_from_env().unwrap(),
            Duration::from_secs(7200)
        );

        let lane_cfg = OmoBackendConfig::lane_from_env().unwrap();
        assert_eq!(lane_cfg.total_timeout, Duration::from_secs(7200));

        std::env::set_var("OMON_LANE_TURN_SECS", "not_a_number");
        assert!(lane_turn_timeout_from_env().is_err());
        assert!(OmoBackendConfig::lane_from_env().is_err());

        std::env::remove_var("OMON_LANE_TURN_SECS");
    }

    #[test]
    fn fixr1_zero_timeouts_are_rejected() {
        let _guard = ENV_LOCK.lock().unwrap();
        for var in [
            "OMON_OMO_TURN_TIMEOUT_SECS",
            "OMON_OMO_TURN_TOTAL_TIMEOUT_SECS",
            "OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS",
            "OMON_LANE_TURN_SECS",
        ] {
            std::env::remove_var(var);
        }

        std::env::set_var("OMON_OMO_TURN_TIMEOUT_SECS", "0");
        assert!(OmoBackendConfig::from_env().is_err());
        std::env::remove_var("OMON_OMO_TURN_TIMEOUT_SECS");

        std::env::set_var("OMON_OMO_TURN_TOTAL_TIMEOUT_SECS", "0");
        assert!(OmoBackendConfig::from_env().is_err());
        std::env::remove_var("OMON_OMO_TURN_TOTAL_TIMEOUT_SECS");

        std::env::set_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS", "0");
        assert!(OmoBackendConfig::cron_from_env().is_err());
        std::env::remove_var("OMON_OMO_CRON_TURN_TOTAL_TIMEOUT_SECS");

        std::env::set_var("OMON_LANE_TURN_SECS", "0");
        assert!(lane_turn_timeout_from_env().is_err());
        std::env::remove_var("OMON_LANE_TURN_SECS");
    }
}

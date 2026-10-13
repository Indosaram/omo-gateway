use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub const DISK_DEGRADED_PERCENT: f64 = 90.0;
pub const DISK_BYTES_PER_MB: u64 = 1024 * 1024;
pub const DISK_CRITICAL_FREE_FLOOR_MB: u64 = 256;
pub const DISK_CRITICAL_PERCENT_FLOOR: f64 = 95.0;
pub const DISK_CRITICAL_HEADROOM_MB: u64 = 1024;
pub const DISK_ELEVATED_FREE_FLOOR_MB: u64 = 512;
pub const DISK_ELEVATED_PERCENT_FLOOR: f64 = 85.0;
pub const DISK_ELEVATED_HEADROOM_MB: u64 = 4096;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metrics: HashMap<String, serde_json::Value>,
}

impl CheckResult {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            detail: None,
            metrics: HashMap::new(),
        }
    }

    pub fn ok_with_detail(detail: impl Into<String>) -> Self {
        Self {
            status: "ok".to_string(),
            detail: Some(detail.into()),
            metrics: HashMap::new(),
        }
    }

    pub fn degraded(detail: impl Into<String>) -> Self {
        Self {
            status: "degraded".to_string(),
            detail: Some(detail.into()),
            metrics: HashMap::new(),
        }
    }

    pub fn with_metric(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metrics.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub status: String,
    pub checks: HashMap<String, CheckResult>,
}

impl ReadinessReport {
    pub fn is_ok(&self) -> bool {
        self.status == "ok"
    }
}

/// Classifies disk pressure from total and free byte counts into "ok", "elevated", "critical", or "unknown".
pub fn classify_disk_pressure(total_bytes: u64, free_bytes: u64) -> &'static str {
    classify_disk_pressure_opt(Some(total_bytes), Some(free_bytes))
}

/// Classifies disk pressure from optional total and free byte counts.
///
/// Unreadable or zero-capacity samples return "unknown" so unusable filesystems are never reported as healthy.
pub fn classify_disk_pressure_opt(
    total_bytes: Option<u64>,
    free_bytes: Option<u64>,
) -> &'static str {
    let (Some(total), Some(free)) = (total_bytes, free_bytes) else {
        return "unknown";
    };
    if total == 0 || free > total {
        return "unknown";
    }
    let total_mb = total / DISK_BYTES_PER_MB;
    let free_mb = free / DISK_BYTES_PER_MB;
    if total_mb == 0 || free_mb > total_mb {
        return "unknown";
    }
    let used_percent = (1.0 - (free_mb as f64 / total_mb as f64)) * 100.0;
    if free_mb < DISK_CRITICAL_FREE_FLOOR_MB
        || (used_percent >= DISK_CRITICAL_PERCENT_FLOOR && free_mb < DISK_CRITICAL_HEADROOM_MB)
    {
        "critical"
    } else if free_mb < DISK_ELEVATED_FREE_FLOOR_MB
        || (used_percent >= DISK_ELEVATED_PERCENT_FLOOR && free_mb < DISK_ELEVATED_HEADROOM_MB)
    {
        "elevated"
    } else {
        "ok"
    }
}

/// Calculates disk headroom status, used percentage, and pressure classification from total and free byte counts.
pub fn calculate_disk_headroom(
    total_bytes: u64,
    free_bytes: u64,
    _threshold_pct: f64,
) -> (String, f64, String) {
    if total_bytes == 0 || free_bytes > total_bytes {
        return ("degraded".to_string(), 0.0, "unknown".to_string());
    }
    let pressure = classify_disk_pressure(total_bytes, free_bytes);
    if pressure == "unknown" {
        return ("degraded".to_string(), 0.0, "unknown".to_string());
    }
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    let used_pct = (used_bytes as f64 / total_bytes as f64) * 100.0;
    let rounded_pct = (used_pct * 10.0).round() / 10.0;
    let status = if pressure == "ok" {
        "ok".to_string()
    } else {
        "degraded".to_string()
    };
    (status, rounded_pct, pressure.to_string())
}

/// Probes SQLite database connectivity via a non-destructive read query.
pub async fn probe_database(pool: &SqlitePool) -> CheckResult {
    match sqlx::query("SELECT name FROM sqlite_master LIMIT 1")
        .fetch_optional(pool)
        .await
    {
        Ok(_) => CheckResult::ok(),
        Err(err) => CheckResult::degraded(format!("SQLite query failed: {err}")),
    }
}

/// Probes workspace disk usage and headroom.
pub fn probe_disk(workspace_root: &Path) -> CheckResult {
    let total = match fs2::total_space(workspace_root) {
        Ok(t) => t,
        Err(err) => {
            return CheckResult::degraded(format!("failed to read total disk space: {err}"))
                .with_metric("pressure", "unknown")
        }
    };
    let free = match fs2::available_space(workspace_root) {
        Ok(f) => f,
        Err(err) => {
            return CheckResult::degraded(format!("failed to read available disk space: {err}"))
                .with_metric("total_bytes", total)
                .with_metric("pressure", "unknown")
        }
    };

    let (status, used_pct, pressure) = calculate_disk_headroom(total, free, DISK_DEGRADED_PERCENT);
    let mut check = if status == "ok" {
        CheckResult::ok()
    } else {
        CheckResult::degraded(format!("disk pressure is {pressure}, usage at {used_pct}%"))
    };
    check = check
        .with_metric("total_bytes", total)
        .with_metric("free_bytes", free)
        .with_metric("total_mb", total / DISK_BYTES_PER_MB)
        .with_metric("free_mb", free / DISK_BYTES_PER_MB)
        .with_metric("used_percent", used_pct)
        .with_metric("pressure", pressure);
    check
}

/// Provider whose credentials a model id needs; `None` for unknown/custom ids.
fn credential_provider(model: &str) -> Option<(&'static str, &'static str)> {
    let name = model.rsplit('/').next().unwrap_or(model);
    let is_openai_reasoning = |prefix: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['-', '.']))
    };
    if model.starts_with("anthropic/") || name.starts_with("claude") {
        Some(("anthropic", "ANTHROPIC_API_KEY"))
    } else if model.starts_with("openai/")
        || name.starts_with("gpt")
        || is_openai_reasoning("o1")
        || is_openai_reasoning("o3")
        || is_openai_reasoning("o4")
    {
        Some(("openai", "OPENAI_API_KEY"))
    } else if name.starts_with("deepseek") {
        Some(("deepseek", "DEEPSEEK_API_KEY"))
    } else {
        None
    }
}

/// Probes whether required credentials for the configured default LLM provider are present in the environment.
///
/// A known provider requires its own key (or the generic `LLM_API_KEY`); a key for a
/// different provider never satisfies it. Blank values count as absent.
pub fn probe_credentials(default_model: &str) -> CheckResult {
    probe_credentials_with(default_model, |name| std::env::var(name).ok())
}

fn probe_credentials_with(
    default_model: &str,
    lookup: impl Fn(&str) -> Option<String>,
) -> CheckResult {
    let env_present = |name: &str| lookup(name).is_some_and(|value| !value.trim().is_empty());
    let model = default_model.trim().to_ascii_lowercase();
    if model.is_empty() {
        return CheckResult::degraded("no default model configured");
    }

    let present =
        || CheckResult::ok_with_detail(format!("credentials present for model '{default_model}'"));
    match credential_provider(&model) {
        Some((_, key)) if env_present(key) || env_present("LLM_API_KEY") => present(),
        Some((_, key)) => CheckResult::degraded(format!("{key} is not set in environment")),
        None => {
            let any_key = [
                "ANTHROPIC_API_KEY",
                "OPENAI_API_KEY",
                "DEEPSEEK_API_KEY",
                "OPENROUTER_API_KEY",
                "GEMINI_API_KEY",
                "LLM_API_KEY",
            ]
            .iter()
            .any(|name| env_present(name));
            if any_key {
                present()
            } else {
                CheckResult::degraded(format!(
                    "no API key found in environment for model '{default_model}'"
                ))
            }
        }
    }
}

/// Probes configured platform bots count.
///
/// The count passed here is the number of bots CONFIGURED in the environment,
/// not the number with live gateway connections (that state is not reachable
/// from readiness collection). The metric is named to match reality; labeling
/// it "connected_bots" made readiness report green while the gateway could
/// not receive messages.
pub fn probe_gateway(bot_count: usize) -> CheckResult {
    if bot_count == 0 {
        CheckResult::degraded("no platform bot tokens configured")
    } else {
        CheckResult::ok().with_metric("configured_bots", bot_count)
    }
}

fn backend_http_url(appserver_url: Option<&str>) -> String {
    let url = appserver_url.unwrap_or("ws://127.0.0.1:19742");
    if let Some(stripped) = url.strip_prefix("ws://") {
        format!("http://{stripped}")
    } else if let Some(stripped) = url.strip_prefix("wss://") {
        format!("https://{stripped}")
    } else {
        url.to_string()
    }
}

/// Probes the local agent backend daemon health.
pub async fn probe_backend(appserver_url: Option<&str>) -> CheckResult {
    let http_url = backend_http_url(appserver_url);

    // Loopback probe: never route it through HTTP(S)_PROXY.
    let client = match reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_millis(800))
        .build()
    {
        Ok(c) => c,
        Err(e) => return CheckResult::degraded(format!("failed to build HTTP client: {e}")),
    };

    let base = http_url.trim_end_matches('/');
    let target_readyz = format!("{base}/readyz");
    let target_health = format!("{base}/health");

    let resp = match client.get(&target_readyz).send().await {
        Ok(r) if r.status().is_success() => return CheckResult::ok(),
        Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND => {
            client.get(&target_health).send().await
        }
        Ok(r) => Ok(r),
        Err(_) => client.get(&target_health).send().await,
    };

    match resp {
        Ok(r) if r.status().is_success() => CheckResult::ok(),
        Ok(r) => CheckResult::degraded(format!("backend returned HTTP {}", r.status())),
        Err(err) => {
            CheckResult::degraded(format!("backend connection refused or unreachable: {err}"))
        }
    }
}

/// Collects non-destructive startup and runtime readiness probes across database, disk, credentials, and gateway.
pub async fn collect_runtime_readiness(
    pool: &SqlitePool,
    workspace_root: &Path,
    default_model: &str,
    bot_count: usize,
) -> ReadinessReport {
    let mut checks = HashMap::new();

    checks.insert("state_db".to_string(), probe_database(pool).await);
    checks.insert("disk".to_string(), probe_disk(workspace_root));
    checks.insert("credentials".to_string(), probe_credentials(default_model));
    checks.insert("gateway".to_string(), probe_gateway(bot_count));
    let appserver_url = std::env::var("OMON_OMO_APPSERVER_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    checks.insert(
        "backend".to_string(),
        probe_backend(appserver_url.as_deref()).await,
    );

    let overall_status = if checks.values().all(|c| c.status == "ok") {
        "ok".to_string()
    } else {
        "degraded".to_string()
    };

    ReadinessReport {
        status: overall_status,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_probe_defaults_to_omo_appserver_url() {
        assert_eq!(backend_http_url(None), "http://127.0.0.1:19742");
        assert_eq!(
            backend_http_url(Some("wss://example.test:9000")),
            "https://example.test:9000"
        );
    }

    #[test]
    fn disk_pressure_uses_absolute_headroom() {
        let sample1 = calculate_disk_headroom(
            1000 * DISK_BYTES_PER_MB,
            200 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        let sample2 = calculate_disk_headroom(
            1000000 * DISK_BYTES_PER_MB,
            50000 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        let sample3 = calculate_disk_headroom(0, 0, DISK_DEGRADED_PERCENT);

        println!(
            "case=1 total_mb=1000 free_mb=200 status={} pct={} pressure={}",
            sample1.0, sample1.1, sample1.2
        );
        println!(
            "case=2 total_mb=1000000 free_mb=50000 status={} pct={} pressure={}",
            sample2.0, sample2.1, sample2.2
        );
        println!(
            "case=3 total_mb=0 free_mb=0 status={} pct={} pressure={}",
            sample3.0, sample3.1, sample3.2
        );

        // Case 1: 1000 MiB total, 200 MiB free (80% used, but < 256 MiB free floor)
        // GREEN: status must be "degraded", pressure must be "critical"
        assert_eq!(
            sample1.0, "degraded",
            "sample 1 must be degraded due to critical headroom"
        );
        assert_eq!(
            sample1.2, "critical",
            "sample 1 must classify as critical pressure"
        );

        // Case 2: 1000000 MiB total, 50000 MiB free (95% used, but 50 GB free headroom)
        // GREEN: status must be "ok", pressure must be "ok"
        assert_eq!(sample2.0, "ok", "sample 2 must be ok with 50 GB headroom");
        assert_eq!(sample2.2, "ok", "sample 2 must classify as ok pressure");

        // Case 3: 0 total, 0 free (unusable / zero capacity sample)
        // GREEN: status must be nonhealthy ("degraded"), pressure must be "unknown"
        assert_ne!(
            sample3.0, "ok",
            "sample 3 must not be healthy for zero total capacity"
        );
        assert_eq!(
            sample3.0, "degraded",
            "sample 3 must be degraded for zero total capacity"
        );
        assert_eq!(
            sample3.2, "unknown",
            "sample 3 must classify as unknown pressure"
        );
    }

    #[test]
    fn test_disk_headroom_calculation() {
        // 50% usage with sufficient headroom -> ok
        let (status, pct, pressure) = calculate_disk_headroom(
            100_000 * DISK_BYTES_PER_MB,
            50_000 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "ok");
        assert_eq!(pct, 50.0);
        assert_eq!(pressure, "ok");

        // 89.9% usage with sufficient headroom -> ok
        let (status, pct, pressure) = calculate_disk_headroom(
            100_000 * DISK_BYTES_PER_MB,
            10_100 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "ok");
        assert_eq!(pct, 89.9);
        assert_eq!(pressure, "ok");

        // High usage (95%) but large absolute headroom (50 GB) -> ok
        let (status, pct, pressure) = calculate_disk_headroom(
            1_000_000 * DISK_BYTES_PER_MB,
            50_000 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "ok");
        assert_eq!(pct, 95.0);
        assert_eq!(pressure, "ok");

        // Elevated: < 512 MB free (and < 95% used) -> degraded / elevated
        let (status, pct, pressure) = calculate_disk_headroom(
            4_000 * DISK_BYTES_PER_MB,
            400 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "degraded");
        assert_eq!(pct, 90.0);
        assert_eq!(pressure, "elevated");

        // Critical: >= 95% used AND < 1024 MB free -> degraded / critical
        let (status, pct, pressure) = calculate_disk_headroom(
            10_000 * DISK_BYTES_PER_MB,
            400 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "degraded");
        assert_eq!(pct, 96.0);
        assert_eq!(pressure, "critical");

        // Critical: < 256 MB free -> degraded / critical
        let (status, pct, pressure) = calculate_disk_headroom(
            1_000 * DISK_BYTES_PER_MB,
            200 * DISK_BYTES_PER_MB,
            DISK_DEGRADED_PERCENT,
        );
        assert_eq!(status, "degraded");
        assert_eq!(pct, 80.0);
        assert_eq!(pressure, "critical");

        // 0 total -> unusable capacity, degraded / unknown (not ok fallback)
        let (status, pct, pressure) = calculate_disk_headroom(0, 0, DISK_DEGRADED_PERCENT);
        assert_eq!(status, "degraded");
        assert_eq!(pct, 0.0);
        assert_eq!(pressure, "unknown");
    }

    #[tokio::test]
    async fn test_database_probe_in_memory() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let check = probe_database(&pool).await;
        assert_eq!(check.status, "ok");
    }

    #[test]
    fn test_gateway_probe() {
        let zero = probe_gateway(0);
        assert_eq!(zero.status, "degraded");

        let active = probe_gateway(2);
        assert_eq!(active.status, "ok");
        // The metric must name what the number actually is: configured bots,
        // not live connections.
        assert_eq!(
            active.metrics.get("configured_bots"),
            Some(&serde_json::json!(2))
        );
        assert!(!active.metrics.contains_key("connected_bots"));
    }

    #[test]
    fn credential_provider_matches_prefixes_not_substrings() {
        let cases = [
            ("claude-sonnet-4", Some("ANTHROPIC_API_KEY")),
            ("anthropic/claude-3", Some("ANTHROPIC_API_KEY")),
            ("gpt-4o", Some("OPENAI_API_KEY")),
            ("openai/o3-mini", Some("OPENAI_API_KEY")),
            ("o1", Some("OPENAI_API_KEY")),
            ("deepseek-chat", Some("DEEPSEEK_API_KEY")),
            // Substrings of unrelated names must not select a provider.
            ("llama-3o1b", None),
            ("custom-o3x", None),
            ("custom-model", None),
        ];
        for (model, expected) in cases {
            assert_eq!(
                credential_provider(model).map(|(_, key)| key),
                expected,
                "{model}"
            );
        }
    }

    #[test]
    fn credentials_require_the_models_own_provider_key() {
        let only = |name: &'static str, value: &'static str| {
            move |key: &str| (key == name).then(|| value.to_owned())
        };
        // A key for a different provider must not make a gpt model look ready.
        let wrong = probe_credentials_with("gpt-4o", only("ANTHROPIC_API_KEY", "sk-ant"));
        assert_eq!(wrong.status, "degraded");
        // The model's own key, or the generic LLM_API_KEY, does.
        assert_eq!(
            probe_credentials_with("gpt-4o", only("OPENAI_API_KEY", "sk")).status,
            "ok"
        );
        assert_eq!(
            probe_credentials_with("gpt-4o", only("LLM_API_KEY", "sk")).status,
            "ok"
        );
        // A blank value is not a credential.
        let blank = probe_credentials_with("claude-sonnet-4", only("ANTHROPIC_API_KEY", "  "));
        assert_eq!(blank.status, "degraded");
        // Unknown models accept any non-blank key.
        let custom = probe_credentials_with("custom-model", only("OPENROUTER_API_KEY", "k"));
        assert_eq!(custom.status, "ok");
        let none = probe_credentials_with("custom-model", |_| None);
        assert_eq!(none.status, "degraded");
    }

    #[test]
    fn test_credential_probe_model_logic() {
        // Empty model -> degraded
        let empty = probe_credentials("");
        assert_eq!(empty.status, "degraded");
    }

    #[tokio::test]
    async fn test_collect_runtime_readiness_overall() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let temp = tempfile::tempdir().unwrap();

        let report = collect_runtime_readiness(&pool, temp.path(), "custom-model", 1).await;
        assert!(report.checks.contains_key("state_db"));
        assert!(report.checks.contains_key("disk"));
        assert!(report.checks.contains_key("credentials"));
        assert!(report.checks.contains_key("gateway"));
        assert_eq!(report.checks["state_db"].status, "ok");
        assert!(report.checks["disk"].status == "ok" || report.checks["disk"].status == "degraded");
        assert!(report.checks["disk"].metrics.contains_key("used_percent"));
        assert!(report.checks["disk"].metrics.contains_key("pressure"));
        assert_eq!(report.checks["gateway"].status, "ok");
        assert!(report.status == "ok" || report.status == "degraded");
    }
}

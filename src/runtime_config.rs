//! Global runtime configuration overrides for the gateway process.
//!
//! Replaces runtime environment variable mutation (such as `std::env::set_var`) which is not
//! thread-safe against concurrent reads in a multi-threaded runtime: `boot_slack` used to write
//! `OMON_LANE_DEFAULT_LEAD` after lanes, kibitzer, cron health, proposals and the actor had
//! already spawned. Writers call [`set_default_lead`] once, readers call [`default_lead`].

use std::sync::OnceLock;

static DEFAULT_LEAD: OnceLock<String> = OnceLock::new();

/// Sets the default lead session key once at runtime.
///
/// First write wins and later calls are ignored (`false`), so a Slack reconnect cannot move the
/// owner home out from under the lanes that resolved it. Blank values are refused: an empty lead
/// would make every consumer address nothing.
pub fn set_default_lead(lead: impl Into<String>) -> bool {
    let lead = lead.into();
    if lead.trim().is_empty() {
        return false;
    }
    DEFAULT_LEAD.set(lead).is_ok()
}

/// Returns the configured default lead session key.
///
/// Precedence:
/// 1. The runtime-configured lead set via [`set_default_lead`].
/// 2. A non-empty `OMON_LANE_DEFAULT_LEAD` environment variable.
/// 3. The built-in default (`crate::lanes::kibitzer::DEFAULT_LEAD`).
pub fn default_lead() -> String {
    if let Some(lead) = DEFAULT_LEAD.get() {
        return lead.clone();
    }
    std::env::var("OMON_LANE_DEFAULT_LEAD")
        .ok()
        .filter(|lead| !lead.trim().is_empty())
        .unwrap_or_else(|| crate::lanes::kibitzer::DEFAULT_LEAD.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHILD: &str = "OMON_RUNTIME_CONFIG_TEST_CHILD";

    /// A child process is the only way to test a `OnceLock` that other tests in this binary share.
    #[test]
    fn test_default_lead_runtime_config() {
        if std::env::var_os(CHILD).is_some() {
            assert_eq!(default_lead(), "slack:environment-owner");
            assert!(set_default_lead("slack:runtime-owner"));
            assert_eq!(default_lead(), "slack:runtime-owner");
            assert!(!set_default_lead("slack:replacement"));
            assert!(!set_default_lead("   "));
            assert_eq!(default_lead(), "slack:runtime-owner");
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "runtime_config::tests::test_default_lead_runtime_config",
            ])
            .env(CHILD, "1")
            .env("OMON_LANE_DEFAULT_LEAD", "slack:environment-owner")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "child ran no test: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

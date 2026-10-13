use std::sync::LazyLock;

use regex::Regex;

use crate::{OmonError, Result};

static CREDENTIAL_PATTERNS: LazyLock<
    std::result::Result<Vec<(Regex, &'static str)>, regex::Error>,
> = LazyLock::new(|| {
    [
            (
                r#"(?i)(authorization["']?\s*:\s*)(?:[^\s"']+\s+)?(?:"(?:\\.|[^"\\])*"|'[^']*'|[^\s"']+)"#,
                "${1}[REDACTED]",
            ),
            (
                r#"(?i)(["']?[a-z0-9_-]*(?:token|secret|password|api[_-]?key|access[_-]?key|credentials?)[a-z0-9_-]*["']?\s*[:=]\s*)(?:"(?:\\.|[^"\\])*"|'[^']*'|[^\s'";|&]+)"#,
                "${1}[REDACTED]",
            ),
            (
                r#"(?i)(--(?:api[-_]key|token|password|secret|client[-_]secret|access[-_]key)(?:=|\s+))(?:"(?:\\.|[^"\\])*"|'[^']*'|[^\s'";|&]+)"#,
                "${1}[REDACTED]",
            ),
            (
                r#"(?i)([a-z][a-z0-9+.-]*://)[^/\s@]+@"#,
                "${1}[REDACTED]@",
            ),
        ]
        .into_iter()
        .map(|(pat, rep)| Regex::new(pat).map(|re| (re, rep)))
        .collect()
});

pub fn redact_approval_display(value: &str) -> Result<String> {
    let patterns = CREDENTIAL_PATTERNS.as_ref().map_err(|error| {
        OmonError::Config(format!("invalid approval redaction pattern: {error}"))
    })?;
    let mut display = value.to_owned();
    for (pattern, replacement) in patterns {
        display = pattern.replace_all(&display, *replacement).into_owned();
    }
    Ok(display)
}

#[cfg(test)]
mod tests {
    use super::redact_approval_display;

    #[test]
    fn redacts_recognized_credentials_and_preserves_benign_text() {
        for input in [
            "curl -H 'Authorization: Bearer sentinel-value' https://example.invalid",
            "Authorization: Basic sentinel-value",
            "OPENAI_API_KEY='sentinel-value with spaces' command",
            r#"{"access_token": "sentinel-value"}"#,
            "command --password 'sentinel-value with spaces'",
            "https://user:sentinel-value@example.invalid/path",
            "https://user:sentinel-value@example.invalid:8443/path",
            "postgres://app:sentinel-value@db.example.invalid:5432/app",
        ] {
            let rendered = redact_approval_display(input).unwrap();
            assert!(!rendered.contains("sentinel-value"), "{rendered}");
            assert!(rendered.contains("[REDACTED]"));
        }
        for benign in [
            "cargo test --test example -- --exact",
            "curl http://localhost:8080/metrics",
            "http://localhost:8080/health",
            "http://localhost:8080",
            "https://example.invalid:8443/api/v1",
            "http://127.0.0.1:3000",
        ] {
            assert_eq!(redact_approval_display(benign).unwrap(), benign);
        }
    }

    #[test]
    fn preserves_userinfo_delimiter_and_host_path() {
        let rendered =
            redact_approval_display("https://user:sentinel-value@example.invalid:8080/api/v1")
                .unwrap();
        assert_eq!(rendered, "https://[REDACTED]@example.invalid:8080/api/v1");
    }
}

use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};

use crate::security::redact_approval_display;

// The leading group anchors to a path start, so URL paths like host/home/x are kept.
// A dotted account (first.last) is one name; a trailing sentence dot is not part of it.
static HOME_DIR_SLASH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(^|[\s'"`(=\[{<,;:]|file://)/(?:Users|home)/[a-zA-Z0-9_\-]+(?:\.[a-zA-Z0-9_\-]+)*/"#,
    )
    .expect("valid regex")
});

static HOME_DIR_BARE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(^|[\s'"`(=\[{<,;:]|file://)/(?:Users|home)/[a-zA-Z0-9_\-]+(?:\.[a-zA-Z0-9_\-]+)*"#,
    )
    .expect("valid regex")
});

static LITERAL_HOME_SLASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(?:\{HOME\}|HOME)/").expect("valid regex"));

static LITERAL_HOME_BARE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(?:\{HOME\}|HOME\b)").expect("valid regex"));

static GITHUB_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:gh[pousr]_[A-Za-z0-9_]{10,}|github_pat_[A-Za-z0-9_]{20,})\b")
        .expect("valid regex")
});

static OPENAI_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_\-]{20,})\b").expect("valid regex")
});

static SLACK_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bxox[baprs]-[A-Za-z0-9_\-]{10,}\b").expect("valid regex"));

static JWT_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b")
        .expect("valid regex")
});

static PRIVATE_KEY_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)-----BEGIN (?:[A-Z ]+ )?PRIVATE KEY-----.*?-----END (?:[A-Z ]+ )?PRIVATE KEY-----",
    )
    .expect("valid regex")
});

// Truncated output can lose the END marker; whatever follows a BEGIN is key material.
static UNTERMINATED_PRIVATE_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN (?:[A-Z ]+ )?PRIVATE KEY-----.*").expect("valid regex")
});

pub fn scrub(text: &str) -> String {
    let hosts_env = std::env::var("OMON_SCRUB_HOSTS").unwrap_or_default();
    let hosts: Vec<&str> = hosts_env
        .split(',')
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .collect();
    scrub_with_hosts(text, &hosts)
}

pub fn scrub_with_hosts(text: &str, hosts: &[&str]) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut scrubbed = redact_approval_display(text).unwrap_or_else(|_| text.to_string());
    scrubbed = scrubbed.replace("[REDACTED]", "<redacted>");

    scrubbed = GITHUB_TOKEN_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();
    scrubbed = OPENAI_TOKEN_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();
    scrubbed = SLACK_TOKEN_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();
    scrubbed = JWT_TOKEN_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();
    scrubbed = PRIVATE_KEY_BLOCK_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();
    scrubbed = UNTERMINATED_PRIVATE_KEY_RE
        .replace_all(&scrubbed, "<redacted>")
        .into_owned();

    for &host in hosts {
        let trimmed = host.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(re) = RegexBuilder::new(&format!(r"(?i)\b{}\b", regex::escape(trimmed))).build() {
            scrubbed = re.replace_all(&scrubbed, "<redacted>").into_owned();
        } else {
            let lower_host = trimmed.to_lowercase();
            let mut result = String::with_capacity(scrubbed.len());
            let mut remainder = scrubbed.as_str();
            while let Some(idx) = remainder.to_lowercase().find(&lower_host) {
                result.push_str(&remainder[..idx]);
                result.push_str("<redacted>");
                remainder = &remainder[idx + trimmed.len()..];
            }
            result.push_str(remainder);
            scrubbed = result;
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let trimmed_home = home.trim_end_matches('/');
        if !trimmed_home.is_empty() {
            let with_slash = format!("{trimmed_home}/");
            scrubbed = scrubbed.replace(&with_slash, "~/");
            scrubbed = scrubbed.replace(trimmed_home, "~");
        }
    }

    scrubbed = LITERAL_HOME_SLASH_RE
        .replace_all(&scrubbed, "~/")
        .into_owned();
    scrubbed = LITERAL_HOME_BARE_RE
        .replace_all(&scrubbed, "~")
        .into_owned();

    scrubbed = HOME_DIR_SLASH_RE
        .replace_all(&scrubbed, "${1}~/")
        .into_owned();
    scrubbed = HOME_DIR_BARE_RE
        .replace_all(&scrubbed, "${1}~")
        .into_owned();

    scrubbed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redacts_fake_gh_token_36_chars() {
        let fake_gh_token = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
        assert_eq!(fake_gh_token.len(), 4 + 36);

        let report = format!("[REPORT] qa-1 | done | pushed with token {fake_gh_token} to origin");
        let scrubbed = scrub(&report);
        assert!(!scrubbed.contains(fake_gh_token));
        assert!(scrubbed.contains("<redacted>"));
        assert_eq!(
            scrubbed,
            "[REPORT] qa-1 | done | pushed with token <redacted> to origin"
        );

        for prefix in ["gho_", "ghu_", "ghs_", "ghr_"] {
            let token = format!("{prefix}0123456789abcdefghijklmnopqrstuvwx");
            let input = format!("Token: {token}");
            let out = scrub(&input);
            assert!(!out.contains(&token), "Failed for {prefix}");
            assert!(out.contains("<redacted>"));
        }

        let fine_pat = "github_pat_11ABCD0123456789_abcdefghijklmnopqrstuvwxyz";
        let out = scrub(&format!("PAT: {fine_pat}"));
        assert!(!out.contains(fine_pat));
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn test_redacts_tokens_openai_anthropic_slack_jwt_keys() {
        let openai = "sk-1234567890abcdefghijklmnopqrstuvwxyz";
        assert_eq!(scrub(&format!("Key: {openai}")), "Key: <redacted>");

        let anthropic = "sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz";
        assert_eq!(
            scrub(&format!("Anthropic: {anthropic}")),
            "Anthropic: <redacted>"
        );

        let slack = "xoxb-1234567890-1234567890123-abcdefghijkl";
        assert_eq!(scrub(&format!("Slack: {slack}")), "Slack: <redacted>");

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijklmnopqrstuvwxyz1234567890_ABCD";
        assert_eq!(scrub(&format!("JWT: {jwt}")), "JWT: <redacted>");

        let private_key = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0fakecontent\n-----END RSA PRIVATE KEY-----";
        assert_eq!(
            scrub(&format!("Cert:\n{private_key}\nDone")),
            "Cert:\n<redacted>\nDone"
        );
    }

    #[test]
    fn test_redacts_security_approval_display_credentials() {
        let bearer = "Authorization: Bearer secret-auth-token-123";
        let out = scrub(bearer);
        assert!(!out.contains("secret-auth-token-123"));
        assert!(out.contains("<redacted>"));

        let basic = "Authorization: Basic dXNlcjpwYXNz";
        let out = scrub(basic);
        assert!(!out.contains("dXNlcjpwYXNz"));
        assert!(out.contains("<redacted>"));

        let url_cred = "https://admin:supersecret@internal.corp:8443/api/v1";
        let out = scrub(url_cred);
        assert!(!out.contains("supersecret"));
        assert!(out.contains("<redacted>"));

        let flag = "curl --api-key supersecret12345 https://example.com";
        let out = scrub(flag);
        assert!(!out.contains("supersecret12345"));
        assert!(out.contains("<redacted>"));

        let kv = "DB_PASSWORD=\"topsecret!123\"";
        let out = scrub(kv);
        assert!(!out.contains("topsecret!123"));
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn test_scrubs_home_directory_and_private_paths() {
        assert_eq!(
            scrub("/Users/indo/code/project/src/main.rs"),
            "~/code/project/src/main.rs"
        );
        assert_eq!(
            scrub("/home/ubuntu/repo/target/debug/app"),
            "~/repo/target/debug/app"
        );
        assert_eq!(scrub("/Users/alice/"), "~/");
        assert_eq!(scrub("/home/runner/"), "~/");
        assert_eq!(scrub("Path is /Users/indo"), "Path is ~");
        assert_eq!(scrub("Working in /home/tester"), "Working in ~");
        assert_eq!(
            scrub("Files: /Users/bob/a.rs and /home/alice/b.rs"),
            "Files: ~/a.rs and ~/b.rs"
        );
    }

    #[test]
    fn test_scrubs_paths_with_korean_text() {
        let input1 = "작업 디렉토리: /Users/indo/code/project 입니다.";
        assert_eq!(scrub(input1), "작업 디렉토리: ~/code/project 입니다.");

        let input2 = "홈 디렉터리는 /Users/indo 입니다.";
        assert_eq!(scrub(input2), "홈 디렉터리는 ~ 입니다.");

        let input3 = "빌드 오류 발생 위치: /home/runner/work/omon-gateway/src/main.rs:42 확인 필요";
        assert_eq!(
            scrub(input3),
            "빌드 오류 발생 위치: ~/work/omon-gateway/src/main.rs:42 확인 필요"
        );

        let input4 = "출력 경로 '/Users/custom-user/secret-repo/output.txt' 저장 완료";
        assert_eq!(
            scrub(input4),
            "출력 경로 '~/secret-repo/output.txt' 저장 완료"
        );
    }

    #[test]
    fn test_scrubs_hostnames_from_omon_scrub_hosts() {
        let hosts = ["secret.internal", "db-cluster.corp.local"];
        let input = "Connecting to https://secret.internal:8080/v1 and db-cluster.corp.local:5432";
        let out = scrub_with_hosts(input, &hosts);
        assert!(!out.contains("secret.internal"));
        assert!(!out.contains("db-cluster.corp.local"));
        assert_eq!(
            out,
            "Connecting to https://<redacted>:8080/v1 and <redacted>:5432"
        );

        let input_upper = "Host SECRET.INTERNAL down; failover to DB-CLUSTER.CORP.LOCAL";
        let out_upper = scrub_with_hosts(input_upper, &hosts);
        assert_eq!(out_upper, "Host <redacted> down; failover to <redacted>");

        let out_empty = scrub_with_hosts("Connecting to secret.internal", &[]);
        assert_eq!(out_empty, "Connecting to secret.internal");
    }

    #[test]
    fn test_preserves_benign_text() {
        for benign in [
            "cargo test --lib lanes::scrub",
            "curl http://localhost:8080/metrics",
            "http://localhost:8080/health",
            "http://127.0.0.1:3000",
            "정상 실행되었습니다: 15개의 테스트 모두 통과.",
            "Git branch: dori-work, status: clean",
        ] {
            assert_eq!(scrub(benign), benign);
        }
    }

    #[test]
    fn test_redacts_pkcs8_and_rsa_private_key_blocks() {
        let pkcs8 = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAAfake\n-----END PRIVATE KEY-----";
        assert_eq!(scrub(&format!("k:\n{pkcs8}\nok")), "k:\n<redacted>\nok");
        let rsa =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEAfake\n-----END RSA PRIVATE KEY-----";
        assert_eq!(scrub(&format!("k:\n{rsa}\nok")), "k:\n<redacted>\nok");
    }

    #[test]
    fn test_home_dir_regex_does_not_rewrite_url_paths() {
        for url in [
            "https://example.com/home/alice/profile",
            "https://example.com/Users/alice",
            "http://localhost:8080/home/runner/x",
        ] {
            assert_eq!(scrub(url), url);
        }
        assert_eq!(scrub("path=/home/alice/x"), "path=~/x");
    }

    #[test]
    fn test_redacts_an_unterminated_private_key_and_scrubs_file_urls_and_dotted_users() {
        assert_eq!(
            scrub("out:\n-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAAfake"),
            "out:\n<redacted>"
        );
        assert_eq!(scrub("file:///Users/bob/x.txt"), "file://~/x.txt");
        assert_eq!(scrub("/Users/first.last/x"), "~/x");
        assert_eq!(scrub("/home/first.last"), "~");
        assert_eq!(scrub("Path is /Users/indo."), "Path is ~.");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(scrub(""), "");
    }

    #[test]
    fn test_idempotence() {
        let text = "[REPORT] qa-1 | done | token: ghp_0123456789abcdefghijklmnopqrstuvwxyz path: /Users/indo/code";
        let once = scrub(text);
        let twice = scrub(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_scrubs_literal_dollar_home() {
        assert_eq!(scrub("$HOME/repo/file.txt"), "~/repo/file.txt");
        assert_eq!(scrub("${HOME}/repo/file.txt"), "~/repo/file.txt");
        assert_eq!(scrub("Path is $HOME"), "Path is ~");
        assert_eq!(scrub("Path is ${HOME}"), "Path is ~");
        assert_eq!(
            scrub("작업 경로: $HOME/data 입니다"),
            "작업 경로: ~/data 입니다"
        );
        assert_eq!(
            scrub("홈 디렉터리는 $HOME 입니다."),
            "홈 디렉터리는 ~ 입니다."
        );
        assert_eq!(scrub("$HOMESICK is untouched"), "$HOMESICK is untouched");
        assert_eq!(scrub("$HOME_DIR is untouched"), "$HOME_DIR is untouched");
    }
}

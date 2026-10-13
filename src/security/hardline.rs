use regex::Regex;
use std::sync::LazyLock;

use super::normalize::{
    command_detection_variants, command_detection_variants_with_budget,
    command_parser_limit_exceeded, grep_safe_detection_variant, normalize_command_for_detection,
};

struct HardlineRule {
    regex: Regex,
    description: &'static str,
}

static SUDO_STDIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[;&|`\n]|&&|\|\||[$]\()\s*sudo\s+-S\b").unwrap());

fn build_hardline_rm_path(path_alt: &str) -> String {
    let tail = r"(?:\s|$|[)`;|&])";
    format!(r#"(?:['\x22](?:{path_alt})['\x22]|(?:{path_alt}){tail})"#)
}

static HARDLINE_PATTERNS: LazyLock<Vec<HardlineRule>> = LazyLock::new(|| {
    let cmdpos = r"(?:^|[\n`]|[$]\()\s*(?:(?:\S*/)?sudo\s+(?:-[^\s]+\s+(?:[^-\s]\S*\s+)?)*)?(?:(?:\S*/)?env\s+(?:-[^\s]+\s+(?:[^-\s=]+\s+)?|\w+=\S*\s+)*)?(?:(?:exec|nohup|setsid|time)\s+)*\s*(?:\S*/)?";
    let rm_prefix = format!(r"{cmdpos}rm\s+(?:[^\s;&|\n]+\s+)*");
    let hardline_system_dirs = r"/home|/home/\*|/root|/root/\*|/etc|/etc/\*|/usr|/usr/\*|/var|/var/\*|/bin|/bin/\*|/sbin|/sbin/\*|/boot|/boot/\*|/lib|/lib/\*|/System|/System/\*";

    let root_rm_path = build_hardline_rm_path(r"/(?:/|[.]/|[.][.]/)*(?:[.]|[.][.])?\**|/ \*");
    let sys_rm_path = build_hardline_rm_path(hardline_system_dirs);
    let home_rm_path = build_hardline_rm_path(r"(?:~|\$\{?HOME\}?)(?:/?|/\*)?");

    let raw_rules = vec![
        // rm root / system / home
        (
            format!(r"{rm_prefix}{root_rm_path}"),
            "recursive delete of root filesystem",
        ),
        (
            format!(r"{rm_prefix}{sys_rm_path}"),
            "recursive delete of system directory",
        ),
        (
            format!(r"{rm_prefix}{home_rm_path}"),
            "recursive delete of home directory",
        ),
        // Filesystem format
        (
            r"\bmkfs(\.[a-z0-9]+)?\b".to_string(),
            "format filesystem (mkfs)",
        ),
        // Raw block device overwrites
        (
            r"\bdd\b[^\n]*\bof=/dev/(sd|nvme|hd|mmcblk|vd|xvd|disk)[a-z0-9]*".to_string(),
            "dd to raw block device",
        ),
        (
            r">\s*/dev/(sd|nvme|hd|mmcblk|vd|xvd|disk)[a-z0-9]*\b".to_string(),
            "redirect to raw block device",
        ),
        // Fork bomb
        (
            r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:".to_string(),
            "fork bomb",
        ),
        // Kill all processes
        // `kill -1` / `kill -9 -1` (pid -1 = every process). `-1` must be the final operand:
        // `kill -1 1234` sends SIGHUP to one process and is not a kill-all.
        (
            r"\bkill\s+(?:[^\s;&|]+\s+)*-1(?:\s+(?:\d*[<>]\S*|#.*))*\s*(?:$|[;&|)`\n])".to_string(),
            "kill all processes",
        ),
        (r"\bkill\s+-9\s+-1\b".to_string(), "kill all processes"),
        // Shutdown / reboot / halt
        (
            format!(r"{cmdpos}(shutdown|reboot|halt|poweroff)(?:\s|$|[;&|)<>`])"),
            "system shutdown/reboot",
        ),
        (
            format!(r"{cmdpos}init\s+[06]\b"),
            "init 0/6 (shutdown/reboot)",
        ),
        (
            format!(r"{cmdpos}systemctl\s+(poweroff|reboot|halt|kexec)\b"),
            "systemctl poweroff/reboot",
        ),
        (
            format!(r"{cmdpos}telinit\s+[06]\b"),
            "telinit 0/6 (shutdown/reboot)",
        ),
    ];

    raw_rules
        .into_iter()
        .map(|(pat, desc)| {
            let regex = regex::RegexBuilder::new(&pat)
                .case_insensitive(true)
                .dot_matches_new_line(true)
                .build()
                .unwrap_or_else(|e| panic!("invalid hardline regex {pat}: {e}"));
            HardlineRule {
                regex,
                description: desc,
            }
        })
        .collect()
});

pub fn check_sudo_stdin_guard(command: &str) -> Option<String> {
    if std::env::var("SUDO_PASSWORD").is_ok() {
        return None;
    }
    let normalized = normalize_command_for_detection(command);
    if SUDO_STDIN_RE.is_match(&normalized) {
        return Some("sudo password guessing via stdin (sudo -S)".to_string());
    }
    None
}

pub fn detect_hardline_command(command: &str) -> Option<String> {
    if command_parser_limit_exceeded(command) {
        return Some("command parser limit exceeded".to_string());
    }
    let normalized = normalize_command_for_detection(command);
    let (_, malformed_grep) = grep_safe_detection_variant(&normalized);
    if malformed_grep {
        return Some("command parser limit or malformed executable payload".to_string());
    }
    let (variants, budget_exhausted) = command_detection_variants_with_budget(command);
    for variant in variants {
        let variant_lower = variant.to_ascii_lowercase();
        for rule in HARDLINE_PATTERNS.iter() {
            if rule.regex.is_match(&variant_lower) {
                return Some(rule.description.to_string());
            }
        }
    }
    if budget_exhausted {
        return Some("command parser limit exceeded".to_string());
    }
    if let Some(reason) = check_sudo_stdin_guard(command) {
        return Some(reason);
    }
    None
}

/// Matches wildcard patterns (fnmatch style) where `*` matches any sequence of characters and `?` matches any single character.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pc: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let tc: Vec<char> = text.to_ascii_lowercase().chars().collect();
    // Iterative matcher with single-star backtracking: no O(pattern * text) table, so a huge
    // command cannot force a huge allocation per variant and glob.
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while t < tc.len() {
        if p < pc.len() && pc[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if p < pc.len() && (pc[p] == '?' || pc[p] == tc[t]) {
            p += 1;
            t += 1;
        } else if let Some(s) = star {
            p = s + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pc.len() && pc[p] == '*' {
        p += 1;
    }
    p == pc.len()
}

/// A trailing ` *` only adds arguments, so `npm publish *` must also deny the bare `npm publish`.
fn glob_matches(pattern: &str, candidate: &str) -> bool {
    wildcard_match(pattern, candidate)
        || pattern.strip_suffix(" *").is_some_and(|stem| {
            !stem.trim().is_empty() && wildcard_match(stem.trim_end(), candidate)
        })
}

/// Returns the matching `APPROVALS_DENY` pattern if the command or any normalized variant matches.
pub fn match_user_deny_rule<'a>(command: &str, deny_patterns: &'a [String]) -> Option<&'a str> {
    if deny_patterns.is_empty() {
        return None;
    }
    let globs: Vec<&'a str> = deny_patterns
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if globs.is_empty() {
        return None;
    }
    for variant in command_detection_variants(command) {
        let candidate = variant.trim();
        for pattern in &globs {
            if glob_matches(pattern, candidate) {
                return Some(pattern);
            }
        }
        // A deny glob names an executable and its arguments. Matching only the whole
        // variant lets a wrapper prefix (env / sudo / exec / nohup, env assignments)
        // launder the very command the operator unconditionally denied, so match the
        // wrapper-stripped executable form as well.
        for stripped in wrapper_stripped_forms(candidate) {
            for pattern in &globs {
                if glob_matches(pattern, stripped.trim()) {
                    return Some(pattern);
                }
            }
        }
    }
    None
}

/// Upper bound on command-start spans re-rooted per variant, so a crafted script cannot
/// multiply glob-matching work.
const MAX_WRAPPER_STRIPPED_FORMS: usize = 256;

/// Yields each command-start span of `command` re-rooted at its real executable, i.e.
/// with wrapper prefixes such as `env`, `sudo`, `exec` and leading `VAR=value`
/// assignments removed. Returns nothing when there was no prefix to strip.
fn wrapper_stripped_forms(command: &str) -> Vec<String> {
    let mut forms = Vec::new();
    for (start, _, _) in crate::security::normalize::iter_shell_command_word_spans(command)
        .into_iter()
        .take(MAX_WRAPPER_STRIPPED_FORMS)
    {
        if start == 0 {
            continue;
        }
        let candidate = command[start..].trim();
        if !candidate.is_empty() && !forms.iter().any(|f| f == candidate) {
            forms.push(candidate.to_string());
        }
    }
    forms
}

#[cfg(test)]
mod deny_bypass_tests {
    use super::match_user_deny_rule;

    #[test]
    fn wrapper_prefixed_commands_still_trip_a_deny_glob() {
        let deny = vec!["npm publish *".to_string()];

        // Baseline: the bare form is denied today.
        assert_eq!(
            match_user_deny_rule("npm publish --access public", &deny),
            Some("npm publish *"),
            "bare command must be denied"
        );

        // Wrapper prefixes must not launder the same command past the operator's deny rule.
        for wrapped in [
            "env npm publish --access public",
            "sudo npm publish --access public",
            "exec npm publish --access public",
            "nohup npm publish --access public",
            "env FOO=bar npm publish --access public",
        ] {
            assert_eq!(
                match_user_deny_rule(wrapped, &deny),
                Some("npm publish *"),
                "wrapper-prefixed command must still be denied: {wrapped}"
            );
        }
    }
}

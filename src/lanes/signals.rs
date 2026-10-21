use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SignalParseError {
    #[error("signals list is empty")]
    Empty,
    #[error("unknown signal kind: {0}")]
    UnknownKind(String),
    #[error("uncheckable command: {0}")]
    UncheckableCommand(String),
    #[error("malformed JSON value: {0}")]
    MalformedJson(String),
    #[error("invalid regex: {0}")]
    InvalidRegex(String),
    #[error("invalid syntax: {0}")]
    InvalidSyntax(String),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoneSignal {
    Merged {
        owner_repo: String,
        pr_number: u64,
    },
    Closed {
        owner_repo: String,
        issue_number: u64,
    },
    Published {
        package: String,
        version: String,
    },
    Command {
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stdout_regex: Option<String>,
    },
    File {
        path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        json_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        json_value: Option<serde_json::Value>,
    },
    Url {
        url: String,
        #[serde(default = "default_status")]
        status: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body_regex: Option<String>,
    },
}

fn default_status() -> u16 {
    200
}

impl std::fmt::Display for DoneSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoneSignal::Merged {
                owner_repo,
                pr_number,
            } => {
                write!(f, "merged {}#{}", owner_repo, pr_number)
            }
            DoneSignal::Closed {
                owner_repo,
                issue_number,
            } => {
                write!(f, "closed {}#{}", owner_repo, issue_number)
            }
            DoneSignal::Published { package, version } => {
                write!(f, "published {}@{}", package, version)
            }
            DoneSignal::Command { argv, stdout_regex } => {
                let argv_json = serde_json::to_string(argv).unwrap_or_else(|_| "[]".to_string());
                match stdout_regex {
                    Some(regex) => write!(f, "command {} stdout~{:?}", argv_json, regex),
                    None => write!(f, "command {}", argv_json),
                }
            }
            DoneSignal::File {
                path,
                sha256,
                json_path,
                json_value,
            } => {
                write!(f, "file {}", path.display())?;
                if let Some(sha) = sha256 {
                    write!(f, " sha256={}", sha)?;
                }
                if let (Some(jp), Some(jv)) = (json_path, json_value) {
                    let val_str = serde_json::to_string(jv).unwrap_or_default();
                    write!(f, " json:{}={}", jp, val_str)?;
                }
                Ok(())
            }
            DoneSignal::Url {
                url,
                status,
                body_regex,
            } => {
                write!(f, "url {}", url)?;
                if *status != 200 {
                    write!(f, " status={}", status)?;
                }
                if let Some(regex) = body_regex {
                    write!(f, " body~{:?}", regex)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignalResult {
    pub signal: String,
    pub ok: bool,
    pub detail: String,
}

impl SignalResult {
    pub fn pass(signal: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            signal: signal.into(),
            ok: true,
            detail: detail.into(),
        }
    }

    pub fn fail(signal: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            signal: signal.into(),
            ok: false,
            detail: detail.into(),
        }
    }
}

#[async_trait::async_trait]
pub trait SignalRunner: Send + Sync {
    async fn check(&self, signal: &DoneSignal, cwd: &Path) -> SignalResult;

    async fn check_all(&self, signals: &[DoneSignal], cwd: &Path) -> Vec<SignalResult> {
        let mut results = Vec::with_capacity(signals.len());
        for signal in signals {
            results.push(self.check(signal, cwd).await);
        }
        results
    }
}

pub async fn check_signals(
    signals: &[DoneSignal],
    runner: &impl SignalRunner,
    cwd: &Path,
) -> Vec<SignalResult> {
    runner.check_all(signals, cwd).await
}

pub async fn check_signals_str(
    signals_str: &str,
    runner: &impl SignalRunner,
    cwd: &Path,
) -> Result<Vec<SignalResult>, SignalParseError> {
    let signals = parse_signals(signals_str)?;
    Ok(runner.check_all(&signals, cwd).await)
}

/// Removes a leading `(?i)done\s*=\s*` prefix if present, returning the trimmed remainder.
pub fn strip_done_prefix(raw: &str) -> &str {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("done") {
        let rest_trimmed = rest.trim_start();
        if rest_trimmed.starts_with('=') {
            let offset = trimmed.len() - rest_trimmed.len() + 1;
            return trimmed[offset..].trim();
        }
    }
    trimmed
}

pub fn parse_signals(input: &str) -> Result<Vec<DoneSignal>, SignalParseError> {
    let stripped = strip_done_prefix(input);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return Err(SignalParseError::Empty);
    }

    let segments = split_top_level_semicolons(trimmed)?;
    if segments.is_empty() {
        return Err(SignalParseError::Empty);
    }

    let mut signals = Vec::with_capacity(segments.len());
    for segment in segments {
        signals.push(parse_single_signal(&segment)?);
    }

    Ok(signals)
}

fn split_top_level_semicolons(input: &str) -> Result<Vec<String>, SignalParseError> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }
        if let Some(q) = in_quote {
            current.push(ch);
            if ch == q {
                in_quote = None;
            }
            continue;
        }
        match ch {
            // The grammar quotes with JSON strings only; an apostrophe in a path
            // (it's.txt) must not swallow the semicolons after it.
            '"' => {
                in_quote = Some(ch);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            '{' => {
                brace_depth += 1;
                current.push(ch);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(ch);
            }
            ';' if bracket_depth == 0 && brace_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                } else if parts.is_empty() {
                    return Err(SignalParseError::InvalidSyntax(
                        "empty signal before semicolon".into(),
                    ));
                }
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }

    if parts.is_empty() {
        return Err(SignalParseError::Empty);
    }

    Ok(parts)
}

fn parse_single_signal(raw: &str) -> Result<DoneSignal, SignalParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SignalParseError::Empty);
    }

    let (kind, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((k, r)) => (k.trim(), r.trim()),
        None => (trimmed, ""),
    };

    match kind {
        "merged" => parse_merged(rest),
        "closed" => parse_closed(rest),
        "published" => parse_published(rest),
        "command" => parse_command(rest),
        "file" => parse_file(rest),
        "url" => parse_url(rest),
        unknown => Err(SignalParseError::UnknownKind(unknown.to_string())),
    }
}

fn parse_github_target(rest: &str, kind: &str) -> Result<(String, u64), SignalParseError> {
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return Err(SignalParseError::InvalidSyntax(format!(
            "missing target for {kind} signal"
        )));
    }

    // Accepts:
    // 1) owner/repo#N
    // 2) owner/repo/pull/N
    // 3) owner/repo/issues/N
    let (repo_part, num_str) = if let Some((r, n)) = trimmed.split_once('#') {
        (r, n)
    } else if let Some((r, n)) = trimmed.split_once("/pull/") {
        (r, n)
    } else if let Some((r, n)) = trimmed.split_once("/issues/") {
        (r, n)
    } else {
        return Err(SignalParseError::InvalidSyntax(format!(
            "missing '#', '/pull/' or '/issues/' in {kind} signal: {rest}"
        )));
    };

    let (owner, repo) = repo_part.split_once('/').ok_or_else(|| {
        SignalParseError::InvalidSyntax(format!(
            "missing '/' in owner/repo for {kind} signal: {repo_part}"
        ))
    })?;
    let owner = owner.trim();
    let repo = repo.trim();
    let num_str = num_str.trim();
    if owner.is_empty() || repo.is_empty() {
        return Err(SignalParseError::InvalidSyntax(format!(
            "invalid owner/repo in {kind} signal: {repo_part}"
        )));
    }
    let number: u64 = num_str.parse().map_err(|e| {
        SignalParseError::InvalidSyntax(format!("invalid {kind} number '{num_str}': {e}"))
    })?;
    if number == 0 {
        return Err(SignalParseError::InvalidSyntax(format!(
            "{kind} number must be positive"
        )));
    }

    Ok((format!("{owner}/{repo}"), number))
}

fn parse_merged(rest: &str) -> Result<DoneSignal, SignalParseError> {
    let (owner_repo, pr_number) = parse_github_target(rest, "merged")?;
    Ok(DoneSignal::Merged {
        owner_repo,
        pr_number,
    })
}

fn parse_closed(rest: &str) -> Result<DoneSignal, SignalParseError> {
    let (owner_repo, issue_number) = parse_github_target(rest, "closed")?;
    Ok(DoneSignal::Closed {
        owner_repo,
        issue_number,
    })
}

fn parse_published(rest: &str) -> Result<DoneSignal, SignalParseError> {
    if rest.is_empty() {
        return Err(SignalParseError::InvalidSyntax(
            "missing target for published signal".into(),
        ));
    }
    let last_at = rest.rfind('@').ok_or_else(|| {
        SignalParseError::InvalidSyntax(format!("missing '@' in published signal: {}", rest))
    })?;
    if last_at == 0 {
        return Err(SignalParseError::InvalidSyntax(format!(
            "missing package name before '@' in published signal: {}",
            rest
        )));
    }
    let package = rest[..last_at].trim();
    let version = rest[last_at + 1..].trim();
    if package.is_empty() || version.is_empty() {
        return Err(SignalParseError::InvalidSyntax(format!(
            "invalid package or version in published signal: {}",
            rest
        )));
    }

    Ok(DoneSignal::Published {
        package: package.to_string(),
        version: version.to_string(),
    })
}

fn parse_command(rest: &str) -> Result<DoneSignal, SignalParseError> {
    if rest.is_empty() {
        return Err(SignalParseError::InvalidSyntax(
            "missing argv for command signal".into(),
        ));
    }
    if !rest.starts_with('[') {
        return Err(SignalParseError::InvalidSyntax(format!(
            "command argv must be a JSON array starting with '[': {}",
            rest
        )));
    }

    let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<serde_json::Value>();
    let value = match stream.next() {
        Some(Ok(v)) => v,
        Some(Err(e)) => {
            return Err(SignalParseError::InvalidSyntax(format!(
                "malformed command argv JSON array: {}",
                e
            )))
        }
        None => {
            return Err(SignalParseError::InvalidSyntax(
                "missing command argv".into(),
            ))
        }
    };

    let argv: Vec<String> = match value {
        serde_json::Value::Array(arr) => {
            let mut res = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    serde_json::Value::String(s) => res.push(s),
                    _ => {
                        return Err(SignalParseError::InvalidSyntax(
                            "command argv elements must be strings".into(),
                        ))
                    }
                }
            }
            res
        }
        _ => {
            return Err(SignalParseError::InvalidSyntax(
                "command argv must be a JSON array".into(),
            ))
        }
    };

    if argv.is_empty() {
        return Err(SignalParseError::UncheckableCommand(
            "empty command argv".into(),
        ));
    }
    if argv.len() == 1
        && (argv[0] == "true"
            || argv[0] == ":"
            || argv[0] == "/bin/true"
            || argv[0] == "/usr/bin/true")
    {
        return Err(SignalParseError::UncheckableCommand(format!(
            "uncheckable command: {:?}",
            argv
        )));
    }
    if argv[0] == "true" || argv[0] == ":" {
        return Err(SignalParseError::UncheckableCommand(format!(
            "uncheckable command: {:?}",
            argv
        )));
    }

    let byte_offset = stream.byte_offset();
    let remaining = rest[byte_offset..].trim();
    let mut stdout_regex = None;
    let mut curr = remaining;
    while let Some((token, next_rem)) = scan_next_option(curr)? {
        let clause = unwrap_option_brackets(token);
        if let Some(pat_str) = clause.strip_prefix("stdout~") {
            let pat = parse_quoted_or_raw(pat_str)?;
            regex::Regex::new(&pat).map_err(|e| SignalParseError::InvalidRegex(e.to_string()))?;
            stdout_regex = Some(pat);
        } else {
            return Err(SignalParseError::InvalidSyntax(format!(
                "unexpected clause after command argv: {}",
                clause
            )));
        }
        curr = next_rem;
    }

    Ok(DoneSignal::Command { argv, stdout_regex })
}

fn scan_next_option(input: &str) -> Result<Option<(&str, &str)>, SignalParseError> {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.starts_with('[') {
        let mut depth = 0usize;
        let mut in_quote = false;
        let mut escape = false;
        let mut end_idx = None;

        for (idx, ch) in trimmed.char_indices() {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                if in_quote {
                    escape = true;
                }
                continue;
            }
            if ch == '"' {
                in_quote = !in_quote;
                continue;
            }
            if !in_quote {
                if ch == '[' {
                    depth += 1;
                } else if ch == ']' {
                    if depth == 0 {
                        return Err(SignalParseError::InvalidSyntax(format!(
                            "unexpected closing bracket in option: {}",
                            trimmed
                        )));
                    }
                    depth -= 1;
                    if depth == 0 {
                        end_idx = Some(idx + ch.len_utf8());
                        break;
                    }
                }
            }
        }

        if in_quote {
            return Err(SignalParseError::InvalidSyntax(format!(
                "unterminated quote in option: {}",
                trimmed
            )));
        }
        if depth != 0 {
            return Err(SignalParseError::InvalidSyntax(format!(
                "unterminated bracket in option: {}",
                trimmed
            )));
        }

        let end = end_idx.ok_or_else(|| {
            SignalParseError::InvalidSyntax(format!("unterminated bracket in option: {}", trimmed))
        })?;
        Ok(Some((&trimmed[..end], &trimmed[end..])))
    } else {
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut in_quote = false;
        let mut escape = false;
        let mut end_idx = None;

        for (idx, ch) in trimmed.char_indices() {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                if in_quote {
                    escape = true;
                }
                continue;
            }
            if ch == '"' {
                in_quote = !in_quote;
                continue;
            }
            if !in_quote {
                if ch == '[' {
                    bracket_depth += 1;
                } else if ch == ']' {
                    bracket_depth = bracket_depth.saturating_sub(1);
                } else if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth = brace_depth.saturating_sub(1);
                } else if ch.is_whitespace() && bracket_depth == 0 && brace_depth == 0 {
                    end_idx = Some(idx);
                    break;
                }
            }
        }

        if in_quote {
            return Err(SignalParseError::InvalidSyntax(format!(
                "unterminated quote in option: {}",
                trimmed
            )));
        }

        let end = end_idx.unwrap_or(trimmed.len());
        Ok(Some((&trimmed[..end], &trimmed[end..])))
    }
}

fn unwrap_option_brackets(token: &str) -> &str {
    let t = token.trim();
    if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

fn parse_file(rest: &str) -> Result<DoneSignal, SignalParseError> {
    if rest.is_empty() {
        return Err(SignalParseError::InvalidSyntax(
            "missing path for file signal".into(),
        ));
    }

    let (path_str, remaining) = if let Some(after_quote) = rest.strip_prefix('"') {
        let end_quote = after_quote.find('"').ok_or_else(|| {
            SignalParseError::InvalidSyntax("unterminated quote in file path".into())
        })?;
        (&after_quote[..end_quote], &after_quote[end_quote + 1..])
    } else {
        match rest.split_once(char::is_whitespace) {
            Some((p, rem)) => (p, rem),
            None => (rest, ""),
        }
    };

    let path = PathBuf::from(path_str.trim());
    if path.as_os_str().is_empty() {
        return Err(SignalParseError::InvalidSyntax("empty file path".into()));
    }

    let mut sha256 = None;
    let mut json_path = None;
    let mut json_value = None;

    let mut curr = remaining;
    while let Some((token, next_rem)) = scan_next_option(curr)? {
        let clause = unwrap_option_brackets(token);
        parse_file_option(clause, &mut sha256, &mut json_path, &mut json_value)?;
        curr = next_rem;
    }

    Ok(DoneSignal::File {
        path,
        sha256,
        json_path,
        json_value,
    })
}

fn parse_file_option(
    opt: &str,
    sha256: &mut Option<String>,
    json_path: &mut Option<String>,
    json_value: &mut Option<serde_json::Value>,
) -> Result<(), SignalParseError> {
    if let Some(hex) = opt.strip_prefix("sha256=") {
        let trimmed_hex = hex.trim();
        if trimmed_hex.len() != 64 || !trimmed_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SignalParseError::InvalidSyntax(
                "sha256= takes 64 hex characters".into(),
            ));
        }
        *sha256 = Some(trimmed_hex.to_ascii_lowercase());
        Ok(())
    } else if let Some(after_json) = opt.strip_prefix("json:") {
        let (jp, json_str) = after_json.split_once('=').ok_or_else(|| {
            SignalParseError::InvalidSyntax(format!("missing '=' in json option: {}", opt))
        })?;
        let stripped_path = jp.strip_prefix('.').unwrap_or(jp);
        if stripped_path.is_empty() {
            return Err(SignalParseError::InvalidSyntax(
                "json: needs a field path like json:.version".into(),
            ));
        }
        let val: serde_json::Value = serde_json::from_str(json_str.trim()).map_err(|e| {
            SignalParseError::MalformedJson(format!("invalid JSON value in file signal: {}", e))
        })?;
        *json_path = Some(jp.to_string());
        *json_value = Some(val);
        Ok(())
    } else {
        Err(SignalParseError::InvalidSyntax(format!(
            "unexpected file option: {}",
            opt
        )))
    }
}

fn parse_url(rest: &str) -> Result<DoneSignal, SignalParseError> {
    if rest.is_empty() {
        return Err(SignalParseError::InvalidSyntax(
            "missing target for url signal".into(),
        ));
    }

    let (url_str, remaining) = match rest.split_once(char::is_whitespace) {
        Some((u, rem)) => (u, rem),
        None => (rest, ""),
    };
    let url = url_str.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(SignalParseError::InvalidUrl(format!(
            "URL must start with http:// or https://: {}",
            url
        )));
    }

    let mut status = 200u16;
    let mut body_regex = None;

    let mut curr = remaining;
    while let Some((token, next_rem)) = scan_next_option(curr)? {
        let clause = unwrap_option_brackets(token);

        if let Some(s) = clause.strip_prefix("status=") {
            status = s.trim().parse().map_err(|e| {
                SignalParseError::InvalidSyntax(format!("invalid status code '{}': {}", s, e))
            })?;
        } else if let Some(pat_str) = clause.strip_prefix("body~") {
            let pat = parse_quoted_or_raw(pat_str)?;
            regex::Regex::new(&pat).map_err(|e| SignalParseError::InvalidRegex(e.to_string()))?;
            body_regex = Some(pat);
        } else {
            return Err(SignalParseError::InvalidSyntax(format!(
                "unexpected option in url signal: {}",
                clause
            )));
        }

        curr = next_rem;
    }

    Ok(DoneSignal::Url {
        url: url.to_string(),
        status,
        body_regex,
    })
}

fn parse_quoted_or_raw(s: &str) -> Result<String, SignalParseError> {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        serde_json::from_str::<String>(trimmed)
            .or_else(|_| Ok(trimmed[1..trimmed.len() - 1].to_string()))
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn get_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    let clean_path = path.strip_prefix('.').unwrap_or(path);
    if clean_path.is_empty() {
        return Some(current);
    }

    for part in clean_path.split('.') {
        if part.is_empty() {
            continue;
        }
        let mut rest = part;
        while !rest.is_empty() {
            if let Some(bracket_start) = rest.find('[') {
                let key = &rest[..bracket_start];
                if !key.is_empty() {
                    current = match current {
                        serde_json::Value::Object(map) => map.get(key)?,
                        _ => return None,
                    };
                }
                let after_bracket = &rest[bracket_start + 1..];
                let bracket_end = after_bracket.find(']')?;
                let idx_str = &after_bracket[..bracket_end];
                let idx: usize = idx_str.parse().ok()?;
                current = match current {
                    serde_json::Value::Array(arr) => arr.get(idx)?,
                    _ => return None,
                };
                rest = &after_bracket[bracket_end + 1..];
            } else {
                current = match current {
                    serde_json::Value::Object(map) => map.get(rest)?,
                    serde_json::Value::Array(arr) => {
                        if let Ok(idx) = rest.parse::<usize>() {
                            arr.get(idx)?
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                };
                break;
            }
        }
    }
    Some(current)
}

fn resolve_file_path(path: &Path, cwd: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str == "~" || path_str.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            let without_tilde = path_str.strip_prefix('~').unwrap_or("");
            let clean = without_tilde.strip_prefix('/').unwrap_or(without_tilde);
            return PathBuf::from(home).join(clean);
        }
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

const OUTPUT_TAIL_BYTES: usize = 4096;

/// Keeps only the last few KB of command output so failure details stay bounded.
/// Command output kept in memory per stream (the tail; earlier output is dropped).
const COMMAND_OUTPUT_CAP: usize = 256 * 1024;
const FILE_SIGNAL_MAX_BYTES: u64 = 64 * 1024 * 1024;

fn is_secret_env_name(name: &str) -> bool {
    // Forge credentials stay: done signals such as `gh pr view` need them.
    if matches!(name, "GH_TOKEN" | "GITHUB_TOKEN" | "NPM_TOKEN") {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "API_KEY",
        "APIKEY",
        "CREDENTIAL",
    ]
    .iter()
    .any(|marker| upper.contains(marker))
}

/// Drains a pipe to EOF, keeping only the last `cap` bytes.
async fn read_tail<R: tokio::io::AsyncRead + Unpin>(mut pipe: R, cap: usize) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut kept = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                kept.extend_from_slice(&chunk[..read]);
                if kept.len() > cap {
                    let excess = kept.len() - cap;
                    kept.drain(..excess);
                }
            }
        }
    }
    kept
}

fn output_tail(text: &str) -> &str {
    if text.len() <= OUTPUT_TAIL_BYTES {
        return text;
    }
    let mut start = text.len() - OUTPUT_TAIL_BYTES;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

#[cfg(test)]
type ChildReadyHook = std::sync::Arc<
    dyn Fn(u32) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

#[cfg(test)]
type CommandSetupHook = std::sync::Arc<dyn Fn(&mut tokio::process::Command) + Send + Sync>;

#[derive(Clone)]
pub struct SystemSignalRunner {
    pub command_timeout: Duration,
    pub url_timeout: Duration,
    pub gh_bin: OsString,
    pub npm_bin: OsString,
    #[cfg(test)]
    pub(crate) child_ready_hook: Option<ChildReadyHook>,
    #[cfg(test)]
    pub(crate) command_setup_hook: Option<CommandSetupHook>,
}

impl std::fmt::Debug for SystemSignalRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemSignalRunner")
            .field("command_timeout", &self.command_timeout)
            .field("url_timeout", &self.url_timeout)
            .field("gh_bin", &self.gh_bin)
            .field("npm_bin", &self.npm_bin)
            .finish()
    }
}

impl Default for SystemSignalRunner {
    fn default() -> Self {
        Self {
            command_timeout: Duration::from_secs(120),
            url_timeout: Duration::from_secs(15),
            gh_bin: OsString::from("gh"),
            npm_bin: OsString::from("npm"),
            #[cfg(test)]
            child_ready_hook: None,
            #[cfg(test)]
            command_setup_hook: None,
        }
    }
}

impl SystemSignalRunner {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn with_child_ready_hook<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(u32) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.child_ready_hook = Some(std::sync::Arc::new(move |pid| Box::pin(hook(pid))));
        self
    }

    #[cfg(test)]
    pub fn with_command_setup_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&mut tokio::process::Command) + Send + Sync + 'static,
    {
        self.command_setup_hook = Some(std::sync::Arc::new(hook));
        self
    }

    pub fn with_command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    pub fn with_url_timeout(mut self, timeout: Duration) -> Self {
        self.url_timeout = timeout;
        self
    }

    pub fn with_gh_bin(mut self, bin: impl Into<OsString>) -> Self {
        self.gh_bin = bin.into();
        self
    }

    pub fn with_npm_bin(mut self, bin: impl Into<OsString>) -> Self {
        self.npm_bin = bin.into();
        self
    }
}

#[async_trait::async_trait]
impl SignalRunner for SystemSignalRunner {
    async fn check(&self, signal: &DoneSignal, cwd: &Path) -> SignalResult {
        match signal {
            DoneSignal::Merged {
                owner_repo,
                pr_number,
            } => {
                let mut cmd = tokio::process::Command::new(&self.gh_bin);
                cmd.args([
                    "pr",
                    "view",
                    &pr_number.to_string(),
                    "-R",
                    owner_repo,
                    "--json",
                    "state,mergeCommit",
                ]);
                cmd.current_dir(cwd);
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                cmd.kill_on_drop(true);

                let output = match tokio::time::timeout(self.command_timeout, cmd.output()).await {
                    Ok(Ok(out)) => out,
                    Ok(Err(e)) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to execute gh: {}", e),
                        )
                    }
                    Err(_) => {
                        return SignalResult::fail(signal.to_string(), "gh command timed out")
                    }
                };

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "gh pr view failed with exit code {:?}: {}",
                            output.status.code(),
                            stderr.trim()
                        ),
                    );
                }

                let v: serde_json::Value = match serde_json::from_slice(&output.stdout) {
                    Ok(val) => val,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to parse gh JSON output: {}", e),
                        )
                    }
                };

                let state = v.get("state").and_then(|s| s.as_str()).unwrap_or("");
                let merge_commit_is_non_null = v.get("mergeCommit").is_some_and(|mc| !mc.is_null());

                if state.eq_ignore_ascii_case("MERGED") && merge_commit_is_non_null {
                    SignalResult::pass(
                        signal.to_string(),
                        format!("PR #{} merged (mergeCommit present)", pr_number),
                    )
                } else {
                    SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "PR #{} not merged (state: {}, mergeCommit: {:?})",
                            pr_number,
                            state,
                            v.get("mergeCommit")
                        ),
                    )
                }
            }

            DoneSignal::Closed {
                owner_repo,
                issue_number,
            } => {
                let mut cmd = tokio::process::Command::new(&self.gh_bin);
                cmd.args([
                    "issue",
                    "view",
                    &issue_number.to_string(),
                    "-R",
                    owner_repo,
                    "--json",
                    "state",
                ]);
                cmd.current_dir(cwd);
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                cmd.kill_on_drop(true);

                let output = match tokio::time::timeout(self.command_timeout, cmd.output()).await {
                    Ok(Ok(out)) => out,
                    Ok(Err(e)) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to execute gh: {}", e),
                        )
                    }
                    Err(_) => {
                        return SignalResult::fail(signal.to_string(), "gh command timed out")
                    }
                };

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "gh issue view failed with exit code {:?}: {}",
                            output.status.code(),
                            stderr.trim()
                        ),
                    );
                }

                let v: serde_json::Value = match serde_json::from_slice(&output.stdout) {
                    Ok(val) => val,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to parse gh JSON output: {}", e),
                        )
                    }
                };

                let state = v.get("state").and_then(|s| s.as_str()).unwrap_or("");
                if state.eq_ignore_ascii_case("CLOSED") {
                    SignalResult::pass(
                        signal.to_string(),
                        format!("issue #{} closed (state: CLOSED)", issue_number),
                    )
                } else {
                    SignalResult::fail(
                        signal.to_string(),
                        format!("issue #{} not closed (state: {})", issue_number, state),
                    )
                }
            }

            DoneSignal::Published { package, version } => {
                let target = format!("{}@{}", package, version);
                let mut cmd = tokio::process::Command::new(&self.npm_bin);
                cmd.args(["view", &target, "version"]);
                cmd.current_dir(cwd);
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                cmd.kill_on_drop(true);

                let output = match tokio::time::timeout(self.command_timeout, cmd.output()).await {
                    Ok(Ok(out)) => out,
                    Ok(Err(e)) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to execute npm: {}", e),
                        )
                    }
                    Err(_) => {
                        return SignalResult::fail(signal.to_string(), "npm command timed out")
                    }
                };

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "npm view failed with exit code {:?}: {}",
                            output.status.code(),
                            stderr.trim()
                        ),
                    );
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let raw_version = stdout.trim().trim_matches('\'').trim_matches('"');
                if raw_version == version {
                    SignalResult::pass(
                        signal.to_string(),
                        format!("package {} version {} is published", package, version),
                    )
                } else {
                    SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "npm view returned version '{}', expected '{}'",
                            raw_version, version
                        ),
                    )
                }
            }

            DoneSignal::Command { argv, stdout_regex } => {
                let mut cmd = tokio::process::Command::new(&argv[0]);
                cmd.args(&argv[1..]);
                cmd.current_dir(cwd);
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                cmd.kill_on_drop(true);
                // Lane-authored commands must not inherit the gateway's credentials.
                for (name, _) in std::env::vars_os() {
                    if is_secret_env_name(&name.to_string_lossy()) {
                        cmd.env_remove(&name);
                    }
                }
                #[cfg(unix)]
                cmd.process_group(0);

                #[cfg(test)]
                if let Some(ref hook) = self.command_setup_hook {
                    hook(&mut cmd);
                }

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("command execution failed: {}", e),
                        )
                    }
                };

                #[cfg(test)]
                if let Some(ref hook) = self.child_ready_hook {
                    if let Some(pid) = child.id() {
                        hook(pid).await;
                    }
                }

                let pgid = child.id();
                let stdout_task = child
                    .stdout
                    .take()
                    .map(|pipe| tokio::spawn(read_tail(pipe, COMMAND_OUTPUT_CAP)));
                let stderr_task = child
                    .stderr
                    .take()
                    .map(|pipe| tokio::spawn(read_tail(pipe, COMMAND_OUTPUT_CAP)));
                let collected = async move {
                    let status = child.wait().await?;
                    let stdout = match stdout_task {
                        Some(task) => task.await.unwrap_or_default(),
                        None => Vec::new(),
                    };
                    let stderr = match stderr_task {
                        Some(task) => task.await.unwrap_or_default(),
                        None => Vec::new(),
                    };
                    Ok::<_, std::io::Error>(std::process::Output {
                        status,
                        stdout,
                        stderr,
                    })
                };
                let output = match tokio::time::timeout(self.command_timeout, collected).await {
                    Ok(Ok(out)) => out,
                    Ok(Err(e)) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("command execution failed: {}", e),
                        )
                    }
                    Err(_) => {
                        // The child leads its own group; kill grandchildren it spawned too.
                        #[cfg(unix)]
                        if let Some(pgid) = pgid {
                            unsafe {
                                libc::killpg(pgid as libc::pid_t, libc::SIGKILL);
                            }
                        }
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "command timed out after {}s",
                                self.command_timeout.as_secs()
                            ),
                        );
                    }
                };

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    return SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "command exited with status {:?}. stdout: {}, stderr: {}",
                            output.status.code(),
                            output_tail(stdout.trim()),
                            output_tail(stderr.trim())
                        ),
                    );
                }

                if let Some(regex_pat) = stdout_regex {
                    let re = match regex::Regex::new(regex_pat) {
                        Ok(r) => r,
                        Err(e) => {
                            return SignalResult::fail(
                                signal.to_string(),
                                format!("invalid stdout regex: {}", e),
                            )
                        }
                    };
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !re.is_match(&stdout) {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "command stdout did not match regex '{}'. stdout: '{}'",
                                regex_pat,
                                output_tail(stdout.trim())
                            ),
                        );
                    }
                }

                SignalResult::pass(
                    signal.to_string(),
                    "command exited 0 and matches constraints",
                )
            }

            DoneSignal::File {
                path,
                sha256,
                json_path,
                json_value,
            } => {
                let resolved_path = resolve_file_path(path, cwd);
                if let Ok(meta) = tokio::fs::metadata(&resolved_path).await {
                    if meta.len() > FILE_SIGNAL_MAX_BYTES {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "file '{}' is larger than {} bytes",
                                resolved_path.display(),
                                FILE_SIGNAL_MAX_BYTES
                            ),
                        );
                    }
                }
                let bytes = match tokio::fs::read(&resolved_path).await {
                    Ok(b) => b,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to read file '{}': {}", resolved_path.display(), e),
                        )
                    }
                };

                if let Some(expected_sha) = sha256 {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    let actual_sha = format!("{:x}", hasher.finalize());
                    if !actual_sha.eq_ignore_ascii_case(expected_sha) {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "sha256 mismatch for '{}': expected {}, got {}",
                                resolved_path.display(),
                                expected_sha,
                                actual_sha
                            ),
                        );
                    }
                }

                if let (Some(jp), Some(expected_val)) = (json_path, json_value) {
                    let root: serde_json::Value = match serde_json::from_slice(&bytes) {
                        Ok(v) => v,
                        Err(e) => {
                            return SignalResult::fail(
                                signal.to_string(),
                                format!(
                                    "file '{}' contains invalid JSON: {}",
                                    resolved_path.display(),
                                    e
                                ),
                            )
                        }
                    };
                    let actual_val = match get_json_path(&root, jp) {
                        Some(v) => v,
                        None => {
                            return SignalResult::fail(
                                signal.to_string(),
                                format!(
                                    "json path '{}' not found in file '{}'",
                                    jp,
                                    resolved_path.display()
                                ),
                            )
                        }
                    };
                    if actual_val != expected_val {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "json path '{}' in file '{}' mismatch: expected {}, got {}",
                                jp,
                                resolved_path.display(),
                                expected_val,
                                actual_val
                            ),
                        );
                    }
                }

                SignalResult::pass(
                    signal.to_string(),
                    format!("file '{}' verified", resolved_path.display()),
                )
            }

            DoneSignal::Url {
                url,
                status: expected_status,
                body_regex,
            } => {
                let client = match reqwest::Client::builder().timeout(self.url_timeout).build() {
                    Ok(c) => c,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("failed to build reqwest client: {}", e),
                        )
                    }
                };

                let resp = match client.get(url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!("HTTP request to '{}' failed: {}", url, e),
                        )
                    }
                };

                let actual_status = resp.status().as_u16();
                if actual_status != *expected_status {
                    return SignalResult::fail(
                        signal.to_string(),
                        format!(
                            "HTTP GET '{}' returned status {}, expected {}",
                            url, actual_status, expected_status
                        ),
                    );
                }

                if let Some(regex_pat) = body_regex {
                    let body = match resp.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            return SignalResult::fail(
                                signal.to_string(),
                                format!("failed to read response body from '{}': {}", url, e),
                            )
                        }
                    };
                    let re = match regex::Regex::new(regex_pat) {
                        Ok(r) => r,
                        Err(e) => {
                            return SignalResult::fail(
                                signal.to_string(),
                                format!("invalid body regex '{}': {}", regex_pat, e),
                            )
                        }
                    };
                    if !re.is_match(&body) {
                        return SignalResult::fail(
                            signal.to_string(),
                            format!(
                                "HTTP response body from '{}' did not match regex '{}'",
                                url, regex_pat
                            ),
                        );
                    }
                }

                SignalResult::pass(
                    signal.to_string(),
                    format!("URL '{}' returned status {}", url, actual_status),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn apostrophe_in_a_path_does_not_swallow_the_next_signal() {
        let signals = parse_signals("file it's.txt; file b.txt").unwrap();
        assert_eq!(signals.len(), 2);
    }

    #[test]
    fn secret_env_names_are_recognised() {
        for name in [
            "DISCORD_BOT_TOKEN",
            "TYPESAFE_API_KEY",
            "db_password",
            "AWS_SECRET_ACCESS_KEY",
        ] {
            assert!(is_secret_env_name(name), "{name}");
        }
        for name in ["PATH", "HOME", "DATABASE_URL", "CARGO_HOME", "GH_TOKEN"] {
            assert!(!is_secret_env_name(name), "{name}");
        }
    }

    #[tokio::test]
    async fn read_tail_keeps_only_the_last_bytes_of_unbounded_output() {
        let data: Vec<u8> = (0..100_000u32).map(|n| (n % 251) as u8).collect();
        let tail = read_tail(std::io::Cursor::new(data.clone()), 1000).await;
        assert_eq!(tail, data[data.len() - 1000..]);
    }

    #[test]
    fn test_parse_ok_all_kinds() {
        let s = parse_signals("merged code-yeongyu/oh-my-openagent#123").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Merged {
                owner_repo: "code-yeongyu/oh-my-openagent".into(),
                pr_number: 123,
            }
        );

        let s = parse_signals("closed foo/bar#456").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Closed {
                owner_repo: "foo/bar".into(),
                issue_number: 456,
            }
        );

        let s = parse_signals("published my-pkg@1.0.0").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Published {
                package: "my-pkg".into(),
                version: "1.0.0".into(),
            }
        );
        let s = parse_signals("published @scope/my-pkg@2.3.4").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Published {
                package: "@scope/my-pkg".into(),
                version: "2.3.4".into(),
            }
        );

        let s = parse_signals(r#"command ["cargo", "test"]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Command {
                argv: vec!["cargo".into(), "test".into()],
                stdout_regex: None,
            }
        );
        let s = parse_signals(r#"command ["echo", "hello"] stdout~"hello""#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Command {
                argv: vec!["echo".into(), "hello".into()],
                stdout_regex: Some("hello".into()),
            }
        );
        let s = parse_signals(r#"command ["echo", "hello"] [stdout~"hello"]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Command {
                argv: vec!["echo".into(), "hello".into()],
                stdout_regex: Some("hello".into()),
            }
        );

        let s = parse_signals("file done.txt").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: PathBuf::from("done.txt"),
                sha256: None,
                json_path: None,
                json_value: None,
            }
        );
        let s = parse_signals(
            "file done.txt sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: PathBuf::from("done.txt"),
                sha256: Some(
                    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into()
                ),
                json_path: None,
                json_value: None,
            }
        );
        let s = parse_signals(r#"file data.json json:.a.b=42"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: PathBuf::from("data.json"),
                sha256: None,
                json_path: Some(".a.b".into()),
                json_value: Some(serde_json::json!(42)),
            }
        );
        let s = parse_signals(
            r#"file data.json [sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef] [json:.status="ok"]"#,
        )
        .unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: PathBuf::from("data.json"),
                sha256: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
                ),
                json_path: Some(".status".into()),
                json_value: Some(serde_json::json!("ok")),
            }
        );

        let s = parse_signals("url http://localhost:8080/health").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Url {
                url: "http://localhost:8080/health".into(),
                status: 200,
                body_regex: None,
            }
        );
        let s = parse_signals("url https://example.com/api status=201 body~\"success\"").unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Url {
                url: "https://example.com/api".into(),
                status: 201,
                body_regex: Some("success".into()),
            }
        );

        let s = parse_signals(
            r#"merged a/b#1; command ["echo", "pass"]; file test.txt; url http://localhost:8080"#,
        )
        .unwrap();
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn test_parse_rejects() {
        assert!(matches!(parse_signals(""), Err(SignalParseError::Empty)));
        assert!(matches!(parse_signals("   "), Err(SignalParseError::Empty)));
        assert!(matches!(
            parse_signals(";"),
            Err(SignalParseError::InvalidSyntax(_)) | Err(SignalParseError::Empty)
        ));

        assert!(matches!(
            parse_signals("foo a/b#1"),
            Err(SignalParseError::UnknownKind(k)) if k == "foo"
        ));

        assert!(matches!(
            parse_signals(r#"command ["true"]"#),
            Err(SignalParseError::UncheckableCommand(_))
        ));
        assert!(matches!(
            parse_signals(r#"command [":"]"#),
            Err(SignalParseError::UncheckableCommand(_))
        ));
        assert!(matches!(
            parse_signals(r#"command ["/bin/true"]"#),
            Err(SignalParseError::UncheckableCommand(_))
        ));
        assert!(matches!(
            parse_signals(r#"command []"#),
            Err(SignalParseError::UncheckableCommand(_))
        ));

        assert!(matches!(
            parse_signals("file data.json json:.a=not_a_json_value"),
            Err(SignalParseError::MalformedJson(_))
        ));

        assert!(matches!(
            parse_signals("command not_an_array"),
            Err(SignalParseError::InvalidSyntax(_))
        ));

        assert!(matches!(
            parse_signals("url ftp://example.com"),
            Err(SignalParseError::InvalidUrl(_))
        ));

        assert!(matches!(
            parse_signals(r#"command ["echo"] stdout~"[unclosed""#),
            Err(SignalParseError::InvalidRegex(_))
        ));
    }

    #[test]
    fn test_parse_quoted_regex_and_nested_json_options() {
        // Case 1: unbracketed body regex with spaces inside quotes
        let s = parse_signals(r#"url https://example.com body~"Hello World""#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Url {
                url: "https://example.com".into(),
                status: 200,
                body_regex: Some("Hello World".into()),
            }
        );

        // Case 2: bracketed body regex with character class '[' and ']'
        let s = parse_signals(r#"url https://example.com [body~"[A-Z]+"]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Url {
                url: "https://example.com".into(),
                status: 200,
                body_regex: Some("[A-Z]+".into()),
            }
        );

        // Case 3: bracketed json option with nested array
        let s = parse_signals(r#"file data.json [json:.a=[1,2]]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: std::path::PathBuf::from("data.json"),
                sha256: None,
                json_path: Some(".a".into()),
                json_value: Some(serde_json::json!([1, 2])),
            }
        );

        // Case 4: unbracketed json option with nested array
        let s = parse_signals(r#"file data.json json:.a=[1,2]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: std::path::PathBuf::from("data.json"),
                sha256: None,
                json_path: Some(".a".into()),
                json_value: Some(serde_json::json!([1, 2])),
            }
        );

        // Case 5: bracketed json option with nested object
        let s = parse_signals(r#"file data.json [json:.a={"foo":"bar"}]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::File {
                path: std::path::PathBuf::from("data.json"),
                sha256: None,
                json_path: Some(".a".into()),
                json_value: Some(serde_json::json!({"foo": "bar"})),
            }
        );

        // Case 6: bracketed command stdout regex with character class
        let s = parse_signals(r#"command ["echo", "ABC"] [stdout~"[A-Z]+"]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Command {
                argv: vec!["echo".into(), "ABC".into()],
                stdout_regex: Some("[A-Z]+".into()),
            }
        );

        // Case 7: unbracketed command stdout regex with spaces inside quotes
        let s = parse_signals(r#"command ["echo", "A B"] stdout~"A B""#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Command {
                argv: vec!["echo".into(), "A B".into()],
                stdout_regex: Some("A B".into()),
            }
        );

        // Case 8: combined status and body regex
        let s = parse_signals(r#"url https://example.com [status=201] [body~"created"]"#).unwrap();
        assert_eq!(
            s[0],
            DoneSignal::Url {
                url: "https://example.com".into(),
                status: 201,
                body_regex: Some("created".into()),
            }
        );
    }

    #[test]
    fn test_parse_done_prefix() {
        let signals = parse_signals("Done = file x.txt").unwrap();
        assert_eq!(
            signals[0],
            DoneSignal::File {
                path: PathBuf::from("x.txt"),
                sha256: None,
                json_path: None,
                json_value: None,
            }
        );

        let signals = parse_signals("done=merged a/b#1").unwrap();
        assert_eq!(
            signals[0],
            DoneSignal::Merged {
                owner_repo: "a/b".into(),
                pr_number: 1,
            }
        );

        let signals = parse_signals("DONE   =   closed a/b#2").unwrap();
        assert_eq!(
            signals[0],
            DoneSignal::Closed {
                owner_repo: "a/b".into(),
                issue_number: 2,
            }
        );
    }

    #[test]
    fn test_parse_github_url_forms() {
        let merged_pull = parse_signals("merged owner/repo/pull/42").unwrap();
        assert_eq!(
            merged_pull[0],
            DoneSignal::Merged {
                owner_repo: "owner/repo".into(),
                pr_number: 42,
            }
        );

        let merged_issues = parse_signals("merged owner/repo/issues/43").unwrap();
        assert_eq!(
            merged_issues[0],
            DoneSignal::Merged {
                owner_repo: "owner/repo".into(),
                pr_number: 43,
            }
        );

        let closed_pull = parse_signals("closed owner/repo/pull/10").unwrap();
        assert_eq!(
            closed_pull[0],
            DoneSignal::Closed {
                owner_repo: "owner/repo".into(),
                issue_number: 10,
            }
        );

        let closed_issues = parse_signals("closed owner/repo/issues/20").unwrap();
        assert_eq!(
            closed_issues[0],
            DoneSignal::Closed {
                owner_repo: "owner/repo".into(),
                issue_number: 20,
            }
        );

        let standard = parse_signals("merged owner/repo#99").unwrap();
        assert_eq!(
            standard[0],
            DoneSignal::Merged {
                owner_repo: "owner/repo".into(),
                pr_number: 99,
            }
        );
    }

    #[test]
    fn test_parse_rejects_short_sha_and_empty_json_path() {
        let short_sha_err = parse_signals("file data.json sha256=1234abcd").unwrap_err();
        match short_sha_err {
            SignalParseError::InvalidSyntax(msg) => {
                assert!(
                    msg.contains("sha256= takes 64 hex characters"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected InvalidSyntax for short sha, got: {other:?}"),
        }

        let non_hex_sha_err = parse_signals(
            "file data.json sha256=zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        )
        .unwrap_err();
        match non_hex_sha_err {
            SignalParseError::InvalidSyntax(msg) => {
                assert!(
                    msg.contains("sha256= takes 64 hex characters"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected InvalidSyntax for non-hex sha, got: {other:?}"),
        }

        let empty_json_path_err = parse_signals("file data.json json:.=1").unwrap_err();
        match empty_json_path_err {
            SignalParseError::InvalidSyntax(msg) => {
                assert!(
                    msg.contains("json: needs a field path like json:.version"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected InvalidSyntax for empty json path, got: {other:?}"),
        }

        let bare_empty_json_path_err = parse_signals("file data.json json:=1").unwrap_err();
        match bare_empty_json_path_err {
            SignalParseError::InvalidSyntax(msg) => {
                assert!(
                    msg.contains("json: needs a field path like json:.version"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected InvalidSyntax for bare empty json path, got: {other:?}"),
        }
    }

    #[test]
    fn test_strip_done_prefix_keeps_plain_lines() {
        assert_eq!(strip_done_prefix("file x.txt"), "file x.txt");
        assert_eq!(strip_done_prefix("Done = file x.txt"), "file x.txt");
        assert_eq!(strip_done_prefix("done=file x.txt"), "file x.txt");
        assert_eq!(strip_done_prefix("DONE   =   file x.txt"), "file x.txt");
        assert_eq!(
            strip_done_prefix("  Done = merged a/b#1 ; file y.txt  "),
            "merged a/b#1 ; file y.txt"
        );
        assert_eq!(strip_done_prefix("done"), "done");
        assert_eq!(strip_done_prefix(""), "");
    }

    #[test]
    fn test_json_path_navigation() {
        let v = serde_json::json!({
            "a": {
                "b": 123,
                "c": "hello"
            },
            "items": [
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"}
            ],
            "ok": true
        });

        assert_eq!(get_json_path(&v, ".a.b"), Some(&serde_json::json!(123)));
        assert_eq!(get_json_path(&v, "a.b"), Some(&serde_json::json!(123)));
        assert_eq!(get_json_path(&v, ".a.c"), Some(&serde_json::json!("hello")));
        assert_eq!(get_json_path(&v, ".ok"), Some(&serde_json::json!(true)));
        assert_eq!(
            get_json_path(&v, ".items[0].id"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            get_json_path(&v, ".items[1].name"),
            Some(&serde_json::json!("second"))
        );
        assert_eq!(get_json_path(&v, ".nonexistent"), None);
    }

    #[tokio::test]
    async fn test_fake_gh_and_npm() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        let gh_path = bin_dir.join("fake_gh");
        let gh_script = r#"#!/bin/sh
if [ "$1" = "pr" ] && [ "$2" = "view" ]; then
    if [ "$3" = "100" ]; then
        echo '{"state":"MERGED","mergeCommit":{"oid":"commit123"}}'
        exit 0
    else
        echo '{"state":"OPEN","mergeCommit":null}'
        exit 0
    fi
elif [ "$1" = "issue" ] && [ "$2" = "view" ]; then
    if [ "$3" = "200" ]; then
        echo '{"state":"CLOSED"}'
        exit 0
    else
        echo '{"state":"OPEN"}'
        exit 0
    fi
fi
exit 1
"#;
        std::fs::write(&gh_path, gh_script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gh_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let npm_path = bin_dir.join("fake_npm");
        let npm_script = r#"#!/bin/sh
if [ "$1" = "view" ] && [ "$2" = "my-pkg@1.0.0" ]; then
    echo "1.0.0"
    exit 0
fi
exit 1
"#;
        std::fs::write(&npm_path, npm_script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&npm_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let runner = SystemSignalRunner::new()
            .with_gh_bin(gh_path.into_os_string())
            .with_npm_bin(npm_path.into_os_string());

        let pr_pass = parse_signals("merged my/repo#100").unwrap().remove(0);
        let res = runner.check(&pr_pass, temp.path()).await;
        assert!(res.ok, "Expected pass, got: {:?}", res);

        let pr_fail = parse_signals("merged my/repo#101").unwrap().remove(0);
        let res = runner.check(&pr_fail, temp.path()).await;
        assert!(!res.ok, "Expected fail, got: {:?}", res);

        let issue_pass = parse_signals("closed my/repo#200").unwrap().remove(0);
        let res = runner.check(&issue_pass, temp.path()).await;
        assert!(res.ok, "Expected pass, got: {:?}", res);

        let issue_fail = parse_signals("closed my/repo#201").unwrap().remove(0);
        let res = runner.check(&issue_fail, temp.path()).await;
        assert!(!res.ok, "Expected fail, got: {:?}", res);

        let npm_pass = parse_signals("published my-pkg@1.0.0").unwrap().remove(0);
        let res = runner.check(&npm_pass, temp.path()).await;
        assert!(res.ok, "Expected pass, got: {:?}", res);

        let npm_fail = parse_signals("published my-pkg@2.0.0").unwrap().remove(0);
        let res = runner.check(&npm_fail, temp.path()).await;
        assert!(!res.ok, "Expected fail, got: {:?}", res);
    }

    #[tokio::test]
    async fn test_url_local_listener() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\nHello, World!";
                    let _ = socket.write_all(response.as_bytes()).await;
                }
            }
        });

        let runner = SystemSignalRunner::new().with_url_timeout(Duration::from_secs(2));

        let sig = parse_signals(&format!(
            "url http://127.0.0.1:{} status=200 body~\"Hello\"",
            port
        ))
        .unwrap()
        .remove(0);
        let res = runner.check(&sig, Path::new(".")).await;
        assert!(res.ok, "Expected pass, got: {:?}", res);

        let sig_wrong_status = parse_signals(&format!("url http://127.0.0.1:{} status=404", port))
            .unwrap()
            .remove(0);
        let res = runner.check(&sig_wrong_status, Path::new(".")).await;
        assert!(!res.ok, "Expected fail on wrong status, got: {:?}", res);

        let sig_wrong_body =
            parse_signals(&format!("url http://127.0.0.1:{} body~\"Goodbye\"", port))
                .unwrap()
                .remove(0);
        let res = runner.check(&sig_wrong_body, Path::new(".")).await;
        assert!(!res.ok, "Expected fail on wrong body regex, got: {:?}", res);
    }

    #[tokio::test]
    async fn test_command_timeout_override() {
        let runner = SystemSignalRunner::new().with_command_timeout(Duration::from_secs(1));
        let sig = parse_signals(r#"command ["sleep", "5"]"#)
            .unwrap()
            .remove(0);

        let start = std::time::Instant::now();
        let res = runner.check(&sig, Path::new(".")).await;
        let elapsed = start.elapsed();

        assert!(!res.ok, "Command should have timed out");
        assert!(
            res.detail.contains("timed out"),
            "Detail should mention timeout, got: {}",
            res.detail
        );
        assert!(
            elapsed < Duration::from_secs(3),
            "Elapsed should be ~1s, was {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_file_signal_checks() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("test.json");
        let content = br#"{"version": "1.0", "status": "ready"}"#;
        std::fs::write(&file_path, content).unwrap();

        let runner = SystemSignalRunner::new();

        let sig = parse_signals("file test.json").unwrap().remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(res.ok);

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let sha = format!("{:x}", hasher.finalize());

        let sig = parse_signals(&format!("file test.json sha256={}", sha))
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(res.ok);

        let sig = parse_signals("file test.json sha256=0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(!res.ok);

        let sig = parse_signals(r#"file test.json json:.status="ready""#)
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(res.ok);

        let sig = parse_signals(r#"file test.json json:.status="error""#)
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(!res.ok);
    }

    #[tokio::test]
    async fn test_command_execution_behavior_and_constraints() {
        let temp = tempfile::tempdir().unwrap();
        let runner = SystemSignalRunner::new();

        // 1. Success with stdout constraint matching
        let sig = parse_signals(r#"command ["printf", "%s", "match_123"] [stdout~"^match_\d+$"]"#)
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(
            res.ok,
            "Expected pass for matching stdout regex, got: {:?}",
            res
        );

        // 2. Stdout constraint mismatch
        let sig = parse_signals(r#"command ["printf", "%s", "wrong_abc"] [stdout~"^match_\d+$"]"#)
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(
            !res.ok,
            "Expected fail for mismatched stdout regex, got: {:?}",
            res
        );
        assert!(
            res.detail.contains("did not match regex"),
            "Expected mismatch message, got: {}",
            res.detail
        );

        // 3. Nonzero exit status
        let sig = parse_signals(r#"command ["sh", "-c", "exit 42"]"#)
            .unwrap()
            .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(!res.ok, "Expected fail for nonzero exit, got: {:?}", res);
        assert!(
            res.detail.contains("42"),
            "Expected detail to contain exit code 42, got: {}",
            res.detail
        );

        // 4. Working directory (cwd) respected
        let witness_file = temp.path().join("witness.txt");
        std::fs::write(&witness_file, "token_from_cwd_file").unwrap();
        let sig =
            parse_signals(r#"command ["cat", "witness.txt"] [stdout~"^token_from_cwd_file$"]"#)
                .unwrap()
                .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(res.ok, "Expected pass reading file in cwd, got: {:?}", res);

        // 5. Literal argv: no shell expansion performed on arguments
        let sig = parse_signals(
            r#"command ["printf", "%s", "$UNEXPANDED_VAR"] [stdout~"^\$UNEXPANDED_VAR$"]"#,
        )
        .unwrap()
        .remove(0);
        let res = runner.check(&sig, temp.path()).await;
        assert!(
            res.ok,
            "Expected literal argv passing without shell expansion, got: {:?}",
            res
        );
    }

    #[test]
    #[ignore]
    fn helper_signals_hung_child() {
        let sock_path = match std::env::var_os("OMON_SIGNALS_TEST_SOCK") {
            Some(p) => std::path::PathBuf::from(p),
            None => {
                // If invoked without the child marker/sync socket (e.g. broad `cargo test -- --ignored`),
                // return immediately to avoid hanging the test suite.
                return;
            }
        };

        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = writeln!(stdout, "READY");
        let _ = stdout.flush();

        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            let _stream = match UnixStream::connect(&sock_path) {
                Ok(mut stream) => {
                    let pid = std::process::id();
                    let _ = writeln!(stream, "{}", pid);
                    let _ = stream.flush();
                    Some(stream)
                }
                Err(_) => None,
            };

            // Retain socket ownership throughout blocked lifetime so EOF is only generated
            // upon actual process termination.
            unsafe {
                libc::pause();
            }
            std::hint::black_box(_stream);
        }

        #[cfg(not(unix))]
        loop {
            std::thread::park();
        }
    }

    #[tokio::test]
    async fn test_command_timeout_and_process_cleanup() {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::AsyncReadExt;

        let temp = tempfile::tempdir().unwrap();
        let sock_path = temp.path().join("child_sync.sock");
        let listener = tokio::net::UnixListener::bind(&sock_path).expect("bind sync socket");

        let current_exe = std::env::current_exe().expect("current test exe");
        let sig = parse_signals(&format!(
            r#"command ["{}", "lanes::signals::tests::helper_signals_hung_child", "--ignored", "--exact", "--nocapture"]"#,
            current_exe.display()
        ))
        .unwrap()
        .remove(0);

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_clone = notify.clone();
        let runner = SystemSignalRunner::new()
            .with_command_timeout(Duration::from_millis(100))
            .with_command_setup_hook({
                let sock_path = sock_path.clone();
                move |cmd| {
                    cmd.env("OMON_SIGNALS_TEST_SOCK", &sock_path);
                }
            })
            .with_child_ready_hook(move |_pid| {
                let notify = notify_clone.clone();
                async move {
                    notify.notified().await;
                }
            });
        let cwd = temp.path().to_path_buf();

        // 1. Spawn runner task: child is spawned and pauses in hook prior to timeout activation
        let runner_task = tokio::spawn(async move { runner.check(&sig, &cwd).await });

        // 2. Explicit child-ready observation: await connection on sync socket boundedly
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("child must connect within 5s")
            .expect("listener accept");

        // 3. Bounded read of PID from child
        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("read pid within 5s")
            .expect("read line");
        let child_pid: i32 = line.trim().parse().expect("valid child pid");
        assert!(child_pid > 0);

        // 4. Deterministic non-blocking check: verify stream is held open and NOT at EOF while child is paused
        let stream = reader.into_inner();
        let mut buf = [0u8; 1];
        match stream.try_read(&mut buf) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Stream is held open and connected with no EOF pending
            }
            other => panic!(
                "expected stream to be held open and return WouldBlock while child is paused, got: {:?}",
                other
            ),
        }

        #[cfg(unix)]
        {
            let res = unsafe { libc::kill(child_pid, 0) };
            assert_eq!(
                res, 0,
                "child process must be alive prior to timeout activation"
            );
        }

        // 5. Release readiness barrier: NOW timeout deadline activates on the ready child!
        notify.notify_one();

        // 6. Await runner completion: should time out after 100ms
        let res = tokio::time::timeout(Duration::from_secs(5), runner_task)
            .await
            .expect("runner task must complete within 5s")
            .expect("task join");

        assert!(!res.ok, "Expected timeout failure");
        assert!(
            res.detail.contains("timed out"),
            "Detail should state timed out: {}",
            res.detail
        );

        // 7. Bounded cleanup verification: confirm the child process closed its socket upon termination
        let eof = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
            .await
            .expect("child termination signal within 5s")
            .expect("socket read");
        assert_eq!(
            eof, 0,
            "child process socket must reach EOF upon timeout kill"
        );
    }

    #[tokio::test]
    async fn test_command_failure_detail_keeps_bounded_output_tail() {
        let temp = tempfile::tempdir().unwrap();
        let sig = parse_signals(
            r#"command ["sh", "-c", "yes x | head -c 200000; printf TAIL_END; exit 3"]"#,
        )
        .unwrap()
        .remove(0);
        let res = SystemSignalRunner::new().check(&sig, temp.path()).await;
        assert!(!res.ok);
        assert!(res.detail.contains("TAIL_END"), "tail must be kept");
        assert!(
            res.detail.len() < 16 * 1024,
            "detail must be bounded, got {} bytes",
            res.detail.len()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_command_timeout_kills_whole_process_group() {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::AsyncReadExt;

        let temp = tempfile::tempdir().unwrap();
        let sock_path = temp.path().join("grandchild_sync.sock");
        let listener = tokio::net::UnixListener::bind(&sock_path).expect("bind sync socket");
        let current_exe = std::env::current_exe().expect("current test exe");
        let sig = parse_signals(&format!(
            r#"command ["sh", "-c", "$0 lanes::signals::tests::helper_signals_hung_child --ignored --exact --nocapture & wait", "{}"]"#,
            current_exe.display()
        ))
        .unwrap()
        .remove(0);
        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        let notify_clone = notify.clone();
        let runner = SystemSignalRunner::new()
            .with_command_timeout(Duration::from_millis(100))
            .with_command_setup_hook({
                let sock_path = sock_path.clone();
                move |cmd| {
                    cmd.env("OMON_SIGNALS_TEST_SOCK", &sock_path);
                }
            })
            .with_child_ready_hook(move |_pid| {
                let notify = notify_clone.clone();
                async move { notify.notified().await }
            });
        let cwd = temp.path().to_path_buf();
        let runner_task = tokio::spawn(async move { runner.check(&sig, &cwd).await });

        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("grandchild must connect within 5s")
            .expect("listener accept");
        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("read pid within 5s")
            .expect("read line");
        let grandchild: i32 = line.trim().parse().expect("valid grandchild pid");
        notify.notify_one();
        let res = tokio::time::timeout(Duration::from_secs(5), runner_task)
            .await
            .expect("runner task must complete within 5s")
            .expect("task join");
        assert!(res.detail.contains("timed out"), "{}", res.detail);

        let mut buf = [0u8; 1];
        let eof = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await;
        if !matches!(eof, Ok(Ok(0))) {
            unsafe { libc::kill(grandchild, libc::SIGKILL) };
            panic!("grandchild survived the command timeout: {eof:?}");
        }
    }

    #[tokio::test]
    async fn test_command_caller_cancellation_cleans_up_child() {
        use tokio::io::AsyncBufReadExt;
        use tokio::io::AsyncReadExt;

        let temp = tempfile::tempdir().unwrap();
        let sock_path = temp.path().join("child_sync.sock");
        let listener = tokio::net::UnixListener::bind(&sock_path).expect("bind sync socket");

        let current_exe = std::env::current_exe().expect("current test exe");
        let sig = parse_signals(&format!(
            r#"command ["{}", "lanes::signals::tests::helper_signals_hung_child", "--ignored", "--exact", "--nocapture"]"#,
            current_exe.display()
        ))
        .unwrap()
        .remove(0);

        let runner = SystemSignalRunner::new()
            .with_command_timeout(Duration::from_secs(60))
            .with_command_setup_hook({
                let sock_path = sock_path.clone();
                move |cmd| {
                    cmd.env("OMON_SIGNALS_TEST_SOCK", &sock_path);
                }
            });
        let cwd = temp.path().to_path_buf();

        // 1. Spawn runner task
        let runner_task = tokio::spawn(async move { runner.check(&sig, &cwd).await });

        // 2. Explicit child-ready observation: await connection on sync socket boundedly
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("child must connect within 5s")
            .expect("listener accept");

        // 3. Bounded read of PID from child
        let mut reader = tokio::io::BufReader::new(&mut stream);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("read pid within 5s")
            .expect("read line");
        let child_pid: i32 = line.trim().parse().expect("valid child pid");
        assert!(child_pid > 0);

        // 4. Deterministic non-blocking check: verify stream is held open and NOT at EOF while child is paused
        let stream = reader.into_inner();
        let mut buf = [0u8; 1];
        match stream.try_read(&mut buf) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Stream is held open and connected with no EOF pending
            }
            other => panic!(
                "expected stream to be held open and return WouldBlock while child is paused, got: {:?}",
                other
            ),
        }

        #[cfg(unix)]
        {
            let res = unsafe { libc::kill(child_pid, 0) };
            assert_eq!(res, 0, "child process must be alive before cancellation");
        }

        // 5. Trigger cancellation: abort the runner task and assert join reports cancellation
        runner_task.abort();
        let join_res = runner_task.await;
        assert!(
            join_res.unwrap_err().is_cancelled(),
            "runner task join error must report cancellation"
        );

        // 6. Bounded cleanup verification: confirm the child process closed its socket (terminated)
        let eof = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
            .await
            .expect("child termination signal within 5s")
            .expect("socket read");
        assert_eq!(
            eof, 0,
            "child process socket must reach EOF upon task cancellation"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_gh_merged_and_open_prs() {
        let runner = SystemSignalRunner::new();
        let merged_sig = parse_signals("merged code-yeongyu/oh-my-openagent#9896")
            .unwrap()
            .remove(0);
        let res_merged = runner.check(&merged_sig, Path::new(".")).await;
        println!("REAL GH MERGED RESULT: {:?}", res_merged);
        assert!(res_merged.ok, "PR 9896 should be merged: {:?}", res_merged);

        let open_sig = parse_signals("merged code-yeongyu/oh-my-openagent#9907")
            .unwrap()
            .remove(0);
        let res_open = runner.check(&open_sig, Path::new(".")).await;
        println!("REAL GH OPEN RESULT: {:?}", res_open);
        assert!(
            !res_open.ok,
            "PR 9907 should be open (not merged): {:?}",
            res_open
        );
    }
}

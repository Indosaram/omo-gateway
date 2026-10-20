use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::autonomy::{classify_argv, Effect};
use crate::{OutboundAction, OutboundDispatcher, SessionKey};

pub const ENABLED_ENV: &str = "OMON_REPORT_CLAIMS_ENABLED";
pub const DEFAULT_REMEASURE_DELAY: Duration = Duration::from_secs(30);
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
pub const STALE_CLAIM_AGE_SECS: i64 = 600;
const STDOUT_DISPLAY_CHARS: usize = 300;

pub fn report_claims_enabled() -> bool {
    !matches!(
        std::env::var(ENABLED_ENV)
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Ok("0" | "false" | "no" | "off")
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedClaim {
    pub sentence: String,
    pub argv: Vec<String>,
    pub expect_regex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReportClaimRow {
    pub id: i64,
    pub session_key: String,
    pub platform_message_id: String,
    pub sentence: String,
    pub argv_json: String,
    pub expect_regex: String,
    pub state: String,
    pub measured_output: Option<String>,
    pub created_at: String,
    pub checked_at: Option<String>,
}

/// Strips `[[measure [...] expect~"..."]]` markers from cumulative stream chunks — holding back
/// an incomplete `[[...` suffix until it closes — so no marker text ever reaches the user.
#[derive(Default, Clone)]
pub struct StreamClaimFilter;

impl StreamClaimFilter {
    pub fn new() -> Self {
        Self
    }

    /// Accepts the latest stream chunk text (either cumulative or delta) and returns the safe,
    /// marker-stripped text to display right now. When `!is_final`, any trailing incomplete
    /// `[[...` marker prefix is held back until closed or finalized.
    pub fn filter_chunk(
        &self,
        session_key: &str,
        stream_id: Uuid,
        chunk_text: &str,
        is_final: bool,
    ) -> (String, Vec<ExtractedClaim>) {
        let _ = (session_key, stream_id);
        // Gateway stream chunks are cumulative (each carries the whole reply so far), so no
        // per-stream state is kept: a stream that never sends its final chunk (failed or
        // stopped turn) must not pin its partial text in memory forever.
        if is_final {
            strip_and_extract_claims(chunk_text)
        } else {
            let safe_prefix = hold_incomplete_marker_suffix(chunk_text);
            let (stripped, _) = strip_and_extract_claims(safe_prefix);
            (stripped, Vec::new())
        }
    }
}

fn hold_incomplete_marker_suffix(text: &str) -> &str {
    if let Some(open_idx) = text.rfind("[[") {
        let tail = &text[open_idx..];
        if !tail.contains("]]") {
            return &text[..open_idx];
        }
    }
    if let Some(stripped) = text.strip_suffix('[') {
        return stripped;
    }
    text
}

pub fn strip_and_extract_claims(text: &str) -> (String, Vec<ExtractedClaim>) {
    let mut cleaned = String::with_capacity(text.len());
    let mut claims = Vec::new();
    let mut cursor = 0;

    while let Some(rel_start) = text[cursor..].find("[[measure") {
        let start = cursor + rel_start;
        let before = &text[cursor..start];
        cleaned.push_str(before);

        let after_open = &text[start..];
        let Some(end_rel) = find_marker_end(after_open) else {
            cleaned.push_str(&text[start..start + 2]);
            cursor = start + 2;
            continue;
        };
        let marker_body = &after_open[2..end_rel - 2];
        match parse_measure_marker(marker_body) {
            Some((argv, expect_regex)) => claims.push(ExtractedClaim {
                sentence: extract_preceding_sentence(&cleaned),
                argv,
                expect_regex,
            }),
            None => cleaned.push_str(&text[start..start + end_rel]),
        }
        cursor = start + end_rel;
    }

    if cursor < text.len() {
        cleaned.push_str(&text[cursor..]);
    }

    let normalized = collapse_double_spaces(&cleaned);
    (normalized, claims)
}

fn find_marker_end(s: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut escaped = false;
    let bytes = s.as_bytes();
    let mut i = 2;
    while i + 1 < bytes.len() {
        let b = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' && in_quotes {
            escaped = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if !in_quotes && b == b']' && bytes[i + 1] == b']' {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

fn parse_measure_marker(body: &str) -> Option<(Vec<String>, String)> {
    let rest = body.trim().strip_prefix("measure")?.trim();
    let bracket_start = rest.find('[')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut esc = false;
    let mut bracket_end = None;
    for (idx, ch) in rest[bracket_start..].char_indices() {
        if esc {
            esc = false;
            continue;
        }
        if ch == '\\' && in_str {
            esc = true;
            continue;
        }
        if ch == '"' {
            in_str = !in_str;
            continue;
        }
        if !in_str {
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    bracket_end = Some(bracket_start + idx + 1);
                    break;
                }
            }
        }
    }
    let end = bracket_end?;
    let argv_slice = &rest[bracket_start..end];
    let argv: Vec<String> = serde_json::from_str(argv_slice).ok()?;
    if argv.is_empty() {
        return None;
    }
    let after_argv = rest[end..].trim();
    let after_expect = after_argv.strip_prefix("expect~")?.trim();
    let expect_regex: String = if after_expect.starts_with('"') {
        quoted_expect_value(after_expect)?
    } else {
        after_expect.to_string()
    };
    Some((argv, expect_regex))
}

/// A quoted `expect~` value: strict JSON when the model escaped it, otherwise a single properly
/// closed quoted span (`expect~"^\d+$"` is a valid regex but not valid JSON). Anything with text
/// after the closing quote is malformed and yields no claim, so a broken marker is never turned
/// into a garbage regex.
fn quoted_expect_value(raw: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<String>(raw) {
        return Some(value);
    }
    let inner = raw.strip_prefix('"')?;
    let mut chars = inner.char_indices();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '\\' => {
                chars.next()?;
            }
            '"' => return (index + 1 == inner.len()).then(|| inner[..index].to_string()),
            _ => {}
        }
    }
    None
}

fn extract_preceding_sentence(cleaned_so_far: &str) -> String {
    let trimmed = cleaned_so_far.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    let last_line = trimmed.lines().last().unwrap_or(trimmed).trim();
    for sep in [". ", "! ", "? ", "다. ", "요. "] {
        if let Some((_, tail)) = last_line.rsplit_once(sep) {
            if !tail.trim().is_empty() {
                return tail.trim().to_string();
            }
        }
    }
    last_line.to_string()
}

fn collapse_double_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line.trim_end());
    }
    out
}

pub async fn record_and_schedule_claims(
    pool: SqlitePool,
    dispatcher: Arc<dyn OutboundDispatcher>,
    session: SessionKey,
    platform_message_id: String,
    claims: Vec<ExtractedClaim>,
    cwd: PathBuf,
    delay: Duration,
) -> sqlx::Result<Vec<i64>> {
    if !report_claims_enabled() || claims.is_empty() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let cwd_str = cwd.to_string_lossy().to_string();
    for claim in claims {
        let classification = classify_argv(&claim.argv, Some(&cwd_str));
        let argv_json = serde_json::to_string(&claim.argv).unwrap_or_else(|_| "[]".into());
        if classification.effect != Effect::Read || !remeasure_allowed(&claim.argv) {
            let res = sqlx::query(
                "INSERT INTO report_claims
                    (session_key, platform_message_id, sentence, argv_json, expect_regex, state, checked_at)
                 VALUES (?, ?, ?, ?, ?, 'refused', CURRENT_TIMESTAMP)",
            )
            .bind(session.storage_key())
            .bind(&platform_message_id)
            .bind(&claim.sentence)
            .bind(&argv_json)
            .bind(&claim.expect_regex)
            .execute(&pool)
            .await?;
            ids.push(res.last_insert_rowid());
            continue;
        }

        let res = sqlx::query(
            "INSERT INTO report_claims
                (session_key, platform_message_id, sentence, argv_json, expect_regex, state)
             VALUES (?, ?, ?, ?, ?, 'pending')",
        )
        .bind(session.storage_key())
        .bind(&platform_message_id)
        .bind(&claim.sentence)
        .bind(&argv_json)
        .bind(&claim.expect_regex)
        .execute(&pool)
        .await?;
        let claim_id = res.last_insert_rowid();
        ids.push(claim_id);

        let pool_clone = pool.clone();
        let disp_clone = dispatcher.clone();
        let sess_clone = session.clone();
        let cwd_clone = cwd.clone();
        tokio::spawn(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let _ = execute_pending_claim(
                &pool_clone,
                disp_clone.as_ref(),
                &sess_clone,
                claim_id,
                &cwd_clone,
            )
            .await;
        });
    }
    Ok(ids)
}

pub async fn execute_pending_claim(
    pool: &SqlitePool,
    dispatcher: &dyn OutboundDispatcher,
    session: &SessionKey,
    claim_id: i64,
    cwd: &Path,
) -> sqlx::Result<String> {
    let row: Option<ReportClaimRow> =
        sqlx::query_as("SELECT * FROM report_claims WHERE id = ? AND state = 'pending'")
            .bind(claim_id)
            .fetch_optional(pool)
            .await?;
    let Some(row) = row else {
        return Ok("skipped".into());
    };

    let argv: Vec<String> = match serde_json::from_str::<Vec<String>>(&row.argv_json) {
        Ok(v) if !v.is_empty() => v,
        _ => {
            mark_claim_state(pool, claim_id, "unmeasurable", None).await?;
            return Ok("unmeasurable".into());
        }
    };

    let classification = classify_argv(&argv, cwd.to_str());
    if classification.effect != Effect::Read || !remeasure_allowed(&argv) {
        mark_claim_state(pool, claim_id, "refused", None).await?;
        return Ok("refused".into());
    }

    let regex = match Regex::new(&row.expect_regex) {
        Ok(r) => r,
        Err(_) => {
            mark_claim_state(pool, claim_id, "unmeasurable", None).await?;
            return Ok("unmeasurable".into());
        }
    };

    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    if cwd.exists() {
        cmd.current_dir(cwd);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);
    for (name, _) in std::env::vars_os() {
        let upper = name.to_string_lossy().to_ascii_uppercase();
        if upper.contains("TOKEN")
            || upper.contains("SECRET")
            || upper.contains("KEY")
            || upper.contains("PASSWORD")
        {
            cmd.env_remove(&name);
        }
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            mark_claim_state(pool, claim_id, "unmeasurable", None).await?;
            return Ok("unmeasurable".into());
        }
    };

    let output = match tokio::time::timeout(COMMAND_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(out)) if out.status.success() => out,
        _ => {
            mark_claim_state(pool, claim_id, "unmeasurable", None).await?;
            return Ok("unmeasurable".into());
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let first_300: String = stdout.chars().take(STDOUT_DISPLAY_CHARS).collect();

    if regex.is_match(&stdout) {
        mark_claim_state(pool, claim_id, "confirmed", Some(&first_300)).await?;
        Ok("confirmed".into())
    } else {
        let correction = format!("정정: {} -> 실제 측정값: {}", row.sentence, first_300);
        let reply_to = if row.platform_message_id.is_empty() {
            session.thread_id.clone()
        } else {
            Some(row.platform_message_id.clone())
        };
        let _ = dispatcher
            .dispatch(OutboundAction::SendMessage {
                session: session.clone(),
                content: correction,
                reply_to,
            })
            .await;
        mark_claim_state(pool, claim_id, "corrected", Some(&first_300)).await?;
        Ok("corrected".into())
    }
}

/// Strict allowlist for claim re-measurement on top of the `read` classification: only a few
/// read-only programs, invoked by bare name, and never on secret-looking paths. Output of a
/// mismatch is posted back to the chat, so anything outside this list is refused.
fn remeasure_allowed(argv: &[String]) -> bool {
    let Some(program) = argv.first() else {
        return false;
    };
    let args = &argv[1..];
    let sensitive = |arg: &str| {
        let lower = arg.to_ascii_lowercase();
        let base = lower.rsplit('/').next().unwrap_or(&lower);
        [".ssh", ".aws", ".gnupg", "key", "secret", "token"]
            .iter()
            .any(|needle| lower.contains(needle))
            || base.starts_with(".env")
    };
    match program.as_str() {
        "echo" | "wc" | "ls" | "stat" | "test" => !args.iter().any(|a| sensitive(a)),
        "date" => !args
            .iter()
            .any(|a| a == "-s" || a.starts_with("--set") || a.starts_with("-f")),
        "cat" | "head" | "tail" => !args.iter().any(|a| sensitive(a)),
        "git" => {
            // The subcommand must come first: no global options such as -c/--config-env/-C.
            let Some(sub) = args.first() else {
                return false;
            };
            let rest = &args[1..];
            let no_writes = !rest.iter().any(|a| {
                a.starts_with("--output")
                    || a.starts_with("--ext-diff")
                    || a.starts_with("--config-env")
                    || a == "-c"
            });
            match sub.as_str() {
                "status" | "log" | "rev-parse" | "show" | "diff" => no_writes,
                "branch" => rest.iter().all(|a| {
                    matches!(
                        a.as_str(),
                        "-a" | "-r"
                            | "-v"
                            | "-vv"
                            | "--all"
                            | "--remotes"
                            | "--list"
                            | "--show-current"
                            | "--merged"
                            | "--no-merged"
                    )
                }),
                _ => false,
            }
        }
        _ => false,
    }
}

async fn mark_claim_state(
    pool: &SqlitePool,
    claim_id: i64,
    state: &str,
    measured_output: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE report_claims
         SET state = ?, measured_output = ?, checked_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(state)
    .bind(measured_output)
    .bind(claim_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn recover_stale_report_claims(
    pool: &SqlitePool,
    _now: DateTime<Utc>,
) -> sqlx::Result<u64> {
    let res = sqlx::query(
        "UPDATE report_claims
         SET state = 'unmeasurable', checked_at = CURRENT_TIMESTAMP
         WHERE state = 'pending'",
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unclosed_measure_marker_keeps_rest_of_reply() {
        let text = "Result is ready [[measure [\"echo\", \"test\"] expect~\"test\" but forgot closing. And more text.";
        let (cleaned, claims) = strip_and_extract_claims(text);
        assert!(cleaned.contains("And more text."));
        assert!(cleaned.contains("[[measure"));
        assert!(claims.is_empty());

        let unbalanced = "Notice [[measure [\"echo\"] expect~\"say \"hi\"\" extra]] final words.";
        let (cleaned_unbalanced, claims_unbalanced) = strip_and_extract_claims(unbalanced);
        assert!(cleaned_unbalanced.contains("final words."));
        assert!(claims_unbalanced.is_empty());
    }

    #[tokio::test]
    async fn recover_stale_report_claims_sweeps_young_pending_claims_at_startup() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO report_claims (session_key, platform_message_id, sentence, argv_json, expect_regex, state, created_at)
             VALUES ('s1', 'm1', 'sentence', '[\"echo\"]', 'regex', 'pending', CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let affected = recover_stale_report_claims(&pool, Utc::now())
            .await
            .unwrap();
        assert_eq!(affected, 1);

        let state: String =
            sqlx::query_scalar("SELECT state FROM report_claims WHERE platform_message_id = 'm1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(state, "unmeasurable");
    }
}

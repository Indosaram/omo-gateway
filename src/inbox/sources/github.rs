//! Read-only GitHub source: the owner's own PRs (CI and merge state) plus participating
//! notifications. It only emits on change; the previous snapshot lives in the inbox
//! cursor, so nothing is stored beyond URLs, states and timestamps (never a token).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;

use crate::inbox::{InboxItem, InboxSource, InboxSourceKind};
use crate::{OmonError, Result};

/// Kill switch, read once at gateway startup. Default off until the integration gate.
pub const ENABLED_ENV: &str = "OMON_GITHUB_SOURCE_ENABLED";
pub const POLL_INTERVAL: Duration = Duration::from_secs(300);

/// Read-only GraphQL query: `gh search prs` cannot return statusCheckRollup, so the
/// head commit rollup is read through the search API instead. No mutation is ever sent.
const PR_QUERY: &str = "query{search(query:\"is:pr author:@me archived:false sort:updated-desc\",type:ISSUE,first:50){nodes{... on PullRequest{url title number updatedAt state repository{nameWithOwner} commits(last:1){nodes{commit{oid statusCheckRollup{state}}}}}}}}";
/// GET only; listing notifications does not mark them read.
const NOTIFICATIONS_PATH: &str = "notifications?participating=true&per_page=50";
/// Notification reasons worth the owner's attention. CI activity comes from the PR rollup.
const NOTIFY_REASONS: &[&str] = &[
    "review_requested",
    "mention",
    "team_mention",
    "comment",
    "assign",
    "author",
];

pub fn enabled() -> bool {
    enabled_from(std::env::var(ENABLED_ENV).ok().as_deref())
}

pub fn enabled_from(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

#[derive(Clone, Debug)]
pub struct GithubSource {
    gh_bin: OsString,
}

impl Default for GithubSource {
    fn default() -> Self {
        Self {
            gh_bin: OsString::from("gh"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OpenPr {
    pub url: String,
    pub title: String,
    pub repo: String,
    pub updated_at: String,
    pub ci: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GithubPoll {
    pub items: Vec<InboxItem>,
    pub open_prs: Vec<OpenPr>,
    pub cursor: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Snapshot {
    #[serde(default)]
    prs: BTreeMap<String, PrState>,
    #[serde(default)]
    notes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PrState {
    state: String,
    ci: Option<String>,
    oid: Option<String>,
    updated_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrChange {
    CiFailed,
    CiRecovered,
    Merged,
    Closed,
}

impl PrChange {
    fn tag(self) -> &'static str {
        match self {
            Self::CiFailed => "ci_failed",
            Self::CiRecovered => "ci_recovered",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CiFailed => "CI failed",
            Self::CiRecovered => "CI recovered",
            Self::Merged => "merged",
            Self::Closed => "closed without merge",
        }
    }
}

impl GithubSource {
    pub fn with_gh_bin(bin: impl Into<OsString>) -> Self {
        Self { gh_bin: bin.into() }
    }

    pub async fn poll(&self, cursor: Option<&str>) -> Result<GithubPoll> {
        let notifications = self.gh(&["api", NOTIFICATIONS_PATH]).await?;
        let query = format!("query={PR_QUERY}");
        let prs = self.gh(&["api", "graphql", "-f", query.as_str()]).await?;
        Self::diff(cursor, &notifications, &prs)
    }

    async fn gh(&self, args: &[&str]) -> Result<Value> {
        let what = args.get(1).copied().unwrap_or_default();
        let what = what.split('?').next().unwrap_or_default();
        let output = Command::new(&self.gh_bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|error| unavailable(format!("gh could not start: {error}")))?;
        if !output.status.success() {
            let stderr: String = String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(500)
                .collect();
            return Err(unavailable(format!(
                "gh api {what} exited {}: {stderr}",
                output.status
            )));
        }
        serde_json::from_slice(&output.stdout).map_err(|error| {
            OmonError::ToolExecution(format!(
                "github source returned invalid JSON from gh api {what}: {error}"
            ))
        })
    }

    /// Pure change detection. Without a previous snapshot the call only records a
    /// baseline, so a first boot does not flood the inbox with old activity.
    pub fn diff(cursor: Option<&str>, notifications: &Value, prs: &Value) -> Result<GithubPoll> {
        let previous: Option<Snapshot> = cursor
            .filter(|c| !c.trim().is_empty())
            .and_then(|c| serde_json::from_str(c).ok());
        let baseline = previous.is_none();
        let previous = previous.unwrap_or_default();
        let mut next = Snapshot::default();
        let mut items = Vec::new();
        let mut open_prs = Vec::new();

        let nodes = prs
            .pointer("/data/search/nodes")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                let detail = prs
                    .pointer("/errors/0/message")
                    .and_then(Value::as_str)
                    .unwrap_or("missing data.search.nodes");
                OmonError::ToolExecution(format!("github source PR query failed: {detail}"))
            })?;
        for node in nodes {
            // Non-PR search hits come back as empty objects.
            let Some(url) = node.get("url").and_then(Value::as_str) else {
                continue;
            };
            let title = text(node, "title");
            let repo = node
                .pointer("/repository/nameWithOwner")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let commit = node.pointer("/commits/nodes/0/commit");
            let current = PrState {
                state: text(node, "state"),
                ci: commit
                    .and_then(|c| c.pointer("/statusCheckRollup/state"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                oid: commit
                    .and_then(|c| c.get("oid"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                updated_at: text(node, "updatedAt"),
            };
            if current.state == "OPEN" {
                open_prs.push(OpenPr {
                    url: url.to_owned(),
                    title: title.clone(),
                    repo: repo.clone(),
                    updated_at: current.updated_at.clone(),
                    ci: current.ci.clone(),
                });
            }
            if !baseline {
                if let Some(prev) = previous.prs.get(url) {
                    for change in pr_changes(prev, &current) {
                        items.push(pr_item(url, &title, &repo, node, &current, change));
                    }
                }
            }
            next.prs.insert(url.to_owned(), current);
        }

        let threads = notifications.as_array().ok_or_else(|| {
            OmonError::ToolExecution("github notifications response must be a JSON array".into())
        })?;
        for thread in threads {
            let (Some(id), Some(updated)) = (
                thread.get("id").and_then(Value::as_str),
                thread.get("updated_at").and_then(Value::as_str),
            ) else {
                continue;
            };
            next.notes.insert(id.to_owned(), updated.to_owned());
            let reason = thread
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if baseline
                || !NOTIFY_REASONS.contains(&reason)
                || previous.notes.get(id).map(String::as_str) == Some(updated)
            {
                continue;
            }
            items.push(notification_item(thread, reason, updated));
        }

        let cursor = serde_json::to_string(&next)
            .map_err(|error| OmonError::ToolExecution(format!("github snapshot: {error}")))?;
        Ok(GithubPoll {
            items,
            open_prs,
            cursor,
        })
    }
}

fn unavailable(detail: String) -> OmonError {
    OmonError::ToolExecution(format!("github source unavailable: {detail}"))
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn failing(ci: Option<&str>) -> bool {
    matches!(ci, Some("FAILURE" | "ERROR"))
}

fn pr_changes(prev: &PrState, cur: &PrState) -> Vec<PrChange> {
    if prev.state == "OPEN" && cur.state == "MERGED" {
        return vec![PrChange::Merged];
    }
    if prev.state == "OPEN" && cur.state == "CLOSED" {
        return vec![PrChange::Closed];
    }
    if cur.state != "OPEN" {
        return Vec::new();
    }
    let now_failing = failing(cur.ci.as_deref());
    let was_failing = failing(prev.ci.as_deref());
    if now_failing && !(was_failing && prev.oid == cur.oid) {
        vec![PrChange::CiFailed]
    } else if was_failing && cur.ci.as_deref() == Some("SUCCESS") {
        vec![PrChange::CiRecovered]
    } else {
        Vec::new()
    }
}

fn received_at(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|at| at.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn pr_item(
    url: &str,
    title: &str,
    repo: &str,
    node: &Value,
    current: &PrState,
    change: PrChange,
) -> InboxItem {
    let number = node
        .get("number")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    // updatedAt does not move when checks finish, so the change tag (and head commit for
    // CI) keeps distinct events from colliding on one source_ref.
    let mut tag = change.tag().to_owned();
    if matches!(change, PrChange::CiFailed | PrChange::CiRecovered) {
        if let Some(oid) = current.oid.as_deref() {
            tag.push('@');
            tag.extend(oid.chars().take(12));
        }
    }
    let source_ref = format!("gh:{url}:{}:{tag}", current.updated_at);
    let snippet = format!("[{}] {repo}#{number}: {title}", change.label());
    let mut item = InboxItem::new(
        InboxSourceKind::Github,
        source_ref,
        snippet,
        received_at(&current.updated_at),
    );
    item.thread_ref = Some(url.to_owned());
    item.author = Some(repo.to_owned());
    item.title = Some(title.to_owned());
    item.room_type = Some("github".into());
    item.link = Some(url.to_owned());
    item
}

fn notification_item(thread: &Value, reason: &str, updated: &str) -> InboxItem {
    let repo = thread
        .pointer("/repository/full_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let title = thread
        .pointer("/subject/title")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let link = thread
        .pointer("/subject/url")
        .and_then(Value::as_str)
        .and_then(api_to_html)
        .or_else(|| {
            thread
                .pointer("/repository/html_url")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("https://github.com/{repo}"));
    let source_ref = format!("gh:{link}:{updated}");
    let snippet = format!("[{}] {repo}: {title}", reason.replace('_', " "));
    let mut item = InboxItem::new(
        InboxSourceKind::Github,
        source_ref,
        snippet,
        received_at(updated),
    );
    item.thread_ref = Some(link.clone());
    item.author = Some(repo.to_owned());
    item.title = Some(title.to_owned());
    item.room_type = Some("github".into());
    item.link = Some(link);
    item
}

fn api_to_html(api: &str) -> Option<String> {
    let path = api.strip_prefix("https://api.github.com/repos/")?;
    let path = path
        .replacen("/pulls/", "/pull/", 1)
        .replacen("/commits/", "/commit/", 1);
    Some(format!("https://github.com/{path}"))
}

#[async_trait]
impl InboxSource for GithubSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Github
    }

    async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        let poll = self.poll(cursor).await?;
        Ok((poll.items, poll.cursor))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn prs(ci: &str, state: &str) -> Value {
        json!({"data": {"search": {"nodes": [{
            "url": "https://github.com/o/r/pull/7", "title": "t", "number": 7,
            "updatedAt": "2026-10-15T11:57:11Z", "state": state,
            "repository": {"nameWithOwner": "o/r"},
            "commits": {"nodes": [{"commit": {"oid": "abc", "statusCheckRollup": {"state": ci}}}]}
        }, {}]}}})
    }

    #[test]
    fn baseline_then_merge_emits_once() {
        let base = GithubSource::diff(None, &json!([]), &prs("SUCCESS", "OPEN")).unwrap();
        assert!(base.items.is_empty());
        assert_eq!(base.open_prs.len(), 1);
        let merged =
            GithubSource::diff(Some(&base.cursor), &json!([]), &prs("SUCCESS", "MERGED")).unwrap();
        assert_eq!(merged.items.len(), 1);
        assert!(merged.items[0].source_ref.ends_with(":merged"));
        let again = GithubSource::diff(Some(&merged.cursor), &json!([]), &prs("SUCCESS", "MERGED"))
            .unwrap();
        assert!(again.items.is_empty());
    }

    #[test]
    fn api_urls_become_web_links() {
        assert_eq!(
            api_to_html("https://api.github.com/repos/o/r/pulls/7").as_deref(),
            Some("https://github.com/o/r/pull/7")
        );
        assert_eq!(
            api_to_html("https://api.github.com/repos/o/r/issues/3").as_deref(),
            Some("https://github.com/o/r/issues/3")
        );
    }

    #[test]
    fn kill_switch_defaults_off() {
        assert!(!enabled_from(None));
        assert!(!enabled_from(Some("false")));
        assert!(enabled_from(Some("true")));
    }
}

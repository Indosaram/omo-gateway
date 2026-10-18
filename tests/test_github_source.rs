use std::path::{Path, PathBuf};

use omon_gateway::inbox::sources::github::GithubSource;
use omon_gateway::inbox::{collect_all, prefilter::PrefilterRules, InboxSource};
use omon_gateway::storage::Database;
use serde_json::json;

const PR_URL: &str = "https://github.com/o/r/pull/7";

struct FakeGh {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    prs: PathBuf,
}

fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn fake_gh() -> FakeGh {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("gh");
    let prs = dir.path().join("prs.json");
    let notes = dir.path().join("notes.json");
    std::fs::write(&notes, "[]").unwrap();
    write_script(
        &bin,
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$a\" = graphql ]; then cat '{}'; exit 0; fi\ndone\ncat '{}'\n",
            prs.display(),
            notes.display()
        ),
    );
    FakeGh {
        _dir: dir,
        bin,
        prs,
    }
}

fn set_ci(gh: &FakeGh, ci: &str) {
    let body = json!({"data": {"search": {"nodes": [{
        "url": PR_URL, "title": "Fix login", "number": 7,
        "updatedAt": "2026-10-15T11:57:11Z", "state": "OPEN",
        "repository": {"nameWithOwner": "o/r"},
        "commits": {"nodes": [{"commit": {"oid": "abc123", "statusCheckRollup": {"state": ci}}}]}
    }]}}});
    std::fs::write(&gh.prs, body.to_string()).unwrap();
}

#[tokio::test]
async fn unchanged_response_gives_no_items() {
    let gh = fake_gh();
    set_ci(&gh, "SUCCESS");
    let source = GithubSource::with_gh_bin(&gh.bin);

    let (baseline, cursor) = source.fetch_since(None).await.unwrap();
    assert!(baseline.is_empty(), "first poll only records a baseline");
    let (items, next) = source.fetch_since(Some(&cursor)).await.unwrap();

    assert_eq!(items.len(), 0);
    assert_eq!(next, cursor);
}

#[tokio::test]
async fn ci_flip_gives_one_item() {
    let gh = fake_gh();
    set_ci(&gh, "SUCCESS");
    let source = GithubSource::with_gh_bin(&gh.bin);
    let (_, cursor) = source.fetch_since(None).await.unwrap();

    set_ci(&gh, "FAILURE");
    let (items, cursor) = source.fetch_since(Some(&cursor)).await.unwrap();

    assert_eq!(items.len(), 1);
    assert!(items[0]
        .source_ref
        .starts_with(&format!("gh:{PR_URL}:2026-10-15T11:57:11Z")));
    assert_eq!(items[0].link.as_deref(), Some(PR_URL));
    assert!(items[0].snippet.contains("CI failed"));

    let (again, _) = source.fetch_since(Some(&cursor)).await.unwrap();
    assert!(again.is_empty(), "a still-failing PR is not reported twice");
}

#[tokio::test]
async fn gh_exit_one_is_a_source_error() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("gh");
    write_script(&bin, "#!/bin/sh\necho 'gh: not logged in' >&2\nexit 1\n");
    let source = GithubSource::with_gh_bin(&bin);

    let error = source.fetch_since(None).await.unwrap_err().to_string();
    assert!(error.contains("github source unavailable"), "{error}");

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let results = collect_all(
        database.pool(),
        &[&source as &dyn InboxSource],
        &PrefilterRules::default(),
    )
    .await;
    assert!(results[0]
        .error
        .as_deref()
        .is_some_and(|e| e.contains("github source unavailable")));
    assert_eq!(results[0].stats.inserted, 0);
}

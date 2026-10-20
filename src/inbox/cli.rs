use clap::{Args, Subcommand, ValueEnum};
use sqlx::SqlitePool;

use super::{
    list::{self, ClassFilter},
    set::{self, Status},
    sources::github::GithubSource,
    InboxSource,
};

#[derive(Debug, Args)]
pub struct InboxArgs {
    #[command(subcommand)]
    pub command: InboxCommand,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ListStatus {
    Open,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CollectSource {
    Github,
}

#[derive(Debug, Subcommand)]
pub enum InboxCommand {
    List {
        #[arg(long, value_enum)]
        class: Option<ClassFilter>,
        #[arg(long, value_enum)]
        status: Option<ListStatus>,
        #[arg(long)]
        json: bool,
    },
    Set {
        item_id: String,
        #[arg(long, value_enum)]
        status: Status,
    },
    /// Runs one read-only collection for a source; --dry-run prints without storing.
    Collect {
        #[arg(long, value_enum)]
        source: CollectSource,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug)]
pub struct CliOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute(pool: &SqlitePool, args: InboxArgs) -> CliOutput {
    let result: Result<String, String> = async {
        match args.command {
            InboxCommand::List {
                class,
                status,
                json,
            } => {
                let items = list::list(pool, class, status.is_some())
                    .await
                    .map_err(|e| e.to_string())?;
                if json {
                    serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
                } else {
                    Ok(list::render(&items))
                }
            }
            InboxCommand::Set { item_id, status } => {
                set::set(pool, &item_id, status)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(String::new())
            }
            InboxCommand::Collect {
                source: CollectSource::Github,
                dry_run,
                json,
            } => collect_github(pool, &GithubSource::default(), dry_run, json).await,
        }
    }
    .await;
    match result {
        Ok(stdout) => CliOutput {
            code: 0,
            stdout,
            stderr: String::new(),
        },
        Err(stderr) => CliOutput {
            code: 2,
            stdout: String::new(),
            stderr,
        },
    }
}

struct PrePolledSource {
    source: super::InboxSourceKind,
    items: Vec<super::InboxItem>,
    cursor: String,
}

#[async_trait::async_trait]
impl InboxSource for PrePolledSource {
    fn name(&self) -> super::InboxSourceKind {
        self.source
    }

    async fn fetch_since(
        &self,
        _cursor: Option<&str>,
    ) -> crate::Result<(Vec<super::InboxItem>, String)> {
        Ok((self.items.clone(), self.cursor.clone()))
    }
}

pub async fn collect_github(
    pool: &SqlitePool,
    source: &GithubSource,
    dry_run: bool,
    json: bool,
) -> Result<String, String> {
    let cursor: Option<String> =
        sqlx::query_scalar("SELECT cursor FROM inbox_cursors WHERE source = 'github'")
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    let poll = source
        .poll(cursor.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    let mut stored = None;
    if !dry_run {
        let pre_polled = PrePolledSource {
            source: super::InboxSourceKind::Github,
            items: poll.items.clone(),
            cursor: poll.cursor.clone(),
        };
        let results = super::collect_all(
            pool,
            &[&pre_polled as &dyn InboxSource],
            &super::prefilter::PrefilterRules::default(),
        )
        .await;
        if let Some(error) = results.first().and_then(|r| r.error.clone()) {
            return Err(error);
        }
        stored = results.first().map(|r| r.stats.inserted);
    }
    if json {
        let items: Vec<_> = poll
            .items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.source_ref,
                    "title": item.title,
                    "snippet": item.snippet,
                    "link": item.link,
                })
            })
            .collect();
        return serde_json::to_string_pretty(&serde_json::json!({
            "source": "github",
            "dry_run": dry_run,
            "open_prs": poll.open_prs,
            "changes": items,
            "stored": stored,
        }))
        .map_err(|e| e.to_string());
    }
    let mut out = format!("open PRs: {}\n", poll.open_prs.len());
    for pr in &poll.open_prs {
        out.push_str(&format!(
            "- {} [{}] {}\n",
            pr.url,
            pr.ci.as_deref().unwrap_or("no checks"),
            pr.title
        ));
    }
    out.push_str(&format!("changes: {}", poll.items.len()));
    for item in &poll.items {
        out.push_str(&format!("\n- {}", item.snippet));
    }
    Ok(out)
}

pub async fn run(args: InboxArgs) -> i32 {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".into());
    let database = match crate::Database::connect(&url).await {
        Ok(database) => database,
        Err(error) => {
            eprintln!("{error}");
            return 2;
        }
    };
    let output = execute(database.pool(), args).await;
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
    output.code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn collect_github_polls_once_and_prints_stored_changes() {
        use std::os::unix::fs::PermissionsExt;

        // Given a local gh fixture that records every API request, without network access.
        let dir = tempfile::tempdir().unwrap();
        let gh = dir.path().join("gh");
        std::fs::write(
            &gh,
            r#"#!/bin/sh
printf 'call\n' >> "$0.calls"
case "$2" in
  notifications*) printf '%s\n' '[{"id":"note-1","updated_at":"2026-10-20T00:00:00Z","reason":"mention","subject":{"title":"Needs review","url":"https://api.github.com/repos/o/r/pulls/1"},"repository":{"full_name":"o/r"}}]' ;;
  graphql) printf '%s\n' '{"data":{"search":{"nodes":[]}}}' ;;
  *) exit 1 ;;
esac
"#,
        )
        .unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o700)).unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO inbox_cursors(source, cursor, updated_at) VALUES('github', '{}', '2026-10-20T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let source = GithubSource::with_gh_bin(gh.as_os_str());

        // When the real CLI collection path stores and renders the fetched changes.
        let output = collect_github(&pool, &source, false, true).await.unwrap();

        // Then one poll makes two gh requests and its printed change is the stored row.
        let calls = std::fs::read_to_string(dir.path().join("gh.calls")).unwrap();
        assert_eq!(calls.lines().count(), 2);
        let output: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["stored"], 1);
        assert_eq!(output["changes"].as_array().unwrap().len(), 1);
        let stored: (String, String) =
            sqlx::query_as("SELECT source_ref, snippet FROM inbox_items WHERE source = 'github'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(output["changes"][0]["id"], stored.0);
        assert_eq!(output["changes"][0]["snippet"], stored.1);
    }

    #[tokio::test]
    async fn pre_polled_source_commits_without_repolling() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let item = crate::inbox::InboxItem::new(
            crate::inbox::InboxSourceKind::Github,
            "gh:https://github.com/o/r/pull/1:2026-10-20T00:00:00Z:ci_failed",
            "[CI failed] o/r#1: test pr",
            chrono::Utc::now(),
        );

        let pre_polled = PrePolledSource {
            source: crate::inbox::InboxSourceKind::Github,
            items: vec![item],
            cursor: "test_cursor".into(),
        };

        let results = crate::inbox::collect_all(
            &pool,
            &[&pre_polled as &dyn InboxSource],
            &crate::inbox::prefilter::PrefilterRules::default(),
        )
        .await;

        assert_eq!(results.len(), 1);
        assert!(results[0].error.is_none());
        assert_eq!(results[0].stats.inserted, 1);

        let stored_cursor: String =
            sqlx::query_scalar("SELECT cursor FROM inbox_cursors WHERE source = 'github'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored_cursor, "test_cursor");

        let stored_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM inbox_items WHERE source = 'github'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored_count, 1);
    }
}

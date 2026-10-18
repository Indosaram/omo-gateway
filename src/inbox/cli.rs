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
        let results = super::collect_all(
            pool,
            &[source as &dyn InboxSource],
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

use clap::{Args, Subcommand, ValueEnum};
use sqlx::SqlitePool;

use super::{
    list::{self, ClassFilter},
    set::{self, Status},
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

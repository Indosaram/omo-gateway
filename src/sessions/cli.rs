use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::control::{ControlError, SendMode, SessionControl, Target};
use super::external::ExternalDriver;
use crate::storage::Database;

#[derive(Debug, Args)]
pub struct SessionsArgs {
    #[command(subcommand)]
    pub command: SessionsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionsCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Read {
        target: Target,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        cursor: Option<String>,
    },
    Send {
        target: Target,
        #[arg(long)]
        text: String,
        #[arg(long, value_enum, default_value = "auto")]
        mode: SendMode,
    },
    Interrupt {
        target: Target,
    },
    Adopt {
        target: Target,
        #[arg(long)]
        goal: String,
        #[arg(long)]
        done: String,
        #[arg(long)]
        lead: Option<String>,
    },
    Release {
        target: Target,
    },
}

pub async fn run(args: SessionsArgs) -> i32 {
    match execute(args.command).await {
        Ok((code, value)) => {
            println!("{value}");
            code
        }
        Err(error) => {
            eprintln!("{}", json!({"ok":false,"error":error.to_string()}));
            error.exit_code()
        }
    }
}

async fn execute(command: SessionsCommand) -> Result<(i32, Value), ControlError> {
    let control = SessionControl::from_environment()?;
    let value = match command {
        SessionsCommand::List { .. } => {
            let list = control.registry().list_all().await;
            return Ok((list.exit_code, serde_json::to_value(list)?));
        }
        SessionsCommand::Read {
            target,
            limit,
            cursor,
        } => control.read(&target, limit, cursor.as_deref()).await?,
        SessionsCommand::Send { target, text, mode } => {
            control
                .send(&target, &text, mode, uuid::Uuid::new_v4())
                .await?
        }
        SessionsCommand::Interrupt { target } => control.interrupt(&target).await?,
        SessionsCommand::Adopt {
            target,
            goal,
            done,
            lead,
        } => {
            let lead = lead
                .or_else(|| std::env::var("OMON_LANE_DEFAULT_LEAD").ok())
                .ok_or_else(|| failure("--lead or OMON_LANE_DEFAULT_LEAD is required"))?;
            let database = database().await?;
            let driver =
                ExternalDriver::from_environment(database.pool().clone()).map_err(failure)?;
            let result = driver
                .adopt(&target, &goal, &done, &lead)
                .await
                .map_err(failure);
            database.close().await;
            json!({"key":result?,"target":target.to_string(),"kind":"external","status":"working"})
        }
        SessionsCommand::Release { target } => {
            let database = database().await?;
            let driver =
                ExternalDriver::from_environment(database.pool().clone()).map_err(failure)?;
            let result = driver.release(&target).await.map_err(failure);
            database.close().await;
            result?;
            json!({"target":target.to_string(),"released":true})
        }
    };
    Ok((0, value))
}

fn failure(error: impl std::fmt::Display) -> ControlError {
    ControlError::Command {
        code: 2,
        message: error.to_string(),
    }
}

async fn database() -> Result<Database, ControlError> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".into());
    Database::connect(&url).await.map_err(failure)
}

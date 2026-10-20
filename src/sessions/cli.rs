use clap::{Args, Subcommand};
use serde_json::json;

use super::control::{ControlError, SendMode, SessionControl, Target};
use super::external::ExternalDriver;
use super::overview;
use super::watch::{self, Watcher};
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
    Overview {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 1900)]
        max_chars: usize,
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
    Answer {
        target: Target,
        #[arg(long)]
        text: String,
        #[arg(long)]
        request: Option<String>,
    },
    Watch {
        target: Target,
    },
    Unwatch {
        target: Target,
    },
    Watched,
    WatchTick {
        #[arg(long)]
        once: bool,
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
        Ok(code) => code,
        Err(error) => {
            eprintln!("{}", json!({"ok":false,"error":error.to_string()}));
            error.exit_code()
        }
    }
}

async fn execute(command: SessionsCommand) -> Result<i32, ControlError> {
    let control = SessionControl::from_environment()?;
    match command {
        SessionsCommand::List { .. } => {
            let list = control.registry().list_all().await;
            println!("{}", serde_json::to_string(&list)?);
            Ok(list.exit_code)
        }
        SessionsCommand::Overview { json, max_chars } => {
            let overview = overview::collect(&control, 1000).await;
            if json {
                println!("{}", serde_json::to_string(&overview)?);
            } else {
                let formatted = overview::format_overview(&overview, chrono::Utc::now(), max_chars);
                println!("{formatted}");
            }
            Ok(0)
        }
        SessionsCommand::Read {
            target,
            limit,
            cursor,
        } => {
            let value = control.read(&target, limit, cursor.as_deref()).await?;
            println!("{value}");
            Ok(0)
        }
        SessionsCommand::Send { target, text, mode } => {
            let value = control
                .send(&target, &text, mode, uuid::Uuid::new_v4())
                .await?;
            println!("{value}");
            Ok(0)
        }
        SessionsCommand::Interrupt { target } => {
            let value = control.interrupt(&target).await?;
            println!("{value}");
            Ok(0)
        }
        SessionsCommand::Answer {
            target,
            text,
            request,
        } => {
            let value = control.answer(&target, &text, request.as_deref()).await?;
            println!("{value}");
            Ok(0)
        }
        SessionsCommand::Watch { target } => {
            let (resolved_target, _) = control.resolve(&target).await?;
            let canonical = resolved_target.to_string();
            let database = database().await?;
            watch::watch(database.pool(), &canonical)
                .await
                .map_err(failure)?;
            database.close().await;
            println!(
                "{}",
                json!({"ok": true, "target": canonical, "watched": true})
            );
            Ok(0)
        }
        SessionsCommand::Unwatch { target } => {
            let canonical = match control.resolve(&target).await {
                Ok((resolved, _)) => resolved.to_string(),
                Err(_) => target.to_string(),
            };
            let database = database().await?;
            let removed = watch::unwatch(database.pool(), &canonical)
                .await
                .map_err(failure)?;
            database.close().await;
            println!(
                "{}",
                json!({"ok": true, "target": canonical, "unwatched": removed})
            );
            Ok(0)
        }
        SessionsCommand::Watched => {
            let database = database().await?;
            let targets = watch::watched(database.pool()).await.map_err(failure)?;
            database.close().await;
            println!("{}", json!({"ok": true, "watched": targets}));
            Ok(0)
        }
        SessionsCommand::WatchTick { once: _ } => {
            let database = database().await?;
            let mut watcher = Watcher::with_policy(
                database.pool().clone(),
                control,
                std::time::Duration::from_secs(60),
                watch::NotifyPolicy {
                    notify_after: std::time::Duration::ZERO,
                    quiet_hours: None,
                },
            );
            let notices = watcher.tick().await.map_err(failure)?;
            for notice in notices {
                let line = serde_json::to_string(&json!({
                    "dedupe_key": notice.dedupe_key,
                    "target": notice.target,
                    "kind": notice.kind,
                    "text": notice.text,
                }))?;
                println!("{line}");
                watcher.mark_sent(&notice).await.map_err(failure)?;
            }
            database.close().await;
            Ok(0)
        }
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
            // Report the canonical target the lane was stored with, not the user's prefix:
            // `sessions release` needs the full id once the session has ended.
            let stored = match &result {
                Ok(key) => driver
                    .target(key)
                    .await
                    .ok()
                    .flatten()
                    .map(|resolved| resolved.to_string()),
                Err(_) => None,
            };
            database.close().await;
            let key = result?;
            let value = json!({"key":key,"target":stored.unwrap_or_else(|| target.to_string()),"kind":"external","status":"working"});
            println!("{value}");
            Ok(0)
        }
        SessionsCommand::Release { target } => {
            let target = match control.resolve(&target).await {
                Ok((resolved, _)) => resolved,
                Err(_) => target,
            };
            let database = database().await?;
            let driver =
                ExternalDriver::from_environment(database.pool().clone()).map_err(failure)?;
            let result = driver.release(&target).await.map_err(failure);
            database.close().await;
            result?;
            let value = json!({"target":target.to_string(),"released":true});
            println!("{value}");
            Ok(0)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: SessionsCommand,
    }

    #[test]
    fn parse_overview_flags() {
        let parsed =
            TestCli::try_parse_from(["sessions", "overview", "--json", "--max-chars", "1200"])
                .expect("parse overview with args");
        match parsed.command {
            SessionsCommand::Overview { json, max_chars } => {
                assert!(json);
                assert_eq!(max_chars, 1200);
            }
            _ => panic!("expected Overview command"),
        }
    }

    #[test]
    fn parse_overview_defaults() {
        let parsed =
            TestCli::try_parse_from(["sessions", "overview"]).expect("parse overview default");
        match parsed.command {
            SessionsCommand::Overview { json, max_chars } => {
                assert!(!json);
                assert_eq!(max_chars, 1900);
            }
            _ => panic!("expected Overview command"),
        }
    }

    #[test]
    fn parse_answer_flags() {
        let parsed = TestCli::try_parse_from([
            "sessions",
            "answer",
            "local:01a13e55",
            "--text",
            "yes",
            "--request",
            "req-1",
        ])
        .expect("parse answer with request");
        match parsed.command {
            SessionsCommand::Answer {
                target,
                text,
                request,
            } => {
                assert_eq!(target.to_string(), "local:01a13e55");
                assert_eq!(text, "yes");
                assert_eq!(request.as_deref(), Some("req-1"));
            }
            _ => panic!("expected Answer command"),
        }
    }

    #[test]
    fn parse_watch_and_unwatch() {
        let watch_cmd =
            TestCli::try_parse_from(["sessions", "watch", "local:sess-1"]).expect("parse watch");
        match watch_cmd.command {
            SessionsCommand::Watch { target } => {
                assert_eq!(target.to_string(), "local:sess-1");
            }
            _ => panic!("expected Watch"),
        }

        let unwatch_cmd = TestCli::try_parse_from(["sessions", "unwatch", "omarchy:sess-2"])
            .expect("parse unwatch");
        match unwatch_cmd.command {
            SessionsCommand::Unwatch { target } => {
                assert_eq!(target.to_string(), "omarchy:sess-2");
            }
            _ => panic!("expected Unwatch"),
        }

        let watched_cmd = TestCli::try_parse_from(["sessions", "watched"]).expect("parse watched");
        match watched_cmd.command {
            SessionsCommand::Watched => {}
            _ => panic!("expected Watched"),
        }
    }

    #[test]
    fn parse_watch_tick() {
        let parsed = TestCli::try_parse_from(["sessions", "watch-tick", "--once"])
            .expect("parse watch-tick --once");
        match parsed.command {
            SessionsCommand::WatchTick { once } => {
                assert!(once);
            }
            _ => panic!("expected WatchTick"),
        }

        let parsed_no_flag =
            TestCli::try_parse_from(["sessions", "watch-tick"]).expect("parse watch-tick default");
        match parsed_no_flag.command {
            SessionsCommand::WatchTick { once } => {
                assert!(!once);
            }
            _ => panic!("expected WatchTick"),
        }
    }

    #[test]
    fn failure_helper_maps_to_control_error_code_2() {
        let err = failure("test error");
        assert_eq!(err.exit_code(), 2);
        assert_eq!(err.to_string(), "session command failed (2): test error");
    }
}

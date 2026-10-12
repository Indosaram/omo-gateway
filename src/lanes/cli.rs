use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::agent::{model_for_purpose, ModelPurpose};
use crate::judge::{JevCli, Judge, Purpose};
use crate::storage::{get_lane, insert_lane, list_lanes, read_transcript, Database, Lane};
use crate::{OmonError, Result, SessionKey};

use super::guard::{self, GuardThresholds, LaunchDecision};
use super::runner::{lane_key, wake};
use super::signals::parse_signals;
use super::state::{transition, LaneStatus};

#[derive(Debug, Args)]
pub struct LaneArgs {
    #[command(subcommand)]
    pub command: LaneCommand,
}

#[derive(Debug, Subcommand)]
pub enum LaneCommand {
    Start(StartArgs),
    List {
        #[arg(long)]
        status: Option<LaneStatus>,
    },
    Log {
        key: String,
        #[arg(long)]
        last: Option<usize>,
    },
    Send {
        key: String,
        #[arg(long)]
        text: String,
    },
    ClaimDone {
        key: String,
        #[arg(long)]
        evidence: String,
    },
    Object {
        key: String,
        #[arg(long)]
        reason: String,
    },
    Close {
        key: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        note: Option<String>,
    },
    Reopen {
        key: String,
        #[arg(long)]
        text: Option<String>,
    },
    RegisterWorktree {
        key: String,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        path: PathBuf,
    },
    CanLaunch,
    Heavy {
        label: String,
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LaneModel {
    Small,
    Large,
    Auto,
}

#[derive(Debug, Args)]
pub struct StartArgs {
    pub key: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub brief: PathBuf,
    #[arg(long)]
    pub done: String,
    #[arg(long)]
    pub thread: Option<u64>,
    #[arg(long)]
    pub cwd: Option<PathBuf>,
    #[arg(long)]
    pub lead: Option<String>,
    #[arg(long, value_enum, default_value = "auto")]
    pub model: LaneModel,
}

fn invalid(message: impl Into<String>) -> OmonError {
    OmonError::Config(message.into())
}

pub async fn run(args: LaneArgs) -> i32 {
    let result = async {
        let url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".into());
        let database = Database::connect(&url).await?;
        let judge = JevCli::new(None, Some(database.pool().clone()), None, None)
            .init()
            .await
            .map_err(|error| invalid(error.to_string()))?;
        let result = execute(database.pool(), args.command, &judge).await;
        database.close().await;
        result
    }
    .await;
    match result {
        Ok((code, value)) => {
            println!("{value}");
            code
        }
        Err(error) => {
            println!("{}", json!({"ok": false, "error": error.to_string()}));
            2
        }
    }
}

pub async fn execute(
    pool: &SqlitePool,
    command: LaneCommand,
    judge: &dyn Judge,
) -> Result<(i32, Value)> {
    match command {
        LaneCommand::Start(args) => {
            validate_start(&args).await?;
            let decision = guard::can_launch(pool)
                .await
                .map_err(|error| invalid(error.to_string()))?;
            start(pool, args, judge, decision).await
        }
        LaneCommand::List { status } => {
            let statuses: Vec<&str> = status
                .as_ref()
                .map(|status| vec![status.as_str()])
                .unwrap_or_default();
            Ok((0, json!({"lanes": list_lanes(pool, &statuses).await?})))
        }
        LaneCommand::Log { key, last } => {
            require_lane(pool, &key).await?;
            Ok((
                0,
                json!({"key": key, "transcript": read_transcript(pool, &key, last).await?}),
            ))
        }
        LaneCommand::Send { key, text } => {
            if text.trim().is_empty() {
                return Err(invalid("text must not be empty"));
            }
            let lane = require_lane(pool, &key).await?;
            if !matches!(lane.status.as_str(), "working" | "blocked" | "not_done") {
                return Err(invalid("lane must be working, blocked or not_done"));
            }
            enqueue(pool, &key, &text).await?;
            if lane.status == "blocked" {
                transition(pool, &key, LaneStatus::Working, Some("lead message"))
                    .await
                    .map_err(|error| invalid(error.to_string()))?;
            }
            wake(pool).await?;
            Ok((0, json!({"key": key, "queued": true})))
        }
        LaneCommand::ClaimDone { key, evidence } => {
            if evidence.trim().is_empty() {
                return Err(invalid("evidence must not be empty"));
            }
            change_claim(pool, &key, true, &evidence).await?;
            wake(pool).await?;
            Ok((0, json!({"key": key, "status": "done_claimed"})))
        }
        LaneCommand::Object { key, reason } => {
            if reason.trim().is_empty() {
                return Err(invalid("reason must not be empty"));
            }
            change_claim(pool, &key, false, &reason).await?;
            wake(pool).await?;
            let lane = require_lane(pool, &key).await?;
            Ok((0, json!({"key": key, "status": lane.status})))
        }
        LaneCommand::Close { key, force, note } => {
            if force
                && !note
                    .as_deref()
                    .is_some_and(|note| note.starts_with("forced:"))
            {
                return Err(invalid("--force requires --note beginning with forced:"));
            }
            if !force
                && note
                    .as_deref()
                    .is_some_and(|note| note.starts_with("forced:"))
            {
                return Err(invalid("forced note requires --force"));
            }
            let intent =
                crate::storage::lanes::request_close(pool, &key, force, note.as_deref()).await?;
            wake(pool).await?;
            Ok((
                0,
                json!({"key": key, "close_intent": intent, "status": "close_pending"}),
            ))
        }
        LaneCommand::Reopen { key, text } => {
            let lane = require_lane(pool, &key).await?;
            if !matches!(lane.status.as_str(), "closed" | "failed") {
                return Err(invalid(format!(
                    "IllegalTransition: {} -> reopened",
                    lane.status
                )));
            }
            sqlx::query("INSERT INTO lane_owner_replies(message_id,lane_key,author_id,text,generation) SELECT ?,?,?,?,COALESCE(MAX(id),0) FROM lane_history WHERE lane_key=? AND reason='reopened'")
                .bind(format!("cli:{}", uuid::Uuid::new_v4())).bind(&key).bind("local-cli").bind(text).bind(&key).execute(pool).await?;
            wake(pool).await?;
            Ok((0, json!({"key": key, "status": "reopen_pending"})))
        }
        LaneCommand::RegisterWorktree { key, repo, path } => {
            super::runner::register_owned_worktree(pool, &key, &repo, &path).await?;
            Ok((0, json!({"key":key,"registered":true})))
        }
        LaneCommand::CanLaunch => {
            let decision = guard::can_launch(pool)
                .await
                .map_err(|error| invalid(error.to_string()))?;
            Ok((
                if decision.is_hold() { 4 } else { 0 },
                json!({"can_launch": decision.is_ok(), "reasons": decision.reasons()}),
            ))
        }
        LaneCommand::Heavy { label, argv } => {
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| invalid("heavy requires argv"))?;
            let output =
                guard::run_with_heavy_slot(&label, None, &GuardThresholds::from_env(), || async {
                    tokio::process::Command::new(program)
                        .args(args)
                        .kill_on_drop(true)
                        .output()
                        .await
                })
                .await
                .map_err(|error| invalid(error.to_string()))?
                .map_err(super::runner::io_error)?;
            Ok((
                if output.status.success() { 0 } else { 2 },
                json!({"label": label, "exit_code": output.status.code(), "stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr)}),
            ))
        }
    }
}

async fn validate_start(args: &StartArgs) -> Result<()> {
    if args.key.is_empty()
        || args.key.len() > 48
        || !args
            .key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(invalid(
            "key must be 1..48 lowercase letters, digits or hyphens",
        ));
    }
    if args.title.trim().is_empty() {
        return Err(invalid("title must not be empty"));
    }
    parse_signals(&args.done).map_err(|error| invalid(error.to_string()))?;
    let cwd = args
        .cwd
        .clone()
        .unwrap_or(std::env::current_dir().map_err(super::runner::io_error)?);
    if !tokio::fs::metadata(cwd)
        .await
        .map_err(super::runner::io_error)?
        .is_dir()
    {
        return Err(invalid("cwd must be a directory"));
    }
    tokio::fs::read_to_string(&args.brief)
        .await
        .map_err(super::runner::io_error)?;
    if let Some(lead) = &args.lead {
        SessionKey::from_storage_key(lead)?;
    }
    if args.thread == Some(0) {
        return Err(invalid("thread must be a nonzero channel id"));
    }
    Ok(())
}

pub async fn start(
    pool: &SqlitePool,
    args: StartArgs,
    judge: &dyn Judge,
    decision: LaunchDecision,
) -> Result<(i32, Value)> {
    validate_start(&args).await?;
    if get_lane(pool, &args.key).await?.is_some() {
        return Err(invalid("lane key already exists"));
    }
    let cwd = tokio::fs::canonicalize(
        args.cwd
            .clone()
            .unwrap_or(std::env::current_dir().map_err(super::runner::io_error)?),
    )
    .await
    .map_err(super::runner::io_error)?;
    let brief_path = tokio::fs::canonicalize(&args.brief)
        .await
        .map_err(super::runner::io_error)?;
    let brief = tokio::fs::read_to_string(&brief_path)
        .await
        .map_err(super::runner::io_error)?;
    let lead = if let Some(lead) = args.lead {
        lead
    } else {
        let newest: Option<(String,)> = if let Some(thread) = args.thread {
            sqlx::query_as("SELECT session_key FROM sessions WHERE thread_id = ? OR channel_id = ? ORDER BY updated_at DESC LIMIT 1")
                .bind(thread.to_string()).bind(thread.to_string()).fetch_optional(pool).await?
        } else {
            None
        };
        newest
            .map(|(key,)| key)
            .or_else(|| std::env::var("OMON_LANE_DEFAULT_LEAD").ok())
            .ok_or_else(|| {
                invalid("--lead or OMON_LANE_DEFAULT_LEAD is required without a matching session")
            })?
    };
    SessionKey::from_storage_key(&lead)?;
    let purpose = match args.model {
        LaneModel::Small => ModelPurpose::LaneSmall,
        LaneModel::Large => ModelPurpose::LaneLarge,
        LaneModel::Auto => {
            let excerpt: String = brief.chars().take(2000).collect();
            match judge.choice(Purpose::LaneSize, &excerpt,
                "small = one file, obvious, under 30 min; large = multi-file, design, debugging or unclear", &["small", "large"]).await {
                Ok(choice) if choice.selected == "small" => ModelPurpose::LaneSmall,
                Ok(_) => ModelPurpose::LaneLarge,
                Err(error) => { tracing::warn!(%error, "lane size judge unavailable; treating as large"); ModelPurpose::LaneLarge }
            }
        }
    };
    let mut lane = Lane::new(
        &args.key,
        args.title,
        lead,
        cwd.to_string_lossy(),
        brief_path.to_string_lossy(),
        lane_key(&args.key).storage_key(),
        args.done,
    );
    lane.model = Some(model_for_purpose(purpose));
    let mut created_bot = None;
    if !decision.is_hold() {
        if let Some(parent) = args.thread {
            let threads = super::discord_threads::DiscordThreads::from_env()
                .map_err(|error| invalid(error.to_string()))?;
            let http = threads.http();
            created_bot = Some(
                http.get_current_user()
                    .await
                    .map_err(|error| invalid(error.to_string()))?
                    .id
                    .to_string(),
            );
            let thread = super::discord_threads::create_thread(
                http,
                serenity::all::ChannelId::new(parent),
                &lane.key,
                &lane.title,
            )
            .await
            .map_err(|error| invalid(error.to_string()))?;
            lane.thread_channel_id = Some(thread.thread_channel_id.to_string());
        }
        lane.status = "working".into();
    }
    lane.parent_channel_id = args.thread.map(|id| id.to_string());
    insert_lane(pool, &lane).await?;
    crate::usage::capture_lane_start(
        pool,
        &lane.key,
        &cwd,
        match purpose {
            ModelPurpose::LaneSmall => crate::usage::Purpose::LaneSmall,
            ModelPurpose::LaneLarge => crate::usage::Purpose::LaneLarge,
            _ => unreachable!("lane selection only yields lane purposes"),
        },
    )
    .await?;
    if let Some(bot) = created_bot {
        crate::storage::lanes::bind_lane_thread(pool, &lane.key, &bot).await?;
    }
    if !decision.is_hold() {
        wake(pool).await?;
    }
    Ok((
        if decision.is_hold() { 4 } else { 0 },
        json!({"lane": lane, "reasons": decision.reasons()}),
    ))
}

async fn require_lane(pool: &SqlitePool, key: &str) -> Result<Lane> {
    get_lane(pool, key)
        .await?
        .ok_or_else(|| invalid(format!("lane not found: {key}")))
}

pub async fn enqueue(pool: &SqlitePool, key: &str, text: &str) -> Result<()> {
    sqlx::query("INSERT INTO lane_inbox(lane_key, text) VALUES (?, ?)")
        .bind(key)
        .bind(text)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn change_claim(pool: &SqlitePool, key: &str, claim: bool, text: &str) -> Result<()> {
    change_claim_at(pool, key, claim, text, chrono::Utc::now()).await
}

pub async fn change_claim_at(
    pool: &SqlitePool,
    key: &str,
    claim: bool,
    text: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE lanes SET updated_at = updated_at WHERE key = ?")
        .bind(key)
        .execute(&mut *tx)
        .await?;
    let (from, lead): (String, String) =
        sqlx::query_as("SELECT status, lead_session_key FROM lanes WHERE key = ?")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| invalid("lane not found"))?;
    let status: LaneStatus = from
        .parse()
        .map_err(|error: super::state::UnknownLaneStatus| invalid(error.to_string()))?;
    let blocked_answer = !claim && status == LaneStatus::Blocked;
    let target = if claim {
        LaneStatus::DoneClaimed
    } else if blocked_answer {
        LaneStatus::Working
    } else {
        LaneStatus::NotDone
    };
    let generation: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened'",
    )
    .bind(key)
    .fetch_one(&mut *tx)
    .await?;
    if !status.can_transition_to(target, None) {
        return Err(invalid(format!(
            "illegal lane transition: {from} -> {target}"
        )));
    }
    let query = if claim {
        "UPDATE lanes SET status = ?, claim_at = ?, claim_evidence = ?, updated_at = ?, objection = NULL WHERE key = ? AND status = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?"
    } else {
        "UPDATE lanes SET status = ?, claim_at = ?, objection = ?, updated_at = ?, last_turn_outcome = NULL WHERE key = ? AND status = ? AND (SELECT COALESCE(MAX(id), 0) FROM lane_history WHERE lane_key = ? AND reason = 'reopened') = ?"
    };
    let updated = sqlx::query(query)
        .bind(target.as_str())
        .bind(if claim { Some(now) } else { None })
        .bind(text)
        .bind(now)
        .bind(key)
        .bind(&from)
        .bind(key)
        .bind(generation)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() != 1 {
        return Err(invalid("concurrent lane transition"));
    }
    sqlx::query("INSERT INTO lane_history(lane_key, at, from_status, to_status, reason) VALUES (?, ?, ?, ?, ?)")
        .bind(key).bind(now).bind(&from).bind(target.as_str()).bind(text).execute(&mut *tx).await?;
    if !blocked_answer {
        let body = if claim {
            super::scrub::scrub(text)
                .chars()
                .take(2000)
                .collect::<String>()
        } else {
            text.to_owned()
        };
        sqlx::query("INSERT INTO lane_events(lane_key, lead_session_key, kind, body, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(key).bind(lead).bind(target.as_str()).bind(body).bind(now).execute(&mut *tx).await?;
    }
    let guidance = if claim {
        let deadline = (now + chrono::Duration::minutes(5)).with_timezone(&chrono_tz::Asia::Seoul);
        format!(
            "[LEAD] claim received; stay idle; closes at {} KST unless objected",
            deadline.format("%H:%M")
        )
    } else if blocked_answer {
        text.to_owned()
    } else {
        format!("[LEAD] not done: {text}; keep working, claim again when fixed")
    };
    sqlx::query("INSERT INTO lane_inbox(lane_key, text) VALUES (?, ?)")
        .bind(key)
        .bind(guidance)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

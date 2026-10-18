#[path = "loop.rs"]
pub mod action_loop;
pub mod effect;
pub mod ledger;

use std::str::FromStr;

pub use effect::{
    classify_argv, classify_command, classify_file_change, classify_frame, classify_json,
    is_bare_eval_frame, requires_always_ask, Classification, Effect,
};
pub use ledger::{
    decide_proposal, evaluate_pre_ask, finalize_ask_result, find_matching_precedent, get_proposal,
    in_quiet_hours_kst, insert_proposal, list_active_precedents,
    list_pending_proposals_for_session, next_0800_kst, parse_short_owner_reply,
    record_once_and_check_candidate, recover_pending_proposals_on_startup, revoke_precedent,
    set_proposal_message_id, try_resolve_short_reply, update_proposal_decision, upsert_precedent,
    ActionPrecedent, ActionProposal, AskResolution, NewProposal, PreAskInput, PreAskOutcome,
    PrecedentDecision, PrecedentScope, ProposalOrigin, ProposalState,
};

#[derive(Debug, clap::Args)]
pub struct AutonomyArgs {
    #[command(subcommand)]
    pub command: AutonomyCommand,
}

#[derive(Debug, clap::Subcommand)]
pub enum AutonomyCommand {
    Classify {
        #[arg(long)]
        json: bool,
        frame: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Decide {
        proposal_id: i64,
        decision: String,
        #[arg(long, default_value = "exact")]
        scope: String,
        #[arg(long)]
        words: Option<String>,
    },
    Revoke {
        precedent_id: i64,
    },
}

pub fn is_owner_invocation_env() -> bool {
    if let Ok(origin) = std::env::var("OMON_AUTONOMY_CALLER_SESSION") {
        let trimmed = origin.trim();
        if trimmed.is_empty() {
            return true;
        }
        if trimmed.starts_with("cron:") || trimmed.starts_with("lane:") {
            return false;
        }
        let slack_owner = crate::slack::owner_id();
        let discord_owner = crate::inbox::proposals::OWNER_ID;
        return trimmed.contains(&slack_owner) || trimmed.contains(discord_owner);
    }
    true
}

async fn connect_cli_pool() -> Result<sqlx::SqlitePool, String> {
    let url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:omon_gateway.db".to_string());
    let db = crate::Database::connect(&url)
        .await
        .map_err(|e| e.to_string())?;
    Ok(db.pool().clone())
}

pub async fn run_cli(args: AutonomyArgs) -> i32 {
    match args.command {
        AutonomyCommand::Classify { json, frame } => {
            let classification = match serde_json::from_str::<serde_json::Value>(&frame) {
                Ok(val) => classify_json(&val),
                Err(_) => classify_command(&frame, None),
            };
            if json {
                match serde_json::to_string(&classification) {
                    Ok(out) => {
                        println!("{out}");
                        0
                    }
                    Err(err) => {
                        eprintln!("error: {err}");
                        1
                    }
                }
            } else {
                println!("{}", classification.effect.as_str());
                0
            }
        }
        AutonomyCommand::List { json } => {
            let pool = match connect_cli_pool().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            let rows = match list_active_precedents(&pool).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".into())
                );
            } else {
                for r in rows {
                    println!(
                        "#{} [{}] {} {} target={} sig={}",
                        r.id, r.scope, r.decision, r.effect, r.target_scope, r.signature
                    );
                }
            }
            0
        }
        AutonomyCommand::Decide {
            proposal_id,
            decision,
            scope,
            words,
        } => {
            if !is_owner_invocation_env() {
                eprintln!("error: only an owner-origin session or local CLI may decide proposals");
                return 2;
            }
            let parsed_scope = match PrecedentScope::from_str(&scope) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 2;
                }
            };
            let pool = match connect_cli_pool().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            match decide_proposal(
                &pool,
                None,
                proposal_id,
                &decision,
                parsed_scope,
                words.as_deref(),
                None,
            )
            .await
            {
                Ok(updated) => {
                    println!("{}", serde_json::to_string(&updated).unwrap_or_default());
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        AutonomyCommand::Revoke { precedent_id } => {
            if !is_owner_invocation_env() {
                eprintln!("error: only an owner-origin session or local CLI may revoke precedents");
                return 2;
            }
            let pool = match connect_cli_pool().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            match revoke_precedent(&pool, precedent_id).await {
                Ok(true) => {
                    println!("revoked precedent #{precedent_id}");
                    0
                }
                Ok(false) => {
                    eprintln!("error: precedent #{precedent_id} not found or already revoked");
                    1
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
    }
}

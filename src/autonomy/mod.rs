#[path = "loop.rs"]
pub mod action_loop;
pub mod effect;
pub mod ledger;

use std::str::FromStr;

pub use effect::{
    classify_argv, classify_command, classify_eval_code, classify_file_change, classify_frame,
    classify_json, is_bare_eval_frame, requires_always_ask, Classification, Effect,
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
    /// Record what happened to an action-loop item (`inbox:<id>`, `github:<id>`, ...).
    /// `executed` is the lead's own report of the action plus the reference of the result it
    /// produced; the item is not done yet. `verified` is the only "done", and it is not taken on
    /// trust: the gateway reads the event back itself with `<calendar cmd> cal get <event id>`
    /// (`OMON_ACTION_LOOP_CALENDAR_CMD`, default `zele`, 30s bound), requires the parsed `id` to
    /// match and the parsed `description` to carry the item key as a whole token. So put the item
    /// key in the event description when creating it. A result this gateway cannot read back
    /// (a pull request, a message) cannot be verified.
    Outcome {
        item_key: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        external_id: Option<String>,
        #[arg(long)]
        detail: Option<String>,
    },
    /// Adopt an item delivered **before** `action_loop_outcomes` existed: it has a seen mark but
    /// can never own an outcome row, so it would stay invisible to reconciliation forever.
    ///
    /// Not a backfill and never automatic. The item must already be a bot/owner `inbox_decisions`
    /// row with an `action_loop_seen` mark and no outcome row, and `--external-id` must pass the
    /// same exact provider readback as `verified`: the event must exist and carry this item key as
    /// its own token. The row is written as `executed` with `legacy-reconcile:` provenance — never
    /// `verified` — and the seen mark is kept, so the item can never be re-emitted.
    Adopt {
        item_key: String,
        #[arg(long)]
        external_id: String,
        #[arg(long)]
        detail: Option<String>,
    },
}

pub fn is_owner_invocation_env() -> bool {
    // Every command an engine turn runs carries PI_SESSION_ID: a model may never decide, revoke or
    // adopt on its own, whatever session it runs in. The owner decides in Slack, Discord, the
    // dashboard or a plain terminal.
    if std::env::var("PI_SESSION_ID").is_ok_and(|id| !id.trim().is_empty()) {
        return false;
    }
    if let Ok(origin) = std::env::var("OMON_AUTONOMY_CALLER_SESSION") {
        let trimmed = origin.trim();
        if trimmed.is_empty() {
            return false;
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
        AutonomyCommand::Outcome {
            item_key,
            status,
            external_id,
            detail,
        } => {
            let pool = match connect_cli_pool().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            match action_loop::record_outcome(
                &pool,
                &item_key,
                action_loop::OutcomeReport::new(&status)
                    .external_id(external_id.as_deref())
                    .detail(detail.as_deref()),
                chrono::Utc::now(),
            )
            .await
            {
                Ok(()) => {
                    println!("recorded {item_key} {status}");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        AutonomyCommand::Adopt {
            item_key,
            external_id,
            detail,
        } => {
            // Adoption is an explicit historical mapping of one already-delivered item, not
            // automated event processing, so it is an owner decision: an engine turn (any session
            // carrying PI_SESSION_ID), a cron job or a lane may never run it, whatever lead it
            // belongs to. The owner runs it from a terminal or an owner-origin session.
            if !is_owner_invocation_env() {
                eprintln!(
                    "error: only an owner-origin session or local CLI may adopt a historical action-loop item"
                );
                return 2;
            }
            let pool = match connect_cli_pool().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 1;
                }
            };
            match action_loop::adopt_legacy_item(
                &pool,
                &item_key,
                &external_id,
                detail.as_deref(),
                chrono::Utc::now(),
            )
            .await
            {
                Ok(()) => {
                    println!("adopted {item_key} executed external_id={external_id}");
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

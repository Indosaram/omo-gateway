use std::collections::BTreeMap;
use std::str::FromStr;

use chrono::{DateTime, NaiveTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use super::effect::{requires_always_ask, Classification, Effect};
use crate::{ApprovalDecision, ApprovalError, SessionKey, SmartApprovalGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalOrigin {
    EngineApproval,
    ToolApproval,
    Inbox,
    Github,
    Loop,
    Yolo,
}

impl ProposalOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EngineApproval => "engine_approval",
            Self::ToolApproval => "tool_approval",
            Self::Inbox => "inbox",
            Self::Github => "github",
            Self::Loop => "loop",
            Self::Yolo => "yolo",
        }
    }
}

impl FromStr for ProposalOrigin {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "engine_approval" => Ok(Self::EngineApproval),
            "tool_approval" => Ok(Self::ToolApproval),
            "inbox" => Ok(Self::Inbox),
            "github" => Ok(Self::Github),
            "loop" => Ok(Self::Loop),
            "yolo" => Ok(Self::Yolo),
            other => Err(format!("unknown proposal origin: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Pending,
    Approved,
    Declined,
    Expired,
    AutoAllowed,
    AutoDenied,
}

impl ProposalState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::Expired => "expired",
            Self::AutoAllowed => "auto_allowed",
            Self::AutoDenied => "auto_denied",
        }
    }
}

impl FromStr for ProposalState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "declined" => Ok(Self::Declined),
            "expired" => Ok(Self::Expired),
            "auto_allowed" => Ok(Self::AutoAllowed),
            "auto_denied" => Ok(Self::AutoDenied),
            other => Err(format!("unknown proposal state: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecedentDecision {
    Allow,
    Deny,
}

impl PrecedentDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

impl FromStr for PrecedentDecision {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            other => Err(format!("unknown precedent decision: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecedentScope {
    Exact,
    Class,
}

impl PrecedentScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Class => "class",
        }
    }
}

impl FromStr for PrecedentScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "exact" => Ok(Self::Exact),
            "class" => Ok(Self::Class),
            other => Err(format!("unknown precedent scope: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionProposal {
    pub id: i64,
    pub origin: String,
    pub session_key: String,
    pub signature: String,
    pub effect: String,
    pub target: String,
    pub summary: String,
    pub payload_json: String,
    pub tainted: i64,
    pub state: String,
    pub precedent_id: Option<i64>,
    pub owner_words: Option<String>,
    pub platform_message_id: Option<String>,
    pub created_at: String,
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionPrecedent {
    pub id: i64,
    pub signature: String,
    pub effect: String,
    pub target_scope: String,
    pub decision: String,
    pub scope: String,
    pub approvals: i64,
    pub declines: i64,
    pub origin: String,
    pub owner_words: Option<String>,
    pub revoked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct NewProposal {
    pub origin: ProposalOrigin,
    pub session_key: String,
    pub signature: String,
    pub effect: Effect,
    pub target: String,
    pub summary: String,
    pub payload_json: String,
    pub tainted: bool,
    pub state: ProposalState,
    pub precedent_id: Option<i64>,
    pub owner_words: Option<String>,
    pub platform_message_id: Option<String>,
}

pub fn in_quiet_hours_kst(now: DateTime<Utc>) -> bool {
    now.with_timezone(&chrono_tz::Asia::Seoul).hour() < 8
}

pub fn next_0800_kst(now: DateTime<Utc>) -> DateTime<Utc> {
    let kst = now.with_timezone(&chrono_tz::Asia::Seoul);
    let eight = NaiveTime::from_hms_opt(8, 0, 0).expect("valid 08:00");
    let date = if kst.hour() < 8 {
        kst.date_naive()
    } else {
        kst.date_naive() + chrono::Duration::days(1)
    };
    chrono_tz::Asia::Seoul
        .from_local_datetime(&date.and_time(eight))
        .single()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now)
}

pub fn parse_short_owner_reply(text: &str) -> Option<ApprovalDecision> {
    let trimmed = text.trim();
    match trimmed {
        "응" | "ㅇㅇ" | "해" | "허용" | "좋아" => Some(ApprovalDecision::Once),
        "앞으로도" | "항상" => Some(ApprovalDecision::Always),
        "아니" | "하지마" | "거절" | "ㄴㄴ" => Some(ApprovalDecision::Deny {
            reason: Some(trimmed.to_string()),
        }),
        _ => None,
    }
}

pub async fn try_resolve_short_reply(
    pool: Option<&SqlitePool>,
    guard: Option<&SmartApprovalGuard>,
    session: &SessionKey,
    text: &str,
) -> Result<bool, sqlx::Error> {
    let Some(decision) = parse_short_owner_reply(text) else {
        return Ok(false);
    };
    let guard_pending = guard
        .map(|g| g.pending_for_session(session))
        .unwrap_or_default();
    // An engine or tool approval is only answerable while its turn still waits on the guard;
    // a row without a live waiter belongs to a turn that is gone, so a short reply is ordinary
    // conversation, not an approval of an action nothing will run.
    let db_pending: Vec<ActionProposal> = match pool {
        Some(p) => list_pending_proposals_for_session(p, &session.storage_key())
            .await?
            .into_iter()
            .filter(|row| !matches!(row.origin.as_str(), "engine_approval" | "tool_approval"))
            .collect(),
        None => Vec::new(),
    };
    if guard_pending.len() > 1 || db_pending.len() > 1 {
        return Ok(false);
    }
    if guard_pending.len() == 1 {
        let resolved = guard
            .expect("guard_pending non-empty")
            .resolve_single_session_decision(session, decision);
        return Ok(resolved);
    }
    if db_pending.len() == 1 {
        let pool = pool.expect("db_pending non-empty");
        let proposal = &db_pending[0];
        let kind = match &decision {
            ApprovalDecision::Once | ApprovalDecision::Session => "allow",
            ApprovalDecision::Always => "allow-always",
            ApprovalDecision::Deny { .. } => "deny",
        };
        let words = match &decision {
            ApprovalDecision::Deny { reason } => reason.as_deref(),
            _ => None,
        };
        if let Err(error) = decide_proposal(
            pool,
            guard,
            proposal.id,
            kind,
            PrecedentScope::Exact,
            words,
            Some(&proposal.session_key),
        )
        .await
        {
            // Not recorded: let the message reach the conversation instead of eating it.
            tracing::warn!(proposal = proposal.id, %error, "short owner reply could not decide the proposal");
            return Ok(false);
        }
        return Ok(true);
    }
    Ok(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreAskOutcome {
    Allow {
        proposal_id: Option<i64>,
        precedent_id: Option<i64>,
        notice: Option<String>,
    },
    Deny {
        proposal_id: i64,
        precedent_id: Option<i64>,
        reason: String,
        count_toward_limit: bool,
    },
    Ask {
        proposal_id: i64,
    },
}

#[derive(Debug, Clone)]
pub struct PreAskInput<'a> {
    pub origin: ProposalOrigin,
    pub session_key: &'a str,
    pub lead_session_key: &'a str,
    pub classification: &'a Classification,
    pub payload_json: &'a str,
    pub tainted: bool,
    pub yolo: bool,
    pub always_mode: bool,
    pub now: DateTime<Utc>,
}

pub async fn evaluate_pre_ask(
    pool: &SqlitePool,
    input: PreAskInput<'_>,
) -> Result<PreAskOutcome, sqlx::Error> {
    let cls = input.classification;
    // Payments always ask, even in yolo.
    if input.yolo && cls.effect != Effect::Spend {
        let id = insert_proposal(
            pool,
            &NewProposal {
                origin: ProposalOrigin::Yolo,
                session_key: input.session_key.to_string(),
                signature: cls.signature.clone(),
                effect: cls.effect,
                target: cls.target.clone(),
                summary: cls.summary.clone(),
                payload_json: input.payload_json.to_string(),
                tainted: input.tainted,
                state: ProposalState::Approved,
                precedent_id: None,
                owner_words: None,
                platform_message_id: None,
            },
        )
        .await?;
        return Ok(PreAskOutcome::Allow {
            proposal_id: Some(id),
            precedent_id: None,
            notice: None,
        });
    }

    if !input.always_mode && cls.effect == Effect::Read {
        return Ok(PreAskOutcome::Allow {
            proposal_id: None,
            precedent_id: None,
            notice: None,
        });
    }

    if !input.always_mode {
        if let Some(prec) =
            find_matching_precedent(pool, &cls.signature, cls.effect, &cls.target, input.tainted)
                .await?
        {
            if prec.decision == "allow" {
                let id = insert_proposal(
                    pool,
                    &NewProposal {
                        origin: input.origin,
                        session_key: input.session_key.to_string(),
                        signature: cls.signature.clone(),
                        effect: cls.effect,
                        target: cls.target.clone(),
                        summary: cls.summary.clone(),
                        payload_json: input.payload_json.to_string(),
                        tainted: input.tainted,
                        state: ProposalState::AutoAllowed,
                        precedent_id: Some(prec.id),
                        owner_words: None,
                        platform_message_id: None,
                    },
                )
                .await?;
                let notice = format!("선례 #{}로 자동 진행: {}", prec.id, cls.summary);
                return Ok(PreAskOutcome::Allow {
                    proposal_id: Some(id),
                    precedent_id: Some(prec.id),
                    notice: Some(notice),
                });
            } else {
                let words = prec
                    .owner_words
                    .clone()
                    .unwrap_or_else(|| "declined".to_string());
                let id = insert_proposal(
                    pool,
                    &NewProposal {
                        origin: input.origin,
                        session_key: input.session_key.to_string(),
                        signature: cls.signature.clone(),
                        effect: cls.effect,
                        target: cls.target.clone(),
                        summary: cls.summary.clone(),
                        payload_json: input.payload_json.to_string(),
                        tainted: input.tainted,
                        state: ProposalState::AutoDenied,
                        precedent_id: Some(prec.id),
                        owner_words: Some(words.clone()),
                        platform_message_id: None,
                    },
                )
                .await?;
                return Ok(PreAskOutcome::Deny {
                    proposal_id: id,
                    precedent_id: Some(prec.id),
                    reason: format!("owner declined this before (#{}): {}", prec.id, words),
                    count_toward_limit: false,
                });
            }
        }
    }

    let proposal_id = insert_proposal(
        pool,
        &NewProposal {
            origin: input.origin,
            session_key: input.session_key.to_string(),
            signature: cls.signature.clone(),
            effect: cls.effect,
            target: cls.target.clone(),
            summary: cls.summary.clone(),
            payload_json: input.payload_json.to_string(),
            tainted: input.tainted,
            state: ProposalState::Pending,
            precedent_id: None,
            owner_words: None,
            platform_message_id: None,
        },
    )
    .await?;

    if in_quiet_hours_kst(input.now)
        && !owner_recently_active(
            pool,
            &[input.lead_session_key, input.session_key],
            input.now,
        )
        .await?
    {
        let release_at = next_0800_kst(input.now);
        let body = format!(
            "[ACTION_LOOP] quiet-hour proposal #{proposal_id} ({} {}): {}",
            cls.effect.as_str(),
            cls.target,
            cls.summary
        );
        sqlx::query(
            "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at, release_at)
             VALUES (NULL, ?, 'action_loop', ?, ?, ?)",
        )
        .bind(input.lead_session_key)
        .bind(&body)
        .bind(input.now)
        .bind(release_at)
        .execute(pool)
        .await?;
        // The action did not run. The 08:00 action_loop event asks the lead to re-issue it,
        // which creates a fresh proposal; this row must never be approved later by a short
        // reply or count toward a precedent candidate.
        update_proposal_decision(pool, proposal_id, ProposalState::Expired, None, None).await?;

        return Ok(PreAskOutcome::Deny {
            proposal_id,
            precedent_id: None,
            reason: format!("quiet hours; proposal #{proposal_id} will be asked at 08:00"),
            count_toward_limit: false,
        });
    }

    Ok(PreAskOutcome::Ask { proposal_id })
}

/// Window in which an owner message counts as "the owner is awake and driving this session".
const OWNER_ACTIVE_WINDOW_MINUTES: i64 = 10;

/// True when the owner sent a real (non-system) message in any of `keys` within the last
/// 10 minutes. Quiet hours exist to avoid waking the owner, not to refuse requests the
/// owner is making live.
pub async fn owner_recently_active(
    pool: &SqlitePool,
    keys: &[&str],
    now: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let cutoff = (now - chrono::Duration::minutes(OWNER_ACTIVE_WINDOW_MINUTES)).to_rfc3339();
    for key in keys {
        if key.is_empty() {
            continue;
        }
        // created_at is stored either as RFC3339 (bound DateTime<Utc>) or as SQLite
        // CURRENT_TIMESTAMP text; julianday() normalizes both.
        let hit: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM messages
             WHERE session_key = ? AND role = 'user' AND content NOT LIKE '[system]%'
               AND julianday(created_at) >= julianday(?)
             LIMIT 1",
        )
        .bind(key)
        .bind(&cutoff)
        .fetch_optional(pool)
        .await?;
        if hit.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskResolution {
    Allow,
    Deny {
        reason: String,
        count_toward_limit: bool,
    },
}

pub async fn finalize_ask_result(
    pool: &SqlitePool,
    proposal_id: i64,
    classification: &Classification,
    origin: &str,
    lead_session_key: &str,
    result: Result<ApprovalDecision, ApprovalError>,
) -> Result<AskResolution, sqlx::Error> {
    // The claim is the first statement of the transaction, so it takes the write lock before any
    // read: a second answer waits for this one to commit and then claims nothing. Only the winner
    // writes the precedent, so the counters move once.
    let mut tx = pool.begin().await?;
    let claim_state = match &result {
        Ok(ApprovalDecision::Deny { .. }) => ProposalState::Declined,
        Err(ApprovalError::Timeout | ApprovalError::Cancelled | ApprovalError::Persistence(_)) => {
            ProposalState::Expired
        }
        Ok(ApprovalDecision::Once | ApprovalDecision::Session | ApprovalDecision::Always) => {
            ProposalState::Approved
        }
    };
    let claimed = claim_proposal(&mut tx, proposal_id, claim_state).await?;
    let Some(row) = proposal_row(&mut tx, proposal_id).await? else {
        // Nothing to decide: never leave a precedent behind for a proposal that does not exist.
        tx.rollback().await?;
        return Ok(AskResolution::Deny {
            reason: format!("proposal #{proposal_id} no longer exists; the action was not run"),
            count_toward_limit: false,
        });
    };
    if !claimed {
        // `decide_proposal` (CLI or short reply) or a concurrent answer already recorded the
        // owner's decision and the precedent; recording it again would bump the counters twice.
        tx.rollback().await?;
        return Ok(
            settled_resolution(&row).unwrap_or_else(|| AskResolution::Deny {
                reason: pending_ask_reason(proposal_id),
                count_toward_limit: false,
            }),
        );
    }
    let resolution = match result {
        Ok(ApprovalDecision::Once | ApprovalDecision::Session) => {
            count_once_candidate(
                &mut tx,
                proposal_id,
                classification.effect,
                &classification.target,
                lead_session_key,
            )
            .await?;
            AskResolution::Allow
        }
        Ok(ApprovalDecision::Always) => {
            let prec_id = write_precedent(
                &mut tx,
                &PrecedentWrite::of_classification(
                    classification,
                    PrecedentDecision::Allow,
                    PrecedentScope::Exact,
                    origin,
                    None,
                ),
            )
            .await?;
            finish_decided(&mut tx, proposal_id, prec_id, None).await?;
            AskResolution::Allow
        }
        Ok(ApprovalDecision::Deny { reason }) => {
            let words = reason.as_deref().unwrap_or("declined by owner");
            let prec_id = write_precedent(
                &mut tx,
                &PrecedentWrite::of_classification(
                    classification,
                    PrecedentDecision::Deny,
                    PrecedentScope::Exact,
                    origin,
                    Some(words),
                ),
            )
            .await?;
            finish_decided(&mut tx, proposal_id, prec_id, Some(words)).await?;
            AskResolution::Deny {
                reason: words.to_string(),
                count_toward_limit: true,
            }
        }
        Err(ApprovalError::Timeout | ApprovalError::Cancelled | ApprovalError::Persistence(_)) => {
            AskResolution::Deny {
                reason: pending_ask_reason(proposal_id),
                count_toward_limit: false,
            }
        }
    };
    tx.commit().await?;
    Ok(resolution)
}

/// The resolution a proposal already recorded, or `None` while it is still open.
fn settled_resolution(row: &ActionProposal) -> Option<AskResolution> {
    match row.state.as_str() {
        "approved" | "auto_allowed" => Some(AskResolution::Allow),
        "declined" | "auto_denied" => Some(AskResolution::Deny {
            reason: row
                .owner_words
                .clone()
                .unwrap_or_else(|| "declined by owner".to_string()),
            count_toward_limit: true,
        }),
        _ => None,
    }
}

fn pending_ask_reason(proposal_id: i64) -> String {
    format!(
        "owner has not answered yet; continue with other work and mention pending proposal #{proposal_id} in your report"
    )
}

/// The proposal row, read inside the caller's transaction.
async fn proposal_row(
    conn: &mut SqliteConnection,
    id: i64,
) -> Result<Option<ActionProposal>, sqlx::Error> {
    sqlx::query_as::<_, ActionProposal>("SELECT * FROM action_proposals WHERE id = ?")
        .bind(id)
        .fetch_optional(conn)
        .await
}

/// Records the decided state and, for a precedent-writing decision, the precedent it created.
async fn set_decided(
    conn: &mut SqliteConnection,
    id: i64,
    state: ProposalState,
    precedent_id: Option<i64>,
    owner_words: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE action_proposals
         SET state = ?, precedent_id = COALESCE(?, precedent_id), owner_words = COALESCE(?, owner_words), decided_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(state.as_str())
    .bind(precedent_id)
    .bind(owner_words)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Claims an open proposal for one decision. True only for the answer that won the claim.
async fn claim_proposal(
    conn: &mut SqliteConnection,
    id: i64,
    state: ProposalState,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE action_proposals
         SET state = ?, decided_at = CURRENT_TIMESTAMP
         WHERE id = ? AND state = 'pending'",
    )
    .bind(state.as_str())
    .bind(id)
    .execute(conn)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// Attaches the precedent and the owner's words to a proposal already claimed by this decision.
async fn finish_decided(
    conn: &mut SqliteConnection,
    id: i64,
    precedent_id: Option<i64>,
    owner_words: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE action_proposals
         SET precedent_id = COALESCE(?, precedent_id), owner_words = COALESCE(?, owner_words)
         WHERE id = ?",
    )
    .bind(precedent_id)
    .bind(owner_words)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Records an `once` approval and emits the third-owner-once candidate. Claims the proposal first,
/// so a decision that was already recorded is never counted a second time.
pub async fn record_once_and_check_candidate(
    pool: &SqlitePool,
    proposal_id: i64,
    effect: Effect,
    target: &str,
    lead_session_key: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !claim_proposal(&mut tx, proposal_id, ProposalState::Approved).await? {
        tx.rollback().await?;
        return Ok(false);
    }
    let emitted =
        count_once_candidate(&mut tx, proposal_id, effect, target, lead_session_key).await?;
    tx.commit().await?;
    Ok(emitted)
}

async fn count_once_candidate(
    conn: &mut SqliteConnection,
    proposal_id: i64,
    effect: Effect,
    target: &str,
    lead_session_key: &str,
) -> Result<bool, sqlx::Error> {
    if !effect.allows_class_precedent() {
        return Ok(false);
    }
    // Count only owner-decided once approvals: yolo rows are `approved` with NULL precedent
    // too, but no owner answered them.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM action_proposals
         WHERE state = 'approved' AND effect = ? AND target = ? AND precedent_id IS NULL
           AND origin != 'yolo'",
    )
    .bind(effect.as_str())
    .bind(target)
    .fetch_one(&mut *conn)
    .await?;

    if count < 3 {
        return Ok(false);
    }
    let already_emitted: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM lane_events
            WHERE kind = 'precedent_candidate' AND body LIKE ?
        )",
    )
    .bind(format!(
        "precedent_candidate effect={} target={} proposal_id=%",
        effect.as_str(),
        target
    ))
    .fetch_one(&mut *conn)
    .await?;
    if already_emitted {
        return Ok(false);
    }

    let has_deny: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM action_precedents
            WHERE decision = 'deny' AND revoked_at IS NULL
              AND ((effect = ? AND target_scope = ?) OR signature LIKE ?)
        )",
    )
    .bind(effect.as_str())
    .bind(target)
    .bind(format!("{}:%:{target}", effect.as_str()))
    .fetch_one(&mut *conn)
    .await?;
    if has_deny {
        return Ok(false);
    }

    let sigs: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT signature FROM action_proposals
         WHERE state = 'approved' AND effect = ? AND target = ? AND origin != 'yolo'
         ORDER BY id ASC",
    )
    .bind(effect.as_str())
    .bind(target)
    .fetch_all(&mut *conn)
    .await?;
    let sig_list: Vec<String> = sigs.into_iter().map(|(s,)| s).collect();
    let body = format!(
        "precedent_candidate effect={} target={} proposal_id={} signatures=[{}]",
        effect.as_str(),
        target,
        proposal_id,
        sig_list.join(", ")
    );
    sqlx::query(
        "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at)
         VALUES (NULL, ?, 'precedent_candidate', ?, CURRENT_TIMESTAMP)",
    )
    .bind(lead_session_key)
    .bind(&body)
    .execute(&mut *conn)
    .await?;
    Ok(true)
}

pub async fn recover_pending_proposals_on_startup(
    pool: &SqlitePool,
    default_lead: &str,
) -> Result<usize, sqlx::Error> {
    // Inbox/github/loop rows are durable drafts owned by their own flows; only engine and
    // tool approval frames die with the process.
    let pending: Vec<ActionProposal> = sqlx::query_as(
        "SELECT * FROM action_proposals
         WHERE state = 'pending' AND origin IN ('engine_approval','tool_approval')
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    if pending.is_empty() {
        return Ok(0);
    }
    // Every engine and tool approval dies with the process: the turn that waited on it is gone,
    // so approving it later would record a decision for an action nothing runs. (Its card, if
    // pressed, is answered as stale.)
    let expired = &pending;
    for p in expired {
        sqlx::query(
            "UPDATE action_proposals
             SET state = 'expired', decided_at = CURRENT_TIMESTAMP
             WHERE id = ? AND state = 'pending'",
        )
        .bind(p.id)
        .execute(pool)
        .await?;
    }

    let mut expired_by_lead: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in expired {
        let lead = proposal_lead(pool, p, default_lead).await?;
        expired_by_lead
            .entry(lead)
            .or_default()
            .push(format!("#{} ({} {})", p.id, p.effect, p.summary));
    }

    for (lead, items) in expired_by_lead {
        let body = format!(
            "[ACTION_LOOP] {} expired pending proposal(s) on restart: {}",
            items.len(),
            items.join("; ")
        );
        insert_action_loop_event(pool, &lead, &body).await?;
    }
    Ok(expired.len())
}

async fn insert_action_loop_event(
    pool: &SqlitePool,
    lead: &str,
    body: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at)
         VALUES (NULL, ?, 'action_loop', ?, CURRENT_TIMESTAMP)",
    )
    .bind(lead)
    .bind(body)
    .execute(pool)
    .await?;
    Ok(())
}

async fn proposal_lead(
    pool: &SqlitePool,
    p: &ActionProposal,
    default_lead: &str,
) -> Result<String, sqlx::Error> {
    Ok(if p.session_key.starts_with("lane:") {
        let lane_key = p.session_key.trim_start_matches("lane:");
        sqlx::query_scalar::<_, String>("SELECT lead_session_key FROM lanes WHERE key = ?")
            .bind(lane_key)
            .fetch_optional(pool)
            .await?
            .unwrap_or_else(|| default_lead.to_string())
    } else if SessionKey::from_storage_key(&p.session_key).is_ok() {
        p.session_key.clone()
    } else {
        default_lead.to_string()
    })
}

pub async fn decide_proposal(
    pool: &SqlitePool,
    guard: Option<&SmartApprovalGuard>,
    proposal_id: i64,
    decision_kind: &str,
    scope: PrecedentScope,
    owner_words: Option<&str>,
    lead_session_key: Option<&str>,
) -> Result<ActionProposal, String> {
    let kind = decision_kind.trim().to_ascii_lowercase();
    let claim_state = match kind.as_str() {
        "allow" | "once" | "allow-always" | "always" => ProposalState::Approved,
        "deny" | "decline" => ProposalState::Declined,
        other => {
            return Err(format!(
                "invalid decision `{other}` (expected allow, deny, or allow-always)"
            ))
        }
    };
    // Claim first, then read: the claim takes the write lock so a card press that lands at the
    // same moment waits here and then claims nothing, instead of bumping the same counters again.
    // Only an exact-scope decision is claimed: it is the one a card press and a short reply can
    // both issue for the same proposal. A class-scope decision is an explicit owner instruction
    // that may also be recorded after the proposal was already answered.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let claimed = if scope == PrecedentScope::Exact {
        claim_proposal(&mut tx, proposal_id, claim_state)
            .await
            .map_err(|e| e.to_string())?
    } else {
        set_decided(&mut tx, proposal_id, claim_state, None, None)
            .await
            .map_err(|e| e.to_string())?;
        true
    };
    let Some(proposal) = proposal_row(&mut tx, proposal_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err(format!("proposal #{proposal_id} not found"));
    };
    let effect = Effect::from_str(&proposal.effect)?;
    let fallback_lead = Some(crate::runtime_config::default_lead());
    let lead = [lead_session_key, Some(proposal.session_key.as_str())]
        .into_iter()
        .flatten()
        .find(|key| SessionKey::from_storage_key(key).is_ok())
        .or(fallback_lead.as_deref())
        .unwrap_or(&proposal.session_key);
    let sig_for_scope = match scope {
        PrecedentScope::Exact => proposal.signature.clone(),
        PrecedentScope::Class => format!("{}:class:{}", effect.as_str(), proposal.target),
    };

    if matches!(kind.as_str(), "allow-always" | "always") && !effect.allows_any_precedent() {
        return Err("spend never gets a precedent".to_string());
    }
    if scope == PrecedentScope::Class && !effect.allows_class_precedent() {
        return Err(format!(
            "effect `{}` does not allow class precedents",
            effect.as_str()
        ));
    }

    if !claimed {
        // Another answer already decided this proposal; record nothing and report the row.
        tx.rollback().await.map_err(|e| e.to_string())?;
        return Ok(proposal);
    }

    match kind.as_str() {
        "allow" | "once" => {
            if scope == PrecedentScope::Class {
                let prec_id = write_precedent(
                    &mut tx,
                    &PrecedentWrite {
                        signature: &sig_for_scope,
                        effect,
                        target_scope: &proposal.target,
                        decision: PrecedentDecision::Allow,
                        scope: PrecedentScope::Class,
                        origin: "owner_decide",
                        owner_words,
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
                finish_decided(&mut tx, proposal_id, prec_id, owner_words)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                count_once_candidate(&mut tx, proposal_id, effect, &proposal.target, lead)
                    .await
                    .map_err(|e| e.to_string())?;
                if owner_words.is_some() {
                    finish_decided(&mut tx, proposal_id, None, owner_words)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "allow-always" | "always" => {
            let prec_id = write_precedent(
                &mut tx,
                &PrecedentWrite {
                    signature: &sig_for_scope,
                    effect,
                    target_scope: &proposal.target,
                    decision: PrecedentDecision::Allow,
                    scope,
                    origin: "owner_decide",
                    owner_words,
                },
            )
            .await
            .map_err(|e| e.to_string())?;
            finish_decided(&mut tx, proposal_id, prec_id, owner_words)
                .await
                .map_err(|e| e.to_string())?;
        }
        "deny" | "decline" => {
            let prec_id = write_precedent(
                &mut tx,
                &PrecedentWrite {
                    signature: &sig_for_scope,
                    effect,
                    target_scope: &proposal.target,
                    decision: PrecedentDecision::Deny,
                    scope,
                    origin: "owner_decide",
                    owner_words,
                },
            )
            .await
            .map_err(|e| e.to_string())?;
            finish_decided(&mut tx, proposal_id, prec_id, owner_words)
                .await
                .map_err(|e| e.to_string())?;
        }
        other => {
            return Err(format!(
                "invalid decision `{other}` (expected allow, deny, or allow-always)"
            ));
        }
    }
    tx.commit().await.map_err(|e| e.to_string())?;

    // Wake a live waiter only after the decision is committed, so it reads the recorded state.
    if let Some(g) = guard {
        let decision = match kind.as_str() {
            "allow" | "once" => ApprovalDecision::Once,
            "allow-always" | "always" => ApprovalDecision::Always,
            _ => ApprovalDecision::Deny {
                reason: owner_words.map(str::to_string),
            },
        };
        let _ = g.resolve_proposal_id(proposal_id, decision);
    }

    get_proposal(pool, proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("proposal #{proposal_id} vanished"))
}

pub async fn insert_proposal(
    pool: &SqlitePool,
    proposal: &NewProposal,
) -> Result<i64, sqlx::Error> {
    let decided = proposal.state != ProposalState::Pending;
    let res = if decided {
        sqlx::query(
            "INSERT INTO action_proposals
                (origin, session_key, signature, effect, target, summary, payload_json, tainted, state, precedent_id, owner_words, platform_message_id, decided_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(proposal.origin.as_str())
        .bind(&proposal.session_key)
        .bind(&proposal.signature)
        .bind(proposal.effect.as_str())
        .bind(&proposal.target)
        .bind(&proposal.summary)
        .bind(&proposal.payload_json)
        .bind(i64::from(proposal.tainted))
        .bind(proposal.state.as_str())
        .bind(proposal.precedent_id)
        .bind(&proposal.owner_words)
        .bind(&proposal.platform_message_id)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            "INSERT INTO action_proposals
                (origin, session_key, signature, effect, target, summary, payload_json, tainted, state, precedent_id, owner_words, platform_message_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(proposal.origin.as_str())
        .bind(&proposal.session_key)
        .bind(&proposal.signature)
        .bind(proposal.effect.as_str())
        .bind(&proposal.target)
        .bind(&proposal.summary)
        .bind(&proposal.payload_json)
        .bind(i64::from(proposal.tainted))
        .bind(proposal.state.as_str())
        .bind(proposal.precedent_id)
        .bind(&proposal.owner_words)
        .bind(&proposal.platform_message_id)
        .execute(pool)
        .await?
    };
    Ok(res.last_insert_rowid())
}

pub async fn get_proposal(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<ActionProposal>, sqlx::Error> {
    sqlx::query_as::<_, ActionProposal>("SELECT * FROM action_proposals WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_pending_proposals_for_session(
    pool: &SqlitePool,
    session_key: &str,
) -> Result<Vec<ActionProposal>, sqlx::Error> {
    sqlx::query_as::<_, ActionProposal>(
        "SELECT * FROM action_proposals WHERE state = 'pending' AND session_key = ? ORDER BY id ASC",
    )
    .bind(session_key)
    .fetch_all(pool)
    .await
}

pub async fn update_proposal_decision(
    pool: &SqlitePool,
    id: i64,
    state: ProposalState,
    precedent_id: Option<i64>,
    owner_words: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE action_proposals
         SET state = ?, precedent_id = COALESCE(?, precedent_id), owner_words = COALESCE(?, owner_words), decided_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(state.as_str())
    .bind(precedent_id)
    .bind(owner_words)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_proposal_message_id(
    pool: &SqlitePool,
    id: i64,
    platform_message_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE action_proposals SET platform_message_id = ? WHERE id = ?")
        .bind(platform_message_id)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_precedent(
    pool: &SqlitePool,
    signature: &str,
    effect: Effect,
    target_scope: &str,
    decision: PrecedentDecision,
    scope: PrecedentScope,
    origin: &str,
    owner_words: Option<&str>,
) -> Result<Option<i64>, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    write_precedent(
        &mut conn,
        &PrecedentWrite {
            signature,
            effect,
            target_scope,
            decision,
            scope,
            origin,
            owner_words,
        },
    )
    .await
}

/// One precedent write: what to record, for whom, and the owner's words.
struct PrecedentWrite<'a> {
    signature: &'a str,
    effect: Effect,
    target_scope: &'a str,
    decision: PrecedentDecision,
    scope: PrecedentScope,
    origin: &'a str,
    owner_words: Option<&'a str>,
}

impl<'a> PrecedentWrite<'a> {
    fn of_classification(
        classification: &'a Classification,
        decision: PrecedentDecision,
        scope: PrecedentScope,
        origin: &'a str,
        owner_words: Option<&'a str>,
    ) -> Self {
        Self {
            signature: &classification.signature,
            effect: classification.effect,
            target_scope: &classification.target,
            decision,
            scope,
            origin,
            owner_words,
        }
    }
}

/// The precedent write shared by the pooled and transactional callers.
async fn write_precedent(
    conn: &mut SqliteConnection,
    spec: &PrecedentWrite<'_>,
) -> Result<Option<i64>, sqlx::Error> {
    let &PrecedentWrite {
        signature,
        effect,
        target_scope,
        decision,
        scope,
        origin,
        owner_words,
    } = spec;
    if !effect.allows_any_precedent() {
        return Ok(None);
    }
    if scope == PrecedentScope::Class && !effect.allows_class_precedent() {
        return Ok(None);
    }
    let (add_app, add_dec) = match decision {
        PrecedentDecision::Allow => (1_i64, 0_i64),
        PrecedentDecision::Deny => (0_i64, 1_i64),
    };
    sqlx::query(
        "INSERT INTO action_precedents
            (signature, effect, target_scope, decision, scope, approvals, declines, origin, owner_words, revoked_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
         ON CONFLICT(signature, scope) DO UPDATE SET
            effect = excluded.effect,
            target_scope = excluded.target_scope,
            decision = excluded.decision,
            approvals = action_precedents.approvals + excluded.approvals,
            declines = action_precedents.declines + excluded.declines,
            origin = excluded.origin,
            owner_words = COALESCE(excluded.owner_words, action_precedents.owner_words),
            revoked_at = NULL,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(signature)
    .bind(effect.as_str())
    .bind(target_scope)
    .bind(decision.as_str())
    .bind(scope.as_str())
    .bind(add_app)
    .bind(add_dec)
    .bind(origin)
    .bind(owner_words)
    .execute(&mut *conn)
    .await?;

    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM action_precedents WHERE signature = ? AND scope = ?")
            .bind(signature)
            .bind(scope.as_str())
            .fetch_optional(&mut *conn)
            .await?;
    Ok(row.map(|(id,)| id))
}

pub async fn find_matching_precedent(
    pool: &SqlitePool,
    signature: &str,
    effect: Effect,
    target_scope: &str,
    tainted: bool,
) -> Result<Option<ActionPrecedent>, sqlx::Error> {
    if effect == Effect::Spend {
        return Ok(None);
    }
    if tainted
        && matches!(
            effect,
            Effect::Delete | Effect::Spend | Effect::VcsProtected
        )
    {
        return Ok(None);
    }

    let exact = sqlx::query_as::<_, ActionPrecedent>(
        "SELECT * FROM action_precedents
         WHERE signature = ? AND scope = 'exact' AND revoked_at IS NULL
         ORDER BY id DESC LIMIT 1",
    )
    .bind(signature)
    .fetch_optional(pool)
    .await?;
    if exact.is_some() {
        return Ok(exact);
    }

    if requires_always_ask(effect, tainted) || !effect.allows_class_precedent() {
        return Ok(None);
    }

    sqlx::query_as::<_, ActionPrecedent>(
        "SELECT * FROM action_precedents
         WHERE scope = 'class' AND effect = ? AND target_scope = ? AND revoked_at IS NULL
         ORDER BY id DESC LIMIT 1",
    )
    .bind(effect.as_str())
    .bind(target_scope)
    .fetch_optional(pool)
    .await
}

pub async fn list_active_precedents(
    pool: &SqlitePool,
) -> Result<Vec<ActionPrecedent>, sqlx::Error> {
    sqlx::query_as::<_, ActionPrecedent>(
        "SELECT * FROM action_precedents WHERE revoked_at IS NULL ORDER BY updated_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn revoke_precedent(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE action_precedents SET revoked_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pending_push(pool: &SqlitePool, target: &str) -> (i64, Classification) {
        let cls = Classification::new(
            Effect::VcsRemote,
            "git push",
            target,
            "git push origin lane/1",
        );
        let id = insert_proposal(
            pool,
            &NewProposal {
                origin: ProposalOrigin::EngineApproval,
                session_key: "slack:T1:C1::U1".into(),
                signature: cls.signature.clone(),
                effect: cls.effect,
                target: cls.target.clone(),
                summary: cls.summary.clone(),
                payload_json: "{}".into(),
                tainted: false,
                state: ProposalState::Pending,
                precedent_id: None,
                owner_words: None,
                platform_message_id: None,
            },
        )
        .await
        .unwrap();
        (id, cls)
    }

    #[tokio::test]
    async fn one_owner_decision_via_decide_then_waiter_counts_once() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let lead = "slack:T1:C1::U1";

        let (allow_id, allow_cls) = pending_push(&pool, "/repo-allow").await;
        decide_proposal(
            &pool,
            None,
            allow_id,
            "allow-always",
            PrecedentScope::Exact,
            None,
            Some(lead),
        )
        .await
        .unwrap();
        // The live engine waiter wakes with the same decision and finalizes.
        let res = finalize_ask_result(
            &pool,
            allow_id,
            &allow_cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Always),
        )
        .await
        .unwrap();
        assert_eq!(res, AskResolution::Allow);
        let approvals: i64 = sqlx::query_scalar(
            "SELECT approvals FROM action_precedents WHERE signature = ? AND scope = 'exact'",
        )
        .bind(&allow_cls.signature)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(approvals, 1, "one decision must bump approvals once");

        let (deny_id, deny_cls) = pending_push(&pool, "/repo-deny").await;
        decide_proposal(
            &pool,
            None,
            deny_id,
            "deny",
            PrecedentScope::Exact,
            Some("직접 올릴게"),
            Some(lead),
        )
        .await
        .unwrap();
        let res = finalize_ask_result(
            &pool,
            deny_id,
            &deny_cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Deny { reason: None }),
        )
        .await
        .unwrap();
        assert_eq!(
            res,
            AskResolution::Deny {
                reason: "직접 올릴게".into(),
                count_toward_limit: true
            }
        );
        let (declines, words): (i64, Option<String>) = sqlx::query_as(
            "SELECT declines, owner_words FROM action_precedents WHERE signature = ? AND scope = 'exact'",
        )
        .bind(&deny_cls.signature)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(declines, 1, "one decision must bump declines once");
        assert_eq!(words.as_deref(), Some("직접 올릴게"));
    }

    #[tokio::test]
    async fn waiter_then_short_reply_also_counts_once() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let lead = "slack:T1:C1::U1";

        let (id, cls) = pending_push(&pool, "/repo-waiter-first").await;
        let res = finalize_ask_result(
            &pool,
            id,
            &cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Always),
        )
        .await
        .unwrap();
        assert_eq!(res, AskResolution::Allow);
        // The owner's short reply arrives after the card was already answered.
        let row = decide_proposal(
            &pool,
            None,
            id,
            "allow-always",
            PrecedentScope::Exact,
            None,
            Some(lead),
        )
        .await
        .unwrap();
        assert_eq!(row.state, "approved");

        let approvals: i64 = sqlx::query_scalar(
            "SELECT approvals FROM action_precedents WHERE signature = ? AND scope = 'exact'",
        )
        .bind(&cls.signature)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            approvals, 1,
            "the reply after the card press must not bump approvals again"
        );
    }

    /// Two pools on one database file stand in for the gateway and the CLI: each has its own
    /// connection, so nothing but the claim stops both answers from writing.
    async fn file_backed_pools(path: &std::path::Path) -> (SqlitePool, SqlitePool) {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        async fn connect(path: &std::path::Path) -> SqlitePool {
            let options = SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .foreign_keys(true)
                .busy_timeout(std::time::Duration::from_secs(10))
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
            SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(options)
                .await
                .unwrap()
        }
        let first = connect(path).await;
        sqlx::migrate!("./migrations").run(&first).await.unwrap();
        let second = connect(path).await;
        (first, second)
    }

    /// A second runtime thread keeps a busy-waiting writer on one connection from starving the
    /// other answer; the assertions are on committed state, so no timing is involved.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_card_press_and_short_reply_record_one_decision() {
        let dir = std::env::temp_dir().join(format!(
            "omon-ledger-b4-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ledger.sqlite3");
        let (card_pool, reply_pool) = file_backed_pools(&path).await;
        let lead = "slack:T1:C1::U1";
        let (id, cls) = pending_push(&card_pool, "/repo-race").await;

        let (card, reply) = tokio::join!(
            finalize_ask_result(
                &card_pool,
                id,
                &cls,
                "engine_approval",
                lead,
                Ok(ApprovalDecision::Always),
            ),
            decide_proposal(
                &reply_pool,
                None,
                id,
                "allow-always",
                PrecedentScope::Exact,
                None,
                Some(lead),
            ),
        );
        assert_eq!(card.unwrap(), AskResolution::Allow);
        assert_eq!(reply.unwrap().state, "approved");

        let (approvals, declines): (i64, i64) = sqlx::query_as(
            "SELECT approvals, declines FROM action_precedents WHERE signature = ? AND scope = 'exact'",
        )
        .bind(&cls.signature)
        .fetch_one(&card_pool)
        .await
        .unwrap();
        assert_eq!(
            (approvals, declines),
            (1, 0),
            "a card press and a short reply at once are one owner decision"
        );
        let precedents: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM action_precedents WHERE signature = ?")
                .bind(&cls.signature)
                .fetch_one(&card_pool)
                .await
                .unwrap();
        assert_eq!(
            precedents, 1,
            "the losing answer must not add a precedent row"
        );
        let state: String = sqlx::query_scalar("SELECT state FROM action_proposals WHERE id = ?")
            .bind(id)
            .fetch_one(&card_pool)
            .await
            .unwrap();
        assert_eq!(state, "approved");

        card_pool.close().await;
        reply_pool.close().await;
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn deciding_a_missing_proposal_writes_no_precedent() {
        let pool = crate::storage::init_pool("sqlite::memory:").await.unwrap();
        let lead = "slack:T1:C1::U1";
        let cls = Classification::new(
            Effect::VcsRemote,
            "git push",
            "/repo-missing",
            "git push origin lane/1",
        );

        let resolution = finalize_ask_result(
            &pool,
            9_999,
            &cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Always),
        )
        .await
        .unwrap();
        assert_eq!(
            resolution,
            AskResolution::Deny {
                reason: "proposal #9999 no longer exists; the action was not run".into(),
                count_toward_limit: false,
            }
        );

        let decided = decide_proposal(
            &pool,
            None,
            9_999,
            "allow-always",
            PrecedentScope::Exact,
            None,
            Some(lead),
        )
        .await;
        assert!(decided.is_err(), "a missing proposal cannot be decided");

        let precedents: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM action_precedents WHERE signature = ?")
                .bind(&cls.signature)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            precedents, 0,
            "a decision for a missing proposal must not create a precedent"
        );
        let proposals: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM action_proposals WHERE id = ?")
                .bind(9_999_i64)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(proposals, 0, "nothing to update either");
    }
}

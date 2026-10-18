use std::collections::BTreeMap;
use std::str::FromStr;

use chrono::{DateTime, NaiveTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

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
    let db_pending = match pool {
        Some(p) => list_pending_proposals_for_session(p, &session.storage_key()).await?,
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
        let _ = decide_proposal(
            pool,
            guard,
            proposal.id,
            kind,
            PrecedentScope::Exact,
            words,
            Some(&proposal.session_key),
        )
        .await;
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
    if input.yolo {
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

    if in_quiet_hours_kst(input.now) {
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

        return Ok(PreAskOutcome::Deny {
            proposal_id,
            precedent_id: None,
            reason: format!("quiet hours; proposal #{proposal_id} will be asked at 08:00"),
            count_toward_limit: false,
        });
    }

    Ok(PreAskOutcome::Ask { proposal_id })
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
    match result {
        Ok(ApprovalDecision::Once | ApprovalDecision::Session) => {
            record_once_and_check_candidate(
                pool,
                proposal_id,
                classification.effect,
                &classification.target,
                lead_session_key,
            )
            .await?;
            Ok(AskResolution::Allow)
        }
        Ok(ApprovalDecision::Always) => {
            let prec_id = upsert_precedent(
                pool,
                &classification.signature,
                classification.effect,
                &classification.target,
                PrecedentDecision::Allow,
                PrecedentScope::Exact,
                origin,
                None,
            )
            .await?;
            update_proposal_decision(pool, proposal_id, ProposalState::Approved, prec_id, None)
                .await?;
            Ok(AskResolution::Allow)
        }
        Ok(ApprovalDecision::Deny { reason }) => {
            let words = reason.as_deref().unwrap_or("declined by owner");
            let prec_id = upsert_precedent(
                pool,
                &classification.signature,
                classification.effect,
                &classification.target,
                PrecedentDecision::Deny,
                PrecedentScope::Exact,
                origin,
                Some(words),
            )
            .await?;
            update_proposal_decision(
                pool,
                proposal_id,
                ProposalState::Declined,
                prec_id,
                Some(words),
            )
            .await?;
            Ok(AskResolution::Deny {
                reason: words.to_string(),
                count_toward_limit: true,
            })
        }
        Err(ApprovalError::Timeout | ApprovalError::Cancelled | ApprovalError::Persistence(_)) => {
            update_proposal_decision(pool, proposal_id, ProposalState::Expired, None, None).await?;
            Ok(AskResolution::Deny {
                reason: format!(
                    "owner has not answered yet; continue with other work and mention pending proposal #{proposal_id} in your report"
                ),
                count_toward_limit: false,
            })
        }
    }
}

pub async fn record_once_and_check_candidate(
    pool: &SqlitePool,
    proposal_id: i64,
    effect: Effect,
    target: &str,
    lead_session_key: &str,
) -> Result<bool, sqlx::Error> {
    update_proposal_decision(pool, proposal_id, ProposalState::Approved, None, None).await?;
    if !effect.allows_class_precedent() {
        return Ok(false);
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM action_proposals
         WHERE state = 'approved' AND effect = ? AND target = ? AND precedent_id IS NULL",
    )
    .bind(effect.as_str())
    .bind(target)
    .fetch_one(pool)
    .await?;

    if count != 3 {
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
    .fetch_one(pool)
    .await?;
    if has_deny {
        return Ok(false);
    }

    let sigs: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT signature FROM action_proposals
         WHERE state = 'approved' AND effect = ? AND target = ?
         ORDER BY id ASC",
    )
    .bind(effect.as_str())
    .bind(target)
    .fetch_all(pool)
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
    .execute(pool)
    .await?;
    Ok(true)
}

pub async fn recover_pending_proposals_on_startup(
    pool: &SqlitePool,
    default_lead: &str,
) -> Result<usize, sqlx::Error> {
    let pending: Vec<ActionProposal> =
        sqlx::query_as("SELECT * FROM action_proposals WHERE state = 'pending' ORDER BY id ASC")
            .fetch_all(pool)
            .await?;
    if pending.is_empty() {
        return Ok(0);
    }
    sqlx::query(
        "UPDATE action_proposals
         SET state = 'expired', decided_at = CURRENT_TIMESTAMP
         WHERE state = 'pending'",
    )
    .execute(pool)
    .await?;

    let mut by_lead: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in &pending {
        let lead = if p.session_key.starts_with("lane:") {
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
        };
        by_lead
            .entry(lead)
            .or_default()
            .push(format!("#{} ({} {})", p.id, p.effect, p.summary));
    }

    for (lead, items) in by_lead {
        let body = format!(
            "[ACTION_LOOP] {} expired pending proposal(s) on restart: {}",
            items.len(),
            items.join("; ")
        );
        sqlx::query(
            "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at)
             VALUES (NULL, ?, 'action_loop', ?, CURRENT_TIMESTAMP)",
        )
        .bind(&lead)
        .bind(&body)
        .execute(pool)
        .await?;
    }
    Ok(pending.len())
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
    let proposal = get_proposal(pool, proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("proposal #{proposal_id} not found"))?;
    let effect = Effect::from_str(&proposal.effect)?;
    let fallback_lead = std::env::var("OMON_LANE_DEFAULT_LEAD").ok();
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

    match decision_kind.trim().to_ascii_lowercase().as_str() {
        "allow" | "once" => {
            if scope == PrecedentScope::Class {
                if !effect.allows_class_precedent() {
                    return Err(format!(
                        "effect `{}` does not allow class precedents",
                        effect.as_str()
                    ));
                }
                let prec_id = upsert_precedent(
                    pool,
                    &sig_for_scope,
                    effect,
                    &proposal.target,
                    PrecedentDecision::Allow,
                    PrecedentScope::Class,
                    "owner_decide",
                    owner_words,
                )
                .await
                .map_err(|e| e.to_string())?;
                update_proposal_decision(
                    pool,
                    proposal_id,
                    ProposalState::Approved,
                    prec_id,
                    owner_words,
                )
                .await
                .map_err(|e| e.to_string())?;
            } else {
                record_once_and_check_candidate(pool, proposal_id, effect, &proposal.target, lead)
                    .await
                    .map_err(|e| e.to_string())?;
                if let Some(words) = owner_words {
                    update_proposal_decision(
                        pool,
                        proposal_id,
                        ProposalState::Approved,
                        None,
                        Some(words),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
            if let Some(g) = guard {
                let _ = g.resolve_proposal_id(proposal_id, ApprovalDecision::Once);
            }
        }
        "allow-always" | "always" => {
            if !effect.allows_any_precedent() {
                return Err("spend never gets a precedent".to_string());
            }
            if scope == PrecedentScope::Class && !effect.allows_class_precedent() {
                return Err(format!(
                    "effect `{}` does not allow class precedents",
                    effect.as_str()
                ));
            }
            let prec_id = upsert_precedent(
                pool,
                &sig_for_scope,
                effect,
                &proposal.target,
                PrecedentDecision::Allow,
                scope,
                "owner_decide",
                owner_words,
            )
            .await
            .map_err(|e| e.to_string())?;
            update_proposal_decision(
                pool,
                proposal_id,
                ProposalState::Approved,
                prec_id,
                owner_words,
            )
            .await
            .map_err(|e| e.to_string())?;
            if let Some(g) = guard {
                let _ = g.resolve_proposal_id(proposal_id, ApprovalDecision::Always);
            }
        }
        "deny" | "decline" => {
            let prec_id = upsert_precedent(
                pool,
                &sig_for_scope,
                effect,
                &proposal.target,
                PrecedentDecision::Deny,
                scope,
                "owner_decide",
                owner_words,
            )
            .await
            .map_err(|e| e.to_string())?;
            update_proposal_decision(
                pool,
                proposal_id,
                ProposalState::Declined,
                prec_id,
                owner_words,
            )
            .await
            .map_err(|e| e.to_string())?;
            if let Some(g) = guard {
                let _ = g.resolve_proposal_id(
                    proposal_id,
                    ApprovalDecision::Deny {
                        reason: owner_words.map(str::to_string),
                    },
                );
            }
        }
        other => {
            return Err(format!(
                "invalid decision `{other}` (expected allow, deny, or allow-always)"
            ));
        }
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
    .execute(pool)
    .await?;

    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM action_precedents WHERE signature = ? AND scope = ?")
            .bind(signature)
            .bind(scope.as_str())
            .fetch_optional(pool)
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

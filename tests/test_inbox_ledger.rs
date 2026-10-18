use std::sync::Arc;

use async_trait::async_trait;
use omon_gateway::autonomy::{
    get_proposal, upsert_precedent, Effect, PrecedentDecision, PrecedentScope,
};
use omon_gateway::inbox::proposals::{
    backfill_pending_inbox_proposals, Action, Actions, ApprovalResult, Proposal, Proposals,
    OWNER_ID,
};
use omon_gateway::storage::init_pool;
use omon_gateway::{OutboundAction, OutboundDispatcher, Result, SessionKey};
use parking_lot::Mutex;
use sqlx::SqlitePool;

#[derive(Default, Clone)]
struct FakeActions {
    lookups: Arc<Mutex<Vec<i64>>>,
    drafts: Arc<Mutex<Vec<i64>>>,
    lanes: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Actions for FakeActions {
    async fn start_lane(
        &self,
        key: &str,
        _proposal: &Proposal,
        _cwd: &str,
        _done: &str,
    ) -> Result<()> {
        self.lanes.lock().push(key.to_string());
        Ok(())
    }

    async fn lookup(&self, proposal: &Proposal) -> Result<String> {
        self.lookups.lock().push(proposal.number);
        Ok(format!("lookup result for #{}", proposal.number))
    }

    async fn draft(&self, proposal: &Proposal) -> Result<String> {
        self.drafts.lock().push(proposal.number);
        Ok(format!("draft text for #{}", proposal.number))
    }
}

#[derive(Default, Clone)]
struct RecordingCards {
    cards: Arc<Mutex<Vec<OutboundAction>>>,
}

#[async_trait]
impl OutboundDispatcher for RecordingCards {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        if matches!(action, OutboundAction::ApprovalRequest { .. }) {
            self.cards.lock().push(action);
        }
        Ok(())
    }
}

async fn seed_bot_item(
    pool: &SqlitePool,
    id: &str,
    author: &str,
    title: &str,
    proposed_action: &str,
) {
    sqlx::query(
        "INSERT INTO inbox_items (id, source, source_ref, author, title, snippet, link, received_at, raw_hash, collected_at)
         VALUES (?, 'mail', ?, ?, ?, 'body', 'https://example.com/item', CURRENT_TIMESTAMP, 'hash', CURRENT_TIMESTAMP)",
    )
    .bind(id)
    .bind(id)
    .bind(author)
    .bind(title)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO inbox_decisions (item_id, class, reason, proposed_action, status, decided_at, model)
         VALUES (?, 'bot', 'actionable', ?, 'open', CURRENT_TIMESTAMP, 'jev')",
    )
    .bind(id)
    .bind(proposed_action)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn a_lookup_auto_runs_with_no_card() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(
        &pool,
        "item-lookup",
        "alice@example.com",
        "Check status",
        "lookup link",
    )
    .await;

    let prop = proposals
        .propose("item-lookup", "C1", &Action::Lookup)
        .await
        .unwrap();

    let actions = FakeActions::default();
    let cards = RecordingCards::default();
    let sess = SessionKey::new("slack", Some("T1"), "C1", None::<String>, OWNER_ID);

    let res = proposals
        .evaluate_with_ledger(prop.number, &actions, Some(&cards), Some(&sess))
        .await
        .unwrap();
    assert!(matches!(res, ApprovalResult::Executed { number, .. } if number == prop.number));
    assert_eq!(actions.lookups.lock().as_slice(), &[prop.number]);
    assert!(
        cards.cards.lock().is_empty(),
        "Lookup (read) must auto-run with 0 approval cards"
    );

    let ledger_id = proposals
        .ledger_proposal_id("item-lookup")
        .await
        .unwrap()
        .unwrap();
    let row = get_proposal(&pool, ledger_id).await.unwrap().unwrap();
    assert_eq!(row.origin, "inbox");
    assert_eq!(row.tainted, 1);
    assert_eq!(row.effect, "read");
    assert_eq!(row.state, "auto_allowed");
}

#[tokio::test]
async fn b_draft_with_no_precedent_posts_one_card() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(
        &pool,
        "item-draft",
        "bob@example.com",
        "Meeting notes",
        "prepare summary note",
    )
    .await;

    let prop = proposals
        .propose("item-draft", "C1", &Action::Draft)
        .await
        .unwrap();

    let actions = FakeActions::default();
    let cards = RecordingCards::default();
    let sess = SessionKey::new("slack", Some("T1"), "C1", None::<String>, OWNER_ID);

    let res = proposals
        .evaluate_with_ledger(prop.number, &actions, Some(&cards), Some(&sess))
        .await
        .unwrap();
    assert_eq!(res, ApprovalResult::NotMatched);
    assert!(actions.drafts.lock().is_empty());
    assert_eq!(
        cards.cards.lock().len(),
        1,
        "Draft with no precedent must post exactly 1 card"
    );
}

#[tokio::test]
async fn c_exact_recipient_precedent_auto_runs_the_draft() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(
        &pool,
        "item-send-exact",
        "carol@example.com",
        "Invoice question",
        "reply to carol@example.com with receipt",
    )
    .await;

    let prop = proposals
        .propose("item-send-exact", "C1", &Action::Draft)
        .await
        .unwrap();

    upsert_precedent(
        &pool,
        "external_send:inbox_draft:carol@example.com",
        Effect::ExternalSend,
        "carol@example.com",
        PrecedentDecision::Allow,
        PrecedentScope::Exact,
        "owner",
        Some("Carol에게는 바로 회신 초안 실행해"),
    )
    .await
    .unwrap();

    let actions = FakeActions::default();
    let cards = RecordingCards::default();
    let sess = SessionKey::new("slack", Some("T1"), "C1", None::<String>, OWNER_ID);

    let res = proposals
        .evaluate_with_ledger(prop.number, &actions, Some(&cards), Some(&sess))
        .await
        .unwrap();
    assert!(matches!(res, ApprovalResult::Executed { number, .. } if number == prop.number));
    assert_eq!(actions.drafts.lock().as_slice(), &[prop.number]);
    assert!(
        cards.cards.lock().is_empty(),
        "exact recipient precedent must auto-run the draft with no card"
    );
}

#[tokio::test]
async fn d_tainted_external_send_with_only_class_precedent_still_asks() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(
        &pool,
        "item-send-class",
        "dave@example.com",
        "Contract",
        "reply to dave@example.com immediately",
    )
    .await;

    let prop = proposals
        .propose("item-send-class", "C1", &Action::Draft)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO action_precedents
            (signature, effect, target_scope, decision, scope, approvals, declines, origin)
         VALUES ('external_send:class:dave@example.com', 'external_send', 'dave@example.com', 'allow', 'class', 3, 0, 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let actions = FakeActions::default();
    let cards = RecordingCards::default();
    let sess = SessionKey::new("slack", Some("T1"), "C1", None::<String>, OWNER_ID);

    let res = proposals
        .evaluate_with_ledger(prop.number, &actions, Some(&cards), Some(&sess))
        .await
        .unwrap();
    assert_eq!(res, ApprovalResult::NotMatched);
    assert!(
        actions.drafts.lock().is_empty(),
        "tainted external_send with only class precedent must NOT auto-run"
    );
    assert_eq!(
        cards.cards.lock().len(),
        1,
        "tainted external_send with only class precedent must still ask"
    );
}

#[tokio::test]
async fn e_startup_pass_is_idempotent() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let _proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(&pool, "legacy-1", "eve@example.com", "Legacy 1", "draft 1").await;
    seed_bot_item(
        &pool,
        "legacy-2",
        "frank@example.com",
        "Legacy 2",
        "draft 2",
    )
    .await;

    for (num, item_id) in [(1_i64, "legacy-1"), (2_i64, "legacy-2")] {
        sqlx::query(
            "INSERT INTO inbox_proposals
                (number, item_id, owner_id, channel_id, action_json, proposed_action, title, state)
             VALUES (?, ?, ?, 'C1', '{\"kind\":\"draft\"}', 'draft reply', 'Legacy', 'pending')",
        )
        .bind(num)
        .bind(item_id)
        .bind(OWNER_ID)
        .execute(&pool)
        .await
        .unwrap();
    }

    let first_pass = backfill_pending_inbox_proposals(&pool).await.unwrap();
    assert_eq!(first_pass, 2);

    let second_pass = backfill_pending_inbox_proposals(&pool).await.unwrap();
    assert_eq!(second_pass, 0, "startup backfill pass must be idempotent");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM action_proposals WHERE origin = 'inbox'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn m4_production_call_without_card_sink_runs_lookup_and_leaves_draft_pending() {
    // triage.rs serve() calls evaluate_with_ledger(number, actions, None, None) for each
    // pending proposal; drafts stay pending for the owner's numbered reply.
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let proposals = Proposals::new(pool.clone(), OWNER_ID).await.unwrap();
    seed_bot_item(
        &pool,
        "m4-lookup",
        "alice@example.com",
        "Status",
        "lookup link",
    )
    .await;
    seed_bot_item(
        &pool,
        "m4-draft",
        "bob@example.com",
        "Notes",
        "prepare summary note",
    )
    .await;
    let lookup = proposals
        .propose("m4-lookup", "C1", &Action::Lookup)
        .await
        .unwrap();
    let draft = proposals
        .propose("m4-draft", "C1", &Action::Draft)
        .await
        .unwrap();
    let actions = FakeActions::default();

    for number in [lookup.number, draft.number] {
        proposals
            .evaluate_with_ledger(number, &actions, None, None)
            .await
            .unwrap();
    }
    assert_eq!(actions.lookups.lock().as_slice(), &[lookup.number]);
    assert!(actions.drafts.lock().is_empty());
    let pending: Vec<i64> = proposals
        .pending("C1")
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.number)
        .collect();
    assert_eq!(pending, vec![draft.number]);
}

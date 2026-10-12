use async_trait::async_trait;
use omon_gateway::{
    inbox::proposals::{
        approval_number, Action, Actions, ApprovalResult, Proposal, Proposals, Reply, OWNER_ID,
    },
    storage::{insert_lane, Database, Lane},
    OmonError, Result,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct FakeActions {
    pool: sqlx::SqlitePool,
    calls: AtomicUsize,
    fail: bool,
}

#[async_trait]
impl Actions for FakeActions {
    async fn start_lane(&self, key: &str, proposal: &Proposal, _: &str, done: &str) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(proposal.brief().contains("https://example.invalid/source"));
        if self.fail {
            return Err(OmonError::Config("fake lane failure".into()));
        }
        let status: String =
            sqlx::query_scalar("SELECT status FROM inbox_decisions WHERE item_id=?")
                .bind(&proposal.item_id)
                .fetch_one(&self.pool)
                .await?;
        assert_eq!(status, "approved");
        insert_lane(
            &self.pool,
            &Lane::new(
                key,
                "test",
                "discord:owner",
                "/scratch",
                "/scratch/brief",
                key,
                done,
            ),
        )
        .await
    }
    async fn lookup(&self, _: &Proposal) -> Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("lookup result".into())
    }
    async fn draft(&self, _: &Proposal) -> Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("draft for owner to copy/send".into())
    }
}

async fn fixture(action: Action, fail: bool) -> (Database, Proposals, Proposal, FakeActions) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::query("INSERT INTO inbox_items(id,source,source_ref,snippet,received_at,collected_at,raw_hash,link,title)
        VALUES('item','mail','ref','bounded snippet','2026-10-11','2026-10-11','hash','https://example.invalid/source','request')")
        .execute(db.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO inbox_decisions(item_id,class,reason,proposed_action,status,decided_at)
        VALUES('item','bot','reason','prepare implementation','open','2026-10-11')",
    )
    .execute(db.pool())
    .await
    .unwrap();
    let proposals = Proposals::new(db.pool().clone(), OWNER_ID).await.unwrap();
    let proposal = proposals
        .propose("item", "owner-dm", &action)
        .await
        .unwrap();
    proposals
        .presented(proposal.number, "owner-dm", "list-message")
        .await
        .unwrap();
    let fake = FakeActions {
        pool: db.pool().clone(),
        calls: AtomicUsize::new(0),
        fail,
    };
    (db, proposals, proposal, fake)
}

fn repo() -> Action {
    Action::Repo {
        cwd: "/scratch".into(),
        done: "file /scratch/result".into(),
    }
}

#[tokio::test]
async fn local_approval_starts_one_linked_lane_and_repeat_is_claimed() {
    let (db, proposals, proposal, fake) = fixture(repo(), false).await;
    omon_gateway::inbox::set::set_with_actions(
        db.pool(),
        "item",
        omon_gateway::inbox::set::Status::Approved,
        &fake,
    )
    .await
    .unwrap();
    omon_gateway::inbox::set::set_with_actions(
        db.pool(),
        "item",
        omon_gateway::inbox::set::Status::Approved,
        &fake,
    )
    .await
    .unwrap();
    assert_eq!(
        proposals.approve_item("item", &fake).await.unwrap(),
        ApprovalResult::AlreadyClaimed
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let (state, approval): (String, String) =
        sqlx::query_as("SELECT state,approval_message_id FROM inbox_proposals WHERE number=?")
            .bind(proposal.number)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(state, "completed");
    assert_eq!(approval, format!("cli:inbox-proposal-{}", proposal.number));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lanes l JOIN inbox_decisions d ON d.lane_key=l.key WHERE d.item_id='item'")
        .fetch_one(db.pool()).await.unwrap();
    assert_eq!(count, 1);
}
fn reply(content: &str) -> Reply<'_> {
    Reply {
        author_id: OWNER_ID,
        is_bot: false,
        channel_id: "owner-dm",
        message_id: "owner-reply",
        referenced_message_id: Some("list-message"),
        content,
    }
}

#[tokio::test]
async fn explicit_owner_reply_starts_lane_and_persists_outcome() {
    // Given
    let (db, proposals, proposal, fake) = fixture(repo(), false).await;
    let content = format!("{} 해", proposal.number);
    // When
    let result = proposals.approve(reply(&content), &fake).await.unwrap();
    // Then
    let key = format!("inbox-proposal-{}", proposal.number);
    assert_eq!(
        result,
        ApprovalResult::Executed {
            number: proposal.number,
            lane_key: Some(key.clone()),
            owner_text: None
        }
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let (status, lane, outcome): (String, String, String) =
        sqlx::query_as("SELECT status,lane_key,outcome FROM inbox_decisions WHERE item_id='item'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(status, "approved");
    assert_eq!(lane, key);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&outcome).unwrap()["state"],
        "completed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lanes")
            .fetch_one(db.pool())
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn unauthenticated_ambiguous_or_unmatched_replies_never_execute() {
    // Given
    let (_, proposals, proposal, fake) = fixture(repo(), false).await;
    let good = format!("{} 해", proposal.number);
    // When
    for case in 0..8 {
        let mut input = reply(&good);
        match case {
            0 => input.author_id = "stranger",
            1 => input.is_bot = true,
            2 => input.channel_id = "shared",
            3 => input.referenced_message_id = None,
            4 => input.referenced_message_id = Some("other-list"),
            5 => input.content = "해",
            6 => input.content = "1 해 2 해",
            7 => input.content = "999 해",
            _ => unreachable!(),
        }
        assert_eq!(
            proposals.approve(input, &fake).await.unwrap(),
            ApprovalResult::NotMatched
        );
    }
    // Then
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn disabled_auto_lookup_leaves_work_unstarted() {
    // Given
    let (db, proposals, proposal, fake) = fixture(Action::Lookup, false).await;
    // When
    for enabled in [false] {
        assert_eq!(
            proposals
                .auto_lookup(proposal.number, enabled, &fake)
                .await
                .unwrap(),
            ApprovalResult::NotMatched
        );
    }
    // Then
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM inbox_decisions")
            .fetch_one(db.pool())
            .await
            .unwrap(),
        "proposed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lanes")
            .fetch_one(db.pool())
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn concurrent_duplicate_approval_runs_exactly_once() {
    // Given
    let (_, proposals, proposal, fake) = fixture(repo(), false).await;
    let content = format!("{} 해", proposal.number);
    // When
    let (a, b) = tokio::join!(
        proposals.approve(reply(&content), &fake),
        proposals.approve(reply(&content), &fake)
    );
    // Then
    let results = [a.unwrap(), b.unwrap()];
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, ApprovalResult::Executed { .. }))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == ApprovalResult::AlreadyClaimed)
            .count(),
        1
    );
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn enabled_auto_lookup_executes_only_lookup_once() {
    for action in [Action::Lookup, Action::Draft, repo()] {
        let is_lookup = action == Action::Lookup;
        let (_, proposals, proposal, fake) = fixture(action, false).await;
        let result = proposals
            .auto_lookup(proposal.number, true, &fake)
            .await
            .unwrap();
        assert_eq!(matches!(result, ApprovalResult::Executed { .. }), is_lookup);
        let repeat = proposals
            .auto_lookup(proposal.number, true, &fake)
            .await
            .unwrap();
        assert_eq!(
            repeat,
            if is_lookup {
                ApprovalResult::AlreadyClaimed
            } else {
                ApprovalResult::NotMatched
            }
        );
        assert_eq!(fake.calls.load(Ordering::SeqCst), usize::from(is_lookup));
    }
}

#[tokio::test]
async fn failed_execution_is_recorded_and_not_retried_after_restart() {
    // Given
    let (db, proposals, proposal, fake) = fixture(repo(), true).await;
    let content = format!("{} 해", proposal.number);
    assert!(proposals.approve(reply(&content), &fake).await.is_err());
    let restarted = Proposals::new(db.pool().clone(), OWNER_ID).await.unwrap();
    // When
    let result = restarted.approve(reply(&content), &fake).await.unwrap();
    // Then
    assert_eq!(result, ApprovalResult::AlreadyClaimed);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let raw: String = sqlx::query_scalar("SELECT outcome FROM inbox_decisions")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&raw).unwrap()["state"],
        "failed"
    );
}

#[tokio::test]
async fn approved_drafts_and_lookups_return_only_owner_text() {
    for action in [Action::Draft, Action::Lookup] {
        // Given
        let (db, proposals, proposal, fake) = fixture(action, false).await;
        let content = format!("{} 해", proposal.number);
        // When
        let result = proposals.approve(reply(&content), &fake).await.unwrap();
        // Then
        assert!(matches!(
            result,
            ApprovalResult::Executed {
                lane_key: None,
                owner_text: Some(_),
                ..
            }
        ));
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM lanes")
                .fetch_one(db.pool())
                .await
                .unwrap(),
            0
        );
    }
}

#[tokio::test]
async fn changed_or_dismissed_or_status_only_approved_proposals_do_not_run() {
    for mutation in [
        "proposed_action='changed'",
        "status='dismissed'",
        "status='approved'",
        "class='owner'",
    ] {
        // Given
        let (db, proposals, proposal, fake) = fixture(repo(), false).await;
        sqlx::query(&format!("UPDATE inbox_decisions SET {mutation}"))
            .execute(db.pool())
            .await
            .unwrap();
        let content = format!("{} 해", proposal.number);
        // When
        let result = proposals.approve(reply(&content), &fake).await.unwrap();
        // Then
        assert_eq!(result, ApprovalResult::AlreadyClaimed);
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn numbering_and_snapshot_survive_relisting_and_restart() {
    // Given
    let (db, _, proposal, _) = fixture(repo(), false).await;
    let restarted = Proposals::new(db.pool().clone(), OWNER_ID).await.unwrap();
    // When
    let repeated = restarted
        .propose("item", "owner-dm", &Action::Draft)
        .await
        .unwrap();
    // Then
    assert_eq!(repeated.number, proposal.number);
    assert_eq!(repeated.action_json, proposal.action_json);
    assert_eq!(repeated.message_id.as_deref(), Some("list-message"));
}

#[test]
fn approval_parser_accepts_only_one_positive_number_and_explicit_verb() {
    // Given
    let invalid = [
        "",
        "해",
        "0 해",
        "-1 해",
        "1",
        "1 해?",
        "1 해 말고",
        "1,2 해",
        "승인 1",
        "1 해\n2 해",
    ];
    // When / Then
    for content in invalid {
        assert_eq!(approval_number(content), None);
    }
    assert_eq!(approval_number("3 해"), Some(3));
}

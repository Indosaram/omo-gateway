use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::{
    inbox::{
        collect_all,
        prefilter::PrefilterRules,
        triage::{parse_due_at, Class, Triage, TriageWriter, WritingItem},
        InboxItem, InboxSource, InboxSourceKind,
    },
    judge::{Choice, DiffVerdict, Judge, JudgeError, Purpose},
    storage::Database,
    Result,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

struct Source(Vec<InboxItem>);
#[async_trait]
impl InboxSource for Source {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Mail
    }
    async fn fetch_since(&self, _: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        Ok((self.0.clone(), "cursor".into()))
    }
}

struct FakeJudge {
    calls: AtomicUsize,
    mode: &'static str,
}
#[async_trait]
impl Judge for FakeJudge {
    async fn choice(
        &self,
        purpose: Purpose,
        state: &str,
        _: &str,
        options: &[&str],
    ) -> std::result::Result<Choice, JudgeError> {
        assert_eq!(purpose, Purpose::InboxClass);
        assert_eq!(options.len(), 4);
        assert!(state.chars().count() <= 830);
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(match self.mode {
            "mixed" => Choice::new(options[call % 4]),
            "fallback" => Choice::fallback("owner_must_act"),
            "capped" => Choice::capped("unclassified"),
            mode => Choice::new(mode),
        })
    }
    async fn noul(&self, _: Purpose, _: &str, _: &str) -> std::result::Result<f64, JudgeError> {
        panic!("unexpected score")
    }
    async fn diff(
        &self,
        _: Purpose,
        _: &str,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<DiffVerdict, JudgeError> {
        panic!("unexpected diff")
    }
}

struct FakeWriter {
    calls: AtomicUsize,
    mode: &'static str,
}
#[async_trait]
impl TriageWriter for FakeWriter {
    fn model(&self) -> &str {
        "fake-small"
    }
    async fn write(&self, items: &[WritingItem]) -> Result<String> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(items.len() <= omon_gateway::inbox::triage::WRITING_BATCH_SIZE);
        assert!(items
            .iter()
            .all(|i| matches!(i.class, Class::Owner | Class::Bot)));
        assert!(serde_json::to_string(items).unwrap().chars().count() <= 6000);
        if self.mode == "failure" {
            return Err(omon_gateway::OmonError::Config("writer timeout".into()));
        }
        if self.mode == "malformed" || (self.mode == "recover-malformed" && call == 0) {
            return Ok("not JSON".into());
        }
        if self.mode == "missing" {
            return Ok(format!(
                "[{{\"id\":\"{}\",\"reason\":\"reason\"}}]",
                items[0].id
            ));
        }
        let rows: Vec<_> = items
            .iter()
            .map(|i| {
                serde_json::json!({
                    "id": if self.mode == "unknown" || (self.mode == "recover-unknown" && call == 0) { "unknown" } else { &i.id },
                    "reason": "fake reason", "proposed_action": if self.mode == "proposal" { Some("prepare implementation") } else { None }
                })
            })
            .collect();
        Ok(serde_json::to_string(&rows).unwrap())
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 0, 0, 0).unwrap()
}
fn source(count: usize) -> Source {
    Source(
        (0..count)
            .map(|i| {
                let mut item = InboxItem::new(
                    InboxSourceKind::Mail,
                    format!("item-{i}"),
                    "signature due today 12:00",
                    now(),
                );
                item.author = Some("notifications@github.com".into());
                item.title = Some("Issue notification".into());
                item
            })
            .collect(),
    )
}
async fn fixture(
    mode: &'static str,
    writer_mode: &'static str,
) -> (Database, Triage, Arc<FakeJudge>, Arc<FakeWriter>) {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let judge = Arc::new(FakeJudge {
        calls: AtomicUsize::new(0),
        mode,
    });
    let writer = Arc::new(FakeWriter {
        calls: AtomicUsize::new(0),
        mode: writer_mode,
    });
    let triage = Triage::new(
        db.pool().clone(),
        judge.clone(),
        writer.clone(),
        omon_gateway::SessionKey::new(
            "discord",
            None::<String>,
            "owner-dm",
            None::<String>,
            omon_gateway::inbox::proposals::OWNER_ID,
        )
        .storage_key(),
    );
    (db, triage, judge, writer)
}
async fn count(db: &Database, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(db.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn burst_classifies_every_item_and_batches_only_actionable_writing() {
    // Given
    let (db, mut triage, judge, writer) = fixture("mixed", "valid").await;
    let src = source(25);
    let collected = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
    assert_eq!(count(&db, "inbox_decisions").await, 0);
    // When
    triage.collected(&collected, now(), true).await.unwrap();
    triage
        .due(now() + chrono::Duration::seconds(120), true)
        .await
        .unwrap();
    // Then
    assert_eq!(judge.calls.load(Ordering::SeqCst), 25);
    assert_eq!(writer.calls.load(Ordering::SeqCst), 2);
    assert_eq!(count(&db, "inbox_decisions").await, 25);
    assert_eq!(count(&db, "lane_events").await, 7);
    let duplicate = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
    triage.collected(&duplicate, now(), true).await.unwrap();
    assert_eq!(judge.calls.load(Ordering::SeqCst), 25);
    assert!(triage.deadline().is_none());
}

#[tokio::test]
async fn informational_batches_never_call_chat_model() {
    for mode in ["fyi", "ignore"] {
        // Given
        let (db, mut triage, judge, writer) = fixture(mode, "valid").await;
        let src = source(20);
        let results = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
        // When
        triage.collected(&results, now(), true).await.unwrap();
        // Then
        assert_eq!(judge.calls.load(Ordering::SeqCst), 20);
        assert_eq!(writer.calls.load(Ordering::SeqCst), 0);
        assert_eq!(count(&db, "inbox_decisions").await, 20);
        assert_eq!(count(&db, "lane_events").await, 0);
    }
}

#[tokio::test]
async fn idle_collection_and_kill_switch_do_not_arm_or_call() {
    // Given
    let (db, mut triage, judge, writer) = fixture("owner_must_act", "valid").await;
    let empty = collect_all(db.pool(), &[&source(0)], &PrefilterRules::default()).await;
    // When
    triage.collected(&empty, now(), true).await.unwrap();
    let results = collect_all(db.pool(), &[&source(1)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), false).await.unwrap();
    triage
        .due(now() + chrono::Duration::seconds(120), false)
        .await
        .unwrap();
    // Then
    assert!(triage.deadline().is_none());
    assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    assert_eq!(writer.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn one_owner_item_waits_for_deadline_and_enqueues_once() {
    // Given
    let (db, mut triage, judge, _) = fixture("owner_must_act", "valid").await;
    let results = collect_all(db.pool(), &[&source(1)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), true).await.unwrap();
    assert_eq!(judge.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        triage.deadline(),
        Some(now() + chrono::Duration::seconds(120))
    );
    // When
    triage
        .due(now() + chrono::Duration::seconds(120), true)
        .await
        .unwrap();
    // Then
    assert_eq!(count(&db, "inbox_decisions").await, 1);
    let event: (String, String) = sqlx::query_as("SELECT kind, lead_session_key FROM lane_events")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(event.0, "inbox_urgent");
}

#[tokio::test]
async fn invalid_writing_and_unknown_ids_leave_decisions_pending() {
    for mode in ["malformed", "missing", "unknown"] {
        // Given
        let (db, mut triage, _, _) = fixture("owner_must_act", mode).await;
        let results = collect_all(db.pool(), &[&source(1)], &PrefilterRules::default()).await;
        triage.collected(&results, now(), true).await.unwrap();
        // When
        let result = triage
            .due(now() + chrono::Duration::seconds(120), true)
            .await;
        // Then
        assert_eq!(result.is_err(), mode != "unknown");
        assert_eq!(count(&db, "inbox_decisions").await, 0);
        assert_eq!(count(&db, "lane_events").await, 0);
    }
}

#[tokio::test]
async fn fallback_or_capped_judgment_never_reaches_writer() {
    for mode in ["fallback", "capped"] {
        // Given
        let (db, mut triage, _, writer) = fixture(mode, "valid").await;
        let results = collect_all(db.pool(), &[&source(20)], &PrefilterRules::default()).await;
        // When
        let result = triage.collected(&results, now(), true).await;
        // Then
        assert!(result.is_err());
        assert_eq!(writer.calls.load(Ordering::SeqCst), 0);
        assert_eq!(count(&db, "inbox_decisions").await, 0);
    }
}

#[tokio::test]
async fn pending_writing_recovers_at_next_deadline_without_new_insert() {
    for mode in ["recover-malformed", "recover-unknown"] {
        let (db, mut triage, _, writer) = fixture("owner_must_act", mode).await;
        let src = source(1);
        let results = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
        triage.collected(&results, now(), true).await.unwrap();
        let first = now() + chrono::Duration::seconds(120);
        let result = triage.due(first, true).await;
        assert_eq!(result.is_err(), mode == "recover-malformed");
        assert_eq!(
            triage.deadline(),
            Some(first + chrono::Duration::seconds(120))
        );
        let duplicates = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
        triage.collected(&duplicates, first, true).await.unwrap();
        triage.due(first, true).await.unwrap();
        assert_eq!(writer.calls.load(Ordering::SeqCst), 1);
        triage
            .due(first + chrono::Duration::seconds(120), true)
            .await
            .unwrap();
        assert_eq!(count(&db, "inbox_decisions").await, 1);
        assert_eq!(count(&db, "lane_events").await, 1);
        assert!(triage.deadline().is_none());
    }
}

#[tokio::test]
async fn writing_failure_preserves_information_and_bounds_recovery() {
    let (db, mut triage, _, writer) = fixture("mixed", "failure").await;
    let results = collect_all(db.pool(), &[&source(4)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), true).await.unwrap();
    assert!(triage
        .due(now() + chrono::Duration::seconds(120), true)
        .await
        .is_err());
    assert_eq!(count(&db, "inbox_decisions").await, 2);
    let classes: Vec<String> =
        sqlx::query_scalar("SELECT class FROM inbox_decisions ORDER BY class")
            .fetch_all(db.pool())
            .await
            .unwrap();
    assert_eq!(classes, ["fyi", "ignore"]);
    assert_eq!(
        triage.deadline(),
        Some(now() + chrono::Duration::seconds(240))
    );
    for seconds in [240, 360, 480] {
        let _result = triage
            .due(now() + chrono::Duration::seconds(seconds), true)
            .await;
    }
    assert!(triage.deadline().is_none());
    triage
        .due(now() + chrono::Duration::seconds(600), true)
        .await
        .unwrap();
    assert_eq!(writer.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn persistent_writer_failure_stops_after_three_recovery_deadlines() {
    let (db, mut triage, _, writer) = fixture("owner_must_act", "failure").await;
    let results = collect_all(db.pool(), &[&source(1)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), true).await.unwrap();
    for seconds in [120, 240, 360, 480] {
        assert!(triage
            .due(now() + chrono::Duration::seconds(seconds), true)
            .await
            .is_err());
    }
    assert!(triage.deadline().is_none());
    triage
        .due(now() + chrono::Duration::seconds(600), true)
        .await
        .unwrap();
    assert_eq!(writer.calls.load(Ordering::SeqCst), 4);
    assert_eq!(count(&db, "inbox_decisions").await, 0);
}

#[tokio::test]
async fn bot_triage_creates_numbered_executable_proposal_in_ordinary_list() {
    let (db, triage, _, _) = fixture("bot_can_handle", "proposal").await;
    let mut triage = triage.with_proposal_action(omon_gateway::inbox::proposals::Action::Repo {
        cwd: "/scratch".into(),
        done: "file /scratch/result".into(),
    });
    let results = collect_all(db.pool(), &[&source(1)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), true).await.unwrap();
    triage
        .due(now() + chrono::Duration::seconds(120), true)
        .await
        .unwrap();
    let items = omon_gateway::inbox::list::list(
        db.pool(),
        Some(omon_gateway::inbox::list::ClassFilter::Bot),
        true,
    )
    .await
    .unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].proposal_number.is_some());
    assert_eq!(items[0].status, "proposed");
    let action: String = sqlx::query_scalar("SELECT action_json FROM inbox_proposals")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<omon_gateway::inbox::proposals::Action>(&action).unwrap(),
        omon_gateway::inbox::proposals::Action::Repo {
            cwd: "/scratch".into(),
            done: "file /scratch/result".into()
        }
    );
    assert_eq!(count(&db, "lanes").await, 0);
    struct LaneActions(sqlx::SqlitePool, AtomicUsize);
    #[async_trait]
    impl omon_gateway::inbox::proposals::Actions for LaneActions {
        async fn start_lane(
            &self,
            key: &str,
            proposal: &omon_gateway::inbox::proposals::Proposal,
            cwd: &str,
            done: &str,
        ) -> Result<()> {
            self.1.fetch_add(1, Ordering::SeqCst);
            omon_gateway::storage::insert_lane(
                &self.0,
                &omon_gateway::storage::Lane::new(
                    key,
                    &proposal.title,
                    "discord:owner",
                    cwd,
                    "/scratch/brief",
                    key,
                    done,
                ),
            )
            .await
        }
        async fn lookup(&self, _: &omon_gateway::inbox::proposals::Proposal) -> Result<String> {
            panic!("unexpected lookup")
        }
        async fn draft(&self, _: &omon_gateway::inbox::proposals::Proposal) -> Result<String> {
            panic!("unexpected draft")
        }
    }
    let actions = LaneActions(db.pool().clone(), AtomicUsize::new(0));
    for _ in 0..2 {
        omon_gateway::inbox::set::set_with_actions(
            db.pool(),
            &items[0].id,
            omon_gateway::inbox::set::Status::Approved,
            &actions,
        )
        .await
        .unwrap();
    }
    assert_eq!(actions.1.load(Ordering::SeqCst), 1);
    assert_eq!(count(&db, "lanes").await, 1);
    let linked: i64 = sqlx::query_scalar("SELECT count(*) FROM lanes l JOIN inbox_decisions d ON d.lane_key=l.key WHERE d.status='approved'").fetch_one(db.pool()).await.unwrap();
    assert_eq!(linked, 1);
}

#[tokio::test]
async fn escaped_unicode_payload_is_bounded_without_extra_turns() {
    // Given
    let (db, mut triage, _, writer) = fixture("bot_can_handle", "valid").await;
    let mut src = source(20);
    for item in &mut src.0 {
        item.snippet = "\"\\\n한".repeat(250);
    }
    let results = collect_all(db.pool(), &[&src], &PrefilterRules::default()).await;
    // When
    triage.collected(&results, now(), true).await.unwrap();
    // Then: 20 owner/bot items fit one writing turn; the 6000-char cap
    // bounds the payload by truncating text, not by adding turns.
    assert_eq!(writer.calls.load(Ordering::SeqCst), 1);
    assert_eq!(count(&db, "inbox_decisions").await, 20);
    assert_eq!(count(&db, "lane_events").await, 0);
}

#[test]
fn deadlines_parse_without_model_in_owner_timezone() {
    // Given
    let clock = now();
    // When
    let due = parse_due_at("due today 12:00", clock);
    // Then
    assert_eq!(
        due,
        Some(Utc.with_ymd_and_hms(2026, 10, 11, 3, 0, 0).unwrap())
    );
    assert_eq!(
        parse_due_at("due 2026-10-12T12:00:00Z", clock),
        Some(Utc.with_ymd_and_hms(2026, 10, 12, 12, 0, 0).unwrap())
    );
    assert!(parse_due_at("2026-99-99", clock).is_none());
    assert!(parse_due_at("no deadline", clock).is_none());
}

#[tokio::test]
async fn zero_fallback_cap_still_admits_successful_jev_classification() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("typesafe-judge");
    std::fs::write(&bin, "#!/bin/sh\nprintf 'fyi\\n'\n").unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let judge = Arc::new(
        omon_gateway::judge::JevCli::new(Some(bin), Some(db.pool().clone()), None, None)
            .with_caps(300, 0)
            .with_jev_enabled(true),
    );
    let writer = Arc::new(FakeWriter {
        calls: AtomicUsize::new(0),
        mode: "valid",
    });
    let mut triage = Triage::new(
        db.pool().clone(),
        judge,
        writer.clone(),
        "discord:owner".into(),
    );
    let results = collect_all(db.pool(), &[&source(20)], &PrefilterRules::default()).await;
    triage.collected(&results, now(), true).await.unwrap();
    assert_eq!(count(&db, "inbox_decisions").await, 20);
    assert_eq!(writer.calls.load(Ordering::SeqCst), 0);
    let fallback: i64 =
        sqlx::query_scalar("SELECT count(*) FROM judge_calls WHERE used_fallback=1")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(fallback, 0);
}

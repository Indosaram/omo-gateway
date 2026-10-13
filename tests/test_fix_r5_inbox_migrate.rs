use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::agent::WorkHeartbeatConfig;
use omon_gateway::inbox::sources::{
    imessage::IMessageSource, kakao::KakaoSource, mail::MailSource,
};
use omon_gateway::inbox::{
    collect_all, prefilter::PrefilterRules, InboxItem, InboxSource, InboxSourceKind,
};
use omon_gateway::storage::{insert_work_item, Database, WorkItem};
use omon_gateway::usage;
use omon_gateway::work_heartbeat::{
    CapturedDiscordPayload, MockClock, MockLivenessChecker, RecordingHeartbeatSink,
    WorkHeartbeatRunner, WorkHeartbeatSink,
};
use omon_gateway::{OmonError, Result};

struct OneShotSource {
    items: Vec<InboxItem>,
}

#[async_trait]
impl InboxSource for OneShotSource {
    fn name(&self) -> InboxSourceKind {
        InboxSourceKind::Mail
    }

    async fn fetch_since(&self, _cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        Ok((self.items.clone(), "cursor".into()))
    }
}

fn mail_item(thread: &str, snippet: &str) -> InboxItem {
    InboxItem::new(
        InboxSourceKind::Mail,
        thread,
        snippet,
        Utc.timestamp_opt(1_760_000_000, 0).unwrap(),
    )
}

#[tokio::test]
async fn mail_follow_up_in_collected_thread_refreshes_item_and_retriages() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let first = mail_item("thread-1", "fyi newsletter");
    let item_id = first.id();
    let rules = PrefilterRules::default();
    let results = collect_all(pool, &[&OneShotSource { items: vec![first] }], &rules).await;
    assert_eq!(results[0].stats.inserted, 1);
    sqlx::query(
        "INSERT INTO inbox_decisions(item_id,class,reason,status,decided_at) VALUES(?,'fyi','r','open',CURRENT_TIMESTAMP)",
    )
    .bind(&item_id)
    .execute(pool)
    .await
    .unwrap();

    let follow_up = mail_item("thread-1", "please sign by today");
    let results = collect_all(
        pool,
        &[&OneShotSource {
            items: vec![follow_up],
        }],
        &rules,
    )
    .await;

    assert_eq!(results[0].stats.inserted, 1);
    assert_eq!(results[0].stats.duplicates, 0);
    let (rows, snippet): (i64, String) =
        sqlx::query_as("SELECT count(*), max(snippet) FROM inbox_items")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(rows, 1);
    assert_eq!(snippet, "please sign by today");
    let decisions: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_decisions")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(decisions, 0, "stale decision must be dropped for re-triage");

    let repeat = mail_item("thread-1", "please sign by today");
    let results = collect_all(
        pool,
        &[&OneShotSource {
            items: vec![repeat],
        }],
        &rules,
    )
    .await;
    assert_eq!(results[0].stats.duplicates, 1);
    assert_eq!(results[0].stats.inserted, 0);
}

#[tokio::test]
async fn dropped_items_are_counted_under_the_kst_day() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let before = Utc::now()
        .with_timezone(&chrono_tz::Asia::Seoul)
        .date_naive()
        .to_string();
    let source = OneShotSource {
        items: vec![mail_item("thread-empty", " ")],
    };
    let results = collect_all(pool, &[&source], &PrefilterRules::default()).await;
    let after = Utc::now()
        .with_timezone(&chrono_tz::Asia::Seoul)
        .date_naive()
        .to_string();
    assert_eq!(results[0].stats.dropped, 1);
    let day: String = sqlx::query_scalar("SELECT day FROM inbox_filter_stats")
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(
        day == before || day == after,
        "day {day} is not the KST day"
    );
}

#[tokio::test]
async fn intra_day_usage_window_counts_same_kst_day_drops() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    sqlx::query(
        "INSERT INTO inbox_filter_stats(day,source,rule,count) VALUES('2026-10-11','mail','ads',4)",
    )
    .execute(pool)
    .await
    .unwrap();
    let until = Utc.with_ymd_and_hms(2026, 10, 11, 3, 0, 0).unwrap();
    let report = usage::report(pool, until - chrono::Duration::hours(1), until)
        .await
        .unwrap();
    assert_eq!(report.inbox_collected, 4);
}

#[test]
fn mail_thread_without_snippet_does_not_fail_the_batch() {
    let payload = serde_json::json!({"items":[
        {"thread_id":"t1","date":"2026-10-11T08:00:00Z"},
        {"thread_id":"t2","snippet":"hello","date":"2026-10-11T08:01:00Z"}
    ]});
    let (items, _) = MailSource::parse(&payload, None).unwrap();
    assert_eq!(items.len(), 2);
    assert!(items[0].snippet.is_empty());
    assert_eq!(items[1].snippet, "hello");
}

#[test]
fn imessage_attachment_only_message_is_skipped_and_cursor_advances() {
    let chats = serde_json::json!([{"id":"1","last_message_at":"2026-10-11T08:02:00Z"}]);
    let history = serde_json::json!([
        {"id":"10","created_at":"2026-10-11T08:01:00Z","is_from_me":false},
        {"id":"11","created_at":"2026-10-11T08:02:00Z","text":"hi","is_from_me":false}
    ]);
    let (items, cursor) = IMessageSource::parse(&chats, &[("1".into(), history)], None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "11");
    assert!(cursor.starts_with("2026-10-11T08:02:00"));
}

#[test]
fn kakao_attachment_only_message_is_skipped_but_still_pages() {
    let page = serde_json::json!([
        {"id":"500","time":1_700_000_100,"text":"hello"},
        {"id":"400","time":1_700_000_000}
    ]);
    let (items, oldest, newest) =
        KakaoSource::parse_messages("room", None, "DirectChat", &page, "").unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "500");
    assert_eq!(oldest.as_deref(), Some("400"));
    assert_eq!(newest.as_deref(), Some("500"));
}

struct SelectiveSink {
    fail_channel: &'static str,
    inner: RecordingHeartbeatSink,
}

#[async_trait]
impl WorkHeartbeatSink for SelectiveSink {
    async fn post_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<CapturedDiscordPayload> {
        if channel_id == self.fail_channel {
            return Err(OmonError::Config("channel deleted".into()));
        }
        self.inner.post_message(channel_id, content).await
    }
}

#[tokio::test]
async fn heartbeat_failing_item_does_not_starve_later_items() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(t0));
    let sink = Arc::new(SelectiveSink {
        fail_channel: "bad-thread",
        inner: RecordingHeartbeatSink::new(),
    });
    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        WorkHeartbeatConfig {
            channel_id: Some("fallback".into()),
            threshold: Duration::from_secs(15 * 60),
            ..Default::default()
        },
        clock.clone(),
        Arc::new(MockLivenessChecker::new()),
        sink.clone(),
    );
    let older = (t0 - chrono::Duration::minutes(1)).to_rfc3339();
    insert_work_item(
        &pool,
        &WorkItem::new("bad", "Bad", Some("bad-thread".into()), None, older),
    )
    .await
    .unwrap();
    insert_work_item(
        &pool,
        &WorkItem::new(
            "good",
            "Good",
            Some("good-thread".into()),
            None,
            t0.to_rfc3339(),
        ),
    )
    .await
    .unwrap();
    clock.advance(chrono::Duration::minutes(20));

    let processed = runner.step().await.unwrap();

    assert_eq!(processed, 1);
    let payloads = sink.inner.recorded_payloads();
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].path, "/api/v10/channels/good-thread/messages");
}

#[tokio::test]
async fn heartbeat_message_is_bounded_below_discord_limit() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let t0 = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(t0));
    let sink = Arc::new(RecordingHeartbeatSink::new());
    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        WorkHeartbeatConfig {
            channel_id: Some("fallback".into()),
            threshold: Duration::from_secs(15 * 60),
            ..Default::default()
        },
        clock.clone(),
        Arc::new(MockLivenessChecker::new()),
        sink.clone(),
    );
    insert_work_item(
        &pool,
        &WorkItem::new(
            "long",
            "t".repeat(5000),
            Some("thread".into()),
            None,
            t0.to_rfc3339(),
        ),
    )
    .await
    .unwrap();
    clock.advance(chrono::Duration::minutes(20));

    assert_eq!(runner.step().await.unwrap(), 1);

    let payloads = sink.recorded_payloads();
    let content = payloads[0].body["content"].as_str().unwrap();
    assert!(content.chars().count() <= 2000);
}

#[tokio::test]
async fn deleting_a_session_purges_its_search_documents() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    sqlx::query(
        "INSERT INTO sessions(session_key,platform,channel_id,user_id) VALUES('s1','discord','chan','user')",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO messages(id,session_key,role,content,platform_message_id) VALUES('m1','s1','user','secret-token-text','p1')",
    )
    .execute(pool)
    .await
    .unwrap();
    let indexed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM message_search_documents WHERE message_id='p1'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(indexed, 1);

    sqlx::query("DELETE FROM sessions WHERE session_key='s1'")
        .execute(pool)
        .await
        .unwrap();

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM message_search_documents")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    let fts_hits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM message_search_fts WHERE message_search_fts MATCH 'secret'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(fts_hits, 0);
}

#[tokio::test]
async fn deleting_a_message_purges_only_its_search_document() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    sqlx::query(
        "INSERT INTO sessions(session_key,platform,channel_id,user_id) VALUES('s1','discord','chan','user')",
    )
    .execute(pool)
    .await
    .unwrap();
    for (id, platform_id) in [("m1", "p1"), ("m2", "p2")] {
        sqlx::query(
            "INSERT INTO messages(id,session_key,role,content,platform_message_id) VALUES(?,'s1','user','text',?)",
        )
        .bind(id)
        .bind(platform_id)
        .execute(pool)
        .await
        .unwrap();
    }

    sqlx::query("DELETE FROM messages WHERE id='m1'")
        .execute(pool)
        .await
        .unwrap();

    let ids: Vec<String> = sqlx::query_scalar("SELECT message_id FROM message_search_documents")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(ids, vec!["p2".to_string()]);
}

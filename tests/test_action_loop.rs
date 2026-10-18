use chrono::{TimeZone, Utc};
use omon_gateway::autonomy::action_loop::{action_loop_enabled, ActionLoop};
use omon_gateway::storage::{init_pool, take_undelivered_events_at};
use sqlx::SqlitePool;

const LEAD: &str = "slack:T1:C1::U1";

fn daytime_utc() -> chrono::DateTime<Utc> {
    chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 14, 0, 0)
        .unwrap()
        .with_timezone(&Utc)
}

fn quiet_0300_utc() -> chrono::DateTime<Utc> {
    chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 3, 0, 0)
        .unwrap()
        .with_timezone(&Utc)
}

async fn insert_inbox_item_and_decision(
    pool: &SqlitePool,
    id: &str,
    class: &str,
    author: &str,
    title: &str,
) {
    sqlx::query(
        "INSERT INTO inbox_items (id, source, source_ref, author, title, snippet, received_at, raw_hash, collected_at)
         VALUES (?, 'kakao', ?, ?, ?, 'body', CURRENT_TIMESTAMP, 'hash', CURRENT_TIMESTAMP)",
    )
    .bind(id)
    .bind(id)
    .bind(author)
    .bind(title)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO inbox_decisions (item_id, class, reason, status, decided_at, model)
         VALUES (?, ?, 'needs follow-up', 'open', CURRENT_TIMESTAMP, 'jev')",
    )
    .bind(id)
    .bind(class)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn a_three_inbox_rows_in_120s_make_one_event() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    for i in 1..=3 {
        insert_inbox_item_and_decision(
            &pool,
            &format!("kakao-{i}"),
            "owner",
            "Alice",
            &format!("문의 #{i}"),
        )
        .await;
    }

    let loop_worker = ActionLoop::new(pool.clone(), LEAD);
    let added = loop_worker.collect_new_items(daytime_utc()).await.unwrap();
    assert_eq!(added, 3);

    let event_id = loop_worker
        .flush_coalesced(daytime_utc())
        .await
        .unwrap()
        .expect("coalesced batch must emit 1 event");
    assert!(event_id > 0);

    let events = take_undelivered_events_at(&pool, None, daytime_utc())
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "action_loop");
    assert!(events[0].body.starts_with("[ACTION_LOOP] 3 items:"));
    assert!(events[0].body.contains("content from Alice:"));
}

#[tokio::test]
async fn b_duplicate_item_id_makes_none_including_after_restart() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    insert_inbox_item_and_decision(&pool, "kakao-dup", "bot", "Bob", "확인 요청").await;

    let worker_1 = ActionLoop::new(pool.clone(), LEAD);
    assert!(worker_1
        .flush_coalesced(daytime_utc())
        .await
        .unwrap()
        .is_some());

    let worker_2 = ActionLoop::new(pool.clone(), LEAD);
    let second = worker_2.flush_coalesced(daytime_utc()).await.unwrap();
    assert_eq!(
        second, None,
        "restarted ActionLoop must skip already-seen item via action_loop_seen"
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'action_loop'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn c_item_with_existing_proposal_makes_none() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let _ = omon_gateway::inbox::proposals::Proposals::new(pool.clone(), "U1")
        .await
        .unwrap();
    insert_inbox_item_and_decision(&pool, "kakao-prop", "bot", "Charlie", "작업 요청").await;

    sqlx::query(
        "INSERT INTO inbox_proposals
            (number, item_id, owner_id, channel_id, action_json, proposed_action, title, state)
         VALUES (1, 'kakao-prop', 'U1', 'C1', '{\"kind\":\"draft\"}', 'draft reply', '작업 요청', 'pending')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let worker = ActionLoop::new(pool.clone(), LEAD);
    let emitted = worker.flush_coalesced(daytime_utc()).await.unwrap();
    assert_eq!(
        emitted, None,
        "item with existing inbox_proposals row must make 0 events"
    );
}

#[tokio::test]
async fn d_event_over_daily_cap_is_dropped() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let worker = ActionLoop::new(pool.clone(), LEAD).with_daily_cap(2);

    for i in 1..=3 {
        insert_inbox_item_and_decision(
            &pool,
            &format!("cap-{i}"),
            "owner",
            "User",
            &format!("msg {i}"),
        )
        .await;
        let res = worker.flush_coalesced(daytime_utc()).await.unwrap();
        if i <= 2 {
            assert!(res.is_some(), "item {i} under cap must emit");
        } else {
            assert_eq!(res, None, "3rd event over cap=2 must be dropped");
        }
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'action_loop'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn e_quiet_hour_0300_kst_event_gets_release_at_0800_and_is_not_dispatched_before_then() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    insert_inbox_item_and_decision(&pool, "night-1", "owner", "NightUser", "새벽 메시지").await;

    let worker = ActionLoop::new(pool.clone(), LEAD);
    let now_0300 = quiet_0300_utc();
    let id = worker
        .flush_coalesced(now_0300)
        .await
        .unwrap()
        .expect("event inserted with release_at");
    assert!(id > 0);

    let before_0800 = take_undelivered_events_at(&pool, None, now_0300)
        .await
        .unwrap();
    assert!(
        before_0800.is_empty(),
        "03:00 KST event must not be dispatched before 08:00 KST"
    );

    let at_0801 = chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 8, 1, 0)
        .unwrap()
        .with_timezone(&Utc);
    let after_0800 = take_undelivered_events_at(&pool, None, at_0801)
        .await
        .unwrap();
    assert_eq!(after_0800.len(), 1);
    assert_eq!(after_0800[0].id, id);
}

#[test]
fn f_flag_on_means_kibitzer_is_not_started() {
    std::env::set_var("OMON_ACTION_LOOP_ENABLED", "true");
    let loop_on = action_loop_enabled();
    std::env::remove_var("OMON_ACTION_LOOP_ENABLED");
    assert!(loop_on);
    let kibitzer_should_start = !loop_on;
    assert!(!kibitzer_should_start);
}

#[tokio::test]
async fn g_first_enable_baselines_backlog_and_caps_listed_items() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    for i in 1..=30 {
        insert_inbox_item_and_decision(&pool, &format!("old-{i}"), "owner", "Old", "backlog").await;
    }
    let worker = ActionLoop::new(pool.clone(), LEAD);
    assert_eq!(
        worker.baseline_if_first_run(daytime_utc()).await.unwrap(),
        30
    );
    assert_eq!(
        worker.baseline_if_first_run(daytime_utc()).await.unwrap(),
        0
    );
    assert_eq!(worker.flush_coalesced(daytime_utc()).await.unwrap(), None);

    for i in 1..=25 {
        insert_inbox_item_and_decision(&pool, &format!("new-{i}"), "owner", "New", "fresh").await;
    }
    worker
        .flush_coalesced(daytime_utc())
        .await
        .unwrap()
        .expect("new rows emit");
    let events = take_undelivered_events_at(&pool, None, daytime_utc())
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].body.starts_with("[ACTION_LOOP] 25 items:"));
    assert!(events[0].body.ends_with("| +5 more"));
    assert!(!events[0].body.contains("backlog"));
}

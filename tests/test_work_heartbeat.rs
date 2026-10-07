use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use omon_gateway::agent::WorkHeartbeatConfig;
use omon_gateway::storage::{
    get_work_item, insert_work_item, mark_work_item_done, Database, WorkItem,
};
use omon_gateway::work_heartbeat::{
    CapturedDiscordPayload, Clock, DelegateLiveness, MockClock, MockLivenessChecker,
    RecordingHeartbeatSink, WorkHeartbeatRunner,
};
use sqlx::Row;

#[tokio::test]
async fn migration_0030_creates_work_items_table_with_expected_columns() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let table_rows =
        sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'work_items'")
            .fetch_all(pool)
            .await
            .unwrap();
    assert_eq!(table_rows.len(), 1);

    let column_rows = sqlx::query("PRAGMA table_info(work_items)")
        .fetch_all(pool)
        .await
        .unwrap();

    let columns: HashSet<String> = column_rows
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    for expected_column in [
        "id",
        "title",
        "thread_id",
        "delegate_pane",
        "status",
        "last_progress_at",
        "receipt_path",
        "created_at",
        "updated_at",
    ] {
        assert!(
            columns.contains(expected_column),
            "missing column {expected_column} in work_items"
        );
    }
}

#[tokio::test]
async fn heartbeat_15_minute_threshold_triggers_single_post_and_subsequent_step_skips() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let initial_time = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(initial_time));
    let checker = Arc::new(MockLivenessChecker::new());
    checker.set_status("pane-42", DelegateLiveness::Alive);
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        channel_id: Some("alert-chan-99".to_string()),
        threshold: Duration::from_secs(15 * 60),
        ..Default::default()
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let item = WorkItem::new(
        "item-101",
        "Database Index Migration",
        Some("thread-555".to_string()),
        Some("pane-42".to_string()),
        initial_time.to_rfc3339(),
    );
    insert_work_item(&pool, &item).await.unwrap();

    clock.advance(chrono::Duration::minutes(10));
    let processed_before_threshold = runner.step().await.unwrap();
    assert_eq!(processed_before_threshold, 0);
    assert_eq!(sink.recorded_payloads().len(), 0);

    clock.advance(chrono::Duration::minutes(5));
    let processed_at_threshold = runner.step().await.unwrap();
    assert_eq!(processed_at_threshold, 1);
    let payloads = sink.recorded_payloads();
    assert_eq!(payloads.len(), 1);

    let first_payload = &payloads[0];
    assert_eq!(first_payload.method, "POST");
    assert_eq!(first_payload.path, "/api/v10/channels/thread-555/messages");
    let content = first_payload.body["content"].as_str().unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("Database Index Migration"));
    assert!(lines[1].contains("Status: in_progress"));
    assert!(lines[1].contains("pane-42"));

    let updated_item = get_work_item(&pool, "item-101").await.unwrap().unwrap();
    assert_eq!(updated_item.last_progress_at, clock.now().to_rfc3339());

    let immediate_next_step = runner.step().await.unwrap();
    assert_eq!(immediate_next_step, 0);
    assert_eq!(sink.recorded_payloads().len(), 1);

    clock.advance(chrono::Duration::minutes(5));
    let sub_threshold_step = runner.step().await.unwrap();
    assert_eq!(sub_threshold_step, 0);
    assert_eq!(sink.recorded_payloads().len(), 1);

    clock.advance(chrono::Duration::minutes(10));
    let next_interval_step = runner.step().await.unwrap();
    assert_eq!(next_interval_step, 1);
    assert_eq!(sink.recorded_payloads().len(), 2);
}

#[tokio::test]
async fn heartbeat_dead_worker_escalation_posts_single_line_warning() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let initial_time = Utc.with_ymd_and_hms(2026, 10, 7, 10, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(initial_time));
    let checker = Arc::new(MockLivenessChecker::new());
    checker.set_status("pane-crashed", DelegateLiveness::Dead);
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        enabled: true,
        channel_id: Some("fallback-channel-1".to_string()),
        interval: Duration::from_secs(60),
        threshold: Duration::from_secs(900),
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let item = WorkItem::new(
        "item-dead-9",
        "Nightly Backup Job",
        None,
        Some("pane-crashed".to_string()),
        (initial_time - chrono::Duration::minutes(20)).to_rfc3339(),
    );
    insert_work_item(&pool, &item).await.unwrap();

    let processed = runner.step().await.unwrap();
    assert_eq!(processed, 1);

    let payloads = sink.recorded_payloads();
    assert_eq!(payloads.len(), 1);

    let payload = &payloads[0];
    assert_eq!(payload.method, "POST");
    assert_eq!(
        payload.path,
        "/api/v10/channels/fallback-channel-1/messages"
    );
    let content = payload.body["content"].as_str().unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("⚠️"));
    assert!(lines[0].contains("Worker dead"));
    assert!(lines[0].contains("Nightly Backup Job"));
    assert!(lines[0].contains("pane-crashed"));

    let second_pass = runner.step().await.unwrap();
    assert_eq!(second_pass, 0);
    assert_eq!(sink.recorded_payloads().len(), 1);
}

#[tokio::test]
async fn heartbeat_post_restart_recovers_without_spamming_stale_items() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let restart_time = Utc.with_ymd_and_hms(2026, 10, 7, 18, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(restart_time));
    let checker = Arc::new(MockLivenessChecker::new());
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        enabled: true,
        channel_id: Some("alert-chan".to_string()),
        interval: Duration::from_secs(60),
        threshold: Duration::from_secs(900),
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let stale_1 = WorkItem::new(
        "stale-1",
        "Legacy Import A",
        Some("ch-1".to_string()),
        None,
        (restart_time - chrono::Duration::hours(5)).to_rfc3339(),
    );
    let stale_2 = WorkItem::new(
        "stale-2",
        "Legacy Import B",
        Some("ch-2".to_string()),
        None,
        (restart_time - chrono::Duration::hours(2)).to_rfc3339(),
    );
    let fresh_3 = WorkItem::new(
        "fresh-3",
        "Recent Task C",
        Some("ch-3".to_string()),
        None,
        (restart_time - chrono::Duration::minutes(5)).to_rfc3339(),
    );
    let done_4 = WorkItem::new(
        "done-4",
        "Finished Task D",
        Some("ch-4".to_string()),
        None,
        (restart_time - chrono::Duration::hours(10)).to_rfc3339(),
    );

    insert_work_item(&pool, &stale_1).await.unwrap();
    insert_work_item(&pool, &stale_2).await.unwrap();
    insert_work_item(&pool, &fresh_3).await.unwrap();
    insert_work_item(&pool, &done_4).await.unwrap();
    mark_work_item_done(&pool, "done-4", Some("/receipts/done-4.json"))
        .await
        .unwrap();

    let first_run_processed = runner.step().await.unwrap();
    assert_eq!(first_run_processed, 2);
    assert_eq!(sink.recorded_payloads().len(), 2);

    let second_run_processed = runner.step().await.unwrap();
    assert_eq!(second_run_processed, 0);
    assert_eq!(sink.recorded_payloads().len(), 2);
}

#[tokio::test]
async fn heartbeat_discord_rest_payload_contract_matches_serenity_spec() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let now = Utc.with_ymd_and_hms(2026, 10, 7, 21, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(now));
    let checker = Arc::new(MockLivenessChecker::new());
    checker.set_status("worker-alpha", DelegateLiveness::Alive);
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        enabled: true,
        channel_id: Some("11223344".to_string()),
        interval: Duration::from_secs(60),
        threshold: Duration::from_secs(900),
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let item = WorkItem::new(
        "wi-contract-check",
        "Contract Verification Task",
        Some("99887766".to_string()),
        Some("worker-alpha".to_string()),
        (now - chrono::Duration::minutes(30)).to_rfc3339(),
    );
    insert_work_item(&pool, &item).await.unwrap();

    let processed = runner.step().await.unwrap();
    assert_eq!(processed, 1);

    let payloads = sink.recorded_payloads();
    assert_eq!(payloads.len(), 1);

    let payload: &CapturedDiscordPayload = &payloads[0];
    assert_eq!(payload.method, "POST");
    assert_eq!(payload.path, "/api/v10/channels/99887766/messages");
    assert!(payload.body.is_object());
    assert!(payload.body["content"].is_string());
    assert_eq!(payload.body["allowed_mentions"]["all_users"], true);
    assert_eq!(payload.body["allowed_mentions"]["everyone"], false);
    assert_eq!(payload.body["allowed_mentions"]["all_roles"], false);
    assert_eq!(payload.body["allowed_mentions"]["replied_user"], true);
}

#[tokio::test]
async fn heartbeat_disabled_by_config_skips_processing() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let now = Utc::now();
    let clock = Arc::new(MockClock::new(now));
    let checker = Arc::new(MockLivenessChecker::new());
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        enabled: false,
        channel_id: Some("11223344".to_string()),
        interval: Duration::from_secs(60),
        threshold: Duration::from_secs(900),
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let item = WorkItem::new(
        "wi-disabled",
        "Disabled Task",
        Some("11223344".to_string()),
        None,
        (now - chrono::Duration::minutes(60)).to_rfc3339(),
    );
    insert_work_item(&pool, &item).await.unwrap();

    let processed = runner.step().await.unwrap();
    assert_eq!(processed, 0);
    assert_eq!(sink.recorded_payloads().len(), 0);
}

#[tokio::test]
async fn heartbeat_scans_tasks_with_status_running() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();

    let now = Utc.with_ymd_and_hms(2026, 10, 7, 15, 0, 0).unwrap();
    let clock = Arc::new(MockClock::new(now));
    let checker = Arc::new(MockLivenessChecker::new());
    checker.set_status("worker-running", DelegateLiveness::Alive);
    let sink = Arc::new(RecordingHeartbeatSink::new());

    let config = WorkHeartbeatConfig {
        enabled: true,
        channel_id: Some("running-chan".to_string()),
        interval: Duration::from_secs(60),
        threshold: Duration::from_secs(900),
    };

    let runner = WorkHeartbeatRunner::new(
        pool.clone(),
        config,
        clock.clone(),
        checker.clone(),
        sink.clone(),
    );

    let mut item = WorkItem::new(
        "item-running-1",
        "Active Worker Job",
        Some("running-chan".to_string()),
        Some("worker-running".to_string()),
        (now - chrono::Duration::minutes(20)).to_rfc3339(),
    );
    item.status = "running".to_string();
    insert_work_item(&pool, &item).await.unwrap();

    let processed = runner.step().await.unwrap();
    assert_eq!(processed, 1);
    assert_eq!(sink.recorded_payloads().len(), 1);

    let updated = get_work_item(&pool, "item-running-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.last_progress_at, clock.now().to_rfc3339());
}

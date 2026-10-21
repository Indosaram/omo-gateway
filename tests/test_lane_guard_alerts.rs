use omon_gateway::lanes::guard::{
    retry_queued, GuardState, GuardThresholds, GuardWatcher, HostSample,
};
use omon_gateway::storage::{get_lane, insert_lane, take_undelivered_events, Database, Lane};

fn thresholds() -> GuardThresholds {
    GuardThresholds {
        load_alert: 150.0,
        load_ok: 80.0,
        mem_free_min_pct: 20,
        disk_free_min_gb: 50,
        lanes_max: 1,
    }
}

async fn queued(database: &Database, key: &str) {
    let lane = Lane::new(
        key,
        "Queued",
        "lead",
        "/tmp",
        "/tmp/brief",
        format!("lane:{key}"),
        "file done.txt",
    );
    insert_lane(database.pool(), &lane).await.unwrap();
}

#[tokio::test]
async fn events_are_emitted_once_per_transition() {
    // Given a fresh watcher and a real scratch database.
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let mut watcher = GuardWatcher::new(thresholds());
    // When repeated samples cross ALERT and CLEAR with hysteresis.
    for load in [10.0, 300.0, 300.0, 100.0, 10.0, 10.0] {
        watcher
            .enqueue_transition(database.pool(), "lead", &HostSample::new(load, 50, 100, 0))
            .await
            .unwrap();
    }
    // Then only the two transitions are durably queued for the lead.
    let events = take_undelivered_events(database.pool(), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.kind == "guard"
        && event.lead_session_key == "lead"
        && event.lane_key.is_none()));
    assert!(events[0].body.starts_with("ALERT:"));
    assert_eq!(events[1].body, "CLEAR");
}

#[tokio::test]
async fn failed_event_write_preserves_watcher_state() {
    // Given a watcher whose event destination cannot accept writes.
    let database = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::query("DROP TABLE lane_events")
        .execute(database.pool())
        .await
        .unwrap();
    let mut watcher = GuardWatcher::new(thresholds());
    // When an alert cannot be persisted.
    let result = watcher
        .enqueue_transition(database.pool(), "lead", &HostSample::new(300.0, 50, 100, 0))
        .await;
    // Then the transition is still available for a later retry.
    assert!(result.is_err());
    assert_eq!(watcher.state(), GuardState::Clear);
}

#[tokio::test]
async fn queued_lane_launches_when_guard_recovers_even_at_high_cpu() {
    // Given a lane held by low disk space.
    let database = Database::connect("sqlite::memory:").await.unwrap();
    queued(&database, "queued-one").await;
    assert!(retry_queued(
        database.pool(),
        &HostSample::new(300.0, 50, 10, 0),
        &thresholds()
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        get_lane(database.pool(), "queued-one")
            .await
            .unwrap()
            .unwrap()
            .status,
        "queued"
    );
    // When disk recovers but CPU remains above its alert threshold.
    let launched = retry_queued(
        database.pool(),
        &HostSample::new(300.0, 50, 100, 0),
        &thresholds(),
    )
    .await
    .unwrap();
    // Then the lane is working with exactly one launch event and history row.
    assert_eq!(launched, ["queued-one"]);
    assert_eq!(
        get_lane(database.pool(), "queued-one")
            .await
            .unwrap()
            .unwrap()
            .status,
        "working"
    );
    let events = take_undelivered_events(database.pool(), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "launched");
    assert_eq!(events[0].lane_key.as_deref(), Some("queued-one"));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_history")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn retries_recount_active_lanes_and_do_not_duplicate_launches() {
    // Given two queued lanes and only one available slot.
    let database = Database::connect("sqlite::memory:").await.unwrap();
    queued(&database, "queued-one").await;
    queued(&database, "queued-two").await;
    // When repeated retries use a stale sample reporting zero active lanes.
    let host = HostSample::new(10.0, 50, 100, 0);
    let first = retry_queued(database.pool(), &host, &thresholds())
        .await
        .unwrap();
    let second = retry_queued(database.pool(), &host, &thresholds())
        .await
        .unwrap();
    // Then the DB count prevents overlaunch and duplicate events.
    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert_eq!(
        take_undelivered_events(database.pool(), None)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn launch_event_failure_rolls_back_lane_and_history() {
    // Given a queued lane with an unavailable event table.
    let database = Database::connect("sqlite::memory:").await.unwrap();
    queued(&database, "queued-one").await;
    sqlx::query("DROP TABLE lane_events")
        .execute(database.pool())
        .await
        .unwrap();
    // When promotion cannot write its launch event.
    let result = retry_queued(
        database.pool(),
        &HostSample::new(10.0, 50, 100, 0),
        &thresholds(),
    )
    .await;
    // Then no partial launch is committed.
    assert!(result.is_err());
    assert_eq!(
        get_lane(database.pool(), "queued-one")
            .await
            .unwrap()
            .unwrap()
            .status,
        "queued"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM lane_history")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn lane_saturation_alert_once_then_clear() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let mut watcher = GuardWatcher::new(thresholds());

    // lanes_max is 1 in thresholds().
    // Active lanes (1) >= maximum (1) -> produces ALERT event with reason.
    watcher
        .enqueue_transition(database.pool(), "lead", &HostSample::new(10.0, 50, 100, 1))
        .await
        .unwrap();

    let events = take_undelivered_events(database.pool(), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "guard");
    assert!(
        events[0].body.contains("active lanes (1) >= maximum (1)"),
        "expected lane alert reason in body, got: {}",
        events[0].body
    );

    // A repeat sample adds none (deduplication). Taking events does not mark them
    // delivered, so the queue still holds exactly the first ALERT.
    watcher
        .enqueue_transition(database.pool(), "lead", &HostSample::new(10.0, 50, 100, 1))
        .await
        .unwrap();
    let events = take_undelivered_events(database.pool(), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);

    // Active lanes drop to 0 -> produces a CLEAR event.
    watcher
        .enqueue_transition(database.pool(), "lead", &HostSample::new(10.0, 50, 100, 0))
        .await
        .unwrap();
    let events = take_undelivered_events(database.pool(), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].kind, "guard");
    assert_eq!(events[1].body, "CLEAR");
}

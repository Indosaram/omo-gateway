use std::collections::HashSet;
use std::sync::Arc;

use omon_gateway::storage::{
    append_transcript, enqueue_lane_event, get_lane, get_lane_by_thread, get_lane_history,
    insert_lane, list_lanes, mark_events_delivered, read_transcript, set_lane_fields,
    take_undelivered_events, transition_lane, Database, Lane, LaneFieldsUpdate,
};
use sqlx::Row;

#[tokio::test]
async fn migration_0031_creates_four_tables_and_indexes() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let table_rows = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('lanes', 'lane_history', 'lane_events', 'lane_transcript')",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let table_names: HashSet<String> = table_rows
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(table_names.contains("lanes"), "missing lanes table");
    assert!(
        table_names.contains("lane_history"),
        "missing lane_history table"
    );
    assert!(
        table_names.contains("lane_events"),
        "missing lane_events table"
    );
    assert!(
        table_names.contains("lane_transcript"),
        "missing lane_transcript table"
    );
    assert_eq!(table_names.len(), 4, "expected exactly four lane tables");

    let lane_columns: HashSet<String> = sqlx::query("PRAGMA table_info(lanes)")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    let expected_lane_columns = [
        "key",
        "title",
        "lead_session_key",
        "thread_channel_id",
        "parent_channel_id",
        "status_message_id",
        "cwd",
        "brief_path",
        "session_key",
        "omo_thread_id",
        "model",
        "status",
        "done_signals",
        "claim_evidence",
        "claim_at",
        "objection",
        "turns_run",
        "last_turn_outcome",
        "last_report",
        "last_activity_at",
        "stale_posted_at",
        "created_at",
        "updated_at",
    ];
    for col in expected_lane_columns {
        assert!(lane_columns.contains(col), "missing column {col} in lanes");
    }

    let history_columns: HashSet<String> = sqlx::query("PRAGMA table_info(lane_history)")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    for col in ["id", "lane_key", "at", "from_status", "to_status", "reason"] {
        assert!(
            history_columns.contains(col),
            "missing column {col} in lane_history"
        );
    }

    let event_columns: HashSet<String> = sqlx::query("PRAGMA table_info(lane_events)")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    for col in [
        "id",
        "lane_key",
        "lead_session_key",
        "kind",
        "body",
        "created_at",
        "delivered_at",
    ] {
        assert!(
            event_columns.contains(col),
            "missing column {col} in lane_events"
        );
    }

    let transcript_columns: HashSet<String> = sqlx::query("PRAGMA table_info(lane_transcript)")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    for col in ["id", "lane_key", "at", "role", "text"] {
        assert!(
            transcript_columns.contains(col),
            "missing column {col} in lane_transcript"
        );
    }

    let index_rows = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name IN ('idx_lanes_status', 'idx_lane_events_delivered_lead', 'idx_lane_transcript_lane_id')",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let index_names: HashSet<String> = index_rows
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(
        index_names.contains("idx_lanes_status"),
        "missing idx_lanes_status"
    );
    assert!(
        index_names.contains("idx_lane_events_delivered_lead"),
        "missing idx_lane_events_delivered_lead"
    );
    assert!(
        index_names.contains("idx_lane_transcript_lane_id"),
        "missing idx_lane_transcript_lane_id"
    );
}

#[tokio::test]
async fn key_bad_key_rejected_by_check_constraint() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let bad_lane = Lane::new(
        "Bad Key",
        "Test title",
        "lead-1",
        "/tmp",
        "/tmp/brief.md",
        "session-1",
        "file done.txt",
    );
    let result = insert_lane(pool, &bad_lane).await;
    assert!(
        result.is_err(),
        "expected 'Bad Key' to be rejected by CHECK constraint"
    );

    for bad_key in [
        "bad_key",
        "BadKey",
        "bad.key",
        "bad/key",
        "",
        "this-key-is-way-too-long-and-exceeds-the-maximum-length-of-48-characters",
    ] {
        let mut lane = bad_lane.clone();
        lane.key = bad_key.to_string();
        let err = insert_lane(pool, &lane).await;
        assert!(
            err.is_err(),
            "expected key '{bad_key}' to be rejected by CHECK constraint"
        );
    }

    let mut good_lane = bad_lane.clone();
    good_lane.key = "good-lane-123".to_string();
    let ok = insert_lane(pool, &good_lane).await;
    assert!(
        ok.is_ok(),
        "expected valid key 'good-lane-123' to succeed: {:?}",
        ok.err()
    );
}

#[tokio::test]
async fn concurrent_transitions_exactly_one_succeeds_and_history_recorded() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = Arc::new(database.pool().clone());

    let mut lane = Lane::new(
        "concurrent-lane",
        "Concurrent Title",
        "lead-1",
        "/tmp/cwd",
        "/tmp/brief.md",
        "session-1",
        "file done.txt",
    );
    lane.status = "working".to_string();
    insert_lane(&*pool, &lane)
        .await
        .expect("insert must succeed");

    let pool_a = Arc::clone(&pool);
    let pool_b = Arc::clone(&pool);

    let task_a = tokio::spawn(async move {
        transition_lane(
            &pool_a,
            "concurrent-lane",
            &["working"],
            "blocked",
            Some("task a reason"),
        )
        .await
    });

    let task_b = tokio::spawn(async move {
        transition_lane(
            &pool_b,
            "concurrent-lane",
            &["working"],
            "blocked",
            Some("task b reason"),
        )
        .await
    });

    let (res_a, res_b) = tokio::join!(task_a, task_b);
    let ok_a = res_a.expect("task a join").expect("task a query");
    let ok_b = res_b.expect("task b join").expect("task b query");

    assert!(
        (ok_a && !ok_b) || (!ok_a && ok_b),
        "expected exactly one transition to succeed, got a: {ok_a}, b: {ok_b}"
    );

    let fetched = get_lane(&pool, "concurrent-lane")
        .await
        .unwrap()
        .expect("lane must exist");
    assert_eq!(fetched.status, "blocked");

    let history = get_lane_history(&pool, "concurrent-lane")
        .await
        .expect("history query must succeed");
    assert_eq!(history.len(), 1, "expected exactly 1 history row");
    assert_eq!(history[0].lane_key, "concurrent-lane");
    assert_eq!(history[0].from_status.as_deref(), Some("working"));
    assert_eq!(history[0].to_status, "blocked");
}

#[tokio::test]
async fn lane_crud_and_status_filtering() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let mut lane1 = Lane::new(
        "lane-1",
        "Lane 1",
        "lead-1",
        "/tmp/1",
        "/tmp/1/brief.md",
        "sess-1",
        "file 1.txt",
    );
    lane1.status = "working".to_string();
    lane1.thread_channel_id = Some("thread-101".to_string());
    insert_lane(pool, &lane1).await.unwrap();

    let mut lane2 = Lane::new(
        "lane-2",
        "Lane 2",
        "lead-2",
        "/tmp/2",
        "/tmp/2/brief.md",
        "sess-2",
        "file 2.txt",
    );
    lane2.status = "queued".to_string();
    insert_lane(pool, &lane2).await.unwrap();

    let fetched = get_lane(pool, "lane-1").await.unwrap().unwrap();
    assert_eq!(fetched.title, "Lane 1");
    assert_eq!(fetched.status, "working");

    let by_thread = get_lane_by_thread(pool, "thread-101")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_thread.key, "lane-1");

    let by_missing_thread = get_lane_by_thread(pool, "thread-999").await.unwrap();
    assert!(by_missing_thread.is_none());

    let all = list_lanes(pool, &[] as &[&str]).await.unwrap();
    assert_eq!(all.len(), 2);

    let queued = list_lanes(pool, &["queued"]).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].key, "lane-2");

    let active = list_lanes(pool, &["working", "blocked"]).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].key, "lane-1");

    let update = LaneFieldsUpdate {
        turns_run: Some(3),
        last_report: Some("milestone 1".to_string()),
        omo_thread_id: Some("omo-thread-xyz".to_string()),
        ..Default::default()
    };
    let updated = set_lane_fields(pool, "lane-1", &update).await.unwrap();
    assert!(updated);

    let fetched_updated = get_lane(pool, "lane-1").await.unwrap().unwrap();
    assert_eq!(fetched_updated.turns_run, 3);
    assert_eq!(fetched_updated.last_report.as_deref(), Some("milestone 1"));
    assert_eq!(
        fetched_updated.omo_thread_id.as_deref(),
        Some("omo-thread-xyz")
    );

    let clear_update = LaneFieldsUpdate {
        clear_omo_thread_id: true,
        ..Default::default()
    };
    let cleared = set_lane_fields(pool, "lane-1", &clear_update)
        .await
        .unwrap();
    assert!(cleared);

    let fetched_cleared = get_lane(pool, "lane-1").await.unwrap().unwrap();
    assert!(fetched_cleared.omo_thread_id.is_none());

    let missed = transition_lane(pool, "lane-1", &["queued"], "blocked", "this should fail")
        .await
        .unwrap();
    assert!(!missed);

    let history = get_lane_history(pool, "lane-1").await.unwrap();
    assert_eq!(history.len(), 0);
}

#[tokio::test]
async fn transcript_append_and_read_ordering() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let lane = Lane::new(
        "transcript-lane",
        "Transcript Test",
        "lead-1",
        "/tmp",
        "/tmp/brief.md",
        "sess-1",
        "file done.txt",
    );
    insert_lane(pool, &lane).await.unwrap();

    let id1 = append_transcript(pool, "transcript-lane", "user", "turn 1 request")
        .await
        .unwrap();
    let id2 = append_transcript(pool, "transcript-lane", "agent", "turn 1 response")
        .await
        .unwrap();
    let id3 = append_transcript(pool, "transcript-lane", "user", "turn 2 request")
        .await
        .unwrap();
    assert!(id1 < id2 && id2 < id3);

    let all = read_transcript(pool, "transcript-lane", None)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].role, "user");
    assert_eq!(all[0].text, "turn 1 request");
    assert_eq!(all[1].role, "agent");
    assert_eq!(all[1].text, "turn 1 response");
    assert_eq!(all[2].role, "user");
    assert_eq!(all[2].text, "turn 2 request");

    let last_two = read_transcript(pool, "transcript-lane", Some(2))
        .await
        .unwrap();
    assert_eq!(last_two.len(), 2);
    assert_eq!(last_two[0].role, "agent");
    assert_eq!(last_two[0].text, "turn 1 response");
    assert_eq!(last_two[1].role, "user");
    assert_eq!(last_two[1].text, "turn 2 request");
}

#[tokio::test]
async fn lane_events_queue_take_and_mark_delivered() {
    let database = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory db must connect and migrate");
    let pool = database.pool();

    let ev1 = enqueue_lane_event(
        pool,
        Some("lane-a"),
        "lead-session-1",
        "blocked",
        "need auth",
    )
    .await
    .unwrap();
    let ev2 = enqueue_lane_event(
        pool,
        Some("lane-a"),
        "lead-session-1",
        "question",
        "choice 1 or 2?",
    )
    .await
    .unwrap();
    let ev3 = enqueue_lane_event(pool, None, "lead-session-1", "guard", "host load clear")
        .await
        .unwrap();
    assert!(ev1 < ev2 && ev2 < ev3);

    let first_two = take_undelivered_events(pool, Some(2)).await.unwrap();
    assert_eq!(first_two.len(), 2);
    assert_eq!(first_two[0].kind, "blocked");
    assert_eq!(first_two[1].kind, "question");
    assert!(first_two[0].delivered_at.is_none());

    let delivered_count = mark_events_delivered(pool, &[first_two[0].id, first_two[1].id])
        .await
        .unwrap();
    assert_eq!(delivered_count, 2);

    let remaining = take_undelivered_events(pool, None).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].kind, "guard");
    assert_eq!(remaining[0].id, ev3);

    let mark_rest = mark_events_delivered(pool, &[remaining[0].id])
        .await
        .unwrap();
    assert_eq!(mark_rest, 1);

    let empty = take_undelivered_events(pool, None).await.unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn real_data_migration_preserves_work_items_and_adds_lanes_tables() {
    let live_db = std::path::Path::new("/Users/indo/code/project/omon-gateway/omon_gateway.db");
    if !live_db.exists() {
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let target_db = dir.path().join("omon_gateway.db");
    std::fs::copy(live_db, &target_db).expect("copy live db");

    let live_wal =
        std::path::Path::new("/Users/indo/code/project/omon-gateway/omon_gateway.db-wal");
    if live_wal.exists() {
        let _ = std::fs::copy(live_wal, dir.path().join("omon_gateway.db-wal"));
    }
    let live_shm =
        std::path::Path::new("/Users/indo/code/project/omon-gateway/omon_gateway.db-shm");
    if live_shm.exists() {
        let _ = std::fs::copy(live_shm, dir.path().join("omon_gateway.db-shm"));
    }

    let db_url = format!("sqlite://{}", target_db.to_str().unwrap());
    let db = Database::connect(&db_url).await.expect("connect to copy");
    let pool = db.pool();

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM work_items")
        .fetch_one(pool)
        .await
        .expect("query work_items count");
    assert_eq!(count.0, 0, "work_items count must remain 0");

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('lanes', 'lane_history', 'lane_events', 'lane_transcript')",
    )
    .fetch_all(pool)
    .await
    .expect("query tables");
    assert_eq!(tables.len(), 4, "expected all four lane tables to exist");

    db.close().await;
}

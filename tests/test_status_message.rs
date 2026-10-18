use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use omon_gateway::report::status::{StatusDriver, TurnStart};
use omon_gateway::{Database, OutboundAction, OutboundDispatcher, Result, SessionKey};
use parking_lot::Mutex;
use sqlx::SqlitePool;

#[derive(Default)]
struct RecordingDispatcher {
    sends: Mutex<Vec<String>>,
    edits: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl OutboundDispatcher for RecordingDispatcher {
    async fn dispatch(&self, action: OutboundAction) -> Result<()> {
        if let OutboundAction::EditMessage {
            platform_message_id,
            content,
            ..
        } = action
        {
            self.edits.lock().push((platform_message_id, content));
        }
        Ok(())
    }

    async fn post_status(
        &self,
        _session: &SessionKey,
        content: String,
        _reply_to: Option<String>,
    ) -> Result<Option<String>> {
        let mut sends = self.sends.lock();
        sends.push(content);
        Ok(Some(format!("msg-{}", sends.len())))
    }
}

fn session() -> SessionKey {
    SessionKey::new("slack", None::<String>, "D1", None::<String>, "U1")
}

fn t0_kst_10am() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 17, 1, 0, 0).unwrap()
}

fn at(seconds: i64) -> DateTime<Utc> {
    t0_kst_10am() + Duration::seconds(seconds)
}

async fn pool() -> SqlitePool {
    Database::connect("sqlite::memory:")
        .await
        .unwrap()
        .pool()
        .clone()
}

fn turn(started_at: DateTime<Utc>) -> TurnStart {
    TurnStart {
        session: session(),
        event_id: "event-1".into(),
        owner_text: "배포 상태 확인해줘".into(),
        reply_to: Some("1699.0001".into()),
        owner_message_at: Some(t0_kst_10am()),
        started_at,
    }
}

async fn stored(pool: &SqlitePool) -> Vec<(String, String)> {
    sqlx::query_as("SELECT platform_message_id, state FROM turn_status_messages")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn short_turn_sends_nothing() {
    let pool = pool().await;
    let recorder = Arc::new(RecordingDispatcher::default());
    let mut driver = StatusDriver::start(pool.clone(), recorder.clone(), turn(t0_kst_10am())).await;

    assert_eq!(driver.next_due(), at(180));
    driver.tick(at(100)).await;
    driver.finish(at(100), Ok(())).await;

    assert!(recorder.sends.lock().is_empty());
    assert!(recorder.edits.lock().is_empty());
    assert!(stored(&pool).await.is_empty());
}

#[tokio::test]
async fn long_turn_posts_once_edits_once_and_finalizes() {
    let pool = pool().await;
    let recorder = Arc::new(RecordingDispatcher::default());
    let mut driver = StatusDriver::start(pool.clone(), recorder.clone(), turn(t0_kst_10am())).await;

    driver.tick(at(180)).await;
    assert_eq!(driver.next_due(), at(360));
    driver.tick(at(360)).await;
    driver.finish(at(400), Ok(())).await;

    let sends = recorder.sends.lock().clone();
    assert_eq!(sends.len(), 1);
    assert!(sends[0].starts_with("작업 중: 배포 상태 확인해줘"));
    let edits = recorder.edits.lock().clone();
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|(id, _)| id == "msg-1"));
    assert!(edits[0].1.contains("6분"));
    assert_eq!(edits[1].1, "완료 · 6분");
    assert_eq!(stored(&pool).await, vec![("msg-1".into(), "done".into())]);
}

#[tokio::test]
async fn restart_mid_turn_reuses_stored_message() {
    let pool = pool().await;
    let recorder = Arc::new(RecordingDispatcher::default());
    let mut first = StatusDriver::start(pool.clone(), recorder.clone(), turn(t0_kst_10am())).await;
    first.tick(at(180)).await;
    first.abandon();
    assert_eq!(
        stored(&pool).await,
        vec![("msg-1".into(), "running".into())]
    );

    let mut resumed = StatusDriver::start(pool.clone(), recorder.clone(), turn(at(200))).await;
    assert_eq!(resumed.message_id(), Some("msg-1"));
    assert_eq!(resumed.next_due(), at(360));
    resumed.tick(at(360)).await;
    resumed.finish(at(420), Err("backend error".into())).await;

    assert_eq!(recorder.sends.lock().len(), 1);
    let edits = recorder.edits.lock().clone();
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|(id, _)| id == "msg-1"));
    assert_eq!(edits[1].1, "실패 · backend error");
    assert_eq!(stored(&pool).await, vec![("msg-1".into(), "failed".into())]);
}

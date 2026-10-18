//! Context pack for owner turns, against a loopback fake of the Slack Web API.
use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Json, Router};
use omon_gateway::context::{
    build_pack, wrap_prompt, ContextPackSettings, LedgerPrecedents, NoPrecedents, ThreadReader,
    MAX_PACK_CHARS, MAX_PRECEDENTS,
};
use omon_gateway::slack::SlackApi;
use omon_gateway::{Database, InboundEvent, SessionKey};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const OWNER: &str = "U04OWNER";
const OTHER: &str = "U0OTHER";
const BOT: &str = "U0BOT";
const TEAM: &str = "T1";
const CHANNEL: &str = "C0QA";
const ROOT: &str = "1700000000.000100";
const NEW_TS: &str = "1700000999.000000";

struct FakeSlack {
    replies: Vec<Value>,
}

async fn fake_slack(State(fake): State<Arc<FakeSlack>>) -> impl IntoResponse {
    Json(json!({ "ok": true, "messages": fake.replies, "has_more": false }))
}

async fn serve(replies: Vec<Value>) -> (SlackApi, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .fallback(fake_slack)
        .with_state(Arc::new(FakeSlack { replies }));
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let api = SlackApi::with_base_url("xoxb-test".into(), format!("http://{address}"));
    (api, task)
}

fn reply_ts(index: usize) -> String {
    format!("1700000{:03}.000000", index + 1)
}

fn remote_thread(count: usize, text_len: usize) -> Vec<Value> {
    (0..count)
        .rev()
        .map(|i| {
            let user = if i % 2 == 0 { OWNER } else { OTHER };
            json!({
                "type": "message",
                "user": user,
                "ts": reply_ts(i),
                "thread_ts": ROOT,
                "text": format!("msg-{i:02} {}", "x".repeat(text_len)),
            })
        })
        .collect()
}

fn thread_key() -> SessionKey {
    SessionKey::new("slack", Some(TEAM), CHANNEL, Some(ROOT), "").with_bot_id(BOT)
}

fn settings(enabled: bool) -> ContextPackSettings {
    ContextPackSettings {
        enabled,
        slack_owner: OWNER.into(),
        discord_owner: "414011632306618368".into(),
    }
}

fn owner_event(content: &str) -> InboundEvent {
    InboundEvent::message(thread_key(), NEW_TS, content).with_author_id(OWNER)
}

async fn database() -> (Database, SqlitePool) {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool().clone();
    let key = thread_key();
    sqlx::query(
        "INSERT INTO sessions (session_key, platform, guild_id, channel_id, thread_id, user_id)
         VALUES (?, 'slack', ?, ?, ?, '')",
    )
    .bind(key.storage_key())
    .bind(TEAM)
    .bind(CHANNEL)
    .bind(ROOT)
    .execute(&pool)
    .await
    .unwrap();
    (database, pool)
}

async fn store_message(pool: &SqlitePool, id: &str, content: &str, platform_message_id: &str) {
    sqlx::query(
        "INSERT INTO messages (id, session_key, role, content, platform_message_id)
         VALUES (?, ?, 'user', ?, ?)",
    )
    .bind(id)
    .bind(thread_key().storage_key())
    .bind(content)
    .bind(platform_message_id)
    .execute(pool)
    .await
    .unwrap();
}

fn thread_lines(pack: &str) -> Vec<&str> {
    pack.lines()
        .skip_while(|line| !line.starts_with("Thread (not seen before"))
        .skip(1)
        .take_while(|line| line.starts_with("- "))
        .filter(|line| *line != "- none")
        .collect()
}

#[tokio::test]
async fn a_unseen_thread_messages_are_listed_oldest_first() {
    // Given: 25 remote replies, the first 10 already stored for this session.
    let (_db, pool) = database().await;
    for i in 0..10 {
        store_message(&pool, &format!("m{i}"), "stored", &reply_ts(i)).await;
    }
    let (api, server) = serve(remote_thread(25, 5)).await;

    // When
    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &owner_event("what now?"),
        &settings(true),
        &NoPrecedents,
    )
    .await
    .expect("owner thread message gets a pack");
    server.abort();

    // Then: the 15 unseen messages, oldest first, and all three sections present.
    let lines = thread_lines(&pack);
    assert_eq!(lines.len(), 15, "{pack}");
    for (offset, line) in lines.iter().enumerate() {
        assert!(
            line.contains(&format!("msg-{:02}", offset + 10)),
            "{line} at {offset}"
        );
    }
    assert!(pack.contains("Lanes in this thread:"));
    assert!(pack.contains("Precedents:"));
}

#[tokio::test]
async fn a_more_than_twenty_unseen_keeps_the_latest_twenty() {
    let (_db, pool) = database().await;
    let (api, server) = serve(remote_thread(30, 5)).await;

    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &owner_event("hi"),
        &settings(true),
        &NoPrecedents,
    )
    .await
    .unwrap();
    server.abort();

    let lines = thread_lines(&pack);
    assert_eq!(lines.len(), 20, "{pack}");
    assert!(lines[0].contains("msg-10"));
    assert!(lines[19].contains("msg-29"));
}

#[tokio::test]
async fn b_pack_is_capped_at_4000_chars() {
    let (_db, pool) = database().await;
    let (api, server) = serve(remote_thread(25, 1000)).await;

    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &owner_event("hi"),
        &settings(true),
        &NoPrecedents,
    )
    .await
    .unwrap();
    server.abort();

    assert!(pack.chars().count() <= MAX_PACK_CHARS, "{}", pack.len());
    assert!(!thread_lines(&pack).is_empty());
    assert!(pack.contains("Precedents:"));
}

#[tokio::test]
async fn c_non_owner_gets_no_pack() {
    let (_db, pool) = database().await;
    let (api, server) = serve(remote_thread(5, 5)).await;
    let reader = Some(&api as &dyn ThreadReader);

    let other = InboundEvent::message(thread_key(), NEW_TS, "hi").with_author_id(OTHER);
    let anonymous = InboundEvent::message(thread_key(), NEW_TS, "hi");
    let mut system = owner_event("hi");
    system.origin = omon_gateway::EventOrigin::System;
    let other_dm = InboundEvent::message(
        SessionKey::new("slack", None::<String>, "D0OTHER", None::<String>, OTHER).with_bot_id(BOT),
        NEW_TS,
        "hi",
    )
    .with_author_id(OWNER);
    let cron = InboundEvent::message(
        SessionKey::new("cron", None::<String>, "cron:x", None::<String>, "cron:x"),
        "cron:x",
        "run",
    )
    .with_author_id(OWNER);

    for event in [other, anonymous, system, other_dm, cron] {
        let pack = build_pack(&pool, reader, &event, &settings(true), &NoPrecedents).await;
        assert_eq!(pack, None, "{:?}", event.session);
    }
    server.abort();
}

#[tokio::test]
async fn d_stored_message_content_is_unchanged() {
    let (_db, pool) = database().await;
    let original = "please check the deploy";
    store_message(&pool, "owner-msg", original, NEW_TS).await;
    let (api, server) = serve(remote_thread(5, 5)).await;
    let event = owner_event(original);

    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &event,
        &settings(true),
        &NoPrecedents,
    )
    .await
    .unwrap();
    server.abort();
    let prompt = wrap_prompt(&pack, original);

    let stored: Vec<String> = sqlx::query_scalar("SELECT content FROM messages WHERE id = ?")
        .bind("owner-msg")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(stored, vec![original.to_owned()]);
    assert_eq!(event.content, original);
    assert!(prompt.starts_with("[CONTEXT]\n"));
    assert!(prompt.ends_with(&format!("[/CONTEXT]\n{original}")));
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE content LIKE '%[CONTEXT]%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn e_disabled_flag_gives_no_pack() {
    let (_db, pool) = database().await;
    let (api, server) = serve(remote_thread(5, 5)).await;

    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &owner_event("hi"),
        &settings(false),
        &NoPrecedents,
    )
    .await;
    server.abort();

    assert_eq!(pack, None);
}

#[tokio::test]
async fn f_at_most_ten_matching_precedents_appear_in_context_pack() {
    let (_db, pool) = database().await;
    let key = thread_key();
    let thread_ref = format!("{CHANNEL}:{ROOT}");

    let mut lane = omon_gateway::storage::Lane::new(
        "lane-alpha",
        "ship alpha",
        key.storage_key(),
        "/Users/indo/code/project/mhc",
        "/tmp/brief.md",
        "lane:lane-alpha",
        "[]",
    );
    lane.status = "working".into();
    lane.thread_channel_id = Some(thread_ref.clone());
    omon_gateway::storage::insert_lane(&pool, &lane)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO lane_thread_bindings (lane_key, bot_id, bound_at)
         VALUES ('lane-alpha', ?, CURRENT_TIMESTAMP)",
    )
    .bind(BOT)
    .execute(&pool)
    .await
    .unwrap();

    for i in 0..14 {
        let sig = format!("vcs_remote:git push-{i}:/Users/indo/code/project/mhc");
        sqlx::query(
            "INSERT INTO action_precedents (signature, effect, target_scope, decision, scope, approvals, declines, origin)
             VALUES (?, 'vcs_remote', '/Users/indo/code/project/mhc', 'allow', 'exact', 1, 0, 'owner')",
        )
        .bind(sig)
        .execute(&pool)
        .await
        .unwrap();
    }

    sqlx::query(
        "INSERT INTO action_precedents (signature, effect, target_scope, decision, scope, approvals, declines, origin)
         VALUES ('vcs_remote:git push:/Users/indo/code/project/unrelated-repo', 'vcs_remote', '/Users/indo/code/project/unrelated-repo', 'allow', 'exact', 1, 0, 'owner')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO action_precedents (signature, effect, target_scope, decision, scope, approvals, declines, origin, revoked_at)
         VALUES ('vcs_remote:git push-revoked:/Users/indo/code/project/mhc', 'vcs_remote', '/Users/indo/code/project/mhc', 'allow', 'exact', 1, 0, 'owner', CURRENT_TIMESTAMP)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let (api, server) = serve(remote_thread(2, 5)).await;
    let pack = build_pack(
        &pool,
        Some(&api as &dyn ThreadReader),
        &owner_event("status?"),
        &settings(true),
        &LedgerPrecedents,
    )
    .await
    .expect("pack built with matching precedents");
    server.abort();

    let prec_lines: Vec<&str> = pack
        .lines()
        .skip_while(|line| !line.starts_with("Precedents:"))
        .skip(1)
        .take_while(|line| line.starts_with("- "))
        .filter(|line| *line != "- none")
        .collect();

    assert_eq!(prec_lines.len(), MAX_PRECEDENTS, "{pack}");
    for line in &prec_lines {
        assert!(line.contains("/Users/indo/code/project/mhc"), "{line}");
        assert!(!line.contains("unrelated-repo"), "{line}");
        assert!(!line.contains("revoked"), "{line}");
    }
}

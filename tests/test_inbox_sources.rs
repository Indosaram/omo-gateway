use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use omon_gateway::inbox::sources::{
    discord::DiscordSource, imessage::IMessageSource, kakao::KakaoSource, mail::MailSource,
};
use omon_gateway::inbox::{
    collect_all, prefilter::PrefilterRules, InboxItem, InboxSource, InboxSourceKind,
};
use omon_gateway::storage::Database;
use omon_gateway::{OmonError, Result};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "mail" => include_str!("fixtures/inbox/mail.json"),
        "kakao-rooms" => include_str!("fixtures/inbox/kakao-rooms.json"),
        "kakao-messages" => include_str!("fixtures/inbox/kakao-messages.json"),
        "imessage-chats" => include_str!("fixtures/inbox/imessage-chats.json"),
        "imessage-history" => include_str!("fixtures/inbox/imessage-history.json"),
        "discord" => include_str!("fixtures/inbox/discord.json"),
        "captured-mail-empty" => include_str!("fixtures/inbox/captured-task23/mail-empty.json"),
        "captured-imessage-chats" => {
            include_str!("fixtures/inbox/captured-task23/imessage-chats.json")
        }
        "captured-imessage-history" => {
            include_str!("fixtures/inbox/captured-task23/imessage-history.json")
        }
        "captured-discord" => {
            include_str!("fixtures/inbox/captured-task23/discord-indexed-row.json")
        }
        "captured-kakao-rooms-historical" => {
            include_str!("fixtures/inbox/captured-task23/kakao-rooms-historical.json")
        }
        "captured-kakao-rooms" => include_str!("fixtures/inbox/captured-task23/kakao-rooms.json"),
        "captured-kakao-numeric" => {
            include_str!("fixtures/inbox/captured-task23/kakao-messages-numeric.json")
        }
        _ => panic!("unknown fixture"),
    })
    .unwrap()
}

#[test]
fn mail_parses_authenticated_empty_capture_without_populated_substitution() {
    // Given: real empty capture, separate from synthetic populated mail.
    let payload = fixture("captured-mail-empty");
    for prior in [None, Some("1791676800")] {
        // When
        let (items, cursor) = MailSource::parse(&payload, prior).unwrap();
        // Then
        assert!(items.is_empty());
        assert_eq!(cursor, prior.unwrap_or(""));
    }
}

#[test]
fn imessage_parses_authenticated_sanitized_chat_and_history() {
    // Given
    let chats = fixture("captured-imessage-chats");
    let history = fixture("captured-imessage-history");
    let chat_id = chats["id"].as_i64().unwrap().to_string();
    // When
    let (items, cursor) =
        IMessageSource::parse(&chats, &[(chat_id.clone(), history.clone())], None).unwrap();
    // Then
    assert_eq!(items.len(), 2);
    for (item, row) in items.iter().zip(history.as_array().unwrap()) {
        assert_eq!(item.source, InboxSourceKind::Imessage);
        assert_eq!(item.source_ref, row["id"].as_i64().unwrap().to_string());
        assert_eq!(item.thread_ref.as_deref(), Some(chat_id.as_str()));
        assert_eq!(
            item.received_at,
            row["created_at"]
                .as_str()
                .unwrap()
                .parse::<chrono::DateTime<Utc>>()
                .unwrap()
        );
    }
    assert_eq!(
        cursor.parse::<chrono::DateTime<Utc>>().unwrap(),
        items.iter().map(|item| item.received_at).max().unwrap()
    );
}

#[test]
fn imessage_decodes_captured_rows_in_observed_ndjson_framing() {
    // Given: same captured rows re-serialized into observed framing, not raw bytes.
    let bytes = include_bytes!("fixtures/inbox/captured-task23/imessage-history.ndjson");
    // When
    let decoded = omon_gateway::inbox::sources::decode_structured_output(bytes, "imsg").unwrap();
    // Then
    assert_eq!(decoded, fixture("captured-imessage-history"));
}

#[tokio::test]
async fn discord_parses_authenticated_sanitized_indexed_row() {
    // Given
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let row = fixture("captured-discord");
    sqlx::query("INSERT INTO message_search_documents (platform,guild_id,channel_id,thread_id,message_id,author_id,author_name,content,attachment_names,timestamp,metadata_json) VALUES (?,?,?,?,?,?,?,?, '[]',?,?)")
        .bind(row["platform"].as_str().unwrap())
        .bind(row["guild_id"].as_str())
        .bind(row["channel_id"].as_str().unwrap())
        .bind(row["thread_id"].as_str())
        .bind(row["message_id"].as_str().unwrap())
        .bind(row["author_id"].as_str().unwrap())
        .bind(row["author_name"].as_str().unwrap())
        .bind(row["content"].as_str().unwrap())
        .bind(row["timestamp"].as_str().unwrap())
        .bind(row["metadata_json"].as_str().unwrap())
        .execute(pool).await.unwrap();
    let source = DiscordSource::with_ids(pool.clone(), "414011632306618368", None);
    // When
    let (items, cursor) = source.fetch_since(None).await.unwrap();
    // Then
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source, InboxSourceKind::Discord);
    assert_eq!(items[0].source_ref, row["message_id"].as_str().unwrap());
    assert_eq!(cursor, row["message_id"].as_str().unwrap());
    assert_eq!(items[0].author.as_deref(), row["author_id"].as_str());
    assert_eq!(items[0].title.as_deref(), row["author_name"].as_str());
    assert_eq!(items[0].snippet, row["content"].as_str().unwrap());
    assert!(items[0].thread_ref.is_none());
    assert_eq!(items[0].room_type.as_deref(), Some("dm"));
    assert_eq!(
        items[0].received_at,
        row["timestamp"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<Utc>>()
            .unwrap()
    );
    pool.close().await;
}

#[test]
fn kakao_parses_authenticated_numeric_seconds_capture() {
    // Given
    let rooms = fixture("captured-kakao-rooms");
    let room = &rooms[0];
    let page = fixture("captured-kakao-numeric");
    let rows = page.as_array().unwrap();
    // When
    let (items, oldest, newest) = KakaoSource::parse_messages(
        room["chatId"].as_str().unwrap(),
        room["title"].as_str(),
        room["type"].as_str().unwrap(),
        &page,
        "",
    )
    .unwrap();
    // Then
    assert!(!items.is_empty());
    assert_eq!(items.len(), rows.len());
    for (item, row) in items.iter().zip(rows) {
        assert_eq!(item.received_at.timestamp(), row["time"].as_i64().unwrap());
        assert_eq!(item.source_ref, row["id"].as_str().unwrap());
        assert_eq!(item.thread_ref.as_deref(), room["chatId"].as_str());
        assert_eq!(item.title.as_deref(), room["title"].as_str());
        assert_eq!(item.room_type.as_deref(), room["type"].as_str());
        assert_eq!(item.author.as_deref(), row["authorName"].as_str());
    }
    assert_eq!(
        oldest.as_deref(),
        rows.iter().map(|r| r["id"].as_str().unwrap()).min()
    );
    assert_eq!(
        newest.as_deref(),
        rows.iter().map(|r| r["id"].as_str().unwrap()).max()
    );
}

#[test]
fn mail_parses_sanitized_thread_and_builds_read_only_cursor_command() {
    let (items, cursor) = MailSource::parse(&fixture("mail"), None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "thread-redacted-001");
    assert!(items[0].snippet.contains("REDACTED"));
    assert_eq!(cursor, "1791676800");
    let args = MailSource::command_args(Some("1791676800"));
    assert_eq!(args[0..2], ["mail", "list"]);
    assert!(!args.iter().any(
        |arg| ["send", "reply", "forward", "archive", "read-mark", "trash"].contains(&arg.as_str())
    ));
    assert!(args.iter().any(|arg| arg.contains("after:1791676800")));
    assert!(!args.iter().any(|arg| arg == "--json"));
}

#[test]
fn mail_rejects_missing_shape_and_empty_cursor_is_stable() {
    assert!(MailSource::parse(&serde_json::json!({"items":[{"subject":"x"}]}), None).is_err());
    assert!(MailSource::parse(&serde_json::json!({}), None).is_err());
    assert!(MailSource::parse(&serde_json::json!({"items": null}), None).is_err());
    let (items, cursor) =
        MailSource::parse(&serde_json::json!({"items":[]}), Some("1791676800")).unwrap();
    assert!(items.is_empty());
    assert_eq!(cursor, "1791676800");
    let (items, cursor) = MailSource::parse(&serde_json::json!({"items":[]}), Some("abc")).unwrap();
    assert!(items.is_empty());
    assert_eq!(cursor, "abc");
}

#[test]
fn mail_rfc3339_internal_date_advances_cursor_from_parsed_timestamp() {
    let payload = serde_json::json!({
        "items": [{
            "thread_id": "thread-rfc3339-1",
            "internal_date": "2026-10-11T08:00:00Z",
            "from": "alice@example.invalid",
            "subject": "RFC3339 timestamp test",
            "snippet": "hello world"
        }]
    });
    let (items, cursor) = MailSource::parse(&payload, None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "thread-rfc3339-1");
    assert_eq!(items[0].received_at.timestamp(), 1791705600);
    assert_eq!(cursor, "1791705600");
}

#[test]
fn imessage_uses_only_read_subcommands_and_skips_own_messages() {
    let chats = fixture("imessage-chats");
    let history = fixture("imessage-history");
    let mut history_array = match history {
        Value::Array(items) => items,
        obj => vec![obj],
    };
    history_array.push(serde_json::json!({
        "id": 812346,
        "created_at": "2026-10-11T08:15:00Z",
        "is_from_me": true,
        "sender": "[REDACTED HANDLE]",
        "text": "[REDACTED OWNER TEXT]"
    }));
    let history_val = Value::Array(history_array);
    let (items, cursor) =
        IMessageSource::parse(&chats, &[("4653".into(), history_val)], None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "812345");
    assert!(cursor.contains("2026-10-11T08:14:10"));
    for args in [
        IMessageSource::chats_args(),
        IMessageSource::history_args("4653"),
    ] {
        assert!(matches!(args[0].as_str(), "chats" | "history"));
        assert!(!args
            .iter()
            .any(|arg| ["send", "read", "react"].contains(&arg.as_str())));
    }
}

#[test]
fn imessage_accepts_single_object_output_and_numeric_rowids() {
    let chat = fixture("imessage-chats");
    let history = fixture("imessage-history");
    let (items, cursor) = IMessageSource::parse(&chat, &[("4653".into(), history)], None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "812345");
    assert!(cursor.starts_with("2026-10-11T08:14:10"));
}

#[test]
fn imessage_parses_multi_item_ndjson_history() {
    let chat = fixture("imessage-chats");
    let ndjson_bytes = include_bytes!("fixtures/inbox/imessage-history.ndjson");
    let decoded =
        omon_gateway::inbox::sources::decode_structured_output(ndjson_bytes, "imsg").unwrap();
    let (items, cursor) = IMessageSource::parse(&chat, &[("4653".into(), decoded)], None).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].source_ref, "812345");
    assert_eq!(items[1].source_ref, "812346");
    assert!(cursor.starts_with("2026-10-11T08:15:10"));
}

#[test]
fn imessage_rejects_missing_history_and_bad_timestamps() {
    let chats = fixture("imessage-chats");
    assert!(IMessageSource::parse(&chats, &[], None).is_err());
    let malformed = serde_json::json!([{"id":"1","last_message_at":"yesterday"}]);
    assert!(IMessageSource::parse(&malformed, &[], None).is_err());
}

#[tokio::test]
async fn kakao_bad_endpoint_collects_other_three_sources_and_preserves_cursor() {
    use omon_gateway::models::{InboundEvent, SessionKey};
    use omon_gateway::storage::MessageSearchIndex;

    // Given: only Mail/iMessage CLI boundaries use synthetic sanitized fixtures.
    struct FixtureSource {
        kind: InboxSourceKind,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl InboxSource for FixtureSource {
        fn name(&self) -> InboxSourceKind {
            self.kind
        }

        async fn fetch_since(&self, cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self.kind {
                InboxSourceKind::Mail => MailSource::parse(&fixture("mail"), cursor),
                InboxSourceKind::Imessage => IMessageSource::parse(
                    &fixture("imessage-chats"),
                    &[("4653".into(), fixture("imessage-history"))],
                    cursor,
                ),
                InboxSourceKind::Kakao
                | InboxSourceKind::Discord
                | InboxSourceKind::Slack
                | InboxSourceKind::Github => {
                    Err(OmonError::Config("unsupported fixture source".into()))
                }
            }
        }
    }

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let old_cursor = serde_json::json!({"room-old": "100"}).to_string();
    sqlx::query("INSERT INTO inbox_cursors(source,cursor,updated_at) VALUES('kakao',?,?)")
        .bind(&old_cursor)
        .bind(chrono::DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap())
        .execute(pool)
        .await
        .unwrap();
    let session = SessionKey::new("discord", None::<&str>, "fixture-dm", None::<&str>, "other");
    let mut event = InboundEvent::message(session.clone(), "423456789012345672", "synthetic DM")
        .with_author_id("other");
    event.received_at = chrono::DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
    assert!(MessageSearchIndex::new(pool.clone())
        .index_inbound(&session, &event)
        .await
        .unwrap());
    let kakao = KakaoSource::new("http://127.0.0.1:9", None);
    let mail = FixtureSource {
        kind: InboxSourceKind::Mail,
        calls: AtomicUsize::new(0),
    };
    let imessage = FixtureSource {
        kind: InboxSourceKind::Imessage,
        calls: AtomicUsize::new(0),
    };
    let discord = DiscordSource::with_ids(pool.clone(), "414011632306618368", None);

    // When: the real collector sees failing Kakao before all three other sources.
    let results = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        collect_all(
            pool,
            &[&kakao, &mail, &imessage, &discord],
            &PrefilterRules::default(),
        ),
    )
    .await
    .unwrap();

    // Then: failure is persisted without moving its cursor or stopping other sources.
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].source, InboxSourceKind::Kakao);
    assert!(results[0]
        .error
        .as_deref()
        .unwrap()
        .contains("PirateTalk GET /api/rooms failed"));
    assert_eq!(results[0].stats.inserted, 0);
    let (cursor, error): (String, Option<String>) =
        sqlx::query_as("SELECT cursor,error FROM inbox_cursors WHERE source='kakao'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        KakaoSource::parse_cursor(Some(&cursor)),
        KakaoSource::parse_cursor(Some(&old_cursor))
    );
    assert_eq!(error, results[0].error);
    for (result, kind, expected_cursor) in [
        (&results[1], InboxSourceKind::Mail, "1791676800"),
        (
            &results[2],
            InboxSourceKind::Imessage,
            "2026-10-11T08:14:10.830+00:00",
        ),
        (&results[3], InboxSourceKind::Discord, "423456789012345672"),
    ] {
        assert_eq!(result.source, kind);
        assert!(result.error.is_none());
        assert_eq!(result.stats.inserted, 1);
        let (cursor, error): (String, Option<String>) =
            sqlx::query_as("SELECT cursor,error FROM inbox_cursors WHERE source=?")
                .bind(kind.as_str())
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(cursor, expected_cursor);
        assert!(error.is_none());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items WHERE source=?")
            .bind(kind.as_str())
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
    assert_eq!(mail.calls.load(Ordering::Relaxed), 1);
    assert_eq!(imessage.calls.load(Ordering::Relaxed), 1);
    database.pool().close().await;
}

#[test]
fn kakao_production_parser_and_per_room_cursor() {
    let messages = fixture("kakao-messages");
    let (items, oldest, newest) = KakaoSource::parse_messages(
        "18491246151592761",
        Some("[REDACTED ROOM TITLE]"),
        "OM",
        &messages,
        "",
    )
    .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "3949127159769622529");
    assert_eq!(items[0].thread_ref.as_deref(), Some("18491246151592761"));
    assert_eq!(items[0].room_type.as_deref(), Some("OM"));
    assert_eq!(items[0].title.as_deref(), Some("[REDACTED ROOM TITLE]"));
    assert!(items[0].snippet.contains("REDACTED"));
    assert_eq!(oldest.as_deref(), Some("3949127159769622529"));
    assert_eq!(newest.as_deref(), Some("3949127159769622529"));

    let mut cursor_map = KakaoSource::parse_cursor(None);
    cursor_map.insert("18491246151592761".into(), "3949127159769622529".into());
    cursor_map.insert("room2".into(), "100".into());
    let serialized = KakaoSource::serialize_cursor(&cursor_map);
    let parsed_back = KakaoSource::parse_cursor(Some(&serialized));
    assert_eq!(
        parsed_back.get("18491246151592761").unwrap(),
        "3949127159769622529"
    );
    assert_eq!(parsed_back.get("room2").unwrap(), "100");

    let page2 = serde_json::json!([
        {"id": "500", "logId": "500", "createdAt": "2026-10-11T08:00:00Z", "text": "older message"}
    ]);
    let (items2, oldest2, _) =
        KakaoSource::parse_messages("room2", None, "DirectChat", &page2, "100").unwrap();
    assert_eq!(items2.len(), 1);
    assert_eq!(oldest2.as_deref(), Some("500"));
}

#[test]
fn kakao_numeric_seconds_when_synthetic_contract_row_has_no_alias() {
    // Given: synthetic contract values, NOT an authenticated capture projection.
    let page = serde_json::json!([{
        "id": "500", "logId": "500", "time": 1_700_000_000,
        "text": "[REDACTED SYNTHETIC TEXT]", "authorName": "[REDACTED SYNTHETIC AUTHOR]"
    }]);
    // When
    let (items, oldest, newest) =
        KakaoSource::parse_messages("room", None, "DirectChat", &page, "").unwrap();
    // Then
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].received_at.timestamp(), 1_700_000_000);
    assert_eq!(oldest.as_deref(), Some("500"));
    assert_eq!(newest.as_deref(), Some("500"));
}

#[test]
fn kakao_numeric_seconds_take_precedence_when_alias_conflicts() {
    // Given: explicitly synthetic seconds and a conflicting compatibility alias.
    let page = serde_json::json!([{"id":"500", "text":"synthetic", "time":1_700_000_000, "createdAt":"2026-10-11T08:00:00Z"}]);
    // When
    let (items, _, _) = KakaoSource::parse_messages("room", None, "DirectChat", &page, "").unwrap();
    // Then
    assert_eq!(items[0].received_at.timestamp(), 1_700_000_000);
}

#[test]
fn kakao_rfc3339_aliases_when_numeric_time_absent() {
    for alias in ["createdAt", "created_at", "timestamp", "date"] {
        // Given
        let mut row = serde_json::json!({"id":"500", "text":"synthetic"});
        row[alias] = Value::String("2026-10-11T08:00:00Z".into());
        // When
        let (items, _, _) =
            KakaoSource::parse_messages("room", None, "DirectChat", &serde_json::json!([row]), "")
                .unwrap();
        // Then
        assert_eq!(items[0].received_at.timestamp(), 1_791_705_600);
    }
}

#[test]
fn kakao_rejects_invalid_numeric_time_even_when_alias_valid() {
    for time in [
        serde_json::json!(1.5),
        serde_json::json!("1700000000"),
        Value::Null,
        serde_json::json!(true),
        serde_json::json!(i64::MAX),
        serde_json::json!(9_223_372_036_854_775_808_u64),
    ] {
        // Given: every value is a synthetic invalid contract input.
        let page = serde_json::json!([{"id":"500", "text":"synthetic", "time":time, "createdAt":"2026-10-11T08:00:00Z"}]);
        // When
        let result = KakaoSource::parse_messages("room", None, "DirectChat", &page, "");
        // Then: no successful item batch is returned.
        assert!(result.is_err());
    }
}

#[test]
fn kakao_rejects_send_at_when_supported_timestamp_absent() {
    // Given: synthetic sendAt and logId must never supply a guessed timestamp.
    let page = serde_json::json!([{"id":"500", "logId":"500", "text":"synthetic", "sendAt":1_700_000_000}]);
    // When
    let result = KakaoSource::parse_messages("room", None, "DirectChat", &page, "");
    // Then
    assert!(result.is_err());
}

#[test]
fn kakao_skips_invalid_time_when_message_at_or_below_floor() {
    // Given
    let page = serde_json::json!([{"id":"499", "time":null}, {"id":"500", "time":true}]);
    // When
    let (items, oldest, newest) =
        KakaoSource::parse_messages("room", None, "DirectChat", &page, "500").unwrap();
    // Then
    assert!(items.is_empty());
    assert!(oldest.is_none());
    assert!(newest.is_none());
}

#[tokio::test]
async fn discord_behavioral_db_cases_and_cursor() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let owner_id = "414011632306618368";
    let bot_id = "999999999999999999";

    let now = Utc::now();
    let doc_fixture = fixture("discord");
    let initial_msg_id = doc_fixture["message_id"].as_str().unwrap();
    let test_channel_id = doc_fixture["channel_id"].as_str().unwrap();

    sqlx::query(
        "INSERT INTO message_search_documents (
            platform, guild_id, channel_id, thread_id, message_id,
            author_id, author_name, content, attachment_names, timestamp, metadata_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, ?)",
    )
    .bind("discord")
    .bind(doc_fixture["guild_id"].as_str())
    .bind(test_channel_id)
    .bind(doc_fixture["thread_id"].as_str())
    .bind(initial_msg_id)
    .bind(owner_id)
    .bind("owner")
    .bind("initial owner announcement")
    .bind(now)
    .bind("{}")
    .execute(pool)
    .await
    .unwrap();

    let reply_to_owner_meta = format!("{{\"referenced_message_id\":\"{initial_msg_id}\"}}");
    let cases = vec![
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None::<&str>,
            "423456789012345671",
            "other1",
            "other",
            "guild chatter without mention",
            "{}",
            false,
        ),
        (
            "discord",
            None,
            "dm1",
            None,
            "423456789012345672",
            "other2",
            "other",
            "true direct message to bot",
            "{}",
            true,
        ),
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None,
            "423456789012345673",
            "other3",
            "other",
            "guild mention <@414011632306618368> please check",
            "{}",
            true,
        ),
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None,
            "423456789012345674",
            bot_id,
            "bot",
            "bot mentioning <@414011632306618368>",
            "{}",
            false,
        ),
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None,
            "423456789012345675",
            owner_id,
            "owner",
            "owner self mention <@414011632306618368>",
            "{}",
            false,
        ),
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None,
            "423456789012345676",
            "other4",
            "other",
            "replying to owner message",
            &reply_to_owner_meta,
            true,
        ),
        (
            "discord",
            Some("guild1"),
            test_channel_id,
            None,
            "423456789012345677",
            "other5",
            "other",
            "replying to other message",
            "{\"referenced_message_id\":\"423456789012345671\"}",
            false,
        ),
    ];

    for (platform, guild, channel, thread, msg_id, author, author_name, content, meta, _) in &cases
    {
        sqlx::query(
            "INSERT INTO message_search_documents (
                platform, guild_id, channel_id, thread_id, message_id,
                author_id, author_name, content, attachment_names, timestamp, metadata_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]', ?, ?)",
        )
        .bind(platform)
        .bind(*guild)
        .bind(*channel)
        .bind(*thread)
        .bind(msg_id)
        .bind(author)
        .bind(author_name)
        .bind(content)
        .bind(now)
        .bind(meta)
        .execute(pool)
        .await
        .unwrap();
    }

    let source = DiscordSource::with_ids(pool.clone(), owner_id, Some(bot_id.into()));
    let (items, next_cursor) = source.fetch_since(Some(initial_msg_id)).await.unwrap();
    let collected_ids: Vec<String> = items.iter().map(|i| i.source_ref.clone()).collect();

    assert_eq!(
        collected_ids,
        vec![
            "423456789012345672",
            "423456789012345673",
            "423456789012345676"
        ]
    );
    assert_eq!(items[0].room_type.as_deref(), Some("dm"));
    assert_eq!(items[1].room_type.as_deref(), Some("channel"));
    assert_eq!(items[2].room_type.as_deref(), Some("owner_reply"));
    assert_eq!(next_cursor, "423456789012345676");
}

#[tokio::test]
async fn discord_producer_event_to_adapter_end_to_end() {
    use omon_gateway::models::{InboundEvent, SessionKey};
    use omon_gateway::storage::MessageSearchIndex;

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let pool = database.pool();
    let search_index = MessageSearchIndex::new(pool.clone());
    let owner_id = "414011632306618368";
    let bot_id = "999999999999999999";

    let session = SessionKey::new(
        "discord",
        Some("guild1"),
        "c1",
        None::<&str>,
        "ignored_user",
    );
    assert_eq!(session.user_id, "");

    let parent_event =
        InboundEvent::message(session.clone(), "423456789012345670", "parent announcement")
            .with_author_id(owner_id);
    let parent_indexed = search_index
        .index_inbound(&session, &parent_event)
        .await
        .unwrap();
    assert!(parent_indexed);

    let (parent_author,): (String,) = sqlx::query_as(
        "SELECT author_id FROM message_search_documents WHERE message_id = '423456789012345670'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(parent_author, owner_id);

    let reply_event =
        InboundEvent::message(session.clone(), "423456789012345679", "I agree with you")
            .with_author_id("other_user")
            .with_referenced_message_id("423456789012345670");

    let reply_indexed = search_index
        .index_inbound(&session, &reply_event)
        .await
        .unwrap();
    assert!(reply_indexed);

    let source = DiscordSource::with_ids(pool.clone(), owner_id, Some(bot_id.into()));
    let (items, cursor) = source
        .fetch_since(Some("423456789012345670"))
        .await
        .unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].source_ref, "423456789012345679");
    assert_eq!(items[0].room_type.as_deref(), Some("owner_reply"));
    assert_eq!(cursor, "423456789012345679");
}

#[tokio::test]
async fn kakao_fetches_authenticated_historical_room_on_owned_loopback() {
    captured_kakao_loopback(true).await;
}

#[tokio::test]
async fn kakao_fetches_authenticated_room_and_numeric_page_on_owned_loopback() {
    captured_kakao_loopback(false).await;
}

async fn captured_kakao_loopback(historical: bool) {
    use axum::{
        http::{Method, StatusCode, Uri},
        routing::get,
        Json, Router,
    };
    use tokio::{
        net::TcpListener,
        sync::{mpsc, oneshot},
        task::JoinSet,
    };

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        // Given: captured projections; empty pages are synthetic paging controls only.
        let rooms = fixture(if historical { "captured-kakao-rooms-historical" } else { "captured-kakao-rooms" });
        let page = if historical { serde_json::json!([]) } else { fixture("captured-kakao-numeric") };
        let room = rooms[0].clone();
        let room_id = room["chatId"].as_str().unwrap().to_owned();
        let message_path = format!("/api/rooms/{room_id}/messages");
        let minimum = page.as_array().unwrap().iter().map(|row| row["id"].as_str().unwrap()).min().map(str::to_owned);
        let maximum = page.as_array().unwrap().iter().map(|row| row["id"].as_str().unwrap()).max().map(str::to_owned);
        let terminal = minimum.as_ref().map(|id| format!("{message_path}?before_id={id}"));
        let (events, mut requests) = mpsc::channel::<String>(8);
        let expected = if historical { vec!["/api/rooms".to_owned(), message_path.clone()] } else {
            vec!["/api/rooms".to_owned(), message_path.clone(), terminal.clone().unwrap()]
        };
        let served_rooms = rooms.clone();
        let served_page = page.clone();
        let router = Router::new().fallback(get(move |method: Method, uri: Uri| {
            let events = events.clone();
            let rooms = served_rooms.clone();
            let page = served_page.clone();
            let message_path = message_path.clone();
            let terminal = terminal.clone();
            async move {
                let path = uri.path_and_query().unwrap().as_str().to_owned();
                events.send(path.clone()).await.unwrap();
                if method != Method::GET {
                    return (StatusCode::METHOD_NOT_ALLOWED, Json(serde_json::json!({})));
                }
                let body = if path == "/api/rooms" { rooms } else if path == message_path { page }
                    else if terminal.as_deref() == Some(path.as_str()) { serde_json::json!([]) }
                    else { return (StatusCode::NOT_FOUND, Json(serde_json::json!({}))); };
                (StatusCode::OK, Json(body))
            }
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, stopped) = oneshot::channel::<()>();
        let mut tasks = JoinSet::new();
        tasks.spawn(async move {
            axum::serve(listener, router).with_graceful_shutdown(async { stopped.await.unwrap(); }).await.unwrap();
        });
        let source = KakaoSource::new(format!("http://{address}"), None);
        let prior = historical.then(|| serde_json::json!({room_id.clone(): format!("{}0", room["lastLogId"].as_str().unwrap())}).to_string());

        // When: production fetch parses rooms and pages from owned loopback.
        let (items, cursor) = source.fetch_since(prior.as_deref()).await.unwrap();

        // Then: observed field values survive parsing and exact paging ends cleanly.
        if historical {
            assert!(items.is_empty());
            assert_eq!(KakaoSource::parse_cursor(Some(&cursor)), KakaoSource::parse_cursor(prior.as_deref()));
        } else {
            assert_eq!(items.len(), page.as_array().unwrap().len());
            for (item, row) in items.iter().zip(page.as_array().unwrap()) {
                assert_eq!(item.received_at.timestamp(), row["time"].as_i64().unwrap());
                assert_eq!(item.source_ref, row["id"].as_str().unwrap());
                assert_eq!(item.thread_ref.as_deref(), Some(room_id.as_str()));
                assert_eq!(item.room_type.as_deref(), room["type"].as_str());
                assert_eq!(item.title.as_deref(), room["title"].as_str());
            }
            assert_eq!(KakaoSource::parse_cursor(Some(&cursor)).get(&room_id), maximum.as_ref());
        }
        for path in expected { assert_eq!(requests.try_recv().unwrap(), path); }
        assert!(matches!(requests.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
        shutdown.send(()).unwrap();
        tasks.join_next().await.unwrap().unwrap();
        assert!(tasks.is_empty());
    }).await.unwrap();
}

#[tokio::test]
async fn kakao_event_driven_paging_loop() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(_) => break,
                };
                let mut req_bytes = Vec::new();
                let mut buf = [0u8; 1024];
                loop {
                    let n = match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => n,
                        _ => break,
                    };
                    req_bytes.extend_from_slice(&buf[..n]);
                    if req_bytes.windows(4).any(|w| w == b"\r\n\r\n")
                        || req_bytes.windows(2).any(|w| w == b"\n\n")
                    {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&req_bytes);
                let first_line = req.lines().next().unwrap_or("");

                let (status, body) = if first_line.contains("GET /api/rooms ") {
                    (
                        "200 OK",
                        serde_json::json!([
                            {
                                "chatId": "room-1",
                                "type": "OM",
                                "title": "Room 1",
                                "unreadCount": 1,
                                "lastLogId": "500"
                            },
                            {
                                "chatId": "room-2",
                                "type": "DirectChat",
                                "title": "Room 2",
                                "unreadCount": 0,
                                "lastLogId": "100"
                            }
                        ])
                        .to_string(),
                    )
                } else if first_line.contains("GET /api/rooms/room-1/messages ") {
                    (
                        "200 OK",
                        serde_json::json!([
                            {"id": "500", "logId": "500", "createdAt": "2026-10-11T08:02:00Z", "text": "r1-msg-500"},
                            {"id": "400", "logId": "400", "createdAt": "2026-10-11T08:01:00Z", "text": "r1-msg-400"}
                        ])
                        .to_string(),
                    )
                } else if first_line.contains("GET /api/rooms/room-1/messages?before_id=400 ") {
                    (
                        "200 OK",
                        serde_json::json!([
                            {"id": "300", "logId": "300", "createdAt": "2026-10-11T08:00:00Z", "text": "r1-msg-300"}
                        ])
                        .to_string(),
                    )
                } else if first_line.contains("GET /api/rooms/room-1/messages?before_id=300 ") {
                    ("200 OK", "[]".to_string())
                } else if first_line.contains("GET /api/rooms/room-2/messages ") {
                    (
                        "200 OK",
                        serde_json::json!([
                            {"id": "100", "logId": "100", "createdAt": "2026-10-11T07:50:00Z", "text": "r2-msg-100"}
                        ])
                        .to_string(),
                    )
                } else if first_line.contains("GET /api/rooms/room-2/messages?before_id=100 ") {
                    ("200 OK", "[]".to_string())
                } else {
                    ("404 Not Found", "{}".to_string())
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let source = KakaoSource::new(format!("http://{addr}"), None);
        let (items, cursor) = source.fetch_since(None).await.unwrap();

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].source_ref, "500");
        assert_eq!(items[1].source_ref, "400");
        assert_eq!(items[2].source_ref, "300");
        assert_eq!(items[3].source_ref, "100");

        let cursor_map = KakaoSource::parse_cursor(Some(&cursor));
        assert_eq!(cursor_map.get("room-1").unwrap(), "500");
        assert_eq!(cursor_map.get("room-2").unwrap(), "100");

        let (items2, cursor2) = source.fetch_since(Some(&cursor)).await.unwrap();
        assert_eq!(items2.len(), 0);

        let map1 = KakaoSource::parse_cursor(Some(&cursor));
        let map2 = KakaoSource::parse_cursor(Some(&cursor2));
        assert_eq!(map1, map2);

        server_task.abort();
    })
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn live_mail_source_fetch_once() {
    let source = MailSource;
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(60), source.fetch_since(None)).await;
    let (items, _cursor) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => panic!("QA fetch failed source=mail"),
        Err(_) => panic!("QA fetch timeout source=mail"),
    };
    eprintln!("QA_COUNT source=mail items={}", items.len());
}

#[tokio::test]
#[ignore]
async fn live_kakao_source_fetch_once() {
    let source = KakaoSource::default();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(60), source.fetch_since(None)).await;
    let (items, _cursor) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => panic!("QA fetch failed source=kakao"),
        Err(_) => panic!("QA fetch timeout source=kakao"),
    };
    eprintln!("QA_COUNT source=kakao items={}", items.len());
}

#[tokio::test]
#[ignore]
async fn live_imessage_source_fetch_once() {
    let source = IMessageSource;
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(60), source.fetch_since(None)).await;
    let (items, _cursor) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => panic!("QA fetch failed source=imessage"),
        Err(_) => panic!("QA fetch timeout source=imessage"),
    };
    eprintln!("QA_COUNT source=imessage items={}", items.len());
}

#[tokio::test]
#[ignore]
async fn live_discord_source_fetch_once() {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let copy_path = std::env::var("OMON_QA_DISCORD_COPY_DB")
        .unwrap_or_else(|_| panic!("QA copy configuration missing source=discord"));
    let options = SqliteConnectOptions::from_str(&copy_path)
        .unwrap_or_else(|_| panic!("QA copy configuration invalid source=discord"))
        .read_only(true);
    let pool_options = sqlx::sqlite::SqlitePoolOptions::new();
    let pool = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        pool_options.connect_with(options),
    )
    .await
    {
        Ok(Ok(pool)) => pool,
        Ok(Err(_)) => panic!("QA copy connection failed source=discord"),
        Err(_) => panic!("QA copy connection timeout source=discord"),
    };
    let source = DiscordSource::new(pool);
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(60), source.fetch_since(None)).await;
    let (items, _cursor) = match result {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => panic!("QA fetch failed source=discord"),
        Err(_) => panic!("QA fetch timeout source=discord"),
    };
    eprintln!("QA_COUNT source=discord items={}", items.len());
}

#[test]
fn fixture_files_contain_only_sanitized_personal_text() {
    for value in [
        fixture("mail"),
        fixture("kakao-messages"),
        fixture("imessage-history"),
        fixture("discord"),
    ] {
        let text = value.to_string();
        assert!(text.contains("REDACTED"));
        assert!(!text.contains("@gmail.com"));
        assert!(!text.contains("안녕하세요"));
    }
}

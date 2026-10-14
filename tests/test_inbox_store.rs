use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use omon_gateway::inbox::{
    collect_all, prefilter::PrefilterRules, watchers, InboxItem, InboxSource, InboxSourceKind,
};
use omon_gateway::storage::Database;
use omon_gateway::{OmonError, Result};
use sqlx::Row;

struct FakeSource {
    kind: InboxSourceKind,
    items: Vec<InboxItem>,
    cursor: &'static str,
    fail: bool,
}

#[async_trait]
impl InboxSource for FakeSource {
    fn name(&self) -> InboxSourceKind {
        self.kind
    }

    async fn fetch_since(&self, _cursor: Option<&str>) -> Result<(Vec<InboxItem>, String)> {
        if self.fail {
            return Err(OmonError::Config("fake failure".into()));
        }
        Ok((self.items.clone(), self.cursor.into()))
    }
}

fn item(source: InboxSourceKind, source_ref: &str, snippet: &str) -> InboxItem {
    InboxItem::new(
        source,
        source_ref,
        snippet,
        Utc.timestamp_opt(1_760_000_000, 0).unwrap(),
    )
}

async fn db() -> Database {
    Database::connect("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn repeated_collection_deduplicates_and_caps_snippets() {
    let database = db().await;
    let pool = database.pool();
    let mut valid = item(InboxSourceKind::Mail, "mail-1", &"x".repeat(1100));
    valid.author = Some("person@example.test".into());
    let source = FakeSource {
        kind: InboxSourceKind::Mail,
        items: vec![valid],
        cursor: "cursor-1",
        fail: false,
    };
    for _ in 0..2 {
        let results = collect_all(pool, &[&source], &PrefilterRules::default()).await;
        assert!(results[0].error.is_none());
    }
    let row = sqlx::query("SELECT count(*) AS count, length(snippet) AS len FROM inbox_items")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("count"), 1);
    assert_eq!(row.get::<i64, _>("len"), 1000);
    let cursor =
        sqlx::query_scalar::<_, String>("SELECT cursor FROM inbox_cursors WHERE source='mail'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(cursor, "cursor-1");
}

#[tokio::test]
async fn empty_kakao_rows_are_skipped_without_losing_valid_items_or_cursor() {
    // Given: an attachment-only row and an unusable reference among valid rows.
    let database = db().await;
    let mut valid = item(InboxSourceKind::Kakao, "valid", "hello");
    valid.room_type = Some("DirectChat".into());
    let source = FakeSource {
        kind: InboxSourceKind::Kakao,
        items: vec![
            valid,
            item(InboxSourceKind::Kakao, "attachment", " "),
            item(InboxSourceKind::Kakao, "", "text"),
        ],
        cursor: "next",
        fail: false,
    };
    // When
    let results = collect_all(database.pool(), &[&source], &PrefilterRules::default()).await;
    // Then
    assert!(results[0].error.is_none());
    assert_eq!(results[0].stats.inserted, 1);
    assert_eq!(results[0].stats.dropped, 2);
    let cursor: String =
        sqlx::query_scalar("SELECT cursor FROM inbox_cursors WHERE source='kakao'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(cursor, "next");
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(stored, 1);
}

#[tokio::test]
async fn prefilter_drops_and_counts_rules_and_keeps_required_passes() {
    let database = db().await;
    let pool = database.pool();
    let rules = PrefilterRules::default();
    let mut promotion = item(InboxSourceKind::Mail, "promo", "offer");
    promotion.title = Some("[promotions] sale".into());
    let mut noise = item(InboxSourceKind::Kakao, "noise", "hello");
    noise.room_type = Some("OM".into());
    let mut shortcode = item(InboxSourceKind::Imessage, "sms", "code");
    shortcode.author = Some("114".into());
    let mut pomodan = item(InboxSourceKind::Kakao, "pomodan", "text");
    pomodan.thread_ref = Some("18491246151592761".into());
    pomodan.room_type = Some("MultiChat".into());
    let mut direct = item(InboxSourceKind::Kakao, "direct", "hello");
    direct.room_type = Some("DirectChat".into());
    let mut github = item(InboxSourceKind::Mail, "github", "please review");
    github.author = Some("notifications@github.com".into());
    github.title = Some("Issue notification".into());
    let mut mention = item(InboxSourceKind::Discord, "mention", "<@414011632306618368>");
    mention.author = Some("other-user".into());
    mention.thread_ref = Some("channel".into());

    for (kind, it) in [
        (InboxSourceKind::Mail, promotion),
        (InboxSourceKind::Mail, github.clone()),
        (InboxSourceKind::Kakao, noise),
        (InboxSourceKind::Kakao, pomodan),
        (InboxSourceKind::Kakao, direct),
        (InboxSourceKind::Imessage, shortcode),
        (InboxSourceKind::Discord, mention),
    ] {
        let result = collect_all(
            pool,
            &[&FakeSource {
                kind,
                items: vec![it],
                cursor: "next",
                fail: false,
            }],
            &rules,
        )
        .await;
        assert!(result[0].error.is_none());
    }
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(stored, 3);
    let drops: i64 = sqlx::query_scalar("SELECT sum(count) FROM inbox_filter_stats")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(drops, 4);
    let class: Option<String> = sqlx::query_scalar(
        "SELECT class FROM inbox_decisions WHERE item_id=(SELECT id FROM inbox_items WHERE source_ref='github')",
    )
    .fetch_optional(pool)
    .await
    .unwrap();
    assert_eq!(class, None);
    let mut review = github;
    review.source_ref = "github-review".into();
    review.title = Some("Review requested".into());
    assert_eq!(rules.direct_class(&review), None);
}

#[tokio::test]
async fn source_failure_isolated_and_preserves_cursor() {
    let database = db().await;
    let pool = database.pool();
    sqlx::query("INSERT INTO inbox_cursors(source,cursor,updated_at) VALUES('mail','old',?)")
        .bind(Utc::now())
        .execute(pool)
        .await
        .unwrap();
    let broken = FakeSource {
        kind: InboxSourceKind::Mail,
        items: vec![],
        cursor: "new",
        fail: true,
    };
    let mut good_item = item(InboxSourceKind::Discord, "dm", "hi");
    good_item.room_type = Some("dm".into());
    let good = FakeSource {
        kind: InboxSourceKind::Discord,
        items: vec![good_item],
        cursor: "d1",
        fail: false,
    };
    let results = collect_all(pool, &[&broken, &good], &PrefilterRules::default()).await;
    assert!(results[0]
        .error
        .as_deref()
        .unwrap()
        .contains("fake failure"));
    assert!(results[1].error.is_none());
    let cursor: String = sqlx::query_scalar("SELECT cursor FROM inbox_cursors WHERE source='mail'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(cursor, "old");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items WHERE source='discord'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn failed_item_upsert_rolls_back_cursor_advance() {
    let database = db().await;
    let pool = database.pool();
    sqlx::query("INSERT INTO inbox_cursors(source,cursor,updated_at) VALUES('mail','old',?)")
        .bind(Utc::now())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_inbox_insert BEFORE INSERT ON inbox_items BEGIN SELECT RAISE(FAIL, 'injected upsert failure'); END")
        .execute(pool).await.unwrap();
    let source = FakeSource {
        kind: InboxSourceKind::Mail,
        items: vec![item(InboxSourceKind::Mail, "bad", "body")],
        cursor: "new",
        fail: false,
    };
    let result = collect_all(pool, &[&source], &PrefilterRules::default()).await;
    assert!(result[0]
        .error
        .as_deref()
        .unwrap()
        .contains("injected upsert failure"));
    let cursor: String = sqlx::query_scalar("SELECT cursor FROM inbox_cursors WHERE source='mail'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(cursor, "old");
}

#[tokio::test]
async fn filter_defaults_are_created_only_at_requested_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inbox-filters.json");
    let rules = PrefilterRules::load_or_create(&path).unwrap();
    assert!(rules.excluded_kakao_rooms.contains("18491246151592761"));
    assert!(path.exists());
}

#[tokio::test]
async fn invalid_source_item_aborts_items_and_cursor_transaction() {
    let database = db().await;
    let pool = database.pool();
    let wrong = item(InboxSourceKind::Discord, "wrong-source", "hi");
    let source = FakeSource {
        kind: InboxSourceKind::Mail,
        items: vec![wrong],
        cursor: "new",
        fail: false,
    };
    let result = collect_all(pool, &[&source], &PrefilterRules::default()).await;
    assert!(result[0].error.is_some());
    let cursor: (String, Option<String>) =
        sqlx::query_as("SELECT cursor, error FROM inbox_cursors WHERE source='mail'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(cursor.0, "");
    assert_eq!(cursor.1, result[0].error);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM inbox_items")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn pure_prefilter_discord_rules_and_author_excludes() {
    let rules = PrefilterRules {
        owner_discord_id: "414011632306618368".into(),
        bot_discord_id: Some("999888777666".into()),
        ..Default::default()
    };

    // 1. Owner mention passes
    let mut mention = item(
        InboxSourceKind::Discord,
        "d-mention",
        "hello <@414011632306618368>",
    );
    mention.author = Some("other-user".into());
    mention.thread_ref = Some("channel-1".into());
    assert_eq!(rules.drop_reason(&mention), None);

    // 2. Owner nickname mention passes
    let mut nick_mention = item(
        InboxSourceKind::Discord,
        "d-nick",
        "ping <@!414011632306618368>",
    );
    nick_mention.author = Some("other-user".into());
    assert_eq!(rules.drop_reason(&nick_mention), None);

    // 3. Direct message (DM) passes when room_type is "dm"
    let mut dm = item(InboxSourceKind::Discord, "d-dm", "private chat");
    dm.author = Some("other-user".into());
    dm.room_type = Some("dm".into());
    assert_eq!(rules.drop_reason(&dm), None);

    // 4. Owner reply passes when room_type is "owner_reply"
    let mut reply = item(
        InboxSourceKind::Discord,
        "d-reply",
        "replying to owner announcement",
    );
    reply.author = Some("other-user".into());
    reply.room_type = Some("owner_reply".into());
    reply.thread_ref = Some("channel-1".into());
    assert_eq!(rules.drop_reason(&reply), None);

    // 5. Author is owner (self-mention) dropped
    let mut self_mention = item(
        InboxSourceKind::Discord,
        "d-self",
        "I mention myself <@414011632306618368>",
    );
    self_mention.author = Some("414011632306618368".into());
    assert_eq!(
        rules.drop_reason(&self_mention),
        Some("discord_author_excluded")
    );

    // 6. Author is bot dropped
    let mut bot_msg = item(
        InboxSourceKind::Discord,
        "d-bot",
        "bot output <@414011632306618368>",
    );
    bot_msg.author = Some("999888777666".into());
    assert_eq!(rules.drop_reason(&bot_msg), Some("discord_author_excluded"));

    // 7. Non-mention, non-reply guild message with thread_ref=None dropped (fixes wrongDM bug)
    let mut guild_chatter = item(
        InboxSourceKind::Discord,
        "d-chatter",
        "random chatter in guild",
    );
    guild_chatter.author = Some("other-user".into());
    guild_chatter.thread_ref = None;
    guild_chatter.room_type = None;
    assert_eq!(
        rules.drop_reason(&guild_chatter),
        Some("discord_not_owner_mention_or_dm")
    );

    // 8. Non-mention, non-reply guild message with thread_ref=Some dropped
    let mut thread_chatter = item(
        InboxSourceKind::Discord,
        "d-thread",
        "random chatter in thread",
    );
    thread_chatter.author = Some("other-user".into());
    thread_chatter.thread_ref = Some("thread-101".into());
    thread_chatter.room_type = None;
    assert_eq!(
        rules.drop_reason(&thread_chatter),
        Some("discord_not_owner_mention_or_dm")
    );
}

#[tokio::test]
async fn pure_prefilter_mail_kakao_imessage_rules() {
    let rules = PrefilterRules {
        owner_kakao_name: "Indo".into(),
        kakao_keywords: vec!["urgent".into(), "important".into()],
        ..Default::default()
    };

    // Mail categories
    let mut promo = item(InboxSourceKind::Mail, "m-promo", "sale 50%");
    promo.mail_categories = vec!["promotions".into()];
    assert_eq!(rules.drop_reason(&promo), Some("mail_promotions"));

    let mut social = item(InboxSourceKind::Mail, "m-social", "new connection");
    social.mail_categories = vec!["social".into()];
    assert_eq!(rules.drop_reason(&social), Some("mail_social"));

    let mut updates = item(InboxSourceKind::Mail, "m-updates", "terms update");
    updates.mail_categories = vec!["updates".into()];
    assert_eq!(rules.drop_reason(&updates), Some("mail_updates"));

    let clean_mail = item(InboxSourceKind::Mail, "m-clean", "personal note");
    assert_eq!(rules.drop_reason(&clean_mail), None);

    // Kakao rules
    let mut pomodan = item(InboxSourceKind::Kakao, "k-pomodan", "chat");
    pomodan.thread_ref = Some("18491246151592761".into());
    assert_eq!(rules.drop_reason(&pomodan), Some("kakao_excluded_room"));

    let mut noise_no_keyword = item(InboxSourceKind::Kakao, "k-noise", "hello everyone");
    noise_no_keyword.room_type = Some("OM".into());
    assert_eq!(
        rules.drop_reason(&noise_no_keyword),
        Some("kakao_noise_room_no_keyword")
    );

    let mut noise_with_owner = item(
        InboxSourceKind::Kakao,
        "k-owner",
        "hello Indo can you check this",
    );
    noise_with_owner.room_type = Some("OM".into());
    assert_eq!(rules.drop_reason(&noise_with_owner), None);

    let mut noise_with_keyword = item(InboxSourceKind::Kakao, "k-keyword", "this is urgent");
    noise_with_keyword.room_type = Some("OM".into());
    assert_eq!(rules.drop_reason(&noise_with_keyword), None);

    let mut unsupported = item(InboxSourceKind::Kakao, "k-unsupp", "some text");
    unsupported.room_type = Some("SecretChat".into());
    assert_eq!(
        rules.drop_reason(&unsupported),
        Some("kakao_unsupported_room_type")
    );

    let mut direct = item(InboxSourceKind::Kakao, "k-direct", "hello");
    direct.room_type = Some("DirectChat".into());
    assert_eq!(rules.drop_reason(&direct), None);

    let mut multi = item(InboxSourceKind::Kakao, "k-multi", "group text");
    multi.room_type = Some("MultiChat".into());
    assert_eq!(rules.drop_reason(&multi), None);

    let mut own_message = item(InboxSourceKind::Kakao, "k-own", "오늘만좀봐줘요");
    own_message.room_type = Some("DirectChat".into());
    own_message.author = Some("윤인도".into());
    assert_eq!(rules.drop_reason(&own_message), Some("kakao_owner_author"));

    let mut otp = item(
        InboxSourceKind::Imessage,
        "i-otp",
        "[국제발신] G-683968 - Google 인증 코드입니다.",
    );
    otp.author = Some("00614664531".into());
    assert_eq!(rules.drop_reason(&otp), Some("imessage_otp"));

    let mut stripe = item(
        InboxSourceKind::Imessage,
        "i-stripe",
        "Your Stripe verification code is: 087462.",
    );
    stripe.author = Some("+447873398241".into());
    assert_eq!(rules.drop_reason(&stripe), Some("imessage_otp"));

    let mut request = item(
        InboxSourceKind::Imessage,
        "i-request",
        "강의자료는 jhkj1224@korea.kr로 보내주시면 됩니다",
    );
    request.author = Some("+821099995218".into());
    assert_eq!(rules.drop_reason(&request), None);

    // iMessage rules
    let mut shortcode = item(
        InboxSourceKind::Imessage,
        "im-short",
        "verification code 1234",
    );
    shortcode.author = Some("114".into());
    assert_eq!(rules.drop_reason(&shortcode), Some("imessage_shortcode"));

    let ad_web = item(
        InboxSourceKind::Imessage,
        "im-ad1",
        "[Web발신] loan advertisement",
    );
    assert_eq!(rules.drop_reason(&ad_web), Some("imessage_ad"));

    let ad_prefix = item(
        InboxSourceKind::Imessage,
        "im-ad2",
        "(광고) special discount",
    );
    assert_eq!(rules.drop_reason(&ad_prefix), Some("imessage_ad"));

    let mut normal_imsg = item(
        InboxSourceKind::Imessage,
        "im-normal",
        "Hey, are you free tomorrow?",
    );
    normal_imsg.author = Some("+821012345678".into());
    assert_eq!(rules.drop_reason(&normal_imsg), None);
}

#[tokio::test]
async fn watcher_event_callback_dispatches_without_sleep() {
    let database = db().await;
    let pool = database.pool();
    let rules = PrefilterRules::default();

    let mut discord_item = item(
        InboxSourceKind::Discord,
        "d-evt-1",
        "ping <@414011632306618368>",
    );
    discord_item.author = Some("sender-1".into());

    let imsg_item = item(InboxSourceKind::Imessage, "812345", "sms text");
    let mail_item = item(InboxSourceKind::Mail, "m-1", "regular email text");
    let mut kakao_item = item(InboxSourceKind::Kakao, "k-1", "hello direct");
    kakao_item.room_type = Some("DirectChat".into());

    let sources: Vec<Box<dyn InboxSource>> = vec![
        Box::new(FakeSource {
            kind: InboxSourceKind::Discord,
            items: vec![discord_item],
            cursor: "d-cur-1",
            fail: false,
        }),
        Box::new(FakeSource {
            kind: InboxSourceKind::Imessage,
            items: vec![imsg_item],
            cursor: "2026-10-11T08:00:00Z",
            fail: false,
        }),
        Box::new(FakeSource {
            kind: InboxSourceKind::Mail,
            items: vec![mail_item],
            cursor: "1791676800",
            fail: false,
        }),
        Box::new(FakeSource {
            kind: InboxSourceKind::Kakao,
            items: vec![kakao_item],
            cursor: "k-cur-1",
            fail: false,
        }),
    ];

    let (event_tx, event_rx) = watchers::event_channel(16);
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(16);

    let watcher_handle = tokio::spawn(watchers::run_event_watch(
        pool.clone(),
        sources,
        rules,
        event_rx,
        Some(progress_tx),
    ));

    // 1. GatewayMessage event -> Discord collection
    watchers::on_gateway_message(&event_tx).await.unwrap();
    let results = progress_rx.recv().await.expect("progress delivered");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, InboxSourceKind::Discord);
    assert_eq!(results[0].stats.inserted, 1);

    // 2. IMessage event -> iMessage collection
    watchers::on_imessage_event(&event_tx, Some(812345))
        .await
        .unwrap();
    let results = progress_rx.recv().await.expect("progress delivered");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, InboxSourceKind::Imessage);
    assert_eq!(results[0].stats.inserted, 1);

    // 3. Mail event -> Mail collection
    watchers::on_mail_event(&event_tx).await.unwrap();
    let results = progress_rx.recv().await.expect("progress delivered");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, InboxSourceKind::Mail);
    assert_eq!(results[0].stats.inserted, 1);

    // 4. Periodic event -> Kakao collection
    watchers::on_periodic(&event_tx).await.unwrap();
    let results = progress_rx.recv().await.expect("progress delivered");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, InboxSourceKind::Kakao);
    assert_eq!(results[0].stats.inserted, 1);

    drop(event_tx);
    watcher_handle.await.unwrap();
}

#[tokio::test]
async fn imessage_cursor_timestamp_and_watcher_rowid_consistency() {
    let database = db().await;
    let pool = database.pool();

    let timestamp_cursor = "2026-10-11T08:14:10.830Z";
    sqlx::query("INSERT INTO inbox_cursors(source, cursor, updated_at) VALUES ('imessage', ?, ?)")
        .bind(timestamp_cursor)
        .bind(Utc::now())
        .execute(pool)
        .await
        .unwrap();

    for rowid in ["812340", "812345", "812346"] {
        let it = item(InboxSourceKind::Imessage, rowid, &format!("text {rowid}"));
        sqlx::query(
            "INSERT INTO inbox_items (id, source, source_ref, snippet, received_at, raw_hash, collected_at) \
             VALUES (?, 'imessage', ?, ?, ?, ?, ?)",
        )
        .bind(it.id())
        .bind(rowid)
        .bind(&it.snippet)
        .bind(it.received_at)
        .bind(&it.raw_hash)
        .bind(Utc::now())
        .execute(pool)
        .await
        .unwrap();
    }

    let read_cursor = watchers::read_imessage_source_cursor(pool).await.unwrap();
    assert_eq!(read_cursor.as_deref(), Some(timestamp_cursor));

    let latest_rowid = watchers::latest_imessage_rowid(pool).await.unwrap();
    assert_eq!(latest_rowid, Some(812346));

    let args = watchers::imsg_watch_args(latest_rowid);
    assert_eq!(args, vec!["watch", "--json", "--since-rowid", "812346"]);

    let no_rowid_args = watchers::imsg_watch_args(None);
    assert_eq!(no_rowid_args, vec!["watch", "--json"]);

    let parsed_id = watchers::parse_imsg_watch_line(r#"{"id": 812347, "text": "new msg"}"#);
    assert_eq!(parsed_id, Some(812347));

    let parsed_rowid =
        watchers::parse_imsg_watch_line(r#"{"rowid": "812348", "text": "rowid msg"}"#);
    assert_eq!(parsed_rowid, Some(812348));

    let verify_cursor = watchers::read_imessage_source_cursor(pool).await.unwrap();
    assert_eq!(verify_cursor.as_deref(), Some(timestamp_cursor));
}

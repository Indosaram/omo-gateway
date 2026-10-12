#[path = "../src/profiles/lanes_block.rs"]
mod lanes_block;

use lanes_block::{apply_block, export_json, export_profile, ApplyBlockError};
use omon_gateway::Database;
use sha2::{Digest, Sha256};

const BLOCK: &str = "<!-- omon:lanes:begin -->\nnew lane rules\n<!-- omon:lanes:end -->\n";

#[tokio::test]
async fn cli_applies_block_with_named_flags_on_scratch_profile() {
    // Given: a scratch database containing a copied profile and the shipped block.
    let scratch = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", scratch.path().join("profiles.db").display());
    let database = Database::connect(&url).await.unwrap();
    sqlx::query(
        "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets)
         VALUES ('scratch-bot', 'Copied profile', 'unchanged-model', 'owner prompt', '[]')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    let before = export_profile(database.pool(), "scratch-bot")
        .await
        .unwrap()
        .unwrap();
    let block_file =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bot_profiles/blocks/lanes.md");

    // When: the actual CLI applies the block twice using its public flags.
    let invoke = |bot_id: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
            .current_dir(scratch.path())
            .env("DATABASE_URL", &url)
            .args(["profile", "apply-block", "--bot-id", bot_id, "--block-file"])
            .arg(&block_file)
            .output()
            .unwrap()
    };
    let first = invoke("scratch-bot");
    let second = invoke("scratch-bot");
    let missing = invoke("missing");

    // Then: digests are idempotent, other columns survive, and unknown IDs exit 2.
    assert!(first.status.success(), "{:?}", first);
    assert!(second.status.success(), "{:?}", second);
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first_json["after"], second_json["before"]);
    assert_eq!(first_json["after"], second_json["after"]);
    assert_eq!(missing.status.code(), Some(2));
    let after = export_profile(database.pool(), "scratch-bot")
        .await
        .unwrap()
        .unwrap();
    let prompt = after.system_prompt.as_deref().unwrap();
    assert_eq!(first_json["after"]["sha256"], sha256(prompt));
    assert_eq!(first_json["after"]["characters"], prompt.chars().count());
    assert_eq!(prompt.matches("<!-- omon:lanes:begin -->").count(), 1);
    let mut expected = before;
    expected.system_prompt = after.system_prompt.clone();
    assert_eq!(expected, after);
}

async fn database_with_profile(prompt: Option<&str>) -> Database {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "INSERT INTO bot_profiles
         (bot_id, name, model, system_prompt, enabled_toolsets, custom_settings_json, created_at, updated_at)
         VALUES ('bot-1', 'Test', 'model-x', ?, 'tools-json', '{\"setting\": 1}', 'created-byte', 'updated-byte')",
    )
    .bind(prompt)
    .execute(database.pool())
    .await
    .unwrap();
    database
}

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

async fn row_bytes(
    pool: &sqlx::SqlitePool,
) -> (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
) {
    sqlx::query_as(
        "SELECT name, model, system_prompt, enabled_toolsets, custom_settings_json, created_at, updated_at
         FROM bot_profiles WHERE bot_id = 'bot-1'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn appends_once_and_is_idempotent_with_prompt_only_update() {
    let database = database_with_profile(Some("original prompt")).await;
    let pool = database.pool();
    let before_other_columns = row_bytes(pool).await;

    let first = apply_block(pool, "bot-1", BLOCK).await.unwrap();
    let first_prompt = row_bytes(pool).await.2.unwrap();
    let second = apply_block(pool, "bot-1", BLOCK).await.unwrap();
    let second_prompt = row_bytes(pool).await.2.unwrap();

    assert_eq!(first.before.length, "original prompt".chars().count());
    assert_eq!(first.before.sha256, sha256("original prompt"));
    assert_eq!(first_prompt.matches("<!-- omon:lanes:begin -->").count(), 1);
    assert_eq!(first_prompt.matches("<!-- omon:lanes:end -->").count(), 1);
    assert_eq!(first_prompt, second_prompt);
    assert_eq!(second.after.sha256, sha256(&second_prompt));
    assert_eq!(before_other_columns.0, row_bytes(pool).await.0);
    let after = row_bytes(pool).await;
    assert_eq!(before_other_columns.0, after.0);
    assert_eq!(before_other_columns.1, after.1);
    assert_eq!(before_other_columns.3, after.3);
    assert_eq!(before_other_columns.4, after.4);
    assert_eq!(before_other_columns.5, after.5);
    assert_eq!(before_other_columns.6, after.6);
}

#[tokio::test]
async fn replaces_only_marked_region_and_preserves_surrounding_text() {
    let initial =
        "before\n<!-- omon:lanes:begin -->\nold\n<!-- omon:lanes:end -->\nowner suffix\nafter";
    let database = database_with_profile(Some(initial)).await;
    let pool = database.pool();
    let other_before = row_bytes(pool).await;
    let replacement = "<!-- omon:lanes:begin -->\nreplacement\n<!-- omon:lanes:end -->\n";

    let first = apply_block(pool, "bot-1", replacement).await.unwrap();
    let first_prompt = row_bytes(pool).await.2.unwrap();
    let second = apply_block(pool, "bot-1", replacement).await.unwrap();

    let after = row_bytes(pool).await;
    assert_eq!(
        after.2.as_deref(),
        Some("before\n<!-- omon:lanes:begin -->\nreplacement\n<!-- omon:lanes:end -->\nowner suffix\nafter")
    );
    assert_eq!(after.2.as_deref(), Some(first_prompt.as_str()));
    assert_eq!(first.after.sha256, second.after.sha256);
    assert_eq!(other_before.0, after.0);
    assert_eq!(other_before.1, after.1);
    assert_eq!(other_before.3, after.3);
    assert_eq!(other_before.4, after.4);
    assert_eq!(other_before.5, after.5);
    assert_eq!(other_before.6, after.6);
}

#[tokio::test]
async fn replacement_ignores_block_framing_and_preserves_owner_prefix_suffix() {
    let initial =
        "owner prefix\n<!-- omon:lanes:begin -->\nold\n<!-- omon:lanes:end -->\nowner suffix";
    let database = database_with_profile(Some(initial)).await;
    let block_with_framing =
        "file header\n<!-- omon:lanes:begin -->\nnew rules\n<!-- omon:lanes:end -->\nfile footer\n";

    let first = apply_block(database.pool(), "bot-1", block_with_framing)
        .await
        .unwrap();
    let first_prompt = row_bytes(database.pool()).await.2.unwrap();
    let second = apply_block(database.pool(), "bot-1", block_with_framing)
        .await
        .unwrap();
    let second_prompt = row_bytes(database.pool()).await.2.unwrap();

    assert_eq!(
        first_prompt,
        "owner prefix\n<!-- omon:lanes:begin -->\nnew rules\n<!-- omon:lanes:end -->\nowner suffix"
    );
    assert_eq!(first_prompt, second_prompt);
    assert_eq!(first.after.sha256, second.after.sha256);
}

#[tokio::test]
async fn unknown_bot_has_exit_code_two_and_writes_nothing() {
    let database = database_with_profile(None).await;
    let error = apply_block(database.pool(), "missing", BLOCK)
        .await
        .unwrap_err();
    assert_eq!(error, ApplyBlockError::UnknownBotId("missing".into()));
    assert_eq!(error.exit_code(), Some(2));
    assert_eq!(row_bytes(database.pool()).await.2, None);
}

#[tokio::test]
async fn export_returns_complete_profile_as_json() {
    let database = database_with_profile(Some("export-prompt")).await;
    let profile = export_profile(database.pool(), "bot-1")
        .await
        .unwrap()
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&export_json(&profile).unwrap()).unwrap();
    assert_eq!(json["bot_id"], "bot-1");
    assert_eq!(json["system_prompt"], "export-prompt");
    assert_eq!(json["custom_settings_json"], "{\"setting\": 1}");
    assert!(export_profile(database.pool(), "missing")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn malformed_markers_return_error_without_changing_row() {
    let database = database_with_profile(Some("before\n<!-- omon:lanes:begin -->\npartial")).await;
    let before = row_bytes(database.pool()).await;
    let error = apply_block(database.pool(), "bot-1", BLOCK)
        .await
        .unwrap_err();
    assert!(matches!(error, ApplyBlockError::InvalidBlock(_)));
    assert_eq!(row_bytes(database.pool()).await, before);
}

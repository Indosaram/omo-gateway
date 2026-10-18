use omon_gateway::profiles::{
    apply_block, apply_block_with_markers, export_profile, ApplyBlockError, BlockMarkers,
};
use omon_gateway::Database;

const LANES_BEGIN: &str = "<!-- omon:lanes:begin -->";
const LANES_END: &str = "<!-- omon:lanes:end -->";
const AUTONOMY_BEGIN: &str = "<!-- omon:autonomy:begin -->";

fn shipped_block(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bot_profiles/blocks")
        .join(format!("{name}.md"));
    std::fs::read_to_string(path).unwrap()
}

fn lanes_section(prompt: &str) -> &str {
    let start = prompt.find(LANES_BEGIN).expect("lanes begin marker");
    let end = prompt.find(LANES_END).expect("lanes end marker") + LANES_END.len();
    &prompt[start..end]
}

async fn database_with_prompt(prompt: &str) -> Database {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets)
         VALUES ('bot-1', 'Test', 'model-x', ?, '[]')",
    )
    .bind(prompt)
    .execute(database.pool())
    .await
    .unwrap();
    database
}

async fn prompt_of(database: &Database) -> String {
    export_profile(database.pool(), "bot-1")
        .await
        .unwrap()
        .unwrap()
        .system_prompt
        .unwrap()
}

#[tokio::test]
async fn applying_autonomy_keeps_lanes_block_byte_identical() {
    // Given: a persona with owner text and the shipped omon:lanes block.
    let database = database_with_prompt("owner persona\n").await;
    apply_block(database.pool(), "bot-1", &shipped_block("lanes"))
        .await
        .unwrap();
    let before = prompt_of(&database).await;
    let markers = BlockMarkers::named("autonomy").unwrap();

    // When: the omon:autonomy block is applied, then replaced with new content.
    apply_block_with_markers(
        database.pool(),
        "bot-1",
        &markers,
        &shipped_block("autonomy"),
    )
    .await
    .unwrap();
    let after_insert = prompt_of(&database).await;
    let replacement = "<!-- omon:autonomy:begin -->\nreplaced\n<!-- omon:autonomy:end -->";
    apply_block_with_markers(database.pool(), "bot-1", &markers, replacement)
        .await
        .unwrap();
    let after_replace = prompt_of(&database).await;

    // Then: everything before the autonomy block, lanes included, is untouched.
    for after in [&after_insert, &after_replace] {
        assert_eq!(lanes_section(after), lanes_section(&before));
        assert!(after.starts_with(&before));
        assert_eq!(after.matches(AUTONOMY_BEGIN).count(), 1);
    }
    assert!(after_replace.ends_with(replacement));
}

#[tokio::test]
async fn reapplying_autonomy_block_is_idempotent() {
    // Given: a persona that already carries both blocks.
    let database = database_with_prompt("owner persona").await;
    apply_block(database.pool(), "bot-1", &shipped_block("lanes"))
        .await
        .unwrap();
    let markers = BlockMarkers::named("autonomy").unwrap();
    let block = shipped_block("autonomy");
    let first = apply_block_with_markers(database.pool(), "bot-1", &markers, &block)
        .await
        .unwrap();
    let first_prompt = prompt_of(&database).await;

    // When: the same autonomy block is applied again.
    let second = apply_block_with_markers(database.pool(), "bot-1", &markers, &block)
        .await
        .unwrap();

    // Then: the prompt and its digest do not change.
    assert_eq!(second.before, first.after);
    assert_eq!(second.after, first.after);
    assert_eq!(prompt_of(&database).await, first_prompt);
    assert_eq!(first_prompt.matches(AUTONOMY_BEGIN).count(), 1);
    assert_eq!(first_prompt.matches(LANES_BEGIN).count(), 1);
}

#[tokio::test]
async fn block_with_other_markers_is_rejected() {
    // Given: the lanes block passed under the autonomy name.
    let database = database_with_prompt("owner persona").await;
    let markers = BlockMarkers::named("autonomy").unwrap();

    // When / Then: it is rejected and the prompt is unchanged.
    let error =
        apply_block_with_markers(database.pool(), "bot-1", &markers, &shipped_block("lanes"))
            .await
            .unwrap_err();
    assert!(matches!(error, ApplyBlockError::InvalidBlock(_)));
    assert_eq!(prompt_of(&database).await, "owner persona");
    assert!(BlockMarkers::named("../lanes").is_err());
}

#[tokio::test]
async fn cli_block_flag_applies_named_block_and_keeps_lanes() {
    // Given: a scratch database with a persona carrying the lanes block.
    let scratch = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", scratch.path().join("profiles.db").display());
    let database = Database::connect(&url).await.unwrap();
    sqlx::query(
        "INSERT INTO bot_profiles (bot_id, name, model, system_prompt, enabled_toolsets)
         VALUES ('bot-1', 'Test', 'model-x', 'owner persona', '[]')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    apply_block(database.pool(), "bot-1", &shipped_block("lanes"))
        .await
        .unwrap();
    let before = prompt_of(&database).await;
    let block_file =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bot_profiles/blocks/autonomy.md");

    // When: the CLI applies the autonomy block twice with --block.
    let invoke = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_omo-gateway"))
            .current_dir(scratch.path())
            .env("DATABASE_URL", &url)
            .args([
                "profile",
                "apply-block",
                "--bot-id",
                "bot-1",
                "--block",
                "autonomy",
            ])
            .arg("--block-file")
            .arg(&block_file)
            .output()
            .unwrap()
    };
    let first = invoke();
    let second = invoke();

    // Then: both succeed, the second is a no-op, and lanes is untouched.
    assert!(first.status.success(), "{first:?}");
    assert!(second.status.success(), "{second:?}");
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(first_json["after"], second_json["before"]);
    assert_eq!(first_json["after"], second_json["after"]);
    let after = prompt_of(&database).await;
    assert_eq!(lanes_section(&after), lanes_section(&before));
    assert_eq!(after.matches(AUTONOMY_BEGIN).count(), 1);
}

use omon_gateway::agent::agent_workspace::{
    ensure_workspace_permission_preset, WORKSPACE_PERMISSION_PRESET,
};
use serde_json::{json, Value};

#[tokio::test]
async fn provisioning_adds_permission_preset_and_keeps_default_model() {
    // Given: a bot workspace settings.json that already carries a default model
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "defaultProvider": "mahoquot",
            "defaultModel": "nekos-claude-sonnet-5-5"
        }))
        .unwrap(),
    )
    .unwrap();

    // When: provisioning runs with the approval ledger enabled
    let wrote = ensure_workspace_permission_preset(&path, true, WORKSPACE_PERMISSION_PRESET).await;

    // Then: the preset is merged in and the existing keys survive
    assert!(wrote);
    let settings: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(settings["permissionPreset"], WORKSPACE_PERMISSION_PRESET);
    assert_eq!(settings["defaultModel"], "nekos-claude-sonnet-5-5");
    assert_eq!(settings["defaultProvider"], "mahoquot");
    assert!(!dir.path().join("settings.json.tmp").exists());

    // And: a second run leaves an already-correct file alone
    assert!(!ensure_workspace_permission_preset(&path, true, WORKSPACE_PERMISSION_PRESET).await);
}

#[tokio::test]
async fn provisioning_with_flag_false_leaves_file_unchanged() {
    // Given: an existing settings.json without a preset
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let original = "{\n  \"defaultModel\": \"nekos-claude-sonnet-5-5\"\n}";
    std::fs::write(&path, original).unwrap();

    // When: provisioning runs with the approval ledger disabled
    let wrote = ensure_workspace_permission_preset(&path, false, WORKSPACE_PERMISSION_PRESET).await;

    // Then: the file is byte-for-byte unchanged, and a missing file is not created either
    assert!(!wrote);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    let missing = dir.path().join("absent.json");
    assert!(
        !ensure_workspace_permission_preset(&missing, false, WORKSPACE_PERMISSION_PRESET).await
    );
    assert!(!missing.exists());
}

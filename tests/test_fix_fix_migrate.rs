use chrono::{TimeZone, Utc};
use omon_gateway::cron::{HermesStore, HermesStoreSynchronizer};
use omon_gateway::migrate::cron_cutover::cutover_cron_stores;
use omon_gateway::migrate::sys::{FakeMigrationEnv, MigrationEnv, OsEnv};
use omon_gateway::migrate::{run_migrate_with, MigrateArgs, MigrationPaths};
use omon_gateway::Database;
use std::fs;

const INVALID_JOBS: &str = r#"{"jobs":[{"id":"broken","name":"Broken","prompt":"status","schedule":{"kind":"cron","expr":"not a cron"},"enabled":true}],"updated_at":"old"}"#;

#[tokio::test]
async fn cutover_validation_failure_leaves_hermes_gateway_running() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("hermes");
    let launch_agents = dir.path().join("LaunchAgents");
    // Synchronizer reads the real filesystem; cutover reads the fake env.
    fs::create_dir_all(root.join("cron")).unwrap();
    fs::write(root.join("cron/jobs.json"), INVALID_JOBS).unwrap();

    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    env.write(&root.join("config.yaml"), b"model:\n  default: gpt-4o\n")
        .unwrap();
    env.write(&root.join(".env"), b"DISCORD_BOT_TOKEN=bot-secret\n")
        .unwrap();
    env.write(&root.join("cron/jobs.json"), INVALID_JOBS.as_bytes())
        .unwrap();
    env.write(
        &root.join("gateway.lock"),
        br#"{"pid":4242,"kind":"hermes-gateway","start_time":1}"#,
    )
    .unwrap();
    env.set_pid_alive(4242, true);
    env.set_process_start_time(4242, 1);
    env.set_process_command_line(4242, "python3 -m hermes_cli.main gateway run");
    env.set_current_uid(501);
    let plist = launch_agents.join("ai.hermes.gateway.plist");
    env.write(&plist, b"<plist/>").unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();

    let error = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: false,
        },
        &env,
        MigrationPaths {
            hermes_root: root.clone(),
            target_env: dir.path().join("gateway.env"),
            launch_agents_dir: launch_agents.clone(),
        },
        Some(database.pool().clone()),
    )
    .await
    .unwrap_err();

    println!("validation error={error}");
    assert!(error.to_string().contains("cron cutover"));
    assert!(
        env.terminate_calls().is_empty(),
        "Hermes must not be stopped"
    );
    assert!(env.kill_calls().is_empty(), "Hermes must not be killed");
    assert!(
        env.launchctl_calls().is_empty(),
        "LaunchAgent must not be booted out"
    );
    assert!(env.exists(&plist), "plist must not be disabled");
    assert_eq!(
        env.read_to_string(&root.join("cron/jobs.json")).unwrap(),
        INVALID_JOBS
    );
}

#[tokio::test]
async fn cutover_accepts_jobs_imported_with_config_timezone_and_home_channel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("cron")).unwrap();
    fs::write(
        root.join("config.yaml"),
        "timezone: Asia/Seoul\ndiscord:\n  home_channel: \"123456\"\n  thread_id: \"789\"\n",
    )
    .unwrap();
    fs::write(
        root.join("cron/jobs.json"),
        r#"{"jobs":[{"id":"daily","name":"Daily","prompt":"status","schedule":{"kind":"cron","expr":"0 9 * * *"},"enabled":true}],"updated_at":"old"}"#,
    )
    .unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let synchronizer = HermesStoreSynchronizer::new(
        database.pool().clone(),
        vec![HermesStore::new("default", root)],
    );
    assert_eq!(synchronizer.sync().await.unwrap(), 1);
    let payload: String =
        sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(payload.contains("Asia/Seoul"));
    assert!(payload.contains("_omon_discord_home_channel"));
    assert!(payload.contains("_omon_discord_home_thread"));

    let summary = cutover_cron_stores(&OsEnv, root, database.pool(), true)
        .await
        .unwrap();

    println!("dry-run summary={summary:?}");
    assert_eq!(summary.stores[0].jobs_found, 1);
    assert!(summary.stores[0].unverified_jobs.is_empty());
    assert!(summary.stores[0].all_imported);
    database.close().await;
}

#[tokio::test]
async fn cutover_still_rejects_changed_source_timezone() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("cron")).unwrap();
    fs::write(root.join("config.yaml"), "timezone: Asia/Seoul\n").unwrap();
    fs::write(
        root.join("cron/jobs.json"),
        r#"{"jobs":[{"id":"daily","prompt":"status","schedule":{"kind":"cron","expr":"0 9 * * *"}}],"updated_at":"old"}"#,
    )
    .unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let synchronizer = HermesStoreSynchronizer::new(
        database.pool().clone(),
        vec![HermesStore::new("default", root)],
    );
    synchronizer.sync().await.unwrap();
    fs::write(
        root.join("cron/jobs.json"),
        r#"{"jobs":[{"id":"daily","prompt":"status","schedule":{"kind":"cron","expr":"0 9 * * *","timezone":"UTC"}}],"updated_at":"new"}"#,
    )
    .unwrap();

    let summary = cutover_cron_stores(&OsEnv, root, database.pool(), true)
        .await
        .unwrap();

    assert_eq!(summary.stores[0].unverified_jobs, ["daily"]);
    database.close().await;
}

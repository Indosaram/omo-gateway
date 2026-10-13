use chrono::{TimeZone, Utc};
use omon_gateway::migrate::sys::{FakeMigrationEnv, MigrationEnv};
use omon_gateway::migrate::{run_migrate_with, CronJobRejection, MigrateArgs, MigrationPaths};
use omon_gateway::Database;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);
static SERIAL_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[test]
#[cfg(unix)]
fn secret_files_are_private_and_backup_unique() {
    use omon_gateway::migrate::config_import::import_config;
    use omon_gateway::migrate::sys::OsEnv;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let target = root.join("gateway.env");
    let original = b"DEFAULT_MODEL=old\n";
    fs::write(
        root.join("config.yaml"),
        "model:\n  default: new\n  api_key: sentinel-private\n",
    )
    .unwrap();
    fs::write(&target, original).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let old_inode = root.join("old-inode");
    fs::hard_link(&target, &old_inode).unwrap();
    let imported = import_config(&OsEnv, root, &target, false).unwrap();
    let backup = imported.backup_path.unwrap();
    let target_mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    let backup_mode = fs::metadata(&backup).unwrap().permissions().mode() & 0o777;
    let atomic = fs::read(&old_inode).unwrap() == original;

    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap());
    env.write(&root.join("config.yaml"), b"model:\n  default: new\n")
        .unwrap();
    env.write(&target, original).unwrap();
    let first = import_config(&env, root, &target, false)
        .unwrap()
        .backup_path
        .unwrap();
    let intermediate = env.read(&target).unwrap();
    let second = import_config(&env, root, &target, false)
        .unwrap()
        .backup_path
        .unwrap();
    let unique = first != second
        && env.read(&first).unwrap() == original
        && env.read(&second).unwrap() == intermediate;
    env.set_read_only(&target, true);
    let failed = import_config(&env, root, &target, false).is_err();
    let intact = env.read(&target).unwrap() == intermediate;
    let no_temps = env
        .read_dir(root)
        .unwrap()
        .iter()
        .all(|p| !p.to_string_lossy().contains(".tmp-omon-migration-"));
    println!("C05 target_mode={target_mode:o} backup_mode={backup_mode:o} atomic={atomic} unique={unique} failed={failed} intact={intact} no_temps={no_temps}");
    assert!(
        target_mode == 0o600
            && backup_mode == 0o600
            && atomic
            && unique
            && failed
            && intact
            && no_temps
    );
}

const JOBS: &str = r#"{"jobs":[{"id":"daily","name":"Daily","prompt":"status","schedule":{"kind":"cron","expr":"0 9 * * *"},"enabled":true}],"updated_at":"old"}"#;

#[tokio::test]
#[cfg(unix)]
async fn private_migration_os_surface_controls() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    use omon_gateway::migrate::sys::OsEnv;
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join("cron")).unwrap();
    fs::write(root.join("cron/jobs.json"), JOBS).unwrap();
    fs::write(
        root.join("config.yaml"),
        "model:\n  default: new\n  api_key: sentinel\n",
    )
    .unwrap();
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: false,
        },
        &OsEnv,
        MigrationPaths {
            hermes_root: root.to_owned(),
            target_env: root.join("gateway.env"),
            launch_agents_dir: root.join("absent-agents"),
        },
        Some(database.pool().clone()),
    )
    .await
    .unwrap();
    assert_eq!(summary.cron_imported, 1);
    assert!(summary.pids_stopped.is_empty());
    let cron_backup = fs::read_dir(root.join("cron"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains(".bak-omon-migration-"))
        .unwrap();
    assert_eq!(fs::read(&cron_backup).unwrap(), JOBS.as_bytes());
    for path in [
        root.join("gateway.env"),
        root.join("cron/jobs.json"),
        cron_backup,
    ] {
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let occupied = root.join("occupied");
    let sentinel = root.join("sentinel");
    fs::write(&sentinel, b"untouched").unwrap();
    symlink(&sentinel, &occupied).unwrap();
    let first = OsEnv.write_unique(&occupied, b"first").unwrap();
    let second = OsEnv.write_unique(&occupied, b"second").unwrap();
    assert_ne!(first, second);
    assert_eq!(fs::read(first).unwrap(), b"first");
    assert_eq!(fs::read(second).unwrap(), b"second");
    assert_eq!(fs::read(&sentinel).unwrap(), b"untouched");
    let blocked = root.join("blocked");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("original"), b"original").unwrap();
    assert!(OsEnv.write_atomic(&blocked, b"replacement").is_err());
    assert_eq!(fs::read(blocked.join("original")).unwrap(), b"original");
    for directory in [root.to_owned(), root.join("cron")] {
        assert!(fs::read_dir(directory).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-omon-migration-")));
    }
    database.pool().close().await;
    dir.close().unwrap();
    println!("OsEnv full migration: private config/cron/backup; exclusive symlink collision; failed rename cleanup; fixture removed");
}

struct Fixture {
    root: PathBuf,
    target_env: PathBuf,
    launch_agents: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("omon-migrate-{}-{nonce}", std::process::id()));
        let target_env = root.join("gateway.env");
        let launch_agents = root.join("Library/LaunchAgents");
        fs::create_dir_all(root.join("cron")).unwrap();
        fs::write(root.join("cron/jobs.json"), JOBS).unwrap();
        Self {
            root,
            target_env,
            launch_agents,
        }
    }

    fn paths(&self) -> MigrationPaths {
        MigrationPaths {
            hermes_root: self.root.clone(),
            target_env: self.target_env.clone(),
            launch_agents_dir: self.launch_agents.clone(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn seed_fake(env: &FakeMigrationEnv, fixture: &Fixture, include_cutover: bool) {
    env.write(
        &fixture.root.join("config.yaml"),
        b"model:\n  default: gpt-4o\n  base_url: https://example.test/v1\n  api_key: provider-secret\napprovals:\n  mode: smart\n",
    )
    .unwrap();
    env.write(
        &fixture.root.join(".env"),
        b"DISCORD_BOT_TOKEN=bot-secret\nDISCORD_ALLOWED_USERS=42\n",
    )
    .unwrap();
    env.write(&fixture.root.join("cron/jobs.json"), JOBS.as_bytes())
        .unwrap();
    env.write(
        &fixture.target_env,
        b"DATABASE_URL=sqlite://custom.db\nOMON_WORKSPACE_ROOT=/x\nDEFAULT_MODEL=old\n",
    )
    .unwrap();

    if include_cutover {
        env.write(
            &fixture.root.join("gateway.lock"),
            br#"{"pid":4242,"kind":"hermes-gateway","start_time":1}"#,
        )
        .unwrap();
        env.set_pid_alive(4242, true);
        env.set_process_start_time(4242, 1);
        env.set_process_command_line(4242, "python3 -m hermes_cli.main gateway run");
        env.set_current_uid(501);
        env.write(
            &fixture.launch_agents.join("ai.hermes.gateway.plist"),
            b"<plist/>",
        )
        .unwrap();
    }
}

#[tokio::test]
async fn full_migration_imports_config_and_cron_before_cutover() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);
    let database = Database::connect("sqlite::memory:").await.unwrap();

    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: false,
        },
        &env,
        fixture.paths(),
        Some(database.pool().clone()),
    )
    .await
    .unwrap();

    let migrated_env = env.read_to_string(&fixture.target_env).unwrap();
    assert_eq!(
        migrated_env,
        "DATABASE_URL=sqlite://custom.db\nOMON_WORKSPACE_ROOT=/x\nDEFAULT_MODEL=gpt-4o\nAPPROVAL_MODE=smart\nDISCORD_ALLOWED_USERS=42\nDISCORD_BOT_TOKEN=bot-secret\nOPENAI_API_BASE=https://example.test/v1\nOPENAI_API_KEY=provider-secret\n"
    );
    assert_eq!(
        env.read_to_string(
            &fixture
                .target_env
                .with_file_name("gateway.env.bak-20260815T150000Z")
        )
        .unwrap(),
        "DATABASE_URL=sqlite://custom.db\nOMON_WORKSPACE_ROOT=/x\nDEFAULT_MODEL=old\n"
    );
    assert!(summary.config_diff.contains("= DATABASE_URL="));
    assert!(summary.config_diff.contains("= OMON_WORKSPACE_ROOT="));
    assert!(!summary
        .config_diff
        .lines()
        .any(|line| line.starts_with("- ")));

    let imported: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(imported, 1);

    let emptied: serde_json::Value =
        serde_json::from_slice(&env.read(&fixture.root.join("cron/jobs.json")).unwrap()).unwrap();
    assert_eq!(emptied["jobs"], serde_json::json!([]));
    assert!(env.write_calls().iter().any(|(path, bytes)| path
        .to_string_lossy()
        .contains("bak-omon-migration-")
        && bytes == JOBS.as_bytes()));
    assert_eq!(env.terminate_calls(), [4242]);
    assert_eq!(env.kill_calls(), [4242]);
    assert_eq!(env.launchctl_calls().len(), 1);
    assert!(env.rename_calls().iter().any(|(_, to)| to
        == &fixture
            .launch_agents
            .join("ai.hermes.gateway.plist.disabled")));
    assert_eq!(summary.cron_imported, 1);
    assert_eq!(summary.cron_stores[0].jobs_found, 1);
    assert_eq!(summary.pids_stopped, vec![4242]);
}

#[tokio::test]
async fn dry_run_projects_every_step_with_zero_writes_or_side_effects() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);
    let database = Database::connect("sqlite::memory:").await.unwrap();
    let writes_before = env.write_calls();
    let renames_before = env.rename_calls();

    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: true,
            no_cutover: false,
        },
        &env,
        fixture.paths(),
        Some(database.pool().clone()),
    )
    .await
    .unwrap();

    assert_eq!(env.write_calls(), writes_before);
    assert_eq!(env.rename_calls(), renames_before);
    assert!(env.terminate_calls().is_empty());
    assert!(env.kill_calls().is_empty());
    assert!(env.launchctl_calls().is_empty());
    assert_eq!(
        env.read_to_string(&fixture.root.join("cron/jobs.json"))
            .unwrap(),
        JOBS
    );
    let imported: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(imported, 0, "dry-run must not invoke sync()");
    assert_eq!(summary.cron_importable, vec!["hermes:default:daily"]);
    assert!(summary.cron_already_present.is_empty());
    assert_eq!(summary.cron_stores[0].unverified_jobs, ["daily"]);
    assert_eq!(summary.pids_stopped, vec![4242]);
    assert_eq!(summary.plists_disabled.len(), 1);
}

#[tokio::test]
async fn dry_run_without_database_reports_creation_without_creating_it() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, false);
    let writes_before = env.write_calls();

    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: true,
            no_cutover: true,
        },
        &env,
        fixture.paths(),
        None,
    )
    .await
    .unwrap();

    assert!(summary.database_would_be_created);
    assert_eq!(summary.cron_importable, vec!["hermes:default:daily"]);
    assert_eq!(env.write_calls(), writes_before);
    assert!(!Path::new(&fixture.root.join("missing.db")).exists());
}

#[tokio::test]
async fn cutover_survives_startup_sync() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    use omon_gateway::cron::{
        CronAuthority, CronScheduler, HermesStore, HermesStoreSynchronizer, PayloadTaskExecutor,
    };
    use std::sync::Arc;

    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);
    let database = Database::connect("sqlite::memory:").await.unwrap();

    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: false,
        },
        &env,
        fixture.paths(),
        Some(database.pool().clone()),
    )
    .await
    .unwrap();

    assert_eq!(summary.cron_imported, 1);
    let imported_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(imported_count, 1, "job must be present after migration");

    // Verify authority is omon_owned after successful cutover
    let authority: String =
        sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(
        authority, "omon_owned",
        "cutover job must have omon_owned authority"
    );

    // Verify source provenance is strictly retained
    let payload: String =
        sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(
        payload.contains("_omon_hermes_source"),
        "_omon_hermes_source provenance must not be erased"
    );
    assert!(payload.contains("status"), "payload prompt must be status");

    // Cutover emptied the store in migration env; persist empty store to fixture file for production synchronizer
    let emptied = env.read(&fixture.root.join("cron/jobs.json")).unwrap();
    fs::write(fixture.root.join("cron/jobs.json"), &emptied).unwrap();

    // Production synchronizer runs on startup against the emptied store
    let synchronizer = HermesStoreSynchronizer::new(
        database.pool().clone(),
        vec![HermesStore::new("default", &fixture.root)],
    );

    // Startup sync 1
    let sync1_imported = synchronizer.sync().await.unwrap();
    assert_eq!(sync1_imported, 0);
    let count_after_sync1: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    println!("C01 RED check: count after migrate = {imported_count}, count after startup sync = {count_after_sync1}");
    assert_eq!(
        count_after_sync1, 1,
        "cutover owned job must survive first startup sync (RED: count 1 -> 0)"
    );

    // Startup sync 2 (TWO exact startup syncs retaining owned job)
    let sync2_imported = synchronizer.sync().await.unwrap();
    assert_eq!(sync2_imported, 0);
    let count_after_sync2: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(
        count_after_sync2, 1,
        "cutover owned job must survive second startup sync"
    );

    // Verify live source sync cannot overwrite omon_owned job
    fs::write(
        fixture.root.join("cron/jobs.json"),
        br#"{"jobs":[{"id":"daily","prompt":"attempted-overwrite","schedule":{"kind":"cron","expr":"0 9 * * *"}}],"updated_at":"new"}"#,
    )
    .unwrap();
    let sync_overwrite = synchronizer.sync().await.unwrap();
    assert_eq!(
        sync_overwrite, 0,
        "live sync must not overwrite omon_owned row"
    );
    let payload_after_sync: String =
        sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(
        payload_after_sync.contains("status"),
        "payload must not be overwritten by live sync"
    );
    // Restore empty store
    fs::write(fixture.root.join("cron/jobs.json"), &emptied).unwrap();

    // Actual due-job claim via scheduler (no Discord). The clock is pinned so the
    // 09:00 occurrence is inside the catch-up window regardless of wall time.
    let fixed_now = Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 30).unwrap();
    let past = Utc.with_ymd_and_hms(2026, 1, 15, 8, 59, 0).unwrap();
    sqlx::query("UPDATE cron_jobs SET next_run_at = ? WHERE id = 'hermes:default:daily'")
        .bind(past)
        .execute(database.pool())
        .await
        .unwrap();

    let scheduler = CronScheduler::new(database.pool().clone(), Arc::new(PayloadTaskExecutor))
        .with_clock(move || fixed_now);
    let claimed = scheduler.run_due_jobs().await.unwrap();
    assert_eq!(
        claimed, 1,
        "due cutover owned job must be claimed by scheduler"
    );

    // Verify pending authority blocks scheduler claims atomically (scheduled + manual trigger)
    scheduler
        .set_authority("hermes:default:daily", CronAuthority::CutoverPending)
        .await
        .unwrap();
    assert_eq!(
        scheduler
            .get_authority("hermes:default:daily")
            .await
            .unwrap(),
        Some(CronAuthority::CutoverPending)
    );

    // Scheduled run must ignore cutover_pending
    sqlx::query("UPDATE cron_jobs SET next_run_at = ? WHERE id = 'hermes:default:daily'")
        .bind(past)
        .execute(database.pool())
        .await
        .unwrap();
    // Also complete or clear any running run from previous claim so status is not blocked by active lease
    sqlx::query("UPDATE cron_runs SET status = 'succeeded' WHERE job_id = 'hermes:default:daily'")
        .execute(database.pool())
        .await
        .unwrap();

    let claimed_pending = scheduler.run_due_jobs().await.unwrap();
    assert_eq!(
        claimed_pending, 0,
        "pending authority must block scheduled claims atomically"
    );

    // Manual trigger must also be blocked by cutover_pending
    let triggered_pending = scheduler.trigger_job("hermes:default:daily").await.unwrap();
    assert!(
        !triggered_pending,
        "pending authority must block manual trigger atomically"
    );

    // Restoring to omon_owned unblocks manual trigger
    scheduler
        .set_authority("hermes:default:daily", CronAuthority::OmonOwned)
        .await
        .unwrap();
    let triggered_owned = scheduler.trigger_job("hermes:default:daily").await.unwrap();
    assert!(triggered_owned, "omon_owned job unblocks manual trigger");

    database.close().await;
}

#[tokio::test]
async fn changed_payload_blocks_cutover() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);
    let database = Database::connect("sqlite::memory:").await.unwrap();

    // 1. Initial state: DB has daily job with prompt "old"
    sqlx::query(
        "INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority)
         VALUES ('hermes:default:daily', '0 9 * * *', '{\"id\":\"daily\",\"name\":\"Daily\",\"prompt\":\"old\",\"schedule\":{\"kind\":\"cron\",\"expr\":\"0 9 * * *\"}}', 1, 'hermes_mirror')",
    )
    .execute(database.pool())
    .await
    .unwrap();

    // 2. Source store has same ID "daily", but prompt "new" before cutover read
    let new_source = r#"{"jobs":[{"id":"daily","name":"Daily","prompt":"new","schedule":{"kind":"cron","expr":"0 9 * * *"},"enabled":true}],"updated_at":"new"}"#;
    env.write(&fixture.root.join("cron/jobs.json"), new_source.as_bytes())
        .unwrap();

    // 3. Perform cutover
    let cutover_result = omon_gateway::migrate::cron_cutover::cutover_cron_stores(
        &env,
        &fixture.root,
        database.pool(),
        false,
    )
    .await;

    // GREEN: cutover must refuse due to changed payload without mutating source
    assert!(
        cutover_result.is_err(),
        "cutover must refuse on changed payload"
    );
    let current_source = env
        .read_to_string(&fixture.root.join("cron/jobs.json"))
        .unwrap();
    assert_eq!(
        current_source, new_source,
        "source store must be retained intact on payload mismatch (RED: emptied source with old DB)"
    );
    let db_payload: String =
        sqlx::query_scalar("SELECT payload_json FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(db_payload.contains("old"));
}

#[tokio::test]
async fn cutover_second_store_failure_preserves_bytes_and_receipt() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    use omon_gateway::cron::{CronScheduler, PayloadTaskExecutor};
    use omon_gateway::migrate::cron_cutover::reconcile_pending_cutover;
    use omon_gateway::migrate::sys::MigrationOperation;
    use std::sync::Arc;

    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);
    let database = Database::connect("sqlite::memory:").await.unwrap();

    // 1. Setup two stores: default and work profile
    let work_store = fixture.root.join("profiles/work/cron/jobs.json");
    let work_jobs = r#"{"jobs":[{"id":"standup","name":"Standup","prompt":"standup-check","schedule":{"kind":"cron","expr":"0 10 * * *"},"enabled":true}],"updated_at":"old"}"#;
    env.write(&work_store, work_jobs.as_bytes()).unwrap();

    // Seed DB with matching canonical payloads for both jobs
    sqlx::query(
        "INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority)
         VALUES ('hermes:default:daily', '0 9 * * *', '{\"id\":\"daily\",\"name\":\"Daily\",\"prompt\":\"status\",\"schedule\":{\"kind\":\"cron\",\"expr\":\"0 9 * * *\"},\"enabled\":true}', 1, 'hermes_mirror')",
    )
    .execute(database.pool())
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority)
         VALUES ('hermes:work:standup', '0 10 * * *', '{\"id\":\"standup\",\"name\":\"Standup\",\"prompt\":\"standup-check\",\"schedule\":{\"kind\":\"cron\",\"expr\":\"0 10 * * *\"},\"enabled\":true}', 1, 'hermes_mirror')",
    )
    .execute(database.pool())
    .await
    .unwrap();

    // 2. Inject failure on second store rewrite by making work_store path read-only
    env.set_read_only(&work_store, true);

    // 3. Run full cutover via run_migrate_with
    let result = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: false,
        },
        &env,
        fixture.paths(),
        Some(database.pool().clone()),
    )
    .await;

    assert!(
        result.is_err(),
        "cutover must fail on injected second-store failure"
    );

    // 4. Assert operation ordering in FakeMigrationEnv
    let operations = env.operations();
    let bootout_idx = operations
        .iter()
        .position(|op| matches!(op, MigrationOperation::Bootout(_)))
        .expect("gateway bootout");
    let term_idx = operations
        .iter()
        .position(|op| matches!(op, MigrationOperation::Terminate(4242)))
        .expect("process terminate");

    let default_lock = fixture.root.join("cron/.jobs.lock");
    let work_lock = fixture.root.join("profiles/work/cron/.jobs.lock");

    let lock1_idx = operations
        .iter()
        .position(|op| op == &MigrationOperation::LockAcquired(default_lock.clone()))
        .expect("default lock acquired");
    let lock2_idx = operations
        .iter()
        .position(|op| op == &MigrationOperation::LockAcquired(work_lock.clone()))
        .expect("work lock acquired");

    // Quiesce precedes locks
    assert!(bootout_idx < lock1_idx);
    assert!(term_idx < lock1_idx);
    // Canonical sorted lock acquisition
    assert!(
        lock1_idx < lock2_idx,
        "locks must be acquired in canonical sorted order"
    );

    // Assert backups for ALL stores are prepared before the first source rewrite
    let default_backup_idx = operations
        .iter()
        .position(|op| {
            matches!(op, MigrationOperation::Write(p) if p.to_string_lossy().replace('\\', "/").contains("cron/jobs.json.bak-omon-migration-"))
        })
        .expect("default backup written");
    let work_backup_idx = operations
        .iter()
        .position(|op| {
            matches!(op, MigrationOperation::Write(p) if p.to_string_lossy().replace('\\', "/").contains("profiles/work/cron/jobs.json.bak-omon-migration-"))
        })
        .expect("work backup written");

    let first_rename_idx = operations
        .iter()
        .position(|op| {
            matches!(op, MigrationOperation::Rename(_, to) if to.to_string_lossy().replace('\\', "/").ends_with("cron/jobs.json"))
        })
        .expect("first source rename");

    assert!(
        default_backup_idx < first_rename_idx,
        "default backup must precede first rewrite"
    );
    assert!(
        work_backup_idx < first_rename_idx,
        "all backups must be prepared before first rewrite"
    );

    // Assert locks released after failure
    let release1_idx = operations
        .iter()
        .position(|op| op == &MigrationOperation::LockReleased(default_lock.clone()))
        .expect("default lock released");
    let release2_idx = operations
        .iter()
        .position(|op| op == &MigrationOperation::LockReleased(work_lock.clone()))
        .expect("work lock released");
    assert!(first_rename_idx < release1_idx);
    assert!(first_rename_idx < release2_idx);

    // 5. Assert every store's bytes on injected second-store failure
    let default_current: serde_json::Value =
        serde_json::from_slice(&env.read(&fixture.root.join("cron/jobs.json")).unwrap()).unwrap();
    assert_eq!(
        default_current["jobs"],
        serde_json::json!([]),
        "first store was replaced"
    );

    let work_current = env.read_to_string(&work_store).unwrap();
    assert_eq!(
        work_current, work_jobs,
        "second store bytes must remain unmutated on injected failure"
    );

    // Verify backups exist and match original bytes for both stores
    let default_backup = env
        .write_calls()
        .into_iter()
        .find(|(p, _)| {
            p.to_string_lossy()
                .replace('\\', "/")
                .contains("cron/jobs.json.bak-omon-migration-")
        })
        .expect("default backup");
    assert_eq!(default_backup.1, JOBS.as_bytes());

    let work_backup = env
        .write_calls()
        .into_iter()
        .find(|(p, _)| {
            p.to_string_lossy()
                .replace('\\', "/")
                .contains("profiles/work/cron/jobs.json.bak-omon-migration-")
        })
        .expect("work backup");
    assert_eq!(work_backup.1, work_jobs.as_bytes());

    // 6. Assert durable pending receipt in SQLite
    let receipt_status: String =
        sqlx::query_scalar("SELECT status FROM cron_cutover_receipts LIMIT 1")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(receipt_status, "pending");

    let default_phase: String = sqlx::query_scalar(
        "SELECT phase FROM cron_cutover_receipt_stores WHERE profile = 'default'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(default_phase, "replaced");

    let work_phase: String =
        sqlx::query_scalar("SELECT phase FROM cron_cutover_receipt_stores WHERE profile = 'work'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(work_phase, "backed_up");

    let daily_auth: String =
        sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(daily_auth, "cutover_pending");

    // 7. Assert scheduler execution blocked while receipt is pending
    // Clock pinned so both the 09:00 (daily) and 10:00 (standup) occurrences are due.
    let fixed_now = Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 30).unwrap();
    let past = Utc.with_ymd_and_hms(2026, 1, 15, 8, 59, 0).unwrap();
    sqlx::query("UPDATE cron_jobs SET next_run_at = ?")
        .bind(past)
        .execute(database.pool())
        .await
        .unwrap();

    let scheduler = CronScheduler::new(database.pool().clone(), Arc::new(PayloadTaskExecutor))
        .with_clock(move || fixed_now);
    let claimed_while_pending = scheduler.run_due_jobs().await.unwrap();
    assert_eq!(
        claimed_while_pending, 0,
        "scheduler must claim zero jobs while cutover receipt is pending"
    );
    let triggered_while_pending = scheduler.trigger_job("hermes:default:daily").await.unwrap();
    assert!(
        !triggered_while_pending,
        "manual trigger blocked while cutover receipt is pending"
    );

    // 8. Reopen twice
    // Reopen 1: scheduler is STILL blocked
    let claimed_reopen1 = scheduler.run_due_jobs().await.unwrap();
    assert_eq!(
        claimed_reopen1, 0,
        "scheduler blocked on reopen 1 before recovery"
    );

    // Unfreeze second store and perform recovery on Reopen 2
    env.set_read_only(&work_store, false);
    let recovered_op = reconcile_pending_cutover(&env, database.pool())
        .await
        .unwrap();
    assert!(
        recovered_op.is_some(),
        "reconcile must complete pending cutover"
    );

    // 9. Post-recovery verification
    let work_recovered: serde_json::Value =
        serde_json::from_slice(&env.read(&work_store).unwrap()).unwrap();
    assert_eq!(
        work_recovered["jobs"],
        serde_json::json!([]),
        "second store emptied after recovery roll-forward"
    );

    let committed_status: String =
        sqlx::query_scalar("SELECT status FROM cron_cutover_receipts LIMIT 1")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(committed_status, "committed");

    let daily_owned: String =
        sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = 'hermes:default:daily'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(daily_owned, "omon_owned");

    let standup_owned: String =
        sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = 'hermes:work:standup'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(standup_owned, "omon_owned");

    // Scheduler now successfully claims due jobs
    let claimed_after_recovery = scheduler.run_due_jobs().await.unwrap();
    assert_eq!(
        claimed_after_recovery, 2,
        "both due jobs claimed after cutover committed"
    );

    database.close().await;
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(val) = &self.original {
            std::env::set_var(self.key, val);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

#[tokio::test]
async fn dry_run_reports_nonimportable_jobs() {
    let _lock = SERIAL_TEST_LOCK.lock().await;
    let fixture = Fixture::new();
    let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 9, 6, 12, 0, 0).unwrap());
    seed_fake(&env, &fixture, true);

    // Setup work profile store
    let work_cron_dir = fixture.root.join("profiles/work/cron");
    fs::create_dir_all(&work_cron_dir).unwrap();
    env.create_dir_all(&work_cron_dir).unwrap();

    // Default store has a job: "daily" (from seed_fake).
    // Work profile has 3 jobs:
    // 1. "bad" - missing schedule expression (only kind specified, no expr)
    // 2. "restart" - forbidden gateway lifecycle prompt
    // 3. "standup" - valid job with valid schedule expression
    let work_jobs = serde_json::json!({
        "jobs": [
            {
                "id": "bad",
                "name": "Bad Schedule Job",
                "prompt": "echo hello",
                "schedule": {
                    "kind": "cron"
                },
                "enabled": true
            },
            {
                "id": "restart",
                "name": "Forbidden Lifecycle Job",
                "prompt": "hermes gateway restart",
                "schedule": {
                    "kind": "cron",
                    "expr": "0 9 * * *"
                },
                "enabled": true
            },
            {
                "id": "standup",
                "name": "Valid Standup Job",
                "prompt": "team standup update",
                "schedule": {
                    "kind": "cron",
                    "expr": "0 9 * * *"
                },
                "enabled": true
            }
        ],
        "updated_at": "2026-09-06T12:00:00Z"
    });
    let work_bytes = serde_json::to_vec(&work_jobs).unwrap();
    fs::write(work_cron_dir.join("jobs.json"), &work_bytes).unwrap();
    env.write(&work_cron_dir.join("jobs.json"), &work_bytes)
        .unwrap();

    // Restrict selected profiles to "work" while default has "daily"
    let _guard = EnvVarGuard::set("OMON_HERMES_PROFILES", "work");

    let database = Database::connect("sqlite::memory:").await.unwrap();
    let writes_before = env.write_calls();
    let renames_before = env.rename_calls();

    // 1. Dry run projection
    let summary = run_migrate_with(
        MigrateArgs {
            dry_run: true,
            no_cutover: false,
        },
        &env,
        fixture.paths(),
        Some(database.pool().clone()),
    )
    .await
    .unwrap();

    // Zero writes / mutations in dry-run
    assert_eq!(
        env.write_calls(),
        writes_before,
        "dry-run must perform zero file writes"
    );
    assert_eq!(
        env.rename_calls(),
        renames_before,
        "dry-run must perform zero file renames"
    );
    assert!(
        env.terminate_calls().is_empty(),
        "dry-run must not terminate processes"
    );
    assert!(
        env.kill_calls().is_empty(),
        "dry-run must not kill processes"
    );
    assert!(
        env.launchctl_calls().is_empty(),
        "dry-run must not execute launchctl"
    );

    let db_job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cron_jobs")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(db_job_count, 0, "dry-run must not write to database");

    // Rejected jobs: bad schedule and forbidden lifecycle prompt must be reported
    assert!(
        summary
            .cron_rejected
            .iter()
            .any(|r: &CronJobRejection| r.job_id == "bad"),
        "job with missing schedule must be reported in cron_rejected, got: {:?}",
        summary.cron_rejected
    );
    assert!(
        summary.cron_rejected.iter().any(|r| r.job_id == "restart"),
        "job with forbidden restart prompt must be reported in cron_rejected, got: {:?}",
        summary.cron_rejected
    );

    // Nonselected jobs: default profile has job 'daily', but only 'work' profile is selected
    assert!(
        summary
            .cron_nonselected
            .iter()
            .any(|id| id.contains("daily")),
        "default profile job must be reported in cron_nonselected, got: {:?}",
        summary.cron_nonselected
    );

    // Importable: only valid selected work job ('standup') must be importable;
    // rejected jobs ('bad', 'restart') and nonselected jobs ('daily') must NOT be importable
    assert_eq!(
        summary.cron_importable,
        vec!["hermes:work:standup"],
        "only valid selected job must be importable, got: {:?}",
        summary.cron_importable
    );

    // Cutover would_empty check:
    // Work store contains rejected jobs -> cutover is unsafe, would_empty MUST be false
    let work_store_summary = summary
        .cron_stores
        .iter()
        .find(|s| s.profile == "work")
        .expect("work store must be present in cron_stores");
    assert!(
        !work_store_summary.would_empty,
        "work store containing invalid/rejected jobs must have would_empty=false"
    );

    // Default store is nonselected -> cutover must not empty it, would_empty MUST be false
    let default_store_summary = summary
        .cron_stores
        .iter()
        .find(|s| s.profile == "default")
        .expect("default store must be present in cron_stores");
    assert!(
        !default_store_summary.would_empty,
        "nonselected default store must have would_empty=false"
    );

    // 2. Import-only check: verify that actual import matches the dry-run validation classifications
    let import_database = Database::connect("sqlite::memory:").await.unwrap();
    let import_summary = run_migrate_with(
        MigrateArgs {
            dry_run: false,
            no_cutover: true,
        },
        &env,
        fixture.paths(),
        Some(import_database.pool().clone()),
    )
    .await
    .unwrap();

    // Actual import only imported 1 job (standup), matching dry-run's cron_importable
    assert_eq!(
        import_summary.cron_imported, 1,
        "import-only must import exactly the valid selected job"
    );
    let imported_rows: Vec<String> = sqlx::query_scalar("SELECT id FROM cron_jobs ORDER BY id")
        .fetch_all(import_database.pool())
        .await
        .unwrap();
    assert_eq!(
        imported_rows,
        vec!["hermes:work:standup"],
        "actual imported jobs in database must exactly match dry-run cron_importable"
    );

    database.close().await;
    import_database.close().await;
}

#[test]
fn provider_override_is_not_silently_dropped() {
    use omon_gateway::migrate::config_import::import_config;

    let env = FakeMigrationEnv::new(chrono::Utc::now());
    env.write(
        Path::new("/hermes/config.yaml"),
        b"model:\n  default: my-model\n  provider: custom:my_provider\n  base_url: http://127.0.0.1:18801/v1\n",
    )
    .unwrap();
    env.write(Path::new("/hermes/.env"), b"DISCORD_BOT_TOKEN=token\n")
        .unwrap();

    let result = import_config(
        &env,
        Path::new("/hermes"),
        Path::new("/gateway/.env"),
        false,
    )
    .unwrap();
    assert_eq!(
        result.values.get("LLM_PROVIDER").map(String::as_str),
        Some("custom:my_provider")
    );
    assert_eq!(
        result.values.get("OPENAI_API_BASE").map(String::as_str),
        Some("http://127.0.0.1:18801/v1")
    );
}

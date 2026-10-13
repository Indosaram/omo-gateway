pub mod config_import;
pub mod cron_cutover;
pub mod gateway_down;
pub mod sys;

use crate::cron::{HermesJob, HermesStore, HermesStoreSynchronizer};
use crate::migrate::config_import::import_config;
use crate::migrate::cron_cutover::{cutover_cron_stores, CronStoreCutoverSummary};
use crate::migrate::gateway_down::bring_gateway_down;
use crate::migrate::sys::{MigrationEnv, OsEnv};
use crate::{Database, OmonError, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(clap::Args, Debug, Clone)]
pub struct MigrateArgs {
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub no_cutover: bool,
}

#[derive(Debug, Clone)]
pub struct MigrationPaths {
    pub hermes_root: PathBuf,
    pub target_env: PathBuf,
    pub launch_agents_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronJobRejection {
    pub id: String,
    pub job_id: String,
    pub profile: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationSummary {
    pub dry_run: bool,
    pub database_would_be_created: bool,
    pub config_keys: usize,
    pub config_tokens: usize,
    pub config_diff: String,
    pub cron_imported: usize,
    pub cron_importable: Vec<String>,
    pub cron_already_present: Vec<String>,
    pub cron_rejected: Vec<CronJobRejection>,
    pub cron_nonselected: Vec<String>,
    pub cron_stores: Vec<CronStoreCutoverSummary>,
    pub pids_stopped: Vec<i32>,
    pub plists_disabled: Vec<PathBuf>,
}

pub async fn run_migrate(args: MigrateArgs) -> Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| OmonError::Config("HOME is required for migration".into()))?;
    let hermes_root = std::env::var_os("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".hermes"));
    let current_dir = std::env::current_dir().map_err(|error| {
        OmonError::Config(format!("failed to determine current directory: {error}"))
    })?;
    let paths = MigrationPaths {
        hermes_root,
        target_env: current_dir.join(".env"),
        launch_agents_dir: home.join("Library").join("LaunchAgents"),
    };
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://omon_gateway.db".to_owned());
    let env = OsEnv;

    let summary = if args.dry_run {
        let pool = open_read_only_pool_if_present(&database_url).await?;
        run_migrate_with(args, &env, paths, pool).await?
    } else {
        // Config import is intentionally performed before opening a writable database so the
        // command's externally visible order is exactly config -> cron import -> cutover.
        let config =
            import_config(&env, &paths.hermes_root, &paths.target_env, false).map_err(|error| {
                step_error(
                    "config import",
                    "cron import, cron cutover, gateway-down",
                    error,
                )
            })?;
        let database = Database::connect(&database_url).await.map_err(|error| {
            step_error(
                "database initialization",
                "cron import, cron cutover, gateway-down",
                error,
            )
        })?;
        let synchronizer = HermesStoreSynchronizer::from_environment(database.pool().clone())
            .map_err(|error| {
                step_error(
                    "cron import setup",
                    "cron import, cron cutover, gateway-down",
                    error,
                )
            })?;
        run_after_config(
            args,
            &env,
            paths,
            database.pool().clone(),
            synchronizer,
            config,
        )
        .await?
    };

    print_summary(&summary);
    Ok(())
}

pub async fn run_migrate_with(
    args: MigrateArgs,
    env: &dyn MigrationEnv,
    paths: MigrationPaths,
    pool: Option<SqlitePool>,
) -> Result<MigrationSummary> {
    let config = import_config(env, &paths.hermes_root, &paths.target_env, args.dry_run).map_err(
        |error| {
            step_error(
                "config import",
                "cron import, cron cutover, gateway-down",
                error,
            )
        },
    )?;

    if args.dry_run {
        return project_migration(args, env, paths, pool, config).await;
    }

    let pool = pool.ok_or_else(|| {
        step_error(
            "database initialization",
            "cron import, cron cutover, gateway-down",
            OmonError::Config("a writable gateway database pool is required".into()),
        )
    })?;
    let stores = discover_hermes_stores(env, &paths.hermes_root)?;
    let synchronizer = HermesStoreSynchronizer::new(pool.clone(), stores);
    run_after_config(args, env, paths, pool, synchronizer, config).await
}

async fn run_after_config(
    args: MigrateArgs,
    env: &dyn MigrationEnv,
    paths: MigrationPaths,
    pool: SqlitePool,
    synchronizer: HermesStoreSynchronizer,
    config: config_import::ConfigImportResult,
) -> Result<MigrationSummary> {
    let cron_imported = synchronizer
        .sync()
        .await
        .map_err(|error| step_error("cron import", "cron cutover, gateway-down", error))?;

    let mut summary = MigrationSummary {
        dry_run: false,
        database_would_be_created: false,
        config_keys: config.values.len(),
        config_tokens: imported_token_count(&config.values),
        config_diff: config.diff,
        cron_imported,
        cron_importable: Vec::new(),
        cron_already_present: Vec::new(),
        cron_rejected: Vec::new(),
        cron_nonselected: Vec::new(),
        cron_stores: Vec::new(),
        pids_stopped: Vec::new(),
        plists_disabled: Vec::new(),
    };
    if args.no_cutover {
        return Ok(summary);
    }

    // Validate every store before touching Hermes processes or LaunchAgents.
    let preflight = cutover_cron_stores(env, &paths.hermes_root, &pool, true)
        .await
        .map_err(|error| step_error("cron cutover", "gateway-down, cron cutover", error))?;
    if let Some(store) = preflight
        .stores
        .iter()
        .find(|store| !store.unverified_jobs.is_empty())
    {
        return Err(step_error(
            "cron cutover",
            "gateway-down, cron cutover",
            OmonError::Config(format!(
                "Hermes cron job hermes:{}:{} payload does not match imported cron_jobs or has not been imported; Hermes gateway left running",
                store.profile, store.unverified_jobs[0]
            )),
        ));
    }

    let gateway = bring_gateway_down(env, &paths.hermes_root, &paths.launch_agents_dir, false)
        .map_err(|error| step_error("gateway-down", "cron cutover", error))?;
    summary.pids_stopped = gateway.pids_terminated;
    summary.plists_disabled = gateway.plists_disabled;

    let cron_cutover = cutover_cron_stores(env, &paths.hermes_root, &pool, false)
        .await
        .map_err(|error| step_error("cron cutover", "none", error))?;
    summary.cron_stores = cron_cutover.stores;

    Ok(summary)
}

async fn project_migration(
    args: MigrateArgs,
    env: &dyn MigrationEnv,
    paths: MigrationPaths,
    pool: Option<SqlitePool>,
    config: config_import::ConfigImportResult,
) -> Result<MigrationSummary> {
    let now = env.now();
    let selected_profiles = HermesStoreSynchronizer::selected_profiles();
    let all_stores = discover_all_hermes_stores(env, &paths.hermes_root)?;

    let mut cron_importable = Vec::new();
    let mut cron_already_present = Vec::new();
    let mut cron_rejected = Vec::new();
    let mut cron_nonselected = Vec::new();

    let existing = existing_cron_ids(pool.as_ref()).await?;

    struct ScannedStore {
        profile: String,
        job_ids: Vec<String>,
        is_selected: bool,
        has_rejected: bool,
    }
    let mut scanned_stores = Vec::new();

    for store in &all_stores {
        let profile = store.profile().to_string();
        let is_selected =
            HermesStoreSynchronizer::is_profile_selected(&profile, selected_profiles.as_deref());
        let jobs_path = store.jobs_path();

        let raw_jobs: Vec<serde_json::Value> = if env.exists(&jobs_path) {
            let bytes = env.read(&jobs_path)?;
            #[derive(Deserialize)]
            struct StoreDoc {
                #[serde(default)]
                jobs: Vec<serde_json::Value>,
            }
            let doc: StoreDoc = serde_json::from_slice(&bytes).map_err(|error| {
                OmonError::Config(format!(
                    "invalid Hermes cron store {}: {error}",
                    jobs_path.display()
                ))
            })?;
            doc.jobs
        } else {
            Vec::new()
        };

        let timezone = read_store_timezone(env, store.home());
        let mut has_rejected = false;
        let mut job_ids = Vec::with_capacity(raw_jobs.len());

        for job_val in &raw_jobs {
            let job_id = job_val
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();
            job_ids.push(job_id.clone());
            let full_id = format!("hermes:{profile}:{job_id}");
            if !is_selected {
                cron_nonselected.push(full_id);
            } else {
                match serde_json::from_value::<HermesJob>(job_val.clone()) {
                    Ok(job) => match job.validate(timezone.as_deref(), now) {
                        Ok(_) => {
                            if existing.contains(&full_id) {
                                cron_already_present.push(full_id);
                            } else {
                                cron_importable.push(full_id);
                            }
                        }
                        Err(reason) => {
                            has_rejected = true;
                            cron_rejected.push(CronJobRejection {
                                id: full_id,
                                job_id,
                                profile: profile.clone(),
                                reason,
                            });
                        }
                    },
                    Err(err) => {
                        has_rejected = true;
                        cron_rejected.push(CronJobRejection {
                            id: full_id,
                            job_id,
                            profile: profile.clone(),
                            reason: format!("invalid Hermes cron job specification: {err}"),
                        });
                    }
                }
            }
        }

        scanned_stores.push(ScannedStore {
            profile,
            job_ids,
            is_selected,
            has_rejected,
        });
    }

    let (pids_stopped, plists_disabled) = if !args.no_cutover {
        let gateway = bring_gateway_down(env, &paths.hermes_root, &paths.launch_agents_dir, true)
            .map_err(|error| {
            step_error("gateway-down projection", "cron cutover projection", error)
        })?;
        (gateway.pids_terminated, gateway.plists_disabled)
    } else {
        (Vec::new(), Vec::new())
    };

    let cron_stores = if args.no_cutover {
        Vec::new()
    } else if let Some(pool) = pool.as_ref() {
        let mut stores = cutover_cron_stores(env, &paths.hermes_root, pool, true)
            .await
            .map_err(|error| step_error("cron cutover projection", "none", error))?
            .stores;
        for store in &mut stores {
            let scan = scanned_stores.iter().find(|s| s.profile == store.profile);
            let is_selected = scan.map(|s| s.is_selected).unwrap_or(true);
            let has_rejected = scan.map(|s| s.has_rejected).unwrap_or(false);
            store.would_empty = is_selected && store.jobs_found > 0 && !has_rejected;
        }
        stores
    } else {
        scanned_stores
            .iter()
            .map(|scan| {
                let ids = scan.job_ids.clone();
                CronStoreCutoverSummary {
                    profile: scan.profile.clone(),
                    jobs_found: ids.len(),
                    all_imported: false,
                    would_empty: scan.is_selected && !ids.is_empty() && !scan.has_rejected,
                    unverified_jobs: ids,
                }
            })
            .collect()
    };

    let summary = MigrationSummary {
        dry_run: true,
        database_would_be_created: pool.is_none(),
        config_keys: config.values.len(),
        config_tokens: imported_token_count(&config.values),
        config_diff: config.diff,
        cron_imported: 0,
        cron_importable,
        cron_already_present,
        cron_rejected,
        cron_nonselected,
        cron_stores,
        pids_stopped,
        plists_disabled,
    };
    Ok(summary)
}

fn discover_hermes_stores(env: &dyn MigrationEnv, root: &Path) -> Result<Vec<HermesStore>> {
    let mut stores = discover_all_hermes_stores(env, root)?;
    let selected = HermesStoreSynchronizer::selected_profiles();
    stores.retain(|store| {
        HermesStoreSynchronizer::is_profile_selected(store.profile(), selected.as_deref())
    });
    Ok(stores)
}

fn discover_all_hermes_stores(env: &dyn MigrationEnv, root: &Path) -> Result<Vec<HermesStore>> {
    let mut stores = vec![HermesStore::new("default", root)];
    let profiles_root = root.join("profiles");
    if env.is_dir(&profiles_root) {
        let mut profiles = env
            .read_dir(&profiles_root)?
            .into_iter()
            .filter(|path| env.is_dir(path))
            .collect::<Vec<_>>();
        profiles.sort();
        for home in profiles {
            let profile = home
                .file_name()
                .ok_or_else(|| {
                    OmonError::Config(format!("invalid Hermes profile path: {}", home.display()))
                })?
                .to_string_lossy()
                .into_owned();
            stores.push(HermesStore::new(profile, home));
        }
    }
    Ok(stores)
}

fn read_store_timezone(env: &dyn MigrationEnv, home: &Path) -> Option<String> {
    let path = home.join("config.yaml");
    let bytes = env.read(&path).ok()?;
    #[derive(Deserialize)]
    struct ConfigTimezone {
        #[serde(default)]
        timezone: Option<String>,
    }
    serde_yaml::from_slice::<ConfigTimezone>(&bytes)
        .ok()?
        .timezone
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn existing_cron_ids(pool: Option<&SqlitePool>) -> Result<BTreeSet<String>> {
    let Some(pool) = pool else {
        return Ok(BTreeSet::new());
    };
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM cron_jobs")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

async fn open_read_only_pool_if_present(database_url: &str) -> Result<Option<SqlitePool>> {
    let Some(path) = sqlite_file_path(database_url) else {
        return Err(OmonError::Config(format!(
            "dry-run requires a file-backed SQLite DATABASE_URL, got {database_url}"
        )));
    };
    if !path.exists() {
        return Ok(None);
    }
    let options = SqliteConnectOptions::from_str(database_url)?
        .read_only(true)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    Ok(Some(pool))
}

fn sqlite_file_path(database_url: &str) -> Option<PathBuf> {
    let value = database_url.strip_prefix("sqlite://")?;
    let value = value.split('?').next().unwrap_or(value);
    if value.is_empty() || value == ":memory:" {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

fn imported_token_count(values: &std::collections::BTreeMap<String, String>) -> usize {
    values.get("DISCORD_BOT_TOKEN").map_or(0, |_| 1)
        + values.get("DISCORD_BOT_TOKENS").map_or(0, |tokens| {
            tokens.split(',').filter(|token| !token.is_empty()).count()
        })
}

fn step_error(step: &str, remaining: &str, error: OmonError) -> OmonError {
    OmonError::Config(format!(
        "migration step `{step}` failed: {error}; remaining steps not run: {remaining}"
    ))
}

fn print_summary(summary: &MigrationSummary) {
    let mode = if summary.dry_run {
        "DRY RUN"
    } else {
        "COMPLETE"
    };
    println!("migration_summary:");
    println!("  mode: {mode}");
    println!("  config_keys: {}", summary.config_keys);
    println!("  config_tokens: {}", summary.config_tokens);
    println!("  config_changes:");
    for line in summary.config_diff.lines() {
        println!("    {line}");
    }
    println!(
        "  database_would_be_created: {}",
        summary.database_would_be_created
    );
    println!("  cron_import:");
    println!("    imported: {}", summary.cron_imported);
    println!("    importable: {:?}", summary.cron_importable);
    println!("    already_present: {:?}", summary.cron_already_present);
    println!("    rejected: {:?}", summary.cron_rejected);
    println!("    nonselected: {:?}", summary.cron_nonselected);
    println!("  cron_delete:");
    for store in &summary.cron_stores {
        println!(
            "    {}: jobs={}, would_empty={}, currently_unverified={:?}",
            store.profile, store.jobs_found, store.would_empty, store.unverified_jobs
        );
    }
    println!("  gateway_down:");
    println!("    pids_to_stop: {:?}", summary.pids_stopped);
    println!("    plists_to_disable: {:?}", summary.plists_disabled);
}

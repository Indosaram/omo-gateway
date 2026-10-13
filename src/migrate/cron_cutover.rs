use crate::migrate::sys::MigrationEnv;
use crate::{OmonError, Result};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronStoreCutoverSummary {
    pub profile: String,
    pub jobs_found: usize,
    pub all_imported: bool,
    pub would_empty: bool,
    pub unverified_jobs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronCutoverSummary {
    pub stores: Vec<CronStoreCutoverSummary>,
}

#[derive(Debug, Deserialize)]
struct StoreDocument {
    #[serde(default)]
    jobs: Vec<Value>,
}

pub struct PreparedStore {
    pub profile: String,
    pub path: PathBuf,
    pub original: Vec<u8>,
    pub job_ids: Vec<String>,
    pub unverified_jobs: Vec<String>,
}

struct PreparedCutoverStore {
    profile: String,
    path: PathBuf,
    original_bytes: Vec<u8>,
    original_hash: String,
    backup_candidate: PathBuf,
    backup_path: Option<PathBuf>,
    replacement_bytes: Vec<u8>,
    replacement_hash: String,
    job_ids: Vec<String>,
    unverified_jobs: Vec<String>,
    job_digests: Vec<(String, String)>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(hash.len() * 2);
    for b in hash {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

pub fn normalize_job_payload(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, val) in map {
                // Ignore runtime state and internal provenance metadata
                if key == "_omon_hermes_source"
                    || key == "_omon_hermes_profile"
                    || key == "_omon_hermes_home"
                    || key == "last_status"
                    || key == "last_run_at"
                    || key == "last_error"
                    || key == "last_delivery_error"
                    || key == "created_at"
                    || key == "updated_at"
                    || key == "next_run_at"
                    || key == "id"
                {
                    continue;
                }
                if key == "repeat" {
                    if let Value::Object(repeat_map) = val {
                        let mut cleaned_repeat = serde_json::Map::new();
                        for (rk, rv) in repeat_map {
                            if rk != "completed" {
                                cleaned_repeat.insert(rk.clone(), rv.clone());
                            }
                        }
                        cleaned.insert(key.clone(), Value::Object(cleaned_repeat));
                        continue;
                    }
                }
                cleaned.insert(key.clone(), normalize_job_payload(val));
            }
            Value::Object(cleaned)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_job_payload).collect()),
        other => other.clone(),
    }
}

pub fn canonical_job_payload_matches(source: &Value, db_payload: &Value) -> bool {
    let source_norm =
        if let Ok(hermes_job) = serde_json::from_value::<crate::cron::HermesJob>(source.clone()) {
            if let Ok(serialized) = serde_json::to_value(&hermes_job) {
                normalize_job_payload(&serialized)
            } else {
                normalize_job_payload(source)
            }
        } else {
            normalize_job_payload(source)
        };

    let mut db_norm = if let Ok(hermes_job) =
        serde_json::from_value::<crate::cron::HermesJob>(db_payload.clone())
    {
        if let Ok(serialized) = serde_json::to_value(&hermes_job) {
            normalize_job_payload(&serialized)
        } else {
            normalize_job_payload(db_payload)
        }
    } else {
        normalize_job_payload(db_payload)
    };

    // Strip values injected at import time (config.yaml home channel/thread and
    // timezone) when the source job does not carry them itself.
    if let Value::Object(db_map) = &mut db_norm {
        for key in ["_omon_discord_home_channel", "_omon_discord_home_thread"] {
            if source_norm.get(key).is_none() {
                db_map.remove(key);
            }
        }
        let source_tz = source_norm
            .get("schedule")
            .and_then(|schedule| schedule.get("timezone"));
        if source_tz.is_none_or(Value::is_null) {
            if let Some(Value::Object(schedule)) = db_map.get_mut("schedule") {
                match source_tz {
                    Some(null) => {
                        schedule.insert("timezone".into(), null.clone());
                    }
                    None => {
                        schedule.remove("timezone");
                    }
                }
            }
        }
    }

    source_norm == db_norm
}

pub async fn reconcile_pending_cutover(
    env: &dyn MigrationEnv,
    pool: &SqlitePool,
) -> Result<Option<String>> {
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='cron_cutover_receipts')",
    )
    .fetch_one(pool)
    .await?;
    if !table_exists {
        return Ok(None);
    }

    let pending_op: Option<(String,)> = sqlx::query_as(
        "SELECT operation_id FROM cron_cutover_receipts WHERE status = 'pending' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let Some((operation_id,)) = pending_op else {
        return Ok(None);
    };

    type CutoverStoreRow = (String, String, String, String, Option<Vec<u8>>);
    let stores: Vec<CutoverStoreRow> = sqlx::query_as(
        "SELECT store_path, original_hash, replacement_hash, backup_path, replacement_bytes
         FROM cron_cutover_receipt_stores
         WHERE operation_id = ?
         ORDER BY store_path",
    )
    .bind(&operation_id)
    .fetch_all(pool)
    .await?;

    // Validate every payload before advancing any store. Legacy receipts lack
    // exact bytes; neither receipt times nor backup names can reconstruct them.
    let stores = stores
        .into_iter()
        .map(|(path, original_hash, replacement_hash, backup, bytes)| {
            let bytes = bytes.ok_or_else(|| {
                OmonError::Config(format!(
                    "Hermes cron store {path} has no journaled replacement bytes; automatic cutover recovery unavailable; original backup preserved at {backup}"
                ))
            })?;
            if sha256_hex(&bytes) != replacement_hash {
                return Err(OmonError::Config(format!(
                    "Hermes cron store {path} journaled replacement bytes do not match replacement hash; original backup preserved at {backup}"
                )));
            }
            Ok((path, original_hash, backup, bytes))
        })
        .collect::<Result<Vec<_>>>()?;

    // Acquire locks on all stores in canonical order
    let mut lock_paths = BTreeSet::new();
    for (store_path_str, _, _, _) in &stores {
        let p = PathBuf::from(store_path_str);
        if let Some(parent) = p.parent() {
            if env.exists(parent) {
                let lock_path = env.canonicalize(&parent.join(".jobs.lock"))?;
                lock_paths.insert(lock_path);
            }
        }
    }
    let mut locks = Vec::new();
    for lock_path in &lock_paths {
        locks.push(env.acquire_jobs_lock(lock_path)?);
    }

    let now = env.now();
    for (store_path_str, orig_hash, backup_path, replacement_bytes) in &stores {
        let path = PathBuf::from(store_path_str);
        let current_bytes = if env.exists(&path) {
            env.read(&path)?
        } else {
            Vec::new()
        };
        let current_hash = sha256_hex(&current_bytes);

        if current_bytes == *replacement_bytes {
            sqlx::query(
                "UPDATE cron_cutover_receipt_stores SET phase = 'replaced', updated_at = ? WHERE operation_id = ? AND store_path = ?",
            )
            .bind(now)
            .bind(&operation_id)
            .bind(store_path_str)
            .execute(pool)
            .await?;
        } else if current_hash == *orig_hash {
            env.write_atomic(&path, replacement_bytes)?;
            sqlx::query(
                "UPDATE cron_cutover_receipt_stores SET phase = 'replaced', updated_at = ? WHERE operation_id = ? AND store_path = ?",
            )
            .bind(now)
            .bind(&operation_id)
            .bind(store_path_str)
            .execute(pool)
            .await?;
        } else {
            return Err(OmonError::Config(format!(
                "Hermes cron store {} has unknown content during cutover recovery (hash does not match original or replacement); backups preserved at {}",
                path.display(),
                backup_path
            )));
        }
    }

    let mut tx = pool.begin().await?;
    let now = env.now();
    sqlx::query(
        "UPDATE cron_cutover_receipts SET status = 'committed', updated_at = ? WHERE operation_id = ?",
    )
    .bind(now)
    .bind(&operation_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE cron_cutover_receipt_stores SET phase = 'committed', updated_at = ? WHERE operation_id = ?",
    )
    .bind(now)
    .bind(&operation_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE cron_jobs SET authority = 'omon_owned', updated_at = ?
         WHERE authority = 'cutover_pending'",
    )
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    drop(locks);

    Ok(Some(operation_id))
}

pub async fn cutover_cron_stores(
    env: &dyn MigrationEnv,
    hermes_root: &Path,
    pool: &SqlitePool,
    dry_run: bool,
) -> Result<CronCutoverSummary> {
    if !dry_run {
        reconcile_pending_cutover(env, pool).await?;
    }

    let stores = discover_stores(env, hermes_root)?;

    // Lock all selected stores in canonical order (deduplicating aliases safely)
    let mut lock_paths = BTreeSet::new();
    for (_profile, path) in &stores {
        if let Some(parent) = path.parent() {
            if env.exists(parent) {
                let lock_path = env.canonicalize(&parent.join(".jobs.lock"))?;
                lock_paths.insert(lock_path);
            }
        }
    }

    let mut locks = Vec::new();
    if !dry_run {
        for lock_path in &lock_paths {
            locks.push(env.acquire_jobs_lock(lock_path)?);
        }
    }

    // Under locks take final snapshot and validate whole import receipt and source set
    let mut prepared = Vec::with_capacity(stores.len());

    for (profile, path) in &stores {
        let original = if env.exists(path) {
            if !env.is_file(path) {
                return Err(OmonError::Config(format!(
                    "Hermes cron store is not a file: {}",
                    path.display()
                )));
            }
            env.read(path)?
        } else {
            Vec::new()
        };
        let original_hash = sha256_hex(&original);
        let store_jobs = if original.is_empty() && !env.exists(path) {
            Vec::new()
        } else {
            parse_store_jobs(path, &original)?
        };
        let mut job_ids = Vec::with_capacity(store_jobs.len());
        let mut unverified_jobs = Vec::new();
        let mut job_digests = Vec::with_capacity(store_jobs.len());

        for job_val in &store_jobs {
            let job_id = job_val.get("id").and_then(Value::as_str).unwrap_or("");
            job_ids.push(job_id.to_owned());
            let imported_id = format!("hermes:{profile}:{job_id}");
            let db_row: Option<(String,)> =
                sqlx::query_as("SELECT payload_json FROM cron_jobs WHERE id = ?")
                    .bind(&imported_id)
                    .fetch_optional(pool)
                    .await?;

            match db_row {
                None => {
                    unverified_jobs.push(job_id.to_owned());
                }
                Some((db_payload_str,)) => {
                    let db_payload: Value =
                        serde_json::from_str(&db_payload_str).unwrap_or(Value::Null);
                    if !canonical_job_payload_matches(job_val, &db_payload) {
                        unverified_jobs.push(job_id.to_owned());
                    } else {
                        let digest = sha256_hex(
                            serde_json::to_string(&normalize_job_payload(job_val))
                                .unwrap_or_default()
                                .as_bytes(),
                        );
                        job_digests.push((job_id.to_owned(), digest));
                    }
                }
            }
        }

        let parent = path.parent().ok_or_else(|| {
            OmonError::Config(format!(
                "invalid Hermes cron store path: {}",
                path.display()
            ))
        })?;
        let file_name = path.file_name().ok_or_else(|| {
            OmonError::Config(format!(
                "invalid Hermes cron store path: {}",
                path.display()
            ))
        })?;
        let timestamp = env.now().format("%Y%m%dT%H%M%S%.fZ").to_string();
        let backup_candidate = parent.join(format!(
            "{}.bak-omon-migration-{timestamp}",
            file_name.to_string_lossy()
        ));
        let emptied = serde_json::to_vec(&serde_json::json!({
            "jobs": [],
            "updated_at": env.now().to_rfc3339(),
        }))
        .map_err(|e| OmonError::Config(format!("failed to serialize empty cron store: {e}")))?;
        let replacement_hash = sha256_hex(&emptied);

        prepared.push(PreparedCutoverStore {
            profile: profile.clone(),
            path: path.clone(),
            original_bytes: original,
            original_hash,
            backup_candidate,
            backup_path: None,
            replacement_bytes: emptied,
            replacement_hash,
            job_ids,
            unverified_jobs,
            job_digests,
        });
    }

    let summary = CronCutoverSummary {
        stores: prepared
            .iter()
            .map(|store| CronStoreCutoverSummary {
                profile: store.profile.clone(),
                jobs_found: store.job_ids.len(),
                all_imported: store.unverified_jobs.is_empty(),
                would_empty: !store.job_ids.is_empty() && store.unverified_jobs.is_empty(),
                unverified_jobs: store.unverified_jobs.clone(),
            })
            .collect(),
    };

    if dry_run {
        return Ok(summary);
    }

    if let Some(store) = prepared
        .iter()
        .find(|store| !store.unverified_jobs.is_empty())
    {
        return Err(OmonError::Config(format!(
            "Hermes cron job hermes:{}:{} payload does not match imported cron_jobs or has not been imported; refusing to empty {}",
            store.profile,
            store.unverified_jobs[0],
            store.path.display()
        )));
    }

    let stores_with_jobs: Vec<_> = prepared
        .into_iter()
        .filter(|s| !s.job_ids.is_empty())
        .collect();

    if stores_with_jobs.is_empty() {
        return Ok(summary);
    }

    // Prepare private unique backups for ALL stores before first source rewrite
    let mut backed_up_stores = Vec::with_capacity(stores_with_jobs.len());
    for mut store in stores_with_jobs {
        let backup_path = env.write_unique(&store.backup_candidate, &store.original_bytes)?;
        store.backup_path = Some(backup_path);
        backed_up_stores.push(store);
    }

    // Record durable pending receipt with exact store set/hashes/provenance in SQLite
    let operation_id = uuid::Uuid::new_v4().to_string();
    let now = env.now();
    let all_job_ids: Vec<String> = backed_up_stores
        .iter()
        .flat_map(|store| {
            store
                .job_ids
                .iter()
                .map(|job_id| format!("hermes:{}:{}", store.profile, job_id))
        })
        .collect();

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO cron_cutover_receipts (operation_id, status, store_count, policy_digest, created_at, updated_at)
         VALUES (?, 'pending', ?, ?, ?, ?)",
    )
    .bind(&operation_id)
    .bind(backed_up_stores.len() as i64)
    .bind("hermes_cutover")
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    for store in &backed_up_stores {
        let digests_json =
            serde_json::to_string(&store.job_digests).unwrap_or_else(|_| "[]".into());
        let backup_str = store
            .backup_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        sqlx::query(
            "INSERT INTO cron_cutover_receipt_stores
             (operation_id, profile, store_path, original_hash, replacement_hash, backup_path, job_digests_json, replacement_bytes, phase, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'backed_up', ?)",
        )
        .bind(&operation_id)
        .bind(&store.profile)
        .bind(store.path.to_string_lossy())
        .bind(&store.original_hash)
        .bind(&store.replacement_hash)
        .bind(backup_str)
        .bind(digests_json)
        .bind(&store.replacement_bytes)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    for id in &all_job_ids {
        sqlx::query(
            "UPDATE cron_jobs SET authority = 'cutover_pending', updated_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Rewrite each source store and record progress
    for (index, store) in backed_up_stores.iter().enumerate() {
        let current = env.read(&store.path)?;
        if current != store.original_bytes {
            // Do not leave the receipt pending: reconcile would fail forever on
            // unknown content and the cutover_pending jobs would never run. Settle
            // the receipt as failed; stores already emptied hand their jobs to
            // omon, the untouched ones return to the Hermes mirror.
            let mut tx = pool.begin().await?;
            let failed_at = env.now();
            sqlx::query(
                "UPDATE cron_cutover_receipts SET status = 'failed', updated_at = ? WHERE operation_id = ?",
            )
            .bind(failed_at)
            .bind(&operation_id)
            .execute(&mut *tx)
            .await?;
            for (position, other) in backed_up_stores.iter().enumerate() {
                let authority = if position < index {
                    "omon_owned"
                } else {
                    "hermes_mirror"
                };
                for job_id in &other.job_ids {
                    sqlx::query(
                        "UPDATE cron_jobs SET authority = ?, updated_at = ? WHERE id = ? AND authority = 'cutover_pending'",
                    )
                    .bind(authority)
                    .bind(failed_at)
                    .bind(format!("hermes:{}:{}", other.profile, job_id))
                    .execute(&mut *tx)
                    .await?;
                }
            }
            tx.commit().await?;
            return Err(OmonError::Config(format!(
                "Hermes cron store changed during migration: {}; refusing to empty stale state",
                store.path.display()
            )));
        }
        env.write_atomic(&store.path, &store.replacement_bytes)?;
        sqlx::query(
            "UPDATE cron_cutover_receipt_stores
             SET phase = 'replaced', updated_at = ?
             WHERE operation_id = ? AND store_path = ?",
        )
        .bind(env.now())
        .bind(&operation_id)
        .bind(store.path.to_string_lossy())
        .execute(pool)
        .await?;
    }

    // Final ownership commit in SQLite
    let mut tx = pool.begin().await?;
    let now = env.now();
    sqlx::query(
        "UPDATE cron_cutover_receipts SET status = 'committed', updated_at = ? WHERE operation_id = ?",
    )
    .bind(now)
    .bind(&operation_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE cron_cutover_receipt_stores SET phase = 'committed', updated_at = ? WHERE operation_id = ?",
    )
    .bind(now)
    .bind(&operation_id)
    .execute(&mut *tx)
    .await?;

    for id in &all_job_ids {
        sqlx::query("UPDATE cron_jobs SET authority = 'omon_owned', updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    drop(locks);

    Ok(summary)
}

fn discover_stores(env: &dyn MigrationEnv, hermes_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut stores = vec![(
        "default".to_owned(),
        hermes_root.join("cron").join("jobs.json"),
    )];
    let profiles_root = hermes_root.join("profiles");
    if !env.exists(&profiles_root) {
        return Ok(stores);
    }
    if !env.is_dir(&profiles_root) {
        return Err(OmonError::Config(format!(
            "Hermes profiles path is not a directory: {}",
            profiles_root.display()
        )));
    }
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
        stores.push((profile, home.join("cron").join("jobs.json")));
    }
    Ok(stores)
}

fn parse_store_jobs(path: &Path, bytes: &[u8]) -> Result<Vec<Value>> {
    let document: StoreDocument = serde_json::from_slice(bytes).map_err(|error| {
        OmonError::Config(format!(
            "invalid Hermes cron store {}: {error}",
            path.display()
        ))
    })?;
    for job in &document.jobs {
        let id = job.get("id").and_then(Value::as_str).unwrap_or("");
        if id.trim().is_empty() {
            return Err(OmonError::Config(format!(
                "Hermes cron store {} contains a job without an id",
                path.display()
            )));
        }
    }
    Ok(document.jobs)
}

pub fn cutover_store(env: &dyn MigrationEnv, store: &PreparedStore) -> Result<()> {
    let directory = store.path.parent().ok_or_else(|| {
        OmonError::Config(format!(
            "invalid Hermes cron store path: {}",
            store.path.display()
        ))
    })?;
    let _lock = env.acquire_jobs_lock(&directory.join(".jobs.lock"))?;
    let current = env.read(&store.path)?;
    if current != store.original {
        return Err(OmonError::Config(format!(
            "Hermes cron store changed during migration: {}; refusing to back up or empty stale state",
            store.path.display()
        )));
    }

    let now = env.now();
    let timestamp = now.format("%Y%m%dT%H%M%S%.fZ").to_string();
    let file_name = store.path.file_name().ok_or_else(|| {
        OmonError::Config(format!(
            "invalid Hermes cron store path: {}",
            store.path.display()
        ))
    })?;
    let backup = directory.join(format!(
        "{}.bak-omon-migration-{timestamp}",
        file_name.to_string_lossy()
    ));

    let emptied = serde_json::json!({
        "jobs": [],
        "updated_at": now.to_rfc3339(),
    });
    let emptied = serde_json::to_vec(&emptied).map_err(|error| {
        OmonError::Config(format!("failed to serialize empty cron store: {error}"))
    })?;

    env.write_unique(&backup, &store.original)?;
    env.write_atomic(&store.path, &emptied)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{cutover_cron_stores, cutover_store, PreparedStore};
    use crate::migrate::sys::{FakeMigrationEnv, MigrationEnv, MigrationOperation};
    use crate::storage::Database;
    use crate::OmonError;
    use chrono::{TimeZone, Utc};
    use std::path::{Path, PathBuf};

    const ROOT: &str = "/fixtures/.hermes";
    const STORE: &str = "/fixtures/.hermes/cron/jobs.json";
    const ORIGINAL: &[u8] = br#"{"jobs":[{"id":"daily"},{"id":"weekly"}],"updated_at":"old"}"#;

    async fn fixture() -> (FakeMigrationEnv, Database) {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 12, 34, 56).unwrap();
        let env = FakeMigrationEnv::new(now);
        env.write(Path::new(STORE), ORIGINAL).unwrap();
        let database = Database::connect("sqlite::memory:").await.unwrap();
        (env, database)
    }

    async fn import_job(database: &Database, id: &str) {
        sqlx::query(
            "INSERT INTO cron_jobs (id, expression, payload_json, enabled) VALUES (?, '* * * * *', '{}', 1)",
        )
        .bind(id)
        .execute(database.pool())
        .await
        .unwrap();
    }

    fn backup_write(env: &FakeMigrationEnv) -> (PathBuf, Vec<u8>) {
        env.write_calls()
            .into_iter()
            .find(|(path, _)| path.to_string_lossy().contains("bak-omon-migration-"))
            .expect("backup write")
    }

    #[tokio::test]
    async fn verified_jobs_are_backed_up_then_atomically_emptied() {
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;
        let writes_before = env.write_calls().len();

        let summary = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();

        assert_eq!(summary.stores[0].jobs_found, 2);
        assert!(summary.stores[0].all_imported);
        assert!(summary.stores[0].would_empty);
        let migration_writes = &env.write_calls()[writes_before..];
        assert!(migration_writes[0]
            .0
            .to_string_lossy()
            .contains("jobs.json.bak-omon-migration-"));
        assert!(migration_writes[1]
            .0
            .to_string_lossy()
            .contains(".jobs.json.tmp-omon-migration-"));
        let emptied: serde_json::Value =
            serde_json::from_slice(&env.read(Path::new(STORE)).unwrap()).unwrap();
        assert_eq!(emptied["jobs"], serde_json::json!([]));
        assert_eq!(env.rename_calls().len(), 1);
        let lock_path = PathBuf::from("/fixtures/.hermes/cron/.jobs.lock");
        let operations = env.operations();
        let lock_index = operations
            .iter()
            .position(|operation| operation == &MigrationOperation::LockAcquired(lock_path.clone()))
            .expect("jobs lock acquisition");
        let first_write_index = operations
            .iter()
            .position(|operation| {
                matches!(operation, MigrationOperation::Write(path) if path.to_string_lossy().contains("bak-omon-migration-"))
            })
            .expect("first cutover write");
        let release_index = operations
            .iter()
            .position(|operation| operation == &MigrationOperation::LockReleased(lock_path.clone()))
            .expect("jobs lock release");
        assert!(lock_index < first_write_index);
        assert!(first_write_index < release_index);
        assert!(
            !env.exists(&lock_path),
            "fake lock must not create/delete store files"
        );
        println!("verified-delete summary={summary:?}");
    }

    #[tokio::test]
    async fn rerun_on_empty_store_creates_no_second_backup() {
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;
        cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();
        let writes_after_first = env.write_calls().len();

        let summary = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();

        assert_eq!(env.write_calls().len(), writes_after_first);
        assert_eq!(summary.stores[0].jobs_found, 0);
        assert!(!summary.stores[0].would_empty);
        println!("idempotent rerun summary={summary:?}");
    }

    #[tokio::test]
    async fn unimported_job_returns_typed_error_without_changing_store() {
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        let writes_before = env.write_calls().len();

        let error = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap_err();

        assert!(matches!(error, OmonError::Config(_)));
        assert!(error.to_string().contains("hermes:default:weekly"));
        assert_eq!(env.read(Path::new(STORE)).unwrap(), ORIGINAL);
        assert_eq!(env.write_calls().len(), writes_before);
        assert!(env.rename_calls().is_empty());
        println!("unimported-block error={error}");
    }

    #[tokio::test]
    async fn backup_bytes_equal_the_pre_delete_store() {
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;

        cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();

        let (backup_path, backup_bytes) = backup_write(&env);
        assert_eq!(backup_bytes, ORIGINAL);
        assert_eq!(env.read(&backup_path).unwrap(), ORIGINAL);
        println!("backup-equality path={}", backup_path.display());
    }

    #[tokio::test]
    async fn dry_run_reports_unverified_jobs_and_performs_zero_writes() {
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        let writes_before = env.write_calls().len();

        let summary = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), true)
            .await
            .unwrap();

        assert_eq!(env.write_calls().len(), writes_before);
        assert!(env.rename_calls().is_empty());
        assert_eq!(env.read(Path::new(STORE)).unwrap(), ORIGINAL);
        assert_eq!(summary.stores[0].jobs_found, 2);
        assert!(!summary.stores[0].all_imported);
        assert!(!summary.stores[0].would_empty);
        assert_eq!(summary.stores[0].unverified_jobs, ["weekly"]);
        println!("dry-run summary={summary:?}");
    }

    #[tokio::test]
    async fn profiles_are_discovered_from_the_profiles_directory() {
        let (env, database) = fixture().await;
        let profile_store = Path::new("/fixtures/.hermes/profiles/work/cron/jobs.json");
        env.write(
            profile_store,
            br#"{"jobs":[{"id":"standup"}],"updated_at":"old"}"#,
        )
        .unwrap();
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;
        import_job(&database, "hermes:work:standup").await;

        let summary = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();

        assert_eq!(
            summary
                .stores
                .iter()
                .map(|store| store.profile.as_str())
                .collect::<Vec<_>>(),
            ["default", "work"]
        );
        let emptied: serde_json::Value =
            serde_json::from_slice(&env.read(profile_store).unwrap()).unwrap();
        assert_eq!(emptied["jobs"], serde_json::json!([]));
    }

    #[test]
    fn stale_store_state_is_rejected_before_backup() {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 12, 34, 56).unwrap();
        let env = FakeMigrationEnv::new(now);
        env.write(Path::new(STORE), ORIGINAL).unwrap();
        let prepared = PreparedStore {
            profile: "default".into(),
            path: PathBuf::from(STORE),
            original: ORIGINAL.to_vec(),
            job_ids: vec!["daily".into(), "weekly".into()],
            unverified_jobs: Vec::new(),
        };
        env.write(Path::new(STORE), b"changed concurrently")
            .unwrap();
        let writes_before = env.write_calls().len();

        let error = cutover_store(&env, &prepared).unwrap_err();

        assert!(matches!(error, OmonError::Config(_)));
        assert!(error.to_string().contains("changed during migration"));
        assert_eq!(env.write_calls().len(), writes_before);
        assert!(env.rename_calls().is_empty());
        println!("stale-state error={error}");
    }

    #[tokio::test]
    async fn malformed_store_returns_typed_error_and_performs_no_writes() {
        let (env, database) = fixture().await;
        env.write(Path::new(STORE), b"{not-json").unwrap();
        let writes_before = env.write_calls().len();

        let error = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap_err();

        assert!(matches!(error, OmonError::Config(_)));
        assert!(error.to_string().contains("invalid Hermes cron store"));
        assert_eq!(env.read(Path::new(STORE)).unwrap(), b"{not-json");
        assert_eq!(env.write_calls().len(), writes_before);
        assert!(env.rename_calls().is_empty());
        println!("malformed-input error={error}");
    }

    #[tokio::test]
    async fn alias_store_locks_deduplicate_safely() {
        let (env, database) = fixture().await;
        let profile_store = Path::new("/fixtures/.hermes/profiles/work/cron/jobs.json");
        env.write(
            profile_store,
            br#"{"jobs":[{"id":"standup"}],"updated_at":"old"}"#,
        )
        .unwrap();
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;
        import_job(&database, "hermes:work:standup").await;

        // Add alias: make work store's lock point to default store's lock directory
        let default_lock = PathBuf::from("/fixtures/.hermes/cron/.jobs.lock");
        let work_lock = PathBuf::from("/fixtures/.hermes/profiles/work/cron/.jobs.lock");
        env.add_alias(work_lock.clone(), default_lock.clone());

        let summary = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap();

        assert_eq!(summary.stores.len(), 2);
        let lock_ops: Vec<_> = env
            .operations()
            .into_iter()
            .filter(|op| matches!(op, MigrationOperation::LockAcquired(_)))
            .collect();
        assert_eq!(
            lock_ops,
            vec![MigrationOperation::LockAcquired(default_lock)]
        );
    }

    #[tokio::test]
    async fn reconcile_refuses_unknown_store_state_without_destroying_source() {
        use super::reconcile_pending_cutover;
        let (env, database) = fixture().await;
        import_job(&database, "hermes:default:daily").await;
        import_job(&database, "hermes:default:weekly").await;

        // Make store read-only to inject failure on rewrite
        env.set_read_only(Path::new(STORE), true);
        let error = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("read-only"));

        // Now mutate the store externally so it matches NEITHER original NOR replacement
        env.set_read_only(Path::new(STORE), false);
        env.write(Path::new(STORE), b"external-unrecognized-corruption")
            .unwrap();

        // Recovery must refuse without destroying the store
        let reconcile_err = reconcile_pending_cutover(&env, database.pool())
            .await
            .unwrap_err();
        assert!(reconcile_err.to_string().contains("unknown content"));
        assert_eq!(
            env.read(Path::new(STORE)).unwrap(),
            b"external-unrecognized-corruption"
        );
    }
}

#[cfg(test)]
mod fix_r5_tests {
    use super::cutover_cron_stores;
    use crate::migrate::sys::{FakeMigrationEnv, LaunchctlOutput, MigrationEnv, MigrationLock};
    use crate::storage::Database;
    use crate::Result;
    use chrono::{DateTime, TimeZone, Utc};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    const ROOT: &str = "/fixtures/.hermes";
    const STORE: &str = "/fixtures/.hermes/cron/jobs.json";
    const ORIGINAL: &[u8] = br#"{"jobs":[{"id":"daily"}],"updated_at":"old"}"#;

    /// Rewrites the store right after its backup is taken, simulating Hermes
    /// restarting between the snapshot and the emptying rewrite.
    struct RewriteAfterBackupEnv {
        inner: FakeMigrationEnv,
    }

    impl MigrationEnv for RewriteAfterBackupEnv {
        fn read_to_string(&self, path: &Path) -> Result<String> {
            self.inner.read_to_string(path)
        }
        fn read(&self, path: &Path) -> Result<Vec<u8>> {
            self.inner.read(path)
        }
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
            self.inner.write(path, bytes)
        }
        fn write_unique(&self, path: &Path, bytes: &[u8]) -> Result<PathBuf> {
            let written = self.inner.write_unique(path, bytes)?;
            if path.to_string_lossy().contains("bak-omon-migration-") {
                self.inner.write(
                    Path::new(STORE),
                    br#"{"jobs":[{"id":"daily"},{"id":"new"}]}"#,
                )?;
            }
            Ok(written)
        }
        fn rename(&self, from: &Path, to: &Path) -> Result<()> {
            self.inner.rename(from, to)
        }
        fn remove_file(&self, path: &Path) -> Result<()> {
            self.inner.remove_file(path)
        }
        fn acquire_jobs_lock(&self, path: &Path) -> Result<Box<dyn MigrationLock>> {
            self.inner.acquire_jobs_lock(path)
        }
        fn exists(&self, path: &Path) -> bool {
            self.inner.exists(path)
        }
        fn is_file(&self, path: &Path) -> bool {
            self.inner.is_file(path)
        }
        fn is_dir(&self, path: &Path) -> bool {
            self.inner.is_dir(path)
        }
        fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
            self.inner.read_dir(path)
        }
        fn create_dir_all(&self, path: &Path) -> Result<()> {
            self.inner.create_dir_all(path)
        }
        fn current_uid(&self) -> u32 {
            self.inner.current_uid()
        }
        fn now(&self) -> DateTime<Utc> {
            self.inner.now()
        }
        fn pid_alive(&self, pid: i32) -> bool {
            self.inner.pid_alive(pid)
        }
        fn terminate(&self, pid: i32) -> Result<()> {
            self.inner.terminate(pid)
        }
        fn kill(&self, pid: i32) -> Result<()> {
            self.inner.kill(pid)
        }
        fn sleep(&self, duration: Duration) {
            self.inner.sleep(duration)
        }
        fn run_launchctl(&self, args: &[&str]) -> Result<LaunchctlOutput> {
            self.inner.run_launchctl(args)
        }
    }

    #[tokio::test]
    async fn store_changed_after_snapshot_settles_receipt_and_restores_authority() {
        let now = Utc.with_ymd_and_hms(2026, 8, 15, 12, 34, 56).unwrap();
        let env = RewriteAfterBackupEnv {
            inner: FakeMigrationEnv::new(now),
        };
        env.inner.write(Path::new(STORE), ORIGINAL).unwrap();
        let database = Database::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO cron_jobs (id, expression, payload_json, enabled) VALUES ('hermes:default:daily', '* * * * *', '{}', 1)",
        )
        .execute(database.pool())
        .await
        .unwrap();

        let error = cutover_cron_stores(&env, Path::new(ROOT), database.pool(), false)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("changed during migration"));
        let status: String = sqlx::query_scalar("SELECT status FROM cron_cutover_receipts")
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(status, "failed");
        let authority: String =
            sqlx::query_scalar("SELECT authority FROM cron_jobs WHERE id = 'hermes:default:daily'")
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(authority, "hermes_mirror");
    }
}

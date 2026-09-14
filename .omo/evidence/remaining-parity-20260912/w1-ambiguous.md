# Ambiguous Parity States Verification Report (2026-09-12)

## VERDICT

- **ITEM A (migration 0025 `cron_monitor_states`)**: IMPLEMENTED
  - Table is fully integrated and active in the cron run flow: columns (`job_id`, `last_hash`, `last_snapshot`, `updated_at`) are both read and written in `src/cron/executor.rs` to short-circuit unchanged monitor jobs and record state snapshots. Not a dead reference.
- **ITEM B (Units U52, U53, U54 Cutover/Import Verification)**:
  - **U52 (CFG.C01 - Durable cutover job ownership)**: CLOSED-BY-EVIDENCE
  - **U53 (CFG.C07 - Quiesced receipt-verified all-store cutover)**: CLOSED-BY-EVIDENCE
  - **U54 (CFG.C08 - Dry-run shares full import validation)**: CLOSED-BY-EVIDENCE
- **ITEM C (R.R14 / CFG.I03 Daemon-Invariant Conflict)**: RECONCILED
  - Former port 19743 isolation has been reconciled. Defaults unify cron and interactive lanes on port 19742 (`ws://127.0.0.1:19742`), and daemon supervisor spawns exactly one multiplexed daemon process when endpoints match.

## EVIDENCE

### ITEM A: Migration 0025 Integration (`cron_monitor_states`)

1. **Table Definition**:
   - `migrations/0025_cron_monitor_states.sql:1-6`
   ```sql
   CREATE TABLE IF NOT EXISTS cron_monitor_states (
       job_id TEXT PRIMARY KEY NOT NULL,
       last_hash TEXT NOT NULL,
       last_snapshot TEXT,
       updated_at TEXT NOT NULL
   );
   ```

2. **Read Flow in Executor (Hash Verification & Skip Optimization)**:
   - `src/cron/executor.rs:144-149`
   ```rust
               let prev_state: Option<(String,)> =
                   sqlx::query_as("SELECT last_hash FROM cron_monitor_states WHERE job_id = ?")
                       .bind(&hermes.id)
                       .fetch_optional(&self.pool)
   ```
   - `src/cron/executor.rs:151-159`
   ```rust
               if let Some((prev_hash,)) = prev_state {
                   if prev_hash == current_hash {
                       tracing::info!(
                           job_id = %hermes.id,
                           "monitor state unchanged for job {}, skipping agent execution",
                           hermes.id
                       );
                       return Ok(None);
                   }
               }
   ```

3. **Write Flow in Executor (Upsert Snapshot State)**:
   - `src/cron/executor.rs:163-176`
   ```rust
               let now_str = chrono::Utc::now().to_rfc3339();
               sqlx::query(
                   "INSERT INTO cron_monitor_states (job_id, last_hash, last_snapshot, updated_at)
                    VALUES (?, ?, ?, ?)
                    ON CONFLICT(job_id) DO UPDATE SET last_hash = excluded.last_hash,
                                                      last_snapshot = excluded.last_snapshot,
                                                      updated_at = excluded.updated_at",
               )
               .bind(&hermes.id)
               .bind(&current_hash)
               .bind(&m_output)
               .bind(&now_str)
   ```

4. **Integration Test Verification**:
   - `src/main.rs:1095-1104`
   ```rust
           let res2 = executor.execute(&cron_job).await;
           assert!(res2.is_ok());
           assert_eq!(
               *run_count.lock().unwrap(),
               1,
               "Agent must be SKIPPED when monitor snapshot is unchanged"
           );
   ```

---

### ITEM B: Units U52, U53, U54 Verification

1. **U52 (B1: Source store emptied only after receipt/ownership validation)**:
   - **Canonical Payload Validation Under Store Lock**:
     - `src/migrate/cron_cutover.rs:343-346`
     ```rust
                         let db_payload: Value =
                             serde_json::from_str(&db_payload_str).unwrap_or(Value::Null);
                         if !canonical_job_payload_matches(job_val, &db_payload) {
                             unverified_jobs.push(job_id.to_owned());
     ```
   - **Cutover Refusal on Unverified Jobs Before Any Mutation**:
     - `src/migrate/cron_cutover.rs:414-417`
     ```rust
             return Err(OmonError::Config(format!(
                 "Hermes cron job hermes:{}:{} payload does not match imported cron_jobs or has not been imported; refusing to empty {}",
                 store.profile,
                 store.unverified_jobs[0],
     ```
   - **Pending Receipt and Authority Boundary Set Before Emptying**:
     - `src/migrate/cron_cutover.rs:493-496`
     ```rust
             sqlx::query(
                 "UPDATE cron_jobs SET authority = 'cutover_pending', updated_at = ? WHERE id = ?",
             )
             .bind(now)
     ```
   - **Source Store Emptying Atomic Rewrite**:
     - `src/migrate/cron_cutover.rs:514-515`
     ```rust
             env.write_atomic(&store.path, &store.replacement_bytes)?;
             sqlx::query(
     ```
   - **Ownership Commit & Synchronizer Deletion Guard**:
     - `src/migrate/cron_cutover.rs:545-546`
     ```rust
             sqlx::query("UPDATE cron_jobs SET authority = 'omon_owned', updated_at = ? WHERE id = ?")
                 .bind(now)
     ```
     - `src/cron/store.rs:987-989`
     ```rust
                         "DELETE FROM cron_jobs WHERE id = ? AND authority = 'hermes_mirror'",
                     )
                     .bind(id)
     ```

2. **U53 (B2: Quiesce + all-store receipt flow present)**:
   - **Pre-Cutover Quiesce**:
     - `src/migrate/mod.rs:185-188`
     ```rust
         let gateway = bring_gateway_down(env, &paths.hermes_root, &paths.launch_agents_dir, false)
             .map_err(|error| step_error("gateway-down", "cron cutover", error))?;
         summary.pids_stopped = gateway.pids_terminated;
         summary.plists_disabled = gateway.plists_disabled;
     ```
   - **Multi-Store Sorted Lock & Pre-Rewrite Backups for All Stores**:
     - `src/migrate/cron_cutover.rs:282-286`
     ```rust
         let mut lock_paths = BTreeSet::new();
         for (_profile, path) in &stores {
             if let Some(parent) = path.parent() {
                 if env.exists(parent) {
                     let lock_path = env.canonicalize(&parent.join(".jobs.lock"))?;
     ```
     - `src/migrate/cron_cutover.rs:428-431`
     ```rust
         let mut backed_up_stores = Vec::with_capacity(stores_with_jobs.len());
         for mut store in stores_with_jobs {
             let backup_path = env.write_unique(&store.backup_candidate, &store.original_bytes)?;
             store.backup_path = Some(backup_path);
     ```
   - **Durable Multi-Store Receipt with Status Tracking**:
     - `src/migrate/cron_cutover.rs:451-454`
     ```rust
             "INSERT INTO cron_cutover_receipts (operation_id, status, store_count, policy_digest, created_at, updated_at)
              VALUES (?, 'pending', ?, ?, ?, ?)",
         )
         .bind(&operation_id)
     ```
     - `src/migrate/cron_cutover.rs:528-531`
     ```rust
             "UPDATE cron_cutover_receipts SET status = 'committed', updated_at = ? WHERE operation_id = ?",
         )
         .bind(now)
         .bind(&operation_id)
     ```

3. **U54 (B3: Dry-run shares full import validation with apply)**:
   - **Shared Pure Validation Seam**:
     - `src/cron/store.rs:320-324`
     ```rust
         pub fn validate(
             &self,
             default_timezone: Option<&str>,
             now: DateTime<Utc>,
         ) -> std::result::Result<ValidatedHermesJob, String> {
     ```
   - **Dry-Run Classification via Shared Validator**:
     - `src/migrate/mod.rs:265-268`
     ```rust
                     match serde_json::from_value::<HermesJob>(job_val.clone()) {
                         Ok(job) => match job.validate(timezone.as_deref(), now) {
                             Ok(_) => {
                                 if existing.contains(&full_id) {
     ```
     - `src/migrate/mod.rs:274-279`
     ```rust
                             Err(reason) => {
                                 has_rejected = true;
                                 cron_rejected.push(CronJobRejection {
                                     id: full_id,
                                     job_id,
                                     profile: profile.clone(),
     ```
   - **Identical Runtime Import Validation**:
     - `src/cron/store.rs:940-943`
     ```rust
                     let validated = match job.validate(timezone.as_deref(), now) {
                         Ok(val) => val,
                         Err(reason) => {
                             tracing::warn!(job_id = %job.id, %reason, "skipping invalid Hermes job");
     ```

---

### ITEM C: Daemon-Invariant Conflict Reconciliation (R.R14 / CFG.I03)

1. **Default Daemon Port Unified to 19742**:
   - `src/agent/omo_config.rs:55`
   ```rust
   pub const CRON_APPSERVER_URL_DEFAULT: &str = "ws://127.0.0.1:19742";
   ```

2. **Cron Config Inherits Interactive Endpoint by Default**:
   - `src/agent/omo_config.rs:226-231`
   ```rust
           if let Some(appserver_url) = std::env::var("OMON_OMO_CRON_APPSERVER_URL")
               .ok()
               .map(|s| s.trim().to_string())
               .filter(|s| !s.is_empty())
           {
               config.appserver_url = appserver_url;
           }
   ```

3. **Single Multiplexed Daemon Supervised in Main Gateway**:
   - `src/main.rs:770-774`
   ```rust
       let _cron_daemon_supervisor = if cron_omo_config.appserver_url == omo_config.appserver_url {
           None
       } else {
           OmoDaemonSupervisor::ensure(&cron_omo_config).await?
       };
   ```

4. **Single Multiplexed Daemon Supervised in Dashboard Runtime**:
   - `src/dashboard_runtime.rs:146-150`
   ```rust
       let _cron_daemon_supervisor = if cron_omo_config.appserver_url == omo_config.appserver_url {
           None
       } else {
           OmoDaemonSupervisor::ensure(&cron_omo_config).await?
       };
   ```

5. **Contract Pinning Tests**:
   - `src/main.rs:1536-1540`
   ```rust
           assert_eq!(interactive.appserver_url, "ws://127.0.0.1:19742");
           assert_eq!(
               cron.appserver_url, interactive.appserver_url,
               "Cron and interactive must share one daemon endpoint"
           );
   ```
   - `src/agent/omo_config.rs:338-341`
   ```rust
           assert_eq!(interactive.appserver_url, "ws://127.0.0.1:19742");
           assert_eq!(cron.appserver_url, "ws://127.0.0.1:19742");
           assert_eq!(cron.appserver_url, interactive.appserver_url);
   ```

## RESIDUAL

None.
- Table `cron_monitor_states` is fully hooked into executor evaluation and snapshot updating.
- Units U52, U53, and U54 have verified production code paths and regression assertions covering durable ownership, quiesced multi-store receipts, and shared dry-run import validation.
- The two-daemon topology conflict on port 19743 was resolved under unit U66, enforcing a single multiplexed daemon on port 19742 while preserving distinct lane turn timeouts.

## CLAIM

CLOSED: All three ambiguous parity states verified in the current tree: Item A is IMPLEMENTED, Item B units U52/U53/U54 are CLOSED-BY-EVIDENCE, and Item C is RECONCILED.

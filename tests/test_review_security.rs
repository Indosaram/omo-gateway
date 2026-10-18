mod sec01 {
    use std::panic::AssertUnwindSafe;
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use futures_util::FutureExt;
    use omon_gateway::{
        Database, DiscordApprovalRequester, McpClientTool, McpTool, McpTransport, OmonError,
        OutboundAction, OutboundDispatcher, SessionKey, SmartApprovalGuard, ToolRegistry,
    };
    use serde_json::{json, Value};
    use tokio::sync::{mpsc, Mutex};
    use tokio::task::JoinHandle;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    const BOUND: Duration = Duration::from_secs(10);

    #[derive(Default)]
    struct ApprovalRecord {
        prompts: Vec<Uuid>,
        expired: Vec<Uuid>,
        decisions: Vec<&'static str>,
        errors: Vec<String>,
    }

    struct RecordedDispatcher(mpsc::UnboundedSender<OutboundAction>);

    #[async_trait]
    impl OutboundDispatcher for RecordedDispatcher {
        async fn dispatch(&self, action: OutboundAction) -> omon_gateway::Result<()> {
            self.0
                .send(action)
                .map_err(|error| OmonError::Approval(format!("fixture dispatcher: {error}")))
        }
    }

    // On a cleanup deadline, abort only this owned task and await its termination.
    // A deadline remains a cleanup failure even if cancellation succeeds.
    async fn join_owned<T>(mut task: JoinHandle<T>, name: &str) -> Result<T, String> {
        match timeout(BOUND, &mut task).await {
            Ok(result) => result.map_err(|error| format!("{name}: {error}")),
            Err(_) => {
                task.abort();
                let termination = timeout(BOUND, task).await;
                let detail = match termination {
                    Ok(Err(error)) if error.is_cancelled() => "aborted and joined".to_owned(),
                    Ok(Err(error)) => format!("joined with {error}"),
                    Ok(Ok(_)) => "joined after deadline".to_owned(),
                    Err(_) => "abort join deadline exceeded".to_owned(),
                };
                Err(format!("{name}: cleanup deadline exceeded; {detail}"))
            }
        }
    }

    #[test]
    fn same_method_client_consent() {
        // No environment-derived registry, filesystem fixture, or live database.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("SEC-01 fixture runtime");

        let (outcome, cleanup_errors) = runtime.block_on(async {
            let mut db: Option<Database> = None;
            let mut server = None;
            let mut approval_actor = None;
            let stop = CancellationToken::new();
            let guard = SmartApprovalGuard::new();
            let session = SessionKey::new("discord", None::<String>, "42", None::<String>, "7");
            let approvals = Arc::new(Mutex::new(ApprovalRecord::default()));
            let calls = Arc::new(Mutex::new(Vec::<Value>::new()));

            // Keep resource handles outside the caught future. Assertions and
            // cancellation drop the real registry/requester before cleanup joins.
            let outcome = timeout(
                Duration::from_secs(60),
                AssertUnwindSafe(async {
                    db = Some(
                        Database::connect("sqlite::memory:")
                            .await
                            .expect("SEC-01 isolated migrated SQLite"),
                    );
                    let pool = db.as_ref().unwrap().pool();
                    guard.set_pool(pool.clone()).await;
                    assert_eq!(
                        guard.load_persisted_allowlist().await.unwrap(),
                        0,
                        "fixture database must start without grants"
                    );
                    assert!(!guard.is_yolo(&session).await);

                    let fixture_calls = calls.clone();
                    let app = axum::Router::new().route(
                        "/",
                        axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
                            let calls = fixture_calls.clone();
                            async move {
                                calls.lock().await.push(request.clone());
                                axum::Json(json!({
                                    "jsonrpc": "2.0",
                                    "id": request["id"],
                                    "result": request["params"]
                                }))
                            }
                        }),
                    );
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                        .await
                        .expect("SEC-01 owned loopback bind");
                    let address = listener.local_addr().unwrap();
                    let url = format!("http://{address}/");
                    println!("SEC01_BIND {address}");
                    let stopped = stop.clone();
                    server = Some(tokio::spawn(async move {
                        axum::serve(listener, app)
                            .with_graceful_shutdown(stopped.cancelled_owned())
                            .await
                    }));

                    // Subscribe before either registry call. Only the external
                    // approval transport and human decisions are fixture actors;
                    // the requester, pending lease, caches and rule key are real.
                    let (sender, mut actions) = mpsc::unbounded_channel();
                    let actor_guard = guard.clone();
                    let actor_record = approvals.clone();
                    let actor_session = session.clone();
                    approval_actor =
                        Some(tokio::spawn(async move {
                            while let Some(action) = actions.recv().await {
                                match action {
                                    OutboundAction::ApprovalRequest {
                                        request_id,
                                        session,
                                        ..
                                    } => {
                                        let decision = {
                                            let mut record = actor_record.lock().await;
                                            if session != actor_session {
                                                record.errors.push(
                                                    "approval used the wrong session".to_owned(),
                                                );
                                            }
                                            let decision = if record.prompts.is_empty() {
                                                "always"
                                            } else {
                                                "deny"
                                            };
                                            record.prompts.push(request_id);
                                            record.decisions.push(decision);
                                            decision
                                        };
                                        let resolved = actor_guard
                                            .resolve_custom_id(&format!(
                                                "omon:approval:{request_id}:{decision}"
                                            ))
                                            .await;
                                        if !resolved {
                                            actor_record.lock().await.errors.push(format!(
                                                "approval {request_id} was not pending"
                                            ));
                                        }
                                    }
                                    OutboundAction::ExpireApproval { request_id } => {
                                        actor_record.lock().await.expired.push(request_id);
                                    }
                                    _ => actor_record
                                        .lock()
                                        .await
                                        .errors
                                        .push("unexpected outbound action".to_owned()),
                                }
                            }
                        }));

                    let requester = Arc::new(DiscordApprovalRequester::new(guard.clone(), BOUND));
                    requester
                        .set_dispatcher(Arc::new(RecordedDispatcher(sender)))
                        .await;
                    let mut registry = ToolRegistry::default()
                        .with_approval_requester(requester, Duration::from_secs(30));
                    registry.register(McpTool::new(
                        ["client_a", "client_b"]
                            .into_iter()
                            .map(|name| {
                                McpClientTool::new(
                                    name,
                                    "execute",
                                    "SEC-01 loopback fixture",
                                    json!({"type": "object"}),
                                    McpTransport::Sse {
                                        url: url.clone(),
                                        bearer_token: None,
                                    },
                                )
                                .with_approval(true)
                                .with_timeout(BOUND)
                            })
                            .collect(),
                    ));

                    let first = registry
                        .execute_with_context(
                            "mcp",
                            json!({"tool":"client_a","arguments":{"sentinel":"A"}}),
                            Some(&session),
                        )
                        .await
                        .expect("client_a must execute after its actual Always decision");
                    assert_eq!(
                        first,
                        json!({"name":"execute","arguments":{"sentinel":"A"}})
                    );
                    let persisted: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM action_precedents WHERE origin != 'legacy_allowlist'",
                    )
                    .fetch_one(pool)
                    .await
                    .unwrap();
                    assert_eq!(persisted, 1, "Always must reach real SQLite persistence");

                    let second = registry
                        .execute_with_context(
                            "mcp",
                            json!({"tool":"client_b","arguments":{"sentinel":"B"}}),
                            Some(&session),
                        )
                        .await;

                    // Dropping all dispatch senders closes the subscribed stream
                    // only after requester delivery owners have released theirs.
                    drop(registry);
                    join_owned(approval_actor.take().unwrap(), "approval actor")
                        .await
                        .expect("SEC-01 approval actor must join");

                    let record = approvals.lock().await;
                    assert!(record.errors.is_empty(), "{:?}", record.errors);
                    assert_eq!(record.expired, record.prompts);
                    assert_eq!(guard.pending_count().await, 0);
                    let requests = calls.lock().await;
                    for request in requests.iter() {
                        assert_eq!(request["jsonrpc"], "2.0");
                        assert_eq!(request["method"], "tools/call");
                        assert_eq!(request["params"]["name"], "execute");
                    }
                    assert_eq!(requests[0]["params"]["arguments"], json!({"sentinel":"A"}));
                    let b_denied = matches!(&second, Err(OmonError::Approval(_)));
                    if !b_denied {
                        assert_eq!(
                            second.as_ref().expect("B failed outside approval"),
                            &json!({"name":"execute","arguments":{"sentinel":"B"}})
                        );
                        assert_eq!(requests[1]["params"]["arguments"], json!({"sentinel":"B"}));
                    }
                    println!(
                        "SEC01_OBSERVED {}",
                        json!({
                            "prompts": record.prompts.len(),
                            "remoteCalls": requests.len(),
                            "bDenied": b_denied,
                            "decisions": record.decisions,
                            "persistedGrants": persisted
                        })
                    );
                    assert_eq!(
                        (record.prompts.len(), requests.len(), b_denied),
                        (2, 1, true),
                        "SEC-01: client_b inherited client_a's Always grant for execute"
                    );
                    assert_eq!(record.decisions, ["always", "deny"]);
                })
                .catch_unwind(),
            )
            .await;

            // This runs after behavioral RED, setup panic, or scenario deadline.
            // Collect failures rather than letting one cleanup error skip others.
            let mut cleanup_errors = Vec::new();
            if let Some(actor) = approval_actor.take() {
                if let Err(error) = join_owned(actor, "approval actor").await {
                    cleanup_errors.push(error);
                }
            }
            let pending = guard.pending_count().await;
            if pending != 0 {
                cleanup_errors.push(format!("pending approvals after caller drop: {pending}"));
            }
            guard.clear_session(&session).await;
            stop.cancel();
            if let Some(peer) = server.take() {
                match join_owned(peer, "MCP HTTP server").await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => cleanup_errors.push(format!("MCP HTTP server: {error}")),
                    Err(error) => cleanup_errors.push(error),
                }
            }
            drop(guard);
            if let Some(database) = db.take() {
                if timeout(BOUND, database.pool().close()).await.is_err() {
                    cleanup_errors.push("SQLite pool close deadline exceeded".to_owned());
                }
                drop(database);
            }
            (outcome, cleanup_errors)
        });

        // Do not rethrow the caught oracle until the owned runtime is shut down.
        runtime.shutdown_timeout(BOUND);
        println!(
            "SEC01_CLEANUP {}",
            json!({
                "ok": cleanup_errors.is_empty(),
                "errors": cleanup_errors,
                "runtimeShutdownReturned": true,
                "filesystemFixtures": 0,
                "childProcesses": 0,
                "environmentMutations": 0
            })
        );
        assert!(
            cleanup_errors.is_empty(),
            "SEC-01 cleanup: {cleanup_errors:?}"
        );
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(error) => panic!("SEC-01 scenario deadline exceeded: {error}"),
        }
    }
}

mod sec03 {
    use std::panic::AssertUnwindSafe;
    use std::path::Path;
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use futures_util::FutureExt;
    use omon_gateway::migrate::cron_cutover::{
        cutover_cron_stores, normalize_job_payload, reconcile_pending_cutover,
    };
    use omon_gateway::migrate::sys::{FakeMigrationEnv, MigrationEnv, MigrationOperation};
    use omon_gateway::{Database, OmonError};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use sqlx::SqlitePool;
    use tokio::time::timeout;

    const BOUND: Duration = Duration::from_secs(10);
    // Native separators keep discovery via root.join and fake read_dir consistent
    // when SQLite orders the journal's TEXT store_path values on Windows.
    #[cfg(windows)]
    const ROOT: &str = r"\sec03-owned\.hermes";
    #[cfg(windows)]
    const DEFAULT: &str = r"\sec03-owned\.hermes\cron\jobs.json";
    #[cfg(windows)]
    const WORK: &str = r"\sec03-owned\.hermes\profiles\work\cron\jobs.json";
    #[cfg(not(windows))]
    const ROOT: &str = "/sec03-owned/.hermes";
    #[cfg(not(windows))]
    const DEFAULT: &str = "/sec03-owned/.hermes/cron/jobs.json";
    #[cfg(not(windows))]
    const WORK: &str = "/sec03-owned/.hermes/profiles/work/cron/jobs.json";
    const ORIGINALS: [&[u8]; 2] = [
        br#"{"jobs":[{"id":"daily","name":"Daily","prompt":"status","schedule":{"kind":"cron","expr":"0 9 * * *"},"enabled":true}],"updated_at":"old"}"#,
        br#"{"jobs":[{"id":"standup","name":"Standup","prompt":"standup-check","schedule":{"kind":"cron","expr":"0 10 * * *"},"enabled":true}],"updated_at":"old"}"#,
    ];

    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct Receipt {
        operation_id: String,
        status: String,
        store_count: i64,
        policy_digest: String,
        created_at: String,
        updated_at: String,
    }

    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct Store {
        profile: String,
        store_path: String,
        original_hash: String,
        replacement_hash: String,
        backup_path: String,
        job_digests_json: String,
        phase: String,
        updated_at: String,
    }

    #[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
    struct Job {
        id: String,
        expression: String,
        payload_json: String,
        enabled: i64,
        authority: String,
        updated_at: String,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct State {
        receipts: Vec<Receipt>,
        stores: Vec<Store>,
        jobs: Vec<Job>,
    }

    impl State {
        fn store(&self, profile: &str) -> &Store {
            self.stores
                .iter()
                .find(|store| store.profile == profile)
                .unwrap()
        }
    }

    fn hash(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    async fn state(pool: &SqlitePool) -> State {
        State {
            receipts: sqlx::query_as(
                "SELECT operation_id, status, store_count, policy_digest, created_at, updated_at
                 FROM cron_cutover_receipts ORDER BY operation_id",
            )
            .fetch_all(pool)
            .await
            .unwrap(),
            stores: sqlx::query_as(
                "SELECT profile, store_path, original_hash, replacement_hash, backup_path,
                        job_digests_json, phase, updated_at
                 FROM cron_cutover_receipt_stores ORDER BY store_path",
            )
            .fetch_all(pool)
            .await
            .unwrap(),
            jobs: sqlx::query_as(
                "SELECT id, expression, payload_json, enabled, authority, updated_at
                 FROM cron_jobs ORDER BY id",
            )
            .fetch_all(pool)
            .await
            .unwrap(),
        }
    }

    fn retained(env: &FakeMigrationEnv, current: &State, initial: &State) {
        assert_eq!(current.receipts.len(), 1);
        assert_eq!(current.stores.len(), 2);
        assert_eq!(current.jobs.len(), 2);
        let receipt = &current.receipts[0];
        let original = &initial.receipts[0];
        assert_eq!(receipt.operation_id, original.operation_id);
        assert_eq!(receipt.created_at, original.created_at);
        assert_eq!(receipt.store_count, 2);
        assert_eq!(receipt.policy_digest, "hermes_cutover");
        for (index, profile) in ["default", "work"].into_iter().enumerate() {
            let store = current.store(profile);
            let before = initial.store(profile);
            assert_eq!(store.profile, before.profile);
            assert_eq!(store.store_path, before.store_path);
            assert_eq!(store.original_hash, before.original_hash);
            assert_eq!(store.replacement_hash, before.replacement_hash);
            assert_eq!(store.backup_path, before.backup_path);
            assert_eq!(store.job_digests_json, before.job_digests_json);
            let backup = env.read(Path::new(&store.backup_path)).unwrap();
            assert_eq!(backup, ORIGINALS[index]);
            assert_eq!(hash(&backup), store.original_hash);
        }
        for job in &current.jobs {
            let before = initial
                .jobs
                .iter()
                .find(|before| before.id == job.id)
                .unwrap();
            assert_eq!(job.id, before.id);
            assert_eq!(job.expression, before.expression);
            assert_eq!(job.payload_json, before.payload_json);
            assert_eq!(job.enabled, 1);
        }
        let backups = env
            .write_calls()
            .into_iter()
            .filter(|(path, _)| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(".bak-omon-migration-")
            })
            .count();
        assert_eq!(backups, 2, "recovery must retain the two original backups");
    }

    fn checkpoint(current: &State, status: &str, phases: [&str; 2], authority: &str) {
        assert_eq!(current.receipts[0].status, status);
        for (profile, phase) in ["default", "work"].into_iter().zip(phases) {
            assert_eq!(current.store(profile).phase, phase);
        }
        for job in &current.jobs {
            assert_eq!(job.authority, authority);
        }
    }

    // Inspect only the owned in-memory filesystem and lock event log.
    // Return errors so a resource failure cannot bypass database/runtime cleanup.
    fn resource_errors(env: &FakeMigrationEnv) -> Vec<String> {
        let mut errors = Vec::new();
        let operations = env.operations();
        for source in [DEFAULT, WORK] {
            let parent = Path::new(source).parent().unwrap();
            let lock = parent.join(".jobs.lock");
            let acquired = operations
                .iter()
                .filter(|op| matches!(op, MigrationOperation::LockAcquired(path) if path == &lock))
                .count();
            let released = operations
                .iter()
                .filter(|op| matches!(op, MigrationOperation::LockReleased(path) if path == &lock))
                .count();
            if acquired != released {
                errors.push(format!(
                    "unreleased lock: {} ({acquired}/{released})",
                    lock.display()
                ));
            }
            if env.is_dir(parent) {
                match env.read_dir(parent) {
                    Ok(paths) => {
                        for path in paths {
                            if path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .contains(".tmp-omon-migration-")
                            {
                                errors
                                    .push(format!("atomic temporary retained: {}", path.display()));
                            }
                        }
                    }
                    Err(error) => errors.push(format!("fixture directory: {error}")),
                }
            }
        }
        if operations.iter().any(|op| {
            !matches!(
                op,
                MigrationOperation::LockAcquired(_)
                    | MigrationOperation::LockReleased(_)
                    | MigrationOperation::Write(_)
                    | MigrationOperation::Rename(_, _)
                    | MigrationOperation::RemoveFile(_)
            )
        }) {
            errors.push("unexpected process/service/time operation".to_owned());
        }
        errors
    }

    #[test]
    fn repeated_cutover_recovery() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("SEC-03 fixture runtime");
        let t0 = Utc.with_ymd_and_hms(2026, 9, 8, 0, 0, 0).unwrap();
        let env = FakeMigrationEnv::new(t0);
        let (outcome, cleanup_errors) = runtime.block_on(async {
            let mut db: Option<Database> = None;
            let outcome = timeout(Duration::from_secs(60), AssertUnwindSafe(async {
                db = Some(Database::connect("sqlite::memory:").await.unwrap());
                let pool = db.as_ref().unwrap().pool();
                let expected_replacement = serde_json::to_vec(&json!({
                    "jobs": [], "updated_at": t0.to_rfc3339()
                })).unwrap();
                for (index, (profile, path)) in [("default", DEFAULT), ("work", WORK)].into_iter().enumerate() {
                    env.write(Path::new(path), ORIGINALS[index]).unwrap();
                    let document: Value = serde_json::from_slice(ORIGINALS[index]).unwrap();
                    let job = &document["jobs"][0];
                    sqlx::query(
                        "INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority)
                         VALUES (?, ?, ?, 1, 'hermes_mirror')",
                    )
                    .bind(format!("hermes:{profile}:{}", job["id"].as_str().unwrap()))
                    .bind(job["schedule"]["expr"].as_str().unwrap())
                    .bind(serde_json::to_string(job).unwrap())
                    .execute(pool).await.unwrap();
                    env.set_read_only(Path::new(path), true);
                }

                // T0: the pending transaction commits; the first atomic rename fails.
                let first = cutover_cron_stores(&env, Path::new(ROOT), pool, false).await;
                assert!(matches!(first, Err(OmonError::Config(_))), "T0: {first:?}");
                let initial = state(pool).await;
                assert_eq!(initial.receipts.len(), 1);
                assert_eq!(initial.stores.len(), 2);
                assert_eq!(initial.jobs.len(), 2);
                assert_eq!(initial.receipts[0].store_count, 2);
                assert!(!initial.receipts[0].operation_id.is_empty());
                checkpoint(&initial, "pending", ["backed_up", "backed_up"], "cutover_pending");
                assert!(env.rename_calls().is_empty());
                for (index, (profile, path)) in [("default", DEFAULT), ("work", WORK)].into_iter().enumerate() {
                    let store = initial.store(profile);
                    let document: Value = serde_json::from_slice(ORIGINALS[index]).unwrap();
                    let job = &document["jobs"][0];
                    assert_eq!(store.profile, profile);
                    assert_eq!(Path::new(&store.store_path), Path::new(path));
                    assert_eq!(env.read(Path::new(path)).unwrap(), ORIGINALS[index]);
                    assert_eq!(store.replacement_hash, hash(&expected_replacement));
                    let digests: Value = serde_json::from_str(&store.job_digests_json).unwrap();
                    let payload = serde_json::to_vec(&normalize_job_payload(job)).unwrap();
                    assert_eq!(digests, json!([[job["id"], hash(&payload)]]));
                    let id = format!("hermes:{profile}:{}", job["id"].as_str().unwrap());
                    let imported = initial.jobs.iter().find(|row| row.id == id).unwrap();
                    assert_eq!(imported.id, id);
                    assert_eq!(imported.expression, job["schedule"]["expr"].as_str().unwrap());
                    assert_eq!(serde_json::from_str::<Value>(&imported.payload_json).unwrap(), *job);
                }
                retained(&env, &initial, &initial);
                assert!(resource_errors(&env).is_empty());
                println!("SEC03_T0 {initial:?}");

                // T1: no source edits; release only default and interrupt recovery at work.
                env.set_now(Utc.with_ymd_and_hms(2026, 9, 8, 0, 1, 0).unwrap());
                env.set_read_only(Path::new(DEFAULT), false);
                let second = reconcile_pending_cutover(&env, pool).await;
                assert!(matches!(second, Err(OmonError::Config(_))), "T1: {second:?}");
                let interrupted = state(pool).await;
                retained(&env, &interrupted, &initial);
                checkpoint(&interrupted, "pending", ["replaced", "backed_up"], "cutover_pending");
                let t1_default = env.read(Path::new(DEFAULT)).unwrap();
                assert_eq!(serde_json::from_slice::<Value>(&t1_default).unwrap()["jobs"], json!([]));
                assert_eq!(env.read(Path::new(WORK)).unwrap(), ORIGINALS[1]);
                assert_eq!(env.rename_calls().len(), 1);
                assert_eq!(env.rename_calls()[0].1, Path::new(DEFAULT));
                assert!(resource_errors(&env).is_empty());
                // Observe this mismatch now, but do not short-circuit the literal T2 call.
                let t1_hash = hash(&t1_default);
                println!("SEC03_T1 sourceHash={t1_hash} journalHash={} {interrupted:?}", interrupted.store("default").replacement_hash);

                // T2: release work and invoke the same exported recovery API.
                env.set_now(Utc.with_ymd_and_hms(2026, 9, 8, 0, 2, 0).unwrap());
                env.set_read_only(Path::new(WORK), false);
                let third = reconcile_pending_cutover(&env, pool).await;
                let recovered = state(pool).await;
                retained(&env, &recovered, &initial);
                let final_default = env.read(Path::new(DEFAULT)).unwrap();
                let final_work = env.read(Path::new(WORK)).unwrap();
                println!("SEC03_OBSERVED {}", json!({
                    "thirdCallError": third.is_err(),
                    "receiptStatus": recovered.receipts[0].status,
                    "authorities": recovered.jobs.iter().map(|job| job.authority.as_str()).collect::<Vec<_>>(),
                    "defaultHash": hash(&final_default), "workHash": hash(&final_work),
                    "t1HashJournaled": t1_hash == interrupted.store("default").replacement_hash,
                    "error": third.as_ref().err().map(ToString::to_string)
                }));
                if third.is_err() {
                    assert_eq!(recovered, interrupted, "failed T2 must retain the prior journal");
                    assert_eq!(final_default, t1_default);
                    assert_eq!(final_work, ORIGINALS[1]);
                    assert_eq!(env.rename_calls().len(), 1);
                    assert!(resource_errors(&env).is_empty());
                }
                assert!(third.is_ok(), "SEC-03: T2 recovery rejected its own prior replacement: {third:?}");
                assert_eq!(third.unwrap(), Some(initial.receipts[0].operation_id.clone()));
                checkpoint(&recovered, "committed", ["committed", "committed"], "omon_owned");
                assert_eq!(t1_default, expected_replacement, "T1 must use the journaled replacement bytes");
                assert_eq!(t1_hash, interrupted.store("default").replacement_hash);
                for (profile, bytes) in [("default", &final_default), ("work", &final_work)] {
                    assert_eq!(bytes, &expected_replacement);
                    assert_eq!(hash(bytes), recovered.store(profile).replacement_hash);
                }
                assert_eq!(env.rename_calls().len(), 2);
                assert_eq!(env.rename_calls()[1].1, Path::new(WORK));

                // Anonymous in-memory SQLite cannot survive pool closure. Exercise two
                // fresh recovery invocations instead; do not call this a disk reopen.
                let writes = env.write_calls();
                let renames = env.rename_calls();
                for _ in 0..2 {
                    assert_eq!(reconcile_pending_cutover(&env, pool).await.unwrap(), None);
                    assert_eq!(state(pool).await, recovered);
                    assert_eq!(env.write_calls(), writes);
                    assert_eq!(env.rename_calls(), renames);
                    for path in [DEFAULT, WORK] {
                        assert_eq!(env.read(Path::new(path)).unwrap(), expected_replacement);
                    }
                }
                retained(&env, &state(pool).await, &initial);
            }).catch_unwind()).await;

            let mut cleanup_errors = resource_errors(&env);
            if let Some(database) = db.take() {
                if timeout(BOUND, database.pool().close()).await.is_err() {
                    cleanup_errors.push("SQLite pool close deadline exceeded".to_owned());
                }
                drop(database);
            }
            (outcome, cleanup_errors)
        });
        drop(env);
        runtime.shutdown_timeout(BOUND);
        println!(
            "SEC03_CLEANUP {}",
            json!({
                "ok": cleanup_errors.is_empty(), "errors": cleanup_errors,
                "fixtureEnvDropped": true, "runtimeShutdownReturned": true,
                "filesystemFixtures": 0, "childProcesses": 0, "environmentMutations": 0
            })
        );
        assert!(
            cleanup_errors.is_empty(),
            "SEC-03 cleanup: {cleanup_errors:?}"
        );
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(error) => panic!("SEC-03 scenario deadline exceeded: {error}"),
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum PayloadEdge {
        Replay,
        LegacyOriginal,
        LegacyReplaced,
        Corrupt,
        UnknownCurrentTime,
    }

    // Seed the persisted boundary, not recovery logic. Payload timestamps differ
    // per store and from both receipt timestamps and backup names; whitespace is
    // significant. This also models preparation crossing clock ticks before commit.
    fn payload_edge(edge: PayloadEdge) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 9, 8, 0, 2, 0).unwrap());
        let payloads: [&[u8]; 2] = [
            b"{\"jobs\":[], \"updated_at\":\"2026-09-08T00:00:00.001+00:00\"}\n",
            b"{\"jobs\":[], \"updated_at\":\"2026-09-08T00:00:00.002+00:00\"}\n",
        ];
        let (outcome, cleanup_errors) = runtime.block_on(async {
            let mut db: Option<Database> = None;
            let outcome = timeout(
                Duration::from_secs(60),
                AssertUnwindSafe(async {
                    db = Some(Database::connect("sqlite::memory:").await.unwrap());
                    let pool = db.as_ref().unwrap().pool();
                    sqlx::query(
                        "INSERT INTO cron_cutover_receipts
                         (operation_id, status, store_count, policy_digest, created_at, updated_at)
                         VALUES ('payload-edge', 'pending', 2, 'hermes_cutover',
                                 '2026-09-08T00:00:01+00:00', '2026-09-08T00:01:00+00:00')",
                    )
                    .execute(pool).await.unwrap();
                    for (index, (profile, path)) in
                        [("default", DEFAULT), ("work", WORK)].into_iter().enumerate()
                    {
                        env.write(Path::new(path), ORIGINALS[index]).unwrap();
                        let backup = env.write_unique(
                            &Path::new(path).with_extension("json.bak-omon-migration-edge"),
                            ORIGINALS[index],
                        ).unwrap();
                        let job: Value = serde_json::from_slice::<Value>(ORIGINALS[index])
                            .unwrap()["jobs"][0].clone();
                        sqlx::query(
                            "INSERT INTO cron_jobs (id, expression, payload_json, enabled, authority)
                             VALUES (?, ?, ?, 1, 'cutover_pending')",
                        )
                        .bind(format!("hermes:{profile}:{}", job["id"].as_str().unwrap()))
                        .bind(job["schedule"]["expr"].as_str().unwrap())
                        .bind(job.to_string())
                        .execute(pool).await.unwrap();
                        // Old writers omit the new nullable column. The legacy
                        // work row stays NULL even when its source is already replaced.
                        sqlx::query(
                            "INSERT INTO cron_cutover_receipt_stores
                             (operation_id, profile, store_path, original_hash, replacement_hash,
                              backup_path, job_digests_json, phase, updated_at)
                             VALUES ('payload-edge', ?, ?, ?, ?, ?, '[]', 'backed_up',
                                     '2026-09-08T00:01:00+00:00')",
                        )
                        .bind(profile).bind(path).bind(hash(ORIGINALS[index]))
                        .bind(hash(payloads[index])).bind(backup.to_string_lossy())
                        .execute(pool).await.unwrap();
                        if index == 0 || !matches!(edge,
                            PayloadEdge::LegacyOriginal | PayloadEdge::LegacyReplaced)
                        {
                            let bytes = if index == 1 && matches!(edge, PayloadEdge::Corrupt) {
                                b"damaged journal payload".as_slice()
                            } else {
                                payloads[index]
                            };
                            sqlx::query(
                                "UPDATE cron_cutover_receipt_stores SET replacement_bytes = ?
                                 WHERE profile = ?",
                            )
                            .bind(bytes).bind(profile).execute(pool).await.unwrap();
                        }
                        if matches!(edge, PayloadEdge::LegacyReplaced) {
                            env.write(Path::new(path), payloads[index]).unwrap();
                        }
                    }
                    if matches!(edge, PayloadEdge::UnknownCurrentTime) {
                        let unknown = serde_json::to_vec(&json!({
                            "jobs": [], "updated_at": env.now().to_rfc3339()
                        })).unwrap();
                        env.write(Path::new(DEFAULT), &unknown).unwrap();
                    }
                    let initial = state(pool).await;
                    let journal: Vec<Option<Vec<u8>>> = sqlx::query_scalar(
                        "SELECT replacement_bytes FROM cron_cutover_receipt_stores ORDER BY store_path",
                    ).fetch_all(pool).await.unwrap();
                    let source_bytes = [DEFAULT, WORK].map(|p| env.read(Path::new(p)).unwrap());
                    let writes = env.write_calls();
                    if matches!(edge, PayloadEdge::Replay) {
                        env.set_read_only(Path::new(WORK), true);
                        let first = reconcile_pending_cutover(&env, pool).await;
                        assert!(matches!(first, Err(OmonError::Config(_))), "{first:?}");
                        assert_eq!(env.read(Path::new(DEFAULT)).unwrap(), payloads[0]);
                        assert_eq!(env.read(Path::new(WORK)).unwrap(), ORIGINALS[1]);
                        let interrupted = state(pool).await;
                        checkpoint(&interrupted, "pending", ["replaced", "backed_up"], "cutover_pending");
                        retained(&env, &interrupted, &initial);
                        env.set_read_only(Path::new(WORK), false);
                        env.set_now(Utc.with_ymd_and_hms(2026, 9, 8, 0, 3, 0).unwrap());
                        assert_eq!(reconcile_pending_cutover(&env, pool).await.unwrap(),
                            Some("payload-edge".to_owned()));
                        let committed = state(pool).await;
                        checkpoint(&committed, "committed", ["committed", "committed"], "omon_owned");
                        retained(&env, &committed, &initial);
                        for (index, path) in [DEFAULT, WORK].into_iter().enumerate() {
                            assert_eq!(env.read(Path::new(path)).unwrap(), payloads[index]);
                        }
                        assert_eq!(env.rename_calls().len(), 2);
                    } else {
                        let result = reconcile_pending_cutover(&env, pool).await;
                        assert!(matches!(result, Err(OmonError::Config(_))), "{edge:?}: {result:?}");
                        assert_eq!(state(pool).await, initial);
                        assert_eq!(env.write_calls(), writes);
                        assert!(env.rename_calls().is_empty());
                        for (index, path) in [DEFAULT, WORK].into_iter().enumerate() {
                            assert_eq!(env.read(Path::new(path)).unwrap(), source_bytes[index]);
                        }
                        retained(&env, &state(pool).await, &initial);
                    }
                    let after: Vec<Option<Vec<u8>>> = sqlx::query_scalar(
                        "SELECT replacement_bytes FROM cron_cutover_receipt_stores ORDER BY store_path",
                    ).fetch_all(pool).await.unwrap();
                    assert_eq!(after, journal, "recovery must not rewrite payload evidence");
                    println!("SEC03_PAYLOAD_EDGE {edge:?} ok=true");
                }).catch_unwind(),
            ).await;
            let mut cleanup_errors = resource_errors(&env);
            if let Some(database) = db.take() {
                if timeout(BOUND, database.pool().close()).await.is_err() {
                    cleanup_errors.push("SQLite pool close deadline exceeded".to_owned());
                }
                drop(database);
            }
            (outcome, cleanup_errors)
        });
        drop(env);
        runtime.shutdown_timeout(BOUND);
        println!(
            "SEC03_EDGE_CLEANUP {}",
            json!({
                "edge": format!("{edge:?}"), "ok": cleanup_errors.is_empty(),
                "errors": cleanup_errors, "fixtureEnvDropped": true,
                "runtimeShutdownReturned": true, "filesystemFixtures": 0,
                "childProcesses": 0, "environmentMutations": 0
            })
        );
        assert!(cleanup_errors.is_empty(), "{cleanup_errors:?}");
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(error) => panic!("SEC-03 payload edge deadline exceeded: {error}"),
        }
    }

    #[test]
    fn replacement_payload_replays_exact_per_store_bytes() {
        payload_edge(PayloadEdge::Replay);
    }

    #[test]
    fn replacement_payload_legacy_original_stays_pending() {
        payload_edge(PayloadEdge::LegacyOriginal);
    }

    #[test]
    fn replacement_payload_legacy_replaced_stays_pending() {
        payload_edge(PayloadEdge::LegacyReplaced);
    }

    #[test]
    fn replacement_payload_corruption_preserves_all_stores() {
        payload_edge(PayloadEdge::Corrupt);
    }

    #[test]
    fn replacement_payload_rejects_unjournaled_current_time() {
        payload_edge(PayloadEdge::UnknownCurrentTime);
    }
}
mod sec05_disabled_root_config_import {
    use chrono::{TimeZone, Utc};
    use omon_gateway::migrate::{
        config_import::import_config,
        sys::{FakeMigrationEnv, MigrationEnv},
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
    use std::path::Path;

    #[test]
    fn disabled_root_does_not_activate_env_token() {
        let env = FakeMigrationEnv::new(Utc.with_ymd_and_hms(2026, 8, 15, 14, 30, 45).unwrap());

        // Keep the sole fixture owner outside the caught setup and assertions.
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let root = Path::new("/hermes");
            let target = Path::new("/gateway/fixture.env");
            env.write(
                Path::new("/hermes/config.yaml"),
                b"model:\n  default: fixture\ndiscord:\n  enabled: false\n",
            )
            .unwrap();
            env.write(
                Path::new("/hermes/.env"),
                b"DISCORD_BOT_TOKEN=disabled-root-sentinel\n",
            )
            .unwrap();
            env.create_dir_all(Path::new("/hermes/profiles")).unwrap();
            assert!(env
                .read_dir(Path::new("/hermes/profiles"))
                .unwrap()
                .is_empty());
            assert!(!env.exists(target));

            match import_config(&env, root, target, false) {
                Err(error) => {
                    // Refusal is an explicitly allowed outcome for this fixture.
                    println!(
                        "SEC05_OBSERVED {}",
                        json!({
                            "importSucceeded": false,
                            "error": error.to_string()
                        })
                    );
                }
                Ok(result) => {
                    // Parse persisted bytes in memory, without loading process env.
                    // Inspect both surfaces before the behavioral assertion.
                    let rendered = env.read_to_string(target).unwrap();
                    let persisted = dotenvy::from_read_iter(rendered.as_bytes())
                        .collect::<Result<BTreeMap<String, String>, _>>()
                        .unwrap();
                    let has_root_token = |values: &BTreeMap<String, String>| {
                        ["DISCORD_BOT_TOKEN", "DISCORD_BOT_TOKENS"]
                            .into_iter()
                            .filter_map(|key| values.get(key))
                            .flat_map(|value| value.split(','))
                            .any(|token| {
                                token.trim().trim_matches('"').trim_matches('\'')
                                    == "disabled-root-sentinel"
                            })
                    };
                    let returned_active = has_root_token(&result.values);
                    let persisted_active = has_root_token(&persisted);
                    println!(
                        "SEC05_OBSERVED {}",
                        json!({
                            "importSucceeded": true,
                            "activeRootTokenInValues": returned_active,
                            "activeRootTokenInTarget": persisted_active
                        })
                    );
                    assert_eq!(
                        (returned_active, persisted_active),
                        (false, false),
                        "SEC-05: disabled root emitted an active Discord token"
                    );
                }
            }
        }));

        // All files, directories, write history and any temporary paths are owned
        // by this in-memory fake. Release them even after setup/assertion panic.
        drop(env);
        println!(
            "SEC05_CLEANUP {}",
            json!({
                "fixtureDropped": true,
                "filesystemFixtures": 0,
                "childProcesses": 0,
                "environmentMutations": 0
            })
        );
        if let Err(panic) = outcome {
            resume_unwind(panic);
        }
    }
}

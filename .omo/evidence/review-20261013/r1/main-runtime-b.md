# main-runtime-b review
### [P1] RuntimeOwnershipLock is never acquired by any production path
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs:241 (definition), /Users/indo/code/project/omon-gateway/src/entry.rs:155 (`run_gateway_with_optional_dashboard`), /Users/indo/code/project/omon-gateway/src/entry.rs:113 (`main`)
- Trigger: An operator runs `omo-gateway run` by hand while a launchd or systemd instance of the same gateway is already running, with the same DATABASE_URL, workspace and Discord tokens. A grep for `try_acquire` and `RuntimeOwnership` across src/ finds callers only in entry.rs:318-330, which is the test `duplicate_gateway_start_is_rejected`. No call exists in `main`, `run_gateway_with_optional_dashboard`, `run_gateway` or `run_gateway_with_dashboard` (main.rs:702-710).
- Impact: Both processes start Discord clients, multiplexers, cron schedulers, resume-pending recovery and delivery-obligation recovery against the same SQLite store. That means duplicate replies, double-executed cron jobs, and the same resume-pending turn replayed twice. The unit test passes, so it gives false assurance that duplicate starts are rejected. The earlier review round already reported this and it is still present in the working tree.
- Fix: Acquire `RuntimeOwnershipLock::try_acquire(<name derived from workspace root or DB path>)` in `run_gateway_with_optional_dashboard` before any of the three startup branches. Bind the guard to a local that lives until return, as in `let _lock = ...?;`. Do the same for the standalone `dashboard` and `serve` commands if they start the scheduler.

### [P2] Lock file is created in the shared temp dir with default permissions and no symlink protection
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs:247-263
- Trigger: `OMON_RUNTIME_LOCK_DIR` is unset, so the lock lives in `std::env::temp_dir()`, which is `/tmp` on Linux. Another local user pre-creates `/tmp/omon-gateway-<name>.lock`, either as a file we cannot write or as a symlink.
- Impact: Startup fails with "failed to open runtime lock" (a denial of service), or `create(true)` follows the symlink and creates a file at the attacker-chosen target. The lock name is not scoped to the DB or workspace, so two unrelated gateways under the same temp dir would also collide if the caller reuses a fixed name. This only matters once the lock is wired in.
- Fix: Default the lock dir to the workspace or state directory, not the shared temp dir. Open with mode 0o600 and `custom_flags(libc::O_NOFOLLOW)` on Unix.

### [P2] Gateway shutdown error is masked by a dashboard error in the Discord-disabled path
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs:181-185
- Trigger: With `OMON_DISCORD_ENABLED=false`, the dashboard enabled and the gateway future finishing first (for example with a lanes error), `dashboard.await?` runs after `shutdown.cancel()`. If the dashboard returns `Err` during that teardown, `?` returns it and the original gateway result is dropped.
- Impact: The operator sees an unrelated dashboard error instead of the real gateway failure.
- Fix: Bind `let dash = dashboard.await;` and return `result` first, logging the dashboard error. For example: `result.and_then(|_| dash)`.

### [P2] `profile` subcommands open or create a DB at a default path that differs from the gateway's
- Location: /Users/indo/code/project/omon-gateway/src/entry.rs:130-136
- Trigger: `omo-gateway profile export <bot>` is run in a shell where `DATABASE_URL` is unset. The gateway requires `DATABASE_URL` (`required_env` at main.rs:714), but this code falls back to `sqlite://omon_gateway.db` in the current directory. `Database::connect` probably creates and migrates that file; I did not read its source.
- Impact: A stray empty DB is created in the cwd, and the command reports "unknown bot id" (exit 2) instead of a configuration error.
- Fix: Use the same required-env resolution as the gateway and fail with exit 1 when `DATABASE_URL` is unset.

### [P2] Credential readiness check is effectively provider-agnostic and misclassifies models
- Location: /Users/indo/code/project/omon-gateway/src/readiness.rs:187-229
- Trigger: The default model is `gpt-...` and only `ANTHROPIC_API_KEY` is set. The `has_any_api_key` fallback reports "credentials present" for the wrong provider. Separately, `model.contains("o1")` and `contains("o3")` match unrelated model names, and `env::var(..).is_ok()` treats an empty variable as present.
- Impact: Readiness is wrongly green or wrongly degraded. It is only logged at startup (main.rs:1140-1150), so the impact is operator confusion.
- Fix: Require the provider-specific key (or `LLM_API_KEY`) and filter out empty values. Match provider prefixes explicitly instead of using substring checks.

### [P2] Backend readiness probe goes through env proxies
- Location: /Users/indo/code/project/omon-gateway/src/readiness.rs:251-256
- Trigger: `HTTP_PROXY` or `HTTPS_PROXY` is set without a `NO_PROXY` entry for 127.0.0.1. `reqwest::Client::builder()` uses system proxies by default.
- Impact: The loopback probe is sent to the proxy and reports "backend unreachable" while the daemon is healthy. `/api/ready` in dashboard.rs:878 reuses this probe, so it can return 503 spuriously.
- Fix: Add `.no_proxy()` to the client builder.

### [P2] Drain marker writer leaks temp files, and the clear path has a TOCTOU error
- Location: /Users/indo/code/project/omon-gateway/src/drain_control.rs:163-186 and :189-199
- Trigger: In `write_drain_request`, `fs::rename` fails (for example the target is a directory, or on a permissions error). The `.drain_request.<uuid>.tmp` file is never removed. In `clear_drain_request`, the marker is removed by another process between `path.exists()` and `remove_file`, which returns an error.
- Impact: Stray temp files accumulate in the workspace root. Clearing a drain spuriously fails with an error although the marker is gone.
- Fix: On rename failure, `let _ = fs::remove_file(&tmp_path)`. In `clear_drain_request`, call `remove_file` directly and map `ErrorKind::NotFound` to `Ok(false)`.

### [P2] Tests in main.rs mutate process-global env and weakly assert drain behavior
- Location: /Users/indo/code/project/omon-gateway/src/main.rs:3224-3255 (`default_model_precedence_matches_dashboard`), :3283-3300 (`cron_and_interactive_share_one_daemon`), :3305-3385 (`drain_is_reversible_and_preserves_accepted_work`)
- Trigger: The two env tests call `set_var` and `remove_var` for `OMON_DEFAULT_MODEL`, `DEFAULT_MODEL` and `OMON_OMO_APPSERVER_URL` under the default parallel test runner. Any other test in the same binary that reads these variables can observe inconsistent values. `drain_is_reversible_and_preserves_accepted_work` creates a `processed` counter but never asserts on it, so it does not check the "preserves accepted work" part of its name.
- Impact: Possible flaky tests, and a test that does not verify its stated property.
- Fix: Serialize the env tests behind a shared mutex or `serial_test`. Assert that `processed` reaches 2 using the signal-based wait pattern (subscribe first, then await with a timeout) from the other tests.

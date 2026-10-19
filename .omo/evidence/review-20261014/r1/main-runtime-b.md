# Lane main-runtime-b

I reviewed src/main.rs from line 2600 to the end, plus src/entry.rs, src/drain_control.rs and src/readiness.rs. I found no P0 or P1 defects and three P2 items. I did not re-report anything already recorded in CONFIRMED.md or CONFIRMED-r3.md. Everything is from reading the code; I ran no builds or tests, as instructed. I re-read each cited line, and the quotes below match.

### [P2] Runtime lock name does not normalize the DB path, so two spellings of one DB get different locks
- Location: src/entry.rs:346
- Trigger: One gateway runs with `DATABASE_URL=sqlite:///Users/x/gw/omon.db` (for example under launchd). The operator starts a second `run` with `DATABASE_URL=sqlite://./omon.db` from `/Users/x/gw`. Symlinked or `..` spellings, such as `/var/...` versus `/private/var/...`, behave the same way.
- Impact: Both processes open the same SQLite file but hash to different `omon-gateway-<hash>.lock` files. Both acquire their lock, which recreates the duplicate-gateway failure the lock exists to prevent: double replies, double cron runs and two Discord sessions on the same tokens. The doc comment promises "two gateways on the same database always do [collide]", and `runtime_lock_name_is_scoped_to_the_database` only covers `sqlite:` versus `sqlite://` and `?mode=`.
- Evidence: `std::env::current_dir().map(|cwd| cwd.join(path).to_string_lossy().into_owned())` (line 350). `Path::join` keeps the `./` component, and the absolute branch at line 346 uses `path.to_owned()` with no canonicalization. So `/cwd/./omon.db` hashes differently from `/cwd/omon.db`.
- Fix: Lexically normalize the path before hashing, dropping `.` and resolving `..`. Better still, canonicalize the parent directory (`std::fs::canonicalize(parent)`) and join the file name. The parent exists by the time the lock is taken, or the lock can fall back to the normalized form.

### [P2] `approval_timeout_secs_from` accepts any u64, and callers do `+ 5` on it
- Location: src/main.rs:2606
- Trigger: `APPROVAL_TIMEOUT_SECS=18446744073709551615`, or any value of 2^64-5 or more.
- Impact: `Config::from_env` stores the value unclamped. src/main.rs:939 and src/main.rs:946 then compute `config.approval_timeout_secs + 5`. A debug build panics at startup. A release build wraps (I saw no `overflow-checks` setting in Cargo.toml), so the approval timeout becomes a few seconds, and every approval request times out almost immediately. The dashboard path already uses `saturating_add` (dashboard_runtime.rs:113), so the two paths disagree.
- Evidence: `.filter(|secs| *secs > 0).unwrap_or(900)` has no upper bound, and the caller does `std::time::Duration::from_secs(config.approval_timeout_secs + 5)`.
- Fix: Clamp in the helper, for example `.map(|secs| secs.min(7 * 24 * 3600))`, or use `saturating_add(5)` at the main.rs callers.

### [P2] `cron_script_timeout_reaps_descendants` depends on a fixed sleep and an unchecked pid-file read
- Location: src/main.rs:2722
- Trigger: A loaded host, such as the heavily loaded one this review is running on. The pid file is not yet written when the 1s script timeout fires. Alternatively, the orphaned `sleep` is still a zombie, or not yet reaped, when `kill(pid, 0)` runs 100ms after the timeout.
- Impact: The test can panic on `parse().unwrap()` at line 2718, or fail the `assert_ne!(ret, 0, ...)` check. Either way it reports a spurious "descendant leaked" failure with no code defect behind it.
- Evidence: `let pid: i32 = pid_str.trim().parse().unwrap();` (line 2718), with `unwrap_or_default()` on the read just above it. Then `tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;` followed by `libc::kill(pid, 0)`.
- Fix: Have the script create the pid file before `wait`. Wait for the pid file to appear with a bounded, event-driven wait (a `monitor`-style or notify-based wait). Replace the fixed sleep with a bounded loop that waits for `kill(pid, 0)` to fail with ESRCH, with a timeout, so it does not depend on scheduling luck.
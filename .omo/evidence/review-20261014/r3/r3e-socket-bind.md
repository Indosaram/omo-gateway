# Lane r3e-socket-bind

### [P2] Staging dir briefly has umask-default mode before the chmod to 0700
- **Location:** /Users/indo/code/project/omon-gateway/src/cron/report.rs, `create_dir(&staging)`, then `set_permissions(&staging, 0o700)`, at about lines 293-299.
- **Trigger:** The staging dir is created with `0777 & ~umask`, usually 0755, and stays that way until the chmod runs.
- **Impact:** Your example (0755 staging dir, no socket yet) is not a real exposure: the dir is empty and other users cannot write into a 0755 dir. Two cases are slightly worse:
  - If the umask makes the parent group-writable (for example `umask 002`), a group member could create `<staging>/s` during the window. `bind` would then fail with EADDRINUSE. That is a denial of service only: `bind` does not follow an existing symlink, so nothing can be hijacked.
  - Between `bind` and the 0600 chmod, the socket itself has umask-default mode. It sits inside a dir that is already 0700, so nobody else can traverse to it.

  After the rename, the socket at the final path is never more permissive than 0600.
- **Evidence:** I read the code order. The chmod to 0700 happens before `UnixListener::bind`.
- **Fix (optional):** Replace the two steps with `std::fs::DirBuilder::new().mode(0o700).create(&staging)`, using `std::os::unix::fs::DirBuilderExt`. This closes the window entirely. Not required.

### [P2] Leftover `.cs<8hex>` dirs if the process dies mid-bind
- **Location:** Same block, at about lines 293-306.
- **Trigger:** The process crashes or is SIGKILLed between `create_dir` and `remove_dir_all`.
- **Impact:** An empty or socket-containing `.csXXXXXXXX` dir is left in `~/.omon`. The names are random, so a leftover never blocks a later bind; it is just clutter. A 32-bit name collision would surface as an AlreadyExists error mapped to `OmonError::Config`, with no retry. The odds are negligible.
- **Fix (optional):** At startup, sweep `.cs*` dirs older than N minutes. Not needed for F17.

### [P2] The stale-socket check and the rename can race with a second gateway (not a regression)
- **Location:** The exists, connect and remove_file check (about lines 271-282), then `rename(&staged, &path)`.
- **Trigger:** Two gateways start at almost the same moment.
- **Impact:** `rename` silently replaces whatever is at `path`. The old remove-then-`bind` code had the same race, because the second `remove_file` could delete the first gateway's fresh socket. So this is no worse than before.
- **Fix:** Only if a single-instance guarantee matters here: take an flock on a sibling lock file around the check-and-rename.

### Check results
1. **Socket still works after rename.** A listening socket is bound to its inode, and `rename` only moves the directory entry. `connect(path)` resolves the path to that inode, so it works on both Linux and macOS, which matches the lead's Python check. The listener's `getsockname` keeps reporting the old staged path, but nothing in this file reads `local_addr`. Staging is in the same parent dir as the final path, so EXDEV cannot happen. The final path is 0600 from the moment it exists.
2. **Path length.** With the new name, the staged path is 4 bytes longer than the final one ("/.cs" + 8 hex + "/s" replaces "/cron.sock"). I didn't recompute the lead's exact 77/73 figures, but that difference matches them. Both are well under macOS's 104-byte limit and Linux's 108. Production (about 41 bytes) is fine.
3. **Compiles and passes clippy (by reading only).**
   - `uuid` 1.12 with the `v4` feature is in Cargo.toml, so `new_v4()` is available.
   - `Simple` implements Display, so `.to_string()[..8]` works. The temporary String lives until the end of the `format!` statement, so slicing it is sound.
   - The block-scoped `use std::os::unix::fs::PermissionsExt` is used by both `from_mode` calls.
   - The `std::io::Result::Ok(listener)` annotation fixes the error type that `?` converts to inside the async block.
   - No `libc::` remains in report.rs, and `libc` is still used across the crate (executor.rs, ack.rs, main.rs and others), so there is no unused-dependency problem.
   - `let _ = remove_dir_all(...)` does not trip clippy's default lints, and ignoring a cleanup error is acceptable here.
4. **Other P0/P1:** none. Both callers (`bind_native` and main.rs:881, plus the tests at main.rs:1489 and tests/test_fix_r5_cron_exec.rs:390) only need `bind` to return a listening server at `path`, and it still does.

## Verdict
0 P0 / 0 P1 (3 optional P2s); F17 VERIFIED: the process-wide umask is gone, the socket reaches its final path at 0600, and both path lengths fit the sun_path limit.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/cron/report.rs - `NativeServer::bind` (about lines 266-310) contains the F17 fix: a short staging dir, bind, chmod to 0600, then rename.
- /Users/indo/code/project/omon-gateway/Cargo.toml - `uuid` 1.12 with the `v4` feature (line 50) and `libc` (line 53), which other files still use.
- /Users/indo/code/project/omon-gateway/src/main.rs - callers at 881 (`bind_native`) and 1489 (a tempdir test).
- /Users/indo/code/project/omon-gateway/tests/test_fix_r5_cron_exec.rs - tempdir bind test at 390, which also checks the socket mode.
</files>
<answer>The current code is correct. Renaming a listening Unix socket keeps it reachable at the new path. The socket is 0600 before it appears at the final path. The 8-hex staging name keeps the staged path 4 bytes longer than the final one, well inside the 104-byte macOS limit. The code should compile and pass clippy with -D warnings. The remaining issues are optional P2 hardening: creating the staging dir with DirBuilder mode 0700, clearing out leftover `.cs*` dirs, and the start-up race between two gateways that existed before this change.</answer>
<next_steps>None required. If you want, swap create_dir plus chmod for `DirBuilder::new().mode(0o700)` to remove the brief window before the chmod.</next_steps>
</results>
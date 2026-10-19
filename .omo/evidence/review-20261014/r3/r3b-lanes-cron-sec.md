# Lane r3b-lanes-cron-sec

Limit: the gh/curl-only tool here can't run `git diff HEAD`, so I reviewed the current contents of the scoped files at the fix sites. The P2 below is the only issue I found; there are no P0/P1 findings.

### [P2] Remote worktree check on Linux uses `st_ctime` as a stand-in for creation time
- **Location:** `/Users/indo/code/project/omon-gateway/src/sessions/external.rs:489`, in `include()`: `getattr(st,'st_birthtime',st.st_ctime)>=start`
- **Trigger:** The remote host runs Linux, which has no `st_birthtime`. A linked worktree outside the cwd that is older than the lane has its inode metadata changed after the lane started (chmod, touch-on-dir, or adding/removing entries in that directory).
- **Impact:** The check includes that old worktree. If it has unrelated uncommitted or unpushed work, the lane's Done gets rejected wrongly. This errs on the safe side: Done is refused, it is never accepted wrongly, and no data is lost. The local check in `settle.rs:610-660` (`external_creation_time`) is stricter: when creation time isn't available, it falls back to the git registration directory, verified through its `gitdir` back-pointer.
- **Evidence:** The script line quoted above, compared with how `external_creation_time` handles `ErrorKind::Unsupported`.
- **Fix:** Optional. On Linux, fall back to the mtime of `git rev-parse --absolute-git-dir` for that worktree, which mirrors the local logic. Otherwise, document that the remote check is conservative.

## F16 check details
- **Rust literal and escaping:** The script has no inner `"` and no backslashes other than `\n`. The Python is valid: inline `if ...: raise` statements, one-space-indented bodies for `def git`, `def include` and the `for` loop, and a top-level `if p.returncode:` block.
- **Arguments:** `sys.argv[1]` is the cwd (followed by `os.chdir`) and `sys.argv[2]` is the start time, guarded by `len(sys.argv)>2 and sys.argv[2]`. The Rust side passes `lane.cwd` and then `format!("{:.3}", ms/1000.0)`, or `""` when `created_at` is None. `Lane::new` always sets `created_at: Some(now)` (`storage/lanes.rs:70`), so the None path (which includes every worktree) can't be reached through adopt.
- **Path prefix:** `path==cwd or path.startswith(cwd+os.sep)` matches local `Path::starts_with`, and the `os.sep` stops `/a/b` from matching `/a/bc`. `os.getcwd()` resolves symlinks, and git's porcelain worktree paths are already resolved, so they compare consistently.
- **Same results as the local check:**
  - Missing worktree paths are skipped (`not os.path.exists` returns False).
  - Duplicates are removed by `dict.fromkeys`.
  - `git log HEAD --not --remotes --oneline` only runs when `git remote` is non-empty.
  - A non-repository cwd exits 0, as local does.
  - When the cwd is a subdirectory, the old main worktree at the top level is excluded in both versions. The `status` run in the cwd still covers the whole repo.
- **`lead` shadowing:** `let lead = lead.trim();` comes before validation. The later `.trim()` inside `from_storage_key` is redundant but harmless. `Lane::new(&key, goal, lead, ...)` stores the trimmed value, and nothing later uses the untrimmed one.

## The other fixes (no new issues)
- **F2 (`settle.rs:288-340`):** The file list uses `--no-relative`, and the working-tree side is read from `toplevel.join(path)`. `{revision}:{path}` resolves from the repo root, so it's correct.
- **F11 (`runner.rs:1337-1343`):** The activity reset at turn start is limited by `status = 'working'`.
- **F14 (`guard.rs:593, 683-690`):** The 10-minute backoff uses `julianday` arithmetic and still honours the "lane row changed" condition. The test ages `lane_events.created_at` to `backoff+1` minutes. There are no sleeps in the test, and it imports `LAUNCH_RETRY_BACKOFF` publicly.
- **F3 (`scheduler.rs:2806-2840`):** Mapping 7 to 1 and `d` to `d+1` is correct. `*/2` passes through unchanged and still means Sun/Tue/Thu/Sat under both numberings. Names and `*` also pass through.
- **F4 (`hardline.rs:27`):** The system-dir alternation needs the tail (whitespace, end of string, or a separator), so `/usr/local/x` isn't caught but `/usr`, `/usr/` and `/usr/*` are.

## Verdict
0 P0, 0 P1, 1 P2. F16: VERIFIED.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/sessions/external.rs - F16 remote worktree script (line 489), arguments (lines 490-503), lead trim (lines 94-112)
- /Users/indo/code/project/omon-gateway/src/lanes/settle.rs - F2 top-level diff (lines 288-340); the local include_worktree and external_creation_time it is compared against (lines 543-660)
- /Users/indo/code/project/omon-gateway/src/lanes/guard.rs - F14 LAUNCH_RETRY_BACKOFF (line 593) and its query (lines 683-690)
- /Users/indo/code/project/omon-gateway/src/lanes/runner.rs - F11 activity reset (lines 1337-1343)
- /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs - F3 unix_day_of_week_to_crate (line 2810) and its tests (line 2929)
- /Users/indo/code/project/omon-gateway/src/security/hardline.rs - F4 system-dir rm path regex (line 27)
- /Users/indo/code/project/omon-gateway/tests/test_fix_fix_lanes_guard.rs - backoff test (lines 238-271)
- /Users/indo/code/project/omon-gateway/src/storage/lanes.rs - Lane::new sets created_at Some(now) (line 70)
</files>
<answer>0 P0/P1. One optional P2: on Linux the remote check uses ctime as a stand-in for creation time, which can wrongly reject Done but never wrongly accepts it. F16 is VERIFIED: the escaping, argument order, path-prefix logic, missing-path skip, the `--not --remotes` log, and the lead trim are all correct.</answer>
<next_steps>Ready to proceed. The P2 is an optional follow-up.</next_steps>
</results>
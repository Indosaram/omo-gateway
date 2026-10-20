# Lane agent-daemon-deploy

I reviewed every hunk of the diff: `scripts/deploy.sh`, `scripts/gateway_watchdog.sh`, `src/agent/omo_daemon.rs` and `src/readiness.rs`. I checked each finding against the current files. Three bash behaviors were reproduced with `bash -c` probes on this machine: the `--verify` recursion, `set -e` aborting on a failed `sed` of a missing file, and the same abort when `launchctl print` fails (exit 113).

The Rust changes have no P0/P1 issues. The checked areas are the slow-start supervision, the liveness kill, `ready_wait`, `app_server_patch_state`, the `daemon_command` PATH prepend, the new `owned_daemon_responsive`/`forced_restart_since`/`wait_owned_daemon_ready` and how `omo_backend.rs` uses them, and the Slack readiness wiring in `main.rs:1671-1727`. All four findings below are in `scripts/deploy.sh`.

### [P1] `deploy.sh --verify` recurses forever when the gateway is not running from `$INSTALL_BIN`
- Location: scripts/deploy.sh:212
- Trigger: Run `scripts/deploy.sh --verify` while the gateway is down, or while it still runs from `target/release` (the exact case the comment says this code handles).
- Impact: `gateway_pid` is redefined to call `detect_old_pid`, and `detect_old_pid` calls `gateway_pid` again. That recursion never ends. Instead of reporting "running gateway is not healthy" (or verifying the legacy process), the health check hangs until bash runs out of stack and crashes. A probe confirmed the recursion keeps going (it reached subshell depth 16 before my guard stopped it).
- Evidence:
  ```
  92 detect_old_pid() {
  93     local pid
  94     pid="$(gateway_pid)"
  95     if [[ -z "$pid" ]]; then
  96         pid="$(pgrep -f "$TARGET_BIN run" | head -1 || true)"
  ...
  212     if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { detect_old_pid; }; fi
  ```
- Fix: Do not route through `detect_old_pid`. Use `if [[ -z "$(gateway_pid)" ]]; then gateway_pid() { pgrep -f "$TARGET_BIN run" | head -1 || true; }; fi`.

### [P1] Deploy exits silently after installing, without restarting, when the launchd job is not loaded
- Location: scripts/deploy.sh:329
- Trigger: Run `scripts/deploy.sh` while `ai.omon.gateway` is not loaded (for example after a manual `launchctl bootout`, or after an earlier deploy where bootstrap failed, which the 2026-10-16 comment in `restart()` describes) and the plist already matches (`plist_modified=0`).
- Impact: Under `set -euo pipefail`, the failing `launchctl print` makes the command substitution fail, so the script exits right there with no message. This happens after the new binary, the web bundle and `DEPLOYED` are already installed, but before `restart` and `verify`. The gateway stays down and `DEPLOYED` names a revision that was never started. A probe reproduced the silent exit (code 113).
- Evidence:
  ```
  set -euo pipefail                                   # line 27
  ...
  329         loaded_env="$(launchctl print "gui/${UID:-$(id -u)}/$LABEL" 2>/dev/null \
  330             | sed -n '/^\tenvironment = {/,/^\t}/p' | sed -n 's/^\t\t\([^ ]*\) => \(.*\)$/\1=\2/p' | sort)"
  ```
- Fix: Add `|| true` inside the substitution: `loaded_env="$( { launchctl print ... || true; } | sed ... | sort)"`. The existing `[[ -n "$loaded_env" ]]` branches already handle an empty result, and `restart()` falls back to `bootstrap`.

### [P1] `DEPLOYED` is written before verification and not restored on rollback, so the rewind guard pins a failed revision
- Location: scripts/deploy.sh:302
- Trigger: A deploy whose new build fails `verify` gets rolled back (lines 354-370). Later, another worktree deploys a fix that does not contain the failed commit.
- Impact: After the rollback, `~/.omon/bin/DEPLOYED` still says `rev=<failed build>`, but the old binary is what runs. The rewind guard at line 221 then refuses every later deploy that does not contain the failed revision ("HEAD does not contain the deployed revision ..."), and `DEPLOYED` misreports what is running. This contradicts the plan (`.omo/plans/autonomy-two-axis.md:608`: "a probe failure makes deploy.sh roll back, and DEPLOYED keeps the old rev").
- Evidence:
  ```
  302 printf 'rev=%s dirty_paths=%s source=%s deployed_at=%s\n' "$source_rev" "$source_dirty" "$REPO" "$stamp" \
  303     > "$INSTALL_DIR/DEPLOYED"
  ...
  343 [[ -n "$old_pid" ]] && wait_for_idle_sessions
  ...
  347 if verify "$since" "$old_pid"; then
  348     say "deploy OK"
  ```
- Fix: Write `DEPLOYED` only after `verify` succeeds (move the `printf` into the `if verify ...; then` branch before `exit 0`). Alternatively, back it up next to `$backup` and restore it in the rollback branch.

### [P2] First deploy with no `DEPLOYED` file exits silently under `set -e`
- Location: scripts/deploy.sh:221
- Trigger: Run `scripts/deploy.sh` with a fresh or overridden `INSTALL_BIN`, where `$INSTALL_DIR/DEPLOYED` does not exist yet.
- Impact: `sed` exits 2 on the missing file, `pipefail` makes the substitution fail, and the script exits with no message before doing anything. A probe confirmed it. Nothing gets damaged, but a first deploy is impossible without creating the file by hand. I graded it P2 because only a first-time setup can hit it; this machine already has the file.
- Evidence:
  ```
  221 deployed_rev="$(sed -n 's/^rev=\([0-9a-f]*\).*/\1/p' "$INSTALL_DIR/DEPLOYED" 2>/dev/null | head -1)"
  ```
- Fix: Use `deployed_rev="$(sed -n '...' "$INSTALL_DIR/DEPLOYED" 2>/dev/null | head -1 || true)"`.

### [P2] `deploy.sh` lost its executable bit
- Location: scripts/deploy.sh:1
- Trigger: Run `scripts/deploy.sh` in any fresh checkout or worktree. That is the documented command (header line 4, `.omo/plans/*.md`), and the script itself says several worktrees deploy.
- Impact: "permission denied". The workaround is `bash scripts/deploy.sh`. The error is loud, so nothing goes wrong silently.
- Evidence: The diff header shows `old mode 100755` / `new mode 100644`. `git ls-files -s` shows `100644 ... scripts/deploy.sh`, and the working file is `-rw-r--r--`.
- Fix: `git update-index --chmod=+x scripts/deploy.sh` (and `chmod +x`).

<results>
<files>
- /Users/indo/code/project/omon-gateway/scripts/deploy.sh - all five findings: the recursion at :212, the silent abort at :329, `DEPLOYED` written before verify at :302, the missing-file abort at :221, and the mode change
- /Users/indo/code/project/omon-gateway/scripts/gateway_watchdog.sh - reviewed; no findings
- /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs - reviewed slow-start supervision (:579-648), the liveness kill (:697-742), `ready_wait`, the patch check, the PATH prepend, and the new public helpers; no findings
- /Users/indo/code/project/omon-gateway/src/readiness.rs - reviewed `SlackReadiness`/`probe_slack`/`collect_runtime_readiness_with_slack`; no findings
- /Users/indo/code/project/omon-gateway/src/main.rs - :1671-1727 confirms the Slack readiness wiring is reached in production
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - :2434-2554 confirms the production callers of the new daemon helpers (WIP included)
</files>

<answer>
There are three P1 bugs and two P2 issues, all in `scripts/deploy.sh`. The Rust daemon and readiness changes have no actionable issues.
- **P1:** `--verify` recurses forever when the gateway is not running from `~/.omon/bin`.
- **P1:** `set -e` with `pipefail` silently stops a deploy after install and before restart when the launchd job is not loaded.
- **P1:** `DEPLOYED` is written before verification and never restored on rollback, so the rewind guard then pins a failed revision.
- **P2:** A first deploy with no `DEPLOYED` file exits silently.
- **P2:** The script lost its executable bit.
</answer>

<next_steps>
Apply the one-line fixes listed in each finding. All are in `scripts/deploy.sh`, and none touch Rust code.
</next_steps>
</results>
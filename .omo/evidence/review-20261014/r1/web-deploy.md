# Lane web-deploy

I found three new defects; none of them repeats an item in CONFIRMED.md. The two P1s are in deployment surfaces, not in the dashboard: the Docker container can't start, and `deploy.sh`'s rollback can fail. I found no dashboard payload bug, XSS or token leak. I read and grepped only; nothing was built, run or tested.

### [P1] The Docker image has no `omo` CLI, so every container boot fails and restarts in a loop
- Location: Dockerfile:17-18 (runtime packages), docker-compose.yml:8 and :11-25 (`restart: unless-stopped`, no backend settings)
- Trigger: `docker compose up -d`, the path the README documents.
  - The runtime stage installs only `ca-certificates libopus0 fonts-dejavu-core fonts-noto-cjk`. It has no `omo` CLI and no bun/node.
  - Compose sets neither `OMON_OMO_APPSERVER_URL` nor `OMON_OMO_AUTOSPAWN`, so the backend URL defaults to `ws://127.0.0.1:19742` (src/agent/omo_config.rs:265) and autospawn stays on (src/agent/omo_daemon.rs:443-451).
  - `OmoDaemonSupervisor::ensure` finds nothing listening and spawns `omo app-server`. `resolve_daemon_bin` returns a bare `omo` that isn't on any path (omo_daemon.rs:476-479), so `.spawn()` returns ENOENT. It is mapped to `Err("failed to spawn 'omo app-server' ... is the omo CLI installed?")` (omo_daemon.rs:506-515).
  - Every run path propagates that error with `?`: src/main.rs:1003 (Discord gateway), main.rs:828 (Discord-free path), src/dashboard_runtime.rs:147 (standalone dashboard).
- Impact: The gateway exits during startup on every boot, and `restart: unless-stopped` turns that into an endless crash loop. The Docker deployment cannot serve a single turn.
- Grading: I graded this P1, not P0, to match the earlier ruling that Docker is not the production surface (production runs under macOS launchd). Under the literal rubric (crash with a reachable trigger) it would be P0.
- Evidence:
  - Dockerfile:18: `apt-get install -y --no-install-recommends ca-certificates libopus0 fonts-dejavu-core fonts-noto-cjk`.
  - main.rs:1003: `let _daemon_supervisor = OmoDaemonSupervisor::ensure(&omo_config).await?;`.
  - None of the compose `environment:` keys (lines 12-25) set any `OMON_OMO_*` variable.
- Fix: Pick one of these.
  - Install the omo/senpi CLI and its runtime in the image.
  - Or have compose point at an external daemon: set `OMON_OMO_AUTOSPAWN=off`, `OMON_OMO_APPSERVER_URL=ws://<host>:19742` and `OMON_OMO_APPSERVER_AUTH_TOKEN`, and document it.

### [P1] `deploy.sh` rollback overwrites the signed binary in place, so the restored binary can be killed at launch
- Location: scripts/deploy.sh:97 (`cp -p "$backup" "$BIN"`), then the `restart` call at :100
- Trigger: A deploy whose new build fails `verify`, for example Discord is not ready within 120s. The rollback runs `cp -p` onto the existing `target/release/omo-gateway`. `cp` truncates and rewrites the same inode, which the failing gateway is usually still running from. Then `launchctl kickstart -k` relaunches that path.
- Impact: macOS caches the code signature per vnode, and Apple's guidance is to replace signed code by rename, never by rewriting it in place. A signed Mach-O whose bytes change in place can be SIGKILLed at exec ("Code Signature Invalid").
  - If that happens, the rollback brings up nothing and the script ends with "rollback did not come up healthy". The gateway is then down until someone intervenes.
  - The build path is not affected: cargo removes the target file before relinking, so the new build gets a fresh inode.
- Evidence: deploy.sh:97 `cp -p "$backup" "$BIN"` followed by :100 `restart`. The backup copy at :72 is also `cp -p`, but it writes to a new file, so it is safe.
- Not verified: I did not reproduce the kernel kill on this machine. It is documented platform behaviour.
- Fix: Replace atomically: `cp -p "$backup" "$BIN.rollback" && mv -f "$BIN.rollback" "$BIN"`, or `rm -f "$BIN"` before the copy.

### [P2] The watchdog restarts on the first failed health probe, with no allowance for startup
- Location: scripts/gateway_watchdog.sh:81-88
- Trigger: The 60-second watchdog tick fires while the gateway is still booting, or a single `/api/health` call takes more than 5s.
  - During boot, `pgrep` already matches, but the dashboard listener only starts at src/main.rs:1334.
  - Boot first runs `OmoDaemonSupervisor::ensure` (main.rs:1003). That can take up to 30s waiting for readiness (`READY_WAIT_AFTER_SPAWN`, omo_daemon.rs:37), plus up to about 20s reclaiming a leftover daemon.
  - The else branch calls `restart_gateway` (`kickstart -k`) on every failure. `FAIL_LIMIT` only controls when the alert is sent.
- Impact: A booting gateway, including one just started by `deploy.sh` or by the watchdog itself, gets killed and restarted. Restarts cascade whenever boot takes longer than the gap to the next tick.
- Evidence: Line 81 is `if pgrep -f "$GATEWAY_PATTERN" ... && http_ok "$DASHBOARD_URL"; then`. The else branch increments `gateway_fail`, then runs `restart_gateway` (line 87) before the `fails >= FAIL_LIMIT` check.
- Also unverified: the default pattern `/target/release/omo-gateway run` requires a literal `run` argument. If the launchd plist starts the binary with no subcommand (valid, since it defaults to `Run`), the watchdog would restart the gateway every minute. The plist is not in the repo, so I could not confirm this.
- Fix: Restart only when `fails >= FAIL_LIMIT`, or skip the check while the process is younger than a startup grace period (for example, check `ps -o etimes=`).

Checked and found consistent:
- **Dashboard payloads:** the compiled dashboard (`web/src/App.tsx`, `web/src/api.ts`, `web/src/pages/BotsPage.tsx`, `web/src/lib/bot-payload.ts`) matches the `src/dashboard.rs` handlers for sessions, chat, cron, bots, approvals and logs.
- **Hermes files:** `web/src/lib/api.ts` and the hermes-style components are outside `tsconfig.app.json` and are never imported from `main.tsx`, so they are not built.
- **Rendering:** ReactMarkdown 9 renders without raw HTML, and no session token is injected into the client.
- **Migration 0042:** its triggers order correctly against the FK cascade.
- **Scripts already fixed:** the cutover, rollback and seed scripts now carry the fixes from the earlier rounds.
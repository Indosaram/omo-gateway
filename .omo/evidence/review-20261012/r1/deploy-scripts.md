# deploy-scripts review

### [P0] Docker build fails: [[test]] targets declared in Cargo.toml but `tests/` is never copied into the builder
- Location: Dockerfile:5 (`COPY src ./src`; the `COPY` lines at 4-7 omit `tests/`; consumer is Cargo.toml:64-66 and the 14 later `[[test]]` blocks)
- Trigger: `docker compose up -d` (README step 2) runs `docker build`. The builder stage copies only `Cargo.toml Cargo.lock src migrations README.md LICENSE`, then runs `cargo build --locked --release --bin omo-gateway` (Dockerfile:8). Cargo.toml declares 15 `[[test]]` entries with explicit paths (`tests/test_voice_transcription.rs`, and so on). Cargo resolves every declared target path when it loads the manifest, whichever `--bin` is selected. It aborts with "can't find `test_voice_transcription` test at `tests/test_voice_transcription.rs`".
- Impact: the image can never be built, so the documented Docker deploy is dead on arrival.
- Fix: add `COPY tests ./tests` before the build (this keeps the manifest valid), or remove the explicit `[[test]]` blocks from Cargo.toml, which are redundant with autodiscovery.

### [P0] Docker build fails: builder toolchain rust:1.85 is older than the locked dependencies' MSRV
- Location: Dockerfile:1 (`FROM rust:1.85-bookworm AS builder`)
- Trigger: Cargo.lock pins `time 0.3.55` (Cargo.lock:3812-3813). It is a non-optional dependency of serenity 0.12.5, and the registry metadata says it requires `rust-version = "1.88.0"` (confirmed in the local registry copy of its Cargo.toml and on crates.io). `home 0.5.12` also declares 1.88. `cargo build --locked` on rustc 1.85 refuses to build a dependency whose `rust-version` exceeds the compiler ("rustc 1.85.0 is not supported by the following packages: time@0.3.55 requires rustc 1.88.0").
- Impact: the Docker build fails even after the `tests/` issue is fixed.
- Fix: bump the builder image to `rust:1.88-bookworm` or newer, and set `rust-version` in Cargo.toml so the requirement is explicit.

### [P0] Docker build fails: builder image has no cmake, and no libopus-dev or pkg-config
- Location: Dockerfile:13 (the runtime stage installs only `ca-certificates libopus0`; the builder stage at line 1 installs nothing)
- Trigger: songbird 0.5.0 with the `driver` feature pulls `audiopus 0.3.0-rc.0`, then `audiopus_sys 0.2.2` (Cargo.lock:162-179; its dependencies are `cmake`, `log` and `pkg-config`). Its build.rs first probes `pkg-config` for `opus`. Without libopus-dev that fails, so it falls back to `cmake::build("opus")` (build.rs `build_opus`). The package list in the upstream `buildpack-deps:bookworm` image, which `rust:*-bookworm` is based on, has no `cmake`, `pkg-config` or `libopus-dev`. I verified this from the upstream Dockerfile text, not by building the image.
- Impact: the build fails in the `audiopus_sys` build script. The runtime stage's `libopus0` shows the intended path was dynamic linking against a system libopus, which cannot work without the build-time packages.
- Fix: in the builder stage, run `apt-get install -y --no-install-recommends cmake pkg-config libopus-dev` (libopus-dev lets the build link the same libopus that the runtime installs).

### [P0] Compose bind mount `./data` is root-owned on Linux, so the non-root container cannot open its SQLite DB
- Location: docker-compose.yml:27 (`- ./data:/app/data`; same issue at line 28 for `./workspace`)
- Trigger: on a Linux host with a fresh clone, `./data` does not exist (`ls` of the repo shows no `data/`). `docker compose up -d` creates the missing bind-mount source as `root:root`. The image runs as `USER omon` (uid 10001, Dockerfile:27). The `chown -R omon:omon /app` at Dockerfile:18 is shadowed by the bind mount. Startup opens `sqlite:///app/data/omon_gateway.db` with `create_if_missing` in WAL mode (`Database::connect`, src/storage/db.rs:158-170).
- Impact: SQLite cannot create or open the file ("unable to open database file"), the process exits, and `restart: unless-stopped` turns that into a crash loop. The same applies to the `./workspace` writes.
- Fix: use named volumes (they inherit the image's `omon` ownership), or document and script `mkdir -p data workspace && chown 10001:10001 data workspace`, or set `user:` to the host uid.

### [P1] Phase 1 cutover enables 14 cron jobs that can never fire, after legacy cron has already been disabled
- Location: scripts/seed_omomeow_crons.sql:1 (`INSERT OR REPLACE INTO cron_jobs (id, session_key, expression, payload_json, enabled)` omits `next_run_at`); scripts/cutover_phase1_cron.sh:71 (`UPDATE cron_jobs SET enabled = 1 ...` does not set `next_run_at`)
- Trigger: seed the DB, then run `cutover_phase1_cron.sh`. Seeded rows have `next_run_at = NULL`. The scheduler selects only `enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?` (src/cron/scheduler.rs:1126, 1316). It skips NULL rows in its missed-run scan (scheduler.rs:728-730). No code path fills `next_run_at` for an existing enabled row; only register/resume/tools paths compute it. Even if a row were scheduled, the payloads (seed lines 2-15) have no `schedule`, `prompt`, `script` or `brief_file`. They therefore take `execute_native_cron`, which returns `Ok(None)` at src/cron/executor.rs:509-511 and does nothing. They also set `deliver:"origin"` with no `origin` object.
- Impact: after Phase 1, the legacy launchd and crontab jobs are disabled (script lines 37-56) while the 14 replacement jobs never run. The script then prints "Verified: Successfully enabled exactly 14 OmOMeow cron jobs" because it only counts `enabled = 1`. The calendar, mail and monitoring jobs go silent with no error.
- Fix: seed real payloads (prompt, script or brief_file) and set `next_run_at` to the next occurrence, for example by registering the jobs through the gateway's scheduler registration path instead of raw SQL. At minimum, make the cutover script fail unless every `omomeow:%` row has `next_run_at IS NOT NULL` and a non-empty prompt or script.

### [P1] Phase 1 cutover disables legacy cron before validating the target DB, and a wrong or missing result still ends "Completed Successfully"
- Location: scripts/cutover_phase1_cron.sh:66 (`if [ ! -f "${DB_PATH}" ]` runs only after Step 1 has disabled the legacy jobs); scripts/cutover_phase1_cron.sh:74 (`if [ "${ACTIVE_COUNT}" -ne 14 ]` only warns); scripts/cutover_phase1_cron.sh:8 (default `DB_PATH=${REPO_ROOT}/data/gateway.db`)
- Trigger: run the script in a default deployment. The gateway's actual default DB is `omon_gateway.db` (`DATABASE_URL` default at src/entry.rs:139), or `/app/data/omon_gateway.db` in Docker. The script's default path therefore usually does not exist. Step 1 (lines 37-56) has by then already booted out the launchd job, renamed the plist, and commented out the crontab entries. Step 2 then exits 1 at line 68. If the DB exists but has 0 matching rows, the `UPDATE` at line 71 silently matches nothing, line 74 only prints a WARNING, and the script exits 0 with "Completed Successfully".
- Impact: the legacy jobs are off and nothing replaces them, with no automatic rollback.
- Fix: do the DB existence check and a row-count check (`omomeow:%` count equals 14) before Step 1, and make a count mismatch exit non-zero. Default `DB_PATH` to the real DB, derived from `DATABASE_URL`.

### [P1] Phase 3 inbound cutover and rollback destroy bot tokens and can silently fail to inject the production token
- Location: scripts/cutover_phase3_inbound.sh:85 (`sed -i.bak "s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=${EXTRACTED_TOKEN}|" ... || true` followed by `rm -f "${ENV_FILE}.bak"`); scripts/rollback_phase3_inbound.sh:45 (`sed -i.bak "s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=|"`)
- Trigger: (a) The `sed` only rewrites an existing `DISCORD_BOT_TOKENS=` line. If the operator's `.env` has no such line, nothing changes, yet line 87 prints "Injected production token". Legacy omomeow was already killed in Step 1 (lines 47-66), and the gateway is restarted without the token. (b) If `.env` already holds a comma-separated list in `DISCORD_BOT_TOKENS` (the documented multi-bot form, which `main.rs` splits on commas at lines 136-143), it is overwritten with the single omomeow token and the `.bak` is deleted immediately. (c) Rollback blanks the line with no way to restore the previous value. If the operator kept their tokens only in `DISCORD_BOT_TOKENS`, the gateway restarts with zero tokens and fails with `missing required environment variable DISCORD_BOT_TOKEN` (src/main.rs:144-147).
- Impact: other bots' tokens are lost, inbound traffic can go dark, and rollback cannot restore the pre-cutover config.
- Fix: keep a timestamped backup of the env file and restore it on rollback. Append the token if the key is absent. Abort if the post-`sed` grep does not show the new token. Merge into the existing comma list rather than replacing it.

### [P1] Phase 3 "restart" fallback only kills processes and never starts one, and its patterns are wrong
- Location: scripts/cutover_phase3_inbound.sh:104 (`pkill -f "omon-gateway"`); scripts/rollback_phase3_inbound.sh:53 (same)
- Trigger: the launchd branch requires `~/Library/LaunchAgents/com.omon.gateway.plist` (cutover script line 13). The README's LaunchAgent instructions use `ai.omon.gateway.plist` (README.md:201-202), and no plist exists in the repo. So the `else` branch (the pkill fallback) runs. The shipped binary is named `omo-gateway` (Cargo.toml `[[bin]] name`), not `omon-gateway`. `pkill -f` matches the full command line, so it hits any process whose path contains the repo directory name `omon-gateway`, for example an absolute-path invocation of this very script, cargo, or lane agents under that directory. It does not match `/usr/local/bin/omo-gateway`. In the repo-built-binary case it kills the running gateway but nothing restarts it.
- Impact: after the legacy daemon is killed and the token is swapped, the gateway is either left stopped, or never picked up the new env. It also kills unrelated processes, and possibly the script's own shell, which would skip the final "Completed" step. The script reports success regardless.
- Fix: use a label that matches the README plist (or document `com.omon.gateway`), match the real binary with `pkill -x omo-gateway`, and actually start the service after the kill. Fail loudly if neither a launchd service nor a supervisor is found.

### [P1] Docker runtime image has no fonts, so markdown tables render as blank images and the table text is dropped from the message
- Location: Dockerfile:13 (`apt-get install -y --no-install-recommends ca-certificates libopus0`)
- Trigger: any final agent reply containing a markdown table. `adapter.rs:3096` calls `transform_markdown_tables_to_images`, which rasterizes via `svg_to_png` using `usvg` with `opt.fontdb_mut().load_system_fonts()` (src/discord/table_render/mod.rs:304). `debian:bookworm-slim` ships no fonts, so usvg finds no font for the `<text>` elements and skips them. Rendering still "succeeds" and returns a PNG of empty cells.
- Impact: on success the code replaces the table with the placeholder "see attached table" (mod.rs:333-345) and uploads that PNG, so Discord users receive a table with no text. The original table content is lost from the message.
- Fix: install a font package in the runtime stage, for example `fonts-noto-cjk fonts-dejavu-core` (the SVG font stack names Noto Sans KR).

### [P1] Phase 2 shadow verification can pass when its checks fail
- Location: scripts/verify_phase2_shadow.sh:83 (`cargo test --test test_discord_adapter ... || true` followed by `pass_check`); scripts/verify_phase2_shadow.sh:55 (`grep -q "DISCORD_BOT_TOKENS"`)
- Trigger: check 3 swallows the test exit code unconditionally, so a failing adapter test still prints `[PASS] Message response dispatch contract verified`. Check 1 passes if the string `DISCORD_BOT_TOKENS` appears anywhere in the env file, including in a comment or as an empty `DISCORD_BOT_TOKENS=`. Neither the cargo tests nor the file reads are anchored to the repo, because the script never `cd`s to `REPO_ROOT`.
- Impact: this script is the gate before the Phase 3 cutover. It can report 6/6 passed with a broken adapter or an empty token, which authorizes a cutover that takes production down.
- Fix: drop `|| true` on check 3, check for a non-empty value with `grep -Eq '^DISCORD_BOT_TOKENS?=.+'`, and `cd "${REPO_ROOT}"` before the `cargo test` calls.

### [P2] `omomeow-delegate` overwrites an existing lane's brief file on a lane-key collision
- Location: bin/omomeow-delegate:105 (`printf '%s' "$BRIEF" > "$BRIEF_PATH"`; key built at lines 101-103 with only a 16-bit random suffix `od -An -N2`)
- Trigger: delegating a second task whose title slugs identically to an existing lane and whose 4-hex suffix collides (1 in 65536 per same-title pair). The script writes the brief before the gateway checks for an existing lane (`lane key already exists` in src/lanes/cli.rs's `start`, via `get_lane`).
- Impact: the gateway rejects the start, but the live lane's `brief.md` has already been replaced. The lane runner and settlement later read `lane.brief_path` (src/lanes/runner.rs:1230, src/lanes/settle.rs:181), so they use the wrong brief.
- Fix: create the brief with `set -o noclobber` or `mktemp -d`, or use at least 8 random bytes in the suffix and fail if the directory already exists.

### [P2] Re-running the seed SQL resets already-cut-over jobs
- Location: scripts/seed_omomeow_crons.sql:1 (`INSERT OR REPLACE INTO cron_jobs`)
- Trigger: running the seed again after Phase 1 (the file is a plain SQL script with no guard).
- Impact: `REPLACE` deletes and reinserts each row, so `enabled` returns to 0 and the payload state (`last_status`, counters) and `created_at` are reset. With `PRAGMA foreign_keys` enabled on the connection, the `cron_runs` history rows cascade-delete too (migrations/0004_cron_leases.sql:11).
- Fix: use `INSERT ... ON CONFLICT(id) DO NOTHING`, or an `UPDATE` that only touches the schedule fields.

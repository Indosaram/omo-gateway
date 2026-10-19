# fix-deploy evidence

Scope files changed: `Dockerfile`, `docker-compose.yml`, `Cargo.toml` ([package] `rust-version = "1.88"` only), `README.md` (Docker section, one paragraph).
Test file: `tests/test_fix_fix_deploy.rs` (5 deterministic file-content tests, no sleeps; they parse the real Dockerfile/compose/Cargo.toml).

Docker: `docker: command not found` on this machine, so no `docker build` was run. Verification is static plus the focused test file.

## RED / GREEN commands

RED (original files from `git show HEAD:<file>` for Dockerfile, docker-compose.yml, Cargo.toml, README.md; test compiled standalone with `CARGO_MANIFEST_DIR` pointing at a temp dir with the HEAD files):

    rustc --edition 2021 --test tests/test_fix_fix_deploy.rs -o $T/t && CARGO_MANIFEST_DIR=$T $T/t

    test builder_installs_native_build_deps_for_audiopus_sys ... FAILED
    test compose_uses_named_volumes_not_root_owned_bind_mounts ... FAILED
    test runtime_installs_fonts_for_table_rendering ... FAILED
    test builder_copies_every_top_level_dir_the_manifest_paths_need ... FAILED
    test builder_rust_image_satisfies_package_rust_version_and_locked_floor ... FAILED
    bind mount would be root-owned on Linux: - ./data:/app/data
    runtime must install fonts-dejavu-core
    builder stage must COPY tests ./tests
    [package] rust-version missing
    test result: FAILED. 0 passed; 5 failed

GREEN (current files):

    cargo test --test test_fix_fix_deploy
    running 5 tests
    test compose_uses_named_volumes_not_root_owned_bind_mounts ... ok
    test builder_installs_native_build_deps_for_audiopus_sys ... ok
    test runtime_installs_fonts_for_table_rendering ... ok
    test builder_rust_image_satisfies_package_rust_version_and_locked_floor ... ok
    test builder_copies_every_top_level_dir_the_manifest_paths_need ... ok
    test result: ok. 5 passed; 0 failed

(A first `cargo test --test test_fix_fix_deploy` run executed after the edits had landed, so it was already green; the RED above is the authoritative one against the unfixed files.)

## Per defect

### 1. [P0] builder does not COPY tests/ - FIXED (with a caveat on severity)
- Change: `Dockerfile` builder stage, added `COPY tests ./tests`. `[[test]]` blocks untouched.
- Test: `builder_copies_every_top_level_dir_the_manifest_paths_need` (derives every top-level dir from `path = "..."` in Cargo.toml, asserts builder COPYs each).
- RED excerpt: `builder stage must COPY tests ./tests`; GREEN: ok.
- Caveat from static probing in a temp copy containing only the old Dockerfile COPY set (Cargo.toml, Cargo.lock, README.md, LICENSE, src, migrations), temp dir deleted afterwards:
  - `cargo metadata --locked --offline --format-version 1` exits 0 WITHOUT tests/ on cargo 1.92, 1.87, 1.88, 1.98.1 (so the "manifest fails to load" claim does not reproduce for metadata; no RED->GREEN on metadata is possible).
  - `cargo build --locked --release --bin omo-gateway --unit-graph` (nightly) succeeds without tests/.
  - `cargo build --tests --unit-graph` fails without tests/: `error: can't find integration-test 'test_cron_health' at path '.../tests/test_cron_health.rs'` (and the other declared targets).
  So a `--bin`-only release build is probably not broken by the missing dir on current cargo, but any `cargo test`/`--tests`/`--all-targets` step in the image would be. The COPY is cheap, matches the lead's instruction, and is kept. I could not confirm it with a real `docker build`.

### 2. [P0] rust:1.85 vs locked crates needing 1.88 - FIXED
- Change: `Dockerfile` line 1 `rust:1.85-bookworm` -> `rust:1.92-bookworm`; `Cargo.toml` [package] `rust-version = "1.88"`.
- Locked max rust-version (from `cargo metadata --locked`): `time 0.3.55` 1.88.0, `time-core 0.1.9` 1.88.0, `time-macros 0.2.32` 1.88.0, `icu_provider 2.3.0` / `icu_properties 2.3.0` / `icu_properties_data 2.3.0` / `icu_normalizer_data 2.3.0` 1.88. Max = 1.88, so 1.92 >= 1.88; 1.92 also equals the local `rustc 1.92.0` floor.
- Test: `builder_rust_image_satisfies_package_rust_version_and_locked_floor` (declared >= 1.88 and image version >= declared).
- RED excerpt: `[package] rust-version missing`; GREEN: ok.
- Not verified: that the `rust:1.92-bookworm` tag exists on Docker Hub (no network/docker check); 1.92 was published Dec 2025 and the local toolchain is 1.92.0.

### 3. [P0] builder lacks cmake/pkg-config/libopus-dev - FIXED
- Change: `Dockerfile` builder stage: `apt-get install -y --no-install-recommends cmake pkg-config libopus-dev` + apt list cleanup, before `WORKDIR /build`.
- Test: `builder_installs_native_build_deps_for_audiopus_sys`.
- RED excerpt: panic at the missing `apt-get install`/package assertion (`tests/test_fix_fix_deploy.rs:92`); GREEN: ok.
- Not verified: actual audiopus_sys compile inside the image (no docker).

### 4. [P1] bind mounts root-owned vs uid 10001 - FIXED
- Change: `docker-compose.yml` volumes `./data:/app/data` / `./workspace:/app/workspace` -> named volumes `omon-data:/app/data`, `omon-workspace:/app/workspace`, plus top-level `volumes:` declaring both. Docker initializes a new named volume from the image directory (`/app/data`, `/app/workspace` are chowned to omon in the image), so uid 10001 can write; persistence is kept across restarts. README Docker section gets one paragraph describing this.
- Test: `compose_uses_named_volumes_not_root_owned_bind_mounts`.
- RED excerpt: `bind mount would be root-owned on Linux: - ./data:/app/data`; GREEN: ok.
- Not verified: live `docker compose up` write to SQLite. Note for operators: existing `./data` on disk is no longer mounted; migrate with a one-off copy into the `omon-data` volume if needed.

### 5. [P1] runtime stage has no fonts - FIXED
- Change: `Dockerfile` runtime apt install adds `fonts-dejavu-core fonts-noto-cjk`. src/discord/table_render/mod.rs:118 uses a font-family list (`-apple-system, ..., Roboto, 'Apple SD Gothic Neo', 'Noto Sans KR', sans-serif`), so Noto CJK (covers Korean) plus DejaVu as the sans-serif fallback is appropriate.
- Test: `runtime_installs_fonts_for_table_rendering`.
- RED excerpt: `runtime must install fonts-dejavu-core`; GREEN: ok.
- Not verified: actual PNG rendering in the container.

## Cleanup
All mktemp dirs and spawned cargo probes were removed/killed; no files outside the write scope were edited; no git operations performed.

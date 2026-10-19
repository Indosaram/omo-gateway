# Attempt 4 build provenance

Working checkout: `/Users/indo/code/project/omon-gateway-wt/dori-work`
Git HEAD: `245a5876138acdd1f4a2c31416121f39e95bf416`
Cargo target directory: `target-w4` (8.6 GB observed before run)
Cargo build jobs: `2`
Cargo target count: `29` (`cargo metadata --no-deps --format-version 1`)

Source manifest command, before and after the run:

```sh
find src tests migrations -type f -print0 | sort -z | xargs -0 shasum -a 256
```

The exact 145-entry manifests are saved in `wave1-integrated-attempt4-pre-manifest.sha256` and `wave1-integrated-attempt4-post-manifest.sha256`. Both contain the same sorted paths and all 145 per-file hashes compare byte-for-byte identical. This establishes no source, test, or migration file drift within those trees during Attempt 4.

Additional build inputs and toolchain:

- `Cargo.toml`: SHA-256 `17486e1195bc5a1b534e02845a58e7cad5e292abd9e784e18b3572d5f3d3d16e`
- `Cargo.lock`: SHA-256 `6129c4128398f0bbe50d9b2b69d725648fae5daa76a2915db2d6784fc3cfcba3`
- `build.rs`: absent
- `.cargo/config.toml`: absent
- `rustc 1.92.0 (ded5c06cf 2025-12-08)`, host `aarch64-apple-darwin`
- `cargo 1.92.0 (344c4567c 2025-10-21)`
- The invoking shell did not export `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS`, or `RUSTFLAGS`; the run command explicitly exported target-w4 and jobs=2.
- Backend test executable `target-w4/debug/deps/test_omo_backend-a074f7c420765738`: SHA-256 `ab18769df5a57c36b4533677b184b75d802ef1103d793c843606c78ec6a437ae`.
- A later global `pgrep -x cargo` found PID 79932, but `ps` and `lsof` identify it as a separate FerryX test command with cwd `/Volumes/T9-Mac/ferryx-release-builds/qa-06862328`; it is not an Attempt 4 process and was not touched.

Final tracked source changes relative to HEAD remain the pre-existing eight-file Wave 1 set: `Cargo.toml`, `src/agent/mod.rs`, `src/agent/omo_backend.rs`, `src/agent/omo_config.rs`, `src/lib.rs`, `src/multiplexer/actor.rs`, `src/storage/mod.rs`, and `tests/test_omo_backend.rs`. The verification made no product changes. Existing untracked source, test, migration, evidence, and target directories were retained.

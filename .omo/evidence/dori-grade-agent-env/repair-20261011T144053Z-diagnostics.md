# Strict gate diagnostics

`cargo fmt --check` exit 0.
`cargo clippy -j2 --all-targets --all-features -- -D warnings` exit 101.
Complete raw diagnostics: `repair-20261011T144053Z-clippy.log`.

Two errors were emitted, both `clippy::await_holding_lock`:

- `tests/test_lane_reopen.rs:121:9`: `backend.seen.lock().unwrap()` guard remains in scope through awaits at lines 130 and 137.
- `tests/test_lane_reopen.rs:242:9`: same guard remains in scope through awaits at lines 246 and 249.

The emitted build-failed warning and final compile error are preserved in the raw log. No diagnostics suppressed. This file is outside authorized repair paths and unchanged relative to canonical manifest SHA-256 `623e1ade66f7c91164d44d4ef2f9d0e2ab0a73c8a8d118c67260ce3108897b0a` in the settled pre-manifest. These errors were newly exposed after the inbox warnings were repaired, not newly introduced source changes. No broader fix or repeated gate run attempted.

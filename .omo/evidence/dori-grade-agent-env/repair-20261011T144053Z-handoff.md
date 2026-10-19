# Minimal repair results

Batch completed, PID 28848 exited, no Cargo process observed at final process check. Source pre/post manifests are byte-identical. Original canonical batch135508Z evidence pre/post hash manifests compare equal (exit 0). Relative evidence paths below are under `.omo/evidence/dori-grade-agent-env` in dori-work.

Exact exits, each command run once: scoped rustfmt 0; isolated shell regression 0; cargo fmt --check 0; strict all-target/all-feature clippy 101; focused state library test 0; focused inbox integration target 0. Harness exit 0 means evidence collection completed, not that all gates passed; `repair-20261011T144053Z-status.tsv` is authoritative for individual exits.

Strict gate remains failed on two newly exposed `await_holding_lock` diagnostics at `tests/test_lane_reopen.rs:121` and `:242`. File unchanged and outside authorized scope. Full raw diagnostic log preserved; no suppression or repair attempted. See `repair-20261011T144053Z-diagnostics.md`.

State: 11 passed, 0 failed, 0 ignored. Raw `task-7.log` SHA-256 `509e65ffdb3cb77b8cbdd95f31cd4b24c3c67f2484046d63e796e3ee1d82ef6b`. Adversarial assertion mapping and limits: `repair-20261011T144053Z-task7-adversarial.md`. Scratch SQLite cleanup assertion passed. Historical malformed-row coverage claim is unsupported: the named test actually checks a missing lane only.

Inbox: 13 passed, 0 failed, 4 live tests ignored; no live ignored tests executed. Raw `repair-20261011T144053Z-inbox.log` SHA-256 `aadbac1bc6f1c34ab9a41609496b8b4f8898e2c87a740168f4e8a37e171bfd9f`.

Shell: exact integrated settled test ran once under isolated environment, exit 0. Scratch path absence verified, wrapper unchanged. `task-11.log` updated with integrated provenance; raw `repair-20261011T144053Z-shell.log` SHA-256 `f0d714a88d7ec3e9ebbd0691735e03f0ee4ef343e3586cfc747ac49d254ce99a`.

Only three source paths differ from canonical source manifest:
- `src/agent/backend.rs`: final blank-line removal, SHA-256 `9c31fd7a7a16585fd1398797609c020e40eb75a16dd38b1c7dd949732725d9bd`.
- `tests/test_inbox_sources.rs`: two direct unit-struct constructors, SHA-256 `2aef0dd17a3182734bb6ba7cc88c40ad129d31d8cf95f92847a3c33037995a48`.
- `tests/test_omomeow_delegate.sh`: owned-cwd insertion, SHA-256 `c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea`.

No full-suite replay: preserved canonical full batch reports 934 passed, 15 repeated baseline failures, 16 ignored, zero new baseline failures. Those baseline failures remain outside this repair. No product behavior change, dori-next edit, git/DB/service/Discord action, or plan/ledger/Boulder update. Bash LSP unavailable; shell syntax harness check and actual regression executed successfully. Stop reason: evidence-backed focused results and concrete remaining strict-gate failures.

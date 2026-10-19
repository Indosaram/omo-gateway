# QA2 loopback source settlement

Date: 2026-10-12. Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.

## Outcome and remaining ownership boundary

The parent change in `src/lanes/cli.rs:371` is correct: `DiscordThreads::from_env()` supplies the HTTP client for both `GET /users/@me` and thread creation. An actual tokenless CLI invocation now passes against an owned, non-forwarding loopback peer and persists the created thread and bot binding in SQLite.

The requested direct start, post, status, close, archive, reopen and unarchive paths honor the loopback transport. One additional queued-lane promotion path remains outside the explicitly assigned source ownership: `src/lanes/guard.rs:629-632` still calls `first_bot_token()` and constructs `Http::new`. No claim is made that every lane creation path is tokenless. The guard owner must replace those two constructions with `DiscordThreads::from_env()` and pass `threads.http()` to `create_thread`; this report does not change guard state or binding semantics.

## Review and source audit

- `cli.rs`: start uses the parent's environment factory. Close and reopen enqueue durable intents/replies and wake the runner; neither constructs an HTTP client.
- `discord_threads.rs`: `from_env()` selects the opted-in loopback sink before reading credentials. With no `OMON_DISCORD_API_BASE`, it still uses `first_bot_token()` and `Self::new(token)` / `Http::new(token)`. The helper and production request builders were not modified in this task.
- `runner.rs:58`: direct reopen uses `DiscordThreads::from_env()`. Service construction at lines 912-923 selects `loopback_from_env()` even when Discord is disabled, otherwise uses `from_env()` when enabled. The same transport backs post delivery and lifecycle effects.
- `runner.rs:133-208`: lifecycle bot identity, status, archive and unarchive use that transport's HTTP client; no independent credential read occurs.
- `posts.rs`: `PostSink for DiscordThreads` uses `self.http()` for posts and status edits and the same helper for renames. There is no independently constructed client to replace.
- `guard.rs:629-632`: unresolved queued promotion constructor described above, inspected read-only.

No further edits were needed in the owned production files. The no-loopback factory branch and all production payload builders remain byte-for-byte untouched by this task. No live Discord behavior is claimed as tested.

## Changes

`qa2-common.mjs` recorder now returns its existing bot user object for exact `GET /api/v10/users/@me` rather than a 404. An owned recorder-only probe returned HTTP 200, bot ID `1465631383862120451` and `bot: true`; the recorder was stopped afterward. No gateway or app-server service was launched.

`tests/test_lane_thread_transport.rs` adds `lane_start_creates_and_binds_thread_when_only_loopback_is_configured`. It invokes the actual `omo-gateway lane start --thread 200 --model small` binary in an isolated temporary HOME/database with a cleared environment and only the owned Discord loopback origin, so neither `DISCORD_BOT_TOKEN` nor `DISCORD_BOT_TOKENS` is present. Host guard thresholds are explicitly permissive to avoid machine-load timing dependence. It asserts exit 0, the three expected HTTP requests, thread ID 300 and an authoritative SQLite join showing working status, parent 200, thread 300 and bot binding 99. The child has a bounded timeout and kill-on-drop; the listener task is aborted and joined before assertions, and a final connection verifies listener closure.

## Verification

Executed once in the warm shared target, without deleting locks or creating a separate target:

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_lane_close --test test_lane_status_transport --test test_lane_thread_transport
```

Exit code: 0. Compilation finished in 14.68 seconds.

- `test_lane_close`: 27 passed, 0 failed, 2.69 seconds.
- `test_lane_status_transport`: 1 passed, 0 failed, 0.10 seconds.
- `test_lane_thread_transport`: 2 passed, 0 failed, 1.70 seconds, including the new tokenless CLI test.

`rustfmt --edition 2021 --check tests/test_lane_thread_transport.rs` exited 0 after fixing the initial new-code formatting differences. Rust LSP initially returned no diagnostics but subsequent freshness requests timed out after 3000 ms; the actual Cargo compilation and tests succeeded. JavaScript LSP initialization was unavailable because the workspace has no valid TypeScript installation; the recorder was instead executed and its real HTTP response checked.

Full task15/task18 model-driven QA2 scenarios were not rerun, and their earlier FAIL reports are preserved. No environment files, excluded source files, git commands, service management or live Discord operations were performed. Existing close tests themselves use temporary git fixtures as part of the explicitly requested test command.

## Relevant source SHA-256

```text
42c40da8013510ac5979a32e43664907493055707636fa336ca117accd097b8b  src/lanes/cli.rs
1a6ea344b09af40cb5eed08ce2ffe5c778a77d9585b643ea55d557340f150c47  src/lanes/discord_threads.rs
e2562ae76e241c5d30096cbb834cf8a28c1ca8073df81bf08a47a94d9567e77c  src/lanes/posts.rs
62d45ee97091ceda83eba0cc84505b2fc4a22787f5c949c1a7f5467f8c8cda11  .omo/evidence/dori-grade-agent-env/qa2-common.mjs
8b870c6ad676ba3885736904a9ed277d2667e5e31d15ab593c83cd6b3c3054c5  tests/test_lane_thread_transport.rs
```

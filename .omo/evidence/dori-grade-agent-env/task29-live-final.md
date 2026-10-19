# Task 29 live QA: PASS, with retained findings

## Owner direct reply: PASS

Real HTTP chat surface: POST /api/sessions/qa29-owner-a31ad4dd-c106-4388-b577-0e47b778067d/chat on an owned loopback dashboard, port 61423, backed by scratch DB /private/tmp/omon-task29-live-final-iErvve/qa.db and owned authenticated app-server port 61424. OMON_DISCORD_ENABLED=false. The headless gateway and dashboard used the same scratch DB; dashboard started after headless migrations to avoid initialization contention.

Prompt sent 2026-10-12T07:20:54.401Z; HTTP direct reply received 2026-10-12T07:21:00.131Z: 5729.895 ms. SQLite user timestamp 07:20:54.404282Z and assistant timestamp 07:21:00.130277Z agree. Reply was `local\n- live 19, resumable 7`, matching a real release-binary `sessions list --json` result with errors=[] and identical counts. The implementation calls SessionRegistry::list_all directly; it does not spawn a model to issue a CLI command.

No backend model turn: HTTP 200 has direct_reply=true and queued=false; assistant metadata has direct_reply=true; exactly one delivered obligation; session state remains {}; zero omo_thread_id bindings; owner dashboard logs contain no turn execution. Source src/dashboard.rs:1221-1231 and src/multiplexer/router.rs:386-456 confirms this branch dispatches directly before actor/backend routing.

## Shared channel fallthrough: PASS

Same prompt submitted through real HTTP chat using a scratch Discord guild channel key. HTTP 202, queued=true, no direct_reply field; received in 4.344 ms. SQLite contains the user prompt under the normal canonical shared key with empty user component, no assistant session summary and zero delivery obligations. Earlier scratch attempt /private/tmp/omon-task29-live-final-oYAQ6C also persisted an omo_thread_id binding, proving normal backend routing; its observer had incorrectly queried the pre-normalization key. No Discord API was called and no model reply was required.

Final shared receipt: /private/tmp/omon-task29-live-final-xFeXJV/task29-live-final.json. Raw aggregate evidence: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-29.log. Owner receipt and authoritative DB: /private/tmp/omon-task29-live-final-iErvve/task29-live-final.json and qa.db. Receipts remain labeled with their original harness status rather than rewritten green; this report derives the scenario verdict from HTTP and authoritative DB evidence.

## Identity, cleanup and findings

SHA-256 before and after: d5d48e4d79d2a8a6239e4cb24ccdefa257537473d4052578b7d301b78a4d9b9b. Only that prebuilt release executable was used. Owned gateway/dashboard PIDs are absent, app-server process groups empty, private auth directories removed, owned app-server ports rebound successfully. All scratch evidence retained. No cargo/build, git, launchctl, Discord, production DB, ws 19742, port 9119, or env-file edits.

Retained findings: simultaneous first-time gateway/dashboard migration hit database is locked; using normal gateway HOME hit an already-owned cron socket (nothing was stopped or taken over); one later real list request timed out after 20 seconds and delivered a direct error reply in 20.112 seconds. These attempts remain in task-29.log. The final shared dashboard shutdown logged `attempted to acquire a connection on a closed pool` while interrupting/flushing its actor; exit code was 0 and owned processes were removed. This prebuilt shutdown problem was not changed. App-server shutdown required owned-group SIGKILL after its bounded grace period. LSP could not initialize because no valid TypeScript installation was available; runtime QA script execution completed with exit 0.

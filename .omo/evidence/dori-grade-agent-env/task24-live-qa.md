# Todo 24 live QA

FAIL: first collection/triage is not reachable in headless pinned gateway. Source runtime src/main.rs:647 returns before spawn_inbox_collector at line731. Released CLI inbox collect exits 2 with error preserved in task-24.log. BLOCKED: second-collect deduplication cannot be established without a real first collection. No fabricated collector or model result.

Read-only sources: mail list exit0 (36 output bytes), imsg chats exit0 (345 output bytes), PirateTalk GET /api/rooms HTTP200. Source content not logged; live Discord/owner gateway DB not read. Scratch collector fallback attempted through binary CLI, unsupported. Class counts are zero/unverified, not a triage pass.

Scratch retained /private/tmp/omon-wave4-task24-3dgqEt; DB /private/tmp/omon-wave4-task24-3dgqEt/qa.db; owned socket /tmp/omon-lanes-7d5f7f51bea0622432f97acf.sock removed. Gateway exit0 PID96153 absent. Descriptors, initial PRAGMA and DB/WAL pre-subscription in log. Binary hashes before/after 71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86. No source writes or new daemon.

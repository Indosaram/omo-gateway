# Todo 25 live verification on production (2026-10-12)

Binary: production /Users/indo/code/project/omon-gateway/target/release/omo-gateway, sha256 cf16cc7a2e75909bde418a16229d1de15f9b71a97acc4e705239b535b13d8692 (Developer ID, 5DUM8WPB4C), commit 1bbe7e2.

- Digest registration: cron_jobs row `omon:inbox-digest`, session_key = the OmOMeow owner DM
  (7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451), expression `0 9 * * *` Asia/Seoul,
  enabled=1, next_run_at 2026-10-13T00:00:00Z (09:00 KST). No "not registered" warning in gateway.log since start.
  Before the startup fix the query would have picked Eris's DM (1529738312833830984); fixed to the exact lead session.
- `inbox list --class owner --json` exit 0 -> task-36/cli-inbox.json (`[]`: triage pending until jev is reachable, see task-36 jev PATH fix).
- Failure scenario: `inbox set nope --status done` exit 2.
- Seeded-DB CLI tests (list filters/ordering, set transitions, empty digest -> None) are part of the final verification test run.

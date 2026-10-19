# Task 28 live QA: PASS

- Happy: PASS. Owned interactive scratch session `4c1485d7-4cc3-4f58-b4f7-8eaef9f97eb5` started with script in `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/qa-task28-fbTDXZ`. The sessions control code runs local omo argv without a scope override, so send/read used that same cwd. Send exited 0; read exited 0 and returned an assistant text item exactly `pong`.
- Missing session: PASS. Random UUID `09d03700-5b51-48d4-b8a0-48ada165c7c4` returned `not running - kept 24 h`, exit 3.
- Raw evidence: `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task-28.log`.
- Cleanup: owned bash_3 process tree killed; process audit found no command containing the owned UUID. Scratch artifacts retained.
- SHA-256 before/after: `d5d48e4d79d2a8a6239e4cb24ccdefa257537473d4052578b7d301b78a4d9b9b` / `d5d48e4d79d2a8a6239e4cb24ccdefa257537473d4052578b7d301b78a4d9b9b`. Only the requested release binary was executed.
- Read receipt also reports `error_note: live_unresponsive` and persisted mode; the authoritative assistant transcript contains pong. This does not hide or relabel that diagnostic.
- No cargo, build, git, launchctl, Discord, production DB, or env-file changes.

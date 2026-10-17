# Slack migration live QA recovery

## Observed during recovery

- Existing `qa-slack-lane-1` remained queued after the monitor expired. It was not recreated.
- `omo-gateway lane can-launch` returned only `disk free (44 GB) is below minimum (50 GB)`.
- No cargo or rustc processes were running. Cleaning this task's Slack worktree dev artifacts with `cargo clean --profile dev` made `can_launch` return true without changing production thresholds.
- The existing lane then created `slack:C0C9Z1Z4G02:1792240697.588879`. Slack replies showed its starter and status message. `/tmp/omon-lane-qa/done.txt` contained `ok`.
- Lane subsequently reached `closed`. Slack API replies confirmed the starter changed to a check mark, its reaction changed to `white_check_mark`, and reply `1792241029.510679` said `lane closed`. Final settlement and Slack close lifecycle PASS.

## External script migration correction

- `omon-content-intel-publish-ready` uses `deliver=local` because its script posts directly. The earlier claim that local delivery meant no user-facing output was incorrect for this job.
- Its stored script now supplies `SLACK_BOT_TOKEN_FILE=/Users/indo/.omon/secrets/slack-bot-token` and `SLACK_NOTIFY_CHANNEL_ID=C0CATRWNX3J` before invoking `publish-ready-digest.ts`.
- Manual run returned `publishing rerun requires owner approval through remediation`. No approval was fabricated and no bypass was attempted. Actual Slack content recommendation delivery remains unverified.

## Digest result

- Run `ed4758e8-a759-49ec-aa22-6950cd0e941b` succeeded with `output:null`; this does not prove delivery.
- The newest persisted digest output was already delivered at `2026-10-17T00:00:05.237679+00:00` (15,824 characters).
- `digest::build` suppresses further outputs on that local calendar day. Slack owner DM history had no new digest. The guard was not bypassed.

## Remaining work

- Restore omitted tracker backend: live `/api/tracker/bootstrap` returns 404 while the frontend contains the tracker. Main has the existing tracker implementation; prepare integration without changing main's uncommitted files.
- Verify Slack-only startup without consuming production Socket Mode events in a competing instance.
- Complete remaining real-surface QA, including approvals with harmless fixtures rather than real pending proposals.
- Gate review remains outstanding.
- Main has other sessions' uncommitted tracker and deployment changes, including overlapping `src/dashboard.rs` and `src/lib.rs`. Do not stash, overwrite or commit them for this migration.
- Main merge and final completion report remain outstanding. Do not describe this work as completely verified.

## User-reported Slack failure, 2026-10-17 12:51 UTC

- User message in C0C9XG5FP8W reached the gateway, but fallback failed with `Agent is already processing`.
- Session JSONL proves primary mahoquot/claude-opus-5-5 returned 503, while a later inferhub continuation produced text after the gateway had already ended. An authenticated direct request to 127.0.0.1:18801 also returned 503 `no available accounts for this model`.
- Global omo changed to 5.1.35 and removed app-server. Forced restart therefore entered a crash loop. The gateway launchd OMON_OMO_BIN now points to ~/.omon/bin/omo-gateway-runtime, which starts the preserved 2026.10.11 runtime. Global omo was not downgraded. Production readyz 200 and Slack hello were verified.
- Preserved runtime TurnEngine now completes RPC prompts after the prompt promise settles instead of on an intermediate agent_end. It also subscribes per session instance. A direct TurnEngine check proved no premature completion and terminal completion after settlement. Live primary-model retries still exceeded the gateway 45-second limit; do not count runtime readiness as response recovery.
- Only the affected home-channel session active_model was set to the already-configured inferhub/cb/deepseek-v4.1-flash fallback after stopping its failed turn. Reload and actual Slack response verification remain pending.
- Tracker backend, web source, and coherent deploy script are now copied into the Slack worktree. No new build or deployment has run. Main files remain untouched.

## Verified deployment, 13:21 UTC

- Format and strict release clippy passed. Release library tests: 936 passed, 7 ignored. Tracker API tests: 6 passed. Frontend build passed.
- Actual release CLI migrated a production DB copy successfully; foreign_key_check was empty and 22 tracker tables existed.
- deploy.sh --no-build signed and installed matching binary and web. Verification passed: gateway 18312, child app-server 18698, readyz 200, health and tracker bootstrap 200. Web is pinned to ~/.omon/web.
- Dashboard replay UUID was incorrectly passed as Slack thread_ts. Egress now accepts only numeric timestamp-shaped reply IDs; regression test passed.
- Slack conversations.history confirmed message 1792243276.414839, text `연결 정상입니다.`. The previously undelivered reply was also recovered as message 1792243243.237759. Actual response recovery PASS.
- No open lanes remain. All enabled gateway cron delivery targets are Slack, origin with Slack, or local script execution. Content recommendation and fresh daily digest delivery remain unverified as noted above.
- Discord offboarding started by overriding DISCORD_BOT_TOKEN, DISCORD_BOT_TOKENS and DISCORD_HOME_CHANNEL to empty in launchd, preserving .env and histories. Slack-only restart verification pending.

## Slack-only confirmation and gate caveat

- Slack-only restart passed deploy verification: gateway 30164, child app-server 30332, readyz and both dashboard probes 200. Boot log has Discord bot_count=0 and `no Discord token; lane threads use Slack only`.
- Slack messages 1792243367.798309 and 1792243371.591469 arrived after offboarding. Transport PASS; their contradictory prose revealed confusion between gateway credentials and standalone messaging CLIs. Workspace AGENTS.md now documents the actual Slack transport and existing token file without exposing its contents.
- Gate reviewer st_01a14a08 recommended APPROVE, but lead does not accept its assertion that all requested verification is complete: live manifest registration, new digest delivery, content recommendation delivery and fresh visual QA remain unverified. Reviewer also wrote evidence into main despite a read-only brief; no source change there is authorized by that action.

## Blocked verification handoff

- Browser connection question expired without an answer. Recheck still reports no connected browser (doctor state no-daemon, Aside extension registered). Do not repeat the question or silently substitute a different browser.
- Tracker bootstrap still returns HTTP 200. This is API evidence, not a visual pass.
- Remaining live QA is stopped as blocked, not completed: Slack app manifest registration and visual QA require the browser connection; fresh digest is suppressed on the same date; content recommendation rerun requires an approval that was not supplied. The successful delivery, lane, migration and deployment evidence above remains valid.
- Commit/main merge was removed from execution because there is no explicit authorization. No additional commit or merge was performed.

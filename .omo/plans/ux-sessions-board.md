# Plan: ux-sessions-board — 트래커를 omo 세션 관제판으로, 사용자 편의 개선

Worktree: /Users/indo/code/project/omon-gateway-wt/ux (branch ux-sessions-board, base main aa49764). Commits: one per track after the single gate. Deploy: scripts/deploy.sh only. No push.
Owner rule: workers write code + tests only and NEVER run cargo/bun/npm/tsc/vite. The lead runs one integration gate at the end.

## Evidence (2026-10-19)
- Tracker shows only gateway lanes (25 QA lanes in team AGT); first screen opens team OMO (0 issues) -> "No issues".
- Real omo sessions: `omo-gateway sessions overview --json` returns 9 local sessions (3 streaming, 6 idle) with cwd, name, last_user/last_assistant, questions. omarchy reachable, `omo thread list --all-scope --json` -> [] (0 sessions).
- Slack: owner reply p50 23s, p90 47min, max 85min; 21x "no output within 45s", 26x turn interrupted.
- Quiet hours refused owner-initiated push at 00:23-00:38 KST; restart expired pending proposal #8.
- Cron deliver slack: worked around with an in-prompt agent-slack step; bot replied to its own cron digest; disk ALERT re-fired 8+ times.

## Tracks (disjoint file ownership)
### A. Sessions board backend (owner: worker-A)
Files: src/tracker/sessions.rs (new), src/tracker/mod.rs (add mod), src/tracker/api.rs (routes only), tests/test_tracker_sessions.rs (new), Cargo.toml ([[test]] only if needed).
- GET /api/tracker/sessions -> collect via crate::sessions::overview::collect(SessionControl::from_environment()) with a 15s in-memory cache; JSON: machines[{machine, error?}], sessions[{machine, session_id, name, cwd, state: working|question|blocked|idle, streaming, message_count, last_user, last_assistant (<=400 chars), questions[], updated_at}], counts {working, question, idle}, collected_at.
- POST /api/tracker/sessions/{machine}/{id}/send {text} -> SessionControl::send (mode auto); /interrupt -> interrupt; /answer {text, request?} -> answer. Validate machine against configured hosts + "local".
- Unit test: JSON shaping from a fixed Overview (no ssh).
### B. Sessions board frontend (owner: worker-B)
Files: web/src/tracker/pages/SessionsPage.tsx (new), web/src/tracker/router.ts (add route 'sessions', make '/' home redirect to sessions), web/src/tracker/shell/Sidebar.tsx (add "Sessions" entry at top with working/question badge), web/src/tracker/api.ts (sessions calls), web/src/tracker/types.ts (types), web/src/tracker/TrackerApp.tsx (route mount).
- Default landing = Sessions. Group by state (Needs answer -> Working -> Idle), each row: machine chip, name or cwd basename, cwd, last activity relative, last user/assistant lines; expand -> full last messages, question options as buttons, send box, Interrupt button. Auto refresh 15s. Machines with errors shown as a banner. Linear visual language, no emoji icons.
### C. Response reliability (owner: worker-C)
Files: src/multiplexer/actor.rs, src/slack/ingress.rs (reaction only).
- On TurnOutcome error: replace raw error text with a Korean owner-facing line ("응답이 끊겨서 다시 시도하고 있어요" on first failure; on final failure "응답을 만들지 못했어요: <short cause>. 다시 보내 주시면 이어서 할게요") and keep the failure reaction. Raw error goes to logs only.
- When an owner message arrives while a turn is running and it is queued (not steered), post one short line "앞 작업이 끝나면 바로 이어서 할게요" (once per running turn).
### D. Approval convenience (owner: worker-D)
Files: src/autonomy/ledger.rs, tests/test_action_ledger.rs (append tests).
- Quiet hours do not defer when the owner sent a message in the same lead session within the last 10 minutes (query messages table role='user' non-system for that session) -> return Ask.
- Restart: pending proposals that already have a platform card stay pending (re-announce via one action_loop event) instead of expiring; only cardless pendings older than 24h expire.
### E. Notification noise (owner: worker-E)
Files: src/cron/scheduler.rs and/or src/cron/executor.rs (slack delivery path), src/lanes/guard.rs, src/slack/ingress.rs is NOT owned (C owns) -> self-message filter goes in src/multiplexer/platform_router.rs or the debounce entry if needed; tests in existing cron/guard test modules.
- Verify cron `deliver: slack:<ch>[:<ts>]` actually posts via the Slack transport; fix if it does not.
- Disk/load/mem guard: re-alert only when value worsens by >=10 GB (disk) since last alert or 24h passed; CLEAR only once.
- Messages authored by our bot or posted by cron into a channel are never ingested as owner turns.
### F. Persona rules (owner: lead)
Files: bot_profiles/blocks/autonomy.md.
- "멈춰/그만" = stop what is running now; never change schedules/config without asking. Say the searched scope when claiming something does not exist. On Slack introduce as the Slack assistant, never as a Discord bot. Never re-ask the same decision within 24h; small reversible actions: propose once.

- Finding: Slack bot profile U0C9ST69M8B has the lanes block but NOT the autonomy block (only Discord wawabot 1465631383862120451 has it). After deploy: `omo-gateway profile apply-block --bot-id U0C9ST69M8B --block autonomy --block-file bot_profiles/blocks/autonomy.md` and re-apply to 1465631383862120451. Also its sessions rule says 'Mention /sessions directly in Discord' -> owner edits persona text only with approval; leave as is.

## Gate (lead, once)
cargo fmt --check; cargo clippy --all-targets -D warnings; cargo test --no-fail-fast (known pre-existing fail r_dash_001); cd web && bun run build.
## Live QA
Tracker opens on Sessions with the 9 real sessions and correct states; send/interrupt on a scratch session; Slack DM reaction + failure line; disk alert dedupe unit; quiet-hour owner-active ask unit.

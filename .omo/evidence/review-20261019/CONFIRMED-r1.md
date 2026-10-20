# CONFIRMED-r1: review-20261019 round 1 (e57b472..c943813 + WIP)

- Lanes: 26 explore lanes (opus-5-5), DAG dag_a328a08c. This file is appended as lanes land.
- Citation resolution: every `Location:` citation in r1/*.md is resolved with the `resolveCites()` helper against the current tree (missing file / out-of-range lines are flagged).
- Rubric: P0 = reachable trigger causing data loss, misdelivery, a security breach, a crash or a broken build. P1 = reachable trigger causing wrong user-visible behavior, a hang, a silent drop, or a leak. Otherwise P2 (recorded, not fixed).
- Pre-existing failure (not from this diff): web `bun test` has 4 failures. bun cannot resolve the `@/` and `@hermes/shared` aliases. The result is identical on e57b472: 104 pass / 4 fail (`web-test-preexisting-e57b472.txt`).

## Confirmed P1 -> fixes
| Fix | Lane | Defect (lead-verified at source) | Owner |
|---|---|---|---|
| F1 | slack-commands | slash commands at channel top level target the channel key (commands.rs:436/610): model/yolo/steer/retry/skill-run silently create a channel session that makes ingress answer every unmentioned message (ingress.rs:144-153, :927); stop/deny miss the auto-thread session (ingress.rs:934) | lane st_01a154ad |
| F2 | sessions-a | `collect` awaits listing (20s/host) then probe (20s/host), which can exceed REPLY_TIMEOUT 25s (overview.rs:458/474, status_reply.rs:9) | lane st_01a154ac |
| F3 | sessions-a | watched-session completion notice is built only on the streaming-to-idle tick; dropped on quiet hours / dispatch failure (watch.rs:175/257/278/345) | lane st_01a154ae |
| F4 | slack-transport | intercept_lane_reply returns Ok(true) without advancing the cursor, so reconcile replays the reply as an agent turn (ingress.rs:172, backfill.rs:149) | LEAD: write_cursor before Ok(true) + test assertion |
| F5 | agent-daemon-deploy | `deploy.sh --verify` recursion gateway_pid -> detect_old_pid -> gateway_pid (deploy.sh:92/212) | LEAD |
| F6 | agent-daemon-deploy | `launchctl print` failure under set -euo pipefail silently aborts after install (deploy.sh:329) | LEAD: `|| true` |
| F7 | agent-daemon-deploy | DEPLOYED is written before verify and not restored on rollback; the rewind guard then pins a failed rev (deploy.sh:302) | LEAD: write only after verify OK |
| F8 | agent-daemon-deploy (P2 -> fixed, one-liner) | missing DEPLOYED aborts silently (deploy.sh:221) | LEAD: `|| true` |
| F9 | agent-daemon-deploy (P2 -> fixed) | deploy.sh lost its +x bit | LEAD: git index 100755 |
| F10 | cron | cron_report enqueued with lane_key NULL; dispatch collapses by (lane_key, kind), so reports from different jobs are lost (scheduler.rs:2433, dispatch.rs:116) | lane st_01a154af |
| F11 | cron | review_session has no DEFAULT_LEAD fallback, so delivery fails when OMON_LANE_DEFAULT_LEAD is unset (scheduler.rs:2630) | lane st_01a154af |
| F12 | cron | ack_command runs after queue_review, before real delivery (scheduler.rs:2271/2298) | lane st_01a154af |
| F13 | cron | ~/.omon/cron-reports grows unbounded and unredacted (scheduler.rs:2413-2427) | lane st_01a154af |

## Lanes with 0 P0/P1
- slack-interactive (2 P2), tracker-a (4 P2)

## Recorded P2 (not fixed)
- slack-commands: slash payload thread_ts unverified (commands.rs:137)
- slack-interactive: get_message on thread reply (message_context_slack.rs:262); 200-message thread window (:184)
- sessions-a: Slack home-channel owner_thread unreachable (status_reply.rs:69); probe failure hides listing (overview.rs:152); first_seen reset on failed snapshot (watch.rs:264)
- tracker-a: SES key takeover (seed.rs:92); system-team message text (workspace.rs:876); create_project not transactional (:387); label cycles (:769)
- slack-transport: startup backfill / reconcile overlap (runtime.rs:91); allow_all_users falls back to the Discord variable (slack/mod.rs:167)
- cron: interval miss check (scheduler.rs:1451); multi-target deliver session key (executor.rs:636); pgid race (executor.rs:1239)

## Wave 2 confirmations (lead regrade)
| ID | Lane | Sev | Location | Status |
|---|---|---|---|---|
| F14 | slack-ingress | P1 | ingress.rs route_awaiting_turn terminal error -> Ok(true) | FIXED (lead) |
| F15 | slack-ingress | P1 | /stop advance_cursor | FIXED (lead) |
| F16 | slack-ingress | P1 | auto-thread root channel cursor | FIXED (lead) |
| F17 | slack-ingress | P1 | backfill intercept_replayed_lane_reply | FIXED (lead) |
| F18 | tracker-b | P1 | issues.rs title/description PATCH on session issue | FIXED (st_01a154b0) |
| F19 | context-report | P1 | pack.rs user-less author = "bot <name>" | FIXED (lead) |
| F20 | context-report | P1 | claims.rs stateless filter_chunk | FIXED (lead) |
| F21 | web-issues | P1 | BoardView same-column reorder | FIXED (st_01a154b1) |
| F22 | web-issues | P1 | BoardView label-group composite ids | FIXED (st_01a154b1) |
| F23 | slack-egress | P1 | non-final early return | FIXED (lead) |
| F24 | slack-egress | P1 | completed key includes sequence | FIXED (lead) |
| F25 | web-core-shell | P1 | hotkeys sequence-first | FIXED (st_01a154b2) |
| F26 | web-core-shell | P1 | unreadCount excludes snoozed | FIXED (st_01a154b2) |
| F27 | web-projects-ui | P1 | ProjectPicker keeps milestone | FIXED (lead) |
| F28 | sessions-b | P1 | external.rs window seq not an id/cursor | FIXED (lead) |
| F29 | tracker-c | P1 | lanes::sync concurrency (SYNC_LOCK) | FIXED; delta VERIFIED |
| F30 | tracker-c | P1 | delegate vs sync adoption race | FIXED; delta VERIFIED |
| F31 | web-pages/web-detail-create | P1 | Inbox embedded IssuePage hotkeys | FIXED (lead) |
| F32 | web-pages | P1 | ViewPage dirty after save | FIXED (lead) |
| F33 | web-detail-create | P1 | comment link 404 | FIXED (lead) |
| F34 | web-detail-create | P1 | description autosave flush on unmount | FIXED (lead) |
| F35 | main-runtime | P1 | boot_slack auth.test/owner DM retry | FIXED (lead) |
| F36 | main-runtime | P2 | dashboard "session" -> Session | FIXED (lead, trivial) |
| R-A | autonomy-effect | P0/P1 -> P2 | OMON_FULL_ACCESS defaults on (owner instruction) | REGRADED |
| R-B | multiplexer-discord | P1 -> P2 | Discord claims (Discord offboarded) | REGRADED |
| F37 | agent-backend | P1 | omo_backend.rs transport-reset retry resubmits a turn whose tools ran or whose messages went out -> side_effects flag gates the retry; omo_daemon.rs unstamps LAST_FORCED_RESTART when nothing was killed | FIXED (lead) |
| F38 | agent-backend | P1 | follow.rs engine-turn final delivered twice while stopping (the pre-ack relay already delivers it) -> skip delivery when stopping | FIXED (lead) |
| F39 | autonomy-ledger | P1 | loop.rs items marked seen at collect -> lost on restart/flush error/cap; seen now written in the emit tx, cap defers | FIXED (lead) + tests d (deferred) / d2 (restart) |
| F40 | autonomy-ledger | P1 | ledger.rs carded engine approvals kept across restart swallow the owner's short reply -> all expire on restart; short reply ignores engine/tool rows without a live waiter; decide error no longer eaten | FIXED (lead), d3 test rewritten |
| F41 | lanes | P1 | guard.rs Alert branch silent after 24h even when disk got worse -> got_worse() ignores elapsed time | FIXED (lead) + test |
| F42 | inbox | P1 | Slack timestamp cursor skips late-indexed messages; use insertion rowid, rescan legacy timestamp cursors, and test late insertion plus upsert deduplication | FIXED; delta VERIFIED; final gate pending |
| R-C | agent-backend/autonomy-ledger/lanes | P2 | follower replay, ask spawn outliving turn, preset env, double precedent, dup github items, pause survives reopen, unregistered worktree, unmerged lane branch | NOT FIXED (P2, follow-up) |

## Delta evidence
- `r2/cron-deploy.md`: F5-F13, F35-F36 VERIFIED.
- `r2/web-tracker.md`: F18, F21-F22, F25-F27, F29-F34 VERIFIED.
- `r2/slack-sessions.md`: F1-F4, F14-F17, F19-F20, F23-F24, F28 VERIFIED.
- `r2/runtime-consolidated.md`: F37-F42 individually VERIFIED, preserving the initial F39 miss and amended verdict. `r2/runtime-followup.md` contains the focused cap scheduling trace.
- Lead rejected the initial F39 cap scheduling verdict, removed pending-batch reinsertion, and added a same-worker re-collection assertion before requesting the follow-up.
- Final integration evidence remains pending in `gate/run4/`.

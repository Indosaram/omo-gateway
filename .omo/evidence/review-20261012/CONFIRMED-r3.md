# CONFIRMED-r3 (lead falsification)

## r3-dashboard-main (st_01a132b9)
- 0 P0/P1. All four R2 dashboard fixes verified by trace (types/imports/shadowing/callers/web usage).
- [P2 recorded] standalone scheduler opt-in (OMON_DASHBOARD_RUN_SCHEDULER) undocumented in web/README.md.

## r3-actor-judge (st_01a132b8)
- 0 P0/P1. Reset (idle + mid-turn) and judge stdin/timeout join verified by trace, including tests exercising the fix and kill_on_drop on timeout.
- [P2 recorded] messages queued before a mid-turn /reset still run after it (no loss; semantic choice). Kept: a message the user sent is still answered, against the cleared context.

## r3-tools-env-ack (st_01a132c9)
- 0 P0/P1. Env filter verified (case-insensitive, gateway session env set before the filter, approval test at ~857 still refuses via Approval). ack_command screening verified (echo passes; Hermes import path unchanged).
- [P2 recorded] denylist still misses JDK_JAVA_OPTIONS, _JAVA_OPTIONS, PYTHONUSERBASE, PYTHONINSPECT, PERL5DB, CDPATH, GCONV_PATH, HOME, XDG_CONFIG_HOME (defense in depth; allowlist would be the full fix).

## r3-security-migrate-voice (st_01a132ba)
- 0 P0/P1. hardline boundary (5 listed forms), BACKSLASH_ESCAPE_RE, approval_display, config_import APPROVAL precedence, run_bounded, test_migrate pinned clock, chat-turn lifecycle all verified by trace.
- [P2 regression from lead fix -> FIXED] shutdown/reboot boundary dropped redirect-attached forms (reboot>/dev/null). Boundary class now includes < and >. Tests added.

## R3 VERDICT
All 4 R3 lanes: 0 confirmed P0/P1 after lead falsification. Loop terminates; proceed to the single integration gate.

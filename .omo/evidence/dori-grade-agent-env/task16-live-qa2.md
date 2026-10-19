# Task 16 live QA2

FAIL: insertion generation 1 remained consumed 0 / admitted_ms null, with zero kibitzer usage, judge and events at deadline. Idle subscenario NEEDS-REVIEW: monitor terminated execution before required ten minutes; final authority remained unchanged, but numeric gateway exit and complete idle duration are unavailable. No relaunch.

## inbox-insert-real-kibitzer

Root: `/private/tmp/omon-task16-qa2-Cnugr8`. Scenario exit 1. Gateway cleanup: `[{"pid":35716,"code":0,"pidAbsent":true,"socketRemoved":true}]`. Raw evidence: `/private/tmp/omon-task16-qa2-Cnugr8/task16-qa2.json`.

```text
Error: single real kibitzer decision deadline; final authoritative query failed
    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:39:71)
```

## idle-ten-minutes

Root: `/private/tmp/omon-task16-qa2-eMULg7`. Scenario exit 1. Gateway cleanup: `[{"pid":35709,"code":null,"pidAbsent":true,"socketRemoved":true}]`. Raw evidence: `task-16-qa2-idle-interrupted.json`.

```text
Execution monitor timed_out before required 600000 ms idle measurement; runner disappeared and no final helper receipt was emitted. No rerun authorized.
```

Idle orphan app-server PID 35656 was stopped with SIGTERM to its owned group after runner disappearance; kill command exited 0, PID disappeared, port 55582 exclusive rebind succeeded. Its private token directory is retained because the helper owner was terminated; no credential contents were read.

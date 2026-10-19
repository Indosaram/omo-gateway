# Task 20 live QA2

FAIL: persona lane-start predicate timed out; zero lanes and zero model usage. Profile apply twice was idempotent and missing profile exited 2. Reply-meaning coverage remains NEEDS-REVIEW and was not reached.

## task20-scenarios

Raw receipt: `/private/tmp/omon-task20-qa2-VvXove/task20-qa2.json`. Retained DB: `/private/tmp/omon-task20-qa2-VvXove/qa.db`. Scenario exit 1. Gateway cleanup: `[{"pid":35713,"code":0,"pidAbsent":true,"socketRemoved":true},{"pid":36170,"code":0,"pidAbsent":true,"socketRemoved":true}]`. Owned app-server cleanup: `{"pid":35664,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

```text
Error: persona real lane start tool deadline; final authoritative query failed
    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:39:71)
```

Detached owned RPC process groups were subsequently killed; see qa2-owned-orphan-cleanup.json. No shared daemon was touched.

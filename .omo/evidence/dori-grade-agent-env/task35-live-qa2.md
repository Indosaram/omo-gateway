# Task 35 live QA2

FAIL: both real lane turns completed, but model_usage remained empty. Token predicate timed out; usage CLI, five-minute settlement/close and unrelated-diff done_diff rejection were not reached. Agent prose is retained but not used as success proof.

## real-lane-tokens

Raw receipt: `/private/tmp/omon-task35-qa2-X9CZFj/task35-qa2.json`. Retained DB: `/private/tmp/omon-task35-qa2-X9CZFj/qa.db`. Scenario exit 1. Gateway cleanup: `[{"pid":35745,"code":0,"pidAbsent":true,"socketRemoved":true}]`. Owned app-server cleanup: `{"pid":35707,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

```text
Error
    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:39:71)
```

## real-unrelated-diff-rejected

Raw receipt: `/private/tmp/omon-task35-qa2-cKSB2N/task35-qa2.json`. Retained DB: `/private/tmp/omon-task35-qa2-cKSB2N/qa.db`. Scenario exit 1. Gateway cleanup: `[{"pid":35742,"code":0,"pidAbsent":true,"socketRemoved":true}]`. Owned app-server cleanup: `{"pid":35708,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

```text
Error
    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:39:71)
```

Detached owned RPC process groups were subsequently killed; see qa2-owned-orphan-cleanup.json. No shared daemon was touched.

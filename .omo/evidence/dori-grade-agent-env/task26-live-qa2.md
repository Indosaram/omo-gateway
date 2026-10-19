# Task 26 live QA2

NEEDS-REVIEW: corrected proposal CLI created proposal 1; real unapproved lead reply persisted, zero lanes. CLI approval persisted status approved; native proposal stayed pending with null approval_message_id and item lane_key null. Approved persona lane was not observed; monitor interrupted final receipt. Authenticated Discord numbered approval was not exercised; numeric gateway exit unavailable.

## task26-scenarios

Root: `/private/tmp/omon-task26-qa2-0PK82X`; DB `/private/tmp/omon-task26-qa2-0PK82X/qa.db` retained. Scenario exit 1. Gateway cleanup: `[{"pid":35735,"code":0,"pidAbsent":true,"socketRemoved":true}]`.

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway inbox-proposal qa26 --channel 1474231559895646240 --action-json {"kind":"repo","cwd":"/private/tmp/omon-task26-qa2-0PK82X/workspace/qa26","done":"file done.txt"}

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:109:12)
    at async approvalScenario (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:216:39)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:263:25)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:147:34)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:115)
    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)
    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:43)
    at processTicksAndRejections (native:7:39)
```

## corrected-preparation-retry

Root: `/private/tmp/omon-task26-qa2-7Glkdp`; DB `/private/tmp/omon-task26-qa2-7Glkdp/qa.db` retained. Scenario exit 1. Gateway cleanup: `[{"pid":42500,"code":null,"pidAbsent":true,"socketRemoved":true}]`.

```text
Execution monitor timed_out; no final scenario receipt emitted. No relaunch.
```

Corrected retry backend was killed only after its runner disappeared. See qa2-owned-orphan-cleanup-extra.json; PIDs absent, socket removed, backend ports 55814/55815 exclusively rebound. Private token directories retained after interrupted helper owner. No model override or external source write.

# Task 15 live QA2

Authenticated Discord input boundary: NEEDS-REVIEW. Close, archive, worktree removal and durable-reply reopen were not reached because lane start failed.

## dirty-worktree-refused

Scratch: `/private/tmp/omon-task15-qa2-AX05TM`. Scenario exit: 1. Gateway cleanup: `[{"pid":35720,"code":0,"pidAbsent":true,"socketRemoved":true}]`. App-server cleanup: `{"pid":35645,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

Raw receipt: `/private/tmp/omon-task15-qa2-AX05TM/task15-qa2.json`. DB retained: `/private/tmp/omon-task15-qa2-AX05TM/qa.db`.

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa15-dirty --title QA2 qa15-dirty --brief /private/tmp/omon-task15-qa2-AX05TM/qa15-dirty-brief.md --done file done.txt --cwd /private/tmp/omon-task15-qa2-AX05TM/workspace/lane-worktree --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:109:12)
    at async startLane (unknown)
    at unknown
    at unknown
    at unknown
    at async withOwnedAppServer (unknown)
    at async runQA (unknown)
    at processTicksAndRejections (native:7:39)
```

## close-archive-reply-unarchive

Scratch: `/private/tmp/omon-task15-qa2-CK16AC`. Scenario exit: 1. Gateway cleanup: `[{"pid":35700,"code":0,"pidAbsent":true,"socketRemoved":true}]`. App-server cleanup: `{"pid":35644,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

Raw receipt: `/private/tmp/omon-task15-qa2-CK16AC/task15-qa2.json`. DB retained: `/private/tmp/omon-task15-qa2-CK16AC/qa.db`.

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa15-clean --title QA2 qa15-clean --brief /private/tmp/omon-task15-qa2-CK16AC/qa15-clean-brief.md --done file done.txt --cwd /private/tmp/omon-task15-qa2-CK16AC/workspace/lane-worktree --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:109:12)
    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:28:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:46:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:147:34)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:115)
    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)
    at async runQA (unknown)
    at processTicksAndRejections (native:7:39)
```

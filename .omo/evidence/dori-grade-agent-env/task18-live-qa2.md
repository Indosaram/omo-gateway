# Task 18 live QA2

Two acknowledged milestone posts, status PATCH and deletion/404 replacement were not reached.

## two-milestones

Scratch: `/private/tmp/omon-task18-qa2-IllvXn`. Scenario exit: 1. Gateway cleanup: `[{"pid":35712,"code":0,"pidAbsent":true,"socketRemoved":true}]`. App-server cleanup: `{"pid":35661,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

Raw receipt: `/private/tmp/omon-task18-qa2-IllvXn/task18-qa2.json`. DB retained: `/private/tmp/omon-task18-qa2-IllvXn/qa.db`.

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa18 --title QA2 qa18 --brief /private/tmp/omon-task18-qa2-IllvXn/qa18-brief.md --done file unused.txt --cwd /private/tmp/omon-task18-qa2-IllvXn/workspace/lane --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:109:12)
    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:28:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:109:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:147:34)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:115)
    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)
    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:43)
    at processTicksAndRejections (native:7:39)
```

## deleted-status-replaced

Scratch: `/private/tmp/omon-task18-qa2-pXeG9j`. Scenario exit: 1. Gateway cleanup: `[{"pid":35719,"code":0,"pidAbsent":true,"socketRemoved":true}]`. App-server cleanup: `{"pid":35662,"exit":{"code":null,"signal":"SIGKILL"},"pidAbsent":true,"remaining":[],"rebind":true}`.

Raw receipt: `/private/tmp/omon-task18-qa2-pXeG9j/task18-qa2.json`. DB retained: `/private/tmp/omon-task18-qa2-pXeG9j/qa.db`.

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa18 --title QA2 qa18 --brief /private/tmp/omon-task18-qa2-pXeG9j/qa18-brief.md --done file unused.txt --cwd /private/tmp/omon-task18-qa2-pXeG9j/workspace/lane --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:109:12)
    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:28:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:109:11)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:147:34)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:115)
    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)
    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:150:43)
    at processTicksAndRejections (native:7:39)
```

# Task 32: FAIL

## task32-final

Scratch and DB: `/private/tmp/omon-task32-live-final-YhA5QX`, `/private/tmp/omon-task32-live-final-YhA5QX/qa.db`.
Raw receipt: `/private/tmp/omon-task32-live-final-YhA5QX/task32-live-final.json`. Authoritative state: `/private/tmp/omon-task32-live-final-YhA5QX/final-machine-state.json`.
SHA before/after: `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52` / `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52`.
Gateway cleanup: [{"pid":84384,"code":0,"pidAbsent":true,"socketRemoved":true}]. App-server absent: true; group remaining: []; port rebind true.

Error verbatim:

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/omo-gateway cron run-now qa32-ok --wait --json

2 !== 0

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:154:12)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-extra.mjs:56:15)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:211:34)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:214:115)
    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)
    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:214:43)
    at processTicksAndRejections (native:7:39)
```

Command evidence:

```json
[
  {
    "argv": [
      "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/omo-gateway",
      "cron",
      "run-now",
      "qa32-ok",
      "--wait",
      "--json"
    ],
    "pid": 84916,
    "code": 2,
    "out": "",
    "err": "{\"error\":\"configuration error: No such file or directory (os error 2)\",\"ok\":false}\n"
  }
]
```

Final RPC group audit follows in final-qa-summary.md.

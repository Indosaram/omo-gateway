# Task 31: FAIL

## task31-final

Scratch and DB: `/private/tmp/omon-task31-live-final-LYUKOY`, `/private/tmp/omon-task31-live-final-LYUKOY/qa.db`.
Raw receipt: `/private/tmp/omon-task31-live-final-LYUKOY/task31-live-final.json`. Authoritative state: `/private/tmp/omon-task31-live-final-LYUKOY/final-machine-state.json`.
SHA before/after: `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52` / `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52`.
Gateway cleanup: [{"pid":84689,"code":0,"pidAbsent":true,"socketRemoved":true}]. App-server absent: true; group remaining: []; port rebind true.

Error verbatim:

```text
AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/omo-gateway cron run-now qa31-scratch --wait --json

2 !== 1

    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:154:12)
    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-extra.mjs:44:38)
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
      "/usr/bin/git",
      "init",
      "/private/tmp/omon-task31-live-final-LYUKOY/workspace/cron-fixture"
    ],
    "pid": 84994,
    "code": 0,
    "out": "Initialized empty Git repository in /private/tmp/omon-task31-live-final-LYUKOY/workspace/cron-fixture/.git/\n",
    "err": ""
  },
  {
    "argv": [
      "/usr/bin/git",
      "-C",
      "/private/tmp/omon-task31-live-final-LYUKOY/workspace/cron-fixture",
      "add",
      "job.py"
    ],
    "pid": 85175,
    "code": 0,
    "out": "",
    "err": ""
  },
  {
    "argv": [
      "/usr/bin/git",
      "-C",
      "/private/tmp/omon-task31-live-final-LYUKOY/workspace/cron-fixture",
      "-c",
      "user.name=Owned QA",
      "-c",
      "user.email=qa@example.invalid",
      "commit",
      "-m",
      "Failing scratch cron fixture"
    ],
    "pid": 85264,
    "code": 0,
    "out": "[main (root-commit) c775760] Failing scratch cron fixture\n 1 file changed, 2 insertions(+)\n create mode 100644 job.py\n",
    "err": ""
  },
  {
    "argv": [
      "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/omo-gateway",
      "cron",
      "run-now",
      "qa31-scratch",
      "--wait",
      "--json"
    ],
    "pid": 85326,
    "code": 2,
    "out": "",
    "err": "{\"error\":\"configuration error: No such file or directory (os error 2)\",\"ok\":false}\n"
  }
]
```

Final RPC group audit follows in final-qa-summary.md.

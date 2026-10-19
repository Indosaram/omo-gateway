# Task 25: NEEDS-REVIEW

## task25-scenarios

Scratch and DB: `/private/tmp/omon-task25-live-final-ykHscd`, `/private/tmp/omon-task25-live-final-ykHscd/qa.db`.
Raw receipt: `/private/tmp/omon-task25-live-final-ykHscd/task25-live-final.json`. Authoritative state: `/private/tmp/omon-task25-live-final-ykHscd/final-machine-state.json`.
SHA before/after: `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52` / `4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52`.
Gateway cleanup: [{"pid":84435,"code":0,"pidAbsent":true,"socketRemoved":true}]. App-server absent: true; group remaining: []; port rebind true.

Error verbatim:

```text
[
  {
    "scenario": "native digest local headless delivery",
    "reason": "Current headless main.rs never constructs CronScheduler/DigestExecutor or registers omon:inbox-digest; standalone dashboard uses AgentCronExecutor, not DigestExecutor. Do not substitute a script/model-generated digest for the native executor."
  }
]
```

Command evidence:

```json
[]
```

Final RPC group audit follows in final-qa-summary.md.

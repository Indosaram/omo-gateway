# Task 25 LAST live QA

Verdict: **FAIL**. Binary SHA-256: `0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00` (before and after match).

Scratch root: `/private/tmp/omon-task25-live-final-xIB5GK`; retained DB: `/private/tmp/omon-task25-live-final-xIB5GK/qa.db`. Deadline: 600000 ms, 2026-10-12T03:51:33.374Z.

Native digest registration: false. Authoritative cron_jobs query for omon:inbox-digest returned []; delivery output count 0. Native delivery not reached. The helper labels this blocked, but the requested registration is absent, so this scenario is FAIL. Its explanatory source-code prose is not treated as proof.

## Raw command and cleanup receipt

```json
{
  "todo": 25,
  "scenario": "task25-scenarios",
  "root": "/private/tmp/omon-task25-live-final-xIB5GK",
  "dbPath": "/private/tmp/omon-task25-live-final-xIB5GK/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54549",
        "-Fn"
      ],
      "pid": 54839,
      "code": 0,
      "out": "p54549\nfcwd\nn/private/tmp/omon-task25-live-final-xIB5GK\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task25-live-final-xIB5GK/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xb8d2c85653d6dc0d\nf2\nn->0x45ea8e514cb87cff\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x307847ba5f5b49d7\nf7\nn->0x1d91de796177e205\nf8\nn->0x307847ba5f5b49d7\nf9\nn/private/tmp/omon-task25-live-final-xIB5GK/qa.db\nf10\nn/private/tmp/omon-task25-live-final-xIB5GK/qa.db-wal\nf11\nn/private/tmp/omon-task25-live-final-xIB5GK/qa.db-shm\nf12\nn/private/tmp/omon-task25-live-final-xIB5GK/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-874bc6a8a6add1f2e037efd3.lock\nf14\nn/tmp/omon-lanes-874bc6a8a6add1f2e037efd3.sock\n",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 54549,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "blocked",
  "deadlineMs": 600000,
  "deadlineAt": "2026-10-12T03:51:33.374Z",
  "judgeEnv": {
    "keys": [
      "TYPESAFE_API_KEY",
      "TYPESAFE_URL",
      "TYPESAFE_MODEL"
    ],
    "loadedFrom": "owner ~/.zshenv",
    "valuesRecorded": false
  },
  "initial": {
    "pragma": [
      {
        "seq": 0,
        "name": "main",
        "file": "/private/tmp/omon-task25-live-final-xIB5GK/qa.db"
      }
    ],
    "lanes": {
      "n": 0
    },
    "messages": {
      "n": 0
    }
  },
  "blocked": [
    {
      "scenario": "native digest local headless delivery",
      "reason": "Current headless main.rs never constructs CronScheduler/DigestExecutor or registers omon:inbox-digest; standalone dashboard uses AgentCronExecutor, not DigestExecutor. Do not substitute a script/model-generated digest for the native executor."
    }
  ],
  "appserver": {
    "startedAt": "2026-10-12T03:41:33.414Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:53565",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-XRYdR3/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task25-live-final-xIB5GK/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:53565",
    "port": 53565,
    "timingsMs": {
      "ready": 694,
      "cleanup": 80,
      "total": 2202
    },
    "pid": 54319,
    "membersBeforeShutdown": [
      54319
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": 0,
      "signal": null
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:41:35.613Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 3,
  "scratchRetained": true
}
```

Final DB state: `/private/tmp/omon-task25-live-final-xIB5GK/final-machine-state.json`. RPC-group verification is pending the aggregate cleanup sweep.


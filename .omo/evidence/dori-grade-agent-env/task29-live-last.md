# Task 29 LAST live QA: authorized readonly-wrapper rerun

Verdict: **FAIL** for the 30-second reply deadline. Original host_unavailable-related scratch-HOME run is superseded as a harness issue, not a proven product bug. After the authorized wrapper was applied, the exact 30000 ms observation again expired with one user message, zero assistant messages and zero delivery obligations. No further rerun or unproven root-cause claim.

## Rerun DB state

```json
{
  "messages": [
    {
      "sequence": 1,
      "id": "404c8644-6fa4-4e25-aa70-4011c1870437",
      "session_key": "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
      "role": "user",
      "content": "지금 실행 중인 세션 상태 알려줘. 로컬 세션만 조회하고 목록을 보여줘. send/interrupt/adopt는 하지 마.",
      "metadata_json": "[]",
      "created_at": "2026-10-12T03:56:05.142234+00:00",
      "platform_message_id": "af6fe16b-17eb-4db8-8dde-2c8e8a84dfd6"
    }
  ],
  "deliveries": []
}
```

## Rerun receipt and exact wrapper

```json
{
  "todo": 29,
  "scenario": "task29-final",
  "root": "/private/tmp/omon-task29-live-final-1lkxge",
  "dbPath": "/private/tmp/omon-task29-live-final-1lkxge/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "91893",
        "-Fn"
      ],
      "pid": 91930,
      "code": 0,
      "out": "p91893\nfcwd\nn/private/tmp/omon-task29-live-final-1lkxge\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0x94fcddc631665c59\nf2\nn->0x5f774a4ee17bf00f\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xf45b717c8c521356\nf7\nn->0xb33b0455c67299db\nf8\nn->0xf45b717c8c521356\nf9\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db\nf10\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-wal\nf11\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-shm\nf12\nn/private/tmp/omon-task29-live-final-1lkxge/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-6e0ed78c4abe3e2507403e5f.lock\nf14\nn/tmp/omon-lanes-6e0ed78c4abe3e2507403e5f.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "91950",
        "-Fn"
      ],
      "pid": 91981,
      "code": 0,
      "out": "p91950\nfcwd\nn/private/tmp/omon-task29-live-final-1lkxge\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xddcb9b49a6b85374\nf2\nn->0x94fcddc631665c59\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x806fa9335686b6f4\nf7\nn->0x495655ded3d6fa81\nf8\nn->0x806fa9335686b6f4\nf9\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db\nf10\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db\nf11\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-wal\nf12\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-wal\nf13\nn/private/tmp/omon-task29-live-final-1lkxge/qa.db-shm\nf14\nn/private/tmp/omon-task29-live-final-1lkxge/home/.omon/cron-run-now.sock\nf15\nn/private/tmp/omon-lanes-6e0ed78c4abe3e2507403e5f.lock\nf16\nn/tmp/omon-lanes-6e0ed78c4abe3e2507403e5f.sock\nf21\nnlocalhost:55822\n",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 91893,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    },
    {
      "pid": 91950,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "failed",
  "deadlineMs": 600000,
  "deadlineAt": "2026-10-12T04:06:04.119Z",
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
        "file": "/private/tmp/omon-task29-live-final-1lkxge/qa.db"
      }
    ],
    "lanes": {
      "n": 0
    },
    "messages": {
      "n": 0
    }
  },
  "readonlyOmoWrapper": {
    "path": "/private/tmp/omon-task29-live-final-1lkxge/bin/omo",
    "content": "#!/bin/sh\ncase \"$1:$2\" in thread:list|thread:read) exec /usr/bin/env HOME=/Users/indo /Users/indo/.bun/bin/omo \"$@\" ;; *) exit 64 ;; esac\n",
    "allowed": [
      "thread list",
      "thread read"
    ],
    "rejectedExit": 64,
    "gatewayHome": "/private/tmp/omon-task29-live-final-1lkxge/home",
    "readonlyThreadHome": "/Users/indo"
  },
  "error": "Error: session status delivered within 30 seconds deadline; final authoritative query failed\n    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:77:71)",
  "appserver": {
    "startedAt": "2026-10-12T03:56:04.161Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:55820",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-nMjE3t/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task29-live-final-1lkxge/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:55820",
    "port": 55820,
    "timingsMs": {
      "ready": 669,
      "cleanup": 15137,
      "total": 46127
    },
    "pid": 91737,
    "membersBeforeShutdown": [
      91737,
      92364
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": null,
      "signal": "SIGKILL"
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:56:50.285Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 1,
  "scratchRetained": true
}
```

## Superseded initial receipt

```json
{
  "todo": 29,
  "scenario": "task29-final",
  "root": "/private/tmp/omon-task29-live-final-sqx30O",
  "dbPath": "/private/tmp/omon-task29-live-final-sqx30O/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54605",
        "-Fn"
      ],
      "pid": 54838,
      "code": 0,
      "out": "p54605\nfcwd\nn/private/tmp/omon-task29-live-final-sqx30O\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xe88440962f564e23\nf2\nn->0x99fcdec353db33c8\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xa9350d3e2166b24b\nf7\nn->0x653681483b02efcd\nf8\nn->0xa9350d3e2166b24b\nf9\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db\nf10\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-wal\nf11\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-shm\nf12\nn/private/tmp/omon-task29-live-final-sqx30O/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-9e5b9f809d8c955f6fd9f8fa.lock\nf14\nn/tmp/omon-lanes-9e5b9f809d8c955f6fd9f8fa.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54878",
        "-Fn"
      ],
      "pid": 54915,
      "code": 0,
      "out": "p54878\nfcwd\nn/private/tmp/omon-task29-live-final-sqx30O\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0x176d0b55ab106096\nf2\nn->0x14d2cd96089c81ab\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x653681483b02efcd\nf7\nn->0x307847ba5f5b49d7\nf8\nn->0x653681483b02efcd\nf9\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db\nf10\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db\nf11\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-wal\nf12\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-wal\nf13\nn/private/tmp/omon-task29-live-final-sqx30O/qa.db-shm\nf14\nn/private/tmp/omon-task29-live-final-sqx30O/home/.omon/cron-run-now.sock\nf15\nn/private/tmp/omon-lanes-9e5b9f809d8c955f6fd9f8fa.lock\nf16\nn/tmp/omon-lanes-9e5b9f809d8c955f6fd9f8fa.sock\nf21\nnlocalhost:53608\n",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 54605,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    },
    {
      "pid": 54878,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "failed",
  "deadlineMs": 600000,
  "deadlineAt": "2026-10-12T03:51:33.582Z",
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
        "file": "/private/tmp/omon-task29-live-final-sqx30O/qa.db"
      }
    ],
    "lanes": {
      "n": 0
    },
    "messages": {
      "n": 0
    }
  },
  "error": "Error: session status delivered within 30 seconds deadline; final authoritative query failed\n    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:77:71)",
  "appserver": {
    "startedAt": "2026-10-12T03:41:33.623Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:53570",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-9Rbqx8/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task29-live-final-sqx30O/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:53570",
    "port": 53570,
    "timingsMs": {
      "ready": 978,
      "cleanup": 15067,
      "total": 47385
    },
    "pid": 54381,
    "membersBeforeShutdown": [
      54381,
      55363
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": null,
      "signal": "SIGKILL"
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:42:21.004Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 1,
  "scratchRetained": true
}
```

All gateway exits numeric 0, PID and socket absence, owned app-server shutdown and escaped RPC-group cleanup are verified in last-qa-integrity.json. Both scratch DBs retained.


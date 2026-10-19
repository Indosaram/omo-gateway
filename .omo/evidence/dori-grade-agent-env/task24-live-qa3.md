# Task 24 live QA3

Release: wave4c. Source and binary were verified before execution.

## task24-scenarios

Verdict: FAIL. Scratch retained: /private/tmp/omon-task24-qa3-LRy2gn.

```json
{
  "receipt": {
    "todo": 24,
    "scenario": "task24-scenarios",
    "root": "/private/tmp/omon-task24-qa3-LRy2gn",
    "dbPath": "/private/tmp/omon-task24-qa3-LRy2gn/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56807",
          "-Fn"
        ],
        "pid": 57065,
        "code": 0,
        "out": "p56807\nfcwd\nn/private/tmp/omon-task24-qa3-LRy2gn\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task24-qa3-LRy2gn/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xd5fcb90bc6e0c300\nf2\nn->0xae2024658ad7e867\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x87b63c6b680b1861\nf7\nn->0x12f9625279ec9da2\nf8\nn->0x87b63c6b680b1861\nf9\nn/private/tmp/omon-task24-qa3-LRy2gn/qa.db\nf10\nn/private/tmp/omon-task24-qa3-LRy2gn/qa.db-wal\nf11\nn/private/tmp/omon-task24-qa3-LRy2gn/qa.db-shm\nf12\nn/private/tmp/omon-lanes-db933632c42dbf3e7b51198e.lock\nf13\nn/tmp/omon-lanes-db933632c42dbf3e7b51198e.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/private/tmp/omon-task24-qa3-LRy2gn/bin/zele",
          "mail",
          "list",
          "--folder",
          "inbox",
          "--account",
          "freedomzero91@gmail.com",
          "--filter",
          "is:unread -category:promotions -category:social -category:updates",
          "--limit",
          "1"
        ],
        "pid": 57090,
        "code": 1,
        "outputBytes": 0,
        "errorBytes": 1016
      }
    ],
    "cleanup": [
      {
        "pid": 56807,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 1200000,
    "deadlineAt": "2026-10-12T01:02:34.075Z",
    "judgeEnv": {
      "keys": [
        "TYPESAFE_API_KEY",
        "TYPESAFE_URL",
        "TYPESAFE_MODEL"
      ],
      "loadedFrom": "owner ~/.zshenv",
      "valuesRecorded": false
    },
    "mailSource": {
      "home": "/Users/indo",
      "commands": [
        "mail list",
        "mail watch"
      ],
      "ownerHomeWrites": "denied by sandbox-exec file-write* policy",
      "otherHome": "/private/tmp/omon-task24-qa3-LRy2gn/home",
      "contentLogged": false
    },
    "initial": {
      "pragma": [
        {
          "seq": 0,
          "name": "main",
          "file": "/private/tmp/omon-task24-qa3-LRy2gn/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /private/tmp/omon-task24-qa3-LRy2gn/bin/zele mail list --folder inbox --account freedomzero91@gmail.com --filter is:unread -category:promotions -category:social -category:updates --limit 1\n\n1 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:154:12)\n    at async collectScenario (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:190:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:274:25)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:214:115)\n    at async withOwnedAppServer (unknown)\n    at async runQA (unknown)\n    at processTicksAndRejections (native:7:39)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:34.213Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58582",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-jEOBOl/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task24-qa3-LRy2gn/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58582",
      "port": 58582,
      "timingsMs": {
        "ready": 3158,
        "cleanup": 194,
        "total": 8524
      },
      "pid": 56696,
      "membersBeforeShutdown": [
        56696
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:42.731Z"
    },
    "finalBinarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "exitCode": 1,
    "scratchRetained": true
  },
  "dbState": {
    "lanesCount": 0,
    "lanes": [],
    "lane_historyCount": 0,
    "lane_history": [],
    "lane_transcriptCount": 0,
    "judge_callsCount": 0,
    "judge_calls": [],
    "model_usageCount": 0,
    "model_usage": [],
    "inbox_itemsCount": 0,
    "inbox_decisionsCount": 0,
    "inbox_cursorsCount": 0,
    "inbox_cursors": [],
    "cron_jobsCount": 0,
    "messagesCount": 0,
    "delivery_obligationsCount": 0
  }
}
```

Readonly mail preflight exited 1 with stdout 0 bytes and stderr 1016 bytes. Source contents were deliberately not logged; exact stderr is unavailable under the helper privacy contract. Collection was not enabled and no inbox rows were produced.


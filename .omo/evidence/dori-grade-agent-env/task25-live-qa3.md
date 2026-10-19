# Task 25 live QA3

Release: wave4c. Source and binary were verified before execution.

## task25-scenarios

Verdict: NEEDS-REVIEW. Scratch retained: /private/tmp/omon-task25-qa3-O06dJV.

```json
{
  "receipt": {
    "todo": 25,
    "scenario": "task25-scenarios",
    "root": "/private/tmp/omon-task25-qa3-O06dJV",
    "dbPath": "/private/tmp/omon-task25-qa3-O06dJV/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56813",
          "-Fn"
        ],
        "pid": 56994,
        "code": 0,
        "out": "p56813\nfcwd\nn/private/tmp/omon-task25-qa3-O06dJV\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task25-qa3-O06dJV/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0x23198637650ac646\nf2\nn->0xa628fa9ceb5b2363\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x53afd0c466049c73\nf7\nn->0x6811bfc22e8626a1\nf8\nn->0x53afd0c466049c73\nf9\nn/private/tmp/omon-task25-qa3-O06dJV/qa.db\nf10\nn/private/tmp/omon-task25-qa3-O06dJV/qa.db-wal\nf11\nn/private/tmp/omon-task25-qa3-O06dJV/qa.db-shm\nf12\nn/private/tmp/omon-lanes-625533ea6098da6c05b4a1f0.lock\nf13\nn/tmp/omon-lanes-625533ea6098da6c05b4a1f0.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "inbox",
          "list",
          "--class",
          "owner",
          "--json"
        ],
        "pid": 57070,
        "code": 0,
        "out": "[\n  {\n    \"id\": \"qa25-owner\",\n    \"source\": \"discord\",\n    \"author\": null,\n    \"title\": \"Owned QA\",\n    \"class\": \"owner\",\n    \"reason\": \"Owned QA fixture\",\n    \"proposed_action\": \"Perform isolated coding task\",\n    \"due_at\": null,\n    \"received_at\": \"2026-10-12T00:42:38.857Z\",\n    \"link\": \"https://example.invalid/qa25-owner\",\n    \"status\": \"open\"\n  }\n]\n",
        "err": ""
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "inbox",
          "set",
          "nope",
          "--status",
          "done"
        ],
        "pid": 57088,
        "code": 2,
        "out": "",
        "err": "unknown inbox item: nope\n"
      }
    ],
    "cleanup": [
      {
        "pid": 56813,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "blocked",
    "deadlineMs": 600000,
    "deadlineAt": "2026-10-12T00:52:34.493Z",
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
          "file": "/private/tmp/omon-task25-qa3-O06dJV/qa.db"
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
      "startedAt": "2026-10-12T00:42:34.599Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58583",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-63jGLk/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task25-qa3-O06dJV/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58583",
      "port": 58583,
      "timingsMs": {
        "ready": 2990,
        "cleanup": 303,
        "total": 5133
      },
      "pid": 56751,
      "membersBeforeShutdown": [
        56751
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:39.712Z"
    },
    "finalBinarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "exitCode": 3,
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
    "inbox_itemsCount": 2,
    "inbox_decisionsCount": 2,
    "inbox_cursorsCount": 0,
    "inbox_cursors": [],
    "cron_jobsCount": 0,
    "messagesCount": 0,
    "delivery_obligationsCount": 0
  }
}
```

Seeded owner list count 1 and unknown item exit 2: PASS. Native digest job count 0: NEEDS-REVIEW; no substituted executor.


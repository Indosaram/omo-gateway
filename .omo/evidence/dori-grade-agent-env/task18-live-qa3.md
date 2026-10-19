# Task 18 live QA3

Release: wave4c. Source and binary were verified before execution.

## deleted-status-replaced

Verdict: FAIL. Scratch retained: /private/tmp/omon-task18-qa3-3Kr90t.

```json
{
  "receipt": {
    "todo": 18,
    "scenario": "deleted-status-replaced",
    "root": "/private/tmp/omon-task18-qa3-3Kr90t",
    "dbPath": "/private/tmp/omon-task18-qa3-3Kr90t/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56804",
          "-Fn"
        ],
        "pid": 57063,
        "code": 0,
        "out": "p56804\nfcwd\nn/private/tmp/omon-task18-qa3-3Kr90t\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task18-qa3-3Kr90t/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xa0da1427030a65cd\nf2\nn->0x430ff5ed2d6ac518\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x916f5731bd9cb07b\nf7\nn->0xbf0cf341bc53b763\nf8\nn->0x916f5731bd9cb07b\nf9\nn/private/tmp/omon-task18-qa3-3Kr90t/qa.db\nf10\nn/private/tmp/omon-task18-qa3-3Kr90t/qa.db-wal\nf11\nn/private/tmp/omon-task18-qa3-3Kr90t/qa.db-shm\nf12\nn/private/tmp/omon-lanes-c0acee549f7467a885ba3628.lock\nf13\nn/tmp/omon-lanes-c0acee549f7467a885ba3628.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "init",
          "/private/tmp/omon-task18-qa3-3Kr90t/workspace/lane"
        ],
        "pid": 57094,
        "code": 0,
        "out": "Initialized empty Git repository in /private/tmp/omon-task18-qa3-3Kr90t/workspace/lane/.git/\n",
        "err": ""
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "lane",
          "start",
          "qa18",
          "--title",
          "QA2 qa18",
          "--brief",
          "/private/tmp/omon-task18-qa3-3Kr90t/qa18-brief.md",
          "--done",
          "file unused.txt",
          "--cwd",
          "/private/tmp/omon-task18-qa3-3Kr90t/workspace/lane",
          "--lead",
          "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
          "--thread",
          "700000000000000002"
        ],
        "pid": 57149,
        "code": 2,
        "out": "{\"error\":\"configuration error: discord api error (status: 500 Internal Server Error, code: -1): [Serenity] Could not decode json when receiving error response from discord:, expected value at line 1 column 1\",\"ok\":false}\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56804,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 900000,
    "deadlineAt": "2026-10-12T00:57:33.996Z",
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
          "file": "/private/tmp/omon-task18-qa3-3Kr90t/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa18 --title QA2 qa18 --brief /private/tmp/omon-task18-qa3-3Kr90t/qa18-brief.md --done file unused.txt --cwd /private/tmp/omon-task18-qa3-3Kr90t/workspace/lane --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002\n\n2 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:154:12)\n    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:35:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:120:11)\n    at unknown\n    at unknown\n    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)\n    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:214:43)\n    at processTicksAndRejections (native:7:39)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:34.009Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58580",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-4wW5hR/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task18-qa3-3Kr90t/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58580",
      "port": 58580,
      "timingsMs": {
        "ready": 3041,
        "cleanup": 218,
        "total": 8722
      },
      "pid": 56663,
      "membersBeforeShutdown": [
        56663
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:42.723Z"
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
    "judge_callsCount": 1,
    "judge_calls": [
      {
        "id": "7855d7b2-075a-46ac-a000-c127be1ec35c",
        "purpose": "lane_size",
        "kind": "choice",
        "ms": 2050,
        "outcome": "small",
        "used_fallback": 0,
        "created_at": "2026-10-12T00:42:42.328209+00:00"
      }
    ],
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

## two-milestones

Verdict: FAIL. Scratch retained: /private/tmp/omon-task18-qa3-ZZDZwF.

```json
{
  "receipt": {
    "todo": 18,
    "scenario": "two-milestones",
    "root": "/private/tmp/omon-task18-qa3-ZZDZwF",
    "dbPath": "/private/tmp/omon-task18-qa3-ZZDZwF/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56797",
          "-Fn"
        ],
        "pid": 56999,
        "code": 0,
        "out": "p56797\nfcwd\nn/private/tmp/omon-task18-qa3-ZZDZwF\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task18-qa3-ZZDZwF/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0x653681483b02efcd\nf2\nn->0x7650a39841e90e4b\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xf374f901a9b4e928\nf7\nn->0x6810595b8d5d00ef\nf8\nn->0xf374f901a9b4e928\nf9\nn/private/tmp/omon-task18-qa3-ZZDZwF/qa.db\nf10\nn/private/tmp/omon-task18-qa3-ZZDZwF/qa.db-wal\nf11\nn/private/tmp/omon-task18-qa3-ZZDZwF/qa.db-shm\nf12\nn/private/tmp/omon-lanes-b468eb1cb67a49078b46ca0b.lock\nf13\nn/tmp/omon-lanes-b468eb1cb67a49078b46ca0b.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "init",
          "/private/tmp/omon-task18-qa3-ZZDZwF/workspace/lane"
        ],
        "pid": 57081,
        "code": 0,
        "out": "Initialized empty Git repository in /private/tmp/omon-task18-qa3-ZZDZwF/workspace/lane/.git/\n",
        "err": ""
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "lane",
          "start",
          "qa18",
          "--title",
          "QA2 qa18",
          "--brief",
          "/private/tmp/omon-task18-qa3-ZZDZwF/qa18-brief.md",
          "--done",
          "file unused.txt",
          "--cwd",
          "/private/tmp/omon-task18-qa3-ZZDZwF/workspace/lane",
          "--lead",
          "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
          "--thread",
          "700000000000000002"
        ],
        "pid": 57114,
        "code": 2,
        "out": "{\"error\":\"configuration error: discord api error (status: 500 Internal Server Error, code: -1): [Serenity] Could not decode json when receiving error response from discord:, expected value at line 1 column 1\",\"ok\":false}\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56797,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 900000,
    "deadlineAt": "2026-10-12T00:57:33.908Z",
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
          "file": "/private/tmp/omon-task18-qa3-ZZDZwF/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa18 --title QA2 qa18 --brief /private/tmp/omon-task18-qa3-ZZDZwF/qa18-brief.md --done file unused.txt --cwd /private/tmp/omon-task18-qa3-ZZDZwF/workspace/lane --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002\n\n2 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:154:12)\n    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:35:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:120:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:214:115)\n    at async withOwnedAppServer (unknown)\n    at async runQA (unknown)\n    at processTicksAndRejections (native:7:39)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:34.004Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58578",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-58Gi5k/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task18-qa3-ZZDZwF/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58578",
      "port": 58578,
      "timingsMs": {
        "ready": 2765,
        "cleanup": 162,
        "total": 7362
      },
      "pid": 56661,
      "membersBeforeShutdown": [
        56661
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:41.362Z"
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
    "judge_callsCount": 1,
    "judge_calls": [
      {
        "id": "93ec82c0-eeb2-436e-90dc-44faa1d7575f",
        "purpose": "lane_size",
        "kind": "choice",
        "ms": 855,
        "outcome": "small",
        "used_fallback": 0,
        "created_at": "2026-10-12T00:42:40.989590+00:00"
      }
    ],
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

Loopback recorder rejected the actual lane-start POST: `AssertionError: Mentions must be disabled`; `allowed_mentions.parse` was `["users"]`, `replied_user` was true. GET identity returned 200. No live Discord API was used. Downstream scenario predicates were not reached.


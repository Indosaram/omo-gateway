# Task 15 live QA3

Release: wave4c. Source and binary were verified before execution.

## dirty-worktree-refused

Verdict: FAIL. Scratch retained: /private/tmp/omon-task15-qa3-q25GW1.

```json
{
  "receipt": {
    "todo": 15,
    "scenario": "dirty-worktree-refused",
    "root": "/private/tmp/omon-task15-qa3-q25GW1",
    "dbPath": "/private/tmp/omon-task15-qa3-q25GW1/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56793",
          "-Fn"
        ],
        "pid": 56995,
        "code": 0,
        "out": "p56793\nfcwd\nn/private/tmp/omon-task15-qa3-q25GW1\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task15-qa3-q25GW1/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xe9587216ce53c36c\nf2\nn->0x913e8c8363db102c\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x779b82bfbf42f893\nf7\nn->0x1f82b59014eb6ea7\nf8\nn->0x779b82bfbf42f893\nf9\nn/private/tmp/omon-task15-qa3-q25GW1/qa.db\nf10\nn/private/tmp/omon-task15-qa3-q25GW1/qa.db-wal\nf11\nn/private/tmp/omon-task15-qa3-q25GW1/qa.db-shm\nf12\nn/private/tmp/omon-lanes-d60df5fcaa6dbf8aab7e23ff.lock\nf13\nn/tmp/omon-lanes-d60df5fcaa6dbf8aab7e23ff.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "init",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/base"
        ],
        "pid": 57084,
        "code": 0,
        "out": "Initialized empty Git repository in /private/tmp/omon-task15-qa3-q25GW1/workspace/base/.git/\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/base",
          "add",
          "README.md"
        ],
        "pid": 57113,
        "code": 0,
        "out": "",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/base",
          "-c",
          "user.name=Owned QA",
          "-c",
          "user.email=qa@example.invalid",
          "commit",
          "-m",
          "QA fixture baseline"
        ],
        "pid": 57200,
        "code": 0,
        "out": "[main (root-commit) 6bce86f] QA fixture baseline\n 1 file changed, 1 insertion(+)\n create mode 100644 README.md\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/base",
          "worktree",
          "add",
          "-b",
          "qa-owned",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/lane-worktree"
        ],
        "pid": 57213,
        "code": 0,
        "out": "HEAD is now at 6bce86f QA fixture baseline\n",
        "err": "Preparing worktree (new branch 'qa-owned')\n"
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "lane",
          "start",
          "qa15-dirty",
          "--title",
          "QA2 qa15-dirty",
          "--brief",
          "/private/tmp/omon-task15-qa3-q25GW1/qa15-dirty-brief.md",
          "--done",
          "file done.txt",
          "--cwd",
          "/private/tmp/omon-task15-qa3-q25GW1/workspace/lane-worktree",
          "--lead",
          "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
          "--thread",
          "700000000000000002"
        ],
        "pid": 57284,
        "code": 2,
        "out": "{\"error\":\"configuration error: discord api error (status: 500 Internal Server Error, code: -1): [Serenity] Could not decode json when receiving error response from discord:, expected value at line 1 column 1\",\"ok\":false}\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56793,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 1200000,
    "deadlineAt": "2026-10-12T01:02:33.826Z",
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
          "file": "/private/tmp/omon-task15-qa3-q25GW1/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa15-dirty --title QA2 qa15-dirty --brief /private/tmp/omon-task15-qa3-q25GW1/qa15-dirty-brief.md --done file done.txt --cwd /private/tmp/omon-task15-qa3-q25GW1/workspace/lane-worktree --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002\n\n2 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:154:12)\n    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:35:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:53:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:214:115)\n    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)\n    at async runQA (unknown)\n    at processTicksAndRejections (native:7:39)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:33.833Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58574",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-J5Bhbg/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task15-qa3-q25GW1/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58574",
      "port": 58574,
      "timingsMs": {
        "ready": 2913,
        "cleanup": 323,
        "total": 9352
      },
      "pid": 56607,
      "membersBeforeShutdown": [
        56607
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:43.181Z"
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
        "id": "0e055418-c82c-4c84-90e4-99a3f2f30277",
        "purpose": "lane_size",
        "kind": "choice",
        "ms": 1645,
        "outcome": "small",
        "used_fallback": 0,
        "created_at": "2026-10-12T00:42:42.722750+00:00"
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

## close-archive-reply-unarchive

Verdict: FAIL. Scratch retained: /private/tmp/omon-task15-qa3-p3NFr0.

```json
{
  "receipt": {
    "todo": 15,
    "scenario": "close-archive-reply-unarchive",
    "root": "/private/tmp/omon-task15-qa3-p3NFr0",
    "dbPath": "/private/tmp/omon-task15-qa3-p3NFr0/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56800",
          "-Fn"
        ],
        "pid": 57068,
        "code": 0,
        "out": "p56800\nfcwd\nn/private/tmp/omon-task15-qa3-p3NFr0\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task15-qa3-p3NFr0/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xb243f2adede96d18\nf2\nn->0xe98aaa55c7e0c125\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xf09b3ce3b6f69deb\nf7\nn->0xfc5a20b721f3f104\nf8\nn->0xf09b3ce3b6f69deb\nf9\nn/private/tmp/omon-task15-qa3-p3NFr0/qa.db\nf10\nn/private/tmp/omon-task15-qa3-p3NFr0/qa.db-wal\nf11\nn/private/tmp/omon-task15-qa3-p3NFr0/qa.db-shm\nf12\nn/private/tmp/omon-lanes-408b53edd438073d19fdfab2.lock\nf13\nn/tmp/omon-lanes-408b53edd438073d19fdfab2.sock\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "init",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/base"
        ],
        "pid": 57097,
        "code": 0,
        "out": "Initialized empty Git repository in /private/tmp/omon-task15-qa3-p3NFr0/workspace/base/.git/\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/base",
          "add",
          "README.md"
        ],
        "pid": 57145,
        "code": 0,
        "out": "",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/base",
          "-c",
          "user.name=Owned QA",
          "-c",
          "user.email=qa@example.invalid",
          "commit",
          "-m",
          "QA fixture baseline"
        ],
        "pid": 57190,
        "code": 0,
        "out": "[main (root-commit) 6bce86f] QA fixture baseline\n 1 file changed, 1 insertion(+)\n create mode 100644 README.md\n",
        "err": ""
      },
      {
        "argv": [
          "/usr/bin/git",
          "-C",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/base",
          "worktree",
          "add",
          "-b",
          "qa-owned",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/lane-worktree"
        ],
        "pid": 57206,
        "code": 0,
        "out": "HEAD is now at 6bce86f QA fixture baseline\n",
        "err": "Preparing worktree (new branch 'qa-owned')\n"
      },
      {
        "argv": [
          "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway",
          "lane",
          "start",
          "qa15-clean",
          "--title",
          "QA2 qa15-clean",
          "--brief",
          "/private/tmp/omon-task15-qa3-p3NFr0/qa15-clean-brief.md",
          "--done",
          "file done.txt",
          "--cwd",
          "/private/tmp/omon-task15-qa3-p3NFr0/workspace/lane-worktree",
          "--lead",
          "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
          "--thread",
          "700000000000000002"
        ],
        "pid": 57260,
        "code": 2,
        "out": "{\"error\":\"configuration error: discord api error (status: 500 Internal Server Error, code: -1): [Serenity] Could not decode json when receiving error response from discord:, expected value at line 1 column 1\",\"ok\":false}\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56800,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 1200000,
    "deadlineAt": "2026-10-12T01:02:33.754Z",
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
          "file": "/private/tmp/omon-task15-qa3-p3NFr0/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway lane start qa15-clean --title QA2 qa15-clean --brief /private/tmp/omon-task15-qa3-p3NFr0/qa15-clean-brief.md --done file done.txt --cwd /private/tmp/omon-task15-qa3-p3NFr0/workspace/lane-worktree --lead 7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451 --thread 700000000000000002\n\n2 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:154:12)\n    at async startLane (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:35:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-scenarios.mjs:53:11)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:214:115)\n    at async withOwnedAppServer (unknown)\n    at async runQA (unknown)\n    at processTicksAndRejections (native:7:39)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:33.831Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58572",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-wPjKDd/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task15-qa3-p3NFr0/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58572",
      "port": 58572,
      "timingsMs": {
        "ready": 3178,
        "cleanup": 183,
        "total": 9055
      },
      "pid": 56605,
      "membersBeforeShutdown": [
        56605
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:42:42.883Z"
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
        "id": "df4e0d29-856b-4d4d-adb8-27acd8cd808f",
        "purpose": "lane_size",
        "kind": "choice",
        "ms": 1582,
        "outcome": "small",
        "used_fallback": 0,
        "created_at": "2026-10-12T00:42:42.565349+00:00"
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


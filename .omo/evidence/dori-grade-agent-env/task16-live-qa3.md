# Task 16 live QA3

Inbox insertion: FAIL, real decision deadline 180 seconds; generation 1, consumed 0, admitted_ms null, no judge/events. Idle behavior: PASS, full 600000 ms state equality observation with no activity. Idle scenario release integrity: FAIL, binary changed before final identity check; manifest remained unchanged. No rerun and no product edits by this runner.

## inbox-insert-real-kibitzer

```json
{
  "receipt": {
    "todo": 16,
    "scenario": "inbox-insert-real-kibitzer",
    "root": "/private/tmp/omon-task16-qa3-ICrEM3",
    "dbPath": "/private/tmp/omon-task16-qa3-ICrEM3/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56803",
          "-Fn"
        ],
        "pid": 56939,
        "code": 0,
        "out": "p56803\nfcwd\nn/private/tmp/omon-task16-qa3-ICrEM3\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task16-qa3-ICrEM3/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0x39874dfc4705565a\nf2\nn->0x3d8798aa54fabe63\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x2a1faa15896a9a3a\nf7\nn->0x7d49fe1c2ad0055a\nf8\nn->0x2a1faa15896a9a3a\nf9\nn/private/tmp/omon-task16-qa3-ICrEM3/qa.db\nf10\nn/private/tmp/omon-task16-qa3-ICrEM3/qa.db-wal\nf11\nn/private/tmp/omon-task16-qa3-ICrEM3/qa.db-shm\nf12\nn/private/tmp/omon-lanes-966d9ef1a84e905f3a076aef.lock\nf13\nn/tmp/omon-lanes-966d9ef1a84e905f3a076aef.sock\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56803,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 780000,
    "deadlineAt": "2026-10-12T00:55:33.798Z",
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
          "file": "/private/tmp/omon-task16-qa3-ICrEM3/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "error": "Error: single real kibitzer decision deadline; final authoritative query failed\n    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-common.mjs:77:71)",
    "appserver": {
      "startedAt": "2026-10-12T00:42:33.872Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58575",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-wCliQM/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task16-qa3-ICrEM3/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58575",
      "port": 58575,
      "timingsMs": {
        "ready": 3148,
        "cleanup": 114,
        "total": 185285
      },
      "pid": 56640,
      "membersBeforeShutdown": [
        56640
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:45:39.155Z"
    },
    "identityError": "AssertionError [ERR_ASSERTION]: Release binary SHA-256 mismatch\n\u001b[32mactual\u001b[39m \u001b[31mexpected\u001b[39m\n\n\u001b[39m'\u001b[39m\u001b[32m3\u001b[39m\u001b[32m5\u001b[39m\u001b[39md\u001b[39m\u001b[32m9\u001b[39m\u001b[31m1\u001b[39m\u001b[31m6\u001b[39m\u001b[31m2\u001b[39m\u001b[39ma\u001b[39m\u001b[32m8\u001b[39m\u001b[32mf\u001b[39m\u001b[31m2\u001b[39m\u001b[31m3\u001b[39m\u001b[39m4\u001b[39m\u001b[32me\u001b[39m\u001b[32mf\u001b[39m\u001b[32m8\u001b[39m\u001b[32m8\u001b[39m\u001b[32me\u001b[39m\u001b[32m7\u001b[39m\u001b[32md\u001b[39m\u001b[39m3\u001b[39m\u001b[32m0\u001b[39m\u001b[32m5\u001b[39m\u001b[32me\u001b[39m\u001b[32m9\u001b[39m\u001b[39m1\u001b[39m\u001b[32m5\u001b[39m\u001b[32m8\u001b[39m\u001b[32m5\u001b[39m\u001b[32ma\u001b[39m\u001b[32m4\u001b[39m\u001b[32m5\u001b[39m\u001b[32m1\u001b[39m\u001b[32m2\u001b[39m\u001b[31m3\u001b[39m\u001b[39me\u001b[39m\u001b[32me\u001b[39m\u001b[32mf\u001b[39m\u001b[32m2\u001b[39m\u001b[32m0\u001b[39m\u001b[39m3\u001b[39m\u001b[32m6\u001b[39m\u001b[39m3\u001b[39m\u001b[31m2\u001b[39m\u001b[39m9\u001b[39m\u001b[32m5\u001b[39m\u001b[32m7\u001b[39m\u001b[31md\u001b[39m\u001b[31m4\u001b[39m\u001b[39m0\u001b[39m\u001b[32mc\u001b[39m\u001b[32m5\u001b[39m\u001b[32mf\u001b[39m\u001b[32mf\u001b[39m\u001b[32m0\u001b[39m\u001b[32m5\u001b[39m\u001b[32m5\u001b[39m\u001b[31m7\u001b[39m\u001b[31m2\u001b[39m\u001b[31m4\u001b[39m\u001b[39me\u001b[39m\u001b[32ma\u001b[39m\u001b[31mb\u001b[39m\u001b[31m0\u001b[39m\u001b[31m8\u001b[39m\u001b[39m3\u001b[39m\u001b[39m4\u001b[39m\u001b[39m5\u001b[39m\u001b[39m8\u001b[39m\u001b[31mb\u001b[39m\u001b[31m1\u001b[39m\u001b[39ma\u001b[39m\u001b[32mc\u001b[39m\u001b[32mc\u001b[39m\u001b[32m6\u001b[39m\u001b[39me\u001b[39m\u001b[39m1\u001b[39m\u001b[31m8\u001b[39m\u001b[31mf\u001b[39m\u001b[31mb\u001b[39m\u001b[31m7\u001b[39m\u001b[31m7\u001b[39m\u001b[31m9\u001b[39m\u001b[31m5\u001b[39m\u001b[31me\u001b[39m\u001b[31mf\u001b[39m\u001b[39m3\u001b[39m\u001b[31m7\u001b[39m\u001b[31me\u001b[39m\u001b[39m5\u001b[39m\u001b[31mc\u001b[39m\u001b[31m1\u001b[39m\u001b[39m6\u001b[39m\u001b[31m1\u001b[39m\u001b[39m8\u001b[39m\u001b[31m4\u001b[39m\u001b[31me\u001b[39m\u001b[31ma\u001b[39m\u001b[31ma\u001b[39m\u001b[31mf\u001b[39m\u001b[31mb\u001b[39m\u001b[31me\u001b[39m\u001b[31m2\u001b[39m\u001b[31m7\u001b[39m\u001b[31m8\u001b[39m\u001b[31mf\u001b[39m\u001b[39m'\u001b[39m\n",
    "exitCode": 1,
    "scratchRetained": true
  },
  "dbState": {
    "kibitzer_state": [
      {
        "lead": "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
        "generation": 1,
        "consumed": 0,
        "admitted_ms": null
      }
    ],
    "model_usage": [],
    "judge_calls": [],
    "lane_events": [],
    "inbox_items": [
      {
        "id": "qa16-new",
        "source": "discord",
        "source_ref": "qa16-new",
        "thread_ref": null,
        "author": null,
        "title": "Owned QA",
        "snippet": "Owner must decide an urgent operational blocker now; agent should ask for a recommendation immediately.",
        "received_at": "2026-10-12T00:42:39.031Z",
        "link": "https://example.invalid/qa16-new",
        "raw_hash": "ff15409bca87e59187324ffdc8d81e49478a7349706fc4d94da56c410ed527cb",
        "collected_at": "2026-10-12T00:42:39.031Z"
      }
    ]
  }
}
```
## idle-ten-minutes

```json
{
  "receipt": {
    "todo": 16,
    "scenario": "idle-ten-minutes",
    "root": "/private/tmp/omon-task16-qa3-aMsHYZ",
    "dbPath": "/private/tmp/omon-task16-qa3-aMsHYZ/qa.db",
    "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/wave4c-release.json",
    "binarySha256": "d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f",
    "commands": [
      {
        "argv": [
          "/usr/sbin/lsof",
          "-a",
          "-p",
          "56795",
          "-Fn"
        ],
        "pid": 57062,
        "code": 0,
        "out": "p56795\nfcwd\nn/private/tmp/omon-task16-qa3-aMsHYZ\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway\nftxt\nn/private/tmp/omon-task16-qa3-aMsHYZ/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xc754c82fbd6e2388\nf2\nn->0x937fa3313e752993\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xeb9a2135f41fe2a5\nf7\nn->0x261ff1caf620f68\nf8\nn->0xeb9a2135f41fe2a5\nf9\nn/private/tmp/omon-task16-qa3-aMsHYZ/qa.db\nf10\nn/private/tmp/omon-task16-qa3-aMsHYZ/qa.db-wal\nf11\nn/private/tmp/omon-task16-qa3-aMsHYZ/qa.db-shm\nf12\nn/private/tmp/omon-lanes-6133f30e9a6944fb5e0ef840.lock\nf13\nn/tmp/omon-lanes-6133f30e9a6944fb5e0ef840.sock\n",
        "err": ""
      }
    ],
    "cleanup": [
      {
        "pid": 56795,
        "code": 0,
        "pidAbsent": true,
        "socketRemoved": true
      }
    ],
    "status": "failed",
    "deadlineMs": 780000,
    "deadlineAt": "2026-10-12T00:55:33.867Z",
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
          "file": "/private/tmp/omon-task16-qa3-aMsHYZ/qa.db"
        }
      ],
      "lanes": {
        "n": 0
      },
      "messages": {
        "n": 0
      }
    },
    "appserver": {
      "startedAt": "2026-10-12T00:42:33.877Z",
      "argv": [
        "omo",
        "app-server",
        "--listen",
        "ws://127.0.0.1:58576",
        "--ws-auth",
        "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-C80HRJ/ws-token"
      ],
      "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
      "home": "/private/tmp/omon-task16-qa3-aMsHYZ/home",
      "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
      "url": "ws://127.0.0.1:58576",
      "port": 58576,
      "timingsMs": {
        "ready": 2877,
        "cleanup": 52,
        "total": 605415
      },
      "pid": 56641,
      "membersBeforeShutdown": [
        56641
      ],
      "remainingGroupPids": [],
      "pidAbsent": true,
      "childExit": {
        "code": 0,
        "signal": null
      },
      "portRebindSucceeded": true,
      "privateScratchRemoved": true,
      "finishedAt": "2026-10-12T00:52:39.289Z"
    },
    "identityError": "AssertionError [ERR_ASSERTION]: Release binary SHA-256 mismatch\n\u001b[32mactual\u001b[39m \u001b[31mexpected\u001b[39m\n\n\u001b[39m'\u001b[39m\u001b[31md\u001b[39m\u001b[39m1\u001b[39m\u001b[32m5\u001b[39m\u001b[32m1\u001b[39m\u001b[32m4\u001b[39m\u001b[32m0\u001b[39m\u001b[31m6\u001b[39m\u001b[31m2\u001b[39m\u001b[39ma\u001b[39m\u001b[32m5\u001b[39m\u001b[31m2\u001b[39m\u001b[39m3\u001b[39m\u001b[32m5\u001b[39m\u001b[32mc\u001b[39m\u001b[32m7\u001b[39m\u001b[32m6\u001b[39m\u001b[32m6\u001b[39m\u001b[31m4\u001b[39m\u001b[39m3\u001b[39m\u001b[31m1\u001b[39m\u001b[39m3\u001b[39m\u001b[32ma\u001b[39m\u001b[32m8\u001b[39m\u001b[32m9\u001b[39m\u001b[31me\u001b[39m\u001b[39m3\u001b[39m\u001b[32md\u001b[39m\u001b[32md\u001b[39m\u001b[32mf\u001b[39m\u001b[32m2\u001b[39m\u001b[39m3\u001b[39m\u001b[31m2\u001b[39m\u001b[39m9\u001b[39m\u001b[32ma\u001b[39m\u001b[39md\u001b[39m\u001b[32m1\u001b[39m\u001b[32mc\u001b[39m\u001b[32m8\u001b[39m\u001b[32m6\u001b[39m\u001b[31m4\u001b[39m\u001b[31m0\u001b[39m\u001b[31m7\u001b[39m\u001b[31m2\u001b[39m\u001b[31m4\u001b[39m\u001b[31me\u001b[39m\u001b[39mb\u001b[39m\u001b[32m3\u001b[39m\u001b[32m9\u001b[39m\u001b[39m0\u001b[39m\u001b[32m1\u001b[39m\u001b[32mc\u001b[39m\u001b[32m6\u001b[39m\u001b[32mc\u001b[39m\u001b[32m0\u001b[39m\u001b[31m8\u001b[39m\u001b[31m3\u001b[39m\u001b[31m4\u001b[39m\u001b[39m5\u001b[39m\u001b[32m6\u001b[39m\u001b[31m8\u001b[39m\u001b[31mb\u001b[39m\u001b[31m1\u001b[39m\u001b[31ma\u001b[39m\u001b[39me\u001b[39m\u001b[31m1\u001b[39m\u001b[31m8\u001b[39m\u001b[31mf\u001b[39m\u001b[31mb\u001b[39m\u001b[39m7\u001b[39m\u001b[31m7\u001b[39m\u001b[39m9\u001b[39m\u001b[32mc\u001b[39m\u001b[39m5\u001b[39m\u001b[32m7\u001b[39m\u001b[39me\u001b[39m\u001b[32m5\u001b[39m\u001b[32me\u001b[39m\u001b[32m9\u001b[39m\u001b[32m9\u001b[39m\u001b[32md\u001b[39m\u001b[31mf\u001b[39m\u001b[39m3\u001b[39m\u001b[32md\u001b[39m\u001b[32md\u001b[39m\u001b[39m7\u001b[39m\u001b[32mb\u001b[39m\u001b[31me\u001b[39m\u001b[39m5\u001b[39m\u001b[39mc\u001b[39m\u001b[32m7\u001b[39m\u001b[31m1\u001b[39m\u001b[31m6\u001b[39m\u001b[31m1\u001b[39m\u001b[31m8\u001b[39m\u001b[39m4\u001b[39m\u001b[32md\u001b[39m\u001b[31me\u001b[39m\u001b[31ma\u001b[39m\u001b[31ma\u001b[39m\u001b[31mf\u001b[39m\u001b[31mb\u001b[39m\u001b[31me\u001b[39m\u001b[31m2\u001b[39m\u001b[31m7\u001b[39m\u001b[31m8\u001b[39m\u001b[31mf\u001b[39m\u001b[39m'\u001b[39m\n",
    "exitCode": 1,
    "scratchRetained": true
  },
  "dbState": {
    "kibitzer_state": [
      {
        "lead": "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
        "generation": 0,
        "consumed": 0,
        "admitted_ms": null
      }
    ],
    "model_usage": [],
    "judge_calls": [],
    "lane_events": [],
    "inbox_items": []
  }
}
```


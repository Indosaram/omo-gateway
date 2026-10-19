# Task 24 LAST live QA

Verdict: **FAIL**. Live source collection retained 2395 items and 25 decisions (18 fyi, 7 ignore); decisions persist while writing has no completed usage row. Full triage was not reached within the 300000 ms window. Mail cursor error verbatim: `tool execution error: mail thread has invalid date`. No repair or rerun.

Scratch DB: `/private/tmp/omon-task24-live-final-ObzuN4/qa.db`. Binary SHA before/after: `0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00`. Overall deadline 1200000 ms at 2026-10-12T04:01:33.459Z.

## Machine evidence

```json
{
  "items": {
    "n": 2395
  },
  "decisions": [
    {
      "class": "fyi",
      "n": 18
    },
    {
      "class": "ignore",
      "n": 7
    }
  ],
  "cursors": [
    {
      "source": "mail",
      "cursor": "",
      "updated_at": "2026-10-12T03:41:40.221913+00:00",
      "error": "tool execution error: mail thread has invalid date"
    },
    {
      "source": "kakao",
      "cursor": "{\"473141270907887\":\"3946140385183289347\",\"362523295536474\":\"3938153069541922817\",\"439574144043688\":\"3946677091337961474\",\"478220938392397\":\"3934368620825477122\",\"480124656282390\":\"3949319327947905025\",\"105526708659510\":\"3949680658620479489\",\"423865457526597\":\"3943060042162733059\",\"18495682269601817\":\"3949685904144678913\",\"18493130682391129\":\"3945970744848814081\",\"466862616116095\":\"3939699903368155137\",\"389877320942893\":\"3941737668081666049\",\"207630757210407\":\"3940543615875295233\",\"455091709510530\":\"3931707706442031105\",\"192140114177389\":\"3948467843120572418\"}",
      "updated_at": "2026-10-12T03:45:39.416109+00:00",
      "error": null
    },
    {
      "source": "imessage",
      "cursor": "2026-10-11T10:57:55.385+00:00",
      "updated_at": "2026-10-12T03:41:50.831287+00:00",
      "error": null
    },
    {
      "source": "discord",
      "cursor": "",
      "updated_at": "2026-10-12T03:41:50.833660+00:00",
      "error": null
    }
  ],
  "writing": []
}
```

## Original helper receipt

```json
{
  "todo": 24,
  "scenario": "task24-scenarios",
  "root": "/private/tmp/omon-task24-live-final-ObzuN4",
  "dbPath": "/private/tmp/omon-task24-live-final-ObzuN4/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54583",
        "-Fn"
      ],
      "pid": 54833,
      "code": 0,
      "out": "p54583\nfcwd\nn/private/tmp/omon-task24-live-final-ObzuN4\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/usr/lib/dyld\nftxt\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-shm\nf0\nn/dev/null\nf1\nn->0x558379b2c559f73e\nf2\nn->0xbc5d84e6f2311682\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x937fa3313e752993\nf7\nn->0xa3245693d800255\nf8\nn->0x937fa3313e752993\nf9\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db\nf10\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-wal\nf11\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-shm\nf12\nn/private/tmp/omon-task24-live-final-ObzuN4/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-fcf0b6e36f911a369a962d61.lock\nf14\nn/tmp/omon-lanes-fcf0b6e36f911a369a962d61.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/private/tmp/omon-task24-live-final-ObzuN4/bin/zele",
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
      "pid": 54875,
      "code": 0,
      "outputBytes": 894,
      "errorBytes": 0
    },
    {
      "argv": [
        "/opt/homebrew/bin/imsg",
        "chats",
        "--limit",
        "1",
        "--json"
      ],
      "pid": 55197,
      "code": 0,
      "outputBytes": 345,
      "errorBytes": 0
    },
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "55232",
        "-Fn"
      ],
      "pid": 55265,
      "code": 0,
      "out": "p55232\nfcwd\nn/private/tmp/omon-task24-live-final-ObzuN4\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xf0503d86534da654\nf2\nn->0xa3245693d800255\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xe2b06d882ab834b2\nf7\nn->0x5460de1523168b46\nf8\nn->0xe2b06d882ab834b2\nf9\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db\nf10\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-wal\nf11\nn/private/tmp/omon-task24-live-final-ObzuN4/qa.db-shm\nf12\nn/private/tmp/omon-task24-live-final-ObzuN4/home/.omon/cron-run-now.sock\nf13\nn/tmp/omon-lanes-fcf0b6e36f911a369a962d61.sock\nf14\nn/private/tmp/omon-lanes-fcf0b6e36f911a369a962d61.lock\nf16\nn->0x96f906e5d95446b0\nf17\nn->0x577e6f09f040e26f\nf18\nn->0x7a8bad045a18cccf\nf20\nn->0xaa00404f6cebb6b8\n",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 54583,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    },
    {
      "pid": 55232,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "failed",
  "deadlineMs": 1200000,
  "deadlineAt": "2026-10-12T04:01:33.459Z",
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
    "ownerHomeWrites": "denied except ~/.zele sandbox allowance",
    "otherHome": "/private/tmp/omon-task24-live-final-ObzuN4/home",
    "contentLogged": false
  },
  "initial": {
    "pragma": [
      {
        "seq": 0,
        "name": "main",
        "file": "/private/tmp/omon-task24-live-final-ObzuN4/qa.db"
      }
    ],
    "lanes": {
      "n": 0
    },
    "messages": {
      "n": 0
    }
  },
  "error": "Error\n    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:77:71)",
  "appserver": {
    "startedAt": "2026-10-12T03:41:33.500Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:53567",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-n2P6Dr/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task24-live-final-ObzuN4/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:53567",
    "port": 53567,
    "timingsMs": {
      "ready": 874,
      "cleanup": 15063,
      "total": 325206
    },
    "pid": 54349,
    "membersBeforeShutdown": [
      54349,
      58349
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": null,
      "signal": "SIGKILL"
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:46:58.708Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 1,
  "scratchRetained": true
}
```

Full DB/logs retained at scratch root. Owned RPC cleanup verified separately in last-qa-integrity.json.


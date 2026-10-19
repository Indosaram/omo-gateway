# Task 26 LAST live QA

Verdict: **NEEDS-REVIEW**. Machine approval flow PASS: pending numbered proposal 1, zero lanes before approval, local approval linked exactly one lane and one completed real lane turn. Bot classification is a fixture; authenticated numbered Discord ingress is excluded. These unexercised and human judgement parts require review, not an automatic PASS.

Scratch DB: `/private/tmp/omon-task26-live-final-3Cq4dM/qa.db`. Binary SHA before/after: `0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00`. Overall deadline 900000 ms at 2026-10-12T03:56:33.417Z.

## Machine evidence

```json
{
  "lanes": [
    {
      "key": "inbox-proposal-1",
      "title": "discord: Owned QA",
      "lead_session_key": "7:discord|-|19:1474231559895646240|-|18:414011632306618368|19:1465631383862120451",
      "thread_channel_id": null,
      "parent_channel_id": null,
      "status_message_id": null,
      "cwd": "/private/tmp/omon-task26-live-final-3Cq4dM/workspace/qa26",
      "brief_path": "/private/tmp/omon-task26-live-final-3Cq4dM/.omo/inbox-proposals/inbox-proposal-1.md",
      "session_key": "4:lane|-|16:inbox-proposal-1|-|21:lane:inbox-proposal-1",
      "omo_thread_id": "01a12e33-d8af-788c-8d2b-b7a36650f7d3",
      "model": "inferhub/cb/deepseek-v4.1-flash",
      "status": "done_claimed",
      "done_signals": "file done.txt",
      "claim_evidence": "done signal live: file done.txt exists at /private/tmp/omon-task26-live-final-3Cq4dM/workspace/qa26/done.txt; lane cwd git clean (untracked only, 0 commits, no remote); qa26 isolated task complete, no owner-facing send/reply performed",
      "claim_at": "2026-10-12T03:42:37.197168+00:00",
      "objection": null,
      "turns_run": 1,
      "last_turn_outcome": "completed",
      "last_report": "[REPORT] inbox-proposal-1 | done | done.txt created at lane cwd and verified live; lane claimed done_claimed (claim_at 2026-10-12T03:42:37Z, no objection); source treated as data only, no owner-facing send/reply.",
      "last_activity_at": "2026-10-12T03:42:51.834612+00:00",
      "stale_posted_at": null,
      "created_at": "2026-10-12T03:41:52.149661+00:00",
      "updated_at": "2026-10-12T03:42:51.839005+00:00",
      "kind": "managed",
      "external_target": null
    }
  ],
  "decision": {
    "item_id": "qa26",
    "class": "bot",
    "reason": "Owned QA fixture",
    "due_at": null,
    "proposed_action": "Perform isolated coding task",
    "lane_key": "inbox-proposal-1",
    "status": "approved",
    "decided_at": "2026-10-12T03:41:35.752Z",
    "model": "fixture",
    "outcome": "{\"error\":null,\"owner_text\":null,\"proposal\":1,\"state\":\"completed\"}"
  }
}
```

## Original helper receipt

```json
{
  "todo": 26,
  "scenario": "task26-scenarios",
  "root": "/private/tmp/omon-task26-live-final-3Cq4dM",
  "dbPath": "/private/tmp/omon-task26-live-final-3Cq4dM/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54573",
        "-Fn"
      ],
      "pid": 54841,
      "code": 0,
      "out": "p54573\nfcwd\nn/private/tmp/omon-task26-live-final-3Cq4dM\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task26-live-final-3Cq4dM/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xeb9ef869db59a589\nf2\nn->0x33d43735a2df6068\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x913e8c8363db102c\nf7\nn->0xb2882a1f59c2a3c4\nf8\nn->0x913e8c8363db102c\nf9\nn/private/tmp/omon-task26-live-final-3Cq4dM/qa.db\nf10\nn/private/tmp/omon-task26-live-final-3Cq4dM/qa.db-wal\nf11\nn/private/tmp/omon-task26-live-final-3Cq4dM/qa.db-shm\nf12\nn/private/tmp/omon-task26-live-final-3Cq4dM/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-98302305b543ae30e8a96f00.lock\nf14\nn/tmp/omon-lanes-98302305b543ae30e8a96f00.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/usr/bin/git",
        "init",
        "/private/tmp/omon-task26-live-final-3Cq4dM/workspace/qa26"
      ],
      "pid": 54876,
      "code": 0,
      "out": "Initialized empty Git repository in /private/tmp/omon-task26-live-final-3Cq4dM/workspace/qa26/.git/\n",
      "err": ""
    },
    {
      "argv": [
        "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway",
        "inbox-propose",
        "qa26",
        "--channel",
        "1474231559895646240",
        "--action-json",
        "{\"kind\":\"repo\",\"cwd\":\"/private/tmp/omon-task26-live-final-3Cq4dM/workspace/qa26\",\"done\":\"file done.txt\"}"
      ],
      "pid": 54928,
      "code": 0,
      "out": "{\"display\":\"#1 discord: Owned QA -> Perform isolated coding task\\nhttps://example.invalid/qa26\",\"item_id\":\"qa26\",\"number\":1}\n",
      "err": ""
    },
    {
      "argv": [
        "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway",
        "inbox",
        "set",
        "qa26",
        "--status",
        "approved"
      ],
      "pid": 56331,
      "code": 0,
      "out": "",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 54573,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "blocked",
  "deadlineMs": 900000,
  "deadlineAt": "2026-10-12T03:56:33.417Z",
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
        "file": "/private/tmp/omon-task26-live-final-3Cq4dM/qa.db"
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
      "scenario": "authenticated approval -> lane",
      "reason": "NEEDS-REVIEW: local inbox set approval exercises native proposal execution, not authenticated Discord numbered-reply ingress. No fabricated CAS approval rows."
    }
  ],
  "appserver": {
    "startedAt": "2026-10-12T03:41:33.463Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:53566",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-z4O5xu/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task26-live-final-3Cq4dM/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:53566",
    "port": 53566,
    "timingsMs": {
      "ready": 829,
      "cleanup": 15064,
      "total": 333778
    },
    "pid": 54338,
    "membersBeforeShutdown": [
      54338,
      56722
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": null,
      "signal": "SIGKILL"
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:47:07.240Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 3,
  "scratchRetained": true
}
```

Full DB/logs retained at scratch root. Owned RPC cleanup verified separately in last-qa-integrity.json.


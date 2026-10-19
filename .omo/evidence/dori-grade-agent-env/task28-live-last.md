# Task 28 LAST live QA: authorized readonly-wrapper rerun

Verdict: **FAIL** for list/read together; list PASS (exit 0, errors=[]), read exit 1. Original host_unavailable failure is a scratch-HOME harness issue, not a product bug. With the exact authorized wrapper, sessions list succeeds, but sessions read selects the first local session outside the scratch workspace and receives scope_denied. No additional scope flag was injected; no further rerun or remote mutation. This scoped-read limitation is retained verbatim rather than called a proven product bug.

## Read error

```text
{"error":"session command failed (1): omo thread read: scope_denied: Thread 01a12db5-407a-7644-84e2-467b2d469998 exists outside this workspace.\n  next: Call thread_list with all_scope to confirm the id, then retry with all_scope set, or target a thread in this workspace.","ok":false}

```

## Rerun receipt including exact wrapper

```json
{
  "todo": 28,
  "scenario": "task28-final",
  "root": "/private/tmp/omon-task28-live-final-JXeIXg",
  "dbPath": "/private/tmp/omon-task28-live-final-JXeIXg/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "91837",
        "-Fn"
      ],
      "pid": 91878,
      "code": 0,
      "out": "p91837\nfcwd\nn/private/tmp/omon-task28-live-final-JXeIXg\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/usr/lib/dyld\nftxt\nn/private/tmp/omon-task28-live-final-JXeIXg/qa.db-shm\nf0\nn/dev/null\nf1\nn->0x91380cf4336f35d8\nf2\nn->0x725f617ae80d0291\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0xdc7f5cb826a86989\nf7\nn->0x41b0f22ffd3d4a74\nf8\nn->0xdc7f5cb826a86989\nf9\nn/private/tmp/omon-task28-live-final-JXeIXg/qa.db\nf10\nn/private/tmp/omon-task28-live-final-JXeIXg/qa.db-wal\nf11\nn/private/tmp/omon-task28-live-final-JXeIXg/qa.db-shm\nf12\nn/private/tmp/omon-task28-live-final-JXeIXg/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-6345b5d698234846ca355b3f.lock\nf14\nn/tmp/omon-lanes-6345b5d698234846ca355b3f.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway",
        "sessions",
        "list",
        "--json"
      ],
      "pid": 91919,
      "code": 0,
      "out": "{\"errors\":[],\"sessions\":[{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"리모트 채팅 메시지 전송 오류 수정\",\"session_id\":\"01a12db5-407a-7644-84e2-467b2d469998\",\"status\":\"live\",\"updated_at\":\"2026-10-12T03:56:07.927Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/omon-gateway\",\"machine\":\"local\",\"name\":\"Check PirateTalk MCP chat history range\",\"session_id\":\"01a123f9-7fea-7e15-b769-45ce3aae26aa\",\"status\":\"live\",\"updated_at\":\"2026-10-12T03:56:06.942Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/content-intel-dashboard\",\"machine\":\"local\",\"name\":\"/Users/indo/.omo/agent/sessions/--Users-indo-code-project-co\",\"session_id\":\"01a104d8-dd17-7e24-b60c-94c0ffc9e98c\",\"status\":\"live\",\"updated_at\":\"2026-10-12T03:56:02.707Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/ferryx-past\",\"session_id\":\"01a12cfc-1599-7900-9232-194f175d2386\",\"status\":\"live\",\"updated_at\":\"2026-10-12T03:10:53.577Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/lectures\",\"machine\":\"local\",\"name\":\"패스트캠퍼스 스타일 상세페이지 검토\",\"session_id\":\"01a09ee8-e388-7e44-80b7-863306112106\",\"status\":\"live\",\"updated_at\":\"2026-10-12T01:45:02.838Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/lectures\",\"machine\":\"local\",\"name\":\"Start Development Server\",\"session_id\":\"01a12026-495e-7a4c-9c16-9b8187ba54e3\",\"status\":\"live\",\"updated_at\":\"2026-10-12T00:55:53.098Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/maho-workspace\",\"machine\":\"local\",\"name\":\"Troubleshoot Release Notes Download Error\",\"session_id\":\"01a1191b-7b2d-785f-834f-a9f72bdb9122\",\"status\":\"live\",\"updated_at\":\"2026-10-12T00:52:28.022Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"\",\"session_id\":\"01a12d8c-b71b-762e-9804-ad54f8266a9b\",\"status\":\"live\",\"updated_at\":\"2026-10-12T00:39:21.997Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Clean up stale worktrees and add rule\",\"session_id\":\"01a12cf7-9370-7408-923e-ef10f3ac6c3e\",\"status\":\"live\",\"updated_at\":\"2026-10-11T23:31:57.455Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/mahoquot\",\"machine\":\"local\",\"name\":\"CLIProxyAPI 업스트림 기능 캐시 프록시 비교\",\"session_id\":\"01a128b6-228f-71e1-9449-369f1deb5a21\",\"status\":\"live\",\"updated_at\":\"2026-10-11T23:07:01.026Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/lectures\",\"machine\":\"local\",\"name\":\"Resume Lectures Project Session\",\"session_id\":\"01a0e9cd-10fd-7ed9-b996-cb81ad77602b\",\"status\":\"live\",\"updated_at\":\"2026-10-11T22:56:58.418Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/lectures\",\"machine\":\"local\",\"name\":\"슬라이드 소스코드 책 페이지 참조로 변경\",\"session_id\":\"01a0ecbb-2829-72a1-9aca-cb9ff8f9e44e\",\"status\":\"live\",\"updated_at\":\"2026-10-11T22:43:35.093Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project/lectures\",\"machine\":\"local\",\"name\":\"인프런 슬라이드 시각화 개선안\",\"session_id\":\"01a0a7a8-ff23-72bd-879d-8c68ca37c33a\",\"status\":\"live\",\"updated_at\":\"2026-10-11T22:37:47.583Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Volumes/T9-Mac/project/mahoquot\",\"machine\":\"local\",\"name\":\"Debug invalid native tool call order\",\"session_id\":\"01a12b77-4ab7-7860-b25c-4955e8c1fc42\",\"status\":\"live\",\"updated_at\":\"2026-10-11T21:59:22.839Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project\",\"machine\":\"local\",\"name\":\"\",\"session_id\":\"01a12cfa-1643-7ce6-8994-11eb6d510d15\",\"status\":\"live\",\"updated_at\":\"2026-10-11T21:59:13.977Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Goal: run one already-built Ferryx test binary inside the IN\",\"session_id\":\"01a12b75-ec01-7ecf-92eb-81f2f1b8a461\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T15:03:41.795Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Ferryx repo /Volumes/T9-Mac/project/ferryx (shared tree; pre\",\"session_id\":\"01a12b77-1b59-78a2-bd68-a52bce2691b5\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T15:03:38.406Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project\",\"machine\":\"local\",\"name\":\"Personal Identity Ontology Update\",\"session_id\":\"01a11b4e-d349-79f0-ae53-71c361bfd815\",\"status\":\"live\",\"updated_at\":\"2026-10-11T14:49:38.658Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project\",\"machine\":\"local\",\"name\":\"mass ulw research plan 국내/해외 크라우드펀딩 사이트에서 노션 템플릿과 같이 단순 템플릿 \",\"session_id\":\"01a0de23-d4fb-70d5-9297-34893238d2b2\",\"status\":\"live\",\"updated_at\":\"2026-10-11T14:49:38.633Z\"},{\"controls\":[\"send\",\"read\",\"rename\",\"set_model\",\"set_reasoning\",\"interrupt\"],\"cwd\":\"/Users/indo/code/project\",\"machine\":\"local\",\"name\":\"Oliver Heldens AI Music Production\",\"session_id\":\"01a11b4f-8aeb-774d-82a7-55cd22b9be37\",\"status\":\"live\",\"updated_at\":\"2026-10-11T14:49:37.923Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Ferryx repo /Volumes/T9-Mac/project/ferryx (shared tree; pre\",\"session_id\":\"01a12b68-b728-72a8-bdf1-98b2ccb5274f\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T14:45:28.259Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Ferryx repo /Volumes/T9-Mac/project/ferryx (shared tree; pre\",\"session_id\":\"01a12b69-0057-76c0-90ce-7a2870d9f0c9\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T14:43:13.471Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"User requires batch repair of all existing diagnostics befor\",\"session_id\":\"01a129dd-3b35-7a61-a797-5feef67b298a\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T14:40:35.094Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"Investigate Windows live SSH test failed74587 A/B candidate.\",\"session_id\":\"01a128e5-807e-71da-ad85-35aa77953114\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T14:38:13.304Z\"},{\"controls\":[],\"cwd\":\"/Volumes/T9-Mac/project/ferryx\",\"machine\":\"local\",\"name\":\"User requires all diagnostics repaired as one batch followed\",\"session_id\":\"01a129dd-43c3-7951-9682-59fb2f4d485e\",\"status\":\"resumable\",\"updated_at\":\"2026-10-11T14:37:41.118Z\"}]}\n",
      "err": ""
    },
    {
      "argv": [
        "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway",
        "sessions",
        "read",
        "local:01a12db5-407a-7644-84e2-467b2d469998",
        "--limit",
        "2"
      ],
      "pid": 92765,
      "code": 1,
      "out": "",
      "err": "{\"error\":\"session command failed (1): omo thread read: scope_denied: Thread 01a12db5-407a-7644-84e2-467b2d469998 exists outside this workspace.\\n  next: Call thread_list with all_scope to confirm the id, then retry with all_scope set, or target a thread in this workspace.\",\"ok\":false}\n"
    }
  ],
  "cleanup": [
    {
      "pid": 91837,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "failed",
  "deadlineMs": 600000,
  "deadlineAt": "2026-10-12T04:06:04.151Z",
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
        "file": "/private/tmp/omon-task28-live-final-JXeIXg/qa.db"
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
    "path": "/private/tmp/omon-task28-live-final-JXeIXg/bin/omo",
    "content": "#!/bin/sh\ncase \"$1:$2\" in thread:list|thread:read) exec /usr/bin/env HOME=/Users/indo /Users/indo/.bun/bin/omo \"$@\" ;; *) exit 64 ;; esac\n",
    "allowed": [
      "thread list",
      "thread read"
    ],
    "rejectedExit": 64,
    "gatewayHome": "/private/tmp/omon-task28-live-final-JXeIXg/home",
    "readonlyThreadHome": "/Users/indo"
  },
  "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway sessions read local:01a12db5-407a-7644-84e2-467b2d469998 --limit 2\n\n1 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:154:12)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-last-readonly-extra.mjs:27:71)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:214:115)\n    at async withOwnedAppServer (unknown)\n    at async runQA (unknown)\n    at processTicksAndRejections (native:7:39)",
  "appserver": {
    "startedAt": "2026-10-12T03:56:04.186Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:55821",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-H1GObk/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task28-live-final-JXeIXg/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:55821",
    "port": 55821,
    "timingsMs": {
      "ready": 510,
      "cleanup": 48,
      "total": 6987
    },
    "pid": 91742,
    "membersBeforeShutdown": [
      91742
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": 0,
      "signal": null
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:56:11.170Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 1,
  "scratchRetained": true
}
```

## Superseded scratch-HOME receipt

```json
{
  "todo": 28,
  "scenario": "task28-final",
  "root": "/private/tmp/omon-task28-live-final-PfIgXH",
  "dbPath": "/private/tmp/omon-task28-live-final-PfIgXH/qa.db",
  "releasePath": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/final-release.json",
  "binarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "commands": [
    {
      "argv": [
        "/usr/sbin/lsof",
        "-a",
        "-p",
        "54594",
        "-Fn"
      ],
      "pid": 54835,
      "code": 0,
      "out": "p54594\nfcwd\nn/private/tmp/omon-task28-live-final-PfIgXH\nftxt\nn/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway\nftxt\nn/private/tmp/omon-task28-live-final-PfIgXH/qa.db-shm\nftxt\nn/usr/lib/dyld\nf0\nn/dev/null\nf1\nn->0xe6f20658097b192c\nf2\nn->0x40b6b375507c3545\nf3\nncount=0, state=0xa\nf4\nncount=0, state=0xa\nf5\nncount=0, state=0xa\nf6\nn->0x388267ba98a352d5\nf7\nn->0x2ef986ddfa7e2f9\nf8\nn->0x388267ba98a352d5\nf9\nn/private/tmp/omon-task28-live-final-PfIgXH/qa.db\nf10\nn/private/tmp/omon-task28-live-final-PfIgXH/qa.db-wal\nf11\nn/private/tmp/omon-task28-live-final-PfIgXH/qa.db-shm\nf12\nn/private/tmp/omon-task28-live-final-PfIgXH/home/.omon/cron-run-now.sock\nf13\nn/private/tmp/omon-lanes-1a0849d06d53f0d7fb940d1f.lock\nf14\nn/tmp/omon-lanes-1a0849d06d53f0d7fb940d1f.sock\n",
      "err": ""
    },
    {
      "argv": [
        "/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway",
        "sessions",
        "list",
        "--json"
      ],
      "pid": 54869,
      "code": 3,
      "out": "{\"errors\":[{\"code\":3,\"machine\":\"local\",\"message\":\"timed out after 20 seconds\"}],\"sessions\":[]}\n",
      "err": ""
    }
  ],
  "cleanup": [
    {
      "pid": 54594,
      "code": 0,
      "pidAbsent": true,
      "socketRemoved": true
    }
  ],
  "status": "failed",
  "deadlineMs": 600000,
  "deadlineAt": "2026-10-12T03:51:33.523Z",
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
        "file": "/private/tmp/omon-task28-live-final-PfIgXH/qa.db"
      }
    ],
    "lanes": {
      "n": 0
    },
    "messages": {
      "n": 0
    }
  },
  "error": "AssertionError [ERR_ASSERTION]: Unexpected command exit: /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00/omo-gateway sessions list --json\n\n3 !== 0\n\n    at cmd (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:154:12)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-extra.mjs:18:41)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:211:34)\n    at async <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:214:115)\n    at async withOwnedAppServer (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-owned-appserver.mjs:138:11)\n    at async runQA (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:214:43)\n    at processTicksAndRejections (native:7:39)",
  "appserver": {
    "startedAt": "2026-10-12T03:41:33.569Z",
    "argv": [
      "omo",
      "app-server",
      "--listen",
      "ws://127.0.0.1:53569",
      "--ws-auth",
      "/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/omon-qa-owned-appserver-SbJ8LY/ws-token"
    ],
    "cwd": "/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env",
    "home": "/private/tmp/omon-task28-live-final-PfIgXH/home",
    "auth": "private 0600 token file via --ws-auth; Authorization: Bearer; no shared token access",
    "url": "ws://127.0.0.1:53569",
    "port": 53569,
    "timingsMs": {
      "ready": 915,
      "cleanup": 131,
      "total": 22186
    },
    "pid": 54367,
    "membersBeforeShutdown": [
      54367
    ],
    "remainingGroupPids": [],
    "pidAbsent": true,
    "childExit": {
      "code": 0,
      "signal": null
    },
    "portRebindSucceeded": true,
    "privateScratchRemoved": true,
    "finishedAt": "2026-10-12T03:41:55.752Z"
  },
  "finalBinarySha256": "0924e30d9a73ebc57092b07d501b26da898841b200a2daba7dcb346b2d769e00",
  "exitCode": 1,
  "scratchRetained": true
}
```

Both DB roots retained. All cleanup and final hash checks are recorded in last-qa-integrity.json.


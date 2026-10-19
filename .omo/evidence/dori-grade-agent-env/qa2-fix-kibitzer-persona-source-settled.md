# QA2 kibitzer and persona source settlement

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Retained scratch databases were opened read-only. No services, live Discord,
models, env files, or git commands were used in this investigation.

## A: task 16

The retained `qa.db` at `/private/tmp/omon-task16-qa2-Cnugr8` contains one
`kibitzer_state` row with generation 1, consumed 0, admitted_ms NULL. The run
log covers 00:17:20Z to 00:20:20Z. 00:17Z is 09:17 KST, inside the allowed
08:00-24:00 window. Admission spacing cannot explain a NULL admitted_ms.

The scenario explicitly sets `OMON_KIBITZER_ENABLED=true`; its default in both
startup and `Kibitzer::from_env` is enabled. `main.rs` initializes the worker
before producers, then `spawn_lane_service` puts `serve` in the owned JoinSet
with a shared Notify. The retained trigger and state row establish that
initialization ran. The headless-ready and lane-service-ready messages establish
that startup passed the spawn call. There was no serve-start log in that source,
so the old log alone cannot independently prove the spawned task was polled.
There is no evidence of a missing headless wiring call or a task error.

The concrete product defect is the unarmed serve wait: after scanning durable
state it waited indefinitely for an in-process Notify. The QA helper inserts
through a separate SQLite connection. SQL triggers increment generation but do
not signal that Notify. Startup already scans persisted arming, and a lane socket
hint causes another scan, but neither happens after this isolated insert.

`src/lanes/kibitzer.rs` now bounds those waits with a five-second local DB scan.
This does not invoke a judge or model when unarmed; `evaluate` still checks
durable generation, hours, spacing and daily cap before admission. Notify remains
the immediate wake path. The comments on this interval are necessary to explain
the cross-process limitation and distinguish the local check from idle model
polling; the retained Notify-permit comment predates this change.

Info logs now identify configured credential-key presence (never values), serve
startup, and admitted arming. `src/main.rs` logs initialization before producers
and disabled startup. No credential fallback or env-file change was added.

The retained descriptor's envKeys does NOT include `TYPESAFE_API_KEY`,
`TYPESAFE_URL`, or `TYPESAFE_MODEL`. The helper copies these only if inherited.
Its private HOME prevents fallback to the owner's `.zshenv`; the retained private
HOME has no such file. The real `typesafe-judge` executable is symlinked in
scratch/bin, on the helper's PATH, but required API configuration was not supplied
to the gateway in the recorded environment. No judge was attempted before
admission, so missing credentials are an additional QA prerequisite finding,
not the explanation for consumed 0. An API endpoint call was not authorized and
no credential values were inspected.

The new module test `qa_gaps_kibitzer_serves_external_insert_without_notify`
uses a file-backed SQLite database and a separate writer pool. It subscribes to
oneshot signals before starting serve, waits for its unarmed startup scan, inserts
without any Notify, and awaits one judge decision with a bounded timeout. It then
cancels and joins serve and asserts exactly one call and generation/consumed 1/1
with the expected admitted timestamp. The fake judge returns a low score to
prevent model execution. There are no fixed test sleeps or polling loops. The
old unarmed wait has no branch that can deliver this decision after the insert.

## B: task 20

The retained `/private/tmp/omon-task20-qa2-VvXove/qa.db` proves that chat DID run:
one user message at 00:17:22.903411Z, one assistant message at
00:18:57.134687Z, and session metadata binding OMO thread
`01a12d78-a1f1-7ef6-814f-b561c70aa454`. Zero `model_usage` rows is not evidence
of no turn. A delivery obligation with the exact assistant content is also in
state `delivered`. Usage recording is outside this change's ownership.

The log configures dashboard interactive and cron backends on owned app-server
`ws://127.0.0.1:55585`. The scenario inserts the applied prompt into session state
before the first turn. The stored session prompt contains an exact byte-for-byte
copy of the applied marked lanes block; it differs from bot_profiles only by the
scenario's appended isolation constraints. The backend sends that session prompt
as `thread/start.params.developerInstructions`, and enabled toolsets as
`enabledToolsets`. No raw app-server request transcript was retained, so prompt
delivery is supported by stored state plus the executed source path, not a
fabricated wire capture.

Session enabled_toolsets are terminal and file. The model's reply reports reading
`test.mjs` and running `node --test`. The owned app-server helper receives the
QA tool environment, including scratch/bin first in PATH, a symlinked
`omo-gateway`, and `DATABASE_URL=sqlite:///private/tmp/omon-task20-qa2-VvXove/qa.db`.
The backend provisions a per-agent cwd below the configured scratch workspace
and sends that cwd to thread/start; the daemon's initial cwd is the evidence
directory. These are distinct, deliberate working directories. No stored tool
trace proves a lane command was attempted or that its environment was inspected
by the model. No missing terminal tool or persona injection bug was found.

The model chose direct inspection of a single intentionally failing assertion,
then declined to weaken that test. This is a model-behavior/fixture finding, not
a dashboard or profile product fix. The scenario asks to fix test failures but
provides only `assert.equal(1,2)` with no implementation; the lanes block allows
direct small work. No dashboard/profile source was changed.

Actual stored assistant reply, verbatim:

````text
테스트 실패 원인은 확인했지만, **수정은 완료하지 못했습니다.**

확인 결과: 지정한 레포에는 구현 코드 없이 `test.mjs` 하나만 있으며, 내용은 다음과 같습니다.

```js
import assert from 'node:assert/strict'; assert.equal(1,2);
```

`node --test` 실행 결과는 **1개 중 0개 통과, 1개 실패, 종료 코드 1**입니다. 실패 위치는 `test.mjs:1`이고 오류는 `1 !== 2`입니다.

고칠 구현 코드가 없으며, 기대값을 바꾸는 것은 기존의 실패한 검증을 통과하도록 약화하는 변경이므로 파일은 수정하지 않았습니다. 실제로 검증하려는 구현 코드나 올바른 테스트 명세가 필요합니다.

현재: 조사·검증·보고 완료, 수정 차단. 다음: 올바른 구현 또는 테스트 명세 확인.
````

## Validation

LSP error diagnostics: no diagnostics in `src/lanes/kibitzer.rs` or `src/main.rs`.
Scoped rustfmt and rustfmt --check: exit 0.

Focused shared-target test batch (monitor `mon_CMDDDPK1NZF15H2H`): exit 0.
`test_lane_kibitzer`: 9 passed; `test_profile_import`: 7 passed.
`--lib qa_gaps`: 3 passed, including the external-insert regression (5.22s);
`--bin omo-gateway qa_gaps`: 4 passed. Total: 23 passed, 0 failed.
No lock bypass or alternate target was used.
The final repeated LSP diagnostic requests timed out after 3000ms for both files;
the earlier requests reported no diagnostics, and the final source compiled
successfully in both required cargo commands.

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_lane_kibitzer --test test_profile_import
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --lib --bin omo-gateway qa_gaps
```

Source SHA-256 after scoped formatting:

- `src/lanes/kibitzer.rs`: `4afb049aaa3bc141a5b9b153b1bcd0e2c58e1f924e5f47fb52badd890e881dc2`
- `src/main.rs`: `db580fa970830aeb06dc5691442739e2914475006e23c6f7a28ef8b43056e00b`

This is source settlement and local regression coverage, not a rerun or PASS of
the second-round live QA. Judge availability and the original zero-usage finding
remain explicit limits. Retained scratch evidence is not overwritten.

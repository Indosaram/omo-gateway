# QA3 harness readiness

Date: 2026-10-12.
Scope: evidence helpers in this directory only. Harness preparation and local
checks passed; this is not a live QA3 PASS.

## Changes

1. `qa2-common.mjs` loads exactly `TYPESAFE_API_KEY`, `TYPESAFE_URL`, and
   `TYPESAFE_MODEL` through
   `zsh -c 'source ~/.zshenv; printf "%s\0" "$TYPESAFE_API_KEY" "$TYPESAFE_URL" "$TYPESAFE_MODEL"'`.
   The in-memory promise is shared by concurrent variants, so the shell runs once
   per runner process. Stderr is discarded, output is parsed in memory, and only
   those three keys enter the gateway environment. No values are recorded.
   `clean()` redacts all three values and both app-server token keys from command
   captures, JSON artifacts, and gateway logs. Pipe fragments are buffered through
   newline boundaries before redaction to avoid leaking secrets split by chunks.
2. Task 24 installs a scratch-only `zele` wrapper first on PATH, used by both
   source preflight and gateway collection. Only `mail list` and `mail watch` are
   allowed. Those commands receive the owner's real HOME inside a `sandbox-exec`
   policy denying all file writes below that HOME. No account copy, login, or
   owner configuration write is performed. All other commands retain the private
   HOME. The owned app-server no longer overrides the supplied private HOME.
   Receipts record the mail HOME exception, command allowlist and write denial;
   source command receipts retain byte counts, not message contents.
3. Task 20 now supplies four files: a range implementation with an off-by-one,
   a schedule consumer, and two test files containing three failing tests. An
   implementation-only `+ 1` correction fixes all three predicates. The user
   message is unchanged. Actual delivered replies are retained even when no lane
   starts; delegation and question semantics remain `NEEDS-REVIEW` rather than
   a timeout mislabeled as a product failure.
4. The existing exact `GET /api/v10/users/@me` route was preserved and tested
   against the owned recorder: HTTP 200, bot ID `1465631383862120451`, `bot: true`.
5. Each scenario gets a deadline recorded as `deadlineMs` and `deadlineAt`.
   Deadline handling persists a receipt before aborting observers and active
   commands, then follows normal gateway/app-server cleanup and final receipt
   writing. Task 16's actual 600-second idle observation is cancellable. New
   scratch roots and receipts/logs use `qa3`, preserving QA2 artifacts.
6. Task 35 waits for a real completed turn independently of token counters.
   Empty real usage rows give the token assertion `BLOCKED-UPSTREAM`, citing the
   absent Senpi `thread/tokenUsage/updated` emitter. Usage CLI and five-minute
   settlement/jev done_diff checks still run. Nonempty invalid counters still
   fail; no synthetic counters are inserted.

The task 26 helper was also aligned with `qa2-fix-inbox-source-settled.md`:
local `inbox set approved` now executes the native proposal. Its obsolete
zero-lanes assertion and second model instruction were removed. The predicate
checks the linked lane and real completed outcome, not unavailable token rows.
Authenticated Discord numbered-reply approval remains a separate review boundary.

## Deadlines and launch contract

| Task | Scenario deadline |
| --- | --- |
| 15 | 1200 seconds per variant |
| 16 | 780 seconds per variant, including the 600-second idle measurement |
| 18 | 900 seconds per variant |
| 20 | 900 seconds |
| 24 | 1200 seconds |
| 25 | 600 seconds |
| 26 | 900 seconds |
| 35 | 1200 seconds per variant |

The existing execution entry point is retained:

```sh
bun qa2-scenarios.mjs execute <todo> /absolute/released-receipt.json
```

When launching live QA later, any outer command monitor must allow at least
1500 seconds for a runner, including cleanup, rather than the shorter QA2
monitor windows. Do not impose an outer kill deadline inside the scenario
observation window. A SIGKILL cannot be caught; the in-process deadline writes
its receipt before cancellation. Release hold/hash validation remains in place.

## Local verification

Executed once, without a gateway binary, Cargo, model/judge calls, owner mail
access, shared daemon commands, env file edits, or git commands:

```sh
bun test /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa2-predicates.test.mjs
```

Exit 0: **10 passed, 0 failed**, 28 ms. Coverage includes release identity/hold
gates, exact judge-key parsing and redaction with fake values, readonly mail
wrapper policy, deadline receipt-before-abort ordering, cancelled observers,
the real loopback identity response, fixable persona predicates, token verdicts,
and existing lane completion/post predicates. No sleeps or polling were added.

`node --check` passed with exit 0 for all four changed helpers:
`qa2-common.mjs`, `qa2-scenarios.mjs`, `qa2-predicates.test.mjs`, and
`qa-owned-appserver.mjs`.

JavaScript LSP diagnostics were unavailable: initialization reported no valid
TypeScript installation. No dependency was installed to change that environment.

Not exercised here: actual secret loading from the owner's shell, mail sandbox
execution/account discovery, gateway shutdown on a real deadline, owned backend
startup under private HOME, model delegation, or real jev settlement. These are
live QA observations, not claimed passing results from the local predicates.
The known upstream token emitter omission remains blocked.

## Changed-helper SHA-256

```text
3b05d71a6d346302fdadbbd760412c3c586580cf8c8902e871625778c3510c89  qa2-common.mjs
9ab405f111f16b4a22f6363ed88cac8a96ff8ac8995d0a640c69bdda95d73c6d  qa2-scenarios.mjs
8b580bf6a3d989b9fd4537813837c01ed63701c6ec457c10547d1d61a251e53d  qa2-predicates.test.mjs
cbdf83cba11d927ec8b66405d8c356601f15bc65ac70893443ffcca0d2cc64c8  qa-owned-appserver.mjs
```

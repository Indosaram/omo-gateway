# Task 35: PASS against 4196efab (later binary owns final acceptance)

Negative unrelated diff was rejected after the real five-minute window. Claim 2026-10-12T03:10:00.568347+00:00; not_done 2026-10-12T03:15:27.893435+00:00. Reason verbatim: `done_diff fulfills=0.120 clean=0.670 risk=0.000`. One real done_diff judge call, no fallback; native resumed lane then blocked without repairing or re-claiming. Scope is done_diff and judge_calls only, not token counters.

Scratch: /private/tmp/omon-task35-live-final-G6TR8O; DB: /private/tmp/omon-task35-live-final-G6TR8O/qa.db. Raw evidence: /private/tmp/omon-task35-live-final-G6TR8O/done-gate-result.json and task35-live-final.json. Gateway cleanup [{"pid":84741,"code":0,"pidAbsent":true,"socketRemoved":true}]. App-server absent true; remaining group []. SHA before/after 4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52. RPC audit in final-qa-summary.md.

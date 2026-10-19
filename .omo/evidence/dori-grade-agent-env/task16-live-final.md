# Task 16: PASS (inbox insertion -> one decision)

Scratch: /private/tmp/omon-task16-live-final-AwdiyQ; DB: /private/tmp/omon-task16-live-final-AwdiyQ/qa.db.
Machine state records generation=1, consumed=1, admitted_ms=1791774362719; exactly one kibitzer_wake judge call, outcome 0.330, no fallback. This is below the native 0.7 wake threshold (src/lanes/kibitzer.rs:261), so zero model turns and zero lane events are correct. The legacy helper timed out because it requires an event even for a no-wake decision; raw failed helper receipt is retained, not rewritten. No scenario rerun. Idle variant retained prior PASS and was not rerun.

Raw receipt: /private/tmp/omon-task16-live-final-AwdiyQ/task16-live-final.json. Authoritative machine state: /private/tmp/omon-task16-live-final-AwdiyQ/final-machine-state.json. Gateway cleanup: [{"pid":84666,"code":0,"pidAbsent":true,"socketRemoved":true}]. App-server absent true; remaining group []. SHA before/after 4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52/4196efab15f1857eae0747da06386fbf68ce9cfb99c3bc680ad7df2423fb8f52.

Verbatim helper error:

```text
Error: single real kibitzer decision deadline; final authoritative query failed
    at <anonymous> (/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/qa-final-common.mjs:77:71)
```

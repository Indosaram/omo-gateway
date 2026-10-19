# Interrupted coordinated batch

Monitor mon_PFW3ZRS4C7NRGHAJ / session bash_205 reported timed_out, exit code 1. Recovery read confirmed the session terminated. A single process and target-lock probe found no remaining batch-gates.sh or target-w4 Cargo/rustc owner; unrelated mahoquot Cargo PID 63517 remained on its separate target. No processes were killed by this recovery and no gates were restarted.

Prefix: /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/batch-20261011T121136Z

- fmt: exit 1. Raw log contains only the preserved src/agent/backend.rs:61 blank-line formatting baseline.
- clippy: exit 101. Two diagnostics: unused update_referenced_message at src/storage/message_search.rs:245; useless conversion .map_err(Into::into) at src/profiles/lanes_block.rs:99. Both lib and lib-test compilation failed on those diagnostics.
- full: started; no exit recorded. Raw log stops after compilation and the same message_search dead-code warning. Test execution is not established.
- dashboard: not started; log absent.

status.tsv has only fmt and clippy rows. Pre-manifest and provenance exist. Post-manifest and final summary do not exist; manifest equality is unverified. The monitor exit is not a full-test exit or a completed aggregate gate result.

Historical batch 20261011T102415Z remains separate and unchanged. Shared fixes receipt is task-22-batch-fixes.md, including the actual zele/0.8.0 mail-watch help observation: --interval [interval], default 15 seconds; production arguments use --interval 60.

All sources remain frozen. No fixes, reruns, runtime checks, environment changes, service changes, or commits during recovery.

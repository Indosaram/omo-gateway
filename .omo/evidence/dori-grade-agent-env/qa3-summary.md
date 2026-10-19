# Live Manual-QA round 3

All eight tasks / twelve isolated variants finished. Overall gate: FAIL. Binary release identity changed during QA; no successful full release QA claim is made. Task aggregates: 15 FAIL, 16 FAIL (idle behavior passed), 18 FAIL, 20 FAIL, 24 FAIL, 25 NEEDS-REVIEW, 26 FAIL, 35 FAIL. Human boundaries stay NEEDS-REVIEW. Task 35 follows qa3-owner-scope.md and is judged only on jev done_diff and judge_calls.

## Release integrity

Before binary SHA-256: `d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f`.

After binary SHA-256: `15140a535c76633a893ddf239ad1c86b3901c6c056e79c57e5e99d3dd7b5c74d` — mismatch.

Before and after manifest SHA-256: `8f08a9546538d4ff7af0b8d4641eecbcd8153dbb0d25b8b5bb10807298c67028` — match.

Every variant validated the specified released binary before creating its scratch. Eleven variants completed their final hash checks while the binary still matched; the final idle variant recorded Release binary SHA-256 mismatch. This QA runner did not build or modify product source/binary. The writer responsible for replacement was not investigated.

| Task / scenario | Verdict | Machine evidence / limits |
| --- | --- | --- |
| 15 close/archive/reopen | FAIL | Lane-start exit 2; recorder rejected allowed_mentions.parse=["users"]. Lanes 0; downstream predicates not reached. |
| 15 dirty worktree refusal | FAIL | Same lane-start failure; owned fixture retained, refusal not reached. |
| 15 authenticated owner-reply ingress | NEEDS-REVIEW | Authenticated Discord input not exercised. |
| 16 inbox insertion | FAIL | 180-second real decision deadline; generation 1, consumed 0, admitted_ms null; judges/events 0. |
| 16 ten-minute idle behavior | PASS | Full 600000 ms observation completed; generation/consumed 0, no activity. |
| 16 idle release identity | FAIL | Final binary differs from released hash; helper final status failed. |
| 18 two milestones | FAIL | Lane-start exit 2; same mentions assertion, lanes 0. |
| 18 deleted status replacement | FAIL | Same lane-start failure; replacement not reached. |
| 20 profile CLI | PASS | Unknown bot exit 2; repeated apply hash identical, immutable fields equal. |
| 20 live persona reply | FAIL | 240-second actual delivered reply deadline; only user message, lanes 0, delivery rows 0. |
| 20 delegation/question meaning | NEEDS-REVIEW | Reply/question phase not reached; no semantic automatic verdict. |
| 24 live collection | FAIL | Readonly mail preflight exit 1, output 0 bytes, error 1016 bytes; inbox rows/cursors 0; collection not enabled. Error content deliberately not captured. |
| 25 owner list/unknown item | PASS | Seeded owner count 1; unknown item exit 2. |
| 25 native digest | NEEDS-REVIEW | Native omon:inbox-digest registration count 0, no substitute executor. |
| 26 unapproved turn | PASS | Actual QA26_NO_APPROVAL delivered and no lane started before approval. |
| 26 local approval linkage | PASS | Proposal completed, approval_message_id cli:inbox-proposal-1, decision linked to real lane/thread. |
| 26 completed approved lane | FAIL | 300-second completion deadline; lane failed: error: LLM error: omo turn ended with no content (likely upstream LLM failure). |
| 26 authenticated numbered approval | NEEDS-REVIEW | Discord ingress boundary not exercised. |
| 35 jev unrelated-diff gate | FAIL | Required completed turn failed with no content; claim_at null, no not_done/jev reason. Gate not reached; done_diff judge_calls 0, lane_size 1 per DB (2 total). |

## Cleanup and evidence

All 13 gateway lifetimes across 12 scratch roots recorded numeric exit 0, PID absent and lane socket removed. Every owned ephemeral app-server PID/group is absent and its helper proved port rebind; task20, task26 and both task35 app-servers required bounded SIGKILL. Seven escaped detached RPC groups were cleaned only after ownership was established from their scratch host.sock paths; final process inspection found no scratch processes and none of these groups. Scratch DBs are retained. See qa3-final-integrity.json for per-PID evidence and ownership proofs.

All eight task<N>-live-qa3.md receipts exist. Existing task-15.log, task-16.log, task-18.log, task-20.log, task-24.log, task-25.log, task-26.log and task-35.log were appended, preserving earlier rounds. Raw helper receipts, gateway logs, loopback requests and authoritative SQLite DBs remain in the roots linked by receipts. No shared daemon command, production gateway/DB edit, live Discord API, env file write, profile write or model override was performed by this runner. Judge values were loaded only by the supplied helper into memory and redacted. Mail owner HOME exception remained readonly and allowlisted. No product bug was edited or failed scenario relaunched.

Task35 negative history includes additional lane-send recovery messages not issued by this QA runner (they are absent from its command receipt); retained transcript records them without treating them as completed turns. They still ended with no content. No synthetic claim, judge result or counter was inserted.


## Parent note on the release-identity FAIL (2026-10-12)

The "binary after" hash differs because the parent rebuilt target-w4/debug/omo-gateway in place (wave4d) while QA3 was still running. The wave4d gate waited only for processes matching omon-task*-qa3-*, and the 16 idle scenario had already been observed complete; the rebuild landed before the QA3 runner wrote its final integrity check. Scenarios that started earlier executed the wave4c binary d162a234...; no scenario is attributed to the wave4d binary. This is a parent process error, not a product finding. Fix for later rounds: build each release into its own copied path (target-w4/release-bins/<sha>/omo-gateway) so a running QA always keeps its exact binary.

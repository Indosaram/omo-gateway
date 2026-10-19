# QA2 execution ledger

Release: `wave4b-release.json`, status released and release_allowed true.
Before execution binary SHA-256: `5b765f9394354022dde3006252f8d874c74801ee2c54bd373758b54b7eb20f97`.
Before execution manifest SHA-256: `a0837584175fecc6b6ac3907e9f378703c59b510d8ce4887161f27b3ca109f4f`.

The prepared helper did not recognize the released receipt's binary_path/manifest_path keys. Only the evidence helper was adapted to accept those keys without changing release authority or expected hashes. Independent variants were changed from sequential execution to Promise.all; each runQA still owns a separate root, DB, socket, recorder, and ephemeral app-server. Product source and release bytes were not edited.

Helper verification: node --check passed for both changed helpers; bun test qa2-predicates.test.mjs: 4 pass, 0 fail, monitor exit 0. LSP could not initialize because the workspace has no valid TypeScript installation; no dependency was installed.

Subscribed execution monitors:

| Task | Monitor | Session |
| --- | --- | --- |
| 15 | mon_BH7V98FZQV07NP30 | bash_2 |
| 16 | mon_FVCRK3DSFQCPKVEE | bash_3 |
| 18 | mon_ME5E4356Y1BY9Z22 | bash_4 |
| 20 | mon_5ST8R9ZWDVMFEETZ | bash_5 |
| 24 | mon_967NC88E822EAQZE | bash_6 |
| 25 | mon_G78KNPV223TDCXYG | bash_7 |
| 26 | mon_Q45Z24074DRN9CX8 | bash_8 |
| 35 | mon_K6P6CY0W8TWJR4ZR | bash_9 |

Tasks 15 and 18 stopped after lane start returned exit 2: configuration error: missing required environment variable DISCORD_BOT_TOKEN. The local recorders received zero requests, and retained DBs have zero lanes/usage. Source src/lanes/cli.rs:371-379 constructs a direct serenity Http and calls first_bot_token/get_current_user rather than the opt-in loopback constructor. No token was added and no live Discord request was attempted. Product source remains untouched.

Task 25 completed with scenario exit 3: filtered owner count 1, unknown item exit 2, native digest job count 0. Native access remains NEEDS-REVIEW.

Tasks 24 and 26 first attempts exposed preparation errors, not proven product bugs: stale PirateTalk hostname and inbox-proposal instead of the released inbox-propose CLI. Corrected only the evidence helper using the recorded PirateTalk endpoint and actual CLI --help. Fresh retries are subscribed as mon_CBMKSYGAKH6SF57X (bash_11) and mon_0XYCFPQTHEBH0DH0 (bash_12); initial scratch/receipts are retained.

Task 16 insertion scenario finished with exit 1: decision deadline, generation 1 / consumed 0 / admitted_ms null, zero usage/events/judge rows. Its owned gateway exited 0, PID absent, socket removed; app-server exited 0 and listener rebind succeeded. The independent ten-minute idle scenario remains pending.

Tasks 20 and 35 and corrected tasks 24/26 remain pending. Existing monitors remain subscribed; no pending execution is relaunched. Initial task 24 retry state showed 166 collected items and zero classifications; this is not a final verdict. Task 15/26 authenticated Discord boundaries, task 20 response meaning, and task 25 native digest access remain review-only coverage; no prose-based automatic PASS.

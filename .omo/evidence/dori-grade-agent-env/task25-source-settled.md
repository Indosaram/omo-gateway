# Task 25 source settlement

Worktree: `/Users/indo/code/project/omon-gateway-wt/dori-work`.
Status: PASS for source acceptance and focused tests after product registration fixes. NOT a runtime deployment claim. LSP freshness remains unavailable; successful cargo compilation provides the type-check evidence.

## Source implementation

- `src/inbox/cli.rs`: clap list/set commands and a pool-injected command handler. `list` accepts owner/bot/fyi, optional open filter, and JSON output. Unknown set IDs return exit code 2. The entrypoint maps that handler code to `process::exit` in the unapplied wiring patch.
- `src/inbox/list.rs`: joins existing inbox items/decisions, excludes snippets from its output type, sorts deadlines first ascending, then received time ascending, then ID for deterministic ties. Open work includes `open` and `proposed`; approved work is excluded from pending lists. Plain text uses source word, author, action/title/reason, KST due time, link, and item ID.
- `src/inbox/set.rs`: changes local decision status to done/dismissed/approved. It does not execute proposals, write to sources, or create lanes.
- `src/inbox/digest.rs`: registers one stable native gateway job `omon:inbox-digest`, expression `0 9 * * *`, timezone Asia/Seoul. `DigestExecutor` intercepts only this job; other jobs delegate to the original executor. It returns one combined owner/bot message to the normal scheduler outbound path, without a model invocation. No work and no auth problems returns None. Auth-only failures produce a login notice without the raw error. Successful nonempty cron output suppresses another digest in the same KST day, surviving restarts; failure does not suppress retry.
- `src/inbox/mod.rs`: exports the new modules while preserving the concurrent task's proposals export. No triage internals were changed.
- `bot_profiles/blocks/lanes.md`: appended only the task-list/status/digest rule within the existing todo-20 markers. Existing onboarding and lane rules were preserved.
- `tests/test_inbox_cli_digest.rs`: focused scratch SQLite tests and a fake normal cron executor.

## Integration artifact and boundaries

`.omo/evidence/dori-grade-agent-env/task25-wiring.patch` contains entry.rs CLI registration and main.rs scheduler wrapping/registration. It is intentionally NOT applied. No edits were made to main.rs, entry.rs, runner.rs, lanes/cli.rs, storage/lanes.rs, or discord/adapter.rs.

The registration hunk uses the persisted owner's Discord DM session, refuses guild channels and threads, and never falls back to a shared home channel. If no owner DM has been persisted, it logs the missing destination rather than posting anywhere else. Deployment/integration must apply the patch and have a persisted owner DM. Existing related notice cron jobs were not read or modified; consolidation is expressed by the single combined digest and the appended owner rule.

No live DB, models, services, environment files, Discord operations, or git commands were used. The plan's live debug-gateway QA is replaced with seeded scratch handler QA under the explicit user constraints. Actual binary inbox invocation remains unverified because entry/main wiring must remain unapplied. Source APIs and clap parsing are tested directly.

## Acceptance and QA ledger

- Seeded list filters and deadline/received ordering: PASS.
- All three set statuses and no lane execution: PASS.
- Empty digest -> None: PASS.
- Seeded owner JSON happy handler QA: PASS; counts-only diagnostic, no source bodies logged.
- `set nope --status done` -> handler exit 2: PASS; process-level propagation supplied by unapplied wiring patch.
- Native job identity, 09:00 KST next occurrence, one destination, model bypass, non-digest delegation: PASS.
- Auth-only output, raw-error exclusion, daily suppression and KST rollover: PASS.
- Shared/thread/other-user destinations rejected: PASS.
- Persona import regression: PASS, 7 tests including scratch actual-profile CLI import.
- Scoped rustfmt: passed before cargo compilation.
- Wiring patch: `patch --dry-run -p1` passed before cargo compilation; no source file was changed by that command.
- LSP: attempted on owned Rust modules and the focused test. Freshness and daemon requests timed out (3,000/30,000 ms). No clean LSP result is claimed; compilation is the available type-check gate. The daemon was not restarted or reconfigured.

Verification command (one invocation, monitor deadline 1,800 seconds):

```text
CARGO_TARGET_DIR=target-w25 cargo test -j2 --offline --test test_inbox_cli_digest --test test_profile_import -- --nocapture
```

Monitor: `mon_TFHBY9JFTAMJGCDV`, terminal `bash_6`. Final result: watcher timed_out, exit code 1 at the 1,800-second deadline. The last output was dependency compilation at 37/478 (`itoa`, `lock_api`); neither gateway compilation nor either test target completed. No test-pass output and no implementation compiler diagnostic were produced. A subsequent read-only process check found no remaining cargo/rustc process for target-w25. Other tasks' cargo processes were left untouched. The command was not relaunched, preserving the user's ONE focused cargo invocation limit.

Initial outcome before authorized follow-up: blocked by cold compilation timeout. The warm-target results below supersede that verification blocker.

## Authorized warm-target follow-up

The owner identified parallel cold-target contention as the initial timeout cause and authorized target-w4 reuse after target-w15 completion. The monitor's pgrep pattern was split into adjacent shell strings to avoid matching its own command. No target-w25 run was started again.

First warm run: compilation finished in 49.00 seconds; test_inbox_cli_digest had 7 passes and 2 FK failures (exit 101). The initial fixture-only correction added a scratch DM row. Second warm run compiled in 25.68 seconds and yielded 8 passes and 1 failure: `invalid Hermes cron payload: missing field kind`. Cargo stopped before test_profile_import on both failures.

### FK determination and product correction

The original main wiring queried an existing sessions row before calling register; with no persisted DM it logged a warning and skipped registration. Thus that particular startup path could not insert a job with a missing FK, but it also could not run a digest before the owner DM was known. Independently, the public product register API accepted a known DM key without ensuring its sessions row, so it could fail with the same FK error. This was a product prerequisite gap, not solely a fixture gap.

`src/inbox/digest.rs:49` validates the DM spec; `src/inbox/digest.rs:50-59` performs `INSERT INTO sessions ... ON CONFLICT(session_key) DO NOTHING`; `src/inbox/digest.rs:60` only then invokes scheduler.register_with_id. It does not overwrite existing session state. The test fixture pre-seeding was removed. `digest_registers_one_gateway_job_at_nine_kst_and_bypasses_models` now explicitly checks that sessions is empty before registration and checks the created DM row afterward, with repeated registration and normal destination resolution still asserted. This unseeded-session test PASSED.

A separate product payload bug was corrected: the timezone-bearing schedule must satisfy HermesJob decoding used by the normal scheduler destination resolver. The spec now carries `id`, schedule `kind=cron`, and `expr=0 9 * * *` in addition to timezone. The executor still intercepts this job and never calls the model.

Fresh install limitation: a gateway cannot infer a Discord DM channel ID when the owner has never opened one. The unapplied startup wiring still skips registration if no DM address is persisted; it does not create or send to an unknown/shared destination. Once a trusted owner DM key is supplied, register now ensures the local session row rather than assuming one exists.

Final run: `CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j2 --offline --test test_inbox_cli_digest --test test_profile_import`, monitor `mon_5BNCDFRC1MWMSST7`, terminal bash_12, deadline 1,800 seconds. Compilation completed in 44.79 seconds; test_inbox_cli_digest: 9 passed, 0 failed (0.86 seconds); test_profile_import: 7 passed, 0 failed (1.98 seconds); monitor completed with exit code 0. Profile import had not run on the preceding failures, so it was included as the still-unverified target, not as an already-passing rerun.

Scoped rustfmt for the changed digest and focused test passed before this final run. The updated wiring patch passed `patch --dry-run -p1` with exit 0. A fixture-stage LSP test check returned no diagnostics; after the product edits, both digest and test checks timed out waiting for fresh diagnostics at 3,000 ms. No clean final LSP result is claimed.

Final source acceptance: 16 focused tests passed. Only owned inbox/test files and the wiring artifact were changed in this follow-up. Wiring remains unapplied, inbox binary-level and live delivery QA remain unverified by design, and no live service, Discord, environment file, or git action was performed.

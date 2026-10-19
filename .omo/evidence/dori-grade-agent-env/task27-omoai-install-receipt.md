# Task27 omo-ai installation receipt

Status: INSTALLATION INTENTIONALLY SKIPPED - owner machine already runs newer upstream omo-ai 5.1.30 / senpi 2026.10.11, which passes todo27 live QA.

Owner authorization was supplied: "omo-ai 설치: 설치 진행". The approved candidate was verified, but the live package had already changed from the gate baseline. No package switch, dependency upgrade/downgrade, registry deletion, daemon stop/restart or session control occurred.

Owner follow-up explicitly confirmed no candidate installation: installing 5.1.29 would downgrade 5.1.30. Happy gateway QA passed in 10948.827709 ms with local 23 + omarchy 3; unreachable host isolation passed in 5725.626332999998 ms. Plan artifact task-27.json now contains the passing names/cwd-only session projection grouped by machine; its previous failing contents were renamed intact to task-27-historical-failure.json.

## Exact preflight commands and exits

```sh
command -v omo
ls -l /Users/indo/.bun/bin/omo
shasum -a 256 /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-omo-ai-5.1.29-candidate.tgz /Users/indo/.bun/install/global/node_modules/omo-ai/plugin/runtime/thread-sdk/sdk.js /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway
du -sh /Users/indo/.bun/install/global
omo --version
```
Combined shell exit 0. Output: omo 5.1.30 (engine: senpi 2026.10.11); global directory 6.4G.

Candidate SHA-256 matched: 3cd918b55c6b0baaffe7f8d0db038c0b57a62ab41fa38342c3411355c6a6fa10.
Released gateway SHA-256 matched: 71c37a9081d9eeccc61723a1add08ce79dde12dfa380dca58bec225835110d86.

Actual installed package: /Users/indo/.bun/install/global/node_modules/omo-ai, version 5.1.30. Actual engine: senpi 2026.10.11. Both package manifests were read. The global package manifest selects omo-ai ^5.1.30. command -v omo resolves /Users/indo/.bun/install/global/node_modules/.bin/omo; /Users/indo/.bun/bin/omo is a shell shim selecting the same package.

Actual original SDK SHA-256: f3bb6a7edd235c22673835932c8bef353429ec0e4c92f9a65dbe834d860d557e.
Expected gate baseline SDK SHA-256: 4e48e3f376c76fc7f4a9883c41c274c5816f04e57a0df2c9b790e8a9e135a5ee.
Required candidate identity: omo 5.1.29, engine senpi 2026.10.10-11, SDK a920873d397a6cb8a95e2110405bccd75793769d2c51ac3126fda2528d7be1af. NOT installed or verified in the global path.

The instruction to roll back on any verification failure was applied before mutation: the rollback verification returned the actual original hash and version. Restoring the historical 4e48e3f3 hash would overwrite a pre-existing newer installation, not restore the installation observed at the beginning of this task. No such overwrite was performed. This receipt does not claim candidate acceptance.

## Backup and rollback

Recovery update: the initial owned backup directory still exists, but its verify.mjs and expected copied SDK/launcher were absent after the copy/watch exited 1; cause of the missing helpers is not established. No shared process was killed. Final retained backup now uses /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-backup.WLesiV/global; final rollback script /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-backup.WLesiV/rollback.sh; verification helper /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-backup.WLesiV/verify.mjs. Monitor mon_TN54Z5DK95RP1N8S runs /bin/cp -ac /Users/indo/.bun/install/global /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task27-backup.WLesiV/global, then copies the launcher and executes verify.mjs. The earlier paths below describe historical failed attempts, not the authoritative backup. Final rollback script passed /bin/sh -n and verification-only execution (no installed identity change). No dependency restoration was needed because installation was intentionally skipped.

Owned backup root: /Users/indo/.bun/task27-backup.cB97lM. Full global directory copy includes the package, external dependency closure, global manifests, lock and bin helpers; launcher copy: /Users/indo/.bun/task27-backup.cB97lM/launcher-omo. Backup command: /Users/indo/.bun/bin/bun --no-env-file /Users/indo/.bun/task27-backup.cB97lM/backup.mjs; internally /bin/cp -a /Users/indo/.bun/install/global /Users/indo/.bun/task27-backup.cB97lM/global and /bin/cp -p /Users/indo/.bun/bin/omo /Users/indo/.bun/task27-backup.cB97lM/launcher-omo.
Initial backup monitor mon_YN0B7ACPFTPCJRRB timed out (exit 1). Read-only process inspection found no remaining backup/cp process; owned partial copy was 1.8G and lacked omo-ai SDK and launcher. No successful backup was claimed. Recovery monitor mon_XRHNZ84SNNBE7590 runs /bin/cp -ac /Users/indo/.bun/install/global/. /Users/indo/.bun/task27-backup.cB97lM/global/ followed by /bin/cp -p /Users/indo/.bun/bin/omo /Users/indo/.bun/task27-backup.cB97lM/launcher-omo, with 1800000 ms deadline. APFS clone copy preserves the full directory and completes the existing owned destination without changing the live installation. Verification monitor mon_376E5WA6752N3Y1F starts verify.mjs only when launcher-omo exists after copy completion. Final copy and verification results follow below.
Rollback script: /Users/indo/.bun/task27-backup.cB97lM/rollback.sh. Run once initially as /bin/sh /Users/indo/.bun/task27-backup.cB97lM/rollback.sh; exit 0, stdout omo 5.1.30 (engine: senpi 2026.10.11), ROLLBACK_NO_SWITCH_ORIGINAL_VERIFIED. Script subsequently expanded to restore the full backed-up omo-ai package and launcher if the installed SDK differs; external dependencies remain untouched. Final script will be executed once after backup completes. No switch occurred, so its verification path performs no write to the installation.

Preflight command durations were not separately instrumented; no duration is invented. The shell command completed with status completed. The final rollback and measured read-only commands have duration receipts.

Expanded rollback executed before backup completion and printed omo 5.1.30 (engine: senpi 2026.10.11), ROLLBACK_ORIGINAL_VERIFIED; no restore branch ran. The combined diagnostic shell took 395.4953749999986 ms and exited 1 because its subsequent backup SDK lookup ran before that file was copied, and a guessed endpoint directory was absent. Those lookups are not backup-failure or registry-deletion evidence. The running backup was preserved, not restarted. Rollback script shell syntax validation completed without output. Registry path existence is not asserted; no registry write/delete command was executed.

## One direct read-only enumeration

Command: cd /private/tmp/omon-task27-qa.7NWyFr && /usr/bin/env -i HOME=/private/tmp PATH=/usr/bin:/bin TMPDIR=/private/tmp/omon-task27-qa.7NWyFr /Users/indo/.bun/bin/bun --no-env-file /private/tmp/omon-task27-qa.7NWyFr/direct.mjs.
Child argv: ["/Users/indo/.bun/bin/omo", "thread", "list", "--all-scope", "--json"].
Child environment: HOME=/Users/indo; PATH=/Users/indo/.bun/bin:/usr/bin:/bin:/usr/sbin:/sbin; TMPDIR=/private/tmp/omon-task27-qa.7NWyFr; cwd=/private/tmp/omon-task27-qa.7NWyFr; stdin ignored.
Result: exit 0, 12812.365208 ms, JSON array true, 23 rows, stderr 0 bytes; runner/monitor exit 0 (mon_FQ20PF78MNFWZKEW). Executed exactly once as a direct list, on unchanged 5.1.30, NOT on the candidate. Gateway QA performs its own required local listings, separately documented in task-27-live-qa.md.

No send/adopt/interrupt, daemon termination or registry pruning was performed. Existing c12d59ec1b7d76c7 record was not edited/deleted.


## Backup cleanup (parent, 2026-10-12)

The candidate was intentionally not installed (machine runs newer upstream omo 5.1.30 / senpi 2026.10.11, which passes todo 27 live QA; installing 5.1.29 would downgrade). The full-copy backup of ~/.bun/install/global existed only to roll back that install, was still copying (2.9G, under system load ~49), and was never completed. The parent stopped the owned copy (PIDs 10758/10759) and its verify waiter (PID 22968) and removed the partial backup directory /Users/indo/.bun/task27-backup.cB97lM, including its rollback.sh, so an incomplete rollback cannot be run. The live omo installation was never modified; `omo --version` after cleanup is recorded in the parent turn.

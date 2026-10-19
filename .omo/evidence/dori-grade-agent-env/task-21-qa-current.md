# Todo21 focused independent QA

Result: **MISMATCH**, not a final confirmed claim. The frozen binary lists 11 subcommands; the guide contains literal references for 10. Missing: `lane help`.

## Help execution

Working directory: `/Users/indo/code/project/omon-gateway-wt/dori-work`

Actual argv: `["/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway","lane","--help"]`

Process exit: 0. Stderr: empty. Full stdout is captured in the companion log. No lane actions were invoked.

## Literal comparison

Comparator result code: 1 (failure). Generated `help` is included because the plan says every listed subcommand.

| Subcommand | Literal | Present |
| --- | --- | --- |
| start | `lane start` | yes |
| list | `lane list` | yes |
| log | `lane log` | yes |
| send | `lane send` | yes |
| claim-done | `lane claim-done` | yes |
| object | `lane object` | yes |
| close | `lane close` | yes |
| reopen | `lane reopen` | yes |
| can-launch | `lane can-launch` | yes |
| heavy | `lane heavy` | yes |
| help | `lane help` | no |

## Legacy reference

Result code: 0. Exactly one case-insensitive herdr line, and exactly one explicit nonuse line, at `docs/AGENTS_DELEGATE.md:5`:

> The legacy herdr pane manager is not used by lane execution.

## Negative case

Created a throwaway guide copy through apply_patch in the evidence directory with the entire `### Run a heavy command` section removed. The identical comparator was run on original and throwaway content. Original missing set: `help`; negative missing set: `heavy, help`. Negative comparator result code: 1, as required; negative-proof result code: 0. The additional `heavy` mismatch demonstrates detection despite the baseline already failing. These comparator codes are in-kernel results, not subprocess exits. Comparator source and every comparison row are in the log.

## SHA256 and cleanup receipt

Binary: `dac35879f07ad1f93d0fe37b7cb14cc3cd92a4268691fdbf1e812fa2e2fd34ec`

Guide: `e9cfd454c69a07b5dfae5cff769c10c4ba0314c49956a60e739851331a168b6b`

Both hashes were identical after QA. The authorized temporary copy was deleted through apply_patch and its absence checked. An initial incorrect relative path created a copy under the main workspace's `wt/dori-work/.omo/evidence/dori-grade-agent-env/`; that file was also deleted through apply_patch and its absence checked. Empty directories from that path mistake may remain. One in-kernel invocation failed because `compare` was not bound; the subsequent original and negative comparisons used the same bound function successfully.

No source, original guide, Cargo, git, service, DB, or external communication actions occurred. Only help was executed. No runtime behavior beyond command listing was verified; no broad gates were rerun.

## Superseding current-binary QA - 2026-10-11

Status: **source-settled**, not confirmed. This supersedes the historical 10/11 mismatch above for the current dori-work guide; the historical receipt and companion `task-21-qa-current.log` are retained.

Ported only the existing dori-next sentence to `docs/AGENTS_DELEGATE.md:19`: `omo-gateway lane help` prints lane help; `omo-gateway lane help <command>` prints help for the given subcommand. The work guide now matches the next guide byte-for-byte.

The exact work binary `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway` matched required SHA256 `dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e` before execution and after QA. Actual argv: `["/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway","lane","--help"]`. Process exit 0, stderr empty. Only `lane --help` ran, in owned scratch cwd `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/task21-owned-JRgtxZ`, with an empty dotenv and a replaced explicit scratch environment. No database file was created. Full argv, environment, stdout and stderr are in `task-21.log`.

Identical literal comparator: positive result code 0, all 11/11 listed subcommands including generated help covered. Case-insensitive herdr line count 1 equals explicit not-used line count 1, at line 5; at-most-one check passed. Negative proof used an apply_patch-created temporary doc copy with the entire heavy-command section removed: comparator result code 1, missing exactly `heavy`; proof result code 0. Codes are in-kernel comparator results, not subprocess exits. The real doc was not altered for the negative proof.

Work guide before SHA256: `e9cfd454c69a07b5dfae5cff769c10c4ba0314c49956a60e739851331a168b6b`. Work guide after and unchanged next guide SHA256: `685ddd2692ce48f9a32ab204972a81233b34152150bbf98d191aec40dc508e1c`. `task-21.log` records plan, CLI source, main source, both prior receipt hashes and their post-QA identities. Next receipt remains unchanged. All owned temporary files and empty directories were removed; scratch absence verified.

Scope: documentation and .omo evidence only. No source, config, tests, docs/research, Cargo, model/backend, network, Discord, service, git or DB commands. No runtime-action verification or broad gates; main plan checkbox remains unchanged. Stop result is source-settled QA, not a confirmed completion claim.

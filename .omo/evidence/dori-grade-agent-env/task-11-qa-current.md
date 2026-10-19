# Todo11 current independent shell QA

Result: PASS for the requested focused shell checks, including the corrected injection-marker test. This is QA evidence, not final confirmation of the plan or live lane behavior.

## Scope and isolation

Read plan lines 279-288 (full todo11), the complete wrapper, and the complete existing shell test before execution. The test uses a generated PATH stub that executes its own stub branch, a fresh temporary HOME, and a temporary database filename. It does not open a database or invoke real gateway/service/Discord actions. The wrapper was executable before running, so the test's conditional git/chmod fallback was not entered. No Cargo, git, service, DB, or Discord action was performed. During initial QA no product/source/test/config files were edited. The follow-up changes only tests/test_omomeow_delegate.sh in source; this receipt and its log are updated evidence.

Run from `/Users/indo/code/project/omon-gateway-wt/dori-next`: `bash tests/test_omomeow_delegate.sh`, exactly once. Forced `OMO_GATEWAY_BIN=omo-gateway`, sanitized PATH to owned stub plus system tools, unset inherited stub/BASH_ENV/ENV flags, and inherited shell xtrace to capture argv without changing the test. Test exit: **0**. Four existing PASS assertions and the all-tests-passed banner were observed. The test checks help, required arguments, lane-start argv and done signal, key shape, literal brief persistence, thread/cwd forwarding, DATABASE_URL override, no temporary DB write, and default done-signal warning. Full stdout/stderr and argv traces are in [the log](task-11-qa-current.log).

The test's required-argument case supplies a title but omits repo/brief. Independently invoked the real wrapper with `--repo /Users/indo/code/project/omon-gateway-wt/dori-next --brief 'QA missing title'` under the same stub convention (no --title). Exit: **1**; stdout starts with `Usage:`; stderr is `Error: --title, --repo, and --brief are required.`. No gateway capture file, database, or HOME state was created by that case.

## Static checks and stable inputs

Wrapper literal occurrence counts: `herdr=0`, `work_items=0`. Inspection shows direct argv execution through `omo-gateway lane start`, with no legacy launcher or row-insert branch. No git command was executed; git occurs only as a literal completion-signal argument.

SHA-256 for initial QA (historical):

- `bin/omomeow-delegate`: `38f8d09e368b7852b965cd76d1a2d56df1ac088625137598f407fc0823fd03ba`
- `tests/test_omomeow_delegate.sh`: `d82b88b3b6e73a6df402a10fb4a536c286c29bc98bc3a3718c58163e6d2fc5d8`

## Cleanup and limits

Test-owned temporary path `/var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/test_omomeow_delegate_XXXXXX.YWp5zOIrIg` was removed by its EXIT trap, verified absent. macOS mktemp -t used the system temporary base rather than supplied TMPDIR. The independent stub was created with apply_patch and deleted with apply_patch; owned empty HOME/TMPDIR directories and `/Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/todo11-qa-temp-o311pY` were removed and verified absent. Assertions recorded in the log are all true.

## Injection-marker gap resolved

Before: the wrapper ran in the repository invocation cwd while the marker assertion checked TEST_TMP_DIR/SHOULD_NOT_RUN. The wrapper only forwards --cwd as gateway argv; it does not change process cwd. Thus a shell evaluating the relative injected touch could write outside the asserted path.

After: tests/test_omomeow_delegate.sh:26-27 changes into the owned TEST_TMP_DIR before invoking the absolute wrapper path. Stub isolation, absolute stub-script path, and all existing assertions remain. Both injection-bearing calls now run in the owned root. An evaluated relative touch would create exactly the marker checked by the existing failure branch (or trigger an earlier literal argv/brief assertion); cleanup covers that same path. No injected command was intentionally evaluated, and no repository marker was written or removed.

Ran the changed shell test exactly once from dori-next with PATH restricted to system tools, OMO_GATEWAY_BIN=omo-gateway, inherited xtrace, and top-level stub/BASH_ENV/ENV unset. Exit **0**, all four existing PASS lines and final success banner observed. Full actual argv/stdout/stderr are appended to the log. Trace proves cd into /var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/test_omomeow_delegate_XXXXXX.kleb3J4oOs before absolute wrapper invocation, followed by the marker assertion at /var/folders/zh/7cc25lt91b1_dj577306nwdh0000gn/T/test_omomeow_delegate_XXXXXX.kleb3J4oOs/SHOULD_NOT_RUN. EXIT cleanup ran and the owned path is verified absent.

Test SHA-256 changed from d82b88b3b6e73a6df402a10fb4a536c286c29bc98bc3a3718c58163e6d2fc5d8 to **c86f6af488903f045590489af0f331967f707f9c61568c535fc1bbd65aad86ea**. Wrapper SHA-256 remains 38f8d09e368b7852b965cd76d1a2d56df1ac088625137598f407fc0823fd03ba. Post-run source inspection confirms only the owned-cwd insertion; source is settled with no further source edits after the passing run. Bash LSP was unavailable because bash-language-server is not installed; no config/dependency changes were made. No Cargo/git/services/DB/Discord actions. Live lane execution, Rust behavior, deployment, and final plan completion remain outside this receipt.

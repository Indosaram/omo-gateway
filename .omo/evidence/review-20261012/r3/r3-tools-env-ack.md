# r3-tools-env-ack review

### [P2] Protected-env denylist is incomplete for interpreter and tool hooks
- Location: /Users/indo/code/project/omon-gateway/src/tools/terminal.rs:657-681
- Trigger: A caller passes `env` with a key that is not listed, for example `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS`, `PYTHONUSERBASE`, `PYTHONINSPECT`, `PERL5DB`, `CDPATH`, `GCONV_PATH`, `NODE_EXTRA_CA_CERTS`, or `HOME`/`XDG_CONFIG_HOME`. An already-approved `java`, `python`, or `git` can then load attacker-chosen code or config. `HOME` and `XDG_CONFIG_HOME` matter for git because `GIT_` is blocked but `~/.gitconfig` is still read.
- Impact: Partial hardening only. This is defense in depth against a caller who already holds the terminal tool, so it is not a regression and not P1.
- Fix: Add `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS`, `PYTHONUSERBASE`, `PYTHONINSPECT`, `PERL5DB`, `CDPATH`, `GCONV_PATH`, `HOME`, `XDG_CONFIG_HOME`. Alternatively switch to an allowlist, though that is a larger change.

## Verified, no finding
- **Protected env filter**
  - `is_protected_env_key` uppercases the key. `Path`, `git_external_diff` and `Ld_Preload` are therefore caught.
  - Every key in the test list is covered. The listed keys are PATH, LD_*, DYLD_*, BASH_ENV, NODE_OPTIONS, PYTHONPATH, PERL5OPT and GIT_*. Prefix matches handle LD_*, DYLD_* and GIT_*.
  - The error message uses the original-case key, so `msg.contains(key)` holds for `git_external_diff`.
  - The harmless-variable case reaches `command.env`, so it passes.
- **Gateway session env is not filtered**
  - `build_session_environment` (terminal.rs:359) sets `OMON_SESSION_*` and `HERMES_SESSION_*` directly via `command.env`, before the caller-env loop at line 364.
  - None of those keys match the protected list or prefixes. Gateway-set env is not routed through the filter.
  - The augmented `PATH` is also set by the gateway at line 350, before the filter, so it is unaffected.
- **Callers passing `"env"`**
  - The only `"env"` object callers in `src/` and `tests/` are inside terminal.rs: the schema at line 620, test lines 857, 1122 and 1137.
  - No other legitimate caller is broken.
- **Approval test at line 857**
  - It runs `sh -c` with a `$(reboot)` payload and `env: {"PATH": dir.path()}`, expecting `OmonError::Approval`.
  - `require_approval` runs at about line 345, before the env loop at line 364. The refusal is therefore still `Approval`, not the new `ToolExecution` env error, so the test still passes.
- **`check_cron_script` on `echo acked`**
  - `detect_hardline_command` has no `echo` patterns.
  - The only `echo` pattern in `detect_dangerous_command` requires `echo ... | tr ... | <shell>`.
  - `echo acked` therefore passes. The sibling `echo ok` script test already relies on the same path.
- **`cron_ack_command_rejects_hardline_and_dangerous_commands`**
  - The add calls include `action`, `id`, `expression` (`interval:5m`) and `prompt`, which satisfies CronTool.
  - The ack check at cron.rs:612-616 runs before the DB insert. `CronTool::new` has no scheduler, so the DB path is used.
  - The IDs `a1`, `a2` and `a3` are distinct and do not collide.
- **Import paths and existing tests with `ack_command`**
  - `store.rs:444` and `scheduler.rs` `validate_cron_payload_lifecycle` (lines 801-806) still apply only `check_gateway_lifecycle`. Hermes-imported jobs and scheduler-registered payloads are therefore not newly rejected, and `src/cron/ack.rs` is unchanged in behavior.
  - Of the tests with `ack_command`, only the new test goes through `CronTool`. `test_cron_delivery_parity`, `test_fix_fix_cron_exec`, `test_review_agent` and `test_omo_backend` do not.
  - `test_cron_boundary_parity.rs:632` expects `launchctl kickstart` to be rejected, which the lifecycle check still does.
- **Existing behavior, unchanged**
  - `CronTool` `update` does not handle `ack_command` at all, so there is no bypass there.
  - A non-string `ack_command` in `add` is stored unscreened (cron.rs:612-618). The consumers use `as_str`, so it is never executed.

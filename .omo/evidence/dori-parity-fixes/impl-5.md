# Implementation Report: Todo 5 (dori-parity-fixes)

## Overview
- Todo: 5. Accept the documented Done line forms and reject unusable checks
- Worktree: `/Users/indo/code/project/omon-gateway/wt/dori-parity` (branch `fix/dori-parity`)
- Scope compliance: Only modified permitted paths (`src/lanes/signals.rs`, `src/lanes/cli.rs`, `src/sessions/external.rs`, `.omo/evidence/dori-parity-fixes/impl-5.md`).

## Files Changed

1. `src/lanes/signals.rs`
   - Added `pub fn strip_done_prefix(raw: &str) -> &str` removing leading `(?i)done\s*=\s*`.
   - Updated `parse_signals` to strip the `Done =` prefix using `strip_done_prefix` before parsing.
   - Refactored `parse_merged` and `parse_closed` to accept GitHub reference formats: `owner/repo#N`, `owner/repo/pull/N`, and `owner/repo/issues/N` with positive integers `N`.
   - Updated `parse_file_option`:
     - Enforced `sha256=` option must be exactly 64 hex characters (lowercased), returning `SignalParseError::InvalidSyntax("sha256= takes 64 hex characters")` otherwise.
     - Enforced `json:` field path non-empty after stripping optional leading `.`, returning `SignalParseError::InvalidSyntax("json: needs a field path like json:.version")`.
   - Updated in-file tests with short hashes to valid 64-char hex strings.
   - Added 4 new test functions to `lanes::signals`.

2. `src/lanes/cli.rs`
   - Updated `start` function: stores `crate::lanes::signals::strip_done_prefix(&args.done)` into `done_signals` column when creating `Lane::new`, preventing duplicate `Done = Done =` footers.

3. `src/sessions/external.rs`
   - Updated `adopt` function: stores `crate::lanes::signals::strip_done_prefix(done)` into `done_signals` when instantiating `Lane::new`.

## Tests Added or Updated

### In `src/lanes/signals.rs`:
- `test_parse_done_prefix` (new)
  - Asserts `Done = file x.txt`, `done=merged a/b#1`, and `DONE   =   closed a/b#2` parse properly into signals.
- `test_parse_github_url_forms` (new)
  - Asserts `merged owner/repo/pull/42`, `merged owner/repo/issues/43`, `closed owner/repo/pull/10`, `closed owner/repo/issues/20`, and standard `merged owner/repo#99` parse properly.
- `test_parse_rejects_short_sha_and_empty_json_path` (new)
  - Asserts short sha (e.g. `1234abcd`) and non-hex sha fail with `sha256= takes 64 hex characters`.
  - Asserts empty json paths (`json:.=1`, `json:=1`) fail with `json: needs a field path like json:.version`.
- `test_strip_done_prefix_keeps_plain_lines` (new)
  - Asserts plain signal strings without prefix remain intact, and various case/whitespace variations of `Done =` are cleanly stripped.
- `test_parse_rejects` and related test cases in `src/lanes/signals.rs`:
  - Updated legacy test case `[sha256=1234abcd]` to 64-char hex string `[sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef]`.

## Deviations from the Plan
- None. All requirements implemented exactly as described in todo 5.

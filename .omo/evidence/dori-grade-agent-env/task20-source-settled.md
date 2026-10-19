# Todo 20 Source-Settled Evidence Report

- **Date**: 2026-10-12
- **Worktree**: `/Users/indo/code/project/omon-gateway-wt/dori-work`
- **Scope**: Todo 20 (Persona block mechanism, profile export/apply-block, OmOMeow persona update)
- **Status**: SETTLED (all verification passed cleanly)

---

## 1. Changed Files & SHA-256 Hashes

The following files represent the settled implementation for Todo 20:

| File Path | SHA-256 Digest |
|---|---|
| `src/profiles/mod.rs` | `d3d14afd06b66cce667c1267360a7b8f515604e9ce025c8c88b45c03ee49e27a` |
| `src/profiles/lanes_block.rs` | `fc6fab79b0384ed7d59972063fae5d990a164e54d7ec950cdbaa23c67f6751e7` |
| `bot_profiles/blocks/lanes.md` | `43a8915c37b1ad031f89ad3c253764ce094ccf0239874f6f57b4de5910d3ebef` |
| `bot_profiles/omomeow.json` | `1442f048cc1f7b320cd5a47b674282c44f789c62ca5e9ebf3d71485bb3d2e318` |
| `tests/test_profile_import.rs` | `51fa6893b39c7dbb9b1138c3bd926c892809c1f79bce392912d0bbbe33ce2b13` |

---

## 2. Scoped rustfmt, Sentinels, Block Parity & LSP Results

From previous worker (`st_01a12cf3`) and verified in current session:

1. **Scoped rustfmt**:
   - `rustfmt --edition 2021 --check src/profiles/mod.rs src/profiles/lanes_block.rs tests/test_profile_import.rs`
   - **Result**: Clean pass (exit code 0, 0 formatting diffs).

2. **Persona Sentinels**:
   - Machine-consumed sentinels checked in `bot_profiles/omomeow.json`:
     - Contains `[LANE_EVENT]`
     - Contains `omo-gateway lane start`
   - Command:
     ```bash
     jq -e '.system_prompt | contains("[LANE_EVENT]") and contains("omo-gateway lane start")' bot_profiles/omomeow.json
     ```
   - **Result**: `true`, exits 0.

3. **Block Parity**:
   - Markers `<!-- omon:lanes:begin -->` and `<!-- omon:lanes:end -->` wrap `bot_profiles/blocks/lanes.md`.
   - The identical block content is present in `bot_profiles/omomeow.json` `system_prompt`.
   - Idempotent: repeated application does not grow newlines or alter surrounding prompt text.

4. **LSP Error Diagnostics**:
   - `src/profiles/lanes_block.rs`: 0 diagnostics (no errors, no warnings).
   - `tests/test_profile_import.rs`: 0 diagnostics (no errors, no warnings).

---

## 3. Focused Test Execution & Verification

Focused test suite executed in dedicated target directory `target-w20`:
- **Command**:
  ```bash
  CARGO_TARGET_DIR=$PWD/target-w20 cargo test --test test_profile_import -j2 --offline
  ```
- **Log Path**: `.omo/evidence/dori-grade-agent-env/task20-focused.log`
- **Exit Code**: `0`
- **Output**:
  ```
     Running tests/test_profile_import.rs (target-w20/debug/deps/test_profile_import-62a3ffc8ea682d86)

  running 7 tests
  test export_returns_complete_profile_as_json ... ok
  test unknown_bot_has_exit_code_two_and_writes_nothing ... ok
  test replacement_ignores_block_framing_and_preserves_owner_prefix_suffix ... ok
  test replaces_only_marked_region_and_preserves_surrounding_text ... ok
  test appends_once_and_is_idempotent_with_prompt_only_update ... ok
  test malformed_markers_return_error_without_changing_row ... ok
  test cli_applies_block_with_named_flags_on_scratch_profile ... ok

  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.32s
  ```

---

## 4. Preservation & Invariants

- **Untouched Critical Files**:
  - `src/lanes/cli.rs` and `src/lanes/runner.rs` were NOT modified. Any lane CLI wiring stays as a patch artifact.
  - Live bot profiles in the production database remain untouched.
  - Owner main checkout (`/Users/indo/code/project/omon-gateway`), `.env` files, and running gateway daemon were not modified or restarted.
  - All test database interactions use scratch SQLite instances (`tempfile::tempdir()` / `sqlite::memory:`).

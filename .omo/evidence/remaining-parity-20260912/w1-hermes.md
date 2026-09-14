# Hermes Reference State Report (2026-09-12)

## CURRENT-STATE

- **Checkout Path**: `/Users/indo/.hermes/hermes-agent`
- **Git HEAD**: `146f4ed07d1c10031c86a5e7f2ac78d11b253dd9`
- **Recent Git Log**:
  - `146f4ed07 fmt(js): npm run fix on merge (#68938)`
  - `81f4095bc Merge pull request #68918 from NousResearch/bb/composer-focus-keys`
  - `a1a406fd4 feat(desktop): type-to-focus the composer from empty chat chrome`
- **Dirty File Count**: 5
- **Dirty Files (`git status --short`)**:
  - ` M agent/chat_completion_helpers.py`
  - ` M agent/codex_responses_adapter.py`
  - ` M agent/transports/codex.py`
  - ` M gateway/run.py`
  - ` M tests/agent/test_codex_responses_adapter.py`

## DRIFT

- **Same HEAD**: Yes (`146f4ed07d1c10031c86a5e7f2ac78d11b253dd9` vs baseline `146f4ed07d1c10031c86a5e7f2ac78d11b253dd9`)
- **Same Dirty Set**: Yes (exactly the same 5 modified files)
- **New Modifications**: None
- **Drift Observed**: No (`drift=no`)

## VERDICT

IMPLEMENTED

The Hermes reference checkout state has been fully verified against the 2026-09-05 parity baseline. No upstream drift or local modification change has occurred.

## EVIDENCE

- Baseline commit hash in reference version record:
  - File: `.omo/evidence/hermes-parity-20260905/reference-version.txt:1`
    ```
    146f4ed07d1c10031c86a5e7f2ac78d11b253dd9
    ```
- Baseline dirty files in reference version record:
  - File: `.omo/evidence/hermes-parity-20260905/reference-version.txt:2-6`
    ```
     M agent/chat_completion_helpers.py
     M agent/codex_responses_adapter.py
     M agent/transports/codex.py
     M gateway/run.py
     M tests/agent/test_codex_responses_adapter.py
    ```
- Current Hermes commit verification:
  - Command: `git -C /Users/indo/.hermes/hermes-agent rev-parse HEAD` returns `146f4ed07d1c10031c86a5e7f2ac78d11b253dd9`
  - Command: `git -C /Users/indo/.hermes/hermes-agent status --short` returns identical 5 modified files.

## RESIDUAL

None.

## CLAIM

CLOSED: Hermes reference checkout matches baseline HEAD 146f4ed07d1c10031c86a5e7f2ac78d11b253dd9 with identical 5 dirty files and no drift.
